//! Horizon's optional extras (minimap, lighting, inventory tools): their
//! settings, kept in `horizon-extras.txt` next to `options.txt` so that file
//! stays exactly vanilla's, and the logic behind them. Every extra is off by
//! default; see `docs/extras.md`. Horizon's own switches outside the Extras page
//! (No AI, Agent Control, Discord Rich Presence) are kept here too.
use std::{collections::VecDeque, fs, path::Path};

pub const FILE: &str = "horizon-extras.txt";
/// The name the options screen uses while the sort key waits for a key.
pub const SORT_KEY: &str = "horizon.sort";

/// The minimap's sizes in GUI pixels, and its zooms in blocks per pixel.
pub const MINIMAP_SIZES: [(u8, &str); 3] = [(64, "Small"), (96, "Medium"), (128, "Large")];
pub const MINIMAP_ZOOMS: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

/// A screen corner for an overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Corner::TopLeft => "top_left",
            Corner::TopRight => "top_right",
            Corner::BottomLeft => "bottom_left",
            Corner::BottomRight => "bottom_right",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Corner::TopLeft => "Top left",
            Corner::TopRight => "Top right",
            Corner::BottomLeft => "Bottom left",
            Corner::BottomRight => "Bottom right",
        }
    }
    pub fn parse(s: &str, default: Corner) -> Corner {
        Self::ALL
            .into_iter()
            .find(|c| c.name() == s)
            .unwrap_or(default)
    }
    pub fn top(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::TopRight)
    }
    pub fn left(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::BottomLeft)
    }
    pub fn next(self) -> Self {
        match self {
            Corner::TopLeft => Corner::TopRight,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomRight => Corner::BottomLeft,
            Corner::BottomLeft => Corner::TopLeft,
        }
    }
}

pub use crate::volt_extras_cfg::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Extras {
    pub fullbright: bool,
    /// Held, dropped and burning light sources light their surroundings (`dynamic_light`).
    pub dynamic_lights: bool,
    pub zoom: bool,
    /// A vanilla key name (`key.keyboard.z`).
    pub zoom_key: String,
    /// The zoom the key starts at.
    pub zoom_level: u8,
    /// Ease through vanilla's FOV easing (as the spyglass does).
    pub zoom_smooth: bool,
    /// Divide mouse turning by the zoom.
    pub zoom_sensitivity: bool,
    /// The FPS counter and keystrokes extras are official mods now
    /// (`moved_extras`): their old settings stay here until the game has
    /// installed the mod for a player who had one on.
    pub fps: bool,
    pub fps_corner: Corner,
    /// Also show the minimum and 1% low.
    pub fps_lows: bool,
    pub keystrokes: bool,
    pub keystrokes_corner: Corner,
    /// Clicks per second under the mouse buttons.
    pub keystrokes_cps: bool,
    /// The official mods installed for the extras they replace, by mod id
    /// (`movedToMods`): each is done once.
    pub moved: Vec<String>,
    pub minimap: bool,
    pub minimap_corner: Corner,
    /// Side in GUI pixels (`MINIMAP_SIZES`).
    pub minimap_size: u8,
    pub minimap_round: bool,
    /// Blocks per pixel (`MINIMAP_ZOOMS`).
    pub minimap_zoom: f32,
    /// Turn with the player (forward up) instead of north up.
    pub minimap_rotate: bool,
    /// Retry lost multiplayer connections (`reconnect`).
    pub auto_reconnect: bool,
    /// The player's own name tag over its model in third person.
    pub own_name_tag: bool,
    /// Mouse Tweaks-style drags in the inventory and storage containers.
    pub mouse_tweaks: bool,
    /// With them, the wheel over a slot moves its items one at a time.
    pub mouse_tweaks_wheel: bool,
    /// Sort buttons on the inventory and chest-like containers.
    pub sort: bool,
    /// A vanilla key name: sorts the part under the cursor.
    pub sort_key: String,
    /// No AI: every AI feature hidden and off (`horizon_core::no_ai`; the launcher writes it too).
    pub no_ai: bool,
    /// Agents may control the game the player starts: it opens its control socket for
    /// horizon-ctl and MCP clients (off by default; an agent's own launch always opens it).
    pub agent_control: bool,
    /// "Playing Volt+" on the player's Discord profile, with the mode, the dimension and the
    /// time played (`discord_presence`; on by default).
    pub discord_presence: bool,
    /// Native built-in Volt+ Extras mods housed in the Control Hub's Volt+ Extras tab.
    pub volt: VoltExtras,
    /// Native built-in Volt Shaders configuration (`Page::VoltShaders`).
    pub volt_shaders: horizon_shader_api::volt_shaders::VoltShadersConfig,
}
impl Default for Extras {
    fn default() -> Self {
        Self {
            fullbright: false,
            dynamic_lights: false,
            zoom: false,
            zoom_key: "key.keyboard.z".into(),
            zoom_level: 4,
            zoom_smooth: true,
            zoom_sensitivity: true,
            fps: false,
            fps_corner: Corner::TopLeft,
            fps_lows: false,
            keystrokes: false,
            keystrokes_corner: Corner::TopLeft,
            keystrokes_cps: true,
            moved: Vec::new(),
            minimap: false,
            minimap_corner: Corner::TopRight,
            minimap_size: 96,
            minimap_round: false,
            minimap_zoom: 1.0,
            minimap_rotate: false,
            auto_reconnect: false,
            own_name_tag: false,
            mouse_tweaks: false,
            mouse_tweaks_wheel: true,
            sort: false,
            sort_key: "key.keyboard.unknown".into(),
            no_ai: false,
            agent_control: false,
            discord_presence: true,
            volt: VoltExtras::default(),
            volt_shaders: horizon_shader_api::volt_shaders::VoltShadersConfig::default(),
        }
    }
}
fn bool_text(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}
impl Extras {
    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(dir.join(FILE))
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }
    pub fn parse(text: &str) -> Self {
        let mut e = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            let flag = |d: bool| match value {
                "true" => true,
                "false" => false,
                _ => d,
            };
            match key.trim() {
                "fullbright" => e.fullbright = flag(e.fullbright),
                "dynamicLights" => e.dynamic_lights = flag(e.dynamic_lights),
                "zoom" => e.zoom = flag(e.zoom),
                "zoomKey" if value.starts_with("key.") => e.zoom_key = value.to_owned(),
                "zoomLevel" => {
                    if let Ok(v) = value.parse::<u8>() {
                        e.zoom_level = v.clamp(2, 10);
                    }
                }
                "zoomSmooth" => e.zoom_smooth = flag(e.zoom_smooth),
                "zoomSensitivity" => e.zoom_sensitivity = flag(e.zoom_sensitivity),
                "fps" => e.fps = flag(e.fps),
                "fpsCorner" => e.fps_corner = Corner::parse(value, e.fps_corner),
                "fpsLows" => e.fps_lows = flag(e.fps_lows),
                "keystrokes" => e.keystrokes = flag(e.keystrokes),
                "keystrokesCorner" => {
                    e.keystrokes_corner = Corner::parse(value, e.keystrokes_corner)
                }
                "keystrokesCps" => e.keystrokes_cps = flag(e.keystrokes_cps),
                "movedToMods" => {
                    e.moved = value
                        .split(',')
                        .map(str::trim)
                        .filter(|id| !id.is_empty())
                        .map(str::to_owned)
                        .collect()
                }
                "minimap" => e.minimap = flag(e.minimap),
                "minimapCorner" => e.minimap_corner = Corner::parse(value, e.minimap_corner),
                "minimapSize" => {
                    if let Some(&(v, _)) = MINIMAP_SIZES.iter().find(|s| s.0.to_string() == value) {
                        e.minimap_size = v;
                    }
                }
                "minimapShape" => e.minimap_round = value == "round",
                "minimapZoom" => {
                    if let Some(&z) = MINIMAP_ZOOMS.iter().find(|z| z.to_string() == value) {
                        e.minimap_zoom = z;
                    }
                }
                "minimapRotate" => e.minimap_rotate = flag(e.minimap_rotate),
                "autoReconnect" => e.auto_reconnect = flag(e.auto_reconnect),
                "ownNameTag" => e.own_name_tag = flag(e.own_name_tag),
                "mouseTweaks" => e.mouse_tweaks = flag(e.mouse_tweaks),
                "mouseTweaksWheel" => e.mouse_tweaks_wheel = flag(e.mouse_tweaks_wheel),
                "sort" => e.sort = flag(e.sort),
                "sortKey" if value.starts_with("key.") => e.sort_key = value.to_owned(),
                horizon_core::no_ai::KEY => e.no_ai = flag(e.no_ai),
                "agentControl" => e.agent_control = flag(e.agent_control),
                "discordPresence" => e.discord_presence = flag(e.discord_presence),
                "voltShadersEnabled" => e.volt_shaders.enabled = flag(e.volt_shaders.enabled),
                "voltShadersProfile" => {
                    if let Some(p) =
                        horizon_shader_api::volt_shaders::VoltShaderProfile::parse(value)
                    {
                        e.volt_shaders.profile = p;
                    }
                }
                "voltShaderWavePlants" => {
                    e.volt_shaders.waving_plants = flag(e.volt_shaders.waving_plants)
                }
                "voltShaderWaveLeaves" => {
                    e.volt_shaders.waving_leaves = flag(e.volt_shaders.waving_leaves)
                }
                "voltShaderWaveWater" => {
                    e.volt_shaders.waving_water = flag(e.volt_shaders.waving_water)
                }
                "voltShaderWindSpeed" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.wind_speed = v.clamp(25, 200);
                    }
                }
                "voltShaderWindStrength" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.wind_strength = v.clamp(25, 200);
                    }
                }
                "voltShaderStormWind" => {
                    e.volt_shaders.storm_wind = flag(e.volt_shaders.storm_wind)
                }
                "voltShaderSunShading" => {
                    e.volt_shaders.sun_shading = flag(e.volt_shaders.sun_shading)
                }
                "voltShaderShadowRichness" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.shadow_richness = v.clamp(0, 100);
                    }
                }
                "voltShaderFoliageGlow" => {
                    e.volt_shaders.foliage_subsurface = flag(e.volt_shaders.foliage_subsurface)
                }
                "voltShaderTorchStyle" => {
                    if let Some(v) =
                        horizon_shader_api::volt_shaders::TorchLightStyle::parse(value)
                    {
                        e.volt_shaders.torch_style = v;
                    }
                }
                "voltShaderTorchIntensity" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.torch_intensity = v.clamp(50, 200);
                    }
                }
                "voltShaderEmissiveGlow" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.emissive_glow = v.clamp(0, 200);
                    }
                }
                "voltShaderRainWetness" => {
                    e.volt_shaders.rain_wetness = flag(e.volt_shaders.rain_wetness)
                }
                "voltShaderWaterRipples" => {
                    e.volt_shaders.water_ripples = flag(e.volt_shaders.water_ripples)
                }
                "voltShaderWaterReflection" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.water_reflection = v.clamp(0, 150);
                    }
                }
                "voltShaderWaterGlint" => {
                    e.volt_shaders.water_glint = flag(e.volt_shaders.water_glint)
                }
                "voltShaderClearWater" => {
                    e.volt_shaders.clear_water = flag(e.volt_shaders.clear_water)
                }
                "voltShaderUnderwaterRefract" => {
                    e.volt_shaders.underwater_refraction =
                        flag(e.volt_shaders.underwater_refraction)
                }
                "voltShaderCustomSky" => {
                    e.volt_shaders.custom_sky = flag(e.volt_shaders.custom_sky)
                }
                "voltShaderSunsetGlow" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.sunset_glow = v.clamp(0, 200);
                    }
                }
                "voltShaderSunHalo" => {
                    e.volt_shaders.sun_halo = flag(e.volt_shaders.sun_halo)
                }
                "voltShaderAurora" => {
                    if let Some(v) = horizon_shader_api::volt_shaders::AuroraMode::parse(value) {
                        e.volt_shaders.aurora = v;
                    }
                }
                "voltShaderDepthHaze" => {
                    if let Some(v) = horizon_shader_api::volt_shaders::DepthHazeMode::parse(value)
                    {
                        e.volt_shaders.depth_haze = v;
                    }
                }
                "voltShaderTonemap" => {
                    if let Some(v) = horizon_shader_api::volt_shaders::TonemapMode::parse(value) {
                        e.volt_shaders.tonemap = v;
                    }
                }
                "voltShaderBloom" => {
                    if let Some(v) = horizon_shader_api::volt_shaders::BloomMode::parse(value) {
                        e.volt_shaders.bloom = v;
                    }
                }
                "voltShaderSunRays" => e.volt_shaders.sun_rays = flag(e.volt_shaders.sun_rays),
                "voltShaderVibrance" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.vibrance = v.clamp(50, 180);
                    }
                }
                "voltShaderSaturation" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.saturation = v.clamp(50, 180);
                    }
                }
                "voltShaderContrast" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.contrast = v.clamp(70, 150);
                    }
                }
                "voltShaderExposure" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.exposure = v.clamp(70, 150);
                    }
                }
                "voltShaderWarmth" => {
                    if let Ok(v) = value.parse::<i16>() {
                        e.volt_shaders.warmth = v.clamp(-50, 50);
                    }
                }
                "voltShaderVignette" => {
                    if let Some(v) = horizon_shader_api::volt_shaders::VignetteMode::parse(value) {
                        e.volt_shaders.vignette = v;
                    }
                }
                "voltShaderStyle" => {
                    if let Some(v) =
                        horizon_shader_api::volt_shaders::ShaderVisualStyle::parse(value)
                    {
                        e.volt_shaders.style = v;
                    }
                }
                "voltShaderCloudShadows" => {
                    e.volt_shaders.cloud_shadows = flag(e.volt_shaders.cloud_shadows)
                }
                "voltShaderGlowingOres" => {
                    e.volt_shaders.glowing_ores = flag(e.volt_shaders.glowing_ores)
                }
                "voltShaderOreGlow" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.ore_glow = v.clamp(25, 200);
                    }
                }
                "voltShaderAutoPbr" => e.volt_shaders.auto_pbr = flag(e.volt_shaders.auto_pbr),
                "voltShaderWaterFoam" => {
                    e.volt_shaders.water_foam = flag(e.volt_shaders.water_foam)
                }
                "voltShaderCloudsMode" => {
                    if let Some(v) = horizon_shader_api::volt_shaders::CloudMode::parse(value) {
                        e.volt_shaders.clouds_mode = v;
                    }
                }
                "voltShaderCloudCoverage" => {
                    if let Ok(v) = value.parse::<u16>() {
                        e.volt_shaders.cloud_coverage = v.clamp(30, 160);
                    }
                }
                "voltShaderNightSkyPlus" => {
                    e.volt_shaders.night_sky_plus = flag(e.volt_shaders.night_sky_plus)
                }
                "voltShaderRainbow" => {
                    e.volt_shaders.post_rain_rainbow = flag(e.volt_shaders.post_rain_rainbow)
                }
                "voltShaderEyeComfort" => {
                    e.volt_shaders.eye_comfort = flag(e.volt_shaders.eye_comfort)
                }
                k if k.starts_with("volt.") => e.volt.parse_key(k, value),
                _ => {}
            }
        }
        e
    }
    pub fn serialize(&self) -> String {
        let mut s = format!(
            "fullbright:{}\ndynamicLights:{}\nzoom:{}\nzoomKey:{}\nzoomLevel:{}\nzoomSmooth:{}\nzoomSensitivity:{}\nfps:{}\nfpsCorner:{}\nfpsLows:{}\nkeystrokes:{}\nkeystrokesCorner:{}\nkeystrokesCps:{}\nmovedToMods:{}\nminimap:{}\nminimapCorner:{}\nminimapSize:{}\nminimapShape:{}\nminimapZoom:{}\nminimapRotate:{}\nautoReconnect:{}\nownNameTag:{}\nmouseTweaks:{}\nmouseTweaksWheel:{}\nsort:{}\nsortKey:{}\n{}:{}\nagentControl:{}\ndiscordPresence:{}\n",
            bool_text(self.fullbright),
            bool_text(self.dynamic_lights),
            bool_text(self.zoom),
            self.zoom_key,
            self.zoom_level,
            bool_text(self.zoom_smooth),
            bool_text(self.zoom_sensitivity),
            bool_text(self.fps),
            self.fps_corner.name(),
            bool_text(self.fps_lows),
            bool_text(self.keystrokes),
            self.keystrokes_corner.name(),
            bool_text(self.keystrokes_cps),
            self.moved.join(","),
            bool_text(self.minimap),
            self.minimap_corner.name(),
            self.minimap_size,
            if self.minimap_round {
                "round"
            } else {
                "square"
            },
            self.minimap_zoom,
            bool_text(self.minimap_rotate),
            bool_text(self.auto_reconnect),
            bool_text(self.own_name_tag),
            bool_text(self.mouse_tweaks),
            bool_text(self.mouse_tweaks_wheel),
            bool_text(self.sort),
            self.sort_key,
            horizon_core::no_ai::KEY,
            bool_text(self.no_ai),
            bool_text(self.agent_control),
            bool_text(self.discord_presence),
        );
        self.volt.serialize_into(&mut s);
        let mut push = |k: &str, v: &str| {
            s.push_str(k);
            s.push(':');
            s.push_str(v);
            s.push('\n');
        };
        push("voltShadersEnabled", bool_text(self.volt_shaders.enabled));
        push("voltShadersProfile", self.volt_shaders.profile.as_str());
        push(
            "voltShaderWavePlants",
            bool_text(self.volt_shaders.waving_plants),
        );
        push(
            "voltShaderWaveLeaves",
            bool_text(self.volt_shaders.waving_leaves),
        );
        push(
            "voltShaderWaveWater",
            bool_text(self.volt_shaders.waving_water),
        );
        push(
            "voltShaderWindSpeed",
            &self.volt_shaders.wind_speed.to_string(),
        );
        push(
            "voltShaderWindStrength",
            &self.volt_shaders.wind_strength.to_string(),
        );
        push(
            "voltShaderStormWind",
            bool_text(self.volt_shaders.storm_wind),
        );
        push(
            "voltShaderSunShading",
            bool_text(self.volt_shaders.sun_shading),
        );
        push(
            "voltShaderShadowRichness",
            &self.volt_shaders.shadow_richness.to_string(),
        );
        push(
            "voltShaderFoliageGlow",
            bool_text(self.volt_shaders.foliage_subsurface),
        );
        push(
            "voltShaderTorchStyle",
            self.volt_shaders.torch_style.as_str(),
        );
        push(
            "voltShaderTorchIntensity",
            &self.volt_shaders.torch_intensity.to_string(),
        );
        push(
            "voltShaderEmissiveGlow",
            &self.volt_shaders.emissive_glow.to_string(),
        );
        push(
            "voltShaderRainWetness",
            bool_text(self.volt_shaders.rain_wetness),
        );
        push(
            "voltShaderWaterRipples",
            bool_text(self.volt_shaders.water_ripples),
        );
        push(
            "voltShaderWaterReflection",
            &self.volt_shaders.water_reflection.to_string(),
        );
        push(
            "voltShaderWaterGlint",
            bool_text(self.volt_shaders.water_glint),
        );
        push(
            "voltShaderClearWater",
            bool_text(self.volt_shaders.clear_water),
        );
        push(
            "voltShaderUnderwaterRefract",
            bool_text(self.volt_shaders.underwater_refraction),
        );
        push(
            "voltShaderCustomSky",
            bool_text(self.volt_shaders.custom_sky),
        );
        push(
            "voltShaderSunsetGlow",
            &self.volt_shaders.sunset_glow.to_string(),
        );
        push("voltShaderSunHalo", bool_text(self.volt_shaders.sun_halo));
        push("voltShaderAurora", self.volt_shaders.aurora.as_str());
        push("voltShaderDepthHaze", self.volt_shaders.depth_haze.as_str());
        push("voltShaderTonemap", self.volt_shaders.tonemap.as_str());
        push("voltShaderBloom", self.volt_shaders.bloom.as_str());
        push("voltShaderSunRays", bool_text(self.volt_shaders.sun_rays));
        push(
            "voltShaderVibrance",
            &self.volt_shaders.vibrance.to_string(),
        );
        push(
            "voltShaderSaturation",
            &self.volt_shaders.saturation.to_string(),
        );
        push(
            "voltShaderContrast",
            &self.volt_shaders.contrast.to_string(),
        );
        push(
            "voltShaderExposure",
            &self.volt_shaders.exposure.to_string(),
        );
        push("voltShaderWarmth", &self.volt_shaders.warmth.to_string());
        push("voltShaderVignette", self.volt_shaders.vignette.as_str());
        push("voltShaderStyle", self.volt_shaders.style.as_str());
        push(
            "voltShaderCloudShadows",
            bool_text(self.volt_shaders.cloud_shadows),
        );
        push(
            "voltShaderGlowingOres",
            bool_text(self.volt_shaders.glowing_ores),
        );
        push("voltShaderOreGlow", &self.volt_shaders.ore_glow.to_string());
        push("voltShaderAutoPbr", bool_text(self.volt_shaders.auto_pbr));
        push(
            "voltShaderWaterFoam",
            bool_text(self.volt_shaders.water_foam),
        );
        push(
            "voltShaderCloudsMode",
            self.volt_shaders.clouds_mode.as_str(),
        );
        push(
            "voltShaderCloudCoverage",
            &self.volt_shaders.cloud_coverage.to_string(),
        );
        push(
            "voltShaderNightSkyPlus",
            bool_text(self.volt_shaders.night_sky_plus),
        );
        push(
            "voltShaderRainbow",
            bool_text(self.volt_shaders.post_rain_rainbow),
        );
        push(
            "voltShaderEyeComfort",
            bool_text(self.volt_shaders.eye_comfort),
        );
        s
    }
    /// Binds the extra key `name` (`SORT_KEY`); false for
    /// another name (a vanilla mapping).
    pub fn set_key(&mut self, name: &str, value: String) -> bool {
        match name {
            SORT_KEY => self.sort_key = value,
            _ => return false,
        }
        true
    }
    /// Written only once something differs from the defaults, so a vanilla
    /// setup gains no file.
    pub fn save(&self, dir: &Path) {
        let path = dir.join(FILE);
        if *self == Self::default() && !path.exists() {
            return;
        }
        let _ = fs::write(path, self.serialize());
    }
}

/// The field-of-view modifier target `Camera.tickFov` eases toward: vanilla's
/// (`getFieldOfViewModifier`) divided by a mod's zoom (`render.set-zoom`). A
/// zoom of 1 leaves it as is.
pub fn fov_target(vanilla: f32, zoom: f32) -> f32 {
    if zoom == 1.0 { vanilla } else { vanilla / zoom }
}
/// Frame times over the last few seconds: frames per second, the minimum
/// and the 1% low, recomputed twice a second.
pub struct FpsCounter {
    /// (end time in seconds, frame milliseconds), oldest first.
    frames: VecDeque<(f64, f32)>,
    shown: FpsReading,
    computed_at: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FpsReading {
    pub fps: u32,
    pub min: u32,
    pub low: u32,
}
/// Seconds the minimum and 1% low look back.
const LOW_WINDOW: f64 = 5.0;
impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            frames: VecDeque::new(),
            shown: FpsReading::default(),
            computed_at: f64::NEG_INFINITY,
        }
    }
}
impl FpsCounter {
    /// A frame that ended at `now` (seconds) and took `ms`.
    pub fn record(&mut self, now: f64, ms: f32) {
        self.frames.push_back((now, ms));
        while self
            .frames
            .front()
            .is_some_and(|&(t, _)| now - t > LOW_WINDOW)
        {
            self.frames.pop_front();
        }
        if now - self.computed_at >= 0.5 {
            self.computed_at = now;
            self.shown = self.compute(now);
        }
    }
    fn compute(&self, now: f64) -> FpsReading {
        let fps = self.frames.iter().filter(|&&(t, _)| now - t < 1.0).count() as u32;
        let mut times: Vec<f32> = self.frames.iter().map(|f| f.1).collect();
        if times.is_empty() {
            return FpsReading::default();
        }
        times.sort_by(|a, b| b.total_cmp(a));
        let to_fps = |ms: f32| (1000.0 / ms.max(0.001)).round() as u32;
        // The 1% low: the frame rate of the 99th percentile frame time.
        let low = times[(times.len() - 1) / 100];
        FpsReading {
            fps,
            min: to_fps(times[0]),
            low: to_fps(low),
        }
    }
    pub fn reading(&self) -> FpsReading {
        self.shown
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Clicks of one button in the last second.
#[derive(Default)]
pub struct Cps {
    clicks: VecDeque<f64>,
}
impl Cps {
    pub fn click(&mut self, now: f64) {
        self.clicks.push_back(now);
        self.forget(now);
    }
    fn forget(&mut self, now: f64) {
        while self.clicks.front().is_some_and(|&t| now - t >= 1.0) {
            self.clicks.pop_front();
        }
    }
    pub fn per_second(&mut self, now: f64) -> usize {
        self.forget(now);
        self.clicks.len()
    }
    pub fn count(&self, now: f64) -> usize {
        self.clicks.iter().filter(|&&t| now - t < 1.0).count()
    }
}

/// The settings the Extras page shows, one per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Fullbright,
    DynamicLights,
    OwnNameTag,
    AutoReconnect,
    MouseTweaks,
    MouseTweaksWheel,
    Sort,
    SortKey,
    /// Not on the Extras page: Horizon Settings shows No AI, its AI Agent page Agent Control.
    NoAi,
    AgentControl,
    /// On the Horizon Settings page.
    DiscordPresence,
    // Native Volt Shaders (Page::VoltShaders)
    VoltShaderEnable,
    VoltShaderProfile,
    VoltShaderReset,
    VoltShaderWavePlants,
    VoltShaderWaveLeaves,
    VoltShaderWaveWater,
    VoltShaderStormWind,
    VoltShaderSunShading,
    VoltShaderFoliageGlow,
    VoltShaderTorchStyle,
    VoltShaderRainWetness,
    VoltShaderWaterRipples,
    VoltShaderWaterGlint,
    VoltShaderClearWater,
    VoltShaderUnderwaterRefract,
    VoltShaderCustomSky,
    VoltShaderSunHalo,
    VoltShaderAurora,
    VoltShaderDepthHaze,
    VoltShaderTonemap,
    VoltShaderBloom,
    VoltShaderSunRays,
    VoltShaderVignette,
    VoltShaderStyle,
    VoltShaderEyeComfort,
    VoltShaderCloudsMode,
    VoltShaderCloudCoverage,
    VoltShaderCloudShadows,
    VoltShaderGlowingOres,
    VoltShaderOreGlow,
    VoltShaderAutoPbr,
    VoltShaderWaterFoam,
    VoltShaderNightSkyPlus,
    VoltShaderRainbow,
}
/// The Extras page: headers and items.
pub fn rows() -> Vec<Result<Item, &'static str>> {
    use Item::*;
    vec![
        Err("Lighting"),
        Ok(Fullbright),
        Ok(DynamicLights),
        Err("Third person"),
        Ok(OwnNameTag),
        Err("Inventory"),
        Ok(MouseTweaks),
        Ok(MouseTweaksWheel),
        Ok(Sort),
        Ok(SortKey),
        Err("Multiplayer"),
        Ok(AutoReconnect),
    ]
}
impl Item {
    pub fn caption(self) -> &'static str {
        match self {
            Item::Fullbright => "Fullbright",
            Item::DynamicLights => "Dynamic lights",
            Item::OwnNameTag => "Own name tag",
            Item::AutoReconnect => "Auto-reconnect",
            Item::MouseTweaks => "Mouse Tweaks controls",
            Item::MouseTweaksWheel => "Scroll wheel moves items",
            Item::Sort => "Sort buttons",
            Item::SortKey => "Sort key",
            Item::NoAi => "No AI",
            Item::AgentControl => "Agent Control",
            Item::DiscordPresence => "Discord Rich Presence",
            Item::VoltShaderEnable => "Enable Shaders",
            Item::VoltShaderProfile => "Shader Profile",
            Item::VoltShaderReset => "Reset Profile",
            Item::VoltShaderWavePlants => "Waving Grass & Plants",
            Item::VoltShaderWaveLeaves => "Waving Leaves & Vines",
            Item::VoltShaderWaveWater => "Waving Water",
            Item::VoltShaderStormWind => "Storm Wind Boost",
            Item::VoltShaderSunShading => "Sun Angle Shading",
            Item::VoltShaderFoliageGlow => "Foliage Subsurface",
            Item::VoltShaderTorchStyle => "Torchlight Color",
            Item::VoltShaderRainWetness => "Rain Wet Sheen",
            Item::VoltShaderWaterRipples => "Water Ripples",
            Item::VoltShaderWaterGlint => "Water Sun Glint",
            Item::VoltShaderClearWater => "Clear Water",
            Item::VoltShaderUnderwaterRefract => "Underwater Wave",
            Item::VoltShaderCustomSky => "Atmospheric Sky",
            Item::VoltShaderSunHalo => "Sun & Moon Halo",
            Item::VoltShaderAurora => "Night Aurora",
            Item::VoltShaderDepthHaze => "Distance Haze",
            Item::VoltShaderTonemap => "Tonemapper",
            Item::VoltShaderBloom => "Bloom",
            Item::VoltShaderSunRays => "Sun Rays",
            Item::VoltShaderVignette => "Vignette",
            Item::VoltShaderStyle => "Visual Style",
            Item::VoltShaderEyeComfort => "Eye-Comfort Softness",
            Item::VoltShaderCloudsMode => "3D Volumetric Clouds",
            Item::VoltShaderCloudCoverage => "Cloud Coverage",
            Item::VoltShaderCloudShadows => "Ground Cloud Shadows",
            Item::VoltShaderGlowingOres => "Glowing Ore Veins",
            Item::VoltShaderOreGlow => "Ore Glow Brightness",
            Item::VoltShaderAutoPbr => "Auto-PBR Reflections",
            Item::VoltShaderWaterFoam => "Shore Foam & Caustics",
            Item::VoltShaderNightSkyPlus => "Milky Way & Meteors",
            Item::VoltShaderRainbow => "Post-Rain Rainbow",
        }
    }
    /// The switch this setting belongs to: it does nothing while that is off.
    pub fn parent(self) -> Option<Item> {
        use Item::*;
        Some(match self {
            MouseTweaksWheel => MouseTweaks,
            SortKey => Sort,
            Fullbright | DynamicLights | OwnNameTag | MouseTweaks | Sort | AutoReconnect | NoAi
            | AgentControl | DiscordPresence | VoltShaderEnable | VoltShaderProfile
            | VoltShaderReset | VoltShaderWavePlants | VoltShaderWaveLeaves
            | VoltShaderWaveWater | VoltShaderStormWind | VoltShaderSunShading
            | VoltShaderFoliageGlow | VoltShaderTorchStyle | VoltShaderRainWetness
            | VoltShaderWaterRipples | VoltShaderWaterGlint | VoltShaderClearWater
            | VoltShaderUnderwaterRefract | VoltShaderCustomSky | VoltShaderSunHalo
            | VoltShaderAurora | VoltShaderDepthHaze | VoltShaderTonemap | VoltShaderBloom
            | VoltShaderSunRays | VoltShaderVignette | VoltShaderStyle
            | VoltShaderEyeComfort | VoltShaderCloudsMode | VoltShaderCloudCoverage
            | VoltShaderCloudShadows | VoltShaderGlowingOres | VoltShaderOreGlow
            | VoltShaderAutoPbr | VoltShaderWaterFoam | VoltShaderNightSkyPlus
            | VoltShaderRainbow => {
                return None;
            }
        })
    }
    /// What the row says under its caption.
    pub fn description(self) -> &'static str {
        match self {
            Item::Fullbright => "Show blocks and entities at full light, even in dark areas.",
            Item::DynamicLights => {
                "Torches and other light sources held, dropped or on fire light the blocks around them."
            }
            Item::OwnNameTag => {
                "Shows your name above your head in third person, as other players see it. Hidden when the server hides your team's names or you're invisible."
            }
            Item::AutoReconnect => {
                "Rejoins a server after a lost connection or a restart, never after a kick or ban."
            }
            Item::MouseTweaks => {
                "Right-drag places one per slot, left-drag gathers the held item, Shift + left-drag moves every slot."
            }
            Item::MouseTweaksWheel => {
                "Down sends one item of the slot to the other side, up brings one back."
            }
            Item::Sort => {
                "Sorts the inventory, chests, barrels, ender chests and shulker boxes in creative tab order, merging stacks."
            }
            Item::SortKey => {
                "Sorts the part under the cursor. Click, then press a key; Escape unbinds it."
            }
            Item::NoAi => {
                "Hides and turns off every AI feature: agent control, horizon-ctl and the MCP servers. The launcher's switch too."
            }
            Item::DiscordPresence => {
                "Shows Playing Volt+ on your Discord profile while Discord runs: the launcher or the world, the game mode, single player, a server or a LAN world, the dimension, how many players are in it and the time played. Never a world's name or a server's address."
            }
            Item::AgentControl => {
                "Your own AI agent can play this game through horizon-ctl or an MCP client, on this computer only."
            }
            Item::VoltShaderEnable => {
                "Turns on Volt Shaders: horizon's built-in mobile-friendly WGSL sky, foliage wave, lighting, water reflection and color pipeline."
            }
            Item::VoltShaderProfile => {
                "Switches all shader features and post-processing quality at once (Potato, Low, Medium, High, Ultra)."
            }
            Item::VoltShaderReset => {
                "Restores the current profile's recommended defaults."
            }
            Item::VoltShaderWavePlants => {
                "Sways grass, flowers, ferns, crops and tall seagrass gently in the wind."
            }
            Item::VoltShaderWaveLeaves => {
                "Adds soft breeze motion to tree leaves and hanging vines."
            }
            Item::VoltShaderWaveWater => {
                "Applies subtle rolling wave ripples to water surface geometry."
            }
            Item::VoltShaderStormWind => {
                "Boosts wind speed and sway strength while raining or thundering."
            }
            Item::VoltShaderSunShading => {
                "Adds warm directional sunlight highlights and cool ambient shadow contrast."
            }
            Item::VoltShaderFoliageGlow => {
                "Softly brightens sunlit grass and leaves with translucent subsurface scattering."
            }
            Item::VoltShaderTorchStyle => {
                "Chooses warm cozy lantern glow or crisp bright torchlight."
            }
            Item::VoltShaderRainWetness => {
                "Adds glossy wet specular sheen on outdoor surfaces during rain."
            }
            Item::VoltShaderWaterRipples => {
                "Renders animated procedural normal ripples across water surfaces."
            }
            Item::VoltShaderWaterGlint => {
                "Adds sparkling sun and moon reflections on rippling water."
            }
            Item::VoltShaderClearWater => {
                "Makes shallow water clearer so underwater blocks stay crisp and readable."
            }
            Item::VoltShaderUnderwaterRefract => {
                "Adds gentle caustic wave shimmer while swimming underwater."
            }
            Item::VoltShaderCustomSky => {
                "Replaces the flat sky with atmospheric gradients, golden sunsets and crisp starfields."
            }
            Item::VoltShaderSunHalo => {
                "Adds a soft glowing solar corona and moon halo in the sky."
            }
            Item::VoltShaderAurora => {
                "Draws northern aurora ribbons across cold or night skies."
            }
            Item::VoltShaderDepthHaze => {
                "Blends distant terrain smoothly into atmospheric sky haze."
            }
            Item::VoltShaderTonemap => {
                "Selects ACES Filmic, Soft Filmic or AgX color tone-mapping curve."
            }
            Item::VoltShaderBloom => {
                "Adds soft HDR bloom around bright torches, emissives, sunbeams and glowing ores."
            }
            Item::VoltShaderSunRays => {
                "Projects gentle crepuscular god-rays when looking toward the sun."
            }
            Item::VoltShaderVignette => {
                "Adds subtle cinematic edge darkening around the viewport."
            }
            Item::VoltShaderStyle => {
                "Switches overall BSL Soft Cinema, Complementary Reimagined, Volt Balanced, or Pastel Dream atmosphere."
            }
            Item::VoltShaderEyeComfort => {
                "BSL soft highlight shoulder & velvet shadow lift for comfortable zero-glare sessions."
            }
            Item::VoltShaderCloudsMode => {
                "Selects multi-layer BSL Puffy 3D clouds, Complementary Reimagined voxel clouds, or soft Cirrus."
            }
            Item::VoltShaderCloudCoverage => {
                "Adjusts 3D sky cloud density and sunlit cloud shadow coverage."
            }
            Item::VoltShaderCloudShadows => {
                "Projects drifting sunlight & cool cloud shadow patches across outdoor landscapes."
            }
            Item::VoltShaderGlowingOres => {
                "Complementary-style selective ore vein emission: diamond, emerald, gold, redstone & amethyst veins glow!"
            }
            Item::VoltShaderOreGlow => {
                "Controls HDR emission brightness of glowing ore veins inside caves."
            }
            Item::VoltShaderAutoPbr => {
                "Integrated Auto-PBR specular glints & Fresnel reflections on metals, gems, ice, polished stone & glass."
            }
            Item::VoltShaderWaterFoam => {
                "Adds shimmering underwater caustics and bubbly white shoreline/wave-crest foam."
            }
            Item::VoltShaderNightSkyPlus => {
                "Renders a subtle Milky Way nebula band and occasional night shooting stars."
            }
            Item::VoltShaderRainbow => {
                "Displays a natural 42-degree optical rainbow arc during sun-showers and clearing rain."
            }
        }
    }
    /// The frame-time cost the main switch of an extra shows.
    pub fn cost(self) -> Option<crate::graphics_style::Cost> {
        use crate::graphics_style::Cost;
        match self {
            Item::Fullbright => Some(Cost::None),
            Item::DynamicLights => Some(Cost::Low),
            Item::OwnNameTag | Item::AutoReconnect | Item::MouseTweaks | Item::Sort => {
                Some(Cost::None)
            }
            _ => None,
        }
    }
    /// On/off items and their value.
    pub fn switch(self, e: &Extras) -> Option<bool> {
        Some(match self {
            Item::Fullbright => e.fullbright,
            Item::DynamicLights => e.dynamic_lights,
            Item::OwnNameTag => e.own_name_tag,
            Item::AutoReconnect => e.auto_reconnect,
            Item::MouseTweaks => e.mouse_tweaks,
            Item::MouseTweaksWheel => e.mouse_tweaks_wheel,
            Item::Sort => e.sort,
            Item::NoAi => e.no_ai,
            Item::AgentControl => e.agent_control,
            Item::DiscordPresence => e.discord_presence,
            Item::VoltShaderEnable => e.volt_shaders.enabled,
            Item::VoltShaderWavePlants => e.volt_shaders.waving_plants,
            Item::VoltShaderWaveLeaves => e.volt_shaders.waving_leaves,
            Item::VoltShaderWaveWater => e.volt_shaders.waving_water,
            Item::VoltShaderStormWind => e.volt_shaders.storm_wind,
            Item::VoltShaderSunShading => e.volt_shaders.sun_shading,
            Item::VoltShaderFoliageGlow => e.volt_shaders.foliage_subsurface,
            Item::VoltShaderRainWetness => e.volt_shaders.rain_wetness,
            Item::VoltShaderWaterRipples => e.volt_shaders.water_ripples,
            Item::VoltShaderWaterGlint => e.volt_shaders.water_glint,
            Item::VoltShaderClearWater => e.volt_shaders.clear_water,
            Item::VoltShaderUnderwaterRefract => e.volt_shaders.underwater_refraction,
            Item::VoltShaderCustomSky => e.volt_shaders.custom_sky,
            Item::VoltShaderSunHalo => e.volt_shaders.sun_halo,
            Item::VoltShaderSunRays => e.volt_shaders.sun_rays,
            Item::VoltShaderEyeComfort => e.volt_shaders.eye_comfort,
            Item::VoltShaderCloudShadows => e.volt_shaders.cloud_shadows,
            Item::VoltShaderGlowingOres => e.volt_shaders.glowing_ores,
            Item::VoltShaderAutoPbr => e.volt_shaders.auto_pbr,
            Item::VoltShaderWaterFoam => e.volt_shaders.water_foam,
            Item::VoltShaderNightSkyPlus => e.volt_shaders.night_sky_plus,
            Item::VoltShaderRainbow => e.volt_shaders.post_rain_rainbow,
            _ => return None,
        })
    }
    /// The value of the other items, as their button shows it.
    pub fn value(self, e: &Extras) -> String {
        match self {
            Item::SortKey => crate::keybinds::display_name(&e.sort_key).plain(),
            Item::VoltShaderEnable => if e.volt_shaders.enabled { "ON" } else { "OFF" }.into(),
            Item::VoltShaderProfile => e.volt_shaders.profile.label().into(),
            Item::VoltShaderReset => "Restore".into(),
            Item::VoltShaderTorchStyle => e.volt_shaders.torch_style.label().into(),
            Item::VoltShaderAurora => e.volt_shaders.aurora.label().into(),
            Item::VoltShaderDepthHaze => e.volt_shaders.depth_haze.label().into(),
            Item::VoltShaderTonemap => e.volt_shaders.tonemap.label().into(),
            Item::VoltShaderBloom => e.volt_shaders.bloom.label().into(),
            Item::VoltShaderVignette => e.volt_shaders.vignette.label().into(),
            Item::VoltShaderStyle => e.volt_shaders.style.label().into(),
            Item::VoltShaderCloudsMode => e.volt_shaders.clouds_mode.label().into(),
            Item::VoltShaderCloudCoverage => format!("{}%", e.volt_shaders.cloud_coverage),
            Item::VoltShaderOreGlow => format!("{}%", e.volt_shaders.ore_glow),
            other => match other.switch(e) {
                Some(true) => "On".into(),
                _ => "Off".into(),
            },
        }
    }
    /// A click (`back` for the right button).
    pub fn cycle(self, e: &mut Extras, back: bool) {
        fn step<T: Copy + PartialEq>(opts: &[T], cur: T, back: bool) -> T {
            let idx = opts.iter().position(|&v| v == cur).unwrap_or(0);
            if back {
                opts[(idx + opts.len() - 1) % opts.len()]
            } else {
                opts[(idx + 1) % opts.len()]
            }
        }
        fn step_u16(cur: u16, min: u16, max: u16, delta: u16, back: bool) -> u16 {
            if back {
                if cur <= min { max } else { cur.saturating_sub(delta).max(min) }
            } else if cur >= max {
                min
            } else {
                (cur + delta).min(max)
            }
        }
        match self {
            Item::Fullbright => e.fullbright = !e.fullbright,
            Item::DynamicLights => e.dynamic_lights = !e.dynamic_lights,
            Item::OwnNameTag => e.own_name_tag = !e.own_name_tag,
            Item::AutoReconnect => e.auto_reconnect = !e.auto_reconnect,
            Item::MouseTweaks => e.mouse_tweaks = !e.mouse_tweaks,
            Item::MouseTweaksWheel => e.mouse_tweaks_wheel = !e.mouse_tweaks_wheel,
            Item::Sort => e.sort = !e.sort,
            Item::NoAi => e.no_ai = !e.no_ai,
            Item::AgentControl => e.agent_control = !e.agent_control,
            Item::DiscordPresence => e.discord_presence = !e.discord_presence,
            Item::SortKey => {}
            Item::VoltShaderEnable => e.volt_shaders.enabled = !e.volt_shaders.enabled,
            Item::VoltShaderProfile => {
                use horizon_shader_api::volt_shaders::VoltShaderProfile;
                let next = match e.volt_shaders.profile {
                    VoltShaderProfile::Potato => VoltShaderProfile::Low,
                    VoltShaderProfile::Low => VoltShaderProfile::Medium,
                    VoltShaderProfile::Medium => VoltShaderProfile::High,
                    VoltShaderProfile::High => VoltShaderProfile::Ultra,
                    VoltShaderProfile::Ultra => VoltShaderProfile::Potato,
                    VoltShaderProfile::Custom => {
                        if back {
                            VoltShaderProfile::Ultra
                        } else {
                            VoltShaderProfile::Potato
                        }
                    }
                };
                let next = if back && e.volt_shaders.profile != VoltShaderProfile::Custom {
                    match e.volt_shaders.profile {
                        VoltShaderProfile::Potato => VoltShaderProfile::Ultra,
                        VoltShaderProfile::Low => VoltShaderProfile::Potato,
                        VoltShaderProfile::Medium => VoltShaderProfile::Low,
                        VoltShaderProfile::High => VoltShaderProfile::Medium,
                        VoltShaderProfile::Ultra => VoltShaderProfile::High,
                        VoltShaderProfile::Custom => VoltShaderProfile::High,
                    }
                } else {
                    next
                };
                e.volt_shaders.apply_profile(next);
            }
            Item::VoltShaderReset => e.volt_shaders.reset_current_profile(),
            Item::VoltShaderWavePlants => {
                e.volt_shaders.waving_plants = !e.volt_shaders.waving_plants;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderWaveLeaves => {
                e.volt_shaders.waving_leaves = !e.volt_shaders.waving_leaves;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderWaveWater => {
                e.volt_shaders.waving_water = !e.volt_shaders.waving_water;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderStormWind => {
                e.volt_shaders.storm_wind = !e.volt_shaders.storm_wind;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderSunShading => {
                e.volt_shaders.sun_shading = !e.volt_shaders.sun_shading;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderFoliageGlow => {
                e.volt_shaders.foliage_subsurface = !e.volt_shaders.foliage_subsurface;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderTorchStyle => {
                e.volt_shaders.torch_style = step(
                    &horizon_shader_api::volt_shaders::TorchLightStyle::ALL,
                    e.volt_shaders.torch_style,
                    back,
                );
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderRainWetness => {
                e.volt_shaders.rain_wetness = !e.volt_shaders.rain_wetness;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderWaterRipples => {
                e.volt_shaders.water_ripples = !e.volt_shaders.water_ripples;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderWaterGlint => {
                e.volt_shaders.water_glint = !e.volt_shaders.water_glint;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderClearWater => {
                e.volt_shaders.clear_water = !e.volt_shaders.clear_water;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderUnderwaterRefract => {
                e.volt_shaders.underwater_refraction = !e.volt_shaders.underwater_refraction;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderCustomSky => {
                e.volt_shaders.custom_sky = !e.volt_shaders.custom_sky;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderSunHalo => {
                e.volt_shaders.sun_halo = !e.volt_shaders.sun_halo;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderAurora => {
                e.volt_shaders.aurora = step(
                    &horizon_shader_api::volt_shaders::AuroraMode::ALL,
                    e.volt_shaders.aurora,
                    back,
                );
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderDepthHaze => {
                e.volt_shaders.depth_haze = step(
                    &horizon_shader_api::volt_shaders::DepthHazeMode::ALL,
                    e.volt_shaders.depth_haze,
                    back,
                );
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderTonemap => {
                e.volt_shaders.tonemap = step(
                    &horizon_shader_api::volt_shaders::TonemapMode::ALL,
                    e.volt_shaders.tonemap,
                    back,
                );
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderBloom => {
                e.volt_shaders.bloom = step(
                    &horizon_shader_api::volt_shaders::BloomMode::ALL,
                    e.volt_shaders.bloom,
                    back,
                );
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderSunRays => {
                e.volt_shaders.sun_rays = !e.volt_shaders.sun_rays;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderVignette => {
                e.volt_shaders.vignette = step(
                    &horizon_shader_api::volt_shaders::VignetteMode::ALL,
                    e.volt_shaders.vignette,
                    back,
                );
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderStyle => {
                e.volt_shaders.style = e.volt_shaders.style.cycle(back);
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderEyeComfort => {
                e.volt_shaders.eye_comfort = !e.volt_shaders.eye_comfort;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderCloudsMode => {
                e.volt_shaders.clouds_mode = e.volt_shaders.clouds_mode.cycle(back);
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderCloudCoverage => {
                e.volt_shaders.cloud_coverage =
                    step_u16(e.volt_shaders.cloud_coverage, 40, 160, 15, back);
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderCloudShadows => {
                e.volt_shaders.cloud_shadows = !e.volt_shaders.cloud_shadows;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderGlowingOres => {
                e.volt_shaders.glowing_ores = !e.volt_shaders.glowing_ores;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderOreGlow => {
                e.volt_shaders.ore_glow = step_u16(e.volt_shaders.ore_glow, 40, 200, 20, back);
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderAutoPbr => {
                e.volt_shaders.auto_pbr = !e.volt_shaders.auto_pbr;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderWaterFoam => {
                e.volt_shaders.water_foam = !e.volt_shaders.water_foam;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderNightSkyPlus => {
                e.volt_shaders.night_sky_plus = !e.volt_shaders.night_sky_plus;
                e.volt_shaders.mark_custom();
            }
            Item::VoltShaderRainbow => {
                e.volt_shaders.post_rain_rainbow = !e.volt_shaders.post_rain_rainbow;
                e.volt_shaders.mark_custom();
            }
        }
    }
    /// The key name the options screen waits on for a key item.
    pub fn key_name(self) -> Option<&'static str> {
        match self {
            Item::SortKey => Some(SORT_KEY),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_off_by_default_and_round_trips() {
        let d = Extras::default();
        assert!(!d.fullbright && !d.dynamic_lights);
        assert!(!d.zoom && !d.fps && !d.keystrokes && !d.minimap && !d.auto_reconnect);
        assert!(!d.mouse_tweaks && !d.sort && !d.own_name_tag);
        assert!(d.discord_presence);
        assert_eq!(Extras::parse(""), d);
        assert_eq!(Extras::parse(&d.serialize()), d);
        let mut e = d.clone();
        Item::Fullbright.cycle(&mut e, false);
        assert_eq!(Item::Fullbright.switch(&e), Some(true));
        Item::DynamicLights.cycle(&mut e, false);
        assert_eq!(Item::DynamicLights.switch(&e), Some(true));
        assert_eq!(Extras::parse(&e.serialize()), e);
        e.zoom = true;
        e.zoom_key = "key.keyboard.c".into();
        e.zoom_level = 6;
        e.fps_corner = Corner::BottomRight;
        e.keystrokes = true;
        e.keystrokes_cps = false;
        e.moved = vec!["fps_overlay".into(), "keystrokes".into()];
        e.minimap = true;
        e.minimap_size = 128;
        e.minimap_round = true;
        e.minimap_zoom = 0.5;
        e.minimap_rotate = true;
        e.minimap_corner = Corner::BottomLeft;
        e.auto_reconnect = true;
        Item::OwnNameTag.cycle(&mut e, false);
        assert_eq!(Item::OwnNameTag.switch(&e), Some(true));
        e.mouse_tweaks = true;
        e.mouse_tweaks_wheel = false;
        e.sort = true;
        e.sort_key = "key.keyboard.r".into();
        e.discord_presence = false;
        assert_eq!(Extras::parse(&e.serialize()), e);
    }

    #[test]
    fn bad_lines_keep_defaults() {
        let e = Extras::parse("zoom:maybe\nzoomLevel:99\nfpsCorner:middle\nzoomKey:z\nnoise");
        assert!(!e.zoom);
        assert_eq!(e.zoom_level, 10);
        assert_eq!(e.fps_corner, Corner::TopLeft);
        assert_eq!(e.zoom_key, "key.keyboard.z");
    }

    #[test]
    fn a_mods_zoom_eases_like_the_spyglass() {
        // Camera.tickFov halves the distance to the target each tick.
        let mut camera = crate::camera::Camera::default();
        camera.tick(None, 1.0);
        for _ in 0..30 {
            camera.tick(None, 1.0);
        }
        let target = fov_target(1.0, 4.0);
        camera.tick(None, target);
        let first = camera.calculate_fov(1.0, 70, 1.0, None, Default::default(), false);
        assert!((first - 70.0 * 0.625).abs() < 1e-3, "{first}");
        for _ in 0..30 {
            camera.tick(None, target);
        }
        let settled = camera.calculate_fov(1.0, 70, 1.0, None, Default::default(), false);
        assert!((settled - 70.0 / 4.0).abs() < 1e-3, "{settled}");
        // Deeper than the 0.1 floor stops there, as vanilla clamps.
        for _ in 0..30 {
            camera.tick(None, fov_target(1.0, 20.0));
        }
        let floor = camera.calculate_fov(1.0, 70, 1.0, None, Default::default(), false);
        assert!((floor - 7.0).abs() < 1e-3, "{floor}");
    }

    #[test]
    fn fps_counter_reports_rate_minimum_and_low() {
        let mut c = FpsCounter::default();
        let mut t = 0.0;
        // Two seconds at 100 fps with one 50 ms hitch.
        for i in 0..200 {
            let ms = if i == 150 { 50.0 } else { 10.0 };
            t += f64::from(ms) / 1000.0;
            c.record(t, ms);
        }
        c.computed_at = f64::NEG_INFINITY;
        c.record(t + 0.01, 10.0);
        let r = c.reading();
        assert!((95..=101).contains(&r.fps), "{r:?}");
        assert_eq!(r.min, 20);
        assert_eq!(r.low, 100, "one frame in 200 is under the 1% mark");
        // Hitches beyond 1% of frames pull the 1% low down.
        let mut c = FpsCounter::default();
        let mut t = 0.0;
        for i in 0..100 {
            let ms = if i % 25 == 0 { 40.0 } else { 10.0 };
            t += f64::from(ms) / 1000.0;
            c.record(t, ms);
        }
        c.computed_at = f64::NEG_INFINITY;
        c.record(t, 10.0);
        assert_eq!(c.reading().low, 25);
    }

    #[test]
    fn old_frames_leave_the_window() {
        let mut c = FpsCounter::default();
        c.record(0.0, 100.0);
        for i in 1..=700 {
            c.record(f64::from(i) / 100.0, 10.0);
        }
        assert_eq!(c.reading().min, 100, "the 100 ms frame is 7 s old");
    }

    #[test]
    fn cps_counts_the_last_second() {
        let mut cps = Cps::default();
        for i in 0..10 {
            cps.click(f64::from(i) * 0.1);
        }
        assert_eq!(cps.per_second(0.95), 10);
        assert_eq!(cps.per_second(1.25), 7);
        assert_eq!(cps.per_second(3.0), 0);
    }

    #[test]
    fn items_cycle_through_their_values() {
        let mut e = Extras::default();
        Item::Sort.cycle(&mut e, true);
        assert_eq!(Item::Sort.value(&e), "On");
        // The minimap's old keys still parse, for the move to its mod.
        assert_eq!(
            Extras::parse("minimapSize:7\nminimapZoom:3").minimap_zoom,
            1.0
        );
        for item in rows().into_iter().flatten() {
            assert!(!item.caption().is_empty() && !item.description().is_empty());
        }
    }
}
