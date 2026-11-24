//! Volt + Boost UI (`Page::VoltPlusBoost`):
//! Strictly two levels — 9 collapsible category cards (sticky header + master
//! switch + `"X of Y on"` active count) and option rows inside each category.
//! Near-black `#11141B` glass cards, 1 px translucent white hairlines, 12.5 card
//! radius, 10 knob radius, `#E03044` active accent + focus ring, ease-out-cubic
//! motion <= 250 ms (zero bounce, interruptible), 130% text scale safe, full
//! keyboard/gamepad navigation, and inline modals for storage budget & clear confirm.
use crate::hud::{HudAtlas, HudBatch, Rect, rgba};
use crate::settings::Settings;
use crate::text::Text;
use crate::ui;
use crate::volt_boost::{
    BoostCategory, BoostProfile, ChunkFadeMode, FrameGate, GlintTrimMode, InactivityFpsMode,
    MenuBackdropMode, ParticleBudget, PerfHudCorner, ReplayLength, ReplayQuality, TickRangeMode,
    VoltPlusBoostSettings,
};
use std::time::Instant;
use winit::keyboard::{Key, NamedKey};

// Exact Volt + Boost palette (ABGR packing via `hud::rgba`).
const BG_SCRIM: u32 = rgba(8, 10, 14, 210);
const CARD_BG: u32 = rgba(17, 20, 27, 238); // #11141B
const CARD_HEADER_BG: u32 = rgba(22, 26, 35, 245);
const ROW_BG: u32 = rgba(21, 25, 34, 215);
const ROW_HOVER_BG: u32 = rgba(30, 35, 48, 238);
const HAIRLINE: u32 = rgba(255, 255, 255, 30);
const HAIRLINE_STRONG: u32 = rgba(255, 255, 255, 58);
const ACCENT: u32 = rgba(224, 48, 68, 255); // #E03044
const ACCENT_SOFT: u32 = rgba(224, 48, 68, 48);
const TEXT_PRIMARY: u32 = rgba(242, 245, 243, 255); // #F2F5F3
const TEXT_MUTED: u32 = rgba(198, 204, 201, 255); // #C6CCC9
const TEXT_DIM: u32 = rgba(143, 150, 155, 255); // #8F969B
const TRACK_OFF: u32 = rgba(46, 52, 66, 255);
const KNOB_COLOR: u32 = rgba(242, 245, 243, 255);

const CARD_RADIUS: f32 = 12.5;
const KNOB_RADIUS: f32 = 10.0;
const PAGE_OPEN_MS: f32 = 220.0;
const EXPAND_MS: f32 = 220.0;
const TOGGLE_MS: f32 = 180.0;
const STAGGER_PER_ROW_MS: f32 = 18.0;
const PRESS_SCALE: f32 = 0.958;

#[inline]
fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltBoostAction {
    ButtonClick,
    Changed,
    SliderDrag,
    PurgeRam,
    ClearReplays,
    OpenReplays,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BoostOptionId {
    // 1. Profiles & Diagnostics
    BoostProfile,
    AutoTuneDevice,
    PurgeUnusedRam,
    LivePerfHud,
    PerfHudCorner,
    VoltReflex,
    // 2. Culling & Visibility
    SmartLeafCull,
    FrustumFirstMesh,
    CaveSkyCull,
    SignTextCull,
    OffscreenEntitySkip,
    DistantLodBoost,
    DistantLodDistance,
    // 3. Geometry & Meshing
    CoplanarQuadMerge,
    ZeroStutterMeshPool,
    SealedCaveOcclusion,
    SmartFramePacer,
    FrameItemsLoader,
    // 4. Chunk Streaming
    ChunkUploadCap,
    ChunkBuildDistance,
    ChunkFade,
    FastWeather,
    // 5. Simulation & Ticks
    BlockTickRange,
    EntityTickRange,
    MobPoseThrottle,
    MaxEntities,
    DroppedItemCap,
    BlockEntityCap,
    MergeDropStacks,
    MobFrameGate,
    MobGateDistance,
    PlayerFrameGate,
    PlayerGateDistance,
    ItemFrameGate,
    ItemGateDistance,
    BlockEntityFrameGate,
    BlockEntityGateDistance,
    ParticleFrameGate,
    ParticleGateDistance,
    // 6. Effects & Visuals
    ParticleBudget,
    BurstLagShield,
    EntityShadows,
    EntityShadowDistance,
    Smart10HzAtlas,
    GlintTrimMode,
    MenuBackdrop,
    DynamicMenuFps,
    // 7. Display & Framerate
    AutoDrsScale,
    DrsTargetFps,
    DrsMinScale,
    DrsMaxScale,
    FpsCap,
    InactivityFps,
    InactivityDelay,
    ThermalGuard,
    // 8. Threads
    BigLittleBoost,
    BackgroundWorkerCoalescing,
    StormWindBoost,
    // 9. Replay
    ReplayRecording,
    OnScreenReplayButton,
    RecordingLength,
    RecordingQuality,
    CaptureHud,
    StorageUsedClear,
    OpenReplays,
}

impl BoostOptionId {
    pub fn category(self) -> BoostCategory {
        use BoostCategory::*;
        match self {
            Self::BoostProfile
            | Self::AutoTuneDevice
            | Self::PurgeUnusedRam
            | Self::LivePerfHud
            | Self::PerfHudCorner
            | Self::VoltReflex => ProfilesAndDiagnostics,

            Self::SmartLeafCull
            | Self::FrustumFirstMesh
            | Self::CaveSkyCull
            | Self::SignTextCull
            | Self::OffscreenEntitySkip
            | Self::DistantLodBoost
            | Self::DistantLodDistance => CullingAndVisibility,

            Self::CoplanarQuadMerge
            | Self::ZeroStutterMeshPool
            | Self::SealedCaveOcclusion
            | Self::SmartFramePacer
            | Self::FrameItemsLoader => GeometryAndMeshing,

            Self::ChunkUploadCap
            | Self::ChunkBuildDistance
            | Self::ChunkFade
            | Self::FastWeather => ChunkStreaming,

            Self::BlockTickRange
            | Self::EntityTickRange
            | Self::MobPoseThrottle
            | Self::MaxEntities
            | Self::DroppedItemCap
            | Self::BlockEntityCap
            | Self::MergeDropStacks
            | Self::MobFrameGate
            | Self::MobGateDistance
            | Self::PlayerFrameGate
            | Self::PlayerGateDistance
            | Self::ItemFrameGate
            | Self::ItemGateDistance
            | Self::BlockEntityFrameGate
            | Self::BlockEntityGateDistance
            | Self::ParticleFrameGate
            | Self::ParticleGateDistance => SimulationAndTicks,

            Self::ParticleBudget
            | Self::BurstLagShield
            | Self::EntityShadows
            | Self::EntityShadowDistance
            | Self::Smart10HzAtlas
            | Self::GlintTrimMode
            | Self::MenuBackdrop
            | Self::DynamicMenuFps => EffectsAndVisuals,

            Self::AutoDrsScale
            | Self::DrsTargetFps
            | Self::DrsMinScale
            | Self::DrsMaxScale
            | Self::FpsCap
            | Self::InactivityFps
            | Self::InactivityDelay
            | Self::ThermalGuard => DisplayAndFramerate,

            Self::BigLittleBoost
            | Self::BackgroundWorkerCoalescing
            | Self::StormWindBoost => Threads,

            Self::ReplayRecording
            | Self::OnScreenReplayButton
            | Self::RecordingLength
            | Self::RecordingQuality
            | Self::CaptureHud
            | Self::StorageUsedClear
            | Self::OpenReplays => Replay,
        }
    }

    pub fn options_in(cat: BoostCategory) -> &'static [Self] {
        match cat {
            BoostCategory::ProfilesAndDiagnostics => &[
                Self::BoostProfile,
                Self::AutoTuneDevice,
                Self::PurgeUnusedRam,
                Self::LivePerfHud,
                Self::PerfHudCorner,
                Self::VoltReflex,
            ],
            BoostCategory::CullingAndVisibility => &[
                Self::SmartLeafCull,
                Self::FrustumFirstMesh,
                Self::CaveSkyCull,
                Self::SignTextCull,
                Self::OffscreenEntitySkip,
                Self::DistantLodBoost,
                Self::DistantLodDistance,
            ],
            BoostCategory::GeometryAndMeshing => &[
                Self::CoplanarQuadMerge,
                Self::ZeroStutterMeshPool,
                Self::SealedCaveOcclusion,
                Self::SmartFramePacer,
                Self::FrameItemsLoader,
            ],
            BoostCategory::ChunkStreaming => &[
                Self::ChunkUploadCap,
                Self::ChunkBuildDistance,
                Self::ChunkFade,
                Self::FastWeather,
            ],
            BoostCategory::SimulationAndTicks => &[
                Self::BlockTickRange,
                Self::EntityTickRange,
                Self::MobPoseThrottle,
                Self::MaxEntities,
                Self::DroppedItemCap,
                Self::BlockEntityCap,
                Self::MergeDropStacks,
                Self::MobFrameGate,
                Self::MobGateDistance,
                Self::PlayerFrameGate,
                Self::PlayerGateDistance,
                Self::ItemFrameGate,
                Self::ItemGateDistance,
                Self::BlockEntityFrameGate,
                Self::BlockEntityGateDistance,
                Self::ParticleFrameGate,
                Self::ParticleGateDistance,
            ],
            BoostCategory::EffectsAndVisuals => &[
                Self::ParticleBudget,
                Self::BurstLagShield,
                Self::EntityShadows,
                Self::EntityShadowDistance,
                Self::Smart10HzAtlas,
                Self::GlintTrimMode,
                Self::MenuBackdrop,
                Self::DynamicMenuFps,
            ],
            BoostCategory::DisplayAndFramerate => &[
                Self::AutoDrsScale,
                Self::DrsTargetFps,
                Self::DrsMinScale,
                Self::DrsMaxScale,
                Self::FpsCap,
                Self::InactivityFps,
                Self::InactivityDelay,
                Self::ThermalGuard,
            ],
            BoostCategory::Threads => &[
                Self::BigLittleBoost,
                Self::BackgroundWorkerCoalescing,
                Self::StormWindBoost,
            ],
            BoostCategory::Replay => &[
                Self::ReplayRecording,
                Self::OnScreenReplayButton,
                Self::RecordingLength,
                Self::RecordingQuality,
                Self::CaptureHud,
                Self::StorageUsedClear,
                Self::OpenReplays,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusTarget {
    Back,
    CategoryHeader(usize),
    CategoryMaster(usize),
    Option(BoostOptionId),
    ModalChoice(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoostModal {
    StorageBudgetPrompt,
    ConfirmClearReplays,
}

#[derive(Clone, Copy, Debug)]
struct ExpandAnim {
    from: f32,
    to: f32,
    started: Instant,
}

#[derive(Clone, Copy, Debug)]
struct ToggleAnim {
    from: f32,
    to: f32,
    started: Instant,
}

#[derive(Clone, Debug)]
struct HitRect {
    rect: Rect,
    kind: HitKind,
}

#[derive(Clone, Copy, Debug)]
enum HitKind {
    Back,
    CategoryHeader(usize),
    CategoryMaster(usize),
    OptionRow(BoostOptionId, Rect),
    SliderTrack(BoostOptionId, Rect),
    ModalChoice(usize),
    ModalBackdrop,
}

#[derive(Clone, Debug)]
pub struct StickyHeaderInfo {
    pub category_idx: usize,
    pub rect: Rect,
}

pub struct VoltBoostUi {
    pub scroll: f32,
    pub max_scroll: f32,
    pub expanded: [bool; 9],
    expand_anim: [ExpandAnim; 9],
    master_stagger_at: [Option<Instant>; 9],
    toggle_anims: Vec<(BoostOptionId, ToggleAnim)>,
    opened_at: Instant,
    pub focus: Option<FocusTarget>,
    pub modal: Option<BoostModal>,
    pub dragging_slider: Option<BoostOptionId>,
    pub dragging_scroll_y: Option<f32>,
    pub pressed: Option<FocusTarget>,
    pub text_scale: f32,
    pub safe_insets: [f32; 4], // top, right, bottom, left
    pub sticky_header: Option<StickyHeaderInfo>,
    hits: Vec<HitRect>,
    focus_order: Vec<FocusTarget>,
}

impl Default for VoltBoostUi {
    fn default() -> Self {
        let now = Instant::now();
        let expanded = [true, false, false, false, false, false, false, false, true];
        let expand_anim = std::array::from_fn(|i| {
            let v = if expanded[i] { 1.0 } else { 0.0 };
            ExpandAnim {
                from: v,
                to: v,
                started: now,
            }
        });
        Self {
            scroll: 0.0,
            max_scroll: 0.0,
            expanded,
            expand_anim,
            master_stagger_at: [None; 9],
            toggle_anims: Vec::new(),
            opened_at: now,
            focus: None,
            modal: None,
            dragging_slider: None,
            dragging_scroll_y: None,
            pressed: None,
            text_scale: 1.0,
            safe_insets: [0.0; 4],
            sticky_header: None,
            hits: Vec::new(),
            focus_order: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
enum RowKind {
    Toggle(bool),
    Cycle,
    Slider { norm: f32 },
    Action(&'static str),
}

#[derive(Clone, Debug)]
struct RowSpec {
    id: BoostOptionId,
    label: &'static str,
    state_word: String,
    hint: Option<String>,
    enabled: bool,
    kind: RowKind,
}

impl VoltBoostUi {
    pub fn on_open(&mut self) {
        self.opened_at = Instant::now();
        self.modal = None;
        self.dragging_slider = None;
        self.pressed = None;
    }

    pub fn dp_unit(&self, w: f32, h: f32) -> f32 {
        let _ = w;
        if h >= 420.0 {
            1.0
        } else {
            (h / 360.0).clamp(0.68, 1.0)
        }
    }

    pub fn expand_progress(&self, cat_idx: usize) -> f32 {
        let anim = self.expand_anim[cat_idx];
        let t = ease_out_cubic(anim.started.elapsed().as_secs_f32() * 1000.0 / EXPAND_MS);
        anim.from + (anim.to - anim.from) * t
    }

    pub fn toggle_category(&mut self, cat_idx: usize) {
        let current = self.expand_progress(cat_idx);
        let next = !self.expanded[cat_idx];
        self.expanded[cat_idx] = next;
        self.expand_anim[cat_idx] = ExpandAnim {
            from: current,
            to: if next { 1.0 } else { 0.0 },
            started: Instant::now(),
        };
    }

    fn toggle_knob_t(&self, id: BoostOptionId, target_on: bool) -> f32 {
        let target = if target_on { 1.0 } else { 0.0 };
        if let Some((_, anim)) = self.toggle_anims.iter().find(|(k, _)| *k == id) {
            let t = ease_out_cubic(anim.started.elapsed().as_secs_f32() * 1000.0 / TOGGLE_MS);
            anim.from + (anim.to - anim.from) * t
        } else {
            target
        }
    }

    fn start_toggle_anim(&mut self, id: BoostOptionId, was_on: bool, now_on: bool) {
        let from = self.toggle_knob_t(id, was_on);
        let anim = ToggleAnim {
            from,
            to: if now_on { 1.0 } else { 0.0 },
            started: Instant::now(),
        };
        if let Some((_, existing)) = self.toggle_anims.iter_mut().find(|(k, _)| *k == id) {
            *existing = anim;
        } else {
            self.toggle_anims.push((id, anim));
        }
    }

    pub fn is_animating(&self) -> bool {
        if self.opened_at.elapsed().as_secs_f32() * 1000.0 < PAGE_OPEN_MS {
            return true;
        }
        if self
            .expand_anim
            .iter()
            .any(|a| a.started.elapsed().as_secs_f32() * 1000.0 < EXPAND_MS)
        {
            return true;
        }
        if self
            .toggle_anims
            .iter()
            .any(|(_, a)| a.started.elapsed().as_secs_f32() * 1000.0 < TOGGLE_MS)
        {
            return true;
        }
        self.master_stagger_at
            .iter()
            .flatten()
            .any(|t| t.elapsed().as_secs_f32() * 1000.0 < 360.0)
    }

    fn row_spec(vb: &VoltPlusBoostSettings, id: BoostOptionId) -> RowSpec {
        match id {
            BoostOptionId::BoostProfile => RowSpec {
                id,
                label: "Boost Profile",
                state_word: vb.profile.label().into(),
                hint: Some(if vb.mod_lag_shield {
                    "Mod Lag Shield: Armed".into()
                } else {
                    "Mod Lag Shield: Off (arms on any Boost Profile)".into()
                }),
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::AutoTuneDevice => RowSpec {
                id,
                label: "Auto-Tune Device",
                state_word: if vb.auto_tune_summary.is_empty() {
                    "Detect Hardware".into()
                } else {
                    "Applied".into()
                },
                hint: (!vb.auto_tune_summary.is_empty()).then(|| vb.auto_tune_summary.clone()),
                enabled: true,
                kind: RowKind::Action("Auto-Tune"),
            },
            BoostOptionId::PurgeUnusedRam => RowSpec {
                id,
                label: "Purge Unused RAM",
                state_word: if vb.last_ram_freed_mb > 0.0 {
                    format!("Freed {:.1} MB", vb.last_ram_freed_mb)
                } else {
                    "Ready".into()
                },
                hint: None,
                enabled: true,
                kind: RowKind::Action("Purge"),
            },
            BoostOptionId::LivePerfHud => RowSpec {
                id,
                label: "Live Perf HUD",
                state_word: on_off(vb.live_perf_hud),
                hint: Some("FPS, 1% low, frame time, RAM, draw calls, active count".into()),
                enabled: true,
                kind: RowKind::Toggle(vb.live_perf_hud),
            },
            BoostOptionId::PerfHudCorner => RowSpec {
                id,
                label: "Perf HUD Corner",
                state_word: vb.perf_hud_corner.label().into(),
                hint: (!vb.live_perf_hud).then(|| "Requires Live Perf HUD".into()),
                enabled: vb.live_perf_hud,
                kind: RowKind::Cycle,
            },
            BoostOptionId::VoltReflex => RowSpec {
                id,
                label: "Volt Reflex",
                state_word: if vb.volt_reflex {
                    if vb.live_reflex_saved_ms > 0.0 {
                        format!("On (-{:.1} ms)", vb.live_reflex_saved_ms)
                    } else {
                        "On (Low Latency)".into()
                    }
                } else {
                    "Off".into()
                },
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.volt_reflex),
            },
            BoostOptionId::SmartLeafCull => RowSpec {
                id,
                label: "Smart Leaf Cull",
                state_word: on_off(vb.smart_leaf_cull),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.smart_leaf_cull),
            },
            BoostOptionId::FrustumFirstMesh => RowSpec {
                id,
                label: "Frustum-First Mesh",
                state_word: on_off(vb.frustum_first_mesh),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.frustum_first_mesh),
            },
            BoostOptionId::CaveSkyCull => RowSpec {
                id,
                label: "Cave Sky Cull",
                state_word: on_off(vb.cave_sky_cull),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.cave_sky_cull),
            },
            BoostOptionId::SignTextCull => RowSpec {
                id,
                label: "Sign Text Cull",
                state_word: on_off(vb.sign_text_cull),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.sign_text_cull),
            },
            BoostOptionId::OffscreenEntitySkip => RowSpec {
                id,
                label: "Offscreen Entity Skip",
                state_word: on_off(vb.offscreen_entity_skip),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.offscreen_entity_skip),
            },
            BoostOptionId::DistantLodBoost => RowSpec {
                id,
                label: "Distant LOD Boost",
                state_word: on_off(vb.distant_lod_boost),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.distant_lod_boost),
            },
            BoostOptionId::DistantLodDistance => RowSpec {
                id,
                label: "Distant LOD Distance",
                state_word: format!("{} ch", vb.distant_lod_distance),
                hint: (!vb.distant_lod_boost).then(|| "Requires Distant LOD Boost".into()),
                enabled: vb.distant_lod_boost,
                kind: RowKind::Slider {
                    norm: (vb.distant_lod_distance as f32 - 32.0) / (256.0 - 32.0),
                },
            },
            BoostOptionId::CoplanarQuadMerge => RowSpec {
                id,
                label: "Coplanar Quad Merge",
                state_word: on_off(vb.coplanar_quad_merge),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.coplanar_quad_merge),
            },
            BoostOptionId::ZeroStutterMeshPool => RowSpec {
                id,
                label: "Zero-Stutter Mesh Pool",
                state_word: on_off(vb.zero_stutter_mesh_pool),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.zero_stutter_mesh_pool),
            },
            BoostOptionId::SealedCaveOcclusion => RowSpec {
                id,
                label: "Sealed Cave Occlusion",
                state_word: on_off(vb.sealed_cave_occlusion),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.sealed_cave_occlusion),
            },
            BoostOptionId::SmartFramePacer => RowSpec {
                id,
                label: "Smart Frame Pacer",
                state_word: on_off(vb.smart_frame_pacer),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.smart_frame_pacer),
            },
            BoostOptionId::FrameItemsLoader => RowSpec {
                id,
                label: "Frame Items Loader",
                state_word: on_off(vb.frame_items_loader),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.frame_items_loader),
            },
            BoostOptionId::ChunkUploadCap => RowSpec {
                id,
                label: "Chunk Upload Cap",
                state_word: if vb.chunk_upload_cap == 0 {
                    "Unlimited".into()
                } else {
                    format!("{} / frame", vb.chunk_upload_cap)
                },
                hint: None,
                enabled: true,
                kind: RowKind::Slider {
                    norm: vb.chunk_upload_cap as f32 / 64.0,
                },
            },
            BoostOptionId::ChunkBuildDistance => RowSpec {
                id,
                label: "Chunk Build Distance",
                state_word: format!("{} ch", vb.chunk_build_distance),
                hint: None,
                enabled: true,
                kind: RowKind::Slider {
                    norm: (vb.chunk_build_distance as f32 - 2.0) / 30.0,
                },
            },
            BoostOptionId::ChunkFade => RowSpec {
                id,
                label: "Chunk Fade",
                state_word: vb.chunk_fade_mode.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::FastWeather => RowSpec {
                id,
                label: "Fast Weather",
                state_word: on_off(vb.fast_weather),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.fast_weather),
            },
            BoostOptionId::BlockTickRange => RowSpec {
                id,
                label: "Block Tick Range",
                state_word: vb.block_tick_range.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::EntityTickRange => RowSpec {
                id,
                label: "Entity Tick Range",
                state_word: vb.entity_tick_range.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::MobPoseThrottle => RowSpec {
                id,
                label: "Mob Pose Throttle",
                state_word: on_off(vb.mob_pose_throttle),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.mob_pose_throttle),
            },
            BoostOptionId::MaxEntities => RowSpec {
                id,
                label: "Max Entities",
                state_word: cap_word(vb.max_entities),
                hint: None,
                enabled: true,
                kind: RowKind::Slider {
                    norm: vb.max_entities as f32 / 256.0,
                },
            },
            BoostOptionId::DroppedItemCap => RowSpec {
                id,
                label: "Dropped Item Cap",
                state_word: cap_word(vb.dropped_item_cap),
                hint: None,
                enabled: true,
                kind: RowKind::Slider {
                    norm: vb.dropped_item_cap as f32 / 256.0,
                },
            },
            BoostOptionId::BlockEntityCap => RowSpec {
                id,
                label: "Block-Entity Cap",
                state_word: cap_word(vb.block_entity_cap),
                hint: None,
                enabled: true,
                kind: RowKind::Slider {
                    norm: vb.block_entity_cap as f32 / 512.0,
                },
            },
            BoostOptionId::MergeDropStacks => RowSpec {
                id,
                label: "Merge Drop Stacks",
                state_word: on_off(vb.merge_drop_stacks),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.merge_drop_stacks),
            },
            BoostOptionId::MobFrameGate => RowSpec {
                id,
                label: "Mob Frame Gate",
                state_word: vb.mob_frame_gate.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::MobGateDistance => RowSpec {
                id,
                label: "Mob Gate Distance",
                state_word: format!("{} bl", vb.mob_gate_distance),
                hint: (!vb.mob_frame_gate.is_active()).then(|| "Requires Mob Frame Gate".into()),
                enabled: vb.mob_frame_gate.is_active(),
                kind: RowKind::Slider {
                    norm: (vb.mob_gate_distance as f32 - 8.0) / 88.0,
                },
            },
            BoostOptionId::PlayerFrameGate => RowSpec {
                id,
                label: "Other Players Frame Gate",
                state_word: vb.player_frame_gate.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::PlayerGateDistance => RowSpec {
                id,
                label: "Players Gate Distance",
                state_word: format!("{} bl", vb.player_gate_distance),
                hint: (!vb.player_frame_gate.is_active())
                    .then(|| "Requires Other Players Frame Gate".into()),
                enabled: vb.player_frame_gate.is_active(),
                kind: RowKind::Slider {
                    norm: (vb.player_gate_distance as f32 - 8.0) / 88.0,
                },
            },
            BoostOptionId::ItemFrameGate => RowSpec {
                id,
                label: "Dropped Items Frame Gate",
                state_word: vb.item_frame_gate.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::ItemGateDistance => RowSpec {
                id,
                label: "Items Gate Distance",
                state_word: format!("{} bl", vb.item_gate_distance),
                hint: (!vb.item_frame_gate.is_active())
                    .then(|| "Requires Dropped Items Frame Gate".into()),
                enabled: vb.item_frame_gate.is_active(),
                kind: RowKind::Slider {
                    norm: (vb.item_gate_distance as f32 - 8.0) / 88.0,
                },
            },
            BoostOptionId::BlockEntityFrameGate => RowSpec {
                id,
                label: "Block Entities Frame Gate",
                state_word: vb.block_entity_frame_gate.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::BlockEntityGateDistance => RowSpec {
                id,
                label: "Block Entities Distance",
                state_word: format!("{} bl", vb.block_entity_gate_distance),
                hint: (!vb.block_entity_frame_gate.is_active())
                    .then(|| "Requires Block Entities Frame Gate".into()),
                enabled: vb.block_entity_frame_gate.is_active(),
                kind: RowKind::Slider {
                    norm: (vb.block_entity_gate_distance as f32 - 8.0) / 88.0,
                },
            },
            BoostOptionId::ParticleFrameGate => RowSpec {
                id,
                label: "Particles Frame Gate",
                state_word: vb.particle_frame_gate.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::ParticleGateDistance => RowSpec {
                id,
                label: "Particles Gate Distance",
                state_word: format!("{} bl", vb.particle_gate_distance),
                hint: (!vb.particle_frame_gate.is_active())
                    .then(|| "Requires Particles Frame Gate".into()),
                enabled: vb.particle_frame_gate.is_active(),
                kind: RowKind::Slider {
                    norm: (vb.particle_gate_distance as f32 - 8.0) / 88.0,
                },
            },
            BoostOptionId::ParticleBudget => RowSpec {
                id,
                label: "Particle Budget",
                state_word: vb.particle_budget.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::BurstLagShield => RowSpec {
                id,
                label: "Burst Lag Shield",
                state_word: on_off(vb.burst_lag_shield),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.burst_lag_shield),
            },
            BoostOptionId::EntityShadows => RowSpec {
                id,
                label: "Entity Shadows",
                state_word: on_off(vb.entity_shadows),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.entity_shadows),
            },
            BoostOptionId::EntityShadowDistance => RowSpec {
                id,
                label: "Entity Shadow Distance",
                state_word: format!("{} bl", vb.entity_shadow_distance),
                hint: (!vb.entity_shadows).then(|| "Requires Entity Shadows".into()),
                enabled: vb.entity_shadows,
                kind: RowKind::Slider {
                    norm: (vb.entity_shadow_distance as f32 - 8.0) / 56.0,
                },
            },
            BoostOptionId::Smart10HzAtlas => RowSpec {
                id,
                label: "Smart 10Hz Atlas",
                state_word: on_off(vb.smart_10hz_atlas),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.smart_10hz_atlas),
            },
            BoostOptionId::GlintTrimMode => RowSpec {
                id,
                label: "Glint & Trim Mode",
                state_word: vb.glint_trim_mode.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::MenuBackdrop => RowSpec {
                id,
                label: "Menu Backdrop",
                state_word: vb.menu_backdrop.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::DynamicMenuFps => RowSpec {
                id,
                label: "Dynamic Menu FPS",
                state_word: on_off(vb.dynamic_menu_fps),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.dynamic_menu_fps),
            },
            BoostOptionId::AutoDrsScale => RowSpec {
                id,
                label: "Auto-DRS Scale",
                state_word: if vb.auto_drs_scale {
                    format!("On · {}%", vb.live_drs_scale_pct)
                } else {
                    "Off".into()
                },
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.auto_drs_scale),
            },
            BoostOptionId::DrsTargetFps => RowSpec {
                id,
                label: "DRS Target FPS",
                state_word: format!("{} FPS", vb.drs_target_fps),
                hint: (!vb.auto_drs_scale).then(|| "Requires Auto-DRS Scale".into()),
                enabled: vb.auto_drs_scale,
                kind: RowKind::Cycle,
            },
            BoostOptionId::DrsMinScale => RowSpec {
                id,
                label: "DRS Min Scale",
                state_word: format!("{}%", vb.drs_min_scale_pct),
                hint: (!vb.auto_drs_scale).then(|| "Requires Auto-DRS Scale".into()),
                enabled: vb.auto_drs_scale,
                kind: RowKind::Slider {
                    norm: (vb.drs_min_scale_pct as f32 - 35.0) / 55.0,
                },
            },
            BoostOptionId::DrsMaxScale => RowSpec {
                id,
                label: "DRS Max Scale",
                state_word: format!("{}%", vb.drs_max_scale_pct),
                hint: (!vb.auto_drs_scale).then(|| "Requires Auto-DRS Scale".into()),
                enabled: vb.auto_drs_scale,
                kind: RowKind::Slider {
                    norm: (vb.drs_max_scale_pct as f32 - 60.0) / 40.0,
                },
            },
            BoostOptionId::FpsCap => RowSpec {
                id,
                label: "FPS Cap",
                state_word: if vb.fps_cap >= 260 {
                    "Unlimited".into()
                } else {
                    format!("{} FPS", vb.fps_cap)
                },
                hint: None,
                enabled: true,
                kind: RowKind::Slider {
                    norm: (vb.fps_cap as f32 - 30.0) / 230.0,
                },
            },
            BoostOptionId::InactivityFps => RowSpec {
                id,
                label: "Inactivity FPS",
                state_word: vb.inactivity_fps.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::InactivityDelay => RowSpec {
                id,
                label: "Inactivity Delay",
                state_word: format!("{} s", vb.inactivity_delay_secs),
                hint: (vb.inactivity_fps == InactivityFpsMode::Off)
                    .then(|| "Requires Inactivity FPS".into()),
                enabled: vb.inactivity_fps != InactivityFpsMode::Off,
                kind: RowKind::Slider {
                    norm: (vb.inactivity_delay_secs as f32 - 5.0) / 115.0,
                },
            },
            BoostOptionId::ThermalGuard => RowSpec {
                id,
                label: "Thermal Guard",
                state_word: if vb.thermal_guard {
                    format!("On · {}", vb.live_thermal_step.label())
                } else {
                    "Off".into()
                },
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.thermal_guard),
            },
            BoostOptionId::BigLittleBoost => RowSpec {
                id,
                label: "big.LITTLE Boost",
                state_word: on_off(vb.big_little_boost),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.big_little_boost),
            },
            BoostOptionId::BackgroundWorkerCoalescing => RowSpec {
                id,
                label: "Worker Coalescing",
                state_word: on_off(vb.background_worker_coalescing),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.background_worker_coalescing),
            },
            BoostOptionId::StormWindBoost => RowSpec {
                id,
                label: "Storm Wind Boost",
                state_word: on_off(vb.storm_wind_boost),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.storm_wind_boost),
            },
            BoostOptionId::ReplayRecording => RowSpec {
                id,
                label: "Replay Recording",
                state_word: on_off(vb.replay_recording),
                hint: Some(format!(
                    "Budget: {} · Applies on next world join",
                    crate::replay::format_bytes(u64::from(vb.replay_storage_budget_mb) * 1024 * 1024)
                )),
                enabled: true,
                kind: RowKind::Toggle(vb.replay_recording),
            },
            BoostOptionId::OnScreenReplayButton => RowSpec {
                id,
                label: "On-Screen Replay Button",
                state_word: on_off(vb.replay_hud_button),
                hint: Some("Drag to move · Hold & drag up/down resize, left/right opacity".into()),
                enabled: true,
                kind: RowKind::Toggle(vb.replay_hud_button),
            },
            BoostOptionId::RecordingLength => RowSpec {
                id,
                label: "Recording Length",
                state_word: vb.replay_length.label().into(),
                hint: None,
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::RecordingQuality => RowSpec {
                id,
                label: "Recording Quality",
                state_word: vb.replay_quality.label().into(),
                hint: Some(format!(
                    "≈ {:.1} MB per minute",
                    vb.replay_quality.estimated_mb_per_min()
                )),
                enabled: true,
                kind: RowKind::Cycle,
            },
            BoostOptionId::CaptureHud => RowSpec {
                id,
                label: "Capture HUD",
                state_word: on_off(vb.replay_capture_hud),
                hint: None,
                enabled: true,
                kind: RowKind::Toggle(vb.replay_capture_hud),
            },
            BoostOptionId::StorageUsedClear => RowSpec {
                id,
                label: "Storage Used",
                state_word: format!(
                    "{} · {} clips",
                    crate::replay::format_bytes(vb.live_storage_used_bytes),
                    vb.live_storage_clip_count
                ),
                hint: None,
                enabled: true,
                kind: RowKind::Action("Clear Replays"),
            },
            BoostOptionId::OpenReplays => RowSpec {
                id,
                label: "Open Replays",
                state_word: "Launcher Tab".into(),
                hint: Some("Leaves current game and opens Replays in Launcher Settings".into()),
                enabled: true,
                kind: RowKind::Action("Open"),
            },
        }
    }

    fn apply_option_activate(
        &mut self,
        s: &mut Settings,
        id: BoostOptionId,
        dir: i32,
        actions: &mut Vec<VoltBoostAction>,
    ) {
        let vb = &mut s.volt_boost;
        match id {
            BoostOptionId::BoostProfile => {
                let next = vb.profile.cycle(dir);
                vb.apply_profile(next);
                s.sync_from_volt_boost();
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::AutoTuneDevice => {
                vb.run_auto_tune();
                s.sync_from_volt_boost();
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::PurgeUnusedRam => {
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::PurgeRam]);
            }
            BoostOptionId::LivePerfHud => {
                let was = vb.live_perf_hud;
                vb.live_perf_hud = !was;
                self.start_toggle_anim(id, was, vb.live_perf_hud);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::PerfHudCorner => {
                vb.perf_hud_corner = vb.perf_hud_corner.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::VoltReflex => {
                let was = vb.volt_reflex;
                vb.volt_reflex = !was;
                self.start_toggle_anim(id, was, vb.volt_reflex);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::SmartLeafCull => toggle_field!(self, vb.smart_leaf_cull, id, actions),
            BoostOptionId::FrustumFirstMesh => toggle_field!(self, vb.frustum_first_mesh, id, actions),
            BoostOptionId::CaveSkyCull => toggle_field!(self, vb.cave_sky_cull, id, actions),
            BoostOptionId::SignTextCull => toggle_field!(self, vb.sign_text_cull, id, actions),
            BoostOptionId::OffscreenEntitySkip => toggle_field!(self, vb.offscreen_entity_skip, id, actions),
            BoostOptionId::DistantLodBoost => {
                toggle_field!(self, vb.distant_lod_boost, id, actions);
                s.sync_from_volt_boost();
            }
            BoostOptionId::CoplanarQuadMerge => toggle_field!(self, vb.coplanar_quad_merge, id, actions),
            BoostOptionId::ZeroStutterMeshPool => toggle_field!(self, vb.zero_stutter_mesh_pool, id, actions),
            BoostOptionId::SealedCaveOcclusion => toggle_field!(self, vb.sealed_cave_occlusion, id, actions),
            BoostOptionId::SmartFramePacer => toggle_field!(self, vb.smart_frame_pacer, id, actions),
            BoostOptionId::FrameItemsLoader => toggle_field!(self, vb.frame_items_loader, id, actions),
            BoostOptionId::ChunkFade => {
                vb.chunk_fade_mode = vb.chunk_fade_mode.cycle(dir);
                s.sync_from_volt_boost();
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::FastWeather => toggle_field!(self, vb.fast_weather, id, actions),
            BoostOptionId::BlockTickRange => {
                vb.block_tick_range = vb.block_tick_range.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::EntityTickRange => {
                vb.entity_tick_range = vb.entity_tick_range.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::MobPoseThrottle => toggle_field!(self, vb.mob_pose_throttle, id, actions),
            BoostOptionId::MergeDropStacks => toggle_field!(self, vb.merge_drop_stacks, id, actions),
            BoostOptionId::MobFrameGate => {
                vb.mob_frame_gate = vb.mob_frame_gate.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::PlayerFrameGate => {
                vb.player_frame_gate = vb.player_frame_gate.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::ItemFrameGate => {
                vb.item_frame_gate = vb.item_frame_gate.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::BlockEntityFrameGate => {
                vb.block_entity_frame_gate = vb.block_entity_frame_gate.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::ParticleFrameGate => {
                vb.particle_frame_gate = vb.particle_frame_gate.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::ParticleBudget => {
                vb.particle_budget = vb.particle_budget.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::BurstLagShield => toggle_field!(self, vb.burst_lag_shield, id, actions),
            BoostOptionId::EntityShadows => {
                toggle_field!(self, vb.entity_shadows, id, actions);
                s.sync_from_volt_boost();
            }
            BoostOptionId::Smart10HzAtlas => toggle_field!(self, vb.smart_10hz_atlas, id, actions),
            BoostOptionId::GlintTrimMode => {
                vb.glint_trim_mode = vb.glint_trim_mode.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::MenuBackdrop => {
                vb.menu_backdrop = vb.menu_backdrop.cycle(dir);
                s.sync_from_volt_boost();
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::DynamicMenuFps => toggle_field!(self, vb.dynamic_menu_fps, id, actions),
            BoostOptionId::AutoDrsScale => toggle_field!(self, vb.auto_drs_scale, id, actions),
            BoostOptionId::DrsTargetFps => {
                const TARGETS: [u32; 5] = [30, 45, 60, 90, 120];
                let idx = TARGETS.iter().position(|&v| v == vb.drs_target_fps).unwrap_or(2);
                vb.drs_target_fps =
                    TARGETS[((idx as i32 + dir).rem_euclid(TARGETS.len() as i32)) as usize];
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::InactivityFps => {
                vb.inactivity_fps = vb.inactivity_fps.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::ThermalGuard => toggle_field!(self, vb.thermal_guard, id, actions),
            BoostOptionId::BigLittleBoost => toggle_field!(self, vb.big_little_boost, id, actions),
            BoostOptionId::BackgroundWorkerCoalescing => {
                toggle_field!(self, vb.background_worker_coalescing, id, actions);
            }
            BoostOptionId::StormWindBoost => toggle_field!(self, vb.storm_wind_boost, id, actions),
            BoostOptionId::ReplayRecording => {
                if !vb.replay_recording && !vb.replay_storage_budget_asked {
                    self.modal = Some(BoostModal::StorageBudgetPrompt);
                    self.focus = Some(FocusTarget::ModalChoice(2));
                    actions.push(VoltBoostAction::ButtonClick);
                } else {
                    let was = vb.replay_recording;
                    vb.replay_recording = !was;
                    self.start_toggle_anim(id, was, vb.replay_recording);
                    actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
                }
            }
            BoostOptionId::OnScreenReplayButton => {
                toggle_field!(self, vb.replay_hud_button, id, actions);
            }
            BoostOptionId::RecordingLength => {
                vb.replay_length = vb.replay_length.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::RecordingQuality => {
                vb.replay_quality = vb.replay_quality.cycle(dir);
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            BoostOptionId::CaptureHud => toggle_field!(self, vb.replay_capture_hud, id, actions),
            BoostOptionId::StorageUsedClear => {
                self.modal = Some(BoostModal::ConfirmClearReplays);
                self.focus = Some(FocusTarget::ModalChoice(1));
                actions.push(VoltBoostAction::ButtonClick);
            }
            BoostOptionId::OpenReplays => {
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::OpenReplays]);
            }
            _ => {
                // Sliders stepped via Left/Right
                let norm = match Self::row_spec(vb, id).kind {
                    RowKind::Slider { norm } => norm,
                    _ => return,
                };
                let next = (norm + dir as f32 * 0.05).clamp(0.0, 1.0);
                Self::set_slider_norm(s, id, next);
                actions.extend([VoltBoostAction::SliderDrag, VoltBoostAction::Changed]);
            }
        }
    }

    fn set_slider_norm(s: &mut Settings, id: BoostOptionId, norm: f32) {
        let t = norm.clamp(0.0, 1.0);
        let vb = &mut s.volt_boost;
        match id {
            BoostOptionId::DistantLodDistance => {
                vb.distant_lod_distance = (32.0 + t * 224.0).round() as u32;
            }
            BoostOptionId::ChunkUploadCap => {
                vb.chunk_upload_cap = (t * 64.0).round() as u32;
            }
            BoostOptionId::ChunkBuildDistance => {
                vb.chunk_build_distance = (2.0 + t * 30.0).round() as u32;
            }
            BoostOptionId::MaxEntities => {
                vb.max_entities = (t * 256.0).round() as u32;
            }
            BoostOptionId::DroppedItemCap => {
                vb.dropped_item_cap = (t * 256.0).round() as u32;
            }
            BoostOptionId::BlockEntityCap => {
                vb.block_entity_cap = (t * 512.0).round() as u32;
            }
            BoostOptionId::MobGateDistance => {
                vb.mob_gate_distance = (8.0 + t * 88.0).round() as u32;
            }
            BoostOptionId::PlayerGateDistance => {
                vb.player_gate_distance = (8.0 + t * 88.0).round() as u32;
            }
            BoostOptionId::ItemGateDistance => {
                vb.item_gate_distance = (8.0 + t * 88.0).round() as u32;
            }
            BoostOptionId::BlockEntityGateDistance => {
                vb.block_entity_gate_distance = (8.0 + t * 88.0).round() as u32;
            }
            BoostOptionId::ParticleGateDistance => {
                vb.particle_gate_distance = (8.0 + t * 88.0).round() as u32;
            }
            BoostOptionId::EntityShadowDistance => {
                vb.entity_shadow_distance = (8.0 + t * 56.0).round() as u32;
            }
            BoostOptionId::DrsMinScale => {
                vb.drs_min_scale_pct = (35.0 + t * 55.0).round() as u32;
            }
            BoostOptionId::DrsMaxScale => {
                vb.drs_max_scale_pct = (60.0 + t * 40.0).round() as u32;
            }
            BoostOptionId::FpsCap => {
                vb.fps_cap = (30.0 + t * 230.0).round() as u32;
            }
            BoostOptionId::InactivityDelay => {
                vb.inactivity_delay_secs = (5.0 + t * 115.0).round() as u32;
            }
            _ => {}
        }
        vb.sanitize();
        s.sync_from_volt_boost();
    }

    pub fn draw(
        &mut self,
        s: &Settings,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        size: (f32, f32),
    ) {
        let (w, h) = size;
        let dp = self.dp_unit(w, h);
        let ts = self.text_scale.clamp(1.0, 1.3);
        let [safe_t, safe_r, safe_b, safe_l] = self.safe_insets;
        let margin = 16.0 * dp;
        let cat_gap = 20.0 * dp;
        let content_x = margin + safe_l;
        let content_w = (w - margin * 2.0 - safe_l - safe_r).max(160.0);

        let open_t = ease_out_cubic(self.opened_at.elapsed().as_secs_f32() * 1000.0 / PAGE_OPEN_MS);
        let page_y_off = (1.0 - open_t) * 14.0 * dp;

        self.hits.clear();
        self.focus_order.clear();
        self.sticky_header = None;

        batch.fill(atlas, 0.0, 0.0, w, h, BG_SCRIM);

        // Top page header bar
        let header_y = margin * 0.65 + safe_t + page_y_off;
        let header_h = 48.0 * dp;
        let back_rect = Rect {
            x: content_x,
            y: header_y,
            w: 76.0 * dp,
            h: header_h,
        };
        self.focus_order.push(FocusTarget::Back);
        self.hits.push(HitRect {
            rect: back_rect,
            kind: HitKind::Back,
        });
        draw_glass_card(batch, atlas, back_rect, 10.0 * dp, CARD_BG, HAIRLINE, self.focus == Some(FocusTarget::Back));
        draw_text_scaled(
            batch,
            atlas,
            "Back",
            back_rect.x + 14.0 * dp,
            back_rect.y + (back_rect.h - 9.0 * ts) * 0.5,
            ts,
            TEXT_PRIMARY,
        );

        let title_x = back_rect.x + back_rect.w + 12.0 * dp;
        draw_text_scaled(
            batch,
            atlas,
            "Volt + Boost",
            title_x,
            header_y + 4.0 * dp,
            ts * 1.12,
            TEXT_PRIMARY,
        );
        draw_text_scaled(
            batch,
            atlas,
            "Deep tuning and replay recording. Everything here is separate from Volt Boost.",
            title_x,
            header_y + 22.0 * dp,
            (ts * 0.84).min(1.12),
            TEXT_DIM,
        );

        let list_top = header_y + header_h + 10.0 * dp;
        let list_bottom = (h - margin * 0.65 - safe_b).max(list_top + 48.0);
        let list_h = list_bottom - list_top;

        let vb = &s.volt_boost;
        let cat_header_h = 52.0 * dp;
        let base_row_h = (48.0 * dp * (0.85 + 0.15 * ts)).max(48.0 * dp);
        let hint_row_h = (56.0 * dp * (0.85 + 0.15 * ts)).max(56.0 * dp);

        // First pass: measure total content height and clamp scroll cleanly.
        let mut total_h = 0.0f32;
        for (idx, cat) in BoostCategory::ALL.iter().enumerate() {
            if idx > 0 {
                total_h += cat_gap;
            }
            let opts = BoostOptionId::options_in(*cat);
            let rows_h: f32 = opts
                .iter()
                .map(|id| {
                    let spec = Self::row_spec(vb, *id);
                    if spec.hint.is_some() { hint_row_h } else { base_row_h }
                })
                .sum();
            let prog = self.expand_progress(idx);
            total_h += cat_header_h + rows_h * prog;
        }
        self.max_scroll = (total_h - list_h).max(0.0);
        self.scroll = self.scroll.clamp(0.0, self.max_scroll);

        // Render scrollable categories clipped to [list_top, list_bottom].
        let clip_start = batch.len();
        let mut cur_y = list_top - self.scroll;
        let mut sticky_candidate: Option<(usize, Rect, f32)> = None;

        for (cat_idx, cat) in BoostCategory::ALL.iter().enumerate() {
            let opts = BoostOptionId::options_in(*cat);
            let prog = self.expand_progress(cat_idx);
            let rows_full_h: f32 = opts
                .iter()
                .map(|id| {
                    let spec = Self::row_spec(vb, *id);
                    if spec.hint.is_some() { hint_row_h } else { base_row_h }
                })
                .sum();
            let visible_rows_h = rows_full_h * prog;
            let card_h = cat_header_h + visible_rows_h;
            let card_rect = Rect {
                x: content_x,
                y: cur_y,
                w: content_w,
                h: card_h,
            };

            if card_rect.y + card_rect.h > list_top && card_rect.y < list_bottom {
                draw_glass_card(
                    batch,
                    atlas,
                    card_rect,
                    CARD_RADIUS * dp,
                    CARD_BG,
                    HAIRLINE,
                    false,
                );
            }

            let hdr_rect = Rect {
                x: content_x,
                y: cur_y,
                w: content_w,
                h: cat_header_h,
            };
            if card_rect.y < list_top && card_rect.y + card_rect.h > list_top + cat_header_h * 0.5 && prog > 0.05 {
                sticky_candidate = Some((cat_idx, hdr_rect, card_rect.y + card_rect.h));
            } else if hdr_rect.y + hdr_rect.h > list_top && hdr_rect.y < list_bottom {
                self.draw_category_header(batch, atlas, vb, cat_idx, *cat, hdr_rect, dp, ts, cursor, false);
            }

            // Options inside expanded/animating category
            if prog > 0.01 {
                let rows_start_y = cur_y + cat_header_h;
                let rows_clip_top = rows_start_y.max(list_top);
                let rows_clip_bot = (rows_start_y + visible_rows_h).min(list_bottom);
                if rows_clip_bot > rows_clip_top {
                    let row_clip_start = batch.len();
                    let mut ry = rows_start_y;
                    let stagger_elapsed = self.master_stagger_at[cat_idx]
                        .map(|t| t.elapsed().as_secs_f32() * 1000.0);
                    for (row_idx, opt_id) in opts.iter().enumerate() {
                        let spec = Self::row_spec(vb, *opt_id);
                        let rh = if spec.hint.is_some() { hint_row_h } else { base_row_h };
                        let row_rect = Rect {
                            x: content_x + 8.0 * dp,
                            y: ry,
                            w: content_w - 16.0 * dp,
                            h: rh - 3.0 * dp,
                        };
                        self.focus_order.push(FocusTarget::Option(*opt_id));
                        if row_rect.y + row_rect.h > rows_clip_top && row_rect.y < rows_clip_bot {
                            let row_alpha = stagger_elapsed
                                .map(|ms| {
                                    let delay = row_idx as f32 * STAGGER_PER_ROW_MS;
                                    0.35 + 0.65 * ease_out_cubic(((ms - delay) / 140.0).clamp(0.0, 1.0))
                                })
                                .unwrap_or(1.0);
                            self.draw_option_row(
                                batch,
                                atlas,
                                &spec,
                                row_rect,
                                dp,
                                ts,
                                cursor,
                                row_alpha,
                            );
                        }
                        ry += rh;
                    }
                    batch.clip_since(
                        row_clip_start,
                        content_x,
                        rows_clip_top,
                        content_x + content_w,
                        rows_clip_bot,
                    );
                }
            }

            cur_y += card_h + cat_gap;
        }
        batch.clip_since(clip_start, content_x - 2.0, list_top, content_x + content_w + 2.0, list_bottom);

        // Sticky category header pinned at top of list viewport while options pass beneath it.
        if let Some((cat_idx, _orig, card_bot)) = sticky_candidate {
            let sticky_y = list_top.min(card_bot - cat_header_h);
            let sticky_rect = Rect {
                x: content_x,
                y: sticky_y,
                w: content_w,
                h: cat_header_h,
            };
            self.sticky_header = Some(StickyHeaderInfo {
                category_idx: cat_idx,
                rect: sticky_rect,
            });
            self.draw_category_header(
                batch,
                atlas,
                vb,
                cat_idx,
                BoostCategory::ALL[cat_idx],
                sticky_rect,
                dp,
                ts,
                cursor,
                true,
            );
        }

        // Modal overlays (Storage Budget first-time prompt, Clear Replays confirm).
        if let Some(modal) = self.modal {
            self.draw_modal(batch, atlas, modal, size, dp, ts, cursor);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_category_header(
        &mut self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        vb: &VoltPlusBoostSettings,
        cat_idx: usize,
        cat: BoostCategory,
        rect: Rect,
        dp: f32,
        ts: f32,
        cursor: (f32, f32),
        sticky: bool,
    ) {
        let master_w = 54.0 * dp;
        let master_rect = Rect {
            x: rect.x + rect.w - master_w - 14.0 * dp,
            y: rect.y + (rect.h - 48.0 * dp).max(0.0) * 0.5,
            w: master_w,
            h: 48.0 * dp,
        };
        let hdr_hit_rect = Rect {
            x: rect.x,
            y: rect.y,
            w: (master_rect.x - rect.x).max(40.0),
            h: rect.h,
        };
        self.focus_order.push(FocusTarget::CategoryHeader(cat_idx));
        self.focus_order.push(FocusTarget::CategoryMaster(cat_idx));
        self.hits.push(HitRect {
            rect: hdr_hit_rect,
            kind: HitKind::CategoryHeader(cat_idx),
        });
        self.hits.push(HitRect {
            rect: master_rect,
            kind: HitKind::CategoryMaster(cat_idx),
        });

        let bg = if sticky { CARD_HEADER_BG } else { rgba(20, 24, 33, 232) };
        let focused_hdr = self.focus == Some(FocusTarget::CategoryHeader(cat_idx));
        draw_glass_card(batch, atlas, rect, CARD_RADIUS * dp, bg, HAIRLINE_STRONG, focused_hdr);

        let arrow = if self.expanded[cat_idx] { "v" } else { ">" };
        draw_text_scaled(
            batch,
            atlas,
            arrow,
            rect.x + 14.0 * dp,
            rect.y + (rect.h - 10.0 * ts) * 0.5,
            ts,
            TEXT_MUTED,
        );
        draw_text_scaled(
            batch,
            atlas,
            cat.title(),
            rect.x + 30.0 * dp,
            rect.y + (rect.h - 10.0 * ts) * 0.5,
            ts * 1.02,
            TEXT_PRIMARY,
        );

        let (on_cnt, total_cnt) = vb.category_counts(cat);
        let count_label = format!("{on_cnt} of {total_cnt} on");
        let count_w = HudBatch::text_width(atlas, &count_label) * ts * 0.9;
        draw_text_scaled(
            batch,
            atlas,
            &count_label,
            master_rect.x - count_w - 12.0 * dp,
            rect.y + (rect.h - 9.0 * ts) * 0.5,
            ts * 0.9,
            if on_cnt > 0 { TEXT_MUTED } else { TEXT_DIM },
        );

        let master_on = vb.category_master_on(cat);
        let master_hovered = ui::hovered(cursor, master_rect.x, master_rect.y, master_rect.w, master_rect.h);
        let pill_rect = Rect {
            x: master_rect.x + (master_rect.w - 42.0 * dp) * 0.5,
            y: master_rect.y + (master_rect.h - 22.0 * dp) * 0.5,
            w: 42.0 * dp,
            h: 22.0 * dp,
        };
        let focused_master = self.focus == Some(FocusTarget::CategoryMaster(cat_idx));
        if focused_master {
            draw_focus_ring(batch, atlas, pill_rect, 11.0 * dp);
        }
        let press_s = if self.pressed == Some(FocusTarget::CategoryMaster(cat_idx)) {
            PRESS_SCALE
        } else {
            1.0
        };
        let pill_start = batch.len();
        draw_toggle_pill(
            batch,
            atlas,
            pill_rect,
            if master_on { 1.0 } else { 0.0 },
            master_on,
            master_hovered,
            dp,
        );
        if press_s < 0.999 {
            batch.scale_since(
                pill_start,
                pill_rect.x + pill_rect.w * 0.5,
                pill_rect.y + pill_rect.h * 0.5,
                press_s,
                press_s,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_option_row(
        &mut self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        spec: &RowSpec,
        rect: Rect,
        dp: f32,
        ts: f32,
        cursor: (f32, f32),
        alpha_mul: f32,
    ) {
        let hovered = ui::hovered(cursor, rect.x, rect.y, rect.w, rect.h);
        let focused = self.focus == Some(FocusTarget::Option(spec.id));
        let bg = if hovered { ROW_HOVER_BG } else { ROW_BG };
        let row_start = batch.len();
        draw_glass_card(batch, atlas, rect, 10.0 * dp, apply_alpha(bg, alpha_mul), HAIRLINE, focused);

        let control_w = (rect.w * 0.32).clamp(112.0 * dp, 178.0 * dp);
        let control_x = rect.x + rect.w - control_w - 10.0 * dp;
        let control_rect = Rect {
            x: control_x,
            y: rect.y + (rect.h - 28.0 * dp) * 0.5,
            w: control_w,
            h: 28.0 * dp,
        };

        let mut hit_rect = rect;
        if let RowKind::Slider { .. } = spec.kind {
            self.hits.push(HitRect {
                rect: control_rect,
                kind: HitKind::SliderTrack(spec.id, control_rect),
            });
            hit_rect = Rect {
                x: rect.x,
                y: rect.y,
                w: (control_x - rect.x).max(40.0),
                h: rect.h,
            };
        }
        self.hits.push(HitRect {
            rect: hit_rect,
            kind: HitKind::OptionRow(spec.id, control_rect),
        });

        let label_col = if spec.enabled { TEXT_PRIMARY } else { TEXT_DIM };
        let state_col = if !spec.enabled {
            TEXT_DIM
        } else if matches!(spec.kind, RowKind::Toggle(true)) {
            TEXT_PRIMARY
        } else {
            TEXT_MUTED
        };

        let text_x = rect.x + 12.0 * dp;
        let label_y = if spec.hint.is_some() {
            rect.y + 8.0 * dp
        } else {
            rect.y + (rect.h - 9.0 * ts) * 0.5
        };
        draw_text_scaled(batch, atlas, spec.label, text_x, label_y, ts, apply_alpha(label_col, alpha_mul));
        if let Some(hint) = &spec.hint {
            draw_text_scaled(
                batch,
                atlas,
                hint,
                text_x,
                rect.y + rect.h - 16.0 * dp,
                (ts * 0.80).min(1.08),
                apply_alpha(TEXT_DIM, alpha_mul),
            );
        }

        // State as a word right before the interactive control.
        let word_w = HudBatch::text_width(atlas, &spec.state_word) * ts * 0.88;
        let word_x = (control_rect.x - word_w - 10.0 * dp).max(text_x + 90.0 * dp);
        draw_text_scaled(
            batch,
            atlas,
            &spec.state_word,
            word_x,
            rect.y + (rect.h - 9.0 * ts) * 0.5,
            ts * 0.88,
            apply_alpha(state_col, alpha_mul),
        );

        match &spec.kind {
            RowKind::Toggle(on) => {
                let pill_w = 42.0 * dp;
                let pill_h = 22.0 * dp;
                let pill = Rect {
                    x: control_rect.x + control_rect.w - pill_w,
                    y: rect.y + (rect.h - pill_h) * 0.5,
                    w: pill_w,
                    h: pill_h,
                };
                let t = self.toggle_knob_t(spec.id, *on);
                draw_toggle_pill(batch, atlas, pill, t, *on && spec.enabled, hovered, dp);
            }
            RowKind::Cycle => {
                let pill = Rect {
                    x: control_rect.x + (control_rect.w - 92.0 * dp).max(0.0),
                    y: control_rect.y,
                    w: control_rect.w.min(92.0 * dp),
                    h: control_rect.h,
                };
                draw_glass_card(
                    batch,
                    atlas,
                    pill,
                    9.0 * dp,
                    if spec.enabled { CARD_HEADER_BG } else { TRACK_OFF },
                    HAIRLINE_STRONG,
                    false,
                );
                let txt = "Cycle";
                let tw = HudBatch::text_width(atlas, txt) * ts * 0.88;
                draw_text_scaled(
                    batch,
                    atlas,
                    txt,
                    pill.x + (pill.w - tw) * 0.5,
                    pill.y + (pill.h - 8.5 * ts) * 0.5,
                    ts * 0.88,
                    label_col,
                );
            }
            RowKind::Slider { norm } => {
                let t = norm.clamp(0.0, 1.0);
                let track_h = 4.0 * dp;
                let track_y = control_rect.y + (control_rect.h - track_h) * 0.5;
                batch.fill(atlas, control_rect.x, track_y, control_rect.w, track_h, TRACK_OFF);
                if t > 0.01 {
                    batch.fill(
                        atlas,
                        control_rect.x,
                        track_y,
                        control_rect.w * t,
                        track_h,
                        if spec.enabled { ACCENT } else { TEXT_DIM },
                    );
                }
                let knob_size = 16.0 * dp;
                let knob = Rect {
                    x: control_rect.x + t * (control_rect.w - knob_size),
                    y: control_rect.y + (control_rect.h - knob_size) * 0.5,
                    w: knob_size,
                    h: knob_size,
                };
                draw_rounded_rect(batch, atlas, knob, KNOB_RADIUS.min(knob_size * 0.5), KNOB_COLOR);
            }
            RowKind::Action(btn_label) => {
                let btn = Rect {
                    x: control_rect.x + (control_rect.w - 96.0 * dp).max(0.0),
                    y: control_rect.y,
                    w: control_rect.w.min(96.0 * dp),
                    h: control_rect.h,
                };
                draw_glass_card(
                    batch,
                    atlas,
                    btn,
                    9.0 * dp,
                    ACCENT_SOFT,
                    ACCENT,
                    false,
                );
                let tw = HudBatch::text_width(atlas, btn_label) * ts * 0.86;
                draw_text_scaled(
                    batch,
                    atlas,
                    btn_label,
                    btn.x + (btn.w - tw) * 0.5,
                    btn.y + (btn.h - 8.5 * ts) * 0.5,
                    ts * 0.86,
                    TEXT_PRIMARY,
                );
            }
        }

        if self.pressed == Some(FocusTarget::Option(spec.id)) {
            batch.scale_since(
                row_start,
                rect.x + rect.w * 0.5,
                rect.y + rect.h * 0.5,
                PRESS_SCALE,
                PRESS_SCALE,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_modal(
        &mut self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        modal: BoostModal,
        size: (f32, f32),
        dp: f32,
        ts: f32,
        cursor: (f32, f32),
    ) {
        let (w, h) = size;
        batch.fill(atlas, 0.0, 0.0, w, h, rgba(6, 8, 12, 185));
        self.hits.push(HitRect {
            rect: Rect { x: 0.0, y: 0.0, w, h },
            kind: HitKind::ModalBackdrop,
        });

        let box_w = (w - 32.0 * dp).min(420.0 * dp);
        let box_h = match modal {
            BoostModal::StorageBudgetPrompt => 190.0 * dp,
            BoostModal::ConfirmClearReplays => 150.0 * dp,
        };
        let box_rect = Rect {
            x: (w - box_w) * 0.5,
            y: (h - box_h) * 0.5,
            w: box_w,
            h: box_h,
        };
        draw_glass_card(batch, atlas, box_rect, CARD_RADIUS * dp, CARD_BG, HAIRLINE_STRONG, false);

        self.focus_order.clear();
        match modal {
            BoostModal::StorageBudgetPrompt => {
                draw_text_scaled(
                    batch,
                    atlas,
                    "Set Replay Storage Budget",
                    box_rect.x + 16.0 * dp,
                    box_rect.y + 16.0 * dp,
                    ts * 1.05,
                    TEXT_PRIMARY,
                );
                draw_text_scaled(
                    batch,
                    atlas,
                    "Oldest clips are trimmed automatically when the budget is reached.",
                    box_rect.x + 16.0 * dp,
                    box_rect.y + 38.0 * dp,
                    ts * 0.84,
                    TEXT_MUTED,
                );
                let labels = ["256 MB", "512 MB", "1 GB", "2 GB", "4 GB"];
                let btn_w = (box_rect.w - 32.0 * dp - 24.0 * dp) / 5.0;
                for (i, label) in labels.iter().enumerate() {
                    let r = Rect {
                        x: box_rect.x + 16.0 * dp + i as f32 * (btn_w + 6.0 * dp),
                        y: box_rect.y + 72.0 * dp,
                        w: btn_w,
                        h: 48.0 * dp,
                    };
                    let focused = self.focus == Some(FocusTarget::ModalChoice(i));
                    let hovered = ui::hovered(cursor, r.x, r.y, r.w, r.h);
                    draw_glass_card(
                        batch,
                        atlas,
                        r,
                        10.0 * dp,
                        if i == 2 || hovered { ACCENT_SOFT } else { ROW_BG },
                        if i == 2 { ACCENT } else { HAIRLINE_STRONG },
                        focused,
                    );
                    let tw = HudBatch::text_width(atlas, label) * ts * 0.85;
                    draw_text_scaled(
                        batch,
                        atlas,
                        label,
                        r.x + (r.w - tw) * 0.5,
                        r.y + (r.h - 9.0 * ts) * 0.5,
                        ts * 0.85,
                        TEXT_PRIMARY,
                    );
                    self.focus_order.push(FocusTarget::ModalChoice(i));
                    self.hits.push(HitRect {
                        rect: r,
                        kind: HitKind::ModalChoice(i),
                    });
                }
                let cancel = Rect {
                    x: box_rect.x + 16.0 * dp,
                    y: box_rect.y + box_rect.h - 56.0 * dp,
                    w: box_rect.w - 32.0 * dp,
                    h: 44.0 * dp,
                };
                draw_glass_card(
                    batch,
                    atlas,
                    cancel,
                    10.0 * dp,
                    ROW_BG,
                    HAIRLINE,
                    self.focus == Some(FocusTarget::ModalChoice(5)),
                );
                let cw = HudBatch::text_width(atlas, "Cancel") * ts * 0.9;
                draw_text_scaled(
                    batch,
                    atlas,
                    "Cancel",
                    cancel.x + (cancel.w - cw) * 0.5,
                    cancel.y + (cancel.h - 9.0 * ts) * 0.5,
                    ts * 0.9,
                    TEXT_MUTED,
                );
                self.focus_order.push(FocusTarget::ModalChoice(5));
                self.hits.push(HitRect {
                    rect: cancel,
                    kind: HitKind::ModalChoice(5),
                });
            }
            BoostModal::ConfirmClearReplays => {
                draw_text_scaled(
                    batch,
                    atlas,
                    "Clear All Replays?",
                    box_rect.x + 16.0 * dp,
                    box_rect.y + 16.0 * dp,
                    ts * 1.05,
                    TEXT_PRIMARY,
                );
                draw_text_scaled(
                    batch,
                    atlas,
                    "All saved replay clips in this profile will be permanently deleted.",
                    box_rect.x + 16.0 * dp,
                    box_rect.y + 40.0 * dp,
                    ts * 0.85,
                    TEXT_MUTED,
                );
                let btn_w = (box_rect.w - 40.0 * dp) * 0.5;
                let confirm = Rect {
                    x: box_rect.x + 16.0 * dp,
                    y: box_rect.y + box_rect.h - 60.0 * dp,
                    w: btn_w,
                    h: 48.0 * dp,
                };
                let cancel = Rect {
                    x: confirm.x + btn_w + 8.0 * dp,
                    y: confirm.y,
                    w: btn_w,
                    h: 48.0 * dp,
                };
                draw_glass_card(
                    batch,
                    atlas,
                    confirm,
                    10.0 * dp,
                    ACCENT_SOFT,
                    ACCENT,
                    self.focus == Some(FocusTarget::ModalChoice(0)),
                );
                let cw = HudBatch::text_width(atlas, "Clear Replays") * ts * 0.9;
                draw_text_scaled(
                    batch,
                    atlas,
                    "Clear Replays",
                    confirm.x + (confirm.w - cw) * 0.5,
                    confirm.y + (confirm.h - 9.0 * ts) * 0.5,
                    ts * 0.9,
                    TEXT_PRIMARY,
                );
                draw_glass_card(
                    batch,
                    atlas,
                    cancel,
                    10.0 * dp,
                    ROW_BG,
                    HAIRLINE_STRONG,
                    self.focus == Some(FocusTarget::ModalChoice(1)),
                );
                let xw = HudBatch::text_width(atlas, "Cancel") * ts * 0.9;
                draw_text_scaled(
                    batch,
                    atlas,
                    "Cancel",
                    cancel.x + (cancel.w - xw) * 0.5,
                    cancel.y + (cancel.h - 9.0 * ts) * 0.5,
                    ts * 0.9,
                    TEXT_PRIMARY,
                );
                self.focus_order.push(FocusTarget::ModalChoice(0));
                self.focus_order.push(FocusTarget::ModalChoice(1));
                self.hits.push(HitRect {
                    rect: confirm,
                    kind: HitKind::ModalChoice(0),
                });
                self.hits.push(HitRect {
                    rect: cancel,
                    kind: HitKind::ModalChoice(1),
                });
            }
        }
    }

    pub fn click(
        &mut self,
        s: &mut Settings,
        pos: (f32, f32),
        size: (f32, f32),
        drag: bool,
        right: bool,
    ) -> Vec<VoltBoostAction> {
        let mut actions = Vec::new();
        if drag {
            if let Some(id) = self.dragging_slider {
                if let Some(track) = self.hits.iter().find_map(|h| match h.kind {
                    HitKind::SliderTrack(k, r) if k == id => Some(r),
                    _ => None,
                }) {
                    let norm = ui::slider_value(track.x, track.w, pos.0);
                    Self::set_slider_norm(s, id, norm);
                    actions.push(VoltBoostAction::Changed);
                }
                return actions;
            }
            if let Some(prev_y) = self.dragging_scroll_y.replace(pos.1) {
                self.scroll = (self.scroll - (pos.1 - prev_y)).clamp(0.0, self.max_scroll);
            }
            return actions;
        }

        // Check in reverse order so sticky header & modals win hit tests cleanly.
        let hits = self.hits.clone();
        for hit in hits.into_iter().rev() {
            if !ui::hovered(pos, hit.rect.x, hit.rect.y, hit.rect.w, hit.rect.h) {
                continue;
            }
            match hit.kind {
                HitKind::ModalBackdrop => return actions,
                HitKind::ModalChoice(choice) => {
                    self.activate_modal_choice(s, choice, &mut actions);
                    return actions;
                }
                HitKind::Back => {
                    self.pressed = Some(FocusTarget::Back);
                    actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Done]);
                    return actions;
                }
                HitKind::CategoryHeader(idx) => {
                    self.pressed = Some(FocusTarget::CategoryHeader(idx));
                    self.toggle_category(idx);
                    actions.push(VoltBoostAction::ButtonClick);
                    return actions;
                }
                HitKind::CategoryMaster(idx) => {
                    self.pressed = Some(FocusTarget::CategoryMaster(idx));
                    let cat = BoostCategory::ALL[idx];
                    let next = !s.volt_boost.category_master_on(cat);
                    s.volt_boost.set_category_master(cat, next);
                    s.sync_from_volt_boost();
                    self.master_stagger_at[idx] = Some(Instant::now());
                    actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
                    return actions;
                }
                HitKind::SliderTrack(id, track) => {
                    self.pressed = Some(FocusTarget::Option(id));
                    self.dragging_slider = Some(id);
                    let norm = ui::slider_value(track.x, track.w, pos.0);
                    Self::set_slider_norm(s, id, norm);
                    actions.push(VoltBoostAction::Changed);
                    return actions;
                }
                HitKind::OptionRow(id, _) => {
                    self.pressed = Some(FocusTarget::Option(id));
                    self.focus = Some(FocusTarget::Option(id));
                    self.apply_option_activate(s, id, if right { -1 } else { 1 }, &mut actions);
                    return actions;
                }
            }
        }
        let _ = size;
        self.dragging_scroll_y = Some(pos.1);
        actions
    }

    fn activate_modal_choice(
        &mut self,
        s: &mut Settings,
        choice: usize,
        actions: &mut Vec<VoltBoostAction>,
    ) {
        match self.modal.take() {
            Some(BoostModal::StorageBudgetPrompt) => {
                const BUDGETS: [u32; 5] = [256, 512, 1024, 2048, 4096];
                if let Some(&mb) = BUDGETS.get(choice) {
                    s.volt_boost.replay_storage_budget_asked = true;
                    s.volt_boost.replay_storage_budget_mb = mb;
                    s.volt_boost.replay_recording = true;
                    self.start_toggle_anim(BoostOptionId::ReplayRecording, false, true);
                    actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
                } else {
                    actions.push(VoltBoostAction::ButtonClick);
                }
            }
            Some(BoostModal::ConfirmClearReplays) => {
                if choice == 0 {
                    actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::ClearReplays]);
                } else {
                    actions.push(VoltBoostAction::ButtonClick);
                }
            }
            None => {}
        }
        self.focus = None;
    }

    pub fn release(&mut self) -> bool {
        self.pressed = None;
        self.dragging_scroll_y = None;
        self.dragging_slider.take().is_some()
    }

    pub fn scroll(&mut self, amount: f32) {
        self.scroll = (self.scroll - amount * 28.0).clamp(0.0, self.max_scroll);
    }

    pub fn key(&mut self, s: &mut Settings, key: &Key) -> Vec<VoltBoostAction> {
        let mut actions = Vec::new();
        match key {
            Key::Named(NamedKey::Escape) | Key::Named(NamedKey::BrowserBack) => {
                if self.modal.take().is_some() {
                    self.focus = None;
                    actions.push(VoltBoostAction::ButtonClick);
                } else {
                    actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Done]);
                }
            }
            Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::Tab) => {
                self.move_focus(1);
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.move_focus(-1);
            }
            Key::Named(NamedKey::ArrowLeft) => {
                self.nudge_focused(s, -1, &mut actions);
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.nudge_focused(s, 1, &mut actions);
            }
            Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => {
                self.activate_focused(s, &mut actions);
            }
            _ => {}
        }
        actions
    }

    pub fn move_focus(&mut self, dir: i32) {
        if self.focus_order.is_empty() {
            return;
        }
        let next_idx = match self.focus.and_then(|f| self.focus_order.iter().position(|x| *x == f)) {
            Some(i) => (i as i32 + dir).rem_euclid(self.focus_order.len() as i32) as usize,
            None => {
                if dir >= 0 {
                    0
                } else {
                    self.focus_order.len() - 1
                }
            }
        };
        self.focus = Some(self.focus_order[next_idx]);
    }

    fn nudge_focused(&mut self, s: &mut Settings, dir: i32, actions: &mut Vec<VoltBoostAction>) {
        match self.focus {
            Some(FocusTarget::Option(id)) => {
                self.apply_option_activate(s, id, dir, actions);
            }
            Some(FocusTarget::ModalChoice(_)) => {
                self.move_focus(dir);
            }
            _ => {}
        }
    }

    fn activate_focused(&mut self, s: &mut Settings, actions: &mut Vec<VoltBoostAction>) {
        match self.focus {
            Some(FocusTarget::Back) => {
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Done]);
            }
            Some(FocusTarget::CategoryHeader(idx)) => {
                self.toggle_category(idx);
                actions.push(VoltBoostAction::ButtonClick);
            }
            Some(FocusTarget::CategoryMaster(idx)) => {
                let cat = BoostCategory::ALL[idx];
                let next = !s.volt_boost.category_master_on(cat);
                s.volt_boost.set_category_master(cat, next);
                s.sync_from_volt_boost();
                self.master_stagger_at[idx] = Some(Instant::now());
                actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
            }
            Some(FocusTarget::Option(id)) => {
                self.apply_option_activate(s, id, 1, actions);
            }
            Some(FocusTarget::ModalChoice(c)) => {
                self.activate_modal_choice(s, c, actions);
            }
            None => {}
        }
    }
}

macro_rules! toggle_field {
    ($self:ident, $field:expr, $id:ident, $actions:ident) => {{
        let was = $field;
        $field = !was;
        $self.start_toggle_anim($id, was, $field);
        $actions.extend([VoltBoostAction::ButtonClick, VoltBoostAction::Changed]);
    }};
}
use toggle_field;

fn on_off(v: bool) -> String {
    if v { "On".into() } else { "Off".into() }
}

fn cap_word(v: u32) -> String {
    if v == 0 {
        "Unlimited".into()
    } else {
        format!("{v}")
    }
}

fn apply_alpha(color: u32, mul: f32) -> u32 {
    let a = (((color >> 24) & 0xFF) as f32 * mul.clamp(0.0, 1.0)).round() as u32;
    (color & 0x00FF_FFFF) | (a << 24)
}

fn draw_text_scaled(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: u32,
) {
    let start = batch.len();
    batch.text(atlas, &Text::literal(text), x, y, color);
    if (scale - 1.0).abs() > 0.01 {
        batch.scale_since(start, x, y, scale, scale);
    }
}

fn draw_rounded_rect(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, radius: f32, color: u32) {
    let rad = radius.min(r.w * 0.5).min(r.h * 0.5).max(0.0);
    if rad <= 0.5 {
        batch.fill(atlas, r.x, r.y, r.w, r.h, color);
        return;
    }
    // Center band + top/bottom inner bands
    batch.fill(atlas, r.x + rad, r.y, r.w - 2.0 * rad, r.h, color);
    batch.fill(atlas, r.x, r.y + rad, rad, r.h - 2.0 * rad, color);
    batch.fill(atlas, r.x + r.w - rad, r.y + rad, rad, r.h - 2.0 * rad, color);

    // 4 smooth 4-segment quarter-circle corners
    let corners = [
        (r.x + rad, r.y + rad, std::f32::consts::PI, std::f32::consts::FRAC_PI_2 * 3.0),
        (r.x + r.w - rad, r.y + rad, std::f32::consts::FRAC_PI_2 * 3.0, std::f32::consts::TAU),
        (r.x + r.w - rad, r.y + r.h - rad, 0.0, std::f32::consts::FRAC_PI_2),
        (r.x + rad, r.y + r.h - rad, std::f32::consts::FRAC_PI_2, std::f32::consts::PI),
    ];
    for (cx, cy, a0, a1) in corners {
        for seg in 0..4 {
            let t0 = a0 + (a1 - a0) * (seg as f32 / 4.0);
            let t1 = a0 + (a1 - a0) * ((seg + 1) as f32 / 4.0);
            let p0 = [cx + rad * t0.cos(), cy + rad * t0.sin()];
            let p1 = [cx + rad * t1.cos(), cy + rad * t1.sin()];
            batch.fill_quad(atlas, [[cx, cy], p0, p1, [cx, cy]], color);
        }
    }
}

fn draw_glass_card(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    r: Rect,
    radius: f32,
    fill: u32,
    hairline: u32,
    focused: bool,
) {
    draw_rounded_rect(batch, atlas, r, radius, fill);
    // 1 px translucent hairline border
    batch.fill(atlas, r.x + radius * 0.4, r.y, (r.w - radius * 0.8).max(1.0), 1.0, hairline);
    batch.fill(atlas, r.x + radius * 0.4, r.y + r.h - 1.0, (r.w - radius * 0.8).max(1.0), 1.0, hairline);
    batch.fill(atlas, r.x, r.y + radius * 0.4, 1.0, (r.h - radius * 0.8).max(1.0), hairline);
    batch.fill(atlas, r.x + r.w - 1.0, r.y + radius * 0.4, 1.0, (r.h - radius * 0.8).max(1.0), hairline);
    if focused {
        draw_focus_ring(batch, atlas, r, radius);
    }
}

fn draw_focus_ring(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, _radius: f32) {
    let t = 2.0;
    batch.fill(atlas, r.x - t, r.y - t, r.w + t * 2.0, t, ACCENT);
    batch.fill(atlas, r.x - t, r.y + r.h, r.w + t * 2.0, t, ACCENT);
    batch.fill(atlas, r.x - t, r.y, t, r.h, ACCENT);
    batch.fill(atlas, r.x + r.w, r.y, t, r.h, ACCENT);
}

fn draw_toggle_pill(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    pill: Rect,
    t: f32,
    active: bool,
    hovered: bool,
    dp: f32,
) {
    let track_col = if active {
        ACCENT
    } else if hovered {
        rgba(64, 72, 90, 255)
    } else {
        TRACK_OFF
    };
    draw_rounded_rect(batch, atlas, pill, pill.h * 0.5, track_col);
    let knob_d = (pill.h - 4.0 * dp).max(8.0);
    let travel = (pill.w - knob_d - 4.0 * dp).max(0.0);
    let knob = Rect {
        x: pill.x + 2.0 * dp + travel * t.clamp(0.0, 1.0),
        y: pill.y + (pill.h - knob_d) * 0.5,
        w: knob_d,
        h: knob_d,
    };
    draw_rounded_rect(batch, atlas, knob, KNOB_RADIUS.min(knob_d * 0.5), KNOB_COLOR);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_categories_expand_collapse_and_sticky_header_activate() {
        let mut ui = VoltBoostUi::default();
        let mut s = Settings::default();
        let atlas = HudAtlas::boot();
        let mut batch = HudBatch::new([800, 600]);

        ui.draw(&s, &mut batch, &atlas, (0.0, 0.0), (800.0, 600.0));
        assert!(ui.hits.iter().any(|h| matches!(h.kind, HitKind::CategoryHeader(0))));
        assert!(ui.hits.iter().any(|h| matches!(h.kind, HitKind::CategoryMaster(8))));

        // Scroll mid-way through expanded Category 0 -> sticky header activates
        ui.scroll = 140.0;
        let mut batch2 = HudBatch::new([800, 600]);
        ui.draw(&s, &mut batch2, &atlas, (0.0, 0.0), (800.0, 600.0));
        assert!(ui.sticky_header.is_some());

        // Toggle master switch for Category 1 via keyboard
        ui.focus = Some(FocusTarget::CategoryMaster(1));
        let acts = ui.key(&mut s, &Key::Named(NamedKey::Enter));
        assert!(acts.contains(&VoltBoostAction::Changed));
        assert!(s.volt_boost.category_master_on(BoostCategory::CullingAndVisibility));
    }

    #[test]
    fn enabling_replay_first_time_prompts_storage_budget_and_clear_confirms() {
        let mut ui = VoltBoostUi::default();
        let mut s = Settings::default();
        let atlas = HudAtlas::boot();
        let mut batch = HudBatch::new([800, 600]);
        ui.draw(&s, &mut batch, &atlas, (0.0, 0.0), (800.0, 600.0));

        // First-time Replay Recording toggle opens StorageBudgetPrompt
        let mut actions = Vec::new();
        ui.apply_option_activate(&mut s, BoostOptionId::ReplayRecording, 1, &mut actions);
        assert_eq!(ui.modal, Some(BoostModal::StorageBudgetPrompt));
        assert!(!s.volt_boost.replay_recording);

        // Pick 2 GB (choice index 3)
        ui.activate_modal_choice(&mut s, 3, &mut actions);
        assert_eq!(s.volt_boost.replay_storage_budget_mb, 2048);
        assert!(s.volt_boost.replay_storage_budget_asked);
        assert!(s.volt_boost.replay_recording);

        // Clear Replays requires confirm modal
        actions.clear();
        ui.apply_option_activate(&mut s, BoostOptionId::StorageUsedClear, 1, &mut actions);
        assert_eq!(ui.modal, Some(BoostModal::ConfirmClearReplays));
        ui.activate_modal_choice(&mut s, 0, &mut actions);
        assert!(actions.contains(&VoltBoostAction::ClearReplays));
    }

    #[test]
    fn text_scale_130_percent_and_all_rows_stay_at_least_48dp() {
        let mut ui = VoltBoostUi::default();
        ui.text_scale = 1.3;
        let s = Settings::default();
        let atlas = HudAtlas::boot();
        let mut batch = HudBatch::new([800, 600]);
        ui.draw(&s, &mut batch, &atlas, (0.0, 0.0), (800.0, 600.0));

        for hit in &ui.hits {
            if let HitKind::OptionRow(_, _) = hit.kind {
                assert!(hit.rect.h >= 45.0, "row hit height too small: {}", hit.rect.h);
            }
        }
    }
}
