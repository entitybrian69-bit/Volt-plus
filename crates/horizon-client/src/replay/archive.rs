//! `.mcpr` archive and `.tmcpr` binary packet stream support.
//!
//! Structure inside `<name>.mcpr` (ZIP):
//! - `metaData.json`: session metadata (`singleplayer`, `serverName`, `duration`,
//!   `date`, `mcversion`, `fileFormat`, `fileFormatVersion`, `protocol`,
//!   `generator`, `selfId`, `players`)
//! - `recording.tmcpr`: `[u32 BE timestamp_ms][u32 BE length][VarInt packet_id + payload]`
//! - `recording.tmcpr.crc32`: decimal CRC32 checksum of `recording.tmcpr`
//! - `markers.json`: saved timeline markers
//! - `timelines.json`: saved camera path & time keyframe timelines
//! - `mods.json`: `[]`
//! - `thumb`: optional PNG thumbnail

use super::timeline::{ReplayMarker, ReplayTimelineSet};
use crate::Result;
use horizon_protocol::{
    PROTOCOL_VERSION,
    buf::{Reader, write_varint},
    ids,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const REPLAYS_DIR: &str = "replays";
pub const RAW_REPLAYS_SUBDIR: &str = "raw";
pub const RENDERED_REPLAYS_SUBDIR: &str = "rendered";
pub const MCPR_EXT: &str = "mcpr";
pub const FILE_FORMAT: &str = "MCPR";
pub const FILE_FORMAT_VERSION: u32 = 14;
pub const GENERATOR_NAME: &str = "VoltPlusReplayMod";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacketState {
    Configuration,
    Play,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayMetadata {
    pub singleplayer: bool,
    pub server_name: String,
    pub duration: u64,
    pub date: u64,
    pub mcversion: String,
    pub file_format: String,
    pub file_format_version: u32,
    pub protocol: i32,
    pub generator: String,
    pub self_id: i32,
    pub players: Vec<String>,
}

impl Default for ReplayMetadata {
    fn default() -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        Self {
            singleplayer: true,
            server_name: "World".to_owned(),
            duration: 0,
            date: now_ms,
            mcversion: horizon_core::VANILLA_VERSION.to_owned(),
            file_format: FILE_FORMAT.to_owned(),
            file_format_version: FILE_FORMAT_VERSION,
            protocol: PROTOCOL_VERSION,
            generator: GENERATOR_NAME.to_owned(),
            self_id: -1,
            players: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReplaySummary {
    pub path: PathBuf,
    pub file_name: String,
    pub display_name: String,
    pub meta: ReplayMetadata,
    pub markers_count: usize,
    pub keyframes_count: usize,
    pub size_bytes: u64,
    pub has_thumb: bool,
}

#[derive(Clone, Debug)]
pub struct PacketIndexEntry {
    pub file_offset: u64,
    pub body_len: usize,
    pub timestamp_ms: u32,
    pub state: PacketState,
}

#[derive(Clone, Debug)]
pub struct RenderedReplaySummary {
    pub path: PathBuf,
    pub file_name: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub modified_secs: u64,
}

pub fn replays_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(REPLAYS_DIR)
}

pub fn raw_replays_dir(game_dir: &Path) -> PathBuf {
    replays_dir(game_dir).join(RAW_REPLAYS_SUBDIR)
}

pub fn rendered_replays_dir(game_dir: &Path) -> PathBuf {
    replays_dir(game_dir).join(RENDERED_REPLAYS_SUBDIR)
}

pub fn sanitize_replay_stem(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(48));
    let mut prev_sep = false;
    for ch in raw.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            prev_sep = false;
            ch
        } else if ch.is_whitespace() || ch == '.' || ch == ':' || ch == '/' || ch == '\\' {
            if prev_sep {
                continue;
            }
            prev_sep = true;
            '_'
        } else {
            continue;
        };
        out.push(mapped);
        if out.len() >= 48 {
            break;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "replay".to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn unique_mcpr_path(dir: &Path, stem: &str) -> PathBuf {
    let clean = sanitize_replay_stem(stem);
    let candidate = dir.join(format!("{clean}.{MCPR_EXT}"));
    if !candidate.exists() {
        return candidate;
    }
    for idx in 2..10_000 {
        let next = dir.join(format!("{clean}_{idx}.{MCPR_EXT}"));
        if !next.exists() {
            return next;
        }
    }
    dir.join(format!("{clean}_{}.{MCPR_EXT}", chrono::Utc::now().timestamp()))
}

pub fn encode_raw_body(id: i32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 5);
    write_varint(&mut out, id);
    out.extend_from_slice(payload);
    out
}

pub fn decode_raw_body(body: &[u8]) -> Result<(i32, &[u8])> {
    let mut r = Reader::new(body);
    let id = r.varint()?;
    Ok((id, r.rest()))
}

pub fn write_tmcpr_record<W: Write>(
    writer: &mut W,
    crc: &mut Crc32,
    timestamp_ms: u32,
    body: &[u8],
) -> std::io::Result<usize> {
    let ts = timestamp_ms.to_be_bytes();
    let len = (body.len() as u32).to_be_bytes();
    writer.write_all(&ts)?;
    writer.write_all(&len)?;
    writer.write_all(body)?;
    crc.update(&ts);
    crc.update(&len);
    crc.update(body);
    Ok(8 + body.len())
}

/// Pure-Rust IEEE CRC32 (matches Java's `java.util.zip.CRC32` used in `.mcpr`).
#[derive(Clone, Debug, Default)]
pub struct Crc32 {
    state: u32,
}

impl Crc32 {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        let mut crc = !self.state;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        self.state = !crc;
    }

    pub fn finish(&self) -> u32 {
        self.state
    }
}

pub fn write_mcpr_archive(
    dest_path: &Path,
    meta: &ReplayMetadata,
    tmcpr_source_path: &Path,
    crc32_value: u32,
    markers: &[ReplayMarker],
    timelines: &ReplayTimelineSet,
    thumb_png: Option<&[u8]>,
) -> Result<()> {
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let stage = dest_path.with_extension("mcpr.tmp");
    {
        let file = File::create(&stage)?;
        let mut zip = zip::ZipWriter::new(BufWriter::new(file));
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("metaData.json", deflated)?;
        serde_json::to_writer_pretty(&mut zip, meta)?;

        zip.start_file("recording.tmcpr", deflated)?;
        let mut src = BufReader::new(File::open(tmcpr_source_path)?);
        std::io::copy(&mut src, &mut zip)?;

        zip.start_file("recording.tmcpr.crc32", deflated)?;
        write!(zip, "{crc32_value}")?;

        zip.start_file("markers.json", deflated)?;
        serde_json::to_writer(&mut zip, markers)?;

        zip.start_file("timelines.json", deflated)?;
        serde_json::to_writer_pretty(&mut zip, timelines)?;

        zip.start_file("mods.json", deflated)?;
        zip.write_all(b"[]")?;

        if let Some(thumb) = thumb_png {
            zip.start_file("thumb", stored)?;
            zip.write_all(thumb)?;
        }

        zip.finish()?;
    }
    fs::rename(&stage, dest_path)?;
    Ok(())
}

/// Reads metadata, markers, timelines, and whether a thumbnail exists without
/// extracting `recording.tmcpr`.
pub fn read_mcpr_summary(path: &Path) -> Result<ReplaySummary> {
    let size_bytes = fs::metadata(path).map_or(0, |m| m.len());
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file))?;

    let meta: ReplayMetadata = {
        let entry = zip.by_name("metaData.json")?;
        serde_json::from_reader(entry)?
    };
    let markers_count = zip
        .by_name("markers.json")
        .ok()
        .and_then(|e| serde_json::from_reader::<_, Vec<ReplayMarker>>(e).ok())
        .map_or(0, |v| v.len());
    let keyframes_count = zip
        .by_name("timelines.json")
        .ok()
        .and_then(|e| serde_json::from_reader::<_, ReplayTimelineSet>(e).ok())
        .map_or(0, |t| t.active().position_keyframes.len());
    let has_thumb = zip.by_name("thumb").is_ok();

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let display_name = path
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.clone());

    Ok(ReplaySummary {
        path: path.to_path_buf(),
        file_name,
        display_name,
        meta,
        markers_count,
        keyframes_count,
        size_bytes,
        has_thumb,
    })
}

pub fn list_replays(game_dir: &Path) -> Vec<ReplaySummary> {
    let raw_dir = raw_replays_dir(game_dir);
    let legacy_dir = replays_dir(game_dir);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in [raw_dir, legacy_dir] {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some(MCPR_EXT) {
                continue;
            }
            if !seen.insert(path.clone()) {
                continue;
            }
            if let Ok(summary) = read_mcpr_summary(&path) {
                out.push(summary);
            }
        }
    }
    out.sort_by(|a, b| b.meta.date.cmp(&a.meta.date));
    out
}

pub fn list_rendered_replays(game_dir: &Path) -> Vec<RenderedReplaySummary> {
    let dir = rendered_replays_dir(game_dir);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_none_or(|e| !e.eq_ignore_ascii_case("mp4"))
        {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let display_name = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_name.clone());
        let modified_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        out.push(RenderedReplaySummary {
            path,
            file_name,
            display_name,
            size_bytes: meta.len(),
            modified_secs,
        });
    }
    out.sort_by(|a, b| b.modified_secs.cmp(&a.modified_secs));
    out
}

pub fn delete_rendered_replay(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn rename_replay(path: &Path, new_name: &str) -> Result<PathBuf> {
    let parent = path.parent().ok_or("Replay has no parent directory")?;
    let clean = sanitize_replay_stem(new_name);
    let target = parent.join(format!("{clean}.{MCPR_EXT}"));
    if target == path {
        return Ok(target);
    }
    if target.exists() {
        return Err(format!("A replay named '{clean}' already exists").into());
    }
    fs::rename(path, &target)?;
    Ok(target)
}

pub fn delete_replay(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Extracts `recording.tmcpr` to a temporary file on disk and reads `metaData.json`,
/// `markers.json`, `timelines.json`, and builds an offset index over all packets.
pub fn extract_and_index_mcpr(
    mcpr_path: &Path,
    tmcpr_out_path: &Path,
) -> Result<(
    ReplayMetadata,
    Vec<ReplayMarker>,
    ReplayTimelineSet,
    Vec<PacketIndexEntry>,
)> {
    let file = File::open(mcpr_path)?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file))?;

    let meta: ReplayMetadata = {
        let entry = zip.by_name("metaData.json")?;
        serde_json::from_reader(entry)?
    };
    let markers: Vec<ReplayMarker> = zip
        .by_name("markers.json")
        .ok()
        .and_then(|e| serde_json::from_reader(e).ok())
        .unwrap_or_default();
    let timelines: ReplayTimelineSet = zip
        .by_name("timelines.json")
        .ok()
        .and_then(|e| serde_json::from_reader(e).ok())
        .unwrap_or_default();

    if let Some(parent) = tmcpr_out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    {
        let mut entry = zip.by_name("recording.tmcpr")?;
        let mut out = BufWriter::new(File::create(tmcpr_out_path)?);
        std::io::copy(&mut entry, &mut out)?;
        out.flush()?;
    }

    let index = index_tmcpr_file(tmcpr_out_path)?;
    Ok((meta, markers, timelines, index))
}

pub fn index_tmcpr_file(path: &Path) -> Result<Vec<PacketIndexEntry>> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut index = Vec::new();
    let mut state = PacketState::Configuration;
    let mut offset = 0u64;
    let mut hdr = [0u8; 8];
    let mut scratch = Vec::new();

    while offset + 8 <= file_len {
        if reader.read_exact(&mut hdr).is_err() {
            break;
        }
        let timestamp_ms = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let body_len = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
        if body_len == 0 || offset + 8 + body_len as u64 > file_len {
            break;
        }
        scratch.resize(body_len, 0);
        if reader.read_exact(&mut scratch).is_err() {
            break;
        }
        let pkt_state = state;
        if let Ok((id, _)) = decode_raw_body(&scratch) {
            match state {
                PacketState::Configuration => {
                    if id == ids::configuration::clientbound::FINISH_CONFIGURATION {
                        state = PacketState::Play;
                    }
                }
                PacketState::Play => {
                    if id == ids::play::clientbound::START_CONFIGURATION {
                        state = PacketState::Configuration;
                    }
                }
            }
        }
        index.push(PacketIndexEntry {
            file_offset: offset + 8,
            body_len,
            timestamp_ms,
            state: pkt_state,
        });
        offset += 8 + body_len as u64;
    }

    Ok(index)
}

/// Rewrites `markers.json` and `timelines.json` inside an existing `.mcpr` archive
/// while preserving `recording.tmcpr`, `metaData.json`, `mods.json`, and `thumb` byte-for-byte.
pub fn update_mcpr_edits(
    mcpr_path: &Path,
    markers: &[ReplayMarker],
    timelines: &ReplayTimelineSet,
) -> Result<()> {
    let stage = mcpr_path.with_extension("mcpr.edit.tmp");
    {
        let src_file = File::open(mcpr_path)?;
        let mut src_zip = zip::ZipArchive::new(BufReader::new(src_file))?;
        let dst_file = File::create(&stage)?;
        let mut dst_zip = zip::ZipWriter::new(BufWriter::new(dst_file));
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for i in 0..src_zip.len() {
            let mut entry = src_zip.by_index(i)?;
            let name = entry.name()?.into_owned();
            if name == "markers.json" || name == "timelines.json" {
                continue;
            }
            let opts = if name == "thumb" {
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored)
            } else {
                deflated
            };
            dst_zip.start_file(&name, opts)?;
            std::io::copy(&mut entry, &mut dst_zip)?;
        }

        dst_zip.start_file("markers.json", deflated)?;
        serde_json::to_writer(&mut dst_zip, markers)?;

        dst_zip.start_file("timelines.json", deflated)?;
        serde_json::to_writer_pretty(&mut dst_zip, timelines)?;

        dst_zip.finish()?;
    }
    fs::rename(&stage, mcpr_path)?;
    Ok(())
}

pub fn format_duration_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let tenths = (ms % 1000) / 100;
    if mins >= 60 {
        let hrs = mins / 60;
        let rem_mins = mins % 60;
        format!("{hrs}:{rem_mins:02}:{secs:02}.{tenths}")
    } else {
        format!("{mins:02}:{secs:02}.{tenths}")
    }
}

pub fn format_size_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
