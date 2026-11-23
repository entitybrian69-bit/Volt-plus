//! The screen recorder's options and the state the in-game overlay draws.
//!
//! The capture itself is Android's: `dev.horizon.android.ScreenRecorder` asks for the
//! projection and `RecorderService` records, through the JNI bridge in
//! `horizon-android`'s `android/recorder.rs`. This module is plain data, so the touch UI can
//! lay the controls out (and show what the last check said) without touching Android.
use std::sync::{Mutex, OnceLock};

/// The heights the overlay offers, tallest label first: the width follows the display.
pub const RESOLUTIONS: [(&str, u32); 6] = [
    ("240p", 240),
    ("360p", 360),
    ("480p", 480),
    ("720p", 720),
    ("1080p", 1080),
    ("1440p", 1440),
];
/// Every frame rate the overlay offers; the encoder decides which it can keep.
pub const FRAME_RATES: [u32; 6] = [30, 60, 90, 120, 144, 165];
/// The audio choices, in the order `ScreenRecorder` numbers them.
pub const AUDIO: [&str; 3] = ["No audio", "Microphone", "In-game audio"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// The system's screen-capture prompt is up.
    Asking,
    /// Consent was given and the service is opening the encoder.
    Starting,
    Recording,
    Stopping,
    /// The last recording finished; the message names where it went.
    Saved,
    Denied,
    Error,
    /// The requested combination (or audio source) is not available on this device.
    Unsupported,
}

impl Phase {
    pub fn active(self) -> bool {
        matches!(
            self,
            Phase::Asking | Phase::Starting | Phase::Recording | Phase::Stopping
        )
    }
    /// Whether the overlay should offer Start: nothing else is running.
    pub fn startable(self) -> bool {
        !self.active()
    }
    pub fn label(self) -> &'static str {
        match self {
            Phase::Idle => "Ready",
            Phase::Asking => "Waiting for permission",
            Phase::Starting => "Starting",
            Phase::Recording => "Recording",
            Phase::Stopping => "Saving",
            Phase::Saved => "Saved",
            Phase::Denied => "Permission declined",
            Phase::Error => "Failed",
            Phase::Unsupported => "Not supported",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Index into [`RESOLUTIONS`].
    pub resolution: usize,
    /// Index into [`FRAME_RATES`].
    pub fps: usize,
    /// Index into [`AUDIO`].
    pub audio: usize,
    /// Record the touch controls and the HUD into the video. Off records raw
    /// gameplay: the overlay is hidden for as long as the capture runs, so the
    /// video has no controls in it.
    pub record_controls: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            resolution: 3,
            fps: 1,
            audio: 0,
            // The controls stay on screen and in the video until the player asks
            // for a raw recording: nothing hides the HUD behind their back.
            record_controls: true,
        }
    }
}

impl Settings {
    pub fn height(&self) -> u32 {
        RESOLUTIONS[self.resolution.min(RESOLUTIONS.len() - 1)].1
    }
    pub fn frames(&self) -> u32 {
        FRAME_RATES[self.fps.min(FRAME_RATES.len() - 1)]
    }
    pub fn audio_label(&self) -> &'static str {
        AUDIO[self.audio.min(AUDIO.len() - 1)]
    }
}

/// One finished recording, as the Recordings list shows it.
#[derive(Clone, Debug, Default)]
pub struct Recording {
    pub name: String,
    pub bytes: u64,
    /// When it was written, in milliseconds since the Unix epoch.
    pub added_ms: u64,
    pub uri: String,
}

/// What this device can do, as the recorder last checked it.
#[derive(Clone, Debug)]
pub struct Capability {
    pub sdk: i32,
    pub display: (u32, u32),
    pub game_audio: bool,
    pub microphone: bool,
    pub projection: bool,
    pub device: String,
    pub checked: bool,
}

impl Default for Capability {
    fn default() -> Self {
        Self {
            sdk: 0,
            display: (0, 0),
            game_audio: false,
            microphone: false,
            projection: false,
            device: String::new(),
            checked: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Status {
    pub phase: Phase,
    pub millis: u64,
    pub bytes: u64,
    pub uri: String,
    pub message: String,
    /// The size and rate the recorder actually runs at (after fitting to the encoder).
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub recordings: Vec<Recording>,
    pub capability: Capability,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            millis: 0,
            bytes: 0,
            uri: String::new(),
            message: String::new(),
            width: 0,
            height: 0,
            fps: 0,
            recordings: Vec::new(),
            capability: Capability::default(),
        }
    }
}

static SETTINGS: OnceLock<Mutex<Settings>> = OnceLock::new();
static STATUS: OnceLock<Mutex<Status>> = OnceLock::new();

static PREVIOUS_HIDDEN: OnceLock<Mutex<Option<bool>>> = OnceLock::new();

/// Keeps the game's own "hide the HUD" flag in step with a raw recording: hidden while the
/// capture runs, and exactly as the player left it afterwards. Returns the value to apply,
/// or `None` when nothing has to change.
pub fn sync_controls(hide_gui: bool) -> Option<bool> {
    let hiding = hiding_controls();
    let mut previous = PREVIOUS_HIDDEN
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    match (hiding, *previous) {
        (true, None) => {
            *previous = Some(hide_gui);
            (hide_gui != true).then_some(true)
        }
        (false, Some(was)) => {
            *previous = None;
            (hide_gui != was).then_some(was)
        }
        _ => None,
    }
}

fn settings_lock() -> &'static Mutex<Settings> {
    SETTINGS.get_or_init(|| Mutex::new(Settings::default()))
}

fn status_lock() -> &'static Mutex<Status> {
    STATUS.get_or_init(|| Mutex::new(Status::default()))
}

pub fn settings() -> Settings {
    *settings_lock().lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_settings(next: Settings) {
    *settings_lock().lock().unwrap_or_else(|e| e.into_inner()) = next;
}

pub fn update_settings(f: impl FnOnce(&mut Settings)) {
    let mut guard = settings_lock().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard);
}

pub fn status() -> Status {
    status_lock().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Replaces the status the overlay draws (the bridge publishes what Android reports).
pub fn publish(next: Status) {
    *status_lock().lock().unwrap_or_else(|e| e.into_inner()) = next;
}

/// Whether the touch controls and HUD should be kept off screen for the recording.
///
/// The player always keeps full visibility of controls, hand, items, and HUD on screen;
/// raw gameplay capture (`record_controls` on with no audio) copies HUD-free frames directly
/// into the encoder surface via [`capture_target`] without hiding anything on screen.
pub fn hiding_controls() -> bool {
    false
}

/// The encoder's own surface while the engine records its frames: the `ANativeWindow` the
/// renderer copies each HUD-free frame into, the size to render it at, and a serial that
/// names this attachment (the renderer compares it to know when to build a new surface).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureTarget {
    pub window: usize,
    pub width: u32,
    pub height: u32,
    pub serial: u64,
}

static CAPTURE_TARGET: OnceLock<Mutex<Option<CaptureTarget>>> = OnceLock::new();
static CAPTURE_SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The capture the game should render into, when the recorder asked for the engine's frames.
pub fn capture_target() -> Option<CaptureTarget> {
    *CAPTURE_TARGET
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Points the renderer at a new capture, or clears it. Answers with what was attached before,
/// so the caller can free it.
pub fn set_capture_target(next: Option<CaptureTarget>) -> Option<CaptureTarget> {
    let mut guard = CAPTURE_TARGET
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::mem::replace(&mut guard, next)
}

/// A serial no earlier capture used, so the renderer rebuilds its surface exactly once.
pub fn next_capture_serial() -> u64 {
    CAPTURE_SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// `1.2 MB`, `930 kB`: a recording's size as the list shows it.
pub fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1_048_576.0;
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.0} kB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

/// `04:12`: a recording's length, from its milliseconds.
pub fn format_duration(millis: u64) -> String {
    let seconds = millis / 1000;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_match_the_overlay() {
        assert_eq!(RESOLUTIONS.len(), 6);
        assert_eq!(FRAME_RATES.len(), 6);
        assert_eq!(AUDIO.len(), 3);
        let defaults = Settings::default();
        assert_eq!(defaults.height(), 720);
        assert_eq!(defaults.frames(), 60);
        assert!(defaults.audio_label().eq("No audio"));
    }

    #[test]
    fn sizes_and_lengths_read_like_the_list() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024 * 900), "900 kB");
        assert_eq!(format_bytes(1024 * 1024 * 3 / 2), "1.5 MB");
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(252_000), "04:12");
        assert_eq!(format_duration(3_723_000), "62:03");
    }

    #[test]
    fn the_settings_are_clamped_to_the_offered_choices() {
        let out_of_range = Settings {
            resolution: 99,
            fps: 99,
            audio: 99,
            record_controls: true,
        };
        assert_eq!(out_of_range.height(), 1440);
        assert_eq!(out_of_range.frames(), 165);
        assert_eq!(out_of_range.audio_label(), "In-game audio");
    }

    #[test]
    fn running_phases_are_not_startable() {
        for phase in [
            Phase::Asking,
            Phase::Starting,
            Phase::Recording,
            Phase::Stopping,
        ] {
            assert!(phase.active() && !phase.startable(), "{phase:?}");
        }
        for phase in [Phase::Idle, Phase::Saved, Phase::Error, Phase::Denied] {
            assert!(!phase.active() && phase.startable(), "{phase:?}");
        }
    }
}
