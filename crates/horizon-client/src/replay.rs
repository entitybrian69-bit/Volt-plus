//! Native Replay Mod for Horizon / Volt Plus (`.mcpr` + `.tmcpr`):
//! - Records both Singleplayer (integrated server pipe) and Multiplayer sessions
//! - Synthesizes local player ("self") movement, swings, sneaking & equipment
//! - Streams packets directly to disk with CRC32 and packages `.mcpr` ZIP archives
//! - Plays back `.mcpr` files with fast forward/backward seek, free-cam + roll + FOV,
//!   entity spectating, time/weather overrides, Catmull-Rom keyframe camera paths,
//!   3D path gizmos, and offline HUD-less MP4 path rendering.

pub mod archive;
pub mod playback;
pub mod record;
pub mod timeline;

pub use archive::{
    MCPR_EXT, REPLAYS_DIR, ReplayMetadata, ReplaySummary, delete_replay, format_duration_ms,
    format_size_bytes, list_replays, read_mcpr_summary, rename_replay, replays_dir,
    sanitize_replay_stem, unique_mcpr_path,
};
pub use playback::{CameraMode, ReplayPlayer, ReplayRenderJob, ReplayStep, WeatherOverride};
pub use record::{
    EXPORT_FPS, EXPORT_RESOLUTIONS, RecordingStatusSnapshot, ReplayRecorder, ReplaySettings,
    init_settings, settings, update_settings,
};
pub use timeline::{
    CameraPose, Interpolation, PositionKeyframe, ReplayMarker, ReplayTimeline, ReplayTimelineSet,
    TimeKeyframe,
};

use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct ReplayPlayerRow {
    pub entity_id: u32,
    pub name: String,
    pub pos: [f64; 3],
}

/// Read-only snapshot published each frame by `Game` so `touch_ui` can render
/// the Replay Editor timeline, keyframe controls, drawers, and render modal.
#[derive(Clone, Debug, Default)]
pub struct ReplayHudSnapshot {
    pub active: bool,
    pub title: String,
    pub playing: bool,
    pub speed: f32,
    pub loop_playback: bool,
    pub current_ms: u64,
    pub duration_ms: u64,
    pub path_cursor_ms: u64,
    pub path_duration_ms: u64,
    pub path_playing: bool,
    pub sync_time_with_path: bool,
    pub cam_pos: [f64; 3],
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    pub cam_roll: f32,
    pub cam_fov: f32,
    pub fly_speed: f32,
    pub spectating_entity: Option<u32>,
    pub show_hud: bool,
    pub show_path_gizmos: bool,
    pub hide_nametags: bool,
    pub hide_players: bool,
    pub interpolation: Interpolation,
    pub time_override: Option<i64>,
    pub weather_override: Option<WeatherOverride>,
    pub players_drawer_open: bool,
    pub markers_drawer_open: bool,
    pub render_modal_open: bool,
    pub position_keyframes: Vec<PositionKeyframe>,
    pub time_keyframes: Vec<TimeKeyframe>,
    pub markers: Vec<ReplayMarker>,
    pub players: Vec<ReplayPlayerRow>,
    pub banner: Option<String>,
    pub render_job: Option<ReplayRenderJob>,
}

static GLOBAL_RECORDER: OnceLock<ReplayRecorder> = OnceLock::new();
static HUD_SNAPSHOT: OnceLock<Mutex<ReplayHudSnapshot>> = OnceLock::new();

pub fn global_recorder() -> &'static ReplayRecorder {
    GLOBAL_RECORDER.get_or_init(ReplayRecorder::new)
}

pub fn publish_hud_snapshot(snapshot: ReplayHudSnapshot) {
    let lock = HUD_SNAPSHOT.get_or_init(|| Mutex::new(ReplayHudSnapshot::default()));
    *lock.lock().unwrap_or_else(|e| e.into_inner()) = snapshot;
}

pub fn hud_snapshot() -> ReplayHudSnapshot {
    HUD_SNAPSHOT
        .get_or_init(|| Mutex::new(ReplayHudSnapshot::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Platform hooks for Replay Mod deterministic MP4 path export (registered on Android).
#[derive(Clone, Copy)]
pub struct VideoExportCallbacks {
    pub start: fn(width: u32, height: u32, fps: u32, bitrate_mbps: u32, name: &str) -> Result<String>,
    pub push_frame: fn(rgba: &[u8], width: u32, height: u32, pts_us: u64) -> Result<()>,
    pub finish: fn() -> Result<String>,
    pub cancel: fn(),
}

static VIDEO_EXPORTER: OnceLock<VideoExportCallbacks> = OnceLock::new();

pub fn register_video_exporter(callbacks: VideoExportCallbacks) {
    let _ = VIDEO_EXPORTER.set(callbacks);
}

pub fn video_export_supported() -> bool {
    VIDEO_EXPORTER.get().is_some()
}

pub fn video_export_start(
    width: u32,
    height: u32,
    fps: u32,
    bitrate_mbps: u32,
    name: &str,
) -> Result<String> {
    let cb = VIDEO_EXPORTER
        .get()
        .ok_or("Replay MP4 export is not registered on this platform")?;
    (cb.start)(width, height, fps, bitrate_mbps, name)
}

pub fn video_export_push_frame(
    rgba: &[u8],
    width: u32,
    height: u32,
    pts_us: u64,
) -> Result<()> {
    let cb = VIDEO_EXPORTER
        .get()
        .ok_or("Replay MP4 export is not registered on this platform")?;
    (cb.push_frame)(rgba, width, height, pts_us)
}

pub fn video_export_finish() -> Result<String> {
    let cb = VIDEO_EXPORTER
        .get()
        .ok_or("Replay MP4 export is not registered on this platform")?;
    (cb.finish)()
}

pub fn video_export_cancel() {
    if let Some(cb) = VIDEO_EXPORTER.get() {
        (cb.cancel)();
    }
}

/// Every action the Replay Viewer HUD or In-Game Replay controls can trigger.
#[derive(Clone, Debug, PartialEq)]
pub enum ReplayAction {
    TogglePlay,
    StepSeconds(i64),
    SeekMs(u64),
    CycleSpeed(i32),
    ToggleLoop,
    AddPosKeyframe,
    AddTimeKeyframe,
    AddSyncedKeyframe,
    RemoveNearestKeyframe,
    AddMarker,
    JumpToMarker(usize),
    TogglePathPreview,
    ToggleSyncTime,
    ToggleInterpolation,
    ClearPath,
    SaveEdits,
    TogglePathGizmos,
    ToggleHideHud,
    ToggleHideNametags,
    ToggleHidePlayers,
    CycleTimeOverride,
    CycleWeatherOverride,
    AdjustRoll(f32),
    ResetRoll,
    AdjustFov(f32),
    AdjustFlySpeed(f32),
    TogglePlayersDrawer,
    ToggleMarkersDrawer,
    SpectateEntity(Option<u32>),
    TeleportToPos([f64; 3]),
    ToggleRenderModal,
    StartRenderMp4 {
        width: u32,
        height: u32,
        fps: u32,
        bitrate_mbps: u32,
    },
    CancelRenderMp4,
    ExitReplay,
    // Live recording actions during SP/MP gameplay
    LiveStartRecording,
    LiveTogglePauseRecording,
    LiveAddBookmark,
    LiveSaveRecording,
    LiveDiscardRecording,
    LiveToggleAutoSp,
    LiveToggleAutoMp,
    LiveToggleRecordSelf,
    LiveToggleBadge,
}

