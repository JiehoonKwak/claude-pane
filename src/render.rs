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
pub fn render(state: &State, rows: usize, cols: usize) {
    if !state.visible {
        return;
    }

    let accent = "#648CF0";
    let muted = "#6c7086";
    // Header
    let header = format!(
        "{}{} claude-pane {}{}",
        fg(accent),
        BOLD,
        RESET,
        fg(muted),
    );
    println!("{}", header);

    // Search bar
    let search_icon = if state.search_query.is_empty() {
        format!("{}/ search...{}", fg(muted), RESET)
    } else {
        format!(
            "{}/ {}{}{}",
            fg(accent),
            BOLD,
            state.search_query,
            RESET
        )
    };
    println!("{}", search_icon);
    println!();

    // Available rows for pane list (header=2 + blank=1 + footer=2)
    let list_rows = rows.saturating_sub(5);
    if list_rows == 0 {
        return;
    }

    let entries = &state.filtered_entries;

    if entries.is_empty() {
        println!(
            "{}  No matches{}",
            fg(muted),
            RESET
        );
    } else {
        // Scrolling window around selected_index
        let total = entries.len();
        let start = if state.selected_index >= list_rows {
            state.selected_index - list_rows + 1
        } else {
            0
        };
        let end = (start + list_rows).min(total);

        for (i, entry) in entries.iter().enumerate().skip(start).take(end - start) {
            render_entry(state, entry, i == state.selected_index, cols);
        }
    }

    // Footer
    let blank_lines = rows
        .saturating_sub(5)
        .saturating_sub(entries.len().min(list_rows));
    for _ in 0..blank_lines {
        println!();
    }

    println!();
    println!(
        "{}{} j/k{} nav  {}{} enter{} go  {}{} space{} star  {}{} esc{} close{}",
        fg(accent), BOLD, RESET,
        fg(accent), BOLD, RESET,
        fg(accent), BOLD, RESET,
        fg(accent), BOLD, RESET,
        RESET,
    );
}

fn render_entry(state: &State, entry: &PaneEntry, selected: bool, cols: usize) {
    let mut line = String::with_capacity(cols);

    // Selection indicator
    if selected {
        line.push_str(REVERSE);
    }

    // Star
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

    // Activity symbol (if Claude session)
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

    // Pane ID
    if state.config.show_pane_id {
        line.push_str(&fg("#6c7086"));
        line.push_str(&format!("#{:<3} ", entry.pane_id));
        line.push_str(RESET);
        if selected {
            line.push_str(REVERSE);
        }
    }

    // Tab name
    line.push_str(DIM);
    let tab_display = truncate(&entry.tab_name, 12);
    line.push_str(&tab_display);
    line.push_str(RESET);
    if selected {
        line.push_str(REVERSE);
    }
    line.push_str(" \u{2502} "); // │

    // Title / project name
    if let Some(ref session) = entry.session {
        let label = session
            .project_name
            .as_deref()
            .unwrap_or(&entry.title);
        line.push_str(BOLD);
        line.push_str(&truncate(label, 24));
        line.push_str(RESET);
        if selected {
            line.push_str(REVERSE);
        }

        // Elapsed time
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
        line.push_str(&truncate(&entry.title, 36));
    }

    line.push_str(RESET);

    // Truncate to terminal width
    println!("{}", &line[..line.len().min(cols * 4)]); // rough limit; ANSI codes inflate length
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('\u{2026}'); // …
        t
    }
}
