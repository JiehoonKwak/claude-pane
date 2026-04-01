use crate::state::{
    format_elapsed, Activity, Config, FocusPayload, HookPayload, NotificationFlash, SessionInfo,
};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Activity::from_hook_event
// ---------------------------------------------------------------------------

#[test]
fn test_hook_event_to_activity_pretooluse_bash() {
    assert_eq!(
        Activity::from_hook_event("PreToolUse", Some("Bash")),
        Activity::BashExec
    );
}

#[test]
fn test_hook_event_to_activity_pretooluse_read() {
    assert_eq!(
        Activity::from_hook_event("PreToolUse", Some("Read")),
        Activity::Reading
    );
}

#[test]
fn test_hook_event_to_activity_pretooluse_write() {
    assert_eq!(
        Activity::from_hook_event("PreToolUse", Some("Write")),
        Activity::Writing
    );
}

#[test]
fn test_hook_event_to_activity_pretooluse_websearch() {
    assert_eq!(
        Activity::from_hook_event("PreToolUse", Some("WebSearch")),
        Activity::WebSearch
    );
}

#[test]
fn test_hook_event_to_activity_pretooluse_agent() {
    assert_eq!(
        Activity::from_hook_event("PreToolUse", Some("Agent")),
        Activity::Agent
    );
}

#[test]
fn test_hook_event_to_activity_pretooluse_mcp() {
    assert_eq!(
        Activity::from_hook_event("PreToolUse", Some("mcp__server__tool")),
        Activity::Mcp
    );
}

#[test]
fn test_hook_event_to_activity_stop() {
    assert_eq!(Activity::from_hook_event("Stop", None), Activity::Done);
}

#[test]
fn test_hook_event_to_activity_notification() {
    assert_eq!(
        Activity::from_hook_event("Notification", None),
        Activity::Notification
    );
}

#[test]
fn test_hook_event_to_activity_user_prompt() {
    assert_eq!(
        Activity::from_hook_event("UserPromptSubmit", None),
        Activity::Thinking
    );
}

#[test]
fn test_hook_event_to_activity_permission() {
    assert_eq!(
        Activity::from_hook_event("PermissionRequest", None),
        Activity::PermissionNeeded
    );
}

#[test]
fn test_hook_event_to_activity_unknown() {
    assert_eq!(
        Activity::from_hook_event("SomethingNew", None),
        Activity::Thinking
    );
}

// ---------------------------------------------------------------------------
// HookPayload deserialization
// ---------------------------------------------------------------------------

#[test]
fn test_hook_payload_full() {
    let json = r#"{"pane_id":5,"hook_event":"PreToolUse","tool_name":"Bash","project_name":"myproj"}"#;
    let p: HookPayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.pane_id, 5);
    assert_eq!(p.hook_event, "PreToolUse");
    assert_eq!(p.tool_name.as_deref(), Some("Bash"));
    assert_eq!(p.project_name.as_deref(), Some("myproj"));
}

#[test]
fn test_hook_payload_minimal() {
    let json = r#"{"pane_id":10,"hook_event":"Stop"}"#;
    let p: HookPayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.pane_id, 10);
    assert_eq!(p.hook_event, "Stop");
    assert!(p.tool_name.is_none());
    assert!(p.project_name.is_none());
}

#[test]
fn test_malformed_payload() {
    let json = r#"{"not_valid": true}"#;
    let result = serde_json::from_str::<HookPayload>(json);
    assert!(result.is_err());
}

#[test]
fn test_empty_payload() {
    let result = serde_json::from_str::<HookPayload>("");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// FocusPayload deserialization
// ---------------------------------------------------------------------------

#[test]
fn test_focus_payload_full() {
    let json = r##"{"pane_id":5,"flash_color":"#ff0000","flash_duration_ms":500}"##;
    let p: FocusPayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.pane_id, 5);
    assert_eq!(p.flash_color.as_deref(), Some("#ff0000"));
    assert_eq!(p.flash_duration_ms, Some(500));
}

#[test]
fn test_focus_payload_minimal() {
    let json = r#"{"pane_id":10}"#;
    let p: FocusPayload = serde_json::from_str(json).unwrap();
    assert_eq!(p.pane_id, 10);
    assert!(p.flash_color.is_none());
    assert!(p.flash_duration_ms.is_none());
}

#[test]
fn test_focus_payload_missing_pane_id() {
    let json = r##"{"flash_color":"#ff0000"}"##;
    assert!(serde_json::from_str::<FocusPayload>(json).is_err());
}

// ---------------------------------------------------------------------------
// Config defaults and parsing
// ---------------------------------------------------------------------------

#[test]
fn test_config_defaults() {
    let cfg = Config::from_map(&BTreeMap::new());
    assert_eq!(cfg.key_select_down, "j");
    assert_eq!(cfg.key_select_up, "k");
    assert_eq!(cfg.key_confirm, "Enter");
    assert_eq!(cfg.key_cancel, "Esc");
    assert_eq!(cfg.notification_flash, NotificationFlash::Persist);
    assert_eq!(cfg.flash_duration_ms, 2000);
    assert!((cfg.done_timeout_s - 30.0).abs() < f64::EPSILON);
    assert!((cfg.idle_remove_s - 300.0).abs() < f64::EPSILON);
    assert!(cfg.show_elapsed_time);
    assert!(cfg.show_non_claude);
}

#[test]
fn test_config_partial_override() {
    let mut map = BTreeMap::new();
    map.insert("done_timeout_s".into(), "2".into());
    map.insert("notification_flash".into(), "off".into());
    let cfg = Config::from_map(&map);
    assert!((cfg.done_timeout_s - 2.0).abs() < f64::EPSILON);
    assert_eq!(cfg.notification_flash, NotificationFlash::Off);
    assert_eq!(cfg.key_select_down, "j");
}

#[test]
fn test_config_invalid_value() {
    let mut map = BTreeMap::new();
    map.insert("done_timeout_s".into(), "not-a-number".into());
    map.insert("flash_duration_ms".into(), "abc".into());
    let cfg = Config::from_map(&map);
    assert!((cfg.done_timeout_s - 30.0).abs() < f64::EPSILON);
    assert_eq!(cfg.flash_duration_ms, 2000);
}

// ---------------------------------------------------------------------------
// Elapsed time formatting
// ---------------------------------------------------------------------------

#[test]
fn test_format_elapsed() {
    assert_eq!(format_elapsed(5.0), "5s");
    assert_eq!(format_elapsed(65.0), "1m");
    assert_eq!(format_elapsed(3700.0), "1h");
}

// ---------------------------------------------------------------------------
// Star operations (uses default(), no WASM)
// ---------------------------------------------------------------------------

#[test]
fn test_star_toggle() {
    let mut stars = crate::star::StarSet::default();
    assert!(!stars.contains(1));
    stars.toggle(1);
    assert!(stars.contains(1));
    stars.toggle(1);
    assert!(!stars.contains(1));
}

#[test]
fn test_star_cycle_wrap() {
    let mut stars = crate::star::StarSet::default();
    stars.toggle(10);
    stars.toggle(20);
    stars.toggle(30);

    let a = stars.next().unwrap();
    let b = stars.next().unwrap();
    let c = stars.next().unwrap();
    let d = stars.next().unwrap();
    assert_eq!(a, 10);
    assert_eq!(b, 20);
    assert_eq!(c, 30);
    assert_eq!(d, 10);
}

#[test]
fn test_star_prune_dead() {
    use std::collections::HashSet;
    let mut stars = crate::star::StarSet::default();
    stars.toggle(1);
    stars.toggle(2);
    stars.toggle(3);

    let live: HashSet<u32> = [1, 3].into_iter().collect();
    stars.prune(&live);
    assert!(stars.contains(1));
    assert!(!stars.contains(2));
    assert!(stars.contains(3));
    assert_eq!(stars.len(), 2);
}

#[test]
fn test_star_empty_cycle() {
    let mut stars = crate::star::StarSet::default();
    assert!(stars.next().is_none());
    assert!(stars.prev().is_none());
}

// ---------------------------------------------------------------------------
// Activity::is_running
// ---------------------------------------------------------------------------

#[test]
fn test_is_running_active() {
    assert!(Activity::Thinking.is_running());
    assert!(Activity::Reading.is_running());
    assert!(Activity::Writing.is_running());
    assert!(Activity::BashExec.is_running());
    assert!(Activity::WebSearch.is_running());
    assert!(Activity::Agent.is_running());
    assert!(Activity::Mcp.is_running());
    assert!(Activity::PermissionNeeded.is_running());
    assert!(Activity::Notification.is_running());
}

#[test]
fn test_is_running_inactive() {
    assert!(!Activity::Done.is_running());
    assert!(!Activity::Idle.is_running());
}

// ---------------------------------------------------------------------------
// Staleness detection
// ---------------------------------------------------------------------------

fn make_session(activity: Activity, last_event_ts: f64) -> SessionInfo {
    SessionInfo {
        activity,
        tab_index: None,
        tab_name: None,
        last_event_ts,
        project_name: None,
        tool_name: None,
        flash_deadline: 0.0,
        focus_highlight_deadline: 0.0,
    }
}

#[test]
fn test_stale_running_session_transitions_to_done() {
    // Timer-based: session with no hook events for idle_remove_s transitions to Done
    let session = make_session(Activity::Thinking, 0.0);
    let now = 400.0; // well past idle_remove_s (300s default)
    let idle_remove = 300.0;
    assert!(
        session.activity.is_running()
            && !session.activity.is_attention()
            && (now - session.last_event_ts) > idle_remove
    );
}

#[test]
fn test_recent_running_session_stays_running() {
    // Session with recent events should NOT transition
    let session = make_session(Activity::BashExec, 200.0);
    let now = 400.0; // 200s since last event — under idle_remove_s (300s)
    let idle_remove = 300.0;
    assert!(
        !(session.activity.is_running()
            && !session.activity.is_attention()
            && (now - session.last_event_ts) > idle_remove)
    );
}

#[test]
fn test_attention_session_not_stale() {
    // PermissionNeeded should NOT be auto-transitioned even if stale
    let session = make_session(Activity::PermissionNeeded, 0.0);
    let now = 120.0;
    let done_timeout = 30.0;
    assert!(
        !(session.activity.is_running()
            && !session.activity.is_attention()
            && (now - session.last_event_ts) > done_timeout * 2.0)
    );
}

// ---------------------------------------------------------------------------
// Codex process detection
// ---------------------------------------------------------------------------

#[test]
fn test_codex_process_detection() {
    assert!("codex".contains("codex"));
    assert!(!"claude".contains("codex"));
    assert!(!"zsh".contains("codex"));
    assert!("codex-cli".contains("codex"));
}
