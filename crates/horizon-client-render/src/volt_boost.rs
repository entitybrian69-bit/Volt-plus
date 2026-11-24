//! Volt + Boost: deep in-game performance tuning, hardware diagnostics,
//! dynamic resolution, thermal guard, per-entity frame gates, and replay
//! settings. Separate from the existing Volt Boost (`Page::Horizon`) screen.
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FILE_NAME: &str = "volt-plus-boost.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoostProfile {
    #[default]
    Off,
    Balanced,
    CompetitivePvp,
    UltraFps,
    BatterySaver,
    LowRamRescue,
}

impl BoostProfile {
    pub const ALL: [Self; 6] = [
        Self::Off,
        Self::Balanced,
        Self::CompetitivePvp,
        Self::UltraFps,
        Self::BatterySaver,
        Self::LowRamRescue,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Balanced => "Balanced",
            Self::CompetitivePvp => "Competitive PvP",
            Self::UltraFps => "Ultra FPS",
            Self::BatterySaver => "Battery Saver",
            Self::LowRamRescue => "Low-RAM Rescue",
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerfHudCorner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl PerfHudCorner {
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "Top-Left",
            Self::TopRight => "Top-Right",
            Self::BottomLeft => "Bottom-Left",
            Self::BottomRight => "Bottom-Right",
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TickRangeMode {
    #[default]
    Normal,
    Reduced,
    Minimal,
}

impl TickRangeMode {
    pub const ALL: [Self; 3] = [Self::Normal, Self::Reduced, Self::Minimal];

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Reduced => "Reduced",
            Self::Minimal => "Minimal",
        }
    }

    pub fn scale(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Reduced => 0.65,
            Self::Minimal => 0.35,
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameGate {
    #[default]
    Off,
    Every2nd,
    Every3rd,
    Every4th,
}

impl FrameGate {
    pub const ALL: [Self; 4] = [Self::Off, Self::Every2nd, Self::Every3rd, Self::Every4th];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Every2nd => "Every 2nd",
            Self::Every3rd => "Every 3rd",
            Self::Every4th => "Every 4th",
        }
    }

    pub fn stride(self) -> u64 {
        match self {
            Self::Off => 1,
            Self::Every2nd => 2,
            Self::Every3rd => 3,
            Self::Every4th => 4,
        }
    }

    pub fn is_active(self) -> bool {
        self != Self::Off
    }

    pub fn should_run(self, frame_index: u64, dist_blocks: f32, gate_distance: u32) -> bool {
        let stride = self.stride();
        if stride <= 1 || dist_blocks <= gate_distance as f32 {
            return true;
        }
        frame_index.is_multiple_of(stride)
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkFadeMode {
    Off,
    Fast,
    Smooth,
    #[default]
    Full,
}

impl ChunkFadeMode {
    pub const ALL: [Self; 4] = [Self::Off, Self::Fast, Self::Smooth, Self::Full];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off (0.0s)",
            Self::Fast => "Fast (0.2s)",
            Self::Smooth => "Smooth (0.6s)",
            Self::Full => "Full (1.0s)",
        }
    }

    pub fn seconds(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Fast => 0.2,
            Self::Smooth => 0.6,
            Self::Full => 1.0,
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleBudget {
    #[default]
    Unlimited,
    High,
    Medium,
    Low,
    Minimal,
}

impl ParticleBudget {
    pub const ALL: [Self; 5] = [
        Self::Unlimited,
        Self::High,
        Self::Medium,
        Self::Low,
        Self::Minimal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Unlimited => "Unlimited",
            Self::High => "High (1024)",
            Self::Medium => "Medium (512)",
            Self::Low => "Low (256)",
            Self::Minimal => "Minimal (64)",
        }
    }

    pub fn max_particles(self) -> usize {
        match self {
            Self::Unlimited => usize::MAX,
            Self::High => 1024,
            Self::Medium => 512,
            Self::Low => 256,
            Self::Minimal => 64,
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlintTrimMode {
    #[default]
    All,
    Nearby,
    Off,
}

impl GlintTrimMode {
    pub const ALL: [Self; 3] = [Self::All, Self::Nearby, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Nearby => "Nearby",
            Self::Off => "Off",
        }
    }

    pub fn allows_at_distance(self, dist_blocks: f32) -> bool {
        match self {
            Self::All => true,
            Self::Nearby => dist_blocks <= 16.0,
            Self::Off => false,
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuBackdropMode {
    #[default]
    Full,
    Blurred,
    Static,
    Off,
}

impl MenuBackdropMode {
    pub const ALL: [Self; 4] = [Self::Full, Self::Blurred, Self::Static, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Blurred => "Blurred",
            Self::Static => "Static",
            Self::Off => "Off",
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InactivityFpsMode {
    #[default]
    Off,
    Fps30,
    Fps15,
    Fps10,
}

impl InactivityFpsMode {
    pub const ALL: [Self; 4] = [Self::Off, Self::Fps30, Self::Fps15, Self::Fps10];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fps30 => "30 FPS",
            Self::Fps15 => "15 FPS",
            Self::Fps10 => "10 FPS",
        }
    }

    pub fn fps_cap(self) -> Option<u32> {
        match self {
            Self::Off => None,
            Self::Fps30 => Some(30),
            Self::Fps15 => Some(15),
            Self::Fps10 => Some(10),
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayLength {
    Sec30,
    Min1,
    #[default]
    Min5,
    Min10,
    Session,
}

impl ReplayLength {
    pub const ALL: [Self; 5] = [
        Self::Sec30,
        Self::Min1,
        Self::Min5,
        Self::Min10,
        Self::Session,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Sec30 => "30 s",
            Self::Min1 => "1 min",
            Self::Min5 => "5 min",
            Self::Min10 => "10 min",
            Self::Session => "Session",
        }
    }

    pub fn max_duration_ms(self) -> Option<u64> {
        match self {
            Self::Sec30 => Some(30_000),
            Self::Min1 => Some(60_000),
            Self::Min5 => Some(300_000),
            Self::Min10 => Some(600_000),
            Self::Session => None,
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayQuality {
    Low,
    #[default]
    Medium,
    High,
    Max,
}

impl ReplayQuality {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Max];

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Max => "Max",
        }
    }

    /// Approximate storage consumed per minute of gameplay.
    pub fn estimated_mb_per_min(self) -> f32 {
        match self {
            Self::Low => 0.8,
            Self::Medium => 1.8,
            Self::High => 3.4,
            Self::Max => 6.5,
        }
    }

    /// Minimum interval between captured replay keyframes/snapshots.
    pub fn sample_interval_ms(self) -> u64 {
        match self {
            Self::Low => 100,    // 10 Hz
            Self::Medium => 50,  // 20 Hz
            Self::High => 33,    // 30 Hz
            Self::Max => 16,     // 60 Hz
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
        let len = Self::ALL.len() as i32;
        Self::ALL[((idx as i32 + dir).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayHudButtonLayout {
    /// Normalized anchor coordinates in `[0.04, 0.96]` of viewport width/height.
    pub norm_x: f32,
    pub norm_y: f32,
    /// Size scale multiplier in `[0.65, 1.85]`.
    pub size: f32,
    /// Opacity in `[0.25, 1.0]`.
    pub opacity: f32,
}

impl Default for ReplayHudButtonLayout {
    fn default() -> Self {
        Self {
            norm_x: 0.86,
            norm_y: 0.16,
            size: 1.0,
            opacity: 0.88,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoltPlusBoostSettings {
    // 1. Profiles & Diagnostics
    pub profile: BoostProfile,
    pub mod_lag_shield: bool,
    pub auto_tune_summary: String,
    pub last_ram_freed_mb: f32,
    pub live_perf_hud: bool,
    pub perf_hud_corner: PerfHudCorner,
    pub volt_reflex: bool,

    // 2. Culling & Visibility
    pub smart_leaf_cull: bool,
    pub frustum_first_mesh: bool,
    pub cave_sky_cull: bool,
    pub sign_text_cull: bool,
    pub offscreen_entity_skip: bool,
    pub distant_lod_boost: bool,
    pub distant_lod_distance: u32,

    // 3. Geometry & Meshing
    pub coplanar_quad_merge: bool,
    pub zero_stutter_mesh_pool: bool,
    pub sealed_cave_occlusion: bool,
    pub smart_frame_pacer: bool,
    pub frame_items_loader: bool,

    // 4. Chunk Streaming
    pub chunk_upload_cap: u32,
    pub chunk_build_distance: u32,
    pub chunk_fade_mode: ChunkFadeMode,
    pub fast_weather: bool,

    // 5. Simulation & Ticks
    pub block_tick_range: TickRangeMode,
    pub entity_tick_range: TickRangeMode,
    pub mob_pose_throttle: bool,
    pub max_entities: u32,
    pub dropped_item_cap: u32,
    pub block_entity_cap: u32,
    pub merge_drop_stacks: bool,
    pub mob_frame_gate: FrameGate,
    pub mob_gate_distance: u32,
    pub player_frame_gate: FrameGate,
    pub player_gate_distance: u32,
    pub item_frame_gate: FrameGate,
    pub item_gate_distance: u32,
    pub block_entity_frame_gate: FrameGate,
    pub block_entity_gate_distance: u32,
    pub particle_frame_gate: FrameGate,
    pub particle_gate_distance: u32,

    // 6. Effects, Particles & Visual Overhead
    pub particle_budget: ParticleBudget,
    pub burst_lag_shield: bool,
    pub entity_shadows: bool,
    pub entity_shadow_distance: u32,
    pub smart_10hz_atlas: bool,
    pub glint_trim_mode: GlintTrimMode,
    pub menu_backdrop: MenuBackdropMode,
    pub dynamic_menu_fps: bool,

    // 7. Display & Framerate
    pub auto_drs_scale: bool,
    pub drs_target_fps: u32,
    pub drs_min_scale_pct: u32,
    pub drs_max_scale_pct: u32,
    pub fps_cap: u32,
    pub inactivity_fps: InactivityFpsMode,
    pub inactivity_delay_secs: u32,
    pub thermal_guard: bool,

    // 8. Threads
    pub big_little_boost: bool,
    pub background_worker_coalescing: bool,
    pub storm_wind_boost: bool,

    // 9. Replay
    pub replay_recording: bool,
    pub replay_hud_button: bool,
    pub replay_length: ReplayLength,
    pub replay_quality: ReplayQuality,
    pub replay_capture_hud: bool,
    pub replay_storage_budget_mb: u32,
    pub replay_storage_budget_asked: bool,
    pub replay_hud_layout: ReplayHudButtonLayout,

    // Runtime-updated diagnostics (persisted or reset cleanly)
    #[serde(skip)]
    pub live_drs_scale_pct: u32,
    #[serde(skip)]
    pub live_reflex_saved_ms: f32,
    #[serde(skip)]
    pub live_thermal_step: ThermalStep,
    #[serde(skip)]
    pub live_storage_used_bytes: u64,
    #[serde(skip)]
    pub live_storage_clip_count: usize,
}

impl Default for VoltPlusBoostSettings {
    fn default() -> Self {
        Self {
            profile: BoostProfile::Off,
            mod_lag_shield: false,
            auto_tune_summary: String::new(),
            last_ram_freed_mb: 0.0,
            live_perf_hud: false,
            perf_hud_corner: PerfHudCorner::TopLeft,
            volt_reflex: false,

            smart_leaf_cull: false,
            frustum_first_mesh: false,
            cave_sky_cull: false,
            sign_text_cull: false,
            offscreen_entity_skip: false,
            distant_lod_boost: false,
            distant_lod_distance: 96,

            coplanar_quad_merge: false,
            zero_stutter_mesh_pool: false,
            sealed_cave_occlusion: false,
            smart_frame_pacer: false,
            frame_items_loader: false,

            chunk_upload_cap: 0,
            chunk_build_distance: 12,
            chunk_fade_mode: ChunkFadeMode::Full,
            fast_weather: false,

            block_tick_range: TickRangeMode::Normal,
            entity_tick_range: TickRangeMode::Normal,
            mob_pose_throttle: false,
            max_entities: 0,
            dropped_item_cap: 0,
            block_entity_cap: 0,
            merge_drop_stacks: false,
            mob_frame_gate: FrameGate::Off,
            mob_gate_distance: 32,
            player_frame_gate: FrameGate::Off,
            player_gate_distance: 48,
            item_frame_gate: FrameGate::Off,
            item_gate_distance: 24,
            block_entity_frame_gate: FrameGate::Off,
            block_entity_gate_distance: 32,
            particle_frame_gate: FrameGate::Off,
            particle_gate_distance: 24,

            particle_budget: ParticleBudget::Unlimited,
            burst_lag_shield: false,
            entity_shadows: true,
            entity_shadow_distance: 32,
            smart_10hz_atlas: false,
            glint_trim_mode: GlintTrimMode::All,
            menu_backdrop: MenuBackdropMode::Full,
            dynamic_menu_fps: false,

            auto_drs_scale: false,
            drs_target_fps: 60,
            drs_min_scale_pct: 50,
            drs_max_scale_pct: 100,
            fps_cap: 120,
            inactivity_fps: InactivityFpsMode::Off,
            inactivity_delay_secs: 30,
            thermal_guard: false,

            big_little_boost: false,
            background_worker_coalescing: false,
            storm_wind_boost: false,

            replay_recording: false,
            replay_hud_button: false,
            replay_length: ReplayLength::Min5,
            replay_quality: ReplayQuality::Medium,
            replay_capture_hud: false,
            replay_storage_budget_mb: 1024,
            replay_storage_budget_asked: false,
            replay_hud_layout: ReplayHudButtonLayout::default(),

            live_drs_scale_pct: 100,
            live_reflex_saved_ms: 0.0,
            live_thermal_step: ThermalStep::Normal,
            live_storage_used_bytes: 0,
            live_storage_clip_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThermalStep {
    #[default]
    Normal,
    Step1Scale,
    Step2Fps,
    Step3Effects,
}

impl ThermalStep {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal (0 steps)",
            Self::Step1Scale => "Step 1: Scale -10%",
            Self::Step2Fps => "Step 2: FPS Cap 45",
            Self::Step3Effects => "Step 3: Effects Low",
        }
    }

    pub fn scale_multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Step1Scale => 0.90,
            Self::Step2Fps => 0.85,
            Self::Step3Effects => 0.75,
        }
    }

    pub fn fps_cap_override(self) -> Option<u32> {
        match self {
            Self::Normal | Self::Step1Scale => None,
            Self::Step2Fps => Some(45),
            Self::Step3Effects => Some(30),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareProbe {
    pub cores: usize,
    pub arch: &'static str,
    pub ram_mb: u64,
}

impl HardwareProbe {
    pub fn detect() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let arch = std::env::consts::ARCH;
        let ram_mb = read_total_ram_mb().unwrap_or(4096);
        Self { cores, arch, ram_mb }
    }

    pub fn recommended_profile(&self) -> (BoostProfile, String) {
        let profile = if self.ram_mb < 3584 {
            BoostProfile::LowRamRescue
        } else if self.cores >= 8 && self.ram_mb >= 6144 {
            BoostProfile::UltraFps
        } else {
            BoostProfile::Balanced
        };
        let gb = self.ram_mb as f32 / 1024.0;
        let note = format!(
            "{} cores · {} · {:.1} GB -> {}",
            self.cores,
            self.arch,
            gb,
            profile.label()
        );
        (profile, note)
    }
}

fn read_total_ram_mb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

/// Reads current SoC temperature in Celsius from `/sys/class/thermal` when available.
pub fn read_max_thermal_celsius() -> Option<f32> {
    let entries = std::fs::read_dir("/sys/class/thermal").ok()?;
    let mut max_c: Option<f32> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.starts_with("thermal_zone") {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(path.join("temp"))
            && let Ok(val) = raw.trim().parse::<f32>()
        {
            let celsius = if val > 200.0 { val / 1000.0 } else { val };
            if (10.0..=120.0).contains(&celsius) {
                max_c = Some(max_c.map_or(celsius, |m| m.max(celsius)));
            }
        }
    }
    max_c
}

#[derive(Clone, Debug)]
pub struct DrsController {
    ema_frame_ms: f32,
    current_scale_pct: u32,
    cooldown_frames: u32,
}

impl Default for DrsController {
    fn default() -> Self {
        Self {
            ema_frame_ms: 16.6,
            current_scale_pct: 100,
            cooldown_frames: 0,
        }
    }
}

impl DrsController {
    pub fn current_scale_pct(&self) -> u32 {
        self.current_scale_pct
    }

    pub fn update(&mut self, cfg: &VoltPlusBoostSettings, frame_ms: f32) -> u32 {
        let min_s = cfg.drs_min_scale_pct.clamp(35, 95);
        let max_s = cfg.drs_max_scale_pct.clamp(min_s, 100);
        if !cfg.auto_drs_scale {
            self.current_scale_pct = 100;
            return 100;
        }
        self.current_scale_pct = self.current_scale_pct.clamp(min_s, max_s);
        let clamped_ms = frame_ms.clamp(1.0, 250.0);
        self.ema_frame_ms = self.ema_frame_ms * 0.85 + clamped_ms * 0.15;
        if self.cooldown_frames > 0 {
            self.cooldown_frames -= 1;
            return self.current_scale_pct;
        }
        let target_ms = 1000.0 / (cfg.drs_target_fps.max(20) as f32);
        if self.ema_frame_ms > target_ms * 1.12 && self.current_scale_pct > min_s {
            self.current_scale_pct = self.current_scale_pct.saturating_sub(5).max(min_s);
            self.cooldown_frames = 12;
        } else if self.ema_frame_ms < target_ms * 0.82 && self.current_scale_pct < max_s {
            self.current_scale_pct = (self.current_scale_pct + 5).min(max_s);
            self.cooldown_frames = 20;
        }
        self.current_scale_pct
    }
}

pub fn evaluate_thermal_step(
    enabled: bool,
    temp_celsius: Option<f32>,
    sustained_frame_ms: f32,
) -> ThermalStep {
    if !enabled {
        return ThermalStep::Normal;
    }
    let temp = temp_celsius.unwrap_or(0.0);
    if temp >= 78.0 || sustained_frame_ms >= 38.0 {
        ThermalStep::Step3Effects
    } else if temp >= 70.0 || sustained_frame_ms >= 29.0 {
        ThermalStep::Step2Fps
    } else if temp >= 62.0 || sustained_frame_ms >= 22.5 {
        ThermalStep::Step1Scale
    } else {
        ThermalStep::Normal
    }
}

impl VoltPlusBoostSettings {
    pub fn load(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let mut s: Self = serde_json::from_str(&raw).unwrap_or_default();
        s.sanitize();
        s
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut copy = self.clone();
        copy.sanitize();
        let json = serde_json::to_string_pretty(&copy).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn sanitize(&mut self) {
        self.distant_lod_distance = self.distant_lod_distance.clamp(32, 256);
        self.chunk_upload_cap = self.chunk_upload_cap.min(64);
        self.chunk_build_distance = self.chunk_build_distance.clamp(2, 32);
        self.max_entities = self.max_entities.min(256);
        self.dropped_item_cap = self.dropped_item_cap.min(256);
        self.block_entity_cap = self.block_entity_cap.min(512);
        self.mob_gate_distance = self.mob_gate_distance.clamp(8, 96);
        self.player_gate_distance = self.player_gate_distance.clamp(8, 96);
        self.item_gate_distance = self.item_gate_distance.clamp(8, 96);
        self.block_entity_gate_distance = self.block_entity_gate_distance.clamp(8, 96);
        self.particle_gate_distance = self.particle_gate_distance.clamp(8, 96);
        self.entity_shadow_distance = self.entity_shadow_distance.clamp(8, 64);
        self.drs_target_fps = match self.drs_target_fps {
            0..=37 => 30,
            38..=52 => 45,
            53..=75 => 60,
            76..=105 => 90,
            _ => 120,
        };
        self.drs_min_scale_pct = self.drs_min_scale_pct.clamp(35, 90);
        self.drs_max_scale_pct = self
            .drs_max_scale_pct
            .clamp(self.drs_min_scale_pct.max(60), 100);
        self.fps_cap = self.fps_cap.clamp(30, 260);
        self.inactivity_delay_secs = self.inactivity_delay_secs.clamp(5, 120);
        self.replay_storage_budget_mb = self.replay_storage_budget_mb.clamp(128, 8192);
        self.replay_hud_layout.norm_x = self.replay_hud_layout.norm_x.clamp(0.04, 0.96);
        self.replay_hud_layout.norm_y = self.replay_hud_layout.norm_y.clamp(0.04, 0.96);
        self.replay_hud_layout.size = self.replay_hud_layout.size.clamp(0.65, 1.85);
        self.replay_hud_layout.opacity = self.replay_hud_layout.opacity.clamp(0.25, 1.0);
    }

    pub fn apply_profile(&mut self, profile: BoostProfile) {
        self.profile = profile;
        self.mod_lag_shield = profile != BoostProfile::Off;
        match profile {
            BoostProfile::Off => {
                let replay_rec = self.replay_recording;
                let replay_btn = self.replay_hud_button;
                let replay_len = self.replay_length;
                let replay_q = self.replay_quality;
                let replay_hud = self.replay_capture_hud;
                let replay_budget = self.replay_storage_budget_mb;
                let replay_asked = self.replay_storage_budget_asked;
                let replay_layout = self.replay_hud_layout.clone();
                *self = Self::default();
                self.replay_recording = replay_rec;
                self.replay_hud_button = replay_btn;
                self.replay_length = replay_len;
                self.replay_quality = replay_q;
                self.replay_capture_hud = replay_hud;
                self.replay_storage_budget_mb = replay_budget;
                self.replay_storage_budget_asked = replay_asked;
                self.replay_hud_layout = replay_layout;
            }
            BoostProfile::Balanced => {
                self.volt_reflex = true;
                self.smart_leaf_cull = true;
                self.frustum_first_mesh = true;
                self.cave_sky_cull = true;
                self.sign_text_cull = true;
                self.offscreen_entity_skip = true;
                self.distant_lod_boost = false;
                self.coplanar_quad_merge = true;
                self.zero_stutter_mesh_pool = true;
                self.sealed_cave_occlusion = true;
                self.smart_frame_pacer = true;
                self.frame_items_loader = true;
                self.chunk_upload_cap = 16;
                self.chunk_build_distance = 12;
                self.chunk_fade_mode = ChunkFadeMode::Fast;
                self.fast_weather = false;
                self.block_tick_range = TickRangeMode::Normal;
                self.entity_tick_range = TickRangeMode::Normal;
                self.mob_pose_throttle = true;
                self.max_entities = 0;
                self.dropped_item_cap = 128;
                self.block_entity_cap = 256;
                self.merge_drop_stacks = true;
                self.mob_frame_gate = FrameGate::Every2nd;
                self.player_frame_gate = FrameGate::Off;
                self.item_frame_gate = FrameGate::Every2nd;
                self.block_entity_frame_gate = FrameGate::Every2nd;
                self.particle_frame_gate = FrameGate::Every2nd;
                self.particle_budget = ParticleBudget::High;
                self.burst_lag_shield = true;
                self.entity_shadows = true;
                self.entity_shadow_distance = 24;
                self.smart_10hz_atlas = true;
                self.glint_trim_mode = GlintTrimMode::All;
                self.menu_backdrop = MenuBackdropMode::Blurred;
                self.dynamic_menu_fps = true;
                self.auto_drs_scale = true;
                self.drs_target_fps = 60;
                self.drs_min_scale_pct = 65;
                self.drs_max_scale_pct = 100;
                self.fps_cap = 120;
                self.inactivity_fps = InactivityFpsMode::Fps30;
                self.thermal_guard = true;
                self.big_little_boost = true;
                self.background_worker_coalescing = true;
                self.storm_wind_boost = true;
            }
            BoostProfile::CompetitivePvp => {
                self.apply_profile(BoostProfile::Balanced);
                self.profile = BoostProfile::CompetitivePvp;
                self.volt_reflex = true;
                self.chunk_upload_cap = 24;
                self.chunk_fade_mode = ChunkFadeMode::Off;
                self.fast_weather = true;
                self.player_frame_gate = FrameGate::Off;
                self.mob_frame_gate = FrameGate::Every2nd;
                self.particle_budget = ParticleBudget::Medium;
                self.glint_trim_mode = GlintTrimMode::Nearby;
                self.menu_backdrop = MenuBackdropMode::Static;
                self.auto_drs_scale = true;
                self.drs_target_fps = 90;
                self.drs_min_scale_pct = 60;
                self.fps_cap = 165;
            }
            BoostProfile::UltraFps => {
                self.apply_profile(BoostProfile::CompetitivePvp);
                self.profile = BoostProfile::UltraFps;
                self.distant_lod_boost = true;
                self.distant_lod_distance = 128;
                self.chunk_upload_cap = 12;
                self.chunk_build_distance = 8;
                self.block_tick_range = TickRangeMode::Reduced;
                self.entity_tick_range = TickRangeMode::Reduced;
                self.max_entities = 96;
                self.dropped_item_cap = 64;
                self.block_entity_cap = 128;
                self.mob_frame_gate = FrameGate::Every3rd;
                self.item_frame_gate = FrameGate::Every3rd;
                self.block_entity_frame_gate = FrameGate::Every3rd;
                self.particle_frame_gate = FrameGate::Every3rd;
                self.particle_budget = ParticleBudget::Low;
                self.entity_shadows = false;
                self.glint_trim_mode = GlintTrimMode::Off;
                self.menu_backdrop = MenuBackdropMode::Off;
                self.drs_target_fps = 120;
                self.drs_min_scale_pct = 50;
                self.fps_cap = 260;
            }
            BoostProfile::BatterySaver => {
                self.apply_profile(BoostProfile::Balanced);
                self.profile = BoostProfile::BatterySaver;
                self.volt_reflex = false;
                self.chunk_upload_cap = 8;
                self.chunk_build_distance = 8;
                self.block_tick_range = TickRangeMode::Reduced;
                self.entity_tick_range = TickRangeMode::Reduced;
                self.particle_budget = ParticleBudget::Low;
                self.menu_backdrop = MenuBackdropMode::Static;
                self.dynamic_menu_fps = true;
                self.auto_drs_scale = true;
                self.drs_target_fps = 45;
                self.drs_min_scale_pct = 60;
                self.fps_cap = 45;
                self.inactivity_fps = InactivityFpsMode::Fps10;
                self.inactivity_delay_secs = 15;
                self.thermal_guard = true;
            }
            BoostProfile::LowRamRescue => {
                self.apply_profile(BoostProfile::UltraFps);
                self.profile = BoostProfile::LowRamRescue;
                self.distant_lod_boost = false;
                self.chunk_upload_cap = 6;
                self.chunk_build_distance = 6;
                self.block_tick_range = TickRangeMode::Minimal;
                self.entity_tick_range = TickRangeMode::Minimal;
                self.max_entities = 48;
                self.dropped_item_cap = 32;
                self.block_entity_cap = 64;
                self.mob_frame_gate = FrameGate::Every4th;
                self.item_frame_gate = FrameGate::Every4th;
                self.block_entity_frame_gate = FrameGate::Every4th;
                self.particle_frame_gate = FrameGate::Every4th;
                self.particle_budget = ParticleBudget::Minimal;
                self.fps_cap = 60;
                self.inactivity_fps = InactivityFpsMode::Fps15;
            }
        }
        self.sanitize();
    }

    pub fn run_auto_tune(&mut self) -> String {
        let probe = HardwareProbe::detect();
        let (profile, summary) = probe.recommended_profile();
        self.apply_profile(profile);
        self.auto_tune_summary = summary.clone();
        summary
    }

    pub fn active_optimization_count(&self) -> usize {
        (0..9).map(|c| self.category_counts(BoostCategory::ALL[c]).0).sum()
    }

    /// Returns `(active_on_count, total_count)` for a category card (`"X of Y on"`).
    pub fn category_counts(&self, cat: BoostCategory) -> (usize, usize) {
        match cat {
            BoostCategory::ProfilesAndDiagnostics => {
                let on = usize::from(self.profile != BoostProfile::Off)
                    + usize::from(!self.auto_tune_summary.is_empty())
                    + usize::from(self.last_ram_freed_mb > 0.0)
                    + usize::from(self.live_perf_hud)
                    + usize::from(self.live_perf_hud && self.perf_hud_corner != PerfHudCorner::TopLeft)
                    + usize::from(self.volt_reflex);
                (on, 6)
            }
            BoostCategory::CullingAndVisibility => {
                let on = usize::from(self.smart_leaf_cull)
                    + usize::from(self.frustum_first_mesh)
                    + usize::from(self.cave_sky_cull)
                    + usize::from(self.sign_text_cull)
                    + usize::from(self.offscreen_entity_skip)
                    + usize::from(self.distant_lod_boost)
                    + usize::from(self.distant_lod_boost && self.distant_lod_distance != 96);
                (on, 7)
            }
            BoostCategory::GeometryAndMeshing => {
                let on = usize::from(self.coplanar_quad_merge)
                    + usize::from(self.zero_stutter_mesh_pool)
                    + usize::from(self.sealed_cave_occlusion)
                    + usize::from(self.smart_frame_pacer)
                    + usize::from(self.frame_items_loader);
                (on, 5)
            }
            BoostCategory::ChunkStreaming => {
                let on = usize::from(self.chunk_upload_cap > 0)
                    + usize::from(self.chunk_build_distance != 12)
                    + usize::from(self.chunk_fade_mode != ChunkFadeMode::Full)
                    + usize::from(self.fast_weather);
                (on, 4)
            }
            BoostCategory::SimulationAndTicks => {
                let on = usize::from(self.block_tick_range != TickRangeMode::Normal)
                    + usize::from(self.entity_tick_range != TickRangeMode::Normal)
                    + usize::from(self.mob_pose_throttle)
                    + usize::from(self.max_entities > 0)
                    + usize::from(self.dropped_item_cap > 0)
                    + usize::from(self.block_entity_cap > 0)
                    + usize::from(self.merge_drop_stacks)
                    + usize::from(self.mob_frame_gate.is_active())
                    + usize::from(self.mob_frame_gate.is_active() && self.mob_gate_distance != 32)
                    + usize::from(self.player_frame_gate.is_active())
                    + usize::from(self.player_frame_gate.is_active() && self.player_gate_distance != 48)
                    + usize::from(self.item_frame_gate.is_active())
                    + usize::from(self.item_frame_gate.is_active() && self.item_gate_distance != 24)
                    + usize::from(self.block_entity_frame_gate.is_active())
                    + usize::from(
                        self.block_entity_frame_gate.is_active()
                            && self.block_entity_gate_distance != 32,
                    )
                    + usize::from(self.particle_frame_gate.is_active())
                    + usize::from(
                        self.particle_frame_gate.is_active() && self.particle_gate_distance != 24,
                    );
                (on, 17)
            }
            BoostCategory::EffectsAndVisuals => {
                let on = usize::from(self.particle_budget != ParticleBudget::Unlimited)
                    + usize::from(self.burst_lag_shield)
                    + usize::from(self.entity_shadows)
                    + usize::from(self.entity_shadows && self.entity_shadow_distance != 32)
                    + usize::from(self.smart_10hz_atlas)
                    + usize::from(self.glint_trim_mode != GlintTrimMode::All)
                    + usize::from(self.menu_backdrop != MenuBackdropMode::Full)
                    + usize::from(self.dynamic_menu_fps);
                (on, 8)
            }
            BoostCategory::DisplayAndFramerate => {
                let on = usize::from(self.auto_drs_scale)
                    + usize::from(self.auto_drs_scale && self.drs_target_fps != 60)
                    + usize::from(self.auto_drs_scale && self.drs_min_scale_pct != 50)
                    + usize::from(self.auto_drs_scale && self.drs_max_scale_pct != 100)
                    + usize::from(self.fps_cap != 120)
                    + usize::from(self.inactivity_fps != InactivityFpsMode::Off)
                    + usize::from(
                        self.inactivity_fps != InactivityFpsMode::Off
                            && self.inactivity_delay_secs != 30,
                    )
                    + usize::from(self.thermal_guard);
                (on, 8)
            }
            BoostCategory::Threads => {
                let on = usize::from(self.big_little_boost)
                    + usize::from(self.background_worker_coalescing)
                    + usize::from(self.storm_wind_boost);
                (on, 3)
            }
            BoostCategory::Replay => {
                let on = usize::from(self.replay_recording)
                    + usize::from(self.replay_hud_button)
                    + usize::from(self.replay_length != ReplayLength::Min5)
                    + usize::from(self.replay_quality != ReplayQuality::Medium)
                    + usize::from(self.replay_capture_hud)
                    + usize::from(self.live_storage_clip_count > 0);
                (on, 7)
            }
        }
    }

    pub fn category_master_on(&self, cat: BoostCategory) -> bool {
        match cat {
            BoostCategory::ProfilesAndDiagnostics => {
                self.profile != BoostProfile::Off || self.live_perf_hud || self.volt_reflex
            }
            BoostCategory::CullingAndVisibility => {
                self.smart_leaf_cull
                    || self.frustum_first_mesh
                    || self.cave_sky_cull
                    || self.sign_text_cull
                    || self.offscreen_entity_skip
                    || self.distant_lod_boost
            }
            BoostCategory::GeometryAndMeshing => {
                self.coplanar_quad_merge
                    || self.zero_stutter_mesh_pool
                    || self.sealed_cave_occlusion
                    || self.smart_frame_pacer
                    || self.frame_items_loader
            }
            BoostCategory::ChunkStreaming => {
                self.chunk_upload_cap > 0
                    || self.chunk_build_distance != 12
                    || self.chunk_fade_mode != ChunkFadeMode::Full
                    || self.fast_weather
            }
            BoostCategory::SimulationAndTicks => {
                self.block_tick_range != TickRangeMode::Normal
                    || self.entity_tick_range != TickRangeMode::Normal
                    || self.mob_pose_throttle
                    || self.max_entities > 0
                    || self.dropped_item_cap > 0
                    || self.block_entity_cap > 0
                    || self.merge_drop_stacks
                    || self.mob_frame_gate.is_active()
                    || self.player_frame_gate.is_active()
                    || self.item_frame_gate.is_active()
                    || self.block_entity_frame_gate.is_active()
                    || self.particle_frame_gate.is_active()
            }
            BoostCategory::EffectsAndVisuals => {
                self.particle_budget != ParticleBudget::Unlimited
                    || self.burst_lag_shield
                    || self.smart_10hz_atlas
                    || self.glint_trim_mode != GlintTrimMode::All
                    || self.menu_backdrop != MenuBackdropMode::Full
                    || self.dynamic_menu_fps
            }
            BoostCategory::DisplayAndFramerate => {
                self.auto_drs_scale
                    || self.inactivity_fps != InactivityFpsMode::Off
                    || self.thermal_guard
            }
            BoostCategory::Threads => {
                self.big_little_boost
                    || self.background_worker_coalescing
                    || self.storm_wind_boost
            }
            BoostCategory::Replay => {
                self.replay_recording || self.replay_hud_button || self.replay_capture_hud
            }
        }
    }

    pub fn set_category_master(&mut self, cat: BoostCategory, on: bool) {
        match cat {
            BoostCategory::ProfilesAndDiagnostics => {
                if on {
                    self.apply_profile(BoostProfile::Balanced);
                    self.live_perf_hud = true;
                    self.volt_reflex = true;
                } else {
                    self.profile = BoostProfile::Off;
                    self.mod_lag_shield = false;
                    self.live_perf_hud = false;
                    self.volt_reflex = false;
                }
            }
            BoostCategory::CullingAndVisibility => {
                self.smart_leaf_cull = on;
                self.frustum_first_mesh = on;
                self.cave_sky_cull = on;
                self.sign_text_cull = on;
                self.offscreen_entity_skip = on;
                self.distant_lod_boost = on;
            }
            BoostCategory::GeometryAndMeshing => {
                self.coplanar_quad_merge = on;
                self.zero_stutter_mesh_pool = on;
                self.sealed_cave_occlusion = on;
                self.smart_frame_pacer = on;
                self.frame_items_loader = on;
            }
            BoostCategory::ChunkStreaming => {
                if on {
                    self.chunk_upload_cap = 16;
                    self.chunk_build_distance = 10;
                    self.chunk_fade_mode = ChunkFadeMode::Fast;
                    self.fast_weather = true;
                } else {
                    self.chunk_upload_cap = 0;
                    self.chunk_build_distance = 12;
                    self.chunk_fade_mode = ChunkFadeMode::Full;
                    self.fast_weather = false;
                }
            }
            BoostCategory::SimulationAndTicks => {
                if on {
                    self.block_tick_range = TickRangeMode::Reduced;
                    self.entity_tick_range = TickRangeMode::Reduced;
                    self.mob_pose_throttle = true;
                    self.max_entities = 128;
                    self.dropped_item_cap = 96;
                    self.block_entity_cap = 192;
                    self.merge_drop_stacks = true;
                    self.mob_frame_gate = FrameGate::Every2nd;
                    self.player_frame_gate = FrameGate::Off;
                    self.item_frame_gate = FrameGate::Every2nd;
                    self.block_entity_frame_gate = FrameGate::Every2nd;
                    self.particle_frame_gate = FrameGate::Every2nd;
                } else {
                    self.block_tick_range = TickRangeMode::Normal;
                    self.entity_tick_range = TickRangeMode::Normal;
                    self.mob_pose_throttle = false;
                    self.max_entities = 0;
                    self.dropped_item_cap = 0;
                    self.block_entity_cap = 0;
                    self.merge_drop_stacks = false;
                    self.mob_frame_gate = FrameGate::Off;
                    self.player_frame_gate = FrameGate::Off;
                    self.item_frame_gate = FrameGate::Off;
                    self.block_entity_frame_gate = FrameGate::Off;
                    self.particle_frame_gate = FrameGate::Off;
                }
            }
            BoostCategory::EffectsAndVisuals => {
                if on {
                    self.particle_budget = ParticleBudget::Medium;
                    self.burst_lag_shield = true;
                    self.entity_shadows = true;
                    self.entity_shadow_distance = 24;
                    self.smart_10hz_atlas = true;
                    self.glint_trim_mode = GlintTrimMode::Nearby;
                    self.menu_backdrop = MenuBackdropMode::Blurred;
                    self.dynamic_menu_fps = true;
                } else {
                    self.particle_budget = ParticleBudget::Unlimited;
                    self.burst_lag_shield = false;
                    self.entity_shadows = true;
                    self.entity_shadow_distance = 32;
                    self.smart_10hz_atlas = false;
                    self.glint_trim_mode = GlintTrimMode::All;
                    self.menu_backdrop = MenuBackdropMode::Full;
                    self.dynamic_menu_fps = false;
                }
            }
            BoostCategory::DisplayAndFramerate => {
                if on {
                    self.auto_drs_scale = true;
                    self.inactivity_fps = InactivityFpsMode::Fps30;
                    self.thermal_guard = true;
                } else {
                    self.auto_drs_scale = false;
                    self.inactivity_fps = InactivityFpsMode::Off;
                    self.thermal_guard = false;
                }
            }
            BoostCategory::Threads => {
                self.big_little_boost = on;
                self.background_worker_coalescing = on;
                self.storm_wind_boost = on;
            }
            BoostCategory::Replay => {
                self.replay_recording = on;
                self.replay_hud_button = on;
                self.replay_capture_hud = on;
            }
        }
        self.sanitize();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoostCategory {
    ProfilesAndDiagnostics,
    CullingAndVisibility,
    GeometryAndMeshing,
    ChunkStreaming,
    SimulationAndTicks,
    EffectsAndVisuals,
    DisplayAndFramerate,
    Threads,
    Replay,
}

impl BoostCategory {
    pub const ALL: [Self; 9] = [
        Self::ProfilesAndDiagnostics,
        Self::CullingAndVisibility,
        Self::GeometryAndMeshing,
        Self::ChunkStreaming,
        Self::SimulationAndTicks,
        Self::EffectsAndVisuals,
        Self::DisplayAndFramerate,
        Self::Threads,
        Self::Replay,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::ProfilesAndDiagnostics => "1. Profiles & Diagnostics",
            Self::CullingAndVisibility => "2. Culling & Visibility",
            Self::GeometryAndMeshing => "3. Geometry & Meshing",
            Self::ChunkStreaming => "4. Chunk Streaming",
            Self::SimulationAndTicks => "5. Simulation & Ticks",
            Self::EffectsAndVisuals => "6. Effects, Particles & Visual Overhead",
            Self::DisplayAndFramerate => "7. Display & Framerate",
            Self::Threads => "8. Threads",
            Self::Replay => "9. Replay",
        }
    }
}

/// Visual dropped-item stack merge helper: merges nearby matching items within 1.5 blocks.
pub fn merge_nearby_drop_stacks<T: Clone>(
    items: &[(i32, [f64; 3], u32, T)],
) -> Vec<(i32, [f64; 3], u32, T)> {
    const MERGE_DIST_SQ: f64 = 1.5 * 1.5;
    let mut merged: Vec<(i32, [f64; 3], u32, T)> = Vec::with_capacity(items.len());
    for (item_id, pos, count, extra) in items {
        if let Some(existing) = merged.iter_mut().find(|(id, epos, _, _)| {
            *id == *item_id && {
                let dx = epos[0] - pos[0];
                let dy = epos[1] - pos[1];
                let dz = epos[2] - pos[2];
                dx * dx + dy * dy + dz * dz <= MERGE_DIST_SQ
            }
        }) {
            existing.2 = existing.2.saturating_add(*count);
        } else {
            merged.push((*item_id, *pos, *count, extra.clone()));
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe_and_round_trip_through_disk() {
        let dir = std::env::temp_dir().join(format!(
            "volt_boost_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join(FILE_NAME);
        let mut s = VoltPlusBoostSettings::default();
        assert_eq!(s.profile, BoostProfile::Off);
        assert!(!s.replay_recording);
        assert!(s.entity_shadows);

        s.apply_profile(BoostProfile::UltraFps);
        assert!(s.mod_lag_shield);
        assert!(s.smart_leaf_cull);
        assert!(s.coplanar_quad_merge);
        assert!(s.big_little_boost);
        s.replay_recording = true;
        s.replay_storage_budget_mb = 2048;
        s.replay_hud_layout = ReplayHudButtonLayout {
            norm_x: 0.25,
            norm_y: 0.75,
            size: 1.4,
            opacity: 0.6,
        };
        s.save(&path).unwrap();

        let loaded = VoltPlusBoostSettings::load(&path);
        assert_eq!(loaded.profile, BoostProfile::UltraFps);
        assert!(loaded.mod_lag_shield);
        assert!(loaded.replay_recording);
        assert_eq!(loaded.replay_storage_budget_mb, 2048);
        assert_eq!(loaded.replay_hud_layout.size, 1.4);
        assert_eq!(loaded.replay_hud_layout.opacity, 0.6);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn drs_controller_scales_down_when_slow_and_recovers_when_fast() {
        let mut cfg = VoltPlusBoostSettings::default();
        cfg.auto_drs_scale = true;
        cfg.drs_target_fps = 60;
        cfg.drs_min_scale_pct = 50;
        cfg.drs_max_scale_pct = 100;
        let mut drs = DrsController::default();
        for _ in 0..40 {
            drs.update(&cfg, 28.0);
        }
        assert!(drs.current_scale_pct() < 100);
        for _ in 0..80 {
            drs.update(&cfg, 8.0);
        }
        assert!(drs.current_scale_pct() > 60);
    }

    #[test]
    fn frame_gate_and_thermal_and_drop_merge_have_observable_effects() {
        // Frame gate skips distant frames at configured stride
        assert!(FrameGate::Every3rd.should_run(0, 40.0, 24));
        assert!(!FrameGate::Every3rd.should_run(1, 40.0, 24));
        assert!(FrameGate::Every3rd.should_run(1, 10.0, 24));

        // Thermal guard steps cleanly
        assert_eq!(evaluate_thermal_step(false, Some(85.0), 40.0), ThermalStep::Normal);
        assert_eq!(evaluate_thermal_step(true, Some(64.0), 16.0), ThermalStep::Step1Scale);
        assert_eq!(evaluate_thermal_step(true, Some(72.0), 16.0), ThermalStep::Step2Fps);
        assert_eq!(evaluate_thermal_step(true, Some(80.0), 16.0), ThermalStep::Step3Effects);

        // Drop stack merge coalesces nearby matching drops
        let drops = vec![
            (1, [0.0, 64.0, 0.0], 3, ()),
            (1, [0.5, 64.0, 0.5], 4, ()),
            (2, [0.5, 64.0, 0.5], 1, ()),
        ];
        let merged = merge_nearby_drop_stacks(&drops);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].2, 7);
    }

    #[test]
    fn all_nine_categories_support_master_switch_and_counts() {
        let mut s = VoltPlusBoostSettings::default();
        for cat in BoostCategory::ALL {
            s.set_category_master(cat, true);
            let (on, total) = s.category_counts(cat);
            assert!(on > 0 && on <= total, "{cat:?}: {on}/{total}");
            assert!(s.category_master_on(cat), "{cat:?} master should be on");
            s.set_category_master(cat, false);
        }
    }
}
