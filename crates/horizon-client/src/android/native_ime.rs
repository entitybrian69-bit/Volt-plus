//! System IME for NativeActivity, using stock EditText on Android's UI looper.
//! No project Java classes or cross-thread View operations are required.
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JValue},
};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
};

#[link(name = "android")]
unsafe extern "C" {
    fn ALooper_forThread() -> *mut std::ffi::c_void;
    fn ALooper_addFd(
        looper: *mut std::ffi::c_void,
        fd: i32,
        ident: i32,
        events: i32,
        callback: Option<unsafe extern "C" fn(i32, i32, *mut std::ffi::c_void) -> i32>,
        data: *mut std::ffi::c_void,
    ) -> i32;
}
enum Command {
    Show(String, bool),
    Hide,
    Poll,
    Capture(bool),
    HidePointer,
    Bounds,
    ImportDocument,
    ImportFolderTree,
    CreateDocument(String, String),
    Screen(bool, bool, bool),
}
struct Ui {
    vm: JavaVM,
    edit: GlobalRef,
    input: GlobalRef,
    native: GlobalRef,
    activity: GlobalRef,
    commands: Mutex<VecDeque<Command>>,
    reader: Mutex<UnixStream>,
    writer: Mutex<UnixStream>,
    text: Mutex<String>,
    polling: AtomicBool,
    active: AtomicBool,
    ready: AtomicBool,
    bounds_pending: AtomicBool,
    bottom: AtomicI32,
    failed: AtomicBool,
    keyboard_request: Mutex<Option<std::time::Instant>>,
    last_bounds: Mutex<Option<std::time::Instant>>,
    avoid_cutout: AtomicBool,
    immersive: AtomicBool,
}
static UI: OnceLock<Arc<Ui>> = OnceLock::new();

/// Must run during NativeActivity onCreate, on the Java UI thread.
///
/// # Safety
/// VM and activity pointers must be supplied by android-activity and valid for
/// the duration of this call. No local JNI reference is retained.
pub unsafe fn initialize(
    vm: *mut jni::sys::JavaVM,
    activity: jni::sys::jobject,
) -> Result<(), String> {
    if UI.get().is_some() {
        return Ok(());
    }
    let vm = unsafe { JavaVM::from_raw(vm) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let (edit, input, native, activity) = env
        .with_local_frame(32, |env| -> jni::errors::Result<_> {
            let activity = unsafe { JObject::from_raw(activity) };
            crate::document_bridge::register(env, &activity, crate::documents::document_result)?;
            let content = env
                .call_method(
                    &activity,
                    "findViewById",
                    "(I)Landroid/view/View;",
                    &[JValue::Int(0x01020002)],
                )?
                .l()?;
            // NativeActivity's first content child owns the native input queue.
            let native = env
                .call_method(
                    &content,
                    "getChildAt",
                    "(I)Landroid/view/View;",
                    &[JValue::Int(0)],
                )?
                .l()?;
            let edit = env.new_object(
                "android/widget/EditText",
                "(Landroid/content/Context;)V",
                &[JValue::Object(&activity)],
            )?;
            env.call_method(&edit, "setInputType", "(I)V", &[JValue::Int(0x20001)])?;
            env.call_method(&edit, "setImeOptions", "(I)V", &[JValue::Int(0x12000000)])?;
            env.call_method(&edit, "setAlpha", "(F)V", &[JValue::Float(0.0)])?;
            env.call_method(&edit, "setSaveEnabled", "(Z)V", &[JValue::Bool(0)])?;
            env.call_method(&edit, "setFocusableInTouchMode", "(Z)V", &[JValue::Bool(1)])?;
            env.call_method(&edit, "setFocusable", "(Z)V", &[JValue::Bool(0)])?;
            let params = env.new_object(
                "android/view/ViewGroup$LayoutParams",
                "(II)V",
                &[JValue::Int(1), JValue::Int(1)],
            )?;
            env.call_method(
                &activity,
                "addContentView",
                "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V",
                &[JValue::Object(&edit), JValue::Object(&params)],
            )?;
            let service = JObject::from(env.new_string("input_method")?);
            let input = env
                .call_method(
                    &activity,
                    "getSystemService",
                    "(Ljava/lang/String;)Ljava/lang/Object;",
                    &[JValue::Object(&service)],
                )?
                .l()?;
            env.call_method(&native, "setFocusable", "(Z)V", &[JValue::Bool(1)])?;
            env.call_method(
                &native,
                "setFocusableInTouchMode",
                "(Z)V",
                &[JValue::Bool(1)],
            )?;
            let sdk = env
                .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
                .and_then(|v| v.i())
                .unwrap_or(0);
            if sdk >= 26 {
                if env
                    .call_method(
                        &native,
                        "setDefaultFocusHighlightEnabled",
                        "(Z)V",
                        &[JValue::Bool(0)],
                    )
                    .is_err()
                {
                    let _ = env.exception_clear();
                }
            }
            env.call_method(&native, "requestFocus", "()Z", &[])?;
            let _ = env.call_static_method(
                "dev/horizon/android/HorizonActivity",
                "hideOsPointer",
                "(Landroid/app/Activity;)V",
                &[JValue::Object(&activity)],
            );
            let _ = env.exception_clear();
            Ok((
                env.new_global_ref(edit)?,
                env.new_global_ref(input)?,
                env.new_global_ref(native)?,
                env.new_global_ref(activity)?,
            ))
        })
        .map_err(|e| {
            let _ = env.exception_clear();
            e.to_string()
        })?;
    drop(env);
    let (reader, writer) = UnixStream::pair().map_err(|e| e.to_string())?;
    reader.set_nonblocking(true).map_err(|e| e.to_string())?;
    writer.set_nonblocking(true).map_err(|e| e.to_string())?;
    let ui = Arc::new(Ui {
        vm,
        edit,
        input,
        native,
        activity,
        commands: Mutex::new(VecDeque::new()),
        reader: Mutex::new(reader),
        writer: Mutex::new(writer),
        text: Mutex::new(String::new()),
        polling: AtomicBool::new(false),
        active: AtomicBool::new(false),
        ready: AtomicBool::new(false),
        bounds_pending: AtomicBool::new(false),
        bottom: AtomicI32::new(0),
        failed: AtomicBool::new(false),
        keyboard_request: Mutex::new(None),
        last_bounds: Mutex::new(None),
        avoid_cutout: AtomicBool::new(false),
        immersive: AtomicBool::new(false),
    });
    let looper = unsafe { ALooper_forThread() };
    if looper.is_null() {
        return Err("No Android UI looper during onCreate".into());
    }
    let fd = ui
        .reader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_raw_fd();
    let data = Arc::into_raw(ui.clone()).cast_mut().cast();
    if unsafe { ALooper_addFd(looper, fd, -1, 1, Some(callback), data) } != 1 {
        // Registration failed: the looper never owns this reference.
        drop(unsafe { Arc::from_raw(data.cast::<Ui>()) });
        return Err("Cannot register Android UI input channel".into());
    }
    // The callback reference and these global JNI refs live for the activity's
    // process lifetime, not per edit. Activity shutdown exits that process.
    UI.set(ui).ok();
    Ok(())
}
impl Ui {
    fn enqueue(&self, c: Command) {
        self.commands
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(c);
        let _ = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write(&[1]);
    }
    fn try_show_keyboard(&self, env: &mut JNIEnv<'_>) -> jni::errors::Result<()> {
        let mut request = self
            .keyboard_request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(started) = *request else {
            return Ok(());
        };
        if !self.active.load(Ordering::Acquire) {
            *request = None;
            return Ok(());
        }
        let focused = env.call_method(&self.edit, "hasFocus", "()Z", &[])?.z()?;
        let window_focused = env
            .call_method(&self.edit, "hasWindowFocus", "()Z", &[])?
            .z()?;
        let served = focused
            && window_focused
            && env
                .call_method(
                    &self.input,
                    "isActive",
                    "(Landroid/view/View;)Z",
                    &[JValue::Object(self.edit.as_obj())],
                )?
                .z()?;
        if served
            && env
                .call_method(
                    &self.input,
                    "showSoftInput",
                    "(Landroid/view/View;I)Z",
                    &[JValue::Object(self.edit.as_obj()), JValue::Int(0)],
                )?
                .z()?
        {
            *request = None;
        } else if started.elapsed() > std::time::Duration::from_secs(2) {
            log::warn!(
                "Android keyboard was not served (view focus={focused}, window focus={window_focused}, served={served})"
            );
            *request = None;
        }
        Ok(())
    }
    fn hide_system_pointer(&self, env: &mut JNIEnv<'_>) {
        let res = (|| -> jni::errors::Result<()> {
            let _ = env.call_static_method(
                "dev/horizon/android/HorizonActivity",
                "hideOsPointer",
                "(Landroid/app/Activity;)V",
                &[JValue::Object(self.activity.as_obj())],
            );
            let _ = env.exception_clear();
            let icon = env
                .call_static_method(
                    "dev/horizon/android/HorizonActivity",
                    "invisiblePointerIcon",
                    "(Landroid/content/Context;)Landroid/view/PointerIcon;",
                    &[JValue::Object(self.activity.as_obj())],
                )
                .and_then(|v| v.l())
                .or_else(|_| {
                    let _ = env.exception_clear();
                    env.call_static_method(
                        "android/view/PointerIcon",
                        "getSystemIcon",
                        "(Landroid/content/Context;I)Landroid/view/PointerIcon;",
                        &[JValue::Object(self.activity.as_obj()), JValue::Int(0)],
                    )?
                    .l()
                })?;
            if !icon.is_null() {
                env.call_method(
                    &self.native,
                    "setPointerIcon",
                    "(Landroid/view/PointerIcon;)V",
                    &[JValue::Object(&icon)],
                )?;
                let _ = env.call_method(
                    self.edit.as_obj(),
                    "setPointerIcon",
                    "(Landroid/view/PointerIcon;)V",
                    &[JValue::Object(&icon)],
                );
                if let Ok(window) = env
                    .call_method(&self.activity, "getWindow", "()Landroid/view/Window;", &[])
                    .and_then(|v| v.l())
                    && let Ok(decor) = env
                        .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
                        .and_then(|v| v.l())
                {
                    let _ = env.call_method(
                        &decor,
                        "setPointerIcon",
                        "(Landroid/view/PointerIcon;)V",
                        &[JValue::Object(&icon)],
                    );
                }
            }
            Ok(())
        })();
        if res.is_err() {
            let _ = env.exception_clear();
        }
    }
    fn apply_screen(
        &self,
        env: &mut JNIEnv<'_>,
        immersive: bool,
        keep_screen_on: bool,
    ) -> jni::errors::Result<()> {
        self.hide_system_pointer(env);
        let _ = env.call_static_method(
            "dev/horizon/android/HorizonActivity",
            "setKeepScreenOnFlag",
            "(Landroid/app/Activity;Z)V",
            &[
                JValue::Object(self.activity.as_obj()),
                JValue::Bool(u8::from(keep_screen_on)),
            ],
        );
        let _ = env.exception_clear();
        let window = env
            .call_method(&self.activity, "getWindow", "()Landroid/view/Window;", &[])?
            .l()?;
        const FLAG_KEEP_SCREEN_ON: i32 = 0x0000_0080;
        env.call_method(
            &window,
            if keep_screen_on { "addFlags" } else { "clearFlags" },
            "(I)V",
            &[JValue::Int(FLAG_KEEP_SCREEN_ON)],
        )?;
        let _ = env.call_method(
            &self.native,
            "setKeepScreenOn",
            "(Z)V",
            &[JValue::Bool(u8::from(keep_screen_on))],
        );
        let decor = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?
            .l()?;
        let _ = env.call_method(
            &decor,
            "setKeepScreenOn",
            "(Z)V",
            &[JValue::Bool(u8::from(keep_screen_on))],
        );
        // Layout always occupies the native surface. Cutout avoidance is handled
        // by the shared rendering/input viewport, including Android 15+.
        let sdk = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()?;
        if sdk >= 28 {
            let attrs = env
                .call_method(
                    &window,
                    "getAttributes",
                    "()Landroid/view/WindowManager$LayoutParams;",
                    &[],
                )?
                .l()?;
            env.set_field(&attrs, "layoutInDisplayCutoutMode", "I", JValue::Int(1))?;
            env.call_method(
                &window,
                "setAttributes",
                "(Landroid/view/WindowManager$LayoutParams;)V",
                &[JValue::Object(&attrs)],
            )?;
        }
        if sdk >= 30 {
            // Keep DecorView edge-to-edge in both modes so toggling immersive
            // system bars never resizes the live ANativeWindow while Vulkan
            // frames are in flight; navigation bar insets are handled via
            // set_native_content_insets below.
            env.call_method(
                &window,
                "setDecorFitsSystemWindows",
                "(Z)V",
                &[JValue::Bool(0)],
            )?;
            let controller = env
                .call_method(
                    &window,
                    "getInsetsController",
                    "()Landroid/view/WindowInsetsController;",
                    &[],
                )?
                .l()?;
            if !controller.is_null() {
                env.call_method(
                    &controller,
                    "setSystemBarsBehavior",
                    "(I)V",
                    &[JValue::Int(2)],
                )?;
                // Status bars stay hidden in both modes. This preference only
                // changes navigation buttons / the gesture bar, never the IME.
                env.call_method(&controller, "hide", "(I)V", &[JValue::Int(1)])?;
                env.call_method(
                    &controller,
                    if immersive { "hide" } else { "show" },
                    "(I)V",
                    &[JValue::Int(2)],
                )?;
            }
        } else {
            env.call_method(
                decor,
                "setSystemUiVisibility",
                "(I)V",
                &[JValue::Int(if immersive { 0x1706 } else { 0x0704 })],
            )?;
        }
        Ok(())
    }
    fn apply(&self, env: &mut JNIEnv<'_>, c: Command) -> jni::errors::Result<()> {
        match c {
            Command::Screen(immersive, avoid, keep_screen_on) => {
                self.avoid_cutout.store(avoid, Ordering::Release);
                self.immersive.store(immersive, Ordering::Release);
                if let Err(error) = self.apply_screen(env, immersive, keep_screen_on) {
                    let _ = env.exception_clear();
                    log::warn!("Android screen preferences: {error}");
                }
                self.enqueue(Command::Bounds);
            }
            Command::ImportDocument => {
                let result = (|| -> jni::errors::Result<()> {
                    let action =
                        JObject::from(env.new_string("android.intent.action.OPEN_DOCUMENT")?);
                    let intent = env.new_object(
                        "android/content/Intent",
                        "(Ljava/lang/String;)V",
                        &[JValue::Object(&action)],
                    )?;
                    let category =
                        JObject::from(env.new_string("android.intent.category.OPENABLE")?);
                    env.call_method(
                        &intent,
                        "addCategory",
                        "(Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&category)],
                    )?;
                    let mime = JObject::from(env.new_string("*/*")?);
                    env.call_method(
                        &intent,
                        "setType",
                        "(Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&mime)],
                    )?;
                    let allow_multiple = JObject::from(
                        env.new_string("android.intent.extra.ALLOW_MULTIPLE")?,
                    );
                    let _ = env.call_method(
                        &intent,
                        "putExtra",
                        "(Ljava/lang/String;Z)Landroid/content/Intent;",
                        &[JValue::Object(&allow_multiple), JValue::Bool(1)],
                    );
                    env.call_method(
                        &intent,
                        "addFlags",
                        "(I)Landroid/content/Intent;",
                        &[JValue::Int(1)],
                    )?;
                    env.call_method(
                        &self.activity,
                        "startActivityForResult",
                        "(Landroid/content/Intent;I)V",
                        &[JValue::Object(&intent), JValue::Int(4101)],
                    )?;
                    Ok(())
                })();
                if let Err(error) = result {
                    let _ = env.exception_clear();
                    log::warn!("Document picker: {error}");
                    crate::documents::picker_failed("No Android document picker is available");
                }
            }
            Command::ImportFolderTree => {
                let result = (|| -> jni::errors::Result<()> {
                    let action =
                        JObject::from(env.new_string("android.intent.action.OPEN_DOCUMENT_TREE")?);
                    let intent = env.new_object(
                        "android/content/Intent",
                        "(Ljava/lang/String;)V",
                        &[JValue::Object(&action)],
                    )?;
                    env.call_method(
                        &intent,
                        "addFlags",
                        "(I)Landroid/content/Intent;",
                        &[JValue::Int(1 | 2)],
                    )?;
                    env.call_method(
                        &self.activity,
                        "startActivityForResult",
                        "(Landroid/content/Intent;I)V",
                        &[JValue::Object(&intent), JValue::Int(4102)],
                    )?;
                    Ok(())
                })();
                if let Err(error) = result {
                    let _ = env.exception_clear();
                    log::warn!("Folder tree picker: {error}");
                    crate::documents::picker_failed("No Android folder picker is available");
                }
            }
            Command::CreateDocument(suggested_name, mime_type) => {
                let result = (|| -> jni::errors::Result<()> {
                    let action =
                        JObject::from(env.new_string("android.intent.action.CREATE_DOCUMENT")?);
                    let intent = env.new_object(
                        "android/content/Intent",
                        "(Ljava/lang/String;)V",
                        &[JValue::Object(&action)],
                    )?;
                    let category =
                        JObject::from(env.new_string("android.intent.category.OPENABLE")?);
                    env.call_method(
                        &intent,
                        "addCategory",
                        "(Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&category)],
                    )?;
                    let mime = JObject::from(env.new_string(&mime_type)?);
                    env.call_method(
                        &intent,
                        "setType",
                        "(Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&mime)],
                    )?;
                    let extra_title =
                        JObject::from(env.new_string("android.intent.extra.TITLE")?);
                    let title_val = JObject::from(env.new_string(&suggested_name)?);
                    env.call_method(
                        &intent,
                        "putExtra",
                        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&extra_title), JValue::Object(&title_val)],
                    )?;
                    env.call_method(
                        &intent,
                        "addFlags",
                        "(I)Landroid/content/Intent;",
                        &[JValue::Int(1 | 2)],
                    )?;
                    env.call_method(
                        &self.activity,
                        "startActivityForResult",
                        "(Landroid/content/Intent;I)V",
                        &[JValue::Object(&intent), JValue::Int(4103)],
                    )?;
                    Ok(())
                })();
                if let Err(error) = result {
                    let _ = env.exception_clear();
                    log::warn!("Create document picker: {error}");
                    crate::documents::picker_failed("No Android file save picker is available");
                }
            }
            Command::Show(text, secret) => {
                env.call_method(
                    &self.edit,
                    "setInputType",
                    "(I)V",
                    &[JValue::Int(if secret { 0x80081 } else { 0x20001 })],
                )?;
                let s = JObject::from(env.new_string(&text)?);
                env.call_method(
                    &self.edit,
                    "setText",
                    "(Ljava/lang/CharSequence;)V",
                    &[JValue::Object(&s)],
                )?;
                env.call_method(
                    &self.edit,
                    "setSelection",
                    "(I)V",
                    &[JValue::Int(text.encode_utf16().count() as i32)],
                )?;
                // Re-enable touch focus on every edit. NativeActivity owns focus
                // between editors, and some Android versions do not serve a view
                // until the next UI-loop turn after requestFocus.
                env.call_method(&self.native, "clearFocus", "()V", &[])?;
                env.call_method(
                    &self.edit,
                    "setFocusableInTouchMode",
                    "(Z)V",
                    &[JValue::Bool(1)],
                )?;
                env.call_method(&self.edit, "requestFocus", "()Z", &[])?;
                env.call_method(
                    &self.input,
                    "restartInput",
                    "(Landroid/view/View;)V",
                    &[JValue::Object(self.edit.as_obj())],
                )?;
                *self
                    .keyboard_request
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some(std::time::Instant::now());
                self.try_show_keyboard(env)?;
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
                self.ready.store(true, Ordering::Release);
            }
            Command::Hide => {
                *self
                    .keyboard_request
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
                let token = env
                    .call_method(&self.edit, "getWindowToken", "()Landroid/os/IBinder;", &[])?
                    .l()?;
                env.call_method(
                    &self.input,
                    "hideSoftInputFromWindow",
                    "(Landroid/os/IBinder;I)Z",
                    &[JValue::Object(&token), JValue::Int(0)],
                )?;
                env.call_method(&self.edit, "clearFocus", "()V", &[])?;
                env.call_method(&self.edit, "setFocusable", "(Z)V", &[JValue::Bool(0)])?;
                env.call_method(&self.native, "requestFocus", "()Z", &[])?;
            }
            Command::Poll => {
                self.polling.store(false, Ordering::Release);
                self.try_show_keyboard(env)?;
                let editable = env
                    .call_method(&self.edit, "getText", "()Landroid/text/Editable;", &[])?
                    .l()?;
                let text = env
                    .call_method(editable, "toString", "()Ljava/lang/String;", &[])?
                    .l()?;
                let text: String = env.get_string(&text.into())?.into();
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
            }
            Command::Bounds => {
                self.bounds_pending.store(false, Ordering::Release);
                let rect = env.new_object("android/graphics/Rect", "()V", &[])?;
                env.call_method(
                    &self.native,
                    "getWindowVisibleDisplayFrame",
                    "(Landroid/graphics/Rect;)V",
                    &[JValue::Object(&rect)],
                )?;
                let location = env.new_int_array(2)?;
                env.call_method(
                    &self.native,
                    "getLocationOnScreen",
                    "([I)V",
                    &[JValue::Object(&location)],
                )?;
                let mut window_origin = [0; 2];
                env.get_int_array_region(location, 0, &mut window_origin)?;
                self.bottom.store(
                    (env.get_field(rect, "bottom", "I")?.i()? - window_origin[1]).max(0),
                    Ordering::Release,
                );
                let mut safe = [0; 4];
                if self.avoid_cutout.load(Ordering::Acquire)
                    || !self.immersive.load(Ordering::Acquire)
                {
                    let result = (|| -> jni::errors::Result<()> {
                        let insets = env
                            .call_method(
                                &self.native,
                                "getRootWindowInsets",
                                "()Landroid/view/WindowInsets;",
                                &[],
                            )?
                            .l()?;
                        if !insets.is_null() {
                            let sdk = env
                                .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
                                .i()?;
                            if !self.immersive.load(Ordering::Acquire) && sdk >= 30 {
                                let navigation = env
                                    .call_method(
                                        &insets,
                                        "getInsets",
                                        "(I)Landroid/graphics/Insets;",
                                        &[JValue::Int(2)],
                                    )?
                                    .l()?;
                                for (i, field) in
                                    ["left", "top", "right", "bottom"].into_iter().enumerate()
                                {
                                    safe[i] =
                                        env.get_field(&navigation, field, "I")?.i()?.max(0) as u32;
                                }
                            }
                            if self.avoid_cutout.load(Ordering::Acquire) && sdk >= 28 {
                                let cutout = env
                                    .call_method(
                                        &insets,
                                        "getDisplayCutout",
                                        "()Landroid/view/DisplayCutout;",
                                        &[],
                                    )?
                                    .l()?;
                                if !cutout.is_null() {
                                    for (i, method) in [
                                        "getSafeInsetLeft",
                                        "getSafeInsetTop",
                                        "getSafeInsetRight",
                                        "getSafeInsetBottom",
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    {
                                        safe[i] = safe[i].max(
                                            env.call_method(&cutout, method, "()I", &[])?
                                                .i()?
                                                .max(0)
                                                as u32,
                                        );
                                    }
                                }
                            }
                        }
                        Ok(())
                    })();
                    if result.is_err() {
                        let _ = env.exception_clear();
                    }
                }
                if safe != [0; 4] {
                    let location = env.new_int_array(2)?;
                    env.call_method(
                        &self.native,
                        "getLocationOnScreen",
                        "([I)V",
                        &[JValue::Object(&location)],
                    )?;
                    let mut origin = [0; 2];
                    env.get_int_array_region(location, 0, &mut origin)?;
                    let width = env
                        .call_method(&self.native, "getWidth", "()I", &[])?
                        .i()?
                        .max(0);
                    let height = env
                        .call_method(&self.native, "getHeight", "()I", &[])?
                        .i()?
                        .max(0);
                    let window = env
                        .call_method(&self.activity, "getWindow", "()Landroid/view/Window;", &[])?
                        .l()?;
                    let decor = env
                        .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?
                        .l()?;
                    let dw = env
                        .call_method(&decor, "getWidth", "()I", &[])?
                        .i()?
                        .max(width);
                    let dh = env
                        .call_method(&decor, "getHeight", "()I", &[])?
                        .i()?
                        .max(height);
                    safe[0] = safe[0].saturating_sub(origin[0].max(0) as u32);
                    safe[1] = safe[1].saturating_sub(origin[1].max(0) as u32);
                    safe[2] = safe[2].saturating_sub((dw - origin[0] - width).max(0) as u32);
                    safe[3] = safe[3].saturating_sub((dh - origin[1] - height).max(0) as u32);
                }
                winit::platform::android::set_native_content_insets(safe);
            }
            Command::Capture(capture) => {
                self.hide_system_pointer(env);
                if capture {
                    env.call_method(&self.edit, "clearFocus", "()V", &[])?;
                    env.call_method(&self.native, "setFocusable", "(Z)V", &[JValue::Bool(1)])?;
                    env.call_method(
                        &self.native,
                        "setFocusableInTouchMode",
                        "(Z)V",
                        &[JValue::Bool(1)],
                    )?;
                    env.call_method(&self.native, "requestFocus", "()Z", &[])?;
                    env.call_method(&self.native, "requestPointerCapture", "()V", &[])?;
                } else {
                    env.call_method(&self.native, "releasePointerCapture", "()V", &[])?;
                }
            }
            Command::HidePointer => {
                self.hide_system_pointer(env);
            }
        }
        Ok(())
    }
}
unsafe extern "C" fn callback(_: i32, _: i32, data: *mut std::ffi::c_void) -> i32 {
    let ui = unsafe { &*data.cast::<Ui>() };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback_inner(ui))).unwrap_or_else(
        |_| {
            ui.failed.store(true, Ordering::Release);
            ui.active.store(false, Ordering::Release);
            winit::platform::android::set_native_text_input_active(false);
            log::error!("Android keyboard UI callback failed");
            0
        },
    )
}
fn callback_inner(ui: &Ui) -> i32 {
    let mut bytes = [0; 256];
    while ui
        .reader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .read(&mut bytes)
        .is_ok_and(|n| n > 0)
    {}
    let commands: Vec<_> = ui
        .commands
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .drain(..)
        .collect();
    match ui.vm.attach_current_thread() {
        Ok(mut env) => {
            for c in commands {
                if let Err(e) = env.with_local_frame(16, |env| ui.apply(env, c)) {
                    let _ = env.exception_clear();
                    log::warn!("IME command: {e}");
                    continue;
                }
            }
        }
        Err(e) => {
            log::error!("IME attach: {e}");
            ui.failed.store(true, Ordering::Release);
            ui.active.store(false, Ordering::Release);
            winit::platform::android::set_native_text_input_active(false);
            return 0;
        }
    }
    1
}
pub fn show(text: &str) -> bool {
    show_mode(text, false)
}
pub fn show_secret(text: &str) -> bool {
    show_mode(text, true)
}
fn show_mode(text: &str, secret: bool) -> bool {
    if let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        winit::platform::android::set_native_text_input_active(true);
        // Do not diff an earlier editor buffer while Show is queued on the UI thread.
        ui.ready.store(false, Ordering::Release);
        ui.active.store(true, Ordering::Release);
        ui.enqueue(Command::Show(text.into(), secret));
        true
    } else {
        false
    }
}
pub fn hide() {
    winit::platform::android::set_native_text_input_active(false);
    if let Some(ui) = UI.get() {
        ui.active.store(false, Ordering::Release);
        ui.enqueue(Command::Hide)
    }
}
pub fn text() -> Option<String> {
    let ui = UI.get()?;
    if !ui.active.load(Ordering::Acquire) || !ui.ready.load(Ordering::Acquire) {
        return None;
    }
    if !ui.polling.swap(true, Ordering::AcqRel) {
        ui.enqueue(Command::Poll)
    }
    Some(
        ui.text
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone(),
    )
}

/// View operations, including pointer capture, belong to the Java UI thread.
pub fn capture_mouse(capture: bool) -> bool {
    if let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        ui.enqueue(Command::Capture(capture));
        true
    } else {
        false
    }
}

pub fn hide_system_pointer_now() {
    if let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        ui.enqueue(Command::HidePointer);
    }
}

/// Launcher-only safe bounds; View geometry is read on the Android UI thread.
pub fn viewport_bottom() -> Option<u32> {
    let ui = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire))?;
    if !ui.bounds_pending.swap(true, Ordering::AcqRel) {
        ui.enqueue(Command::Bounds);
    }
    let bottom = ui.bottom.load(Ordering::Acquire);
    (bottom > 0).then(|| {
        ((bottom - winit::platform::android::native_content_origin()[1] as i32).max(0) as f64
            * winit::platform::android::native_input_scale()[1])
            .round() as u32
    })
}
pub fn pick_document() -> bool {
    let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) else {
        return false;
    };
    ui.enqueue(Command::ImportDocument);
    true
}
pub fn pick_document_tree() -> bool {
    let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) else {
        return false;
    };
    ui.enqueue(Command::ImportFolderTree);
    true
}
pub fn create_document(suggested_name: &str, mime_type: &str) -> bool {
    let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) else {
        return false;
    };
    ui.enqueue(Command::CreateDocument(
        suggested_name.to_owned(),
        mime_type.to_owned(),
    ));
    true
}

pub fn configure_screen(immersive: bool, avoid_cutout: bool, keep_screen_on: bool) {
    if let Some(ui) = UI.get() {
        ui.enqueue(Command::Screen(immersive, avoid_cutout, keep_screen_on));
    }
}
pub fn refresh_bounds() {
    if let Some(ui) = UI.get().filter(|ui| {
        ui.avoid_cutout.load(Ordering::Acquire) || !ui.immersive.load(Ordering::Acquire)
    }) {
        let mut last = ui
            .last_bounds
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if last.is_none_or(|t| t.elapsed() >= std::time::Duration::from_millis(250))
            && !ui.bounds_pending.swap(true, Ordering::AcqRel)
        {
            *last = Some(std::time::Instant::now());
            ui.enqueue(Command::Bounds);
        }
    }
}

/// Whether the optional experimental control editor was included in this APK.
pub fn zalith_studio_available() -> bool {
    let Some(ui) = UI.get() else { return false; };
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let Ok(mut env) = ui.vm.attach_current_thread() else { return false; };
        match env.call_method(&ui.activity, "hasZalithStudio", "()Z", &[]).and_then(|v| v.z()) {
            Ok(value) => value,
            Err(_) => { let _ = env.exception_clear(); false }
        }
    })
}

/// The Android activity checks the layout path is private and of the right type.
pub fn open_zalith_studio(path: &std::path::Path) -> bool {
    let Some(ui) = UI.get() else { return false; };
    let Ok(mut env) = ui.vm.attach_current_thread() else { return false; };
    let result = env.with_local_frame(8, |env| -> jni::errors::Result<bool> {
        let path = JObject::from(env.new_string(path.to_string_lossy().as_ref())?);
        env.call_method(&ui.activity, "openZalithStudio", "(Ljava/lang/String;)Z", &[JValue::Object(&path)])?.z()
    });
    match result { Ok(value) => value, Err(_) => { let _ = env.exception_clear(); false } }
}

/// Start or stop the wallpaper decoder; it performs GL/media work on its own looper.
pub fn wallpaper_start(path: &std::path::Path, source: [u32; 2], output: [u32; 2]) -> Result<(), String> {
    let ui = UI.get().ok_or("Android activity is not ready")?;
    let mut env = ui.vm.attach_current_thread().map_err(|e| e.to_string())?;
    let result = env.with_local_frame(8, |env| -> jni::errors::Result<()> {
        let path = JObject::from(env.new_string(path.to_string_lossy().as_ref())?);
        env.call_method(&ui.activity, "startWallpaper", "(Ljava/lang/String;IIII)V",
            &[JValue::Object(&path), JValue::Int(source[0] as i32), JValue::Int(source[1] as i32),
              JValue::Int(output[0] as i32), JValue::Int(output[1] as i32)])?;
        Ok(())
    });
    if result.is_err() { let _ = env.exception_clear(); }
    result.map_err(|e| e.to_string())
}

pub fn wallpaper_stop() {
    if let Some(ui) = UI.get() && let Ok(mut env) = ui.vm.attach_current_thread() {
        if env.call_method(&ui.activity, "stopWallpaper", "()V", &[]).is_err() { let _ = env.exception_clear(); }
    }
}

/// Copy one native-resolution frame while the decoder's single buffer is held.
pub fn wallpaper_frame(expected: usize, recycled: &mut Vec<u8>) -> Option<Vec<u8>> {
    let ui = UI.get()?;
    let mut env = ui.vm.attach_current_thread().ok()?;
    let result = env.with_local_frame(8, |env| -> jni::errors::Result<Option<Vec<u8>>> {
        let obj = env.call_method(&ui.activity, "acquireWallpaperFrame", "()Ljava/nio/ByteBuffer;", &[])?.l()?;
        if obj.is_null() { return Ok(None); }
        let buffer = jni::objects::JByteBuffer::from(obj);
        let copy = match (env.get_direct_buffer_address(&buffer), env.get_direct_buffer_capacity(&buffer)) {
            (Ok(address), Ok(capacity)) if capacity == expected && expected <= 40_000_000 => {
                // The decoder cannot mutate the direct buffer until releaseWallpaperFrame.
                recycled.resize(capacity, 0);
                recycled.copy_from_slice(unsafe { std::slice::from_raw_parts(address, capacity) });
                Some(std::mem::take(recycled))
            }
            _ => None,
        };
        env.call_method(&ui.activity, "releaseWallpaperFrame", "()V", &[])?;
        Ok(copy)
    });
    match result { Ok(frame) => frame, Err(_) => { let _ = env.exception_clear(); None } }
}

pub fn wallpaper_probe_size(path: &std::path::Path) -> [u32; 2] {
    let Some(ui) = UI.get() else {
        return [1280, 720];
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return [1280, 720];
    };
    let result = env.with_local_frame(8, |env| -> jni::errors::Result<[u32; 2]> {
        let path = JObject::from(env.new_string(path.to_string_lossy().as_ref())?);
        let arr_obj = env
            .call_method(
                &ui.activity,
                "probeWallpaperSize",
                "(Ljava/lang/String;)[I",
                &[JValue::Object(&path)],
            )?
            .l()?;
        if arr_obj.is_null() {
            return Ok([1280, 720]);
        }
        let arr = jni::objects::JIntArray::from(arr_obj);
        let mut out = [1280i32, 720i32];
        env.get_int_array_region(&arr, 0, &mut out)?;
        Ok([
            u32::try_from(out[0]).unwrap_or(1280).max(1),
            u32::try_from(out[1]).unwrap_or(720).max(1),
        ])
    });
    match result {
        Ok(size) => size,
        Err(_) => {
            let _ = env.exception_clear();
            [1280, 720]
        }
    }
}

pub fn wallpaper_status() -> Option<String> {
    let ui = UI.get()?;
    let mut env = ui.vm.attach_current_thread().ok()?;
    let result = env.with_local_frame(8, |env| -> jni::errors::Result<String> {
        let obj = env.call_method(&ui.activity, "wallpaperStatus", "()Ljava/lang/String;", &[])?.l()?;
        Ok(env.get_string(&jni::objects::JString::from(obj))?.into())
    });
    match result { Ok(status) => Some(status), Err(_) => { let _ = env.exception_clear(); None } }
}

fn music_call_void(method: &str) {
    let Some(ui) = UI.get() else {
        return;
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return;
    };
    if env.call_method(&ui.activity, method, "()V", &[]).is_err() {
        let _ = env.exception_clear();
    }
}

fn music_play_queue(tracks: &[horizon_client::internet_music::MusicTrack], start_index: usize, playlist_name: &str) {
    let Some(ui) = UI.get() else {
        return;
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return;
    };
    let res = env.with_local_frame(24, |env| -> jni::errors::Result<()> {
        let n = tracks.len() as i32;
        let ids = env.new_object_array(n, "java/lang/String", JObject::null())?;
        let urls = env.new_object_array(n, "java/lang/String", JObject::null())?;
        let fallbacks = env.new_object_array(n, "java/lang/String", JObject::null())?;
        let titles = env.new_object_array(n, "java/lang/String", JObject::null())?;
        let artists = env.new_object_array(n, "java/lang/String", JObject::null())?;
        let durs_vec: Vec<i64> = tracks
            .iter()
            .map(|t| (t.duration_secs.saturating_mul(1000)) as i64)
            .collect();
        let durs = env.new_long_array(n)?;
        env.set_long_array_region(&durs, 0, &durs_vec)?;
        for (i, t) in tracks.iter().enumerate() {
            let idx = i as i32;
            let id = env.new_string(&t.id)?;
            let u = env.new_string(&t.stream_url)?;
            let fb = env.new_string(t.fallback_urls.join("\n"))?;
            let ti = env.new_string(&t.title)?;
            let ar = env.new_string(&t.artist)?;
            env.set_object_array_element(&ids, idx, id)?;
            env.set_object_array_element(&urls, idx, u)?;
            env.set_object_array_element(&fallbacks, idx, fb)?;
            env.set_object_array_element(&titles, idx, ti)?;
            env.set_object_array_element(&artists, idx, ar)?;
        }
        let pl = env.new_string(playlist_name)?;
        let ids_obj = JObject::from(ids);
        let urls_obj = JObject::from(urls);
        let fallbacks_obj = JObject::from(fallbacks);
        let titles_obj = JObject::from(titles);
        let artists_obj = JObject::from(artists);
        let durs_obj = JObject::from(durs);
        let pl_obj = JObject::from(pl);
        env.call_method(
            &ui.activity,
            "musicPlayQueue",
            "([Ljava/lang/String;[Ljava/lang/String;[Ljava/lang/String;[Ljava/lang/String;[Ljava/lang/String;[JILjava/lang/String;)V",
            &[
                JValue::Object(&ids_obj),
                JValue::Object(&urls_obj),
                JValue::Object(&fallbacks_obj),
                JValue::Object(&titles_obj),
                JValue::Object(&artists_obj),
                JValue::Object(&durs_obj),
                JValue::Int(start_index as i32),
                JValue::Object(&pl_obj),
            ],
        )?;
        Ok(())
    });
    if res.is_err() {
        let _ = env.exception_clear();
    }
}

fn music_toggle_pause() {
    music_call_void("musicTogglePause");
}

fn music_next() {
    music_call_void("musicNext");
}

fn music_prev() {
    music_call_void("musicPrev");
}

fn music_seek_by_ms(delta_ms: i64) {
    let Some(ui) = UI.get() else {
        return;
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return;
    };
    if env
        .call_method(&ui.activity, "musicSeekBy", "(J)V", &[JValue::Long(delta_ms)])
        .is_err()
    {
        let _ = env.exception_clear();
    }
}

fn music_seek_to_ms(pos_ms: u64) {
    let Some(ui) = UI.get() else {
        return;
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return;
    };
    if env
        .call_method(
            &ui.activity,
            "musicSeekTo",
            "(J)V",
            &[JValue::Long(pos_ms as i64)],
        )
        .is_err()
    {
        let _ = env.exception_clear();
    }
}

fn music_set_volume(vol: f32) {
    let Some(ui) = UI.get() else {
        return;
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return;
    };
    if env
        .call_method(&ui.activity, "musicSetVolume", "(F)V", &[JValue::Float(vol)])
        .is_err()
    {
        let _ = env.exception_clear();
    }
}

fn music_set_muted(muted: bool) {
    let Some(ui) = UI.get() else {
        return;
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return;
    };
    if env
        .call_method(
            &ui.activity,
            "musicSetMuted",
            "(Z)V",
            &[JValue::Bool(u8::from(muted))],
        )
        .is_err()
    {
        let _ = env.exception_clear();
    }
}

fn music_stop() {
    music_call_void("musicStop");
}

fn music_snapshot() -> horizon_client::internet_music::MusicStatusSnapshot {
    use horizon_client::internet_music::MusicStatusSnapshot;
    let Some(ui) = UI.get() else {
        return MusicStatusSnapshot::default();
    };
    let Ok(mut env) = ui.vm.attach_current_thread() else {
        return MusicStatusSnapshot::default();
    };
    let res = env.with_local_frame(8, |env| -> jni::errors::Result<String> {
        let obj = env
            .call_method(&ui.activity, "musicSnapshot", "()Ljava/lang/String;", &[])?
            .l()?;
        Ok(env.get_string(&jni::objects::JString::from(obj))?.into())
    });
    let Ok(raw) = res else {
        let _ = env.exception_clear();
        return MusicStatusSnapshot::default();
    };
    let parts: Vec<&str> = raw.split('\t').collect();
    if parts.len() < 13 {
        return MusicStatusSnapshot::default();
    }
    MusicStatusSnapshot {
        playing: parts[0] == "1",
        buffering: parts[1] == "1",
        muted: parts[2] == "1",
        volume: parts[3].parse::<f32>().unwrap_or(0.85).clamp(0.0, 1.0),
        position_ms: parts[4].parse::<u64>().unwrap_or(0),
        duration_ms: parts[5].parse::<u64>().unwrap_or(0),
        queue_index: parts[6].parse::<usize>().unwrap_or(0),
        queue_len: parts[7].parse::<usize>().unwrap_or(0),
        active_playlist: parts[8].to_owned(),
        current_title: parts[9].to_owned(),
        current_artist: parts[10].to_owned(),
        current_url: parts[11].to_owned(),
        status: parts[12].to_owned(),
    }
}

pub fn music_install_backend() {
    horizon_client::internet_music::install_backend(
        horizon_client::internet_music::MusicBackend {
            play_queue: music_play_queue,
            toggle_pause: music_toggle_pause,
            next: music_next,
            prev: music_prev,
            seek_by_ms: music_seek_by_ms,
            seek_to_ms: music_seek_to_ms,
            set_volume: music_set_volume,
            set_muted: music_set_muted,
            stop: music_stop,
            snapshot: music_snapshot,
        },
    );
}
