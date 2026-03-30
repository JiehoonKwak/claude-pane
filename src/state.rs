use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use zellij_tile::prelude::*;

use crate::star::StarSet;

// ---------------------------------------------------------------------------
// Activity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    Thinking,
    Reading,
    Writing,
    BashExec,
    WebSearch,
    Agent,
    Mcp,
    Done,
    PermissionNeeded,
    Notification,
    Idle,
}

impl Activity {
    pub fn from_hook_event(event: &str, tool_name: Option<&str>) -> Self {
        match event {
            "Stop" => Activity::Done,
            "Notification" => Activity::Notification,
            "UserPromptSubmit" => Activity::Thinking,
            "PreToolUse" | "PostToolUse" => match tool_name {
                Some("Read" | "Glob" | "Grep" | "ListFiles") => Activity::Reading,
                Some("Write" | "Edit" | "NotebookEdit") => Activity::Writing,
                Some("Bash") => Activity::BashExec,
                Some("WebSearch" | "WebFetch") => Activity::WebSearch,
                Some("Agent") => Activity::Agent,
                Some(t) if t.starts_with("mcp__") => Activity::Mcp,
                _ => Activity::Thinking,
            },
            s if s.contains("ermission") => Activity::PermissionNeeded,
            _ => Activity::Thinking,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Thinking => "\u{25d0}",         // ◐
            Self::Reading => "\u{25ce}",           // ◎
            Self::Writing => "\u{270e}",           // ✎
            Self::BashExec => "\u{26a1}",          // ⚡
            Self::WebSearch => "\u{25cd}",         // ◍
            Self::Agent | Self::Mcp => "\u{25b6}", // ▶
            Self::Done => "\u{2713}",              // ✓
            Self::PermissionNeeded => "\u{26a0}",  // ⚠
            Self::Notification => "?",
            Self::Idle => "\u{25cb}",              // ○
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Thinking => "#a9b1d6",
            Self::Reading | Self::WebSearch => "#0074d9",
            Self::Writing => "#4166F5",
            Self::BashExec => "#ff851b",
            Self::Agent | Self::Mcp => "#b10dc9",
            Self::Done => "#2ecc40",
            Self::PermissionNeeded | Self::Notification => "#ff4136",
            Self::Idle => "#6c7086",
        }
    }

    pub fn is_attention(self) -> bool {
        matches!(self, Self::PermissionNeeded | Self::Notification)
    }
}

// ---------------------------------------------------------------------------
// SessionInfo
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub pane_id: u32,
    pub activity: Activity,
    pub tab_index: Option<usize>,
    pub tab_name: Option<String>,
    pub last_event_ts: f64,
    pub project_name: Option<String>,
    pub tool_name: Option<String>,
    // Deadline-based animation state (absolute uptime_s values)
    pub flash_deadline: f64,
    pub focus_highlight_deadline: f64,
}

impl SessionInfo {
    pub fn new(pane_id: u32, activity: Activity, now: f64) -> Self {
        Self {
            pane_id,
            activity,
            tab_index: None,
            tab_name: None,
            last_event_ts: now,
            project_name: None,
            tool_name: None,
            flash_deadline: 0.0,
            focus_highlight_deadline: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// HookPayload (from hook bridge)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct HookPayload {
    pub pane_id: u32,
    #[serde(default)]
    #[allow(dead_code)]
    pub session_id: Option<String>,
    pub hook_event: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationFlash {
    Persist,
    Brief,
    Off,
}

#[derive(Debug, Clone)]
pub struct Config {
    // Palette keybindings stored as lowercase strings for matching
    pub key_select_down: String,
    pub key_select_up: String,
    pub key_confirm: String,
    pub key_cancel: String,
    pub key_toggle_star: String,
    pub key_delete_char: String,

    // Notification behavior
    pub notification_flash: NotificationFlash,
    pub flash_duration_ms: u64,
    pub done_timeout_s: f64,
    pub idle_remove_s: f64,

    // Display
    pub show_elapsed_time: bool,
    pub show_non_claude: bool,
    pub show_pane_id: bool,

    // zjstatus
    pub zjstatus_pipe: bool,
    pub zjstatus_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            key_select_down: "j".into(),
            key_select_up: "k".into(),
            key_confirm: "Enter".into(),
            key_cancel: "Esc".into(),
            key_toggle_star: "Space".into(),
            key_delete_char: "Backspace".into(),
            notification_flash: NotificationFlash::Persist,
            flash_duration_ms: 2000,
            done_timeout_s: 30.0,
            idle_remove_s: 300.0,
            show_elapsed_time: true,
            show_non_claude: true,
            show_pane_id: true,
            zjstatus_pipe: true,
            zjstatus_url: "file:~/.config/zellij/plugins/zjstatus.wasm".into(),
        }
    }
}

impl Config {
    pub fn from_map(map: &BTreeMap<String, String>) -> Self {
        let mut cfg = Self::default();

        if let Some(v) = map.get("key_select_down") {
            cfg.key_select_down = v.clone();
        }
        if let Some(v) = map.get("key_select_up") {
            cfg.key_select_up = v.clone();
        }
        if let Some(v) = map.get("key_confirm") {
            cfg.key_confirm = v.clone();
        }
        if let Some(v) = map.get("key_cancel") {
            cfg.key_cancel = v.clone();
        }
        if let Some(v) = map.get("key_toggle_star") {
            cfg.key_toggle_star = v.clone();
        }
        if let Some(v) = map.get("key_delete_char") {
            cfg.key_delete_char = v.clone();
        }
        if let Some(v) = map.get("notification_flash") {
            cfg.notification_flash = match v.as_str() {
                "brief" => NotificationFlash::Brief,
                "off" => NotificationFlash::Off,
                _ => NotificationFlash::Persist,
            };
        }
        if let Some(v) = map.get("flash_duration_ms") {
            cfg.flash_duration_ms = v.parse().unwrap_or(cfg.flash_duration_ms);
        }
        if let Some(v) = map.get("done_timeout_s") {
            cfg.done_timeout_s = v.parse().unwrap_or(cfg.done_timeout_s);
        }
        if let Some(v) = map.get("idle_remove_s") {
            cfg.idle_remove_s = v.parse().unwrap_or(cfg.idle_remove_s);
        }
        if let Some(v) = map.get("show_elapsed_time") {
            cfg.show_elapsed_time = v != "false";
        }
        if let Some(v) = map.get("show_non_claude") {
            cfg.show_non_claude = v != "false";
        }
        if let Some(v) = map.get("show_pane_id") {
            cfg.show_pane_id = v != "false";
        }
        if let Some(v) = map.get("zjstatus_pipe") {
            cfg.zjstatus_pipe = v != "false";
        }
        if let Some(v) = map.get("zjstatus_url") {
            cfg.zjstatus_url = v.clone();
        }

        cfg
    }

    /// Check if a key event matches a config key string.
    pub fn key_matches(key: &KeyWithModifier, config_key: &str) -> bool {
        match config_key {
            "Enter" => key.bare_key == BareKey::Enter && key.has_no_modifiers(),
            "Esc" => key.bare_key == BareKey::Esc && key.has_no_modifiers(),
            "Space" => key.bare_key == BareKey::Char(' ') && key.has_no_modifiers(),
            "Backspace" => key.bare_key == BareKey::Backspace && key.has_no_modifiers(),
            "Tab" => key.bare_key == BareKey::Tab && key.has_no_modifiers(),
            s if s.len() == 1 => {
                let ch = s.chars().next().unwrap();
                key.bare_key == BareKey::Char(ch) && key.has_no_modifiers()
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Pane entry for palette display
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PaneEntry {
    pub pane_id: u32,
    pub tab_index: usize,
    pub tab_name: String,
    pub title: String,
    pub is_plugin: bool,
    pub is_focused: bool,
    /// Claude session info if this pane has one
    pub session: Option<SessionInfo>,
    pub is_starred: bool,
}

// ---------------------------------------------------------------------------
// State (main plugin state)
// ---------------------------------------------------------------------------

pub struct State {
    pub config: Config,
    pub sessions: BTreeMap<u32, SessionInfo>,
    pub pane_manifest: HashMap<usize, Vec<PaneInfo>>,
    pub tabs: Vec<TabInfo>,
    pub stars: StarSet,

    // UI state
    pub visible: bool,
    pub search_query: String,
    pub selected_index: usize,
    pub filtered_entries: Vec<PaneEntry>,

    // Focus tracking
    pub current_focus_pane: Option<u32>,
    pub previous_focus_pane: Option<u32>,

    // Timer state
    pub uptime_s: f64,
    pub tick_count: u64,
    pub permissions_granted: bool,

    // zjstatus debounce
    pub last_zjstatus_update: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            config: Config::default(),
            sessions: BTreeMap::new(),
            pane_manifest: HashMap::new(),
            tabs: Vec::new(),
            stars: StarSet::new(),

            visible: false,
            search_query: String::new(),
            selected_index: 0,
            filtered_entries: Vec::new(),

            current_focus_pane: None,
            previous_focus_pane: None,

            uptime_s: 0.0,
            tick_count: 0,
            permissions_granted: false,

            last_zjstatus_update: 0.0,
        }
    }
}

impl State {
    /// Rebuild pane_to_tab mapping and enrich sessions with tab info.
    pub fn rebuild_pane_map(&mut self) {
        for (&tab_idx, panes) in &self.pane_manifest {
            let tab_name = self
                .tabs
                .iter()
                .find(|t| t.position == tab_idx)
                .map(|t| t.name.clone());

            for pane in panes {
                if let Some(session) = self.sessions.get_mut(&pane.id) {
                    session.tab_index = Some(tab_idx);
                    session.tab_name = tab_name.clone();
                }
            }
        }
    }

    /// Prune sessions whose pane_id no longer exists in the manifest.
    pub fn prune_dead_sessions(&mut self) {
        let live_pane_ids: std::collections::HashSet<u32> = self
            .pane_manifest
            .values()
            .flat_map(|panes| panes.iter().map(|p| p.id))
            .collect();

        self.sessions.retain(|id, _| live_pane_ids.contains(id));
        self.stars.prune(&live_pane_ids);
    }

    /// Build the list of pane entries for the palette.
    pub fn build_entries(&self) -> Vec<PaneEntry> {
        let mut entries = Vec::new();

        for (&tab_idx, panes) in &self.pane_manifest {
            let tab_name = self
                .tabs
                .iter()
                .find(|t| t.position == tab_idx)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| format!("Tab {}", tab_idx));

            for pane in panes {
                if pane.is_plugin && !self.sessions.contains_key(&pane.id) {
                    continue; // skip non-Claude plugin panes
                }
                if !self.config.show_non_claude
                    && !pane.is_plugin
                    && !self.sessions.contains_key(&pane.id)
                {
                    continue;
                }

                entries.push(PaneEntry {
                    pane_id: pane.id,
                    tab_index: tab_idx,
                    tab_name: tab_name.clone(),
                    title: pane.title.clone(),
                    is_plugin: pane.is_plugin,
                    is_focused: pane.is_focused,
                    session: self.sessions.get(&pane.id).cloned(),
                    is_starred: self.stars.contains(pane.id),
                });
            }
        }

        // Sort: starred first, then by tab index, then pane_id
        entries.sort_by(|a, b| {
            b.is_starred
                .cmp(&a.is_starred)
                .then(a.tab_index.cmp(&b.tab_index))
                .then(a.pane_id.cmp(&b.pane_id))
        });

        entries
    }

    pub fn format_zjstatus(&self) -> String {
        format_zjstatus(&self.sessions)
    }
}

// ---------------------------------------------------------------------------
// Free functions (testable without WASM host)
// ---------------------------------------------------------------------------

/// Format the zjstatus pipe_status string.
pub fn format_zjstatus(sessions: &BTreeMap<u32, SessionInfo>) -> String {
    if sessions.is_empty() {
        return String::new();
    }

    let mut parts: Vec<String> = Vec::new();
    for session in sessions.values() {
        let sym = session.activity.symbol();
        let color = session.activity.color();
        let label = session.project_name.as_deref().unwrap_or("");

        if label.is_empty() {
            parts.push(format!("#[fg={}]{}", color, sym));
        } else {
            parts.push(format!("#[fg={}]{} {}", color, sym, label));
        }
    }

    parts.join("  ")
}

/// Format elapsed time as human-readable string.
pub fn format_elapsed(seconds: f64) -> String {
    let s = seconds as u64;
    if s < 60 {
        format!("{}s", s)
    } else if s < 3600 {
        format!("{}m", s / 60)
    } else {
        format!("{}h", s / 3600)
    }
}
