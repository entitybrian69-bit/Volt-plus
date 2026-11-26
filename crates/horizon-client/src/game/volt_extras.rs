//! Built-in Volt+ Extras mods (`AppleSkin`, `Marlow's Crystal Optimizer`,
//! `Shield Status`, and `Uku's Armor and Potion HUD`), all off by default
//! and controlled from the `Volt+ Extras` tab in the Control Hub.
use super::Game;
use horizon_client::{
    extras::{
        AppleSkinConfig, ShieldColorPreset, ShieldDisplayMode, UkuAnchor, UkuDurability, UkuSide,
        UkuStyle,
    },
    hud::{HudAtlas, HudBatch, rgba},
    hud_layout,
};
use horizon_world::inventory::ItemStack;
use std::time::{Duration, Instant};

impl Game<'_> {
    // =========================================================================
    // 1. AppleSkin
    // =========================================================================

    /// Returns `(nutrition, saturation_gain)` for a food item stack, if edible.
    pub(super) fn appleskin_food_values(&self, stack: &ItemStack) -> Option<(i32, f32)> {
        let info = self.items().get(&stack.item)?;
        let (nutrition, saturation_gain) = info.food?;
        if nutrition == 0 && saturation_gain <= 0.0 {
            return None;
        }
        Some((nutrition as i32, saturation_gain))
    }

    fn appleskin_can_preview_food(&self, cfg: &AppleSkinConfig, food_level: i32, stack: &ItemStack) -> bool {
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

    /// Draws AppleSkin's exhaustion underlay, gold saturation outlines, held-food
    /// hunger/saturation preview, and predicted health regeneration hearts.
    pub(super) fn draw_appleskin_hud(
        &self,
        batch: &mut HudBatch,
        inputs: &hud_layout::HealthInputs,
        exhaustion: f32,
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

        // 1) Exhaustion underlay (4.0 exhaustion = 1 hunger drain cycle)
        if cfg.show_exhaustion_underlay {
            let ratio = (exhaustion / 4.0).clamp(0.0, 1.0);
            if ratio > 0.005 {
                let w = (81.0 * ratio).round().max(1.0);
                let x = x_right as f32 - w;
                batch.fill(atlas, x, y_food as f32, w, 9.0, rgba(255, 255, 255, 76));
            }
        }

        // Determine held food preview (main hand, fallback offhand)
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

        // Flashing alpha in AppleSkin style (0.2 .. 0.85 sine pulse)
        let pulse = (((inputs.millis as f32 / 1000.0) * 4.5).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        let preview_alpha = ((0.25 + 0.60 * pulse) * 255.0).round() as u8;

        // 2) Held-food hunger preview drumsticks (before saturation outlines so gold rims sit on top)
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

            // 3) Gold saturation outlines (current + preview)
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

            // 4) Predicted health regeneration hearts when eating brings food >= 18
            if cfg.show_health_regen_overlay && inputs.health < inputs.max_health {
                let target_sat = (inputs.saturation + add_sat).clamp(0.0, target_food as f32);
                let estimated_regen = if target_food >= 20 && target_sat > 0.0 {
                    (target_sat / 6.0).min(inputs.max_health - inputs.health)
                } else if target_food >= 18 {
                    (2.0_f32).min(inputs.max_health - inputs.health)
                } else {
                    0.0
                };
                if estimated_regen >= 0.5 {
                    let new_health = (inputs.health + estimated_regen).min(inputs.max_health);
                    let x_left = wi / 2 - 91;
                    let rows = ((inputs.max_health + inputs.absorption).ceil() as i32 + 1) / 20;
                    let row_height = (10 - (rows - 2)).max(3);
                    let base_y = hi - 39;
                    let cur_half = inputs.health.ceil() as i32;
                    let new_half = new_health.ceil() as i32;
                    let heart_col = rgba(255, 255, 255, preview_alpha);
                    for h_idx in (cur_half / 2)..((new_half + 1) / 2) {
                        let row = h_idx / 10;
                        let col = h_idx % 10;
                        let hx = (x_left + col * 8) as f32;
                        let hy = (base_y - row * row_height) as f32;
                        let is_half = (h_idx * 2 + 1) == new_half;
                        let sprite = if is_half {
                            "hud/heart/half"
                        } else {
                            "hud/heart/full"
                        };
                        batch.sprite_scaled(atlas, sprite, hx, hy, 9.0, 9.0, heart_col);
                    }
                }
            }
        } else if cfg.show_saturation_overlay && inputs.saturation > 0.01 {
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
        if sat <= 0.01 {
            return;
        }
        let full_color = rgba(255, 214, 64, alpha);
        let inner_color = rgba(255, 179, 0, alpha);
        let slots = (sat / 2.0).ceil() as i32;
        for i in 0..slots.min(10) {
            let rem = sat - (i as f32) * 2.0;
            let x = (x_right - i * 8 - 9) as f32;
            let y = y_food as f32;
            if rem >= 1.5 {
                // Full gold outline around drumstick (top, bottom, left, right)
                batch.fill(atlas, x + 1.0, y, 7.0, 1.0, full_color);
                batch.fill(atlas, x + 1.0, y + 8.0, 7.0, 1.0, inner_color);
                batch.fill(atlas, x, y + 1.0, 1.0, 7.0, full_color);
                batch.fill(atlas, x + 8.0, y + 1.0, 1.0, 7.0, inner_color);
            } else {
                // Half gold outline (right half of the drumstick, matching vanilla right-to-left drain)
                batch.fill(atlas, x + 5.0, y, 3.0, 1.0, full_color);
                batch.fill(atlas, x + 5.0, y + 8.0, 3.0, 1.0, inner_color);
                batch.fill(atlas, x + 8.0, y + 1.0, 1.0, 7.0, inner_color);
            }
        }
    }

    /// Tooltip layout measurement for AppleSkin's food + saturation icons.
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

    /// Draws AppleSkin's hunger + gold saturation tooltip component inside an item tooltip.
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

        // Row 1: Hunger drumsticks
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
        let food_lbl = format!("+{nutrition}");
        batch.text(
            atlas,
            &food_lbl,
            x0 + food_icons as f32 * 9.0 + 4.0,
            y0 + 1.0,
            rgba(220, 224, 232, 255),
            true,
        );

        // Row 2: Saturation icons
        let sat_icons = ((saturation / 2.0).ceil() as i32).clamp(1, 10);
        let y1 = y0 + 10.0;
        for i in 0..sat_icons {
            let ix = x0 + i as f32 * 9.0;
            let rem = saturation - (i as f32) * 2.0;
            let sprite = if rem < 1.5 {
                "hud/food_half"
            } else {
                "hud/food_full"
            };
            batch.sprite_scaled(atlas, sprite, ix, y1, 9.0, 9.0, rgba(255, 214, 64, 255));
        }
        let sat_lbl = format!("+{saturation:.1}");
        batch.text(
            atlas,
            &sat_lbl,
            x0 + sat_icons as f32 * 9.0 + 4.0,
            y1 + 1.0,
            rgba(255, 214, 64, 255),
            true,
        );
    }

    // =========================================================================
    // 2. Marlow's Crystal Optimizer
    // =========================================================================

    fn crystal_opt_can_break(&self) -> bool {
        let cfg = &self.settings.extras.volt.crystal_optimizer;
        if !cfg.enabled || !cfg.instant_break {
            return false;
        }
        if cfg.respect_weakness && self.potions.effects.get("weakness").is_some() {
            let dmg = self
                .player
                .as_ref()
                .and_then(|p| p.attribute("minecraft:attack_damage").ok())
                .unwrap_or(1.0);
            if dmg <= 0.5 {
                return false;
            }
        }
        true
    }

    /// Called right after `S::Attack(play::Attack { id })` on a remote object.
    /// Immediately hides/removes an attacked End Crystal client-side so follow-up
    /// placements don't block on server `RemoveEntities` latency.
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
        if cfg.fast_place && self.use_delay > 0 {
            let holding_crystal_or_obi = |s: &ItemStack| {
                matches!(
                    s.item.as_str(),
                    "minecraft:end_crystal" | "minecraft:obsidian"
                )
            };
            let active = self.stack_in(self.selected).is_some_and(holding_crystal_or_obi)
                || self.offhand.as_ref().is_some_and(holding_crystal_or_obi);
            if active {
                self.use_delay = 0;
            }
        }
    }

    // =========================================================================
    // 3. Shield Status
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
        // Vanilla axe disable cooldown is 100 ticks (5.0s).
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
            .filter(|s| s.item == "minecraft:shield");
        let off_shield = self
            .offhand
            .as_ref()
            .filter(|s| s.item == "minecraft:shield");
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

        // Held-shield subtle highlight on offhand/selected slot when Shield display is enabled
        if cfg.display_mode != ShieldDisplayMode::HudOnly {
            let slot_tint = rgba(rgb[0], rgb[1], rgb[2], if self.shield_cooldown > 0 { 110 } else { 70 });
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

        // Crisp Shield Status pill beside the hotbar
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
                    .is_some_and(|s| s.item == "minecraft:shield")
            });
            let lbl = if active { "BLK" } else { "RDY" };
            batch.text(atlas, lbl, bx + 15.0, by + 6.0, border, true);
        }
    }

    // =========================================================================
    // 4. Uku's Armor and Potion HUD
    // =========================================================================

    pub(super) fn draw_volt_extras_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        self.draw_shield_status_hud(batch, wi, hi);
        self.draw_uku_armor_hud(batch, wi, hi);
        self.draw_uku_potion_hud(batch, wi, hi);
    }

    fn draw_uku_armor_hud(&self, batch: &mut HudBatch, wi: i32, hi: i32) {
        let cfg = &self.settings.extras.volt.uku_hud;
        if !cfg.enabled || !cfg.armor_enabled {
            return;
        }

        // Collect armor slots in authentic top-to-bottom order: Helmet, Chestplate, Leggings, Boots (+ optional Offhand)
        let mut entries: Vec<Option<&ItemStack>> = Vec::with_capacity(5);
        for slot in [3usize, 2, 1, 0] {
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
        let panel_w = if horizontal { n * slot_step + 4.0 } else { if matches!(cfg.durability_display, UkuDurability::Numeric | UkuDurability::Percentage) { 52.0 } else { 24.0 } };
        let panel_h = if horizontal { 24.0 } else { n * 20.0 + 4.0 };

        let (px, py) = match (cfg.armor_anchor, cfg.armor_side) {
            (UkuAnchor::Hotbar, UkuSide::Left) => ((wi / 2 - 91 - 36) as f32 - panel_w, (hi - 23) as f32),
            (UkuAnchor::Hotbar, UkuSide::Right) => ((wi / 2 + 91 + 8) as f32, (hi - 23) as f32),
            (UkuAnchor::Bottom, UkuSide::Left) => (6.0, (hi - 6) as f32 - panel_h),
            (UkuAnchor::Bottom, UkuSide::Right) => ((wi as f32) - panel_w - 6.0, (hi - 6) as f32 - panel_h),
            (UkuAnchor::Top, UkuSide::Left) => (6.0, 28.0),
            (UkuAnchor::Top, UkuSide::Right) => ((wi as f32) - panel_w - 6.0, 28.0),
        };

        // Background box matching selected UkuStyle
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

            let max_d = self
                .items()
                .get(&stack.item)
                .map_or(0, |info| info.max_damage);
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
            let beneficial = horizon_client_core::potions::effect(eff.id)
                .is_some_and(|d| d.category == horizon_client_core::potions::Category::Beneficial);
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

            let sprite = horizon_client_core::potions::sprite(eff.id);
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
                n => {
                    let _ = n;
                    "+"
                }
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
}
