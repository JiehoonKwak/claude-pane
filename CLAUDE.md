# claude-pane

Zellij WASM plugin for Claude Code session management.

## Architecture

**Single WASM instance** loaded via `load_plugins` (background). Floating palette toggled via `show_self(true)` / `hide_self()` on the SAME instance. Keybind invocation: `MessagePlugin` action in locked mode targeting the background plugin alias.

## Source Layout

```
src/
├── main.rs     # Plugin registration (cfg wasm32), event routing, fuzzy filter
├── state.rs    # State, SessionInfo, Activity, HookPayload, Config, PaneEntry
├── render.rs   # ANSI floating palette renderer
├── star.rs     # IndexSet<u32> bookmark with JSON persistence
└── tests.rs    # Unit tests (pure logic, no WASM deps)
```

## Build

```bash
cargo build --release          # → target/wasm32-wasip1/release/claude_pane.wasm
cargo test --lib               # host-target unit tests (26 tests)
```

## Key Design

- **Timer**: Single heartbeat loop. ONE `set_timeout` at a time. All deadline checks in timer tick.
- **zjstatus**: `pipe_message_to_plugin()` with debounce (250ms). Payload: `zjstatus::pipe::pipe_status::FORMATTED`.
- **Animation**: `highlight_and_unhighlight_panes` for border flash (0.5s blink). Falls back to zjstatus-only if frameless.
- **Tests**: `#[cfg(target_arch = "wasm32")]` gates all host-calling code. Tests run on native target.
- **Hook bridge**: ONE bridge per user. Detects existing `send_event.py` integration.

## Config Keys (14)

| Key | Default | Description |
|-----|---------|-------------|
| `key_select_down` | `j` | Navigate down in palette |
| `key_select_up` | `k` | Navigate up |
| `key_confirm` | `Enter` | Focus selected pane |
| `key_cancel` | `Esc` | Close palette |
| `key_toggle_star` | `Space` | Toggle star bookmark |
| `key_delete_char` | `Backspace` | Delete search char |
| `notification_flash` | `persist` | persist / brief / off |
| `flash_duration_ms` | `2000` | Brief flash duration |
| `done_timeout_s` | `30` | Done → Idle timeout |
| `idle_remove_s` | `300` | Idle → Remove timeout |
| `show_elapsed_time` | `true` | Show elapsed in palette |
| `show_non_claude` | `true` | Show non-Claude panes |
| `show_pane_id` | `true` | Show pane IDs |
| `zjstatus_pipe` | `true` | Enable zjstatus updates |

## Pipe Messages

| Name | Payload | Effect |
|------|---------|--------|
| `claude-pane:event` | HookPayload JSON | Upsert session |
| `show` / `claude-pane:show` | — | Open palette |
| `hide` / `claude-pane:hide` | — | Close palette |
| `dump-state` | — | Log state to stderr |
| `test` | — | Log "test ping OK" |
