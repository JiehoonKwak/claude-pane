use crate::state::{format_elapsed, PaneEntry, State};

// ANSI escape helpers
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const REVERSE: &str = "\x1b[7m";

fn fg(hex: &str) -> String {
    let (r, g, b) = parse_hex(hex);
    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 {
        return (128, 128, 128);
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(128);
    (r, g, b)
}

/// Render the floating command palette.
pub fn render(state: &mut State, rows: usize, cols: usize) {
    if !state.visible {
        return;
    }

    let accent = "#648CF0";
    let muted = "#6c7086";

    // Header
    println!("{}{} pane-palette {}", fg(accent), BOLD, RESET);

    // Search bar
    if state.search_query.is_empty() {
        println!("{}/ search...{}", fg(muted), RESET);
    } else {
        println!("{}/ {}{}{}", fg(accent), BOLD, state.search_query, RESET);
    }
    println!();

    // Available rows for pane list (header=2 + blank=1 + footer=2)
    let list_rows = rows.saturating_sub(5);
    if list_rows == 0 {
        return;
    }

    let entries = &state.filtered_entries;

    state.jump_targets.clear();

    let lines_used = if entries.is_empty() {
        println!("{}  No matches{}", fg(muted), RESET);
        1
    } else if state.search_query.is_empty() {
        render_grouped(state, list_rows, cols)
    } else {
        render_flat(state, list_rows, cols)
    };

    // Footer padding
    for _ in 0..list_rows.saturating_sub(lines_used) {
        println!();
    }

    println!();
    println!(
        "{0}{1} j/k{2} nav  {0}{1} enter{2} go  {0}{1} space{2} star  \
         {0}{1} h/l{2} fold  {0}{1} 1-9{2} jump  {0}{1} esc{2} close{2}",
        fg(accent), BOLD, RESET,
    );
}

// ---------------------------------------------------------------------------
// Tab-grouped view (no search)
// ---------------------------------------------------------------------------

enum VisualItem<'a> {
    TabHeader {
        name: &'a str,
        active: bool,
        collapsed: bool,
        entry_count: usize,
    },
    Entry(usize, &'a PaneEntry),
}

fn render_grouped(state: &mut State, list_rows: usize, cols: usize) -> usize {
    let entries = &state.filtered_entries;
    let muted = "#6c7086";
    let accent = "#648CF0";

    // Build visual items: tab headers interleaved with entries
    let mut items: Vec<VisualItem> = Vec::new();
    let mut prev_tab: Option<usize> = None;
    let active_tab = state.tabs.iter().find(|t| t.active).map(|t| t.position);

    // Count entries per tab for collapsed headers
    let mut tab_counts: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    for entry in entries.iter() {
        *tab_counts.entry(entry.tab_index).or_insert(0) += 1;
    }

    for (i, entry) in entries.iter().enumerate() {
        if prev_tab != Some(entry.tab_index) {
            let collapsed = state.collapsed_tabs.contains(&entry.tab_index);
            let count = tab_counts.get(&entry.tab_index).copied().unwrap_or(0);
            items.push(VisualItem::TabHeader {
                name: &entry.tab_name,
                active: active_tab == Some(entry.tab_index),
                collapsed,
                entry_count: count,
            });
            prev_tab = Some(entry.tab_index);
        }
        // Skip entries for collapsed tabs
        if !state.collapsed_tabs.contains(&entry.tab_index) {
            items.push(VisualItem::Entry(i, entry));
        }
    }

    // Find visual position of selected entry
    let selected_visual = items
        .iter()
        .position(|item| matches!(item, VisualItem::Entry(i, _) if *i == state.selected_index))
        .unwrap_or(0);

    // Scroll window centered on selection
    let total = items.len();
    let start = if selected_visual >= list_rows {
        selected_visual - list_rows + 1
    } else {
        0
    };
    let end = (start + list_rows).min(total);
    let visible = end - start;

    // Build jump number mapping from the VISIBLE window only
    let mut jump_num: u8 = 0;
    let mut jump_map: std::collections::HashMap<usize, u8> = std::collections::HashMap::new();
    for item in items.iter().skip(start).take(visible) {
        if let VisualItem::Entry(i, _) = item {
            jump_num += 1;
            if jump_num <= 9 {
                jump_map.insert(*i, jump_num);
                state.jump_targets.push(*i);
            }
        }
    }

    for item in items.iter().skip(start).take(visible) {
        match item {
            VisualItem::TabHeader {
                name,
                active,
                collapsed,
                entry_count,
            } => {
                if *collapsed {
                    if *active {
                        println!(
                            "{}{}▸ {} {}({}){}", fg(accent), BOLD, name, fg(muted), entry_count, RESET
                        );
                    } else {
                        println!(
                            "{}▸ {} ({}){}", fg(muted), name, entry_count, RESET
                        );
                    }
                } else if *active {
                    println!("{}{}▾ {}{}", fg(accent), BOLD, name, RESET);
                } else {
                    println!("{}▾ {}{}", fg(muted), name, RESET);
                }
            }
            VisualItem::Entry(i, entry) => {
                let num = jump_map.get(i).copied();
                render_entry(state, entry, *i == state.selected_index, false, cols, num);
            }
        }
    }

    visible
}

// ---------------------------------------------------------------------------
// Flat view (search active)
// ---------------------------------------------------------------------------

fn render_flat(state: &mut State, list_rows: usize, cols: usize) -> usize {
    let entries = &state.filtered_entries;
    let total = entries.len();
    let start = if state.selected_index >= list_rows {
        state.selected_index - list_rows + 1
    } else {
        0
    };
    let end = (start + list_rows).min(total);
    let visible = end - start;

    let mut jump_num: u8 = 0;
    for (i, entry) in entries.iter().enumerate().skip(start).take(visible) {
        jump_num += 1;
        let num = if jump_num <= 9 {
            state.jump_targets.push(i);
            Some(jump_num)
        } else {
            None
        };
        render_entry(state, entry, i == state.selected_index, true, cols, num);
    }

    visible
}

// ---------------------------------------------------------------------------
// Entry rendering
// ---------------------------------------------------------------------------

fn render_entry(
    state: &State,
    entry: &PaneEntry,
    selected: bool,
    show_tab: bool,
    cols: usize,
    jump_num: Option<u8>,
) {
    let mut line = String::with_capacity(cols * 3);

    if selected {
        line.push_str(REVERSE);
    }

    // Jump number (2 visible cols)
    if let Some(n) = jump_num {
        line.push_str(&fg("#6c7086"));
        line.push_str(&format!("{:<2}", n));
        line.push_str(RESET);
        if selected {
            line.push_str(REVERSE);
        }
    } else {
        line.push_str("  ");
    }

    // Star (2 visible cols)
    if entry.is_starred {
        line.push_str(&fg("#ffdc00"));
        line.push_str("\u{2605} "); // ★
        line.push_str(RESET);
        if selected {
            line.push_str(REVERSE);
        }
    } else {
        line.push_str("  ");
    }

    // Activity symbol (2 visible cols)
    if let Some(ref session) = entry.session {
        let color = session.activity.color();
        let sym = session.activity.symbol();
        line.push_str(&fg(color));
        line.push_str(sym);
        line.push_str(RESET);
        if selected {
            line.push_str(REVERSE);
        }
        line.push(' ');
    } else {
        line.push_str("  ");
    }

    // Tab name in search mode (up to 15 visible cols: 12 name + " │ ")
    if show_tab {
        line.push_str(DIM);
        line.push_str(&truncate(&entry.tab_name, 12));
        line.push_str(RESET);
        if selected {
            line.push_str(REVERSE);
        }
        line.push_str(" \u{2502} "); // │
    }

    // Dynamic width: remaining columns for command + label
    let prefix_width = 2 + 2 + 2 + if show_tab { 15 } else { 0 };
    let content_width = cols.saturating_sub(prefix_width);

    let cmd = entry.running_command.as_deref().unwrap_or("");
    let is_claude = entry.session.is_some();

    if is_claude {
        // Claude session
        let session = entry.session.as_ref().unwrap();
        let elapsed_reserve = if state.config.show_elapsed_time { 5 } else { 0 };

        if !cmd.is_empty() {
            if session.activity.is_running() {
                line.push_str(&fg("#ff851b"));
            } else {
                line.push_str(&fg("#a9b1d6"));
            }
            line.push_str(BOLD);
            line.push_str(cmd);
            line.push_str(RESET);
            if selected {
                line.push_str(REVERSE);
            }
            line.push(' ');
        }

        let label = session.project_name.as_deref().unwrap_or(&entry.title);
        let cmd_used = if cmd.is_empty() { 0 } else { cmd.len() + 1 };
        let label_width = content_width
            .saturating_sub(cmd_used)
            .saturating_sub(elapsed_reserve);

        if session.activity.is_running() {
            line.push_str(&fg("#ff851b"));
        }
        if !cmd.is_empty() {
            line.push_str(DIM);
        } else {
            line.push_str(BOLD);
        }
        line.push_str(&truncate(label, label_width));
        line.push_str(RESET);
        if selected {
            line.push_str(REVERSE);
        }

        if state.config.show_elapsed_time {
            let elapsed = state.uptime_s - session.last_event_ts;
            if elapsed > 0.0 {
                line.push_str(&fg("#6c7086"));
                line.push_str(&format!(" {}", format_elapsed(elapsed)));
                line.push_str(RESET);
                if selected {
                    line.push_str(REVERSE);
                }
            }
        }
    } else {
        // Non-Claude pane (codex, nvim, lazygit, regular, etc.)
        if !cmd.is_empty() {
            let color = command_color(cmd).unwrap_or("#a9b1d6");
            line.push_str(&fg(color));
            line.push_str(BOLD);
            line.push_str(cmd);
            line.push_str(RESET);
            if selected {
                line.push_str(REVERSE);
            }
            line.push(' ');
            let title_width = content_width.saturating_sub(cmd.len() + 1);
            line.push_str(DIM);
            line.push_str(&truncate(&entry.title, title_width));
            line.push_str(RESET);
        } else {
            line.push_str(&truncate(&entry.title, content_width));
        }
    }

    line.push_str(RESET);

    println!("{}", truncate_ansi(&line, cols));
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('\u{2026}'); // …
        t
    }
}

/// Highlight color for known commands. None = default color.
pub fn command_color(cmd: &str) -> Option<&'static str> {
    match cmd {
        "codex" | "codex-cli" => Some("#10a37f"),
        "nvim" | "vim" | "vi" | "helix" | "hx" => Some("#57a143"),
        "lazygit" | "lazydocker" | "gitui" => Some("#e06c75"),
        "htop" | "btop" | "top" => Some("#e5c07b"),
        _ => None,
    }
}

/// Truncate an ANSI-coded string to max_visible display columns.
fn truncate_ansi(s: &str, max_visible: usize) -> String {
    let mut visible = 0;
    let mut in_escape = false;
    let mut result = String::with_capacity(s.len());

    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
            result.push(ch);
        } else if in_escape {
            result.push(ch);
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            if visible >= max_visible {
                result.push_str(RESET);
                return result;
            }
            result.push(ch);
            visible += 1;
        }
    }

    result
}
