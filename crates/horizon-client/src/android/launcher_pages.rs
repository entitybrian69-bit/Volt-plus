use super::*;
use crate::catalog::SettingsCategory;
use horizon_client::{
    gfx::Renderer,
    osk::Output,
    ui_theme::{Icon, color},
};
use std::sync::atomic::Ordering;
use winit::{
    event::{KeyEvent, Touch},
    keyboard::{KeyCode, PhysicalKey},
};

thread_local! {
    static ACTIVE_SLIDER_DRAG: std::cell::Cell<Option<(Action, f32, f32)>> = const { std::cell::Cell::new(None) };
    static SLIDER_TRACK_TARGETS: std::cell::RefCell<Vec<([f32; 4], Action, f32, f32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Centralized dynamic theme accessors backed by `horizon_client::ui_theme::active_theme()`.
#[inline]
fn my3_bg() -> u32 {
    horizon_client::ui_theme::active_theme().bg_solid
}
#[inline]
fn my3_surface() -> u32 {
    horizon_client::ui_theme::active_theme().surface
}
#[inline]
fn my3_card_bg() -> u32 {
    horizon_client::ui_theme::active_theme().surface_dim
}
#[inline]
fn my3_card_border() -> u32 {
    color::with_alpha(horizon_client::ui_theme::active_theme().border, 0.42)
}
#[inline]
fn my3_surface_high() -> u32 {
    horizon_client::ui_theme::active_theme().surface_bright
}
#[inline]
fn my3_border() -> u32 {
    color::with_alpha(horizon_client::ui_theme::active_theme().border, 0.52)
}
#[inline]
fn my3_nav_active() -> u32 {
    horizon_client::ui_theme::active_theme().surface_selected
}
#[inline]
fn my3_accent() -> u32 {
    horizon_client::ui_theme::active_theme().accent
}
#[inline]
fn my3_accent_soft() -> u32 {
    horizon_client::ui_theme::active_theme().accent_soft
}
#[inline]
fn my3_on_accent() -> u32 {
    horizon_client::ui_theme::active_theme().on_accent
}
#[inline]
fn my3_track() -> u32 {
    horizon_client::ui_theme::active_theme().surface_dim
}
#[inline]
fn my3_pause_bg() -> u32 {
    horizon_client::ui_theme::active_theme().surface_bright
}
#[inline]
fn my3_cancel_bg() -> u32 {
    color::with_alpha(horizon_client::ui_theme::active_theme().danger, 0.28)
}
#[inline]
fn my3_text() -> u32 {
    horizon_client::ui_theme::active_theme().text_primary
}
#[inline]
fn my3_muted() -> u32 {
    horizon_client::ui_theme::active_theme().text_muted
}

/// Smooth 60fps UI animation state across navigation, pages, accordions, theme previews, progress bars, and modals.
/// Automatically respects `motion::reduced_motion()` on low-end Android hardware.
pub(crate) struct AnimState {
    pub last_tick: std::time::Instant,
    pub last_page: Page,
    pub last_category: SettingsCategory,
    pub last_has_modal: bool,
    pub nav_pill_y: f32,
    pub nav_pill_vel: f32,
    pub target_nav_y: f32,
    pub page_fade: f32,
    pub page_slide: f32,
    pub sub_tab_t: f32,
    pub toast_anim: f32,
    pub progress_smooth: f32,
    pub modal_anim: f32,
    pub shimmer_phase: f32,
    pub section_expand: [f32; 3],
    pub theme_pulse: f32,
    pub intro_start: Option<std::time::Instant>,
    pub intro_elapsed: f32,
    pub intro_done: bool,
    pub reveal_start: Option<std::time::Instant>,
    pub switches: std::collections::HashMap<String, f32>,
    pub scene_offset: [f32; 2],
    pub scene_shift: f32,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            last_tick: std::time::Instant::now(),
            last_page: Page::Play,
            last_category: SettingsCategory::Overview,
            last_has_modal: false,
            nav_pill_y: 62.0,
            nav_pill_vel: 0.0,
            target_nav_y: 62.0,
            page_fade: 1.0,
            page_slide: 0.0,
            sub_tab_t: 1.0,
            toast_anim: 1.0,
            progress_smooth: 0.0,
            modal_anim: 0.0,
            shimmer_phase: 0.0,
            section_expand: [0.0; 3],
            theme_pulse: 1.0,
            intro_start: Some(std::time::Instant::now()),
            intro_elapsed: 0.0,
            intro_done: false,
            reveal_start: None,
            switches: std::collections::HashMap::new(),
            scene_offset: [0.0; 2],
            scene_shift: 0.0,
        }
    }
}

impl AnimState {
    pub fn step(
        &mut self,
        page: Page,
        category: SettingsCategory,
        target_nav_y: f32,
        has_modal: bool,
        sections: [bool; 3],
    ) {
        let now = std::time::Instant::now();
        let intro_t0 = *self.intro_start.get_or_insert(now);
        self.intro_elapsed = now.duration_since(intro_t0).as_secs_f32();
        let raw_dt = now
            .duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(0.001, 0.08);
        self.last_tick = now;
        self.target_nav_y = target_nav_y;

        if horizon_client::ui_theme::motion::reduced_motion() {
            self.intro_done = true;
            self.last_page = page;
            self.last_category = category;
            self.last_has_modal = has_modal;
            self.nav_pill_y = target_nav_y;
            self.page_fade = 1.0;
            self.page_slide = 0.0;
            self.modal_anim = if has_modal { 1.0 } else { 0.0 };
            self.theme_pulse = 1.0;
            for i in 0..3 {
                self.section_expand[i] = if sections[i] { 1.0 } else { 0.0 };
            }
            return;
        }

        let page_changed =
            page != self.last_page || (page == Page::Settings && category != self.last_category);
        let modal_changed = has_modal != self.last_has_modal;
        self.last_has_modal = has_modal;

        if page_changed {
            self.last_page = page;
            self.last_category = category;
            self.page_fade = 0.0;
            self.page_slide = 18.0;
        }
        if modal_changed && has_modal {
            self.modal_anim = 0.0;
        }

        // Clamp first transition frame dt to 16ms so an idle gap before a tap never
        // consumes most of the entrance animation before the first frame renders.
        let dt = if page_changed || modal_changed {
            raw_dt.min(0.016)
        } else {
            raw_dt
        };

        let speed = (dt * 14.0).min(1.0);
        horizon_client::ui_theme::motion::step_spring_damper(
            &mut self.nav_pill_y,
            &mut self.nav_pill_vel,
            target_nav_y,
            380.0,
            26.0,
            dt,
        );
        self.page_fade = (self.page_fade + dt * 7.5).min(1.0);
        self.page_slide *= (1.0 - speed).max(0.0);
        self.sub_tab_t = (self.sub_tab_t + dt * 7.0).min(1.0);
        self.toast_anim = (self.toast_anim + dt * 6.5).min(1.0);
        let modal_target = if has_modal { 1.0 } else { 0.0 };
        self.modal_anim += (modal_target - self.modal_anim) * (dt * 14.0).min(1.0);
        self.theme_pulse = (self.theme_pulse + dt * 6.5).min(1.0);
        for i in 0..3 {
            let target = if sections[i] { 1.0 } else { 0.0 };
            self.section_expand[i] += (target - self.section_expand[i]) * (dt * 14.0).min(1.0);
        }
        self.shimmer_phase = (self.shimmer_phase + dt * 0.95) % 1.0;
    }

    pub fn trigger_page_transition(&mut self) {
        if !horizon_client::ui_theme::motion::reduced_motion() {
            self.last_tick = std::time::Instant::now();
            self.page_fade = 0.0;
            self.page_slide = 18.0;
        }
    }

    pub fn trigger_theme_pulse(&mut self) {
        if !horizon_client::ui_theme::motion::reduced_motion() {
            self.last_tick = std::time::Instant::now();
            self.theme_pulse = 0.0;
        }
    }

    pub fn step_switch(&mut self, key: &str, on: bool, dt: f32) -> f32 {
        if horizon_client::ui_theme::motion::reduced_motion() {
            return if on { 1.0 } else { 0.0 };
        }
        let target = if on { 1.0 } else { 0.0 };
        let entry = self.switches.entry(key.to_string()).or_insert(target);
        if (*entry - target).abs() > 0.001 {
            *entry += (target - *entry) * (dt * 16.0).min(1.0);
            if (*entry - target).abs() < 0.005 {
                *entry = target;
            }
        }
        *entry
    }

    pub fn is_active(&self) -> bool {
        if !self.intro_done || self.reveal_start.is_some_and(|t| t.elapsed().as_secs_f32() < 1.6) {
            return true;
        }
        if horizon_client::ui_theme::motion::reduced_motion() {
            return false;
        }
        self.page_fade < 0.99
            || self.page_slide.abs() > 0.35
            || (self.scene_shift - if self.last_page == Page::Play { 0.0 } else { 1.0 }).abs() > 0.005
            || (self.nav_pill_y - self.target_nav_y).abs() > 0.25
            || self.nav_pill_vel.abs() > 0.5
            || self.sub_tab_t < 0.99
            || self.toast_anim < 0.99
            || (self.modal_anim > 0.01 && self.modal_anim < 0.99)
            || self.theme_pulse < 0.99
            || self.section_expand.iter().any(|v| *v > 0.01 && *v < 0.99)
            || self.switches.values().any(|v| *v > 0.005 && *v < 0.995)
    }
}

pub(crate) struct DashboardTutorial {
    pub active: bool,
    pub started: bool,
    pub closing: bool,
    pub step: usize,
    pub alpha: f32,
    pub card_t: f32,
    pub card_vel: f32,
    pub spot_rect: [f32; 4],
    pub spot_vel: [f32; 4],
    pub pulse_t: f32,
    pub skip_rect: [f32; 4],
    pub last_tick: std::time::Instant,
}

impl Default for DashboardTutorial {
    fn default() -> Self {
        Self {
            active: false,
            started: false,
            closing: false,
            step: 0,
            alpha: 0.0,
            card_t: 0.0,
            card_vel: 0.0,
            spot_rect: [0.0; 4],
            spot_vel: [0.0; 4],
            pulse_t: 0.0,
            skip_rect: [0.0; 4],
            last_tick: std::time::Instant::now(),
        }
    }
}

impl DashboardTutorial {
    pub const STEP_COUNT: usize = 15;

    pub fn is_animating(&self) -> bool {
        self.active
    }

    pub fn start(&mut self, initial_spot: [f32; 4]) {
        self.active = true;
        self.started = true;
        self.closing = false;
        self.step = 0;
        self.alpha = if horizon_client::ui_theme::motion::reduced_motion() { 1.0 } else { 0.02 };
        self.card_t = if horizon_client::ui_theme::motion::reduced_motion() { 1.0 } else { 0.0 };
        self.card_vel = 0.0;
        self.spot_rect = initial_spot;
        self.spot_vel = [0.0; 4];
        self.pulse_t = 0.0;
        self.last_tick = std::time::Instant::now();
    }

    pub fn step_motion(&mut self, target_spot: [f32; 4]) {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32().clamp(0.001, 0.05);
        self.last_tick = now;
        if horizon_client::ui_theme::motion::reduced_motion() {
            self.spot_rect = target_spot;
            self.spot_vel = [0.0; 4];
            self.card_t = 1.0;
            self.card_vel = 0.0;
            if self.closing {
                self.alpha = 0.0;
                self.active = false;
                self.closing = false;
            } else {
                self.alpha = 1.0;
            }
            return;
        }
        self.pulse_t = (self.pulse_t + dt) % 62.8318;
        if self.closing {
            self.alpha = (self.alpha - dt * 5.2).max(0.0);
            if self.alpha <= 0.005 {
                self.alpha = 0.0;
                self.active = false;
                self.closing = false;
            }
        } else {
            self.alpha = (self.alpha + dt * 4.4).min(1.0);
        }
        self.card_t = (self.card_t + dt * 3.8).min(1.0);
        for i in 0..4 {
            horizon_client::ui_theme::motion::step_spring_damper(
                &mut self.spot_rect[i],
                &mut self.spot_vel[i],
                target_spot[i],
                260.0,
                24.0,
                dt,
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    Page(Page),
    SceneGyro(i8),
    OpenWallpapers,
    CloseWallpapers,
    DownloadWallpaper(usize),
    EquipWallpaper(usize),
    ClearWallpaper,
    UploadCustomWallpaper,
    UiOpacityDelta(i8),
    SettingsCat(SettingsCategory),
    Play,
    PauseInstall,
    CancelInstall,
    OpenRoadGame,
    CloseRoadGame,
    RoadMove(i8, i8),
    RestartRoadGame,
    PauseCatalogDownload,
    CancelCatalogDownload,
    Search,
    ShaderSource(crate::shader_catalog::Origin),
    ModSource(crate::shader_catalog::Origin),
    SetCompilerMode(crate::catalog::CompilerMode),
    CompilerRunQuery,
    CompilerPortLocal(usize),
    Field(&'static str),
    FinishNameEdit,
    Install(usize),
    OpenProject(usize),
    OpenBundledProject(&'static str),
    CloseProjectModal,
    SelectModalVersion(usize),
    InstallModalProject,
    Import(horizon_client::content_files::Kind),
    OpenFolder(horizon_client::content_files::Kind),
    ToggleSection(usize),
    ToggleLocal(usize),
    DeleteLocal(usize),
    ConfirmDeleteLocal,
    ConfirmExit,
    Next,
    Previous,
    Profile,
    Logs,
    RecordPerformance,
    ExportPerformance,
    ExportDiagnostics,
    Account,
    AddAccount,
    SignIn,
    ElySignIn,
    MicrosoftAccount,
    OfflineAccount,
    SubmitEly,
    SavedAccounts,
    SavedPage(bool),
    SelectAccount(usize),
    SelectOffline,
    WardrobeKind(bool),
    WardrobeSelect(usize),
    WardrobeImport,
    WardrobeCloud,
    WardrobeUpload,
    WardrobeActivateCape(usize),
    WardrobeUseCloud,
    WardrobeModel,
    WardrobeRotate(i8),
    WardrobeZoom(bool),
    WardrobeHideCape,
    DeviceSignIn,
    CopyLoginCode,
    CancelLogin,
    ReviewApp,
    Repository,
    Discord,
    SignOut,
    LoginLink,
    CloseLogin,
    Renderer,
    RenderScale,
    ChooseRenderScale(crate::display::RenderScale),
    UiScale,
    ChooseUiScale(u32),
    Orientation,
    InputHost,
    ChooseInputHost(crate::catalog::InputHost),
    ChooseOrientation(crate::catalog::Orientation),
    ChooseRenderer(crate::catalog::RendererMode),
    ChooseProfile(u32),
    Dismiss,
    Immersive,
    Cutout,
    KeepScreenOn,
    Aspect,
    ChooseAspect(crate::display::Aspect),
    DisplayHz,
    ChooseDisplayHz(u16),
    CacheBudget,
    ChooseCacheBudget(u32),
    ToggleBundled(&'static str),
    WorldFiles,
    ScreenshotFiles,
    GameFiles,
    WorkerLimit,
    ChooseWorkerLimit(u32),
    PerformanceHints,
    GpuProfile,
    // Video settings actions (options.txt)
    RenderScaleDelta(i32),
    // Replay Mod browser & settings
    SetReplaysSubTab(crate::catalog::ReplaysSubTab),
    PlayReplay(usize),
    DeleteReplay(usize),
    PlayRenderedReplay(usize),
    DeleteRenderedReplay(usize),
    OpenRawReplaysFolder,
    OpenRenderedReplaysFolder,
    RefreshReplays,
    ReplayToggleAutoSp,
    ReplayToggleAutoMp,
    ReplayToggleRecordSelf,
    // Inventory (World / Server / Screenshot) & CurseForge Worlds
    SetInventorySubTab(crate::catalog::InventorySubTab),
    RefreshWorlds,
    PlayWorld(usize),
    InspectWorld(usize),
    CloseWorldInspection,
    DeleteWorld(usize),
    ImportWorldZip,
    ImportWorldFolder,
    JoinServer(usize),
    DeleteServer(usize),
    AddServer,
    RefreshScreenshots,
    ExportScreenshot(usize),
    DeleteScreenshot(usize),
    OpenScreenshotsFolder,
    SearchDiscoverWorlds,
    InstallDiscoverWorld(usize),
    RenderDistDelta(i32),
    SimDistDelta(i32),
    CycleMaxFps,
    CycleGraphicsPreset,
    ToggleVsync,
    ToggleSmoothLighting,
    CycleClouds,
    ToggleMenuBlur,
    FovDelta(f32),
    // Animation & Diagnostics Modal actions
    ToggleLauncherAnimations,
    ToggleGameAnimations,
    CloseDiagnosticsSummary,
    OpenReportsFolder,
    // Experimental settings actions
    ToggleSuperPerf,
    ToggleBatteryPerf,
    ToggleMaxFpsMode,
    ToggleZeroLatencyTouch,
    ToggleFastChunkUpload,
    // Controls & Control Editor actions
    OpenControlEditor,
    CloseControlEditor,
    ControlProfileChooser,
    ChooseControlProfile(u32),
    SaveControlProfileAsNew,
    ExportControlProfile,
    ImportControlProfile,
    OpenExperimentalZalithStudio,
    MouseSensDelta(f32),
    TouchSensDelta(f32),
    VirtualMouseSpeedDelta(f32),
    ButtonScaleDelta(f32),
    ToggleControlScheme,
    SelectControlMode(horizon_client::touch::ControlStyle),
    ActivateStudioPreset(horizon_client::touch::ControlStyle, u32),
    DeleteStudioPreset(horizon_client::touch::ControlStyle, u32),
    CopyActivePreset,
    EditActivePresetMeta,
    ConfirmReplaceDuplicatePreset,
    CancelReplaceDuplicatePreset,
    ToggleSplitControls,
    ToggleSprintLatch,
    ToggleLeftHanded,
    ToggleInvertY,
    ToggleGyro,
    GyroSensDelta(f32),
    ToggleVirtualMouseEnabled,
    ToggleOverlayButton,
    PointerSizeDelta(f32),
    PointerOpacityDelta(f32),
    SelectCursorIcon(horizon_client::touch::CursorIcon),
    ResetMouseCustomization,
    CycleJoystickZone,
    ToggleTouchVibrate,
    ToggleAutoJump,
    ToggleInventoryControls,
    // Theme customization actions (Settings -> Launcher Settings -> Theme)
    SelectThemePreset(horizon_client::ui_theme::ThemePreset),
    SetThemeAppearance(horizon_client::ui_theme::ThemeAppearance),
    SetThemeBackgroundMode(horizon_client::ui_theme::BackgroundMode),
    SetGradientDirection(horizon_client::ui_theme::GradientDirection),
    ToggleGradientThreeStops,
    StepGradientMidPos(i8),
    SelectAccentSwatch(u32),
    SelectBgSwatch(u32),
    SelectGradStartSwatch(u32),
    SelectGradEndSwatch(u32),
    ResetTheme,
    // Intro & Installed / File Manager actions
    SkipIntro,
    SetInstalledSubTab(crate::catalog::InstalledSubTab),
    SetInstalledFilter(crate::catalog::InstalledFilter),
    CycleInstalledSort,
    ExportLocal(usize),
    FmNavigateRoot(&'static str),
    FmBreadcrumb(usize),
    FmBack,
    FmForward,
    FmUp,
    FmOpenEntry(usize),
    FmToggleSelect(usize),
    FmSelectAll,
    FmClearSelection,
    FmCopy,
    FmCut,
    FmPaste,
    FmImportHere,
    FmImportFolderHere,
    FmOpenNewFolder,
    FmSubmitNewFolder,
    FmOpenRename(usize),
    FmOpenRenameSelected,
    FmSubmitRename,
    FmRequestDeleteSingle(usize),
    FmRequestDeleteSelected,
    FmConfirmDelete,
    FmOpenDetails(usize),
    FmExportSingle(usize),
    FmExportSelected,
    FmCloseModal,
    OpenUpdateScreen,
    CloseUpdateScreen,
    TriggerUpdateInstall,
    CheckForUpdates,
    OpenMusic,
    CloseMusic,
    MusicTab(bool),
    MusicSearch,
    MusicQuickQuery(&'static str),
    MusicSelectGenre(&'static str),
    MusicOpenSort,
    MusicCloseSort,
    MusicSetSort(horizon_client::internet_music::MusicSort),
    MusicPrevPage,
    MusicNextPage,
    MusicDownloadResult(usize),
    MusicOpenDownloads,
    MusicCloseDownloads,
    MusicClearFinishedDownloads,
    MusicPlayResult(usize),
    MusicPlayPlaylistTrack(usize),
    MusicSelectPlaylist(usize),
    MusicPlaySelectedPlaylist,
    MusicDeleteSelectedPlaylist,
    MusicRemoveTrackFromPlaylist(usize),
    MusicOpenSaveResult(usize),
    MusicOpenSavePlaylistTrack(usize),
    OpenSkinOnline,
    CloseSkinOnline,
    SkinOnlineTab(bool),
    SkinOnlineSkinCategory(super::skin_online::SkinCategory),
    SkinOnlineCapeFilter(super::skin_online::CapeKindFilter),
    SkinOnlineCapeSource(super::skin_online::CapeSourceFilter),
    SkinOnlineSearch,
    SkinOnlineRefresh,
    SkinOnlinePage(isize),
    SkinOnlineEquip(usize),
    SkinOnlineToggleWavey,
    SkinOnlineCycleFps,
    MusicCloseSave,
    MusicAddToPlaylist(usize),
    MusicCreatePlaylist,
    MusicCreateAndAdd,
    MusicTogglePause,
    MusicNext,
    MusicPrev,
    MusicSeekBy(i64),
    MusicSeekFrac(f32),
    MusicToggleMute,
    MusicStepVolume(f32),
    MusicToggleDisableVanilla,
}

pub(super) fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
) {
    let [x, y, w, h] = rect;
    let radius = (h * 0.22).clamp(6.0, 10.0);
    ui.rounded(
        x,
        y,
        w,
        h,
        radius,
        if accent {
            my3_accent()
        } else {
            my3_surface_high()
        },
    );
    if !accent {
        ui.border(x, y, w, h, radius, my3_border());
    }
    let padding = (w * 0.055).clamp(4.0, 9.0);
    let avail_w = (w - 2.0 * padding).max(16.0);
    let mut font = if w < 90.0 {
        12.5
    } else if h <= 36.0 {
        13.5
    } else {
        14.5
    };
    while font > 9.5 && ui.measure(Face::Medium, font, label) > avail_w {
        font -= 0.5;
    }
    let fitted = ui.fit_text(Face::Medium, font, label, avail_w);
    let text_w = ui.measure(Face::Medium, font, &fitted);
    let left_align = matches!(action, Action::Field(_)) || label.ends_with(" v");
    let tx = if left_align {
        x + padding
    } else {
        x + ((w - text_w) * 0.5).max(padding)
    };
    let ty = y + ((h - font * 1.45) * 0.5).max(2.0);
    ui.text(
        Face::Medium,
        font,
        tx,
        ty,
        if accent { my3_on_accent() } else { my3_text() },
        &fitted,
    );
    if let Some(c) = ui.clip_rect(rect) {
        targets.push((c, action));
    }
}

fn toggle(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    on: bool,
    action: Action,
    switch_t: f32,
) {
    let [x, y, w, h] = rect;
    ui.text(
        Face::Medium,
        13.5,
        x,
        y + (h - 20.0) * 0.5,
        if on { my3_accent() } else { my3_muted() },
        if on { "Enabled" } else { "Off" },
    );
    let track_x = x + w - 52.0;
    let track_y = y + (h - 28.0) * 0.5;
    ui.m3_switch_animated(track_x, track_y, switch_t);
    if let Some(c) = ui.clip_rect([x, y, w, h]) {
        targets.push((c, action));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_setting_card(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    list_targets: &mut Vec<([f32; 4], Action)>,
    anim: &mut AnimState,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    title: &str,
    subtitle: &str,
    right_control: SettingControl<'_>,
) {
    if let SettingControl::SectionHeader(badge, icon) = right_control {
        ui.rounded(x, y + 6.0, w, h - 10.0, 10.0, my3_surface());
        ui.border(x, y + 6.0, w, h - 10.0, 10.0, my3_border());
        ui.rounded(x + 12.0, y + 13.0, 34.0, 34.0, 8.0, my3_accent_soft());
        ui.icon(icon, x + 20.0, y + 21.0, 18.0, my3_accent());
        let text_max = (w - 170.0).max(90.0);
        let fitted_title = ui.fit_text(Face::SemiBold, 13.5, title, text_max);
        ui.text(
            Face::SemiBold,
            13.5,
            x + 56.0,
            y + 12.0,
            my3_accent(),
            &fitted_title,
        );
        let fitted_sub = ui.fit_text(Face::Regular, 11.5, subtitle, text_max);
        ui.text(
            Face::Regular,
            11.5,
            x + 56.0,
            y + 31.0,
            my3_muted(),
            &fitted_sub,
        );
        let chip_w = (ui.measure(Face::SemiBold, 10.5, badge) + 22.0).clamp(64.0, 124.0);
        let chip_x = x + w - chip_w - 14.0;
        let chip_y = y + (h - 24.0) * 0.5 + 1.0;
        ui.rounded(chip_x, chip_y, chip_w, 24.0, 6.0, my3_accent_soft());
        ui.border(chip_x, chip_y, chip_w, 24.0, 6.0, my3_accent());
        ui.centered_fit(
            Face::SemiBold,
            10.0,
            [chip_x + 4.0, chip_y, chip_w - 8.0, 24.0],
            my3_accent(),
            badge,
        );
        return;
    }

    let is_active_card = matches!(
        right_control,
        SettingControl::Toggle(true, _)
            | SettingControl::Button(_, _, true)
            | SettingControl::TextField { active: true, .. }
    );
    ui.rounded(
        x,
        y + 1.0,
        w,
        (h - 2.0).max(24.0),
        10.0,
        if is_active_card {
            my3_surface_high()
        } else {
            my3_card_bg()
        },
    );
    ui.border(
        x,
        y + 1.0,
        w,
        (h - 2.0).max(24.0),
        10.0,
        if is_active_card {
            my3_accent()
        } else {
            my3_card_border()
        },
    );
    if is_active_card {
        ui.rounded(
            x + 4.0,
            y + 12.0,
            3.5,
            (h - 24.0).max(12.0),
            1.8,
            my3_accent(),
        );
    }

    let text_max = (w - 236.0).max(90.0);
    let fitted_title = ui.fit_text(Face::SemiBold, 15.0, title, text_max);
    ui.text(
        Face::SemiBold,
        15.0,
        x + 16.0,
        y + 10.0,
        my3_text(),
        &fitted_title,
    );
    let fitted_sub = ui.fit_text(Face::Regular, 12.5, subtitle, text_max);
    ui.text(
        Face::Regular,
        12.5,
        x + 16.0,
        y + 32.0,
        my3_muted(),
        &fitted_sub,
    );

    match right_control {
        SettingControl::SectionHeader(..) => {}
        SettingControl::Toggle(on, action) => {
            let rect = [x + w - 122.0, y + (h - 38.0) * 0.5, 108.0, 38.0];
            list_targets.push((rect, action));
            let switch_t = anim.step_switch(title, on, 0.016);
            toggle(ui, targets, rect, on, action, switch_t);
        }
        SettingControl::Button(label, action, accent) => {
            let bw = 176.0f32.min(w * 0.44);
            let rect = [x + w - bw - 14.0, y + (h - 38.0) * 0.5, bw, 38.0];
            list_targets.push((rect, action));
            button(ui, targets, rect, label, action, accent);
        }
        SettingControl::TextField {
            label,
            value,
            hint,
            active,
            action,
        } => {
            let bw = 186.0f32.min(w * 0.46);
            let fh = if label.is_empty() { 36.0 } else { 42.0 };
            let rect = ui.text_field(
                x + w - bw - 14.0,
                y + (h - fh) * 0.5,
                bw,
                fh,
                label,
                value,
                hint,
                active,
            );
            list_targets.push((rect, action));
            if let Some(c) = ui.clip_rect(rect) {
                targets.push((c, action));
            }
        }
        SettingControl::Stepper(value_label, minus_action, plus_action) => {
            let right_x = x + w - 14.0;
            let plus_rect = [right_x - 36.0, y + (h - 36.0) * 0.5, 36.0, 36.0];
            let val_w = 88.0;
            let val_x = plus_rect[0] - val_w - 6.0;
            let minus_rect = [val_x - 36.0 - 6.0, y + (h - 36.0) * 0.5, 36.0, 36.0];

            ui.rounded(
                val_x,
                y + (h - 36.0) * 0.5,
                val_w,
                36.0,
                12.0,
                my3_surface(),
            );
            ui.border(val_x, y + (h - 36.0) * 0.5, val_w, 36.0, 12.0, my3_border());
            let fitted_v = ui.fit_text(Face::SemiBold, 13.5, value_label, val_w - 10.0);
            let tw = ui.measure(Face::SemiBold, 13.5, &fitted_v);
            ui.text(
                Face::SemiBold,
                13.5,
                val_x + (val_w - tw) * 0.5,
                y + (h - 20.0) * 0.5,
                my3_accent(),
                &fitted_v,
            );
            list_targets.push((minus_rect, minus_action));
            list_targets.push((plus_rect, plus_action));
            button(ui, targets, minus_rect, "-", minus_action, false);
            button(ui, targets, plus_rect, "+", plus_action, true);
        }
        SettingControl::Slider {
            value_label,
            fraction,
            minus_action,
            plus_action,
        } => {
            let right_x = x + w - 14.0;
            let plus_rect = [right_x - 34.0, y + (h - 34.0) * 0.5, 34.0, 34.0];
            let track_w = 112.0f32.min((w * 0.26).max(76.0));
            let track_x = plus_rect[0] - track_w - 6.0;
            let minus_rect = [track_x - 34.0 - 6.0, y + (h - 34.0) * 0.5, 34.0, 34.0];

            let pill_y = y + (h - 36.0) * 0.5;
            ui.rounded(track_x, pill_y, track_w, 36.0, 12.0, my3_surface());
            ui.border(track_x, pill_y, track_w, 36.0, 12.0, my3_border());

            // M3 Expressive Value Label + Linear Track Bar with Active Thumb
            let fitted_v = ui.fit_text(Face::SemiBold, 12.0, value_label, track_w - 12.0);
            let tw = ui.measure(Face::SemiBold, 12.0, &fitted_v);
            ui.text(
                Face::SemiBold,
                12.0,
                track_x + (track_w - tw) * 0.5,
                pill_y + 4.0,
                my3_accent(),
                &fitted_v,
            );

            let bar_x = track_x + 10.0;
            let bar_w = (track_w - 20.0).max(24.0);
            let bar_y = pill_y + 23.0;
            let t = fraction.clamp(0.0, 1.0);
            let is_dragging = ACTIVE_SLIDER_DRAG.with(|c| {
                c.get().is_some_and(|(_, bx, _)| (bx - bar_x).abs() < 1.0)
            });
            ui.m3_expressive_slider(bar_x, bar_y, bar_w, 6.0, t, is_dragging);

            let left_half = [track_x, pill_y, track_w * 0.5, 36.0];
            let right_half = [track_x + track_w * 0.5, pill_y, track_w * 0.5, 36.0];
            if let Some(c) = ui.clip_rect([track_x, pill_y, track_w, 36.0]) {
                SLIDER_TRACK_TARGETS.with(|t| t.borrow_mut().push((c, minus_action, bar_x, bar_w)));
            }
            if let Some(c) = ui.clip_rect(left_half) {
                targets.push((c, minus_action));
            }
            if let Some(c) = ui.clip_rect(right_half) {
                targets.push((c, plus_action));
            }
            list_targets.push((left_half, minus_action));
            list_targets.push((right_half, plus_action));
            list_targets.push((minus_rect, minus_action));
            list_targets.push((plus_rect, plus_action));
            button(ui, targets, minus_rect, "-", minus_action, false);
            button(ui, targets, plus_rect, "+", plus_action, true);
        }
    }
}

enum SettingControl<'a> {
    SectionHeader(&'a str, Icon),
    Toggle(bool, Action),
    Button(&'a str, Action, bool),
    TextField {
        label: &'a str,
        value: &'a str,
        hint: &'a str,
        active: bool,
        action: Action,
    },
    Stepper(&'a str, Action, Action),
    Slider {
        value_label: &'a str,
        fraction: f32,
        minus_action: Action,
        plus_action: Action,
    },
}

fn page_icon(page: Page) -> Icon {
    match page {
        Page::Play => Icon::Play,
        Page::Worlds | Page::Inventory => Icon::Folder,
        Page::Mods => Icon::Mods,
        Page::Packs => Icon::Packs,
        Page::ControlsStudio => Icon::Gamepad,
        Page::Shaders => Icon::Shaders,
        Page::Compiler => Icon::Terminal,
        Page::Installed => Icon::Installed,
        Page::Replays => Icon::Play,
        Page::Settings => Icon::Settings,
        Page::Skins => Icon::Skins,
        Page::Music => Icon::Music,
        Page::SkinOnline => Icon::Skins,
    }
}

fn catalog_fallback_badge(id: &str, default_icon: Icon) -> (Icon, u32, u32) {
    use horizon_client::ui_theme::color;
    let lower = id.to_ascii_lowercase();
    if lower == "zoom" || lower.contains("zoom") {
        return (
            Icon::Search,
            color::with_alpha(color::hex(0x6366F1, 255), 0.28),
            color::hex(0xA5B4FC, 255),
        );
    }
    if lower == "minimap" || lower.contains("map") || lower.contains("coords") {
        return (
            Icon::Crosshair,
            color::with_alpha(color::hex(0x0EA5E9, 255), 0.28),
            color::hex(0x7DD3FC, 255),
        );
    }
    if lower == "recipes" || lower.contains("recipe") || lower.contains("rei") || lower.contains("emi") || lower.contains("jei") {
        return (
            Icon::FileText,
            color::with_alpha(color::hex(0x6366F1, 255), 0.26),
            color::hex(0xC4B5FD, 255),
        );
    }
    if lower == "smartf3" || lower.contains("f3") || lower.contains("debug") {
        return (
            Icon::Terminal,
            color::with_alpha(color::hex(0x10B981, 255), 0.26),
            color::hex(0x6EE7B7, 255),
        );
    }
    if lower.contains("wrench") || lower.contains("config") || lower.contains("tweak") || lower.contains("menu") {
        return (
            Icon::Gear,
            color::with_alpha(color::hex(0xF59E0B, 255), 0.24),
            color::hex(0xFCD34D, 255),
        );
    }
    if lower.contains("fps") || lower.contains("hud") || lower.contains("sodium") || lower.contains("lithium") || lower.contains("ferrite") {
        return (
            Icon::Monitor,
            color::with_alpha(color::hex(0x3B82F6, 255), 0.25),
            color::hex(0x93C5FD, 255),
        );
    }
    if lower.contains("keystroke") || lower.contains("sprint") || lower.contains("sneak") {
        return (
            Icon::Gamepad,
            color::with_alpha(color::hex(0x8B5CF6, 255), 0.25),
            color::hex(0xDDD6FE, 255),
        );
    }
    if lower.contains("mouse") || lower.contains("scroll") {
        return (
            Icon::Mouse,
            color::with_alpha(color::hex(0xEC4899, 255), 0.24),
            color::hex(0xF9A8D4, 255),
        );
    }
    if lower.contains("apple") || lower.contains("potion") || lower.contains("effect") || lower.contains("jade") {
        return (
            Icon::Flask,
            color::with_alpha(color::hex(0x14B8A6, 255), 0.25),
            color::hex(0x5EEAD4, 255),
        );
    }
    let hash = id
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    const PALETTE: [u32; 6] = [0x6366F1, 0x0EA5E9, 0x10B981, 0xF59E0B, 0x8B5CF6, 0xEC4899];
    let rgb = PALETTE[(hash as usize) % PALETTE.len()];
    (
        default_icon,
        color::with_alpha(color::hex(rgb, 255), 0.24),
        color::hex(rgb, 255),
    )
}

fn icon_action_button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    bg: u32,
    icon: Icon,
    action: Action,
) {
    let [x, y, w, h] = rect;
    ui.rounded(x, y, w, h, 8.0, bg);
    ui.border(x, y, w, h, 8.0, my3_border());
    let icon_size = 18.0;
    ui.icon(
        icon,
        x + (w - icon_size) * 0.5,
        y + (h - icon_size) * 0.5,
        icon_size,
        my3_text(),
    );
    targets.push((rect, action));
}

#[allow(clippy::too_many_arguments)]
fn draw_my3_progress_view(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    content_x: f32,
    content_w: f32,
    h: f32,
    title: &str,
    subtitle_left: &str,
    subtitle_right: &str,
    speed_label: &str,
    eta_label: &str,
    step_index: usize,
    steps: &[&str],
    fraction: f32,
    shimmer_phase: f32,
    stage_text: &str,
    paused: bool,
    pause_action: Action,
    cancel_action: Action,
) {
    use horizon_client::ui_theme::GradientDirection;
    let progress = fraction.clamp(0.0, 1.0);
    let cyan = color::hex(0x4cdefa, 255);
    let violet = color::hex(0xaa67f8, 255);
    let dim = color::hex(0x0c1829, 205);
    let line = color::hex(0x96b5d4, 140);
    let y = (h * 0.12).clamp(54.0, 76.0);
    let card_h = (h - y - 10.0).max(125.0).min(300.0);
    let k = (card_h / 250.0).min(1.0);
    let pad = 16.0 * k;
    let x = content_x;
    let w = content_w;
    let ix = x + pad;
    let iw = (w - pad * 2.0).max(80.0);
    // Layered translucent glass, with the background still visible through it.
    ui.rounded(x - 2.0, y - 2.0, w + 4.0, card_h + 4.0, 20.0 * k, color::hex(0x6da4ed, 32));
    ui.rounded(x, y, w, card_h, 18.0 * k, color::hex(0x111c30, 215));
    ui.stroke_rounded(x, y, w, card_h, 18.0 * k, 1.2, line);
    ui.fill(x + 22.0 * k, y + 1.0, (w - 44.0 * k).max(0.0), 1.0, color::hex(0xd6eafa, 110));

    let badge = 43.0 * k;
    let top = y + pad;
    ui.rounded(ix, top, badge, badge, 10.0 * k, dim);
    ui.stroke_rounded(ix, top, badge, badge, 10.0 * k, 1.3, cyan);
    ui.icon(if paused { Icon::Pause } else { Icon::Packs }, ix + 10.0*k, top + 10.0*k, 23.0*k, my3_text());
    let pct_w = 60.0*k;
    let pct_x = x + w - pad - pct_w;
    ui.rounded(pct_x, top + 4.0*k, pct_w, 33.0*k, 15.0*k, dim);
    ui.stroke_rounded(pct_x, top + 4.0*k, pct_w, 33.0*k, 15.0*k, 1.5, violet);
    ui.centered_fit(Face::SemiBold, 15.0*k, [pct_x, top + 4.0*k, pct_w, 33.0*k], my3_text(), &format!("{:.0}%", progress * 100.0));
    let tx = ix + badge + 13.0*k;
    let tw = (pct_x - tx - 8.0*k).max(10.0);
    let heading = ui.fit_text(Face::SemiBold, 20.0*k, title, tw);
    ui.text(Face::SemiBold, 20.0*k, tx, top + 2.0*k, my3_text(), &heading);
    let sub = ui.fit_text(Face::Regular, 12.5*k, subtitle_left, tw);
    ui.text(Face::Regular, 12.5*k, tx, top + 27.0*k, my3_muted(), &sub);

    let bar_y = top + 53.0*k;
    let bar_h = 11.0*k;
    let value_w = (ui.measure(Face::Medium, 10.5*k, subtitle_right) + 10.0*k).min(iw * 0.38);
    let bar_w = (iw - value_w - 6.0*k).max(20.0);
    ui.rounded(ix, bar_y, bar_w, bar_h, bar_h * 0.5, dim);
    ui.stroke_rounded(ix, bar_y, bar_w, bar_h, bar_h * 0.5, 1.0, line);
    let fill_w = (bar_w - 2.0).max(0.0) * progress;
    if fill_w > 1.0 {
        ui.fill_gradient(ix + 1.0, bar_y + 1.0, fill_w, (bar_h - 2.0).max(1.0), GradientDirection::Horizontal, violet, cyan, None, 24);
    }
    let right = ui.fit_text(Face::Medium, 10.5*k, subtitle_right, value_w);
    ui.text(Face::Medium, 10.5*k, ix + bar_w + 7.0*k, bar_y, my3_text(), &right);
    let details_y = bar_y + 17.0*k;
    let detail = if paused { format!("Paused  •  {stage_text}") }
        else { format!("{stage_text}  •  {speed_label}  •  {eta_label}") };
    let detail = ui.fit_text(Face::Regular, 11.0*k, &detail, iw);
    ui.text(Face::Regular, 11.0*k, ix, details_y, my3_muted(), &detail);

    // A linked timeline with true completed/current/future step states.
    let timeline_y = details_y + 20.0*k;
    let timeline_h = 64.0*k;
    ui.rounded(ix, timeline_y, iw, timeline_h, 12.0*k, dim);
    ui.border(ix, timeline_y, iw, timeline_h, 12.0*k, line);
    if !steps.is_empty() {
        let step_w = iw / steps.len() as f32;
        let cy = timeline_y + 21.0*k;
        for (i, name) in steps.iter().enumerate() {
            let cx = ix + step_w * (i as f32 + 0.5);
            if i > 0 {
                ui.fill(cx - step_w + 11.0*k, cy - 1.0*k, (step_w - 22.0*k).max(0.0), 2.0*k,
                    if i <= step_index { violet } else { line });
            }
            let active = i == step_index;
            let complete = i < step_index;
            ui.rounded(cx - 11.0*k, cy - 11.0*k, 22.0*k, 22.0*k, 11.0*k, dim);
            ui.stroke_rounded(cx - 11.0*k, cy - 11.0*k, 22.0*k, 22.0*k, 11.0*k, 1.6, if active { cyan } else if complete { violet } else { line });
            let icon = if complete { Icon::Check } else { match *name {
                "Manifest" | "Resolve" => Icon::FileText,
                "Metadata" => Icon::Terminal,
                "Client JAR" | "Download" => Icon::Packs,
                "Verify" => Icon::Search,
                "Extract" | "Install" => Icon::Folder,
                _ => Icon::Play,
            }};
            ui.icon(icon, cx - 6.0*k, cy - 6.0*k, 12.0*k, if active || complete { my3_text() } else { my3_muted() });
            ui.centered_fit(Face::Medium, 10.0*k, [ix + i as f32*step_w + 2.0*k, timeline_y + 40.0*k, (step_w - 4.0*k).max(1.0), 17.0*k], my3_text(), name);
        }
    }
    let by = y + card_h - 45.0*k;
    let bh = 32.0*k;
    let cw = 92.0*k;
    let pw = 104.0*k;
    let cx = ix + iw - cw;
    let px = cx - pw - 9.0*k;
    for (rect, label, icon, fill, border, action) in [
        ([px, by, pw, bh], if paused { "Resume" } else { "Pause" }, if paused { Icon::Play } else { Icon::Pause }, color::hex(0x373b92, 225), cyan, pause_action),
        ([cx, by, cw, bh], "Cancel", Icon::Close, color::hex(0x792b5b, 225), color::hex(0xf47fb8, 255), cancel_action),
    ] {
        ui.rounded(rect[0], rect[1], rect[2], rect[3], 10.0*k, fill);
        ui.stroke_rounded(rect[0], rect[1], rect[2], rect[3], 10.0*k, 1.2, border);
        ui.icon(icon, rect[0] + 12.0*k, rect[1] + 9.0*k, 14.0*k, my3_text());
        ui.text(Face::SemiBold, 12.0*k, rect[0] + 32.0*k, rect[1] + 8.0*k, my3_text(), label);
        targets.push((rect, action));
    }
    let _ = shimmer_phase;

}

/// Original geometric art: a lane-crossing game, not Crossy Road assets.
fn draw_cross_road(ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>, road: &super::cross_road::RoadGame) {
    let (w, h) = (ui.w, ui.h);
    ui.fill(0.0, 0.0, w, h, color::hex(0x102a2b, 255));
    let tile = (h / 9.0).min(w / 13.0).max(10.0);
    let board_w = tile * 9.0;
    let bx = (w - board_w) * 0.5;
    let by = 34.0;
    for i in 0..8 {
        let row = (road.row + 5 - i).max(0);
        let y = by + i as f32 * tile;
        let pavement = super::cross_road::RoadGame::road(row);
        ui.fill(bx, y, board_w, tile, if pavement { color::hex(0x343e4b, 255) } else { color::hex(0x477c58, 255) });
        if pavement {
            for d in 0..5 { ui.fill(bx + (d as f32 * 2.0 + 0.4) * tile, y + tile*0.47, tile*0.8, tile*0.05, color::hex(0xf2d696, 135)); }
            for car in 0..2 {
                let cx = road.car_x(row, car);
                if (-0.8..9.0).contains(&cx) {
                    let left = (bx + cx * tile).max(bx);
                    let right = (bx + (cx + 0.8) * tile).min(bx + board_w);
                    if right > left {
                        ui.rounded(left, y + tile*0.14, right-left, tile*0.72, tile*0.12, if (row+car)%2 == 0 { color::hex(0xe45f5b,255) } else { color::hex(0x63bad5,255) });
                        ui.fill(left + (right-left)*0.2, y + tile*0.25, (right-left)*0.38, tile*0.18, color::hex(0xd4f2ff,220));
                    }
                }
            }
        }
        if row == road.row && i == 5 {
            let x = bx + road.x as f32 * tile + tile*0.14;
            ui.rounded(x, y + tile*0.14, tile*0.72, tile*0.72, tile*0.12, color::hex(0xffdf87,255));
            ui.fill(x + tile*0.12, y + tile*0.25, tile*0.09, tile*0.09, color::hex(0x222030,255));
            ui.fill(x + tile*0.48, y + tile*0.25, tile*0.09, tile*0.09, color::hex(0x222030,255));
        }
    }
    ui.text(Face::SemiBold, 16.0, 15.0, 10.0, my3_text(), "Cross the Road");
    ui.text(Face::Medium, 12.0, w - 120.0, 14.0, my3_text(), &format!("Score: {}", road.best));
    button(ui, targets, [15.0, h - 48.0, 85.0, 34.0], "Back", Action::CloseRoadGame, false);
    let x = w - 155.0;
    let y = h - 83.0;
    if road.dead {
        button(ui, targets, [x - 10.0, y + 24.0, 140.0, 34.0], "Try again", Action::RestartRoadGame, true);
    } else {
        button(ui, targets, [x + 42.0, y, 38.0, 32.0], "↑", Action::RoadMove(0, 1), true);
        button(ui, targets, [x, y + 35.0, 38.0, 32.0], "←", Action::RoadMove(-1, 0), false);
        button(ui, targets, [x + 42.0, y + 35.0, 38.0, 32.0], "↓", Action::RoadMove(0, -1), false);
        button(ui, targets, [x + 84.0, y + 35.0, 38.0, 32.0], "→", Action::RoadMove(1, 0), false);
    }
    if road.dead { ui.centered_fit(Face::SemiBold, 18.0, [bx, h*0.42, board_w, 28.0], my3_text(), "Oops! Try again"); }
}

impl Launcher {
    /// The same spatial focus algorithm as the game's controller menus.
    pub(super) fn navigate(&mut self, key: &KeyEvent) -> bool {
        self.navigate_key(&key.logical_key)
    }
    pub(super) fn navigate_key(&mut self, key: &winit::keyboard::Key) -> bool {
        use horizon_client::gamepad::{self, Button};
        use winit::keyboard::{Key, NamedKey};
        if self.pending_focus.is_some() {
            return true;
        }
        if self.catalog.editing.is_some() || self.targets.is_empty() {
            return false;
        }
        let button = match key {
            Key::Named(NamedKey::ArrowUp) => Button::DPadUp,
            Key::Named(NamedKey::ArrowDown) => Button::DPadDown,
            Key::Named(NamedKey::ArrowLeft) => Button::DPadLeft,
            Key::Named(NamedKey::ArrowRight) => Button::DPadRight,
            Key::Named(NamedKey::Tab) => {
                self.focus = Some(self.focus.map_or(0, |i| (i + 1) % self.targets.len()));
                return true;
            }
            _ => return false,
        };
        if matches!(button, Button::DPadUp | Button::DPadDown)
            && let Some((_, action)) = self.focus.and_then(|i| self.targets.get(i))
            && let Some(index) = self.list_targets.iter().position(|(_, a)| a == action)
        {
            let rects: Vec<_> = self.list_targets.iter().map(|(r, _)| *r).collect();
            if let Some(next) = gamepad::focus_step_strict(rects[index], button, &rects)
                && let Some(view) = self.list_view
            {
                let (rect, action) = self.list_targets[next];
                let scroll = if let Some(chooser) = &mut self.chooser {
                    &mut chooser.scroll
                } else if let Some(modal) = &mut self.catalog.project_modal {
                    &mut modal.scroll
                } else {
                    &mut self.catalog.scroll
                };
                *scroll = super::gesture::reveal(*scroll, rect, view);
                self.pending_focus = Some(action);
                self.focus = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return true;
            }
        }
        self.focus = Some(
            if let Some(index) = self.focus.filter(|i| *i < self.targets.len()) {
                let rects: Vec<_> = self.targets.iter().map(|(rect, _)| *rect).collect();
                gamepad::focus_step_rect(rects[index], button, &rects).unwrap_or(index)
            } else {
                0
            },
        );
        true
    }

    pub(super) fn draw_pages(&mut self) -> Result<()> {
        // Page drawing borrows `self` mutably (settings and the project modal do), so lend the
        // renderer and its resources out as plain locals while the frame is built, and put them
        // back whatever the result is.
        let Some(mut r) = self.renderer.take() else {
            return Ok(());
        };
        let Some(gui) = self.gui.take() else {
            self.renderer = Some(r);
            return Ok(());
        };
        // Scope the glass palette to the launcher; game screens keep the user's theme.
        let saved_theme = horizon_client::ui_theme::active_theme();
        let ui_opacity = self.catalog.config.theme.ui_opacity.clamp(0.10, 1.0);
        let scale_a = |base: u8, min_a: u8| -> u8 {
            ((f32::from(base) * ui_opacity).round() as u32)
                .clamp(u32::from(min_a), 255) as u8
        };
        let fg_a = ((255.0 * (0.45 + 0.55 * ui_opacity)).round() as u32).clamp(90, 255) as u8;
        let mut glass = saved_theme;
        glass.is_light = false;
        glass.ui_opacity = ui_opacity;
        glass.bg_solid = color::hex(0x151922, scale_a(235, 16));
        glass.sidebar_bg = color::hex(0x10151c, scale_a(205, 16));
        glass.header_bg = color::hex(0x151922, scale_a(205, 16));
        glass.surface = color::hex(0x151922, scale_a(175, 16));
        glass.surface_dim = color::hex(0x10151c, scale_a(155, 14));
        glass.surface_bright = color::hex(0x3a4251, scale_a(185, 18));
        glass.surface_selected = color::hex(0xaaa0c8, scale_a(155, 18));
        glass.border = color::hex(0xe5dbf5, scale_a(155, 24));
        glass.border_strong = color::hex(0xfaf5ff, scale_a(205, 42));
        glass.accent = color::hex(0xc7b9df, fg_a);
        glass.accent_soft = color::hex(0xaaa0c8, scale_a(130, 14));
        glass.on_accent = color::hex(0x181321, fg_a);
        glass.accent_text = color::hex(0xeee6ff, fg_a);
        glass.text_primary = color::hex(0xffffff, fg_a);
        glass.text_muted = color::hex(0xd9d3e5, fg_a);
        if let Some([r, g, b]) = self.wallpaper_color {
            let [ar, ag, ab] = self.wallpaper_accent.unwrap_or([r, g, b]);
            let rgb = |r: u8, g: u8, b: u8| (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
            let mix = |a: u8, b: u8, num: u16, den: u16| -> u8 {
                ((u16::from(a) * (den - num) + u16::from(b) * num) / den).min(255) as u8
            };
            // Blend base scene mood with vibrant accent for rich Material You dark glass surfaces.
            let surf = [mix(r, ar, 28, 100), mix(g, ag, 28, 100), mix(b, ab, 28, 100)];
            let dark = |v: u8| (u16::from(v) * 18 / 100 + 11).min(78) as u8;
            let deeper = |v: u8| (u16::from(v) * 14 / 100 + 8).min(64) as u8;
            let medium = |v: u8| (u16::from(v) * 58 / 100 + 44).min(215) as u8;
            let bright = |v: u8| (u16::from(v) * 58 / 100 + 108).min(250) as u8;
            let accent_rgb_tuple = (bright(ar), bright(ag), bright(ab));
            let accent_hex = rgb(accent_rgb_tuple.0, accent_rgb_tuple.1, accent_rgb_tuple.2);
            let accent_luma = u16::from(accent_rgb_tuple.0) * 3
                + u16::from(accent_rgb_tuple.1) * 6
                + u16::from(accent_rgb_tuple.2);
            let muted_hex = rgb(
                mix(220, accent_rgb_tuple.0, 24, 100),
                mix(216, accent_rgb_tuple.1, 24, 100),
                mix(232, accent_rgb_tuple.2, 24, 100),
            );
            glass.bg_solid = color::hex(rgb(dark(surf[0]), dark(surf[1]), dark(surf[2])), scale_a(236, 16));
            glass.sidebar_bg = color::hex(rgb(deeper(surf[0]), deeper(surf[1]), deeper(surf[2])), scale_a(212, 16));
            glass.header_bg = color::hex(rgb(dark(surf[0]), dark(surf[1]), dark(surf[2])), scale_a(212, 16));
            glass.surface = color::hex(rgb(dark(surf[0]), dark(surf[1]), dark(surf[2])), scale_a(188, 16));
            glass.surface_dim = color::hex(rgb(deeper(surf[0]), deeper(surf[1]), deeper(surf[2])), scale_a(168, 14));
            glass.surface_bright = color::hex(rgb(medium(ar), medium(ag), medium(ab)), scale_a(185, 18));
            glass.surface_selected = color::hex(rgb(medium(ar), medium(ag), medium(ab)), scale_a(175, 18));
            glass.border = color::hex(accent_hex, scale_a(165, 24));
            glass.border_strong = color::hex(accent_hex, scale_a(225, 42));
            glass.accent = color::hex(accent_hex, fg_a);
            glass.accent_soft = color::hex(rgb(medium(ar), medium(ag), medium(ab)), scale_a(145, 14));
            glass.on_accent = color::hex(if accent_luma > 1350 { 0x14101C } else { 0xFFFFFF }, fg_a);
            glass.accent_text = glass.accent;
            glass.text_muted = color::hex(muted_hex, fg_a);
        }
        horizon_client::ui_theme::set_active_theme(glass);
        let result = self.draw_pages_with(&mut r, &gui);
        horizon_client::ui_theme::set_active_theme(saved_theme);
        if result.is_ok() && self.anim.intro_done && !r.suspended {
            crate::android::first_frame_shown();
        }
        self.renderer = Some(r);
        self.gui = Some(gui);
        result
    }

    /// The launcher and control editor share the selected live background.
    fn draw_scene_background(&mut self, ui: &mut Ui, w: f32, full_h: f32) {
        let sensitivity = self.catalog.config.scene_gyro_sensitivity.min(300) as f32 / 100.0;
        let motion_ok = sensitivity > 0.0 && !horizon_client::ui_theme::motion::reduced_motion()
            && self.focused && self.control_editor.is_none();
        let delta = crate::device_runtime::gyro(motion_ok, sensitivity.max(0.1), crate::platform::display_rotation());
        for (offset, change) in self.anim.scene_offset.iter_mut().zip(delta) {
            *offset = (*offset + change * 0.018).clamp(-1.0, 1.0);
            if !motion_ok { *offset = 0.0; }
        }
        static SCENE: std::sync::OnceLock<Option<image::RgbaImage>> = std::sync::OnceLock::new();
        let scene = SCENE.get_or_init(|| image::load_from_memory(include_bytes!("../../assets/greenhouse_poster.png"))
            .ok().map(|image| image.to_rgba8()));
        ui.fill(0.0, 0.0, w, full_h, color::hex(0x171822, 255));
        if let Some(scene) = scene {
            // Cover-fit with overscan, leaving room for generous bounded gyro parallax.
            let scale = (w / scene.width() as f32).max(full_h / scene.height() as f32) * 1.30;
            let sw = scene.width() as f32 * scale;
            let sh = scene.height() as f32 * scale;
            let dx = (sw - w) * 0.5;
            let dy = (sh - full_h) * 0.5;
            let x = -dx + self.anim.scene_offset[0] * dx * 0.95;
            let y = -dy + self.anim.scene_offset[1] * dy * 0.95;
            ui.scene_image(scene.width(), scene.height(), scene.as_raw(), x, y, sw, sh);
        }
        if self.catalog.config.custom_wallpaper_path.is_some() {
            if !self.catalog.config.custom_wallpaper_is_video {
                if let Some((pw, ph, rgba)) = &self.custom_photo_rgba {
                    let scale = (w / *pw as f32).max(full_h / *ph as f32) * 1.30;
                    let sw = *pw as f32 * scale;
                    let sh = *ph as f32 * scale;
                    let dx = (sw - w) * 0.5;
                    let dy = (sh - full_h) * 0.5;
                    let x = -dx + self.anim.scene_offset[0] * dx * 0.95;
                    let y = -dy + self.anim.scene_offset[1] * dy * 0.95;
                    let upload = if self.custom_photo_dirty {
                        self.custom_photo_dirty = false;
                        Some(rgba.clone())
                    } else {
                        None
                    };
                    ui.video_scene(*pw, *ph, upload, x, y, sw, sh);
                }
            } else {
                let [frame_w, frame_h] = self.wallpaper_frame_size;
                let expected = frame_w as usize * frame_h as usize * 4;
                let frame = if self.focused && expected > 0 {
                    crate::native_ime::wallpaper_frame(expected, &mut self.wallpaper_buffer)
                } else {
                    None
                };
                if let Some(bytes) = frame.as_ref() {
                    self.wallpaper_has_frame = true;
                    if let Some((base, accent)) = crate::wallpapers::sample_theme_colors(bytes) {
                        self.wallpaper_color = Some(match self.wallpaper_color {
                            Some(old) => std::array::from_fn(|i| {
                                ((u16::from(old[i]) * 15 + u16::from(base[i])) / 16) as u8
                            }),
                            None => base,
                        });
                        self.wallpaper_accent = Some(match self.wallpaper_accent {
                            Some(old) => std::array::from_fn(|i| {
                                ((u16::from(old[i]) * 15 + u16::from(accent[i])) / 16) as u8
                            }),
                            None => accent,
                        });
                    }
                }
                if self.wallpaper_has_frame {
                    let [src_w, src_h] = [
                        self.custom_video_source_size[0].max(1) as f32,
                        self.custom_video_source_size[1].max(1) as f32,
                    ];
                    let scale = (w / src_w).max(full_h / src_h) * 1.30;
                    let sw = src_w * scale;
                    let sh = src_h * scale;
                    let dx = (sw - w) * 0.5;
                    let dy = (sh - full_h) * 0.5;
                    let x = -dx + self.anim.scene_offset[0] * dx * 0.95;
                    let y = -dy + self.anim.scene_offset[1] * dy * 0.95;
                    ui.video_scene(frame_w, frame_h, frame, x, y, sw, sh);
                }
            }
        } else {
            let index = crate::wallpapers::effective_index(self.catalog.config.wallpaper_index);
            if let Some(item) = crate::wallpapers::WALLPAPERS.get(index)
            {
                let [frame_w, frame_h] = self.wallpaper_frame_size;
                let expected = frame_w as usize * frame_h as usize * 4;
                // Keep the last uploaded frame visible across transient focus changes.
                let frame = if self.focused && expected > 0 { crate::native_ime::wallpaper_frame(expected, &mut self.wallpaper_buffer) } else { None };
                if let Some(bytes) = frame.as_ref() {
                    self.wallpaper_has_frame = true;
                    if let Some((base, accent)) = crate::wallpapers::sample_theme_colors(bytes) {
                        self.wallpaper_color = Some(match self.wallpaper_color {
                            Some(old) => std::array::from_fn(|i| ((u16::from(old[i]) * 15 + u16::from(base[i])) / 16) as u8),
                            None => base,
                        });
                        self.wallpaper_accent = Some(match self.wallpaper_accent {
                            Some(old) => std::array::from_fn(|i| ((u16::from(old[i]) * 15 + u16::from(accent[i])) / 16) as u8),
                            None => accent,
                        });
                    }
                }
                if self.wallpaper_has_frame {
                    let scale = (w / item.width as f32).max(full_h / item.height as f32) * 1.30;
                    let sw = item.width as f32 * scale;
                    let sh = item.height as f32 * scale;
                    let dx = (sw - w) * 0.5;
                    let dy = (sh - full_h) * 0.5;
                    let x = -dx + self.anim.scene_offset[0] * dx * 0.95;
                    let y = -dy + self.anim.scene_offset[1] * dy * 0.95;
                    ui.video_scene(frame_w, frame_h, frame, x, y, sw, sh);
                }
            }
        }
        // A restrained veil keeps the landscape visible and the white controls readable,
        // scaling with UI Opacity so transparent UI shows the background vividly.
        let veil_a = (42.0 * self.catalog.config.theme.ui_opacity.clamp(0.10, 1.0)).round() as u8;
        ui.fill(0.0, 0.0, w, full_h, color::hex(0x10111d, veil_a));

    }

    /// Draws one launcher frame with the renderer lent out (see `draw_pages`).
    fn draw_pages_with(&mut self, r: &mut Renderer, gui: &game::UiResources) -> Result<()> {
        if r.suspended {
            return Ok(());
        }
        // Start the 1.6s rise-from-below reveal after the first frame actually presents so
        // frame-0 wallpaper/atlas/GPU initialization never consumes the animation window.
        let reveal = if horizon_client::ui_theme::motion::reduced_motion() {
            1.0
        } else {
            self.anim
                .reveal_start
                .map_or(0.0, |t| (t.elapsed().as_secs_f32() / 1.6).min(1.0))
        };
        custom_background_poll(&self.home);
        self.poll_wallpaper_previews();
        self.targets.clear();
        SLIDER_TRACK_TARGETS.with(|t| t.borrow_mut().clear());
        self.list_targets.clear();
        self.list_view = None;

        if let Some(road) = &mut self.road_game {
            road.tick(self.focused);
            let mut b = HudBatch::new([r.config.width, r.config.height]);
            let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
            draw_cross_road(&mut ui, &mut self.targets, road);
            ui.finish();
            if !r.render(&game::loading_frame(&b))? && !b.patches.is_empty() {
                horizon_client::ui_theme::reset_atlas();
            }
            return Ok(());
        }

        // Keep the same wallpaper behind the full-screen touch layout editor.
        if self.control_editor.is_some() {
            let game_home = self.catalog.config.game_home(&self.home);
            let prev_cwd = std::env::current_dir().ok();
            let _ = std::env::set_current_dir(&game_home);
            let mut b = HudBatch::new([r.config.width, r.config.height]);
            {
                let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
                let (w, h) = (ui.w, ui.h);
                self.draw_scene_background(&mut ui, w, h);
                ui.finish();
            }
            let tc = self.control_editor.as_mut().expect("editor is open");
            tc.viewport(r.config.width, r.config.height);
            let profile_name = tc.settings.active_profile.clone();
            horizon_client::touch_ui::draw_hud_editor(tc, &mut b, &gui.atlas, &profile_name, None);
            if let Some(prev) = prev_cwd {
                let _ = std::env::set_current_dir(prev);
            }
            let rendered = r.render(&game::loading_frame(&b));
            self.recycle_wallpaper_buffer(&mut b);
            if !rendered? && !b.patches.is_empty() {
                horizon_client::ui_theme::reset_atlas();
            }
            return Ok(());
        }

        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        // Keep ui.w and ui.h synchronized with Launcher::ui_density(). When the soft keyboard is
        // active, compute ime_top_dp and bias scroll offsets / modal Y positions instead of shrinking ui.h.
        let ime_top_dp = if self.catalog.editing.is_some() {
            crate::native_ime::viewport_bottom()
                .map(|bottom_px| (bottom_px as f32 / ui.density).clamp(110.0, ui.h))
        } else {
            None
        };
        let (w, full_h) = (ui.w, ui.h);
        // Leave room for the navigation dock in all page layouts and scroll calculations.
        let h = (full_h - 54.0).max(210.0);
        ui.h = h;
        self.draw_scene_background(&mut ui, w, full_h);

        if self.catalog.page == Page::Music {
            ui.h = full_h;
            self.draw_music_fullscreen(&mut ui, w, full_h, ime_top_dp);
        } else if self.catalog.page == Page::SkinOnline {
            ui.h = full_h;
            self.skin_online.draw(
                &mut ui,
                &mut self.targets,
                &mut self.wardrobe,
                &self.catalog.skin_online_query,
                self.catalog.editing,
                w,
                full_h,
                ime_top_dp,
            );
        } else {
        let account_name = self.login.profile.clone().unwrap_or_else(|| self.catalog.config.name.clone());
        // Brand and account enter first, followed by the avatar and navigation.
        ui.set_stagger_transition((reveal * 3.0).min(1.0), 0.0, 0.0);
        ui.rounded(12.0, 8.0, 198.0, 40.0, 20.0, my3_surface());
        ui.border(12.0, 8.0, 198.0, 40.0, 20.0, my3_border());
        ui.text(Face::SemiBold, 19.0, 24.0, 14.0, my3_text(), "V+");
        ui.text(Face::SemiBold, 12.5, 55.0, 11.0, my3_text(), "Volt launcher plus");
        ui.text(Face::Regular, 9.0, 55.0, 28.0, my3_muted(), &format!("Android {}", crate::backend::VERSION));
        let account_rect = [12.0, 54.0, 100.0, 36.0];
        ui.rounded(account_rect[0], account_rect[1], account_rect[2], account_rect[3], 18.0, my3_surface());
        ui.border(account_rect[0], account_rect[1], account_rect[2], account_rect[3], 18.0, my3_border());
        self.wardrobe.draw_head(&mut ui, 18.0, 57.0, 30.0);
        let label = ui.fit_text(Face::Medium, 11.0, &account_name, 45.0);
        ui.text(Face::Medium, 11.0, 49.0, 63.0, my3_text(), &label);
        self.targets.push((account_rect, Action::Account));
        let discord_space = w - 378.0 - 8.0 - 218.0;
        let top_compact = w < 596.0;
        let discord_rect = if discord_space >= 104.0 {
            [218.0, 11.0, discord_space.min(148.0), 34.0]
        } else {
            let max_disc = (w - 234.0).clamp(84.0, 136.0);
            [118.0, 55.0, max_disc, 34.0]
        };
        button(&mut ui, &mut self.targets, discord_rect,
            if top_compact { "Discord" } else { "Join our Discord" }, Action::Discord, false);
        if top_compact {
            let bg_w = 72.0;
            let inv_w = 72.0;
            let mus_w = 62.0;
            let rx0 = (w - 12.0 - bg_w).max(216.0);
            let rx1 = (rx0 - 6.0 - inv_w).max(140.0);
            let rx2 = (rx1 - 6.0 - mus_w).max(74.0);
            button(&mut ui, &mut self.targets, [rx2, 12.0, mus_w, 34.0],
                "Music", Action::OpenMusic, false);
            button(&mut ui, &mut self.targets, [rx1, 12.0, inv_w, 34.0],
                "Items", Action::Page(Page::Inventory), self.catalog.page == Page::Inventory);
            button(&mut ui, &mut self.targets, [rx0, 12.0, bg_w, 34.0],
                "Theme", Action::OpenWallpapers, false);
        } else {
            button(&mut ui, &mut self.targets, [w - 378.0, 12.0, 88.0, 34.0],
                "Musics", Action::OpenMusic, false);
            button(&mut ui, &mut self.targets, [w - 282.0, 12.0, 100.0, 34.0],
                "Inventory", Action::Page(Page::Inventory), self.catalog.page == Page::Inventory);
            button(&mut ui, &mut self.targets, [w - 174.0, 12.0, 162.0, 34.0],
                "Background images", Action::OpenWallpapers, false);
        }

        ui.set_stagger_transition(1.0, 0.0, 0.0);
        let page = self.catalog.page;
        if page == Page::Play {
            let so_rect = if discord_space >= 104.0 {
                [118.0, 55.0, 106.0, 34.0]
            } else {
                let x0 = discord_rect[0] + discord_rect[2] + 6.0;
                let avail = (w - x0 - 12.0).max(86.0);
                [x0.min(w - avail - 12.0), 55.0, avail.min(106.0), 34.0]
            };
            button(
                &mut ui,
                &mut self.targets,
                so_rect,
                "Skin Online",
                Action::OpenSkinOnline,
                true,
            );
        }
        let target_shift = if page == Page::Play { 0.0 } else { 1.0 };
        if horizon_client::ui_theme::motion::reduced_motion() {
            self.anim.scene_shift = target_shift;
        } else {
            let dt = self.anim.last_tick.elapsed().as_secs_f32().clamp(0.001, 0.05);
            self.anim.scene_shift += (target_shift - self.anim.scene_shift) * (dt * 10.0).min(1.0);
        }
        let t = self.anim.scene_shift;
        // The marked left corner, below the account badge and above controls.
        let from = [12.0, 98.0, (w * 0.19).clamp(100.0, 200.0), (h - 180.0).max(90.0)];
        let to = [12.0, 98.0, (w * 0.23 - 18.0).clamp(100.0, 210.0), (h - 114.0).max(90.0)];
        let scene_rect = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t);
        if reveal >= 0.14 {
            self.wardrobe.draw_scene(&mut ui, scene_rect);
        }

        let nav_pages = [
            Page::Play,
            Page::Worlds,
            Page::Mods,
            Page::Packs,
            Page::ControlsStudio,
            Page::Shaders,
            Page::Installed,
            Page::Replays,
            Page::Skins,
            Page::Settings,
        ];
        let nav_w = (w - 24.0).min(760.0);
        let nav_x = (w - nav_w) * 0.5;
        let nav_y = full_h - 49.0;
        let nav_h = 42.0;
        let nav_step = nav_w / nav_pages.len() as f32;
        let selected_idx = nav_pages.iter().position(|p| *p == page).unwrap_or(0);
        let has_any_modal = self.catalog.project_modal.is_some() || self.catalog.fm_modal.is_some()
            || self.chooser.is_some() || self.login.visible || self.exit_confirmation;
        self.anim.step(page, self.catalog.settings_category,
            nav_x + selected_idx as f32 * nav_step, has_any_modal, self.catalog.sections);
        ui.set_stagger_transition(((reveal - 0.16) * 4.0).clamp(0.0, 1.0), 0.0, 0.0);
        ui.rounded(nav_x, nav_y, nav_w, nav_h, 21.0, my3_surface());
        ui.stroke_rounded(nav_x, nav_y, nav_w, nav_h, 21.0, 1.0, my3_border());
        ui.rounded(self.anim.nav_pill_y + 3.0, nav_y + 3.0, nav_step - 6.0, nav_h - 6.0,
            18.0, my3_nav_active());
        for (i, item) in nav_pages.iter().enumerate() {
            let x = nav_x + i as f32 * nav_step;
            let item_progress = ((reveal - 0.24 - i as f32 * 0.045) * 5.0).clamp(0.0, 1.0);
            if item_progress <= 0.0 { continue; }
            ui.set_stagger_transition(item_progress, x, nav_y);
            let selected = *item == page;
            let label = match item {
                Page::Packs => "Packs",
                Page::ControlsStudio => "Studio",
                _ => item.name(),
            };
            let icon_x = x + if nav_step < 62.0 { (nav_step - 15.0) * 0.5 } else { 10.0 };
            ui.icon(page_icon(*item), icon_x, nav_y + 13.0, 15.0, my3_text());
            if nav_step >= 62.0 {
                let text = ui.fit_text(if selected { Face::SemiBold } else { Face::Medium }, 11.5, label, nav_step - 31.0);
                ui.text(if selected { Face::SemiBold } else { Face::Medium }, 11.5,
                    x + 29.0, nav_y + 12.0, my3_text(), &text);
            }
            self.targets.push(([x, nav_y, nav_step, nav_h], Action::Page(*item)));
        }

        ui.set_stagger_transition(1.0, 0.0, 0.0);
        let content_x = (w * 0.25).max(176.0);
        let content_w = (w - content_x - 22.0).max(160.0);
        let compact = content_w < 430.0;
        let slide_y = self.anim.page_slide * 0.35;
        if page != Page::Play {
            ui.rounded(content_x - 12.0, 54.0, content_w + 22.0, (h - 58.0).max(100.0), 20.0, my3_surface());
            ui.stroke_rounded(content_x - 12.0, 54.0, content_w + 22.0, (h - 58.0).max(100.0), 20.0, 1.0, my3_border());
        }
        let content_reveal = ((reveal - 0.38) / 0.62).clamp(0.0, 1.0);
        ui.set_stagger_transition(self.anim.page_fade.min(content_reveal), 0.0, 52.0);

        // When any catalog item (Mod, Pack, Shader) is downloading/installing, take the entire
        // middle screen to show the MY3 downloading view with Speed, ETA, and Steps.
        if let Some((source_id, _)) = self
            .catalog
            .downloading
            .as_ref()
            .filter(|_| self.catalog.busy())
        {
            let frac = self.catalog.progress_fraction().clamp(0.0, 1.0);
            self.anim.progress_smooth += (frac - self.anim.progress_smooth) * 0.25;
            let smooth_frac = self.anim.progress_smooth.clamp(0.04, 1.0);
            let title = if self.catalog.downloading_title.is_empty() {
                "Downloading add-on...".to_owned()
            } else {
                format!("Installing {}", self.catalog.downloading_title)
            };
            let sub_left = format!(
                "Minecraft 26.3 • {} • Profile {}",
                source_id.label(),
                self.catalog.config.profile + 1
            );
            let sub_right = format!("{:.0}%", frac * 100.0);
            let stage = self.catalog.progress_stage();
            let step_idx = if frac < 0.15 {
                0
            } else if frac < 0.72 {
                1
            } else if frac < 0.92 {
                2
            } else {
                3
            };
            draw_my3_progress_view(
                &mut ui,
                &mut self.targets,
                content_x,
                content_w,
                h,
                &title,
                &sub_left,
                &sub_right,
                "Speed unavailable",
                "ETA unavailable",
                step_idx,
                &["Resolve", "Download", "Verify", "Install"],
                smooth_frac,
                self.anim.shimmer_phase,
                &stage,
                self.catalog.is_paused(),
                Action::PauseCatalogDownload,
                Action::CancelCatalogDownload,
            );
        } else if page == Page::Play {
            use horizon_client::official_install::InstallStage;
            let installed = horizon_client::official_install::ready(&self.home);
            let installing = self.worker.is_some();
            let progress = *self.install.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if installing {
                let current = InstallStage::ALL
                    .iter()
                    .position(|s| *s == progress.stage)
                    .unwrap_or(0);
                let fraction = if progress.total > 0 {
                    progress.done as f32 / progress.total as f32
                } else {
                    0.0
                };
                let overall = ((current as f32 + fraction.clamp(0.0, 1.0)) / 6.0).clamp(0.02, 1.0);
                self.anim.progress_smooth += (overall - self.anim.progress_smooth) * 0.25;
                let smooth_overall = self.anim.progress_smooth.clamp(0.02, 1.0);
                let (done_mib, total_mib) =
                    if progress.stage == InstallStage::Download && progress.total > 0 {
                        (
                            progress.done as f64 / 1_048_576.0,
                            progress.total as f64 / 1_048_576.0,
                        )
                    } else {
                        (0.0, 0.0)
                    };
                let paused = self.install_paused.load(Ordering::Relaxed);
                self.install_transfer.observe(done_mib, paused);
                let sub_left = format!(
                    "Minecraft 26.3 • Profile {}",
                    self.catalog.config.profile + 1
                );
                let sub_right = if total_mib > 0.0 {
                    format!("{done_mib:.1} / {total_mib:.1} MiB")
                } else { format!("Step {} / 6", current + 1) };
                let stage_text = match progress.stage {
                    InstallStage::Download => "Downloading game resources and sound",
                    other => other.label(),
                };
                draw_my3_progress_view(
                    &mut ui,
                    &mut self.targets,
                    content_x,
                    content_w,
                    h,
                    "Getting your game ready",
                    &sub_left,
                    &sub_right,
                    &if total_mib > 0.0 { self.install_transfer.speed_label() } else { "Speed unavailable".into() },
                    &if total_mib > 0.0 { self.install_transfer.eta_label(done_mib, total_mib) } else { "ETA unavailable".into() },
                    current,
                    &[
                        "Metadata",
                        "Download",
                        "Extract",
                        "World data",
                        "Controls",
                        "Finalize",
                    ],
                    smooth_overall,
                    self.anim.shimmer_phase,
                    stage_text,
                    paused,
                    Action::PauseInstall,
                    Action::CancelInstall,
                );
                let game_button = [content_x + 16.0, h - 36.0, 150.0, 29.0];
                button(&mut ui, &mut self.targets, game_button, "Play Cross the Road", Action::OpenRoadGame, true);
            } else {
                let is_launching = self.launching || self.session_worker.is_some();
                let label = if is_launching { "Launching..." } else if installed {
                    "Play Minecraft 26.3"
                } else if self.error.is_some() { "Retry installation" } else { "Download Minecraft 26.3" };
                let play_w = (w * 0.34).clamp(210.0, 310.0);
                let play_rect = [(w - play_w) * 0.5, h - 46.0, play_w, 40.0];
                ui.rounded(play_rect[0], play_rect[1], play_rect[2], play_rect[3], 20.0, my3_surface_high());
                ui.stroke_rounded(play_rect[0], play_rect[1], play_rect[2], play_rect[3], 20.0, 1.2, my3_border());
                ui.icon(Icon::Play, play_rect[0] + 16.0, play_rect[1] + 12.0, 16.0, my3_text());
                ui.centered_fit(Face::SemiBold, 14.0, [play_rect[0] + 28.0, play_rect[1], play_rect[2] - 36.0, play_rect[3]], my3_text(), label);
                if !is_launching { self.targets.push((play_rect, Action::Play)); }
                button(&mut ui, &mut self.targets, [12.0, h - 82.0, 118.0, 32.0], "Touch layout", Action::OpenControlEditor, false);
                button(&mut ui, &mut self.targets, [136.0, h - 82.0, 108.0, 32.0],
                    &format!("Profile {}", self.catalog.config.profile + 1), Action::Profile, false);
                button(&mut ui, &mut self.targets, [12.0, h - 44.0, 232.0, 32.0],
                    &format!("Renderer: {}", self.catalog.config.renderer.label()), Action::Renderer, false);
                // A compact control within the open lower-right portion of the home scene.
                let gyro_w = 218.0_f32.min((w * 0.26).max(150.0));
                let gx = w - gyro_w - 12.0;
                let gy = h - 44.0;
                ui.rounded(gx, gy, gyro_w, 32.0, 10.0, my3_surface());
                ui.border(gx, gy, gyro_w, 32.0, 10.0, my3_border());
                button(&mut ui, &mut self.targets, [gx + 4.0, gy + 3.0, 30.0, 26.0], "−", Action::SceneGyro(-1), false);
                button(&mut ui, &mut self.targets, [gx + gyro_w - 34.0, gy + 3.0, 30.0, 26.0], "+", Action::SceneGyro(1), false);
                let value = if self.catalog.config.scene_gyro_sensitivity == 0 { "Off".to_owned() }
                    else { format!("{}%", self.catalog.config.scene_gyro_sensitivity) };
                ui.centered_fit(Face::Medium, 11.0, [gx + 35.0, gy, gyro_w - 70.0, 32.0], my3_text(), &format!("Gyroscope {value}"));
                let nav_right = (w + nav_w) * 0.5;
                let right_space = w - nav_right - 20.0;
                let keep_on = self.catalog.config.screen.keep_screen_on;
                let keep_label = if keep_on {
                    "Keep screen on: ON"
                } else {
                    "Keep screen on"
                };
                let keep_rect = if right_space >= 92.0 {
                    [nav_right + 8.0, full_h - 45.0, right_space.min(180.0), 34.0]
                } else {
                    [gx, h - 82.0, gyro_w, 32.0]
                };
                button(
                    &mut ui,
                    &mut self.targets,
                    keep_rect,
                    keep_label,
                    Action::KeepScreenOn,
                    keep_on,
                );
            }
        } else if page == Page::Skins {
            self.wardrobe.draw(
                &mut ui,
                &mut self.targets,
                self.catalog.scroll,
                content_x,
                content_w,
                62.0 + slide_y,
            );
            self.list_view = Some(self.wardrobe.view);
            self.catalog.scroll = self.catalog.scroll.min(self.wardrobe.view[2]);
        } else if page.is_catalog() {
            let header_y = 62.0 + slide_y;
            ui.text(
                Face::SemiBold,
                19.0,
                content_x,
                header_y,
                my3_text(),
                if page == Page::Mods {
                    "Mods"
                } else if page == Page::Shaders {
                    "Shader Packs"
                } else {
                    "Resource Packs"
                },
            );
            if page == Page::Mods {
                let bx = content_x + 72.0;
                let bw = (content_w - 72.0).max(120.0);
                let by = header_y - 4.0;
                ui.rounded(bx, by, bw, 32.0, 9.0, horizon_client::hud::rgba(28, 22, 14, 195));
                ui.border(bx, by, bw, 32.0, 9.0, horizon_client::hud::rgba(242, 184, 75, 130));
                ui.text_clipped(
                    Face::SemiBold,
                    10.2,
                    bx + 10.0,
                    by + 3.0,
                    bw - 20.0,
                    horizon_client::hud::rgba(255, 220, 140, 255),
                    "Not all mods are working and supported",
                );
                ui.text_clipped(
                    Face::Regular,
                    9.4,
                    bx + 10.0,
                    by + 16.5,
                    bw - 20.0,
                    horizon_client::hud::rgba(224, 214, 196, 235),
                    "These are community mods and need heavy work on",
                );
            } else if page == Page::Shaders {
                use crate::shader_catalog::Origin;
                let seg_w = 104.0;
                let group_x = content_x + content_w - seg_w * 2.0 - 3.0;
                for (i, origin) in [Origin::Horizon, Origin::Modrinth].into_iter().enumerate() {
                    let rx = group_x + i as f32 * (seg_w + 3.0);
                    let rect = [rx, header_y - 2.0, seg_w, 30.0];
                    let active = self.catalog.config.shader_source == origin;
                    ui.m3_connected_segment(
                        rect,
                        i,
                        2,
                        origin.source().label(),
                        active,
                        Some(Icon::Shaders),
                    );
                    self.targets.push((rect, Action::ShaderSource(origin)));
                }
            }

            // Search bar + Search button + Open folder / Import file pills
            let search_y = 98.0 + slide_y;
            let search_btn_w = 98.0;
            let field_w = (content_w - search_btn_w - 8.0).max(120.0);
            let search_hint = if page == Page::Mods {
                "Search mods or paste .hmod HTTPS link..."
            } else {
                "Search catalog..."
            };
            let search_rect = ui.text_field(
                content_x,
                search_y,
                field_w,
                36.0,
                "",
                &self.catalog.query,
                search_hint,
                self.catalog.editing == Some("search"),
            );
            self.targets.push((search_rect, Action::Field("search")));

            button(
                &mut ui,
                &mut self.targets,
                [content_x + field_w + 8.0, search_y, search_btn_w, 36.0],
                if page == Page::Mods && self.catalog.query.trim().starts_with("https://") {
                    "Import .hmod"
                } else {
                    "Search"
                },
                Action::Search,
                true,
            );

            let kind = match page {
                Page::Mods => horizon_client::content_files::Kind::Mods,
                Page::Shaders => horizon_client::content_files::Kind::Shaders,
                _ => horizon_client::content_files::Kind::Packs,
            };
            let sub_y = 140.0 + slide_y;
            button(
                &mut ui,
                &mut self.targets,
                [content_x, sub_y, 114.0, 29.0],
                "Open folder",
                Action::OpenFolder(kind),
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [content_x + 122.0, sub_y, 114.0, 29.0],
                "Import file",
                Action::Import(kind),
                false,
            );
            if !compact {
                let status_line = self.error.as_deref().unwrap_or(&self.catalog.status);
                let fitted_status = ui.fit_text(
                    Face::Regular,
                    12.5,
                    status_line,
                    (content_w - 248.0).max(60.0),
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    content_x + 246.0,
                    sub_y + 5.0,
                    my3_muted(),
                    &fitted_status,
                );
            }

            let bundled: Vec<_> = if page == Page::Mods {
                let query = self.catalog.query.trim().to_lowercase();
                horizon_client::mod_list::ANDROID_BUNDLED
                    .into_iter()
                    .filter(|(id, title, _)| {
                        query.is_empty()
                            || id.contains(&query)
                            || title.to_lowercase().contains(&query)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let catalog_rows: Vec<_> = self
                .catalog
                .results
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    !(page == Page::Mods
                        && p.source == crate::browser::SourceId::Horizon
                        && horizon_client::mod_list::ANDROID_BUNDLED
                            .iter()
                            .any(|(id, _, _)| *id == p.id))
                })
                .collect();
            let is_horizon_paged = page == Page::Mods
                || (page == Page::Shaders
                    && self.catalog.config.shader_source == crate::shader_catalog::Origin::Horizon);
            let total_horizon_items = bundled.len() + catalog_rows.len();
            let total_pages = if is_horizon_paged {
                total_horizon_items
                    .div_ceil(crate::catalog::CATALOG_PAGE_SIZE)
                    .max(1)
            } else {
                1
            };
            if is_horizon_paged {
                let max_offset = (total_pages - 1) * crate::catalog::CATALOG_PAGE_SIZE;
                if self.catalog.offset > max_offset {
                    self.catalog.offset = max_offset;
                }
            }
            let page_start = if is_horizon_paged {
                self.catalog.offset.min(total_horizon_items)
            } else {
                0
            };
            let page_end = if is_horizon_paged {
                (page_start + crate::catalog::CATALOG_PAGE_SIZE).min(total_horizon_items)
            } else {
                catalog_rows.len().min(crate::catalog::CATALOG_PAGE_SIZE)
            };
            let bundled_start = page_start.min(bundled.len());
            let bundled_end = page_end.min(bundled.len());
            let page_bundled = &bundled[bundled_start..bundled_end];
            let cat_start = page_start.saturating_sub(bundled.len()).min(catalog_rows.len());
            let cat_end = page_end.saturating_sub(bundled.len()).min(catalog_rows.len());
            let page_catalog_rows = &catalog_rows[cat_start..cat_end];

            let row_h = 72.0;
            let row_count = page_bundled.len() + page_catalog_rows.len();
            let top = 176.0 + slide_y;
            let bottom = h - 46.0;
            self.catalog.scroll = self
                .catalog
                .scroll
                .clamp(0.0, (row_count as f32 * row_h - (bottom - top)).max(0.0));
            self.list_view = Some([
                top,
                bottom,
                (row_count as f32 * row_h - (bottom - top)).max(0.0),
            ]);

            let fallback_icon = page_icon(page);
            ui.set_clip(Some([
                content_x - 2.0,
                top,
                content_w + 4.0,
                (bottom - top).max(1.0),
            ]));

            for (row, (id, title, description)) in page_bundled.iter().enumerate() {
                let y = top + row as f32 * row_h - self.catalog.scroll;
                let enabled = self.catalog.bundled_enabled.contains(*id);
                let card_rect = [content_x, y, content_w - 118.0, 64.0];
                let btn_rect = [content_x + content_w - 112.0, y + 13.0, 100.0, 38.0];
                self.list_targets
                    .push((card_rect, Action::OpenBundledProject(id)));
                self.list_targets
                    .push((btn_rect, Action::ToggleBundled(id)));
                if y + 64.0 < top || y > bottom {
                    continue;
                }
                ui.rounded(content_x, y, content_w, 64.0, 14.0, my3_card_bg());
                ui.border(content_x, y, content_w, 64.0, 14.0, my3_card_border());

                // Project icon: fetched Modrinth PFP if available, else MY3 vector icon badge.
                let drawn_icon = if let Some((iw, ih, rgba)) = self.catalog.icons.get(*id) {
                    ui.image(id, *iw, *ih, rgba, content_x + 10.0, y + 10.0, 44.0, 44.0)
                } else {
                    false
                };
                if !drawn_icon {
                    let (badge_icon, badge_bg, badge_fg) =
                        catalog_fallback_badge(id, fallback_icon);
                    ui.rounded(
                        content_x + 10.0,
                        y + 10.0,
                        44.0,
                        44.0,
                        12.0,
                        badge_bg,
                    );
                    ui.icon(
                        badge_icon,
                        content_x + 21.0,
                        y + 21.0,
                        22.0,
                        badge_fg,
                    );
                }

                // Badge chip ("Built-in • Tap for info")
                let text_max = (content_w - 190.0).max(80.0);
                let fitted_title = ui.fit_text(Face::SemiBold, 15.0, title, text_max);
                let rich_desc = self
                    .catalog
                    .enriched_summaries
                    .get(*id)
                    .map_or(*description, String::as_str);
                let summary = ui.fit_text(Face::Regular, 12.5, rich_desc, text_max);
                ui.text(
                    Face::SemiBold,
                    15.0,
                    content_x + 64.0,
                    y + 8.0,
                    my3_text(),
                    &fitted_title,
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    content_x + 64.0,
                    y + 28.0,
                    my3_muted(),
                    &summary,
                );
                ui.text(
                    Face::Medium,
                    11.0,
                    content_x + 64.0,
                    y + 45.0,
                    my3_accent(),
                    "Horizon Built-in • Tap for versions, Modrinth info & screenshots",
                );
                if let Some(c) = ui.clip_rect(card_rect) {
                    self.targets.push((c, Action::OpenBundledProject(id)));
                }
                button(
                    &mut ui,
                    &mut self.targets,
                    btn_rect,
                    if enabled { "Installed" } else { "Install" },
                    Action::ToggleBundled(id),
                    !enabled,
                );
            }

            for (row, &(i, p)) in page_catalog_rows.iter().enumerate() {
                let y = top + (row + page_bundled.len()) as f32 * row_h - self.catalog.scroll;
                let download = self.catalog.download_label(p);
                let card_rect = [content_x, y, content_w - 118.0, 64.0];
                let btn_rect = [content_x + content_w - 112.0, y + 13.0, 100.0, 38.0];
                self.list_targets.push((card_rect, Action::OpenProject(i)));
                if download.is_none() {
                    self.list_targets.push((btn_rect, Action::Install(i)));
                }
                if y + 64.0 < top || y > bottom {
                    continue;
                }
                ui.rounded(content_x, y, content_w, 64.0, 14.0, my3_card_bg());
                ui.border(content_x, y, content_w, 64.0, 14.0, my3_card_border());

                // Project icon: fetched RGBA icon if ready, else MY3 vector icon badge.
                let drawn_icon = if let Some((iw, ih, rgba)) = self.catalog.icons.get(&p.id) {
                    ui.image(
                        &p.id,
                        *iw,
                        *ih,
                        rgba,
                        content_x + 10.0,
                        y + 10.0,
                        44.0,
                        44.0,
                    )
                } else {
                    false
                };
                if !drawn_icon {
                    let (badge_icon, badge_bg, badge_fg) =
                        catalog_fallback_badge(&p.id, fallback_icon);
                    ui.rounded(
                        content_x + 10.0,
                        y + 10.0,
                        44.0,
                        44.0,
                        12.0,
                        badge_bg,
                    );
                    ui.icon(
                        badge_icon,
                        content_x + 21.0,
                        y + 21.0,
                        22.0,
                        badge_fg,
                    );
                }

                let text_max = (content_w - 190.0).max(80.0);
                let title = ui.fit_text(Face::SemiBold, 15.0, &p.title, text_max);
                let rich_summary = self
                    .catalog
                    .enriched_summaries
                    .get(&p.id)
                    .map_or(p.summary.as_str(), String::as_str);
                let summary = ui.fit_text(Face::Regular, 12.5, rich_summary, text_max);
                ui.text(
                    Face::SemiBold,
                    15.0,
                    content_x + 64.0,
                    y + 8.0,
                    my3_text(),
                    &title,
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    content_x + 64.0,
                    y + 28.0,
                    my3_muted(),
                    &summary,
                );
                let meta_tag = if p.downloads > 0 {
                    format!(
                        "{} • {} downloads • Tap for versions & screenshots",
                        p.source.label(),
                        format_downloads(p.downloads)
                    )
                } else {
                    format!(
                        "{} • Tap for versions, description & screenshots",
                        p.source.label()
                    )
                };
                let fitted_meta = ui.fit_text(Face::Medium, 11.0, &meta_tag, text_max);
                ui.text(
                    Face::Medium,
                    11.0,
                    content_x + 64.0,
                    y + 45.0,
                    my3_accent(),
                    &fitted_meta,
                );
                if let Some(c) = ui.clip_rect(card_rect) {
                    self.targets.push((c, Action::OpenProject(i)));
                }

                if let Some(state) = download {
                    ui.rounded(
                        btn_rect[0],
                        btn_rect[1],
                        btn_rect[2],
                        btn_rect[3],
                        19.0,
                        my3_surface_high(),
                    );
                    ui.icon(
                        Icon::Check,
                        btn_rect[0] + 10.0,
                        btn_rect[1] + 11.0,
                        15.0,
                        my3_accent(),
                    );
                    ui.text(
                        Face::Medium,
                        13.0,
                        btn_rect[0] + 28.0,
                        btn_rect[1] + 9.0,
                        my3_text(),
                        &state,
                    );
                    if let Some(c) = ui.clip_rect(btn_rect) {
                        self.targets.push((c, Action::OpenProject(i)));
                    }
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        btn_rect,
                        self.catalog.install_button(p),
                        Action::Install(i),
                        true,
                    );
                }
            }

            ui.set_clip(None);
            let pager_y = h - 42.0;
            let has_prev = self.catalog.offset >= crate::catalog::CATALOG_PAGE_SIZE;
            let has_next = if is_horizon_paged {
                self.catalog.offset + crate::catalog::CATALOG_PAGE_SIZE < total_horizon_items
            } else {
                self.catalog.results.len() >= crate::catalog::CATALOG_PAGE_SIZE
            };
            button(
                &mut ui,
                &mut self.targets,
                [content_x, pager_y, 102.0, 32.0],
                "Previous",
                Action::Previous,
                has_prev,
            );
            button(
                &mut ui,
                &mut self.targets,
                [content_x + 110.0, pager_y, 102.0, 32.0],
                "Next",
                Action::Next,
                has_next,
            );
            let page_num = (self.catalog.offset / crate::catalog::CATALOG_PAGE_SIZE) + 1;
            let page_info = if is_horizon_paged {
                let shown_first = if total_horizon_items == 0 {
                    0
                } else {
                    page_start + 1
                };
                format!(
                    "Page {page_num} of {total_pages}  •  Showing {shown_first}–{page_end} of {total_horizon_items}"
                )
            } else {
                format!("Page {page_num}  •  {} per page", crate::catalog::CATALOG_PAGE_SIZE)
            };
            let fitted_page_info = ui.fit_text(
                Face::Medium,
                12.5,
                &page_info,
                (content_w - 226.0).max(60.0),
            );
            ui.text(
                Face::Medium,
                12.5,
                content_x + 222.0,
                pager_y + 7.0,
                my3_muted(),
                &fitted_page_info,
            );
        } else if page == Page::Worlds {
            self.draw_worlds_page(&mut ui, content_x, content_w, h, compact, slide_y);
        } else if page == Page::Inventory {
            self.draw_inventory_page(&mut ui, content_x, content_w, h, compact, slide_y);
        } else if page == Page::ControlsStudio {
            self.draw_controls_studio_page(&mut ui, content_x, content_w, h, compact, slide_y);
        } else if page == Page::Compiler {
            self.draw_compiler_page(&mut ui, content_x, content_w, h, compact, slide_y);
        } else if page == Page::Installed {
            self.draw_installed(&mut ui, content_x, content_w, h, compact, slide_y);
        } else if page == Page::Replays {
            self.draw_replays_page(&mut ui, content_x, content_w, h, compact, slide_y);
        } else {
            self.draw_settings_page(
                &mut ui, content_x, content_w, h, compact, slide_y, ime_top_dp,
            );
        }

        // Bottom status strip when not in a full-screen progress dialog.
        let in_progress_dialog = (page == Page::Play && self.worker.is_some())
            || (self.catalog.downloading.is_some() && self.catalog.busy());
        if !in_progress_dialog {
            let show_corner_links = page == Page::Settings
                && self.catalog.settings_category == SettingsCategory::Overview;
            let status = self.error.as_deref().unwrap_or(&self.catalog.status);
            if !status.is_empty() && !page.is_catalog() && (page != Page::Play || self.error.is_some()) {
                let max_snack_w = if show_corner_links {
                    (content_w - 224.0).max(120.0)
                } else {
                    content_w.max(160.0)
                };
                ui.m3_snackbar_toast(
                    content_x,
                    h - 38.0,
                    max_snack_w,
                    status,
                    self.error.is_some(),
                    self.anim.toast_anim,
                );
            }
            if show_corner_links {
                button(
                    &mut ui,
                    &mut self.targets,
                    [w - 118.0, h - 38.0, 92.0, 28.0],
                    "GitHub",
                    Action::Repository,
                    false,
                );
                button(
                    &mut ui,
                    &mut self.targets,
                    [w - 222.0, h - 38.0, 96.0, 28.0],
                    "Discord",
                    Action::Discord,
                    false,
                );
            }
        }

        ui.set_stagger_transition(1.0, 0.0, 0.0);
        ui.h = full_h;
        // Material You 3 Project Detail Modal (Versions, Modrinth Description, Gallery Screenshots)
        if self.catalog.project_modal.is_some() {
            self.draw_project_modal(&mut ui, w, h);
        }

        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            self.login.draw(
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.editing,
                self.anim.modal_anim,
            );
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(&mut ui, &mut self.targets, self.anim.modal_anim);
            self.list_targets = chooser.navigation(ui.w);
            self.list_view = Some(chooser.viewport(ui.h));
        }
        } // end dashboard chrome (non-Music fullscreen)
        if let Some([top, bottom, maximum]) = self.list_view
            && maximum > 0.0
            && bottom > top
            && self.catalog.project_modal.is_none()
        {
            let scroll = self
                .chooser
                .as_ref()
                .map_or(self.catalog.scroll, |c| c.scroll);
            let height = bottom - top;
            let thumb = (height * height / (height + maximum)).max(20.0).min(height);
            let y = top + (height - thumb) * (scroll / maximum).clamp(0.0, 1.0);
            ui.rounded(w - 10.0, top, 4.0, height, 2.0, my3_border());
            ui.rounded(w - 10.0, y, 4.0, thumb, 2.0, my3_muted());
        }
        if self.catalog.fm_modal.is_some() {
            self.draw_fm_modal(&mut ui, w, h, ime_top_dp);
        }
        if let Some((_, ref dup_name, _)) = self.catalog.duplicate_preset_confirm.clone() {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            ui.rounded(
                0.0,
                0.0,
                w,
                h,
                0.0,
                horizon_client::hud::rgba(4, 5, 10, 210),
            );
            let mw = (w - 32.0).clamp(300.0, 440.0);
            let mh = 190.0;
            let mx = (w - mw) * 0.5;
            let my = (h - mh) * 0.5;
            ui.rounded(mx, my, mw, mh, 20.0, my3_surface());
            ui.border(mx, my, mw, mh, 20.0, my3_accent());
            ui.text(
                Face::SemiBold,
                16.5,
                mx + 20.0,
                my + 18.0,
                my3_text(),
                "Replace Existing Control Preset?",
            );
            let line1 = format!(
                "A control preset named '{}' already exists.",
                dup_name
            );
            let fitted1 = ui.fit_text(Face::Regular, 12.0, &line1, mw - 40.0);
            ui.text(
                Face::Regular,
                12.0,
                mx + 20.0,
                my + 52.0,
                my3_muted(),
                &fitted1,
            );
            let line2 = "Duplicate names are not allowed. Save to replace the current one?";
            let fitted2 = ui.fit_text(Face::Regular, 11.5, line2, mw - 40.0);
            ui.text(
                Face::Regular,
                11.5,
                mx + 20.0,
                my + 76.0,
                my3_muted(),
                &fitted2,
            );
            let bw = (mw - 52.0) * 0.5;
            button(
                &mut ui,
                &mut self.targets,
                [mx + 20.0, my + mh - 52.0, bw, 38.0],
                "Save",
                Action::ConfirmReplaceDuplicatePreset,
                true,
            );
            button(
                &mut ui,
                &mut self.targets,
                [mx + 32.0 + bw, my + mh - 52.0, bw, 38.0],
                "Cancel",
                Action::CancelReplaceDuplicatePreset,
                false,
            );
        }
        if let Some(result) = self
            .custom_photo_worker
            .as_ref()
            .and_then(|worker| worker.try_recv().ok())
        {
            self.custom_photo_worker = None;
            match result {
                Ok((rel, pw, ph, rgba, sample)) => {
                    self.custom_photo_loaded_path = Some(rel);
                    self.custom_photo_rgba = Some((pw, ph, rgba));
                    self.custom_photo_dirty = true;
                    if let Some((base, accent)) = sample {
                        self.wallpaper_color = Some(base);
                        self.wallpaper_accent = Some(accent);
                    } else {
                        self.wallpaper_color = None;
                        self.wallpaper_accent = None;
                    }
                    self.wallpaper_status = "Custom photo equipped".into();
                }
                Err(e) => {
                    self.wallpaper_status = e;
                }
            }
        }
        if self.awaiting_custom_wallpaper && !crate::documents::busy() {
            self.awaiting_custom_wallpaper = false;
            let dir = self.home.join("custom-wallpapers");
            let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && !self.custom_wallpaper_known.contains(&p) {
                        let mtime = entry
                            .metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        if newest.as_ref().is_none_or(|(_, t)| mtime > *t) {
                            newest = Some((p, mtime));
                        }
                    }
                }
            }
            if let Some((picked, _)) = newest {
                let ext = picked
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let is_video = matches!(
                    ext.as_str(),
                    "mp4" | "mkv" | "webm" | "mov" | "m4v" | "3gp" | "avi"
                );
                let is_photo = matches!(
                    ext.as_str(),
                    "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif"
                );
                if is_video || is_photo {
                    // Remove older custom wallpapers to keep storage tidy.
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        for entry in entries.flatten() {
                            let other = entry.path();
                            if other != picked && other.is_file() {
                                let _ = std::fs::remove_file(other);
                            }
                        }
                    }
                    let fname = picked
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let rel = format!("custom-wallpapers/{fname}");
                    crate::native_ime::wallpaper_stop();
                    self.catalog.config.custom_wallpaper_path = Some(rel);
                    self.catalog.config.custom_wallpaper_is_video = is_video;
                    self.wallpaper_has_frame = false;
                    self.wallpaper_color = None;
                    self.wallpaper_accent = None;
                    self.custom_photo_rgba = None;
                    self.custom_photo_loaded_path = None;
                    self.custom_photo_dirty = false;
                    let _ = self.catalog.config.save(&self.home);
                    self.wallpaper_status = if is_video {
                        format!("Custom video equipped: {fname}")
                    } else {
                        format!("Loading photo: {fname}...")
                    };
                    self.start_wallpaper();
                } else {
                    let _ = std::fs::remove_file(&picked);
                    self.wallpaper_status =
                        "Unsupported file. Pick a photo (PNG/JPG/WEBP/GIF) or video (MP4/WEBM/MKV/MOV)."
                            .into();
                }
            }
        }
        if let Some(result) = self.wallpaper_worker.as_ref().and_then(|worker| worker.try_recv().ok()) {
            self.wallpaper_worker = None;
            self.wallpaper_downloading = None;
            match result {
                Ok((idx, msg)) => {
                    if idx < self.wallpaper_downloaded.len() {
                        self.wallpaper_downloaded[idx] = true;
                    }
                    let _ = self.equip_wallpaper(idx);
                    self.wallpaper_status = msg;
                    if idx < self.wallpaper_previews.len() && self.wallpaper_previews[idx].is_none() {
                        let home = self.home.clone();
                        let (tx, rx) = mpsc::channel();
                        if self.wallpaper_preview_worker.is_none() {
                            self.wallpaper_preview_worker = Some(rx);
                            std::thread::spawn(move || {
                                if let Some((w, h, rgba, palette)) =
                                    crate::wallpapers::load_or_fetch_preview(&home, idx)
                                {
                                    let _ = tx.send((idx, w, h, rgba, palette));
                                }
                            });
                        }
                    }
                    if let Some(next) = self.wallpaper_queued_download.take()
                        && next != idx
                    {
                        if self.wallpaper_downloaded.get(next).copied().unwrap_or(false) {
                            let _ = self.equip_wallpaper(next);
                        } else {
                            self.begin_wallpaper_download(next);
                        }
                    }
                }
                Err(e) => {
                    self.wallpaper_queued_download = None;
                    self.wallpaper_status = format!("Download failed: {e}");
                }
            }
        }
        if self.wallpapers_open {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            let modal_scrim_a = (220.0 * self.catalog.config.theme.ui_opacity.clamp(0.10, 1.0)).round() as u8;
            ui.fill(0.0, 0.0, w, full_h, horizon_client::hud::rgba(4, 5, 10, modal_scrim_a));
            let mx = 14.0;
            let mw = (w - 28.0).max(240.0);
            let my = 10.0;
            let mh = (full_h - 20.0).max(200.0);
            ui.rounded(mx, my, mw, mh, 18.0, my3_surface());
            ui.border(mx, my, mw, mh, 18.0, my3_border());
            ui.text(Face::SemiBold, 16.5, mx + 16.0, my + 10.0, my3_text(), "Background images");
            let sub_hdr = format!(
                "{} live wallpapers · 100% resolution",
                crate::wallpapers::WALLPAPERS.len()
            );
            let upload_w = if mw >= 520.0 { 148.0 } else { 104.0 };
            let upload_lbl = if mw >= 520.0 { "Upload Photo/Video" } else { "Upload" };
            let hdr_btn_left = mx + mw - 160.0 - upload_w;
            let sub_max_w = (hdr_btn_left - (mx + 22.0)).max(80.0);
            let sub_fit = ui.fit_text(Face::Regular, 9.5, &sub_hdr, sub_max_w);
            ui.text(Face::Regular, 9.5, mx + 16.0, my + 30.0, my3_muted(), &sub_fit);
            button(
                &mut ui,
                &mut self.targets,
                [hdr_btn_left, my + 9.0, upload_w, 30.0],
                upload_lbl,
                Action::UploadCustomWallpaper,
                self.catalog.config.custom_wallpaper_path.is_some(),
            );
            button(&mut ui, &mut self.targets, [mx + mw - 154.0, my + 9.0, 74.0, 30.0], "Default", Action::ClearWallpaper, false);
            button(&mut ui, &mut self.targets, [mx + mw - 74.0, my + 9.0, 62.0, 30.0], "Close", Action::CloseWallpapers, false);
            let custom_banner_h = if self.catalog.config.custom_wallpaper_path.is_some() { 24.0 } else { 0.0 };
            if let Some(custom_rel) = &self.catalog.config.custom_wallpaper_path {
                let kind_lbl = if self.catalog.config.custom_wallpaper_is_video { "Video" } else { "Photo" };
                let fname = custom_rel.rsplit('/').next().unwrap_or(custom_rel);
                let banner = ui.fit_text(
                    Face::Medium,
                    10.5,
                    &format!("Custom {kind_lbl} active: {fname}"),
                    mw - 36.0,
                );
                ui.text(Face::Medium, 10.5, mx + 16.0, my + 48.0, my3_accent(), &banner);
            }
            let grid_y = my + 50.0 + custom_banner_h;
            let footer_h = 80.0;
            let grid_h = (mh - 50.0 - custom_banner_h - footer_h).max(72.0);
            let grid_bottom = grid_y + grid_h;
            let grid_x = mx + 12.0;
            let grid_w = (mw - 28.0).max(160.0);
            let cols = if grid_w >= 560.0 { 3usize } else { 2usize };
            let gap = 10.0;
            let card_w = ((grid_w - gap * (cols as f32 - 1.0)) / cols as f32).max(96.0);
            let img_pad = 6.0;
            let img_w = (card_w - img_pad * 2.0).max(64.0);
            let img_h = (img_w * 9.0 / 16.0).round();
            let card_h = img_h + 44.0;
            let total_rows = crate::wallpapers::WALLPAPERS.len().div_ceil(cols);
            let total_h = if total_rows == 0 {
                0.0
            } else {
                total_rows as f32 * card_h + (total_rows.saturating_sub(1)) as f32 * gap
            };
            let max_scroll = (total_h - grid_h).max(0.0);
            self.wallpaper_scroll = self.wallpaper_scroll.clamp(0.0, max_scroll);
            self.list_view = Some([grid_y, grid_bottom, max_scroll]);

            let (dl_done, dl_total) = if self.wallpaper_worker.is_some() {
                crate::wallpapers::download_progress()
            } else {
                (0, 0)
            };
            let dl_frac = if dl_total > 0 {
                (dl_done as f32 / dl_total as f32).clamp(0.0, 0.99)
            } else {
                0.0
            };
            let dl_pct_str = if dl_total > 0 {
                format!("{:.0}%", dl_frac * 100.0)
            } else {
                "...".to_owned()
            };
            let active_idx = crate::wallpapers::effective_index(self.catalog.config.wallpaper_index);
            let custom_active = self.catalog.config.custom_wallpaper_path.is_some();

            ui.clipped([mx + 8.0, grid_y, mw - 18.0, grid_h], |ui| {
                for (i, item) in crate::wallpapers::WALLPAPERS.iter().enumerate() {
                    let row = i / cols;
                    let col = i % cols;
                    let card_x = grid_x + col as f32 * (card_w + gap);
                    let card_y = grid_y + row as f32 * (card_h + gap) - self.wallpaper_scroll;
                    if card_y + card_h < grid_y || card_y > grid_bottom {
                        continue;
                    }
                    let downloaded = self.wallpaper_downloaded.get(i).copied().unwrap_or(false);
                    let selected = !custom_active && active_idx == i;
                    let is_downloading = self.wallpaper_downloading == Some(i);
                    let is_queued = self.wallpaper_queued_download == Some(i);

                    let card_bg = if selected {
                        my3_accent_soft()
                    } else {
                        my3_card_bg()
                    };
                    let card_border = if selected || is_downloading {
                        my3_accent()
                    } else {
                        my3_card_border()
                    };
                    ui.rounded(card_x, card_y, card_w, card_h, 13.0, card_bg);
                    ui.border(card_x, card_y, card_w, card_h, 13.0, card_border);

                    let img_x = card_x + img_pad;
                    let img_y = card_y + img_pad;
                    ui.rounded(img_x, img_y, img_w, img_h, 9.0, my3_surface_high());
                    if let Some((pw, ph, rgba)) = self.wallpaper_previews.get(i).and_then(|p| p.as_ref()) {
                        let thumb_key = format!("wp-thumb-{i}");
                        let _ = ui.image(&thumb_key, *pw, *ph, rgba, img_x, img_y, img_w, img_h);
                    } else {
                        let ph_lbl = ui.fit_text(Face::Medium, 9.5, item.name, img_w - 16.0);
                        ui.centered_fit(
                            Face::Medium,
                            9.5,
                            [img_x + 8.0, img_y, img_w - 16.0, img_h],
                            my3_muted(),
                            &ph_lbl,
                        );
                    }

                    // Bottom gradient scrim on thumbnail for crisp badge legibility
                    ui.rounded(
                        img_x,
                        img_y + img_h - 20.0,
                        img_w,
                        20.0,
                        0.0,
                        horizon_client::hud::rgba(6, 8, 14, 135),
                    );

                    // Top-left resolution pill
                    let res_w = (item.resolution.len() as f32 * 6.5 + 12.0).max(32.0);
                    ui.rounded(
                        img_x + 5.0,
                        img_y + 5.0,
                        res_w,
                        16.0,
                        8.0,
                        horizon_client::hud::rgba(8, 10, 18, 195),
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        8.5,
                        [img_x + 5.0, img_y + 5.0, res_w, 16.0],
                        horizon_client::hud::rgba(245, 245, 252, 245),
                        item.resolution,
                    );

                    // Top-right status pill
                    let (badge_txt, badge_bg, badge_fg) = if selected {
                        ("Equipped".to_owned(), my3_accent(), my3_on_accent())
                    } else if is_downloading {
                        (dl_pct_str.clone(), my3_accent(), my3_on_accent())
                    } else if is_queued {
                        ("Queued".to_owned(), my3_surface_high(), my3_text())
                    } else if downloaded {
                        ("Saved".to_owned(), horizon_client::hud::rgba(14, 18, 28, 210), horizon_client::hud::rgba(235, 245, 255, 245))
                    } else {
                        (
                            format!("{:.1} MB", item.size as f64 / 1_000_000.0),
                            horizon_client::hud::rgba(10, 12, 20, 205),
                            horizon_client::hud::rgba(240, 242, 255, 240),
                        )
                    };
                    let badge_w = (badge_txt.len() as f32 * 5.8 + if !downloaded && !is_downloading && !is_queued && !selected { 22.0 } else { 14.0 }).clamp(42.0, img_w * 0.58);
                    let badge_x = img_x + img_w - badge_w - 5.0;
                    ui.rounded(badge_x, img_y + 5.0, badge_w, 16.0, 8.0, badge_bg);
                    if !downloaded && !is_downloading && !is_queued && !selected {
                        ui.icon(Icon::Download, badge_x + 5.0, img_y + 8.0, 10.0, badge_fg);
                        ui.centered_fit(
                            Face::SemiBold,
                            8.2,
                            [badge_x + 15.0, img_y + 5.0, badge_w - 18.0, 16.0],
                            badge_fg,
                            &badge_txt,
                        );
                    } else {
                        ui.centered_fit(
                            Face::SemiBold,
                            8.2,
                            [badge_x + 3.0, img_y + 5.0, badge_w - 6.0, 16.0],
                            badge_fg,
                            &badge_txt,
                        );
                    }

                    // Active download bar along bottom edge of thumbnail
                    if is_downloading {
                        ui.rounded(
                            img_x + 4.0,
                            img_y + img_h - 6.0,
                            img_w - 8.0,
                            4.0,
                            2.0,
                            horizon_client::hud::rgba(0, 0, 0, 160),
                        );
                        ui.rounded(
                            img_x + 4.0,
                            img_y + img_h - 6.0,
                            ((img_w - 8.0) * dl_frac).max(6.0),
                            4.0,
                            2.0,
                            my3_accent(),
                        );
                    }

                    // Title + subtle status line
                    let title_y = img_y + img_h + 5.0;
                    let title = ui.fit_text(Face::SemiBold, 11.0, item.name, card_w - 14.0);
                    ui.text(Face::SemiBold, 11.0, card_x + 7.0, title_y, my3_text(), &title);
                    let sub_str = if selected {
                        "Active theme & wallpaper".to_owned()
                    } else if is_downloading {
                        format!("Downloading {}...", dl_pct_str)
                    } else if is_queued {
                        "Queued to download...".to_owned()
                    } else if downloaded {
                        "Tap to use".to_owned()
                    } else {
                        format!("Tap to download · {:.1} MB", item.size as f64 / 1_000_000.0)
                    };
                    let sub_col = if selected || is_downloading { my3_accent() } else { my3_muted() };
                    let sub_fit = ui.fit_text(Face::Regular, 9.0, &sub_str, card_w - 14.0);
                    ui.text(Face::Regular, 9.0, card_x + 7.0, title_y + 16.0, sub_col, &sub_fit);

                    let hit_y0 = card_y.max(grid_y);
                    let hit_y1 = (card_y + card_h).min(grid_bottom);
                    if hit_y1 > hit_y0 {
                        let action = if downloaded {
                            Action::EquipWallpaper(i)
                        } else {
                            Action::DownloadWallpaper(i)
                        };
                        self.targets.push(([card_x, hit_y0, card_w, hit_y1 - hit_y0], action));
                    }
                }
            });

            if max_scroll > 0.0 {
                let thumb = (grid_h * grid_h / (grid_h + max_scroll)).max(24.0).min(grid_h);
                let sb_y = grid_y + (grid_h - thumb) * (self.wallpaper_scroll / max_scroll).clamp(0.0, 1.0);
                ui.rounded(mx + mw - 10.0, grid_y, 4.0, grid_h, 2.0, my3_border());
                ui.rounded(mx + mw - 10.0, sb_y, 4.0, thumb, 2.0, my3_accent());
            }

            // Live UI Opacity slider right inside Background images modal
            let op_y = my + mh - 74.0;
            ui.fill(mx + 14.0, op_y - 4.0, mw - 28.0, 1.0, my3_border());
            let op_val = self.catalog.config.theme.ui_opacity.clamp(0.10, 1.0);
            let op_frac = ((op_val - 0.10) / 0.90).clamp(0.0, 1.0);
            let op_pct = format!("UI Opacity: {:.0}%", op_val * 100.0);
            ui.text(Face::SemiBold, 11.0, mx + 16.0, op_y + 9.0, my3_text(), &op_pct);
            let minus_r = [mx + 148.0, op_y + 2.0, 30.0, 30.0];
            let plus_r = [mx + mw - 46.0, op_y + 2.0, 30.0, 30.0];
            let track_x = minus_r[0] + minus_r[2] + 8.0;
            let track_w = (plus_r[0] - track_x - 8.0).max(40.0);
            let bar_x = track_x + 6.0;
            let bar_w = (track_w - 12.0).max(24.0);
            let bar_y = op_y + 14.0;
            let is_dragging = ACTIVE_SLIDER_DRAG.with(|c| {
                c.get().is_some_and(|(act, _, _)| matches!(act, Action::UiOpacityDelta(_)))
            });
            ui.m3_expressive_slider(bar_x, bar_y, bar_w, 6.0, op_frac, is_dragging);
            SLIDER_TRACK_TARGETS.with(|t| {
                t.borrow_mut().push(([track_x, op_y, track_w, 34.0], Action::UiOpacityDelta(-1), bar_x, bar_w));
            });
            button(&mut ui, &mut self.targets, minus_r, "-", Action::UiOpacityDelta(-1), false);
            button(&mut ui, &mut self.targets, plus_r, "+", Action::UiOpacityDelta(1), true);

            let playback = crate::native_ime::wallpaper_status().unwrap_or_default();
            let live_dl_msg = if self.wallpaper_worker.is_some() {
                if dl_total > 0 {
                    let pct = (dl_done as f64 / dl_total as f64 * 100.0).clamp(0.0, 99.0);
                    Some(format!(
                        "{}  ({:.0}%  ·  {:.1} / {:.1} MB)",
                        self.wallpaper_status,
                        pct,
                        dl_done as f64 / 1_000_000.0,
                        dl_total as f64 / 1_000_000.0
                    ))
                } else {
                    Some(self.wallpaper_status.clone())
                }
            } else {
                None
            };
            let message = if let Some(ref dl_msg) = live_dl_msg {
                dl_msg.as_str()
            } else if self.catalog.config.custom_wallpaper_path.is_some()
                && !self.catalog.config.custom_wallpaper_is_video
            {
                &self.wallpaper_status
            } else if !playback.is_empty() && self.wallpaper_worker.is_none() {
                &playback
            } else {
                &self.wallpaper_status
            };
            let line = ui.fit_text(Face::Regular, 10.0, message, mw - 32.0);
            ui.text(Face::Regular, 10.0, mx + 16.0, my + mh - 36.0, my3_muted(), &line);
            ui.text(Face::Regular, 9.0, mx + 16.0, my + mh - 20.0, my3_muted(), "Launcher colors adapt to your active wallpaper. Tap any tile to download & use.");
        }
        if self.diagnostics_modal_open {
            self.draw_diagnostics_summary_modal(&mut ui, w, h);
        }
        self.draw_update_fullscreen_ui(&mut ui, w, h);
        if self.exit_confirmation {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            let modal_t = self.anim.modal_anim.clamp(0.2, 1.0);
            ui.rounded(
                0.0,
                0.0,
                w,
                h,
                0.0,
                horizon_client::hud::rgba(0, 0, 0, (180.0 * modal_t) as u8),
            );
            let scale_in = 0.94 + 0.06 * modal_t;
            let width = (w - 32.0).min(360.0) * scale_in;
            let x = (w - width) * 0.5;
            let y = (h - 160.0) * 0.5 + (1.0 - modal_t) * 8.0;
            ui.rounded(x, y, width, 160.0, 20.0, my3_surface());
            ui.border(x, y, width, 160.0, 20.0, my3_border());
            ui.text(
                Face::SemiBold,
                20.0,
                x + 22.0,
                y + 24.0,
                my3_text(),
                "Exit volt plus?",
            );
            let bw = (width - 52.0) * 0.5;
            button(
                &mut ui,
                &mut self.targets,
                [x + 20.0, y + 94.0, bw, 42.0],
                "Cancel",
                Action::Dismiss,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [x + 32.0 + bw, y + 94.0, bw, 42.0],
                "Exit",
                Action::ConfirmExit,
                true,
            );
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            ui.border(*x - 2.0, *y - 2.0, *w + 4.0, *h + 4.0, 12.0, my3_accent());
            self.cursor = (
                f64::from((x + w / 2.0) * ui.density),
                f64::from((y + h / 2.0) * ui.density),
            );
        }
        if self.mouse_down || self.drag.as_ref().is_some_and(|drag| !drag.moved) {
            let point = [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ];
            if let Some(([x, y, w, h], _)) = self.targets.iter().rev().find(|(rect, _)| {
                point[0] >= rect[0]
                    && point[0] <= rect[0] + rect[2]
                    && point[1] >= rect[1]
                    && point[1] <= rect[1] + rect[3]
            }) {
                let sc = horizon_client::ui_theme::motion::press_scale(true, 0.0);
                let sw = *w * sc;
                let sh = *h * sc;
                let sx = *x + (*w - sw) * 0.5;
                let sy = *y + (*h - sh) * 0.5;
                let rad = horizon_client::ui_theme::motion::morph_radius(
                    (*h * 0.22).clamp(6.0, 10.0),
                    6.0,
                    4.5,
                    0.0,
                    1.0,
                );
                ui.rounded(
                    sx,
                    sy,
                    sw,
                    sh,
                    rad,
                    horizon_client::hud::rgba(255, 255, 255, 38),
                );
                ui.border(sx, sy, sw, sh, rad, my3_accent());
            }
        }
        if !self.mouse_down
            && self.drag.is_none()
            && let Some((action, time)) = self.feedback
            && time.elapsed() < std::time::Duration::from_millis(260)
            && let Some(([x, y, w, h], _)) = self.targets.iter().find(|(_, a)| *a == action)
        {
            let p = (time.elapsed().as_secs_f32() / 0.26).clamp(0.0, 1.0);
            let spring = horizon_client::ui_theme::motion::spring_expressive_spatial(p);
            let fade = (1.0 - horizon_client::ui_theme::motion::spring_expressive_effects(p)).clamp(0.0, 1.0);
            let rad = horizon_client::ui_theme::motion::morph_radius(
                (*h * 0.22).clamp(6.0, 10.0),
                7.0,
                5.0,
                spring,
                1.0 - p,
            );
            let expand = spring * 3.2;
            ui.rounded(
                *x,
                *y,
                *w,
                *h,
                rad,
                horizon_client::hud::rgba(255, 255, 255, (42.0 * fade) as u8),
            );
            ui.border(
                *x - expand,
                *y - expand,
                *w + expand * 2.0,
                *h + expand * 2.0,
                rad + expand * 0.6,
                color::with_alpha(my3_accent(), 0.78 * fade),
            );
        }
        crate::documents::draw_ui(&mut ui);
        // Do not activate controls before they have finished entering.
        if reveal < 1.0 {
            self.targets.clear();
            self.list_targets.clear();
        }
        self.draw_dashboard_tutorial(&mut ui, w, full_h, h, reveal);
        let pointer_density = ui.density;
        ui.finish();
        if crate::keyboard_mouse_input() {
            horizon_client::touch_ui::draw_pointer_styled(
                &mut b,
                &gui.atlas,
                (
                    self.cursor.0 as f32 / pointer_density,
                    self.cursor.1 as f32 / pointer_density,
                ),
                self.mouse_touch.virtual_mouse_pointer_scale,
                self.mouse_touch.virtual_mouse_opacity,
                self.mouse_touch.virtual_mouse_cursor,
            );
        }

        let rendered = r.render(&game::loading_frame(&b));
        self.recycle_wallpaper_buffer(&mut b);
        let rendered = rendered?;
        if !rendered && !b.patches.is_empty() {
            // A suspended/lost surface skips the frame, including its glyph
            // uploads. Rebuild them next time instead of caching invisible text.
            horizon_client::ui_theme::reset_atlas();
        }
        if rendered && !self.anim.intro_done {
            self.anim.intro_done = true;
            self.anim.reveal_start = Some(std::time::Instant::now());
            self.anim.trigger_page_transition();
        }
        Ok(())
    }

    /// Material Design 3 Library & Content Manager with two top sub-views:
    /// View 1: `InstalledSubTab::Library` (Search, Category Filter Pills, Sort, Rich M3 Cards)
    /// View 2: `InstalledSubTab::FileManager` (Real filesystem manager with Back/Forward history,
    /// Breadcrumbs, Multi-select, Copy/Cut/Paste, Import, Export, Rename, New Folder, Delete, Details)
    /// Material 3 Expressive `horizon-port-pack` Native Porting & Compiler Workbench (`Page::Compiler`).
    fn draw_compiler_page(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
    ) {
        use crate::catalog::CompilerMode;
        use horizon_client::content_files::Kind;

        let header_y = 60.0 + slide_y;
        ui.text(
            Face::SemiBold,
            18.5,
            content_x,
            header_y + 2.0,
            my3_text(),
            if compact {
                "Port Pack Compiler"
            } else {
                "horizon-port-pack & Native Compiler"
            },
        );

        // Top-right 4-segment Material 2026 Connected Button Group for CompilerMode
        let mode_count = CompilerMode::ALL.len();
        let seg_w = if compact { 76.0 } else { 98.0 };
        let group_x = content_x + content_w - seg_w * (mode_count as f32) - (mode_count as f32 - 1.0) * 3.0;
        for (i, mode) in CompilerMode::ALL.into_iter().enumerate() {
            let rx = group_x + i as f32 * (seg_w + 3.0);
            let rect = [rx, header_y - 1.0, seg_w, 30.0];
            let active = self.catalog.compiler_mode == mode;
            ui.m3_connected_segment(
                rect,
                i,
                mode_count,
                mode.label(),
                active,
                Some(Icon::Terminal),
            );
            self.targets.push((rect, Action::SetCompilerMode(mode)));
        }

        // Row 1: Modrinth Slug / URL / File Path Input + "Port & Compile" / "Build Native .so" CTA + "+ Pick .jar"
        let row1_y = 96.0 + slide_y;
        let pick_w = if compact { 88.0 } else { 108.0 };
        let run_w = if compact { 98.0 } else { 122.0 };
        let field_w = (content_w - pick_w - run_w - 14.0).max(100.0);
        let hint = match self.catalog.compiler_mode {
            CompilerMode::JarMod => "Modrinth mod slug/URL (e.g. appleskin) or .jar path...",
            CompilerMode::Modpack => "Modrinth modpack slug/URL or local .mrpack path...",
            CompilerMode::DataPackUpgrade => "Enter 1.21 / 26.2 data pack path to upgrade to 26.3...",
            CompilerMode::NativeSoBuild => "Tap 'Build Native .so' to bundle all active .hnmod/.hmod mods...",
        };
        let query_r = ui.text_field(
            content_x,
            row1_y,
            field_w,
            34.0,
            "",
            &self.catalog.compiler_query,
            hint,
            self.catalog.editing == Some("compiler_query"),
        );
        self.targets.push((query_r, Action::Field("compiler_query")));

        button(
            ui,
            &mut self.targets,
            [content_x + field_w + 7.0, row1_y, run_w, 34.0],
            if self.catalog.compiler_mode == CompilerMode::NativeSoBuild {
                "Build Native .so"
            } else {
                "Port & Compile"
            },
            Action::CompilerRunQuery,
            true,
        );
        button(
            ui,
            &mut self.targets,
            [content_x + field_w + run_w + 14.0, row1_y, pick_w, 34.0],
            "+ Pick .jar",
            Action::Import(Kind::Mods),
            false,
        );

        // Row 2: Live `horizon-port-pack` Diagnostic & Coverage Card
        let diag_y = 136.0 + slide_y;
        let diag_h = 96.0;
        ui.rounded(content_x, diag_y, content_w, diag_h, 12.0, my3_surface());
        ui.border(content_x, diag_y, content_w, diag_h, 12.0, my3_accent());

        if let Some(ref res) = self.catalog.last_port_result {
            let verdict_str = format!("{} • {}", res.loader.to_uppercase(), res.verdict.label());
            let vw = (ui.measure(Face::SemiBold, 10.5, &verdict_str) + 18.0).clamp(72.0, 168.0);
            let vx = content_x + content_w - vw - 12.0;
            ui.rounded(vx, diag_y + 10.0, vw, 22.0, 6.0, my3_accent_soft());
            ui.border(vx, diag_y + 10.0, vw, 22.0, 6.0, my3_accent());
            ui.centered_fit(
                Face::SemiBold,
                10.0,
                [vx + 4.0, diag_y + 10.0, vw - 8.0, 22.0],
                my3_accent(),
                &verdict_str,
            );

            let hdr = format!("{} ({} v{})", res.name, res.mod_id, res.version);
            let fit_hdr = ui.fit_text(Face::SemiBold, 14.5, &hdr, (content_w - vw - 36.0).max(80.0));
            ui.text(Face::SemiBold, 14.5, content_x + 14.0, diag_y + 8.0, my3_text(), &fit_hdr);

            let fit_sum = ui.fit_text(Face::Regular, 11.5, &res.summary, content_w - 28.0);
            ui.text(Face::Regular, 11.5, content_x + 14.0, diag_y + 30.0, my3_muted(), &fit_sum);

            // 4 Metric Chips (Blocks, Items, 26.3 Recipes, Hooks/Classes)
            let metrics = [
                format!("Blocks: {}", res.blocks_ported),
                format!("Items: {}", res.items_ported),
                format!("26.3 Recipes: {}", res.recipes_upgraded),
                format!(
                    "Hooks: {} • Classes: {}",
                    res.mapped_hooks.len(),
                    res.classes_scanned
                ),
            ];
            let chip_gap = 6.0;
            let chip_w = ((content_w - 28.0 - chip_gap * 3.0) / 4.0).max(54.0);
            let chip_y = diag_y + 52.0;
            for (idx, m_lbl) in metrics.iter().enumerate() {
                let cx = content_x + 14.0 + idx as f32 * (chip_w + chip_gap);
                ui.rounded(cx, chip_y, chip_w, 20.0, 6.0, my3_card_bg());
                ui.border(cx, chip_y, chip_w, 20.0, 6.0, my3_card_border());
                ui.centered_fit(
                    Face::Medium,
                    10.0,
                    [cx + 3.0, chip_y, chip_w - 6.0, 20.0],
                    my3_accent(),
                    m_lbl,
                );
            }

            let log_line = self
                .catalog
                .compiler_logs
                .last()
                .map(String::as_str)
                .unwrap_or("Carried over into horizon-mods/ & horizon-native-mods/");
            let fit_log = ui.fit_text(Face::Mono, 10.2, log_line, content_w - 28.0);
            ui.text(Face::Mono, 10.2, content_x + 14.0, diag_y + 76.0, my3_text(), &fit_log);
        } else {
            ui.rounded(content_x + 14.0, diag_y + 12.0, 38.0, 38.0, 10.0, my3_accent_soft());
            ui.icon(Icon::Terminal, content_x + 23.0, diag_y + 21.0, 20.0, my3_accent());
            ui.text(
                Face::SemiBold,
                14.5,
                content_x + 62.0,
                diag_y + 10.0,
                my3_text(),
                "Native horizon-port-pack Pipeline (In-Process Android)",
            );
            let sub = "Carries over 1.21+ blocks, items, fluids, 26.3 recipes, loot tables & worldgen + synthesizes Hook API 415.";
            let fit_sub = ui.fit_text(Face::Regular, 11.5, sub, content_w - 76.0);
            ui.text(Face::Regular, 11.5, content_x + 62.0, diag_y + 31.0, my3_muted(), &fit_sub);

            for (i, line) in self.catalog.compiler_logs.iter().take(2).enumerate() {
                let fit_l = ui.fit_text(Face::Mono, 10.5, line, content_w - 28.0);
                ui.text(
                    Face::Mono,
                    10.5,
                    content_x + 14.0,
                    diag_y + 54.0 + i as f32 * 18.0,
                    my3_accent(),
                    &fit_l,
                );
            }
        }

        // Row 3: Scrollable Local Profile Mods & Port Queue
        let list_top = diag_y + diag_h + 8.0;
        let list_bottom = h - 42.0;
        let mod_items: Vec<(usize, crate::catalog::LocalItem)> = self
            .catalog
            .local
            .iter()
            .cloned()
            .enumerate()
            .filter(|(_, item)| item.kind == Kind::Mods)
            .collect();

        let row_h = 58.0;
        let total_h = mod_items.len() as f32 * row_h;
        let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
        self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
        self.list_view = Some([list_top, list_bottom, max_scroll]);

        ui.set_clip(Some([
            content_x - 2.0,
            list_top,
            content_w + 4.0,
            (list_bottom - list_top).max(1.0),
        ]));
        for (row_idx, (orig_idx, item)) in mod_items.into_iter().enumerate() {
            let y = list_top + row_idx as f32 * row_h - self.catalog.scroll;
            let card_h = 52.0;
            let port_btn_r = [content_x + content_w - 118.0, y + 9.0, 106.0, 34.0];
            self.list_targets
                .push((port_btn_r, Action::CompilerPortLocal(orig_idx)));
            if y + card_h < list_top || y > list_bottom {
                continue;
            }
            ui.rounded(content_x, y, content_w, card_h, 12.0, my3_card_bg());
            ui.border(content_x, y, content_w, card_h, 12.0, my3_card_border());
            ui.rounded(content_x + 10.0, y + 8.0, 36.0, 36.0, 9.0, my3_accent_soft());
            ui.icon(Icon::Mods, content_x + 19.0, y + 17.0, 18.0, my3_accent());

            let max_tw = (content_w - 190.0).max(70.0);
            let fit_t = ui.fit_text(Face::SemiBold, 13.8, &item.title, max_tw);
            ui.text(Face::SemiBold, 13.8, content_x + 56.0, y + 7.0, my3_text(), &fit_t);
            let fit_s = ui.fit_text(Face::Regular, 11.0, &item.subtitle, max_tw);
            ui.text(Face::Regular, 11.0, content_x + 56.0, y + 27.0, my3_muted(), &fit_s);

            button(
                ui,
                &mut self.targets,
                port_btn_r,
                "Port / Inspect",
                Action::CompilerPortLocal(orig_idx),
                true,
            );
        }
        ui.set_clip(None);
    }

    fn draw_replays_page(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
    ) {
        use crate::catalog::ReplaysSubTab;

        let header_y = 60.0 + slide_y;
        ui.text(
            Face::SemiBold,
            19.0,
            content_x,
            header_y + 2.0,
            my3_text(),
            "Replays",
        );

        let seg_w = if compact { 92.0 } else { 112.0 };
        let tabs = [
            (ReplaysSubTab::Raw, "Raw Replay"),
            (ReplaysSubTab::Rendered, "Rendered Replay"),
        ];
        let refresh_w = 82.0;
        let refresh_r = [
            content_x + content_w - refresh_w,
            header_y,
            refresh_w,
            30.0,
        ];
        button(
            ui,
            &mut self.targets,
            refresh_r,
            "Refresh",
            Action::RefreshReplays,
            false,
        );
        let seg_start = refresh_r[0] - seg_w * 2.0 - 10.0;
        for (i, (tab, lbl)) in tabs.into_iter().enumerate() {
            let r = [seg_start + i as f32 * (seg_w + 3.0), header_y, seg_w, 30.0];
            ui.m3_connected_segment(
                r,
                i,
                2,
                lbl,
                self.catalog.replays_sub_tab == tab,
                None,
            );
            self.targets.push((r, Action::SetReplaysSubTab(tab)));
        }

        match self.catalog.replays_sub_tab {
            ReplaysSubTab::Raw => {
                let rs = horizon_client::replay::settings();
                let chip_y = 98.0 + slide_y;
                let chip_gap = 6.0;
                let open_dir_w = 96.0;
                let cw = ((content_w - open_dir_w - chip_gap * 3.0) / 3.0).min(148.0);
                let chips = [
                    (
                        if rs.auto_record_sp {
                            "Auto SP: ON"
                        } else {
                            "Auto SP: OFF"
                        },
                        rs.auto_record_sp,
                        Action::ReplayToggleAutoSp,
                    ),
                    (
                        if rs.auto_record_mp {
                            "Auto MP: ON"
                        } else {
                            "Auto MP: OFF"
                        },
                        rs.auto_record_mp,
                        Action::ReplayToggleAutoMp,
                    ),
                    (
                        if rs.record_self {
                            "Self Player: ON"
                        } else {
                            "Self Player: OFF"
                        },
                        rs.record_self,
                        Action::ReplayToggleRecordSelf,
                    ),
                ];
                for (i, (label, on, act)) in chips.into_iter().enumerate() {
                    let r = [content_x + i as f32 * (cw + chip_gap), chip_y, cw, 28.0];
                    ui.m3_filter_chip(r, label, on, None);
                    self.targets.push((r, act));
                }
                button(
                    ui,
                    &mut self.targets,
                    [content_x + content_w - open_dir_w, chip_y, open_dir_w, 28.0],
                    "Open Folder",
                    Action::OpenRawReplaysFolder,
                    false,
                );

                let info_y = chip_y + 34.0;
                let info_txt = ui.fit_text(
                    Face::Regular,
                    10.2,
                    &format!(
                        "{} raw .mcpr replay(s)  •  Stored in replays/raw/",
                        self.catalog.replays.len()
                    ),
                    content_w,
                );
                ui.text(
                    Face::Regular,
                    10.2,
                    content_x,
                    info_y,
                    my3_muted(),
                    &info_txt,
                );

                let list_top = info_y + 20.0;
                let list_bottom = (h - 46.0).max(list_top + 60.0);
                let row_h = if compact { 82.0 } else { 72.0 };
                let row_gap = 8.0;
                let total_h = self.catalog.replays.len() as f32 * (row_h + row_gap);
                let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let replays = self.catalog.replays.clone();
                let delete_confirm = self.catalog.replay_delete_confirm;
                let list_targets = &mut self.list_targets;
                ui.clipped([content_x - 2.0, list_top, content_w + 4.0, list_bottom - list_top], |ui| {
                    if replays.is_empty() {
                        ui.rounded(content_x, list_top + 6.0, content_w, 84.0, 14.0, my3_card_bg());
                        ui.border(content_x, list_top + 6.0, content_w, 84.0, 14.0, my3_card_border());
                        ui.text(
                            Face::SemiBold,
                            13.0,
                            content_x + 14.0,
                            list_top + 20.0,
                            my3_text(),
                            "No .mcpr raw replays recorded yet",
                        );
                        let hint = ui.fit_text(
                            Face::Regular,
                            10.8,
                            "Enable Auto SP/MP above or start recording from Control Hub -> Replay in-game.",
                            content_w - 28.0,
                        );
                        ui.text(
                            Face::Regular,
                            10.8,
                            content_x + 14.0,
                            list_top + 44.0,
                            my3_muted(),
                            &hint,
                        );
                        return;
                    }
                    for (i, item) in replays.iter().enumerate() {
                        let ry = list_top + i as f32 * (row_h + row_gap) - self.catalog.scroll;
                        if ry + row_h < list_top || ry > list_bottom {
                            continue;
                        }
                        ui.rounded(content_x, ry, content_w, row_h, 13.0, my3_card_bg());
                        ui.border(content_x, ry, content_w, row_h, 13.0, my3_card_border());
                        let badge = if item.meta.singleplayer {
                            "Singleplayer"
                        } else {
                            "Multiplayer"
                        };
                        let title = ui.fit_text(Face::SemiBold, 13.0, &item.display_name, content_w - 230.0);
                        ui.text(
                            Face::SemiBold,
                            13.0,
                            content_x + 14.0,
                            ry + 11.0,
                            my3_text(),
                            &title,
                        );
                        let size_mb = item.size_bytes as f64 / 1_048_576.0;
                        let dur = horizon_client::replay::format_duration_ms(item.meta.duration);
                        let meta = ui.fit_text(
                            Face::Regular,
                            10.4,
                            &format!(
                                "{badge}  •  {dur}  •  {size_mb:.2} MB  •  {} KF  •  {} Mrk",
                                item.keyframes_count, item.markers_count
                            ),
                            content_w - 210.0,
                        );
                        ui.text(
                            Face::Regular,
                            10.4,
                            content_x + 14.0,
                            ry + 33.0,
                            my3_muted(),
                            &meta,
                        );
                        let recorded_str = chrono::DateTime::from_timestamp_millis(item.meta.date as i64)
                            .map(|dt| {
                                dt.with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_else(|| format!("MC {}", item.meta.mcversion));
                        let date_line = ui.fit_text(
                            Face::Regular,
                            9.6,
                            &recorded_str,
                            content_w - 210.0,
                        );
                        ui.text(
                            Face::Regular,
                            9.6,
                            content_x + 14.0,
                            ry + 50.0,
                            my3_muted(),
                            &date_line,
                        );

                        let open_w = 104.0;
                        let del_w = 72.0;
                        let open_r = [
                            content_x + content_w - open_w - del_w - 18.0,
                            ry + (row_h - 32.0) * 0.5,
                            open_w,
                            32.0,
                        ];
                        let del_r = [
                            content_x + content_w - del_w - 12.0,
                            ry + (row_h - 32.0) * 0.5,
                            del_w,
                            32.0,
                        ];
                        ui.rounded(open_r[0], open_r[1], open_r[2], open_r[3], 9.0, my3_accent());
                        ui.centered_fit(
                            Face::SemiBold,
                            11.0,
                            open_r,
                            horizon_client::ui_theme::active_theme().on_accent,
                            "Open Viewer",
                        );
                        if let Some(c) = ui.clip_rect(open_r) {
                            self.targets.push((c, Action::PlayReplay(i)));
                            list_targets.push((c, Action::PlayReplay(i)));
                        }
                        let confirming = delete_confirm == Some(i);
                        ui.rounded(
                            del_r[0],
                            del_r[1],
                            del_r[2],
                            del_r[3],
                            9.0,
                            if confirming {
                                horizon_client::hud::rgba(220, 38, 38, 220)
                            } else {
                                my3_surface()
                            },
                        );
                        ui.border(del_r[0], del_r[1], del_r[2], del_r[3], 9.0, my3_card_border());
                        ui.centered_fit(
                            Face::SemiBold,
                            10.6,
                            del_r,
                            my3_text(),
                            if confirming { "Confirm" } else { "Delete" },
                        );
                        if let Some(c) = ui.clip_rect(del_r) {
                            self.targets.push((c, Action::DeleteReplay(i)));
                            list_targets.push((c, Action::DeleteReplay(i)));
                        }
                    }
                });
            }
            ReplaysSubTab::Rendered => {
                let bar_y = 98.0 + slide_y;
                let info_txt = ui.fit_text(
                    Face::Regular,
                    10.4,
                    &format!(
                        "{} rendered MP4(s)  •  replays/rendered/ & Movies/VoltPlus Replays",
                        self.catalog.rendered_replays.len()
                    ),
                    content_w - 116.0,
                );
                ui.text(Face::Regular, 10.4, content_x, bar_y + 7.0, my3_muted(), &info_txt);
                button(
                    ui,
                    &mut self.targets,
                    [content_x + content_w - 106.0, bar_y, 106.0, 28.0],
                    "Open Folder",
                    Action::OpenRenderedReplaysFolder,
                    false,
                );

                let list_top = bar_y + 38.0;
                let list_bottom = (h - 46.0).max(list_top + 60.0);
                let row_h = 68.0;
                let row_gap = 8.0;
                let total_h = self.catalog.rendered_replays.len() as f32 * (row_h + row_gap);
                let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let rendered = self.catalog.rendered_replays.clone();
                let del_confirm = self.catalog.rendered_replay_delete_confirm;
                let list_targets = &mut self.list_targets;
                ui.clipped([content_x - 2.0, list_top, content_w + 4.0, list_bottom - list_top], |ui| {
                    if rendered.is_empty() {
                        ui.rounded(content_x, list_top + 6.0, content_w, 84.0, 14.0, my3_card_bg());
                        ui.border(content_x, list_top + 6.0, content_w, 84.0, 14.0, my3_card_border());
                        ui.text(
                            Face::SemiBold,
                            13.0,
                            content_x + 14.0,
                            list_top + 20.0,
                            my3_text(),
                            "No rendered MP4 replays yet",
                        );
                        let hint = ui.fit_text(
                            Face::Regular,
                            10.6,
                            "Open a Raw Replay, add 2+ Pos Keyframes, and tap Export to render an MP4.",
                            content_w - 28.0,
                        );
                        ui.text(Face::Regular, 10.6, content_x + 14.0, list_top + 44.0, my3_muted(), &hint);
                        return;
                    }
                    for (i, item) in rendered.iter().enumerate() {
                        let ry = list_top + i as f32 * (row_h + row_gap) - self.catalog.scroll;
                        if ry + row_h < list_top || ry > list_bottom {
                            continue;
                        }
                        ui.rounded(content_x, ry, content_w, row_h, 13.0, my3_card_bg());
                        ui.border(content_x, ry, content_w, row_h, 13.0, my3_card_border());
                        let title = ui.fit_text(Face::SemiBold, 13.0, &item.display_name, content_w - 186.0);
                        ui.text(Face::SemiBold, 13.0, content_x + 14.0, ry + 12.0, my3_text(), &title);
                        let size_str = horizon_client::replay::format_size_bytes(item.size_bytes);
                        let date_str = chrono::DateTime::from_timestamp(item.modified_secs as i64, 0)
                            .map(|dt| {
                                dt.with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_else(|| "MP4 Video".to_owned());
                        let meta = ui.fit_text(
                            Face::Regular,
                            10.4,
                            &format!("MP4  •  {size_str}  •  {date_str}"),
                            content_w - 186.0,
                        );
                        ui.text(Face::Regular, 10.4, content_x + 14.0, ry + 36.0, my3_muted(), &meta);

                        let by = ry + (row_h - 32.0) * 0.5;
                        let play_r = [content_x + content_w - 166.0, by, 80.0, 32.0];
                        let del_r = [content_x + content_w - 80.0, by, 72.0, 32.0];
                        ui.rounded(play_r[0], play_r[1], play_r[2], play_r[3], 9.0, my3_accent());
                        ui.centered_fit(
                            Face::SemiBold,
                            11.0,
                            play_r,
                            horizon_client::ui_theme::active_theme().on_accent,
                            "Play MP4",
                        );
                        if let Some(c) = ui.clip_rect(play_r) {
                            self.targets.push((c, Action::PlayRenderedReplay(i)));
                            list_targets.push((c, Action::PlayRenderedReplay(i)));
                        }
                        let confirming = del_confirm == Some(i);
                        ui.rounded(
                            del_r[0],
                            del_r[1],
                            del_r[2],
                            del_r[3],
                            9.0,
                            if confirming {
                                horizon_client::hud::rgba(220, 38, 38, 220)
                            } else {
                                my3_surface()
                            },
                        );
                        ui.border(del_r[0], del_r[1], del_r[2], del_r[3], 9.0, my3_card_border());
                        ui.centered_fit(
                            Face::SemiBold,
                            10.6,
                            del_r,
                            my3_text(),
                            if confirming { "Confirm" } else { "Delete" },
                        );
                        if let Some(c) = ui.clip_rect(del_r) {
                            self.targets.push((c, Action::DeleteRenderedReplay(i)));
                            list_targets.push((c, Action::DeleteRenderedReplay(i)));
                        }
                    }
                });
            }
        }
    }

    fn draw_worlds_page(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
    ) {
        let header_y = 60.0 + slide_y;
        ui.text(
            Face::SemiBold,
            19.0,
            content_x,
            header_y + 2.0,
            my3_text(),
            "CurseForge Worlds",
        );
        let badge_w = if compact { 120.0 } else { 142.0 };
        let badge_r = [content_x + content_w - badge_w, header_y + 2.0, badge_w, 26.0];
        ui.rounded(badge_r[0], badge_r[1], badge_r[2], badge_r[3], 8.0, my3_surface_high());
        ui.border(badge_r[0], badge_r[1], badge_r[2], badge_r[3], 8.0, my3_card_border());
        ui.centered_fit(Face::SemiBold, 10.0, badge_r, my3_accent(), "CurseForge API");

        let row1_y = 98.0 + slide_y;
        let search_w = (content_w - 104.0).max(120.0);
        let search_r = ui.text_field(
            content_x,
            row1_y,
            search_w,
            32.0,
            "",
            &self.catalog.world_search_query,
            "Search CurseForge worlds & maps...",
            self.catalog.editing == Some("world_search_query"),
        );
        self.targets.push((search_r, Action::Field("world_search_query")));
        button(
            ui,
            &mut self.targets,
            [content_x + search_w + 8.0, row1_y, 96.0, 32.0],
            if self.catalog.world_discover_busy { "Loading..." } else { "Search" },
            Action::SearchDiscoverWorlds,
            true,
        );

        let list_top = row1_y + 42.0;
        let list_bottom = (h - 46.0).max(list_top + 60.0);
        let row_h = 72.0;
        let row_gap = 8.0;
        let total_h = self.catalog.world_discover_results.len() as f32 * (row_h + row_gap);
        let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
        self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
        self.list_view = Some([list_top, list_bottom, max_scroll]);

        let hits = self.catalog.world_discover_results.clone();
        let busy = self.catalog.world_discover_busy;
        let list_targets = &mut self.list_targets;
        ui.clipped([content_x - 2.0, list_top, content_w + 4.0, list_bottom - list_top], |ui| {
            if hits.is_empty() {
                ui.rounded(content_x, list_top + 6.0, content_w, 82.0, 14.0, my3_card_bg());
                ui.border(content_x, list_top + 6.0, content_w, 82.0, 14.0, my3_card_border());
                ui.text(Face::SemiBold, 13.0, content_x + 14.0, list_top + 20.0, my3_text(), "Browse CurseForge Worlds");
                let hint = ui.fit_text(
                    Face::Regular,
                    10.5,
                    if busy {
                        "Loading CurseForge worlds..."
                    } else {
                        "Tap Search to browse & install Minecraft worlds from CurseForge into Inventory -> World."
                    },
                    content_w - 28.0,
                );
                ui.text(Face::Regular, 10.5, content_x + 14.0, list_top + 43.0, my3_muted(), &hint);
                return;
            }
            for (i, proj) in hits.iter().enumerate() {
                let ry = list_top + i as f32 * (row_h + row_gap) - self.catalog.scroll;
                if ry + row_h < list_top || ry > list_bottom {
                    continue;
                }
                ui.rounded(content_x, ry, content_w, row_h, 13.0, my3_card_bg());
                ui.border(content_x, ry, content_w, row_h, 13.0, my3_card_border());

                let drawn_icon = if let Some((iw, ih, rgba)) = self.catalog.icons.get(&proj.id) {
                    ui.image(
                        &proj.id,
                        *iw,
                        *ih,
                        rgba,
                        content_x + 12.0,
                        ry + 12.0,
                        48.0,
                        48.0,
                    )
                } else {
                    false
                };
                if !drawn_icon {
                    ui.rounded(
                        content_x + 12.0,
                        ry + 12.0,
                        48.0,
                        48.0,
                        12.0,
                        my3_accent_soft(),
                    );
                    ui.icon(
                        Icon::Folder,
                        content_x + 24.0,
                        ry + 24.0,
                        24.0,
                        my3_accent(),
                    );
                }

                let text_x = content_x + 70.0;
                let text_max_w = (content_w - 172.0).max(80.0);
                let title = ui.fit_text(Face::SemiBold, 13.0, &proj.title, text_max_w);
                ui.text(Face::SemiBold, 13.0, text_x, ry + 11.0, my3_text(), &title);
                let meta_txt = if proj.author.is_empty() {
                    proj.summary.clone()
                } else {
                    format!("by {}  •  {} downloads  •  {}", proj.author, proj.downloads, proj.summary)
                };
                let summary = ui.fit_text(Face::Regular, 10.4, &meta_txt, text_max_w);
                ui.text(Face::Regular, 10.4, text_x, ry + 35.0, my3_muted(), &summary);

                let by = ry + (row_h - 30.0) * 0.5;
                let ins_r = [content_x + content_w - 92.0, by, 84.0, 30.0];
                ui.rounded(ins_r[0], ins_r[1], ins_r[2], ins_r[3], 9.0, my3_accent());
                ui.centered_fit(
                    Face::SemiBold,
                    10.6,
                    ins_r,
                    color::hex(0x0c0e14, 255),
                    if busy { "Wait..." } else { "Install" },
                );
                if !busy && let Some(c) = ui.clip_rect(ins_r) {
                    self.targets.push((c, Action::InstallDiscoverWorld(i)));
                    list_targets.push((c, Action::InstallDiscoverWorld(i)));
                }
            }
        });
    }

    fn draw_inventory_page(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
    ) {
        use crate::catalog::{InventorySubTab, format_play_time, game_mode_label};

        let header_y = 60.0 + slide_y;
        if let Some(info) = self.catalog.world_inspection.clone() {
            ui.text(
                Face::SemiBold,
                18.0,
                content_x,
                header_y + 2.0,
                my3_text(),
                &ui.fit_text(Face::SemiBold, 18.0, &info.name, content_w - 196.0),
            );
            let back_r = [content_x + content_w - 86.0, header_y, 86.0, 30.0];
            button(ui, &mut self.targets, back_r, "Back", Action::CloseWorldInspection, false);
            if let Some(idx) = self.catalog.worlds.iter().position(|w| w.folder == info.folder) {
                let play_r = [content_x + content_w - 184.0, header_y, 90.0, 30.0];
                button(ui, &mut self.targets, play_r, "Play World", Action::PlayWorld(idx), true);
            }

            let list_top = 98.0 + slide_y;
            let list_bottom = (h - 46.0).max(list_top + 60.0);
            let mut y_cursor = 0.0_f32;
            let card_gap = 10.0;
            let overview_h = if compact { 126.0 } else { 104.0 };
            let analytics_h = if compact { 126.0 } else { 104.0 };
            let equip_h = 108.0;
            let inv_rows = (info.inventory.len().min(12) + 1) / 2;
            let inv_h = (56.0 + inv_rows as f32 * 20.0).max(82.0);
            let ender_rows = (info.ender_chest.len().min(8) + 1) / 2;
            let ender_h = if info.ender_chest.is_empty() {
                68.0
            } else {
                54.0 + ender_rows as f32 * 20.0
            };
            let total_h = overview_h + analytics_h + equip_h + inv_h + ender_h + card_gap * 5.0;
            let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
            self.list_view = Some([list_top, list_bottom, max_scroll]);

            ui.clipped([content_x - 2.0, list_top, content_w + 4.0, list_bottom - list_top], |ui| {
                // Card 1: World & Player Overview
                let cy = list_top + y_cursor - self.catalog.scroll;
                if cy + overview_h >= list_top && cy <= list_bottom {
                    ui.rounded(content_x, cy, content_w, overview_h, 13.0, my3_card_bg());
                    ui.border(content_x, cy, content_w, overview_h, 13.0, my3_card_border());
                    ui.text(Face::SemiBold, 13.0, content_x + 14.0, cy + 11.0, my3_text(), "World & Player Overview");
                    let mode_s = game_mode_label(info.game_mode, info.hardcore);
                    let size_mb = info.size_bytes as f64 / 1_048_576.0;
                    let line1 = ui.fit_text(
                        Face::Regular,
                        10.6,
                        &format!(
                            "{mode_s}  •  {}  •  Day {}  •  Difficulty: {}  •  {size_mb:.1} MB",
                            info.dimension, info.day, info.difficulty
                        ),
                        content_w - 28.0,
                    );
                    ui.text(Face::Regular, 10.6, content_x + 14.0, cy + 34.0, my3_muted(), &line1);
                    let hearts = (info.health / 2.0).max(0.0);
                    let line2 = ui.fit_text(
                        Face::Regular,
                        10.6,
                        &format!(
                            "Health: {hearts:.1}/10 ♥  •  Hunger: {}/20  •  XP Level: {}  •  Commands: {}",
                            info.food,
                            info.xp_level,
                            if info.allow_commands { "ON" } else { "OFF" }
                        ),
                        content_w - 28.0,
                    );
                    ui.text(Face::Regular, 10.6, content_x + 14.0, cy + 54.0, my3_text(), &line2);
                    let line3 = ui.fit_text(
                        Face::Regular,
                        10.2,
                        &format!("Seed: {}  •  Folder: saves/{}", info.seed, info.folder),
                        content_w - 28.0,
                    );
                    ui.text(Face::Regular, 10.2, content_x + 14.0, cy + 74.0, my3_muted(), &line3);
                }
                y_cursor += overview_h + card_gap;

                // Card 2: Play Analysis (stats/*.json)
                let ay = list_top + y_cursor - self.catalog.scroll;
                if ay + analytics_h >= list_top && ay <= list_bottom {
                    ui.rounded(content_x, ay, content_w, analytics_h, 13.0, my3_card_bg());
                    ui.border(content_x, ay, content_w, analytics_h, 13.0, my3_card_border());
                    ui.text(Face::SemiBold, 13.0, content_x + 14.0, ay + 11.0, my3_text(), "Play Analysis & Session Stats");
                    let st = &info.stats;
                    let walked_km = st.walked_meters as f64 / 1000.0;
                    let s1 = ui.fit_text(
                        Face::Regular,
                        10.8,
                        &format!(
                            "Play Time: {}   •   Blocks Mined: {}   •   Items Crafted: {}",
                            format_play_time(st.play_time_secs),
                            st.blocks_mined,
                            st.items_crafted
                        ),
                        content_w - 28.0,
                    );
                    ui.text(Face::Regular, 10.8, content_x + 14.0, ay + 35.0, my3_text(), &s1);
                    let s2 = ui.fit_text(
                        Face::Regular,
                        10.6,
                        &format!(
                            "Mob Kills: {}   •   Deaths: {}   •   Jumps: {}   •   Walked: {walked_km:.2} km",
                            st.mob_kills, st.deaths, st.jumps
                        ),
                        content_w - 28.0,
                    );
                    ui.text(Face::Regular, 10.6, content_x + 14.0, ay + 57.0, my3_muted(), &s2);
                    let s3 = ui.fit_text(
                        Face::Regular,
                        10.2,
                        &format!(
                            "Hotbar: {}/9 used   •   Inventory: {}/36 slots ({} items)   •   Ender Chest: {}/27 stacks",
                            info.hotbar.len(), info.occupied_slots, info.total_items, info.ender_stacks
                        ),
                        content_w - 28.0,
                    );
                    ui.text(Face::Regular, 10.2, content_x + 14.0, ay + 77.0, my3_text(), &s3);
                }
                y_cursor += analytics_h + card_gap;

                // Card 3: Equipped Armor & Offhand + Hotbar
                let ey = list_top + y_cursor - self.catalog.scroll;
                if ey + equip_h >= list_top && ey <= list_bottom {
                    ui.rounded(content_x, ey, content_w, equip_h, 13.0, my3_card_bg());
                    ui.border(content_x, ey, content_w, equip_h, 13.0, my3_card_border());
                    ui.text(Face::SemiBold, 13.0, content_x + 14.0, ey + 11.0, my3_text(), "Equipped Gear & Hotbar");
                    let armor_str = format!(
                        "Head: {}  •  Chest: {}  •  Legs: {}  •  Feet: {}",
                        info.armor[3].as_deref().unwrap_or("Empty"),
                        info.armor[2].as_deref().unwrap_or("Empty"),
                        info.armor[1].as_deref().unwrap_or("Empty"),
                        info.armor[0].as_deref().unwrap_or("Empty"),
                    );
                    let armor_fit = ui.fit_text(Face::Regular, 10.4, &armor_str, content_w - 28.0);
                    ui.text(Face::Regular, 10.4, content_x + 14.0, ey + 34.0, my3_muted(), &armor_fit);
                    let off_str = format!("Offhand: {}", info.offhand.as_deref().unwrap_or("Empty"));
                    ui.text(Face::Regular, 10.4, content_x + 14.0, ey + 54.0, my3_text(), &off_str);
                    let hb_line = if info.hotbar.is_empty() {
                        "Hotbar: Empty".to_owned()
                    } else {
                        info.hotbar
                            .iter()
                            .take(6)
                            .map(|(s, name, cnt)| {
                                if *cnt > 1 {
                                    format!("[{}] {name} x{cnt}", s + 1)
                                } else {
                                    format!("[{}] {name}", s + 1)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("  •  ")
                    };
                    let hb_fit = ui.fit_text(Face::Regular, 10.2, &hb_line, content_w - 28.0);
                    ui.text(Face::Regular, 10.2, content_x + 14.0, ey + 75.0, my3_muted(), &hb_fit);
                }
                y_cursor += equip_h + card_gap;

                // Card 4: Main Inventory Summary
                let iy = list_top + y_cursor - self.catalog.scroll;
                if iy + inv_h >= list_top && iy <= list_bottom {
                    ui.rounded(content_x, iy, content_w, inv_h, 13.0, my3_card_bg());
                    ui.border(content_x, iy, content_w, inv_h, 13.0, my3_card_border());
                    ui.text(
                        Face::SemiBold,
                        13.0,
                        content_x + 14.0,
                        iy + 11.0,
                        my3_text(),
                        &format!("Inventory Breakdown ({} items across {} slots)", info.total_items, info.occupied_slots),
                    );
                    if info.inventory.is_empty() {
                        ui.text(Face::Regular, 10.5, content_x + 14.0, iy + 38.0, my3_muted(), "Inventory is empty.");
                    } else {
                        let col_w = (content_w - 36.0) * 0.5;
                        for (idx, (item_name, count)) in info.inventory.iter().take(12).enumerate() {
                            let col = idx % 2;
                            let row = idx / 2;
                            let ix = content_x + 14.0 + col as f32 * (col_w + 8.0);
                            let i_y = iy + 36.0 + row as f32 * 20.0;
                            let txt = ui.fit_text(Face::Regular, 10.4, &format!("{item_name}  x{count}"), col_w);
                            ui.text(Face::Regular, 10.4, ix, i_y, my3_text(), &txt);
                        }
                    }
                }
                y_cursor += inv_h + card_gap;

                // Card 5: Ender Chest Summary
                let cy2 = list_top + y_cursor - self.catalog.scroll;
                if cy2 + ender_h >= list_top && cy2 <= list_bottom {
                    ui.rounded(content_x, cy2, content_w, ender_h, 13.0, my3_card_bg());
                    ui.border(content_x, cy2, content_w, ender_h, 13.0, my3_card_border());
                    ui.text(
                        Face::SemiBold,
                        13.0,
                        content_x + 14.0,
                        cy2 + 11.0,
                        my3_text(),
                        &format!("Ender Chest ({}/27 slots)", info.ender_stacks),
                    );
                    if info.ender_chest.is_empty() {
                        ui.text(Face::Regular, 10.5, content_x + 14.0, cy2 + 36.0, my3_muted(), "Ender Chest is empty.");
                    } else {
                        let col_w = (content_w - 36.0) * 0.5;
                        for (idx, (item_name, count)) in info.ender_chest.iter().take(8).enumerate() {
                            let col = idx % 2;
                            let row = idx / 2;
                            let ix = content_x + 14.0 + col as f32 * (col_w + 8.0);
                            let i_y = cy2 + 36.0 + row as f32 * 20.0;
                            let txt = ui.fit_text(Face::Regular, 10.4, &format!("{item_name}  x{count}"), col_w);
                            ui.text(Face::Regular, 10.4, ix, i_y, my3_text(), &txt);
                        }
                    }
                }
            });
            return;
        }

        ui.text(
            Face::SemiBold,
            19.0,
            content_x,
            header_y + 2.0,
            my3_text(),
            "Inventory",
        );
        let seg_w = if compact { 76.0 } else { 92.0 };
        let tabs = [
            (InventorySubTab::World, "World"),
            (InventorySubTab::Server, "Server"),
            (InventorySubTab::Screenshot, "Screenshot"),
        ];
        let seg_start = content_x + content_w - seg_w * 3.0 - 6.0;
        for (i, (tab, lbl)) in tabs.into_iter().enumerate() {
            let r = [seg_start + i as f32 * (seg_w + 3.0), header_y - 1.0, seg_w, 30.0];
            ui.m3_connected_segment(
                r,
                i,
                3,
                lbl,
                self.catalog.inventory_sub_tab == tab,
                None,
            );
            self.targets.push((r, Action::SetInventorySubTab(tab)));
        }

        match self.catalog.inventory_sub_tab {
            InventorySubTab::World => {
                let bar_y = 98.0 + slide_y;
                let btn_w = ((content_w - 16.0) / 3.0).min(138.0);
                button(ui, &mut self.targets, [content_x, bar_y, btn_w, 30.0], "Refresh", Action::RefreshWorlds, false);
                button(ui, &mut self.targets, [content_x + btn_w + 8.0, bar_y, btn_w, 30.0], "Import .zip", Action::ImportWorldZip, true);
                button(ui, &mut self.targets, [content_x + (btn_w + 8.0) * 2.0, bar_y, btn_w, 30.0], "Import Folder", Action::ImportWorldFolder, false);

                let list_top = bar_y + 40.0;
                let list_bottom = (h - 46.0).max(list_top + 60.0);
                let row_h = if compact { 82.0 } else { 72.0 };
                let row_gap = 8.0;
                let total_h = self.catalog.worlds.len() as f32 * (row_h + row_gap);
                let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let worlds = self.catalog.worlds.clone();
                let del_confirm = self.catalog.world_delete_confirm;
                let list_targets = &mut self.list_targets;
                ui.clipped([content_x - 2.0, list_top, content_w + 4.0, list_bottom - list_top], |ui| {
                    if worlds.is_empty() {
                        ui.rounded(content_x, list_top + 6.0, content_w, 84.0, 14.0, my3_card_bg());
                        ui.border(content_x, list_top + 6.0, content_w, 84.0, 14.0, my3_card_border());
                        ui.text(Face::SemiBold, 13.0, content_x + 14.0, list_top + 20.0, my3_text(), "No saved worlds in saves/ yet");
                        let hint = ui.fit_text(
                            Face::Regular,
                            10.6,
                            "Create a world in-game, import a .zip/.mcworld archive above, or download one from Worlds.",
                            content_w - 28.0,
                        );
                        ui.text(Face::Regular, 10.6, content_x + 14.0, list_top + 44.0, my3_muted(), &hint);
                        return;
                    }
                    for (i, w_item) in worlds.iter().enumerate() {
                        let ry = list_top + i as f32 * (row_h + row_gap) - self.catalog.scroll;
                        if ry + row_h < list_top || ry > list_bottom {
                            continue;
                        }
                        ui.rounded(content_x, ry, content_w, row_h, 13.0, my3_card_bg());
                        ui.border(content_x, ry, content_w, row_h, 13.0, my3_card_border());
                        let btn_area = if compact { 216.0 } else { 236.0 };
                        let title = ui.fit_text(Face::SemiBold, 13.0, &w_item.name, content_w - btn_area);
                        ui.text(Face::SemiBold, 13.0, content_x + 14.0, ry + 11.0, my3_text(), &title);
                        let size_mb = w_item.size_bytes as f64 / 1_048_576.0;
                        let mode_lbl = game_mode_label(w_item.game_mode, w_item.hardcore);
                        let status_tag = if w_item.corrupted {
                            "Corrupted"
                        } else if w_item.locked {
                            "In Use"
                        } else {
                            mode_lbl
                        };
                        let meta = ui.fit_text(
                            Face::Regular,
                            10.4,
                            &format!(
                                "{status_tag}  •  Day {}  •  {}  •  {size_mb:.1} MB",
                                w_item.day,
                                format_play_time(w_item.play_time_secs)
                            ),
                            content_w - btn_area,
                        );
                        ui.text(Face::Regular, 10.4, content_x + 14.0, ry + 34.0, my3_muted(), &meta);

                        let by = ry + (row_h - 30.0) * 0.5;
                        let play_r = [content_x + content_w - 208.0, by, 58.0, 30.0];
                        let ana_r = [content_x + content_w - 144.0, by, 70.0, 30.0];
                        let del_r = [content_x + content_w - 68.0, by, 60.0, 30.0];
                        ui.rounded(play_r[0], play_r[1], play_r[2], play_r[3], 9.0, my3_accent());
                        ui.centered_fit(Face::SemiBold, 10.6, play_r, color::hex(0x0c0e14, 255), "Play");
                        if let Some(c) = ui.clip_rect(play_r) {
                            self.targets.push((c, Action::PlayWorld(i)));
                            list_targets.push((c, Action::PlayWorld(i)));
                        }
                        ui.rounded(ana_r[0], ana_r[1], ana_r[2], ana_r[3], 9.0, my3_surface_high());
                        ui.border(ana_r[0], ana_r[1], ana_r[2], ana_r[3], 9.0, my3_card_border());
                        ui.centered_fit(Face::SemiBold, 10.2, ana_r, my3_text(), "Analyze");
                        if let Some(c) = ui.clip_rect(ana_r) {
                            self.targets.push((c, Action::InspectWorld(i)));
                            list_targets.push((c, Action::InspectWorld(i)));
                        }
                        let confirming = del_confirm == Some(i);
                        ui.rounded(
                            del_r[0],
                            del_r[1],
                            del_r[2],
                            del_r[3],
                            9.0,
                            if confirming {
                                horizon_client::hud::rgba(220, 38, 38, 220)
                            } else {
                                my3_surface()
                            },
                        );
                        ui.border(del_r[0], del_r[1], del_r[2], del_r[3], 9.0, my3_card_border());
                        ui.centered_fit(
                            Face::SemiBold,
                            10.2,
                            del_r,
                            my3_text(),
                            if confirming { "Confirm" } else { "Delete" },
                        );
                        if let Some(c) = ui.clip_rect(del_r) {
                            self.targets.push((c, Action::DeleteWorld(i)));
                            list_targets.push((c, Action::DeleteWorld(i)));
                        }
                    }
                });
            }
            InventorySubTab::Server => {
                let row1_y = 98.0 + slide_y;
                let add_btn_w = 78.0;
                let fields_w = (content_w - add_btn_w - 14.0).max(160.0);
                let name_w = fields_w * 0.42;
                let ip_w = fields_w - name_w - 6.0;
                let name_r = ui.text_field(
                    content_x,
                    row1_y,
                    name_w,
                    32.0,
                    "",
                    &self.catalog.add_server_name,
                    "Server name...",
                    self.catalog.editing == Some("add_server_name"),
                );
                self.targets.push((name_r, Action::Field("add_server_name")));
                let ip_r = ui.text_field(
                    content_x + name_w + 6.0,
                    row1_y,
                    ip_w,
                    32.0,
                    "",
                    &self.catalog.add_server_ip,
                    "Address (play.example.net)...",
                    self.catalog.editing == Some("add_server_ip"),
                );
                self.targets.push((ip_r, Action::Field("add_server_ip")));
                button(
                    ui,
                    &mut self.targets,
                    [content_x + fields_w + 12.0, row1_y, add_btn_w, 32.0],
                    "+ Add",
                    Action::AddServer,
                    true,
                );

                let list_top = row1_y + 42.0;
                let list_bottom = (h - 46.0).max(list_top + 60.0);
                let row_h = 66.0;
                let row_gap = 8.0;
                let total_h = self.catalog.servers.len() as f32 * (row_h + row_gap);
                let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let servers = self.catalog.servers.clone();
                let del_confirm = self.catalog.server_delete_confirm;
                let list_targets = &mut self.list_targets;
                ui.clipped([content_x - 2.0, list_top, content_w + 4.0, list_bottom - list_top], |ui| {
                    if servers.is_empty() {
                        ui.rounded(content_x, list_top + 6.0, content_w, 76.0, 14.0, my3_card_bg());
                        ui.border(content_x, list_top + 6.0, content_w, 76.0, 14.0, my3_card_border());
                        ui.text(Face::SemiBold, 13.0, content_x + 14.0, list_top + 20.0, my3_text(), "No saved multiplayer servers");
                        ui.text(Face::Regular, 10.6, content_x + 14.0, list_top + 42.0, my3_muted(), "Add a server above — it saves directly to servers.dat.");
                        return;
                    }
                    for (i, srv) in servers.iter().enumerate() {
                        let ry = list_top + i as f32 * (row_h + row_gap) - self.catalog.scroll;
                        if ry + row_h < list_top || ry > list_bottom {
                            continue;
                        }
                        ui.rounded(content_x, ry, content_w, row_h, 13.0, my3_card_bg());
                        ui.border(content_x, ry, content_w, row_h, 13.0, my3_card_border());
                        let title = ui.fit_text(Face::SemiBold, 13.0, &srv.name, content_w - 156.0);
                        ui.text(Face::SemiBold, 13.0, content_x + 14.0, ry + 11.0, my3_text(), &title);
                        let ip_txt = ui.fit_text(Face::Regular, 10.5, &srv.ip, content_w - 156.0);
                        ui.text(Face::Regular, 10.5, content_x + 14.0, ry + 34.0, my3_muted(), &ip_txt);

                        let by = ry + (row_h - 30.0) * 0.5;
                        let join_r = [content_x + content_w - 140.0, by, 64.0, 30.0];
                        let del_r = [content_x + content_w - 70.0, by, 62.0, 30.0];
                        ui.rounded(join_r[0], join_r[1], join_r[2], join_r[3], 9.0, my3_accent());
                        ui.centered_fit(Face::SemiBold, 10.6, join_r, color::hex(0x0c0e14, 255), "Join");
                        if let Some(c) = ui.clip_rect(join_r) {
                            self.targets.push((c, Action::JoinServer(i)));
                            list_targets.push((c, Action::JoinServer(i)));
                        }
                        let confirming = del_confirm == Some(i);
                        ui.rounded(
                            del_r[0],
                            del_r[1],
                            del_r[2],
                            del_r[3],
                            9.0,
                            if confirming {
                                horizon_client::hud::rgba(220, 38, 38, 220)
                            } else {
                                my3_surface()
                            },
                        );
                        ui.border(del_r[0], del_r[1], del_r[2], del_r[3], 9.0, my3_card_border());
                        ui.centered_fit(
                            Face::SemiBold,
                            10.2,
                            del_r,
                            my3_text(),
                            if confirming { "Confirm" } else { "Delete" },
                        );
                        if let Some(c) = ui.clip_rect(del_r) {
                            self.targets.push((c, Action::DeleteServer(i)));
                            list_targets.push((c, Action::DeleteServer(i)));
                        }
                    }
                });
            }
            InventorySubTab::Screenshot => {
                let bar_y = 98.0 + slide_y;
                let btn_w = ((content_w - 8.0) / 2.0).min(156.0);
                button(
                    ui,
                    &mut self.targets,
                    [content_x, bar_y, btn_w, 30.0],
                    "Refresh",
                    Action::RefreshScreenshots,
                    false,
                );
                button(
                    ui,
                    &mut self.targets,
                    [content_x + btn_w + 8.0, bar_y, btn_w, 30.0],
                    "Open Folder",
                    Action::OpenScreenshotsFolder,
                    false,
                );

                let list_top = bar_y + 40.0;
                let list_bottom = (h - 46.0).max(list_top + 60.0);
                let row_h = 68.0;
                let row_gap = 8.0;
                let total_h = self.catalog.screenshots.len() as f32 * (row_h + row_gap);
                let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let shots = self.catalog.screenshots.clone();
                let del_confirm = self.catalog.screenshot_delete_confirm;
                let list_targets = &mut self.list_targets;
                ui.clipped([content_x - 2.0, list_top, content_w + 4.0, list_bottom - list_top], |ui| {
                    if shots.is_empty() {
                        ui.rounded(content_x, list_top + 6.0, content_w, 80.0, 14.0, my3_card_bg());
                        ui.border(content_x, list_top + 6.0, content_w, 80.0, 14.0, my3_card_border());
                        ui.text(Face::SemiBold, 13.0, content_x + 14.0, list_top + 20.0, my3_text(), "No screenshots captured yet");
                        ui.text(Face::Regular, 10.5, content_x + 14.0, list_top + 43.0, my3_muted(), "In-game screenshots from screenshots/ will appear here.");
                        return;
                    }
                    for (i, shot) in shots.iter().enumerate() {
                        let ry = list_top + i as f32 * (row_h + row_gap) - self.catalog.scroll;
                        if ry + row_h < list_top || ry > list_bottom {
                            continue;
                        }
                        ui.rounded(content_x, ry, content_w, row_h, 13.0, my3_card_bg());
                        ui.border(content_x, ry, content_w, row_h, 13.0, my3_card_border());
                        let title = ui.fit_text(Face::SemiBold, 13.0, &shot.file_name, content_w - 178.0);
                        ui.text(Face::SemiBold, 13.0, content_x + 14.0, ry + 11.0, my3_text(), &title);
                        let size_str = horizon_client::replay::format_size_bytes(shot.size_bytes);
                        let date_str = chrono::DateTime::from_timestamp(shot.modified_secs as i64, 0)
                            .map(|dt| {
                                dt.with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_else(|| "Screenshot".to_owned());
                        let meta = ui.fit_text(
                            Face::Regular,
                            10.4,
                            &format!("{size_str}  •  {date_str}"),
                            content_w - 178.0,
                        );
                        ui.text(Face::Regular, 10.4, content_x + 14.0, ry + 34.0, my3_muted(), &meta);

                        let by = ry + (row_h - 30.0) * 0.5;
                        let exp_r = [content_x + content_w - 156.0, by, 76.0, 30.0];
                        let del_r = [content_x + content_w - 74.0, by, 66.0, 30.0];
                        ui.rounded(exp_r[0], exp_r[1], exp_r[2], exp_r[3], 9.0, my3_accent());
                        ui.centered_fit(Face::SemiBold, 10.6, exp_r, color::hex(0x0c0e14, 255), "Export");
                        if let Some(c) = ui.clip_rect(exp_r) {
                            self.targets.push((c, Action::ExportScreenshot(i)));
                            list_targets.push((c, Action::ExportScreenshot(i)));
                        }
                        let confirming = del_confirm == Some(i);
                        ui.rounded(
                            del_r[0],
                            del_r[1],
                            del_r[2],
                            del_r[3],
                            9.0,
                            if confirming {
                                horizon_client::hud::rgba(220, 38, 38, 220)
                            } else {
                                my3_surface()
                            },
                        );
                        ui.border(del_r[0], del_r[1], del_r[2], del_r[3], 9.0, my3_card_border());
                        ui.centered_fit(
                            Face::SemiBold,
                            10.2,
                            del_r,
                            my3_text(),
                            if confirming { "Confirm" } else { "Delete" },
                        );
                        if let Some(c) = ui.clip_rect(del_r) {
                            self.targets.push((c, Action::DeleteScreenshot(i)));
                            list_targets.push((c, Action::DeleteScreenshot(i)));
                        }
                    }
                });
            }
        }
    }

    fn draw_installed(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
    ) {
        use crate::catalog::{InstalledFilter, InstalledSubTab};
        use horizon_client::content_files::Kind;

        let header_y = 60.0 + slide_y;
        ui.text(
            Face::SemiBold,
            19.0,
            content_x,
            header_y + 2.0,
            my3_text(),
            if self.catalog.installed_sub_tab == InstalledSubTab::Library {
                "Content Library"
            } else {
                "Profile File Manager"
            },
        );

        // Top right Material 2026 Connected Button Group (Library vs File Manager)
        let tab_w = if compact { 106.0 } else { 128.0 };
        let lib_rect = [
            content_x + content_w - tab_w * 2.0 - 3.0,
            header_y - 1.0,
            tab_w,
            32.0,
        ];
        let fm_rect = [content_x + content_w - tab_w, header_y - 1.0, tab_w, 32.0];
        ui.m3_connected_segment(
            lib_rect,
            0,
            2,
            "Library",
            self.catalog.installed_sub_tab == InstalledSubTab::Library,
            Some(Icon::Installed),
        );
        self.targets
            .push((lib_rect, Action::SetInstalledSubTab(InstalledSubTab::Library)));
        ui.m3_connected_segment(
            fm_rect,
            1,
            2,
            "File Manager",
            self.catalog.installed_sub_tab == InstalledSubTab::FileManager,
            Some(Icon::Folder),
        );
        self.targets
            .push((fm_rect, Action::SetInstalledSubTab(InstalledSubTab::FileManager)));

        if self.catalog.installed_sub_tab == InstalledSubTab::Library {
            // Row 1: Search filter input + Sort selector + Quick Import CTA
            let row1_y = 96.0 + slide_y;
            let sort_w = if compact { 96.0 } else { 114.0 };
            let imp_w = if compact { 78.0 } else { 94.0 };
            let search_w = (content_w - sort_w - imp_w - 14.0).max(100.0);
            let search_r = ui.text_field(
                content_x,
                row1_y,
                search_w,
                34.0,
                "",
                &self.catalog.installed_query,
                "Search installed mods, packs & shaders...",
                self.catalog.editing == Some("installed_search"),
            );
            self.targets
                .push((search_r, Action::Field("installed_search")));

            let sort_r = [content_x + search_w + 7.0, row1_y, sort_w, 34.0];
            button(
                ui,
                &mut self.targets,
                sort_r,
                self.catalog.installed_sort.label(),
                Action::CycleInstalledSort,
                false,
            );

            let import_kind = match self.catalog.installed_filter {
                InstalledFilter::Mods | InstalledFilter::All => Kind::Mods,
                InstalledFilter::Packs => Kind::Packs,
                InstalledFilter::Shaders => Kind::Shaders,
            };
            let imp_r = [content_x + search_w + sort_w + 14.0, row1_y, imp_w, 34.0];
            button(
                ui,
                &mut self.targets,
                imp_r,
                "+ Import",
                Action::Import(import_kind),
                true,
            );

            // Row 2: Category filter pills (All, Mods, Resource Packs, Shaders)
            let row2_y = 136.0 + slide_y;
            let pill_gap = 6.0;
            let pill_w = ((content_w - pill_gap * 3.0) / 4.0).max(64.0);
            for (i, filter) in InstalledFilter::ALL.into_iter().enumerate() {
                let count = self
                    .catalog
                    .local
                    .iter()
                    .filter(|item| filter.matches(item.kind))
                    .count();
                let lbl = if compact && filter == InstalledFilter::Packs {
                    format!("Packs ({count})")
                } else {
                    format!("{} ({count})", filter.label())
                };
                let pr = [
                    content_x + i as f32 * (pill_w + pill_gap),
                    row2_y,
                    pill_w,
                    28.0,
                ];
                let f_icon = match filter {
                    InstalledFilter::All => Some(Icon::Installed),
                    InstalledFilter::Mods => Some(Icon::Mods),
                    InstalledFilter::Packs => Some(Icon::Packs),
                    InstalledFilter::Shaders => Some(Icon::Shaders),
                };
                ui.m3_filter_chip(
                    pr,
                    &lbl,
                    self.catalog.installed_filter == filter,
                    f_icon,
                );
                self.targets.push((pr, Action::SetInstalledFilter(filter)));
            }

            // Filtered items list
            let q = self.catalog.installed_query.trim().to_lowercase();
            let filtered: Vec<(usize, crate::catalog::LocalItem)> = self
                .catalog
                .local
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, item)| {
                    self.catalog.installed_filter.matches(item.kind)
                        && (q.is_empty()
                            || item.title.to_lowercase().contains(&q)
                            || item.file.to_lowercase().contains(&q)
                            || item.subtitle.to_lowercase().contains(&q))
                })
                .collect();

            let top = 172.0 + slide_y;
            let bottom = h - 42.0;
            if filtered.is_empty() {
                self.list_view = None;
                let card_h = (bottom - top - 8.0).clamp(110.0, 176.0);
                ui.rounded(content_x, top, content_w, card_h, 18.0, my3_surface());
                ui.border(content_x, top, content_w, card_h, 18.0, my3_border());
                ui.rounded(
                    content_x + 20.0,
                    top + 18.0,
                    42.0,
                    42.0,
                    12.0,
                    my3_accent_soft(),
                );
                ui.icon(
                    Icon::Installed,
                    content_x + 30.0,
                    top + 28.0,
                    22.0,
                    my3_accent(),
                );
                ui.text(
                    Face::SemiBold,
                    16.0,
                    content_x + 74.0,
                    top + 18.0,
                    my3_text(),
                    if self.catalog.local.is_empty() {
                        "No local add-ons installed yet"
                    } else {
                        "No installed add-ons match your filter"
                    },
                );
                ui.hint_line(
                    content_x + 74.0,
                    top + 42.0,
                    (content_w - 92.0).max(80.0),
                    "Import .hmod/.jar mods, .zip resource packs, or shader packs — or browse files in File Manager.",
                    my3_muted(),
                );
                let cta_y = top + card_h - 46.0;
                let cta_w = ((content_w - 56.0) / 3.0).clamp(88.0, 168.0);
                button(
                    ui,
                    &mut self.targets,
                    [content_x + 20.0, cta_y, cta_w, 34.0],
                    "Import Mod",
                    Action::Import(Kind::Mods),
                    true,
                );
                button(
                    ui,
                    &mut self.targets,
                    [content_x + 28.0 + cta_w, cta_y, cta_w, 34.0],
                    "Import Pack",
                    Action::Import(Kind::Packs),
                    false,
                );
                button(
                    ui,
                    &mut self.targets,
                    [content_x + 36.0 + cta_w * 2.0, cta_y, cta_w, 34.0],
                    "File Manager",
                    Action::SetInstalledSubTab(InstalledSubTab::FileManager),
                    false,
                );
                return;
            }

            let row_h = 68.0;
            let total_h = filtered.len() as f32 * row_h;
            let max_scroll = (total_h - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
            self.list_view = Some([top, bottom, max_scroll]);

            ui.set_clip(Some([
                content_x - 2.0,
                top,
                content_w + 4.0,
                (bottom - top).max(1.0),
            ]));
            for (row_idx, (orig_idx, item)) in filtered.into_iter().enumerate() {
                let y = top + row_idx as f32 * row_h - self.catalog.scroll;
                let card_h = 62.0;
                let toggle_r = [content_x + content_w - 196.0, y + 12.0, 56.0, 38.0];
                let export_r = [content_x + content_w - 132.0, y + 14.0, 58.0, 34.0];
                let delete_r = [content_x + content_w - 68.0, y + 14.0, 58.0, 34.0];
                self.list_targets
                    .push((toggle_r, Action::ToggleLocal(orig_idx)));
                self.list_targets
                    .push((export_r, Action::ExportLocal(orig_idx)));
                self.list_targets
                    .push((delete_r, Action::DeleteLocal(orig_idx)));

                if y + card_h < top || y > bottom {
                    continue;
                }
                ui.rounded(content_x, y, content_w, card_h, 14.0, my3_card_bg());
                ui.border(
                    content_x,
                    y,
                    content_w,
                    card_h,
                    14.0,
                    if item.enabled {
                        my3_accent()
                    } else {
                        my3_card_border()
                    },
                );

                // Left project/mod icon or centered vector icon inside 44x44dp box
                let kind_icon = match item.kind {
                    Kind::Mods => Icon::Mods,
                    Kind::Packs => Icon::Packs,
                    Kind::Shaders => Icon::Shaders,
                };
                let raw_stem = item
                    .file
                    .strip_prefix("horizon-native-mods/")
                    .or_else(|| item.file.strip_prefix("horizon-mods/"))
                    .or_else(|| item.file.strip_prefix("builtin:"))
                    .unwrap_or(&item.file);
                let raw_stem = raw_stem
                    .strip_suffix(".hnmod")
                    .or_else(|| raw_stem.strip_suffix(".hmod"))
                    .or_else(|| raw_stem.strip_suffix(".jar"))
                    .or_else(|| raw_stem.strip_suffix(".hshader"))
                    .or_else(|| raw_stem.strip_suffix(".zip"))
                    .unwrap_or(raw_stem);
                let canon_icon_id = horizon_client::jar_transpiler::sanitize_mod_id(raw_stem);
                let icon_entry = self
                    .catalog
                    .icons
                    .get(&item.file)
                    .map(|v| (item.file.as_str(), v))
                    .or_else(|| {
                        self.catalog
                            .icons
                            .get(raw_stem)
                            .map(|v| (raw_stem, v))
                    })
                    .or_else(|| {
                        self.catalog
                            .icons
                            .get(&canon_icon_id)
                            .map(|v| (canon_icon_id.as_str(), v))
                    });
                let drawn_img = if let Some((ik, (iw, ih, rgba))) = icon_entry {
                    ui.image(ik, *iw, *ih, rgba, content_x + 10.0, y + 9.0, 44.0, 44.0)
                } else {
                    false
                };
                if !drawn_img {
                    ui.rounded(
                        content_x + 10.0,
                        y + 9.0,
                        44.0,
                        44.0,
                        12.0,
                        if item.enabled {
                            my3_accent_soft()
                        } else {
                            my3_surface_high()
                        },
                    );
                    ui.icon(
                        kind_icon,
                        content_x + 21.0,
                        y + 20.0,
                        22.0,
                        if item.enabled {
                            my3_accent()
                        } else {
                            my3_muted()
                        },
                    );
                }

                // Status badge pill ("ACTIVE" vs "OFF")
                let status_str = if item.enabled { "ACTIVE" } else { "OFF" };
                let status_w = if item.enabled { 48.0 } else { 34.0 };
                let text_x = content_x + 64.0;
                let max_title_w = (content_w - 280.0 - status_w).max(60.0);
                let fitted_title = ui.fit_text(Face::SemiBold, 14.5, &item.title, max_title_w);
                let title_w = ui.text(
                    Face::SemiBold,
                    14.5,
                    text_x,
                    y + 9.0,
                    my3_text(),
                    &fitted_title,
                );
                let badge_x = text_x + title_w + 8.0;
                ui.rounded(
                    badge_x,
                    y + 11.0,
                    status_w,
                    18.0,
                    9.0,
                    if item.enabled {
                        my3_accent_soft()
                    } else {
                        my3_surface_high()
                    },
                );
                ui.centered_fit(
                    Face::SemiBold,
                    8.5,
                    [badge_x, y + 11.0, status_w, 18.0],
                    if item.enabled {
                        my3_accent()
                    } else {
                        my3_muted()
                    },
                    status_str,
                );

                ui.text_clipped(
                    Face::Regular,
                    11.5,
                    text_x,
                    y + 34.0,
                    (content_w - 270.0).max(70.0),
                    my3_muted(),
                    &item.subtitle,
                );

                // Enable/Disable M3 switch + Export & Delete buttons
                let switch_t = self.anim.step_switch(&format!("local_{}", item.file), item.enabled, 0.016);
                ui.m3_switch_animated(toggle_r[0] + 2.0, y + 17.0, switch_t);
                if let Some(c) = ui.clip_rect(toggle_r) {
                    self.targets.push((c, Action::ToggleLocal(orig_idx)));
                }
                button(
                    ui,
                    &mut self.targets,
                    export_r,
                    "Export",
                    Action::ExportLocal(orig_idx),
                    false,
                );
                button(
                    ui,
                    &mut self.targets,
                    delete_r,
                    "Delete",
                    Action::DeleteLocal(orig_idx),
                    false,
                );
            }
            ui.set_clip(None);
            return;
        }

        // ---- Sub-View 2: Integrated Mini File Manager (`InstalledSubTab::FileManager`) ----
        // Row 1: Back, Forward, Up + Clickable Breadcrumb Trail + Import & New Folder CTAs
        let nav_y = 94.0 + slide_y;
        let nav_btn_w = 32.0;
        let has_back = !self.catalog.fm_back_stack.is_empty();
        let has_fwd = !self.catalog.fm_forward_stack.is_empty();
        let has_up = !self.catalog.fm_current_dir.is_empty();
        button(
            ui,
            &mut self.targets,
            [content_x, nav_y, nav_btn_w, 30.0],
            "←",
            Action::FmBack,
            has_back,
        );
        button(
            ui,
            &mut self.targets,
            [content_x + 36.0, nav_y, nav_btn_w, 30.0],
            "→",
            Action::FmForward,
            has_fwd,
        );
        button(
            ui,
            &mut self.targets,
            [content_x + 72.0, nav_y, nav_btn_w, 30.0],
            "↑",
            Action::FmUp,
            has_up,
        );

        let right_cta_w = if compact { 256.0 } else { 296.0 };
        let imp_btn_w = (right_cta_w - 8.0) * 0.26;
        let imp_fld_w = (right_cta_w - 8.0) * 0.46;
        let new_folder_w = right_cta_w - 8.0 - imp_btn_w - imp_fld_w;
        let btn1_x = content_x + content_w - right_cta_w;
        let btn2_x = btn1_x + imp_btn_w + 4.0;
        let btn3_x = btn2_x + imp_fld_w + 4.0;
        button(
            ui,
            &mut self.targets,
            [btn1_x, nav_y, imp_btn_w, 30.0],
            "Import",
            Action::FmImportHere,
            false,
        );
        button(
            ui,
            &mut self.targets,
            [btn2_x, nav_y, imp_fld_w, 30.0],
            "Import Folder",
            Action::FmImportFolderHere,
            true,
        );
        button(
            ui,
            &mut self.targets,
            [btn3_x, nav_y, new_folder_w, 30.0],
            "+ Folder",
            Action::FmOpenNewFolder,
            false,
        );

        // Clickable Breadcrumb Bar between Up button and Right CTAs
        let bc_x = content_x + 110.0;
        let bc_w = (content_x + content_w - right_cta_w - 8.0 - bc_x).max(60.0);
        ui.rounded(bc_x, nav_y, bc_w, 30.0, 10.0, my3_surface());
        ui.border(bc_x, nav_y, bc_w, 30.0, 10.0, my3_border());
        let mut cur_bx = bc_x + 6.0;
        let root_lbl = format!("P{}", self.catalog.config.profile + 1);
        let root_bw = (ui.measure(Face::SemiBold, 11.5, &root_lbl) + 12.0).min(bc_w - 10.0);
        let root_br = [cur_bx, nav_y + 4.0, root_bw, 22.0];
        ui.rounded(
            root_br[0],
            root_br[1],
            root_br[2],
            root_br[3],
            6.0,
            if self.catalog.fm_current_dir.is_empty() {
                my3_accent_soft()
            } else {
                my3_surface_high()
            },
        );
        ui.centered_fit(Face::SemiBold, 11.0, root_br, my3_accent(), &root_lbl);
        self.targets.push((root_br, Action::FmBreadcrumb(0)));
        cur_bx += root_bw + 4.0;

        if !self.catalog.fm_current_dir.is_empty() {
            let parts: Vec<&str> = self
                .catalog
                .fm_current_dir
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();
            for (idx, seg) in parts.iter().enumerate() {
                if cur_bx + 28.0 > bc_x + bc_w {
                    break;
                }
                ui.text(Face::Regular, 11.0, cur_bx, nav_y + 7.0, my3_muted(), "/");
                cur_bx += 8.0;
                let avail = (bc_x + bc_w - 6.0 - cur_bx).max(24.0);
                let fit_seg = ui.fit_text(Face::Medium, 11.0, seg, (avail - 10.0).max(16.0));
                let seg_w = (ui.measure(Face::Medium, 11.0, &fit_seg) + 10.0).min(avail);
                let seg_r = [cur_bx, nav_y + 4.0, seg_w, 22.0];
                let is_last = idx + 1 == parts.len();
                ui.rounded(
                    seg_r[0],
                    seg_r[1],
                    seg_r[2],
                    seg_r[3],
                    6.0,
                    if is_last {
                        my3_accent_soft()
                    } else {
                        my3_surface_high()
                    },
                );
                ui.centered_fit(
                    Face::Medium,
                    11.0,
                    seg_r,
                    if is_last { my3_text() } else { my3_muted() },
                    &fit_seg,
                );
                self.targets.push((seg_r, Action::FmBreadcrumb(idx + 1)));
                cur_bx += seg_w + 4.0;
            }
        }

        // Row 2: Quick Root Folder Pills (Root, Worlds, Mods, Packs, Shaders, Controls, Shots, Reports)
        let root_y = 128.0 + slide_y;
        let root_dirs = crate::document_store::FM_ROOT_DIRS;
        let r_gap = 4.0;
        let r_w = ((content_w - r_gap * (root_dirs.len() as f32 - 1.0)) / root_dirs.len() as f32)
            .max(38.0);
        for (i, (rel, label)) in root_dirs.iter().enumerate() {
            let rx = content_x + i as f32 * (r_w + r_gap);
            let active = self.catalog.fm_current_dir == *rel
                || (!rel.is_empty()
                    && self
                        .catalog
                        .fm_current_dir
                        .strip_prefix(*rel)
                        .is_some_and(|rest| rest.starts_with('/')));
            button(
                ui,
                &mut self.targets,
                [rx, root_y, r_w, 26.0],
                label,
                Action::FmNavigateRoot(rel),
                active,
            );
        }

        // Row 3: Multi-Select & Clipboard Action Toolbar
        let tool_y = 158.0 + slide_y;
        let sel_count = self.catalog.fm_selected.len();
        let clip_count = self
            .catalog
            .fm_clipboard
            .as_ref()
            .map_or(0, |(v, _)| v.len());
        let paste_label = if clip_count > 0 {
            format!("Paste ({clip_count})")
        } else {
            "Paste".into()
        };
        let del_label = if sel_count > 0 {
            format!("Delete ({sel_count})")
        } else {
            "Delete".into()
        };
        let has_dir_sel = self.catalog.fm_selected.iter().any(|rel| {
            self.catalog
                .fm_entries
                .iter()
                .any(|e| e.is_dir && &e.rel_path == rel)
        });
        let export_lbl = if has_dir_sel {
            "Export Folder".to_string()
        } else {
            "Export".to_string()
        };
        let tools: [(&str, Action, bool); 8] = [
            ("All", Action::FmSelectAll, false),
            ("Clear", Action::FmClearSelection, false),
            ("Copy", Action::FmCopy, sel_count > 0),
            ("Cut", Action::FmCut, sel_count > 0),
            (&paste_label, Action::FmPaste, clip_count > 0),
            ("Rename", Action::FmOpenRenameSelected, sel_count == 1),
            (&export_lbl, Action::FmExportSelected, sel_count > 0),
            (&del_label, Action::FmRequestDeleteSelected, sel_count > 0),
        ];
        let t_gap = 4.0;
        let t_w = ((content_w - t_gap * (tools.len() as f32 - 1.0)) / tools.len() as f32).max(38.0);
        for (i, (lbl, act, hi)) in tools.into_iter().enumerate() {
            let tx = content_x + i as f32 * (t_w + t_gap);
            button(ui, &mut self.targets, [tx, tool_y, t_w, 26.0], lbl, act, hi);
        }

        // Scrollable File & Directory Rows
        let top = 190.0 + slide_y;
        let bottom = h - 42.0;
        if self.catalog.fm_entries.is_empty() {
            self.list_view = None;
            let empty_h = (bottom - top - 8.0).clamp(96.0, 152.0);
            ui.rounded(content_x, top, content_w, empty_h, 16.0, my3_surface());
            ui.border(content_x, top, content_w, empty_h, 16.0, my3_border());
            ui.icon(
                Icon::Folder,
                content_x + 20.0,
                top + 20.0,
                24.0,
                my3_accent(),
            );
            ui.text(
                Face::SemiBold,
                15.0,
                content_x + 54.0,
                top + 18.0,
                my3_text(),
                "This folder is empty",
            );
            ui.hint_line(
                content_x + 54.0,
                top + 40.0,
                (content_w - 72.0).max(80.0),
                "Use Import to copy files from Android storage, + Folder to create a directory, or Paste from clipboard.",
                my3_muted(),
            );
            return;
        }

        let row_h = 52.0;
        let total_h = self.catalog.fm_entries.len() as f32 * row_h;
        let max_scroll = (total_h - (bottom - top)).max(0.0);
        self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
        self.list_view = Some([top, bottom, max_scroll]);

        ui.set_clip(Some([
            content_x - 2.0,
            top,
            content_w + 4.0,
            (bottom - top).max(1.0),
        ]));
        let entries_snapshot = self.catalog.fm_entries.clone();
        for (idx, entry) in entries_snapshot.iter().enumerate() {
            let y = top + idx as f32 * row_h - self.catalog.scroll;
            let card_h = 46.0;
            let selected = self.catalog.fm_selected.contains(&entry.rel_path);
            let check_r = [content_x + 6.0, y + 7.0, 32.0, 32.0];
            let actions_w = if compact { 256.0 } else { 304.0 };
            let open_r = [
                content_x + 40.0,
                y,
                (content_w - 44.0 - actions_w).max(60.0),
                card_h,
            ];
            let btn_w = (actions_w - 12.0) / 4.0;
            let exp_r = [content_x + content_w - actions_w, y + 8.0, btn_w, 30.0];
            let info_r = [exp_r[0] + btn_w + 4.0, y + 8.0, btn_w, 30.0];
            let ren_r = [info_r[0] + btn_w + 4.0, y + 8.0, btn_w, 30.0];
            let del_r = [ren_r[0] + btn_w + 4.0, y + 8.0, btn_w, 30.0];

            self.list_targets.push((open_r, Action::FmOpenEntry(idx)));
            self.list_targets
                .push((check_r, Action::FmToggleSelect(idx)));
            self.list_targets.push((exp_r, Action::FmExportSingle(idx)));
            self.list_targets.push((info_r, Action::FmOpenDetails(idx)));
            self.list_targets.push((ren_r, Action::FmOpenRename(idx)));
            self.list_targets
                .push((del_r, Action::FmRequestDeleteSingle(idx)));

            if y + card_h < top || y > bottom {
                continue;
            }
            ui.rounded(
                content_x,
                y,
                content_w,
                card_h,
                12.0,
                if selected {
                    my3_nav_active()
                } else {
                    my3_card_bg()
                },
            );
            ui.border(
                content_x,
                y,
                content_w,
                card_h,
                12.0,
                if selected {
                    my3_accent()
                } else {
                    my3_card_border()
                },
            );

            ui.m3_checkbox(content_x + 11.0, y + 13.0, 20.0, selected, "");
            if let Some(c) = ui.clip_rect(check_r) {
                self.targets.push((c, Action::FmToggleSelect(idx)));
            }

            let entry_icon = if entry.is_dir {
                Icon::Folder
            } else if entry.name.ends_with(".hmod")
                || entry.name.ends_with(".hnmod")
                || entry.name.ends_with(".jar")
                || entry.name.ends_with(".mod.json")
            {
                Icon::Mods
            } else if entry.name.ends_with(".hshader") {
                Icon::Shaders
            } else if entry.name.ends_with(".zip") {
                Icon::Packs
            } else {
                Icon::FileText
            };
            ui.rounded(
                content_x + 38.0,
                y + 7.0,
                32.0,
                32.0,
                9.0,
                if entry.is_dir {
                    my3_accent_soft()
                } else {
                    my3_surface_high()
                },
            );
            ui.icon(
                entry_icon,
                content_x + 46.0,
                y + 15.0,
                16.0,
                if entry.is_dir {
                    my3_accent()
                } else {
                    my3_text()
                },
            );

            let tx = content_x + 78.0;
            let max_tw = (content_w - 86.0 - actions_w).max(50.0);
            ui.text_clipped(
                Face::SemiBold,
                13.5,
                tx,
                y + 5.0,
                max_tw,
                my3_text(),
                &entry.name,
            );
            let meta_line = if entry.is_dir {
                format!("Folder  •  {} item(s)  •  Tap to open", entry.size_bytes)
            } else {
                format!(
                    "{}  •  {}  •  {}",
                    entry.kind_label,
                    crate::catalog::format_file_size(entry.size_bytes),
                    crate::document_store::format_timestamp_utc(entry.modified_secs)
                )
            };
            ui.text_clipped(
                Face::Regular,
                10.5,
                tx,
                y + 24.0,
                max_tw,
                my3_muted(),
                &meta_line,
            );
            if let Some(c) = ui.clip_rect(open_r) {
                self.targets.push((c, Action::FmOpenEntry(idx)));
            }

            button(
                ui,
                &mut self.targets,
                exp_r,
                "Export",
                Action::FmExportSingle(idx),
                false,
            );
            button(
                ui,
                &mut self.targets,
                info_r,
                "Details",
                Action::FmOpenDetails(idx),
                false,
            );
            button(
                ui,
                &mut self.targets,
                ren_r,
                "Rename",
                Action::FmOpenRename(idx),
                false,
            );
            button(
                ui,
                &mut self.targets,
                del_r,
                "Delete",
                Action::FmRequestDeleteSingle(idx),
                false,
            );
        }
        ui.set_clip(None);
    }

    fn draw_music_fullscreen(
        &mut self,
        ui: &mut Ui,
        w: f32,
        full_h: f32,
        ime_top_dp: Option<f32>,
    ) {
        use horizon_client::internet_music::{
            self, format_bytes, format_duration, format_ms, MusicSort,
        };

        if !self.catalog.music_fetched_initial
            && !self.catalog.music_searching
            && self.catalog.music_results.is_empty()
        {
            self.catalog.search_music(&self.home);
        }

        let snap = internet_music::snapshot();
        let playlists = internet_music::playlists();
        let downloads = internet_music::active_downloads();
        let active_dl_count = downloads.iter().filter(|d| !d.done).count();
        let vanilla_off = internet_music::is_vanilla_music_disabled();

        let pad = 16.0;
        let inner_w = (w - pad * 2.0).max(240.0);

        // Fullscreen Material You 3 panel backdrop
        ui.rounded(8.0, 8.0, w - 16.0, full_h - 16.0, 20.0, my3_surface());
        ui.stroke_rounded(8.0, 8.0, w - 16.0, full_h - 16.0, 20.0, 1.0, my3_border());

        // Top Header Bar
        button(
            ui,
            &mut self.targets,
            [pad + 4.0, 16.0, 76.0, 32.0],
            "Back",
            Action::CloseMusic,
            false,
        );
        ui.rounded(pad + 88.0, 18.0, 28.0, 28.0, 8.0, my3_accent_soft());
        ui.icon(Icon::Music, pad + 94.0, 24.0, 16.0, my3_accent());
        ui.text(Face::SemiBold, 17.5, pad + 124.0, 16.0, my3_text(), "Musics");

        let status_line = if self.catalog.music_status.is_empty() {
            "Stream & download songs"
        } else {
            &self.catalog.music_status
        };
        let status_fit = ui.fit_text(Face::Regular, 10.5, status_line, (inner_w - 540.0).max(60.0));
        if inner_w >= 640.0 {
            ui.text(
                Face::Regular,
                10.5,
                pad + 124.0,
                36.0,
                my3_muted(),
                &status_fit,
            );
        }

        // Top-right Mode Tabs (Discover | Playlists), Downloads, & Vanilla Music Toggle
        let vanilla_lbl = if vanilla_off {
            "MC Music: Off"
        } else {
            "MC Music: On"
        };
        let dl_lbl = if active_dl_count > 0 {
            format!("Downloads ({active_dl_count})")
        } else if !downloads.is_empty() {
            format!("Downloads ({})", downloads.len())
        } else {
            "Downloads".to_owned()
        };
        let vanilla_w = 104.0;
        let dl_w = 106.0;
        let tab_w = 88.0;
        let right_block_w = vanilla_w + dl_w + tab_w * 2.0 + 18.0;
        let tabs_x = (w - pad - right_block_w - 4.0).max(pad + 195.0);
        button(
            ui,
            &mut self.targets,
            [tabs_x, 16.0, tab_w, 32.0],
            "Discover",
            Action::MusicTab(false),
            !self.catalog.music_tab_playlists,
        );
        button(
            ui,
            &mut self.targets,
            [tabs_x + tab_w + 6.0, 16.0, tab_w, 32.0],
            "Playlists",
            Action::MusicTab(true),
            self.catalog.music_tab_playlists,
        );
        button(
            ui,
            &mut self.targets,
            [tabs_x + tab_w * 2.0 + 12.0, 16.0, dl_w, 32.0],
            &dl_lbl,
            Action::MusicOpenDownloads,
            active_dl_count > 0,
        );
        button(
            ui,
            &mut self.targets,
            [tabs_x + tab_w * 2.0 + dl_w + 18.0, 16.0, vanilla_w, 32.0],
            vanilla_lbl,
            Action::MusicToggleDisableVanilla,
            vanilla_off,
        );

        let player_bar_h = 64.0;
        let player_y = full_h - player_bar_h - 12.0;

        if !self.catalog.music_tab_playlists {
            // Search Row + Sort By Button
            let search_y = 58.0;
            let sort_btn_w = 128.0;
            let search_btn_w = 92.0;
            let field_w = (inner_w - search_btn_w - sort_btn_w - 24.0).max(120.0);
            let field_rect = ui.text_field(
                pad + 4.0,
                search_y,
                field_w,
                36.0,
                "",
                &self.catalog.music_query,
                "Search songs, artists, or paste https:// audio URL...",
                self.catalog.editing == Some("music_query"),
            );
            self.targets.push((field_rect, Action::Field("music_query")));
            button(
                ui,
                &mut self.targets,
                [pad + 4.0 + field_w + 8.0, search_y, search_btn_w, 36.0],
                if self.catalog.music_searching {
                    "Searching..."
                } else {
                    "Search"
                },
                Action::MusicSearch,
                true,
            );
            let sort_lbl = format!("Sort: {}", self.catalog.music_sort.label());
            button(
                ui,
                &mut self.targets,
                [pad + 4.0 + field_w + search_btn_w + 16.0, search_y, sort_btn_w, 36.0],
                &sort_lbl,
                Action::MusicOpenSort,
                false,
            );

            // Quick Genre / Trending Chips (independent from search query text)
            let chip_y = 102.0;
            let chips = ["Trending", "Lo-Fi", "Chill", "Synthwave", "Electronic", "Ambient"];
            let chip_gap = 6.0;
            let chip_w = ((inner_w - 8.0 - chip_gap * (chips.len() as f32 - 1.0)) / chips.len() as f32)
                .clamp(68.0, 112.0);
            for (ci, chip) in chips.iter().enumerate() {
                let cx = pad + 4.0 + ci as f32 * (chip_w + chip_gap);
                if cx + chip_w > w - pad {
                    break;
                }
                let active = if *chip == "Trending" {
                    self.catalog.music_genre.is_none()
                } else {
                    self.catalog
                        .music_genre
                        .as_deref()
                        .is_some_and(|g| g.eq_ignore_ascii_case(chip))
                };
                button(
                    ui,
                    &mut self.targets,
                    [cx, chip_y, chip_w, 28.0],
                    chip,
                    Action::MusicSelectGenre(chip),
                    active,
                );
            }

            // Song Results List + Bottom-Middle Pagination Bar
            let pager_h = 34.0;
            let pager_y = player_y - pager_h - 6.0;
            let top = 138.0;
            let bottom = (pager_y - 6.0).max(top + 60.0);
            let row_h = 68.0;
            let total_h = self.catalog.music_results.len() as f32 * row_h;
            let max_scroll = (total_h - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
            self.list_view = Some([top, bottom, max_scroll]);

            ui.set_clip(Some([pad, top, inner_w, (bottom - top).max(1.0)]));
            if self.catalog.music_results.is_empty() {
                let msg = if self.catalog.music_searching {
                    "Searching internet music..."
                } else {
                    "Search for a song above or tap a genre chip to browse music."
                };
                ui.text(Face::Regular, 13.0, pad + 12.0, top + 20.0, my3_muted(), msg);
            }
            for (idx, track) in self.catalog.music_results.iter().enumerate() {
                let y = top + idx as f32 * row_h - self.catalog.scroll;
                if y + 60.0 < top || y > bottom {
                    continue;
                }
                let row_x = pad + 4.0;
                let row_w = (inner_w - 16.0).max(200.0);
                let is_playing_this = snap.has_track() && snap.current_url == track.stream_url;
                let is_downloaded = internet_music::is_track_downloaded(track);
                let is_downloading = downloads.iter().any(|d| d.id == track.id && !d.done);
                ui.rounded(
                    row_x,
                    y,
                    row_w,
                    60.0,
                    14.0,
                    if is_playing_this {
                        my3_accent_soft()
                    } else {
                        my3_card_bg()
                    },
                );
                ui.border(
                    row_x,
                    y,
                    row_w,
                    60.0,
                    14.0,
                    if is_playing_this {
                        my3_accent()
                    } else {
                        my3_card_border()
                    },
                );

                // Song cover icon
                let drawn = if let Some((iw, ih, rgba)) = self.catalog.music_icons.get(&track.id) {
                    ui.image(&track.id, *iw, *ih, rgba, row_x + 8.0, y + 8.0, 44.0, 44.0)
                } else {
                    false
                };
                if !drawn {
                    ui.rounded(row_x + 8.0, y + 8.0, 44.0, 44.0, 10.0, my3_accent_soft());
                    ui.icon(Icon::Music, row_x + 19.0, y + 19.0, 22.0, my3_accent());
                }

                // "+" Save-to-Playlist button directly beside the song logo/icon
                let plus_rect = [row_x + 58.0, y + 13.0, 34.0, 34.0];
                ui.rounded(
                    plus_rect[0],
                    plus_rect[1],
                    plus_rect[2],
                    plus_rect[3],
                    10.0,
                    my3_surface(),
                );
                ui.border(
                    plus_rect[0],
                    plus_rect[1],
                    plus_rect[2],
                    plus_rect[3],
                    10.0,
                    my3_border(),
                );
                ui.icon(
                    Icon::Plus,
                    plus_rect[0] + 9.0,
                    plus_rect[1] + 9.0,
                    16.0,
                    my3_text(),
                );
                self.list_targets
                    .push((plus_rect, Action::MusicOpenSaveResult(idx)));
                if let Some(c) = ui.clip_rect(plus_rect) {
                    self.targets.push((c, Action::MusicOpenSaveResult(idx)));
                }

                // Download song button right beside "+"
                let dl_rect = [row_x + 96.0, y + 13.0, 34.0, 34.0];
                ui.rounded(
                    dl_rect[0],
                    dl_rect[1],
                    dl_rect[2],
                    dl_rect[3],
                    10.0,
                    if is_downloaded || is_downloading {
                        my3_accent_soft()
                    } else {
                        my3_surface()
                    },
                );
                ui.border(
                    dl_rect[0],
                    dl_rect[1],
                    dl_rect[2],
                    dl_rect[3],
                    10.0,
                    if is_downloaded || is_downloading {
                        my3_accent()
                    } else {
                        my3_border()
                    },
                );
                ui.icon(
                    if is_downloaded {
                        Icon::Check
                    } else {
                        Icon::Download
                    },
                    dl_rect[0] + 9.0,
                    dl_rect[1] + 9.0,
                    16.0,
                    if is_downloaded || is_downloading {
                        my3_accent()
                    } else {
                        my3_text()
                    },
                );
                self.list_targets
                    .push((dl_rect, Action::MusicDownloadResult(idx)));
                if let Some(c) = ui.clip_rect(dl_rect) {
                    self.targets.push((c, Action::MusicDownloadResult(idx)));
                }

                // Play button on right + duration badge
                let play_rect = [row_x + row_w - 94.0, y + 12.0, 84.0, 36.0];
                let dur_str = if track.duration_secs > 0 {
                    format_duration(track.duration_secs)
                } else {
                    "Stream".to_owned()
                };
                let dur_w = 52.0;
                let dur_x = play_rect[0] - dur_w - 8.0;
                ui.rounded(dur_x, y + 17.0, dur_w, 26.0, 8.0, my3_surface());
                ui.centered_fit(
                    Face::Medium,
                    11.0,
                    [dur_x, y + 17.0, dur_w, 26.0],
                    my3_muted(),
                    &dur_str,
                );

                let text_x = row_x + 138.0;
                let text_max = (dur_x - text_x - 8.0).max(80.0);
                let title_fit = ui.fit_text(Face::SemiBold, 14.5, &track.title, text_max);
                let sub = if is_downloading {
                    format!("{} • Downloading...", track.artist)
                } else if is_downloaded {
                    format!("{} • Downloaded", track.artist)
                } else {
                    format!("{} • {}", track.artist, track.source)
                };
                let sub_fit = ui.fit_text(Face::Regular, 12.0, &sub, text_max);
                ui.text(Face::SemiBold, 14.5, text_x, y + 10.0, my3_text(), &title_fit);
                ui.text(Face::Regular, 12.0, text_x, y + 31.0, my3_muted(), &sub_fit);

                let card_tap = [text_x, y, (dur_x + dur_w - text_x).max(40.0), 60.0];
                self.list_targets
                    .push((card_tap, Action::MusicPlayResult(idx)));
                if let Some(c) = ui.clip_rect(card_tap) {
                    self.targets.push((c, Action::MusicPlayResult(idx)));
                }
                self.list_targets
                    .push((play_rect, Action::MusicPlayResult(idx)));
                if let Some(c) = ui.clip_rect(play_rect) {
                    let lbl = if is_playing_this && snap.playing {
                        "Playing"
                    } else {
                        "Play"
                    };
                    button(
                        ui,
                        &mut self.targets,
                        play_rect,
                        lbl,
                        Action::MusicPlayResult(idx),
                        is_playing_this,
                    );
                    let _ = c;
                }
            }
            ui.set_clip(None);

            // Bottom-middle Search Pagination (Previous | Page N | Next)
            let prev_w = 104.0;
            let page_w = 90.0;
            let next_w = 104.0;
            let pager_total = prev_w + page_w + next_w + 16.0;
            let pager_x = (w - pager_total) * 0.5;
            if self.catalog.music_page > 0 {
                button(
                    ui,
                    &mut self.targets,
                    [pager_x, pager_y, prev_w, 30.0],
                    "Previous",
                    Action::MusicPrevPage,
                    false,
                );
            } else {
                ui.rounded(pager_x, pager_y, prev_w, 30.0, 10.0, my3_card_bg());
                ui.centered_fit(
                    Face::Medium,
                    11.5,
                    [pager_x, pager_y, prev_w, 30.0],
                    my3_muted(),
                    "Previous",
                );
            }
            ui.rounded(pager_x + prev_w + 8.0, pager_y, page_w, 30.0, 10.0, my3_surface());
            ui.border(pager_x + prev_w + 8.0, pager_y, page_w, 30.0, 10.0, my3_border());
            ui.centered_fit(
                Face::SemiBold,
                11.5,
                [pager_x + prev_w + 8.0, pager_y, page_w, 30.0],
                my3_text(),
                &format!("Page {}", self.catalog.music_page + 1),
            );
            if self.catalog.music_has_next {
                button(
                    ui,
                    &mut self.targets,
                    [pager_x + prev_w + page_w + 16.0, pager_y, next_w, 30.0],
                    "Next",
                    Action::MusicNextPage,
                    true,
                );
            } else {
                ui.rounded(
                    pager_x + prev_w + page_w + 16.0,
                    pager_y,
                    next_w,
                    30.0,
                    10.0,
                    my3_card_bg(),
                );
                ui.centered_fit(
                    Face::Medium,
                    11.5,
                    [pager_x + prev_w + page_w + 16.0, pager_y, next_w, 30.0],
                    my3_muted(),
                    "Next",
                );
            }
        } else {
            // Playlists View (includes Downloaded & Exported Music's offline playlists)
            let selected_name = self
                .catalog
                .music_selected_playlist
                .clone()
                .filter(|n| playlists.iter().any(|p| p.name == *n))
                .or_else(|| playlists.first().map(|p| p.name.clone()))
                .unwrap_or_else(|| "Favorites".to_owned());
            let is_virtual = internet_music::is_virtual_playlist(&selected_name);

            // Create New Playlist Row
            let new_y = 58.0;
            let create_w = 132.0;
            let field_w = (inner_w - create_w - 16.0).max(120.0);
            let np_rect = ui.text_field(
                pad + 4.0,
                new_y,
                field_w,
                34.0,
                "",
                &self.catalog.music_new_playlist_name,
                "New playlist name...",
                self.catalog.editing == Some("music_new_playlist"),
            );
            self.targets
                .push((np_rect, Action::Field("music_new_playlist")));
            button(
                ui,
                &mut self.targets,
                [pad + 4.0 + field_w + 8.0, new_y, create_w, 34.0],
                "+ Create Playlist",
                Action::MusicCreatePlaylist,
                true,
            );

            // Wrapping Playlist Selector Chips so Downloaded & Exported Music's are always visible
            let mut px = pad + 4.0;
            let mut py = 100.0;
            let chips_right = w - pad - (if is_virtual { 112.0 } else { 190.0 });
            for (pi, pl) in playlists.iter().enumerate() {
                let chip_lbl = format!("{} ({})", pl.name, pl.tracks.len());
                let cw = (chip_lbl.len() as f32 * 7.2 + 22.0).clamp(96.0, 200.0);
                if px + cw > chips_right && px > pad + 4.0 {
                    px = pad + 4.0;
                    py += 34.0;
                }
                let active = pl.name == selected_name;
                button(
                    ui,
                    &mut self.targets,
                    [px, py, cw, 28.0],
                    &chip_lbl,
                    Action::MusicSelectPlaylist(pi),
                    active,
                );
                px += cw + 6.0;
            }
            button(
                ui,
                &mut self.targets,
                [
                    w - pad - (if is_virtual { 106.0 } else { 184.0 }),
                    100.0,
                    102.0,
                    30.0,
                ],
                "Play All",
                Action::MusicPlaySelectedPlaylist,
                true,
            );
            if !is_virtual {
                button(
                    ui,
                    &mut self.targets,
                    [w - pad - 76.0, 100.0, 72.0, 30.0],
                    "Delete",
                    Action::MusicDeleteSelectedPlaylist,
                    false,
                );
            }

            let tracks = playlists
                .iter()
                .find(|p| p.name == selected_name)
                .map(|p| p.tracks.clone())
                .unwrap_or_default();

            let top = (py + 36.0).max(138.0);
            let bottom = (player_y - 8.0).max(top + 60.0);
            let row_h = 68.0;
            let total_h = tracks.len() as f32 * row_h;
            let max_scroll = (total_h - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
            self.list_view = Some([top, bottom, max_scroll]);

            ui.set_clip(Some([pad, top, inner_w, (bottom - top).max(1.0)]));
            if tracks.is_empty() {
                let empty_msg = if selected_name == internet_music::DOWNLOADED_PLAYLIST_NAME {
                    "No downloaded songs yet. Tap the Download icon beside any song in Discover."
                } else if selected_name == internet_music::EXPORTED_PLAYLIST_NAME {
                    "Put .mp3 / .ogg / .m4a / .wav / .flac files in the musics folder (File Manager -> Musics) to play offline!"
                } else {
                    "This playlist is empty. Go to Discover and tap + next to any song to add it!"
                };
                ui.text(Face::Regular, 12.5, pad + 12.0, top + 20.0, my3_muted(), empty_msg);
            }
            for (idx, track) in tracks.iter().enumerate() {
                let y = top + idx as f32 * row_h - self.catalog.scroll;
                if y + 60.0 < top || y > bottom {
                    continue;
                }
                let row_x = pad + 4.0;
                let row_w = (inner_w - 16.0).max(200.0);
                let is_playing_this = snap.has_track() && snap.current_url == track.stream_url;
                ui.rounded(
                    row_x,
                    y,
                    row_w,
                    60.0,
                    14.0,
                    if is_playing_this {
                        my3_accent_soft()
                    } else {
                        my3_card_bg()
                    },
                );
                ui.border(row_x, y, row_w, 60.0, 14.0, my3_card_border());

                let drawn = if let Some((iw, ih, rgba)) = self.catalog.music_icons.get(&track.id) {
                    ui.image(&track.id, *iw, *ih, rgba, row_x + 8.0, y + 8.0, 44.0, 44.0)
                } else {
                    false
                };
                if !drawn {
                    ui.rounded(row_x + 8.0, y + 8.0, 44.0, 44.0, 10.0, my3_accent_soft());
                    ui.icon(Icon::Music, row_x + 19.0, y + 19.0, 22.0, my3_accent());
                }

                let plus_rect = [row_x + 58.0, y + 13.0, 34.0, 34.0];
                ui.rounded(
                    plus_rect[0],
                    plus_rect[1],
                    plus_rect[2],
                    plus_rect[3],
                    10.0,
                    my3_surface(),
                );
                ui.border(
                    plus_rect[0],
                    plus_rect[1],
                    plus_rect[2],
                    plus_rect[3],
                    10.0,
                    my3_border(),
                );
                ui.icon(
                    Icon::Plus,
                    plus_rect[0] + 9.0,
                    plus_rect[1] + 9.0,
                    16.0,
                    my3_text(),
                );
                self.list_targets
                    .push((plus_rect, Action::MusicOpenSavePlaylistTrack(idx)));
                if let Some(_c) = ui.clip_rect(plus_rect) {
                    self.targets
                        .push((plus_rect, Action::MusicOpenSavePlaylistTrack(idx)));
                }

                let rem_rect = [row_x + row_w - 78.0, y + 13.0, 68.0, 34.0];
                let play_rect = [rem_rect[0] - 82.0, y + 13.0, 74.0, 34.0];
                let dur_str = if track.duration_secs > 0 {
                    format_duration(track.duration_secs)
                } else if !track.stream_url.starts_with("https://") {
                    "Offline".to_owned()
                } else {
                    "Stream".to_owned()
                };
                let dur_x = play_rect[0] - 58.0;
                ui.rounded(dur_x, y + 17.0, 50.0, 26.0, 8.0, my3_surface());
                ui.centered_fit(
                    Face::Medium,
                    11.0,
                    [dur_x, y + 17.0, 50.0, 26.0],
                    my3_muted(),
                    &dur_str,
                );

                let text_x = row_x + 100.0;
                let text_max = (dur_x - text_x - 8.0).max(80.0);
                let title_fit = ui.fit_text(Face::SemiBold, 14.5, &track.title, text_max);
                let sub_fit = ui.fit_text(Face::Regular, 12.0, &track.artist, text_max);
                ui.text(Face::SemiBold, 14.5, text_x, y + 10.0, my3_text(), &title_fit);
                ui.text(Face::Regular, 12.0, text_x, y + 31.0, my3_muted(), &sub_fit);

                if ui.clip_rect(play_rect).is_some() {
                    button(
                        ui,
                        &mut self.targets,
                        play_rect,
                        if is_playing_this && snap.playing {
                            "Playing"
                        } else {
                            "Play"
                        },
                        Action::MusicPlayPlaylistTrack(idx),
                        is_playing_this,
                    );
                }
                if ui.clip_rect(rem_rect).is_some() {
                    button(
                        ui,
                        &mut self.targets,
                        rem_rect,
                        if is_virtual { "Delete" } else { "Remove" },
                        Action::MusicRemoveTrackFromPlaylist(idx),
                        false,
                    );
                }
            }
            ui.set_clip(None);
        }

        // Bottom Now-Playing Player Bar
        ui.rounded(pad + 4.0, player_y, inner_w - 8.0, player_bar_h, 16.0, my3_card_bg());
        ui.border(pad + 4.0, player_y, inner_w - 8.0, player_bar_h, 16.0, my3_accent());

        let info_x = pad + 16.0;
        let now_title = if snap.has_track() {
            snap.current_title.as_str()
        } else {
            "No song playing"
        };
        let now_sub = if snap.has_track() {
            if !snap.status.is_empty() {
                snap.status.clone()
            } else if snap.buffering {
                "Buffering audio stream...".to_owned()
            } else if !snap.active_playlist.is_empty() {
                format!("{} • Playlist: {}", snap.current_artist, snap.active_playlist)
            } else {
                snap.current_artist.clone()
            }
        } else {
            "Tap any song above to start listening".to_owned()
        };
        let controls_w = 398.0;
        let info_max = (inner_w - controls_w - 28.0).max(90.0);
        let nt_fit = ui.fit_text(Face::SemiBold, 13.5, now_title, info_max);
        let ns_fit = ui.fit_text(Face::Regular, 11.0, &now_sub, info_max);
        ui.text(Face::SemiBold, 13.5, info_x, player_y + 8.0, my3_text(), &nt_fit);
        ui.text(Face::Regular, 11.0, info_x, player_y + 27.0, my3_muted(), &ns_fit);

        // Interactive Seek Bar inside Now-Playing Bar
        let seek_x = info_x;
        let seek_w = (info_max - 4.0).max(80.0);
        let seek_y = player_y + 46.0;
        let frac = if snap.duration_ms > 0 {
            (snap.position_ms as f32 / snap.duration_ms as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        ui.rounded(seek_x, seek_y, seek_w, 6.0, 3.0, my3_surface());
        if frac > 0.0 {
            ui.rounded(seek_x, seek_y, (seek_w * frac).max(6.0), 6.0, 3.0, my3_accent());
        }
        for seg in 0..8 {
            let sw = seek_w / 8.0;
            let sx = seek_x + seg as f32 * sw;
            self.targets.push((
                [sx, seek_y - 6.0, sw, 18.0],
                Action::MusicSeekFrac((seg as f32 + 0.5) / 8.0),
            ));
        }

        // Transport + Volume Buttons on Right
        let ctrl_x = (w - pad - controls_w - 8.0).max(info_x + info_max + 8.0);
        let time_lbl = if snap.duration_ms > 0 {
            format!("{} / {}", format_ms(snap.position_ms), format_ms(snap.duration_ms))
        } else {
            format_ms(snap.position_ms)
        };
        ui.text(Face::Medium, 10.5, ctrl_x, player_y + 6.0, my3_muted(), &time_lbl);

        let by = player_y + 24.0;
        button(ui, &mut self.targets, [ctrl_x, by, 52.0, 32.0], "Prev", Action::MusicPrev, false);
        button(
            ui,
            &mut self.targets,
            [ctrl_x + 56.0, by, 68.0, 32.0],
            if snap.playing { "Pause" } else { "Play" },
            Action::MusicTogglePause,
            true,
        );
        button(
            ui,
            &mut self.targets,
            [ctrl_x + 128.0, by, 58.0, 32.0],
            "+10s",
            Action::MusicSeekBy(10_000),
            false,
        );
        button(
            ui,
            &mut self.targets,
            [ctrl_x + 190.0, by, 52.0, 32.0],
            "Next",
            Action::MusicNext,
            false,
        );
        let vol_lbl = if snap.muted {
            "Muted".to_owned()
        } else {
            format!("Vol {}%", (snap.volume * 100.0).round() as i32)
        };
        button(
            ui,
            &mut self.targets,
            [ctrl_x + 248.0, by, 72.0, 32.0],
            &vol_lbl,
            Action::MusicToggleMute,
            snap.muted,
        );
        button(
            ui,
            &mut self.targets,
            [ctrl_x + 324.0, by, 32.0, 32.0],
            "-",
            Action::MusicStepVolume(-0.15),
            false,
        );
        button(
            ui,
            &mut self.targets,
            [ctrl_x + 360.0, by, 32.0, 32.0],
            "+",
            Action::MusicStepVolume(0.15),
            false,
        );

        // Sort By Modal
        if self.catalog.music_sort_modal {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            ui.rounded(0.0, 0.0, w, full_h, 0.0, horizon_client::hud::rgba(6, 5, 12, 195));
            let mw = (w - 36.0).clamp(280.0, 360.0);
            let mh = 278.0;
            let mx = (w - mw) * 0.5;
            let my = ((full_h - mh) * 0.5).max(12.0);
            ui.rounded(mx, my, mw, mh, 20.0, my3_surface());
            ui.border(mx, my, mw, mh, 20.0, my3_accent());
            ui.text(Face::SemiBold, 16.5, mx + 18.0, my + 16.0, my3_text(), "Sort Songs By");
            button(
                ui,
                &mut self.targets,
                [mx + mw - 86.0, my + 12.0, 70.0, 30.0],
                "Close",
                Action::MusicCloseSort,
                false,
            );
            for (si, sort_opt) in MusicSort::ALL.iter().enumerate() {
                let ry = my + 58.0 + si as f32 * 48.0;
                let selected = self.catalog.music_sort == *sort_opt;
                button(
                    ui,
                    &mut self.targets,
                    [mx + 18.0, ry, mw - 36.0, 40.0],
                    sort_opt.label(),
                    Action::MusicSetSort(*sort_opt),
                    selected,
                );
            }
        } else if self.catalog.music_downloads_modal {
            // Concurrent Song Downloads Progress Modal
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            ui.rounded(0.0, 0.0, w, full_h, 0.0, horizon_client::hud::rgba(6, 5, 12, 195));
            let mw = (w - 36.0).clamp(320.0, 480.0);
            let mh = (full_h - 36.0).clamp(240.0, 390.0);
            let mx = (w - mw) * 0.5;
            let my = ((full_h - mh) * 0.5).max(12.0);
            ui.rounded(mx, my, mw, mh, 20.0, my3_surface());
            ui.border(mx, my, mw, mh, 20.0, my3_accent());
            ui.text(
                Face::SemiBold,
                16.5,
                mx + 18.0,
                my + 16.0,
                my3_text(),
                "Song Downloads",
            );
            button(
                ui,
                &mut self.targets,
                [mx + mw - 86.0, my + 12.0, 70.0, 30.0],
                "Close",
                Action::MusicCloseDownloads,
                false,
            );
            if downloads.iter().any(|d| d.done) {
                button(
                    ui,
                    &mut self.targets,
                    [mx + mw - 196.0, my + 12.0, 102.0, 30.0],
                    "Clear Done",
                    Action::MusicClearFinishedDownloads,
                    false,
                );
            }
            if downloads.is_empty() {
                ui.text(
                    Face::Regular,
                    12.5,
                    mx + 18.0,
                    my + 68.0,
                    my3_muted(),
                    "No active or recent song downloads. Tap the Download icon on any song!",
                );
            } else {
                let mut dy = my + 56.0;
                for dl in downloads.iter().rev() {
                    if dy + 54.0 > my + mh - 14.0 {
                        break;
                    }
                    ui.rounded(mx + 16.0, dy, mw - 32.0, 48.0, 12.0, my3_card_bg());
                    ui.border(mx + 16.0, dy, mw - 32.0, 48.0, 12.0, my3_card_border());
                    let title_fit = ui.fit_text(Face::SemiBold, 12.5, &dl.title, mw - 160.0);
                    ui.text(Face::SemiBold, 12.5, mx + 26.0, dy + 6.0, my3_text(), &title_fit);
                    let status_txt = if let Some(err) = &dl.error {
                        format!("Failed: {err}")
                    } else if dl.done {
                        format!("Saved ({})", format_bytes(dl.downloaded_bytes))
                    } else if let Some(tot) = dl.total_bytes.filter(|t| *t > 0) {
                        format!(
                            "{} / {}",
                            format_bytes(dl.downloaded_bytes),
                            format_bytes(tot)
                        )
                    } else {
                        format!("{}...", format_bytes(dl.downloaded_bytes))
                    };
                    let st_fit = ui.fit_text(Face::Regular, 10.5, &status_txt, 140.0);
                    ui.text(
                        Face::Regular,
                        10.5,
                        mx + mw - 156.0,
                        dy + 7.0,
                        my3_muted(),
                        &st_fit,
                    );
                    let prog_w = mw - 52.0;
                    let frac = if dl.done && dl.error.is_none() {
                        1.0
                    } else if let Some(tot) = dl.total_bytes.filter(|t| *t > 0) {
                        (dl.downloaded_bytes as f32 / tot as f32).clamp(0.0, 1.0)
                    } else {
                        0.35
                    };
                    ui.rounded(mx + 26.0, dy + 31.0, prog_w, 6.0, 3.0, my3_surface());
                    ui.rounded(
                        mx + 26.0,
                        dy + 31.0,
                        (prog_w * frac).max(6.0),
                        6.0,
                        3.0,
                        my3_accent(),
                    );
                    dy += 54.0;
                }
            }
        } else if let Some(track) = self.catalog.music_save_modal.clone() {
            // Save-to-Playlist Modal Window (opened by tapping "+" beside a song's icon)
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            ui.rounded(
                0.0,
                0.0,
                w,
                full_h,
                0.0,
                horizon_client::hud::rgba(6, 5, 12, 195),
            );
            let mw = (w - 36.0).clamp(320.0, 480.0);
            let mh = (full_h - 32.0).clamp(260.0, 390.0);
            let mx = (w - mw) * 0.5;
            let max_my = ime_top_dp.map_or(full_h - mh - 12.0, |t| (t - mh - 10.0).max(10.0));
            let my = ((full_h - mh) * 0.5).min(max_my).max(10.0);
            ui.rounded(mx, my, mw, mh, 20.0, my3_surface());
            ui.border(mx, my, mw, mh, 20.0, my3_accent());

            ui.text(
                Face::SemiBold,
                16.5,
                mx + 18.0,
                my + 16.0,
                my3_text(),
                "Save Song to Playlist",
            );
            button(
                ui,
                &mut self.targets,
                [mx + mw - 86.0, my + 12.0, 70.0, 30.0],
                "Close",
                Action::MusicCloseSave,
                false,
            );

            let t_fit = ui.fit_text(Face::Medium, 13.0, &track.title, mw - 36.0);
            let a_fit = ui.fit_text(
                Face::Regular,
                11.0,
                &format!("{} • {}", track.artist, format_duration(track.duration_secs)),
                mw - 36.0,
            );
            ui.text(Face::Medium, 13.0, mx + 18.0, my + 46.0, my3_accent(), &t_fit);
            ui.text(Face::Regular, 11.0, mx + 18.0, my + 64.0, my3_muted(), &a_fit);

            // User playlist choices (exclude virtual Downloaded / Exported Music's)
            let list_y = my + 90.0;
            let list_h = (mh - 196.0).max(76.0);
            let row_step = 42.0;
            let mut row_idx = 0usize;
            for (pi, pl) in playlists.iter().enumerate() {
                if internet_music::is_virtual_playlist(&pl.name) {
                    continue;
                }
                let ry = list_y + row_idx as f32 * row_step;
                if ry + 36.0 > list_y + list_h {
                    break;
                }
                row_idx += 1;
                let already = pl
                    .tracks
                    .iter()
                    .any(|t| t.id == track.id || t.stream_url == track.stream_url);
                let label = if already {
                    format!("{} ({}) • Saved", pl.name, pl.tracks.len())
                } else {
                    format!("{} ({})", pl.name, pl.tracks.len())
                };
                button(
                    ui,
                    &mut self.targets,
                    [mx + 18.0, ry, mw - 36.0, 36.0],
                    &label,
                    Action::MusicAddToPlaylist(pi),
                    already,
                );
            }

            // Create New Playlist + Add
            let create_label_y = my + mh - 94.0;
            ui.text(
                Face::Medium,
                12.0,
                mx + 18.0,
                create_label_y,
                my3_text(),
                "Or create a new playlist:",
            );
            let input_y = my + mh - 70.0;
            let btn_w = 122.0;
            let inp_w = (mw - 44.0 - btn_w).max(120.0);
            let new_pl_rect = ui.text_field(
                mx + 18.0,
                input_y,
                inp_w,
                36.0,
                "",
                &self.catalog.music_new_playlist_name,
                "Playlist name...",
                self.catalog.editing == Some("music_new_playlist"),
            );
            self.targets
                .push((new_pl_rect, Action::Field("music_new_playlist")));
            button(
                ui,
                &mut self.targets,
                [mx + 24.0 + inp_w, input_y, btn_w, 36.0],
                "Create & Save",
                Action::MusicCreateAndAdd,
                true,
            );
        }
    }

    /// Modal dialogs for the Integrated Mini File Manager:
    /// - Mandatory Delete Confirmation (`FmModal::ConfirmDelete`)
    /// - Rename with `Ui::text_field` & path-traversal validation (`FmModal::Rename`)
    /// - Create New Folder with `Ui::text_field` (`FmModal::NewFolder`)
    /// - View File Details with metadata & read-only text preview (`FmModal::Details`)
    fn draw_fm_modal(&mut self, ui: &mut Ui, w: f32, h: f32, ime_top_dp: Option<f32>) {
        let Some(modal) = self.catalog.fm_modal.clone() else {
            return;
        };
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;

        let modal_t = self.anim.modal_anim.clamp(0.2, 1.0);
        ui.rounded(
            0.0,
            0.0,
            w,
            h,
            0.0,
            horizon_client::hud::rgba(6, 5, 12, (185.0 * modal_t) as u8),
        );

        match modal {
            crate::catalog::FmModal::ConfirmDelete { paths } => {
                let mw = (w - 32.0).min(420.0);
                let mh = 190.0;
                let mx = (w - mw) * 0.5;
                let my = (h - mh) * 0.5;
                ui.rounded(mx, my, mw, mh, 12.0, my3_surface());
                ui.border(mx, my, mw, mh, 12.0, my3_border());
                ui.icon(Icon::Trash, mx + 20.0, my + 20.0, 22.0, my3_accent());
                let title = if paths.len() == 1 {
                    "Delete selected item?".to_owned()
                } else {
                    format!("Delete {} selected items?", paths.len())
                };
                ui.text(
                    Face::SemiBold,
                    18.0,
                    mx + 50.0,
                    my + 18.0,
                    my3_text(),
                    &title,
                );
                let preview_names = paths.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
                ui.text_clipped(
                    Face::Medium,
                    12.5,
                    mx + 20.0,
                    my + 54.0,
                    mw - 40.0,
                    my3_accent(),
                    &preview_names,
                );
                ui.hint_line(
                    mx + 20.0,
                    my + 80.0,
                    mw - 40.0,
                    "This permanently deletes the selected file(s) from this profile and cannot be undone.",
                    my3_muted(),
                );
                let bw = (mw - 52.0) * 0.5;
                button(
                    ui,
                    &mut self.targets,
                    [mx + 20.0, my + mh - 54.0, bw, 38.0],
                    "Cancel",
                    Action::FmCloseModal,
                    false,
                );
                button(
                    ui,
                    &mut self.targets,
                    [mx + 32.0 + bw, my + mh - 54.0, bw, 38.0],
                    "Confirm Delete",
                    Action::FmConfirmDelete,
                    true,
                );
            }
            crate::catalog::FmModal::Rename { target_rel } => {
                let mw = (w - 32.0).min(430.0);
                let mh = 196.0;
                let mx = (w - mw) * 0.5;
                let mut my = (h - mh) * 0.5;
                if let Some(ime_y) = ime_top_dp {
                    my = my.min((ime_y - mh - 8.0).max(6.0));
                }
                ui.rounded(mx, my, mw, mh, 12.0, my3_surface());
                ui.border(mx, my, mw, mh, 12.0, my3_border());
                ui.text(
                    Face::SemiBold,
                    18.0,
                    mx + 20.0,
                    my + 16.0,
                    my3_text(),
                    "Rename File or Folder",
                );
                ui.hint_line(
                    mx + 20.0,
                    my + 42.0,
                    mw - 40.0,
                    &format!("Target: /{target_rel} (Slashes and '..' are rejected)"),
                    my3_muted(),
                );
                let field_r = ui.text_field(
                    mx + 20.0,
                    my + 68.0,
                    mw - 40.0,
                    48.0,
                    "NEW NAME",
                    &self.catalog.fm_input_text,
                    "Enter new filename",
                    self.catalog.editing == Some("fm_rename"),
                );
                self.targets.push((field_r, Action::Field("fm_rename")));
                let bw = (mw - 52.0) * 0.5;
                button(
                    ui,
                    &mut self.targets,
                    [mx + 20.0, my + mh - 52.0, bw, 38.0],
                    "Cancel",
                    Action::FmCloseModal,
                    false,
                );
                button(
                    ui,
                    &mut self.targets,
                    [mx + 32.0 + bw, my + mh - 52.0, bw, 38.0],
                    "Save Name",
                    Action::FmSubmitRename,
                    true,
                );
            }
            crate::catalog::FmModal::NewFolder => {
                let mw = (w - 32.0).min(430.0);
                let mh = 196.0;
                let mx = (w - mw) * 0.5;
                let mut my = (h - mh) * 0.5;
                if let Some(ime_y) = ime_top_dp {
                    my = my.min((ime_y - mh - 8.0).max(6.0));
                }
                ui.rounded(mx, my, mw, mh, 12.0, my3_surface());
                ui.border(mx, my, mw, mh, 12.0, my3_border());
                ui.text(
                    Face::SemiBold,
                    18.0,
                    mx + 20.0,
                    my + 16.0,
                    my3_text(),
                    "Create New Folder",
                );
                let loc = if self.catalog.fm_current_dir.is_empty() {
                    "/".to_owned()
                } else {
                    format!("/{}", self.catalog.fm_current_dir)
                };
                ui.hint_line(
                    mx + 20.0,
                    my + 42.0,
                    mw - 40.0,
                    &format!("Location: {loc}"),
                    my3_muted(),
                );
                let field_r = ui.text_field(
                    mx + 20.0,
                    my + 68.0,
                    mw - 40.0,
                    48.0,
                    "FOLDER NAME",
                    &self.catalog.fm_input_text,
                    "Enter folder name",
                    self.catalog.editing == Some("fm_new_folder"),
                );
                self.targets.push((field_r, Action::Field("fm_new_folder")));
                let bw = (mw - 52.0) * 0.5;
                button(
                    ui,
                    &mut self.targets,
                    [mx + 20.0, my + mh - 52.0, bw, 38.0],
                    "Cancel",
                    Action::FmCloseModal,
                    false,
                );
                button(
                    ui,
                    &mut self.targets,
                    [mx + 32.0 + bw, my + mh - 52.0, bw, 38.0],
                    "Create Folder",
                    Action::FmSubmitNewFolder,
                    true,
                );
            }
            crate::catalog::FmModal::Details(details) => {
                let mw = (w - 36.0).min(560.0);
                let mh = (h - 32.0).min(340.0);
                let mx = (w - mw) * 0.5;
                let my = (h - mh) * 0.5;
                ui.rounded(mx, my, mw, mh, 12.0, my3_surface());
                ui.border(mx, my, mw, mh, 12.0, my3_border());
                ui.text_clipped(
                    Face::SemiBold,
                    17.5,
                    mx + 20.0,
                    my + 14.0,
                    mw - 80.0,
                    my3_text(),
                    &details.name,
                );
                let close_r = [mx + mw - 50.0, my + 12.0, 34.0, 34.0];
                icon_action_button(
                    ui,
                    &mut self.targets,
                    close_r,
                    my3_surface_high(),
                    Icon::Close,
                    Action::FmCloseModal,
                );

                let meta_summary = format!(
                    "Path: /{}  •  Kind: {}  •  Size: {} ({} bytes)  •  Modified: {}",
                    details.rel_path,
                    details.kind_label,
                    crate::catalog::format_file_size(details.size_bytes),
                    details.size_bytes,
                    details.modified_label
                );
                ui.hint_line(mx + 20.0, my + 40.0, mw - 40.0, &meta_summary, my3_accent());
                ui.rounded(mx + 18.0, my + 62.0, mw - 36.0, 1.0, 0.0, my3_border());

                let box_y = my + 70.0;
                let box_h = (mh - 126.0).max(60.0);
                ui.rounded(mx + 18.0, box_y, mw - 36.0, box_h, 12.0, my3_card_bg());
                ui.border(mx + 18.0, box_y, mw - 36.0, box_h, 12.0, my3_card_border());
                ui.set_clip(Some([mx + 22.0, box_y + 4.0, mw - 44.0, box_h - 8.0]));
                for (i, line) in details.preview_lines.iter().enumerate() {
                    let ly = box_y + 8.0 + i as f32 * 16.0;
                    if ly + 14.0 > box_y + box_h {
                        break;
                    }
                    ui.text_clipped(Face::Mono, 10.5, mx + 26.0, ly, mw - 52.0, my3_text(), line);
                }
                ui.set_clip(None);

                let btn_y = my + mh - 46.0;
                if let Some(entry_idx) = self
                    .catalog
                    .fm_entries
                    .iter()
                    .position(|e| e.rel_path == details.rel_path)
                {
                    let bw = (mw - 56.0) / 4.0;
                    button(
                        ui,
                        &mut self.targets,
                        [mx + 18.0, btn_y, bw, 34.0],
                        "Rename",
                        Action::FmOpenRename(entry_idx),
                        false,
                    );
                    button(
                        ui,
                        &mut self.targets,
                        [mx + 24.0 + bw, btn_y, bw, 34.0],
                        "Export",
                        Action::FmExportSingle(entry_idx),
                        false,
                    );
                    button(
                        ui,
                        &mut self.targets,
                        [mx + 30.0 + bw * 2.0, btn_y, bw, 34.0],
                        "Delete",
                        Action::FmRequestDeleteSingle(entry_idx),
                        false,
                    );
                    button(
                        ui,
                        &mut self.targets,
                        [mx + 36.0 + bw * 3.0, btn_y, bw, 34.0],
                        "Done",
                        Action::FmCloseModal,
                        true,
                    );
                } else {
                    button(
                        ui,
                        &mut self.targets,
                        [mx + mw - 128.0, btn_y, 110.0, 34.0],
                        "Close",
                        Action::FmCloseModal,
                        true,
                    );
                }
            }
        }
    }

    /// Full screen Material 3 view for update detection and automated installation.
    fn draw_update_fullscreen_ui(&mut self, ui: &mut Ui, w: f32, h: f32) {
        let status = crate::updater::get_status();
        let (info, is_downloading, is_downloaded) = match &status {
            crate::updater::UpdateStatus::Available(info) => (Some(info.clone()), false, false),
            crate::updater::UpdateStatus::Downloading { .. } => {
                let info = self.catalog.update_info.clone();
                (info, true, false)
            }
            crate::updater::UpdateStatus::Downloaded { .. } => {
                let info = self.catalog.update_info.clone();
                (info, false, true)
            }
            crate::updater::UpdateStatus::Error(_) => {
                let info = self.catalog.update_info.clone();
                (info, false, false)
            }
            _ => (None, false, false),
        };

        if let Some(ref i) = info {
            self.catalog.update_info = Some(i.clone());
            if !self.catalog.update_ui_open && status != crate::updater::UpdateStatus::Dismissed {
                self.catalog.update_ui_open = true;
            }
        }

        if !self.catalog.update_ui_open {
            return;
        }

        let Some(info) = self.catalog.update_info.clone() else {
            return;
        };

        // Material 3 Animation: smooth slide-up and fade-in
        self.catalog.update_anim_time = (self.catalog.update_anim_time + 0.06).min(1.0);
        let ease = 1.0 - (1.0 - self.catalog.update_anim_time).powi(3);
        let slide_y = (1.0 - ease) * 60.0;

        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;

        // Full screen background scrim
        ui.rounded(
            0.0,
            0.0,
            w,
            h,
            0.0,
            horizon_client::hud::rgba(8, 7, 16, (235.0 * ease) as u8),
        );

        // Center Material 3 Container Card
        let mw = (w - 32.0).clamp(320.0, 680.0);
        let mh = (h - 28.0).clamp(340.0, 520.0);
        let mx = (w - mw) * 0.5;
        let my = (h - mh) * 0.5 + slide_y;

        ui.rounded(mx, my, mw, mh, 12.0, my3_surface());
        ui.border(mx, my, mw, mh, 12.0, my3_border());

        // Header with Material Icon & Accent Glow
        let head_y = my + 18.0;
        let icon_box_r = [mx + 20.0, head_y, 44.0, 44.0];
        ui.rounded(
            icon_box_r[0],
            icon_box_r[1],
            icon_box_r[2],
            icon_box_r[3],
            14.0,
            my3_accent_soft(),
        );
        ui.border(
            icon_box_r[0],
            icon_box_r[1],
            icon_box_r[2],
            icon_box_r[3],
            14.0,
            my3_accent(),
        );
        ui.icon(
            Icon::HorizonLogo,
            mx + 31.0,
            head_y + 11.0,
            22.0,
            my3_accent(),
        );

        let title_x = mx + 74.0;
        ui.text(
            Face::SemiBold,
            18.5,
            title_x,
            head_y + 2.0,
            my3_text(),
            "Volt Plus Update",
        );
        let badge_text = format!("v{}  →  v{}", crate::backend::VERSION, info.version);
        let badge_w = ui.measure(Face::SemiBold, 11.0, &badge_text) + 16.0;
        let badge_r = [title_x, head_y + 24.0, badge_w, 20.0];
        ui.rounded(
            badge_r[0],
            badge_r[1],
            badge_r[2],
            badge_r[3],
            6.0,
            my3_surface_high(),
        );
        ui.border(
            badge_r[0],
            badge_r[1],
            badge_r[2],
            badge_r[3],
            6.0,
            my3_accent(),
        );
        ui.centered_fit(Face::SemiBold, 11.0, badge_r, my3_accent(), &badge_text);

        // Changelog Content Box
        let box_y = head_y + 54.0;
        let box_h = mh - 138.0;
        let box_w = mw - 40.0;
        ui.rounded(mx + 20.0, box_y, box_w, box_h, 16.0, my3_surface_high());
        ui.border(mx + 20.0, box_y, box_w, box_h, 16.0, my3_border());

        ui.text(
            Face::SemiBold,
            13.0,
            mx + 32.0,
            box_y + 12.0,
            my3_accent(),
            "What's New in this release:",
        );

        let lines: Vec<&str> = info.changelog.lines().collect();
        let line_h = 18.0;
        let text_top = box_y + 34.0;
        let text_bottom = box_y + box_h - 10.0;
        let mut cur_ty = text_top;
        ui.set_clip(Some([
            mx + 28.0,
            text_top,
            box_w - 16.0,
            (text_bottom - text_top).max(1.0),
        ]));
        for line in lines {
            if cur_ty + line_h > text_bottom {
                break;
            }
            let is_header = line.starts_with('#');
            let is_bullet = line.starts_with('-') || line.starts_with('*');
            let (draw_text, face, size, col) = if is_header {
                (
                    line.trim_start_matches('#').trim(),
                    Face::SemiBold,
                    13.0,
                    my3_text(),
                )
            } else if is_bullet {
                (line, Face::Regular, 12.0, my3_text())
            } else {
                (line, Face::Regular, 11.5, my3_muted())
            };
            ui.text(face, size, mx + 32.0, cur_ty, col, draw_text);
            cur_ty += line_h;
        }
        ui.set_clip(None);

        // Progress or Status Bar when downloading / installing
        let status_y = my + mh - 70.0;
        match &status {
            crate::updater::UpdateStatus::Downloading {
                progress,
                downloaded,
                total,
            } => {
                let track_w = mw - 40.0;
                let track_r = [mx + 20.0, status_y, track_w, 8.0];
                ui.rounded(
                    track_r[0],
                    track_r[1],
                    track_r[2],
                    track_r[3],
                    4.0,
                    my3_border(),
                );
                let fill_w = (track_w * progress).clamp(4.0, track_w);
                ui.rounded(
                    track_r[0],
                    track_r[1],
                    fill_w,
                    track_r[3],
                    4.0,
                    my3_accent(),
                );
                let prog_lbl = format!(
                    "Downloading: {:.1} / {:.1} MiB ({:.0}%)",
                    *downloaded as f32 / 1048576.0,
                    *total as f32 / 1048576.0,
                    progress * 100.0
                );
                ui.text(
                    Face::Medium,
                    11.0,
                    mx + 20.0,
                    status_y - 16.0,
                    my3_accent(),
                    &prog_lbl,
                );
            }
            crate::updater::UpdateStatus::Downloaded { .. } => {
                ui.text(
                    Face::Medium,
                    11.5,
                    mx + 20.0,
                    status_y - 14.0,
                    my3_accent(),
                    "Download complete. Ready to install.",
                );
            }
            crate::updater::UpdateStatus::Error(e) => {
                ui.text(
                    Face::Medium,
                    11.0,
                    mx + 20.0,
                    status_y - 14.0,
                    horizon_client::hud::rgba(239, 68, 68, 255),
                    e,
                );
            }
            _ => {}
        }

        // Bottom CTA Buttons
        let btn_y = my + mh - 44.0;
        let btn_h = 32.0;
        let dismiss_w = 90.0;
        let update_btn_w = 170.0;

        button(
            ui,
            &mut self.targets,
            [mx + 20.0, btn_y, dismiss_w, btn_h],
            "Later",
            Action::CloseUpdateScreen,
            false,
        );

        let (btn_lbl, btn_act, btn_enabled) = if is_downloading {
            ("Downloading...", Action::TriggerUpdateInstall, false)
        } else if is_downloaded {
            ("Install Now", Action::TriggerUpdateInstall, true)
        } else {
            ("Update & Install", Action::TriggerUpdateInstall, true)
        };

        button(
            ui,
            &mut self.targets,
            [mx + mw - 20.0 - update_btn_w, btn_y, update_btn_w, btn_h],
            btn_lbl,
            btn_act,
            btn_enabled,
        );
    }

    /// Material You 3 Project Detail Dialog showing Modrinth PFP, Description,
    /// Gallery Screenshots, and selectable Versions to Download.
    fn draw_project_modal(&mut self, ui: &mut Ui, w: f32, h: f32) {
        // The install label asks the catalog about the project the modal is showing, so work it out
        // before the modal is borrowed mutably for drawing.
        let Some(install_label) = self.catalog.project_modal.as_ref().map(|modal| {
            let version = modal
                .versions
                .get(modal.selected_version)
                .map_or_else(|| "Latest".to_owned(), |v| v.number.clone());
            if let Some(bundled) = modal.bundled_id {
                if self.catalog.bundled_enabled.contains(bundled) {
                    "Installed (Tap to Disable)".to_owned()
                } else {
                    format!("Enable Built-in ({version})")
                }
            } else if let Some(state) = self.catalog.download_label(&modal.project) {
                state
            } else {
                format!("Download {} ({version})", modal.project.kind.label())
            }
        }) else {
            return;
        };
        let Some(modal) = &mut self.catalog.project_modal else {
            return;
        };
        self.targets.clear();
        self.list_targets.clear();

        // Scrim backdrop
        let alpha = (185.0 * self.anim.modal_anim.clamp(0.2, 1.0)) as u8;
        ui.rounded(
            0.0,
            0.0,
            w,
            h,
            0.0,
            horizon_client::hud::rgba(6, 5, 12, alpha),
        );

        let scale_in = 0.94 + 0.06 * self.anim.modal_anim.clamp(0.0, 1.0);
        let mw = ((w - 44.0).min(640.0) * scale_in).max(280.0);
        let mh = ((h - 36.0).min(385.0) * scale_in).max(230.0);
        let mx = (w - mw) * 0.5;
        let my = (h - mh) * 0.5;

        ui.rounded(mx, my, mw, mh, 12.0, my3_surface());
        ui.border(mx, my, mw, mh, 12.0, my3_border());

        // Modal Top Header (PFP + Title + Author/Downloads + Close Button)
        let icon_drawn = if let Some((iw, ih, rgba)) = self.catalog.icons.get(&modal.project.id) {
            ui.image(
                &modal.project.id,
                *iw,
                *ih,
                rgba,
                mx + 18.0,
                my + 16.0,
                50.0,
                50.0,
            )
        } else {
            false
        };
        if !icon_drawn {
            ui.rounded(mx + 18.0, my + 16.0, 50.0, 50.0, 14.0, my3_accent_soft());
            ui.icon(
                page_icon(self.catalog.page),
                mx + 31.0,
                my + 29.0,
                24.0,
                my3_accent(),
            );
        }

        let close_rect = [mx + mw - 54.0, my + 16.0, 38.0, 38.0];
        icon_action_button(
            ui,
            &mut self.targets,
            close_rect,
            my3_surface_high(),
            Icon::Close,
            Action::CloseProjectModal,
        );

        let title_max = (mw - 146.0).max(100.0);
        let fitted_title = ui.fit_text(Face::SemiBold, 18.5, &modal.project.title, title_max);
        ui.text(
            Face::SemiBold,
            18.5,
            mx + 78.0,
            my + 16.0,
            my3_text(),
            &fitted_title,
        );

        let mut meta_parts = vec![format!("by {}", modal.author)];
        if modal.downloads > 0 {
            meta_parts.push(format!("{} downloads", format_downloads(modal.downloads)));
        }
        if !modal.license.is_empty() {
            meta_parts.push(modal.license.clone());
        }
        if modal.loading {
            meta_parts.push("Syncing Modrinth details...".into());
        }
        let meta_line = meta_parts.join("  •  ");
        let fitted_meta = ui.fit_text(Face::Regular, 12.5, &meta_line, title_max);
        ui.text(
            Face::Regular,
            12.5,
            mx + 78.0,
            my + 41.0,
            my3_muted(),
            &fitted_meta,
        );

        // Divider under modal header
        ui.rounded(mx + 16.0, my + 74.0, mw - 32.0, 1.0, 0.0, my3_border());

        // Scrollable Body Area (Description + Gallery Screenshots + Versions List)
        let body_top = my + 82.0;
        let body_bottom = my + mh - 62.0;
        let inner_x = mx + 18.0;
        let inner_w = mw - 36.0;

        let desc_lines =
            wrap_text_lines(ui, Face::Regular, 13.0, &modal.description, inner_w - 8.0);
        let desc_h = (desc_lines.len() as f32 * 18.5).max(24.0);
        let gallery_h = if modal.screenshots.is_empty() {
            0.0
        } else {
            118.0
        };
        let versions_h = 28.0 + (modal.versions.len().min(6) as f32) * 38.0;
        let total_content_h = 24.0 + desc_h + gallery_h + versions_h + 16.0;
        let max_scroll = (total_content_h - (body_bottom - body_top)).max(0.0);
        modal.scroll = modal.scroll.clamp(0.0, max_scroll);
        self.list_view = Some([body_top, body_bottom, max_scroll]);

        ui.set_clip(Some([
            mx + 4.0,
            body_top,
            mw - 8.0,
            (body_bottom - body_top).max(1.0),
        ]));
        let mut cy = body_top - modal.scroll;

        // Section 1: About / Description
        if cy + 20.0 >= body_top && cy <= body_bottom {
            ui.text(
                Face::SemiBold,
                14.0,
                inner_x,
                cy,
                my3_accent(),
                "About Project",
            );
        }
        cy += 22.0;
        for line in &desc_lines {
            if cy + 18.0 >= body_top && cy <= body_bottom {
                ui.text(Face::Regular, 13.0, inner_x, cy, my3_text(), line);
            }
            cy += 18.5;
        }
        cy += 8.0;

        // Section 2: Screenshots (from Modrinth gallery)
        if !modal.screenshots.is_empty() {
            if cy + 20.0 >= body_top && cy <= body_bottom {
                ui.text(
                    Face::SemiBold,
                    14.0,
                    inner_x,
                    cy,
                    my3_accent(),
                    "Screenshots",
                );
            }
            cy += 22.0;
            let count = modal.screenshots.len().min(3) as f32;
            let gap = 10.0;
            let card_w = ((inner_w - gap * (count - 1.0)) / count).clamp(110.0, 200.0);
            let card_h = 84.0;
            if cy + card_h >= body_top && cy <= body_bottom {
                for (idx, (caption, sw, sh, rgba)) in modal.screenshots.iter().take(3).enumerate() {
                    let sx = inner_x + idx as f32 * (card_w + gap);
                    ui.rounded(sx, cy, card_w, card_h, 12.0, my3_card_bg());
                    ui.border(sx, cy, card_w, card_h, 12.0, my3_card_border());
                    let key = format!("shot:{}:{idx}", modal.project.id);
                    let _ = ui.image(
                        &key,
                        *sw,
                        *sh,
                        rgba,
                        sx + 3.0,
                        cy + 3.0,
                        card_w - 6.0,
                        card_h - 22.0,
                    );
                    let cap_fit = ui.fit_text(Face::Regular, 11.0, caption, card_w - 12.0);
                    ui.text(
                        Face::Regular,
                        11.0,
                        sx + 6.0,
                        cy + card_h - 17.0,
                        my3_muted(),
                        &cap_fit,
                    );
                }
            }
            cy += card_h + 12.0;
        }

        // Section 3: Selectable Versions to Download
        if cy + 20.0 >= body_top && cy <= body_bottom {
            ui.text(
                Face::SemiBold,
                14.0,
                inner_x,
                cy,
                my3_accent(),
                "Available Versions",
            );
        }
        cy += 24.0;
        for (v_idx, ver) in modal.versions.iter().take(6).enumerate() {
            let selected = modal.selected_version == v_idx;
            let row_rect = [inner_x, cy, inner_w, 34.0];
            self.list_targets
                .push((row_rect, Action::SelectModalVersion(v_idx)));
            if cy + 34.0 >= body_top && cy <= body_bottom {
                ui.rounded(
                    inner_x,
                    cy,
                    inner_w,
                    34.0,
                    10.0,
                    if selected {
                        my3_accent_soft()
                    } else {
                        my3_card_bg()
                    },
                );
                ui.border(
                    inner_x,
                    cy,
                    inner_w,
                    34.0,
                    10.0,
                    if selected {
                        my3_accent()
                    } else {
                        my3_card_border()
                    },
                );
                let ver_title = format!(
                    "{}v{}  •  MC {}  ({})",
                    if selected { "● " } else { "○ " },
                    ver.number,
                    ver.game_versions.first().map_or("26.3", String::as_str),
                    if ver.loaders.is_empty() {
                        modal.project.source.label().to_owned()
                    } else {
                        ver.loaders.join(", ")
                    }
                );
                let fitted_v = ui.fit_text(Face::Medium, 13.0, &ver_title, inner_w - 24.0);
                ui.text(
                    Face::Medium,
                    13.0,
                    inner_x + 12.0,
                    cy + 7.0,
                    if selected { my3_text() } else { my3_muted() },
                    &fitted_v,
                );
                if let Some(c) = ui.clip_rect(row_rect) {
                    self.targets.push((c, Action::SelectModalVersion(v_idx)));
                }
            }
            cy += 38.0;
        }

        ui.set_clip(None);
        // Modal Bottom Action Bar
        ui.rounded(mx + 16.0, my + mh - 56.0, mw - 32.0, 1.0, 0.0, my3_border());

        button(
            ui,
            &mut self.targets,
            [mx + 18.0, my + mh - 48.0, 108.0, 38.0],
            "Close",
            Action::CloseProjectModal,
            false,
        );
        button(
            ui,
            &mut self.targets,
            [mx + 136.0, my + mh - 48.0, mw - 154.0, 38.0],
            &install_label,
            Action::InstallModalProject,
            true,
        );
    }

    fn draw_diagnostics_summary_modal(&mut self, ui: &mut Ui, w: f32, h: f32) {
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;

        let modal_t = self.anim.modal_anim.clamp(0.25, 1.0);
        ui.rounded(
            0.0,
            0.0,
            w,
            h,
            0.0,
            horizon_client::hud::rgba(4, 6, 14, (198.0 * modal_t) as u8),
        );

        let mw = (w - 28.0).clamp(320.0, 560.0);
        let mh = (h - 24.0).clamp(248.0, 356.0);
        let mx = (w - mw) * 0.5;
        let my = (h - mh) * 0.5 + (1.0 - modal_t) * 10.0;

        ui.rounded(
            mx + 2.0,
            my + 6.0,
            mw,
            mh,
            22.0,
            horizon_client::hud::rgba(0, 0, 0, 175),
        );
        ui.rounded(mx, my, mw, mh, 12.0, my3_surface());
        ui.border(mx, my, mw, mh, 12.0, my3_accent());

        // Header icon & title
        ui.rounded(mx + 18.0, my + 14.0, 36.0, 36.0, 11.0, my3_accent_soft());
        ui.icon(Icon::Terminal, mx + 27.0, my + 23.0, 18.0, my3_accent());
        ui.text(
            Face::SemiBold,
            17.0,
            mx + 64.0,
            my + 13.0,
            my3_text(),
            "Diagnostics & Runtime Health Summary",
        );
        ui.text(
            Face::Regular,
            11.5,
            mx + 64.0,
            my + 33.0,
            my3_muted(),
            "Live renderer pipeline, profile state, error log & telemetry status",
        );

        ui.rounded(mx + 16.0, my + 58.0, mw - 32.0, 1.0, 0.0, my3_border());

        // 2x2 Metric Grid Cards
        let c = &self.catalog.config;
        let gx = mx + 18.0;
        let gw = mw - 36.0;
        let half_w = (gw - 10.0) * 0.5;
        let card1_y = my + 68.0;
        let card2_y = card1_y + 52.0;

        let gpu_txt = format!(
            "{}  •  {}% Scale  •  {}",
            c.renderer.label(),
            c.render_scale.percent(),
            c.device.display_label()
        );
        let prof_txt = format!(
            "Profile {}  •  UI {}  •  {}",
            c.profile + 1,
            c.ui_scale_label(),
            c.input_host.label()
        );
        let anim_txt = format!(
            "Launcher Anim: {}  •  Game Anim: {}",
            if c.disable_launcher_animations {
                "OFF"
            } else {
                "ON"
            },
            if c.disable_game_animations {
                "OFF"
            } else {
                "ON"
            }
        );
        let perf_txt = format!(
            "SuperPerf: {}  •  FastUpload: {}  •  ADPF: {}",
            if c.super_perf_mode { "ON" } else { "OFF" },
            if c.fast_chunk_upload { "ON" } else { "OFF" },
            if c.device.performance_hints {
                "ON"
            } else {
                "OFF"
            }
        );

        for (r_idx, (title, val, x_off, y_pos)) in [
            ("GRAPHICS & DISPLAY PIPELINE", gpu_txt.as_str(), gx, card1_y),
            (
                "ACTIVE PROFILE & INPUT HOST",
                prof_txt.as_str(),
                gx + half_w + 10.0,
                card1_y,
            ),
            ("ANIMATION & MOTION STATE", anim_txt.as_str(), gx, card2_y),
            (
                "PERFORMANCE & UPLOAD BOOST",
                perf_txt.as_str(),
                gx + half_w + 10.0,
                card2_y,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let _ = r_idx;
            ui.rounded(x_off, y_pos, half_w, 46.0, 12.0, my3_card_bg());
            ui.border(x_off, y_pos, half_w, 46.0, 12.0, my3_card_border());
            ui.text(
                Face::SemiBold,
                9.5,
                x_off + 10.0,
                y_pos + 6.0,
                my3_accent(),
                title,
            );
            let fit_v = ui.fit_text(Face::Medium, 11.2, val, half_w - 20.0);
            ui.text(
                Face::Medium,
                11.2,
                x_off + 10.0,
                y_pos + 23.0,
                my3_text(),
                &fit_v,
            );
        }

        // Summary & Error Log Box
        let log_y = card2_y + 54.0;
        let log_h = (mh - (log_y - my) - 60.0).max(52.0);
        ui.rounded(gx, log_y, gw, log_h, 12.0, my3_card_bg());
        ui.border(gx, log_y, gw, log_h, 12.0, my3_card_border());

        let summary_str = crate::diagnostics::summary(&self.home);
        let err_str = std::fs::read_to_string(self.home.join("android-error.txt"))
            .ok()
            .map(|e| crate::diagnostics::redact(&e))
            .unwrap_or_else(|| {
                "None — last session exited cleanly with zero unhandled errors.".into()
            });

        ui.text(
            Face::SemiBold,
            10.0,
            gx + 12.0,
            log_y + 6.0,
            my3_accent(),
            "DIAGNOSTICS SUMMARY & LAST HANDLED ERROR",
        );
        let fit_sum = ui.fit_text(Face::Regular, 11.0, &summary_str, gw - 24.0);
        ui.text(
            Face::Regular,
            11.0,
            gx + 12.0,
            log_y + 22.0,
            my3_text(),
            &fit_sum,
        );
        let fit_err = ui.fit_text(
            Face::Mono,
            10.2,
            &format!("Last Error: {err_str}"),
            gw - 24.0,
        );
        ui.text(
            Face::Mono,
            10.2,
            gx + 12.0,
            log_y + 39.0,
            my3_muted(),
            &fit_err,
        );

        // Bottom Action Buttons: Close | Open Reports | Export ZIP
        let btn_y = my + mh - 46.0;
        let bw3 = (gw - 16.0) / 3.0;
        button(
            ui,
            &mut self.targets,
            [gx, btn_y, bw3, 36.0],
            "Close",
            Action::CloseDiagnosticsSummary,
            false,
        );
        button(
            ui,
            &mut self.targets,
            [gx + bw3 + 8.0, btn_y, bw3, 36.0],
            "Open Reports",
            Action::OpenReportsFolder,
            false,
        );
        button(
            ui,
            &mut self.targets,
            [gx + (bw3 + 8.0) * 2.0, btn_y, bw3, 36.0],
            "Export ZIP",
            Action::ExportDiagnostics,
            true,
        );
    }

    fn draw_controls_studio_page(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
    ) {
        use horizon_client::touch::{ControlStyle, Settings as TouchSettings};

        let header_y = 62.0 + slide_y;
        ui.text(
            Face::SemiBold,
            19.5,
            content_x,
            header_y,
            my3_text(),
            "Controls Studio",
        );
        ui.text(
            Face::Regular,
            11.0,
            content_x,
            header_y + 23.0,
            my3_muted(),
            "Import custom control layouts (Pojav, Zalith, FCL, MCPE, Horizon) or customize HUD controls",
        );

        let game_home = self.catalog.config.game_home(&self.home);
        let touch_cfg = TouchSettings::load_from(&game_home.join("horizon-touch.json"));
        let active_style = touch_cfg.control_style;

        // Mode selector strip (MCPE vs GoyControl)
        let mode_y = header_y + 44.0;
        let mode_w = (content_w - 10.0) * 0.5;
        for (i, (label, style)) in [
            ("MCPE Classic", ControlStyle::Mcpe),
            ("GoyControl / Java", ControlStyle::GoyControl),
        ]
        .into_iter()
        .enumerate()
        {
            let rx = content_x + i as f32 * (mode_w + 10.0);
            let rect = [rx, mode_y, mode_w, 34.0];
            let selected = active_style == style;
            ui.rounded(
                rx,
                mode_y,
                mode_w,
                34.0,
                14.0,
                if selected { my3_accent_soft() } else { my3_card_bg() },
            );
            ui.border(
                rx,
                mode_y,
                mode_w,
                34.0,
                14.0,
                if selected { my3_accent() } else { my3_card_border() },
            );
            ui.centered_fit(
                Face::SemiBold,
                12.0,
                rect,
                if selected { my3_text() } else { my3_muted() },
                label,
            );
            self.targets.push((rect, Action::SelectControlMode(style)));
        }

        // Studio toolbar buttons
        let bar_y = mode_y + 42.0;
        let btn_specs = [
            ("Import Custom Control", Action::ImportControlProfile, true),
            ("Layout Editor", Action::OpenControlEditor, true),
            ("Export JSON", Action::ExportControlProfile, false),
            ("Duplicate", Action::CopyActivePreset, false),
        ];
        let cols = if compact { 2 } else { 4 };
        let gap = 8.0;
        let btn_w = (content_w - gap * (cols as f32 - 1.0)) / cols as f32;
        for (idx, (label, act, primary)) in btn_specs.into_iter().enumerate() {
            let col = idx % cols;
            let row = idx / cols;
            let bx = content_x + col as f32 * (btn_w + gap);
            let by = bar_y + row as f32 * 36.0;
            button(ui, &mut self.targets, [bx, by, btn_w, 30.0], label, act, primary);
        }

        let list_top = bar_y + if compact { 78.0 } else { 40.0 };
        let list_bottom = h - 42.0;
        let mut entries = Vec::new();
        for style in [ControlStyle::GoyControl, ControlStyle::Mcpe] {
            for (idx, meta) in TouchSettings::list_presets(&game_home, style)
                .into_iter()
                .enumerate()
            {
                entries.push((style, idx, meta));
            }
        }
        let step_y = 64.0;
        let max_scroll =
            (entries.len() as f32 * step_y - (list_bottom - list_top)).max(0.0);
        self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
        self.list_view = Some([list_top, list_bottom, max_scroll]);

        ui.set_clip(Some([
            content_x - 2.0,
            list_top,
            content_w + 4.0,
            (list_bottom - list_top).max(1.0),
        ]));
        for (row_i, (style, idx, meta)) in entries.into_iter().enumerate() {
            let ry = list_top + row_i as f32 * step_y - self.catalog.scroll;
            if ry + 58.0 < list_top || ry > list_bottom {
                continue;
            }
            let active = active_style == style && touch_cfg.preset_id == meta.id;
            let card = [content_x, ry, content_w, 56.0];
            ui.rounded(
                card[0],
                card[1],
                card[2],
                card[3],
                14.0,
                if active { my3_surface_high() } else { my3_card_bg() },
            );
            ui.border(
                card[0],
                card[1],
                card[2],
                card[3],
                14.0,
                if active { my3_accent() } else { my3_card_border() },
            );
            let badge_w = 88.0;
            let badge_x = content_x + 12.0;
            let badge_y = ry + 16.0;
            ui.rounded(
                badge_x,
                badge_y,
                badge_w,
                24.0,
                10.0,
                if active { my3_accent_soft() } else { my3_surface() },
            );
            ui.centered_fit(
                Face::SemiBold,
                9.5,
                [badge_x + 4.0, badge_y, badge_w - 8.0, 24.0],
                my3_text(),
                if style == ControlStyle::GoyControl {
                    "GOYCONTROL"
                } else {
                    "MCPE"
                },
            );
            let text_x = badge_x + badge_w + 12.0;
            let btns_reserve = if meta.is_default { 134.0 } else { 216.0 };
            let text_w = (content_w - badge_w - btns_reserve).max(70.0);
            let title = ui.fit_text(Face::SemiBold, 13.5, &meta.name, text_w);
            let desc = ui.fit_text(Face::Regular, 10.5, &meta.description, text_w);
            ui.text(Face::SemiBold, 13.5, text_x, ry + 10.0, my3_text(), &title);
            ui.text(Face::Regular, 10.5, text_x, ry + 30.0, my3_muted(), &desc);

            let act_rect = [content_x + content_w - 94.0, ry + 13.0, 82.0, 30.0];
            let action = Action::ActivateStudioPreset(style, idx as u32);
            if !meta.is_default {
                let del_rect = [content_x + content_w - 178.0, ry + 13.0, 76.0, 30.0];
                let del_action = Action::DeleteStudioPreset(style, idx as u32);
                ui.rounded(
                    del_rect[0],
                    del_rect[1],
                    del_rect[2],
                    del_rect[3],
                    15.0,
                    0xff4a_2028,
                );
                ui.border(
                    del_rect[0],
                    del_rect[1],
                    del_rect[2],
                    del_rect[3],
                    15.0,
                    0xffb8_4a58,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    11.0,
                    del_rect,
                    0xffff_d7dc,
                    "Delete",
                );
                if let Some(clipped) = ui.clip_rect(del_rect) {
                    self.targets.push((clipped, del_action.clone()));
                }
                self.list_targets.push((del_rect, del_action));
            }
            ui.rounded(
                act_rect[0],
                act_rect[1],
                act_rect[2],
                act_rect[3],
                15.0,
                if active { my3_accent() } else { my3_surface_high() },
            );
            ui.centered_fit(
                Face::SemiBold,
                11.0,
                act_rect,
                if active { 0xff1b_1327 } else { my3_text() },
                if active { "Active" } else { "Use" },
            );
            let card_use_rect = if meta.is_default {
                card
            } else {
                [card[0], card[1], (content_w - 184.0).max(40.0), card[3]]
            };
            if let Some(clipped) = ui.clip_rect(card_use_rect) {
                self.targets.push((clipped, action.clone()));
            }
            if !meta.is_default
                && let Some(clipped) = ui.clip_rect(act_rect)
            {
                self.targets.push((clipped, action.clone()));
            }
            self.list_targets.push((act_rect, action));
        }
        ui.set_clip(None);
    }

    /// Revamped Settings tab matching `45062.jpg` with 2-column MY3 category cards
    /// and dedicated sub-pages for Video, Launcher, Controls, Experimental, and Diagnostics.
    #[allow(clippy::too_many_arguments)]
    fn draw_settings_page(
        &mut self,
        ui: &mut Ui,
        content_x: f32,
        content_w: f32,
        h: f32,
        compact: bool,
        slide_y: f32,
        ime_top_dp: Option<f32>,
    ) {
        let cat = self.catalog.settings_category;
        if cat == SettingsCategory::Overview {
            ui.text(
                Face::SemiBold,
                20.0,
                content_x,
                62.0 + slide_y,
                my3_text(),
                "Settings",
            );

            let cards: [(Icon, &str, &str, Action); 8] = [
                (
                    Icon::Monitor,
                    "Video Settings",
                    "Rendering, resolution and performance",
                    Action::SettingsCat(SettingsCategory::Video),
                ),
                (
                    Icon::Gear,
                    "Launcher Settings",
                    "Theme customization, profiles, DPI scale & preferences",
                    Action::SettingsCat(SettingsCategory::Launcher),
                ),
                (
                    Icon::Gamepad,
                    "Controls",
                    "HUD touch layout, buttons, sensitivity and profiles",
                    Action::SettingsCat(SettingsCategory::Controls),
                ),
                (
                    Icon::Mouse,
                    "Mouse Customization",
                    "Cursor icon, pointer size, opacity & virtual mouse",
                    Action::SettingsCat(SettingsCategory::Mouse),
                ),
                (
                    Icon::Gamepad,
                    "Controls Studio",
                    "Import custom controls, edit HUD & switch presets",
                    Action::Page(Page::ControlsStudio),
                ),
                (
                    Icon::Flask,
                    "Experimental Stuff",
                    "Unstable features. We put the weird shit here.",
                    Action::SettingsCat(SettingsCategory::Experimental),
                ),
                (
                    Icon::Terminal,
                    "Diagnostics",
                    "Logs, device info and troubleshooting",
                    Action::SettingsCat(SettingsCategory::Diagnostics),
                ),
                (
                    Icon::MicrosoftGrid,
                    "Account",
                    "Microsoft and Xbox sign-in",
                    Action::Account,
                ),
            ];

            let grid_top = 98.0 + slide_y;
            let gap_x = 14.0;
            let gap_y = 10.0;
            let card_h = if compact {
                ((h - grid_top - 44.0 - gap_y * 7.0) / 8.0).clamp(38.0, 60.0)
            } else {
                ((h - grid_top - 48.0 - gap_y * 3.0) / 4.0).clamp(52.0, 76.0)
            };

            for (idx, (icon, title, subtitle, action)) in cards.into_iter().enumerate() {
                let (cx, cy, card_w) = if compact {
                    (content_x, grid_top + idx as f32 * (card_h + gap_y), content_w)
                } else {
                    let col = idx % 2;
                    let row = idx / 2;
                    let w2 = (content_w - gap_x) * 0.5;
                    (
                        content_x + col as f32 * (w2 + gap_x),
                        grid_top + row as f32 * (card_h + gap_y),
                        w2,
                    )
                };
                let rect = [cx, cy, card_w, card_h];

                ui.rounded(cx, cy, card_w, card_h, 16.0, my3_card_bg());
                ui.border(cx, cy, card_w, card_h, 16.0, my3_card_border());

                // Left vector icon + vertical divider line matching 45062.jpg
                let icon_size = 20.0;
                ui.icon(
                    icon,
                    cx + 20.0,
                    cy + (card_h - icon_size) * 0.5,
                    icon_size,
                    my3_accent(),
                );
                ui.rounded(
                    cx + 54.0,
                    cy + card_h * 0.26,
                    1.5,
                    card_h * 0.48,
                    0.75,
                    my3_border(),
                );

                let text_x = cx + 68.0;
                let text_max = (card_w - 82.0).max(60.0);
                let fitted_title = ui.fit_text(Face::SemiBold, 15.5, title, text_max);
                let fitted_sub = ui.fit_text(Face::Regular, 12.5, subtitle, text_max);
                let text_block_y = cy + (card_h - 40.0) * 0.5;
                ui.text(
                    Face::SemiBold,
                    15.5,
                    text_x,
                    text_block_y,
                    my3_text(),
                    &fitted_title,
                );
                ui.text(
                    Face::Regular,
                    12.5,
                    text_x,
                    text_block_y + 22.0,
                    my3_muted(),
                    &fitted_sub,
                );

                self.list_targets.push((rect, action));
                self.targets.push((rect, action));
            }
            return;
        }

        // Sub-category Header Bar with hierarchical Back button
        let header_y = 60.0 + slide_y;
        let back_cat = if cat == SettingsCategory::Theme {
            SettingsCategory::Launcher
        } else {
            SettingsCategory::Overview
        };
        button(
            ui,
            &mut self.targets,
            [content_x, header_y, 88.0, 32.0],
            "← Back",
            Action::SettingsCat(back_cat),
            false,
        );
        let fitted_cat = ui.fit_text(
            Face::SemiBold,
            18.5,
            cat.title(),
            (content_w - 104.0).max(80.0),
        );
        ui.text(
            Face::SemiBold,
            18.5,
            content_x + 100.0,
            header_y + 3.0,
            my3_text(),
            &fitted_cat,
        );

        let list_top = 102.0 + slide_y;
        let list_bottom = h - 42.0;
        let card_h = 58.0;
        let step_y = 66.0;

        let game_home = self.catalog.config.game_home(&self.home);
        let opts = horizon_client::settings::Settings::load(&game_home.join("options.txt"));
        let touch_cfg =
            horizon_client::touch::Settings::load_from(&game_home.join("horizon-touch.json"));
        self.mouse_touch = touch_cfg.clone();

        match cat {
            SettingsCategory::Video => {
                let c = &self.catalog.config;
                let count = 17;
                let extra_ime = if ime_top_dp.is_some() { 180.0 } else { 0.0 };
                let max_scroll =
                    (count as f32 * step_y + extra_ime - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let renderer_str = format!("{} v", c.renderer.label());
                let res_pct = c.render_scale.percent().clamp(25, 100);
                let res_frac = ((res_pct - 25) as f32 / 75.0).clamp(0.0, 1.0);
                let res_str = if self.catalog.editing == Some("render_scale") {
                    self.catalog.render_scale_text.clone()
                } else {
                    format!("{res_pct}%")
                };
                let hz_str = format!("{} v", c.device.display_label());
                let aspect_str = format!("{} v", c.screen.aspect.label());
                let orient_str = format!("{} v", c.orientation.label());
                let rd_str = format!("{} chunks", opts.render_distance);
                let rd_frac = ((opts.render_distance - 2) as f32 / 30.0).clamp(0.0, 1.0);
                let sd_str = format!("{} chunks", opts.simulation_distance);
                let sd_frac = ((opts.simulation_distance - 5) as f32 / 27.0).clamp(0.0, 1.0);
                let fov_str = format!("{:.0}°", opts.fov);
                let fov_frac = ((opts.fov - 30.0) / 80.0).clamp(0.0, 1.0);
                let preset_str = format!("{} ›", capitalize(&opts.graphics_preset));
                let fps_str = if opts.max_fps >= 260 {
                    "Unlimited ›".to_owned()
                } else {
                    format!("{} FPS ›", opts.max_fps)
                };
                let clouds_str = match opts.clouds.as_str() {
                    "false" => "Off ›",
                    "fast" => "Fast ›",
                    _ => "Fancy ›",
                };
                let game_anim_off = c.disable_game_animations || touch_cfg.disable_game_animations;

                let rows: [(&str, &str, SettingControl<'_>); 18] = [
                    (
                        "DISPLAY, RESOLUTION & RENDERER PIPELINE",
                        "Hardware GPU backend, 3D resolution scale, refresh rate & display geometry",
                        SettingControl::SectionHeader(c.renderer.label(), Icon::Monitor),
                    ),
                    (
                        "Rendering backend (Vulkan / GLES)",
                        "Hardware GPU driver pipeline used when launching Minecraft",
                        SettingControl::Button(&renderer_str, Action::Renderer, true),
                    ),
                    (
                        "Client resolution scale",
                        "3D framebuffer resolution percentage (also adjustable live in-game via Floating Menu)",
                        if self.catalog.editing == Some("render_scale") {
                            SettingControl::TextField {
                                label: "SCALE %",
                                value: &self.catalog.render_scale_text,
                                hint: "25 - 100%",
                                active: true,
                                action: Action::Field("render_scale"),
                            }
                        } else {
                            SettingControl::Slider {
                                value_label: &res_str,
                                fraction: res_frac,
                                minus_action: Action::RenderScaleDelta(-5),
                                plus_action: Action::RenderScaleDelta(5),
                            }
                        },
                    ),
                    (
                        "Display refresh rate",
                        "Preferred Android screen refresh rate (60Hz / 90Hz / 120Hz)",
                        SettingControl::Button(&hz_str, Action::DisplayHz, false),
                    ),
                    (
                        "Display aspect ratio",
                        "Native fullscreen, 16:9 widescreen, 4:3 classic or square",
                        SettingControl::Button(&aspect_str, Action::Aspect, false),
                    ),
                    (
                        "Screen orientation",
                        "Sensor landscape, locked landscape or portrait layout",
                        SettingControl::Button(&orient_str, Action::Orientation, false),
                    ),
                    (
                        "Immersive fullscreen",
                        "Hide Android system navigation bars during launcher and gameplay",
                        SettingControl::Toggle(c.screen.immersive, Action::Immersive),
                    ),
                    (
                        "Avoid camera cutout / notch",
                        "Inset HUD away from punch-hole cameras and curved display edges",
                        SettingControl::Toggle(c.screen.avoid_cutout, Action::Cutout),
                    ),
                    (
                        "Keep screen from turning off",
                        "Prevent the display from sleeping while Volt Plus is open",
                        SettingControl::Toggle(c.screen.keep_screen_on, Action::KeepScreenOn),
                    ),
                    (
                        "IN-GAME GRAPHICS, DISTANCES & MOTION",
                        "Chunk visibility, simulation radius, FOV, framerate & animation controls",
                        SettingControl::SectionHeader("Minecraft Video", Icon::Shaders),
                    ),
                    (
                        "Render Distance",
                        "Visible chunk radius (6–8 chunks recommended on Mali-G52 with shaders)",
                        SettingControl::Slider {
                            value_label: &rd_str,
                            fraction: rd_frac,
                            minus_action: Action::RenderDistDelta(-2),
                            plus_action: Action::RenderDistDelta(2),
                        },
                    ),
                    (
                        "Simulation Distance",
                        "Active entity and block-tick radius around the player",
                        SettingControl::Slider {
                            value_label: &sd_str,
                            fraction: sd_frac,
                            minus_action: Action::SimDistDelta(-1),
                            plus_action: Action::SimDistDelta(1),
                        },
                    ),
                    (
                        "Field of View (FOV)",
                        "Horizontal camera view angle in degrees",
                        SettingControl::Slider {
                            value_label: &fov_str,
                            fraction: fov_frac,
                            minus_action: Action::FovDelta(-5.0),
                            plus_action: Action::FovDelta(5.0),
                        },
                    ),
                    (
                        "Graphics Quality Preset",
                        "Cycle between Fast, Fancy and Fabulous rendering modes",
                        SettingControl::Button(&preset_str, Action::CycleGraphicsPreset, false),
                    ),
                    (
                        "Max Framerate Ceiling",
                        "Cap gameplay FPS to save power or unlock for high-refresh displays",
                        SettingControl::Button(&fps_str, Action::CycleMaxFps, false),
                    ),
                    (
                        "Vertical Sync & Cloud Rendering",
                        "Tap switch to toggle VSync, or cycle Clouds in in-game video options",
                        SettingControl::Toggle(opts.vsync, Action::ToggleVsync),
                    ),
                    (
                        "Menu Background Blur",
                        "Cached zero-lag backdrop blur when opening inventory or pause menus",
                        SettingControl::Toggle(opts.menu_blur > 0, Action::ToggleMenuBlur),
                    ),
                    (
                        "Disable In-Game Animations",
                        "Turn off animated block textures, chunk fade, hand bobbing & UI transitions completely",
                        SettingControl::Toggle(game_anim_off, Action::ToggleGameAnimations),
                    ),
                ];
                let _ = clouds_str;

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        &mut self.anim,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Launcher => {
                let c = &self.catalog.config;
                let count = 11;
                let extra_ime = if ime_top_dp.is_some() { 180.0 } else { 0.0 };
                let max_scroll =
                    (count as f32 * step_y + extra_ime - (list_bottom - list_top)).max(0.0);
                if self.catalog.editing == Some("server")
                    && let Some(ime_y) = ime_top_dp
                {
                    let server_y = list_top + 8.0 * step_y - self.catalog.scroll;
                    if server_y + card_h > ime_y - 12.0 {
                        self.catalog.scroll = (list_top + 8.0 * step_y + card_h - (ime_y - 16.0))
                            .clamp(0.0, max_scroll);
                    }
                }
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let ui_scale_str = format!("{} v", c.ui_scale_label());
                let profile_str = format!("Profile {} v", c.profile + 1);
                let input_str = format!("{} v", c.input_host.label());
                let worker_str = if c.worker_limit == 0 {
                    "Auto (Big.LITTLE Safe) v".to_owned()
                } else {
                    format!("{} threads v", c.worker_limit)
                };
                let cache_str = if c.cache_budget_mib == 0 {
                    "Default (80 MiB) v".to_owned()
                } else {
                    format!("{} MiB v", c.cache_budget_mib)
                };
                let server_str = if c.server.is_empty() {
                    "Singleplayer / Tap to set".to_owned()
                } else {
                    c.server.clone()
                };
                let game_anim_off = c.disable_game_animations || touch_cfg.disable_game_animations;

                let rows: [(&str, &str, SettingControl<'_>); 11] = [
                    (
                        "Launcher UI scale (Phone DPI)",
                        "Adapts launcher cards, text and buttons to your screen DPI",
                        SettingControl::Button(&ui_scale_str, Action::UiScale, false),
                    ),
                    (
                        "Disable Launcher Animations",
                        "Turn off page transitions, nav rail slides, shimmer & modal animations in launcher",
                        SettingControl::Toggle(
                            c.disable_launcher_animations,
                            Action::ToggleLauncherAnimations,
                        ),
                    ),
                    (
                        "Disable In-Game Animations",
                        "Turn off animated block textures, chunk fade, view bobbing & HUD animations in-game",
                        SettingControl::Toggle(game_anim_off, Action::ToggleGameAnimations),
                    ),
                    (
                        "PROFILES, WORKER THREADS & STORAGE",
                        "Isolated game profiles, CPU worker threads, RAM cache & direct server launch",
                        SettingControl::SectionHeader(&profile_str, Icon::Folder),
                    ),
                    (
                        "Active game profile",
                        "Each profile isolates its own worlds, mods and options",
                        SettingControl::Button(&profile_str, Action::Profile, false),
                    ),
                    (
                        "Input host mode",
                        "Switch between touchscreen HUD and physical mouse/keyboard",
                        SettingControl::Button(&input_str, Action::InputHost, false),
                    ),
                    (
                        "Chunk worker thread ceiling",
                        "Background worldgen and chunk meshing worker limit",
                        SettingControl::Button(&worker_str, Action::WorkerLimit, false),
                    ),
                    (
                        "Reusable RAM cache budget",
                        "Pooled audio & mesh buffer cache ceiling",
                        SettingControl::Button(&cache_str, Action::CacheBudget, false),
                    ),
                    (
                        "Quick-join server address",
                        "Optional server IP:port to connect immediately on Play",
                        SettingControl::TextField {
                            label: "SERVER IP",
                            value: &c.server,
                            hint: &server_str,
                            active: self.catalog.editing == Some("server"),
                            action: Action::Field("server"),
                        },
                    ),
                    (
                        "Open game & world folders",
                        "Manage saves, resource packs, mods and screenshots in Android Files",
                        SettingControl::Button("Open Files", Action::GameFiles, false),
                    ),
                    (
                        "Screenshots directory",
                        "Open the screenshots folder in Android Files",
                        SettingControl::Button("Screenshots", Action::ScreenshotFiles, false),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        &mut self.anim,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Theme => {
                use horizon_client::ui_theme::{
                    BackgroundMode, GradientDirection, THEME_ACCENT_SWATCHES,
                    THEME_BG_SWATCHES_DARK, THEME_BG_SWATCHES_LIGHT, ThemeAppearance, ThemePreset,
                    color,
                };
                let theme_cfg = self.catalog.config.theme.clone();
                let resolved = theme_cfg.resolve();
                let total_h = if theme_cfg.background_mode == BackgroundMode::Gradient {
                    812.0
                } else {
                    702.0
                };
                let extra_ime = if ime_top_dp.is_some() { 200.0 } else { 0.0 };
                let max_scroll = (total_h + extra_ime - (list_bottom - list_top)).max(0.0);
                if let Some(ime_y) = ime_top_dp {
                    let target_row_y = match self.catalog.editing {
                        Some("theme_accent") => Some(list_top + 438.0),
                        Some("theme_bg" | "theme_surface") => Some(list_top + 522.0),
                        Some("theme_grad_start" | "theme_grad_mid" | "theme_grad_end") => {
                            Some(list_top + 650.0)
                        }
                        _ => None,
                    };
                    if let Some(raw_y) = target_row_y {
                        let vis_y = raw_y - self.catalog.scroll;
                        if vis_y + 56.0 > ime_y - 12.0 {
                            self.catalog.scroll =
                                (raw_y + 56.0 - (ime_y - 16.0)).clamp(0.0, max_scroll);
                        }
                    }
                }
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                let mut cy = list_top - self.catalog.scroll;

                // 1. Live Interactive Theme Preview Card (with smooth pulse on change)
                let pulse = 0.985 + 0.015 * self.anim.theme_pulse;
                let prev_h = 96.0;
                let pw = content_w * pulse;
                let px = content_x + (content_w - pw) * 0.5;
                if cy + prev_h >= list_top && cy <= list_bottom {
                    ui.rounded(px, cy, pw, prev_h, 16.0, resolved.surface);
                    ui.clipped([px + 2.0, cy + 2.0, pw - 4.0, prev_h - 4.0], |ui| {
                        ui.draw_theme_background(
                            px + 2.0,
                            cy + 2.0,
                            pw - 4.0,
                            prev_h - 4.0,
                            &resolved,
                        );
                    });
                    ui.stroke_rounded(px, cy, pw, prev_h, 16.0, 1.5, resolved.accent);

                    // Mini surface card inside Live Preview
                    let inner_w = (pw - 28.0).max(160.0);
                    let inner_x = px + 14.0;
                    let inner_y = cy + 12.0;
                    ui.rounded(inner_x, inner_y, inner_w, 72.0, 12.0, resolved.surface);
                    ui.border(inner_x, inner_y, inner_w, 72.0, 12.0, resolved.border);
                    ui.rounded(
                        inner_x + 12.0,
                        inner_y + 12.0,
                        42.0,
                        20.0,
                        6.0,
                        resolved.accent_soft,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        9.5,
                        [inner_x + 12.0, inner_y + 12.0, 42.0, 20.0],
                        resolved.accent,
                        "LIVE",
                    );
                    let p_title = format!(
                        "{} • {} • {}",
                        theme_cfg.preset.label(),
                        if resolved.is_light { "Light" } else { "Dark" },
                        theme_cfg.background_mode.label()
                    );
                    let fit_pt =
                        ui.fit_text(Face::SemiBold, 13.5, &p_title, (inner_w - 190.0).max(80.0));
                    ui.text(
                        Face::SemiBold,
                        13.5,
                        inner_x + 62.0,
                        inner_y + 11.0,
                        resolved.text_primary,
                        &fit_pt,
                    );
                    let p_sub = format!(
                        "Accent {}  •  Background {}",
                        theme_cfg.custom_accent_hex,
                        if resolved.is_gradient {
                            format!(
                                "{} → {}",
                                theme_cfg.gradient_start_hex, theme_cfg.gradient_end_hex
                            )
                        } else {
                            theme_cfg.custom_bg_hex.clone()
                        }
                    );
                    let fit_ps =
                        ui.fit_text(Face::Regular, 11.0, &p_sub, (inner_w - 190.0).max(80.0));
                    ui.text(
                        Face::Regular,
                        11.0,
                        inner_x + 12.0,
                        inner_y + 42.0,
                        resolved.text_muted,
                        &fit_ps,
                    );

                    // Reset to Default button inside Live Preview card
                    let reset_r = [inner_x + inner_w - 118.0, inner_y + 18.0, 106.0, 36.0];
                    button(
                        ui,
                        &mut self.targets,
                        reset_r,
                        "Reset Default",
                        Action::ResetTheme,
                        false,
                    );
                    self.list_targets.push((reset_r, Action::ResetTheme));
                }
                cy += prev_h + 14.0;

                // 2. Appearance Mode (Dark vs Light) & Background Mode (Solid, Gradient, Horizon, Nightly, Custom GIF)
                if cy + 44.0 >= list_top && cy <= list_bottom {
                    let half = (content_w - 10.0) * 0.5;
                    let quarter = (half - 6.0) * 0.5;
                    for (i, (lbl, mode, active)) in [
                        ("Dark", ThemeAppearance::Dark, !resolved.is_light),
                        ("Light", ThemeAppearance::Light, resolved.is_light),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let r = [content_x + i as f32 * (quarter + 6.0), cy, quarter, 38.0];
                        button(
                            ui,
                            &mut self.targets,
                            r,
                            lbl,
                            Action::SetThemeAppearance(mode),
                            active,
                        );
                        self.list_targets
                            .push((r, Action::SetThemeAppearance(mode)));
                    }
                }
                cy += 50.0;
                if cy + 44.0 >= list_top && cy <= list_bottom {
                    let fifth = (content_w - 24.0) / 5.0;
                    for (i, (lbl, bmode, active)) in [
                        (
                            "Solid BG",
                            BackgroundMode::Solid,
                            theme_cfg.background_mode == BackgroundMode::Solid,
                        ),
                        (
                            "Gradient BG",
                            BackgroundMode::Gradient,
                            theme_cfg.background_mode == BackgroundMode::Gradient,
                        ),
                        (
                            "Horizon BG",
                            BackgroundMode::Horizon,
                            theme_cfg.background_mode == BackgroundMode::Horizon,
                        ),
                        (
                            "Nightly BG",
                            BackgroundMode::Nightly,
                            theme_cfg.background_mode == BackgroundMode::Nightly,
                        ),
                        (
                            "Custom GIF",
                            BackgroundMode::Custom,
                            theme_cfg.background_mode == BackgroundMode::Custom,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let r = [content_x + i as f32 * (fifth + 6.0), cy, fifth, 38.0];
                        button(
                            ui,
                            &mut self.targets,
                            r,
                            lbl,
                            Action::SetThemeBackgroundMode(bmode),
                            active,
                        );
                        self.list_targets
                            .push((r, Action::SetThemeBackgroundMode(bmode)));
                    }
                }
                cy += 50.0;

                // 2b. Global UI Opacity Control
                if cy + 62.0 >= list_top && cy <= list_bottom {
                    let op_val = theme_cfg.ui_opacity.clamp(0.10, 1.0);
                    let op_frac = ((op_val - 0.10) / 0.90).clamp(0.0, 1.0);
                    let op_str = format!("{:.0}%", op_val * 100.0);
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        &mut self.anim,
                        content_x,
                        cy,
                        content_w,
                        58.0,
                        "UI Opacity",
                        "Adjust transparency across the entire UI interface layout",
                        SettingControl::Slider {
                            value_label: &op_str,
                            fraction: op_frac,
                            minus_action: Action::UiOpacityDelta(-1),
                            plus_action: Action::UiOpacityDelta(1),
                        },
                    );
                }
                cy += 68.0;

                // 3. Preset Themes Grid (8 Presets: 2 rows x 4 columns on wide, or 4 rows x 2 columns on compact)
                ui.text(
                    Face::SemiBold,
                    12.5,
                    content_x,
                    cy,
                    my3_accent(),
                    "PRESET THEMES",
                );
                cy += 22.0;
                let p_cols = if compact { 2 } else { 4 };
                let p_gap = 8.0;
                let p_w = (content_w - p_gap * (p_cols as f32 - 1.0)) / p_cols as f32;
                let p_h = 54.0;
                let p_rows = ThemePreset::ALL.len().div_ceil(p_cols);
                for (idx, preset) in ThemePreset::ALL.into_iter().enumerate() {
                    let col = idx % p_cols;
                    let row = idx / p_cols;
                    let rx = content_x + col as f32 * (p_w + p_gap);
                    let ry = cy + row as f32 * (p_h + p_gap);
                    let r = [rx, ry, p_w, p_h];
                    let active = theme_cfg.preset == preset;
                    self.list_targets
                        .push((r, Action::SelectThemePreset(preset)));
                    if ry + p_h >= list_top && ry <= list_bottom {
                        ui.rounded(
                            rx,
                            ry,
                            p_w,
                            p_h,
                            12.0,
                            if active {
                                my3_nav_active()
                            } else {
                                my3_card_bg()
                            },
                        );
                        ui.stroke_rounded(
                            rx,
                            ry,
                            p_w,
                            p_h,
                            12.0,
                            if active { 1.8 } else { 1.0 },
                            if active {
                                my3_accent()
                            } else {
                                my3_card_border()
                            },
                        );
                        let (p_bg, _, p_acc, _, _, _, _) = preset.palette_rgb();
                        ui.rounded(rx + 10.0, ry + 11.0, 16.0, 16.0, 8.0, color::hex(p_bg, 255));
                        ui.stroke_rounded(
                            rx + 10.0,
                            ry + 11.0,
                            16.0,
                            16.0,
                            8.0,
                            1.2,
                            color::hex(p_acc, 255),
                        );
                        ui.rounded(
                            rx + 20.0,
                            ry + 15.0,
                            12.0,
                            12.0,
                            6.0,
                            color::hex(p_acc, 255),
                        );
                        let fit_lbl = ui.fit_text(Face::SemiBold, 11.8, preset.label(), p_w - 42.0);
                        ui.text(
                            Face::SemiBold,
                            11.8,
                            rx + 36.0,
                            ry + 8.0,
                            my3_text(),
                            &fit_lbl,
                        );
                        let fit_sub =
                            ui.fit_text(Face::Regular, 9.5, preset.subtitle(), p_w - 20.0);
                        ui.text(
                            Face::Regular,
                            9.5,
                            rx + 10.0,
                            ry + 31.0,
                            my3_muted(),
                            &fit_sub,
                        );
                        if let Some(c) = ui.clip_rect(r) {
                            self.targets.push((c, Action::SelectThemePreset(preset)));
                        }
                    }
                }
                cy += p_rows as f32 * (p_h + p_gap) + 8.0;

                // 4. Custom Accent Color Swatches + Hex Input
                ui.text(
                    Face::SemiBold,
                    12.5,
                    content_x,
                    cy,
                    my3_accent(),
                    "CUSTOM ACCENT COLOR",
                );
                cy += 22.0;
                if cy + 52.0 >= list_top && cy <= list_bottom {
                    ui.rounded(content_x, cy, content_w, 50.0, 12.0, my3_card_bg());
                    ui.border(content_x, cy, content_w, 50.0, 12.0, my3_card_border());
                    let sw_size = 30.0;
                    let sw_gap = 8.0;
                    for (i, (rgb, hex_str, _name)) in THEME_ACCENT_SWATCHES.into_iter().enumerate()
                    {
                        let sx = content_x + 12.0 + i as f32 * (sw_size + sw_gap);
                        if sx + sw_size > content_x + content_w - 134.0 {
                            break;
                        }
                        let sr = [sx, cy + 10.0, sw_size, sw_size];
                        let is_sel = theme_cfg.custom_accent_hex.eq_ignore_ascii_case(hex_str);
                        ui.rounded(sr[0], sr[1], sr[2], sr[3], 15.0, color::hex(rgb, 255));
                        if is_sel {
                            ui.stroke_rounded(
                                sr[0] - 2.0,
                                sr[1] - 2.0,
                                sr[2] + 4.0,
                                sr[3] + 4.0,
                                17.0,
                                2.0,
                                my3_text(),
                            );
                        }
                        if let Some(c) = ui.clip_rect(sr) {
                            self.targets.push((c, Action::SelectAccentSwatch(rgb)));
                            self.list_targets.push((c, Action::SelectAccentSwatch(rgb)));
                        }
                    }
                    let hex_btn_r = ui.text_field(
                        content_x + content_w - 126.0,
                        cy + 5.0,
                        114.0,
                        40.0,
                        "ACCENT HEX",
                        &theme_cfg.custom_accent_hex,
                        "#9A7BFF",
                        self.catalog.editing == Some("theme_accent"),
                    );
                    if let Some(c) = ui.clip_rect(hex_btn_r) {
                        self.targets.push((c, Action::Field("theme_accent")));
                    }
                    self.list_targets
                        .push((hex_btn_r, Action::Field("theme_accent")));
                }
                cy += 62.0;

                // 5. Custom Background & Surface Colors
                ui.text(
                    Face::SemiBold,
                    12.5,
                    content_x,
                    cy,
                    my3_accent(),
                    "CUSTOM SOLID BACKGROUND & SURFACE COLORS",
                );
                cy += 22.0;
                if cy + 52.0 >= list_top && cy <= list_bottom {
                    ui.rounded(content_x, cy, content_w, 50.0, 12.0, my3_card_bg());
                    ui.border(content_x, cy, content_w, 50.0, 12.0, my3_card_border());
                    let bg_swatches = if resolved.is_light {
                        THEME_BG_SWATCHES_LIGHT
                    } else {
                        THEME_BG_SWATCHES_DARK
                    };
                    let sw_size = 30.0;
                    let sw_gap = 8.0;
                    for (i, (rgb, hex_str, _name)) in bg_swatches.into_iter().enumerate() {
                        let sx = content_x + 12.0 + i as f32 * (sw_size + sw_gap);
                        if sx + sw_size > content_x + content_w - 238.0 {
                            break;
                        }
                        let sr = [sx, cy + 10.0, sw_size, sw_size];
                        let is_sel = theme_cfg.custom_bg_hex.eq_ignore_ascii_case(hex_str);
                        ui.rounded(sr[0], sr[1], sr[2], sr[3], 15.0, color::hex(rgb, 255));
                        ui.stroke_rounded(
                            sr[0],
                            sr[1],
                            sr[2],
                            sr[3],
                            15.0,
                            if is_sel { 2.0 } else { 1.0 },
                            if is_sel { my3_accent() } else { my3_border() },
                        );
                        if let Some(c) = ui.clip_rect(sr) {
                            self.targets.push((c, Action::SelectBgSwatch(rgb)));
                            self.list_targets.push((c, Action::SelectBgSwatch(rgb)));
                        }
                    }
                    let bg_hex_r = ui.text_field(
                        content_x + content_w - 232.0,
                        cy + 5.0,
                        108.0,
                        40.0,
                        "BG HEX",
                        &theme_cfg.custom_bg_hex,
                        "#13111C",
                        self.catalog.editing == Some("theme_bg"),
                    );
                    if let Some(c) = ui.clip_rect(bg_hex_r) {
                        self.targets.push((c, Action::Field("theme_bg")));
                    }
                    self.list_targets
                        .push((bg_hex_r, Action::Field("theme_bg")));

                    let surf_hex_r = ui.text_field(
                        content_x + content_w - 118.0,
                        cy + 5.0,
                        108.0,
                        40.0,
                        "CARD HEX",
                        &theme_cfg.custom_surface_hex,
                        "#1D192B",
                        self.catalog.editing == Some("theme_surface"),
                    );
                    if let Some(c) = ui.clip_rect(surf_hex_r) {
                        self.targets.push((c, Action::Field("theme_surface")));
                    }
                    self.list_targets
                        .push((surf_hex_r, Action::Field("theme_surface")));
                }
                cy += 62.0;

                // 6. Custom Gradient Direction & Color Stops
                ui.text(
                    Face::SemiBold,
                    12.5,
                    content_x,
                    cy,
                    my3_accent(),
                    "CUSTOM GRADIENT DIRECTION & COLOR STOPS",
                );
                cy += 22.0;
                if cy + 44.0 >= list_top && cy <= list_bottom {
                    let dir_w = (content_w - 18.0) * 0.25;
                    for (i, dir) in GradientDirection::ALL.into_iter().enumerate() {
                        let r = [content_x + i as f32 * (dir_w + 6.0), cy, dir_w, 36.0];
                        button(
                            ui,
                            &mut self.targets,
                            r,
                            dir.label(),
                            Action::SetGradientDirection(dir),
                            theme_cfg.gradient_direction == dir,
                        );
                        self.list_targets
                            .push((r, Action::SetGradientDirection(dir)));
                    }
                }
                cy += 44.0;

                if cy + 54.0 >= list_top && cy <= list_bottom {
                    ui.rounded(content_x, cy, content_w, 50.0, 12.0, my3_card_bg());
                    ui.border(content_x, cy, content_w, 50.0, 12.0, my3_card_border());
                    let stop_w = ((content_w - 36.0) / 3.0).max(80.0);
                    for (i, (lbl, hex_val, field_name)) in [
                        (
                            "Stop 1",
                            theme_cfg.gradient_start_hex.as_str(),
                            "theme_grad_start",
                        ),
                        (
                            if theme_cfg.use_three_stops {
                                "Mid Stop"
                            } else {
                                "Mid (Off)"
                            },
                            theme_cfg.gradient_mid_hex.as_str(),
                            "theme_grad_mid",
                        ),
                        (
                            "End Stop",
                            theme_cfg.gradient_end_hex.as_str(),
                            "theme_grad_end",
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let r = ui.text_field(
                            content_x + 8.0 + i as f32 * (stop_w + 8.0),
                            cy + 5.0,
                            stop_w,
                            40.0,
                            lbl,
                            hex_val,
                            "#1A1333",
                            self.catalog.editing == Some(field_name),
                        );
                        if let Some(c) = ui.clip_rect(r) {
                            self.targets.push((c, Action::Field(field_name)));
                        }
                        self.list_targets.push((r, Action::Field(field_name)));
                    }
                }
                cy += 58.0;

                if cy + 44.0 >= list_top && cy <= list_bottom {
                    let half = (content_w - 8.0) * 0.5;
                    let three_r = [content_x, cy, half, 38.0];
                    button(
                        ui,
                        &mut self.targets,
                        three_r,
                        if theme_cfg.use_three_stops {
                            "3-Stop Gradient: Enabled"
                        } else {
                            "2-Stop Gradient (Tap for 3)"
                        },
                        Action::ToggleGradientThreeStops,
                        theme_cfg.use_three_stops,
                    );
                    self.list_targets
                        .push((three_r, Action::ToggleGradientThreeStops));

                    let mid_lbl = format!(
                        "Mid Stop Position: {:.0}%",
                        theme_cfg.gradient_mid_pos * 100.0
                    );
                    let mid_r = [content_x + half + 8.0, cy, half, 38.0];
                    button(
                        ui,
                        &mut self.targets,
                        mid_r,
                        &mid_lbl,
                        Action::StepGradientMidPos(1),
                        false,
                    );
                    self.list_targets
                        .push((mid_r, Action::StepGradientMidPos(1)));
                }

                ui.set_clip(None);
            }
            SettingsCategory::Controls => {
                let studio_offset = 196.0;
                let zalith_studio_available = crate::native_ime::zalith_studio_available();
                let count = 26 + usize::from(zalith_studio_available);
                let max_scroll =
                    (studio_offset + count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let style = touch_cfg.control_style;
                let is_mcpe = style == horizon_client::touch::ControlStyle::Mcpe;
                let is_goy = style == horizon_client::touch::ControlStyle::GoyControl;
                let sys_name = style.label();
                let sys_presets = horizon_client::touch::Settings::list_presets(&game_home, style);
                let preset_label = format!(
                    "{} ({} presets) v",
                    touch_cfg.preset_name,
                    sys_presets.len()
                );
                let preset_sub = format!(
                    "{}  •  controls/{}/{}.json{}",
                    touch_cfg.preset_description,
                    style.storage_subdir(),
                    touch_cfg.preset_id,
                    if touch_cfg.is_default_preset() {
                        " (Protected Default)"
                    } else {
                        ""
                    }
                );
                let editor_title = format!("{} Control Editor", sys_name);
                let editor_sub = match style {
                    horizon_client::touch::ControlStyle::Mcpe => {
                        "Customize MCPE controls with exact Minecraft sprites, size, opacity & alignment"
                    }
                    horizon_client::touch::ControlStyle::GoyControl => {
                        "Customize GoyControl buttons, custom keys, toggle, swipe & pass-through"
                    }
                };
                let preset_row_title = format!("{} Preset Selector", sys_name);
                let copy_title = format!("Copy {} Preset ('{}')", sys_name, touch_cfg.preset_name);
                let copy_btn_label = if self.catalog.editing == Some("profile_name")
                    && self.catalog.config.preset_edit_mode == "copy"
                {
                    "Typing Name..."
                } else if self.catalog.editing == Some("profile_desc")
                    && self.catalog.config.preset_edit_mode == "copy"
                {
                    "Typing Desc..."
                } else {
                    "Copy Preset"
                };
                let edit_meta_title = format!("Edit {} Preset Name & Description", sys_name);
                let edit_meta_sub = if touch_cfg.is_default_preset() {
                    "Default layout is protected and cannot be renamed or overwritten; use Copy Preset"
                } else {
                    "Pencil / Edit: rename or update description of the selected user preset"
                };
                let edit_meta_btn = if touch_cfg.is_default_preset() {
                    "Protected"
                } else if self.catalog.editing == Some("profile_name")
                    && self.catalog.config.preset_edit_mode == "edit"
                {
                    "Typing Name..."
                } else if self.catalog.editing == Some("profile_desc")
                    && self.catalog.config.preset_edit_mode == "edit"
                {
                    "Typing Desc..."
                } else {
                    "Edit (Pencil)"
                };

                let mouse_sens = format!("{:.0}%", opts.sensitivity * 200.0);
                let touch_sens = format!("{:.0}%", touch_cfg.sensitivity * 100.0);
                let btn_scale = format!("{:.0}%", touch_cfg.button_scale * 100.0);
                let ptr_size = format!("{:.0}%", touch_cfg.virtual_mouse_pointer_scale * 100.0);
                let gyro_sens = format!("{:.0}%", touch_cfg.gyro_sensitivity * 100.0);
                let touch_sens_frac = ((touch_cfg.sensitivity - 0.20) / 2.30).clamp(0.0, 1.0);
                let btn_scale_frac = ((touch_cfg.button_scale - 0.50) / 1.50).clamp(0.0, 1.0);
                let ptr_size_frac =
                    ((touch_cfg.virtual_mouse_pointer_scale - 0.50) / 1.50).clamp(0.0, 1.0);
                let gyro_sens_frac = ((touch_cfg.gyro_sensitivity - 0.20) / 2.80).clamp(0.0, 1.0);
                let mouse_sens_frac = opts.sensitivity.clamp(0.0, 1.0);
                let scheme_str = touch_cfg.mode.label();
                let zone_str = touch_cfg.joystick_style.move_area.label();
                let invert_y_active = opts.invert_y || touch_cfg.invert_y;
                let sprint_latch_active = opts.toggle_sprint || touch_cfg.sprint_toggle;

                let mut rows: Vec<(&str, &str, SettingControl<'_>)> = vec![
                    (
                        "LAYOUT STUDIO, IMPORT & EXPORT PRESETS",
                        "Material 3 HUD editor, automatic MCPE/GoyControl file importer & location picker export",
                        SettingControl::SectionHeader(sys_name, Icon::MenuEquals),
                    ),
                    (
                        &editor_title,
                        editor_sub,
                        SettingControl::Button(
                            "Customize Controls",
                            Action::OpenControlEditor,
                            true,
                        ),
                    ),
                    (
                        &preset_row_title,
                        &preset_sub,
                        SettingControl::Button(&preset_label, Action::ControlProfileChooser, false),
                    ),
                    (
                        "Import Custom Control from File Manager",
                        "Pick a control .json file (Pojav, Zalith, FCL, MCPE, or GoyControl); auto-detects and activates",
                        SettingControl::Button("Import Custom Control", Action::ImportControlProfile, true),
                    ),
                    (
                        "Export Active Control Preset",
                        "Open system File Manager to choose where to save your exported control .json",
                        SettingControl::Button("Export Controls", Action::ExportControlProfile, false),
                    ),
                    (
                        &copy_title,
                        "Duplicate selected layout into a new independent preset (asks to replace if same name)",
                        if self.catalog.editing == Some("profile_name") {
                            SettingControl::TextField {
                                label: "PRESET NAME",
                                value: &self.catalog.config.custom_profile_name,
                                hint: "Enter preset name",
                                active: true,
                                action: Action::Field("profile_name"),
                            }
                        } else {
                            SettingControl::Button(copy_btn_label, Action::CopyActivePreset, false)
                        },
                    ),
                    (
                        &edit_meta_title,
                        edit_meta_sub,
                        if self.catalog.editing == Some("profile_desc") {
                            SettingControl::TextField {
                                label: "DESCRIPTION",
                                value: &self.catalog.config.custom_profile_desc,
                                hint: "Enter preset description",
                                active: true,
                                action: Action::Field("profile_desc"),
                            }
                        } else {
                            SettingControl::Button(
                                edit_meta_btn,
                                Action::EditActivePresetMeta,
                                false,
                            )
                        },
                    ),
                    (
                        "CAMERA LOOK & AIMING SENSITIVITY",
                        "Touch swipe responsiveness, 3D look multiplier, Y-axis inversion & crosshair mode",
                        SettingControl::SectionHeader("CAMERA", Icon::Shaders),
                    ),
                    (
                        "Sensitivity (Touch Look Speed)",
                        "Direct finger drag camera rotation speed multiplier",
                        SettingControl::Slider {
                            value_label: &touch_sens,
                            fraction: touch_sens_frac,
                            minus_action: Action::TouchSensDelta(-0.10),
                            plus_action: Action::TouchSensDelta(0.10),
                        },
                    ),
                    (
                        "3D Camera Base Sensitivity",
                        "Base in-game 3D look sensitivity stored in options.txt",
                        SettingControl::Slider {
                            value_label: &mouse_sens,
                            fraction: mouse_sens_frac,
                            minus_action: Action::MouseSensDelta(-0.05),
                            plus_action: Action::MouseSensDelta(0.05),
                        },
                    ),
                    (
                        "Invert Y-Axis",
                        "Inverts vertical camera look direction for touch and mouse",
                        SettingControl::Toggle(invert_y_active, Action::ToggleInvertY),
                    ),
                    (
                        "Split Touch (Aim & Look)",
                        "Separates aiming and camera look with center crosshair",
                        SettingControl::Toggle(touch_cfg.crosshair(), Action::ToggleSplitControls),
                    ),
                    (
                        "MOVEMENT, HUD SCALE & ERGONOMICS",
                        "On-screen button scale, thumbstick vs D-Pad, active touch zone & haptics",
                        SettingControl::SectionHeader("ERGONOMICS", Icon::Settings),
                    ),
                    (
                        "HUD Button Scale",
                        "Global size multiplier for on-screen touch buttons",
                        SettingControl::Slider {
                            value_label: &btn_scale,
                            fraction: btn_scale_frac,
                            minus_action: Action::ButtonScaleDelta(-0.10),
                            plus_action: Action::ButtonScaleDelta(0.10),
                        },
                    ),
                    (
                        "Movement Control Scheme",
                        "Switch between classic D-Pad buttons and analog thumb joystick",
                        SettingControl::Button(scheme_str, Action::ToggleControlScheme, false),
                    ),
                    (
                        "Joystick Active Touch Zone",
                        "Choose Follow Finger, Mid-Bottom Left, Right Side, or Fixed origin",
                        SettingControl::Button(zone_str, Action::CycleJoystickZone, false),
                    ),
                    (
                        "Left-Handed HUD Mirroring",
                        "Mirrors movement controls to the right side of the screen",
                        SettingControl::Toggle(touch_cfg.left_handed, Action::ToggleLeftHanded),
                    ),
                    (
                        "Toggle Sprint Mode",
                        "Latch sprint on a single tap instead of holding the sprint button",
                        SettingControl::Toggle(sprint_latch_active, Action::ToggleSprintLatch),
                    ),
                    (
                        "Inventory Walk Controls",
                        "Keep movement controls active while inventory or chest screens are open",
                        SettingControl::Toggle(
                            touch_cfg.inventory_controls,
                            Action::ToggleInventoryControls,
                        ),
                    ),
                    (
                        "Auto-Jump",
                        "Automatically jump when walking up 1-block terrain steps",
                        SettingControl::Toggle(opts.auto_jump, Action::ToggleAutoJump),
                    ),
                    (
                        "Haptic Touch Vibration",
                        "Tactile vibration feedback when pressing HUD buttons and breaking blocks",
                        SettingControl::Toggle(touch_cfg.vibrate, Action::ToggleTouchVibrate),
                    ),
                    (
                        "VIRTUAL MOUSE & GYROSCOPE MOTION",
                        "Windows 11 menu cursor scaling and motion-assisted gyroscope aiming",
                        SettingControl::SectionHeader("POINTER & GYRO", Icon::Play),
                    ),
                    (
                        "Authoritative Virtual Mouse",
                        "Enable virtual mouse pointer in menus and on-demand in GoyControl",
                        SettingControl::Toggle(
                            touch_cfg.virtual_mouse_enabled,
                            Action::ToggleVirtualMouseEnabled,
                        ),
                    ),
                    (
                        "Show V+ Overlay Button",
                        "Display the floating V+ Control Hub button in menus and when controls are hidden",
                        SettingControl::Toggle(
                            !touch_cfg.overlay_hidden,
                            Action::ToggleOverlayButton,
                        ),
                    ),
                    (
                        "Virtual Cursor Scale",
                        "Adjust size of the Windows 11 virtual mouse arrow in menus",
                        SettingControl::Slider {
                            value_label: &ptr_size,
                            fraction: ptr_size_frac,
                            minus_action: Action::PointerSizeDelta(-0.10),
                            plus_action: Action::PointerSizeDelta(0.10),
                        },
                    ),
                    (
                        "Gyroscope Aiming",
                        "Tilt your phone for fine bow and crosshair aiming adjustments",
                        SettingControl::Toggle(touch_cfg.gyroscope, Action::ToggleGyro),
                    ),
                    (
                        "Gyroscope Sensitivity",
                        "Fine-tune motion tilt speed when Gyroscope Aiming is enabled",
                        SettingControl::Slider {
                            value_label: &gyro_sens,
                            fraction: gyro_sens_frac,
                            minus_action: Action::GyroSensDelta(-0.10),
                            plus_action: Action::GyroSensDelta(0.10),
                        },
                    ),
                ];

                if zalith_studio_available {
                    rows.insert(4, (
                        "Zalith Control Studio (Experimental)",
                        "Choose a saved Zalith layout to edit; gameplay integration is not available yet",
                        SettingControl::Button("Open Studio", Action::OpenExperimentalZalithStudio, false),
                    ));
                }

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));

                // M3 Expressive Control Engine Hero Selector Cards
                let picker_y = list_top - self.catalog.scroll;
                if picker_y + 66.0 >= list_top && picker_y <= list_bottom {
                    let half_w = (content_w - 10.0) * 0.5;
                    for (i, (title, sub, badge, selected, action)) in [
                        (
                            "MCPE (Default)",
                            "Vanilla touch sprites, D-Pad & thumbstick",
                            "CLASSIC HUD",
                            is_mcpe,
                            Action::SelectControlMode(horizon_client::touch::ControlStyle::Mcpe),
                        ),
                        (
                            "Java like controlls",
                            "Custom multi-bind HUD, macros & pass-through",
                            "PRO OVERLAY",
                            is_goy,
                            Action::SelectControlMode(
                                horizon_client::touch::ControlStyle::GoyControl,
                            ),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = content_x + i as f32 * (half_w + 10.0);
                        let rect = [rx, picker_y, half_w, 66.0];
                        if let Some(c) = ui.clip_rect(rect) {
                            self.targets.push((c, action));
                        }
                        self.list_targets.push((rect, action));
                        ui.rounded(
                            rx,
                            picker_y,
                            half_w,
                            66.0,
                            18.0,
                            if selected {
                                my3_surface()
                            } else {
                                my3_card_bg()
                            },
                        );
                        ui.border(
                            rx,
                            picker_y,
                            half_w,
                            66.0,
                            18.0,
                            if selected {
                                my3_accent()
                            } else {
                                my3_card_border()
                            },
                        );
                        if selected {
                            ui.rounded(rx + 4.0, picker_y + 12.0, 4.0, 42.0, 2.0, my3_accent());
                        }
                        let rad_x = rx + 16.0;
                        let rad_y = picker_y + 23.0;
                        ui.stroke_rounded(
                            rad_x,
                            rad_y,
                            20.0,
                            20.0,
                            10.0,
                            1.8,
                            if selected { my3_accent() } else { my3_muted() },
                        );
                        if selected {
                            ui.rounded(rad_x + 5.0, rad_y + 5.0, 10.0, 10.0, 5.0, my3_accent());
                        }
                        let chip_w = 78.0;
                        let chip_x = rx + half_w - chip_w - 10.0;
                        ui.rounded(
                            chip_x,
                            picker_y + 10.0,
                            chip_w,
                            20.0,
                            10.0,
                            if selected {
                                my3_accent_soft()
                            } else {
                                my3_surface_high()
                            },
                        );
                        ui.centered_fit(
                            Face::SemiBold,
                            8.5,
                            [chip_x + 4.0, picker_y + 10.0, chip_w - 8.0, 20.0],
                            if selected { my3_accent() } else { my3_muted() },
                            if selected { "ACTIVE" } else { badge },
                        );
                        let text_x = rx + 44.0;
                        let max_w = (half_w - 134.0).max(56.0);
                        let fitted = ui.fit_text(Face::SemiBold, 14.5, title, max_w);
                        ui.text(
                            Face::SemiBold,
                            14.5,
                            text_x,
                            picker_y + 11.0,
                            my3_text(),
                            &fitted,
                        );
                        let sub_max_w = (half_w - 54.0).max(60.0);
                        let fitted_sub = ui.fit_text(Face::Regular, 11.5, sub, sub_max_w);
                        ui.text(
                            Face::Regular,
                            11.5,
                            text_x,
                            picker_y + 36.0,
                            my3_muted(),
                            &fitted_sub,
                        );
                    }
                }

                // Material Design 3 Control Studio Workbench Card (Two-tier: Full-width Header + 3-Button Action Bar)
                let wb_y = list_top + 74.0 - self.catalog.scroll;
                let wb_h = 114.0;
                if wb_y + wb_h >= list_top && wb_y <= list_bottom {
                    ui.rounded(content_x, wb_y, content_w, wb_h, 20.0, my3_surface());
                    ui.border(content_x, wb_y, content_w, wb_h, 20.0, my3_accent());
                    ui.rounded(
                        content_x + 14.0,
                        wb_y + 12.0,
                        42.0,
                        42.0,
                        13.0,
                        my3_accent_soft(),
                    );
                    ui.icon(
                        Icon::Gamepad,
                        content_x + 25.0,
                        wb_y + 23.0,
                        20.0,
                        my3_accent(),
                    );
                    let studio_hdr = format!(
                        "{} Studio  •  Active Preset: {}",
                        sys_name, touch_cfg.preset_name
                    );
                    let hdr_max = (content_w - 82.0).max(120.0);
                    let fitted_hdr = ui.fit_text(Face::SemiBold, 14.0, &studio_hdr, hdr_max);
                    ui.text(
                        Face::SemiBold,
                        14.0,
                        content_x + 66.0,
                        wb_y + 10.0,
                        my3_text(),
                        &fitted_hdr,
                    );
                    let path_tag = format!(
                        "Storage: {}/{}.json  •  {} buttons",
                        style.storage_subdir(),
                        touch_cfg.preset_id,
                        if is_goy {
                            touch_cfg.custom_buttons.len()
                        } else {
                            14 + touch_cfg.custom_buttons.len()
                        }
                    );
                    let fitted_path = ui.fit_text(Face::Regular, 11.2, &path_tag, hdr_max);
                    ui.text(
                        Face::Regular,
                        11.2,
                        content_x + 66.0,
                        wb_y + 30.0,
                        my3_muted(),
                        &fitted_path,
                    );
                    let fitted_desc =
                        ui.fit_text(Face::Regular, 10.8, &touch_cfg.preset_description, hdr_max);
                    ui.text(
                        Face::Regular,
                        10.8,
                        content_x + 66.0,
                        wb_y + 47.0,
                        my3_accent(),
                        &fitted_desc,
                    );

                    // Studio Quick Action Buttons along the bottom row of the Workbench Card
                    let btn_gap = 8.0;
                    let btn_w_sm = ((content_w - 28.0 - 2.0 * btn_gap) / 3.0).max(76.0);
                    let btn_y_wb = wb_y + 68.0;
                    let r_editor = [content_x + 14.0, btn_y_wb, btn_w_sm, 36.0];
                    let r_import = [r_editor[0] + btn_w_sm + btn_gap, btn_y_wb, btn_w_sm, 36.0];
                    let r_export = [r_import[0] + btn_w_sm + btn_gap, btn_y_wb, btn_w_sm, 36.0];
                    for (rect, lbl, act, primary) in [
                        (r_editor, "Open Editor", Action::OpenControlEditor, true),
                        (r_import, "Import JSON", Action::ImportControlProfile, false),
                        (r_export, "Export Save", Action::ExportControlProfile, false),
                    ] {
                        button(ui, &mut self.targets, rect, lbl, act, primary);
                        self.list_targets.push((rect, act));
                    }
                }

                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + studio_offset + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        &mut self.anim,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Mouse => {
                use horizon_client::touch::CursorIcon;

                let preview_h = 84.0;
                let rows_start = preview_h + 14.0;
                let slider_count = 4;
                let picker_hdr_y = rows_start + slider_count as f32 * step_y + 8.0;
                let cols = if compact { 2 } else { 3 };
                let picker_rows = CursorIcon::ALL.len().div_ceil(cols);
                let picker_card_h = 66.0;
                let picker_gap = 10.0;
                let total_h = picker_hdr_y
                    + 28.0
                    + picker_rows as f32 * (picker_card_h + picker_gap)
                    + 12.0;
                let max_scroll = (total_h - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));

                let py = list_top - self.catalog.scroll;
                if py + preview_h >= list_top && py <= list_bottom {
                    ui.rounded(content_x, py, content_w, preview_h, 18.0, my3_surface());
                    ui.border(content_x, py, content_w, preview_h, 18.0, my3_accent());

                    let box_s = 60.0;
                    let box_x = content_x + 12.0;
                    let box_y = py + (preview_h - box_s) * 0.5;
                    ui.rounded(box_x, box_y, box_s, box_s, 14.0, my3_surface_high());
                    ui.border(box_x, box_y, box_s, box_s, 14.0, my3_card_border());
                    ui.cursor_preview(
                        box_x,
                        box_y,
                        box_s,
                        touch_cfg.virtual_mouse_cursor,
                        touch_cfg.virtual_mouse_pointer_scale,
                        touch_cfg.virtual_mouse_opacity,
                    );

                    let reset_w = 112.0;
                    let reset_r = [
                        content_x + content_w - reset_w - 14.0,
                        py + (preview_h - 36.0) * 0.5,
                        reset_w,
                        36.0,
                    ];
                    let text_x = box_x + box_s + 14.0;
                    let text_max = (reset_r[0] - text_x - 12.0).max(60.0);
                    let title_str = format!("Active Cursor: {}", touch_cfg.virtual_mouse_cursor.label());
                    let fitted_title = ui.fit_text(Face::SemiBold, 15.0, &title_str, text_max);
                    ui.text(Face::SemiBold, 15.0, text_x, py + 17.0, my3_text(), &fitted_title);

                    let stats = format!(
                        "Size {:.0}%  •  Opacity {:.0}%  •  Sensitivity {:.0}%",
                        touch_cfg.virtual_mouse_pointer_scale * 100.0,
                        touch_cfg.virtual_mouse_opacity * 100.0,
                        opts.sensitivity * 100.0,
                    );
                    let fitted_stats = ui.fit_text(Face::Regular, 12.0, &stats, text_max);
                    ui.text(Face::Regular, 12.0, text_x, py + 43.0, my3_muted(), &fitted_stats);

                    button(
                        ui,
                        &mut self.targets,
                        reset_r,
                        "Reset Default",
                        Action::ResetMouseCustomization,
                        false,
                    );
                    self.list_targets.push((reset_r, Action::ResetMouseCustomization));
                }

                let size_lbl = format!("{:.0}%", touch_cfg.virtual_mouse_pointer_scale * 100.0);
                let size_frac =
                    ((touch_cfg.virtual_mouse_pointer_scale - 0.50) / 1.50).clamp(0.0, 1.0);
                let op_lbl = format!("{:.0}%", touch_cfg.virtual_mouse_opacity * 100.0);
                let op_frac = ((touch_cfg.virtual_mouse_opacity - 0.15) / 0.85).clamp(0.0, 1.0);
                let sens_lbl = format!("{:.0}%", opts.sensitivity * 100.0);
                let sens_frac = ((opts.sensitivity - 0.05) / 0.95).clamp(0.0, 1.0);

                let rows: [(&str, &str, SettingControl<'_>); 4] = [
                    (
                        "Cursor Size",
                        "Scale of both the physical mouse pointer and virtual menu cursor (50% - 200%)",
                        SettingControl::Slider {
                            value_label: &size_lbl,
                            fraction: size_frac,
                            minus_action: Action::PointerSizeDelta(-0.10),
                            plus_action: Action::PointerSizeDelta(0.10),
                        },
                    ),
                    (
                        "Cursor Opacity",
                        "Transparency of the in-game & launcher custom cursor (15% - 100%)",
                        SettingControl::Slider {
                            value_label: &op_lbl,
                            fraction: op_frac,
                            minus_action: Action::PointerOpacityDelta(-0.05),
                            plus_action: Action::PointerOpacityDelta(0.05),
                        },
                    ),
                    (
                        "Mouse Look Sensitivity",
                        "Camera turn speed for physical mouse and trackpad input",
                        SettingControl::Slider {
                            value_label: &sens_lbl,
                            fraction: sens_frac,
                            minus_action: Action::MouseSensDelta(-0.05),
                            plus_action: Action::MouseSensDelta(0.05),
                        },
                    ),
                    (
                        "Virtual Mouse in Touch Menus",
                        "Show draggable on-screen cursor when using touch controls in menus",
                        SettingControl::Toggle(
                            touch_cfg.virtual_mouse_enabled,
                            Action::ToggleVirtualMouseEnabled,
                        ),
                    ),
                ];

                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + rows_start + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        &mut self.anim,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }

                let hdr_y = list_top + picker_hdr_y - self.catalog.scroll;
                if hdr_y + 24.0 >= list_top && hdr_y <= list_bottom {
                    ui.text(
                        Face::SemiBold,
                        12.5,
                        content_x + 4.0,
                        hdr_y + 4.0,
                        my3_accent(),
                        "CURSOR ICON",
                    );
                }

                let grid_y0 = list_top + picker_hdr_y + 28.0 - self.catalog.scroll;
                let gap_x = 12.0;
                let tile_w = (content_w - gap_x * (cols as f32 - 1.0)) / cols as f32;
                for (idx, &icon) in CursorIcon::ALL.iter().enumerate() {
                    let col = idx % cols;
                    let row = idx / cols;
                    let tx = content_x + col as f32 * (tile_w + gap_x);
                    let ty = grid_y0 + row as f32 * (picker_card_h + picker_gap);
                    if ty + picker_card_h < list_top || ty > list_bottom {
                        continue;
                    }
                    let selected = touch_cfg.virtual_mouse_cursor == icon;
                    let rect = [tx, ty, tile_w, picker_card_h];
                    ui.rounded(
                        tx,
                        ty,
                        tile_w,
                        picker_card_h,
                        16.0,
                        if selected { my3_accent_soft() } else { my3_card_bg() },
                    );
                    ui.border(
                        tx,
                        ty,
                        tile_w,
                        picker_card_h,
                        16.0,
                        if selected { my3_accent() } else { my3_card_border() },
                    );
                    let p_box = 44.0;
                    let p_x = tx + 11.0;
                    let p_y = ty + (picker_card_h - p_box) * 0.5;
                    ui.rounded(p_x, p_y, p_box, p_box, 11.0, my3_surface_high());
                    ui.cursor_preview(p_x, p_y, p_box, icon, 1.0, 1.0);

                    let lx = p_x + p_box + 10.0;
                    let lmax = (tile_w - p_box - 28.0).max(40.0);
                    let fitted_lbl = ui.fit_text(Face::SemiBold, 13.5, icon.label(), lmax);
                    ui.text(Face::SemiBold, 13.5, lx, ty + 14.0, my3_text(), &fitted_lbl);
                    let sub_lbl = if selected { "Active" } else { "Tap to equip" };
                    ui.text(
                        Face::Regular,
                        11.5,
                        lx,
                        ty + 36.0,
                        if selected { my3_accent() } else { my3_muted() },
                        sub_lbl,
                    );

                    if let Some(c) = ui.clip_rect(rect) {
                        self.targets.push((c, Action::SelectCursorIcon(icon)));
                    }
                    self.list_targets.push((rect, Action::SelectCursorIcon(icon)));
                }

                ui.set_clip(None);
            }
            SettingsCategory::Experimental => {
                let c = &self.catalog.config;
                let count = 9;
                let max_scroll = (count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let active_mode_badge = if c.super_perf_mode {
                    "Super Perf Active"
                } else if c.battery_perf_mode {
                    "Battery Saver Active"
                } else if c.max_fps_mode {
                    "260 FPS Uncapped"
                } else {
                    "Balanced Default"
                };

                let rows: [(&str, &str, SettingControl<'_>); 9] = [
                    (
                        "PERFORMANCE PROFILES & GPU PIPELINE",
                        "Hardware-tuned presets for Mali-G52 / Adreno GPUs and high-refresh displays",
                        SettingControl::SectionHeader(active_mode_badge, Icon::Flask),
                    ),
                    (
                        "Super Performance Mode",
                        "Aggressive chunk culling, fast clouds, minimal particles & Vulkan batching",
                        SettingControl::Toggle(c.super_perf_mode, Action::ToggleSuperPerf),
                    ),
                    (
                        "Battery Performance Mode",
                        "Caps framerate to 60 FPS with VSync and reduces background worker wakeups",
                        SettingControl::Toggle(c.battery_perf_mode, Action::ToggleBatteryPerf),
                    ),
                    (
                        "Max FPS Mode (Uncapped 260 FPS)",
                        "Disables VSync, unlocks framerate ceiling and requests maximum display Hz",
                        SettingControl::Toggle(c.max_fps_mode, Action::ToggleMaxFpsMode),
                    ),
                    (
                        "LOW-LATENCY INPUT, STREAMING & SCHEDULER",
                        "Direct touch dispatch, 4 MiB chunk upload budget boost & Android ADPF thermal hints",
                        SettingControl::SectionHeader("Engine Tuning", Icon::Gear),
                    ),
                    (
                        "Zero-Latency Touch Pipeline",
                        "Bypasses redundant pointer filtering for instant camera & virtual mouse response",
                        SettingControl::Toggle(
                            c.zero_latency_touch,
                            Action::ToggleZeroLatencyTouch,
                        ),
                    ),
                    (
                        "Fast Chunk Mesh Upload Boost",
                        "Increases per-frame GPU upload budget to 4 MiB to eliminate movement stutter",
                        SettingControl::Toggle(c.fast_chunk_upload, Action::ToggleFastChunkUpload),
                    ),
                    (
                        "Android ADPF Performance Hints",
                        "Sends real-time CPU thermal & frame-work hints to the Android scheduler",
                        SettingControl::Toggle(
                            c.device.performance_hints,
                            Action::PerformanceHints,
                        ),
                    ),
                    (
                        "Detailed GPU Pass Profiling",
                        "Records per-pass GPU timings in diagnostic reports (adds minor overhead)",
                        SettingControl::Toggle(c.device.gpu_profile, Action::GpuProfile),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        &mut self.anim,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            SettingsCategory::Diagnostics => {
                let c = &self.catalog.config;
                let count = 7;
                let max_scroll = (count as f32 * step_y - (list_bottom - list_top)).max(0.0);
                self.catalog.scroll = self.catalog.scroll.clamp(0.0, max_scroll);
                self.list_view = Some([list_top, list_bottom, max_scroll]);

                let has_err = self.home.join("android-error.txt").is_file();
                let health_badge = if has_err {
                    "Crash Log Recorded"
                } else {
                    "Runtime Healthy"
                };

                let rows: [(&str, &str, SettingControl<'_>); 7] = [
                    (
                        "RUNTIME HEALTH & CRASH DIAGNOSTICS",
                        "Live engine summary, shader/Vulkan health check, crash logs & ZIP bundle export",
                        SettingControl::SectionHeader(health_badge, Icon::Terminal),
                    ),
                    (
                        "Inspect logs & device summary",
                        "Open interactive Diagnostics & Runtime Health Summary dialog",
                        SettingControl::Button("View Summary", Action::Logs, true),
                    ),
                    (
                        "Export diagnostic report (.zip)",
                        "Packages logs and device info into a local ZIP for troubleshooting",
                        SettingControl::Button("Export ZIP", Action::ExportDiagnostics, false),
                    ),
                    (
                        "FRAME TELEMETRY & WORLD STORAGE",
                        "JSONL frame-time recorder and direct access to profile world saves",
                        SettingControl::SectionHeader("Telemetry & Saves", Icon::Folder),
                    ),
                    (
                        "Record gameplay performance",
                        "Captures frame-time and chunk-build telemetry during Play (up to 30 min)",
                        SettingControl::Toggle(c.record_performance, Action::RecordPerformance),
                    ),
                    (
                        "Export raw performance data (.jsonl)",
                        "Share or inspect recorded frame telemetry in Android Files",
                        SettingControl::Button("Export JSONL", Action::ExportPerformance, false),
                    ),
                    (
                        "World backup & file manager",
                        "Open worlds folder in system Files to back up or copy saves",
                        SettingControl::Button("World Files", Action::WorldFiles, false),
                    ),
                ];

                ui.set_clip(Some([
                    content_x - 2.0,
                    list_top,
                    content_w + 4.0,
                    (list_bottom - list_top).max(1.0),
                ]));
                for (i, (title, sub, ctrl)) in rows.into_iter().enumerate() {
                    let y = list_top + i as f32 * step_y - self.catalog.scroll;
                    if y + card_h < list_top || y > list_bottom {
                        continue;
                    }
                    draw_setting_card(
                        ui,
                        &mut self.targets,
                        &mut self.list_targets,
                        &mut self.anim,
                        content_x,
                        y,
                        content_w,
                        card_h,
                        title,
                        sub,
                        ctrl,
                    );
                }
                ui.set_clip(None);
            }
            // Not reachable from the category grid yet.
            SettingsCategory::Overview | SettingsCategory::Account => {}
        }
    }

    pub(super) fn activate(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.session_worker.is_some() && self.chooser.is_none() && !self.exit_confirmation {
            return Ok(());
        }
        let (Some(r), Some(gui)) = (&self.renderer, &self.gui) else {
            return Ok(());
        };
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        let p = self.focus.and_then(|index| self.targets.get(index)).map_or(
            [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ],
            |([x, y, w, h], _)| [x + w / 2.0, y + h / 2.0],
        );
        if self.tutorial.active && !self.tutorial.closing {
            self.handle_tutorial_tap(p[0], p[1]);
            return Ok(());
        }
        let action = self
            .targets
            .iter()
            .rev()
            .find(|([x, y, w, h], _)| {
                (*x..*x + *w).contains(&p[0]) && (*y..*y + *h).contains(&p[1])
            })
            .map(|(_, a)| *a);
        if crate::documents::busy() {
            return Ok(());
        }
        if self.catalog.busy()
            && !matches!(
                action,
                Some(
                    Action::PauseCatalogDownload
                        | Action::CancelCatalogDownload
                        | Action::Page(_)
                        | Action::Account
                )
            )
        {
            return Ok(());
        }
        self.feedback = action.map(|action| (action, std::time::Instant::now()));
        self.focus = None;
        if action.is_some() && horizon_client::official_install::ready(&self.home) {
            let audio = self.audio.get_or_insert_with(|| {
                let mut audio = horizon_client::audio::Audio::new(
                    &self.home.join(".local/horizon-client/26.3"),
                );
                let settings = horizon_client::settings::Settings::load(
                    &self
                        .catalog
                        .config
                        .game_home(&self.home)
                        .join("options.txt"),
                );
                audio.master = settings.volumes[0];
                let mut volumes = settings.volumes;
                volumes[0] = 1.0;
                horizon_client::settings::set_source_volumes(volumes);
                audio
            });
            audio.play_button_click();
        }
        match action {
            Some(Action::PauseInstall) => {
                let next = !self.install_paused.load(Ordering::Relaxed);
                self.install_paused.store(next, Ordering::Relaxed);
            }
            Some(Action::OpenRoadGame) => {
                if self.worker.is_some() {
                    self.auto_launch_after_install = true;
                    self.road_game = Some(super::cross_road::RoadGame::default());
                }
            }
            Some(Action::CloseRoadGame) => self.road_game = None,
            Some(Action::RoadMove(dx, dy)) => {
                if let Some(road) = &mut self.road_game { road.move_by(i32::from(dx), i32::from(dy)); }
            }
            Some(Action::RestartRoadGame) => {
                self.road_game = Some(super::cross_road::RoadGame::default());
            }
            Some(Action::CancelInstall) => {
                self.auto_launch_after_install = false;
                self.road_game = None;
                self.install_cancelled.store(true, Ordering::Relaxed);
                self.install_paused.store(false, Ordering::Relaxed);
                self.catalog.status = "Cancelling game preparation…".into();
            }
            Some(Action::PauseCatalogDownload) => {
                self.catalog.toggle_pause();
            }
            Some(Action::CancelCatalogDownload) => {
                self.catalog.cancel_download();
            }
            Some(Action::ConfirmExit) => {
                events.exit();
            }
            Some(Action::SettingsCat(cat)) => {
                self.catalog.settings_category = cat;
                self.catalog.scroll = 0.0;
                self.anim.trigger_page_transition();
            }
            Some(Action::OpenProject(index)) => {
                if let Some(project) = self.catalog.results.get(index).cloned() {
                    self.catalog
                        .open_project_modal(&self.home, project, None, Some(index));
                }
            }
            Some(Action::OpenBundledProject(id)) => {
                if let Some((_, title, desc)) = horizon_client::mod_list::ANDROID_BUNDLED
                    .iter()
                    .find(|(bid, _, _)| *bid == id)
                {
                    let project = crate::browser::Project {
                        source: crate::browser::SourceId::Horizon,
                        id: (*id).to_owned(),
                        slug: (*id).to_owned(),
                        title: (*title).to_owned(),
                        summary: (*desc).to_owned(),
                        author: "Horizon Built-in".into(),
                        downloads: 0,
                        icon: Some(format!(
                            "https://horizon-rust.github.io/horizon-mods/icons/{id}.png"
                        )),
                        page: String::new(),
                        kind: crate::browser::Kind::Mod,
                        license: "MIT / Apache-2.0".into(),
                        game_versions: vec![crate::browser::GAME.into()],
                        loaders: vec!["horizon".into()],
                    };
                    self.catalog
                        .open_project_modal(&self.home, project, Some(id), None);
                }
            }
            Some(Action::CloseProjectModal) => {
                self.catalog.project_modal = None;
                self.catalog.modal_worker = None;
            }
            Some(Action::SelectModalVersion(v_idx)) => {
                if let Some(modal) = &mut self.catalog.project_modal {
                    modal.selected_version = v_idx;
                }
            }
            Some(Action::InstallModalProject) => {
                if let Some(modal) = self.catalog.project_modal.take() {
                    self.catalog.modal_worker = None;
                    if let Some(id) = modal.bundled_id {
                        self.toggle_bundled_mod(id)?;
                    } else if let Some(idx) = modal.catalog_index {
                        self.catalog.install(&self.home, idx);
                    }
                }
            }
            Some(Action::UiScale) => self.open_chooser(chooser::Kind::UiScale),
            Some(Action::ChooseUiScale(pct)) => {
                self.catalog.config.ui_scale_percent = pct;
                horizon_client::ui_theme::set_ui_scale_percent(pct);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = format!(
                    "Launcher UI scale set to {}.",
                    self.catalog.config.ui_scale_label()
                );
            }
            // Video settings mutations
            Some(Action::RenderScaleDelta(delta)) => {
                let next_pct = (self.catalog.config.render_scale.percent() as i32 + delta)
                    .clamp(25, 100) as u32;
                let scale = crate::display::RenderScale::new(next_pct);
                self.catalog.config.render_scale = scale;
                let _ = self.catalog.config.save(&self.home);
                self.modify_game_settings(|s| {
                    s.horizon.resolution_scale = next_pct;
                });
                self.modify_touch_settings(|t| {
                    t.resolution_scale_percent = next_pct;
                });
            }
            Some(Action::RenderDistDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.render_distance = (s.render_distance + delta).clamp(2, 32);
                });
            }
            Some(Action::SimDistDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.simulation_distance = (s.simulation_distance + delta).clamp(5, 32);
                });
            }
            Some(Action::FovDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.fov = (s.fov + delta).clamp(30.0, 110.0);
                });
            }
            Some(Action::CycleMaxFps) => {
                let mut now_uncapped = false;
                self.modify_game_settings(|s| {
                    s.max_fps = match s.max_fps {
                        0..=45 => 60,
                        46..=75 => 90,
                        76..=105 => 120,
                        106..=200 => 260,
                        _ => 30,
                    };
                    if s.max_fps >= 260 {
                        s.vsync = false;
                        s.horizon.present_mode = horizon_client::graphics::PresentMode::Immediate;
                        s.inactivity_fps_limit = "minimized".into();
                        now_uncapped = true;
                    }
                });
                self.catalog.config.max_fps_mode = now_uncapped;
                if now_uncapped {
                    self.catalog.config.battery_perf_mode = false;
                }
                let _ = self.catalog.config.save(&self.home);
            }
            Some(Action::CycleGraphicsPreset) => {
                self.modify_game_settings(|s| {
                    s.graphics_preset = match s.graphics_preset.as_str() {
                        "fast" => "fancy".into(),
                        "fancy" => "fabulous".into(),
                        _ => "fast".into(),
                    };
                });
            }
            Some(Action::ToggleVsync) => {
                let mut vsync_now = false;
                let mut fps_now = 120;
                self.modify_game_settings(|s| {
                    s.vsync = !s.vsync;
                    vsync_now = s.vsync;
                    fps_now = s.max_fps;
                    s.horizon.present_mode = if s.vsync {
                        horizon_client::graphics::PresentMode::Fifo
                    } else {
                        horizon_client::graphics::PresentMode::Immediate
                    };
                });
                if vsync_now {
                    self.catalog.config.max_fps_mode = false;
                } else if fps_now >= 260 {
                    self.catalog.config.max_fps_mode = true;
                    self.catalog.config.battery_perf_mode = false;
                }
                let _ = self.catalog.config.save(&self.home);
            }
            Some(Action::ToggleSmoothLighting) => {
                self.modify_game_settings(|s| s.smooth_lighting = !s.smooth_lighting);
            }
            Some(Action::CycleClouds) => {
                self.modify_game_settings(|s| {
                    s.clouds = match s.clouds.as_str() {
                        "false" => "fast".into(),
                        "fast" => "true".into(),
                        _ => "false".into(),
                    };
                });
            }
            Some(Action::ToggleMenuBlur) => {
                self.modify_game_settings(|s| {
                    s.menu_blur = if s.menu_blur > 0 { 0 } else { 4 };
                });
            }
            // Experimental settings mutations
            Some(Action::ToggleSuperPerf) => {
                self.catalog.config.super_perf_mode = !self.catalog.config.super_perf_mode;
                if self.catalog.config.super_perf_mode {
                    self.catalog.config.battery_perf_mode = false;
                    self.catalog.config.renderer = crate::catalog::RendererMode::Vulkan;
                }
                horizon_client::ui_theme::motion::set_reduced_motion(
                    self.catalog.config.super_perf_mode || self.catalog.config.battery_perf_mode,
                );
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.super_perf_mode {
                    "Super Performance Mode enabled: fast chunk culling, minimal particles & Vulkan batching."
                } else {
                    "Super Performance Mode disabled."
                }
                .into();
            }
            Some(Action::ToggleBatteryPerf) => {
                self.catalog.config.battery_perf_mode = !self.catalog.config.battery_perf_mode;
                if self.catalog.config.battery_perf_mode {
                    self.catalog.config.super_perf_mode = false;
                    self.catalog.config.max_fps_mode = false;
                    self.catalog.config.device.display_hz = 60;
                } else if self.catalog.config.device.display_hz == 60 {
                    self.catalog.config.device.display_hz = 0;
                }
                crate::device_runtime::configure(self.catalog.config.device);
                horizon_client::ui_theme::motion::set_reduced_motion(
                    self.catalog.config.super_perf_mode || self.catalog.config.battery_perf_mode,
                );
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.battery_perf_mode {
                    "Battery Performance Mode enabled: 60 FPS cap with VSync and balanced workers."
                } else {
                    "Battery Performance Mode disabled."
                }
                .into();
            }
            Some(Action::ToggleMaxFpsMode) => {
                self.catalog.config.max_fps_mode = !self.catalog.config.max_fps_mode;
                if self.catalog.config.max_fps_mode {
                    self.catalog.config.battery_perf_mode = false;
                    if self.catalog.config.device.display_hz < 120 {
                        self.catalog.config.device.display_hz = 120;
                    }
                } else {
                    if self.catalog.config.device.display_hz == 120 {
                        self.catalog.config.device.display_hz = 0;
                    }
                    self.modify_game_settings(|s| {
                        s.max_fps = 120;
                        s.vsync = true;
                        s.horizon.present_mode = horizon_client::graphics::PresentMode::Auto;
                    });
                }
                crate::device_runtime::configure(self.catalog.config.device);
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.max_fps_mode {
                    "Max FPS Mode enabled: 260 FPS uncapped with VSync disabled and high-refresh requested."
                } else {
                    "Max FPS Mode disabled (balanced 120 FPS default restored)."
                }
                .into();
            }
            Some(Action::ToggleZeroLatencyTouch) => {
                self.catalog.config.zero_latency_touch = !self.catalog.config.zero_latency_touch;
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.zero_latency_touch {
                    "Zero-Latency Touch Pipeline active: zero deadzone on look and virtual mouse."
                } else {
                    "Standard touch pipeline active."
                }
                .into();
            }
            Some(Action::ToggleFastChunkUpload) => {
                self.catalog.config.fast_chunk_upload = !self.catalog.config.fast_chunk_upload;
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.catalog.status = if self.catalog.config.fast_chunk_upload {
                    "Fast Chunk Mesh Upload Boost active."
                } else {
                    "Standard chunk mesh upload active."
                }
                .into();
            }
            // Controls & Control Editor actions
            Some(Action::SelectControlMode(style)) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                touch.switch_control_system(&game_home, style);
                self.catalog.status = format!(
                    "Control Mode set to {} (Active preset: '{}').",
                    style.label(),
                    touch.preset_name
                );
            }
            Some(Action::ActivateStudioPreset(style, idx)) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                let presets = horizon_client::touch::Settings::list_presets(&game_home, style);
                if let Some(meta) = presets.get(idx as usize) {
                    touch.select_preset(&game_home, style, &meta.id);
                    self.catalog.status = format!(
                        "Activated {} preset '{}'.",
                        style.label(),
                        meta.name
                    );
                }
            }
            Some(Action::DeleteStudioPreset(style, idx)) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                let presets = horizon_client::touch::Settings::list_presets(&game_home, style);
                if let Some(meta) = presets.get(idx as usize) {
                    match touch.delete_preset(&game_home, style, &meta.id) {
                        Ok(()) => {
                            self.catalog.status = format!(
                                "Deleted {} preset '{}'.",
                                style.label(),
                                meta.name
                            );
                        }
                        Err(e) => {
                            self.catalog.status = format!("Could not delete preset: {e}");
                        }
                    }
                }
            }
            Some(Action::OpenControlEditor) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut tc = horizon_client::touch::TouchControls::default();
                tc.settings = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                tc.open_editor();
                self.control_editor = Some(tc);
            }
            Some(Action::CloseControlEditor) => {
                if let Some(mut tc) = self.control_editor.take() {
                    let game_home = self.catalog.config.game_home(&self.home);
                    if !tc.settings.is_default_preset() {
                        let _ = tc.settings.save_replace_preset(&game_home);
                    } else {
                        let _ = tc.settings.save_to(&game_home.join("horizon-touch.json"));
                    }
                    self.catalog.status = format!(
                        "Saved {} preset '{}'.",
                        tc.settings.control_style.label(),
                        tc.settings.preset_name
                    );
                }
            }
            Some(Action::ControlProfileChooser) => {
                self.open_chooser(chooser::Kind::ControlProfile);
            }
            Some(Action::ChooseControlProfile(idx)) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let mut touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                let style = touch.control_style;
                let presets = horizon_client::touch::Settings::list_presets(&game_home, style);
                if let Some(meta) = presets.get(idx as usize) {
                    touch.select_preset(&game_home, style, &meta.id);
                    self.catalog.status = format!(
                        "Selected {} preset '{}' ({}).",
                        style.label(),
                        meta.name,
                        meta.description
                    );
                }
                self.chooser = None;
            }
            Some(Action::CopyActivePreset) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                self.catalog.config.preset_edit_mode = "copy".into();
                self.catalog.config.custom_profile_name = format!("{} Copy", touch.preset_name);
                self.catalog.config.custom_profile_desc = touch.preset_description.clone();
                self.catalog.editing = Some("profile_name");
                crate::platform::show(&self.catalog.config.custom_profile_name);
                self.catalog.status = format!(
                    "Step 1/2: Enter new {} preset name and press Enter.",
                    touch.control_style.label()
                );
            }
            Some(Action::EditActivePresetMeta) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                if touch.is_default_preset() {
                    self.catalog.status = format!(
                        "{} Default preset is protected and cannot be renamed or overwritten. Use 'Copy Preset' to create an editable copy.",
                        touch.control_style.label()
                    );
                } else {
                    self.catalog.config.preset_edit_mode = "edit".into();
                    self.catalog.config.custom_profile_name = touch.preset_name.clone();
                    self.catalog.config.custom_profile_desc = touch.preset_description.clone();
                    self.catalog.editing = Some("profile_name");
                    crate::platform::show(&self.catalog.config.custom_profile_name);
                    self.catalog.status = format!(
                        "Step 1/2: Edit {} preset name and press Enter.",
                        touch.control_style.label()
                    );
                }
            }
            Some(Action::SaveControlProfileAsNew) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                self.catalog.config.preset_edit_mode = "copy".into();
                self.catalog.config.custom_profile_name = format!("{} Copy", touch.preset_name);
                self.catalog.config.custom_profile_desc = touch.preset_description.clone();
                self.catalog.editing = Some("profile_name");
                crate::platform::show(&self.catalog.config.custom_profile_name);
                self.catalog.status = format!(
                    "Step 1/2: Enter new {} preset name and press Enter.",
                    touch.control_style.label()
                );
            }
            Some(Action::ConfirmReplaceDuplicatePreset) => {
                if let Some((_, name, desc)) = self.catalog.duplicate_preset_confirm.take() {
                    let game_home = self.catalog.config.game_home(&self.home);
                    let mut touch = horizon_client::touch::Settings::load_from(
                        &game_home.join("horizon-touch.json"),
                    );
                    if let Ok(meta) = touch.save_or_replace_preset_by_name(&game_home, &name, &desc)
                    {
                        self.catalog.status = format!(
                            "Replaced {} preset '{}' (controls/{}/{}.json).",
                            meta.system.label(),
                            meta.name,
                            meta.system.storage_subdir(),
                            meta.id
                        );
                    }
                }
            }
            Some(Action::CancelReplaceDuplicatePreset) => {
                self.catalog.duplicate_preset_confirm = None;
                self.catalog.status = "Cancelled replacing existing control preset.".into();
            }
            Some(Action::ExportControlProfile) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let touch = horizon_client::touch::Settings::load_from(
                    &game_home.join("horizon-touch.json"),
                );
                let clean = horizon_client::touch::Settings::sanitize_profile_name(&touch.preset_id);
                let suggested_name = format!("horizon-controls-{clean}.json");
                if let Ok(saved_path) = touch.export_profile(&self.home, &game_home, &touch.preset_id) {
                    self.catalog.fm_clipboard = Some((vec![saved_path], false));
                }
                match serde_json::to_vec_pretty(&touch) {
                    Ok(bytes) => {
                        match crate::documents::begin_control_export(
                            game_home.clone(),
                            suggested_name.clone(),
                            bytes,
                        ) {
                            Ok(()) => {
                                self.catalog.status = format!(
                                    "Select where to save '{}' (also saved in controls/{}/ and copied to clipboard).",
                                    suggested_name,
                                    touch.control_style.storage_subdir()
                                );
                            }
                            Err(_) => {
                                self.catalog.status = format!(
                                    "Saved '{}' in controls/{}/{}.json and copied to File Manager clipboard.",
                                    suggested_name,
                                    touch.control_style.storage_subdir(),
                                    touch.preset_id
                                );
                            }
                        }
                    }
                    Err(e) => self.catalog.status = e.to_string(),
                }
            }
            Some(Action::ImportControlProfile) => {
                let game_home = self.catalog.config.game_home(&self.home);
                horizon_client::touch::Settings::ensure_system_storage(
                    &game_home,
                    horizon_client::touch::ControlStyle::Mcpe,
                );
                horizon_client::touch::Settings::ensure_system_storage(
                    &game_home,
                    horizon_client::touch::ControlStyle::GoyControl,
                );
                match crate::documents::begin_control_import(game_home) {
                    Ok(()) => {
                        self.catalog.status =
                            "Pick a control .json file. Java layouts are saved separately but cannot be used yet."
                                .into();
                    }
                    Err(err) => {
                        self.catalog.status = err;
                    }
                }
            }
            Some(Action::OpenExperimentalZalithStudio) => {
                let game_home = self.catalog.config.game_home(&self.home);
                let dir = game_home.join("controls/java/zalith");
                let latest = std::fs::read_dir(dir).ok().into_iter().flatten()
                    .filter_map(std::result::Result::ok)
                    .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file())
                        && entry.path().extension().is_some_and(|ext| ext == "json"))
                    .max_by_key(|entry| (entry.metadata().ok().and_then(|m| m.modified().ok()), entry.path()))
                    .map(|entry| entry.path());
                self.catalog.status = match latest {
                    Some(path) if crate::native_ime::open_zalith_studio(&path) =>
                        "Opening experimental Zalith Studio. Saved layouts are not usable in gameplay yet.".into(),
                    Some(_) => "Could not open the experimental Zalith Studio.".into(),
                    None => "Import a Zalith JSON layout first.".into(),
                };
            }
            Some(Action::MouseSensDelta(delta)) => {
                self.modify_game_settings(|s| {
                    s.sensitivity = (s.sensitivity + delta).clamp(0.05, 1.0);
                });
            }
            Some(Action::TouchSensDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.sensitivity = (t.sensitivity + delta).clamp(0.10, 3.0);
                });
            }
            Some(Action::VirtualMouseSpeedDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.virtual_mouse_sensitivity =
                        (t.virtual_mouse_sensitivity + delta).clamp(0.25, 3.0);
                });
            }
            Some(Action::ButtonScaleDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.button_scale = (t.button_scale + delta).clamp(0.60, 1.60);
                });
            }
            Some(Action::ToggleControlScheme) => {
                self.modify_touch_settings(|t| {
                    t.mode = match t.mode {
                        horizon_client::touch::Mode::Crosshair => horizon_client::touch::Mode::Tap,
                        horizon_client::touch::Mode::Tap => horizon_client::touch::Mode::Dpad,
                        horizon_client::touch::Mode::Dpad => horizon_client::touch::Mode::Crosshair,
                    };
                });
            }
            Some(Action::ToggleSplitControls) => {
                self.modify_touch_settings(|t| t.toggle_split_touch());
            }
            Some(Action::ToggleSprintLatch) => {
                self.modify_touch_settings(|t| t.sprint_toggle = !t.sprint_toggle);
            }
            Some(Action::ToggleLeftHanded) => {
                self.modify_touch_settings(|t| t.left_handed = !t.left_handed);
            }
            Some(Action::ToggleInvertY) => {
                let mut next = false;
                self.modify_game_settings(|s| {
                    s.invert_y = !s.invert_y;
                    next = s.invert_y;
                });
                self.modify_touch_settings(|t| t.invert_y = next);
            }
            Some(Action::ToggleGyro) => {
                self.modify_touch_settings(|t| t.gyroscope = !t.gyroscope);
            }
            Some(Action::GyroSensDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.gyro_sensitivity = (t.gyro_sensitivity + delta).clamp(0.10, 3.0);
                });
            }
            Some(Action::ToggleVirtualMouseEnabled) => {
                self.modify_touch_settings(|t| {
                    t.virtual_mouse_enabled = !t.virtual_mouse_enabled;
                });
            }
            Some(Action::ToggleOverlayButton) => {
                self.modify_touch_settings(|t| {
                    t.overlay_hidden = !t.overlay_hidden;
                    t.overlay_hide_in_menus = false;
                    t.overlay_show_in_gameplay = false;
                    if !t.overlay_hidden {
                        if t.overlay_opacity < 0.35 || !t.overlay_opacity.is_finite() {
                            t.overlay_opacity = 1.0;
                        }
                        if !t.floating_menu_pos[0].is_finite()
                            || !t.floating_menu_pos[1].is_finite()
                            || t.floating_menu_pos[0] < 0.0
                            || t.floating_menu_pos[0] > 0.95
                            || t.floating_menu_pos[1] < 0.0
                            || t.floating_menu_pos[1] > 0.95
                        {
                            t.floating_menu_pos = [0.02, 0.03];
                        }
                    }
                });
            }
            Some(Action::PointerSizeDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.virtual_mouse_pointer_scale =
                        (t.virtual_mouse_pointer_scale + delta).clamp(0.50, 2.00);
                });
            }
            Some(Action::PointerOpacityDelta(delta)) => {
                self.modify_touch_settings(|t| {
                    t.virtual_mouse_opacity =
                        (t.virtual_mouse_opacity + delta).clamp(0.15, 1.00);
                });
            }
            Some(Action::SelectCursorIcon(icon)) => {
                self.modify_touch_settings(|t| {
                    t.virtual_mouse_cursor = icon;
                });
            }
            Some(Action::ResetMouseCustomization) => {
                self.modify_touch_settings(|t| {
                    t.virtual_mouse_cursor = horizon_client::touch::CursorIcon::Default;
                    t.virtual_mouse_pointer_scale = 1.0;
                    t.virtual_mouse_opacity = 1.0;
                });
            }
            Some(Action::CycleJoystickZone) => {
                self.modify_touch_settings(|t| {
                    t.joystick_style.move_area = t.joystick_style.move_area.next();
                });
            }
            Some(Action::ToggleTouchVibrate) => {
                self.modify_touch_settings(|t| t.vibrate = !t.vibrate);
            }
            Some(Action::ToggleAutoJump) => {
                self.modify_game_settings(|s| s.auto_jump = !s.auto_jump);
            }
            Some(Action::ToggleInventoryControls) => {
                self.modify_touch_settings(|t| t.inventory_controls = !t.inventory_controls);
            }
            Some(Action::Immersive) => {
                self.catalog.config.screen.immersive ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::Cutout) => {
                self.catalog.config.screen.avoid_cutout ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::KeepScreenOn) => {
                self.catalog.config.screen.keep_screen_on ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
                self.catalog.status = if self.catalog.config.screen.keep_screen_on {
                    "Keep screen on enabled: display stays awake."
                } else {
                    "Keep screen on disabled."
                }
                .into();
            }
            Some(Action::Aspect) => self.open_chooser(chooser::Kind::Aspect),
            Some(Action::ChooseAspect(aspect)) => {
                self.catalog.config.screen.aspect = aspect;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
                self.chooser = None;
            }
            Some(Action::CacheBudget) => self.open_chooser(chooser::Kind::CacheBudget),
            Some(Action::ChooseCacheBudget(mib)) => {
                self.catalog.config.cache_budget_mib = mib.min(1024);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Disposable cache budget saved. Applies on next Play; not a total RAM or VRAM limit.".into();
            }
            Some(Action::ToggleBundled(id)) => {
                self.toggle_bundled_mod(id)?;
            }
            Some(Action::GameFiles) => {
                self.catalog.status = if crate::platform::open_documents(crate::document_store::ROOT) {
                    "Game files opened. Worlds, mods, packs, shaders, screenshots and reports are available here."
                } else { "Open system Files > Volt Plus." }.into();
            }
            Some(Action::ScreenshotFiles) => {
                let id = crate::document_store::folder_id(self.catalog.config.profile, "screenshots");
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Screenshots directory opened in Android Files."
                } else { "Open system Files > Volt Plus > Screenshots." }.into();
            }
            Some(Action::WorldFiles) => {
                let id = crate::document_store::folder_id(self.catalog.config.profile, "worlds");
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Copy a complete world folder to back up or import. Finish copying before Play; never replace an existing world silently."
                } else { "Open system Files > Volt Plus > Worlds. Copy complete world folders while the game is closed." }.into();
            }
            Some(Action::WorkerLimit) => self.open_chooser(chooser::Kind::WorkerLimit),
            Some(Action::ChooseWorkerLimit(limit)) => {
                self.catalog.config.worker_limit = limit.min(64);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Chunk worker ceiling saved. Applies on the next Play; other engine threads are unchanged.".into();
            }
            Some(Action::DisplayHz) => self.open_chooser(chooser::Kind::DisplayHz),
            Some(Action::ChooseDisplayHz(hz)) => {
                self.catalog.config.device.display_hz = hz;
                self.catalog.config.device.sanitize();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
                self.chooser = None;
            }
            Some(Action::PerformanceHints) => {
                self.catalog.config.device.performance_hints ^= true;
                self.catalog.status="ADPF gives Android CPU frame-work hints on supported devices. It does not cap FPS or guarantee a speedup.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }
            Some(Action::GpuProfile) => {
                self.catalog.config.device.gpu_profile ^= true;
                self.catalog.status="Detailed GPU profiling records diagnostics and has overhead. Reports remain on this device.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }

            Some(Action::ShaderSource(origin)) => {
                self.catalog.config.shader_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.scroll = 0.0;
                self.catalog.browse(&self.home);
            }
            Some(Action::ModSource(origin)) => {
                self.catalog.config.mod_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.scroll = 0.0;
                self.catalog.browse(&self.home);
            }
            Some(Action::SetCompilerMode(mode)) => {
                self.catalog.compiler_mode = mode;
                self.anim.trigger_page_transition();
            }
            Some(Action::CompilerRunQuery) => {
                crate::platform::hide();
                self.catalog.run_compiler_from_query(&self.home);
            }
            Some(Action::CompilerPortLocal(idx)) => {
                self.catalog.run_compiler_on_local(&self.home, idx);
            }
            Some(Action::OpenFolder(kind)) => {
                crate::platform::hide();
                let rel_dir = match kind {
                    horizon_client::content_files::Kind::Mods => "horizon-mods",
                    horizon_client::content_files::Kind::Packs => "resourcepacks",
                    horizon_client::content_files::Kind::Shaders => "shaderpacks",
                };
                self.catalog.page = crate::catalog::Page::Installed;
                self.catalog.installed_sub_tab = crate::catalog::InstalledSubTab::FileManager;
                self.catalog.fm_navigate(&self.home, rel_dir.to_owned());
                self.anim.trigger_page_transition();
                self.catalog.status = format!("Opened '{rel_dir}' in Profile File Manager.");
            }
            Some(Action::Import(kind)) => {
                crate::platform::hide();
                let home = self.catalog.config.game_home(&self.home);
                if let Err(e) = crate::documents::begin(kind, home) {
                    self.catalog.status = e;
                }
            }
            Some(Action::ToggleSection(i)) => {
                self.catalog.sections[i] = !self.catalog.sections[i];
                self.catalog.scroll = 0.0;
            }
            Some(Action::ToggleLocal(i)) => self.catalog.toggle_local(&self.home, i),
            Some(Action::DeleteLocal(i)) => {
                self.catalog.pending_delete = self.catalog.local.get(i).cloned();
                if let Some(item) = &self.catalog.pending_delete {
                    self.chooser = Some(super::chooser::Chooser::confirm_delete(item.file.clone()));
                }
            }
            Some(Action::ConfirmDeleteLocal) => {
                self.catalog.remove_local(&self.home);
                self.chooser = None;
            }
            Some(Action::Repository) => {
                self.catalog.status = if crate::platform::open_repository() {
                    "Repository opened in browser."
                } else {
                    "No browser available to open the repository."
                }
                .into();
            }
            Some(Action::Discord) => {
                self.catalog.status = if crate::platform::open_url("https://discord.gg/WsBxGsKyhU")
                {
                    "Discord opened in browser."
                } else {
                    "No browser available to open Discord."
                }
                .into();
            }
            Some(Action::Account) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.login.refresh_status();
                self.login.visible = true;
                self.login.offline = false;
                self.login.ely = false;
                if self.login.signed_in {
                    // Already signed in: open the selector, with the current
                    // Microsoft/Ely.by account called out, instead of asking
                    // the player to choose a provider again.
                    self.login.providers = false;
                    self.login.choosing = true;
                    self.login.account_offset = 0;
                } else {
                    self.login.providers = true;
                    self.login.choosing = false;
                }
            }
            Some(Action::AddAccount) => {
                self.login.choosing = false;
                self.login.providers = true;
                self.login.offline = false;
                self.login.ely = false;
                self.login.visible = true;
            }
            Some(Action::OfflineAccount | Action::MicrosoftAccount) => {
                self.login.providers = false;
                self.login.offline = matches!(action, Some(Action::OfflineAccount));
                self.login.ely = false;
                self.login.choosing = false;
                self.login.visible = true;
            }
            Some(Action::SavedAccounts) => {
                self.login.providers = false;
                self.login.refresh_status();
                self.login.choosing = true;
                self.login.ely = false;
            }
            Some(Action::SavedPage(next)) => {
                if next && self.login.account_offset + 3 < self.login.saved_accounts.len() {
                    self.login.account_offset += 3;
                } else if !next {
                    self.login.account_offset = self.login.account_offset.saturating_sub(3);
                }
            }
            Some(Action::SelectAccount(index)) => {
                let mut accounts = crate::account::Accounts::load();
                if let Some(account) = self.login.saved_accounts.get(index)
                    && accounts.accounts.iter().any(|a| a.key() == account.key())
                {
                    accounts.selected = Some(account.key());
                    accounts.save()?;
                }
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.refresh_status();
                self.refresh_wardrobe_account();
            }
            Some(Action::SelectOffline) => {
                self.catalog.config.save(&self.home)?;
                self.catalog.editing = None;
                crate::platform::hide();
                self.login.hide();
                let mut accounts = crate::account::Accounts::load();
                accounts.selected = None;
                accounts.save()?;
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.choosing = false;
                self.login.refresh_status();
                self.refresh_wardrobe_account();
            }
            Some(Action::ElySignIn) => {
                self.login.providers = false;
                self.login.close();
                self.login.ely = true;
                self.login.visible = true;
            }
            Some(Action::SubmitEly) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                if !self.login.pending() {
                    self.login.start_ely();
                }
            }
            Some(Action::CloseLogin) => {
                self.login.hide();
                crate::platform::hide();
                self.catalog.editing = None;
                self.refresh_wardrobe_account();
            }
            Some(Action::DeviceSignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login
                    .start_device(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::CopyLoginCode) => self.login.copy_code(),
            Some(Action::CancelLogin) => {
                self.login.close();
                self.login.visible = true;
            }
            Some(Action::ReviewApp) => self.login.review_app(),
            Some(Action::SignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.start(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::SignOut) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.sign_out();
                self.refresh_wardrobe_account();
            }
            Some(Action::LoginLink) => self.login.open_link(),
            Some(Action::Page(page)) => {
                if page == Page::Skins {
                    self.wardrobe.open(
                        &self.catalog.config.name,
                        &self.catalog.config.microsoft_client_id,
                    );
                }
                self.catalog.editing = None;
                self.catalog.project_modal = None;
                self.catalog.modal_worker = None;
                if crate::platform::active() {
                    crate::platform::hide();
                }
                if page == Page::Settings && self.catalog.page == Page::Settings {
                    self.catalog.settings_category = SettingsCategory::Overview;
                }
                self.catalog.page = page;
                self.catalog.scroll = 0.0;
                self.error = None;
                self.catalog.offset = 0;
                if page == Page::Worlds {
                    if self.catalog.world_discover_results.is_empty()
                        && !self.catalog.world_discover_busy
                    {
                        self.catalog.search_discover_worlds();
                    }
                } else if page == Page::Inventory {
                    self.catalog.refresh_worlds(&self.home);
                    self.catalog.refresh_screenshots(&self.home);
                } else if page == Page::Replays {
                    self.catalog.refresh_replays(&self.home);
                }
                self.catalog.browse(&self.home);
                self.sync_discord_presence();
            }
            Some(Action::SetInventorySubTab(tab)) => {
                self.catalog.inventory_sub_tab = tab;
                self.catalog.scroll = 0.0;
                self.catalog.world_inspection = None;
                self.catalog.world_delete_confirm = None;
                self.catalog.server_delete_confirm = None;
                self.catalog.screenshot_delete_confirm = None;
                if tab == crate::catalog::InventorySubTab::World {
                    self.catalog.refresh_worlds(&self.home);
                } else if tab == crate::catalog::InventorySubTab::Screenshot {
                    self.catalog.refresh_screenshots(&self.home);
                }
            }
            Some(Action::RefreshScreenshots) => {
                self.catalog.refresh_screenshots(&self.home);
                self.catalog.status = format!(
                    "Found {} screenshot(s).",
                    self.catalog.screenshots.len()
                );
            }
            Some(Action::ExportScreenshot(idx)) => {
                self.catalog.export_screenshot(idx);
            }
            Some(Action::DeleteScreenshot(idx)) => {
                if self.catalog.screenshot_delete_confirm == Some(idx) {
                    self.catalog.delete_screenshot(&self.home, idx);
                } else {
                    self.catalog.screenshot_delete_confirm = Some(idx);
                    self.catalog.status = "Tap Delete again to remove this screenshot.".into();
                }
            }
            Some(Action::OpenScreenshotsFolder) => {
                let doc = crate::document_store::folder_id(
                    self.catalog.config.profile,
                    "screenshots",
                );
                if !crate::platform::open_documents(&doc) {
                    self.catalog.page = Page::Installed;
                    self.catalog.installed_sub_tab =
                        crate::catalog::InstalledSubTab::FileManager;
                    self.catalog.fm_navigate(&self.home, "screenshots".into());
                }
            }
            Some(Action::RefreshWorlds) => {
                self.catalog.refresh_worlds(&self.home);
                self.catalog.status = format!(
                    "Found {} world(s) and {} server(s).",
                    self.catalog.worlds.len(),
                    self.catalog.servers.len()
                );
            }
            Some(Action::PlayWorld(idx)) => {
                if let Some(w) = self.catalog.worlds.get(idx) {
                    self.pending_world = Some(w.folder.clone());
                    self.action(events)?;
                }
            }
            Some(Action::InspectWorld(idx)) => {
                self.catalog.scroll = 0.0;
                self.catalog.inspect_world(&self.home, idx);
            }
            Some(Action::CloseWorldInspection) => {
                self.catalog.world_inspection = None;
                self.catalog.scroll = 0.0;
            }
            Some(Action::DeleteWorld(idx)) => {
                if self.catalog.world_delete_confirm == Some(idx) {
                    self.catalog.delete_world(&self.home, idx);
                    self.catalog.world_delete_confirm = None;
                } else {
                    self.catalog.world_delete_confirm = Some(idx);
                    self.catalog.status = "Tap Delete again to permanently remove this world.".into();
                }
            }
            Some(Action::ImportWorldZip) => {
                crate::platform::hide();
                let saves = self.catalog.config.game_home(&self.home).join("saves");
                let _ = std::fs::create_dir_all(&saves);
                if let Err(e) = crate::documents::begin_into_dir(saves) {
                    self.catalog.status = e;
                }
            }
            Some(Action::ImportWorldFolder) => {
                crate::platform::hide();
                let saves = self.catalog.config.game_home(&self.home).join("saves");
                let _ = std::fs::create_dir_all(&saves);
                if let Err(e) = crate::documents::begin_folder_into_dir(saves) {
                    self.catalog.status = e;
                }
            }
            Some(Action::JoinServer(idx)) => {
                if let Some(srv) = self.catalog.servers.get(idx) {
                    self.pending_server = Some(srv.ip.clone());
                    self.action(events)?;
                }
            }
            Some(Action::DeleteServer(idx)) => {
                if self.catalog.server_delete_confirm == Some(idx) {
                    self.catalog.delete_server(&self.home, idx);
                } else {
                    self.catalog.server_delete_confirm = Some(idx);
                    self.catalog.status = "Tap Delete again to remove this server.".into();
                }
            }
            Some(Action::AddServer) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.add_server(&self.home);
            }
            Some(Action::SearchDiscoverWorlds) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.search_discover_worlds();
            }
            Some(Action::InstallDiscoverWorld(idx)) => {
                if !self.catalog.world_discover_busy {
                    self.catalog.install_discover_world(&self.home, idx);
                }
            }
            Some(Action::Play) => self.action(events)?,
            Some(Action::SetReplaysSubTab(tab)) => {
                self.catalog.replays_sub_tab = tab;
                self.catalog.scroll = 0.0;
                self.catalog.replay_delete_confirm = None;
                self.catalog.rendered_replay_delete_confirm = None;
                self.catalog.refresh_replays(&self.home);
            }
            Some(Action::RefreshReplays) => {
                self.catalog.refresh_replays(&self.home);
                self.catalog.status = format!(
                    "Found {} raw replay(s) and {} rendered MP4(s).",
                    self.catalog.replays.len(),
                    self.catalog.rendered_replays.len()
                );
            }
            Some(Action::OpenRawReplaysFolder) => {
                let doc = crate::document_store::folder_id(
                    self.catalog.config.profile,
                    "raw_replays",
                );
                if !crate::platform::open_documents(&doc) {
                    self.catalog.page = Page::Installed;
                    self.catalog.installed_sub_tab =
                        crate::catalog::InstalledSubTab::FileManager;
                    self.catalog.fm_navigate(&self.home, "replays/raw".into());
                }
            }
            Some(Action::OpenRenderedReplaysFolder) => {
                let doc = crate::document_store::folder_id(
                    self.catalog.config.profile,
                    "rendered_replays",
                );
                if !crate::platform::open_documents(&doc) {
                    self.catalog.page = Page::Installed;
                    self.catalog.installed_sub_tab =
                        crate::catalog::InstalledSubTab::FileManager;
                    self.catalog
                        .fm_navigate(&self.home, "replays/rendered".into());
                }
            }
            Some(Action::PlayRenderedReplay(idx)) => {
                if let Some(item) = self.catalog.rendered_replays.get(idx) {
                    let profile = self.catalog.config.profile;
                    let note = crate::screen_record::play_rendered_replay(profile, &item.file_name);
                    if !note.is_empty() {
                        self.catalog.status = note;
                    }
                }
            }
            Some(Action::DeleteRenderedReplay(idx)) => {
                if self.catalog.rendered_replay_delete_confirm == Some(idx) {
                    if let Some(item) = self.catalog.rendered_replays.get(idx) {
                        let name = item.display_name.clone();
                        match horizon_client::replay::delete_rendered_replay(&item.path) {
                            Ok(()) => {
                                self.catalog.refresh_replays(&self.home);
                                self.catalog.status = format!("Deleted rendered replay: {name}");
                            }
                            Err(err) => {
                                self.error = Some(format!("Delete failed: {err}"));
                            }
                        }
                    }
                    self.catalog.rendered_replay_delete_confirm = None;
                } else {
                    self.catalog.rendered_replay_delete_confirm = Some(idx);
                    self.catalog.status =
                        "Tap Delete again to confirm removing this rendered MP4.".into();
                }
            }
            Some(Action::PlayReplay(idx)) => {
                if let Some(item) = self.catalog.replays.get(idx) {
                    self.pending_replay = Some(item.path.clone());
                    self.action(events)?;
                }
            }
            Some(Action::DeleteReplay(idx)) => {
                if self.catalog.replay_delete_confirm == Some(idx) {
                    if let Some(item) = self.catalog.replays.get(idx) {
                        let name = item.display_name.clone();
                        match horizon_client::replay::delete_replay(&item.path) {
                            Ok(()) => {
                                self.catalog.refresh_replays(&self.home);
                                self.catalog.status = format!("Deleted replay: {name}");
                            }
                            Err(err) => {
                                self.error = Some(format!("Delete failed: {err}"));
                            }
                        }
                    }
                    self.catalog.replay_delete_confirm = None;
                } else {
                    self.catalog.replay_delete_confirm = Some(idx);
                    self.catalog.status = "Tap Delete again to confirm removing this .mcpr file.".into();
                }
            }
            Some(Action::ReplayToggleAutoSp) => {
                let game_home = self.catalog.config.game_home(&self.home);
                horizon_client::replay::update_settings(&game_home, |s| {
                    s.auto_record_sp = !s.auto_record_sp
                });
            }
            Some(Action::ReplayToggleAutoMp) => {
                let game_home = self.catalog.config.game_home(&self.home);
                horizon_client::replay::update_settings(&game_home, |s| {
                    s.auto_record_mp = !s.auto_record_mp
                });
            }
            Some(Action::ReplayToggleRecordSelf) => {
                let game_home = self.catalog.config.game_home(&self.home);
                horizon_client::replay::update_settings(&game_home, |s| {
                    s.record_self = !s.record_self
                });
            }
            Some(Action::WardrobeKind(cape)) => {
                self.wardrobe.set_tab(cape);
                self.catalog.scroll = 0.0;
            }
            Some(Action::WardrobeSelect(index)) => self
                .wardrobe
                .select_and_sync(index, &self.catalog.config.microsoft_client_id),
            Some(Action::WardrobeImport) => self.wardrobe.import(),
            Some(Action::WardrobeCloud) => self
                .wardrobe
                .cloud(&self.catalog.config.microsoft_client_id, None),
            Some(Action::WardrobeUpload) => self.wardrobe.cloud(
                &self.catalog.config.microsoft_client_id,
                Some(wardrobe::CloudAction::Upload),
            ),
            Some(Action::WardrobeActivateCape(index)) => self.wardrobe.cloud(
                &self.catalog.config.microsoft_client_id,
                Some(wardrobe::CloudAction::Cape(index)),
            ),
            Some(Action::WardrobeUseCloud) => self.wardrobe.use_cloud(),
            Some(Action::WardrobeModel) => self
                .wardrobe
                .model_and_sync(&self.catalog.config.microsoft_client_id),
            Some(Action::WardrobeRotate(direction)) => {
                self.wardrobe.camera.rotate(-direction as f32 * 25.0, 0.0);
                self.wardrobe.preview();
            }
            Some(Action::WardrobeZoom(inward)) => {
                self.wardrobe
                    .camera
                    .zoom(if inward { 1.15 } else { 1.0 / 1.15 });
                self.wardrobe.preview();
            }
            Some(Action::WardrobeHideCape) => self
                .wardrobe
                .hide_cape_and_sync(&self.catalog.config.microsoft_client_id),
            Some(Action::Search) => {
                crate::platform::hide();
                self.catalog.offset = 0;
                self.catalog.scroll = 0.0;
                self.catalog.search(&self.home);
            }
            Some(Action::FinishNameEdit) => {
                if self.catalog.editing == Some("name") {
                    self.refresh_wardrobe_account();
                }
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.config.touch_name_changed();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::Field(name)) => {
                if let Some(value) = self.login.field(name) {
                    self.catalog.editing = Some(name);
                    if name == "ely_password" || name == "ely_otp" {
                        crate::platform::show_secret(value);
                    } else {
                        crate::platform::show(value);
                    }
                    return Ok(());
                }
                if name == "render_scale" {
                    self.catalog.render_scale_text =
                        self.catalog.config.render_scale.percent().to_string();
                    self.chooser = None;
                }
                self.catalog.editing = Some(name);
                let initial = match name {
                    "name" => &self.catalog.config.name,
                    "profile_name" => &self.catalog.config.custom_profile_name,
                    "profile_desc" => &self.catalog.config.custom_profile_desc,
                    "server" => &self.catalog.config.server,
                    "render_scale" => &self.catalog.render_scale_text,
                    "client_id" => &self.catalog.config.microsoft_client_id,
                    "theme_bg" => &self.catalog.config.theme.custom_bg_hex,
                    "theme_surface" => &self.catalog.config.theme.custom_surface_hex,
                    "theme_accent" => &self.catalog.config.theme.custom_accent_hex,
                    "theme_grad_start" => &self.catalog.config.theme.gradient_start_hex,
                    "theme_grad_mid" => &self.catalog.config.theme.gradient_mid_hex,
                    "theme_grad_end" => &self.catalog.config.theme.gradient_end_hex,
                    "installed_search" => &self.catalog.installed_query,
                    "compiler_query" => &self.catalog.compiler_query,
                    "fm_rename" | "fm_new_folder" => &self.catalog.fm_input_text,
                    "add_server_name" => &self.catalog.add_server_name,
                    "add_server_ip" => &self.catalog.add_server_ip,
                    "world_search_query" => &self.catalog.world_search_query,
                    "music_query" => &self.catalog.music_query,
                    "skin_online_search" => &self.catalog.skin_online_query,
                    "music_new_playlist" => &self.catalog.music_new_playlist_name,
                    _ => &self.catalog.query,
                };
                crate::platform::show(initial);
            }
            Some(Action::Install(i)) => self.catalog.install(&self.home, i),
            Some(Action::Next) => {
                let is_horizon_paged = self.catalog.page == Page::Mods
                    || (self.catalog.page == Page::Shaders
                        && self.catalog.config.shader_source
                            == crate::shader_catalog::Origin::Horizon);
                if is_horizon_paged {
                    let bundled_count = if self.catalog.page == Page::Mods {
                        let query = self.catalog.query.trim().to_lowercase();
                        horizon_client::mod_list::ANDROID_BUNDLED
                            .iter()
                            .filter(|(id, title, _)| {
                                query.is_empty()
                                    || id.contains(&query)
                                    || title.to_lowercase().contains(&query)
                            })
                            .count()
                    } else {
                        0
                    };
                    let cat_count = self
                        .catalog
                        .results
                        .iter()
                        .filter(|p| {
                            !(self.catalog.page == Page::Mods
                                && p.source == crate::browser::SourceId::Horizon
                                && horizon_client::mod_list::ANDROID_BUNDLED
                                    .iter()
                                    .any(|(id, _, _)| *id == p.id))
                        })
                        .count();
                    let total = bundled_count + cat_count;
                    if self.catalog.offset + crate::catalog::CATALOG_PAGE_SIZE < total {
                        self.catalog.offset += crate::catalog::CATALOG_PAGE_SIZE;
                        self.catalog.scroll = 0.0;
                        self.catalog.fetch_icons(&self.home);
                    }
                } else if self.catalog.results.len() >= crate::catalog::CATALOG_PAGE_SIZE {
                    self.catalog.offset += crate::catalog::CATALOG_PAGE_SIZE;
                    self.catalog.scroll = 0.0;
                    self.catalog.search(&self.home);
                }
            }
            Some(Action::Previous) => {
                if self.catalog.offset >= crate::catalog::CATALOG_PAGE_SIZE {
                    self.catalog.offset -= crate::catalog::CATALOG_PAGE_SIZE;
                    self.catalog.scroll = 0.0;
                    let is_horizon_paged = self.catalog.page == Page::Mods
                        || (self.catalog.page == Page::Shaders
                            && self.catalog.config.shader_source
                                == crate::shader_catalog::Origin::Horizon);
                    if !is_horizon_paged {
                        self.catalog.search(&self.home);
                    } else {
                        self.catalog.fetch_icons(&self.home);
                    }
                }
            }
            Some(Action::EquipWallpaper(i)) => {
                if self.wallpaper_downloaded.get(i).copied().unwrap_or(false)
                    || crate::wallpapers::is_downloaded(&self.home, i)
                {
                    if i < self.wallpaper_downloaded.len() {
                        self.wallpaper_downloaded[i] = true;
                    }
                    self.equip_wallpaper(i)?;
                } else {
                    if i < self.wallpaper_downloaded.len() {
                        self.wallpaper_downloaded[i] = false;
                    }
                    if self.wallpaper_worker.is_none() {
                        self.begin_wallpaper_download(i);
                    } else if self.wallpaper_downloading != Some(i) {
                        self.wallpaper_queued_download = Some(i);
                        self.wallpaper_status = format!(
                            "Queued {} after current download...",
                            crate::wallpapers::WALLPAPERS[i].name
                        );
                    }
                }
            }
            Some(Action::ClearWallpaper) => {
                crate::native_ime::wallpaper_stop();
                self.catalog.config.custom_wallpaper_path = None;
                self.catalog.config.custom_wallpaper_is_video = false;
                self.custom_photo_rgba = None;
                self.custom_photo_loaded_path = None;
                self.custom_photo_dirty = false;
                self.catalog.config.wallpaper_index = None;
                self.catalog.config.save(&self.home)?;
                self.wallpaper_has_frame = false;
                self.wallpaper_queued_download = None;
                if let Some((base, accent)) =
                    self.wallpaper_preview_palettes[crate::wallpapers::DEFAULT_INDEX]
                {
                    self.wallpaper_color = Some(base);
                    self.wallpaper_accent = Some(accent);
                } else {
                    self.wallpaper_color = None;
                    self.wallpaper_accent = None;
                }
                self.wallpaper_status = "Peaceful greenhouse selected".into();
                self.start_wallpaper();
            }
            Some(Action::UploadCustomWallpaper) => {
                let dir = self.home.join("custom-wallpapers");
                let _ = std::fs::create_dir_all(&dir);
                self.custom_wallpaper_known.clear();
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_file() {
                            self.custom_wallpaper_known.insert(p);
                        }
                    }
                }
                match crate::documents::begin_into_dir(dir) {
                    Ok(()) => {
                        self.awaiting_custom_wallpaper = true;
                        self.wallpaper_status = "Choose a photo or video in Android Files...".into();
                    }
                    Err(e) => {
                        self.awaiting_custom_wallpaper = false;
                        self.wallpaper_status = e;
                    }
                }
            }
            Some(Action::UiOpacityDelta(delta)) => {
                let cur = self.catalog.config.theme.ui_opacity.clamp(0.10, 1.0);
                let next = (cur + f32::from(delta) * 0.05).clamp(0.10, 1.0);
                let rounded = (next * 100.0).round() / 100.0;
                self.catalog.config.theme.ui_opacity = rounded;
                self.catalog.config.theme.sanitize();
                let resolved = self.catalog.config.theme.resolve();
                horizon_client::ui_theme::set_active_theme(resolved);
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::OpenWallpapers) => {
                self.wallpapers_open = true;
                self.refresh_wallpaper_downloads();
                self.ensure_wallpaper_previews();
            }
            Some(Action::CloseWallpapers) => {
                self.wallpapers_open = false;
            }
            Some(Action::DownloadWallpaper(i)) => {
                if i < crate::wallpapers::WALLPAPERS.len() {
                    if crate::wallpapers::is_downloaded(&self.home, i) {
                        self.wallpaper_downloaded[i] = true;
                        self.equip_wallpaper(i)?;
                    } else if self.wallpaper_worker.is_none() {
                        self.begin_wallpaper_download(i);
                    } else if self.wallpaper_downloading != Some(i) {
                        self.wallpaper_queued_download = Some(i);
                        self.wallpaper_status = format!(
                            "Queued {} after current download...",
                            crate::wallpapers::WALLPAPERS[i].name
                        );
                    }
                }
            }
            Some(Action::SceneGyro(direction)) => {
                self.catalog.config.scene_gyro_sensitivity =
                    (self.catalog.config.scene_gyro_sensitivity as i32 + i32::from(direction) * 25).clamp(0, 300) as u32;
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::Profile) => self.open_chooser(chooser::Kind::Profile),
            Some(Action::Renderer) => self.open_chooser(chooser::Kind::Renderer),
            Some(Action::RenderScale) => self.open_chooser(chooser::Kind::RenderScale),
            Some(Action::ChooseRenderScale(scale)) => {
                let pct = scale.percent();
                self.catalog.config.render_scale = scale;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.modify_game_settings(|s| {
                    s.horizon.resolution_scale = pct;
                });
                self.modify_touch_settings(|ts| {
                    ts.resolution_scale_percent = pct;
                });
            }
            Some(Action::Orientation) => self.open_chooser(chooser::Kind::Orientation),
            Some(Action::InputHost) => self.open_chooser(chooser::Kind::InputHost),
            Some(Action::ChooseInputHost(mode)) => {
                self.catalog.config.input_host = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::configure_input_mode(mode);
            }
            Some(Action::ChooseOrientation(mode)) => {
                self.catalog.config.orientation = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::platform::set_orientation(mode);
            }
            Some(Action::ChooseRenderer(mode)) => {
                self.catalog.config.renderer = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
            }
            Some(Action::ChooseProfile(profile)) => {
                std::fs::create_dir_all(if profile == 0 {
                    self.home.clone()
                } else {
                    self.home
                        .join("profiles")
                        .join(format!("profile-{profile}"))
                })?;
                self.catalog.config.profile = profile;
                self.catalog.config.save(&self.home)?;
                self.catalog.refresh(&self.home);
                self.chooser = None;
            }
            Some(Action::Dismiss) => {
                self.exit_confirmation = false;
                self.chooser = None;
                if self.session_worker.is_none() {
                    self.session_ready = false;
                    self.prepared_session = None;
                }
            }
            Some(Action::Logs) => {
                self.catalog.status = crate::diagnostics::summary(&self.home);
                if let Ok(error) = std::fs::read_to_string(self.home.join("android-error.txt")) {
                    self.catalog.status = format!(
                        "Last handled error: {}. {}",
                        crate::diagnostics::redact(&error),
                        self.catalog.status
                    );
                }
                self.diagnostics_modal_open = true;
            }
            Some(Action::CloseDiagnosticsSummary) => {
                self.diagnostics_modal_open = false;
            }
            Some(Action::OpenReportsFolder) => {
                self.diagnostics_modal_open = false;
                self.catalog.status = if crate::platform::open_documents("reports") {
                    "Opened Reports folder in system Files.".into()
                } else {
                    "Reports are saved under Volt Plus > Reports.".into()
                };
            }
            Some(Action::ToggleLauncherAnimations) => {
                self.catalog.config.disable_launcher_animations =
                    !self.catalog.config.disable_launcher_animations;
                horizon_client::ui_theme::motion::set_launcher_animations_disabled(
                    self.catalog.config.disable_launcher_animations,
                );
                self.catalog.config.save(&self.home)?;
                self.catalog.status = if self.catalog.config.disable_launcher_animations {
                    "Launcher animations disabled (instant page & modal transitions)."
                } else {
                    "Launcher animations enabled."
                }
                .into();
            }
            Some(Action::ToggleGameAnimations) => {
                let next = !self.catalog.config.disable_game_animations;
                self.catalog.config.disable_game_animations = next;
                horizon_client::ui_theme::motion::set_game_animations_disabled(next);
                self.catalog.config.save(&self.home)?;
                self.catalog.config.apply_experimental_to_game(&self.home);
                self.modify_touch_settings(|t| t.disable_game_animations = next);
                self.catalog.status = if next {
                    "In-game animations disabled (static textures, instant FOV/menus, no view bobbing)."
                } else {
                    "In-game animations enabled."
                }
                .into();
            }
            Some(Action::ExportDiagnostics) if self.diagnostic_export.is_none() => {
                let home = self.home.clone();
                let (sender, receiver) = mpsc::channel();
                self.diagnostic_export = Some(receiver);
                self.catalog.status =
                    "Preparing local diagnostic ZIP. No upload; review before sharing.".into();
                std::thread::spawn(move || {
                    let result = crate::diagnostics::export(&home).map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    });
                    let _ = sender.send(result);
                });
            }
            Some(Action::ExportDiagnostics) => {}
            Some(Action::RecordPerformance) => {
                self.catalog.config.record_performance = !self.catalog.config.record_performance;
                if !self.catalog.config.record_performance {
                    crate::performance::stop();
                }
                self.catalog.config.save(&self.home)?;
                self.catalog.status = if self.catalog.config.record_performance {
                    "Recording starts on Play. Detailed timings add overhead. Return here to save the report."
                } else { "Performance recording off." }.into();
            }
            Some(Action::ExportPerformance) => {
                self.catalog.status = match crate::performance::export(&self.home) {
                    Ok(()) => "Reports opened in Files. Copy or share a report from there.".into(),
                    Err(e) => e,
                };
            }
            Some(Action::SelectThemePreset(preset)) => {
                self.catalog.config.theme.apply_preset(preset);
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
                self.catalog.status = format!("Applied '{}' theme.", preset.label());
            }
            Some(Action::SetThemeAppearance(appearance)) => {
                self.catalog.config.theme.appearance = appearance;
                if self.catalog.config.theme.preset
                    == horizon_client::ui_theme::ThemePreset::DaylightFrost
                    && appearance == horizon_client::ui_theme::ThemeAppearance::Dark
                {
                    self.catalog.config.theme.preset =
                        horizon_client::ui_theme::ThemePreset::EmeraldForest;
                }
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
                self.catalog.status = format!("Switched to {}.", appearance.label());
            }
            Some(Action::SetThemeBackgroundMode(mode)) => {
                let was_custom = self.catalog.config.theme.background_mode
                    == horizon_client::ui_theme::BackgroundMode::Custom;
                self.catalog.config.theme.background_mode = mode;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
                self.catalog.status = format!("Background style set to {}.", mode.label());
                // Custom GIF: pick a file the first time, or when Custom is tapped again.
                if mode == horizon_client::ui_theme::BackgroundMode::Custom
                    && (was_custom
                        || !(horizon_client::ui_theme::custom_background_loaded()
                            || custom_background_file_exists(&self.home)))
                {
                    match crate::documents::begin_into_dir(self.home.join("backgrounds")) {
                        Ok(()) => {
                            self.catalog.status =
                                "Choose a GIF for your custom background (photos are ignored)."
                                    .into();
                        }
                        Err(error) => self.catalog.status = error,
                    }
                }
            }
            Some(Action::SetGradientDirection(dir)) => {
                self.catalog.config.theme.gradient_direction = dir;
                self.catalog.config.theme.background_mode =
                    horizon_client::ui_theme::BackgroundMode::Gradient;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
                self.catalog.status = format!("Gradient direction set to {}.", dir.label());
            }
            Some(Action::ToggleGradientThreeStops) => {
                self.catalog.config.theme.use_three_stops =
                    !self.catalog.config.theme.use_three_stops;
                self.catalog.config.theme.background_mode =
                    horizon_client::ui_theme::BackgroundMode::Gradient;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::StepGradientMidPos(dir)) => {
                let cur = self.catalog.config.theme.gradient_mid_pos;
                let next = if dir >= 0 {
                    if cur < 0.35 {
                        0.50
                    } else if cur < 0.65 {
                        0.75
                    } else {
                        0.25
                    }
                } else if cur > 0.65 {
                    0.50
                } else {
                    0.25
                };
                self.catalog.config.theme.gradient_mid_pos = next;
                self.catalog.config.theme.background_mode =
                    horizon_client::ui_theme::BackgroundMode::Gradient;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::SelectAccentSwatch(rgb)) => {
                self.catalog.config.theme.custom_accent_hex =
                    horizon_client::ui_theme::format_hex_color(rgb);
                self.catalog.config.theme.preset = horizon_client::ui_theme::ThemePreset::Custom;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::SelectBgSwatch(rgb)) => {
                self.catalog.config.theme.custom_bg_hex =
                    horizon_client::ui_theme::format_hex_color(rgb);
                self.catalog.config.theme.preset = horizon_client::ui_theme::ThemePreset::Custom;
                self.catalog.config.theme.background_mode =
                    horizon_client::ui_theme::BackgroundMode::Solid;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::SelectGradStartSwatch(rgb)) => {
                self.catalog.config.theme.gradient_start_hex =
                    horizon_client::ui_theme::format_hex_color(rgb);
                self.catalog.config.theme.preset = horizon_client::ui_theme::ThemePreset::Custom;
                self.catalog.config.theme.background_mode =
                    horizon_client::ui_theme::BackgroundMode::Gradient;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::SelectGradEndSwatch(rgb)) => {
                self.catalog.config.theme.gradient_end_hex =
                    horizon_client::ui_theme::format_hex_color(rgb);
                self.catalog.config.theme.preset = horizon_client::ui_theme::ThemePreset::Custom;
                self.catalog.config.theme.background_mode =
                    horizon_client::ui_theme::BackgroundMode::Gradient;
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::ResetTheme) => {
                self.catalog.config.theme =
                    horizon_client::ui_theme::LauncherThemeConfig::default();
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                self.anim.trigger_theme_pulse();
                self.catalog.config.save(&self.home)?;
                self.catalog.status = "Theme reset to Horizon default (Obsidian Violet).".into();
            }
            Some(Action::SkipIntro) => {
                self.anim.intro_done = true;
            }
            Some(Action::SetInstalledSubTab(tab)) => {
                self.catalog.installed_sub_tab = tab;
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.scroll = 0.0;
                if tab == crate::catalog::InstalledSubTab::FileManager {
                    self.catalog.refresh_fm(&self.home);
                }
                self.anim.trigger_page_transition();
            }
            Some(Action::SetInstalledFilter(filter)) => {
                self.catalog.installed_filter = filter;
                self.catalog.scroll = 0.0;
                self.anim.trigger_page_transition();
            }
            Some(Action::CycleInstalledSort) => {
                self.catalog.installed_sort = self.catalog.installed_sort.next();
                self.catalog.sort_local_items();
                self.catalog.scroll = 0.0;
            }
            Some(Action::ExportLocal(idx)) => {
                if let Some(item) = self.catalog.local.get(idx).cloned() {
                    let game_home = self.catalog.config.game_home(&self.home);
                    crate::document_store::sync_mods_folder(&game_home);
                    let path = if let Some(bid) = item.file.strip_prefix("builtin:") {
                        game_home.join("horizon-mods").join(format!("{bid}.mod.json"))
                    } else {
                        let direct = game_home.join(&item.file);
                        if direct.exists() {
                            direct
                        } else {
                            game_home.join(item.kind.folder()).join(&item.file)
                        }
                    };
                    if path.is_dir() {
                        match crate::documents::export_folder_prepared(
                            &self.home,
                            self.catalog.config.profile,
                            &path,
                        ) {
                            Ok((zip_path, _, msg)) => {
                                self.catalog.fm_clipboard = Some((vec![zip_path], false));
                                self.catalog.status = msg;
                            }
                            Err(e) => self.catalog.status = e,
                        }
                    } else {
                        self.catalog.fm_clipboard = Some((vec![path.clone()], false));
                        self.catalog.status = match crate::documents::export_file(
                            &self.home,
                            self.catalog.config.profile,
                            &path,
                        ) {
                            Ok(msg) => msg,
                            Err(e) => e,
                        };
                    }
                }
            }
            Some(Action::FmNavigateRoot(rel)) => {
                self.catalog.fm_navigate(&self.home, (*rel).to_owned());
                self.anim.trigger_page_transition();
            }
            Some(Action::FmBreadcrumb(depth)) => {
                if depth == 0 {
                    self.catalog.fm_navigate(&self.home, String::new());
                } else {
                    let target = self
                        .catalog
                        .fm_current_dir
                        .split('/')
                        .filter(|s| !s.is_empty())
                        .take(depth)
                        .collect::<Vec<_>>()
                        .join("/");
                    self.catalog.fm_navigate(&self.home, target);
                }
            }
            Some(Action::FmBack) => {
                let _ = self.catalog.fm_back(&self.home);
            }
            Some(Action::FmForward) => {
                let _ = self.catalog.fm_forward(&self.home);
            }
            Some(Action::FmUp) => {
                self.catalog.fm_up(&self.home);
            }
            Some(Action::FmOpenEntry(idx)) => {
                if let Some(entry) = self.catalog.fm_entries.get(idx).cloned() {
                    if entry.is_dir {
                        self.catalog.fm_navigate(&self.home, entry.rel_path);
                    } else {
                        let game_home = self.catalog.config.game_home(&self.home);
                        match crate::document_store::fm_details(
                            &self.home,
                            &game_home,
                            &entry.rel_path,
                        ) {
                            Ok(details) => {
                                self.catalog.fm_modal =
                                    Some(crate::catalog::FmModal::Details(details));
                            }
                            Err(e) => self.catalog.status = e,
                        }
                    }
                }
            }
            Some(Action::FmToggleSelect(idx)) => {
                if let Some(entry) = self.catalog.fm_entries.get(idx) {
                    let rel = entry.rel_path.clone();
                    if !self.catalog.fm_selected.remove(&rel) {
                        self.catalog.fm_selected.insert(rel);
                    }
                }
            }
            Some(Action::FmSelectAll) => {
                for entry in &self.catalog.fm_entries {
                    self.catalog.fm_selected.insert(entry.rel_path.clone());
                }
            }
            Some(Action::FmClearSelection) => {
                self.catalog.fm_selected.clear();
            }
            Some(Action::FmCopy) => {
                self.catalog.fm_copy_or_cut(&self.home, false);
            }
            Some(Action::FmCut) => {
                self.catalog.fm_copy_or_cut(&self.home, true);
            }
            Some(Action::FmPaste) => {
                self.catalog.fm_paste_here(&self.home);
            }
            Some(Action::FmImportHere) => {
                let game_home = self.catalog.config.game_home(&self.home);
                match crate::document_store::resolve_fm_path(
                    &self.home,
                    &game_home,
                    &self.catalog.fm_current_dir,
                ) {
                    Ok(target_dir) => {
                        if let Err(e) = crate::documents::begin_into_dir(target_dir) {
                            self.catalog.status = e;
                        }
                    }
                    Err(e) => self.catalog.status = e,
                }
            }
            Some(Action::FmImportFolderHere) => {
                let game_home = self.catalog.config.game_home(&self.home);
                match crate::document_store::resolve_fm_path(
                    &self.home,
                    &game_home,
                    &self.catalog.fm_current_dir,
                ) {
                    Ok(target_dir) => {
                        if let Err(e) = crate::documents::begin_folder_into_dir(target_dir) {
                            self.catalog.status = e;
                        }
                    }
                    Err(e) => self.catalog.status = e,
                }
            }
            Some(Action::FmOpenNewFolder) => {
                self.catalog.fm_input_text = "New Folder".into();
                self.catalog.fm_modal = Some(crate::catalog::FmModal::NewFolder);
                self.catalog.editing = Some("fm_new_folder");
                crate::platform::show(&self.catalog.fm_input_text);
            }
            Some(Action::FmSubmitNewFolder) => {
                self.catalog.fm_submit_new_folder(&self.home);
            }
            Some(Action::FmOpenRename(idx)) => {
                if let Some(entry) = self.catalog.fm_entries.get(idx).cloned() {
                    if crate::document_store::is_protected_root_folder(&entry.rel_path) {
                        self.catalog.status =
                            "Built-in profile root folders cannot be renamed.".into();
                    } else {
                        self.catalog.fm_input_text = entry.name;
                        self.catalog.fm_modal = Some(crate::catalog::FmModal::Rename {
                            target_rel: entry.rel_path,
                        });
                        self.catalog.editing = Some("fm_rename");
                        crate::platform::show(&self.catalog.fm_input_text);
                    }
                }
            }
            Some(Action::FmOpenRenameSelected) => {
                if let Some(rel) = self.catalog.fm_selected.iter().next().cloned() {
                    if crate::document_store::is_protected_root_folder(&rel) {
                        self.catalog.status =
                            "Built-in profile root folders cannot be renamed.".into();
                    } else {
                        let cur_name = rel.rsplit_once('/').map_or(rel.as_str(), |(_, n)| n);
                        self.catalog.fm_input_text = cur_name.to_owned();
                        self.catalog.fm_modal =
                            Some(crate::catalog::FmModal::Rename { target_rel: rel });
                        self.catalog.editing = Some("fm_rename");
                        crate::platform::show(&self.catalog.fm_input_text);
                    }
                } else {
                    self.catalog.status = "Select a single file or folder to rename.".into();
                }
            }
            Some(Action::FmSubmitRename) => {
                self.catalog.fm_submit_rename(&self.home);
            }
            Some(Action::FmRequestDeleteSingle(idx)) => {
                if let Some(entry) = self.catalog.fm_entries.get(idx).cloned() {
                    if crate::document_store::is_protected_root_folder(&entry.rel_path) {
                        self.catalog.status =
                            "Built-in profile root folders cannot be deleted.".into();
                    } else {
                        self.catalog.fm_modal = Some(crate::catalog::FmModal::ConfirmDelete {
                            paths: vec![entry.rel_path],
                        });
                    }
                }
            }
            Some(Action::FmRequestDeleteSelected) => {
                if self.catalog.fm_selected.is_empty() {
                    self.catalog.status = "Select one or more items to delete.".into();
                } else {
                    let paths: Vec<String> = self
                        .catalog
                        .fm_selected
                        .iter()
                        .filter(|p| !crate::document_store::is_protected_root_folder(p))
                        .cloned()
                        .collect();
                    if paths.is_empty() {
                        self.catalog.status =
                            "Built-in profile root folders cannot be deleted.".into();
                    } else {
                        self.catalog.fm_modal =
                            Some(crate::catalog::FmModal::ConfirmDelete { paths });
                    }
                }
            }
            Some(Action::FmConfirmDelete) => {
                self.catalog.fm_confirm_delete(&self.home);
            }
            Some(Action::FmOpenDetails(idx)) => {
                if let Some(entry) = self.catalog.fm_entries.get(idx).cloned() {
                    let game_home = self.catalog.config.game_home(&self.home);
                    match crate::document_store::fm_details(&self.home, &game_home, &entry.rel_path)
                    {
                        Ok(details) => {
                            self.catalog.fm_modal = Some(crate::catalog::FmModal::Details(details));
                        }
                        Err(e) => self.catalog.status = e,
                    }
                }
            }
            Some(Action::FmExportSingle(idx)) => {
                self.catalog.fm_modal = None;
                if let Some(entry) = self.catalog.fm_entries.get(idx).cloned() {
                    let game_home = self.catalog.config.game_home(&self.home);
                    match crate::document_store::resolve_fm_path(
                        &self.home,
                        &game_home,
                        &entry.rel_path,
                    ) {
                        Ok(path) => {
                            if path.is_dir() {
                                match crate::documents::export_folder_prepared(
                                    &self.home,
                                    self.catalog.config.profile,
                                    &path,
                                ) {
                                    Ok((zip_path, _, msg)) => {
                                        self.catalog.fm_clipboard = Some((vec![zip_path], false));
                                        self.catalog.status = msg;
                                    }
                                    Err(e) => self.catalog.status = e,
                                }
                            } else {
                                self.catalog.fm_clipboard = Some((vec![path.clone()], false));
                                self.catalog.status = match crate::documents::export_file(
                                    &self.home,
                                    self.catalog.config.profile,
                                    &path,
                                ) {
                                    Ok(msg) => msg,
                                    Err(e) => e,
                                };
                            }
                        }
                        Err(e) => self.catalog.status = e,
                    }
                }
            }
            Some(Action::FmExportSelected) => {
                let target_rel = self.catalog.fm_selected.iter().next().cloned().or_else(|| {
                    (!self.catalog.fm_current_dir.is_empty())
                        .then(|| self.catalog.fm_current_dir.clone())
                });
                if let Some(rel) = target_rel {
                    let game_home = self.catalog.config.game_home(&self.home);
                    match crate::document_store::resolve_fm_path(&self.home, &game_home, &rel) {
                        Ok(path) => {
                            if path.is_dir() {
                                match crate::documents::export_folder_prepared(
                                    &self.home,
                                    self.catalog.config.profile,
                                    &path,
                                ) {
                                    Ok((zip_path, _, msg)) => {
                                        self.catalog.fm_clipboard = Some((vec![zip_path], false));
                                        self.catalog.status = msg;
                                    }
                                    Err(e) => self.catalog.status = e,
                                }
                            } else {
                                self.catalog.fm_clipboard = Some((vec![path.clone()], false));
                                self.catalog.status = match crate::documents::export_file(
                                    &self.home,
                                    self.catalog.config.profile,
                                    &path,
                                ) {
                                    Ok(msg) => msg,
                                    Err(e) => e,
                                };
                            }
                        }
                        Err(e) => self.catalog.status = e,
                    }
                } else {
                    self.catalog.status = "Select a file or folder (or tap Export on any row) to export.".into();
                }
            }
            Some(Action::OpenUpdateScreen) => {
                self.catalog.update_ui_open = true;
                self.catalog.update_anim_time = 0.0;
            }
            Some(Action::CloseUpdateScreen) => {
                self.catalog.update_ui_open = false;
                crate::updater::dismiss();
            }
            Some(Action::TriggerUpdateInstall) => {
                let status = crate::updater::get_status();
                match status {
                    crate::updater::UpdateStatus::Available(info) => {
                        crate::updater::download_and_install_in_background(info, self.home.clone());
                    }
                    crate::updater::UpdateStatus::Downloaded { path, .. } => {
                        crate::updater::trigger_install(&path);
                    }
                    crate::updater::UpdateStatus::Error(_) => {
                        if let Some(info) = self.catalog.update_info.clone() {
                            crate::updater::download_and_install_in_background(
                                info,
                                self.home.clone(),
                            );
                        }
                    }
                    _ => {}
                }
            }
            Some(Action::CheckForUpdates) => {
                self.catalog.status = "Checking for updates...".into();
                crate::updater::check_in_background();
            }
            Some(Action::FmCloseModal) => {
                self.catalog.fm_modal = None;
                self.catalog.editing = None;
                crate::platform::hide();
            }
            Some(Action::OpenMusic) => {
                if self.catalog.page != Page::Music {
                    self.catalog.music_return_page = self.catalog.page;
                }
                self.catalog.page = Page::Music;
                self.catalog.scroll = 0.0;
                self.catalog.editing = None;
                crate::platform::hide();
                self.sync_discord_presence();
                if !self.catalog.music_fetched_initial {
                    self.catalog.search_music(&self.home);
                }
            }
            Some(Action::CloseMusic) => {
                self.catalog.music_save_modal = None;
                self.catalog.music_sort_modal = false;
                self.catalog.music_downloads_modal = false;
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.page = self.catalog.music_return_page;
                self.catalog.scroll = 0.0;
                self.anim.trigger_page_transition();
                self.sync_discord_presence();
            }
            Some(Action::OpenSkinOnline) => {
                self.login.hide();
                if self.catalog.page != Page::SkinOnline {
                    self.catalog.skin_online_return_page = self.catalog.page;
                }
                self.catalog.page = Page::SkinOnline;
                self.catalog.scroll = 0.0;
                self.catalog.editing = None;
                crate::platform::hide();
                self.refresh_wardrobe_account();
                self.skin_online.open_initial();
                self.sync_discord_presence();
            }
            Some(Action::CloseSkinOnline) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.page = self.catalog.skin_online_return_page;
                self.catalog.scroll = 0.0;
                self.anim.trigger_page_transition();
                self.sync_discord_presence();
            }
            Some(Action::SkinOnlineTab(capes)) => {
                let tab = if capes {
                    super::skin_online::Tab::Capes
                } else {
                    super::skin_online::Tab::Skins
                };
                self.skin_online.set_tab(tab, &self.catalog.skin_online_query);
                if capes && (self.wardrobe.camera.yaw < 70.0 || self.wardrobe.camera.yaw > 290.0) {
                    self.wardrobe.camera.yaw = 155.0;
                    self.wardrobe.camera.pitch = -6.0;
                    self.wardrobe.preview();
                } else if !capes && self.wardrobe.camera.yaw >= 90.0 && self.wardrobe.camera.yaw <= 270.0 {
                    self.wardrobe.camera.yaw = 335.0;
                    self.wardrobe.camera.pitch = -8.0;
                    self.wardrobe.preview();
                }
            }
            Some(Action::SkinOnlineSkinCategory(cat)) => {
                self.skin_online.skin_cat = cat;
                self.skin_online.page = 1;
                self.skin_online.scroll = 0.0;
                self.skin_online.fetch_current(self.catalog.skin_online_query.clone());
            }
            Some(Action::SkinOnlineCapeFilter(filter)) => {
                self.skin_online.cape_filter = filter;
                self.skin_online.scroll = 0.0;
            }
            Some(Action::SkinOnlineCapeSource(source)) => {
                self.skin_online.cape_source = source;
                self.skin_online.scroll = 0.0;
            }
            Some(Action::SkinOnlineSearch) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.skin_online.search(self.catalog.skin_online_query.clone());
            }
            Some(Action::SkinOnlineRefresh) => {
                self.skin_online.fetch_current(self.catalog.skin_online_query.clone());
            }
            Some(Action::SkinOnlinePage(delta)) => {
                self.skin_online.next_page(delta, &self.catalog.skin_online_query);
            }
            Some(Action::SkinOnlineEquip(index)) => {
                let client_id = self.catalog.config.microsoft_client_id.clone();
                self.skin_online.equip_item(index, &mut self.wardrobe, &client_id);
            }
            Some(Action::SkinOnlineToggleWavey) => {
                self.wardrobe.toggle_wavey_capes();
            }
            Some(Action::SkinOnlineCycleFps) => {
                self.wardrobe.cycle_cape_fps();
            }
            Some(Action::MusicTab(playlists_tab)) => {
                self.catalog.music_tab_playlists = playlists_tab;
                self.catalog.scroll = 0.0;
                if playlists_tab {
                    horizon_client::internet_music::invalidate_offline_cache();
                    self.catalog.fetch_music_icons(&self.home);
                }
            }
            Some(Action::MusicSearch) => {
                self.catalog.music_tab_playlists = false;
                self.catalog.music_page = 0;
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.search_music(&self.home);
            }
            Some(Action::MusicQuickQuery(q)) => {
                self.catalog.music_query = q.to_owned();
                self.catalog.music_tab_playlists = false;
                self.catalog.music_page = 0;
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.search_music(&self.home);
            }
            Some(Action::MusicSelectGenre(chip)) => {
                if chip.eq_ignore_ascii_case("Trending") {
                    self.catalog.music_genre = None;
                } else if self
                    .catalog
                    .music_genre
                    .as_deref()
                    .is_some_and(|g| g.eq_ignore_ascii_case(chip))
                {
                    self.catalog.music_genre = None;
                } else {
                    self.catalog.music_genre = Some(chip.to_owned());
                }
                self.catalog.music_tab_playlists = false;
                self.catalog.music_page = 0;
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.search_music(&self.home);
            }
            Some(Action::MusicOpenSort) => {
                self.catalog.music_sort_modal = true;
                self.catalog.music_downloads_modal = false;
                self.catalog.music_save_modal = None;
                self.catalog.editing = None;
                crate::platform::hide();
            }
            Some(Action::MusicCloseSort) => {
                self.catalog.music_sort_modal = false;
            }
            Some(Action::MusicSetSort(sort)) => {
                self.catalog.music_sort = sort;
                self.catalog.music_sort_modal = false;
                self.catalog.music_page = 0;
                self.catalog.search_music(&self.home);
            }
            Some(Action::MusicPrevPage) => {
                if self.catalog.music_page > 0 {
                    self.catalog.music_page -= 1;
                    self.catalog.search_music(&self.home);
                }
            }
            Some(Action::MusicNextPage) => {
                if self.catalog.music_has_next {
                    self.catalog.music_page += 1;
                    self.catalog.search_music(&self.home);
                }
            }
            Some(Action::MusicDownloadResult(idx)) => {
                if let Some(track) = self.catalog.music_results.get(idx).cloned() {
                    if horizon_client::internet_music::is_track_downloaded(&track) {
                        self.catalog.music_status =
                            format!("Already downloaded \"{}\"", track.title);
                    } else {
                        self.catalog.music_status =
                            format!("Downloading \"{}\"...", track.title);
                        horizon_client::internet_music::start_download_track(track);
                    }
                }
            }
            Some(Action::MusicOpenDownloads) => {
                self.catalog.music_downloads_modal = true;
                self.catalog.music_sort_modal = false;
                self.catalog.music_save_modal = None;
                self.catalog.editing = None;
                crate::platform::hide();
            }
            Some(Action::MusicCloseDownloads) => {
                self.catalog.music_downloads_modal = false;
            }
            Some(Action::MusicClearFinishedDownloads) => {
                horizon_client::internet_music::dismiss_finished_downloads();
            }
            Some(Action::MusicPlayResult(idx)) => {
                if !self.catalog.music_results.is_empty() && idx < self.catalog.music_results.len() {
                    horizon_client::internet_music::play_queue(
                        &self.catalog.music_results,
                        idx,
                        "",
                    );
                    self.sync_discord_presence();
                }
            }
            Some(Action::MusicSelectPlaylist(idx)) => {
                let pls = horizon_client::internet_music::playlists();
                if let Some(pl) = pls.get(idx) {
                    self.catalog.music_selected_playlist = Some(pl.name.clone());
                    self.catalog.scroll = 0.0;
                }
            }
            Some(Action::MusicPlayPlaylistTrack(idx)) => {
                let pls = horizon_client::internet_music::playlists();
                let selected = self
                    .catalog
                    .music_selected_playlist
                    .clone()
                    .filter(|n| pls.iter().any(|p| p.name == *n))
                    .or_else(|| pls.first().map(|p| p.name.clone()))
                    .unwrap_or_else(|| "Favorites".to_owned());
                horizon_client::internet_music::play_playlist(&selected, idx);
                self.sync_discord_presence();
            }
            Some(Action::MusicPlaySelectedPlaylist) => {
                let pls = horizon_client::internet_music::playlists();
                let selected = self
                    .catalog
                    .music_selected_playlist
                    .clone()
                    .filter(|n| pls.iter().any(|p| p.name == *n))
                    .or_else(|| pls.first().map(|p| p.name.clone()))
                    .unwrap_or_else(|| "Favorites".to_owned());
                if !horizon_client::internet_music::play_playlist(&selected, 0) {
                    self.catalog.music_status = format!("Playlist \"{selected}\" has no tracks yet.");
                }
                self.sync_discord_presence();
            }
            Some(Action::MusicDeleteSelectedPlaylist) => {
                let pls = horizon_client::internet_music::playlists();
                let selected = self
                    .catalog
                    .music_selected_playlist
                    .clone()
                    .filter(|n| pls.iter().any(|p| p.name == *n))
                    .or_else(|| pls.first().map(|p| p.name.clone()))
                    .unwrap_or_else(|| "Favorites".to_owned());
                horizon_client::internet_music::delete_playlist(&selected);
                self.catalog.music_selected_playlist = None;
                self.catalog.music_status = format!("Deleted playlist \"{selected}\"");
            }
            Some(Action::MusicRemoveTrackFromPlaylist(idx)) => {
                let pls = horizon_client::internet_music::playlists();
                let selected = self
                    .catalog
                    .music_selected_playlist
                    .clone()
                    .filter(|n| pls.iter().any(|p| p.name == *n))
                    .or_else(|| pls.first().map(|p| p.name.clone()))
                    .unwrap_or_else(|| "Favorites".to_owned());
                if let Some(pl) = pls.iter().find(|p| p.name == selected)
                    && let Some(track) = pl.tracks.get(idx)
                {
                    horizon_client::internet_music::remove_track_from_playlist(&selected, &track.id);
                }
            }
            Some(Action::MusicOpenSaveResult(idx)) => {
                if let Some(track) = self.catalog.music_results.get(idx).cloned() {
                    self.catalog.music_save_modal = Some(track);
                }
            }
            Some(Action::MusicOpenSavePlaylistTrack(idx)) => {
                let pls = horizon_client::internet_music::playlists();
                let selected = self
                    .catalog
                    .music_selected_playlist
                    .clone()
                    .filter(|n| pls.iter().any(|p| p.name == *n))
                    .or_else(|| pls.first().map(|p| p.name.clone()))
                    .unwrap_or_else(|| "Favorites".to_owned());
                if let Some(pl) = pls.iter().find(|p| p.name == selected)
                    && let Some(track) = pl.tracks.get(idx).cloned()
                {
                    self.catalog.music_save_modal = Some(track);
                }
            }
            Some(Action::MusicCloseSave) => {
                self.catalog.music_save_modal = None;
                self.catalog.editing = None;
                crate::platform::hide();
            }
            Some(Action::MusicAddToPlaylist(idx)) => {
                let pls = horizon_client::internet_music::playlists();
                if let Some(pl) = pls.get(idx)
                    && let Some(track) = self.catalog.music_save_modal.take()
                {
                    match horizon_client::internet_music::add_track_to_playlist(&pl.name, track) {
                        Ok(true) => {
                            self.catalog.music_status = format!("Added song to \"{}\"", pl.name);
                        }
                        Ok(false) => {
                            self.catalog.music_status =
                                format!("Already in \"{}\"", pl.name);
                        }
                        Err(e) => self.catalog.music_status = e,
                    }
                    self.catalog.editing = None;
                    crate::platform::hide();
                }
            }
            Some(Action::MusicCreatePlaylist) => {
                let name = self.catalog.music_new_playlist_name.trim().to_owned();
                match horizon_client::internet_music::create_playlist(&name) {
                    Ok(created) => {
                        self.catalog.music_selected_playlist = Some(created.clone());
                        self.catalog.music_new_playlist_name.clear();
                        self.catalog.music_status = format!("Created playlist \"{created}\"");
                        self.catalog.editing = None;
                        crate::platform::hide();
                    }
                    Err(e) => self.catalog.music_status = e,
                }
            }
            Some(Action::MusicCreateAndAdd) => {
                let name = self.catalog.music_new_playlist_name.trim().to_owned();
                match horizon_client::internet_music::create_playlist(&name) {
                    Ok(created) => {
                        if let Some(track) = self.catalog.music_save_modal.take() {
                            let _ = horizon_client::internet_music::add_track_to_playlist(
                                &created, track,
                            );
                            self.catalog.music_status =
                                format!("Created \"{created}\" and saved song");
                        }
                        self.catalog.music_selected_playlist = Some(created);
                        self.catalog.music_new_playlist_name.clear();
                        self.catalog.editing = None;
                        crate::platform::hide();
                    }
                    Err(e) => self.catalog.music_status = e,
                }
            }
            Some(Action::MusicTogglePause) => {
                horizon_client::internet_music::toggle_pause();
                self.sync_discord_presence();
            }
            Some(Action::MusicNext) => {
                horizon_client::internet_music::next_track();
                self.sync_discord_presence();
            }
            Some(Action::MusicPrev) => {
                horizon_client::internet_music::prev_track();
                self.sync_discord_presence();
            }
            Some(Action::MusicSeekBy(delta)) => {
                horizon_client::internet_music::seek_by_ms(delta);
            }
            Some(Action::MusicSeekFrac(frac)) => {
                let snap = horizon_client::internet_music::snapshot();
                if snap.duration_ms > 0 {
                    let target = ((snap.duration_ms as f32) * frac.clamp(0.0, 1.0)) as u64;
                    horizon_client::internet_music::seek_to_ms(target);
                }
            }
            Some(Action::MusicToggleMute) => {
                horizon_client::internet_music::toggle_mute();
            }
            Some(Action::MusicStepVolume(delta)) => {
                let cur = horizon_client::internet_music::volume();
                horizon_client::internet_music::set_volume(cur + delta);
            }
            Some(Action::MusicToggleDisableVanilla) => {
                horizon_client::internet_music::toggle_vanilla_music_disabled();
            }
            None => {}
        }
        Ok(())
    }

    fn toggle_bundled_mod(&mut self, id: &'static str) -> Result<()> {
        let game = self.catalog.config.game_home(&self.home);
        let mut list = horizon_client::mod_list::ModList::load(&game);
        let enabled = list.enabled.contains(id) && !list.disabled.contains(id);
        if enabled {
            list.enabled.remove(id);
            list.disabled.insert(id.into());
        } else {
            list.disabled.remove(id);
            list.enabled.insert(id.into());
        }
        std::fs::create_dir_all(&game)?;
        std::fs::write(game.join(horizon_client::mod_list::FILE), list.serialize())?;
        self.catalog.refresh(&self.home);
        self.catalog.status = format!(
            "Bundled {id} {}. Applies on Play; settings are in Options > Mods.",
            if enabled {
                "disabled"
            } else {
                "installed & enabled"
            }
        );
        Ok(())
    }

    fn modify_game_settings(&mut self, f: impl FnOnce(&mut horizon_client::settings::Settings)) {
        let game_home = self.catalog.config.game_home(&self.home);
        let _ = std::fs::create_dir_all(&game_home);
        let path = game_home.join("options.txt");
        let mut s = horizon_client::settings::Settings::load(&path);
        f(&mut s);
        s.save(&path);
    }

    fn modify_touch_settings(&mut self, f: impl FnOnce(&mut horizon_client::touch::Settings)) {
        let game_home = self.catalog.config.game_home(&self.home);
        let _ = std::fs::create_dir_all(&game_home);
        let mut t =
            horizon_client::touch::Settings::load_from(&game_home.join("horizon-touch.json"));
        f(&mut t);
        if !t.is_default_preset() {
            let _ = t.save_replace_preset(&game_home);
        } else {
            let _ = t.save_to(&game_home.join("horizon-touch.json"));
        }
        self.mouse_touch = t;
    }

    fn apply_slider_fraction(&mut self, action: Action, frac: f32) {
        let t = frac.clamp(0.0, 1.0);
        match action {
            Action::RenderScaleDelta(_) => {
                let next_pct = (25.0 + t * 75.0).round() as u32;
                let scale = crate::display::RenderScale::new(next_pct);
                self.catalog.config.render_scale = scale;
                let _ = self.catalog.config.save(&self.home);
                self.modify_game_settings(|s| {
                    s.horizon.resolution_scale = next_pct;
                });
                self.modify_touch_settings(|ts| {
                    ts.resolution_scale_percent = next_pct;
                });
            }
            Action::RenderDistDelta(_) => {
                let rd = (2.0 + t * 30.0).round() as i32;
                self.modify_game_settings(|s| s.render_distance = rd.clamp(2, 32));
            }
            Action::SimDistDelta(_) => {
                let sd = (5.0 + t * 27.0).round() as i32;
                self.modify_game_settings(|s| s.simulation_distance = sd.clamp(5, 32));
            }
            Action::FovDelta(_) => {
                let fov = (30.0 + t * 80.0).round();
                self.modify_game_settings(|s| s.fov = fov.clamp(30.0, 110.0));
            }
            Action::TouchSensDelta(_) => {
                let v = ((0.10 + t * 2.90) * 100.0).round() / 100.0;
                self.modify_touch_settings(|ts| ts.sensitivity = v.clamp(0.10, 3.0));
            }
            Action::MouseSensDelta(_) => {
                let v = ((0.05 + t * 0.95) * 100.0).round() / 100.0;
                self.modify_game_settings(|s| s.sensitivity = v.clamp(0.05, 1.0));
            }
            Action::ButtonScaleDelta(_) => {
                let v = ((0.60 + t * 1.00) * 100.0).round() / 100.0;
                self.modify_touch_settings(|ts| ts.button_scale = v.clamp(0.60, 1.60));
            }
            Action::PointerSizeDelta(_) => {
                let v = ((0.50 + t * 1.50) * 100.0).round() / 100.0;
                self.modify_touch_settings(|ts| {
                    ts.virtual_mouse_pointer_scale = v.clamp(0.50, 2.0)
                });
            }
            Action::PointerOpacityDelta(_) => {
                let v = ((0.15 + t * 0.85) * 100.0).round() / 100.0;
                self.modify_touch_settings(|ts| {
                    ts.virtual_mouse_opacity = v.clamp(0.15, 1.0)
                });
            }
            Action::GyroSensDelta(_) => {
                let v = ((0.10 + t * 2.90) * 100.0).round() / 100.0;
                self.modify_touch_settings(|ts| ts.gyro_sensitivity = v.clamp(0.10, 3.0));
            }
            Action::UiOpacityDelta(_) => {
                let v = ((0.10 + t * 0.90) * 100.0).round() / 100.0;
                self.catalog.config.theme.ui_opacity = v.clamp(0.10, 1.0);
                self.catalog.config.theme.sanitize();
                horizon_client::ui_theme::set_active_theme(self.catalog.config.theme.resolve());
                let _ = self.catalog.config.save(&self.home);
            }
            _ => {}
        }
    }

    pub(super) fn ui_density(&self) -> f32 {
        let (Some(r), Some(gui)) = (&self.renderer, &self.gui) else {
            return 1.0;
        };
        let mut batch = HudBatch::new([r.config.width, r.config.height]);
        Ui::mobile(&gui.atlas, &mut batch, (0.0, 0.0)).density
    }

    pub(super) fn touch_event(&mut self, events: &ActiveEventLoop, t: Touch) -> Result<()> {
        let point = [t.location.x as f32 / self.ui_density(), t.location.y as f32 / self.ui_density()];
        if self.tutorial.active && !self.tutorial.closing {
            self.focus = None;
            self.pending_focus = None;
            self.cursor = (t.location.x, t.location.y);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            match t.phase {
                TouchPhase::Started => {
                    self.drag = Some(super::gesture::Drag::new(t.location.x, t.location.y));
                }
                TouchPhase::Ended => {
                    let _ = self.drag.take();
                    self.handle_tutorial_tap(point[0], point[1]);
                }
                TouchPhase::Cancelled => {
                    self.drag = None;
                }
                TouchPhase::Moved => {}
            }
            return Ok(());
        }
        let on_control = t.phase == winit::event::TouchPhase::Started && self.targets.iter().any(|(r, _)|
            point[0] >= r[0] && point[0] < r[0] + r[2] && point[1] >= r[1] && point[1] < r[1] + r[3]);
        if !on_control && !self.login.visible
            && self.control_editor.is_none()
            && self.chooser.is_none()
            && self.catalog.project_modal.is_none()
            && self.catalog.fm_modal.is_none()
            && !self.exit_confirmation
            && self.wardrobe.gesture(&t, self.ui_density())
        {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return Ok(());
        }
        self.focus = None;
        self.pending_focus = None;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.cursor = (t.location.x, t.location.y);

        // Route all touches in the MCPE + GoyControl Editor through touch_hud_editor
        // and persist changes directly to the active profile's game_home.
        if self.control_editor.is_some() {
            let (rw, rh) = self.renderer.as_ref().map_or((1920, 1080), |r| {
                (r.config.width.max(1), r.config.height.max(1))
            });
            let game_home = self.catalog.config.game_home(&self.home);
            let prev_cwd = std::env::current_dir().ok();
            let _ = std::env::set_current_dir(&game_home);
            if let (Some(tc), Some(gui)) = (&mut self.control_editor, &self.gui) {
                tc.viewport(rw, rh);
                let mut b = HudBatch::new([rw, rh]);
                let norm = [
                    (t.location.x as f32 / rw as f32).clamp(0.0, 1.0),
                    (t.location.y as f32 / rh as f32).clamp(0.0, 1.0),
                ];
                let ended = matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
                let closed = horizon_client::touch_ui::touch_hud_editor(
                    tc,
                    &mut b,
                    &gui.atlas,
                    t.id,
                    norm,
                    t.phase == TouchPhase::Started,
                    ended,
                );
                if let Some(ed) = &mut tc.editor
                    && std::mem::take(&mut ed.keyboard_request)
                {
                    let text = if ed.picker_step != horizon_client::touch::PickerStep::None
                        || ed.key_picker
                    {
                        String::new()
                    } else if ed.editing_button_label {
                        ed.button_label_input.clone()
                    } else if ed.editing_save_desc {
                        ed.save_desc_input.clone()
                    } else {
                        ed.save_name_input.clone()
                    };
                    crate::platform::show(&text);
                }
                if ended || closed {
                    let _ = tc.settings.save_to(&game_home.join("horizon-touch.json"));
                }
                if closed || tc.editor.is_none() {
                    self.catalog.status = format!(
                        "Saved {} preset '{}' (controls/{}/{}.json).",
                        tc.settings.control_style.label(),
                        tc.settings.preset_name,
                        tc.settings.control_style.storage_subdir(),
                        tc.settings.preset_id
                    );
                    self.control_editor = None;
                }
            }
            if let Some(prev) = prev_cwd {
                let _ = std::env::set_current_dir(prev);
            }
            return Ok(());
        }

        let density = self.ui_density();
        let px = t.location.x as f32 / density;
        let py = t.location.y as f32 / density;
        match t.phase {
            TouchPhase::Started => {
                let hit_slider = SLIDER_TRACK_TARGETS.with(|list| {
                    list.borrow()
                        .iter()
                        .rev()
                        .find(|([x, y, w, h], _, _, _)| {
                            (*x..*x + *w).contains(&px) && (*y..*y + *h).contains(&py)
                        })
                        .map(|(_, action, bar_x, bar_w)| (*action, *bar_x, *bar_w))
                });
                if let Some((action, bar_x, bar_w)) = hit_slider {
                    ACTIVE_SLIDER_DRAG.with(|c| c.set(Some((action, bar_x, bar_w))));
                    self.drag = None;
                    let frac = ((px - bar_x) / bar_w.max(1.0)).clamp(0.0, 1.0);
                    self.apply_slider_fraction(action, frac);
                } else {
                    ACTIVE_SLIDER_DRAG.with(|c| c.set(None));
                    self.drag = Some(super::gesture::Drag::new(t.location.x, t.location.y));
                }
            }
            TouchPhase::Moved => {
                if let Some((action, bar_x, bar_w)) = ACTIVE_SLIDER_DRAG.with(|c| c.get()) {
                    let frac = ((px - bar_x) / bar_w.max(1.0)).clamp(0.0, 1.0);
                    self.apply_slider_fraction(action, frac);
                } else {
                    let density_f64 = density as f64;
                    if let Some(drag) = &mut self.drag {
                        let delta = drag.update(t.location.x, t.location.y, density_f64);
                        if drag.moved {
                            if let Some(chooser) = &mut self.chooser {
                                chooser.scroll = (chooser.scroll + delta).max(0.0);
                            } else if self.wallpapers_open {
                                if self.list_view.is_some_and(|view| {
                                    let oy = drag.origin_y() / density_f64;
                                    oy >= view[0] as f64 && oy <= view[1] as f64
                                }) {
                                    self.wallpaper_scroll = (self.wallpaper_scroll + delta).max(0.0);
                                }
                            } else if let Some(modal) = &mut self.catalog.project_modal {
                                modal.scroll = (modal.scroll + delta).max(0.0);
                            } else if self.catalog.page == Page::SkinOnline {
                                self.skin_online.scroll = (self.skin_online.scroll + delta).max(0.0);
                            } else if self
                                .list_view
                                .is_some_and(|view| drag.origin_y() / density_f64 >= view[0] as f64)
                            {
                                self.catalog.scroll = (self.catalog.scroll + delta).max(0.0);
                            }
                        }
                    }
                }
            }
            TouchPhase::Ended => {
                if ACTIVE_SLIDER_DRAG.with(|c| c.take()).is_some() {
                    self.drag = None;
                } else if self.drag.take().is_some_and(|drag| !drag.moved) {
                    self.activate(events)?;
                }
            }
            TouchPhase::Cancelled => {
                ACTIVE_SLIDER_DRAG.with(|c| c.set(None));
                self.drag = None;
            }
        }
        Ok(())
    }
    pub(super) fn type_key(&mut self, key: KeyEvent) {
        if key.physical_key == PhysicalKey::Code(KeyCode::Escape)
            || key.logical_key
                == winit::keyboard::Key::Named(winit::keyboard::NamedKey::BrowserBack)
        {
            if !key.repeat {
                self.request_back();
            }
            return;
        }
        let editor_mode = self
            .control_editor
            .as_ref()
            .and_then(|tc| tc.editor.as_ref())
            .map(|ed| {
                (
                    ed.save_dialog_open || ed.editing_button_label,
                    ed.picker_step != horizon_client::touch::PickerStep::None || ed.key_picker,
                )
            });
        let (editor_typing, editor_picking) = editor_mode.unwrap_or((false, false));
        if self.catalog.editing.is_none() && !editor_typing && !editor_picking {
            return;
        }
        let out = if editor_picking {
            match key.physical_key {
                PhysicalKey::Code(code) => Some(Output::Key(code)),
                _ => key.text.map(|s| Output::Text(s.into())),
            }
        } else {
            match key.physical_key {
                PhysicalKey::Code(KeyCode::Backspace) => Some(Output::Backspace),
                PhysicalKey::Code(KeyCode::Enter) => {
                    crate::platform::hide();
                    Some(Output::Enter)
                }
                _ => key.text.map(|s| Output::Text(s.into())),
            }
        };
        if let Some(out) = out {
            self.edit_text(out);
        }
    }

    pub(super) fn dismiss_dashboard_tutorial(&mut self) {
        if !self.tutorial.active {
            return;
        }
        self.tutorial.closing = true;
        self.tutorial.last_tick = std::time::Instant::now();
        crate::catalog::mark_dashboard_tutorial_done(&self.home, &mut self.catalog.config);
        if self.catalog.page != Page::Play {
            self.catalog.page = Page::Play;
            self.catalog.scroll = 0.0;
            self.catalog.editing = None;
            self.anim.trigger_page_transition();
        }
    }

    fn tutorial_apply_step_page(&mut self, step: usize) {
        let target_page = match step {
            4 => Page::Worlds,
            5 => Page::Mods,
            6 => Page::Packs,
            7 => Page::ControlsStudio,
            8 => Page::Shaders,
            9 => Page::Installed,
            10 => Page::Replays,
            11 => Page::Skins,
            12 => Page::Settings,
            _ => Page::Play,
        };
        if target_page == Page::Settings {
            self.catalog.settings_category = SettingsCategory::Overview;
        }
        if self.catalog.page != target_page {
            self.catalog.page = target_page;
            self.catalog.scroll = 0.0;
            self.catalog.editing = None;
            self.anim.trigger_page_transition();
            self.sync_discord_presence();
        }
    }

    pub(super) fn handle_tutorial_tap(&mut self, px: f32, py: f32) {
        if !self.tutorial.active || self.tutorial.closing {
            return;
        }
        let [sx, sy, sw, sh] = self.tutorial.skip_rect;
        let pad = 10.0_f32;
        if px >= sx - pad && px <= sx + sw + pad && py >= sy - pad && py <= sy + sh + pad {
            self.dismiss_dashboard_tutorial();
            return;
        }
        if self.tutorial.step + 1 >= DashboardTutorial::STEP_COUNT {
            self.dismiss_dashboard_tutorial();
            return;
        }
        self.tutorial.step += 1;
        self.tutorial.card_t = 0.0;
        self.tutorial.card_vel = 0.0;
        self.tutorial.last_tick = std::time::Instant::now();
        self.tutorial_apply_step_page(self.tutorial.step);
    }

    fn tutorial_step_spec(
        step: usize,
        w: f32,
        full_h: f32,
        h: f32,
    ) -> (Icon, &'static str, &'static str, [f32; 4]) {
        let nav_w = (w - 24.0).min(760.0);
        let nav_x = (w - nav_w) * 0.5;
        let nav_y = full_h - 49.0;
        let nav_h = 42.0;
        let nav_step = nav_w / 10.0;
        let content_x = (w * 0.25).max(176.0);
        let content_w = (w - content_x - 22.0).max(160.0);
        let content_rect = [
            content_x - 12.0,
            54.0,
            content_w + 22.0,
            (h - 58.0).max(100.0),
        ];
        let play_w = (w * 0.34).clamp(210.0, 310.0);
        let play_rect = [(w - play_w) * 0.5, h - 46.0, play_w, 40.0];
        let top_compact = w < 596.0;
        let utility_rect = if top_compact {
            let rx2 = (w - 12.0 - 72.0 - 6.0 - 72.0 - 6.0 - 62.0).max(74.0);
            [rx2 - 4.0, 8.0, (w - 8.0 - rx2 + 4.0).max(120.0), 42.0]
        } else {
            [w - 384.0, 8.0, 376.0, 42.0]
        };

        match step {
            0 => (
                Icon::HorizonLogo,
                "Welcome to Volt Launcher Plus",
                "Here is a quick interactive tour of every tab and option on your dashboard. Tap anywhere to continue, or tap Skip anytime.",
                [8.0, 4.0, 206.0, 48.0],
            ),
            1 => (
                Icon::Skins,
                "Account & Skin Online",
                "Tap your avatar badge to sign in with Microsoft or Offline mode, and tap Skin Online to browse community skins and capes in 3D.",
                [8.0, 50.0, 224.0, 44.0],
            ),
            2 => (
                Icon::Music,
                "Musics, Inventory & Wallpapers",
                "Stream built-in music playlists, open your Worlds/Servers/Screenshots Inventory, or equip 4K video and custom photo backgrounds.",
                utility_rect,
            ),
            3 => (
                Icon::Play,
                "Play & Quick Controls",
                "Spin your 3D character on the left, adjust Touch layout, Profile, Renderer, Gyroscope, or Keep screen on, then tap Play to launch.",
                [8.0, h - 86.0, w - 16.0, 82.0],
            ),
            4 => (
                Icon::Folder,
                "Worlds & Multiplayer Servers",
                "Jump straight into singleplayer worlds, join featured servers, or save your favorite multiplayer server addresses.",
                content_rect,
            ),
            5 => (
                Icon::Mods,
                "Mods Browser",
                "Search, preview, and install Horizon & Modrinth mods directly in the launcher with automatic version compatibility.",
                content_rect,
            ),
            6 => (
                Icon::Packs,
                "Resource Packs",
                "Discover texture packs, UI overhauls, and audio packs — inspect galleries and install in one tap.",
                content_rect,
            ),
            7 => (
                Icon::Gamepad,
                "Controls Studio",
                "Switch between MCPE and GoyControl layouts, customize touch HUD buttons, and tune virtual mouse or gyro aiming.",
                content_rect,
            ),
            8 => (
                Icon::Shaders,
                "Shaders",
                "Browse and install shader packs tailored for Android Vulkan and OpenGL ES rendering pipelines.",
                content_rect,
            ),
            9 => (
                Icon::Installed,
                "Installed & File Manager",
                "Toggle installed mods, packs, shaders, and bundled Volt+ extras, or manage files directly in the built-in File Manager.",
                content_rect,
            ),
            10 => (
                Icon::Play,
                "Replays & Video Export",
                "Rewatch recorded gameplay in free-cam and render smooth MP4 clips directly on your device.",
                content_rect,
            ),
            11 => (
                Icon::Skins,
                "Skins & Capes Wardrobe",
                "Import local skin files, equip capes, toggle slim or classic arms, and inspect your 3D character from every angle.",
                content_rect,
            ),
            12 => (
                Icon::Settings,
                "Settings & Optimization",
                "Fine-tune Video resolution & Max FPS, Theme presets, Launcher animations, Diagnostics logs, and Experimental performance boosts.",
                content_rect,
            ),
            13 => (
                Icon::HorizonLogo,
                "Floating Navigation Dock",
                "All 10 dashboard tabs live in this responsive bottom dock — switch between Play, Mods, Controls, Replays, and Settings in one tap.",
                [nav_x - 4.0, nav_y - 3.0, nav_w + 8.0, nav_h + 6.0],
            ),
            _ => {
                let _ = nav_step;
                (
                    Icon::Check,
                    "You Are Ready to Play!",
                    "That covers every tab and option on your Volt+ dashboard. Tap anywhere to close this guide and jump into Minecraft 26.3.",
                    [
                        play_rect[0] - 6.0,
                        play_rect[1] - 6.0,
                        play_rect[2] + 12.0,
                        play_rect[3] + 12.0,
                    ],
                )
            }
        }
    }

    fn draw_dashboard_tutorial(
        &mut self,
        ui: &mut Ui,
        w: f32,
        full_h: f32,
        h: f32,
        reveal: f32,
    ) {
        if self.catalog.page == Page::Music || self.catalog.page == Page::SkinOnline {
            return;
        }
        if !self.tutorial.started
            && !crate::catalog::is_dashboard_tutorial_done(&self.home, &self.catalog.config)
            && self.anim.intro_done
            && reveal >= 0.98
            && !self.login.visible
            && self.chooser.is_none()
            && !self.exit_confirmation
            && !self.wallpapers_open
            && !self.diagnostics_modal_open
            && self.worker.is_none()
            && self.road_game.is_none()
            && self.catalog.project_modal.is_none()
            && self.catalog.fm_modal.is_none()
            && self.control_editor.is_none()
        {
            let (_, _, _, initial_spot) = Self::tutorial_step_spec(0, w, full_h, h);
            self.tutorial.start(initial_spot);
        }
        if !self.tutorial.active {
            return;
        }
        let (icon, title, body, target_spot) =
            Self::tutorial_step_spec(self.tutorial.step, w, full_h, h);
        self.tutorial.step_motion(target_spot);
        if !self.tutorial.active {
            return;
        }
        if !self.tutorial.closing {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
        }

        let alpha = self.tutorial.alpha.clamp(0.0, 1.0);
        let sx = self.tutorial.spot_rect[0].clamp(4.0, (w - 24.0).max(4.0));
        let sy = self.tutorial.spot_rect[1].clamp(4.0, (full_h - 24.0).max(4.0));
        let sw = self.tutorial.spot_rect[2].clamp(20.0, (w - sx - 4.0).max(20.0));
        let sh = self.tutorial.spot_rect[3].clamp(20.0, (full_h - sy - 4.0).max(20.0));
        let right = sx + sw;
        let bottom = sy + sh;

        // 4-quad scrim framing the spotlighted dashboard element cleanly.
        let scrim = horizon_client::hud::rgba(6, 8, 16, (176.0 * alpha).round() as u8);
        if sy > 0.0 {
            ui.fill(0.0, 0.0, w, sy, scrim);
        }
        if bottom < full_h {
            ui.fill(0.0, bottom, w, (full_h - bottom).max(0.0), scrim);
        }
        if sx > 0.0 {
            ui.fill(0.0, sy, sx, sh, scrim);
        }
        if right < w {
            ui.fill(right, sy, (w - right).max(0.0), sh, scrim);
        }

        // Animated pulsing spotlight halo around the active feature/tab.
        let pulse = 0.5 + 0.5 * (self.tutorial.pulse_t * 3.4).sin();
        let outer_pad = 3.0 + pulse * 4.5;
        let accent = my3_accent();
        ui.stroke_rounded(
            (sx - outer_pad).max(1.0),
            (sy - outer_pad).max(1.0),
            (sw + outer_pad * 2.0).min(w - 2.0),
            (sh + outer_pad * 2.0).min(full_h - 2.0),
            18.0,
            1.6,
            color::with_alpha(accent, (0.28 + 0.34 * (1.0 - pulse)) * alpha),
        );
        ui.stroke_rounded(
            sx,
            sy,
            sw,
            sh,
            14.0,
            2.2,
            color::with_alpha(accent, 0.92 * alpha),
        );

        // Smooth spring-animated floating tooltip card.
        let card_ease = horizon_client::ui_theme::motion::spring_expressive_spatial(self.tutorial.card_t);
        let card_w = (w - 28.0).clamp(250.0, 416.0);
        let body_lines = wrap_text_lines(ui, Face::Regular, 11.5, body, card_w - 36.0);
        let body_h = (body_lines.len().max(1) as f32) * 16.0;
        let card_h = (118.0 + body_h).min(full_h - 24.0);

        let room_above = sy - 16.0;
        let room_below = full_h - bottom - 16.0;
        let base_y = if sy > full_h * 0.48 && room_above >= card_h {
            (sy - card_h - 14.0).max(12.0)
        } else if room_below >= card_h {
            (bottom + 14.0).min(full_h - card_h - 12.0)
        } else if room_above >= card_h {
            (sy - card_h - 14.0).max(12.0)
        } else {
            (full_h - card_h - 60.0).max(12.0)
        };
        let cx_ideal = sx + sw * 0.5 - card_w * 0.5;
        let card_x = cx_ideal.clamp(14.0, (w - card_w - 14.0).max(14.0));
        let lift_y = (1.0 - card_ease) * 16.0 + (1.0 - alpha) * 10.0;
        let card_y = (base_y + lift_y).clamp(10.0, (full_h - card_h - 10.0).max(10.0));

        let card_bg = color::with_alpha(my3_surface(), 0.97 * alpha);
        let card_border = color::with_alpha(accent, 0.55 * alpha);
        let text_col = color::with_alpha(my3_text(), alpha);
        let muted_col = color::with_alpha(my3_muted(), alpha);

        // Soft shadow + card body + top accent bar.
        ui.rounded(
            card_x + 2.0,
            card_y + 6.0,
            card_w,
            card_h,
            20.0,
            horizon_client::hud::rgba(0, 0, 0, (110.0 * alpha).round() as u8),
        );
        ui.rounded(card_x, card_y, card_w, card_h, 20.0, card_bg);
        ui.stroke_rounded(card_x, card_y, card_w, card_h, 20.0, 1.4, card_border);

        // Header row: icon badge + step pill + Skip button.
        let badge_r = [card_x + 16.0, card_y + 14.0, 32.0, 32.0];
        ui.rounded(
            badge_r[0],
            badge_r[1],
            badge_r[2],
            badge_r[3],
            11.0,
            color::with_alpha(accent, 0.18 * alpha),
        );
        ui.stroke_rounded(
            badge_r[0],
            badge_r[1],
            badge_r[2],
            badge_r[3],
            11.0,
            1.1,
            color::with_alpha(accent, 0.65 * alpha),
        );
        ui.icon(icon, badge_r[0] + 7.5, badge_r[1] + 7.5, 17.0, color::with_alpha(accent, alpha));

        let step_lbl = format!("TOUR  •  {} / {}", self.tutorial.step + 1, DashboardTutorial::STEP_COUNT);
        ui.text(Face::SemiBold, 10.5, card_x + 58.0, card_y + 22.0, color::with_alpha(accent, alpha), &step_lbl);

        let skip_w = 72.0_f32;
        let skip_h = 28.0_f32;
        let skip_rect = [card_x + card_w - skip_w - 14.0, card_y + 16.0, skip_w, skip_h];
        self.tutorial.skip_rect = skip_rect;
        ui.rounded(
            skip_rect[0],
            skip_rect[1],
            skip_rect[2],
            skip_rect[3],
            14.0,
            color::with_alpha(my3_surface_high(), 0.92 * alpha),
        );
        ui.stroke_rounded(
            skip_rect[0],
            skip_rect[1],
            skip_rect[2],
            skip_rect[3],
            14.0,
            1.0,
            color::with_alpha(my3_border(), 0.85 * alpha),
        );
        ui.centered_fit(Face::SemiBold, 11.0, skip_rect, text_col, "Skip");

        // Title + wrapped description.
        let fitted_title = ui.fit_text(Face::SemiBold, 15.5, title, card_w - 36.0);
        ui.text(Face::SemiBold, 15.5, card_x + 18.0, card_y + 54.0, text_col, &fitted_title);
        for (idx, line) in body_lines.iter().enumerate() {
            ui.text(
                Face::Regular,
                11.5,
                card_x + 18.0,
                card_y + 78.0 + idx as f32 * 16.0,
                muted_col,
                line,
            );
        }

        // Animated progress dots + pulsing tap-anywhere footer.
        let footer_y = card_y + card_h - 32.0;
        let total = DashboardTutorial::STEP_COUNT;
        let mut dot_x = card_x + 18.0;
        for i in 0..total {
            let active = i == self.tutorial.step;
            let passed = i < self.tutorial.step;
            let dw = if active { 14.0 + 4.0 * card_ease } else { 5.0 };
            let col = if active {
                color::with_alpha(accent, alpha)
            } else if passed {
                color::with_alpha(accent, 0.45 * alpha)
            } else {
                color::with_alpha(my3_border(), 0.65 * alpha)
            };
            ui.rounded(dot_x, footer_y + 8.0, dw, 5.0, 2.5, col);
            dot_x += dw + 4.0;
        }

        let is_last = self.tutorial.step + 1 >= total;
        let hint = if is_last {
            "Tap anywhere to finish"
        } else {
            "Tap anywhere for next"
        };
        let hint_alpha = (0.68 + 0.32 * pulse) * alpha;
        let hint_col = color::with_alpha(accent, hint_alpha);
        let hint_w = ui.measure(Face::SemiBold, 11.0, hint);
        let hint_x = (card_x + card_w - 34.0 - hint_w).max(dot_x + 6.0);
        ui.text(Face::SemiBold, 11.0, hint_x, footer_y + 3.0, hint_col, hint);
        ui.icon(
            if is_last { Icon::Check } else { Icon::ArrowRight },
            card_x + card_w - 28.0,
            footer_y + 4.0,
            13.0,
            hint_col,
        );
    }
}

fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn wrap_text_lines(ui: &Ui, face: Face, size: f32, text: &str, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            if lines.last().is_some_and(|l: &String| !l.is_empty()) {
                lines.push(String::new());
            }
            continue;
        }
        let mut current = String::new();
        for word in trimmed.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if ui.measure(face, size, &candidate) <= max_w || current.is_empty() {
                current = candidate;
            } else {
                lines.push(current);
                current = word.to_owned();
            }
            if lines.len() >= 16 {
                break;
            }
        }
        if !current.is_empty() && lines.len() < 16 {
            lines.push(current);
        }
        if lines.len() >= 16 {
            break;
        }
    }
    if lines.is_empty() {
        lines.push("No additional description available.".into());
    }
    lines
}

/// Whether any `.gif` has already been imported for the Custom background.
fn custom_background_file_exists(home: &std::path::Path) -> bool {
    std::fs::read_dir(home.join("backgrounds"))
        .map(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("gif"))
            })
        })
        .unwrap_or(false)
}

/// While the Custom background is selected, watches `<home>/backgrounds` (every ~1.5 s) for
/// the newest `.gif` and decodes it on a worker thread. Photos and videos are ignored.
fn custom_background_poll(home: &std::path::Path) {
    use std::sync::Mutex;
    use std::time::{Duration, Instant, SystemTime};
    static LAST_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
    static LOADED: Mutex<Option<(std::path::PathBuf, SystemTime)>> = Mutex::new(None);
    static LOADING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !horizon_client::ui_theme::active_theme().is_custom || LOADING.load(Ordering::Relaxed) {
        return;
    }
    {
        let mut last = LAST_CHECK.lock().unwrap_or_else(|e| e.into_inner());
        if last.is_some_and(|t| t.elapsed() < Duration::from_millis(1500)) {
            return;
        }
        *last = Some(Instant::now());
    }
    let mut newest: Option<(std::path::PathBuf, SystemTime)> = None;
    if let Ok(entries) = std::fs::read_dir(home.join("backgrounds")) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_gif = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("gif"));
            if !is_gif {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            if newest.as_ref().is_none_or(|(_, t)| modified > *t) {
                newest = Some((path, modified));
            }
        }
    }
    let Some(key) = newest else {
        return;
    };
    if LOADED.lock().unwrap_or_else(|e| e.into_inner()).as_ref() == Some(&key) {
        return;
    }
    LOADING.store(true, Ordering::Relaxed);
    std::thread::spawn(move || {
        let (path, _) = &key;
        match decode_custom_gif(path) {
            Ok(background) => {
                horizon_client::ui_theme::set_custom_background(Some(background));
                // Keep only the newest GIF on disk.
                if let Some(dir) = path.parent()
                    && let Ok(entries) = std::fs::read_dir(dir)
                {
                    for entry in entries.flatten() {
                        let other = entry.path();
                        let is_gif = other
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| e.eq_ignore_ascii_case("gif"));
                        if is_gif && other != *path {
                            let _ = std::fs::remove_file(other);
                        }
                    }
                }
            }
            Err(error) => log::warn!("Custom background: {error}"),
        }
        *LOADED.lock().unwrap_or_else(|e| e.into_inner()) = Some(key);
        LOADING.store(false, Ordering::Relaxed);
    });
}

/// Decodes a GIF into at most 150 frames of an 80x45 colour grid (centre-cropped to 16:9,
/// colours reduced to 12 bits so neighbouring cells merge into long runs).
fn decode_custom_gif(
    path: &std::path::Path,
) -> std::result::Result<horizon_client::ui_theme::CustomBackground, String> {
    use image::{AnimationDecoder, imageops};
    const COLS: u32 = 80;
    const ROWS: u32 = 45;
    const MAX_FRAMES: usize = 150;
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > 40 * 1024 * 1024 {
        return Err("GIF is larger than 40 MB".into());
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(file))
        .map_err(|e| e.to_string())?;
    let mut frames: Vec<Vec<u32>> = Vec::new();
    let mut delays_ms: Vec<u32> = Vec::new();
    for frame in decoder.into_frames().take(MAX_FRAMES) {
        let frame = frame.map_err(|e| e.to_string())?;
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay = (numer / denom.max(1)).clamp(20, 1000);
        let buffer = frame.into_buffer();
        let (sw, sh) = buffer.dimensions();
        if sw == 0 || sh == 0 {
            continue;
        }
        let (cw, ch) = if sw * ROWS > sh * COLS {
            ((sh * COLS / ROWS).max(1), sh)
        } else {
            (sw, (sw * ROWS / COLS).max(1))
        };
        let cropped =
            imageops::crop_imm(&buffer, (sw - cw) / 2, (sh - ch) / 2, cw, ch).to_image();
        let small = imageops::resize(&cropped, COLS, ROWS, imageops::FilterType::Triangle);
        let mut cells = Vec::with_capacity((COLS * ROWS) as usize);
        for pixel in small.pixels() {
            let [r, g, b, a] = pixel.0;
            if a < 128 {
                cells.push(0);
                continue;
            }
            let q = |v: u8| ((v >> 4) as u32) * 17;
            cells.push((q(r) << 16) | (q(g) << 8) | q(b));
        }
        frames.push(cells);
        delays_ms.push(delay);
    }
    if frames.is_empty() {
        return Err("GIF has no frames".into());
    }
    Ok(horizon_client::ui_theme::CustomBackground {
        cols: COLS,
        rows: ROWS,
        frames,
        delays_ms,
    })
}
