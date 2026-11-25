//! Replay Mod settings and live packet-stream recorder (Singleplayer + Multiplayer).

use super::{
    archive::{
        Crc32, ReplayMetadata, encode_raw_body, replays_dir, unique_mcpr_path, write_mcpr_archive,
        write_tmcpr_record,
    },
    timeline::{ReplayMarker, ReplayTimelineSet},
};
use crate::Result;
use horizon_protocol::packets::play;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};

pub const SETTINGS_FILE: &str = "replay-settings.json";
pub const EXPORT_RESOLUTIONS: [(&str, u32, u32); 3] = [
    ("720p (1280x720)", 1280, 720),
    ("1080p (1920x1080)", 1920, 1080),
    ("1440p (2560x1440)", 2560, 1440),
];
pub const EXPORT_FPS: [u32; 2] = [30, 60];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplaySettings {
    pub auto_record_sp: bool,
    pub auto_record_mp: bool,
    pub record_self: bool,
    pub show_recording_badge: bool,
    pub export_resolution: usize,
    pub export_fps: usize,
    pub export_bitrate_mbps: u32,
}

impl Default for ReplaySettings {
    fn default() -> Self {
        Self {
            auto_record_sp: true,
            auto_record_mp: true,
            record_self: true,
            show_recording_badge: true,
            export_resolution: 1, // 1080p
            export_fps: 1,        // 60 fps
            export_bitrate_mbps: 18,
        }
    }
}

impl ReplaySettings {
    pub fn load(game_dir: &Path) -> Self {
        let path = game_dir.join(SETTINGS_FILE);
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, game_dir: &Path) {
        let path = game_dir.join(SETTINGS_FILE);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn export_dimensions(&self) -> (u32, u32) {
        let entry = EXPORT_RESOLUTIONS
            .get(self.export_resolution)
            .unwrap_or(&EXPORT_RESOLUTIONS[1]);
        (entry.1, entry.2)
    }

    pub fn export_frame_rate(&self) -> u32 {
        *EXPORT_FPS.get(self.export_fps).unwrap_or(&60)
    }
}

static SHARED_SETTINGS: OnceLock<Mutex<ReplaySettings>> = OnceLock::new();

fn settings_lock() -> &'static Mutex<ReplaySettings> {
    SHARED_SETTINGS.get_or_init(|| Mutex::new(ReplaySettings::default()))
}

pub fn init_settings(game_dir: &Path) {
    let loaded = ReplaySettings::load(game_dir);
    *settings_lock().lock().unwrap_or_else(|e| e.into_inner()) = loaded;
}

pub fn settings() -> ReplaySettings {
    settings_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

pub fn update_settings(game_dir: &Path, f: impl FnOnce(&mut ReplaySettings)) {
    let mut guard = settings_lock().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard);
    guard.save(game_dir);
}

#[derive(Clone, Debug)]
pub struct RecordingStatusSnapshot {
    pub active: bool,
    pub paused: bool,
    pub elapsed_ms: u64,
    pub packet_count: u64,
    pub bytes_written: u64,
    pub markers_count: usize,
    pub label: String,
    pub last_saved_message: Option<String>,
}

impl Default for RecordingStatusSnapshot {
    fn default() -> Self {
        Self {
            active: false,
            paused: false,
            elapsed_ms: 0,
            packet_count: 0,
            bytes_written: 0,
            markers_count: 0,
            label: String::new(),
            last_saved_message: None,
        }
    }
}

struct RecorderInner {
    game_dir: PathBuf,
    temp_tmcpr: PathBuf,
    writer: BufWriter<File>,
    crc: Crc32,
    meta: ReplayMetadata,
    started_at: Instant,
    paused_since: Option<Instant>,
    accumulated_pause_ms: u64,
    last_timestamp_ms: u32,
    packet_count: u64,
    bytes_written: u64,
    players_seen: BTreeSet<u128>,
    markers: Vec<ReplayMarker>,
    in_configuration: bool,
    /// Last configuration packet bytes kept in case recording starts mid-session.
    config_prelude: Vec<Vec<u8>>,
    self_tracker: SelfPlayerTracker,
}

#[derive(Default)]
struct SelfPlayerTracker {
    spawned: bool,
    entity_id: i32,
    uuid: u128,
    name: String,
    player_type_id: Option<i32>,
    last_pos: [f64; 3],
    last_yaw: f32,
    last_pitch: f32,
    last_on_ground: bool,
    last_flags: u8,
    last_pose_id: i32,
    last_main_item: Option<i32>,
    last_off_item: Option<i32>,
    tick_counter: u32,
}

/// Thread-safe handle shared between `Game` and `net::Session`.
#[derive(Clone, Default)]
pub struct ReplayRecorder {
    inner: Arc<Mutex<Option<RecorderInner>>>,
    config_cache: Arc<Mutex<Vec<Vec<u8>>>>,
    last_status: Arc<Mutex<Option<String>>>,
}

impl ReplayRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    pub fn is_paused(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|r| r.paused_since.is_some())
    }

    pub fn status(&self) -> RecordingStatusSnapshot {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let last_msg = self
            .last_status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let Some(r) = guard.as_ref() else {
            return RecordingStatusSnapshot {
                last_saved_message: last_msg,
                ..Default::default()
            };
        };
        RecordingStatusSnapshot {
            active: true,
            paused: r.paused_since.is_some(),
            elapsed_ms: u64::from(r.current_timestamp_ms()),
            packet_count: r.packet_count,
            bytes_written: r.bytes_written,
            markers_count: r.markers.len(),
            label: r.meta.server_name.clone(),
            last_saved_message: last_msg,
        }
    }

    /// Clears cached configuration prelude on a fresh connection.
    pub fn reset_connection_prelude(&self) {
        self.config_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Starts recording a new `.mcpr` session.
    pub fn start(
        &self,
        game_dir: &Path,
        singleplayer: bool,
        label: &str,
        player_type_id: Option<i32>,
    ) -> Result<()> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_some() {
            return Ok(());
        }
        let dir = replays_dir(game_dir);
        fs::create_dir_all(&dir)?;
        let temp_tmcpr = dir.join(format!(
            ".active_{}_{}.tmcpr",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        let file = File::create(&temp_tmcpr)?;
        let mut inner = RecorderInner {
            game_dir: game_dir.to_path_buf(),
            temp_tmcpr,
            writer: BufWriter::with_capacity(64 * 1024, file),
            crc: Crc32::new(),
            meta: ReplayMetadata {
                singleplayer,
                server_name: if label.trim().is_empty() {
                    if singleplayer {
                        "Singleplayer".to_owned()
                    } else {
                        "Multiplayer".to_owned()
                    }
                } else {
                    label.trim().to_owned()
                },
                ..ReplayMetadata::default()
            },
            started_at: Instant::now(),
            paused_since: None,
            accumulated_pause_ms: 0,
            last_timestamp_ms: 0,
            packet_count: 0,
            bytes_written: 0,
            players_seen: BTreeSet::new(),
            markers: Vec::new(),
            in_configuration: true,
            config_prelude: Vec::new(),
            self_tracker: SelfPlayerTracker {
                player_type_id,
                ..Default::default()
            },
        };
        let cached = self.config_cache.lock().unwrap_or_else(|e| e.into_inner());
        for body in cached.iter() {
            inner.write_body_at(0, body)?;
        }
        *guard = Some(inner);
        *self.last_status.lock().unwrap_or_else(|e| e.into_inner()) =
            Some("Recording started".to_owned());
        Ok(())
    }

    pub fn toggle_pause(&self) -> bool {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(r) = guard.as_mut() else {
            return false;
        };
        if let Some(since) = r.paused_since.take() {
            r.accumulated_pause_ms += since.elapsed().as_millis() as u64;
            false
        } else {
            r.paused_since = Some(Instant::now());
            true
        }
    }

    pub fn add_marker(
        &self,
        name: Option<String>,
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
    ) -> Option<ReplayMarker> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let r = guard.as_mut()?;
        let time_ms = u64::from(r.current_timestamp_ms());
        let idx = r.markers.len() + 1;
        let marker = ReplayMarker {
            name: name.unwrap_or_else(|| format!("Bookmark #{idx}")),
            time_ms,
            x,
            y,
            z,
            yaw,
            pitch,
            roll: 0.0,
        };
        r.markers.push(marker.clone());
        *self.last_status.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(format!("Added {}", marker.name));
        Some(marker)
    }

    /// Records a raw configuration packet `(id, payload)` and caches it in the
    /// connection prelude so manual start mid-session still has valid registries.
    pub fn record_cfg_raw(&self, id: i32, payload: &[u8]) {
        let body = encode_raw_body(id, payload);
        {
            let mut cache = self.config_cache.lock().unwrap_or_else(|e| e.into_inner());
            if id == horizon_protocol::ids::configuration::clientbound::SELECT_KNOWN_PACKS
                || id == horizon_protocol::ids::configuration::clientbound::REGISTRY_DATA
                || id == horizon_protocol::ids::configuration::clientbound::UPDATE_TAGS
                || id == horizon_protocol::ids::configuration::clientbound::FINISH_CONFIGURATION
            {
                cache.push(body.clone());
            }
        }
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = guard.as_mut() {
            let ts = r.current_timestamp_ms();
            let _ = r.write_body_at(ts, &body);
            if id == horizon_protocol::ids::configuration::clientbound::FINISH_CONFIGURATION {
                r.in_configuration = false;
            }
        }
    }

    /// Records a raw play packet `(id, payload)` coming from the server/integrated server.
    pub fn record_play_raw(&self, id: i32, payload: &[u8]) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(r) = guard.as_mut() else {
            return;
        };
        if r.paused_since.is_some() {
            return;
        }
        let body = encode_raw_body(id, payload);
        let ts = r.current_timestamp_ms();
        let _ = r.write_body_at(ts, &body);
        if id == horizon_protocol::ids::play::clientbound::START_CONFIGURATION {
            r.in_configuration = true;
            r.self_tracker.spawned = false;
        }
    }

    pub fn record_player_uuid(&self, uuid: u128) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = guard.as_mut() {
            r.players_seen.insert(uuid);
        }
    }

    /// Synthesizes local player ("self") packets into the `.tmcpr` stream so
    /// the recording player appears in third-person inside the replay.
    #[allow(clippy::too_many_arguments)]
    pub fn sync_local_player(
        &self,
        entity_id: i32,
        uuid: u128,
        name: &str,
        pos: [f64; 3],
        yaw: f32,
        pitch: f32,
        on_ground: bool,
        sneaking: bool,
        sprinting: bool,
        main_hand_item: Option<i32>,
        off_hand_item: Option<i32>,
        swing_main: bool,
        swing_off: bool,
    ) {
        if !settings().record_self {
            return;
        }
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(r) = guard.as_mut() else {
            return;
        };
        if r.paused_since.is_some() || r.in_configuration {
            return;
        }
        let Some(player_type_id) = r.self_tracker.player_type_id else {
            return;
        };
        let replay_self_id = 1_000_000_000_i32.saturating_add(entity_id & 0x0FFF_FFFF);
        r.meta.self_id = replay_self_id;
        r.players_seen.insert(uuid);
        let ts = r.current_timestamp_ms();

        if !r.self_tracker.spawned || r.self_tracker.entity_id != replay_self_id {
            r.self_tracker.entity_id = replay_self_id;
            r.self_tracker.uuid = uuid;
            r.self_tracker.name = name.to_owned();
            r.self_tracker.spawned = true;
            r.self_tracker.last_pos = pos;
            r.self_tracker.last_yaw = yaw;
            r.self_tracker.last_pitch = pitch;
            r.self_tracker.last_on_ground = on_ground;

            let info = play::Clientbound::PlayerInfoUpdate(play::PlayerInfoUpdate {
                actions: 0x01 | 0x08, // ADD_PLAYER | UPDATE_LISTED
                entries: vec![play::PlayerInfo {
                    uuid,
                    name: Some(name.to_owned()),
                    listed: Some(true),
                    ..Default::default()
                }],
            });
            let _ = r.write_body_at(ts, &info.encode());

            let yaw_b = angle_to_byte(yaw);
            let pitch_b = angle_to_byte(pitch);
            let spawn = play::Clientbound::AddEntity(play::AddEntity {
                id: replay_self_id,
                uuid,
                kind: player_type_id,
                position: pos,
                velocity: [0.0; 3],
                pitch: pitch_b,
                yaw: yaw_b,
                head_yaw: yaw_b,
                data: 0,
            });
            let _ = r.write_body_at(ts, &spawn.encode());
        } else {
            let moved = (pos[0] - r.self_tracker.last_pos[0]).abs() > 1e-4
                || (pos[1] - r.self_tracker.last_pos[1]).abs() > 1e-4
                || (pos[2] - r.self_tracker.last_pos[2]).abs() > 1e-4
                || (yaw - r.self_tracker.last_yaw).abs() > 0.25
                || (pitch - r.self_tracker.last_pitch).abs() > 0.25
                || on_ground != r.self_tracker.last_on_ground;

            r.self_tracker.tick_counter = r.self_tracker.tick_counter.wrapping_add(1);
            if moved || r.self_tracker.tick_counter.is_multiple_of(20) {
                let sync = play::Clientbound::EntityPositionSync(play::EntityPositionSync {
                    id: replay_self_id,
                    path: play::PositionPath::Linear(pos),
                    yaw,
                    pitch,
                    on_ground,
                });
                let _ = r.write_body_at(ts, &sync.encode());
                let head = play::Clientbound::RotateHead(play::RotateHead {
                    id: replay_self_id,
                    head_yaw: angle_to_byte(yaw),
                });
                let _ = r.write_body_at(ts, &head.encode());
                r.self_tracker.last_pos = pos;
                r.self_tracker.last_yaw = yaw;
                r.self_tracker.last_pitch = pitch;
                r.self_tracker.last_on_ground = on_ground;
            }
        }

        let mut flags = 0u8;
        if sneaking {
            flags |= 0x02;
        }
        if sprinting {
            flags |= 0x08;
        }
        let pose_id = if sneaking { 5 } else { 0 }; // CROUCHING = 5, STANDING = 0
        if flags != r.self_tracker.last_flags || pose_id != r.self_tracker.last_pose_id {
            r.self_tracker.last_flags = flags;
            r.self_tracker.last_pose_id = pose_id;
            let data = play::Clientbound::SetEntityData(play::SetEntityData {
                id: replay_self_id,
                values: vec![
                    (
                        0,
                        horizon_protocol::entity_data::serializer::BYTE,
                        horizon_protocol::entity_data::Value::Byte(flags as i8),
                    ),
                    (
                        6,
                        horizon_protocol::entity_data::serializer::POSE,
                        horizon_protocol::entity_data::Value::Id(pose_id),
                    ),
                ],
            });
            let _ = r.write_body_at(ts, &data.encode());
        }

        if main_hand_item != r.self_tracker.last_main_item
            || off_hand_item != r.self_tracker.last_off_item
        {
            r.self_tracker.last_main_item = main_hand_item;
            r.self_tracker.last_off_item = off_hand_item;
            let eq = play::Clientbound::SetEquipment(play::SetEquipment {
                id: replay_self_id,
                slots: vec![
                    (
                        0,
                        main_hand_item.map(|id| horizon_protocol::item::Stack::new(id, 1)),
                    ),
                    (
                        1,
                        off_hand_item.map(|id| horizon_protocol::item::Stack::new(id, 1)),
                    ),
                ],
            });
            let _ = r.write_body_at(ts, &eq.encode());
        }

        if swing_main {
            let anim = play::Clientbound::Animate(play::Animate {
                id: replay_self_id,
                action: 0,
            });
            let _ = r.write_body_at(ts, &anim.encode());
        }
        if swing_off {
            let anim = play::Clientbound::Animate(play::Animate {
                id: replay_self_id,
                action: 3,
            });
            let _ = r.write_body_at(ts, &anim.encode());
        }
    }

    pub fn discard(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = guard.take() {
            drop(r.writer);
            let _ = fs::remove_file(&r.temp_tmcpr);
            *self.last_status.lock().unwrap_or_else(|e| e.into_inner()) =
                Some("Recording discarded".to_owned());
        }
    }

    /// Finalizes the `.tmcpr` stream and writes a complete `.mcpr` archive.
    pub fn stop_and_save(
        &self,
        custom_name: Option<&str>,
        thumb_png: Option<&[u8]>,
    ) -> Result<Option<PathBuf>> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(mut r) = guard.take() else {
            return Ok(None);
        };
        r.writer.flush()?;
        let duration_ms = u64::from(r.current_timestamp_ms());
        drop(r.writer);

        if r.packet_count == 0 {
            let _ = fs::remove_file(&r.temp_tmcpr);
            return Ok(None);
        }

        r.meta.duration = duration_ms;
        r.meta.players = r
            .players_seen
            .iter()
            .map(|u| format!("{u:032x}"))
            .collect();

        let dir = replays_dir(&r.game_dir);
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let default_stem = format!("{stamp}_{}", r.meta.server_name);
        let stem = custom_name.unwrap_or(&default_stem);
        let dest = unique_mcpr_path(&dir, stem);
        let crc_val = r.crc.finish();
        let timelines = ReplayTimelineSet::default();

        let res = write_mcpr_archive(
            &dest,
            &r.meta,
            &r.temp_tmcpr,
            crc_val,
            &r.markers,
            &timelines,
            thumb_png,
        );
        let _ = fs::remove_file(&r.temp_tmcpr);
        res?;

        let name = dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        *self.last_status.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(format!("Saved replay: {name}"));
        Ok(Some(dest))
    }
}

impl RecorderInner {
    fn current_timestamp_ms(&self) -> u32 {
        let elapsed = self.started_at.elapsed().as_millis() as u64;
        let paused_now = self
            .paused_since
            .map_or(0, |s| s.elapsed().as_millis() as u64);
        let active_ms = elapsed.saturating_sub(self.accumulated_pause_ms + paused_now);
        let ts = active_ms.min(u64::from(u32::MAX)) as u32;
        ts.max(self.last_timestamp_ms)
    }

    fn write_body_at(&mut self, ts: u32, body: &[u8]) -> std::io::Result<()> {
        let written = write_tmcpr_record(&mut self.writer, &mut self.crc, ts, body)?;
        self.last_timestamp_ms = ts;
        self.packet_count += 1;
        self.bytes_written += written as u64;
        Ok(())
    }
}

fn angle_to_byte(deg: f32) -> i8 {
    ((deg.rem_euclid(360.0) * 256.0 / 360.0).round() as i32 & 0xFF) as i8
}
