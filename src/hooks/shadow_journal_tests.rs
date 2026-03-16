use super::*;
use std::path::Path;

fn make_hook(dir: &Path) -> ShadowJournalHook {
    ShadowJournalHook::new(dir.to_path_buf(), "test-session".to_string())
}

fn edit_event(path: &str) -> ToolEvent {
    ToolEvent {
        tool_name: "Edit".to_string(),
        input_summary: format!("modified {path}"),
        outcome: "success".to_string(),
        file_paths: vec![path.to_string()],
        decision_note: None,
    }
}

#[test]
fn record_event_zero_io() {
    let tmp = tempfile::tempdir().unwrap();
    let mut hook = make_hook(tmp.path());
    hook.record_tool_event(edit_event("src/main.rs"));
    assert_eq!(hook.entries.len(), 1);
    let files: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(files.is_empty(), "record_tool_event must not create files");
}

#[test]
fn max_entries_drops_excess() {
    let tmp = tempfile::tempdir().unwrap();
    let mut hook = make_hook(tmp.path());
    hook.config.max_entries = 3;
    for i in 0..5 {
        hook.record_tool_event(edit_event(&format!("f{i}.rs")));
    }
    assert_eq!(hook.entries.len(), 3);
    assert_eq!(hook.dropped_events, 2);
}

#[test]
fn disabled_hook_does_not_accumulate() {
    let tmp = tempfile::tempdir().unwrap();
    let mut hook = make_hook(tmp.path());
    hook.config.enabled = false;
    hook.record_tool_event(edit_event("src/lib.rs"));
    assert!(hook.entries.is_empty());
}

#[test]
fn render_contains_required_sections() {
    let tmp = tempfile::tempdir().unwrap();
    let mut hook = make_hook(tmp.path());
    hook.record_tool_event(edit_event("src/main.rs"));
    let md = hook.render_journal_md();
    assert!(md.contains("## Decisões"), "must have Decisões section");
    assert!(md.contains("## Arquivos Modificados"), "must have Arquivos section");
    assert!(md.contains("## Eventos Detalhados"), "must have Eventos section");
}
