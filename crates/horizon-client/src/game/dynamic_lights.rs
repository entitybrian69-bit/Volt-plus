//! Horizon Extras' dynamic lights (`horizon_client::dynamic_light`): the light sources near the
//! camera this frame. Nothing is gathered while the extra is off.
use super::{Game, shield};
use glam::DVec3;
use horizon_client::dynamic_light::{self, BURNING, Luminance, Source};

impl Game<'_> {
    /// Fills `dynamic_lights` for the frame: the player's and mobs' held items and fire, and
    /// glowing dropped items, the nearest [`dynamic_light::MAX`] of them.
    pub(super) fn gather_dynamic_lights(&mut self, camera: DVec3, alpha: f64) {
        let mut out = std::mem::take(&mut self.dynamic_lights);
        out.clear();
        let vcfg = &self.settings.extras.volt.dynamic_lights;
        if !self.settings.extras.dynamic_lights && !vcfg.enabled {
            self.dynamic_lights = out;
            return;
        }
        let allow_held = !vcfg.enabled || vcfg.held_items;
        let allow_dropped = !vcfg.enabled || vcfg.dropped_items;
        let allow_burn = !vcfg.enabled || vcfg.burning_entities;
        let min_lvl = if vcfg.enabled { vcfg.min_level } else { 1 };
        if self.luminance.is_none() {
            self.luminance = Some(Luminance::new(self.res.facts(), self.pack.assets.registry));
        }
        let lum = self.luminance.as_ref().expect("built above");
        let held = |item: Option<&str>| item.map_or(0, |i| lum.of(i));
        let mut sources = Vec::new();
        if let Some(p) = &self.player
            && !p.spectator
        {
            let feet = self
                .previous_position
                .lerp(p.position, self.clock.camera_fraction());
            let mut level = if allow_held {
                held(self.stack_in(self.selected).map(|s| &*s.item))
                    .max(held(self.stack_in(shield::OFFHAND_SLOT).map(|s| &*s.item)))
            } else {
                0
            };
            if allow_burn && p.fire_ticks > 0 {
                level = level.max(BURNING);
            }
            if level >= min_lvl {
                // About where the hands are.
                sources.push(Source {
                    at: DVec3::new(feet.x, feet.y + 1.2, feet.z),
                    level,
                });
            }
        }
        for m in self.mobs.iter().filter(|m| !m.removed) {
            let mut level = if allow_held {
                held(m.extra.main_hand.as_deref()).max(held(m.extra.off_hand.as_deref()))
            } else {
                0
            };
            if allow_burn && m.fire_ticks > 0 {
                level = level.max(BURNING);
            }
            if level < min_lvl {
                continue;
            }
            let at = m.previous.lerp(m.position, alpha);
            sources.push(Source {
                at: DVec3::new(at.x, at.y + f64::from(m.height()) * 0.6, at.z),
                level,
            });
        }
        if allow_dropped {
            for item in &self.dropped {
                let mut level = lum.of(&item.stack.item);
                if allow_burn && item.fire_ticks > 0 {
                    level = level.max(BURNING);
                }
                if level < min_lvl {
                    continue;
                }
                let at = item.previous.lerp(item.position, alpha);
                sources.push(Source {
                    at: DVec3::new(at.x, at.y + 0.25, at.z),
                    level,
                });
            }
        }
        dynamic_light::select(&sources, camera, &mut out);
        self.dynamic_lights = out;
    }
}
