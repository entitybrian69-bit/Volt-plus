//! Replay `.mcpr` playback controller: indexed `.tmcpr` packet streaming,
//! forward/backward seeking, free-cam with roll/FOV, path preview, and offline MP4 render state.

use super::{
    archive::{
        PacketIndexEntry, PacketState, ReplayMetadata, decode_raw_body, extract_and_index_mcpr,
        update_mcpr_edits,
    },
    timeline::{
        Interpolation, PositionKeyframe, ReplayMarker, ReplayTimelineSet, TimeKeyframe,
        wrap_degrees,
    },
};
use crate::Result;
use horizon_protocol::{
    packets::{configuration as cfg, play},
    registry::Dynamic,
};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherOverride {
    Clear,
    Rain,
    Thunder,
}

impl WeatherOverride {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::Rain => "Rain",
            Self::Thunder => "Thunder",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    FreeCam,
    Spectate(u32),
    PathPreview,
    RenderPath,
}

#[derive(Clone, Debug)]
pub enum ReplayStep {
    Joined {
        registries: Dynamic,
        uuid: u128,
        name: String,
    },
    Reconfiguring,
    Reconfigured {
        registries: Dynamic,
    },
    Packet(Box<play::Clientbound>),
}

#[derive(Clone, Debug)]
pub struct ReplayRenderJob {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub current_frame: u64,
    pub total_frames: u64,
    pub finishing: bool,
    pub awaiting_readback: bool,
    pub status_text: String,
    pub saved_uri: Option<String>,
    pub error: Option<String>,
}

pub struct ReplayPlayer {
    pub mcpr_path: PathBuf,
    temp_tmcpr: PathBuf,
    reader: BufReader<File>,
    pub meta: ReplayMetadata,
    pub markers: Vec<ReplayMarker>,
    pub timelines: ReplayTimelineSet,
    index: Vec<PacketIndexEntry>,
    cursor: usize,
    pending_registries: Dynamic,
    joined_once: bool,
    reconfiguring: bool,

    pub playing: bool,
    pub speed: f32,
    pub loop_playback: bool,
    pub current_ms: u64,
    pub duration_ms: u64,
    pub path_cursor_ms: u64,
    pub path_playing: bool,
    pub sync_time_with_path: bool,

    pub camera_mode: CameraMode,
    pub cam_pos: [f64; 3],
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    pub cam_roll: f32,
    pub cam_fov: f32,
    pub fly_speed: f32,
    pub cam_initialized: bool,

    pub show_hud: bool,
    pub show_path_gizmos: bool,
    pub hide_nametags: bool,
    pub hide_players: bool,
    pub time_override: Option<i64>,
    pub weather_override: Option<WeatherOverride>,
    pub players_drawer_open: bool,
    pub markers_drawer_open: bool,
    pub render_modal_open: bool,
    pub status_banner: Option<(String, std::time::Instant)>,
    pub render_job: Option<ReplayRenderJob>,
    pub dirty_edits: bool,
}

impl Drop for ReplayPlayer {
    fn drop(&mut self) {
        if self.dirty_edits {
            let _ = self.save_edits();
        }
        let _ = fs::remove_file(&self.temp_tmcpr);
    }
}

impl ReplayPlayer {
    pub fn open(mcpr_path: &Path) -> Result<Self> {
        let dir = mcpr_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let temp_tmcpr = dir.join(format!(
            ".play_{}_{}.tmcpr",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        ));
        let (meta, markers, timelines, index) = extract_and_index_mcpr(mcpr_path, &temp_tmcpr)?;
        let duration_ms = meta
            .duration
            .max(index.last().map_or(0, |e| u64::from(e.timestamp_ms)));
        let reader = BufReader::new(File::open(&temp_tmcpr)?);
        let path_cursor_ms = timelines
            .active()
            .position_keyframes
            .first()
            .map_or(0, |k| k.timeline_ms);

        Ok(Self {
            mcpr_path: mcpr_path.to_path_buf(),
            temp_tmcpr,
            reader,
            meta,
            markers,
            timelines,
            index,
            cursor: 0,
            pending_registries: Dynamic::default(),
            joined_once: false,
            reconfiguring: false,
            playing: true,
            speed: 1.0,
            loop_playback: false,
            current_ms: 0,
            duration_ms,
            path_cursor_ms,
            path_playing: false,
            sync_time_with_path: true,
            camera_mode: CameraMode::FreeCam,
            cam_pos: [0.0, 80.0, 0.0],
            cam_yaw: 0.0,
            cam_pitch: 0.0,
            cam_roll: 0.0,
            cam_fov: 80.0,
            fly_speed: 12.0,
            cam_initialized: false,
            show_hud: true,
            show_path_gizmos: true,
            hide_nametags: false,
            hide_players: false,
            time_override: None,
            weather_override: None,
            players_drawer_open: false,
            markers_drawer_open: false,
            render_modal_open: false,
            status_banner: None,
            render_job: None,
            dirty_edits: false,
        })
    }

    pub fn set_banner(&mut self, msg: impl Into<String>) {
        self.status_banner = Some((msg.into(), std::time::Instant::now()));
    }

    pub fn active_banner(&self) -> Option<&str> {
        self.status_banner.as_ref().and_then(|(msg, at)| {
            (at.elapsed().as_secs_f32() < 3.5).then_some(msg.as_str())
        })
    }

    pub fn save_edits(&mut self) -> Result<()> {
        update_mcpr_edits(&self.mcpr_path, &self.markers, &self.timelines)?;
        self.dirty_edits = false;
        self.set_banner("Saved keyframes & markers to .mcpr");
        Ok(())
    }

    pub fn add_position_keyframe_here(&mut self) {
        let mut t = if self.path_playing {
            self.path_cursor_ms
        } else {
            self.current_ms
        };
        {
            let tl = self.timelines.active();
            while tl
                .position_keyframes
                .iter()
                .any(|k| k.timeline_ms.abs_diff(t) <= 25)
            {
                t = t.saturating_add(2000);
            }
        }
        self.path_cursor_ms = t;
        let kf = PositionKeyframe {
            timeline_ms: t,
            x: self.cam_pos[0],
            y: self.cam_pos[1],
            z: self.cam_pos[2],
            yaw: wrap_degrees(self.cam_yaw),
            pitch: self.cam_pitch.clamp(-90.0, 90.0),
            roll: wrap_degrees(self.cam_roll),
            fov: self.cam_fov.clamp(20.0, 140.0),
            interp: self.timelines.active().interpolation,
        };
        self.timelines.active_mut().add_position_keyframe(kf);
        self.dirty_edits = true;
        self.set_banner(format!(
            "Added Pos KF #{} @ {}",
            self.timelines.active().position_keyframes.len(),
            super::archive::format_duration_ms(t)
        ));
    }

    pub fn add_time_keyframe_here(&mut self) {
        let mut t = if self.path_playing {
            self.path_cursor_ms
        } else {
            self.current_ms
        };
        {
            let tl = self.timelines.active();
            while tl
                .time_keyframes
                .iter()
                .any(|k| k.timeline_ms.abs_diff(t) <= 25)
            {
                t = t.saturating_add(2000);
            }
        }
        self.path_cursor_ms = t;
        let kf = TimeKeyframe {
            timeline_ms: t,
            replay_time_ms: self.current_ms,
        };
        self.timelines.active_mut().add_time_keyframe(kf);
        self.dirty_edits = true;
        self.set_banner(format!(
            "Added Time KF #{} ({})",
            self.timelines.active().time_keyframes.len(),
            super::archive::format_duration_ms(self.current_ms)
        ));
    }

    pub fn add_synced_keyframes_here(&mut self) {
        let mut t = if self.path_playing {
            self.path_cursor_ms
        } else {
            self.current_ms
        };
        {
            let tl = self.timelines.active();
            while tl
                .position_keyframes
                .iter()
                .any(|k| k.timeline_ms.abs_diff(t) <= 25)
                || tl
                    .time_keyframes
                    .iter()
                    .any(|k| k.timeline_ms.abs_diff(t) <= 25)
            {
                t = t.saturating_add(2000);
            }
        }
        self.path_cursor_ms = t;
        let pos_kf = PositionKeyframe {
            timeline_ms: t,
            x: self.cam_pos[0],
            y: self.cam_pos[1],
            z: self.cam_pos[2],
            yaw: wrap_degrees(self.cam_yaw),
            pitch: self.cam_pitch.clamp(-90.0, 90.0),
            roll: wrap_degrees(self.cam_roll),
            fov: self.cam_fov.clamp(20.0, 140.0),
            interp: self.timelines.active().interpolation,
        };
        let time_kf = TimeKeyframe {
            timeline_ms: t,
            replay_time_ms: self.current_ms,
        };
        let tl = self.timelines.active_mut();
        tl.add_position_keyframe(pos_kf);
        tl.add_time_keyframe(time_kf);
        self.dirty_edits = true;
        self.set_banner(format!(
            "Added Sync KF #{} @ {}",
            tl.position_keyframes.len(),
            super::archive::format_duration_ms(t)
        ));
    }

    pub fn add_marker_here(&mut self) {
        let idx = self.markers.len() + 1;
        let marker = ReplayMarker {
            name: format!("Marker #{idx}"),
            time_ms: self.current_ms,
            x: self.cam_pos[0],
            y: self.cam_pos[1],
            z: self.cam_pos[2],
            yaw: self.cam_yaw,
            pitch: self.cam_pitch,
            roll: self.cam_roll,
        };
        self.markers.push(marker);
        self.markers.sort_by_key(|m| m.time_ms);
        self.dirty_edits = true;
        self.set_banner(format!("Added Marker #{idx}"));
    }

    pub fn toggle_interpolation(&mut self) -> Interpolation {
        let next = self.timelines.active().interpolation.toggle();
        self.timelines.active_mut().interpolation = next;
        for kf in &mut self.timelines.active_mut().position_keyframes {
            kf.interp = next;
        }
        self.dirty_edits = true;
        self.set_banner(format!("Path: {}", next.label()));
        next
    }

    pub fn clear_path(&mut self) {
        self.timelines.active_mut().clear();
        self.path_playing = false;
        self.path_cursor_ms = 0;
        if self.camera_mode == CameraMode::PathPreview {
            self.camera_mode = CameraMode::FreeCam;
        }
        self.dirty_edits = true;
        self.set_banner("Cleared camera path");
    }

    pub fn advance(&mut self, dt_secs: f32) -> Option<u64> {
        if self.camera_mode == CameraMode::RenderPath {
            return None;
        }
        if self.path_playing {
            let active = self.timelines.active();
            let path_dur = active.duration_ms().max(1);
            let step_ms = (dt_secs * 1000.0 * self.speed.max(0.05)).round() as u64;
            let next_path = self.path_cursor_ms.saturating_add(step_ms.max(1));
            if next_path >= path_dur {
                if self.loop_playback {
                    self.path_cursor_ms = 0;
                } else {
                    self.path_cursor_ms = path_dur;
                    self.path_playing = false;
                    self.camera_mode = CameraMode::FreeCam;
                }
            } else {
                self.path_cursor_ms = next_path;
            }
            if let Some(pose) = self.timelines.active().sample_camera_pose(self.path_cursor_ms) {
                self.cam_pos = [pose.x, pose.y, pose.z];
                self.cam_yaw = pose.yaw;
                self.cam_pitch = pose.pitch;
                self.cam_roll = pose.roll;
                self.cam_fov = pose.fov;
            }
            if self.sync_time_with_path && !self.timelines.active().time_keyframes.is_empty() {
                let target_replay = self
                    .timelines
                    .active()
                    .sample_replay_time_ms(self.path_cursor_ms, self.current_ms)
                    .min(self.duration_ms);
                return Some(target_replay);
            }
        }
        if self.playing {
            let step_ms = (dt_secs * 1000.0 * self.speed.max(0.05)).round() as u64;
            let next = self.current_ms.saturating_add(step_ms.max(1));
            if next >= self.duration_ms {
                if self.loop_playback && self.duration_ms > 0 {
                    return Some(0);
                }
                self.playing = false;
                return Some(self.duration_ms);
            }
            return Some(next);
        }
        None
    }

    pub fn prepare_seek(&mut self, target_ms: u64) -> bool {
        let target = target_ms.min(self.duration_ms);
        let needs_rewind = target < self.current_ms;
        if needs_rewind {
            self.cursor = 0;
            self.pending_registries = Dynamic::default();
            self.joined_once = false;
            self.reconfiguring = false;
            self.current_ms = 0;
        }
        needs_rewind
    }

    pub fn drain_packets_up_to(&mut self, target_ms: u64, limit: usize) -> Vec<ReplayStep> {
        let target = target_ms.min(self.duration_ms);
        let mut out = Vec::new();
        let mut body = Vec::new();

        while self.cursor < self.index.len() && out.len() < limit {
            let entry = &self.index[self.cursor];
            if u64::from(entry.timestamp_ms) > target {
                break;
            }
            self.cursor += 1;
            body.resize(entry.body_len, 0);
            if self.reader.seek(SeekFrom::Start(entry.file_offset)).is_err()
                || self.reader.read_exact(&mut body).is_err()
            {
                break;
            }
            let Ok((id, payload)) = decode_raw_body(&body) else {
                continue;
            };
            match entry.state {
                PacketState::Configuration => {
                    let Ok(pkt) = cfg::Clientbound::decode(id, payload) else {
                        continue;
                    };
                    match pkt {
                        cfg::Clientbound::RegistryData(r) => self.pending_registries.add(r),
                        cfg::Clientbound::UpdateTags(t) => self.pending_registries.add_tags(&t),
                        cfg::Clientbound::FinishConfiguration(_) => {
                            let reg = std::mem::take(&mut self.pending_registries);
                            if !self.joined_once {
                                self.joined_once = true;
                                out.push(ReplayStep::Joined {
                                    registries: reg,
                                    uuid: 0,
                                    name: "ReplayViewer".to_owned(),
                                });
                            } else {
                                self.reconfiguring = false;
                                out.push(ReplayStep::Reconfigured { registries: reg });
                            }
                        }
                        _ => {}
                    }
                }
                PacketState::Play => {
                    let Ok(pkt) = play::Clientbound::decode(id, payload) else {
                        continue;
                    };
                    match pkt {
                        play::Clientbound::StartConfiguration(_) => {
                            self.reconfiguring = true;
                            self.pending_registries = Dynamic::default();
                            out.push(ReplayStep::Reconfiguring);
                        }
                        play::Clientbound::KeepAlive(_)
                        | play::Clientbound::Disconnect(_)
                        | play::Clientbound::Transfer(_)
                        | play::Clientbound::CookieRequest(_)
                        | play::Clientbound::ResourcePackPush(_)
                        | play::Clientbound::ResourcePackPop(_)
                        | play::Clientbound::ShowDialog(_)
                        | play::Clientbound::ClearDialog(_)
                        | play::Clientbound::OpenScreen(_)
                        | play::Clientbound::ContainerSetContent(_)
                        | play::Clientbound::ContainerSetSlot(_)
                        | play::Clientbound::SetCursorItem(_)
                        | play::Clientbound::OpenBook(_) => {}
                        other => out.push(ReplayStep::Packet(Box::new(other))),
                    }
                }
            }
        }

        if self.cursor >= self.index.len()
            || u64::from(self.index[self.cursor].timestamp_ms) > target
        {
            self.current_ms = target;
        } else if let Some(last) = out.last() {
            let _ = last;
            if self.cursor > 0 {
                self.current_ms = u64::from(self.index[self.cursor - 1].timestamp_ms);
            }
        }
        out
    }

    pub fn at_end(&self) -> bool {
        self.cursor >= self.index.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::archive::{
        Crc32, write_mcpr_archive, write_tmcpr_record,
    };
    use horizon_protocol::packets::{Empty, configuration as cfg, play};

    #[test]
    fn mcpr_roundtrip_and_timeline_spline() {
        let dir = std::env::temp_dir().join(format!("volt_replay_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let tmcpr = dir.join("raw.tmcpr");
        let mut crc = Crc32::new();
        {
            let mut f = File::create(&tmcpr).unwrap();
            let fin = cfg::Clientbound::FinishConfiguration(Empty).encode();
            write_tmcpr_record(&mut f, &mut crc, 0, &fin).unwrap();
            let time = play::Clientbound::SetTime(play::SetTime {
                game_time: 100,
                clock_updates: vec![],
            })
            .encode();
            write_tmcpr_record(&mut f, &mut crc, 500, &time).unwrap();
        }
        let mcpr = dir.join("test_session.mcpr");
        let meta = ReplayMetadata {
            singleplayer: false,
            server_name: "play.example.test".to_owned(),
            duration: 1000,
            ..ReplayMetadata::default()
        };
        let mut timelines = ReplayTimelineSet::default();
        timelines.active_mut().add_position_keyframe(PositionKeyframe {
            timeline_ms: 0,
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 170.0,
            pitch: 0.0,
            roll: 0.0,
            fov: 70.0,
            interp: Interpolation::CatmullRom,
        });
        timelines.active_mut().add_position_keyframe(PositionKeyframe {
            timeline_ms: 1000,
            x: 10.0,
            y: 74.0,
            z: -5.0,
            yaw: -170.0,
            pitch: 15.0,
            roll: 10.0,
            fov: 90.0,
            interp: Interpolation::CatmullRom,
        });
        write_mcpr_archive(&mcpr, &meta, &tmcpr, crc.finish(), &[], &timelines, None).unwrap();

        let mut player = ReplayPlayer::open(&mcpr).unwrap();
        assert_eq!(player.meta.server_name, "play.example.test");
        let pose = player.timelines.active().sample_camera_pose(500).unwrap();
        assert!((pose.x - 5.0).abs() < 0.01);
        // Shortest-angle path from +170 to -170 crosses 180 / -180, not 0!
        assert!(pose.yaw.abs() > 175.0);

        let steps = player.drain_packets_up_to(600, 100);
        assert!(matches!(steps.first(), Some(ReplayStep::Joined { .. })));
        assert!(matches!(steps.get(1), Some(ReplayStep::Packet(_))));

        drop(player);
        let _ = fs::remove_dir_all(&dir);
    }
}
