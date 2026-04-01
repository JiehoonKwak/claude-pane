use crate::state::{PaneEntry, State};

pub fn fuzzy_filter(query: &str, entries: Vec<PaneEntry>) -> Vec<PaneEntry> {
    use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
    use nucleo_matcher::{Config as NucleoConfig, Matcher, Utf32Str};

    let mut matcher = Matcher::new(NucleoConfig::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut scored: Vec<(u32, PaneEntry)> = entries
        .into_iter()
        .filter_map(|entry| {
            let haystack_str = format!(
                "{} {} {} {} {}",
                entry.tab_name,
                entry.title,
                entry
                    .session
                    .as_ref()
                    .and_then(|s| s.project_name.as_deref())
                    .unwrap_or(""),
                entry.running_command.as_deref().unwrap_or(""),
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

/// Query OS for running commands and cache in State.
/// Called once on palette open, not on every keystroke.
/// Also marks stale sessions as Done when process is no longer "claude".
#[cfg(target_arch = "wasm32")]
pub fn refresh_running_commands(state: &mut State) {
    use zellij_tile::prelude::*;
    state.running_command_cache.clear();
    for panes in state.pane_manifest.values() {
        for pane in panes {
            if pane.is_plugin {
                continue;
            }
            if let Ok(cmd) = get_pane_running_command(PaneId::Terminal(pane.id)) {
                if let Some(bin) = cmd.first() {
                    let basename = bin.rsplit('/').next().unwrap_or(bin);
                    state
                        .running_command_cache
                        .insert(pane.id, basename.to_string());
                }
            }
        }
    }

    // Mark sessions as Done when stale by time AND foreground is a shell.
    // Only shell (zsh/bash/fish/sh) indicates Claude has exited — any other
    // process (git, pytest, node, etc.) could be a running tool invocation.
    let stale_threshold = state.config.done_timeout_s * 2.0;
    let now = state.uptime_s;
    let shells: &[&str] = &["zsh", "bash", "fish", "sh"];
    let stale: Vec<u32> = state
        .sessions
        .iter()
        .filter(|(&id, session)| {
            session.activity.is_running()
                && !session.activity.is_attention()
                && (now - session.last_event_ts) > stale_threshold
                && state
                    .running_command_cache
                    .get(&id)
                    .map(|cmd| shells.iter().any(|&s| cmd == s))
                    .unwrap_or(false)
        })
        .map(|(&id, _)| id)
        .collect();

    for id in stale {
        if let Some(s) = state.sessions.get_mut(&id) {
            s.activity = crate::state::Activity::Done;
            s.last_event_ts = now;
        }
    }
}

impl State {
    pub fn refresh_filtered(&mut self) {
        let mut entries = self.build_entries();

        // Apply cached running commands (no OS calls here)
        for entry in &mut entries {
            if entry.running_command.is_none() {
                if let Some(cmd) = self.running_command_cache.get(&entry.pane_id) {
                    entry.running_command = Some(cmd.clone());
                }
            }
        }

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
