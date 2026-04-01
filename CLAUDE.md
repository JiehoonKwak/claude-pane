# zellij-pane-palette

Zellij WASM plugin — AI pane palette with fuzzy search, bookmarks, and process display.

## Architecture

**Single WASM instance** loaded via `load_plugins` (background). Floating palette toggled via `show_self(true)` / `hide_self()` on the SAME instance. Keybind invocation: `MessagePlugin` action in locked mode targeting the background plugin alias.

## Source Layout

```
src/
├── main.rs     # Plugin entry, ZellijPlugin impl (cfg wasm32), pipe/key/mouse/timer handlers
├── state.rs    # State, SessionInfo, Activity, HookPayload, FocusPayload, Config, PaneEntry, format_elapsed
├── filter.rs   # Fuzzy search (nucleo-matcher), running command cache, stale session cleanup, refresh_filtered()
├── render.rs   # ANSI floating palette renderer (grouped/flat views, fold/unfold, jump numbers, codex highlighting)
├── star.rs     # IndexSet<u32> bookmark with JSON persistence (~/.config/zellij/plugins/pane-palette.json)
└── tests.rs    # Unit tests (pure logic, no WASM deps)
```

## Build

```bash
cargo build --release                        # → target/wasm32-wasip1/release/zellij-pane-palette.wasm
cargo test --target aarch64-apple-darwin     # host-target unit tests (32 tests)
bash scripts/verify.sh                       # live Zellij integration tests
```

## Key Design

- **Timer**: Single heartbeat loop. ONE `set_timeout` at a time (0.1s during flash/highlight, 1.0s otherwise). All deadline checks in timer tick.
- **Flash**: `set_pane_color()` background tinting. Tick-based blink (alternates bg on even/odd tick at 0.1s). Flash-expiry reset path ensures deterministic cleanup. Focused pane skips persist tint.
- **Fuzzy search**: `nucleo-matcher` in `filter.rs`. Haystack = tab_name + title + project_name + running_command + pane_id.
- **Tests**: `#[cfg(target_arch = "wasm32")]` gates all host-calling code. Tests run on native target.
- **Palette entries**: Plugin panes always hidden. Terminal panes deduplicated by pane_id. Sorted: tab_index → starred → pane_id.
- **Fold/unfold**: `collapsed_tabs: HashSet<usize>` tracks collapsed tab groups. h/l toggle, j/k skip collapsed.
- **Process detection**: `get_pane_running_command()` called for ALL panes when palette opens. Cached in `running_command_cache`. Also marks stale sessions as Done (time + shell-confirmed).
- **Running indicator**: Orange text for active Claude sessions (`Activity::is_running()`). Green for Codex processes.
- **Jump numbers**: 1-9 displayed for visible window entries only. `jump_targets` in State syncs render ↔ key handler.
- **Staleness**: Timer-based (idle_remove_s with no hook events) + process-confirmed on palette open. Process check only triggers when foreground is a shell (zsh/bash/fish/sh) — non-shell processes may be Claude tool invocations.
- **Backwards compat**: `claude-pane:event` pipe alias still accepted for existing hook bridges.
- **Star cycling**: `star-next`/`star-prev` pipe messages. Cross-tab focus via `switch_tab_to` + `focus_terminal_pane`.
- **Focus pipe**: Direct pane focus with optional flash color/duration via `FocusPayload`. Unblocks CLI pipe input.
- **Hook bridge**: ONE bridge per user. Detects existing `send_event.py` integration.

## Config Keys (13)

| Key | Default | Description |
|-----|---------|-------------|
| `key_select_down` | `j` | Navigate down in palette |
| `key_select_up` | `k` | Navigate up |
| `key_confirm` | `Enter` | Focus selected pane |
| `key_cancel` | `Esc` | Close palette |
| `key_toggle_star` | `Space` | Toggle star bookmark |
| `key_delete_char` | `Backspace` | Delete search char |
| `notification_flash` | `persist` | persist / brief / off |
| `flash_duration_ms` | `2000` | Brief flash duration (ms) |
| `done_timeout_s` | `30` | Done → Idle timeout (s) |
| `idle_remove_s` | `300` | Idle → Remove timeout (s) |
| `show_elapsed_time` | `true` | Show elapsed in palette |
| `show_non_claude` | `true` | Show non-Claude panes |
| `focus_highlight_s` | `0.5` | Selection highlight duration (s) |

## Pipe Messages

| Name | Payload | Effect |
|------|---------|--------|
| `pane-palette:event` / `event` | HookPayload JSON | Upsert session |
| `show` / `pane-palette:show` | — | Open palette |
| `hide` / `pane-palette:hide` | — | Close palette |
| `star-next` / `pane-palette:star-next` | — | Focus next starred pane |
| `star-prev` / `pane-palette:star-prev` | — | Focus previous starred pane |
| `focus` / `pane-palette:focus` | FocusPayload JSON | Direct pane focus with flash |
| `dump-state` / `pane-palette:dump-state` | — | Log state to file |
| `test` / `pane-palette:test` | — | Log "test ping OK" |
