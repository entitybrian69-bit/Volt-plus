//! Built-in Volt+ Extras mods (all 84 mods), off by default and controlled
//! from the `Volt+ Extras` tab in the Control Hub.
use super::{Dim, Game, Screen};
use horizon_client::{
    brewing,
    chat::ChatSource,
    chunk::block_entity::BlockEntity,
    crafting::{Cooking, Recipe},
    extras::{
        AppleSkinConfig, Corner, CrosshairStyle, ShieldColorPreset,
        ShieldDisplayMode, TimelapsePathMode, UkuAnchor, UkuDurability, UkuSide, UkuStyle,
        VOLT_MOD_COUNT, VOLT_MODS, VoltKeybindMode, VoltQuickAction, set_volt_mod_enabled,
        volt_mod_enabled,
    },
    gizmos::Gizmos,
    hud::{HudAtlas, HudBatch, rgba},
    hud_layout,
    items::ItemStack,
    text::Text,
};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::keyboard::KeyCode;

/// Saved world waypoint (including auto-created death markers).
#[derive(Clone, Debug)]
pub(super) struct VoltWaypoint {
    pub name: String,
    pub pos: [f64; 3],
    pub dim: Dim,
    pub is_death: bool,
    pub color: u32,
}

enum JeiRecipeView {
    Crafting {
        shapeless: bool,
        grid: [Option<String>; 9],
        output: String,
        count: u32,
    },
    Cooking {
        kind: String,
        input: String,
        output: String,
        count: u32,
        xp: f32,
        secs: f32,
    },
}

/// Floating world-space damage popup tracked per mob.
#[derive(Clone, Debug)]
pub(super) struct DamagePopup {
    pub pos: [f64; 3],
    pub amount: f32,
    pub crit: bool,
    pub created_tick: u64,
}

/// Runtime state for all 84 built-in Volt+ Extras mods.
#[derive(Default)]
pub(super) struct VoltRuntimeState {
    pub freecam_active: bool,
    pub freecam_pos: [f64; 3],
    pub freecam_yaw: f32,
    pub freecam_pitch: f32,

    pub freelook_active: bool,
    pub freelook_yaw: f32,
    pub freelook_pitch: f32,

    pub waypoints: Vec<VoltWaypoint>,
    pub mob_last_hp: HashMap<u32, f32>,
    pub damage_popups: Vec<DamagePopup>,

    pub hit_marker_until_tick: u64,
    pub crit_ring_until_tick: u64,
    pub combo_count: u32,
    pub last_hit_target: Option<u32>,
    pub last_hit_tick: u64,
    pub last_reach_blocks: Option<f32>,

    pub totem_pops_total: u32,
    pub totem_pop_feed: VecDeque<(String, u64)>,

    pub last_player_pos: Option<[f64; 3]>,
    pub speed_bps: f64,
    pub was_on_ground: bool,
    pub air_jump_ticks: u8,

    pub last_sign_lines: [String; 4],
    pub last_auto_fish_tick: u64,
    pub last_anti_afk_tick: u64,
    pub auto_eating_slot: Option<usize>,
    pub status_toast: Option<(String, u64)>,

    // New mod runtime state (52..=83)
    pub last_biome: String,
    pub biome_banner: Option<(String, u64)>,
    pub last_biome_banner_tick: u64,

    pub xp_last_total: Option<i32>,
    pub xp_gained_session: i32,
    pub xp_session_start: Option<Instant>,

    pub session_started: Option<Instant>,
    pub session_blocks_mined: u32,
    pub session_blocks_placed: u32,
    pub session_mob_kills: u32,
    pub session_damage_dealt: f32,

    pub ruler_a: Option<[i32; 3]>,
    pub ruler_b: Option<[i32; 3]>,
    pub death_history: VecDeque<(String, [i32; 3], Dim)>,

    pub drop_confirm_pending: Option<(String, Instant)>,
    pub last_held_slot: usize,
    pub last_held_item: Option<String>,
    pub last_durability_warn_tick: u64,
    pub mention_banner: Option<(String, u64)>,

    pub camera_roll_deg: f32,

    // JEI Recipe / Item Lookup state
    pub jei_open: bool,
    pub jei_search: String,
    pub jei_page: usize,
    pub jei_selected_item: Option<String>,
    pub jei_recipe_idx: usize,
    pub jei_search_focused: bool,

    // Cinematic Time Lapse state
    pub timelapse_countdown_until: Option<Instant>,
    pub timelapse_active: bool,
    pub timelapse_started: Option<Instant>,
    pub timelapse_anchor: [f64; 3],
    pub timelapse_base_yaw: f32,
    pub timelapse_base_pitch: f32,
}

/// 9x9 diagonal drumstick perimeter pixels matching vanilla `food_empty` / `food_full`
/// (meat bulb at top-left `(0..5, 0..5)` and bone pointing bottom-right `(6..8, 6..8)`).
const DRUMSTICK_OUTLINE_PIXELS: [(i32, i32); 24] = [
    (2, 0),
    (3, 0),
    (4, 0),
    (1, 1),
    (5, 1),
    (0, 2),
    (5, 2),
    (0, 3),
    (5, 3),
    (0, 4),
    (6, 4),
    (1, 5),
    (2, 5),
    (6, 5),
    (3, 6),
    (4, 6),
    (6, 6),
    (7, 6),
    (5, 7),
    (6, 7),
    (7, 7),
    (8, 7),
    (7, 8),
    (8, 8),
];

impl Game<'_> {
    // =========================================================================
    // 1. AppleSkin (authentic drumstick-hugging golden saturation outlines)
    // =========================================================================

    pub(super) fn appleskin_food_values(&self, stack: &ItemStack) -> Option<(i32, f32)> {
        let info = self.items().get(&stack.item)?;
        let (nutrition, saturation_gain) = info.food?;
        if nutrition == 0 && saturation_gain <= 0.0 {
            return None;
        }
        Some((nutrition as i32, saturation_gain))
    }

    fn appleskin_can_preview_food(
        &self,
        cfg: &AppleSkinConfig,
        food_level: i32,
        stack: &ItemStack,
    ) -> bool {
        if !cfg.show_held_food_preview {
            return false;
        }
        let Some(info) = self.items().get(&stack.item) else {
            return false;
        };
        if info.food.is_none() {
            return false;
        }
        info.can_always_eat || food_level < 20
    }

    pub(super) fn draw_appleskin_hud(
        &self,
        batch: &mut HudBatch,
        inputs: &hud_layout::HealthInputs,
        _exhaustion: f32,
        wi: i32,
        hi: i32,
    ) {
        let cfg = &self.settings.extras.volt.appleskin;
        if !cfg.enabled || inputs.vehicle_hearts > 0 {
            return;
        }

        let atlas = &self.gui.atlas;
        let x_right = wi / 2 + 91;
        let y_food = hi - 39;

        let held_food = self
            .stack_in(self.selected)
            .filter(|s| self.appleskin_can_preview_food(cfg, inputs.food, s))
            .and_then(|s| self.appleskin_food_values(s))
            .or_else(|| {
                self.offhand
                    .as_ref()
                    .filter(|s| self.appleskin_can_preview_food(cfg, inputs.food, s))
                    .and_then(|s| self.appleskin_food_values(s))
            });

        let pulse = (((inputs.millis as f32 / 1000.0) * 4.5).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        let preview_alpha = ((0.25 + 0.60 * pulse) * 255.0).round() as u8;

        if let Some((add_food, add_sat)) = held_food {
            let target_food = (inputs.food + add_food).clamp(0, 20);
            if target_food > inputs.food {
                let preview_col = rgba(255, 255, 255, preview_alpha);
                for i in 0..10 {
                    let threshold = i * 2 + 1;
                    if threshold <= inputs.food {
                        continue;
                    }
                    if threshold > target_food {
                        break;
                    }
                    let x = (x_right - i * 8 - 9) as f32;
                    let y = y_food as f32;
                    let sprite = if threshold == target_food {
                        if inputs.hunger {
                            "hud/food_half_hunger"
                        } else {
                            "hud/food_half"
                        }
                    } else if inputs.hunger {
                        "hud/food_full_hunger"
                    } else {
                        "hud/food_full"
                    };
                    batch.sprite_scaled(atlas, sprite, x, y, 9.0, 9.0, preview_col);
                }
            }

            if cfg.show_saturation_overlay {
                Self::draw_saturation_outlines(
                    batch,
                    atlas,
                    x_right,
                    y_food,
                    inputs.saturation,
                    255,
                );
                let target_sat = (inputs.saturation + add_sat).clamp(0.0, target_food as f32);
                if target_sat > inputs.saturation + 0.01 {
                    Self::draw_saturation_outlines(
                        batch,
                        atlas,
                        x_right,
                        y_food,
                        target_sat,
                        preview_alpha,
                    );
                }
            }

            if cfg.show_health_regen_overlay && inputs.health < inputs.max_health {
                let potential = target_food as f32 * 0.5;
                if target_food >= 18 && potential > 0.0 {
                    let preview_hearts = ((potential.min(inputs.max_health - inputs.health)) / 2.0)
                        .ceil() as i32;
                    let start_heart = (inputs.health / 2.0).floor() as i32;
                    let max_hearts = (inputs.max_health / 2.0).ceil() as i32;
                    let heart_col = rgba(255, 255, 255, preview_alpha);
                    for h in 0..preview_hearts {
                        let idx = start_heart + h;
                        if idx >= max_hearts {
                            break;
                        }
                        let hx = (wi / 2 - 91 + (idx % 10) * 8) as f32;
                        let hy = (hi - 39 - (idx / 10) * 10) as f32;
                        batch.sprite_scaled(atlas, "hud/heart/full", hx, hy, 9.0, 9.0, heart_col);
                    }
                }
            }
        } else if cfg.show_saturation_overlay {
            Self::draw_saturation_outlines(batch, atlas, x_right, y_food, inputs.saturation, 255);
        }
    }

    fn draw_saturation_outlines(
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        x_right: i32,
        y_food: i32,
        saturation: f32,
        alpha: u8,
    ) {
        let sat = saturation.clamp(0.0, 20.0);
        if sat <= 0.05 || alpha == 0 {
            return;
        }
        let light_gold = rgba(255, 228, 74, alpha);
        let warm_gold = rgba(244, 174, 28, alpha);
        let slots = (sat / 2.0).ceil() as i32;
        for i in 0..slots.min(10) {
            let rem = sat - (i as f32) * 2.0;
            let x = (x_right - i * 8 - 9) as f32;
            let y = y_food as f32;
            let is_half = rem < 1.5;
            for &(px, py) in &DRUMSTICK_OUTLINE_PIXELS {
                if is_half && px < 4 {
                    continue;
                }
                let col = if px + py <= 7 { light_gold } else { warm_gold };
                batch.fill(atlas, x + px as f32, y + py as f32, 1.0, 1.0, col);
            }
        }
    }

    pub(super) fn appleskin_tooltip_size(&self, stack: &ItemStack) -> Option<[f32; 2]> {
        let cfg = &self.settings.extras.volt.appleskin;
        if !cfg.enabled || !cfg.show_food_tooltip {
            return None;
        }
        let (nutrition, saturation) = self.appleskin_food_values(stack)?;
        let food_bars = ((nutrition + 1) / 2).max(1);
        let sat_bars = ((saturation / 2.0).ceil() as i32).max(1);
        let cols = food_bars.max(sat_bars).min(10);
        let rows = if sat_bars > 0 { 2 } else { 1 };
        Some([cols as f32 * 9.0 + 30.0, rows as f32 * 10.0 + 2.0])
    }

    pub(super) fn appleskin_tooltip_draw(
        &self,
        stack: &ItemStack,
        batch: &mut HudBatch,
        pos: [f32; 2],
        _size: [f32; 2],
    ) {
        let Some((nutrition, saturation)) = self.appleskin_food_values(stack) else {
            return;
        };
        let atlas = &self.gui.atlas;
        let x0 = pos[0];
        let y0 = pos[1];

        let food_icons = ((nutrition + 1) / 2).clamp(1, 10);
        for i in 0..food_icons {
            let ix = x0 + i as f32 * 9.0;
            let is_half = i == food_icons - 1 && (nutrition % 2 == 1);
            let sprite = if is_half {
                "hud/food_half"
            } else {
                "hud/food_full"
            };
            batch.sprite_scaled(atlas, sprite, ix, y0, 9.0, 9.0, u32::MAX);
        }
        batch.text(
            atlas,
            &format!("+{nutrition}"),
            x0 + food_icons as f32 * 9.0 + 4.0,
            y0 + 1.0,
            rgba(220, 224, 232, 255),
            true,
        );

        let sat_icons = ((saturation / 2.0).ceil() as i32).clamp(1, 10);
        let y1 = y0 + 10.0;
        let light_gold = rgba(255, 228, 74, 255);
        let warm_gold = rgba(244, 174, 28, 255);
        for i in 0..sat_icons {
            let ix = x0 + i as f32 * 9.0;
            let rem = saturation - (i as f32) * 2.0;
            let is_half = rem < 1.5;
            batch.sprite_scaled(
                atlas,
                if is_half {
                    "hud/food_half"
                } else {
                    "hud/food_full"
                },
                ix,
                y1,
                9.0,
                9.0,
                rgba(255, 214, 64, 255),
            );
            for &(px, py) in &DRUMSTICK_OUTLINE_PIXELS {
                if is_half && px < 4 {
                    continue;
                }
                let col = if px + py <= 7 { light_gold } else { warm_gold };
                batch.fill(atlas, ix + px as f32, y1 + py as f32, 1.0, 1.0, col);
            }
        }
        batch.text(
            atlas,
            &format!("+{saturation:.1}"),
            x0 + sat_icons as f32 * 9.0 + 4.0,
            y1 + 1.0,
            rgba(255, 214, 64, 255),
            true,
        );
    }

    // =========================================================================
    // 2. Shulker Box Peek Tooltip (for stations.rs)
    // =========================================================================

    pub(super) fn shulker_peek_tooltip_size(&self, stack: &ItemStack) -> Option<[f32; 2]> {
        let cfg = &self.settings.extras.volt.shulker_peek;
        if !cfg.enabled || !cfg.peek_shulkers {
            return None;
        }
        let extra = stack.extra.as_ref()?;
        if !extra.container.is_empty() && stack.item.contains("shulker_box") {
            return Some([168.0, 58.0]);
        }
        None
    }

    pub(super) fn shulker_peek_tooltip_draw(
        &self,
        stack: &ItemStack,
        batch: &mut HudBatch,
        pos: [f32; 2],
        _size: [f32; 2],
    ) {
        let Some(extra) = stack.extra.as_ref() else {
            return;
        };
        if extra.container.is_empty() {
            return;
        }
        let atlas = &self.gui.atlas;
        let x0 = pos[0];
        let y0 = pos[1];
        batch.fill(atlas, x0, y0, 166.0, 58.0, rgba(20, 22, 30, 215));
        for (slot, item) in &extra.container {
            let cell = usize::from(*slot);
            if cell >= 27 {
                continue;
            }
            let cx = x0 + 2.0 + (cell % 9) as f32 * 18.0;
            let cy = y0 + 2.0 + (cell / 9) as f32 * 18.0;
            batch.fill(atlas, cx, cy, 16.0, 16.0, rgba(255, 255, 255, 24));
            self.draw_item(batch, item, cx, cy);
            if item.count > 1 {
                batch.text(
                    atlas,
                    &format!("{}", item.count),
                    cx + 7.0,
                    cy + 9.0,
                    u32::MAX,
                    true,
                );
            }
        }
    }

    // =========================================================================
    // 3. Marlow's Crystal Optimizer
    // =========================================================================

    fn crystal_opt_can_break(&mut self) -> bool {
        let cfg = &self.settings.extras.volt.crystal_optimizer;
        if !cfg.enabled || !cfg.instant_break {
            return false;
        }
        if cfg.respect_weakness && self.potions.effects.get("weakness").is_some() {
            let dmg = self
                .player
                .as_mut()
                .and_then(|p| p.attribute("minecraft:attack_damage").ok())
                .unwrap_or(1.0);
            if dmg <= 0.5 {
                return false;
            }
        }
        true
    }

    pub(super) fn crystal_opt_on_attack(&mut self, id: i32) {
        if !self.crystal_opt_can_break() {
            return;
        }
        if let Some(pos) = self.crystals.iter().position(|c| c.id as i32 == id) {
            let crystal = self.crystals.remove(pos);
            self.pending_crystal_breaks.push((crystal, Instant::now()));
        }
    }

    pub(super) fn crystal_opt_on_remote_remove(&mut self, id: u32) {
        self.pending_crystal_breaks.retain(|(c, _)| c.id != id);
    }

    pub(super) fn crystal_opt_tick(&mut self) {
        let cfg = &self.settings.extras.volt.crystal_optimizer;
        if !cfg.enabled {
            if !self.pending_crystal_breaks.is_empty() {
                for (c, _) in self.pending_crystal_breaks.drain(..) {
                    if !self.crystals.iter().any(|e| e.id == c.id) {
                        self.crystals.push(c);
                    }
                }
            }
            return;
        }
        let timeout = Duration::from_millis(u64::from(cfg.timeout_ms.clamp(100, 2000)));
        let now = Instant::now();
        let mut i = 0;
        while i < self.pending_crystal_breaks.len() {
            if now.duration_since(self.pending_crystal_breaks[i].1) >= timeout {
                let (c, _) = self.pending_crystal_breaks.swap_remove(i);
                if !self.crystals.iter().any(|e| e.id == c.id) {
                    self.crystals.push(c);
                }
            } else {
                i += 1;
            }
        }
        let fast_place_tweak = self.settings.extras.volt.tweakeroo.enabled
            && self.settings.extras.volt.tweakeroo.fast_block_place;
        if (cfg.fast_place || fast_place_tweak) && self.use_delay > 0 {
            let holding_crystal_or_obi = |s: &ItemStack| {
                matches!(&*s.item, "minecraft:end_crystal" | "minecraft:obsidian")
            };
            let active = fast_place_tweak
                || self.stack_in(self.selected).is_some_and(holding_crystal_or_obi)
                || self.offhand.as_ref().is_some_and(holding_crystal_or_obi);
            if active {
                self.use_delay = 0;
            }
        }
    }

    // =========================================================================
    // 4. Shield Status
    // =========================================================================

    pub(super) fn tick_shield_status(&mut self) {
        let cfg = &self.settings.extras.volt.shield_status;
        let cur = self.shield_cooldown;
        if cfg.enabled && cfg.disable_sound && cur > 0 && self.prev_shield_cooldown == 0 {
            self.play_at_feet("minecraft:item.shield.break");
        }
        self.prev_shield_cooldown = cur;
    }

    fn shield_status_rgb(
        enabled_preset: ShieldColorPreset,
        disabled_preset: ShieldColorPreset,
        interpolate: bool,
        cooldown_ticks: u32,
    ) -> [u8; 3] {
        if cooldown_ticks == 0 {
            return enabled_preset.rgb();
        }
        if !interpolate {
            return disabled_preset.rgb();
        }
        let t = (1.0 - (cooldown_ticks as f32 / 100.0)).clamp(0.0, 1.0);
        let a = disabled_preset.rgb();
        let b = enabled_preset.rgb();
        [
            (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t).round() as u8,
            (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t).round() as u8,
            (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t).round() as u8,
        ]
    }

    fn draw_shield_status_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.shield_status;
        if !cfg.enabled {
            return;
        }
        let main_shield = self
            .stack_in(self.selected)
            .filter(|s| &*s.item == "minecraft:shield");
        let off_shield = self
            .offhand
            .as_ref()
            .filter(|s| &*s.item == "minecraft:shield");
        let Some(shield_stack) = off_shield.or(main_shield) else {
            return;
        };

        let rgb = Self::shield_status_rgb(
            cfg.enabled_color,
            cfg.disabled_color,
            cfg.interpolate_colors,
            self.shield_cooldown,
        );
        let atlas = &self.gui.atlas;

        if cfg.display_mode != ShieldDisplayMode::HudOnly {
            let slot_tint = rgba(
                rgb[0],
                rgb[1],
                rgb[2],
                if self.shield_cooldown > 0 { 110 } else { 70 },
            );
            if off_shield.is_some() {
                let ox = (wi / 2 - 91 - 29 + 3) as f32;
                let oy = (hi - 19) as f32;
                batch.fill(atlas, ox, oy, 16.0, 2.0, slot_tint);
                batch.fill(atlas, ox, oy + 14.0, 16.0, 2.0, slot_tint);
            } else {
                let sx = (wi / 2 - 91 + 3 + self.selected as i32 * 20) as f32;
                let sy = (hi - 19) as f32;
                batch.fill(atlas, sx, sy, 16.0, 2.0, slot_tint);
                batch.fill(atlas, sx, sy + 14.0, 16.0, 2.0, slot_tint);
            }
        }

        if cfg.display_mode == ShieldDisplayMode::ShieldOnly {
            return;
        }

        let bx = (wi / 2 - 91 - 29 - 34) as f32;
        let by = (hi - 23) as f32;
        let bw = 30.0;
        let bh = 22.0;
        let border = rgba(rgb[0], rgb[1], rgb[2], 235);
        batch.fill(atlas, bx, by, bw, bh, rgba(16, 16, 22, 178));
        batch.fill(atlas, bx, by, bw, 1.5, border);
        batch.fill(atlas, bx, by + bh - 1.5, bw, 1.5, border);
        batch.fill(atlas, bx, by, 1.5, bh, border);
        batch.fill(atlas, bx + bw - 1.5, by, 1.5, bh, border);

        self.draw_item(batch, shield_stack, bx + 2.0, by + 3.0);

        if self.shield_cooldown > 0 {
            let secs = self.shield_cooldown as f32 / 20.0;
            let lbl = format!("{secs:.1}s");
            let frac = (1.0 - (self.shield_cooldown as f32 / 100.0)).clamp(0.0, 1.0);
            batch.fill(atlas, bx + 2.0, by + bh - 3.5, (bw - 4.0) * frac, 2.0, border);
            batch.text(atlas, &lbl, bx + 15.0, by + 6.0, border, true);
        } else {
            let active = self.using.is_some_and(|(slot, _)| {
                self.stack_in(slot)
                    .is_some_and(|s| &*s.item == "minecraft:shield")
            });
            let lbl = if active { "BLK" } else { "RDY" };
            batch.text(atlas, lbl, bx + 15.0, by + 6.0, border, true);
        }
    }

    // =========================================================================
    // 5. Uku's Armor and Potion HUD
    // =========================================================================

    fn draw_uku_armor_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.uku_hud;
        if !cfg.enabled || !cfg.armor_enabled {
            return;
        }
        let mut entries: Vec<Option<&ItemStack>> = Vec::with_capacity(5);
        for slot in [0usize, 1, 2, 3] {
            let s = self.armor[slot].as_ref();
            if s.is_some() || !cfg.hide_empty {
                entries.push(s);
            }
        }
        if cfg.show_offhand {
            let s = self.offhand.as_ref();
            if s.is_some() || !cfg.hide_empty {
                entries.push(s);
            }
        }
        if entries.is_empty() {
            return;
        }

        let atlas = &self.gui.atlas;
        let horizontal = cfg.armor_anchor == UkuAnchor::Hotbar;
        let slot_step = match cfg.durability_display {
            UkuDurability::Numeric | UkuDurability::Percentage if !horizontal => 34.0,
            UkuDurability::Numeric | UkuDurability::Percentage if horizontal => 26.0,
            _ => 20.0,
        };
        let n = entries.len() as f32;
        let panel_w = if horizontal {
            n * slot_step + 4.0
        } else if matches!(
            cfg.durability_display,
            UkuDurability::Numeric | UkuDurability::Percentage
        ) {
            52.0
        } else {
            24.0
        };
        let panel_h = if horizontal { 24.0 } else { n * 20.0 + 4.0 };

        let (px, py) = match (cfg.armor_anchor, cfg.armor_side) {
            (UkuAnchor::Hotbar, UkuSide::Left) => ((wi / 2 - 91 - 36) as f32 - panel_w, (hi - 23) as f32),
            (UkuAnchor::Hotbar, UkuSide::Right) => ((wi / 2 + 91 + 8) as f32, (hi - 23) as f32),
            (UkuAnchor::Bottom, UkuSide::Left) => (6.0, (hi - 6) as f32 - panel_h),
            (UkuAnchor::Bottom, UkuSide::Right) => ((wi as f32) - panel_w - 6.0, (hi - 6) as f32 - panel_h),
            (UkuAnchor::Top, UkuSide::Left) => (6.0, 28.0),
            (UkuAnchor::Top, UkuSide::Right) => ((wi as f32) - panel_w - 6.0, 28.0),
        };

        match cfg.box_style {
            UkuStyle::Hotbar => {
                batch.fill(atlas, px, py, panel_w, panel_h, rgba(20, 20, 26, 170));
                let rim = rgba(185, 190, 205, 130);
                batch.fill(atlas, px, py, panel_w, 1.0, rim);
                batch.fill(atlas, px, py + panel_h - 1.0, panel_w, 1.0, rim);
                batch.fill(atlas, px, py, 1.0, panel_h, rim);
                batch.fill(atlas, px + panel_w - 1.0, py, 1.0, panel_h, rim);
            }
            UkuStyle::Rounded => {
                batch.fill(atlas, px + 1.0, py, panel_w - 2.0, panel_h, rgba(16, 16, 24, 185));
                batch.fill(atlas, px, py + 1.0, panel_w, panel_h - 2.0, rgba(16, 16, 24, 185));
            }
            UkuStyle::Flat => {
                batch.fill(atlas, px, py, panel_w, panel_h, rgba(10, 10, 14, 115));
            }
            UkuStyle::None => {}
        }

        for (idx, item_opt) in entries.iter().enumerate() {
            let ix = if horizontal {
                px + 2.0 + idx as f32 * slot_step
            } else {
                px + 4.0
            };
            let iy = if horizontal {
                py + 4.0
            } else {
                py + 2.0 + idx as f32 * 20.0
            };
            let Some(stack) = item_opt else {
                batch.fill(atlas, ix + 1.0, iy + 1.0, 14.0, 14.0, rgba(255, 255, 255, 24));
                continue;
            };

            let max_d = self.items().get(&stack.item).map_or(0, |info| info.max_damage);
            let rem_d = max_d.saturating_sub(stack.damage);
            let pct = if max_d > 0 {
                ((rem_d as f32 / max_d as f32) * 100.0).round() as u8
            } else {
                100
            };
            let low = max_d > 0 && cfg.low_durability_pct > 0 && pct <= cfg.low_durability_pct;
            if low {
                batch.fill(atlas, ix - 1.0, iy - 1.0, 18.0, 18.0, rgba(220, 48, 58, 96));
            }
            self.draw_item(batch, stack, ix, iy);

            if max_d > 0 && cfg.durability_display != UkuDurability::None {
                let ratio = (rem_d as f32 / max_d as f32).clamp(0.0, 1.0);
                let col = if ratio > 0.5 {
                    rgba(96, 232, 128, 255)
                } else if ratio > 0.25 {
                    rgba(255, 212, 72, 255)
                } else {
                    rgba(255, 86, 86, 255)
                };
                match cfg.durability_display {
                    UkuDurability::Bar => {
                        batch.fill(atlas, ix + 2.0, iy + 13.0, 13.0, 2.0, rgba(0, 0, 0, 220));
                        let bw = (13.0 * ratio).round().max(1.0);
                        batch.fill(atlas, ix + 2.0, iy + 13.0, bw, 1.0, col);
                    }
                    UkuDurability::Numeric => {
                        let txt = format!("{rem_d}");
                        if horizontal {
                            batch.text(atlas, &txt, ix, iy - 7.0, col, true);
                        } else {
                            batch.text(atlas, &txt, ix + 20.0, iy + 4.0, col, true);
                        }
                    }
                    UkuDurability::Percentage => {
                        let txt = format!("{pct}%");
                        if horizontal {
                            batch.text(atlas, &txt, ix, iy - 7.0, col, true);
                        } else {
                            batch.text(atlas, &txt, ix + 20.0, iy + 4.0, col, true);
                        }
                    }
                    UkuDurability::None => {}
                }
            }
        }
    }

    fn draw_uku_potion_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.uku_hud;
        if !cfg.enabled || !cfg.potion_enabled {
            return;
        }
        let effects = &self.potions.effects.list;
        if effects.is_empty() {
            return;
        }
        let atlas = &self.gui.atlas;
        let row_h = if cfg.potion_compact { 20.0 } else { 24.0 };
        let card_w = if cfg.potion_compact { 62.0 } else { 112.0 };

        for (visible, eff) in effects.iter().enumerate() {
            let beneficial = brewing::effect(eff.id)
                .is_some_and(|d| d.category == brewing::Category::Beneficial);
            let x = match cfg.potion_anchor {
                horizon_client::extras::UkuPotionAnchor::TopRight => wi as f32 - card_w - 6.0,
                horizon_client::extras::UkuPotionAnchor::TopLeft
                | horizon_client::extras::UkuPotionAnchor::BottomLeft => 6.0,
            };
            let y = match cfg.potion_anchor {
                horizon_client::extras::UkuPotionAnchor::TopRight
                | horizon_client::extras::UkuPotionAnchor::TopLeft => 6.0 + visible as f32 * (row_h + 3.0),
                horizon_client::extras::UkuPotionAnchor::BottomLeft => {
                    hi as f32 - 32.0 - (visible + 1) as f32 * (row_h + 3.0)
                }
            };

            let bg_alpha = if eff.duration >= 0 && eff.duration < 200 && (self.ticks / 6).is_multiple_of(2) {
                125
            } else {
                182
            };
            batch.fill(atlas, x, y, card_w, row_h, rgba(18, 18, 26, bg_alpha));
            let accent = if beneficial {
                rgba(116, 92, 235, 220)
            } else {
                rgba(225, 84, 96, 220)
            };
            batch.fill(atlas, x, y, 2.0, row_h, accent);

            let mut sprite = String::new();
            let path = match eff.id.split_once(':') {
                None => eff.id,
                Some(("minecraft", path)) => path,
                Some((ns, path)) => {
                    sprite.push_str(ns);
                    sprite.push(':');
                    path
                }
            };
            sprite.push_str("mob_effect/");
            sprite.push_str(path);
            let icon_sz = if cfg.potion_compact { 14.0 } else { 18.0 };
            batch.sprite_scaled(
                atlas,
                &sprite,
                x + 5.0,
                y + (row_h - icon_sz) * 0.5,
                icon_sz,
                icon_sz,
                u32::MAX,
            );

            let amp = match eff.amplifier {
                0 => "I",
                1 => "II",
                2 => "III",
                3 => "IV",
                4 => "V",
                _ => "+",
            };
            let secs_total = (eff.duration.max(0) / 20) as u32;
            let time_str = if eff.duration < 0 {
                "**:**".to_owned()
            } else {
                format!("{:02}:{:02}", secs_total / 60, secs_total % 60)
            };

            if cfg.potion_compact {
                batch.text(atlas, &time_str, x + 23.0, y + 6.0, rgba(235, 238, 246, 255), true);
            } else {
                let short_name = eff
                    .id
                    .strip_prefix("minecraft:")
                    .unwrap_or(eff.id)
                    .replace('_', " ");
                let title = if eff.amplifier > 0 {
                    format!("{short_name} {amp}")
                } else {
                    short_name
                };
                batch.text(atlas, &title, x + 27.0, y + 3.0, rgba(240, 242, 250, 255), true);
                let time_col = if eff.duration >= 0 && eff.duration < 200 {
                    rgba(255, 120, 120, 255)
                } else {
                    rgba(180, 186, 206, 255)
                };
                batch.text(atlas, &time_str, x + 27.0, y + 13.0, time_col, true);
            }
        }
    }

    // =========================================================================
    // 6. Core Tick & Combat / World / Utility Events for All 84 Mods
    // =========================================================================

    pub(super) fn volt_push_toast(&mut self, msg: impl Into<String>) {
        self.extras.volt.status_toast = Some((msg.into(), self.ticks + 60));
    }

    pub(super) fn volt_effective_hide_gui(&self) -> bool {
        self.ui.hide_gui
            || (self.settings.extras.volt.photo_mode.enabled
                && self.settings.extras.volt.photo_mode.active)
            || (self.extras.volt.timelapse_active
                && self.settings.extras.volt.cinematic_timelapse.hide_hud)
    }

    #[cfg(target_os = "android")]
    pub(super) fn sync_volt_touch_ui(&mut self) {
        self.pad
            .touch_ui
            .sync_volt_extras(&self.settings.extras.volt);
        self.pad.touch_ui.volt_timelapse_status = if let Some(until) =
            self.extras.volt.timelapse_countdown_until
        {
            let secs = until
                .saturating_duration_since(Instant::now())
                .as_secs_f32()
                .ceil() as u32;
            Some(format!("In {}s...", secs.max(1)))
        } else if self.extras.volt.timelapse_active {
            Some("Active".to_owned())
        } else {
            None
        };
        let rows: Vec<horizon_client::touch_ui::VoltWaypointUiRow> = self
            .extras
            .volt
            .waypoints
            .iter()
            .map(|w| horizon_client::touch_ui::VoltWaypointUiRow {
                name: w.name.clone(),
                coords: [
                    w.pos[0].round() as i32,
                    w.pos[1].round() as i32,
                    w.pos[2].round() as i32,
                ],
                dim_label: match w.dim {
                    Dim::Overworld => "Overworld",
                    Dim::Nether => "Nether",
                    Dim::End => "The End",
                },
                is_death: w.is_death,
                color: w.color,
            })
            .collect();
        if self.pad.touch_ui.volt_waypoints_snapshot != rows {
            self.pad.touch_ui.volt_waypoints_snapshot = rows;
        }
    }

    #[cfg(not(target_os = "android"))]
    pub(super) fn sync_volt_touch_ui(&mut self) {}

    pub(super) fn volt_on_attack_hit(&mut self, target_id: Option<u32>, target_pos: Option<[f64; 3]>) {
        if let (Some(p), Some(tp)) = (self.player.as_ref(), target_pos) {
            let dx = tp[0] - p.position.x;
            let dy = tp[1] - (p.position.y + 1.62);
            let dz = tp[2] - p.position.z;
            self.extras.volt.last_reach_blocks = Some((dx * dx + dy * dy + dz * dz).sqrt() as f32);
            if p.fall_distance > 0.0 && !p.ground.on_ground && !p.sprinting {
                self.extras.volt.crit_ring_until_tick = self.ticks + 12;
            }
        }
        self.extras.volt.hit_marker_until_tick = self.ticks + 6;
        if self.settings.extras.volt.session_stats.enabled {
            self.extras.volt.session_damage_dealt += 2.0;
        }
        if let Some(tid) = target_id {
            if self.extras.volt.last_hit_target == Some(tid)
                && self.ticks.saturating_sub(self.extras.volt.last_hit_tick) <= 50
            {
                self.extras.volt.combo_count = self.extras.volt.combo_count.saturating_add(1);
            } else {
                self.extras.volt.combo_count = 1;
            }
            self.extras.volt.last_hit_target = Some(tid);
            self.extras.volt.last_hit_tick = self.ticks;
        }
    }

    pub(super) fn volt_on_totem_pop(&mut self, entity_id: i32, own: bool) {
        let cfg = &self.settings.extras.volt.totem_pop_counter;
        let who = if own {
            "You".to_owned()
        } else {
            self.mobs
                .iter()
                .find(|m| m.id as i32 == entity_id)
                .map(|m| m.kind.id.strip_prefix("minecraft:").unwrap_or(&m.kind.id).to_owned())
                .unwrap_or_else(|| format!("Entity #{entity_id}"))
        };
        if cfg.enabled && ((own && cfg.track_self) || (!own && cfg.track_enemies)) {
            if cfg.show_chat_feed {
                self.ui.chat.add(
                    &self.gui.atlas,
                    Text::literal(format!("[Volt+] {who} popped a Totem of Undying!")),
                    None,
                    ChatSource::Client,
                    self.ticks,
                    self.screen == Screen::Chat,
                );
            }
            self.extras
                .volt
                .totem_pop_feed
                .push_front((who, self.ticks));
            while self.extras.volt.totem_pop_feed.len() > 4 {
                self.extras.volt.totem_pop_feed.pop_back();
            }
        }
        if own {
            self.extras.volt.totem_pops_total = self.extras.volt.totem_pops_total.saturating_add(1);
            if self.settings.extras.volt.item_scroller.enabled
                && self.settings.extras.volt.item_scroller.quick_offhand_totem
            {
                self.volt_quick_offhand_totem();
            }
        }
    }

    pub(super) fn volt_add_waypoint(&mut self, name: &str, is_death: bool) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let color = if is_death {
            rgba(255, 82, 82, 255)
        } else {
            rgba(105, 225, 255, 255)
        };
        self.extras.volt.waypoints.push(VoltWaypoint {
            name: name.to_owned(),
            pos: [p.position.x, p.position.y + 1.0, p.position.z],
            dim: self.dim,
            is_death,
            color,
        });
        if self.extras.volt.waypoints.len() > 24 {
            self.extras.volt.waypoints.remove(0);
        }
    }

    pub(super) fn volt_on_player_death(&mut self) {
        let ar = &self.settings.extras.volt.auto_respawn;
        let wp = &self.settings.extras.volt.waypoints;
        let dh = &self.settings.extras.volt.death_history;
        if let Some(p) = self.player.as_ref() {
            let x = p.position.x.round() as i32;
            let y = p.position.y.round() as i32;
            let z = p.position.z.round() as i32;
            let label = format!("Death ({x},{y},{z})");
            self.extras
                .volt
                .death_history
                .push_front((label.clone(), [x, y, z], self.dim));
            let max_d = usize::from(dh.keep_count.clamp(3, 12));
            while self.extras.volt.death_history.len() > max_d {
                self.extras.volt.death_history.pop_back();
            }
            if (ar.enabled && ar.log_death_coords) || (dh.enabled && dh.auto_log_chat) {
                self.ui.chat.add(
                    &self.gui.atlas,
                    Text::literal(format!("You died at {x}, {y}, {z} ({:?})", self.dim)),
                    None,
                    ChatSource::Client,
                    self.ticks,
                    self.screen == Screen::Chat,
                );
            }
            if (ar.enabled && ar.save_death_waypoint)
                || (wp.enabled && wp.auto_death_waypoint)
                || dh.enabled
            {
                self.volt_add_waypoint(&label, true);
            }
        }
    }

    pub(super) fn volt_swap_chestplate_elytra(&mut self) {
        let want_elytra = self.armor[1]
            .as_ref()
            .is_none_or(|s| &*s.item != "minecraft:elytra");
        let idx = self.inventory.iter().position(|slot| {
            let Some(s) = slot.as_ref() else {
                return false;
            };
            if want_elytra {
                &*s.item == "minecraft:elytra"
            } else {
                s.item.contains("chestplate")
            }
        });
        if let Some(i) = idx {
            let cur = self.armor[1].take();
            self.armor[1] = self.inventory[i].take();
            self.inventory[i] = cur;
            self.volt_push_toast(if want_elytra {
                "Equipped Elytra"
            } else {
                "Equipped Chestplate"
            });
        }
    }

    pub(super) fn volt_quick_offhand_totem(&mut self) {
        if self
            .offhand
            .as_ref()
            .is_some_and(|s| &*s.item == "minecraft:totem_of_undying")
        {
            return;
        }
        if let Some(idx) = self.inventory.iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|s| &*s.item == "minecraft:totem_of_undying")
        }) {
            let prev = self.offhand.take();
            self.offhand = self.inventory[idx].take();
            self.inventory[idx] = prev;
            self.volt_push_toast("Totem -> Offhand");
        }
    }

    pub(super) fn volt_extras_quick_action(&mut self, action: VoltQuickAction) {
        match action {
            VoltQuickAction::ToggleZoom => {
                self.settings.extras.volt.zoom.enabled = true;
                self.settings.extras.volt.zoom.active = !self.settings.extras.volt.zoom.active;
                let on = self.settings.extras.volt.zoom.active;
                self.volt_push_toast(if on { "Zoom ON" } else { "Zoom OFF" });
            }
            VoltQuickAction::ToggleFreecam => {
                let on = !self.extras.volt.freecam_active;
                self.settings.extras.volt.freecam.enabled = on;
                self.settings.extras.volt.freecam.active = on;
                self.extras.volt.freecam_active = on;
                if on && let Some(p) = self.player.as_ref() {
                    self.extras.volt.freecam_pos =
                        [p.position.x, p.position.y + 1.62, p.position.z];
                    self.extras.volt.freecam_yaw = self.yaw;
                    self.extras.volt.freecam_pitch = self.pitch;
                }
                self.volt_push_toast(if on { "Freecam ON" } else { "Freecam OFF" });
            }
            VoltQuickAction::ToggleFreelook => {
                let on = !self.extras.volt.freelook_active;
                self.settings.extras.volt.freelook.enabled = on;
                self.settings.extras.volt.freelook.active = on;
                self.extras.volt.freelook_active = on;
                if on {
                    self.extras.volt.freelook_yaw = self.yaw;
                    self.extras.volt.freelook_pitch = self.pitch;
                }
                self.volt_push_toast(if on { "Freelook ON" } else { "Freelook OFF" });
            }
            VoltQuickAction::ToggleAutoSprint => {
                let next = !self.settings.extras.volt.auto_sprint.enabled;
                self.settings.extras.volt.auto_sprint.enabled = next;
                self.volt_push_toast(if next { "Auto-Sprint ON" } else { "Auto-Sprint OFF" });
            }
            VoltQuickAction::ToggleFullbright => {
                let next = !self.settings.extras.volt.fullbright.enabled;
                self.settings.extras.volt.fullbright.enabled = next;
                self.apply_ui_settings();
                self.volt_push_toast(if next { "Fullbright ON" } else { "Fullbright OFF" });
            }
            VoltQuickAction::SwapChestElytra => self.volt_swap_chestplate_elytra(),
            VoltQuickAction::QuickOffhandTotem => self.volt_quick_offhand_totem(),
            VoltQuickAction::AddWaypointHere => {
                let count = self.extras.volt.waypoints.len() + 1;
                self.volt_add_waypoint(&format!("WP #{count}"), false);
                self.volt_push_toast(format!("Waypoint #{count} saved"));
            }
            VoltQuickAction::ClearDeathWaypoints => {
                self.extras.volt.waypoints.retain(|w| !w.is_death);
                self.volt_push_toast("Cleared death markers");
            }
            VoltQuickAction::RunChatMacro(slot) => {
                let msg = match slot {
                    1 => "/home",
                    2 => "/spawn",
                    _ => "gg",
                };
                if let Some(cmd) = msg.strip_prefix('/') {
                    if self.remote.is_some() {
                        self.send_command(cmd);
                    } else {
                        let _ = self.run_command(cmd);
                    }
                } else if self.remote.is_some() {
                    self.send_chat(msg);
                } else {
                    self.ui.chat.add(
                        &self.gui.atlas,
                        Text::literal(msg),
                        None,
                        ChatSource::Player,
                        self.ticks,
                        self.screen == Screen::Chat,
                    );
                }
            }
            VoltQuickAction::CopyCoords => {
                if let Some(p) = self.player.as_ref() {
                    let msg = format!(
                        "Coords: {:.1}, {:.1}, {:.1} ({:?})",
                        p.position.x, p.position.y, p.position.z, self.dim
                    );
                    self.ui.chat.add(
                        &self.gui.atlas,
                        Text::literal(msg),
                        None,
                        ChatSource::Client,
                        self.ticks,
                        self.screen == Screen::Chat,
                    );
                    self.volt_push_toast("Coords logged to chat");
                }
            }
            VoltQuickAction::ToggleChunkBorders => {
                self.settings.extras.volt.chunk_hitboxes.enabled = true;
                self.settings.extras.volt.chunk_hitboxes.chunk_borders =
                    !self.settings.extras.volt.chunk_hitboxes.chunk_borders;
            }
            VoltQuickAction::ToggleHitboxes => {
                self.settings.extras.volt.chunk_hitboxes.enabled = true;
                self.settings.extras.volt.chunk_hitboxes.entity_hitboxes =
                    !self.settings.extras.volt.chunk_hitboxes.entity_hitboxes;
            }
            VoltQuickAction::OpenModKeyPicker { .. }
            | VoltQuickAction::OpenWaypointManager
            | VoltQuickAction::OpenBlockOutlineColor => {}
            VoltQuickAction::DeleteWaypoint(idx) => {
                if idx < self.extras.volt.waypoints.len() {
                    let removed = self.extras.volt.waypoints.remove(idx);
                    self.volt_push_toast(format!("Deleted {}", removed.name));
                }
            }
            VoltQuickAction::ClearAllWaypoints => {
                self.extras.volt.waypoints.clear();
                self.volt_push_toast("Cleared all waypoints");
            }
            VoltQuickAction::CycleWaypointColor(idx) => {
                if let Some(wp) = self.extras.volt.waypoints.get_mut(idx) {
                    const PALETTE: [u32; 6] = [
                        0xFF69_E1FF,
                        0xFFFF_5252,
                        0xFF5A_F28E,
                        0xFFFF_D248,
                        0xFFD6_84FF,
                        0xFFFF_FFFF,
                    ];
                    let pos = PALETTE.iter().position(|&c| c == wp.color).unwrap_or(0);
                    wp.color = PALETTE[(pos + 1) % PALETTE.len()];
                }
            }
            VoltQuickAction::ShareWaypoint(idx) => {
                if let Some(wp) = self.extras.volt.waypoints.get(idx) {
                    let msg = format!(
                        "[Waypoint] {}: {:.0}, {:.0}, {:.0} ({:?})",
                        wp.name, wp.pos[0], wp.pos[1], wp.pos[2], wp.dim
                    );
                    self.ui.chat.add(
                        &self.gui.atlas,
                        Text::literal(msg),
                        None,
                        ChatSource::Client,
                        self.ticks,
                        self.screen == Screen::Chat,
                    );
                    self.volt_push_toast("Waypoint shared to chat");
                }
            }
            VoltQuickAction::ResetSessionStats => {
                self.extras.volt.session_started = Some(Instant::now());
                self.extras.volt.session_blocks_mined = 0;
                self.extras.volt.session_blocks_placed = 0;
                self.extras.volt.session_mob_kills = 0;
                self.extras.volt.session_damage_dealt = 0.0;
                self.volt_push_toast("Session stats reset");
            }
            VoltQuickAction::SetRulerPointA => {
                if let Some(hit) = self.debug_block_hit().or_else(|| {
                    self.player.as_ref().map(|p| {
                        [
                            p.position.x.floor() as i32,
                            p.position.y.floor() as i32,
                            p.position.z.floor() as i32,
                        ]
                    })
                }) {
                    self.settings.extras.volt.litematica_grid.enabled = true;
                    self.extras.volt.ruler_a = Some(hit);
                    self.volt_push_toast(format!("Ruler A: {},{},{}", hit[0], hit[1], hit[2]));
                }
            }
            VoltQuickAction::SetRulerPointB => {
                if let Some(hit) = self.debug_block_hit().or_else(|| {
                    self.player.as_ref().map(|p| {
                        [
                            p.position.x.floor() as i32,
                            p.position.y.floor() as i32,
                            p.position.z.floor() as i32,
                        ]
                    })
                }) {
                    self.settings.extras.volt.litematica_grid.enabled = true;
                    self.extras.volt.ruler_b = Some(hit);
                    self.volt_push_toast(format!("Ruler B: {},{},{}", hit[0], hit[1], hit[2]));
                }
            }
            VoltQuickAction::ClearRuler => {
                self.extras.volt.ruler_a = None;
                self.extras.volt.ruler_b = None;
                self.volt_push_toast("Ruler cleared");
            }
            VoltQuickAction::SetHomeWaypoint => {
                self.settings.extras.volt.waypoints.enabled = true;
                self.volt_add_waypoint("Home", false);
                self.volt_push_toast("Home waypoint saved");
            }
            VoltQuickAction::TogglePhotoMode => {
                self.settings.extras.volt.photo_mode.enabled = true;
                let next = !self.settings.extras.volt.photo_mode.active;
                self.settings.extras.volt.photo_mode.active = next;
                self.volt_push_toast(if next {
                    "Photo Mode ON"
                } else {
                    "Photo Mode OFF"
                });
            }
            VoltQuickAction::ToggleJei => {
                self.settings.extras.volt.jei.enabled = true;
                self.extras.volt.jei_open = !self.extras.volt.jei_open;
                if self.extras.volt.jei_open {
                    if self.screen == Screen::Playing {
                        self.capture(false);
                    }
                    self.volt_push_toast("JEI Browser ON");
                } else {
                    self.extras.volt.jei_search_focused = false;
                    if self.screen == Screen::Playing {
                        self.capture(true);
                    }
                    self.volt_push_toast("JEI Browser OFF");
                }
            }
            VoltQuickAction::StartStopTimelapse => {
                self.volt_toggle_timelapse();
            }
        }
        #[cfg(target_os = "android")]
        self.sync_volt_touch_ui();
    }

    pub(super) fn volt_start_timelapse(&mut self) {
        self.settings.extras.volt.cinematic_timelapse.enabled = true;
        if let Some(p) = self.player.as_ref() {
            self.extras.volt.timelapse_anchor =
                [p.position.x, p.position.y + 4.0, p.position.z];
            self.extras.volt.timelapse_base_yaw = self.yaw;
            self.extras.volt.timelapse_base_pitch = self.pitch.clamp(-35.0, 45.0);
        }
        let delay = self
            .settings
            .extras
            .volt
            .cinematic_timelapse
            .start_delay_secs;
        if delay == 0 {
            self.extras.volt.timelapse_countdown_until = None;
            self.extras.volt.timelapse_active = true;
            self.extras.volt.timelapse_started = Some(Instant::now());
        } else {
            self.extras.volt.timelapse_active = false;
            self.extras.volt.timelapse_started = None;
            self.extras.volt.timelapse_countdown_until =
                Some(Instant::now() + Duration::from_secs(u64::from(delay)));
        }
    }

    pub(super) fn volt_stop_timelapse(&mut self) {
        self.extras.volt.timelapse_active = false;
        self.extras.volt.timelapse_countdown_until = None;
        self.extras.volt.timelapse_started = None;
    }

    pub(super) fn volt_sync_timelapse_from_settings(&mut self, was_enabled: bool) {
        let now_enabled = self.settings.extras.volt.cinematic_timelapse.enabled;
        if !was_enabled && now_enabled {
            self.volt_start_timelapse();
        } else if was_enabled && !now_enabled {
            self.volt_stop_timelapse();
        }
    }

    pub(super) fn volt_toggle_timelapse(&mut self) {
        let active_or_counting = self.extras.volt.timelapse_active
            || self.extras.volt.timelapse_countdown_until.is_some();
        if active_or_counting {
            self.volt_stop_timelapse();
            return;
        }
        self.volt_start_timelapse();
    }

    pub(super) fn volt_trigger_mod_use(&mut self, mod_id: u8) {
        match mod_id {
            4 => self.volt_extras_quick_action(VoltQuickAction::ToggleFreelook),
            7 => self.volt_extras_quick_action(VoltQuickAction::QuickOffhandTotem),
            10 => self.volt_extras_quick_action(VoltQuickAction::AddWaypointHere),
            16 => self.volt_extras_quick_action(VoltQuickAction::ToggleZoom),
            19 => self.volt_extras_quick_action(VoltQuickAction::CopyCoords),
            21 => self.volt_extras_quick_action(VoltQuickAction::ToggleChunkBorders),
            25 => self.volt_extras_quick_action(VoltQuickAction::ToggleFreecam),
            28 => self.volt_extras_quick_action(VoltQuickAction::RunChatMacro(1)),
            31 => self.volt_extras_quick_action(VoltQuickAction::SwapChestElytra),
            33 => self.volt_extras_quick_action(VoltQuickAction::ToggleAutoSprint),
            36 => self.volt_extras_quick_action(VoltQuickAction::ToggleFullbright),
            67 => self.volt_extras_quick_action(VoltQuickAction::ResetSessionStats),
            72 => {
                if self.extras.volt.ruler_a.is_none() {
                    self.volt_extras_quick_action(VoltQuickAction::SetRulerPointA);
                } else if self.extras.volt.ruler_b.is_none() {
                    self.volt_extras_quick_action(VoltQuickAction::SetRulerPointB);
                } else {
                    self.volt_extras_quick_action(VoltQuickAction::ClearRuler);
                }
            }
            73 => self.volt_extras_quick_action(VoltQuickAction::SetHomeWaypoint),
            79 => self.volt_extras_quick_action(VoltQuickAction::TogglePhotoMode),
            80 => self.volt_extras_quick_action(VoltQuickAction::ToggleJei),
            81 => self.volt_extras_quick_action(VoltQuickAction::StartStopTimelapse),
            other => {
                let cur = volt_mod_enabled(&self.settings.extras.volt, other);
                set_volt_mod_enabled(&mut self.settings.extras.volt, other, !cur);
                let title = VOLT_MODS
                    .get(usize::from(other))
                    .map_or("Mod", |m| m.title);
                self.volt_push_toast(format!("{title} {}", if !cur { "ON" } else { "OFF" }));
            }
        }
    }

    fn volt_set_hold_mod_active(&mut self, mod_id: u8, active: bool) {
        set_volt_mod_enabled(&mut self.settings.extras.volt, mod_id, active);
        match mod_id {
            4 => self.settings.extras.volt.freelook.active = active,
            16 => self.settings.extras.volt.zoom.active = active,
            25 => self.settings.extras.volt.freecam.active = active,
            79 => self.settings.extras.volt.photo_mode.active = active,
            80 => {
                self.extras.volt.jei_open = active;
                if !active {
                    self.extras.volt.jei_search_focused = false;
                }
                if self.screen == Screen::Playing {
                    self.capture(!active);
                }
            }
            _ => {}
        }
    }

    pub(super) fn volt_handle_key_token(
        &mut self,
        token: &str,
        pressed: bool,
        repeat: bool,
    ) -> bool {
        if token.is_empty() {
            return false;
        }
        let mut matched = false;
        for id in 0..VOLT_MOD_COUNT {
            let bind = self.settings.extras.volt.binds.0[id].clone();
            if bind.toggle_key == token {
                matched = true;
                match bind.mode {
                    VoltKeybindMode::Tap => {
                        if pressed && !repeat {
                            let cur = volt_mod_enabled(&self.settings.extras.volt, id as u8);
                            set_volt_mod_enabled(&mut self.settings.extras.volt, id as u8, !cur);
                            if id == 4 {
                                self.settings.extras.volt.freelook.active = !cur;
                            } else if id == 16 {
                                self.settings.extras.volt.zoom.active = !cur;
                            } else if id == 25 {
                                self.settings.extras.volt.freecam.active = !cur;
                            } else if id == 36 {
                                self.apply_ui_settings();
                            } else if id == 79 {
                                self.settings.extras.volt.photo_mode.active = !cur;
                            } else if id == 80 {
                                self.extras.volt.jei_open = !cur;
                                if !self.extras.volt.jei_open {
                                    self.extras.volt.jei_search_focused = false;
                                }
                                if self.screen == Screen::Playing {
                                    self.capture(!self.extras.volt.jei_open);
                                }
                            }
                            let title = VOLT_MODS[id].title;
                            self.volt_push_toast(format!(
                                "{title} {}",
                                if !cur { "ON" } else { "OFF" }
                            ));
                            self.settings.extras.save(std::path::Path::new("."));
                        }
                    }
                    VoltKeybindMode::Hold => {
                        if !repeat {
                            self.volt_set_hold_mod_active(id as u8, pressed);
                            if id == 36 {
                                self.apply_ui_settings();
                            }
                        }
                    }
                }
            }
            if bind.use_key == token {
                matched = true;
                match bind.mode {
                    VoltKeybindMode::Tap => {
                        if pressed && !repeat {
                            self.volt_trigger_mod_use(id as u8);
                            self.settings.extras.save(std::path::Path::new("."));
                        }
                    }
                    VoltKeybindMode::Hold => {
                        if !repeat {
                            if id == 81 {
                                if pressed && !self.extras.volt.timelapse_active {
                                    self.volt_toggle_timelapse();
                                } else if !pressed && self.extras.volt.timelapse_active {
                                    self.volt_toggle_timelapse();
                                }
                            } else {
                                self.volt_set_hold_mod_active(id as u8, pressed);
                            }
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "android")]
        if matched {
            self.sync_volt_touch_ui();
        }
        matched
    }

    pub(super) fn volt_drop_guard_allows(&mut self, stack: &ItemStack) -> bool {
        let cfg = &self.settings.extras.volt.drop_guard;
        if !cfg.enabled {
            return true;
        }
        let s = &*stack.item;
        let is_rare = s.contains("netherite")
            || s.contains("diamond")
            || s.contains("elytra")
            || s.contains("totem")
            || s.contains("mace")
            || s.contains("trident")
            || s.contains("beacon")
            || s.contains("shulker_box")
            || stack
                .extra
                .as_ref()
                .is_some_and(|e| !e.enchantments.is_empty());
        if !cfg.double_tap_bypass && !is_rare {
            return true;
        }
        let win = Duration::from_millis(1200);
        if let Some((ref prev_item, at)) = self.extras.volt.drop_confirm_pending
            && prev_item == s
            && at.elapsed() <= win
        {
            self.extras.volt.drop_confirm_pending = None;
            return true;
        }
        self.extras.volt.drop_confirm_pending = Some((s.to_owned(), Instant::now()));
        let short = s.strip_prefix("minecraft:").unwrap_or(s);
        self.volt_push_toast(format!("Press Drop again to drop {short}"));
        false
    }

    pub(super) fn volt_timelapse_camera(&self) -> Option<(glam::DVec3, f32, f32)> {
        if !self.extras.volt.timelapse_active {
            return None;
        }
        let cfg = &self.settings.extras.volt.cinematic_timelapse;
        let t = self
            .extras
            .volt
            .timelapse_started
            .map_or(0.0, |s| s.elapsed().as_secs_f64());
        let spd = f64::from(cfg.orbit_speed_deg.max(5)) / 45.0;
        let a = self.extras.volt.timelapse_anchor;
        match cfg.path_mode {
            TimelapsePathMode::Orbit => {
                let rad = 10.0;
                let ang = t * 0.28 * spd + f64::from(self.extras.volt.timelapse_base_yaw.to_radians());
                let cam = glam::DVec3::new(
                    a[0] + rad * ang.cos(),
                    a[1] + 3.0 + (t * 0.4 * spd).sin() * 1.2,
                    a[2] + rad * ang.sin(),
                );
                let dx = a[0] - cam.x;
                let dy = (a[1] - 1.5) - cam.y;
                let dz = a[2] - cam.z;
                let yaw = (-(dx)).atan2(dz).to_degrees() as f32;
                let horiz = (dx * dx + dz * dz).sqrt().max(0.001);
                let pitch = (-(dy / horiz).atan()).to_degrees() as f32;
                Some((cam, yaw, pitch.clamp(-85.0, 85.0)))
            }
            TimelapsePathMode::DollyForward => {
                let sy = f64::from(self.extras.volt.timelapse_base_yaw.to_radians()).sin();
                let cy = f64::from(self.extras.volt.timelapse_base_yaw.to_radians()).cos();
                let dist = t * 1.6 * spd;
                let cam = glam::DVec3::new(a[0] - sy * dist, a[1] + 1.0, a[2] + cy * dist);
                Some((
                    cam,
                    self.extras.volt.timelapse_base_yaw,
                    self.extras.volt.timelapse_base_pitch,
                ))
            }
            TimelapsePathMode::PanSide => {
                let sy = f64::from(self.extras.volt.timelapse_base_yaw.to_radians()).sin();
                let cy = f64::from(self.extras.volt.timelapse_base_yaw.to_radians()).cos();
                let dist = t * 1.4 * spd;
                let cam = glam::DVec3::new(a[0] + cy * dist, a[1] + 1.5, a[2] + sy * dist);
                Some((
                    cam,
                    self.extras.volt.timelapse_base_yaw,
                    self.extras.volt.timelapse_base_pitch,
                ))
            }
            TimelapsePathMode::StaticTripod => {
                let cam = glam::DVec3::new(a[0], a[1] + 1.5, a[2]);
                Some((
                    cam,
                    self.extras.volt.timelapse_base_yaw,
                    self.extras.volt.timelapse_base_pitch,
                ))
            }
        }
    }

    pub(super) fn volt_extras_tick(&mut self) {
        self.audio.muffle = self.settings.extras.volt.sound_muffler.clone();
        self.ui.chat.volt_chat = self.settings.extras.volt.chat_tweaks.clone();
        self.ui.chat.volt_mention = self.settings.extras.volt.chat_mention.clone();
        self.ui.chat.volt_player_name = self.own_profile_name();

        // Hit Color sync
        let hc = &self.settings.extras.volt.hit_color;
        if hc.enabled {
            horizon_client::mobs::render::set_volt_hit_color(
                Some(hc.color.rgb()),
                hc.intensity_pct,
                hc.flash_armor,
            );
        } else {
            horizon_client::mobs::render::set_volt_hit_color(None, 0, false);
        }

        // Chat Mention Ping chime
        if self.ui.chat.volt_mention_triggered {
            self.ui.chat.volt_mention_triggered = false;
            if self.settings.extras.volt.chat_mention.play_ping_chime {
                self.play_at_feet("minecraft:block.note_block.bell");
            }
            if self.settings.extras.volt.chat_mention.show_popup_banner {
                let preview: String = self.ui.chat.volt_mention_last.chars().take(42).collect();
                self.extras.volt.mention_banner = Some((preview, self.ticks + 80));
            }
        }

        // Cinematic Time Lapse countdown -> active transition
        if let Some(until) = self.extras.volt.timelapse_countdown_until {
            if Instant::now() >= until {
                self.extras.volt.timelapse_countdown_until = None;
                self.extras.volt.timelapse_active = true;
                self.extras.volt.timelapse_started = Some(Instant::now());
            }
            self.sync_volt_touch_ui();
        }

        if let Some(p) = self.player.as_ref() {
            let cur = [p.position.x, p.position.y, p.position.z];
            if let Some(prev) = self.extras.volt.last_player_pos {
                let dx = cur[0] - prev[0];
                let dz = cur[2] - prev[2];
                let inst = (dx * dx + dz * dz).sqrt() * 20.0;
                self.extras.volt.speed_bps = self.extras.volt.speed_bps * 0.75 + inst * 0.25;
                self.extras.volt.camera_roll_deg = 0.0;
            }
            self.extras.volt.last_player_pos = Some(cur);

            // Biome Title Banner
            if self.settings.extras.volt.biome_title.enabled && self.ticks.is_multiple_of(5) {
                let bx = p.position.x.floor() as i32;
                let by = p.position.y.floor() as i32;
                let bz = p.position.z.floor() as i32;
                if let Some(chunk) = self.world.chunks.get(&(bx >> 4, bz >> 4)) {
                    let biome = self.res.biomes().get(chunk.biome(bx, by, bz));
                    let raw = biome.name.strip_prefix("minecraft:").unwrap_or(&biome.name);
                    if !raw.is_empty() && raw != self.extras.volt.last_biome {
                        let first_seen = self.extras.volt.last_biome.is_empty();
                        self.extras.volt.last_biome = raw.to_owned();
                        let cd = 100_u64;
                        if !first_seen
                            && self.ticks.saturating_sub(self.extras.volt.last_biome_banner_tick) >= cd
                        {
                            let pretty = raw
                                .split('_')
                                .map(|w| {
                                    let mut ch = w.chars();
                                    match ch.next() {
                                        Some(c) => format!("{}{}", c.to_ascii_uppercase(), ch.as_str()),
                                        None => String::new(),
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            let dur = u64::from(self.settings.extras.volt.biome_title.display_secs.max(1)) * 20;
                            self.extras.volt.biome_banner = Some((pretty, self.ticks + dur));
                            self.extras.volt.last_biome_banner_tick = self.ticks;
                        }
                    }
                }
            }

            // Offhand Torch auto-swap in dark caves
            if self.settings.extras.volt.offhand_torch.enabled
                && self.settings.extras.volt.offhand_torch.auto_place_dark
                && self.ticks.is_multiple_of(10)
                && self.offhand.is_none()
            {
                let l = self.world.light(
                    p.position.x.floor() as i32,
                    p.position.y.floor() as i32,
                    p.position.z.floor() as i32,
                );
                let eff_light = (l & 15).max(l >> 4);
                if eff_light <= self.settings.extras.volt.offhand_torch.light_threshold {
                    if let Some(idx) = self.inventory.iter().position(|slot| {
                        slot.as_ref().is_some_and(|s| {
                            matches!(
                                &*s.item,
                                "minecraft:torch" | "minecraft:soul_torch" | "minecraft:lantern"
                            )
                        })
                    }) {
                        self.offhand = self.inventory[idx].take();
                        self.volt_push_toast("Torch -> Offhand");
                    }
                }
            }
        }

        // XP Tracker session tracking
        if self.settings.extras.volt.xp_tracker.enabled {
            let total_xp = self.experience.level as i32 * 100
                + (self.experience.progress * 100.0).round() as i32;
            if self.extras.volt.xp_session_start.is_none() {
                self.extras.volt.xp_session_start = Some(Instant::now());
            }
            if let Some(prev) = self.extras.volt.xp_last_total {
                if total_xp > prev {
                    self.extras.volt.xp_gained_session += total_xp - prev;
                }
            }
            self.extras.volt.xp_last_total = Some(total_xp);
        }

        // Session Stats timer init
        if self.settings.extras.volt.session_stats.enabled
            && self.extras.volt.session_started.is_none()
        {
            self.extras.volt.session_started = Some(Instant::now());
        }

        // Hotbar Restock (auto-refill consumed held stack from main inventory)
        if self.settings.extras.volt.hotbar_restock.enabled {
            let cur_item = self.stack_in(self.selected).map(|s| s.item.to_string());
            if self.extras.volt.last_held_slot == self.selected
                && cur_item.is_none()
                && let Some(prev_name) = self.extras.volt.last_held_item.clone()
                && let Some(src_idx) = (9..self.inventory.len().min(36)).find(|&i| {
                    self.inventory[i]
                        .as_ref()
                        .is_some_and(|s| &*s.item == prev_name)
                })
            {
                self.inventory[self.selected] = self.inventory[src_idx].take();
                if self.settings.extras.volt.hotbar_restock.show_toast {
                    let short = prev_name.strip_prefix("minecraft:").unwrap_or(&prev_name);
                    self.volt_push_toast(format!("Restocked {short}"));
                }
            }
            self.extras.volt.last_held_slot = self.selected;
            self.extras.volt.last_held_item = cur_item;
        }

        // Low Durability Sound Alert
        if self.settings.extras.volt.durability_warn.enabled
            && self.settings.extras.volt.durability_warn.play_chime
            && self.ticks.saturating_sub(self.extras.volt.last_durability_warn_tick) >= 120
        {
            let thresh = self.settings.extras.volt.durability_warn.warn_pct;
            let low_any = self
                .armor
                .iter()
                .flatten()
                .chain(self.stack_in(self.selected))
                .any(|s| {
                    let max_d = self.items().get(&s.item).map_or(0, |i| i.max_damage);
                    if max_d == 0 {
                        return false;
                    }
                    let rem = max_d.saturating_sub(s.damage);
                    ((rem as f32 / max_d as f32) * 100.0).round() as u8 <= thresh
                });
            if low_any {
                self.extras.volt.last_durability_warn_tick = self.ticks;
                self.play_at_feet("minecraft:block.note_block.pling");
            }
        }

        // Auto-Sprint
        let move_vec = self.pad_move();
        if self.settings.extras.volt.auto_sprint.enabled
            && let Some(p) = self.player.as_mut()
            && !p.shift_key_down
            && p.health.food.food > 6
            && (self.keys.contains(&KeyCode::KeyW) || move_vec.is_some_and(|[_, y]| y > 0.2))
        {
            p.sprinting = true;
        }

        // Freecam sync + fly movement
        let fc_enabled =
            self.settings.extras.volt.freecam.enabled && self.settings.extras.volt.freecam.active;
        if fc_enabled != self.extras.volt.freecam_active {
            self.extras.volt.freecam_active = fc_enabled;
            if fc_enabled && let Some(p) = self.player.as_ref() {
                self.extras.volt.freecam_pos = [p.position.x, p.position.y + 1.62, p.position.z];
                self.extras.volt.freecam_yaw = self.yaw;
                self.extras.volt.freecam_pitch = self.pitch;
            }
        }
        if self.extras.volt.freecam_active {
            let speed = f64::from(self.settings.extras.volt.freecam.speed.max(1)) * 0.15;
            let sy = f64::from(self.yaw.to_radians()).sin();
            let cy = f64::from(self.yaw.to_radians()).cos();
            let fwd = if self.keys.contains(&KeyCode::KeyW) {
                1.0
            } else if self.keys.contains(&KeyCode::KeyS) {
                -1.0
            } else {
                0.0
            };
            let strafe = if self.keys.contains(&KeyCode::KeyD) {
                1.0
            } else if self.keys.contains(&KeyCode::KeyA) {
                -1.0
            } else {
                0.0
            };
            let vert = if self.keys.contains(&KeyCode::Space) {
                1.0
            } else if self.keys.contains(&KeyCode::ShiftLeft) {
                -1.0
            } else {
                0.0
            };
            self.extras.volt.freecam_pos[0] += (-sy * fwd - cy * strafe) * speed;
            self.extras.volt.freecam_pos[1] += vert * speed;
            self.extras.volt.freecam_pos[2] += (cy * fwd - sy * strafe) * speed;
            self.extras.volt.freecam_yaw = self.yaw;
            self.extras.volt.freecam_pitch = self.pitch;
        }

        // Freelook sync
        let fl_enabled =
            self.settings.extras.volt.freelook.enabled && self.settings.extras.volt.freelook.active;
        if fl_enabled != self.extras.volt.freelook_active {
            self.extras.volt.freelook_active = fl_enabled;
            if fl_enabled {
                self.extras.volt.freelook_yaw = self.yaw;
                self.extras.volt.freelook_pitch = self.pitch;
            }
        }

        // Elytra double-jump swap + swap back on land
        if self.settings.extras.volt.elytra_swap.enabled
            && let Some(p) = self.player.as_ref()
        {
            let on_ground = p.ground.on_ground;
            let falling = p.fall_distance > 1.2 && !on_ground;
            if self.settings.extras.volt.elytra_swap.double_jump_swap
                && falling
                && self.armor[1]
                    .as_ref()
                    .is_none_or(|s| &*s.item != "minecraft:elytra")
                && self.keys.contains(&KeyCode::Space)
                && self.extras.volt.air_jump_ticks == 0
            {
                self.volt_swap_chestplate_elytra();
                self.extras.volt.air_jump_ticks = 12;
            }
            if self.settings.extras.volt.elytra_swap.swap_back_on_land
                && on_ground
                && !self.extras.volt.was_on_ground
                && self.armor[1]
                    .as_ref()
                    .is_some_and(|s| &*s.item == "minecraft:elytra")
            {
                self.volt_swap_chestplate_elytra();
            }
            self.extras.volt.was_on_ground = on_ground;
            self.extras.volt.air_jump_ticks = self.extras.volt.air_jump_ticks.saturating_sub(1);
        }

        // Auto-Armor
        if self.settings.extras.volt.auto_armor.enabled && self.ticks.is_multiple_of(8) {
            self.volt_tick_auto_armor();
        }

        // Auto-Eat
        if self.settings.extras.volt.auto_eat.enabled
            && let Some(p) = self.player.as_ref()
        {
            let threshold = i32::from(self.settings.extras.volt.auto_eat.hunger_threshold);
            let hp_thresh = f32::from(self.settings.extras.volt.auto_eat.hp_threshold);
            if (p.health.food.food <= threshold || p.health.health() <= hp_thresh)
                && self.using.is_none()
            {
                if let Some(hotbar_slot) = (0..9).find(|&i| {
                    self.inventory.get(i).and_then(|s| s.as_ref()).is_some_and(|s| {
                        self.items()
                            .get(&s.item)
                            .is_some_and(|info| info.food.is_some())
                    })
                }) {
                    self.selected = hotbar_slot;
                    self.extras.volt.auto_eating_slot = Some(hotbar_slot);
                    if self.remote.is_some() {
                        self.remote_use_hands();
                    } else {
                        let _ = self.use_item();
                    }
                }
            }
        }

        // Auto-Fish
        if self.settings.extras.volt.auto_fish.enabled
            && self.settings.extras.volt.auto_fish.auto_reel
            && let Some(hook) = self.hook.as_ref()
            && hook.nibble > 0
            && self.ticks.saturating_sub(self.extras.volt.last_auto_fish_tick) >= 12
        {
            self.extras.volt.last_auto_fish_tick = self.ticks;
            if self.remote.is_some() {
                self.remote_use_hands();
            } else {
                let _ = self.use_item();
            }
        }

        // Auto-Respawn
        if self.settings.extras.volt.auto_respawn.enabled
            && self.settings.extras.volt.auto_respawn.instant_respawn
            && self.screen == Screen::Dead
            && self.ticks.is_multiple_of(10)
        {
            let _ = self.respawn_player();
        }

        // Safe-Walk & Anti-AFK
        if self.settings.extras.volt.safe_walk.enabled {
            if self.settings.extras.volt.safe_walk.anti_afk {
                let interval =
                    u64::from(self.settings.extras.volt.safe_walk.afk_interval_secs.max(10)) * 20;
                if self.ticks.saturating_sub(self.extras.volt.last_anti_afk_tick) >= interval {
                    self.extras.volt.last_anti_afk_tick = self.ticks;
                    self.yaw = (self.yaw + 0.5).rem_euclid(360.0);
                }
            }
            if self.settings.extras.volt.safe_walk.edge_stop
                && let Some(p) = self.player.as_mut()
                && p.ground.on_ground
            {
                let below = self.world.block(
                    p.position.x.floor() as i32,
                    (p.position.y - 0.15).floor() as i32,
                    p.position.z.floor() as i32,
                );
                if below.0 == 0 {
                    p.shift_key_down = true;
                }
            }
        }

        // Damage popups tracking from mob HP deltas + Mob Kills
        let track_popups = self.settings.extras.volt.health_indicators.enabled
            && self.settings.extras.volt.health_indicators.show_damage_popups;
        let track_kills = self.settings.extras.volt.session_stats.enabled;
        if track_popups || track_kills {
            for m in &self.mobs {
                if let Some(&prev) = self.extras.volt.mob_last_hp.get(&m.id) {
                    if track_popups && prev - m.health > 0.25 {
                        let dmg = prev - m.health;
                        self.extras.volt.damage_popups.push(DamagePopup {
                            pos: [m.position.x, m.position.y + m.kind.height + 0.35, m.position.z],
                            amount: dmg,
                            crit: dmg >= 6.0,
                            created_tick: self.ticks,
                        });
                    }
                    if track_kills && prev > 0.0 && m.health <= 0.0 {
                        self.extras.volt.session_mob_kills =
                            self.extras.volt.session_mob_kills.saturating_add(1);
                    }
                }
                self.extras.volt.mob_last_hp.insert(m.id, m.health);
            }
            let now_t = self.ticks;
            self.extras
                .volt
                .damage_popups
                .retain(|d| now_t.saturating_sub(d.created_tick) <= 22);
            if self.extras.volt.mob_last_hp.len() > 512 {
                self.extras.volt.mob_last_hp.clear();
            }
        }
    }

    fn volt_tick_auto_armor(&mut self) {
        let cfg = &self.settings.extras.volt.auto_armor;
        let armor_score = |item: &str| -> i32 {
            if item.contains("netherite") {
                50
            } else if item.contains("diamond") {
                40
            } else if item.contains("iron") {
                30
            } else if item.contains("chainmail") {
                25
            } else if item.contains("golden") {
                20
            } else if item.contains("turtle") {
                32
            } else {
                10
            }
        };
        let slot_for = |item: &str| -> Option<usize> {
            if item.ends_with("_boots") {
                Some(3)
            } else if item.ends_with("_leggings") {
                Some(2)
            } else if item.ends_with("_chestplate") {
                Some(1)
            } else if item.ends_with("_helmet") || item == "minecraft:turtle_helmet" {
                Some(0)
            } else {
                None
            }
        };
        for inv_i in 0..self.inventory.len().min(36) {
            let Some(stack) = self.inventory[inv_i].as_ref() else {
                continue;
            };
            let Some(g_idx) = slot_for(&stack.item) else {
                continue;
            };
            if g_idx == 1
                && cfg.respect_elytra
                && self.armor[1]
                    .as_ref()
                    .is_some_and(|s| &*s.item == "minecraft:elytra")
            {
                continue;
            }
            let cand_score = armor_score(&stack.item);
            let cur_score = self.armor[g_idx]
                .as_ref()
                .map_or(-1, |s| armor_score(&s.item));
            if cand_score > cur_score {
                let prev = self.armor[g_idx].take();
                self.armor[g_idx] = self.inventory[inv_i].take();
                self.inventory[inv_i] = prev;
                break;
            }
        }
    }

    // =========================================================================
    // 7. World-Space 3D Gizmos (ESP, Trajectories, TNT/Light/Block/Waypoints)
    // =========================================================================

    pub(super) fn volt_extras_gizmos(&self, _alpha: f32, _camera: [f64; 3], g: &mut Gizmos) {
        let v = &self.settings.extras.volt;
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let px = p.position.x;
        let py = p.position.y;
        let pz = p.position.z;

        // 1) Radar: Chests, Spawners, Dropped Items & Pets
        if v.radar.enabled {
            let r2 = f64::from(v.radar.range).powi(2);
            for (&(bx, by, bz), be) in self.world.block_entities.iter() {
                let fx = f64::from(bx);
                let fy = f64::from(by);
                let fz = f64::from(bz);
                let dx = fx - px;
                let dy = fy - py;
                let dz = fz - pz;
                if dx * dx + dy * dy + dz * dz > r2 {
                    continue;
                }
                let col = match be {
                    BlockEntity::Chest(_) if v.radar.highlight_chests => Some(0xFFFF_C840),
                    BlockEntity::Spawner(_) | BlockEntity::TrialSpawner(_)
                        if v.radar.highlight_spawners =>
                    {
                        Some(0xFFFF_4466)
                    }
                    _ => None,
                };
                if let Some(c) = col {
                    g.cuboid_on_top([fx, fy, fz], [fx + 1.0, fy + 1.0, fz + 1.0], c, 2.0);
                }
            }
            if v.radar.highlight_items {
                for item in &self.dropped {
                    let dx = item.position.x - px;
                    let dy = item.position.y - py;
                    let dz = item.position.z - pz;
                    if dx * dx + dy * dy + dz * dz > r2 {
                        continue;
                    }
                    g.cuboid_on_top(
                        [item.position.x - 0.2, item.position.y, item.position.z - 0.2],
                        [item.position.x + 0.2, item.position.y + 0.4, item.position.z + 0.2],
                        0xFF55_FFFF,
                        1.8,
                    );
                }
            }
        }

        // 2) Ground Drop Highlight Beams
        if v.ground_drops.enabled && v.ground_drops.rare_item_beams {
            let r2 = f64::from(v.ground_drops.range).powi(2);
            for item in &self.dropped {
                let x = item.position.x;
                let y = item.position.y;
                let z = item.position.z;
                if (x - px).powi(2) + (z - pz).powi(2) > r2 {
                    continue;
                }
                let col = if item.stack.item.contains("diamond")
                    || item.stack.item.contains("netherite")
                    || item.stack.item.contains("totem")
                {
                    0xFF55_FFFF
                } else {
                    0xFFFF_E26A
                };
                g.line([x, y, z], [x, y + 1.35, z], col, 2.0);
            }
        }

        // 3) Projectile Trajectories
        if v.trajectories.enabled
            && let Some(held) = self.stack_in(self.selected)
        {
            let is_proj = match &*held.item {
                "minecraft:bow" | "minecraft:crossbow" => v.trajectories.show_bow,
                "minecraft:ender_pearl" => v.trajectories.show_pearl,
                "minecraft:trident" => v.trajectories.show_trident,
                "minecraft:wind_charge" => v.trajectories.show_wind_charge,
                "minecraft:splash_potion" | "minecraft:lingering_potion" => {
                    v.trajectories.show_potion
                }
                "minecraft:snowball" | "minecraft:egg" => v.trajectories.show_pearl,
                _ => false,
            };
            if is_proj {
                let sy = f64::from(self.yaw.to_radians()).sin();
                let cy = f64::from(self.yaw.to_radians()).cos();
                let sp = f64::from(self.pitch.to_radians()).sin();
                let cp = f64::from(self.pitch.to_radians()).cos();
                let speed = if &*held.item == "minecraft:bow" {
                    2.8
                } else if &*held.item == "minecraft:crossbow" {
                    3.15
                } else {
                    1.5
                };
                let [cr, cg, cb] = v.trajectories.color.rgb();
                let col = 0xFF00_0000
                    | (u32::from(cr) << 16)
                    | (u32::from(cg) << 8)
                    | u32::from(cb);
                let lw = f32::from(v.trajectories.line_width.max(1));
                let mut pos = [px, py + 1.52, pz];
                let mut vel = [-sy * cp * speed, -sp * speed, cy * cp * speed];
                for _ in 0..48 {
                    let next = [
                        pos[0] + vel[0] * 0.5,
                        pos[1] + vel[1] * 0.5,
                        pos[2] + vel[2] * 0.5,
                    ];
                    g.line(pos, next, col, lw);
                    let hit = self.world.block(
                        next[0].floor() as i32,
                        next[1].floor() as i32,
                        next[2].floor() as i32,
                    );
                    if hit.0 != 0 {
                        if v.trajectories.show_impact_box {
                            let bx = next[0].floor();
                            let by = next[1].floor();
                            let bz = next[2].floor();
                            g.cuboid_on_top(
                                [bx, by, bz],
                                [bx + 1.0, by + 1.0, bz + 1.0],
                                0xFFFF_5252,
                                lw,
                            );
                        }
                        break;
                    }
                    pos = next;
                    vel[0] *= 0.99;
                    vel[1] = vel[1] * 0.99 - 0.025;
                    vel[2] *= 0.99;
                }
            }
        }

        // 4) Tweakeroo Light Level Overlay
        if v.tweakeroo.enabled && v.tweakeroo.light_overlay {
            let r = i32::from(v.tweakeroo.light_range.clamp(4, 16));
            let bx0 = px.floor() as i32;
            let by0 = py.floor() as i32;
            let bz0 = pz.floor() as i32;
            for dx in -r..=r {
                for dz in -r..=r {
                    for dy in -2..=2 {
                        let bx = bx0 + dx;
                        let by = by0 + dy;
                        let bz = bz0 + dz;
                        let below = self.world.block(bx, by - 1, bz);
                        let here = self.world.block(bx, by, bz);
                        if below.0 == 0 || here.0 != 0 {
                            continue;
                        }
                        let l = self.world.light(bx, by, bz);
                        let block_l = l & 15;
                        let sky_l = l >> 4;
                        let col = if block_l == 0 && sky_l == 0 {
                            Some(0xFFFF_4444)
                        } else if block_l == 0 {
                            Some(0xFFFF_D644)
                        } else {
                            None
                        };
                        if let Some(c) = col {
                            let fx = f64::from(bx) + 0.15;
                            let fz = f64::from(bz) + 0.15;
                            let fy = f64::from(by) + 0.03;
                            g.line([fx, fy, fz], [fx + 0.7, fy, fz + 0.7], c, 2.0);
                            g.line([fx + 0.7, fy, fz], [fx, fy, fz + 0.7], c, 2.0);
                        }
                        break;
                    }
                }
            }
        }

        // 5) TNT / Crystal Danger Ring
        if v.tnt_timer.enabled && v.tnt_timer.danger_halo {
            for t in &self.tnts {
                Self::draw_ring_gizmo(g, [t.position.x, t.position.y + 0.05, t.position.z], 5.0, 0xFFFF_4848);
            }
            if v.tnt_timer.crystal_ring {
                for c in &self.crystals {
                    Self::draw_ring_gizmo(g, [c.position.x, c.position.y + 0.05, c.position.z], 6.0, 0xFFFF_66FF);
                }
            }
        }

        // 6) Waypoint 3D Beacons
        if v.waypoints.enabled && v.waypoints.show_beacons {
            let bh = f64::from(v.waypoints.beam_height.max(16));
            for wp in &self.extras.volt.waypoints {
                if wp.dim != self.dim {
                    continue;
                }
                g.line(
                    [wp.pos[0], wp.pos[1] - 1.0, wp.pos[2]],
                    [wp.pos[0], wp.pos[1] + bh, wp.pos[2]],
                    wp.color,
                    2.5,
                );
            }
        }

        // 7) Reach Box (#59)
        if v.reach_box.enabled
            && let Some((mob_idx, dist)) = self.mob_hit()
            && let Some(m) = self.mobs.get(mob_idx)
        {
            let max_reach = if self.creative { 5.0 } else { 3.0 };
            let in_reach = dist <= max_reach;
            let col = if in_reach { 0xFF52_F28A } else { 0xFFFF_5252 };
            let hw = f64::from(m.kind.width) * 0.5;
            let hh = f64::from(m.kind.height);
            g.cuboid_on_top(
                [m.position.x - hw, m.position.y, m.position.z - hw],
                [m.position.x + hw, m.position.y + hh, m.position.z + hw],
                col,
                2.2,
            );
        }

        // 8) Player Highlight (#73)
        if v.player_highlight.enabled {
            let [cr, cg, cb] = v.player_highlight.color.rgb();
            let col = 0xFF00_0000 | (u32::from(cr) << 16) | (u32::from(cg) << 8) | u32::from(cb);
            let max_dist_sq = f64::from(v.player_highlight.range.max(4)).powi(2);
            if v.player_highlight.highlight_players {
                for (id, _) in self.remote_player_names() {
                    if let Some(m) = self.mobs.iter().find(|m| m.id == id) {
                        if m.invisible {
                            continue;
                        }
                        let d2 = (m.position.x - px).powi(2) + (m.position.z - pz).powi(2);
                        if d2 > max_dist_sq {
                            continue;
                        }
                        let hw = f64::from(m.kind.width) * 0.52;
                        let hh = f64::from(m.kind.height);
                        g.cuboid_on_top(
                            [m.position.x - hw, m.position.y, m.position.z - hw],
                            [m.position.x + hw, m.position.y + hh, m.position.z + hw],
                            col,
                            2.5,
                        );
                    }
                }
                if let Some(host) = &self.lan {
                    for (_, m) in &host.models {
                        if m.invisible {
                            continue;
                        }
                        let hw = f64::from(m.kind.width) * 0.52;
                        let hh = f64::from(m.kind.height);
                        g.cuboid_on_top(
                            [m.position.x - hw, m.position.y, m.position.z - hw],
                            [m.position.x + hw, m.position.y + hh, m.position.z + hw],
                            col,
                            2.5,
                        );
                    }
                }
            }
            if v.player_highlight.highlight_hostiles {
                for m in &self.mobs {
                    if m.kind.category != "monster" || m.invisible {
                        continue;
                    }
                    let d2 = (m.position.x - px).powi(2) + (m.position.z - pz).powi(2);
                    if d2 > max_dist_sq {
                        continue;
                    }
                    let hw = f64::from(m.kind.width) * 0.52;
                    let hh = f64::from(m.kind.height);
                    g.cuboid_on_top(
                        [m.position.x - hw, m.position.y, m.position.z - hw],
                        [m.position.x + hw, m.position.y + hh, m.position.z + hw],
                        0xFFFF_5555,
                        2.0,
                    );
                }
            }
        }

        // 9) Litematica Ruler / Placement Grid (#74)
        if v.litematica_grid.enabled {
            if let (Some(a), Some(b)) = (self.extras.volt.ruler_a, self.extras.volt.ruler_b) {
                let min_x = f64::from(a[0].min(b[0]));
                let min_y = f64::from(a[1].min(b[1]));
                let min_z = f64::from(a[2].min(b[2]));
                let max_x = f64::from(a[0].max(b[0]) + 1);
                let max_y = f64::from(a[1].max(b[1]) + 1);
                let max_z = f64::from(a[2].max(b[2]) + 1);
                if v.litematica_grid.show_3d_box {
                    g.cuboid_on_top([min_x, min_y, min_z], [max_x, max_y, max_z], 0xFF55_FFFF, 2.2);
                }
                if v.litematica_grid.show_grid_lines {
                    let mut gx = min_x;
                    while gx <= max_x + 0.01 {
                        g.line([gx, min_y, min_z], [gx, min_y, max_z], 0x8855_FFFF, 1.0);
                        gx += 1.0;
                    }
                    let mut gz = min_z;
                    while gz <= max_z + 0.01 {
                        g.line([min_x, min_y, gz], [max_x, min_y, gz], 0x8855_FFFF, 1.0);
                        gz += 1.0;
                    }
                }
            } else if v.litematica_grid.show_grid_lines {
                let bx = px.floor();
                let by = py.floor() + 0.02;
                let bz = pz.floor();
                for d in -4..=4 {
                    let df = f64::from(d);
                    g.line([bx - 4.0, by, bz + df], [bx + 4.0, by, bz + df], 0x5555_FFFF, 1.0);
                    g.line([bx + df, by, bz - 4.0], [bx + df, by, bz + 4.0], 0x5555_FFFF, 1.0);
                }
            }
        }

        // 10) Chunk Pillars (#76)
        if v.chunk_pillars.enabled && v.chunk_pillars.draw_corner_pillars {
            let cx0 = px.floor() as i32 >> 4;
            let cz0 = pz.floor() as i32 >> 4;
            let rad = 2_i32;
            let ph = f64::from(v.chunk_pillars.pillar_height.max(16));
            let y0 = (py - 16.0).max(-64.0);
            let y1 = py + ph;
            for dcx in -rad..=rad {
                for dcz in -rad..=rad {
                    let x = f64::from((cx0 + dcx) * 16);
                    let z = f64::from((cz0 + dcz) * 16);
                    let col = if dcx == 0 && dcz == 0 { 0xFF55_FF88 } else { 0xAAFF_DD55 };
                    g.line([x, y0, z], [x, y1, z], col, 1.8);
                }
            }
        }
    }

    fn draw_ring_gizmo(g: &mut Gizmos, center: [f64; 3], radius: f64, color: u32) {
        let segs = 20;
        for i in 0..segs {
            let a0 = (i as f64 / segs as f64) * std::f64::consts::TAU;
            let a1 = ((i + 1) as f64 / segs as f64) * std::f64::consts::TAU;
            g.line(
                [center[0] + radius * a0.cos(), center[1], center[2] + radius * a0.sin()],
                [center[0] + radius * a1.cos(), center[1], center[2] + radius * a1.sin()],
                color,
                2.0,
            );
        }
    }

    // =========================================================================
    // 8. Screen Projection & Full Volt+ Extras HUD Drawing
    // =========================================================================

    fn world_to_screen(&self, pos: [f64; 3], wi: i32, hi: i32) -> Option<[f32; 2]> {
        let (cam, yaw, pitch, fov) = self.locator_camera?;
        let sy = f64::from(yaw.to_radians()).sin();
        let cy = f64::from(yaw.to_radians()).cos();
        let sp = f64::from(pitch.to_radians()).sin();
        let cp = f64::from(pitch.to_radians()).cos();
        let forward = [-sy * cp, -sp, cy * cp];
        let up = [-sy * sp, cp, cy * sp];
        let right = [-cy, 0.0, -sy];
        let o = [pos[0] - cam[0], pos[1] - cam[1], pos[2] - cam[2]];
        let depth = o[0] * forward[0] + o[1] * forward[1] + o[2] * forward[2];
        if depth <= 0.15 {
            return None;
        }
        let focal_y = 1.0 / f64::from((fov / 2.0).to_radians()).tan();
        let aspect = wi as f64 / hi.max(1) as f64;
        let focal_x = focal_y / aspect;
        let rx = (o[0] * right[0] + o[1] * right[1] + o[2] * right[2]) * focal_x / depth;
        let ry = (o[0] * up[0] + o[1] * up[1] + o[2] * up[2]) * focal_y / depth;
        let sx = (rx * 0.5 + 0.5) * wi as f64;
        let sy_px = (0.5 - ry * 0.5) * hi as f64;
        if sx < -40.0 || sx > wi as f64 + 40.0 || sy_px < -40.0 || sy_px > hi as f64 + 40.0 {
            return None;
        }
        Some([sx as f32, sy_px as f32])
    }

    fn corner_xy(corner: Corner, wi: i32, hi: i32, w: f32, h: f32, y_off: f32) -> (f32, f32) {
        match corner {
            Corner::TopLeft => (6.0, 6.0 + y_off),
            Corner::TopRight => (wi as f32 - w - 6.0, 6.0 + y_off),
            Corner::BottomLeft => (6.0, hi as f32 - h - 6.0 - y_off),
            Corner::BottomRight => (wi as f32 - w - 6.0, hi as f32 - h - 6.0 - y_off),
        }
    }

    pub(super) fn draw_volt_extras_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        self.draw_health_bars_and_popups_hud(batch, wi, hi);
        self.draw_tnt_and_drops_hud(batch, wi, hi);
        self.draw_waypoints_hud(batch, wi, hi);

        self.draw_shield_status_hud(batch, wi, hi);
        self.draw_uku_armor_hud(batch, wi, hi);
        self.draw_uku_potion_hud(batch, wi, hi);
        self.draw_standalone_potion_hud(batch, wi, hi);
        self.draw_totem_and_low_hp_hud(batch, wi, hi);
        self.draw_elytra_and_mace_hud(batch, wi, hi);
        self.draw_combat_hud(batch, wi, hi);
        self.draw_keystrokes_hud(batch, wi, hi);
        self.draw_crosshair_extras_hud(batch, wi, hi);

        self.draw_info_hud(batch, wi, hi);
        self.draw_new_hud_panels(batch, wi, hi);
        self.draw_compass_hud(batch, wi, hi);
        self.draw_look_and_portal_hud(batch, wi, hi);
        self.draw_quick_strip_and_toast(batch, wi, hi);
    }

    fn draw_health_bars_and_popups_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.health_indicators;
        if !cfg.enabled {
            return;
        }
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let atlas = &self.gui.atlas;
        let max_d2 = f64::from(cfg.max_range).powi(2);

        if cfg.show_bars {
            for m in &self.mobs {
                let is_hostile = m.kind.category == "monster";
                if is_hostile && !cfg.show_hostiles {
                    continue;
                }
                let dx = m.position.x - p.position.x;
                let dy = m.position.y - p.position.y;
                let dz = m.position.z - p.position.z;
                if dx * dx + dy * dy + dz * dz > max_d2 {
                    continue;
                }
                let Some([sx, sy]) = self.world_to_screen(
                    [m.position.x, m.position.y + m.kind.height + 0.4, m.position.z],
                    wi,
                    hi,
                ) else {
                    continue;
                };
                let max_hp = m.kind.max_health.max(1.0);
                let ratio = (m.health / max_hp).clamp(0.0, 1.0);
                let bw = 40.0 * (f32::from(cfg.bar_scale) / 100.0).clamp(0.6, 1.6);
                let bx = sx - bw * 0.5;
                let by = sy - 8.0;
                batch.fill(atlas, bx - 1.0, by - 1.0, bw + 2.0, 6.0, rgba(12, 12, 16, 185));
                let bar_col = if ratio > 0.5 {
                    rgba(86, 228, 124, 240)
                } else if ratio > 0.25 {
                    rgba(255, 206, 66, 240)
                } else {
                    rgba(255, 76, 76, 240)
                };
                batch.fill(atlas, bx, by, bw * ratio, 4.0, bar_col);
                let txt = format!("{:.0}/{:.0}", m.health.max(0.0), max_hp);
                let tw = HudBatch::text_width(atlas, &txt);
                batch.text(atlas, &txt, sx - tw * 0.5, by - 10.0, u32::MAX, true);
            }
        }

        if cfg.show_damage_popups {
            for pop in &self.extras.volt.damage_popups {
                let age = self.ticks.saturating_sub(pop.created_tick) as f64;
                let Some([sx, sy]) = self.world_to_screen(
                    [pop.pos[0], pop.pos[1] + age * 0.03, pop.pos[2]],
                    wi,
                    hi,
                ) else {
                    continue;
                };
                let col = if cfg.highlight_crits && pop.crit {
                    rgba(255, 214, 60, 255)
                } else {
                    rgba(255, 92, 92, 255)
                };
                batch.text(atlas, &format!("-{:.1}", pop.amount), sx, sy, col, true);
            }
        }
    }

    fn draw_tnt_and_drops_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;
        let tnt_cfg = &self.settings.extras.volt.tnt_timer;
        if tnt_cfg.enabled && tnt_cfg.show_tnt_fuse {
            for t in &self.tnts {
                if let Some([sx, sy]) =
                    self.world_to_screen([t.position.x, t.position.y + 1.15, t.position.z], wi, hi)
                {
                    let secs = t.fuse.max(0) as f32 / 20.0;
                    let txt = format!("{secs:.1}s");
                    let tw = HudBatch::text_width(atlas, &txt);
                    batch.fill(
                        atlas,
                        sx - tw * 0.5 - 3.0,
                        sy - 2.0,
                        tw + 6.0,
                        11.0,
                        rgba(20, 10, 10, 200),
                    );
                    batch.text(
                        atlas,
                        &txt,
                        sx - tw * 0.5,
                        sy,
                        if secs < 1.2 {
                            rgba(255, 82, 82, 255)
                        } else {
                            rgba(255, 214, 86, 255)
                        },
                        true,
                    );
                }
            }
        }

        let gd_cfg = &self.settings.extras.volt.ground_drops;
        if gd_cfg.enabled && gd_cfg.merge_count_labels {
            let Some(p) = self.player.as_ref() else {
                return;
            };
            let r2 = f64::from(gd_cfg.range).powi(2);
            for item in &self.dropped {
                let d2 = (item.position.x - p.position.x).powi(2)
                    + (item.position.z - p.position.z).powi(2);
                if d2 > r2 {
                    continue;
                }
                if let Some([sx, sy]) = self.world_to_screen(
                    [item.position.x, item.position.y + 0.65, item.position.z],
                    wi,
                    hi,
                ) {
                    let short = item
                        .stack
                        .item
                        .strip_prefix("minecraft:")
                        .unwrap_or(&item.stack.item);
                    let lbl = if item.stack.count > 1 {
                        format!("{short} x{}", item.stack.count)
                    } else {
                        short.to_owned()
                    };
                    let tw = HudBatch::text_width(atlas, &lbl);
                    batch.fill(
                        atlas,
                        sx - tw * 0.5 - 2.0,
                        sy - 1.0,
                        tw + 4.0,
                        10.0,
                        rgba(14, 14, 20, 165),
                    );
                    batch.text(atlas, &lbl, sx - tw * 0.5, sy, rgba(235, 240, 250, 255), true);
                }
            }
        }
    }

    fn draw_waypoints_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.waypoints;
        if !cfg.enabled || !cfg.show_hud_distances {
            return;
        }
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let atlas = &self.gui.atlas;
        for wp in &self.extras.volt.waypoints {
            if wp.dim != self.dim {
                continue;
            }
            let dx = wp.pos[0] - p.position.x;
            let dy = wp.pos[1] - p.position.y;
            let dz = wp.pos[2] - p.position.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist > f64::from(cfg.max_distance) {
                continue;
            }
            let Some([sx, sy]) = self.world_to_screen(wp.pos, wi, hi) else {
                continue;
            };
            let lbl = format!("{} [{:.0}m]", wp.name, dist);
            let tw = HudBatch::text_width(atlas, &lbl);
            batch.fill(
                atlas,
                sx - tw * 0.5 - 3.0,
                sy - 2.0,
                tw + 6.0,
                11.0,
                rgba(16, 18, 26, 190),
            );
            batch.text(atlas, &lbl, sx - tw * 0.5, sy, wp.color, true);
        }
    }

    fn draw_totem_and_low_hp_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;
        let tcfg = &self.settings.extras.volt.totem_counter;
        if tcfg.enabled && tcfg.show_hud_badge {
            let mut count = 0u32;
            for slot in &self.inventory {
                if let Some(s) = slot.as_ref()
                    && &*s.item == "minecraft:totem_of_undying"
                {
                    count += s.count;
                }
            }
            if let Some(s) = self.offhand.as_ref()
                && &*s.item == "minecraft:totem_of_undying"
            {
                count += s.count;
            }
            let txt = format!("Totem:{count}");
            let tw = HudBatch::text_width(atlas, &txt);
            let (bx, by) = Self::corner_xy(tcfg.corner, wi, hi, tw + 10.0, 15.0, 0.0);
            batch.fill(atlas, bx, by, tw + 10.0, 15.0, rgba(18, 18, 24, 180));
            let col = if count == 0 && tcfg.warn_when_empty {
                rgba(255, 86, 86, 255)
            } else {
                rgba(255, 220, 96, 255)
            };
            batch.text(atlas, &txt, bx + 5.0, by + 3.0, col, true);
        }

        if let Some(p) = self.player.as_ref()
            && tcfg.enabled
            && tcfg.low_hp_alert
        {
            let thresh = f32::from(tcfg.low_hp_hearts) * 2.0;
            let hp = p.health.health();
            if hp > 0.0 && hp <= thresh {
                let pulse = if (self.ticks / 6).is_multiple_of(2) {
                    110
                } else {
                    45
                };
                let c = rgba(235, 32, 42, pulse);
                batch.fill(atlas, 0.0, 0.0, wi as f32, 6.0, c);
                batch.fill(atlas, 0.0, hi as f32 - 6.0, wi as f32, 6.0, c);
                batch.fill(atlas, 0.0, 0.0, 6.0, hi as f32, c);
                batch.fill(atlas, wi as f32 - 6.0, 0.0, 6.0, hi as f32, c);
                let msg = format!("LOW HP ({hp:.1})");
                let tw = HudBatch::text_width(atlas, &msg);
                batch.text(
                    atlas,
                    &msg,
                    (wi as f32 - tw) * 0.5,
                    (hi as f32) * 0.25,
                    rgba(255, 92, 92, 255),
                    true,
                );
            }
        }

        let fcfg = &self.settings.extras.volt.totem_pop_counter;
        if fcfg.enabled && !self.extras.volt.totem_pop_feed.is_empty() {
            for (i, (who, tick)) in self.extras.volt.totem_pop_feed.iter().enumerate() {
                if self.ticks.saturating_sub(*tick) > 140 {
                    continue;
                }
                let line = format!("POP: {who}");
                let tw = HudBatch::text_width(atlas, &line);
                let (x, y) = Self::corner_xy(fcfg.corner, wi, hi, tw + 8.0, 12.0, 20.0 + i as f32 * 13.0);
                batch.fill(atlas, x, y, tw + 8.0, 12.0, rgba(24, 16, 12, 180));
                batch.text(atlas, &line, x + 4.0, y + 2.0, rgba(255, 206, 78, 255), true);
            }
        }
    }

    fn draw_elytra_and_mace_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;
        let ecfg = &self.settings.extras.volt.elytra_hud;
        if ecfg.enabled
            && let Some(p) = self.player.as_ref()
            && p.fall_flying
        {
            let bx = (wi as f32) * 0.5 + 48.0;
            let by = (hi as f32) * 0.5 - 24.0;
            let spd = (p.velocity.x * p.velocity.x + p.velocity.z * p.velocity.z).sqrt() * 20.0;
            let mut parts: Vec<String> = Vec::new();
            parts.push(format!("ELY {:.1}b/s", spd));
            if ecfg.show_altitude {
                parts.push(format!("Y:{:.0}", p.position.y));
            }
            if ecfg.show_pitch_gauge {
                parts.push(format!("{:.0}°", self.pitch));
            }
            let txt = parts.join(" ");
            let tw = HudBatch::text_width(atlas, &txt);
            batch.fill(atlas, bx - 4.0, by - 2.0, tw + 8.0, 13.0, rgba(16, 20, 28, 180));
            batch.text(atlas, &txt, bx, by, rgba(130, 224, 255, 255), true);
        }

        if ecfg.enabled
            && ecfg.show_mace_damage
            && let Some(p) = self.player.as_ref()
            && self
                .stack_in(self.selected)
                .is_some_and(|s| &*s.item == "minecraft:mace")
        {
            let fall = p.fall_distance.max(0.0);
            if fall >= 1.5 {
                let first = fall.min(3.0) * 4.0;
                let next = (fall - 3.0).clamp(0.0, 5.0) * 2.0;
                let rest = (fall - 8.0).max(0.0);
                let smash_dmg = 6.0 + first + next + rest;
                let lbl = format!("MACE {:.1}m (+{:.0} DMG)", fall, smash_dmg);
                let tw = HudBatch::text_width(atlas, &lbl);
                let x = (wi as f32 - tw) * 0.5;
                let y = (hi as f32) * 0.5 + 22.0;
                batch.fill(atlas, x - 4.0, y - 2.0, tw + 8.0, 12.0, rgba(24, 18, 28, 195));
                batch.text(atlas, &lbl, x, y, rgba(255, 204, 72, 255), true);
            }
        }
    }

    fn draw_combat_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.cps_combo;
        if !cfg.enabled {
            return;
        }
        let atlas = &self.gui.atlas;
        let mut parts: Vec<String> = Vec::new();
        let now = self.started.elapsed().as_secs_f64();
        if cfg.show_lmb_cps || cfg.show_rmb_cps {
            let lc = self.extras.cps[0].count(now);
            let rc = self.extras.cps[1].count(now);
            parts.push(format!("{lc} | {rc} CPS"));
        }
        if cfg.show_reach
            && let Some(r) = self.extras.volt.last_reach_blocks
        {
            parts.push(format!("{r:.2}m"));
        }
        if cfg.show_combo && self.extras.volt.combo_count > 0 {
            parts.push(format!("{}x", self.extras.volt.combo_count));
        }
        if parts.is_empty() {
            return;
        }
        let line = parts.join(" · ");
        let tw = HudBatch::text_width(atlas, &line);
        let (x, y) = Self::corner_xy(cfg.corner, wi, hi, tw + 10.0, 14.0, 32.0);
        batch.fill(atlas, x, y, tw + 10.0, 14.0, rgba(16, 18, 24, 180));
        batch.text(atlas, &line, x + 5.0, y + 3.0, rgba(240, 244, 252, 255), true);
    }

    fn draw_keystrokes_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.keystrokes;
        if !cfg.enabled {
            return;
        }
        let atlas = &self.gui.atlas;
        let bg_a = ((f32::from(cfg.bg_opacity_pct.clamp(10, 100)) / 100.0) * 230.0) as u8;
        let (bx, by) = Self::corner_xy(cfg.corner, wi, hi, 64.0, 62.0, 50.0);
        let key_box = |batch: &mut HudBatch, x: f32, y: f32, w: f32, h: f32, lbl: &str, down: bool| {
            let bg = if down {
                rgba(240, 242, 248, 215)
            } else {
                rgba(16, 18, 24, bg_a)
            };
            let fg = if down { rgba(16, 18, 24, 255) } else { u32::MAX };
            batch.fill(atlas, x, y, w, h, bg);
            let tw = HudBatch::text_width(atlas, lbl);
            batch.text(atlas, lbl, x + (w - tw) * 0.5, y + (h - 8.0) * 0.5, fg, !down);
        };
        let mv = self.pad_move();
        let w = self.keys.contains(&KeyCode::KeyW) || mv.is_some_and(|[_, y]| y > 0.2);
        let a = self.keys.contains(&KeyCode::KeyA) || mv.is_some_and(|[x, _]| x < -0.2);
        let s = self.keys.contains(&KeyCode::KeyS) || mv.is_some_and(|[_, y]| y < -0.2);
        let d = self.keys.contains(&KeyCode::KeyD) || mv.is_some_and(|[x, _]| x > 0.2);
        key_box(batch, bx + 22.0, by, 20.0, 18.0, "W", w);
        key_box(batch, bx, by + 20.0, 20.0, 18.0, "A", a);
        key_box(batch, bx + 22.0, by + 20.0, 20.0, 18.0, "S", s);
        key_box(batch, bx + 44.0, by + 20.0, 20.0, 18.0, "D", d);
        if cfg.show_cps {
            key_box(batch, bx, by + 40.0, 31.0, 16.0, "LMB", self.extras.mouse[0]);
            key_box(batch, bx + 33.0, by + 40.0, 31.0, 16.0, "RMB", self.extras.mouse[1]);
        }
    }

    fn draw_crosshair_extras_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;
        let cx = wi as f32 * 0.5;
        let cy = hi as f32 * 0.5;
        let cfg = &self.settings.extras.volt.crosshair_tweaks;
        if cfg.enabled {
            let [cr, cg, cb] = cfg.color.rgb();
            let col = rgba(cr, cg, cb, 235);
            let sz = f32::from(cfg.size.clamp(3, 12));
            let th = f32::from(cfg.thickness.clamp(1, 3));
            match cfg.style {
                CrosshairStyle::Vanilla => {}
                CrosshairStyle::Dot => {
                    batch.fill(atlas, cx - th, cy - th, th * 2.0, th * 2.0, col);
                }
                CrosshairStyle::Cross => {
                    batch.fill(atlas, cx - sz, cy - th * 0.5, sz * 2.0, th, col);
                    batch.fill(atlas, cx - th * 0.5, cy - sz, th, sz * 2.0, col);
                }
                CrosshairStyle::Circle => {
                    batch.fill(atlas, cx - sz, cy - sz, sz * 2.0, th, col);
                    batch.fill(atlas, cx - sz, cy + sz - th, sz * 2.0, th, col);
                    batch.fill(atlas, cx - sz, cy - sz, th, sz * 2.0, col);
                    batch.fill(atlas, cx + sz - th, cy - sz, th, sz * 2.0, col);
                }
            }
            if cfg.hit_marker && self.ticks <= self.extras.volt.hit_marker_until_tick {
                let hc = rgba(255, 232, 92, 245);
                for d in 3..=6 {
                    let f = d as f32;
                    batch.fill(atlas, cx - f, cy - f, 1.5, 1.5, hc);
                    batch.fill(atlas, cx + f - 1.5, cy - f, 1.5, 1.5, hc);
                    batch.fill(atlas, cx - f, cy + f - 1.5, 1.5, 1.5, hc);
                    batch.fill(atlas, cx + f - 1.5, cy + f - 1.5, 1.5, 1.5, hc);
                }
            }
        }

        // Crit Ring (#56)
        let cr_cfg = &self.settings.extras.volt.crit_ring;
        if cr_cfg.enabled && self.ticks <= self.extras.volt.crit_ring_until_tick {
            let rem = self.extras.volt.crit_ring_until_tick.saturating_sub(self.ticks) as f32;
            let rad = 7.0 + (12.0 - rem) * 0.65;
            let [r, g, b] = cr_cfg.crit_color.rgb();
            let alpha = ((rem / 12.0) * 240.0).round() as u8;
            let col = rgba(r, g, b, alpha);
            batch.fill(atlas, cx - rad, cy - rad, rad * 2.0, 1.5, col);
            batch.fill(atlas, cx - rad, cy + rad - 1.5, rad * 2.0, 1.5, col);
            batch.fill(atlas, cx - rad, cy - rad, 1.5, rad * 2.0, col);
            batch.fill(atlas, cx + rad - 1.5, cy - rad, 1.5, rad * 2.0, col);
            if cr_cfg.show_crit_badge {
                let msg = "CRIT!";
                let tw = HudBatch::text_width(atlas, msg);
                batch.text(atlas, msg, cx - tw * 0.5, cy - rad - 10.0, col, true);
            }
        }

        // Bow Charge Arc / Bar (#57)
        let bc_cfg = &self.settings.extras.volt.bow_charge;
        if bc_cfg.enabled
            && let Some(held) = self.stack_in(self.selected)
        {
            let is_bow = &*held.item == "minecraft:bow";
            let is_xbow = &*held.item == "minecraft:crossbow";
            let is_trident = &*held.item == "minecraft:trident";
            if is_bow || is_xbow || is_trident {
                let frac = if is_xbow && held.data().is_some_and(|d| !d.charged.is_empty()) {
                    1.0
                } else if let Some((slot, ticks_used)) = self.using
                    && slot == self.selected
                {
                    let max_t = if is_xbow {
                        25.0
                    } else if is_trident {
                        10.0
                    } else {
                        20.0
                    };
                    (ticks_used as f32 / max_t).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if frac > 0.01 {
                    let bw = 36.0;
                    let bx = cx - bw * 0.5;
                    let by = cy + 14.0;
                    let col = if frac >= 0.99 {
                        rgba(96, 245, 130, 245)
                    } else if frac >= 0.5 {
                        rgba(255, 218, 72, 245)
                    } else {
                        rgba(220, 226, 238, 225)
                    };
                    batch.fill(atlas, bx, by, bw, 3.0, rgba(16, 18, 24, 195));
                    batch.fill(atlas, bx, by, bw * frac, 3.0, col);
                    if bc_cfg.show_pct_bar {
                        let txt = if frac >= 0.99 {
                            "MAX".to_owned()
                        } else {
                            format!("{:.0}%", frac * 100.0)
                        };
                        let tw = HudBatch::text_width(atlas, &txt);
                        batch.text(atlas, &txt, cx - tw * 0.5, by + 5.0, col, true);
                    }
                }
            }
        }

        // Reach distance readout (#59)
        if self.settings.extras.volt.reach_box.enabled
            && self.settings.extras.volt.reach_box.draw_reach_ring
            && let Some((_, dist)) = self.mob_hit()
        {
            let max_reach = if self.creative { 5.0 } else { 3.0 };
            let txt = format!("{dist:.2}m");
            let tw = HudBatch::text_width(atlas, &txt);
            let col = if dist <= max_reach {
                rgba(96, 245, 138, 255)
            } else {
                rgba(255, 92, 92, 255)
            };
            batch.fill(atlas, cx - tw * 0.5 - 3.0, cy + 25.0, tw + 6.0, 11.0, rgba(16, 18, 24, 175));
            batch.text(atlas, &txt, cx - tw * 0.5, cy + 26.5, col, true);
        }

        let scfg = &self.settings.extras.volt.auto_sprint;
        if scfg.enabled && scfg.show_indicator {
            let sprinting = self.player.as_ref().is_some_and(|p| p.sprinting);
            let txt = if sprinting {
                "[SPRINT ON]"
            } else {
                "[SPRINT RDY]"
            };
            let tw = HudBatch::text_width(atlas, txt);
            let (x, y) = Self::corner_xy(scfg.corner, wi, hi, tw + 8.0, 12.0, 16.0);
            batch.fill(atlas, x, y, tw + 8.0, 12.0, rgba(16, 18, 24, 175));
            batch.text(
                atlas,
                txt,
                x + 4.0,
                y + 2.0,
                if sprinting {
                    rgba(120, 245, 155, 255)
                } else {
                    rgba(220, 224, 232, 225)
                },
                true,
            );
        }

        if self.settings.extras.volt.auto_fish.enabled
            && self.settings.extras.volt.auto_fish.visual_bite_alert
            && let Some(h) = self.hook.as_ref()
            && h.nibble > 0
        {
            let msg = "FISH BITE!";
            let tw = HudBatch::text_width(atlas, msg);
            batch.text(
                atlas,
                msg,
                cx - tw * 0.5,
                cy - 24.0,
                rgba(110, 240, 255, 255),
                true,
            );
        }
    }

    fn draw_info_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;
        let Some(p) = self.player.as_ref() else {
            return;
        };

        // 1) Coords + Biome + Facing
        let ccfg = &self.settings.extras.volt.coords_hud;
        if ccfg.enabled {
            let mut lines: Vec<String> = Vec::new();
            if ccfg.show_xyz {
                lines.push(format!(
                    "XYZ: {:.0}, {:.0}, {:.0}",
                    p.position.x, p.position.y, p.position.z
                ));
            }
            if ccfg.show_facing {
                let y = self.yaw.rem_euclid(360.0);
                let dir = if !(45.0..315.0).contains(&y) {
                    "S (+Z)"
                } else if y < 135.0 {
                    "W (-X)"
                } else if y < 225.0 {
                    "N (-Z)"
                } else {
                    "E (+X)"
                };
                lines.push(format!("Facing: {dir}"));
            }
            if ccfg.show_biome {
                let bx = p.position.x.floor() as i32;
                let by = p.position.y.floor() as i32;
                let bz = p.position.z.floor() as i32;
                if let Some(chunk) = self.world.chunks.get(&(bx >> 4, bz >> 4)) {
                    let biome = self.res.biomes().get(chunk.biome(bx, by, bz));
                    lines.push(format!(
                        "Biome: {}",
                        biome.name.strip_prefix("minecraft:").unwrap_or(&biome.name)
                    ));
                }
            }
            if ccfg.show_nether_equiv && matches!(self.dim, Dim::Overworld | Dim::Nether) {
                if self.dim == Dim::Overworld {
                    lines.push(format!(
                        "Nether: {:.0}, {:.0}",
                        p.position.x / 8.0,
                        p.position.z / 8.0
                    ));
                } else {
                    lines.push(format!(
                        "Overworld: {:.0}, {:.0}",
                        p.position.x * 8.0,
                        p.position.z * 8.0
                    ));
                }
            }
            if !lines.is_empty() {
                let max_w = lines
                    .iter()
                    .map(|l| HudBatch::text_width(atlas, l))
                    .fold(0.0f32, f32::max)
                    + 10.0;
                let h = lines.len() as f32 * 11.0 + 4.0;
                let (x, y) = Self::corner_xy(ccfg.corner, wi, hi, max_w, h, 28.0);
                batch.fill(atlas, x, y, max_w, h, rgba(16, 18, 24, 175));
                for (i, l) in lines.iter().enumerate() {
                    batch.text(atlas, l, x + 5.0, y + 3.0 + i as f32 * 11.0, u32::MAX, true);
                }
            }
        }

        // 2) FPS / Ping / Speedometer
        let fcfg = &self.settings.extras.volt.telemetry_hud;
        if fcfg.enabled {
            let mut parts: Vec<String> = Vec::new();
            let reading = self.extras.fps.reading();
            if fcfg.show_fps {
                parts.push(format!("{} FPS", reading.fps));
            }
            if fcfg.show_lows {
                parts.push(format!("1%:{}", reading.low));
            }
            if fcfg.show_ping {
                let ping = self.debug_charts.ping.latest().unwrap_or(0);
                parts.push(format!("{ping}ms"));
            }
            if fcfg.show_bps {
                parts.push(format!("{:.1} b/s", self.extras.volt.speed_bps));
            }
            if !parts.is_empty() {
                let line = parts.join(" | ");
                let tw = HudBatch::text_width(atlas, &line);
                let (x, y) = Self::corner_xy(fcfg.corner, wi, hi, tw + 10.0, 14.0, 0.0);
                batch.fill(atlas, x, y, tw + 10.0, 14.0, rgba(16, 18, 24, 175));
                batch.text(atlas, &line, x + 5.0, y + 3.0, rgba(185, 245, 200, 255), true);
            }
        }

        // 3) Clock / Day / Light
        let tcfg = &self.settings.extras.volt.clock_day_hud;
        if tcfg.enabled {
            let mut parts: Vec<String> = Vec::new();
            let day = self.world.day_time.div_euclid(24000);
            let tod = self.world.day_time.rem_euclid(24000);
            let total_mins = ((tod + 6000) % 24000) * 1440 / 24000;
            let hh = total_mins / 60;
            let mm = total_mins % 60;
            if tcfg.use_24h {
                parts.push(format!("{hh:02}:{mm:02}"));
            } else {
                let ampm = if hh >= 12 { "PM" } else { "AM" };
                let h12 = if hh % 12 == 0 { 12 } else { hh % 12 };
                parts.push(format!("{h12}:{mm:02}{ampm}"));
            }
            if tcfg.show_day {
                parts.push(format!("Day {day}"));
            }
            if tcfg.show_light_level {
                let l = self.world.light(
                    p.position.x.floor() as i32,
                    p.position.y.floor() as i32,
                    p.position.z.floor() as i32,
                );
                parts.push(format!("L:{}", (l & 15).max(l >> 4)));
            }
            if !parts.is_empty() {
                let line = parts.join(" · ");
                let tw = HudBatch::text_width(atlas, &line);
                let (x, y) = Self::corner_xy(tcfg.corner, wi, hi, tw + 10.0, 14.0, 0.0);
                batch.fill(atlas, x, y, tw + 10.0, 14.0, rgba(16, 18, 24, 175));
                batch.text(atlas, &line, x + 5.0, y + 3.0, rgba(255, 224, 145, 255), true);
            }
        }

        // 4) Item Counters
        let icfg = &self.settings.extras.volt.item_counters;
        if icfg.enabled {
            let mut counts = [0u32; 4];
            for slot in self.inventory.iter().chain(std::iter::once(&self.offhand)) {
                let Some(s) = slot.as_ref() else { continue };
                match &*s.item {
                    "minecraft:arrow" | "minecraft:tipped_arrow" | "minecraft:spectral_arrow"
                        if icfg.count_arrows =>
                    {
                        counts[0] += s.count;
                    }
                    "minecraft:ender_pearl" if icfg.count_pearls => counts[1] += s.count,
                    "minecraft:wind_charge" if icfg.count_wind_charges => counts[2] += s.count,
                    "minecraft:golden_apple" | "minecraft:enchanted_golden_apple"
                        if icfg.count_gapples =>
                    {
                        counts[3] += s.count;
                    }
                    _ => {}
                }
            }
            let labels = ["Arr", "Prl", "Wnd", "Gap"];
            let line = labels
                .iter()
                .zip(counts)
                .filter(|(_, c)| *c > 0 || icfg.show_zero)
                .map(|(l, c)| format!("{l}:{c}"))
                .collect::<Vec<_>>()
                .join(" ");
            if !line.is_empty() {
                let tw = HudBatch::text_width(atlas, &line);
                let x = (wi / 2 + 96) as f32;
                let y = (hi - 16) as f32;
                batch.fill(atlas, x, y, tw + 8.0, 13.0, rgba(16, 18, 24, 175));
                batch.text(atlas, &line, x + 4.0, y + 2.5, rgba(220, 232, 255, 255), true);
            }
        }

        // 5) Slime Chunk & Mob-Cap Badge
        let mcfg = &self.settings.extras.volt.slime_mobcap;
        if mcfg.enabled {
            let cx = p.position.x.floor() as i64 >> 4;
            let cz = p.position.z.floor() as i64 >> 4;
            let seed = self
                .level
                .seed
                .wrapping_add(cx.wrapping_mul(cx).wrapping_mul(0x4c1906))
                .wrapping_add(cx.wrapping_mul(0x5ac0db))
                .wrapping_add(cz.wrapping_mul(cz).wrapping_mul(0x4307a7))
                .wrapping_add(cz.wrapping_mul(0x5f24f))
                ^ 0x3ad8025f;
            let mut rng = (seed ^ 0x5DEECE66D) & ((1i64 << 48) - 1);
            rng = (rng.wrapping_mul(0x5DEECE66D).wrapping_add(0xB)) & ((1i64 << 48) - 1);
            let bits = ((rng >> 17) as i32).rem_euclid(10);
            let is_slime = self.dim == Dim::Overworld && bits == 0;
            let mut parts: Vec<String> = Vec::new();
            if mcfg.show_mob_count {
                parts.push(format!("Mobs: {}", self.mobs.len()));
            }
            if mcfg.show_slime_chunk && is_slime {
                parts.push("SLIME CHUNK".to_owned());
            }
            if !parts.is_empty() {
                let txt = parts.join(" | ");
                let tw = HudBatch::text_width(atlas, &txt);
                let (x, y) = Self::corner_xy(mcfg.corner, wi, hi, tw + 8.0, 12.0, 32.0);
                batch.fill(atlas, x, y, tw + 8.0, 12.0, rgba(16, 18, 24, 170));
                batch.text(
                    atlas,
                    &txt,
                    x + 4.0,
                    y + 2.0,
                    if is_slime {
                        rgba(128, 255, 128, 255)
                    } else {
                        rgba(215, 220, 232, 255)
                    },
                    true,
                );
            }
        }

        // 6) Respawn Anchor & Bed Status
        let rcfg = &self.settings.extras.volt.respawn_status;
        if rcfg.enabled && rcfg.show_spawn_coords {
            let line = if let Some(rc) = self.respawn_config.as_ref() {
                format!("Spawn: {}, {}, {} ({:?})", rc.pos[0], rc.pos[1], rc.pos[2], rc.dim)
            } else if let Some(sp) = self.spawn {
                format!("World Spawn: {:.0}, {:.0}, {:.0}", sp.x, sp.y, sp.z)
            } else {
                "Spawn: Unset".to_owned()
            };
            let tw = HudBatch::text_width(atlas, &line);
            let (x, y) = Self::corner_xy(rcfg.corner, wi, hi, tw + 8.0, 13.0, 18.0);
            batch.fill(atlas, x, y, tw + 8.0, 13.0, rgba(16, 18, 24, 175));
            batch.text(atlas, &line, x + 4.0, y + 2.5, rgba(195, 225, 255, 255), true);
        }
    }

    fn draw_compass_hud(&self, batch: &mut HudBatch, wi: i32, _hi: i32) {
        let atlas = &self.gui.atlas;
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let ccfg = &self.settings.extras.volt.compass_hud;
        if !ccfg.enabled {
            return;
        }
        let bw = 180.0 * (f32::from(ccfg.width_pct.clamp(50, 140)) / 100.0);
        let bx = (wi as f32 - bw) * 0.5;
        let by = 6.0;
        batch.fill(atlas, bx, by, bw, 12.0, rgba(14, 16, 22, 175));
        let yaw = self.yaw.rem_euclid(360.0);
        if ccfg.show_cardinals {
            for (deg, lbl) in [(0.0, "S"), (90.0, "W"), (180.0, "N"), (270.0, "E")] {
                let mut diff = (deg - yaw).rem_euclid(360.0);
                if diff > 180.0 {
                    diff -= 360.0;
                }
                if diff.abs() <= 75.0 {
                    let tx = bx + bw * 0.5 + (diff / 75.0) * (bw * 0.45);
                    batch.text(atlas, lbl, tx - 3.0, by + 2.0, rgba(245, 245, 255, 255), true);
                }
            }
        }
        if ccfg.show_waypoints {
            for wp in &self.extras.volt.waypoints {
                if wp.dim != self.dim || (wp.is_death && !ccfg.show_death_marker) {
                    continue;
                }
                let angle = (-(wp.pos[0] - p.position.x))
                    .atan2(wp.pos[2] - p.position.z)
                    .to_degrees() as f32;
                let mut diff = (angle - yaw).rem_euclid(360.0);
                if diff > 180.0 {
                    diff -= 360.0;
                }
                if diff.abs() <= 75.0 {
                    let tx = bx + bw * 0.5 + (diff / 75.0) * (bw * 0.45);
                    batch.fill(atlas, tx - 1.5, by + 2.0, 3.0, 7.0, wp.color);
                }
            }
        }
    }

    fn draw_look_and_portal_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;
        let lcfg = &self.settings.extras.volt.look_inspector;
        if lcfg.enabled {
            enum LookCard<'a> {
                Block {
                    name: String,
                    raw_id: String,
                    pos: [i32; 3],
                    mining_pct: Option<f32>,
                },
                Entity {
                    name: String,
                    hp: f32,
                    max_hp: f32,
                    cat: &'a str,
                },
            }
            let mut card: Option<LookCard<'_>> = None;
            if lcfg.show_mob_health
                && let Some((mob_idx, _)) = self.mob_hit()
                && let Some(m) = self.mobs.get(mob_idx)
            {
                let disp = m
                    .custom_name
                    .clone()
                    .unwrap_or_else(|| {
                        m.kind
                            .id
                            .strip_prefix("minecraft:")
                            .unwrap_or(&m.kind.id)
                            .replace('_', " ")
                    });
                card = Some(LookCard::Entity {
                    name: disp,
                    hp: m.health.max(0.0),
                    max_hp: m.kind.max_health.max(1.0),
                    cat: &m.kind.category,
                });
            } else if lcfg.show_blocks
                && let Some(pos) = self.debug_block_hit()
            {
                let id = self.world.block(pos[0], pos[1], pos[2]);
                if let Some(st) = self.pack.assets.registry.state(id) {
                    let short = st
                        .name
                        .strip_prefix("minecraft:")
                        .unwrap_or(&st.name)
                        .replace('_', " ");
                    let mining_pct = self
                        .mining
                        .as_ref()
                        .filter(|mn| mn.pos == pos && mn.progress > 0.0)
                        .map(|mn| mn.progress.clamp(0.0, 1.0));
                    card = Some(LookCard::Block {
                        name: short,
                        raw_id: st.name.clone(),
                        pos,
                        mining_pct,
                    });
                }
            }
            if let Some(c) = card {
                let y = if self.settings.extras.volt.compass_hud.enabled {
                    28.0
                } else {
                    14.0
                };
                match c {
                    LookCard::Block {
                        name,
                        raw_id,
                        pos,
                        mining_pct,
                    } => {
                        let sub = if !lcfg.compact && lcfg.show_tool_hint {
                            format!(
                                "[{}, {}, {}] • {}",
                                pos[0],
                                pos[1],
                                pos[2],
                                raw_id.strip_prefix("minecraft:").unwrap_or(&raw_id)
                            )
                        } else if !lcfg.compact && lcfg.show_mod_source {
                            "Minecraft".to_owned()
                        } else {
                            String::new()
                        };
                        let title_w = HudBatch::text_width(atlas, &name);
                        let sub_w = if sub.is_empty() {
                            0.0
                        } else {
                            HudBatch::text_width(atlas, &sub)
                        };
                        let has_icon = self.items().get(raw_id.as_str()).is_some();
                        let icon_space = if lcfg.show_3d_icon { 26.0 } else { 8.0 };
                        let cw = (icon_space + title_w.max(sub_w) + 12.0).clamp(112.0, 240.0);
                        let ch = if sub.is_empty() && mining_pct.is_none() {
                            26.0
                        } else if mining_pct.is_some() {
                            36.0
                        } else {
                            32.0
                        };
                        let cx = (wi as f32 - cw) * 0.5;
                        batch.fill(atlas, cx, y, cw, ch, rgba(16, 18, 24, 205));
                        batch.fill(atlas, cx, y, cw, 1.5, rgba(96, 185, 255, 220));
                        batch.fill(atlas, cx, y + ch - 1.0, cw, 1.0, rgba(70, 82, 105, 190));
                        if lcfg.show_3d_icon {
                            let ix = cx + 5.0;
                            let iy = y + (ch - 18.0) * 0.5;
                            batch.fill(atlas, ix - 1.0, iy - 1.0, 18.0, 18.0, rgba(255, 255, 255, 24));
                            if has_icon {
                                let stack = ItemStack {
                                    item: Arc::from(raw_id.as_str()),
                                    count: 1,
                                    damage: 0,
                                    extra: None,
                                };
                                self.draw_item_model(batch, &stack, ix, iy);
                            } else {
                                batch.fill(atlas, ix + 2.0, iy + 2.0, 12.0, 12.0, rgba(105, 185, 255, 210));
                            }
                        }
                        let tx = cx + icon_space;
                        let ty = if sub.is_empty() { y + 9.0 } else { y + 5.0 };
                        batch.text(atlas, &name, tx, ty, rgba(242, 246, 255, 255), true);
                        if !sub.is_empty() {
                            batch.text(atlas, &sub, tx, ty + 11.0, rgba(168, 182, 208, 255), true);
                        }
                        if let Some(pct) = mining_pct {
                            batch.fill(atlas, cx + 4.0, y + ch - 5.0, cw - 8.0, 3.0, rgba(35, 38, 48, 220));
                            batch.fill(
                                atlas,
                                cx + 4.0,
                                y + ch - 5.0,
                                (cw - 8.0) * pct,
                                3.0,
                                rgba(110, 232, 150, 245),
                            );
                        }
                    }
                    LookCard::Entity {
                        name,
                        hp,
                        max_hp,
                        cat,
                    } => {
                        let hp_txt = format!("{hp:.0}/{max_hp:.0} HP  •  {cat}");
                        let tw = HudBatch::text_width(atlas, &name)
                            .max(HudBatch::text_width(atlas, &hp_txt));
                        let cw = (34.0 + tw + 12.0).clamp(132.0, 250.0);
                        let ch = 34.0;
                        let cx = (wi as f32 - cw) * 0.5;
                        batch.fill(atlas, cx, y, cw, ch, rgba(18, 16, 24, 210));
                        batch.fill(atlas, cx, y, cw, 1.5, rgba(255, 110, 110, 225));
                        let ix = cx + 6.0;
                        let iy = y + 7.0;
                        batch.fill(atlas, ix, iy, 18.0, 18.0, rgba(255, 92, 92, 60));
                        batch.text(atlas, "♥", ix + 5.0, iy + 5.0, rgba(255, 105, 105, 255), true);
                        batch.text(atlas, &name, cx + 30.0, y + 5.0, rgba(248, 242, 245, 255), true);
                        batch.text(atlas, &hp_txt, cx + 30.0, y + 15.0, rgba(220, 185, 195, 255), true);
                        let ratio = (hp / max_hp).clamp(0.0, 1.0);
                        let bar_col = if ratio > 0.5 {
                            rgba(88, 232, 130, 245)
                        } else if ratio > 0.25 {
                            rgba(255, 208, 72, 245)
                        } else {
                            rgba(255, 82, 82, 245)
                        };
                        batch.fill(atlas, cx + 30.0, y + 26.0, cw - 36.0, 3.0, rgba(38, 34, 42, 220));
                        batch.fill(atlas, cx + 30.0, y + 26.0, (cw - 36.0) * ratio, 3.0, bar_col);
                    }
                }
            }
        }

        let ncfg = &self.settings.extras.volt.portal_calc;
        if ncfg.enabled
            && ncfg.show_hud_pill
            && let Some(p) = self.player.as_ref()
            && self
                .stack_in(self.selected)
                .is_some_and(|s| &*s.item == "minecraft:obsidian" || &*s.item == "minecraft:flint_and_steel")
        {
            let txt = if self.dim == Dim::Nether {
                format!("Link OW: {:.0}, {:.0}", p.position.x * 8.0, p.position.z * 8.0)
            } else {
                format!(
                    "Link Nether: {:.0}, {:.0}",
                    p.position.x / 8.0,
                    p.position.z / 8.0
                )
            };
            let tw = HudBatch::text_width(atlas, &txt);
            let (x, y) = Self::corner_xy(ncfg.corner, wi, hi, tw + 8.0, 12.0, 32.0);
            batch.fill(atlas, x, y, tw + 8.0, 12.0, rgba(32, 14, 42, 185));
            batch.text(atlas, &txt, x + 4.0, y + 2.0, rgba(220, 148, 255, 255), true);
        }
    }

    fn draw_quick_strip_and_toast(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;
        if let Some((msg, until)) = self.extras.volt.status_toast.as_ref()
            && self.ticks <= *until
        {
            let tw = HudBatch::text_width(atlas, msg);
            let x = (wi as f32 - tw) * 0.5;
            let y = hi as f32 - 58.0;
            batch.fill(atlas, x - 6.0, y - 2.0, tw + 12.0, 13.0, rgba(14, 16, 22, 210));
            batch.text(atlas, msg, x, y, rgba(255, 255, 255, 255), true);
        }

        // Biome Title Banner (#63)
        if self.settings.extras.volt.biome_title.enabled
            && let Some((ref name, until)) = self.extras.volt.biome_banner
            && self.ticks <= until
        {
            let tw = HudBatch::text_width(atlas, name);
            let x = (wi as f32 - tw) * 0.5;
            let y = hi as f32 * 0.22;
            batch.fill(atlas, x - 12.0, y - 4.0, tw + 24.0, 18.0, rgba(14, 16, 22, 185));
            batch.text(atlas, name, x, y + 1.0, rgba(245, 232, 175, 255), true);
        }

        // Chat Mention Banner (#78)
        if self.settings.extras.volt.chat_mention.enabled
            && let Some((ref msg, until)) = self.extras.volt.mention_banner
            && self.ticks <= until
        {
            let txt = format!("★ Mention: {msg}");
            let tw = HudBatch::text_width(atlas, &txt);
            let x = (wi as f32 - tw) * 0.5;
            let y = hi as f32 * 0.14;
            batch.fill(atlas, x - 8.0, y - 3.0, tw + 16.0, 15.0, rgba(36, 26, 8, 215));
            batch.text(atlas, &txt, x, y, rgba(255, 215, 72, 255), true);
        }
    }

    fn draw_standalone_potion_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.potion_hud;
        if !cfg.enabled || self.potions.effects.list.is_empty() {
            return;
        }
        let atlas = &self.gui.atlas;
        for (i, eff) in self.potions.effects.list.iter().enumerate() {
            let short = eff.id.strip_prefix("minecraft:").unwrap_or(eff.id).replace('_', " ");
            let secs = (eff.duration.max(0) / 20) as u32;
            let tstr = if eff.duration < 0 {
                "∞".to_owned()
            } else {
                format!("{:02}:{:02}", secs / 60, secs % 60)
            };
            let txt = if cfg.compact_pills {
                format!("{short} {tstr}")
            } else {
                format!("{short} {} • {tstr}", eff.amplifier + 1)
            };
            let tw = HudBatch::text_width(atlas, &txt);
            let (x, y) = Self::corner_xy(cfg.corner, wi, hi, tw + 10.0, 13.0, 44.0 + i as f32 * 14.0);
            let flash = cfg.blink_under_10s && eff.duration >= 0 && eff.duration < 200 && (self.ticks / 6).is_multiple_of(2);
            let fg = if flash {
                rgba(255, 108, 108, 255)
            } else {
                rgba(232, 238, 252, 255)
            };
            batch.fill(atlas, x, y, tw + 10.0, 13.0, rgba(16, 18, 26, 185));
            batch.text(atlas, &txt, x + 5.0, y + 2.5, fg, true);
        }
    }

    fn draw_new_hud_panels(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let atlas = &self.gui.atlas;

        // 1) Cooldown HUD (#58)
        let cd_cfg = &self.settings.extras.volt.cooldown_hud;
        if cd_cfg.enabled {
            let mut pills: Vec<(&str, f32)> = Vec::new();
            if cd_cfg.track_pearls {
                let p = self.cooldowns.percent("minecraft:ender_pearl", 0.0);
                if p > 0.0 {
                    pills.push(("Pearl", p));
                }
            }
            if cd_cfg.track_wind_charge {
                let p = self.cooldowns.percent("minecraft:wind_charge", 0.0);
                if p > 0.0 {
                    pills.push(("Wind", p));
                }
            }
            if cd_cfg.track_chorus_gapple {
                let chorus = self.cooldowns.percent("minecraft:chorus_fruit", 0.0);
                if chorus > 0.0 {
                    pills.push(("Chorus", chorus));
                }
            }
            if self.shield_cooldown > 0 {
                pills.push(("Shield", (self.shield_cooldown as f32 / 100.0).clamp(0.0, 1.0)));
            }
            for (i, (lbl, frac)) in pills.into_iter().enumerate() {
                let txt = format!("{lbl} {:.0}%", frac * 100.0);
                let tw = HudBatch::text_width(atlas, &txt);
                let x = (wi / 2 - 91 - 36) as f32 - tw - 12.0;
                let y = (hi - 46 - i as i32 * 14) as f32;
                batch.fill(atlas, x, y, tw + 10.0, 12.0, rgba(16, 18, 26, 190));
                batch.fill(atlas, x, y + 10.0, (tw + 10.0) * frac, 2.0, rgba(112, 206, 255, 240));
                batch.text(atlas, &txt, x + 5.0, y + 1.5, rgba(235, 242, 255, 255), true);
            }
        }

        // 2) Durability Warning Banner (#61)
        let dw_cfg = &self.settings.extras.volt.durability_warn;
        if dw_cfg.enabled {
            let mut worst: Option<(&str, u8)> = None;
            for s in self.armor.iter().flatten().chain(self.stack_in(self.selected)) {
                let max_d = self.items().get(&s.item).map_or(0, |i| i.max_damage);
                if max_d == 0 {
                    continue;
                }
                let rem = max_d.saturating_sub(s.damage);
                let pct = ((rem as f32 / max_d as f32) * 100.0).round() as u8;
                if pct <= dw_cfg.warn_pct && worst.is_none_or(|(_, wp)| pct < wp) {
                    let short = s.item.strip_prefix("minecraft:").unwrap_or(&s.item);
                    worst = Some((short, pct));
                }
            }
            if let Some((item, pct)) = worst {
                let msg = format!("LOW DURABILITY: {item} ({pct}%)");
                let tw = HudBatch::text_width(atlas, &msg);
                let x = (wi as f32 - tw) * 0.5;
                let y = (hi as f32) - 74.0;
                batch.fill(atlas, x - 6.0, y - 2.0, tw + 12.0, 13.0, rgba(58, 16, 16, 215));
                batch.text(atlas, &msg, x, y, rgba(255, 118, 118, 255), true);
            }
        }

        // 4) XP Tracker (#64)
        let xp_cfg = &self.settings.extras.volt.xp_tracker;
        if xp_cfg.enabled {
            let secs = self
                .extras
                .volt
                .xp_session_start
                .map_or(1.0, |s| s.elapsed().as_secs_f32().max(1.0));
            let rate = (self.extras.volt.xp_gained_session as f32 / secs) * 60.0;
            let mut txt = format!("Lvl {}", self.experience.level);
            if xp_cfg.show_next_level_xp {
                txt.push_str(&format!(" • +{:.0} XP/m", rate));
            }
            let tw = HudBatch::text_width(atlas, &txt);
            let (x, y) = Self::corner_xy(xp_cfg.corner, wi, hi, tw + 10.0, 14.0, 46.0);
            batch.fill(atlas, x, y, tw + 10.0, 14.0, rgba(16, 22, 16, 185));
            batch.text(atlas, &txt, x + 5.0, y + 3.0, rgba(132, 255, 112, 255), true);
        }

        // 5) Mini Inventory HUD (#65)
        let inv_cfg = &self.settings.extras.volt.inventory_hud;
        if inv_cfg.enabled {
            let pw = 9.0 * 18.0 + 4.0;
            let ph = 3.0 * 18.0 + 4.0;
            let (x0, y0) = Self::corner_xy(inv_cfg.corner, wi, hi, pw, ph, 16.0);
            let bg_a = ((f32::from(inv_cfg.bg_opacity_pct.clamp(15, 100)) / 100.0) * 215.0) as u8;
            batch.fill(atlas, x0, y0, pw, ph, rgba(16, 18, 24, bg_a));
            for slot in 9..36usize {
                let col = (slot - 9) % 9;
                let row = (slot - 9) / 9;
                let sx = x0 + 2.0 + col as f32 * 18.0;
                let sy = y0 + 2.0 + row as f32 * 18.0;
                batch.fill(atlas, sx, sy, 16.0, 16.0, rgba(255, 255, 255, 16));
                if let Some(stack) = self.inventory.get(slot).and_then(|o| o.as_ref()) {
                    self.draw_item(batch, stack, sx, sy);
                    if stack.count > 1 {
                        batch.text(
                            atlas,
                            &format!("{}", stack.count),
                            sx + 7.0,
                            sy + 8.0,
                            u32::MAX,
                            true,
                        );
                    }
                }
            }
        }

        // 6) Moon Phase (#67)
        let mp_cfg = &self.settings.extras.volt.moon_phase;
        if mp_cfg.enabled {
            let phase = (self.world.day_time.div_euclid(24000)).rem_euclid(8);
            let (name, slime_pct) = match phase {
                0 => ("Full Moon", 100),
                1 => ("Waning Gibbous", 75),
                2 => ("Last Quarter", 50),
                3 => ("Waning Crescent", 25),
                4 => ("New Moon", 0),
                5 => ("Waxing Crescent", 25),
                6 => ("First Quarter", 50),
                _ => ("Waxing Gibbous", 75),
            };
            let txt = if mp_cfg.show_slime_rating {
                format!("{name} • Slime {slime_pct}%")
            } else {
                name.to_owned()
            };
            let tw = HudBatch::text_width(atlas, &txt);
            let (x, y) = Self::corner_xy(mp_cfg.corner, wi, hi, tw + 10.0, 13.0, 16.0);
            batch.fill(atlas, x, y, tw + 10.0, 13.0, rgba(16, 18, 28, 185));
            batch.text(atlas, &txt, x + 5.0, y + 2.5, rgba(205, 224, 255, 255), true);
        }

        // 7) Session Stats (#68)
        let ss_cfg = &self.settings.extras.volt.session_stats;
        if ss_cfg.enabled {
            let secs = self
                .extras
                .volt
                .session_started
                .map_or(0, |s| s.elapsed().as_secs());
            let mut parts = Vec::new();
            parts.push(format!("{:02}:{:02}", secs / 60, secs % 60));
            if ss_cfg.show_blocks {
                parts.push(format!(
                    "M:{} P:{}",
                    self.extras.volt.session_blocks_mined,
                    self.extras.volt.session_blocks_placed
                ));
            }
            if ss_cfg.show_kills {
                parts.push(format!("Kills:{}", self.extras.volt.session_mob_kills));
            }
            let txt = parts.join(" · ");
            let tw = HudBatch::text_width(atlas, &txt);
            let (x, y) = Self::corner_xy(ss_cfg.corner, wi, hi, tw + 10.0, 13.0, 62.0);
            batch.fill(atlas, x, y, tw + 10.0, 13.0, rgba(16, 18, 24, 180));
            batch.text(atlas, &txt, x + 5.0, y + 2.5, rgba(226, 234, 248, 255), true);
        }

        // 8) Litematica Ruler Distance HUD (#74)
        if self.settings.extras.volt.litematica_grid.enabled
            && let (Some(a), Some(b)) = (self.extras.volt.ruler_a, self.extras.volt.ruler_b)
        {
            let dx = (a[0] - b[0]).abs() + 1;
            let dy = (a[1] - b[1]).abs() + 1;
            let dz = (a[2] - b[2]).abs() + 1;
            let vol = i64::from(dx) * i64::from(dy) * i64::from(dz);
            let txt = format!("Ruler: {dx}×{dy}×{dz} ({vol} blocks)");
            let tw = HudBatch::text_width(atlas, &txt);
            let x = (wi as f32 - tw) * 0.5;
            batch.fill(atlas, x - 6.0, 48.0, tw + 12.0, 13.0, rgba(16, 22, 28, 195));
            batch.text(atlas, &txt, x, 50.5, rgba(105, 242, 255, 255), true);
        }
    }

    // =========================================================================
    // 9. JEI (Just Enough Items) Interactive Overlay + Recipe Browser (#82)
    // =========================================================================

    pub(super) fn volt_jei_visible(&self) -> bool {
        let cfg = &self.settings.extras.volt.jei;
        if !cfg.enabled {
            return false;
        }
        self.extras.volt.jei_open
            || (cfg.show_in_inventory && matches!(self.screen, Screen::Inventory | Screen::Creative))
    }

    fn jei_filtered_items(&self) -> Vec<String> {
        let q = self.extras.volt.jei_search.trim().to_ascii_lowercase();
        let items = self.items();
        let src = if items.registry_order.is_empty() {
            &items.order
        } else {
            &items.registry_order
        };
        src.iter()
            .filter(|id| {
                if q.is_empty() {
                    return true;
                }
                id.to_ascii_lowercase().contains(&q)
            })
            .map(|id| id.to_string())
            .collect()
    }

    fn jei_top_offset(&self) -> f32 {
        let mut top: f32 = match self.settings.horizon.overlay {
            horizon_client::graphics::Overlay::Graph => 82.0,
            horizon_client::graphics::Overlay::Fps => 40.0,
            horizon_client::graphics::Overlay::Off => 6.0,
        };
        if self.settings.extras.volt.telemetry_hud.enabled
            && self.settings.extras.volt.telemetry_hud.corner == Corner::TopRight
        {
            top = top.max(24.0);
        }
        if self.settings.extras.volt.tab_ping.enabled
            && self.settings.extras.volt.tab_ping.corner == Corner::TopRight
        {
            top = top.max(38.0);
        }
        top
    }

    fn jei_panel_rect(&self, wi: f32, hi: f32) -> [f32; 4] {
        let pw = (6.0_f32 * 18.0 + 8.0).min(wi * 0.34);
        let py = self.jei_top_offset();
        let ph = (hi - py - 6.0).max(112.0);
        [wi - pw - 6.0, py, pw, ph]
    }

    fn jei_grid_layout(pw: f32, ph: f32) -> (usize, usize) {
        let cols = ((pw - 8.0) / 18.0).floor().clamp(4.0, 7.0) as usize;
        let rows = ((ph - 38.0) / 18.0).floor().max(3.0) as usize;
        (cols, rows)
    }

    fn jei_item_label(id: &str) -> String {
        let raw = id.strip_prefix("minecraft:").unwrap_or(id);
        let mut out = String::with_capacity(raw.len());
        for (i, word) in raw.split('_').enumerate() {
            if i > 0 {
                out.push(' ');
            }
            let mut chs = word.chars();
            if let Some(first) = chs.next() {
                for up in first.to_uppercase() {
                    out.push(up);
                }
                out.push_str(chs.as_str());
            }
        }
        out
    }

    fn jei_recipe_views_for(&self, item_id: &str) -> Vec<JeiRecipeView> {
        let mut out = Vec::new();
        let recipes = self.recipes();
        for (_rid, rec) in &recipes.crafting {
            match rec {
                Recipe::Shaped {
                    width,
                    height,
                    cells,
                    result,
                } => {
                    if result.item.as_deref() == Some(item_id) {
                        let mut grid: [Option<String>; 9] = Default::default();
                        for r in 0..(*height).min(3) {
                            for c in 0..(*width).min(3) {
                                if let Some(ing) = cells.get(r * width + c).and_then(|o| o.as_ref())
                                    && let Some(first) = ing.1.first()
                                {
                                    grid[r * 3 + c] = Some(first.clone());
                                }
                            }
                        }
                        out.push(JeiRecipeView::Crafting {
                            shapeless: false,
                            grid,
                            output: item_id.to_owned(),
                            count: result.count.max(1),
                        });
                    }
                }
                Recipe::Shapeless {
                    ingredients,
                    result,
                } => {
                    if result.item.as_deref() == Some(item_id) {
                        let mut grid: [Option<String>; 9] = Default::default();
                        for (idx, ing) in ingredients.iter().take(9).enumerate() {
                            if let Some(first) = ing.1.first() {
                                grid[idx] = Some(first.clone());
                            }
                        }
                        out.push(JeiRecipeView::Crafting {
                            shapeless: true,
                            grid,
                            output: item_id.to_owned(),
                            count: result.count.max(1),
                        });
                    }
                }
                Recipe::Special(_) => {}
            }
        }
        for Cooking {
            kind,
            ingredient,
            result,
            experience,
            time,
            ..
        } in &recipes.cooking
        {
            if &*result.item == item_id {
                let in_item = ingredient
                    .1
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "minecraft:air".to_owned());
                let k = kind.strip_prefix("minecraft:").unwrap_or(kind);
                out.push(JeiRecipeView::Cooking {
                    kind: Self::jei_item_label(k),
                    input: in_item,
                    output: item_id.to_owned(),
                    count: result.count.max(1),
                    xp: *experience,
                    secs: (*time as f32 / 20.0).max(0.5),
                });
            }
        }
        out
    }

    fn draw_mc_slot(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, size: f32) {
        batch.fill(atlas, x, y, size, size, rgba(139, 139, 139, 255));
        batch.fill(atlas, x, y, size - 1.0, 1.0, rgba(55, 55, 55, 255));
        batch.fill(atlas, x, y, 1.0, size - 1.0, rgba(55, 55, 55, 255));
        batch.fill(atlas, x + 1.0, y + size - 1.0, size - 1.0, 1.0, rgba(255, 255, 255, 230));
        batch.fill(atlas, x + size - 1.0, y + 1.0, 1.0, size - 1.0, rgba(255, 255, 255, 230));
    }

    fn jei_recipe_rect(px: f32, hi: f32) -> [f32; 4] {
        let rw = 148.0_f32.min((px - 12.0).max(120.0));
        let rh = 114.0_f32;
        let rx = (px - rw - 8.0).max(6.0);
        let ry = ((hi - rh) * 0.5).max(16.0);
        [rx, ry, rw, rh]
    }

    pub(super) fn draw_volt_jei_overlay(&self, batch: &mut HudBatch) {
        if !self.volt_jei_visible() {
            return;
        }
        let atlas = &self.gui.atlas;
        let [px, py, pw, ph] = self.jei_panel_rect(batch.width, batch.height);
        let items = self.jei_filtered_items();
        let (cols, rows) = Self::jei_grid_layout(pw, ph);
        let per_page = (cols * rows).max(1);
        let pages = items.len().div_ceil(per_page).max(1);
        let page = self.extras.volt.jei_page.min(pages - 1);

        // Top pagination bar (<  1/N  >) matching classic JEI layout
        let prev_col = if page > 0 { rgba(52, 56, 68, 225) } else { rgba(28, 30, 38, 185) };
        let next_col = if page + 1 < pages { rgba(52, 56, 68, 225) } else { rgba(28, 30, 38, 185) };
        batch.fill(atlas, px, py, 16.0, 14.0, prev_col);
        batch.fill(atlas, px, py, 16.0, 1.0, rgba(160, 168, 184, 210));
        batch.text(atlas, "<", px + 5.0, py + 3.0, u32::MAX, true);

        let ptxt = format!("{}/{}", page + 1, pages);
        let ptw = HudBatch::text_width(atlas, &ptxt);
        batch.text(atlas, &ptxt, px + (pw - ptw) * 0.5, py + 3.0, u32::MAX, true);

        batch.fill(atlas, px + pw - 16.0, py, 16.0, 14.0, next_col);
        batch.fill(atlas, px + pw - 16.0, py, 16.0, 1.0, rgba(160, 168, 184, 210));
        batch.text(atlas, ">", px + pw - 11.0, py + 3.0, u32::MAX, true);

        // Clean transparent right-side item grid (no dark card, no per-slot boxes)
        let start = page * per_page;
        let end = (start + per_page).min(items.len());
        let gx = px + 4.0;
        let gy = py + 18.0;
        let (mx, my) = self.cursor;
        let mut hovered_item: Option<String> = None;
        for (i, item_id) in items[start..end].iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let sx = gx + col as f32 * 18.0;
            let sy = gy + row as f32 * 18.0;
            let hovered = mx >= sx && mx <= sx + 17.0 && my >= sy && my <= sy + 17.0;
            let selected = self
                .extras
                .volt
                .jei_selected_item
                .as_deref()
                .is_some_and(|s| s == item_id);
            if hovered {
                hovered_item = Some(item_id.clone());
                batch.fill(atlas, sx, sy, 16.0, 16.0, rgba(255, 255, 255, 110));
            } else if selected {
                batch.fill(atlas, sx, sy, 16.0, 16.0, rgba(255, 255, 255, 68));
            }
            let stack = ItemStack {
                item: Arc::from(item_id.as_str()),
                count: 1,
                damage: 0,
                extra: None,
            };
            self.draw_item_model(batch, &stack, sx, sy);
        }

        // Bottom framed search bar + clear/close icon button
        let fy = py + ph - 16.0;
        let sw = (pw - 19.0).max(40.0);
        batch.fill(atlas, px, fy, sw, 15.0, rgba(6, 8, 12, 210));
        let border_c = if self.extras.volt.jei_search_focused {
            rgba(120, 200, 255, 240)
        } else {
            rgba(156, 162, 174, 220)
        };
        batch.fill(atlas, px, fy, sw, 1.0, border_c);
        batch.fill(atlas, px, fy + 14.0, sw, 1.0, border_c);
        batch.fill(atlas, px, fy, 1.0, 15.0, border_c);
        batch.fill(atlas, px + sw - 1.0, fy, 1.0, 15.0, border_c);
        let search_txt = if self.extras.volt.jei_search.is_empty() {
            "Search..."
        } else {
            &self.extras.volt.jei_search
        };
        batch.text(
            atlas,
            search_txt,
            px + 4.0,
            fy + 3.5,
            if self.extras.volt.jei_search.is_empty() {
                rgba(135, 142, 158, 255)
            } else {
                u32::MAX
            },
            true,
        );
        let bx = px + pw - 16.0;
        batch.fill(atlas, bx, fy, 16.0, 15.0, rgba(46, 50, 62, 225));
        batch.fill(atlas, bx, fy, 16.0, 1.0, rgba(156, 162, 174, 220));
        let btn_lbl = if !self.extras.volt.jei_search.is_empty() { "×" } else { "x" };
        batch.text(atlas, btn_lbl, bx + 5.0, fy + 3.5, rgba(225, 232, 245, 255), true);

        // Visual Crafting / Smelting Recipe Guide Popup
        if let Some(ref sel) = self.extras.volt.jei_selected_item {
            let recs = self.jei_recipe_views_for(sel);
            let [rx, ry, rw, rh] = Self::jei_recipe_rect(px, batch.height);

            // Classic MC light-gray recipe panel with crisp bevel
            batch.fill(atlas, rx, ry, rw, rh, rgba(198, 198, 198, 248));
            batch.fill(atlas, rx, ry, rw, 1.5, rgba(255, 255, 255, 240));
            batch.fill(atlas, rx, ry, 1.5, rh, rgba(255, 255, 255, 240));
            batch.fill(atlas, rx, ry + rh - 1.5, rw, 1.5, rgba(85, 85, 85, 245));
            batch.fill(atlas, rx + rw - 1.5, ry, 1.5, rh, rgba(85, 85, 85, 245));

            // Close [x] button
            batch.fill(atlas, rx + rw - 16.0, ry + 4.0, 12.0, 12.0, rgba(120, 48, 48, 235));
            batch.text(atlas, "x", rx + rw - 12.5, ry + 6.0, u32::MAX, true);

            let title = Self::jei_item_label(sel);
            let short_title: String = title.chars().take(20).collect();
            batch.text(atlas, &short_title, rx + 7.0, ry + 6.0, rgba(44, 44, 44, 255), false);

            if recs.is_empty() {
                Self::draw_mc_slot(batch, atlas, rx + (rw - 22.0) * 0.5, ry + 34.0, 22.0);
                let st = ItemStack {
                    item: Arc::from(sel.as_str()),
                    count: 1,
                    damage: 0,
                    extra: None,
                };
                self.draw_item_model(batch, &st, rx + (rw - 22.0) * 0.5 + 3.0, ry + 37.0);
                let msg = "No recipe";
                let mw = HudBatch::text_width(atlas, msg);
                batch.text(atlas, msg, rx + (rw - mw) * 0.5, ry + 66.0, rgba(72, 72, 72, 255), false);
            } else {
                let r_idx = self.extras.volt.jei_recipe_idx % recs.len();
                let view = &recs[r_idx];
                let cat_lbl = match view {
                    JeiRecipeView::Crafting { shapeless: false, .. } => "Crafting",
                    JeiRecipeView::Crafting { shapeless: true, .. } => "Shapeless",
                    JeiRecipeView::Cooking { kind, .. } => kind.as_str(),
                };
                batch.text(atlas, cat_lbl, rx + 7.0, ry + 18.0, rgba(78, 78, 78, 255), false);

                if recs.len() > 1 {
                    let p_x = rx + rw - 54.0;
                    let p_y = ry + 17.0;
                    batch.fill(atlas, p_x, p_y, 12.0, 11.0, rgba(130, 130, 130, 230));
                    batch.text(atlas, "<", p_x + 3.5, p_y + 1.5, u32::MAX, true);
                    let rp = format!("{}/{}", r_idx + 1, recs.len());
                    batch.text(atlas, &rp, p_x + 14.0, p_y + 1.5, rgba(44, 44, 44, 255), false);
                    batch.fill(atlas, p_x + 34.0, p_y, 12.0, 11.0, rgba(130, 130, 130, 230));
                    batch.text(atlas, ">", p_x + 37.5, p_y + 1.5, u32::MAX, true);
                }

                match view {
                    JeiRecipeView::Crafting { grid, output, count, .. } => {
                        let gx0 = rx + 10.0;
                        let gy0 = ry + 33.0;
                        for row in 0..3 {
                            for col in 0..3 {
                                let sx = gx0 + col as f32 * 18.0;
                                let sy = gy0 + row as f32 * 18.0;
                                Self::draw_mc_slot(batch, atlas, sx, sy, 18.0);
                                if let Some(ing_id) = &grid[row * 3 + col] {
                                    let st = ItemStack {
                                        item: Arc::from(ing_id.as_str()),
                                        count: 1,
                                        damage: 0,
                                        extra: None,
                                    };
                                    self.draw_item_model(batch, &st, sx + 1.0, sy + 1.0);
                                }
                            }
                        }
                        // Crafting arrow ->
                        let ax = gx0 + 58.0;
                        let ay = gy0 + 23.0;
                        batch.fill(atlas, ax, ay + 3.0, 11.0, 3.0, rgba(96, 96, 96, 255));
                        batch.fill(atlas, ax + 9.0, ay + 1.0, 3.0, 7.0, rgba(96, 96, 96, 255));
                        batch.fill(atlas, ax + 12.0, ay + 3.0, 2.0, 3.0, rgba(96, 96, 96, 255));

                        // Result slot
                        let ox = gx0 + 78.0;
                        let oy = gy0 + 15.0;
                        Self::draw_mc_slot(batch, atlas, ox, oy, 24.0);
                        let out_st = ItemStack {
                            item: Arc::from(output.as_str()),
                            count: *count,
                            damage: 0,
                            extra: None,
                        };
                        self.draw_item_model(batch, &out_st, ox + 4.0, oy + 4.0);
                        if *count > 1 {
                            let ctxt = count.to_string();
                            let ctw = HudBatch::text_width(atlas, &ctxt);
                            batch.text(atlas, &ctxt, ox + 22.0 - ctw, oy + 14.0, u32::MAX, true);
                        }
                    }
                    JeiRecipeView::Cooking { input, output, count, xp, secs, .. } => {
                        let ix = rx + 24.0;
                        let iy = ry + 36.0;
                        Self::draw_mc_slot(batch, atlas, ix, iy, 18.0);
                        let in_st = ItemStack {
                            item: Arc::from(input.as_str()),
                            count: 1,
                            damage: 0,
                            extra: None,
                        };
                        self.draw_item_model(batch, &in_st, ix + 1.0, iy + 1.0);
                        // Flame indicator
                        batch.fill(atlas, ix + 3.0, iy + 22.0, 12.0, 10.0, rgba(255, 136, 32, 235));
                        batch.fill(atlas, ix + 6.0, iy + 24.0, 6.0, 6.0, rgba(255, 224, 96, 245));
                        // Arrow
                        let ax = ix + 28.0;
                        let ay = iy + 6.0;
                        batch.fill(atlas, ax, ay + 3.0, 14.0, 3.0, rgba(96, 96, 96, 255));
                        batch.fill(atlas, ax + 12.0, ay + 1.0, 3.0, 7.0, rgba(96, 96, 96, 255));
                        // Output slot
                        let ox = ix + 52.0;
                        let oy = iy - 3.0;
                        Self::draw_mc_slot(batch, atlas, ox, oy, 24.0);
                        let out_st = ItemStack {
                            item: Arc::from(output.as_str()),
                            count: *count,
                            damage: 0,
                            extra: None,
                        };
                        self.draw_item_model(batch, &out_st, ox + 4.0, oy + 4.0);
                        if *count > 1 {
                            let ctxt = count.to_string();
                            let ctw = HudBatch::text_width(atlas, &ctxt);
                            batch.text(atlas, &ctxt, ox + 22.0 - ctw, oy + 14.0, u32::MAX, true);
                        }
                        let meta_txt = format!("{secs:.0}s  •  {xp:.1} XP");
                        batch.text(atlas, &meta_txt, rx + 12.0, ry + 78.0, rgba(64, 64, 64, 255), false);
                    }
                }
            }

            if self.creative {
                let gx = rx + rw - 44.0;
                let gy = ry + rh - 18.0;
                batch.fill(atlas, gx, gy, 36.0, 13.0, rgba(62, 118, 64, 235));
                batch.text(atlas, "Give", gx + 8.0, gy + 2.5, u32::MAX, true);
            }
        }

        // Mouse hover tooltip for JEI grid items
        if let Some(ref item_id) = hovered_item {
            let label = Self::jei_item_label(item_id);
            let show_id = self.settings.extras.volt.jei.show_item_ids;
            let lw = HudBatch::text_width(atlas, &label);
            let iw = if show_id {
                HudBatch::text_width(atlas, item_id)
            } else {
                0.0
            };
            let tw = lw.max(iw) + 10.0;
            let th = if show_id { 24.0 } else { 14.0 };
            let tx = (mx - tw - 6.0).clamp(4.0, (batch.width - tw - 4.0).max(4.0));
            let ty = (my - th * 0.5).clamp(4.0, (batch.height - th - 4.0).max(4.0));
            batch.fill(atlas, tx, ty, tw, th, rgba(16, 8, 28, 238));
            batch.fill(atlas, tx, ty, tw, 1.0, rgba(96, 64, 180, 235));
            batch.fill(atlas, tx, ty + th - 1.0, tw, 1.0, rgba(96, 64, 180, 235));
            batch.fill(atlas, tx, ty, 1.0, th, rgba(96, 64, 180, 235));
            batch.fill(atlas, tx + tw - 1.0, ty, 1.0, th, rgba(96, 64, 180, 235));
            batch.text(atlas, &label, tx + 5.0, ty + 3.0, u32::MAX, true);
            if show_id {
                batch.text(atlas, item_id, tx + 5.0, ty + 13.0, rgba(160, 168, 184, 255), true);
            }
        }
    }

    pub(super) fn volt_jei_contains_point(&self, w: f32, h: f32, pt: [f32; 2]) -> bool {
        if !self.volt_jei_visible() {
            return false;
        }
        let [px, py, pw, ph] = self.jei_panel_rect(w, h);
        let (cx, cy) = (pt[0], pt[1]);
        if cx >= px && cx <= px + pw && cy >= py && cy <= py + ph {
            return true;
        }
        if self.extras.volt.jei_selected_item.is_some() {
            let [rx, ry, rw, rh] = Self::jei_recipe_rect(px, h);
            if cx >= rx && cx <= rx + rw && cy >= ry && cy <= ry + rh {
                return true;
            }
        }
        false
    }

    pub(super) fn volt_jei_contains(&self, w: f32, h: f32) -> bool {
        self.volt_jei_contains_point(w, h, [self.cursor.0, self.cursor.1])
    }

    pub(super) fn volt_jei_click(&mut self, w: f32, h: f32, right: bool) -> bool {
        if !self.volt_jei_visible() {
            return false;
        }
        let [px, py, pw, ph] = self.jei_panel_rect(w, h);
        let (cx, cy) = self.cursor;

        // First check recipe popup interactions if open
        if let Some(sel) = self.extras.volt.jei_selected_item.clone() {
            let [rx, ry, rw, rh] = Self::jei_recipe_rect(px, h);
            if cx >= rx && cx <= rx + rw && cy >= ry && cy <= ry + rh {
                self.extras.volt.jei_search_focused = false;
                if cx >= rx + rw - 17.0 && cy <= ry + 17.0 {
                    self.extras.volt.jei_selected_item = None;
                    return true;
                }
                if self.creative && cx >= rx + rw - 44.0 && cy >= ry + rh - 18.0 {
                    let _ = self.give(ItemStack {
                        item: Arc::from(sel.as_str()),
                        count: 1,
                        damage: 0,
                        extra: None,
                    });
                    return true;
                }
                let recs = self.jei_recipe_views_for(&sel);
                if recs.len() > 1 {
                    let p_x = rx + rw - 54.0;
                    let p_y = ry + 17.0;
                    if cy >= p_y - 2.0 && cy <= p_y + 13.0 {
                        if cx >= p_x && cx <= p_x + 14.0 {
                            self.extras.volt.jei_recipe_idx =
                                (self.extras.volt.jei_recipe_idx + recs.len() - 1) % recs.len();
                            return true;
                        }
                        if cx >= p_x + 32.0 && cx <= p_x + 48.0 {
                            self.extras.volt.jei_recipe_idx =
                                (self.extras.volt.jei_recipe_idx + 1) % recs.len();
                            return true;
                        }
                    }
                }
                if !recs.is_empty() {
                    let r_idx = self.extras.volt.jei_recipe_idx % recs.len();
                    match &recs[r_idx] {
                        JeiRecipeView::Crafting { grid, .. } => {
                            let gx0 = rx + 10.0;
                            let gy0 = ry + 33.0;
                            for row in 0..3 {
                                for col in 0..3 {
                                    let sx = gx0 + col as f32 * 18.0;
                                    let sy = gy0 + row as f32 * 18.0;
                                    if cx >= sx && cx <= sx + 18.0 && cy >= sy && cy <= sy + 18.0 {
                                        if let Some(ing_id) = &grid[row * 3 + col] {
                                            self.extras.volt.jei_selected_item = Some(ing_id.clone());
                                            self.extras.volt.jei_recipe_idx = 0;
                                        }
                                        return true;
                                    }
                                }
                            }
                        }
                        JeiRecipeView::Cooking { input, .. } => {
                            let ix = rx + 24.0;
                            let iy = ry + 36.0;
                            if cx >= ix && cx <= ix + 18.0 && cy >= iy && cy <= iy + 18.0 {
                                self.extras.volt.jei_selected_item = Some(input.clone());
                                self.extras.volt.jei_recipe_idx = 0;
                                return true;
                            }
                        }
                    }
                }
                return true;
            }
        }

        if cx < px || cx > px + pw || cy < py || cy > py + ph {
            self.extras.volt.jei_search_focused = false;
            return false;
        }

        let items = self.jei_filtered_items();
        let (cols, rows) = Self::jei_grid_layout(pw, ph);
        let per_page = (cols * rows).max(1);
        let pages = items.len().div_ceil(per_page).max(1);
        let page = self.extras.volt.jei_page.min(pages - 1);

        // Top pagination bar
        if cy >= py && cy <= py + 15.0 {
            self.extras.volt.jei_search_focused = false;
            if cx <= px + 20.0 && page > 0 {
                self.extras.volt.jei_page = page - 1;
            } else if cx >= px + pw - 20.0 && page + 1 < pages {
                self.extras.volt.jei_page = page + 1;
            }
            return true;
        }

        // Bottom search bar & clear/close button
        let fy = py + ph - 16.0;
        let sw = (pw - 19.0).max(40.0);
        if cy >= fy - 1.0 && cy <= fy + 16.0 {
            if cx >= px + pw - 16.0 {
                self.extras.volt.jei_search_focused = false;
                if !self.extras.volt.jei_search.is_empty() {
                    self.extras.volt.jei_search.clear();
                    self.extras.volt.jei_page = 0;
                } else {
                    self.extras.volt.jei_open = false;
                    self.extras.volt.jei_selected_item = None;
                    if self.screen == Screen::Playing {
                        self.capture(true);
                    }
                }
                return true;
            }
            if cx <= px + sw {
                if right {
                    self.extras.volt.jei_search.clear();
                    self.extras.volt.jei_page = 0;
                    self.extras.volt.jei_search_focused = true;
                } else {
                    self.extras.volt.jei_search_focused = true;
                }
                #[cfg(target_os = "android")]
                crate::platform::show(&self.extras.volt.jei_search);
                return true;
            }
        }
        self.extras.volt.jei_search_focused = false;

        // Item grid
        let start = page * per_page;
        let end = (start + per_page).min(items.len());
        let gx = px + 4.0;
        let gy = py + 18.0;
        for (i, item_id) in items[start..end].iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let sx = gx + col as f32 * 18.0;
            let sy = gy + row as f32 * 18.0;
            if cx >= sx && cx <= sx + 17.0 && cy >= sy && cy <= sy + 17.0 {
                if self.creative && (self.modifiers.shift_key() || right) {
                    let count = if self.modifiers.shift_key() { 64 } else { 1 };
                    let _ = self.give(ItemStack {
                        item: Arc::from(item_id.as_str()),
                        count,
                        damage: 0,
                        extra: None,
                    });
                } else if self.extras.volt.jei_selected_item.as_deref() == Some(item_id.as_str()) {
                    self.extras.volt.jei_selected_item = None;
                } else {
                    self.extras.volt.jei_selected_item = Some(item_id.clone());
                    self.extras.volt.jei_recipe_idx = 0;
                }
                return true;
            }
        }
        true
    }

    pub(super) fn volt_jei_scroll(&mut self, delta: f32, w: f32, h: f32) -> bool {
        if !self.volt_jei_visible() {
            return false;
        }
        let [px, py, pw, ph] = self.jei_panel_rect(w, h);
        let (cx, cy) = self.cursor;
        if cx < px || cx > px + pw || cy < py || cy > py + ph {
            return false;
        }
        let items = self.jei_filtered_items();
        let (cols, rows) = Self::jei_grid_layout(pw, ph);
        let per_page = (cols * rows).max(1);
        let pages = items.len().div_ceil(per_page).max(1);
        if delta < 0.0 && self.extras.volt.jei_page + 1 < pages {
            self.extras.volt.jei_page += 1;
        } else if delta > 0.0 && self.extras.volt.jei_page > 0 {
            self.extras.volt.jei_page -= 1;
        }
        true
    }

    pub(super) fn volt_jei_key(&mut self, key: KeyCode, typed: Option<&str>) -> bool {
        if !self.volt_jei_visible() || !self.extras.volt.jei_search_focused {
            return false;
        }
        if key == KeyCode::Escape {
            self.extras.volt.jei_search_focused = false;
            return true;
        }
        if key == KeyCode::Backspace {
            self.extras.volt.jei_search.pop();
            self.extras.volt.jei_page = 0;
            return true;
        }
        if let Some(txt) = typed {
            for ch in txt.chars().filter(|c| !c.is_control()) {
                self.extras.volt.jei_search.push(ch);
            }
            self.extras.volt.jei_page = 0;
            return true;
        }
        false
    }
}
