//! UI metadata, Modrinth icon fetching, and per-mod settings rows/actions
//! for all 82 built-in Volt+ Extras mods inside the Control Hub.
use crate::extras::{
    VOLT_MODS, VoltExtras, VoltModCategory, VoltModKeybind, VoltQuickAction, volt_mod_enabled,
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub struct SettingRow {
    pub title: &'static str,
    pub sub: &'static str,
    pub toggle: Option<bool>,
    pub badge: Option<String>,
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn modrinth_icon_url(key: &'static str) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, String>>> = OnceLock::new();
    static STARTED: OnceLock<()> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if STARTED.set(()).is_ok() {
        for chunk in VOLT_MODS.chunks(21) {
            std::thread::spawn(move || {
                let agent = ureq::AgentBuilder::new()
                    .timeout(std::time::Duration::from_secs(6))
                    .build();
                let ua = "VoltPlus/1.0 (https://github.com/entitybrian69-bit/volt-plus)";
                for m in chunk {
                    let mut found: Option<String> = None;
                    for &slug in m.modrinth_slugs {
                        let url = format!("https://api.modrinth.com/v2/project/{slug}");
                        if let Ok(resp) = agent.get(&url).set("User-Agent", ua).call()
                            && let Ok(val) = resp.into_json::<serde_json::Value>()
                            && let Some(icon) = val.get("icon_url").and_then(|v| v.as_str())
                            && icon.starts_with("https://")
                        {
                            found = Some(icon.to_owned());
                            break;
                        }
                    }
                    if found.is_none() && !m.search_query.is_empty() {
                        let q = percent_encode(m.search_query);
                        let url = format!("https://api.modrinth.com/v2/search?query={q}&limit=1");
                        if let Ok(resp) = agent.get(&url).set("User-Agent", ua).call()
                            && let Ok(val) = resp.into_json::<serde_json::Value>()
                            && let Some(icon) = val
                                .get("hits")
                                .and_then(|h| h.get(0))
                                .and_then(|h| h.get("icon_url"))
                                .and_then(|v| v.as_str())
                            && icon.starts_with("https://")
                        {
                            found = Some(icon.to_owned());
                        }
                    }
                    if let Some(icon) = found
                        && let Ok(mut guard) = cache.lock()
                    {
                        guard.insert(m.key, icon);
                    }
                }
            });
        }
    }
    cache.lock().ok().and_then(|g| g.get(key).cloned())
}

pub fn filtered_mod_indices(cat_idx: u8) -> Vec<u8> {
    let cat = VoltModCategory::TABS
        .get(usize::from(cat_idx))
        .copied()
        .unwrap_or(VoltModCategory::All);
    VOLT_MODS
        .iter()
        .filter(|m| cat == VoltModCategory::All || m.category == cat)
        .map(|m| m.id)
        .collect()
}

pub fn filtered_mod_indices_with(
    v: &VoltExtras,
    cat_idx: u8,
    query: &str,
    filter_mode: u8,
    sort_enabled_first: bool,
) -> Vec<u8> {
    let cat = VoltModCategory::TABS
        .get(usize::from(cat_idx))
        .copied()
        .unwrap_or(VoltModCategory::All);
    let q = query.trim().to_ascii_lowercase();
    let mut out: Vec<u8> = VOLT_MODS
        .iter()
        .filter(|m| cat == VoltModCategory::All || m.category == cat)
        .filter(|m| match filter_mode {
            1 => volt_mod_enabled(v, m.id),
            2 => !volt_mod_enabled(v, m.id),
            _ => true,
        })
        .filter(|m| {
            q.is_empty()
                || m.title.to_ascii_lowercase().contains(&q)
                || m.summary.to_ascii_lowercase().contains(&q)
                || m.key.to_ascii_lowercase().contains(&q)
        })
        .map(|m| m.id)
        .collect();
    if sort_enabled_first {
        out.sort_by_key(|&id| (!volt_mod_enabled(v, id), id));
    }
    out
}

pub fn active_mod_count(v: &VoltExtras) -> usize {
    (0..VOLT_MODS.len())
        .filter(|&i| volt_mod_enabled(v, i as u8))
        .count()
}

fn r_tog(title: &'static str, sub: &'static str, val: bool) -> SettingRow {
    SettingRow {
        title,
        sub,
        toggle: Some(val),
        badge: None,
    }
}

fn r_val(title: &'static str, sub: &'static str, val: String) -> SettingRow {
    SettingRow {
        title,
        sub,
        toggle: None,
        badge: Some(val),
    }
}

fn format_keybind_badge(tok: &str) -> String {
    if tok.is_empty() {
        return "Not bound".to_owned();
    }
    match tok {
        "key.mouse.left" | "key.mouse.1" => "LMB".to_owned(),
        "key.mouse.right" | "key.mouse.2" => "RMB".to_owned(),
        "key.mouse.middle" | "key.mouse.3" => "MMB".to_owned(),
        "mouse:scroll_up" => "Scroll Up".to_owned(),
        "mouse:scroll_down" => "Scroll Down".to_owned(),
        _ => {
            if let Some(act) = tok.strip_prefix("action:") {
                act.trim_start_matches("key.").to_owned()
            } else {
                crate::keybinds::display_name(tok).plain()
            }
        }
    }
}

pub fn mod_setting_rows(v: &VoltExtras, idx: u8) -> Vec<SettingRow> {
    let mut rows = base_mod_setting_rows(v, idx);
    let bind = v
        .binds
        .0
        .get(usize::from(idx))
        .cloned()
        .unwrap_or_else(|| VoltModKeybind::default_for_id(idx));
    rows.push(r_val(
        "Toggle Keybind",
        "Tap to bind a key/button that turns this mod ON or OFF",
        format_keybind_badge(&bind.toggle_key),
    ));
    rows.push(r_val(
        "Use Keybind",
        "Tap to bind a key/button for this mod's active trigger",
        format_keybind_badge(&bind.use_key),
    ));
    rows.push(r_val(
        "Keybind Activation Mode",
        "Choose Tap (toggle) or Hold (active only while held down)",
        bind.mode.label().to_owned(),
    ));
    rows
}

fn base_mod_setting_rows(v: &VoltExtras, idx: u8) -> Vec<SettingRow> {
    match idx {
        0 => {
            let c = &v.appleskin;
            vec![
                r_tog("Saturation Overlay", "Gold outlines over hunger drumsticks", c.show_saturation_overlay),
                r_tog("Exhaustion Underlay", "Translucent drain progress behind hunger bar", c.show_exhaustion_underlay),
                r_tog("Held Food Preview", "Flashing hunger & saturation gain while holding food", c.show_held_food_preview),
                r_tog("Health Regen Overlay", "Predicted heart regen from eating held food", c.show_health_regen_overlay),
                r_tog("Food Value Tooltips", "Show hunger & saturation icons in item tooltips", c.show_food_tooltip),
                r_tog("Always Show Tooltip", "Show food values even without holding Shift", c.always_show_tooltip),
                r_tog("F3 Debug Food Stats", "Display saturation & exhaustion on debug screen", c.show_debug_stats),
                r_tog("Highlight Full Hunger Bar", "Outline every active food drumstick cleanly", c.highlight_full_bar),
                r_val("Preview Pulse Speed", "Animation speed for held food saturation flash", format!("{}%", c.pulse_speed_pct)),
            ]
        }
        1 => {
            let c = &v.crystal_optimizer;
            vec![
                r_tog("Instant Client Crystal Break", "Remove attacked End Crystals client-side immediately", c.instant_break),
                r_tog("Respect Weakness Effect", "Skip client removal when Weakness prevents damage", c.respect_weakness),
                r_tog("Fast Crystal & Obsidian Place", "Clear use delay while holding crystals or obsidian", c.fast_place),
                r_val("Sync Restore Timeout", "Restore crystal if server does not confirm break", format!("{} ms", c.timeout_ms)),
            ]
        }
        2 => {
            let c = &v.shield_status;
            vec![
                r_val("Enabled Color", "Color when shield is ready to block", c.enabled_color.label().to_owned()),
                r_val("Disabled Color", "Color right after an axe disables the shield", c.disabled_color.label().to_owned()),
                r_tog("Interpolate Cooldown Color", "Smoothly blend disabled -> enabled during 5s recovery", c.interpolate_colors),
                r_val("Display Mode", "Where Shield Status tint & indicator appear", c.display_mode.label().to_owned()),
                r_tog("Disable Sound Alert", "Play audio cue when shield gets axe-disabled", c.disable_sound),
            ]
        }
        3 => {
            let c = &v.uku_hud;
            let low_txt = if c.low_durability_pct == 0 {
                "Off".to_owned()
            } else {
                format!("{}%", c.low_durability_pct)
            };
            vec![
                r_tog("Show Armor HUD", "Render equipped armor & durability overlay", c.armor_enabled),
                r_tog("Show Potion HUD", "Render Uku's custom potion timer cards", c.potion_enabled),
                r_val("Armor Anchor", "Screen region for the Armor HUD panel", c.armor_anchor.label().to_owned()),
                r_val("Armor Side", "Left or right placement relative to anchor", c.armor_side.label().to_owned()),
                r_val("Panel Style", "Hotbar slice, rounded, flat, or minimal background", c.box_style.label().to_owned()),
                r_val("Durability Display", "Show bar, numeric value, percentage, or off", c.durability_display.label().to_owned()),
                r_tog("Show Offhand Slot", "Include offhand item in the Armor HUD", c.show_offhand),
                r_tog("Hide Empty Slots", "Skip armor slots with no item equipped", c.hide_empty),
                r_val("Low Durability Alert", "Red highlight threshold for damaged armor", low_txt),
                r_val("Potion HUD Position", "Corner anchor for active effect timers", c.potion_anchor.label().to_owned()),
                r_tog("Compact Potion Cards", "Slimmer icon + MM:SS cards for tight screens", c.potion_compact),
            ]
        }
        4 => {
            let c = &v.freelook;
            vec![
                r_val("Quick Action", "Toggle active Freelook camera right now", if c.active { "Active".into() } else { "Activate".into() }),
                r_tog("Toggle Mode", "Tap once to lock/unlock instead of hold", c.toggle_mode),
                r_tog("Invert Y", "Flip vertical pitch during freelook", c.invert_y),
                r_val("Sensitivity", "Freelook orbit sensitivity", format!("{}%", c.sensitivity_pct)),
                r_tog("Preserve Perspective", "Keep current 1st/3rd person view", c.preserve_perspective),
            ]
        }
        5 => {
            let c = &v.trajectories;
            vec![
                r_tog("Bow & Crossbow Arc", "Predict arrows & bolts", c.show_bow),
                r_tog("Ender Pearl Arc", "Predict pearl landing point", c.show_pearl),
                r_tog("Trident Arc", "Predict thrown trident flight", c.show_trident),
                r_tog("Wind Charge Arc", "Predict wind charge path", c.show_wind_charge),
                r_tog("Impact Box", "Highlight landing block in red", c.show_impact_box),
                r_val("Arc Color", "Trajectory line color", c.color.label().to_owned()),
            ]
        }
        6 => {
            let c = &v.health_indicators;
            vec![
                r_tog("Show Mob HP Bars", "Floating health bar above mobs", c.show_bars),
                r_tog("Damage Popups", "Floating -HP hit numbers", c.show_damage_popups),
                r_tog("Highlight Crits", "Gold popup text on critical hits", c.highlight_crits),
                r_tog("Show Hostiles", "Display bars on monster mobs", c.show_hostiles),
                r_val("Max Range", "Render distance in blocks", format!("{}m", c.max_range)),
                r_val("Bar Scale", "Width multiplier for health bars", format!("{}%", c.bar_scale)),
            ]
        }
        7 => {
            let c = &v.item_scroller;
            vec![
                r_val("Quick Offhand Totem", "Move totem from inventory to offhand now", "Swap Now".into()),
                r_tog("Auto-Refill Hotbar", "Replenish empty hotbar stacks automatically", c.auto_refill_hotbar),
                r_tog("Auto Re-Totem on Pop", "Swap next totem into offhand after pop", c.quick_offhand_totem),
                r_tog("Preserve Low Tools", "Stop using tools near breaking", c.preserve_tools),
            ]
        }
        8 => {
            let c = &v.tweakeroo;
            vec![
                r_tog("Fast Block Place", "Remove right-click block place delay", c.fast_block_place),
                r_tog("Light Level X Overlay", "Draw red/yellow X on spawnable floor blocks", c.light_overlay),
                r_tog("Auto-Tool", "Switch to best mining tool when digging", c.auto_tool),
                r_tog("Silk Touch Guard", "Warn before breaking fragile blocks", c.silk_touch_guard),
                r_val("Light Overlay Range", "Scan radius around player", format!("{}m", c.light_range)),
            ]
        }
        9 => {
            let c = &v.elytra_hud;
            vec![
                r_tog("Show Altitude (Y)", "Display Y height while gliding", c.show_altitude),
                r_tog("Show Pitch Gauge", "Display glide angle in degrees", c.show_pitch_gauge),
                r_tog("Show Fireworks Count", "Show rocket count on glide HUD", c.show_fireworks),
                r_tog("Mace Smash Damage", "Show live fall distance & smash damage", c.show_mace_damage),
            ]
        }
        10 => {
            let c = &v.waypoints;
            vec![
                r_val("Waypoint Manager", "View, recolor, share coords & delete saved waypoints", "Manage".into()),
                r_val("Add Waypoint Here", "Save a new waypoint at your current position", "+ Save".into()),
                r_val("Clear Death Markers", "Remove saved death waypoints", "Clear".into()),
                r_tog("Auto Death Waypoint", "Save marker automatically on death", c.auto_death_waypoint),
                r_tog("3D Sky Beacons", "Render vertical waypoint beams in world", c.show_beacons),
                r_tog("HUD Distance Tags", "Show waypoint names & distances on screen", c.show_hud_distances),
                r_val("Max Distance", "Maximum waypoint visibility", format!("{}m", c.max_distance)),
            ]
        }
        11 => {
            let c = &v.totem_counter;
            vec![
                r_tog("Show HUD Counter", "Show total Totems in inventory/offhand", c.show_hud_badge),
                r_tog("Low-HP Border Alert", "Flash red screen border at low HP", c.low_hp_alert),
                r_val("Low-HP Threshold", "Hearts remaining to trigger alert", format!("{} ♥", c.low_hp_hearts)),
                r_tog("Warn When Empty", "Red alert when 0 totems remain", c.warn_when_empty),
                r_val("HUD Corner", "Screen corner for badge", c.corner.label().to_owned()),
            ]
        }
        12 => {
            let c = &v.cps_combo;
            vec![
                r_tog("Show LMB CPS", "Left-click CPS counter", c.show_lmb_cps),
                r_tog("Show RMB CPS", "Right-click CPS counter", c.show_rmb_cps),
                r_tog("Show Hit Reach", "Distance in blocks of last hit", c.show_reach),
                r_tog("Show Combo Streak", "Consecutive hits on target", c.show_combo),
                r_val("HUD Corner", "Screen corner for combat pill", c.corner.label().to_owned()),
            ]
        }
        13 => {
            let c = &v.keystrokes;
            vec![
                r_tog("Show LMB / RMB Tiles", "Include mouse buttons under WASD", c.show_cps),
                r_tog("Show Spacebar", "Include jump bar tile", c.show_space),
                r_val("Background Opacity", "Tile backdrop alpha", format!("{}%", c.bg_opacity_pct)),
                r_val("HUD Corner", "Screen corner for keystrokes", c.corner.label().to_owned()),
            ]
        }
        14 => {
            let c = &v.pvp_visuals;
            vec![
                r_val("Fire Height", "Reduce on-fire screen flames", format!("{}%", c.low_fire_pct)),
                r_tog("Hide Pumpkin Overlay", "Remove carved pumpkin screen blur", c.hide_pumpkin),
                r_tog("Hide Portal Overlay", "Remove purple nether portal swirl", c.hide_portal_overlay),
                r_val("Hurt Camera Shake", "Screen tilt intensity on damage", format!("{}%", c.hurt_cam_pct)),
                r_tog("Reduced View Bobbing", "Halve camera walking bob", c.reduced_bobbing),
            ]
        }
        15 => {
            let c = &v.crosshair_tweaks;
            vec![
                r_tog("Hit Marker Flash", "Draw gold X around crosshair on hit", c.hit_marker),
                r_val("Crosshair Style", "Vanilla, Cross, Dot, or Circle", c.style.label().to_owned()),
                r_val("Crosshair Color", "Tint color for custom crosshair", c.color.label().to_owned()),
                r_val("Crosshair Size", "Arm radius in pixels", format!("{}px", c.size)),
            ]
        }
        16 => {
            let c = &v.zoom;
            vec![
                r_val("Toggle Zoom Now", "Activate or deactivate zoom immediately", if c.active { "Zoomed".into() } else { "Zoom In".into() }),
                r_val("Zoom Level", "FOV magnification factor", format!("{}x", c.zoom_level)),
                r_tog("Smooth Transition", "Smooth FOV interpolation", c.smooth),
                r_tog("Scale Sensitivity", "Slow camera turn speed while zoomed", c.scale_sensitivity),
            ]
        }
        17 => {
            let c = &v.auto_sprint;
            vec![
                r_tog("Show Sprint Badge", "Display [SPRINT ON] corner tag", c.show_indicator),
                r_tog("Sprint in Water", "Keep sprinting while swimming", c.sprint_in_water),
                r_val("HUD Corner", "Corner for sprint badge", c.corner.label().to_owned()),
            ]
        }
        18 => {
            let c = &v.fullbright;
            vec![
                r_val("Gamma Boost", "Brightness percentage", format!("{}%", c.gamma_pct)),
                r_tog("Smooth Fade", "Ease gamma changes smoothly", c.smooth_fade),
            ]
        }
        19 => {
            let c = &v.coords_hud;
            vec![
                r_tog("Show XYZ", "Display current block coordinates", c.show_xyz),
                r_tog("Show Facing", "Display cardinal compass direction", c.show_facing),
                r_tog("Show Biome", "Display current biome name", c.show_biome),
                r_tog("Nether / OW Link", "Show ÷8 / ×8 equivalent coordinates", c.show_nether_equiv),
                r_val("Copy Coords to Chat", "Print formatted coordinates into chat", "Log Now".into()),
                r_val("HUD Corner", "Corner for coords box", c.corner.label().to_owned()),
            ]
        }
        20 => {
            let c = &v.telemetry_hud;
            vec![
                r_tog("Show FPS", "Display live frames per second", c.show_fps),
                r_tog("Show Ping", "Display server latency in ms", c.show_ping),
                r_tog("Show Speed (BPS)", "Display horizontal speed in blocks/sec", c.show_bps),
                r_val("HUD Corner", "Corner for telemetry pill", c.corner.label().to_owned()),
            ]
        }
        21 => {
            let c = &v.item_counters;
            vec![
                r_tog("Count Arrows", "Show arrow stack total", c.count_arrows),
                r_tog("Count Ender Pearls", "Show pearl stack total", c.count_pearls),
                r_tog("Count Wind Charges", "Show wind charge stack total", c.count_wind_charges),
                r_tog("Count Golden Apples", "Show gapple stack total", c.count_gapples),
                r_tog("Show Zero Counts", "Display counters even when 0", c.show_zero),
            ]
        }
        22 => {
            let c = &v.clock_day_hud;
            vec![
                r_tog("Show Day Counter", "Display Day #", c.show_day),
                r_tog("24-Hour Clock", "Use 24h HH:MM instead of AM/PM", c.use_24h),
                r_tog("Show Light Level", "Show current block/sky light", c.show_light_level),
                r_val("HUD Corner", "Corner for clock pill", c.corner.label().to_owned()),
            ]
        }
        23 => {
            let c = &v.chunk_hitboxes;
            vec![
                r_tog("Chunk Borders (F3+G)", "Show chunk grid lines around player", c.chunk_borders),
                r_tog("Entity Hitboxes (F3+B)", "Draw entity AABB & eye-sight boxes", c.entity_hitboxes),
                r_tog("Hide in F1", "Hide debug boxes when HUD is hidden", c.hide_in_f1),
            ]
        }
        24 => {
            let c = &v.chat_tweaks;
            vec![
                r_tog("Message Timestamps", "Prepend [HH:MM] to incoming chat", c.timestamps),
                r_tog("24-Hour Timestamps", "Use 24-hour time format", c.use_24h),
                r_tog("Compact Duplicates", "Collapse repeated lines into [xN]", c.compact_duplicates),
                r_tog("Extended Chat History", "Keep up to 250 chat messages", c.keep_history),
            ]
        }
        25 => {
            let c = &v.freecam;
            vec![
                r_val("Toggle Freecam Now", "Enter or exit detached camera mode", if c.active { "Active".into() } else { "Enter".into() }),
                r_val("Fly Speed", "Freecam movement speed", format!("{}", c.speed)),
                r_tog("Hide First-Person Hand", "Hide hand while scouting in freecam", c.hide_hand),
            ]
        }
        26 => {
            let c = &v.radar;
            vec![
                r_tog("Highlight Chests", "3D gold boxes around chests & barrels", c.highlight_chests),
                r_tog("Highlight Spawners", "3D red boxes around spawners & vaults", c.highlight_spawners),
                r_tog("Highlight Dropped Items", "3D cyan boxes around ground items", c.highlight_items),
                r_val("Scan Range", "Radar radius in blocks", format!("{}m", c.range)),
            ]
        }
        27 => {
            let c = &v.auto_reconnect;
            vec![
                r_val("Reconnect Delay", "Seconds before auto-rejoin attempt", format!("{}s", c.delay_secs)),
                r_val("Max Attempts", "Maximum retry count per disconnect", format!("{}", c.max_attempts)),
                r_tog("Show Net Health", "Show packet/ping status indicator", c.show_server_health),
            ]
        }
        28 => {
            let c = &v.chat_macros;
            vec![
                r_tog("Auto-GG", "Send GG automatically on match end", c.auto_gg),
                r_val("Run /home", "Execute /home macro immediately", "Send".into()),
                r_val("Run /spawn", "Execute /spawn macro immediately", "Send".into()),
                r_val("Say gg", "Send gg in chat immediately", "Send".into()),
            ]
        }
        29 => {
            let c = &v.tnt_timer;
            vec![
                r_tog("Show TNT Fuse Timer", "Floating countdown above primed TNT", c.show_tnt_fuse),
                r_tog("Show Creeper Fuse", "Countdown when creeper swells", c.show_creeper_fuse),
                r_tog("Blast Danger Halo", "Draw 5-block ground blast ring", c.danger_halo),
                r_tog("End Crystal Ring", "Draw 6-block crystal blast ring", c.crystal_ring),
            ]
        }
        30 => {
            let c = &v.auto_armor;
            vec![
                r_tog("Auto-Equip on Tick", "Equip stronger armor from inventory", c.auto_on_pickup),
                r_tog("Respect Equipped Elytra", "Do not overwrite worn Elytra with chestplate", c.respect_elytra),
                r_tog("Prefer Protection", "Prioritize higher tier armor pieces", c.prefer_protection),
            ]
        }
        31 => {
            let c = &v.elytra_swap;
            vec![
                r_val("Swap Chestplate ↔ Elytra", "Swap chest slot right now", "Swap Now".into()),
                r_tog("Midair Jump Swap", "Press Jump midair to equip Elytra", c.double_jump_swap),
                r_tog("Swap Back on Land", "Re-equip Chestplate upon landing", c.swap_back_on_land),
            ]
        }
        32 => {
            let c = &v.slime_mobcap;
            vec![
                r_tog("Slime Chunk Indicator", "Badge when standing inside a slime chunk", c.show_slime_chunk),
                r_tog("Local Mob Count", "Show loaded nearby entity count", c.show_mob_count),
                r_val("HUD Corner", "Corner for slime/mob badge", c.corner.label().to_owned()),
            ]
        }
        33 => {
            let c = &v.auto_eat;
            vec![
                r_val("Hunger Threshold", "Start eating at or below drumsticks", format!("{}", c.hunger_threshold)),
                r_val("Emergency HP Threshold", "Eat food when HP drops below value", format!("{}", c.hp_threshold)),
                r_tog("Avoid Rotten/Raw Food", "Skip harmful foods when auto-eating", c.avoid_rotten_raw),
            ]
        }
        34 => {
            let c = &v.radial_menu;
            vec![
                r_tog("Include Pearl Action", "Show Ender Pearl shortcut", c.include_pearl),
                r_tog("Include Gapple Action", "Show Golden Apple shortcut", c.include_gapple),
                r_tog("Include Firework Action", "Show Firework shortcut", c.include_firework),
                r_tog("Include Totem Action", "Show Offhand Totem shortcut", c.include_totem),
            ]
        }
        35 => {
            let c = &v.clear_skies;
            vec![
                r_tog("Hide Rain & Snow", "Keep weather visually clear client-side", c.hide_rain_snow),
                r_tog("Clear Water Fog", "Improve underwater visibility", c.clear_water_fog),
                r_tog("Clear Nether Fog", "Push back red Nether distance fog", c.clear_nether_fog),
                r_tog("Clear Terrain Fog", "Push back overworld distance fog", c.clear_terrain_fog),
                r_val("Time Override", "Lock sky time visually (Default/Noon/Sunset/Midnight)", c.time_override.label().to_owned()),
            ]
        }
        36 => {
            let c = &v.smart_trade;
            vec![
                r_tog("Highlight Max Enchants", "Highlight top-tier villager books", c.highlight_max_enchants),
                r_tog("Highlight Cheap Trades", "Highlight low emerald cost offers", c.highlight_cheap_trades),
                r_val("Max Emerald Threshold", "Price ceiling for cheap trade badge", format!("{} em", c.cheap_emerald_max)),
            ]
        }
        37 => {
            let c = &v.totem_pop_counter;
            vec![
                r_tog("Track Self Pops", "Count your own totem pops", c.track_self),
                r_tog("Track Enemy Pops", "Show HUD pop feed for nearby players/mobs", c.track_enemies),
                r_tog("Chat Pop Feed", "Log totem pops into system chat", c.show_chat_feed),
                r_val("HUD Corner", "Corner for pop feed", c.corner.label().to_owned()),
            ]
        }
        38 => {
            let c = &v.auto_respawn;
            vec![
                r_tog("Instant Auto-Respawn", "Skip death screen automatically", c.instant_respawn),
                r_tog("Log Death Coords", "Print exact XYZ coordinates to chat on death", c.log_death_coords),
                r_tog("Save Death Waypoint", "Create red Death waypoint automatically", c.save_death_waypoint),
            ]
        }
        39 => {
            let c = &v.shulker_peek;
            vec![
                r_tog("Peek Shulker Boxes", "Render 9x3 inventory grid in item tooltips", c.peek_shulkers),
                r_tog("Peek Bundles", "Preview bundle contents in tooltips", c.peek_bundles),
                r_tog("Color by Shulker Dye", "Tint peek frame by shulker color", c.color_by_shulker),
            ]
        }
        40 => {
            let c = &v.ground_drops;
            vec![
                r_tog("Rare Item Beams", "Draw vertical glow beams on dropped items", c.rare_item_beams),
                r_tog("Stack Count Labels", "Show item name & count over drops", c.merge_count_labels),
                r_tog("Highlight Totems", "Special beam color for Totems & Netherite", c.highlight_totems),
                r_val("Scan Range", "Drop label range in blocks", format!("{}m", c.range)),
            ]
        }
        41 => {
            let c = &v.entity_culling;
            vec![
                r_val("Cull Distance", "Skip distant entity rendering past range", format!("{}m", c.cull_distance)),
                r_tog("Skip Behind Camera", "Cull off-screen entities behind view", c.skip_back_entities),
                r_tog("Reduce Explosion Particles", "Trim heavy explosion particles", c.reduce_explosions),
                r_tog("Reduce Smoke Particles", "Trim campfires/torch smoke", c.reduce_smoke),
            ]
        }
        42 => {
            let c = &v.auto_fish;
            vec![
                r_tog("Visual Bite Alert", "Flash FISH BITE! banner above crosshair", c.visual_bite_alert),
                r_tog("Audio Bite Chime", "Play chime on bobber nibble", c.audio_chime),
                r_tog("Auto-Reel on Bite", "Automatically reel rod when fish bites", c.auto_reel),
                r_tog("Auto-Recast", "Cast bobber again after reeling", c.auto_recast),
            ]
        }
        43 => {
            let c = &v.look_inspector;
            vec![
                r_tog("Show Targeted Block", "Top HUD card with looked-at block", c.show_blocks),
                r_tog("Show 3D Block/Item Icon", "Render looked-at block in 3D inside Jade card", c.show_3d_icon),
                r_tog("Show Targeted Entity", "Show mob/player name & HP bar under crosshair", c.show_mob_health),
                r_tog("Tool & Break Hint", "Display recommended tool / breakable status", c.show_tool_hint),
                r_tog("Show Mod Source", "Display Minecraft / mod namespace in blue", c.show_mod_source),
                r_tog("Compact Mode", "Hide XYZ block coordinates in card", c.compact),
                r_val("Top Bar Offset", "Vertical clearance below top touch bar", format!("{}px", c.y_offset)),
            ]
        }
        44 => {
            let c = &v.sound_muffler;
            vec![
                r_tog("Mute Nether Portals", "Silence ambient portal hum", c.mute_portals),
                r_tog("Mute Ghasts", "Lower/mute ghast screams", c.mute_ghasts),
                r_tog("Mute Weather", "Mute rain & thunder", c.mute_weather),
                r_tog("Mute Pistons", "Mute piston extend/retract clicks", c.mute_pistons),
                r_tog("Mute Minecarts", "Mute rolling minecart loops", c.mute_minecarts),
                r_tog("Mute Villagers", "Mute villager idle hrmms", c.mute_villagers),
                r_val("Muffle Level", "Volume reduction percentage", format!("{}%", c.muffle_pct)),
            ]
        }
        45 => {
            let c = &v.portal_calc;
            vec![
                r_tog("Show Link Pill", "Display ÷8 / ×8 coords while holding obsidian", c.show_hud_pill),
                r_tog("Show Link Radius", "Portal search radius info", c.show_link_radius),
                r_val("HUD Corner", "Corner for portal helper", c.corner.label().to_owned()),
            ]
        }
        46 => {
            let c = &v.compass_hud;
            vec![
                r_tog("Show Cardinals (N E S W)", "Display compass directions", c.show_cardinals),
                r_tog("Show Waypoints", "Render saved waypoint pips on bar", c.show_waypoints),
                r_tog("Show Death Marker", "Render death marker pip on bar", c.show_death_marker),
                r_val("Compass Width", "Bar width scale", format!("{}%", c.width_pct)),
            ]
        }
        47 => {
            let c = &v.safe_walk;
            vec![
                r_tog("Ledge Edge-Stop", "Automatically sneak at block edges", c.edge_stop),
                r_tog("Only With Blocks", "Activate only while holding building blocks", c.only_with_blocks),
                r_tog("Anti-AFK Micro-Turn", "Prevent AFK kick while at farms", c.anti_afk),
                r_val("Anti-AFK Interval", "Seconds between micro-turns", format!("{}s", c.afk_interval_secs)),
            ]
        }
        48 => {
            let c = &v.respawn_status;
            vec![
                r_tog("Show Spawn Coords", "Show bed/anchor respawn coordinates", c.show_spawn_coords),
                r_tog("Warn If Obstructed", "Alert when spawn is missing or blocked", c.warn_obstructed),
                r_val("HUD Corner", "Corner for spawn indicator", c.corner.label().to_owned()),
            ]
        }
        49 => {
            let c = &v.scoreboard_tweaks;
            vec![
                r_tog("Hide Red Numbers", "Strip right-side score numbers on sidebar", c.hide_numbers),
                r_tog("Hide Scoreboard", "Hide sidebar completely", c.hide_scoreboard),
                r_tog("Compact Bossbars", "Show only the primary bossbar", c.compact_bossbars),
                r_tog("Hide Bossbars", "Hide all top bossbars", c.hide_bossbars),
            ]
        }
        50 => {
            let c = &v.sign_stand_helper;
            vec![
                r_tog("Auto-Copy Last Sign", "Remember last edited sign lines", c.auto_copy_last_sign),
                r_tog("Auto-Paste Empty Signs", "Prefill new blank signs with last text", c.quick_paste_button),
                r_tog("Armor Stand Presets", "Enable quick pose presets", c.armor_stand_presets),
            ]
        }
        51 => {
            let c = &v.dynamic_lights;
            vec![
                r_tog("Held Light Items", "Torches, glowstone & lanterns emit light in hand", c.held_items),
                r_tog("Dropped Light Items", "Glowing items dropped on ground emit light", c.dropped_items),
                r_tog("Burning Entities", "On-fire mobs/entities emit light", c.burning_entities),
                r_val("Minimum Light Level", "Clamp minimum emitted light level", format!("{}", c.min_level)),
            ]
        }
        52 => {
            let c = &v.low_shield;
            vec![
                r_val("Shield Drop", "Lower 1st-person shield height", format!("{}%", c.shield_drop_pct)),
                r_val("Offhand Drop", "Lower offhand item/totem height", format!("{}%", c.offhand_drop_pct)),
                r_val("Mainhand Drop", "Lower mainhand weapon height", format!("{}%", c.mainhand_drop_pct)),
                r_tog("Hide Blocking Tilt", "Keep view steady while raising shield", c.hide_blocking_tilt),
            ]
        }
        53 => {
            let c = &v.clean_view;
            vec![
                r_tog("Hide Self Potion Swirls", "Stop your own effect particles blocking sight", c.hide_self_potions),
                r_tog("Hide Ambient Beacons", "Suppress ambient beacon swirl particles", c.hide_ambient_beacons),
                r_tog("Keep Enemy Particles", "Still render opponents' potion particles", c.keep_enemy_particles),
            ]
        }
        54 => {
            let c = &v.name_tags_plus;
            vec![
                r_tog("Show Own NameTag", "Display your own nametag in 3rd-person", c.show_own_nametag),
                r_tog("Show Distance", "Append distance in meters on player tags", c.show_distance),
                r_tog("Show Health Tag", "Append HP badge on player nametags", c.show_hp_tag),
                r_tog("No Sneak Fade", "Keep tags bright even when sneaking", c.no_sneak_fade),
            ]
        }
        55 => {
            let c = &v.hit_color;
            vec![
                r_val("Hit Flash Color", "Tint color when hitting entities", c.color.label().to_owned()),
                r_val("Flash Intensity", "Damage flash opacity strength", format!("{}%", c.intensity_pct)),
                r_tog("Flash Worn Armor", "Tint equipped armor pieces during hurt flash", c.flash_armor),
            ]
        }
        56 => {
            let c = &v.crit_ring;
            vec![
                r_tog("Airborne CRIT Badge", "Show CRIT READY cue when falling for a crit", c.show_crit_badge),
                r_tog("Attack Cooldown Bar", "Draw compact swing recharge bar under crosshair", c.show_cooldown_bar),
                r_val("Crit Accent Color", "Badge & bar highlight color", c.crit_color.label().to_owned()),
            ]
        }
        57 => {
            let c = &v.bow_charge;
            vec![
                r_tog("Draw % Progress Bar", "Show bow/crossbow pull percentage", c.show_pct_bar),
                r_tog("Full Draw Chime", "Audio ding when bow reaches 100% power", c.full_draw_chime),
                r_tog("Auto-Release at 100%", "Release arrow automatically at max charge", c.auto_release_full),
                r_val("HUD Corner", "Corner for bow gauge", c.corner.label().to_owned()),
            ]
        }
        58 => {
            let c = &v.cooldown_hud;
            vec![
                r_tog("Track Ender Pearls", "Show live pearl throw cooldown", c.track_pearls),
                r_tog("Track Wind Charges", "Show wind charge cooldown", c.track_wind_charge),
                r_tog("Track Chorus & Gapples", "Show chorus fruit & enchanted apple cooldowns", c.track_chorus_gapple),
                r_val("HUD Corner", "Corner for cooldown pill", c.corner.label().to_owned()),
            ]
        }
        59 => {
            let c = &v.reach_box;
            vec![
                r_tog("Highlight Target Box", "Draw wireframe box on aimed entity in 3.0m reach", c.draw_target_box),
                r_tog("3.0m Reach Ring", "Draw ground reach circle around target", c.draw_reach_ring),
                r_tog("Recolor on Crit Ready", "Turn box gold/orange when airborne crit armed", c.color_on_crit),
                r_val("Box Color", "Wireframe highlight color", c.color.label().to_owned()),
            ]
        }
        60 => {
            let c = &v.potion_hud;
            vec![
                r_tog("Show MM:SS Timers", "Display remaining time on effect pills", c.show_timers),
                r_tog("Blink Under 10s", "Pulse icon when effect is nearly expired", c.blink_under_10s),
                r_tog("Compact Pills", "Use slim horizontal chips", c.compact_pills),
                r_val("HUD Corner", "Screen corner for effects bar", c.corner.label().to_owned()),
            ]
        }
        61 => {
            let c = &v.durability_warn;
            vec![
                r_val("Alert Threshold", "Trigger warning below durability %", format!("{}%", c.warn_pct)),
                r_tog("Check Worn Armor", "Monitor helmet, chest, legs & boots", c.check_armor),
                r_tog("Check Held Tool / Shield", "Monitor active mainhand & offhand items", c.check_held),
                r_tog("Play Warning Chime", "Sound alert when durability crosses threshold", c.play_chime),
            ]
        }
        62 => {
            let c = &v.biome_title;
            vec![
                r_tog("Biome Entry Banner", "Show title banner when stepping into new biome", c.show_biome_changes),
                r_tog("Dimension Entry Banner", "Show banner when entering Nether/End/Overworld", c.show_dimension_changes),
                r_val("Display Duration", "Banner visibility seconds", format!("{}s", c.display_secs)),
                r_tog("Include Coordinates", "Show XYZ subtitle beneath biome name", c.show_sub_coords),
            ]
        }
        63 => {
            let c = &v.xp_tracker;
            vec![
                r_tog("XP to Next Level", "Show exact points & % needed to level up", c.show_next_level_xp),
                r_tog("Mending Repair Summary", "Show total missing durability across Mending gear", c.show_mending_summary),
                r_val("HUD Corner", "Corner for XP tracker pill", c.corner.label().to_owned()),
            ]
        }
        64 => {
            let c = &v.inventory_hud;
            vec![
                r_tog("Show 3x9 Inventory Grid", "Compact translucent grid of main inventory", c.show_3x9_grid),
                r_tog("Show Free Slots Counter", "Display empty slot count badge", c.show_free_slots),
                r_val("Background Opacity", "Transparency of inventory preview card", format!("{}%", c.bg_opacity_pct)),
                r_val("HUD Corner", "Screen corner for preview grid", c.corner.label().to_owned()),
            ]
        }
        65 => {
            let c = &v.tab_ping;
            vec![
                r_tog("Show Ping (ms)", "Display server latency next to online status", c.show_ping_ms),
                r_tog("Show Player Count", "Display active nearby/server player count", c.show_player_count),
                r_tog("Show Dimension Tag", "Display Overworld/Nether/End pill", c.show_dim_tag),
                r_val("HUD Corner", "Corner for status list", c.corner.label().to_owned()),
            ]
        }
        66 => {
            let c = &v.moon_phase;
            vec![
                r_tog("Show Moon Phase", "Current lunar phase & illumination %", c.show_moon_phase),
                r_tog("Swamp Slime Rating", "Indicate high/low slime spawn moon phase", c.show_slime_rating),
                r_tog("Phantom Insomnia Timer", "Warn after 3 in-game days without sleep", c.show_insomnia_warn),
                r_val("HUD Corner", "Corner for lunar pill", c.corner.label().to_owned()),
            ]
        }
        67 => {
            let c = &v.session_stats;
            vec![
                r_val("Reset Session Stats", "Zero out kills, blocks & distance", "Reset".into()),
                r_tog("Track Kills", "Count mobs & players defeated", c.show_kills),
                r_tog("Track Blocks", "Count blocks mined & placed", c.show_blocks),
                r_tog("Track Distance", "Measure blocks traveled this session", c.show_distance),
                r_val("HUD Corner", "Corner for stats pill", c.corner.label().to_owned()),
            ]
        }
        68 => {
            let c = &v.weather_sound;
            vec![
                r_tog("Suppress Lightning Flash", "Remove blinding white sky flash on strikes", c.suppress_lightning_flash),
                r_tog("Limit Rain Splashes", "Reduce heavy rain splash particles for FPS", c.limit_rain_splashes),
                r_tog("Mute Thunder Shake", "Keep camera steady during thunderstorms", c.mute_thunder_shake),
            ]
        }
        69 => {
            let c = &v.custom_glint;
            vec![
                r_val("Glint Color Preset", "Visual tint style for enchanted items", c.preset.label().to_owned()),
                r_val("Glint Speed", "Shimmer animation speed", format!("{}%", c.speed_pct)),
                r_val("Glint Strength", "Shimmer brightness alpha", format!("{}%", c.strength_pct)),
                r_tog("Compact Item Scale", "Slightly shrink 1st-person held items", c.compact_item_scale),
            ]
        }
        70 => {
            let c = &v.clear_caves;
            vec![
                r_tog("Remove Void/Bedrock Fog", "Clear dark underground distance haze", c.remove_void_fog),
                r_tog("Smooth Night Vision", "Eliminate flickering in last 10s of Night Vision", c.smooth_night_vision),
                r_tog("Boost Lava Clarity", "Push back thick orange fog near lava", c.boost_lava_clarity),
            ]
        }
        71 => {
            let c = &v.player_highlight;
            vec![
                r_tog("Highlight Players", "Draw clean outline box around visible players", c.highlight_players),
                r_tog("Highlight Hostiles", "Draw outline box around nearby monster mobs", c.highlight_hostiles),
                r_val("Scan Range", "Maximum highlight distance", format!("{}m", c.range)),
                r_val("Outline Color", "Box highlight color", c.color.label().to_owned()),
            ]
        }
        72 => {
            let c = &v.litematica_grid;
            vec![
                r_val("Set Point A (Here)", "Anchor first corner of 3D measure box", "Set A".into()),
                r_val("Set Point B (Here)", "Anchor second corner of 3D measure box", "Set B".into()),
                r_val("Clear Ruler Box", "Reset current Point A & B selection", "Clear".into()),
                r_tog("Show Volume HUD", "Display dX x dY x dZ and total block volume", c.show_volume_hud),
                r_tog("Show 3D Wireframe Box", "Render bounding box & inner grid lines", c.show_3d_box),
                r_val("Box Color", "Wireframe outline color", c.color.label().to_owned()),
            ]
        }
        73 => {
            let c = &v.death_history;
            vec![
                r_val("Waypoint Manager", "View, manage & delete death/home markers", "Manage".into()),
                r_val("Set Home Bookmark", "Save current coordinates as your Home point", "Set Home".into()),
                r_val("History Limit", "How many recent deaths to keep", format!("{}", c.keep_count)),
                r_tog("Show Home Distance Pill", "Keep Home distance badge on HUD", c.show_home_pill),
                r_tog("Auto-Log Death History", "Write death spot & dimension to chat", c.auto_log_chat),
                r_val("HUD Corner", "Corner for death/home pill", c.corner.label().to_owned()),
            ]
        }
        74 => {
            let c = &v.chunk_pillars;
            vec![
                r_tog("Chunk Corner Pillars", "Vertical glowing pillars at chunk corners", c.draw_corner_pillars),
                r_tog("Biome Transition Lines", "Ground markers where biomes change", c.draw_biome_borders),
                r_val("Pillar Height", "Vertical span of corner markers", format!("{}m", c.pillar_height)),
            ]
        }
        75 => {
            let c = &v.offhand_torch;
            vec![
                r_tog("Auto-Place in Dark", "Place offhand torch when standing in dark caves", c.auto_place_dark),
                r_val("Light Threshold", "Trigger placement below light level", format!("L{}", c.light_threshold)),
                r_tog("Low Torches Alert", "Warn when offhand torches drop below 8", c.notify_low_torches),
            ]
        }
        76 => {
            let c = &v.chat_mention;
            vec![
                r_tog("Ping on Username", "Chime & banner when someone says your name", c.alert_on_username),
                r_tog("Ping on TPA / Raid", "Alert on /tpa, whisper, or raid keywords", c.alert_on_tpa_msg),
                r_tog("Play Ping Chime", "Audible notification sound on mention", c.play_ping_chime),
                r_tog("Show Mention Banner", "Flash top banner with mentioning message", c.show_popup_banner),
            ]
        }
        77 => {
            let c = &v.drop_guard;
            vec![
                r_tog("Protect Tools & Weapons", "Prevent accidental Q-drop of swords/picks/axes", c.protect_tools_weapons),
                r_tog("Protect Totems", "Prevent dropping Totems of Undying", c.protect_totems),
                r_tog("Lock Hotbar Slots 1-3", "Guard primary combat/tool slots", c.lock_slots_1_to_3),
                r_tog("Double-Tap Bypass", "Press Drop twice within 1s to intentionally drop", c.double_tap_bypass),
            ]
        }
        78 => {
            let c = &v.hotbar_restock;
            vec![
                r_tog("Restock Building Blocks", "Auto-pull matching blocks when hotbar stack runs out", c.restock_blocks),
                r_tog("Restock Pearls & Rockets", "Auto-refill ender pearls & firework stacks", c.restock_pearls_fireworks),
                r_tog("Restock Food", "Auto-refill food stacks from main inventory", c.restock_food),
                r_tog("Restock Toast", "Show brief confirmation when slot refills", c.show_toast),
            ]
        }
        79 => {
            let c = &v.photo_mode;
            vec![
                r_val("Toggle Photo Mode", "Enter or exit clean HUD-free photo view now", if c.active { "Active".into() } else { "Activate".into() }),
                r_tog("Hide All HUD", "Hide hotbar, crosshairs, chat & overlays", c.hide_all_hud),
                r_tog("Rule-of-Thirds Guides", "Draw subtle composition grid lines", c.show_frame_guides),
                r_tog("Pair With Freecam", "Freeze body & move camera freely", c.lock_freecam_pose),
            ]
        }
        80 => {
            let c = &v.jei;
            vec![
                r_val("Open / Toggle JEI Overlay", "Open searchable item & recipe browser now", if c.overlay_open { "Open".into() } else { "Toggle".into() }),
                r_tog("Show Beside Inventory", "Auto-dock JEI panel when opening Inventory/Crafting", c.show_in_inventory),
                r_tog("Show Furnace / Cooking", "Include smelting, blasting & smoking recipes", c.show_cooking_recipes),
                r_tog("Show Item IDs", "Display minecraft:item_id under tooltip header", c.show_item_ids),
                r_tog("Compact Grid", "Tighter 5-column item tiles for smaller screens", c.compact_grid),
            ]
        }
        _ => {
            let c = &v.cinematic_timelapse;
            vec![
                r_val("Start / Stop Time Lapse", "Launch or stop cinematic timelapse (countdown only shown here)", "Start / Stop".into()),
                r_val("Time Lapse Start Delay", "Countdown delay before timelapse starts (0/3/5/10/15/30s)", format!("{}s", c.start_delay_secs)),
                r_val("Camera Path Mode", "Smooth camera motion path during timelapse", c.path_mode.label().to_owned()),
                r_val("Motion Speed", "Camera orbit / pan speed degrees per sec", format!("{}°/s", c.orbit_speed_deg)),
                r_tog("Hide HUD While Active", "Hide touch buttons & bars during timelapse", c.hide_hud),
            ]
        }
    }
}

pub fn click_mod_setting_row(v: &mut VoltExtras, idx: u8, row: usize) -> Option<VoltQuickAction> {
    let base_len = base_mod_setting_rows(v, idx).len();
    if row >= base_len {
        return match row - base_len {
            0 => Some(VoltQuickAction::OpenModKeyPicker {
                mod_id: idx,
                is_use: false,
            }),
            1 => Some(VoltQuickAction::OpenModKeyPicker {
                mod_id: idx,
                is_use: true,
            }),
            2 => {
                if let Some(b) = v.binds.0.get_mut(usize::from(idx)) {
                    b.mode = b.mode.next();
                }
                None
            }
            _ => None,
        };
    }
    match (idx, row) {
        (0, 0) => v.appleskin.show_saturation_overlay = !v.appleskin.show_saturation_overlay,
        (0, 1) => v.appleskin.show_exhaustion_underlay = !v.appleskin.show_exhaustion_underlay,
        (0, 2) => v.appleskin.show_held_food_preview = !v.appleskin.show_held_food_preview,
        (0, 3) => v.appleskin.show_health_regen_overlay = !v.appleskin.show_health_regen_overlay,
        (0, 4) => v.appleskin.show_food_tooltip = !v.appleskin.show_food_tooltip,
        (0, 5) => v.appleskin.always_show_tooltip = !v.appleskin.always_show_tooltip,
        (0, 6) => v.appleskin.show_debug_stats = !v.appleskin.show_debug_stats,
        (0, 7) => v.appleskin.highlight_full_bar = !v.appleskin.highlight_full_bar,
        (0, 8) => {
            v.appleskin.pulse_speed_pct = match v.appleskin.pulse_speed_pct {
                100 => 150,
                150 => 200,
                200 => 50,
                _ => 100,
            };
        }

        (1, 0) => v.crystal_optimizer.instant_break = !v.crystal_optimizer.instant_break,
        (1, 1) => v.crystal_optimizer.respect_weakness = !v.crystal_optimizer.respect_weakness,
        (1, 2) => v.crystal_optimizer.fast_place = !v.crystal_optimizer.fast_place,
        (1, 3) => {
            v.crystal_optimizer.timeout_ms = match v.crystal_optimizer.timeout_ms {
                250 => 500,
                500 => 1000,
                _ => 250,
            };
        }

        (2, 0) => v.shield_status.enabled_color = v.shield_status.enabled_color.next_enabled(),
        (2, 1) => v.shield_status.disabled_color = v.shield_status.disabled_color.next_disabled(),
        (2, 2) => v.shield_status.interpolate_colors = !v.shield_status.interpolate_colors,
        (2, 3) => v.shield_status.display_mode = v.shield_status.display_mode.next(),
        (2, 4) => v.shield_status.disable_sound = !v.shield_status.disable_sound,

        (3, 0) => v.uku_hud.armor_enabled = !v.uku_hud.armor_enabled,
        (3, 1) => v.uku_hud.potion_enabled = !v.uku_hud.potion_enabled,
        (3, 2) => v.uku_hud.armor_anchor = v.uku_hud.armor_anchor.next(),
        (3, 3) => v.uku_hud.armor_side = v.uku_hud.armor_side.next(),
        (3, 4) => v.uku_hud.box_style = v.uku_hud.box_style.next(),
        (3, 5) => v.uku_hud.durability_display = v.uku_hud.durability_display.next(),
        (3, 6) => v.uku_hud.show_offhand = !v.uku_hud.show_offhand,
        (3, 7) => v.uku_hud.hide_empty = !v.uku_hud.hide_empty,
        (3, 8) => {
            v.uku_hud.low_durability_pct = match v.uku_hud.low_durability_pct {
                15 => 25,
                25 => 10,
                10 => 0,
                _ => 15,
            };
        }
        (3, 9) => v.uku_hud.potion_anchor = v.uku_hud.potion_anchor.next(),
        (3, 10) => v.uku_hud.potion_compact = !v.uku_hud.potion_compact,

        (4, 0) => return Some(VoltQuickAction::ToggleFreelook),
        (4, 1) => v.freelook.toggle_mode = !v.freelook.toggle_mode,
        (4, 2) => v.freelook.invert_y = !v.freelook.invert_y,
        (4, 3) => {
            v.freelook.sensitivity_pct = if v.freelook.sensitivity_pct >= 150 {
                75
            } else {
                v.freelook.sensitivity_pct + 25
            };
        }
        (4, 4) => v.freelook.preserve_perspective = !v.freelook.preserve_perspective,

        (5, 0) => v.trajectories.show_bow = !v.trajectories.show_bow,
        (5, 1) => v.trajectories.show_pearl = !v.trajectories.show_pearl,
        (5, 2) => v.trajectories.show_trident = !v.trajectories.show_trident,
        (5, 3) => v.trajectories.show_wind_charge = !v.trajectories.show_wind_charge,
        (5, 4) => v.trajectories.show_impact_box = !v.trajectories.show_impact_box,
        (5, 5) => v.trajectories.color = v.trajectories.color.next_any(),

        (6, 0) => v.health_indicators.show_bars = !v.health_indicators.show_bars,
        (6, 1) => v.health_indicators.show_damage_popups = !v.health_indicators.show_damage_popups,
        (6, 2) => v.health_indicators.highlight_crits = !v.health_indicators.highlight_crits,
        (6, 3) => v.health_indicators.show_hostiles = !v.health_indicators.show_hostiles,
        (6, 4) => {
            v.health_indicators.max_range = match v.health_indicators.max_range {
                16 => 24,
                24 => 36,
                36 => 48,
                _ => 16,
            };
        }
        (6, 5) => {
            v.health_indicators.bar_scale = match v.health_indicators.bar_scale {
                80 => 100,
                100 => 125,
                125 => 150,
                _ => 80,
            };
        }

        (7, 0) => return Some(VoltQuickAction::QuickOffhandTotem),
        (7, 1) => v.item_scroller.auto_refill_hotbar = !v.item_scroller.auto_refill_hotbar,
        (7, 2) => v.item_scroller.quick_offhand_totem = !v.item_scroller.quick_offhand_totem,
        (7, 3) => v.item_scroller.preserve_tools = !v.item_scroller.preserve_tools,

        (8, 0) => v.tweakeroo.fast_block_place = !v.tweakeroo.fast_block_place,
        (8, 1) => v.tweakeroo.light_overlay = !v.tweakeroo.light_overlay,
        (8, 2) => v.tweakeroo.auto_tool = !v.tweakeroo.auto_tool,
        (8, 3) => v.tweakeroo.silk_touch_guard = !v.tweakeroo.silk_touch_guard,
        (8, 4) => {
            v.tweakeroo.light_range = match v.tweakeroo.light_range {
                6 => 10,
                10 => 14,
                _ => 6,
            };
        }

        (9, 0) => v.elytra_hud.show_altitude = !v.elytra_hud.show_altitude,
        (9, 1) => v.elytra_hud.show_pitch_gauge = !v.elytra_hud.show_pitch_gauge,
        (9, 2) => v.elytra_hud.show_fireworks = !v.elytra_hud.show_fireworks,
        (9, 3) => v.elytra_hud.show_mace_damage = !v.elytra_hud.show_mace_damage,

        (10, 0) => return Some(VoltQuickAction::OpenWaypointManager),
        (10, 1) => return Some(VoltQuickAction::AddWaypointHere),
        (10, 2) => return Some(VoltQuickAction::ClearDeathWaypoints),
        (10, 3) => v.waypoints.auto_death_waypoint = !v.waypoints.auto_death_waypoint,
        (10, 4) => v.waypoints.show_beacons = !v.waypoints.show_beacons,
        (10, 5) => v.waypoints.show_hud_distances = !v.waypoints.show_hud_distances,
        (10, 6) => {
            v.waypoints.max_distance = match v.waypoints.max_distance {
                500 => 1000,
                1000 => 2500,
                _ => 500,
            };
        }

        (11, 0) => v.totem_counter.show_hud_badge = !v.totem_counter.show_hud_badge,
        (11, 1) => v.totem_counter.low_hp_alert = !v.totem_counter.low_hp_alert,
        (11, 2) => {
            v.totem_counter.low_hp_hearts = match v.totem_counter.low_hp_hearts {
                2 => 3,
                3 => 4,
                4 => 5,
                _ => 2,
            };
        }
        (11, 3) => v.totem_counter.warn_when_empty = !v.totem_counter.warn_when_empty,
        (11, 4) => v.totem_counter.corner = v.totem_counter.corner.next(),

        (12, 0) => v.cps_combo.show_lmb_cps = !v.cps_combo.show_lmb_cps,
        (12, 1) => v.cps_combo.show_rmb_cps = !v.cps_combo.show_rmb_cps,
        (12, 2) => v.cps_combo.show_reach = !v.cps_combo.show_reach,
        (12, 3) => v.cps_combo.show_combo = !v.cps_combo.show_combo,
        (12, 4) => v.cps_combo.corner = v.cps_combo.corner.next(),

        (13, 0) => v.keystrokes.show_cps = !v.keystrokes.show_cps,
        (13, 1) => v.keystrokes.show_space = !v.keystrokes.show_space,
        (13, 2) => {
            v.keystrokes.bg_opacity_pct = match v.keystrokes.bg_opacity_pct {
                40 => 65,
                65 => 85,
                _ => 40,
            };
        }
        (13, 3) => v.keystrokes.corner = v.keystrokes.corner.next(),

        (14, 0) => {
            v.pvp_visuals.low_fire_pct = match v.pvp_visuals.low_fire_pct {
                45 => 25,
                25 => 0,
                0 => 100,
                _ => 45,
            };
        }
        (14, 1) => v.pvp_visuals.hide_pumpkin = !v.pvp_visuals.hide_pumpkin,
        (14, 2) => v.pvp_visuals.hide_portal_overlay = !v.pvp_visuals.hide_portal_overlay,
        (14, 3) => {
            v.pvp_visuals.hurt_cam_pct = match v.pvp_visuals.hurt_cam_pct {
                25 => 0,
                0 => 50,
                50 => 100,
                _ => 25,
            };
        }
        (14, 4) => v.pvp_visuals.reduced_bobbing = !v.pvp_visuals.reduced_bobbing,

        (15, 0) => v.crosshair_tweaks.hit_marker = !v.crosshair_tweaks.hit_marker,
        (15, 1) => v.crosshair_tweaks.style = v.crosshair_tweaks.style.next(),
        (15, 2) => v.crosshair_tweaks.color = v.crosshair_tweaks.color.next_any(),
        (15, 3) => {
            v.crosshair_tweaks.size = if v.crosshair_tweaks.size >= 10 {
                4
            } else {
                v.crosshair_tweaks.size + 2
            };
        }

        (16, 0) => return Some(VoltQuickAction::ToggleZoom),
        (16, 1) => {
            v.zoom.zoom_level = match v.zoom.zoom_level {
                3 => 4,
                4 => 6,
                6 => 8,
                _ => 3,
            };
        }
        (16, 2) => v.zoom.smooth = !v.zoom.smooth,
        (16, 3) => v.zoom.scale_sensitivity = !v.zoom.scale_sensitivity,

        (17, 0) => v.auto_sprint.show_indicator = !v.auto_sprint.show_indicator,
        (17, 1) => v.auto_sprint.sprint_in_water = !v.auto_sprint.sprint_in_water,
        (17, 2) => v.auto_sprint.corner = v.auto_sprint.corner.next(),

        (18, 0) => {
            v.fullbright.gamma_pct = match v.fullbright.gamma_pct {
                300 => 600,
                600 => 1000,
                _ => 300,
            };
        }
        (18, 1) => v.fullbright.smooth_fade = !v.fullbright.smooth_fade,

        (19, 0) => v.coords_hud.show_xyz = !v.coords_hud.show_xyz,
        (19, 1) => v.coords_hud.show_facing = !v.coords_hud.show_facing,
        (19, 2) => v.coords_hud.show_biome = !v.coords_hud.show_biome,
        (19, 3) => v.coords_hud.show_nether_equiv = !v.coords_hud.show_nether_equiv,
        (19, 4) => return Some(VoltQuickAction::CopyCoords),
        (19, 5) => v.coords_hud.corner = v.coords_hud.corner.next(),

        (20, 0) => v.telemetry_hud.show_fps = !v.telemetry_hud.show_fps,
        (20, 1) => v.telemetry_hud.show_ping = !v.telemetry_hud.show_ping,
        (20, 2) => v.telemetry_hud.show_bps = !v.telemetry_hud.show_bps,
        (20, 3) => v.telemetry_hud.corner = v.telemetry_hud.corner.next(),

        (21, 0) => v.item_counters.count_arrows = !v.item_counters.count_arrows,
        (21, 1) => v.item_counters.count_pearls = !v.item_counters.count_pearls,
        (21, 2) => v.item_counters.count_wind_charges = !v.item_counters.count_wind_charges,
        (21, 3) => v.item_counters.count_gapples = !v.item_counters.count_gapples,
        (21, 4) => v.item_counters.show_zero = !v.item_counters.show_zero,

        (22, 0) => v.clock_day_hud.show_day = !v.clock_day_hud.show_day,
        (22, 1) => v.clock_day_hud.use_24h = !v.clock_day_hud.use_24h,
        (22, 2) => v.clock_day_hud.show_light_level = !v.clock_day_hud.show_light_level,
        (22, 3) => v.clock_day_hud.corner = v.clock_day_hud.corner.next(),

        (23, 0) => {
            v.chunk_hitboxes.chunk_borders = !v.chunk_hitboxes.chunk_borders;
            return Some(VoltQuickAction::ToggleChunkBorders);
        }
        (23, 1) => {
            v.chunk_hitboxes.entity_hitboxes = !v.chunk_hitboxes.entity_hitboxes;
            return Some(VoltQuickAction::ToggleHitboxes);
        }
        (23, 2) => v.chunk_hitboxes.hide_in_f1 = !v.chunk_hitboxes.hide_in_f1,

        (24, 0) => v.chat_tweaks.timestamps = !v.chat_tweaks.timestamps,
        (24, 1) => v.chat_tweaks.use_24h = !v.chat_tweaks.use_24h,
        (24, 2) => v.chat_tweaks.compact_duplicates = !v.chat_tweaks.compact_duplicates,
        (24, 3) => v.chat_tweaks.keep_history = !v.chat_tweaks.keep_history,

        (25, 0) => return Some(VoltQuickAction::ToggleFreecam),
        (25, 1) => {
            v.freecam.speed = match v.freecam.speed {
                2 => 4,
                4 => 6,
                6 => 1,
                _ => 2,
            };
        }
        (25, 2) => v.freecam.hide_hand = !v.freecam.hide_hand,

        (26, 0) => v.radar.highlight_chests = !v.radar.highlight_chests,
        (26, 1) => v.radar.highlight_spawners = !v.radar.highlight_spawners,
        (26, 2) => v.radar.highlight_items = !v.radar.highlight_items,
        (26, 3) => {
            v.radar.range = match v.radar.range {
                24 => 36,
                36 => 48,
                _ => 24,
            };
        }

        (27, 0) => {
            v.auto_reconnect.delay_secs = match v.auto_reconnect.delay_secs {
                3 => 5,
                5 => 10,
                _ => 3,
            };
        }
        (27, 1) => {
            v.auto_reconnect.max_attempts = match v.auto_reconnect.max_attempts {
                3 => 5,
                5 => 10,
                _ => 3,
            };
        }
        (27, 2) => v.auto_reconnect.show_server_health = !v.auto_reconnect.show_server_health,

        (28, 0) => v.chat_macros.auto_gg = !v.chat_macros.auto_gg,
        (28, 1) => return Some(VoltQuickAction::RunChatMacro(1)),
        (28, 2) => return Some(VoltQuickAction::RunChatMacro(2)),
        (28, 3) => return Some(VoltQuickAction::RunChatMacro(3)),

        (29, 0) => v.tnt_timer.show_tnt_fuse = !v.tnt_timer.show_tnt_fuse,
        (29, 1) => v.tnt_timer.show_creeper_fuse = !v.tnt_timer.show_creeper_fuse,
        (29, 2) => v.tnt_timer.danger_halo = !v.tnt_timer.danger_halo,
        (29, 3) => v.tnt_timer.crystal_ring = !v.tnt_timer.crystal_ring,

        (30, 0) => v.auto_armor.auto_on_pickup = !v.auto_armor.auto_on_pickup,
        (30, 1) => v.auto_armor.respect_elytra = !v.auto_armor.respect_elytra,
        (30, 2) => v.auto_armor.prefer_protection = !v.auto_armor.prefer_protection,

        (31, 0) => return Some(VoltQuickAction::SwapChestElytra),
        (31, 1) => v.elytra_swap.double_jump_swap = !v.elytra_swap.double_jump_swap,
        (31, 2) => v.elytra_swap.swap_back_on_land = !v.elytra_swap.swap_back_on_land,

        (32, 0) => v.slime_mobcap.show_slime_chunk = !v.slime_mobcap.show_slime_chunk,
        (32, 1) => v.slime_mobcap.show_mob_count = !v.slime_mobcap.show_mob_count,
        (32, 2) => v.slime_mobcap.corner = v.slime_mobcap.corner.next(),

        (33, 0) => {
            v.auto_eat.hunger_threshold = match v.auto_eat.hunger_threshold {
                12 => 14,
                14 => 16,
                16 => 8,
                _ => 12,
            };
        }
        (33, 1) => {
            v.auto_eat.hp_threshold = match v.auto_eat.hp_threshold {
                8 => 12,
                12 => 16,
                _ => 8,
            };
        }
        (33, 2) => v.auto_eat.avoid_rotten_raw = !v.auto_eat.avoid_rotten_raw,

        (34, 0) => v.radial_menu.include_pearl = !v.radial_menu.include_pearl,
        (34, 1) => v.radial_menu.include_gapple = !v.radial_menu.include_gapple,
        (34, 2) => v.radial_menu.include_firework = !v.radial_menu.include_firework,
        (34, 3) => v.radial_menu.include_totem = !v.radial_menu.include_totem,

        (35, 0) => v.clear_skies.hide_rain_snow = !v.clear_skies.hide_rain_snow,
        (35, 1) => v.clear_skies.clear_water_fog = !v.clear_skies.clear_water_fog,
        (35, 2) => v.clear_skies.clear_nether_fog = !v.clear_skies.clear_nether_fog,
        (35, 3) => v.clear_skies.clear_terrain_fog = !v.clear_skies.clear_terrain_fog,
        (35, 4) => v.clear_skies.time_override = v.clear_skies.time_override.next(),

        (36, 0) => v.smart_trade.highlight_max_enchants = !v.smart_trade.highlight_max_enchants,
        (36, 1) => v.smart_trade.highlight_cheap_trades = !v.smart_trade.highlight_cheap_trades,
        (36, 2) => {
            v.smart_trade.cheap_emerald_max = match v.smart_trade.cheap_emerald_max {
                8 => 16,
                16 => 24,
                _ => 8,
            };
        }

        (37, 0) => v.totem_pop_counter.track_self = !v.totem_pop_counter.track_self,
        (37, 1) => v.totem_pop_counter.track_enemies = !v.totem_pop_counter.track_enemies,
        (37, 2) => v.totem_pop_counter.show_chat_feed = !v.totem_pop_counter.show_chat_feed,
        (37, 3) => v.totem_pop_counter.corner = v.totem_pop_counter.corner.next(),

        (38, 0) => v.auto_respawn.instant_respawn = !v.auto_respawn.instant_respawn,
        (38, 1) => v.auto_respawn.log_death_coords = !v.auto_respawn.log_death_coords,
        (38, 2) => v.auto_respawn.save_death_waypoint = !v.auto_respawn.save_death_waypoint,

        (39, 0) => v.shulker_peek.peek_shulkers = !v.shulker_peek.peek_shulkers,
        (39, 1) => v.shulker_peek.peek_bundles = !v.shulker_peek.peek_bundles,
        (39, 2) => v.shulker_peek.color_by_shulker = !v.shulker_peek.color_by_shulker,

        (40, 0) => v.ground_drops.rare_item_beams = !v.ground_drops.rare_item_beams,
        (40, 1) => v.ground_drops.merge_count_labels = !v.ground_drops.merge_count_labels,
        (40, 2) => v.ground_drops.highlight_totems = !v.ground_drops.highlight_totems,
        (40, 3) => {
            v.ground_drops.range = match v.ground_drops.range {
                24 => 36,
                36 => 48,
                _ => 24,
            };
        }

        (41, 0) => {
            v.entity_culling.cull_distance = match v.entity_culling.cull_distance {
                32 => 48,
                48 => 64,
                _ => 32,
            };
        }
        (41, 1) => v.entity_culling.skip_back_entities = !v.entity_culling.skip_back_entities,
        (41, 2) => v.entity_culling.reduce_explosions = !v.entity_culling.reduce_explosions,
        (41, 3) => v.entity_culling.reduce_smoke = !v.entity_culling.reduce_smoke,

        (42, 0) => v.auto_fish.visual_bite_alert = !v.auto_fish.visual_bite_alert,
        (42, 1) => v.auto_fish.audio_chime = !v.auto_fish.audio_chime,
        (42, 2) => v.auto_fish.auto_reel = !v.auto_fish.auto_reel,
        (42, 3) => v.auto_fish.auto_recast = !v.auto_fish.auto_recast,

        (43, 0) => v.look_inspector.show_blocks = !v.look_inspector.show_blocks,
        (43, 1) => v.look_inspector.show_3d_icon = !v.look_inspector.show_3d_icon,
        (43, 2) => v.look_inspector.show_mob_health = !v.look_inspector.show_mob_health,
        (43, 3) => v.look_inspector.show_tool_hint = !v.look_inspector.show_tool_hint,
        (43, 4) => v.look_inspector.show_mod_source = !v.look_inspector.show_mod_source,
        (43, 5) => v.look_inspector.compact = !v.look_inspector.compact,
        (43, 6) => {
            v.look_inspector.y_offset = match v.look_inspector.y_offset {
                12 => 36,
                36 => 52,
                52 => 72,
                _ => 12,
            };
        }

        (44, 0) => v.sound_muffler.mute_portals = !v.sound_muffler.mute_portals,
        (44, 1) => v.sound_muffler.mute_ghasts = !v.sound_muffler.mute_ghasts,
        (44, 2) => v.sound_muffler.mute_weather = !v.sound_muffler.mute_weather,
        (44, 3) => v.sound_muffler.mute_pistons = !v.sound_muffler.mute_pistons,
        (44, 4) => v.sound_muffler.mute_minecarts = !v.sound_muffler.mute_minecarts,
        (44, 5) => v.sound_muffler.mute_villagers = !v.sound_muffler.mute_villagers,
        (44, 6) => {
            v.sound_muffler.muffle_pct = match v.sound_muffler.muffle_pct {
                100 => 75,
                75 => 50,
                _ => 100,
            };
        }

        (45, 0) => v.portal_calc.show_hud_pill = !v.portal_calc.show_hud_pill,
        (45, 1) => v.portal_calc.show_link_radius = !v.portal_calc.show_link_radius,
        (45, 2) => v.portal_calc.corner = v.portal_calc.corner.next(),

        (46, 0) => v.compass_hud.show_cardinals = !v.compass_hud.show_cardinals,
        (46, 1) => v.compass_hud.show_waypoints = !v.compass_hud.show_waypoints,
        (46, 2) => v.compass_hud.show_death_marker = !v.compass_hud.show_death_marker,
        (46, 3) => {
            v.compass_hud.width_pct = match v.compass_hud.width_pct {
                80 => 100,
                100 => 120,
                _ => 80,
            };
        }

        (47, 0) => v.safe_walk.edge_stop = !v.safe_walk.edge_stop,
        (47, 1) => v.safe_walk.only_with_blocks = !v.safe_walk.only_with_blocks,
        (47, 2) => v.safe_walk.anti_afk = !v.safe_walk.anti_afk,
        (47, 3) => {
            v.safe_walk.afk_interval_secs = match v.safe_walk.afk_interval_secs {
                30 => 60,
                60 => 120,
                _ => 30,
            };
        }

        (48, 0) => v.respawn_status.show_spawn_coords = !v.respawn_status.show_spawn_coords,
        (48, 1) => v.respawn_status.warn_obstructed = !v.respawn_status.warn_obstructed,
        (48, 2) => v.respawn_status.corner = v.respawn_status.corner.next(),

        (49, 0) => v.scoreboard_tweaks.hide_numbers = !v.scoreboard_tweaks.hide_numbers,
        (49, 1) => v.scoreboard_tweaks.hide_scoreboard = !v.scoreboard_tweaks.hide_scoreboard,
        (49, 2) => v.scoreboard_tweaks.compact_bossbars = !v.scoreboard_tweaks.compact_bossbars,
        (49, 3) => v.scoreboard_tweaks.hide_bossbars = !v.scoreboard_tweaks.hide_bossbars,

        (50, 0) => v.sign_stand_helper.auto_copy_last_sign = !v.sign_stand_helper.auto_copy_last_sign,
        (50, 1) => v.sign_stand_helper.quick_paste_button = !v.sign_stand_helper.quick_paste_button,
        (50, 2) => v.sign_stand_helper.armor_stand_presets = !v.sign_stand_helper.armor_stand_presets,

        (51, 0) => v.dynamic_lights.held_items = !v.dynamic_lights.held_items,
        (51, 1) => v.dynamic_lights.dropped_items = !v.dynamic_lights.dropped_items,
        (51, 2) => v.dynamic_lights.burning_entities = !v.dynamic_lights.burning_entities,
        (51, 3) => {
            v.dynamic_lights.min_level = match v.dynamic_lights.min_level {
                0 => 6,
                6 => 12,
                _ => 0,
            };
        }

        (52, 0) => {
            v.low_shield.shield_drop_pct = match v.low_shield.shield_drop_pct {
                25 => 45,
                45 => 65,
                65 => 0,
                _ => 25,
            };
        }
        (52, 1) => {
            v.low_shield.offhand_drop_pct = match v.low_shield.offhand_drop_pct {
                25 => 45,
                45 => 65,
                65 => 0,
                _ => 25,
            };
        }
        (52, 2) => {
            v.low_shield.mainhand_drop_pct = match v.low_shield.mainhand_drop_pct {
                0 => 15,
                15 => 30,
                30 => 45,
                _ => 0,
            };
        }
        (52, 3) => v.low_shield.hide_blocking_tilt = !v.low_shield.hide_blocking_tilt,

        (53, 0) => v.clean_view.hide_self_potions = !v.clean_view.hide_self_potions,
        (53, 1) => v.clean_view.hide_ambient_beacons = !v.clean_view.hide_ambient_beacons,
        (53, 2) => v.clean_view.keep_enemy_particles = !v.clean_view.keep_enemy_particles,

        (54, 0) => v.name_tags_plus.show_own_nametag = !v.name_tags_plus.show_own_nametag,
        (54, 1) => v.name_tags_plus.show_distance = !v.name_tags_plus.show_distance,
        (54, 2) => v.name_tags_plus.show_hp_tag = !v.name_tags_plus.show_hp_tag,
        (54, 3) => v.name_tags_plus.no_sneak_fade = !v.name_tags_plus.no_sneak_fade,

        (55, 0) => v.hit_color.color = v.hit_color.color.next_any(),
        (55, 1) => {
            v.hit_color.intensity_pct = match v.hit_color.intensity_pct {
                40 => 70,
                70 => 100,
                _ => 40,
            };
        }
        (55, 2) => v.hit_color.flash_armor = !v.hit_color.flash_armor,

        (56, 0) => v.crit_ring.show_crit_badge = !v.crit_ring.show_crit_badge,
        (56, 1) => v.crit_ring.show_cooldown_bar = !v.crit_ring.show_cooldown_bar,
        (56, 2) => v.crit_ring.crit_color = v.crit_ring.crit_color.next_any(),

        (57, 0) => v.bow_charge.show_pct_bar = !v.bow_charge.show_pct_bar,
        (57, 1) => v.bow_charge.full_draw_chime = !v.bow_charge.full_draw_chime,
        (57, 2) => v.bow_charge.auto_release_full = !v.bow_charge.auto_release_full,
        (57, 3) => v.bow_charge.corner = v.bow_charge.corner.next(),

        (58, 0) => v.cooldown_hud.track_pearls = !v.cooldown_hud.track_pearls,
        (58, 1) => v.cooldown_hud.track_wind_charge = !v.cooldown_hud.track_wind_charge,
        (58, 2) => v.cooldown_hud.track_chorus_gapple = !v.cooldown_hud.track_chorus_gapple,
        (58, 3) => v.cooldown_hud.corner = v.cooldown_hud.corner.next(),

        (59, 0) => v.reach_box.draw_target_box = !v.reach_box.draw_target_box,
        (59, 1) => v.reach_box.draw_reach_ring = !v.reach_box.draw_reach_ring,
        (59, 2) => v.reach_box.color_on_crit = !v.reach_box.color_on_crit,
        (59, 3) => v.reach_box.color = v.reach_box.color.next_any(),

        (60, 0) => v.potion_hud.show_timers = !v.potion_hud.show_timers,
        (60, 1) => v.potion_hud.blink_under_10s = !v.potion_hud.blink_under_10s,
        (60, 2) => v.potion_hud.compact_pills = !v.potion_hud.compact_pills,
        (60, 3) => v.potion_hud.corner = v.potion_hud.corner.next(),

        (61, 0) => {
            v.durability_warn.warn_pct = match v.durability_warn.warn_pct {
                10 => 15,
                15 => 25,
                25 => 35,
                _ => 10,
            };
        }
        (61, 1) => v.durability_warn.check_armor = !v.durability_warn.check_armor,
        (61, 2) => v.durability_warn.check_held = !v.durability_warn.check_held,
        (61, 3) => v.durability_warn.play_chime = !v.durability_warn.play_chime,

        (62, 0) => v.biome_title.show_biome_changes = !v.biome_title.show_biome_changes,
        (62, 1) => v.biome_title.show_dimension_changes = !v.biome_title.show_dimension_changes,
        (62, 2) => {
            v.biome_title.display_secs = match v.biome_title.display_secs {
                2 => 3,
                3 => 5,
                _ => 2,
            };
        }
        (62, 3) => v.biome_title.show_sub_coords = !v.biome_title.show_sub_coords,

        (63, 0) => v.xp_tracker.show_next_level_xp = !v.xp_tracker.show_next_level_xp,
        (63, 1) => v.xp_tracker.show_mending_summary = !v.xp_tracker.show_mending_summary,
        (63, 2) => v.xp_tracker.corner = v.xp_tracker.corner.next(),

        (64, 0) => v.inventory_hud.show_3x9_grid = !v.inventory_hud.show_3x9_grid,
        (64, 1) => v.inventory_hud.show_free_slots = !v.inventory_hud.show_free_slots,
        (64, 2) => {
            v.inventory_hud.bg_opacity_pct = match v.inventory_hud.bg_opacity_pct {
                45 => 65,
                65 => 85,
                _ => 45,
            };
        }
        (64, 3) => v.inventory_hud.corner = v.inventory_hud.corner.next(),

        (65, 0) => v.tab_ping.show_ping_ms = !v.tab_ping.show_ping_ms,
        (65, 1) => v.tab_ping.show_player_count = !v.tab_ping.show_player_count,
        (65, 2) => v.tab_ping.show_dim_tag = !v.tab_ping.show_dim_tag,
        (65, 3) => v.tab_ping.corner = v.tab_ping.corner.next(),

        (66, 0) => v.moon_phase.show_moon_phase = !v.moon_phase.show_moon_phase,
        (66, 1) => v.moon_phase.show_slime_rating = !v.moon_phase.show_slime_rating,
        (66, 2) => v.moon_phase.show_insomnia_warn = !v.moon_phase.show_insomnia_warn,
        (66, 3) => v.moon_phase.corner = v.moon_phase.corner.next(),

        (67, 0) => return Some(VoltQuickAction::ResetSessionStats),
        (67, 1) => v.session_stats.show_kills = !v.session_stats.show_kills,
        (67, 2) => v.session_stats.show_blocks = !v.session_stats.show_blocks,
        (67, 3) => v.session_stats.show_distance = !v.session_stats.show_distance,
        (67, 4) => v.session_stats.corner = v.session_stats.corner.next(),

        (68, 0) => v.weather_sound.suppress_lightning_flash = !v.weather_sound.suppress_lightning_flash,
        (68, 1) => v.weather_sound.limit_rain_splashes = !v.weather_sound.limit_rain_splashes,
        (68, 2) => v.weather_sound.mute_thunder_shake = !v.weather_sound.mute_thunder_shake,

        (69, 0) => v.custom_glint.preset = v.custom_glint.preset.next(),
        (69, 1) => {
            v.custom_glint.speed_pct = match v.custom_glint.speed_pct {
                50 => 100,
                100 => 150,
                150 => 200,
                _ => 50,
            };
        }
        (69, 2) => {
            v.custom_glint.strength_pct = match v.custom_glint.strength_pct {
                50 => 90,
                90 => 100,
                100 => 25,
                _ => 50,
            };
        }
        (69, 3) => v.custom_glint.compact_item_scale = !v.custom_glint.compact_item_scale,

        (70, 0) => v.clear_caves.remove_void_fog = !v.clear_caves.remove_void_fog,
        (70, 1) => v.clear_caves.smooth_night_vision = !v.clear_caves.smooth_night_vision,
        (70, 2) => v.clear_caves.boost_lava_clarity = !v.clear_caves.boost_lava_clarity,

        (71, 0) => v.player_highlight.highlight_players = !v.player_highlight.highlight_players,
        (71, 1) => v.player_highlight.highlight_hostiles = !v.player_highlight.highlight_hostiles,
        (71, 2) => {
            v.player_highlight.range = match v.player_highlight.range {
                24 => 32,
                32 => 48,
                _ => 24,
            };
        }
        (71, 3) => v.player_highlight.color = v.player_highlight.color.next_any(),

        (72, 0) => return Some(VoltQuickAction::SetRulerPointA),
        (72, 1) => return Some(VoltQuickAction::SetRulerPointB),
        (72, 2) => return Some(VoltQuickAction::ClearRuler),
        (72, 3) => v.litematica_grid.show_volume_hud = !v.litematica_grid.show_volume_hud,
        (72, 4) => v.litematica_grid.show_3d_box = !v.litematica_grid.show_3d_box,
        (72, 5) => v.litematica_grid.color = v.litematica_grid.color.next_any(),

        (73, 0) => return Some(VoltQuickAction::OpenWaypointManager),
        (73, 1) => return Some(VoltQuickAction::SetHomeWaypoint),
        (73, 2) => {
            v.death_history.keep_count = match v.death_history.keep_count {
                3 => 5,
                5 => 10,
                _ => 3,
            };
        }
        (73, 3) => v.death_history.show_home_pill = !v.death_history.show_home_pill,
        (73, 4) => v.death_history.auto_log_chat = !v.death_history.auto_log_chat,
        (73, 5) => v.death_history.corner = v.death_history.corner.next(),

        (74, 0) => v.chunk_pillars.draw_corner_pillars = !v.chunk_pillars.draw_corner_pillars,
        (74, 1) => v.chunk_pillars.draw_biome_borders = !v.chunk_pillars.draw_biome_borders,
        (74, 2) => {
            v.chunk_pillars.pillar_height = match v.chunk_pillars.pillar_height {
                32 => 64,
                64 => 128,
                _ => 32,
            };
        }

        (75, 0) => v.offhand_torch.auto_place_dark = !v.offhand_torch.auto_place_dark,
        (75, 1) => {
            v.offhand_torch.light_threshold = match v.offhand_torch.light_threshold {
                2 => 4,
                4 => 6,
                _ => 2,
            };
        }
        (75, 2) => v.offhand_torch.notify_low_torches = !v.offhand_torch.notify_low_torches,

        (76, 0) => v.chat_mention.alert_on_username = !v.chat_mention.alert_on_username,
        (76, 1) => v.chat_mention.alert_on_tpa_msg = !v.chat_mention.alert_on_tpa_msg,
        (76, 2) => v.chat_mention.play_ping_chime = !v.chat_mention.play_ping_chime,
        (76, 3) => v.chat_mention.show_popup_banner = !v.chat_mention.show_popup_banner,

        (77, 0) => v.drop_guard.protect_tools_weapons = !v.drop_guard.protect_tools_weapons,
        (77, 1) => v.drop_guard.protect_totems = !v.drop_guard.protect_totems,
        (77, 2) => v.drop_guard.lock_slots_1_to_3 = !v.drop_guard.lock_slots_1_to_3,
        (77, 3) => v.drop_guard.double_tap_bypass = !v.drop_guard.double_tap_bypass,

        (78, 0) => v.hotbar_restock.restock_blocks = !v.hotbar_restock.restock_blocks,
        (78, 1) => v.hotbar_restock.restock_pearls_fireworks = !v.hotbar_restock.restock_pearls_fireworks,
        (78, 2) => v.hotbar_restock.restock_food = !v.hotbar_restock.restock_food,
        (78, 3) => v.hotbar_restock.show_toast = !v.hotbar_restock.show_toast,

        (79, 0) => return Some(VoltQuickAction::TogglePhotoMode),
        (79, 1) => v.photo_mode.hide_all_hud = !v.photo_mode.hide_all_hud,
        (79, 2) => v.photo_mode.show_frame_guides = !v.photo_mode.show_frame_guides,
        (79, 3) => v.photo_mode.lock_freecam_pose = !v.photo_mode.lock_freecam_pose,

        (80, 0) => return Some(VoltQuickAction::ToggleJei),
        (80, 1) => v.jei.show_in_inventory = !v.jei.show_in_inventory,
        (80, 2) => v.jei.show_cooking_recipes = !v.jei.show_cooking_recipes,
        (80, 3) => v.jei.show_item_ids = !v.jei.show_item_ids,
        (80, 4) => v.jei.compact_grid = !v.jei.compact_grid,

        (81, 0) => return Some(VoltQuickAction::StartStopTimelapse),
        (81, 1) => {
            v.cinematic_timelapse.start_delay_secs = match v.cinematic_timelapse.start_delay_secs {
                0 => 3,
                3 => 5,
                5 => 10,
                10 => 15,
                15 => 30,
                _ => 0,
            };
        }
        (81, 2) => v.cinematic_timelapse.path_mode = v.cinematic_timelapse.path_mode.next(),
        (81, 3) => {
            v.cinematic_timelapse.orbit_speed_deg = match v.cinematic_timelapse.orbit_speed_deg {
                0 => 2,
                2 => 5,
                5 => 10,
                10 => 20,
                _ => 0,
            };
        }
        (81, 4) => v.cinematic_timelapse.hide_hud = !v.cinematic_timelapse.hide_hud,
        _ => {}
    }
    None
}
