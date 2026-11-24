//! Volt + Boost Replay system: low-overhead background ring-buffer recording,
//! portable `.vreplay` archive format with thumbnail & integrity checks,
//! launcher clip management, floating HUD control gestures, and read-only
//! frame-accurate playback state.
use crate::volt_boost::{ReplayHudButtonLayout, ReplayLength, ReplayQuality};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"VREP0001";
pub const REPLAY_EXT: &str = "vreplay";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayError {
    Damaged(String),
    Io(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Damaged(msg) => write!(f, "This clip is damaged ({msg})"),
            Self::Io(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ReplayError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayEntitySnapshot {
    pub id: u32,
    pub kind: String,
    pub pos: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub sneaking: bool,
    pub sprinting: bool,
    pub swinging: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayBlockEvent {
    pub pos: [i32; 3],
    pub block_state: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReplayHudSnapshot {
    pub health: f32,
    pub food: i32,
    pub armor: i32,
    pub selected_slot: u8,
    pub hotbar_items: [String; 9],
    pub recent_chat: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayFrame {
    pub timestamp_ms: u64,
    pub pos: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub velocity: [f32; 3],
    pub sneaking: bool,
    pub sprinting: bool,
    pub swimming: bool,
    pub swinging: bool,
    pub held_item: String,
    pub entities: Vec<ReplayEntitySnapshot>,
    pub block_events: Vec<ReplayBlockEvent>,
    pub hud: Option<ReplayHudSnapshot>,
}

impl ReplayFrame {
    pub fn synthetic(timestamp_ms: u64, pos: [f64; 3], yaw: f32, pitch: f32) -> Self {
        Self {
            timestamp_ms,
            pos,
            yaw,
            pitch,
            head_yaw: yaw,
            velocity: [0.0; 3],
            sneaking: false,
            sprinting: false,
            swimming: false,
            swinging: false,
            held_item: "minecraft:air".into(),
            entities: Vec::new(),
            block_events: Vec::new(),
            hud: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayMetadata {
    pub id: String,
    pub name: String,
    pub world_name: String,
    pub is_multiplayer: bool,
    pub player_name: String,
    pub dimension: String,
    pub created_unix_ms: u64,
    pub duration_ms: u64,
    pub frame_count: usize,
    pub quality: ReplayQuality,
    pub capture_hud: bool,
    pub thumbnail_w: u16,
    pub thumbnail_h: u16,
    /// Raw RGBA8 bytes (`thumbnail_w * thumbnail_h * 4`) when captured, or empty.
    pub thumbnail_rgba: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayClip {
    pub meta: ReplayMetadata,
    pub frames: Vec<ReplayFrame>,
}

impl ReplayClip {
    pub fn to_bytes(&self) -> Result<Vec<u8>, ReplayError> {
        let meta_bytes =
            serde_json::to_vec(&self.meta).map_err(|e| ReplayError::Io(e.to_string()))?;
        let payload_json =
            serde_json::to_vec(&self.frames).map_err(|e| ReplayError::Io(e.to_string()))?;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(&payload_json)
            .map_err(|e| ReplayError::Io(e.to_string()))?;
        let compressed = enc.finish().map_err(|e| ReplayError::Io(e.to_string()))?;

        let mut hasher = Sha256::new();
        hasher.update(&meta_bytes);
        hasher.update(&compressed);
        let digest: [u8; 32] = hasher.finalize().into();

        let mut out = Vec::with_capacity(8 + 4 + meta_bytes.len() + 4 + compressed.len() + 32);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(meta_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&meta_bytes);
        out.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        out.extend_from_slice(&compressed);
        out.extend_from_slice(&digest);
        Ok(out)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplayError> {
        let (meta, compressed, digest) = parse_envelope(bytes)?;
        let mut dec = ZlibDecoder::new(compressed);
        let mut payload = Vec::new();
        dec.read_to_end(&mut payload)
            .map_err(|e| ReplayError::Damaged(format!("corrupt stream: {e}")))?;
        let frames: Vec<ReplayFrame> = serde_json::from_slice(&payload)
            .map_err(|e| ReplayError::Damaged(format!("corrupt frames: {e}")))?;
        let _ = digest;
        Ok(Self { meta, frames })
    }

    pub fn read_metadata_only(bytes: &[u8]) -> Result<ReplayMetadata, ReplayError> {
        let (meta, _, _) = parse_envelope(bytes)?;
        Ok(meta)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), ReplayError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ReplayError::Io(e.to_string()))?;
        }
        let encoded = self.to_bytes()?;
        let tmp = path.with_extension("vreplay.partial");
        std::fs::write(&tmp, encoded).map_err(|e| ReplayError::Io(e.to_string()))?;
        std::fs::rename(tmp, path).map_err(|e| ReplayError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self, ReplayError> {
        let bytes = std::fs::read(path).map_err(|e| ReplayError::Damaged(e.to_string()))?;
        Self::from_bytes(&bytes)
    }
}

fn parse_envelope(bytes: &[u8]) -> Result<(ReplayMetadata, &[u8], [u8; 32]), ReplayError> {
    if bytes.len() < 8 + 4 + 4 + 32 {
        return Err(ReplayError::Damaged("truncated header".into()));
    }
    if &bytes[0..8] != MAGIC {
        return Err(ReplayError::Damaged("invalid replay magic".into()));
    }
    let meta_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let meta_end = 12usize
        .checked_add(meta_len)
        .ok_or_else(|| ReplayError::Damaged("invalid metadata size".into()))?;
    if meta_end + 4 + 32 > bytes.len() {
        return Err(ReplayError::Damaged("truncated metadata".into()));
    }
    let meta_bytes = &bytes[12..meta_end];
    let payload_len =
        u32::from_le_bytes(bytes[meta_end..meta_end + 4].try_into().unwrap()) as usize;
    let payload_start = meta_end + 4;
    let payload_end = payload_start
        .checked_add(payload_len)
        .ok_or_else(|| ReplayError::Damaged("invalid payload size".into()))?;
    if payload_end + 32 != bytes.len() {
        return Err(ReplayError::Damaged("checksum boundary mismatch".into()));
    }
    let compressed = &bytes[payload_start..payload_end];
    let stored_digest: [u8; 32] = bytes[payload_end..payload_end + 32]
        .try_into()
        .map_err(|_| ReplayError::Damaged("missing checksum".into()))?;

    let mut hasher = Sha256::new();
    hasher.update(meta_bytes);
    hasher.update(compressed);
    let actual: [u8; 32] = hasher.finalize().into();
    if actual != stored_digest {
        return Err(ReplayError::Damaged("checksum mismatch".into()));
    }

    let meta: ReplayMetadata = serde_json::from_slice(meta_bytes)
        .map_err(|e| ReplayError::Damaged(format!("invalid metadata: {e}")))?;
    Ok((meta, compressed, stored_digest))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording,
    Paused,
}

pub struct ReplayRecorder {
    pub state: RecordingState,
    pub world_name: String,
    pub is_multiplayer: bool,
    pub player_name: String,
    pub dimension: String,
    pub length: ReplayLength,
    pub quality: ReplayQuality,
    pub capture_hud: bool,
    pub storage_budget_mb: u32,
    pub frames: VecDeque<ReplayFrame>,
    pub thumbnail: Option<(u16, u16, Vec<u8>)>,
    last_sample_ms: Option<u64>,
    session_start_ms: Option<u64>,
    paused_accum_ms: u64,
    pause_started_ms: Option<u64>,
}

impl Default for ReplayRecorder {
    fn default() -> Self {
        Self {
            state: RecordingState::Idle,
            world_name: String::new(),
            is_multiplayer: false,
            player_name: "Player".into(),
            dimension: "overworld".into(),
            length: ReplayLength::Min5,
            quality: ReplayQuality::Medium,
            capture_hud: false,
            storage_budget_mb: 1024,
            frames: VecDeque::with_capacity(2048),
            thumbnail: None,
            last_sample_ms: None,
            session_start_ms: None,
            paused_accum_ms: 0,
            pause_started_ms: None,
        }
    }
}

impl ReplayRecorder {
    pub fn start(
        &mut self,
        world_name: impl Into<String>,
        is_multiplayer: bool,
        player_name: impl Into<String>,
        dimension: impl Into<String>,
        length: ReplayLength,
        quality: ReplayQuality,
        capture_hud: bool,
        storage_budget_mb: u32,
    ) {
        self.frames.clear();
        self.thumbnail = None;
        self.last_sample_ms = None;
        self.session_start_ms = None;
        self.paused_accum_ms = 0;
        self.pause_started_ms = None;
        self.world_name = world_name.into();
        self.is_multiplayer = is_multiplayer;
        self.player_name = player_name.into();
        self.dimension = dimension.into();
        self.length = length;
        self.quality = quality;
        self.capture_hud = capture_hud;
        self.storage_budget_mb = storage_budget_mb.clamp(128, 8192);
        self.state = RecordingState::Recording;
    }

    pub fn pause(&mut self, now_ms: u64) {
        if self.state == RecordingState::Recording {
            self.state = RecordingState::Paused;
            self.pause_started_ms = Some(now_ms);
        }
    }

    pub fn resume(&mut self, now_ms: u64) {
        if self.state == RecordingState::Paused {
            if let Some(p) = self.pause_started_ms.take() {
                self.paused_accum_ms = self.paused_accum_ms.saturating_add(now_ms.saturating_sub(p));
            }
            self.state = RecordingState::Recording;
        }
    }

    pub fn should_sample(&self, now_ms: u64) -> bool {
        if self.state != RecordingState::Recording {
            return false;
        }
        match self.last_sample_ms {
            None => true,
            Some(prev) => now_ms.saturating_sub(prev) >= self.quality.sample_interval_ms(),
        }
    }

    pub fn set_thumbnail(&mut self, w: u16, h: u16, rgba: Vec<u8>) {
        if w > 0 && h > 0 && rgba.len() == w as usize * h as usize * 4 {
            self.thumbnail = Some((w, h, rgba));
        }
    }

    /// Appends a frame if the quality sample cadence is satisfied, and evicts
    /// oldest frames outside `ReplayLength` ring buffer window.
    pub fn record_frame(&mut self, now_ms: u64, mut frame: ReplayFrame) -> bool {
        if !self.should_sample(now_ms) {
            return false;
        }
        let start = *self.session_start_ms.get_or_insert(now_ms);
        let rel_ms = now_ms
            .saturating_sub(start)
            .saturating_sub(self.paused_accum_ms);
        frame.timestamp_ms = rel_ms;
        if !self.capture_hud {
            frame.hud = None;
        }
        self.last_sample_ms = Some(now_ms);
        self.frames.push_back(frame);
        self.evict_ring();
        true
    }

    fn evict_ring(&mut self) {
        let Some(max_ms) = self.length.max_duration_ms() else {
            return;
        };
        let Some(newest_ts) = self.frames.back().map(|f| f.timestamp_ms) else {
            return;
        };
        while self.frames.len() > 1 {
            let oldest_ts = self.frames.front().map_or(newest_ts, |f| f.timestamp_ms);
            if newest_ts.saturating_sub(oldest_ts) > max_ms {
                self.frames.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        match (self.frames.front(), self.frames.back()) {
            (Some(a), Some(b)) => b.timestamp_ms.saturating_sub(a.timestamp_ms),
            _ => 0,
        }
    }

    pub fn finish_and_save(&mut self, replays_dir: &Path) -> Result<Option<PathBuf>, ReplayError> {
        let was_active = self.state != RecordingState::Idle;
        self.state = RecordingState::Idle;
        if !was_active || self.frames.is_empty() {
            self.frames.clear();
            return Ok(None);
        }

        // Normalize timestamps so saved clip starts at 0 ms after ring eviction.
        let base_ts = self.frames.front().map_or(0, |f| f.timestamp_ms);
        let mut frames: Vec<ReplayFrame> = self.frames.drain(..).collect();
        for f in &mut frames {
            f.timestamp_ms = f.timestamp_ms.saturating_sub(base_ts);
        }
        let duration_ms = frames.last().map_or(0, |f| f.timestamp_ms);
        let created_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let id = format!("replay_{created_unix_ms}");
        let world_label = if self.world_name.is_empty() {
            "World".to_string()
        } else {
            self.world_name.clone()
        };
        let name = format!("{world_label} · {}", format_duration(duration_ms));
        let (tw, th, trgba) = self.thumbnail.take().unwrap_or_else(default_thumbnail);
        let clip = ReplayClip {
            meta: ReplayMetadata {
                id: id.clone(),
                name,
                world_name: world_label,
                is_multiplayer: self.is_multiplayer,
                player_name: self.player_name.clone(),
                dimension: self.dimension.clone(),
                created_unix_ms,
                duration_ms,
                frame_count: frames.len(),
                quality: self.quality,
                capture_hud: self.capture_hud,
                thumbnail_w: tw,
                thumbnail_h: th,
                thumbnail_rgba: trgba,
            },
            frames,
        };
        let out_path = replays_dir.join(format!("{id}.{REPLAY_EXT}"));
        clip.save_to_file(&out_path)?;
        enforce_storage_budget(replays_dir, u64::from(self.storage_budget_mb) * 1024 * 1024)?;
        Ok(Some(out_path))
    }
}

fn default_thumbnail() -> (u16, u16, Vec<u8>) {
    let (w, h) = (32u16, 18u16);
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        for x in 0..w {
            let sky = y < h / 2;
            let stripe = (x + y) % 4 == 0;
            let (r, g, b) = if sky {
                if stripe { (74, 118, 178) } else { (58, 96, 152) }
            } else if stripe {
                (56, 108, 62)
            } else {
                (44, 88, 52)
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (w, h, rgba)
}

pub fn enforce_storage_budget(replays_dir: &Path, max_bytes: u64) -> Result<(), ReplayError> {
    if max_bytes == 0 {
        return Ok(());
    }
    let mut files: Vec<(PathBuf, u64, u64)> = Vec::new();
    let Ok(entries) = std::fs::read_dir(replays_dir) else {
        return Ok(());
    };
    let mut total: u64 = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some(REPLAY_EXT) {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            let len = meta.len();
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis() as u64);
            total = total.saturating_add(len);
            files.push((path, len, modified));
        }
    }
    files.sort_by_key(|(_, _, modified)| *modified); // oldest first
    for (path, len, _) in files {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplaySort {
    #[default]
    DateDesc,
    SizeDesc,
}

impl ReplaySort {
    pub fn label(self) -> &'static str {
        match self {
            Self::DateDesc => "Date (Newest)",
            Self::SizeDesc => "Size (Largest)",
        }
    }
    pub fn toggle(self) -> Self {
        match self {
            Self::DateDesc => Self::SizeDesc,
            Self::SizeDesc => Self::DateDesc,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplayListItem {
    pub path: PathBuf,
    pub file_size_bytes: u64,
    pub created_unix_ms: u64,
    pub name: String,
    pub world_name: String,
    pub duration_ms: u64,
    pub frame_count: usize,
    pub thumbnail_w: u16,
    pub thumbnail_h: u16,
    pub thumbnail_rgba: Vec<u8>,
    pub damaged: bool,
    pub error_message: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplayStorageSummary {
    pub total_bytes: u64,
    pub clip_count: usize,
    pub free_disk_bytes: u64,
}

pub fn list_replays(
    replays_dir: &Path,
    sort: ReplaySort,
    world_filter: Option<&str>,
) -> Vec<ReplayListItem> {
    let Ok(entries) = std::fs::read_dir(replays_dir) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some(REPLAY_EXT) {
            continue;
        }
        let file_size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let fallback_time = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as u64);
        let fallback_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Replay")
            .to_string();

        let item = match std::fs::read(&path) {
            Ok(bytes) => match ReplayClip::read_metadata_only(&bytes) {
                Ok(meta) => ReplayListItem {
                    path,
                    file_size_bytes,
                    created_unix_ms: meta.created_unix_ms,
                    name: meta.name,
                    world_name: meta.world_name,
                    duration_ms: meta.duration_ms,
                    frame_count: meta.frame_count,
                    thumbnail_w: meta.thumbnail_w,
                    thumbnail_h: meta.thumbnail_h,
                    thumbnail_rgba: meta.thumbnail_rgba,
                    damaged: false,
                    error_message: None,
                },
                Err(err) => ReplayListItem {
                    path,
                    file_size_bytes,
                    created_unix_ms: fallback_time,
                    name: fallback_name,
                    world_name: "Unknown".into(),
                    duration_ms: 0,
                    frame_count: 0,
                    thumbnail_w: 0,
                    thumbnail_h: 0,
                    thumbnail_rgba: Vec::new(),
                    damaged: true,
                    error_message: Some(err.to_string()),
                },
            },
            Err(err) => ReplayListItem {
                path,
                file_size_bytes,
                created_unix_ms: fallback_time,
                name: fallback_name,
                world_name: "Unknown".into(),
                duration_ms: 0,
                frame_count: 0,
                thumbnail_w: 0,
                thumbnail_h: 0,
                thumbnail_rgba: Vec::new(),
                damaged: true,
                error_message: Some(format!("This clip is damaged ({err})")),
            },
        };
        items.push(item);
    }

    if let Some(world) = world_filter.filter(|w| !w.is_empty()) {
        items.retain(|it| it.world_name == world);
    }

    match sort {
        ReplaySort::DateDesc => {
            items.sort_by(|a, b| b.created_unix_ms.cmp(&a.created_unix_ms).then(a.name.cmp(&b.name)));
        }
        ReplaySort::SizeDesc => {
            items.sort_by(|a, b| b.file_size_bytes.cmp(&a.file_size_bytes).then(b.created_unix_ms.cmp(&a.created_unix_ms)));
        }
    }
    items
}

pub fn available_worlds(replays_dir: &Path) -> Vec<String> {
    let mut worlds: Vec<String> = list_replays(replays_dir, ReplaySort::DateDesc, None)
        .into_iter()
        .filter(|it| !it.damaged && !it.world_name.is_empty())
        .map(|it| it.world_name)
        .collect();
    worlds.sort();
    worlds.dedup();
    worlds
}

pub fn storage_summary(replays_dir: &Path) -> ReplayStorageSummary {
    let mut total_bytes = 0u64;
    let mut clip_count = 0usize;
    if let Ok(entries) = std::fs::read_dir(replays_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some(REPLAY_EXT) {
                clip_count += 1;
                total_bytes = total_bytes.saturating_add(entry.metadata().map(|m| m.len()).unwrap_or(0));
            }
        }
    }
    let probe_path = if replays_dir.exists() {
        replays_dir.to_path_buf()
    } else {
        replays_dir.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."))
    };
    let free_disk_bytes = free_space_bytes(&probe_path);
    ReplayStorageSummary {
        total_bytes,
        clip_count,
        free_disk_bytes,
    }
}

#[cfg(unix)]
fn free_space_bytes(path: &Path) -> u64 {
    rustix::fs::statvfs(path)
        .map(|s| s.f_bavail as u64 * s.f_frsize as u64)
        .unwrap_or(0)
}

#[cfg(not(unix))]
fn free_space_bytes(_path: &Path) -> u64 {
    0
}

pub fn rename_clip(path: &Path, new_name: &str) -> Result<(), ReplayError> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(ReplayError::Io("Replay name cannot be empty".into()));
    }
    let mut clip = ReplayClip::load_from_file(path)?;
    clip.meta.name = trimmed.to_string();
    clip.save_to_file(path)
}

pub fn delete_clip(path: &Path) -> Result<(), ReplayError> {
    std::fs::remove_file(path).map_err(|e| ReplayError::Io(e.to_string()))
}

pub fn clear_all_replays(replays_dir: &Path) -> Result<usize, ReplayError> {
    let Ok(entries) = std::fs::read_dir(replays_dir) else {
        return Ok(0);
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some(REPLAY_EXT) {
            std::fs::remove_file(&path).map_err(|e| ReplayError::Io(e.to_string()))?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn export_portable(
    source_path: &Path,
    dest_path: &Path,
    mut on_progress: impl FnMut(f32),
) -> Result<(), ReplayError> {
    // Validate clip integrity before exporting so broken files aren't exported silently.
    let _ = ReplayClip::load_from_file(source_path)?;
    on_progress(0.1);
    let mut src = std::fs::File::open(source_path).map_err(|e| ReplayError::Io(e.to_string()))?;
    let total = src
        .metadata()
        .map(|m| m.len().max(1))
        .unwrap_or(1);
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ReplayError::Io(e.to_string()))?;
    }
    let tmp = dest_path.with_extension("vreplay.export");
    let mut dst = std::fs::File::create(&tmp).map_err(|e| ReplayError::Io(e.to_string()))?;
    let mut buf = [0u8; 32 * 1024];
    let mut copied = 0u64;
    loop {
        let n = src.read(&mut buf).map_err(|e| ReplayError::Io(e.to_string()))?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n])
            .map_err(|e| ReplayError::Io(e.to_string()))?;
        copied += n as u64;
        on_progress((copied as f32 / total as f32).clamp(0.1, 0.99));
    }
    dst.flush().map_err(|e| ReplayError::Io(e.to_string()))?;
    drop(dst);
    std::fs::rename(tmp, dest_path).map_err(|e| ReplayError::Io(e.to_string()))?;
    on_progress(1.0);
    Ok(())
}

pub fn import_portable(source_path: &Path, replays_dir: &Path) -> Result<PathBuf, ReplayError> {
    let clip = ReplayClip::load_from_file(source_path)?;
    std::fs::create_dir_all(replays_dir).map_err(|e| ReplayError::Io(e.to_string()))?;
    let mut dest = replays_dir.join(format!("{}.{REPLAY_EXT}", clip.meta.id));
    if dest.exists() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(1);
        dest = replays_dir.join(format!("{}_{suffix}.{REPLAY_EXT}", clip.meta.id));
    }
    clip.save_to_file(&dest)?;
    Ok(dest)
}

pub const PLAYBACK_SPEEDS: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

#[derive(Clone, Debug)]
pub struct ReplayPlayback {
    pub clip: ReplayClip,
    pub playing: bool,
    pub scrubbing: bool,
    pub speed_index: usize,
    pub current_time_ms: f64,
    pub current_frame: usize,
    pub free_camera: bool,
    pub third_person_self: bool,
    pub free_cam_pos: [f64; 3],
    pub free_cam_yaw: f32,
    pub free_cam_pitch: f32,
}

impl ReplayPlayback {
    pub fn open(path: &Path) -> Result<Self, ReplayError> {
        let clip = ReplayClip::load_from_file(path)?;
        if clip.frames.is_empty() {
            return Err(ReplayError::Damaged("replay has no frames".into()));
        }
        let first = &clip.frames[0];
        Ok(Self {
            free_cam_pos: [first.pos[0], first.pos[1] + 1.62, first.pos[2]],
            free_cam_yaw: first.yaw,
            free_cam_pitch: first.pitch,
            clip,
            playing: true,
            scrubbing: false,
            speed_index: 2, // 1x
            current_time_ms: 0.0,
            current_frame: 0,
            free_camera: false,
            third_person_self: false,
        })
    }

    pub fn speed(&self) -> f32 {
        PLAYBACK_SPEEDS[self.speed_index.min(PLAYBACK_SPEEDS.len() - 1)]
    }

    pub fn speed_label(&self) -> &'static str {
        match self.speed_index {
            0 => "0.25×",
            1 => "0.5×",
            2 => "1×",
            3 => "2×",
            _ => "4×",
        }
    }

    pub fn cycle_speed(&mut self, dir: i32) {
        let len = PLAYBACK_SPEEDS.len() as i32;
        self.speed_index = (self.speed_index as i32 + dir).rem_euclid(len) as usize;
    }

    pub fn toggle_play(&mut self) {
        self.playing = !self.playing;
    }

    pub fn rewind(&mut self) {
        self.seek_frame(0);
    }

    pub fn step_back(&mut self) {
        self.playing = false;
        self.seek_frame(self.current_frame.saturating_sub(1));
    }

    pub fn step_forward(&mut self) {
        self.playing = false;
        let max_idx = self.clip.frames.len().saturating_sub(1);
        self.seek_frame((self.current_frame + 1).min(max_idx));
    }

    pub fn seek_frame(&mut self, index: usize) {
        if self.clip.frames.is_empty() {
            return;
        }
        let idx = index.min(self.clip.frames.len() - 1);
        self.current_frame = idx;
        self.current_time_ms = self.clip.frames[idx].timestamp_ms as f64;
    }

    pub fn begin_scrub(&mut self) {
        self.scrubbing = true;
    }

    pub fn scrub_fraction(&mut self, fraction: f32) {
        let max_idx = self.clip.frames.len().saturating_sub(1);
        let idx = ((fraction.clamp(0.0, 1.0)) * max_idx as f32).round() as usize;
        self.seek_frame(idx);
    }

    pub fn end_scrub(&mut self) {
        self.scrubbing = false;
        self.playing = true;
    }

    pub fn fraction(&self) -> f32 {
        let max_idx = self.clip.frames.len().saturating_sub(1);
        if max_idx == 0 {
            0.0
        } else {
            self.current_frame as f32 / max_idx as f32
        }
    }

    pub fn tick(&mut self, dt_secs: f32) {
        if !self.playing || self.scrubbing || self.clip.frames.len() <= 1 {
            return;
        }
        let end_ms = self.clip.frames.last().map_or(0.0, |f| f.timestamp_ms as f64);
        self.current_time_ms = (self.current_time_ms
            + (dt_secs.max(0.0) as f64) * 1000.0 * self.speed() as f64)
            .min(end_ms);
        let idx = self
            .clip
            .frames
            .partition_point(|f| (f.timestamp_ms as f64) <= self.current_time_ms)
            .saturating_sub(1);
        self.current_frame = idx.min(self.clip.frames.len() - 1);
        if self.current_time_ms >= end_ms {
            self.playing = false;
        }
    }

    pub fn current_snapshot(&self) -> &ReplayFrame {
        &self.clip.frames[self.current_frame.min(self.clip.frames.len() - 1)]
    }

    pub fn toggle_free_camera(&mut self) {
        self.free_camera = !self.free_camera;
        if self.free_camera {
            let f = self.current_snapshot();
            self.free_cam_pos = [f.pos[0], f.pos[1] + 1.62, f.pos[2]];
            self.free_cam_yaw = f.yaw;
            self.free_cam_pitch = f.pitch;
        }
    }

    pub fn toggle_third_person_self(&mut self) {
        self.third_person_self = !self.third_person_self;
    }

    pub fn move_free_camera(&mut self, forward: f32, strafe: f32, up: f32, dt_secs: f32) {
        if !self.free_camera {
            return;
        }
        let speed = 14.0 * dt_secs as f64;
        let yaw_rad = self.free_cam_yaw.to_radians() as f64;
        let fx = -yaw_rad.sin();
        let fz = yaw_rad.cos();
        let rx = yaw_rad.cos();
        let rz = yaw_rad.sin();
        self.free_cam_pos[0] += (fx * forward as f64 + rx * strafe as f64) * speed;
        self.free_cam_pos[1] += up as f64 * speed;
        self.free_cam_pos[2] += (fz * forward as f64 + rz * strafe as f64) * speed;
    }

    /// Computes camera eye `(pos, yaw, pitch)` honoring Free Camera and Third-Person Self.
    pub fn camera_view(&self) -> ([f64; 3], f32, f32) {
        if self.free_camera {
            return (self.free_cam_pos, self.free_cam_yaw, self.free_cam_pitch);
        }
        let f = self.current_snapshot();
        if !self.third_person_self {
            return ([f.pos[0], f.pos[1] + 1.62, f.pos[2]], f.yaw, f.pitch);
        }
        let yaw_rad = f.yaw.to_radians() as f64;
        let pitch_rad = f.pitch.to_radians() as f64;
        let dist = 4.0;
        let dx = yaw_rad.sin() * pitch_rad.cos() * dist;
        let dy = pitch_rad.sin() * dist;
        let dz = -yaw_rad.cos() * pitch_rad.cos() * dist;
        (
            [f.pos[0] + dx, f.pos[1] + 1.62 + dy, f.pos[2] + dz],
            f.yaw,
            f.pitch,
        )
    }
}

/// Interactive gesture controller for the on-screen floating Replay HUD button.
#[derive(Clone, Debug, Default)]
pub struct ReplayHudGesture {
    pub active_pointer: Option<u64>,
    pub down_pos: (f32, f32),
    pub last_pos: (f32, f32),
    pub down_ms: u64,
    pub adjust_mode: bool,
    pub moved: bool,
    pub start_size: f32,
    pub start_opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayHudAction {
    None,
    ToggleStartPause,
    EndAndSave,
    LayoutChanged,
}

impl ReplayHudGesture {
    pub fn press(&mut self, pointer_id: u64, pos: (f32, f32), now_ms: u64, layout: &ReplayHudButtonLayout) {
        self.active_pointer = Some(pointer_id);
        self.down_pos = pos;
        self.last_pos = pos;
        self.down_ms = now_ms;
        self.adjust_mode = false;
        self.moved = false;
        self.start_size = layout.size;
        self.start_opacity = layout.opacity;
    }

    /// Drag handler:
    /// - If held still >= 260 ms before dragging, enters hold-drag adjust mode:
    ///   up/down resizes, left/right changes opacity.
    /// - Otherwise drags button freely across the viewport.
    pub fn move_to(
        &mut self,
        pointer_id: u64,
        pos: (f32, f32),
        now_ms: u64,
        viewport: (f32, f32),
        layout: &mut ReplayHudButtonLayout,
    ) -> ReplayHudAction {
        if self.active_pointer != Some(pointer_id) {
            return ReplayHudAction::None;
        }
        let dx_total = pos.0 - self.down_pos.0;
        let dy_total = pos.1 - self.down_pos.1;
        let dist_total = (dx_total * dx_total + dy_total * dy_total).sqrt();
        if !self.moved && !self.adjust_mode && now_ms.saturating_sub(self.down_ms) >= 260 && dist_total <= 12.0 {
            self.adjust_mode = true;
        }
        if dist_total > 6.0 {
            self.moved = true;
            if !self.adjust_mode && now_ms.saturating_sub(self.down_ms) >= 260 && (self.last_pos.0 - self.down_pos.0).abs() < 4.0 && (self.last_pos.1 - self.down_pos.1).abs() < 4.0 {
                self.adjust_mode = true;
            }
        }
        if self.adjust_mode {
            // Up increases size, down decreases size; right increases opacity, left decreases.
            layout.size = (self.start_size - dy_total / 140.0).clamp(0.65, 1.85);
            layout.opacity = (self.start_opacity + dx_total / 180.0).clamp(0.25, 1.0);
            self.last_pos = pos;
            return ReplayHudAction::LayoutChanged;
        }
        if self.moved && viewport.0 > 1.0 && viewport.1 > 1.0 {
            let dx = pos.0 - self.last_pos.0;
            let dy = pos.1 - self.last_pos.1;
            layout.norm_x = (layout.norm_x + dx / viewport.0).clamp(0.04, 0.96);
            layout.norm_y = (layout.norm_y + dy / viewport.1).clamp(0.04, 0.96);
            self.last_pos = pos;
            return ReplayHudAction::LayoutChanged;
        }
        self.last_pos = pos;
        ReplayHudAction::None
    }

    pub fn release(&mut self, pointer_id: u64, tap_end_zone: bool) -> ReplayHudAction {
        if self.active_pointer != Some(pointer_id) {
            return ReplayHudAction::None;
        }
        self.active_pointer = None;
        if self.adjust_mode || self.moved {
            return ReplayHudAction::LayoutChanged;
        }
        if tap_end_zone {
            ReplayHudAction::EndAndSave
        } else {
            ReplayHudAction::ToggleStartPause
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "volt_replay_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn ring_buffer_evicts_oldest_footage_beyond_window() {
        let mut rec = ReplayRecorder::default();
        rec.start(
            "SurvivalWorld",
            false,
            "Steve",
            "overworld",
            ReplayLength::Sec30,
            ReplayQuality::Low,
            false,
            1024,
        );
        // Record 45 seconds of frames (100ms apart)
        for step in 0..450u64 {
            let ts = 10_000 + step * 100;
            rec.record_frame(ts, ReplayFrame::synthetic(0, [step as f64, 64.0, 0.0], 0.0, 0.0));
        }
        assert!(rec.elapsed_ms() <= 30_000);
        // Oldest kept frame should be around 15 seconds into the session
        assert!(rec.frames.front().unwrap().pos[0] >= 140.0);
    }

    #[test]
    fn portable_export_import_rename_delete_and_damaged_detection() {
        let dir = temp_dir("lifecycle");
        let replays_dir = dir.join("replays");
        let export_path = dir.join("exported.vreplay");

        let mut rec = ReplayRecorder::default();
        rec.start(
            "PvP Arena",
            true,
            "Alex",
            "overworld",
            ReplayLength::Min1,
            ReplayQuality::Medium,
            true,
            512,
        );
        for step in 0..10u64 {
            let mut frame = ReplayFrame::synthetic(0, [step as f64 * 2.0, 70.0, -3.0], 45.0, 10.0);
            frame.hud = Some(ReplayHudSnapshot {
                health: 18.0,
                food: 20,
                ..Default::default()
            });
            rec.record_frame(1000 + step * 50, frame);
        }
        let saved = rec.finish_and_save(&replays_dir).unwrap().unwrap();
        assert!(saved.exists());

        // List & rename
        let list = list_replays(&replays_dir, ReplaySort::DateDesc, None);
        assert_eq!(list.len(), 1);
        assert!(!list[0].damaged);
        assert_eq!(list[0].world_name, "PvP Arena");
        rename_clip(&saved, "Clutch Win").unwrap();
        assert_eq!(list_replays(&replays_dir, ReplaySort::DateDesc, None)[0].name, "Clutch Win");

        // Portable export with progress + import roundtrip
        let mut last_p = 0.0;
        export_portable(&saved, &export_path, |p| last_p = p).unwrap();
        assert_eq!(last_p, 1.0);
        let imported = import_portable(&export_path, &replays_dir).unwrap();
        assert_eq!(list_replays(&replays_dir, ReplaySort::DateDesc, None).len(), 2);

        // Damaged clip detection
        let corrupt = replays_dir.join("broken.vreplay");
        std::fs::write(&corrupt, b"VREP0001corruptdata").unwrap();
        let with_broken = list_replays(&replays_dir, ReplaySort::DateDesc, None);
        let broken_item = with_broken.iter().find(|i| i.path == corrupt).unwrap();
        assert!(broken_item.damaged);
        assert!(broken_item.error_message.as_deref().unwrap().contains("damaged"));

        // Clear all
        let removed = clear_all_replays(&replays_dir).unwrap();
        assert_eq!(removed, 3);
        assert!(list_replays(&replays_dir, ReplaySort::DateDesc, None).is_empty());
        let _ = imported;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn playback_scrub_step_rewind_speed_free_cam_and_third_person() {
        let dir = temp_dir("playback");
        let clip_path = dir.join("clip.vreplay");
        let mut frames = Vec::new();
        for i in 0..20 {
            frames.push(ReplayFrame::synthetic(i * 100, [i as f64, 64.0, 0.0], 90.0, 0.0));
        }
        let clip = ReplayClip {
            meta: ReplayMetadata {
                id: "clip1".into(),
                name: "Test".into(),
                world_name: "World".into(),
                is_multiplayer: false,
                player_name: "Steve".into(),
                dimension: "overworld".into(),
                created_unix_ms: 1,
                duration_ms: 1900,
                frame_count: frames.len(),
                quality: ReplayQuality::Medium,
                capture_hud: false,
                thumbnail_w: 1,
                thumbnail_h: 1,
                thumbnail_rgba: vec![0, 0, 0, 255],
            },
            frames,
        };
        clip.save_to_file(&clip_path).unwrap();

        let mut pb = ReplayPlayback::open(&clip_path).unwrap();
        assert!(pb.playing);
        pb.cycle_speed(2); // 1x -> 4x
        assert_eq!(pb.speed(), 4.0);
        pb.tick(0.25); // +1000ms -> frame 10
        assert_eq!(pb.current_frame, 10);

        pb.step_back();
        assert!(!pb.playing);
        assert_eq!(pb.current_frame, 9);
        pb.step_forward();
        assert_eq!(pb.current_frame, 10);

        pb.begin_scrub();
        pb.scrub_fraction(0.5);
        assert!(pb.scrubbing);
        pb.end_scrub();
        assert!(pb.playing);

        pb.rewind();
        assert_eq!(pb.current_frame, 0);

        // Third person self offsets camera behind player
        let (fp_pos, _, _) = pb.camera_view();
        pb.toggle_third_person_self();
        let (tp_pos, _, _) = pb.camera_view();
        assert_ne!(fp_pos, tp_pos);

        // Free camera detaches & flies
        pb.toggle_free_camera();
        let before = pb.free_cam_pos;
        pb.move_free_camera(1.0, 0.0, 0.5, 0.5);
        assert_ne!(pb.free_cam_pos, before);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn hud_button_gesture_supports_drag_and_hold_resize_opacity() {
        let mut layout = ReplayHudButtonLayout::default();
        let mut g = ReplayHudGesture::default();

        // Quick drag repositions
        g.press(1, (200.0, 100.0), 100, &layout);
        let act = g.move_to(1, (300.0, 150.0), 140, (1000.0, 500.0), &mut layout);
        assert_eq!(act, ReplayHudAction::LayoutChanged);
        assert!((layout.norm_x - 0.96).abs() < 0.02);
        g.release(1, false);

        // Hold >= 260ms then drag up/right adjusts size and opacity
        let orig_size = layout.size;
        let orig_op = layout.opacity;
        g.press(2, (200.0, 200.0), 1000, &layout);
        g.move_to(2, (202.0, 202.0), 1300, (1000.0, 500.0), &mut layout);
        g.move_to(2, (260.0, 140.0), 1350, (1000.0, 500.0), &mut layout);
        assert!(layout.size > orig_size);
        assert!(layout.opacity > orig_op);
    }
}
