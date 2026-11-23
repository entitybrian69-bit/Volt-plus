//! The screen recorder's Android side (`crate::screen_record`): JNI calls into
//! `dev.horizon.android.ScreenRecorder`.
//!
//! The overlay only reads [`horizon_client::recorder`], so this module keeps that state in
//! step with the platform: it polls the Java status a few times a second (never per frame),
//! lists the recordings when the list is asked for, and turns the player's choices into a
//! capture request. Everything that can fail is reported as a phase and a sentence, so the
//! overlay can say why nothing started instead of pretending it did.
use horizon_client::recorder::{self, Capability, Phase, Recording, Status};
use jni::{
    JNIEnv,
    objects::{JClass, JObject, JString, JValue},
};
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const CLASS_NAME: &str = "dev.horizon.android.ScreenRecorder";

/// Looks `ScreenRecorder` up through the activity's class loader.
///
/// `FindClass` on a thread that merely attached to the VM resolves names with the
/// boot class loader, which cannot see the app's own classes, so every call made
/// that way throws. Going through the activity's loader works from any thread.
fn recorder_class<'a>(
    env: &mut JNIEnv<'a>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JClass<'a>> {
    let name = JObject::from(env.new_string(CLASS_NAME)?);
    let loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;
    let class = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&name)],
        )?
        .l()?;
    if class.is_null() {
        return Err(jni::errors::Error::NullPtr(CLASS_NAME));
    }
    Ok(JClass::from(class))
}

fn polled() -> &'static Mutex<Option<Instant>> {
    static POLLED: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    POLLED.get_or_init(|| Mutex::new(None))
}

fn listed() -> &'static Mutex<Option<Instant>> {
    static LISTED: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    LISTED.get_or_init(|| Mutex::new(None))
}

/// The status poll is throttled: the Java call crosses JNI and reads a few statics.
const POLL_EVERY: Duration = Duration::from_millis(250);
/// The recordings list is a MediaStore query, so it is refreshed rarely.
const LIST_EVERY: Duration = Duration::from_secs(3);

fn text(env: &mut JNIEnv<'_>, value: JObject<'_>) -> jni::errors::Result<String> {
    if value.is_null() {
        return Ok(String::new());
    }
    let text = JString::from(value);
    Ok(env.get_string(&text)?.into())
}

fn phase_of(name: &str) -> Phase {
    match name {
        "asking" => Phase::Asking,
        "starting" => Phase::Starting,
        "recording" => Phase::Recording,
        "stopping" => Phase::Stopping,
        "saved" => Phase::Saved,
        "denied" => Phase::Denied,
        "unsupported" | "permission" => Phase::Unsupported,
        "error" | "busy" => Phase::Error,
        _ => Phase::Idle,
    }
}

fn publish_state(raw: &str) {
    let mut parts = raw.splitn(2, ';');
    let name = parts.next().unwrap_or_default();
    let note = parts.next().unwrap_or_default().to_owned();
    let mut status = recorder::status();
    let phase = phase_of(name);
    if phase == Phase::Error && name == "busy" {
        status.message = "already recording".to_owned();
    } else if !note.is_empty() {
        status.message = note;
    }
    status.phase = phase;
    if !phase.active() && phase != Phase::Saved {
        status.width = 0;
        status.height = 0;
        status.fps = 0;
    }
    recorder::publish(status);
}

/// Reads what this device can do: Android version, display, audio sources, capture service.
pub fn capabilities() -> Capability {
    let Some(raw) = crate::platform::with_context(|env, activity| {
        let class = recorder_class(env, activity)?;
        let value = env
            .call_static_method(
                class,
                "capabilities",
                "(Landroid/content/Context;)Ljava/lang/String;",
                &[JValue::Object(activity)],
            )?
            .l()?;
        text(env, value)
    }) else {
        return Capability::default();
    };
    let fields: Vec<&str> = raw.split(';').collect();
    let number = |i: usize| -> i64 {
        fields
            .get(i)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or_default()
    };
    Capability {
        sdk: number(0) as i32,
        display: (number(1).max(0) as u32, number(2).max(0) as u32),
        game_audio: number(3) == 1,
        microphone: number(4) == 1,
        projection: number(5) == 1,
        device: fields.get(6).copied().unwrap_or_default().to_owned(),
        checked: !raw.is_empty(),
    }
}

fn parse_status(raw: &str) -> Status {
    let mut status = recorder::status();
    let fields: Vec<&str> = raw.splitn(8, ';').collect();
    let field = |i: usize| fields.get(i).copied().unwrap_or_default();
    status.phase = phase_of(field(0));
    status.millis = field(1).parse().unwrap_or_default();
    status.bytes = field(2).parse().unwrap_or_default();
    status.uri = field(3).to_owned();
    status.width = field(4).parse().unwrap_or_default();
    status.height = field(5).parse().unwrap_or_default();
    status.fps = field(6).parse().unwrap_or_default();
    let message = field(7).trim().to_owned();
    if !message.is_empty() {
        status.message = message;
    }
    if status.phase == Phase::Idle && !status.uri.is_empty() {
        status.phase = Phase::Saved;
    }
    status
}

fn parse_recordings(raw: &str) -> Vec<Recording> {
    raw.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.trim().to_owned();
            if name.is_empty() {
                return None;
            }
            let bytes = fields.next().and_then(|v| v.parse().ok()).unwrap_or_default();
            let added_ms = fields.next().and_then(|v| v.parse().ok()).unwrap_or_default();
            let uri = fields.next().unwrap_or_default().trim().to_owned();
            Some(Recording {
                name,
                bytes,
                added_ms,
                uri,
            })
        })
        .collect()
}

/// Keeps the shared status in step with Android, and refreshes the capability list once.
///
/// `watching` is whether the overlay is showing the recorder's page: when nothing is
/// recording and nobody is looking, the poll is skipped so an idle game pays nothing.
pub fn refresh(watching: bool) {
    let known = recorder::status();
    if !watching && !known.phase.active() && known.capability.checked {
        return;
    }
    let now = Instant::now();
    {
        let mut last = polled().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(at) = *last
            && now.duration_since(at) < POLL_EVERY
        {
            return;
        }
        *last = Some(now);
    }
    let Some(raw) = crate::platform::with_context(|env, activity| {
        let class = recorder_class(env, activity)?;
        let value = env
            .call_static_method(class, "status", "()Ljava/lang/String;", &[])?
            .l()?;
        text(env, value)
    }) else {
        return;
    };
    let mut status = parse_status(&raw);
    if !status.capability.checked {
        status.capability = capabilities();
    }
    let listing_due = {
        let guard = listed().lock().unwrap_or_else(|e| e.into_inner());
        guard.is_none_or(|at| now.duration_since(at) > LIST_EVERY)
    };
    if listing_due || status.recordings.is_empty() {
        *listed().lock().unwrap_or_else(|e| e.into_inner()) = Some(now);
        status.recordings = list_recordings();
    }
    recorder::publish(status);
}

/// Asks the system for the screen-capture prompt with the overlay's current choices.
pub fn start() {
    // The capability poll can come back empty (a failed JNI call looks exactly like a
    // device without the service), so it only *warns* here: `ScreenRecorder.start`
    // re-checks everything and answers with the exact reason when it cannot record.
    let capability = recorder::status().capability;
    let settings = recorder::settings();
    if capability.checked && settings.audio == 2 && !capability.game_audio {
        let mut status = recorder::status();
        status.phase = Phase::Unsupported;
        status.message =
            "in-game audio needs Android 10 or newer, choose microphone or no audio".to_owned();
        recorder::publish(status);
        return;
    }
    let (display_width, display_height) = capability.display;
    // The height is the request; Android fits it to the display and the encoder. When the
    // display size is unknown, keep the chosen height and a 16:9 width.
    let height = if display_height >= 240 {
        settings.height().min(display_height)
    } else {
        settings.height()
    };
    let width = if display_width >= 160 && display_height >= 240 {
        ((u64::from(height) * u64::from(display_width)) / u64::from(display_height)) as u32
    } else {
        (height * 16) / 9
    };
    let frames = settings.frames() as i32;
    let audio = settings.audio as i32;
    // When `record_controls` is OFF, the engine copies each raw 3D gameplay frame before the
    // HUD/controls into `EngineRecorder`, so the player keeps seeing controls on screen while the
    // saved MP4 contains only raw gameplay without controls.
    if !settings.record_controls {
        if audio != 0 {
            let mut status = recorder::status();
            status.phase = Phase::Unsupported;
            status.message = "raw gameplay (controls off) supports No audio only — turn Record With Controls on to record audio".to_owned();
            recorder::publish(status);
            return;
        }
        start_engine(width, height, frames);
        return;
    }
    let result = crate::platform::with_context(|env, activity| {
        let class = recorder_class(env, activity)?;
        let value = env
            .call_static_method(
                class,
                "start",
                "(Landroid/app/Activity;IIII)Ljava/lang/String;",
                &[
                    JValue::Object(activity),
                    JValue::Int(width.max(160) as i32),
                    JValue::Int(height as i32),
                    JValue::Int(frames),
                    JValue::Int(audio),
                ],
            )?
            .l()?;
        text(env, value)
    });
    match result {
        Some(raw) => publish_state(&raw),
        None => {
            // Say so instead of leaving the overlay silent.
            let mut status = recorder::status();
            status.phase = Phase::Error;
            status.message = "the recorder service could not be reached".to_owned();
            recorder::publish(status);
        }
    }
}

/// Starts a recording of the game's own frames: `EngineRecorder.java` opens a MediaCodec
/// encoder, the game renders into its surface (HUD-free, see `render_target`), and the
/// codec's output goes to the same `Recordings` folder as a screen recording.
fn start_engine(width: u32, height: u32, frames: i32) {
    release_capture();
    let result = crate::platform::with_context(|env, activity| {
        let class = recorder_class(env, activity)?;
        let value = env
            .call_static_method(
                class,
                "startEngine",
                "(Landroid/app/Activity;III)Ljava/lang/String;",
                &[
                    JValue::Object(activity),
                    JValue::Int(width.max(160) as i32),
                    JValue::Int(height as i32),
                    JValue::Int(frames),
                ],
            )?
            .l()?;
        let note = text(env, value)?;
        // `JClass` is a local reference and not `Clone`: look the class up again.
        let class = recorder_class(env, activity)?;
        let surface = env
            .call_static_method(class, "engineSurface", "()Landroid/view/Surface;", &[])?
            .l()?;
        // The window keeps its own reference to the surface (the codec's); the one taken
        // here is never released, so a frame still in flight when the recording stops
        // always has a live window behind it.
        let window = if surface.is_null() {
            0
        } else {
            let raw = surface.into_raw();
            unsafe { ANativeWindow_fromSurface(env.get_native_interface(), raw) as usize }
        };
        Ok((note, window))
    });
    let Some((note, window)) = result else {
        let mut status = recorder::status();
        status.phase = Phase::Error;
        status.message = "the recorder service could not be reached".to_owned();
        recorder::publish(status);
        return;
    };
    let Some((width, height)) = engine_size(&note) else {
        // The encoder refused (or told us nothing): show why, and start nothing.
        publish_state(&note);
        return;
    };
    // The size the encoder chose is not something the overlay says: it reads it from the
    // status the recorder reports from now on.
    publish_state("recording;");
    if window == 0 {
        let _ = crate::platform::with_context(|env, activity| {
            let class = recorder_class(env, activity)?;
            env.call_static_method(class, "stop", "()Ljava/lang/String;", &[])?;
            Ok(())
        });
        let mut status = recorder::status();
        status.phase = Phase::Error;
        status.message = "the encoder gave no surface to render into".to_owned();
        recorder::publish(status);
        return;
    }
    recorder::set_capture_target(Some(recorder::CaptureTarget {
        window,
        width,
        height,
        serial: recorder::next_capture_serial(),
    }));
}

/// `recording;1280x720`: the size the encoder settled on, from `startEngine`'s answer.
fn engine_size(note: &str) -> Option<(u32, u32)> {
    let (state, rest) = note.split_once(';')?;
    if state != "recording" {
        return None;
    }
    let (width, height) = rest.split_once('x')?;
    let width = width.trim().parse::<u32>().ok()?;
    let height = height.trim().parse::<u32>().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

/// Drops the engine capture, so the renderer stops copying frames into the encoder.
fn release_capture() {
    let _ = recorder::set_capture_target(None);
}

/// Hands the recorder's surface to the renderer, or takes it away again. Called once a frame
/// while a recording can be running; a surface that cannot be made ends the recording with a
/// sentence instead of leaving an empty video behind.
pub fn sync_capture(renderer: &mut horizon_client_render::gfx::Renderer) {
    static ATTEMPTED: Mutex<u64> = Mutex::new(0);
    let mut attempted = ATTEMPTED.lock().unwrap_or_else(|e| e.into_inner());
    let want = recorder::capture_target();
    let Some(target) = want else {
        if renderer.record_serial() != 0 || renderer.record_window() != 0 {
            renderer.set_record_surface(None, [0, 0], 0, 0);
        }
        *attempted = 0;
        return;
    };
    if *attempted == target.serial {
        if renderer.record_serial() == 0 {
            // The surface could not be configured: stop rather than record nothing.
            *attempted = 0;
            release_capture();
            stop();
        }
        return;
    }
    *attempted = target.serial;
    let surface = build_capture_surface(renderer.instance(), target.window);
    match surface {
        Some(surface) => renderer.set_record_surface(
            Some(surface),
            [target.width, target.height],
            target.serial,
            target.window,
        ),
        None => {
            *attempted = 0;
            release_capture();
            let mut status = recorder::status();
            status.message =
                "this device could not capture the game's own frames, so the recording stopped"
                    .to_owned();
            recorder::publish(status);
            stop();
        }
    }
}

/// The encoder's `Surface` as a swapchain the renderer can draw into.
fn build_capture_surface(
    instance: &wgpu::Instance,
    window: usize,
) -> Option<wgpu::Surface<'static>> {
    use winit::raw_window_handle::{
        AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
    };
    let non_null = std::ptr::NonNull::new(window as *mut std::ffi::c_void)?;
    let handle = RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(non_null));
    let display = RawDisplayHandle::Android(AndroidDisplayHandle::new());
    // The window is owned by the codec and outlives this surface (see `start_engine`).
    unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(display),
                raw_window_handle: handle,
            })
            .ok()
    }
}

#[cfg(target_os = "android")]
#[link(name = "android")]
unsafe extern "C" {
    /// The codec's surface as a native window; the reference stays with the game.
    fn ANativeWindow_fromSurface(
        env: *mut jni::sys::JNIEnv,
        surface: jni::sys::jobject,
    ) -> *mut std::ffi::c_void;
}

/// Ends the recording; the service finalises the file before the phase turns to `Saved`.
pub fn stop() {
    // The renderer lets go of the encoder's surface on its next frame, before the encoder
    // itself is stopped: nothing is presented to a surface that is going away.
    release_capture();
    let result = crate::platform::with_context(|env, activity| {
        let class = recorder_class(env, activity)?;
        let value = env
            .call_static_method(class, "stop", "()Ljava/lang/String;", &[])?
            .l()?;
        text(env, value)
    });
    if let Some(raw) = result {
        publish_state(&raw);
    }
    *polled().lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// The finished recordings, newest first.
pub fn list_recordings() -> Vec<Recording> {
    let Some(raw) = crate::platform::with_context(|env, activity| {
        let class = recorder_class(env, activity)?;
        let value = env
            .call_static_method(
                class,
                "recordings",
                "(Landroid/content/Context;)Ljava/lang/String;",
                &[JValue::Object(activity)],
            )?
            .l()?;
        text(env, value)
    }) else {
        return Vec::new();
    };
    parse_recordings(&raw)
}

/// Opens a recording in the system's video player.
pub fn play(uri: &str) -> String {
    let Some(uri) = (!uri.is_empty()).then_some(uri.to_owned()) else {
        return "nothing to play".to_owned();
    };
    let result = crate::platform::with_context(|env, activity| {
        let class = recorder_class(env, activity)?;
        let item = JObject::from(env.new_string(&uri)?);
        let value = env
            .call_static_method(
                class,
                "play",
                "(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(activity), JValue::Object(&item)],
            )?
            .l()?;
        text(env, value)
    });
    let Some(raw) = result else {
        return "the player could not be opened".to_owned();
    };
    raw.split_once(';').map(|(_, note)| note.to_owned()).unwrap_or(raw)
}
