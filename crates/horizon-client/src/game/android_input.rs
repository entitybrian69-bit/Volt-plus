//! Extra desktop keys, a touch keyboard and menu scrolling on Android.
use super::*;
use horizon_client::{
    debug_keys::KeyAction,
    osk::Output,
    touch_ui::{Action, Gesture},
};
use winit::event::{Touch, TouchPhase};

impl Game<'_> {
    pub(crate) fn android_apply_display_scale(&mut self) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer)
            && let Err(error) = crate::display::apply(window, renderer)
        {
            crate::display::set_scale(crate::display::RenderScale::default());
            log::warn!("Android display scale: {error}");
        }
    }

    pub(super) fn android_keypad_menu_key(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
        event: &winit::event::KeyEvent,
    ) -> Result<bool> {
        if !self.pad_in_menu()
            || crate::platform::active()
            || self.ui.options.selected_key.is_some()
        {
            return Ok(false);
        }
        let Some(key) = horizon_client::keypad::resolve_key(event.physical_key, &event.logical_key)
        else {
            return Ok(false);
        };
        // Menu arrows work independently of the optional gameplay keypad profile.
        let arrow = matches!(
            key,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
        );
        if !self.settings.keypad.enabled && !arrow && !self.pad.menu.keyboard_focus {
            return Ok(false);
        }
        if !self.settings.keypad.enabled
            && !matches!(
                key,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Enter
                    | KeyCode::NumpadEnter
                    | KeyCode::Space
                    | KeyCode::Escape
            )
        {
            return Ok(false);
        }
        let Some(button) = self.settings.keypad.menu_button(key) else {
            return Ok(false);
        };
        if !event.repeat {
            self.pad.menu.keyboard_focus = true;
            self.pad.in_use = true;
            self.pad_menu_button(events, button, event.state == ElementState::Pressed)?;
        }
        Ok(true)
    }
    /// A freely moving physical pointer can use the same utility buttons as touch.
    pub(super) fn android_aux_mouse(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        const POINTER: u64 = u64::MAX - 2;
        if !self.settings.keypad.enabled || (self.captured && crate::keyboard_mouse_input()) {
            return false;
        }
        let WindowEvent::MouseInput {
            device_id,
            state,
            button: MouseButton::Left,
            ..
        } = event
        else {
            return false;
        };
        if *device_id == DeviceId::dummy() {
            return false;
        }
        let Some(batch) = self.gui_scale() else {
            return false;
        };
        let point = [self.cursor.0, self.cursor.1];
        let normalized = [point[0] / batch.width, point[1] / batch.height];
        // What the pad has under the pointer, and what it holds: touches are
        // position-dependent and change with the layout, ownership survives
        // screen transitions (a held press must complete before the pad
        // frees it, see `aux_mouse_phase`).
        let touched = self
            .pad
            .touch_ui
            .action(batch.width, batch.height, point, self.pad_in_menu())
            .is_some()
            || (!self.pad_in_menu() && self.pad.touch.auxiliary_at(normalized));
        let owned = self.pad.touch_ui.owns_key(POINTER)
            || self.pad.touch_ui.owns_mouse(POINTER)
            || self.pad.touch.owns_pointer(POINTER);
        let Some(phase) = horizon_client::touch_ui::aux_mouse_phase(
            *state == ElementState::Pressed,
            owned,
            touched,
        ) else {
            return false;
        };
        let touch = Touch {
            device_id: *device_id,
            phase,
            location: winit::dpi::PhysicalPosition::new(
                f64::from(point[0] * batch.scale),
                f64::from(point[1] * batch.scale),
            ),
            force: None,
            id: POINTER,
        };
        self.handle_window_event(events, id, WindowEvent::Touch(touch));
        true
    }

    fn token_maps_to_key(&self, token: &str, key: KeyCode) -> bool {
        if let Some(mapping) = token.strip_prefix("action:") {
            horizon_client::keybinds::key_code(
                horizon_client::keybinds::KeyBinds::default_of(mapping),
            ) == Some(key)
        } else {
            horizon_client::keybinds::key_code(token).is_some_and(|physical| {
                physical == key
                    || self
                        .settings
                        .keys
                        .translate_all(physical)
                        .contains(&Ok(key))
            })
        }
    }

    fn token_maps_to_mouse(&self, token: &str, button: MouseButton) -> bool {
        if let Some(mapping) = token.strip_prefix("action:") {
            horizon_client::keybinds::mouse_button(
                horizon_client::keybinds::KeyBinds::default_of(mapping),
            ) == Some(button)
        } else if horizon_client::keybinds::mouse_button(token) == Some(button) {
            true
        } else {
            horizon_client::keybinds::key_code(token).is_some_and(|physical| {
                self.settings
                    .keys
                    .translate_all(physical)
                    .contains(&Err(button))
            })
        }
    }

    pub(super) fn android_custom_key_held(&self, key: KeyCode) -> bool {
        let count = self
            .pad
            .touch
            .custom_hold_count_for(|tok| self.token_maps_to_key(tok, key));
        let skip = usize::from(
            self.pad
                .releasing_step_token
                .as_deref()
                .is_some_and(|tok| self.token_maps_to_key(tok, key)),
        );
        count > skip
            || (self.pad.releasing_step_token.is_none()
                && self
                    .pad
                    .late_custom_releases
                    .iter()
                    .any(|tok| self.token_maps_to_key(tok, key)))
    }
    pub(super) fn android_custom_mouse_held(&self, button: MouseButton) -> bool {
        let count = self
            .pad
            .touch
            .custom_hold_count_for(|tok| self.token_maps_to_mouse(tok, button));
        let skip = usize::from(
            self.pad
                .releasing_step_token
                .as_deref()
                .is_some_and(|tok| self.token_maps_to_mouse(tok, button)),
        );
        count > skip
            || (self.pad.releasing_step_token.is_none()
                && self
                    .pad
                    .late_custom_releases
                    .iter()
                    .any(|tok| self.token_maps_to_mouse(tok, button)))
    }
    fn is_hotbar_switch_token(token: &str) -> bool {
        matches!(
            token,
            "key.keyboard.1"
                | "key.keyboard.2"
                | "key.keyboard.3"
                | "key.keyboard.4"
                | "key.keyboard.5"
                | "key.keyboard.6"
                | "key.keyboard.7"
                | "key.keyboard.8"
                | "key.keyboard.9"
                | "action:key.hotbar.1"
                | "action:key.hotbar.2"
                | "action:key.hotbar.3"
                | "action:key.hotbar.4"
                | "action:key.hotbar.5"
                | "action:key.hotbar.6"
                | "action:key.hotbar.7"
                | "action:key.hotbar.8"
                | "action:key.hotbar.9"
                | "mouse.scroll.up"
                | "mouse.scroll.down"
                | "mouse.scroll.up.held"
                | "mouse.scroll.down.held"
        )
    }

    fn is_hotbar_switch_token_resolved(&self, token: &str) -> bool {
        if Self::is_hotbar_switch_token(token) {
            return true;
        }
        if let Some(code) = horizon_client::keybinds::key_code(token) {
            for mapped in self.settings.keys.translate_all(code) {
                if matches!(
                    mapped,
                    Ok(KeyCode::Digit1
                        | KeyCode::Digit2
                        | KeyCode::Digit3
                        | KeyCode::Digit4
                        | KeyCode::Digit5
                        | KeyCode::Digit6
                        | KeyCode::Digit7
                        | KeyCode::Digit8
                        | KeyCode::Digit9)
                ) {
                    return true;
                }
            }
        }
        if let Some(button) = horizon_client::keybinds::mouse_button(token) {
            for mapped in self.settings.keys.translate_mouse_all(button) {
                if matches!(
                    mapped,
                    Ok(KeyCode::Digit1
                        | KeyCode::Digit2
                        | KeyCode::Digit3
                        | KeyCode::Digit4
                        | KeyCode::Digit5
                        | KeyCode::Digit6
                        | KeyCode::Digit7
                        | KeyCode::Digit8
                        | KeyCode::Digit9)
                ) {
                    return true;
                }
            }
        }
        false
    }

    fn is_tick_deferred_hand_token(&self, token: &str) -> bool {
        if matches!(
            token,
            "key.mouse.left"
                | "key.mouse.right"
                | "key.mouse.middle"
                | "action:key.attack"
                | "action:key.use"
                | "action:key.pickItem"
        ) {
            return true;
        }
        if self.remote.is_some()
            && matches!(
                token,
                "key.keyboard.f" | "key.keyboard.q" | "action:key.swapOffhand" | "action:key.drop"
            )
        {
            return true;
        }
        if let Some(code) = horizon_client::keybinds::key_code(token) {
            for mapped in self.settings.keys.translate_all(code) {
                match mapped {
                    Err(MouseButton::Left | MouseButton::Right | MouseButton::Middle) => {
                        return true;
                    }
                    Ok(KeyCode::KeyF | KeyCode::KeyQ) if self.remote.is_some() => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        if let Some(button) = horizon_client::keybinds::mouse_button(token) {
            for mapped in self.settings.keys.translate_mouse_all(button) {
                match mapped {
                    Err(MouseButton::Left | MouseButton::Right | MouseButton::Middle) => {
                        return true;
                    }
                    Ok(KeyCode::KeyF | KeyCode::KeyQ) if self.remote.is_some() => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    }

    pub(super) fn android_custom_events(&mut self) {
        let events = self.pad.touch.key_events();
        if !events.is_empty() {
            self.android_process_custom_events(events);
        }
    }

    fn enqueue_deferred_custom_release(&mut self, name: String) {
        if let Some(idx) = self
            .pad
            .deferred_custom_events
            .iter()
            .rposition(|(tok, d)| *d && tok == &name)
        {
            self.pad.deferred_custom_events.insert(idx + 1, (name, false));
        } else {
            self.pad.deferred_custom_events.push_back((name, false));
        }
    }

    pub(super) fn android_flush_late_custom_releases(&mut self) {
        if self.pad.late_custom_releases.is_empty() {
            return;
        }
        let ticking = self.title.is_none() && self.screen == Screen::Playing && !self.pause;
        for token in std::mem::take(&mut self.pad.late_custom_releases) {
            if !ticking || self.pad.custom_pressed_at.get(&token) != Some(&self.ticks) {
                self.android_dispatch_custom_token(&token, false);
            } else {
                self.pad.late_custom_releases.push(token);
            }
        }
    }

    fn android_release_step_hand_tokens(&mut self) {
        let tokens = std::mem::take(&mut self.pad.step_hand_tokens);
        if tokens.is_empty() {
            return;
        }
        self.android_flush_late_custom_releases();
        for token in tokens {
            self.pad.late_custom_releases.retain(|t| t != &token);
            self.pad.releasing_step_token = Some(token.clone());
            self.android_dispatch_custom_token(&token, false);
            self.pad.releasing_step_token = None;
        }
        if !self.right && self.using.is_some() {
            if self.remote.is_some() {
                self.release_remote_use();
            } else {
                self.tick_using();
            }
        }
    }

    pub(super) fn android_process_custom_events(
        &mut self,
        events: impl IntoIterator<Item = (String, bool)>,
    ) {
        let in_gameplay = self.title.is_none()
            && self.screen == Screen::Playing
            && !self.pause
            && !self.pad_in_menu();
        let mut hand_action_queued = in_gameplay
            && (self.attack_clicks > 0
                || self.use_clicks > 0
                || self
                    .remote
                    .as_ref()
                    .is_some_and(|r| r.input.has_pending() || !r.pending_keys.is_empty()));
        let mut hotbar_switched_this_tick = false;
        let mut deferring = false;
        let mut immediate_hand_tokens: Vec<String> = Vec::new();
        for (name, down) in events {
            if !in_gameplay {
                self.android_custom_token_edge(&name, down);
                continue;
            }
            if deferring {
                if down {
                    self.pad.deferred_custom_events.push_back((name, true));
                } else {
                    self.enqueue_deferred_custom_release(name);
                }
                continue;
            }
            if down {
                let is_hotbar = self.is_hotbar_switch_token_resolved(&name);
                let is_hand = self.is_tick_deferred_hand_token(&name);
                let needs_next_tick = (hand_action_queued && (is_hotbar || is_hand))
                    || (hotbar_switched_this_tick && is_hotbar)
                    || (self.pad.deferred_custom_events.iter().any(|(_, d)| *d)
                        && (is_hotbar || is_hand));
                if needs_next_tick {
                    deferring = true;
                    self.pad.deferred_custom_events.push_back((name, true));
                    continue;
                }
                self.android_custom_token_edge(&name, true);
                if is_hand {
                    hand_action_queued = true;
                    if !immediate_hand_tokens.iter().any(|t| t == &name) {
                        immediate_hand_tokens.push(name);
                    }
                }
                if is_hotbar {
                    hotbar_switched_this_tick = true;
                }
            } else {
                if self
                    .pad
                    .deferred_custom_events
                    .iter()
                    .any(|(tok, d)| *d && tok == &name)
                {
                    self.enqueue_deferred_custom_release(name);
                    continue;
                }
                self.android_custom_token_edge(&name, false);
            }
        }
        if self.pad.deferred_custom_events.iter().any(|(_, d)| *d) {
            for tok in immediate_hand_tokens {
                if !self.pad.step_hand_tokens.iter().any(|t| t == &tok) {
                    self.pad.step_hand_tokens.push(tok);
                }
            }
        }
    }

    pub(super) fn android_drain_deferred_custom_events(&mut self) {
        if self.pad.deferred_custom_events.is_empty() {
            return;
        }
        let in_gameplay = self.title.is_none()
            && self.screen == Screen::Playing
            && !self.pause
            && !self.pad_in_menu();
        if !in_gameplay {
            self.pad.step_hand_tokens.clear();
            while let Some((name, down)) = self.pad.deferred_custom_events.pop_front() {
                self.android_custom_token_edge(&name, down);
            }
            return;
        }
        let mut hand_action_queued = self.attack_clicks > 0
            || self.use_clicks > 0
            || self
                .remote
                .as_ref()
                .is_some_and(|r| r.input.has_pending() || !r.pending_keys.is_empty());
        let mut hotbar_switched_this_tick = false;
        let mut released_prev_step = false;
        while let Some((name, down)) = self.pad.deferred_custom_events.front().cloned() {
            let is_hotbar = down && self.is_hotbar_switch_token_resolved(&name);
            let is_hand = down && self.is_tick_deferred_hand_token(&name);
            if down
                && ((hand_action_queued && (is_hotbar || is_hand))
                    || (hotbar_switched_this_tick && is_hotbar))
            {
                break;
            }
            if down && !released_prev_step {
                released_prev_step = true;
                self.android_release_step_hand_tokens();
            }
            self.pad.deferred_custom_events.pop_front();
            self.android_custom_token_edge(&name, down);
            if is_hand {
                hand_action_queued = true;
                if !self.pad.step_hand_tokens.iter().any(|t| t == &name) {
                    self.pad.step_hand_tokens.push(name);
                }
            }
            if is_hotbar {
                hotbar_switched_this_tick = true;
            }
        }
        if !self.pad.deferred_custom_events.iter().any(|(_, d)| *d) {
            self.pad.step_hand_tokens.clear();
        }
    }

    pub(super) fn android_custom_token_edge(&mut self, token: &str, down: bool) {
        let ticking = self.title.is_none()
            && self.screen == Screen::Playing
            && !self.pause
            && !self.pad_in_menu();
        if down {
            if let Some(i) = self
                .pad
                .late_custom_releases
                .iter()
                .position(|t| t == token)
            {
                let prev = self.pad.late_custom_releases.remove(i);
                self.android_dispatch_custom_token(&prev, false);
            }
            self.pad
                .custom_pressed_at
                .insert(token.to_owned(), self.ticks);
        } else if ticking && self.pad.custom_pressed_at.get(token) == Some(&self.ticks) {
            if !self.pad.late_custom_releases.iter().any(|t| t == token) {
                self.pad.late_custom_releases.push(token.to_owned());
            }
            return;
        } else {
            self.pad.step_hand_tokens.retain(|t| t != token);
        }
        self.android_dispatch_custom_token(token, down);
    }

    pub(super) fn android_custom_token_tap(&mut self, token: &str) {
        self.android_custom_token_edge(token, true);
        if token != "mouse.virtual.toggle"
            && token != "mouse.scroll.up"
            && token != "mouse.scroll.down"
        {
            self.android_custom_token_edge(token, false);
        }
    }

    fn android_step_scroll(&mut self, up: bool) {
        let sign = if up { 1.0_f32 } else { -1.0_f32 };
        let d = sign * self.settings.mouse_wheel_sensitivity;
        if self.dialog_open() {
            self.dialog_scroll(d);
            return;
        }
        if self.code_of_conduct.is_some() {
            self.code_of_conduct_scroll(d);
            return;
        }
        if self.screen == Screen::Options {
            let _ = self.options_scroll(d);
            return;
        }
        if self.screen == Screen::Chat {
            self.chat_scroll(d);
            return;
        }
        if self.screen == Screen::WorldOptions {
            self.world_options_scroll(d);
            return;
        }
        if self.screen == Screen::DebugOptions {
            self.debug_options_scroll(d);
            return;
        }
        if self.screen == Screen::Mod {
            self.mod_screen_scroll(d);
            return;
        }
        let scale = self.gui_scale();
        if let Some(title) = &mut self.title {
            if let Some(b) = scale {
                title.scroll_worlds(d, self.cursor, (b.width, b.height));
            }
            return;
        }
        let delta = if up { 8.0 } else { 1.0 };
        let next = horizon_client::mouse::ScrollWheelHandler::next_selection(
            delta,
            self.selected as i32,
            9,
        );
        self.selected = next as usize;
    }

    pub(super) fn android_dispatch_custom_token(&mut self, name: &str, down: bool) {
        self.last_input = Instant::now();
        self.pad.in_use = true;
        if down && !self.pad_in_menu() && !self.captured && self.focused {
            self.capture(true);
        }
        if self.screen == Screen::Playing {
            let _ = self.volt_handle_key_token(name, down, false);
        }
        if self.screen == Screen::Options && self.options_waiting_for_key() {
            if !down {
                return;
            }
            if let Some(mapping) = name.strip_prefix("action:") {
                let default = horizon_client::keybinds::KeyBinds::default_of(mapping);
                if let Some(key) = horizon_client::keybinds::key_code(default) {
                    let _ = self.options_key(key, None);
                } else if let Some(button) = horizon_client::keybinds::mouse_button(default) {
                    let _ = self.options_bind_mouse(button);
                }
                return;
            }
            if let Some(button) = horizon_client::keybinds::mouse_button(name) {
                let _ = self.options_bind_mouse(button);
                return;
            }
            if let Some(key) = horizon_client::keybinds::key_code(name) {
                let _ = self.options_key(key, None);
                return;
            }
        }
        if let Some(mapping) = name.strip_prefix("action:") {
            if mapping == "horizon.toggle_keyboard" {
                if down {
                    self.android_show_keyboard();
                }
                return;
            }
            if mapping == "horizon.open_menu" {
                if down {
                    if self.pad_in_menu() {
                        let _ = self.pad_back();
                    } else {
                        let _ = self.key_pressed(KeyCode::Escape);
                    }
                }
                return;
            }
            if let Some(text_msg) = mapping.strip_prefix("horizon.send_text:") {
                if down {
                    self.open_chat(text_msg);
                }
                return;
            }
            if mapping == "key.sneak" || mapping == "key.sprint" {
                let slot = usize::from(mapping == "key.sprint");
                let key = [KeyCode::ShiftLeft, KeyCode::ControlLeft][slot];
                if down {
                    self.pad.custom_keys.insert(key);
                    self.physical_keys.insert(key);
                    self.keys.insert(key);
                } else {
                    self.pad.custom_keys.remove(&key);
                    self.physical_keys.remove(&key);
                    self.keys.remove(&key);
                }
                self.toggles.drop_pad_toggle(slot);
                self.toggles.0[slot].set_down(down, false);
                self.sync_toggle(slot);
                if slot == 1 && !down {
                    if let Some(player) = &mut self.player {
                        player.sprinting = false;
                    }
                }
                return;
            }
            if self.pad_in_menu()
                && down
                && mapping == "key.inventory"
                && matches!(self.screen, Screen::Inventory | Screen::Creative)
            {
                let _ = self.pad_back();
                return;
            }
            let default = horizon_client::keybinds::KeyBinds::default_of(mapping);
            let result = if let Some(key) = horizon_client::keybinds::key_code(default) {
                self.mapped_key(key, down, false)
            } else if let Some(button) = horizon_client::keybinds::mouse_button(default) {
                self.android_custom_mouse(button, down)
            } else {
                Ok(())
            };
            if let Err(error) = result {
                log::warn!("Custom touch action: {error}");
            }
            if down && self.screen == Screen::Chat {
                self.android_show_keyboard();
            }
            return;
        }
        if let Some(button) = horizon_client::keybinds::mouse_button(name) {
            if let Err(error) = self.android_custom_mouse(button, down) {
                log::warn!("Custom mouse button: {error}");
            }
            return;
        }
        if matches!(
            name,
            "mouse.scroll.up" | "mouse.scroll.down" | "mouse.scroll.up.held" | "mouse.scroll.down.held"
        ) {
            let up = name.starts_with("mouse.scroll.up");
            let held = name.ends_with(".held");
            if down {
                self.android_step_scroll(up);
                if held {
                    self.pad.held_scroll = Some((up, Instant::now()));
                }
            } else if held && self.pad.held_scroll.is_some_and(|(u, _)| u == up) {
                self.pad.held_scroll = None;
            }
            return;
        }
        if name == "mouse.look" {
            return;
        }
        if name == "mouse.virtual.toggle" {
            if down {
                self.pad.touch_ui.goy_in_game_mouse = false;
                let next = !self.pad.touch.settings.virtual_mouse_enabled;
                self.pad.touch.settings.virtual_mouse_enabled = next;
                self.pad.touch_ui.pointer_off = !next;
                let _ = self.pad.touch.settings.save();
                self.settings.touch = self.pad.touch.settings.clone();
                self.android_sync_pointer();
                self.android_touch_haptic(14);
            }
            return;
        }
        let Some(key) = horizon_client::keybinds::key_code(name) else {
            return;
        };
        if down {
            self.pad.custom_keys.insert(key);
            self.physical_keys.insert(key);
        } else {
            self.pad.custom_keys.remove(&key);
            self.physical_keys.remove(&key);
        }
        if down
            && horizon_client::keybinds::key_name(key)
                == Some(self.settings.keys.get("key.fullscreen"))
        {
            self.toggle_fullscreen();
            return;
        }
        if down && self.screenshot_shortcut(key, false) {
            return;
        }
        let action = if down {
            KeyAction::Press
        } else {
            KeyAction::Release
        };
        if self.debug_key(key, action) {
            return;
        }
        self.mod_key(key, down, false);
        self.native_key_event(key, down, false);
        if self.screen == Screen::Options {
            if down {
                let _ = self.options_key(key, None);
            }
            return;
        }
        // Raw Shift and Control are the touch Sneak and Sprint buttons:
        // set their toggle state directly so both Hold and Toggle modes work reliably,
        // and also update `self.keys` so inventory Shift-click works.
        if matches!(
            key,
            KeyCode::ShiftLeft | KeyCode::ShiftRight | KeyCode::ControlLeft | KeyCode::ControlRight
        ) {
            if down {
                self.keys.insert(key);
            } else {
                self.keys.remove(&key);
            }
            let slot = usize::from(matches!(key, KeyCode::ControlLeft | KeyCode::ControlRight));
            self.toggles.drop_pad_toggle(slot);
            self.toggles.0[slot].set_down(down, false);
            self.sync_toggle(slot);
            if slot == 1 && !down {
                if let Some(player) = &mut self.player {
                    player.sprinting = false;
                }
            }
            for mapped in self.settings.keys.translate_all(key) {
                match mapped {
                    Ok(code)
                        if !matches!(
                            code,
                            KeyCode::ShiftLeft
                                | KeyCode::ShiftRight
                                | KeyCode::ControlLeft
                                | KeyCode::ControlRight
                        ) =>
                    {
                        let _ = self.mapped_key(code, down, false);
                    }
                    Err(button) => {
                        let _ = self.android_custom_mouse(button, down);
                    }
                    _ => {}
                }
            }
            return;
        }
        if self.pad_in_menu() && down {
            if key == KeyCode::Escape {
                let _ = self.pad_back();
                return;
            }
            if self.dialog_open() {
                let _ = self.dialog_key(key, None, None);
                return;
            }
            if self.screen == Screen::Paused && self.pause_menu {
                let _ = self.pause_key(key);
                return;
            }
            if self.screen == Screen::Dead {
                let _ = self.death_key(key);
                return;
            }
            if key == KeyCode::KeyE && matches!(self.screen, Screen::Inventory | Screen::Creative) {
                let _ = self.pad_back();
                return;
            }
            if self.screen == Screen::Inventory
                && (self.merchant_key(key) || self.recipe_book_key(key))
            {
                return;
            }
            if self.screen == Screen::Chat && self.title.is_none() {
                self.keys.insert(key);
                self.chat_key_code(key, None);
                return;
            }
            if self.title.is_some() {
                let (w, h) = self.gui_size();
                let shift = self.has_shift_down();
                if let Some(t) = self.title.as_mut() {
                    match key {
                        KeyCode::Escape => {
                            t.escape();
                            self.save_title_options();
                        }
                        KeyCode::F5 => t.refresh_servers(),
                        KeyCode::Tab => t.tab_dir(shift, (w, h)),
                        KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::ArrowUp
                        | KeyCode::ArrowDown => t.arrow_key(key, (w, h)),
                        KeyCode::Backspace => self.title_edit(None, true),
                        _ => {}
                    }
                }
                return;
            }
        }
        if !down && self.screen == Screen::Chat && self.title.is_none() {
            self.keys.remove(&key);
        }
        if self.screen == Screen::Playing
            && down
            && horizon_client::keybinds::key_name(key)
                .is_some_and(|n| self.settings.keys.get("key.smoothCamera") == n)
        {
            self.smooth_camera = !self.smooth_camera;
        }
        let translated = self.settings.keys.translate_all(key);
        if translated.is_empty() {
            if down {
                self.keys.insert(key);
            } else {
                self.keys.remove(&key);
            }
            if down && matches!(self.screen, Screen::Inventory | Screen::Creative) {
                let _ = self.key_pressed(key);
            }
        } else {
            for mapped in translated {
                let result = match mapped {
                    Ok(code) => self.mapped_key(code, down, false),
                    Err(button) => self.android_custom_mouse(button, down),
                };
                if let Err(error) = result {
                    log::warn!("Custom touch key: {error}");
                }
            }
        }
        if down && matches!(key, KeyCode::KeyT | KeyCode::Slash) && self.screen == Screen::Chat {
            self.android_show_keyboard();
        }
    }

    pub(super) fn android_touch_haptic(&self, millis: u32) {
        if self.pad.touch.settings.vibrate {
            crate::platform::vibrate(horizon_client::gamepad::Pulse {
                strong: 0.30,
                weak: 0.55,
                millis,
            });
        }
    }

    pub(super) fn android_set_virtual_mouse(&mut self, enabled: bool) {
        if !enabled {
            self.pad.touch_ui.cancel_relative();
            for right in self.pad.touch_ui.release_all_mouse() {
                let button = if right {
                    MouseButton::Right
                } else {
                    MouseButton::Left
                };
                let _ = self.android_custom_mouse(button, false);
            }
            self.pad.touch_ui.virtual_mouse = false;
            if self.captured && crate::keyboard_mouse_input() && !self.os_mouse_captured {
                self.os_mouse_captured = crate::platform::capture_mouse(true);
            }
            return;
        }
        self.pad.touch_ui.virtual_mouse = true;
        if self.os_mouse_captured {
            self.os_mouse_captured = false;
            let _ = crate::platform::capture_mouse(false);
        }
        if let Some(b) = self.gui_scale() {
            self.pad
                .touch_ui
                .sync_viewport_cursor(&mut self.cursor, [b.width, b.height]);
        }
    }

    /// State-driven Virtual Mouse visibility synchronization across MCPE and GoyControl.
    pub(super) fn android_sync_pointer(&mut self) {
        self.pad.touch_ui.pointer_off = !self.pad.touch.settings.virtual_mouse_enabled;
        let in_menu = self.pad_in_menu();
        if let Some(b) = self.gui_scale() {
            self.pad
                .touch_ui
                .sync_viewport_cursor(&mut self.cursor, [b.width, b.height]);
        }
        let want = if self.replay.is_some() && !in_menu {
            false
        } else {
            self.pad.touch_ui.compute_virtual_mouse_visibility(
                &self.pad.touch,
                in_menu || self.volt_jei_visible(),
                self.ui.hide_gui,
                crate::touch_visible(),
                self.captured,
            )
        };
        if self.pad.touch_ui.virtual_mouse != want {
            self.android_set_virtual_mouse(want);
        }
    }

    fn android_custom_mouse(&mut self, button: MouseButton, down: bool) -> Result<()> {
        if self.screen == Screen::Options && self.options_waiting_for_key() {
            // The virtual mouse drives the picker: a tap on its plate picks the
            // key under the pointer, a press anywhere else binds the button.
            if down && self.options_picker_over_panel() {
                self.options_picker_press();
                return Ok(());
            }
            if !down && self.ui.options.picker_armed() {
                return self.options_picker_click();
            }
            if down {
                return self.options_bind_mouse(button);
            }
            return Ok(());
        }
        if self.volt_jei_visible()
            && matches!(button, MouseButton::Left | MouseButton::Right)
            && let Some(b) = self.gui_scale()
        {
            if down {
                if self.volt_jei_click(b.width, b.height, button == MouseButton::Right) {
                    return Ok(());
                }
            } else if self.volt_jei_contains(b.width, b.height) {
                return Ok(());
            }
        }
        if self.pad_in_menu() && !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            let right = button == MouseButton::Right;
            if matches!(button, MouseButton::Left | MouseButton::Right) {
                let id = self.window.as_ref().map_or(WindowId::dummy(), |w| w.id());
                return self.android_menu_mouse(&super::event_loop::Noop, id, right, down);
            }
            return Ok(());
        }
        if !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            if down && !self.captured && self.focused {
                self.capture(true);
            }
            self.extras_mouse(button, down);
            self.mod_mouse_key(button, down);
            if down
                && self.settings.keys.get("key.smoothCamera")
                    == horizon_client::keybinds::mouse_name(button)
            {
                self.smooth_camera = !self.smooth_camera;
            }
            let mapped = self.settings.keys.translate_mouse_all(button);
            if mapped.is_empty() {
                return self.play_mouse(button, down);
            }
            for item in mapped {
                match item {
                    Err(b) => self.play_mouse(b, down)?,
                    Ok(k) => self.mapped_key(k, down, false)?,
                }
            }
            return Ok(());
        }
        if let Some(key) = self
            .settings
            .keys
            .container_mouse_key(button, self.creative)
        {
            if down && self.carried.is_none() {
                self.inventory_hotkey(key);
            }
            return Ok(());
        }
        let right = button == MouseButton::Right;
        if !matches!(button, MouseButton::Left | MouseButton::Right) {
            if self.settings.keys.get("key.pickItem")
                == horizon_client::keybinds::mouse_name(button)
            {
                if down {
                    self.inventory_pick_press();
                } else {
                    self.inventory_pick_release();
                }
                if let Some(container) = self.container {
                    self.container_changed(container);
                }
            }
            return Ok(());
        }
        if down {
            if self.screen == Screen::Creative {
                self.click_creative(right);
            } else {
                self.inventory_press(right);
            }
            if let Some(container) = self.container {
                self.container_changed(container);
            }
        } else {
            self.inventory_mouse_release(right);
            if self.screen == Screen::Creative && !right {
                self.creative_screen.release();
            }
            self.station_release();
            self.recipe_book_release();
            self.loom_release();
        }
        Ok(())
    }

    fn android_menu_mouse(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
        right: bool,
        down: bool,
    ) -> Result<()> {
        let button = if right {
            MouseButton::Right
        } else {
            MouseButton::Left
        };
        if matches!(self.screen, Screen::Inventory | Screen::Creative) {
            self.android_custom_mouse(button, down)
        } else {
            let id = if id == WindowId::dummy() {
                self.window.as_ref().map_or(WindowId::dummy(), |w| w.id())
            } else {
                id
            };
            self.pad_synthetic_event(
                events,
                id,
                WindowEvent::MouseInput {
                    device_id: DeviceId::dummy(),
                    button,
                    state: if down {
                        ElementState::Pressed
                    } else {
                        ElementState::Released
                    },
                },
            );
            Ok(())
        }
    }
    pub(super) fn android_aux_touch(
        &mut self,
        t: &Touch,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
    ) -> Result<bool> {
        self.pad.touch_ui.goy_mode = self.pad.touch.settings.control_style
            == horizon_client::touch::ControlStyle::GoyControl;
        if t.phase == TouchPhase::Ended
            && let Some((right, last)) = self.pad.touch_ui.release_mouse(t.id)
        {
            if last {
                self.android_menu_mouse(events, id, right, false)?;
            }
            return Ok(true);
        }
        if t.phase == TouchPhase::Cancelled {
            self.pad.touch_ui.cancel();
            self.android_release_touch_keys(true);
            return Ok(true);
        }
        if t.phase == TouchPhase::Ended && self.pad.touch_ui.owns_key(t.id) {
            if let Some(key) = self.pad.touch_ui.key_up(t.id) {
                self.android_key_up(key);
            }
            return Ok(true);
        }
        let size = if let Some(r) = self.renderer.as_ref() {
            winit::dpi::PhysicalSize::new(r.config.width, r.config.height)
        } else if let Some(window) = self.window.as_ref() {
            window.inner_size()
        } else {
            return Ok(false);
        };
        let mut b = HudBatch::new([size.width, size.height]);
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        #[cfg(target_os = "android")]
        if self.pad_in_menu()
            && !self.pad.touch_ui.floating_menu
            && self.android_updater_modal_touch(t.phase, [t.location.x as f32, t.location.y as f32], size.width, size.height)
        {
            self.pad.touch.clear();
            self.pad.touch.ignore(t.id);
            return Ok(true);
        }
        if self.pad.touch_ui.overlay_button_visible(
            &self.pad.touch.settings,
            self.pad_in_menu(),
            horizon_client::recorder::hiding_controls(),
        ) && !self.pad.touch_ui.owns_relative(t.id)
            && !self.pad.touch_ui.owns_gesture(t.id)
            && {
                self.sync_volt_touch_ui();
                true
            }
            && let Some(menu_act) = self.pad.touch_ui.floating_menu_touch(
                &mut b,
                &self.gui.atlas,
                t.id,
                t.phase,
                p,
                &mut self.pad.touch.settings,
            )
        {
            use horizon_client::touch_ui::InGameMenuAction;
            self.android_release_touch_keys(false);
            self.pad.touch.clear();
            self.pad.touch.ignore(t.id);
            if t.phase == TouchPhase::Started {
                self.audio.play_button_click();
                self.android_touch_haptic(12);
            }
            match menu_act {
                InGameMenuAction::Close => {}
                InGameMenuAction::OpenControlEditor => {
                    self.pad.touch.open_editor();
                    self.android_sync_pointer();
                }
                InGameMenuAction::SetControlStyle(style) => {
                    self.android_release_touch_keys(true);
                    self.pad.touch_ui.goy_in_game_mouse = false;
                    self.pad.touch.clear();
                    self.pad
                        .touch
                        .settings
                        .switch_control_system(std::path::Path::new("."), style);
                    self.pad.touch_ui.goy_mode =
                        style == horizon_client::touch::ControlStyle::GoyControl;
                    self.pad.touch_ui.refresh_presets_cache(style);
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.android_sync_pointer();
                }
                InGameMenuAction::SelectPreset(preset_id) => {
                    let cur_style = self.pad.touch.settings.control_style;
                    self.pad.touch.settings.select_preset(
                        std::path::Path::new("."),
                        cur_style,
                        &preset_id,
                    );
                    self.pad.touch_ui.refresh_presets_cache(cur_style);
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::CopyPreset {
                    source_id,
                    name,
                    description,
                } => {
                    let cur_style = self.pad.touch.settings.control_style;
                    if let Ok(meta) = horizon_client::touch::Settings::copy_or_replace_preset_by_name(
                        std::path::Path::new("."),
                        cur_style,
                        &source_id,
                        &name,
                        &description,
                    ) {
                        self.pad.touch.settings.select_preset(
                            std::path::Path::new("."),
                            cur_style,
                            &meta.id,
                        );
                        self.pad.touch_ui.refresh_presets_cache(cur_style);
                        self.settings.touch = self.pad.touch.settings.clone();
                    }
                }
                InGameMenuAction::UpdatePresetMeta {
                    id: preset_id,
                    name,
                    description,
                } => {
                    let cur_style = self.pad.touch.settings.control_style;
                    let _ = self.pad.touch.settings.update_preset_meta(
                        std::path::Path::new("."),
                        cur_style,
                        &preset_id,
                        &name,
                        &description,
                    );
                    self.pad.touch_ui.refresh_presets_cache(cur_style);
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ApplyPreset(name) => {
                    self.pad.touch.settings.apply_preset(name);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::LoadSavedProfile(idx) => {
                    let saved =
                        horizon_client::touch::Settings::list_profiles(std::path::Path::new("."));
                    if let Some(name) = saved.get(idx) {
                        self.pad.touch.settings = horizon_client::touch::Settings::load_profile(
                            std::path::Path::new("."),
                            name,
                        );
                        self.settings.touch = self.pad.touch.settings.clone();
                    }
                }
                InGameMenuAction::PresetVisibilityAll => {
                    self.pad.touch.settings.apply_visibility_preset_all();
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::CycleMovementMode => {
                    use horizon_client::touch::Mode;
                    self.pad.touch.settings.mode = match self.pad.touch.settings.mode {
                        Mode::Dpad => Mode::Crosshair,
                        Mode::Crosshair => Mode::Tap,
                        Mode::Tap => Mode::Dpad,
                    };
                    if self.pad.touch.settings.mode != Mode::Crosshair {
                        self.pad.touch.settings.split_controls = false;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::CycleJoystickArea => {
                    self.pad.touch.settings.joystick_style.move_area =
                        self.pad.touch.settings.joystick_style.move_area.next();
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleSplitTouch => {
                    self.pad.touch.settings.toggle_split_touch();
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleInvertY => {
                    let next = !self.pad.touch.settings.invert_y;
                    self.pad.touch.settings.invert_y = next;
                    self.settings.invert_y = next;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.settings.save(&Self::options_path());
                }
                InGameMenuAction::ToggleLeftHanded => {
                    self.pad.touch.settings.left_handed = !self.pad.touch.settings.left_handed;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleSprintMode => {
                    let next = !self.pad.touch.settings.sprint_toggle;
                    self.pad.touch.settings.sprint_toggle = next;
                    self.pad.touch.settings.toggle_sprint = next;
                    self.settings.toggle_sprint = next;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.settings.save(&Self::options_path());
                }
                InGameMenuAction::ToggleInventoryControls => {
                    self.pad.touch.settings.inventory_controls =
                        !self.pad.touch.settings.inventory_controls;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.android_sync_pointer();
                }
                InGameMenuAction::ToggleVibrate => {
                    let next = !self.pad.touch.settings.vibrate;
                    self.pad.touch.settings.vibrate = next;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    if next {
                        self.android_touch_haptic(22);
                    } else {
                        crate::platform::cancel_vibration();
                    }
                }
                InGameMenuAction::ToggleAutoJump => {
                    let next = !self.pad.touch.settings.auto_jump;
                    self.pad.touch.settings.auto_jump = next;
                    self.settings.auto_jump = next;
                    if let Some(player) = &mut self.player {
                        player.auto_jump_enabled = next;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.settings.save(&Self::options_path());
                }
                InGameMenuAction::ToggleGyro => {
                    self.pad.touch.settings.gyroscope = !self.pad.touch.settings.gyroscope;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleVirtualMouse => {
                    let next = !self.pad.touch.settings.virtual_mouse_enabled;
                    self.pad.touch.settings.virtual_mouse_enabled = next;
                    self.pad.touch_ui.pointer_off = !next;
                    if !next {
                        self.pad.touch_ui.goy_in_game_mouse = false;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    self.android_sync_pointer();
                }
                InGameMenuAction::SetVirtualMouseSpeed(spd) => {
                    self.pad.touch.settings.virtual_mouse_sensitivity = spd.clamp(0.25, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepVirtualMouseSpeed(dir) => {
                    self.pad.touch.settings.virtual_mouse_sensitivity =
                        (self.pad.touch.settings.virtual_mouse_sensitivity + dir as f32 * 0.25)
                            .clamp(0.25, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetVirtualMousePointerSize(scale) => {
                    self.pad.touch.settings.virtual_mouse_pointer_scale = scale.clamp(0.5, 2.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetGyroSensitivity(val) => {
                    self.pad.touch.settings.gyro_sensitivity = val.clamp(0.10, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepGyroSensitivity(dir) => {
                    self.pad.touch.settings.gyro_sensitivity =
                        (self.pad.touch.settings.gyro_sensitivity + dir as f32 * 0.20)
                            .clamp(0.20, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetTouchSensitivity(sens) => {
                    self.pad.touch.settings.sensitivity = sens.clamp(0.10, 3.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepTouchSensitivity(dir) => {
                    self.pad.touch.settings.sensitivity =
                        (self.pad.touch.settings.sensitivity + dir as f32 * 0.15).clamp(0.10, 3.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetButtonScale(val) => {
                    self.pad.touch.settings.button_scale = val.clamp(0.50, 2.00);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::StepButtonScale(dir) => {
                    self.pad.touch.settings.button_scale =
                        (self.pad.touch.settings.button_scale + dir as f32 * 0.10).clamp(0.5, 2.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::TriggerKey(key) => {
                    if !self.debug_key(key, KeyAction::Press) {
                        self.key_pressed(key)?;
                    }
                    self.debug_key(key, KeyAction::Release);
                    if matches!(key, KeyCode::KeyT | KeyCode::Slash) && self.screen == Screen::Chat
                    {
                        self.android_show_keyboard();
                    }
                }
                InGameMenuAction::OpenKeyboard => {
                    self.android_show_keyboard();
                }
                InGameMenuAction::ForceKill => {
                    let _ = self.pad.touch.settings.save();
                    std::process::exit(0);
                }
                InGameMenuAction::OpenLogs => {
                    // Logs are private to the app; the SAF provider shares the Reports
                    // folder the diagnostics exporter writes into.
                    if !crate::platform::open_documents("reports") {
                        self.system_message(horizon_client::text::Text::literal(
                            "Could not open Reports. Logs are in the game folder's logs/ directory.",
                        ));
                    }
                }
                InGameMenuAction::EditPresetField(description) => {
                    if let Some(dlg) = self.pad.touch_ui.preset_dialog.as_mut() {
                        dlg.editing_desc = description;
                        crate::platform::show(if description {
                            &dlg.desc_input
                        } else {
                            &dlg.name_input
                        });
                    }
                }
                InGameMenuAction::SetResolutionScale(pct) => {
                    let clamped = pct.clamp(25, 100);
                    let changed = self.pad.touch.settings.resolution_scale_percent != clamped
                        || self.settings.horizon.resolution_scale != clamped;
                    self.pad.touch.settings.resolution_scale_percent = clamped;
                    self.settings.horizon.resolution_scale = clamped;
                    self.settings.touch = self.pad.touch.settings.clone();
                    if let Some(r) = self.renderer.as_mut() {
                        r.set_world_post(
                            clamped as f32 / 100.0,
                            self.settings.horizon.fxaa,
                            self.settings.horizon.taa,
                            self.settings.horizon.upscaling.ratio(),
                        );
                        r.invalidate_menu_blur();
                    }
                    if changed {
                        let _ = self.pad.touch.settings.save();
                        self.settings.save(&Self::options_path());
                        sync_launcher_render_scale(clamped);
                    }
                }
                InGameMenuAction::ToggleGameAnimations => {
                    let next = !self.pad.touch.settings.disable_game_animations;
                    self.pad.touch.settings.disable_game_animations = next;
                    horizon_client::ui_theme::motion::set_game_animations_disabled(next);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                    if let Some(r) = self.renderer.as_mut() {
                        r.animated_textures = self.settings.horizon.animated_textures && !next;
                        r.invalidate_menu_blur();
                    }
                }
                InGameMenuAction::SendCustomInput(token) => {
                    self.android_sync_pointer();
                    self.android_custom_token_tap(&token);
                }
                InGameMenuAction::SetRecorderResolution(index) => {
                    let index = usize::from(index).min(horizon_client::recorder::RESOLUTIONS.len() - 1);
                    horizon_client::recorder::update_settings(|s| s.resolution = index);
                }
                InGameMenuAction::SetRecorderFps(index) => {
                    let index = usize::from(index).min(horizon_client::recorder::FRAME_RATES.len() - 1);
                    horizon_client::recorder::update_settings(|s| s.fps = index);
                }
                InGameMenuAction::SetRecorderAudio(index) => {
                    let index = usize::from(index).min(horizon_client::recorder::AUDIO.len() - 1);
                    horizon_client::recorder::update_settings(|s| s.audio = index);
                    let status = horizon_client::recorder::status();
                    if index == 2 && status.capability.checked && !status.capability.game_audio {
                        // The choice is kept, but say plainly that it cannot run here.
                        let mut status = status;
                        status.phase = horizon_client::recorder::Phase::Unsupported;
                        status.message =
                            "in-game audio needs Android 10 or newer; use microphone or none"
                                .to_owned();
                        horizon_client::recorder::publish(status);
                    }
                }
                InGameMenuAction::ToggleRecorderOverlay => {
                    horizon_client::recorder::update_settings(|s| {
                        s.record_controls = !s.record_controls
                    });
                }
                InGameMenuAction::SetOverlayScale(scale) => {
                    self.pad.touch.settings.overlay_scale = scale.clamp(0.5, 2.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::SetOverlayOpacity(opacity) => {
                    self.pad.touch.settings.overlay_opacity = opacity.clamp(0.20, 1.0);
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleOverlayHidden => {
                    let next = !self.pad.touch.settings.overlay_hidden;
                    self.pad.touch.settings.overlay_hidden = next;
                    if next {
                        self.pad.touch_ui.floating_menu = false;
                    }
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::ToggleOverlayShowControlsInMenus => {
                    self.pad.touch.settings.overlay_show_controls_in_menus =
                        !self.pad.touch.settings.overlay_show_controls_in_menus;
                    let _ = self.pad.touch.settings.save();
                    self.settings.touch = self.pad.touch.settings.clone();
                }
                InGameMenuAction::RecorderStart => crate::screen_record::start(),
                InGameMenuAction::RecorderStop => crate::screen_record::stop(),
                InGameMenuAction::RecorderPlay(uri) => {
                    let note = crate::screen_record::play(&uri);
                    let mut status = horizon_client::recorder::status();
                    if !note.is_empty() {
                        status.message = note;
                    }
                    horizon_client::recorder::publish(status);
                }
                InGameMenuAction::Replay(act) => {
                    self.handle_replay_action(act)?;
                }
                InGameMenuAction::MusicTogglePause => {
                    horizon_client::internet_music::toggle_pause();
                    self.update_discord_presence();
                }
                InGameMenuAction::MusicNext => {
                    horizon_client::internet_music::next_track();
                    self.update_discord_presence();
                }
                InGameMenuAction::MusicPrev => {
                    horizon_client::internet_music::prev_track();
                    self.update_discord_presence();
                }
                InGameMenuAction::MusicSeekBy(delta_ms) => {
                    horizon_client::internet_music::seek_by_ms(delta_ms);
                }
                InGameMenuAction::MusicSeekFrac(frac) => {
                    let snap = horizon_client::internet_music::snapshot();
                    if snap.duration_ms > 0 {
                        let target = ((snap.duration_ms as f32) * frac.clamp(0.0, 1.0)) as u64;
                        horizon_client::internet_music::seek_to_ms(target);
                    }
                }
                InGameMenuAction::MusicSetVolume(vol) => {
                    horizon_client::internet_music::set_volume(vol);
                }
                InGameMenuAction::MusicToggleMute => {
                    horizon_client::internet_music::toggle_mute();
                }
                InGameMenuAction::MusicToggleVanilla => {
                    let off = horizon_client::internet_music::toggle_vanilla_music_disabled();
                    if off {
                        self.music.stop_playing(None, &mut self.audio);
                    }
                }
                InGameMenuAction::MusicSelectPlaylist(_) => {}
                InGameMenuAction::MusicPlayPlaylist(pi) => {
                    let pls = horizon_client::internet_music::playlists();
                    if let Some(pl) = pls.get(pi) {
                        horizon_client::internet_music::play_playlist(&pl.name, 0);
                        self.update_discord_presence();
                    }
                }
                InGameMenuAction::MusicPlayPlaylistTrack(pi, ti) => {
                    let pls = horizon_client::internet_music::playlists();
                    if let Some(pl) = pls.get(pi) {
                        horizon_client::internet_music::play_playlist(&pl.name, ti);
                        self.update_discord_presence();
                    }
                }
                InGameMenuAction::MusicSetSearchTab(search_tab) => {
                    if search_tab {
                        let s = horizon_client::internet_music::in_game_search_snapshot();
                        if !s.fetched_initial && !s.searching {
                            horizon_client::internet_music::trigger_in_game_search(
                                s.query,
                                s.genre,
                                s.sort,
                                0,
                            );
                        }
                    } else {
                        horizon_client::internet_music::invalidate_offline_cache();
                    }
                }
                InGameMenuAction::MusicOpenSearchKeyboard => {
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    crate::platform::show(&s.query);
                }
                InGameMenuAction::MusicTriggerSearch => {
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    crate::platform::hide();
                    horizon_client::internet_music::trigger_in_game_search(
                        s.query,
                        s.genre,
                        s.sort,
                        0,
                    );
                }
                InGameMenuAction::MusicSelectGenre(genre) => {
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    let next_genre = match genre {
                        None => None,
                        Some(g) => {
                            if s.genre.as_deref().is_some_and(|cur| cur.eq_ignore_ascii_case(g)) {
                                None
                            } else {
                                Some(g.to_owned())
                            }
                        }
                    };
                    horizon_client::internet_music::trigger_in_game_search(
                        s.query,
                        next_genre,
                        s.sort,
                        0,
                    );
                }
                InGameMenuAction::MusicCycleSort => {
                    use horizon_client::internet_music::MusicSort;
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    let idx = MusicSort::ALL
                        .iter()
                        .position(|&x| x == s.sort)
                        .unwrap_or(0);
                    let next_sort = MusicSort::ALL[(idx + 1) % MusicSort::ALL.len()];
                    horizon_client::internet_music::trigger_in_game_search(
                        s.query,
                        s.genre,
                        next_sort,
                        0,
                    );
                }
                InGameMenuAction::MusicPrevPage => {
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    if s.page > 0 {
                        horizon_client::internet_music::trigger_in_game_search(
                            s.query,
                            s.genre,
                            s.sort,
                            s.page - 1,
                        );
                    }
                }
                InGameMenuAction::MusicNextPage => {
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    if s.has_next {
                        horizon_client::internet_music::trigger_in_game_search(
                            s.query,
                            s.genre,
                            s.sort,
                            s.page + 1,
                        );
                    }
                }
                InGameMenuAction::MusicPlaySearchResult(idx) => {
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    if idx < s.results.len() {
                        horizon_client::internet_music::play_queue(&s.results, idx, "");
                        self.update_discord_presence();
                    }
                }
                InGameMenuAction::MusicDownloadSearchResult(idx) => {
                    let s = horizon_client::internet_music::in_game_search_snapshot();
                    if let Some(track) = s.results.get(idx).cloned()
                        && !horizon_client::internet_music::is_track_downloaded(&track)
                    {
                        horizon_client::internet_music::start_download_track(track);
                    }
                }
                InGameMenuAction::UpdateVoltExtras(volt) => {
                    let was_timelapse = self.settings.extras.volt.cinematic_timelapse.enabled;
                    self.settings.extras.volt = volt;
                    self.volt_sync_timelapse_from_settings(was_timelapse);
                    self.settings.extras.save(std::path::Path::new("."));
                    self.apply_ui_settings();
                    self.sync_volt_touch_ui();
                }
                InGameMenuAction::VoltQuickAction(act) => {
                    self.volt_extras_quick_action(act);
                    self.settings.extras.save(std::path::Path::new("."));
                }
                InGameMenuAction::FocusVoltExtrasSearch => {
                    self.pad.touch_ui.volt_extras_search_focused = true;
                    crate::platform::show(&self.pad.touch_ui.volt_extras_search);
                }
            }
            return Ok(true);
        }
        if self.replay.is_some()
            && self.screen == Screen::Playing
            && let Some(maybe_act) =
                self.pad
                    .touch_ui
                    .replay_touch(&mut b, &self.gui.atlas, t.id, t.phase, p)
        {
            if t.phase == TouchPhase::Started {
                self.pad.touch.ignore(t.id);
            }
            if let Some(act) = maybe_act {
                if t.phase == TouchPhase::Started {
                    self.audio.play_button_click();
                    self.android_touch_haptic(10);
                }
                self.handle_replay_action(act)?;
            }
            return Ok(true);
        }
        if t.phase == TouchPhase::Moved {
            return Ok(false);
        }
        if t.phase == TouchPhase::Started
            && !self.pad.touch_ui.owns_relative(t.id)
            && !self.pad.touch_ui.owns_gesture(t.id)
            && let Some(action) = self
                .pad
                .touch_ui
                .action(b.width, b.height, p, self.pad_in_menu())
        {
            {
                self.pad.touch.clear();
                self.pad.touch.ignore(t.id);
                match action {
                    Action::LatchF3 => {
                        let was = self.pad.touch_ui.key_held(KeyCode::F3);
                        self.pad.touch_ui.f3_latched = !self.pad.touch_ui.f3_latched;
                        let now = self.pad.touch_ui.key_held(KeyCode::F3);
                        if !was && now {
                            self.physical_keys.insert(KeyCode::F3);
                            self.debug_key(KeyCode::F3, KeyAction::Press);
                        }
                        if was && !now {
                            self.android_key_up(KeyCode::F3);
                        }
                    }
                    Action::Pick => {
                        self.play_mouse(MouseButton::Middle, true)?;
                        self.play_mouse(MouseButton::Middle, false)?;
                        self.pad.touch_ui.expanded = false;
                    }
                    Action::DebugCombo(key) => {
                        let held = self.pad.touch_ui.key_held(KeyCode::F3);
                        if !held {
                            self.debug_key(KeyCode::F3, KeyAction::Press);
                        }
                        self.debug_key(key, KeyAction::Press);
                        self.debug_key(key, KeyAction::Release);
                        if !held {
                            self.debug_key(KeyCode::F3, KeyAction::Release);
                        }
                        self.pad.touch_ui.expanded = false;
                    }
                    Action::MouseButton(right) => {
                        if self.pad.touch_ui.press_mouse(t.id, right) {
                            self.android_menu_mouse(events, id, right, true)?;
                        }
                    }
                    Action::VirtualMouse => {
                        if matches!(self.screen, Screen::Inventory | Screen::Creative) {
                            // Cancel a prior drag without clicking/dropping at
                            // the toggle button, which is outside the inventory.
                            self.inv_clicks = Default::default();
                            self.inv_drag = None;
                            self.creative_screen.release();
                            self.loom_release();
                        }
                        for right in self.pad.touch_ui.release_all_mouse() {
                            self.android_menu_mouse(events, id, right, false)?;
                        }
                        self.pad.touch_ui.cancel();
                        let next = !self.pad.touch.settings.virtual_mouse_enabled;
                        self.pad.touch.settings.virtual_mouse_enabled = next;
                        self.pad.touch_ui.pointer_off = !next;
                        let _ = self.pad.touch.settings.save();
                        self.settings.touch = self.pad.touch.settings.clone();
                        self.android_sync_pointer();
                    }
                    Action::More => self.pad.touch_ui.expanded = !self.pad.touch_ui.expanded,
                    Action::Keyboard => {
                        if crate::platform::active() {
                            crate::platform::hide();
                        } else {
                            self.android_show_keyboard();
                        }
                    }
                    Action::Key(key) => {
                        if key != KeyCode::F3 && !self.pad.touch_ui.key_held(KeyCode::F3) {
                            self.pad.touch_ui.expanded = false;
                        }
                        if key == KeyCode::Escape {
                            crate::platform::hide();
                            self.pad_back()?;
                        } else if matches!(
                            key,
                            KeyCode::Tab | KeyCode::ShiftLeft | KeyCode::ControlLeft
                        ) {
                            let down = if key == KeyCode::Tab {
                                if self.physical_keys.remove(&key) {
                                    false
                                } else {
                                    self.physical_keys.insert(key);
                                    true
                                }
                            } else {
                                self.pad.touch_ui.toggle_modifier(key)
                            };
                            if !self.pad_in_menu() {
                                self.mapped_key(key, down, false)?;
                            }
                        } else {
                            if !self.pad.touch_ui.key_down(t.id, key) {
                                return Ok(true);
                            }
                            self.physical_keys.insert(key);
                            if !self.debug_key(key, KeyAction::Press) {
                                self.key_pressed(key)?;
                            }
                            if matches!(key, KeyCode::KeyT | KeyCode::Slash)
                                && self.screen == Screen::Chat
                            {
                                self.android_show_keyboard();
                            }
                        }
                    }
                }
            }
            return Ok(true);
        }
        let _ = (events, id);
        Ok(false)
    }
    pub(super) fn android_show_keyboard(&mut self) {
        // The in-world keyboard button needs a real editor to receive text.
        if !self.pad_in_menu() {
            self.open_chat("");
        }
        let initial = if self.dialog_open() {
            self.dialog_edit_value()
        } else if self.screen == Screen::BlockEditor {
            let Some(value) = self.block_editor_edit_value() else {
                return;
            };
            value
        } else if self.screen == Screen::CommandBlock {
            self.command_block_edit_value()
        } else if self.sign_edit.is_some() {
            self.sign_input(Some(KeyCode::End), None);
            self.sign_current().unwrap_or_default()
        } else if self.screen == Screen::Chat && self.title.is_none() {
            self.chat_key_code(KeyCode::End, None);
            self.ui
                .chat_screen
                .as_ref()
                .map(|s| s.value().to_owned())
                .unwrap_or_default()
        } else if let Some(t) = self.title.as_mut() {
            t.field_key(KeyCode::End, false, false);
            t.focused_field()
                .map(|(_, s, _)| s.to_owned())
                .unwrap_or_default()
        } else if self.screen == Screen::Creative {
            self.creative_screen.search.move_cursor_to_end(false);
            self.creative_screen.search.value.clone()
        } else if self.recipe_search_focused() {
            self.recipe_search_value()
        } else if self.screen == Screen::Options {
            self.ui.options.android_edit_value(&self.gui.atlas)
        } else if self.screen == Screen::WorldOptions {
            self.world_options_edit_value()
        } else if self.mod_browser_typing() {
            self.mod_browser_edit_value()
        } else if self.station_typing() {
            self.station
                .as_mut()
                .map(|s| {
                    s.rename.move_cursor_to_end(false);
                    s.rename.value.clone()
                })
                .unwrap_or_default()
        } else if self.book.is_some() {
            self.book_edit_value().unwrap_or_default()
        } else {
            String::new()
        };
        if matches!(
            self.book,
            Some(super::books::BookScreen::Edit { signing: false, .. })
        ) {
            crate::platform::show_multiline(&initial);
        } else {
            crate::platform::show(&initial);
        }
    }
    /// Text from the Android keyboard while a preset naming field has focus.
    /// Both naming dialogs (the editor's save-as and the hub's copy) type here
    /// instead of showing a keyboard inside the dialog.
    fn android_preset_typing(&mut self, out: &Output) -> bool {
        let handled = horizon_client::touch_ui::apply_editor_or_preset_output(
            self.pad.touch.editor.as_mut(),
            self.pad.touch_ui.preset_dialog.as_mut(),
            out,
        );
        if handled && horizon_client::touch_ui::output_dismisses(out) {
            crate::platform::hide();
        }
        handled
    }

    pub(super) fn android_poll_keyboard(&mut self) {
        self.android_custom_events();
        if let Some((up, last)) = self.pad.held_scroll
            && last.elapsed() >= std::time::Duration::from_millis(120)
        {
            self.pad.held_scroll = Some((up, Instant::now()));
            self.android_step_scroll(up);
        }
        self.android_sync_pointer();
        if let Some(ed) = self.pad.touch.editor.as_mut()
            && std::mem::take(&mut ed.keyboard_request)
        {
            let text = if ed.picker_step != horizon_client::touch::PickerStep::None || ed.key_picker
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
        if let Some(notice) = crate::documents::poll() {
            if notice.changed {
                self.refresh_pack_lists();
                self.settings.shader_packs =
                    horizon_client::shader_pack_list::scan(&Self::shader_packs_dir());
            }
            self.system_message(horizon_client::text::Text::literal(notice.text));
        }
        for result in crate::gallery::poll() {
            self.system_message(horizon_client::text::Text::literal(
                result.unwrap_or_else(|e| format!("File export failed: {e}")),
            ));
        }
        for out in crate::platform::poll() {
            if self.pad.touch.editor.is_some() {
                let captured = match &out {
                    Output::Text(t) => self.pad.touch.capture_custom_text(t),
                    Output::Key(code) => self.pad.touch.capture_custom_key_code(*code),
                    Output::Backspace => self.pad.touch.capture_custom_key_code(KeyCode::Backspace),
                    Output::Enter => self.pad.touch.capture_custom_key_code(KeyCode::Enter),
                    Output::Close => false,
                };
                if captured {
                    self.settings.touch = self.pad.touch.settings.clone();
                    crate::platform::hide();
                    continue;
                }
            }
            if self.pad.touch_ui.volt_key_picker.is_some() {
                let captured = match &out {
                    Output::Text(t) => self.pad.touch_ui.capture_volt_key_picker_text(t),
                    Output::Key(code) => self.pad.touch_ui.capture_volt_key_picker_key(*code),
                    Output::Backspace => self
                        .pad
                        .touch_ui
                        .capture_volt_key_picker_key(KeyCode::Backspace),
                    Output::Enter => self.pad.touch_ui.capture_volt_key_picker_key(KeyCode::Enter),
                    Output::Close => None,
                };
                if let Some(volt) = captured {
                    crate::platform::hide();
                    self.android_sync_pointer();
                    self.settings.extras.volt = volt;
                    self.settings.extras.save(std::path::Path::new("."));
                    self.apply_ui_settings();
                    self.sync_volt_touch_ui();
                    continue;
                }
            }
            if self.pad.touch_ui.custom_input_picker != horizon_client::touch::PickerStep::None {
                let captured = match &out {
                    Output::Text(t) => self.pad.touch_ui.capture_custom_input_text(t),
                    Output::Key(code) => self.pad.touch_ui.capture_custom_input_key(*code),
                    Output::Backspace => self
                        .pad
                        .touch_ui
                        .capture_custom_input_key(KeyCode::Backspace),
                    Output::Enter => self.pad.touch_ui.capture_custom_input_key(KeyCode::Enter),
                    Output::Close => None,
                };
                if let Some(token) = captured {
                    crate::platform::hide();
                    self.android_sync_pointer();
                    self.android_custom_token_tap(&token);
                    continue;
                }
            }
            if horizon_client::touch_ui::apply_editor_button_label_output(&mut self.pad.touch, &out) {
                self.settings.touch = self.pad.touch.settings.clone();
                if horizon_client::touch_ui::output_dismisses(&out) {
                    crate::platform::hide();
                }
                continue;
            }
            if self.android_preset_typing(&out) {
                continue;
            }
            if self.pad.touch_ui.floating_menu && self.pad.touch_ui.volt_extras_search_focused {
                match &out {
                    Output::Text(t) => {
                        for ch in t.chars() {
                            if ch == '\n' || ch == '\r' {
                                self.pad.touch_ui.volt_extras_search_focused = false;
                                crate::platform::hide();
                                break;
                            } else if !ch.is_control() {
                                self.pad.touch_ui.volt_extras_search.push(ch);
                                self.pad.touch_ui.hub_scroll = 0.0;
                            }
                        }
                    }
                    Output::Backspace => {
                        self.pad.touch_ui.volt_extras_search.pop();
                        self.pad.touch_ui.hub_scroll = 0.0;
                    }
                    Output::Enter | Output::Close => {
                        self.pad.touch_ui.volt_extras_search_focused = false;
                        crate::platform::hide();
                    }
                    Output::Key(code) => {
                        use winit::keyboard::KeyCode;
                        match code {
                            KeyCode::Backspace => {
                                self.pad.touch_ui.volt_extras_search.pop();
                                self.pad.touch_ui.hub_scroll = 0.0;
                            }
                            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape => {
                                self.pad.touch_ui.volt_extras_search_focused = false;
                                crate::platform::hide();
                            }
                            _ => {
                                if let Some(ch) = horizon_client::touch_ui::key_code_to_char(*code) {
                                    self.pad.touch_ui.volt_extras_search.push(ch);
                                    self.pad.touch_ui.hub_scroll = 0.0;
                                }
                            }
                        }
                    }
                }
                continue;
            }
            if self.volt_jei_visible() && self.extras.volt.jei_search_focused {
                match &out {
                    Output::Text(t) => {
                        for ch in t.chars() {
                            if ch == '\n' || ch == '\r' {
                                self.extras.volt.jei_search_focused = false;
                                crate::platform::hide();
                                break;
                            } else if !ch.is_control() {
                                self.extras.volt.jei_search.push(ch);
                                self.extras.volt.jei_page = 0;
                            }
                        }
                    }
                    Output::Backspace => {
                        self.extras.volt.jei_search.pop();
                        self.extras.volt.jei_page = 0;
                    }
                    Output::Enter | Output::Close => {
                        self.extras.volt.jei_search_focused = false;
                        crate::platform::hide();
                    }
                    Output::Key(code) => {
                        use winit::keyboard::KeyCode;
                        match code {
                            KeyCode::Backspace => {
                                self.extras.volt.jei_search.pop();
                                self.extras.volt.jei_page = 0;
                            }
                            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape => {
                                self.extras.volt.jei_search_focused = false;
                                crate::platform::hide();
                            }
                            _ => {
                                if let Some(ch) = horizon_client::touch_ui::key_code_to_char(*code) {
                                    self.extras.volt.jei_search.push(ch);
                                    self.extras.volt.jei_page = 0;
                                }
                            }
                        }
                    }
                }
                continue;
            }
            if self.pad.touch_ui.floating_menu && self.pad.touch_ui.music_editing_search {
                match &out {
                    Output::Text(t) => {
                        let mut s = horizon_client::internet_music::in_game_search_snapshot();
                        for ch in t.chars() {
                            if ch == '\n' || ch == '\r' {
                                self.pad.touch_ui.music_editing_search = false;
                                crate::platform::hide();
                                horizon_client::internet_music::set_in_game_query(s.query.clone());
                                horizon_client::internet_music::trigger_in_game_search(
                                    s.query.clone(), s.genre, s.sort, 0,
                                );
                                break;
                            } else if !ch.is_control() {
                                s.query.push(ch);
                            }
                        }
                        if self.pad.touch_ui.music_editing_search {
                            horizon_client::internet_music::set_in_game_query(s.query);
                        }
                    }
                    Output::Backspace => {
                        let mut s = horizon_client::internet_music::in_game_search_snapshot();
                        s.query.pop();
                        horizon_client::internet_music::set_in_game_query(s.query);
                    }
                    Output::Enter => {
                        self.pad.touch_ui.music_editing_search = false;
                        crate::platform::hide();
                        let s = horizon_client::internet_music::in_game_search_snapshot();
                        horizon_client::internet_music::trigger_in_game_search(
                            s.query, s.genre, s.sort, 0,
                        );
                    }
                    Output::Close => {
                        self.pad.touch_ui.music_editing_search = false;
                        crate::platform::hide();
                    }
                    Output::Key(code) => {
                        use winit::keyboard::KeyCode;
                        let mut s = horizon_client::internet_music::in_game_search_snapshot();
                        match code {
                            KeyCode::Backspace => {
                                s.query.pop();
                                horizon_client::internet_music::set_in_game_query(s.query);
                            }
                            KeyCode::Enter | KeyCode::NumpadEnter => {
                                self.pad.touch_ui.music_editing_search = false;
                                crate::platform::hide();
                                horizon_client::internet_music::trigger_in_game_search(
                                    s.query, s.genre, s.sort, 0,
                                );
                            }
                            KeyCode::Escape => {
                                self.pad.touch_ui.music_editing_search = false;
                                crate::platform::hide();
                            }
                            _ => {
                                if let Some(ch) = horizon_client::touch_ui::key_code_to_char(*code) {
                                    s.query.push(ch);
                                    horizon_client::internet_music::set_in_game_query(s.query);
                                }
                            }
                        }
                    }
                }
                continue;
            }
            let next_sign_line = self.sign_edit.is_some() && matches!(out, Output::Enter);
            let done =
                matches!(out, Output::Close) || (matches!(out, Output::Enter) && !next_sign_line);
            self.pad_osk_output(out);
            if done {
                crate::platform::hide();
            } else if next_sign_line {
                self.android_show_keyboard();
            }
        }
        if !self.pad.touch_ui.floating_menu {
            self.pad.touch_ui.music_editing_search = false;
            self.pad.touch_ui.volt_extras_search_focused = false;
        }
        // Title screens also use Playing: their focused text fields must keep
        // the IME until the user confirms or closes the editor.
        if !self.pad_in_menu()
            && self.pad.touch.editor.is_none()
            && self.pad.touch_ui.custom_input_picker == horizon_client::touch::PickerStep::None
            && self.pad.touch_ui.volt_key_picker.is_none()
            && !self.pad.touch_ui.music_editing_search
            && !self.pad.touch_ui.volt_extras_search_focused
            && !(self.volt_jei_visible() && self.extras.volt.jei_search_focused)
            && crate::platform::active()
        {
            crate::platform::hide();
        }
    }
    pub(super) fn android_performance_context(&self) -> serde_json::Value {
        let renderer = self.renderer.as_ref().map(|r| {
            let [arena, vertices] = r.section_vertex_bytes();
            let memory = r.device.generate_allocator_report();
            serde_json::json!({"gpu":r.info.name,"backend":format!("{:?}",r.info.backend),
                "surface":[r.config.width,r.config.height],"world_size":r.world_render_size(),
                "present":format!("{:?}",r.config.present_mode),
                "wait_ms":r.last_wait.as_secs_f32()*1000.0,
                "gpu_profiling_requested":crate::device_runtime::preferences().gpu_profile,
                "gpu_encoder_timestamps_supported":r.device.features().contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
                "terrain_pass_draws": r.terrain_pass_draws,
                "gpu_stage_distributions":r.gpu_stage_distributions().into_iter().map(|(stage,p50,p95,p99,n)|serde_json::json!({"stage":stage,"p50_ms":p50,"p95_ms":p95,"p99_ms":p99,"samples":n})).collect::<Vec<_>>(),
                "gpu_stages":r.gpu_stage_times().into_iter().map(|(stage, median_ms, max_ms, samples)|
                    serde_json::json!({"stage":stage,"median_ms":median_ms,"max_ms":max_ms,"samples":samples})).collect::<Vec<_>>(),
                "draw_calls":r.section_draw_calls,"drawn_sections":r.drawn_sections,
                "resident_sections":r.resident_sections(),"uploaded_bytes_total":r.uploaded_bytes,
                "arena_bytes":arena,"section_vertex_bytes":vertices,
                "gpu_allocated_bytes":memory.as_ref().map(|m|m.total_allocated_bytes),
                "gpu_reserved_bytes":memory.as_ref().map(|m|m.total_reserved_bytes)})
        });
        serde_json::json!({"renderer":renderer,"android_native":crate::device_runtime::diagnostics(),"android_power":crate::platform::performance_power(),
            "menu_blur_radius":self.menu_blur(),"shader_pack_active":!self.settings.horizon.shader_pack.is_empty(),
            "render_distance":self.settings.render_distance,
            "simulation_distance":self.settings.simulation_distance,"fps_limit":self.framerate_limit(),
            "focused":self.focused,"paused":self.pause,"in_world":self.player.is_some(),
            "playing_screen":self.screen == Screen::Playing,
            "idle_seconds":self.last_input.elapsed().as_secs(),
            "camera_angles":[self.yaw, self.pitch],
            "player_velocity":self.player.as_ref().map(|p|[p.velocity.x,p.velocity.y,p.velocity.z]),
            "underwater":self.ambience.was_underwater(),"mobs":self.mobs.len(),
            "particles":self.particles.len(),"chunks":self.world.chunks.len(),
            "dirty_light":self.world.dirty_light.len(),"pending_uploads":self.pending_uploads.len(),
            "pending_mesh_bytes":self.pending_uploads.iter().map(|(_,m)|(m.solid.len()+m.translucent.len()) as u64*std::mem::size_of::<horizon_client::mesher::TerrainVertex>() as u64).sum::<u64>(),
            "remote_backlog":self.remote_backlog(),
            "streaming":self.streaming.stages().into_iter().collect::<std::collections::BTreeMap<_,_>>()})
    }
    pub(super) fn android_key_up(&mut self, key: KeyCode) {
        if self.pad.custom_keys.contains(&key) {
            return;
        }
        self.physical_keys.remove(&key);
        self.keys.remove(&key);
        self.debug_key(key, KeyAction::Release);
    }
    pub(super) fn android_release_touch_keys(&mut self, cancelled: bool) {
        self.pad.touch_ui.clear_modifiers();
        if cancelled && matches!(self.screen, Screen::Inventory | Screen::Creative) {
            self.inv_clicks = Default::default();
            self.inv_drag = None;
            self.creative_screen.release();
            self.loom_release();
        }
        self.pad.touch_ui.cancel_relative();
        for right in self.pad.touch_ui.release_all_mouse() {
            let button = if right {
                MouseButton::Right
            } else {
                MouseButton::Left
            };
            if let Err(error) = self.android_custom_mouse(button, false) {
                log::warn!("Release virtual mouse: {error}");
            }
        }

        if cancelled {
            self.pad.hardware_keys.clear();
            crate::platform::cancel_vibration();
            self.pad.touch.clear();
            self.pad.touch_aim = None;
        }
        self.android_custom_events();
        for key in self.pad.touch_ui.release_keys() {
            if cancelled && key == KeyCode::F3 {
                self.physical_keys.remove(&key);
                self.debug_keys.modifier_down = false;
                self.debug_keys.used_debug_key_as_modifier = false;
                if self.screen == Screen::GameModeSwitcher {
                    self.close_gamemode_switcher(false);
                }
            } else {
                self.android_key_up(key);
            }
        }
    }
    pub(super) fn android_menu_touch(
        &mut self,
        t: &Touch,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
    ) {
        if self.native_builds_touch(t) {
            return;
        }
        self.android_sync_pointer();
        if self.pad.touch_ui.virtual_mouse {
            self.android_virtual_touch(t, events, id);
            return;
        }
        let Some(b) = self.gui_scale() else {
            return;
        };
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        if t.phase == TouchPhase::Started && self.screen == Screen::Options {
            self.ui.options.touch_start();
        }
        let direct = self.pad.touch_ui.direct_pointer(t.id, p, t.phase);
        self.handle_window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id: t.device_id,
                position: t.location,
            },
        );
        if direct {
            if t.phase != TouchPhase::Moved {
                self.handle_window_event(
                    events,
                    id,
                    WindowEvent::MouseInput {
                        device_id: t.device_id,
                        button: MouseButton::Left,
                        state: if t.phase == TouchPhase::Started {
                            ElementState::Pressed
                        } else {
                            ElementState::Released
                        },
                    },
                );
            }
            return;
        }
        match self
            .pad
            .touch_ui
            .gesture(t.id, [t.location.x as f32, t.location.y as f32], t.phase)
        {
            Gesture::Tap => {
                let field = crate::platform::field_at(p);
                for state in [ElementState::Pressed, ElementState::Released] {
                    self.handle_window_event(
                        events,
                        id,
                        WindowEvent::MouseInput {
                            device_id: t.device_id,
                            button: MouseButton::Left,
                            state,
                        },
                    );
                }
                if field || self.sign_edit.is_some() {
                    self.android_show_keyboard();
                }
            }
            Gesture::Scroll(y) if self.screen == Screen::Options => {
                self.ui
                    .options
                    .touch_scroll(&self.settings, (b.width, b.height), (p[0], p[1]), y);
            }
            Gesture::Scroll(y) => self.handle_window_event(
                events,
                id,
                WindowEvent::MouseWheel {
                    device_id: t.device_id,
                    delta: MouseScrollDelta::LineDelta(0.0, y),
                    phase: TouchPhase::Moved,
                },
            ),
            Gesture::None => {}
        }
    }
    pub(super) fn android_virtual_touch(
        &mut self,
        t: &Touch,
        events: &dyn super::event_loop::EventLoopCtl,
        id: WindowId,
    ) {
        let Some(b) = self.gui_scale() else {
            return;
        };
        let position = self.pad.touch_ui.relative_touch(
            t.id,
            [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale],
            t.phase,
            [self.cursor.0, self.cursor.1],
            [b.width, b.height],
            self.pad.touch.settings.virtual_mouse_sensitivity,
            !self.pad.touch_ui.mouse_held(),
        );
        self.cursor = (position[0], position[1]);
        self.handle_window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id: t.device_id,
                position: winit::dpi::PhysicalPosition::new(
                    f64::from(position[0] * b.scale),
                    f64::from(position[1] * b.scale),
                ),
            },
        );
        if t.phase == TouchPhase::Ended {
            self.android_virtual_mouse_clicks(events);
        }
    }
    pub(super) fn android_virtual_mouse_clicks(
        &mut self,
        events: &dyn super::event_loop::EventLoopCtl,
    ) {
        if !self.focused || !self.pad.touch_ui.virtual_mouse {
            self.pad.touch_ui.cancel_relative();
            for right in self.pad.touch_ui.release_all_mouse() {
                if let Some(id) = self.window.as_ref().map(|w| w.id())
                    && let Err(error) = self.android_menu_mouse(events, id, right, false)
                {
                    log::warn!("Release virtual mouse: {error}");
                }
            }
            return;
        }
        if !self.pad_in_menu() && !self.volt_jei_visible() {
            return;
        }
        let Some(right) = self.pad.touch_ui.relative_click(true) else {
            return;
        };
        let button = if right {
            MouseButton::Right
        } else {
            MouseButton::Left
        };
        if matches!(self.screen, Screen::Inventory | Screen::Creative) || self.volt_jei_visible() {
            if let Err(e) = self
                .android_custom_mouse(button, true)
                .and_then(|()| self.android_custom_mouse(button, false))
            {
                log::warn!("Virtual mouse click: {e}");
            }
        } else if let Some(id) = self.window.as_ref().map(|w| w.id()) {
            for state in [ElementState::Pressed, ElementState::Released] {
                self.handle_window_event(
                    events,
                    id,
                    WindowEvent::MouseInput {
                        device_id: DeviceId::dummy(),
                        state,
                        button,
                    },
                );
            }
        }
    }

    #[cfg(target_os = "android")]
    fn android_updater_modal_rects(w: f32, h: f32) -> ([f32; 4], [f32; 4], [f32; 4]) {
        let mw = 390.0_f32.min(w - 32.0);
        let mh = 230.0_f32.min(h - 32.0);
        let mx = (w - mw) * 0.5;
        let my = (h - mh) * 0.5;
        let btn_y = my + mh - 50.0;
        let btn_w = (mw - 52.0) * 0.5;
        let later_r = [mx + 20.0, btn_y, btn_w, 34.0];
        let action_r = [mx + 32.0 + btn_w, btn_y, btn_w, 34.0];
        ([mx, my, mw, mh], later_r, action_r)
    }

    #[cfg(target_os = "android")]
    fn android_updater_modal_active() -> Option<crate::updater::UpdateStatus> {
        let status = crate::updater::get_status();
        match &status {
            crate::updater::UpdateStatus::Available(info)
                if info.download_url.is_some() && crate::updater::should_auto_prompt() =>
            {
                Some(status)
            }
            crate::updater::UpdateStatus::Downloading { .. }
            | crate::updater::UpdateStatus::Downloaded { .. } => Some(status),
            crate::updater::UpdateStatus::Error(_)
                if (crate::updater::last_release_info().is_some()
                    || crate::updater::last_downloaded_path().is_some())
                    && crate::updater::should_auto_prompt() =>
            {
                Some(status)
            }
            _ => None,
        }
    }

    #[cfg(target_os = "android")]
    pub(super) fn android_draw_updater_modal(&self, hud: &mut HudBatch) {
        use horizon_client::ui_theme::{Face, Icon, Ui, active_theme};
        static IN_GAME_CHECK: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        if !self.pad_in_menu() || self.pad.touch_ui.floating_menu {
            return;
        }
        if let Ok(home) = std::env::current_dir() {
            crate::updater::init_home(&home);
        }
        if !IN_GAME_CHECK.swap(true, std::sync::atomic::Ordering::Relaxed)
            && crate::updater::get_status() == crate::updater::UpdateStatus::Idle
        {
            crate::updater::check_in_background();
        }
        let Some(status) = Self::android_updater_modal_active() else {
            return;
        };
        crate::updater::mark_auto_prompt_shown();
        let mut ui = Ui::mobile(&self.gui.atlas, hud, (0.0, 0.0));
        let theme = active_theme();
        let (modal_r, later_r, action_r) = Self::android_updater_modal_rects(ui.w, ui.h);
        let [mx, my, mw, mh] = modal_r;

        ui.fill(0.0, 0.0, ui.w, ui.h, 0x9900_0000);
        ui.rounded(mx, my, mw, mh, 20.0, theme.bg_solid);
        ui.stroke_rounded(mx, my, mw, mh, 20.0, 1.5, theme.accent);

        ui.rounded(mx + 20.0, my + 18.0, 38.0, 38.0, 12.0, theme.accent_soft);
        ui.icon(Icon::HorizonLogo, mx + 29.0, my + 27.0, 20.0, theme.accent);
        ui.text(
            Face::SemiBold,
            16.5,
            mx + 68.0,
            my + 19.0,
            theme.text_primary,
            "Volt Plus Update",
        );

        let info = crate::updater::last_release_info();
        let ver_badge = if let Some(ref rel) = info {
            format!("v{}  ->  v{}", crate::backend::VERSION, rel.version)
        } else if let crate::updater::UpdateStatus::Downloaded { ref version, .. } = status {
            format!("v{}  ->  v{}", crate::backend::VERSION, version)
        } else {
            format!("v{}", crate::backend::VERSION)
        };
        ui.text(Face::Medium, 11.5, mx + 68.0, my + 40.0, theme.accent_text, &ver_badge);

        match &status {
            crate::updater::UpdateStatus::Available(rel) => {
                let note = rel
                    .changelog
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("New release available on GitHub.");
                let fitted = ui.fit_text(Face::Regular, 11.5, note, mw - 44.0);
                ui.text(Face::Regular, 11.5, mx + 22.0, my + 74.0, theme.text_muted, &fitted);
                ui.text(
                    Face::Regular,
                    10.5,
                    mx + 22.0,
                    my + 98.0,
                    theme.text_muted,
                    &ui.fit_text(Face::Regular, 10.5, &rel.asset_name, mw - 44.0),
                );
            }
            crate::updater::UpdateStatus::Downloading {
                progress,
                downloaded,
                total,
            } => {
                let pct = (progress * 100.0).clamp(0.0, 100.0);
                let label = if *total > 0 {
                    format!(
                        "Downloading APK... {pct:.0}% ({:.1} / {:.1} MB)",
                        *downloaded as f32 / 1_048_576.0,
                        *total as f32 / 1_048_576.0
                    )
                } else {
                    format!("Downloading APK... {pct:.0}%")
                };
                ui.text(Face::Medium, 12.0, mx + 22.0, my + 82.0, theme.text_primary, &label);
                ui.rounded(mx + 22.0, my + 112.0, mw - 44.0, 10.0, 5.0, theme.surface);
                ui.rounded(
                    mx + 22.0,
                    my + 112.0,
                    ((mw - 44.0) * progress.clamp(0.04, 1.0)).max(10.0),
                    10.0,
                    5.0,
                    theme.accent,
                );
            }
            crate::updater::UpdateStatus::Downloaded { version, .. } => {
                ui.text(
                    Face::Medium,
                    12.0,
                    mx + 22.0,
                    my + 86.0,
                    theme.text_primary,
                    &format!("Volt Plus v{version} is ready to install."),
                );
            }
            crate::updater::UpdateStatus::Error(err) => {
                let fitted = ui.fit_text(Face::Regular, 11.0, err, mw - 44.0);
                ui.text(Face::Regular, 11.0, mx + 22.0, my + 86.0, theme.text_muted, &fitted);
            }
            _ => {}
        }

        ui.rounded(later_r[0], later_r[1], later_r[2], later_r[3], 14.0, theme.surface);
        ui.border(later_r[0], later_r[1], later_r[2], later_r[3], 14.0, theme.border);
        ui.centered_fit(Face::Medium, 12.5, later_r, theme.text_primary, "Later");

        let action_label = match &status {
            crate::updater::UpdateStatus::Available(_) => "Update Now",
            crate::updater::UpdateStatus::Downloading { .. } => "Downloading...",
            crate::updater::UpdateStatus::Downloaded { .. } => "Install APK",
            crate::updater::UpdateStatus::Error(_) => "Retry",
            _ => "Update",
        };
        ui.rounded(
            action_r[0],
            action_r[1],
            action_r[2],
            action_r[3],
            14.0,
            theme.accent,
        );
        ui.centered_fit(Face::SemiBold, 12.5, action_r, theme.on_accent, action_label);
        ui.finish();
    }

    #[cfg(target_os = "android")]
    pub(super) fn android_updater_modal_touch(
        &mut self,
        phase: TouchPhase,
        phys_pos: [f32; 2],
        width_px: u32,
        height_px: u32,
    ) -> bool {
        let Some(status) = Self::android_updater_modal_active() else {
            return false;
        };
        let mut dummy = HudBatch::new([width_px, height_px]);
        let ui = horizon_client::ui_theme::Ui::mobile(&self.gui.atlas, &mut dummy, (0.0, 0.0));
        let p = [phys_pos[0] / ui.density, phys_pos[1] / ui.density];
        let (_modal_r, later_r, action_r) = Self::android_updater_modal_rects(ui.w, ui.h);
        ui.finish();
        if phase == TouchPhase::Started {
            let hit = |r: [f32; 4]| {
                p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
            };
            if hit(later_r) {
                crate::updater::dismiss();
                self.audio.play_button_click();
                self.android_touch_haptic(10);
            } else if hit(action_r) {
                self.audio.play_button_click();
                self.android_touch_haptic(12);
                match status {
                    crate::updater::UpdateStatus::Available(info) => {
                        let home = std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("."));
                        crate::updater::download_and_install_in_background(info, home);
                    }
                    crate::updater::UpdateStatus::Downloaded { path, .. } => {
                        crate::updater::trigger_install(&path);
                    }
                    crate::updater::UpdateStatus::Error(_) => {
                        if let Some(apk) = crate::updater::last_downloaded_path() {
                            crate::updater::trigger_install(&apk);
                        } else if let Some(info) = crate::updater::last_release_info() {
                            let home = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                            crate::updater::download_and_install_in_background(info, home);
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }
}

pub(super) fn sync_launcher_render_scale(pct: u32) {
    let clamped = pct.clamp(25, 100);
    let Ok(mut dir) = std::env::current_dir() else {
        return;
    };
    for _ in 0..4 {
        let launcher_cfg = dir.join("android-launcher.json");
        if launcher_cfg.is_file() {
            if let Ok(bytes) = std::fs::read(&launcher_cfg)
                && let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(&bytes)
                && let Some(obj) = json.as_object_mut()
            {
                obj.insert(
                    "render_scale".to_owned(),
                    serde_json::Value::Number(serde_json::Number::from(clamped)),
                );
                if let Ok(pretty) = serde_json::to_vec_pretty(&json) {
                    let partial = dir.join("android-launcher.partial");
                    if std::fs::write(&partial, &pretty).is_ok() {
                        let _ = std::fs::rename(&partial, &launcher_cfg);
                    }
                }
            }
            return;
        }
        if !dir.pop() {
            break;
        }
    }
}

