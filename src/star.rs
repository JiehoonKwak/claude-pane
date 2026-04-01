use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const PERSIST_PATH: &str = ".config/zellij/plugins/pane-palette.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StarSet {
    #[serde(default)]
    starred: IndexSet<u32>,
    /// Current cycle position (wraps around)
    #[serde(skip)]
    cycle_index: usize,
}

#[allow(dead_code)]
impl StarSet {
    pub fn new() -> Self {
        Self::load().unwrap_or_default()
    }

    pub fn contains(&self, pane_id: u32) -> bool {
        self.starred.contains(&pane_id)
    }

    pub fn toggle(&mut self, pane_id: u32) {
        if self.starred.contains(&pane_id) {
            self.starred.shift_remove(&pane_id);
        } else {
            self.starred.insert(pane_id);
        }
        self.save();
    }

    pub fn is_empty(&self) -> bool {
        self.starred.is_empty()
    }

    pub fn len(&self) -> usize {
        self.starred.len()
    }

    /// Get the next starred pane_id (forward cycle).
    pub fn next(&mut self) -> Option<u32> {
        if self.starred.is_empty() {
            return None;
        }
        self.cycle_index %= self.starred.len();
        let id = self.starred[self.cycle_index];
        self.cycle_index = (self.cycle_index + 1) % self.starred.len();
        Some(id)
    }

    /// Get the previous starred pane_id (backward cycle).
    pub fn prev(&mut self) -> Option<u32> {
        if self.starred.is_empty() {
            return None;
        }
        if self.cycle_index == 0 {
            self.cycle_index = self.starred.len() - 1;
        } else {
            self.cycle_index -= 1;
        }
        Some(self.starred[self.cycle_index])
    }

    /// Remove pane_ids that no longer exist.
    pub fn prune(&mut self, live: &HashSet<u32>) {
        let before = self.starred.len();
        self.starred.retain(|id| live.contains(id));
        if self.starred.len() != before {
            self.cycle_index = 0;
            self.save();
        }
    }

    fn persist_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(PERSIST_PATH)
    }

    fn load() -> Option<Self> {
        let new_path = Self::persist_path();
        if let Ok(data) = std::fs::read_to_string(&new_path) {
            return serde_json::from_str(&data).ok();
        }
        // Migration: try old path
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let old_path = PathBuf::from(home).join(".config/zellij/plugins/claude-pane.json");
        let data = std::fs::read_to_string(old_path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save(&self) {
        let path = Self::persist_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
