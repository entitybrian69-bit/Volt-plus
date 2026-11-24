//! Runtime wiring for Volt + Boost optimizations, Live Perf HUD,
//! Auto-DRS, Thermal Guard, RAM Purge, floating Replay HUD, and the
//! read-only Replay Playback Viewer.
use super::{Game, Screen};
use horizon_client::{
    clock::FramePacer,
    hud::{HudBatch, rgba},
    perf_overlay::LivePerfMetrics,
    replay::{
        RecordingState, ReplayError, ReplayFrame, ReplayHudAction, ReplayHudGesture,
        ReplayPlayback, ReplayRecorder, clear_all_replays, format_duration, storage_summary,
    },
    text::Text,
    volt_boost::{
        DrsController, MenuBackdropMode, ThermalStep, VoltPlusBoostSettings, evaluate_thermal_step,
        read_max_thermal_celsius,
    },
};
use std::time::Instant;
use winit::keyboard::KeyCode;

pub(super) struct VoltBoostRuntime {
    pub active: VoltPlusBoostSettings,
    pub drs: DrsController,
    pub pacer: FramePacer,
    pub thermal_step: ThermalStep,
    pub last_temp_c: Option<f32>,
    pub last_battery_pct: Option<u8>,
    pub last_probe_at: Option<Instant>,
    pub entity_frame_counter: u64,
    pub recorder: ReplayRecorder,
    pub hud_gesture: ReplayHudGesture,
    pub playback: Option<ReplayPlayback>,
    pub playback_error: Option<String>,
    pub hud_rect: Option<[f32; 4]>,
    pub hud_end_rect: Option<[f32; 4]>,
    pub viewer_buttons: Vec<(ViewerAction, [f32; 4])>,
    pub toast: Option<(String, Instant)>,
    pub last_replay_tick: Option<Instant>,
    pub last_storage_refresh: Option<Instant>,
}

impl Default for VoltBoostRuntime {
    fn default() -> Self {
        Self {
            active: VoltPlusBoostSettings::default(),
            drs: DrsController::default(),
            pacer: FramePacer::default(),
            thermal_step: ThermalStep::Normal,
            last_temp_c: None,
            last_battery_pct: None,
            last_probe_at: None,
            entity_frame_counter: 0,
            recorder: ReplayRecorder::default(),
            hud_gesture: ReplayHudGesture::default(),
            playback: None,
            playback_error: None,
            hud_rect: None,
            hud_end_rect: None,
            viewer_buttons: Vec::new(),
            toast: None,
            last_replay_tick: None,
            last_storage_refresh: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewerAction {
    PlayPause,
    Rewind10,
    Forward10,
    CycleSpeed,
    ToggleFreeCam,
    Exit,
}

fn read_battery_pct() -> Option<u8> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        for path in [
            "/sys/class/power_supply/battery/capacity",
            "/sys/class/power_supply/BAT0/capacity",
            "/sys/class/power_supply/BAT1/capacity",
        ] {
            if let Ok(raw) = std::fs::read_to_string(path)
                && let Ok(pct) = raw.trim().parse::<u8>()
            {
                return Some(pct.min(100));
            }
        }
    }
    None
}

fn read_process_rss_mb() -> f32 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if let Ok(raw) = std::fs::read_to_string("/proc/self/statm")
        && let Some(pages_str) = raw.split_whitespace().nth(1)
        && let Ok(pages) = pages_str.parse::<u64>()
    {
        return (pages * 4096) as f32 / (1024.0 * 1024.0);
    }
    0.0
}

impl Game<'_> {
    pub(super) fn replays_dir() -> std::path::PathBuf {
        Self::game_dir().join("replays")
    }

    pub(super) fn push_volt_toast(&mut self, msg: impl Into<String>) {
        self.volt_rt.toast = Some((msg.into(), Instant::now()));
    }

    pub(super) fn apply_volt_boost_settings(&mut self) {
        let vb = self.settings.volt_boost.clone();
        self.volt_rt.active = vb.clone();

        horizon_threads::set_big_little_boost(vb.big_little_boost);
        horizon_threads::set_worker_coalescing(vb.background_worker_coalescing);
        horizon_client::mesher::set_smart_leaf_cull(vb.smart_leaf_cull);
        horizon_client::mesher::set_coplanar_quad_merge(vb.coplanar_quad_merge);
        horizon_client::mesher::set_zero_stutter_mesh_pool(vb.zero_stutter_mesh_pool);

        let yaw_rad = self.yaw.to_radians();
        let fwd_xz = [-yaw_rad.sin(), yaw_rad.cos()];
        horizon_client::streaming::set_frustum_first_mesh(vb.frustum_first_mesh, fwd_xz);
        horizon_client::streaming::set_chunk_build_distance(vb.chunk_build_distance);

        horizon_client::gfx::set_fast_weather(vb.fast_weather);
        horizon_client::block_entity_render::set_sign_text_cull(vb.sign_text_cull);
        horizon_client::entities::set_drop_merge_radius(vb.merge_drop_stacks, 1.5);

        self.particles.max_budget = vb.particle_budget.max_particles();
        self.particles.burst_lag_shield = vb.burst_lag_shield;
        self.particles.storm_wind_boost = vb.storm_wind_boost;
        self.particles.particle_frame_gate = vb.particle_frame_gate;
        self.particles.particle_gate_distance = vb.particle_gate_distance;

        if let Some(r) = self.renderer.as_mut() {
            r.set_volt_reflex(vb.volt_reflex);
            r.smart_10hz_atlas = vb.smart_10hz_atlas;
            r.sealed_cave_occlusion = vb.sealed_cave_occlusion;
        }
        self.refresh_replay_storage_stats(false);
    }

    pub(super) fn refresh_replay_storage_stats(&mut self, force: bool) {
        let now = Instant::now();
        if !force
            && self
                .volt_rt
                .last_storage_refresh
                .is_some_and(|t| now.duration_since(t).as_secs_f32() < 2.0)
        {
            return;
        }
        self.volt_rt.last_storage_refresh = Some(now);
        let summary = storage_summary(&Self::replays_dir());
        self.settings.volt_boost.live_storage_used_bytes = summary.total_bytes;
        self.settings.volt_boost.live_storage_clip_count = summary.clip_count;
        self.volt_rt.active.live_storage_used_bytes = summary.total_bytes;
        self.volt_rt.active.live_storage_clip_count = summary.clip_count;
    }

    pub(super) fn on_world_joined_volt_boost(&mut self) {
        self.apply_volt_boost_settings();
        let vb = &self.volt_rt.active;
        if vb.replay_recording {
            let world_name = if self.level.name.is_empty() {
                "World".to_string()
            } else {
                self.level.name.clone()
            };
            let is_mp = self.remote.is_some();
            let player_name = self.options.player_name();
            let dim = self.dim.id().to_string();
            self.volt_rt.recorder.start(
                world_name,
                is_mp,
                player_name,
                dim,
                vb.replay_length,
                vb.replay_quality,
                vb.replay_capture_hud,
                vb.replay_storage_budget_mb,
            );
            self.push_volt_toast("Replay recording active");
        } else {
            self.volt_rt.recorder.state = RecordingState::Idle;
        }
    }

    pub(super) fn on_world_left_volt_boost(&mut self) {
        if self.volt_rt.recorder.state != RecordingState::Idle {
            let _ = self.volt_rt.recorder.finish_and_save(&Self::replays_dir());
        }
        self.volt_rt.playback = None;
        self.volt_rt.playback_error = None;
        self.refresh_replay_storage_stats(true);
    }

    pub(super) fn purge_unused_ram_now(&mut self) {
        let mut freed_mb = horizon_client::mesher::purge_mesh_pool() as f32 / (1024.0 * 1024.0);
        freed_mb += self.particles.trim_unused_capacity() as f32 / (1024.0 * 1024.0);
        if let Some(r) = self.renderer.as_mut() {
            freed_mb += r.purge_unused_ram();
        }
        self.settings.volt_boost.last_ram_freed_mb = freed_mb;
        self.volt_rt.active.last_ram_freed_mb = freed_mb;
        self.push_volt_toast(format!("RAM purged: {freed_mb:.1} MB freed"));
    }

    pub(super) fn clear_all_replays_now(&mut self) {
        let removed = clear_all_replays(&Self::replays_dir()).unwrap_or(0);
        self.refresh_replay_storage_stats(true);
        self.push_volt_toast(format!("Cleared {removed} replay(s)"));
    }

    pub(super) fn open_replays_page_from_game(&mut self) {
        #[cfg(target_os = "android")]
        {
            horizon_client::android_runtime::request_replays_return();
            self.push_volt_toast("Opening Replays...");
        }
        #[cfg(not(target_os = "android"))]
        {
            let dir = Self::replays_dir();
            let _ = std::fs::create_dir_all(&dir);
            super::packs::open_folder(&dir);
            self.push_volt_toast("Opened replays folder");
        }
    }

    pub(super) fn check_pending_replay_open(&mut self) {
        let Some(path) = horizon_client::android_runtime::take_pending_replay_path() else {
            return;
        };
        match ReplayPlayback::open(&path) {
            Ok(pb) => {
                self.volt_rt.playback = Some(pb);
                self.volt_rt.playback_error = None;
                self.volt_rt.last_replay_tick = Some(Instant::now());
                self.capture(false);
                self.push_volt_toast("Replay Viewer opened (Read-Only)");
            }
            Err(err) => {
                let msg = match err {
                    ReplayError::Damaged(m) => format!("Damaged replay: {m}"),
                    other => format!("Cannot open replay: {other}"),
                };
                self.volt_rt.playback_error = Some(msg.clone());
                self.push_volt_toast(msg);
            }
        }
    }

    pub(super) fn tick_volt_boost_frame(&mut self) {
        self.check_pending_replay_open();
        self.volt_rt.entity_frame_counter = self.volt_rt.entity_frame_counter.wrapping_add(1);

        let now = Instant::now();
        // Update frustum-first forward vector every frame.
        let yaw_rad = self.yaw.to_radians();
        horizon_client::streaming::set_frustum_first_mesh(
            self.volt_rt.active.frustum_first_mesh,
            [-yaw_rad.sin(), yaw_rad.cos()],
        );

        // Thermal & battery probe at 1 Hz.
        if self
            .volt_rt
            .last_probe_at
            .is_none_or(|t| now.duration_since(t).as_secs_f32() >= 1.0)
        {
            self.volt_rt.last_probe_at = Some(now);
            self.volt_rt.last_temp_c = read_max_thermal_celsius();
            self.volt_rt.last_battery_pct = read_battery_pct();
            let avg_ms = self.frame_graph.avg_frame_ms();
            let step = evaluate_thermal_step(
                self.volt_rt.active.thermal_guard,
                self.volt_rt.last_temp_c,
                avg_ms,
            );
            self.volt_rt.thermal_step = step;
            self.settings.volt_boost.live_thermal_step = step;
        }

        // Auto-DRS + Thermal scale update.
        let avg_ms = self.frame_graph.avg_frame_ms().max(1.0);
        let drs_pct = self.volt_rt.drs.update(&self.volt_rt.active, avg_ms);
        let thermal_mult = self.volt_rt.thermal_step.scale_multiplier();
        let effective_pct = ((drs_pct as f32) * thermal_mult).round().clamp(35.0, 100.0) as u32;
        self.settings.volt_boost.live_drs_scale_pct = effective_pct;
        self.volt_rt.active.live_drs_scale_pct = effective_pct;

        if self.volt_rt.active.auto_drs_scale || self.volt_rt.thermal_step != ThermalStep::Normal {
            let base_scale = (self.settings.horizon.resolution_scale as f32) / 100.0;
            let combined = (base_scale * (effective_pct as f32 / 100.0)).clamp(0.35, 2.0);
            if let Some(r) = self.renderer.as_mut() {
                r.set_world_post(
                    combined,
                    self.settings.horizon.fxaa,
                    self.settings.horizon.taa,
                    self.settings.horizon.upscaling.ratio(),
                );
            }
        }

        // Volt Reflex live saved latency estimate.
        if self.volt_rt.active.volt_reflex {
            self.settings.volt_boost.live_reflex_saved_ms = (avg_ms * 0.5).clamp(1.0, 16.0);
        } else {
            self.settings.volt_boost.live_reflex_saved_ms = 0.0;
        }

        // Replay frame sampling while playing in world.
        let now_ms = self.started.elapsed().as_millis() as u64;
        if self.title.is_none()
            && self.volt_rt.playback.is_none()
            && self.volt_rt.recorder.state == RecordingState::Recording
            && self.volt_rt.recorder.should_sample(now_ms)
            && let Some(p) = &self.player
        {
            let frame = ReplayFrame {
                timestamp_ms: now_ms,
                pos: [p.position.x, p.position.y, p.position.z],
                yaw: self.yaw,
                pitch: self.pitch,
                head_yaw: self.yaw,
                velocity: [0.0; 3],
                sneaking: self.keys.contains(&KeyCode::ShiftLeft),
                sprinting: p.sprinting,
                swimming: p.swimming,
                swinging: false,
                held_item: "minecraft:air".into(),
                entities: Vec::new(),
                block_events: Vec::new(),
                hud: None,
            };
            self.volt_rt.recorder.record_frame(now_ms, frame);
        }

        // Advance read-only Replay Viewer playback clock.
        let dt = self
            .volt_rt
            .last_replay_tick
            .map_or(0.016, |t| now.duration_since(t).as_secs_f32().clamp(0.0, 0.2));
        self.volt_rt.last_replay_tick = Some(now);
        if let Some(pb) = &mut self.volt_rt.playback {
            pb.tick(dt);
        }
    }

    /// Computes effective framerate limit honoring Volt+ Boost FPS Cap,
    /// Dynamic Menu FPS, Inactivity FPS, and Thermal Guard step-down.
    pub(super) fn volt_boost_effective_fps_cap(&self, base_limit: i32) -> i32 {
        let vb = &self.volt_rt.active;
        let mut cap = if vb.fps_cap > 0 && vb.fps_cap < 260 {
            base_limit.min(vb.fps_cap as i32)
        } else {
            base_limit
        };
        if vb.dynamic_menu_fps && (self.title.is_some() || self.screen != Screen::Playing) {
            cap = cap.min(30);
        }
        if let Some(inactivity_cap) = vb.inactivity_fps.fps_cap() {
            let idle_secs = self.last_input.elapsed().as_secs_f32();
            if !self.focused || idle_secs >= vb.inactivity_delay_secs as f32 {
                cap = cap.min(inactivity_cap as i32);
            }
        }
        if let Some(t_cap) = self.volt_rt.thermal_step.fps_cap_override() {
            cap = cap.min(t_cap as i32);
        }
        cap
    }

    pub(super) fn effective_menu_blur(&self, raw_blur: Option<f32>) -> Option<f32> {
        match self.volt_rt.active.menu_backdrop {
            MenuBackdropMode::Full => raw_blur,
            MenuBackdropMode::Blurred => raw_blur.map(|r| r.min(3.0)),
            MenuBackdropMode::Static | MenuBackdropMode::Off => None,
        }
    }

    pub(super) fn save_or_toggle_replay_from_hud(&mut self, action: ReplayHudAction) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        match action {
            ReplayHudAction::ToggleStartPause => match self.volt_rt.recorder.state {
                RecordingState::Idle => {
                    let vb = self.volt_rt.active.clone();
                    let world_name = if self.level.name.is_empty() {
                        "World".to_string()
                    } else {
                        self.level.name.clone()
                    };
                    self.volt_rt.recorder.start(
                        world_name,
                        self.remote.is_some(),
                        self.options.player_name(),
                        self.dim.id().to_string(),
                        vb.replay_length,
                        vb.replay_quality,
                        vb.replay_capture_hud,
                        vb.replay_storage_budget_mb,
                    );
                    self.push_volt_toast("Replay recording started");
                }
                RecordingState::Recording => {
                    self.volt_rt.recorder.pause(now_ms);
                    self.push_volt_toast("Replay paused");
                }
                RecordingState::Paused => {
                    self.volt_rt.recorder.resume(now_ms);
                    self.push_volt_toast("Replay resumed");
                }
            },
            ReplayHudAction::EndAndSave => {
                match self.volt_rt.recorder.finish_and_save(&Self::replays_dir()) {
                    Ok(Some(_)) => {
                        self.refresh_replay_storage_stats(true);
                        self.push_volt_toast("Replay clip saved (.vreplay)");
                    }
                    Ok(None) => self.push_volt_toast("No replay frames to save"),
                    Err(err) => self.push_volt_toast(format!("Replay save failed: {err}")),
                }
            }
            ReplayHudAction::LayoutChanged => {
                self.settings.volt_boost.replay_hud_layout =
                    self.volt_rt.active.replay_hud_layout.clone();
                self.settings.save(&Self::options_path());
            }
            ReplayHudAction::None => {}
        }
    }

    pub(super) fn handle_volt_replay_click(&mut self, x: f32, y: f32, pressed: bool) -> bool {
        if self.volt_rt.playback.is_some() {
            if !pressed {
                return true;
            }
            for (act, r) in self.volt_rt.viewer_buttons.clone() {
                if x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3] {
                    match act {
                        ViewerAction::PlayPause => {
                            if let Some(pb) = &mut self.volt_rt.playback {
                                pb.toggle_play();
                            }
                        }
                        ViewerAction::Rewind10 => {
                            if let Some(pb) = &mut self.volt_rt.playback {
                                pb.step_back();
                            }
                        }
                        ViewerAction::Forward10 => {
                            if let Some(pb) = &mut self.volt_rt.playback {
                                pb.step_forward();
                            }
                        }
                        ViewerAction::CycleSpeed => {
                            if let Some(pb) = &mut self.volt_rt.playback {
                                pb.cycle_speed(1);
                            }
                        }
                        ViewerAction::ToggleFreeCam => {
                            if let Some(pb) = &mut self.volt_rt.playback {
                                pb.toggle_free_camera();
                            }
                        }
                        ViewerAction::Exit => {
                            self.volt_rt.playback = None;
                            #[cfg(target_os = "android")]
                            horizon_client::android_runtime::request_replays_return();
                        }
                    }
                    return true;
                }
            }
            return true;
        }

        if self.screen != Screen::Playing
            || self.title.is_some()
            || !self.volt_rt.active.replay_hud_button
        {
            return false;
        }
        let now_ms = self.started.elapsed().as_millis() as u64;
        if pressed {
            if let Some(rect) = self.volt_rt.hud_rect
                && x >= rect[0]
                && x <= rect[0] + rect[2]
                && y >= rect[1]
                && y <= rect[1] + rect[3]
            {
                let layout = &self.volt_rt.active.replay_hud_layout;
                self.volt_rt.hud_gesture.press(0, (x, y), now_ms, layout);
                return true;
            }
            return false;
        } else if self.volt_rt.hud_gesture.active_pointer == Some(0) {
            let tap_end = self.volt_rt.hud_end_rect.is_some_and(|r| {
                x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3]
            });
            let action = self.volt_rt.hud_gesture.release(0, tap_end);
            self.save_or_toggle_replay_from_hud(action);
            return true;
        }
        false
    }

    pub(super) fn handle_volt_replay_drag(&mut self, x: f32, y: f32, width: f32, height: f32) -> bool {
        if self.volt_rt.hud_gesture.active_pointer != Some(0) {
            return false;
        }
        let now_ms = self.started.elapsed().as_millis() as u64;
        let mut layout = self.volt_rt.active.replay_hud_layout.clone();
        let action = self
            .volt_rt
            .hud_gesture
            .move_to(0, (x, y), now_ms, (width, height), &mut layout);
        self.volt_rt.active.replay_hud_layout = layout;
        if action == ReplayHudAction::LayoutChanged {
            self.settings.volt_boost.replay_hud_layout =
                self.volt_rt.active.replay_hud_layout.clone();
        }
        true
    }

    #[cfg(target_os = "android")]
    pub(super) fn handle_volt_replay_touch(
        &mut self,
        pointer_id: u64,
        phase: winit::event::TouchPhase,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> bool {
        use winit::event::TouchPhase;
        if self.volt_rt.playback.is_some() {
            if phase == TouchPhase::Started {
                return self.handle_volt_replay_click(x, y, true);
            }
            return true;
        }
        if self.screen != Screen::Playing
            || self.title.is_some()
            || !self.volt_rt.active.replay_hud_button
        {
            return false;
        }
        let now_ms = self.started.elapsed().as_millis() as u64;
        match phase {
            TouchPhase::Started => {
                if let Some(rect) = self.volt_rt.hud_rect
                    && x >= rect[0]
                    && x <= rect[0] + rect[2]
                    && y >= rect[1]
                    && y <= rect[1] + rect[3]
                {
                    let layout = &self.volt_rt.active.replay_hud_layout;
                    self.volt_rt
                        .hud_gesture
                        .press(pointer_id, (x, y), now_ms, layout);
                    return true;
                }
                false
            }
            TouchPhase::Moved => {
                if self.volt_rt.hud_gesture.active_pointer != Some(pointer_id) {
                    return false;
                }
                let mut layout = self.volt_rt.active.replay_hud_layout.clone();
                let action = self.volt_rt.hud_gesture.move_to(
                    pointer_id,
                    (x, y),
                    now_ms,
                    (width, height),
                    &mut layout,
                );
                self.volt_rt.active.replay_hud_layout = layout;
                if action == ReplayHudAction::LayoutChanged {
                    self.settings.volt_boost.replay_hud_layout =
                        self.volt_rt.active.replay_hud_layout.clone();
                }
                true
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.volt_rt.hud_gesture.active_pointer != Some(pointer_id) {
                    return false;
                }
                let tap_end = phase == TouchPhase::Ended
                    && self.volt_rt.hud_end_rect.is_some_and(|r| {
                        x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3]
                    });
                let action = self.volt_rt.hud_gesture.release(pointer_id, tap_end);
                if phase == TouchPhase::Ended {
                    self.save_or_toggle_replay_from_hud(action);
                }
                true
            }
        }
    }

    pub(super) fn handle_volt_replay_viewer_key(&mut self, key: KeyCode) -> bool {
        let Some(pb) = &mut self.volt_rt.playback else {
            return false;
        };
        match key {
            KeyCode::Space => pb.toggle_play(),
            KeyCode::ArrowLeft => pb.step_back(),
            KeyCode::ArrowRight => pb.step_forward(),
            KeyCode::KeyW => pb.move_free_camera(1.0, 0.0, 0.0, 0.25),
            KeyCode::KeyS => pb.move_free_camera(-1.0, 0.0, 0.0, 0.25),
            KeyCode::KeyA => pb.move_free_camera(0.0, -1.0, 0.0, 0.25),
            KeyCode::KeyD => pb.move_free_camera(0.0, 1.0, 0.0, 0.25),
            KeyCode::KeyC => pb.toggle_free_camera(),
            KeyCode::Escape => {
                self.volt_rt.playback = None;
                #[cfg(target_os = "android")]
                horizon_client::android_runtime::request_replays_return();
            }
            _ => {}
        }
        true
    }

    pub(super) fn draw_volt_overlays(&mut self, batch: &mut HudBatch) {
        let atlas = &self.gui.atlas;
        // 1. Live Perf HUD
        if self.volt_rt.active.live_perf_hud {
            let draw_calls = self.renderer.as_ref().map_or(0, |r| r.draw_calls());
            let metrics = LivePerfMetrics {
                ram_mb: read_process_rss_mb(),
                draw_calls,
                drs_scale_pct: self.volt_rt.active.live_drs_scale_pct,
                temp_c: self.volt_rt.last_temp_c,
                battery_pct: self.volt_rt.last_battery_pct,
                thermal_tier: self.volt_rt.thermal_step.label(),
                reflex_active: self.volt_rt.active.volt_reflex,
                replay_recording: self.volt_rt.recorder.state == RecordingState::Recording,
                replay_clips: self.volt_rt.active.live_storage_clip_count,
            };
            self.frame_graph.draw_live_hud(
                true,
                self.volt_rt.active.perf_hud_corner,
                &metrics,
                batch,
                atlas,
            );
        }

        // 2. Floating in-game Replay HUD button
        self.volt_rt.hud_rect = None;
        self.volt_rt.hud_end_rect = None;
        if self.screen == Screen::Playing
            && self.title.is_none()
            && self.volt_rt.playback.is_none()
            && self.volt_rt.active.replay_hud_button
        {
            let layout = &self.volt_rt.active.replay_hud_layout;
            let bw = 82.0 * layout.size;
            let bh = 22.0 * layout.size;
            let bx = (layout.norm_x * batch.width - bw * 0.5).clamp(4.0, (batch.width - bw - 4.0).max(4.0));
            let by = (layout.norm_y * batch.height - bh * 0.5).clamp(4.0, (batch.height - bh - 4.0).max(4.0));
            let alpha = (layout.opacity * 230.0).round() as u8;
            batch.fill(atlas, bx, by, bw, bh, rgba(13, 17, 24, alpha));
            batch.fill(atlas, bx, by, bw, 1.5, rgba(224, 48, 68, alpha));
            let label = match self.volt_rt.recorder.state {
                RecordingState::Recording => format!("REC {}", format_duration(self.volt_rt.recorder.elapsed_ms())),
                RecordingState::Paused => "PAUSED".into(),
                RecordingState::Idle => "REC OFF".into(),
            };
            batch.text_styled(
                atlas,
                &Text::literal(label),
                bx + 6.0,
                by + (bh - 8.0) * 0.5,
                rgba(244, 246, 250, alpha),
                true,
            );
            let ew = 26.0 * layout.size;
            let ex = bx + bw - ew - 2.0;
            let ey = by + 2.0;
            let eh = bh - 4.0;
            batch.fill(atlas, ex, ey, ew, eh, rgba(224, 48, 68, alpha));
            batch.text_styled(
                atlas,
                &Text::literal("Save".to_string()),
                ex + 3.0,
                ey + (eh - 8.0) * 0.5,
                rgba(255, 255, 255, 255),
                true,
            );
            self.volt_rt.hud_rect = Some([bx, by, bw, bh]);
            self.volt_rt.hud_end_rect = Some([ex, ey, ew, eh]);
        }

        // 3. Read-only Replay Playback Viewer HUD
        self.volt_rt.viewer_buttons.clear();
        if let Some(pb) = &self.volt_rt.playback {
            let panel_w = (batch.width - 24.0).min(380.0);
            let panel_h = 54.0;
            let px = (batch.width - panel_w) * 0.5;
            let py = batch.height - panel_h - 10.0;
            batch.fill(atlas, px, py, panel_w, panel_h, rgba(10, 14, 20, 224));
            batch.fill(atlas, px, py, panel_w, 2.0, rgba(224, 48, 68, 255));
            let title = format!(
                "REPLAY (Read-Only) · {} · {} · {}",
                pb.clip.meta.world_name,
                pb.speed_label(),
                if pb.free_camera { "Free Cam" } else { "Orbit" }
            );
            batch.text_styled(
                atlas,
                &Text::literal(title),
                px + 8.0,
                py + 6.0,
                rgba(235, 240, 248, 255),
                true,
            );
            // Scrub progress bar
            let bar_x = px + 8.0;
            let bar_y = py + 18.0;
            let bar_w = panel_w - 16.0;
            batch.fill(atlas, bar_x, bar_y, bar_w, 4.0, rgba(45, 52, 66, 255));
            batch.fill(
                atlas,
                bar_x,
                bar_y,
                bar_w * pb.fraction().clamp(0.0, 1.0),
                4.0,
                rgba(224, 48, 68, 255),
            );
            let buttons = [
                (ViewerAction::Rewind10, "-10s"),
                (
                    ViewerAction::PlayPause,
                    if pb.playing { "Pause" } else { "Play" },
                ),
                (ViewerAction::Forward10, "+10s"),
                (ViewerAction::CycleSpeed, pb.speed_label()),
                (ViewerAction::ToggleFreeCam, "Cam"),
                (ViewerAction::Exit, "Exit"),
            ];
            let btn_w = ((panel_w - 16.0) / buttons.len() as f32) - 4.0;
            for (idx, (act, label)) in buttons.into_iter().enumerate() {
                let bx = px + 8.0 + idx as f32 * (btn_w + 4.0);
                let by = py + 27.0;
                let bh = 20.0;
                batch.fill(atlas, bx, by, btn_w, bh, rgba(28, 34, 46, 240));
                let tw = atlas.font.width(label);
                batch.text_styled(
                    atlas,
                    &Text::literal(label.to_string()),
                    bx + (btn_w - tw).max(4.0) * 0.5,
                    by + 6.0,
                    rgba(244, 247, 252, 255),
                    true,
                );
                self.volt_rt.viewer_buttons.push((act, [bx, by, btn_w, bh]));
            }
        }

        // 4. Non-blocking toast banner
        if let Some((msg, at)) = &self.volt_rt.toast {
            if at.elapsed().as_secs_f32() < 3.0 {
                let tw = atlas.font.width(msg) + 16.0;
                let tx = (batch.width - tw) * 0.5;
                let ty = 12.0;
                batch.fill(atlas, tx, ty, tw, 18.0, rgba(16, 20, 28, 224));
                batch.fill(atlas, tx, ty, 3.0, 18.0, rgba(224, 48, 68, 255));
                batch.text_styled(
                    atlas,
                    &Text::literal(msg.clone()),
                    tx + 8.0,
                    ty + 5.0,
                    rgba(248, 250, 252, 255),
                    true,
                );
            } else {
                self.volt_rt.toast = None;
            }
        }
    }
}
