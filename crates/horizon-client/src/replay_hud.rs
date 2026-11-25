//! Replay Mod In-Game HUD, Free-Cam Touch Controls, Timeline/Keyframe Editor,
//! Drawers, MP4 Export Modal, Live REC Badge, and Control Hub Replay Tab.

use super::{InGameMenuAction, TouchUi};
use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    replay::{
        self, EXPORT_FPS, EXPORT_RESOLUTIONS, Interpolation, ReplayAction, ReplaySettings,
        format_duration_ms,
    },
    ui_theme::{Face, Ui, active_theme},
};
use winit::event::TouchPhase;

const EXPORT_BITRATES: [u32; 4] = [10, 18, 28, 40];

fn hit(p: [f32; 2], r: [f32; 4]) -> bool {
    p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
}

struct ReplayHudLayout {
    show_hud_pill: [f32; 4],
    // Top bar
    exit_btn: [f32; 4],
    speed_down: [f32; 4],
    speed_up: [f32; 4],
    loop_btn: [f32; 4],
    players_btn: [f32; 4],
    markers_btn: [f32; 4],
    gizmo_btn: [f32; 4],
    nametag_btn: [f32; 4],
    hide_players_btn: [f32; 4],
    time_btn: [f32; 4],
    weather_btn: [f32; 4],
    save_btn: [f32; 4],
    export_btn: [f32; 4],
    hide_hud_btn: [f32; 4],
    // Left free-cam fly pad + vertical + lens chips
    fly_pad_center: [f32; 2],
    fly_pad_radius: f32,
    ascend_btn: [f32; 4],
    descend_btn: [f32; 4],
    roll_minus: [f32; 4],
    roll_reset: [f32; 4],
    roll_plus: [f32; 4],
    fov_minus: [f32; 4],
    fov_plus: [f32; 4],
    fly_minus: [f32; 4],
    fly_plus: [f32; 4],
    // Keyframe toolbar (above timeline)
    kf_pos: [f32; 4],
    kf_time: [f32; 4],
    kf_sync: [f32; 4],
    kf_del: [f32; 4],
    kf_marker: [f32; 4],
    kf_interp: [f32; 4],
    kf_preview: [f32; 4],
    kf_sync_time: [f32; 4],
    kf_clear: [f32; 4],
    // Bottom timeline bar
    play_btn: [f32; 4],
    back5_btn: [f32; 4],
    fwd5_btn: [f32; 4],
    scrubber_hit: [f32; 4],
    scrubber_track: [f32; 4],
    // Cancel render button
    cancel_render_btn: [f32; 4],
    // Drawers / Modal
    drawer_rect: [f32; 4],
    modal_rect: [f32; 4],
    modal_res: [[f32; 4]; 3],
    modal_fps: [[f32; 4]; 2],
    modal_br: [[f32; 4]; 4],
    modal_start: [f32; 4],
    modal_cancel: [f32; 4],
}

fn replay_layout(w: f32, h: f32) -> ReplayHudLayout {
    let top_y = 8.0;
    let btn_h = 24.0;
    let show_hud_pill = [w - 86.0, 8.0, 78.0, 24.0];

    let exit_btn = [8.0, top_y, 44.0, btn_h];
    let speed_down = [58.0, top_y, 20.0, btn_h];
    let speed_up = [122.0, top_y, 20.0, btn_h];
    let loop_btn = [146.0, top_y, 40.0, btn_h];

    let right_margin = 8.0;
    let hide_hud_btn = [w - right_margin - 44.0, top_y, 44.0, btn_h];
    let export_btn = [hide_hud_btn[0] - 58.0, top_y, 54.0, btn_h];
    let save_btn = [export_btn[0] - 44.0, top_y, 40.0, btn_h];
    let weather_btn = [save_btn[0] - 54.0, top_y, 50.0, btn_h];
    let time_btn = [weather_btn[0] - 50.0, top_y, 46.0, btn_h];
    let hide_players_btn = [time_btn[0] - 52.0, top_y, 48.0, btn_h];
    let nametag_btn = [hide_players_btn[0] - 46.0, top_y, 42.0, btn_h];
    let gizmo_btn = [nametag_btn[0] - 42.0, top_y, 38.0, btn_h];
    let markers_btn = [gizmo_btn[0] - 54.0, top_y, 50.0, btn_h];
    let players_btn = [markers_btn[0] - 54.0, top_y, 50.0, btn_h];

    // Bottom timeline + keyframe strip
    let bot_h = 34.0;
    let bot_y = h - bot_h - 8.0;
    let kf_h = 22.0;
    let kf_y = bot_y - kf_h - 6.0;

    let play_btn = [10.0, bot_y + 4.0, 28.0, 26.0];
    let back5_btn = [42.0, bot_y + 4.0, 26.0, 26.0];
    let fwd5_btn = [72.0, bot_y + 4.0, 26.0, 26.0];
    let scrub_x = 104.0;
    let scrub_w = (w - scrub_x - 88.0).max(80.0);
    let scrubber_hit = [scrub_x, bot_y, scrub_w, bot_h];
    let scrubber_track = [scrub_x + 6.0, bot_y + 13.0, (scrub_w - 12.0).max(40.0), 8.0];

    // 9 keyframe toolbar buttons centered above the timeline
    let kf_gap = 4.0;
    let kf_bw = ((w - 20.0 - kf_gap * 8.0) / 9.0).clamp(44.0, 82.0);
    let kf_total = kf_bw * 9.0 + kf_gap * 8.0;
    let kf_start_x = ((w - kf_total) * 0.5).max(8.0);
    let kf = |i: usize| [kf_start_x + i as f32 * (kf_bw + kf_gap), kf_y, kf_bw, kf_h];

    // Left free-cam fly pad
    let fly_pad_radius = 34.0;
    let fly_pad_center = [56.0, kf_y - 54.0];
    let ascend_btn = [102.0, kf_y - 84.0, 34.0, 26.0];
    let descend_btn = [102.0, kf_y - 52.0, 34.0, 26.0];

    let chip_y = 40.0;
    let roll_minus = [8.0, chip_y, 22.0, 20.0];
    let roll_reset = [32.0, chip_y, 44.0, 20.0];
    let roll_plus = [78.0, chip_y, 22.0, 20.0];
    let fov_minus = [8.0, chip_y + 24.0, 22.0, 20.0];
    let fov_plus = [78.0, chip_y + 24.0, 22.0, 20.0];
    let fly_minus = [8.0, chip_y + 48.0, 22.0, 20.0];
    let fly_plus = [78.0, chip_y + 48.0, 22.0, 20.0];

    let cancel_render_btn = [(w - 120.0) * 0.5, h * 0.5 + 32.0, 120.0, 30.0];

    let drawer_w = (w * 0.36).clamp(210.0, 300.0);
    let drawer_rect = [w - drawer_w - 10.0, 40.0, drawer_w, (kf_y - 48.0).max(120.0)];

    let modal_w = (w * 0.56).clamp(280.0, 400.0);
    let modal_h = 224.0;
    let mx = (w - modal_w) * 0.5;
    let my = ((h - modal_h) * 0.5).max(18.0);
    let modal_rect = [mx, my, modal_w, modal_h];
    let cw = (modal_w - 28.0 - 12.0) / 3.0;
    let modal_res = [
        [mx + 14.0, my + 54.0, cw, 28.0],
        [mx + 14.0 + cw + 6.0, my + 54.0, cw, 28.0],
        [mx + 14.0 + (cw + 6.0) * 2.0, my + 54.0, cw, 28.0],
    ];
    let fw = (modal_w - 28.0 - 6.0) * 0.5;
    let modal_fps = [
        [mx + 14.0, my + 106.0, fw, 26.0],
        [mx + 14.0 + fw + 6.0, my + 106.0, fw, 26.0],
    ];
    let bw = (modal_w - 28.0 - 18.0) / 4.0;
    let modal_br = [
        [mx + 14.0, my + 156.0, bw, 24.0],
        [mx + 14.0 + bw + 6.0, my + 156.0, bw, 24.0],
        [mx + 14.0 + (bw + 6.0) * 2.0, my + 156.0, bw, 24.0],
        [mx + 14.0 + (bw + 6.0) * 3.0, my + 156.0, bw, 24.0],
    ];
    let act_w = (modal_w - 28.0 - 8.0) * 0.5;
    let modal_cancel = [mx + 14.0, my + modal_h - 36.0, act_w, 26.0];
    let modal_start = [mx + 14.0 + act_w + 8.0, my + modal_h - 36.0, act_w, 26.0];

    ReplayHudLayout {
        show_hud_pill,
        exit_btn,
        speed_down,
        speed_up,
        loop_btn,
        players_btn,
        markers_btn,
        gizmo_btn,
        nametag_btn,
        hide_players_btn,
        time_btn,
        weather_btn,
        save_btn,
        export_btn,
        hide_hud_btn,
        fly_pad_center,
        fly_pad_radius,
        ascend_btn,
        descend_btn,
        roll_minus,
        roll_reset,
        roll_plus,
        fov_minus,
        fov_plus,
        fly_minus,
        fly_plus,
        kf_pos: kf(0),
        kf_time: kf(1),
        kf_sync: kf(2),
        kf_del: kf(3),
        kf_marker: kf(4),
        kf_interp: kf(5),
        kf_preview: kf(6),
        kf_sync_time: kf(7),
        kf_clear: kf(8),
        play_btn,
        back5_btn,
        fwd5_btn,
        scrubber_hit,
        scrubber_track,
        cancel_render_btn,
        drawer_rect,
        modal_rect,
        modal_res,
        modal_fps,
        modal_br,
        modal_start,
        modal_cancel,
    }
}

/// Draws the subtle live `REC 00:42` pill during normal SP/MP gameplay.
pub fn draw_recording_badge(batch: &mut HudBatch, atlas: &HudAtlas) {
    let settings = replay::settings();
    if !settings.show_recording_badge {
        return;
    }
    let st = replay::global_recorder().status();
    if !st.active {
        return;
    }
    let mut ui = Ui::mobile(atlas, batch, (0.0, 0.0));
    let theme = active_theme();
    let label = if st.paused {
        format!("PAUSED {}", format_duration_ms(st.elapsed_ms))
    } else {
        format!("REC {}", format_duration_ms(st.elapsed_ms))
    };
    let w = (ui.measure(Face::SemiBold, 9.0, &label) + 26.0).max(74.0);
    let x = ui.w - w - 10.0;
    let y = 10.0;
    ui.rounded(x, y, w, 20.0, 10.0, rgba(18, 20, 28, 195));
    ui.border(
        x,
        y,
        w,
        20.0,
        10.0,
        if st.paused {
            rgba(245, 158, 11, 200)
        } else {
            rgba(239, 68, 68, 220)
        },
    );
    ui.rounded(
        x + 7.0,
        y + 6.0,
        8.0,
        8.0,
        4.0,
        if st.paused {
            rgba(245, 158, 11, 255)
        } else {
            rgba(239, 68, 68, 255)
        },
    );
    ui.text(Face::SemiBold, 8.8, x + 19.0, y + 4.5, theme.text_primary, &label);
    ui.finish();
}

impl TouchUi {
    pub fn replay_fly_move(&self) -> [f32; 3] {
        let [strafe, forward] = self.replay_fly_pad.map_or([0.0, 0.0], |(_, v)| v);
        let vert = self.replay_fly_vert.map_or(0.0, |(_, v)| v);
        [strafe, forward, vert]
    }

    pub fn draw_replay_hud(&mut self, batch: &mut HudBatch, atlas: &HudAtlas) {
        let snap = replay::hud_snapshot();
        if !snap.active {
            return;
        }
        let mut ui = Ui::mobile(atlas, batch, (0.0, 0.0));
        let theme = active_theme();
        let lay = replay_layout(ui.w, ui.h);

        // If MP4 offline render is running, show dedicated progress overlay + Cancel button.
        if let Some(job) = &snap.render_job {
            let bw = (ui.w * 0.48).clamp(240.0, 360.0);
            let bh = 98.0;
            let bx = (ui.w - bw) * 0.5;
            let by = (ui.h - bh) * 0.5;
            ui.rounded(bx, by, bw, bh, 14.0, rgba(16, 18, 28, 225));
            ui.border(bx, by, bw, bh, 14.0, theme.accent);
            let pct = ((job.current_frame * 100) / job.total_frames.max(1)).min(100);
            ui.centered_fit(
                Face::SemiBold,
                11.0,
                [bx + 12.0, by + 10.0, bw - 24.0, 18.0],
                theme.text_primary,
                &format!("Rendering MP4 ({}x{} @ {}fps)", job.width, job.height, job.fps),
            );
            ui.centered_fit(
                Face::Regular,
                9.2,
                [bx + 12.0, by + 29.0, bw - 24.0, 16.0],
                theme.text_muted,
                &format!("{}  •  {}%", job.status_text, pct),
            );
            let bar_x = bx + 16.0;
            let bar_w = bw - 32.0;
            ui.rounded(bar_x, by + 50.0, bar_w, 8.0, 4.0, rgba(255, 255, 255, 28));
            ui.rounded(
                bar_x,
                by + 50.0,
                (bar_w * (pct as f32 / 100.0)).max(6.0),
                8.0,
                4.0,
                theme.accent,
            );
            ui.rounded(
                lay.cancel_render_btn[0],
                lay.cancel_render_btn[1],
                lay.cancel_render_btn[2],
                lay.cancel_render_btn[3],
                8.0,
                rgba(220, 38, 38, 220),
            );
            ui.centered_fit(
                Face::SemiBold,
                9.6,
                lay.cancel_render_btn,
                theme.text_primary,
                "Cancel Export",
            );
            ui.finish();
            return;
        }

        if !snap.show_hud {
            ui.rounded(
                lay.show_hud_pill[0],
                lay.show_hud_pill[1],
                lay.show_hud_pill[2],
                lay.show_hud_pill[3],
                12.0,
                rgba(16, 18, 28, 175),
            );
            ui.border(
                lay.show_hud_pill[0],
                lay.show_hud_pill[1],
                lay.show_hud_pill[2],
                lay.show_hud_pill[3],
                12.0,
                theme.accent,
            );
            ui.centered_fit(
                Face::SemiBold,
                9.0,
                lay.show_hud_pill,
                theme.text_primary,
                "Show HUD",
            );
            ui.finish();
            return;
        }

        let pill = |ui: &mut Ui<'_>, r: [f32; 4], active: bool, accent_btn: bool, label: &str| {
            let bg = if accent_btn {
                theme.accent
            } else if active {
                rgba(139, 92, 246, 75)
            } else {
                rgba(16, 18, 28, 195)
            };
            let br = if accent_btn || active {
                theme.accent
            } else {
                rgba(255, 255, 255, 42)
            };
            let fg = if accent_btn {
                theme.on_accent
            } else if active {
                theme.accent_text
            } else {
                theme.text_primary
            };
            ui.rounded(r[0], r[1], r[2], r[3], 7.0, bg);
            ui.border(r[0], r[1], r[2], r[3], 7.0, br);
            ui.centered_fit(Face::SemiBold, 8.6, r, fg, label);
        };

        // Top Bar
        pill(&mut ui, lay.exit_btn, false, false, "Exit");
        pill(&mut ui, lay.speed_down, false, false, "-");
        let spd_rect = [
            lay.speed_down[0] + lay.speed_down[2] + 2.0,
            lay.speed_down[1],
            (lay.speed_up[0] - (lay.speed_down[0] + lay.speed_down[2]) - 4.0).max(30.0),
            lay.speed_down[3],
        ];
        ui.rounded(spd_rect[0], spd_rect[1], spd_rect[2], spd_rect[3], 7.0, rgba(16, 18, 28, 195));
        ui.centered_fit(
            Face::SemiBold,
            8.6,
            spd_rect,
            theme.accent_text,
            &format!("{}x", snap.speed),
        );
        pill(&mut ui, lay.speed_up, false, false, "+");
        pill(&mut ui, lay.loop_btn, snap.loop_playback, false, "Loop");

        if lay.players_btn[0] > lay.loop_btn[0] + lay.loop_btn[2] + 4.0 {
            pill(
                &mut ui,
                lay.players_btn,
                snap.players_drawer_open || snap.spectating_entity.is_some(),
                false,
                &format!("Ply ({})", snap.players.len()),
            );
            pill(
                &mut ui,
                lay.markers_btn,
                snap.markers_drawer_open,
                false,
                &format!("Mrk ({})", snap.markers.len()),
            );
            pill(&mut ui, lay.gizmo_btn, snap.show_path_gizmos, false, "Path");
            pill(&mut ui, lay.nametag_btn, !snap.hide_nametags, false, "Tags");
            pill(&mut ui, lay.hide_players_btn, snap.hide_players, false, "HideP");
            pill(&mut ui, lay.time_btn, snap.time_override.is_some(), false, "Time");
            pill(&mut ui, lay.weather_btn, snap.weather_override.is_some(), false, "Wthr");
        }
        pill(&mut ui, lay.save_btn, false, false, "Save");
        pill(&mut ui, lay.export_btn, snap.render_modal_open, true, "Export");
        pill(&mut ui, lay.hide_hud_btn, false, false, "Hide");

        // Left Free-Cam Roll / FOV / Speed chips
        pill(&mut ui, lay.roll_minus, false, false, "R-");
        pill(
            &mut ui,
            lay.roll_reset,
            snap.cam_roll.abs() > 0.1,
            false,
            &format!("{:.0}°", snap.cam_roll),
        );
        pill(&mut ui, lay.roll_plus, false, false, "R+");
        pill(&mut ui, lay.fov_minus, false, false, "F-");
        let fov_r = [32.0, lay.fov_minus[1], 44.0, 20.0];
        ui.rounded(fov_r[0], fov_r[1], fov_r[2], fov_r[3], 6.0, rgba(16, 18, 28, 180));
        ui.centered_fit(Face::Medium, 8.2, fov_r, theme.text_primary, &format!("FOV {:.0}", snap.cam_fov));
        pill(&mut ui, lay.fov_plus, false, false, "F+");
        pill(&mut ui, lay.fly_minus, false, false, "V-");
        let fly_r = [32.0, lay.fly_minus[1], 44.0, 20.0];
        ui.rounded(fly_r[0], fly_r[1], fly_r[2], fly_r[3], 6.0, rgba(16, 18, 28, 180));
        ui.centered_fit(Face::Medium, 8.2, fly_r, theme.text_primary, &format!("Spd {:.0}", snap.fly_speed));
        pill(&mut ui, lay.fly_plus, false, false, "V+");

        // Virtual Free-Cam Fly Thumbstick + Ascend/Descend
        let [cx, cy] = lay.fly_pad_center;
        let rad = lay.fly_pad_radius;
        ui.rounded(cx - rad, cy - rad, rad * 2.0, rad * 2.0, rad, rgba(16, 18, 28, 125));
        ui.border(cx - rad, cy - rad, rad * 2.0, rad * 2.0, rad, rgba(255, 255, 255, 48));
        let [sx, sy] = self.replay_fly_pad.map_or([0.0, 0.0], |(_, v)| v);
        let kx = cx + sx * (rad * 0.65);
        let ky = cy - sy * (rad * 0.65);
        ui.rounded(kx - 13.0, ky - 13.0, 26.0, 26.0, 13.0, rgba(139, 92, 246, 190));
        ui.border(kx - 13.0, ky - 13.0, 26.0, 26.0, 13.0, theme.text_primary);

        let vert = self.replay_fly_vert.map_or(0.0, |(_, v)| v);
        pill(&mut ui, lay.ascend_btn, vert > 0.5, false, "Up");
        pill(&mut ui, lay.descend_btn, vert < -0.5, false, "Dn");

        // Keyframe Toolbar
        pill(&mut ui, lay.kf_pos, false, false, "+Pos KF");
        pill(&mut ui, lay.kf_time, false, false, "+Time KF");
        pill(&mut ui, lay.kf_sync, false, false, "+Sync KF");
        pill(&mut ui, lay.kf_del, false, false, "Del KF");
        pill(&mut ui, lay.kf_marker, false, false, "+Marker");
        pill(
            &mut ui,
            lay.kf_interp,
            snap.interpolation == Interpolation::Linear,
            false,
            if snap.interpolation == Interpolation::CatmullRom {
                "Spline"
            } else {
                "Linear"
            },
        );
        pill(
            &mut ui,
            lay.kf_preview,
            snap.path_playing,
            snap.path_playing,
            if snap.path_playing { "Stop Path" } else { "Play Path" },
        );
        pill(&mut ui, lay.kf_sync_time, snap.sync_time_with_path, false, "SyncT");
        pill(&mut ui, lay.kf_clear, false, false, "Clear");

        // Bottom Timeline Bar
        ui.rounded(8.0, lay.scrubber_hit[1], ui.w - 16.0, 34.0, 10.0, rgba(16, 18, 28, 210));
        ui.border(8.0, lay.scrubber_hit[1], ui.w - 16.0, 34.0, 10.0, rgba(255, 255, 255, 36));
        pill(
            &mut ui,
            lay.play_btn,
            snap.playing,
            true,
            if snap.playing { "||" } else { ">" },
        );
        pill(&mut ui, lay.back5_btn, false, false, "-5s");
        pill(&mut ui, lay.fwd5_btn, false, false, "+5s");

        let tr = lay.scrubber_track;
        ui.rounded(tr[0], tr[1], tr[2], tr[3], 4.0, rgba(255, 255, 255, 32));
        let dur = snap.duration_ms.max(1) as f32;
        let prog = (snap.current_ms as f32 / dur).clamp(0.0, 1.0);
        ui.rounded(tr[0], tr[1], (tr[2] * prog).max(4.0), tr[3], 4.0, theme.accent);

        // Draw Markers, Time Keyframes, and Position Keyframes on the track
        for m in &snap.markers {
            let f = (m.time_ms as f32 / dur).clamp(0.0, 1.0);
            let mx = tr[0] + tr[2] * f;
            ui.rounded(mx - 1.5, tr[1] - 4.0, 3.0, tr[3] + 8.0, 1.5, rgba(16, 185, 129, 240));
        }
        for tk in &snap.time_keyframes {
            let f = (tk.timeline_ms as f32 / snap.path_duration_ms.max(1) as f32).clamp(0.0, 1.0);
            let tx = tr[0] + tr[2] * f;
            ui.rounded(tx - 3.5, tr[1] - 5.0, 7.0, 7.0, 2.0, rgba(245, 158, 11, 245));
        }
        for pk in &snap.position_keyframes {
            let f = (pk.timeline_ms as f32 / snap.path_duration_ms.max(1) as f32).clamp(0.0, 1.0);
            let px = tr[0] + tr[2] * f;
            ui.rounded(px - 4.0, tr[1] + tr[3] - 2.0, 8.0, 8.0, 2.0, rgba(56, 189, 248, 250));
        }

        // Playhead thumb
        let head_x = tr[0] + tr[2] * prog;
        ui.rounded(head_x - 5.0, tr[1] - 3.0, 10.0, tr[3] + 6.0, 5.0, theme.text_primary);

        let time_txt = format!(
            "{}/{}",
            format_duration_ms(snap.current_ms),
            format_duration_ms(snap.duration_ms)
        );
        ui.centered_fit(
            Face::Medium,
            8.4,
            [tr[0] + tr[2] + 6.0, lay.scrubber_hit[1], 74.0, 34.0],
            theme.text_primary,
            &time_txt,
        );

        // Players or Markers Drawer
        if snap.players_drawer_open || snap.markers_drawer_open {
            let dr = lay.drawer_rect;
            ui.rounded(dr[0], dr[1], dr[2], dr[3], 12.0, rgba(16, 18, 28, 232));
            ui.border(dr[0], dr[1], dr[2], dr[3], 12.0, theme.accent);
            if snap.players_drawer_open {
                ui.text(Face::SemiBold, 10.0, dr[0] + 12.0, dr[1] + 10.0, theme.text_primary, "Players in Replay");
                let free_r = [dr[0] + 10.0, dr[1] + 30.0, dr[2] - 20.0, 24.0];
                pill(&mut ui, free_r, snap.spectating_entity.is_none(), false, "Return to Free-Cam");
                for (i, pr) in snap.players.iter().take(6).enumerate() {
                    let ry = dr[1] + 60.0 + i as f32 * 32.0;
                    if ry + 28.0 > dr[1] + dr[3] {
                        break;
                    }
                    ui.rounded(dr[0] + 10.0, ry, dr[2] - 20.0, 28.0, 8.0, rgba(255, 255, 255, 16));
                    let lbl = if pr.is_self {
                        format!("{} (You)", pr.name)
                    } else {
                        pr.name.clone()
                    };
                    ui.text(Face::Medium, 8.8, dr[0] + 16.0, ry + 8.0, theme.text_primary, &lbl);
                    let spec_r = [dr[0] + dr[2] - 88.0, ry + 3.0, 42.0, 22.0];
                    let tp_r = [dr[0] + dr[2] - 42.0, ry + 3.0, 30.0, 22.0];
                    pill(
                        &mut ui,
                        spec_r,
                        snap.spectating_entity == Some(pr.entity_id),
                        false,
                        "Spec",
                    );
                    pill(&mut ui, tp_r, false, false, "TP");
                }
            } else {
                ui.text(Face::SemiBold, 10.0, dr[0] + 12.0, dr[1] + 10.0, theme.text_primary, "Bookmarks / Markers");
                for (i, m) in snap.markers.iter().take(7).enumerate() {
                    let ry = dr[1] + 32.0 + i as f32 * 30.0;
                    if ry + 26.0 > dr[1] + dr[3] {
                        break;
                    }
                    ui.rounded(dr[0] + 10.0, ry, dr[2] - 20.0, 26.0, 8.0, rgba(255, 255, 255, 16));
                    ui.text(
                        Face::Medium,
                        8.8,
                        dr[0] + 16.0,
                        ry + 7.0,
                        theme.text_primary,
                        &format!("{}  ({})", m.name, format_duration_ms(m.time_ms)),
                    );
                }
            }
        }

        // Export MP4 Modal
        if snap.render_modal_open {
            let mr = lay.modal_rect;
            ui.rounded(mr[0], mr[1], mr[2], mr[3], 14.0, rgba(16, 18, 28, 242));
            ui.border(mr[0], mr[1], mr[2], mr[3], 14.0, theme.accent);
            ui.text(Face::SemiBold, 11.2, mr[0] + 14.0, mr[1] + 12.0, theme.text_primary, "Export Camera Path to MP4");
            ui.text(Face::Regular, 8.4, mr[0] + 14.0, mr[1] + 29.0, theme.text_muted, "Deterministic offline H.264 render saved to Movies/VoltPlus Replays");
            ui.text(Face::SemiBold, 8.2, mr[0] + 14.0, mr[1] + 42.0, theme.text_muted, "RESOLUTION");
            for (i, (lbl, _, _)) in EXPORT_RESOLUTIONS.iter().enumerate() {
                pill(&mut ui, lay.modal_res[i], self.replay_export_res_idx == i, false, lbl);
            }
            ui.text(Face::SemiBold, 8.2, mr[0] + 14.0, mr[1] + 94.0, theme.text_muted, "FRAME RATE");
            for (i, fps) in EXPORT_FPS.iter().enumerate() {
                pill(&mut ui, lay.modal_fps[i], self.replay_export_fps_idx == i, false, &format!("{fps} fps"));
            }
            ui.text(Face::SemiBold, 8.2, mr[0] + 14.0, mr[1] + 144.0, theme.text_muted, "BITRATE");
            for (i, br) in EXPORT_BITRATES.iter().enumerate() {
                pill(&mut ui, lay.modal_br[i], self.replay_export_bitrate_idx == i, false, &format!("{br} Mbps"));
            }
            pill(&mut ui, lay.modal_cancel, false, false, "Cancel");
            pill(&mut ui, lay.modal_start, true, true, "Start Render MP4");
        }

        // Transient Banner Toast
        if let Some(banner) = &snap.banner {
            let bw = (ui.measure(Face::SemiBold, 9.2, banner) + 26.0).clamp(140.0, ui.w - 40.0);
            let bx = (ui.w - bw) * 0.5;
            let by = 38.0;
            ui.rounded(bx, by, bw, 22.0, 11.0, rgba(16, 18, 28, 220));
            ui.border(bx, by, bw, 22.0, 11.0, theme.accent);
            ui.centered_fit(Face::SemiBold, 8.8, [bx, by, bw, 22.0], theme.accent_text, banner);
        }

        ui.finish();
    }

    /// Returns:
    /// - `None`: touch is on the open 3D viewport -> let `pad.touch` handle Free-Cam look drag.
    /// - `Some(None)`: touch consumed by Replay HUD / Fly Pad without emitting a discrete action.
    /// - `Some(Some(action))`: touch triggered `ReplayAction`.
    pub fn replay_touch(
        &mut self,
        w: f32,
        h: f32,
        id: u64,
        phase: TouchPhase,
        p: [f32; 2],
    ) -> Option<Option<ReplayAction>> {
        let snap = replay::hud_snapshot();
        if !snap.active {
            return None;
        }
        let lay = replay_layout(w, h);

        // Handle active timeline scrubber drag
        if self.replay_timeline_drag == Some(id) {
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.replay_timeline_drag = None;
            }
            let frac = ((p[0] - lay.scrubber_track[0]) / lay.scrubber_track[2]).clamp(0.0, 1.0);
            let target = (frac as f64 * snap.duration_ms as f64).round() as u64;
            return Some(Some(ReplayAction::SeekMs(target)));
        }

        // Handle active virtual fly pad drag
        if self.replay_fly_pad.is_some_and(|(fid, _)| fid == id) {
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.replay_fly_pad = None;
            } else {
                let dx = p[0] - lay.fly_pad_center[0];
                let dy = lay.fly_pad_center[1] - p[1];
                let r = lay.fly_pad_radius;
                let len = (dx * dx + dy * dy).sqrt().max(r);
                self.replay_fly_pad = Some((id, [(dx / len).clamp(-1.0, 1.0), (dy / len).clamp(-1.0, 1.0)]));
            }
            return Some(None);
        }

        // Handle active ascend/descend hold
        if self.replay_fly_vert.is_some_and(|(fid, _)| fid == id) {
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.replay_fly_vert = None;
            }
            return Some(None);
        }

        // Active MP4 export overlay
        if snap.render_job.is_some() {
            if phase == TouchPhase::Started && hit(p, lay.cancel_render_btn) {
                return Some(Some(ReplayAction::CancelRenderMp4));
            }
            return Some(None);
        }

        if !snap.show_hud {
            if phase == TouchPhase::Started && hit(p, lay.show_hud_pill) {
                return Some(Some(ReplayAction::ToggleHideHud));
            }
            return None;
        }

        if phase != TouchPhase::Started {
            return None;
        }

        // Modal has priority
        if snap.render_modal_open {
            for (i, r) in lay.modal_res.iter().enumerate() {
                if hit(p, *r) {
                    self.replay_export_res_idx = i;
                    return Some(None);
                }
            }
            for (i, r) in lay.modal_fps.iter().enumerate() {
                if hit(p, *r) {
                    self.replay_export_fps_idx = i;
                    return Some(None);
                }
            }
            for (i, r) in lay.modal_br.iter().enumerate() {
                if hit(p, *r) {
                    self.replay_export_bitrate_idx = i;
                    return Some(None);
                }
            }
            if hit(p, lay.modal_cancel) {
                return Some(Some(ReplayAction::ToggleRenderModal));
            }
            if hit(p, lay.modal_start) {
                let (_, rw, rh) = EXPORT_RESOLUTIONS[self.replay_export_res_idx.min(2)];
                let fps = EXPORT_FPS[self.replay_export_fps_idx.min(1)];
                let br = EXPORT_BITRATES[self.replay_export_bitrate_idx.min(3)];
                return Some(Some(ReplayAction::StartRenderMp4 {
                    width: rw,
                    height: rh,
                    fps,
                    bitrate_mbps: br,
                }));
            }
            if hit(p, lay.modal_rect) {
                return Some(None);
            }
        }

        // Drawers
        if (snap.players_drawer_open || snap.markers_drawer_open) && hit(p, lay.drawer_rect) {
            let dr = lay.drawer_rect;
            if snap.players_drawer_open {
                let free_r = [dr[0] + 10.0, dr[1] + 30.0, dr[2] - 20.0, 24.0];
                if hit(p, free_r) {
                    return Some(Some(ReplayAction::SpectateEntity(None)));
                }
                for (i, pr) in snap.players.iter().take(6).enumerate() {
                    let ry = dr[1] + 60.0 + i as f32 * 32.0;
                    let spec_r = [dr[0] + dr[2] - 88.0, ry + 3.0, 42.0, 22.0];
                    let tp_r = [dr[0] + dr[2] - 42.0, ry + 3.0, 30.0, 22.0];
                    if hit(p, spec_r) {
                        return Some(Some(ReplayAction::SpectateEntity(Some(pr.entity_id))));
                    }
                    if hit(p, tp_r) {
                        return Some(Some(ReplayAction::TeleportToPos(pr.pos)));
                    }
                }
            } else {
                for (i, _) in snap.markers.iter().take(7).enumerate() {
                    let ry = dr[1] + 32.0 + i as f32 * 30.0;
                    if hit(p, [dr[0] + 10.0, ry, dr[2] - 20.0, 26.0]) {
                        return Some(Some(ReplayAction::JumpToMarker(i)));
                    }
                }
            }
            return Some(None);
        }

        // Virtual Fly Pad start
        let dx = p[0] - lay.fly_pad_center[0];
        let dy = lay.fly_pad_center[1] - p[1];
        if dx * dx + dy * dy <= (lay.fly_pad_radius + 8.0).powi(2) {
            let r = lay.fly_pad_radius;
            let len = (dx * dx + dy * dy).sqrt().max(r);
            self.replay_fly_pad = Some((id, [(dx / len).clamp(-1.0, 1.0), (dy / len).clamp(-1.0, 1.0)]));
            return Some(None);
        }
        if hit(p, lay.ascend_btn) {
            self.replay_fly_vert = Some((id, 1.0));
            return Some(None);
        }
        if hit(p, lay.descend_btn) {
            self.replay_fly_vert = Some((id, -1.0));
            return Some(None);
        }

        // Timeline scrubber start
        if hit(p, lay.scrubber_hit) {
            self.replay_timeline_drag = Some(id);
            let frac = ((p[0] - lay.scrubber_track[0]) / lay.scrubber_track[2]).clamp(0.0, 1.0);
            let target = (frac as f64 * snap.duration_ms as f64).round() as u64;
            return Some(Some(ReplayAction::SeekMs(target)));
        }

        // Discrete buttons
        let checks: &[([f32; 4], ReplayAction)] = &[
            (lay.exit_btn, ReplayAction::ExitReplay),
            (lay.speed_down, ReplayAction::CycleSpeed(-1)),
            (lay.speed_up, ReplayAction::CycleSpeed(1)),
            (lay.loop_btn, ReplayAction::ToggleLoop),
            (lay.players_btn, ReplayAction::TogglePlayersDrawer),
            (lay.markers_btn, ReplayAction::ToggleMarkersDrawer),
            (lay.gizmo_btn, ReplayAction::TogglePathGizmos),
            (lay.nametag_btn, ReplayAction::ToggleHideNametags),
            (lay.hide_players_btn, ReplayAction::ToggleHidePlayers),
            (lay.time_btn, ReplayAction::CycleTimeOverride),
            (lay.weather_btn, ReplayAction::CycleWeatherOverride),
            (lay.save_btn, ReplayAction::SaveEdits),
            (lay.export_btn, ReplayAction::ToggleRenderModal),
            (lay.hide_hud_btn, ReplayAction::ToggleHideHud),
            (lay.roll_minus, ReplayAction::AdjustRoll(-5.0)),
            (lay.roll_reset, ReplayAction::ResetRoll),
            (lay.roll_plus, ReplayAction::AdjustRoll(5.0)),
            (lay.fov_minus, ReplayAction::AdjustFov(-5.0)),
            (lay.fov_plus, ReplayAction::AdjustFov(5.0)),
            (lay.fly_minus, ReplayAction::AdjustFlySpeed(-3.0)),
            (lay.fly_plus, ReplayAction::AdjustFlySpeed(3.0)),
            (lay.kf_pos, ReplayAction::AddPosKeyframe),
            (lay.kf_time, ReplayAction::AddTimeKeyframe),
            (lay.kf_sync, ReplayAction::AddSyncedKeyframe),
            (lay.kf_del, ReplayAction::RemoveNearestKeyframe),
            (lay.kf_marker, ReplayAction::AddMarker),
            (lay.kf_interp, ReplayAction::ToggleInterpolation),
            (lay.kf_preview, ReplayAction::TogglePathPreview),
            (lay.kf_sync_time, ReplayAction::ToggleSyncTime),
            (lay.kf_clear, ReplayAction::ClearPath),
            (lay.play_btn, ReplayAction::TogglePlay),
            (lay.back5_btn, ReplayAction::StepSeconds(-5)),
            (lay.fwd5_btn, ReplayAction::StepSeconds(5)),
        ];
        for (rect, act) in checks {
            if hit(p, *rect) {
                return Some(Some(act.clone()));
            }
        }
        None
    }
}

/// Control Hub Tab 6 ("Replay Mod") — separate from Tab 5 ("Record" Screen Recorder).
pub(super) fn draw_control_hub_replay_tab(
    ui: &mut Ui<'_>,
    cx: f32,
    cw: f32,
    body_y: f32,
) {
    let theme = active_theme();
    let st = replay::global_recorder().status();
    let rs: ReplaySettings = replay::settings();

    // Status Card
    let card_h = 62.0;
    ui.rounded(cx, body_y, cw, card_h, 11.0, theme.surface);
    ui.border(cx, body_y, cw, card_h, 11.0, theme.border);
    let title = if st.active {
        if st.paused {
            format!("Replay Paused  •  {}", format_duration_ms(st.elapsed_ms))
        } else {
            format!("Recording Replay  •  {}", format_duration_ms(st.elapsed_ms))
        }
    } else {
        "Replay Mod Ready (.mcpr)".to_owned()
    };
    let sub = if st.active {
        format!("{}  •  {} pkts  •  {:.1} MB", st.label, st.packet_count, st.bytes_written as f64 / 1_048_576.0)
    } else {
        st.last_saved_message
            .clone()
            .unwrap_or_else(|| "Captures SP & MP packets for free-cam & camera path editing".to_owned())
    };
    ui.text(Face::SemiBold, 10.6, cx + 12.0, body_y + 10.0, theme.text_primary, &title);
    ui.text(Face::Regular, 8.6, cx + 12.0, body_y + 29.0, theme.text_muted, &sub);

    // Action Buttons
    let btn_y = body_y + 72.0;
    let bw = (cw - 16.0) / 3.0;
    let draw_btn = |ui: &mut Ui<'_>, r: [f32; 4], fill: u32, fg: u32, lbl: &str| {
        ui.rounded(r[0], r[1], r[2], r[3], 9.0, fill);
        ui.border(r[0], r[1], r[2], r[3], 9.0, theme.border);
        ui.centered_fit(Face::SemiBold, 9.6, r, fg, lbl);
    };
    if st.active {
        draw_btn(
            ui,
            [cx, btn_y, bw, 32.0],
            theme.accent,
            theme.on_accent,
            "Save & Stop",
        );
        draw_btn(
            ui,
            [cx + bw + 8.0, btn_y, bw, 32.0],
            theme.surface,
            theme.text_primary,
            if st.paused { "Resume" } else { "Pause" },
        );
        draw_btn(
            ui,
            [cx + (bw + 8.0) * 2.0, btn_y, bw, 32.0],
            theme.surface,
            theme.text_primary,
            "+ Bookmark",
        );
    } else {
        draw_btn(
            ui,
            [cx, btn_y, cw, 32.0],
            theme.accent,
            theme.on_accent,
            "Start Recording Session",
        );
    }

    // Settings Rows
    let rows = [
        ("Auto-Record Singleplayer", "Automatically start .mcpr recording in local worlds", rs.auto_record_sp),
        ("Auto-Record Multiplayer", "Automatically start .mcpr recording on servers", rs.auto_record_mp),
        ("Record Local Player (Self)", "Synthesize your own player model so you appear in replays", rs.record_self),
        ("Show Live REC Badge", "Display subtle REC timer pill while recording", rs.show_recording_badge),
    ];
    for (i, (t, s, on)) in rows.iter().enumerate() {
        let ry = body_y + 116.0 + i as f32 * 52.0;
        ui.rounded(cx, ry, cw, 46.0, 11.0, theme.surface);
        ui.border(cx, ry, cw, 46.0, 11.0, theme.border);
        ui.text(Face::SemiBold, 10.4, cx + 14.0, ry + 7.0, theme.text_primary, t);
        ui.text(Face::Regular, 8.6, cx + 14.0, ry + 24.0, theme.text_muted, s);
        ui.m3_switch(cx + cw - 54.0, ry + 12.0, *on);
    }
}

pub(super) fn control_hub_replay_tab_touch(
    cx: f32,
    cw: f32,
    body_y: f32,
    p: [f32; 2],
) -> Option<InGameMenuAction> {
    let st = replay::global_recorder().status();
    let btn_y = body_y + 72.0;
    let bw = (cw - 16.0) / 3.0;
    if st.active {
        if hit(p, [cx, btn_y, bw, 32.0]) {
            return Some(InGameMenuAction::Replay(ReplayAction::LiveSaveRecording));
        }
        if hit(p, [cx + bw + 8.0, btn_y, bw, 32.0]) {
            return Some(InGameMenuAction::Replay(ReplayAction::LiveTogglePauseRecording));
        }
        if hit(p, [cx + (bw + 8.0) * 2.0, btn_y, bw, 32.0]) {
            return Some(InGameMenuAction::Replay(ReplayAction::LiveAddBookmark));
        }
    } else if hit(p, [cx, btn_y, cw, 32.0]) {
        return Some(InGameMenuAction::Replay(ReplayAction::LiveStartRecording));
    }

    let acts = [
        ReplayAction::LiveToggleAutoSp,
        ReplayAction::LiveToggleAutoMp,
        ReplayAction::LiveToggleRecordSelf,
        ReplayAction::LiveToggleBadge,
    ];
    for (i, act) in acts.into_iter().enumerate() {
        let ry = body_y + 116.0 + i as f32 * 52.0;
        if hit(p, [cx, ry, cw, 46.0]) {
            return Some(InGameMenuAction::Replay(act));
        }
    }
    None
}
