//! Volt Graphics video settings UI (`Page::Video`): sidebar tabs, live search,
//! grouped option rows with smooth sliders and cycle selectors, a description
//! footer with performance cost badges, and frame-independent animations.
use std::time::Instant;

use crate::{
    graphics_style::Cost,
    graphics_ui::{self, H, Item},
    hud::{HudAtlas, HudBatch, rgba},
    lang,
    options_ui::{Opt, apply_preset},
    settings::Settings,
    text::Text,
    ui,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VoltTab {
    #[default]
    General,
    Quality,
    Performance,
    Advanced,
    Hdr,
}

impl VoltTab {
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Quality,
        Self::Performance,
        Self::Advanced,
        Self::Hdr,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Quality => "Quality",
            Self::Performance => "Performance",
            Self::Advanced => "Advanced",
            Self::Hdr => "HDR",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Quality => 1,
            Self::Performance => 2,
            Self::Advanced => 3,
            Self::Hdr => 4,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct VideoEntry {
    pub item: Item,
    pub cost: Cost,
    pub text: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum VideoRow {
    Header(&'static str),
    Entry(VideoEntry),
}

fn opt(o: Opt, cost: Cost, text: &'static str) -> VideoRow {
    VideoRow::Entry(VideoEntry {
        item: Item::Vanilla(o),
        cost,
        text,
    })
}

fn hopt(h: H, cost: Cost, text: &'static str) -> VideoRow {
    VideoRow::Entry(VideoEntry {
        item: Item::Horizon(h),
        cost,
        text,
    })
}

pub(crate) fn tab_rows(tab: VoltTab) -> Vec<VideoRow> {
    use Cost::*;
    let far_desc = if crate::graphics::reduced_far_detail_suggested() {
        "Reduced draws far terrain coarser: faster on this integrated GPU."
    } else {
        "Reduced draws far terrain coarser, for GPUs it slows down."
    };
    let mut rows = match tab {
        VoltTab::General => vec![
            VideoRow::Header("Preset & Scale"),
            hopt(
                H::Preset,
                None,
                "Performance, Balanced, Quality or Ultra: tunes distances, clouds and effects together.",
            ),
            opt(
                Opt::RenderDistance,
                High,
                "Full-detail terrain range. LOD Distance can extend local worlds farther.",
            ),
            opt(
                Opt::SimulationDistance,
                Medium,
                "How far mobs, crops and redstone tick.",
            ),
            hopt(
                H::ResolutionScale,
                High,
                "Internal render resolution scale. Lower gives more FPS; higher sharpens edges.",
            ),
            opt(
                Opt::GuiScale,
                None,
                "Size of menus, inventory and the hotbar.",
            ),
            VideoRow::Header("Display & Frame Rate"),
            opt(
                Opt::Fov,
                Low,
                "Field of view. Wider shows more terrain at once.",
            ),
            opt(
                Opt::MaxFps,
                None,
                "Upper limit on frames per second. Lower saves power.",
            ),
            opt(
                Opt::Vsync,
                None,
                "Matches frames to your monitor refresh to stop tearing.",
            ),
            hopt(
                H::PresentMode,
                None,
                "How frames queue to the display. Auto follows VSync.",
            ),
            hopt(
                H::Pacing,
                None,
                "Efficient saves power between frames; Precise lowers input lag.",
            ),
            opt(
                Opt::Gamma,
                None,
                "Adjusts overall display brightness and gamma curve.",
            ),
            opt(
                Opt::InactivityFps,
                Saves,
                "Lowers frame rate while AFK or minimized to save power.",
            ),
        ],
        VoltTab::Quality => vec![
            VideoRow::Header("Terrain & Textures"),
            opt(
                Opt::SmoothLighting,
                Low,
                "Softens light across block faces.",
            ),
            opt(
                Opt::BiomeBlend,
                Low,
                "Smooths grass and water colour across biome edges.",
            ),
            opt(
                Opt::Mipmaps,
                Low,
                "Reduces texture shimmer on distant blocks.",
            ),
            opt(
                Opt::TextureFiltering,
                Low,
                "Selects nearest, bilinear, or anisotropic texture sampling.",
            ),
            opt(
                Opt::MaxAnisotropy,
                Low,
                "Sharpens textures viewed at shallow angles when anisotropic filtering is on.",
            ),
            hopt(
                H::DitherCutoutMips,
                None,
                "Fills mipmap holes in distant leaves and grass so they stay full.",
            ),
            opt(
                Opt::CutoutLeaves,
                Medium,
                "Renders tree leaves with alpha cutouts instead of opaque blocks.",
            ),
            opt(
                Opt::ImprovedTransparency,
                Medium,
                "Blends translucent water, ice and stained glass accurately.",
            ),
            hopt(
                H::AnimatedTextures,
                Low,
                "Updates animated water, lava, fire and portal textures.",
            ),
            opt(
                Opt::ChunkFade,
                None,
                "Smoothly fades newly loaded terrain chunks into view.",
            ),
            VideoRow::Header("Atmosphere & Effects"),
            opt(
                Opt::Clouds,
                Low,
                "Renders sky clouds in Fancy 3D, Fast 2D, or Off.",
            ),
            opt(
                Opt::CloudRange,
                Low,
                "How far clouds extend across the sky.",
            ),
            hopt(H::Sky, Low, "Sun, moon, stars and the sky gradient."),
            hopt(H::Fog, None, "Distance and underwater fog."),
            hopt(H::Weather, Low, "Rain, snow and lightning."),
            opt(
                Opt::WeatherRadius,
                Low,
                "Distance around the camera where rain and snow are drawn.",
            ),
            opt(
                Opt::Particles,
                Low,
                "Controls how many particle effects are drawn.",
            ),
            opt(
                Opt::EntityShadows,
                Low,
                "Draws soft circular shadows under mobs and players.",
            ),
            VideoRow::Header("Post & Camera"),
            hopt(
                H::Antialiasing,
                Medium,
                "Smooths jagged edges with FXAA or temporal TAA.",
            ),
            hopt(
                H::Upscaling,
                Saves,
                "Renders lower than screen resolution and reconstructs sharp detail via TAA.",
            ),
            opt(
                Opt::MenuBlur,
                Low,
                "Blurs the world behind open menus and overlays.",
            ),
            opt(
                Opt::Vignette,
                None,
                "Subtly darkens screen corners in dim areas.",
            ),
            opt(
                Opt::ViewBobbing,
                None,
                "Subtle camera sway while walking.",
            ),
            opt(
                Opt::Autosave,
                None,
                "Shows the autosave indicator when the world saves.",
            ),
            opt(
                Opt::AttackIndicator,
                None,
                "Shows attack cooldown charge on the crosshair or hotbar.",
            ),
        ],
        VoltTab::Performance => vec![
            VideoRow::Header("Distant Terrain & Culling"),
            hopt(
                H::LodDistance,
                Medium,
                "Coarse distant terrain in local worlds. Integrated GPUs stop at 128 chunks.",
            ),
            hopt(H::FarDetail, Medium, far_desc),
            hopt(
                H::CaveCulling,
                Saves,
                "Skips underground chunks hidden behind solid terrain.",
            ),
            hopt(
                H::EntityCulling,
                Saves,
                "Skips mobs hidden behind solid terrain.",
            ),
            opt(
                Opt::EntityDistance,
                Medium,
                "How far away mobs and entities remain visible.",
            ),
            VideoRow::Header("Chunks & GPU"),
            hopt(
                H::ChunkUpdates,
                Medium,
                "Limits chunk meshes rebuilt per frame. Lower reduces hitches while moving.",
            ),
            hopt(
                H::UploadBudget,
                Low,
                "Mesh data sent to the GPU per frame. Lower is smoother while loading.",
            ),
            opt(
                Opt::ChunkBuilder,
                Low,
                "Controls how aggressively nearby chunk updates block frame rendering.",
            ),
            hopt(
                H::Workers,
                None,
                "Threads that generate, light and mesh chunks. Next start.",
            ),
            hopt(
                H::ShadowCache,
                Saves,
                "Reuses the pack shadow map when sun, view and world match. Off by default.",
            ),
            hopt(
                H::BatteryMode,
                Saves,
                "Caps frame rate and halves background work to save battery.",
            ),
        ],
        VoltTab::Advanced => vec![
            VideoRow::Header("Overlay & Capture"),
            hopt(
                H::Overlay,
                None,
                "Frame rate and frame times over the game, with a graph.",
            ),
            hopt(
                H::ScreenshotQuality,
                Medium,
                "Multiplier for F2 screenshots (2x, 4x, or 8x resolution).",
            ),
            VideoRow::Header("Display & Backend"),
            opt(
                Opt::Preset,
                Medium,
                "Vanilla Fast, Fancy, or Fabulous graphics preset.",
            ),
            opt(
                Opt::Fullscreen,
                None,
                "Switches between windowed and fullscreen display.",
            ),
            opt(
                Opt::ExclusiveFullscreen,
                None,
                "Uses exclusive display mode when entering fullscreen.",
            ),
            opt(
                Opt::ExclusiveMode,
                None,
                "Resolution and refresh rate for exclusive fullscreen.",
            ),
            hopt(
                H::GraphicsApi,
                None,
                "Vulkan, then DirectX 12 (Windows) or OpenGL (Linux) where it fails. Next start.",
            ),
        ],
        VoltTab::Hdr => vec![
            VideoRow::Header("High Dynamic Range"),
            hopt(
                H::Hdr,
                None,
                "10-bit wide colour and bright highlights on HDR displays.",
            ),
            hopt(
                H::HdrCalibrate,
                None,
                "Guided brightness setup for your display.",
            ),
            hopt(
                H::HdrBlackLevel,
                None,
                "Raises near-black tones so dark caves stay visible.",
            ),
            hopt(
                H::HdrPeak,
                None,
                "Brightest peak highlights in nits. Auto follows your display.",
            ),
            hopt(
                H::HdrPaperWhite,
                None,
                "Standard white world brightness in nits.",
            ),
            hopt(
                H::HdrUiBrightness,
                None,
                "Scales menus and HUD brightness relative to the world.",
            ),
            hopt(
                H::HdrKey,
                None,
                "Key shortcut to toggle HDR on or off in-game.",
            ),
        ],
    };
    if cfg!(any(target_os = "macos", target_os = "android")) {
        rows.retain(
            |r| !matches!(r, VideoRow::Entry(e) if e.item == Item::Horizon(H::GraphicsApi)),
        );
    }
    rows.retain(|r| match r {
        VideoRow::Header(_) => true,
        VideoRow::Entry(e) => e.item.available(),
    });
    // Drop empty headers if every setting underneath was filtered out.
    let mut has_entry = false;
    rows.reverse();
    rows.retain(|r| match r {
        VideoRow::Entry(_) => {
            has_entry = true;
            true
        }
        VideoRow::Header(_) => std::mem::take(&mut has_entry),
    });
    rows.reverse();
    rows
}

fn item_blocked(item: Item, s: &Settings) -> Option<&'static str> {
    if let Some(why) = item.blocked(s) {
        return Some(why);
    }
    match item {
        Item::Vanilla(Opt::MaxAnisotropy) if s.file.int("textureFiltering") != 2 => {
            Some("Requires Texture Filtering set to Anisotropic.")
        }
        Item::Vanilla(Opt::ExclusiveMode) if s.video_modes.is_empty() => {
            Some("No exclusive monitor modes reported.")
        }
        _ => None,
    }
}

fn split_caption_and_value(item: Item, s: &Settings, binding_hdr: bool) -> (String, String) {
    if item == Item::Horizon(H::HdrKey) && binding_hdr {
        let key_name = crate::keybinds::display_name(&s.horizon.hdr_key).plain();
        return (lang::get("Shortcut"), format!("> {key_name} <"));
    }
    let full = item.label(s).plain();
    if let Some((cap, val)) = full.split_once(": ") {
        let mut val = val.to_owned();
        if let Some(prefix) = val.strip_suffix(" chunks") {
            val = format!("{prefix} Chunks");
        }
        return (cap.to_owned(), val);
    }
    if item == Item::Horizon(H::HdrCalibrate) {
        return (lang::get("Calibration"), full);
    }
    if let Item::Vanilla(opt) = item {
        return (lang::get(opt.caption()), full);
    }
    (full, String::new())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltAction {
    ButtonClick,
    Changed,
    SliderDrag,
    BindKey(&'static str),
    Calibrate,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltFocus {
    Search,
    Tab(VoltTab),
    Item(Item),
    Done,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub(crate) fn contains(self, p: (f32, f32)) -> bool {
        ui::hovered(p, self.x, self.y, self.w, self.h)
    }
    pub(crate) fn array(self) -> [f32; 4] {
        [self.x, self.y, self.w, self.h]
    }
}

#[derive(Clone, Copy, Debug)]
struct LayoutGeometry {
    side_x: f32,
    side_w: f32,
    brand_y: f32,
    search_box: Rect,
    tabs: [(VoltTab, Rect); 5],
    done_btn: Rect,
    main_x: f32,
    main_w: f32,
    header_bar: Rect,
    list_panel: Rect,
    list_clip: Rect,
    scrollbar_x: f32,
    footer_card: Rect,
}

#[derive(Clone, Debug)]
enum LaidRow {
    Header {
        text: String,
        y: f32,
    },
    Entry {
        entry: VideoEntry,
        rect: Rect,
        ctrl: Rect,
        is_slider: bool,
        is_action: bool,
    },
}

pub(crate) const GROUP_HEIGHT: f32 = 17.0;
pub(crate) const ROW_HEIGHT: f32 = 20.0;
pub(crate) const ROW_STRIDE: f32 = 22.0;

pub struct VoltVideoUi {
    pub tab: VoltTab,
    pub scroll: ui::Scroll,
    pub dragging: Option<Item>,
    pub focus: Option<VoltFocus>,
    pub can_change_value: bool,
    pub inspected: Option<Item>,

    opened_at: Instant,
    tab_changed_at: Instant,
    tab_dir: f32,
    inspected_changed_at: Instant,
    last_draw: Option<Instant>,

    visual_scroll: Option<f32>,
    visual_tab_y: Option<f32>,
    done_hover: f32,
    tab_hover: [f32; 5],
    row_hover: Vec<(Item, f32)>,
    slider_anim: Vec<(Item, f32)>,
}

impl Default for VoltVideoUi {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            tab: VoltTab::General,
            scroll: ui::Scroll::default(),
            dragging: None,
            focus: None,
            can_change_value: false,
            inspected: None,
            opened_at: now,
            tab_changed_at: now,
            tab_dir: 1.0,
            inspected_changed_at: now,
            last_draw: None,
            visual_scroll: None,
            visual_tab_y: None,
            done_hover: 0.0,
            tab_hover: [0.0; 5],
            row_hover: Vec::new(),
            slider_anim: Vec::new(),
        }
    }
}

impl VoltVideoUi {
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.tab = VoltTab::General;
        self.scroll = ui::Scroll::default();
        self.dragging = None;
        self.focus = None;
        self.can_change_value = false;
        self.inspected = None;
        self.opened_at = now;
        self.tab_changed_at = now;
        self.tab_dir = 1.0;
        self.inspected_changed_at = now;
        self.last_draw = None;
        self.visual_scroll = None;
        self.visual_tab_y = None;
        self.done_hover = 0.0;
        self.tab_hover = [0.0; 5];
        self.row_hover.clear();
        self.slider_anim.clear();
    }

    pub fn release(&mut self) -> bool {
        self.scroll.release();
        self.dragging.take().is_some()
    }

    pub fn reset_scroll(&mut self) {
        self.scroll.amount = 0.0;
        self.visual_scroll = Some(0.0);
    }

    pub fn select_tab(&mut self, next: VoltTab) {
        if self.tab != next {
            self.tab_dir = if next.index() > self.tab.index() {
                1.0
            } else {
                -1.0
            };
            self.tab = next;
            self.tab_changed_at = Instant::now();
            self.scroll.amount = 0.0;
            self.visual_scroll = Some(0.0);
            self.dragging = None;
            self.inspected = None;
            self.inspected_changed_at = Instant::now();
        }
    }

    fn geometry(w: f32, h: f32) -> LayoutGeometry {
        let pad_x = if w < 380.0 { 8.0 } else { 12.0 };
        let pad_top = 8.0;
        let pad_bottom = 8.0;
        let side_w = (w * 0.23).clamp(84.0, 118.0);
        let side_x = pad_x;
        let brand_y = pad_top;

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
                VoltTab::ALL[i],
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
        let scrollbar_x = list_panel.x + list_panel.w - 6.0;

        LayoutGeometry {
            side_x,
            side_w,
            brand_y,
            search_box,
            tabs,
            done_btn,
            main_x,
            main_w,
            header_bar,
            list_panel,
            list_clip,
            scrollbar_x,
            footer_card,
        }
    }

    fn filtered_rows(&self, s: &Settings, query: &str) -> (String, Vec<(Option<String>, VideoEntry)>) {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            let mut out = Vec::new();
            let mut current_header = None;
            for r in tab_rows(self.tab) {
                match r {
                    VideoRow::Header(h) => current_header = Some(lang::get(h)),
                    VideoRow::Entry(e) => {
                        out.push((current_header.take(), e));
                    }
                }
            }
            return (lang::get(self.tab.label()), out);
        }

        let mut out = Vec::new();
        for tab in VoltTab::ALL {
            let mut current_group = "";
            let mut pending_header = None;
            for r in tab_rows(tab) {
                match r {
                    VideoRow::Header(h) => {
                        current_group = h;
                        pending_header = Some(format!("{} · {}", lang::get(tab.label()), lang::get(h)));
                    }
                    VideoRow::Entry(e) => {
                        let (cap, val) = split_caption_and_value(e.item, s, false);
                        let desc = graphics_ui::description(e.item, s, e.text);
                        let matches = cap.to_lowercase().contains(&q)
                            || val.to_lowercase().contains(&q)
                            || lang::get(current_group).to_lowercase().contains(&q)
                            || lang::get(desc).to_lowercase().contains(&q);
                        if matches {
                            out.push((pending_header.take(), e));
                        }
                    }
                }
            }
        }
        (format!("Search: {}", query.trim()), out)
    }

    fn build_list_rows(
        &self,
        s: &Settings,
        geom: &LayoutGeometry,
        query: &str,
        scroll_amount: f32,
    ) -> (String, Vec<LaidRow>, f32) {
        let (title, entries) = self.filtered_rows(s, query);
        let row_x = geom.list_panel.x + 4.0;
        let row_w = (geom.list_panel.w - 12.0).max(80.0);
        let ctrl_w = (row_w * 0.44).clamp(88.0, 152.0);
        let ctrl_x = row_x + row_w - ctrl_w - 4.0;

        let mut rows = Vec::with_capacity(entries.len() + 6);
        let mut content_y = 4.0f32;
        for (header, entry) in entries {
            if let Some(h_text) = header {
                if content_y > 6.0 {
                    content_y += 4.0;
                }
                rows.push(LaidRow::Header {
                    text: h_text,
                    y: geom.list_panel.y + content_y - scroll_amount,
                });
                content_y += GROUP_HEIGHT;
            }
            let is_slider = entry.item.slider(s).is_some();
            let is_action = matches!(
                entry.item,
                Item::Horizon(H::HdrCalibrate | H::HdrSettings | H::HdrKey)
            );
            rows.push(LaidRow::Entry {
                entry,
                rect: Rect {
                    x: row_x,
                    y: geom.list_panel.y + content_y - scroll_amount,
                    w: row_w,
                    h: ROW_HEIGHT,
                },
                ctrl: Rect {
                    x: ctrl_x,
                    y: geom.list_panel.y + content_y - scroll_amount,
                    w: ctrl_w,
                    h: ROW_HEIGHT,
                },
                is_slider,
                is_action,
            });
            content_y += ROW_STRIDE;
        }
        let content_h = (content_y + 2.0).max(0.0);
        (title, rows, content_h)
    }

    fn clamp_scroll(&mut self, geom: &LayoutGeometry, content_h: f32) {
        self.scroll
            .clamp(geom.list_panel.y, geom.list_panel.y + geom.list_panel.h, content_h);
    }

    pub fn scroll(&mut self, s: &Settings, size: (f32, f32), query: &str, amount: f32) {
        let geom = Self::geometry(size.0, size.1);
        let (_, _, content_h) = self.build_list_rows(s, &geom, query, self.scroll.amount);
        self.scroll.wheel(
            amount,
            20.0,
            geom.list_panel.y,
            geom.list_panel.y + geom.list_panel.h,
            content_h,
        );
    }

    fn set_inspected(&mut self, item: Item) {
        if self.inspected != Some(item) {
            self.inspected = Some(item);
            self.inspected_changed_at = Instant::now();
        }
    }

    fn smooth_value(list: &mut Vec<(Item, f32)>, item: Item, target: f32, dt: f32, speed: f32) -> f32 {
        if let Some((_, val)) = list.iter_mut().find(|(k, _)| *k == item) {
            if dt > 0.0 {
                let k = 1.0 - (-speed * dt).exp();
                *val += (target - *val) * k;
            }
            *val
        } else {
            list.push((item, target));
            target
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        s: &Settings,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        in_world: bool,
        search: &mut ui::EditBox,
        binding_key: Option<&'static str>,
    ) {
        let (w, h) = (batch.width, batch.height);
        let now = Instant::now();
        let dt = self
            .last_draw
            .map_or(0.0, |prev| now.saturating_duration_since(prev).as_secs_f32().clamp(0.0, 0.05));
        self.last_draw = Some(now);

        let open_p = horizon_theme::ease_out_cubic(
            (now.saturating_duration_since(self.opened_at).as_secs_f32() / 0.22).clamp(0.0, 1.0),
        );
        let tab_p = horizon_theme::ease_out_cubic(
            (now.saturating_duration_since(self.tab_changed_at).as_secs_f32() / 0.18).clamp(0.0, 1.0),
        );

        let geom = Self::geometry(w, h);
        let (_, _, content_h) = self.build_list_rows(s, &geom, &search.value, self.scroll.amount);
        self.clamp_scroll(&geom, content_h);

        let scroll_draw = if self.scroll.held() {
            self.visual_scroll = Some(self.scroll.amount);
            self.scroll.amount
        } else {
            let cur = self.visual_scroll.unwrap_or(self.scroll.amount);
            let next = if dt > 0.0 {
                let k = 1.0 - (-24.0 * dt).exp();
                cur + (self.scroll.amount - cur) * k
            } else {
                self.scroll.amount
            };
            let max_scroll = (content_h - geom.list_panel.h).max(0.0);
            let clamped = next.clamp(0.0, max_scroll);
            self.visual_scroll = Some(clamped);
            clamped
        };

        let (header_title, rows, _) = self.build_list_rows(s, &geom, &search.value, scroll_draw);

        // Backdrop scrim (keeps world/menu background subtly tinted lighter violet-slate)
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, in_world);
        let scrim_a = if in_world { 74.0 } else { 96.0 } * (0.35 + 0.65 * open_p);
        batch.fill(atlas, 0.0, 0.0, w, h, rgba(18, 17, 28, scrim_a.round() as u8));

        let side_dx = (1.0 - open_p) * -10.0;
        let main_dy = (1.0 - open_p) * 6.0;
        let ui_alpha = 0.25 + 0.75 * open_p;

        // Volt Graphics palette — lighter frosted slate surfaces for higher clarity
        let panel_bg = with_alpha(rgba(34, 32, 48, 224), ui_alpha);
        let bar_bg = with_alpha(rgba(42, 40, 60, 232), ui_alpha);
        let border_subtle = with_alpha(rgba(255, 255, 255, 58), ui_alpha);
        let border_strong = with_alpha(rgba(255, 255, 255, 232), ui_alpha);
        let accent = with_alpha(rgba(116, 92, 235, 255), ui_alpha);
        let accent_hi = with_alpha(rgba(140, 118, 248, 255), ui_alpha);
        let accent_edge = with_alpha(rgba(180, 164, 255, 210), ui_alpha);
        let text_primary = with_alpha(rgba(248, 247, 255, 255), ui_alpha);
        let text_muted = with_alpha(rgba(188, 186, 206, 255), ui_alpha);
        let text_dim = with_alpha(rgba(136, 134, 154, 255), ui_alpha);

        // 1. Left Sidebar Brand ("Volt" / "Graphics")
        let bx = geom.side_x + side_dx;
        let by = geom.brand_y;
        batch.fill(atlas, bx + 4.0, by + 3.0, 8.0, 8.0, accent);
        batch.fill(atlas, bx + 4.0, by + 3.0, 8.0, 1.0, accent_edge);
        batch.text(atlas, "Volt", bx + 15.0, by + 3.0, text_primary, true);
        batch.text(atlas, "Graphics", bx + 4.0, by + 14.0, text_muted, true);

        // 2. Search Options Input Box
        let sb = geom.search_box;
        let sb_x = sb.x + side_dx;
        let sb_hover = sb.contains(cursor);
        let sb_active = search.focused || self.focus == Some(VoltFocus::Search);
        batch.fill(atlas, sb_x, sb.y, sb.w, sb.h, bar_bg);
        let sb_border = if sb_active {
            border_strong
        } else if sb_hover {
            with_alpha(rgba(255, 255, 255, 115), ui_alpha)
        } else {
            with_alpha(rgba(255, 255, 255, 58), ui_alpha)
        };
        draw_border(batch, atlas, sb_x, sb.y, sb.w, sb.h, sb_border);

        // Sync EditBox position inside our borderless styled container; report full box for Android IME
        crate::widgets::target_as(sb.array(), crate::widgets::Kind::Field);
        search.bordered = false;
        search.x = sb.x + 5.0;
        search.y = sb.y + 4.0;
        search.width = (sb.w - 10.0).max(16.0);
        search.height = 10.0;
        search.text_color = text_primary;
        search.message = "Search options".to_owned();
        if search.value.is_empty() && !search.focused {
            batch.text(atlas, "Search options", sb_x + 5.0, sb.y + 4.0, text_dim, false);
        }
        // Temporarily apply slide offset for drawing, then restore canonical hit-test coords
        let saved_sx = search.x;
        search.x = sb_x + 5.0;
        search.draw(batch, atlas, None);
        search.x = saved_sx;
        crate::widgets::widget_cursor(sb_hover, true);

        // 3. Sidebar Category Tabs + Gliding Active Outline
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
            let focused = self.focus == Some(VoltFocus::Tab(*tab));
            let hovered = rect.contains(cursor);
            let target_h = if hovered || focused { 1.0 } else { 0.0 };
            if dt > 0.0 {
                let k = 1.0 - (-18.0 * dt).exp();
                self.tab_hover[i] += (target_h - self.tab_hover[i]) * k;
            } else {
                self.tab_hover[i] = target_h;
            }
            let rx = rect.x + side_dx;
            let fill_a = if selected {
                0.85
            } else {
                0.08 * self.tab_hover[i]
            };
            if fill_a > 0.01 {
                let fill = if selected {
                    with_alpha(rgba(60, 56, 86, 235), ui_alpha)
                } else {
                    with_alpha(rgba(255, 255, 255, (fill_a * 255.0) as u8), ui_alpha)
                };
                batch.fill(atlas, rx, rect.y, rect.w, rect.h, fill);
            }
            if focused && !selected {
                draw_border(batch, atlas, rx, rect.y, rect.w, rect.h, with_alpha(accent_edge, 0.7));
            }
            let label = lang::get(tab.label());
            let col = if selected {
                text_primary
            } else {
                lerp_color(text_muted, text_primary, self.tab_hover[i] * 0.7)
            };
            batch.text(atlas, &label, rx + 7.0, rect.y + 4.0, col, true);
            crate::widgets::record_as(&label, rect.array(), true, crate::widgets::Kind::Tab(selected));
            crate::widgets::widget_cursor(hovered, true);
        }

        if !searching {
            let rx = active_tab_rect.x + side_dx;
            draw_border(batch, atlas, rx, tab_y, active_tab_rect.w, active_tab_rect.h, border_strong);
            batch.fill(atlas, rx + 1.0, tab_y + 3.0, 2.0, active_tab_rect.h - 6.0, accent_hi);
        }

        // 4. Bottom-left Purple Done Button
        let done = geom.done_btn;
        let done_hovered = done.contains(cursor) || self.focus == Some(VoltFocus::Done);
        if dt > 0.0 {
            let k = 1.0 - (-18.0 * dt).exp();
            let target = if done_hovered { 1.0 } else { 0.0 };
            self.done_hover += (target - self.done_hover) * k;
        } else {
            self.done_hover = if done_hovered { 1.0 } else { 0.0 };
        }
        let done_x = done.x + side_dx;
        let done_fill = lerp_color(accent, accent_hi, self.done_hover);
        batch.fill(atlas, done_x, done.y, done.w, done.h, done_fill);
        batch.fill(atlas, done_x, done.y, done.w, 1.0, accent_edge);
        if self.focus == Some(VoltFocus::Done) {
            draw_border(batch, atlas, done_x, done.y, done.w, done.h, border_strong);
        }
        let done_text = Text::tr("gui.done", &[]);
        ui::centered_text(batch, atlas, &done_text, done_x + 2.0, done.y + 6.0, done.w - 4.0, text_primary);
        crate::widgets::record(&done_text.plain(), done.array(), true);
        crate::widgets::widget_cursor(done_hovered, true);

        // 5. Right Main Area: Header Bar
        let hdr = geom.header_bar;
        let hdr_y = hdr.y + main_dy;
        batch.fill(atlas, hdr.x, hdr_y, hdr.w, hdr.h, bar_bg);
        draw_border(batch, atlas, hdr.x, hdr_y, hdr.w, hdr.h, border_subtle);
        let title_alpha = ui_alpha * (0.35 + 0.65 * tab_p);
        batch.text(
            atlas,
            &header_title,
            hdr.x + 8.0,
            hdr_y + 5.0,
            with_alpha(text_primary, title_alpha),
            true,
        );
        if !cfg!(target_os = "android") && s.horizon.api != s.startup_api {
            let note = lang::get("Restart required");
            let nw = HudBatch::text_width(atlas, &note);
            batch.text(
                atlas,
                &note,
                hdr.x + hdr.w - nw - 8.0,
                hdr_y + 5.0,
                with_alpha(rgba(242, 198, 96, 255), ui_alpha),
                true,
            );
        }

        // 6. Right Main Area: Scrollable Options List Panel
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

        let mut hovered_entry: Option<VideoEntry> = None;
        let mut focused_entry: Option<VideoEntry> = None;
        let mut first_entry: Option<VideoEntry> = None;
        let mut tooltip: Option<Vec<Text>> = None;
        let in_list_y = cursor.1 >= lp.y && cursor.1 < lp.y + lp.h;

        for row in &rows {
            match row {
                LaidRow::Header { text, y } => {
                    let dy = *y + content_slide_y;
                    if dy + GROUP_HEIGHT < clip_y0 - 4.0 || dy > clip_y1 + 4.0 {
                        continue;
                    }
                    batch.text(
                        atlas,
                        text,
                        lp.x + 8.0,
                        dy + 4.0,
                        with_alpha(text_muted, content_alpha),
                        true,
                    );
                    batch.fill(
                        atlas,
                        lp.x + 8.0,
                        dy + GROUP_HEIGHT - 3.0,
                        lp.w - 20.0,
                        1.0,
                        with_alpha(rgba(255, 255, 255, 22), content_alpha),
                    );
                }
                LaidRow::Entry {
                    entry,
                    rect,
                    ctrl,
                    is_slider,
                    is_action,
                } => {
                    if first_entry.is_none() {
                        first_entry = Some(*entry);
                    }
                    let dy = rect.y + content_slide_y;
                    let visible = dy + rect.h >= clip_y0 - 4.0 && dy <= clip_y1 + 4.0;

                    let blocked = item_blocked(entry.item, s);
                    let live = blocked.is_none();
                    let is_hovered = in_list_y && rect.contains(cursor);
                    let is_focused = self.focus == Some(VoltFocus::Item(entry.item));
                    let is_dragging = self.dragging == Some(entry.item);
                    let is_selected = self.inspected == Some(entry.item);

                    if is_hovered {
                        hovered_entry = Some(*entry);
                    }
                    if is_focused {
                        focused_entry = Some(*entry);
                    }
                    if (is_hovered || is_focused) && let Some(why) = blocked {
                        tooltip = Some(atlas.font.split(&Text::literal(lang::get(why)), 180.0));
                    }

                    let target_hl = if is_hovered || is_focused || is_dragging || is_selected {
                        1.0
                    } else {
                        0.0
                    };
                    let hl = Self::smooth_value(&mut self.row_hover, entry.item, target_hl, dt, 18.0);

                    if !visible {
                        continue;
                    }

                    // Row card plate
                    let idle_fill = with_alpha(rgba(255, 255, 255, 20), content_alpha);
                    let active_fill = with_alpha(rgba(255, 255, 255, 44), content_alpha);
                    let row_bg = lerp_color(idle_fill, active_fill, hl);
                    batch.fill(atlas, rect.x, dy, rect.w, rect.h, row_bg);

                    if is_selected || is_focused || is_dragging {
                        let bar_a = if is_focused || is_dragging { 1.0 } else { 0.7 };
                        batch.fill(
                            atlas,
                            rect.x,
                            dy + 3.0,
                            2.0,
                            rect.h - 6.0,
                            with_alpha(accent_hi, content_alpha * bar_a),
                        );
                    }

                    let binding_hdr = binding_key == Some("hdr_toggle");
                    let (cap, val) = split_caption_and_value(entry.item, s, binding_hdr);
                    let label_max = (ctrl.x - (rect.x + 7.0) - 6.0).max(24.0);
                    let cap_drawn = fit_with_ellipsis(atlas, &cap, label_max);
                    let cap_color = if live {
                        with_alpha(text_primary, content_alpha)
                    } else {
                        with_alpha(text_dim, content_alpha)
                    };
                    batch.text(atlas, &cap_drawn, rect.x + 7.0, dy + 6.0, cap_color, true);

                    let full_label = entry.item.label(s).plain();
                    if *is_slider {
                        let real_v = entry.item.slider(s).unwrap_or(0.0).clamp(0.0, 1.0);
                        let shown_v = if is_dragging {
                            Self::smooth_value(&mut self.slider_anim, entry.item, real_v, 0.0, 22.0);
                            if let Some((_, sv)) = self.slider_anim.iter_mut().find(|(k, _)| *k == entry.item) {
                                *sv = real_v;
                            }
                            real_v
                        } else {
                            Self::smooth_value(&mut self.slider_anim, entry.item, real_v, dt, 22.0)
                        };

                        // Value label centered above track
                        let val_color = if live {
                            with_alpha(text_primary, content_alpha)
                        } else {
                            with_alpha(text_dim, content_alpha)
                        };
                        let vw = HudBatch::text_width(atlas, &val);
                        let vx = (ctrl.x + (ctrl.w - vw) * 0.5).floor();
                        batch.text(atlas, &val, vx, dy + 2.0, val_color, true);

                        // Slider track + purple fill + white thumb
                        let track_x = ctrl.x + 6.0;
                        let track_w = (ctrl.w - 12.0).max(16.0);
                        let track_y = dy + 14.0;
                        let track_bg = with_alpha(rgba(255, 255, 255, 52), content_alpha);
                        let track_fill = if live {
                            with_alpha(rgba(130, 108, 245, 255), content_alpha)
                        } else {
                            with_alpha(rgba(110, 108, 128, 180), content_alpha)
                        };
                        batch.fill(atlas, track_x, track_y, track_w, 2.0, track_bg);
                        batch.fill(atlas, track_x, track_y, track_w * shown_v, 2.0, track_fill);

                        let active_thumb = is_hovered || is_dragging || (is_focused && self.can_change_value);
                        let thumb_w = if active_thumb { 4.0 } else { 3.0 };
                        let thumb_h = if active_thumb { 8.0 } else { 6.0 };
                        let thumb_x = (track_x + track_w * shown_v - thumb_w * 0.5)
                            .clamp(track_x - 1.0, track_x + track_w - thumb_w + 1.0);
                        let thumb_y = track_y + 1.0 - thumb_h * 0.5;
                        let thumb_col = if live {
                            with_alpha(rgba(255, 255, 255, 255), content_alpha)
                        } else {
                            with_alpha(text_dim, content_alpha)
                        };
                        batch.fill(atlas, thumb_x, thumb_y, thumb_w, thumb_h, thumb_col);

                        crate::widgets::record_as(
                            &full_label,
                            ctrl.array(),
                            live,
                            crate::widgets::Kind::Slider {
                                value: shown_v,
                                track: [track_x, track_w],
                            },
                        );
                    } else {
                        // Cycle selector `< Value >` or action pill
                        let val_w = HudBatch::text_width(atlas, &val);
                        let min_pill = (ctrl.w * 0.68).max(68.0);
                        let needed = val_w + if *is_action { 16.0 } else { 28.0 };
                        let pill_w = needed.clamp(min_pill.min(ctrl.w), ctrl.w);
                        let pill_x = ctrl.x + ctrl.w - pill_w;
                        let pill_y = dy + 2.0;
                        let pill_h = 16.0;

                        let pill_bg = with_alpha(
                            rgba(46, 43, 68, (175.0 + 45.0 * hl) as u8),
                            content_alpha,
                        );
                        batch.fill(atlas, pill_x, pill_y, pill_w, pill_h, pill_bg);
                        if is_focused || is_hovered || is_selected {
                            let edge_a = if is_focused {
                                0.95
                            } else if is_hovered {
                                0.85
                            } else {
                                0.65
                            };
                            draw_border(
                                batch,
                                atlas,
                                pill_x,
                                pill_y,
                                pill_w,
                                pill_h,
                                with_alpha(border_strong, content_alpha * edge_a),
                            );
                        } else {
                            draw_border(
                                batch,
                                atlas,
                                pill_x,
                                pill_y,
                                pill_w,
                                pill_h,
                                with_alpha(rgba(255, 255, 255, 28), content_alpha),
                            );
                        }

                        let val_col = if live {
                            with_alpha(text_primary, content_alpha)
                        } else {
                            with_alpha(text_dim, content_alpha)
                        };
                        if !*is_action {
                            let arrow_col = if live {
                                with_alpha(text_muted, content_alpha)
                            } else {
                                with_alpha(text_dim, content_alpha)
                            };
                            batch.text(atlas, "<", pill_x + 5.0, pill_y + 4.0, arrow_col, false);
                            let gt_w = HudBatch::text_width(atlas, ">");
                            batch.text(
                                atlas,
                                ">",
                                pill_x + pill_w - gt_w - 5.0,
                                pill_y + 4.0,
                                arrow_col,
                                false,
                            );
                        }
                        let inner_w = (pill_w - if *is_action { 10.0 } else { 22.0 }).max(10.0);
                        let val_drawn = fit_with_ellipsis(atlas, &val, inner_w);
                        let dw = HudBatch::text_width(atlas, &val_drawn);
                        let vx = (pill_x + (pill_w - dw) * 0.5).floor();
                        batch.text(atlas, &val_drawn, vx, pill_y + 4.0, val_col, true);

                        crate::widgets::record_as(
                            &full_label,
                            [pill_x, rect.y, pill_w, rect.h],
                            live,
                            crate::widgets::Kind::Button,
                        );
                    }
                    crate::widgets::widget_cursor(is_hovered, live);
                }
            }
        }

        batch.clip_since(list_verts, clip_x0, clip_y0, clip_x1, clip_y1);
        crate::widgets::clip_targets(target_mark, lp.y, lp.y + lp.h);

        // Custom subtle scrollbar inside list panel
        if let Some((thumb_y, thumb_h)) =
            ui::scrollbar_thumb(lp.y + 3.0, lp.y + lp.h - 3.0, content_h, scroll_draw)
        {
            let bx = geom.scrollbar_x;
            batch.fill(
                atlas,
                bx,
                lp_y + 3.0,
                3.0,
                lp.h - 6.0,
                with_alpha(rgba(255, 255, 255, 18), ui_alpha),
            );
            let thumb_col = if self.scroll.held() {
                accent_hi
            } else {
                with_alpha(rgba(255, 255, 255, 115), ui_alpha)
            };
            batch.fill(atlas, bx, thumb_y + main_dy, 3.0, thumb_h, thumb_col);
        }

        // Update inspected option (hover/focus takes priority, else persisted selection, else first)
        let active_entry = hovered_entry
            .or(focused_entry)
            .or_else(|| {
                self.inspected.and_then(|sel| {
                    rows.iter().find_map(|r| match r {
                        LaidRow::Entry { entry, .. } if entry.item == sel => Some(*entry),
                        _ => None,
                    })
                })
            })
            .or(first_entry);

        if let Some(ae) = active_entry {
            if hovered_entry.is_some() || focused_entry.is_some() || self.inspected.is_none() {
                self.set_inspected(ae.item);
            }
        }

        // 7. Bottom-Right Description Footer Card
        let fc = geom.footer_card;
        let fc_y = fc.y + main_dy;
        batch.fill(atlas, fc.x, fc_y, fc.w, fc.h, bar_bg);
        draw_border(batch, atlas, fc.x, fc_y, fc.w, fc.h, border_subtle);

        if let Some(entry) = active_entry {
            let desc_p = horizon_theme::ease_out_cubic(
                (now.saturating_duration_since(self.inspected_changed_at).as_secs_f32() / 0.14)
                    .clamp(0.0, 1.0),
            );
            let desc_alpha = ui_alpha * (0.35 + 0.65 * desc_p);
            let desc_dy = (1.0 - desc_p) * 3.0;

            let (name, _) = split_caption_and_value(entry.item, s, binding_key == Some("hdr_toggle"));
            let name_col = with_alpha(text_primary, desc_alpha);
            let nw = batch.text(atlas, &name, fc.x + 8.0, fc_y + 6.0 + desc_dy, name_col, true);

            let cost_col = with_alpha(cost_color(entry.cost), desc_alpha);
            let sq_x = fc.x + 8.0 + nw + 8.0;
            let sq_y = fc_y + 7.0 + desc_dy;
            batch.fill(atlas, sq_x, sq_y, 6.0, 6.0, cost_col);
            batch.text(
                atlas,
                &lang::get(entry.cost.label()),
                sq_x + 9.0,
                fc_y + 6.0 + desc_dy,
                cost_col,
                true,
            );

            let desc_raw = graphics_ui::description(entry.item, s, entry.text);
            let desc_text = if let Some(why) = item_blocked(entry.item, s) {
                format!("{} ({})", lang::get(desc_raw), lang::get(why))
            } else {
                lang::get(desc_raw)
            };
            let lines = atlas
                .font
                .split(&Text::literal(desc_text), (fc.w - 16.0).max(40.0));
            for (li, line) in lines.iter().take(2).enumerate() {
                batch.text_styled(
                    atlas,
                    line,
                    fc.x + 8.0,
                    fc_y + 17.0 + li as f32 * 9.0 + desc_dy,
                    with_alpha(text_muted, desc_alpha),
                    true,
                );
            }
        }

        if let Some(lines) = tooltip {
            let max_w = lines
                .iter()
                .map(|l| atlas.font.width_text(l))
                .fold(0.0f32, f32::max);
            let box_w = max_w + 12.0;
            let box_h = lines.len() as f32 * 9.0 + 10.0;
            let tx = (cursor.0 + 10.0).min((w - box_w - 6.0).max(4.0));
            let ty = (cursor.1 - box_h - 6.0).max(4.0);
            batch.fill(atlas, tx, ty, box_w, box_h, rgba(16, 15, 24, 242));
            draw_border(batch, atlas, tx, ty, box_w, box_h, rgba(255, 255, 255, 85));
            for (i, line) in lines.iter().enumerate() {
                batch.text_styled(atlas, line, tx + 6.0, ty + 5.0 + i as f32 * 9.0, text_primary, true);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn click(
        &mut self,
        s: &mut Settings,
        atlas: &HudAtlas,
        pos: (f32, f32),
        size: (f32, f32),
        framebuffer: [i32; 2],
        drag: bool,
        right: bool,
        search: &mut ui::EditBox,
    ) -> Vec<VoltAction> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, content_h) = self.build_list_rows(s, &geom, &search.value, self.scroll.amount);
        let mut actions = Vec::new();

        if drag {
            if self.scroll.held() {
                self.scroll.drag(
                    pos.1,
                    geom.list_panel.y + 3.0,
                    geom.list_panel.y + geom.list_panel.h - 3.0,
                    content_h,
                );
                self.visual_scroll = Some(self.scroll.amount);
                return actions;
            }
            if let Some(item) = self.dragging
                && let Some(LaidRow::Entry { ctrl, .. }) = rows.iter().find(|r| {
                    matches!(r, LaidRow::Entry { entry, .. } if entry.item == item)
                })
            {
                let track_x = ctrl.x + 6.0;
                let track_w = (ctrl.w - 12.0).max(16.0);
                let v = ((pos.0 - track_x) / track_w).clamp(0.0, 1.0);
                item.set_slider(s, v);
                if let Some((_, sv)) = self.slider_anim.iter_mut().find(|(k, _)| *k == item) {
                    *sv = item.slider(s).unwrap_or(v);
                }
                actions.push(VoltAction::SliderDrag);
            }
            return actions;
        }

        // Search box focus & click
        if !right {
            let hit_search = geom.search_box.contains(pos);
            search.focused = hit_search;
            if hit_search {
                search.bordered = false;
                search.x = geom.search_box.x + 5.0;
                search.y = geom.search_box.y + 4.0;
                search.width = (geom.search_box.w - 10.0).max(16.0);
                search.click(atlas, pos.0, false, false);
                self.focus = Some(VoltFocus::Search);
                self.can_change_value = false;
                actions.push(VoltAction::ButtonClick);
                return actions;
            }
        }

        // Sidebar tabs & Done button (left click)
        if !right {
            for (tab, rect) in geom.tabs {
                if rect.contains(pos) {
                    if !search.value.is_empty() {
                        search.set_value("");
                    }
                    self.select_tab(tab);
                    self.focus = Some(VoltFocus::Tab(tab));
                    self.can_change_value = false;
                    actions.push(VoltAction::ButtonClick);
                    return actions;
                }
            }
            if geom.done_btn.contains(pos) {
                self.focus = Some(VoltFocus::Done);
                actions.push(VoltAction::ButtonClick);
                actions.push(VoltAction::Done);
                return actions;
            }
        }

        // Scrollbar column press
        if !right
            && ui::scrollbar_thumb(
                geom.list_panel.y + 3.0,
                geom.list_panel.y + geom.list_panel.h - 3.0,
                content_h,
                self.scroll.amount,
            )
            .is_some()
            && self.scroll.press(
                pos,
                geom.scrollbar_x - 2.0,
                geom.list_panel.y + 3.0,
                geom.list_panel.y + geom.list_panel.h - 3.0,
                content_h,
            )
        {
            return actions;
        }

        // List rows
        if pos.1 >= geom.list_panel.y && pos.1 < geom.list_panel.y + geom.list_panel.h {
            for row in &rows {
                let LaidRow::Entry {
                    entry,
                    rect,
                    ctrl,
                    is_slider,
                    is_action,
                } = row
                else {
                    continue;
                };
                if !rect.contains(pos) {
                    continue;
                }
                self.set_inspected(entry.item);
                self.focus = Some(VoltFocus::Item(entry.item));
                self.can_change_value = *is_slider;

                if item_blocked(entry.item, s).is_some() {
                    return actions;
                }

                if *is_slider {
                    if right {
                        return actions;
                    }
                    // Clicking on the right slider area grabs/sets slider; clicking left label selects row
                    if pos.0 >= ctrl.x {
                        let track_x = ctrl.x + 6.0;
                        let track_w = (ctrl.w - 12.0).max(16.0);
                        let v = ((pos.0 - track_x) / track_w).clamp(0.0, 1.0);
                        entry.item.set_slider(s, v);
                        self.dragging = Some(entry.item);
                        actions.push(VoltAction::SliderDrag);
                    } else {
                        actions.push(VoltAction::ButtonClick);
                    }
                    return actions;
                }

                // Cycle / action row
                if entry.item == Item::Horizon(H::HdrCalibrate) {
                    if !right {
                        if matches!(graphics_ui::setup(s), graphics_ui::Setup::Changed) {
                            s.horizon.hdr_peak = 0;
                            s.horizon.hdr_paper_white = 0;
                            actions.push(VoltAction::ButtonClick);
                            actions.push(VoltAction::Changed);
                        } else {
                            actions.push(VoltAction::ButtonClick);
                            actions.push(VoltAction::Calibrate);
                        }
                    }
                    return actions;
                }
                if entry.item == Item::Horizon(H::HdrKey) {
                    if !right {
                        actions.push(VoltAction::ButtonClick);
                        actions.push(VoltAction::BindKey("hdr_toggle"));
                    }
                    return actions;
                }
                if entry.item == Item::Horizon(H::HdrSettings) {
                    if !right {
                        self.select_tab(VoltTab::Hdr);
                        actions.push(VoltAction::ButtonClick);
                    }
                    return actions;
                }

                // Cycle value: right-click or clicking `<` zone goes backward; otherwise forward
                let val_str = split_caption_and_value(entry.item, s, false).1;
                let val_w = HudBatch::text_width(atlas, &val_str);
                let min_pill = (ctrl.w * 0.68).max(68.0);
                let pill_w = (val_w + if *is_action { 16.0 } else { 28.0 }).clamp(min_pill.min(ctrl.w), ctrl.w);
                let pill_x = ctrl.x + ctrl.w - pill_w;
                let clicked_back = right || (!*is_action && pos.0 >= pill_x && pos.0 <= pill_x + 18.0);

                Self::apply_cycle(entry.item, s, clicked_back, framebuffer);
                actions.push(VoltAction::ButtonClick);
                actions.push(VoltAction::Changed);
                return actions;
            }
        }

        actions
    }

    fn apply_cycle(item: Item, s: &mut Settings, back: bool, framebuffer: [i32; 2]) {
        item.cycle(s, back, framebuffer);
        if item == Item::Vanilla(Opt::Preset) {
            apply_preset(s);
        }
    }

    pub fn wheel(
        &mut self,
        s: &mut Settings,
        cursor: (f32, f32),
        size: (f32, f32),
        framebuffer: [i32; 2],
        amount: f32,
        query: &str,
    ) -> Vec<VoltAction> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, _) = self.build_list_rows(s, &geom, query, self.scroll.amount);
        if amount != 0.0 {
            if let Some(VoltFocus::Item(item)) = self.focus
                && item_blocked(item, s).is_none()
            {
                if item.slider(s).is_some() {
                    let ctrl_w = rows
                        .iter()
                        .find_map(|r| match r {
                            LaidRow::Entry { entry, ctrl, .. } if entry.item == item => Some(ctrl.w),
                            _ => None,
                        })
                        .unwrap_or(120.0);
                    item.step_slider(s, amount > 0.0, ctrl_w - 12.0);
                    return vec![VoltAction::SliderDrag, VoltAction::Changed];
                }
                if !matches!(
                    item,
                    Item::Horizon(H::HdrCalibrate | H::HdrSettings | H::HdrKey)
                ) {
                    Self::apply_cycle(item, s, amount > 0.0, framebuffer);
                    return vec![VoltAction::ButtonClick, VoltAction::Changed];
                }
            }

            if cursor.1 >= geom.list_panel.y && cursor.1 < geom.list_panel.y + geom.list_panel.h {
                for row in &rows {
                    if let LaidRow::Entry {
                        entry,
                        rect,
                        is_slider,
                        ..
                    } = row
                        && rect.contains(cursor)
                        && !is_slider
                        && item_blocked(entry.item, s).is_none()
                        && !matches!(
                            entry.item,
                            Item::Horizon(H::HdrCalibrate | H::HdrSettings | H::HdrKey)
                        )
                    {
                        Self::apply_cycle(entry.item, s, amount > 0.0, framebuffer);
                        self.set_inspected(entry.item);
                        return vec![VoltAction::ButtonClick, VoltAction::Changed];
                    }
                }
            }
        }

        self.scroll(s, size, query, amount);
        Vec::new()
    }

    pub fn key_modifiers(
        &mut self,
        s: &mut Settings,
        key: winit::keyboard::KeyCode,
        text: Option<&str>,
        shift: bool,
        size: (f32, f32),
        framebuffer: [i32; 2],
        search: &mut ui::EditBox,
    ) -> (Vec<VoltAction>, bool) {
        use winit::keyboard::KeyCode as K;
        if key == K::Escape {
            return (vec![VoltAction::Done], true);
        }

        if (self.focus == Some(VoltFocus::Search) || search.focused)
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
        let (_, rows, content_h) = self.build_list_rows(s, &geom, &search.value, self.scroll.amount);
        let mut focusables: Vec<(VoltFocus, Rect, bool)> = Vec::new();
        focusables.push((VoltFocus::Search, geom.search_box, false));
        for (tab, rect) in geom.tabs {
            focusables.push((VoltFocus::Tab(tab), rect, false));
        }
        focusables.push((VoltFocus::Done, geom.done_btn, false));
        for r in &rows {
            if let LaidRow::Entry { entry, rect, .. } = r
                && item_blocked(entry.item, s).is_none()
            {
                focusables.push((VoltFocus::Item(entry.item), *rect, true));
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
            this.clamp_scroll(&geom, content_h);
        };

        if key == K::Tab {
            let cur = self
                .focus
                .and_then(|f| focusables.iter().position(|(id, ..)| *id == f));
            let next = match cur {
                Some(i) => {
                    if shift {
                        if i == 0 { focusables.len() - 1 } else { i - 1 }
                    } else {
                        (i + 1) % focusables.len()
                    }
                }
                None => {
                    if shift {
                        focusables.len() - 1
                    } else {
                        0
                    }
                }
            };
            let (id, rect, in_list) = focusables[next];
            self.focus = Some(id);
            search.focused = id == VoltFocus::Search;
            self.can_change_value = matches!(id, VoltFocus::Item(it) if it.slider(s).is_some());
            if let VoltFocus::Item(it) = id {
                self.set_inspected(it);
            }
            ensure_visible(self, rect, in_list);
            return (Vec::new(), true);
        }

        if matches!(key, K::ArrowUp | K::ArrowDown) {
            let down = key == K::ArrowDown;
            let cur = self
                .focus
                .and_then(|f| focusables.iter().position(|(id, ..)| *id == f));
            let next = match cur {
                Some(i) => {
                    if down {
                        (i + 1) % focusables.len()
                    } else if i == 0 {
                        focusables.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            let (id, rect, in_list) = focusables[next];
            self.focus = Some(id);
            search.focused = id == VoltFocus::Search;
            self.can_change_value = matches!(id, VoltFocus::Item(it) if it.slider(s).is_some());
            if let VoltFocus::Item(it) = id {
                self.set_inspected(it);
            }
            ensure_visible(self, rect, in_list);
            return (Vec::new(), true);
        }

        if matches!(key, K::ArrowLeft | K::ArrowRight) {
            let forward = key == K::ArrowRight;
            if let Some(VoltFocus::Item(item)) = self.focus
                && item_blocked(item, s).is_none()
            {
                if item.slider(s).is_some() {
                    let ctrl_w = rows
                        .iter()
                        .find_map(|r| match r {
                            LaidRow::Entry { entry, ctrl, .. } if entry.item == item => Some(ctrl.w),
                            _ => None,
                        })
                        .unwrap_or(120.0);
                    item.step_slider(s, forward, ctrl_w - 12.0);
                    return (vec![VoltAction::SliderDrag, VoltAction::Changed], true);
                }
                if !matches!(
                    item,
                    Item::Horizon(H::HdrCalibrate | H::HdrSettings | H::HdrKey)
                ) {
                    Self::apply_cycle(item, s, !forward, framebuffer);
                    return (vec![VoltAction::ButtonClick, VoltAction::Changed], true);
                }
            }
            return (Vec::new(), false);
        }

        if matches!(key, K::Enter | K::NumpadEnter | K::Space) {
            match self.focus {
                Some(VoltFocus::Done) => return (vec![VoltAction::ButtonClick, VoltAction::Done], true),
                Some(VoltFocus::Tab(tab)) => {
                    if !search.value.is_empty() {
                        search.set_value("");
                    }
                    self.select_tab(tab);
                    return (vec![VoltAction::ButtonClick], true);
                }
                Some(VoltFocus::Item(item)) if item_blocked(item, s).is_none() => {
                    if item == Item::Horizon(H::HdrCalibrate) {
                        if matches!(graphics_ui::setup(s), graphics_ui::Setup::Changed) {
                            s.horizon.hdr_peak = 0;
                            s.horizon.hdr_paper_white = 0;
                            return (vec![VoltAction::ButtonClick, VoltAction::Changed], true);
                        }
                        return (vec![VoltAction::ButtonClick, VoltAction::Calibrate], true);
                    }
                    if item == Item::Horizon(H::HdrKey) {
                        return (vec![VoltAction::ButtonClick, VoltAction::BindKey("hdr_toggle")], true);
                    }
                    if item == Item::Horizon(H::HdrSettings) {
                        self.select_tab(VoltTab::Hdr);
                        return (vec![VoltAction::ButtonClick], true);
                    }
                    if item.slider(s).is_some() {
                        self.can_change_value = !self.can_change_value;
                        return (vec![VoltAction::ButtonClick], true);
                    }
                    Self::apply_cycle(item, s, shift, framebuffer);
                    return (vec![VoltAction::ButtonClick, VoltAction::Changed], true);
                }
                _ => {}
            }
        }

        (Vec::new(), false)
    }
}

pub(crate) fn cost_color(cost: Cost) -> u32 {
    match cost {
        Cost::None => rgba(182, 184, 198, 255),
        Cost::Low => rgba(138, 218, 138, 255),
        Cost::Medium => rgba(242, 204, 96, 255),
        Cost::High => rgba(242, 112, 96, 255),
        Cost::Saves => rgba(96, 235, 138, 255),
    }
}

pub(crate) fn with_alpha(color: u32, factor: f32) -> u32 {
    let a = ((color >> 24) & 0xff) as f32;
    let na = (a * factor.clamp(0.0, 1.0)).round() as u32;
    (color & 0x00ff_ffff) | (na << 24)
}

pub(crate) fn lerp_color(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |shift: u32| {
        let ca = ((a >> shift) & 0xff) as f32;
        let cb = ((b >> shift) & 0xff) as f32;
        (ca + (cb - ca) * t).round() as u32
    };
    (ch(24) << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

pub(crate) fn draw_border(batch: &mut HudBatch, atlas: &HudAtlas, x: f32, y: f32, w: f32, h: f32, color: u32) {
    batch.fill(atlas, x, y, w, 1.0, color);
    batch.fill(atlas, x, y + h - 1.0, w, 1.0, color);
    batch.fill(atlas, x, y + 1.0, 1.0, (h - 2.0).max(0.0), color);
    batch.fill(atlas, x + w - 1.0, y + 1.0, 1.0, (h - 2.0).max(0.0), color);
}

pub(crate) fn fit_with_ellipsis(atlas: &HudAtlas, text: &str, max_width: f32) -> String {
    if HudBatch::text_width(atlas, text) <= max_width {
        return text.to_owned();
    }
    let dots_w = HudBatch::text_width(atlas, "...");
    let fit = atlas.font.fit(text, (max_width - dots_w).max(0.0));
    let mut out: String = text.chars().take(fit).collect();
    out.push_str("...");
    out
}

#[cfg(test)]
impl VoltVideoUi {
    pub(crate) fn test_geometry(w: f32, h: f32) -> ([f32; 4], [([VoltTab; 5], [[f32; 4]; 5])], [f32; 4], [f32; 4]) {
        let g = Self::geometry(w, h);
        let tabs = g.tabs.map(|(t, _)| t);
        let rects = g.tabs.map(|(_, r)| r.array());
        (g.search_box.array(), [(tabs, rects)], g.done_btn.array(), g.footer_card.array())
    }

    pub(crate) fn test_row_rect(&self, s: &Settings, size: (f32, f32), query: &str, item: Item) -> Option<([f32; 4], [f32; 4])> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, _) = self.build_list_rows(s, &geom, query, self.scroll.amount);
        rows.into_iter().find_map(|r| match r {
            LaidRow::Entry { entry, rect, ctrl, .. } if entry.item == item => {
                Some((rect.array(), ctrl.array()))
            }
            _ => None,
        })
    }

    pub(crate) fn test_list_bounds(&self, s: &Settings, size: (f32, f32), query: &str) -> (f32, f32, f32) {
        let geom = Self::geometry(size.0, size.1);
        let (_, _, content_h) = self.build_list_rows(s, &geom, query, self.scroll.amount);
        (geom.list_panel.y, geom.list_panel.y + geom.list_panel.h, content_h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::Preset;

    fn atlas() -> HudAtlas {
        HudAtlas::build(std::path::Path::new("/nonexistent/horizon-test-assets")).unwrap()
    }

    #[test]
    fn draws_all_tabs_and_screen_sizes_with_animations_and_targets() {
        let atlas = atlas();
        let s = Settings::default();
        let mut ui = VoltVideoUi::default();
        let mut search = ui::EditBox::new(0.0, 0.0, 100.0, 16.0);

        for size in [(320.0, 240.0), (427.0, 240.0), (640.0, 360.0)] {
            for tab in VoltTab::ALL {
                ui.select_tab(tab);
                for frame in 0..3 {
                    let mut batch = HudBatch::new([size.0 as u32, size.1 as u32]);
                    let cursor = (size.0 * 0.6, size.1 * 0.4 + frame as f32);
                    ui.draw(&s, &mut batch, &atlas, cursor, true, &mut search, None);
                    assert!(!batch.vertices.is_empty(), "{tab:?} at {size:?}");
                }
            }
        }

        // Verify widget/focus targets were emitted for Android touch & controller navigation
        crate::widgets::publish_frame();
        let targets = crate::widgets::last_targets();
        assert!(targets.iter().any(|t| t.kind == crate::widgets::Kind::Field));
        assert!(targets.iter().any(|t| matches!(t.kind, crate::widgets::Kind::Tab(_))));
    }

    #[test]
    fn tabs_clear_search_and_done_closes() {
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = VoltVideoUi::default();
        let mut search = ui::EditBox::new(0.0, 0.0, 100.0, 16.0);
        let size = (480.0, 270.0);
        let (sb, [(tabs, rects)], done, _) = VoltVideoUi::test_geometry(size.0, size.1);

        // Click Search box
        let actions = ui.click(
            &mut s,
            &atlas,
            (sb[0] + 4.0, sb[1] + 4.0),
            size,
            [960, 540],
            false,
            false,
            &mut search,
        );
        assert_eq!(actions, vec![VoltAction::ButtonClick]);
        assert!(search.focused);
        search.insert_text("render");

        // Click Quality tab: clears search and switches tab
        let q_rect = rects[1];
        assert_eq!(tabs[1], VoltTab::Quality);
        let actions = ui.click(
            &mut s,
            &atlas,
            (q_rect[0] + 4.0, q_rect[1] + 4.0),
            size,
            [960, 540],
            false,
            false,
            &mut search,
        );
        assert_eq!(actions, vec![VoltAction::ButtonClick]);
        assert_eq!(ui.tab, VoltTab::Quality);
        assert!(search.value.is_empty());

        // Click Done
        let actions = ui.click(
            &mut s,
            &atlas,
            (done[0] + 4.0, done[1] + 4.0),
            size,
            [960, 540],
            false,
            false,
            &mut search,
        );
        assert_eq!(actions, vec![VoltAction::ButtonClick, VoltAction::Done]);
    }

    #[test]
    fn preset_cycle_and_slider_drag_and_keyboard_navigation() {
        use winit::keyboard::KeyCode as K;
        let atlas = atlas();
        let mut s = Settings::default();
        let mut ui = VoltVideoUi::default();
        let mut search = ui::EditBox::new(0.0, 0.0, 100.0, 16.0);
        let size = (480.0, 320.0);

        // Cycle H::Preset forward (Custom/Balanced -> Quality -> Ultra) and back via `<`
        let (p_rect, p_ctrl) = ui
            .test_row_rect(&s, size, "", Item::Horizon(H::Preset))
            .unwrap();
        let right_x = p_ctrl[0] + p_ctrl[2] - 6.0;
        let y = p_rect[1] + 8.0;
        ui.click(&mut s, &atlas, right_x, y, size, [960, 640], false, false);
        let after_forward = s.horizon.preset;
        assert_ne!(after_forward, Preset::Custom);

        // Dragging Render Distance marks Preset as Custom
        let (rd_rect, rd_ctrl) = ui
            .test_row_rect(&s, size, "", Item::Vanilla(Opt::RenderDistance))
            .unwrap();
        let rd_y = rd_rect[1] + 8.0;
        let a = ui.click(
            &mut s,
            &atlas,
            rd_ctrl[0] + rd_ctrl[2] - 10.0,
            rd_y,
            size,
            [960, 640],
            false,
            false,
        );
        assert_eq!(a, vec![VoltAction::SliderDrag]);
        assert!(ui.release());
        assert_eq!(s.horizon.preset, Preset::Custom);

        // Keyboard Tab + ArrowRight + Escape
        ui.focus = Some(VoltFocus::Item(Item::Vanilla(Opt::Fov)));
        let fov_before = s.fov;
        let (acts, handled) = ui.key_modifiers(&mut s, K::ArrowRight, None, false, size, [960, 640], &mut search);
        assert!(handled);
        assert_eq!(acts, vec![VoltAction::SliderDrag, VoltAction::Changed]);
        assert!(s.fov > fov_before);

        let (acts, handled) = ui.key_modifiers(&mut s, K::Escape, None, false, size, [960, 640], &mut search);
        assert!(handled);
        assert_eq!(acts, vec![VoltAction::Done]);
    }

    #[test]
    fn scrolling_clamps_to_list_content() {
        let s = Settings::default();
        let mut ui = VoltVideoUi::default();
        ui.select_tab(VoltTab::Quality);
        let size = (480.0, 240.0);
        let (top, bottom, content) = ui.test_list_bounds(&s, size, "");
        assert!(content > bottom - top);
        ui.scroll(&s, size, "", -1000.0);
        assert!((ui.scroll.amount - (content - (bottom - top))).abs() < 0.01);
        ui.scroll(&s, size, "", 1000.0);
        assert_eq!(ui.scroll.amount, 0.0);
    }
}
