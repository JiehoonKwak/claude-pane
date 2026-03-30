# claude-pane

Zellij WASM plugin for Claude Code session management.

## Architecture

**Single WASM instance** loaded via `load_plugins` (background). Floating palette toggled via `show_self(true)` / `hide_self()` on the SAME instance. Keybind invocation: `MessagePlugin` action in locked mode targeting the background plugin alias.

## Source Layout

```
src/
├── main.rs     # Plugin entry, ZellijPlugin impl (cfg wasm32), pipe/key/mouse/timer handlers, fuzzy filter (nucleo-matcher)
├── state.rs    # State, SessionInfo, Activity, HookPayload, Config, PaneEntry, zjstatus formatting
├── render.rs   # ANSI floating palette renderer
├── star.rs     # IndexSet<u32> bookmark with JSON persistence (~/.config/zellij/plugins/claude-pane.json)
└── tests.rs    # Unit tests (pure logic, no WASM deps)
```

## Build

```bash
cargo build --release                        # → target/wasm32-wasip1/release/claude_pane.wasm
cargo test --target aarch64-apple-darwin     # host-target unit tests (26 tests)
```

## Key Design

- **Timer**: Single heartbeat loop. ONE `set_timeout` at a time (0.5s during flash, 1.0s otherwise). All deadline checks in timer tick.
- **Flash**: `set_pane_color()` background tinting. Tick-based blink (alternates bg on/off each 0.5s tick). Focus highlight uses steady tint (2s).
- **zjstatus**: `pipe_message_to_plugin()` broadcast (no URL) with debounce (250ms). Protocol: `zjstatus::pipe::pipe_status::{FORMATTED}`.
- **Fuzzy search**: `nucleo-matcher` for palette filtering. Haystack = tab_name + title + project_name + pane_id.
- **Tests**: `#[cfg(target_arch = "wasm32")]` gates all host-calling code. Tests run on native target.
- **Palette entries**: Plugin panes always hidden. Terminal panes deduplicated by pane_id. Sorted: starred → tab_index → pane_id.
- **Hook bridge**: ONE bridge per user. Detects existing `send_event.py` integration.

## Config Keys (15)

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
| `show_pane_id` | `true` | Show pane IDs |
| `zjstatus_pipe` | `true` | Enable zjstatus updates |
| `zjstatus_url` | `file:~/.config/zellij/plugins/zjstatus.wasm` | zjstatus plugin URL |

## Pipe Messages

| Name | Payload | Effect |
|------|---------|--------|
| `claude-pane:event` / `event` | HookPayload JSON | Upsert session |
| `show` / `claude-pane:show` | — | Open palette |
| `hide` / `claude-pane:hide` | — | Close palette |
| `dump-state` / `claude-pane:dump-state` | — | Log state to stderr |
| `test` / `claude-pane:test` | — | Log "test ping OK" |
