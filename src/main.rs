mod render;
mod star;
mod state;
#[cfg(test)]
mod tests;

use state::{Activity, Config, HookPayload, NotificationFlash, State};

// ---------------------------------------------------------------------------
// WASM-only: plugin registration + ZellijPlugin trait + host function calls
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod plugin {
    use super::*;
    use std::collections::BTreeMap;
    use zellij_tile::prelude::*;

    register_plugin!(State);

    impl ZellijPlugin for State {
        fn load(&mut self, configuration: BTreeMap<String, String>) {
            self.config = Config::from_map(&configuration);
            request_permission(&[
                PermissionType::ReadApplicationState,
                PermissionType::ChangeApplicationState,
                PermissionType::MessageAndLaunchOtherPlugins,
                PermissionType::RunCommands,
            ]);
            subscribe(&[
                EventType::PaneUpdate,
                EventType::TabUpdate,
                EventType::Timer,
                EventType::Key,
                EventType::Mouse,
                EventType::RunCommandResult,
                EventType::PermissionRequestResult,
            ]);
            set_timeout(1.0);
            eprintln!("claude-pane: loaded (v{})", env!("CARGO_PKG_VERSION"));
        }

        fn update(&mut self, event: Event) -> bool {
            match event {
                Event::PaneUpdate(manifest) => {
                    self.pane_manifest = manifest.panes;
                    self.track_focus();
                    self.rebuild_pane_map();
                    self.prune_dead_sessions();
                    if self.visible {
                        self.refresh_filtered();
                    }
                    self.visible
                }
                Event::TabUpdate(tabs) => {
                    self.tabs = tabs;
                    self.rebuild_pane_map();
                    if self.visible {
                        self.refresh_filtered();
                    }
                    self.visible
                }
                Event::Timer(elapsed) => {
                    self.uptime_s += elapsed;
                    self.tick_count += 1;
                    handle_timer(self)
                }
                Event::Key(key) if self.visible => handle_key(self, key),
                Event::Mouse(mouse) if self.visible => handle_mouse(self, mouse),
                Event::PermissionRequestResult(status) => {
                    if status == PermissionStatus::Granted {
                        self.permissions_granted = true;
                        eprintln!("claude-pane: permissions granted");
                    }
                    false
                }
                _ => false,
            }
        }

        fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
            handle_pipe(self, pipe_message)
        }

        fn render(&mut self, rows: usize, cols: usize) {
            render::render(self, rows, cols);
        }
    }

    // -----------------------------------------------------------------------
    // Pipe handler
    // -----------------------------------------------------------------------

    fn handle_pipe(state: &mut State, msg: PipeMessage) -> bool {
        match msg.name.as_str() {
            "claude-pane:event" | "event" => {
                if let Some(payload) = &msg.payload {
                    match serde_json::from_str::<HookPayload>(payload) {
                        Ok(hook) => handle_hook_event(state, hook),
                        Err(e) => {
                            eprintln!("claude-pane: malformed payload: {e}");
                            false
                        }
                    }
                } else {
                    false
                }
            }
            "show" | "claude-pane:show" => {
                show_palette(state);
                true
            }
            "hide" | "claude-pane:hide" => {
                hide_palette(state);
                true
            }
            "dump-state" | "claude-pane:dump-state" => {
                eprintln!(
                    "claude-pane: sessions={:?}",
                    state.sessions.keys().collect::<Vec<_>>()
                );
                for (id, s) in &state.sessions {
                    eprintln!(
                        "  pane={} activity={:?} project={:?} tab={:?}",
                        id, s.activity, s.project_name, s.tab_name
                    );
                }
                false
            }
            "test" | "claude-pane:test" => {
                eprintln!("claude-pane: test ping OK");
                false
            }
            _ => false,
        }
    }

    fn handle_hook_event(state: &mut State, hook: HookPayload) -> bool {
        let activity =
            Activity::from_hook_event(&hook.hook_event, hook.tool_name.as_deref());
        let now = state.uptime_s;

        let session = state
            .sessions
            .entry(hook.pane_id)
            .or_insert_with(|| state::SessionInfo::new(hook.pane_id, activity, now));

        let prev_activity = session.activity;
        session.activity = activity;
        session.last_event_ts = now;

        if let Some(ref name) = hook.project_name {
            if !name.is_empty() {
                session.project_name = Some(name.clone());
            }
        }
        if let Some(ref name) = hook.tool_name {
            session.tool_name = Some(name.clone());
        }

        // Flash on permission/notification
        if activity.is_attention()
            && state.config.notification_flash != NotificationFlash::Off
        {
            let duration_s = if state.config.notification_flash == NotificationFlash::Brief
            {
                state.config.flash_duration_ms as f64 / 1000.0
            } else {
                f64::MAX
            };
            session.flash_deadline = now + duration_s;
        }

        // Clear flash on UserPromptSubmit
        if hook.hook_event == "UserPromptSubmit" {
            if let Some(s) = state.sessions.get_mut(&hook.pane_id) {
                s.flash_deadline = 0.0;
                highlight_and_unhighlight_panes(
                    vec![],
                    vec![PaneId::Terminal(hook.pane_id)],
                );
            }
        }

        state.rebuild_pane_map();

        if activity != prev_activity {
            update_zjstatus(state);
        }

        state.visible
    }

    // -----------------------------------------------------------------------
    // Timer / animation
    // -----------------------------------------------------------------------

    fn handle_timer(state: &mut State) -> bool {
        let now = state.uptime_s;
        let mut any_flash_active = false;
        let mut need_zjstatus_update = false;
        let mut to_remove: Vec<u32> = Vec::new();

        let done_timeout = state.config.done_timeout_s;
        let idle_remove = state.config.idle_remove_s;
        let tick_even = state.tick_count.is_multiple_of(2);

        let ids: Vec<u32> = state.sessions.keys().copied().collect();

        for pane_id in ids {
            let session = match state.sessions.get_mut(&pane_id) {
                Some(s) => s,
                None => continue,
            };

            if session.flash_deadline > now {
                any_flash_active = true;
                if tick_even {
                    highlight_and_unhighlight_panes(
                        vec![PaneId::Terminal(pane_id)],
                        vec![],
                    );
                } else {
                    highlight_and_unhighlight_panes(
                        vec![],
                        vec![PaneId::Terminal(pane_id)],
                    );
                }
            } else if session.focus_highlight_deadline > now {
                highlight_and_unhighlight_panes(
                    vec![PaneId::Terminal(pane_id)],
                    vec![],
                );
            } else if session.activity == Activity::Done
                && (now - session.last_event_ts) > done_timeout
            {
                session.activity = Activity::Idle;
                need_zjstatus_update = true;
                highlight_and_unhighlight_panes(
                    vec![],
                    vec![PaneId::Terminal(pane_id)],
                );
            } else if session.activity == Activity::Idle
                && (now - session.last_event_ts) > idle_remove
            {
                to_remove.push(pane_id);
                need_zjstatus_update = true;
            }
        }

        for id in to_remove {
            state.sessions.remove(&id);
        }

        if need_zjstatus_update {
            update_zjstatus(state);
        }

        let interval = if any_flash_active { 0.5 } else { 1.0 };
        set_timeout(interval);

        state.visible
    }

    // -----------------------------------------------------------------------
    // zjstatus integration
    // -----------------------------------------------------------------------

    fn update_zjstatus(state: &mut State) {
        if !state.config.zjstatus_pipe {
            return;
        }

        let now = state.uptime_s;
        if (now - state.last_zjstatus_update) < 0.25 {
            return;
        }
        state.last_zjstatus_update = now;

        let formatted = state.format_zjstatus();
        let pipe_payload = format!("zjstatus::pipe::pipe_status::{}", formatted);

        pipe_message_to_plugin(
            MessageToPlugin::new("pipe_status")
                .with_plugin_url(&state.config.zjstatus_url)
                .with_payload(pipe_payload),
        );
    }

    // -----------------------------------------------------------------------
    // Focus tracking
    // -----------------------------------------------------------------------

    impl State {
        fn track_focus(&mut self) {
            for panes in self.pane_manifest.values() {
                for pane in panes {
                    if pane.is_focused && !pane.is_plugin {
                        let new_focus = pane.id;
                        if self.current_focus_pane != Some(new_focus) {
                            self.previous_focus_pane = self.current_focus_pane;
                            self.current_focus_pane = Some(new_focus);
                        }
                        return;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Palette show/hide
    // -----------------------------------------------------------------------

    fn show_palette(state: &mut State) {
        state.visible = true;
        state.search_query.clear();
        state.selected_index = 0;
        state.refresh_filtered();
        show_self(true);
    }

    fn hide_palette(state: &mut State) {
        state.visible = false;
        state.search_query.clear();
        hide_self();
    }

    // -----------------------------------------------------------------------
    // Key handling (palette mode)
    // -----------------------------------------------------------------------

    fn handle_key(state: &mut State, key: KeyWithModifier) -> bool {
        if Config::key_matches(&key, &state.config.key_cancel) {
            hide_palette(state);
            return true;
        }

        if Config::key_matches(&key, &state.config.key_select_down) {
            if !state.filtered_entries.is_empty() {
                state.selected_index =
                    (state.selected_index + 1) % state.filtered_entries.len();
            }
            return true;
        }

        if Config::key_matches(&key, &state.config.key_select_up) {
            if !state.filtered_entries.is_empty() {
                if state.selected_index == 0 {
                    state.selected_index = state.filtered_entries.len() - 1;
                } else {
                    state.selected_index -= 1;
                }
            }
            return true;
        }

        if Config::key_matches(&key, &state.config.key_confirm) {
            if let Some(entry) = state.filtered_entries.get(state.selected_index) {
                let pane_id = entry.pane_id;
                let tab_idx = entry.tab_index;

                if let Some(session) = state.sessions.get_mut(&pane_id) {
                    session.focus_highlight_deadline = state.uptime_s + 2.0;
                }

                hide_palette(state);

                let current_tab = state
                    .tabs
                    .iter()
                    .find(|t| t.active)
                    .map(|t| t.position);

                if current_tab != Some(tab_idx) {
                    switch_tab_to(tab_idx as u32);
                }

                focus_terminal_pane(pane_id, false);
            }
            return true;
        }

        if Config::key_matches(&key, &state.config.key_toggle_star) {
            if let Some(entry) = state.filtered_entries.get(state.selected_index) {
                state.stars.toggle(entry.pane_id);
                state.refresh_filtered();
            }
            return true;
        }

        if Config::key_matches(&key, &state.config.key_delete_char) {
            state.search_query.pop();
            state.refresh_filtered();
            return true;
        }

        if let BareKey::Char(ch) = key.bare_key {
            if key.has_no_modifiers() && ch != ' ' {
                state.search_query.push(ch);
                state.refresh_filtered();
                return true;
            }
        }

        false
    }

    fn handle_mouse(state: &mut State, mouse: Mouse) -> bool {
        match mouse {
            Mouse::ScrollUp(_) => {
                if state.selected_index > 0 {
                    state.selected_index -= 1;
                }
                true
            }
            Mouse::ScrollDown(_) => {
                if !state.filtered_entries.is_empty() {
                    state.selected_index =
                        (state.selected_index + 1).min(state.filtered_entries.len() - 1);
                }
                true
            }
            Mouse::LeftClick(line, _col) => {
                let idx = line.saturating_sub(3);
                if (idx as usize) < state.filtered_entries.len() {
                    state.selected_index = idx as usize;
                }
                true
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzzy filter (pure logic, no WASM deps)
// ---------------------------------------------------------------------------

pub fn fuzzy_filter(query: &str, entries: Vec<state::PaneEntry>) -> Vec<state::PaneEntry> {
    use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
    use nucleo_matcher::{Config as NucleoConfig, Matcher, Utf32Str};

    let mut matcher = Matcher::new(NucleoConfig::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut scored: Vec<(u32, state::PaneEntry)> = entries
        .into_iter()
        .filter_map(|entry| {
            let haystack_str = format!(
                "{} {} {} {}",
                entry.tab_name,
                entry.title,
                entry
                    .session
                    .as_ref()
                    .and_then(|s| s.project_name.as_deref())
                    .unwrap_or(""),
                entry.pane_id,
            );

            let mut buf = Vec::new();
            let haystack = Utf32Str::new(&haystack_str, &mut buf);
            pattern
                .score(haystack, &mut matcher)
                .map(|score| (score, entry))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, e)| e).collect()
}

// refresh_filtered needs to be accessible from both plugin module and state
impl State {
    pub fn refresh_filtered(&mut self) {
        let entries = self.build_entries();

        if self.search_query.is_empty() {
            self.filtered_entries = entries;
        } else {
            self.filtered_entries = fuzzy_filter(&self.search_query, entries);
        }

        if !self.filtered_entries.is_empty() {
            self.selected_index = self.selected_index.min(self.filtered_entries.len() - 1);
        } else {
            self.selected_index = 0;
        }
    }
}
