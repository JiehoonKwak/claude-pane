# claude-pane

A [Zellij](https://zellij.dev) WASM plugin that manages multiple [Claude Code](https://docs.anthropic.com/en/docs/claude-code) sessions across tabs and panes.

![claude-pane command palette](assets/screenshot.png)

## Features

- **zjstatus integration** — activity symbols in your status bar (⚡ ✎ ◎ ✓ ⚠), notification-only tab indicators
- **Background flash** — pane background blinks when Claude needs attention (permission, notification); clears on focus
- **Command palette** — fuzzy-searchable pane picker with fold/unfold tabs, number selection (1-9), process name display
- **Star bookmarks** — pin important panes, cycle through them with Alt+U/I across tabs
- **Running indicator** — orange dot shows which Claude sessions are actively running
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

4. Add keybinds to open the palette and cycle starred panes:

```kdl
keybinds {
    locked {
        bind "Alt o" {
            MessagePlugin "claude-pane" {
                name "show"
            }
        }
        bind "Alt u" {
            MessagePlugin "claude-pane" {
                name "star-prev"
            }
        }
        bind "Alt i" {
            MessagePlugin "claude-pane" {
                name "star-next"
            }
        }
    }
}
```

5. Register the hook bridge:

```bash
cp scripts/claude-pane-hook.sh ~/.config/zellij/plugins/
chmod +x ~/.config/zellij/plugins/claude-pane-hook.sh
```

Add to `~/.claude/settings.json`:

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

## Command Palette

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `Enter` | Focus selected pane |
| `Space` | Toggle star bookmark |
| `h` / `l` | Fold/unfold tab group |
| `1`-`9` | Jump to Nth visible entry |
| `Backspace` | Delete search character |
| `Esc` | Close palette |
| Type | Fuzzy search |

## Pipe Messages

| Name | Payload | Effect |
|------|---------|--------|
| `show` / `claude-pane:show` | — | Open palette |
| `hide` / `claude-pane:hide` | — | Close palette |
| `star-next` / `claude-pane:star-next` | — | Focus next starred pane |
| `star-prev` / `claude-pane:star-prev` | — | Focus previous starred pane |
| `claude-pane:event` / `event` | HookPayload JSON | Upsert session |
| `dump-state` / `claude-pane:dump-state` | — | Write state diagnostic |
| `test` / `claude-pane:test` | — | Test ping |

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
rustup target add wasm32-wasip1
cargo build --release
# → target/wasm32-wasip1/release/claude-pane.wasm
```

## References

**Zellij + Claude Code**
- [claude-code-zellij-status](https://github.com/thoo/claude-code-zellij-status) — Monitor Claude Code activity via zjstatus
- [claude-zellij-whip](https://github.com/rvcas/claude-zellij-whip) — Claude Code notifications for Zellij with pane focusing

**Zellij Plugins**
- [zjstatus](https://github.com/dj95/zjstatus) — Configurable Zellij status bar
- [room](https://github.com/rvcas/room) — Fuzzy tab switcher
- [harpoon](https://github.com/Nacho114/harpoon) — Pane bookmarks (nvim-harpoon port)
- [zellij-pane-picker](https://github.com/shihanng/zellij-pane-picker) — Floating pane switcher with filtering and starring

**Claude Code Session Management**
- [claude-code-tools](https://github.com/pchalasani/claude-code-tools) — Productivity tools for Claude Code (tmux workflows, hooks)
- [recon](https://github.com/gavraz/recon) — tmux dashboard for Claude Code agents
- [tmux-agent-indicator](https://github.com/accessd/tmux-agent-indicator) — Hooks-driven AI agent state visualization for tmux

## License

MIT
