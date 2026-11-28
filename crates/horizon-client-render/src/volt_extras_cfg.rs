//! Configuration structs, metadata, Modrinth project slugs, keybinds, and persistence
//! for all 84 built-in Volt+ Extras mods (every mod defaults to `enabled: false`).
use crate::extras::Corner;
use std::fmt::Write as _;

pub const VOLT_MOD_COUNT: usize = 83;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltKeybindMode {
    Tap,
    Hold,
}
impl VoltKeybindMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tap => "tap",
            Self::Hold => "hold",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Tap => "Tap (Toggle)",
            Self::Hold => "Hold (While Held)",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Tap => Self::Hold,
            Self::Hold => Self::Tap,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "tap" => Self::Tap,
            "hold" => Self::Hold,
            _ => default,
        }
    }
}

/// Smart per-mod default activation style (`Hold` for momentary camera/movement mods, `Tap` for toggles).
pub fn default_keybind_mode_for_mod(id: u8) -> VoltKeybindMode {
    match id {
        4 | 16 | 25 | 47 | 59 => VoltKeybindMode::Hold, // Freelook, Zoomify, Freecam, Safe-Walk, PvP Reach Box
        _ => VoltKeybindMode::Tap,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoltModKeybind {
    pub toggle_key: String,
    pub use_key: String,
    pub mode: VoltKeybindMode,
}
impl VoltModKeybind {
    pub fn default_for_id(id: u8) -> Self {
        Self {
            toggle_key: String::new(),
            use_key: String::new(),
            mode: default_keybind_mode_for_mod(id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoltModKeybinds(pub [VoltModKeybind; VOLT_MOD_COUNT]);
impl Default for VoltModKeybinds {
    fn default() -> Self {
        Self(std::array::from_fn(|i| VoltModKeybind::default_for_id(i as u8)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltModCategory {
    All,
    Pvp,
    Hud,
    Camera,
    World,
    Utility,
}

impl VoltModCategory {
    pub const TABS: [Self; 6] = [
        Self::All,
        Self::Pvp,
        Self::Hud,
        Self::Camera,
        Self::World,
        Self::Utility,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Pvp => "PvP",
            Self::Hud => "HUD",
            Self::Camera => "Camera",
            Self::World => "World",
            Self::Utility => "Utility",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoltModMeta {
    pub id: u8,
    pub key: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub badge: &'static str,
    pub category: VoltModCategory,
    pub modrinth_slugs: &'static [&'static str],
    pub search_query: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldColorPreset {
    Green,
    Cyan,
    Blue,
    White,
    Red,
    Orange,
    Magenta,
    Gray,
}
impl ShieldColorPreset {
    pub const ENABLED_CHOICES: [Self; 4] = [Self::Green, Self::Cyan, Self::Blue, Self::White];
    pub const DISABLED_CHOICES: [Self; 4] = [Self::Red, Self::Orange, Self::Magenta, Self::Gray];
    pub fn name(self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Cyan => "cyan",
            Self::Blue => "blue",
            Self::White => "white",
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Magenta => "magenta",
            Self::Gray => "gray",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Green => "Green",
            Self::Cyan => "Cyan",
            Self::Blue => "Blue",
            Self::White => "White",
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Magenta => "Magenta",
            Self::Gray => "Gray",
        }
    }
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Green => [84, 232, 126],
            Self::Cyan => [82, 224, 240],
            Self::Blue => [96, 152, 255],
            Self::White => [244, 246, 255],
            Self::Red => [242, 82, 92],
            Self::Orange => [255, 162, 68],
            Self::Magenta => [224, 96, 240],
            Self::Gray => [156, 160, 174],
        }
    }
    pub fn next_enabled(self) -> Self {
        let i = Self::ENABLED_CHOICES.iter().position(|&c| c == self).unwrap_or(0);
        Self::ENABLED_CHOICES[(i + 1) % Self::ENABLED_CHOICES.len()]
    }
    pub fn next_disabled(self) -> Self {
        let i = Self::DISABLED_CHOICES.iter().position(|&c| c == self).unwrap_or(0);
        Self::DISABLED_CHOICES[(i + 1) % Self::DISABLED_CHOICES.len()]
    }
    pub fn next_any(self) -> Self {
        const ALL: [ShieldColorPreset; 8] = [
            ShieldColorPreset::Green,
            ShieldColorPreset::Cyan,
            ShieldColorPreset::Blue,
            ShieldColorPreset::White,
            ShieldColorPreset::Red,
            ShieldColorPreset::Orange,
            ShieldColorPreset::Magenta,
            ShieldColorPreset::Gray,
        ];
        let i = ALL.iter().position(|&c| c == self).unwrap_or(0);
        ALL[(i + 1) % ALL.len()]
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "green" => Self::Green,
            "cyan" => Self::Cyan,
            "blue" => Self::Blue,
            "white" => Self::White,
            "red" => Self::Red,
            "orange" => Self::Orange,
            "magenta" => Self::Magenta,
            "gray" => Self::Gray,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldDisplayMode {
    Both,
    HudOnly,
    ShieldOnly,
}
impl ShieldDisplayMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::HudOnly => "hud_only",
            Self::ShieldOnly => "shield_only",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "HUD & Shield",
            Self::HudOnly => "HUD Only",
            Self::ShieldOnly => "Shield Only",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Both => Self::HudOnly,
            Self::HudOnly => Self::ShieldOnly,
            Self::ShieldOnly => Self::Both,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "both" => Self::Both,
            "hud_only" => Self::HudOnly,
            "shield_only" => Self::ShieldOnly,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UkuAnchor {
    Hotbar,
    Top,
    Bottom,
}
impl UkuAnchor {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hotbar => "hotbar",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Hotbar => "Hotbar",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Hotbar => Self::Top,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Hotbar,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "hotbar" => Self::Hotbar,
            "top" => Self::Top,
            "bottom" => Self::Bottom,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UkuSide {
    Left,
    Right,
}
impl UkuSide {
    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "left" => Self::Left,
            "right" => Self::Right,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UkuStyle {
    Hotbar,
    Rounded,
    Flat,
    None,
}
impl UkuStyle {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hotbar => "hotbar",
            Self::Rounded => "rounded",
            Self::Flat => "flat",
            Self::None => "none",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Hotbar => "Hotbar",
            Self::Rounded => "Rounded",
            Self::Flat => "Flat",
            Self::None => "Minimal",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Hotbar => Self::Rounded,
            Self::Rounded => Self::Flat,
            Self::Flat => Self::None,
            Self::None => Self::Hotbar,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "hotbar" => Self::Hotbar,
            "rounded" => Self::Rounded,
            "flat" => Self::Flat,
            "none" => Self::None,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UkuDurability {
    Bar,
    Numeric,
    Percentage,
    None,
}
impl UkuDurability {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Numeric => "numeric",
            Self::Percentage => "percentage",
            Self::None => "none",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Bar => "Bar",
            Self::Numeric => "Numeric",
            Self::Percentage => "Percentage",
            Self::None => "Off",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Bar => Self::Numeric,
            Self::Numeric => Self::Percentage,
            Self::Percentage => Self::None,
            Self::None => Self::Bar,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "bar" => Self::Bar,
            "numeric" => Self::Numeric,
            "percentage" => Self::Percentage,
            "none" => Self::None,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UkuPotionAnchor {
    TopRight,
    TopLeft,
    BottomLeft,
}
impl UkuPotionAnchor {
    pub fn name(self) -> &'static str {
        match self {
            Self::TopRight => "top_right",
            Self::TopLeft => "top_left",
            Self::BottomLeft => "bottom_left",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::TopRight => "Top Right",
            Self::TopLeft => "Top Left",
            Self::BottomLeft => "Bottom Left",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::TopRight => Self::TopLeft,
            Self::TopLeft => Self::BottomLeft,
            Self::BottomLeft => Self::TopRight,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "top_right" => Self::TopRight,
            "top_left" => Self::TopLeft,
            "bottom_left" => Self::BottomLeft,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrosshairStyle {
    Vanilla,
    Cross,
    Dot,
    Circle,
}
impl CrosshairStyle {
    pub fn name(self) -> &'static str {
        match self {
            Self::Vanilla => "vanilla",
            Self::Cross => "cross",
            Self::Dot => "dot",
            Self::Circle => "circle",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Cross => "Clean Cross",
            Self::Dot => "Dot",
            Self::Circle => "Circle",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Vanilla => Self::Cross,
            Self::Cross => Self::Dot,
            Self::Dot => Self::Circle,
            Self::Circle => Self::Vanilla,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "vanilla" => Self::Vanilla,
            "cross" => Self::Cross,
            "dot" => Self::Dot,
            "circle" => Self::Circle,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeMood {
    Default,
    Noon,
    Sunset,
    Midnight,
}
impl TimeMood {
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Noon => "noon",
            Self::Sunset => "sunset",
            Self::Midnight => "midnight",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Server Time",
            Self::Noon => "Noon",
            Self::Sunset => "Sunset",
            Self::Midnight => "Midnight",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Noon,
            Self::Noon => Self::Sunset,
            Self::Sunset => Self::Midnight,
            Self::Midnight => Self::Default,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "default" => Self::Default,
            "noon" => Self::Noon,
            "sunset" => Self::Sunset,
            "midnight" => Self::Midnight,
            _ => default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlintColorPreset {
    Vanilla,
    Rainbow,
    Gold,
    Ice,
    Crimson,
    Emerald,
    Off,
}
impl GlintColorPreset {
    pub fn name(self) -> &'static str {
        match self {
            Self::Vanilla => "vanilla",
            Self::Rainbow => "rainbow",
            Self::Gold => "gold",
            Self::Ice => "ice",
            Self::Crimson => "crimson",
            Self::Emerald => "emerald",
            Self::Off => "off",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla Purple",
            Self::Rainbow => "Rainbow",
            Self::Gold => "Solar Gold",
            Self::Ice => "Frost Cyan",
            Self::Crimson => "Crimson",
            Self::Emerald => "Emerald",
            Self::Off => "Off",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Vanilla => Self::Rainbow,
            Self::Rainbow => Self::Gold,
            Self::Gold => Self::Ice,
            Self::Ice => Self::Crimson,
            Self::Crimson => Self::Emerald,
            Self::Emerald => Self::Off,
            Self::Off => Self::Vanilla,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "vanilla" => Self::Vanilla,
            "rainbow" => Self::Rainbow,
            "gold" => Self::Gold,
            "ice" => Self::Ice,
            "crimson" => Self::Crimson,
            "emerald" => Self::Emerald,
            "off" => Self::Off,
            _ => default,
        }
    }
    /// Returns `[r, g, b, tint_mode]` where `tint_mode > 0.5` enables recoloring
    /// by the glint texture's luminance, and `tint_mode == 0.0` keeps Vanilla purple.
    pub fn rgba_factor(self, time_secs: f32) -> [f32; 4] {
        match self {
            Self::Vanilla => [1.0, 1.0, 1.0, 0.0],
            Self::Off => [0.0, 0.0, 0.0, 1.0],
            Self::Gold => [1.0, 0.84, 0.22, 1.0],
            Self::Ice => [0.35, 0.86, 1.0, 1.0],
            Self::Crimson => [1.0, 0.24, 0.30, 1.0],
            Self::Emerald => [0.24, 0.96, 0.48, 1.0],
            Self::Rainbow => {
                let h = (time_secs * 0.45).fract().clamp(0.0, 0.9999) * 6.0;
                let sector = h.floor() as i32;
                let f = h - sector as f32;
                let s = 0.88_f32;
                let p = 1.0 - s;
                let q = 1.0 - s * f;
                let t = 1.0 - s * (1.0 - f);
                let (r, g, b) = match sector {
                    0 => (1.0, t, p),
                    1 => (q, 1.0, p),
                    2 => (p, 1.0, t),
                    3 => (p, q, 1.0),
                    4 => (t, p, 1.0),
                    _ => (1.0, p, q),
                };
                [r, g, b, 1.0]
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelapsePathMode {
    Orbit,
    DollyForward,
    PanSide,
    StaticTripod,
}
impl TimelapsePathMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Orbit => "orbit",
            Self::DollyForward => "dolly",
            Self::PanSide => "pan",
            Self::StaticTripod => "static",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Orbit => "360° Orbit",
            Self::DollyForward => "Slow Dolly",
            Self::PanSide => "Lateral Pan",
            Self::StaticTripod => "Static Tripod",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Orbit => Self::DollyForward,
            Self::DollyForward => Self::PanSide,
            Self::PanSide => Self::StaticTripod,
            Self::StaticTripod => Self::Orbit,
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        match s {
            "orbit" => Self::Orbit,
            "dolly" => Self::DollyForward,
            "pan" => Self::PanSide,
            "static" => Self::StaticTripod,
            _ => default,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppleSkinConfig {
    pub enabled: bool,
    pub show_saturation_overlay: bool,
    pub show_exhaustion_underlay: bool,
    pub show_held_food_preview: bool,
    pub show_health_regen_overlay: bool,
    pub show_food_tooltip: bool,
    pub always_show_tooltip: bool,
    pub show_debug_stats: bool,
    pub highlight_full_bar: bool,
    pub pulse_speed_pct: u8,
}
impl Default for AppleSkinConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_saturation_overlay: true,
            show_exhaustion_underlay: false,
            show_held_food_preview: true,
            show_health_regen_overlay: true,
            show_food_tooltip: true,
            always_show_tooltip: false,
            show_debug_stats: false,
            highlight_full_bar: false,
            pulse_speed_pct: 100,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrystalOptimizerConfig {
    pub enabled: bool,
    pub instant_break: bool,
    pub respect_weakness: bool,
    pub fast_place: bool,
    pub timeout_ms: u16,
}
impl Default for CrystalOptimizerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            instant_break: true,
            respect_weakness: true,
            fast_place: true,
            timeout_ms: 500,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShieldStatusConfig {
    pub enabled: bool,
    pub enabled_color: ShieldColorPreset,
    pub disabled_color: ShieldColorPreset,
    pub interpolate_colors: bool,
    pub display_mode: ShieldDisplayMode,
    pub disable_sound: bool,
}
impl Default for ShieldStatusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            enabled_color: ShieldColorPreset::Green,
            disabled_color: ShieldColorPreset::Red,
            interpolate_colors: true,
            display_mode: ShieldDisplayMode::Both,
            disable_sound: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UkuHudConfig {
    pub enabled: bool,
    pub armor_enabled: bool,
    pub potion_enabled: bool,
    pub armor_anchor: UkuAnchor,
    pub armor_side: UkuSide,
    pub box_style: UkuStyle,
    pub durability_display: UkuDurability,
    pub show_offhand: bool,
    pub hide_empty: bool,
    pub low_durability_pct: u8,
    pub potion_anchor: UkuPotionAnchor,
    pub potion_compact: bool,
}
impl Default for UkuHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            armor_enabled: true,
            potion_enabled: true,
            armor_anchor: UkuAnchor::Hotbar,
            armor_side: UkuSide::Right,
            box_style: UkuStyle::Hotbar,
            durability_display: UkuDurability::Bar,
            show_offhand: true,
            hide_empty: true,
            low_durability_pct: 15,
            potion_anchor: UkuPotionAnchor::TopRight,
            potion_compact: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreelookConfig {
    pub enabled: bool,
    pub active: bool,
    pub toggle_mode: bool,
    pub invert_y: bool,
    pub sensitivity_pct: u8,
    pub preserve_perspective: bool,
}
impl Default for FreelookConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
            toggle_mode: true,
            invert_y: false,
            sensitivity_pct: 100,
            preserve_perspective: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrajectoriesConfig {
    pub enabled: bool,
    pub show_bow: bool,
    pub show_pearl: bool,
    pub show_trident: bool,
    pub show_wind_charge: bool,
    pub show_potion: bool,
    pub show_impact_box: bool,
    pub color: ShieldColorPreset,
    pub line_width: u8,
}
impl Default for TrajectoriesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_bow: true,
            show_pearl: true,
            show_trident: true,
            show_wind_charge: true,
            show_potion: true,
            show_impact_box: true,
            color: ShieldColorPreset::Cyan,
            line_width: 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthIndicatorsConfig {
    pub enabled: bool,
    pub show_bars: bool,
    pub show_damage_popups: bool,
    pub highlight_crits: bool,
    pub show_players: bool,
    pub show_hostiles: bool,
    pub max_range: u8,
    pub bar_scale: u8,
}
impl Default for HealthIndicatorsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_bars: true,
            show_damage_popups: true,
            highlight_crits: true,
            show_players: true,
            show_hostiles: true,
            max_range: 24,
            bar_scale: 100,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemScrollerConfig {
    pub enabled: bool,
    pub auto_refill_hotbar: bool,
    pub quick_offhand_totem: bool,
    pub sort_shortcut: bool,
    pub scroll_one_by_one: bool,
    pub preserve_tools: bool,
}
impl Default for ItemScrollerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_refill_hotbar: true,
            quick_offhand_totem: true,
            sort_shortcut: true,
            scroll_one_by_one: true,
            preserve_tools: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TweakerooConfig {
    pub enabled: bool,
    pub fast_block_place: bool,
    pub light_overlay: bool,
    pub auto_tool: bool,
    pub silk_touch_guard: bool,
    pub no_portal_gui_close: bool,
    pub light_range: u8,
}
impl Default for TweakerooConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fast_block_place: true,
            light_overlay: false,
            auto_tool: true,
            silk_touch_guard: true,
            no_portal_gui_close: true,
            light_range: 12,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElytraMaceConfig {
    pub enabled: bool,
    pub show_altitude: bool,
    pub show_vertical_speed: bool,
    pub show_pitch_gauge: bool,
    pub show_fireworks: bool,
    pub chestplate_warn_pct: u8,
    pub show_mace_damage: bool,
}
impl Default for ElytraMaceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_altitude: true,
            show_vertical_speed: true,
            show_pitch_gauge: true,
            show_fireworks: true,
            chestplate_warn_pct: 20,
            show_mace_damage: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaypointsConfig {
    pub enabled: bool,
    pub auto_death_waypoint: bool,
    pub show_beacons: bool,
    pub show_hud_distances: bool,
    pub max_distance: u16,
    pub beam_height: u8,
}
impl Default for WaypointsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_death_waypoint: true,
            show_beacons: true,
            show_hud_distances: true,
            max_distance: 2000,
            beam_height: 48,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotemCounterConfig {
    pub enabled: bool,
    pub show_hud_badge: bool,
    pub low_hp_alert: bool,
    pub low_hp_hearts: u8,
    pub warn_when_empty: bool,
    pub corner: Corner,
}
impl Default for TotemCounterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_hud_badge: true,
            low_hp_alert: true,
            low_hp_hearts: 6,
            warn_when_empty: true,
            corner: Corner::TopLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpsComboConfig {
    pub enabled: bool,
    pub show_lmb_cps: bool,
    pub show_rmb_cps: bool,
    pub show_reach: bool,
    pub show_combo: bool,
    pub corner: Corner,
    pub compact: bool,
}
impl Default for CpsComboConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_lmb_cps: true,
            show_rmb_cps: true,
            show_reach: true,
            show_combo: true,
            corner: Corner::TopLeft,
            compact: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeystrokesConfig {
    pub enabled: bool,
    pub show_cps: bool,
    pub show_space: bool,
    pub show_sprint_sneak: bool,
    pub corner: Corner,
    pub chroma: bool,
    pub bg_opacity_pct: u8,
}
impl Default for KeystrokesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_cps: true,
            show_space: true,
            show_sprint_sneak: true,
            corner: Corner::TopLeft,
            chroma: false,
            bg_opacity_pct: 70,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PvpVisualsConfig {
    pub enabled: bool,
    pub low_fire_pct: u8,
    pub hide_pumpkin: bool,
    pub hide_portal_overlay: bool,
    pub hurt_cam_pct: u8,
    pub reduced_bobbing: bool,
    pub glint_pct: u8,
}
impl Default for PvpVisualsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            low_fire_pct: 35,
            hide_pumpkin: true,
            hide_portal_overlay: true,
            hurt_cam_pct: 25,
            reduced_bobbing: true,
            glint_pct: 80,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrosshairTweaksConfig {
    pub enabled: bool,
    pub hit_marker: bool,
    pub style: CrosshairStyle,
    pub color: ShieldColorPreset,
    pub size: u8,
    pub thickness: u8,
    pub dynamic_gap: bool,
}
impl Default for CrosshairTweaksConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hit_marker: true,
            style: CrosshairStyle::Cross,
            color: ShieldColorPreset::White,
            size: 6,
            thickness: 2,
            dynamic_gap: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoomModConfig {
    pub enabled: bool,
    pub active: bool,
    pub zoom_level: u8,
    pub smooth: bool,
    pub scale_sensitivity: bool,
    pub hide_hand: bool,
}
impl Default for ZoomModConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
            zoom_level: 4,
            smooth: true,
            scale_sensitivity: true,
            hide_hand: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoSprintConfig {
    pub enabled: bool,
    pub show_indicator: bool,
    pub toggle_sneak: bool,
    pub sprint_in_water: bool,
    pub corner: Corner,
}
impl Default for AutoSprintConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_indicator: true,
            toggle_sneak: false,
            sprint_in_water: true,
            corner: Corner::TopLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullbrightConfig {
    pub enabled: bool,
    pub gamma_pct: u16,
    pub caves_only: bool,
    pub smooth_fade: bool,
}
impl Default for FullbrightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            gamma_pct: 1200,
            caves_only: false,
            smooth_fade: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoordsHudConfig {
    pub enabled: bool,
    pub show_xyz: bool,
    pub show_nether_equiv: bool,
    pub show_facing: bool,
    pub show_biome: bool,
    pub corner: Corner,
    pub compact: bool,
}
impl Default for CoordsHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_xyz: true,
            show_nether_equiv: true,
            show_facing: true,
            show_biome: true,
            corner: Corner::TopLeft,
            compact: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryHudConfig {
    pub enabled: bool,
    pub show_fps: bool,
    pub show_lows: bool,
    pub show_ping: bool,
    pub show_bps: bool,
    pub corner: Corner,
    pub color_coded: bool,
}
impl Default for TelemetryHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_fps: true,
            show_lows: true,
            show_ping: true,
            show_bps: true,
            corner: Corner::TopLeft,
            color_coded: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemCountersConfig {
    pub enabled: bool,
    pub count_arrows: bool,
    pub count_pearls: bool,
    pub count_wind_charges: bool,
    pub count_gapples: bool,
    pub show_zero: bool,
}
impl Default for ItemCountersConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            count_arrows: true,
            count_pearls: true,
            count_wind_charges: true,
            count_gapples: true,
            show_zero: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockDayConfig {
    pub enabled: bool,
    pub show_day: bool,
    pub use_24h: bool,
    pub show_light_level: bool,
    pub show_spawn_safe: bool,
    pub corner: Corner,
}
impl Default for ClockDayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_day: true,
            use_24h: true,
            show_light_level: true,
            show_spawn_safe: true,
            corner: Corner::TopRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkHitboxConfig {
    pub enabled: bool,
    pub chunk_borders: bool,
    pub entity_hitboxes: bool,
    pub hide_in_f1: bool,
    pub chunk_range: u8,
}
impl Default for ChunkHitboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            chunk_borders: true,
            entity_hitboxes: true,
            hide_in_f1: true,
            chunk_range: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatTweaksConfig {
    pub enabled: bool,
    pub timestamps: bool,
    pub use_24h: bool,
    pub compact_duplicates: bool,
    pub keep_history: bool,
}
impl Default for ChatTweaksConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timestamps: true,
            use_24h: true,
            compact_duplicates: true,
            keep_history: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreecamConfig {
    pub enabled: bool,
    pub active: bool,
    pub speed: u8,
    pub hide_hand: bool,
    pub show_player_marker: bool,
    pub noclip_flight: bool,
}
impl Default for FreecamConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
            speed: 2,
            hide_hand: true,
            show_player_marker: true,
            noclip_flight: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadarConfig {
    pub enabled: bool,
    pub highlight_items: bool,
    pub highlight_chests: bool,
    pub highlight_spawners: bool,
    pub highlight_pets: bool,
    pub range: u8,
    pub show_distances: bool,
}
impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            highlight_items: true,
            highlight_chests: true,
            highlight_spawners: true,
            highlight_pets: true,
            range: 24,
            show_distances: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoReconnectConfig {
    pub enabled: bool,
    pub delay_secs: u8,
    pub max_attempts: u8,
    pub show_server_health: bool,
}
impl Default for AutoReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            delay_secs: 5,
            max_attempts: 5,
            show_server_health: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMacrosConfig {
    pub enabled: bool,
    pub auto_gg: bool,
    pub gg_message_idx: u8,
    pub show_quick_bar: bool,
    pub macro_preset_idx: u8,
}
impl Default for ChatMacrosConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_gg: true,
            gg_message_idx: 0,
            show_quick_bar: true,
            macro_preset_idx: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TntTimerConfig {
    pub enabled: bool,
    pub show_tnt_fuse: bool,
    pub show_creeper_fuse: bool,
    pub danger_halo: bool,
    pub crystal_ring: bool,
    pub anchor_ring: bool,
}
impl Default for TntTimerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_tnt_fuse: true,
            show_creeper_fuse: true,
            danger_halo: true,
            crystal_ring: true,
            anchor_ring: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoArmorConfig {
    pub enabled: bool,
    pub auto_on_pickup: bool,
    pub prefer_protection: bool,
    pub respect_elytra: bool,
    pub min_durability_pct: u8,
}
impl Default for AutoArmorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_on_pickup: true,
            prefer_protection: true,
            respect_elytra: true,
            min_durability_pct: 5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElytraSwapConfig {
    pub enabled: bool,
    pub double_jump_swap: bool,
    pub swap_back_on_land: bool,
    pub prefer_netherite: bool,
}
impl Default for ElytraSwapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            double_jump_swap: true,
            swap_back_on_land: false,
            prefer_netherite: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlimeMobcapConfig {
    pub enabled: bool,
    pub show_slime_chunk: bool,
    pub show_mob_count: bool,
    pub show_spawn_check: bool,
    pub corner: Corner,
}
impl Default for SlimeMobcapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_slime_chunk: true,
            show_mob_count: true,
            show_spawn_check: true,
            corner: Corner::TopRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoEatConfig {
    pub enabled: bool,
    pub hunger_threshold: u8,
    pub hp_threshold: u8,
    pub avoid_rotten_raw: bool,
    pub low_hunger_alert: bool,
}
impl Default for AutoEatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hunger_threshold: 14,
            hp_threshold: 14,
            avoid_rotten_raw: true,
            low_hunger_alert: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadialMenuConfig {
    pub enabled: bool,
    pub show_quick_strip: bool,
    pub include_pearl: bool,
    pub include_gapple: bool,
    pub include_firework: bool,
    pub include_totem: bool,
}
impl Default for RadialMenuConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_quick_strip: true,
            include_pearl: true,
            include_gapple: true,
            include_firework: true,
            include_totem: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClearSkiesConfig {
    pub enabled: bool,
    pub hide_rain_snow: bool,
    pub clear_water_fog: bool,
    pub clear_nether_fog: bool,
    pub clear_terrain_fog: bool,
    pub time_override: TimeMood,
}
impl Default for ClearSkiesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hide_rain_snow: true,
            clear_water_fog: true,
            clear_nether_fog: true,
            clear_terrain_fog: false,
            time_override: TimeMood::Default,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmartTradeConfig {
    pub enabled: bool,
    pub highlight_max_enchants: bool,
    pub highlight_cheap_trades: bool,
    pub cheap_emerald_max: u8,
    pub show_discount_badge: bool,
}
impl Default for SmartTradeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            highlight_max_enchants: true,
            highlight_cheap_trades: true,
            cheap_emerald_max: 16,
            show_discount_badge: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotemPopCounterConfig {
    pub enabled: bool,
    pub track_self: bool,
    pub track_enemies: bool,
    pub play_alert_sound: bool,
    pub show_chat_feed: bool,
    pub corner: Corner,
}
impl Default for TotemPopCounterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            track_self: true,
            track_enemies: true,
            play_alert_sound: true,
            show_chat_feed: true,
            corner: Corner::TopRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoRespawnConfig {
    pub enabled: bool,
    pub instant_respawn: bool,
    pub log_death_coords: bool,
    pub save_death_waypoint: bool,
}
impl Default for AutoRespawnConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            instant_respawn: true,
            log_death_coords: true,
            save_death_waypoint: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShulkerPeekConfig {
    pub enabled: bool,
    pub peek_shulkers: bool,
    pub peek_bundles: bool,
    pub color_by_shulker: bool,
    pub always_show: bool,
}
impl Default for ShulkerPeekConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            peek_shulkers: true,
            peek_bundles: true,
            color_by_shulker: true,
            always_show: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundDropsConfig {
    pub enabled: bool,
    pub rare_item_beams: bool,
    pub merge_count_labels: bool,
    pub highlight_totems: bool,
    pub range: u8,
}
impl Default for GroundDropsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rare_item_beams: true,
            merge_count_labels: true,
            highlight_totems: true,
            range: 24,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityCullingConfig {
    pub enabled: bool,
    pub cull_distance: u8,
    pub reduce_explosions: bool,
    pub reduce_smoke: bool,
    pub hide_potion_swirls: bool,
    pub skip_back_entities: bool,
}
impl Default for EntityCullingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cull_distance: 48,
            reduce_explosions: true,
            reduce_smoke: true,
            hide_potion_swirls: false,
            skip_back_entities: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoFishConfig {
    pub enabled: bool,
    pub visual_bite_alert: bool,
    pub audio_chime: bool,
    pub auto_reel: bool,
    pub auto_recast: bool,
    pub recast_delay_ticks: u8,
}
impl Default for AutoFishConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            visual_bite_alert: true,
            audio_chime: true,
            auto_reel: true,
            auto_recast: true,
            recast_delay_ticks: 10,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookInspectorConfig {
    pub enabled: bool,
    pub show_blocks: bool,
    pub show_crops_pct: bool,
    pub show_mob_health: bool,
    pub show_container_info: bool,
    pub compact: bool,
    pub show_3d_icon: bool,
    pub show_tool_hint: bool,
    pub show_mod_source: bool,
    pub y_offset: u8,
}
impl Default for LookInspectorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_blocks: true,
            show_crops_pct: true,
            show_mob_health: true,
            show_container_info: true,
            compact: false,
            show_3d_icon: true,
            show_tool_hint: true,
            show_mod_source: true,
            y_offset: 36,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoundMufflerConfig {
    pub enabled: bool,
    pub mute_portals: bool,
    pub mute_ghasts: bool,
    pub mute_weather: bool,
    pub mute_pistons: bool,
    pub mute_minecarts: bool,
    pub mute_villagers: bool,
    pub muffle_pct: u8,
}
impl Default for SoundMufflerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mute_portals: true,
            mute_ghasts: true,
            mute_weather: true,
            mute_pistons: false,
            mute_minecarts: false,
            mute_villagers: false,
            muffle_pct: 90,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortalCalcConfig {
    pub enabled: bool,
    pub show_hud_pill: bool,
    pub show_link_radius: bool,
    pub corner: Corner,
}
impl Default for PortalCalcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_hud_pill: true,
            show_link_radius: true,
            corner: Corner::BottomRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompassHudConfig {
    pub enabled: bool,
    pub show_cardinals: bool,
    pub show_waypoints: bool,
    pub show_death_marker: bool,
    pub show_players: bool,
    pub width_pct: u8,
}
impl Default for CompassHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_cardinals: true,
            show_waypoints: true,
            show_death_marker: true,
            show_players: true,
            width_pct: 60,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeWalkConfig {
    pub enabled: bool,
    pub edge_stop: bool,
    pub only_with_blocks: bool,
    pub anti_afk: bool,
    pub afk_interval_secs: u8,
}
impl Default for SafeWalkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            edge_stop: true,
            only_with_blocks: false,
            anti_afk: false,
            afk_interval_secs: 45,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RespawnStatusConfig {
    pub enabled: bool,
    pub show_spawn_coords: bool,
    pub warn_obstructed: bool,
    pub show_anchor_charges: bool,
    pub corner: Corner,
}
impl Default for RespawnStatusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_spawn_coords: true,
            warn_obstructed: true,
            show_anchor_charges: true,
            corner: Corner::BottomLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreboardTweaksConfig {
    pub enabled: bool,
    pub hide_numbers: bool,
    pub hide_scoreboard: bool,
    pub compact_bossbars: bool,
    pub hide_bossbars: bool,
    pub scale_pct: u8,
}
impl Default for ScoreboardTweaksConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hide_numbers: true,
            hide_scoreboard: false,
            compact_bossbars: true,
            hide_bossbars: false,
            scale_pct: 90,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignStandConfig {
    pub enabled: bool,
    pub auto_copy_last_sign: bool,
    pub quick_paste_button: bool,
    pub armor_stand_presets: bool,
}
impl Default for SignStandConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_copy_last_sign: true,
            quick_paste_button: true,
            armor_stand_presets: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicLightsModConfig {
    pub enabled: bool,
    pub held_items: bool,
    pub dropped_items: bool,
    pub burning_entities: bool,
    pub min_level: u8,
}
impl Default for DynamicLightsModConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            held_items: true,
            dropped_items: true,
            burning_entities: true,
            min_level: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LowShieldConfig {
    pub enabled: bool,
    pub shield_drop_pct: u8,
    pub offhand_drop_pct: u8,
    pub mainhand_drop_pct: u8,
    pub hide_blocking_tilt: bool,
}
impl Default for LowShieldConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            shield_drop_pct: 45,
            offhand_drop_pct: 25,
            mainhand_drop_pct: 0,
            hide_blocking_tilt: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanViewConfig {
    pub enabled: bool,
    pub hide_self_potions: bool,
    pub hide_ambient_beacons: bool,
    pub keep_enemy_particles: bool,
}
impl Default for CleanViewConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hide_self_potions: true,
            hide_ambient_beacons: true,
            keep_enemy_particles: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameTagsPlusConfig {
    pub enabled: bool,
    pub show_own_nametag: bool,
    pub show_distance: bool,
    pub show_hp_tag: bool,
    pub no_sneak_fade: bool,
}
impl Default for NameTagsPlusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_own_nametag: true,
            show_distance: true,
            show_hp_tag: true,
            no_sneak_fade: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HitColorConfig {
    pub enabled: bool,
    pub color: ShieldColorPreset,
    pub intensity_pct: u8,
    pub flash_armor: bool,
}
impl Default for HitColorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color: ShieldColorPreset::White,
            intensity_pct: 70,
            flash_armor: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CritRingConfig {
    pub enabled: bool,
    pub show_crit_badge: bool,
    pub show_cooldown_bar: bool,
    pub crit_color: ShieldColorPreset,
}
impl Default for CritRingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_crit_badge: true,
            show_cooldown_bar: true,
            crit_color: ShieldColorPreset::Orange,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BowChargeConfig {
    pub enabled: bool,
    pub show_pct_bar: bool,
    pub full_draw_chime: bool,
    pub auto_release_full: bool,
    pub corner: Corner,
}
impl Default for BowChargeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_pct_bar: true,
            full_draw_chime: true,
            auto_release_full: false,
            corner: Corner::BottomRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CooldownHudConfig {
    pub enabled: bool,
    pub track_pearls: bool,
    pub track_wind_charge: bool,
    pub track_chorus_gapple: bool,
    pub corner: Corner,
}
impl Default for CooldownHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            track_pearls: true,
            track_wind_charge: true,
            track_chorus_gapple: true,
            corner: Corner::TopLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReachBoxConfig {
    pub enabled: bool,
    pub draw_target_box: bool,
    pub draw_reach_ring: bool,
    pub color_on_crit: bool,
    pub color: ShieldColorPreset,
}
impl Default for ReachBoxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            draw_target_box: true,
            draw_reach_ring: true,
            color_on_crit: true,
            color: ShieldColorPreset::Cyan,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PotionHudConfig {
    pub enabled: bool,
    pub show_timers: bool,
    pub blink_under_10s: bool,
    pub compact_pills: bool,
    pub corner: Corner,
}
impl Default for PotionHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_timers: true,
            blink_under_10s: true,
            compact_pills: true,
            corner: Corner::TopLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DurabilityWarnConfig {
    pub enabled: bool,
    pub warn_pct: u8,
    pub check_armor: bool,
    pub check_held: bool,
    pub play_chime: bool,
}
impl Default for DurabilityWarnConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            warn_pct: 15,
            check_armor: true,
            check_held: true,
            play_chime: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BiomeTitleConfig {
    pub enabled: bool,
    pub show_biome_changes: bool,
    pub show_dimension_changes: bool,
    pub display_secs: u8,
    pub show_sub_coords: bool,
}
impl Default for BiomeTitleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_biome_changes: true,
            show_dimension_changes: true,
            display_secs: 3,
            show_sub_coords: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XpTrackerConfig {
    pub enabled: bool,
    pub show_next_level_xp: bool,
    pub show_mending_summary: bool,
    pub corner: Corner,
}
impl Default for XpTrackerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_next_level_xp: true,
            show_mending_summary: true,
            corner: Corner::BottomLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryHudConfig {
    pub enabled: bool,
    pub show_3x9_grid: bool,
    pub show_free_slots: bool,
    pub bg_opacity_pct: u8,
    pub corner: Corner,
}
impl Default for InventoryHudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_3x9_grid: true,
            show_free_slots: true,
            bg_opacity_pct: 65,
            corner: Corner::BottomRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabPingConfig {
    pub enabled: bool,
    pub show_ping_ms: bool,
    pub show_player_count: bool,
    pub show_dim_tag: bool,
    pub corner: Corner,
}
impl Default for TabPingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_ping_ms: true,
            show_player_count: true,
            show_dim_tag: true,
            corner: Corner::TopRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoonPhaseConfig {
    pub enabled: bool,
    pub show_moon_phase: bool,
    pub show_slime_rating: bool,
    pub show_insomnia_warn: bool,
    pub corner: Corner,
}
impl Default for MoonPhaseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_moon_phase: true,
            show_slime_rating: true,
            show_insomnia_warn: true,
            corner: Corner::TopLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStatsConfig {
    pub enabled: bool,
    pub show_kills: bool,
    pub show_blocks: bool,
    pub show_distance: bool,
    pub corner: Corner,
}
impl Default for SessionStatsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_kills: true,
            show_blocks: true,
            show_distance: true,
            corner: Corner::BottomLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeatherSoundConfig {
    pub enabled: bool,
    pub suppress_lightning_flash: bool,
    pub limit_rain_splashes: bool,
    pub mute_thunder_shake: bool,
}
impl Default for WeatherSoundConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            suppress_lightning_flash: true,
            limit_rain_splashes: true,
            mute_thunder_shake: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomGlintConfig {
    pub enabled: bool,
    pub preset: GlintColorPreset,
    pub speed_pct: u8,
    pub strength_pct: u8,
    pub compact_item_scale: bool,
}
impl Default for CustomGlintConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: GlintColorPreset::Gold,
            speed_pct: 100,
            strength_pct: 90,
            compact_item_scale: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClearCavesConfig {
    pub enabled: bool,
    pub remove_void_fog: bool,
    pub smooth_night_vision: bool,
    pub boost_lava_clarity: bool,
}
impl Default for ClearCavesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            remove_void_fog: true,
            smooth_night_vision: true,
            boost_lava_clarity: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerHighlightConfig {
    pub enabled: bool,
    pub highlight_players: bool,
    pub highlight_hostiles: bool,
    pub range: u8,
    pub color: ShieldColorPreset,
}
impl Default for PlayerHighlightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            highlight_players: true,
            highlight_hostiles: false,
            range: 32,
            color: ShieldColorPreset::Cyan,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LitematicaGridConfig {
    pub enabled: bool,
    pub show_volume_hud: bool,
    pub show_3d_box: bool,
    pub show_grid_lines: bool,
    pub color: ShieldColorPreset,
}
impl Default for LitematicaGridConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_volume_hud: true,
            show_3d_box: true,
            show_grid_lines: true,
            color: ShieldColorPreset::Cyan,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeathHistoryConfig {
    pub enabled: bool,
    pub keep_count: u8,
    pub show_home_pill: bool,
    pub auto_log_chat: bool,
    pub corner: Corner,
}
impl Default for DeathHistoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            keep_count: 5,
            show_home_pill: true,
            auto_log_chat: true,
            corner: Corner::TopLeft,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkPillarsConfig {
    pub enabled: bool,
    pub draw_corner_pillars: bool,
    pub draw_biome_borders: bool,
    pub pillar_height: u8,
}
impl Default for ChunkPillarsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            draw_corner_pillars: true,
            draw_biome_borders: true,
            pillar_height: 64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffhandTorchConfig {
    pub enabled: bool,
    pub auto_place_dark: bool,
    pub light_threshold: u8,
    pub notify_low_torches: bool,
}
impl Default for OffhandTorchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_place_dark: false,
            light_threshold: 4,
            notify_low_torches: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMentionConfig {
    pub enabled: bool,
    pub alert_on_username: bool,
    pub alert_on_tpa_msg: bool,
    pub play_ping_chime: bool,
    pub show_popup_banner: bool,
}
impl Default for ChatMentionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            alert_on_username: true,
            alert_on_tpa_msg: true,
            play_ping_chime: true,
            show_popup_banner: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropGuardConfig {
    pub enabled: bool,
    pub protect_tools_weapons: bool,
    pub protect_totems: bool,
    pub lock_slots_1_to_3: bool,
    pub double_tap_bypass: bool,
}
impl Default for DropGuardConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protect_tools_weapons: true,
            protect_totems: true,
            lock_slots_1_to_3: true,
            double_tap_bypass: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotbarRestockConfig {
    pub enabled: bool,
    pub restock_blocks: bool,
    pub restock_pearls_fireworks: bool,
    pub restock_food: bool,
    pub show_toast: bool,
}
impl Default for HotbarRestockConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            restock_blocks: true,
            restock_pearls_fireworks: true,
            restock_food: true,
            show_toast: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhotoModeConfig {
    pub enabled: bool,
    pub active: bool,
    pub hide_all_hud: bool,
    pub lock_freecam_pose: bool,
    pub show_frame_guides: bool,
}
impl Default for PhotoModeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
            hide_all_hud: true,
            lock_freecam_pose: false,
            show_frame_guides: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JeiConfig {
    pub enabled: bool,
    pub overlay_open: bool,
    pub show_in_inventory: bool,
    pub show_item_ids: bool,
    pub show_cooking_recipes: bool,
    pub compact_grid: bool,
}
impl Default for JeiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            overlay_open: false,
            show_in_inventory: true,
            show_item_ids: false,
            show_cooking_recipes: true,
            compact_grid: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CinematicTimelapseConfig {
    pub enabled: bool,
    pub start_delay_secs: u8,
    pub path_mode: TimelapsePathMode,
    pub orbit_speed_deg: u8,
    pub hide_hud: bool,
    pub show_countdown: bool,
}
impl Default for CinematicTimelapseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_delay_secs: 5,
            path_mode: TimelapsePathMode::Orbit,
            orbit_speed_deg: 5,
            hide_hud: true,
            show_countdown: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlocksOutlineConfig {
    pub enabled: bool,
    /// Outline RGB color (`0x00RRGGBB`).
    pub color_rgb: u32,
    /// Outline opacity percentage (`10..=100`).
    pub alpha_pct: u8,
    /// Line thickness in pixels (`1..=10`).
    pub line_width: u8,
    /// Smooth rainbow chroma cycling.
    pub rainbow: bool,
    /// Rainbow cycle speed (`1..=5`).
    pub rainbow_speed: u8,
    /// Breathing alpha/width pulse animation.
    pub pulse: bool,
    /// Translucent filled face overlay inside the targeted block box.
    pub fill_overlay: bool,
    /// Fill overlay opacity percentage (`5..=80`).
    pub fill_alpha_pct: u8,
    /// Whether the fill overlay uses the outline color or its own custom RGB.
    pub fill_match_outline: bool,
    /// Custom fill overlay RGB color (`0x00RRGGBB`) when `fill_match_outline` is false.
    pub fill_color_rgb: u32,
    /// Render the block outline through solid blocks (depth-less / always visible).
    pub see_through: bool,
    /// Draw a thick dark backing stroke behind the colored outline for contrast.
    pub high_contrast_backing: bool,
    /// Slight box inflation (`0..=6`, in thousandths of a block) to prevent Z-fighting.
    pub inflate_pct: u8,
}
impl Default for BlocksOutlineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color_rgb: 0x00E5FF,
            alpha_pct: 100,
            line_width: 3,
            rainbow: false,
            rainbow_speed: 2,
            pulse: false,
            fill_overlay: false,
            fill_alpha_pct: 22,
            fill_match_outline: true,
            fill_color_rgb: 0x00E5FF,
            see_through: false,
            high_contrast_backing: false,
            inflate_pct: 1,
        }
    }
}
impl BlocksOutlineConfig {
    pub fn rgb_bytes(rgb: u32) -> [u8; 3] {
        [
            ((rgb >> 16) & 0xFF) as u8,
            ((rgb >> 8) & 0xFF) as u8,
            (rgb & 0xFF) as u8,
        ]
    }
    pub fn from_rgb(r: u8, g: u8, b: u8) -> u32 {
        (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
    }
    pub fn hsv_to_rgb(h_deg: f32, s: f32, v: f32) -> u32 {
        let h = (h_deg.rem_euclid(360.0)) / 60.0;
        let c = v * s;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let m = v - c;
        let (r1, g1, b1) = match h as u8 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let r = ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        let g = ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        let b = ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        Self::from_rgb(r, g, b)
    }
    pub fn active_rgb(&self, time_secs: f32) -> u32 {
        if self.rainbow {
            let speed = f32::from(self.rainbow_speed.clamp(1, 5)) * 65.0;
            Self::hsv_to_rgb(time_secs * speed, 0.92, 1.0)
        } else {
            self.color_rgb & 0x00FF_FFFF
        }
    }
    pub fn active_fill_rgb(&self, time_secs: f32) -> u32 {
        if self.fill_match_outline {
            self.active_rgb(time_secs)
        } else {
            self.fill_color_rgb & 0x00FF_FFFF
        }
    }
    pub fn to_argb(rgb: u32, alpha: u8) -> u32 {
        (u32::from(alpha) << 24) | (rgb & 0x00FF_FFFF)
    }
    pub fn to_abgr(rgb: u32, alpha: u8) -> u32 {
        let [r, g, b] = Self::rgb_bytes(rgb);
        (u32::from(alpha) << 24) | (u32::from(b) << 16) | (u32::from(g) << 8) | u32::from(r)
    }
    pub fn hex_string(rgb: u32) -> String {
        format!("#{:06X}", rgb & 0x00FF_FFFF)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoltExtras {
    pub appleskin: AppleSkinConfig,
    pub crystal_optimizer: CrystalOptimizerConfig,
    pub shield_status: ShieldStatusConfig,
    pub uku_hud: UkuHudConfig,
    pub freelook: FreelookConfig,
    pub trajectories: TrajectoriesConfig,
    pub health_indicators: HealthIndicatorsConfig,
    pub item_scroller: ItemScrollerConfig,
    pub tweakeroo: TweakerooConfig,
    pub elytra_hud: ElytraMaceConfig,
    pub waypoints: WaypointsConfig,
    pub totem_counter: TotemCounterConfig,
    pub cps_combo: CpsComboConfig,
    pub keystrokes: KeystrokesConfig,
    pub pvp_visuals: PvpVisualsConfig,
    pub crosshair_tweaks: CrosshairTweaksConfig,
    pub zoom: ZoomModConfig,
    pub auto_sprint: AutoSprintConfig,
    pub fullbright: FullbrightConfig,
    pub coords_hud: CoordsHudConfig,
    pub telemetry_hud: TelemetryHudConfig,
    pub item_counters: ItemCountersConfig,
    pub clock_day_hud: ClockDayConfig,
    pub chunk_hitboxes: ChunkHitboxConfig,
    pub chat_tweaks: ChatTweaksConfig,
    pub freecam: FreecamConfig,
    pub radar: RadarConfig,
    pub auto_reconnect: AutoReconnectConfig,
    pub chat_macros: ChatMacrosConfig,
    pub tnt_timer: TntTimerConfig,
    pub auto_armor: AutoArmorConfig,
    pub elytra_swap: ElytraSwapConfig,
    pub slime_mobcap: SlimeMobcapConfig,
    pub auto_eat: AutoEatConfig,
    pub radial_menu: RadialMenuConfig,
    pub clear_skies: ClearSkiesConfig,
    pub smart_trade: SmartTradeConfig,
    pub totem_pop_counter: TotemPopCounterConfig,
    pub auto_respawn: AutoRespawnConfig,
    pub shulker_peek: ShulkerPeekConfig,
    pub ground_drops: GroundDropsConfig,
    pub entity_culling: EntityCullingConfig,
    pub auto_fish: AutoFishConfig,
    pub look_inspector: LookInspectorConfig,
    pub sound_muffler: SoundMufflerConfig,
    pub portal_calc: PortalCalcConfig,
    pub compass_hud: CompassHudConfig,
    pub safe_walk: SafeWalkConfig,
    pub respawn_status: RespawnStatusConfig,
    pub scoreboard_tweaks: ScoreboardTweaksConfig,
    pub sign_stand_helper: SignStandConfig,
    pub dynamic_lights: DynamicLightsModConfig,
    pub low_shield: LowShieldConfig,
    pub clean_view: CleanViewConfig,
    pub name_tags_plus: NameTagsPlusConfig,
    pub hit_color: HitColorConfig,
    pub crit_ring: CritRingConfig,
    pub bow_charge: BowChargeConfig,
    pub cooldown_hud: CooldownHudConfig,
    pub reach_box: ReachBoxConfig,
    pub potion_hud: PotionHudConfig,
    pub durability_warn: DurabilityWarnConfig,
    pub biome_title: BiomeTitleConfig,
    pub xp_tracker: XpTrackerConfig,
    pub inventory_hud: InventoryHudConfig,
    pub tab_ping: TabPingConfig,
    pub moon_phase: MoonPhaseConfig,
    pub session_stats: SessionStatsConfig,
    pub weather_sound: WeatherSoundConfig,
    pub custom_glint: CustomGlintConfig,
    pub clear_caves: ClearCavesConfig,
    pub player_highlight: PlayerHighlightConfig,
    pub litematica_grid: LitematicaGridConfig,
    pub death_history: DeathHistoryConfig,
    pub chunk_pillars: ChunkPillarsConfig,
    pub offhand_torch: OffhandTorchConfig,
    pub chat_mention: ChatMentionConfig,
    pub drop_guard: DropGuardConfig,
    pub hotbar_restock: HotbarRestockConfig,
    pub photo_mode: PhotoModeConfig,
    pub jei: JeiConfig,
    pub cinematic_timelapse: CinematicTimelapseConfig,
    pub blocks_outline: BlocksOutlineConfig,
    pub binds: VoltModKeybinds,
}

pub const VOLT_MODS: [VoltModMeta; VOLT_MOD_COUNT] = [
    VoltModMeta { id: 0, key: "appleskin", title: "AppleSkin", summary: "Saturation drumstick outlines, exhaustion bar, food & regen preview, tooltips", badge: "AS", category: VoltModCategory::Hud, modrinth_slugs: &["appleskin"], search_query: "AppleSkin" },
    VoltModMeta { id: 1, key: "crystal_optimizer", title: "Marlow's Crystal Optimizer", summary: "Instant client-side crystal clearing & fast crystal/obsidian placement", badge: "CO", category: VoltModCategory::Pvp, modrinth_slugs: &["marlows-crystal-optimizer"], search_query: "Crystal Optimizer" },
    VoltModMeta { id: 2, key: "shield_status", title: "Shield Status", summary: "Live ready/disabled color indicator, cooldown bar & timer", badge: "SS", category: VoltModCategory::Pvp, modrinth_slugs: &["shield-statuses", "shield-status"], search_query: "Shield Statuses" },
    VoltModMeta { id: 3, key: "uku_hud", title: "Uku's Armor & Potion HUD", summary: "Customizable armor durability panel & timed potion cards", badge: "UH", category: VoltModCategory::Hud, modrinth_slugs: &["ukus-armor-hud"], search_query: "uku armor hud" },
    VoltModMeta { id: 4, key: "freelook", title: "Better Third-Person / Freelook", summary: "Orbit the camera 360° without changing movement or aim direction", badge: "FL", category: VoltModCategory::Camera, modrinth_slugs: &["freelook", "better-third-person", "camerautilities"], search_query: "Freelook" },
    VoltModMeta { id: 5, key: "trajectories", title: "Projectile Trajectories", summary: "Live landing arc & impact box for Bow, Pearl, Trident, Potion & Wind Charge", badge: "TJ", category: VoltModCategory::Pvp, modrinth_slugs: &["trajectories", "throwable-preview"], search_query: "trajectories" },
    VoltModMeta { id: 6, key: "health_indicators", title: "Health & Damage Popups", summary: "Mini HP bars above mobs/players & floating crit damage numbers", badge: "HP", category: VoltModCategory::Pvp, modrinth_slugs: &["health-indicators", "torohealth-damage-indicators", "neat", "boxclient"], search_query: "health indicators" },
    VoltModMeta { id: 7, key: "item_scroller", title: "Item Scroller & Quick Inv", summary: "Quick offhand totem swap, hotbar auto-refill & inventory sorting", badge: "IS", category: VoltModCategory::Utility, modrinth_slugs: &["item-scroller", "inventory-profiles-next"], search_query: "Item Scroller" },
    VoltModMeta { id: 8, key: "tweakeroo", title: "Tweakeroo Building Suite", summary: "Fast block place, light-level X spawn overlay, auto-tool & silk guard", badge: "TW", category: VoltModCategory::World, modrinth_slugs: &["tweakeroo"], search_query: "Tweakeroo" },
    VoltModMeta { id: 9, key: "elytra_hud", title: "Elytra & Mace Helper HUD", summary: "Altitude, vertical speed, pitch gauge, fireworks & Mace smash preview", badge: "EM", category: VoltModCategory::Hud, modrinth_slugs: &["elytra-hud", "do-a-barrel-roll", "mace-combat-pvp-trainer"], search_query: "elytra hud" },
    VoltModMeta { id: 10, key: "waypoints", title: "Waypoints & Death Marker", summary: "Auto death beacon, 3D sky beams & HUD distance tags", badge: "WP", category: VoltModCategory::World, modrinth_slugs: &["xaeros-minimap", "xaeros-world-map", "journeymap"], search_query: "Xaero Minimap" },
    VoltModMeta { id: 11, key: "totem_counter", title: "Totem Counter & Low-HP Alert", summary: "Tracks totems in inventory/offhand with low-HP & empty alerts", badge: "TC", category: VoltModCategory::Pvp, modrinth_slugs: &["totemcounter", "totem-counter"], search_query: "TotemCounter" },
    VoltModMeta { id: 12, key: "cps_combo", title: "CPS / Reach / Combo Counter", summary: "Live LMB/RMB CPS, hit reach distance & active combo streak HUD", badge: "CC", category: VoltModCategory::Pvp, modrinth_slugs: &["voxelclient", "boxclient", "astrion-mod"], search_query: "CPS counter" },
    VoltModMeta { id: 13, key: "keystrokes", title: "Keystrokes & Mouse Overlay", summary: "WASD, Jump, Sprint, LMB & RMB live key press tiles with CPS", badge: "KS", category: VoltModCategory::Hud, modrinth_slugs: &["keystrokes", "voxelclient", "astrion-mod"], search_query: "Keystrokes" },
    VoltModMeta { id: 14, key: "pvp_visuals", title: "Clean PvP Visuals Pack", summary: "Low fire, no pumpkin/portal overlay, reduced hurt-cam & view bobbing", badge: "PV", category: VoltModCategory::Camera, modrinth_slugs: &["betterhurtcam", "low-fire", "no-hurt-cam"], search_query: "BetterHurtCam" },
    VoltModMeta { id: 15, key: "crosshair_tweaks", title: "Crosshair Tweaks & Hit Marker", summary: "Dynamic hit marker flash plus custom crosshair style, size & color", badge: "XH", category: VoltModCategory::Pvp, modrinth_slugs: &["custom-crosshair-mod", "centered-crosshair"], search_query: "Custom Crosshair" },
    VoltModMeta { id: 16, key: "zoom", title: "Zoomify (Smooth Zoom)", summary: "Optifine-style smooth FOV zoom toggle with sensitivity scaling", badge: "ZM", category: VoltModCategory::Camera, modrinth_slugs: &["zoomify", "ok-zoomer"], search_query: "Zoomify" },
    VoltModMeta { id: 17, key: "auto_sprint", title: "Auto-Sprint (Toggle Sprint)", summary: "Keeps sprint active automatically with optional Sprint/Sneak badge", badge: "SP", category: VoltModCategory::Utility, modrinth_slugs: &["togglesprint", "toggle-sprint", "toggle-sneak"], search_query: "ToggleSprint" },
    VoltModMeta { id: 18, key: "fullbright", title: "Fullbright (Gamma Utils)", summary: "Instant client-side brightness boost for dark caves and builds", badge: "FB", category: VoltModCategory::Camera, modrinth_slugs: &["gamma-utils", "fullbright"], search_query: "Gamma Utils" },
    VoltModMeta { id: 19, key: "coords_hud", title: "Coords, Biome & Facing HUD", summary: "Compact XYZ + Nether/Overworld equivalent coords, facing & biome pill", badge: "XY", category: VoltModCategory::Hud, modrinth_slugs: &["betterf3", "coordinates-display"], search_query: "BetterF3" },
    VoltModMeta { id: 20, key: "telemetry_hud", title: "FPS, Ping & Speedometer", summary: "Corner pill with FPS, 1% lows, color-coded ping & BPS speed", badge: "FP", category: VoltModCategory::Hud, modrinth_slugs: &["better-ping-display-fabric", "better-ping-display", "sodium-extra"], search_query: "Better Ping Display" },
    VoltModMeta { id: 21, key: "item_counters", title: "Item / Arrow / Pearl Counter", summary: "Hotbar-side live counts for Arrows, Pearls, Wind Charges & Gapples", badge: "IC", category: VoltModCategory::Pvp, modrinth_slugs: &["inventoryhud", "inventory-hud-plus"], search_query: "Inventory HUD" },
    VoltModMeta { id: 22, key: "clock_day_hud", title: "Clock, Day & Light Level HUD", summary: "In-game day count, 12/24h time & local mob-spawn light safety", badge: "CL", category: VoltModCategory::Hud, modrinth_slugs: &["minihud", "day-counter"], search_query: "MiniHUD" },
    VoltModMeta { id: 23, key: "chunk_hitboxes", title: "Chunk Borders & Hitboxes", summary: "One-tap toggles for chunk borders (F3+G) and entity hitboxes (F3+B)", badge: "CB", category: VoltModCategory::World, modrinth_slugs: &["chunk-borders", "chunkdebug"], search_query: "Chunk Borders" },
    VoltModMeta { id: 24, key: "chat_tweaks", title: "Chat Timestamps & Compact", summary: "[HH:MM] message timestamps & duplicate message collapsing [xN]", badge: "CT", category: VoltModCategory::Utility, modrinth_slugs: &["compact-chat", "chat-heads", "morechathistory"], search_query: "Compact Chat" },
    VoltModMeta { id: 25, key: "freecam", title: "Freecam (Spectator Camera)", summary: "Detach camera from your body to scout caves, builds & redstone", badge: "FC", category: VoltModCategory::Camera, modrinth_slugs: &["freecam", "camerautilities"], search_query: "Freecam" },
    VoltModMeta { id: 26, key: "radar", title: "Entity & Chest Radar", summary: "3D highlight boxes & distances for dropped items, chests & spawners", badge: "RD", category: VoltModCategory::World, modrinth_slugs: &["chest-tracker", "chesttracker"], search_query: "Chest Tracker" },
    VoltModMeta { id: 27, key: "auto_reconnect", title: "Auto-Reconnect & Net Health", summary: "Auto-rejoins on disconnect & displays live server TPS/packet health", badge: "AR", category: VoltModCategory::Utility, modrinth_slugs: &["autoreconnect", "auto-reconnect"], search_query: "AutoReconnect" },
    VoltModMeta { id: 28, key: "chat_macros", title: "Auto-GG & Quick Chat Macros", summary: "Auto GG on match end plus 1-tap /home, /spawn & /tpa quick bar", badge: "GG", category: VoltModCategory::Utility, modrinth_slugs: &["autogg", "auto-gg"], search_query: "AutoGG" },
    VoltModMeta { id: 29, key: "tnt_timer", title: "TNT Timer & Blast Danger Halo", summary: "Fuse countdown over TNT/creepers & lethal crystal/anchor rings", badge: "TN", category: VoltModCategory::Pvp, modrinth_slugs: &["tnt-time", "tnt-timer"], search_query: "TNT Time" },
    VoltModMeta { id: 30, key: "auto_armor", title: "Auto-Armor & Best Gear", summary: "Automatically equips highest-protection armor while respecting Elytra", badge: "AA", category: VoltModCategory::Pvp, modrinth_slugs: &["auto-armor", "inventory-profiles-next"], search_query: "Inventory Profiles Next" },
    VoltModMeta { id: 31, key: "elytra_swap", title: "Chestplate ↔ Elytra Swap", summary: "Single-tap or midair double-jump swap between Chestplate and Elytra", badge: "ES", category: VoltModCategory::Utility, modrinth_slugs: &["elytraswap", "chestplate-elytra-swap", "itemswapper"], search_query: "Elytra Swap" },
    VoltModMeta { id: 32, key: "slime_mobcap", title: "Slime Chunk & Mob-Cap HUD", summary: "Slime chunk indicator, local entity density & spawnable check", badge: "SL", category: VoltModCategory::World, modrinth_slugs: &["minihud", "slime-chunk"], search_query: "MiniHUD" },
    VoltModMeta { id: 33, key: "auto_eat", title: "Auto-Eat & Hunger Guard", summary: "Automatically eats hotbar food below hunger/HP threshold + warning", badge: "AE", category: VoltModCategory::Utility, modrinth_slugs: &["autoeat", "auto-eat"], search_query: "AutoEat" },
    VoltModMeta { id: 34, key: "radial_menu", title: "Quick Action Utility Strip", summary: "One-tap quick strip for Pearl, Gapple, Firework, Totem & Zoom", badge: "QM", category: VoltModCategory::Pvp, modrinth_slugs: &["itemswapper", "mine-menu"], search_query: "ItemSwapper" },
    VoltModMeta { id: 35, key: "clear_skies", title: "Clear Skies (Fog & Weather)", summary: "Hide rain/snow, push back water/nether fog & preview sky mood", badge: "CS", category: VoltModCategory::Camera, modrinth_slugs: &["no-fog", "nofog", "fog-looks-good-now"], search_query: "No Fog" },
    VoltModMeta { id: 36, key: "smart_trade", title: "Villager Enchant Highlighter", summary: "Highlights max-level enchant books & cheap emerald trades in green", badge: "VT", category: VoltModCategory::Utility, modrinth_slugs: &["trade-cycling", "easy-villagers"], search_query: "Trade Cycling" },
    VoltModMeta { id: 37, key: "totem_pop_counter", title: "Enemy & Self Totem Pop Feed", summary: "Tracks totem pops in the current fight with HUD feed & alert sound", badge: "TP", category: VoltModCategory::Pvp, modrinth_slugs: &["totemcounter", "small-totem-pop"], search_query: "Totem" },
    VoltModMeta { id: 38, key: "auto_respawn", title: "Auto-Respawn & Death Coords", summary: "Instant respawn option + logs death coordinates & saves death point", badge: "DR", category: VoltModCategory::Utility, modrinth_slugs: &["auto-respawn", "fastquit"], search_query: "Auto Respawn" },
    VoltModMeta { id: 39, key: "shulker_peek", title: "Shulker & Bundle Peek", summary: "Compact item grid tooltip preview for Shulker Boxes & Bundles", badge: "SH", category: VoltModCategory::Utility, modrinth_slugs: &["shulkerboxtooltip"], search_query: "Shulker Box Tooltip" },
    VoltModMeta { id: 40, key: "ground_drops", title: "Rare Ground Drops & Beams", summary: "Highlight beams for Netherite/Diamonds/Totems & stack count tags", badge: "GD", category: VoltModCategory::World, modrinth_slugs: &["itemphysic", "item-highlighter"], search_query: "Item Highlighter" },
    VoltModMeta { id: 41, key: "entity_culling", title: "Entity Culling & FPS Boost+", summary: "Back-face/distance entity culling & heavy particle limiter", badge: "EC", category: VoltModCategory::World, modrinth_slugs: &["entityculling", "immediatelyfast"], search_query: "Entity Culling" },
    VoltModMeta { id: 42, key: "auto_fish", title: "Auto-Fish & Bite Chime", summary: "Visual FISH BITE alert + chime & optional auto-reel/recast", badge: "AF", category: VoltModCategory::Utility, modrinth_slugs: &["autofish", "auto-fish"], search_query: "Autofish" },
    VoltModMeta { id: 43, key: "look_inspector", title: "Jade-Style Look Inspector", summary: "Top HUD pill showing targeted block, crop %, mob HP or chest info", badge: "JD", category: VoltModCategory::Hud, modrinth_slugs: &["jade", "wthit"], search_query: "Jade" },
    VoltModMeta { id: 44, key: "sound_muffler", title: "Extreme Sound Muffler", summary: "Mute or lower Portals, Ghasts, Rain, Pistons, Minecarts & Villagers", badge: "SM", category: VoltModCategory::Utility, modrinth_slugs: &["extreme-sound-muffler", "sound-physics-remastered"], search_query: "Extreme Sound Muffler" },
    VoltModMeta { id: 45, key: "portal_calc", title: "Nether Portal Calculator", summary: "Live Overworld ↔ Nether (÷8 / ×8) coords & portal link helper", badge: "NP", category: VoltModCategory::World, modrinth_slugs: &["nether-coords", "minihud"], search_query: "Nether Coords" },
    VoltModMeta { id: 46, key: "compass_hud", title: "Compass & Direction Bar", summary: "Top direction bar (N E S W) with waypoints, death & player pips", badge: "CM", category: VoltModCategory::Hud, modrinth_slugs: &["compass-hud", "xaeros-minimap"], search_query: "Compass HUD" },
    VoltModMeta { id: 47, key: "safe_walk", title: "Safe-Walk & Anti-AFK", summary: "Stops you at block ledges when building/bridging + farm Anti-AFK", badge: "SW", category: VoltModCategory::World, modrinth_slugs: &["tweakeroo", "safe-walk"], search_query: "Tweakeroo" },
    VoltModMeta { id: 48, key: "respawn_status", title: "Bed / Anchor Spawn Indicator", summary: "Shows active spawn point coords & warns if bed/anchor uncharged", badge: "RS", category: VoltModCategory::Hud, modrinth_slugs: &["respawn-anchor", "minihud"], search_query: "Respawn" },
    VoltModMeta { id: 49, key: "scoreboard_tweaks", title: "Scoreboard & Bossbar Tweaks", summary: "Hide red scoreboard numbers, compact bossbars & resize sidebar", badge: "SB", category: VoltModCategory::Hud, modrinth_slugs: &["scoreboard-tweaks", "custom-scoreboard"], search_query: "Scoreboard Tweaks" },
    VoltModMeta { id: 50, key: "sign_stand_helper", title: "Quick Sign & Stand Presets", summary: "Auto-copy/paste last sign text & quick armor stand pose presets", badge: "SG", category: VoltModCategory::Utility, modrinth_slugs: &["armor-stand-editor", "sign-copy"], search_query: "Sign" },
    VoltModMeta { id: 51, key: "dynamic_lights", title: "LambDynamicLights", summary: "Real-time held, dropped & burning dynamic light sources in caves", badge: "DL", category: VoltModCategory::Camera, modrinth_slugs: &["lambdynamiclights"], search_query: "LambDynamicLights" },
    VoltModMeta { id: 52, key: "low_shield", title: "Low Shield & Compact Viewmodel", summary: "Lowers 1st-person shield & offhand item height for clear PvP sightlines", badge: "LS", category: VoltModCategory::Pvp, modrinth_slugs: &["low-shield", "compact-viewmodel"], search_query: "Low Shield" },
    VoltModMeta { id: 53, key: "clean_view", title: "Clean View", summary: "Hides your own swirling potion & beacon particles while keeping enemy FX", badge: "CV", category: VoltModCategory::Pvp, modrinth_slugs: &["cleanview", "particles-off"], search_query: "CleanView" },
    VoltModMeta { id: 54, key: "name_tags_plus", title: "Better NameTags+", summary: "Shows own 3rd-person nametag plus distance, HP & ping beside players", badge: "NT", category: VoltModCategory::Pvp, modrinth_slugs: &["better-nametags", "namepain"], search_query: "Better NameTags" },
    VoltModMeta { id: 55, key: "hit_color", title: "Custom Hit Color & Flash", summary: "Customize damage hit flash tint, intensity & armor flash on entities", badge: "HC", category: VoltModCategory::Pvp, modrinth_slugs: &["custom-hit-color", "hit-color"], search_query: "Custom Hit Color" },
    VoltModMeta { id: 56, key: "crit_ring", title: "Crit & Attack Cooldown Ring", summary: "Live crosshair attack cooldown bar & airborne CRIT READY indicator", badge: "CR", category: VoltModCategory::Pvp, modrinth_slugs: &["better-combat", "attack-indicator"], search_query: "Better Combat" },
    VoltModMeta { id: 57, key: "bow_charge", title: "Bow & Crossbow Charge HUD", summary: "Live draw % progress bar, full-charge chime & optional auto-release", badge: "BW", category: VoltModCategory::Pvp, modrinth_slugs: &["tipthescales", "crossbow-indicator"], search_query: "Bow" },
    VoltModMeta { id: 58, key: "cooldown_hud", title: "Pearl & Item Cooldown Pill", summary: "Floating countdown timers for Ender Pearls, Wind Charges & Chorus", badge: "CD", category: VoltModCategory::Pvp, modrinth_slugs: &["cooldown-hud", "ender-pearl-cooldown"], search_query: "Cooldown" },
    VoltModMeta { id: 59, key: "reach_box", title: "PvP Target Reach Box", summary: "Highlights targeted entities inside 3.0m reach & changes color on crit", badge: "RB", category: VoltModCategory::Pvp, modrinth_slugs: &["hitboxes-plus", "accurate-block-placement"], search_query: "Hitboxes" },
    VoltModMeta { id: 60, key: "potion_hud", title: "Compact Status Effects Bar", summary: "Sleek timed status effect pills with MM:SS countdown & expiry blink", badge: "PE", category: VoltModCategory::Hud, modrinth_slugs: &["status-effect-bars", "stylisheffects"], search_query: "Status Effect Bars" },
    VoltModMeta { id: 61, key: "durability_warn", title: "Durability Notifier & Chime", summary: "Audible chime + HUD alert when tool, shield or armor drops below %", badge: "DW", category: VoltModCategory::Hud, modrinth_slugs: &["durability-notifier", "durability-viewer"], search_query: "Durability Notifier" },
    VoltModMeta { id: 62, key: "biome_title", title: "Traveler's Titles (Biome Banner)", summary: "Smooth fade-in/out title banner when entering a new biome or dimension", badge: "BT", category: VoltModCategory::Hud, modrinth_slugs: &["travelers-titles", "biome-title"], search_query: "Traveler's Titles" },
    VoltModMeta { id: 63, key: "xp_tracker", title: "XP & Mending Progress HUD", summary: "Exact XP needed for next level, % progress & damaged Mending gear status", badge: "XP", category: VoltModCategory::Hud, modrinth_slugs: &["xp-bar-text", "more-xp-info"], search_query: "XP" },
    VoltModMeta { id: 64, key: "inventory_hud", title: "Mini Inventory Preview Grid", summary: "Translucent 3x9 mini-grid overlay showing free slots & stack counts", badge: "IH", category: VoltModCategory::Hud, modrinth_slugs: &["inventory-hud-plus", "inventoryhud"], search_query: "Inventory HUD+" },
    VoltModMeta { id: 65, key: "tab_ping", title: "Better Tab & Latency List", summary: "Compact online player count, numeric ping ms & dimension tags", badge: "TB", category: VoltModCategory::Hud, modrinth_slugs: &["better-tab", "tab-ping"], search_query: "Better Tab" },
    VoltModMeta { id: 66, key: "moon_phase", title: "Moon Phase & Insomnia Timer", summary: "Shows current Moon Phase for swamp slimes + Phantom insomnia alert", badge: "MN", category: VoltModCategory::Hud, modrinth_slugs: &["moon-phase", "minihud"], search_query: "Moon" },
    VoltModMeta { id: 67, key: "session_stats", title: "Session Combat & Mining Stats", summary: "Tracks kills, blocks mined/placed & distance traveled this session", badge: "ST", category: VoltModCategory::Hud, modrinth_slugs: &["session-stats", "stat-tracker"], search_query: "Stats" },
    VoltModMeta { id: 68, key: "weather_sound", title: "Lightning & Weather Tweaks", summary: "Suppresses lightning sky flashes, thunder shake & heavy rain splashes", badge: "WS", category: VoltModCategory::Camera, modrinth_slugs: &["clear-despawn", "ambient-sounds"], search_query: "AmbientSounds" },
    VoltModMeta { id: 69, key: "custom_glint", title: "Custom Enchant Glint & Scale", summary: "Custom glint colors (Rainbow, Gold, Ice, Crimson, Off) & held scale", badge: "GL", category: VoltModCategory::Camera, modrinth_slugs: &["custom-glint", "glint-colorizer"], search_query: "Enchant Glint" },
    VoltModMeta { id: 70, key: "clear_caves", title: "Cave & Void Fog Clearer", summary: "Clears dark subterranean void fog & smooths Night Vision transitions", badge: "CC", category: VoltModCategory::Camera, modrinth_slugs: &["no-fog", "nofog"], search_query: "No Fog" },
    VoltModMeta { id: 71, key: "player_highlight", title: "Line-of-Sight Entity Glow", summary: "Optional highlight boxes around nearby players or mobs in view", badge: "PH", category: VoltModCategory::Camera, modrinth_slugs: &["player-highlighter", "glowing-entities"], search_query: "Highlighter" },
    VoltModMeta { id: 72, key: "litematica_grid", title: "Build Measure / Ruler Box (A-B)", summary: "Set Point A & B to draw a 3D wireframe box with live X x Y x Z volume", badge: "LM", category: VoltModCategory::World, modrinth_slugs: &["litematica", "worldedit"], search_query: "Litematica" },
    VoltModMeta { id: 73, key: "death_history", title: "Death History & Home Marker", summary: "Tracks last 5 death locations + 1-tap Home bookmark & distance pill", badge: "DH", category: VoltModCategory::World, modrinth_slugs: &["death-backup", "xaeros-world-map"], search_query: "Death" },
    VoltModMeta { id: 74, key: "chunk_pillars", title: "Chunk Pillars & Biome Borders", summary: "Draws vertical chunk corner pillars & ground biome transition lines", badge: "CP", category: VoltModCategory::World, modrinth_slugs: &["chunk-borders", "chunkdebug"], search_query: "Chunk Borders" },
    VoltModMeta { id: 75, key: "offhand_torch", title: "Offhand Torch Auto-Placer", summary: "Auto-places an offhand torch when mining in dark caves (light < 4)", badge: "OT", category: VoltModCategory::World, modrinth_slugs: &["offhand-torch", "rightclickharvest"], search_query: "Torch" },
    VoltModMeta { id: 76, key: "chat_mention", title: "Chat Mention & Keyword Ping", summary: "Highlights chat messages & plays a chime when your name is mentioned", badge: "CM", category: VoltModCategory::Utility, modrinth_slugs: &["chat-mention", "chime"], search_query: "Chat Mention" },
    VoltModMeta { id: 77, key: "drop_guard", title: "Drop Guard & Locked Hotbar", summary: "Blocks accidental dropping of tools, weapons, totems & slots 1-3", badge: "DG", category: VoltModCategory::Utility, modrinth_slugs: &["no-drop", "lock-slots"], search_query: "Drop Guard" },
    VoltModMeta { id: 78, key: "hotbar_restock", title: "Hotbar Stack Auto-Restock", summary: "Refills empty building blocks, fireworks & pearls from main inventory", badge: "HR", category: VoltModCategory::Utility, modrinth_slugs: &["mouse-tweaks", "inventory-profiles-next"], search_query: "Mouse Tweaks" },
    VoltModMeta { id: 79, key: "photo_mode", title: "Photo Mode & Clean Capture", summary: "Hides all HUD overlays & optionally adds framing composition guides", badge: "PM", category: VoltModCategory::Camera, modrinth_slugs: &["fabrishot", "screenshot-viewer"], search_query: "Fabrishot" },
    VoltModMeta { id: 80, key: "jei", title: "JEI (Just Enough Items)", summary: "Searchable item browser with live 3x3 crafting & furnace recipe viewer", badge: "JEI", category: VoltModCategory::Utility, modrinth_slugs: &["jei", "roughly-enough-items"], search_query: "Just Enough Items" },
    VoltModMeta { id: 81, key: "cinematic_timelapse", title: "Cinematic Time Lapse", summary: "Configurable start countdown (5s/10s+), smooth camera path & HUD hide", badge: "TL", category: VoltModCategory::Camera, modrinth_slugs: &["replaymod", "flashback"], search_query: "Replay Mod" },
    VoltModMeta { id: 82, key: "blocks_outline", title: "Blocks Outline", summary: "Custom block outline color, thickness, rainbow chroma, pulse & block fill overlay", badge: "BO", category: VoltModCategory::World, modrinth_slugs: &["block-outline", "custom-block-outline", "better-block-outline"], search_query: "Block Outline" },
];

pub fn volt_mod_enabled(v: &VoltExtras, idx: u8) -> bool {
    match idx {
        0 => v.appleskin.enabled,
        1 => v.crystal_optimizer.enabled,
        2 => v.shield_status.enabled,
        3 => v.uku_hud.enabled,
        4 => v.freelook.enabled,
        5 => v.trajectories.enabled,
        6 => v.health_indicators.enabled,
        7 => v.item_scroller.enabled,
        8 => v.tweakeroo.enabled,
        9 => v.elytra_hud.enabled,
        10 => v.waypoints.enabled,
        11 => v.totem_counter.enabled,
        12 => v.cps_combo.enabled,
        13 => v.keystrokes.enabled,
        14 => v.pvp_visuals.enabled,
        15 => v.crosshair_tweaks.enabled,
        16 => v.zoom.enabled,
        17 => v.auto_sprint.enabled,
        18 => v.fullbright.enabled,
        19 => v.coords_hud.enabled,
        20 => v.telemetry_hud.enabled,
        21 => v.item_counters.enabled,
        22 => v.clock_day_hud.enabled,
        23 => v.chunk_hitboxes.enabled,
        24 => v.chat_tweaks.enabled,
        25 => v.freecam.enabled,
        26 => v.radar.enabled,
        27 => v.auto_reconnect.enabled,
        28 => v.chat_macros.enabled,
        29 => v.tnt_timer.enabled,
        30 => v.auto_armor.enabled,
        31 => v.elytra_swap.enabled,
        32 => v.slime_mobcap.enabled,
        33 => v.auto_eat.enabled,
        34 => v.radial_menu.enabled,
        35 => v.clear_skies.enabled,
        36 => v.smart_trade.enabled,
        37 => v.totem_pop_counter.enabled,
        38 => v.auto_respawn.enabled,
        39 => v.shulker_peek.enabled,
        40 => v.ground_drops.enabled,
        41 => v.entity_culling.enabled,
        42 => v.auto_fish.enabled,
        43 => v.look_inspector.enabled,
        44 => v.sound_muffler.enabled,
        45 => v.portal_calc.enabled,
        46 => v.compass_hud.enabled,
        47 => v.safe_walk.enabled,
        48 => v.respawn_status.enabled,
        49 => v.scoreboard_tweaks.enabled,
        50 => v.sign_stand_helper.enabled,
        51 => v.dynamic_lights.enabled,
        52 => v.low_shield.enabled,
        53 => v.clean_view.enabled,
        54 => v.name_tags_plus.enabled,
        55 => v.hit_color.enabled,
        56 => v.crit_ring.enabled,
        57 => v.bow_charge.enabled,
        58 => v.cooldown_hud.enabled,
        59 => v.reach_box.enabled,
        60 => v.potion_hud.enabled,
        61 => v.durability_warn.enabled,
        62 => v.biome_title.enabled,
        63 => v.xp_tracker.enabled,
        64 => v.inventory_hud.enabled,
        65 => v.tab_ping.enabled,
        66 => v.moon_phase.enabled,
        67 => v.session_stats.enabled,
        68 => v.weather_sound.enabled,
        69 => v.custom_glint.enabled,
        70 => v.clear_caves.enabled,
        71 => v.player_highlight.enabled,
        72 => v.litematica_grid.enabled,
        73 => v.death_history.enabled,
        74 => v.chunk_pillars.enabled,
        75 => v.offhand_torch.enabled,
        76 => v.chat_mention.enabled,
        77 => v.drop_guard.enabled,
        78 => v.hotbar_restock.enabled,
        79 => v.photo_mode.enabled,
        80 => v.jei.enabled,
        81 => v.cinematic_timelapse.enabled,
        82 => v.blocks_outline.enabled,
        _ => false,
    }
}

pub fn set_volt_mod_enabled(v: &mut VoltExtras, idx: u8, on: bool) {
    match idx {
        0 => v.appleskin.enabled = on,
        1 => v.crystal_optimizer.enabled = on,
        2 => v.shield_status.enabled = on,
        3 => v.uku_hud.enabled = on,
        4 => { v.freelook.enabled = on; if !on { v.freelook.active = false; } }
        5 => v.trajectories.enabled = on,
        6 => v.health_indicators.enabled = on,
        7 => v.item_scroller.enabled = on,
        8 => v.tweakeroo.enabled = on,
        9 => v.elytra_hud.enabled = on,
        10 => v.waypoints.enabled = on,
        11 => v.totem_counter.enabled = on,
        12 => v.cps_combo.enabled = on,
        13 => v.keystrokes.enabled = on,
        14 => v.pvp_visuals.enabled = on,
        15 => v.crosshair_tweaks.enabled = on,
        16 => { v.zoom.enabled = on; if !on { v.zoom.active = false; } }
        17 => v.auto_sprint.enabled = on,
        18 => v.fullbright.enabled = on,
        19 => v.coords_hud.enabled = on,
        20 => v.telemetry_hud.enabled = on,
        21 => v.item_counters.enabled = on,
        22 => v.clock_day_hud.enabled = on,
        23 => v.chunk_hitboxes.enabled = on,
        24 => v.chat_tweaks.enabled = on,
        25 => { v.freecam.enabled = on; if !on { v.freecam.active = false; } }
        26 => v.radar.enabled = on,
        27 => v.auto_reconnect.enabled = on,
        28 => v.chat_macros.enabled = on,
        29 => v.tnt_timer.enabled = on,
        30 => v.auto_armor.enabled = on,
        31 => v.elytra_swap.enabled = on,
        32 => v.slime_mobcap.enabled = on,
        33 => v.auto_eat.enabled = on,
        34 => v.radial_menu.enabled = on,
        35 => v.clear_skies.enabled = on,
        36 => v.smart_trade.enabled = on,
        37 => v.totem_pop_counter.enabled = on,
        38 => v.auto_respawn.enabled = on,
        39 => v.shulker_peek.enabled = on,
        40 => v.ground_drops.enabled = on,
        41 => v.entity_culling.enabled = on,
        42 => v.auto_fish.enabled = on,
        43 => v.look_inspector.enabled = on,
        44 => v.sound_muffler.enabled = on,
        45 => v.portal_calc.enabled = on,
        46 => v.compass_hud.enabled = on,
        47 => v.safe_walk.enabled = on,
        48 => v.respawn_status.enabled = on,
        49 => v.scoreboard_tweaks.enabled = on,
        50 => v.sign_stand_helper.enabled = on,
        51 => v.dynamic_lights.enabled = on,
        52 => v.low_shield.enabled = on,
        53 => v.clean_view.enabled = on,
        54 => v.name_tags_plus.enabled = on,
        55 => v.hit_color.enabled = on,
        56 => v.crit_ring.enabled = on,
        57 => v.bow_charge.enabled = on,
        58 => v.cooldown_hud.enabled = on,
        59 => v.reach_box.enabled = on,
        60 => v.potion_hud.enabled = on,
        61 => v.durability_warn.enabled = on,
        62 => v.biome_title.enabled = on,
        63 => v.xp_tracker.enabled = on,
        64 => v.inventory_hud.enabled = on,
        65 => v.tab_ping.enabled = on,
        66 => v.moon_phase.enabled = on,
        67 => v.session_stats.enabled = on,
        68 => v.weather_sound.enabled = on,
        69 => v.custom_glint.enabled = on,
        70 => v.clear_caves.enabled = on,
        71 => v.player_highlight.enabled = on,
        72 => v.litematica_grid.enabled = on,
        73 => v.death_history.enabled = on,
        74 => v.chunk_pillars.enabled = on,
        75 => v.offhand_torch.enabled = on,
        76 => v.chat_mention.enabled = on,
        77 => v.drop_guard.enabled = on,
        78 => v.hotbar_restock.enabled = on,
        79 => { v.photo_mode.enabled = on; if !on { v.photo_mode.active = false; } }
        80 => { v.jei.enabled = on; if !on { v.jei.overlay_open = false; } }
        81 => v.cinematic_timelapse.enabled = on,
        82 => v.blocks_outline.enabled = on,
        _ => {}
    }
}

fn flag(value: &str, d: bool) -> bool {
    match value {
        "true" => true,
        "false" => false,
        _ => d,
    }
}
fn bool_s(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}

impl VoltExtras {
    pub fn parse_key(&mut self, key: &str, value: &str) {
        let Some(k) = key.strip_prefix("volt.") else { return };
        match k {
            "appleskin.enabled" => self.appleskin.enabled = flag(value, self.appleskin.enabled),
            "appleskin.saturation" => self.appleskin.show_saturation_overlay = flag(value, self.appleskin.show_saturation_overlay),
            "appleskin.exhaustion" => self.appleskin.show_exhaustion_underlay = flag(value, self.appleskin.show_exhaustion_underlay),
            "appleskin.heldPreview" => self.appleskin.show_held_food_preview = flag(value, self.appleskin.show_held_food_preview),
            "appleskin.regen" => self.appleskin.show_health_regen_overlay = flag(value, self.appleskin.show_health_regen_overlay),
            "appleskin.tooltip" => self.appleskin.show_food_tooltip = flag(value, self.appleskin.show_food_tooltip),
            "appleskin.tooltipAlways" => self.appleskin.always_show_tooltip = flag(value, self.appleskin.always_show_tooltip),
            "appleskin.debug" => self.appleskin.show_debug_stats = flag(value, self.appleskin.show_debug_stats),
            "appleskin.fullBar" => self.appleskin.highlight_full_bar = flag(value, self.appleskin.highlight_full_bar),
            "appleskin.pulse" => if let Ok(v) = value.parse::<u8>() { self.appleskin.pulse_speed_pct = v.clamp(25, 250); },

            "crystal.enabled" => self.crystal_optimizer.enabled = flag(value, self.crystal_optimizer.enabled),
            "crystal.instantBreak" => self.crystal_optimizer.instant_break = flag(value, self.crystal_optimizer.instant_break),
            "crystal.weakness" => self.crystal_optimizer.respect_weakness = flag(value, self.crystal_optimizer.respect_weakness),
            "crystal.fastPlace" => self.crystal_optimizer.fast_place = flag(value, self.crystal_optimizer.fast_place),
            "crystal.timeoutMs" => if let Ok(v) = value.parse::<u16>() { self.crystal_optimizer.timeout_ms = v.clamp(100, 2000); },

            "shield.enabled" => self.shield_status.enabled = flag(value, self.shield_status.enabled),
            "shield.enabledColor" => self.shield_status.enabled_color = ShieldColorPreset::parse(value, self.shield_status.enabled_color),
            "shield.disabledColor" => self.shield_status.disabled_color = ShieldColorPreset::parse(value, self.shield_status.disabled_color),
            "shield.interpolate" => self.shield_status.interpolate_colors = flag(value, self.shield_status.interpolate_colors),
            "shield.mode" => self.shield_status.display_mode = ShieldDisplayMode::parse(value, self.shield_status.display_mode),
            "shield.sound" => self.shield_status.disable_sound = flag(value, self.shield_status.disable_sound),

            "uku.enabled" => self.uku_hud.enabled = flag(value, self.uku_hud.enabled),
            "uku.armor" => self.uku_hud.armor_enabled = flag(value, self.uku_hud.armor_enabled),
            "uku.potion" => self.uku_hud.potion_enabled = flag(value, self.uku_hud.potion_enabled),
            "uku.armorAnchor" => self.uku_hud.armor_anchor = UkuAnchor::parse(value, self.uku_hud.armor_anchor),
            "uku.armorSide" => self.uku_hud.armor_side = UkuSide::parse(value, self.uku_hud.armor_side),
            "uku.style" => self.uku_hud.box_style = UkuStyle::parse(value, self.uku_hud.box_style),
            "uku.durability" => self.uku_hud.durability_display = UkuDurability::parse(value, self.uku_hud.durability_display),
            "uku.offhand" => self.uku_hud.show_offhand = flag(value, self.uku_hud.show_offhand),
            "uku.hideEmpty" => self.uku_hud.hide_empty = flag(value, self.uku_hud.hide_empty),
            "uku.lowPct" => if let Ok(v) = value.parse::<u8>() { self.uku_hud.low_durability_pct = v.min(50); },
            "uku.potionAnchor" => self.uku_hud.potion_anchor = UkuPotionAnchor::parse(value, self.uku_hud.potion_anchor),
            "uku.potionCompact" => self.uku_hud.potion_compact = flag(value, self.uku_hud.potion_compact),

            "freelook.enabled" => self.freelook.enabled = flag(value, self.freelook.enabled),
            "freelook.toggle" => self.freelook.toggle_mode = flag(value, self.freelook.toggle_mode),
            "freelook.invertY" => self.freelook.invert_y = flag(value, self.freelook.invert_y),
            "freelook.sens" => if let Ok(v) = value.parse::<u8>() { self.freelook.sensitivity_pct = v.clamp(25, 200); },
            "freelook.persp" => self.freelook.preserve_perspective = flag(value, self.freelook.preserve_perspective),

            "traj.enabled" => self.trajectories.enabled = flag(value, self.trajectories.enabled),
            "traj.bow" => self.trajectories.show_bow = flag(value, self.trajectories.show_bow),
            "traj.pearl" => self.trajectories.show_pearl = flag(value, self.trajectories.show_pearl),
            "traj.trident" => self.trajectories.show_trident = flag(value, self.trajectories.show_trident),
            "traj.wind" => self.trajectories.show_wind_charge = flag(value, self.trajectories.show_wind_charge),
            "traj.potion" => self.trajectories.show_potion = flag(value, self.trajectories.show_potion),
            "traj.box" => self.trajectories.show_impact_box = flag(value, self.trajectories.show_impact_box),
            "traj.color" => self.trajectories.color = ShieldColorPreset::parse(value, self.trajectories.color),
            "traj.width" => if let Ok(v) = value.parse::<u8>() { self.trajectories.line_width = v.clamp(1, 5); },

            "hpind.enabled" => self.health_indicators.enabled = flag(value, self.health_indicators.enabled),
            "hpind.bars" => self.health_indicators.show_bars = flag(value, self.health_indicators.show_bars),
            "hpind.popups" => self.health_indicators.show_damage_popups = flag(value, self.health_indicators.show_damage_popups),
            "hpind.crits" => self.health_indicators.highlight_crits = flag(value, self.health_indicators.highlight_crits),
            "hpind.players" => self.health_indicators.show_players = flag(value, self.health_indicators.show_players),
            "hpind.hostiles" => self.health_indicators.show_hostiles = flag(value, self.health_indicators.show_hostiles),
            "hpind.range" => if let Ok(v) = value.parse::<u8>() { self.health_indicators.max_range = v.clamp(8, 48); },
            "hpind.scale" => if let Ok(v) = value.parse::<u8>() { self.health_indicators.bar_scale = v.clamp(60, 150); },

            "invscroll.enabled" => self.item_scroller.enabled = flag(value, self.item_scroller.enabled),
            "invscroll.refill" => self.item_scroller.auto_refill_hotbar = flag(value, self.item_scroller.auto_refill_hotbar),
            _ => self.parse_key_rest(k, value),
        }
    }

    fn parse_key_rest(&mut self, k: &str, value: &str) {
        match k {
            "invscroll.autoRefill" => self.item_scroller.auto_refill_hotbar = flag(value, self.item_scroller.auto_refill_hotbar),
            "invscroll.totem" => self.item_scroller.quick_offhand_totem = flag(value, self.item_scroller.quick_offhand_totem),
            "invscroll.sort" => self.item_scroller.sort_shortcut = flag(value, self.item_scroller.sort_shortcut),
            "invscroll.wheel" => self.item_scroller.scroll_one_by_one = flag(value, self.item_scroller.scroll_one_by_one),
            "invscroll.tools" => self.item_scroller.preserve_tools = flag(value, self.item_scroller.preserve_tools),

            "tweak.enabled" => self.tweakeroo.enabled = flag(value, self.tweakeroo.enabled),
            "tweak.fastPlace" => self.tweakeroo.fast_block_place = flag(value, self.tweakeroo.fast_block_place),
            "tweak.lightOverlay" => self.tweakeroo.light_overlay = flag(value, self.tweakeroo.light_overlay),
            "tweak.autoTool" => self.tweakeroo.auto_tool = flag(value, self.tweakeroo.auto_tool),
            "tweak.silkGuard" => self.tweakeroo.silk_touch_guard = flag(value, self.tweakeroo.silk_touch_guard),
            "tweak.noPortalClose" => self.tweakeroo.no_portal_gui_close = flag(value, self.tweakeroo.no_portal_gui_close),
            "tweak.lightRange" => if let Ok(v) = value.parse::<u8>() { self.tweakeroo.light_range = v.clamp(6, 20); },

            "elytraHud.enabled" => self.elytra_hud.enabled = flag(value, self.elytra_hud.enabled),
            "elytraHud.alt" => self.elytra_hud.show_altitude = flag(value, self.elytra_hud.show_altitude),
            "elytraHud.vspd" => self.elytra_hud.show_vertical_speed = flag(value, self.elytra_hud.show_vertical_speed),
            "elytraHud.pitch" => self.elytra_hud.show_pitch_gauge = flag(value, self.elytra_hud.show_pitch_gauge),
            "elytraHud.fireworks" => self.elytra_hud.show_fireworks = flag(value, self.elytra_hud.show_fireworks),
            "elytraHud.warnPct" => if let Ok(v) = value.parse::<u8>() { self.elytra_hud.chestplate_warn_pct = v.min(50); },
            "elytraHud.mace" => self.elytra_hud.show_mace_damage = flag(value, self.elytra_hud.show_mace_damage),

            "waypoints.enabled" => self.waypoints.enabled = flag(value, self.waypoints.enabled),
            "waypoints.autoDeath" => self.waypoints.auto_death_waypoint = flag(value, self.waypoints.auto_death_waypoint),
            "waypoints.beacons" => self.waypoints.show_beacons = flag(value, self.waypoints.show_beacons),
            "waypoints.hud" => self.waypoints.show_hud_distances = flag(value, self.waypoints.show_hud_distances),
            "waypoints.maxDist" => if let Ok(v) = value.parse::<u16>() { self.waypoints.max_distance = v.clamp(100, 10000); },
            "waypoints.beamH" => if let Ok(v) = value.parse::<u8>() { self.waypoints.beam_height = v.clamp(16, 128); },

            "totemHud.enabled" => self.totem_counter.enabled = flag(value, self.totem_counter.enabled),
            "totemHud.badge" => self.totem_counter.show_hud_badge = flag(value, self.totem_counter.show_hud_badge),
            "totemHud.lowHp" => self.totem_counter.low_hp_alert = flag(value, self.totem_counter.low_hp_alert),
            "totemHud.hearts" => if let Ok(v) = value.parse::<u8>() { self.totem_counter.low_hp_hearts = v.clamp(2, 16); },
            "totemHud.warnEmpty" => self.totem_counter.warn_when_empty = flag(value, self.totem_counter.warn_when_empty),
            "totemHud.corner" => self.totem_counter.corner = Corner::parse(value, self.totem_counter.corner),

            "cps.enabled" => self.cps_combo.enabled = flag(value, self.cps_combo.enabled),
            "cps.lmb" => self.cps_combo.show_lmb_cps = flag(value, self.cps_combo.show_lmb_cps),
            "cps.rmb" => self.cps_combo.show_rmb_cps = flag(value, self.cps_combo.show_rmb_cps),
            "cps.reach" => self.cps_combo.show_reach = flag(value, self.cps_combo.show_reach),
            "cps.combo" => self.cps_combo.show_combo = flag(value, self.cps_combo.show_combo),
            "cps.corner" => self.cps_combo.corner = Corner::parse(value, self.cps_combo.corner),
            "cps.compact" => self.cps_combo.compact = flag(value, self.cps_combo.compact),

            "keystrokes.enabled" => self.keystrokes.enabled = flag(value, self.keystrokes.enabled),
            "keystrokes.cps" => self.keystrokes.show_cps = flag(value, self.keystrokes.show_cps),
            "keystrokes.space" => self.keystrokes.show_space = flag(value, self.keystrokes.show_space),
            "keystrokes.sprintSneak" => self.keystrokes.show_sprint_sneak = flag(value, self.keystrokes.show_sprint_sneak),
            "keystrokes.corner" => self.keystrokes.corner = Corner::parse(value, self.keystrokes.corner),
            "keystrokes.chroma" => self.keystrokes.chroma = flag(value, self.keystrokes.chroma),
            "keystrokes.opacity" => if let Ok(v) = value.parse::<u8>() { self.keystrokes.bg_opacity_pct = v.clamp(20, 100); },

            "pvpvis.enabled" => self.pvp_visuals.enabled = flag(value, self.pvp_visuals.enabled),
            "pvpvis.lowFire" => if let Ok(v) = value.parse::<u8>() { self.pvp_visuals.low_fire_pct = v.min(100); },
            "pvpvis.noPumpkin" => self.pvp_visuals.hide_pumpkin = flag(value, self.pvp_visuals.hide_pumpkin),
            "pvpvis.noPortal" => self.pvp_visuals.hide_portal_overlay = flag(value, self.pvp_visuals.hide_portal_overlay),
            "pvpvis.hurtCam" => if let Ok(v) = value.parse::<u8>() { self.pvp_visuals.hurt_cam_pct = v.min(100); },
            "pvpvis.lowBob" => self.pvp_visuals.reduced_bobbing = flag(value, self.pvp_visuals.reduced_bobbing),
            "pvpvis.glint" => if let Ok(v) = value.parse::<u8>() { self.pvp_visuals.glint_pct = v.min(100); },

            "crosshair.enabled" => self.crosshair_tweaks.enabled = flag(value, self.crosshair_tweaks.enabled),
            "crosshair.hitMarker" => self.crosshair_tweaks.hit_marker = flag(value, self.crosshair_tweaks.hit_marker),
            "crosshair.style" => self.crosshair_tweaks.style = CrosshairStyle::parse(value, self.crosshair_tweaks.style),
            "crosshair.color" => self.crosshair_tweaks.color = ShieldColorPreset::parse(value, self.crosshair_tweaks.color),
            "crosshair.size" => if let Ok(v) = value.parse::<u8>() { self.crosshair_tweaks.size = v.clamp(3, 12); },
            "crosshair.thick" => if let Ok(v) = value.parse::<u8>() { self.crosshair_tweaks.thickness = v.clamp(1, 4); },
            "crosshair.dynGap" => self.crosshair_tweaks.dynamic_gap = flag(value, self.crosshair_tweaks.dynamic_gap),

            "zoom.enabled" => self.zoom.enabled = flag(value, self.zoom.enabled),
            "zoom.level" => if let Ok(v) = value.parse::<u8>() { self.zoom.zoom_level = v.clamp(2, 10); },
            "zoom.smooth" => self.zoom.smooth = flag(value, self.zoom.smooth),
            "zoom.sens" => self.zoom.scale_sensitivity = flag(value, self.zoom.scale_sensitivity),
            "zoom.hideHand" => self.zoom.hide_hand = flag(value, self.zoom.hide_hand),

            "sprint.enabled" => self.auto_sprint.enabled = flag(value, self.auto_sprint.enabled),
            "sprint.badge" => self.auto_sprint.show_indicator = flag(value, self.auto_sprint.show_indicator),
            "sprint.sneak" => self.auto_sprint.toggle_sneak = flag(value, self.auto_sprint.toggle_sneak),
            "sprint.water" => self.auto_sprint.sprint_in_water = flag(value, self.auto_sprint.sprint_in_water),
            "sprint.corner" => self.auto_sprint.corner = Corner::parse(value, self.auto_sprint.corner),

            "fullbright.enabled" => self.fullbright.enabled = flag(value, self.fullbright.enabled),
            "fullbright.gamma" => if let Ok(v) = value.parse::<u16>() { self.fullbright.gamma_pct = v.clamp(200, 1500); },
            "fullbright.cavesOnly" => self.fullbright.caves_only = flag(value, self.fullbright.caves_only),
            "fullbright.smooth" => self.fullbright.smooth_fade = flag(value, self.fullbright.smooth_fade),

            "coords.enabled" => self.coords_hud.enabled = flag(value, self.coords_hud.enabled),
            "coords.xyz" => self.coords_hud.show_xyz = flag(value, self.coords_hud.show_xyz),
            "coords.nether" => self.coords_hud.show_nether_equiv = flag(value, self.coords_hud.show_nether_equiv),
            "coords.facing" => self.coords_hud.show_facing = flag(value, self.coords_hud.show_facing),
            "coords.biome" => self.coords_hud.show_biome = flag(value, self.coords_hud.show_biome),
            "coords.corner" => self.coords_hud.corner = Corner::parse(value, self.coords_hud.corner),
            "coords.compact" => self.coords_hud.compact = flag(value, self.coords_hud.compact),

            "telem.enabled" => self.telemetry_hud.enabled = flag(value, self.telemetry_hud.enabled),
            "telem.fps" => self.telemetry_hud.show_fps = flag(value, self.telemetry_hud.show_fps),
            "telem.lows" => self.telemetry_hud.show_lows = flag(value, self.telemetry_hud.show_lows),
            "telem.ping" => self.telemetry_hud.show_ping = flag(value, self.telemetry_hud.show_ping),
            "telem.bps" => self.telemetry_hud.show_bps = flag(value, self.telemetry_hud.show_bps),
            "telem.corner" => self.telemetry_hud.corner = Corner::parse(value, self.telemetry_hud.corner),
            "telem.color" => self.telemetry_hud.color_coded = flag(value, self.telemetry_hud.color_coded),

            "itemcnt.enabled" => self.item_counters.enabled = flag(value, self.item_counters.enabled),
            "itemcnt.arrows" => self.item_counters.count_arrows = flag(value, self.item_counters.count_arrows),
            "itemcnt.pearls" => self.item_counters.count_pearls = flag(value, self.item_counters.count_pearls),
            "itemcnt.wind" => self.item_counters.count_wind_charges = flag(value, self.item_counters.count_wind_charges),
            "itemcnt.gapples" => self.item_counters.count_gapples = flag(value, self.item_counters.count_gapples),
            "itemcnt.zero" => self.item_counters.show_zero = flag(value, self.item_counters.show_zero),

            "clock.enabled" => self.clock_day_hud.enabled = flag(value, self.clock_day_hud.enabled),
            "clock.day" => self.clock_day_hud.show_day = flag(value, self.clock_day_hud.show_day),
            "clock.24h" => self.clock_day_hud.use_24h = flag(value, self.clock_day_hud.use_24h),
            "clock.light" => self.clock_day_hud.show_light_level = flag(value, self.clock_day_hud.show_light_level),
            "clock.safe" => self.clock_day_hud.show_spawn_safe = flag(value, self.clock_day_hud.show_spawn_safe),
            "clock.corner" => self.clock_day_hud.corner = Corner::parse(value, self.clock_day_hud.corner),

            "chunkhb.enabled" => self.chunk_hitboxes.enabled = flag(value, self.chunk_hitboxes.enabled),
            "chunkhb.chunks" => self.chunk_hitboxes.chunk_borders = flag(value, self.chunk_hitboxes.chunk_borders),
            "chunkhb.hitboxes" => self.chunk_hitboxes.entity_hitboxes = flag(value, self.chunk_hitboxes.entity_hitboxes),
            "chunkhb.hideF1" => self.chunk_hitboxes.hide_in_f1 = flag(value, self.chunk_hitboxes.hide_in_f1),
            "chunkhb.range" => if let Ok(v) = value.parse::<u8>() { self.chunk_hitboxes.chunk_range = v.clamp(1, 3); },

            "chat.enabled" => self.chat_tweaks.enabled = flag(value, self.chat_tweaks.enabled),
            "chat.ts" => self.chat_tweaks.timestamps = flag(value, self.chat_tweaks.timestamps),
            "chat.24h" => self.chat_tweaks.use_24h = flag(value, self.chat_tweaks.use_24h),
            "chat.compact" => self.chat_tweaks.compact_duplicates = flag(value, self.chat_tweaks.compact_duplicates),
            "chat.history" => self.chat_tweaks.keep_history = flag(value, self.chat_tweaks.keep_history),

            "freecam.enabled" => self.freecam.enabled = flag(value, self.freecam.enabled),
            "freecam.speed" => if let Ok(v) = value.parse::<u8>() { self.freecam.speed = v.clamp(1, 5); },
            "freecam.hideHand" => self.freecam.hide_hand = flag(value, self.freecam.hide_hand),
            "freecam.marker" => self.freecam.show_player_marker = flag(value, self.freecam.show_player_marker),
            "freecam.noclip" => self.freecam.noclip_flight = flag(value, self.freecam.noclip_flight),

            "radar.enabled" => self.radar.enabled = flag(value, self.radar.enabled),
            "radar.items" => self.radar.highlight_items = flag(value, self.radar.highlight_items),
            "radar.chests" => self.radar.highlight_chests = flag(value, self.radar.highlight_chests),
            "radar.spawners" => self.radar.highlight_spawners = flag(value, self.radar.highlight_spawners),
            "radar.pets" => self.radar.highlight_pets = flag(value, self.radar.highlight_pets),
            "radar.range" => if let Ok(v) = value.parse::<u8>() { self.radar.range = v.clamp(12, 48); },
            "radar.dist" => self.radar.show_distances = flag(value, self.radar.show_distances),

            "reconn.enabled" => self.auto_reconnect.enabled = flag(value, self.auto_reconnect.enabled),
            "reconn.delay" => if let Ok(v) = value.parse::<u8>() { self.auto_reconnect.delay_secs = v.clamp(2, 30); },
            "reconn.attempts" => if let Ok(v) = value.parse::<u8>() { self.auto_reconnect.max_attempts = v.clamp(1, 20); },
            "reconn.health" => self.auto_reconnect.show_server_health = flag(value, self.auto_reconnect.show_server_health),

            "macros.enabled" => self.chat_macros.enabled = flag(value, self.chat_macros.enabled),
            "macros.autogg" => self.chat_macros.auto_gg = flag(value, self.chat_macros.auto_gg),
            "macros.ggIdx" => if let Ok(v) = value.parse::<u8>() { self.chat_macros.gg_message_idx = v % 4; },
            "macros.bar" => self.chat_macros.show_quick_bar = flag(value, self.chat_macros.show_quick_bar),
            "macros.preset" => if let Ok(v) = value.parse::<u8>() { self.chat_macros.macro_preset_idx = v % 4; },

            "tnt.enabled" => self.tnt_timer.enabled = flag(value, self.tnt_timer.enabled),
            "tnt.tntFuse" => self.tnt_timer.show_tnt_fuse = flag(value, self.tnt_timer.show_tnt_fuse),
            "tnt.creeperFuse" => self.tnt_timer.show_creeper_fuse = flag(value, self.tnt_timer.show_creeper_fuse),
            "tnt.halo" => self.tnt_timer.danger_halo = flag(value, self.tnt_timer.danger_halo),
            "tnt.crystalRing" => self.tnt_timer.crystal_ring = flag(value, self.tnt_timer.crystal_ring),
            "tnt.anchorRing" => self.tnt_timer.anchor_ring = flag(value, self.tnt_timer.anchor_ring),

            "autoarmor.enabled" => self.auto_armor.enabled = flag(value, self.auto_armor.enabled),
            "autoarmor.pickup" => self.auto_armor.auto_on_pickup = flag(value, self.auto_armor.auto_on_pickup),
            "autoarmor.prot" => self.auto_armor.prefer_protection = flag(value, self.auto_armor.prefer_protection),
            "autoarmor.elytra" => self.auto_armor.respect_elytra = flag(value, self.auto_armor.respect_elytra),
            "autoarmor.minDur" => if let Ok(v) = value.parse::<u8>() { self.auto_armor.min_durability_pct = v.min(50); },

            "elyswap.enabled" => self.elytra_swap.enabled = flag(value, self.elytra_swap.enabled),
            "elyswap.dblJump" => self.elytra_swap.double_jump_swap = flag(value, self.elytra_swap.double_jump_swap),
            "elyswap.landBack" => self.elytra_swap.swap_back_on_land = flag(value, self.elytra_swap.swap_back_on_land),
            "elyswap.netherite" => self.elytra_swap.prefer_netherite = flag(value, self.elytra_swap.prefer_netherite),

            "slime.enabled" => self.slime_mobcap.enabled = flag(value, self.slime_mobcap.enabled),
            "slime.chunk" => self.slime_mobcap.show_slime_chunk = flag(value, self.slime_mobcap.show_slime_chunk),
            "slime.mobcap" => self.slime_mobcap.show_mob_count = flag(value, self.slime_mobcap.show_mob_count),
            "slime.spawn" => self.slime_mobcap.show_spawn_check = flag(value, self.slime_mobcap.show_spawn_check),
            "slime.corner" => self.slime_mobcap.corner = Corner::parse(value, self.slime_mobcap.corner),

            "autoeat.enabled" => self.auto_eat.enabled = flag(value, self.auto_eat.enabled),
            "autoeat.hunger" => if let Ok(v) = value.parse::<u8>() { self.auto_eat.hunger_threshold = v.clamp(2, 19); },
            "autoeat.hp" => if let Ok(v) = value.parse::<u8>() { self.auto_eat.hp_threshold = v.clamp(4, 20); },
            "autoeat.safeOnly" => self.auto_eat.avoid_rotten_raw = flag(value, self.auto_eat.avoid_rotten_raw),
            "autoeat.warn" => self.auto_eat.low_hunger_alert = flag(value, self.auto_eat.low_hunger_alert),

            "radial.enabled" => self.radial_menu.enabled = flag(value, self.radial_menu.enabled),
            "radial.strip" => self.radial_menu.show_quick_strip = flag(value, self.radial_menu.show_quick_strip),
            "radial.pearl" => self.radial_menu.include_pearl = flag(value, self.radial_menu.include_pearl),
            "radial.gapple" => self.radial_menu.include_gapple = flag(value, self.radial_menu.include_gapple),
            "radial.firework" => self.radial_menu.include_firework = flag(value, self.radial_menu.include_firework),
            "radial.totem" => self.radial_menu.include_totem = flag(value, self.radial_menu.include_totem),

            "skies.enabled" => self.clear_skies.enabled = flag(value, self.clear_skies.enabled),
            "skies.rain" => self.clear_skies.hide_rain_snow = flag(value, self.clear_skies.hide_rain_snow),
            "skies.waterFog" => self.clear_skies.clear_water_fog = flag(value, self.clear_skies.clear_water_fog),
            "skies.netherFog" => self.clear_skies.clear_nether_fog = flag(value, self.clear_skies.clear_nether_fog),
            "skies.terrainFog" => self.clear_skies.clear_terrain_fog = flag(value, self.clear_skies.clear_terrain_fog),
            "skies.time" => self.clear_skies.time_override = TimeMood::parse(value, self.clear_skies.time_override),

            "trade.enabled" => self.smart_trade.enabled = flag(value, self.smart_trade.enabled),
            "trade.maxEnch" => self.smart_trade.highlight_max_enchants = flag(value, self.smart_trade.highlight_max_enchants),
            "trade.cheap" => self.smart_trade.highlight_cheap_trades = flag(value, self.smart_trade.highlight_cheap_trades),
            "trade.maxEm" => if let Ok(v) = value.parse::<u8>() { self.smart_trade.cheap_emerald_max = v.clamp(1, 32); },
            "trade.badge" => self.smart_trade.show_discount_badge = flag(value, self.smart_trade.show_discount_badge),

            "pops.enabled" => self.totem_pop_counter.enabled = flag(value, self.totem_pop_counter.enabled),
            "pops.self" => self.totem_pop_counter.track_self = flag(value, self.totem_pop_counter.track_self),
            "pops.enemy" => self.totem_pop_counter.track_enemies = flag(value, self.totem_pop_counter.track_enemies),
            "pops.sound" => self.totem_pop_counter.play_alert_sound = flag(value, self.totem_pop_counter.play_alert_sound),
            "pops.feed" => self.totem_pop_counter.show_chat_feed = flag(value, self.totem_pop_counter.show_chat_feed),
            "pops.corner" => self.totem_pop_counter.corner = Corner::parse(value, self.totem_pop_counter.corner),

            "respawn.enabled" => self.auto_respawn.enabled = flag(value, self.auto_respawn.enabled),
            "respawn.instant" => self.auto_respawn.instant_respawn = flag(value, self.auto_respawn.instant_respawn),
            "respawn.log" => self.auto_respawn.log_death_coords = flag(value, self.auto_respawn.log_death_coords),
            "respawn.wp" => self.auto_respawn.save_death_waypoint = flag(value, self.auto_respawn.save_death_waypoint),

            "shulker.enabled" => self.shulker_peek.enabled = flag(value, self.shulker_peek.enabled),
            "shulker.shulker" => self.shulker_peek.peek_shulkers = flag(value, self.shulker_peek.peek_shulkers),
            "shulker.bundle" => self.shulker_peek.peek_bundles = flag(value, self.shulker_peek.peek_bundles),
            "shulker.color" => self.shulker_peek.color_by_shulker = flag(value, self.shulker_peek.color_by_shulker),
            "shulker.always" => self.shulker_peek.always_show = flag(value, self.shulker_peek.always_show),

            "drops.enabled" => self.ground_drops.enabled = flag(value, self.ground_drops.enabled),
            "drops.beams" => self.ground_drops.rare_item_beams = flag(value, self.ground_drops.rare_item_beams),
            "drops.labels" => self.ground_drops.merge_count_labels = flag(value, self.ground_drops.merge_count_labels),
            "drops.totems" => self.ground_drops.highlight_totems = flag(value, self.ground_drops.highlight_totems),
            "drops.range" => if let Ok(v) = value.parse::<u8>() { self.ground_drops.range = v.clamp(8, 48); },

            "culling.enabled" => self.entity_culling.enabled = flag(value, self.entity_culling.enabled),
            "culling.dist" => if let Ok(v) = value.parse::<u8>() { self.entity_culling.cull_distance = v.clamp(16, 96); },
            "culling.expl" => self.entity_culling.reduce_explosions = flag(value, self.entity_culling.reduce_explosions),
            "culling.smoke" => self.entity_culling.reduce_smoke = flag(value, self.entity_culling.reduce_smoke),
            "culling.swirls" => self.entity_culling.hide_potion_swirls = flag(value, self.entity_culling.hide_potion_swirls),
            "culling.back" => self.entity_culling.skip_back_entities = flag(value, self.entity_culling.skip_back_entities),

            "autofish.enabled" => self.auto_fish.enabled = flag(value, self.auto_fish.enabled),
            "autofish.visual" => self.auto_fish.visual_bite_alert = flag(value, self.auto_fish.visual_bite_alert),
            "autofish.chime" => self.auto_fish.audio_chime = flag(value, self.auto_fish.audio_chime),
            "autofish.reel" => self.auto_fish.auto_reel = flag(value, self.auto_fish.auto_reel),
            "autofish.recast" => self.auto_fish.auto_recast = flag(value, self.auto_fish.auto_recast),
            "autofish.delay" => if let Ok(v) = value.parse::<u8>() { self.auto_fish.recast_delay_ticks = v.clamp(5, 40); },

            "jade.enabled" => self.look_inspector.enabled = flag(value, self.look_inspector.enabled),
            "jade.blocks" => self.look_inspector.show_blocks = flag(value, self.look_inspector.show_blocks),
            "jade.crops" => self.look_inspector.show_crops_pct = flag(value, self.look_inspector.show_crops_pct),
            "jade.mobs" => self.look_inspector.show_mob_health = flag(value, self.look_inspector.show_mob_health),
            "jade.chests" => self.look_inspector.show_container_info = flag(value, self.look_inspector.show_container_info),
            "jade.compact" => self.look_inspector.compact = flag(value, self.look_inspector.compact),
            "jade.icon3d" => self.look_inspector.show_3d_icon = flag(value, self.look_inspector.show_3d_icon),
            "jade.toolHint" => self.look_inspector.show_tool_hint = flag(value, self.look_inspector.show_tool_hint),
            "jade.modSrc" => self.look_inspector.show_mod_source = flag(value, self.look_inspector.show_mod_source),
            "jade.yOffset" => if let Ok(v) = value.parse::<u8>() { self.look_inspector.y_offset = v.clamp(8, 96); },

            "muffler.enabled" => self.sound_muffler.enabled = flag(value, self.sound_muffler.enabled),
            "muffler.portals" => self.sound_muffler.mute_portals = flag(value, self.sound_muffler.mute_portals),
            "muffler.ghasts" => self.sound_muffler.mute_ghasts = flag(value, self.sound_muffler.mute_ghasts),
            "muffler.weather" => self.sound_muffler.mute_weather = flag(value, self.sound_muffler.mute_weather),
            "muffler.pistons" => self.sound_muffler.mute_pistons = flag(value, self.sound_muffler.mute_pistons),
            "muffler.carts" => self.sound_muffler.mute_minecarts = flag(value, self.sound_muffler.mute_minecarts),
            "muffler.villagers" => self.sound_muffler.mute_villagers = flag(value, self.sound_muffler.mute_villagers),
            "muffler.pct" => if let Ok(v) = value.parse::<u8>() { self.sound_muffler.muffle_pct = v.min(100); },

            "portal.enabled" => self.portal_calc.enabled = flag(value, self.portal_calc.enabled),
            "portal.pill" => self.portal_calc.show_hud_pill = flag(value, self.portal_calc.show_hud_pill),
            "portal.radius" => self.portal_calc.show_link_radius = flag(value, self.portal_calc.show_link_radius),
            "portal.corner" => self.portal_calc.corner = Corner::parse(value, self.portal_calc.corner),

            "compass.enabled" => self.compass_hud.enabled = flag(value, self.compass_hud.enabled),
            "compass.cardinals" => self.compass_hud.show_cardinals = flag(value, self.compass_hud.show_cardinals),
            "compass.waypoints" => self.compass_hud.show_waypoints = flag(value, self.compass_hud.show_waypoints),
            "compass.death" => self.compass_hud.show_death_marker = flag(value, self.compass_hud.show_death_marker),
            "compass.players" => self.compass_hud.show_players = flag(value, self.compass_hud.show_players),
            "compass.width" => if let Ok(v) = value.parse::<u8>() { self.compass_hud.width_pct = v.clamp(35, 85); },

            "safewalk.enabled" => self.safe_walk.enabled = flag(value, self.safe_walk.enabled),
            "safewalk.edge" => self.safe_walk.edge_stop = flag(value, self.safe_walk.edge_stop),
            "safewalk.blocksOnly" => self.safe_walk.only_with_blocks = flag(value, self.safe_walk.only_with_blocks),
            "safewalk.afk" => self.safe_walk.anti_afk = flag(value, self.safe_walk.anti_afk),
            "safewalk.afkSecs" => if let Ok(v) = value.parse::<u8>() { self.safe_walk.afk_interval_secs = v.clamp(15, 120); },

            "spawnst.enabled" => self.respawn_status.enabled = flag(value, self.respawn_status.enabled),
            "spawnst.coords" => self.respawn_status.show_spawn_coords = flag(value, self.respawn_status.show_spawn_coords),
            "spawnst.warn" => self.respawn_status.warn_obstructed = flag(value, self.respawn_status.warn_obstructed),
            "spawnst.anchor" => self.respawn_status.show_anchor_charges = flag(value, self.respawn_status.show_anchor_charges),
            "spawnst.corner" => self.respawn_status.corner = Corner::parse(value, self.respawn_status.corner),

            "sbtweak.enabled" => self.scoreboard_tweaks.enabled = flag(value, self.scoreboard_tweaks.enabled),
            "sbtweak.noNums" => self.scoreboard_tweaks.hide_numbers = flag(value, self.scoreboard_tweaks.hide_numbers),
            "sbtweak.hideSb" => self.scoreboard_tweaks.hide_scoreboard = flag(value, self.scoreboard_tweaks.hide_scoreboard),
            "sbtweak.compactBoss" => self.scoreboard_tweaks.compact_bossbars = flag(value, self.scoreboard_tweaks.compact_bossbars),
            "sbtweak.hideBoss" => self.scoreboard_tweaks.hide_bossbars = flag(value, self.scoreboard_tweaks.hide_bossbars),
            "sbtweak.scale" => if let Ok(v) = value.parse::<u8>() { self.scoreboard_tweaks.scale_pct = v.clamp(60, 120); },

            "signst.enabled" => self.sign_stand_helper.enabled = flag(value, self.sign_stand_helper.enabled),
            "signst.copy" => self.sign_stand_helper.auto_copy_last_sign = flag(value, self.sign_stand_helper.auto_copy_last_sign),
            "signst.paste" => self.sign_stand_helper.quick_paste_button = flag(value, self.sign_stand_helper.quick_paste_button),
            "signst.stand" => self.sign_stand_helper.armor_stand_presets = flag(value, self.sign_stand_helper.armor_stand_presets),

            "dynlight.enabled" => self.dynamic_lights.enabled = flag(value, self.dynamic_lights.enabled),
            "dynlight.held" => self.dynamic_lights.held_items = flag(value, self.dynamic_lights.held_items),
            "dynlight.dropped" => self.dynamic_lights.dropped_items = flag(value, self.dynamic_lights.dropped_items),
            "dynlight.burn" => self.dynamic_lights.burning_entities = flag(value, self.dynamic_lights.burning_entities),
            "dynlight.minLvl" => if let Ok(v) = value.parse::<u8>() { self.dynamic_lights.min_level = v.clamp(1, 14); },
            _ => self.parse_extra_mods(k, value),
        }
    }

    fn parse_extra_mods(&mut self, k: &str, value: &str) {
        if let Some(rest) = k.strip_prefix("bind.") {
            let mut parts = rest.splitn(2, '.');
            if let (Some(mod_key), Some(field)) = (parts.next(), parts.next())
                && let Some(meta) = VOLT_MODS.iter().find(|m| m.key == mod_key)
                && let Some(slot) = self.binds.0.get_mut(usize::from(meta.id))
            {
                match field {
                    "toggle" => slot.toggle_key = value.trim().to_owned(),
                    "use" => slot.use_key = value.trim().to_owned(),
                    "mode" => slot.mode = VoltKeybindMode::parse(value, slot.mode),
                    _ => {}
                }
            }
            return;
        }
        match k {
            "lowshield.enabled" => self.low_shield.enabled = flag(value, self.low_shield.enabled),
            "lowshield.shieldDrop" => if let Ok(v) = value.parse::<u8>() { self.low_shield.shield_drop_pct = v.min(80); },
            "lowshield.offDrop" => if let Ok(v) = value.parse::<u8>() { self.low_shield.offhand_drop_pct = v.min(80); },
            "lowshield.mainDrop" => if let Ok(v) = value.parse::<u8>() { self.low_shield.mainhand_drop_pct = v.min(60); },
            "lowshield.noTilt" => self.low_shield.hide_blocking_tilt = flag(value, self.low_shield.hide_blocking_tilt),

            "cleanview.enabled" => self.clean_view.enabled = flag(value, self.clean_view.enabled),
            "cleanview.selfPot" => self.clean_view.hide_self_potions = flag(value, self.clean_view.hide_self_potions),
            "cleanview.beacons" => self.clean_view.hide_ambient_beacons = flag(value, self.clean_view.hide_ambient_beacons),
            "cleanview.enemies" => self.clean_view.keep_enemy_particles = flag(value, self.clean_view.keep_enemy_particles),

            "tagsplus.enabled" => self.name_tags_plus.enabled = flag(value, self.name_tags_plus.enabled),
            "tagsplus.own" => self.name_tags_plus.show_own_nametag = flag(value, self.name_tags_plus.show_own_nametag),
            "tagsplus.dist" => self.name_tags_plus.show_distance = flag(value, self.name_tags_plus.show_distance),
            "tagsplus.hp" => self.name_tags_plus.show_hp_tag = flag(value, self.name_tags_plus.show_hp_tag),
            "tagsplus.noFade" => self.name_tags_plus.no_sneak_fade = flag(value, self.name_tags_plus.no_sneak_fade),

            "hitcol.enabled" => self.hit_color.enabled = flag(value, self.hit_color.enabled),
            "hitcol.color" => self.hit_color.color = ShieldColorPreset::parse(value, self.hit_color.color),
            "hitcol.pct" => if let Ok(v) = value.parse::<u8>() { self.hit_color.intensity_pct = v.min(100); },
            "hitcol.armor" => self.hit_color.flash_armor = flag(value, self.hit_color.flash_armor),

            "critring.enabled" => self.crit_ring.enabled = flag(value, self.crit_ring.enabled),
            "critring.badge" => self.crit_ring.show_crit_badge = flag(value, self.crit_ring.show_crit_badge),
            "critring.bar" => self.crit_ring.show_cooldown_bar = flag(value, self.crit_ring.show_cooldown_bar),
            "critring.color" => self.crit_ring.crit_color = ShieldColorPreset::parse(value, self.crit_ring.crit_color),

            "bowchg.enabled" => self.bow_charge.enabled = flag(value, self.bow_charge.enabled),
            "bowchg.bar" => self.bow_charge.show_pct_bar = flag(value, self.bow_charge.show_pct_bar),
            "bowchg.chime" => self.bow_charge.full_draw_chime = flag(value, self.bow_charge.full_draw_chime),
            "bowchg.autoRel" => self.bow_charge.auto_release_full = flag(value, self.bow_charge.auto_release_full),
            "bowchg.corner" => self.bow_charge.corner = Corner::parse(value, self.bow_charge.corner),

            "cdhud.enabled" => self.cooldown_hud.enabled = flag(value, self.cooldown_hud.enabled),
            "cdhud.pearls" => self.cooldown_hud.track_pearls = flag(value, self.cooldown_hud.track_pearls),
            "cdhud.wind" => self.cooldown_hud.track_wind_charge = flag(value, self.cooldown_hud.track_wind_charge),
            "cdhud.chorus" => self.cooldown_hud.track_chorus_gapple = flag(value, self.cooldown_hud.track_chorus_gapple),
            "cdhud.corner" => self.cooldown_hud.corner = Corner::parse(value, self.cooldown_hud.corner),

            "reachbox.enabled" => self.reach_box.enabled = flag(value, self.reach_box.enabled),
            "reachbox.box" => self.reach_box.draw_target_box = flag(value, self.reach_box.draw_target_box),
            "reachbox.ring" => self.reach_box.draw_reach_ring = flag(value, self.reach_box.draw_reach_ring),
            "reachbox.crit" => self.reach_box.color_on_crit = flag(value, self.reach_box.color_on_crit),
            "reachbox.color" => self.reach_box.color = ShieldColorPreset::parse(value, self.reach_box.color),

            "minimap.enabled" | "minimap.size" | "minimap.zoom" | "minimap.mobs" | "minimap.players" | "minimap.wp" | "minimap.corner" => {}

            "pothud.enabled" => self.potion_hud.enabled = flag(value, self.potion_hud.enabled),
            "pothud.timers" => self.potion_hud.show_timers = flag(value, self.potion_hud.show_timers),
            "pothud.blink" => self.potion_hud.blink_under_10s = flag(value, self.potion_hud.blink_under_10s),
            "pothud.compact" => self.potion_hud.compact_pills = flag(value, self.potion_hud.compact_pills),
            "pothud.corner" => self.potion_hud.corner = Corner::parse(value, self.potion_hud.corner),

            "durwarn.enabled" => self.durability_warn.enabled = flag(value, self.durability_warn.enabled),
            "durwarn.pct" => if let Ok(v) = value.parse::<u8>() { self.durability_warn.warn_pct = v.clamp(5, 50); },
            "durwarn.armor" => self.durability_warn.check_armor = flag(value, self.durability_warn.check_armor),
            "durwarn.held" => self.durability_warn.check_held = flag(value, self.durability_warn.check_held),
            "durwarn.chime" => self.durability_warn.play_chime = flag(value, self.durability_warn.play_chime),

            "biometitle.enabled" => self.biome_title.enabled = flag(value, self.biome_title.enabled),
            "biometitle.biome" => self.biome_title.show_biome_changes = flag(value, self.biome_title.show_biome_changes),
            "biometitle.dim" => self.biome_title.show_dimension_changes = flag(value, self.biome_title.show_dimension_changes),
            "biometitle.secs" => if let Ok(v) = value.parse::<u8>() { self.biome_title.display_secs = v.clamp(1, 8); },
            "biometitle.coords" => self.biome_title.show_sub_coords = flag(value, self.biome_title.show_sub_coords),

            "xptrack.enabled" => self.xp_tracker.enabled = flag(value, self.xp_tracker.enabled),
            "xptrack.next" => self.xp_tracker.show_next_level_xp = flag(value, self.xp_tracker.show_next_level_xp),
            "xptrack.mend" => self.xp_tracker.show_mending_summary = flag(value, self.xp_tracker.show_mending_summary),
            "xptrack.corner" => self.xp_tracker.corner = Corner::parse(value, self.xp_tracker.corner),

            "invhud.enabled" => self.inventory_hud.enabled = flag(value, self.inventory_hud.enabled),
            "invhud.grid" => self.inventory_hud.show_3x9_grid = flag(value, self.inventory_hud.show_3x9_grid),
            "invhud.free" => self.inventory_hud.show_free_slots = flag(value, self.inventory_hud.show_free_slots),
            "invhud.opacity" => if let Ok(v) = value.parse::<u8>() { self.inventory_hud.bg_opacity_pct = v.clamp(20, 100); },
            "invhud.corner" => self.inventory_hud.corner = Corner::parse(value, self.inventory_hud.corner),

            "tabping.enabled" => self.tab_ping.enabled = flag(value, self.tab_ping.enabled),
            "tabping.ms" => self.tab_ping.show_ping_ms = flag(value, self.tab_ping.show_ping_ms),
            "tabping.cnt" => self.tab_ping.show_player_count = flag(value, self.tab_ping.show_player_count),
            "tabping.dim" => self.tab_ping.show_dim_tag = flag(value, self.tab_ping.show_dim_tag),
            "tabping.corner" => self.tab_ping.corner = Corner::parse(value, self.tab_ping.corner),

            "moon.enabled" => self.moon_phase.enabled = flag(value, self.moon_phase.enabled),
            "moon.phase" => self.moon_phase.show_moon_phase = flag(value, self.moon_phase.show_moon_phase),
            "moon.slime" => self.moon_phase.show_slime_rating = flag(value, self.moon_phase.show_slime_rating),
            "moon.insomnia" => self.moon_phase.show_insomnia_warn = flag(value, self.moon_phase.show_insomnia_warn),
            "moon.corner" => self.moon_phase.corner = Corner::parse(value, self.moon_phase.corner),

            "sstats.enabled" => self.session_stats.enabled = flag(value, self.session_stats.enabled),
            "sstats.kills" => self.session_stats.show_kills = flag(value, self.session_stats.show_kills),
            "sstats.blocks" => self.session_stats.show_blocks = flag(value, self.session_stats.show_blocks),
            "sstats.dist" => self.session_stats.show_distance = flag(value, self.session_stats.show_distance),
            "sstats.corner" => self.session_stats.corner = Corner::parse(value, self.session_stats.corner),

            "camover.enabled" | "camover.strafe" | "camover.turn" | "camover.elytra" | "camover.fov" => {}

            "wsound.enabled" => self.weather_sound.enabled = flag(value, self.weather_sound.enabled),
            "wsound.flash" => self.weather_sound.suppress_lightning_flash = flag(value, self.weather_sound.suppress_lightning_flash),
            "wsound.rain" => self.weather_sound.limit_rain_splashes = flag(value, self.weather_sound.limit_rain_splashes),
            "wsound.thunder" => self.weather_sound.mute_thunder_shake = flag(value, self.weather_sound.mute_thunder_shake),

            "glint.enabled" => self.custom_glint.enabled = flag(value, self.custom_glint.enabled),
            "glint.preset" => self.custom_glint.preset = GlintColorPreset::parse(value, self.custom_glint.preset),
            "glint.speed" => if let Ok(v) = value.parse::<u8>() { self.custom_glint.speed_pct = v.clamp(25, 250); },
            "glint.strength" => if let Ok(v) = value.parse::<u8>() { self.custom_glint.strength_pct = v.min(100); },
            "glint.compact" => self.custom_glint.compact_item_scale = flag(value, self.custom_glint.compact_item_scale),

            "caves.enabled" => self.clear_caves.enabled = flag(value, self.clear_caves.enabled),
            "caves.void" => self.clear_caves.remove_void_fog = flag(value, self.clear_caves.remove_void_fog),
            "caves.nv" => self.clear_caves.smooth_night_vision = flag(value, self.clear_caves.smooth_night_vision),
            "caves.lava" => self.clear_caves.boost_lava_clarity = flag(value, self.clear_caves.boost_lava_clarity),

            "phighlight.enabled" => self.player_highlight.enabled = flag(value, self.player_highlight.enabled),
            "phighlight.players" => self.player_highlight.highlight_players = flag(value, self.player_highlight.highlight_players),
            "phighlight.hostiles" => self.player_highlight.highlight_hostiles = flag(value, self.player_highlight.highlight_hostiles),
            "phighlight.range" => if let Ok(v) = value.parse::<u8>() { self.player_highlight.range = v.clamp(8, 64); },
            "phighlight.color" => self.player_highlight.color = ShieldColorPreset::parse(value, self.player_highlight.color),

            "ruler.enabled" => self.litematica_grid.enabled = flag(value, self.litematica_grid.enabled),
            "ruler.hud" => self.litematica_grid.show_volume_hud = flag(value, self.litematica_grid.show_volume_hud),
            "ruler.box" => self.litematica_grid.show_3d_box = flag(value, self.litematica_grid.show_3d_box),
            "ruler.grid" => self.litematica_grid.show_grid_lines = flag(value, self.litematica_grid.show_grid_lines),
            "ruler.color" => self.litematica_grid.color = ShieldColorPreset::parse(value, self.litematica_grid.color),

            "dhist.enabled" => self.death_history.enabled = flag(value, self.death_history.enabled),
            "dhist.keep" => if let Ok(v) = value.parse::<u8>() { self.death_history.keep_count = v.clamp(1, 10); },
            "dhist.home" => self.death_history.show_home_pill = flag(value, self.death_history.show_home_pill),
            "dhist.chat" => self.death_history.auto_log_chat = flag(value, self.death_history.auto_log_chat),
            "dhist.corner" => self.death_history.corner = Corner::parse(value, self.death_history.corner),

            "cpillar.enabled" => self.chunk_pillars.enabled = flag(value, self.chunk_pillars.enabled),
            "cpillar.pillars" => self.chunk_pillars.draw_corner_pillars = flag(value, self.chunk_pillars.draw_corner_pillars),
            "cpillar.biomes" => self.chunk_pillars.draw_biome_borders = flag(value, self.chunk_pillars.draw_biome_borders),
            "cpillar.height" => if let Ok(v) = value.parse::<u8>() { self.chunk_pillars.pillar_height = v.clamp(16, 128); },

            "otorch.enabled" => self.offhand_torch.enabled = flag(value, self.offhand_torch.enabled),
            "otorch.auto" => self.offhand_torch.auto_place_dark = flag(value, self.offhand_torch.auto_place_dark),
            "otorch.light" => if let Ok(v) = value.parse::<u8>() { self.offhand_torch.light_threshold = v.clamp(1, 7); },
            "otorch.warn" => self.offhand_torch.notify_low_torches = flag(value, self.offhand_torch.notify_low_torches),

            "cmention.enabled" => self.chat_mention.enabled = flag(value, self.chat_mention.enabled),
            "cmention.user" => self.chat_mention.alert_on_username = flag(value, self.chat_mention.alert_on_username),
            "cmention.tpa" => self.chat_mention.alert_on_tpa_msg = flag(value, self.chat_mention.alert_on_tpa_msg),
            "cmention.chime" => self.chat_mention.play_ping_chime = flag(value, self.chat_mention.play_ping_chime),
            "cmention.popup" => self.chat_mention.show_popup_banner = flag(value, self.chat_mention.show_popup_banner),

            "dropguard.enabled" => self.drop_guard.enabled = flag(value, self.drop_guard.enabled),
            "dropguard.tools" => self.drop_guard.protect_tools_weapons = flag(value, self.drop_guard.protect_tools_weapons),
            "dropguard.totems" => self.drop_guard.protect_totems = flag(value, self.drop_guard.protect_totems),
            "dropguard.slots" => self.drop_guard.lock_slots_1_to_3 = flag(value, self.drop_guard.lock_slots_1_to_3),
            "dropguard.dbl" => self.drop_guard.double_tap_bypass = flag(value, self.drop_guard.double_tap_bypass),

            "restock.enabled" => self.hotbar_restock.enabled = flag(value, self.hotbar_restock.enabled),
            "restock.blocks" => self.hotbar_restock.restock_blocks = flag(value, self.hotbar_restock.restock_blocks),
            "restock.util" => self.hotbar_restock.restock_pearls_fireworks = flag(value, self.hotbar_restock.restock_pearls_fireworks),
            "restock.food" => self.hotbar_restock.restock_food = flag(value, self.hotbar_restock.restock_food),
            "restock.toast" => self.hotbar_restock.show_toast = flag(value, self.hotbar_restock.show_toast),

            "photo.enabled" => self.photo_mode.enabled = flag(value, self.photo_mode.enabled),
            "photo.hideHud" => self.photo_mode.hide_all_hud = flag(value, self.photo_mode.hide_all_hud),
            "photo.lock" => self.photo_mode.lock_freecam_pose = flag(value, self.photo_mode.lock_freecam_pose),
            "photo.guides" => self.photo_mode.show_frame_guides = flag(value, self.photo_mode.show_frame_guides),

            "jei.enabled" => self.jei.enabled = flag(value, self.jei.enabled),
            "jei.inInv" => self.jei.show_in_inventory = flag(value, self.jei.show_in_inventory),
            "jei.ids" => self.jei.show_item_ids = flag(value, self.jei.show_item_ids),
            "jei.cook" => self.jei.show_cooking_recipes = flag(value, self.jei.show_cooking_recipes),
            "jei.compact" => self.jei.compact_grid = flag(value, self.jei.compact_grid),

            "timelapse.enabled" => self.cinematic_timelapse.enabled = flag(value, self.cinematic_timelapse.enabled),
            "timelapse.delay" => if let Ok(v) = value.parse::<u8>() { self.cinematic_timelapse.start_delay_secs = v.min(60); },
            "timelapse.path" => self.cinematic_timelapse.path_mode = TimelapsePathMode::parse(value, self.cinematic_timelapse.path_mode),
            "timelapse.speed" => if let Ok(v) = value.parse::<u8>() { self.cinematic_timelapse.orbit_speed_deg = v.min(30); },
            "timelapse.hideHud" => self.cinematic_timelapse.hide_hud = flag(value, self.cinematic_timelapse.hide_hud),
            "timelapse.countdown" => self.cinematic_timelapse.show_countdown = flag(value, self.cinematic_timelapse.show_countdown),

            "blocksOutline.enabled" => self.blocks_outline.enabled = flag(value, self.blocks_outline.enabled),
            "blocksOutline.color" => if let Ok(v) = value.parse::<u32>() { self.blocks_outline.color_rgb = v & 0x00FF_FFFF; },
            "blocksOutline.alpha" => if let Ok(v) = value.parse::<u8>() { self.blocks_outline.alpha_pct = v.clamp(10, 100); },
            "blocksOutline.width" => if let Ok(v) = value.parse::<u8>() { self.blocks_outline.line_width = v.clamp(1, 10); },
            "blocksOutline.rainbow" => self.blocks_outline.rainbow = flag(value, self.blocks_outline.rainbow),
            "blocksOutline.rainbowSpeed" => if let Ok(v) = value.parse::<u8>() { self.blocks_outline.rainbow_speed = v.clamp(1, 5); },
            "blocksOutline.pulse" => self.blocks_outline.pulse = flag(value, self.blocks_outline.pulse),
            "blocksOutline.fill" => self.blocks_outline.fill_overlay = flag(value, self.blocks_outline.fill_overlay),
            "blocksOutline.fillAlpha" => if let Ok(v) = value.parse::<u8>() { self.blocks_outline.fill_alpha_pct = v.clamp(5, 80); },
            "blocksOutline.fillMatch" => self.blocks_outline.fill_match_outline = flag(value, self.blocks_outline.fill_match_outline),
            "blocksOutline.fillColor" => if let Ok(v) = value.parse::<u32>() { self.blocks_outline.fill_color_rgb = v & 0x00FF_FFFF; },
            "blocksOutline.seeThrough" => self.blocks_outline.see_through = flag(value, self.blocks_outline.see_through),
            "blocksOutline.highContrast" => self.blocks_outline.high_contrast_backing = flag(value, self.blocks_outline.high_contrast_backing),
            "blocksOutline.inflate" => if let Ok(v) = value.parse::<u8>() { self.blocks_outline.inflate_pct = v.min(6); },
            _ => {}
        }
    }

    pub fn serialize_into(&self, s: &mut String) {
        let a = &self.appleskin;
        let _ = writeln!(s, "volt.appleskin.enabled:{}", bool_s(a.enabled));
        let _ = writeln!(s, "volt.appleskin.saturation:{}", bool_s(a.show_saturation_overlay));
        let _ = writeln!(s, "volt.appleskin.exhaustion:{}", bool_s(a.show_exhaustion_underlay));
        let _ = writeln!(s, "volt.appleskin.heldPreview:{}", bool_s(a.show_held_food_preview));
        let _ = writeln!(s, "volt.appleskin.regen:{}", bool_s(a.show_health_regen_overlay));
        let _ = writeln!(s, "volt.appleskin.tooltip:{}", bool_s(a.show_food_tooltip));
        let _ = writeln!(s, "volt.appleskin.tooltipAlways:{}", bool_s(a.always_show_tooltip));
        let _ = writeln!(s, "volt.appleskin.debug:{}", bool_s(a.show_debug_stats));
        let _ = writeln!(s, "volt.appleskin.fullBar:{}", bool_s(a.highlight_full_bar));
        let _ = writeln!(s, "volt.appleskin.pulse:{}", a.pulse_speed_pct);

        let c = &self.crystal_optimizer;
        let _ = writeln!(s, "volt.crystal.enabled:{}", bool_s(c.enabled));
        let _ = writeln!(s, "volt.crystal.instantBreak:{}", bool_s(c.instant_break));
        let _ = writeln!(s, "volt.crystal.weakness:{}", bool_s(c.respect_weakness));
        let _ = writeln!(s, "volt.crystal.fastPlace:{}", bool_s(c.fast_place));
        let _ = writeln!(s, "volt.crystal.timeoutMs:{}", c.timeout_ms);

        let sh = &self.shield_status;
        let _ = writeln!(s, "volt.shield.enabled:{}", bool_s(sh.enabled));
        let _ = writeln!(s, "volt.shield.enabledColor:{}", sh.enabled_color.name());
        let _ = writeln!(s, "volt.shield.disabledColor:{}", sh.disabled_color.name());
        let _ = writeln!(s, "volt.shield.interpolate:{}", bool_s(sh.interpolate_colors));
        let _ = writeln!(s, "volt.shield.mode:{}", sh.display_mode.name());
        let _ = writeln!(s, "volt.shield.sound:{}", bool_s(sh.disable_sound));

        let u = &self.uku_hud;
        let _ = writeln!(s, "volt.uku.enabled:{}", bool_s(u.enabled));
        let _ = writeln!(s, "volt.uku.armor:{}", bool_s(u.armor_enabled));
        let _ = writeln!(s, "volt.uku.potion:{}", bool_s(u.potion_enabled));
        let _ = writeln!(s, "volt.uku.armorAnchor:{}", u.armor_anchor.name());
        let _ = writeln!(s, "volt.uku.armorSide:{}", u.armor_side.name());
        let _ = writeln!(s, "volt.uku.style:{}", u.box_style.name());
        let _ = writeln!(s, "volt.uku.durability:{}", u.durability_display.name());
        let _ = writeln!(s, "volt.uku.offhand:{}", bool_s(u.show_offhand));
        let _ = writeln!(s, "volt.uku.hideEmpty:{}", bool_s(u.hide_empty));
        let _ = writeln!(s, "volt.uku.lowPct:{}", u.low_durability_pct);
        let _ = writeln!(s, "volt.uku.potionAnchor:{}", u.potion_anchor.name());
        let _ = writeln!(s, "volt.uku.potionCompact:{}", bool_s(u.potion_compact));

        // Only serialize non-default modules to keep horizon-extras.txt concise & fast.
        if self.freelook != FreelookConfig::default() {
            let x = &self.freelook;
            let _ = writeln!(s, "volt.freelook.enabled:{}\nvolt.freelook.toggle:{}\nvolt.freelook.invertY:{}\nvolt.freelook.sens:{}\nvolt.freelook.persp:{}", bool_s(x.enabled), bool_s(x.toggle_mode), bool_s(x.invert_y), x.sensitivity_pct, bool_s(x.preserve_perspective));
        }
        if self.trajectories != TrajectoriesConfig::default() {
            let x = &self.trajectories;
            let _ = writeln!(s, "volt.traj.enabled:{}\nvolt.traj.bow:{}\nvolt.traj.pearl:{}\nvolt.traj.trident:{}\nvolt.traj.wind:{}\nvolt.traj.potion:{}\nvolt.traj.box:{}\nvolt.traj.color:{}\nvolt.traj.width:{}", bool_s(x.enabled), bool_s(x.show_bow), bool_s(x.show_pearl), bool_s(x.show_trident), bool_s(x.show_wind_charge), bool_s(x.show_potion), bool_s(x.show_impact_box), x.color.name(), x.line_width);
        }
        if self.health_indicators != HealthIndicatorsConfig::default() {
            let x = &self.health_indicators;
            let _ = writeln!(s, "volt.hpind.enabled:{}\nvolt.hpind.bars:{}\nvolt.hpind.popups:{}\nvolt.hpind.crits:{}\nvolt.hpind.players:{}\nvolt.hpind.hostiles:{}\nvolt.hpind.range:{}\nvolt.hpind.scale:{}", bool_s(x.enabled), bool_s(x.show_bars), bool_s(x.show_damage_popups), bool_s(x.highlight_crits), bool_s(x.show_players), bool_s(x.show_hostiles), x.max_range, x.bar_scale);
        }
        if self.item_scroller != ItemScrollerConfig::default() {
            let x = &self.item_scroller;
            let _ = writeln!(s, "volt.invscroll.enabled:{}\nvolt.invscroll.autoRefill:{}\nvolt.invscroll.totem:{}\nvolt.invscroll.sort:{}\nvolt.invscroll.wheel:{}\nvolt.invscroll.tools:{}", bool_s(x.enabled), bool_s(x.auto_refill_hotbar), bool_s(x.quick_offhand_totem), bool_s(x.sort_shortcut), bool_s(x.scroll_one_by_one), bool_s(x.preserve_tools));
        }
        if self.tweakeroo != TweakerooConfig::default() {
            let x = &self.tweakeroo;
            let _ = writeln!(s, "volt.tweak.enabled:{}\nvolt.tweak.fastPlace:{}\nvolt.tweak.lightOverlay:{}\nvolt.tweak.autoTool:{}\nvolt.tweak.silkGuard:{}\nvolt.tweak.noPortalClose:{}\nvolt.tweak.lightRange:{}", bool_s(x.enabled), bool_s(x.fast_block_place), bool_s(x.light_overlay), bool_s(x.auto_tool), bool_s(x.silk_touch_guard), bool_s(x.no_portal_gui_close), x.light_range);
        }
        if self.elytra_hud != ElytraMaceConfig::default() {
            let x = &self.elytra_hud;
            let _ = writeln!(s, "volt.elytraHud.enabled:{}\nvolt.elytraHud.alt:{}\nvolt.elytraHud.vspd:{}\nvolt.elytraHud.pitch:{}\nvolt.elytraHud.fireworks:{}\nvolt.elytraHud.warnPct:{}\nvolt.elytraHud.mace:{}", bool_s(x.enabled), bool_s(x.show_altitude), bool_s(x.show_vertical_speed), bool_s(x.show_pitch_gauge), bool_s(x.show_fireworks), x.chestplate_warn_pct, bool_s(x.show_mace_damage));
        }
        if self.waypoints != WaypointsConfig::default() {
            let x = &self.waypoints;
            let _ = writeln!(s, "volt.waypoints.enabled:{}\nvolt.waypoints.autoDeath:{}\nvolt.waypoints.beacons:{}\nvolt.waypoints.hud:{}\nvolt.waypoints.maxDist:{}\nvolt.waypoints.beamH:{}", bool_s(x.enabled), bool_s(x.auto_death_waypoint), bool_s(x.show_beacons), bool_s(x.show_hud_distances), x.max_distance, x.beam_height);
        }
        if self.totem_counter != TotemCounterConfig::default() {
            let x = &self.totem_counter;
            let _ = writeln!(s, "volt.totemHud.enabled:{}\nvolt.totemHud.badge:{}\nvolt.totemHud.lowHp:{}\nvolt.totemHud.hearts:{}\nvolt.totemHud.warnEmpty:{}\nvolt.totemHud.corner:{}", bool_s(x.enabled), bool_s(x.show_hud_badge), bool_s(x.low_hp_alert), x.low_hp_hearts, bool_s(x.warn_when_empty), x.corner.name());
        }
        if self.cps_combo != CpsComboConfig::default() {
            let x = &self.cps_combo;
            let _ = writeln!(s, "volt.cps.enabled:{}\nvolt.cps.lmb:{}\nvolt.cps.rmb:{}\nvolt.cps.reach:{}\nvolt.cps.combo:{}\nvolt.cps.corner:{}\nvolt.cps.compact:{}", bool_s(x.enabled), bool_s(x.show_lmb_cps), bool_s(x.show_rmb_cps), bool_s(x.show_reach), bool_s(x.show_combo), x.corner.name(), bool_s(x.compact));
        }
        if self.keystrokes != KeystrokesConfig::default() {
            let x = &self.keystrokes;
            let _ = writeln!(s, "volt.keystrokes.enabled:{}\nvolt.keystrokes.cps:{}\nvolt.keystrokes.space:{}\nvolt.keystrokes.sprintSneak:{}\nvolt.keystrokes.corner:{}\nvolt.keystrokes.chroma:{}\nvolt.keystrokes.opacity:{}", bool_s(x.enabled), bool_s(x.show_cps), bool_s(x.show_space), bool_s(x.show_sprint_sneak), x.corner.name(), bool_s(x.chroma), x.bg_opacity_pct);
        }
        if self.pvp_visuals != PvpVisualsConfig::default() {
            let x = &self.pvp_visuals;
            let _ = writeln!(s, "volt.pvpvis.enabled:{}\nvolt.pvpvis.lowFire:{}\nvolt.pvpvis.noPumpkin:{}\nvolt.pvpvis.noPortal:{}\nvolt.pvpvis.hurtCam:{}\nvolt.pvpvis.lowBob:{}\nvolt.pvpvis.glint:{}", bool_s(x.enabled), x.low_fire_pct, bool_s(x.hide_pumpkin), bool_s(x.hide_portal_overlay), x.hurt_cam_pct, bool_s(x.reduced_bobbing), x.glint_pct);
        }
        if self.crosshair_tweaks != CrosshairTweaksConfig::default() {
            let x = &self.crosshair_tweaks;
            let _ = writeln!(s, "volt.crosshair.enabled:{}\nvolt.crosshair.hitMarker:{}\nvolt.crosshair.style:{}\nvolt.crosshair.color:{}\nvolt.crosshair.size:{}\nvolt.crosshair.thick:{}\nvolt.crosshair.dynGap:{}", bool_s(x.enabled), bool_s(x.hit_marker), x.style.name(), x.color.name(), x.size, x.thickness, bool_s(x.dynamic_gap));
        }
        if self.zoom != ZoomModConfig::default() {
            let x = &self.zoom;
            let _ = writeln!(s, "volt.zoom.enabled:{}\nvolt.zoom.level:{}\nvolt.zoom.smooth:{}\nvolt.zoom.sens:{}\nvolt.zoom.hideHand:{}", bool_s(x.enabled), x.zoom_level, bool_s(x.smooth), bool_s(x.scale_sensitivity), bool_s(x.hide_hand));
        }
        if self.auto_sprint != AutoSprintConfig::default() {
            let x = &self.auto_sprint;
            let _ = writeln!(s, "volt.sprint.enabled:{}\nvolt.sprint.badge:{}\nvolt.sprint.sneak:{}\nvolt.sprint.water:{}\nvolt.sprint.corner:{}", bool_s(x.enabled), bool_s(x.show_indicator), bool_s(x.toggle_sneak), bool_s(x.sprint_in_water), x.corner.name());
        }
        if self.fullbright != FullbrightConfig::default() {
            let x = &self.fullbright;
            let _ = writeln!(s, "volt.fullbright.enabled:{}\nvolt.fullbright.gamma:{}\nvolt.fullbright.cavesOnly:{}\nvolt.fullbright.smooth:{}", bool_s(x.enabled), x.gamma_pct, bool_s(x.caves_only), bool_s(x.smooth_fade));
        }
        if self.coords_hud != CoordsHudConfig::default() {
            let x = &self.coords_hud;
            let _ = writeln!(s, "volt.coords.enabled:{}\nvolt.coords.xyz:{}\nvolt.coords.nether:{}\nvolt.coords.facing:{}\nvolt.coords.biome:{}\nvolt.coords.corner:{}\nvolt.coords.compact:{}", bool_s(x.enabled), bool_s(x.show_xyz), bool_s(x.show_nether_equiv), bool_s(x.show_facing), bool_s(x.show_biome), x.corner.name(), bool_s(x.compact));
        }
        if self.telemetry_hud != TelemetryHudConfig::default() {
            let x = &self.telemetry_hud;
            let _ = writeln!(s, "volt.telem.enabled:{}\nvolt.telem.fps:{}\nvolt.telem.lows:{}\nvolt.telem.ping:{}\nvolt.telem.bps:{}\nvolt.telem.corner:{}\nvolt.telem.color:{}", bool_s(x.enabled), bool_s(x.show_fps), bool_s(x.show_lows), bool_s(x.show_ping), bool_s(x.show_bps), x.corner.name(), bool_s(x.color_coded));
        }
        if self.item_counters != ItemCountersConfig::default() {
            let x = &self.item_counters;
            let _ = writeln!(s, "volt.itemcnt.enabled:{}\nvolt.itemcnt.arrows:{}\nvolt.itemcnt.pearls:{}\nvolt.itemcnt.wind:{}\nvolt.itemcnt.gapples:{}\nvolt.itemcnt.zero:{}", bool_s(x.enabled), bool_s(x.count_arrows), bool_s(x.count_pearls), bool_s(x.count_wind_charges), bool_s(x.count_gapples), bool_s(x.show_zero));
        }
        if self.clock_day_hud != ClockDayConfig::default() {
            let x = &self.clock_day_hud;
            let _ = writeln!(s, "volt.clock.enabled:{}\nvolt.clock.day:{}\nvolt.clock.24h:{}\nvolt.clock.light:{}\nvolt.clock.safe:{}\nvolt.clock.corner:{}", bool_s(x.enabled), bool_s(x.show_day), bool_s(x.use_24h), bool_s(x.show_light_level), bool_s(x.show_spawn_safe), x.corner.name());
        }
        if self.chunk_hitboxes != ChunkHitboxConfig::default() {
            let x = &self.chunk_hitboxes;
            let _ = writeln!(s, "volt.chunkhb.enabled:{}\nvolt.chunkhb.chunks:{}\nvolt.chunkhb.hitboxes:{}\nvolt.chunkhb.hideF1:{}\nvolt.chunkhb.range:{}", bool_s(x.enabled), bool_s(x.chunk_borders), bool_s(x.entity_hitboxes), bool_s(x.hide_in_f1), x.chunk_range);
        }
        if self.chat_tweaks != ChatTweaksConfig::default() {
            let x = &self.chat_tweaks;
            let _ = writeln!(s, "volt.chat.enabled:{}\nvolt.chat.ts:{}\nvolt.chat.24h:{}\nvolt.chat.compact:{}\nvolt.chat.history:{}", bool_s(x.enabled), bool_s(x.timestamps), bool_s(x.use_24h), bool_s(x.compact_duplicates), bool_s(x.keep_history));
        }
        if self.freecam != FreecamConfig::default() {
            let x = &self.freecam;
            let _ = writeln!(s, "volt.freecam.enabled:{}\nvolt.freecam.speed:{}\nvolt.freecam.hideHand:{}\nvolt.freecam.marker:{}\nvolt.freecam.noclip:{}", bool_s(x.enabled), x.speed, bool_s(x.hide_hand), bool_s(x.show_player_marker), bool_s(x.noclip_flight));
        }
        if self.radar != RadarConfig::default() {
            let x = &self.radar;
            let _ = writeln!(s, "volt.radar.enabled:{}\nvolt.radar.items:{}\nvolt.radar.chests:{}\nvolt.radar.spawners:{}\nvolt.radar.pets:{}\nvolt.radar.range:{}\nvolt.radar.dist:{}", bool_s(x.enabled), bool_s(x.highlight_items), bool_s(x.highlight_chests), bool_s(x.highlight_spawners), bool_s(x.highlight_pets), x.range, bool_s(x.show_distances));
        }
        if self.auto_reconnect != AutoReconnectConfig::default() {
            let x = &self.auto_reconnect;
            let _ = writeln!(s, "volt.reconn.enabled:{}\nvolt.reconn.delay:{}\nvolt.reconn.attempts:{}\nvolt.reconn.health:{}", bool_s(x.enabled), x.delay_secs, x.max_attempts, bool_s(x.show_server_health));
        }
        if self.chat_macros != ChatMacrosConfig::default() {
            let x = &self.chat_macros;
            let _ = writeln!(s, "volt.macros.enabled:{}\nvolt.macros.autogg:{}\nvolt.macros.ggIdx:{}\nvolt.macros.bar:{}\nvolt.macros.preset:{}", bool_s(x.enabled), bool_s(x.auto_gg), x.gg_message_idx, bool_s(x.show_quick_bar), x.macro_preset_idx);
        }
        if self.tnt_timer != TntTimerConfig::default() {
            let x = &self.tnt_timer;
            let _ = writeln!(s, "volt.tnt.enabled:{}\nvolt.tnt.tntFuse:{}\nvolt.tnt.creeperFuse:{}\nvolt.tnt.halo:{}\nvolt.tnt.crystalRing:{}\nvolt.tnt.anchorRing:{}", bool_s(x.enabled), bool_s(x.show_tnt_fuse), bool_s(x.show_creeper_fuse), bool_s(x.danger_halo), bool_s(x.crystal_ring), bool_s(x.anchor_ring));
        }
        if self.auto_armor != AutoArmorConfig::default() {
            let x = &self.auto_armor;
            let _ = writeln!(s, "volt.autoarmor.enabled:{}\nvolt.autoarmor.pickup:{}\nvolt.autoarmor.prot:{}\nvolt.autoarmor.elytra:{}\nvolt.autoarmor.minDur:{}", bool_s(x.enabled), bool_s(x.auto_on_pickup), bool_s(x.prefer_protection), bool_s(x.respect_elytra), x.min_durability_pct);
        }
        if self.elytra_swap != ElytraSwapConfig::default() {
            let x = &self.elytra_swap;
            let _ = writeln!(s, "volt.elyswap.enabled:{}\nvolt.elyswap.dblJump:{}\nvolt.elyswap.landBack:{}\nvolt.elyswap.netherite:{}", bool_s(x.enabled), bool_s(x.double_jump_swap), bool_s(x.swap_back_on_land), bool_s(x.prefer_netherite));
        }
        if self.slime_mobcap != SlimeMobcapConfig::default() {
            let x = &self.slime_mobcap;
            let _ = writeln!(s, "volt.slime.enabled:{}\nvolt.slime.chunk:{}\nvolt.slime.mobcap:{}\nvolt.slime.spawn:{}\nvolt.slime.corner:{}", bool_s(x.enabled), bool_s(x.show_slime_chunk), bool_s(x.show_mob_count), bool_s(x.show_spawn_check), x.corner.name());
        }
        if self.auto_eat != AutoEatConfig::default() {
            let x = &self.auto_eat;
            let _ = writeln!(s, "volt.autoeat.enabled:{}\nvolt.autoeat.hunger:{}\nvolt.autoeat.hp:{}\nvolt.autoeat.safeOnly:{}\nvolt.autoeat.warn:{}", bool_s(x.enabled), x.hunger_threshold, x.hp_threshold, bool_s(x.avoid_rotten_raw), bool_s(x.low_hunger_alert));
        }
        if self.radial_menu != RadialMenuConfig::default() {
            let x = &self.radial_menu;
            let _ = writeln!(s, "volt.radial.enabled:{}\nvolt.radial.strip:{}\nvolt.radial.pearl:{}\nvolt.radial.gapple:{}\nvolt.radial.firework:{}\nvolt.radial.totem:{}", bool_s(x.enabled), bool_s(x.show_quick_strip), bool_s(x.include_pearl), bool_s(x.include_gapple), bool_s(x.include_firework), bool_s(x.include_totem));
        }
        if self.clear_skies != ClearSkiesConfig::default() {
            let x = &self.clear_skies;
            let _ = writeln!(s, "volt.skies.enabled:{}\nvolt.skies.rain:{}\nvolt.skies.waterFog:{}\nvolt.skies.netherFog:{}\nvolt.skies.terrainFog:{}\nvolt.skies.time:{}", bool_s(x.enabled), bool_s(x.hide_rain_snow), bool_s(x.clear_water_fog), bool_s(x.clear_nether_fog), bool_s(x.clear_terrain_fog), x.time_override.name());
        }
        if self.smart_trade != SmartTradeConfig::default() {
            let x = &self.smart_trade;
            let _ = writeln!(s, "volt.trade.enabled:{}\nvolt.trade.maxEnch:{}\nvolt.trade.cheap:{}\nvolt.trade.maxEm:{}\nvolt.trade.badge:{}", bool_s(x.enabled), bool_s(x.highlight_max_enchants), bool_s(x.highlight_cheap_trades), x.cheap_emerald_max, bool_s(x.show_discount_badge));
        }
        if self.totem_pop_counter != TotemPopCounterConfig::default() {
            let x = &self.totem_pop_counter;
            let _ = writeln!(s, "volt.pops.enabled:{}\nvolt.pops.self:{}\nvolt.pops.enemy:{}\nvolt.pops.sound:{}\nvolt.pops.feed:{}\nvolt.pops.corner:{}", bool_s(x.enabled), bool_s(x.track_self), bool_s(x.track_enemies), bool_s(x.play_alert_sound), bool_s(x.show_chat_feed), x.corner.name());
        }
        if self.auto_respawn != AutoRespawnConfig::default() {
            let x = &self.auto_respawn;
            let _ = writeln!(s, "volt.respawn.enabled:{}\nvolt.respawn.instant:{}\nvolt.respawn.log:{}\nvolt.respawn.wp:{}", bool_s(x.enabled), bool_s(x.instant_respawn), bool_s(x.log_death_coords), bool_s(x.save_death_waypoint));
        }
        if self.shulker_peek != ShulkerPeekConfig::default() {
            let x = &self.shulker_peek;
            let _ = writeln!(s, "volt.shulker.enabled:{}\nvolt.shulker.shulker:{}\nvolt.shulker.bundle:{}\nvolt.shulker.color:{}\nvolt.shulker.always:{}", bool_s(x.enabled), bool_s(x.peek_shulkers), bool_s(x.peek_bundles), bool_s(x.color_by_shulker), bool_s(x.always_show));
        }
        if self.ground_drops != GroundDropsConfig::default() {
            let x = &self.ground_drops;
            let _ = writeln!(s, "volt.drops.enabled:{}\nvolt.drops.beams:{}\nvolt.drops.labels:{}\nvolt.drops.totems:{}\nvolt.drops.range:{}", bool_s(x.enabled), bool_s(x.rare_item_beams), bool_s(x.merge_count_labels), bool_s(x.highlight_totems), x.range);
        }
        if self.entity_culling != EntityCullingConfig::default() {
            let x = &self.entity_culling;
            let _ = writeln!(s, "volt.culling.enabled:{}\nvolt.culling.dist:{}\nvolt.culling.expl:{}\nvolt.culling.smoke:{}\nvolt.culling.swirls:{}\nvolt.culling.back:{}", bool_s(x.enabled), x.cull_distance, bool_s(x.reduce_explosions), bool_s(x.reduce_smoke), bool_s(x.hide_potion_swirls), bool_s(x.skip_back_entities));
        }
        if self.auto_fish != AutoFishConfig::default() {
            let x = &self.auto_fish;
            let _ = writeln!(s, "volt.autofish.enabled:{}\nvolt.autofish.visual:{}\nvolt.autofish.chime:{}\nvolt.autofish.reel:{}\nvolt.autofish.recast:{}\nvolt.autofish.delay:{}", bool_s(x.enabled), bool_s(x.visual_bite_alert), bool_s(x.audio_chime), bool_s(x.auto_reel), bool_s(x.auto_recast), x.recast_delay_ticks);
        }
        if self.look_inspector != LookInspectorConfig::default() {
            let x = &self.look_inspector;
            let _ = writeln!(s, "volt.jade.enabled:{}\nvolt.jade.blocks:{}\nvolt.jade.crops:{}\nvolt.jade.mobs:{}\nvolt.jade.chests:{}\nvolt.jade.compact:{}\nvolt.jade.icon3d:{}\nvolt.jade.toolHint:{}\nvolt.jade.modSrc:{}\nvolt.jade.yOffset:{}", bool_s(x.enabled), bool_s(x.show_blocks), bool_s(x.show_crops_pct), bool_s(x.show_mob_health), bool_s(x.show_container_info), bool_s(x.compact), bool_s(x.show_3d_icon), bool_s(x.show_tool_hint), bool_s(x.show_mod_source), x.y_offset);
        }
        if self.sound_muffler != SoundMufflerConfig::default() {
            let x = &self.sound_muffler;
            let _ = writeln!(s, "volt.muffler.enabled:{}\nvolt.muffler.portals:{}\nvolt.muffler.ghasts:{}\nvolt.muffler.weather:{}\nvolt.muffler.pistons:{}\nvolt.muffler.carts:{}\nvolt.muffler.villagers:{}\nvolt.muffler.pct:{}", bool_s(x.enabled), bool_s(x.mute_portals), bool_s(x.mute_ghasts), bool_s(x.mute_weather), bool_s(x.mute_pistons), bool_s(x.mute_minecarts), bool_s(x.mute_villagers), x.muffle_pct);
        }
        if self.portal_calc != PortalCalcConfig::default() {
            let x = &self.portal_calc;
            let _ = writeln!(s, "volt.portal.enabled:{}\nvolt.portal.pill:{}\nvolt.portal.radius:{}\nvolt.portal.corner:{}", bool_s(x.enabled), bool_s(x.show_hud_pill), bool_s(x.show_link_radius), x.corner.name());
        }
        if self.compass_hud != CompassHudConfig::default() {
            let x = &self.compass_hud;
            let _ = writeln!(s, "volt.compass.enabled:{}\nvolt.compass.cardinals:{}\nvolt.compass.waypoints:{}\nvolt.compass.death:{}\nvolt.compass.players:{}\nvolt.compass.width:{}", bool_s(x.enabled), bool_s(x.show_cardinals), bool_s(x.show_waypoints), bool_s(x.show_death_marker), bool_s(x.show_players), x.width_pct);
        }
        if self.safe_walk != SafeWalkConfig::default() {
            let x = &self.safe_walk;
            let _ = writeln!(s, "volt.safewalk.enabled:{}\nvolt.safewalk.edge:{}\nvolt.safewalk.blocksOnly:{}\nvolt.safewalk.afk:{}\nvolt.safewalk.afkSecs:{}", bool_s(x.enabled), bool_s(x.edge_stop), bool_s(x.only_with_blocks), bool_s(x.anti_afk), x.afk_interval_secs);
        }
        if self.respawn_status != RespawnStatusConfig::default() {
            let x = &self.respawn_status;
            let _ = writeln!(s, "volt.spawnst.enabled:{}\nvolt.spawnst.coords:{}\nvolt.spawnst.warn:{}\nvolt.spawnst.anchor:{}\nvolt.spawnst.corner:{}", bool_s(x.enabled), bool_s(x.show_spawn_coords), bool_s(x.warn_obstructed), bool_s(x.show_anchor_charges), x.corner.name());
        }
        if self.scoreboard_tweaks != ScoreboardTweaksConfig::default() {
            let x = &self.scoreboard_tweaks;
            let _ = writeln!(s, "volt.sbtweak.enabled:{}\nvolt.sbtweak.noNums:{}\nvolt.sbtweak.hideSb:{}\nvolt.sbtweak.compactBoss:{}\nvolt.sbtweak.hideBoss:{}\nvolt.sbtweak.scale:{}", bool_s(x.enabled), bool_s(x.hide_numbers), bool_s(x.hide_scoreboard), bool_s(x.compact_bossbars), bool_s(x.hide_bossbars), x.scale_pct);
        }
        if self.sign_stand_helper != SignStandConfig::default() {
            let x = &self.sign_stand_helper;
            let _ = writeln!(s, "volt.signst.enabled:{}\nvolt.signst.copy:{}\nvolt.signst.paste:{}\nvolt.signst.stand:{}", bool_s(x.enabled), bool_s(x.auto_copy_last_sign), bool_s(x.quick_paste_button), bool_s(x.armor_stand_presets));
        }
        if self.dynamic_lights != DynamicLightsModConfig::default() {
            let x = &self.dynamic_lights;
            let _ = writeln!(s, "volt.dynlight.enabled:{}\nvolt.dynlight.held:{}\nvolt.dynlight.dropped:{}\nvolt.dynlight.burn:{}\nvolt.dynlight.minLvl:{}", bool_s(x.enabled), bool_s(x.held_items), bool_s(x.dropped_items), bool_s(x.burning_entities), x.min_level);
        }
        if self.low_shield != LowShieldConfig::default() {
            let x = &self.low_shield;
            let _ = writeln!(s, "volt.lowshield.enabled:{}\nvolt.lowshield.shieldDrop:{}\nvolt.lowshield.offDrop:{}\nvolt.lowshield.mainDrop:{}\nvolt.lowshield.noTilt:{}", bool_s(x.enabled), x.shield_drop_pct, x.offhand_drop_pct, x.mainhand_drop_pct, bool_s(x.hide_blocking_tilt));
        }
        if self.clean_view != CleanViewConfig::default() {
            let x = &self.clean_view;
            let _ = writeln!(s, "volt.cleanview.enabled:{}\nvolt.cleanview.selfPot:{}\nvolt.cleanview.beacons:{}\nvolt.cleanview.enemies:{}", bool_s(x.enabled), bool_s(x.hide_self_potions), bool_s(x.hide_ambient_beacons), bool_s(x.keep_enemy_particles));
        }
        if self.name_tags_plus != NameTagsPlusConfig::default() {
            let x = &self.name_tags_plus;
            let _ = writeln!(s, "volt.tagsplus.enabled:{}\nvolt.tagsplus.own:{}\nvolt.tagsplus.dist:{}\nvolt.tagsplus.hp:{}\nvolt.tagsplus.noFade:{}", bool_s(x.enabled), bool_s(x.show_own_nametag), bool_s(x.show_distance), bool_s(x.show_hp_tag), bool_s(x.no_sneak_fade));
        }
        if self.hit_color != HitColorConfig::default() {
            let x = &self.hit_color;
            let _ = writeln!(s, "volt.hitcol.enabled:{}\nvolt.hitcol.color:{}\nvolt.hitcol.pct:{}\nvolt.hitcol.armor:{}", bool_s(x.enabled), x.color.name(), x.intensity_pct, bool_s(x.flash_armor));
        }
        if self.crit_ring != CritRingConfig::default() {
            let x = &self.crit_ring;
            let _ = writeln!(s, "volt.critring.enabled:{}\nvolt.critring.badge:{}\nvolt.critring.bar:{}\nvolt.critring.color:{}", bool_s(x.enabled), bool_s(x.show_crit_badge), bool_s(x.show_cooldown_bar), x.crit_color.name());
        }
        if self.bow_charge != BowChargeConfig::default() {
            let x = &self.bow_charge;
            let _ = writeln!(s, "volt.bowchg.enabled:{}\nvolt.bowchg.bar:{}\nvolt.bowchg.chime:{}\nvolt.bowchg.autoRel:{}\nvolt.bowchg.corner:{}", bool_s(x.enabled), bool_s(x.show_pct_bar), bool_s(x.full_draw_chime), bool_s(x.auto_release_full), x.corner.name());
        }
        if self.cooldown_hud != CooldownHudConfig::default() {
            let x = &self.cooldown_hud;
            let _ = writeln!(s, "volt.cdhud.enabled:{}\nvolt.cdhud.pearls:{}\nvolt.cdhud.wind:{}\nvolt.cdhud.chorus:{}\nvolt.cdhud.corner:{}", bool_s(x.enabled), bool_s(x.track_pearls), bool_s(x.track_wind_charge), bool_s(x.track_chorus_gapple), x.corner.name());
        }
        if self.reach_box != ReachBoxConfig::default() {
            let x = &self.reach_box;
            let _ = writeln!(s, "volt.reachbox.enabled:{}\nvolt.reachbox.box:{}\nvolt.reachbox.ring:{}\nvolt.reachbox.crit:{}\nvolt.reachbox.color:{}", bool_s(x.enabled), bool_s(x.draw_target_box), bool_s(x.draw_reach_ring), bool_s(x.color_on_crit), x.color.name());
        }
        if self.potion_hud != PotionHudConfig::default() {
            let x = &self.potion_hud;
            let _ = writeln!(s, "volt.pothud.enabled:{}\nvolt.pothud.timers:{}\nvolt.pothud.blink:{}\nvolt.pothud.compact:{}\nvolt.pothud.corner:{}", bool_s(x.enabled), bool_s(x.show_timers), bool_s(x.blink_under_10s), bool_s(x.compact_pills), x.corner.name());
        }
        if self.durability_warn != DurabilityWarnConfig::default() {
            let x = &self.durability_warn;
            let _ = writeln!(s, "volt.durwarn.enabled:{}\nvolt.durwarn.pct:{}\nvolt.durwarn.armor:{}\nvolt.durwarn.held:{}\nvolt.durwarn.chime:{}", bool_s(x.enabled), x.warn_pct, bool_s(x.check_armor), bool_s(x.check_held), bool_s(x.play_chime));
        }
        if self.biome_title != BiomeTitleConfig::default() {
            let x = &self.biome_title;
            let _ = writeln!(s, "volt.biometitle.enabled:{}\nvolt.biometitle.biome:{}\nvolt.biometitle.dim:{}\nvolt.biometitle.secs:{}\nvolt.biometitle.coords:{}", bool_s(x.enabled), bool_s(x.show_biome_changes), bool_s(x.show_dimension_changes), x.display_secs, bool_s(x.show_sub_coords));
        }
        if self.xp_tracker != XpTrackerConfig::default() {
            let x = &self.xp_tracker;
            let _ = writeln!(s, "volt.xptrack.enabled:{}\nvolt.xptrack.next:{}\nvolt.xptrack.mend:{}\nvolt.xptrack.corner:{}", bool_s(x.enabled), bool_s(x.show_next_level_xp), bool_s(x.show_mending_summary), x.corner.name());
        }
        if self.inventory_hud != InventoryHudConfig::default() {
            let x = &self.inventory_hud;
            let _ = writeln!(s, "volt.invhud.enabled:{}\nvolt.invhud.grid:{}\nvolt.invhud.free:{}\nvolt.invhud.opacity:{}\nvolt.invhud.corner:{}", bool_s(x.enabled), bool_s(x.show_3x9_grid), bool_s(x.show_free_slots), x.bg_opacity_pct, x.corner.name());
        }
        if self.tab_ping != TabPingConfig::default() {
            let x = &self.tab_ping;
            let _ = writeln!(s, "volt.tabping.enabled:{}\nvolt.tabping.ms:{}\nvolt.tabping.cnt:{}\nvolt.tabping.dim:{}\nvolt.tabping.corner:{}", bool_s(x.enabled), bool_s(x.show_ping_ms), bool_s(x.show_player_count), bool_s(x.show_dim_tag), x.corner.name());
        }
        if self.moon_phase != MoonPhaseConfig::default() {
            let x = &self.moon_phase;
            let _ = writeln!(s, "volt.moon.enabled:{}\nvolt.moon.phase:{}\nvolt.moon.slime:{}\nvolt.moon.insomnia:{}\nvolt.moon.corner:{}", bool_s(x.enabled), bool_s(x.show_moon_phase), bool_s(x.show_slime_rating), bool_s(x.show_insomnia_warn), x.corner.name());
        }
        if self.session_stats != SessionStatsConfig::default() {
            let x = &self.session_stats;
            let _ = writeln!(s, "volt.sstats.enabled:{}\nvolt.sstats.kills:{}\nvolt.sstats.blocks:{}\nvolt.sstats.dist:{}\nvolt.sstats.corner:{}", bool_s(x.enabled), bool_s(x.show_kills), bool_s(x.show_blocks), bool_s(x.show_distance), x.corner.name());
        }
        if self.weather_sound != WeatherSoundConfig::default() {
            let x = &self.weather_sound;
            let _ = writeln!(s, "volt.wsound.enabled:{}\nvolt.wsound.flash:{}\nvolt.wsound.rain:{}\nvolt.wsound.thunder:{}", bool_s(x.enabled), bool_s(x.suppress_lightning_flash), bool_s(x.limit_rain_splashes), bool_s(x.mute_thunder_shake));
        }
        if self.custom_glint != CustomGlintConfig::default() {
            let x = &self.custom_glint;
            let _ = writeln!(s, "volt.glint.enabled:{}\nvolt.glint.preset:{}\nvolt.glint.speed:{}\nvolt.glint.strength:{}\nvolt.glint.compact:{}", bool_s(x.enabled), x.preset.name(), x.speed_pct, x.strength_pct, bool_s(x.compact_item_scale));
        }
        if self.clear_caves != ClearCavesConfig::default() {
            let x = &self.clear_caves;
            let _ = writeln!(s, "volt.caves.enabled:{}\nvolt.caves.void:{}\nvolt.caves.nv:{}\nvolt.caves.lava:{}", bool_s(x.enabled), bool_s(x.remove_void_fog), bool_s(x.smooth_night_vision), bool_s(x.boost_lava_clarity));
        }
        if self.player_highlight != PlayerHighlightConfig::default() {
            let x = &self.player_highlight;
            let _ = writeln!(s, "volt.phighlight.enabled:{}\nvolt.phighlight.players:{}\nvolt.phighlight.hostiles:{}\nvolt.phighlight.range:{}\nvolt.phighlight.color:{}", bool_s(x.enabled), bool_s(x.highlight_players), bool_s(x.highlight_hostiles), x.range, x.color.name());
        }
        if self.litematica_grid != LitematicaGridConfig::default() {
            let x = &self.litematica_grid;
            let _ = writeln!(s, "volt.ruler.enabled:{}\nvolt.ruler.hud:{}\nvolt.ruler.box:{}\nvolt.ruler.grid:{}\nvolt.ruler.color:{}", bool_s(x.enabled), bool_s(x.show_volume_hud), bool_s(x.show_3d_box), bool_s(x.show_grid_lines), x.color.name());
        }
        if self.death_history != DeathHistoryConfig::default() {
            let x = &self.death_history;
            let _ = writeln!(s, "volt.dhist.enabled:{}\nvolt.dhist.keep:{}\nvolt.dhist.home:{}\nvolt.dhist.chat:{}\nvolt.dhist.corner:{}", bool_s(x.enabled), x.keep_count, bool_s(x.show_home_pill), bool_s(x.auto_log_chat), x.corner.name());
        }
        if self.chunk_pillars != ChunkPillarsConfig::default() {
            let x = &self.chunk_pillars;
            let _ = writeln!(s, "volt.cpillar.enabled:{}\nvolt.cpillar.pillars:{}\nvolt.cpillar.biomes:{}\nvolt.cpillar.height:{}", bool_s(x.enabled), bool_s(x.draw_corner_pillars), bool_s(x.draw_biome_borders), x.pillar_height);
        }
        if self.offhand_torch != OffhandTorchConfig::default() {
            let x = &self.offhand_torch;
            let _ = writeln!(s, "volt.otorch.enabled:{}\nvolt.otorch.auto:{}\nvolt.otorch.light:{}\nvolt.otorch.warn:{}", bool_s(x.enabled), bool_s(x.auto_place_dark), x.light_threshold, bool_s(x.notify_low_torches));
        }
        if self.chat_mention != ChatMentionConfig::default() {
            let x = &self.chat_mention;
            let _ = writeln!(s, "volt.cmention.enabled:{}\nvolt.cmention.user:{}\nvolt.cmention.tpa:{}\nvolt.cmention.chime:{}\nvolt.cmention.popup:{}", bool_s(x.enabled), bool_s(x.alert_on_username), bool_s(x.alert_on_tpa_msg), bool_s(x.play_ping_chime), bool_s(x.show_popup_banner));
        }
        if self.drop_guard != DropGuardConfig::default() {
            let x = &self.drop_guard;
            let _ = writeln!(s, "volt.dropguard.enabled:{}\nvolt.dropguard.tools:{}\nvolt.dropguard.totems:{}\nvolt.dropguard.slots:{}\nvolt.dropguard.dbl:{}", bool_s(x.enabled), bool_s(x.protect_tools_weapons), bool_s(x.protect_totems), bool_s(x.lock_slots_1_to_3), bool_s(x.double_tap_bypass));
        }
        if self.hotbar_restock != HotbarRestockConfig::default() {
            let x = &self.hotbar_restock;
            let _ = writeln!(s, "volt.restock.enabled:{}\nvolt.restock.blocks:{}\nvolt.restock.util:{}\nvolt.restock.food:{}\nvolt.restock.toast:{}", bool_s(x.enabled), bool_s(x.restock_blocks), bool_s(x.restock_pearls_fireworks), bool_s(x.restock_food), bool_s(x.show_toast));
        }
        if self.photo_mode != PhotoModeConfig::default() {
            let x = &self.photo_mode;
            let _ = writeln!(s, "volt.photo.enabled:{}\nvolt.photo.hideHud:{}\nvolt.photo.lock:{}\nvolt.photo.guides:{}", bool_s(x.enabled), bool_s(x.hide_all_hud), bool_s(x.lock_freecam_pose), bool_s(x.show_frame_guides));
        }
        if self.jei != JeiConfig::default() {
            let x = &self.jei;
            let _ = writeln!(s, "volt.jei.enabled:{}\nvolt.jei.inInv:{}\nvolt.jei.ids:{}\nvolt.jei.cook:{}\nvolt.jei.compact:{}", bool_s(x.enabled), bool_s(x.show_in_inventory), bool_s(x.show_item_ids), bool_s(x.show_cooking_recipes), bool_s(x.compact_grid));
        }
        if self.cinematic_timelapse != CinematicTimelapseConfig::default() {
            let x = &self.cinematic_timelapse;
            let _ = writeln!(s, "volt.timelapse.enabled:{}\nvolt.timelapse.delay:{}\nvolt.timelapse.path:{}\nvolt.timelapse.speed:{}\nvolt.timelapse.hideHud:{}\nvolt.timelapse.countdown:{}", bool_s(x.enabled), x.start_delay_secs, x.path_mode.name(), x.orbit_speed_deg, bool_s(x.hide_hud), bool_s(x.show_countdown));
        }
        if self.blocks_outline != BlocksOutlineConfig::default() {
            let x = &self.blocks_outline;
            let _ = writeln!(
                s,
                "volt.blocksOutline.enabled:{}\nvolt.blocksOutline.color:{}\nvolt.blocksOutline.alpha:{}\nvolt.blocksOutline.width:{}\nvolt.blocksOutline.rainbow:{}\nvolt.blocksOutline.rainbowSpeed:{}\nvolt.blocksOutline.pulse:{}\nvolt.blocksOutline.fill:{}\nvolt.blocksOutline.fillAlpha:{}\nvolt.blocksOutline.fillMatch:{}\nvolt.blocksOutline.fillColor:{}\nvolt.blocksOutline.seeThrough:{}\nvolt.blocksOutline.highContrast:{}\nvolt.blocksOutline.inflate:{}",
                bool_s(x.enabled),
                x.color_rgb & 0x00FF_FFFF,
                x.alpha_pct,
                x.line_width,
                bool_s(x.rainbow),
                x.rainbow_speed,
                bool_s(x.pulse),
                bool_s(x.fill_overlay),
                x.fill_alpha_pct,
                bool_s(x.fill_match_outline),
                x.fill_color_rgb & 0x00FF_FFFF,
                bool_s(x.see_through),
                bool_s(x.high_contrast_backing),
                x.inflate_pct,
            );
        }
        for (i, meta) in VOLT_MODS.iter().enumerate() {
            let b = &self.binds.0[i];
            if !b.toggle_key.is_empty() {
                let _ = writeln!(s, "volt.bind.{}.toggle:{}", meta.key, b.toggle_key);
            }
            if !b.use_key.is_empty() {
                let _ = writeln!(s, "volt.bind.{}.use:{}", meta.key, b.use_key);
            }
            if b.mode != default_keybind_mode_for_mod(meta.id) {
                let _ = writeln!(s, "volt.bind.{}.mode:{}", meta.key, b.mode.name());
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltQuickAction {
    ToggleZoom,
    ToggleFreecam,
    ToggleFreelook,
    ToggleAutoSprint,
    ToggleFullbright,
    SwapChestElytra,
    QuickOffhandTotem,
    AddWaypointHere,
    ClearDeathWaypoints,
    RunChatMacro(u8),
    CopyCoords,
    ToggleChunkBorders,
    ToggleHitboxes,
    OpenModKeyPicker { mod_id: u8, is_use: bool },
    OpenWaypointManager,
    DeleteWaypoint(usize),
    ClearAllWaypoints,
    CycleWaypointColor(usize),
    ShareWaypoint(usize),
    SetRulerPointA,
    SetRulerPointB,
    ClearRuler,
    ToggleJei,
    StartStopTimelapse,
    SetHomeWaypoint,
    ResetSessionStats,
    TogglePhotoMode,
    OpenBlockOutlineColor,
}

