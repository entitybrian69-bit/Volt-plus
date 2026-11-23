//! Full converter from PojavLauncher (`mControlDataList`) and Zalith Launcher
//! (`layers` / `editorVersion`) JSON control layouts into active Horizon
//! `TouchControls` (`Settings` with `ControlStyle::GoyControl`).
use std::collections::HashMap;

use horizon_client_input::java_controls::{ControlEvent, pojav_code, zalith_event};
use serde_json::Value;

use super::custom::CustomButton;
use super::layout::{Control, ControlStyle, Placement, Settings};

const REF_W: f32 = 854.0;
const REF_H: f32 = 480.0;
const REF_ASPECT: f32 = REF_W / REF_H;

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

fn build_placement(
    cx_norm: f32,
    cy_norm: f32,
    w_norm: f32,
    h_norm: f32,
    opacity: f32,
    bg_abgr: u32,
    stroke_abgr: u32,
    stroke_size: f32,
    roundness: f32,
    on_click_abgr: u32,
    on_click_stroke_abgr: u32,
    passthrough: bool,
    toggleable: bool,
    swipeable: bool,
    visible_in_game: bool,
    visible_in_menu: bool,
) -> Placement {
    let w_n = w_norm.clamp(0.02, 0.75);
    let h_n = h_norm.clamp(0.02, 0.75);
    // Solve `size`, `width_ratio`, `height_ratio` inside Placement::sanitize bounds
    // (`size` in 0.045..=0.45, `width_ratio`/`height_ratio` in 0.4..=3.2) so `rect(REF_ASPECT)`
    // reproduces `(w_n, h_n)` accurately.
    let target_w_units = w_n * REF_ASPECT;
    let size = h_n.max(target_w_units / 3.2).clamp(0.045, 0.45);
    let width_ratio = (target_w_units / size).clamp(0.4, 3.2);
    let height_ratio = (h_n / size).clamp(0.4, 3.2);

    let mut p = Placement::new(cx_norm.clamp(0.0, 1.0), cy_norm.clamp(0.0, 1.0), size);
    p.use_goy_style = true;
    p.width_ratio = width_ratio;
    p.height_ratio = height_ratio;
    p.opacity = opacity.clamp(0.0, 1.0);
    p.shape = if roundness > 0.5 { 0 } else { 1 };
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
    // whenever controls are imported from PojavLauncher or Zalith Launcher so the
    // imported layout stands alone without an overlapping touch joystick.
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

// ---------------------------------------------------------------------------
// PojavLauncher v4 importer (`mControlDataList` + `mDrawerDataList`)
// ---------------------------------------------------------------------------

fn eval_pojav_expr(expr: &str, vars: &[(&str, f32)]) -> Option<f32> {
    let mut expanded = expr.trim().to_owned();
    for (k, v) in vars {
        let needle = format!("${{{k}}}");
        expanded = expanded.replace(&needle, &format!("{v:.4}"));
    }
    let tokens = tokenize_expr(&expanded)?;
    let mut pos = 0;
    parse_expr_add_sub(&tokens, &mut pos)
}

#[derive(Clone, Debug)]
enum Tok {
    Num(f32),
    Op(char),
    LParen,
    RParen,
}

fn tokenize_expr(s: &str) -> Option<Vec<Tok>> {
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
            let num: f32 = s[start..i].parse().ok()?;
            out.push(Tok::Num(num));
            continue;
        }
        match ch {
            '+' | '-' | '*' | '/' => {
                out.push(Tok::Op(ch));
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

fn parse_expr_add_sub(toks: &[Tok], pos: &mut usize) -> Option<f32> {
    let mut val = parse_expr_mul_div(toks, pos)?;
    while let Some(Tok::Op(op @ ('+' | '-'))) = toks.get(*pos) {
        *pos += 1;
        let rhs = parse_expr_mul_div(toks, pos)?;
        val = if *op == '+' { val + rhs } else { val - rhs };
    }
    Some(val)
}

fn parse_expr_mul_div(toks: &[Tok], pos: &mut usize) -> Option<f32> {
    let mut val = parse_expr_unary(toks, pos)?;
    while let Some(Tok::Op(op @ ('*' | '/'))) = toks.get(*pos) {
        *pos += 1;
        let rhs = parse_expr_unary(toks, pos)?;
        val = if *op == '*' {
            val * rhs
        } else if rhs.abs() > 1e-6 {
            val / rhs
        } else {
            0.0
        };
    }
    Some(val)
}

fn parse_expr_unary(toks: &[Tok], pos: &mut usize) -> Option<f32> {
    if let Some(Tok::Op('-')) = toks.get(*pos) {
        *pos += 1;
        return Some(-parse_expr_unary(toks, pos)?);
    }
    if let Some(Tok::Op('+')) = toks.get(*pos) {
        *pos += 1;
        return parse_expr_unary(toks, pos);
    }
    match toks.get(*pos)? {
        Tok::Num(n) => {
            *pos += 1;
            Some(*n)
        }
        Tok::LParen => {
            *pos += 1;
            let v = parse_expr_add_sub(toks, pos)?;
            if matches!(toks.get(*pos), Some(Tok::RParen)) {
                *pos += 1;
            }
            Some(v)
        }
        _ => None,
    }
}

fn push_pojav_button(btn_val: &Value, scaled_at: f32, out: &mut Vec<CustomButton>) {
    let scale = if scaled_at > 1.0 { scaled_at / 100.0 } else { 1.0 };
    // Pojav uses ~1dp ≈ 1.45px on a 854x480 reference phone viewport, scaled by scaledAt.
    let dp = 1.45 * scale;
    let w_px = (btn_val["width"].as_f64().unwrap_or(50.0) as f32 * dp).max(12.0);
    let h_px = (btn_val["height"].as_f64().unwrap_or(50.0) as f32 * dp).max(12.0);
    let margin = 2.0 * dp;
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
        ("preferred_scale", scale),
    ];

    let x_px = btn_val["dynamicX"]
        .as_str()
        .and_then(|e| eval_pojav_expr(e, &vars))
        .unwrap_or(0.0);
    let y_px = btn_val["dynamicY"]
        .as_str()
        .and_then(|e| eval_pojav_expr(e, &vars))
        .unwrap_or(0.0);

    let cx_norm = ((x_px + w_px * 0.5) / REF_W).clamp(0.02, 0.98);
    let cy_norm = ((y_px + h_px * 0.5) / REF_H).clamp(0.02, 0.98);
    let w_norm = (w_px / REF_W).clamp(0.02, 0.7);
    let h_norm = (h_px / REF_H).clamp(0.02, 0.7);

    let mut tokens: Vec<String> = Vec::new();
    if let Some(arr) = btn_val["keycodes"].as_array() {
        for code in arr {
            if let Some(c) = code.as_i64()
                && c != 0
                && let Some(ev) = pojav_code(c as i32)
                && let Some(tok) = token_from_event(ev, "")
                && CustomButton::valid_token(&tok)
                && !tokens.contains(&tok)
            {
                tokens.push(tok);
            }
        }
    }

    let name = btn_val["name"].as_str().unwrap_or("").trim().to_owned();
    let opacity = btn_val["opacity"].as_f64().unwrap_or(1.0) as f32;
    let bg = parse_color_u32(btn_val.get("bgColor"), 0x4D00_0000);
    let stroke = parse_color_u32(btn_val.get("strokeColor"), 0xFFFFFFFF);
    let stroke_w = btn_val["strokeWidth"].as_f64().unwrap_or(1.2) as f32;
    let radius = btn_val["cornerRadius"].as_f64().unwrap_or(8.0) as f32;
    let is_toggle = btn_val["isToggle"].as_bool().unwrap_or(false);
    let is_swipe = btn_val["isSwipeable"].as_bool().unwrap_or(false);
    let passthrough = btn_val["passThruEnabled"].as_bool().unwrap_or(false);
    let hideable = btn_val["isHideable"].as_bool().unwrap_or(true);
    let is_menu_relevant = !hideable
        || tokens
            .iter()
            .any(|t| matches!(t.as_str(), "key.mouse.left" | "key.mouse.right" | "mouse.virtual.toggle" | "action:horizon.toggle_keyboard" | "action:horizon.open_menu"));

    let placement = build_placement(
        cx_norm,
        cy_norm,
        w_norm,
        h_norm,
        opacity,
        bg,
        stroke,
        stroke_w,
        radius,
        argb_to_abgr(0x997C5CFF),
        argb_to_abgr(0xFFEADDFF),
        passthrough,
        is_toggle,
        is_swipe,
        true,
        is_menu_relevant,
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
    btn.always_visible = !hideable;
    btn.layouts = [placement; 3];
    out.push(btn);
}

fn import_pojav(value: &Value, file_name: &str) -> Result<Settings, String> {
    let scaled_at = value["scaledAt"].as_f64().unwrap_or(100.0) as f32;
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

    let has_joystick = value["mJoystickDataList"]
        .as_array()
        .is_some_and(|a| !a.is_empty());
    let mut s = base_java_settings(
        preset_name,
        "Imported PojavLauncher control layout".to_owned(),
    );

    if let Some(list) = value["mControlDataList"].as_array() {
        for item in list {
            push_pojav_button(item, scaled_at, &mut s.custom_buttons);
        }
    }
    if let Some(drawers) = value["mDrawerDataList"].as_array() {
        for drawer in drawers {
            if let Some(props) = drawer.get("properties") {
                push_pojav_button(props, scaled_at, &mut s.custom_buttons);
            }
            if let Some(children) = drawer["buttonProperties"].as_array() {
                for child in children {
                    push_pojav_button(child, scaled_at, &mut s.custom_buttons);
                }
            }
        }
    }
    if s.custom_buttons.is_empty() && !has_joystick {
        return Err("Pojav control layout contained no buttons".into());
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// Zalith Launcher importer (`layers` + `styles` + `info`)
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
        let visual = st
            .get("darkStyle")
            .filter(|v| v.is_object())
            .or_else(|| st.get("lightStyle").filter(|v| v.is_object()));
        let Some(vis) = visual else {
            continue;
        };
        let border_r = vis["borderRadius"]["topStart"].as_f64().unwrap_or(0.0) as f32;
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

fn import_zalith(value: &Value, file_name: &str) -> Result<Settings, String> {
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
    let has_joystick = value.get("joystick").is_some_and(|v| v.is_object());
    let mut s = base_java_settings(
        preset_name.chars().take(48).collect(),
        preset_desc.chars().take(120).collect(),
    );

    let Some(layers) = value["layers"].as_array() else {
        return Err("Zalith control layout has no layers".into());
    };
    for layer in layers {
        let layer_uuid = layer["uuid"].as_str().unwrap_or("").to_owned();
        let layer_hide = layer["hide"].as_bool().unwrap_or(false);
        let layer_vis = layer["visibilityType"].as_str().unwrap_or("always");
        let Some(buttons) = layer["normalButtons"].as_array() else {
            continue;
        };
        for b in buttons {
            let size_obj = &b["buttonSize"];
            let size_kind = size_obj["type"].as_str().unwrap_or("percentage");
            let (w_norm, h_norm) = if size_kind == "dp" {
                let w_dp = size_obj["widthDp"].as_f64().unwrap_or(50.0) as f32;
                let h_dp = size_obj["heightDp"].as_f64().unwrap_or(50.0) as f32;
                ((w_dp * 1.45) / REF_W, (h_dp * 1.45) / REF_H)
            } else {
                let wp = size_obj["widthPercentage"].as_f64().unwrap_or(1000.0) as f32 / 10000.0;
                let hp = size_obj["heightPercentage"].as_f64().unwrap_or(1000.0) as f32 / 10000.0;
                let w_ref = size_obj["widthReference"].as_str().unwrap_or("screen_width");
                let h_ref = size_obj["heightReference"].as_str().unwrap_or("screen_height");
                let wn = if w_ref == "screen_height" { wp / REF_ASPECT } else { wp };
                let hn = if h_ref == "screen_width" { hp * REF_ASPECT } else { hp };
                (wn, hn)
            };
            let w_norm = w_norm.clamp(0.02, 0.75);
            let h_norm = h_norm.clamp(0.02, 0.75);

            // Zalith position x, y are 0..10000 normalized along the movable range `(1.0 - size)`.
            let px = (b["position"]["x"].as_f64().unwrap_or(5000.0) as f32 / 10000.0).clamp(0.0, 1.0);
            let py = (b["position"]["y"].as_f64().unwrap_or(5000.0) as f32 / 10000.0).clamp(0.0, 1.0);
            let cx_norm = px * (1.0 - w_norm) + w_norm * 0.5;
            let cy_norm = py * (1.0 - h_norm) + h_norm * 0.5;

            let mut tokens: Vec<String> = Vec::new();
            let mut switch_layers: Vec<String> = Vec::new();
            if let Some(events) = b["clickEvents"].as_array() {
                for ev_val in events {
                    let kind = ev_val["type"].as_str().unwrap_or("");
                    let key = ev_val["key"].as_str().unwrap_or("");
                    if kind == "switch_layer" {
                        if !key.is_empty() && !switch_layers.iter().any(|s| s == key) {
                            switch_layers.push(key.to_owned());
                        }
                        continue;
                    }
                    if let Some(ev) = zalith_event(kind, key)
                        && let Some(tok) = token_from_event(ev, key)
                        && CustomButton::valid_token(&tok)
                        && !tokens.contains(&tok)
                    {
                        tokens.push(tok);
                    }
                }
            }
            if !switch_layers.is_empty() {
                let combined = format!("action:horizon.switch_layer:{}", switch_layers.join(","));
                tokens.push(combined);
            }

            let style_id = b["buttonStyle"].as_str().unwrap_or("");
            let style = styles.get(style_id).copied().unwrap_or_default();
            let layer_vis_pair = match layer_vis {
                "in_game" => (true, false),
                "in_menu" => (false, true),
                _ => (true, true),
            };
            let btn_vis_pair = match b["visibilityType"].as_str().unwrap_or(layer_vis) {
                "in_game" => (true, false),
                "in_menu" => (false, true),
                _ => (true, true),
            };
            let in_game = layer_vis_pair.0 && btn_vis_pair.0;
            let in_menu = layer_vis_pair.1 && btn_vis_pair.1;
            let is_swipe = b["isSwipple"].as_bool().unwrap_or(false);
            let passthrough = b["isPenetrable"].as_bool().unwrap_or(false);
            let is_toggle = b["isToggleable"].as_bool().unwrap_or(false);
            let label = pick_zalith_text(&b["text"]);

            let placement = build_placement(
                cx_norm,
                cy_norm,
                w_norm,
                h_norm,
                style.alpha,
                style.bg_abgr,
                style.stroke_abgr,
                style.border_width,
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

/// Converts a parsed PojavLauncher (`mControlDataList`) or Zalith Launcher (`layers`)
/// JSON value into active Horizon `Settings` (`ControlStyle::GoyControl`).
pub fn convert_java_control_json(value: &Value, file_name: &str) -> Option<Result<Settings, String>> {
    if value.get("mControlDataList").is_some() {
        Some(import_pojav(value, file_name))
    } else if value.get("layers").is_some() {
        Some(import_zalith(value, file_name))
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
        assert_eq!(s.custom_buttons[0].key, "action:horizon.switch_layer:layer_a,layer_b");
        assert!(!s.custom_buttons[0].hidden);
        assert!(s.custom_buttons[1].hidden);
        assert_eq!(s.custom_buttons[1].layer_id, "layer_a");
        assert_eq!(s.custom_buttons[1].key, "key.keyboard.space");
        assert!(s.custom_buttons[1].layouts[0].swipeable);
        assert!(s.custom_buttons[1].layouts[0].passthrough);
    }

    #[test]
    fn suppresses_joystick_and_preserves_over_32_buttons_on_sanitize() {
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
            "mJoystickDataList": [{ "dynamicX": "50", "dynamicY": "50" }],
            "mControlDataList": buttons
        });
        let mut s = convert_java_control_json(&json, "big.json").unwrap().unwrap();
        s.sanitize();
        assert_eq!(s.custom_buttons.len(), 40);
        assert_eq!(s.button_visibility[Control::Move as usize], 2);
        assert!(!s.layouts[0][Control::Move as usize].visible_in_game);
        assert_eq!(s.custom_buttons[0].layouts[0].shape, 0);
        assert_eq!(s.custom_buttons[39].key, "key.keyboard.n");
    }
}
