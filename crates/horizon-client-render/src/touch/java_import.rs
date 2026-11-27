//! Universal converter from PojavLauncher / Amethyst / Mojo (`mControlDataList`),
//! Zalith Launcher (`layers` / `editorVersion`), and Fold Craft Launcher (`viewGroups`)
//! JSON control layouts into active Horizon `TouchControls` (`Settings` with `ControlStyle::GoyControl`).
use std::collections::HashMap;

use horizon_client_input::java_controls::{
    ControlEvent, android_keycode, pojav_code, pojav_code_android, zalith_event,
};
use serde_json::Value;

use super::custom::CustomButton;
use super::layout::{Control, ControlStyle, Mode, Placement, Settings};

const REF_W: f32 = 960.0;
const REF_H: f32 = 480.0;
const REF_ASPECT: f32 = REF_W / REF_H;
const BASE_DP: f32 = 1.2;

/// Converts a Java ARGB packed color (signed i32 or unsigned u32) into Horizon's
/// `HudBatch` ABGR `u32` (`rgba(r, g, b, a)`).
fn argb_to_abgr(argb: u32) -> u32 {
    let a = (argb >> 24) & 0xff;
    let r = (argb >> 16) & 0xff;
    let g = (argb >> 8) & 0xff;
    let b = argb & 0xff;
    (a << 24) | (b << 16) | (g << 8) | r
}

fn parse_color_u32(val: Option<&Value>, fallback_argb: u32) -> u32 {
    match val {
        Some(v) if v.is_u64() => argb_to_abgr(v.as_u64().unwrap_or(fallback_argb as u64) as u32),
        Some(v) if v.is_i64() => argb_to_abgr(v.as_i64().unwrap_or(fallback_argb as i64) as u32),
        Some(v) if v.is_string() => {
            let s = v.as_str().unwrap_or("").trim().trim_start_matches('#');
            let parsed = u32::from_str_radix(s, 16).ok().map(|n| {
                if s.len() <= 6 { 0xFF00_0000 | n } else { n }
            });
            argb_to_abgr(parsed.unwrap_or(fallback_argb))
        }
        _ => argb_to_abgr(fallback_argb),
    }
}

fn token_from_event(ev: ControlEvent, payload: &str) -> Option<String> {
    match ev {
        ControlEvent::Token(tok) => Some(tok.to_owned()),
        ControlEvent::ToggleKeyboard => Some("action:horizon.toggle_keyboard".to_owned()),
        ControlEvent::ToggleControls => Some("action:horizon.toggle_controls".to_owned()),
        ControlEvent::OpenMenu => Some("action:horizon.open_menu".to_owned()),
        ControlEvent::ScrollUpHeld => Some("mouse.scroll.up.held".to_owned()),
        ControlEvent::ScrollDownHeld => Some("mouse.scroll.down.held".to_owned()),
        ControlEvent::LayerSwitch => (!payload.is_empty()).then(|| format!("action:horizon.switch_layer:{payload}")),
        ControlEvent::LayerShow => (!payload.is_empty()).then(|| format!("action:horizon.show_layer:{payload}")),
        ControlEvent::LayerHide => (!payload.is_empty()).then(|| format!("action:horizon.hide_layer:{payload}")),
        ControlEvent::SendText => (!payload.is_empty()).then(|| format!("action:horizon.send_text:{payload}")),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_placement(
    cx_norm: f32,
    cy_norm: f32,
    w_norm: f32,
    h_norm: f32,
    opacity: f32,
    bg_abgr: u32,
    stroke_abgr: u32,
    stroke_size: f32,
    shape: u8,
    roundness: f32,
    on_click_abgr: u32,
    on_click_stroke_abgr: u32,
    passthrough: bool,
    toggleable: bool,
    swipeable: bool,
    visible_in_game: bool,
    visible_in_menu: bool,
) -> Placement {
    let w_n = w_norm.clamp(0.005, 0.95);
    let h_n = h_norm.clamp(0.005, 0.95);
    // Solve `size`, `width_ratio`, `height_ratio` using `ref_aspect = REF_ASPECT`
    // so `rect(aspect)` reproduces `(w_n, h_n)` identically at any screen aspect
    // with zero horizontal gap or aspect distortion.
    let size = h_n.clamp(0.005, 0.85);
    let width_ratio = ((w_n * REF_ASPECT) / size).clamp(0.1, 64.0);
    let height_ratio = (h_n / size).clamp(0.1, 64.0);
    let half_w = ((size * width_ratio) / REF_ASPECT) * 0.5;
    let half_h = (size * height_ratio) * 0.5;
    let cx = if half_w < 0.5 {
        cx_norm.clamp(half_w, 1.0 - half_w)
    } else {
        0.5
    };
    let cy = if half_h < 0.5 {
        cy_norm.clamp(half_h, 1.0 - half_h)
    } else {
        0.5
    };

    let mut p = Placement::new(cx, cy, size);
    p.use_goy_style = true;
    p.ref_aspect = REF_ASPECT;
    p.width_ratio = width_ratio;
    p.height_ratio = height_ratio;
    p.opacity = opacity.clamp(0.0, 1.0);
    p.shape = shape;
    p.roundness = roundness.clamp(0.0, 50.0);
    p.stroke_roundness = p.roundness;
    p.stroke_size = stroke_size.clamp(0.0, 8.0);
    p.button_color = bg_abgr;
    p.stroke_color = stroke_abgr;
    p.on_click_color = on_click_abgr;
    p.on_click_stroke_color = on_click_stroke_abgr;
    p.on_toggle_color = argb_to_abgr(0xCC543B94);
    p.on_toggle_stroke_color = argb_to_abgr(0xFFEADDFF);
    p.passthrough = passthrough;
    p.toggleable = toggleable;
    p.swipeable = swipeable;
    p.visible_in_game = visible_in_game;
    p.visible_in_menu = visible_in_menu;
    p.sanitize();
    p
}

fn base_java_settings(style_name: String, desc: String) -> Settings {
    let mut s = Settings::default_for_style(ControlStyle::GoyControl);
    s.control_style = ControlStyle::GoyControl;
    s.preset_name = style_name;
    s.preset_description = desc;
    s.custom_buttons.clear();
    // Hide every Horizon built-in control (including `Control::Move` joystick/D-pad)
    // whenever controls are imported from external launchers so the imported layout
    // stands alone unless the imported layout explicitly defines a joystick.
    for m in 0..3 {
        for c in Control::ALL {
            s.layouts[m][c as usize].use_goy_style = true;
            s.layouts[m][c as usize].visible_in_game = false;
            s.layouts[m][c as usize].visible_in_menu = false;
        }
    }
    s.button_visibility = [2; 14];
    s
}

fn enable_imported_joystick(s: &mut Settings, cx_norm: f32, cy_norm: f32, diameter_h_norm: f32) {
    let size = diameter_h_norm.clamp(0.06, 0.55);
    let half_w = (size / REF_ASPECT) * 0.5;
    let half_h = size * 0.5;
    let mut p = Placement::new(
        cx_norm.clamp(half_w, 1.0 - half_w),
        cy_norm.clamp(half_h, 1.0 - half_h),
        size,
    );
    p.use_goy_style = true;
    p.ref_aspect = REF_ASPECT;
    p.visible_in_game = true;
    p.visible_in_menu = false;
    p.sanitize();
    s.mode = Mode::Crosshair;
    s.analog_movement = true;
    s.dynamic_joystick = false;
    s.button_visibility[Control::Move as usize] = 0;
    for row in &mut s.layouts {
        row[Control::Move as usize] = p;
    }
}

// ---------------------------------------------------------------------------
// PojavLauncher / Amethyst / Mojo / AngelAura importer (`mControlDataList`)
// ---------------------------------------------------------------------------

fn expand_pojav_vars(expr: &str, vars: &[(&str, f32)]) -> String {
    let mut out = String::with_capacity(expr.len() + 16);
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(rel_end) = expr[i + 2..].find('}') {
                let key = expr[i + 2..i + 2 + rel_end].trim();
                if let Some((_, val)) = vars.iter().find(|(k, _)| *k == key) {
                    out.push_str(&format!("{val:.6}"));
                    i += 2 + rel_end + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn eval_pojav_expr(expr: &str, vars: &[(&str, f32)], base_dp: f32) -> Option<f32> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let expanded = expand_pojav_vars(trimmed, vars);
    let tokens = tokenize_expr(&expanded, vars)?;
    let mut pos = 0;
    let val = parse_expr_add_sub(&tokens, &mut pos, base_dp)?;
    (pos == tokens.len() && val.is_finite()).then_some(val)
}

#[derive(Clone, Debug)]
enum Tok {
    Num(f32),
    Ident(String),
    Op(char),
    Comma,
    LParen,
    RParen,
}

fn tokenize_expr(s: &str, vars: &[(&str, f32)]) -> Option<Vec<Tok>> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                let exp_pos = i;
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                let digits_start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i == digits_start {
                    i = exp_pos;
                }
            }
            let num: f32 = s[start..i].parse().ok()?;
            out.push(Tok::Num(num));
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
            {
                i += 1;
            }
            let ident = &s[start..i];
            if let Some((_, val)) = vars.iter().find(|(k, _)| *k == ident) {
                out.push(Tok::Num(*val));
            } else {
                out.push(Tok::Ident(ident.to_ascii_lowercase()));
            }
            continue;
        }
        match ch {
            '+' | '-' | '*' | '/' | '^' | '%' => {
                out.push(Tok::Op(ch));
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            _ => return None,
        }
    }
    Some(out)
}

fn parse_expr_add_sub(toks: &[Tok], pos: &mut usize, base_dp: f32) -> Option<f32> {
    let mut val = parse_expr_mul_div(toks, pos, base_dp)?;
    while let Some(Tok::Op(op @ ('+' | '-'))) = toks.get(*pos) {
        *pos += 1;
        let rhs = parse_expr_mul_div(toks, pos, base_dp)?;
        val = if *op == '+' { val + rhs } else { val - rhs };
    }
    Some(val)
}

fn parse_expr_mul_div(toks: &[Tok], pos: &mut usize, base_dp: f32) -> Option<f32> {
    let mut val = parse_expr_pow(toks, pos, base_dp)?;
    while let Some(Tok::Op(op @ ('*' | '/' | '%'))) = toks.get(*pos) {
        *pos += 1;
        let rhs = parse_expr_pow(toks, pos, base_dp)?;
        val = match *op {
            '*' => val * rhs,
            '/' => {
                if rhs.abs() > 1e-6 {
                    val / rhs
                } else {
                    0.0
                }
            }
            '%' => {
                if rhs.abs() > 1e-6 {
                    val % rhs
                } else {
                    0.0
                }
            }
            _ => val,
        };
    }
    Some(val)
}

fn parse_expr_pow(toks: &[Tok], pos: &mut usize, base_dp: f32) -> Option<f32> {
    let lhs = parse_expr_unary(toks, pos, base_dp)?;
    if let Some(Tok::Op('^')) = toks.get(*pos) {
        *pos += 1;
        let rhs = parse_expr_pow(toks, pos, base_dp)?;
        return Some(lhs.powf(rhs));
    }
    Some(lhs)
}

fn parse_expr_unary(toks: &[Tok], pos: &mut usize, base_dp: f32) -> Option<f32> {
    if let Some(Tok::Op('-')) = toks.get(*pos) {
        *pos += 1;
        return Some(-parse_expr_unary(toks, pos, base_dp)?);
    }
    if let Some(Tok::Op('+')) = toks.get(*pos) {
        *pos += 1;
        return parse_expr_unary(toks, pos, base_dp);
    }
    match toks.get(*pos)? {
        Tok::Num(n) => {
            *pos += 1;
            Some(*n)
        }
        Tok::Ident(name) => {
            let fn_name = name.clone();
            *pos += 1;
            if matches!(toks.get(*pos), Some(Tok::LParen)) {
                *pos += 1;
                let mut args = Vec::new();
                if !matches!(toks.get(*pos), Some(Tok::RParen)) {
                    loop {
                        args.push(parse_expr_add_sub(toks, pos, base_dp)?);
                        if matches!(toks.get(*pos), Some(Tok::Comma)) {
                            *pos += 1;
                        } else {
                            break;
                        }
                    }
                }
                if !matches!(toks.get(*pos), Some(Tok::RParen)) {
                    return None;
                }
                *pos += 1;
                eval_pojav_fn(&fn_name, &args, base_dp)
            } else {
                None
            }
        }
        Tok::LParen => {
            *pos += 1;
            let v = parse_expr_add_sub(toks, pos, base_dp)?;
            if !matches!(toks.get(*pos), Some(Tok::RParen)) {
                return None;
            }
            *pos += 1;
            Some(v)
        }
        _ => None,
    }
}

fn eval_pojav_fn(name: &str, args: &[f32], base_dp: f32) -> Option<f32> {
    let a0 = *args.first()?;
    match name {
        // Pojav ControlData.java: px(dp_val) -> dpToPx(dp_val); dp(px_val) -> pxToDp(px_val)
        "px" => Some(a0 * base_dp),
        "dp" => Some(if base_dp > 1e-6 { a0 / base_dp } else { a0 }),
        "abs" | "math.abs" => Some(a0.abs()),
        "sqrt" | "math.sqrt" => Some(a0.max(0.0).sqrt()),
        "floor" | "math.floor" => Some(a0.floor()),
        "ceil" | "math.ceil" => Some(a0.ceil()),
        "round" | "math.round" => Some(a0.round()),
        "sin" | "math.sin" => Some(a0.sin()),
        "cos" | "math.cos" => Some(a0.cos()),
        "tan" | "math.tan" => Some(a0.tan()),
        "min" | "math.min" => Some(a0.min(*args.get(1)?)),
        "max" | "math.max" => Some(a0.max(*args.get(1)?)),
        "pow" | "math.pow" => Some(a0.powf(*args.get(1)?)),
        _ => None,
    }
}

fn detect_pojav_uses_android_keycodes(value: &Value) -> bool {
    let version = value["version"].as_i64().unwrap_or(1);
    let mut android_only = 0usize;
    let mut glfw_high = 0usize;
    let mut inspect = |arr: Option<&Vec<Value>>| {
        let Some(items) = arr else {
            return;
        };
        for item in items {
            if let Some(codes) = item["keycodes"].as_array() {
                for code in codes.iter().filter_map(|c| c.as_i64()) {
                    if code <= 0 {
                        continue;
                    }
                    if (256..=348).contains(&code) {
                        glfw_high += 1;
                    } else if matches!(
                        code,
                        7..=31 | 33..=38 | 40..=43 | 62..=64 | 67 | 111..=160
                    ) {
                        android_only += 1;
                    }
                }
            }
            if let Some(c) = item["keycode"].as_i64()
                && c > 0
            {
                if (256..=348).contains(&c) {
                    glfw_high += 1;
                } else if matches!(c, 4 | 7..=31 | 33..=38 | 40..=43 | 62..=64 | 67 | 111..=160) {
                    android_only += 1;
                }
            }
        }
    };
    inspect(value["mControlDataList"].as_array());
    if let Some(drawers) = value["mDrawerDataList"].as_array() {
        for drawer in drawers {
            inspect(drawer["buttonProperties"].as_array());
        }
    }
    if version >= 9 && glfw_high == 0 {
        true
    } else {
        android_only > 0 && glfw_high == 0
    }
}

fn decode_pojav_keycode(code: i32, use_android: bool) -> Option<ControlEvent> {
    if code == 0 {
        return None;
    }
    if code < 0 {
        return pojav_code(code);
    }
    if use_android {
        pojav_code_android(code).or_else(|| {
            // Fallback for any layout mixing GLFW 256..348 codes
            (code >= 256).then(|| pojav_code(code)).flatten()
        })
    } else {
        pojav_code(code).or_else(|| {
            // Fallback if an Android-only code slipped into a lower version
            android_keycode(code).map(ControlEvent::Token)
        })
    }
}

fn collect_pojav_tokens(btn_val: &Value, use_android: bool, has_extra_token: bool) -> Vec<String> {
    let decode = |c: i32| {
        let ev = decode_pojav_keycode(c, use_android)?;
        let tok = token_from_event(ev, "")?;
        CustomButton::valid_token(&tok).then_some(tok)
    };
    let mut tokens = Vec::new();
    if let Some(arr) = btn_val["keycodes"].as_array() {
        for code in arr {
            if let Some(tok) = code.as_i64().and_then(|c| decode(c as i32)) {
                tokens.push(tok);
            } else if !has_extra_token && tokens.len() < 4 {
                tokens.push("key.keyboard.unknown".to_owned());
            }
        }
        while tokens.len() > 1 && tokens.last().is_some_and(|t| t == "key.keyboard.unknown") {
            tokens.pop();
        }
    } else if let Some(single) = btn_val["keycode"].as_i64() {
        // Pojav v1 legacy format
        for (cond, code) in [
            (btn_val["holdShift"].as_bool().unwrap_or(false), 340),
            (btn_val["holdCtrl"].as_bool().unwrap_or(false), 341),
            (btn_val["holdAlt"].as_bool().unwrap_or(false), 342),
            (true, single as i32),
        ] {
            if cond && let Some(tok) = decode(code) {
                tokens.push(tok);
            }
        }
    }
    tokens
}

fn eval_pojav_rect(
    btn_val: &Value,
    scale: f32,
    v1_xy_scale: (f32, f32),
    override_xy_px: Option<(f32, f32)>,
    override_wh_dp: Option<(f32, f32)>,
) -> (f32, f32, f32, f32, f32, f32) {
    // Pojav saves button `width`/`height` already scaled at `scaledAt`.
    let dp = BASE_DP;
    let w_dp = override_wh_dp
        .map(|(w, _)| w)
        .unwrap_or_else(|| btn_val["width"].as_f64().unwrap_or(50.0) as f32)
        .max(2.0);
    let h_dp = override_wh_dp
        .map(|(_, h)| h)
        .unwrap_or_else(|| btn_val["height"].as_f64().unwrap_or(50.0) as f32)
        .max(2.0);
    let w_px = (w_dp * dp).max(2.0);
    let h_px = (h_dp * dp).max(2.0);
    let margin = 2.0 * dp;
    let pref_scale = if scale > 0.01 { 100.0 * scale } else { 100.0 };
    let vars = [
        ("width", w_px),
        ("height", h_px),
        ("screen_width", REF_W),
        ("screen_height", REF_H),
        ("margin", margin),
        ("left", 0.0),
        ("top", 0.0),
        ("right", REF_W - w_px),
        ("bottom", REF_H - h_px),
        ("preferred_scale", pref_scale),
    ];

    let (x_px, y_px) = if let Some((ox, oy)) = override_xy_px {
        (ox, oy)
    } else {
        let x = btn_val["dynamicX"]
            .as_str()
            .and_then(|e| eval_pojav_expr(e, &vars, BASE_DP))
            .or_else(|| btn_val["x"].as_f64().map(|v| v as f32 * v1_xy_scale.0))
            .unwrap_or(0.0);
        let y = btn_val["dynamicY"]
            .as_str()
            .and_then(|e| eval_pojav_expr(e, &vars, BASE_DP))
            .or_else(|| btn_val["y"].as_f64().map(|v| v as f32 * v1_xy_scale.1))
            .unwrap_or(0.0);
        (x, y)
    };
    (x_px, y_px, w_px, h_px, w_dp, h_dp)
}

#[allow(clippy::too_many_arguments)]
fn push_pojav_button(
    btn_val: &Value,
    scale: f32,
    use_android: bool,
    v1_xy_scale: (f32, f32),
    override_xy_px: Option<(f32, f32)>,
    override_wh_dp: Option<(f32, f32)>,
    extra_token: Option<String>,
    layer_id: String,
    hidden: bool,
    out: &mut Vec<CustomButton>,
) -> (f32, f32, f32, f32, f32, f32) {
    let (x_px, y_px, w_px, h_px, w_dp, h_dp) =
        eval_pojav_rect(btn_val, scale, v1_xy_scale, override_xy_px, override_wh_dp);

    let cx_norm = ((x_px + w_px * 0.5) / REF_W).clamp(0.02, 0.98);
    let cy_norm = ((y_px + h_px * 0.5) / REF_H).clamp(0.02, 0.98);
    let w_norm = (w_px / REF_W).clamp(0.02, 0.75);
    let h_norm = (h_px / REF_H).clamp(0.02, 0.75);

    let mut tokens = collect_pojav_tokens(btn_val, use_android, extra_token.is_some());
    if let Some(tok) = extra_token {
        if !tokens.contains(&tok) {
            tokens.insert(0, tok);
        }
    }

    let name = btn_val["name"].as_str().unwrap_or("").trim().to_owned();
    let opacity = btn_val["opacity"]
        .as_f64()
        .map(|v| v as f32)
        .or_else(|| {
            btn_val["transparency"]
                .as_f64()
                .map(|t| ((100.0 - t) / 100.0) as f32)
        })
        .unwrap_or(1.0);
    let bg = parse_color_u32(btn_val.get("bgColor"), 0x4D00_0000);
    let stroke = parse_color_u32(btn_val.get("strokeColor"), 0xFFFFFFFF);
    let stroke_w = btn_val["strokeWidth"].as_f64().unwrap_or(1.2) as f32;
    let radius_pct = btn_val["cornerRadius"]
        .as_f64()
        .map(|v| v as f32)
        .unwrap_or_else(|| if btn_val["isRound"].as_bool().unwrap_or(false) { 35.0 } else { 0.0 })
        .clamp(0.0, 100.0);
    let roundness_dp = (w_dp.min(h_dp) * 0.5) * (radius_pct / 100.0);
    let shape = if radius_pct >= 99.0 && (w_dp - h_dp).abs() < 2.0 {
        2
    } else if roundness_dp > 0.25 {
        0
    } else {
        1
    };

    let is_toggle = btn_val["isToggle"].as_bool().unwrap_or(false);
    let is_swipe = btn_val["isSwipeable"].as_bool().unwrap_or(false);
    let passthrough = btn_val["passThruEnabled"].as_bool().unwrap_or(false);
    let has_non_hideable_action = tokens
        .iter()
        .any(|t| t == "action:horizon.toggle_controls" || t == "mouse.virtual.toggle");
    let hideable = btn_val["isHideable"].as_bool().unwrap_or(!has_non_hideable_action);
    let is_menu_relevant = !hideable
        || tokens.iter().any(|t| {
            matches!(
                t.as_str(),
                "key.mouse.left"
                    | "key.mouse.right"
                    | "mouse.virtual.toggle"
                    | "action:horizon.toggle_keyboard"
                    | "action:horizon.open_menu"
            )
        });
    let visible_in_game = btn_val["displayInGame"].as_bool().unwrap_or(true);
    let visible_in_menu = btn_val["displayInMenu"].as_bool().unwrap_or(is_menu_relevant);

    let placement = build_placement(
        cx_norm,
        cy_norm,
        w_norm,
        h_norm,
        opacity,
        bg,
        stroke,
        stroke_w,
        shape,
        roundness_dp,
        argb_to_abgr(0x997C5CFF),
        argb_to_abgr(0xFFEADDFF),
        passthrough,
        is_toggle,
        is_swipe,
        visible_in_game,
        visible_in_menu,
    );

    let mut btn = CustomButton::new(
        tokens
            .first()
            .cloned()
            .unwrap_or_else(|| "key.keyboard.unknown".to_owned()),
        out.len(),
    );
    btn.second_key = tokens.get(1).cloned().unwrap_or_default();
    btn.third_key = tokens.get(2).cloned().unwrap_or_default();
    btn.fourth_key = tokens.get(3).cloned().unwrap_or_default();
    btn.custom_label = name;
    btn.always_visible = !hideable || has_non_hideable_action;
    btn.layer_id = layer_id;
    btn.hidden = hidden;
    btn.layouts = [placement; 3];
    out.push(btn);
    (x_px, y_px, w_px, h_px, w_dp, h_dp)
}

fn compute_v1_xy_scale(value: &Value) -> (f32, f32) {
    let mut max_x = 0.0f32;
    let mut max_y = 0.0f32;
    if let Some(list) = value["mControlDataList"].as_array() {
        for item in list {
            if item["dynamicX"].as_str().is_none_or(|s| s.trim().is_empty())
                && let Some(x) = item["x"].as_f64()
            {
                max_x = max_x.max(x as f32);
            }
            if item["dynamicY"].as_str().is_none_or(|s| s.trim().is_empty())
                && let Some(y) = item["y"].as_f64()
            {
                max_y = max_y.max(y as f32);
            }
        }
    }
    let sx = if max_x > REF_W { REF_W / (max_x * 1.15) } else { 1.0 };
    let sy = if max_y > REF_H { REF_H / (max_y * 1.15) } else { 1.0 };
    (sx, sy)
}

fn import_pojav(value: &Value, file_name: &str) -> Result<Settings, String> {
    let scaled_at = value["scaledAt"].as_f64().unwrap_or(100.0) as f32;
    let scale = if scaled_at > 1.0 { scaled_at / 100.0 } else { 1.0 };
    let use_android = detect_pojav_uses_android_keycodes(value);
    let v1_xy_scale = compute_v1_xy_scale(value);

    let stem = file_name
        .trim()
        .strip_suffix(".json")
        .unwrap_or(file_name.trim())
        .replace('-', " ");
    let preset_name = if stem.is_empty() || stem.eq_ignore_ascii_case("default") {
        "Pojav Imported".to_owned()
    } else {
        stem.chars().take(48).collect()
    };

    let mut s = base_java_settings(
        preset_name,
        "Imported PojavLauncher control layout".to_owned(),
    );

    if let Some(list) = value["mControlDataList"].as_array() {
        for item in list {
            push_pojav_button(
                item,
                scale,
                use_android,
                v1_xy_scale,
                None,
                None,
                None,
                String::new(),
                false,
                &mut s.custom_buttons,
            );
        }
    }
    if let Some(drawers) = value["mDrawerDataList"].as_array() {
        for (d_idx, drawer) in drawers.iter().enumerate() {
            let layer_id = format!("pojav_drawer_{d_idx}");
            let orient = drawer["orientation"].as_str().unwrap_or("FREE");
            let mut drawer_rect = (0.0f32, 0.0f32, 60.0f32, 60.0f32, 50.0f32, 50.0f32);
            if let Some(props) = drawer.get("properties") {
                drawer_rect = push_pojav_button(
                    props,
                    scale,
                    use_android,
                    v1_xy_scale,
                    None,
                    None,
                    Some(format!("action:horizon.switch_layer:{layer_id}")),
                    String::new(),
                    false,
                    &mut s.custom_buttons,
                );
            }
            if let Some(children) = drawer["buttonProperties"].as_array() {
                let margin = 2.0 * BASE_DP;
                let (dx, dy, dw, dh, dw_dp, dh_dp) = drawer_rect;
                for (c_idx, child) in children.iter().enumerate() {
                    let step = (c_idx + 1) as f32;
                    let (override_xy, override_wh) = match orient {
                        "RIGHT" => (Some((dx + (dw + margin) * step, dy)), Some((dw_dp, dh_dp))),
                        "LEFT" => (Some((dx - (dw + margin) * step, dy)), Some((dw_dp, dh_dp))),
                        "UP" => (Some((dx, dy - (dh + margin) * step)), Some((dw_dp, dh_dp))),
                        "DOWN" => (Some((dx, dy + (dh + margin) * step)), Some((dw_dp, dh_dp))),
                        _ => (None, None),
                    };
                    push_pojav_button(
                        child,
                        scale,
                        use_android,
                        v1_xy_scale,
                        override_xy,
                        override_wh,
                        None,
                        layer_id.clone(),
                        true,
                        &mut s.custom_buttons,
                    );
                }
            }
        }
    }
    let active_joy = value["mJoystickDataList"].as_array().and_then(|joys| {
        joys.iter()
            .find(|j| j["displayInGame"].as_bool() != Some(false))
    });
    let has_joystick = active_joy.is_some();
    if let Some(joy) = active_joy {
        let (jx, jy, jw, jh, _, _) = eval_pojav_rect(joy, scale, v1_xy_scale, None, None);
        let cx_norm = (jx + jw * 0.5) / REF_W;
        let cy_norm = (jy + jh * 0.5) / REF_H;
        let diam_h = (jw.min(jh) / REF_H).clamp(0.06, 0.55);
        enable_imported_joystick(&mut s, cx_norm, cy_norm, diam_h);
        let j_opacity = joy["opacity"].as_f64().unwrap_or(1.0) as f32;
        let absolute = joy["absolute"].as_bool().unwrap_or(true);
        let in_game = joy["displayInGame"].as_bool().unwrap_or(true);
        let in_menu = joy["displayInMenu"].as_bool().unwrap_or(false);
        let bg_abgr = argb_to_abgr(joy["bgColor"].as_i64().unwrap_or(0x40FFFFFF) as u32);
        let stroke_abgr = argb_to_abgr(joy["strokeColor"].as_i64().unwrap_or(0x80FFFFFF) as u32);
        let stroke_w = (joy["strokeWidth"].as_f64().unwrap_or(2.0) as f32).clamp(0.0, 8.0);
        s.dynamic_joystick = !absolute;
        s.joystick_style.base_color = bg_abgr;
        s.joystick_style.base_stroke_color = stroke_abgr;
        s.joystick_style.base_stroke = stroke_w;
        for row in &mut s.layouts {
            let p = &mut row[Control::Move as usize];
            p.opacity = j_opacity.clamp(0.0, 1.0);
            p.visible_in_game = in_game;
            p.visible_in_menu = in_menu;
            p.button_color = bg_abgr;
            p.stroke_color = stroke_abgr;
            p.stroke_size = stroke_w;
        }
    }
    if s.custom_buttons.is_empty() && !has_joystick {
        return Err("Pojav control layout contained no buttons".into());
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// Zalith Launcher importer (`layers` + `styles` + `info` + `editorVersion`)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct ZalithStyle {
    bg_abgr: u32,
    pressed_bg_abgr: u32,
    stroke_abgr: u32,
    pressed_stroke_abgr: u32,
    border_width: f32,
    roundness: f32,
    alpha: f32,
}

impl Default for ZalithStyle {
    fn default() -> Self {
        Self {
            bg_abgr: argb_to_abgr(0x8000_0000),
            pressed_bg_abgr: argb_to_abgr(0xB387_4888),
            stroke_abgr: argb_to_abgr(0xFFFFFFFF),
            pressed_stroke_abgr: argb_to_abgr(0xFFEADDFF),
            border_width: 0.0,
            roundness: 8.0,
            alpha: 1.0,
        }
    }
}

fn pick_zalith_text(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return s.trim().to_owned();
    }
    if let Some(def) = v.get("default").and_then(|d| d.as_str()) {
        return def.trim().to_owned();
    }
    if let Some(arr) = v.get("matchQueue").and_then(|q| q.as_array()) {
        for item in arr {
            if let Some(val) = item.get("value").and_then(|x| x.as_str())
                && !val.trim().is_empty()
            {
                return val.trim().to_owned();
            }
        }
    }
    String::new()
}

fn parse_zalith_styles(value: &Value) -> HashMap<String, ZalithStyle> {
    let mut map = HashMap::new();
    let Some(styles) = value["styles"].as_array() else {
        return map;
    };
    for st in styles {
        let Some(uuid) = st["uuid"].as_str() else {
            continue;
        };
        let common = st["commonStyle"].as_bool().unwrap_or(false);
        let visual = if common {
            st.get("lightStyle")
                .filter(|v| v.is_object())
                .or_else(|| st.get("darkStyle").filter(|v| v.is_object()))
        } else {
            st.get("darkStyle")
                .filter(|v| v.is_object())
                .or_else(|| st.get("lightStyle").filter(|v| v.is_object()))
        };
        let Some(vis) = visual else {
            continue;
        };
        let br = &vis["borderRadius"];
        let corners = [
            br["topStart"].as_f64(),
            br["topEnd"].as_f64(),
            br["bottomStart"].as_f64(),
            br["bottomEnd"].as_f64(),
        ];
        let valid: Vec<f32> = corners.into_iter().flatten().map(|v| v as f32).collect();
        let border_r = if valid.is_empty() {
            0.0
        } else {
            valid.iter().sum::<f32>() / valid.len() as f32
        };
        let parsed = ZalithStyle {
            bg_abgr: parse_color_u32(vis.get("backgroundColor"), 0x8000_0000),
            pressed_bg_abgr: parse_color_u32(vis.get("pressedBackgroundColor"), 0xB387_4888),
            stroke_abgr: parse_color_u32(vis.get("borderColor"), 0xFFFFFFFF),
            pressed_stroke_abgr: parse_color_u32(vis.get("pressedBorderColor"), 0xFFEADDFF),
            border_width: vis["borderWidth"].as_f64().unwrap_or(0.0) as f32,
            roundness: (border_r * 0.45).clamp(0.0, 50.0),
            alpha: vis["alpha"].as_f64().unwrap_or(1.0) as f32,
        };
        map.insert(uuid.to_owned(), parsed);
    }
    map
}

fn parse_zalith_visibility(raw: &str) -> (bool, bool) {
    match raw.trim().to_ascii_lowercase().as_str() {
        "in_game" | "ingame" => (true, false),
        "in_menu" | "inmenu" => (false, true),
        _ => (true, true),
    }
}

fn import_zalith(value: &Value, file_name: &str) -> Result<Settings, String> {
    let editor_ver = value["editorVersion"].as_i64().unwrap_or(12);
    // Zalith editorVersion == 1 stored positions and percentage sizes on 0..1000 instead of 0..10000.
    let pct_divisor = if editor_ver <= 1 { 1000.0 } else { 10000.0 };

    let info_name = pick_zalith_text(&value["info"]["name"]);
    let stem = file_name
        .trim()
        .strip_suffix(".json")
        .unwrap_or(file_name.trim())
        .replace('-', " ");
    let preset_name = if !info_name.is_empty() && !info_name.eq_ignore_ascii_case("default") {
        info_name
    } else if !stem.is_empty() && !stem.eq_ignore_ascii_case("default") {
        stem
    } else {
        "Zalith Imported".to_owned()
    };
    let info_desc = pick_zalith_text(&value["info"]["description"]);
    let preset_desc = if info_desc.is_empty() {
        "Imported Zalith Launcher control layout".to_owned()
    } else {
        info_desc
    };

    let styles = parse_zalith_styles(value);
    let mut has_joystick = value.get("joystick").is_some_and(|v| v.is_object());
    let mut s = base_java_settings(
        preset_name.chars().take(48).collect(),
        preset_desc.chars().take(120).collect(),
    );

    let Some(layers) = value["layers"].as_array() else {
        return Err("Zalith control layout has no layers".into());
    };
    // Compose renders layers[0] on top of later layers; Horizon renders/hits later custom_buttons on top.
    for layer in layers.iter().rev() {
        let layer_uuid = layer["uuid"].as_str().unwrap_or("").to_owned();
        let layer_hide = layer["hide"].as_bool().unwrap_or(false);
        let layer_vis = layer["visibilityType"].as_str().unwrap_or("always");
        let layer_vis_pair = parse_zalith_visibility(layer_vis);

        if let Some(joys) = layer["joystickButtons"].as_array()
            && !layer_hide
            && let Some(joy) = joys.first()
        {
            has_joystick = true;
            let size_type = joy["sizeType"].as_str().unwrap_or("percentage");
            let diam_h = if size_type.eq_ignore_ascii_case("dp") {
                (joy["sizeDp"].as_f64().unwrap_or(200.0) as f32 * BASE_DP) / REF_H
            } else {
                joy["sizePercentage"].as_f64().unwrap_or(2500.0) as f32 / pct_divisor
            };
            let w_norm = (diam_h / REF_ASPECT).clamp(0.05, 0.5);
            let h_norm = diam_h.clamp(0.08, 0.5);
            let px = (joy["position"]["x"].as_f64().unwrap_or(1000.0) as f32 / pct_divisor)
                .clamp(0.0, 1.0);
            let py = (joy["position"]["y"].as_f64().unwrap_or(8000.0) as f32 / pct_divisor)
                .clamp(0.0, 1.0);
            let cx = px * (1.0 - w_norm) + w_norm * 0.5;
            let cy = py * (1.0 - h_norm) + h_norm * 0.5;
            enable_imported_joystick(&mut s, cx, cy, h_norm);
        }

        let Some(buttons) = layer["normalButtons"].as_array() else {
            continue;
        };
        for b in buttons {
            let label = pick_zalith_text(&b["text"]);
            let size_obj = &b["buttonSize"];
            let size_kind = size_obj["type"].as_str().unwrap_or("percentage");
            let (w_norm, h_norm) = if size_kind.eq_ignore_ascii_case("dp") {
                let w_dp = size_obj["widthDp"].as_f64().unwrap_or(50.0) as f32;
                let h_dp = size_obj["heightDp"].as_f64().unwrap_or(50.0) as f32;
                ((w_dp * BASE_DP) / REF_W, (h_dp * BASE_DP) / REF_H)
            } else if size_kind.eq_ignore_ascii_case("wrap_content") {
                let chars = label.chars().count().max(2) as f32;
                let w_dp = (chars * 10.0 + 22.0).clamp(36.0, 220.0);
                let h_dp = 38.0f32;
                ((w_dp * BASE_DP) / REF_W, (h_dp * BASE_DP) / REF_H)
            } else {
                let wp =
                    size_obj["widthPercentage"].as_f64().unwrap_or(1000.0) as f32 / pct_divisor;
                let hp =
                    size_obj["heightPercentage"].as_f64().unwrap_or(1000.0) as f32 / pct_divisor;
                let w_ref = size_obj["widthReference"].as_str().unwrap_or("screen_width");
                let h_ref = size_obj["heightReference"]
                    .as_str()
                    .unwrap_or("screen_height");
                let wn = if w_ref.eq_ignore_ascii_case("screen_height") {
                    wp / REF_ASPECT
                } else {
                    wp
                };
                let hn = if h_ref.eq_ignore_ascii_case("screen_width") {
                    hp * REF_ASPECT
                } else {
                    hp
                };
                (wn, hn)
            };
            let w_norm = w_norm.clamp(0.005, 0.95);
            let h_norm = h_norm.clamp(0.005, 0.95);

            // Zalith position x, y are 0..10000 normalized along the movable range `(1.0 - size)`.
            let px = (b["position"]["x"].as_f64().unwrap_or(5000.0) as f32 / pct_divisor)
                .clamp(0.0, 1.0);
            let py = (b["position"]["y"].as_f64().unwrap_or(5000.0) as f32 / pct_divisor)
                .clamp(0.0, 1.0);
            let cx_norm = px * (1.0 - w_norm) + w_norm * 0.5;
            let cy_norm = py * (1.0 - h_norm) + h_norm * 0.5;

            let mut tokens: Vec<String> = Vec::new();
            let mut switch_layers: Vec<String> = Vec::new();
            let mut show_layers: Vec<String> = Vec::new();
            let mut hide_layers: Vec<String> = Vec::new();
            if let Some(events) = b["clickEvents"].as_array() {
                for ev_val in events {
                    let kind = ev_val["type"].as_str().unwrap_or("");
                    let key = ev_val["key"].as_str().unwrap_or("");
                    match kind {
                        "switch_layer" if !key.is_empty() => {
                            if !switch_layers.iter().any(|id| id == key) {
                                switch_layers.push(key.to_owned());
                            }
                            continue;
                        }
                        "show_layer" if !key.is_empty() => {
                            if !show_layers.iter().any(|id| id == key) {
                                show_layers.push(key.to_owned());
                            }
                            continue;
                        }
                        "hide_layer" if !key.is_empty() => {
                            if !hide_layers.iter().any(|id| id == key) {
                                hide_layers.push(key.to_owned());
                            }
                            continue;
                        }
                        _ => {}
                    }
                    if let Some(ev) = zalith_event(kind, key)
                        && let Some(tok) = token_from_event(ev, key)
                        && CustomButton::valid_token(&tok)
                    {
                        tokens.push(tok);
                    }
                }
            }
            if !switch_layers.is_empty() {
                tokens.push(format!(
                    "action:horizon.switch_layer:{}",
                    switch_layers.join(",")
                ));
            }
            if !show_layers.is_empty() {
                tokens.push(format!(
                    "action:horizon.show_layer:{}",
                    show_layers.join(",")
                ));
            }
            if !hide_layers.is_empty() {
                tokens.push(format!(
                    "action:horizon.hide_layer:{}",
                    hide_layers.join(",")
                ));
            }

            let style_id = b["buttonStyle"].as_str().unwrap_or("");
            let style = styles.get(style_id).copied().unwrap_or_default();
            let btn_vis_pair =
                parse_zalith_visibility(b["visibilityType"].as_str().unwrap_or(layer_vis));
            let in_game = layer_vis_pair.0 && btn_vis_pair.0;
            let in_menu = layer_vis_pair.1 && btn_vis_pair.1;
            let is_swipe = b["isSwipple"].as_bool().unwrap_or(false);
            let passthrough = b["isPenetrable"].as_bool().unwrap_or(false);
            let is_toggle = b["isToggleable"].as_bool().unwrap_or(false);
            let shape = if style.roundness > 0.5 { 0 } else { 1 };

            let placement = build_placement(
                cx_norm,
                cy_norm,
                w_norm,
                h_norm,
                style.alpha,
                style.bg_abgr,
                style.stroke_abgr,
                style.border_width,
                shape,
                style.roundness,
                style.pressed_bg_abgr,
                style.pressed_stroke_abgr,
                passthrough,
                is_toggle,
                is_swipe,
                in_game,
                in_menu,
            );

            let mut btn = CustomButton::new(
                tokens
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "key.keyboard.unknown".to_owned()),
                s.custom_buttons.len(),
            );
            btn.second_key = tokens.get(1).cloned().unwrap_or_default();
            btn.third_key = tokens.get(2).cloned().unwrap_or_default();
            btn.fourth_key = tokens.get(3).cloned().unwrap_or_default();
            btn.custom_label = label;
            btn.always_visible = in_menu;
            btn.layer_id = layer_uuid.clone();
            btn.hidden = layer_hide;
            btn.layouts = [placement; 3];
            s.custom_buttons.push(btn);
        }
    }
    if s.custom_buttons.is_empty() && !has_joystick {
        return Err("Zalith control layout contained no buttons".into());
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// Fold Craft Launcher (FCL) importer (`viewGroups` / `buttonList`)
// ---------------------------------------------------------------------------

fn decode_fcl_keycode(code: i32, use_android: bool) -> Option<ControlEvent> {
    if code == 0 {
        return None;
    }
    match code {
        -1 => Some(ControlEvent::Token("key.mouse.left")),
        -2 => Some(ControlEvent::Token("key.mouse.middle")),
        -3 => Some(ControlEvent::Token("key.mouse.right")),
        -4 => Some(ControlEvent::Token("mouse.scroll.up")),
        -5 => Some(ControlEvent::Token("mouse.scroll.down")),
        _ => decode_pojav_keycode(code, use_android),
    }
}

fn fcl_norm_coord(v: f64) -> f32 {
    let f = v as f32;
    if f > 100.0 {
        (f / 1000.0).clamp(0.0, 1.0)
    } else if f > 1.0 {
        (f / 100.0).clamp(0.0, 1.0)
    } else {
        f.clamp(0.0, 1.0)
    }
}

fn push_fcl_button(
    btn: &Value,
    layer_id: &str,
    hidden: bool,
    use_android: bool,
    out: &mut Vec<CustomButton>,
) {
    let base = btn.get("baseInfo").filter(|v| v.is_object()).unwrap_or(btn);
    let label = base["text"]
        .as_str()
        .or_else(|| btn["text"].as_str())
        .unwrap_or("")
        .trim()
        .to_owned();

    let w_type = base["sizeWidthType"]
        .as_str()
        .or_else(|| base["widthType"].as_str())
        .unwrap_or("PERCENTAGE");
    let h_type = base["sizeHeightType"]
        .as_str()
        .or_else(|| base["heightType"].as_str())
        .unwrap_or("PERCENTAGE");
    let raw_w = base["sizeWidth"]
        .as_f64()
        .or_else(|| base["width"].as_f64())
        .unwrap_or(8.0);
    let raw_h = base["sizeHeight"]
        .as_f64()
        .or_else(|| base["height"].as_f64())
        .unwrap_or(12.0);
    let w_norm = if w_type.eq_ignore_ascii_case("ABSOLUTE") || w_type.eq_ignore_ascii_case("DP") {
        ((raw_w as f32 * BASE_DP) / REF_W).clamp(0.005, 0.95)
    } else {
        fcl_norm_coord(raw_w).clamp(0.005, 0.95)
    };
    let h_norm = if h_type.eq_ignore_ascii_case("ABSOLUTE") || h_type.eq_ignore_ascii_case("DP") {
        ((raw_h as f32 * BASE_DP) / REF_H).clamp(0.005, 0.95)
    } else {
        fcl_norm_coord(raw_h).clamp(0.005, 0.95)
    };

    let px = fcl_norm_coord(
        base["xPosition"]
            .as_f64()
            .or_else(|| base["x"].as_f64())
            .unwrap_or(500.0),
    );
    let py = fcl_norm_coord(
        base["yPosition"]
            .as_f64()
            .or_else(|| base["y"].as_f64())
            .unwrap_or(500.0),
    );
    let cx_norm = px * (1.0 - w_norm) + w_norm * 0.5;
    let cy_norm = py * (1.0 - h_norm) + h_norm * 0.5;

    let mut tokens = Vec::new();
    let ev_root = btn.get("event").filter(|v| v.is_object()).unwrap_or(btn);
    let mut passthrough = false;
    let mut toggleable = false;
    let mut swipeable = false;

    for ev_name in ["pressEvent", "clickEvent", "longPressEvent"] {
        let Some(ev) = ev_root.get(ev_name).filter(|v| v.is_object()) else {
            continue;
        };
        passthrough |= ev["pointerFollow"].as_bool().unwrap_or(false);
        toggleable |= ev["autoKeep"].as_bool().unwrap_or(false);
        swipeable |= ev["movable"].as_bool().unwrap_or(false);
        if ev["openMenu"].as_bool().unwrap_or(false)
            && !tokens.iter().any(|t| t == "action:horizon.open_menu")
        {
            tokens.push("action:horizon.open_menu".to_owned());
        }
        if ev["input"].as_bool().unwrap_or(false)
            && !tokens.iter().any(|t| t == "action:horizon.toggle_keyboard")
        {
            tokens.push("action:horizon.toggle_keyboard".to_owned());
        }
        if ev["switchTouchMode"].as_bool().unwrap_or(false)
            && !tokens.iter().any(|t| t == "mouse.virtual.toggle")
        {
            tokens.push("mouse.virtual.toggle".to_owned());
        }
        if let Some(txt) = ev["outputText"].as_str()
            && !txt.is_empty()
        {
            let tok = format!("action:horizon.send_text:{txt}");
            if !tokens.contains(&tok) {
                tokens.push(tok);
            }
        }
        if let Some(bind) = ev["bindViewGroup"].as_array() {
            let ids: Vec<String> = bind
                .iter()
                .filter_map(|v| v.as_str().map(str::trim))
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            if !ids.is_empty() {
                let tok = format!("action:horizon.switch_layer:{}", ids.join(","));
                if !tokens.contains(&tok) {
                    tokens.push(tok);
                }
            }
        }
        if let Some(codes) = ev["outputKeycodes"].as_array() {
            for code in codes.iter().filter_map(|c| c.as_i64()) {
                if let Some(parsed) = decode_fcl_keycode(code as i32, use_android)
                    && let Some(tok) = token_from_event(parsed, "")
                    && CustomButton::valid_token(&tok)
                {
                    tokens.push(tok);
                }
            }
        }
    }

    let vis = base["visibilityType"].as_str().unwrap_or("ALWAYS");
    let (in_game, in_menu) = parse_zalith_visibility(vis);
    let style = btn.get("style").filter(|v| v.is_object()).unwrap_or(base);
    let bg = parse_color_u32(style.get("fillColor"), 0x4D00_0000);
    let stroke = parse_color_u32(style.get("strokeColor"), 0xFFFFFFFF);
    let stroke_w = style["strokeWidth"].as_f64().unwrap_or(1.2) as f32;
    let round = (style["cornerRadius"].as_f64().unwrap_or(10.0) as f32).clamp(0.0, 50.0);
    let alpha = style["alpha"].as_f64().unwrap_or(1.0) as f32;

    let placement = build_placement(
        cx_norm,
        cy_norm,
        w_norm,
        h_norm,
        alpha,
        bg,
        stroke,
        stroke_w,
        if round > 0.5 { 0 } else { 1 },
        round,
        argb_to_abgr(0x997C5CFF),
        argb_to_abgr(0xFFEADDFF),
        passthrough,
        toggleable,
        swipeable,
        in_game,
        in_menu,
    );

    let mut out_btn = CustomButton::new(
        tokens
            .first()
            .cloned()
            .unwrap_or_else(|| "key.keyboard.unknown".to_owned()),
        out.len(),
    );
    out_btn.second_key = tokens.get(1).cloned().unwrap_or_default();
    out_btn.third_key = tokens.get(2).cloned().unwrap_or_default();
    out_btn.fourth_key = tokens.get(3).cloned().unwrap_or_default();
    out_btn.custom_label = label;
    out_btn.always_visible = in_menu;
    out_btn.layer_id = layer_id.to_owned();
    out_btn.hidden = hidden;
    out_btn.layouts = [placement; 3];
    out.push(out_btn);
}

fn import_fcl(value: &Value, file_name: &str) -> Result<Settings, String> {
    let stem = file_name
        .trim()
        .strip_suffix(".json")
        .unwrap_or(file_name.trim())
        .replace('-', " ");
    let preset_name = if stem.is_empty() || stem.eq_ignore_ascii_case("default") {
        "FCL Imported".to_owned()
    } else {
        stem.chars().take(48).collect()
    };

    let mut s = base_java_settings(
        preset_name,
        "Imported Fold Craft Launcher control layout".to_owned(),
    );
    let mut has_direction = false;

    if let Some(groups) = value["viewGroups"].as_array() {
        for g in groups {
            let gid = g["id"]
                .as_str()
                .or_else(|| g["name"].as_str())
                .unwrap_or("")
                .to_owned();
            let hidden = matches!(
                g["visibility"].as_str(),
                Some("INVISIBLE" | "GONE" | "invisible" | "gone")
            ) || g["visibility"].as_bool() == Some(false);
            let data = g.get("viewData").filter(|v| v.is_object()).unwrap_or(g);
            if let Some(buttons) = data["buttonList"].as_array() {
                for b in buttons {
                    push_fcl_button(b, &gid, hidden, false, &mut s.custom_buttons);
                }
            }
            if let Some(dirs) = data["directionList"].as_array()
                && !hidden
                && let Some(dir) = dirs.first()
            {
                has_direction = true;
                let base = dir.get("baseInfo").filter(|v| v.is_object()).unwrap_or(dir);
                let px = fcl_norm_coord(base["xPosition"].as_f64().unwrap_or(120.0));
                let py = fcl_norm_coord(base["yPosition"].as_f64().unwrap_or(780.0));
                enable_imported_joystick(&mut s, px, py, 0.25);
            }
        }
    } else if let Some(buttons) = value["buttonList"].as_array() {
        for b in buttons {
            push_fcl_button(b, "", false, false, &mut s.custom_buttons);
        }
    }
    if s.custom_buttons.is_empty() && !has_direction {
        return Err("FCL control layout contained no buttons".into());
    }
    Ok(s)
}

/// Converts a parsed PojavLauncher / Amethyst / Mojo (`mControlDataList`),
/// Zalith Launcher (`layers`), or Fold Craft Launcher (`viewGroups` / `buttonList`)
/// JSON value into active Horizon `Settings` (`ControlStyle::GoyControl`).
pub fn convert_java_control_json(value: &Value, file_name: &str) -> Option<Result<Settings, String>> {
    if value.get("mControlDataList").is_some() {
        Some(import_pojav(value, file_name))
    } else if value.get("layers").is_some() {
        Some(import_zalith(value, file_name))
    } else if value.get("viewGroups").is_some() || value.get("buttonList").is_some() {
        Some(import_fcl(value, file_name))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_pojav_v4_controls_with_dynamic_expressions_and_multi_keys() {
        let json = serde_json::json!({
            "scaledAt": 100.0,
            "mControlDataList": [
                {
                    "name": "GUI",
                    "keycodes": [-2, 0, 0, 0],
                    "dynamicX": "${margin}",
                    "dynamicY": "${margin}",
                    "width": 50.0,
                    "height": 30.0,
                    "opacity": 0.9,
                    "bgColor": 1291845632,
                    "strokeColor": -1,
                    "strokeWidth": 1.5,
                    "cornerRadius": 10.0,
                    "isToggle": false,
                    "isSwipeable": false,
                    "passThruEnabled": false,
                    "isHideable": false
                },
                {
                    "name": "PRI",
                    "keycodes": [-3, 340, 0, 0],
                    "dynamicX": "${right} - ${margin}",
                    "dynamicY": "0.5 * ${screen_height} - 0.5 * ${height}",
                    "width": 55.0,
                    "height": 55.0,
                    "opacity": 1.0,
                    "isToggle": false,
                    "isSwipeable": false,
                    "passThruEnabled": true,
                    "isHideable": true
                }
            ],
            "mJoystickDataList": []
        });
        let s = convert_java_control_json(&json, "default.json").unwrap().unwrap();
        assert_eq!(s.control_style, ControlStyle::GoyControl);
        assert_eq!(s.custom_buttons.len(), 2);
        assert_eq!(s.custom_buttons[0].key, "action:horizon.toggle_controls");
        assert!(s.custom_buttons[0].always_visible);
        assert_eq!(s.custom_buttons[1].key, "key.mouse.left");
        assert_eq!(s.custom_buttons[1].second_key, "key.keyboard.left.shift");
        assert!(s.custom_buttons[1].layouts[0].passthrough);
        assert!(s.custom_buttons[1].layouts[0].center[0] > 0.85);
    }

    #[test]
    fn converts_pojav_v9_with_px_preferred_scale_and_android_keycodes() {
        let json = serde_json::json!({
            "scaledAt": 100.0,
            "version": 9,
            "mControlDataList": [
                {
                    "name": "GUI",
                    "keycodes": [-2, 0, 0, 0],
                    "dynamicX": "0.032101654 * ${screen_width}",
                    "dynamicY": "0.6573152 * ${screen_height} - ${height}",
                    "width": 49.09,
                    "height": 47.64,
                    "cornerRadius": 60.0
                },
                {
                    "name": "W",
                    "keycodes": [51, 0, 0, 0],
                    "dynamicX": "0.16781586 * ${screen_width} - (px(54.909092) / 100.0 * ${preferred_scale}) - ${margin} + (px(54.909092) / 100.0 * ${preferred_scale}) + ${margin} - (px(54.909092) / 100.0 * ${preferred_scale}) - ${margin}",
                    "dynamicY": "0.83166593 * ${screen_height} - (px(54.909092) /100.0 * ${preferred_scale}) - ${height} - ${margin}",
                    "width": 54.909092,
                    "height": 54.909092,
                    "cornerRadius": 60.0
                },
                {
                    "name": "S",
                    "keycodes": [47, 0, 0, 0],
                    "dynamicX": "0.16781586 * ${screen_width} - (px(54.909092) / 100.0 * ${preferred_scale}) - ${margin} + (px(54.909092) / 100.0 * ${preferred_scale}) + ${margin} - (px(54.909092) / 100.0 * ${preferred_scale}) - ${margin}",
                    "dynamicY": "0.83166593 * ${screen_height} - (px(54.909092) /100.0 * ${preferred_scale}) + (px(54.909092) /100.0 * ${preferred_scale}) + ${margin}",
                    "width": 54.909092,
                    "height": 54.909092,
                    "cornerRadius": 60.0
                },
                {
                    "name": "A",
                    "keycodes": [29, 0, 0, 0],
                    "dynamicX": "0.16781586 * ${screen_width} - (px(54.909092) / 100.0 * ${preferred_scale}) - ${margin} + (px(54.909092) / 100.0 * ${preferred_scale}) + ${margin} - (px(54.909092) / 100.0 * ${preferred_scale}) - ${margin} - ${width} - ${margin}",
                    "dynamicY": "0.83166593 * ${screen_height} - (px(54.909092) /100.0 * ${preferred_scale})",
                    "width": 54.909092,
                    "height": 54.909092,
                    "cornerRadius": 60.0
                },
                {
                    "name": "D",
                    "keycodes": [32, 0, 0, 0],
                    "dynamicX": "0.16781586 * ${screen_width} - (px(54.909092) / 100.0 * ${preferred_scale}) - ${margin} + (px(54.909092) / 100.0 * ${preferred_scale}) + ${margin}",
                    "dynamicY": "0.83166593 * ${screen_height} - (px(54.909092) /100.0 * ${preferred_scale})",
                    "width": 54.909092,
                    "height": 54.909092,
                    "cornerRadius": 60.0
                },
                {
                    "name": "Debug",
                    "keycodes": [133, 0, 0, 0],
                    "dynamicX": "0.1549465 * ${screen_width} + (px(56.80007) / 100.0 * ${preferred_scale}) + ${margin}",
                    "dynamicY": "0.037314635 * ${screen_height}",
                    "width": 57.127132,
                    "height": 24.90909,
                    "cornerRadius": 53.0
                },
                {
                    "name": "SPEAR",
                    "keycodes": [14, -3, 0, 0],
                    "dynamicX": "0.97803223 * ${screen_width} - ${width}",
                    "dynamicY": "0.07106572 * ${screen_height}",
                    "width": 105.29119,
                    "height": 95.21839,
                    "cornerRadius": 60.0
                }
            ]
        });
        let s = convert_java_control_json(&json, "custom.json").unwrap().unwrap();
        assert_eq!(s.custom_buttons[0].key, "action:horizon.toggle_controls");
        assert!(s.custom_buttons[0].always_visible);
        assert_eq!(s.custom_buttons[1].key, "key.keyboard.w");
        assert_eq!(s.custom_buttons[2].key, "key.keyboard.s");
        assert_eq!(s.custom_buttons[3].key, "key.keyboard.a");
        assert_eq!(s.custom_buttons[4].key, "key.keyboard.d");
        assert_eq!(s.custom_buttons[5].key, "key.keyboard.f3");
        assert_eq!(s.custom_buttons[6].key, "key.keyboard.7");
        assert_eq!(s.custom_buttons[6].second_key, "key.mouse.left");

        let w_pos = s.custom_buttons[1].layouts[0].center;
        let s_pos = s.custom_buttons[2].layouts[0].center;
        let a_pos = s.custom_buttons[3].layouts[0].center;
        let d_pos = s.custom_buttons[4].layouts[0].center;
        // W and S align vertically on X, W is well above S on Y, A is left of W, D is right of W.
        assert!((w_pos[0] - s_pos[0]).abs() < 0.005);
        assert!(w_pos[1] < s_pos[1] - 0.15);
        assert!(a_pos[0] < w_pos[0] - 0.05);
        assert!(d_pos[0] > w_pos[0] + 0.05);
        assert!((a_pos[1] - d_pos[1]).abs() < 0.005);
        assert!(s.custom_buttons[5].layouts[0].center[0] > 0.18);
    }

    #[test]
    fn converts_zalith_v11_layers_styles_and_switch_layer() {
        let json = serde_json::json!({
            "info": {
                "name": { "default": "PvP Layout" },
                "description": { "default": "Imported PvP controls" }
            },
            "layers": [
                {
                    "name": "gui",
                    "uuid": "layer_gui",
                    "hide": false,
                    "visibilityType": "always",
                    "normalButtons": [
                        {
                            "text": { "default": "GUI" },
                            "position": { "x": 10000, "y": 114 },
                            "buttonSize": {
                                "type": "percentage",
                                "widthPercentage": 2289,
                                "heightPercentage": 1100,
                                "widthReference": "screen_height",
                                "heightReference": "screen_height"
                            },
                            "buttonStyle": "st1",
                            "visibilityType": "always",
                            "clickEvents": [
                                { "type": "switch_layer", "key": "layer_a" },
                                { "type": "switch_layer", "key": "layer_b" }
                            ],
                            "isSwipple": false,
                            "isPenetrable": false,
                            "isToggleable": false
                        }
                    ]
                },
                {
                    "name": "combat",
                    "uuid": "layer_a",
                    "hide": true,
                    "visibilityType": "in_game",
                    "normalButtons": [
                        {
                            "text": { "default": "Jump" },
                            "position": { "x": 9000, "y": 8000 },
                            "buttonSize": {
                                "type": "percentage",
                                "widthPercentage": 1200,
                                "heightPercentage": 1200,
                                "widthReference": "screen_height",
                                "heightReference": "screen_height"
                            },
                            "clickEvents": [
                                { "type": "key", "key": "GLFW_KEY_SPACE" }
                            ],
                            "isSwipple": true,
                            "isPenetrable": true,
                            "isToggleable": false
                        }
                    ]
                }
            ],
            "styles": [
                {
                    "uuid": "st1",
                    "darkStyle": {
                        "alpha": 0.85,
                        "backgroundColor": 2147483648u64,
                        "pressedBackgroundColor": 3012069512u64,
                        "borderColor": 4294967295u64,
                        "pressedBorderColor": 4294967295u64,
                        "borderWidth": 2,
                        "borderRadius": { "topStart": 40.0 }
                    }
                }
            ],
            "editorVersion": 11
        });
        let mut s = convert_java_control_json(&json, "mb0v28Uu.json").unwrap().unwrap();
        s.sanitize();
        assert_eq!(s.preset_name, "PvP Layout");
        assert_eq!(s.button_visibility[Control::Move as usize], 2);
        assert!(!s.layouts[0][Control::Move as usize].visible_in_game);
        assert_eq!(s.custom_buttons.len(), 2);
        // Layers are reversed so layers[0] renders/hits on top (at the end of custom_buttons).
        assert_eq!(s.custom_buttons[1].key, "action:horizon.switch_layer:layer_a,layer_b");
        assert!(!s.custom_buttons[1].hidden);
        assert!(s.custom_buttons[0].hidden);
        assert_eq!(s.custom_buttons[0].layer_id, "layer_a");
        assert_eq!(s.custom_buttons[0].key, "key.keyboard.space");
        assert!(s.custom_buttons[0].layouts[0].swipeable);
        assert!(s.custom_buttons[0].layouts[0].passthrough);
    }

    #[test]
    fn suppresses_joystick_when_absent_and_enables_when_present() {
        let buttons: Vec<Value> = (0..40)
            .map(|i| {
                serde_json::json!({
                    "name": format!("B{i}"),
                    "keycodes": [65 + (i % 26)],
                    "dynamicX": "100",
                    "dynamicY": "100",
                    "width": 40.0,
                    "height": 40.0,
                    "cornerRadius": 12.0
                })
            })
            .collect();
        let json = serde_json::json!({
            "scaledAt": 100.0,
            "mJoystickDataList": [{ "dynamicX": "50", "dynamicY": "50", "width": 100.0, "height": 100.0 }],
            "mControlDataList": buttons
        });
        let mut s = convert_java_control_json(&json, "big.json").unwrap().unwrap();
        s.sanitize();
        assert_eq!(s.custom_buttons.len(), 40);
        assert_eq!(s.button_visibility[Control::Move as usize], 0);
        assert!(s.layouts[0][Control::Move as usize].visible_in_game);
        assert_eq!(s.custom_buttons[0].layouts[0].shape, 0);
        assert_eq!(s.custom_buttons[39].key, "key.keyboard.n");
    }

    #[test]
    fn converts_fcl_view_groups() {
        let json = serde_json::json!({
            "controllerVersion": 2,
            "viewGroups": [
                {
                    "name": "main",
                    "visibility": "VISIBLE",
                    "viewData": {
                        "buttonList": [
                            {
                                "baseInfo": {
                                    "text": "Jump",
                                    "xPosition": 880,
                                    "yPosition": 750,
                                    "sizeWidth": 10,
                                    "sizeHeight": 15,
                                    "visibilityType": "ALWAYS"
                                },
                                "event": {
                                    "pressEvent": {
                                        "outputKeycodes": [32],
                                        "pointerFollow": false,
                                        "autoKeep": false,
                                        "movable": true
                                    }
                                }
                            }
                        ],
                        "directionList": []
                    }
                }
            ]
        });
        let s = convert_java_control_json(&json, "fcl.json").unwrap().unwrap();
        assert_eq!(s.custom_buttons.len(), 1);
        assert_eq!(s.custom_buttons[0].key, "key.keyboard.space");
        assert!(s.custom_buttons[0].layouts[0].swipeable);
        assert!(s.custom_buttons[0].layouts[0].center[0] > 0.8);
    }

    #[test]
    fn adjacent_imported_buttons_stay_flush_across_aspect_ratios() {
        let json = serde_json::json!({
            "version": 9,
            "scaledAt": 80,
            "mControlDataList": [
                {
                    "name": "B1",
                    "keycodes": [4],
                    "dynamicX": "0",
                    "dynamicY": "0",
                    "width": 50.0,
                    "height": 50.0
                },
                {
                    "name": "B2",
                    "keycodes": [-10],
                    "dynamicX": "50 * ${dp}",
                    "dynamicY": "0",
                    "width": 50.0,
                    "height": 50.0
                },
                {
                    "name": "ThinStrip",
                    "keycodes": [-3],
                    "dynamicX": "0",
                    "dynamicY": "0.5 * ${screen_height}",
                    "width": 202.6,
                    "height": 3.9
                }
            ],
            "mJoystickDataList": [
                {
                    "dynamicX": "0",
                    "dynamicY": "0",
                    "width": 1.9,
                    "height": 1.9,
                    "displayInGame": false
                },
                {
                    "dynamicX": "10 * ${dp}",
                    "dynamicY": "${bottom} - 10 * ${dp}",
                    "width": 180.0,
                    "height": 180.0,
                    "displayInGame": true,
                    "absolute": false,
                    "opacity": 0.45
                }
            ]
        });
        let s = convert_java_control_json(&json, "Spear-Mace.json").unwrap().unwrap();
        assert_eq!(s.custom_buttons[0].key, "key.keyboard.escape");
        assert_eq!(s.custom_buttons[1].key, "action:horizon.toggle_keyboard");
        assert!(s.dynamic_joystick);
        assert!((s.layouts[0][Control::Move as usize].opacity - 0.45).abs() < 1e-4);
        assert!(s.layouts[0][Control::Move as usize].size > 0.25);

        for aspect in [16.0 / 9.0, 2.0, 20.0 / 9.0, 21.0 / 9.0] {
            let r1 = s.custom_buttons[0].layouts[0].rect(aspect);
            let r2 = s.custom_buttons[1].layouts[0].rect(aspect);
            assert!(
                ((r1[0] + r1[2]) - r2[0]).abs() < 1e-4,
                "gap at aspect {aspect}: r1_right={} r2_left={}",
                r1[0] + r1[2],
                r2[0]
            );
            let thin = s.custom_buttons[2].layouts[0].rect(aspect);
            assert!(
                thin[3] < 0.025,
                "thin strip inflated at aspect {aspect}: h={}",
                thin[3]
            );
        }
    }

    #[test]
    fn preserves_all_four_event_slots_including_duplicates_and_sparse_slots() {
        let pojav = serde_json::json!({
            "version": 9,
            "scaledAt": 100.0,
            "mControlDataList": [
                {
                    "name": "Macro",
                    "keycodes": [9, -4, -4, 8],
                    "dynamicX": "100",
                    "dynamicY": "100",
                    "width": 50.0,
                    "height": 50.0
                },
                {
                    "name": "Sparse",
                    "keycodes": [51, 0, 32, 0],
                    "dynamicX": "200",
                    "dynamicY": "100",
                    "width": 50.0,
                    "height": 50.0
                }
            ]
        });
        let s = convert_java_control_json(&pojav, "four_slots.json").unwrap().unwrap();
        assert_eq!(s.custom_buttons[0].key, "key.keyboard.2");
        assert_eq!(s.custom_buttons[0].second_key, "key.mouse.right");
        assert_eq!(s.custom_buttons[0].third_key, "key.mouse.right");
        assert_eq!(s.custom_buttons[0].fourth_key, "key.keyboard.1");
        assert_eq!(s.custom_buttons[0].active_tokens().len(), 4);

        assert_eq!(s.custom_buttons[1].key, "key.keyboard.w");
        assert_eq!(s.custom_buttons[1].second_key, "key.keyboard.unknown");
        assert_eq!(s.custom_buttons[1].third_key, "key.keyboard.d");
        assert_eq!(s.custom_buttons[1].fourth_key, "");
        assert_eq!(
            s.custom_buttons[1].active_tokens(),
            vec!["key.keyboard.w", "key.keyboard.d"]
        );
    }
}
