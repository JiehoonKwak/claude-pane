mod filter;
mod render;
mod star;
mod state;
#[cfg(test)]
mod tests;

use state::{Activity, Config, FocusPayload, HookPayload, NotificationFlash, State};

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
        // Store our own pane_id for self-filtering
        let ids = get_plugin_ids();
        self.own_pane_id = Some(ids.plugin_id);

        set_timeout(1.0);
        eprintln!("pane-palette: loaded (v{})", env!("CARGO_PKG_VERSION"));
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
                    eprintln!("pane-palette: permissions granted");
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
            filter::refresh_running_commands(self);
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
            "pane-palette:event" | "claude-pane:event" | "event" => {
                if let Some(payload) = &msg.payload {
                    match serde_json::from_str::<HookPayload>(payload) {
                        Ok(hook) => self.handle_hook_event(hook),
                        Err(e) => {
                            eprintln!("pane-palette: malformed payload: {e}");
                            false
                        }
                    }
                } else {
                    false
                }
            }
            "show" | "pane-palette:show" => {
                self.show_palette();
                true
            }
            "hide" | "pane-palette:hide" => {
                self.hide_palette();
                true
            }
            "star-next" | "pane-palette:star-next" => {
                self.focus_starred_pane(true);
                false
            }
            "star-prev" | "pane-palette:star-prev" => {
                self.focus_starred_pane(false);
                false
            }
            "focus" | "pane-palette:focus" => {
                let pipe_name = msg.name.clone();
                if let Some(payload) = &msg.payload {
                    match serde_json::from_str::<FocusPayload>(payload) {
                        Ok(fp) => self.handle_focus(fp),
                        Err(e) => eprintln!("pane-palette: focus bad payload: {e}"),
                    }
                }
                unblock_cli_pipe_input(&pipe_name);
                false
            }
            "dump-state" | "pane-palette:dump-state" => {
                let mut out = String::new();
                out.push_str(&format!("own_pane_id={:?}\n", self.own_pane_id));
                for t in &self.tabs {
                    out.push_str(&format!("tab: pos={} name={:?} active={}\n", t.position, t.name, t.active));
                }
                for (&tab_idx, panes) in &self.pane_manifest {
                    for p in panes {
                        out.push_str(&format!("manifest: tab_key={} pane_id={} plugin={} title={:?}\n",
                            tab_idx, p.id, p.is_plugin, p.title));
                    }
                }
                for (id, s) in &self.sessions {
                    out.push_str(&format!("session: pane={} activity={:?} tab_idx={:?} tab_name={:?} project={:?}\n",
                        id, s.activity, s.tab_index, s.tab_name, s.project_name));
                }
                let home = std::env::var("HOME").unwrap_or_default();
                let path = format!("{home}/.config/zellij/plugins/pane-palette-dump.txt");
                let _ = std::fs::write(&path, &out);
                false
            }
            "test" | "pane-palette:test" => {
                eprintln!("pane-palette: test ping OK");
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
            .or_insert_with(|| state::SessionInfo::new(activity, now));

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
        // Blink for flash_duration_ms, then steady tint until UserPromptSubmit (persist)
        if activity.is_attention()
            && self.config.notification_flash != NotificationFlash::Off
        {
            let blink_s = self.config.flash_duration_ms as f64 / 1000.0;
            session.flash_deadline = now + blink_s;
            // Only persist tint if pane is NOT currently focused
            if self.config.notification_flash == NotificationFlash::Persist
                && self.current_focus_pane != Some(hook.pane_id)
            {
                session.focus_highlight_deadline = f64::MAX;
            }
        }

        // Clear all visual indicators on UserPromptSubmit
        if hook.hook_event == "UserPromptSubmit" {
            if let Some(s) = self.sessions.get_mut(&hook.pane_id) {
                s.flash_deadline = 0.0;
                s.focus_highlight_deadline = 0.0;
                set_pane_color(PaneId::Terminal(hook.pane_id), None, None);
            }
        }

        self.rebuild_pane_map();

        self.visible
    }

    fn handle_timer(&mut self) -> bool {
        let now = self.uptime_s;
        let mut any_flash_active = false;
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
                // BLINK phase
                any_flash_active = true;
                if tick_even {
                    set_pane_color(
                        PaneId::Terminal(pane_id),
                        None,
                        Some("#273548".into()),
                    );
                } else {
                    set_pane_color(PaneId::Terminal(pane_id), None, None);
                }
            } else if session.flash_deadline > 0.0 {
                // Flash just expired — reset blink residue
                session.flash_deadline = 0.0;
                if session.focus_highlight_deadline <= now {
                    // No steady tint follows — fully clear
                    session.focus_highlight_deadline = 0.0;
                    set_pane_color(PaneId::Terminal(pane_id), None, None);
                }
                // else: steady tint will be applied by the next branch on next tick
            } else if session.focus_highlight_deadline > now {
                // Steady tint (persist mode)
                set_pane_color(
                    PaneId::Terminal(pane_id),
                    None,
                    Some("#1a2535".into()),
                );
            } else if session.focus_highlight_deadline > 0.0 {
                // Steady tint just expired — reset
                session.focus_highlight_deadline = 0.0;
                set_pane_color(PaneId::Terminal(pane_id), None, None);
            } else if session.activity == Activity::Done
                && (now - session.last_event_ts) > done_timeout
            {
                session.activity = Activity::Idle;
                // Reset background
                set_pane_color(PaneId::Terminal(pane_id), None, None);
            } else if session.activity == Activity::Idle
                && (now - session.last_event_ts) > idle_remove
            {
                to_remove.push(pane_id);
            } else if session.activity.is_running()
                && !session.activity.is_attention()
                && (now - session.last_event_ts) > idle_remove
            {
                // Stale running session (no hook events for idle_remove_s)
                // Conservative: only fires after 300s (default), well beyond
                // any normal tool invocation gap.
                session.activity = Activity::Done;
                session.last_event_ts = now;
            }
        }

        for id in to_remove {
            self.sessions.remove(&id);
        }

        // Focus highlights: applied once on selection, only reset on expiry
        let expired: Vec<u32> = self
            .focus_highlights
            .iter()
            .filter(|(_, &deadline)| deadline <= now)
            .map(|(&id, _)| id)
            .collect();
        for pane_id in expired {
            self.focus_highlights.remove(&pane_id);
            // Skip reset if notification flash is still active
            let has_flash = self
                .sessions
                .get(&pane_id)
                .map(|s| s.flash_deadline > now)
                .unwrap_or(false);
            if !has_flash {
                set_pane_color(PaneId::Terminal(pane_id), None, None);
            }
        }

        let has_highlights = !self.focus_highlights.is_empty();
        let interval = if any_flash_active || has_highlights {
            0.1
        } else {
            1.0
        };
        set_timeout(interval);

        self.visible
    }

    fn track_focus(&mut self) {
        for panes in self.pane_manifest.values() {
            for pane in panes {
                if pane.is_focused && !pane.is_plugin {
                    let new_focus = pane.id;
                    if self.current_focus_pane != Some(new_focus) {
                        self.previous_focus_pane = self.current_focus_pane;
                        self.current_focus_pane = Some(new_focus);
                        // Clear notification when user focuses the pane
                        if let Some(session) = self.sessions.get_mut(&new_focus) {
                            if session.flash_deadline > 0.0
                                || session.focus_highlight_deadline > 0.0
                            {
                                session.flash_deadline = 0.0;
                                session.focus_highlight_deadline = 0.0;
                                set_pane_color(
                                    PaneId::Terminal(new_focus),
                                    None,
                                    None,
                                );
                            }
                            // Clear attention activity on focus (acknowledged)
                            if session.activity.is_attention() {
                                session.activity = Activity::Thinking;
                            }
                        }
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
        filter::refresh_running_commands(self);
        self.refresh_filtered();
        show_self(true);
    }

    fn hide_palette(&mut self) {
        self.visible = false;
        self.search_query.clear();
        hide_self();
    }

    fn focus_starred_pane(&mut self, forward: bool) {
        // Close palette if open
        if self.visible {
            self.hide_palette();
        }

        // Build starred panes in display order (tab_index → pane_id), deduplicated
        let mut seen = std::collections::HashSet::new();
        let mut starred_ordered: Vec<(usize, u32)> = Vec::new();
        for (&tab_idx, panes) in &self.pane_manifest {
            for pane in panes {
                if !pane.is_plugin && self.stars.contains(pane.id) && seen.insert(pane.id)
                {
                    starred_ordered.push((tab_idx, pane.id));
                }
            }
        }
        starred_ordered.sort_by_key(|&(tab, id)| (tab, id));

        if starred_ordered.is_empty() {
            return;
        }

        // Use last_starred_pane for consistent cycling (not current_focus_pane,
        // which may not have updated yet from the previous focus call)
        let cur_pos = self
            .last_starred_pane
            .and_then(|c| starred_ordered.iter().position(|&(_, id)| id == c));
        let next_pos = match cur_pos {
            Some(pos) => {
                if forward {
                    (pos + 1) % starred_ordered.len()
                } else if pos == 0 {
                    starred_ordered.len() - 1
                } else {
                    pos - 1
                }
            }
            None => 0,
        };

        let (tab_idx, pane_id) = starred_ordered[next_pos];
        self.last_starred_pane = Some(pane_id);

        let current_tab = self.tabs.iter().find(|t| t.active).map(|t| t.position);
        if current_tab != Some(tab_idx) {
            switch_tab_to(tab_idx as u32);
        }
        focus_terminal_pane(pane_id, false, false);

        // Background flash
        self.focus_highlights
            .insert(pane_id, self.uptime_s + self.config.focus_highlight_s);
        set_pane_color(
            PaneId::Terminal(pane_id),
            None,
            Some("#1a2535".into()),
        );
    }

    fn confirm_selection(&mut self) {
        if let Some(entry) = self.filtered_entries.get(self.selected_index) {
            let pane_id = entry.pane_id;
            let tab_idx = entry.tab_index;

            self.focus_highlights
                .insert(pane_id, self.uptime_s + self.config.focus_highlight_s);
            set_pane_color(
                PaneId::Terminal(pane_id),
                None,
                Some("#1a2535".into()),
            );

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
    }

    fn handle_focus(&mut self, fp: FocusPayload) {
        for (&tab_idx, panes) in &self.pane_manifest {
            for pane in panes {
                if pane.id == fp.pane_id && !pane.is_plugin {
                    // Switch tab if needed
                    let current_tab = self.tabs.iter().find(|t| t.active).map(|t| t.position);
                    if current_tab != Some(tab_idx) {
                        switch_tab_to(tab_idx as u32);
                    }
                    // Direct focus — O(1)
                    focus_terminal_pane(fp.pane_id, false, false);
                    // Flash effect via existing focus_highlights system
                    let duration = fp.flash_duration_ms.unwrap_or(800) as f64 / 1000.0;
                    self.focus_highlights
                        .insert(fp.pane_id, self.uptime_s + duration);
                    let color = fp.flash_color.as_deref().unwrap_or("#273548");
                    set_pane_color(PaneId::Terminal(fp.pane_id), None, Some(color.into()));
                    return;
                }
            }
        }
        eprintln!("pane-palette: focus pane {} not found in manifest", fp.pane_id);
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
            self.confirm_selection();
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
            if key.has_no_modifiers() {
                // h/l fold/unfold (only in grouped view, not during search)
                if self.search_query.is_empty() && (ch == 'h' || ch == 'l') {
                    if let Some(entry) = self.filtered_entries.get(self.selected_index)
                    {
                        let tab = entry.tab_index;
                        if ch == 'h' {
                            self.collapsed_tabs.insert(tab);
                        } else {
                            self.collapsed_tabs.remove(&tab);
                        }
                        self.refresh_filtered();
                    }
                    return true;
                }

                // Number selection: 1-9 jump to Nth visible entry and confirm
                // jump_targets is set by render() to match displayed numbers
                if ('1'..='9').contains(&ch) {
                    let target = (ch as usize) - ('1' as usize);
                    if let Some(&idx) = self.jump_targets.get(target) {
                        self.selected_index = idx;
                        self.confirm_selection();
                    }
                    return true;
                }

                if ch != ' ' {
                    self.search_query.push(ch);
                    self.refresh_filtered();
                    return true;
                }
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

