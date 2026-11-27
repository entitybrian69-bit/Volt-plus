//! Volt Graphics UIs for Mods (`Page::Mods`) and Volt+ Settings (`Page::Horizon`),
//! sharing the exact visual language, animations, search, and interaction model
//! of `volt_video_ui`.
use std::time::Instant;

use crate::{
    extras::Item as ExtraItem,
    graphics_style::Cost,
    hud::{HudAtlas, HudBatch, rgba},
    lang,
    options_ui::Page,
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

#[derive(Clone, Copy, Debug)]
struct Palette {
    scrim: u32,
    panel_bg: u32,
    bar_bg: u32,
    border_subtle: u32,
    border_strong: u32,
    accent: u32,
    accent_hi: u32,
    accent_edge: u32,
    text_primary: u32,
    text_muted: u32,
    text_dim: u32,
}

fn palette(in_world: bool, ui_alpha: f32) -> Palette {
    Palette {
        scrim: with_alpha(rgba(18, 17, 28, if in_world { 136 } else { 168 }), ui_alpha),
        panel_bg: with_alpha(rgba(34, 32, 48, 228), ui_alpha),
        bar_bg: with_alpha(rgba(42, 40, 60, 234), ui_alpha),
        border_subtle: with_alpha(rgba(255, 255, 255, 58), ui_alpha),
        border_strong: with_alpha(rgba(140, 118, 248, 235), ui_alpha),
        accent: with_alpha(rgba(116, 92, 235, 255), ui_alpha),
        accent_hi: with_alpha(rgba(140, 118, 248, 255), ui_alpha),
        accent_edge: with_alpha(rgba(180, 164, 255, 230), ui_alpha),
        text_primary: with_alpha(rgba(248, 247, 255, 255), ui_alpha),
        text_muted: with_alpha(rgba(188, 186, 206, 255), ui_alpha),
        text_dim: with_alpha(rgba(136, 134, 154, 255), ui_alpha),
    }
}

// ---------------------------------------------------------------------------
// VoltModsUi (Page::Mods)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ModsTab {
    #[default]
    All,
    Enabled,
    Disabled,
    Configurable,
}

impl ModsTab {
    pub const ALL: [Self; 4] = [
        Self::All,
        Self::Enabled,
        Self::Disabled,
        Self::Configurable,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All Mods",
            Self::Enabled => "Enabled",
            Self::Disabled => "Disabled",
            Self::Configurable => "Configurable",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::All => 0,
            Self::Enabled => 1,
            Self::Disabled => 2,
            Self::Configurable => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltModsAction {
    ButtonClick,
    Changed,
    Toggle(usize),
    Settings(usize),
    OpenFolder,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModsFocus {
    Search,
    Tab(ModsTab),
    Mod(usize),
    ModSettings(usize),
    OpenFolder,
    Done,
}

#[derive(Clone, Copy, Debug)]
struct ModsGeometry {
    side_x: f32,
    brand_y: f32,
    search_box: Rect,
    tabs: [(ModsTab, Rect); 4],
    folder_btn: Rect,
    done_btn: Rect,
    header_bar: Rect,
    list_panel: Rect,
    list_clip: Rect,
    scrollbar_x: f32,
    footer_card: Rect,
}

#[derive(Clone, Debug)]
struct ModRow {
    index: usize,
    rect: Rect,
    toggle_rect: Rect,
    settings_rect: Option<Rect>,
    dead: bool,
}

pub struct VoltModsUi {
    pub tab: ModsTab,
    pub scroll: ui::Scroll,
    pub focus: Option<ModsFocus>,
    pub inspected: Option<usize>,
    opened_at: Option<Instant>,
    tab_changed_at: Instant,
    tab_dir: f32,
    visual_tab_y: Option<f32>,
    visual_scroll: Option<f32>,
    last_frame: Option<Instant>,
    tab_hover: [f32; 4],
    row_hover: Vec<(usize, f32)>,
    done_hover: f32,
    folder_hover: f32,
}

impl Default for VoltModsUi {
    fn default() -> Self {
        Self {
            tab: ModsTab::All,
            scroll: ui::Scroll::default(),
            focus: None,
            inspected: None,
            opened_at: None,
            tab_changed_at: Instant::now(),
            tab_dir: 1.0,
            visual_tab_y: None,
            visual_scroll: None,
            last_frame: None,
            tab_hover: [0.0; 4],
            row_hover: Vec::new(),
            done_hover: 0.0,
            folder_hover: 0.0,
        }
    }
}

impl VoltModsUi {
    pub fn reset(&mut self) {
        self.scroll = ui::Scroll::default();
        self.focus = None;
        self.inspected = None;
        self.opened_at = Some(Instant::now());
        self.tab_changed_at = Instant::now();
        self.visual_tab_y = None;
        self.visual_scroll = None;
        self.last_frame = Some(Instant::now());
        self.row_hover.clear();
    }

    pub fn reset_scroll(&mut self) {
        self.scroll.amount = 0.0;
        self.visual_scroll = None;
    }

    pub fn release(&mut self) -> bool {
        self.scroll.release();
        false
    }

    pub fn select_tab(&mut self, next: ModsTab) {
        if self.tab != next {
            self.tab_dir = if next.index() > self.tab.index() { 1.0 } else { -1.0 };
            self.tab = next;
            self.tab_changed_at = Instant::now();
            self.scroll.amount = 0.0;
            self.visual_scroll = None;
            self.inspected = None;
        }
    }

    fn geometry(w: f32, h: f32) -> ModsGeometry {
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
                ModsTab::ALL[i],
                Rect {
                    x: side_x,
                    y: tabs_top + i as f32 * tab_stride,
                    w: side_w,
                    h: tab_h,
                },
            )
        });

        let done_h = 20.0;
        let folder_h = 17.0;
        let done_y = (h - pad_bottom - done_h).max(tabs_top + 4.0 * tab_stride + folder_h + 8.0);
        let done_btn = Rect {
            x: side_x,
            y: done_y,
            w: side_w,
            h: done_h,
        };
        let folder_btn = Rect {
            x: side_x,
            y: done_y - folder_h - 4.0,
            w: side_w,
            h: folder_h,
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

        ModsGeometry {
            side_x,
            brand_y,
            search_box,
            tabs,
            folder_btn,
            done_btn,
            header_bar,
            list_panel,
            list_clip,
            scrollbar_x: list_panel.x + list_panel.w - 6.0,
            footer_card,
        }
    }

    fn mod_has_settings(s: &Settings, i: usize) -> bool {
        let installed = &s.mods.installed[i];
        installed.has_settings
            || installed
                .native
                .as_ref()
                .is_some_and(|p| !p.keys.is_empty() || !p.buttons.is_empty())
    }

    fn filtered_indices(&self, s: &Settings, query: &str) -> (String, Vec<usize>) {
        let q = query.trim().to_lowercase();
        let mut out = Vec::new();
        for (i, m) in s.mods.installed.iter().enumerate() {
            if !q.is_empty() {
                let hay = format!("{} {} {}", m.title, m.id, m.description).to_lowercase();
                if hay.contains(&q) {
                    out.push(i);
                }
                continue;
            }
            let keep = match self.tab {
                ModsTab::All => true,
                ModsTab::Enabled => s.mods.is_on(i),
                ModsTab::Disabled => !s.mods.is_on(i),
                ModsTab::Configurable => Self::mod_has_settings(s, i),
            };
            if keep {
                out.push(i);
            }
        }
        let header = if q.is_empty() {
            format!("Mods / {}", self.tab.label())
        } else {
            format!("Mods / Search: {}", query.trim())
        };
        (header, out)
    }

    fn build_rows(&self, s: &Settings, geom: &ModsGeometry, query: &str, scroll_y: f32) -> (String, Vec<ModRow>, f32) {
        let (header, indices) = self.filtered_indices(s, query);
        let row_x = geom.list_panel.x + 6.0;
        let row_w = (geom.list_panel.w - 18.0).max(96.0);
        let mut y = 6.0;
        let mut out = Vec::with_capacity(indices.len());
        for idx in indices {
            let installed = &s.mods.installed[idx];
            let dead = installed.dev || installed.id.is_empty();
            let has_settings = Self::mod_has_settings(s, idx);
            let ry = geom.list_panel.y + y - scroll_y;
            let toggle_w = 56.0;
            let toggle_rect = Rect {
                x: row_x + row_w - toggle_w - 4.0,
                y: ry + 2.0,
                w: toggle_w,
                h: 16.0,
            };
            let settings_rect = has_settings.then(|| Rect {
                x: toggle_rect.x - 56.0,
                y: ry + 2.0,
                w: 52.0,
                h: 16.0,
            });
            out.push(ModRow {
                index: idx,
                rect: Rect {
                    x: row_x,
                    y: ry,
                    w: row_w,
                    h: ROW_HEIGHT,
                },
                toggle_rect,
                settings_rect,
                dead,
            });
            y += ROW_STRIDE;
        }
        (header, out, (y + 6.0).max(1.0))
    }

    fn smooth_hover(cache: &mut Vec<(usize, f32)>, key: usize, target: f32, dt: f32) -> f32 {
        if let Some((_, v)) = cache.iter_mut().find(|(k, _)| *k == key) {
            if dt > 0.0 {
                let k = 1.0 - (-18.0 * dt).exp();
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

        let (w, h) = (batch.width, batch.height);
        let geom = Self::geometry(w, h);
        let (_, _, content_h) = self.build_rows(s, &geom, &search.value, self.scroll.amount);
        let lp_top = geom.list_panel.y;
        let lp_bot = geom.list_panel.y + geom.list_panel.h;
        self.scroll.clamp(lp_top, lp_bot, content_h);
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
        let (header_title, rows, _) = self.build_rows(s, &geom, &search.value, scroll_draw);

        let pal = palette(in_world, ui_alpha);
        batch.fill(atlas, 0.0, 0.0, w, h, pal.scrim);

        // Brand header
        batch.text(atlas, "Volt", geom.side_x + side_dx, geom.brand_y + 1.0, pal.text_primary, true);
        let vw = HudBatch::text_width(atlas, "Volt");
        batch.text(atlas, " Mods", geom.side_x + side_dx + vw, geom.brand_y + 1.0, pal.accent_hi, true);
        batch.text(
            atlas,
            &format!("{} installed", s.mods.installed.len()),
            geom.side_x + side_dx,
            geom.brand_y + 12.0,
            pal.text_dim,
            true,
        );

        // Search box
        let sb = geom.search_box;
        let sb_x = sb.x + side_dx;
        let sb_focused = self.focus == Some(ModsFocus::Search) || search.focused;
        let sb_hover = sb.contains(cursor) || sb_focused;
        batch.fill(atlas, sb_x, sb.y, sb.w, sb.h, pal.panel_bg);
        draw_border(
            batch,
            atlas,
            sb_x,
            sb.y,
            sb.w,
            sb.h,
            if sb_focused { pal.border_strong } else if sb_hover { with_alpha(pal.border_strong, 0.65) } else { pal.border_subtle },
        );
        crate::widgets::target_as(sb.array(), crate::widgets::Kind::Field);
        search.bordered = false;
        search.x = sb.x + 5.0;
        search.y = sb.y + 4.0;
        search.width = (sb.w - 10.0).max(16.0);
        search.height = 10.0;
        search.text_color = pal.text_primary;
        search.message = "Search mods".to_owned();
        if search.value.is_empty() && !search.focused {
            batch.text(atlas, "Search mods", sb_x + 5.0, sb.y + 4.0, pal.text_dim, false);
        }
        let saved_sx = search.x;
        search.x = sb_x + 5.0;
        search.draw(batch, atlas, None);
        search.x = saved_sx;
        crate::widgets::widget_cursor(sb_hover, true);

        // Sidebar tabs + gliding outline
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
            let focused = self.focus == Some(ModsFocus::Tab(*tab));
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
                pal.text_primary
            } else {
                lerp_color(pal.text_muted, pal.text_primary, self.tab_hover[i] * 0.7)
            };
            batch.text(atlas, tab.label(), rx + 7.0, rect.y + 4.0, col, true);
            crate::widgets::record_as(tab.label(), rect.array(), true, crate::widgets::Kind::Tab(selected));
            crate::widgets::widget_cursor(hovered, true);
        }
        if !searching {
            let rx = active_tab_rect.x + side_dx;
            draw_border(batch, atlas, rx, tab_y, active_tab_rect.w, active_tab_rect.h, pal.border_strong);
            batch.fill(atlas, rx + 1.0, tab_y + 3.0, 2.0, active_tab_rect.h - 6.0, pal.accent_hi);
        }

        // Open Mods Folder secondary button
        let fb = geom.folder_btn;
        let fb_hover = fb.contains(cursor) || self.focus == Some(ModsFocus::OpenFolder);
        if dt > 0.0 {
            let k = 1.0 - (-18.0 * dt).exp();
            self.folder_hover += ((if fb_hover { 1.0 } else { 0.0 }) - self.folder_hover) * k;
        } else {
            self.folder_hover = if fb_hover { 1.0 } else { 0.0 };
        }
        let fb_x = fb.x + side_dx;
        batch.fill(
            atlas,
            fb_x,
            fb.y,
            fb.w,
            fb.h,
            lerp_color(pal.bar_bg, with_alpha(rgba(60, 56, 86, 235), ui_alpha), self.folder_hover),
        );
        draw_border(
            batch,
            atlas,
            fb_x,
            fb.y,
            fb.w,
            fb.h,
            if fb_hover { pal.border_strong } else { pal.border_subtle },
        );
        let folder_label = lang::get("horizon.mods.open_folder");
        ui::centered_text(
            batch,
            atlas,
            &Text::literal(folder_label.clone()),
            fb_x + 2.0,
            fb.y + 5.0,
            fb.w - 4.0,
            pal.text_primary,
        );
        crate::widgets::record(&folder_label, fb.array(), true);
        crate::widgets::widget_cursor(fb_hover, true);

        // Purple Done button
        let done = geom.done_btn;
        let done_hovered = done.contains(cursor) || self.focus == Some(ModsFocus::Done);
        if dt > 0.0 {
            let k = 1.0 - (-18.0 * dt).exp();
            self.done_hover += ((if done_hovered { 1.0 } else { 0.0 }) - self.done_hover) * k;
        } else {
            self.done_hover = if done_hovered { 1.0 } else { 0.0 };
        }
        let done_x = done.x + side_dx;
        batch.fill(atlas, done_x, done.y, done.w, done.h, lerp_color(pal.accent, pal.accent_hi, self.done_hover));
        batch.fill(atlas, done_x, done.y, done.w, 1.0, pal.accent_edge);
        if self.focus == Some(ModsFocus::Done) {
            draw_border(batch, atlas, done_x, done.y, done.w, done.h, pal.border_strong);
        }
        let done_text = Text::tr("gui.done", &[]);
        ui::centered_text(batch, atlas, &done_text, done_x + 2.0, done.y + 6.0, done.w - 4.0, pal.text_primary);
        crate::widgets::record(&done_text.plain(), done.array(), true);
        crate::widgets::widget_cursor(done_hovered, true);

        // Header bar
        let hdr = geom.header_bar;
        let hdr_y = hdr.y + main_dy;
        batch.fill(atlas, hdr.x, hdr_y, hdr.w, hdr.h, pal.bar_bg);
        draw_border(batch, atlas, hdr.x, hdr_y, hdr.w, hdr.h, pal.border_subtle);
        batch.text(
            atlas,
            &header_title,
            hdr.x + 8.0,
            hdr_y + 5.0,
            with_alpha(pal.text_primary, ui_alpha * (0.35 + 0.65 * tab_p)),
            true,
        );

        // List panel
        let lp = geom.list_panel;
        let lp_y = lp.y + main_dy;
        batch.fill(atlas, lp.x, lp_y, lp.w, lp.h, pal.panel_bg);
        draw_border(batch, atlas, lp.x, lp_y, lp.w, lp.h, pal.border_subtle);

        let content_slide_y = main_dy + (1.0 - tab_p) * self.tab_dir * 8.0;
        let content_alpha = ui_alpha * (0.25 + 0.75 * tab_p);
        let clip_x0 = geom.list_clip.x;
        let clip_y0 = geom.list_clip.y + main_dy;
        let clip_x1 = clip_x0 + geom.list_clip.w;
        let clip_y1 = clip_y0 + geom.list_clip.h;
        let list_verts = batch.vertices.len();
        let target_mark = crate::widgets::targets_mark();
        let in_list_y = cursor.1 >= lp.y && cursor.1 < lp.y + lp.h;

        let mut hovered_idx: Option<usize> = None;
        let mut focused_idx: Option<usize> = None;
        let mut first_idx: Option<usize> = None;

        if rows.is_empty() {
            let empty_msg = if s.mods.installed.is_empty() {
                lang::get("horizon.mods.empty")
            } else {
                "No mods match this filter.".to_owned()
            };
            ui::centered_text(
                batch,
                atlas,
                &Text::literal(empty_msg),
                lp.x + 8.0,
                lp_y + lp.h * 0.45,
                lp.w - 16.0,
                with_alpha(pal.text_muted, content_alpha),
            );
        }

        for row in &rows {
            if first_idx.is_none() {
                first_idx = Some(row.index);
            }
            let dy = row.rect.y + content_slide_y;
            let visible = dy + row.rect.h >= clip_y0 - 4.0 && dy <= clip_y1 + 4.0;
            let is_hovered = in_list_y && row.rect.contains(cursor);
            let is_focused = matches!(
                self.focus,
                Some(ModsFocus::Mod(i)) | Some(ModsFocus::ModSettings(i)) if i == row.index
            );
            if is_hovered {
                hovered_idx = Some(row.index);
            }
            if is_focused {
                focused_idx = Some(row.index);
            }
            let hl = Self::smooth_hover(
                &mut self.row_hover,
                row.index,
                if is_hovered || is_focused || self.inspected == Some(row.index) { 1.0 } else { 0.0 },
                dt,
            );
            if !visible {
                continue;
            }
            let installed = &s.mods.installed[row.index];
            let idle_fill = with_alpha(rgba(255, 255, 255, 20), content_alpha);
            let active_fill = with_alpha(rgba(255, 255, 255, 44), content_alpha);
            batch.fill(atlas, row.rect.x, dy, row.rect.w, row.rect.h, lerp_color(idle_fill, active_fill, hl));
            if is_focused || self.inspected == Some(row.index) {
                batch.fill(atlas, row.rect.x, dy + 3.0, 2.0, row.rect.h - 6.0, with_alpha(pal.accent_hi, content_alpha));
            }

            let title_end = row.settings_rect.map_or(row.toggle_rect.x, |r| r.x) - 8.0;
            let title_max = (title_end - (row.rect.x + 7.0)).max(24.0);
            let title_str = fit_with_ellipsis(atlas, &installed.title, title_max);
            batch.text(
                atlas,
                &title_str,
                row.rect.x + 7.0,
                dy + 6.0,
                with_alpha(if row.dead { pal.text_dim } else { pal.text_primary }, content_alpha),
                true,
            );

            // Settings button (when configurable)
            if let Some(sr) = row.settings_rect {
                let s_hover = in_list_y && sr.contains(cursor);
                let s_focus = self.focus == Some(ModsFocus::ModSettings(row.index));
                batch.fill(
                    atlas,
                    sr.x,
                    dy + 2.0,
                    sr.w,
                    sr.h,
                    with_alpha(rgba(28, 26, 42, if s_hover || s_focus { 235 } else { 190 }), content_alpha),
                );
                draw_border(
                    batch,
                    atlas,
                    sr.x,
                    dy + 2.0,
                    sr.w,
                    sr.h,
                    if s_hover || s_focus { pal.border_strong } else { pal.border_subtle },
                );
                ui::centered_text(
                    batch,
                    atlas,
                    &Text::tr("horizon.mods.settings", &[]),
                    sr.x + 2.0,
                    dy + 6.0,
                    sr.w - 4.0,
                    with_alpha(pal.text_primary, content_alpha),
                );
                crate::widgets::record(&format!("{}: settings", installed.title), sr.array(), true);
                crate::widgets::widget_cursor(s_hover, true);
            }

            // On/Off pill
            let tr = row.toggle_rect;
            let on = s.mods.is_on(row.index);
            let t_hover = in_list_y && tr.contains(cursor);
            let t_focus = self.focus == Some(ModsFocus::Mod(row.index));
            let pill_bg = if row.dead {
                with_alpha(rgba(28, 27, 36, 180), content_alpha)
            } else if on {
                with_alpha(rgba(110, 88, 228, if t_hover || t_focus { 245 } else { 205 }), content_alpha)
            } else {
                with_alpha(rgba(46, 43, 68, if t_hover || t_focus { 225 } else { 180 }), content_alpha)
            };
            batch.fill(atlas, tr.x, dy + 2.0, tr.w, tr.h, pill_bg);
            draw_border(
                batch,
                atlas,
                tr.x,
                dy + 2.0,
                tr.w,
                tr.h,
                if !row.dead && (t_hover || t_focus || on) {
                    pal.border_strong
                } else {
                    pal.border_subtle
                },
            );
            let state_text = if installed.problem.as_deref() == Some(crate::mod_list::NOT_BUILT) {
                "Unbuilt"
            } else if installed.problem.is_some() {
                "Stopped"
            } else if installed.dev {
                "Dev"
            } else if on {
                "ON"
            } else {
                "OFF"
            };
            ui::centered_text(
                batch,
                atlas,
                &Text::literal(state_text),
                tr.x + 2.0,
                dy + 6.0,
                tr.w - 4.0,
                with_alpha(if row.dead { pal.text_dim } else { pal.text_primary }, content_alpha),
            );
            crate::widgets::record(&installed.title, tr.array(), !row.dead);
            crate::widgets::widget_cursor(t_hover && !row.dead, !row.dead);
        }

        if content_h > lp.h + 0.5 {
            let max_s = (content_h - lp.h).max(1.0);
            let thumb_h = ((lp.h / content_h) * (lp.h - 8.0)).clamp(16.0, lp.h - 8.0);
            let ratio = (scroll_draw / max_s).clamp(0.0, 1.0);
            let thumb_y = lp_y + 4.0 + (lp.h - 8.0 - thumb_h) * ratio;
            batch.fill(atlas, geom.scrollbar_x, lp_y + 4.0, 2.0, lp.h - 8.0, with_alpha(rgba(255, 255, 255, 18), ui_alpha));
            batch.fill(atlas, geom.scrollbar_x, thumb_y, 2.0, thumb_h, with_alpha(pal.accent_hi, ui_alpha * 0.85));
        }

        batch.clip_since(list_verts, clip_x0, clip_y0, clip_x1, clip_y1);
        crate::widgets::clip_targets(target_mark, lp_y, lp_y + lp.h);

        // Footer card
        let fc = geom.footer_card;
        let fc_y = fc.y + main_dy;
        batch.fill(atlas, fc.x, fc_y, fc.w, fc.h, pal.bar_bg);
        draw_border(batch, atlas, fc.x, fc_y, fc.w, fc.h, pal.border_subtle);
        if let Some(idx) = hovered_idx.or(focused_idx).or(self.inspected).or(first_idx)
            && let Some(m) = s.mods.installed.get(idx)
        {
            let badge = if m.problem.is_some() {
                ("ERR", rgba(242, 112, 96, 255))
            } else if m.dev {
                ("DEV", rgba(242, 204, 96, 255))
            } else if s.mods.is_on(idx) {
                ("ACTIVE", rgba(96, 235, 138, 255))
            } else {
                ("OFF", rgba(162, 162, 178, 255))
            };
            let badge_txt = format!("[{}]", badge.0);
            let bw = HudBatch::text_width(atlas, &badge_txt);
            batch.text(atlas, &badge_txt, fc.x + fc.w - bw - 8.0, fc_y + 5.0, with_alpha(badge.1, ui_alpha), true);
            let title_fit = fit_with_ellipsis(atlas, &m.title, (fc.w - bw - 22.0).max(40.0));
            batch.text(atlas, &title_fit, fc.x + 8.0, fc_y + 5.0, pal.text_primary, true);
            let desc = m
                .problem
                .as_deref()
                .filter(|p| !p.is_empty())
                .or_else(|| (!m.description.is_empty()).then_some(m.description.as_str()))
                .unwrap_or("Installed Volt+ client modification.");
            let desc_fit = fit_with_ellipsis(atlas, desc, (fc.w - 16.0).max(40.0));
            batch.text(atlas, &desc_fit, fc.x + 8.0, fc_y + 17.0, pal.text_muted, true);
        } else {
            batch.text(atlas, "Volt+ Mods", fc.x + 8.0, fc_y + 5.0, pal.text_primary, true);
            batch.text(
                atlas,
                "Open the Mods folder to add or manage Volt+ mods.",
                fc.x + 8.0,
                fc_y + 17.0,
                pal.text_muted,
                true,
            );
        }
    }

    pub fn click(
        &mut self,
        s: &mut Settings,
        atlas: &HudAtlas,
        pos: (f32, f32),
        size: (f32, f32),
        drag: bool,
        search: &mut ui::EditBox,
    ) -> Vec<VoltModsAction> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, content_h) = self.build_rows(s, &geom, &search.value, self.scroll.amount);
        let lp_top = geom.list_panel.y;
        let lp_bot = geom.list_panel.y + geom.list_panel.h;
        if drag {
            if self.scroll.held() {
                self.scroll.drag(pos.1, lp_top, lp_bot, content_h);
                self.visual_scroll = Some(self.scroll.amount);
            }
            return Vec::new();
        }
        if geom.search_box.contains(pos) {
            self.focus = Some(ModsFocus::Search);
            search.focused = true;
            search.bordered = false;
            search.x = geom.search_box.x + 5.0;
            search.y = geom.search_box.y + 4.0;
            search.width = (geom.search_box.w - 10.0).max(16.0);
            search.click(atlas, pos.0, false, false);
            return Vec::new();
        }
        search.focused = false;
        for (tab, rect) in geom.tabs {
            if rect.contains(pos) {
                self.select_tab(tab);
                self.focus = Some(ModsFocus::Tab(tab));
                if !search.value.is_empty() {
                    search.set_value("");
                }
                return vec![VoltModsAction::ButtonClick];
            }
        }
        if geom.folder_btn.contains(pos) {
            self.focus = Some(ModsFocus::OpenFolder);
            return vec![VoltModsAction::ButtonClick, VoltModsAction::OpenFolder];
        }
        if geom.done_btn.contains(pos) {
            self.focus = Some(ModsFocus::Done);
            return vec![VoltModsAction::ButtonClick, VoltModsAction::Done];
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
        if geom.list_panel.contains(pos) {
            for row in rows {
                if !row.rect.contains(pos) {
                    continue;
                }
                self.inspected = Some(row.index);
                if let Some(sr) = row.settings_rect
                    && sr.contains(pos)
                {
                    self.focus = Some(ModsFocus::ModSettings(row.index));
                    return vec![VoltModsAction::ButtonClick, VoltModsAction::Settings(row.index)];
                }
                self.focus = Some(ModsFocus::Mod(row.index));
                if !row.dead {
                    s.mods.toggle(row.index);
                    return vec![
                        VoltModsAction::ButtonClick,
                        VoltModsAction::Toggle(row.index),
                        VoltModsAction::Changed,
                    ];
                }
                return Vec::new();
            }
        }
        Vec::new()
    }

    pub fn scroll(&mut self, s: &Settings, size: (f32, f32), query: &str, amount: f32) {
        let geom = Self::geometry(size.0, size.1);
        let (_, _, content_h) = self.build_rows(s, &geom, query, self.scroll.amount);
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
    ) -> (Vec<VoltModsAction>, bool) {
        use winit::keyboard::KeyCode as K;
        if key == K::Escape {
            return (vec![VoltModsAction::Done], true);
        }
        if (self.focus == Some(ModsFocus::Search) || search.focused)
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
        if matches!(key, K::Enter | K::NumpadEnter | K::Space) {
            match self.focus {
                Some(ModsFocus::Done) => return (vec![VoltModsAction::ButtonClick, VoltModsAction::Done], true),
                Some(ModsFocus::OpenFolder) => return (vec![VoltModsAction::ButtonClick, VoltModsAction::OpenFolder], true),
                Some(ModsFocus::ModSettings(i)) => {
                    return (vec![VoltModsAction::ButtonClick, VoltModsAction::Settings(i)], true);
                }
                Some(ModsFocus::Mod(i)) => {
                    let dead = s.mods.installed.get(i).is_none_or(|m| m.dev || m.id.is_empty());
                    if !dead {
                        s.mods.toggle(i);
                        return (vec![VoltModsAction::ButtonClick, VoltModsAction::Toggle(i), VoltModsAction::Changed], true);
                    }
                    return (Vec::new(), true);
                }
                _ => {}
            }
        }
        if matches!(key, K::ArrowUp | K::ArrowDown) {
            let geom = Self::geometry(size.0, size.1);
            let (_, rows, content_h) = self.build_rows(s, &geom, &search.value, self.scroll.amount);
            if !rows.is_empty() {
                let cur = match self.focus {
                    Some(ModsFocus::Mod(i)) | Some(ModsFocus::ModSettings(i)) => {
                        rows.iter().position(|r| r.index == i)
                    }
                    _ => None,
                };
                let next_pos = match (cur, key) {
                    (Some(p), K::ArrowDown) => (p + 1).min(rows.len() - 1),
                    (Some(p), K::ArrowUp) => p.saturating_sub(1),
                    (None, K::ArrowDown) => 0,
                    (None, K::ArrowUp) => rows.len() - 1,
                    _ => 0,
                };
                let target = &rows[next_pos];
                self.focus = Some(ModsFocus::Mod(target.index));
                self.inspected = Some(target.index);
                let top = geom.list_panel.y + 4.0;
                let bot = geom.list_panel.y + geom.list_panel.h - 4.0;
                let max_s = (content_h - geom.list_panel.h).max(0.0);
                if target.rect.y < top {
                    self.scroll.amount = (self.scroll.amount - (top - target.rect.y)).clamp(0.0, max_s);
                } else if target.rect.y + target.rect.h > bot {
                    self.scroll.amount = (self.scroll.amount + (target.rect.y + target.rect.h - bot)).clamp(0.0, max_s);
                }
                return (Vec::new(), true);
            }
        }
        (Vec::new(), false)
    }
}

// ---------------------------------------------------------------------------
// VoltHorizonUi (Page::Horizon — Volt+ Settings)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HorizonTab {
    #[default]
    General,
    Visuals,
    Gameplay,
    System,
}

impl HorizonTab {
    pub const ALL: [Self; 4] = [Self::General, Self::Visuals, Self::Gameplay, Self::System];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Visuals => "Visuals",
            Self::Gameplay => "Gameplay",
            Self::System => "System",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Visuals => 1,
            Self::Gameplay => 2,
            Self::System => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizonItem {
    OpenPage(Page),
    Extra(ExtraItem),
}

#[derive(Clone, Copy, Debug)]
struct HorizonEntry {
    item: HorizonItem,
    title_key: &'static str,
    desc: &'static str,
    cost: Option<Cost>,
}

enum HorizonRow {
    Header(&'static str),
    Entry(HorizonEntry),
}

fn page_entry(page: Page, title_key: &'static str, desc: &'static str) -> HorizonRow {
    HorizonRow::Entry(HorizonEntry {
        item: HorizonItem::OpenPage(page),
        title_key,
        desc,
        cost: None,
    })
}

fn extra_entry(item: ExtraItem) -> HorizonRow {
    HorizonRow::Entry(HorizonEntry {
        item: HorizonItem::Extra(item),
        title_key: "",
        desc: item.description(),
        cost: item.cost(),
    })
}

fn horizon_tab_rows(tab: HorizonTab, s: &Settings) -> Vec<HorizonRow> {
    let mut rows = match tab {
        HorizonTab::General => {
            let mut v = vec![
                HorizonRow::Header("Volt+ Screens"),
                page_entry(Page::Graphics, "horizon.pages.open.graphics", "Volt+ graphics quality presets, anti-aliasing and rendering."),
                page_entry(Page::GraphicsAdvanced, "horizon.pages.open.graphics_advanced", "Fine-grained rendering, culling, shadows and frame pacing."),
                page_entry(Page::VoltShaders, "volt.shaders.open", "Built-in lightweight sky, foliage wave, lighting, water reflection and color pipeline."),
                page_entry(Page::ShaderPacks, "horizon.pages.open.shader_packs", "Choose and configure installed shader packs."),
                page_entry(Page::Controller, "horizon.pages.open.controller", "Gamepad sensitivity, deadzone, glyph style and button binds."),
                page_entry(Page::Extras, "horizon.pages.open.extras", "Lighting, inventory tweaks, sorting and multiplayer helpers."),
                page_entry(Page::Mods, "horizon.pages.open.mods", "Manage installed Volt+ client modifications and mod settings."),
                HorizonRow::Header("Quick Toggles"),
                extra_entry(ExtraItem::DiscordPresence),
            ];
            if !s.extras.no_ai {
                v.push(page_entry(Page::Agent, "horizon.pages.open.agent", "Configure local AI agent control and MCP bridges."));
            }
            v.push(extra_entry(ExtraItem::NoAi));
            v
        }
        HorizonTab::Visuals => vec![
            HorizonRow::Header("Graphics & Shaders"),
            page_entry(Page::Graphics, "horizon.pages.open.graphics", "Volt+ graphics quality presets, anti-aliasing and rendering."),
            page_entry(Page::GraphicsAdvanced, "horizon.pages.open.graphics_advanced", "Fine-grained rendering, culling, shadows and frame pacing."),
            page_entry(Page::GraphicsHdr, "horizon.pages.open.graphics_hdr", "HDR tone mapping, brightness and calibration."),
            page_entry(Page::VoltShaders, "volt.shaders.open", "Built-in lightweight sky, foliage wave, lighting, water reflection and color pipeline."),
            page_entry(Page::ShaderPacks, "horizon.pages.open.shader_packs", "Choose and configure installed shader packs."),
            HorizonRow::Header("Lighting & View"),
            extra_entry(ExtraItem::Fullbright),
            extra_entry(ExtraItem::DynamicLights),
            extra_entry(ExtraItem::OwnNameTag),
        ],
        HorizonTab::Gameplay => vec![
            HorizonRow::Header("Input & Tweaks"),
            page_entry(Page::Controller, "horizon.pages.open.controller", "Gamepad sensitivity, deadzone, glyph style and button binds."),
            page_entry(Page::Extras, "horizon.pages.open.extras", "Full list of Volt+ gameplay and utility extras."),
            HorizonRow::Header("Inventory & Multiplayer"),
            extra_entry(ExtraItem::MouseTweaks),
            extra_entry(ExtraItem::MouseTweaksWheel),
            extra_entry(ExtraItem::Sort),
            extra_entry(ExtraItem::SortKey),
            extra_entry(ExtraItem::AutoReconnect),
        ],
        HorizonTab::System => {
            let mut v = vec![
                HorizonRow::Header("Mods & Integration"),
                page_entry(Page::Mods, "horizon.pages.open.mods", "Manage installed Volt+ client modifications and mod settings."),
                extra_entry(ExtraItem::DiscordPresence),
                HorizonRow::Header("AI & Automation"),
            ];
            if !s.extras.no_ai {
                v.push(page_entry(Page::Agent, "horizon.pages.open.agent", "Configure local AI agent control and MCP bridges."));
                v.push(extra_entry(ExtraItem::AgentControl));
            }
            v.push(extra_entry(ExtraItem::NoAi));
            v
        }
    };
    rows.retain(|r| match r {
        HorizonRow::Entry(e) => match e.item {
            HorizonItem::OpenPage(Page::Agent) | HorizonItem::Extra(ExtraItem::AgentControl) => !s.extras.no_ai,
            _ => true,
        },
        HorizonRow::Header(_) => true,
    });
    rows
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoltHorizonAction {
    ButtonClick,
    Changed,
    Extras,
    OpenPage(Page),
    BindKey(&'static str),
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizonFocus {
    Search,
    Tab(HorizonTab),
    Item(HorizonItem),
    Done,
}

#[derive(Clone, Copy, Debug)]
struct HorizonGeometry {
    side_x: f32,
    brand_y: f32,
    search_box: Rect,
    tabs: [(HorizonTab, Rect); 4],
    done_btn: Rect,
    header_bar: Rect,
    list_panel: Rect,
    list_clip: Rect,
    scrollbar_x: f32,
    footer_card: Rect,
}

#[derive(Clone, Debug)]
enum LaidHorizonRow {
    Header { text: String, y: f32 },
    Entry { entry: HorizonEntry, rect: Rect, ctrl: Rect },
}

pub struct VoltHorizonUi {
    pub tab: HorizonTab,
    pub scroll: ui::Scroll,
    pub focus: Option<HorizonFocus>,
    pub inspected: Option<HorizonItem>,
    opened_at: Option<Instant>,
    tab_changed_at: Instant,
    tab_dir: f32,
    visual_tab_y: Option<f32>,
    visual_scroll: Option<f32>,
    last_frame: Option<Instant>,
    tab_hover: [f32; 4],
    row_hover: Vec<(HorizonItem, f32)>,
    done_hover: f32,
}

impl Default for VoltHorizonUi {
    fn default() -> Self {
        Self {
            tab: HorizonTab::General,
            scroll: ui::Scroll::default(),
            focus: None,
            inspected: None,
            opened_at: None,
            tab_changed_at: Instant::now(),
            tab_dir: 1.0,
            visual_tab_y: None,
            visual_scroll: None,
            last_frame: None,
            tab_hover: [0.0; 4],
            row_hover: Vec::new(),
            done_hover: 0.0,
        }
    }
}

impl VoltHorizonUi {
    pub fn reset(&mut self) {
        self.scroll = ui::Scroll::default();
        self.focus = None;
        self.inspected = None;
        self.opened_at = Some(Instant::now());
        self.tab_changed_at = Instant::now();
        self.visual_tab_y = None;
        self.visual_scroll = None;
        self.last_frame = Some(Instant::now());
        self.row_hover.clear();
    }

    pub fn reset_scroll(&mut self) {
        self.scroll.amount = 0.0;
        self.visual_scroll = None;
    }

    pub fn release(&mut self) -> bool {
        self.scroll.release();
        false
    }

    pub fn select_tab(&mut self, next: HorizonTab) {
        if self.tab != next {
            self.tab_dir = if next.index() > self.tab.index() { 1.0 } else { -1.0 };
            self.tab = next;
            self.tab_changed_at = Instant::now();
            self.scroll.amount = 0.0;
            self.visual_scroll = None;
            self.inspected = None;
        }
    }

    fn geometry(w: f32, h: f32) -> HorizonGeometry {
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
                HorizonTab::ALL[i],
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
            y: (h - pad_bottom - done_h).max(tabs_top + 4.0 * tab_stride + 4.0),
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
        HorizonGeometry {
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

    fn entry_caption(e: &HorizonEntry) -> String {
        match e.item {
            HorizonItem::OpenPage(_) => lang::get(e.title_key),
            HorizonItem::Extra(item) => item.caption().to_owned(),
        }
    }

    fn entry_value(e: &HorizonEntry, s: &Settings, binding_key: Option<&'static str>) -> String {
        match e.item {
            HorizonItem::OpenPage(_) => "Open".to_owned(),
            HorizonItem::Extra(item) => {
                if item.key_name().is_some() && item.key_name() == binding_key {
                    "> ... <".to_owned()
                } else {
                    item.value(&s.extras)
                }
            }
        }
    }

    fn entry_live(e: &HorizonEntry, s: &Settings) -> bool {
        match e.item {
            HorizonItem::OpenPage(_) => true,
            HorizonItem::Extra(item) => item.parent().is_none_or(|p| p.switch(&s.extras) == Some(true)),
        }
    }

    fn filtered_rows(&self, s: &Settings, query: &str) -> (String, Vec<(Option<String>, HorizonEntry)>) {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            let mut out = Vec::new();
            let mut hdr = None;
            for r in horizon_tab_rows(self.tab, s) {
                match r {
                    HorizonRow::Header(h) => hdr = Some(lang::get(h)),
                    HorizonRow::Entry(e) => out.push((hdr.take(), e)),
                }
            }
            return (format!("Volt+ / {}", self.tab.label()), out);
        }
        let mut seen = Vec::new();
        let mut out = Vec::new();
        for tab in HorizonTab::ALL {
            for r in horizon_tab_rows(tab, s) {
                if let HorizonRow::Entry(e) = r {
                    if seen.contains(&e.item) {
                        continue;
                    }
                    let cap = Self::entry_caption(&e).to_lowercase();
                    let desc = e.desc.to_lowercase();
                    if cap.contains(&q) || desc.contains(&q) {
                        seen.push(e.item);
                        out.push((None, e));
                    }
                }
            }
        }
        (format!("Volt+ / Search: {}", query.trim()), out)
    }

    fn build_list_rows(
        &self,
        s: &Settings,
        geom: &HorizonGeometry,
        query: &str,
        scroll_y: f32,
    ) -> (String, Vec<LaidHorizonRow>, f32) {
        let (header, entries) = self.filtered_rows(s, query);
        let row_x = geom.list_panel.x + 6.0;
        let row_w = (geom.list_panel.w - 18.0).max(96.0);
        let ctrl_w = (row_w * 0.40).clamp(64.0, 128.0);
        let mut y = 6.0;
        let mut out = Vec::new();
        for (hdr, entry) in entries {
            if let Some(text) = hdr {
                out.push(LaidHorizonRow::Header {
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
            out.push(LaidHorizonRow::Entry { entry, rect, ctrl });
            y += ROW_STRIDE;
        }
        (header, out, (y + 6.0).max(1.0))
    }

    fn smooth_hover(cache: &mut Vec<(HorizonItem, f32)>, key: HorizonItem, target: f32, dt: f32) -> f32 {
        if let Some((_, v)) = cache.iter_mut().find(|(k, _)| *k == key) {
            if dt > 0.0 {
                let k = 1.0 - (-18.0 * dt).exp();
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
        binding_key: Option<&'static str>,
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

        let (w, h) = (batch.width, batch.height);
        let geom = Self::geometry(w, h);
        let (_, _, content_h) = self.build_list_rows(s, &geom, &search.value, self.scroll.amount);
        let lp_top = geom.list_panel.y;
        let lp_bot = geom.list_panel.y + geom.list_panel.h;
        self.scroll.clamp(lp_top, lp_bot, content_h);
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
        let (header_title, rows, _) = self.build_list_rows(s, &geom, &search.value, scroll_draw);

        let pal = palette(in_world, ui_alpha);
        batch.fill(atlas, 0.0, 0.0, w, h, pal.scrim);

        // Brand mark
        batch.text(atlas, "Volt+", geom.side_x + side_dx, geom.brand_y + 1.0, pal.text_primary, true);
        let vw = HudBatch::text_width(atlas, "Volt+");
        batch.text(atlas, " Settings", geom.side_x + side_dx + vw, geom.brand_y + 1.0, pal.accent_hi, true);
        batch.text(atlas, "Volt Launcher Plus", geom.side_x + side_dx, geom.brand_y + 12.0, pal.text_dim, true);

        // Search box
        let sb = geom.search_box;
        let sb_x = sb.x + side_dx;
        let sb_focused = self.focus == Some(HorizonFocus::Search) || search.focused;
        let sb_hover = sb.contains(cursor) || sb_focused;
        batch.fill(atlas, sb_x, sb.y, sb.w, sb.h, pal.panel_bg);
        draw_border(
            batch,
            atlas,
            sb_x,
            sb.y,
            sb.w,
            sb.h,
            if sb_focused { pal.border_strong } else if sb_hover { with_alpha(pal.border_strong, 0.65) } else { pal.border_subtle },
        );
        crate::widgets::target_as(sb.array(), crate::widgets::Kind::Field);
        search.bordered = false;
        search.x = sb.x + 5.0;
        search.y = sb.y + 4.0;
        search.width = (sb.w - 10.0).max(16.0);
        search.height = 10.0;
        search.text_color = pal.text_primary;
        search.message = "Search settings".to_owned();
        if search.value.is_empty() && !search.focused {
            batch.text(atlas, "Search settings", sb_x + 5.0, sb.y + 4.0, pal.text_dim, false);
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
            let focused = self.focus == Some(HorizonFocus::Tab(*tab));
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
                pal.text_primary
            } else {
                lerp_color(pal.text_muted, pal.text_primary, self.tab_hover[i] * 0.7)
            };
            batch.text(atlas, tab.label(), rx + 7.0, rect.y + 4.0, col, true);
            crate::widgets::record_as(tab.label(), rect.array(), true, crate::widgets::Kind::Tab(selected));
            crate::widgets::widget_cursor(hovered, true);
        }
        if !searching {
            let rx = active_tab_rect.x + side_dx;
            draw_border(batch, atlas, rx, tab_y, active_tab_rect.w, active_tab_rect.h, pal.border_strong);
            batch.fill(atlas, rx + 1.0, tab_y + 3.0, 2.0, active_tab_rect.h - 6.0, pal.accent_hi);
        }

        // Done button
        let done = geom.done_btn;
        let done_hovered = done.contains(cursor) || self.focus == Some(HorizonFocus::Done);
        if dt > 0.0 {
            let k = 1.0 - (-18.0 * dt).exp();
            self.done_hover += ((if done_hovered { 1.0 } else { 0.0 }) - self.done_hover) * k;
        } else {
            self.done_hover = if done_hovered { 1.0 } else { 0.0 };
        }
        let done_x = done.x + side_dx;
        batch.fill(atlas, done_x, done.y, done.w, done.h, lerp_color(pal.accent, pal.accent_hi, self.done_hover));
        batch.fill(atlas, done_x, done.y, done.w, 1.0, pal.accent_edge);
        if self.focus == Some(HorizonFocus::Done) {
            draw_border(batch, atlas, done_x, done.y, done.w, done.h, pal.border_strong);
        }
        let done_text = Text::tr("gui.done", &[]);
        ui::centered_text(batch, atlas, &done_text, done_x + 2.0, done.y + 6.0, done.w - 4.0, pal.text_primary);
        crate::widgets::record(&done_text.plain(), done.array(), true);
        crate::widgets::widget_cursor(done_hovered, true);

        // Header bar
        let hdr = geom.header_bar;
        let hdr_y = hdr.y + main_dy;
        batch.fill(atlas, hdr.x, hdr_y, hdr.w, hdr.h, pal.bar_bg);
        draw_border(batch, atlas, hdr.x, hdr_y, hdr.w, hdr.h, pal.border_subtle);
        batch.text(
            atlas,
            &header_title,
            hdr.x + 8.0,
            hdr_y + 5.0,
            with_alpha(pal.text_primary, ui_alpha * (0.35 + 0.65 * tab_p)),
            true,
        );

        // List panel
        let lp = geom.list_panel;
        let lp_y = lp.y + main_dy;
        batch.fill(atlas, lp.x, lp_y, lp.w, lp.h, pal.panel_bg);
        draw_border(batch, atlas, lp.x, lp_y, lp.w, lp.h, pal.border_subtle);

        let content_slide_y = main_dy + (1.0 - tab_p) * self.tab_dir * 8.0;
        let content_alpha = ui_alpha * (0.25 + 0.75 * tab_p);
        let clip_x0 = geom.list_clip.x;
        let clip_y0 = geom.list_clip.y + main_dy;
        let clip_x1 = clip_x0 + geom.list_clip.w;
        let clip_y1 = clip_y0 + geom.list_clip.h;
        let list_verts = batch.vertices.len();
        let target_mark = crate::widgets::targets_mark();
        let in_list_y = cursor.1 >= lp.y && cursor.1 < lp.y + lp.h;

        let mut hovered_entry: Option<HorizonEntry> = None;
        let mut focused_entry: Option<HorizonEntry> = None;
        let mut first_entry: Option<HorizonEntry> = None;

        for row in &rows {
            match row {
                LaidHorizonRow::Header { text, y } => {
                    let dy = *y + content_slide_y;
                    if dy + GROUP_HEIGHT < clip_y0 - 4.0 || dy > clip_y1 + 4.0 {
                        continue;
                    }
                    batch.text(atlas, text, lp.x + 8.0, dy + 4.0, with_alpha(pal.text_muted, content_alpha), true);
                    batch.fill(atlas, lp.x + 8.0, dy + GROUP_HEIGHT - 3.0, lp.w - 20.0, 1.0, with_alpha(rgba(255, 255, 255, 22), content_alpha));
                }
                LaidHorizonRow::Entry { entry, rect, ctrl } => {
                    if first_entry.is_none() {
                        first_entry = Some(*entry);
                    }
                    let dy = rect.y + content_slide_y;
                    let visible = dy + rect.h >= clip_y0 - 4.0 && dy <= clip_y1 + 4.0;
                    let live = Self::entry_live(entry, s);
                    let is_hovered = in_list_y && rect.contains(cursor);
                    let is_focused = self.focus == Some(HorizonFocus::Item(entry.item));
                    let is_selected = self.inspected == Some(entry.item);
                    if is_hovered {
                        hovered_entry = Some(*entry);
                    }
                    if is_focused {
                        focused_entry = Some(*entry);
                    }
                    let hl = Self::smooth_hover(
                        &mut self.row_hover,
                        entry.item,
                        if is_hovered || is_focused || is_selected { 1.0 } else { 0.0 },
                        dt,
                    );
                    if !visible {
                        continue;
                    }
                    let idle_fill = with_alpha(rgba(255, 255, 255, 20), content_alpha);
                    let active_fill = with_alpha(rgba(255, 255, 255, 44), content_alpha);
                    batch.fill(atlas, rect.x, dy, rect.w, rect.h, lerp_color(idle_fill, active_fill, hl));
                    if is_selected || is_focused {
                        batch.fill(atlas, rect.x, dy + 3.0, 2.0, rect.h - 6.0, with_alpha(pal.accent_hi, content_alpha));
                    }

                    let cap = Self::entry_caption(entry);
                    let label_max = (ctrl.x - (rect.x + 7.0) - 6.0).max(24.0);
                    let cap_drawn = fit_with_ellipsis(atlas, &cap, label_max);
                    batch.text(
                        atlas,
                        &cap_drawn,
                        rect.x + 7.0,
                        dy + 6.0,
                        with_alpha(if live { pal.text_primary } else { pal.text_dim }, content_alpha),
                        true,
                    );

                    let val = Self::entry_value(entry, s, binding_key);
                    let val_w = HudBatch::text_width(atlas, &val);
                    let is_page = matches!(entry.item, HorizonItem::OpenPage(_));
                    let pill_w = (val_w + if is_page { 18.0 } else { 24.0 }).clamp((ctrl.w * 0.62).max(58.0).min(ctrl.w), ctrl.w);
                    let pill_x = ctrl.x + ctrl.w - pill_w;
                    let pill_y = dy + 2.0;
                    let pill_h = 16.0;
                    batch.fill(
                        atlas,
                        pill_x,
                        pill_y,
                        pill_w,
                        pill_h,
                        with_alpha(rgba(46, 43, 68, (175.0 + 45.0 * hl) as u8), content_alpha),
                    );
                    draw_border(
                        batch,
                        atlas,
                        pill_x,
                        pill_y,
                        pill_w,
                        pill_h,
                        if is_focused || is_hovered || is_selected {
                            with_alpha(pal.border_strong, content_alpha)
                        } else {
                            with_alpha(rgba(255, 255, 255, 58), content_alpha)
                        },
                    );
                    if !is_page {
                        let arrow_col = with_alpha(if live { pal.text_muted } else { pal.text_dim }, content_alpha);
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
                        with_alpha(if live { pal.text_primary } else { pal.text_dim }, content_alpha),
                    );
                    crate::widgets::record(&cap, ctrl.array(), live);
                    crate::widgets::widget_cursor(is_hovered && live, live);
                }
            }
        }

        if content_h > lp.h + 0.5 {
            let max_s = (content_h - lp.h).max(1.0);
            let thumb_h = ((lp.h / content_h) * (lp.h - 8.0)).clamp(16.0, lp.h - 8.0);
            let ratio = (scroll_draw / max_s).clamp(0.0, 1.0);
            let thumb_y = lp_y + 4.0 + (lp.h - 8.0 - thumb_h) * ratio;
            batch.fill(atlas, geom.scrollbar_x, lp_y + 4.0, 2.0, lp.h - 8.0, with_alpha(rgba(255, 255, 255, 18), ui_alpha));
            batch.fill(atlas, geom.scrollbar_x, thumb_y, 2.0, thumb_h, with_alpha(pal.accent_hi, ui_alpha * 0.85));
        }

        batch.clip_since(list_verts, clip_x0, clip_y0, clip_x1, clip_y1);
        crate::widgets::clip_targets(target_mark, lp_y, lp_y + lp.h);

        // Footer card
        let fc = geom.footer_card;
        let fc_y = fc.y + main_dy;
        batch.fill(atlas, fc.x, fc_y, fc.w, fc.h, pal.bar_bg);
        draw_border(batch, atlas, fc.x, fc_y, fc.w, fc.h, pal.border_subtle);
        let inspected_entry = hovered_entry
            .or(focused_entry)
            .or_else(|| {
                self.inspected.and_then(|wanted| {
                    rows.iter().find_map(|r| match r {
                        LaidHorizonRow::Entry { entry, .. } if entry.item == wanted => Some(*entry),
                        _ => None,
                    })
                })
            })
            .or(first_entry);
        if let Some(e) = inspected_entry {
            let cap = Self::entry_caption(&e);
            let right_label = e.cost.map(|c| (format!("[{}]", c.label()), cost_color(c)));
            let bw = right_label
                .as_ref()
                .map_or(0.0, |(t, _)| HudBatch::text_width(atlas, t) + 12.0);
            if let Some((txt, col)) = right_label {
                let tw = HudBatch::text_width(atlas, &txt);
                batch.text(atlas, &txt, fc.x + fc.w - tw - 8.0, fc_y + 5.0, with_alpha(col, ui_alpha), true);
            }
            let cap_fit = fit_with_ellipsis(atlas, &cap, (fc.w - bw - 16.0).max(40.0));
            batch.text(atlas, &cap_fit, fc.x + 8.0, fc_y + 5.0, pal.text_primary, true);
            let desc_fit = fit_with_ellipsis(atlas, e.desc, (fc.w - 16.0).max(40.0));
            batch.text(atlas, &desc_fit, fc.x + 8.0, fc_y + 17.0, pal.text_muted, true);
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
    ) -> Vec<VoltHorizonAction> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, content_h) = self.build_list_rows(s, &geom, &search.value, self.scroll.amount);
        let lp_top = geom.list_panel.y;
        let lp_bot = geom.list_panel.y + geom.list_panel.h;
        if drag {
            if self.scroll.held() {
                self.scroll.drag(pos.1, lp_top, lp_bot, content_h);
                self.visual_scroll = Some(self.scroll.amount);
            }
            return Vec::new();
        }
        if geom.search_box.contains(pos) {
            self.focus = Some(HorizonFocus::Search);
            search.focused = true;
            search.bordered = false;
            search.x = geom.search_box.x + 5.0;
            search.y = geom.search_box.y + 4.0;
            search.width = (geom.search_box.w - 10.0).max(16.0);
            search.click(atlas, pos.0, false, false);
            return Vec::new();
        }
        search.focused = false;
        for (tab, rect) in geom.tabs {
            if rect.contains(pos) {
                self.select_tab(tab);
                self.focus = Some(HorizonFocus::Tab(tab));
                if !search.value.is_empty() {
                    search.set_value("");
                }
                return vec![VoltHorizonAction::ButtonClick];
            }
        }
        if geom.done_btn.contains(pos) {
            self.focus = Some(HorizonFocus::Done);
            return vec![VoltHorizonAction::ButtonClick, VoltHorizonAction::Done];
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
        if geom.list_panel.contains(pos) {
            for row in rows {
                if let LaidHorizonRow::Entry { entry, rect, ctrl } = row
                    && rect.contains(pos)
                {
                    self.focus = Some(HorizonFocus::Item(entry.item));
                    self.inspected = Some(entry.item);
                    if !Self::entry_live(&entry, s) {
                        return Vec::new();
                    }
                    return match entry.item {
                        HorizonItem::OpenPage(page) => {
                            vec![VoltHorizonAction::ButtonClick, VoltHorizonAction::OpenPage(page)]
                        }
                        HorizonItem::Extra(item) => {
                            if let Some(key_name) = item.key_name() {
                                vec![VoltHorizonAction::ButtonClick, VoltHorizonAction::BindKey(key_name)]
                            } else {
                                let step_back = right || (ctrl.contains(pos) && pos.0 < ctrl.x + ctrl.w * 0.35);
                                item.cycle(&mut s.extras, step_back);
                                vec![
                                    VoltHorizonAction::ButtonClick,
                                    VoltHorizonAction::Changed,
                                    VoltHorizonAction::Extras,
                                ]
                            }
                        }
                    };
                }
            }
        }
        Vec::new()
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

    pub fn key_modifiers(
        &mut self,
        s: &mut Settings,
        key: winit::keyboard::KeyCode,
        text: Option<&str>,
        shift: bool,
        size: (f32, f32),
        search: &mut ui::EditBox,
    ) -> (Vec<VoltHorizonAction>, bool) {
        use winit::keyboard::KeyCode as K;
        if key == K::Escape {
            return (vec![VoltHorizonAction::Done], true);
        }
        if (self.focus == Some(HorizonFocus::Search) || search.focused)
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
        if matches!(key, K::Enter | K::NumpadEnter | K::Space) {
            match self.focus {
                Some(HorizonFocus::Done) => return (vec![VoltHorizonAction::ButtonClick, VoltHorizonAction::Done], true),
                Some(HorizonFocus::Item(HorizonItem::OpenPage(page))) => {
                    return (vec![VoltHorizonAction::ButtonClick, VoltHorizonAction::OpenPage(page)], true);
                }
                Some(HorizonFocus::Item(HorizonItem::Extra(item))) => {
                    let live = item.parent().is_none_or(|p| p.switch(&s.extras) == Some(true));
                    if live {
                        if let Some(k) = item.key_name() {
                            return (vec![VoltHorizonAction::ButtonClick, VoltHorizonAction::BindKey(k)], true);
                        }
                        item.cycle(&mut s.extras, false);
                        return (
                            vec![VoltHorizonAction::ButtonClick, VoltHorizonAction::Changed, VoltHorizonAction::Extras],
                            true,
                        );
                    }
                    return (Vec::new(), true);
                }
                _ => {}
            }
        }
        if matches!(key, K::ArrowUp | K::ArrowDown) {
            let geom = Self::geometry(size.0, size.1);
            let (_, rows, content_h) = self.build_list_rows(s, &geom, &search.value, self.scroll.amount);
            let entries: Vec<_> = rows
                .iter()
                .filter_map(|r| match r {
                    LaidHorizonRow::Entry { entry, rect, .. } => Some((*entry, *rect)),
                    LaidHorizonRow::Header { .. } => None,
                })
                .collect();
            if !entries.is_empty() {
                let cur = match self.focus {
                    Some(HorizonFocus::Item(it)) => entries.iter().position(|(e, _)| e.item == it),
                    _ => None,
                };
                let next_pos = match (cur, key) {
                    (Some(p), K::ArrowDown) => (p + 1).min(entries.len() - 1),
                    (Some(p), K::ArrowUp) => p.saturating_sub(1),
                    (None, K::ArrowDown) => 0,
                    (None, K::ArrowUp) => entries.len() - 1,
                    _ => 0,
                };
                let (target, rect) = entries[next_pos];
                self.focus = Some(HorizonFocus::Item(target.item));
                self.inspected = Some(target.item);
                let top = geom.list_panel.y + 4.0;
                let bot = geom.list_panel.y + geom.list_panel.h - 4.0;
                let max_s = (content_h - geom.list_panel.h).max(0.0);
                if rect.y < top {
                    self.scroll.amount = (self.scroll.amount - (top - rect.y)).clamp(0.0, max_s);
                } else if rect.y + rect.h > bot {
                    self.scroll.amount = (self.scroll.amount + (rect.y + rect.h - bot)).clamp(0.0, max_s);
                }
                return (Vec::new(), true);
            }
        }
        (Vec::new(), false)
    }
}

#[cfg(test)]
impl VoltModsUi {
    pub(crate) fn test_open_folder_rect(w: f32, h: f32) -> [f32; 4] {
        Self::geometry(w, h).folder_btn.array()
    }
    pub(crate) fn test_row_rects(&self, s: &Settings, size: (f32, f32), query: &str) -> Vec<(usize, [f32; 4], Option<[f32; 4]>)> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, _) = self.build_rows(s, &geom, query, self.scroll.amount);
        rows.into_iter()
            .map(|r| (r.index, r.rect.array(), r.settings_rect.map(|sr| sr.array())))
            .collect()
    }
}

#[cfg(test)]
impl VoltHorizonUi {
    pub(crate) fn test_row_rect(&self, s: &Settings, size: (f32, f32), query: &str, item: HorizonItem) -> Option<[f32; 4]> {
        let geom = Self::geometry(size.0, size.1);
        let (_, rows, _) = self.build_list_rows(s, &geom, query, self.scroll.amount);
        rows.into_iter().find_map(|r| match r {
            LaidHorizonRow::Entry { entry, rect, .. } if entry.item == item => Some(rect.array()),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> HudAtlas {
        HudAtlas::build(std::path::Path::new("/nonexistent/horizon-test-assets")).unwrap()
    }

    #[test]
    fn volt_mods_and_horizon_draw_cleanly_across_tabs() {
        let atlas = atlas();
        let s = Settings::default();
        let mut search = ui::EditBox::new(0.0, 0.0, 100.0, 16.0);
        let mut mods = VoltModsUi::default();
        for tab in ModsTab::ALL {
            mods.select_tab(tab);
            let mut batch = HudBatch::new([480, 270]);
            mods.draw(&s, &mut batch, &atlas, (240.0, 135.0), true, &mut search);
            assert!(!batch.vertices.is_empty());
        }
        let mut horizon = VoltHorizonUi::default();
        for tab in HorizonTab::ALL {
            horizon.select_tab(tab);
            let mut batch = HudBatch::new([480, 270]);
            horizon.draw(&s, &mut batch, &atlas, (240.0, 135.0), false, &mut search, None);
            assert!(!batch.vertices.is_empty());
        }
    }
}
