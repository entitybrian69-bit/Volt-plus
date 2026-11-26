//! UI metadata, Modrinth icon fetching, and per-mod settings rows/actions
//! for all 52 built-in Volt+ Extras mods inside the Control Hub.
use crate::extras::{
    VOLT_MODS, VoltExtras, VoltModCategory, VoltQuickAction, volt_mod_enabled,
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
        std::thread::spawn(move || {
            let agent = ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(7))
                .build();
            let ua = "VoltPlus/1.0 (https://github.com/entitybrian69-bit/volt-plus)";
            for m in VOLT_MODS.iter() {
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

pub fn mod_setting_rows(v: &VoltExtras, idx: u8) -> Vec<SettingRow> {
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
                r_tog("Show Targeted Block", "Top HUD pill with looked-at block", c.show_blocks),
                r_tog("Show Targeted Entity", "Show mob/player name under crosshair", c.show_mob_health),
                r_tog("Compact Mode", "Hide XYZ block coordinates in pill", c.compact),
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
        _ => {
            let c = &v.dynamic_lights;
            vec![
                r_tog("Held Light Items", "Torches, glowstone & lanterns emit light in hand", c.held_items),
                r_tog("Dropped Light Items", "Glowing items dropped on ground emit light", c.dropped_items),
                r_tog("Burning Entities", "On-fire mobs/entities emit light", c.burning_entities),
                r_val("Minimum Light Level", "Clamp minimum emitted light level", format!("{}", c.min_level)),
            ]
        }
    }
}

pub fn click_mod_setting_row(v: &mut VoltExtras, idx: u8, row: usize) -> Option<VoltQuickAction> {
    match (idx, row) {
        (0, 0) => v.appleskin.show_saturation_overlay = !v.appleskin.show_saturation_overlay,
        (0, 1) => v.appleskin.show_exhaustion_underlay = !v.appleskin.show_exhaustion_underlay,
        (0, 2) => v.appleskin.show_held_food_preview = !v.appleskin.show_held_food_preview,
        (0, 3) => v.appleskin.show_health_regen_overlay = !v.appleskin.show_health_regen_overlay,
        (0, 4) => v.appleskin.show_food_tooltip = !v.appleskin.show_food_tooltip,
        (0, 5) => v.appleskin.always_show_tooltip = !v.appleskin.always_show_tooltip,
        (0, 6) => v.appleskin.show_debug_stats = !v.appleskin.show_debug_stats,

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

        (10, 0) => return Some(VoltQuickAction::AddWaypointHere),
        (10, 1) => return Some(VoltQuickAction::ClearDeathWaypoints),
        (10, 2) => v.waypoints.auto_death_waypoint = !v.waypoints.auto_death_waypoint,
        (10, 3) => v.waypoints.show_beacons = !v.waypoints.show_beacons,
        (10, 4) => v.waypoints.show_hud_distances = !v.waypoints.show_hud_distances,
        (10, 5) => {
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
        (43, 1) => v.look_inspector.show_mob_health = !v.look_inspector.show_mob_health,
        (43, 2) => v.look_inspector.compact = !v.look_inspector.compact,

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
        _ => {}
    }
    None
}
