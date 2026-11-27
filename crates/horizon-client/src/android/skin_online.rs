//! Fullscreen online Skin & Cape browser (SkinMC, KLauncher animated/plain capes,
//! Mojang player lookup, and built-in animated/static capes) with live 2D/3D preview.
use super::{
    pages::{Action, button},
    wardrobe::Wardrobe,
};
use crate::skins::{self, Model};
use horizon_client::{
    hud::rgba,
    ui_theme::{Face, Icon, Ui, active_theme},
};
use regex::Regex;
use std::{
    collections::HashSet,
    io::Read,
    sync::mpsc,
    time::{Duration, Instant},
};

const MAX_REMOTE_PNG: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tab {
    Skins,
    Capes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SkinCategory {
    Trending,
    Popular,
    Latest,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CapeKindFilter {
    All,
    Animated,
    Plain,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CapeSourceFilter {
    All,
    KLauncher,
    SkinMc,
}

#[derive(Clone)]
pub(super) struct SkinItem {
    pub title: String,
    pub subtitle: String,
    pub source: &'static str,
    pub slim: bool,
    pub png_bytes: Vec<u8>,
    pub runs: Vec<([f32; 4], u32)>,
}

#[derive(Clone)]
pub(super) struct CapeItem {
    pub title: String,
    pub subtitle: String,
    pub source: &'static str,
    pub frames: usize,
    pub is_animated: bool,
    pub png_bytes: Vec<u8>,
    pub frame_runs: Vec<Vec<([f32; 4], u32)>>,
}

enum FetchUpdate {
    SkinsPage {
        generation: u64,
        items: Vec<SkinItem>,
        status: String,
    },
    CapesPage {
        generation: u64,
        items: Vec<CapeItem>,
        status: String,
    },
}

pub(super) struct SkinOnlineState {
    pub tab: Tab,
    pub skin_cat: SkinCategory,
    pub cape_filter: CapeKindFilter,
    pub cape_source: CapeSourceFilter,
    pub page: usize,
    pub scroll: f32,
    pub status: String,
    pub loading: bool,
    pub fetched_skins_once: bool,
    pub fetched_capes_once: bool,
    pub skins: Vec<SkinItem>,
    pub capes: Vec<CapeItem>,
    pub builtin_capes: Vec<CapeItem>,
    pub active_query: String,
    generation: u64,
    worker: Option<mpsc::Receiver<FetchUpdate>>,
    anim_origin: Instant,
    pub last_equip_title: Option<String>,
}

impl SkinOnlineState {
    pub fn new() -> Self {
        Self {
            tab: Tab::Skins,
            skin_cat: SkinCategory::Trending,
            cape_filter: CapeKindFilter::All,
            cape_source: CapeSourceFilter::All,
            page: 1,
            scroll: 0.0,
            status: "Browse SkinMC skins & KLauncher animated/plain capes".into(),
            loading: false,
            fetched_skins_once: false,
            fetched_capes_once: false,
            skins: Vec::new(),
            capes: Vec::new(),
            builtin_capes: build_builtin_capes(),
            active_query: String::new(),
            generation: 0,
            worker: None,
            anim_origin: Instant::now(),
            last_equip_title: None,
        }
    }

    pub fn open_initial(&mut self) {
        if self.tab == Tab::Skins && !self.fetched_skins_once && !self.loading {
            self.fetch_current(String::new());
        } else if self.tab == Tab::Capes && !self.fetched_capes_once && !self.loading {
            self.fetch_current(String::new());
        }
    }

    pub fn set_tab(&mut self, tab: Tab, query: &str) {
        if self.tab != tab {
            self.tab = tab;
            self.page = 1;
            self.scroll = 0.0;
            if (tab == Tab::Skins && !self.fetched_skins_once)
                || (tab == Tab::Capes && !self.fetched_capes_once)
            {
                self.fetch_current(query.to_owned());
            }
        }
    }

    pub fn search(&mut self, query: String) {
        self.page = 1;
        self.scroll = 0.0;
        self.fetch_current(query);
    }

    pub fn next_page(&mut self, delta: isize, query: &str) {
        let next = if delta < 0 {
            self.page.saturating_sub(1).max(1)
        } else {
            self.page.saturating_add(1).min(50)
        };
        if next != self.page {
            self.page = next;
            self.scroll = 0.0;
            self.fetch_current(query.to_owned());
        }
    }

    pub fn fetch_current(&mut self, query: String) {
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.active_query = query.trim().to_owned();
        self.loading = true;
        let tab = self.tab;
        let skin_cat = self.skin_cat;
        let cape_filter = self.cape_filter;
        let cape_source = self.cape_source;
        let page = self.page;
        let q = self.active_query.clone();

        self.status = match tab {
            Tab::Skins => {
                if q.is_empty() {
                    format!("Loading SkinMC page {page}...")
                } else {
                    format!("Searching '{q}' on Mojang & SkinMC...")
                }
            }
            Tab::Capes => {
                if q.is_empty() {
                    format!("Loading KLauncher & SkinMC capes (page {page})...")
                } else {
                    format!("Searching capes for '{q}'...")
                }
            }
        };

        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || match tab {
            Tab::Skins => {
                let (items, status) = fetch_skins_blocking(skin_cat, page, &q);
                let _ = tx.send(FetchUpdate::SkinsPage { generation, items, status });
            }
            Tab::Capes => {
                let (items, status) = fetch_capes_blocking(cape_filter, cape_source, page, &q);
                let _ = tx.send(FetchUpdate::CapesPage { generation, items, status });
            }
        });
    }

    pub fn poll(&mut self) {
        let Some(rx) = &self.worker else {
            return;
        };
        match rx.try_recv() {
            Ok(FetchUpdate::SkinsPage { generation, items, status }) => {
                self.worker = None;
                if generation == self.generation {
                    self.loading = false;
                    self.fetched_skins_once = true;
                    self.skins = items;
                    self.status = status;
                }
            }
            Ok(FetchUpdate::CapesPage { generation, items, status }) => {
                self.worker = None;
                if generation == self.generation {
                    self.loading = false;
                    self.fetched_capes_once = true;
                    self.capes = items;
                    self.status = status;
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.worker = None;
                self.loading = false;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    pub fn is_animating(&self) -> bool {
        self.loading
            || (self.tab == Tab::Capes && self.visible_capes().iter().any(|(_, c)| c.is_animated))
    }

    pub fn visible_capes(&self) -> Vec<(usize, &CapeItem)> {
        // Combine fetched online capes + built-in animated/plain fallback capes (deduped by title).
        let q = self.active_query.to_lowercase();
        let mut out = Vec::new();
        let total = self.capes.len() + self.builtin_capes.len();
        for idx in 0..total {
            let item = if idx < self.capes.len() {
                &self.capes[idx]
            } else {
                &self.builtin_capes[idx - self.capes.len()]
            };
            let kind_ok = match self.cape_filter {
                CapeKindFilter::All => true,
                CapeKindFilter::Animated => item.is_animated,
                CapeKindFilter::Plain => !item.is_animated,
            };
            let source_ok = match self.cape_source {
                CapeSourceFilter::All => true,
                CapeSourceFilter::KLauncher => item.source == "KLauncher" || item.source == "Built-in",
                CapeSourceFilter::SkinMc => item.source == "SkinMC" || item.source == "Player",
            };
            let query_ok = q.is_empty()
                || item.title.to_lowercase().contains(&q)
                || item.source.to_lowercase().contains(&q);
            if kind_ok && source_ok && query_ok {
                out.push((idx, item));
            }
        }
        out
    }

    pub fn equip_item(&mut self, index: usize, wardrobe: &mut Wardrobe, client_id: &str) {
        match self.tab {
            Tab::Skins => {
                let Some(item) = self.skins.get(index).cloned() else {
                    return;
                };
                match wardrobe.equip_online_skin(&item.title, &item.png_bytes, item.slim, client_id) {
                    Ok(()) => {
                        self.last_equip_title = Some(item.title.clone());
                        self.status = wardrobe.status_note().to_owned();
                    }
                    Err(e) => {
                        self.status = e;
                    }
                }
            }
            Tab::Capes => {
                let item = if index < self.capes.len() {
                    self.capes.get(index).cloned()
                } else {
                    self.builtin_capes.get(index - self.capes.len()).cloned()
                };
                let Some(item) = item else {
                    return;
                };
                match wardrobe.equip_online_cape(&item.title, &item.png_bytes) {
                    Ok(()) => {
                        self.last_equip_title = Some(item.title.clone());
                        self.status = wardrobe.status_note().to_owned();
                    }
                    Err(e) => {
                        self.status = e;
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        targets: &mut Vec<([f32; 4], Action)>,
        wardrobe: &mut Wardrobe,
        query: &str,
        editing: Option<&'static str>,
        w: f32,
        full_h: f32,
        ime_top_dp: Option<f32>,
    ) {
        self.poll();
        let theme = active_theme();
        let pad = 16.0;
        let inner_w = (w - pad * 2.0).max(260.0);

        // Fullscreen Material You 3 panel backdrop
        ui.rounded(8.0, 8.0, w - 16.0, full_h - 16.0, 20.0, theme.surface);
        ui.stroke_rounded(
            8.0,
            8.0,
            w - 16.0,
            full_h - 16.0,
            20.0,
            1.0,
            theme.border,
        );

        // Top Header Bar
        button(
            ui,
            targets,
            [pad + 4.0, 16.0, 74.0, 32.0],
            "Back",
            Action::CloseSkinOnline,
            false,
        );
        ui.rounded(pad + 86.0, 18.0, 28.0, 28.0, 8.0, theme.accent_soft);
        ui.icon(Icon::Installed, pad + 92.0, 24.0, 16.0, theme.accent);
        ui.text(
            Face::SemiBold,
            17.0,
            pad + 122.0,
            15.0,
            theme.text_primary,
            "Skin Online",
        );

        if inner_w >= 680.0 {
            let status_fit =
                ui.fit_text(Face::Regular, 10.0, &self.status, (inner_w - 580.0).max(80.0));
            ui.text(
                Face::Regular,
                10.0,
                pad + 122.0,
                35.0,
                if self.loading {
                    theme.accent
                } else {
                    theme.text_muted
                },
                &status_fit,
            );
        }

        // Top-right Controls: Skins | Capes | Wavey Capes | FPS (when Capes)
        let mut rx = w - pad - 4.0;
        if self.tab == Tab::Capes {
            let fps_w = 72.0;
            rx -= fps_w;
            button(
                ui,
                targets,
                [rx, 16.0, fps_w, 32.0],
                &format!("{} FPS", wardrobe.cape_fps()),
                Action::SkinOnlineCycleFps,
                false,
            );
            rx -= 8.0;
        }
        let wavey_w = 122.0;
        rx -= wavey_w;
        button(
            ui,
            targets,
            [rx, 16.0, wavey_w, 32.0],
            if wardrobe.wavey_capes() {
                "Wavey: ON"
            } else {
                "Wavey: OFF"
            },
            Action::SkinOnlineToggleWavey,
            wardrobe.wavey_capes(),
        );
        rx -= 8.0;

        let capes_w = 88.0;
        rx -= capes_w;
        button(
            ui,
            targets,
            [rx, 16.0, capes_w, 32.0],
            "Capes",
            Action::SkinOnlineTab(true),
            self.tab == Tab::Capes,
        );
        rx -= 6.0;

        let skins_w = 88.0;
        rx -= skins_w;
        button(
            ui,
            targets,
            [rx, 16.0, skins_w, 32.0],
            "Skins",
            Action::SkinOnlineTab(false),
            self.tab == Tab::Skins,
        );

        ui.rounded(pad, 56.0, inner_w, 1.0, 0.5, theme.border);

        let body_top = 64.0;
        let body_bottom = ime_top_dp.unwrap_or(full_h - 14.0).min(full_h - 14.0);
        let body_h = (body_bottom - body_top).max(150.0);

        // Left Column: Live 3D Character Preview + Model / Rotate / Cape Visibility
        let left_w = (inner_w * 0.27).clamp(155.0, 235.0);
        let left_x = pad;
        ui.rounded(left_x, body_top, left_w, body_h, 14.0, theme.surface_dim);
        ui.border(left_x, body_top, left_w, body_h, 14.0, theme.border);

        let controls_h = 72.0;
        let scene_h = (body_h - controls_h - 16.0).max(90.0);
        let scene_rect = [left_x + 8.0, body_top + 6.0, left_w - 16.0, scene_h];
        wardrobe.draw_scene(ui, scene_rect);

        let ctrl_y = body_top + body_h - controls_h;
        let half_btn = (left_w - 22.0) * 0.5;
        button(
            ui,
            targets,
            [left_x + 8.0, ctrl_y, half_btn, 28.0],
            "< Turn",
            Action::WardrobeRotate(-1),
            false,
        );
        button(
            ui,
            targets,
            [left_x + 14.0 + half_btn, ctrl_y, half_btn, 28.0],
            "Turn >",
            Action::WardrobeRotate(1),
            false,
        );
        if self.tab == Tab::Capes {
            button(
                ui,
                targets,
                [left_x + 8.0, ctrl_y + 34.0, left_w - 16.0, 30.0],
                if wardrobe.is_cape_hidden() {
                    "Cape: Hidden"
                } else {
                    "Cape: Visible"
                },
                Action::WardrobeHideCape,
                false,
            );
        } else {
            button(
                ui,
                targets,
                [left_x + 8.0, ctrl_y + 34.0, left_w - 16.0, 30.0],
                if wardrobe.is_slim() {
                    "Model: Slim (3px)"
                } else {
                    "Model: Classic (4px)"
                },
                Action::WardrobeModel,
                false,
            );
        }

        // Right Column: Search, Filter Pills, Scrollable Grid, Pagination
        let right_x = left_x + left_w + 12.0;
        let right_w = (inner_w - left_w - 12.0).max(200.0);

        // Search row
        let search_btn_w = 72.0;
        let refresh_btn_w = 78.0;
        let field_w = (right_w - search_btn_w - refresh_btn_w - 14.0).max(90.0);
        ui.text_field(
            right_x,
            body_top,
            field_w,
            38.0,
            if self.tab == Tab::Skins {
                "Search SkinMC or Minecraft username"
            } else {
                "Search cape name or player username"
            },
            query,
            if self.tab == Tab::Skins {
                "e.g. Dream, Knight, Cyberpunk..."
            } else {
                "e.g. Flame, Migrator, Notching..."
            },
            editing == Some("skin_online_search"),
        );
        targets.push((
            [right_x, body_top, field_w, 38.0],
            Action::Field("skin_online_search"),
        ));
        button(
            ui,
            targets,
            [right_x + field_w + 6.0, body_top, search_btn_w, 38.0],
            "Search",
            Action::SkinOnlineSearch,
            true,
        );
        button(
            ui,
            targets,
            [
                right_x + field_w + search_btn_w + 12.0,
                body_top,
                refresh_btn_w,
                38.0,
            ],
            "Refresh",
            Action::SkinOnlineRefresh,
            false,
        );

        // Filter row
        let filter_y = body_top + 44.0;
        let mut fx = right_x;
        match self.tab {
            Tab::Skins => {
                for (label, cat) in [
                    ("Trending", SkinCategory::Trending),
                    ("Popular", SkinCategory::Popular),
                    ("Latest", SkinCategory::Latest),
                ] {
                    let bw = 86.0;
                    button(
                        ui,
                        targets,
                        [fx, filter_y, bw, 28.0],
                        label,
                        Action::SkinOnlineSkinCategory(cat),
                        self.skin_cat == cat,
                    );
                    fx += bw + 6.0;
                }
            }
            Tab::Capes => {
                for (label, kf) in [
                    ("All", CapeKindFilter::All),
                    ("Animated", CapeKindFilter::Animated),
                    ("Plain", CapeKindFilter::Plain),
                ] {
                    let bw = 78.0;
                    button(
                        ui,
                        targets,
                        [fx, filter_y, bw, 28.0],
                        label,
                        Action::SkinOnlineCapeFilter(kf),
                        self.cape_filter == kf,
                    );
                    fx += bw + 6.0;
                }
                fx += 6.0;
                for (label, sf) in [
                    ("All Sites", CapeSourceFilter::All),
                    ("KLauncher", CapeSourceFilter::KLauncher),
                    ("SkinMC", CapeSourceFilter::SkinMc),
                ] {
                    let bw = 82.0;
                    if fx + bw <= right_x + right_w {
                        button(
                            ui,
                            targets,
                            [fx, filter_y, bw, 28.0],
                            label,
                            Action::SkinOnlineCapeSource(sf),
                            self.cape_source == sf,
                        );
                        fx += bw + 6.0;
                    }
                }
            }
        }

        // Scrollable Grid Area
        let grid_top = filter_y + 34.0;
        let footer_h = 34.0;
        let grid_bottom = (body_bottom - footer_h - 6.0).max(grid_top + 60.0);
        let grid_h = grid_bottom - grid_top;

        let cols = if right_w >= 660.0 {
            4usize
        } else if right_w >= 430.0 {
            3
        } else {
            2
        };
        let col_gap = 10.0;
        let row_gap = 10.0;
        let card_w = (right_w - col_gap * (cols as f32 - 1.0)) / cols as f32;
        let card_h = 136.0;

        let item_count = match self.tab {
            Tab::Skins => self.skins.len(),
            Tab::Capes => self.visible_capes().len(),
        };

        let total_rows = item_count.div_ceil(cols);
        let content_h = total_rows as f32 * (card_h + row_gap);
        let max_scroll = (content_h - grid_h).max(0.0);
        self.scroll = self.scroll.clamp(0.0, max_scroll);

        if item_count == 0 {
            ui.rounded(
                right_x,
                grid_top,
                right_w,
                grid_h.min(130.0),
                12.0,
                theme.surface_dim,
            );
            let msg = if self.loading {
                "Fetching skins & capes from online catalogs..."
            } else {
                "No matches found. Try clearing search or changing filter."
            };
            ui.centered_fit(
                Face::Medium,
                13.0,
                [right_x + 12.0, grid_top + 20.0, right_w - 24.0, 80.0],
                theme.text_muted,
                msg,
            );
        } else {
            match self.tab {
                Tab::Skins => {
                    for (i, item) in self.skins.iter().enumerate() {
                        let col = i % cols;
                        let r = i / cols;
                        let cx = right_x + col as f32 * (card_w + col_gap);
                        let cy = grid_top + r as f32 * (card_h + row_gap) - self.scroll;
                        if cy + card_h < grid_top - 4.0 || cy > grid_bottom + 4.0 {
                            continue;
                        }
                        let is_equipped = self
                            .last_equip_title
                            .as_deref()
                            .is_some_and(|t| t == item.title);
                        ui.rounded(
                            cx,
                            cy,
                            card_w,
                            card_h,
                            12.0,
                            if is_equipped {
                                theme.surface_selected
                            } else {
                                theme.surface_dim
                            },
                        );
                        ui.stroke_rounded(
                            cx,
                            cy,
                            card_w,
                            card_h,
                            12.0,
                            if is_equipped { 1.6 } else { 1.0 },
                            if is_equipped {
                                theme.accent
                            } else {
                                theme.border
                            },
                        );

                        // Skin miniature (16x32 aspect -> 44x88)
                        let thumb_x = cx + 10.0;
                        let thumb_y = cy + 12.0;
                        let thumb_w = 44.0;
                        let thumb_h = 88.0;
                        ui.rounded(
                            thumb_x - 3.0,
                            thumb_y - 3.0,
                            thumb_w + 6.0,
                            thumb_h + 6.0,
                            8.0,
                            theme.surface,
                        );
                        draw_runs(ui, &item.runs, thumb_x, thumb_y, thumb_w, thumb_h);

                        let tx = thumb_x + thumb_w + 10.0;
                        let tw = (cx + card_w - tx - 10.0).max(50.0);
                        let title_fit = ui.fit_text(Face::SemiBold, 12.0, &item.title, tw);
                        ui.text(
                            Face::SemiBold,
                            12.0,
                            tx,
                            cy + 12.0,
                            theme.text_primary,
                            &title_fit,
                        );
                        let sub = format!(
                            "{} · {}",
                            item.source,
                            if item.slim { "Slim" } else { "Classic" }
                        );
                        let sub_fit = ui.fit_text(Face::Regular, 9.5, &sub, tw);
                        ui.text(Face::Regular, 9.5, tx, cy + 32.0, theme.text_muted, &sub_fit);

                        let note_fit = ui.fit_text(Face::Regular, 9.0, &item.subtitle, tw);
                        ui.text(Face::Regular, 9.0, tx, cy + 48.0, theme.accent, &note_fit);

                        let btn_rect = [tx, cy + card_h - 38.0, tw, 28.0];
                        button(
                            ui,
                            targets,
                            btn_rect,
                            if is_equipped { "Equipped" } else { "Equip" },
                            Action::SkinOnlineEquip(i),
                            true,
                        );
                    }
                }
                Tab::Capes => {
                    let fps = f32::from(wardrobe.cape_fps().max(1));
                    let elapsed = self.anim_origin.elapsed().as_secs_f32();
                    let visible = self.visible_capes();
                    for (slot_i, (real_idx, item)) in visible.iter().enumerate() {
                        let col = slot_i % cols;
                        let r = slot_i / cols;
                        let cx = right_x + col as f32 * (card_w + col_gap);
                        let cy = grid_top + r as f32 * (card_h + row_gap) - self.scroll;
                        if cy + card_h < grid_top - 4.0 || cy > grid_bottom + 4.0 {
                            continue;
                        }
                        let is_equipped = self
                            .last_equip_title
                            .as_deref()
                            .is_some_and(|t| t == item.title);
                        ui.rounded(
                            cx,
                            cy,
                            card_w,
                            card_h,
                            12.0,
                            if is_equipped {
                                theme.surface_selected
                            } else {
                                theme.surface_dim
                            },
                        );
                        ui.stroke_rounded(
                            cx,
                            cy,
                            card_w,
                            card_h,
                            12.0,
                            if is_equipped { 1.6 } else { 1.0 },
                            if is_equipped {
                                theme.accent
                            } else {
                                theme.border
                            },
                        );

                        // Cape miniature (10x16 aspect -> 50x80)
                        let thumb_x = cx + 10.0;
                        let thumb_y = cy + 14.0;
                        let thumb_w = 50.0;
                        let thumb_h = 80.0;
                        ui.rounded(
                            thumb_x - 3.0,
                            thumb_y - 3.0,
                            thumb_w + 6.0,
                            thumb_h + 6.0,
                            8.0,
                            theme.surface,
                        );
                        if !item.frame_runs.is_empty() {
                            let f_idx = ((elapsed * fps) as usize) % item.frame_runs.len();
                            draw_runs(
                                ui,
                                &item.frame_runs[f_idx],
                                thumb_x,
                                thumb_y,
                                thumb_w,
                                thumb_h,
                            );
                        }

                        let tx = thumb_x + thumb_w + 10.0;
                        let tw = (cx + card_w - tx - 10.0).max(50.0);
                        let title_fit = ui.fit_text(Face::SemiBold, 12.0, &item.title, tw);
                        ui.text(
                            Face::SemiBold,
                            12.0,
                            tx,
                            cy + 12.0,
                            theme.text_primary,
                            &title_fit,
                        );
                        let kind_tag = if item.is_animated {
                            format!("{} · Animated ({}f)", item.source, item.frames)
                        } else {
                            format!("{} · Plain", item.source)
                        };
                        let sub_fit = ui.fit_text(Face::Regular, 9.5, &kind_tag, tw);
                        ui.text(
                            Face::Regular,
                            9.5,
                            tx,
                            cy + 32.0,
                            if item.is_animated {
                                theme.accent
                            } else {
                                theme.text_muted
                            },
                            &sub_fit,
                        );
                        let note_fit = ui.fit_text(Face::Regular, 9.0, &item.subtitle, tw);
                        ui.text(
                            Face::Regular,
                            9.0,
                            tx,
                            cy + 48.0,
                            theme.text_muted,
                            &note_fit,
                        );

                        let btn_rect = [tx, cy + card_h - 38.0, tw, 28.0];
                        button(
                            ui,
                            targets,
                            btn_rect,
                            if is_equipped { "Equipped" } else { "Equip" },
                            Action::SkinOnlineEquip(*real_idx),
                            true,
                        );
                    }
                }
            }
        }

        // Pagination footer
        let foot_y = body_bottom - footer_h;
        button(
            ui,
            targets,
            [right_x, foot_y, 88.0, 30.0],
            "Prev Page",
            Action::SkinOnlinePage(-1),
            false,
        );
        ui.centered_fit(
            Face::Medium,
            11.5,
            [right_x + 94.0, foot_y, 120.0, 30.0],
            theme.text_muted,
            &format!("Page {}", self.page),
        );
        button(
            ui,
            targets,
            [right_x + 220.0, foot_y, 88.0, 30.0],
            "Next Page",
            Action::SkinOnlinePage(1),
            false,
        );
    }
}

fn draw_runs(
    ui: &mut Ui,
    runs: &[([f32; 4], u32)],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    for ([nx, ny, nw, nh], color) in runs {
        ui.rounded(x + nx * w, y + ny * h, nw * w, nh * h, 0.0, *color);
    }
}

fn pack_runs(pixels: &[[u8; 4]], width: usize, height: usize) -> Vec<([f32; 4], u32)> {
    let mut runs = Vec::new();
    let fw = width as f32;
    let fh = height as f32;
    for y in 0..height {
        let mut x = 0;
        while x < width {
            let c = pixels[y * width + x];
            let start = x;
            x += 1;
            while x < width && pixels[y * width + x] == c {
                x += 1;
            }
            if c[3] > 0 {
                runs.push((
                    [
                        start as f32 / fw,
                        y as f32 / fh,
                        (x - start) as f32 / fw,
                        1.0 / fh,
                    ],
                    rgba(c[0], c[1], c[2], c[3]),
                ));
            }
        }
    }
    runs
}

fn make_skin_item(
    title: String,
    subtitle: String,
    source: &'static str,
    slim: bool,
    png_bytes: Vec<u8>,
) -> Option<SkinItem> {
    let img = skins::read_skin(&png_bytes).ok()?;
    let model = if slim { Model::Slim } else { Model::Classic };
    let front = skins::front(&img, model);
    let runs = pack_runs(&front, 16, 32);
    Some(SkinItem {
        title,
        subtitle,
        source,
        slim,
        png_bytes,
        runs,
    })
}

fn make_cape_item(
    title: String,
    subtitle: String,
    source: &'static str,
    png_bytes: Vec<u8>,
) -> Option<CapeItem> {
    let (sheet, frames) = skins::read_cape_sheet(&png_bytes).ok()?;
    let preview_count = frames.min(16).max(1);
    let mut frame_runs = Vec::with_capacity(preview_count);
    for i in 0..preview_count {
        let f_idx = if frames <= preview_count {
            i
        } else {
            (i * frames) / preview_count
        };
        let frame = skins::cape_frame_64x32(&sheet, f_idx);
        let front = skins::cape_front(&frame);
        frame_runs.push(pack_runs(&front, 10, 16));
    }
    Some(CapeItem {
        title,
        subtitle,
        source,
        frames,
        is_animated: frames > 1,
        png_bytes,
        frame_runs,
    })
}

fn http_agent(url: &str) -> ureq::Agent {
    crate::proxy::agent(
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(15)),
        url,
    )
    .build()
}

fn fetch_text(url: &str) -> Result<String, String> {
    let resp = http_agent(url)
        .get(url)
        .set("User-Agent", "VoltLauncherPlus/1.0 (+https://skinmc.net)")
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    let mut body = String::new();
    resp.into_reader()
        .take(2 * 1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|e| format!("{url}: {e}"))?;
    Ok(body)
}

fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = http_agent(url)
        .get(url)
        .set("User-Agent", "VoltLauncherPlus/1.0")
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .take((MAX_REMOTE_PNG + 1) as u64)
        .read_to_end(&mut buf)
        .map_err(|e| format!("{url}: {e}"))?;
    if buf.len() > MAX_REMOTE_PNG {
        return Err("Remote PNG exceeds 2 MiB.".into());
    }
    Ok(buf)
}

fn url_encode_query(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn fetch_player_skin(username: &str) -> Option<SkinItem> {
    let clean = username.trim();
    if clean.is_empty() || clean.len() > 16 || clean.contains(' ') {
        return None;
    }
    let profile_url = format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        url_encode_query(clean)
    );
    let profile_json: serde_json::Value =
        serde_json::from_str(&fetch_text(&profile_url).ok()?).ok()?;
    let uuid = profile_json["id"].as_str()?;
    let exact_name = profile_json["name"].as_str().unwrap_or(clean);
    let sess_url = format!("https://sessionserver.mojang.com/session/minecraft/profile/{uuid}");
    let sess_json: serde_json::Value =
        serde_json::from_str(&fetch_text(&sess_url).ok()?).ok()?;
    let props = sess_json["properties"].as_array()?;
    let tex_val = props
        .iter()
        .find(|p| p["name"].as_str() == Some("textures"))?["value"]
        .as_str()?;
    let textures = horizon_client::skins::unpack(
        tex_val,
        horizon_client::skins::Signature::Unsigned,
        &horizon_client::skins::TEXTURE_URIS,
    );
    let skin_url = textures.skin?;
    let bytes = fetch_bytes(&skin_url).ok()?;
    make_skin_item(
        exact_name.to_owned(),
        "Mojang Player Profile".into(),
        "Player",
        textures.slim,
        bytes,
    )
}

fn fetch_skins_blocking(
    cat: SkinCategory,
    page: usize,
    query: &str,
) -> (Vec<SkinItem>, String) {
    let mut items = Vec::new();
    let mut seen_urls = HashSet::new();

    if !query.is_empty()
        && let Some(player_skin) = fetch_player_skin(query)
    {
        items.push(player_skin);
    }

    let list_url = if !query.is_empty() {
        let eq = url_encode_query(query);
        match cat {
            SkinCategory::Trending => {
                format!("https://skinmc.net/skins/creations?search={eq}&page={page}")
            }
            SkinCategory::Popular | SkinCategory::Latest => {
                format!("https://skinmc.net/skins?search={eq}&page={page}")
            }
        }
    } else {
        match cat {
            SkinCategory::Trending => {
                format!("https://skinmc.net/skins/creations?sort=trending&page={page}")
            }
            SkinCategory::Popular => format!("https://skinmc.net/skins?profiles=1&page={page}"),
            SkinCategory::Latest => format!("https://skinmc.net/skins?latest=1&page={page}"),
        }
    };

    let html = match fetch_text(&list_url) {
        Ok(h) => h,
        Err(e) => {
            let status = if items.is_empty() {
                format!("Could not reach SkinMC: {e}")
            } else {
                format!("Found {} player skin (SkinMC offline)", items.len())
            };
            return (items, status);
        }
    };

    let mut candidates: Vec<(String, String, &'static str)> = Vec::new();
    let re_creation = Regex::new(
        r#"skinmc\.net/(?:skins/creations|api/v1/renders/skin-creations)/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})"#,
    )
    .unwrap();
    for cap in re_creation.captures_iter(&html) {
        let uuid = &cap[1];
        let png_url = format!("https://skinmc.net/api/v1/renders/skin-creations/{uuid}/skin");
        if seen_urls.insert(png_url.clone()) {
            let title = extract_nearby_title(&html, cap.get(0).unwrap().start())
                .unwrap_or_else(|| format!("Creation {}", &uuid[..8]));
            candidates.push((title, png_url, "SkinMC"));
        }
        if candidates.len() >= 12 {
            break;
        }
    }

    if candidates.len() < 12 {
        let re_profile = Regex::new(
            r#"(?:skinmc\.net/api/v1/renders/skins/|/skin/)([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})"#,
        )
        .unwrap();
        for cap in re_profile.captures_iter(&html) {
            let uuid = &cap[1];
            let png_url = format!("https://skinmc.net/api/v1/renders/skins/{uuid}/skin");
            if seen_urls.insert(png_url.clone()) {
                let title = extract_nearby_title(&html, cap.get(0).unwrap().start())
                    .unwrap_or_else(|| format!("Skin {}", &uuid[..8]));
                candidates.push((title, png_url, "SkinMC"));
            }
            if candidates.len() >= 12 {
                break;
            }
        }
    }

    let downloaded = download_parallel(candidates, |(title, url, src)| {
        let bytes = fetch_bytes(&url).ok()?;
        make_skin_item(title, "SkinMC Community".into(), src, false, bytes)
    });
    items.extend(downloaded);

    let status = if items.is_empty() {
        "No skins returned from SkinMC for this query.".into()
    } else {
        format!("Loaded {} skins from SkinMC (page {page})", items.len())
    };
    (items, status)
}

fn fetch_capes_blocking(
    filter: CapeKindFilter,
    source: CapeSourceFilter,
    page: usize,
    query: &str,
) -> (Vec<CapeItem>, String) {
    let mut candidates: Vec<(String, String, &'static str)> = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut items = Vec::new();

    // Optional player cape lookup via api.capes.dev when user searches a single word
    if !query.is_empty() && !query.contains(' ') && query.len() <= 16 {
        let api_url = format!("https://api.capes.dev/load/{}", url_encode_query(query));
        if let Ok(text) = fetch_text(&api_url)
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(obj) = val.as_object()
        {
            for (provider, entry) in obj {
                if let Some(img_url) = entry["imageUrl"].as_str()
                    && entry["exists"].as_bool().unwrap_or(false)
                    && seen_urls.insert(img_url.to_owned())
                {
                    candidates.push((
                        format!("{query} ({provider})"),
                        img_url.to_owned(),
                        "Player",
                    ));
                }
            }
        }
    }

    // 1. KLauncher Animated Capes (category=4)
    if filter != CapeKindFilter::Plain && source != CapeSourceFilter::SkinMc {
        let kl_anim_url = format!("https://klauncher.gg/en/capes?category=4&page={page}");
        if let Ok(html) = fetch_text(&kl_anim_url) {
            let re_anim = Regex::new(
                r#"https://skin\.klaun\.ch/storage/capes/(?:anim_flat/)?([a-f0-9]{16,64})\.png"#,
            )
            .unwrap();
            let mut count = 0usize;
            for cap in re_anim.captures_iter(&html) {
                let hash = &cap[1];
                let full_url = format!("https://skin.klaun.ch/storage/capes/{hash}.png");
                if seen_urls.insert(full_url.clone()) {
                    let title = extract_nearby_title(&html, cap.get(0).unwrap().start())
                        .unwrap_or_else(|| format!("KL Animated #{}", count + 1));
                    candidates.push((title, full_url, "KLauncher"));
                    count += 1;
                    if count >= 8 {
                        break;
                    }
                }
            }
        }
    }

    // 2. KLauncher Plain Capes
    if filter != CapeKindFilter::Animated && source != CapeSourceFilter::SkinMc {
        let kl_plain_url = format!("https://klauncher.gg/en/capes?page={page}");
        if let Ok(html) = fetch_text(&kl_plain_url) {
            let re_plain = Regex::new(
                r#"https://skin\.klaun\.ch/storage/capes/([a-f0-9]{16,64})\.png"#,
            )
            .unwrap();
            let mut count = 0usize;
            for cap in re_plain.captures_iter(&html) {
                let hash = &cap[1];
                let full_url = format!("https://skin.klaun.ch/storage/capes/{hash}.png");
                if seen_urls.insert(full_url.clone()) {
                    let title = extract_nearby_title(&html, cap.get(0).unwrap().start())
                        .unwrap_or_else(|| format!("KL Cape #{}", count + 1));
                    candidates.push((title, full_url, "KLauncher"));
                    count += 1;
                    if count >= 6 {
                        break;
                    }
                }
            }
            if count == 0 {
                let re_detail = Regex::new(r#"/en/capes/(\d+)"#).unwrap();
                let mut detail_ids = HashSet::new();
                for cap in re_detail.captures_iter(&html) {
                    let id = cap[1].to_owned();
                    if detail_ids.insert(id.clone()) {
                        if let Ok(detail_html) =
                            fetch_text(&format!("https://klauncher.gg/en/capes/{id}"))
                            && let Some(m) = re_plain.captures(&detail_html)
                        {
                            let full_url = m[0].to_owned();
                            if seen_urls.insert(full_url.clone()) {
                                let title = extract_nearby_title(&detail_html, 0)
                                    .unwrap_or_else(|| format!("KL Cape #{id}"));
                                candidates.push((title, full_url, "KLauncher"));
                                count += 1;
                            }
                        }
                        if count >= 4 {
                            break;
                        }
                    }
                }
            }
        }
    }

    // 3. SkinMC Official / Community Capes
    if source != CapeSourceFilter::KLauncher && page == 1 {
        if let Ok(html) = fetch_text("https://skinmc.net/capes") {
            let re_smc = Regex::new(r#"/capes/(\d+)"#).unwrap();
            let mut smc_ids = HashSet::new();
            for cap in re_smc.captures_iter(&html) {
                let id = cap[1].to_owned();
                if smc_ids.insert(id.clone()) {
                    let dl_url = format!("https://skinmc.net/capes/{id}/download");
                    if seen_urls.insert(dl_url.clone()) {
                        let title = extract_nearby_title(&html, cap.get(0).unwrap().start())
                            .unwrap_or_else(|| format!("SkinMC Cape #{id}"));
                        candidates.push((title, dl_url, "SkinMC"));
                    }
                    if smc_ids.len() >= 8 {
                        break;
                    }
                }
            }
        }
    }

    let downloaded = download_parallel(candidates, |(title, url, src)| {
        let bytes = fetch_bytes(&url).ok()?;
        let sub = match src {
            "KLauncher" => "klauncher.gg".into(),
            "SkinMC" => "skinmc.net".into(),
            _ => "Online Cape".into(),
        };
        make_cape_item(title, sub, src, bytes)
    });
    items.extend(downloaded);

    let status = if items.is_empty() {
        "Showing built-in animated & plain capes (online catalog unavailable)".into()
    } else {
        format!(
            "Loaded {} online capes + built-in animated/plain collection",
            items.len()
        )
    };
    (items, status)
}

fn download_parallel<T: Send + 'static>(
    jobs: Vec<(String, String, &'static str)>,
    worker: impl Fn((String, String, &'static str)) -> Option<T> + Send + Sync + 'static,
) -> Vec<T> {
    if jobs.is_empty() {
        return Vec::new();
    }
    let worker = std::sync::Arc::new(worker);
    let queue = std::sync::Arc::new(std::sync::Mutex::new(jobs.into_iter().enumerate()));
    let results = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let threads = 4usize;
    let mut handles = Vec::with_capacity(threads);
    for _ in 0..threads {
        let q = queue.clone();
        let out = results.clone();
        let w = worker.clone();
        handles.push(std::thread::spawn(move || loop {
            let next = q.lock().ok().and_then(|mut g| g.next());
            let Some((idx, job)) = next else {
                break;
            };
            if let Some(item) = w(job)
                && let Ok(mut g) = out.lock()
            {
                g.push((idx, item));
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let mut vec = results.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
    vec.sort_by_key(|(idx, _)| *idx);
    vec.into_iter().map(|(_, item)| item).collect()
}

fn extract_nearby_title(html: &str, offset: usize) -> Option<String> {
    let start = offset.saturating_sub(240);
    let end = (offset + 320).min(html.len());
    let slice = html.get(start..end)?;
    for pat in ["alt=\"", "title=\"", "aria-label=\""] {
        if let Some(pos) = slice.find(pat) {
            let rest = &slice[pos + pat.len()..];
            if let Some(q) = rest.find('"') {
                let candidate = rest[..q].trim();
                if candidate.len() >= 2
                    && candidate.len() <= 40
                    && !candidate.starts_with("http")
                    && !candidate.eq_ignore_ascii_case("cape")
                    && !candidate.eq_ignore_ascii_case("skin")
                {
                    return Some(candidate.replace("&amp;", "&").replace("&#039;", "'"));
                }
            }
        }
    }
    None
}

fn encode_rgba_png(img: &image::RgbaImage) -> Option<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    img.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
    Some(cursor.into_inner())
}

fn build_builtin_capes() -> Vec<CapeItem> {
    let mut out = Vec::new();
    // 4 animated multi-frame procedural capes (64x384 -> 12 frames of 64x32)
    let anim_specs: [(&str, [u8; 3], [u8; 3]); 4] = [
        ("Crimson Ember", [195, 36, 48], [255, 186, 66]),
        ("Neon Matrix", [18, 128, 72], [92, 255, 168]),
        ("Cyber Aurora", [44, 62, 184], [120, 232, 255]),
        ("Void Rift", [72, 26, 134], [218, 96, 255]),
    ];
    for (title, base, glow) in anim_specs {
        let frames = 12u32;
        let mut sheet = image::RgbaImage::new(64, 32 * frames);
        for f in 0..frames {
            let phase = (f as f32 / frames as f32) * std::f32::consts::TAU;
            for y in 0..32u32 {
                for x in 0..64u32 {
                    let is_border = (x == 1 || x == 10 || y == 1 || y == 16) && x <= 11 && y <= 17;
                    let wave = ((y as f32 * 0.45 - phase).sin() + (x as f32 * 0.6).cos()) * 0.5;
                    let mix = ((wave + 0.5).clamp(0.0, 1.0) * 0.78)
                        + if is_border { 0.22 } else { 0.0 };
                    let r = (base[0] as f32 * (1.0 - mix) + glow[0] as f32 * mix) as u8;
                    let g = (base[1] as f32 * (1.0 - mix) + glow[1] as f32 * mix) as u8;
                    let b = (base[2] as f32 * (1.0 - mix) + glow[2] as f32 * mix) as u8;
                    sheet.put_pixel(x, f * 32 + y, image::Rgba([r, g, b, 255]));
                }
            }
        }
        if let Some(bytes) = encode_rgba_png(&sheet)
            && let Some(item) = make_cape_item(
                title.into(),
                "Built-in Animated".into(),
                "Built-in",
                bytes,
            )
        {
            out.push(item);
        }
    }

    // 4 clean static plain capes (64x32)
    let plain_specs: [(&str, [u8; 3], [u8; 3]); 4] = [
        ("Obsidian Crest", [26, 24, 38], [164, 112, 255]),
        ("Royal Gold", [36, 44, 92], [248, 202, 74]),
        ("Volt Emerald", [16, 68, 56], [78, 242, 178]),
        ("Glacier Wing", [42, 96, 148], [216, 242, 255]),
    ];
    for (title, bg, accent) in plain_specs {
        let img = image::RgbaImage::from_fn(64, 32, |x, y| {
            let cx = x as i32 - 5;
            let cy = y as i32 - 8;
            let diamond = cx.abs() + cy.abs() <= 3 && (1..=10).contains(&x) && (1..=16).contains(&y);
            let trim = (x == 1 || x == 10 || y == 1 || y == 16) && (1..=10).contains(&x) && (1..=16).contains(&y);
            let rgb = if diamond || trim { accent } else { bg };
            image::Rgba([rgb[0], rgb[1], rgb[2], 255])
        });
        if let Some(bytes) = encode_rgba_png(&img)
            && let Some(item) =
                make_cape_item(title.into(), "Built-in Static".into(), "Built-in", bytes)
        {
            out.push(item);
        }
    }
    out
}
