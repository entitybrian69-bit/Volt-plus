//! Built-in Volt+ Shader Suite (`Volt + settings`):
//! - 5 Quality/Performance Presets: Potato, Low, Medium, High, Ultra
//! - 8 Style Presets: Vanilla+, BSL, Complementary, Sildur's Vibrant, Chocapic13, MakeUp Fast,
//!   Nostalgia, Solas Aurora
//! - Moving vegetation (foliage + leaves) and waving water active across ALL presets including Potato
//! - Extreme 1 GB RAM optimization: zero offscreen post targets, zero shadow passes, zero opaque
//!   copies, and zero LabPBR texture loads on Potato/Low; scaled shadow & quarter-res bloom on
//!   Medium/High/Ultra.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::package::Pack;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltQualityPreset {
    Potato,
    #[default]
    Low,
    Medium,
    High,
    Ultra,
}

impl VoltQualityPreset {
    pub const ALL: [Self; 5] = [
        Self::Potato,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Ultra,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Potato => "Potato",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Potato => "Ultra-fast for 1 GB RAM phones: 0 shadow passes, 0 offscreen post buffers, keeps waving plants & water.",
            Self::Low => "Fast mobile shader: sky-fresnel water, atmospheric sky & fog, waving plants & water, zero shadow overhead.",
            Self::Medium => "Balanced mobile realism: 512px real-time shadows, procedural clouds, caustics, single-pass filmic glow.",
            Self::High => "High fidelity: 1024px soft PCF shadows, planar water mirror, quarter-res bloom, god rays & volumetric clouds.",
            Self::Ultra => "Maximum visual beauty: 9-tap soft shadows, colored stained-glass shadows, deep water refraction, rich aurora & bloom.",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltStylePreset {
    VanillaPlus,
    #[default]
    Bsl,
    Complementary,
    SildursVibrant,
    ChocapicMobile,
    MakeUpFast,
    Nostalgia,
    SolasAurora,
}

impl VoltStylePreset {
    pub const ALL: [Self; 8] = [
        Self::VanillaPlus,
        Self::Bsl,
        Self::Complementary,
        Self::SildursVibrant,
        Self::ChocapicMobile,
        Self::MakeUpFast,
        Self::Nostalgia,
        Self::SolasAurora,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::VanillaPlus => "Vanilla +",
            Self::Bsl => "BSL",
            Self::Complementary => "Complementary",
            Self::SildursVibrant => "Sildur's Vibrant",
            Self::ChocapicMobile => "Chocapic13",
            Self::MakeUpFast => "MakeUp Fast",
            Self::Nostalgia => "Nostalgia",
            Self::SolasAurora => "Solas Aurora",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::VanillaPlus => "Faithful enhanced vanilla lighting, clean skies, warm torchlight & crisp water.",
            Self::Bsl => "Creamy golden sunlight, soft pastel atmosphere, serene turquoise water & dreamy bloom.",
            Self::Complementary => "Crisp balanced daylight, lush foliage, vibrant blue water & cozy amber caves.",
            Self::SildursVibrant => "Rich sunbeams, warm sunset horizons, deep indigo nights & punchy color vibrance.",
            Self::ChocapicMobile => "High-contrast sunlight & cool sky ambient shadows tuned for crisp visibility.",
            Self::MakeUpFast => "Ultra-clean, high-FPS stylized daylight with vivid water glint for mobile GPUs.",
            Self::Nostalgia => "Warm early-shader golden haze, classic specular water & rich starry nights.",
            Self::SolasAurora => "Luminous twilight sky with flowing night Aurora Borealis & glowing emissives.",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltCloudMode {
    #[default]
    Off,
    Fast,
    Volumetric,
}

impl VoltCloudMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Fast, Self::Volumetric];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Vanilla / Off",
            Self::Fast => "Fast Procedural",
            Self::Volumetric => "Volumetric-Style",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltWaterReflections {
    Off,
    #[default]
    SkyFresnel,
    PlanarMirror,
}

impl VoltWaterReflections {
    pub const ALL: [Self; 3] = [Self::Off, Self::SkyFresnel, Self::PlanarMirror];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::SkyFresnel => "Sky Fresnel (Fast)",
            Self::PlanarMirror => "Planar Mirror",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltShadowQuality {
    #[default]
    Off,
    Fast512,
    Medium1024,
    High1536,
    Ultra2048,
}

impl VoltShadowQuality {
    pub const ALL: [Self; 5] = [
        Self::Off,
        Self::Fast512,
        Self::Medium1024,
        Self::High1536,
        Self::Ultra2048,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fast512 => "Fast (512px)",
            Self::Medium1024 => "Medium (1024px)",
            Self::High1536 => "High (1536px)",
            Self::Ultra2048 => "Ultra (2048px)",
        }
    }
    pub fn resolution(self, low_ram_guard: bool) -> u32 {
        match self {
            Self::Off => 0,
            Self::Fast512 => 512,
            Self::Medium1024 => 1024,
            Self::High1536 => if low_ram_guard { 1024 } else { 1536 },
            Self::Ultra2048 => if low_ram_guard { 1536 } else { 2048 },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltShadowSoftness {
    #[default]
    Sharp,
    Soft4,
    Ultra9,
}

impl VoltShadowSoftness {
    pub const ALL: [Self; 3] = [Self::Sharp, Self::Soft4, Self::Ultra9];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sharp => "Sharp (1-Tap)",
            Self::Soft4 => "Soft (4-Tap PCF)",
            Self::Ultra9 => "Ultra (9-Tap PCF)",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltToneMapping {
    Natural,
    AcesFilmic,
    #[default]
    BslSoft,
    Complementary,
}

impl VoltToneMapping {
    pub const ALL: [Self; 4] = [
        Self::Natural,
        Self::AcesFilmic,
        Self::BslSoft,
        Self::Complementary,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Natural => "Natural",
            Self::AcesFilmic => "ACES Filmic",
            Self::BslSoft => "BSL Soft",
            Self::Complementary => "Complementary",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltBloomMode {
    #[default]
    Off,
    Fast,
    SoftQuarter,
    Cinematic,
}

impl VoltBloomMode {
    pub const ALL: [Self; 4] = [Self::Off, Self::Fast, Self::SoftQuarter, Self::Cinematic];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off (Zero VRAM)",
            Self::Fast => "Fast (1-Pass)",
            Self::SoftQuarter => "Soft (1/4 Res)",
            Self::Cinematic => "Cinematic (1/4 Res)",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltVignetteMode {
    Off,
    #[default]
    Subtle,
    Strong,
}

impl VoltVignetteMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Subtle, Self::Strong];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Subtle => "Subtle",
            Self::Strong => "Strong",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltAuroraMode {
    #[default]
    Off,
    Subtle,
    Vivid,
}

impl VoltAuroraMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Subtle, Self::Vivid];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Subtle => "Subtle",
            Self::Vivid => "Vivid",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltStarMode {
    Off,
    #[default]
    Standard,
    Rich,
}

impl VoltStarMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Standard, Self::Rich];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Standard => "Standard",
            Self::Rich => "Rich Milky Way",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltGodRaysMode {
    #[default]
    Off,
    Fast,
    High,
}

impl VoltGodRaysMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Fast, Self::High];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fast => "Fast",
            Self::High => "High",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoltWaterCaustics {
    #[default]
    Off,
    Fast,
    Realistic,
}

impl VoltWaterCaustics {
    pub const ALL: [Self; 3] = [Self::Off, Self::Fast, Self::Realistic];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fast => "Fast",
            Self::Realistic => "Realistic",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoltShaderConfig {
    pub enabled: bool,
    pub quality: VoltQualityPreset,
    pub style: VoltStylePreset,
    pub low_ram_guard: bool,

    // Atmosphere & Sky
    pub sun_glow: f32,
    pub clouds: VoltCloudMode,
    pub aurora: VoltAuroraMode,
    pub stars: VoltStarMode,
    pub fog_density: f32,
    pub god_rays: VoltGodRaysMode,

    // Lighting & Shadows
    pub shadows: VoltShadowQuality,
    pub shadow_softness: VoltShadowSoftness,
    pub translucent_shadows: bool,
    pub shadow_distance: f32,
    pub sunlight_warmth: f32,
    pub ambient_brightness: f32,
    pub torch_warmth: f32,
    pub colored_blocklights: bool,

    // Water & Foliage
    pub waving_vegetation: bool,
    pub wind_speed: f32,
    pub wind_strength: f32,
    pub waving_water: bool,
    pub water_wave_intensity: f32,
    pub water_reflections: VoltWaterReflections,
    pub water_clarity: f32,
    pub water_caustics: VoltWaterCaustics,

    // Color, Bloom & Post
    pub tone_mapping: VoltToneMapping,
    pub bloom: VoltBloomMode,
    pub bloom_intensity: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub vibrance: f32,
    pub exposure: f32,
    pub rain_wetness: bool,
    pub vignette: VoltVignetteMode,
    pub labpbr: bool,
}

impl Default for VoltShaderConfig {
    fn default() -> Self {
        let mut cfg = Self {
            enabled: false,
            quality: VoltQualityPreset::Low,
            style: VoltStylePreset::Bsl,
            low_ram_guard: cfg!(target_os = "android"),
            sun_glow: 0.90,
            clouds: VoltCloudMode::Fast,
            aurora: VoltAuroraMode::Off,
            stars: VoltStarMode::Standard,
            fog_density: 0.70,
            god_rays: VoltGodRaysMode::Off,
            shadows: VoltShadowQuality::Off,
            shadow_softness: VoltShadowSoftness::Sharp,
            translucent_shadows: false,
            shadow_distance: 80.0,
            sunlight_warmth: 0.85,
            ambient_brightness: 1.00,
            torch_warmth: 0.85,
            colored_blocklights: true,
            waving_vegetation: true,
            wind_speed: 1.00,
            wind_strength: 1.00,
            waving_water: true,
            water_wave_intensity: 1.00,
            water_reflections: VoltWaterReflections::SkyFresnel,
            water_clarity: 0.72,
            water_caustics: VoltWaterCaustics::Fast,
            tone_mapping: VoltToneMapping::BslSoft,
            bloom: VoltBloomMode::Off,
            bloom_intensity: 0.65,
            saturation: 1.18,
            contrast: 1.04,
            vibrance: 1.15,
            exposure: 1.08,
            rain_wetness: false,
            vignette: VoltVignetteMode::Subtle,
            labpbr: false,
        };
        cfg.apply_style_and_quality(cfg.quality, cfg.style);
        cfg.enabled = false;
        cfg
    }
}

impl VoltShaderConfig {
    pub fn apply_style_and_quality(
        &mut self,
        quality: VoltQualityPreset,
        style: VoltStylePreset,
    ) {
        self.quality = quality;
        self.style = style;

        // Guarantee moving vegetation and waving water in ALL presets including Potato.
        self.waving_vegetation = true;
        self.waving_water = true;

        // Quality tier defaults (memory & GPU pass budgets).
        match quality {
            VoltQualityPreset::Potato => {
                self.shadows = VoltShadowQuality::Off;
                self.shadow_softness = VoltShadowSoftness::Sharp;
                self.translucent_shadows = false;
                self.shadow_distance = 64.0;
                self.water_reflections = VoltWaterReflections::SkyFresnel;
                self.water_caustics = VoltWaterCaustics::Off;
                self.clouds = VoltCloudMode::Off;
                self.stars = VoltStarMode::Standard;
                self.aurora = VoltAuroraMode::Off;
                self.god_rays = VoltGodRaysMode::Off;
                self.bloom = VoltBloomMode::Off;
                self.rain_wetness = false;
                self.colored_blocklights = false;
                self.labpbr = false;
            }
            VoltQualityPreset::Low => {
                self.shadows = VoltShadowQuality::Off;
                self.shadow_softness = VoltShadowSoftness::Sharp;
                self.translucent_shadows = false;
                self.shadow_distance = 72.0;
                self.water_reflections = VoltWaterReflections::SkyFresnel;
                self.water_caustics = VoltWaterCaustics::Fast;
                self.clouds = VoltCloudMode::Fast;
                self.stars = VoltStarMode::Standard;
                self.aurora = VoltAuroraMode::Off;
                self.god_rays = VoltGodRaysMode::Off;
                self.bloom = VoltBloomMode::Off;
                self.rain_wetness = false;
                self.colored_blocklights = true;
                self.labpbr = false;
            }
            VoltQualityPreset::Medium => {
                self.shadows = VoltShadowQuality::Fast512;
                self.shadow_softness = VoltShadowSoftness::Sharp;
                self.translucent_shadows = false;
                self.shadow_distance = 80.0;
                self.water_reflections = VoltWaterReflections::SkyFresnel;
                self.water_caustics = VoltWaterCaustics::Fast;
                self.clouds = VoltCloudMode::Fast;
                self.stars = VoltStarMode::Standard;
                self.aurora = VoltAuroraMode::Off;
                self.god_rays = VoltGodRaysMode::Fast;
                self.bloom = VoltBloomMode::Fast;
                self.rain_wetness = true;
                self.colored_blocklights = true;
                self.labpbr = false;
            }
            VoltQualityPreset::High => {
                self.shadows = VoltShadowQuality::Medium1024;
                self.shadow_softness = VoltShadowSoftness::Soft4;
                self.translucent_shadows = false;
                self.shadow_distance = 112.0;
                self.water_reflections = VoltWaterReflections::SkyFresnel;
                self.water_caustics = VoltWaterCaustics::Fast;
                self.clouds = VoltCloudMode::Volumetric;
                self.stars = VoltStarMode::Rich;
                self.aurora = VoltAuroraMode::Subtle;
                self.god_rays = VoltGodRaysMode::Fast;
                self.bloom = VoltBloomMode::Fast;
                self.rain_wetness = true;
                self.colored_blocklights = true;
                self.labpbr = false;
            }
            VoltQualityPreset::Ultra => {
                self.shadows = if self.low_ram_guard {
                    VoltShadowQuality::High1536
                } else {
                    VoltShadowQuality::Ultra2048
                };
                self.shadow_softness = VoltShadowSoftness::Ultra9;
                self.translucent_shadows = true;
                self.shadow_distance = 144.0;
                self.water_reflections = VoltWaterReflections::PlanarMirror;
                self.water_caustics = VoltWaterCaustics::Realistic;
                self.clouds = VoltCloudMode::Volumetric;
                self.stars = VoltStarMode::Rich;
                self.aurora = VoltAuroraMode::Vivid;
                self.god_rays = VoltGodRaysMode::High;
                self.bloom = VoltBloomMode::Cinematic;
                self.rain_wetness = true;
                self.colored_blocklights = true;
                self.labpbr = false;
            }
        }

        // Style palette & atmosphere tuning.
        match style {
            VoltStylePreset::VanillaPlus => {
                self.sun_glow = 0.45;
                self.fog_density = 0.50;
                self.sunlight_warmth = 0.35;
                self.ambient_brightness = 1.05;
                self.torch_warmth = 0.55;
                self.wind_speed = 0.90;
                self.wind_strength = 0.85;
                self.water_wave_intensity = 0.80;
                self.water_clarity = 0.78;
                self.tone_mapping = VoltToneMapping::Natural;
                self.bloom_intensity = 0.35;
                self.saturation = 1.10;
                self.contrast = 1.02;
                self.vibrance = 1.12;
                self.exposure = 1.04;
                self.vignette = VoltVignetteMode::Off;
            }
            VoltStylePreset::Bsl => {
                self.sun_glow = 1.05;
                self.fog_density = 0.80;
                self.sunlight_warmth = 0.95;
                self.ambient_brightness = 1.04;
                self.torch_warmth = 0.90;
                self.wind_speed = 1.00;
                self.wind_strength = 1.00;
                self.water_wave_intensity = 1.00;
                self.water_clarity = 0.74;
                self.tone_mapping = VoltToneMapping::BslSoft;
                self.bloom_intensity = 0.78;
                self.saturation = 1.14;
                self.contrast = 1.03;
                self.vibrance = 1.16;
                self.exposure = 1.10;
                self.vignette = VoltVignetteMode::Subtle;
            }
            VoltStylePreset::Complementary => {
                self.sun_glow = 0.80;
                self.fog_density = 0.65;
                self.sunlight_warmth = 0.72;
                self.ambient_brightness = 1.02;
                self.torch_warmth = 0.95;
                self.wind_speed = 1.05;
                self.wind_strength = 1.00;
                self.water_wave_intensity = 1.05;
                self.water_clarity = 0.70;
                self.tone_mapping = VoltToneMapping::Complementary;
                self.bloom_intensity = 0.58;
                self.saturation = 1.22;
                self.contrast = 1.06;
                self.vibrance = 1.20;
                self.exposure = 1.08;
                self.vignette = VoltVignetteMode::Subtle;
            }
            VoltStylePreset::SildursVibrant => {
                self.sun_glow = 1.35;
                self.fog_density = 0.75;
                self.sunlight_warmth = 1.25;
                self.ambient_brightness = 0.96;
                self.torch_warmth = 1.15;
                self.wind_speed = 1.15;
                self.wind_strength = 1.15;
                self.water_wave_intensity = 1.15;
                self.water_clarity = 0.66;
                self.tone_mapping = VoltToneMapping::AcesFilmic;
                self.bloom_intensity = 0.90;
                self.saturation = 1.34;
                self.contrast = 1.10;
                self.vibrance = 1.28;
                self.exposure = 1.12;
                self.vignette = VoltVignetteMode::Subtle;
            }
            VoltStylePreset::ChocapicMobile => {
                self.sun_glow = 0.75;
                self.fog_density = 0.60;
                self.sunlight_warmth = 0.65;
                self.ambient_brightness = 0.95;
                self.torch_warmth = 0.80;
                self.wind_speed = 1.00;
                self.wind_strength = 0.95;
                self.water_wave_intensity = 0.95;
                self.water_clarity = 0.75;
                self.tone_mapping = VoltToneMapping::AcesFilmic;
                self.bloom_intensity = 0.45;
                self.saturation = 1.16;
                self.contrast = 1.10;
                self.vibrance = 1.14;
                self.exposure = 1.09;
                self.vignette = VoltVignetteMode::Off;
            }
            VoltStylePreset::MakeUpFast => {
                self.sun_glow = 0.70;
                self.fog_density = 0.52;
                self.sunlight_warmth = 0.68;
                self.ambient_brightness = 1.12;
                self.torch_warmth = 0.78;
                self.wind_speed = 1.00;
                self.wind_strength = 0.95;
                self.water_wave_intensity = 0.90;
                self.water_clarity = 0.82;
                self.tone_mapping = VoltToneMapping::Complementary;
                self.bloom_intensity = 0.42;
                self.saturation = 1.24;
                self.contrast = 1.05;
                self.vibrance = 1.22;
                self.exposure = 1.10;
                self.vignette = VoltVignetteMode::Off;
            }
            VoltStylePreset::Nostalgia => {
                self.sun_glow = 1.15;
                self.fog_density = 0.85;
                self.sunlight_warmth = 1.10;
                self.ambient_brightness = 0.98;
                self.torch_warmth = 1.00;
                self.wind_speed = 0.85;
                self.wind_strength = 0.90;
                self.water_wave_intensity = 0.85;
                self.water_clarity = 0.68;
                self.tone_mapping = VoltToneMapping::BslSoft;
                self.bloom_intensity = 0.88;
                self.saturation = 1.15;
                self.contrast = 1.04;
                self.vibrance = 1.10;
                self.exposure = 1.06;
                self.vignette = VoltVignetteMode::Strong;
            }
            VoltStylePreset::SolasAurora => {
                self.sun_glow = 0.95;
                self.fog_density = 0.75;
                self.sunlight_warmth = 0.75;
                self.ambient_brightness = 1.00;
                self.torch_warmth = 0.95;
                self.wind_speed = 1.00;
                self.wind_strength = 1.00;
                self.water_wave_intensity = 1.05;
                self.water_clarity = 0.72;
                self.tone_mapping = VoltToneMapping::AcesFilmic;
                self.bloom_intensity = 0.82;
                self.saturation = 1.22;
                self.contrast = 1.06;
                self.vibrance = 1.22;
                self.exposure = 1.08;
                self.vignette = VoltVignetteMode::Subtle;
                if quality != VoltQualityPreset::Potato {
                    self.aurora = VoltAuroraMode::Vivid;
                }
            }
        }
    }

    pub fn style_palette(&self) -> ([f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
        // (zenith_tint, horizon_warmth, water_shallow, water_deep)
        match self.style {
            VoltStylePreset::VanillaPlus => (
                [0.95, 0.99, 1.05],
                [1.00, 0.74, 0.45],
                [0.14, 0.66, 0.84],
                [0.05, 0.28, 0.56],
            ),
            VoltStylePreset::Bsl => (
                [0.74, 0.88, 1.14],
                [1.12, 0.78, 0.48],
                [0.14, 0.76, 0.82],
                [0.04, 0.34, 0.54],
            ),
            VoltStylePreset::Complementary => (
                [0.66, 0.84, 1.16],
                [1.08, 0.72, 0.40],
                [0.10, 0.68, 0.88],
                [0.03, 0.28, 0.58],
            ),
            VoltStylePreset::SildursVibrant => (
                [0.64, 0.80, 1.18],
                [1.22, 0.68, 0.32],
                [0.10, 0.74, 0.84],
                [0.03, 0.30, 0.56],
            ),
            VoltStylePreset::ChocapicMobile => (
                [0.68, 0.84, 1.14],
                [1.06, 0.70, 0.38],
                [0.10, 0.64, 0.78],
                [0.03, 0.26, 0.48],
            ),
            VoltStylePreset::MakeUpFast => (
                [0.76, 0.90, 1.12],
                [1.08, 0.76, 0.44],
                [0.12, 0.72, 0.86],
                [0.04, 0.32, 0.56],
            ),
            VoltStylePreset::Nostalgia => (
                [0.82, 0.90, 1.08],
                [1.16, 0.76, 0.40],
                [0.14, 0.68, 0.78],
                [0.05, 0.30, 0.50],
            ),
            VoltStylePreset::SolasAurora => (
                [0.62, 0.78, 1.18],
                [1.05, 0.68, 0.52],
                [0.10, 0.72, 0.86],
                [0.03, 0.26, 0.52],
            ),
        }
    }

    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::hash::DefaultHasher::new();
        self.enabled.hash(&mut h);
        self.quality.hash(&mut h);
        self.style.hash(&mut h);
        self.low_ram_guard.hash(&mut h);
        self.sun_glow.to_bits().hash(&mut h);
        self.clouds.hash(&mut h);
        self.aurora.hash(&mut h);
        self.stars.hash(&mut h);
        self.fog_density.to_bits().hash(&mut h);
        self.god_rays.hash(&mut h);
        self.shadows.hash(&mut h);
        self.shadow_softness.hash(&mut h);
        self.translucent_shadows.hash(&mut h);
        self.shadow_distance.to_bits().hash(&mut h);
        self.sunlight_warmth.to_bits().hash(&mut h);
        self.ambient_brightness.to_bits().hash(&mut h);
        self.torch_warmth.to_bits().hash(&mut h);
        self.colored_blocklights.hash(&mut h);
        self.waving_vegetation.hash(&mut h);
        self.wind_speed.to_bits().hash(&mut h);
        self.wind_strength.to_bits().hash(&mut h);
        self.waving_water.hash(&mut h);
        self.water_wave_intensity.to_bits().hash(&mut h);
        self.water_reflections.hash(&mut h);
        self.water_clarity.to_bits().hash(&mut h);
        self.water_caustics.hash(&mut h);
        self.tone_mapping.hash(&mut h);
        self.bloom.hash(&mut h);
        self.bloom_intensity.to_bits().hash(&mut h);
        self.saturation.to_bits().hash(&mut h);
        self.contrast.to_bits().hash(&mut h);
        self.vibrance.to_bits().hash(&mut h);
        self.exposure.to_bits().hash(&mut h);
        self.rain_wetness.hash(&mut h);
        self.vignette.hash(&mut h);
        self.labpbr.hash(&mut h);
        h.finish()
    }

    pub fn estimated_vram_mb(&self) -> u32 {
        let shadow_res = self.shadows.resolution(self.low_ram_guard);
        let shadow_mb = if shadow_res == 0 {
            0
        } else if self.translucent_shadows && !self.low_ram_guard {
            // depth (4B) + translucent (4B) + color (4B)
            (shadow_res * shadow_res * 12) / (1024 * 1024)
        } else {
            // depth (4B) only
            (shadow_res * shadow_res * 4) / (1024 * 1024)
        };
        let refl_mb = if self.water_reflections == VoltWaterReflections::PlanarMirror
            && !self.low_ram_guard
        {
            6
        } else {
            0
        };
        let post_mb = if self.low_ram_guard {
            0
        } else {
            match self.bloom {
                VoltBloomMode::Off | VoltBloomMode::Fast => 0,
                VoltBloomMode::SoftQuarter | VoltBloomMode::Cinematic => 18,
            }
        };
        let depth_copy_mb = if self.water_reflections == VoltWaterReflections::PlanarMirror
            && self.water_caustics == VoltWaterCaustics::Realistic
            && !self.low_ram_guard
        {
            8
        } else {
            0
        };
        let pbr_mb = if self.labpbr { 32 } else { 0 };
        shadow_mb + refl_mb + post_mb + depth_copy_mb + pbr_mb
    }

    /// Builds an in-memory `Pack` + override constants tailored cleanly to the current settings.
    pub fn build_pack(&self) -> (Pack, Vec<(String, f64)>) {
        let shadow_res = self.shadows.resolution(self.low_ram_guard);
        let has_shadows = shadow_res > 0;
        let has_translucent_shadow = has_shadows && self.translucent_shadows && !self.low_ram_guard;
        let has_planar_refl =
            self.water_reflections == VoltWaterReflections::PlanarMirror && !self.low_ram_guard;
        let has_opaque_depth =
            has_planar_refl && self.water_caustics == VoltWaterCaustics::Realistic;
        let use_two_pass_bloom = matches!(
            self.bloom,
            VoltBloomMode::SoftQuarter | VoltBloomMode::Cinematic
        ) && !self.low_ram_guard;
        let use_one_pass_post = self.bloom == VoltBloomMode::Fast
            || (self.low_ram_guard
                && matches!(
                    self.bloom,
                    VoltBloomMode::SoftQuarter | VoltBloomMode::Cinematic
                ));
        let has_post = use_two_pass_bloom || use_one_pass_post;

        let mut manifest = format!(
            r#"schema = 1
id = "volt_plus_builtin"
name = "Volt+ Shaders ({style} · {quality})"
version = "1.0.0"
api = "0.1"
description = "Built-in Volt+ customizable shader suite."
authors = ["Volt+"]
license = "MIT"
shared = ["common.wgsl"]

[vertex]
file = "vertex.wgsl"
name = "Waving Foliage & Water"

[sky]
file = "sky.wgsl"
name = "Volt+ Atmosphere"
clouds = {vanilla_clouds}
sun = false
stars = false

[material]
file = "material.wgsl"
name = "Volt+ Surface"
"#,
            style = self.style.label(),
            quality = self.quality.label(),
            vanilla_clouds = if self.clouds == VoltCloudMode::Off {
                "true"
            } else {
                "false"
            },
        );

        if has_shadows {
            manifest.push_str(&format!(
                r#"
[shadow]
resolution = {shadow_res}
distance = {dist}
"#,
                dist = self.shadow_distance.clamp(48.0, 192.0),
            ));
        }

        if has_planar_refl {
            manifest.push_str(
                r#"
[reflection]
"#,
            );
        }

        if use_two_pass_bloom {
            manifest.push_str(
                r#"
[[post]]
file = "post/bloom.wgsl"
name = "Quarter-Res Bloom"
scale = 0.25
output = "bloom"

[[post]]
file = "post/composite.wgsl"
name = "Volt+ Tone Map & Grade"
"#,
            );
        } else if use_one_pass_post {
            manifest.push_str(
                r#"
[[post]]
file = "post/composite_fast.wgsl"
name = "Volt+ Fast Glow & Grade"
"#,
            );
        }

        manifest.push_str(
            r#"
[[materials]]
id = 1
sprites = [
  "*grass*", "*fern*", "*flower*", "*dandelion*", "*poppy*", "*orchid*",
  "*allium*", "*tulip*", "*daisy*", "*cornflower*", "*lily_of_the_valley*",
  "*rose*", "*sunflower*", "*lilac*", "*peony*", "*wheat*", "*carrots*",
  "*potatoes*", "*beetroots*", "*nether_wart*", "*sapling*", "*vine*",
  "*kelp*", "*seagrass*", "*sugar_cane*", "*bamboo*", "*sweet_berry_bush*",
  "*azalea*", "*spore_blossom*", "*dripleaf*", "*moss_carpet*",
  "*pink_petals*", "*wildflowers*", "*firefly_bush*", "*leaf_litter*",
  "*dead_bush*", "*torchflower*", "*pitcher_plant*"
]

[[materials]]
id = 2
sprites = ["*leaves*", "*mangrove_roots*"]

[[lights]]
blocks = ["*torch*", "*lantern*", "*campfire*", "*jack_o_lantern*"]
color = [1.0, 0.72, 0.34]

[[lights]]
blocks = ["*soul_torch*", "*soul_lantern*", "*soul_campfire*"]
color = [0.36, 0.88, 1.0]

[[lights]]
blocks = ["*lava*", "*magma*", "*shroomlight*", "*glowstone*"]
color = [1.0, 0.58, 0.24]

[[lights]]
blocks = ["*sea_lantern*", "*beacon*", "*conduit*", "*end_rod*"]
color = [0.76, 0.94, 1.0]

[[lights]]
blocks = ["*redstone*"]
color = [1.0, 0.28, 0.22]

[[lights]]
blocks = ["*amethyst*"]
color = [0.78, 0.48, 1.0]
"#,
        );

        let common_wgsl = Self::common_wgsl();
        let vertex_wgsl = self.vertex_wgsl();
        let sky_wgsl = self.sky_wgsl(!has_post);
        let material_wgsl = self.material_wgsl(
            has_shadows,
            has_translucent_shadow,
            has_planar_refl,
            has_opaque_depth,
            !has_post,
        );

        let mut files_map = BTreeMap::new();
        files_map.insert(crate::MANIFEST.to_owned(), manifest.as_bytes().to_vec());
        files_map.insert("common.wgsl".to_owned(), common_wgsl.into_bytes());
        files_map.insert("vertex.wgsl".to_owned(), vertex_wgsl.into_bytes());
        files_map.insert("sky.wgsl".to_owned(), sky_wgsl.into_bytes());
        files_map.insert("material.wgsl".to_owned(), material_wgsl.into_bytes());
        if use_two_pass_bloom {
            files_map.insert("post/bloom.wgsl".to_owned(), Self::bloom_wgsl().into_bytes());
            files_map.insert(
                "post/composite.wgsl".to_owned(),
                self.composite_wgsl(true).into_bytes(),
            );
        } else if use_one_pass_post {
            files_map.insert(
                "post/composite_fast.wgsl".to_owned(),
                self.composite_wgsl(false).into_bytes(),
            );
        }
        let manifest_parsed = crate::Manifest::parse(&manifest)
            .expect("generated Volt+ shader manifest must parse");
        let built_pack = Pack {
            manifest: manifest_parsed,
            files: files_map,
        };
        (built_pack, self.override_constants())
    }

    fn override_constants(&self) -> Vec<(String, f64)> {
        let mut out = Vec::new();
        if self.water_reflections == VoltWaterReflections::PlanarMirror {
            out.push(("enable_planar_reflection".to_owned(), 1.0));
        }
        out
    }

    fn common_wgsl() -> String {
        r#"fn volt_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn volt_hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn volt_noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (vec2<f32>(3.0) - 2.0 * f);
    let a = volt_hash21(i);
    let b = volt_hash21(i + vec2<f32>(1.0, 0.0));
    let c = volt_hash21(i + vec2<f32>(0.0, 1.0));
    let d = volt_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn volt_tonemap(c: vec3<f32>, mode: u32, exposure: f32) -> vec3<f32> {
    let x = max(c * exposure, vec3<f32>(0.0));
    if mode == 0u {
        // Natural crisp shoulder
        return x / (x * 0.42 + vec3<f32>(0.85));
    }
    if mode == 1u {
        // ACES Filmic
        return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    if mode == 2u {
        // BSL Soft S-curve
        let y = x / (vec3<f32>(1.0) + x * 0.55);
        return clamp(pow(y, vec3<f32>(0.96)), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    // Complementary punchy filmic
    let comp = (x * (2.20 * x + 0.05)) / (x * (2.05 * x + 0.52) + 0.12);
    return clamp(comp, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn volt_grade(c: vec3<f32>, saturation: f32, contrast: f32, vibrance: f32) -> vec3<f32> {
    let luma = volt_luma(c);
    let max_c = max(c.r, max(c.g, c.b));
    let min_c = min(c.r, min(c.g, c.b));
    let sat = max_c - min_c;
    let vib_mix = mix(1.0, vibrance, 1.0 - sat * 0.65);
    var res = mix(vec3<f32>(luma), c, saturation * vib_mix);
    res = (res - vec3<f32>(0.5)) * contrast + vec3<f32>(0.5);
    return clamp(res, vec3<f32>(0.0), vec3<f32>(1.0));
}
"#
        .to_owned()
    }

    fn vertex_wgsl(&self) -> String {
        let wave_veg = if self.waving_vegetation { "true" } else { "false" };
        let wave_water = if self.waving_water { "true" } else { "false" };
        format!(
            r#"const VOLT_VTX_WAVING_VEG: bool = {wave_veg};
const VOLT_VTX_WAVING_WATER: bool = {wave_water};
const VOLT_VTX_WIND_SPEED: f32 = {wind_speed:.4};
const VOLT_VTX_WIND_STRENGTH: f32 = {wind_strength:.4};
const VOLT_VTX_WATER_WAVE: f32 = {water_wave:.4};

fn vertex(input: VertexInput) -> vec3<f32> {{
    var pos = input.position;
    let world = pos + frame.camera_position;
    let t = frame.time * VOLT_VTX_WIND_SPEED;

    if VOLT_VTX_WAVING_VEG {{
        if input.material == 1u {{
            // Ground plants, grass, crops, flowers, vines, kelp: pin lower verts so bases stay anchored.
            let gust = 0.72 + 0.34 * sin(t * 0.9 + world.x * 0.14 + world.z * 0.11);
            let phase = t * 2.9 + world.x * 0.95 + world.z * 0.78 + world.y * 0.35;
            let amp = 0.072 * VOLT_VTX_WIND_STRENGTH * gust * (0.35 + 0.65 * input.light.y);
            let top_weight = clamp(1.0 - fract(input.uv.y * 16.0), 0.25, 1.0);
            pos.x += sin(phase) * amp * top_weight;
            pos.z += cos(phase * 0.83) * amp * 0.72 * top_weight;
        }} else if input.material == 2u {{
            // Tree leaves: coherent gentle canopy sway so adjacent leaf blocks stay connected.
            let phase = t * 2.1 + world.x * 0.55 + world.z * 0.48 + world.y * 0.42;
            let amp = 0.036 * VOLT_VTX_WIND_STRENGTH * (0.40 + 0.60 * input.light.y);
            pos.x += sin(phase) * amp;
            pos.y += cos(phase * 1.15) * amp * 0.42;
            pos.z += sin(phase * 0.87 + 1.3) * amp * 0.75;
        }}
    }}

    if VOLT_VTX_WAVING_WATER && input.fluid == 1u {{
        let wt = frame.time * 2.4;
        let w1 = sin(wt + world.x * 1.65 + world.z * 1.15);
        let w2 = cos(wt * 0.82 - world.x * 1.10 + world.z * 1.75);
        let h_disp = (w1 + w2 * 0.6) * 0.028 * VOLT_VTX_WATER_WAVE;
        pos.y += h_disp;
    }}

    return pos;
}}
"#,
            wind_speed = self.wind_speed.clamp(0.1, 3.0),
            wind_strength = self.wind_strength.clamp(0.0, 2.5),
            water_wave = self.water_wave_intensity.clamp(0.0, 2.5),
        )
    }

    fn sky_wgsl(&self, inline_grade: bool) -> String {
        let (zenith_tint, horizon_warm, _, _) = self.style_palette();
        let clouds_mode = match self.clouds {
            VoltCloudMode::Off => 0u32,
            VoltCloudMode::Fast => 1u32,
            VoltCloudMode::Volumetric => 2u32,
        };
        let stars_mode = match self.stars {
            VoltStarMode::Off => 0u32,
            VoltStarMode::Standard => 1u32,
            VoltStarMode::Rich => 2u32,
        };
        let aurora_mode = match self.aurora {
            VoltAuroraMode::Off => 0u32,
            VoltAuroraMode::Subtle => 1u32,
            VoltAuroraMode::Vivid => 2u32,
        };
        let tm_mode = match self.tone_mapping {
            VoltToneMapping::Natural => 0u32,
            VoltToneMapping::AcesFilmic => 1u32,
            VoltToneMapping::BslSoft => 2u32,
            VoltToneMapping::Complementary => 3u32,
        };
        let inline_code = if inline_grade {
            "col = volt_grade(volt_tonemap(col, VOLT_TM_MODE, VOLT_EXPOSURE), VOLT_SAT, VOLT_CONTRAST, VOLT_VIBRANCE);"
        } else {
            ""
        };
        format!(
            r#"const VOLT_CLOUDS: u32 = {clouds_mode}u;
const VOLT_STARS: u32 = {stars_mode}u;
const VOLT_AURORA: u32 = {aurora_mode}u;
const VOLT_SUN_GLOW: f32 = {sun_glow:.4};
const VOLT_FOG_DENSITY: f32 = {fog_density:.4};
const VOLT_ZENITH_TINT: vec3<f32> = vec3<f32>({z0:.4}, {z1:.4}, {z2:.4});
const VOLT_HORIZON_WARM: vec3<f32> = vec3<f32>({h0:.4}, {h1:.4}, {h2:.4});
const VOLT_TM_MODE: u32 = {tm_mode}u;
const VOLT_EXPOSURE: f32 = {exposure:.4};
const VOLT_SAT: f32 = {sat:.4};
const VOLT_CONTRAST: f32 = {contrast:.4};
const VOLT_VIBRANCE: f32 = {vibrance:.4};

fn sky(input: SkyInput) -> vec4<f32> {{
    let d = normalize(input.direction);
    if frame.dimension != 0 {{
        var col = input.color.rgb;
        {inline_code}
        return vec4<f32>(col, input.color.a);
    }}

    let sun_dot = max(dot(d, frame.sun_direction), 0.0);
    let moon_dot = max(dot(d, frame.moon_direction), 0.0);
    let day = clamp(frame.sun_direction.y * 2.8 + 0.35, 0.0, 1.0);
    let sunset = clamp(1.0 - abs(frame.sun_direction.y) * 3.8, 0.0, 1.0) * (1.0 - frame.rain * 0.75);
    let night = clamp(-frame.sun_direction.y * 3.0 + 0.15, 0.0, 1.0);
    let up = smoothstep(-0.05, 0.88, max(d.y, 0.0));
    let horizon = 1.0 - smoothstep(-0.06, 0.42, abs(d.y));

    var col = input.color.rgb * mix(vec3<f32>(1.0), VOLT_ZENITH_TINT, up * day);
    // Atmospheric horizon scattering & sunset glow
    let mie = pow(sun_dot, 5.0) * 0.55 + pow(sun_dot, 36.0) * 0.85;
    let sun_disk = smoothstep(0.9986, 0.9994, dot(d, frame.sun_direction));
    let moon_disk = smoothstep(0.9990, 0.9996, dot(d, frame.moon_direction));
    let warm_scatter = VOLT_HORIZON_WARM * (mie + horizon * (0.24 + sunset * 0.48)) * VOLT_SUN_GLOW * (1.0 - frame.rain * 0.8);
    col += warm_scatter * (0.45 + 0.55 * day + sunset * 0.5);
    col += vec3<f32>(1.15, 1.00, 0.82) * sun_disk * (1.4 - frame.rain);
    col += vec3<f32>(0.78, 0.86, 1.02) * moon_disk * night * (1.0 - frame.rain * 0.8);
    col += vec3<f32>(0.24, 0.32, 0.52) * pow(moon_dot, 12.0) * 0.35 * night * (1.0 - frame.rain);

    // Horizon mist
    col = mix(col, mix(col, frame.fog_color, 0.45), horizon * 0.35 * VOLT_FOG_DENSITY);

    // Night stars
    if VOLT_STARS > 0u && night > 0.05 && d.y > 0.02 {{
        let star_scale = select(110.0, 165.0, VOLT_STARS == 2u);
        let cell = floor(d.xz / max(d.y + 0.25, 0.25) * star_scale);
        let h = volt_hash21(cell);
        let thresh = select(0.992, 0.986, VOLT_STARS == 2u);
        if h > thresh {{
            let twinkle = 0.6 + 0.4 * sin(frame.time * 2.4 + h * 120.0);
            let star_col = mix(vec3<f32>(0.75, 0.86, 1.0), vec3<f32>(1.0, 0.90, 0.76), fract(h * 31.7));
            col += star_col * twinkle * smoothstep(0.02, 0.3, d.y) * night * (1.0 - frame.rain);
        }}
    }}

    // Night Aurora Borealis
    if VOLT_AURORA > 0u && night > 0.08 && d.y > 0.04 {{
        let uv_a = d.xz / (d.y + 0.35);
        let wave1 = sin(uv_a.x * 2.2 + frame.time * 0.24 + volt_noise2(uv_a * 1.4 + frame.time * 0.06) * 2.8);
        let band = smoothstep(0.65, 0.0, abs(wave1 + uv_a.y * 0.45));
        let strength = select(0.28, 0.58, VOLT_AURORA == 2u) * band * smoothstep(0.04, 0.35, d.y) * night * (1.0 - frame.rain);
        let aurora_col = mix(vec3<f32>(0.12, 0.95, 0.62), vec3<f32>(0.55, 0.34, 0.98), clamp(d.y * 1.3, 0.0, 1.0));
        col += aurora_col * strength;
    }}

    // Procedural clouds
    if VOLT_CLOUDS > 0u && d.y > 0.03 {{
        let cloud_uv = d.xz / (d.y + 0.18) * 1.15 + vec2<f32>(frame.time * 0.018, frame.time * 0.008);
        var n = volt_noise2(cloud_uv) * 0.62 + volt_noise2(cloud_uv * 2.3 + vec2<f32>(3.1, 1.7)) * 0.38;
        if VOLT_CLOUDS == 2u {{
            n = n * 0.78 + volt_noise2(cloud_uv * 4.8 - vec2<f32>(frame.time * 0.02)) * 0.22;
        }}
        let coverage = smoothstep(0.50 - frame.rain * 0.18, 0.80, n) * smoothstep(0.03, 0.22, d.y);
        let sun_rim = pow(sun_dot, 4.0) * 0.55 + 0.45;
        let cloud_lit = mix(vec3<f32>(0.24, 0.28, 0.38), mix(vec3<f32>(0.96, 0.97, 1.0), VOLT_HORIZON_WARM * 1.08, sunset * 0.65), day);
        let cloud_shadow = cloud_lit * (0.72 + 0.28 * sun_rim) * (1.0 - frame.rain * 0.42);
        col = mix(col, cloud_shadow, coverage * select(0.68, 0.82, VOLT_CLOUDS == 2u));
    }}

    {inline_code}
    return vec4<f32>(col, input.color.a);
}}
"#,
            z0 = zenith_tint[0],
            z1 = zenith_tint[1],
            z2 = zenith_tint[2],
            h0 = horizon_warm[0],
            h1 = horizon_warm[1],
            h2 = horizon_warm[2],
            sun_glow = self.sun_glow.clamp(0.0, 2.5),
            fog_density = self.fog_density.clamp(0.0, 2.5),
            exposure = self.exposure.clamp(0.4, 2.5),
            sat = self.saturation.clamp(0.0, 2.5),
            contrast = self.contrast.clamp(0.5, 2.0),
            vibrance = self.vibrance.clamp(0.0, 2.5),
        )
    }

    fn material_wgsl(
        &self,
        has_shadows: bool,
        has_translucent_shadow: bool,
        has_planar_refl: bool,
        has_opaque_depth: bool,
        inline_grade: bool,
    ) -> String {
        let (_, horizon_warm, water_shallow, water_deep) = self.style_palette();
        let shadow_soft = match self.shadow_softness {
            VoltShadowSoftness::Sharp => 0u32,
            VoltShadowSoftness::Soft4 => 1u32,
            VoltShadowSoftness::Ultra9 => 2u32,
        };
        let refl_mode = match self.water_reflections {
            VoltWaterReflections::Off => 0u32,
            VoltWaterReflections::SkyFresnel => 1u32,
            VoltWaterReflections::PlanarMirror => 2u32,
        };
        let caustics_mode = match self.water_caustics {
            VoltWaterCaustics::Off => 0u32,
            VoltWaterCaustics::Fast => 1u32,
            VoltWaterCaustics::Realistic => 2u32,
        };
        let tm_mode = match self.tone_mapping {
            VoltToneMapping::Natural => 0u32,
            VoltToneMapping::AcesFilmic => 1u32,
            VoltToneMapping::BslSoft => 2u32,
            VoltToneMapping::Complementary => 3u32,
        };

        let shadow_fn = if has_shadows {
            let trans_shadow = if has_translucent_shadow {
                r#"
    if frame.has_shadow != 0u {
        let st = textureSampleCompareLevel(shadow_map_translucent, shadow_sampler, sp.xy, sp.z - bias);
        if vis > 0.5 && st < 0.5 {
            let dim = vec2<f32>(textureDimensions(shadow_color));
            let tc = textureLoad(shadow_color, clamp(vec2<i32>(sp.xy * dim), vec2<i32>(0), vec2<i32>(dim) - 1), 0);
            if tc.a < 0.995 {
                tint_mul = mix(vec3<f32>(1.0), tc.rgb, clamp(tc.a * 1.1, 0.25, 0.85));
            }
        }
    }"#
            } else {
                ""
            };
            format!(
                r#"fn volt_shadow_factor(pos: vec3<f32>, ndotl: f32) -> vec4<f32> {{
    var vis = 1.0;
    var tint_mul = vec3<f32>(1.0);
    if frame.has_shadow == 0u {{
        return vec4<f32>(tint_mul, vis);
    }}
    let sp = shadow_position(pos);
    if sp.x <= 0.01 || sp.x >= 0.99 || sp.y <= 0.01 || sp.y >= 0.99 || sp.z >= 0.999 {{
        return vec4<f32>(tint_mul, vis);
    }}
    let bias = max(0.0012 * (1.0 - ndotl), 0.00035) + frame.shadow_center_texel * 0.0004;
    let texel = 1.0 / max(frame.shadow_resolution, 256.0);
    if VOLT_SHADOW_SOFT == 0u {{
        vis = textureSampleCompareLevel(shadow_map, shadow_sampler, sp.xy, sp.z - bias);
    }} else if VOLT_SHADOW_SOFT == 1u {{
        let o = texel * 1.1;
        vis = 0.25 * (
            textureSampleCompareLevel(shadow_map, shadow_sampler, sp.xy + vec2<f32>(-o, -o), sp.z - bias) +
            textureSampleCompareLevel(shadow_map, shadow_sampler, sp.xy + vec2<f32>( o, -o), sp.z - bias) +
            textureSampleCompareLevel(shadow_map, shadow_sampler, sp.xy + vec2<f32>(-o,  o), sp.z - bias) +
            textureSampleCompareLevel(shadow_map, shadow_sampler, sp.xy + vec2<f32>( o,  o), sp.z - bias)
        );
    }} else {{
        var sum = 0.0;
        let o = texel * 1.15;
        for (var dx = -1; dx <= 1; dx = dx + 1) {{
            for (var dy = -1; dy <= 1; dy = dy + 1) {{
                sum += textureSampleCompareLevel(
                    shadow_map,
                    shadow_sampler,
                    sp.xy + vec2<f32>(f32(dx), f32(dy)) * o,
                    sp.z - bias
                );
            }}
        }}
        vis = sum / 9.0;
    }}
    {trans_shadow}
    return vec4<f32>(tint_mul, vis);
}}"#
            )
        } else {
            r#"fn volt_shadow_factor(pos: vec3<f32>, ndotl: f32) -> vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}"#
            .to_owned()
        };

        let planar_refl_code = if has_planar_refl {
            r#"if frame.has_reflection != 0u && VOLT_WATER_REFL == 2u {
            let r_uv = clamp(input.screen + wave_n.xz * 0.045, vec2<f32>(0.004), vec2<f32>(0.996));
            let mirror = textureSampleLevel(reflection, reflection_sampler, r_uv, 0.0);
            sky_refl = mix(sky_refl, mirror.rgb, mirror.a * 0.88);
        }"#
        } else {
            ""
        };

        let opaque_depth_code = if has_opaque_depth {
            r#"let dist = length(input.position);
        let od_size = vec2<f32>(textureDimensions(opaque_depth));
        let od_tex = clamp(vec2<i32>(input.screen * od_size), vec2<i32>(0), vec2<i32>(od_size) - 1);
        let od = textureLoad(opaque_depth, od_tex, 0);
        let od_clip = vec4<f32>(input.screen.x * 2.0 - 1.0, 1.0 - input.screen.y * 2.0, od, 1.0);
        let od_view = frame.projection_inverse * od_clip;
        let scene_z = length(od_view.xyz / max(abs(od_view.w), 1e-5));
        let water_depth = clamp(max(scene_z - dist, 0.0) * 0.20, 0.0, 1.0);"#
        } else {
            "let water_depth = clamp(1.0 - VOLT_WATER_CLARITY, 0.15, 0.85);"
        };

        let pbr_code = if self.labpbr {
            r#"let nmap = block_normal_map(input.uv);
    let smap = block_specular_map(input.uv);
    let bitan = cross(input.normal, input.tangent.xyz) * input.tangent.w;
    let t_xy = nmap.xy * 2.0 - 1.0;
    let t_z = sqrt(max(1.0 - dot(t_xy, t_xy), 0.0));
    let n = normalize(input.tangent.xyz * t_xy.x + bitan * t_xy.y + input.normal * t_z);
    let pbr_smooth = smap.r;
    let pbr_ao = nmap.b;"#
        } else {
            r#"let n = input.normal;
    let pbr_smooth = 0.0;
    let pbr_ao = 1.0;"#
        };

        let inline_code = if inline_grade {
            "col = volt_grade(volt_tonemap(col, VOLT_TM_MODE, VOLT_EXPOSURE), VOLT_SAT, VOLT_CONTRAST, VOLT_VIBRANCE);"
        } else {
            ""
        };

        format!(
            r#"const VOLT_SHADOW_SOFT: u32 = {shadow_soft}u;
const VOLT_WATER_REFL: u32 = {refl_mode}u;
const VOLT_CAUSTICS: u32 = {caustics_mode}u;
const VOLT_SUN_WARMTH: f32 = {sun_warmth:.4};
const VOLT_AMBIENT: f32 = {ambient:.4};
const VOLT_TORCH_WARMTH: f32 = {torch_warmth:.4};
const VOLT_COLORED_LIGHTS: bool = {colored_lights};
const VOLT_WATER_CLARITY: f32 = {water_clarity:.4};
const VOLT_WATER_WAVE: f32 = {water_wave:.4};
const VOLT_RAIN_WETNESS: bool = {rain_wetness};
const VOLT_HORIZON_WARM: vec3<f32> = vec3<f32>({h0:.4}, {h1:.4}, {h2:.4});
const VOLT_WATER_SHALLOW: vec3<f32> = vec3<f32>({ws0:.4}, {ws1:.4}, {ws2:.4});
const VOLT_WATER_DEEP: vec3<f32> = vec3<f32>({wd0:.4}, {wd1:.4}, {wd2:.4});
const VOLT_TM_MODE: u32 = {tm_mode}u;
const VOLT_EXPOSURE: f32 = {exposure:.4};
const VOLT_SAT: f32 = {sat:.4};
const VOLT_CONTRAST: f32 = {contrast:.4};
const VOLT_VIBRANCE: f32 = {vibrance:.4};

{shadow_fn}

fn material(input: MaterialInput) -> Surface {{
    let world = input.position + frame.camera_position;
    let view_dir = normalize(-input.position);
    {pbr_code}

    let ndotl = max(dot(n, frame.light_direction), 0.0);
    let shadow_info = volt_shadow_factor(input.position, ndotl);
    let shadow_vis = shadow_info.w;
    let shadow_tint = shadow_info.xyz;

    let day_factor = clamp(frame.sun_direction.y * 2.6 + 0.35, 0.0, 1.0);
    let sunset = clamp(1.0 - abs(frame.sun_direction.y) * 3.8, 0.0, 1.0) * (1.0 - frame.rain * 0.7);
    let sun_color = mix(vec3<f32>(0.68, 0.78, 1.0), mix(vec3<f32>(1.06, 0.98, 0.90), VOLT_HORIZON_WARM * 1.15, sunset * 0.75 * VOLT_SUN_WARMTH), day_factor);

    // Directional sun + sky hemisphere ambient + subsurface scattering on foliage
    let sky_hemi = 0.86 + 0.14 * n.y;
    let direct_sun = ndotl * shadow_vis * input.light.y * (1.0 - frame.rain * 0.65);
    var sss = 0.0;
    if input.material == 1u || input.material == 2u {{
        let back = pow(max(dot(view_dir, -frame.light_direction), 0.0), 3.0);
        sss = (0.18 + back * 0.45) * shadow_vis * input.light.y * day_factor;
    }}

    // Warm torch + colored blocklights
    let torch_curve = pow(input.light.x, 1.45) * VOLT_TORCH_WARMTH;
    var torch_tint = vec3<f32>(1.12, 0.68, 0.30) * torch_curve * 0.75;
    if VOLT_COLORED_LIGHTS {{
        let bl = block_light_color(input.position);
        torch_tint = mix(torch_tint, bl.rgb * torch_curve * 0.95, clamp(bl.a, 0.0, 1.0));
        torch_tint += input.colored_light * 0.55;
    }}

    let base_lit = input.lightmap * VOLT_AMBIENT * sky_hemi * pbr_ao;
    let lit = base_lit * (0.78 + 0.46 * direct_sun * sun_color * shadow_tint + sss * sun_color)
        + torch_tint
        + vec3<f32>(input.emission * 0.55);

    var col = input.albedo.rgb * input.tint.rgb * lit;
    var alpha = input.albedo.a * input.tint.a;

    // Animated water surface with sky fresnel / planar mirror + caustics + specular glints
    if input.fluid == 1u {{
        let wt = frame.time * 2.1;
        let wx = sin(world.x * 2.9 + wt + cos(world.z * 1.8 - wt * 0.7)) * 0.085 * VOLT_WATER_WAVE;
        let wz = cos(world.z * 2.7 - wt * 0.85 + sin(world.x * 2.0 + wt * 0.6)) * 0.085 * VOLT_WATER_WAVE;
        let wave_n = normalize(n + vec3<f32>(wx, 0.0, wz));
        let fresnel = pow(1.0 - max(dot(wave_n, view_dir), 0.0), 3.6);
        let refl_dir = reflect(-view_dir, wave_n);
        var sky_refl = mix(frame.fog_color, frame.sky_color * 1.08, smoothstep(0.0, 0.65, max(refl_dir.y, 0.0)));
        {planar_refl_code}
        {opaque_depth_code}
        let water_base = mix(VOLT_WATER_SHALLOW, VOLT_WATER_DEEP, water_depth) * input.lightmap;
        let half_v = normalize(frame.light_direction + view_dir);
        let sun_spec = pow(max(dot(wave_n, half_v), 0.0), 96.0) * shadow_vis * input.light.y * (1.0 - frame.rain * 0.7) * 1.15;
        let refl_mix = select(0.0, clamp(0.14 + fresnel * 0.78, 0.0, 0.92), VOLT_WATER_REFL > 0u);
        col = mix(water_base * 0.88 + col * 0.16, sky_refl, refl_mix) + sun_color * sun_spec;
        alpha = clamp(mix(0.42, 0.88, water_depth * (1.0 - VOLT_WATER_CLARITY * 0.45) + fresnel * 0.38), 0.32, 0.94);
    }} else {{
        // Subtle underwater / sunlit wet caustics on solid blocks
        if VOLT_CAUSTICS > 0u && input.light.y > 0.25 && frame.in_water > 0.05 {{
            let c_wave = sin(world.x * 3.1 + frame.time * 2.5) * cos(world.z * 3.1 - frame.time * 2.1);
            let caust = pow(max(c_wave, 0.0), select(2.0, 3.0, VOLT_CAUSTICS == 2u));
            col += vec3<f32>(0.22, 0.56, 0.64) * caust * frame.in_water * 0.35;
        }}
        // Rain wetness & specular sheen on top faces
        if VOLT_RAIN_WETNESS && frame.wetness > 0.02 && input.face == 1u {{
            let puddle = smoothstep(0.42, 0.72, volt_noise2(world.xz * 0.35));
            let wet = frame.wetness * (0.4 + 0.6 * puddle);
            col *= (1.0 - wet * 0.18);
            let half_v = normalize(frame.light_direction + view_dir);
            let wet_spec = pow(max(dot(n, half_v), 0.0), 42.0) * wet * 0.35 * input.light.y;
            col += sun_color * wet_spec;
        }}
        // Optional LabPBR specular highlight
        if pbr_smooth > 0.05 {{
            let half_v = normalize(frame.light_direction + view_dir);
            let spec_pow = exp2(3.0 + pbr_smooth * 5.5);
            let spec = pow(max(dot(n, half_v), 0.0), spec_pow) * pbr_smooth * 0.42 * shadow_vis * input.light.y;
            col += sun_color * spec;
        }}
    }}

    {inline_code}
    return Surface(vec4<f32>(col, alpha));
}}
"#,
            sun_warmth = self.sunlight_warmth.clamp(0.0, 2.5),
            ambient = self.ambient_brightness.clamp(0.2, 2.5),
            torch_warmth = self.torch_warmth.clamp(0.0, 2.5),
            colored_lights = if self.colored_blocklights { "true" } else { "false" },
            water_clarity = self.water_clarity.clamp(0.05, 1.0),
            water_wave = self.water_wave_intensity.clamp(0.0, 2.5),
            rain_wetness = if self.rain_wetness { "true" } else { "false" },
            h0 = horizon_warm[0],
            h1 = horizon_warm[1],
            h2 = horizon_warm[2],
            ws0 = water_shallow[0],
            ws1 = water_shallow[1],
            ws2 = water_shallow[2],
            wd0 = water_deep[0],
            wd1 = water_deep[1],
            wd2 = water_deep[2],
            exposure = self.exposure.clamp(0.4, 2.5),
            sat = self.saturation.clamp(0.0, 2.5),
            contrast = self.contrast.clamp(0.5, 2.0),
            vibrance = self.vibrance.clamp(0.0, 2.5),
        )
    }

    fn bloom_wgsl() -> String {
        r#"fn post(input: PostInput) -> vec4<f32> {
    let texel = 1.0 / max(frame.resolution * 0.25, vec2<f32>(64.0));
    let o1 = texel * 1.6;
    let o2 = texel * 3.2;
    let c0 = textureSampleLevel(scene_color, linear_sampler, input.uv, 0.0).rgb;
    let c1 = textureSampleLevel(scene_color, linear_sampler, input.uv + vec2<f32>( o1.x,  o1.y), 0.0).rgb;
    let c2 = textureSampleLevel(scene_color, linear_sampler, input.uv + vec2<f32>(-o1.x,  o1.y), 0.0).rgb;
    let c3 = textureSampleLevel(scene_color, linear_sampler, input.uv + vec2<f32>( o1.x, -o1.y), 0.0).rgb;
    let c4 = textureSampleLevel(scene_color, linear_sampler, input.uv + vec2<f32>(-o1.x, -o1.y), 0.0).rgb;
    let c5 = textureSampleLevel(scene_color, linear_sampler, input.uv + vec2<f32>( o2.x, 0.0), 0.0).rgb;
    let c6 = textureSampleLevel(scene_color, linear_sampler, input.uv + vec2<f32>(-o2.x, 0.0), 0.0).rgb;
    let avg = c0 * 0.28 + (c1 + c2 + c3 + c4) * 0.13 + (c5 + c6) * 0.10;
    let luma = volt_luma(avg);
    let glow = avg * smoothstep(0.62, 1.25, luma);
    return vec4<f32>(glow, 1.0);
}
"#
        .to_owned()
    }

    fn composite_wgsl(&self, with_quarter_bloom: bool) -> String {
        let tm_mode = match self.tone_mapping {
            VoltToneMapping::Natural => 0u32,
            VoltToneMapping::AcesFilmic => 1u32,
            VoltToneMapping::BslSoft => 2u32,
            VoltToneMapping::Complementary => 3u32,
        };
        let vig_amt = match self.vignette {
            VoltVignetteMode::Off => 0.0f32,
            VoltVignetteMode::Subtle => 0.24f32,
            VoltVignetteMode::Strong => 0.48f32,
        };
        let god_rays = match self.god_rays {
            VoltGodRaysMode::Off => 0u32,
            VoltGodRaysMode::Fast => 1u32,
            VoltGodRaysMode::High => 2u32,
        };
        let bloom_sample = if with_quarter_bloom {
            r#"let bloom_col = textureSampleLevel(bloom, linear_sampler, input.uv, 0.0).rgb;
    col += bloom_col * VOLT_BLOOM_INTENSITY * 0.65;"#
        } else {
            r#"let texel = 2.5 / max(frame.resolution, vec2<f32>(320.0));
    let b1 = textureSampleLevel(previous, linear_sampler, input.uv + vec2<f32>( texel.x,  texel.y), 0.0).rgb;
    let b2 = textureSampleLevel(previous, linear_sampler, input.uv + vec2<f32>(-texel.x,  texel.y), 0.0).rgb;
    let b3 = textureSampleLevel(previous, linear_sampler, input.uv + vec2<f32>( texel.x, -texel.y), 0.0).rgb;
    let b4 = textureSampleLevel(previous, linear_sampler, input.uv + vec2<f32>(-texel.x, -texel.y), 0.0).rgb;
    let bavg = (b1 + b2 + b3 + b4) * 0.25;
    col += bavg * smoothstep(0.68, 1.25, volt_luma(bavg)) * VOLT_BLOOM_INTENSITY * 0.42;"#
        };

        let sky_mask_code = if self.low_ram_guard {
            "let sky_mask = mix(0.28, 1.0, smoothstep(-0.05, 0.35, normalize(ray_v).y));"
        } else {
            "let sky_mask = select(0.28, 1.0, input.depth <= 0.0001);"
        };

        format!(
            r#"const VOLT_TM_MODE: u32 = {tm_mode}u;
const VOLT_BLOOM_INTENSITY: f32 = {bloom:.4};
const VOLT_EXPOSURE: f32 = {exposure:.4};
const VOLT_SAT: f32 = {sat:.4};
const VOLT_CONTRAST: f32 = {contrast:.4};
const VOLT_VIBRANCE: f32 = {vibrance:.4};
const VOLT_VIGNETTE: f32 = {vig_amt:.4};
const VOLT_GOD_RAYS: u32 = {god_rays}u;

fn post(input: PostInput) -> vec4<f32> {{
    var col = input.color.rgb;
    {bloom_sample}

    if VOLT_GOD_RAYS > 0u && frame.dimension == 0 && frame.rain < 0.85 {{
        let clip_uv = vec2<f32>(input.uv.x * 2.0 - 1.0, 1.0 - input.uv.y * 2.0);
        let ray_v = (frame.view_inverse * (frame.projection_inverse * vec4<f32>(clip_uv, 0.0, 1.0))).xyz;
        let sun_align = max(dot(normalize(ray_v), frame.sun_direction), 0.0);
        let sun_vis = max(frame.sun_direction.y * 3.0 + 0.25, 0.0);
        {sky_mask_code}
        let shaft = pow(sun_align, select(7.0, 5.0, VOLT_GOD_RAYS == 2u)) * sun_vis * sky_mask * (1.0 - frame.rain);
        col += vec3<f32>(1.05, 0.80, 0.46) * shaft * select(0.26, 0.42, VOLT_GOD_RAYS == 2u);
    }}

    col = volt_tonemap(col, VOLT_TM_MODE, VOLT_EXPOSURE);
    col = volt_grade(col, VOLT_SAT, VOLT_CONTRAST, VOLT_VIBRANCE);

    if VOLT_VIGNETTE > 0.001 {{
        let d = distance(input.uv, vec2<f32>(0.5));
        let vig = 1.0 - smoothstep(0.32, 0.82, d) * VOLT_VIGNETTE;
        col *= vig;
    }}

    return vec4<f32>(col, input.color.a);
}}
"#,
            bloom = self.bloom_intensity.clamp(0.0, 2.5),
            exposure = self.exposure.clamp(0.4, 2.5),
            sat = self.saturation.clamp(0.0, 2.5),
            contrast = self.contrast.clamp(0.5, 2.0),
            vibrance = self.vibrance.clamp(0.0, 2.5),
        )
    }
}

fn files_get(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check;

    #[test]
    fn all_quality_and_style_presets_pass_shader_check() {
        for quality in VoltQualityPreset::ALL {
            for style in VoltStylePreset::ALL {
                let mut cfg = VoltShaderConfig::default();
                cfg.enabled = true;
                cfg.apply_style_and_quality(quality, style);
                assert!(
                    cfg.waving_vegetation,
                    "waving vegetation must stay enabled in {quality:?}"
                );
                assert!(
                    cfg.waving_water,
                    "waving water must stay enabled in {quality:?}"
                );
                let (pack, _) = cfg.build_pack();
                let res = check(&pack);
                assert!(
                    res.is_ok(),
                    "Pack validation failed for {quality:?} / {style:?}: {:?}",
                    res.err()
                );
            }
        }
    }

    #[test]
    fn potato_uses_zero_extra_vram_passes() {
        let mut cfg = VoltShaderConfig::default();
        cfg.enabled = true;
        cfg.apply_style_and_quality(VoltQualityPreset::Potato, VoltStylePreset::Bsl);
        assert_eq!(cfg.estimated_vram_mb(), 0);
        let (pack, _) = cfg.build_pack();
        assert!(pack.manifest.shadow.is_none());
        assert!(pack.manifest.reflection.is_none());
        assert!(pack.manifest.post.is_empty());
    }
}
