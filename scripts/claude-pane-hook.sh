#!/bin/sh
# claude-pane hook bridge for Claude Code
# Fire-and-forget: always exit 0, never block Claude Code
[ -z "$ZELLIJ_SESSION_NAME" ] && exit 0
[ -z "$ZELLIJ_PANE_ID" ] && exit 0

# Sanitize values for JSON (escape quotes and backslashes)
escape_json() { printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/	/\\t/g'; }

HOOK_EVENT=$(escape_json "${CLAUDE_HOOK_EVENT_TYPE:-unknown}")
TOOL_NAME=$(escape_json "${CLAUDE_TOOL_NAME:-}")
PROJECT_NAME=$(escape_json "${CLAUDE_PROJECT_DIR##*/}")

PAYLOAD="{\"pane_id\":$ZELLIJ_PANE_ID,\"hook_event\":\"$HOOK_EVENT\",\"tool_name\":\"$TOOL_NAME\",\"project_name\":\"$PROJECT_NAME\"}"
zellij pipe --name "claude-pane:event" -- "$PAYLOAD" >/dev/null 2>&1 &
exit 0
