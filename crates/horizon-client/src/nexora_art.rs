//! Nexora visual identity, demonic boot/loading renderer, glowing crimson feather
//! emblem, and circular Booster telemetry gauges.

use crate::ui_theme::{Face, Ui, color, motion};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

/// Generates and caches a crisp 96x96 RGBA crimson glowing Nexora feather emblem.
fn feather_bitmap() -> &'static Vec<u8> {
    static FEATHER: OnceLock<Vec<u8>> = OnceLock::new();
    FEATHER.get_or_init(|| {
        const S: usize = 96;
        let mut out = vec![0u8; S * S * 4];
        for py in 0..S {
            for px in 0..S {
                // Normalized coords [-1, 1] rotated so the feather sweeps diagonally bottom-left -> top-right.
                let nx = (px as f32 + 0.5) / S as f32 * 2.0 - 1.0;
                let ny = (py as f32 + 0.5) / S as f32 * 2.0 - 1.0;
                // Rotate -38 degrees so stem runs from (-0.65, 0.70) to (0.68, -0.72)
                let cos_a = 0.788_010_8;
                let sin_a = -0.615_661_5;
                let u = nx * cos_a - ny * sin_a; // across feather width
                let v = nx * sin_a + ny * cos_a; // along feather length (-1 top tip, +1 quill base)

                // Slight organic S-curve along the spine
                let spine_u = -0.08 * (v * 2.2).sin() + 0.06 * v * v;
                let du = u - spine_u;

                // Feather envelope along v in [-0.82, 0.58]
                let t = ((v + 0.82) / 1.40).clamp(0.0, 1.0);
                let width_env = if v < -0.82 || v > 0.58 {
                    0.0
                } else {
                    (t.powf(0.55) * (1.0 - t).powf(0.75)) * 1.42
                };

                // Diagonal barb notches on upper and lower vanes
                let barb_phase = (v * 18.0 + du.abs() * 7.5).sin();
                let notch1 = ((v + 0.34).abs() < 0.045 && du > 0.06) as u8;
                let notch2 = ((v + 0.02).abs() < 0.045 && du < -0.08) as u8;
                let notch3 = ((v - 0.24).abs() < 0.038 && du > 0.05) as u8;
                let in_notch = (notch1 | notch2 | notch3) != 0 && du.abs() > width_env * 0.42;

                // Quill stem from v = -0.78 down to v = 0.86
                let stem_half = if v > 0.54 { 0.028 } else { 0.020 * (1.0 - (0.58 - v) * 0.35) };
                let stem_dist = if (-0.78..=0.86).contains(&v) {
                    du.abs() - stem_half.max(0.010)
                } else {
                    1.0
                };

                let vane_dist = if width_env > 0.0 && !in_notch {
                    du.abs() - width_env * (0.88 + 0.12 * barb_phase)
                } else {
                    1.0
                };

                let dist = vane_dist.min(stem_dist);
                let body_alpha = ((-dist) * 32.0).clamp(0.0, 1.0);

                // Soft crimson outer aura halo
                let halo_dist = if (-0.86..=0.82).contains(&v) {
                    (du.abs() - width_env.max(0.05)).max(0.0)
                } else {
                    1.0
                };
                let halo_alpha = (1.0 - (halo_dist * 3.8)).clamp(0.0, 1.0).powf(2.4) * 0.48;

                let alpha = body_alpha.max(halo_alpha).clamp(0.0, 1.0);
                if alpha <= 0.005 {
                    continue;
                }

                // Gradient from fiery bright crimson tip (#FF5964) through rich crimson (#D81B2C) to deep ruby root (#7B0B14)
                let tip_t = (1.0 - t).clamp(0.0, 1.0);
                let spine_hi = (1.0 - (du.abs() / 0.14).clamp(0.0, 1.0)) * 0.38;
                let r = (135.0 + 120.0 * tip_t + 55.0 * spine_hi).clamp(0.0, 255.0);
                let g = (14.0 + 58.0 * tip_t * tip_t + 95.0 * spine_hi * tip_t).clamp(0.0, 255.0);
                let b = (22.0 + 56.0 * tip_t * tip_t + 85.0 * spine_hi * tip_t).clamp(0.0, 255.0);

                // Blend body over crimson glow
                let body_mix = if body_alpha > 0.05 { 1.0 } else { 0.0 };
                let out_r = (r * (0.72 + 0.28 * body_mix)).clamp(0.0, 255.0) as u8;
                let out_g = (g * (0.40 + 0.60 * body_mix)).clamp(0.0, 255.0) as u8;
                let out_b = (b * (0.45 + 0.55 * body_mix)).clamp(0.0, 255.0) as u8;

                let idx = (py * S + px) * 4;
                out[idx] = out_r;
                out[idx + 1] = out_g;
                out[idx + 2] = out_b;
                out[idx + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
        out
    })
}

/// Draws the glowing crimson Nexora feather emblem at `(x, y, size, size)`.
pub fn draw_feather(ui: &mut Ui<'_>, x: f32, y: f32, size: f32, alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.01 || size <= 1.0 {
        return;
    }
    let tint = color::with_alpha(0xFFFF_FFFF, a);
    let rgba = feather_bitmap();
    ui.image_tinted("nexora_feather_logo", 96, 96, rgba, x, y, size, size, tint);
}

/// Returns a cached 64x64 anti-aliased crimson Booster gauge ring for quantized percentage (0..=100 in steps of 5).
fn gauge_ring_bitmap(step_pct: u8) -> (String, Vec<u8>) {
    static RINGS: OnceLock<Mutex<HashMap<u8, Vec<u8>>>> = OnceLock::new();
    let q = (step_pct.min(100) / 5) * 5;
    let map = RINGS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    let pixels = guard
        .entry(q)
        .or_insert_with(|| {
            const S: usize = 64;
            let mut out = vec![0u8; S * S * 4];
            let frac = f32::from(q) / 100.0;
            // Gauge spans 270 degrees: from 135 deg (bottom-left) clockwise to 45 deg (bottom-right)
            let start_deg = 135.0_f32;
            let sweep_deg = 270.0_f32;
            let active_deg = sweep_deg * frac;
            for py in 0..S {
                for px in 0..S {
                    let dx = (px as f32 + 0.5) / S as f32 * 2.0 - 1.0;
                    let dy = (py as f32 + 0.5) / S as f32 * 2.0 - 1.0;
                    let r = (dx * dx + dy * dy).sqrt();
                    // Angle clockwise from +X in [0, 360)
                    let mut ang = dy.atan2(dx).to_degrees();
                    if ang < 0.0 {
                        ang += 360.0;
                    }
                    let rel = (ang - start_deg + 360.0) % 360.0;
                    let in_arc = rel <= sweep_deg;
                    if !in_arc {
                        continue;
                    }
                    let ring_dist = (r - 0.72).abs();
                    let ring_alpha = (1.0 - ((ring_dist - 0.075) * 28.0).clamp(0.0, 1.0)).clamp(0.0, 1.0);
                    let glow_alpha = (1.0 - ((ring_dist - 0.06) * 5.2).clamp(0.0, 1.0)).powf(2.0) * 0.42;
                    let is_active = rel <= active_deg.max(4.0) && q > 0;

                    if is_active {
                        let tip_t = (rel / active_deg.max(1.0)).clamp(0.0, 1.0);
                        let alpha = ring_alpha.max(glow_alpha).clamp(0.0, 1.0);
                        if alpha <= 0.005 {
                            continue;
                        }
                        let cr = (210.0 + 45.0 * tip_t).clamp(0.0, 255.0) as u8;
                        let cg = (26.0 + 78.0 * tip_t * tip_t).clamp(0.0, 255.0) as u8;
                        let cb = (40.0 + 72.0 * tip_t * tip_t).clamp(0.0, 255.0) as u8;
                        let idx = (py * S + px) * 4;
                        out[idx] = cr;
                        out[idx + 1] = cg;
                        out[idx + 2] = cb;
                        out[idx + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
                    } else if ring_alpha > 0.01 {
                        let idx = (py * S + px) * 4;
                        out[idx] = 56;
                        out[idx + 1] = 15;
                        out[idx + 2] = 22;
                        out[idx + 3] = (ring_alpha * 195.0).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
            out
        })
        .clone();
    (format!("nexora_gauge_{q}"), pixels)
}

/// Draws a glowing crimson circular Booster telemetry gauge at center `(cx, cy)` with `radius` dp.
#[allow(clippy::too_many_arguments)]
pub fn draw_booster_gauge(
    ui: &mut Ui<'_>,
    cx: f32,
    cy: f32,
    radius: f32,
    pct: f32,
    value_text: &str,
    unit_text: &str,
    alpha: f32,
) {
    let q = (pct.clamp(0.0, 100.0).round() as u8 / 5) * 5;
    let (key, rgba) = gauge_ring_bitmap(q);
    let size = radius * 2.0;
    let inner_r = radius * 0.60;
    ui.rounded(
        cx - inner_r,
        cy - inner_r,
        inner_r * 2.0,
        inner_r * 2.0,
        inner_r,
        color::with_alpha(color::hex(0x0E0306, 225), alpha),
    );
    ui.image_tinted(
        &key,
        64,
        64,
        &rgba,
        cx - radius,
        cy - radius,
        size,
        size,
        color::with_alpha(0xFFFF_FFFF, alpha),
    );
    let val_sz = (radius * 0.42).clamp(13.0, 24.0);
    let unit_sz = (radius * 0.21).clamp(8.5, 12.5);
    ui.text_centered(
        Face::SemiBold,
        val_sz,
        cx,
        cy - val_sz * 0.85,
        color::with_alpha(color::hex(0xFFF3F5, 255), alpha),
        value_text,
    );
    ui.text_centered(
        Face::Medium,
        unit_sz,
        cx,
        cy + val_sz * 0.28,
        color::with_alpha(color::hex(0xF87171, 255), alpha),
        unit_text,
    );
}

/// Draws angular obsidian-crimson demonic corner shards for the loading screen & dark hero frames.
pub fn draw_crimson_shards(ui: &mut Ui<'_>, w: f32, h: f32, alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.01 {
        return;
    }
    let dark_shard = color::with_alpha(color::hex(0x140408, 235), a);
    let mid_shard = color::with_alpha(color::hex(0x2B070E, 210), a);
    let edge = color::with_alpha(color::hex(0xB91C28, 175), a);
    let ember = color::with_alpha(color::hex(0xFF3342, 220), a);

    // Top-left & bottom-left obsidian crimson spikes
    let sw = (w * 0.16).clamp(72.0, 180.0);
    let sh = (h * 0.28).clamp(76.0, 160.0);
    ui.polygon(
        &[[0.0, 0.0], [sw, 0.0], [sw * 0.48, sh * 0.56], [0.0, sh]],
        [sw * 0.28, sh * 0.30],
        dark_shard,
        1.2,
        edge,
    );
    ui.polygon(
        &[[0.0, h * 0.22], [sw * 0.32, h * 0.48], [0.0, h * 0.72]],
        [sw * 0.12, h * 0.48],
        mid_shard,
        1.0,
        ember,
    );
    ui.polygon(
        &[[0.0, h], [0.0, h - sh * 0.85], [sw * 0.55, h - sh * 0.36], [sw * 0.92, h]],
        [sw * 0.30, h - sh * 0.30],
        dark_shard,
        1.2,
        edge,
    );

    // Top-right & bottom-right symmetric demonic shards
    ui.polygon(
        &[[w, 0.0], [w, sh], [w - sw * 0.48, sh * 0.56], [w - sw, 0.0]],
        [w - sw * 0.28, sh * 0.30],
        dark_shard,
        1.2,
        edge,
    );
    ui.polygon(
        &[[w, h * 0.22], [w, h * 0.72], [w - sw * 0.32, h * 0.48]],
        [w - sw * 0.12, h * 0.48],
        mid_shard,
        1.0,
        ember,
    );
    ui.polygon(
        &[[w, h], [w - sw * 0.92, h], [w - sw * 0.55, h - sh * 0.36], [w, h - sh * 0.85]],
        [w - sw * 0.30, h - sh * 0.30],
        dark_shard,
        1.2,
        edge,
    );
}

/// Renders Concept 3 (`design/nexora_demonic_loading_concept.png`):
/// "SOMETHING DEMONIC IS LOADING", crimson energy progress bar, and "NEXORA CLIENT".
pub fn draw_demonic_loading(
    ui: &mut Ui<'_>,
    elapsed: f32,
    loading_progress: f32,
    status_text: &str,
    logs: &[String],
    show_logs: bool,
    alpha: f32,
) {
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.01 {
        return;
    }
    let (w, h) = (ui.w, ui.h);

    // 1. Deep obsidian-crimson backdrop + subtle pulsing central crimson aura
    ui.fill(0.0, 0.0, w, h, color::with_alpha(color::hex(0x060203, 255), a));
    let pulse = 0.85 + 0.15 * (elapsed * 2.4).sin();
    let cx = w * 0.5;
    let cy = h * 0.42;
    let glow_w = (w * 0.74).clamp(260.0, 720.0);
    let glow_h = (h * 0.48).clamp(160.0, 340.0);
    ui.rounded(
        cx - glow_w * 0.5,
        cy - glow_h * 0.5,
        glow_w,
        glow_h,
        glow_h * 0.5,
        color::with_alpha(color::hex(0x520912, 78), a * pulse),
    );
    let inner_w = glow_w * 0.62;
    let inner_h = glow_h * 0.54;
    ui.rounded(
        cx - inner_w * 0.5,
        cy - inner_h * 0.5,
        inner_w,
        inner_h,
        inner_h * 0.5,
        color::with_alpha(color::hex(0x8F101E, 64), a * pulse),
    );

    // 2. Angular obsidian & crimson demonic shards along screen edges
    draw_crimson_shards(ui, w, h, a);

    // 3. Top subtle Nexora feather crest
    let crest_s = (h * 0.13).clamp(36.0, 56.0);
    draw_feather(ui, cx - crest_s * 0.5, (cy - h * 0.25).max(18.0), crest_s, a * 0.92);

    // 4. Centerpiece Headline: "SOMETHING DEMONIC IS LOADING"
    let title_t = motion::ease_out_cubic(((elapsed - 0.08).max(0.0) / 0.36).clamp(0.0, 1.0)) * a;
    let line1_sz = (w * 0.044).clamp(20.0, 38.0);
    let line2_sz = (w * 0.048).clamp(22.0, 42.0);
    let title_y = cy - line1_sz * 1.15;

    // Crimson drop glow + metallic silver-crimson foreground
    let shadow_col = color::with_alpha(color::hex(0x8A0D19, 220), title_t);
    let silver_col = color::with_alpha(color::hex(0xF6D9DD, 255), title_t);
    let crimson_hi = color::with_alpha(color::hex(0xFF4350, 255), title_t);

    ui.text_centered(Face::SemiBold, line1_sz, cx + 1.5, title_y + 2.0, shadow_col, "SOMETHING DEMONIC");
    ui.text_centered(Face::SemiBold, line1_sz, cx, title_y, silver_col, "SOMETHING DEMONIC");

    let line2_y = title_y + line1_sz * 1.24;
    ui.text_centered(Face::SemiBold, line2_sz, cx + 1.5, line2_y + 2.0, shadow_col, "IS LOADING");
    ui.text_centered(Face::SemiBold, line2_sz, cx, line2_y, crimson_hi, "IS LOADING");

    // Ornamental crimson divider + diamond below headline
    let rule_y = line2_y + line2_sz * 1.38;
    let rule_w = (w * 0.28).clamp(130.0, 240.0);
    ui.fill(
        cx - rule_w * 0.5,
        rule_y,
        rule_w,
        1.5,
        color::with_alpha(color::hex(0x991622, 190), title_t),
    );
    ui.polygon(
        &[
            [cx, rule_y - 5.0],
            [cx + 6.0, rule_y + 0.5],
            [cx, rule_y + 6.0],
            [cx - 6.0, rule_y + 0.5],
        ],
        [cx, rule_y + 0.5],
        color::with_alpha(color::hex(0xFF4757, 255), title_t),
        0.0,
        0,
    );

    // 5. Lower bevelled obsidian-crimson energy progress bar
    let real_prog = loading_progress.clamp(0.0, 1.0);
    let bar_w = (w * 0.54).clamp(220.0, 480.0);
    let bar_h = 14.0;
    let bar_x = cx - bar_w * 0.5;
    let bar_y = (h * 0.72).max(rule_y + 28.0).min(h - 74.0);

    // Outer metallic bevel plate
    ui.rounded(
        bar_x - 3.0,
        bar_y - 3.0,
        bar_w + 6.0,
        bar_h + 6.0,
        5.0,
        color::with_alpha(color::hex(0x160508, 240), a),
    );
    ui.stroke_rounded(
        bar_x - 3.0,
        bar_y - 3.0,
        bar_w + 6.0,
        bar_h + 6.0,
        5.0,
        1.4,
        color::with_alpha(color::hex(0x781822, 220), a),
    );
    ui.rounded(
        bar_x,
        bar_y,
        bar_w,
        bar_h,
        3.0,
        color::with_alpha(color::hex(0x0B0204, 255), a),
    );

    // Crimson energy fill + leading spark
    let fill_frac = real_prog.max(0.06);
    let fill_w = (bar_w * fill_frac).clamp(bar_h, bar_w);
    ui.rounded(
        bar_x,
        bar_y,
        fill_w,
        bar_h,
        3.0,
        color::with_alpha(color::hex(0xC81624, 255), a),
    );
    ui.rounded(
        bar_x,
        bar_y + 2.0,
        fill_w,
        bar_h * 0.42,
        2.0,
        color::with_alpha(color::hex(0xFF4757, 235), a),
    );
    // Glowing spark tip when actively loading
    let tip_x = (bar_x + fill_w - 6.0).max(bar_x);
    ui.rounded(
        tip_x,
        bar_y - 1.0,
        8.0,
        bar_h + 2.0,
        4.0,
        color::with_alpha(color::hex(0xFFC2C7, 255), a * pulse),
    );

    // 6. "N E X O R A   C L I E N T" subtitle + status line
    let sub_y = bar_y + bar_h + 14.0;
    ui.text_centered(
        Face::SemiBold,
        13.5,
        cx,
        sub_y,
        color::with_alpha(color::hex(0xF5D5D9, 255), a),
        "N E X O R A   C L I E N T",
    );

    let msg = if status_text.trim().is_empty() {
        if real_prog >= 0.999 {
            "Ready • Tap anywhere to enter Nexora".to_owned()
        } else {
            format!("Awakening crimson core... {:.0}%", real_prog * 100.0)
        }
    } else {
        format!("{status_text}  •  {:.0}%", real_prog * 100.0)
    };
    let fitted = ui.fit_text(Face::Regular, 10.5, &msg, (w - 48.0).max(120.0));
    ui.text_centered(
        Face::Regular,
        10.5,
        cx,
        sub_y + 21.0,
        color::with_alpha(color::hex(0xB88289, 220), a),
        &fitted,
    );

    if show_logs && !logs.is_empty() {
        let mut ly = h - 18.0;
        for line in logs.iter().rev().take(5) {
            let trimmed = ui.fit_text(Face::Mono, 9.5, line, (w * 0.42).max(140.0));
            ui.text(
                Face::Mono,
                9.5,
                14.0,
                ly,
                color::with_alpha(color::hex(0x9C6B72, 180), a),
                &trimmed,
            );
            ly -= 13.0;
            if ly < sub_y + 36.0 {
                break;
            }
        }
    }
}
