//! Android menu gestures and auxiliary keys, independent of gameplay fingers.
use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    osk::{Keyboard, Output},
    recorder::{self, Phase},
};
use std::{
    cell::Cell,
    collections::BTreeMap,
    time::{Duration, Instant},
};
use winit::{event::TouchPhase, keyboard::KeyCode};

#[path = "replay_hud.rs"]
mod replay_hud;
pub use replay_hud::draw_recording_badge;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetDialogState {
    pub is_copy: bool,
    pub target_id: String,
    pub name_input: String,
    pub desc_input: String,
    pub editing_desc: bool,
    pub confirm_replace_open: bool,
}

pub struct TouchUi {
    pub expanded: bool,
    pub floating_menu: bool,
    pub floating_drag: Option<(u64, [f32; 2], bool)>,
    pub hub_tab: u8,
    pub hub_scroll: f32,
    pub hub_max_scroll: Cell<f32>,
    /// The wheel's current rotation in radians: the selected segment is drawn at the top,
    /// and hit-testing subtracts this same angle, so the ring and the taps agree even
    /// mid-animation.
    pub wheel_angle: Cell<f32>,
    /// 0 shows the wheel home page, 1 shows the open category's page.
    pub hub_page: u8,
    /// Pop-in progress for the home page, eased the same way as every other transition.
    pub page_anim: Cell<f32>,
    pub hub_scroll_drag: Option<(u64, f32, f32, bool)>,
    pub presets_modal_open: bool,
    pub preset_dialog: Option<PresetDialogState>,
    pub cached_presets: Vec<crate::touch::PresetMeta>,
    pub cached_presets_style: Option<crate::touch::ControlStyle>,
    pub sensitivity_dragging: bool,
    /// The pointer-size slider of the Aiming tab is being dragged.
    pub pointer_size_dragging: bool,
    /// The virtual-mouse-speed slider of the Aiming tab is being dragged.
    pub mouse_speed_dragging: bool,
    /// The real-time 3D resolution scale slider of the Recommended tab is being dragged.
    pub resolution_scale_dragging: bool,
    /// The gyroscope sensitivity slider of the Aiming tab is being dragged.
    pub gyro_sensitivity_dragging: bool,
    /// The global button scale slider of the Buttons tab is being dragged.
    pub button_scale_dragging: bool,
    /// The overlay-button size slider of the Overlay tab is being dragged.
    pub overlay_scale_dragging: bool,
    /// The overlay-button opacity slider of the Overlay tab is being dragged.
    pub overlay_opacity_dragging: bool,
    /// The music volume slider in the Music tab is being dragged.
    pub music_volume_dragging: bool,
    /// Selected playlist index inspected in the Music tab.
    pub music_selected_playlist: usize,
    /// Whether the in-game Music tab is showing Search Online (`true`) vs Playlists (`false`).
    pub music_search_tab: bool,
    /// Whether the in-game Music search field is currently focused for Android OSK input.
    pub music_editing_search: bool,
    /// Built-in Volt+ Extras mods state (all 84 built-in client mods).
    pub volt_extras: crate::extras::VoltExtras,
    /// Which Volt+ Extras mod (`0..84`) is currently opened for per-mod configuration (`None` = list).
    pub volt_extras_config_mod: Option<u8>,
    /// Active Volt+ Extras category filter tab (`0=All, 1=PvP, 2=HUD, 3=Camera, 4=World, 5=Utility`).
    pub volt_extras_cat: u8,
    /// Search query inside Volt+ Extras Control Hub tab.
    pub volt_extras_search: String,
    /// Whether the Volt+ Extras search bar is focused for typing / Android OSK.
    pub volt_extras_search_focused: bool,
    /// Volt+ Extras enabled filter (`0=All, 1=Enabled, 2=Disabled`).
    pub volt_extras_filter: u8,
    /// When true, sort enabled mods to the top of the list.
    pub volt_extras_sort_enabled: bool,
    /// Live status/countdown string for Cinematic Time Lapse Configure screen (`None` when idle).
    pub volt_timelapse_status: Option<String>,
    /// Step state for per-mod Toggle/Use Keybind picker `(mod_id, is_use, step)`.
    pub volt_key_picker: Option<(u8, bool, crate::touch::PickerStep)>,
    /// Whether the Waypoint Manager modal is open inside Control Hub.
    pub volt_waypoint_manager: bool,
    /// Page index for Waypoint Manager list.
    pub volt_waypoints_page: usize,
    /// Live waypoint rows synced from `Game` for Waypoint Manager UI.
    pub volt_waypoints_snapshot: Vec<VoltWaypointUiRow>,
    /// Whether the Block Outline Color customization window is open.
    pub volt_block_outline_color_modal: bool,
    /// When true, the Block Outline Color window edits Fill Overlay color instead of Outline color.
    pub volt_block_outline_edit_fill: bool,
    /// Active slider drag `(touch_id, channel 0=R,1=G,2=B,3=Alpha)` inside Block Outline Color window.
    pub volt_block_outline_slider_drag: Option<(u64, u8)>,
    /// Step state for the Floating Menu "Send Custom Input" picker modal.
    pub custom_input_picker: crate::touch::PickerStep,
    /// Horizontal scroll offset for the Floating Menu custom keyboard picker.
    pub custom_kb_scroll_x: f32,
    /// Active drag state `(touch_id, last_x, moved)` for the Floating Menu custom keyboard picker.
    pub custom_kb_drag: Option<(u64, f32, bool)>,
    /// When true, tapping the floating "NX" trigger opens Nexora Booster instead of Control Hub.
    pub booster_mode: bool,
    /// When true, Nexora Booster shows the live FPS/frame-time sparkline graph instead of 2x2 gauges.
    pub booster_graph_mode: bool,
    /// Active Nexora Booster performance profile: 0=Eco, 1=Balanced, 2=Boost, 3=Demonic.
    pub booster_profile: u8,
    /// Whether Nexora Booster's 120Hz high-refresh request is latched on.
    pub booster_120hz: bool,
    /// Whether Android sustained performance / thermal pacing is active.
    pub booster_adpf: bool,
    /// Whether accidental-touch lock / edge guard is toggled on.
    pub booster_touch_lock: bool,
    /// Whether Booster Do-Not-Disturb mode is enabled.
    pub booster_dnd: bool,
    /// Short-lived status banner inside Nexora Booster `(message, timestamp)`.
    pub booster_toast: Option<(String, Instant)>,
    /// Live frame/thermal/RAM telemetry displayed by Nexora Booster.
    pub telemetry: NexoraTelemetry,
    last_hw_poll: Option<Instant>,
    last_history_push: Option<Instant>,
    pub goy_mode: bool,
    pub virtual_mouse: bool,
    /// Mirror of `!settings.virtual_mouse_enabled` for legacy callers; authoritative state lives in `touch::Settings::virtual_mouse_enabled`.
    pub pointer_off: bool,
    /// Whether the virtual pointer has been spawned at the playable viewport center at least once.
    pub cursor_initialized: bool,
    /// Last observed playable viewport `(width, height)` in GUI coordinates for proportional resize handling.
    pub last_viewport: Option<(f32, f32)>,
    /// Manual in-game virtual pointer override when toggled via GoyControl's `mouse.virtual.toggle` button.
    pub goy_in_game_mouse: bool,
    pub open_anim: Cell<f32>,
    pub tab_anim: Cell<f32>,
    pub last_anim_tick: Cell<Option<Instant>>,
    mouse_buttons: BTreeMap<u64, bool>,
    latched_modifiers: std::collections::HashSet<KeyCode>,
    relative: Option<RelativeTouch>,
    relative_click: Option<bool>,
    pub keyboard: Option<Keyboard>,
    gesture: Option<(u64, [f32; 2], [f32; 2], bool)>,
    pointer_drag: Option<u64>,
    held_keys: BTreeMap<u64, KeyCode>,
    pub f3_latched: bool,
    pub replay_timeline_drag: Option<u64>,
    pub replay_scrub_preview_ms: Option<u64>,
    pub replay_last_seek_at: Option<std::time::Instant>,
    pub replay_fly_pad: Option<(u64, [f32; 2])>,
    pub replay_fly_vert: Option<(u64, f32)>,
    pub replay_look_finger: Option<(u64, [f32; 2])>,
    pub replay_export_res_idx: usize,
    pub replay_export_fps_idx: usize,
    pub replay_export_bitrate_idx: usize,
}

impl Default for TouchUi {
    fn default() -> Self {
        Self {
            expanded: false,
            floating_menu: false,
            floating_drag: None,
            hub_tab: 0,
            hub_scroll: 0.0,
            hub_max_scroll: Cell::new(0.0),
            wheel_angle: Cell::new(0.0),
            hub_page: 0,
            page_anim: Cell::new(0.0),
            hub_scroll_drag: None,
            presets_modal_open: false,
            preset_dialog: None,
            cached_presets: Vec::new(),
            cached_presets_style: None,
            sensitivity_dragging: false,
            pointer_size_dragging: false,
            mouse_speed_dragging: false,
            resolution_scale_dragging: false,
            gyro_sensitivity_dragging: false,
            button_scale_dragging: false,
            overlay_scale_dragging: false,
            overlay_opacity_dragging: false,
            music_volume_dragging: false,
            music_selected_playlist: 0,
            music_search_tab: false,
            music_editing_search: false,
            volt_extras: crate::extras::VoltExtras::default(),
            volt_extras_config_mod: None,
            volt_extras_cat: 0,
            volt_extras_search: String::new(),
            volt_extras_search_focused: false,
            volt_extras_filter: 0,
            volt_extras_sort_enabled: false,
            volt_timelapse_status: None,
            volt_key_picker: None,
            volt_waypoint_manager: false,
            volt_waypoints_page: 0,
            volt_waypoints_snapshot: Vec::new(),
            volt_block_outline_color_modal: false,
            volt_block_outline_edit_fill: false,
            volt_block_outline_slider_drag: None,
            custom_input_picker: crate::touch::PickerStep::None,
            custom_kb_scroll_x: 0.0,
            custom_kb_drag: None,
            booster_mode: true,
            booster_graph_mode: false,
            booster_profile: 2,
            booster_120hz: true,
            booster_adpf: true,
            booster_touch_lock: false,
            booster_dnd: false,
            booster_toast: None,
            telemetry: NexoraTelemetry::default(),
            last_hw_poll: None,
            last_history_push: None,
            goy_mode: false,
            virtual_mouse: false,
            pointer_off: false,
            cursor_initialized: false,
            last_viewport: None,
            goy_in_game_mouse: false,
            open_anim: Cell::new(0.0),
            tab_anim: Cell::new(1.0),
            last_anim_tick: Cell::new(None),
            mouse_buttons: BTreeMap::new(),
            latched_modifiers: std::collections::HashSet::new(),
            relative: None,
            relative_click: None,
            keyboard: None,
            gesture: None,
            pointer_drag: None,
            held_keys: BTreeMap::new(),
            f3_latched: false,
            replay_timeline_drag: None,
            replay_scrub_preview_ms: None,
            replay_last_seek_at: None,
            replay_fly_pad: None,
            replay_fly_vert: None,
            replay_look_finger: None,
            replay_export_res_idx: 1,
            replay_export_fps_idx: 1,
            replay_export_bitrate_idx: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InGameMenuAction {
    Close,
    OpenControlEditor,
    SetControlStyle(crate::touch::ControlStyle),
    SelectPreset(String),
    CopyPreset {
        source_id: String,
        name: String,
        description: String,
    },
    UpdatePresetMeta {
        id: String,
        name: String,
        description: String,
    },
    ApplyPreset(&'static str),
    LoadSavedProfile(usize),
    PresetVisibilityAll,
    CycleMovementMode,
    CycleJoystickArea,
    ToggleSplitTouch,
    ToggleInvertY,
    ToggleLeftHanded,
    ToggleGyro,
    ToggleVirtualMouse,
    ToggleSprintMode,
    ToggleInventoryControls,
    ToggleVibrate,
    ToggleAutoJump,
    SetTouchSensitivity(f32),
    StepTouchSensitivity(i32),
    SetVirtualMouseSpeed(f32),
    StepVirtualMouseSpeed(i32),
    SetVirtualMousePointerSize(f32),
    SetGyroSensitivity(f32),
    StepGyroSensitivity(i32),
    SetButtonScale(f32),
    StepButtonScale(i32),
    SetOverlayScale(f32),
    SetOverlayOpacity(f32),
    ToggleOverlayHidden,
    ToggleOverlayShowControlsInMenus,
    TriggerKey(KeyCode),
    OpenKeyboard,
    /// Kill the game process now (no save, no shutdown sequence).
    ForceKill,
    /// Open the game's log folder in the system file manager.
    OpenLogs,
    /// The naming field of a preset dialog was tapped: show the system keyboard.
    EditPresetField(bool),
    /// Adjust real-time 3D world resolution scale percentage (25%..=100%) without recreating UI surface.
    SetResolutionScale(u32),
    /// Toggle in-game animations (animated textures, view bobbing, chunk fade, UI transitions).
    ToggleGameAnimations,
    /// Screen recorder: the resolution, frame rate, audio source and the controls toggle.
    SetRecorderResolution(u8),
    SetRecorderFps(u8),
    SetRecorderAudio(u8),
    ToggleRecorderOverlay,
    /// Ask Android for the screen-capture prompt, or end the recording that is running.
    RecorderStart,
    RecorderStop,
    /// Open a finished recording in the system's video player.
    RecorderPlay(String),
    /// Send a custom keyboard key, mouse button/scroll, or gameplay action token immediately.
    SendCustomInput(String),
    /// Replay Mod recording / settings action from Control Hub.
    Replay(crate::replay::ReplayAction),
    /// Internet Music System controls from the in-game Music tab.
    MusicTogglePause,
    MusicNext,
    MusicPrev,
    MusicSeekBy(i64),
    MusicSeekFrac(f32),
    MusicSetVolume(f32),
    MusicToggleMute,
    MusicToggleVanilla,
    MusicSelectPlaylist(usize),
    MusicPlayPlaylist(usize),
    MusicPlayPlaylistTrack(usize, usize),
    MusicSetSearchTab(bool),
    MusicOpenSearchKeyboard,
    MusicTriggerSearch,
    MusicSelectGenre(Option<&'static str>),
    MusicCycleSort,
    MusicPrevPage,
    MusicNextPage,
    MusicPlaySearchResult(usize),
    MusicDownloadSearchResult(usize),
    UpdateVoltExtras(crate::extras::VoltExtras),
    VoltQuickAction(crate::extras::VoltQuickAction),
    FocusVoltExtrasSearch,
    BoosterSetProfile(u8),
    BoosterToggle120Hz,
    BoosterRamPurge,
    BoosterToggleTouchLock,
    BoosterToggleEntityCulling,
    BoosterToggleZeroLatency,
    BoosterToggleFpsHud,
    BoosterToggleDnd,
    BoosterToggleVsync,
    BoosterIgnite,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NexoraTelemetry {
    pub fps: f32,
    pub frame_ms: f32,
    pub cpu_ms: f32,
    pub gpu_ms: f32,
    pub temp_c: f32,
    pub ram_used_mb: u32,
    pub ram_total_mb: u32,
    pub vsync: bool,
    pub fps_history: [f32; 48],
    pub history_len: usize,
}

impl Default for NexoraTelemetry {
    fn default() -> Self {
        Self {
            fps: 60.0,
            frame_ms: 16.6,
            cpu_ms: 6.2,
            gpu_ms: 5.4,
            temp_c: 36.5,
            ram_used_mb: 2400,
            ram_total_mb: 6144,
            vsync: false,
            fps_history: [60.0; 48],
            history_len: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VoltWaypointUiRow {
    pub name: String,
    pub coords: [i32; 3],
    pub dim_label: &'static str,
    pub is_death: bool,
    pub color: u32,
}

fn volt_extras_modrinth_icon_url(key: &'static str) -> Option<String> {
    crate::volt_extras_ui::modrinth_icon_url(key)
}

impl TouchUi {
    pub fn sync_volt_extras(&mut self, volt: &crate::extras::VoltExtras) {
        if self.volt_extras != *volt {
            self.volt_extras = volt.clone();
        }
    }
    pub fn set_volt_mod_key(&mut self, mod_id: u8, is_use: bool, token: String) {
        if let Some(b) = self.volt_extras.binds.0.get_mut(usize::from(mod_id)) {
            if is_use {
                b.use_key = token;
            } else {
                b.toggle_key = token;
            }
        }
    }

    pub fn set_booster_toast(&mut self, msg: impl Into<String>) {
        self.booster_toast = Some((msg.into(), Instant::now()));
    }

    pub fn push_telemetry(
        &mut self,
        fps: f32,
        frame_ms: f32,
        cpu_ms: f32,
        gpu_ms: f32,
        vsync: bool,
    ) {
        self.telemetry.fps = if fps.is_finite() && fps > 0.0 { fps } else { 60.0 };
        self.telemetry.frame_ms = if frame_ms.is_finite() && frame_ms > 0.0 {
            frame_ms
        } else {
            1000.0 / self.telemetry.fps.max(1.0)
        };
        self.telemetry.cpu_ms = cpu_ms.max(0.0);
        self.telemetry.gpu_ms = gpu_ms.max(0.0);
        self.telemetry.vsync = vsync;

        let now = Instant::now();
        let should_poll_hw = self
            .last_hw_poll
            .is_none_or(|t| now.duration_since(t) >= std::time::Duration::from_millis(1200));
        if should_poll_hw {
            self.last_hw_poll = Some(now);
            if let Some(temp) = crate::android_runtime::booster_battery_temp_c() {
                self.telemetry.temp_c = temp;
            }
            if let Some((used, total)) = crate::android_runtime::booster_memory_used_total_mb() {
                self.telemetry.ram_used_mb = used;
                self.telemetry.ram_total_mb = total.max(1);
            }
        }

        let should_push_hist = self
            .last_history_push
            .is_none_or(|t| now.duration_since(t) >= std::time::Duration::from_millis(120));
        if should_push_hist {
            self.last_history_push = Some(now);
            let n = self.telemetry.fps_history.len();
            self.telemetry.fps_history.rotate_left(1);
            self.telemetry.fps_history[n - 1] = self.telemetry.fps.clamp(0.0, 240.0);
            self.telemetry.history_len = (self.telemetry.history_len + 1).min(n);
        }
    }
}

pub fn key_code_to_char(code: KeyCode) -> Option<char> {
    match code {
        KeyCode::KeyA => Some('a'),
        KeyCode::KeyB => Some('b'),
        KeyCode::KeyC => Some('c'),
        KeyCode::KeyD => Some('d'),
        KeyCode::KeyE => Some('e'),
        KeyCode::KeyF => Some('f'),
        KeyCode::KeyG => Some('g'),
        KeyCode::KeyH => Some('h'),
        KeyCode::KeyI => Some('i'),
        KeyCode::KeyJ => Some('j'),
        KeyCode::KeyK => Some('k'),
        KeyCode::KeyL => Some('l'),
        KeyCode::KeyM => Some('m'),
        KeyCode::KeyN => Some('n'),
        KeyCode::KeyO => Some('o'),
        KeyCode::KeyP => Some('p'),
        KeyCode::KeyQ => Some('q'),
        KeyCode::KeyR => Some('r'),
        KeyCode::KeyS => Some('s'),
        KeyCode::KeyT => Some('t'),
        KeyCode::KeyU => Some('u'),
        KeyCode::KeyV => Some('v'),
        KeyCode::KeyW => Some('w'),
        KeyCode::KeyX => Some('x'),
        KeyCode::KeyY => Some('y'),
        KeyCode::KeyZ => Some('z'),
        KeyCode::Digit0 | KeyCode::Numpad0 => Some('0'),
        KeyCode::Digit1 | KeyCode::Numpad1 => Some('1'),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some('2'),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some('3'),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some('4'),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some('5'),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some('6'),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some('7'),
        KeyCode::Digit8 | KeyCode::Numpad8 => Some('8'),
        KeyCode::Digit9 | KeyCode::Numpad9 => Some('9'),
        KeyCode::Space => Some(' '),
        KeyCode::Minus | KeyCode::NumpadSubtract => Some('-'),
        KeyCode::Period | KeyCode::NumpadDecimal => Some('.'),
        KeyCode::Comma => Some(','),
        KeyCode::Slash | KeyCode::NumpadDivide => Some('/'),
        _ => None,
    }
}

/// Whether a dialog consumed an output that confirms or dismisses it. The
/// library only edits dialog state; the Android host hides its soft keyboard.
pub fn output_dismisses(out: &Output) -> bool {
    matches!(
        out,
        Output::Enter | Output::Close | Output::Key(KeyCode::Enter | KeyCode::Escape)
    )
}

/// Applies an on-screen or hardware keyboard `Output` event to an active Control Editor
/// Save Preset dialog or Floating Menu Copy/Edit Preset dialog.
pub fn apply_editor_or_preset_output(
    ed: Option<&mut crate::touch::Editor>,
    preset_dlg: Option<&mut PresetDialogState>,
    out: &Output,
) -> bool {
    if let Some(ed) = ed
        && ed.save_dialog_open
    {
        let max_len = if ed.editing_save_desc { 48 } else { 28 };
        let target = if ed.editing_save_desc {
            &mut ed.save_desc_input
        } else {
            &mut ed.save_name_input
        };
        match out {
            Output::Text(s) => {
                for ch in s.chars() {
                    if !ch.is_control() && target.chars().count() < max_len {
                        target.push(ch);
                    }
                }
            }
            Output::Backspace | Output::Key(KeyCode::Backspace) => {
                target.pop();
            }
            Output::Enter | Output::Close | Output::Key(KeyCode::Enter | KeyCode::Escape) => {}
            Output::Key(code) => {
                if let Some(ch) = key_code_to_char(*code)
                    && target.chars().count() < max_len
                {
                    target.push(ch);
                }
            }
        }
        return true;
    }
    if let Some(dlg) = preset_dlg {
        let max_len = if dlg.editing_desc { 48 } else { 28 };
        let target = if dlg.editing_desc {
            &mut dlg.desc_input
        } else {
            &mut dlg.name_input
        };
        match out {
            Output::Text(s) => {
                for ch in s.chars() {
                    if !ch.is_control() && target.chars().count() < max_len {
                        target.push(ch);
                    }
                }
            }
            Output::Backspace | Output::Key(KeyCode::Backspace) => {
                target.pop();
            }
            Output::Enter | Output::Close | Output::Key(KeyCode::Enter | KeyCode::Escape) => {}
            Output::Key(code) => {
                if let Some(ch) = key_code_to_char(*code)
                    && target.chars().count() < max_len
                {
                    target.push(ch);
                }
            }
        }
        return true;
    }
    false
}

/// Applies keyboard input to the currently selected button's custom name in the Control Editor.
pub fn apply_editor_button_label_output(
    tc: &mut crate::touch::TouchControls,
    out: &Output,
) -> bool {
    let Some(ed) = tc.editor.as_mut() else {
        return false;
    };
    if !ed.editing_button_label {
        return false;
    }
    let max_len = 18;
    match out {
        Output::Text(txt) => {
            for ch in txt.chars() {
                if !ch.is_control() && ed.button_label_input.chars().count() < max_len {
                    ed.button_label_input.push(ch);
                }
            }
        }
        Output::Backspace | Output::Key(KeyCode::Backspace) => {
            ed.button_label_input.pop();
        }
        Output::Enter | Output::Close | Output::Key(KeyCode::Enter | KeyCode::Escape) => {
            ed.editing_button_label = false;
        }
        Output::Key(code) => {
            if let Some(ch) = key_code_to_char(*code)
                && ed.button_label_input.chars().count() < max_len
            {
                ed.button_label_input.push(ch);
            }
        }
    }
    let new_label = ed.button_label_input.clone();
    let sel_custom = ed.selected_custom;
    let sel_ctrl = ed.selected;
    if let Some(idx) = sel_custom {
        if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
            btn.custom_label = new_label;
        }
    } else if let Some(c) = sel_ctrl {
        tc.settings.control_labels[c as usize] = new_label;
    }
    let _ = tc.settings.save();
    true
}
struct RelativeTouch {
    id: u64,
    origin: [f32; 2],
    last: [f32; 2],
    cursor: [f32; 2],
    since: Instant,
    dragged: bool,
    right_sent: bool,
}
pub enum Gesture {
    None,
    Tap,
    Scroll(f32),
}
pub enum Action {
    LatchF3,
    Pick,
    DebugCombo(KeyCode),
    Key(KeyCode),
    Keyboard,
    VirtualMouse,
    MouseButton(bool),
    More,
}

const KEYS: [(&str, KeyCode); 28] = [
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
    ("Tab", KeyCode::Tab),
    ("Swap", KeyCode::KeyF),
    ("Drop", KeyCode::KeyQ),
    ("Adv", KeyCode::KeyL),
    ("Chat", KeyCode::KeyT),
    ("/", KeyCode::Slash),
    ("Enter", KeyCode::Enter),
    ("Esc", KeyCode::Escape),
    ("Shift", KeyCode::ShiftLeft),
    ("Ctrl", KeyCode::ControlLeft),
    ("Inv", KeyCode::KeyE),
    ("Pick", KeyCode::ContextMenu),
    ("F3+B", KeyCode::KeyB),
    ("F3+G", KeyCode::KeyG),
    ("F3+A", KeyCode::KeyA),
    ("F3+T", KeyCode::KeyT),
];

fn mouse_button_rects(w: f32, h: f32) -> [([f32; 4], bool); 2] {
    [
        ([w - 53.0, h - 52.0, 48.0, 22.0], false),
        ([w - 53.0, h - 28.0, 48.0, 22.0], true),
    ]
}

/// Faithful Windows 11-style standard arrow pointer (`aero_arrow`) with sub-pixel
/// vector geometry, crisp dark outline, pure white body, subtle contact shadow,
/// and exact tip hotspot at `(cursor.0, cursor.1)` clamped cleanly to the screen bounds.
pub fn draw_pointer(batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32), scale: f32) {
    draw_pointer_styled(
        batch,
        atlas,
        cursor,
        scale,
        1.0,
        crate::touch::CursorIcon::Default,
    );
}

/// Renders either the default Windows 11-style vector arrow or one of the bundled
/// custom cursor icons with the configured scale and opacity, anchored at the tip `(cursor.0, cursor.1)`.
pub fn draw_pointer_styled(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    cursor: (f32, f32),
    scale: f32,
    opacity: f32,
    icon: crate::touch::CursorIcon,
) {
    let scale = if scale.is_finite() {
        scale.clamp(0.5, 2.0)
    } else {
        1.0
    };
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.15, 1.0)
    } else {
        1.0
    };
    let sw = batch.width.max(1.0);
    let sh = batch.height.max(1.0);
    let x = cursor.0.clamp(0.0, (sw - 3.0 * scale).max(0.0));
    let y = cursor.1.clamp(0.0, (sh - 3.0 * scale).max(0.0));

    if let Some(sprite_name) = icon.sprite_name()
        && let Some(rect) = atlas.sprites.get(sprite_name).copied()
    {
        let size = (sh * 0.068).clamp(16.0, 22.0) * scale;
        let alpha = (opacity * 255.0).round().clamp(0.0, 255.0) as u8;
        // Offset by 2/64 of sprite size so the visual tip aligns tightly with (x, y).
        let pad = size * (2.0 / 64.0);
        batch.rect_uv(
            (x - pad).clamp(0.0, (sw - 4.0).max(0.0)),
            (y - pad).clamp(0.0, (sh - 4.0).max(0.0)),
            size,
            size,
            rect,
            rgba(255, 255, 255, alpha),
        );
        return;
    }

    let h = (sh * 0.052).clamp(12.0, 15.5) * scale;
    let s = h / 16.0;
    let pt = |dx: f32, dy: f32| -> [f32; 2] {
        [(x + dx * s).clamp(0.0, sw), (y + dy * s).clamp(0.0, sh)]
    };

    let shadow: u32 = rgba(0, 0, 0, (64.0 * opacity).round() as u8);
    let outline: u32 = rgba(18, 18, 22, (248.0 * opacity).round() as u8);
    let fill: u32 = rgba(255, 255, 255, (255.0 * opacity).round() as u8);

    // 1. Subtle contact drop shadow (+0.65, +0.90) for contrast over bright surfaces
    let sx = 0.65;
    let sy = 0.90;
    batch.fill_quad(
        atlas,
        [
            pt(sx, sy),
            pt(4.1 + sx, 11.4 + sy),
            pt(sx, 14.8 + sy),
            pt(sx, sy),
        ],
        shadow,
    );
    batch.fill_quad(
        atlas,
        [
            pt(sx, sy),
            pt(10.6 + sx, 10.6 + sy),
            pt(6.2 + sx, 10.6 + sy),
            pt(4.1 + sx, 11.4 + sy),
        ],
        shadow,
    );
    batch.fill_quad(
        atlas,
        [
            pt(3.8 + sx, 11.0 + sy),
            pt(6.3 + sx, 10.0 + sy),
            pt(8.6 + sx, 15.3 + sy),
            pt(6.0 + sx, 16.4 + sy),
        ],
        shadow,
    );

    // 2. Crisp Windows 11 outer dark silhouette (exact tip at (0.0, 0.0))
    batch.fill_quad(
        atlas,
        [pt(0.0, 0.0), pt(4.1, 11.4), pt(0.0, 14.8), pt(0.0, 0.0)],
        outline,
    );
    batch.fill_quad(
        atlas,
        [pt(0.0, 0.0), pt(10.6, 10.6), pt(6.2, 10.6), pt(4.1, 11.4)],
        outline,
    );
    batch.fill_quad(
        atlas,
        [pt(3.8, 11.0), pt(6.3, 10.0), pt(8.6, 15.3), pt(6.0, 16.4)],
        outline,
    );

    // 3. Pure white interior fill inset cleanly inside the silhouette
    batch.fill_quad(
        atlas,
        [
            pt(0.85, 1.90),
            pt(4.05, 10.35),
            pt(0.85, 12.85),
            pt(0.85, 1.90),
        ],
        fill,
    );
    batch.fill_quad(
        atlas,
        [
            pt(0.85, 1.90),
            pt(8.65, 9.70),
            pt(5.75, 9.70),
            pt(4.05, 10.35),
        ],
        fill,
    );
    batch.fill_quad(
        atlas,
        [
            pt(4.15, 9.95),
            pt(5.55, 9.35),
            pt(7.60, 14.65),
            pt(6.20, 15.25),
        ],
        fill,
    );
}

impl TouchUi {
    /// Ensures the virtual mouse cursor spawns at the exact center `(width * 0.5, height * 0.5)`
    /// of the playable viewport and scales proportionally across viewport/orientation changes
    /// without offset drift or sudden jumps.
    pub fn sync_viewport_cursor(
        &mut self,
        cursor: &mut (f32, f32),
        [width, height]: [f32; 2],
    ) -> bool {
        if !width.is_finite() || !height.is_finite() || width <= 1.0 || height <= 1.0 {
            return false;
        }
        let max_x = (width - 1.0).max(0.0);
        let max_y = (height - 1.0).max(0.0);
        if !self.cursor_initialized || !cursor.0.is_finite() || !cursor.1.is_finite() {
            *cursor = (width * 0.5, height * 0.5);
            self.cursor_initialized = true;
            self.last_viewport = Some((width, height));
            self.cancel_relative();
            return true;
        }
        if let Some((old_w, old_h)) = self.last_viewport {
            if (old_w - width).abs() > 0.5 || (old_h - height).abs() > 0.5 {
                let nx = (cursor.0 / old_w.max(1.0) * width).clamp(0.0, max_x);
                let ny = (cursor.1 / old_h.max(1.0) * height).clamp(0.0, max_y);
                *cursor = (nx, ny);
                self.last_viewport = Some((width, height));
                self.cancel_relative();
                return true;
            }
        } else {
            self.last_viewport = Some((width, height));
        }
        cursor.0 = cursor.0.clamp(0.0, max_x);
        cursor.1 = cursor.1.clamp(0.0, max_y);
        false
    }

    /// Computes whether the virtual mouse pointer should be visible for the current state:
    /// - If `controls.editor.is_some()`, always returns false (never show cursor over Control Editor).
    /// - If `settings.virtual_mouse_enabled` is false, always returns false.
    /// - When the Floating Menu is open, returns true.
    /// - In MCPE mode: hidden when MCPE gameplay controls are visible; automatically visible
    ///   when MCPE controls are hidden (menus, inventory, HUD hidden, or all controls hidden).
    /// - In GoyControl mode: visible in menus or when toggled on in-game via `mouse.virtual.toggle`.
    pub fn compute_virtual_mouse_visibility(
        &mut self,
        controls: &crate::touch::TouchControls,
        in_menu: bool,
        hide_gui: bool,
        touch_active: bool,
        captured: bool,
    ) -> bool {
        self.pointer_off = !controls.settings.virtual_mouse_enabled;
        if controls.editor.is_some() {
            return false;
        }
        // When a physical mouse is active (!touch_active): keep Horizon's unified
        // smooth virtual pointer visible in menus so there is a single lag-free cursor,
        // while keeping it hidden during un-toggled gameplay crosshair look.
        if !touch_active {
            if !in_menu && !self.goy_in_game_mouse {
                return false;
            }
            if self.floating_menu
                || self.preset_dialog.is_some()
                || self.presets_modal_open
                || self.custom_input_picker != crate::touch::PickerStep::None
                || self.volt_key_picker.is_some()
                || self.volt_waypoint_manager
                || self.volt_block_outline_color_modal
            {
                return false;
            }
            return true;
        }
        if self.floating_menu
            || self.preset_dialog.is_some()
            || self.presets_modal_open
            || self.custom_input_picker != crate::touch::PickerStep::None
            || self.volt_key_picker.is_some()
            || self.volt_waypoint_manager
            || self.volt_block_outline_color_modal
        {
            return false;
        }
        if !controls.settings.virtual_mouse_enabled {
            self.goy_in_game_mouse = false;
            return false;
        }
        match controls.settings.control_style {
            crate::touch::ControlStyle::Mcpe => {
                if in_menu {
                    return true;
                }
                let mcpe_visible = controls.mcpe_controls_visible(false, hide_gui, touch_active);
                !mcpe_visible || !captured || self.goy_in_game_mouse
            }
            crate::touch::ControlStyle::GoyControl => in_menu || self.goy_in_game_mouse,
        }
    }

    /// Captures a physical or soft-keyboard `KeyCode` when the `Send Custom Input` picker modal is open.
    pub fn capture_custom_input_key(&mut self, code: KeyCode) -> Option<String> {
        if self.custom_input_picker == crate::touch::PickerStep::None {
            return None;
        }
        if code == KeyCode::Escape {
            self.custom_input_picker = crate::touch::PickerStep::None;
            return None;
        }
        let token = crate::keybinds::key_name(code)?.to_owned();
        self.custom_input_picker = crate::touch::PickerStep::None;
        self.floating_menu = false;
        Some(token)
    }

    /// Captures a typed character from the Android soft keyboard when `Send Custom Input` is open.
    pub fn capture_custom_input_text(&mut self, text: &str) -> Option<String> {
        if self.custom_input_picker == crate::touch::PickerStep::None {
            return None;
        }
        let ch = text.chars().find(|c| !c.is_control())?;
        let tok = crate::touch::TouchControls::char_to_key_token(ch)?;
        self.custom_input_picker = crate::touch::PickerStep::None;
        self.floating_menu = false;
        Some(tok.to_owned())
    }

    /// Captures a physical or soft-keyboard `KeyCode` when a Volt+ Extras mod keybind picker is open.
    pub fn capture_volt_key_picker_key(&mut self, code: KeyCode) -> Option<crate::extras::VoltExtras> {
        let (mod_id, is_use, _) = self.volt_key_picker?;
        if code == KeyCode::Escape {
            self.volt_key_picker = None;
            return None;
        }
        let token = crate::keybinds::key_name(code)?.to_owned();
        self.set_volt_mod_key(mod_id, is_use, token);
        self.volt_key_picker = None;
        Some(self.volt_extras.clone())
    }

    /// Captures a typed character from the Android soft keyboard when a Volt+ Extras mod keybind picker is open.
    pub fn capture_volt_key_picker_text(&mut self, text: &str) -> Option<crate::extras::VoltExtras> {
        let (mod_id, is_use, _) = self.volt_key_picker?;
        let ch = text.chars().find(|c| !c.is_control())?;
        let tok = crate::touch::TouchControls::char_to_key_token(ch)?;
        self.set_volt_mod_key(mod_id, is_use, tok.to_owned());
        self.volt_key_picker = None;
        Some(self.volt_extras.clone())
    }

    pub fn refresh_presets_cache(&mut self, style: crate::touch::ControlStyle) {
        self.cached_presets =
            crate::touch::Settings::list_presets(std::path::Path::new("."), style);
        self.cached_presets_style = Some(style);
    }
    pub fn ensure_presets_cache(&mut self, style: crate::touch::ControlStyle) {
        if self.cached_presets_style != Some(style) || self.cached_presets.is_empty() {
            self.refresh_presets_cache(style);
        }
    }
    pub fn key_down(&mut self, id: u64, key: KeyCode) -> bool {
        let first = !self.key_held(key);
        self.held_keys.insert(id, key);
        first
    }
    pub fn key_up(&mut self, id: u64) -> Option<KeyCode> {
        let key = self.held_keys.remove(&id)?;
        (!self.key_held(key)).then_some(key)
    }
    pub fn owns_key(&self, id: u64) -> bool {
        self.held_keys.contains_key(&id)
    }
    pub fn toggle_modifier(&mut self, key: KeyCode) -> bool {
        if self.latched_modifiers.remove(&key) {
            false
        } else {
            self.latched_modifiers.insert(key);
            true
        }
    }

    pub fn modifier_latched(&self, key: KeyCode) -> bool {
        self.latched_modifiers.contains(&key)
    }

    pub fn clear_modifiers(&mut self) {
        self.latched_modifiers.clear();
    }

    pub fn key_held(&self, key: KeyCode) -> bool {
        (key == KeyCode::F3 && self.f3_latched) || self.held_keys.values().any(|k| *k == key)
    }
    pub fn release_keys(&mut self) -> Vec<KeyCode> {
        let mut keys: Vec<_> = std::mem::take(&mut self.held_keys).into_values().collect();
        if self.f3_latched {
            keys.push(KeyCode::F3);
        }
        self.f3_latched = false;
        keys.sort();
        keys.dedup();
        keys
    }
    pub fn press_mouse(&mut self, id: u64, right: bool) -> bool {
        if let Some(touch) = &mut self.relative {
            touch.dragged = true;
        }
        let first = !self.mouse_buttons.values().any(|v| *v == right);
        self.mouse_buttons.insert(id, right);
        first
    }
    pub fn release_mouse(&mut self, id: u64) -> Option<(bool, bool)> {
        let right = self.mouse_buttons.remove(&id)?;
        Some((right, !self.mouse_buttons.values().any(|v| *v == right)))
    }
    pub fn mouse_held(&self) -> bool {
        !self.mouse_buttons.is_empty()
    }
    /// The pointer holds one of the auxiliary mouse buttons (a `press_mouse`
    /// without its `release_mouse`), so its releases end it even when the
    /// screen or layout changed underneath.
    pub fn owns_mouse(&self, id: u64) -> bool {
        self.mouse_buttons.contains_key(&id)
    }
    pub fn release_all_mouse(&mut self) -> Vec<bool> {
        let mut buttons: Vec<_> = std::mem::take(&mut self.mouse_buttons)
            .into_values()
            .collect();
        buttons.sort();
        buttons.dedup();
        buttons
    }
    pub fn cancel(&mut self) {
        self.gesture = None;
        self.pointer_drag = None;
        self.relative = None;
        self.relative_click = None;
        self.sensitivity_dragging = false;
        self.pointer_size_dragging = false;
        self.mouse_speed_dragging = false;
        self.resolution_scale_dragging = false;
        self.custom_kb_drag = None;
        self.hub_scroll_drag = None;
    }
    pub fn cancel_relative(&mut self) {
        self.relative = None;
        self.relative_click = None;
    }
    pub fn owns_relative(&self, id: u64) -> bool {
        self.relative.as_ref().is_some_and(|t| t.id == id)
    }
    pub fn owns_gesture(&self, id: u64) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|(owner, _, _, _)| *owner == id)
            || self.pointer_drag == Some(id)
    }
    /// Relative motion: touching down does not teleport the cursor, and movement
    /// uses linear 1:1 sensitivity scaling without non-linear acceleration drift.
    pub fn relative_touch(
        &mut self,
        id: u64,
        point: [f32; 2],
        phase: TouchPhase,
        cursor: [f32; 2],
        size: [f32; 2],
        sensitivity: f32,
        clicks: bool,
    ) -> [f32; 2] {
        if phase == TouchPhase::Cancelled {
            self.cancel();
            return cursor;
        }
        let sens = if sensitivity.is_finite() {
            sensitivity.clamp(0.1, 4.0)
        } else {
            1.0
        };
        if phase == TouchPhase::Started && self.relative.is_none() {
            self.relative = Some(RelativeTouch {
                id,
                origin: point,
                last: point,
                cursor,
                since: Instant::now(),
                dragged: !clicks,
                right_sent: false,
            });
        }
        let Some(touch) = self.relative.as_mut().filter(|t| t.id == id) else {
            return cursor;
        };
        if (touch.cursor[0] - cursor[0]).abs() > 1.0 || (touch.cursor[1] - cursor[1]).abs() > 1.0 {
            touch.cursor = cursor;
        }
        touch.dragged |= (point[0] - touch.origin[0]).hypot(point[1] - touch.origin[1]) > 3.0;
        let position = [0, 1].map(|i| {
            let max_bound = (size[i] - 1.0).max(0.0);
            (touch.cursor[i] + (point[i] - touch.last[i]) * sens).clamp(0.0, max_bound)
        });
        touch.cursor = position;
        touch.last = point;
        if phase == TouchPhase::Ended {
            if clicks && !touch.dragged && !touch.right_sent {
                self.relative_click = Some(touch.since.elapsed() >= Duration::from_millis(500));
            }
            self.relative = None;
        }
        position
    }
    /// false = LMB, true = RMB. A stationary hold clicks once, even before release.
    pub fn relative_click(&mut self, clicks: bool) -> Option<bool> {
        if !clicks {
            self.relative_click = None;
            return None;
        }
        if let Some(right) = self.relative_click.take() {
            return Some(right);
        }
        let touch = self.relative.as_mut()?;
        if !touch.dragged
            && !touch.right_sent
            && touch.since.elapsed() >= Duration::from_millis(500)
        {
            touch.right_sent = true;
            return Some(true);
        }
        None
    }
    /// Sliders retain mouse dragging; list rows use scroll-or-tap gestures.
    pub fn direct_pointer(&mut self, id: u64, p: [f32; 2], phase: TouchPhase) -> bool {
        let on_target_slider = |t: &crate::widgets::Target| {
            let [x, y, w, h] = t.rect;
            matches!(t.kind, crate::widgets::Kind::Slider { .. })
                && p[0] >= x - 4.0
                && p[0] <= x + w + 4.0
                && p[1] >= y - 4.0
                && p[1] <= y + h + 4.0
        };
        let on_widget_slider = |t: &crate::widgets::Widget| {
            matches!(t.kind, crate::widgets::Kind::Slider { .. })
                && p[0] >= t.x - 4.0
                && p[0] <= t.x + t.w + 4.0
                && p[1] >= t.y - 4.0
                && p[1] <= t.y + t.h + 4.0
        };
        if phase == TouchPhase::Started
            && (crate::widgets::last_targets().iter().any(on_target_slider)
                || crate::widgets::last_labelled().iter().any(on_widget_slider))
        {
            self.pointer_drag = Some(id);
        }
        if self.pointer_drag == Some(id) {
            if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.pointer_drag = None;
            }
            return true;
        }
        false
    }
    /// A drag scrolls; a tap clicks only on release. Never activate a row while scrolling it.
    pub fn gesture(&mut self, id: u64, p: [f32; 2], phase: TouchPhase) -> Gesture {
        if phase == TouchPhase::Cancelled {
            self.cancel();
            return Gesture::None;
        }
        if phase == TouchPhase::Started {
            if self.gesture.is_none() {
                self.gesture = Some((id, p, p, false));
            }
            return Gesture::None;
        }
        let Some((owner, start, previous, dragged)) = &mut self.gesture else {
            return Gesture::None;
        };
        if *owner != id {
            return Gesture::None;
        }
        *dragged |= (p[0] - start[0]).hypot(p[1] - start[1]) > 12.0;
        let dy = p[1] - previous[1];
        *previous = p;
        if phase == TouchPhase::Ended {
            let tap = !*dragged;
            self.cancel();
            return if tap { Gesture::Tap } else { Gesture::None };
        }
        if *dragged {
            Gesture::Scroll(dy / 30.0)
        } else {
            Gesture::None
        }
    }
    pub fn action(&self, w: f32, h: f32, p: [f32; 2], menu: bool) -> Option<Action> {
        if menu && self.virtual_mouse && !self.pointer_off {
            let [px, py, pw, ph] = pri_button_rect(w, h);
            if (px..px + pw).contains(&p[0]) && (py..py + ph).contains(&p[1]) {
                return Some(Action::MouseButton(false));
            }
        }
        if !self.goy_mode {
            if menu && self.virtual_mouse && !self.pointer_off {
                let [_, rmb] = mouse_button_rects(w, h);
                let [x, y, width, height] = rmb.0;
                if (x..x + width).contains(&p[0]) && (y..y + height).contains(&p[1]) {
                    return Some(Action::MouseButton(true));
                }
            }
            if menu && (3.0..23.0).contains(&p[1]) && (w - 53.0..w - 3.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::Escape));
            }
        }
        if self.expanded && (3.0..23.0).contains(&p[1]) {
            if (3.0..36.0).contains(&p[0]) {
                return Some(Action::More);
            }
            if (39.0..72.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F3));
            }
            if (75.0..108.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::F5));
            }
            if (111.0..144.0).contains(&p[0]) {
                return Some(Action::Key(KeyCode::KeyT));
            }
            if (147.0..186.0).contains(&p[0]) {
                return Some(Action::Keyboard);
            }
        }
        if self.expanded && (3.0..203.0).contains(&p[0]) && (27.0..219.0).contains(&p[1]) {
            let i = ((p[1] - 27.0) / 24.0) as usize * 4 + ((p[0] - 3.0) / 50.0) as usize;
            if i == 28 {
                return Some(Action::LatchF3);
            }
            return KEYS.get(i).map(|(_, k)| match i {
                23 => Action::Pick,
                24.. => Action::DebugCombo(*k),
                _ => Action::Key(*k),
            });
        }
        None
    }
    pub fn draw(&self, b: &mut HudBatch, a: &HudAtlas, menu: bool) {
        let touch_active = crate::android_runtime::active_input() == crate::android_gamepad::InputMode::Touch;
        let hud_vis = crate::touch::current_hud_opacity_scale() > 0.01;
        if menu && self.virtual_mouse && !self.pointer_off {
            draw_pri_button(b, a, pri_button_rect(b.width, b.height));
        }
        if menu && !self.goy_mode && touch_active {
            if hud_vis {
                button(b, a, "Back", b.width - 53.0, 3.0, 50.0);
            }
            if self.virtual_mouse && !self.pointer_off {
                let [_, rmb] = mouse_button_rects(b.width, b.height);
                button(b, a, "RMB", rmb.0[0], rmb.0[1], rmb.0[2]);
            }
        }
        if !self.expanded {
            return;
        }
        for (label, x, width) in [
            ("...", 3.0, 33.0),
            ("F3", 39.0, 33.0),
            ("F5", 75.0, 33.0),
            ("Chat", 111.0, 33.0),
            ("Keys", 147.0, 39.0),
        ] {
            button(
                b,
                a,
                if label == "F3" && self.key_held(KeyCode::F3) {
                    "F3*"
                } else {
                    label
                },
                x,
                3.0,
                width,
            );
        }
        button(
            b,
            a,
            if self.f3_latched { "F3 ON" } else { "HoldF3" },
            3.0,
            195.0,
            48.0,
        );
        for (i, (label, _)) in KEYS.iter().enumerate() {
            button(
                b,
                a,
                label,
                3.0 + (i % 4) as f32 * 50.0,
                27.0 + (i / 4) as f32 * 24.0,
                48.0,
            );
        }
    }
    pub fn keyboard_touch(
        &mut self,
        b: &mut HudBatch,
        a: &HudAtlas,
        p: [f32; 2],
    ) -> Option<Output> {
        let ui = crate::ui_theme::Ui::new(a, b, (0.0, 0.0));
        self.keyboard.as_mut()?.touch(
            ui.w,
            ui.h,
            false,
            [
                p[0] * ui.batch.scale / ui.density,
                p[1] * ui.batch.scale / ui.density,
            ],
        )
    }
    pub fn draw_keyboard(&self, b: &mut HudBatch, a: &HudAtlas) {
        if let Some(k) = &self.keyboard {
            let mut ui = crate::ui_theme::Ui::new(a, b, (0.0, 0.0));
            k.draw(&mut ui, false, crate::gamepad::GlyphStyle::Xbox);
            ui.finish();
        }
    }
}
pub fn pri_button_rect(w: f32, _h: f32) -> [f32; 4] {
    let bw = 48.0;
    let bh = 28.0;
    let bx = (w - 196.0).max(w * 0.64).min((w - bw - 8.0).max(4.0));
    [bx, 4.0, bw, bh]
}
fn draw_pri_button(b: &mut HudBatch, a: &HudAtlas, rect: [f32; 4]) {
    let [x, y, w, h] = rect;
    if !crate::touch::theme_button(b, a, "button", rect, false, u32::MAX, true) {
        b.fill(a, x, y, w, h, rgba(20, 22, 30, 215));
    }
    let tw = HudBatch::text_width(a, "pri");
    let tx = x + ((w - tw) * 0.5).max(4.0);
    let ty = y + ((h - 8.0) * 0.5).max(4.0);
    b.text(a, "pri", tx, ty, rgba(248, 248, 252, 255), true);
}
fn button(b: &mut HudBatch, a: &HudAtlas, label: &str, x: f32, y: f32, w: f32) {
    if !crate::touch::theme_button(b, a, "button", [x, y, w, 20.0], false, u32::MAX, true) {
        b.fill(a, x, y, w, 20.0, rgba(16, 18, 24, 180));
    }
    b.text(a, label, x + 4.0, y + 6.0, rgba(245, 245, 245, 255), true);
}

/// Whether a physical left click takes part in the pad's utility buttons
/// instead of going to the screen handler. Presses start only over a
/// touched button (anything else belongs to the world: attack, use).
/// Releases end whatever the pad already owns even when the layout no
/// longer touches, so a held press survives menu entry, exit and the IME
/// (a release the press never produced is a stray and is ignored).
pub fn aux_mouse_phase(pressed: bool, owned: bool, touched: bool) -> Option<TouchPhase> {
    (if pressed { touched } else { owned }).then_some(if pressed {
        TouchPhase::Started
    } else {
        TouchPhase::Ended
    })
}

const BLOCK_OUTLINE_SWATCHES: [(u32, &str); 12] = [
    (0x00E5FF, "Cyan"),
    (0xFFFFFF, "White"),
    (0x000000, "Black"),
    (0xFF3B30, "Red"),
    (0xFF9500, "Orange"),
    (0xFFD60A, "Gold"),
    (0x32D74B, "Lime"),
    (0x38BDF8, "Sky"),
    (0x0A84FF, "Blue"),
    (0xBF5AF2, "Purple"),
    (0xFF375F, "Pink"),
    (0xDC2626, "Crimson"),
];

const COLOR_SWATCHES: [(u32, &str); 8] = [
    (crate::ui_theme::color::hex(0x000000, 215), "Black"),
    (crate::ui_theme::color::hex(0x9A7BFF, 235), "Violet"),
    (crate::ui_theme::color::hex(0xD0BCFF, 235), "Lavender"),
    (crate::ui_theme::color::hex(0x38BDF8, 235), "Cyan"),
    (crate::ui_theme::color::hex(0x34D399, 235), "Emerald"),
    (crate::ui_theme::color::hex(0xFBBF24, 235), "Amber"),
    (crate::ui_theme::color::hex(0xF87171, 235), "Crimson"),
    (crate::ui_theme::color::hex(0xF4EFFA, 235), "Snow"),
];

const KB_ROWS: &[&[(&str, &str, f32)]] = &[
    &[
        ("ESC", "key.keyboard.escape", 1.1),
        ("F1", "key.keyboard.f1", 1.0),
        ("F2", "key.keyboard.f2", 1.0),
        ("F3", "key.keyboard.f3", 1.0),
        ("F4", "key.keyboard.f4", 1.0),
        ("F5", "key.keyboard.f5", 1.0),
        ("F6", "key.keyboard.f6", 1.0),
        ("F7", "key.keyboard.f7", 1.0),
        ("F8", "key.keyboard.f8", 1.0),
        ("F9", "key.keyboard.f9", 1.0),
        ("F10", "key.keyboard.f10", 1.0),
        ("F11", "key.keyboard.f11", 1.0),
        ("F12", "key.keyboard.f12", 1.0),
        ("PRT", "key.keyboard.print.screen", 1.05),
        ("SCR", "key.keyboard.scroll.lock", 1.05),
        ("PAU", "key.keyboard.pause", 1.05),
    ],
    &[
        ("`", "key.keyboard.grave.accent", 1.0),
        ("1", "key.keyboard.1", 1.0),
        ("2", "key.keyboard.2", 1.0),
        ("3", "key.keyboard.3", 1.0),
        ("4", "key.keyboard.4", 1.0),
        ("5", "key.keyboard.5", 1.0),
        ("6", "key.keyboard.6", 1.0),
        ("7", "key.keyboard.7", 1.0),
        ("8", "key.keyboard.8", 1.0),
        ("9", "key.keyboard.9", 1.0),
        ("0", "key.keyboard.0", 1.0),
        ("-", "key.keyboard.minus", 1.0),
        ("=", "key.keyboard.equal", 1.0),
        ("BKSP", "key.keyboard.backspace", 1.65),
        ("INS", "key.keyboard.insert", 1.05),
        ("HOME", "key.keyboard.home", 1.1),
        ("PGUP", "key.keyboard.page.up", 1.1),
        ("NLK", "key.keyboard.num.lock", 1.05),
        ("KP/", "key.keyboard.keypad.divide", 1.0),
        ("KP*", "key.keyboard.keypad.multiply", 1.0),
        ("KP-", "key.keyboard.keypad.subtract", 1.0),
    ],
    &[
        ("TAB", "key.keyboard.tab", 1.35),
        ("Q", "key.keyboard.q", 1.0),
        ("W", "key.keyboard.w", 1.0),
        ("E", "key.keyboard.e", 1.0),
        ("R", "key.keyboard.r", 1.0),
        ("T", "key.keyboard.t", 1.0),
        ("Y", "key.keyboard.y", 1.0),
        ("U", "key.keyboard.u", 1.0),
        ("I", "key.keyboard.i", 1.0),
        ("O", "key.keyboard.o", 1.0),
        ("P", "key.keyboard.p", 1.0),
        ("[", "key.keyboard.left.bracket", 1.0),
        ("]", "key.keyboard.right.bracket", 1.0),
        ("\\", "key.keyboard.backslash", 1.3),
        ("DEL", "key.keyboard.delete", 1.05),
        ("END", "key.keyboard.end", 1.1),
        ("PGDN", "key.keyboard.page.down", 1.1),
        ("KP7", "key.keyboard.keypad.7", 1.0),
        ("KP8", "key.keyboard.keypad.8", 1.0),
        ("KP9", "key.keyboard.keypad.9", 1.0),
        ("KP+", "key.keyboard.keypad.add", 1.0),
    ],
    &[
        ("CAPS", "key.keyboard.caps.lock", 1.55),
        ("A", "key.keyboard.a", 1.0),
        ("S", "key.keyboard.s", 1.0),
        ("D", "key.keyboard.d", 1.0),
        ("F", "key.keyboard.f", 1.0),
        ("G", "key.keyboard.g", 1.0),
        ("H", "key.keyboard.h", 1.0),
        ("J", "key.keyboard.j", 1.0),
        ("K", "key.keyboard.k", 1.0),
        ("L", "key.keyboard.l", 1.0),
        (";", "key.keyboard.semicolon", 1.0),
        ("'", "key.keyboard.apostrophe", 1.0),
        ("ENTER", "key.keyboard.enter", 1.75),
        ("KP4", "key.keyboard.keypad.4", 1.0),
        ("KP5", "key.keyboard.keypad.5", 1.0),
        ("KP6", "key.keyboard.keypad.6", 1.0),
        ("KP=", "key.keyboard.keypad.equal", 1.0),
    ],
    &[
        ("LSHIFT", "key.keyboard.left.shift", 1.85),
        ("Z", "key.keyboard.z", 1.0),
        ("X", "key.keyboard.x", 1.0),
        ("C", "key.keyboard.c", 1.0),
        ("V", "key.keyboard.v", 1.0),
        ("B", "key.keyboard.b", 1.0),
        ("N", "key.keyboard.n", 1.0),
        ("M", "key.keyboard.m", 1.0),
        (",", "key.keyboard.comma", 1.0),
        (".", "key.keyboard.period", 1.0),
        ("/", "key.keyboard.slash", 1.0),
        ("RSHIFT", "key.keyboard.right.shift", 1.85),
        ("UP", "key.keyboard.up", 1.15),
        ("KP1", "key.keyboard.keypad.1", 1.0),
        ("KP2", "key.keyboard.keypad.2", 1.0),
        ("KP3", "key.keyboard.keypad.3", 1.0),
        ("KPENT", "key.keyboard.keypad.enter", 1.15),
    ],
    &[
        ("LCTRL", "key.keyboard.left.control", 1.35),
        ("LWIN", "key.keyboard.left.win", 1.15),
        ("LALT", "key.keyboard.left.alt", 1.25),
        ("SPACE", "key.keyboard.space", 4.2),
        ("RALT", "key.keyboard.right.alt", 1.25),
        ("MENU", "key.keyboard.menu", 1.15),
        ("RCTRL", "key.keyboard.right.control", 1.35),
        ("LEFT", "key.keyboard.left", 1.1),
        ("DOWN", "key.keyboard.down", 1.1),
        ("RIGHT", "key.keyboard.right", 1.1),
        ("KP0", "key.keyboard.keypad.0", 1.6),
        ("KP.", "key.keyboard.keypad.period", 1.0),
    ],
];

const MOUSE_AND_ACTION_ITEMS: &[(&str, &str)] = &[
    ("Left Click (LMB)", "key.mouse.left"),
    ("Right Click (RMB)", "key.mouse.right"),
    ("Middle Click (MMB)", "key.mouse.middle"),
    ("Mouse 4 (Back)", "key.mouse.4"),
    ("Mouse 5 (Forward)", "key.mouse.5"),
    ("Scroll Wheel Up", "mouse.scroll.up"),
    ("Scroll Wheel Down", "mouse.scroll.down"),
    ("Toggle Virtual Mouse", "mouse.virtual.toggle"),
    ("Mouse Look Area", "mouse.look"),
    ("Action: Attack / Mine", "action:key.attack"),
    ("Action: Use / Place", "action:key.use"),
    ("Action: Jump", "action:key.jump"),
    ("Action: Sneak", "action:key.sneak"),
    ("Action: Sprint", "action:key.sprint"),
    ("Action: Swap Hands (F)", "action:key.swapOffhand"),
    ("Action: Drop Item (Q)", "action:key.drop"),
    ("Action: Inventory (E)", "action:key.inventory"),
    ("Action: Pick Block", "action:key.pickItem"),
    ("Action: Open Chat (T)", "action:key.chat"),
    ("Action: Command (/)", "action:key.command"),
    ("Action: Player List", "action:key.playerlist"),
    ("Action: Hide HUD (F1)", "action:key.toggleGui"),
    ("Action: Screenshot (F2)", "action:key.screenshot"),
    ("Action: Debug Info (F3)", "action:key.debug.overlay"),
    ("Action: Perspective (F5)", "action:key.togglePerspective"),
    ("Action: Advancements (L)", "action:key.advancements"),
];

fn draw_input_picker_modal(
    ui: &mut crate::ui_theme::Ui<'_>,
    w: f32,
    h: f32,
    step: crate::touch::PickerStep,
    kb_scroll_x: f32,
    for_custom_send: bool,
    mcpe_exclusive: bool,
    bw_theme: bool,
) {
    use crate::touch::PickerStep;
    use crate::ui_theme::Face;
    let theme = crate::ui_theme::active_theme();
    let (bg_card, bg_hi, border_c, accent, accent_soft, text_c, muted_c, danger_c) = if bw_theme {
        (
            rgba(0, 0, 0, 248),
            rgba(22, 22, 26, 244),
            rgba(255, 255, 255, 78),
            rgba(255, 255, 255, 255),
            rgba(46, 46, 52, 248),
            rgba(255, 255, 255, 255),
            rgba(194, 194, 200, 255),
            rgba(240, 240, 244, 255),
        )
    } else {
        (
            theme.bg_solid,
            theme.surface,
            theme.border,
            theme.accent,
            theme.surface_selected,
            theme.text_primary,
            theme.text_muted,
            theme.danger,
        )
    };

    ui.fill(0.0, 0.0, w, h, if bw_theme { rgba(0, 0, 0, 205) } else { rgba(4, 5, 10, 195) });
    match step {
        PickerStep::AskInputType { event_slot } => {
            let dw = (w - 36.0).clamp(270.0, 440.0);
            let dh = 236.0;
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            ui.rounded(dx + 2.0, dy + 5.0, dw, dh, 18.0, rgba(0, 0, 0, 150));
            ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
            ui.border(dx, dy, dw, dh, 18.0, accent);
            let title_str = if for_custom_send {
                "Send Custom Input — Choose Input Type".to_owned()
            } else if mcpe_exclusive {
                "MCPE Custom Button — Choose Exclusive Binding".to_owned()
            } else {
                format!("Assign Event Slot #{} — Choose Input Type", event_slot + 1)
            };
            let sub_str = if for_custom_send {
                "Tap any physical key, use System Keyboard, or pick from the PC layout below."
            } else if mcpe_exclusive {
                "Exclusive binding: Keyboard OR Mouse (selecting one clears the other)."
            } else {
                "Each button can trigger up to 4 Keyboard, Mouse, or Action events simultaneously."
            };
            ui.text(
                Face::SemiBold,
                13.5,
                dx + 16.0,
                dy + 12.0,
                text_c,
                &title_str,
            );
            ui.text(Face::Regular, 10.0, dx + 16.0, dy + 31.0, muted_c, sub_str);
            let cw = (dw - 40.0) * 0.5;
            let r_kb = [dx + 16.0, dy + 54.0, cw, 76.0];
            let r_ms = [dx + 24.0 + cw, dy + 54.0, cw, 76.0];
            for (r, title, sub) in [
                (
                    r_kb,
                    "Keyboard Key",
                    if mcpe_exclusive {
                        "Bind 1 key (clears mouse binding)"
                    } else {
                        "Full 104-Key horizontal PC keyboard"
                    },
                ),
                (
                    r_ms,
                    "Mouse & Actions",
                    if mcpe_exclusive {
                        "Bind 1 mouse/action (clears key)"
                    } else {
                        "LMB, RMB, MMB, Scroll, Look & Actions"
                    },
                ),
            ] {
                ui.rounded(r[0], r[1], r[2], r[3], 14.0, accent_soft);
                ui.border(r[0], r[1], r[2], r[3], 14.0, accent);
                ui.centered_fit(
                    Face::SemiBold,
                    13.0,
                    [r[0] + 6.0, r[1] + 14.0, r[2] - 12.0, 24.0],
                    text_c,
                    title,
                );
                ui.centered_fit(
                    Face::Regular,
                    10.0,
                    [r[0] + 6.0, r[1] + 40.0, r[2] - 12.0, 22.0],
                    muted_c,
                    sub,
                );
            }
            let r_sys = [dx + 16.0, dy + 138.0, dw - 32.0, 34.0];
            ui.rounded(r_sys[0], r_sys[1], r_sys[2], r_sys[3], 10.0, bg_hi);
            ui.border(r_sys[0], r_sys[1], r_sys[2], r_sys[3], 10.0, accent);
            ui.centered_fit(
                Face::SemiBold,
                10.8,
                r_sys,
                accent,
                "Type / Capture Key (System / Physical Keyboard)",
            );
            if for_custom_send {
                let r_can = [dx + 16.0, dy + 180.0, dw - 32.0, 36.0];
                ui.rounded(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, bg_hi);
                ui.border(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, border_c);
                ui.centered_fit(Face::Medium, 11.0, r_can, text_c, "Cancel");
            } else {
                let r_clr = [dx + 16.0, dy + 180.0, cw, 36.0];
                let r_can = [dx + 24.0 + cw, dy + 180.0, cw, 36.0];
                ui.rounded(r_clr[0], r_clr[1], r_clr[2], r_clr[3], 10.0, bg_hi);
                ui.border(r_clr[0], r_clr[1], r_clr[2], r_clr[3], 10.0, danger_c);
                ui.centered_fit(
                    Face::SemiBold,
                    11.0,
                    r_clr,
                    danger_c,
                    "Clear / Unassign Slot",
                );
                ui.rounded(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, bg_hi);
                ui.border(r_can[0], r_can[1], r_can[2], r_can[3], 10.0, border_c);
                ui.centered_fit(Face::Medium, 11.0, r_can, text_c, "Cancel");
            }
        }
        PickerStep::Keyboard { event_slot } => {
            let dw = (w - 20.0).clamp(300.0, 620.0);
            let dh = (h - 20.0).clamp(235.0, 340.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            ui.rounded(dx + 2.0, dy + 5.0, dw, dh, 18.0, rgba(0, 0, 0, 150));
            ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
            ui.border(dx, dy, dw, dh, 18.0, accent);
            let title_str = if for_custom_send {
                "Send Custom Input — Full PC Keyboard (Tap or Swipe)".to_owned()
            } else if mcpe_exclusive {
                "MCPE Exclusive Keyboard Binding (Clears Mouse)".to_owned()
            } else {
                format!(
                    "Event Slot #{} — Full PC Keyboard (Tap or Swipe)",
                    event_slot + 1
                )
            };
            ui.text_clipped(
                Face::SemiBold,
                12.5,
                dx + 14.0,
                dy + 11.0,
                (dw - 290.0).max(80.0),
                text_c,
                &title_str,
            );
            let b_ime = [dx + dw - 266.0, dy + 8.0, 64.0, 26.0];
            let b_left = [dx + dw - 196.0, dy + 8.0, 56.0, 26.0];
            let b_right = [dx + dw - 134.0, dy + 8.0, 56.0, 26.0];
            let b_back = [dx + dw - 72.0, dy + 8.0, 58.0, 26.0];
            for (r, lbl) in [
                (b_ime, "Sys Key"),
                (b_left, "◀ Left"),
                (b_right, "Right ▶"),
                (b_back, "Back"),
            ] {
                ui.rounded(r[0], r[1], r[2], r[3], 8.0, bg_hi);
                ui.border(r[0], r[1], r[2], r[3], 8.0, border_c);
                ui.centered_fit(Face::SemiBold, 10.0, r, text_c, lbl);
            }

            let kb_top = dy + 40.0;
            let kb_h = dh - 56.0;
            let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
            let key_unit = 38.0;
            let gap = 4.0;
            ui.set_clip(Some([dx + 10.0, kb_top, dw - 20.0, kb_h]));
            for (r_idx, row) in KB_ROWS.iter().enumerate() {
                let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                let mut kx = dx + 14.0 - kb_scroll_x;
                for (lbl, _tok, width_mult) in *row {
                    let kw = key_unit * (*width_mult) + (*width_mult - 1.0).max(0.0) * gap;
                    if kx + kw >= dx + 8.0 && kx <= dx + dw - 8.0 {
                        ui.rounded(kx, ry, kw, row_h, 6.0, bg_hi);
                        ui.border(kx, ry, kw, row_h, 6.0, border_c);
                        ui.centered_fit(
                            Face::SemiBold,
                            10.0,
                            [kx + 2.0, ry, kw - 4.0, row_h],
                            text_c,
                            lbl,
                        );
                    }
                    kx += kw + gap;
                }
            }
            ui.set_clip(None);

            let max_kb_scroll = (880.0 - (dw - 28.0)).max(1.0);
            let track_w = dw - 28.0;
            let thumb_w = (track_w * (track_w / 880.0)).clamp(48.0, track_w);
            let thumb_x =
                dx + 14.0 + (track_w - thumb_w) * (kb_scroll_x / max_kb_scroll).clamp(0.0, 1.0);
            ui.rounded(dx + 14.0, dy + dh - 12.0, track_w, 5.0, 2.5, border_c);
            ui.rounded(thumb_x, dy + dh - 12.0, thumb_w, 5.0, 2.5, accent);
        }
        PickerStep::MouseAndActions { event_slot } => {
            let dw = (w - 24.0).clamp(290.0, 560.0);
            let dh = (h - 20.0).clamp(235.0, 355.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
            ui.border(dx, dy, dw, dh, 18.0, accent);
            let title_str = if for_custom_send {
                "Send Custom Input — Mouse Buttons, Wheel & Actions".to_owned()
            } else if mcpe_exclusive {
                "MCPE Exclusive Mouse/Action Binding (Clears Keyboard Binding)".to_owned()
            } else {
                format!(
                    "Event Slot #{} — Mouse Buttons, Wheel, Look & Actions",
                    event_slot + 1
                )
            };
            ui.text(
                Face::SemiBold,
                13.0,
                dx + 14.0,
                dy + 11.0,
                text_c,
                &title_str,
            );
            let b_back = [dx + dw - 72.0, dy + 8.0, 58.0, 26.0];
            ui.rounded(b_back[0], b_back[1], b_back[2], b_back[3], 8.0, bg_hi);
            ui.border(b_back[0], b_back[1], b_back[2], b_back[3], 8.0, border_c);
            ui.centered_fit(Face::SemiBold, 10.0, b_back, text_c, "Back");

            let cols = 3usize;
            let rows = MOUSE_AND_ACTION_ITEMS.len().div_ceil(cols).max(1);
            let gx = dx + 12.0;
            let gy = dy + 40.0;
            let gw = dw - 24.0;
            let gh = dh - 50.0;
            let cw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
            let ch = ((gh - (rows as f32 - 1.0) * 4.0) / rows as f32).clamp(21.0, 32.0);
            for (i, (lbl, tok)) in MOUSE_AND_ACTION_ITEMS.iter().enumerate() {
                let col = i % cols;
                let row = i / cols;
                let cx = gx + col as f32 * (cw + 6.0);
                let cy = gy + row as f32 * (ch + 4.0);
                let is_mouse = tok.starts_with("key.mouse.") || tok.starts_with("mouse.");
                ui.rounded(
                    cx,
                    cy,
                    cw,
                    ch,
                    7.0,
                    if is_mouse { accent_soft } else { bg_hi },
                );
                ui.border(
                    cx,
                    cy,
                    cw,
                    ch,
                    7.0,
                    if is_mouse { accent } else { border_c },
                );
                ui.centered_fit(
                    Face::Medium,
                    9.5,
                    [cx + 3.0, cy, cw - 6.0, ch],
                    if is_mouse { accent } else { text_c },
                    lbl,
                );
            }
        }
        PickerStep::None => {}
    }
}

/// Renders the live GoyControl HUD (Analog Joystick + Static Buttons) in-game and in menus
/// using the exact same `Ui::mobile` TrueType & rounded-shape pipeline as `draw_hud_editor`.
pub fn draw_goy_hud(
    tc: &crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    in_menu: bool,
    virtual_mouse: bool,
) {
    use crate::touch::{Control, ControlStyle};
    use crate::ui_theme::{Ui, color};
    crate::touch::set_current_hud_opacity_scale(if tc.editor.is_some() {
        1.0
    } else {
        tc.hud_opacity_scale
    });
    if tc.editor.is_some() || tc.hud_opacity_scale <= 0.001 {
        return;
    }
    let is_goy_system = tc.settings.control_style == ControlStyle::GoyControl;
    let has_goy_move = is_goy_system || tc.placement(Control::Move).use_goy_style;
    let has_goy_builtin = Control::ALL
        .into_iter()
        .any(|c| c != Control::Move && tc.visible_for_state(c, in_menu) && (is_goy_system || tc.placement(c).use_goy_style));
    let has_goy_custom = is_goy_system
        || (0..tc.settings.custom_buttons.len()).any(|i| {
            tc.custom_visible_for_state(i, in_menu) && tc.custom_placement(i).use_goy_style
        });
    if !has_goy_move && !has_goy_builtin && !has_goy_custom {
        return;
    }
    let hud_scale = tc.hud_opacity_scale.clamp(0.0, 1.0);
    let mut ui = Ui::mobile(a, b, (0.0, 0.0));
    let (w, h) = (ui.w, ui.h);
    let text_c = color::hex(0xF4EFFA, 255);
    let state = tc.state();

    // 1. GoyControl Analog Joystick (Control::Move)
    if has_goy_move && tc.visible_for_state(Control::Move, in_menu) {
        let p = tc.placement(Control::Move);
        let mut p_draw = p;
        if tc.settings.mode != crate::touch::Mode::Dpad
            && let Some(anchor) = tc.move_anchor()
        {
            p_draw.center = anchor;
        }
        let [nx, ny, nw, nh] = p_draw.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let alpha = (p.opacity.clamp(0.0, 1.0) * hud_scale).clamp(0.0, 1.0);
        if alpha > 0.001 {
            let js = &tc.settings.joystick_style;
            let side = rw.min(rh);
            let jx = rx + (rw - side) * 0.5;
            let jy = ry + (rh - side) * 0.5;
            let base_fill = color::with_alpha(js.base_color, js.base_opacity * alpha);
            let base_stroke = color::with_alpha(js.base_stroke_color, js.base_opacity * alpha);
            let br = (side * (js.base_roundness / 100.0)).clamp(0.0, side * 0.5);
            if (base_fill >> 24) > 0 {
                ui.rounded(jx, jy, side, side, br, base_fill);
            }
            if js.base_stroke > 0.1 && (base_stroke >> 24) > 0 {
                ui.stroke_rounded(jx, jy, side, side, br, js.base_stroke, base_stroke);
            }
            let kside = side * js.stick_size.clamp(0.20, 0.85);
            let kx = jx + (side - kside) * 0.5 + state.left[0] * (side * 0.26);
            let ky = jy + (side - kside) * 0.5 - state.left[1] * (side * 0.26);
            let kr = kside * 0.5;
            let stick_fill = color::with_alpha(js.stick_color, js.stick_opacity * alpha);
            let stick_stroke = color::with_alpha(js.stick_stroke_color, js.stick_opacity * alpha);
            if (stick_fill >> 24) > 0 {
                ui.rounded(kx, ky, kside, kside, kr, stick_fill);
            }
            if js.stick_stroke > 0.1 && (stick_stroke >> 24) > 0 {
                ui.stroke_rounded(kx, ky, kside, kside, kr, js.stick_stroke, stick_stroke);
            }
        }
    }

    // 1b. Built-in Controls configured with GoyControl Static Style
    for c in Control::ALL {
        if c == Control::Move || !tc.visible_for_state(c, in_menu) {
            continue;
        }
        let p = tc.placement(c);
        if !is_goy_system && !p.use_goy_style {
            continue;
        }
        let [nx, ny, nw, nh] = p.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let alpha = (p.opacity.clamp(0.0, 1.0) * hud_scale).clamp(0.0, 1.0);
        if alpha <= 0.001 {
            continue;
        }
        let fill_c = color::with_alpha(p.button_color, alpha);
        let stroke_c = color::with_alpha(p.stroke_color, alpha);
        let shadow_c = color::with_alpha(p.shadow_color, alpha);
        ui.goy_box(
            rx,
            ry,
            rw,
            rh,
            p.shape,
            p.roundness,
            fill_c,
            p.stroke_size,
            p.stroke_margin,
            p.stroke_roundness,
            stroke_c,
            0.0,
            shadow_c,
        );
        let fg_col = color::with_alpha(text_c, alpha);
        let lbl = tc.settings.control_label(c);
        draw_button_icon_and_label(&mut ui, [rx, ry, rw, rh], p.icon_id, lbl, fg_col);
    }

    // 2. Custom GoyControl Static Buttons (0-100% opacity + press scale micro-interaction)
    for (i, custom) in tc.settings.custom_buttons.iter().enumerate() {
        if !tc.custom_visible_for_state(i, in_menu) {
            continue;
        }
        let p = tc.custom_placement(i);
        if !is_goy_system && !p.use_goy_style {
            continue;
        }
        let [nx, ny, nw, nh] = p.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let alpha = (p.opacity.clamp(0.0, 1.0) * hud_scale).clamp(0.0, 1.0);
        if alpha <= 0.001 {
            continue;
        }
        let label = custom.label();

        let is_vmouse_on = custom.key == "mouse.virtual.toggle" && virtual_mouse;
        let held = tc.custom_button_held(i) && !p.toggleable;
        let toggled = (tc.custom_button_toggled(i) && p.toggleable) || is_vmouse_on;



        let raw_fill = if held {
            p.on_click_color
        } else if toggled {
            p.on_toggle_color
        } else {
            p.button_color
        };
        let raw_stroke = if held {
            p.on_click_stroke_color
        } else if toggled {
            p.on_toggle_stroke_color
        } else {
            p.stroke_color
        };
        let fill_c = color::with_alpha(raw_fill, alpha);
        let stroke_c = color::with_alpha(raw_stroke, alpha);
        let shadow_c = color::with_alpha(p.shadow_color, alpha);
        ui.goy_box(
            rx,
            ry,
            rw,
            rh,
            p.shape,
            p.roundness,
            fill_c,
            p.stroke_size,
            p.stroke_margin,
            p.stroke_roundness,
            stroke_c,
            0.0,
            shadow_c,
        );
        let fg_col = color::with_alpha(text_c, alpha);
        draw_button_icon_and_label(&mut ui, [rx, ry, rw, rh], p.icon_id, &label, fg_col);
    }

    ui.finish();
}

fn draw_mcpe_custom_plate(
    ui: &mut crate::ui_theme::Ui,
    rect: [f32; 4],
    alpha: f32,
    pressed: bool,
) {
    let [rx, ry, rw, rh] = rect;
    let a_u8 = (alpha * 255.0).round() as u8;
    ui.rounded(rx, ry, rw, rh, 3.0, rgba(18, 20, 24, a_u8));
    let inner_col = if pressed {
        rgba(38, 42, 48, (alpha * 225.0).round() as u8)
    } else {
        rgba(58, 62, 70, (alpha * 210.0).round() as u8)
    };
    ui.rounded(
        rx + 2.0,
        ry + 2.0,
        (rw - 4.0).max(2.0),
        (rh - 4.0).max(2.0),
        2.0,
        inner_col,
    );
    let (top_left_col, bot_right_col) = if pressed {
        (
            rgba(28, 30, 36, (alpha * 215.0).round() as u8),
            rgba(145, 152, 164, (alpha * 185.0).round() as u8),
        )
    } else {
        (
            rgba(175, 182, 194, (alpha * 195.0).round() as u8),
            rgba(28, 30, 36, (alpha * 215.0).round() as u8),
        )
    };
    ui.fill(rx + 2.0, ry + 2.0, (rw - 4.0).max(2.0), 2.0, top_left_col);
    ui.fill(rx + 2.0, ry + 4.0, 2.0, (rh - 6.0).max(2.0), top_left_col);
    ui.fill(
        rx + 2.0,
        ry + rh - 4.0,
        (rw - 4.0).max(2.0),
        2.0,
        bot_right_col,
    );
    ui.fill(
        rx + rw - 4.0,
        ry + 4.0,
        2.0,
        (rh - 6.0).max(2.0),
        bot_right_col,
    );
}

/// A label cut to `max` characters with an ellipsis: the hub draws with a fixed width and
/// has no measuring pass of its own.
fn ellipsis(text: &str, max: usize) -> String {
    let max = max.max(4);
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max - 1).collect();
    cut.push('…');
    cut
}

/// The Record tab's cells: the same rectangles are drawn and hit-tested, so a chip cannot
/// drift away from the place a finger finds it.
struct RecordCells {
    start: [f32; 4],
    resolution: [[f32; 4]; 6],
    fps: [[f32; 4]; 6],
    audio: [[f32; 4]; 3],
    overlay: [f32; 4],
    recordings: [[f32; 4]; 3],
}

fn record_cells(cx: f32, cw: f32, body_y: f32) -> RecordCells {
    let gap = 6.0;
    let chip_w = (cw - gap * 2.0) / 3.0;
    let chip_h = 26.0;
    let mut resolution = [[0.0f32; 4]; 6];
    let mut fps = [[0.0f32; 4]; 6];
    for i in 0..6 {
        let x = cx + (i % 3) as f32 * (chip_w + gap);
        let row = (i / 3) as f32 * (chip_h + 6.0);
        resolution[i] = [x, body_y + 80.0 + row, chip_w, chip_h];
        fps[i] = [x, body_y + 162.0 + row, chip_w, chip_h];
    }
    let mut audio = [[0.0f32; 4]; 3];
    for (i, cell) in audio.iter_mut().enumerate() {
        *cell = [
            cx + i as f32 * (chip_w + gap),
            body_y + 244.0,
            chip_w,
            chip_h,
        ];
    }
    let mut recordings = [[0.0f32; 4]; 3];
    for (i, cell) in recordings.iter_mut().enumerate() {
        *cell = [cx, body_y + 354.0 + i as f32 * 40.0, cw, 36.0];
    }
    RecordCells {
        start: [cx + cw - 100.0, body_y + 24.0, 88.0, 32.0],
        resolution,
        fps,
        audio,
        overlay: [cx, body_y + 282.0, cw, 46.0],
        recordings,
    }
}

fn hub_metrics(w: f32, h: f32) -> (f32, f32, f32, f32, f32, f32, f32, f32, f32) {
    let dw = (w - 16.0).clamp(368.0, 628.0);
    let dh = (h - 12.0).clamp(276.0, 424.0);
    let dx = (w - dw) * 0.5;
    let dy = (h - dh) * 0.5;
    let side_w = (dw * 0.275).clamp(128.0, 162.0);
    let cx = dx + side_w + 16.0;
    let cw = dw - side_w - 32.0;
    let body_top = dy + 54.0;
    let body_h = (dh - 64.0).max(168.0);
    (dw, dh, dx, dy, side_w, cx, cw, body_top, body_h)
}

struct HubSidebarRects {
    quick_studio: [f32; 4],
    tabs: [[f32; 4]; 10],
    done_btn: [f32; 4],
    close_x: [f32; 4],
    mode_badge: [f32; 4],
    booster_pill: [f32; 4],
}

fn hub_sidebar_rects(dx: f32, dy: f32, dw: f32, dh: f32, side_w: f32) -> HubSidebarRects {
    let pad = 10.0;
    let inner_w = (side_w - pad * 2.0).max(88.0);
    let quick_studio = [dx + pad, dy + 44.0, inner_w, 25.0];
    let done_h = 30.0;
    let done_y = dy + dh - pad - done_h;
    let tabs_top = quick_studio[1] + quick_studio[3] + 7.0;
    let tabs_avail = (done_y - 7.0 - tabs_top).max(120.0);
    let gap = 4.0;
    let tab_h = ((tabs_avail - gap * 9.0) / 10.0).clamp(14.0, 30.0);
    let mut tabs = [[0.0f32; 4]; 10];
    for (i, slot) in tabs.iter_mut().enumerate() {
        *slot = [
            dx + pad,
            tabs_top + i as f32 * (tab_h + gap),
            inner_w,
            tab_h,
        ];
    }
    let done_btn = [dx + pad, done_y, inner_w, done_h];
    let close_x = [dx + dw - 38.0, dy + 10.0, 26.0, 26.0];
    let mode_badge = [dx + dw - 146.0, dy + 11.0, 100.0, 24.0];
    let booster_pill = [dx + dw - 228.0, dy + 11.0, 76.0, 24.0];
    HubSidebarRects {
        quick_studio,
        tabs,
        done_btn,
        close_x,
        mode_badge,
        booster_pill,
    }
}

struct BoosterLayout {
    shell: [f32; 4],
    close_x: [f32; 4],
    hub_btn: [f32; 4],
    view_toggle: [f32; 4],
    left_rect: [f32; 4],
    right_rect: [f32; 4],
    gauges: [[f32; 4]; 4],
    modes: [[f32; 4]; 4],
    ram_bar: [f32; 4],
    frame_bar: [f32; 4],
    tiles: [[f32; 4]; 6],
    dnd_btn: [f32; 4],
    vsync_btn: [f32; 4],
    ignite_btn: [f32; 4],
}

fn booster_layout(w: f32, h: f32, slide_y: f32) -> BoosterLayout {
    let dw = (w - 16.0).clamp(368.0, 624.0);
    let dh = (h - 14.0).clamp(264.0, 396.0);
    let dx = (w - dw) * 0.5;
    let dy = (h - dh) * 0.5 + slide_y;

    let close_x = [dx + dw - 38.0, dy + 9.0, 26.0, 26.0];
    let hub_btn = [close_x[0] - 92.0, dy + 10.0, 84.0, 24.0];
    let view_toggle = [hub_btn[0] - 114.0, dy + 10.0, 106.0, 24.0];

    let body_y = dy + 50.0;
    let body_h = (dh - 60.0).max(180.0);
    let inner_w = dw - 24.0;
    let col_gap = 12.0;
    let left_w = (inner_w * 0.48).clamp(166.0, 290.0);
    let right_x = dx + 12.0 + left_w + col_gap;
    let right_w = (inner_w - left_w - col_gap).max(166.0);
    let left_x = dx + 12.0;

    let gauge_area_h = (body_h * 0.52).clamp(108.0, 174.0);
    let gw = (left_w - 8.0) * 0.5;
    let gh = (gauge_area_h - 8.0) * 0.5;
    let gauges = [
        [left_x, body_y, gw, gh],
        [left_x + gw + 8.0, body_y, gw, gh],
        [left_x, body_y + gh + 8.0, gw, gh],
        [left_x + gw + 8.0, body_y + gh + 8.0, gw, gh],
    ];

    let mode_y = body_y + gauge_area_h + 8.0;
    let mode_w = (left_w - 9.0) * 0.25;
    let modes = [
        [left_x, mode_y, mode_w, 30.0],
        [left_x + (mode_w + 3.0), mode_y, mode_w, 30.0],
        [left_x + (mode_w + 3.0) * 2.0, mode_y, mode_w, 30.0],
        [left_x + (mode_w + 3.0) * 3.0, mode_y, mode_w, 30.0],
    ];

    let bars_y = mode_y + 38.0;
    let bar_h = ((body_y + body_h - bars_y - 6.0) * 0.5).clamp(24.0, 38.0);
    let ram_bar = [left_x, bars_y, left_w, bar_h];
    let frame_bar = [left_x, bars_y + bar_h + 6.0, left_w, bar_h];

    let act_h = 36.0;
    let act_y = body_y + body_h - act_h;
    let grid_h = (act_y - body_y - 10.0).max(114.0);
    let tw = (right_w - 8.0) * 0.5;
    let th = ((grid_h - 16.0) / 3.0).clamp(32.0, 56.0);
    let mut tiles = [[0.0f32; 4]; 6];
    for i in 0..6 {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        tiles[i] = [
            right_x + col * (tw + 8.0),
            body_y + row * (th + 8.0),
            tw,
            th,
        ];
    }

    let dnd_w = 56.0;
    let vsync_w = 62.0;
    let dnd_btn = [right_x, act_y, dnd_w, act_h];
    let vsync_btn = [right_x + dnd_w + 6.0, act_y, vsync_w, act_h];
    let ignite_x = right_x + dnd_w + vsync_w + 12.0;
    let ignite_btn = [
        ignite_x,
        act_y,
        (right_x + right_w - ignite_x).max(80.0),
        act_h,
    ];

    BoosterLayout {
        shell: [dx, dy, dw, dh],
        close_x,
        hub_btn,
        view_toggle,
        left_rect: [left_x, body_y, left_w, body_h],
        right_rect: [right_x, body_y, right_w, body_h],
        gauges,
        modes,
        ram_bar,
        frame_bar,
        tiles,
        dnd_btn,
        vsync_btn,
        ignite_btn,
    }
}

pub fn floating_button_rect(w: f32, h: f32, settings: &crate::touch::Settings) -> [f32; 4] {
    let scale = if settings.overlay_scale.is_finite() {
        settings.overlay_scale.clamp(0.5, 2.0)
    } else {
        1.0
    };
    let (base_w, base_h) = if settings.control_style == crate::touch::ControlStyle::Mcpe {
        (44.0, 36.0)
    } else {
        (48.0, 42.0)
    };
    let btn_w = (base_w * scale).clamp(22.0, (w - 8.0).max(22.0));
    let btn_h = (base_h * scale).clamp(18.0, (h - 8.0).max(18.0));
    let max_x = (w - btn_w - 4.0).max(4.0);
    let max_y = (h - btn_h - 4.0).max(4.0);
    let btn_x = (settings.floating_menu_pos[0] * w).clamp(4.0, max_x);
    let btn_y = (settings.floating_menu_pos[1] * h).clamp(4.0, max_y);
    [btn_x, btn_y, btn_w, btn_h]
}

impl TouchUi {
    pub fn overlay_button_visible(
        &self,
        settings: &crate::touch::Settings,
        in_menu: bool,
        hiding_controls: bool,
    ) -> bool {
        if self.floating_menu {
            return true;
        }
        if settings.overlay_hidden {
            return false;
        }
        in_menu || hiding_controls
    }
    fn step_animations(&self) -> (f32, f32) {
        let open_target = if self.floating_menu { 1.0 } else { 0.0 };
        if crate::ui_theme::motion::game_animations_disabled()
            || crate::ui_theme::motion::reduced_motion()
        {
            self.open_anim.set(open_target);
            self.tab_anim.set(1.0);
            return (open_target, 1.0);
        }
        let now = Instant::now();
        let dt = self
            .last_anim_tick
            .get()
            .map(|prev| now.duration_since(prev).as_secs_f32().clamp(0.0, 0.08))
            .unwrap_or(0.016);
        self.last_anim_tick.set(Some(now));
        let open_target = if self.floating_menu { 1.0 } else { 0.0 };
        let cur_open = self.open_anim.get();
        let next_open = if (cur_open - open_target).abs() < 0.005 {
            open_target
        } else {
            (cur_open + (open_target - cur_open) * (dt * 14.0).clamp(0.15, 1.0)).clamp(0.0, 1.0)
        };
        self.open_anim.set(next_open);

        let cur_tab = self.tab_anim.get();
        let next_tab = if cur_tab >= 0.995 {
            1.0
        } else {
            (cur_tab + (1.0 - cur_tab) * (dt * 16.0).clamp(0.18, 1.0)).clamp(0.0, 1.0)
        };
        self.tab_anim.set(next_tab);
        (next_open, next_tab)
    }

    /// Handles touch events for the draggable Floating Menu Button and the redesigned Horizon Control Hub.
    pub fn floating_menu_touch(
        &mut self,
        b: &mut HudBatch,
        a: &HudAtlas,
        id: u64,
        phase: TouchPhase,
        p_gui: [f32; 2],
        settings: &mut crate::touch::Settings,
    ) -> Option<InGameMenuAction> {
        use crate::touch::ControlStyle;
        self.goy_mode = settings.control_style == ControlStyle::GoyControl;
        self.pointer_off = !settings.virtual_mouse_enabled;
        let ui = crate::ui_theme::Ui::mobile(a, b, (0.0, 0.0));
        let (w, h) = (ui.w, ui.h);
        let p = [
            p_gui[0] * ui.batch.scale / ui.density,
            p_gui[1] * ui.batch.scale / ui.density,
        ];
        let inside = |r: [f32; 4]| {
            p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
        };

        let btn_rect = floating_button_rect(w, h, settings);
        let [btn_x, btn_y, btn_w, btn_h] = btn_rect;
        let max_btn_x = (w - btn_w - 4.0).max(4.0);
        let max_btn_y = (h - btn_h - 4.0).max(4.0);

        if let Some((drag_id, offset, mut moved)) = self.floating_drag {
            if drag_id == id {
                match phase {
                    TouchPhase::Moved => {
                        let nx = (p[0] - offset[0]).clamp(4.0, max_btn_x);
                        let ny = (p[1] - offset[1]).clamp(4.0, max_btn_y);
                        if (nx - btn_x).hypot(ny - btn_y) > 4.0 {
                            moved = true;
                        }
                        if moved {
                            settings.floating_menu_pos = [
                                (nx / w.max(1.0)).clamp(0.005, (max_btn_x / w.max(1.0)).clamp(0.005, 0.96)),
                                (ny / h.max(1.0)).clamp(0.005, (max_btn_y / h.max(1.0)).clamp(0.005, 0.94)),
                            ];
                        }
                        self.floating_drag = Some((drag_id, offset, moved));
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.floating_drag = None;
                        if moved {
                            let _ = settings.save();
                        } else if phase == TouchPhase::Ended {
                            self.floating_menu = !self.floating_menu;
                            if self.floating_menu {
                                self.booster_mode = true;
                                self.open_anim.set(0.0);
                                self.tab_anim.set(0.0);
                                self.last_anim_tick.set(None);
                                self.hub_scroll = 0.0;
                                self.refresh_presets_cache(settings.control_style);
                            } else {
                                self.presets_modal_open = false;
                                self.preset_dialog = None;
                                self.custom_input_picker = crate::touch::PickerStep::None;
                                self.custom_kb_drag = None;
                                self.sensitivity_dragging = false;
                                self.pointer_size_dragging = false;
                                self.mouse_speed_dragging = false;
                                self.gyro_sensitivity_dragging = false;
                                self.button_scale_dragging = false;
                                self.overlay_scale_dragging = false;
                                self.overlay_opacity_dragging = false;
                                self.music_volume_dragging = false;
                                self.hub_scroll_drag = None;
                            }
                        }
                        return Some(InGameMenuAction::Close);
                    }
                    _ => {}
                }
            }
        }

        if phase == TouchPhase::Started && !self.expanded && inside(btn_rect) {
            self.floating_drag = Some((id, [p[0] - btn_x, p[1] - btn_y], false));
            return Some(InGameMenuAction::Close);
        }

        if !self.floating_menu {
            return None;
        }

        if self.booster_mode {
            let bl = booster_layout(w, h, 0.0);
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            if !inside(bl.shell) || inside(bl.close_x) {
                self.floating_menu = false;
                return Some(InGameMenuAction::Close);
            }
            if inside(bl.hub_btn) {
                self.booster_mode = false;
                self.tab_anim.set(0.0);
                self.last_anim_tick.set(None);
                self.hub_scroll = 0.0;
                self.refresh_presets_cache(settings.control_style);
                return Some(InGameMenuAction::Close);
            }
            if inside(bl.view_toggle) {
                let mid = bl.view_toggle[0] + bl.view_toggle[2] * 0.5;
                self.booster_graph_mode = p[0] < mid;
                return Some(InGameMenuAction::Close);
            }
            for (mi, mr) in bl.modes.iter().enumerate() {
                if inside(*mr) {
                    self.booster_profile = mi as u8;
                    self.booster_120hz = mi >= 2;
                    self.booster_adpf = mi >= 2;
                    let label = match mi {
                        0 => "Eco Mode Active (60Hz • Battery)",
                        1 => "Balanced Mode Active",
                        2 => "Boost Mode Active (120Hz • ADPF)",
                        _ => "Demonic Mode Ignited (Max FPS)",
                    };
                    self.set_booster_toast(label);
                    return Some(InGameMenuAction::BoosterSetProfile(mi as u8));
                }
            }
            if inside(bl.tiles[0]) {
                self.booster_120hz = !self.booster_120hz;
                self.set_booster_toast(if self.booster_120hz {
                    "120Hz Refresh Requested"
                } else {
                    "60Hz Power Refresh Set"
                });
                return Some(InGameMenuAction::BoosterToggle120Hz);
            }
            if inside(bl.tiles[1]) {
                self.set_booster_toast("RAM & Render Caches Purged");
                return Some(InGameMenuAction::BoosterRamPurge);
            }
            if inside(bl.tiles[2]) {
                self.booster_touch_lock = !self.booster_touch_lock;
                self.set_booster_toast(if self.booster_touch_lock {
                    "Touch Lock Armed"
                } else {
                    "Touch Lock Released"
                });
                return Some(InGameMenuAction::BoosterToggleTouchLock);
            }
            if inside(bl.tiles[3]) {
                let next = !self.volt_extras.entity_culling.enabled;
                self.volt_extras.entity_culling.enabled = next;
                self.set_booster_toast(if next {
                    "Entity Culling Enabled"
                } else {
                    "Entity Culling Disabled"
                });
                return Some(InGameMenuAction::BoosterToggleEntityCulling);
            }
            if inside(bl.tiles[4]) {
                let next = !settings.zero_latency_touch;
                self.set_booster_toast(if next {
                    "Zero-Latency Touch Enabled"
                } else {
                    "Zero-Latency Touch Standard"
                });
                return Some(InGameMenuAction::BoosterToggleZeroLatency);
            }
            if inside(bl.tiles[5]) {
                let next = !self.volt_extras.telemetry_hud.show_fps;
                self.volt_extras.telemetry_hud.show_fps = next;
                self.set_booster_toast(if next {
                    "FPS Telemetry HUD Shown"
                } else {
                    "FPS Telemetry HUD Hidden"
                });
                return Some(InGameMenuAction::BoosterToggleFpsHud);
            }
            if inside(bl.dnd_btn) {
                self.booster_dnd = !self.booster_dnd;
                self.set_booster_toast(if self.booster_dnd {
                    "Do Not Disturb On"
                } else {
                    "Do Not Disturb Off"
                });
                return Some(InGameMenuAction::BoosterToggleDnd);
            }
            if inside(bl.vsync_btn) {
                self.telemetry.vsync = !self.telemetry.vsync;
                self.set_booster_toast(if self.telemetry.vsync {
                    "VSync Enabled"
                } else {
                    "VSync Unlocked"
                });
                return Some(InGameMenuAction::BoosterToggleVsync);
            }
            if inside(bl.ignite_btn) {
                self.booster_profile = 3;
                self.booster_120hz = true;
                self.booster_adpf = true;
                self.volt_extras.entity_culling.enabled = true;
                self.volt_extras.telemetry_hud.show_fps = true;
                self.set_booster_toast("NEXORA BOOSTER IGNITED • MAX FPS");
                return Some(InGameMenuAction::BoosterIgnite);
            }
            return Some(InGameMenuAction::Close);
        }

        let (dw, dh, dx, dy, side_w, cx, cw, body_top, body_h) = hub_metrics(w, h);
        let content_pane = [cx - 4.0, body_top, cw + 8.0, body_h];

        // Handle ongoing real-time 3D resolution scale slider drag on Tab 0 (Recommended)
        if self.resolution_scale_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let pct = (((25.0 + t * 75.0) / 5.0).round() as u32 * 5).clamp(25, 100);
                    return Some(InGameMenuAction::SetResolutionScale(pct));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.resolution_scale_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing sensitivity slider drag on Tab 0 (Recommended) or Tab 1 (Controls)
        if self.sensitivity_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.sensitivity_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing virtual-mouse-speed slider drag on Tab 0 or Tab 2
        if self.mouse_speed_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.mouse_speed_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing pointer-size slider drag on Tab 1
        if self.pointer_size_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMousePointerSize(0.5 + t * 1.5));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.pointer_size_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing gyroscope-sensitivity slider drag on Tab 2
        if self.gyro_sensitivity_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetGyroSensitivity(0.10 + t * 2.90));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.gyro_sensitivity_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing global-button-scale slider drag on Tab 3
        if self.button_scale_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetButtonScale(0.50 + t * 1.50));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.button_scale_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // Handle ongoing overlay size/opacity slider drags on Tab 6 (Overlay Menu)
        if self.overlay_scale_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetOverlayScale(0.50 + t * 1.50));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.overlay_scale_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }
        if self.overlay_opacity_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetOverlayOpacity(0.20 + t * 0.80));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.overlay_opacity_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }
        if self.music_volume_dragging {
            match phase {
                TouchPhase::Moved => {
                    let sl_x = cx + 92.0;
                    let sl_w = (cw - 106.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::MusicSetVolume(t));
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.music_volume_dragging = false;
                    return Some(InGameMenuAction::Close);
                }
                _ => {}
            }
        }

        // 0a. If Volt+ Extras Mod Keybind Picker Modal is open:
        if let Some((mod_id, is_use, step)) = self.volt_key_picker {
            use crate::touch::PickerStep;
            if let Some((drag_id, last_x, mut moved)) = self.custom_kb_drag
                && drag_id == id
            {
                match phase {
                    TouchPhase::Moved => {
                        let dx_k = p[0] - last_x;
                        if dx_k.abs() > 3.5 {
                            moved = true;
                        }
                        let mdw = (w - 20.0).clamp(300.0, 620.0);
                        let max_kb_scroll = (880.0 - (mdw - 28.0)).max(0.0);
                        self.custom_kb_scroll_x =
                            (self.custom_kb_scroll_x - dx_k).clamp(0.0, max_kb_scroll);
                        self.custom_kb_drag = Some((drag_id, p[0], moved));
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Ended => {
                        self.custom_kb_drag = None;
                        if !moved && matches!(step, PickerStep::Keyboard { .. }) {
                            let mdw = (w - 20.0).clamp(300.0, 620.0);
                            let mdh = (h - 20.0).clamp(235.0, 340.0);
                            let mdx = (w - mdw) * 0.5;
                            let mdy = (h - mdh) * 0.5;
                            let kb_top = mdy + 40.0;
                            let kb_h = mdh - 56.0;
                            let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
                            let key_unit = 38.0;
                            let gap = 4.0;
                            for (r_idx, row) in KB_ROWS.iter().enumerate() {
                                let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                                let mut kx = mdx + 14.0 - self.custom_kb_scroll_x;
                                for (_lbl, tok, width_mult) in *row {
                                    let kw = key_unit * (*width_mult)
                                        + (*width_mult - 1.0).max(0.0) * gap;
                                    if inside([kx, ry, kw, row_h])
                                        && p[0] >= mdx + 10.0
                                        && p[0] <= mdx + mdw - 10.0
                                    {
                                        self.set_volt_mod_key(mod_id, is_use, (*tok).to_owned());
                                        self.volt_key_picker = None;
                                        return Some(InGameMenuAction::UpdateVoltExtras(
                                            self.volt_extras.clone(),
                                        ));
                                    }
                                    kx += kw + gap;
                                }
                            }
                        }
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Cancelled => {
                        self.custom_kb_drag = None;
                        return Some(InGameMenuAction::Close);
                    }
                    _ => {}
                }
            }
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            match step {
                PickerStep::AskInputType { event_slot } => {
                    let mdw = (w - 36.0).clamp(270.0, 440.0);
                    let mdh = 236.0;
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    let mcw = (mdw - 40.0) * 0.5;
                    if inside([mdx + 16.0, mdy + 54.0, mcw, 76.0]) {
                        self.volt_key_picker = Some((mod_id, is_use, PickerStep::Keyboard { event_slot }));
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 24.0 + mcw, mdy + 54.0, mcw, 76.0]) {
                        self.volt_key_picker = Some((mod_id, is_use, PickerStep::MouseAndActions { event_slot }));
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 16.0, mdy + 138.0, mdw - 32.0, 34.0]) {
                        return Some(InGameMenuAction::OpenKeyboard);
                    }
                    if inside([mdx + 16.0, mdy + 180.0, mcw, 36.0]) {
                        self.set_volt_mod_key(mod_id, is_use, String::new());
                        self.volt_key_picker = None;
                        return Some(InGameMenuAction::UpdateVoltExtras(
                            self.volt_extras.clone(),
                        ));
                    }
                    if inside([mdx + 24.0 + mcw, mdy + 180.0, mcw, 36.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.volt_key_picker = None;
                        return Some(InGameMenuAction::Close);
                    }
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::Keyboard { event_slot } => {
                    let mdw = (w - 20.0).clamp(300.0, 620.0);
                    let mdh = (h - 20.0).clamp(235.0, 340.0);
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    let max_kb_scroll = (880.0 - (mdw - 28.0)).max(0.0);
                    if inside([mdx + mdw - 266.0, mdy + 8.0, 64.0, 26.0]) {
                        return Some(InGameMenuAction::OpenKeyboard);
                    }
                    if inside([mdx + mdw - 196.0, mdy + 8.0, 56.0, 26.0]) {
                        self.custom_kb_scroll_x =
                            (self.custom_kb_scroll_x - 140.0).clamp(0.0, max_kb_scroll);
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + mdw - 134.0, mdy + 8.0, 56.0, 26.0]) {
                        self.custom_kb_scroll_x =
                            (self.custom_kb_scroll_x + 140.0).clamp(0.0, max_kb_scroll);
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + mdw - 72.0, mdy + 8.0, 58.0, 26.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.volt_key_picker = Some((mod_id, is_use, PickerStep::AskInputType { event_slot }));
                        return Some(InGameMenuAction::Close);
                    }
                    self.custom_kb_drag = Some((id, p[0], false));
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::MouseAndActions { event_slot } => {
                    let mdw = (w - 24.0).clamp(290.0, 560.0);
                    let mdh = (h - 20.0).clamp(235.0, 355.0);
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    if inside([mdx + mdw - 72.0, mdy + 8.0, 58.0, 26.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.volt_key_picker = Some((mod_id, is_use, PickerStep::AskInputType { event_slot }));
                        return Some(InGameMenuAction::Close);
                    }
                    let cols = 3usize;
                    let rows = MOUSE_AND_ACTION_ITEMS.len().div_ceil(cols).max(1);
                    let gx = mdx + 12.0;
                    let gy = mdy + 40.0;
                    let gw = mdw - 24.0;
                    let gh = mdh - 50.0;
                    let mcw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
                    let mch = ((gh - (rows as f32 - 1.0) * 4.0) / rows as f32).clamp(21.0, 32.0);
                    for (i, (_lbl, tok)) in MOUSE_AND_ACTION_ITEMS.iter().enumerate() {
                        let col = i % cols;
                        let row = i / cols;
                        let icx = gx + col as f32 * (mcw + 6.0);
                        let icy = gy + row as f32 * (mch + 4.0);
                        if inside([icx, icy, mcw, mch]) {
                            self.set_volt_mod_key(mod_id, is_use, (*tok).to_owned());
                            self.volt_key_picker = None;
                            return Some(InGameMenuAction::UpdateVoltExtras(
                                self.volt_extras.clone(),
                            ));
                        }
                    }
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::None => {}
            }
        }

        // 0b. If Waypoint Manager Modal is open:
        if self.volt_waypoint_manager {
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            let mdw = (w - 24.0).clamp(310.0, 540.0);
            let mdh = (h - 24.0).clamp(235.0, 350.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            let close_r = [mdx + mdw - 72.0, mdy + 10.0, 58.0, 26.0];
            if inside(close_r) || !inside([mdx, mdy, mdw, mdh]) {
                self.volt_waypoint_manager = false;
                return Some(InGameMenuAction::Close);
            }
            let top_y = mdy + 42.0;
            let btn_w = (mdw - 40.0) / 3.0;
            if inside([mdx + 14.0, top_y, btn_w, 26.0]) {
                return Some(InGameMenuAction::VoltQuickAction(
                    crate::extras::VoltQuickAction::AddWaypointHere,
                ));
            }
            if inside([mdx + 20.0 + btn_w, top_y, btn_w, 26.0]) {
                return Some(InGameMenuAction::VoltQuickAction(
                    crate::extras::VoltQuickAction::ClearDeathWaypoints,
                ));
            }
            if inside([mdx + 26.0 + btn_w * 2.0, top_y, btn_w, 26.0]) {
                self.volt_waypoints_page = 0;
                return Some(InGameMenuAction::VoltQuickAction(
                    crate::extras::VoltQuickAction::ClearAllWaypoints,
                ));
            }
            let per_page = 4usize;
            let total = self.volt_waypoints_snapshot.len();
            let pages = total.div_ceil(per_page).max(1);
            let page = self.volt_waypoints_page.min(pages - 1);
            let list_y = top_y + 34.0;
            let start = page * per_page;
            let end = (start + per_page).min(total);
            for (slot_i, wp_idx) in (start..end).enumerate() {
                let ry = list_y + slot_i as f32 * 46.0;
                let del_r = [mdx + mdw - 68.0, ry + 7.0, 54.0, 26.0];
                let share_r = [del_r[0] - 54.0, ry + 7.0, 48.0, 26.0];
                let col_r = [share_r[0] - 32.0, ry + 7.0, 26.0, 26.0];
                if inside(del_r) {
                    return Some(InGameMenuAction::VoltQuickAction(
                        crate::extras::VoltQuickAction::DeleteWaypoint(wp_idx),
                    ));
                }
                if inside(share_r) {
                    return Some(InGameMenuAction::VoltQuickAction(
                        crate::extras::VoltQuickAction::ShareWaypoint(wp_idx),
                    ));
                }
                if inside(col_r) {
                    return Some(InGameMenuAction::VoltQuickAction(
                        crate::extras::VoltQuickAction::CycleWaypointColor(wp_idx),
                    ));
                }
            }
            let pager_y = mdy + mdh - 34.0;
            if inside([mdx + 14.0, pager_y, 74.0, 24.0]) && page > 0 {
                self.volt_waypoints_page = page - 1;
            } else if inside([mdx + mdw - 88.0, pager_y, 74.0, 24.0]) && page + 1 < pages {
                self.volt_waypoints_page = page + 1;
            }
            return Some(InGameMenuAction::Close);
        }

        // 0c. If Block Outline Color Window is open:
        if self.volt_block_outline_color_modal {
            let mdw = (w - 20.0).clamp(310.0, 560.0);
            let mdh = (h - 20.0).clamp(250.0, 360.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            let lx = mdx + 14.0;
            let lw = (mdw * 0.34).clamp(112.0, 174.0);
            let ly = mdy + 44.0;
            let rx = lx + lw + 12.0;
            let rw = (mdx + mdw - 14.0 - rx).max(150.0);
            let ry = mdy + 44.0;
            let sl_y0 = ry + 68.0;

            let apply_slider = |this: &mut Self, ch: u8, px: f32| {
                let t = ((px - rx) / rw).clamp(0.0, 1.0);
                let bo = &mut this.volt_extras.blocks_outline;
                if ch < 3 {
                    let cur_rgb = if this.volt_block_outline_edit_fill {
                        bo.fill_color_rgb
                    } else {
                        bo.color_rgb
                    };
                    let [mut r, mut g, mut b] = crate::extras::BlocksOutlineConfig::rgb_bytes(cur_rgb);
                    let v = (t * 255.0).round().clamp(0.0, 255.0) as u8;
                    match ch {
                        0 => r = v,
                        1 => g = v,
                        _ => b = v,
                    }
                    let new_rgb = crate::extras::BlocksOutlineConfig::from_rgb(r, g, b);
                    if this.volt_block_outline_edit_fill {
                        bo.fill_match_outline = false;
                        bo.fill_color_rgb = new_rgb;
                    } else {
                        bo.rainbow = false;
                        bo.color_rgb = new_rgb;
                    }
                } else if this.volt_block_outline_edit_fill {
                    bo.fill_alpha_pct = (5.0 + t * 75.0).round().clamp(5.0, 80.0) as u8;
                } else {
                    bo.alpha_pct = (10.0 + t * 90.0).round().clamp(10.0, 100.0) as u8;
                }
            };

            if let Some((drag_id, ch)) = self.volt_block_outline_slider_drag
                && drag_id == id
            {
                match phase {
                    TouchPhase::Moved => {
                        apply_slider(self, ch, p[0]);
                        return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        apply_slider(self, ch, p[0]);
                        self.volt_block_outline_slider_drag = None;
                        return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
                    }
                    TouchPhase::Started => {}
                }
            }
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            let done_r = [mdx + mdw - 72.0, mdy + 10.0, 58.0, 26.0];
            let reset_r = [mdx + mdw - 134.0, mdy + 10.0, 56.0, 26.0];
            if inside(done_r) || !inside([mdx, mdy, mdw, mdh]) {
                self.volt_block_outline_color_modal = false;
                self.volt_block_outline_slider_drag = None;
                return Some(InGameMenuAction::Close);
            }
            if inside(reset_r) {
                let was_enabled = self.volt_extras.blocks_outline.enabled;
                self.volt_extras.blocks_outline = crate::extras::BlocksOutlineConfig::default();
                self.volt_extras.blocks_outline.enabled = was_enabled;
                return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
            }
            let tab_w = (lw - 6.0) * 0.5;
            if inside([lx, ly + 116.0, tab_w, 26.0]) {
                self.volt_block_outline_edit_fill = false;
                return Some(InGameMenuAction::Close);
            }
            if inside([lx + tab_w + 6.0, ly + 116.0, tab_w, 26.0]) {
                self.volt_block_outline_edit_fill = true;
                return Some(InGameMenuAction::Close);
            }
            if inside([lx, ly + 148.0, lw, 26.0]) {
                if self.volt_block_outline_edit_fill {
                    self.volt_extras.blocks_outline.fill_match_outline =
                        !self.volt_extras.blocks_outline.fill_match_outline;
                } else {
                    self.volt_extras.blocks_outline.rainbow =
                        !self.volt_extras.blocks_outline.rainbow;
                }
                return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
            }
            if inside([lx, ly + 178.0, lw, 26.0]) {
                self.volt_extras.blocks_outline.fill_overlay =
                    !self.volt_extras.blocks_outline.fill_overlay;
                return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
            }
            let sw_gap = 5.0;
            let sw_w = (rw - sw_gap * 5.0) / 6.0;
            let sw_h = 26.0;
            for (i, &(rgb, _name)) in BLOCK_OUTLINE_SWATCHES.iter().enumerate() {
                let col = (i % 6) as f32;
                let row = (i / 6) as f32;
                let sx = rx + col * (sw_w + sw_gap);
                let sy = ry + row * (sw_h + 5.0);
                if inside([sx, sy, sw_w, sw_h]) {
                    if self.volt_block_outline_edit_fill {
                        self.volt_extras.blocks_outline.fill_match_outline = false;
                        self.volt_extras.blocks_outline.fill_color_rgb = rgb;
                    } else {
                        self.volt_extras.blocks_outline.rainbow = false;
                        self.volt_extras.blocks_outline.color_rgb = rgb;
                    }
                    return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
                }
            }
            for ch in 0..4u8 {
                let sy = sl_y0 + f32::from(ch) * 34.0;
                if inside([rx - 4.0, sy + 4.0, rw + 8.0, 28.0]) {
                    self.volt_block_outline_slider_drag = Some((id, ch));
                    apply_slider(self, ch, p[0]);
                    return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
                }
            }
            return Some(InGameMenuAction::Close);
        }

        // 0. If "Send Custom Input" Picker Modal is open:
        if self.custom_input_picker != crate::touch::PickerStep::None {
            use crate::touch::PickerStep;
            if let Some((drag_id, last_x, mut moved)) = self.custom_kb_drag {
                if drag_id == id {
                    match phase {
                        TouchPhase::Moved => {
                            let dx_k = p[0] - last_x;
                            if dx_k.abs() > 3.5 {
                                moved = true;
                            }
                            let mdw = (w - 20.0).clamp(300.0, 620.0);
                            let max_kb_scroll = (880.0 - (mdw - 28.0)).max(0.0);
                            self.custom_kb_scroll_x =
                                (self.custom_kb_scroll_x - dx_k).clamp(0.0, max_kb_scroll);
                            self.custom_kb_drag = Some((drag_id, p[0], moved));
                            return Some(InGameMenuAction::Close);
                        }
                        TouchPhase::Ended => {
                            self.custom_kb_drag = None;
                            if !moved
                                && matches!(self.custom_input_picker, PickerStep::Keyboard { .. })
                            {
                                let mdw = (w - 20.0).clamp(300.0, 620.0);
                                let mdh = (h - 20.0).clamp(235.0, 340.0);
                                let mdx = (w - mdw) * 0.5;
                                let mdy = (h - mdh) * 0.5;
                                let kb_top = mdy + 40.0;
                                let kb_h = mdh - 56.0;
                                let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
                                let key_unit = 38.0;
                                let gap = 4.0;
                                for (r_idx, row) in KB_ROWS.iter().enumerate() {
                                    let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                                    let mut kx = mdx + 14.0 - self.custom_kb_scroll_x;
                                    for (_lbl, tok, width_mult) in *row {
                                        let kw = key_unit * (*width_mult)
                                            + (*width_mult - 1.0).max(0.0) * gap;
                                        if inside([kx, ry, kw, row_h])
                                            && p[0] >= mdx + 10.0
                                            && p[0] <= mdx + mdw - 10.0
                                        {
                                            self.custom_input_picker = PickerStep::None;
                                            self.floating_menu = false;
                                            return Some(InGameMenuAction::SendCustomInput(
                                                (*tok).to_owned(),
                                            ));
                                        }
                                        kx += kw + gap;
                                    }
                                }
                            }
                            return Some(InGameMenuAction::Close);
                        }
                        TouchPhase::Cancelled => {
                            self.custom_kb_drag = None;
                            return Some(InGameMenuAction::Close);
                        }
                        _ => {}
                    }
                }
            }
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            match self.custom_input_picker {
                PickerStep::AskInputType { event_slot } => {
                    let mdw = (w - 36.0).clamp(270.0, 440.0);
                    let mdh = 236.0;
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    let mcw = (mdw - 40.0) * 0.5;
                    if inside([mdx + 16.0, mdy + 54.0, mcw, 76.0]) {
                        self.custom_input_picker = PickerStep::Keyboard { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 24.0 + mcw, mdy + 54.0, mcw, 76.0]) {
                        self.custom_input_picker = PickerStep::MouseAndActions { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + 16.0, mdy + 138.0, mdw - 32.0, 34.0]) {
                        return Some(InGameMenuAction::OpenKeyboard);
                    }
                    if inside([mdx + 16.0, mdy + 180.0, mdw - 32.0, 36.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.custom_input_picker = PickerStep::None;
                        return Some(InGameMenuAction::Close);
                    }
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::Keyboard { event_slot } => {
                    let mdw = (w - 20.0).clamp(300.0, 620.0);
                    let mdh = (h - 20.0).clamp(235.0, 340.0);
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    let max_kb_scroll = (880.0 - (mdw - 28.0)).max(0.0);
                    if inside([mdx + mdw - 266.0, mdy + 8.0, 64.0, 26.0]) {
                        return Some(InGameMenuAction::OpenKeyboard);
                    }
                    if inside([mdx + mdw - 196.0, mdy + 8.0, 56.0, 26.0]) {
                        self.custom_kb_scroll_x =
                            (self.custom_kb_scroll_x - 140.0).clamp(0.0, max_kb_scroll);
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + mdw - 134.0, mdy + 8.0, 56.0, 26.0]) {
                        self.custom_kb_scroll_x =
                            (self.custom_kb_scroll_x + 140.0).clamp(0.0, max_kb_scroll);
                        return Some(InGameMenuAction::Close);
                    }
                    if inside([mdx + mdw - 72.0, mdy + 8.0, 58.0, 26.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.custom_input_picker = PickerStep::AskInputType { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    self.custom_kb_drag = Some((id, p[0], false));
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::MouseAndActions { event_slot } => {
                    let mdw = (w - 24.0).clamp(290.0, 560.0);
                    let mdh = (h - 20.0).clamp(235.0, 355.0);
                    let mdx = (w - mdw) * 0.5;
                    let mdy = (h - mdh) * 0.5;
                    if inside([mdx + mdw - 72.0, mdy + 8.0, 58.0, 26.0])
                        || !inside([mdx, mdy, mdw, mdh])
                    {
                        self.custom_input_picker = PickerStep::AskInputType { event_slot };
                        return Some(InGameMenuAction::Close);
                    }
                    let cols = 3usize;
                    let rows = MOUSE_AND_ACTION_ITEMS.len().div_ceil(cols).max(1);
                    let gx = mdx + 12.0;
                    let gy = mdy + 40.0;
                    let gw = mdw - 24.0;
                    let gh = mdh - 50.0;
                    let mcw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
                    let mch = ((gh - (rows as f32 - 1.0) * 4.0) / rows as f32).clamp(21.0, 32.0);
                    for (i, (_lbl, tok)) in MOUSE_AND_ACTION_ITEMS.iter().enumerate() {
                        let col = i % cols;
                        let row = i / cols;
                        let icx = gx + col as f32 * (mcw + 6.0);
                        let icy = gy + row as f32 * (mch + 4.0);
                        if inside([icx, icy, mcw, mch]) {
                            self.custom_input_picker = PickerStep::None;
                            self.floating_menu = false;
                            return Some(InGameMenuAction::SendCustomInput((*tok).to_owned()));
                        }
                    }
                    return Some(InGameMenuAction::Close);
                }
                PickerStep::None => {}
            }
        }

        // Handle content pane vertical scroll drag
        if let Some((scroll_id, start_y, start_scroll, mut scrolled)) = self.hub_scroll_drag {
            if scroll_id == id {
                match phase {
                    TouchPhase::Moved => {
                        let dy_touch = p[1] - start_y;
                        if dy_touch.abs() > 6.0 {
                            scrolled = true;
                        }
                        if scrolled {
                            let max_s = self.hub_max_scroll.get().max(0.0);
                            self.hub_scroll = (start_scroll - dy_touch).clamp(0.0, max_s);
                        }
                        self.hub_scroll_drag = Some((scroll_id, start_y, start_scroll, scrolled));
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Cancelled => {
                        self.hub_scroll_drag = None;
                        return Some(InGameMenuAction::Close);
                    }
                    TouchPhase::Ended => {
                        self.hub_scroll_drag = None;
                        if scrolled {
                            return Some(InGameMenuAction::Close);
                        }
                        // Fall through on tap release if not scrolled so the tapped row activates
                    }
                    _ => {}
                }
            }
        }

        // 1. If Copy / Edit Preset Dialog is open:
        if let Some(mut dlg) = self.preset_dialog.take() {
            if phase != TouchPhase::Started {
                self.preset_dialog = Some(dlg);
                return Some(InGameMenuAction::Close);
            }
            if dlg.confirm_replace_open {
                let cdw = (w - 36.0).clamp(290.0, 430.0);
                let cdh = 182.0;
                let cdx = (w - cdw) * 0.5;
                let cdy = (h - cdh) * 0.5;
                let bw = (cdw - 44.0) * 0.5;
                let by = cdy + cdh - 50.0;
                let save_r = [cdx + 18.0, by, bw, 36.0];
                let cancel_r = [cdx + 26.0 + bw, by, bw, 36.0];
                if inside(save_r) {
                    let name = dlg.name_input.trim().to_owned();
                    let description = if dlg.desc_input.trim().is_empty() {
                        format!("Custom {} layout", settings.control_style.label())
                    } else {
                        dlg.desc_input.trim().to_owned()
                    };
                    if dlg.is_copy {
                        return Some(InGameMenuAction::CopyPreset {
                            source_id: dlg.target_id,
                            name,
                            description,
                        });
                    } else {
                        return Some(InGameMenuAction::UpdatePresetMeta {
                            id: dlg.target_id,
                            name,
                            description,
                        });
                    }
                }
                if inside(cancel_r) || !inside([cdx, cdy, cdw, cdh]) {
                    dlg.confirm_replace_open = false;
                }
                self.preset_dialog = Some(dlg);
                return Some(InGameMenuAction::Close);
            }
            let mdw = (w - 28.0).clamp(310.0, 480.0);
            let mdh = (h - 16.0).clamp(242.0, 330.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            if !inside([mdx, mdy, mdw, mdh]) || inside([mdx + mdw - 76.0, mdy + 10.0, 64.0, 26.0]) {
                return Some(InGameMenuAction::Close);
            }
            let name_r = [mdx + 14.0, mdy + 42.0, (mdw - 36.0) * 0.45, 48.0];
            let desc_r = [
                mdx + 22.0 + (mdw - 36.0) * 0.45,
                mdy + 42.0,
                (mdw - 36.0) * 0.55,
                48.0,
            ];
            if inside(name_r) {
                dlg.editing_desc = false;
                self.preset_dialog = Some(dlg);
                return Some(InGameMenuAction::EditPresetField(false));
            }
            if inside(desc_r) {
                dlg.editing_desc = true;
                self.preset_dialog = Some(dlg);
                return Some(InGameMenuAction::EditPresetField(true));
            }
            let save_r = [mdx + 14.0, mdy + mdh - 38.0, mdw - 28.0, 30.0];
            if inside(save_r) {
                let name = if dlg.name_input.trim().is_empty() {
                    "Custom Preset".to_owned()
                } else {
                    dlg.name_input.trim().to_owned()
                };
                let description = if dlg.desc_input.trim().is_empty() {
                    format!("Custom {} layout", settings.control_style.label())
                } else {
                    dlg.desc_input.trim().to_owned()
                };
                let existing = crate::touch::Settings::find_preset_by_name(
                    std::path::Path::new("."),
                    settings.control_style,
                    &name,
                );
                let has_conflict = match &existing {
                    Some(ex) => dlg.is_copy || !ex.id.eq_ignore_ascii_case(&dlg.target_id),
                    None => false,
                };
                if has_conflict {
                    dlg.name_input = name;
                    dlg.desc_input = description;
                    dlg.confirm_replace_open = true;
                    self.preset_dialog = Some(dlg);
                    return Some(InGameMenuAction::Close);
                }
                if dlg.is_copy {
                    return Some(InGameMenuAction::CopyPreset {
                        source_id: dlg.target_id,
                        name,
                        description,
                    });
                } else {
                    return Some(InGameMenuAction::UpdatePresetMeta {
                        id: dlg.target_id,
                        name,
                        description,
                    });
                }
            }
            self.preset_dialog = Some(dlg);
            return Some(InGameMenuAction::Close);
        }

        // 2. If Preset Management Modal is open:
        if self.presets_modal_open {
            if phase != TouchPhase::Started {
                return Some(InGameMenuAction::Close);
            }
            self.ensure_presets_cache(settings.control_style);
            let mdw = (w - 24.0).clamp(320.0, 520.0);
            let mdh = (h - 18.0).clamp(248.0, 360.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            if !inside([mdx, mdy, mdw, mdh]) || inside([mdx + mdw - 78.0, mdy + 10.0, 64.0, 28.0]) {
                self.presets_modal_open = false;
                return Some(InGameMenuAction::Close);
            }
            let list_y = mdy + 48.0;
            let row_h = 46.0;
            let max_rows = ((mdh - 60.0) / (row_h + 6.0)).floor().max(1.0) as usize;
            let presets = self.cached_presets.clone();
            for (idx, preset) in presets.iter().take(max_rows).enumerate() {
                let ry = list_y + idx as f32 * (row_h + 6.0);
                let row_r = [mdx + 14.0, ry, mdw - 28.0, row_h];
                let copy_r = [row_r[0] + row_r[2] - 68.0, ry + 8.0, 60.0, 30.0];
                let edit_r = [row_r[0] + row_r[2] - 104.0, ry + 8.0, 30.0, 30.0];
                if inside(copy_r) {
                    self.preset_dialog = Some(PresetDialogState {
                        is_copy: true,
                        target_id: preset.id.clone(),
                        name_input: format!("{} Copy", preset.name),
                        desc_input: preset.description.clone(),
                        editing_desc: false,
                        confirm_replace_open: false,
                    });
                    return Some(InGameMenuAction::Close);
                }
                if !preset.is_default && inside(edit_r) {
                    self.preset_dialog = Some(PresetDialogState {
                        is_copy: false,
                        target_id: preset.id.clone(),
                        name_input: preset.name.clone(),
                        desc_input: preset.description.clone(),
                        editing_desc: false,
                        confirm_replace_open: false,
                    });
                    return Some(InGameMenuAction::Close);
                }
                if inside(row_r) {
                    return Some(InGameMenuAction::SelectPreset(preset.id.clone()));
                }
            }
            return Some(InGameMenuAction::Close);
        }

        // 3. Main Horizon Control Hub Dialog (Option 2 Volt Graphics Sidebar + Panel)
        let sb_rects = hub_sidebar_rects(dx, dy, dw, dh, side_w);
        if phase == TouchPhase::Started {
            if !inside([dx, dy, dw, dh])
                || inside(sb_rects.done_btn)
                || inside(sb_rects.close_x)
            {
                self.floating_menu = false;
                self.resolution_scale_dragging = false;
                self.sensitivity_dragging = false;
                self.pointer_size_dragging = false;
                self.mouse_speed_dragging = false;
                self.gyro_sensitivity_dragging = false;
                self.button_scale_dragging = false;
                self.overlay_scale_dragging = false;
                self.overlay_opacity_dragging = false;
                self.music_volume_dragging = false;
                self.custom_input_picker = crate::touch::PickerStep::None;
                self.custom_kb_drag = None;
                self.hub_scroll_drag = None;
                return Some(InGameMenuAction::Close);
            }

            if inside(sb_rects.quick_studio) {
                self.floating_menu = false;
                return Some(InGameMenuAction::OpenControlEditor);
            }

            if inside(sb_rects.booster_pill) && sb_rects.booster_pill[0] > dx + side_w + 130.0 {
                self.booster_mode = true;
                return Some(InGameMenuAction::Close);
            }

            if inside(sb_rects.mode_badge) && sb_rects.mode_badge[0] > dx + side_w + 130.0 {
                let next_style = if settings.control_style == ControlStyle::Mcpe {
                    ControlStyle::GoyControl
                } else {
                    ControlStyle::Mcpe
                };
                return Some(InGameMenuAction::SetControlStyle(next_style));
            }

            for (ti, tab_r) in sb_rects.tabs.iter().enumerate() {
                if inside(*tab_r) {
                    let next_tab = ti as u8;
                    if self.hub_tab != next_tab {
                        self.hub_tab = next_tab;
                        self.hub_scroll = 0.0;
                        self.tab_anim.set(0.0);
                        self.last_anim_tick.set(None);
                        if next_tab == 1 {
                            self.refresh_presets_cache(settings.control_style);
                        }
                        if next_tab == 9 {
                            self.volt_extras_config_mod = None;
                        }
                    }
                    return Some(InGameMenuAction::Close);
                }
            }

            // Check if touch started on sliders or quick chips in Tab 0 (Recommended)
            if self.hub_tab == 0 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let res_card_y = body_y + 18.0;
                let chip_y = res_card_y + 48.0;
                let chip_gap = 6.0;
                let chip_w = (cw - 28.0 - chip_gap * 4.0) / 5.0;
                for (ci, pct) in [50u32, 65, 75, 85, 100].into_iter().enumerate() {
                    let rx = cx + 14.0 + ci as f32 * (chip_w + chip_gap);
                    if inside([rx, chip_y, chip_w, 26.0]) {
                        return Some(InGameMenuAction::SetResolutionScale(pct));
                    }
                }
                if inside([cx, res_card_y, cw, 47.0]) {
                    self.resolution_scale_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let pct = (((25.0 + t * 75.0) / 5.0).round() as u32 * 5).clamp(25, 100);
                    return Some(InGameMenuAction::SetResolutionScale(pct));
                }
                let sens_card_y = body_y + 118.0;
                if inside([cx, sens_card_y, cw, 50.0]) {
                    self.sensitivity_dragging = true;
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
                let spd_card_y = body_y + 230.0;
                if inside([cx, spd_card_y, cw, 50.0]) {
                    self.mouse_speed_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
            }

            // Check if touch started on the Sensitivity slider card in Tab 1 (Controls)
            if self.hub_tab == 1 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let sens_card_y = body_y + 148.0;
                let track_r = [cx, sens_card_y, cw, 50.0];
                if inside(track_r) {
                    self.sensitivity_dragging = true;
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
            }

            // Check if touch started on the Virtual Mouse Speed or Pointer Size slider tracks in Tab 2 (Aiming)
            if self.hub_tab == 2 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let spd_card_y = body_y + 180.0;
                if inside([cx, spd_card_y, cw, 50.0]) {
                    self.mouse_speed_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                let size_card_y = body_y + 238.0;
                if inside([cx, size_card_y, cw, 50.0]) {
                    self.pointer_size_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMousePointerSize(0.5 + t * 1.5));
                }
                let gyro_card_y = body_y + 350.0;
                if inside([cx, gyro_card_y, cw, 50.0]) {
                    self.gyro_sensitivity_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetGyroSensitivity(0.10 + t * 2.90));
                }
            }

            // Check if touch started on the Global Button Scale slider track in Tab 3 (Buttons)
            if self.hub_tab == 3 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let scale_card_y = body_y + 18.0;
                if inside([cx, scale_card_y, cw, 50.0]) {
                    self.button_scale_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetButtonScale(0.50 + t * 1.50));
                }
            }

            // Check if touch started on the Overlay Size or Opacity slider tracks in Tab 7 (Overlay)
            if self.hub_tab == 7 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                if inside([cx, body_y + 18.0, cw, 50.0]) {
                    self.overlay_scale_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetOverlayScale(0.50 + t * 1.50));
                }
                if inside([cx, body_y + 76.0, cw, 50.0]) {
                    self.overlay_opacity_dragging = true;
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetOverlayOpacity(0.20 + t * 0.80));
                }
            }

            if self.hub_tab == 8 && inside(content_pane) {
                let body_y = body_top - self.hub_scroll;
                let vol_y = body_y + 132.0;
                if inside([cx + 78.0, vol_y, cw - 78.0, 48.0]) {
                    self.music_volume_dragging = true;
                    let sl_x = cx + 92.0;
                    let sl_w = (cw - 106.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::MusicSetVolume(t));
                }
            }

            // Start scroll-or-tap tracking inside the right content pane
            if inside(content_pane) && self.hub_max_scroll.get() > 2.0 {
                self.hub_scroll_drag = Some((id, p[1], self.hub_scroll, false));
                return Some(InGameMenuAction::Close);
            }
        }

        // Rows activate on release only: pressing and releasing both used to run the row,
        // so switches toggled twice per tap and snapped back as if they had to be held.
        let activate = phase == TouchPhase::Ended && self.hub_scroll_drag.is_none();
        if !activate || !inside(content_pane) {
            return Some(InGameMenuAction::Close);
        }

        let body_y = body_top - self.hub_scroll;
        match self.hub_tab {
            0 => {
                // Tab 0: Recommended (Real-Time Resolution, Fast Sensitivity, Virtual Mouse, Quick Controls & Animations)
                let res_y = body_y + 18.0;
                let chip_y = res_y + 48.0;
                let chip_gap = 6.0;
                let chip_w = (cw - 28.0 - chip_gap * 4.0) / 5.0;
                for (ci, pct) in [50u32, 65, 75, 85, 100].into_iter().enumerate() {
                    let rx = cx + 14.0 + ci as f32 * (chip_w + chip_gap);
                    if inside([rx, chip_y, chip_w, 24.0]) {
                        return Some(InGameMenuAction::SetResolutionScale(pct));
                    }
                }
                if inside([cx, res_y, cw, 46.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let pct = (25.0 + t * 75.0).round() as u32;
                    return Some(InGameMenuAction::SetResolutionScale(pct.clamp(25, 100)));
                }
                let sens_y = body_y + 118.0;
                if inside([cx, sens_y, cw, 50.0]) {
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }
                let vmouse_y = body_y + 176.0;
                if inside([cx, vmouse_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleVirtualMouse);
                }
                let spd_y = body_y + 230.0;
                if inside([cx, spd_y, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                let act_y = body_y + 288.0;
                let half_w = (cw - 10.0) * 0.5;
                if inside([cx, act_y, half_w, 44.0]) {
                    let next_style = if settings.control_style == ControlStyle::Mcpe {
                        ControlStyle::GoyControl
                    } else {
                        ControlStyle::Mcpe
                    };
                    return Some(InGameMenuAction::SetControlStyle(next_style));
                }
                if inside([cx + half_w + 10.0, act_y, half_w, 44.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenControlEditor);
                }
                let send_y = body_y + 340.0;
                if inside([cx, send_y, cw, 44.0]) {
                    self.custom_input_picker =
                        crate::touch::PickerStep::AskInputType { event_slot: 0 };
                    return Some(InGameMenuAction::Close);
                }
                let anim_y = body_y + 392.0;
                if inside([cx, anim_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleGameAnimations);
                }
            }
            1 => {
                // Tab 1: Controls
                let mode_y = body_y + 18.0;
                let half_w = (cw - 10.0) * 0.5;
                if inside([cx, mode_y, half_w, 46.0]) {
                    return Some(InGameMenuAction::SetControlStyle(ControlStyle::Mcpe));
                }
                if inside([cx + half_w + 10.0, mode_y, half_w, 46.0]) {
                    return Some(InGameMenuAction::SetControlStyle(ControlStyle::GoyControl));
                }

                // Customize Controls | Control Presets
                let act_y = body_y + 84.0;
                if inside([cx, act_y, half_w, 44.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenControlEditor);
                }
                if inside([cx + half_w + 10.0, act_y, half_w, 44.0]) {
                    self.refresh_presets_cache(settings.control_style);
                    self.presets_modal_open = true;
                    return Some(InGameMenuAction::Close);
                }

                // Sensitivity Slider card (y = body_y + 148.0, h = 50.0)
                let sens_y = body_y + 148.0;
                if inside([cx, sens_y, cw, 50.0]) {
                    let sl_x = cx + 10.0;
                    let sl_w = (cw - 20.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    let sens = (0.10 + t * 2.90).clamp(0.10, 3.00);
                    return Some(InGameMenuAction::SetTouchSensitivity(sens));
                }

                // Authoritative Virtual Mouse Toggle (y = body_y + 206.0, h = 46.0)
                let vmouse_y = body_y + 206.0;
                if inside([cx, vmouse_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleVirtualMouse);
                }

                // Invert Y-Axis (y = body_y + 260.0, h = 46.0)
                let inv_y = body_y + 260.0;
                if inside([cx, inv_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleInvertY);
                }

                // Split Touch (Aim & Look) (y = body_y + 314.0, h = 46.0)
                let spl_y = body_y + 314.0;
                if inside([cx, spl_y, cw, 46.0]) {
                    return Some(InGameMenuAction::ToggleSplitTouch);
                }
            }
            2 => {
                // Tab 2: Aiming, Camera & Virtual Mouse
                let row_h = 46.0;
                let step = 54.0;
                let r0 = [cx, body_y + 18.0, cw, row_h]; // Movement & Aim Mode
                let r1 = [cx, body_y + 18.0 + step, cw, row_h]; // Joystick Active Zone
                let r2 = [cx, body_y + 18.0 + step * 2.0, cw, row_h]; // Virtual Mouse Toggle
                let spd_card = [cx, body_y + 180.0, cw, 50.0]; // Virtual Mouse Speed slider
                let size_card = [cx, body_y + 238.0, cw, 50.0]; // Pointer Size slider
                let r4 = [cx, body_y + 296.0, cw, row_h]; // Gyroscope Aiming
                let _r5 = [cx, body_y + 350.0, cw, row_h]; // Gyro Sensitivity
                if inside(r0) {
                    return Some(InGameMenuAction::CycleMovementMode);
                }
                if inside(r1) {
                    return Some(InGameMenuAction::CycleJoystickArea);
                }
                if inside(r2) {
                    return Some(InGameMenuAction::ToggleVirtualMouse);
                }
                if inside(spd_card) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMouseSpeed(0.25 + t * 2.75));
                }
                if inside(size_card) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetVirtualMousePointerSize(0.5 + t * 1.5));
                }
                if inside(r4) {
                    return Some(InGameMenuAction::ToggleGyro);
                }
                if inside([cx, body_y + 350.0, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetGyroSensitivity(0.10 + t * 2.90));
                }
            }
            3 => {
                // Tab 3: Buttons & Layout Options
                let row_h = 46.0;
                let step = 54.0;
                let scale_y = body_y + 18.0;
                if inside([cx, scale_y, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetButtonScale(0.50 + t * 1.50));
                }
                if inside([cx, body_y + 18.0 + step, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleLeftHanded);
                }
                if inside([cx, body_y + 18.0 + step * 2.0, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleSprintMode);
                }
                if inside([cx, body_y + 18.0 + step * 3.0, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleInventoryControls);
                }
            }
            4 => {
                // Tab 4: Advanced & Quick Actions
                let row_h = 46.0;
                let step = 54.0;
                if inside([cx, body_y + 18.0, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleVibrate);
                }
                if inside([cx, body_y + 18.0 + step, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleAutoJump);
                }
                if inside([cx, body_y + 18.0 + step * 2.0, cw, row_h]) {
                    return Some(InGameMenuAction::PresetVisibilityAll);
                }
                let ky = body_y + 198.0;
                let kw = (cw - 25.0) / 6.0;
                let quick = [
                    KeyCode::F1,
                    KeyCode::F3,
                    KeyCode::F5,
                    KeyCode::KeyE,
                    KeyCode::KeyT,
                ];
                for (i, key) in quick.iter().enumerate() {
                    let rx = cx + i as f32 * (kw + 5.0);
                    if inside([rx, ky, kw, 36.0]) {
                        self.floating_menu = false;
                        return Some(InGameMenuAction::TriggerKey(*key));
                    }
                }
                if inside([cx + 5.0 * (kw + 5.0), ky, kw, 36.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenKeyboard);
                }
                let send_y = body_y + 244.0;
                if inside([cx, send_y, cw, 40.0]) {
                    self.custom_input_picker =
                        crate::touch::PickerStep::AskInputType { event_slot: 0 };
                    return Some(InGameMenuAction::Close);
                }
                let sys_y = body_y + 292.0;
                let half = (cw - 10.0) * 0.5;
                if inside([cx, sys_y, half, 38.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::ForceKill);
                }
                if inside([cx + half + 10.0, sys_y, half, 38.0]) {
                    self.floating_menu = false;
                    return Some(InGameMenuAction::OpenLogs);
                }
            }
            5 => {
                // Tab 5: Record — resolution, frame rate, audio, controls and the list.
                let cells = record_cells(cx, cw, body_y);
                if inside(cells.start) {
                    return Some(if recorder::status().phase.startable() {
                        InGameMenuAction::RecorderStart
                    } else {
                        InGameMenuAction::RecorderStop
                    });
                }
                for (i, cell) in cells.resolution.into_iter().enumerate() {
                    if inside(cell) {
                        return Some(InGameMenuAction::SetRecorderResolution(i as u8));
                    }
                }
                for (i, cell) in cells.fps.into_iter().enumerate() {
                    if inside(cell) {
                        return Some(InGameMenuAction::SetRecorderFps(i as u8));
                    }
                }
                for (i, cell) in cells.audio.into_iter().enumerate() {
                    if inside(cell) {
                        return Some(InGameMenuAction::SetRecorderAudio(i as u8));
                    }
                }
                if inside(cells.overlay) {
                    return Some(InGameMenuAction::ToggleRecorderOverlay);
                }
                let recordings = recorder::status().recordings;
                for (i, cell) in cells.recordings.into_iter().enumerate() {
                    if inside(cell)
                        && let Some(item) = recordings.get(i)
                    {
                        self.floating_menu = false;
                        return Some(InGameMenuAction::RecorderPlay(item.uri.clone()));
                    }
                }
            }
            6 => {
                if let Some(act) = replay_hud::control_hub_replay_tab_touch(cx, cw, body_y, p) {
                    return Some(act);
                }
            }
            7 => {
                // Tab 7: Overlay Menu — size, opacity & visibility rules for the V+ button.
                if inside([cx, body_y + 18.0, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetOverlayScale(0.50 + t * 1.50));
                }
                if inside([cx, body_y + 76.0, cw, 50.0]) {
                    let sl_x = cx + 14.0;
                    let sl_w = (cw - 28.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::SetOverlayOpacity(0.20 + t * 0.80));
                }
                let row_h = 46.0;
                let step = 54.0;
                if inside([cx, body_y + 134.0, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleOverlayHidden);
                }
                if inside([cx, body_y + 134.0 + step, cw, row_h]) {
                    return Some(InGameMenuAction::ToggleOverlayShowControlsInMenus);
                }
            }
            8 => {
                // Tab 8: Internet Music Player, Fast-Forward, Volume, Playlists & Vanilla Toggle
                let seek_r = [cx + 12.0, body_y + 58.0, (cw - 24.0).max(40.0), 18.0];
                if inside(seek_r) {
                    let t = ((p[0] - seek_r[0]) / seek_r[2]).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::MusicSeekFrac(t));
                }
                let gap = 6.0;
                let bw = ((cw - 24.0 - gap * 3.0) / 4.0).max(40.0);
                let btn_y = body_y + 84.0;
                for (bi, act) in [
                    InGameMenuAction::MusicPrev,
                    InGameMenuAction::MusicTogglePause,
                    InGameMenuAction::MusicSeekBy(10_000),
                    InGameMenuAction::MusicNext,
                ]
                .into_iter()
                .enumerate()
                {
                    let bx = cx + 12.0 + bi as f32 * (bw + gap);
                    if inside([bx, btn_y, bw, 30.0]) {
                        return Some(act);
                    }
                }

                let vol_y = body_y + 132.0;
                if inside([cx, vol_y, 72.0, 48.0]) {
                    return Some(InGameMenuAction::MusicToggleMute);
                }
                if inside([cx + 78.0, vol_y, cw - 78.0, 48.0]) {
                    let sl_x = cx + 92.0;
                    let sl_w = (cw - 106.0).max(40.0);
                    let t = ((p[0] - sl_x) / sl_w).clamp(0.0, 1.0);
                    return Some(InGameMenuAction::MusicSetVolume(t));
                }
                if inside([cx, body_y + 188.0, cw, 46.0]) {
                    return Some(InGameMenuAction::MusicToggleVanilla);
                }

                // Mode Switcher: Playlists | Search Online
                let mode_y = body_y + 242.0;
                let half_w = ((cw - 8.0) * 0.5).max(60.0);
                if inside([cx, mode_y, half_w, 32.0]) {
                    self.music_search_tab = false;
                    self.music_editing_search = false;
                    return Some(InGameMenuAction::MusicSetSearchTab(false));
                }
                if inside([cx + half_w + 8.0, mode_y, half_w, 32.0]) {
                    self.music_search_tab = true;
                    return Some(InGameMenuAction::MusicSetSearchTab(true));
                }

                let sub_y = body_y + 284.0;
                if !self.music_search_tab {
                    let playlists = crate::internet_music::playlists();
                    let pl_start_y = sub_y + 18.0;
                    for (pi, _) in playlists.iter().enumerate() {
                        let ry = pl_start_y + pi as f32 * 42.0;
                        let play_r = [cx + cw - 78.0, ry + 5.0, 68.0, 28.0];
                        if inside(play_r) {
                            self.music_selected_playlist = pi;
                            return Some(InGameMenuAction::MusicPlayPlaylist(pi));
                        }
                        if inside([cx, ry, cw, 38.0]) {
                            self.music_selected_playlist = pi;
                            return Some(InGameMenuAction::MusicSelectPlaylist(pi));
                        }
                    }

                    let sel_idx = if playlists.is_empty() {
                        0
                    } else {
                        self.music_selected_playlist.min(playlists.len() - 1)
                    };
                    let tracks_header_y = pl_start_y + playlists.len().max(1) as f32 * 42.0 + 10.0;
                    let tracks_start_y = tracks_header_y + 20.0;
                    if let Some(pl) = playlists.get(sel_idx) {
                        for (ti, _) in pl.tracks.iter().enumerate() {
                            let ty = tracks_start_y + ti as f32 * 42.0;
                            if inside([cx, ty, cw, 38.0]) {
                                return Some(InGameMenuAction::MusicPlayPlaylistTrack(sel_idx, ti));
                            }
                        }
                    }
                } else {
                    // Search Online Sub-Tab
                    let search_btn_w = 72.0;
                    let field_w = (cw - search_btn_w - 8.0).max(80.0);
                    if inside([cx, sub_y, field_w, 34.0]) {
                        self.music_editing_search = true;
                        return Some(InGameMenuAction::MusicOpenSearchKeyboard);
                    }
                    if inside([cx + field_w + 8.0, sub_y, search_btn_w, 34.0]) {
                        self.music_editing_search = false;
                        return Some(InGameMenuAction::MusicTriggerSearch);
                    }

                    let chip_y = sub_y + 40.0;
                    let sort_w = 88.0;
                    let genres: [(&str, Option<&'static str>); 4] = [
                        ("Trending", None),
                        ("Lo-Fi", Some("Lo-Fi")),
                        ("Chill", Some("Chill")),
                        ("Synth", Some("Synthwave")),
                    ];
                    let chips_w = (cw - sort_w - 8.0).max(120.0);
                    let chip_gap = 4.0;
                    let chip_w = ((chips_w - chip_gap * 3.0) / 4.0).max(36.0);
                    for (gi, (_, gval)) in genres.iter().enumerate() {
                        let gx = cx + gi as f32 * (chip_w + chip_gap);
                        if inside([gx, chip_y, chip_w, 28.0]) {
                            return Some(InGameMenuAction::MusicSelectGenre(*gval));
                        }
                    }
                    if inside([cx + cw - sort_w, chip_y, sort_w, 28.0]) {
                        return Some(InGameMenuAction::MusicCycleSort);
                    }

                    let search_snap = crate::internet_music::in_game_search_snapshot();
                    let list_y = sub_y + 76.0;
                    for (idx, _) in search_snap.results.iter().enumerate() {
                        let ry = list_y + idx as f32 * 50.0;
                        let play_r = [cx + cw - 58.0, ry + 7.0, 52.0, 30.0];
                        let dl_r = [play_r[0] - 36.0, ry + 7.0, 30.0, 30.0];
                        if inside(dl_r) {
                            return Some(InGameMenuAction::MusicDownloadSearchResult(idx));
                        }
                        if inside(play_r) || inside([cx, ry, cw, 44.0]) {
                            return Some(InGameMenuAction::MusicPlaySearchResult(idx));
                        }
                    }

                    let pager_y = list_y + search_snap.results.len().max(1) as f32 * 50.0 + 6.0;
                    let btn_w = 84.0;
                    let page_w = 68.0;
                    let pager_total = btn_w * 2.0 + page_w + 12.0;
                    let px = cx + (cw - pager_total).max(0.0) * 0.5;
                    if search_snap.page > 0 && inside([px, pager_y, btn_w, 30.0]) {
                        return Some(InGameMenuAction::MusicPrevPage);
                    }
                    if search_snap.has_next
                        && inside([px + btn_w + page_w + 12.0, pager_y, btn_w, 30.0])
                    {
                        return Some(InGameMenuAction::MusicNextPage);
                    }
                }
            }
            9 => {
                let body_y = body_top - self.hub_scroll;
                if let Some(mod_idx) = self.volt_extras_config_mod {
                    self.volt_extras_search_focused = false;
                    let back_btn = [cx, body_y, (cw - 60.0).max(120.0), 32.0];
                    let master_sw = [cx + cw - 52.0, body_y + 5.0, 40.0, 22.0];
                    if inside(master_sw) {
                        let cur = crate::extras::volt_mod_enabled(&self.volt_extras, mod_idx);
                        crate::extras::set_volt_mod_enabled(&mut self.volt_extras, mod_idx, !cur);
                        return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
                    }
                    if inside(back_btn) {
                        self.volt_extras_config_mod = None;
                        self.hub_scroll = 0.0;
                        return Some(InGameMenuAction::Close);
                    }
                    let rows = crate::volt_extras_ui::mod_setting_rows(&self.volt_extras, mod_idx);
                    for r in 0..rows.len() {
                        let ry = body_y + 42.0 + r as f32 * 54.0;
                        if inside([cx, ry, cw, 46.0]) {
                            if let Some(act) = crate::volt_extras_ui::click_mod_setting_row(
                                &mut self.volt_extras,
                                mod_idx,
                                r,
                            ) {
                                match act {
                                    crate::extras::VoltQuickAction::OpenModKeyPicker {
                                        mod_id,
                                        is_use,
                                    } => {
                                        self.volt_key_picker = Some((
                                            mod_id,
                                            is_use,
                                            crate::touch::PickerStep::AskInputType { event_slot: 0 },
                                        ));
                                        return Some(InGameMenuAction::Close);
                                    }
                                    crate::extras::VoltQuickAction::OpenWaypointManager => {
                                        self.volt_waypoint_manager = true;
                                        self.volt_waypoints_page = 0;
                                        return Some(InGameMenuAction::Close);
                                    }
                                    crate::extras::VoltQuickAction::OpenBlockOutlineColor => {
                                        self.volt_block_outline_color_modal = true;
                                        self.volt_block_outline_edit_fill = false;
                                        self.volt_block_outline_slider_drag = None;
                                        return Some(InGameMenuAction::Close);
                                    }
                                    other => return Some(InGameMenuAction::VoltQuickAction(other)),
                                }
                            }
                            return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
                        }
                    }
                } else {
                    // Search bar + Enabled/Disabled filter & sort pills
                    let bar_y = body_y + 30.0;
                    let pill_gap = 3.0;
                    let pill_w = [34.0_f32, 52.0, 54.0, 42.0];
                    let right_total: f32 = pill_w.iter().sum::<f32>() + pill_gap * 3.0;
                    let search_w = (cw - right_total - 6.0).max(96.0);
                    let search_r = [cx, bar_y, search_w, 26.0];
                    if inside(search_r) {
                        if !self.volt_extras_search.is_empty()
                            && p[0] >= cx + search_w - 22.0
                        {
                            self.volt_extras_search.clear();
                            self.hub_scroll = 0.0;
                            self.volt_extras_search_focused = false;
                            return Some(InGameMenuAction::Close);
                        }
                        self.volt_extras_search_focused = true;
                        return Some(InGameMenuAction::FocusVoltExtrasSearch);
                    }
                    self.volt_extras_search_focused = false;
                    let mut px_cur = cx + search_w + 6.0;
                    for (fi, &pw_i) in pill_w.iter().enumerate() {
                        if inside([px_cur, bar_y, pw_i, 26.0]) {
                            if fi < 3 {
                                self.volt_extras_filter = fi as u8;
                            } else {
                                self.volt_extras_sort_enabled = !self.volt_extras_sort_enabled;
                            }
                            self.hub_scroll = 0.0;
                            return Some(InGameMenuAction::Close);
                        }
                        px_cur += pw_i + pill_gap;
                    }

                    // Category filter pills (6 tabs: All, PvP, HUD, Camera, World, Utility)
                    let cat_y = body_y + 62.0;
                    let cat_gap = 4.0;
                    let cat_w = ((cw - cat_gap * 5.0) / 6.0).max(36.0);
                    for ci in 0..6u8 {
                        let bx = cx + ci as f32 * (cat_w + cat_gap);
                        if inside([bx, cat_y, cat_w, 22.0]) {
                            self.volt_extras_cat = ci;
                            self.hub_scroll = 0.0;
                            return Some(InGameMenuAction::Close);
                        }
                    }
                    let filtered = crate::volt_extras_ui::filtered_mod_indices_with(
                        &self.volt_extras,
                        self.volt_extras_cat,
                        &self.volt_extras_search,
                        self.volt_extras_filter,
                        self.volt_extras_sort_enabled,
                    );
                    let list_y = body_y + 92.0;
                    for (pos_i, &mod_idx) in filtered.iter().enumerate() {
                        let ry = list_y + pos_i as f32 * 68.0;
                        let cfg_r = [cx + cw - 124.0, ry + 16.0, 64.0, 28.0];
                        let sw_r = [cx + cw - 54.0, ry + 11.0, 48.0, 40.0];
                        if inside(cfg_r) {
                            self.volt_extras_config_mod = Some(mod_idx);
                            self.hub_scroll = 0.0;
                            return Some(InGameMenuAction::Close);
                        }
                        if inside(sw_r) || inside([cx, ry, cw, 62.0]) {
                            let cur = crate::extras::volt_mod_enabled(&self.volt_extras, mod_idx);
                            crate::extras::set_volt_mod_enabled(
                                &mut self.volt_extras,
                                mod_idx,
                                !cur,
                            );
                            return Some(InGameMenuAction::UpdateVoltExtras(self.volt_extras.clone()));
                        }
                    }
                }
            }
            _ => {}
        }

        Some(InGameMenuAction::Close)
    }

    /// Draws the Floating Menu Button (MCPE stone-slate `H` button vs GoyControl `=` bubble) and
    /// the redesigned Horizon Control Hub with smooth motion, generous spacing, scrollable content pane,
    /// and authoritative Virtual Mouse & gameplay settings.
    pub fn draw_floating_menu(
        &self,
        b: &mut HudBatch,
        a: &HudAtlas,
        settings: &crate::touch::Settings,
    ) {
        use crate::touch::ControlStyle;
        use crate::ui_theme::{Face, Icon, Ui, color};
        if !self.floating_menu && settings.overlay_hidden {
            return;
        }
        let mut ui = Ui::mobile(a, b, (0.0, 0.0));
        let (w, h) = (ui.w, ui.h);
        let (open_t, tab_t) = self.step_animations();
        let open_ease = crate::ui_theme::motion::spring_expressive_default(open_t);
        let tab_ease = crate::ui_theme::motion::spring_expressive_default(tab_t);

        // Nexora Obsidian-Crimson Glass theme for the NX overlay, Booster & Control Hub
        let surface_bg = rgba(12, 4, 6, 246);
        let sidebar_bg = rgba(26, 6, 10, 248);
        let card_bg = rgba(22, 9, 12, 238);
        let card_sel = rgba(58, 12, 20, 246);
        let well_bg = rgba(16, 6, 8, 244);
        let border_c = rgba(220, 20, 60, 88);
        let border_strong = rgba(239, 68, 68, 240);
        let accent = rgba(220, 20, 60, 255);
        let accent_hi = rgba(248, 113, 113, 255);
        let accent_edge = rgba(254, 202, 202, 200);
        let accent_fg = rgba(255, 255, 255, 255);
        let text_c = rgba(255, 246, 247, 255);
        let muted_c = rgba(186, 162, 166, 255);
        let danger_c = rgba(239, 68, 68, 245);
        let _ = color::hex;

        // Keep the NX overlay button visible regardless of touch/HUD fade state whenever
        // `overlay_button_visible` allowed it to be drawn.
        let overlay_alpha = settings.overlay_opacity.clamp(0.20, 1.0);
        let bubble_alpha = if settings.overlay_hidden && !self.floating_menu {
            0.0
        } else {
            overlay_alpha
        };

        // Draw moveable Floating Menu Button (MCPE crimson-obsidian bevel NX button OR GoyControl NX pill)
        if !self.expanded && bubble_alpha > 0.001 {
            let [btn_x, btn_y, btn_w, btn_h] = floating_button_rect(w, h, settings);
            let ov_scale = settings.overlay_scale.clamp(0.5, 2.0);
            if settings.control_style == ControlStyle::Mcpe {
                let scale_rgba = |r: u8, g: u8, bl: u8, al: u8| -> u32 {
                    rgba(r, g, bl, (f32::from(al) * bubble_alpha).round() as u8)
                };
                // Nexora Obsidian-Crimson MCPE top-bar NX button plate
                ui.rounded(
                    btn_x + 1.5,
                    btn_y + 2.0,
                    btn_w,
                    btn_h,
                    4.0,
                    scale_rgba(0, 0, 0, 155),
                );
                ui.rounded(btn_x, btn_y, btn_w, btn_h, 4.0, scale_rgba(220, 20, 60, 230));
                let body_col = if self.floating_menu {
                    scale_rgba(220, 20, 60, 245)
                } else {
                    scale_rgba(18, 5, 8, 238)
                };
                ui.rounded(
                    btn_x + 2.0,
                    btn_y + 2.0,
                    btn_w - 4.0,
                    btn_h - 4.0,
                    3.0,
                    body_col,
                );
                let hi_bevel = scale_rgba(248, 113, 113, 210);
                let lo_bevel = scale_rgba(90, 14, 28, 225);
                ui.fill(btn_x + 2.0, btn_y + 2.0, btn_w - 4.0, 2.0, hi_bevel);
                ui.fill(btn_x + 2.0, btn_y + 4.0, 2.0, btn_h - 6.0, hi_bevel);
                ui.fill(btn_x + 2.0, btn_y + btn_h - 4.0, btn_w - 4.0, 2.0, lo_bevel);
                ui.fill(btn_x + btn_w - 4.0, btn_y + 4.0, 2.0, btn_h - 6.0, lo_bevel);
                let v_fg = scale_rgba(255, 255, 255, 255);
                let v_sh = scale_rgba(0, 0, 0, 220);
                let font_sz = (14.5 * ov_scale).clamp(9.0, 25.0);
                ui.centered_fit(
                    Face::SemiBold,
                    font_sz,
                    [btn_x + 1.0, btn_y + 1.0, btn_w, btn_h],
                    v_sh,
                    "NX",
                );
                ui.centered_fit(
                    Face::SemiBold,
                    font_sz,
                    [btn_x, btn_y, btn_w, btn_h],
                    v_fg,
                    "NX",
                );
            } else {
                let rad = (14.0 * ov_scale).clamp(8.0, 24.0);
                let font_sz = (15.0 * ov_scale).clamp(9.0, 26.0);
                ui.rounded(
                    btn_x + 1.5,
                    btn_y + 3.0,
                    btn_w,
                    btn_h,
                    rad,
                    rgba(0, 0, 0, (145.0 * bubble_alpha).round() as u8),
                );
                ui.rounded(
                    btn_x,
                    btn_y,
                    btn_w,
                    btn_h,
                    rad,
                    color::with_alpha(
                        if self.floating_menu { accent } else { surface_bg },
                        bubble_alpha,
                    ),
                );
                ui.stroke_rounded(
                    btn_x,
                    btn_y,
                    btn_w,
                    btn_h,
                    rad,
                    1.6,
                    color::with_alpha(accent, bubble_alpha),
                );
                // Branded Nexora "NX" floating trigger
                ui.centered_fit(
                    Face::SemiBold,
                    font_sz,
                    [btn_x, btn_y, btn_w, btn_h],
                    color::with_alpha(
                        if self.floating_menu {
                            accent_fg
                        } else {
                            text_c
                        },
                        bubble_alpha,
                    ),
                    "NX",
                );
            }
        }

        if !self.floating_menu {
            ui.finish();
            return;
        }

        // Backdrop scrim & Modal Shell with expressive entrance slide
        let scrim_a = (165.0 * open_ease.clamp(0.35, 1.0)).round() as u8;
        ui.fill(0.0, 0.0, w, h, rgba(0, 0, 0, scrim_a));
        let slide_y = (1.0 - open_ease) * 18.0;

        if self.booster_mode {
            let bl = booster_layout(w, h, slide_y);
            let [bx, by, bw, bh] = bl.shell;
            crate::nexora_art::draw_crimson_shards(&mut ui, w, h, 0.32 * open_ease);
            ui.rounded(bx + 2.0, by + 6.0, bw, bh, 18.0, rgba(0, 0, 0, 185));
            ui.rounded(bx, by, bw, bh, 18.0, rgba(12, 4, 6, 246));
            ui.fill(bx + 14.0, by + 43.0, bw - 28.0, 1.0, border_c);
            ui.stroke_rounded(bx, by, bw, bh, 18.0, 1.5, border_strong);

            // Header: Crimson Feather + NEXORA BOOSTER + [GRAPH | NUMERIC] + CONTROL HUB + Close
            crate::nexora_art::draw_feather(&mut ui, bx + 14.0, by + 10.0, 22.0, 1.0);
            ui.text(Face::SemiBold, 13.4, bx + 42.0, by + 11.0, text_c, "NEXORA");
            let nw = ui.measure(Face::SemiBold, 13.4, "NEXORA ");
            ui.text(Face::SemiBold, 11.4, bx + 42.0 + nw, by + 13.0, accent_hi, "BOOSTER");

            // Graph / Numeric Segmented Toggle
            let vt = bl.view_toggle;
            ui.rounded(vt[0], vt[1], vt[2], vt[3], 12.0, well_bg);
            ui.border(vt[0], vt[1], vt[2], vt[3], 12.0, border_c);
            let half_vt = vt[2] * 0.5;
            if self.booster_graph_mode {
                ui.rounded(vt[0] + 2.0, vt[1] + 2.0, half_vt - 2.0, vt[3] - 4.0, 10.0, accent);
            } else {
                ui.rounded(vt[0] + half_vt, vt[1] + 2.0, half_vt - 2.0, vt[3] - 4.0, 10.0, accent);
            }
            ui.centered_fit(
                Face::SemiBold,
                8.8,
                [vt[0], vt[1], half_vt, vt[3]],
                if self.booster_graph_mode { accent_fg } else { muted_c },
                "GRAPH",
            );
            ui.centered_fit(
                Face::SemiBold,
                8.8,
                [vt[0] + half_vt, vt[1], half_vt, vt[3]],
                if !self.booster_graph_mode { accent_fg } else { muted_c },
                "NUMERIC",
            );

            // Control Hub Switch Pill
            let hb = bl.hub_btn;
            ui.rounded(hb[0], hb[1], hb[2], hb[3], 12.0, card_sel);
            ui.border(hb[0], hb[1], hb[2], hb[3], 12.0, accent);
            ui.centered_fit(Face::SemiBold, 8.8, hb, text_c, "CONTROL HUB");

            // Close button
            let xr = bl.close_x;
            ui.rounded(xr[0], xr[1], xr[2], xr[3], 13.0, card_bg);
            ui.border(xr[0], xr[1], xr[2], xr[3], 13.0, border_c);
            ui.centered_fit(Face::SemiBold, 12.5, xr, text_c, "×");

            // Left Column: 2x2 Circular Telemetry Gauges OR Live FPS Sparkline Graph
            let t = &self.telemetry;
            let target_hz: f32 = if self.booster_120hz { 120.0 } else { 60.0 };
            if self.booster_graph_mode {
                let g0 = bl.gauges[0];
                let gw = bl.left_rect[2];
                let gh = bl.gauges[2][1] + bl.gauges[2][3] - g0[1];
                ui.rounded(g0[0], g0[1], gw, gh, 12.0, rgba(18, 6, 9, 236));
                ui.border(g0[0], g0[1], gw, gh, 12.0, border_c);
                let hdr = format!(
                    "{:.0} FPS  •  {:.1}ms  •  {:.1}°C",
                    t.fps, t.frame_ms, t.temp_c
                );
                ui.text(Face::SemiBold, 10.6, g0[0] + 10.0, g0[1] + 8.0, text_c, &hdr);
                let plot_x = g0[0] + 10.0;
                let plot_y = g0[1] + 28.0;
                let plot_w = (gw - 20.0).max(40.0);
                let plot_h = (gh - 40.0).max(32.0);
                ui.rounded(plot_x, plot_y, plot_w, plot_h, 6.0, rgba(10, 3, 5, 220));
                for gi in 1..4 {
                    let ly = plot_y + plot_h * (gi as f32 / 4.0);
                    ui.fill(plot_x, ly, plot_w, 1.0, rgba(220, 20, 60, 36));
                }
                let count = t.fps_history.len();
                let bar_w = (plot_w / count as f32 - 1.0).max(1.5);
                for (si, &sample_fps) in t.fps_history.iter().enumerate() {
                    let frac = (sample_fps / target_hz.max(60.0)).clamp(0.06, 1.0);
                    let bh_bar = (plot_h - 6.0) * frac;
                    let sx = plot_x + si as f32 * (plot_w / count as f32);
                    let sy = plot_y + plot_h - 3.0 - bh_bar;
                    let bar_col = if frac >= 0.80 {
                        rgba(239, 68, 68, 220)
                    } else if frac >= 0.50 {
                        rgba(245, 158, 11, 210)
                    } else {
                        rgba(185, 28, 28, 200)
                    };
                    ui.fill(sx, sy, bar_w, bh_bar, bar_col);
                }
            } else {
                let cpu_pct = ((t.cpu_ms / 16.6) * 100.0).clamp(4.0, 100.0);
                let gpu_pct = ((t.gpu_ms / 16.6) * 100.0).clamp(4.0, 100.0);
                let dials: [(&str, String, f32); 4] = [
                    (
                        "FPS",
                        format!("{:.0}", t.fps),
                        (t.fps / target_hz.max(60.0) * 100.0).clamp(5.0, 100.0),
                    ),
                    (
                        "CPU",
                        format!("{:.0}%", cpu_pct),
                        cpu_pct.clamp(5.0, 100.0),
                    ),
                    (
                        "GPU",
                        format!("{:.0}%", gpu_pct),
                        gpu_pct.clamp(5.0, 100.0),
                    ),
                    (
                        "TEMP",
                        format!("{:.0}°", t.temp_c),
                        ((t.temp_c - 22.0) / 36.0 * 100.0).clamp(8.0, 100.0),
                    ),
                ];
                for (di, (lbl, val, pct)) in dials.iter().enumerate() {
                    let gr = bl.gauges[di];
                    ui.rounded(gr[0], gr[1], gr[2], gr[3], 11.0, rgba(20, 7, 10, 232));
                    ui.border(gr[0], gr[1], gr[2], gr[3], 11.0, border_c);
                    let dial_r = (gr[3].min(gr[2]) * 0.43).clamp(18.0, 29.0);
                    let cx_d = gr[0] + gr[2] * 0.5;
                    let cy_d = gr[1] + gr[3] * 0.5 + 1.0;
                    crate::nexora_art::draw_booster_gauge(
                        &mut ui,
                        cx_d,
                        cy_d,
                        dial_r,
                        *pct,
                        val,
                        lbl,
                        1.0,
                    );
                }
            }

            // 4-Mode Selector Bar: ECO | BALANCED | BOOST | DEMONIC
            let mode_names = ["ECO", "BALANCED", "BOOST", "DEMONIC"];
            for (mi, mr) in bl.modes.iter().enumerate() {
                let active = self.booster_profile == mi as u8;
                ui.rounded(
                    mr[0],
                    mr[1],
                    mr[2],
                    mr[3],
                    8.0,
                    if active { accent } else { card_bg },
                );
                ui.border(
                    mr[0],
                    mr[1],
                    mr[2],
                    mr[3],
                    8.0,
                    if active { accent_hi } else { border_c },
                );
                if active {
                    ui.fill(mr[0] + 4.0, mr[1], (mr[2] - 8.0).max(6.0), 1.3, accent_edge);
                }
                ui.centered_fit(
                    Face::SemiBold,
                    8.8,
                    *mr,
                    if active { accent_fg } else { muted_c },
                    mode_names[mi],
                );
            }

            // RAM & Frame Time Resource Bars
            let ram_frac = (t.ram_used_mb as f32 / t.ram_total_mb.max(1) as f32).clamp(0.05, 1.0);
            let ram_txt = format!(
                "RAM  {:.1} / {:.1} GB",
                t.ram_used_mb as f32 / 1024.0,
                t.ram_total_mb as f32 / 1024.0
            );
            let rb = bl.ram_bar;
            ui.rounded(rb[0], rb[1], rb[2], rb[3], 8.0, rgba(18, 6, 9, 232));
            ui.border(rb[0], rb[1], rb[2], rb[3], 8.0, border_c);
            ui.text(Face::SemiBold, 8.8, rb[0] + 8.0, rb[1] + 4.0, text_c, &ram_txt);
            let r_track = [rb[0] + 8.0, rb[1] + rb[3] - 9.0, rb[2] - 16.0, 5.0];
            ui.rounded(r_track[0], r_track[1], r_track[2], r_track[3], 2.5, well_bg);
            ui.rounded(
                r_track[0],
                r_track[1],
                (r_track[2] * ram_frac).max(4.0),
                r_track[3],
                2.5,
                accent,
            );

            let frame_frac = (t.frame_ms / 33.3).clamp(0.05, 1.0);
            let hz_lbl = if self.booster_120hz { "120Hz" } else { "60Hz" };
            let ft_txt = format!("FRAME  {:.1} ms  •  {}", t.frame_ms, hz_lbl);
            let fb = bl.frame_bar;
            ui.rounded(fb[0], fb[1], fb[2], fb[3], 8.0, rgba(18, 6, 9, 232));
            ui.border(fb[0], fb[1], fb[2], fb[3], 8.0, border_c);
            ui.text(Face::SemiBold, 8.8, fb[0] + 8.0, fb[1] + 4.0, text_c, &ft_txt);
            let f_track = [fb[0] + 8.0, fb[1] + fb[3] - 9.0, fb[2] - 16.0, 5.0];
            ui.rounded(f_track[0], f_track[1], f_track[2], f_track[3], 2.5, well_bg);
            ui.rounded(
                f_track[0],
                f_track[1],
                (f_track[2] * frame_frac).max(4.0),
                f_track[3],
                2.5,
                accent_hi,
            );

            // Right Column: 6 Quick Optimization Tiles
            let tiles_meta: [(Icon, &str, &str, bool); 6] = [
                (
                    Icon::Bolt,
                    "120 Hz",
                    if self.booster_120hz { "HIGH REFRESH" } else { "60 HZ SAVER" },
                    self.booster_120hz,
                ),
                (Icon::Fan, "RAM Purge", "FREE BUFFERS", false),
                (
                    Icon::Shield,
                    "Touch Lock",
                    if self.booster_touch_lock { "ARMED" } else { "UNLOCKED" },
                    self.booster_touch_lock,
                ),
                (
                    Icon::Mods,
                    "Entity Cull",
                    if self.volt_extras.entity_culling.enabled { "ACTIVE" } else { "OFF" },
                    self.volt_extras.entity_culling.enabled,
                ),
                (
                    Icon::Crosshair,
                    "Zero-Lat",
                    if settings.zero_latency_touch { "INSTANT" } else { "STANDARD" },
                    settings.zero_latency_touch,
                ),
                (
                    Icon::Gauge,
                    "FPS HUD",
                    if self.volt_extras.telemetry_hud.show_fps { "SHOWN" } else { "HIDDEN" },
                    self.volt_extras.telemetry_hud.show_fps,
                ),
            ];
            for (ti, (ic, title, sub, active)) in tiles_meta.iter().enumerate() {
                let tr = bl.tiles[ti];
                ui.rounded(
                    tr[0],
                    tr[1],
                    tr[2],
                    tr[3],
                    10.0,
                    if *active { card_sel } else { card_bg },
                );
                ui.border(
                    tr[0],
                    tr[1],
                    tr[2],
                    tr[3],
                    10.0,
                    if *active { accent } else { border_c },
                );
                if *active {
                    ui.rounded(tr[0] + 3.0, tr[1] + 6.0, 2.5, (tr[3] - 12.0).max(6.0), 1.25, accent_hi);
                }
                ui.icon(
                    *ic,
                    tr[0] + 10.0,
                    tr[1] + (tr[3] - 15.0) * 0.5,
                    15.0,
                    if *active { accent_hi } else { muted_c },
                );
                ui.text(
                    Face::SemiBold,
                    9.8,
                    tr[0] + 31.0,
                    tr[1] + (tr[3] * 0.5 - 11.0).max(4.0),
                    text_c,
                    title,
                );
                ui.text(
                    Face::Medium,
                    7.8,
                    tr[0] + 31.0,
                    tr[1] + (tr[3] * 0.5 + 2.0),
                    if *active { accent_hi } else { muted_c },
                    sub,
                );
            }

            // Bottom Action Row: DND | VSYNC | IGNITE BOOST
            let db = bl.dnd_btn;
            ui.rounded(
                db[0],
                db[1],
                db[2],
                db[3],
                9.0,
                if self.booster_dnd { card_sel } else { card_bg },
            );
            ui.border(
                db[0],
                db[1],
                db[2],
                db[3],
                9.0,
                if self.booster_dnd { accent } else { border_c },
            );
            ui.icon(
                Icon::Pause,
                db[0] + 7.0,
                db[1] + 12.0,
                12.0,
                if self.booster_dnd { accent_hi } else { muted_c },
            );
            ui.text(
                Face::SemiBold,
                8.8,
                db[0] + 22.0,
                db[1] + 12.0,
                if self.booster_dnd { text_c } else { muted_c },
                "DND",
            );

            let vb = bl.vsync_btn;
            ui.rounded(
                vb[0],
                vb[1],
                vb[2],
                vb[3],
                9.0,
                if t.vsync { card_sel } else { card_bg },
            );
            ui.border(
                vb[0],
                vb[1],
                vb[2],
                vb[3],
                9.0,
                if t.vsync { accent } else { border_c },
            );
            ui.centered_fit(
                Face::SemiBold,
                8.8,
                vb,
                if t.vsync { text_c } else { muted_c },
                if t.vsync { "VSYNC ON" } else { "VSYNC OFF" },
            );

            let ib = bl.ignite_btn;
            ui.rounded(ib[0], ib[1], ib[2], ib[3], 10.0, accent);
            ui.fill(ib[0] + 5.0, ib[1], (ib[2] - 10.0).max(8.0), 1.4, accent_edge);
            ui.icon(Icon::Flame, ib[0] + 10.0, ib[1] + 11.0, 14.0, accent_fg);
            ui.centered_fit(
                Face::SemiBold,
                10.4,
                [ib[0] + 22.0, ib[1], (ib[2] - 26.0).max(40.0), ib[3]],
                accent_fg,
                "IGNITE BOOST",
            );

            // Ephemeral toast banner inside Booster header
            if let Some((msg, at)) = &self.booster_toast
                && at.elapsed() < std::time::Duration::from_millis(2400)
            {
                let tw = (ui.measure(Face::SemiBold, 9.2, msg) + 22.0).clamp(140.0, bw - 32.0);
                let tx = bx + (bw - tw) * 0.5;
                let ty = by + bh - 26.0;
                ui.rounded(tx, ty, tw, 20.0, 10.0, rgba(32, 8, 12, 245));
                ui.border(tx, ty, tw, 20.0, 10.0, accent_hi);
                ui.centered_fit(Face::SemiBold, 9.0, [tx, ty, tw, 20.0], text_c, msg);
            }

            ui.finish();
            return;
        }

        let (dw, dh, dx, dy_base, side_w, cx, cw, body_top_base, body_h) = hub_metrics(w, h);
        let dy = dy_base + slide_y;
        let body_top = body_top_base + slide_y;
        let sb_rects = hub_sidebar_rects(dx, dy, dw, dh, side_w);

        ui.rounded(dx + 2.0, dy + 6.0, dw, dh, 18.0, rgba(0, 0, 0, 175));
        ui.rounded(dx, dy, dw, dh, 18.0, surface_bg);

        // Left Sidebar Shell & Vertical Divider
        ui.rounded(dx + 1.0, dy + 1.0, side_w - 1.0, dh - 2.0, 17.0, sidebar_bg);
        ui.fill(dx + side_w - 12.0, dy + 1.0, 12.0, dh - 2.0, sidebar_bg);
        ui.fill(dx + side_w, dy + 1.0, 1.0, dh - 2.0, border_c);
        ui.fill(dx + side_w, dy + 47.0, dw - side_w, 1.0, border_c);
        ui.stroke_rounded(dx, dy, dw, dh, 18.0, 1.4, border_c);

        // Sidebar Brand Header: Nexora Feather + "Nexora" + "Control Hub"
        let brand_x = dx + 12.0;
        let brand_y = dy + 10.0;
        ui.rounded(brand_x, brand_y + 2.0, 22.0, 22.0, 6.0, accent);
        ui.fill(brand_x + 2.0, brand_y + 2.0, 18.0, 1.0, accent_edge);
        crate::nexora_art::draw_feather(&mut ui, brand_x + 3.0, brand_y + 5.0, 16.0, 1.0);
        ui.text(Face::SemiBold, 12.2, brand_x + 28.0, brand_y, text_c, "Nexora");
        ui.text(Face::Regular, 9.2, brand_x + 28.0, brand_y + 15.0, muted_c, "Control Hub");

        // Quick Controls Studio Launch Pill in Sidebar
        let qs = sb_rects.quick_studio;
        ui.rounded(qs[0], qs[1], qs[2], qs[3], 8.0, well_bg);
        ui.border(qs[0], qs[1], qs[2], qs[3], 8.0, border_c);
        ui.icon(Icon::MicrosoftGrid, qs[0] + 7.0, qs[1] + 5.0, 13.0, accent_hi);
        ui.text(
            Face::Medium,
            9.2,
            qs[0] + 24.0,
            qs[1] + 6.0,
            muted_c,
            "Customize HUD",
        );

        // 10 Category Tabs in Sidebar
        let tabs: [(Icon, &str); 10] = [
            (Icon::HorizonLogo, "Recommended"),
            (Icon::Gamepad, "Controls"),
            (Icon::Crosshair, "Aiming"),
            (Icon::MicrosoftGrid, "Buttons"),
            (Icon::Flask, "Advanced"),
            (Icon::Monitor, "Record"),
            (Icon::Play, "Replay"),
            (Icon::Settings, "Overlay"),
            (Icon::Music, "Music"),
            (Icon::Mods, "Nexora Mods"),
        ];
        for (ti, (ic, lbl)) in tabs.iter().enumerate() {
            let active = self.hub_tab == ti as u8;
            let tr = sb_rects.tabs[ti];
            if active {
                ui.rounded(tr[0], tr[1], tr[2], tr[3], 8.0, card_sel);
                ui.stroke_rounded(tr[0], tr[1], tr[2], tr[3], 8.0, 1.3, border_strong);
                ui.rounded(tr[0] + 2.0, tr[1] + 4.0, 2.5, (tr[3] - 8.0).max(6.0), 1.25, accent_hi);
            } else {
                ui.rounded(tr[0], tr[1], tr[2], tr[3], 8.0, rgba(255, 255, 255, 10));
            }
            ui.icon(
                *ic,
                tr[0] + 9.0,
                tr[1] + (tr[3] - 13.0) * 0.5,
                13.0,
                if active { accent_hi } else { muted_c },
            );
            ui.text(
                if active { Face::SemiBold } else { Face::Medium },
                9.8,
                tr[0] + 27.0,
                tr[1] + (tr[3] - 12.0) * 0.5,
                if active { text_c } else { muted_c },
                lbl,
            );
        }

        // Bottom-left Purple "Done" Button (matching Option 2 & Volt Graphics)
        let db = sb_rects.done_btn;
        ui.rounded(db[0], db[1], db[2], db[3], 9.0, accent);
        ui.fill(db[0] + 4.0, db[1], (db[2] - 8.0).max(8.0), 1.2, accent_edge);
        ui.centered_fit(Face::SemiBold, 11.2, db, accent_fg, "✓ Done");

        // Top Header Bar (Right Panel): Active Category Title + Mode Badge + Quick Close '×'
        let active_tab = tabs[(self.hub_tab as usize).min(tabs.len() - 1)];
        let hdr_x = dx + side_w + 14.0;
        ui.rounded(hdr_x, dy + 11.0, 24.0, 24.0, 7.0, card_sel);
        ui.border(hdr_x, dy + 11.0, 24.0, 24.0, 7.0, accent);
        ui.icon(active_tab.0, hdr_x + 5.0, dy + 16.0, 14.0, accent_hi);
        ui.text(Face::SemiBold, 13.2, hdr_x + 32.0, dy + 11.0, text_c, active_tab.1);
        ui.text(
            Face::Regular,
            8.8,
            hdr_x + 32.0,
            dy + 27.0,
            muted_c,
            "Touch controls, virtual mouse & feedback",
        );

        let booster_r = sb_rects.booster_pill;
        if booster_r[0] > hdr_x + 150.0 {
            ui.rounded(booster_r[0], booster_r[1], booster_r[2], booster_r[3], 12.0, accent);
            ui.border(booster_r[0], booster_r[1], booster_r[2], booster_r[3], 12.0, accent_hi);
            ui.centered_fit(Face::SemiBold, 9.0, booster_r, accent_fg, "BOOSTER");
        }
        let mode_badge_txt = if settings.control_style == ControlStyle::Mcpe {
            "MCPE Controls"
        } else {
            "GoyControl HUD"
        };
        let badge_r = sb_rects.mode_badge;
        if badge_r[0] > hdr_x + 150.0 {
            ui.rounded(badge_r[0], badge_r[1], badge_r[2], badge_r[3], 12.0, card_sel);
            ui.border(badge_r[0], badge_r[1], badge_r[2], badge_r[3], 12.0, accent);
            ui.centered_fit(Face::SemiBold, 9.2, badge_r, accent_hi, mode_badge_txt);
        }

        let xr = sb_rects.close_x;
        ui.rounded(xr[0], xr[1], xr[2], xr[3], 13.0, card_bg);
        ui.border(xr[0], xr[1], xr[2], xr[3], 13.0, border_c);
        ui.centered_fit(Face::SemiBold, 12.5, xr, text_c, "×");

        // Scrollable Right Content Panel with scissor clip and tab transition slide
        let tab_slide = (1.0 - tab_ease) * 14.0;
        let music_playlists = if self.hub_tab == 8 {
            crate::internet_music::playlists()
        } else {
            Vec::new()
        };
        let music_sel_idx = if music_playlists.is_empty() {
            0
        } else {
            self.music_selected_playlist.min(music_playlists.len() - 1)
        };
        let music_search_snap = if self.hub_tab == 8 && self.music_search_tab {
            crate::internet_music::in_game_search_snapshot()
        } else {
            Default::default()
        };
        let music_sel_tracks = music_playlists
            .get(music_sel_idx)
            .map_or(0, |p| p.tracks.len());
        let content_height = match self.hub_tab {
            0 => 446.0,
            1 => 368.0,
            2 => 410.0,
            3 => 242.0,
            4 => 340.0,
            5 => 488.0,
            6 => 340.0,
            7 => 364.0,
            9 => match self.volt_extras_config_mod {
                None => {
                    let n = crate::volt_extras_ui::filtered_mod_indices_with(
                        &self.volt_extras,
                        self.volt_extras_cat,
                        &self.volt_extras_search,
                        self.volt_extras_filter,
                        self.volt_extras_sort_enabled,
                    )
                    .len()
                    .max(1);
                    102.0 + (n as f32) * 68.0
                }
                Some(idx) => {
                    let rows = crate::volt_extras_ui::mod_setting_rows(&self.volt_extras, idx).len();
                    50.0 + (rows as f32) * 54.0
                }
            },
            _ if self.music_search_tab => {
                284.0 + 76.0 + (music_search_snap.results.len().max(1) as f32) * 50.0 + 48.0
            }
            _ => {
                302.0
                    + (music_playlists.len().max(1) as f32) * 42.0
                    + 32.0
                    + (music_sel_tracks.max(1) as f32) * 42.0
                    + 16.0
            }
        };
        let max_scroll = (content_height - body_h).max(0.0);
        self.hub_max_scroll.set(max_scroll);
        let cur_scroll = self.hub_scroll.clamp(0.0, max_scroll);
        let body_y = body_top - cur_scroll + tab_slide;

        ui.set_clip(Some([cx - 2.0, body_top, cw + 4.0, body_h]));

        let draw_toggle_row = |ui: &mut Ui<'_>,
                               ry: f32,
                               title: &str,
                               sub: &str,
                               on: Option<bool>,
                               right_badge: Option<&str>| {
            let row_h = 46.0;
            ui.rounded(cx, ry, cw, row_h, 11.0, card_bg);
            ui.border(cx, ry, cw, row_h, 11.0, border_c);
            ui.text(Face::SemiBold, 11.0, cx + 14.0, ry + 6.0, text_c, title);
            ui.text(Face::Regular, 9.2, cx + 14.0, ry + 24.0, muted_c, sub);
            if let Some(state) = on {
                let sw_x = cx + cw - 54.0;
                let sw_y = ry + 12.0;
                ui.rounded(
                    sw_x,
                    sw_y,
                    40.0,
                    22.0,
                    11.0,
                    if state { accent } else { well_bg },
                );
                ui.border(
                    sw_x,
                    sw_y,
                    40.0,
                    22.0,
                    11.0,
                    if state { accent_hi } else { border_c },
                );
                let knob_x = if state { sw_x + 21.0 } else { sw_x + 3.0 };
                ui.rounded(knob_x, sw_y + 3.0, 16.0, 16.0, 8.0, accent_fg);
            } else if let Some(badge) = right_badge {
                let bw = 64.0;
                let bx = cx + cw - bw - 10.0;
                let by = ry + 10.0;
                ui.rounded(bx, by, bw, 26.0, 8.0, card_sel);
                ui.border(bx, by, bw, 26.0, 8.0, accent);
                ui.centered_fit(Face::SemiBold, 10.5, [bx, by, bw, 26.0], accent_hi, badge);
            } else {
                ui.text(
                    Face::SemiBold,
                    14.0,
                    cx + cw - 20.0,
                    ry + 12.0,
                    accent_hi,
                    "›",
                );
            }
        };

        match self.hub_tab {
            0 => {
                // Tab 0: Recommended (Real-Time Resolution, Touch Sensitivity, Virtual Mouse, Quick Controls & Animations)
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "REAL-TIME 3D RESOLUTION & PERFORMANCE",
                );
                let res_y = body_y + 18.0;
                ui.rounded(cx, res_y, cw, 78.0, 11.0, card_bg);
                ui.border(cx, res_y, cw, 78.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    res_y + 6.0,
                    text_c,
                    "Real-Time 3D Resolution Scale (Zero-Lag)",
                );
                let res_pct = settings.resolution_scale_percent.clamp(25, 100);
                let res_txt = format!("{res_pct}%");
                let res_pill_w = 52.0;
                ui.rounded(
                    cx + cw - res_pill_w - 12.0,
                    res_y + 6.0,
                    res_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - res_pill_w - 12.0,
                    res_y + 6.0,
                    res_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - res_pill_w - 12.0, res_y + 6.0, res_pill_w, 18.0],
                    accent_hi,
                    &res_txt,
                );
                let r_sl_x = cx + 14.0;
                let r_sl_w = (cw - 28.0).max(40.0);
                let r_sl_y = res_y + 32.0;
                let r_frac = ((res_pct as f32 - 25.0) / 75.0).clamp(0.0, 1.0);
                ui.rounded(r_sl_x, r_sl_y, r_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(r_sl_x, r_sl_y, (r_sl_w * r_frac).max(6.0), 6.0, 3.0, accent);
                let r_thumb_x = r_sl_x + r_sl_w * r_frac;
                ui.rounded(r_thumb_x - 7.0, r_sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(r_thumb_x - 7.0, r_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                let chip_y = res_y + 48.0;
                let chip_gap = 6.0;
                let chip_w = (cw - 28.0 - chip_gap * 4.0) / 5.0;
                for (ci, pct) in [50u32, 65, 75, 85, 100].into_iter().enumerate() {
                    let rx = cx + 14.0 + ci as f32 * (chip_w + chip_gap);
                    let on = (res_pct as i32 - pct as i32).abs() <= 2;
                    ui.rounded(
                        rx,
                        chip_y,
                        chip_w,
                        22.0,
                        7.0,
                        if on { accent } else { sidebar_bg },
                    );
                    if !on {
                        ui.border(rx, chip_y, chip_w, 22.0, 7.0, border_c);
                    }
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [rx + 2.0, chip_y, chip_w - 4.0, 22.0],
                        if on { accent_fg } else { text_c },
                        &format!("{pct}%"),
                    );
                }

                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y + 102.0,
                    muted_c,
                    "FAST AIM, VIRTUAL MOUSE & CONTROLS",
                );
                let sens_y = body_y + 118.0;
                ui.rounded(cx, sens_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, sens_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    sens_y + 6.0,
                    text_c,
                    "Touch Look Sensitivity",
                );
                let pct_txt = format!("{:.0}%", settings.sensitivity * 100.0);
                let pill_w = 48.0;
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, sens_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &pct_txt,
                );
                let sl_x = cx + 14.0;
                let sl_w = (cw - 28.0).max(40.0);
                let sl_y = sens_y + 33.0;
                let frac = ((settings.sensitivity - 0.10) / 2.90).clamp(0.0, 1.0);
                ui.rounded(sl_x, sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, sl_y, (sl_w * frac).max(6.0), 6.0, 3.0, accent);
                let thumb_x = sl_x + sl_w * frac;
                ui.rounded(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                draw_toggle_row(
                    &mut ui,
                    body_y + 176.0,
                    "Virtual Mouse Pointer",
                    "Off by default; enable for desktop pointer in menus & inventory",
                    Some(settings.virtual_mouse_enabled),
                    None,
                );

                let spd_y = body_y + 230.0;
                ui.rounded(cx, spd_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, spd_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    spd_y + 6.0,
                    text_c,
                    "Virtual Mouse Speed",
                );
                let spd_pct = format!("{:.0}%", settings.virtual_mouse_sensitivity * 100.0);
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    spd_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    spd_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, spd_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &spd_pct,
                );
                let spd_sl_y = spd_y + 33.0;
                let spd_frac = ((settings.virtual_mouse_sensitivity - 0.25) / 2.75).clamp(0.0, 1.0);
                ui.rounded(sl_x, spd_sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, spd_sl_y, (sl_w * spd_frac).max(6.0), 6.0, 3.0, accent);
                let spd_thumb_x = sl_x + sl_w * spd_frac;
                ui.rounded(
                    spd_thumb_x - 7.0,
                    spd_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(spd_thumb_x - 7.0, spd_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                let act_y = body_y + 288.0;
                let half_w = (cw - 10.0) * 0.5;
                let sys_sub = format!("Active: {}", settings.control_style.label());
                for (i, (ic, title, sub)) in [
                    (Icon::Gamepad, "Switch Control System", sys_sub.as_str()),
                    (
                        Icon::MicrosoftGrid,
                        "Customize Controls",
                        "Open layout & property studio",
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let rx = cx + i as f32 * (half_w + 10.0);
                    let is_primary = i == 1;
                    ui.rounded(
                        rx,
                        act_y,
                        half_w,
                        44.0,
                        11.0,
                        if is_primary { accent } else { card_bg },
                    );
                    ui.border(
                        rx,
                        act_y,
                        half_w,
                        44.0,
                        11.0,
                        if is_primary { accent_edge } else { border_c },
                    );
                    ui.rounded(
                        rx + 10.0,
                        act_y + 9.0,
                        26.0,
                        26.0,
                        8.0,
                        if is_primary { accent_hi } else { card_sel },
                    );
                    ui.icon(
                        ic,
                        rx + 15.0,
                        act_y + 14.0,
                        16.0,
                        if is_primary { accent_fg } else { accent_hi },
                    );
                    ui.text(
                        Face::SemiBold,
                        10.4,
                        rx + 42.0,
                        act_y + 6.0,
                        if is_primary { accent_fg } else { text_c },
                        title,
                    );
                    ui.text(
                        Face::Regular,
                        8.6,
                        rx + 42.0,
                        act_y + 23.0,
                        if is_primary { accent_edge } else { muted_c },
                        sub,
                    );
                }

                let send_y = body_y + 340.0;
                ui.rounded(cx, send_y, cw, 44.0, 11.0, card_sel);
                ui.border(cx, send_y, cw, 44.0, 11.0, accent);
                ui.icon(Icon::Gamepad, cx + 12.0, send_y + 13.0, 18.0, accent_hi);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 38.0,
                    send_y + 6.0,
                    text_c,
                    "Send Key (Keyboard / Mouse / Action)",
                );
                ui.text(
                    Face::Regular,
                    8.8,
                    cx + 38.0,
                    send_y + 23.0,
                    muted_c,
                    "Fast access: open 104-key PC keyboard, mouse buttons, scroll & actions",
                );
                ui.text(
                    Face::SemiBold,
                    13.5,
                    cx + cw - 18.0,
                    send_y + 13.0,
                    accent_hi,
                    "›",
                );

                draw_toggle_row(
                    &mut ui,
                    body_y + 392.0,
                    "In-Game Animations",
                    "Animated block textures, view bobbing, FOV transitions & menu motion",
                    Some(!settings.disable_game_animations),
                    None,
                );
            }
            1 => {
                // 1. Control Mode Section
                ui.text(Face::SemiBold, 10.2, cx, body_y, muted_c, "CONTROL MODE");
                let mode_y = body_y + 18.0;
                let half_w = (cw - 10.0) * 0.5;
                let is_mcpe = settings.control_style == ControlStyle::Mcpe;
                for (i, (title, sub, sel)) in [
                    ("MCPE (Default)", "Vanilla touch controls", is_mcpe),
                    ("GoyControl", "Custom layout & keybinds", !is_mcpe),
                ]
                .into_iter()
                .enumerate()
                {
                    let rx = cx + i as f32 * (half_w + 10.0);
                    ui.rounded(
                        rx,
                        mode_y,
                        half_w,
                        46.0,
                        11.0,
                        if sel { card_sel } else { card_bg },
                    );
                    ui.stroke_rounded(
                        rx,
                        mode_y,
                        half_w,
                        46.0,
                        11.0,
                        if sel { 1.6 } else { 1.0 },
                        if sel { accent } else { border_c },
                    );
                    let rad_x = rx + 11.0;
                    let rad_y = mode_y + 16.0;
                    ui.stroke_rounded(
                        rad_x,
                        rad_y,
                        14.0,
                        14.0,
                        7.0,
                        1.5,
                        if sel { accent_hi } else { muted_c },
                    );
                    if sel {
                        ui.rounded(rad_x + 3.5, rad_y + 3.5, 7.0, 7.0, 3.5, accent_hi);
                    }
                    ui.text(Face::SemiBold, 11.0, rx + 31.0, mode_y + 7.0, text_c, title);
                    ui.text(Face::Regular, 9.0, rx + 31.0, mode_y + 25.0, muted_c, sub);
                }

                // 2. Dynamic System Controls Section
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y + 68.0,
                    muted_c,
                    settings.control_style.section_title(),
                );
                let act_y = body_y + 84.0;
                let active_preset_lbl = format!("Preset: {}", settings.preset_name);
                for (i, (ic, title, sub)) in [
                    (
                        Icon::MicrosoftGrid,
                        "Customize Controls",
                        "Open layout editor",
                    ),
                    (
                        Icon::Settings,
                        "Control Presets",
                        active_preset_lbl.as_str(),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let rx = cx + i as f32 * (half_w + 10.0);
                    ui.rounded(rx, act_y, half_w, 44.0, 11.0, card_bg);
                    ui.border(rx, act_y, half_w, 44.0, 11.0, border_c);
                    ui.rounded(rx + 10.0, act_y + 9.0, 26.0, 26.0, 8.0, card_sel);
                    ui.icon(ic, rx + 15.0, act_y + 14.0, 16.0, accent_hi);
                    ui.text(Face::SemiBold, 10.6, rx + 42.0, act_y + 6.0, text_c, title);
                    ui.text(Face::Regular, 8.8, rx + 42.0, act_y + 23.0, muted_c, sub);
                    ui.text(
                        Face::SemiBold,
                        13.0,
                        rx + half_w - 14.0,
                        act_y + 12.0,
                        muted_c,
                        "›",
                    );
                }

                // 3. Sensitivity Slider Card
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y + 132.0,
                    muted_c,
                    "POINTER, CAMERA & AIM",
                );
                let sens_y = body_y + 148.0;
                ui.rounded(cx, sens_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, sens_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    sens_y + 6.0,
                    text_c,
                    "Touch Look Sensitivity",
                );
                let pct_txt = format!("{:.0}%", settings.sensitivity * 100.0);
                let pill_w = 48.0;
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    sens_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, sens_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &pct_txt,
                );
                let sl_x = cx + 14.0;
                let sl_w = (cw - 28.0).max(40.0);
                let sl_y = sens_y + 33.0;
                let frac = ((settings.sensitivity - 0.10) / 2.90).clamp(0.0, 1.0);
                ui.rounded(sl_x, sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, sl_y, (sl_w * frac).max(6.0), 6.0, 3.0, accent);
                let thumb_x = sl_x + sl_w * frac;
                ui.rounded(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(thumb_x - 7.0, sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                // 4. Authoritative Virtual Mouse Toggle, Invert Y-Axis & Split Touch
                draw_toggle_row(
                    &mut ui,
                    body_y + 206.0,
                    "Virtual Mouse Pointer",
                    "Windows 11 pointer; auto-shows when MCPE controls are hidden",
                    Some(settings.virtual_mouse_enabled),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 260.0,
                    "Invert Y-Axis",
                    "Reverse vertical camera look direction",
                    Some(settings.invert_y),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 314.0,
                    "Split Touch (Aim & Look)",
                    "Use center crosshair instead of tap-anywhere interaction",
                    Some(settings.crosshair()),
                    None,
                );
            }
            2 => {
                // Tab 2: Aiming, Camera & Virtual Mouse
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "AIMING, VIRTUAL MOUSE & GYROSCOPE",
                );
                let move_lbl = match settings.mode {
                    crate::touch::Mode::Dpad => "D-Pad + Crosshair/Touch",
                    crate::touch::Mode::Crosshair => "Joystick + Crosshair",
                    crate::touch::Mode::Tap => "Joystick + Tap to Interact",
                };
                let area_lbl = settings.joystick_style.move_area.label();
                let gyro_spd_lbl = format!(
                    "Tilt Multiplier ({:.0}%) — Tap left (−) or right (+)",
                    settings.gyro_sensitivity * 100.0
                );
                let step = 54.0;
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0,
                    "Movement & Aim Mode",
                    move_lbl,
                    None,
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step,
                    "Joystick Active Zone",
                    area_lbl,
                    None,
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 2.0,
                    "Virtual Mouse Pointer",
                    "Authoritative toggle for MCPE and GoyControl menus",
                    Some(settings.virtual_mouse_enabled),
                    None,
                );

                // Virtual Mouse Speed slider card (smooth SetVirtualMouseSpeed slider replacing InGameMenuAction::StepVirtualMouseSpeed +/- buttons)
                let spd_y = body_y + 180.0;
                ui.rounded(cx, spd_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, spd_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    spd_y + 6.0,
                    text_c,
                    "Virtual Mouse Speed",
                );
                let spd_pct = format!("{:.0}%", settings.virtual_mouse_sensitivity * 100.0);
                let spd_pill_w = 48.0;
                ui.rounded(
                    cx + cw - spd_pill_w - 12.0,
                    spd_y + 6.0,
                    spd_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - spd_pill_w - 12.0,
                    spd_y + 6.0,
                    spd_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - spd_pill_w - 12.0, spd_y + 6.0, spd_pill_w, 18.0],
                    accent_hi,
                    &spd_pct,
                );
                let spd_sl_x = cx + 14.0;
                let spd_sl_w = (cw - 28.0).max(40.0);
                let spd_sl_y = spd_y + 33.0;
                let spd_frac = ((settings.virtual_mouse_sensitivity - 0.25) / 2.75).clamp(0.0, 1.0);
                ui.rounded(spd_sl_x, spd_sl_y, spd_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    spd_sl_x,
                    spd_sl_y,
                    (spd_sl_w * spd_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let spd_thumb_x = spd_sl_x + spd_sl_w * spd_frac;
                ui.rounded(
                    spd_thumb_x - 7.0,
                    spd_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(spd_thumb_x - 7.0, spd_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                // Pointer Size slider card
                let size_y = body_y + 238.0;
                ui.rounded(cx, size_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, size_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    size_y + 6.0,
                    text_c,
                    "Pointer Size",
                );
                let size_pct = format!("{:.0}%", settings.virtual_mouse_pointer_scale * 100.0);
                let size_pill_w = 48.0;
                ui.rounded(
                    cx + cw - size_pill_w - 12.0,
                    size_y + 6.0,
                    size_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - size_pill_w - 12.0,
                    size_y + 6.0,
                    size_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [
                        cx + cw - size_pill_w - 12.0,
                        size_y + 6.0,
                        size_pill_w,
                        18.0,
                    ],
                    accent_hi,
                    &size_pct,
                );
                let size_sl_x = cx + 14.0;
                let size_sl_w = (cw - 28.0).max(40.0);
                let size_sl_y = size_y + 33.0;
                let size_frac =
                    ((settings.virtual_mouse_pointer_scale - 0.5) / 1.5).clamp(0.0, 1.0);
                ui.rounded(size_sl_x, size_sl_y, size_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    size_sl_x,
                    size_sl_y,
                    (size_sl_w * size_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let size_thumb_x = size_sl_x + size_sl_w * size_frac;
                ui.rounded(
                    size_thumb_x - 7.0,
                    size_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(size_thumb_x - 7.0, size_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                draw_toggle_row(
                    &mut ui,
                    body_y + 296.0,
                    "Gyroscope Aiming",
                    "Tilt device for fine camera and bow aiming",
                    Some(settings.gyroscope),
                    None,
                );
                let _ = gyro_spd_lbl;
                let gyro_y = body_y + 350.0;
                ui.rounded(cx, gyro_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, gyro_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    gyro_y + 6.0,
                    text_c,
                    "Gyroscope Sensitivity",
                );
                let gyro_pct = format!("{:.0}%", settings.gyro_sensitivity * 100.0);
                let gyro_pill_w = 48.0;
                ui.rounded(
                    cx + cw - gyro_pill_w - 12.0,
                    gyro_y + 6.0,
                    gyro_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - gyro_pill_w - 12.0,
                    gyro_y + 6.0,
                    gyro_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - gyro_pill_w - 12.0, gyro_y + 6.0, gyro_pill_w, 18.0],
                    accent_hi,
                    &gyro_pct,
                );
                let gyro_sl_x = cx + 14.0;
                let gyro_sl_w = (cw - 28.0).max(40.0);
                let gyro_sl_y = gyro_y + 33.0;
                let gyro_frac = ((settings.gyro_sensitivity - 0.10) / 2.90).clamp(0.0, 1.0);
                ui.rounded(gyro_sl_x, gyro_sl_y, gyro_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    gyro_sl_x,
                    gyro_sl_y,
                    (gyro_sl_w * gyro_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let gyro_thumb_x = gyro_sl_x + gyro_sl_w * gyro_frac;
                ui.rounded(
                    gyro_thumb_x - 7.0,
                    gyro_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(gyro_thumb_x - 7.0, gyro_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);
            }
            3 => {
                // Tab 3: Buttons & Layout Options
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "BUTTON SIZING, HANDEDNESS & BEHAVIOR",
                );
                let step = 54.0;
                let scale_y = body_y + 18.0;
                ui.rounded(cx, scale_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, scale_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    scale_y + 6.0,
                    text_c,
                    "Global Button Scale",
                );
                let sc_pct = format!("{:.0}%", settings.button_scale * 100.0);
                let sc_pill_w = 48.0;
                ui.rounded(
                    cx + cw - sc_pill_w - 12.0,
                    scale_y + 6.0,
                    sc_pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - sc_pill_w - 12.0,
                    scale_y + 6.0,
                    sc_pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - sc_pill_w - 12.0, scale_y + 6.0, sc_pill_w, 18.0],
                    accent_hi,
                    &sc_pct,
                );
                let sc_sl_x = cx + 14.0;
                let sc_sl_w = (cw - 28.0).max(40.0);
                let sc_sl_y = scale_y + 33.0;
                let sc_frac = ((settings.button_scale - 0.50) / 1.50).clamp(0.0, 1.0);
                ui.rounded(sc_sl_x, sc_sl_y, sc_sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(
                    sc_sl_x,
                    sc_sl_y,
                    (sc_sl_w * sc_frac).max(6.0),
                    6.0,
                    3.0,
                    accent,
                );
                let sc_thumb_x = sc_sl_x + sc_sl_w * sc_frac;
                ui.rounded(
                    sc_thumb_x - 7.0,
                    sc_sl_y - 4.0,
                    14.0,
                    14.0,
                    7.0,
                    accent_hi,
                );
                ui.border(sc_thumb_x - 7.0, sc_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step,
                    "Left-Handed Layout",
                    "Mirror movement and action buttons horizontally",
                    Some(settings.left_handed),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 2.0,
                    "Toggle Sprint",
                    "Latch sprint on tap instead of holding",
                    Some(settings.sprint_toggle || settings.toggle_sprint),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 3.0,
                    "Inventory Walk Controls",
                    "Keep movement controls active while inventory is open",
                    Some(settings.inventory_controls),
                    None,
                );
            }
           4 => {
                // Tab 4: Advanced & Quick Actions
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "FEEDBACK, GAMEPLAY & SHORTCUTS",
                );
                let step = 54.0;
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0,
                    "Haptic Feedback",
                    "Vibrate on block break, damage and touch button presses",
                    Some(settings.vibrate),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step,
                    "Auto-Jump",
                    "Automatically jump when walking into 1-block ledges",
                    Some(settings.auto_jump),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 18.0 + step * 2.0,
                    "Apply Standard Menu Visibility",
                    "Reset in-game vs in-menu visibility flags for active system",
                    None,
                    None,
                );
                ui.text(
                    Face::SemiBold,
                    10.0,
                    cx,
                    body_y + 182.0,
                    muted_c,
                    "QUICK DESKTOP KEYS & CUSTOM INPUT",
                );
                let ky = body_y + 198.0;
                let kw = (cw - 25.0) / 6.0;
                for (i, lbl) in ["F1", "F3", "F5", "Inv", "Chat", "Keys"]
                    .into_iter()
                    .enumerate()
                {
                    let rx = cx + i as f32 * (kw + 5.0);
                    ui.rounded(rx, ky, kw, 36.0, 9.0, card_bg);
                    ui.border(rx, ky, kw, 36.0, 9.0, border_c);
                    ui.centered_fit(
                        Face::SemiBold,
                        10.5,
                        [rx + 2.0, ky, kw - 4.0, 36.0],
                        text_c,
                        lbl,
                    );
                }
                let send_y = body_y + 244.0;
                ui.rounded(cx, send_y, cw, 40.0, 10.0, card_sel);
                ui.border(cx, send_y, cw, 40.0, 10.0, accent);
                ui.icon(Icon::Gamepad, cx + 12.0, send_y + 11.0, 18.0, accent_hi);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 38.0,
                    send_y + 5.0,
                    text_c,
                    "Send Custom Input (Keyboard / Mouse / Action)",
                );
                ui.text(
                    Face::Regular,
                    8.8,
                    cx + 38.0,
                    send_y + 21.0,
                    muted_c,
                    "Open 104-key PC keyboard, mouse buttons, scroll wheel & actions",
                );
                ui.text(
                    Face::SemiBold,
                    13.5,
                    cx + cw - 18.0,
                    send_y + 11.0,
                    accent_hi,
                    "›",
                );

                let sys_y = body_y + 292.0;
                let half = (cw - 10.0) * 0.5;
                ui.rounded(
                    cx,
                    sys_y,
                    half,
                    38.0,
                    10.0,
                    color::with_alpha(danger_c, 0.22),
                );
                ui.border(cx, sys_y, half, 38.0, 10.0, danger_c);
                ui.centered_fit(
                    Face::SemiBold,
                    10.8,
                    [cx, sys_y, half, 38.0],
                    danger_c,
                    "Force Kill",
                );
                ui.rounded(cx + half + 10.0, sys_y, half, 38.0, 10.0, card_bg);
                ui.border(cx + half + 10.0, sys_y, half, 38.0, 10.0, border_c);
                ui.centered_fit(
                    Face::SemiBold,
                    10.8,
                    [cx + half + 10.0, sys_y, half, 38.0],
                    text_c,
                    "Open Logs",
                );
            }
            5 => {
                // Tab 5: Record — the screen recorder, its options and the last recordings.
                let status = recorder::status();
                let settings_rec = recorder::settings();
                let cells = record_cells(cx, cw, body_y);
                let recording = status.phase == Phase::Recording;
                let card_y = body_y + 12.0;
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "SCREEN RECORDER",
                );
                ui.rounded(cx, card_y, cw, 56.0, 12.0, card_bg);
                ui.border(cx, card_y, cw, 56.0, 12.0, border_c);
                // A thin copper edge, red while it records: the launcher dashboard's rule.
                ui.rounded(
                    cx,
                    card_y,
                    3.0,
                    56.0,
                    1.5,
                    if recording { danger_c } else { accent },
                );
                let pulse = if recording {
                    // A slow breathing dot, not a per-frame effect.
                    let t = crate::ui_theme::motion::ambient_time();
                    0.55 + 0.45 * (t * 2.2).sin().abs()
                } else {
                    1.0
                };
                let dot = if recording {
                    color::with_alpha(danger_c, pulse)
                } else if status.phase == Phase::Error || status.phase == Phase::Unsupported {
                    danger_c
                } else {
                    accent_hi
                };
                ui.rounded(cx + 16.0, card_y + 20.0, 9.0, 9.0, 4.5, dot);
                let headline = if recording || status.phase == Phase::Starting {
                    format!(
                        "{}  {}",
                        status.phase.label(),
                        recorder::format_duration(status.millis)
                    )
                } else {
                    status.phase.label().to_owned()
                };
                ui.text(Face::SemiBold, 11.0, cx + 32.0, card_y + 9.0, text_c, &headline);
                let detail = if recording && status.width > 0 {
                    format!(
                        "{}x{}  •  {} fps  •  {}",
                        status.width,
                        status.height,
                        status.fps,
                        settings_rec.audio_label()
                    )
                } else if !status.message.is_empty() {
                    status.message.clone()
                } else if status.capability.checked {
                    format!(
                        "{}  •  display {}x{}",
                        status.capability.device, status.capability.display.0, status.capability.display.1
                    )
                } else {
                    "Checking what this device can capture…".to_owned()
                };
                ui.text(
                    Face::Regular,
                    8.6,
                    cx + 32.0,
                    card_y + 27.0,
                    muted_c,
                    &ellipsis(&detail, ((cw - 140.0) / 4.6).max(8.0) as usize),
                );

                // Start / Stop, in the same plate for either state.
                let busy = !status.phase.startable();
                let start_fill = if busy {
                    color::with_alpha(danger_c, 0.85)
                } else {
                    accent
                };
                ui.rounded(
                    cells.start[0],
                    cells.start[1],
                    cells.start[2],
                    cells.start[3],
                    9.0,
                    start_fill,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    10.6,
                    cells.start,
                    accent_fg,
                    if busy { "Stop" } else { "Start" },
                );

                // The three lists of choices: a label over a grid of chips.
                let chip = |ui: &mut crate::ui_theme::Ui<'_>,
                            cell: [f32; 4],
                            label: &str,
                            selected: bool,
                            enabled: bool| {
                    ui.rounded(cell[0], cell[1], cell[2], cell[3], 8.0, if selected {
                        card_sel
                    } else {
                        sidebar_bg
                    });
                    ui.stroke_rounded(
                        cell[0],
                        cell[1],
                        cell[2],
                        cell[3],
                        8.0,
                        if selected { 1.5 } else { 1.0 },
                        if selected {
                            accent
                        } else {
                            border_c
                        },
                    );
                    ui.centered_fit(
                        if selected { Face::SemiBold } else { Face::Medium },
                        10.0,
                        cell,
                        if !enabled {
                            muted_c
                        } else if selected {
                            accent_hi
                        } else {
                            text_c
                        },
                        label,
                    );
                };
                ui.text(Face::SemiBold, 9.2, cx, body_y + 64.0, muted_c, "RESOLUTION");
                for (i, (label, _)) in recorder::RESOLUTIONS.iter().enumerate() {
                    chip(
                        &mut ui,
                        cells.resolution[i],
                        *label,
                        settings_rec.resolution == i,
                        true,
                    );
                }
                ui.text(Face::SemiBold, 9.2, cx, body_y + 146.0, muted_c, "FRAME RATE");
                for (i, rate) in recorder::FRAME_RATES.iter().enumerate() {
                    chip(
                        &mut ui,
                        cells.fps[i],
                        &format!("{rate} fps"),
                        settings_rec.fps == i,
                        true,
                    );
                }
                ui.text(Face::SemiBold, 9.2, cx, body_y + 228.0, muted_c, "AUDIO");
                for (i, label) in recorder::AUDIO.iter().enumerate() {
                    let label = *label;
                    // In-game audio needs Android 10's playback capture; the chip says so
                    // rather than failing after the player picks it.
                    let enabled = i != 2 || status.capability.game_audio;
                    chip(
                        &mut ui,
                        cells.audio[i],
                        label,
                        settings_rec.audio == i,
                        enabled,
                    );
                }
                if status.capability.checked && !status.capability.game_audio {
                    ui.text(
                        Face::Regular,
                        8.4,
                        cx,
                        body_y + 272.0,
                        muted_c,
                        "In-game audio needs Android 10 or newer.",
                    );
                }
                draw_toggle_row(
                    &mut ui,
                    cells.overlay[1],
                    "Record With Controls",
                    "On records gameplay + controls; off records raw gameplay only",
                    Some(settings_rec.record_controls),
                    None,
                );

                ui.text(
                    Face::SemiBold,
                    9.2,
                    cx,
                    body_y + 338.0,
                    muted_c,
                    "RECORDINGS",
                );
                ui.text(
                    Face::Regular,
                    8.4,
                    cx + cw - 120.0,
                    body_y + 339.0,
                    muted_c,
                    "Recordings",
                );
                let listed = &status.recordings;
                if listed.is_empty() {
                    ui.text(
                        Face::Regular,
                        8.8,
                        cx,
                        body_y + 358.0,
                        muted_c,
                        "Finished recordings appear here, saved as MP4 in the Recordings folder.",
                    );
                }
                for (i, cell) in cells.recordings.into_iter().enumerate() {
                    let Some(item) = listed.get(i) else {
                        break;
                    };
                    ui.rounded(cell[0], cell[1], cell[2], cell[3], 9.0, card_bg);
                    ui.border(cell[0], cell[1], cell[2], cell[3], 9.0, border_c);
                    ui.icon(Icon::Play, cell[0] + 10.0, cell[1] + 9.0, 16.0, accent_hi);
                    let name = ellipsis(&item.name, ((cw - 150.0) / 5.2).max(8.0) as usize);
                    ui.text(Face::Medium, 9.6, cell[0] + 32.0, cell[1] + 6.0, text_c, &name);
                    let meta = format!(
                        "{}  •  {}",
                        recorder::format_bytes(item.bytes),
                        item.uri.starts_with("file:")
                            .then_some("needs Android 10 to play")
                            .unwrap_or("tap to play")
                    );
                    ui.text(
                        Face::Regular,
                        8.2,
                        cell[0] + 32.0,
                        cell[1] + 20.0,
                        muted_c,
                        &meta,
                    );
                }
            }
            6 => {
                replay_hud::draw_control_hub_replay_tab(&mut ui, cx, cw, body_y);
            }
            7 => {
                // Tab 7: Overlay Menu — size, opacity & visibility rules for the V+ button.
                ui.text(
                    Face::SemiBold,
                    10.2,
                    cx,
                    body_y,
                    muted_c,
                    "OVERLAY MENU (V+ BUTTON)",
                );
                let sz_y = body_y + 18.0;
                ui.rounded(cx, sz_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, sz_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    sz_y + 6.0,
                    text_c,
                    "Overlay Button Size",
                );
                let ov_sz_pct = format!("{:.0}%", settings.overlay_scale.clamp(0.5, 2.0) * 100.0);
                let pill_w = 48.0;
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    sz_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    sz_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, sz_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &ov_sz_pct,
                );
                let sl_x = cx + 14.0;
                let sl_w = (cw - 28.0).max(40.0);
                let sz_sl_y = sz_y + 33.0;
                let sz_frac = ((settings.overlay_scale - 0.50) / 1.50).clamp(0.0, 1.0);
                ui.rounded(sl_x, sz_sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, sz_sl_y, (sl_w * sz_frac).max(6.0), 6.0, 3.0, accent);
                let sz_thumb = sl_x + sl_w * sz_frac;
                ui.rounded(sz_thumb - 7.0, sz_sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(sz_thumb - 7.0, sz_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                let op_y = body_y + 76.0;
                ui.rounded(cx, op_y, cw, 50.0, 11.0, card_bg);
                ui.border(cx, op_y, cw, 50.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.8,
                    cx + 14.0,
                    op_y + 6.0,
                    text_c,
                    "Overlay Button Opacity",
                );
                let ov_op_pct = format!("{:.0}%", settings.overlay_opacity.clamp(0.20, 1.0) * 100.0);
                ui.rounded(
                    cx + cw - pill_w - 12.0,
                    op_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    cx + cw - pill_w - 12.0,
                    op_y + 6.0,
                    pill_w,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.4,
                    [cx + cw - pill_w - 12.0, op_y + 6.0, pill_w, 18.0],
                    accent_hi,
                    &ov_op_pct,
                );
                let op_sl_y = op_y + 33.0;
                let op_frac = ((settings.overlay_opacity - 0.20) / 0.80).clamp(0.0, 1.0);
                ui.rounded(sl_x, op_sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, op_sl_y, (sl_w * op_frac).max(6.0), 6.0, 3.0, accent);
                let op_thumb = sl_x + sl_w * op_frac;
                ui.rounded(op_thumb - 7.0, op_sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(op_thumb - 7.0, op_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                let step = 54.0;
                draw_toggle_row(
                    &mut ui,
                    body_y + 134.0,
                    "Hide V+ Button Completely",
                    "Hide the V+ trigger for this session (restores automatically on relaunch)",
                    Some(settings.overlay_hidden),
                    None,
                );
                draw_toggle_row(
                    &mut ui,
                    body_y + 134.0 + step,
                    "Show Controls on Menu Screen",
                    "Show your touch & JVM controls on all screens (Inventory, Pause, Options, Title)",
                    Some(settings.overlay_show_controls_in_menus),
                    None,
                );
            }
            8 => {
                // Tab 8: Internet Music Player (Now Playing, Fast-Forward, Volume, Playlists & Vanilla Toggle)
                use crate::internet_music::{self, format_duration, format_ms};
                let snap = internet_music::snapshot();
                let vanilla_off = internet_music::is_vanilla_music_disabled();

                ui.text(Face::SemiBold, 10.2, cx, body_y, muted_c, "NOW PLAYING");
                let np_y = body_y + 18.0;
                ui.rounded(cx, np_y, cw, 106.0, 12.0, card_bg);
                ui.border(cx, np_y, cw, 106.0, 12.0, accent);

                let np_title = if snap.has_track() {
                    snap.current_title.as_str()
                } else {
                    "No song playing"
                };
                let np_sub = if snap.has_track() {
                    if !snap.status.is_empty() {
                        snap.status.clone()
                    } else if snap.buffering {
                        "Buffering audio stream...".to_owned()
                    } else if !snap.active_playlist.is_empty() {
                        format!("{}  •  Playlist: {}", snap.current_artist, snap.active_playlist)
                    } else {
                        snap.current_artist.clone()
                    }
                } else {
                    "Pick a playlist below or browse in Launcher -> Musics".to_owned()
                };
                let time_txt = if snap.duration_ms > 0 {
                    format!("{} / {}", format_ms(snap.position_ms), format_ms(snap.duration_ms))
                } else {
                    format_ms(snap.position_ms)
                };
                let np_t_fit = ellipsis(np_title, ((cw - 135.0) / 5.6).max(10.0) as usize);
                let np_s_fit = ellipsis(&np_sub, ((cw - 56.0) / 4.8).max(12.0) as usize);
                let np_art_drawn = if snap.has_track()
                    && let Some((art_id, aw, ah, rgba)) =
                        internet_music::find_track_icon_by_url(&snap.current_url)
                {
                    ui.image(&art_id, aw, ah, &rgba, cx + 10.0, np_y + 8.0, 32.0, 32.0)
                } else {
                    false
                };
                if !np_art_drawn {
                    ui.rounded(cx + 10.0, np_y + 8.0, 32.0, 32.0, 8.0, sidebar_bg);
                    ui.icon(Icon::Music, cx + 18.0, np_y + 16.0, 16.0, accent_hi);
                }
                ui.text(Face::SemiBold, 11.2, cx + 48.0, np_y + 8.0, text_c, &np_t_fit);
                ui.text(Face::Medium, 9.2, cx + cw - 86.0, np_y + 10.0, muted_c, &time_txt);
                ui.text(Face::Regular, 9.0, cx + 48.0, np_y + 25.0, muted_c, &np_s_fit);

                // Seek Bar
                let seek_x = cx + 12.0;
                let seek_w = (cw - 24.0).max(40.0);
                let seek_y = np_y + 46.0;
                let seek_frac = if snap.duration_ms > 0 {
                    (snap.position_ms as f32 / snap.duration_ms as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                ui.rounded(seek_x, seek_y, seek_w, 6.0, 3.0, sidebar_bg);
                if seek_frac > 0.0 {
                    ui.rounded(seek_x, seek_y, (seek_w * seek_frac).max(6.0), 6.0, 3.0, accent);
                }

                // Prev | Play/Pause | Fast Fwd +10s | Next
                let gap = 6.0;
                let bw = ((cw - 24.0 - gap * 3.0) / 4.0).max(40.0);
                let btn_y = np_y + 66.0;
                let play_lbl = if snap.playing { "Pause" } else { "Play" };
                for (bi, (lbl, hi)) in [
                    ("Prev", false),
                    (play_lbl, true),
                    ("Fwd +10s", false),
                    ("Next", false),
                ]
                .into_iter()
                .enumerate()
                {
                    let bx = cx + 12.0 + bi as f32 * (bw + gap);
                    ui.rounded(
                        bx,
                        btn_y,
                        bw,
                        30.0,
                        8.0,
                        if hi { accent } else { card_sel },
                    );
                    ui.border(bx, btn_y, bw, 30.0, 8.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.8,
                        [bx, btn_y, bw, 30.0],
                        if hi { accent_fg } else { text_c },
                        lbl,
                    );
                }

                // Volume Button + Slider
                let vol_y = body_y + 132.0;
                ui.rounded(
                    cx,
                    vol_y,
                    72.0,
                    48.0,
                    11.0,
                    if snap.muted { card_sel } else { card_bg },
                );
                ui.border(
                    cx,
                    vol_y,
                    72.0,
                    48.0,
                    11.0,
                    if snap.muted { accent } else { border_c },
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.8,
                    [cx, vol_y, 72.0, 48.0],
                    if snap.muted { accent_hi } else { text_c },
                    if snap.muted { "Unmute" } else { "Mute" },
                );

                let vol_card_x = cx + 78.0;
                let vol_card_w = (cw - 78.0).max(80.0);
                ui.rounded(vol_card_x, vol_y, vol_card_w, 48.0, 11.0, card_bg);
                ui.border(vol_card_x, vol_y, vol_card_w, 48.0, 11.0, border_c);
                ui.text(
                    Face::SemiBold,
                    10.4,
                    vol_card_x + 14.0,
                    vol_y + 6.0,
                    text_c,
                    "Music Volume",
                );
                let vol_pct = if snap.muted {
                    "Muted".to_owned()
                } else {
                    format!("{:.0}%", snap.volume.clamp(0.0, 1.0) * 100.0)
                };
                ui.rounded(
                    vol_card_x + vol_card_w - 56.0,
                    vol_y + 6.0,
                    46.0,
                    18.0,
                    6.0,
                    card_sel,
                );
                ui.border(
                    vol_card_x + vol_card_w - 56.0,
                    vol_y + 6.0,
                    46.0,
                    18.0,
                    6.0,
                    accent,
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.0,
                    [vol_card_x + vol_card_w - 56.0, vol_y + 6.0, 46.0, 18.0],
                    accent_hi,
                    &vol_pct,
                );
                let sl_x = cx + 92.0;
                let sl_w = (cw - 106.0).max(40.0);
                let v_sl_y = vol_y + 32.0;
                let vf = snap.volume.clamp(0.0, 1.0);
                ui.rounded(sl_x, v_sl_y, sl_w, 6.0, 3.0, sidebar_bg);
                ui.rounded(sl_x, v_sl_y, (sl_w * vf).max(6.0), 6.0, 3.0, accent);
                let v_thumb = sl_x + sl_w * vf;
                ui.rounded(v_thumb - 7.0, v_sl_y - 4.0, 14.0, 14.0, 7.0, accent_hi);
                ui.border(v_thumb - 7.0, v_sl_y - 4.0, 14.0, 14.0, 7.0, text_c);

                // Toggle Off Vanilla Minecraft Background Music
                draw_toggle_row(
                    &mut ui,
                    body_y + 188.0,
                    "Turn Off Vanilla Background Music",
                    "Mute Minecraft's built-in music while listening to your songs",
                    Some(vanilla_off),
                    None,
                );

                // Mode Switcher: Playlists | Search Online
                let mode_y = body_y + 242.0;
                let half_w = ((cw - 8.0) * 0.5).max(60.0);
                for (mi, (mlbl, active)) in [
                    ("Playlists", !self.music_search_tab),
                    ("Search Online", self.music_search_tab),
                ]
                .into_iter()
                .enumerate()
                {
                    let mx = cx + mi as f32 * (half_w + 8.0);
                    ui.rounded(
                        mx,
                        mode_y,
                        half_w,
                        32.0,
                        10.0,
                        if active { accent } else { card_bg },
                    );
                    ui.border(
                        mx,
                        mode_y,
                        half_w,
                        32.0,
                        10.0,
                        if active { accent_hi } else { border_c },
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        10.0,
                        [mx, mode_y, half_w, 32.0],
                        if active { accent_fg } else { text_c },
                        mlbl,
                    );
                }

                let sub_y = body_y + 284.0;
                if !self.music_search_tab {
                    // Choose Playlist
                    ui.text(Face::SemiBold, 10.2, cx, sub_y, muted_c, "PLAYLISTS");
                    let pl_start_y = sub_y + 18.0;
                    if music_playlists.is_empty() {
                        ui.text(
                            Face::Regular,
                            9.6,
                            cx,
                            pl_start_y + 6.0,
                            muted_c,
                            "No playlists yet.",
                        );
                    }
                    for (pi, pl) in music_playlists.iter().enumerate() {
                        let ry = pl_start_y + pi as f32 * 42.0;
                        let selected = pi == music_sel_idx;
                        let playing_pl = snap.has_track() && snap.active_playlist == pl.name;
                        ui.rounded(
                            cx,
                            ry,
                            cw,
                            38.0,
                            10.0,
                            if selected { card_sel } else { card_bg },
                        );
                        ui.border(
                            cx,
                            ry,
                            cw,
                            38.0,
                            10.0,
                            if selected || playing_pl { accent } else { border_c },
                        );
                        let p_name = ellipsis(&pl.name, ((cw - 140.0) / 5.4).max(8.0) as usize);
                        ui.text(Face::SemiBold, 10.4, cx + 12.0, ry + 6.0, text_c, &p_name);
                        ui.text(
                            Face::Regular,
                            8.8,
                            cx + 12.0,
                            ry + 21.0,
                            muted_c,
                            &format!("{} songs", pl.tracks.len()),
                        );
                        let play_r = [cx + cw - 78.0, ry + 5.0, 68.0, 28.0];
                        ui.rounded(
                            play_r[0],
                            play_r[1],
                            play_r[2],
                            play_r[3],
                            8.0,
                            if playing_pl { accent } else { sidebar_bg },
                        );
                        ui.border(play_r[0], play_r[1], play_r[2], play_r[3], 8.0, accent);
                        ui.centered_fit(
                            Face::SemiBold,
                            9.4,
                            play_r,
                            if playing_pl { accent_fg } else { text_c },
                            if playing_pl { "Active" } else { "Play" },
                        );
                    }

                    // Check Songs in Selected Playlist (with Album Art!)
                    let tracks_hdr_y =
                        pl_start_y + music_playlists.len().max(1) as f32 * 42.0 + 10.0;
                    let sel_pl_name = music_playlists
                        .get(music_sel_idx)
                        .map_or("Playlist", |p| p.name.as_str());
                    ui.text(
                        Face::SemiBold,
                        10.2,
                        cx,
                        tracks_hdr_y,
                        muted_c,
                        &format!("SONGS IN \"{}\"", sel_pl_name.to_uppercase()),
                    );
                    let tracks_start_y = tracks_hdr_y + 20.0;
                    if let Some(pl) = music_playlists.get(music_sel_idx) {
                        if pl.tracks.is_empty() {
                            ui.text(
                                Face::Regular,
                                9.6,
                                cx,
                                tracks_start_y + 4.0,
                                muted_c,
                                "Empty playlist. Use Search Online or Launcher -> Musics!",
                            );
                        }
                        for (ti, tr) in pl.tracks.iter().enumerate() {
                            let ty = tracks_start_y + ti as f32 * 42.0;
                            let is_cur = snap.has_track() && snap.current_url == tr.stream_url;
                            ui.rounded(
                                cx,
                                ty,
                                cw,
                                38.0,
                                9.0,
                                if is_cur { card_sel } else { card_bg },
                            );
                            ui.border(
                                cx,
                                ty,
                                cw,
                                38.0,
                                9.0,
                                if is_cur { accent } else { border_c },
                            );
                            let drawn = if let Some((iw, ih, rgba)) =
                                internet_music::get_or_request_icon(&tr.id, tr.icon_url.as_deref())
                            {
                                ui.image(&tr.id, iw, ih, &rgba, cx + 6.0, ty + 4.0, 30.0, 30.0)
                            } else {
                                false
                            };
                            if !drawn {
                                ui.rounded(cx + 6.0, ty + 4.0, 30.0, 30.0, 7.0, sidebar_bg);
                                ui.icon(Icon::Music, cx + 13.0, ty + 11.0, 16.0, accent_hi);
                            }
                            let t_fit = ellipsis(&tr.title, ((cw - 118.0) / 5.2).max(8.0) as usize);
                            ui.text(Face::Medium, 9.8, cx + 42.0, ty + 6.0, text_c, &t_fit);
                            ui.text(
                                Face::Regular,
                                8.4,
                                cx + 42.0,
                                ty + 20.0,
                                muted_c,
                                &ellipsis(&tr.artist, ((cw - 118.0) / 4.6).max(8.0) as usize),
                            );
                            ui.centered_fit(
                                Face::Medium,
                                9.0,
                                [cx + cw - 54.0, ty + 8.0, 44.0, 22.0],
                                muted_c,
                                &if tr.duration_secs > 0 {
                                    format_duration(tr.duration_secs)
                                } else if !tr.stream_url.starts_with("https://") {
                                    "Offline".to_owned()
                                } else {
                                    "Stream".to_owned()
                                },
                            );
                        }
                    }
                } else {
                    // Search Online Sub-Tab (OSK input, genre chips, sort, results with logos & download, pagination)
                    if !music_search_snap.fetched_initial && !music_search_snap.searching {
                        internet_music::trigger_in_game_search(
                            music_search_snap.query.clone(),
                            music_search_snap.genre.clone(),
                            music_search_snap.sort,
                            0,
                        );
                    }
                    let search_btn_w = 72.0;
                    let field_w = (cw - search_btn_w - 8.0).max(80.0);
                    ui.rounded(cx, sub_y, field_w, 34.0, 10.0, sidebar_bg);
                    ui.border(
                        cx,
                        sub_y,
                        field_w,
                        34.0,
                        10.0,
                        if self.music_editing_search {
                            accent_hi
                        } else {
                            border_c
                        },
                    );
                    let q_disp = if music_search_snap.query.is_empty() {
                        "Tap to search songs..."
                    } else {
                        music_search_snap.query.as_str()
                    };
                    let q_col = if music_search_snap.query.is_empty() {
                        muted_c
                    } else {
                        text_c
                    };
                    ui.text(
                        Face::Regular,
                        9.8,
                        cx + 10.0,
                        sub_y + 10.0,
                        q_col,
                        &ellipsis(q_disp, ((field_w - 20.0) / 5.2).max(10.0) as usize),
                    );
                    let sbx = cx + field_w + 8.0;
                    ui.rounded(sbx, sub_y, search_btn_w, 34.0, 10.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.6,
                        [sbx, sub_y, search_btn_w, 34.0],
                        accent_fg,
                        if music_search_snap.searching { "..." } else { "Search" },
                    );

                    // Genre Chips + Sort Cycle Button
                    let chip_y = sub_y + 40.0;
                    let sort_w = 88.0;
                    let genres: [(&str, Option<&str>); 4] = [
                        ("Trending", None),
                        ("Lo-Fi", Some("Lo-Fi")),
                        ("Chill", Some("Chill")),
                        ("Synth", Some("Synthwave")),
                    ];
                    let chips_w = (cw - sort_w - 8.0).max(120.0);
                    let chip_gap = 4.0;
                    let chip_w = ((chips_w - chip_gap * 3.0) / 4.0).max(36.0);
                    for (gi, (glbl, gval)) in genres.iter().enumerate() {
                        let gx = cx + gi as f32 * (chip_w + chip_gap);
                        let active = match gval {
                            None => music_search_snap.genre.is_none(),
                            Some(g) => music_search_snap
                                .genre
                                .as_deref()
                                .is_some_and(|cur| cur.eq_ignore_ascii_case(g)),
                        };
                        ui.rounded(
                            gx,
                            chip_y,
                            chip_w,
                            28.0,
                            8.0,
                            if active { accent } else { card_bg },
                        );
                        ui.border(
                            gx,
                            chip_y,
                            chip_w,
                            28.0,
                            8.0,
                            if active { accent_hi } else { border_c },
                        );
                        ui.centered_fit(
                            Face::SemiBold,
                            8.6,
                            [gx, chip_y, chip_w, 28.0],
                            if active { accent_fg } else { text_c },
                            glbl,
                        );
                    }
                    let sx = cx + cw - sort_w;
                    ui.rounded(sx, chip_y, sort_w, 28.0, 8.0, card_sel);
                    ui.border(sx, chip_y, sort_w, 28.0, 8.0, accent);
                    ui.centered_fit(
                        Face::SemiBold,
                        8.4,
                        [sx, chip_y, sort_w, 28.0],
                        accent_hi,
                        music_search_snap.sort.label(),
                    );

                    let list_y = sub_y + 76.0;
                    if music_search_snap.results.is_empty() {
                        let empty_txt = if music_search_snap.searching {
                            "Searching internet music..."
                        } else if !music_search_snap.status.is_empty() {
                            music_search_snap.status.as_str()
                        } else {
                            "No songs found."
                        };
                        ui.text(Face::Regular, 9.6, cx, list_y + 6.0, muted_c, empty_txt);
                    }
                    let downloads = internet_music::active_downloads();
                    for (idx, tr) in music_search_snap.results.iter().enumerate() {
                        let ry = list_y + idx as f32 * 50.0;
                        let is_cur = snap.has_track() && snap.current_url == tr.stream_url;
                        let is_dl = internet_music::is_track_downloaded(tr);
                        let is_dl_active = downloads.iter().any(|d| d.id == tr.id && !d.done);
                        ui.rounded(
                            cx,
                            ry,
                            cw,
                            44.0,
                            10.0,
                            if is_cur { card_sel } else { card_bg },
                        );
                        ui.border(
                            cx,
                            ry,
                            cw,
                            44.0,
                            10.0,
                            if is_cur { accent } else { border_c },
                        );
                        let drawn = if let Some((iw, ih, rgba)) =
                            internet_music::get_or_request_icon(&tr.id, tr.icon_url.as_deref())
                        {
                            ui.image(&tr.id, iw, ih, &rgba, cx + 6.0, ry + 5.0, 34.0, 34.0)
                        } else {
                            false
                        };
                        if !drawn {
                            ui.rounded(cx + 6.0, ry + 5.0, 34.0, 34.0, 8.0, sidebar_bg);
                            ui.icon(Icon::Music, cx + 14.0, ry + 13.0, 18.0, accent_hi);
                        }
                        let play_r = [cx + cw - 58.0, ry + 7.0, 52.0, 30.0];
                        let dl_r = [play_r[0] - 36.0, ry + 7.0, 30.0, 30.0];
                        let text_x = cx + 46.0;
                        let max_chars = ((dl_r[0] - text_x - 6.0) / 5.1).max(8.0) as usize;
                        ui.text(
                            Face::SemiBold,
                            9.8,
                            text_x,
                            ry + 7.0,
                            text_c,
                            &ellipsis(&tr.title, max_chars),
                        );
                        let sub_line = if is_dl_active {
                            format!("{} • Downloading...", tr.artist)
                        } else if is_dl {
                            format!("{} • Downloaded", tr.artist)
                        } else {
                            format!("{} • {}", tr.artist, tr.source)
                        };
                        ui.text(
                            Face::Regular,
                            8.4,
                            text_x,
                            ry + 23.0,
                            muted_c,
                            &ellipsis(&sub_line, max_chars + 4),
                        );

                        ui.rounded(
                            dl_r[0],
                            dl_r[1],
                            dl_r[2],
                            dl_r[3],
                            8.0,
                            if is_dl || is_dl_active { card_sel } else { sidebar_bg },
                        );
                        ui.border(
                            dl_r[0],
                            dl_r[1],
                            dl_r[2],
                            dl_r[3],
                            8.0,
                            if is_dl || is_dl_active { accent } else { border_c },
                        );
                        ui.icon(
                            if is_dl { Icon::Check } else { Icon::Download },
                            dl_r[0] + 7.0,
                            dl_r[1] + 7.0,
                            16.0,
                            if is_dl || is_dl_active { accent_hi } else { text_c },
                        );

                        ui.rounded(
                            play_r[0],
                            play_r[1],
                            play_r[2],
                            play_r[3],
                            8.0,
                            if is_cur { accent } else { sidebar_bg },
                        );
                        ui.border(play_r[0], play_r[1], play_r[2], play_r[3], 8.0, accent);
                        ui.centered_fit(
                            Face::SemiBold,
                            9.0,
                            play_r,
                            if is_cur { accent_fg } else { text_c },
                            if is_cur && snap.playing { "Playing" } else { "Play" },
                        );
                    }

                    // Bottom-middle Pagination (Previous | Page N | Next)
                    let pager_y =
                        list_y + music_search_snap.results.len().max(1) as f32 * 50.0 + 6.0;
                    let btn_w = 84.0;
                    let page_w = 68.0;
                    let pager_total = btn_w * 2.0 + page_w + 12.0;
                    let px = cx + (cw - pager_total).max(0.0) * 0.5;
                    let prev_ok = music_search_snap.page > 0;
                    ui.rounded(
                        px,
                        pager_y,
                        btn_w,
                        30.0,
                        8.0,
                        if prev_ok { card_sel } else { sidebar_bg },
                    );
                    ui.border(
                        px,
                        pager_y,
                        btn_w,
                        30.0,
                        8.0,
                        if prev_ok { accent } else { border_c },
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [px, pager_y, btn_w, 30.0],
                        if prev_ok { text_c } else { muted_c },
                        "Previous",
                    );
                    ui.rounded(px + btn_w + 6.0, pager_y, page_w, 30.0, 8.0, card_bg);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [px + btn_w + 6.0, pager_y, page_w, 30.0],
                        text_c,
                        &format!("Page {}", music_search_snap.page + 1),
                    );
                    let nx = px + btn_w + page_w + 12.0;
                    let next_ok = music_search_snap.has_next;
                    ui.rounded(
                        nx,
                        pager_y,
                        btn_w,
                        30.0,
                        8.0,
                        if next_ok { accent } else { sidebar_bg },
                    );
                    ui.border(
                        nx,
                        pager_y,
                        btn_w,
                        30.0,
                        8.0,
                        if next_ok { accent_hi } else { border_c },
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [nx, pager_y, btn_w, 30.0],
                        if next_ok { accent_fg } else { muted_c },
                        "Next",
                    );
                }
            }
            9 => {
                let draw_volt_mod_emblem = |ui: &mut Ui<'_>, idx: u8, ix: f32, iy: f32| {
                    let meta = &crate::extras::VOLT_MODS[usize::from(idx).min(crate::extras::VOLT_MOD_COUNT - 1)];
                    let icon_url = volt_extras_modrinth_icon_url(meta.key);
                    let cache_id = format!("volt_extras_{}", meta.key);
                    let drawn = if let Some((iw, ih, rgba_bytes)) =
                        crate::internet_music::get_or_request_icon(&cache_id, icon_url.as_deref())
                    {
                        ui.image(&cache_id, iw, ih, &rgba_bytes, ix, iy, 40.0, 40.0)
                    } else {
                        false
                    };
                    if !drawn {
                        ui.rounded(ix, iy, 40.0, 40.0, 9.0, sidebar_bg);
                        ui.border(ix, iy, 40.0, 40.0, 9.0, accent);
                        ui.centered_fit(
                            Face::SemiBold,
                            13.0,
                            [ix, iy, 40.0, 40.0],
                            text_c,
                            meta.badge,
                        );
                    }
                };

                if let Some(mod_idx) = self.volt_extras_config_mod {
                    let meta = &crate::extras::VOLT_MODS[usize::from(mod_idx).min(crate::extras::VOLT_MOD_COUNT - 1)];
                    let enabled = crate::extras::volt_mod_enabled(&self.volt_extras, mod_idx);
                    let back_w = (cw - 60.0).max(120.0);
                    ui.rounded(cx, body_y, back_w, 32.0, 9.0, card_bg);
                    ui.border(cx, body_y, back_w, 32.0, 9.0, border_c);
                    ui.icon(Icon::ArrowLeft, cx + 10.0, body_y + 9.0, 14.0, accent_hi);
                    ui.text_clipped(
                        Face::SemiBold,
                        10.4,
                        cx + 30.0,
                        body_y + 9.0,
                        (back_w - 38.0).max(40.0),
                        text_c,
                        &format!("Back  •  {}", meta.title),
                    );
                    let sw_x = cx + cw - 52.0;
                    let sw_y = body_y + 5.0;
                    ui.rounded(
                        sw_x,
                        sw_y,
                        40.0,
                        22.0,
                        11.0,
                        if enabled { accent } else { well_bg },
                    );
                    ui.border(
                        sw_x,
                        sw_y,
                        40.0,
                        22.0,
                        11.0,
                        if enabled { accent_hi } else { border_c },
                    );
                    ui.rounded(
                        if enabled { sw_x + 21.0 } else { sw_x + 3.0 },
                        sw_y + 3.0,
                        16.0,
                        16.0,
                        8.0,
                        accent_fg,
                    );

                    let rows = crate::volt_extras_ui::mod_setting_rows(&self.volt_extras, mod_idx);
                    for (r, row) in rows.into_iter().enumerate() {
                        let ry = body_y + 42.0 + r as f32 * 54.0;
                        if ry + 48.0 < body_top || ry > body_top + body_h {
                            continue;
                        }
                        let badge_override = if mod_idx == 83 && r == 0 {
                            self.volt_timelapse_status.as_deref()
                        } else {
                            None
                        };
                        draw_toggle_row(
                            &mut ui,
                            ry,
                            row.title,
                            row.sub,
                            row.toggle,
                            badge_override.or(row.badge.as_deref()),
                        );
                    }
                } else {
                    let active_cnt = crate::volt_extras_ui::active_mod_count(&self.volt_extras);
                    ui.text(
                        Face::SemiBold,
                        10.2,
                        cx,
                        body_y,
                        muted_c,
                        &format!("NEXORA MODS  •  {active_cnt} / {} ACTIVE", crate::extras::VOLT_MOD_COUNT),
                    );
                    ui.text(
                        Face::Regular,
                        8.6,
                        cx,
                        body_y + 14.0,
                        muted_c,
                        &format!(
                            "{} built-in client mods (all off by default — tap Configure to customize)",
                            crate::extras::VOLT_MOD_COUNT
                        ),
                    );

                    // Search bar + Enabled/Disabled filter & sort pills
                    let bar_y = body_y + 30.0;
                    let pill_gap = 3.0;
                    let pill_w = [34.0_f32, 52.0, 54.0, 42.0];
                    let pill_labels = ["All", "Enabled", "Disabled", "On↑"];
                    let right_total: f32 = pill_w.iter().sum::<f32>() + pill_gap * 3.0;
                    let search_w = (cw - right_total - 6.0).max(96.0);
                    ui.rounded(
                        cx,
                        bar_y,
                        search_w,
                        26.0,
                        8.0,
                        if self.volt_extras_search_focused {
                            card_sel
                        } else {
                            well_bg
                        },
                    );
                    ui.border(
                        cx,
                        bar_y,
                        search_w,
                        26.0,
                        8.0,
                        if self.volt_extras_search_focused {
                            accent_hi
                        } else {
                            border_c
                        },
                    );
                    let q_disp = if self.volt_extras_search.is_empty() {
                        "Search mods..."
                    } else {
                        self.volt_extras_search.as_str()
                    };
                    let q_col = if self.volt_extras_search.is_empty() {
                        muted_c
                    } else {
                        text_c
                    };
                    ui.text_clipped(
                        Face::Regular,
                        9.2,
                        cx + 9.0,
                        bar_y + 7.0,
                        (search_w - 26.0).max(30.0),
                        q_col,
                        q_disp,
                    );
                    if !self.volt_extras_search.is_empty() {
                        ui.centered_fit(
                            Face::SemiBold,
                            10.0,
                            [cx + search_w - 22.0, bar_y, 20.0, 26.0],
                            muted_c,
                            "×",
                        );
                    }
                    let mut px_cur = cx + search_w + 6.0;
                    for (fi, (&pw_i, &lbl)) in pill_w.iter().zip(pill_labels.iter()).enumerate() {
                        let sel = if fi < 3 {
                            self.volt_extras_filter == fi as u8
                        } else {
                            self.volt_extras_sort_enabled
                        };
                        ui.rounded(
                            px_cur,
                            bar_y,
                            pw_i,
                            26.0,
                            7.0,
                            if sel { accent } else { card_bg },
                        );
                        ui.border(
                            px_cur,
                            bar_y,
                            pw_i,
                            26.0,
                            7.0,
                            if sel { accent_hi } else { border_c },
                        );
                        ui.centered_fit(
                            if sel { Face::SemiBold } else { Face::Medium },
                            8.2,
                            [px_cur, bar_y, pw_i, 26.0],
                            if sel { accent_fg } else { text_c },
                            lbl,
                        );
                        px_cur += pw_i + pill_gap;
                    }

                    let cat_y = body_y + 62.0;
                    let cat_gap = 4.0;
                    let cat_w = ((cw - cat_gap * 5.0) / 6.0).max(36.0);
                    for (ci, cat) in crate::extras::VoltModCategory::TABS.iter().enumerate() {
                        let bx = cx + ci as f32 * (cat_w + cat_gap);
                        let sel = self.volt_extras_cat == ci as u8;
                        ui.rounded(bx, cat_y, cat_w, 22.0, 7.0, if sel { accent } else { card_bg });
                        ui.border(bx, cat_y, cat_w, 22.0, 7.0, if sel { accent_hi } else { border_c });
                        ui.centered_fit(
                            if sel { Face::SemiBold } else { Face::Medium },
                            8.6,
                            [bx, cat_y, cat_w, 22.0],
                            if sel { accent_fg } else { text_c },
                            cat.label(),
                        );
                    }

                    let filtered = crate::volt_extras_ui::filtered_mod_indices_with(
                        &self.volt_extras,
                        self.volt_extras_cat,
                        &self.volt_extras_search,
                        self.volt_extras_filter,
                        self.volt_extras_sort_enabled,
                    );
                    let list_y = body_y + 92.0;
                    if filtered.is_empty() {
                        ui.rounded(cx, list_y, cw, 48.0, 10.0, card_bg);
                        ui.border(cx, list_y, cw, 48.0, 10.0, border_c);
                        ui.centered_fit(
                            Face::Medium,
                            9.6,
                            [cx, list_y, cw, 48.0],
                            muted_c,
                            "No mods match this search / filter",
                        );
                    }
                    for (pos_i, &mod_idx) in filtered.iter().enumerate() {
                        let ry = list_y + pos_i as f32 * 68.0;
                        if ry + 64.0 < body_top || ry > body_top + body_h {
                            continue;
                        }
                        let meta = &crate::extras::VOLT_MODS[usize::from(mod_idx)];
                        let on = crate::extras::volt_mod_enabled(&self.volt_extras, mod_idx);
                        ui.rounded(cx, ry, cw, 62.0, 12.0, if on { card_sel } else { card_bg });
                        ui.border(cx, ry, cw, 62.0, 12.0, if on { accent } else { border_c });
                        draw_volt_mod_emblem(&mut ui, mod_idx, cx + 10.0, ry + 11.0);
                        let cfg_r = [cx + cw - 124.0, ry + 16.0, 64.0, 28.0];
                        let text_x = cx + 58.0;
                        let max_text_w = (cfg_r[0] - text_x - 8.0).max(40.0);
                        ui.text_clipped(Face::SemiBold, 10.4, text_x, ry + 10.0, max_text_w, text_c, meta.title);
                        ui.text_clipped(Face::Regular, 8.4, text_x, ry + 29.0, max_text_w, muted_c, meta.summary);

                        ui.rounded(cfg_r[0], cfg_r[1], cfg_r[2], cfg_r[3], 8.0, sidebar_bg);
                        ui.border(cfg_r[0], cfg_r[1], cfg_r[2], cfg_r[3], 8.0, border_c);
                        ui.centered_fit(Face::Medium, 9.0, cfg_r, text_c, "Configure");

                        let sw_x = cx + cw - 52.0;
                        let sw_y = ry + 19.0;
                        ui.rounded(
                            sw_x,
                            sw_y,
                            40.0,
                            22.0,
                            11.0,
                            if on { accent } else { well_bg },
                        );
                        ui.border(
                            sw_x,
                            sw_y,
                            40.0,
                            22.0,
                            11.0,
                            if on { accent_hi } else { border_c },
                        );
                        ui.rounded(
                            if on { sw_x + 21.0 } else { sw_x + 3.0 },
                            sw_y + 3.0,
                            16.0,
                            16.0,
                            8.0,
                            accent_fg,
                        );
                    }
                }
            }
            _ => {}
        }

        ui.set_clip(None);
        if max_scroll > 2.0 {
            let track_x = dx + dw - 8.0;
            let track_y = body_top + 4.0;
            let track_h = (body_h - 8.0).max(24.0);
            let thumb_h = (track_h * (body_h / content_height)).clamp(20.0, track_h);
            let thumb_y = track_y + (cur_scroll / max_scroll) * (track_h - thumb_h);
            ui.rounded(track_x, track_y, 3.0, track_h, 1.5, rgba(255, 255, 255, 24));
            ui.rounded(track_x, thumb_y, 3.0, thumb_h, 1.5, accent_hi);
        }

        // Preset Management Sub-Modal (when self.presets_modal_open == true)
        if self.presets_modal_open {
            ui.fill(0.0, 0.0, w, h, rgba(0, 0, 0, 200));
            let mdw = (w - 24.0).clamp(320.0, 520.0);
            let mdh = (h - 18.0).clamp(248.0, 360.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            ui.rounded(mdx, mdy, mdw, mdh, 18.0, surface_bg);
            ui.stroke_rounded(mdx, mdy, mdw, mdh, 18.0, 1.5, accent);
            ui.text(
                Face::SemiBold,
                13.0,
                mdx + 16.0,
                mdy + 10.0,
                text_c,
                &format!("{} — Control Presets", settings.control_style.label()),
            );
            ui.text(
                Face::Regular,
                9.2,
                mdx + 16.0,
                mdy + 27.0,
                muted_c,
                &format!(
                    "Independent presets stored in controls/{}/",
                    settings.control_style.storage_subdir()
                ),
            );
            let back_r = [mdx + mdw - 78.0, mdy + 10.0, 64.0, 28.0];
            ui.rounded(back_r[0], back_r[1], back_r[2], back_r[3], 9.0, card_bg);
            ui.border(back_r[0], back_r[1], back_r[2], back_r[3], 9.0, border_c);
            ui.centered_fit(Face::SemiBold, 10.5, back_r, text_c, "Back");

            let list_y = mdy + 48.0;
            let row_h = 46.0;
            let max_rows = ((mdh - 60.0) / (row_h + 6.0)).floor().max(1.0) as usize;
            let fallback_presets;
            let presets_slice = if self.cached_presets_style == Some(settings.control_style)
                && !self.cached_presets.is_empty()
            {
                &self.cached_presets
            } else {
                fallback_presets = crate::touch::Settings::list_presets(
                    std::path::Path::new("."),
                    settings.control_style,
                );
                &fallback_presets
            };
            for (idx, preset) in presets_slice.iter().take(max_rows).enumerate() {
                let ry = list_y + idx as f32 * (row_h + 6.0);
                let row_r = [mdx + 14.0, ry, mdw - 28.0, row_h];
                let sel = settings.preset_id.eq_ignore_ascii_case(&preset.id);
                ui.rounded(
                    row_r[0],
                    row_r[1],
                    row_r[2],
                    row_r[3],
                    11.0,
                    if sel { card_sel } else { card_bg },
                );
                ui.stroke_rounded(
                    row_r[0],
                    row_r[1],
                    row_r[2],
                    row_r[3],
                    11.0,
                    if sel { 1.6 } else { 1.0 },
                    if sel { accent } else { border_c },
                );
                let rx = row_r[0] + 12.0;
                let rcy = ry + 16.0;
                ui.stroke_rounded(
                    rx,
                    rcy,
                    14.0,
                    14.0,
                    7.0,
                    1.5,
                    if sel { accent_hi } else { muted_c },
                );
                if sel {
                    ui.rounded(rx + 3.5, rcy + 3.5, 7.0, 7.0, 3.5, accent_hi);
                }
                let title = if preset.is_default {
                    format!("{} (Protected Default)", preset.name)
                } else {
                    preset.name.clone()
                };
                ui.text(
                    Face::SemiBold,
                    11.0,
                    row_r[0] + 34.0,
                    ry + 6.0,
                    text_c,
                    &title,
                );
                ui.text(
                    Face::Regular,
                    9.0,
                    row_r[0] + 34.0,
                    ry + 24.0,
                    muted_c,
                    &preset.description,
                );

                if !preset.is_default {
                    let edit_r = [row_r[0] + row_r[2] - 104.0, ry + 8.0, 30.0, 30.0];
                    ui.rounded(edit_r[0], edit_r[1], edit_r[2], edit_r[3], 8.0, surface_bg);
                    ui.border(edit_r[0], edit_r[1], edit_r[2], edit_r[3], 8.0, border_c);
                    ui.icon(
                        Icon::Pencil,
                        edit_r[0] + 7.0,
                        edit_r[1] + 7.0,
                        16.0,
                        accent_hi,
                    );
                }

                let copy_r = [row_r[0] + row_r[2] - 68.0, ry + 8.0, 60.0, 30.0];
                ui.rounded(copy_r[0], copy_r[1], copy_r[2], copy_r[3], 8.0, card_sel);
                ui.border(copy_r[0], copy_r[1], copy_r[2], copy_r[3], 8.0, accent);
                ui.icon(
                    Icon::Copy,
                    copy_r[0] + 7.0,
                    copy_r[1] + 8.0,
                    14.0,
                    accent_hi,
                );
                ui.text(
                    Face::SemiBold,
                    9.8,
                    copy_r[0] + 24.0,
                    copy_r[1] + 7.0,
                    text_c,
                    "Copy",
                );
            }
        }

        // Copy / Edit Preset Name & Description Dialog
        if let Some(dlg) = &self.preset_dialog {
            ui.fill(0.0, 0.0, w, h, rgba(0, 0, 0, 212));
            let mdw = (w - 28.0).clamp(310.0, 480.0);
            let mdh = (h - 16.0).clamp(242.0, 330.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            ui.rounded(mdx, mdy, mdw, mdh, 18.0, surface_bg);
            ui.stroke_rounded(mdx, mdy, mdw, mdh, 18.0, 1.6, accent);
            ui.text(
                Face::SemiBold,
                13.0,
                mdx + 14.0,
                mdy + 10.0,
                text_c,
                if dlg.is_copy {
                    "Copy Preset Layout — Enter Name & Description"
                } else {
                    "Edit Preset Name & Description"
                },
            );
            let cancel_r = [mdx + mdw - 76.0, mdy + 10.0, 64.0, 26.0];
            ui.rounded(
                cancel_r[0],
                cancel_r[1],
                cancel_r[2],
                cancel_r[3],
                8.0,
                card_bg,
            );
            ui.border(
                cancel_r[0],
                cancel_r[1],
                cancel_r[2],
                cancel_r[3],
                8.0,
                border_c,
            );
            ui.centered_fit(Face::SemiBold, 10.0, cancel_r, text_c, "Cancel");

            let name_r = [mdx + 14.0, mdy + 42.0, (mdw - 36.0) * 0.45, 48.0];
            let desc_r = [
                mdx + 22.0 + (mdw - 36.0) * 0.45,
                mdy + 42.0,
                (mdw - 36.0) * 0.55,
                48.0,
            ];
            ui.text_field(
                name_r[0],
                name_r[1],
                name_r[2],
                name_r[3],
                "PRESET NAME",
                dlg.name_input.as_str(),
                "Tap to type",
                !dlg.editing_desc,
            );
            ui.text_field(
                desc_r[0],
                desc_r[1],
                desc_r[2],
                desc_r[3],
                "DESCRIPTION",
                dlg.desc_input.as_str(),
                "Tap to type",
                dlg.editing_desc,
            );

            ui.hint_line(
                mdx + 14.0,
                mdy + 104.0,
                mdw - 28.0,
                "Tap a field to type; press Enter to close keyboard, then tap Save below.",
                muted_c,
            );

            let save_r = [mdx + 14.0, mdy + mdh - 38.0, mdw - 28.0, 30.0];
            ui.rounded(save_r[0], save_r[1], save_r[2], save_r[3], 9.0, accent);
            ui.centered_fit(
                Face::SemiBold,
                11.0,
                save_r,
                accent_fg,
                if dlg.is_copy {
                    "Create Independent Preset Copy"
                } else {
                    "Save Preset Details"
                },
            );            if dlg.confirm_replace_open {
                ui.fill(0.0, 0.0, w, h, rgba(0, 0, 0, 212));
                let cdw = (w - 36.0).clamp(290.0, 430.0);
                let cdh = 182.0;
                let cdx = (w - cdw) * 0.5;
                let cdy = (h - cdh) * 0.5;
                ui.rounded(cdx, cdy, cdw, cdh, 18.0, surface_bg);
                ui.stroke_rounded(cdx, cdy, cdw, cdh, 18.0, 1.6, accent);
                ui.text(
                    Face::SemiBold,
                    13.5,
                    cdx + 18.0,
                    cdy + 16.0,
                    text_c,
                    "Replace Existing Control Preset?",
                );
                ui.text(
                    Face::Regular,
                    10.2,
                    cdx + 18.0,
                    cdy + 44.0,
                    muted_c,
                    &format!("A control preset named '{}' already exists.", dlg.name_input.trim()),
                );
                ui.text(
                    Face::Regular,
                    9.8,
                    cdx + 18.0,
                    cdy + 64.0,
                    muted_c,
                    "Duplicate preset names are not allowed. Save to replace the current one?",
                );
                let bw = (cdw - 44.0) * 0.5;
                let by = cdy + cdh - 50.0;
                let save_r = [cdx + 18.0, by, bw, 36.0];
                let cancel_r = [cdx + 26.0 + bw, by, bw, 36.0];
                ui.rounded(save_r[0], save_r[1], save_r[2], save_r[3], 10.0, accent);
                ui.centered_fit(Face::SemiBold, 11.0, save_r, accent_fg, "Save");
                ui.rounded(cancel_r[0], cancel_r[1], cancel_r[2], cancel_r[3], 10.0, card_bg);
                ui.border(cancel_r[0], cancel_r[1], cancel_r[2], cancel_r[3], 10.0, border_c);
                ui.centered_fit(Face::SemiBold, 11.0, cancel_r, text_c, "Cancel");
            }
        }

        // Volt+ Extras Waypoint Manager Sub-Modal
        if self.volt_waypoint_manager {
            ui.fill(0.0, 0.0, w, h, rgba(0, 0, 0, 205));
            let mdw = (w - 24.0).clamp(310.0, 540.0);
            let mdh = (h - 24.0).clamp(235.0, 350.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            ui.rounded(mdx, mdy, mdw, mdh, 16.0, rgba(0, 0, 0, 248));
            ui.border(mdx, mdy, mdw, mdh, 16.0, accent);
            let total = self.volt_waypoints_snapshot.len();
            ui.text(
                Face::SemiBold,
                12.5,
                mdx + 14.0,
                mdy + 12.0,
                text_c,
                &format!("Waypoint Manager  •  {total} saved"),
            );
            let close_r = [mdx + mdw - 72.0, mdy + 10.0, 58.0, 26.0];
            ui.rounded(close_r[0], close_r[1], close_r[2], close_r[3], 8.0, card_bg);
            ui.border(close_r[0], close_r[1], close_r[2], close_r[3], 8.0, border_c);
            ui.centered_fit(Face::SemiBold, 9.5, close_r, text_c, "Close");

            let top_y = mdy + 42.0;
            let btn_w = (mdw - 40.0) / 3.0;
            for (bi, (lbl, fill_c)) in [
                ("+ Add Here", accent),
                ("Clear Deaths", card_sel),
                ("Clear All", card_bg),
            ]
            .into_iter()
            .enumerate()
            {
                let bx = mdx + 14.0 + bi as f32 * (btn_w + 6.0);
                ui.rounded(bx, top_y, btn_w, 26.0, 8.0, fill_c);
                ui.border(bx, top_y, btn_w, 26.0, 8.0, border_c);
                ui.centered_fit(
                    Face::SemiBold,
                    9.2,
                    [bx, top_y, btn_w, 26.0],
                    if bi == 0 { accent_fg } else { text_c },
                    lbl,
                );
            }

            let per_page = 4usize;
            let pages = total.div_ceil(per_page).max(1);
            let page = self.volt_waypoints_page.min(pages - 1);
            let list_y = top_y + 34.0;
            if total == 0 {
                ui.centered_fit(
                    Face::Regular,
                    10.2,
                    [mdx + 14.0, list_y + 28.0, mdw - 28.0, 32.0],
                    muted_c,
                    "No waypoints saved yet. Tap '+ Add Here' to bookmark this spot.",
                );
            } else {
                let start = page * per_page;
                let end = (start + per_page).min(total);
                for (slot_i, wp) in self.volt_waypoints_snapshot[start..end].iter().enumerate() {
                    let ry = list_y + slot_i as f32 * 46.0;
                    ui.rounded(mdx + 14.0, ry, mdw - 28.0, 40.0, 10.0, card_bg);
                    ui.border(mdx + 14.0, ry, mdw - 28.0, 40.0, 10.0, border_c);
                    ui.rounded(mdx + 22.0, ry + 11.0, 10.0, 18.0, 4.0, wp.color);

                    let del_r = [mdx + mdw - 68.0, ry + 7.0, 54.0, 26.0];
                    let share_r = [del_r[0] - 54.0, ry + 7.0, 48.0, 26.0];
                    let col_r = [share_r[0] - 32.0, ry + 7.0, 26.0, 26.0];
                    let tx = mdx + 40.0;
                    let tw = (col_r[0] - tx - 8.0).max(40.0);
                    let tag = if wp.is_death { " [Death]" } else { "" };
                    ui.text_clipped(
                        Face::SemiBold,
                        10.0,
                        tx,
                        ry + 5.0,
                        tw,
                        text_c,
                        &format!("{}{}", wp.name, tag),
                    );
                    ui.text_clipped(
                        Face::Regular,
                        8.4,
                        tx,
                        ry + 21.0,
                        tw,
                        muted_c,
                        &format!(
                            "{}, {}, {}  •  {}",
                            wp.coords[0], wp.coords[1], wp.coords[2], wp.dim_label
                        ),
                    );
                    ui.rounded(col_r[0], col_r[1], col_r[2], col_r[3], 7.0, wp.color);
                    ui.border(col_r[0], col_r[1], col_r[2], col_r[3], 7.0, border_c);
                    ui.rounded(share_r[0], share_r[1], share_r[2], share_r[3], 7.0, card_sel);
                    ui.border(share_r[0], share_r[1], share_r[2], share_r[3], 7.0, border_c);
                    ui.centered_fit(Face::Medium, 8.6, share_r, text_c, "Share");
                    ui.rounded(del_r[0], del_r[1], del_r[2], del_r[3], 7.0, rgba(90, 26, 26, 230));
                    ui.border(del_r[0], del_r[1], del_r[2], del_r[3], 7.0, rgba(235, 96, 96, 220));
                    ui.centered_fit(Face::SemiBold, 8.8, del_r, rgba(255, 220, 220, 255), "Delete");
                }
            }
            let pager_y = mdy + mdh - 34.0;
            let prev_r = [mdx + 14.0, pager_y, 74.0, 24.0];
            let next_r = [mdx + mdw - 88.0, pager_y, 74.0, 24.0];
            ui.rounded(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 7.0, card_bg);
            ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 7.0, border_c);
            ui.centered_fit(Face::Medium, 8.8, prev_r, if page > 0 { text_c } else { muted_c }, "< Prev");
            ui.centered_fit(
                Face::Regular,
                8.8,
                [mdx + 94.0, pager_y, mdw - 188.0, 24.0],
                muted_c,
                &format!("Page {} / {}", page + 1, pages),
            );
            ui.rounded(next_r[0], next_r[1], next_r[2], next_r[3], 7.0, card_bg);
            ui.border(next_r[0], next_r[1], next_r[2], next_r[3], 7.0, border_c);
            ui.centered_fit(
                Face::Medium,
                8.8,
                next_r,
                if page + 1 < pages { text_c } else { muted_c },
                "Next >",
            );
        }

        // Volt+ Extras Block Outline Color Customizer Sub-Modal
        if self.volt_block_outline_color_modal {
            ui.fill(0.0, 0.0, w, h, rgba(0, 0, 0, 205));
            let mdw = (w - 20.0).clamp(310.0, 560.0);
            let mdh = (h - 20.0).clamp(250.0, 360.0);
            let mdx = (w - mdw) * 0.5;
            let mdy = (h - mdh) * 0.5;
            ui.rounded(mdx, mdy, mdw, mdh, 16.0, rgba(0, 0, 0, 248));
            ui.border(mdx, mdy, mdw, mdh, 16.0, accent);

            ui.text(
                Face::SemiBold,
                12.5,
                mdx + 14.0,
                mdy + 12.0,
                text_c,
                "Block Outline Color",
            );
            let done_r = [mdx + mdw - 72.0, mdy + 10.0, 58.0, 26.0];
            let reset_r = [mdx + mdw - 134.0, mdy + 10.0, 56.0, 26.0];
            ui.rounded(reset_r[0], reset_r[1], reset_r[2], reset_r[3], 8.0, card_bg);
            ui.border(reset_r[0], reset_r[1], reset_r[2], reset_r[3], 8.0, border_c);
            ui.centered_fit(Face::Medium, 9.2, reset_r, text_c, "Reset");
            ui.rounded(done_r[0], done_r[1], done_r[2], done_r[3], 8.0, accent);
            ui.border(done_r[0], done_r[1], done_r[2], done_r[3], 8.0, accent_hi);
            ui.centered_fit(Face::SemiBold, 9.5, done_r, accent_fg, "Done");

            let lx = mdx + 14.0;
            let lw = (mdw * 0.34).clamp(112.0, 174.0);
            let ly = mdy + 44.0;
            let rx = lx + lw + 12.0;
            let rw = (mdx + mdw - 14.0 - rx).max(150.0);
            let ry = mdy + 44.0;
            let bo = &self.volt_extras.blocks_outline;

            // Left column: live block outline + fill preview box
            ui.rounded(lx, ly, lw, 108.0, 12.0, well_bg);
            ui.border(lx, ly, lw, 108.0, 12.0, border_c);
            let box_sz = 56.0;
            let bx = lx + (lw - box_sz) * 0.5;
            let by = ly + 12.0;
            // Textured grass/stone-like block preview tile
            ui.rounded(bx, by, box_sz, box_sz, 4.0, rgba(72, 76, 84, 255));
            ui.rounded(bx, by, box_sz, 16.0, 4.0, rgba(76, 138, 66, 255));
            let live_rgb = bo.active_rgb(0.8);
            let live_a = (f32::from(bo.alpha_pct) * 2.55).round().clamp(24.0, 255.0) as u8;
            let live_abgr = crate::extras::BlocksOutlineConfig::to_abgr(live_rgb, live_a);
            if bo.fill_overlay {
                let fill_rgb = bo.active_fill_rgb(0.8);
                let fill_a = (f32::from(bo.fill_alpha_pct) * 2.55)
                    .round()
                    .clamp(12.0, 220.0) as u8;
                let fill_abgr = crate::extras::BlocksOutlineConfig::to_abgr(fill_rgb, fill_a);
                ui.rounded(bx, by, box_sz, box_sz, 4.0, fill_abgr);
            }
            if bo.high_contrast_backing {
                ui.stroke_rounded(
                    bx - 1.5,
                    by - 1.5,
                    box_sz + 3.0,
                    box_sz + 3.0,
                    4.0,
                    (f32::from(bo.line_width) * 0.6).clamp(1.5, 5.0) + 2.0,
                    rgba(0, 0, 0, 255),
                );
            }
            ui.stroke_rounded(
                bx,
                by,
                box_sz,
                box_sz,
                4.0,
                (f32::from(bo.line_width) * 0.6).clamp(1.2, 5.5),
                live_abgr,
            );

            let active_rgb = if self.volt_block_outline_edit_fill {
                bo.active_fill_rgb(0.8)
            } else {
                bo.active_rgb(0.8)
            };
            let [cr, cg, cb] = crate::extras::BlocksOutlineConfig::rgb_bytes(active_rgb);
            let active_a = if self.volt_block_outline_edit_fill {
                bo.fill_alpha_pct
            } else {
                bo.alpha_pct
            };
            let hex_lbl = if !self.volt_block_outline_edit_fill && bo.rainbow {
                format!("RAINBOW • {active_a}%")
            } else {
                format!(
                    "{} • {cr},{cg},{cb}",
                    crate::extras::BlocksOutlineConfig::hex_string(active_rgb)
                )
            };
            ui.centered_fit(
                Face::SemiBold,
                9.0,
                [lx + 4.0, ly + 76.0, lw - 8.0, 24.0],
                text_c,
                &hex_lbl,
            );

            // Target selector pills: Outline vs Fill
            let tab_w = (lw - 6.0) * 0.5;
            for (ti, (lbl, is_fill)) in [("Outline", false), ("Fill", true)].into_iter().enumerate()
            {
                let tr = [lx + ti as f32 * (tab_w + 6.0), ly + 116.0, tab_w, 26.0];
                let active = self.volt_block_outline_edit_fill == is_fill;
                ui.rounded(
                    tr[0],
                    tr[1],
                    tr[2],
                    tr[3],
                    8.0,
                    if active { accent } else { card_bg },
                );
                ui.border(
                    tr[0],
                    tr[1],
                    tr[2],
                    tr[3],
                    8.0,
                    if active { accent_hi } else { border_c },
                );
                ui.centered_fit(
                    Face::SemiBold,
                    9.0,
                    tr,
                    if active { accent_fg } else { text_c },
                    lbl,
                );
            }
            let opt1_r = [lx, ly + 148.0, lw, 26.0];
            let opt1_on = if self.volt_block_outline_edit_fill {
                bo.fill_match_outline
            } else {
                bo.rainbow
            };
            let opt1_lbl = if self.volt_block_outline_edit_fill {
                if opt1_on { "Match Outline: ON" } else { "Match Outline: OFF" }
            } else if opt1_on {
                "Rainbow: ON"
            } else {
                "Rainbow: OFF"
            };
            ui.rounded(
                opt1_r[0],
                opt1_r[1],
                opt1_r[2],
                opt1_r[3],
                8.0,
                if opt1_on { card_sel } else { card_bg },
            );
            ui.border(
                opt1_r[0],
                opt1_r[1],
                opt1_r[2],
                opt1_r[3],
                8.0,
                if opt1_on { accent } else { border_c },
            );
            ui.centered_fit(Face::Medium, 8.8, opt1_r, text_c, opt1_lbl);

            let opt2_r = [lx, ly + 178.0, lw, 26.0];
            ui.rounded(
                opt2_r[0],
                opt2_r[1],
                opt2_r[2],
                opt2_r[3],
                8.0,
                if bo.fill_overlay { card_sel } else { card_bg },
            );
            ui.border(
                opt2_r[0],
                opt2_r[1],
                opt2_r[2],
                opt2_r[3],
                8.0,
                if bo.fill_overlay { accent } else { border_c },
            );
            ui.centered_fit(
                Face::Medium,
                8.8,
                opt2_r,
                text_c,
                if bo.fill_overlay {
                    "Fill Overlay: ON"
                } else {
                    "Fill Overlay: OFF"
                },
            );

            // Right column: 12 Quick Color Swatches + R / G / B / Opacity sliders
            let sw_gap = 5.0;
            let sw_w = (rw - sw_gap * 5.0) / 6.0;
            let sw_h = 26.0;
            let selected_rgb = if self.volt_block_outline_edit_fill {
                bo.fill_color_rgb & 0x00FF_FFFF
            } else {
                bo.color_rgb & 0x00FF_FFFF
            };
            for (i, &(rgb, _name)) in BLOCK_OUTLINE_SWATCHES.iter().enumerate() {
                let col = (i % 6) as f32;
                let row = (i / 6) as f32;
                let sx = rx + col * (sw_w + sw_gap);
                let sy = ry + row * (sw_h + 5.0);
                let sw_abgr = crate::extras::BlocksOutlineConfig::to_abgr(rgb, 255);
                ui.rounded(sx, sy, sw_w, sw_h, 7.0, sw_abgr);
                let is_sel = selected_rgb == rgb
                    && (!self.volt_block_outline_edit_fill || !bo.fill_match_outline)
                    && (self.volt_block_outline_edit_fill || !bo.rainbow);
                ui.stroke_rounded(
                    sx,
                    sy,
                    sw_w,
                    sw_h,
                    7.0,
                    if is_sel { 2.4 } else { 1.0 },
                    if is_sel {
                        rgba(255, 255, 255, 255)
                    } else {
                        border_c
                    },
                );
            }

            let sl_y0 = ry + 68.0;
            let sliders: [(&str, f32, u32, String); 4] = [
                ("Red", f32::from(cr) / 255.0, rgba(235, 72, 72, 255), format!("{cr}")),
                ("Green", f32::from(cg) / 255.0, rgba(64, 210, 108, 255), format!("{cg}")),
                ("Blue", f32::from(cb) / 255.0, rgba(66, 152, 250, 255), format!("{cb}")),
                (
                    "Opacity",
                    if self.volt_block_outline_edit_fill {
                        (f32::from(bo.fill_alpha_pct) - 5.0) / 75.0
                    } else {
                        (f32::from(bo.alpha_pct) - 10.0) / 90.0
                    },
                    accent,
                    format!("{active_a}%"),
                ),
            ];
            for (ch, (lbl, frac, bar_c, val_txt)) in sliders.into_iter().enumerate() {
                let sy = sl_y0 + ch as f32 * 34.0;
                ui.text(Face::Medium, 9.0, rx, sy, text_c, lbl);
                let vw = ui.measure(Face::SemiBold, 9.0, &val_txt);
                ui.text(Face::SemiBold, 9.0, rx + rw - vw, sy, muted_c, &val_txt);
                let bar_y = sy + 13.0;
                ui.rounded(rx, bar_y, rw, 12.0, 6.0, well_bg);
                let fill_w = (rw * frac.clamp(0.0, 1.0)).max(10.0);
                ui.rounded(rx, bar_y, fill_w, 12.0, 6.0, bar_c);
                ui.border(rx, bar_y, rw, 12.0, 6.0, border_c);
                let knob_x = (rx + rw * frac.clamp(0.0, 1.0) - 6.0).clamp(rx, rx + rw - 12.0);
                ui.rounded(knob_x, bar_y - 2.0, 12.0, 16.0, 6.0, rgba(255, 255, 255, 255));
            }
        }

        // Volt+ Extras Mod Keybind Picker Sub-Modal (same picker UI as Controls)
        if let Some((_mod_id, _is_use, step)) = self.volt_key_picker {
            draw_input_picker_modal(
                &mut ui,
                w,
                h,
                step,
                self.custom_kb_scroll_x,
                false,
                false,
                true,
            );
        }

        // Send Custom Input Picker Sub-Modal
        if self.custom_input_picker != crate::touch::PickerStep::None {
            draw_input_picker_modal(
                &mut ui,
                w,
                h,
                self.custom_input_picker,
                self.custom_kb_scroll_x,
                true,
                false,
                true,
            );
        }

        ui.finish();
    }
}

fn push_undo_snapshot(ed: &mut crate::touch::Editor, current: &crate::touch::Settings) {
    if ed.undo_stack.len() >= 40 {
        ed.undo_stack.remove(0);
    }
    ed.undo_stack.push(current.clone());
    ed.redo_stack.clear();
}

fn duplicate_selected_editor_button(ed: &mut crate::touch::Editor, tc: &mut crate::touch::TouchControls) {
    if tc.settings.custom_buttons.len() >= 256 {
        ed.status_msg = Some("Maximum of 256 custom buttons reached.".into());
        return;
    }
    if let Some(idx) = ed.selected_custom {
        if let Some(src) = tc.settings.custom_buttons.get(idx).cloned() {
            push_undo_snapshot(ed, &tc.settings);
            let mut copy = src;
            for m in 0..3 {
                copy.layouts[m].center[0] = (copy.layouts[m].center[0] + 0.04).clamp(0.0, 0.90);
                copy.layouts[m].center[1] = (copy.layouts[m].center[1] + 0.04).clamp(0.0, 0.90);
            }
            let new_idx = tc.settings.custom_buttons.len();
            let lbl = copy.label();
            tc.settings.custom_buttons.push(copy);
            ed.selected_custom = Some(new_idx);
            ed.selected = None;
            let _ = tc.settings.save();
            ed.status_msg = Some(format!("Copied button '{lbl}'."));
        }
    } else if let Some(ctrl) = ed.selected {
        if ctrl != crate::touch::Control::Move {
            push_undo_snapshot(ed, &tc.settings);
            let new_idx = tc.settings.custom_buttons.len();
            let token = match ctrl {
                crate::touch::Control::Attack => "action:key.attack",
                crate::touch::Control::Use => "action:key.use",
                crate::touch::Control::Jump => "action:key.jump",
                crate::touch::Control::Sneak => "action:key.sneak",
                crate::touch::Control::Sprint => "action:key.sprint",
                crate::touch::Control::Offhand => "action:key.swapOffhand",
                crate::touch::Control::Inventory => "action:key.inventory",
                crate::touch::Control::Chat => "action:key.chat",
                crate::touch::Control::Debug => "action:key.debug.overlay",
                crate::touch::Control::Perspective => "action:key.togglePerspective",
                _ => "key.keyboard.unknown",
            };
            let mut copy = crate::touch::CustomButton::new(token.into(), new_idx);
            copy.custom_label = tc.settings.control_label(ctrl).to_owned();
            let slot = tc.settings.slot(ctrl);
            for m in 0..3 {
                let mut pl = tc.settings.layouts[m][slot];
                pl.center[0] = (pl.center[0] + 0.04).clamp(0.0, 0.90);
                pl.center[1] = (pl.center[1] + 0.04).clamp(0.0, 0.90);
                copy.layouts[m] = pl;
            }
            let lbl = copy.label();
            tc.settings.custom_buttons.push(copy);
            ed.selected_custom = Some(new_idx);
            ed.selected = None;
            let _ = tc.settings.save();
            ed.status_msg = Some(format!("Copied '{lbl}' as a custom button."));
        }
    }
}

fn clamp_editor_custom_selection(ed: &mut crate::touch::Editor, settings: &crate::touch::Settings) {
    if let Some(idx) = ed.selected_custom
        && idx >= settings.custom_buttons.len()
    {
        ed.selected_custom = None;
        if ed.selected.is_none() {
            ed.selected = Some(crate::touch::Control::Move);
        }
    }
}

fn shape_name(shape: u8) -> &'static str {
    match shape {
        1 => "Square (Sharp)",
        2 => "Round / Circle",
        3 => "Cut Octagon",
        4 => "Diamond",
        5 => "Hexagon",
        6 => "Triangle",
        _ => "Rounded",
    }
}

pub fn control_icon_from_id(icon_id: u8) -> Option<(crate::ui_theme::Icon, &'static str)> {
    use crate::ui_theme::Icon;
    match icon_id {
        1 => Some((Icon::Crosshair, "Crosshair")),
        2 => Some((Icon::Gamepad, "Gamepad")),
        3 => Some((Icon::Play, "Attack")),
        4 => Some((Icon::Pause, "Pause")),
        5 => Some((Icon::Settings, "Settings")),
        6 => Some((Icon::Search, "Zoom")),
        7 => Some((Icon::Check, "Confirm")),
        8 => Some((Icon::Monitor, "Camera")),
        9 => Some((Icon::Flask, "Use/Potion")),
        10 => Some((Icon::Terminal, "Debug/Cmd")),
        11 => Some((Icon::MicrosoftGrid, "Inventory")),
        12 => Some((Icon::Share, "Swap/Drop")),
        _ => None,
    }
}

pub fn control_icon_label(icon_id: u8) -> &'static str {
    control_icon_from_id(icon_id).map_or("Text Only", |(_, name)| name)
}

fn draw_button_icon_and_label(
    ui: &mut crate::ui_theme::Ui,
    rect: [f32; 4],
    icon_id: u8,
    label: &str,
    fg_col: u32,
) {
    let [rx, ry, rw, rh] = rect;
    if let Some((ic, _)) = control_icon_from_id(icon_id) {
        if !label.is_empty() && rw > rh * 1.35 {
            let ic_sz = (rh * 0.44).clamp(11.0, 18.0);
            ui.icon(ic, rx + 6.0, ry + (rh - ic_sz) * 0.5, ic_sz, fg_col);
            ui.centered_fit(
                crate::ui_theme::Face::SemiBold,
                (rh * 0.28).clamp(9.0, 14.0),
                [
                    rx + ic_sz + 8.0,
                    ry + 2.0,
                    (rw - ic_sz - 11.0).max(6.0),
                    (rh - 4.0).max(6.0),
                ],
                fg_col,
                label,
            );
        } else if !label.is_empty() && rh >= 34.0 {
            let ic_sz = (rh * 0.40).clamp(11.0, 18.0);
            ui.icon(
                ic,
                rx + (rw - ic_sz) * 0.5,
                ry + (rh * 0.52 - ic_sz).max(3.0),
                ic_sz,
                fg_col,
            );
            ui.centered_fit(
                crate::ui_theme::Face::SemiBold,
                (rh * 0.22).clamp(8.0, 11.5),
                [rx + 2.0, ry + rh * 0.52, (rw - 4.0).max(6.0), rh * 0.44],
                fg_col,
                label,
            );
        } else {
            let ic_sz = (rh.min(rw) * 0.52).clamp(12.0, 22.0);
            ui.icon(
                ic,
                rx + (rw - ic_sz) * 0.5,
                ry + (rh - ic_sz) * 0.5,
                ic_sz,
                fg_col,
            );
        }
    } else {
        let trimmed = label.trim();
        if matches!(
            trimmed,
            "⬛" | "⬜" | "■" | "□" | "◼" | "◻" | "▪" | "▫"
        ) {
            let alpha = ((fg_col >> 24) & 0xFF) as f32 / 255.0;
            let sz = (rh.min(rw) * 0.24).clamp(10.0, 18.0);
            let sx = rx + (rw - sz) * 0.5;
            let sy = ry + (rh - sz) * 0.5;
            let rad = (sz * 0.18).max(2.0);
            let a_u8 = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            match trimmed {
                "⬛" | "◼" | "▪" => {
                    ui.rounded(sx, sy, sz, sz, rad, rgba(28, 30, 36, a_u8));
                    ui.border(
                        sx,
                        sy,
                        sz,
                        sz,
                        rad,
                        rgba(195, 200, 212, (alpha * 130.0).round() as u8),
                    );
                }
                "⬜" | "◻" | "▫" | "■" => {
                    ui.rounded(sx, sy, sz, sz, rad, fg_col);
                }
                _ => {
                    ui.border(sx, sy, sz, sz, rad, fg_col);
                }
            }
            return;
        }
        ui.centered_fit(
            crate::ui_theme::Face::SemiBold,
            (rh * 0.30).clamp(9.5, 14.5),
            [rx + 3.0, ry + 2.0, (rw - 6.0).max(6.0), (rh - 4.0).max(6.0)],
            fg_col,
            label,
        );
    }
}

fn draw_preview_brackets(ui: &mut crate::ui_theme::Ui, rect: [f32; 4], col: u32) {
    let [x, y, w, h] = rect;
    let arm = 8.0;
    let t = 1.6;
    // Top-left ⌜
    ui.fill(x + 6.0, y + 6.0, arm, t, col);
    ui.fill(x + 6.0, y + 6.0, t, arm, col);
    // Top-right ⌝
    ui.fill(x + w - 6.0 - arm, y + 6.0, arm, t, col);
    ui.fill(x + w - 6.0 - t, y + 6.0, t, arm, col);
    // Bottom-left ⌞
    ui.fill(x + 6.0, y + h - 6.0 - t, arm, t, col);
    ui.fill(x + 6.0, y + h - 6.0 - arm, t, arm, col);
    // Bottom-right ⌟
    ui.fill(x + w - 6.0 - arm, y + h - 6.0 - t, arm, t, col);
    ui.fill(x + w - 6.0 - t, y + h - 6.0 - arm, t, arm, col);
}

/// Studio Slider Card matching `90511.png`:
/// Renders a rounded dark slate card with top-left icon + title, right value pill badge,
/// and a bottom horizontal cyan slider track (`#1DB4F6`) with a white circular thumb knob.
#[allow(clippy::too_many_arguments)]
fn draw_m3_stepper_cell(
    ui: &mut crate::ui_theme::Ui,
    rect: [f32; 4],
    label: &str,
    fraction: f32,
    accent: u32,
    accent_soft: u32,
    bg_hi: u32,
    _border_c: u32,
    text_c: u32,
) {
    use crate::ui_theme::Face;
    let [rx, ry, cw, ch] = rect;
    let rad = (ch * 0.16).clamp(6.0, 9.0);
    // Clean Material 3 Expressive tonal container
    ui.rounded(rx, ry, cw, ch, rad, bg_hi);
    ui.border(rx, ry, cw, ch, rad, _border_c);

    let (title_part, val_part) = if let Some((t_str, v_str)) = label.split_once('|') {
        (t_str.trim(), v_str.trim())
    } else if let Some((t_str, v_str)) = label.rsplit_once(' ') {
        (t_str.trim(), v_str.trim())
    } else {
        (label, "")
    };

    let t = fraction.clamp(0.0, 1.0);
    if ch >= 52.0 {
        ui.text_clipped(
            Face::Medium,
            9.8,
            rx + 12.0,
            ry + 8.0,
            (cw - 24.0).max(24.0),
            text_c,
            title_part,
        );

        let pill_w = (ui.measure(Face::SemiBold, 9.4, val_part) + 14.0).clamp(38.0, cw - 20.0);
        let pill_x = rx + cw - pill_w - 10.0;
        let pill_y = ry + 23.0;
        ui.rounded(pill_x, pill_y, pill_w, 18.0, 5.0, accent_soft);
        ui.centered_fit(
            Face::SemiBold,
            9.2,
            [pill_x + 3.0, pill_y, pill_w - 6.0, 18.0],
            accent,
            val_part,
        );

        // Material 3 Expressive 8dp Slider Track + Vertical Pill Handle
        let track_x = rx + 12.0;
        let track_w = (cw - 24.0).max(20.0);
        let track_y = ry + ch - 16.0;
        ui.rounded(track_x, track_y, track_w, 8.0, 4.0, accent_soft);
        let fill_w = (track_w * t).max(8.0);
        ui.rounded(track_x, track_y, fill_w, 8.0, 4.0, accent);
        let thumb_x = (track_x + track_w * t - 3.0).clamp(track_x, track_x + track_w - 6.0);
        ui.rounded(thumb_x, track_y - 3.0, 6.0, 14.0, 3.0, text_c);
    } else {
        let track_x = rx + 10.0;
        let track_w = (cw - 20.0).max(16.0);
        let track_y = ry + ch - 10.0;
        ui.rounded(track_x, track_y, track_w, 6.0, 3.0, accent_soft);
        let fill_w = (track_w * t).max(6.0);
        ui.rounded(track_x, track_y, fill_w, 6.0, 3.0, accent);
        let thumb_x = (track_x + track_w * t - 2.5).clamp(track_x, track_x + track_w - 5.0);
        ui.rounded(thumb_x, track_y - 2.0, 5.0, 10.0, 2.5, text_c);
        ui.text_clipped(
            Face::Medium,
            9.0,
            rx + 10.0,
            ry + 4.0,
            (cw * 0.58).max(20.0),
            text_c,
            title_part,
        );
        ui.text_clipped(
            Face::SemiBold,
            9.0,
            rx + cw - (ui.measure(Face::SemiBold, 9.0, val_part) + 10.0).min(cw * 0.45),
            ry + 4.0,
            (cw * 0.42).max(16.0),
            accent,
            val_part,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_m3_selector_cell(
    ui: &mut crate::ui_theme::Ui,
    rect: [f32; 4],
    title: &str,
    value: &str,
    accent: u32,
    accent_soft: u32,
    bg_hi: u32,
    text_c: u32,
    muted_c: u32,
) {
    use crate::ui_theme::Face;
    let [rx, ry, cw, ch] = rect;
    let rad = (ch * 0.16).clamp(6.0, 9.0);
    ui.rounded(rx, ry, cw, ch, rad, bg_hi);
    ui.text_clipped(
        Face::Medium,
        9.6,
        rx + 12.0,
        ry + 8.0,
        (cw - 24.0).max(24.0),
        muted_c,
        title,
    );
    let pill_r = [rx + 10.0, ry + ch - 34.0, (cw - 20.0).max(24.0), 24.0];
    ui.rounded(
        pill_r[0],
        pill_r[1],
        pill_r[2],
        pill_r[3],
        6.0,
        accent_soft,
    );
    ui.centered_fit(
        Face::SemiBold,
        9.6,
        [pill_r[0] + 6.0, pill_r[1], pill_r[2] - 12.0, pill_r[3]],
        accent,
        &format!("‹  {value}  ›"),
    );
    let _ = text_c;
}

/// Circular Swatch Chip matching `90511.png` with a crisp white outer selection ring.
#[allow(clippy::too_many_arguments)]
fn draw_m3_swatch_chip(
    ui: &mut crate::ui_theme::Ui,
    sx: f32,
    sy: f32,
    sw_w: f32,
    sw_h: f32,
    col: u32,
    selected: bool,
    accent: u32,
    _text_c: u32,
) {
    let d = sw_w.min(sw_h);
    let cx = sx + (sw_w - d) * 0.5;
    let cy = sy + (sw_h - d) * 0.5;
    let rad = d * 0.5;
    if selected {
        ui.rounded(
            cx - 3.0,
            cy - 3.0,
            d + 6.0,
            d + 6.0,
            rad + 3.0,
            crate::ui_theme::color::with_alpha(accent, 0.32),
        );
    }
    ui.rounded(cx, cy, d, d, rad, col);
    ui.border(cx, cy, d, d, rad, rgba(255, 255, 255, 64));
    if selected {
        ui.stroke_rounded(
            cx - 2.0,
            cy - 2.0,
            d + 4.0,
            d + 4.0,
            rad + 2.0,
            2.0,
            rgba(255, 255, 255, 255),
        );
    }
}

/// Snaps a dragged button's normalized `center` (`[cx, cy]`) to neighboring buttons' edges/centers
/// (so buttons stick flush right beside each other with zero gap or align cleanly on shared edges)
/// and falls back to the `0.025` grid per axis when `snap_to_grid` is enabled.
pub fn snap_button_to_neighbors_and_grid(
    tc: &crate::touch::TouchControls,
    ed: &crate::touch::Editor,
    raw_center: [f32; 2],
    size: [f32; 2],
) -> ([f32; 2], Option<f32>, Option<f32>) {
    use crate::touch::{Control, ControlStyle};
    if !tc.settings.snap_to_grid {
        return (raw_center, None, None);
    }
    let aspect = tc.aspect();
    let [mut cx, mut cy] = raw_center;
    let [rw, rh] = [size[0].max(0.02), size[1].max(0.02)];
    let (hw, hh) = (rw * 0.5, rh * 0.5);

    let mut neighbors: Vec<[f32; 4]> = Vec::with_capacity(24);
    for c in Control::ALL {
        if tc.settings.control_style == ControlStyle::GoyControl && c != Control::Move {
            continue;
        }
        if ed.selected_custom.is_none() && ed.selected == Some(c) {
            continue;
        }
        if !tc.visible(c) && !ed.show_hidden {
            continue;
        }
        neighbors.push(tc.placement(c).rect(aspect));
    }
    for (i, _) in tc.settings.custom_buttons.iter().enumerate() {
        if ed.selected_custom == Some(i) {
            continue;
        }
        neighbors.push(tc.custom_placement(i).rect(aspect));
    }

    const SNAP_THRESH: f32 = 0.018;
    let mut best_dx = SNAP_THRESH;
    let mut snap_x: Option<(f32, f32)> = None;
    let mut best_dy = SNAP_THRESH;
    let mut snap_y: Option<(f32, f32)> = None;

    // Screen center alignment
    if (cx - 0.5).abs() < best_dx {
        best_dx = (cx - 0.5).abs();
        snap_x = Some((0.5, 0.5));
    }
    if (cy - 0.5).abs() < best_dy {
        best_dy = (cy - 0.5).abs();
        snap_y = Some((0.5, 0.5));
    }

    for [nx, ny, nw, nh] in neighbors {
        let ncx = nx + nw * 0.5;
        let ncy = ny + nh * 0.5;
        let near_y = (cy - ncy).abs() <= (hh + nh * 0.5 + 0.12);
        let near_x = (cx - ncx).abs() <= (hw + nw * 0.5 + 0.12);

        // Stick flush right beside neighbor (left-to-right or right-to-left) + edge/center alignment
        let x_candidates: [(f32, f32, bool); 5] = [
            (nx + nw + hw, nx + nw, near_y), // dragged left edge sticks to neighbor right edge
            (nx - hw, nx, near_y),           // dragged right edge sticks to neighbor left edge
            (nx + hw, nx, true),             // left edges aligned
            (nx + nw - hw, nx + nw, true),   // right edges aligned
            (ncx, ncx, true),                // horizontal centers aligned
        ];
        for (cand_cx, guide_x, allowed) in x_candidates {
            if allowed {
                let d = (cx - cand_cx).abs();
                if d < best_dx {
                    best_dx = d;
                    snap_x = Some((cand_cx, guide_x));
                }
            }
        }

        // Stick flush above/below neighbor (top-to-bottom or bottom-to-top) + edge/center alignment
        let y_candidates: [(f32, f32, bool); 5] = [
            (ny + nh + hh, ny + nh, near_x), // dragged top edge sticks to neighbor bottom edge
            (ny - hh, ny, near_x),           // dragged bottom edge sticks to neighbor top edge
            (ny + hh, ny, true),             // top edges aligned
            (ny + nh - hh, ny + nh, true),   // bottom edges aligned
            (ncy, ncy, true),                // vertical centers aligned
        ];
        for (cand_cy, guide_y, allowed) in y_candidates {
            if allowed {
                let d = (cy - cand_cy).abs();
                if d < best_dy {
                    best_dy = d;
                    snap_y = Some((cand_cy, guide_y));
                }
            }
        }
    }

    let guide_x = if let Some((scx, gx)) = snap_x {
        cx = scx;
        Some(gx)
    } else {
        cx = (cx * 40.0).round() / 40.0;
        None
    };
    let guide_y = if let Some((scy, gy)) = snap_y {
        cy = scy;
        Some(gy)
    } else {
        cy = (cy * 40.0).round() / 40.0;
        None
    };

    ([cx, cy], guide_x, guide_y)
}

pub fn draw_hud_editor(
    tc: &crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    profile_name: &str,
    notice: Option<&str>,
) {
    use crate::touch::{Control, ControlStyle, CustomButton, JoystickMoveArea, PickerStep};
    use crate::ui_theme::{Face, Icon, Ui, color};
    let Some(ed) = &tc.editor else {
        return;
    };
    let mut ui = Ui::mobile(a, b, (0.0, 0.0));
    let (w, h) = (ui.w, ui.h);
    let dp_to_gui = ui.density / ui.batch.scale;
    let gui_to_dp = ui.batch.scale / ui.density;

    let theme = crate::ui_theme::active_theme();
    let bg_card = theme.bg_solid;
    let bg_hi = theme.surface;
    let border_c = theme.border;
    let accent = theme.accent;
    let accent_soft = theme.surface_selected;
    // Filled accent buttons need on-accent contrast, not accent-colored text.
    let accent_fg = theme.on_accent;
    let text_c = theme.text_primary;
    let muted_c = theme.text_muted;
    let danger_c = theme.danger;

    // 1. Dark Blueprint Canvas & Alignment Grid
    ui.fill(0.0, 0.0, w, h, rgba(9, 10, 16, 170));
    let step = if tc.settings.snap_to_grid {
        0.025
    } else {
        0.05
    };
    let grid_line_col = if tc.settings.snap_to_grid {
        color::with_alpha(accent, 0.16)
    } else {
        rgba(255, 255, 255, 11)
    };
    let mut g = step;
    while g < 1.0 {
        ui.fill(w * g, 0.0, 1.0, h, grid_line_col);
        ui.fill(0.0, h * g, w, 1.0, grid_line_col);
        g += step;
    }
    ui.fill(w * 0.5, 0.0, 1.0, h, rgba(154, 123, 255, 38));
    ui.fill(0.0, h * 0.5, w, 1.0, rgba(154, 123, 255, 38));

    // JoystickMoveArea visual zone indicator when Move is selected
    if ed.selected == Some(Control::Move)
        && ed.selected_custom.is_none()
        && tc.visible(Control::Move)
    {
        let zone = match tc.settings.joystick_style.move_area {
            JoystickMoveArea::MidBottomLeft => Some([0.0, h * 0.35, w * 0.45, h * 0.65]),
            JoystickMoveArea::RightSideAnywhere => Some([w * 0.50, 0.0, w * 0.50, h]),
            JoystickMoveArea::FollowFinger => Some([0.0, h * 0.25, w * 0.48, h * 0.75]),
            JoystickMoveArea::AbsoluteFixed => None,
        };
        if let Some([zx, zy, zw, zh]) = zone {
            ui.rounded(
                zx + 2.0,
                zy + 2.0,
                zw - 4.0,
                zh - 4.0,
                12.0,
                rgba(154, 123, 255, 18),
            );
            ui.border(
                zx + 2.0,
                zy + 2.0,
                zw - 4.0,
                zh - 4.0,
                12.0,
                rgba(154, 123, 255, 75),
            );
            ui.text(
                Face::Medium,
                10.0,
                zx + 10.0,
                zy + 8.0,
                accent,
                &format!(
                    "Joystick Active Zone: {}",
                    tc.settings.joystick_style.move_area.label()
                ),
            );
        }
    }

    let _ = (gui_to_dp, muted_c, profile_name);

    // 3. Draw Built-in Controls (MCPE Style or GoyControl Joystick)
    let mut selected_rect_and_title: Option<([f32; 4], String)> = None;
    for c in Control::ALL {
        if tc.settings.control_style == ControlStyle::GoyControl && c != Control::Move {
            continue;
        }
        let vis = tc.visible(c);
        if !vis && !ed.show_hidden {
            continue;
        }
        let p = tc.placement(c);
        let [nx, ny, nw, nh] = p.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let is_sel = ed.selected == Some(c) && ed.selected_custom.is_none();
        let alpha = if vis {
            p.opacity.clamp(0.0, 1.0)
        } else {
            0.35
        };

        if tc.settings.control_style == ControlStyle::GoyControl
            && c == Control::Move
        {
            // Draw GoyControl Customizable Joystick (Base Area + Inner Stick) identically to in-game
            let js = &tc.settings.joystick_style;
            let side = rw.min(rh);
            let jx = rx + (rw - side) * 0.5;
            let jy = ry + (rh - side) * 0.5;
            let base_fill = color::with_alpha(
                js.base_color,
                js.base_opacity * if vis { 1.0 } else { 0.45 },
            );
            let base_stroke = color::with_alpha(js.base_stroke_color, js.base_opacity);
            let br = (side * (js.base_roundness / 100.0)).clamp(0.0, side * 0.5);
            ui.rounded(jx, jy, side, side, br, base_fill);
            if js.base_stroke > 0.1 {
                ui.stroke_rounded(jx, jy, side, side, br, js.base_stroke, base_stroke);
            }
            let kside = side * js.stick_size.clamp(0.20, 0.85);
            let kx = jx + (side - kside) * 0.5;
            let ky = jy + (side - kside) * 0.5;
            let kr = kside * 0.5;
            let stick_fill = color::with_alpha(js.stick_color, js.stick_opacity);
            let stick_stroke = color::with_alpha(js.stick_stroke_color, js.stick_opacity);
            ui.rounded(kx, ky, kside, kside, kr, stick_fill);
            if js.stick_stroke > 0.1 {
                ui.stroke_rounded(kx, ky, kside, kside, kr, js.stick_stroke, stick_stroke);
            }
        } else if tc.settings.control_style == ControlStyle::GoyControl {
            let fill_c = color::with_alpha(p.button_color, alpha);
            let stroke_c = color::with_alpha(p.stroke_color, alpha);
            let shadow_c = color::with_alpha(p.shadow_color, alpha);
            ui.goy_box(
                rx,
                ry,
                rw,
                rh,
                p.shape,
                p.roundness,
                fill_c,
                p.stroke_size,
                p.stroke_margin,
                p.stroke_roundness,
                stroke_c,
                p.shadow_size,
                shadow_c,
            );
            let fg_col = crate::ui_theme::color::with_alpha(text_c, alpha);
            draw_button_icon_and_label(
                &mut ui,
                [rx, ry, rw, rh],
                p.icon_id,
                tc.settings.control_label(c),
                fg_col,
            );
        } else {
            // Authentic MCPE Sprite / Bevel rendering matching in-game MCPE controls!
            let gui_bounds = [
                rx * dp_to_gui,
                ry * dp_to_gui,
                rw * dp_to_gui,
                rh * dp_to_gui,
            ];
            let mcpe_col = rgba(255, 255, 255, (alpha * 255.0) as u8);
            crate::touch::draw_single_mcpe_control(
                tc, ui.batch, ui.atlas, c, gui_bounds, mcpe_col, is_sel,
            );
        }

        if !vis {
            ui.centered_fit(
                Face::SemiBold,
                8.0,
                [rx, ry + rh - 11.0, rw, 10.0],
                color::hex(0xFBBF24, 220),
                "HIDDEN",
            );
        }
        if ed.apply_all && !is_sel && vis {
            ui.stroke_rounded(rx - 1.5, ry - 1.5, rw + 3.0, rh + 3.0, 5.0, 1.2, color::with_alpha(accent, 0.72));
        }
        if is_sel {
            selected_rect_and_title = Some(([rx, ry, rw, rh], tc.settings.control_label(c).to_owned()));
        }
    }

    // 4. Draw Custom Buttons in BOTH MCPE and GoyControl modes!
    for (i, custom) in tc.settings.custom_buttons.iter().enumerate() {
        let p = tc.custom_placement(i);
        let [nx, ny, nw, nh] = p.rect(tc.aspect());
        let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
        let is_sel = ed.selected_custom == Some(i);
        let alpha = p.opacity.clamp(0.0, 1.0);
        let label = custom.label();

        if tc.settings.control_style == ControlStyle::GoyControl {
            let fill_c = color::with_alpha(p.button_color, alpha);
            let stroke_c = color::with_alpha(p.stroke_color, alpha);
            let shadow_c = color::with_alpha(p.shadow_color, alpha);
            ui.goy_box(
                rx,
                ry,
                rw,
                rh,
                p.shape,
                p.roundness,
                fill_c,
                p.stroke_size,
                p.stroke_margin,
                p.stroke_roundness,
                stroke_c,
                p.shadow_size,
                shadow_c,
            );
            let fg_col = crate::ui_theme::color::with_alpha(text_c, alpha);
            draw_button_icon_and_label(&mut ui, [rx, ry, rw, rh], p.icon_id, &label, fg_col);
        } else {
            let dp_to_gui = ui.batch.width / w.max(1.0);
            let gui_bounds = [
                rx * dp_to_gui,
                ry * dp_to_gui,
                rw * dp_to_gui,
                rh * dp_to_gui,
            ];
            let mcpe_col = rgba(255, 255, 255, (alpha * 255.0).round() as u8);
            crate::touch::draw_mcpe_custom_button(
                ui.batch,
                ui.atlas,
                gui_bounds,
                &label,
                false,
                mcpe_col,
            );
        }

        if ed.apply_all && !is_sel {
            ui.stroke_rounded(rx - 1.5, ry - 1.5, rw + 3.0, rh + 3.0, 5.0, 1.2, color::with_alpha(accent, 0.72));
        }
        if is_sel {
            selected_rect_and_title = Some(([rx, ry, rw, rh], format!("Button: {label}")));
        }
    }

    // 5. Draw Magnetic Snap Guides & 8 Interactive Corner/Edge Resize Handles around Selected Component
    if let Some(([rx, ry, rw, rh], sel_name)) = selected_rect_and_title {
        if tc.settings.snap_to_grid {
            let norm_c = [(rx + rw * 0.5) / w.max(1.0), (ry + rh * 0.5) / h.max(1.0)];
            let norm_s = [rw / w.max(1.0), rh / h.max(1.0)];
            let (_, gx, gy) = snap_button_to_neighbors_and_grid(tc, ed, norm_c, norm_s);
            if let Some(sx) = gx {
                ui.fill(sx * w - 0.75, 0.0, 1.5, h, color::with_alpha(accent, 0.75));
            }
            if let Some(sy) = gy {
                ui.fill(0.0, sy * h - 0.75, w, 1.5, color::with_alpha(accent, 0.75));
            }
        }
        ui.stroke_rounded(rx - 2.0, ry - 2.0, rw + 4.0, rh + 4.0, 6.0, 1.5, accent);
        let handles = [
            (rx - 4.5, ry - 4.5, 9.0, 9.0),
            (rx + rw - 4.5, ry - 4.5, 9.0, 9.0),
            (rx - 4.5, ry + rh - 4.5, 9.0, 9.0),
            (rx + rw - 4.5, ry + rh - 4.5, 9.0, 9.0),
            (rx + rw - 3.5, ry + rh * 0.5 - 7.0, 7.0, 14.0),
            (rx - 3.5, ry + rh * 0.5 - 7.0, 7.0, 14.0),
            (rx + rw * 0.5 - 7.0, ry + rh - 3.5, 14.0, 7.0),
            (rx + rw * 0.5 - 7.0, ry - 3.5, 14.0, 7.0),
        ];
        for (hx, hy, hw, hh) in handles {
            ui.rounded(hx, hy, hw, hh, 3.0, accent);
            ui.border(hx, hy, hw, hh, 3.0, accent_fg);
        }
        let tag = format!("{sel_name} (Drag corners/edges to resize)");
        let tw = (ui.measure(Face::SemiBold, 9.5, &tag) + 12.0).min(w - 16.0);
        let tx = (rx + (rw - tw) * 0.5).clamp(8.0, (w - tw - 8.0).max(8.0));
        let ty = if ry > 24.0 {
            ry - 20.0
        } else {
            (ry + rh + 5.0).min(h - 20.0)
        };
        ui.rounded(tx, ty, tw, 16.0, 5.0, bg_card);
        ui.border(tx, ty, tw, 16.0, 5.0, accent);
        ui.centered_fit(
            Face::SemiBold,
            9.0,
            [tx + 3.0, ty, tw - 6.0, 16.0],
            accent,
            &tag,
        );
    }

    // 6. Material 3 Moveable Floating Menu Pod with Select All & Direct Grid Snap Toggle
    let pod_w = if ed.pod_expanded { 568.0 } else { 52.0 };
    let pod_h = 48.0;
    let pod_x = (tc.settings.floating_menu_pos[0] * w).clamp(8.0, (w - pod_w - 8.0).max(8.0));
    let pod_y = (tc.settings.floating_menu_pos[1] * h).clamp(6.0, (h - pod_h - 6.0).max(6.0));

    ui.rounded(
        pod_x + 2.0,
        pod_y + 4.0,
        pod_w,
        pod_h,
        10.0,
        rgba(0, 0, 0, 145),
    );
    ui.rounded(pod_x, pod_y, pod_w, pod_h, 10.0, bg_card);
    ui.stroke_rounded(pod_x, pod_y, pod_w, pod_h, 10.0, 1.5, accent);

    // Main draggable M3 hub button on the left of the pod (40x40dp)
    ui.rounded(
        pod_x + 4.0,
        pod_y + 4.0,
        40.0,
        40.0,
        8.0,
        if ed.menu_open { accent } else { accent_soft },
    );
    ui.icon(
        if tc.settings.control_style == ControlStyle::Mcpe {
            Icon::HorizonLogo
        } else {
            Icon::MenuEquals
        },
        pod_x + 13.0,
        pod_y + 13.0,
        22.0,
        if ed.menu_open { accent_fg } else { accent },
    );

    if ed.pod_expanded {
        let can_copy = ed.selected_custom.is_some()
            || (ed.selected.is_some() && ed.selected != Some(Control::Move));
        let grid_lbl = if tc.settings.snap_to_grid {
            "Grid: ON"
        } else {
            "Grid"
        };
        let all_lbl = if ed.apply_all {
            "All: ON"
        } else {
            "Select All"
        };
        let items: [(Option<Icon>, &str, f32, bool, bool); 8] = [
            (Some(Icon::Undo), "", 38.0, !ed.undo_stack.is_empty(), false),
            (Some(Icon::Redo), "", 38.0, !ed.redo_stack.is_empty(), false),
            (Some(Icon::Gear), "Props", 64.0, true, ed.props_open),
            (Some(Icon::Check), all_lbl, 80.0, true, ed.apply_all),
            (Some(Icon::Crosshair), grid_lbl, 72.0, true, tc.settings.snap_to_grid),
            (Some(Icon::Plus), "Add", 56.0, true, false),
            (Some(Icon::Plus), "Copy", 58.0, can_copy, false),
            (Some(Icon::Check), "Save", 62.0, true, true),
        ];
        let mut ix = pod_x + 50.0;
        for (ic, lbl, iw, enabled, hi) in items {
            let r = [ix, pod_y + 5.0, iw, 38.0];
            let bg = if lbl == "Save" {
                accent
            } else if hi {
                accent_soft
            } else {
                bg_hi
            };
            ui.rounded(r[0], r[1], r[2], r[3], 7.0, bg);
            ui.border(
                r[0],
                r[1],
                r[2],
                r[3],
                7.0,
                if hi || lbl == "Save" {
                    accent
                } else {
                    border_c
                },
            );
            let fg = if lbl == "Save" {
                accent_fg
            } else if !enabled {
                color::hex(0x5E5875, 200)
            } else if hi {
                accent
            } else {
                text_c
            };
            if let Some(icon) = ic {
                if lbl.is_empty() {
                    ui.icon(icon, r[0] + (iw - 18.0) * 0.5, r[1] + 10.0, 18.0, fg);
                } else {
                    ui.icon(icon, r[0] + 8.0, r[1] + 11.0, 15.0, fg);
                    ui.text(Face::SemiBold, 10.8, r[0] + 25.0, r[1] + 10.5, fg, lbl);
                }
            }
            ix += iw + 5.0;
        }
    }

    if let Some(msg) = ed
        .status_msg
        .as_deref()
        .or(notice)
        .filter(|s| !s.is_empty())
    {
        let mw = (ui.measure(Face::Medium, 10.5, msg) + 20.0).min(w - 20.0);
        let mx = (w - mw) * 0.5;
        let my = (pod_y + pod_h + 6.0).min(h - 48.0);
        ui.rounded(mx, my, mw, 22.0, 11.0, bg_card);
        ui.border(mx, my, mw, 22.0, 11.0, border_c);
        ui.centered_fit(
            Face::Medium,
            10.0,
            [mx + 6.0, my, mw - 12.0, 22.0],
            text_c,
            msg,
        );
    }

    // 6b. Unsaved Changes Back Confirmation Modal (when ed.back_confirm_open == true)
    if ed.back_confirm_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 205));
        let dw = (w - 36.0).clamp(290.0, 430.0);
        let dh = 178.0;
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
        ui.stroke_rounded(dx, dy, dw, dh, 18.0, 1.6, accent);
        ui.text(
            Face::SemiBold,
            14.0,
            dx + 18.0,
            dy + 16.0,
            text_c,
            "Unsaved Layout Changes",
        );
        ui.text(
            Face::Regular,
            10.5,
            dx + 18.0,
            dy + 42.0,
            muted_c,
            "You have modified your control layout in this session.",
        );
        ui.text(
            Face::Regular,
            10.0,
            dx + 18.0,
            dy + 60.0,
            muted_c,
            "Save your changes before leaving, or go back without saving?",
        );
        let bw = (dw - 44.0) * 0.5;
        let by = dy + dh - 50.0;
        let save_r = [dx + 18.0, by, bw, 36.0];
        let back_r = [dx + 26.0 + bw, by, bw, 36.0];
        ui.rounded(save_r[0], save_r[1], save_r[2], save_r[3], 10.0, accent);
        ui.centered_fit(Face::SemiBold, 11.5, save_r, accent_fg, "Save");
        ui.rounded(back_r[0], back_r[1], back_r[2], back_r[3], 10.0, bg_hi);
        ui.border(back_r[0], back_r[1], back_r[2], back_r[3], 10.0, danger_c);
        ui.centered_fit(Face::SemiBold, 11.5, back_r, danger_c, "Back");
        ui.finish();
        return;
    }

    // 7. Step 1 / Step 2 Mouse vs Keyboard Horizontal Picker Modal
    if ed.picker_step != PickerStep::None {
        draw_input_picker_modal(
            &mut ui,
            w,
            h,
            ed.picker_step,
            ed.kb_scroll_x,
            false,
            false,
            false,
        );
        ui.finish();
        return;
    }

    // 8. Component Property Inspector Dialog — Studio Layout matching `90511.png` (when ed.props_open == true)
    if ed.props_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 7, 15, 208));
        let dw = (w - 16.0).clamp(350.0, 640.0);
        let dh = (h - 12.0).clamp(256.0, 384.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;

        // Material Design 3 Expressive Tonal Surface Palette synchronized with active theme
        let st_outer = theme.bg_solid;
        let st_inner = theme.surface;
        let st_card = theme.surface_bright;
        let st_border = color::with_alpha(theme.border, 0.28);
        let st_cyan = theme.accent;
        let st_cyan_soft = theme.surface_selected;
        let st_cyan_fg = theme.accent_text;
        let st_text = theme.text_primary;
        let st_muted = theme.text_muted;

        ui.rounded(dx + 2.0, dy + 6.0, dw, dh, 12.0, rgba(0, 0, 0, 165));
        ui.rounded(dx, dy, dw, dh, 12.0, st_outer);
        ui.border(dx, dy, dw, dh, 12.0, st_border);

        let is_joystick = !ed.apply_all && ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
        let is_mcpe_mode = tc.settings.control_style == ControlStyle::Mcpe;
        let (header_txt, sub_txt) = if ed.apply_all {
            (
                "ALL CONTROLS — Select All ON".to_owned(),
                "Every color, opacity, shape, stroke, icon & size edit applies to all controls",
            )
        } else if is_joystick {
            (
                "Joystick & Analog Base Inspector".to_owned(),
                "Customize stick knob, analog base ring, active zone & visibility",
            )
        } else if let Some(idx) = ed.selected_custom {
            let lbl = tc
                .settings
                .custom_buttons
                .get(idx)
                .map(|b| b.label())
                .unwrap_or_else(|| "Button".into());
            if is_mcpe_mode {
                (
                    format!("MCPE Custom Button — {lbl}"),
                    "Multi-Event Mapping (up to 4 slots) • Icon, style & geometry",
                )
            } else {
                (
                    format!("GoyControl Button — {lbl}"),
                    "Customize visual style, icon badge, geometry, stroke & events",
                )
            }
        } else if let Some(c) = ed.selected {
            (
                format!("MCPE Component — {}", c.name()),
                "Customize style, icon, opacity, shape, stroke & visibility",
            )
        } else {
            (
                "Component Properties".to_owned(),
                "Configure control appearance & behavior",
            )
        };
        let cur_pl = if let Some(idx) = ed.selected_custom {
            tc.custom_placement(idx)
        } else if let Some(c) = ed.selected {
            tc.placement(c)
        } else {
            crate::touch::Placement::new(0.5, 0.5, 0.12)
        };
        let telem = format!(
            "X:{:.0}% Y:{:.0}%  •  Sz:{:.0}%",
            cur_pl.center[0] * 100.0,
            cur_pl.center[1] * 100.0,
            cur_pl.size * 100.0,
        );

        // Top Header Bar: [X] Close Button + Circular Double-Ring Preview Badge + Title/Sub + Telemetry Pill + [⛶] Center Snap + Cyan [✓ Done]
        let x_btn = [dx + 12.0, dy + 9.0, 26.0, 26.0];
        ui.rounded(x_btn[0], x_btn[1], x_btn[2], x_btn[3], 8.0, st_card);
        ui.border(x_btn[0], x_btn[1], x_btn[2], x_btn[3], 8.0, st_border);
        ui.icon(Icon::Close, x_btn[0] + 6.0, x_btn[1] + 6.0, 14.0, st_muted);

        // Circular double-ring live preview icon in header
        ui.rounded(dx + 44.0, dy + 8.0, 28.0, 28.0, 14.0, rgba(24, 28, 48, 255));
        ui.stroke_rounded(
            dx + 44.0,
            dy + 8.0,
            28.0,
            28.0,
            14.0,
            1.8,
            if is_joystick {
                tc.settings.joystick_style.base_stroke_color
            } else {
                cur_pl.stroke_color
            },
        );
        ui.rounded(
            dx + 51.0,
            dy + 15.0,
            14.0,
            14.0,
            7.0,
            if is_joystick {
                tc.settings.joystick_style.stick_color
            } else {
                cur_pl.button_color
            },
        );
        ui.stroke_rounded(dx + 51.0, dy + 15.0, 14.0, 14.0, 7.0, 1.4, st_cyan);

        ui.text_clipped(
            Face::SemiBold,
            12.8,
            dx + 78.0,
            dy + 7.0,
            (dw - 332.0).max(90.0),
            st_text,
            &header_txt,
        );
        ui.text_clipped(
            Face::Regular,
            8.8,
            dx + 78.0,
            dy + 23.0,
            (dw - 332.0).max(90.0),
            st_muted,
            sub_txt,
        );

        // Telemetry Pill + Delete [🗑] Button (for Joystick & Custom Buttons) + Center-Snap [⛶] Button + Cyan [✓ Done] Button
        let pill_w = 122.0;
        let pill_r = [dx + dw - 318.0, dy + 9.0, pill_w, 26.0];
        ui.rounded(pill_r[0], pill_r[1], pill_r[2], pill_r[3], 7.0, st_inner);
        ui.border(pill_r[0], pill_r[1], pill_r[2], pill_r[3], 7.0, st_border);
        ui.centered_fit(Face::Mono, 8.5, pill_r, st_text, &telem);

        if !is_joystick && (ed.selected_custom.is_some() || ed.selected.is_some()) {
            let copy_r = [dx + dw - 190.0, dy + 9.0, 46.0, 26.0];
            ui.rounded(
                copy_r[0],
                copy_r[1],
                copy_r[2],
                copy_r[3],
                6.0,
                color::with_alpha(st_cyan, 0.20),
            );
            ui.border(copy_r[0], copy_r[1], copy_r[2], copy_r[3], 6.0, st_cyan);
            ui.centered_fit(Face::SemiBold, 9.0, copy_r, st_cyan, "Copy");
        }

        if is_joystick || ed.selected_custom.is_some() {
            let del_r = [dx + dw - 138.0, dy + 9.0, 26.0, 26.0];
            ui.rounded(
                del_r[0],
                del_r[1],
                del_r[2],
                del_r[3],
                6.0,
                color::with_alpha(danger_c, 0.24),
            );
            ui.border(del_r[0], del_r[1], del_r[2], del_r[3], 6.0, danger_c);
            ui.icon(Icon::Trash, del_r[0] + 5.0, del_r[1] + 5.0, 16.0, danger_c);
        }

        let snap_r = [dx + dw - 106.0, dy + 9.0, 26.0, 26.0];
        ui.rounded(snap_r[0], snap_r[1], snap_r[2], snap_r[3], 6.0, st_card);
        ui.border(snap_r[0], snap_r[1], snap_r[2], snap_r[3], 6.0, st_border);
        ui.icon(
            Icon::Crosshair,
            snap_r[0] + 5.0,
            snap_r[1] + 5.0,
            16.0,
            st_cyan,
        );

        let close_r = [dx + dw - 74.0, dy + 9.0, 62.0, 26.0];
        ui.rounded(
            close_r[0], close_r[1], close_r[2], close_r[3], 7.0, st_cyan,
        );
        ui.icon(Icon::Check, close_r[0] + 6.0, close_r[1] + (close_r[3] - 11.0) * 0.5, 11.0, st_cyan_fg);
        ui.centered_fit(Face::SemiBold, 10.2, [close_r[0] + 15.0, close_r[1], close_r[2] - 17.0, close_r[3]], st_cyan_fg, "Done");

        // Full-Width M3 Workspace Area (single clean top segmented tab bar, no duplicate left rail)
        let wx = dx + 14.0;
        let ww = dw - 28.0;

        // Top 3-Segment Tab Bar with crisp Material 3 architectural corners
        let tab_y = dy + 46.0;
        let tab_w = (ww - 16.0) / 3.0;
        ui.rounded(wx, tab_y - 2.0, ww, 28.0, 8.0, st_inner);
        ui.border(wx, tab_y - 2.0, ww, 28.0, 8.0, st_border);
        let tab_labels = if is_joystick {
            ["1. Stick & Style", "2. Analog Base", "3. Zone & Visibility"]
        } else {
            [
                "1. Style & Colors",
                "2. Shape & Stroke",
                "3. Events & Behavior",
            ]
        };
        for (ti, t_lbl) in tab_labels.into_iter().enumerate() {
            let tx = wx + 4.0 + ti as f32 * (tab_w + 4.0);
            let on = ed.props_tab == ti;
            if on {
                ui.rounded(tx, tab_y, tab_w, 24.0, 6.0, st_cyan);
            }
            ui.centered_fit(
                Face::SemiBold,
                9.8,
                [tx + 4.0, tab_y, tab_w - 8.0, 24.0],
                if on { st_cyan_fg } else { st_muted },
                t_lbl,
            );
        }

        // Main Inner Studio Container Card (`[wx, dy + 80.0, ww, dh - 92.0]`)
        let box_y = dy + 80.0;
        let box_h = dh - 92.0;
        ui.rounded(wx, box_y, ww, box_h, 10.0, st_inner);
        ui.border(wx, box_y, ww, box_h, 10.0, st_border);

        let body_y = box_y + 10.0;
        if is_joystick {
            let js = &tc.settings.joystick_style;
            let p_move = tc.placement(Control::Move);
            let preview_w = 88.0;
            let cards_area_w = ww - preview_w - 30.0;
            let cw3 = (cards_area_w - 16.0) / 3.0;

            match ed.props_tab {
                0 => {
                    // Section Header + Right Style Selector Pill
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "STICK KNOB DIMENSIONS & STROKE",
                    );
                    let st_pill_w = 216.0f32.min(ww * 0.46);
                    let st_pill_r = [wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0];
                    ui.rounded(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    let st_lbl = if tc.settings.control_style == ControlStyle::GoyControl
                        || p_move.use_goy_style
                    {
                        "● Style: Custom GoyControl Stick  v"
                    } else {
                        "● Style: Default MCPE Stick  v"
                    };
                    ui.centered_fit(Face::SemiBold, 8.8, st_pill_r, st_cyan, st_lbl);

                    // 3 Studio Slider Cards + Right Live Component Preview Box with Corner Brackets
                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Stick Size|{:.0}%", js.stick_size * 100.0),
                            (js.stick_size - 0.20) / 0.65,
                        ),
                        (
                            format!("Stick Alpha|{:.0}%", js.stick_opacity * 100.0),
                            js.stick_opacity.clamp(0.0, 1.0),
                        ),
                        (
                            format!("Stick Stroke|{:.1}dp", js.stick_stroke),
                            js.stick_stroke / 8.0,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, cards_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }

                    // Live Joystick Preview Box with Cyan Corner Framing Brackets
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let b_side = 44.0;
                    let bx = prev_r[0] + (prev_r[2] - b_side) * 0.5;
                    let by = prev_r[1] + (prev_r[3] - b_side) * 0.5;
                    let b_rad = (b_side * (js.base_roundness / 100.0)).clamp(4.0, b_side * 0.5);
                    ui.rounded(bx, by, b_side, b_side, b_rad, js.base_color);
                    ui.stroke_rounded(
                        bx,
                        by,
                        b_side,
                        b_side,
                        b_rad,
                        js.base_stroke.clamp(1.0, 3.0),
                        js.base_stroke_color,
                    );
                    let k_side = (b_side * js.stick_size.clamp(0.25, 0.80)).clamp(14.0, 34.0);
                    let kx = bx + (b_side - k_side) * 0.5;
                    let ky = by + (b_side - k_side) * 0.5;
                    ui.rounded(kx, ky, k_side, k_side, k_side * 0.5, js.stick_color);
                    ui.stroke_rounded(
                        kx,
                        ky,
                        k_side,
                        k_side,
                        k_side * 0.5,
                        js.stick_stroke.clamp(1.0, 3.0),
                        js.stick_stroke_color,
                    );

                    // Bottom Swatch Card ("STICK COLOR" + Circular Swatches + Right Cycle Stroke Button)
                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "STICK COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            js.stick_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    let act_r = [
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ];
                    ui.rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, st_inner);
                    ui.stroke_rounded(
                        act_r[0],
                        act_r[1],
                        act_r[2],
                        act_r[3],
                        10.0,
                        1.5,
                        js.stick_stroke_color,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        8.8,
                        act_r,
                        st_text,
                        "● Cycle Stick Stroke Col",
                    );
                }
                1 => {
                    // Joystick Tab 1: Analog Base Ring Geometry, Mode & Base Palette
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "ANALOG BASE RING GEOMETRY & INPUT MODE",
                    );
                    let mode_pill_w = 186.0f32.min(ww * 0.42);
                    let mode_pill_r = [
                        wx + ww - mode_pill_w - 12.0,
                        body_y - 2.0,
                        mode_pill_w,
                        24.0,
                    ];
                    ui.rounded(
                        mode_pill_r[0],
                        mode_pill_r[1],
                        mode_pill_r[2],
                        mode_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        mode_pill_r[0],
                        mode_pill_r[1],
                        mode_pill_r[2],
                        mode_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    let mode_txt = if tc.settings.mode == crate::touch::Mode::Dpad {
                        "● Mode: D-Pad  v"
                    } else {
                        "● Mode: Analog Stick  v"
                    };
                    ui.centered_fit(Face::SemiBold, 8.8, mode_pill_r, st_cyan, mode_txt);

                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Base Alpha|{:.0}%", js.base_opacity * 100.0),
                            js.base_opacity.clamp(0.0, 1.0),
                        ),
                        (
                            format!("Base Stroke|{:.1}dp", js.base_stroke),
                            js.base_stroke / 8.0,
                        ),
                        (
                            format!("Base Round|{:.0}", js.base_roundness),
                            js.base_roundness / 50.0,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, cards_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }

                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let b_side = 46.0;
                    let bx = prev_r[0] + (prev_r[2] - b_side) * 0.5;
                    let by = prev_r[1] + (prev_r[3] - b_side) * 0.5;
                    let b_rad = (b_side * (js.base_roundness / 100.0)).clamp(2.0, b_side * 0.5);
                    ui.rounded(bx, by, b_side, b_side, b_rad, js.base_color);
                    ui.stroke_rounded(
                        bx,
                        by,
                        b_side,
                        b_side,
                        b_rad,
                        js.base_stroke.clamp(1.0, 3.5),
                        js.base_stroke_color,
                    );

                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "BASE COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            js.base_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    let act_r = [
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ];
                    ui.rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, st_inner);
                    ui.stroke_rounded(
                        act_r[0],
                        act_r[1],
                        act_r[2],
                        act_r[3],
                        10.0,
                        1.5,
                        js.base_stroke_color,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        8.8,
                        act_r,
                        st_text,
                        "● Cycle Base Stroke Col",
                    );
                }
                _ => {
                    // Joystick Tab 2: Active Zone, Footprint Size & Visibility Rules
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "ACTIVE TOUCH ZONE & VISIBILITY RULES",
                    );
                    let area_y = body_y + 26.0;
                    let hw = (ww - 24.0 - 8.0) * 0.5;
                    let area_txt = format!("Active Zone: {}", js.move_area.label());
                    ui.rounded(wx + 12.0, area_y, hw, 38.0, 11.0, st_card);
                    ui.border(wx + 12.0, area_y, hw, 38.0, 11.0, st_cyan);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.6,
                        [wx + 16.0, area_y, hw - 8.0, 38.0],
                        st_text,
                        &area_txt,
                    );
                    let sz_txt = format!("Joystick Footprint|{:.0}%", p_move.size * 100.0);
                    let sz_x = wx + 12.0 + hw + 8.0;
                    draw_m3_stepper_cell(
                        &mut ui,
                        [sz_x, area_y, hw, 38.0],
                        &sz_txt,
                        (p_move.size - 0.12) / 0.33,
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_border,
                        st_text,
                    );

                    let y6 = area_y + 48.0;
                    let col4_w = (ww - 24.0 - 12.0) / 4.0;
                    ui.rounded(
                        wx + 12.0,
                        y6,
                        col4_w,
                        34.0,
                        11.0,
                        if p_move.visible_in_game {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        wx + 12.0,
                        y6,
                        col4_w,
                        34.0,
                        11.0,
                        if p_move.visible_in_game {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        wx + 16.0,
                        y6 + 8.0,
                        18.0,
                        p_move.visible_in_game,
                        "In Game",
                    );

                    let vm_x = wx + 12.0 + col4_w + 4.0;
                    ui.rounded(
                        vm_x,
                        y6,
                        col4_w,
                        34.0,
                        11.0,
                        if p_move.visible_in_menu {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        vm_x,
                        y6,
                        col4_w,
                        34.0,
                        11.0,
                        if p_move.visible_in_menu {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        vm_x + 6.0,
                        y6 + 8.0,
                        18.0,
                        p_move.visible_in_menu,
                        "In Menu",
                    );

                    let pv_x = wx + 12.0 + (col4_w + 4.0) * 2.0;
                    ui.rounded(pv_x, y6, col4_w, 34.0, 11.0, st_cyan_soft);
                    ui.border(pv_x, y6, col4_w, 34.0, 11.0, st_cyan);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [pv_x + 4.0, y6, col4_w - 8.0, 34.0],
                        st_cyan,
                        "Preset Vis All",
                    );

                    let del_jx = wx + 12.0 + (col4_w + 4.0) * 3.0;
                    ui.rounded(
                        del_jx,
                        y6,
                        col4_w,
                        34.0,
                        11.0,
                        color::with_alpha(danger_c, 0.24),
                    );
                    ui.border(del_jx, y6, col4_w, 34.0, 11.0, danger_c);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.4,
                        [del_jx + 4.0, y6, col4_w - 8.0, 34.0],
                        danger_c,
                        "Delete Joystick",
                    );
                }
            }
        } else {
            let p = if let Some(idx) = ed.selected_custom {
                tc.custom_placement(idx)
            } else if let Some(c) = ed.selected {
                tc.placement(c)
            } else {
                crate::touch::Placement::new(0.5, 0.5, 0.12)
            };
            let cur_hint = if ed.editing_button_label {
                format!("{}|", ed.button_label_input)
            } else if let Some(idx) = ed.selected_custom {
                tc.settings
                    .custom_buttons
                    .get(idx)
                    .map(|b| b.label())
                    .unwrap_or_default()
            } else if let Some(c) = ed.selected {
                tc.settings.control_label(c).to_owned()
            } else {
                String::new()
            };
            let preview_w = 88.0;
            let cards_area_w = ww - preview_w - 30.0;
            let cw3 = (cards_area_w - 16.0) / 3.0;

            match ed.props_tab {
                0 => {
                    // Button Tab 0: Studio Style Header + 3 Cards (Transparency, Hint Text, Icon Badge) + Right Live Preview + Bottom Color Swatch Bar
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "BUTTON STYLE, ICON BADGE & OPACITY",
                    );
                    let is_goy =
                        tc.settings.control_style == ControlStyle::GoyControl || p.use_goy_style;
                    let st_pill_w = 216.0f32.min(ww * 0.46);
                    let st_pill_r = [wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0];
                    ui.rounded(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        st_pill_r[0],
                        st_pill_r[1],
                        st_pill_r[2],
                        st_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    let st_lbl = if is_goy {
                        "● Style: Static (GoyControl)  v"
                    } else {
                        "● Style: Default (MCPE)  v"
                    };
                    ui.centered_fit(Face::SemiBold, 8.8, st_pill_r, st_cyan, st_lbl);

                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    let icon_lbl = control_icon_label(p.icon_id);
                    draw_m3_stepper_cell(
                        &mut ui,
                        [wx + 12.0, cards_y, cw3, card_h],
                        &format!("Opacity|{:.0}%", p.opacity * 100.0),
                        p.opacity.clamp(0.0, 1.0),
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_border,
                        st_text,
                    );
                    draw_m3_selector_cell(
                        &mut ui,
                        [wx + 12.0 + (cw3 + 8.0), cards_y, cw3, card_h],
                        "Icon Badge",
                        icon_lbl,
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_text,
                        st_muted,
                    );
                    draw_m3_selector_cell(
                        &mut ui,
                        [wx + 12.0 + (cw3 + 8.0) * 2.0, cards_y, cw3, card_h],
                        if ed.editing_button_label { "Typing Name..." } else { "Button Name (Edit)" },
                        &cur_hint,
                        st_cyan,
                        st_cyan_soft,
                        st_card,
                        st_text,
                        st_muted,
                    );

                    // Live Button Preview Box with Cyan Corner Framing Brackets (`⌜ ⌝ ⌞ ⌟`)
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let bw_p = (38.0 * p.width_ratio.clamp(0.6, 1.45)).clamp(26.0, 56.0);
                    let bh_p = (38.0 * p.height_ratio.clamp(0.6, 1.35)).clamp(24.0, 48.0);
                    let bx_p = prev_r[0] + (prev_r[2] - bw_p) * 0.5;
                    let by_p = prev_r[1] + (prev_r[3] - bh_p) * 0.5;
                    let prev_alpha = p.opacity.clamp(0.0, 1.0);
                    ui.goy_box(
                        bx_p,
                        by_p,
                        bw_p,
                        bh_p,
                        p.shape,
                        p.roundness,
                        color::with_alpha(p.button_color, prev_alpha),
                        p.stroke_size.clamp(0.0, 6.0),
                        p.stroke_margin,
                        p.stroke_roundness,
                        color::with_alpha(p.stroke_color, prev_alpha),
                        p.shadow_size,
                        color::with_alpha(p.shadow_color, prev_alpha),
                    );
                    draw_button_icon_and_label(
                        &mut ui,
                        [bx_p, by_p, bw_p, bh_p],
                        p.icon_id,
                        &cur_hint,
                        color::with_alpha(st_text, prev_alpha),
                    );

                    // Bottom Swatch Card ("BUTTON COLOR" + Circular Swatches + onClick/onToggle Cycle Pills)
                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "BUTTON COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            p.button_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 158.0f32.min(ww * 0.36);
                    let act_x = wx + ww - act_w - 20.0;
                    let half_h = (sw_card_h - 14.0) * 0.5;
                    let clk_r = [act_x, sw_y + 5.0, act_w, half_h];
                    let tog_r = [act_x, sw_y + 9.0 + half_h, act_w, half_h];
                    ui.rounded(
                        clk_r[0],
                        clk_r[1],
                        clk_r[2],
                        clk_r[3],
                        7.0,
                        p.on_click_color,
                    );
                    ui.stroke_rounded(
                        clk_r[0],
                        clk_r[1],
                        clk_r[2],
                        clk_r[3],
                        7.0,
                        1.3,
                        p.on_click_stroke_color,
                    );
                    ui.centered_fit(Face::SemiBold, 8.2, clk_r, st_text, "onClick Col (Cycle)");
                    ui.rounded(
                        tog_r[0],
                        tog_r[1],
                        tog_r[2],
                        tog_r[3],
                        7.0,
                        p.on_toggle_color,
                    );
                    ui.stroke_rounded(
                        tog_r[0],
                        tog_r[1],
                        tog_r[2],
                        tog_r[3],
                        7.0,
                        1.3,
                        p.on_toggle_stroke_color,
                    );
                    ui.centered_fit(Face::SemiBold, 8.2, tog_r, st_text, "onToggle Col (Cycle)");
                }
                1 => {
                    // Button Tab 1: Shape, Stroke, Roundness & Aspect Ratios + Live Preview + Stroke Color Swatches
                    ui.text(
                        Face::SemiBold,
                        9.6,
                        wx + 12.0,
                        body_y + 3.0,
                        st_text,
                        "SHAPE GEOMETRY, STROKE & ASPECT RATIO",
                    );
                    let sh_pill_w = 192.0f32.min(ww * 0.44);
                    let sh_pill_r = [wx + ww - sh_pill_w - 12.0, body_y - 2.0, sh_pill_w, 24.0];
                    ui.rounded(
                        sh_pill_r[0],
                        sh_pill_r[1],
                        sh_pill_r[2],
                        sh_pill_r[3],
                        12.0,
                        st_card,
                    );
                    ui.border(
                        sh_pill_r[0],
                        sh_pill_r[1],
                        sh_pill_r[2],
                        sh_pill_r[3],
                        12.0,
                        st_cyan,
                    );
                    ui.centered_fit(
                        Face::SemiBold,
                        8.8,
                        sh_pill_r,
                        st_cyan,
                        &format!("● Shape: {}  v", shape_name(p.shape)),
                    );

                    let cards_y = body_y + 28.0;
                    let card_h = 35.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Stroke|{:.1}dp", p.stroke_size),
                            p.stroke_size / 8.0,
                        ),
                        (format!("Roundness|{:.0}", p.roundness), p.roundness / 50.0),
                        (
                            format!("Shadow|{:.1}dp", p.shadow_size),
                            p.shadow_size / 12.0,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, cards_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }
                    let row2_y = cards_y + 39.0;
                    for (i, (txt, frac)) in [
                        (
                            format!("Scale|{:.0}%", p.size * 100.0),
                            (p.size - 0.05) / 0.39,
                        ),
                        (
                            format!("Width|{:.0}%", p.width_ratio * 100.0),
                            (p.width_ratio - 0.50) / 1.50,
                        ),
                        (
                            format!("Height|{:.0}%", p.height_ratio * 100.0),
                            (p.height_ratio - 0.50) / 1.50,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        draw_m3_stepper_cell(
                            &mut ui,
                            [rx, row2_y, cw3, card_h],
                            &txt,
                            frac,
                            st_cyan,
                            st_cyan_soft,
                            st_card,
                            st_border,
                            st_text,
                        );
                    }

                    // Live Shape & Stroke Preview Box
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, 74.0];
                    ui.rounded(
                        prev_r[0],
                        prev_r[1],
                        prev_r[2],
                        prev_r[3],
                        12.0,
                        rgba(9, 14, 24, 250),
                    );
                    ui.border(prev_r[0], prev_r[1], prev_r[2], prev_r[3], 12.0, st_border);
                    draw_preview_brackets(&mut ui, prev_r, st_cyan);
                    let bw_p = (38.0 * p.width_ratio.clamp(0.6, 1.45)).clamp(26.0, 56.0);
                    let bh_p = (38.0 * p.height_ratio.clamp(0.6, 1.35)).clamp(24.0, 48.0);
                    let bx_p = prev_r[0] + (prev_r[2] - bw_p) * 0.5;
                    let by_p = prev_r[1] + (prev_r[3] - bh_p) * 0.5;
                    let prev_alpha = p.opacity.clamp(0.0, 1.0);
                    ui.goy_box(
                        bx_p,
                        by_p,
                        bw_p,
                        bh_p,
                        p.shape,
                        p.roundness,
                        color::with_alpha(p.button_color, prev_alpha),
                        p.stroke_size.clamp(0.0, 6.0),
                        p.stroke_margin,
                        p.stroke_roundness,
                        color::with_alpha(p.stroke_color, prev_alpha),
                        p.shadow_size,
                        color::with_alpha(p.shadow_color, prev_alpha),
                    );
                    draw_button_icon_and_label(
                        &mut ui,
                        [bx_p, by_p, bw_p, bh_p],
                        p.icon_id,
                        &cur_hint,
                        color::with_alpha(st_text, prev_alpha),
                    );

                    // Bottom Stroke Color Swatch Strip
                    let sw_y = cards_y + 74.0 + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    ui.rounded(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_card);
                    ui.border(wx + 12.0, sw_y, ww - 24.0, sw_card_h, 12.0, st_border);
                    ui.text(
                        Face::SemiBold,
                        8.8,
                        wx + 22.0,
                        sw_y + 6.0,
                        st_muted,
                        "STROKE COLOR",
                    );
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        draw_m3_swatch_chip(
                            &mut ui,
                            sx,
                            sw_row_y,
                            sw_d,
                            sw_d,
                            *col,
                            p.stroke_color == *col,
                            st_cyan,
                            st_text,
                        );
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    let act_r = [
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ];
                    ui.rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, st_inner);
                    ui.stroke_rounded(act_r[0], act_r[1], act_r[2], act_r[3], 10.0, 1.5, st_cyan);
                    ui.centered_fit(Face::SemiBold, 8.8, act_r, st_text, "Reset Shape & Scale");
                }
                _ => {
                    // Button Tab 2: Events, Behavior Toggles & Visibility Checkboxes
                    let ew = (ww - 24.0 - 30.0) / 4.0;
                    for slot in 0..4 {
                        let ex = wx + 12.0 + slot as f32 * (ew + 10.0);
                        let tok_name = if let Some(idx) = ed.selected_custom {
                            tc.settings
                                .custom_buttons
                                .get(idx)
                                .map(|b| {
                                    crate::touch::custom::friendly_token_name(b.event_slot(slot))
                                })
                                .unwrap_or_else(|| "Unassigned".into())
                        } else if let Some(c) = ed.selected {
                            if slot == 0 {
                                crate::touch::custom::friendly_token_name(c.default_binding_token())
                            } else {
                                "Unassigned".to_owned()
                            }
                        } else {
                            "Unassigned".into()
                        };
                        let has_binding = tok_name != "Unassigned" && tok_name != "None";
                        ui.rounded(
                            ex,
                            body_y,
                            ew,
                            42.0,
                            8.0,
                            if has_binding { st_cyan_soft } else { st_card },
                        );
                        ui.border(
                            ex,
                            body_y,
                            ew,
                            42.0,
                            8.0,
                            if has_binding { st_cyan } else { st_border },
                        );
                        ui.centered_fit(
                            Face::SemiBold,
                            8.8,
                            [ex + 6.0, body_y + 4.0, ew - 12.0, 16.0],
                            if has_binding { st_cyan } else { st_muted },
                            &format!("EVENT SLOT #{}", slot + 1),
                        );
                        ui.centered_fit(
                            Face::SemiBold,
                            10.2,
                            [ex + 6.0, body_y + 20.0, ew - 12.0, 18.0],
                            st_text,
                            &tok_name,
                        );
                    }

                    let by = body_y + 54.0;
                    let bw = (ww - 24.0 - 30.0) / 4.0;
                    let is_vis = if let Some(c) = ed.selected {
                        tc.visible(c)
                    } else {
                        true
                    };
                    for (i, (lbl, on, danger)) in [
                        (
                            if p.passthrough {
                                "Pass-Thru: ON"
                            } else {
                                "Pass-Thru: OFF"
                            },
                            p.passthrough,
                            false,
                        ),
                        (
                            if p.toggleable {
                                "Mode: Toggle"
                            } else {
                                "Mode: Hold"
                            },
                            p.toggleable,
                            false,
                        ),
                        (
                            if p.swipeable {
                                "Swipe: ON"
                            } else {
                                "Swipe: OFF"
                            },
                            p.swipeable,
                            false,
                        ),
                        (
                            if ed.selected_custom.is_some() {
                                "Delete Button"
                            } else if is_vis {
                                "Visible: YES"
                            } else {
                                "Visible: NO"
                            },
                            is_vis,
                            ed.selected_custom.is_some(),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rx = wx + 12.0 + i as f32 * (bw + 10.0);
                        ui.rounded(
                            rx,
                            by,
                            bw,
                            30.0,
                            7.0,
                            if danger {
                                color::with_alpha(danger_c, 0.24)
                            } else if on {
                                st_cyan_soft
                            } else {
                                st_card
                            },
                        );
                        ui.border(
                            rx,
                            by,
                            bw,
                            30.0,
                            7.0,
                            if danger {
                                danger_c
                            } else if on {
                                st_cyan
                            } else {
                                st_border
                            },
                        );
                        ui.centered_fit(
                            Face::SemiBold,
                            9.4,
                            [rx + 4.0, by, bw - 8.0, 30.0],
                            if danger {
                                danger_c
                            } else if on {
                                st_cyan
                            } else {
                                st_text
                            },
                            lbl,
                        );
                    }

                    let vy = by + 42.0;
                    let col4_w = (ww - 24.0 - 30.0) / 4.0;
                    ui.rounded(
                        wx + 12.0,
                        vy,
                        col4_w,
                        30.0,
                        7.0,
                        if p.visible_in_game {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        wx + 12.0,
                        vy,
                        col4_w,
                        30.0,
                        7.0,
                        if p.visible_in_game {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        wx + 18.0,
                        vy + 6.0,
                        18.0,
                        p.visible_in_game,
                        "In Game",
                    );

                    let vm_x = wx + 12.0 + col4_w + 10.0;
                    ui.rounded(
                        vm_x,
                        vy,
                        col4_w,
                        30.0,
                        7.0,
                        if p.visible_in_menu {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        vm_x,
                        vy,
                        col4_w,
                        30.0,
                        7.0,
                        if p.visible_in_menu {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        vm_x + 6.0,
                        vy + 6.0,
                        18.0,
                        p.visible_in_menu,
                        "In Menu",
                    );

                    let gs_x = wx + 12.0 + (col4_w + 10.0) * 2.0;
                    ui.rounded(
                        gs_x,
                        vy,
                        col4_w,
                        30.0,
                        7.0,
                        if tc.settings.snap_to_grid {
                            st_cyan_soft
                        } else {
                            st_card
                        },
                    );
                    ui.border(
                        gs_x,
                        vy,
                        col4_w,
                        30.0,
                        7.0,
                        if tc.settings.snap_to_grid {
                            st_cyan
                        } else {
                            st_border
                        },
                    );
                    ui.m3_checkbox(
                        gs_x + 6.0,
                        vy + 6.0,
                        18.0,
                        tc.settings.snap_to_grid,
                        "Snap Grid",
                    );

                    let pv_x = wx + 12.0 + (col4_w + 10.0) * 3.0;
                    ui.rounded(pv_x, vy, col4_w, 30.0, 7.0, st_cyan_soft);
                    ui.border(pv_x, vy, col4_w, 30.0, 7.0, st_cyan);
                    ui.centered_fit(
                        Face::SemiBold,
                        9.2,
                        [pv_x + 4.0, vy, col4_w - 8.0, 30.0],
                        st_cyan,
                        "Preset Vis All",
                    );
                }
            }
        }
        ui.finish();
        return;
    }

    // 8a. Duplicate Preset Name Confirmation Modal (when ed.duplicate_name_confirm_open == true)
    if ed.duplicate_name_confirm_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 210));
        let dw = (w - 36.0).clamp(300.0, 440.0);
        let dh = 184.0;
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
        ui.stroke_rounded(dx, dy, dw, dh, 18.0, 1.6, accent);
        let conflict_name = ed
            .duplicate_conflict_target
            .as_ref()
            .map_or(ed.save_name_input.trim(), |(_, n, _)| n.as_str());
        ui.text(
            Face::SemiBold,
            13.8,
            dx + 18.0,
            dy + 16.0,
            text_c,
            "Replace Existing Control Preset?",
        );
        ui.text(
            Face::Regular,
            10.2,
            dx + 18.0,
            dy + 44.0,
            muted_c,
            &format!(
                "A control preset named '{}' already exists in {}.",
                conflict_name,
                tc.settings.control_style.label()
            ),
        );
        ui.text(
            Face::Regular,
            9.8,
            dx + 18.0,
            dy + 64.0,
            muted_c,
            "Controls with the same name are not allowed. Save to replace the current one?",
        );
        let bw = (dw - 44.0) * 0.5;
        let by = dy + dh - 50.0;
        let save_r = [dx + 18.0, by, bw, 36.0];
        let cancel_r = [dx + 26.0 + bw, by, bw, 36.0];
        ui.rounded(save_r[0], save_r[1], save_r[2], save_r[3], 10.0, accent);
        ui.centered_fit(Face::SemiBold, 11.5, save_r, accent_fg, "Save");
        ui.rounded(cancel_r[0], cancel_r[1], cancel_r[2], cancel_r[3], 10.0, bg_hi);
        ui.border(cancel_r[0], cancel_r[1], cancel_r[2], cancel_r[3], 10.0, border_c);
        ui.centered_fit(Face::SemiBold, 11.5, cancel_r, text_c, "Cancel");
        ui.finish();
        return;
    }

    // 8b. Independent Control Editor Save Workflow Dialog (Option A: Replace / Option B: Create New)
    if ed.save_dialog_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 195));
        let dw = (w - 24.0).clamp(310.0, 520.0);
        let dh = (h - 14.0).clamp(250.0, 360.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
        ui.border(dx, dy, dw, dh, 18.0, accent);

        ui.text(
            Face::SemiBold,
            13.0,
            dx + 14.0,
            dy + 10.0,
            text_c,
            &format!(
                "Save {} Layout — {}",
                tc.settings.control_style.label(),
                tc.settings.preset_name
            ),
        );

        let discard_r = [dx + dw - 166.0, dy + 8.0, 84.0, 24.0];
        ui.rounded(
            discard_r[0],
            discard_r[1],
            discard_r[2],
            discard_r[3],
            7.0,
            bg_hi,
        );
        ui.border(
            discard_r[0],
            discard_r[1],
            discard_r[2],
            discard_r[3],
            7.0,
            danger_c,
        );
        ui.centered_fit(Face::SemiBold, 9.5, discard_r, danger_c, "Discard & Exit");

        let cancel_r = [dx + dw - 76.0, dy + 8.0, 62.0, 24.0];
        ui.rounded(
            cancel_r[0],
            cancel_r[1],
            cancel_r[2],
            cancel_r[3],
            7.0,
            bg_hi,
        );
        ui.border(
            cancel_r[0],
            cancel_r[1],
            cancel_r[2],
            cancel_r[3],
            7.0,
            border_c,
        );
        ui.centered_fit(Face::SemiBold, 9.5, cancel_r, text_c, "Cancel");

        // Option A: Replace existing preset (disabled/protected when Default is active)
        let is_def = tc.settings.is_default_preset();
        let opt_a_r = [dx + 14.0, dy + 38.0, dw - 28.0, 44.0];
        ui.rounded(
            opt_a_r[0],
            opt_a_r[1],
            opt_a_r[2],
            opt_a_r[3],
            10.0,
            if is_def {
                rgba(24, 21, 36, 180)
            } else {
                accent_soft
            },
        );
        ui.border(
            opt_a_r[0],
            opt_a_r[1],
            opt_a_r[2],
            opt_a_r[3],
            10.0,
            if is_def { border_c } else { accent },
        );
        ui.text(
            Face::SemiBold,
            10.5,
            opt_a_r[0] + 10.0,
            opt_a_r[1] + 6.0,
            if is_def { muted_c } else { text_c },
            if is_def {
                "Option A: Replace existing preset (Protected)"
            } else {
                "Option A: Replace existing preset"
            },
        );
        let opt_a_sub = if is_def {
            "Default layout cannot be overwritten. Please create a new preset below.".to_owned()
        } else {
            format!(
                "Update '{}' in controls/{}/",
                tc.settings.preset_name,
                tc.settings.control_style.storage_subdir()
            )
        };
        ui.text(
            Face::Regular,
            9.0,
            opt_a_r[0] + 10.0,
            opt_a_r[1] + 23.0,
            muted_c,
            &opt_a_sub,
        );
        let rep_btn_r = [dx + dw - 118.0, dy + 46.0, 96.0, 28.0];
        ui.rounded(
            rep_btn_r[0],
            rep_btn_r[1],
            rep_btn_r[2],
            rep_btn_r[3],
            8.0,
            if is_def { bg_hi } else { accent },
        );
        ui.centered_fit(
            Face::SemiBold,
            9.5,
            rep_btn_r,
            if is_def { muted_c } else { accent_fg },
            if is_def {
                "Protected"
            } else {
                "Save"
            },
        );

        // Option B: Create a new preset as...
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            dy + 86.0,
            accent,
            "OPTION B: CREATE A NEW PRESET AS...",
        );
        let iy = dy + 100.0;
        let nw = (dw - 28.0 - 8.0) * 0.44;
        let desc_w = (dw - 28.0 - 8.0) - nw;
        let name_box = [dx + 14.0, iy, nw, 46.0];
        let desc_box = [dx + 14.0 + nw + 8.0, iy, desc_w, 46.0];
        let name_focus = !ed.editing_save_desc;
        ui.text_field(
            name_box[0],
            name_box[1],
            name_box[2],
            name_box[3],
            "NEW PRESET NAME",
            ed.save_name_input.as_str(),
            "Tap to type",
            name_focus,
        );
        ui.text_field(
            desc_box[0],
            desc_box[1],
            desc_box[2],
            desc_box[3],
            "DESCRIPTION",
            ed.save_desc_input.as_str(),
            "Tap to type",
            !name_focus,
        );
        ui.hint_line(
            dx + 14.0,
            iy + 56.0,
            dw - 28.0,
            "Tap Name or Desc to type; press Enter, then tap Create New Preset.",
            muted_c,
        );

        let create_btn_r = [dx + 14.0, dy + dh - 34.0, dw - 28.0, 26.0];
        ui.rounded(
            create_btn_r[0],
            create_btn_r[1],
            create_btn_r[2],
            create_btn_r[3],
            8.0,
            accent,
        );
        ui.centered_fit(
            Face::SemiBold,
            10.5,
            create_btn_r,
            accent_fg,
            "Save and Exit",
        );

        ui.finish();
        return;
    }

    // 9. Main Floating Control Manager Dialog (when ed.menu_open == true)
    if ed.menu_open {
        ui.fill(0.0, 0.0, w, h, rgba(4, 5, 10, 175));
        let dw = (w - 24.0).clamp(300.0, 540.0);
        let dh = (h - 18.0).clamp(245.0, 350.0);
        let dx = (w - dw) * 0.5;
        let dy = (h - dh) * 0.5;
        ui.rounded(dx, dy, dw, dh, 18.0, bg_card);
        ui.border(dx, dy, dw, dh, 18.0, border_c);
        ui.rounded(dx + 12.0, dy + 10.0, 26.0, 26.0, 8.0, accent_soft);
        ui.icon(
            if tc.settings.control_style == ControlStyle::Mcpe {
                Icon::HorizonLogo
            } else {
                Icon::MenuEquals
            },
            dx + 16.0,
            dy + 14.0,
            18.0,
            accent,
        );
        ui.text(
            Face::SemiBold,
            13.0,
            dx + 44.0,
            dy + 13.0,
            text_c,
            &format!(
                "{} Editor — Preset: {}",
                tc.settings.control_style.label(),
                tc.settings.preset_name
            ),
        );

        let resume_r = [dx + dw - 76.0, dy + 10.0, 62.0, 26.0];
        ui.rounded(
            resume_r[0],
            resume_r[1],
            resume_r[2],
            resume_r[3],
            8.0,
            bg_hi,
        );
        ui.border(
            resume_r[0],
            resume_r[1],
            resume_r[2],
            resume_r[3],
            8.0,
            border_c,
        );
        ui.centered_fit(Face::SemiBold, 10.5, resume_r, accent, "Resume");

        // Row 1: Save Workflow & Reset to Default for Active System
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            dy + 42.0,
            accent,
            &format!(
                "1. {} PRESET ACTIONS (STORAGE: controls/{}/)",
                tc.settings.control_style.section_title().to_uppercase(),
                tc.settings.control_style.storage_subdir()
            ),
        );
        let ey = dy + 56.0;
        let ew = (dw - 28.0 - 8.0) * 0.5;
        for (i, (title, primary)) in [
            ("Save Preset (Replace / Save As...)", true),
            ("Reset Canvas to Default Layout", false),
        ]
        .into_iter()
        .enumerate()
        {
            let rx = dx + 14.0 + i as f32 * (ew + 8.0);
            ui.rounded(rx, ey, ew, 30.0, 8.0, if primary { accent } else { bg_hi });
            ui.border(
                rx,
                ey,
                ew,
                30.0,
                8.0,
                if primary { accent } else { border_c },
            );
            ui.centered_fit(
                Face::SemiBold,
                10.0,
                [rx + 4.0, ey, ew - 8.0, 30.0],
                if primary { accent_fg } else { text_c },
                title,
            );
        }

        // Row 2: Presets belonging to this control system ONLY
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            ey + 36.0,
            accent,
            &format!(
                "2. LOAD {} PRESET",
                tc.settings.control_style.label().to_uppercase()
            ),
        );
        let py = ey + 50.0;
        let presets = crate::touch::Settings::list_presets(
            std::path::Path::new("."),
            tc.settings.control_style,
        );
        let max_show = presets.len().clamp(1, 5);
        let pw = (dw - 28.0 - 4.0 * (max_show as f32 - 1.0)) / max_show as f32;
        for (i, meta) in presets.iter().take(max_show).enumerate() {
            let rx = dx + 14.0 + i as f32 * (pw + 4.0);
            let active = tc.settings.preset_id.eq_ignore_ascii_case(&meta.id);
            ui.rounded(
                rx,
                py,
                pw,
                28.0,
                7.0,
                if active { accent_soft } else { bg_hi },
            );
            ui.border(
                rx,
                py,
                pw,
                28.0,
                7.0,
                if active { accent } else { border_c },
            );
            ui.centered_fit(
                Face::SemiBold,
                9.5,
                [rx + 3.0, py, pw - 6.0, 28.0],
                if active { accent } else { text_c },
                &meta.name,
            );
        }

        // Row 3: Global Editor & Movement Options
        ui.text(
            Face::SemiBold,
            9.5,
            dx + 14.0,
            py + 36.0,
            accent,
            "3. MOVEMENT, JOYSTICK AREA & GRID SNAP",
        );
        let gy = py + 50.0;
        let gw = (dw - 28.0 - 12.0) / 4.0;
        let joy_deleted = tc.settings.button_visibility[Control::Move as usize] == 2
            || !tc.placement(Control::Move).visible_in_game;
        let move_lbl = if joy_deleted {
            "+ Restore Joystick"
        } else {
            match tc.settings.mode {
                crate::touch::Mode::Dpad => "Move: D-Pad",
                crate::touch::Mode::Crosshair => "Move: Joystick",
                crate::touch::Mode::Tap => "Move: Tap Stick",
            }
        };
        let area_lbl = match tc.settings.joystick_style.move_area {
            JoystickMoveArea::MidBottomLeft => "Area: Mid-Left",
            JoystickMoveArea::RightSideAnywhere => "Area: Right Side",
            JoystickMoveArea::AbsoluteFixed => "Area: Fixed",
            JoystickMoveArea::FollowFinger => "Area: Follow",
        };
        for (i, (lbl, on)) in [
            (move_lbl, true),
            (area_lbl, true),
            (
                if tc.settings.snap_to_grid {
                    "Grid Snap: ON"
                } else {
                    "Grid Snap: OFF"
                },
                tc.settings.snap_to_grid,
            ),
            (
                if ed.show_hidden {
                    "Hidden: SHOWN"
                } else {
                    "Hidden: HIDE"
                },
                ed.show_hidden,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let rx = dx + 14.0 + i as f32 * (gw + 4.0);
            ui.rounded(rx, gy, gw, 28.0, 7.0, if on { accent_soft } else { bg_hi });
            ui.border(rx, gy, gw, 28.0, 7.0, if on { accent } else { border_c });
            ui.centered_fit(
                Face::Medium,
                9.5,
                [rx + 3.0, gy, gw - 6.0, 28.0],
                if on { accent } else { text_c },
                lbl,
            );
        }
    }

    ui.finish();
}

/// Handles touch input for the MCPE + GoyControl Editor (`draw_hud_editor`).
/// Returns `true` when the user completes saving or discarding to exit the editor.
fn apply_props_slider(
    tc: &mut crate::touch::TouchControls,
    ed: &crate::touch::Editor,
    tab: usize,
    index: usize,
    t: f32,
) {
    use crate::touch::Control;
    let t = t.clamp(0.0, 1.0);
    let is_joystick = ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
    let mode_idx = tc.settings.mode.index();
    if is_joystick {
        let slot = tc.settings.slot(Control::Move);
        match (tab, index) {
            (0, 0) => tc.settings.joystick_style.stick_size = (0.20 + t * 0.65).clamp(0.20, 0.85),
            (0, 1) => tc.settings.joystick_style.stick_opacity = ((t * 100.0).round() / 100.0).clamp(0.0, 1.0),
            (0, _) => {
                tc.settings.joystick_style.stick_stroke = ((t * 16.0).round() * 0.5).clamp(0.0, 8.0)
            }
            (1, 0) => tc.settings.joystick_style.base_opacity = ((t * 100.0).round() / 100.0).clamp(0.0, 1.0),
            (1, 1) => {
                tc.settings.joystick_style.base_stroke = ((t * 16.0).round() * 0.5).clamp(0.0, 8.0)
            }
            (1, _) => {
                tc.settings.joystick_style.base_roundness = (t * 50.0).round().clamp(0.0, 50.0)
            }
            (_, _) => {
                tc.settings.layouts[mode_idx][slot].size = (0.12 + t * 0.33).clamp(0.12, 0.45)
            }
        }
        return;
    }
    let apply_pl = |pl: &mut crate::touch::Placement| match (tab, index) {
        (0, 0) => pl.opacity = ((t * 100.0).round() / 100.0).clamp(0.0, 1.0),
        (1, 0) => pl.stroke_size = ((t * 16.0).round() * 0.5).clamp(0.0, 8.0),
        (1, 1) => {
            pl.roundness = (t * 50.0).round().clamp(0.0, 50.0);
            pl.stroke_roundness = pl.roundness;
        }
        (1, 2) => pl.shadow_size = (t * 12.0).round().clamp(0.0, 12.0),
        (1, 3) => pl.size = (0.05 + t * 0.39).clamp(0.05, 0.44),
        (1, 4) => pl.width_ratio = (0.50 + t * 1.50).clamp(0.50, 2.00),
        (1, 5) => pl.height_ratio = (0.50 + t * 1.50).clamp(0.50, 2.00),
        _ => {}
    };
    if ed.apply_all {
        for slot_pl in &mut tc.settings.layouts[mode_idx] {
            apply_pl(slot_pl);
        }
        for btn in &mut tc.settings.custom_buttons {
            for m in 0..3 {
                apply_pl(&mut btn.layouts[m]);
            }
        }
        match (tab, index) {
            (0, 0) => {
                let op = ((t * 100.0).round() / 100.0).clamp(0.0, 1.0);
                tc.settings.joystick_style.base_opacity = op;
                tc.settings.joystick_style.stick_opacity = op;
            }
            (1, 0) => {
                let st = ((t * 16.0).round() * 0.5).clamp(0.0, 8.0);
                tc.settings.joystick_style.base_stroke = st;
                tc.settings.joystick_style.stick_stroke = st;
            }
            (1, 1) => {
                tc.settings.joystick_style.base_roundness = (t * 50.0).round().clamp(0.0, 50.0);
            }
            _ => {}
        }
    } else if let Some(idx) = ed.selected_custom {
        if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
            for m in 0..3 {
                apply_pl(&mut btn.layouts[m]);
            }
        }
    } else if let Some(c) = ed.selected {
        let slot = tc.settings.slot(c);
        apply_pl(&mut tc.settings.layouts[mode_idx][slot]);
    }
}

pub fn touch_hud_editor(
    tc: &mut crate::touch::TouchControls,
    b: &mut HudBatch,
    a: &HudAtlas,
    id: u64,
    point: [f32; 2],
    started: bool,
    ended: bool,
) -> bool {
    use crate::touch::{Control, ControlStyle, CustomButton, EditorDragKind, Mode, PickerStep};
    let ui = crate::ui_theme::Ui::mobile(a, b, (0.0, 0.0));
    let (w, h) = (ui.w, ui.h);
    let p = [point[0] * w, point[1] * h];
    let inside =
        |r: [f32; 4]| p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3];

    let Some(mut ed) = tc.editor.take() else {
        return false;
    };
    clamp_editor_custom_selection(&mut ed, &tc.settings);

    let closed = (|| -> bool {
        // 1. Handle ongoing drag / resize / floating-pod move / horizontal keyboard scroll
        if !started {
            if let Some((drag_id, kind)) = ed.active_drag {
                if drag_id == id {
                    match kind {
                        EditorDragKind::FloatingPod { offset, mut moved } => {
                            let pod_w = if ed.pod_expanded { 568.0 } else { 52.0 };
                            let cur_x = (tc.settings.floating_menu_pos[0] * w)
                                .clamp(8.0, (w - pod_w - 8.0).max(8.0));
                            let cur_y = (tc.settings.floating_menu_pos[1] * h)
                                .clamp(6.0, (h - 54.0).max(6.0));
                            let nx = (p[0] - offset[0]).clamp(8.0, (w - pod_w - 8.0).max(8.0));
                            let ny = (p[1] - offset[1]).clamp(6.0, (h - 54.0).max(6.0));
                            if (nx - cur_x).hypot(ny - cur_y) > 4.0 {
                                moved = true;
                            }
                            if moved {
                                tc.settings.floating_menu_pos = [
                                    (nx / w.max(1.0)).clamp(0.04, 0.92),
                                    (ny / h.max(1.0)).clamp(0.02, 0.90),
                                ];
                            }
                            if ended {
                                ed.active_drag = None;
                                if !moved {
                                    ed.menu_open = !ed.menu_open;
                                    ed.props_open = false;
                                }
                                let _ = tc.settings.save();
                            } else {
                                ed.active_drag =
                                    Some((drag_id, EditorDragKind::FloatingPod { offset, moved }));
                            }
                        }
                        EditorDragKind::KeyboardScroll { last_x, mut moved } => {
                            let dx_k = p[0] - last_x;
                            if dx_k.abs() > 3.5 {
                                moved = true;
                            }
                            let dw = (w - 20.0).clamp(300.0, 620.0);
                            let max_kb_scroll = (880.0 - (dw - 28.0)).max(0.0);
                            ed.kb_scroll_x = (ed.kb_scroll_x - dx_k).clamp(0.0, max_kb_scroll);
                            if ended {
                                ed.active_drag = None;
                                if !moved
                                    && let PickerStep::Keyboard { event_slot } = ed.picker_step
                                {
                                    let dh = (h - 20.0).clamp(235.0, 340.0);
                                    let dx = (w - dw) * 0.5;
                                    let dy = (h - dh) * 0.5;
                                    let kb_top = dy + 40.0;
                                    let kb_h = dh - 56.0;
                                    let row_h = ((kb_h - 25.0) / 6.0).clamp(24.0, 38.0);
                                    let key_unit = 38.0;
                                    let gap = 4.0;
                                    for (r_idx, row) in KB_ROWS.iter().enumerate() {
                                        let ry = kb_top + r_idx as f32 * (row_h + 4.0);
                                        let mut kx = dx + 14.0 - ed.kb_scroll_x;
                                        for (lbl, tok, width_mult) in *row {
                                            let kw = key_unit * (*width_mult)
                                                + (*width_mult - 1.0).max(0.0) * gap;
                                            if inside([kx, ry, kw, row_h])
                                                && p[0] >= dx + 10.0
                                                && p[0] <= dx + dw - 10.0
                                            {
                                                push_undo_snapshot(&mut ed, &tc.settings);
                                                if let Some(idx) = ed.selected_custom
                                                    && let Some(btn) =
                                                        tc.settings.custom_buttons.get_mut(idx)
                                                {
                                                    btn.set_event_slot(
                                                        event_slot,
                                                        (*tok).to_owned(),
                                                    );
                                                    if event_slot == 0
                                                        && (btn.custom_label.is_empty()
                                                            || btn.custom_label == "BTN")
                                                    {
                                                        btn.custom_label = (*lbl).to_owned();
                                                    }
                                                    let _ = tc.settings.save();
                                                }
                                                ed.picker_step = PickerStep::None;
                                                ed.status_msg = Some(format!(
                                                    "Assigned key '{lbl}' to Event Slot #{}.",
                                                    event_slot + 1
                                                ));
                                                tc.clicked = true;
                                                break;
                                            }
                                            kx += kw + gap;
                                        }
                                    }
                                }
                            } else {
                                ed.active_drag = Some((
                                    drag_id,
                                    EditorDragKind::KeyboardScroll {
                                        last_x: p[0],
                                        moved,
                                    },
                                ));
                            }
                        }
                        EditorDragKind::Move { offset } => {
                            let aspect = tc.aspect();
                            let mut center = [point[0] - offset[0], point[1] - offset[1]];
                            if tc.settings.left_handed {
                                center[0] = 1.0 - center[0];
                            }
                            let mode_idx = tc.settings.mode.index();
                            let [_, _, cur_rw, cur_rh] = if let Some(idx) = ed.selected_custom {
                                tc.custom_placement(idx).rect(aspect)
                            } else if let Some(c) = ed.selected {
                                tc.placement(c).rect(aspect)
                            } else {
                                [0.0, 0.0, 0.12, 0.12]
                            };
                            if tc.settings.snap_to_grid {
                                let (snapped, _, _) = snap_button_to_neighbors_and_grid(
                                    tc,
                                    &ed,
                                    center,
                                    [cur_rw, cur_rh],
                                );
                                center = snapped;
                            }
                            if let Some(idx) = ed.selected_custom {
                                if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                                    let pl = &mut btn.layouts[mode_idx];
                                    let [_, _, rw, rh] = pl.rect(aspect);
                                    pl.center = [
                                        center[0].clamp(rw * 0.5, (1.0 - rw * 0.5).max(rw * 0.5)),
                                        center[1].clamp(rh * 0.5, (1.0 - rh * 0.5).max(rh * 0.5)),
                                    ];
                                }
                            } else if let Some(c) = ed.selected {
                                let slot = tc.settings.slot(c);
                                let pl = &mut tc.settings.layouts[mode_idx][slot];
                                let [_, _, rw, rh] = pl.rect(aspect);
                                pl.center = [
                                    center[0].clamp(rw * 0.5, (1.0 - rw * 0.5).max(rw * 0.5)),
                                    center[1].clamp(rh * 0.5, (1.0 - rh * 0.5).max(rh * 0.5)),
                                ];
                            }
                            if ended {
                                ed.active_drag = None;
                                let _ = tc.settings.save();
                            }
                        }
                        EditorDragKind::Resize {
                            mask,
                            start_pos,
                            start_size,
                            start_wr,
                            start_hr,
                        } => {
                            let dx = point[0] - start_pos[0];
                            let dy = point[1] - start_pos[1];
                            let mode_idx = tc.settings.mode.index();
                            let apply_resize = |pl: &mut crate::touch::Placement| {
                                if mask == 1 || mask == 2 {
                                    // Horizontal right/left edge handle: adjust width_ratio
                                    let dir = if mask == 1 { 1.0 } else { -1.0 };
                                    pl.width_ratio = (start_wr + dir * dx * 8.5).clamp(0.45, 3.0);
                                } else if mask == 4 || mask == 8 {
                                    // Vertical bottom/top edge handle: adjust height_ratio
                                    let dir = if mask == 4 { 1.0 } else { -1.0 };
                                    pl.height_ratio = (start_hr + dir * dy * 8.5).clamp(0.45, 3.0);
                                } else {
                                    // Corner handle: scale overall size + aspect ratio
                                    let sx = if (mask & 1) != 0 { dx } else { -dx };
                                    let sy = if (mask & 4) != 0 { dy } else { -dy };
                                    let diag = (sx + sy) * 0.85;
                                    pl.size = (start_size + diag).clamp(0.05, 0.44);
                                }
                            };
                            if let Some(idx) = ed.selected_custom {
                                if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                                    apply_resize(&mut btn.layouts[mode_idx]);
                                }
                            } else if let Some(c) = ed.selected {
                                let slot = tc.settings.slot(c);
                                apply_resize(&mut tc.settings.layouts[mode_idx][slot]);
                            }
                            if ended {
                                ed.active_drag = None;
                                let _ = tc.settings.save();
                            }
                        }
                        EditorDragKind::PropsSlider { tab, index, rect } => {
                            let pad = if rect[3] >= 52.0 { 12.0 } else { 10.0 };
                            let track_x = rect[0] + pad;
                            let track_w = (rect[2] - pad * 2.0).max(16.0);
                            let t = ((p[0] - track_x) / track_w).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, tab, index, t);
                            if ended {
                                ed.active_drag = None;
                                let _ = tc.settings.save();
                            }
                        }
                    }
                }
            }
            return false;
        }

        // 1a. If Unsaved Changes Back Confirmation Modal is open:
        if ed.back_confirm_open {
            let dw = (w - 36.0).clamp(290.0, 430.0);
            let dh = 178.0;
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            let bw = (dw - 44.0) * 0.5;
            let by = dy + dh - 50.0;
            let save_r = [dx + 18.0, by, bw, 36.0];
            let back_r = [dx + 26.0 + bw, by, bw, 36.0];
            if inside(save_r) {
                if tc.settings.is_default_preset() {
                    ed.back_confirm_open = false;
                    ed.save_dialog_open = true;
                    ed.editing_save_desc = false;
                    if ed.save_name_input.trim().is_empty() {
                        ed.save_name_input = format!("{} Custom", tc.settings.preset_name);
                    }
                    if ed.save_desc_input.trim().is_empty() {
                        ed.save_desc_input = tc.settings.preset_description.clone();
                    }
                    tc.clicked = true;
                    return false;
                }
                let _ = tc.settings.save_replace_preset(std::path::Path::new("."));
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }
            if inside(back_r) {
                tc.settings = ed.original.clone();
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }
            if !inside([dx, dy, dw, dh]) {
                ed.back_confirm_open = false;
                tc.clicked = true;
            }
            return false;
        }

        // 1a-bis. If Duplicate Preset Name Confirmation Modal is open (`Save` to replace vs `Cancel`):
        if ed.duplicate_name_confirm_open {
            let dw = (w - 36.0).clamp(300.0, 440.0);
            let dh = 184.0;
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            let bw = (dw - 44.0) * 0.5;
            let by = dy + dh - 50.0;
            let save_r = [dx + 18.0, by, bw, 36.0];
            let cancel_r = [dx + 26.0 + bw, by, bw, 36.0];
            if inside(save_r) {
                let (_, name, desc) = ed.duplicate_conflict_target.take().unwrap_or_else(|| {
                    (
                        String::new(),
                        ed.save_name_input.trim().to_owned(),
                        ed.save_desc_input.trim().to_owned(),
                    )
                });
                ed.duplicate_name_confirm_open = false;
                ed.save_dialog_open = false;
                let _ = tc
                    .settings
                    .save_or_replace_preset_by_name(std::path::Path::new("."), &name, &desc);
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }
            if inside(cancel_r) || !inside([dx, dy, dw, dh]) {
                ed.duplicate_name_confirm_open = false;
                ed.duplicate_conflict_target = None;
                tc.clicked = true;
            }
            return false;
        }

        // 1b. If Save Dialog is open (Option A: Replace existing / Option B: Create new preset as...)
        if ed.save_dialog_open {
            let dw = (w - 24.0).clamp(310.0, 520.0);
            let dh = (h - 14.0).clamp(250.0, 360.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;

            // Cancel button -> back to editor
            if inside([dx + dw - 76.0, dy + 8.0, 62.0, 24.0]) || !inside([dx, dy, dw, dh]) {
                ed.save_dialog_open = false;
                tc.clicked = true;
                return false;
            }
            // Discard & Exit -> reload saved preset from disk and exit editor
            if inside([dx + dw - 166.0, dy + 8.0, 84.0, 24.0]) {
                let cur_style = tc.settings.control_style;
                let cur_id = tc.settings.preset_id.clone();
                tc.settings
                    .select_preset(std::path::Path::new("."), cur_style, &cur_id);
                tc.clicked = true;
                return true;
            }

            // Option A: Replace existing preset
            let is_def = tc.settings.is_default_preset();
            if inside([dx + 14.0, dy + 38.0, dw - 28.0, 44.0]) {
                if is_def {
                    ed.status_msg = Some(
                        "Default preset is protected and cannot be replaced. Create a new preset below."
                            .into(),
                    );
                    tc.clicked = true;
                    return false;
                }
                let _ = tc.settings.save_replace_preset(std::path::Path::new("."));
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }

            // Option B: Focus Name vs Description input
            let iy = dy + 100.0;
            let nw = (dw - 28.0 - 8.0) * 0.44;
            let desc_w = (dw - 28.0 - 8.0) - nw;
            if inside([dx + 14.0, iy, nw, 46.0]) {
                ed.editing_save_desc = false;
                ed.keyboard_request = true;
                tc.clicked = true;
                return false;
            }
            if inside([dx + 14.0 + nw + 8.0, iy, desc_w, 46.0]) {
                ed.editing_save_desc = true;
                ed.keyboard_request = true;
                tc.clicked = true;
                return false;
            }

            // Option B: Create New Preset & Save (checks for duplicate name first)
            if inside([dx + 14.0, dy + dh - 34.0, dw - 28.0, 26.0]) {
                let name = if ed.save_name_input.trim().is_empty() {
                    format!("{} Custom", tc.settings.preset_name)
                } else {
                    ed.save_name_input.trim().to_owned()
                };
                let desc = if ed.save_desc_input.trim().is_empty() {
                    format!("Custom {} layout", tc.settings.control_style.label())
                } else {
                    ed.save_desc_input.trim().to_owned()
                };
                if let Some(existing) = crate::touch::Settings::find_preset_by_name(
                    std::path::Path::new("."),
                    tc.settings.control_style,
                    &name,
                ) {
                    ed.duplicate_name_confirm_open = true;
                    ed.duplicate_conflict_target = Some((existing.id, name, desc));
                    tc.clicked = true;
                    return false;
                }
                let _ = tc
                    .settings
                    .save_new_preset(std::path::Path::new("."), &name, &desc);
                let _ = tc.settings.save();
                tc.clicked = true;
                return true;
            }
            return false;
        }

        // 2. If Mouse vs Keyboard Picker Modal is open:
        if ed.picker_step != PickerStep::None {
            match ed.picker_step {
                PickerStep::AskInputType { event_slot } => {
                    let dw = (w - 36.0).clamp(270.0, 440.0);
                    let dh = 236.0;
                    let dx = (w - dw) * 0.5;
                    let dy = (h - dh) * 0.5;
                    let cw = (dw - 40.0) * 0.5;
                    if inside([dx + 16.0, dy + 54.0, cw, 76.0]) {
                        ed.picker_step = PickerStep::Keyboard { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 24.0 + cw, dy + 54.0, cw, 76.0]) {
                        ed.picker_step = PickerStep::MouseAndActions { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 16.0, dy + 138.0, dw - 32.0, 34.0]) {
                        ed.keyboard_request = true;
                        ed.status_msg =
                            Some("Press any key on your keyboard to bind it to this slot.".into());
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 16.0, dy + 180.0, cw, 36.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        if let Some(idx) = ed.selected_custom
                            && let Some(btn) = tc.settings.custom_buttons.get_mut(idx)
                        {
                            btn.set_event_slot(event_slot, String::new());
                            let _ = tc.settings.save();
                        }
                        ed.picker_step = PickerStep::None;
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + 24.0 + cw, dy + 180.0, cw, 36.0]) || !inside([dx, dy, dw, dh]) {
                        ed.picker_step = PickerStep::None;
                        tc.clicked = true;
                        return false;
                    }
                    return false;
                }
                PickerStep::Keyboard { event_slot } => {
                    let dw = (w - 20.0).clamp(300.0, 620.0);
                    let dh = (h - 20.0).clamp(235.0, 340.0);
                    let dx = (w - dw) * 0.5;
                    let dy = (h - dh) * 0.5;
                    let max_kb_scroll = (880.0 - (dw - 28.0)).max(0.0);
                    if inside([dx + dw - 266.0, dy + 8.0, 64.0, 26.0]) {
                        ed.keyboard_request = true;
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + dw - 196.0, dy + 8.0, 56.0, 26.0]) {
                        ed.kb_scroll_x = (ed.kb_scroll_x - 140.0).clamp(0.0, max_kb_scroll);
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + dw - 134.0, dy + 8.0, 56.0, 26.0]) {
                        ed.kb_scroll_x = (ed.kb_scroll_x + 140.0).clamp(0.0, max_kb_scroll);
                        tc.clicked = true;
                        return false;
                    }
                    if inside([dx + dw - 72.0, dy + 8.0, 58.0, 26.0]) || !inside([dx, dy, dw, dh]) {
                        ed.picker_step = PickerStep::AskInputType { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    ed.active_drag = Some((
                        id,
                        EditorDragKind::KeyboardScroll {
                            last_x: p[0],
                            moved: false,
                        },
                    ));
                    return false;
                }
                PickerStep::MouseAndActions { event_slot } => {
                    let dw = (w - 24.0).clamp(290.0, 560.0);
                    let dh = (h - 20.0).clamp(235.0, 355.0);
                    let dx = (w - dw) * 0.5;
                    let dy = (h - dh) * 0.5;
                    if inside([dx + dw - 72.0, dy + 8.0, 58.0, 26.0]) || !inside([dx, dy, dw, dh]) {
                        ed.picker_step = PickerStep::AskInputType { event_slot };
                        tc.clicked = true;
                        return false;
                    }
                    let cols = 3usize;
                    let rows = MOUSE_AND_ACTION_ITEMS.len().div_ceil(cols).max(1);
                    let gx = dx + 12.0;
                    let gy = dy + 40.0;
                    let gw = dw - 24.0;
                    let gh = dh - 50.0;
                    let cw = (gw - (cols as f32 - 1.0) * 6.0) / cols as f32;
                    let ch = ((gh - (rows as f32 - 1.0) * 4.0) / rows as f32).clamp(21.0, 32.0);
                    for (i, (lbl, tok)) in MOUSE_AND_ACTION_ITEMS.iter().enumerate() {
                        let col = i % cols;
                        let row = i / cols;
                        let cx = gx + col as f32 * (cw + 6.0);
                        let cy = gy + row as f32 * (ch + 4.0);
                        if inside([cx, cy, cw, ch]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            if let Some(idx) = ed.selected_custom
                                && let Some(btn) = tc.settings.custom_buttons.get_mut(idx)
                            {
                                btn.set_event_slot(event_slot, (*tok).to_owned());
                                if *tok == "mouse.look" {
                                    for l in &mut btn.layouts {
                                        l.passthrough = true;
                                    }
                                }
                                let _ = tc.settings.save();
                            }
                            ed.picker_step = PickerStep::None;
                            ed.status_msg = Some(format!(
                                "Assigned '{lbl}' to Event Slot #{}.",
                                event_slot + 1
                            ));
                            tc.clicked = true;
                            return false;
                        }
                    }
                    return false;
                }
                PickerStep::None => {}
            }
        }

        // 3. If Component Property Inspector Dialog is open (Studio Layout matching `90511.png`):
        if ed.props_open {
            let dw = (w - 16.0).clamp(350.0, 640.0);
            let dh = (h - 12.0).clamp(256.0, 384.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;

            // Close via top-left [X] button, top-right [✓ Done] button, or outside tap
            if inside([dx + 12.0, dy + 9.0, 26.0, 26.0])
                || inside([dx + dw - 74.0, dy + 9.0, 62.0, 26.0])
                || !inside([dx, dy, dw, dh])
            {
                ed.props_open = false;
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }
            // Header Copy button (duplicates the selected Custom Button or Built-in Button)
            let is_joy_sel = ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
            if !is_joy_sel
                && (ed.selected_custom.is_some() || ed.selected.is_some())
                && inside([dx + dw - 190.0, dy + 9.0, 46.0, 26.0])
            {
                duplicate_selected_editor_button(&mut ed, tc);
                tc.clicked = true;
                return false;
            }
            // Header Delete [🗑] button (deletes Joystick or selected Custom Button from any tab)
            if (is_joy_sel || ed.selected_custom.is_some())
                && inside([dx + dw - 138.0, dy + 9.0, 26.0, 26.0])
            {
                push_undo_snapshot(&mut ed, &tc.settings);
                if is_joy_sel {
                    let slot = tc.settings.slot(Control::Move);
                    tc.settings.button_visibility[Control::Move as usize] = 2;
                    for m in 0..3 {
                        tc.settings.layouts[m][slot].visible_in_game = false;
                        tc.settings.layouts[m][slot].visible_in_menu = false;
                    }
                    ed.selected = None;
                    if !tc.settings.custom_buttons.is_empty() {
                        ed.selected_custom = Some(0);
                    }
                    ed.props_open = false;
                    ed.status_msg = Some(
                        "Deleted Joystick. Open Control Manager (=) to restore it anytime.".into(),
                    );
                } else if let Some(idx) = ed.selected_custom {
                    if idx < tc.settings.custom_buttons.len() {
                        tc.settings.custom_buttons.remove(idx);
                    }
                    ed.selected_custom = None;
                    ed.selected = tc.visible(Control::Move).then_some(Control::Move);
                    ed.props_open = false;
                    ed.status_msg = Some("Deleted custom button.".into());
                }
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }
            // Center-Snap via telemetry pill OR [⛶] button
            if inside([dx + dw - 318.0, dy + 9.0, 122.0, 26.0])
                || inside([dx + dw - 106.0, dy + 9.0, 26.0, 26.0])
            {
                push_undo_snapshot(&mut ed, &tc.settings);
                let mode_idx = tc.settings.mode.index();
                if let Some(idx) = ed.selected_custom {
                    if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                        btn.layouts[mode_idx].center[0] = 0.5;
                    }
                } else if let Some(c) = ed.selected {
                    let slot = tc.settings.slot(c);
                    tc.settings.layouts[mode_idx][slot].center[0] = 0.5;
                }
                ed.status_msg = Some("Centered component horizontally (X = 50%).".into());
                let _ = tc.settings.save();
                tc.clicked = true;
                return false;
            }

            // Full-Width Workspace & Top 3-Segment Pill Bar clicks
            let wx = dx + 14.0;
            let ww = dw - 28.0;
            let tab_y = dy + 46.0;
            let tab_w = (ww - 8.0) / 3.0;
            for ti in 0..3 {
                let tx = wx + 2.0 + ti as f32 * (tab_w + 2.0);
                if inside([tx, tab_y - 2.0, tab_w, 28.0]) {
                    ed.props_tab = ti;
                    tc.clicked = true;
                    return false;
                }
            }

            let box_y = dy + 80.0;
            let box_h = dh - 92.0;
            let body_y = box_y + 10.0;
            let preview_w = 88.0;
            let cards_area_w = ww - preview_w - 30.0;
            let cw3 = (cards_area_w - 16.0) / 3.0;

            let is_joystick = ed.selected == Some(Control::Move) && ed.selected_custom.is_none();
            if is_joystick {
                let mode_idx = tc.settings.mode.index();
                let slot = tc.settings.slot(Control::Move);

                match ed.props_tab {
                    0 => {
                        let st_pill_w = 216.0f32.min(ww * 0.46);
                        if inside([wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].use_goy_style =
                                !tc.settings.layouts[mode_idx][slot].use_goy_style;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let cards_y = body_y + 28.0;
                        let card_h = 74.0;
                        for i in 0..3 {
                            let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                            let rect = [rx, cards_y, cw3, card_h];
                            if inside(rect) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let t =
                                    ((p[0] - (rx + 12.0)) / (cw3 - 24.0).max(16.0)).clamp(0.0, 1.0);
                                apply_props_slider(tc, &ed, 0, i, t);
                                ed.active_drag = Some((
                                    id,
                                    EditorDragKind::PropsSlider {
                                        tab: 0,
                                        index: i,
                                        rect,
                                    },
                                ));
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                        if inside(prev_r) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next();
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let sw_y = cards_y + card_h + 8.0;
                        let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                        let sw_d = 20.0;
                        let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                        for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                            let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                            if inside([sx, sw_row_y, sw_d, sw_d]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                tc.settings.joystick_style.stick_color = *col;
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let act_w = 152.0f32.min(ww * 0.34);
                        if inside([
                            wx + ww - act_w - 22.0,
                            sw_y + (sw_card_h - 28.0) * 0.5,
                            act_w,
                            28.0,
                        ]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let cur = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| {
                                    *c == tc.settings.joystick_style.stick_stroke_color
                                })
                                .unwrap_or(0);
                            tc.settings.joystick_style.stick_stroke_color =
                                COLOR_SWATCHES[(cur + 1) % COLOR_SWATCHES.len()].0;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    1 => {
                        let mode_pill_w = 186.0f32.min(ww * 0.42);
                        if inside([
                            wx + ww - mode_pill_w - 12.0,
                            body_y - 2.0,
                            mode_pill_w,
                            24.0,
                        ]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.mode = match tc.settings.mode {
                                Mode::Dpad => Mode::Crosshair,
                                Mode::Crosshair => Mode::Tap,
                                Mode::Tap => Mode::Dpad,
                            };
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let cards_y = body_y + 28.0;
                        let card_h = 74.0;
                        for i in 0..3 {
                            let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                            let rect = [rx, cards_y, cw3, card_h];
                            if inside(rect) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let t =
                                    ((p[0] - (rx + 12.0)) / (cw3 - 24.0).max(16.0)).clamp(0.0, 1.0);
                                apply_props_slider(tc, &ed, 1, i, t);
                                ed.active_drag = Some((
                                    id,
                                    EditorDragKind::PropsSlider {
                                        tab: 1,
                                        index: i,
                                        rect,
                                    },
                                ));
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let sw_y = cards_y + card_h + 8.0;
                        let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                        let sw_d = 20.0;
                        let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                        for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                            let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                            if inside([sx, sw_row_y, sw_d, sw_d]) {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                tc.settings.joystick_style.base_color = *col;
                                let _ = tc.settings.save();
                                tc.clicked = true;
                                return false;
                            }
                        }
                        let act_w = 152.0f32.min(ww * 0.34);
                        if inside([
                            wx + ww - act_w - 22.0,
                            sw_y + (sw_card_h - 28.0) * 0.5,
                            act_w,
                            28.0,
                        ]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let cur = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| {
                                    *c == tc.settings.joystick_style.base_stroke_color
                                })
                                .unwrap_or(0);
                            tc.settings.joystick_style.base_stroke_color =
                                COLOR_SWATCHES[(cur + 1) % COLOR_SWATCHES.len()].0;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    _ => {
                        let area_y = body_y + 26.0;
                        let hw = (ww - 24.0 - 8.0) * 0.5;
                        if inside([wx + 12.0, area_y, hw, 38.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next();
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let sz_x = wx + 12.0 + hw + 8.0;
                        let sz_rect = [sz_x, area_y, hw, 38.0];
                        if inside(sz_rect) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let t =
                                ((p[0] - (sz_x + 10.0)) / (hw - 20.0).max(16.0)).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, 2, 0, t);
                            ed.active_drag = Some((
                                id,
                                EditorDragKind::PropsSlider {
                                    tab: 2,
                                    index: 0,
                                    rect: sz_rect,
                                },
                            ));
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        let y6 = area_y + 48.0;
                        let col4_w = (ww - 24.0 - 12.0) / 4.0;
                        if inside([wx + 12.0, y6, col4_w, 34.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let next = !tc.settings.layouts[mode_idx][slot].visible_in_game;
                            tc.settings.layouts[mode_idx][slot].visible_in_game = next;
                            if next {
                                tc.settings.button_visibility[Control::Move as usize] = 1;
                            }
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        if inside([wx + 12.0 + col4_w + 4.0, y6, col4_w, 34.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.layouts[mode_idx][slot].visible_in_menu ^= true;
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                        if inside([wx + 12.0 + (col4_w + 4.0) * 2.0, y6, col4_w, 34.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.apply_visibility_preset_all();
                            let _ = tc.settings.save();
                            ed.status_msg =
                                Some("Applied preset visibility to all controls.".into());
                            tc.clicked = true;
                            return false;
                        }
                        if inside([wx + 12.0 + (col4_w + 4.0) * 3.0, y6, col4_w, 34.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.button_visibility[Control::Move as usize] = 2;
                            for m in 0..3 {
                                tc.settings.layouts[m][slot].visible_in_game = false;
                                tc.settings.layouts[m][slot].visible_in_menu = false;
                            }
                            ed.selected = None;
                            if !tc.settings.custom_buttons.is_empty() {
                                ed.selected_custom = Some(0);
                            }
                            ed.props_open = false;
                            ed.status_msg = Some(
                                "Deleted Joystick. Open Control Manager (=) to restore it anytime."
                                    .into(),
                            );
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                }
                return false;
            }

            let mode_idx = tc.settings.mode.index();
            let mutate_placement =
                |tc: &mut crate::touch::TouchControls,
                 ed: &crate::touch::Editor,
                 f: &dyn Fn(&mut crate::touch::Placement)| {
                    if ed.apply_all {
                        for slot_pl in &mut tc.settings.layouts[mode_idx] {
                            f(slot_pl);
                        }
                        for btn in &mut tc.settings.custom_buttons {
                            for m in 0..3 {
                                f(&mut btn.layouts[m]);
                            }
                        }
                    } else if let Some(idx) = ed.selected_custom {
                        if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                            for m in 0..3 {
                                f(&mut btn.layouts[m]);
                            }
                        }
                    } else if let Some(c) = ed.selected {
                        let slot = tc.settings.slot(c);
                        f(&mut tc.settings.layouts[mode_idx][slot]);
                    }
                };

            match ed.props_tab {
                0 => {
                    let st_pill_w = 216.0f32.min(ww * 0.46);
                    if inside([wx + ww - st_pill_w - 12.0, body_y - 2.0, st_pill_w, 24.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.use_goy_style = !pl.use_goy_style);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let cards_y = body_y + 28.0;
                    let card_h = 74.0;
                    for i in 0..3 {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        let rect = [rx, cards_y, cw3, card_h];
                        if inside(rect) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let sign = if p[0] < rx + cw3 * 0.5 { -1.0 } else { 1.0 };
                            match i {
                                0 => {
                                    let t = ((p[0] - (rx + 12.0)) / (cw3 - 24.0).max(16.0))
                                        .clamp(0.0, 1.0);
                                    apply_props_slider(tc, &ed, 0, 0, t);
                                    ed.active_drag = Some((
                                        id,
                                        EditorDragKind::PropsSlider {
                                            tab: 0,
                                            index: 0,
                                            rect,
                                        },
                                    ));
                                }
                                1 => {
                                    mutate_placement(tc, &ed, &move |pl| {
                                        pl.icon_id = if sign < 0.0 {
                                            if pl.icon_id == 0 { 12 } else { pl.icon_id - 1 }
                                        } else {
                                            (pl.icon_id + 1) % 13
                                        };
                                    });
                                }
                                _ => {
                                    if p[0] > rx + cw3 * 0.78 {
                                        let hints = [
                                            "PRI", "SEC", "JUMP", "SNEAK", "SPRINT", "INV", "SWAP",
                                            "DROP", "CHAT", "DEBUG", "3RD", "MOUSE", "MACRO",
                                            "CLICK",
                                        ];
                                        if let Some(idx) = ed.selected_custom {
                                            if let Some(btn) = tc.settings.custom_buttons.get_mut(idx) {
                                                let pos = hints
                                                    .iter()
                                                    .position(|h| *h == btn.custom_label)
                                                    .map(|idx_h| (idx_h + 1) % hints.len())
                                                    .unwrap_or(0);
                                                btn.custom_label = hints[pos].to_owned();
                                                ed.button_label_input = btn.custom_label.clone();
                                            }
                                        } else if let Some(c) = ed.selected {
                                            let cur = tc.settings.control_label(c).to_owned();
                                            let pos = hints
                                                .iter()
                                                .position(|h| *h == cur)
                                                .map(|idx_h| (idx_h + 1) % hints.len())
                                                .unwrap_or(0);
                                            tc.settings.control_labels[c as usize] = hints[pos].to_owned();
                                            ed.button_label_input = hints[pos].to_owned();
                                        }
                                    } else {
                                        let initial = if let Some(idx) = ed.selected_custom {
                                            tc.settings
                                                .custom_buttons
                                                .get(idx)
                                                .map(|b| b.label())
                                                .unwrap_or_default()
                                        } else if let Some(c) = ed.selected {
                                            tc.settings.control_label(c).to_owned()
                                        } else {
                                            String::new()
                                        };
                                        ed.button_label_input = initial;
                                        ed.editing_button_label = true;
                                        ed.keyboard_request = true;
                                        ed.status_msg = Some("Type custom button name on keyboard (Enter to finish).".into());
                                    }
                                }
                            }
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    // Tapping Live Preview Box also cycles Icon Badge
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, card_h];
                    if inside(prev_r) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            pl.icon_id = (pl.icon_id + 1) % 13;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let sw_y = cards_y + card_h + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        if inside([sx, sw_row_y, sw_d, sw_d]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let c_val = *col;
                            mutate_placement(tc, &ed, &move |pl| pl.button_color = c_val);
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let act_w = 158.0f32.min(ww * 0.36);
                    let act_x = wx + ww - act_w - 20.0;
                    let half_h = (sw_card_h - 14.0) * 0.5;
                    if inside([act_x, sw_y + 5.0, act_w, half_h]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            let idx = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| *c == pl.on_click_color)
                                .unwrap_or(0);
                            pl.on_click_color = COLOR_SWATCHES[(idx + 1) % COLOR_SWATCHES.len()].0;
                            pl.on_click_stroke_color =
                                COLOR_SWATCHES[(idx + 2) % COLOR_SWATCHES.len()].0;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([act_x, sw_y + 9.0 + half_h, act_w, half_h]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            let idx = COLOR_SWATCHES
                                .iter()
                                .position(|(c, _)| *c == pl.on_toggle_color)
                                .unwrap_or(0);
                            pl.on_toggle_color = COLOR_SWATCHES[(idx + 1) % COLOR_SWATCHES.len()].0;
                            pl.on_toggle_stroke_color =
                                COLOR_SWATCHES[(idx + 3) % COLOR_SWATCHES.len()].0;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                }
                1 => {
                    let sh_pill_w = 192.0f32.min(ww * 0.44);
                    if inside([wx + ww - sh_pill_w - 12.0, body_y - 2.0, sh_pill_w, 24.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            pl.shape = (pl.shape + 1) % 7;
                            if pl.shape == 1 {
                                pl.roundness = 0.0;
                                pl.stroke_roundness = 0.0;
                            } else if pl.shape == 0 && pl.roundness < 1.0 {
                                pl.roundness = 10.0;
                                pl.stroke_roundness = 10.0;
                            }
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let cards_y = body_y + 28.0;
                    let card_h = 35.0;
                    for i in 0..3 {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        let rect = [rx, cards_y, cw3, card_h];
                        if inside([rx, cards_y - 2.0, cw3, card_h + 4.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let t = ((p[0] - (rx + 10.0)) / (cw3 - 20.0).max(16.0)).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, 1, i, t);
                            ed.active_drag = Some((
                                id,
                                EditorDragKind::PropsSlider {
                                    tab: 1,
                                    index: i,
                                    rect,
                                },
                            ));
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let row2_y = cards_y + 39.0;
                    for i in 0..3 {
                        let rx = wx + 12.0 + i as f32 * (cw3 + 8.0);
                        let rect = [rx, row2_y, cw3, card_h];
                        if inside([rx, row2_y - 2.0, cw3, card_h + 4.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let t = ((p[0] - (rx + 10.0)) / (cw3 - 20.0).max(16.0)).clamp(0.0, 1.0);
                            apply_props_slider(tc, &ed, 1, 3 + i, t);
                            ed.active_drag = Some((
                                id,
                                EditorDragKind::PropsSlider {
                                    tab: 1,
                                    index: 3 + i,
                                    rect,
                                },
                            ));
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let prev_r = [wx + ww - preview_w - 12.0, cards_y, preview_w, 74.0];
                    if inside(prev_r) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            pl.shape = (pl.shape + 1) % 7;
                            if pl.shape == 1 {
                                pl.roundness = 0.0;
                                pl.stroke_roundness = 0.0;
                            } else if pl.shape == 0 && pl.roundness < 1.0 {
                                pl.roundness = 10.0;
                                pl.stroke_roundness = 10.0;
                            }
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    let sw_y = cards_y + 74.0 + 8.0;
                    let sw_card_h = (box_y + box_h - sw_y - 10.0).clamp(48.0, 62.0);
                    let sw_d = 20.0;
                    let sw_row_y = sw_y + sw_card_h - sw_d - 7.0;
                    for (i, (col, _)) in COLOR_SWATCHES.iter().enumerate() {
                        let sx = wx + 22.0 + i as f32 * (sw_d + 6.0);
                        if inside([sx, sw_row_y, sw_d, sw_d]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            let c_val = *col;
                            mutate_placement(tc, &ed, &move |pl| pl.stroke_color = c_val);
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let act_w = 152.0f32.min(ww * 0.34);
                    if inside([
                        wx + ww - act_w - 22.0,
                        sw_y + (sw_card_h - 28.0) * 0.5,
                        act_w,
                        28.0,
                    ]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| {
                            pl.shape = 0;
                            pl.roundness = 12.0;
                            pl.stroke_roundness = 12.0;
                            pl.width_ratio = 1.0;
                            pl.height_ratio = 1.0;
                            pl.stroke_size = 1.5;
                            pl.shadow_size = 4.0;
                        });
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                }
                _ => {
                    let ew = (ww - 24.0 - 30.0) / 4.0;
                    for slot in 0..4 {
                        let ex = wx + 12.0 + slot as f32 * (ew + 10.0);
                        if inside([ex, body_y, ew, 42.0]) {
                            if ed.selected_custom.is_none() {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let idx = tc.settings.custom_buttons.len();
                                let is_goy = tc.settings.control_style == ControlStyle::GoyControl;
                                let mut nb = if let Some(c) = ed.selected {
                                    let mut btn = CustomButton::new(
                                        c.default_binding_token().to_owned(),
                                        idx,
                                    );
                                    btn.custom_label = c.short_label().to_owned();
                                    let base_pl = tc.placement(c);
                                    btn.layouts = [base_pl.clone(), base_pl.clone(), base_pl];
                                    tc.settings.button_visibility[c as usize] = 2;
                                    btn
                                } else {
                                    let mut btn =
                                        CustomButton::new("key.mouse.left".into(), idx);
                                    btn.custom_label = format!("BTN {}", idx + 1);
                                    btn
                                };
                                for m in 0..3 {
                                    nb.layouts[m].use_goy_style = is_goy;
                                }
                                tc.settings.custom_buttons.push(nb);
                                let _ = tc.settings.save();
                                ed.selected_custom = Some(idx);
                                ed.selected = None;
                            }
                            if ed.selected_custom.is_some() {
                                ed.picker_step = PickerStep::AskInputType { event_slot: slot };
                            }
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let by = body_y + 54.0;
                    let bw = (ww - 24.0 - 30.0) / 4.0;
                    for i in 0..4 {
                        let rx = wx + 12.0 + i as f32 * (bw + 10.0);
                        if inside([rx, by, bw, 30.0]) {
                            push_undo_snapshot(&mut ed, &tc.settings);
                            match i {
                                0 => mutate_placement(tc, &ed, &|pl| {
                                    pl.passthrough = !pl.passthrough
                                }),
                                1 => {
                                    mutate_placement(tc, &ed, &|pl| pl.toggleable = !pl.toggleable)
                                }
                                2 => mutate_placement(tc, &ed, &|pl| pl.swipeable = !pl.swipeable),
                                _ => {
                                    if let Some(idx) = ed.selected_custom {
                                        if idx < tc.settings.custom_buttons.len() {
                                            tc.settings.custom_buttons.remove(idx);
                                        }
                                        ed.selected_custom = None;
                                        ed.selected = Some(Control::Move);
                                        ed.props_open = false;
                                    } else if let Some(c) = ed.selected {
                                        let show = !tc.visible(c);
                                        tc.settings.button_visibility[c as usize] =
                                            if show { 1 } else { 2 };
                                    }
                                }
                            }
                            let _ = tc.settings.save();
                            tc.clicked = true;
                            return false;
                        }
                    }
                    let vy = by + 42.0;
                    let col4_w = (ww - 24.0 - 30.0) / 4.0;
                    if inside([wx + 12.0, vy, col4_w, 30.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_game = !pl.visible_in_game);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([wx + 12.0 + col4_w + 10.0, vy, col4_w, 30.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        mutate_placement(tc, &ed, &|pl| pl.visible_in_menu = !pl.visible_in_menu);
                        let _ = tc.settings.save();
                        tc.clicked = true;
                        return false;
                    }
                    if inside([wx + 12.0 + (col4_w + 10.0) * 2.0, vy, col4_w, 30.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        tc.settings.snap_to_grid = !tc.settings.snap_to_grid;
                        let _ = tc.settings.save();
                        ed.status_msg = Some(if tc.settings.snap_to_grid {
                            "Snap to Grid & Neighbor Buttons: ON".into()
                        } else {
                            "Snap to Grid & Neighbor Buttons: OFF".into()
                        });
                        tc.clicked = true;
                        return false;
                    }
                    if inside([wx + 12.0 + (col4_w + 10.0) * 3.0, vy, col4_w, 30.0]) {
                        push_undo_snapshot(&mut ed, &tc.settings);
                        tc.settings.apply_visibility_preset_all();
                        let _ = tc.settings.save();
                        ed.status_msg = Some("Applied preset visibility to all controls.".into());
                        tc.clicked = true;
                        return false;
                    }
                }
            }
            return false;
        }

        // 4. If Main Control Manager Dialog is open:
        if ed.menu_open {
            let dw = (w - 24.0).clamp(300.0, 540.0);
            let dh = (h - 18.0).clamp(245.0, 350.0);
            let dx = (w - dw) * 0.5;
            let dy = (h - dh) * 0.5;
            if inside([dx + dw - 76.0, dy + 10.0, 62.0, 26.0]) || !inside([dx, dy, dw, dh]) {
                ed.menu_open = false;
                tc.clicked = true;
                return false;
            }
            let ey = dy + 56.0;
            let ew = (dw - 28.0 - 8.0) * 0.5;
            // Button 1: Open Save Preset Dialog
            if inside([dx + 14.0, ey, ew, 30.0]) {
                ed.menu_open = false;
                ed.save_dialog_open = true;
                ed.editing_save_desc = false;
                ed.save_name_input = if tc.settings.is_default_preset() {
                    format!("{} Custom", tc.settings.preset_name)
                } else {
                    tc.settings.preset_name.clone()
                };
                ed.save_desc_input = tc.settings.preset_description.clone();
                tc.clicked = true;
                return false;
            }
            // Button 2: Reset Canvas to Default Layout of active system (without overwriting Default on disk)
            if inside([dx + 14.0 + ew + 8.0, ey, ew, 30.0]) {
                push_undo_snapshot(&mut ed, &tc.settings);
                let cur_style = tc.settings.control_style;
                let cur_id = tc.settings.preset_id.clone();
                let cur_name = tc.settings.preset_name.clone();
                let cur_desc = tc.settings.preset_description.clone();
                let active_mcpe = tc.settings.active_mcpe_preset.clone();
                let active_goy = tc.settings.active_goy_preset.clone();
                let fresh = crate::touch::Settings::default_for_style(cur_style);
                tc.settings.layouts = fresh.layouts;
                tc.settings.custom_buttons = fresh.custom_buttons;
                tc.settings.button_visibility = fresh.button_visibility;
                tc.settings.joystick_style = fresh.joystick_style;
                tc.settings.preset_id = cur_id;
                tc.settings.preset_name = cur_name;
                tc.settings.preset_description = cur_desc;
                tc.settings.active_mcpe_preset = active_mcpe;
                tc.settings.active_goy_preset = active_goy;
                clamp_editor_custom_selection(&mut ed, &tc.settings);
                ed.status_msg = Some(format!(
                    "Reset editor canvas to {} Default layout.",
                    cur_style.label()
                ));
                tc.clicked = true;
                return false;
            }

            let py = ey + 50.0;
            let presets = crate::touch::Settings::list_presets(
                std::path::Path::new("."),
                tc.settings.control_style,
            );
            let max_show = presets.len().clamp(1, 5);
            let pw = (dw - 28.0 - 4.0 * (max_show as f32 - 1.0)) / max_show as f32;
            for (i, meta) in presets.iter().take(max_show).enumerate() {
                let rx = dx + 14.0 + i as f32 * (pw + 4.0);
                if inside([rx, py, pw, 28.0]) {
                    push_undo_snapshot(&mut ed, &tc.settings);
                    tc.settings.select_preset(
                        std::path::Path::new("."),
                        tc.settings.control_style,
                        &meta.id,
                    );
                    clamp_editor_custom_selection(&mut ed, &tc.settings);
                    ed.save_name_input = tc.settings.preset_name.clone();
                    ed.save_desc_input = tc.settings.preset_description.clone();
                    ed.status_msg = Some(format!("Loaded preset '{}'.", meta.name));
                    tc.clicked = true;
                    return false;
                }
            }

            let gy = py + 50.0;
            let gw = (dw - 28.0 - 12.0) / 4.0;
            for i in 0..4 {
                let rx = dx + 14.0 + i as f32 * (gw + 4.0);
                if inside([rx, gy, gw, 28.0]) {
                    push_undo_snapshot(&mut ed, &tc.settings);
                    match i {
                        0 => {
                            let slot = tc.settings.slot(Control::Move);
                            if tc.settings.button_visibility[Control::Move as usize] == 2
                                || !tc.placement(Control::Move).visible_in_game
                            {
                                tc.settings.button_visibility[Control::Move as usize] = 1;
                                for m in 0..3 {
                                    tc.settings.layouts[m][slot].visible_in_game = true;
                                }
                                ed.selected = Some(Control::Move);
                                ed.selected_custom = None;
                                ed.status_msg = Some("Restored Joystick to layout.".into());
                            } else {
                                tc.settings.mode = match tc.settings.mode {
                                    Mode::Dpad => Mode::Crosshair,
                                    Mode::Crosshair => Mode::Tap,
                                    Mode::Tap => Mode::Dpad,
                                };
                            }
                        }
                        1 => {
                            tc.settings.joystick_style.move_area =
                                tc.settings.joystick_style.move_area.next()
                        }
                        2 => tc.settings.snap_to_grid = !tc.settings.snap_to_grid,
                        _ => ed.show_hidden = !ed.show_hidden,
                    }
                    let _ = tc.settings.save();
                    tc.clicked = true;
                    return false;
                }
            }
            return false;
        }

        // 5. Check Material 3 Moveable Floating Menu Pod with Select All & Direct Grid Snap Toggle
        let pod_w = if ed.pod_expanded { 568.0 } else { 52.0 };
        let pod_h = 48.0;
        let pod_x = (tc.settings.floating_menu_pos[0] * w).clamp(8.0, (w - pod_w - 8.0).max(8.0));
        let pod_y = (tc.settings.floating_menu_pos[1] * h).clamp(6.0, (h - pod_h - 6.0).max(6.0));
        if inside([pod_x, pod_y, 48.0, pod_h]) {
            ed.active_drag = Some((
                id,
                EditorDragKind::FloatingPod {
                    offset: [p[0] - pod_x, p[1] - pod_y],
                    moved: false,
                },
            ));
            tc.clicked = true;
            return false;
        }
        if ed.pod_expanded && inside([pod_x, pod_y, pod_w, pod_h]) {
            let widths = [38.0, 38.0, 64.0, 80.0, 72.0, 56.0, 58.0, 62.0];
            let mut ix = pod_x + 50.0;
            for (btn_i, iw) in widths.into_iter().enumerate() {
                if inside([ix, pod_y + 5.0, iw, 38.0]) {
                    match btn_i {
                        0 => {
                            // Undo
                            if let Some(prev) = ed.undo_stack.pop() {
                                ed.redo_stack.push(tc.settings.clone());
                                tc.settings = prev;
                                clamp_editor_custom_selection(&mut ed, &tc.settings);
                                let _ = tc.settings.save();
                                ed.status_msg = Some("Undid last change.".into());
                            }
                        }
                        1 => {
                            // Redo
                            if let Some(next) = ed.redo_stack.pop() {
                                ed.undo_stack.push(tc.settings.clone());
                                tc.settings = next;
                                clamp_editor_custom_selection(&mut ed, &tc.settings);
                                let _ = tc.settings.save();
                                ed.status_msg = Some("Redid change.".into());
                            }
                        }
                        2 => {
                            // Open Component Property Inspector
                            ed.props_open = !ed.props_open;
                            ed.menu_open = false;
                        }
                        3 => {
                            // Toggle Select All (bulk-edit every control's Props & style together)
                            ed.apply_all = !ed.apply_all;
                            if ed.apply_all && ed.selected.is_none() && ed.selected_custom.is_none() {
                                if !tc.settings.custom_buttons.is_empty() {
                                    ed.selected_custom = Some(0);
                                } else {
                                    ed.selected = Some(Control::Jump);
                                }
                            }
                            ed.status_msg = Some(if ed.apply_all {
                                "Select All: ON — Props changes apply to every control.".into()
                            } else {
                                "Select All: OFF — Editing selected control only.".into()
                            });
                        }
                        4 => {
                            // Toggle Snap to Grid & Neighbor Edge-Sticking
                            push_undo_snapshot(&mut ed, &tc.settings);
                            tc.settings.snap_to_grid = !tc.settings.snap_to_grid;
                            let _ = tc.settings.save();
                            ed.status_msg = Some(if tc.settings.snap_to_grid {
                                "Snap to Grid & Neighbor Buttons: ON".into()
                            } else {
                                "Snap to Grid & Neighbor Buttons: OFF".into()
                            });
                        }
                        5 => {
                            // + Add Button (supported in BOTH MCPE and GoyControl modes!)
                            if tc.settings.custom_buttons.len() < 256 {
                                push_undo_snapshot(&mut ed, &tc.settings);
                                let idx = tc.settings.custom_buttons.len();
                                let is_goy = tc.settings.control_style == ControlStyle::GoyControl;
                                let mut nb = CustomButton::new("key.mouse.left".into(), idx);
                                nb.custom_label = format!("BTN {}", idx + 1);
                                for m in 0..3 {
                                    nb.layouts[m].use_goy_style = is_goy;
                                }
                                tc.settings.custom_buttons.push(nb);
                                ed.selected_custom = Some(idx);
                                ed.selected = None;
                                ed.props_open = true;
                                ed.props_tab = 2;
                                if !is_goy {
                                    ed.picker_step = PickerStep::AskInputType { event_slot: 0 };
                                }
                                let _ = tc.settings.save();
                            }
                        }
                        6 => {
                            // Copy Selected Button
                            duplicate_selected_editor_button(&mut ed, tc);
                        }
                        _ => {
                            // Open Save Dialog (Option A: Replace / Option B: Create New Preset)
                            ed.save_dialog_open = true;
                            ed.menu_open = false;
                            ed.props_open = false;
                            ed.editing_save_desc = false;
                            ed.save_name_input = if tc.settings.is_default_preset() {
                                format!("{} Custom", tc.settings.preset_name)
                            } else {
                                tc.settings.preset_name.clone()
                            };
                            ed.save_desc_input = tc.settings.preset_description.clone();
                        }
                    }
                    tc.clicked = true;
                    return false;
                }
                ix += iw + 5.0;
            }
        }

        // 6. Check 8 Corner & Edge Resize Handles on currently selected component FIRST
        let aspect = tc.aspect();
        let cur_sel_placement = if let Some(idx) = ed.selected_custom {
            tc.settings
                .custom_buttons
                .get(idx)
                .map(|_| tc.custom_placement(idx))
        } else {
            ed.selected.map(|c| tc.placement(c))
        };
        if let Some(sel_p) = cur_sel_placement {
            let [nx, ny, nw, nh] = sel_p.rect(aspect);
            let [rx, ry, rw, rh] = [nx * w, ny * h, nw * w, nh * h];
            let handle_tests: [([f32; 4], u8); 8] = [
                ([rx - 10.0, ry - 10.0, 18.0, 18.0], 2 | 8),
                ([rx + rw - 8.0, ry - 10.0, 18.0, 18.0], 1 | 8),
                ([rx - 10.0, ry + rh - 8.0, 18.0, 18.0], 2 | 4),
                ([rx + rw - 8.0, ry + rh - 8.0, 18.0, 18.0], 1 | 4),
                ([rx + rw - 8.0, ry + rh * 0.5 - 12.0, 16.0, 24.0], 1),
                ([rx - 8.0, ry + rh * 0.5 - 12.0, 16.0, 24.0], 2),
                ([rx + rw * 0.5 - 12.0, ry + rh - 8.0, 24.0, 16.0], 4),
                ([rx + rw * 0.5 - 12.0, ry - 8.0, 24.0, 16.0], 8),
            ];
            for (hr, mask) in handle_tests {
                if inside(hr) {
                    push_undo_snapshot(&mut ed, &tc.settings);
                    ed.active_drag = Some((
                        id,
                        EditorDragKind::Resize {
                            mask,
                            start_pos: point,
                            start_size: sel_p.size,
                            start_wr: sel_p.width_ratio,
                            start_hr: sel_p.height_ratio,
                        },
                    ));
                    tc.clicked = true;
                    return false;
                }
            }
        }

        // 7. Hit-test Custom Buttons (in both MCPE and GoyControl modes) & Built-in Controls for Selection + Move Drag
        for i in (0..tc.settings.custom_buttons.len()).rev() {
            let pl = tc.custom_placement(i);
            let r = pl.rect(aspect);
            if crate::touch::layout::contains(r, point) {
                push_undo_snapshot(&mut ed, &tc.settings);
                ed.selected_custom = Some(i);
                ed.selected = None;
                ed.active_drag = Some((
                    id,
                    EditorDragKind::Move {
                        offset: [point[0] - pl.center[0], point[1] - pl.center[1]],
                    },
                ));
                tc.clicked = true;
                return false;
            }
        }

        for &c in Control::ALL.iter().rev() {
            if tc.settings.control_style == ControlStyle::GoyControl && c != Control::Move {
                continue;
            }
            let vis = tc.visible(c);
            if !vis && !ed.show_hidden {
                continue;
            }
            let pl = tc.placement(c);
            let r = pl.rect(aspect);
            if crate::touch::layout::contains(r, point) {
                push_undo_snapshot(&mut ed, &tc.settings);
                ed.selected = Some(c);
                ed.selected_custom = None;
                ed.active_drag = Some((
                    id,
                    EditorDragKind::Move {
                        offset: [point[0] - pl.center[0], point[1] - pl.center[1]],
                    },
                ));
                tc.clicked = true;
                return false;
            }
        }

        false
    })();

    if !closed {
        tc.editor = Some(ed);
    }
    closed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_mouse_drag_does_not_generate_an_extra_tap_or_hold_click() {
        let mut ui = TouchUi::default();
        ui.relative_touch(
            1,
            [10.0, 10.0],
            TouchPhase::Started,
            [20.0, 20.0],
            [400.0, 250.0],
            1.0,
            true,
        );
        ui.press_mouse(2, false);
        ui.relative.as_mut().unwrap().since = Instant::now() - Duration::from_millis(700);
        assert_eq!(ui.relative_click(false), None);
        assert_eq!(ui.release_mouse(2), Some((false, true)));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(
            1,
            [10.0, 10.0],
            TouchPhase::Ended,
            [20.0, 20.0],
            [400.0, 250.0],
            1.0,
            true,
        );
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn authoritative_virtual_mouse_visibility_and_viewport_centering() {
        let mut ui = TouchUi::default();
        let mut controls = crate::touch::TouchControls::default();
        let mut cursor = (0.0, 0.0);

        // Initial spawn centers cursor in playable viewport
        assert!(ui.sync_viewport_cursor(&mut cursor, [427.0, 240.0]));
        assert_eq!(cursor, (213.5, 120.0));

        // Moving cursor to (0.0, 0.0) after initialization does NOT teleport it back to center
        cursor = (0.0, 0.0);
        assert!(!ui.sync_viewport_cursor(&mut cursor, [427.0, 240.0]));
        assert_eq!(cursor, (0.0, 0.0));

        // Proportional resize preserves relative position without drift
        cursor = (213.5, 120.0);
        assert!(ui.sync_viewport_cursor(&mut cursor, [854.0, 480.0]));
        assert_eq!(cursor, (427.0, 240.0));

        // Default is OFF unless previously enabled by the user
        assert!(!controls.settings.virtual_mouse_enabled);
        assert!(!ui.compute_virtual_mouse_visibility(&controls, true, false, true, false));
        controls.settings.virtual_mouse_enabled = true;

        // MCPE mode: hidden when MCPE gameplay controls are visible, visible when hidden (menus / F1)
        assert!(!ui.compute_virtual_mouse_visibility(&controls, false, false, true, true));
        assert!(ui.compute_virtual_mouse_visibility(&controls, true, false, true, false));
        assert!(ui.compute_virtual_mouse_visibility(&controls, false, true, true, true));

        // Control Editor open: game cursor is ALWAYS hidden over the Control Editor
        controls.open_editor();
        assert!(!ui.compute_virtual_mouse_visibility(&controls, false, false, true, false));
        controls.cancel_editor();

        // Authoritative toggle OFF: remains hidden regardless of auto-visibility conditions
        controls.settings.virtual_mouse_enabled = false;
        assert!(!ui.compute_virtual_mouse_visibility(&controls, true, false, true, false));
        assert!(!ui.compute_virtual_mouse_visibility(&controls, false, true, true, false));
    }
    #[test]
    fn menu_mouse_buttons_share_ownership_and_release_outside_their_bounds() {
        let mut ui = TouchUi {
            virtual_mouse: true,
            ..Default::default()
        };
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 226.0], true),
            Some(Action::MouseButton(false))
        ));
        assert!(matches!(
            ui.action(427.0, 267.0, [399.0, 250.0], true),
            Some(Action::MouseButton(true))
        ));
        assert!(ui.action(427.0, 267.0, [399.0, 226.0], false).is_none());
        assert!(ui.press_mouse(1, false));
        assert!(!ui.press_mouse(2, false));
        assert!(ui.press_mouse(3, true));
        assert_eq!(ui.release_mouse(1), Some((false, false)));
        assert_eq!(ui.release_mouse(2), Some((false, true)));
        assert_eq!(ui.release_all_mouse(), vec![true]);
        assert_eq!(ui.release_mouse(3), None);
        ui.virtual_mouse = false;
        assert!(ui.action(427.0, 267.0, [399.0, 226.0], true).is_none());
    }
    #[test]
    fn virtual_mouse_moves_relatively_and_dragging_never_clicks() {
        let mut ui = TouchUi::default();
        let cursor = [100.0, 80.0];
        let size = [400.0, 240.0];
        assert_eq!(
            ui.relative_touch(
                1,
                [300.0, 200.0],
                TouchPhase::Started,
                cursor,
                size,
                2.0,
                true
            ),
            cursor
        );
        let p = ui.relative_touch(
            1,
            [310.0, 195.0],
            TouchPhase::Moved,
            cursor,
            size,
            2.0,
            true,
        );
        assert_eq!(p, [120.0, 70.0]);
        ui.relative_touch(1, [310.0, 195.0], TouchPhase::Ended, p, size, 2.0, true);
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn virtual_mouse_tap_hold_cancel_and_click_preference() {
        let mut ui = TouchUi::default();
        let p = [100.0, 80.0];
        let size = [400.0, 240.0];
        ui.relative_touch(1, p, TouchPhase::Started, p, size, 1.0, true);
        ui.relative_touch(1, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), Some(false));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(2, p, TouchPhase::Started, p, size, 1.0, true);
        ui.relative.as_mut().unwrap().since = Instant::now() - Duration::from_secs(1);
        assert_eq!(ui.relative_click(true), Some(true));
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(2, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), None);
        ui.relative_touch(3, p, TouchPhase::Started, p, size, 1.0, false);
        ui.relative_touch(3, p, TouchPhase::Ended, p, size, 1.0, false);
        assert_eq!(ui.relative_click(false), None);
        ui.relative_touch(4, p, TouchPhase::Started, p, size, 1.0, true);
        ui.cancel_relative();
        ui.relative_touch(4, p, TouchPhase::Ended, p, size, 1.0, true);
        assert_eq!(ui.relative_click(true), None);
    }
    #[test]
    fn simultaneous_f3_f4_and_f6_release_independently() {
        let mut ui = TouchUi::default();
        assert!(ui.key_down(10, KeyCode::F3));
        assert!(ui.key_down(11, KeyCode::F4));
        assert!(ui.key_held(KeyCode::F3));
        assert_eq!(ui.key_up(11), Some(KeyCode::F4));
        assert!(ui.key_down(12, KeyCode::F6));
        assert_eq!(ui.key_up(12), Some(KeyCode::F6));
        assert!(ui.key_held(KeyCode::F3));
        assert_eq!(ui.key_up(10), Some(KeyCode::F3));
    }
    #[test]
    fn shared_key_and_latch_do_not_release_early() {
        let mut ui = TouchUi::default();
        assert!(ui.key_down(1, KeyCode::F3));
        assert!(!ui.key_down(2, KeyCode::F3));
        assert_eq!(ui.key_up(1), None);
        ui.f3_latched = true;
        assert_eq!(ui.key_up(2), None);
        assert_eq!(ui.release_keys(), vec![KeyCode::F3]);
        assert!(!ui.key_held(KeyCode::F3));
    }
    #[test]
    fn scrolling_never_clicks() {
        let mut u = TouchUi::default();
        assert!(matches!(
            u.gesture(1, [20.0, 200.0], TouchPhase::Started),
            Gesture::None
        ));
        assert!(
            matches!(u.gesture(1,[20.0,140.0],TouchPhase::Moved),Gesture::Scroll(v) if v == -2.0)
        );
        assert!(matches!(
            u.gesture(1, [20.0, 140.0], TouchPhase::Ended),
            Gesture::None
        ));
    }
    #[test]
    fn tap_release_cancel_and_second_finger() {
        let mut u = TouchUi::default();
        u.gesture(1, [20.0, 20.0], TouchPhase::Started);
        assert!(matches!(
            u.gesture(2, [20.0, 20.0], TouchPhase::Ended),
            Gesture::None
        ));
        assert!(matches!(
            u.gesture(1, [21.0, 21.0], TouchPhase::Ended),
            Gesture::Tap
        ));
        u.gesture(1, [20.0, 20.0], TouchPhase::Started);
        u.gesture(1, [20.0, 20.0], TouchPhase::Cancelled);
        assert!(matches!(
            u.gesture(1, [20.0, 20.0], TouchPhase::Ended),
            Gesture::None
        ));
    }

    /// The press/release decision table of a physical left click the pad may
    /// take: `(pressed, owned, touched)`.
    #[test]
    fn aux_mouse_presses_belong_to_the_pad_only_over_its_buttons() {
        // Gameplay in a world: attack and use are not the pad's.
        assert_eq!(aux_mouse_phase(true, false, false), None);
        // The free pointer uses the gameplay auxiliary buttons (keypad
        // profile) and menu actions alike: the press starts a touch.
        assert_eq!(
            aux_mouse_phase(true, false, true),
            Some(TouchPhase::Started)
        );
    }
    #[test]
    fn a_menu_press_survives_closing_the_menu_before_its_release() {
        // Press an action, then the screen closes and redraws elsewhere: the
        // release must still complete the press, through the owned key.
        let mut ui = TouchUi::default();
        let owned = |ui: &TouchUi| ui.owns_key(7);
        assert_eq!(aux_mouse_phase(true, owned(&ui), false), None);
        assert_eq!(
            aux_mouse_phase(true, owned(&ui), true),
            Some(TouchPhase::Started)
        );
        ui.key_down(7, KeyCode::KeyT);
        assert_eq!(
            aux_mouse_phase(false, owned(&ui), false),
            Some(TouchPhase::Ended)
        );
        assert_eq!(ui.key_up(7), Some(KeyCode::KeyT));
        // The pad released everything: later strays do nothing (a release
        // game screens can drop on menu entry also stays inert).
        assert_eq!(aux_mouse_phase(false, owned(&ui), false), None);
        assert_eq!(aux_mouse_phase(true, owned(&ui), false), None);
    }
    #[test]
    fn a_held_auxiliary_mouse_button_releases_after_a_screen_change() {
        // Where LMB/RMB utility buttons own the pointer, a release stays
        // theirs across ownership checks, then frees it.
        let mut ui = TouchUi::default();
        assert_eq!(aux_mouse_phase(false, ui.owns_mouse(7), false), None);
        ui.press_mouse(7, true);
        assert_eq!(
            aux_mouse_phase(false, ui.owns_mouse(7), false),
            Some(TouchPhase::Ended)
        );
        assert_eq!(ui.release_mouse(7), Some((true, true)));
        assert_eq!(aux_mouse_phase(false, ui.owns_mouse(7), false), None);
    }
    #[test]
    fn a_release_without_ownership_stays_with_the_game() {
        assert_eq!(aux_mouse_phase(false, false, true), None);
    }
}
