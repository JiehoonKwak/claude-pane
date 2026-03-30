# /uninstall — Clean Removal of claude-pane

## Description
Removes claude-pane plugin and its configuration, non-destructively.

## Steps

### 1. Detect Installation
- Find `claude-pane.wasm` in plugins directory
- Check `config.kdl` for claude-pane references
- Check `settings.json` for hook registrations
- Check layouts for `pipe_status` references

### 2. Remove (with confirmation for each step)
Use `AskUserQuestion` before each modification:

1. **Remove WASM**: Delete `$config_dir/plugins/claude-pane.wasm`
2. **Remove hook script**: Delete `$config_dir/plugins/claude-pane-hook.sh`
3. **Clean config.kdl**:
   - Remove `claude-pane` from `plugins` block
   - Remove `"claude-pane"` from `load_plugins`
   - Remove keybind that references `claude-pane`
4. **Clean settings.json**: Remove claude-pane-hook.sh entries from hooks
   - Do NOT remove other hooks
5. **Clean layout**: Remove `pipe_status` and `pipe_status_rendermode` from zjstatus config
6. **Remove star data**: Delete `$config_dir/plugins/claude-pane.json`

### 3. Verify
- Confirm all artifacts removed
- Print summary

## Important
- Never remove `send_event.py` modifications (those belong to the observability system)
- Always ask before each destructive action
- Leave other hooks and plugins intact
