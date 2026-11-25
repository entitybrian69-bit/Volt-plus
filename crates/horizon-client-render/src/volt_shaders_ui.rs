//! Built-in Volt+ Shader Suite Settings UI (`Page::VoltShaders` / "Volt + settings"),
//! sharing the exact frosted slate & violet glass visual language, animations, search,
//! sliders, cycle pills, and footer description card of `volt_video_ui` & `volt_pages_ui`.

use std::time::Instant;

use horizon_shader_api::volt_shaders::{
    VoltAuroraMode, VoltBloomMode, VoltCloudMode, VoltGodRaysMode, VoltQualityPreset,
    VoltShaderConfig, VoltShadowQuality, VoltShadowSoftness, VoltStarMode, VoltStylePreset,
    VoltToneMapping, VoltVignetteMode, VoltWaterCaustics, VoltWaterReflections,
};

use crate::{
    graphics_style::Cost,
    hud::{HudAtlas, HudBatch, rgba},
    settings::Settings,
    text::Text,
    ui,
    volt_video_ui::{
        GROUP_HEIGHT, ROW_HEIGHT, ROW_STRIDE, Rect, cost_color, draw_border, fit_with_ellipsis,
        lerp_color, with_alpha,
    },
};

fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t.clamp(0.0, 1.0);
    1.0 - inv * inv * inv
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShaderTab {
    #[default]
    Presets,
    Atmosphere,
    Lighting,
    Water,
    Post,
}

impl ShaderTab {
    pub const ALL: [Self; 5] = [
        Self::Presets,
        Self::Atmosphere,
        Self::Lighting,
        Self::Water,
        Self::Post,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Presets => "Presets",
            Self::Atmosphere => "Sky & Fog",
            Self::Lighting => "Lighting",
            Self::Water => "Water & Wind",
            Self::Post => "Color & Post",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Presets => 0,
            Self::Atmosphere => 1,
            Self::Lighting => 2,
            Self::Water => 3,
            Self::Post => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderOpt {
    Enabled,
    QualityPreset,
    StylePreset,
    LowRamGuard,
    ResetPreset,

    SunGlow,
    Clouds,
    Aurora,
    Stars,
    FogDensity,
    GodRays,

    Shadows,
    ShadowSoftness,
    TranslucentShadows,
    ShadowDistance,
    SunlightWarmth,
    AmbientBrightness,
    TorchWarmth,
    ColoredBlocklights,

    WavingVegetation,
    WindSpeed,
    WindStrength,
    WavingWater,
    WaterWaveIntensity,
    WaterReflections,
    WaterClarity,
    WaterCaustics,

    ToneMapping,
    Bloom,
    BloomIntensity,
    Saturation,
    Contrast,
    Vibrance,
    Exposure,
    RainWetness,
    Vignette,
}

#[derive(Clone, Copy, Debug)]
struct ShaderEntry {
    opt: ShaderOpt,
    cost: Cost,
    title: &'static str,
    desc: &'static str,
}

enum ShaderRow {
    Header(&'static str),
    Entry(ShaderEntry),
}

fn entry(opt: ShaderOpt, cost: Cost, title: &'static str, desc: &'static str) -> ShaderRow {
    ShaderRow::Entry(ShaderEntry {
        opt,
        cost,
        title,
        desc,
    })
}

fn tab_rows(tab: ShaderTab) -> Vec<ShaderRow> {
    use Cost::*;
    match tab {
        ShaderTab::Presets => vec![
            ShaderRow::Header("Volt+ Built-in Shader"),
            entry(
                ShaderOpt::Enabled,
                Low,
                "Volt+ Shaders",
                "Enable the built-in Volt+ shader suite (automatically turns off external packs).",
            ),
            entry(
                ShaderOpt::QualityPreset,
                Medium,
                "Quality Preset",
                "Potato, Low (1 GB RAM fast), Medium, High or Ultra. All presets keep waving plants & water.",
            ),
            entry(
                ShaderOpt::StylePreset,
                None,
                "Style Preset",
                "Vanilla +, BSL, Complementary, Sildur's Vibrant, Chocapic13, MakeUp Fast, Nostalgia or Solas Aurora.",
            ),
            ShaderRow::Header("Memory & Safety"),
            entry(
                ShaderOpt::LowRamGuard,
                Saves,
                "1 GB RAM Guard",
                "Caps heavy shadow maps and protects low-memory Android GPUs from VRAM exhaustion.",
            ),
            entry(
                ShaderOpt::ResetPreset,
                None,
                "Reapply Preset Defaults",
                "Reset every fine-tuned slider and toggle back to the active Quality & Style preset.",
            ),
        ],
        ShaderTab::Atmosphere => vec![
            ShaderRow::Header("Sky & Scattering"),
            entry(
                ShaderOpt::SunGlow,
                None,
                "Sun & Horizon Glow",
                "Intensity of warm Mie horizon scattering and sunrise/sunset halo.",
            ),
            entry(
                ShaderOpt::Clouds,
                Low,
                "Procedural Clouds",
                "Off (fastest), Fast procedural sky clouds, or rich Volumetric-style layered clouds.",
            ),
            entry(
                ShaderOpt::Aurora,
                Low,
                "Aurora Borealis",
                "Shifting emerald and violet night sky ribbons (featured in Solas Aurora).",
            ),
            entry(
                ShaderOpt::Stars,
                None,
                "Night Stars",
                "Twinkling starfield and Milky Way density during night.",
            ),
            ShaderRow::Header("Atmosphere"),
            entry(
                ShaderOpt::FogDensity,
                None,
                "Atmospheric Mist",
                "Horizon haze blending distant terrain into the sky.",
            ),
            entry(
                ShaderOpt::GodRays,
                Medium,
                "Sunbeams (God Rays)",
                "Screen-space light shafts radiating from the sun when looking toward the sky.",
            ),
        ],
        ShaderTab::Lighting => vec![
            ShaderRow::Header("Real-Time Shadows"),
            entry(
                ShaderOpt::Shadows,
                High,
                "Shadow Map Resolution",
                "Off saves 100% shadow pass time & VRAM on 1 GB phones; 512px–2048px casts real sun shadows.",
            ),
            entry(
                ShaderOpt::ShadowSoftness,
                Medium,
                "Shadow Softness",
                "Sharp 1-tap, Soft 4-tap PCF, or Ultra 9-tap PCF filtering.",
            ),
            entry(
                ShaderOpt::TranslucentShadows,
                Medium,
                "Colored Glass Shadows",
                "Stained glass and translucent blocks tint sunlight passing through them.",
            ),
            entry(
                ShaderOpt::ShadowDistance,
                Medium,
                "Shadow Distance",
                "How far around the camera real-time shadows reach (blocks).",
            ),
            ShaderRow::Header("Direct & Block Light"),
            entry(
                ShaderOpt::SunlightWarmth,
                None,
                "Sunlight Warmth",
                "Warm golden tint on sunlit surfaces and sunset edges.",
            ),
            entry(
                ShaderOpt::AmbientBrightness,
                None,
                "Ambient Brightness",
                "Overall brightness of shaded areas, caves and nights.",
            ),
            entry(
                ShaderOpt::TorchWarmth,
                None,
                "Torchlight Warmth",
                "Amber glow warmth from torches, lanterns, fire and lava.",
            ),
            entry(
                ShaderOpt::ColoredBlocklights,
                Low,
                "Colored Blocklights",
                "Nearby emissive blocks and handheld torches cast colored light tints.",
            ),
        ],
        ShaderTab::Water => vec![
            ShaderRow::Header("Foliage & Wind"),
            entry(
                ShaderOpt::WavingVegetation,
                Low,
                "Waving Plants & Leaves",
                "Gentle wind animation on grass, crops, flowers, vines, kelp and tree leaves.",
            ),
            entry(
                ShaderOpt::WindSpeed,
                None,
                "Wind Speed",
                "How quickly plants and tree leaves sway in the breeze.",
            ),
            entry(
                ShaderOpt::WindStrength,
                None,
                "Wind Strength",
                "Amplitude of foliage and leaf wind sway.",
            ),
            ShaderRow::Header("Water & Reflections"),
            entry(
                ShaderOpt::WavingWater,
                Low,
                "Waving Water",
                "Animated water surface ripples and normal shimmer.",
            ),
            entry(
                ShaderOpt::WaterWaveIntensity,
                None,
                "Wave Strength",
                "Height and normal choppiness of water waves.",
            ),
            entry(
                ShaderOpt::WaterReflections,
                High,
                "Water Reflections",
                "Sky Fresnel uses zero extra VRAM; Planar Mirror renders real-time reflected terrain.",
            ),
            entry(
                ShaderOpt::WaterClarity,
                None,
                "Water Clarity",
                "How transparent shallow and deep water appears.",
            ),
            entry(
                ShaderOpt::WaterCaustics,
                Low,
                "Underwater Caustics",
                "Sunlit water ripple patterns projected onto submerged surfaces.",
            ),
        ],
        ShaderTab::Post => vec![
            ShaderRow::Header("Tone Mapping & Glow"),
            entry(
                ShaderOpt::ToneMapping,
                None,
                "Tone Mapping",
                "Natural, ACES Filmic, BSL Soft, or Complementary color curve.",
            ),
            entry(
                ShaderOpt::Bloom,
                Medium,
                "Bloom Glow",
                "Off uses 0 extra VRAM; Fast is 1-pass; Soft & Cinematic use a quarter-res bloom pass.",
            ),
            entry(
                ShaderOpt::BloomIntensity,
                None,
                "Bloom Intensity",
                "Strength of soft glow around bright skies, sunbeams and torches.",
            ),
            ShaderRow::Header("Color Grading & Weather"),
            entry(
                ShaderOpt::Exposure,
                None,
                "Exposure",
                "Overall scene brightness before tone mapping.",
            ),
            entry(
                ShaderOpt::Saturation,
                None,
                "Saturation",
                "Color richness across terrain, foliage and skies.",
            ),
            entry(
                ShaderOpt::Contrast,
                None,
                "Contrast",
                "Separation between deep shadows and sunlit highlights.",
            ),
            entry(
                ShaderOpt::Vibrance,
                None,
                "Vibrance",
                "Boosts muted colors cleanly without clipping bright highlights.",
            ),
            entry(
                ShaderOpt::RainWetness,
                Low,
                "Rain Wetness",
                "Adds glossy puddles and specular sheen to top block faces during rain.",
            ),
            entry(
                ShaderOpt::Vignette,
                None,
                "Vignette",
                "Soft darkening around screen corners.",
            ),
        ],
    }
}

fn cycle_index<T: Copy + PartialEq>(all: &[T], cur: T, back: bool) -> T {
    let i = all.iter().position(|x| *x == cur).unwrap_or(0);
    let next = if back {
        if i == 0 { all.len() - 1 } else { i - 1 }
    } else {
        (i + 1) % all.len()
    };
    all[next]
}

fn on_off(v: bool) -> String {
    if v { "On".to_owned() } else { "Off".to_owned() }
}

impl ShaderOpt {
    fn is_slider(self) -> bool {
        matches!(
            self,
            Self::SunGlow
                | Self::FogDensity
                | Self::ShadowDistance
                | Self::SunlightWarmth
                | Self::AmbientBrightness
                | Self::TorchWarmth
                | Self::WindSpeed
                | Self::WindStrength
                | Self::WaterWaveIntensity
                | Self::WaterClarity
                | Self::BloomIntensity
                | Self::Saturation
                | Self::Contrast
                | Self::Vibrance
                | Self::Exposure
        )
    }

    fn is_action(self) -> bool {
        matches!(self, Self::ResetPreset)
    }

    fn value_text(self, c: &VoltShaderConfig) -> String {
        match self {
            Self::Enabled => on_off(c.enabled),
            Self::QualityPreset => c.quality.label().to_owned(),
            Self::StylePreset => c.style.label().to_owned(),
            Self::LowRamGuard => on_off(c.low_ram_guard),
            Self::ResetPreset => "Apply".to_owned(),
            Self::SunGlow => format!("{:.0}%", c.sun_glow * 100.0),
            Self::Clouds => c.clouds.label().to_owned(),
            Self::Aurora => c.aurora.label().to_owned(),
            Self::Stars => c.stars.label().to_owned(),
            Self::FogDensity => format!("{:.0}%", c.fog_density * 100.0),
            Self::GodRays => c.god_rays.label().to_owned(),
            Self::Shadows => c.shadows.label().to_owned(),
            Self::ShadowSoftness => c.shadow_softness.label().to_owned(),
            Self::TranslucentShadows => on_off(c.translucent_shadows),
            Self::ShadowDistance => format!("{:.0} blk", c.shadow_distance),
            Self::SunlightWarmth => format!("{:.0}%", c.sunlight_warmth * 100.0),
            Self::AmbientBrightness => format!("{:.0}%", c.ambient_brightness * 100.0),
            Self::TorchWarmth => format!("{:.0}%", c.torch_warmth * 100.0),
            Self::ColoredBlocklights => on_off(c.colored_blocklights),
            Self::WavingVegetation => on_off(c.waving_vegetation),
            Self::WindSpeed => format!("{:.0}%", c.wind_speed * 100.0),
            Self::WindStrength => format!("{:.0}%", c.wind_strength * 100.0),
            Self::WavingWater => on_off(c.waving_water),
            Self::WaterWaveIntensity => format!("{:.0}%", c.water_wave_intensity * 100.0),
            Self::WaterReflections => c.water_reflections.label().to_owned(),
            Self::WaterClarity => format!("{:.0}%", c.water_clarity * 100.0),
            Self::WaterCaustics => c.water_caustics.label().to_owned(),
            Self::ToneMapping => c.tone_mapping.label().to_owned(),
            Self::Bloom => c.bloom.label().to_owned(),
            Self::BloomIntensity => format!("{:.0}%", c.bloom_intensity * 100.0),
            Self::Saturation => format!("{:.0}%", c.saturation * 100.0),
            Self::Contrast => format!("{:.0}%", c.contrast * 100.0),
            Self::Vibrance => format!("{:.0}%", c.vibrance * 100.0),
            Self::Exposure => format!("{:.0}%", c.exposure * 100.0),
            Self::RainWetness => on_off(c.rain_wetness),
            Self::Vignette => c.vignette.label().to_owned(),
        }
    }

    fn slider_frac(self, c: &VoltShaderConfig) -> Option<f32> {
        let norm = |v: f32, lo: f32, hi: f32| ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
        Some(match self {
            Self::SunGlow => norm(c.sun_glow, 0.0, 2.0),
            Self::FogDensity => norm(c.fog_density, 0.0, 2.0),
            Self::ShadowDistance => norm(c.shadow_distance, 48.0, 192.0),
            Self::SunlightWarmth => norm(c.sunlight_warmth, 0.0, 2.0),
            Self::AmbientBrightness => norm(c.ambient_brightness, 0.4, 1.8),
            Self::TorchWarmth => norm(c.torch_warmth, 0.0, 2.0),
            Self::WindSpeed => norm(c.wind_speed, 0.25, 2.5),
            Self::WindStrength => norm(c.wind_strength, 0.0, 2.0),
            Self::WaterWaveIntensity => norm(c.water_wave_intensity, 0.0, 2.0),
            Self::WaterClarity => norm(c.water_clarity, 0.15, 1.0),
            Self::BloomIntensity => norm(c.bloom_intensity, 0.0, 2.0),
            Self::Saturation => norm(c.saturation, 0.5, 1.8),
            Self::Contrast => norm(c.contrast, 0.75, 1.4),
            Self::Vibrance => norm(c.vibrance, 0.5, 1.8),
            Self::Exposure => norm(c.exposure, 0.6, 1.6),
            _ => return None,
        })
    }

    fn set_slider(self, c: &mut VoltShaderConfig, frac: f32) {
        let f = frac.clamp(0.0, 1.0);
        let lerp_step = |lo: f32, hi: f32, step: f32| -> f32 {
            let v = lo + (hi - lo) * f;
            (v / step).round() * step
        };
        match self {
            Self::SunGlow => c.sun_glow = lerp_step(0.0, 2.0, 0.05),
            Self::FogDensity => c.fog_density = lerp_step(0.0, 2.0, 0.05),
            Self::ShadowDistance => c.shadow_distance = lerp_step(48.0, 192.0, 8.0),
            Self::SunlightWarmth => c.sunlight_warmth = lerp_step(0.0, 2.0, 0.05),
            Self::AmbientBrightness => c.ambient_brightness = lerp_step(0.4, 1.8, 0.05),
            Self::TorchWarmth => c.torch_warmth = lerp_step(0.0, 2.0, 0.05),
            Self::WindSpeed => c.wind_speed = lerp_step(0.25, 2.5, 0.05),
            Self::WindStrength => c.wind_strength = lerp_step(0.0, 2.0, 0.05),
            Self::WaterWaveIntensity => c.water_wave_intensity = lerp_step(0.0, 2.0, 0.05),
            Self::WaterClarity => c.water_clarity = lerp_step(0.15, 1.0, 0.05),
            Self::BloomIntensity => c.bloom_intensity = lerp_step(0.0, 2.0, 0.05),
            Self::Saturation => c.saturation = lerp_step(0.5, 1.8, 0.05),
            Self::Contrast => c.contrast = lerp_step(0.75, 1.4, 0.02),
            Self::Vibrance => c.vibrance = lerp_step(0.5, 1.8, 0.05),
            Self::Exposure => c.exposure = lerp_step(0.6, 1.6, 0.02),
            _ => {}
        }
    }

    fn step_slider(self, s: &mut Settings, forward: bool) {
        let Some(cur) = self.slider_frac(&s.graphics.volt_shaders) else {
            return;
        };
        let delta = if forward { 0.05 } else { -0.05 };
        self.set_slider(&mut s.graphics.volt_shaders, (cur + delta).clamp(0.0, 1.0));
    }

    /// Applies a cycle/toggle change. Returns `true` if `s.horizon.shader_pack` was touched.
    fn cycle(self, s: &mut Settings, back: bool) -> bool {
        let c = &mut s.graphics.volt_shaders;
        let mut pack_changed = false;
        match self {
            Self::Enabled => {
                c.enabled = !c.enabled;
                if c.enabled && !s.horizon.shader_pack.is_empty() {
                    s.horizon.shader_pack.clear();
                    pack_changed = true;
                }
            }
            Self::QualityPreset => {
                let q = cycle_index(&VoltQualityPreset::ALL, c.quality, back);
                let style = c.style;
                c.apply_style_and_quality(q, style);
            }
            Self::StylePreset => {
                let st = cycle_index(&VoltStylePreset::ALL, c.style, back);
                let q = c.quality;
                c.apply_style_and_quality(q, st);
            }
            Self::LowRamGuard => c.low_ram_guard = !c.low_ram_guard,
            Self::ResetPreset => {
                let q = c.quality;
                let st = c.style;
                c.apply_style_and_quality(q, st);
            }
            Self::Clouds => c.clouds = cycle_index(&VoltCloudMode::ALL, c.clouds, back),
            Self::Aurora => c.aurora = cycle_index(&VoltAuroraMode::ALL, c.aurora, back),
            Self::Stars => c.stars = cycle_index(&VoltStarMode::ALL, c.stars, back),
            Self::GodRays => c.god_rays = cycle_index(&VoltGodRaysMode::ALL, c.god_rays, back),
            Self::Shadows => c.shadows = cycle_index(&VoltShadowQuality::ALL, c.shadows, back),
            Self::ShadowSoftness => {
                c.shadow_softness = cycle_index(&VoltShadowSoftness::ALL, c.shadow_softness, back)
            }
            Self::TranslucentShadows => c.translucent_shadows = !c.translucent_shadows,
            Self::ColoredBlocklights => c.colored_blocklights = !c.colored_blocklights,
            Self::WavingVegetation => c.waving_vegetation = !c.waving_vegetation,
            Self::WavingWater => c.waving_water = !c.waving_water,
            Self::WaterReflections => {
                c.water_reflections =
                    cycle_index(&VoltWaterReflections::ALL, c.water_reflections, back)
            }
            Self::WaterCaustics => {
                c.water_caustics = cycle_index(&VoltWaterCaustics::ALL, c.water_caustics, back)
            }
            Self::ToneMapping => {
                c.tone_mapping = cycle_index(&VoltToneMapping::ALL, c.tone_mapping, back)
            }
            Self::Bloom => c.bloom = cycle_index(&VoltBloomMode::ALL, c.bloom, back),
            Self::RainWetness => c.rain_wetness = !c.rain_wetness,
            Self::Vignette => c.vignette = cycle_index(&VoltVignetteMode::ALL, c.vignette, back),
            _ => {}
        }
        pack_changed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltShadersAction {
    ButtonClick,
    Changed,
    SliderDrag,
    ShaderPackChanged,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadersFocus {
    Search,
    Tab(ShaderTab),
    Item(ShaderOpt),
    Done,
}

#[derive(Clone, Copy, Debug)]
struct ShadersGeometry {
    side_x: f32,
    brand_y: f32,
    search_box: Rect,
    tabs: [(ShaderTab, Rect); 5],
    done_btn: Rect,
    header_bar: Rect,
    list_panel: Rect,
    list_clip: Rect,
    scrollbar_x: f32,
    footer_card: Rect,
}

#[derive(Clone, Debug)]
enum LaidShaderRow {
    Header { text: String, y: f32 },
    Entry { entry: ShaderEntry, rect: Rect, ctrl: Rect },
}

pub struct VoltShadersUi {
    pub tab: ShaderTab,
    pub scroll: ui::Scroll,
    pub focus: Option<ShadersFocus>,
    pub inspected: Option<ShaderOpt>,
    pub dragging: Option<ShaderOpt>,
    pub can_change_value: bool,
    opened_at: Option<Instant>,
    tab_changed_at: Instant,
    tab_dir: f32,
    visual_tab_y: Option<f32>,
    visual_scroll: Option<f32>,
    last_frame: Option<Instant>,
    tab_hover: [f32; 5],
    row_hover: Vec<(ShaderOpt, f32)>,
    slider_anim: Vec<(ShaderOpt, f32)>,
    done_hover: f32,
}

impl Default for VoltShadersUi {
    fn default() -> Self {
        Self {
            tab: ShaderTab::Presets,
            scroll: ui::Scroll::default(),
            focus: None,
            inspected: None,
            dragging: None,
            can_change_value: false,
            opened_at: None,
            tab_changed_at: Instant::now(),
            tab_dir: 1.0,
            visual_tab_y: None,
            visual_scroll: None,
            last_frame: None,
            tab_hover: [0.0; 5],
            row_hover: Vec::new(),
            slider_anim: Vec::new(),
            done_hover: 0.0,
        }
    }
}

impl VoltShadersUi {
    pub fn reset(&mut self) {
        self.scroll = ui::Scroll::default();
        self.focus = None;
        self.inspected = None;
        self.dragging = None;
        self.can_change_value = false;
        self.opened_at = Some(Instant::now());
        self.tab_changed_at = Instant::now();
        self.visual_tab_y = None;
        self.visual_scroll = None;
        self.last_frame = Some(Instant::now());
        self.row_hover.clear();
        self.slider_anim.clear();
    }

    pub fn reset_scroll(&mut self) {
        self.scroll.amount = 0.0;
        self.visual_scroll = None;
    }

    pub fn release(&mut self) -> bool {
        self.scroll.release();
        self.dragging.take().is_some()
    }

    pub fn select_tab(&mut self, next: ShaderTab) {
        if self.tab != next {
            self.tab_dir = if next.index() > self.tab.index() { 1.0 } else { -1.0 };
            self.tab = next;
            self.tab_changed_at = Instant::now();
            self.scroll.amount = 0.0;
            self.visual_scroll = None;
            self.inspected = None;
            self.dragging = None;
        }
    }

    fn geometry(w: f32, h: f32) -> ShadersGeometry {
        let pad_x = if w < 380.0 { 8.0 } else { 12.0 };
        let pad_top = 8.0;
        let pad_bottom = 8.0;
        let side_w = (w * 0.23).clamp(84.0, 118.0);
        let side_x = pad_x;
        let search_box = Rect {
            x: side_x,
            y: pad_top + 28.0,
            w: side_w,
            h: 16.0,
        };
        let tabs_top = search_box.y + search_box.h + 6.0;
        let tab_h = 16.0;
        let tab_stride = 18.0;
        let tabs = std::array::from_fn(|i| {
            (
                ShaderTab::ALL[i],
                Rect {
                    x: side_x,
                    y: tabs_top + i as f32 * tab_stride,
                    w: side_w,
                    h: tab_h,
                },
            )
        });
        let done_h = 20.0;
        let done_btn = Rect {
            x: side_x,
            y: (h - pad_bottom - done_h).max(tabs_top + 5.0 * tab_stride + 4.0),
            w: side_w,
            h: done_h,
        };
        let main_x = side_x + side_w + 10.0;
        let main_w = (w - main_x - pad_x).max(120.0);
        let header_bar = Rect {
            x: main_x,
            y: pad_top,
            w: main_w,
            h: 18.0,
        };
        let foot_h = if h < 220.0 { 34.0 } else { 38.0 };
        let footer_card = Rect {
            x: main_x,
            y: (h - pad_bottom - foot_h).max(header_bar.y + header_bar.h + 48.0),
            w: main_w,
            h: foot_h,
        };
        let list_y = header_bar.y + header_bar.h + 4.0;
        let list_bottom = (footer_card.y - 4.0).max(list_y + 24.0);
        let list_panel = Rect {
            x: main_x,
            y: list_y,
            w: main_w,
            h: list_bottom - list_y,
        };
        let list_clip = Rect {
            x: list_panel.x + 1.0,
            y: list_panel.y + 1.0,
            w: (list_panel.w - 2.0).max(1.0),
            h: (list_panel.h - 2.0).max(1.0),
        };
        ShadersGeometry {
            side_x,
            brand_y: pad_top,
            search_box,
            tabs,
            done_btn,
            header_bar,
            list_panel,
            list_clip,
            scrollbar_x: list_panel.x + list_panel.w - 6.0,
            footer_card,
        }
    }

    fn filtered_rows(&self, query: &str) -> (String, Vec<(Option<String>, ShaderEntry)>) {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            let mut out = Vec::new();
            let mut hdr = None;
            for r in tab_rows(self.tab) {
                match r {
                    ShaderRow::Header(h) => hdr = Some(h.to_owned()),
                    ShaderRow::Entry(e) => out.push((hdr.take(), e)),
                }
            }
            return (format!("Volt+ Shaders / {}", self.tab.label()), out);
        }
        let mut out = Vec::new();
        for tab in ShaderTab::ALL {
            for r in tab_rows(tab) {
                if let ShaderRow::Entry(e) = r {
                    if e.title.to_lowercase().contains(&q) || e.desc.to_lowercase().contains(&q) {
                        out.push((None, e));
                    }
                }
            }
        }
        (format!("Volt+ Shaders / Search: {}", query.trim()), out)
    }

    fn build_list_rows(
        &self,
        geom: &ShadersGeometry,
        query: &str,
        scroll_y: f32,
    ) -> (String, Vec<LaidShaderRow>, f32) {
        let (header, entries) = self.filtered_rows(query);
        let row_x = geom.list_panel.x + 6.0;
        let row_w = (geom.list_panel.w - 18.0).max(96.0);
        let ctrl_w = (row_w * 0.42).clamp(80.0, 146.0);
        let mut y = 6.0;
        let mut out = Vec::new();
        for (hdr, entry) in entries {
            if let Some(text) = hdr {
                out.push(LaidShaderRow::Header {
                    text,
                    y: geom.list_panel.y + y - scroll_y,
                });
                y += GROUP_HEIGHT;
            }
            let ry = geom.list_panel.y + y - scroll_y;
            let rect = Rect { x: row_x, y: ry, w: row_w, h: ROW_HEIGHT };
            let ctrl = Rect {
                x: row_x + row_w - ctrl_w - 4.0,
                y: ry + 2.0,
                w: ctrl_w,
                h: 16.0,
            };
            out.push(LaidShaderRow::Entry { entry, rect, ctrl });
            y += ROW_STRIDE;
        }
        (header, out, (y + 6.0).max(1.0))
    }

    fn smooth_value(cache: &mut Vec<(ShaderOpt, f32)>, key: ShaderOpt, target: f32, dt: f32, rate: f32) -> f32 {
        if let Some((_, v)) = cache.iter_mut().find(|(k, _)| *k == key) {
            if dt > 0.0 {
                let k = 1.0 - (-rate * dt).exp();
                *v += (target - *v) * k;
            } else {
                *v = target;
            }
            *v
        } else {
            let init = if dt > 0.0 { target * 0.25 } else { target };
            cache.push((key, init));
            init
        }
    }

    pub fn draw(
        &mut self,
        s: &Settings,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        in_world: bool,
        search: &mut ui::EditBox,
    ) {
        let now = Instant::now();
        let opened = *self.opened_at.get_or_insert(now);
        let dt = self
            .last_frame
            .replace(now)
            .map_or(0.0, |prev| now.saturating_duration_since(prev).as_secs_f32().min(0.05));

        let open_p = ease_out_cubic((now.saturating_duration_since(opened).as_secs_f32() / 0.16).clamp(0.0, 1.0));
        let tab_p = ease_out_cubic((now.saturating_duration_since(self.tab_changed_at).as_secs_f32() / 0.14).clamp(0.0, 1.0));
        let ui_alpha = 0.15 + 0.85 * open_p;
        let side_dx = (1.0 - open_p) * -10.0;
        let main_dy = (1.0 - open_p) * 6.0;

        let scrim = with_alpha(rgba(18, 17, 28, if in_world { 136 } else { 168 }), ui_alpha);
        let panel_bg = with_alpha(rgba(34, 32, 48, 228), ui_alpha);
        let bar_bg = with_alpha(rgba(42, 40, 60, 234), ui_alpha);
        let border_subtle = with_alpha(rgba(255, 255, 255, 58), ui_alpha);
        let border_strong = with_alpha(rgba(140, 118, 248, 235), ui_alpha);
        let accent = with_alpha(rgba(116, 92, 235, 255), ui_alpha);
        let accent_hi = with_alpha(rgba(140, 118, 248, 255), ui_alpha);
        let accent_edge = with_alpha(rgba(180, 164, 255, 230), ui_alpha);
        let text_primary = with_alpha(rgba(248, 247, 255, 255), ui_alpha);
        let text_muted = with_alpha(rgba(188, 186, 206, 255), ui_alpha);
        let text_dim = with_alpha(rgba(136, 134, 154, 255), ui_alpha);

        let (w, h) = (batch.width, batch.height);
        let geom = Self::geometry(w, h);
        let (_, _, content_h) = self.build_list_rows(&geom, &search.value, self.scroll.amount);
        self.scroll.clamp(geom.list_panel.y, geom.list_panel.y + geom.list_panel.h, content_h);
        let scroll_draw = if self.scroll.held() {
            self.visual_scroll = Some(self.scroll.amount);
            self.scroll.amount
        } else {
            let cur = self.visual_scroll.unwrap_or(self.scroll.amount);
            let next = if dt > 0.0 {
                let k = 1.0 - (-22.0 * dt).exp();
                cur + (self.scroll.amount - cur) * k
            } else {
                self.scroll.amount
            };
            self.visual_scroll = Some(next);
            next
        };
        let (header_title, rows, _) = self.build_list_rows(&geom, &search.value, scroll_draw);

        batch.fill(atlas, 0.0, 0.0, w, h, scrim);

        // Brand mark
        batch.text(atlas, "Volt+", geom.side_x + side_dx, geom.brand_y + 1.0, text_primary, true);
        let vw = HudBatch::text_width(atlas, "Volt+");
        batch.text(atlas, " Shaders", geom.side_x + side_dx + vw, geom.brand_y + 1.0, accent_hi, true);
        batch.text(atlas, "Built-in Suite", geom.side_x + side_dx, geom.brand_y + 12.0, text_dim, true);

        // Search box
        let sb = geom.search_box;
        let sb_x = sb.x + side_dx;
        let sb_focused = self.focus == Some(ShadersFocus::Search) || search.focused;
        let sb_hover = sb.contains(cursor) || sb_focused;
        batch.fill(atlas, sb_x, sb.y, sb.w, sb.h, panel_bg);
        draw_border(
            batch,
            atlas,
            sb_x,
            sb.y,
            sb.w,
            sb.h,
            if sb_focused { border_strong } else if sb_hover { with_alpha(border_strong, 0.65) } else { border_subtle },
        );
        crate::widgets::target_as(sb.array(), crate::widgets::Kind::Field);
        search.bordered = false;
        search.x = sb.x + 5.0;
        search.y = sb.y + 4.0;
        search.width = (sb.w - 10.0).max(16.0);
        search.height = 10.0;
        search.text_color = text_primary;
        search.message = "Search shaders".to_owned();
        if search.value.is_empty() && !search.focused {
            batch.text(atlas, "Search shaders", sb_x + 5.0, sb.y + 4.0, text_dim, false);
        }
        let saved_sx = search.x;
        search.x = sb_x + 5.0;
        search.draw(batch, atlas, None);
        search.x = saved_sx;
        crate::widgets::widget_cursor(sb_hover, true);

        // Sidebar tabs
        let searching = !search.value.trim().is_empty();
        let active_tab_rect = geom.tabs[self.tab.index()].1;
        let tab_y = {
            let cur = self.visual_tab_y.unwrap_or(active_tab_rect.y);
            let next = if dt > 0.0 {
                let k = 1.0 - (-22.0 * dt).exp();
                cur + (active_tab_rect.y - cur) * k
            } else {
                active_tab_rect.y
            };
            self.visual_tab_y = Some(next);
            next
        };
        for (i, (tab, rect)) in geom.tabs.iter().enumerate() {
            let selected = !searching && *tab == self.tab;
            let focused = self.focus == Some(ShadersFocus::Tab(*tab));
            let hovered = rect.contains(cursor);
            let target_h = if hovered || focused { 1.0 } else { 0.0 };
            if dt > 0.0 {
                let k = 1.0 - (-18.0 * dt).exp();
                self.tab_hover[i] += (target_h - self.tab_hover[i]) * k;
            } else {
                self.tab_hover[i] = target_h;
            }
            let rx = rect.x + side_dx;
            if selected {
                batch.fill(atlas, rx, rect.y, rect.w, rect.h, with_alpha(rgba(60, 56, 86, 235), ui_alpha));
            } else if self.tab_hover[i] > 0.02 {
                batch.fill(
                    atlas,
                    rx,
                    rect.y,
                    rect.w,
                    rect.h,
                    with_alpha(rgba(255, 255, 255, (self.tab_hover[i] * 34.0) as u8), ui_alpha),
                );
            }
            let col = if selected {
                text_primary
            } else {
                lerp_color(text_muted, text_primary, self.tab_hover[i] * 0.7)
            };
            batch.text(atlas, tab.label(), rx + 7.0, rect.y + 4.0, col, true);
            crate::widgets::record_as(tab.label(), rect.array(), true, crate::widgets::Kind::Tab(selected));
            crate::widgets::widget_cursor(hovered, true);
        }
        if !searching {
            let rx = active_tab_rect.x + side_dx;
            draw_border(batch, atlas, rx, tab_y, active_tab_rect.w, active_tab_rect.h, border_strong);
            batch.fill(atlas, rx + 1.0, tab_y + 3.0, 2.0, active_tab_rect.h - 6.0, accent_hi);
        }

        // Done button
        let done = geom.done_btn;
        let done_hovered = done.contains(cursor) || self.focus == Some(ShadersFocus::Done);
        if dt > 0.0 {
            let k = 1.0 - (-18.0 * dt).exp();
            self.done_hover += ((if done_hovered { 1.0 } else { 0.0 }) - self.done_hover) * k;
        } else {
            self.done_hover = if done_hovered { 1.0 } else { 0.0 };
        }
        let done_x = done.x + side_dx;
        batch.fill(atlas, done_x, done.y, done.w, done.h, lerp_color(accent, accent_hi, self.done_hover));
        batch.fill(atlas, done_x, done.y, done.w, 1.0, accent_edge);
        if self.focus == Some(ShadersFocus::Done) {
            draw_border(batch, atlas, done_x, done.y, done.w, done.h, border_strong);
        }
        let done_text = Text::tr("gui.done", &[]);
        ui::centered_text(batch, atlas, &done_text, done_x + 2.0, done.y + 6.0, done.w - 4.0, text_primary);
        crate::widgets::record(&done_text.plain(), done.array(), true);
        crate::widgets::widget_cursor(done_hovered, true);

        // Header bar + live VRAM badge
        let hdr = geom.header_bar;
        let hdr_y = hdr.y + main_dy;
        batch.fill(atlas, hdr.x, hdr_y, hdr.w, hdr.h, bar_bg);
        draw_border(batch, atlas, hdr.x, hdr_y, hdr.w, hdr.h, border_subtle);
        batch.text(
            atlas,
            &header_title,
            hdr.x + 8.0,
            hdr_y + 5.0,
            with_alpha(text_primary, ui_alpha * (0.35 + 0.65 * tab_p)),
            true,
        );
        let cfg = &s.graphics.volt_shaders;
        let vram_mb = cfg.estimated_vram_mb();
        let badge = if !cfg.enabled {
            "Off".to_owned()
        } else if vram_mb == 0 {
            "~0 MB VRAM (1 GB Safe)".to_owned()
        } else {
            format!("~{vram_mb} MB Extra VRAM")
        };
        let badge_col = if !cfg.enabled {
            text_dim
        } else if vram_mb == 0 {
            rgba(116, 224, 152, 255)
        } else if vram_mb <= 16 {
            rgba(150, 210, 255, 255)
        } else {
            rgba(244, 198, 96, 255)
        };
        let bw = HudBatch::text_width(atlas, &badge);
        batch.text(
            atlas,
            &badge,
            hdr.x + hdr.w - bw - 8.0,
            hdr_y + 5.0,
            with_alpha(badge_col, ui_alpha),
            true,
        );

        // List panel
        let lp = geom.list_panel;
        let lp_y = lp.y + main_dy;
        batch.fill(atlas, lp.x, lp_y, lp.w, lp.h, panel_bg);
        draw_border(batch, atlas, lp.x, lp_y, lp.w, lp.h, border_subtle);

        let content_slide_y = main_dy + (1.0 - tab_p) * self.tab_dir * 8.0;
        let content_alpha = ui_alpha * (0.25 + 0.75 * tab_p);
        let clip_x0 = geom.list_clip.x;
        let clip_y0 = geom.list_clip.y + main_dy;
        let clip_x1 = clip_x0 + geom.list_clip.w;
        let clip_y1 = clip_y0 + geom.list_clip.h;
        let list_verts = batch.vertices.len();
        let target_mark = crate::widgets::targets_mark();
        let in_list_y = cursor.1 >= lp.y && cursor.1 < lp.y + lp.h;

        let mut hovered_entry: Option<ShaderEntry> = None;
        let mut focused_entry: Option<ShaderEntry> = None;
        let mut first_entry: Option<ShaderEntry> = None;

        for row in &rows {
            match row {
                LaidShaderRow::Header { text, y } => {
                    let dy = *y + content_slide_y;
                    if dy + GROUP_HEIGHT < clip_y0 - 4.0 || dy > clip_y1 + 4.0 {
                        continue;
                    }
                    batch.text(atlas, text, lp.x + 8.0, dy + 4.0, with_alpha(text_muted, content_alpha), true);
                    batch.fill(atlas, lp.x + 8.0, dy + GROUP_HEIGHT - 3.0, lp.w - 20.0, 1.0, with_alpha(rgba(255, 255, 255, 22), content_alpha));
                }
                LaidShaderRow::Entry { entry, rect, ctrl } => {
                    if first_entry.is_none() {
                        first_entry = Some(*entry);
                    }
                    let dy = rect.y + content_slide_y;
                    let visible = dy + rect.h >= clip_y0 - 4.0 && dy <= clip_y1 + 4.0;
                    let is_hovered = in_list_y && rect.contains(cursor);
                    let is_focused = self.focus == Some(ShadersFocus::Item(entry.opt));
                    let is_dragging = self.dragging == Some(entry.opt);
                    let is_selected = self.inspected == Some(entry.opt);
                    if is_hovered {
                        hovered_entry = Some(*entry);
                    }
                    if is_focused {
                        focused_entry = Some(*entry);
                    }
                    let hl = Self::smooth_value(
                        &mut self.row_hover,
                        entry.opt,
                        if is_hovered || is_focused || is_dragging || is_selected { 1.0 } else { 0.0 },
                        dt,
                        18.0,
                    );
                    if !visible {
                        continue;
                    }
                    let idle_fill = with_alpha(rgba(255, 255, 255, 20), content_alpha);
                    let active_fill = with_alpha(rgba(255, 255, 255, 44), content_alpha);
                    batch.fill(atlas, rect.x, dy, rect.w, rect.h, lerp_color(idle_fill, active_fill, hl));
                    if is_selected || is_focused || is_dragging {
                        batch.fill(atlas, rect.x, dy + 3.0, 2.0, rect.h - 6.0, with_alpha(accent_hi, content_alpha));
                    }

                    let label_max = (ctrl.x - (rect.x + 7.0) - 6.0).max(24.0);
                    let cap_drawn = fit_with_ellipsis(atlas, entry.title, label_max);
                    batch.text(atlas, &cap_drawn, rect.x + 7.0, dy + 6.0, with_alpha(text_primary, content_alpha), true);

                    let val = entry.opt.value_text(cfg);
                    if let Some(real_v) = entry.opt.slider_frac(cfg) {
                        let shown_v = if is_dragging {
                            Self::smooth_value(&mut self.slider_anim, entry.opt, real_v, 0.0, 22.0);
                            if let Some((_, sv)) = self.slider_anim.iter_mut().find(|(k, _)| *k == entry.opt) {
                                *sv = real_v;
                            }
                            real_v
                        } else {
                            Self::smooth_value(&mut self.slider_anim, entry.opt, real_v, dt, 22.0)
                        };
                        let vw = HudBatch::text_width(atlas, &val);
                        let vx = (ctrl.x + (ctrl.w - vw) * 0.5).floor();
                        batch.text(atlas, &val, vx, dy + 2.0, with_alpha(text_primary, content_alpha), true);

                        let track_x = ctrl.x + 6.0;
                        let track_w = (ctrl.w - 12.0).max(16.0);
                        let track_y = dy + 14.0;
                        batch.fill(atlas, track_x, track_y, track_w, 2.0, with_alpha(rgba(255, 255, 255, 52), content_alpha));
                        batch.fill(atlas, track_x, track_y, track_w * shown_v, 2.0, with_alpha(rgba(130, 108, 245, 255), content_alpha));

                        let active_thumb = is_hovered || is_dragging || (is_focused && self.can_change_value);
                        let thumb_w = if active_thumb { 4.0 } else { 3.0 };
                        let thumb_h = if active_thumb { 8.0 } else { 6.0 };
                        let thumb_x = (track_x + track_w * shown_v - thumb_w * 0.5)
                            .clamp(track_x - 1.0, track_x + track_w - thumb_w + 1.0);
                        let thumb_y = track_y + 1.0 - thumb_h * 0.5;
                        batch.fill(atlas, thumb_x, thumb_y, thumb_w, thumb_h, with_alpha(rgba(255, 255, 255, 255), content_alpha));

                        crate::widgets::record_as(
                            entry.title,
                            ctrl.array(),
                            true,
                            crate::widgets::Kind::Slider {
                                value: shown_v,
                                track: [track_x, track_w],
                            },
                        );
                    } else {
                        let val_w = HudBatch::text_width(atlas, &val);
                        let is_act = entry.opt.is_action();
                        let pill_w = (val_w + if is_act { 18.0 } else { 26.0 })
                            .clamp((ctrl.w * 0.65).max(62.0).min(ctrl.w), ctrl.w);
                        let pill_x = ctrl.x + ctrl.w - pill_w;
                        let pill_y = dy + 2.0;
                        let pill_h = 16.0;
                        let active_on = entry.opt == ShaderOpt::Enabled && cfg.enabled;
                        let pill_bg = if active_on {
                            with_alpha(rgba(96, 74, 206, 225), content_alpha)
                        } else {
                            with_alpha(rgba(46, 43, 68, (175.0 + 45.0 * hl) as u8), content_alpha)
                        };
                        batch.fill(atlas, pill_x, pill_y, pill_w, pill_h, pill_bg);
                        draw_border(
                            batch,
                            atlas,
                            pill_x,
                            pill_y,
                            pill_w,
                            pill_h,
                            if is_focused || is_hovered || is_selected || active_on {
                                with_alpha(border_strong, content_alpha)
                            } else {
                                with_alpha(rgba(255, 255, 255, 58), content_alpha)
                            },
                        );
                        if !is_act {
                            let arrow_col = with_alpha(text_muted, content_alpha);
                            batch.text(atlas, "<", pill_x + 5.0, pill_y + 4.0, arrow_col, false);
                            let gt_w = HudBatch::text_width(atlas, ">");
                            batch.text(atlas, ">", pill_x + pill_w - gt_w - 5.0, pill_y + 4.0, arrow_col, false);
                        }
                        ui::centered_text(
                            batch,
                            atlas,
                            &Text::literal(val),
                            pill_x + 10.0,
                            pill_y + 4.0,
                            (pill_w - 20.0).max(8.0),
                            with_alpha(text_primary, content_alpha),
                        );
                        crate::widgets::record(entry.title, ctrl.array(), true);
                    }
                    crate::widgets::widget_cursor(is_hovered, true);
                }
            }
        }

        if content_h > lp.h + 0.5 {
            let max_s = (content_h - lp.h).max(1.0);
            let thumb_h = ((lp.h / content_h) * (lp.h - 8.0)).clamp(16.0, lp.h - 8.0);
            let ratio = (scroll_draw / max_s).clamp(0.0, 1.0);
            let thumb_y = lp_y + 4.0 + (lp.h - 8.0 - thumb_h) * ratio;
            batch.fill(atlas, geom.scrollbar_x, lp_y + 4.0, 2.0, lp.h - 8.0, with_alpha(rgba(255, 255, 255, 18), ui_alpha));
            batch.fill(atlas, geom.scrollbar_x, thumb_y, 2.0, thumb_h, with_alpha(accent_hi, ui_alpha * 0.85));
        }

        batch.clip_since(list_verts, clip_x0, clip_y0, clip_x1, clip_y1);
        crate::widgets::clip_targets(target_mark, lp_y, lp_y + lp.h);

        // Footer card
        let fc = geom.footer_card;
        let fc_y = fc.y + main_dy;
        batch.fill(atlas, fc.x, fc_y, fc.w, fc.h, bar_bg);
        draw_border(batch, atlas, fc.x, fc_y, fc.w, fc.h, border_subtle);
        let inspected_entry = hovered_entry
            .or(focused_entry)
            .or_else(|| {
                self.inspected.and_then(|wanted| {
                    rows.iter().find_map(|r| match r {
                        LaidShaderRow::Entry { entry, .. } if entry.opt == wanted => Some(*entry),
                        _ => None,
                    })
                })
            })
            .or(first_entry);
        if let Some(e) = inspected_entry {
            let right_label = (e.cost != Cost::None).then(|| (format!("[{}]", e.cost.label()), cost_color(e.cost)));
            let bw = right_label
                .as_ref()
                .map_or(0.0, |(t, _)| HudBatch::text_width(atlas, t) + 12.0);
            if let Some((txt, col)) = right_label {
                let tw = HudBatch::text_width(atlas, &txt);
                batch.text(atlas, &txt, fc.x + fc.w - tw - 8.0, fc_y + 5.0, with_alpha(col, ui_alpha), true);
            }
            let cap_fit = fit_with_ellipsis(atlas, e.title, (fc.w - bw - 16.0).max(40.0));
            batch.text(atlas, &cap_fit, fc.x + 8.0, fc_y + 5.0, text_primary, true);
            let desc_fit = fit_with_ellipsis(atlas, e.desc, (fc.w - 16.0).max(40.0));
            batch.text(atlas, &desc_fit, fc.x + 8.0, fc_y + 17.0, text_muted, true);
        }
    }

    pub fn click(
        &mut self,
        s: &mut Settings,
        atlas: &HudAtlas,
        pos: (f32, f32),
        size: (f32, f32),
        drag: bool,
        right: bool,
        search: &mut ui::EditBox,
    ) -> Vec<VoltShadersAction> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, content_h) = self.build_list_rows(&geom, &search.value, self.scroll.amount);
        let lp_top = geom.list_panel.y;
        let lp_bot = geom.list_panel.y + geom.list_panel.h;

        if drag {
            if self.scroll.held() {
                self.scroll.drag(pos.1, lp_top, lp_bot, content_h);
                self.visual_scroll = Some(self.scroll.amount);
                return Vec::new();
            }
            if let Some(opt) = self.dragging
                && let Some(LaidShaderRow::Entry { ctrl, .. }) = rows
                    .iter()
                    .find(|r| matches!(r, LaidShaderRow::Entry { entry, .. } if entry.opt == opt))
            {
                let track_x = ctrl.x + 6.0;
                let track_w = (ctrl.w - 12.0).max(16.0);
                let v = ((pos.0 - track_x) / track_w).clamp(0.0, 1.0);
                opt.set_slider(&mut s.graphics.volt_shaders, v);
                if let Some((_, sv)) = self.slider_anim.iter_mut().find(|(k, _)| *k == opt) {
                    *sv = opt.slider_frac(&s.graphics.volt_shaders).unwrap_or(v);
                }
                return vec![VoltShadersAction::SliderDrag, VoltShadersAction::Changed];
            }
            return Vec::new();
        }

        if !right && geom.search_box.contains(pos) {
            self.focus = Some(ShadersFocus::Search);
            search.focused = true;
            search.bordered = false;
            search.x = geom.search_box.x + 5.0;
            search.y = geom.search_box.y + 4.0;
            search.width = (geom.search_box.w - 10.0).max(16.0);
            search.click(atlas, pos.0, false, false);
            return vec![VoltShadersAction::ButtonClick];
        }
        search.focused = false;

        if !right {
            for (tab, rect) in geom.tabs {
                if rect.contains(pos) {
                    self.select_tab(tab);
                    self.focus = Some(ShadersFocus::Tab(tab));
                    if !search.value.is_empty() {
                        search.set_value("");
                    }
                    return vec![VoltShadersAction::ButtonClick];
                }
            }
            if geom.done_btn.contains(pos) {
                self.focus = Some(ShadersFocus::Done);
                return vec![VoltShadersAction::ButtonClick, VoltShadersAction::Done];
            }
            if content_h > geom.list_panel.h + 0.5
                && pos.0 >= geom.scrollbar_x - 3.0
                && pos.0 <= geom.list_panel.x + geom.list_panel.w
                && pos.1 >= lp_top
                && pos.1 < lp_bot
                && self.scroll.press(pos, geom.scrollbar_x - 2.0, lp_top, lp_bot, content_h)
            {
                return Vec::new();
            }
        }

        if geom.list_panel.contains(pos) {
            for row in rows {
                if let LaidShaderRow::Entry { entry, rect, ctrl } = row
                    && rect.contains(pos)
                {
                    self.focus = Some(ShadersFocus::Item(entry.opt));
                    self.inspected = Some(entry.opt);
                    if entry.opt.is_slider() {
                        if right {
                            return Vec::new();
                        }
                        if pos.0 >= ctrl.x {
                            let track_x = ctrl.x + 6.0;
                            let track_w = (ctrl.w - 12.0).max(16.0);
                            let v = ((pos.0 - track_x) / track_w).clamp(0.0, 1.0);
                            entry.opt.set_slider(&mut s.graphics.volt_shaders, v);
                            self.dragging = Some(entry.opt);
                            self.can_change_value = true;
                            return vec![VoltShadersAction::SliderDrag, VoltShadersAction::Changed];
                        }
                        return vec![VoltShadersAction::ButtonClick];
                    }
                    let step_back = right || (ctrl.contains(pos) && pos.0 < ctrl.x + ctrl.w * 0.35);
                    let pack_changed = entry.opt.cycle(s, step_back);
                    let mut out = vec![VoltShadersAction::ButtonClick, VoltShadersAction::Changed];
                    if pack_changed {
                        out.push(VoltShadersAction::ShaderPackChanged);
                    }
                    return out;
                }
            }
        }
        Vec::new()
    }

    pub fn scroll(&mut self, size: (f32, f32), query: &str, amount: f32) {
        let geom = Self::geometry(size.0, size.1);
        let (_, _, content_h) = self.build_list_rows(&geom, query, self.scroll.amount);
        self.scroll.wheel(
            amount,
            20.0,
            geom.list_panel.y,
            geom.list_panel.y + geom.list_panel.h,
            content_h,
        );
    }

    pub fn key_modifiers(
        &mut self,
        s: &mut Settings,
        key: winit::keyboard::KeyCode,
        text: Option<&str>,
        shift: bool,
        size: (f32, f32),
        search: &mut ui::EditBox,
    ) -> (Vec<VoltShadersAction>, bool) {
        use winit::keyboard::KeyCode as K;
        if key == K::Escape {
            return (vec![VoltShadersAction::Done], true);
        }
        if (self.focus == Some(ShadersFocus::Search) || search.focused)
            && !matches!(key, K::Tab | K::ArrowDown | K::ArrowUp | K::Enter | K::NumpadEnter)
        {
            let before = search.value.clone();
            if !search.key(key, false, shift)
                && let Some(t) = text
            {
                let typed: String = t.chars().filter(|c| !c.is_control()).collect();
                if !typed.is_empty() {
                    search.insert_text(&typed);
                }
            }
            if search.value != before {
                self.reset_scroll();
            }
            return (Vec::new(), true);
        }

        let geom = Self::geometry(size.0, size.1);
        let (_, rows, content_h) = self.build_list_rows(&geom, &search.value, self.scroll.amount);
        let mut focusables: Vec<(ShadersFocus, Rect, bool)> = Vec::new();
        focusables.push((ShadersFocus::Search, geom.search_box, false));
        for (tab, rect) in geom.tabs {
            focusables.push((ShadersFocus::Tab(tab), rect, false));
        }
        focusables.push((ShadersFocus::Done, geom.done_btn, false));
        for r in &rows {
            if let LaidShaderRow::Entry { entry, rect, .. } = r {
                focusables.push((ShadersFocus::Item(entry.opt), *rect, true));
            }
        }
        if focusables.is_empty() {
            return (Vec::new(), false);
        }

        let ensure_visible = |this: &mut Self, rect: Rect, in_list: bool| {
            if !in_list {
                return;
            }
            let top = geom.list_panel.y + 4.0;
            let bottom = geom.list_panel.y + geom.list_panel.h - 4.0;
            if rect.y < top {
                this.scroll.amount -= top - rect.y;
            } else if rect.y + rect.h > bottom {
                this.scroll.amount += rect.y + rect.h - bottom;
            }
            this.scroll.clamp(geom.list_panel.y, geom.list_panel.y + geom.list_panel.h, content_h);
        };

        if matches!(key, K::Tab | K::ArrowUp | K::ArrowDown) {
            let back = (key == K::Tab && shift) || key == K::ArrowUp;
            let cur = self
                .focus
                .and_then(|f| focusables.iter().position(|(id, ..)| *id == f));
            let next = match cur {
                Some(i) => {
                    if back {
                        if i == 0 { focusables.len() - 1 } else { i - 1 }
                    } else {
                        (i + 1) % focusables.len()
                    }
                }
                None => 0,
            };
            let (id, rect, in_list) = focusables[next];
            self.focus = Some(id);
            search.focused = id == ShadersFocus::Search;
            self.can_change_value = matches!(id, ShadersFocus::Item(o) if o.is_slider());
            if let ShadersFocus::Item(o) = id {
                self.inspected = Some(o);
            }
            ensure_visible(self, rect, in_list);
            return (Vec::new(), true);
        }

        if matches!(key, K::ArrowLeft | K::ArrowRight) {
            let forward = key == K::ArrowRight;
            if let Some(ShadersFocus::Item(opt)) = self.focus {
                if opt.is_slider() {
                    opt.step_slider(s, forward);
                    return (vec![VoltShadersAction::SliderDrag, VoltShadersAction::Changed], true);
                }
                let pack_changed = opt.cycle(s, !forward);
                let mut out = vec![VoltShadersAction::ButtonClick, VoltShadersAction::Changed];
                if pack_changed {
                    out.push(VoltShadersAction::ShaderPackChanged);
                }
                return (out, true);
            }
            return (Vec::new(), false);
        }

        if matches!(key, K::Enter | K::NumpadEnter | K::Space) {
            match self.focus {
                Some(ShadersFocus::Done) => return (vec![VoltShadersAction::ButtonClick, VoltShadersAction::Done], true),
                Some(ShadersFocus::Tab(tab)) => {
                    if !search.value.is_empty() {
                        search.set_value("");
                    }
                    self.select_tab(tab);
                    return (vec![VoltShadersAction::ButtonClick], true);
                }
                Some(ShadersFocus::Item(opt)) => {
                    if !opt.is_slider() {
                        let pack_changed = opt.cycle(s, shift);
                        let mut out = vec![VoltShadersAction::ButtonClick, VoltShadersAction::Changed];
                        if pack_changed {
                            out.push(VoltShadersAction::ShaderPackChanged);
                        }
                        return (out, true);
                    }
                }
                _ => {}
            }
        }
        (Vec::new(), false)
    }
}
