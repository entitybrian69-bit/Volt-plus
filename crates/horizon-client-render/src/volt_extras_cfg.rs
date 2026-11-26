//! Configuration structs, metadata, Modrinth project slugs, and persistence
//! for all 52 built-in Volt+ Extras mods (every mod defaults to `enabled: false`).
use crate::extras::Corner;
use std::fmt::Write as _;

pub const VOLT_MOD_COUNT: usize = 52;

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
}
impl Default for AppleSkinConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_saturation_overlay: true,
            show_exhaustion_underlay: true,
            show_held_food_preview: true,
            show_health_regen_overlay: true,
            show_food_tooltip: true,
            always_show_tooltip: false,
            show_debug_stats: true,
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
            let _ = writeln!(s, "volt.jade.enabled:{}\nvolt.jade.blocks:{}\nvolt.jade.crops:{}\nvolt.jade.mobs:{}\nvolt.jade.chests:{}\nvolt.jade.compact:{}", bool_s(x.enabled), bool_s(x.show_blocks), bool_s(x.show_crops_pct), bool_s(x.show_mob_health), bool_s(x.show_container_info), bool_s(x.compact));
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
}

