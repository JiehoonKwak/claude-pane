# claude-pane

A [Zellij](https://zellij.dev) WASM plugin that manages multiple [Claude Code](https://docs.anthropic.com/en/docs/claude-code) sessions across tabs and panes.

**Scope**: Local Zellij sessions only (v0.1.0). Remote SSH panes are out of scope.

## Features

- **zjstatus integration** — activity symbols in your status bar (⚡ ✎ ◎ ✓ ⚠)
- **Border highlighting** — pane borders flash purple when Claude needs attention
- **Command palette** — fuzzy-searchable pane picker across all tabs
- **Star bookmarks** — pin important panes, cycle through them with a keybind
- **Hook bridge** — zero-config if you use Claude Code's hook system

## Quick Start

### Requirements

- Zellij >= 0.44.0
- [zjstatus](https://github.com/dj95/zjstatus) (optional, for status bar symbols)

### Install

**Option A: `/install` skill (recommended)**

Open Claude Code inside Zellij and run `/install`. The skill detects your environment and configures everything interactively.

**Option B: Manual**

1. Download `claude_pane.wasm` from the [latest release](https://github.com/jiehoonk/claude-pane/releases)
2. Copy to `~/.config/zellij/plugins/claude-pane.wasm`
3. Add to your `config.kdl`:

```kdl
plugins {
    claude-pane location="file:~/.config/zellij/plugins/claude-pane.wasm" {
        notification_flash "persist"
        done_timeout_s     "30"
        idle_remove_s      "300"
    }
}

load_plugins {
    "claude-pane"
}
```

4. Add a keybind to open the palette:

```kdl
keybinds {
    locked {
        bind "Alt o" {
            MessagePlugin "claude-pane" {
                name "show"
            }
        }
    }
}
```

5. Register the hook bridge (skip if you already have `send_event.py` with zellij pipe):

```bash
cp scripts/claude-pane-hook.sh ~/.config/zellij/plugins/
chmod +x ~/.config/zellij/plugins/claude-pane-hook.sh
```

Add to your Claude Code `settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [{ "command": "~/.config/zellij/plugins/claude-pane-hook.sh" }],
    "PostToolUse": [{ "command": "~/.config/zellij/plugins/claude-pane-hook.sh" }],
    "Notification": [{ "command": "~/.config/zellij/plugins/claude-pane-hook.sh" }],
    "Stop": [{ "command": "~/.config/zellij/plugins/claude-pane-hook.sh" }],
    "UserPromptSubmit": [{ "command": "~/.config/zellij/plugins/claude-pane-hook.sh" }]
  }
}
```

6. If using zjstatus, add to your layout:

```kdl
format_right "{pipe_status} {session}"
pipe_status_rendermode "dynamic"
```

## Activity Symbols

| Symbol | Activity | Color |
|--------|----------|-------|
| ◐ | Thinking | `#a9b1d6` |
| ◎ | Reading | `#0074d9` |
| ✎ | Writing | `#4166F5` |
| ⚡ | Bash/Shell | `#ff851b` |
| ◍ | Web Search | `#0074d9` |
| ▶ | Agent/MCP | `#b10dc9` |
| ✓ | Done | `#2ecc40` |
| ⚠ | Permission Needed | `#ff4136` |
| ○ | Idle | `#6c7086` |

## Configuration

All keys are optional. Defaults shown below.

```kdl
claude-pane location="file:~/.config/zellij/plugins/claude-pane.wasm" {
    // Palette keybindings
    key_select_down    "j"
    key_select_up      "k"
    key_confirm        "Enter"
    key_cancel         "Esc"
    key_toggle_star    "Space"
    key_delete_char    "Backspace"

    // Notification behavior
    notification_flash "persist"    // persist | brief | off
    flash_duration_ms  "2000"
    done_timeout_s     "30"
    idle_remove_s      "300"

    // Display
    show_elapsed_time  "true"
    show_non_claude    "true"
    show_pane_id       "true"

    // zjstatus
    zjstatus_pipe      "true"
}
```

## Build from Source

```bash
# Requires Rust with wasm32-wasip1 target
rustup target add wasm32-wasip1
cargo build --release
# Output: target/wasm32-wasip1/release/claude_pane.wasm (~960KB)
```

## How It Works

1. Plugin loads in the background via `load_plugins`
2. Claude Code hooks fire on tool use, sending JSON payloads via `zellij pipe`
3. Plugin receives events, tracks activity per pane, updates zjstatus and pane borders
4. Press `Alt+o` to open the floating palette — shows all panes enriched with Claude activity
5. Fuzzy search, navigate, star bookmarks, cross-tab focus

## References

Inspired by and builds on ideas from:

- [cmux](https://github.com/jnoortheen/cmux) — Claude session manager for tmux
- [zellaude](https://github.com/nag763/zellaude) — Zellij + Claude automation
- [claude-zellij-whip](https://github.com/kennethnym/claude-zellij-whip) — Claude Code in Zellij workflows
- [zellij-pane-picker](https://github.com/Jikstra/zellij-pane-picker) — Fuzzy pane picker plugin
- [zjstatus](https://github.com/dj95/zjstatus) — Configurable Zellij status bar

## License

MIT
