# claude-pane

A [Zellij](https://zellij.dev) WASM plugin that manages multiple [Claude Code](https://docs.anthropic.com/en/docs/claude-code) sessions across tabs and panes.

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

## Remote Setup

The plugin runs inside Zellij's WASM runtime and is host-agnostic — it works wherever Zellij runs, including remote SSH sessions.

### Shared `~/.claude/settings.json`

If you sync `~/.claude/settings.json` across machines (dotfiles, NFS, etc.), hook registrations are already present on remote hosts. You only need to set up the WASM plugin and Zellij config.

### Steps

```bash
# 1. SSH to remote host
ssh <server>

# 2. Install Zellij (if not present)
cargo install zellij   # or use package manager

# 3. Copy WASM plugin
scp ~/.config/zellij/plugins/claude-pane.wasm <server>:~/.config/zellij/plugins/

# 4. Copy hook bridge script
scp ~/.config/zellij/plugins/claude-pane-hook.sh <server>:~/.config/zellij/plugins/
ssh <server> chmod +x ~/.config/zellij/plugins/claude-pane-hook.sh

# 5. Copy or create config.kdl on remote (same plugin block + keybinds)

# 6. Start Zellij on remote
ssh <server> -t zellij
```

Everything works identically: palette, flash notifications, zjstatus, star cycling, process detection.

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
# Output: target/wasm32-wasip1/release/claude-pane.wasm (~1.2MB)
```

## How It Works

1. Plugin loads in the background via `load_plugins`
2. Claude Code hooks fire on tool use, sending JSON payloads via `zellij pipe`
3. Plugin receives events, tracks activity per pane, pipes status to zjstatus
4. Tab names only change when attention needed (⚠ prefix)
5. Press `Alt+o` to open the floating palette — shows all panes with activity, process names, and stars
6. `Alt+u`/`Alt+i` cycle through starred panes without opening the palette
7. Non-Claude panes show their running process (nvim, lazygit, etc.)

## References

This is a personal project. Inspired by and builds on ideas from:

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
