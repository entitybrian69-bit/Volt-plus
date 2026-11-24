//! Shared input translation for the Pojav and Zalith Android control adapters.
//! These helpers do not provide an overlay or import layouts by themselves.
use std::collections::{BTreeMap, BTreeSet};

/// The GLFW keyboard identifiers used by both upstream launchers. Unknown keys
/// are rejected, not interpreted as an unassigned or unrelated game control.
const GLFW_KEYS: &[(i32, &str, &str)] = &[
    (32, "GLFW_KEY_SPACE", "key.keyboard.space"),
    (39, "GLFW_KEY_APOSTROPHE", "key.keyboard.apostrophe"),
    (44, "GLFW_KEY_COMMA", "key.keyboard.comma"),
    (45, "GLFW_KEY_MINUS", "key.keyboard.minus"),
    (46, "GLFW_KEY_PERIOD", "key.keyboard.period"),
    (47, "GLFW_KEY_SLASH", "key.keyboard.slash"),
    (48, "GLFW_KEY_0", "key.keyboard.0"),
    (49, "GLFW_KEY_1", "key.keyboard.1"),
    (50, "GLFW_KEY_2", "key.keyboard.2"),
    (51, "GLFW_KEY_3", "key.keyboard.3"),
    (52, "GLFW_KEY_4", "key.keyboard.4"),
    (53, "GLFW_KEY_5", "key.keyboard.5"),
    (54, "GLFW_KEY_6", "key.keyboard.6"),
    (55, "GLFW_KEY_7", "key.keyboard.7"),
    (56, "GLFW_KEY_8", "key.keyboard.8"),
    (57, "GLFW_KEY_9", "key.keyboard.9"),
    (59, "GLFW_KEY_SEMICOLON", "key.keyboard.semicolon"),
    (61, "GLFW_KEY_EQUAL", "key.keyboard.equal"),
    (65, "GLFW_KEY_A", "key.keyboard.a"),
    (66, "GLFW_KEY_B", "key.keyboard.b"),
    (67, "GLFW_KEY_C", "key.keyboard.c"),
    (68, "GLFW_KEY_D", "key.keyboard.d"),
    (69, "GLFW_KEY_E", "key.keyboard.e"),
    (70, "GLFW_KEY_F", "key.keyboard.f"),
    (71, "GLFW_KEY_G", "key.keyboard.g"),
    (72, "GLFW_KEY_H", "key.keyboard.h"),
    (73, "GLFW_KEY_I", "key.keyboard.i"),
    (74, "GLFW_KEY_J", "key.keyboard.j"),
    (75, "GLFW_KEY_K", "key.keyboard.k"),
    (76, "GLFW_KEY_L", "key.keyboard.l"),
    (77, "GLFW_KEY_M", "key.keyboard.m"),
    (78, "GLFW_KEY_N", "key.keyboard.n"),
    (79, "GLFW_KEY_O", "key.keyboard.o"),
    (80, "GLFW_KEY_P", "key.keyboard.p"),
    (81, "GLFW_KEY_Q", "key.keyboard.q"),
    (82, "GLFW_KEY_R", "key.keyboard.r"),
    (83, "GLFW_KEY_S", "key.keyboard.s"),
    (84, "GLFW_KEY_T", "key.keyboard.t"),
    (85, "GLFW_KEY_U", "key.keyboard.u"),
    (86, "GLFW_KEY_V", "key.keyboard.v"),
    (87, "GLFW_KEY_W", "key.keyboard.w"),
    (88, "GLFW_KEY_X", "key.keyboard.x"),
    (89, "GLFW_KEY_Y", "key.keyboard.y"),
    (90, "GLFW_KEY_Z", "key.keyboard.z"),
    (91, "GLFW_KEY_LEFT_BRACKET", "key.keyboard.left.bracket"),
    (92, "GLFW_KEY_BACKSLASH", "key.keyboard.backslash"),
    (93, "GLFW_KEY_RIGHT_BRACKET", "key.keyboard.right.bracket"),
    (96, "GLFW_KEY_GRAVE_ACCENT", "key.keyboard.grave.accent"),
    (161, "GLFW_KEY_WORLD_1", "key.keyboard.world.1"),
    (256, "GLFW_KEY_ESCAPE", "key.keyboard.escape"),
    (257, "GLFW_KEY_ENTER", "key.keyboard.enter"),
    (258, "GLFW_KEY_TAB", "key.keyboard.tab"),
    (259, "GLFW_KEY_BACKSPACE", "key.keyboard.backspace"),
    (260, "GLFW_KEY_INSERT", "key.keyboard.insert"),
    (261, "GLFW_KEY_DELETE", "key.keyboard.delete"),
    (262, "GLFW_KEY_RIGHT", "key.keyboard.right"),
    (263, "GLFW_KEY_LEFT", "key.keyboard.left"),
    (264, "GLFW_KEY_DOWN", "key.keyboard.down"),
    (265, "GLFW_KEY_UP", "key.keyboard.up"),
    (266, "GLFW_KEY_PAGE_UP", "key.keyboard.page.up"),
    (267, "GLFW_KEY_PAGE_DOWN", "key.keyboard.page.down"),
    (268, "GLFW_KEY_HOME", "key.keyboard.home"),
    (269, "GLFW_KEY_END", "key.keyboard.end"),
    (280, "GLFW_KEY_CAPS_LOCK", "key.keyboard.caps.lock"),
    (281, "GLFW_KEY_SCROLL_LOCK", "key.keyboard.scroll.lock"),
    (282, "GLFW_KEY_NUM_LOCK", "key.keyboard.num.lock"),
    (283, "GLFW_KEY_PRINT_SCREEN", "key.keyboard.print.screen"),
    (284, "GLFW_KEY_PAUSE", "key.keyboard.pause"),
    (290, "GLFW_KEY_F1", "key.keyboard.f1"),
    (291, "GLFW_KEY_F2", "key.keyboard.f2"),
    (292, "GLFW_KEY_F3", "key.keyboard.f3"),
    (293, "GLFW_KEY_F4", "key.keyboard.f4"),
    (294, "GLFW_KEY_F5", "key.keyboard.f5"),
    (295, "GLFW_KEY_F6", "key.keyboard.f6"),
    (296, "GLFW_KEY_F7", "key.keyboard.f7"),
    (297, "GLFW_KEY_F8", "key.keyboard.f8"),
    (298, "GLFW_KEY_F9", "key.keyboard.f9"),
    (299, "GLFW_KEY_F10", "key.keyboard.f10"),
    (300, "GLFW_KEY_F11", "key.keyboard.f11"),
    (301, "GLFW_KEY_F12", "key.keyboard.f12"),
    (302, "GLFW_KEY_F13", "key.keyboard.f13"),
    (303, "GLFW_KEY_F14", "key.keyboard.f14"),
    (304, "GLFW_KEY_F15", "key.keyboard.f15"),
    (305, "GLFW_KEY_F16", "key.keyboard.f16"),
    (306, "GLFW_KEY_F17", "key.keyboard.f17"),
    (307, "GLFW_KEY_F18", "key.keyboard.f18"),
    (308, "GLFW_KEY_F19", "key.keyboard.f19"),
    (309, "GLFW_KEY_F20", "key.keyboard.f20"),
    (310, "GLFW_KEY_F21", "key.keyboard.f21"),
    (311, "GLFW_KEY_F22", "key.keyboard.f22"),
    (312, "GLFW_KEY_F23", "key.keyboard.f23"),
    (313, "GLFW_KEY_F24", "key.keyboard.f24"),
    (320, "GLFW_KEY_KP_0", "key.keyboard.keypad.0"),
    (321, "GLFW_KEY_KP_1", "key.keyboard.keypad.1"),
    (322, "GLFW_KEY_KP_2", "key.keyboard.keypad.2"),
    (323, "GLFW_KEY_KP_3", "key.keyboard.keypad.3"),
    (324, "GLFW_KEY_KP_4", "key.keyboard.keypad.4"),
    (325, "GLFW_KEY_KP_5", "key.keyboard.keypad.5"),
    (326, "GLFW_KEY_KP_6", "key.keyboard.keypad.6"),
    (327, "GLFW_KEY_KP_7", "key.keyboard.keypad.7"),
    (328, "GLFW_KEY_KP_8", "key.keyboard.keypad.8"),
    (329, "GLFW_KEY_KP_9", "key.keyboard.keypad.9"),
    (330, "GLFW_KEY_KP_DECIMAL", "key.keyboard.keypad.period"),
    (331, "GLFW_KEY_KP_DIVIDE", "key.keyboard.keypad.divide"),
    (332, "GLFW_KEY_KP_MULTIPLY", "key.keyboard.keypad.multiply"),
    (333, "GLFW_KEY_KP_SUBTRACT", "key.keyboard.keypad.subtract"),
    (334, "GLFW_KEY_KP_ADD", "key.keyboard.keypad.add"),
    (335, "GLFW_KEY_KP_ENTER", "key.keyboard.keypad.enter"),
    (336, "GLFW_KEY_KP_EQUAL", "key.keyboard.keypad.equal"),
    (340, "GLFW_KEY_LEFT_SHIFT", "key.keyboard.left.shift"),
    (341, "GLFW_KEY_LEFT_CONTROL", "key.keyboard.left.control"),
    (342, "GLFW_KEY_LEFT_ALT", "key.keyboard.left.alt"),
    (343, "GLFW_KEY_LEFT_SUPER", "key.keyboard.left.win"),
    (344, "GLFW_KEY_RIGHT_SHIFT", "key.keyboard.right.shift"),
    (345, "GLFW_KEY_RIGHT_CONTROL", "key.keyboard.right.control"),
    (346, "GLFW_KEY_RIGHT_ALT", "key.keyboard.right.alt"),
    (347, "GLFW_KEY_RIGHT_SUPER", "key.keyboard.right.win"),
    (348, "GLFW_KEY_MENU", "key.keyboard.menu"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlEvent {
    Token(&'static str),
    ToggleKeyboard,
    ToggleControls,
    OpenMenu,
    ScrollUpHeld,
    ScrollDownHeld,
    LayerSwitch,
    LayerShow,
    LayerHide,
    SendText,
}

pub fn glfw_key(code: i32) -> Option<&'static str> {
    GLFW_KEYS.iter().find(|entry| entry.0 == code).map(|entry| entry.2)
}
pub fn glfw_key_named(name: &str) -> Option<&'static str> {
    GLFW_KEYS.iter().find(|entry| entry.1 == name).map(|entry| entry.2)
}
/// Pojav reserves negative codes for launcher actions, distinct from GLFW keys.
pub fn pojav_code(code: i32) -> Option<ControlEvent> {
    Some(match code {
        -1 => ControlEvent::ToggleKeyboard,
        -2 => ControlEvent::ToggleControls,
        -3 => ControlEvent::Token("key.mouse.left"),
        -4 => ControlEvent::Token("key.mouse.right"),
        -5 => ControlEvent::Token("mouse.virtual.toggle"),
        -6 => ControlEvent::Token("key.mouse.middle"),
        -7 => ControlEvent::Token("mouse.scroll.up"),
        -8 => ControlEvent::Token("mouse.scroll.down"),
        -9 => ControlEvent::OpenMenu,
        -10 => ControlEvent::ToggleKeyboard,
        _ => ControlEvent::Token(glfw_key(code)?),
    })
}

/// Maps Android `android.view.KeyEvent.KEYCODE_*` integers (used by Pojav v9+ /
/// Android-KeyEvent control layouts) to Horizon `key.keyboard.*` tokens.
pub fn android_keycode(code: i32) -> Option<&'static str> {
    Some(match code {
        4 => "key.keyboard.escape",
        7 => "key.keyboard.0",
        8 => "key.keyboard.1",
        9 => "key.keyboard.2",
        10 => "key.keyboard.3",
        11 => "key.keyboard.4",
        12 => "key.keyboard.5",
        13 => "key.keyboard.6",
        14 => "key.keyboard.7",
        15 => "key.keyboard.8",
        16 => "key.keyboard.9",
        19 => "key.keyboard.up",
        20 => "key.keyboard.down",
        21 => "key.keyboard.left",
        22 => "key.keyboard.right",
        29 => "key.keyboard.a",
        30 => "key.keyboard.b",
        31 => "key.keyboard.c",
        32 => "key.keyboard.d",
        33 => "key.keyboard.e",
        34 => "key.keyboard.f",
        35 => "key.keyboard.g",
        36 => "key.keyboard.h",
        37 => "key.keyboard.i",
        38 => "key.keyboard.j",
        39 => "key.keyboard.k",
        40 => "key.keyboard.l",
        41 => "key.keyboard.m",
        42 => "key.keyboard.n",
        43 => "key.keyboard.o",
        44 => "key.keyboard.p",
        45 => "key.keyboard.q",
        46 => "key.keyboard.r",
        47 => "key.keyboard.s",
        48 => "key.keyboard.t",
        49 => "key.keyboard.u",
        50 => "key.keyboard.v",
        51 => "key.keyboard.w",
        52 => "key.keyboard.x",
        53 => "key.keyboard.y",
        54 => "key.keyboard.z",
        55 => "key.keyboard.comma",
        56 => "key.keyboard.period",
        57 => "key.keyboard.left.alt",
        58 => "key.keyboard.right.alt",
        59 => "key.keyboard.left.shift",
        60 => "key.keyboard.right.shift",
        61 => "key.keyboard.tab",
        62 => "key.keyboard.space",
        66 => "key.keyboard.enter",
        67 => "key.keyboard.backspace",
        68 => "key.keyboard.grave.accent",
        69 => "key.keyboard.minus",
        70 => "key.keyboard.equal",
        71 => "key.keyboard.left.bracket",
        72 => "key.keyboard.right.bracket",
        73 => "key.keyboard.backslash",
        74 => "key.keyboard.semicolon",
        75 => "key.keyboard.apostrophe",
        76 => "key.keyboard.slash",
        82 => "key.keyboard.menu",
        92 => "key.keyboard.page.up",
        93 => "key.keyboard.page.down",
        111 => "key.keyboard.escape",
        112 => "key.keyboard.delete",
        113 => "key.keyboard.left.control",
        114 => "key.keyboard.right.control",
        115 => "key.keyboard.caps.lock",
        116 => "key.keyboard.scroll.lock",
        117 => "key.keyboard.left.win",
        118 => "key.keyboard.right.win",
        120 => "key.keyboard.print.screen",
        121 => "key.keyboard.pause",
        122 => "key.keyboard.home",
        123 => "key.keyboard.end",
        124 => "key.keyboard.insert",
        131 => "key.keyboard.f1",
        132 => "key.keyboard.f2",
        133 => "key.keyboard.f3",
        134 => "key.keyboard.f4",
        135 => "key.keyboard.f5",
        136 => "key.keyboard.f6",
        137 => "key.keyboard.f7",
        138 => "key.keyboard.f8",
        139 => "key.keyboard.f9",
        140 => "key.keyboard.f10",
        141 => "key.keyboard.f11",
        142 => "key.keyboard.f12",
        143 => "key.keyboard.num.lock",
        144 => "key.keyboard.keypad.0",
        145 => "key.keyboard.keypad.1",
        146 => "key.keyboard.keypad.2",
        147 => "key.keyboard.keypad.3",
        148 => "key.keyboard.keypad.4",
        149 => "key.keyboard.keypad.5",
        150 => "key.keyboard.keypad.6",
        151 => "key.keyboard.keypad.7",
        152 => "key.keyboard.keypad.8",
        153 => "key.keyboard.keypad.9",
        154 => "key.keyboard.keypad.divide",
        155 => "key.keyboard.keypad.multiply",
        156 => "key.keyboard.keypad.subtract",
        157 => "key.keyboard.keypad.add",
        158 => "key.keyboard.keypad.period",
        159 => "key.keyboard.keypad.comma",
        160 => "key.keyboard.keypad.enter",
        161 => "key.keyboard.keypad.equal",
        _ => return None,
    })
}

/// Pojav decoder for Android `KeyEvent` layout variants (`version >= 9`).
pub fn pojav_code_android(code: i32) -> Option<ControlEvent> {
    Some(match code {
        -1 => ControlEvent::ToggleKeyboard,
        -2 => ControlEvent::ToggleControls,
        -3 => ControlEvent::Token("key.mouse.left"),
        -4 => ControlEvent::Token("key.mouse.right"),
        -5 => ControlEvent::Token("mouse.virtual.toggle"),
        -6 => ControlEvent::Token("key.mouse.middle"),
        -7 => ControlEvent::Token("mouse.scroll.up"),
        -8 => ControlEvent::Token("mouse.scroll.down"),
        -9 => ControlEvent::OpenMenu,
        -10 => ControlEvent::ToggleKeyboard,
        _ => ControlEvent::Token(android_keycode(code)?),
    })
}
/// Apply an upstream Pojav input edge to a queue. Pojav's wheel buttons
/// fire on release, rather than on press; encode each wheel tick as a complete
/// pair so repeated taps remain balanced. UI-owned actions return false.
pub fn enqueue_pojav(queue: &mut InputQueue, code: i32, down: bool) -> bool {
    let Some(ControlEvent::Token(token)) = pojav_code(code) else { return false; };
    if matches!(code, -7 | -8) {
        if !down {
            queue.send(token, true);
            queue.send(token, false);
        }
    } else {
        queue.send(token, down);
    }
    true
}

/// Layer operations and text keep their original payload with the caller.
/// Unknown launcher-specific actions must be surfaced, never dropped silently.
pub fn zalith_event(kind: &str, key: &str) -> Option<ControlEvent> {
    match kind {
        "key" => glfw_key_named(key).map(ControlEvent::Token),
        "switch_layer" => Some(ControlEvent::LayerSwitch),
        "show_layer" => Some(ControlEvent::LayerShow),
        "hide_layer" => Some(ControlEvent::LayerHide),
        "send_text" => Some(ControlEvent::SendText),
        "launcher_event" => Some(match key {
            "GLFW_MOUSE_BUTTON_LEFT" | "GLFW_MOUSE_BUTTON_1" => ControlEvent::Token("key.mouse.left"),
            "GLFW_MOUSE_BUTTON_RIGHT" | "GLFW_MOUSE_BUTTON_2" => ControlEvent::Token("key.mouse.right"),
            "GLFW_MOUSE_BUTTON_MIDDLE" | "GLFW_MOUSE_BUTTON_3" => ControlEvent::Token("key.mouse.middle"),
            "GLFW_MOUSE_BUTTON_4" => ControlEvent::Token("key.mouse.4"),
            "GLFW_MOUSE_BUTTON_5" => ControlEvent::Token("key.mouse.5"),
            "launcher.event.switch_ime" => ControlEvent::ToggleKeyboard,
            "launcher.event.switch_menu" => ControlEvent::OpenMenu,
            "launcher.event.scroll_up" => ControlEvent::ScrollUpHeld,
            "launcher.event.scroll_down" => ControlEvent::ScrollDownHeld,
            "launcher.event.scroll_up.single" => ControlEvent::Token("mouse.scroll.up"),
            "launcher.event.scroll_down.single" => ControlEvent::Token("mouse.scroll.down"),
            _ => return None,
        }),
        _ => None,
    }
}

/// Tracks per-widget/finger ownership. Releasing one of several controls bound
/// to the same key must not release that key until the last owner lets go.
#[derive(Default)]
pub struct HoldTracker {
    by_owner: BTreeMap<u64, BTreeSet<String>>,
    counts: BTreeMap<String, usize>,
}
impl HoldTracker {
    pub fn press(&mut self, owner: u64, token: &str) -> Option<(String, bool)> {
        if !self.by_owner.entry(owner).or_default().insert(token.to_owned()) { return None; }
        let count = self.counts.entry(token.to_owned()).or_default();
        *count += 1;
        (*count == 1).then(|| (token.to_owned(), true))
    }
    pub fn release(&mut self, owner: u64) -> Vec<(String, bool)> {
        let mut events = Vec::new();
        if let Some(tokens) = self.by_owner.remove(&owner) {
            for token in tokens {
                if let Some(count) = self.counts.get_mut(&token) {
                    *count -= 1;
                    if *count == 0 {
                        self.counts.remove(&token);
                        events.push((token, false));
                    }
                }
            }
        }
        events
    }
    /// Use on focus loss, overlay shutdown and scene changes to prevent stuck keys.
    pub fn release_all(&mut self) -> Vec<(String, bool)> {
        self.by_owner.clear();
        std::mem::take(&mut self.counts).into_keys().map(|token| (token, false)).collect()
    }
}

/// The Android overlay and game loop run on different threads. Keep key edges
/// bounded and balance overlapping controls before they reach the game.
#[derive(Default)]
pub struct InputQueue {
    counts: BTreeMap<&'static str, usize>,
    delivered: BTreeSet<&'static str>,
    pending: std::collections::VecDeque<(&'static str, bool)>,
}
impl InputQueue {
    const MAX_PENDING: usize = 1024;
    const MAX_OWNERS_PER_KEY: usize = 128;
    pub fn send(&mut self, token: &'static str, down: bool) {
        if self.pending.len() >= Self::MAX_PENDING {
            // Do not leave a previously delivered input held after overload.
            self.pending.clear();
            self.counts.clear();
            self.pending.extend(self.delivered.iter().copied().map(|key| (key, false)));
            return;
        }
        if down {
            let count = self.counts.entry(token).or_default();
            if *count >= Self::MAX_OWNERS_PER_KEY { return; }
            *count += 1;
            if *count == 1 { self.pending.push_back((token, true)); }
        } else if let Some(count) = self.counts.get_mut(token) {
            *count -= 1;
            if *count == 0 {
                self.counts.remove(token);
                self.pending.push_back((token, false));
            }
        }
    }
    pub fn release_all(&mut self) {
        // Suppress undelivered presses as well as releases. Only keys already
        // seen by the consumer need a release; this avoids activating controls
        // after the window has lost focus and keeps the queue bounded.
        self.counts.clear();
        self.pending.clear();
        self.pending.extend(self.delivered.iter().copied().map(|key| (key, false)));
    }
    pub fn drain(&mut self) -> Vec<(&'static str, bool)> {
        let edges: Vec<_> = self.pending.drain(..).collect();
        for &(token, down) in &edges {
            if down { self.delivered.insert(token); } else { self.delivered.remove(token); }
        }
        edges
    }
}

fn shared_queue() -> &'static std::sync::Mutex<InputQueue> {
    static QUEUE: std::sync::OnceLock<std::sync::Mutex<InputQueue>> = std::sync::OnceLock::new();
    QUEUE.get_or_init(Default::default)
}
/// Accepts a recognized Pojav/GLFW or Zalith input token. Launcher UI and
/// layer actions remain the responsibility of their respective Android host.
pub fn submit_token(token: &'static str, down: bool) {
    shared_queue().lock().unwrap_or_else(|e| e.into_inner()).send(token, down);
}
pub fn submit_pojav(code: i32, down: bool) -> bool {
    enqueue_pojav(&mut shared_queue().lock().unwrap_or_else(|e| e.into_inner()), code, down)
}
pub fn drain_tokens() -> Vec<(&'static str, bool)> {
    shared_queue().lock().unwrap_or_else(|e| e.into_inner()).drain()
}
pub fn release_tokens() {
    shared_queue().lock().unwrap_or_else(|e| e.into_inner()).release_all();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapped_keys_are_known_and_both_forms_agree() {
        for &(code, name, token) in GLFW_KEYS {
            assert_eq!(glfw_key(code), Some(token));
            assert_eq!(glfw_key_named(name), Some(token));
            assert!(crate::keybinds::key_code(token).is_some(), "{token}");
        }
        assert_eq!(glfw_key(9999), None);
        assert_eq!(glfw_key_named("GLFW_KEY_F25"), None);
    }
    #[test]
    fn launcher_actions_are_not_confused_with_keys() {
        assert_eq!(pojav_code(-1), Some(ControlEvent::ToggleKeyboard));
        assert_eq!(pojav_code(-3), Some(ControlEvent::Token("key.mouse.left")));
        assert_eq!(pojav_code(87), Some(ControlEvent::Token("key.keyboard.w")));
        assert_eq!(pojav_code_android(51), Some(ControlEvent::Token("key.keyboard.w")));
        assert_eq!(pojav_code_android(29), Some(ControlEvent::Token("key.keyboard.a")));
        assert_eq!(pojav_code_android(47), Some(ControlEvent::Token("key.keyboard.s")));
        assert_eq!(pojav_code_android(32), Some(ControlEvent::Token("key.keyboard.d")));
        assert_eq!(pojav_code_android(133), Some(ControlEvent::Token("key.keyboard.f3")));
        assert_eq!(pojav_code_android(59), Some(ControlEvent::Token("key.keyboard.left.shift")));
        assert_eq!(pojav_code_android(62), Some(ControlEvent::Token("key.keyboard.space")));
        for code in [7, 8, 16, 19, 29, 54, 59, 61, 62, 66, 67, 111, 113, 131, 142, 144, 160] {
            let tok = android_keycode(code).unwrap();
            assert!(crate::keybinds::key_code(tok).is_some(), "{code} -> {tok}");
        }
        assert_eq!(zalith_event("switch_layer", "layer-id"), Some(ControlEvent::LayerSwitch));
        assert_eq!(zalith_event("launcher_event", "launcher.event.scroll_up"), Some(ControlEvent::ScrollUpHeld));
        assert_eq!(zalith_event("launcher_event", "launcher.event.scroll_up.single"), Some(ControlEvent::Token("mouse.scroll.up")));
        assert_eq!(zalith_event("launcher_event", "unimplemented"), None);
    }
    #[test]
    fn pojav_wheel_actions_follow_upstream_release_semantics() {
        let mut q = InputQueue::default();
        assert!(enqueue_pojav(&mut q, -7, true));
        assert!(q.drain().is_empty());
        assert!(enqueue_pojav(&mut q, -7, false));
        assert_eq!(q.drain(), vec![("mouse.scroll.up", true), ("mouse.scroll.up", false)]);
        assert!(enqueue_pojav(&mut q, -8, false));
        assert_eq!(q.drain(), vec![("mouse.scroll.down", true), ("mouse.scroll.down", false)]);
        assert!(!enqueue_pojav(&mut q, -1, true));
        assert!(!enqueue_pojav(&mut q, 0, true));
    }
    #[test]
    fn queue_balances_shared_keys_and_releases_after_focus_loss() {
        let mut q = InputQueue::default();
        q.send("key.keyboard.w", true);
        q.send("key.keyboard.w", true);
        assert_eq!(q.drain(), vec![("key.keyboard.w", true)]);
        q.send("key.keyboard.w", false);
        assert!(q.drain().is_empty());
        q.release_all();
        assert_eq!(q.drain(), vec![("key.keyboard.w", false)]);
        q.send("key.keyboard.w", false);
        assert!(q.drain().is_empty());
    }
    #[test]
    fn focus_loss_discards_undelivered_presses_but_releases_delivered_keys() {
        let mut q = InputQueue::default();
        q.send("key.keyboard.w", true);
        assert_eq!(q.drain(), vec![("key.keyboard.w", true)]);
        q.send("key.keyboard.w", false);
        q.send("key.keyboard.a", true);
        q.release_all();
        assert_eq!(q.drain(), vec![("key.keyboard.w", false)]);
        q.send("key.keyboard.a", false);
        assert!(q.drain().is_empty());
    }
    #[test]
    fn queue_overflow_releases_already_delivered_input() {
        let mut q = InputQueue::default();
        q.send("key.keyboard.w", true);
        q.drain();
        for _ in 0..InputQueue::MAX_PENDING {
            q.send("key.keyboard.a", true);
            q.send("key.keyboard.a", false);
        }
        assert!(q.drain().contains(&("key.keyboard.w", false)));
    }
    #[test]
    fn overlapping_presses_release_only_after_last_owner() {
        let mut t = HoldTracker::default();
        assert_eq!(t.press(1, "w"), Some(("w".into(), true)));
        assert_eq!(t.press(1, "w"), None);
        assert_eq!(t.press(2, "w"), None);
        assert!(t.release(1).is_empty());
        assert_eq!(t.release(2), vec![("w".into(), false)]);
        assert_eq!(t.press(3, "s"), Some(("s".into(), true)));
        assert_eq!(t.release_all(), vec![("s".into(), false)]);
        assert!(t.release(3).is_empty());
    }
}
