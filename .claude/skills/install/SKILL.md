# /install — Interactive Claude-Guided Setup for claude-pane

## Description
Detects the user's environment and installs claude-pane step-by-step.

## Steps

### 1. Detect Environment
- Verify zellij >= 0.44: `zellij --version`
- Find zellij config dir: `$ZELLIJ_CONFIG_DIR` or `~/.config/zellij`
- Check for existing plugins: `ls $config_dir/plugins/`
- Check for zjstatus.wasm

### 2. Build or Download WASM
- If Rust toolchain available: `cargo build --release` in the claude-pane directory
- Otherwise: download from latest GitHub release
- Copy `.wasm` to `$config_dir/plugins/claude-pane.wasm`

### 3. Hook Bridge
- **Check if `send_event.py` already has zellij pipe call**: grep for `claude-pane:event` in `~/.claude/observability/hooks/send_event.py`
  - If found: skip hook registration (existing bridge handles events)
  - If not found: register `claude-pane-hook.sh` in Claude Code settings
- Copy `scripts/claude-pane-hook.sh` to `$config_dir/plugins/` and `chmod +x`
- If registering hooks, add to `~/.claude/settings.json`:
  ```json
  {
    "hooks": {
      "PreToolUse": [{"command": "$config_dir/plugins/claude-pane-hook.sh"}],
      "PostToolUse": [{"command": "$config_dir/plugins/claude-pane-hook.sh"}],
      "Notification": [{"command": "$config_dir/plugins/claude-pane-hook.sh"}],
      "Stop": [{"command": "$config_dir/plugins/claude-pane-hook.sh"}],
      "UserPromptSubmit": [{"command": "$config_dir/plugins/claude-pane-hook.sh"}]
    }
  }
  ```

### 4. Configure Keybindings
- Ask user: "Which key to open claude-pane? (default: Alt+o)"
- Add to `config.kdl` plugins block:
  ```kdl
  claude-pane location="file:~/.config/zellij/plugins/claude-pane.wasm" {
      // user config options here
  }
  ```
- Add to `load_plugins`: `"claude-pane"`
- Add keybinds in locked mode:
  ```kdl
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
  ```

### 5. Configure zjstatus (if present)
- If zjstatus.wasm found, update the layout file to include `{pipe_status}`:
  - Add `format_right "{pipe_status} {session}"` (or append to existing format_right)
  - Add `pipe_status_rendermode "dynamic"`

### 6. Verify
- `zellij pipe --name "claude-pane:test"` — check for "test ping OK" in zellij log
- Print summary of what was installed

### 7. Interactive Configuration
- Use `AskUserQuestion` for each decision point
- Show before/after diffs for config file changes
- Offer to revert if anything fails

### 8. Remote Setup (if SSH detected)
- Check `$SSH_CONNECTION` to detect remote context
- Same steps apply — plugin, hooks, and config all live on the remote host
- If `~/.claude/settings.json` is shared across machines, hook registrations are already present
- Just ensure `claude-pane-hook.sh` exists at the expected path on the remote host

## Important
- This skill is idempotent: running it again detects existing installation
- Never duplicate hook bridges (check for existing send_event.py integration)
- Always verify zellij version compatibility first
- Works on remote machines over SSH — the WASM plugin is host-agnostic
