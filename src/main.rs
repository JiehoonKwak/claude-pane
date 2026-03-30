mod render;
mod star;
mod state;
#[cfg(test)]
mod tests;

use state::{Activity, Config, HookPayload, NotificationFlash, State};

#[cfg(target_arch = "wasm32")]
use zellij_tile::prelude::*;

// register_plugin! generates main() — must be at crate root
#[cfg(target_arch = "wasm32")]
register_plugin!(State);

// Dummy main for host-target compilation (tests, clippy)
#[cfg(not(target_arch = "wasm32"))]
fn main() {}

// ---------------------------------------------------------------------------
// ZellijPlugin trait (WASM only)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
impl ZellijPlugin for State {
    fn load(&mut self, configuration: std::collections::BTreeMap<String, String>) {
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
                self.handle_timer()
            }
            Event::Key(key) if self.visible => self.handle_key(key),
            Event::Mouse(mouse) if self.visible => self.handle_mouse(mouse),
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
        self.handle_pipe(pipe_message)
    }

    fn render(&mut self, rows: usize, cols: usize) {
        // render() is only called when plugin pane is visible
        if !self.visible {
            self.visible = true;
            self.search_query.clear();
            self.selected_index = 0;
            self.refresh_filtered();
        }
        render::render(self, rows, cols);
    }
}

// ---------------------------------------------------------------------------
// Methods that call WASM host functions (gated)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
impl State {
    fn handle_pipe(&mut self, msg: PipeMessage) -> bool {
        match msg.name.as_str() {
            "claude-pane:event" | "event" => {
                if let Some(payload) = &msg.payload {
                    match serde_json::from_str::<HookPayload>(payload) {
                        Ok(hook) => self.handle_hook_event(hook),
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
                self.show_palette();
                true
            }
            "hide" | "claude-pane:hide" => {
                self.hide_palette();
                true
            }
            "dump-state" | "claude-pane:dump-state" => {
                eprintln!(
                    "claude-pane: sessions={:?}",
                    self.sessions.keys().collect::<Vec<_>>()
                );
                for (id, s) in &self.sessions {
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

    fn handle_hook_event(&mut self, hook: HookPayload) -> bool {
        let activity =
            Activity::from_hook_event(&hook.hook_event, hook.tool_name.as_deref());
        let now = self.uptime_s;

        let session = self
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
            && self.config.notification_flash != NotificationFlash::Off
        {
            let duration_s = if self.config.notification_flash == NotificationFlash::Brief
            {
                self.config.flash_duration_ms as f64 / 1000.0
            } else {
                f64::MAX
            };
            session.flash_deadline = now + duration_s;
        }

        // Clear flash on UserPromptSubmit
        if hook.hook_event == "UserPromptSubmit" {
            if let Some(s) = self.sessions.get_mut(&hook.pane_id) {
                s.flash_deadline = 0.0;
                highlight_and_unhighlight_panes(
                    vec![],
                    vec![PaneId::Terminal(hook.pane_id)],
                );
            }
        }

        self.rebuild_pane_map();

        if activity != prev_activity {
            self.update_zjstatus();
        }

        self.visible
    }

    fn handle_timer(&mut self) -> bool {
        let now = self.uptime_s;
        let mut any_flash_active = false;
        let mut need_zjstatus_update = false;
        let mut to_remove: Vec<u32> = Vec::new();

        let done_timeout = self.config.done_timeout_s;
        let idle_remove = self.config.idle_remove_s;
        let tick_even = self.tick_count.is_multiple_of(2);

        let ids: Vec<u32> = self.sessions.keys().copied().collect();

        for pane_id in ids {
            let session = match self.sessions.get_mut(&pane_id) {
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
            self.sessions.remove(&id);
        }

        if need_zjstatus_update {
            self.update_zjstatus();
        }

        let interval = if any_flash_active { 0.5 } else { 1.0 };
        set_timeout(interval);

        self.visible
    }

    fn update_zjstatus(&mut self) {
        if !self.config.zjstatus_pipe {
            return;
        }

        let now = self.uptime_s;
        if (now - self.last_zjstatus_update) < 0.25 {
            return;
        }
        self.last_zjstatus_update = now;

        let formatted = self.format_zjstatus();
        let pipe_name = format!("zjstatus::pipe::pipe_status::{}", formatted);

        // Broadcast without URL — targets existing zjstatus instance
        // Using with_plugin_url() would create a new zjstatus without config
        pipe_message_to_plugin(MessageToPlugin::new(&pipe_name));
    }

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

    fn show_palette(&mut self) {
        self.visible = true;
        self.search_query.clear();
        self.selected_index = 0;
        self.refresh_filtered();
        show_self(true);
    }

    fn hide_palette(&mut self) {
        self.visible = false;
        self.search_query.clear();
        hide_self();
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        if Config::key_matches(&key, &self.config.key_cancel) {
            self.hide_palette();
            return true;
        }

        if Config::key_matches(&key, &self.config.key_select_down) {
            if !self.filtered_entries.is_empty() {
                self.selected_index =
                    (self.selected_index + 1) % self.filtered_entries.len();
            }
            return true;
        }

        if Config::key_matches(&key, &self.config.key_select_up) {
            if !self.filtered_entries.is_empty() {
                if self.selected_index == 0 {
                    self.selected_index = self.filtered_entries.len() - 1;
                } else {
                    self.selected_index -= 1;
                }
            }
            return true;
        }

        if Config::key_matches(&key, &self.config.key_confirm) {
            if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                let pane_id = entry.pane_id;
                let tab_idx = entry.tab_index;

                if let Some(session) = self.sessions.get_mut(&pane_id) {
                    session.focus_highlight_deadline = self.uptime_s + 2.0;
                }

                self.hide_palette();

                let current_tab = self
                    .tabs
                    .iter()
                    .find(|t| t.active)
                    .map(|t| t.position);

                if current_tab != Some(tab_idx) {
                    switch_tab_to(tab_idx as u32);
                }

                focus_terminal_pane(pane_id, false, false);
            }
            return true;
        }

        if Config::key_matches(&key, &self.config.key_toggle_star) {
            if let Some(entry) = self.filtered_entries.get(self.selected_index) {
                self.stars.toggle(entry.pane_id);
                self.refresh_filtered();
            }
            return true;
        }

        if Config::key_matches(&key, &self.config.key_delete_char) {
            self.search_query.pop();
            self.refresh_filtered();
            return true;
        }

        if let BareKey::Char(ch) = key.bare_key {
            if key.has_no_modifiers() && ch != ' ' {
                self.search_query.push(ch);
                self.refresh_filtered();
                return true;
            }
        }

        false
    }

    fn handle_mouse(&mut self, mouse: Mouse) -> bool {
        match mouse {
            Mouse::ScrollUp(_) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                true
            }
            Mouse::ScrollDown(_) => {
                if !self.filtered_entries.is_empty() {
                    self.selected_index =
                        (self.selected_index + 1).min(self.filtered_entries.len() - 1);
                }
                true
            }
            Mouse::LeftClick(line, _col) => {
                let idx = line.saturating_sub(3);
                if (idx as usize) < self.filtered_entries.len() {
                    self.selected_index = idx as usize;
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
