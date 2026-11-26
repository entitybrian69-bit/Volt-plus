//! Replay Mod runtime integration inside `Game`:
//! - Opens `.mcpr` files via `Session::replay` + `ReplayPlayer`
//! - Advances playback, handles forward/backward seeking cleanly
//! - Drives Free-Cam (touch + WASD/Space/Shift + roll + FOV), Entity Spectating,
//!   Camera Path Preview (Catmull-Rom spline), and 3D Gizmos
//! - Steps deterministic offline MP4 path rendering (`ReplayVideoEncoder`)
//! - Synthesizes local-player ("self") packets during live SP/MP recording

use super::{Dim, Game, Screen};
use horizon_client::{
    Result,
    replay::{
        self, CameraMode, ReplayAction, ReplayHudSnapshot, ReplayPlayer, ReplayRenderJob,
        ReplayStep, WeatherOverride,
    },
};
use horizon_protocol::{
    packets::play,
    registry::{Builtin, DimensionType},
};
use std::{path::Path, sync::Arc, time::Instant};
use winit::keyboard::KeyCode;

impl Game<'_> {
    pub(super) fn is_replay(&self) -> bool {
        self.replay.is_some()
    }

    /// Launches straight into Replay Viewer mode for `mcpr_path`.
    pub(super) fn open_replay(&mut self, mcpr_path: &Path) -> Result<()> {
        if self.defer_world_entry(super::PendingWorldEntry::Replay {
            path: mcpr_path.to_path_buf(),
        })? {
            return Ok(());
        }
        self.join_world_rest()?;
        // Never record a replay while watching a replay.
        replay::global_recorder().discard();
        let player = ReplayPlayer::open(mcpr_path)?;
        let title_name = player.meta.server_name.clone();
        self.init_replay_remote(&title_name)?;
        self.replay = Some(player);
        self.replay_seeking = false;
        self.replay_last_step = Some(Instant::now());
        // Apply initial config + first packets so terrain starts loading right away.
        if let Err(err) = self.poll_replay_packets(0) {
            log::warn!("Replay initial packet warning: {err}");
        }
        self.replay_publish_hud_snapshot();
        Ok(())
    }

    pub(super) fn exit_replay_to_title(&mut self) -> Result<()> {
        if let Some(mut r) = self.replay.take() {
            if let Some(job) = r.render_job.take()
                && !job.finishing
            {
                replay::video_export_cancel();
            }
            if r.dirty_edits {
                let _ = r.save_edits();
            }
        }
        self.replay_seeking = false;
        self.replay_last_step = None;
        self.replay_render_wait_frames = 0;
        #[cfg(target_os = "android")]
        {
            self.pad.touch_ui.replay_look_finger = None;
            self.pad.touch_ui.replay_fly_pad = None;
            self.pad.touch_ui.replay_fly_vert = None;
            self.pad.touch_ui.replay_timeline_drag = None;
            self.pad.touch_ui.replay_scrub_preview_ms = None;
            self.pad.touch_ui.replay_last_seek_at = None;
        }
        replay::publish_hud_snapshot(ReplayHudSnapshot::default());
        self.leave_server(None)?;
        #[cfg(target_os = "android")]
        horizon_client::android_runtime::return_to_launcher();
        Ok(())
    }

    pub(super) fn poll_replay(&mut self, elapsed: std::time::Duration) -> Result<()> {
        let Some(replay) = self.replay.as_mut() else {
            return Ok(());
        };
        if replay.camera_mode == CameraMode::RenderPath {
            return Ok(());
        }
        let dt = elapsed.as_secs_f32().clamp(0.0, 0.1);
        let next_ms = replay.advance(dt);
        if let Some(target_ms) = next_ms {
            self.poll_replay_packets(target_ms)?;
        }
        Ok(())
    }

    pub(super) fn seek_replay_to(&mut self, target_ms: u64) -> Result<()> {
        let Some(replay) = self.replay.as_mut() else {
            return Ok(());
        };
        let clamped_target = target_ms.min(replay.duration_ms);
        if clamped_target == replay.current_ms {
            return Ok(());
        }
        let needs_rewind = replay.prepare_seek(clamped_target);
        if needs_rewind {
            self.reset_remote_for_replay_rewind();
        }
        self.replay_seeking = true;
        // Drain up to target timestamp cleanly in batches.
        let mut guard = 0;
        loop {
            let steps = {
                let Some(replay) = self.replay.as_mut() else {
                    break;
                };
                replay.drain_packets_up_to(clamped_target, 512)
            };
            if steps.is_empty() {
                break;
            }
            self.apply_replay_steps(steps)?;
            guard += 1;
            if guard > 512 {
                break;
            }
        }
        self.replay_seeking = false;
        Ok(())
    }

    pub(super) fn poll_replay_packets(&mut self, target_ms: u64) -> Result<()> {
        let steps = {
            let Some(replay) = self.replay.as_mut() else {
                return Ok(());
            };
            replay.drain_packets_up_to(target_ms, 256)
        };
        self.apply_replay_steps(steps)
    }

    fn apply_replay_steps(&mut self, steps: Vec<ReplayStep>) -> Result<()> {
        for step in steps {
            match step {
                ReplayStep::Joined {
                    registries,
                    uuid,
                    name,
                } => {
                    self.handle_replay_joined(Arc::new(registries), uuid, name);
                }
                ReplayStep::Reconfiguring => {
                    self.handle_replay_reconfiguring();
                }
                ReplayStep::Reconfigured { registries } => {
                    self.handle_replay_reconfigured(Arc::new(registries));
                }
                ReplayStep::Packet(packet) => {
                    if self.replay_seeking
                        && matches!(
                            *packet,
                            play::Clientbound::Sound(_)
                                | play::Clientbound::SoundEntity(_)
                                | play::Clientbound::LevelParticles(_)
                                | play::Clientbound::LevelEvent(_)
                        )
                    {
                        continue;
                    }
                    self.handle_replay_packet(*packet)?;
                    if let Some(replay) = self.replay.as_mut()
                        && !replay.cam_initialized
                    {
                        if let Some(p) = &self.player {
                            replay.cam_pos = [
                                p.position.x,
                                p.position.y + f64::from(p.dimensions.eye_height),
                                p.position.z,
                            ];
                            replay.cam_yaw = self.yaw;
                            replay.cam_pitch = self.pitch;
                            replay.cam_initialized = true;
                        } else if let Some(m) = self.mobs.first() {
                            replay.cam_pos = [m.position.x, m.eye_y(), m.position.z];
                            replay.cam_yaw = m.head_yaw;
                            replay.cam_pitch = m.head_pitch;
                            replay.cam_initialized = true;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn replay_camera_override(
        &self,
        _alpha: f32,
    ) -> Option<(glam::DVec3, f32, f32, f32, f32)> {
        let replay = self.replay.as_ref()?;
        match replay.camera_mode {
            CameraMode::Spectate(id) => {
                if let Some(m) = self.mobs.iter().find(|m| m.id == id) {
                    return Some((
                        glam::DVec3::new(m.position.x, m.eye_y(), m.position.z),
                        m.head_yaw,
                        m.head_pitch,
                        replay.cam_roll,
                        replay.cam_fov,
                    ));
                }
            }
            CameraMode::PathPreview => {
                if let Some(pose) = replay.timelines.active().sample_camera_pose(replay.path_cursor_ms) {
                    return Some((
                        glam::DVec3::new(pose.x, pose.y, pose.z),
                        pose.yaw,
                        pose.pitch,
                        pose.roll,
                        pose.fov,
                    ));
                }
            }
            CameraMode::RenderPath => {
                if let Some(job) = &replay.render_job {
                    let span = job.end_ms.saturating_sub(job.start_ms);
                    let denom = job.total_frames.saturating_sub(1).max(1);
                    let timeline_ms = job.start_ms + (span * job.current_frame) / denom;
                    if let Some(pose) = replay.timelines.active().sample_camera_pose(timeline_ms) {
                        return Some((
                            glam::DVec3::new(pose.x, pose.y, pose.z),
                            pose.yaw,
                            pose.pitch,
                            pose.roll,
                            pose.fov,
                        ));
                    }
                }
            }
            CameraMode::FreeCam => {}
        }
        Some((
            glam::DVec3::from_array(replay.cam_pos),
            replay.cam_yaw,
            replay.cam_pitch,
            replay.cam_roll,
            replay.cam_fov,
        ))
    }

    pub(super) fn replay_update_freecam(&mut self, dt_secs: f32) {
        if self.replay.is_none() || self.screen != Screen::Playing || self.dialog_open() {
            return;
        }
        let dt = dt_secs.clamp(0.0, 0.1);
        let mut forward = 0.0_f64;
        let mut strafe = 0.0_f64;
        let mut vertical = 0.0_f64;

        if self.keys.contains(&KeyCode::KeyW) {
            forward += 1.0;
        }
        if self.keys.contains(&KeyCode::KeyS) {
            forward -= 1.0;
        }
        if self.keys.contains(&KeyCode::KeyD) {
            strafe += 1.0;
        }
        if self.keys.contains(&KeyCode::KeyA) {
            strafe -= 1.0;
        }
        if self.keys.contains(&KeyCode::Space) {
            vertical += 1.0;
        }
        if self.keys.contains(&KeyCode::ShiftLeft) {
            vertical -= 1.0;
        }

        #[cfg(target_os = "android")]
        {
            let fly = self.pad.touch_ui.replay_fly_move();
            strafe += f64::from(fly[0]);
            forward += f64::from(fly[1]);
            vertical += f64::from(fly[2]);
        }

        let boost = if self.keys.contains(&KeyCode::ControlLeft) {
            2.2
        } else {
            1.0
        };

        let Some(replay) = self.replay.as_mut() else {
            return;
        };
        if matches!(
            replay.camera_mode,
            CameraMode::PathPreview | CameraMode::RenderPath
        ) {
            return;
        }
        if matches!(replay.camera_mode, CameraMode::Spectate(_))
            && (forward.abs() > 0.1 || strafe.abs() > 0.1 || vertical.abs() > 0.1)
        {
            replay.camera_mode = CameraMode::FreeCam;
        }
        if replay.camera_mode != CameraMode::FreeCam {
            return;
        }

        let yaw_rad = f64::from(replay.cam_yaw).to_radians();
        let pitch_rad = f64::from(replay.cam_pitch).to_radians();
        let fwd = [
            -yaw_rad.sin() * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        ];
        let right = [-yaw_rad.cos(), 0.0, -yaw_rad.sin()];
        let speed = f64::from(replay.fly_speed) * boost * f64::from(dt);

        replay.cam_pos[0] += (fwd[0] * forward + right[0] * strafe) * speed;
        replay.cam_pos[1] += (fwd[1] * forward + vertical) * speed;
        replay.cam_pos[2] += (fwd[2] * forward + right[2] * strafe) * speed;
    }

    pub(super) fn replay_sky_overrides(&self, day_time: i64, rain: f32, thunder: f32) -> (i64, f32, f32) {
        if let Some(r) = self.replay.as_ref() {
            let t = r.time_override.unwrap_or(day_time);
            let (rn, th) = match r.weather_override {
                None => (rain, thunder),
                Some(WeatherOverride::Clear) => (0.0, 0.0),
                Some(WeatherOverride::Rain) => (1.0, 0.0),
                Some(WeatherOverride::Thunder) => (1.0, 1.0),
            };
            return (t, rn, th);
        }
        let cs = &self.settings.extras.volt.clear_skies;
        if !cs.enabled {
            return (day_time, rain, thunder);
        }
        let t = match cs.time_override {
            horizon_client::extras::TimeMood::Default => day_time,
            horizon_client::extras::TimeMood::Noon => 6000,
            horizon_client::extras::TimeMood::Sunset => 12500,
            horizon_client::extras::TimeMood::Midnight => 18000,
        };
        let (rn, th) = if cs.hide_rain_snow {
            (0.0, 0.0)
        } else {
            (rain, thunder)
        };
        (t, rn, th)
    }

    pub(super) fn replay_gizmos(&self) -> Vec<horizon_client::gizmos::Line> {
        let Some(r) = self.replay.as_ref() else {
            return Vec::new();
        };
        if !r.show_path_gizmos || r.camera_mode == CameraMode::RenderPath {
            return Vec::new();
        }
        r.timelines
            .active()
            .path_preview_lines()
            .into_iter()
            .map(|(start, end, color)| horizon_client::gizmos::Line {
                start,
                end,
                color,
                width: 2.5,
            })
            .collect()
    }

    pub(super) fn replay_step_render_pre_frame(&mut self) -> Result<()> {
        let seek_target = {
            let Some(replay) = self.replay.as_mut() else {
                return Ok(());
            };
            let Some(job) = replay.render_job.as_mut() else {
                return Ok(());
            };
            if job.finishing || job.awaiting_readback || job.error.is_some() {
                return Ok(());
            }
            if job.current_frame >= job.total_frames {
                job.finishing = true;
                job.status_text = "Finalizing MP4...".to_owned();
                None
            } else {
                let span = job.end_ms.saturating_sub(job.start_ms);
                let denom = job.total_frames.saturating_sub(1).max(1);
                let timeline_ms = job.start_ms + (span * job.current_frame) / denom;
                let target_replay = replay
                    .timelines
                    .active()
                    .sample_replay_time_ms(timeline_ms, timeline_ms.min(replay.duration_ms));
                replay.path_cursor_ms = timeline_ms;
                job.status_text = format!(
                    "Rendering frame {} / {}",
                    job.current_frame + 1,
                    job.total_frames
                );
                Some(target_replay)
            }
        };

        if let Some(target_ms) = seek_target {
            self.seek_replay_to(target_ms)?;
            if let Some((cam_pos, _, _, _, _)) = self.replay_camera_override(1.0) {
                self.streaming.center = (
                    (cam_pos.x.floor() as i32) >> 4,
                    (cam_pos.z.floor() as i32) >> 4,
                );
            }
            if (self.streaming.busy() || !self.pending_uploads.is_empty())
                && self.replay_render_wait_frames < 45
            {
                self.replay_render_wait_frames += 1;
                return Ok(());
            }
            self.replay_render_wait_frames = 0;
            if let Some(replay) = self.replay.as_mut()
                && let Some(job) = replay.render_job.as_mut()
            {
                job.awaiting_readback = true;
            }
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.request_replay_frame();
            }
        } else if self
            .replay
            .as_ref()
            .and_then(|r| r.render_job.as_ref())
            .is_some_and(|j| j.finishing)
        {
            let finish_res = replay::video_export_finish();
            let mut exported_ok = false;
            if let Some(replay) = self.replay.as_mut() {
                replay.camera_mode = CameraMode::FreeCam;
                match finish_res {
                    Ok(saved) => {
                        replay.render_job = None;
                        replay.set_banner(format!("Exported MP4: {saved}"));
                        exported_ok = true;
                    }
                    Err(err) => {
                        replay.render_job = None;
                        replay.set_banner(format!("Export failed: {err}"));
                    }
                }
            }
            if exported_ok {
                #[cfg(target_os = "android")]
                self.exit_replay_to_title()?;
            }
        }
        Ok(())
    }

    pub(super) fn replay_step_render_post_frame(&mut self) {
        let Some(frame_data) = self
            .renderer
            .as_mut()
            .and_then(|r| r.take_replay_frame())
        else {
            return;
        };
        let Some(replay) = self.replay.as_mut() else {
            return;
        };
        let Some(job) = replay.render_job.as_mut() else {
            return;
        };
        job.awaiting_readback = false;
        let (w, h, rgba) = frame_data;
        let pts_us = (job.current_frame * 1_000_000) / u64::from(job.fps.max(1));
        match replay::video_export_push_frame(&rgba, w, h, pts_us) {
            Ok(()) => {
                job.current_frame += 1;
            }
            Err(err) => {
                replay::video_export_cancel();
                replay.camera_mode = CameraMode::FreeCam;
                replay.render_job = None;
                replay.set_banner(format!("Render aborted: {err}"));
            }
        }
    }

    pub(super) fn replay_publish_hud_snapshot(&self) {
        let Some(r) = self.replay.as_ref() else {
            replay::publish_hud_snapshot(ReplayHudSnapshot::default());
            return;
        };
        let active_tl = r.timelines.active();
        let spectating = match r.camera_mode {
            CameraMode::Spectate(id) => Some(id),
            _ => None,
        };
        replay::publish_hud_snapshot(ReplayHudSnapshot {
            active: true,
            title: r.meta.server_name.clone(),
            playing: r.playing,
            speed: r.speed,
            loop_playback: r.loop_playback,
            current_ms: r.current_ms,
            duration_ms: r.duration_ms,
            path_cursor_ms: r.path_cursor_ms,
            path_duration_ms: active_tl.duration_ms().max(r.duration_ms),
            path_playing: r.path_playing,
            sync_time_with_path: r.sync_time_with_path,
            cam_pos: r.cam_pos,
            cam_yaw: r.cam_yaw,
            cam_pitch: r.cam_pitch,
            cam_roll: r.cam_roll,
            cam_fov: r.cam_fov,
            fly_speed: r.fly_speed,
            spectating_entity: spectating,
            show_hud: r.show_hud,
            show_path_gizmos: r.show_path_gizmos,
            hide_nametags: r.hide_nametags,
            hide_players: r.hide_players,
            interpolation: active_tl.interpolation,
            time_override: r.time_override,
            weather_override: r.weather_override,
            players_drawer_open: r.players_drawer_open,
            markers_drawer_open: r.markers_drawer_open,
            render_modal_open: r.render_modal_open,
            position_keyframes: active_tl.position_keyframes.clone(),
            time_keyframes: active_tl.time_keyframes.clone(),
            markers: r.markers.clone(),
            players: self.replay_player_rows(),
            banner: r.active_banner().map(str::to_owned),
            render_job: r.render_job.clone(),
        });
    }

    pub(super) fn sync_replay_local_player(&mut self) {
        if self.replay.is_some() || self.options.split_guest {
            return;
        }
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let Some(r) = self.remote.as_ref() else {
            return;
        };
        if !r.joined || !replay::global_recorder().is_active() {
            return;
        }
        let main_item = self
            .inventory
            .get(self.selected)
            .and_then(|s| s.as_ref())
            .and_then(|s| r.builtin.get("minecraft:item")?.id(&s.item));
        let off_item = self
            .offhand
            .as_ref()
            .and_then(|s| r.builtin.get("minecraft:item")?.id(&s.item));
        let swing_main = std::mem::take(&mut self.replay_swing_main);
        let swing_off = std::mem::take(&mut self.replay_swing_off);
        let self_props = self.own_textures_property();
        let Some(r) = self.remote.as_ref() else {
            return;
        };
        replay::global_recorder().sync_local_player(
            r.entity_id,
            r.profile(),
            &r.profile_name,
            self_props,
            [p.position.x, p.position.y, p.position.z],
            self.yaw,
            self.pitch,
            p.ground.on_ground,
            self.keys.contains(&KeyCode::ShiftLeft),
            p.sprinting,
            main_item,
            off_item,
            swing_main,
            swing_off,
        );
    }

    pub(super) fn handle_replay_action(&mut self, action: ReplayAction) -> Result<()> {
        match action {
            ReplayAction::TogglePlay => {
                if let Some(r) = self.replay.as_mut() {
                    if r.at_end() && !r.playing {
                        self.seek_replay_to(0)?;
                    }
                    if let Some(r) = self.replay.as_mut() {
                        r.playing = !r.playing;
                    }
                }
            }
            ReplayAction::StepSeconds(secs) => {
                let target = if let Some(r) = self.replay.as_ref() {
                    if secs < 0 {
                        r.current_ms.saturating_sub((-secs) as u64 * 1000)
                    } else {
                        r.current_ms.saturating_add(secs as u64 * 1000).min(r.duration_ms)
                    }
                } else {
                    0
                };
                self.seek_replay_to(target)?;
            }
            ReplayAction::SeekMs(ms) => {
                self.seek_replay_to(ms)?;
                if let Some(r) = self.replay.as_mut() {
                    r.path_cursor_ms = ms;
                }
            }
            ReplayAction::CycleSpeed(dir) => {
                const SPEEDS: [f32; 8] = [0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 6.0, 8.0];
                if let Some(r) = self.replay.as_mut() {
                    let cur = SPEEDS
                        .iter()
                        .enumerate()
                        .min_by(|a, b| (a.1 - r.speed).abs().total_cmp(&(b.1 - r.speed).abs()))
                        .map_or(3, |(i, _)| i);
                    let next = (cur as i32 + dir).clamp(0, SPEEDS.len() as i32 - 1) as usize;
                    r.speed = SPEEDS[next];
                    r.set_banner(format!("Speed: {}x", r.speed));
                }
            }
            ReplayAction::ToggleLoop => {
                if let Some(r) = self.replay.as_mut() {
                    r.loop_playback = !r.loop_playback;
                    r.set_banner(if r.loop_playback {
                        "Loop: ON"
                    } else {
                        "Loop: OFF"
                    });
                }
            }
            ReplayAction::AddPosKeyframe => {
                if let Some(r) = self.replay.as_mut() {
                    r.add_position_keyframe_here();
                }
            }
            ReplayAction::AddTimeKeyframe => {
                if let Some(r) = self.replay.as_mut() {
                    r.add_time_keyframe_here();
                }
            }
            ReplayAction::AddSyncedKeyframe => {
                if let Some(r) = self.replay.as_mut() {
                    r.add_synced_keyframes_here();
                }
            }
            ReplayAction::RemoveNearestKeyframe => {
                if let Some(r) = self.replay.as_mut() {
                    let t = r.path_cursor_ms;
                    let removed_p = r.timelines.active_mut().remove_nearest_position_keyframe(t, 1500);
                    let removed_t = r.timelines.active_mut().remove_nearest_time_keyframe(t, 1500);
                    if removed_p || removed_t {
                        r.dirty_edits = true;
                        r.set_banner("Removed nearest keyframe");
                    } else {
                        r.set_banner("No keyframe near cursor");
                    }
                }
            }
            ReplayAction::AddMarker => {
                if let Some(r) = self.replay.as_mut() {
                    r.add_marker_here();
                }
            }
            ReplayAction::JumpToMarker(idx) => {
                let target = self
                    .replay
                    .as_ref()
                    .and_then(|r| r.markers.get(idx).cloned());
                if let Some(m) = target {
                    self.seek_replay_to(m.time_ms)?;
                    if let Some(r) = self.replay.as_mut() {
                        r.cam_pos = [m.x, m.y, m.z];
                        r.cam_yaw = m.yaw;
                        r.cam_pitch = m.pitch;
                        r.cam_roll = m.roll;
                        r.camera_mode = CameraMode::FreeCam;
                        r.markers_drawer_open = false;
                        r.set_banner(format!("Jumped to {}", m.name));
                    }
                }
            }
            ReplayAction::TogglePathPreview => {
                if let Some(r) = self.replay.as_mut() {
                    if !r.timelines.active().can_preview_path() {
                        r.set_banner("Add at least 2 Pos Keyframes first");
                    } else if r.path_playing {
                        r.path_playing = false;
                        r.camera_mode = CameraMode::FreeCam;
                        r.set_banner("Path Preview stopped");
                    } else {
                        let start = r
                            .timelines
                            .active()
                            .position_keyframes
                            .first()
                            .map_or(0, |k| k.timeline_ms);
                        r.path_cursor_ms = start;
                        r.path_playing = true;
                        r.camera_mode = CameraMode::PathPreview;
                        r.set_banner("Playing Camera Path");
                    }
                }
            }
            ReplayAction::ToggleSyncTime => {
                if let Some(r) = self.replay.as_mut() {
                    r.sync_time_with_path = !r.sync_time_with_path;
                    r.set_banner(if r.sync_time_with_path {
                        "Sync Time: ON"
                    } else {
                        "Sync Time: OFF"
                    });
                }
            }
            ReplayAction::ToggleInterpolation => {
                if let Some(r) = self.replay.as_mut() {
                    r.toggle_interpolation();
                }
            }
            ReplayAction::ClearPath => {
                if let Some(r) = self.replay.as_mut() {
                    r.clear_path();
                }
            }
            ReplayAction::SaveEdits => {
                if let Some(r) = self.replay.as_mut() {
                    if let Err(err) = r.save_edits() {
                        r.set_banner(format!("Save error: {err}"));
                    }
                }
            }
            ReplayAction::TogglePathGizmos => {
                if let Some(r) = self.replay.as_mut() {
                    r.show_path_gizmos = !r.show_path_gizmos;
                }
            }
            ReplayAction::ToggleHideHud => {
                if let Some(r) = self.replay.as_mut() {
                    r.show_hud = !r.show_hud;
                }
            }
            ReplayAction::ToggleHideNametags => {
                if let Some(r) = self.replay.as_mut() {
                    r.hide_nametags = !r.hide_nametags;
                }
            }
            ReplayAction::ToggleHidePlayers => {
                if let Some(r) = self.replay.as_mut() {
                    r.hide_players = !r.hide_players;
                }
            }
            ReplayAction::CycleTimeOverride => {
                if let Some(r) = self.replay.as_mut() {
                    r.time_override = match r.time_override {
                        None => Some(1000),
                        Some(1000) => Some(6000),
                        Some(6000) => Some(12500),
                        Some(12500) => Some(18000),
                        _ => None,
                    };
                    let label = match r.time_override {
                        None => "Time: Recorded",
                        Some(1000) => "Time: Morning",
                        Some(6000) => "Time: Noon",
                        Some(12500) => "Time: Sunset",
                        _ => "Time: Midnight",
                    };
                    r.set_banner(label);
                }
            }
            ReplayAction::ToggleNight => {
                if let Some(r) = self.replay.as_mut() {
                    if r.time_override == Some(18000) {
                        r.time_override = None;
                        r.set_banner("Time: Recorded");
                    } else {
                        r.time_override = Some(18000);
                        r.set_banner("Time: Night");
                    }
                }
            }
            ReplayAction::OpenOptions => {
                self.open_options(false);
            }
            ReplayAction::CycleWeatherOverride => {
                if let Some(r) = self.replay.as_mut() {
                    r.weather_override = match r.weather_override {
                        None => Some(WeatherOverride::Clear),
                        Some(WeatherOverride::Clear) => Some(WeatherOverride::Rain),
                        Some(WeatherOverride::Rain) => Some(WeatherOverride::Thunder),
                        Some(WeatherOverride::Thunder) => None,
                    };
                    let label = r
                        .weather_override
                        .map_or("Weather: Recorded".to_owned(), |w| {
                            format!("Weather: {}", w.label())
                        });
                    r.set_banner(label);
                }
            }
            ReplayAction::AdjustRoll(delta) => {
                if let Some(r) = self.replay.as_mut() {
                    r.cam_roll = (r.cam_roll + delta).clamp(-180.0, 180.0);
                }
            }
            ReplayAction::ResetRoll => {
                if let Some(r) = self.replay.as_mut() {
                    r.cam_roll = 0.0;
                }
            }
            ReplayAction::AdjustFov(delta) => {
                if let Some(r) = self.replay.as_mut() {
                    r.cam_fov = (r.cam_fov + delta).clamp(25.0, 120.0);
                }
            }
            ReplayAction::AdjustFlySpeed(delta) => {
                if let Some(r) = self.replay.as_mut() {
                    r.fly_speed = (r.fly_speed + delta).clamp(2.0, 60.0);
                }
            }
            ReplayAction::TogglePlayersDrawer => {
                if let Some(r) = self.replay.as_mut() {
                    r.players_drawer_open = !r.players_drawer_open;
                    r.markers_drawer_open = false;
                }
            }
            ReplayAction::ToggleMarkersDrawer => {
                if let Some(r) = self.replay.as_mut() {
                    r.markers_drawer_open = !r.markers_drawer_open;
                    r.players_drawer_open = false;
                }
            }
            ReplayAction::SpectateEntity(entity) => {
                if let Some(r) = self.replay.as_mut() {
                    r.camera_mode = match entity {
                        Some(id) => CameraMode::Spectate(id),
                        None => CameraMode::FreeCam,
                    };
                    r.players_drawer_open = false;
                }
            }
            ReplayAction::TeleportToPos(pos) => {
                if let Some(r) = self.replay.as_mut() {
                    r.cam_pos = [pos[0], pos[1] + 1.6, pos[2]];
                    r.camera_mode = CameraMode::FreeCam;
                    r.players_drawer_open = false;
                }
            }
            ReplayAction::ToggleRenderModal => {
                if let Some(r) = self.replay.as_mut() {
                    r.render_modal_open = !r.render_modal_open;
                }
            }
            ReplayAction::StartRenderMp4 {
                width,
                height,
                fps,
                bitrate_mbps,
            } => {
                let setup = self.replay.as_mut().and_then(|r| {
                    let tl = r.timelines.active();
                    if !tl.can_preview_path() {
                        r.set_banner("Need 2+ Pos Keyframes to render MP4");
                        return None;
                    }
                    if !replay::video_export_supported() {
                        r.set_banner("MP4 export only supported on Android");
                        return None;
                    }
                    let start_ms = tl.position_keyframes.first().map_or(0, |k| k.timeline_ms);
                    let end_ms = tl.position_keyframes.last().map_or(0, |k| k.timeline_ms);
                    let span_ms = end_ms.saturating_sub(start_ms).max(100);
                    let total_frames = ((span_ms * u64::from(fps.max(1))) / 1000).max(2);
                    let stem = r
                        .mcpr_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "VoltPlus_Replay".to_owned());
                    Some((start_ms, end_ms, total_frames, stem))
                });
                if let Some((start_ms, end_ms, total_frames, stem)) = setup {
                    let rendered_dir = replay::rendered_replays_dir(&Self::game_dir());
                    let _ = std::fs::create_dir_all(&rendered_dir);
                    replay::set_rendered_output_dir(Some(rendered_dir));
                    match replay::video_export_start(width, height, fps, bitrate_mbps, &stem) {
                        Ok(_) => {
                            self.close_dialogs();
                            self.screen = Screen::Playing;
                            self.replay_render_wait_frames = 0;
                            if let Some(r) = self.replay.as_mut() {
                                r.render_modal_open = false;
                                r.playing = false;
                                r.path_playing = false;
                                r.camera_mode = CameraMode::RenderPath;
                                r.render_job = Some(ReplayRenderJob {
                                    width,
                                    height,
                                    fps,
                                    bitrate_kbps: bitrate_mbps * 1000,
                                    start_ms,
                                    end_ms,
                                    current_frame: 0,
                                    total_frames,
                                    finishing: false,
                                    awaiting_readback: false,
                                    status_text: "Starting MP4 render...".to_owned(),
                                    saved_uri: None,
                                    error: None,
                                });
                            }
                        }
                        Err(err) => {
                            if let Some(r) = self.replay.as_mut() {
                                r.set_banner(format!("MP4 export failed: {err}"));
                            }
                        }
                    }
                }
            }
            ReplayAction::CancelRenderMp4 => {
                self.replay_render_wait_frames = 0;
                if let Some(r) = self.replay.as_mut()
                    && r.render_job.is_some()
                {
                    replay::video_export_cancel();
                    r.render_job = None;
                    r.camera_mode = CameraMode::FreeCam;
                    r.set_banner("MP4 render cancelled");
                }
            }
            ReplayAction::ExitReplay => {
                self.exit_replay_to_title()?;
            }
            ReplayAction::LiveStartRecording => {
                if !self.is_replay() && self.remote.is_some() {
                    let singleplayer = self.integrated.is_some();
                    let label = self
                        .remote
                        .as_ref()
                        .map(|r| {
                            if singleplayer {
                                "Singleplayer".to_owned()
                            } else {
                                r.session.address.clone()
                            }
                        })
                        .unwrap_or_else(|| "World".to_owned());
                    let player_type_id = self
                        .remote
                        .as_ref()
                        .and_then(|r| r.builtin.get("minecraft:entity_type")?.id("minecraft:player"));
                    let _ = replay::global_recorder().start(
                        &Self::game_dir(),
                        singleplayer,
                        &label,
                        player_type_id,
                    );
                }
            }
            ReplayAction::LiveTogglePauseRecording => {
                replay::global_recorder().toggle_pause();
            }
            ReplayAction::LiveAddBookmark => {
                let pos = self
                    .player
                    .as_ref()
                    .map_or([0.0, 64.0, 0.0], |p| [p.position.x, p.position.y, p.position.z]);
                replay::global_recorder().add_marker(None, pos[0], pos[1], pos[2], self.yaw, self.pitch);
            }
            ReplayAction::LiveSaveRecording => {
                let _ = replay::global_recorder().stop_and_save(None, None);
            }
            ReplayAction::LiveDiscardRecording => {
                replay::global_recorder().discard();
            }
            ReplayAction::LiveToggleAutoSp => {
                replay::update_settings(&Self::game_dir(), |s| s.auto_record_sp = !s.auto_record_sp);
            }
            ReplayAction::LiveToggleAutoMp => {
                replay::update_settings(&Self::game_dir(), |s| s.auto_record_mp = !s.auto_record_mp);
            }
            ReplayAction::LiveToggleRecordSelf => {
                replay::update_settings(&Self::game_dir(), |s| s.record_self = !s.record_self);
            }
            ReplayAction::LiveToggleBadge => {
                replay::update_settings(&Self::game_dir(), |s| {
                    s.show_recording_badge = !s.show_recording_badge
                });
            }
        }
        self.replay_publish_hud_snapshot();
        Ok(())
    }

    /// Keyboard shortcuts when Replay Viewer mode is active (`Screen::Playing`),
    /// also blocking survival-only keys (`E`, `T`, `/`, `Q`) from reaching `remote_key`.
    pub(super) fn replay_key(&mut self, key: KeyCode) -> Result<bool> {
        if self.screen != Screen::Playing {
            return Ok(false);
        }
        let action = match key {
            KeyCode::KeyP => Some(ReplayAction::TogglePlay),
            KeyCode::ArrowLeft => Some(ReplayAction::StepSeconds(-5)),
            KeyCode::ArrowRight => Some(ReplayAction::StepSeconds(5)),
            KeyCode::BracketLeft => Some(ReplayAction::CycleSpeed(-1)),
            KeyCode::BracketRight => Some(ReplayAction::CycleSpeed(1)),
            KeyCode::KeyK => Some(ReplayAction::AddSyncedKeyframe),
            KeyCode::KeyI => Some(ReplayAction::AddPosKeyframe),
            KeyCode::KeyO => Some(ReplayAction::AddTimeKeyframe),
            KeyCode::KeyM => Some(ReplayAction::AddMarker),
            KeyCode::KeyV => Some(ReplayAction::TogglePathPreview),
            KeyCode::KeyH => Some(ReplayAction::ToggleHideHud),
            KeyCode::KeyX | KeyCode::Delete => Some(ReplayAction::RemoveNearestKeyframe),
            KeyCode::KeyZ => Some(ReplayAction::AdjustRoll(-5.0)),
            KeyCode::KeyC => Some(ReplayAction::AdjustRoll(5.0)),
            KeyCode::KeyR => Some(ReplayAction::ResetRoll),
            KeyCode::KeyE | KeyCode::KeyT | KeyCode::Slash | KeyCode::KeyQ => return Ok(true),
            _ => None,
        };
        if let Some(action) = action {
            self.handle_replay_action(action)?;
            return Ok(true);
        }
        Ok(false)
    }
}
