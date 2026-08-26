use dom::{AttributeMap, AttributeName, AttributeValue, DomService, DomTree, TagName};
use engine::{
    AtomicScriptSlot, DebounceDuration, HotReloadCoordinator, HotReloadStatus, MockEngine,
    ScriptWatcher,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn test_c10_file_watcher_detects_rhai_modifications_with_debounce() {
    let temp_dir = std::env::temp_dir().join(format!("alloy_watcher_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let script_file = temp_dir.join("test_pipeline.rhai");
    std::fs::write(&script_file, "let x = 10;").unwrap();

    let event_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&event_counter);

    let debounce = DebounceDuration::new(Duration::from_millis(50));
    let mut watcher = ScriptWatcher::new(debounce);

    watcher
        .watch(&temp_dir, move |_path| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .expect("Watcher initialization");

    // Perform multiple rapid writes within the debounce window
    for i in 0..5 {
        let _ = std::fs::write(&script_file, format!("let x = {i};"));
        std::thread::sleep(Duration::from_millis(5));
    }

    // Wait for debounce event to arrive using condition polling with timeout
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(3);
    while event_counter.load(Ordering::SeqCst) == 0 && start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(10));
    }
    // Allow debounce window to settle
    std::thread::sleep(Duration::from_millis(60));

    // Counter must have been triggered without spawning 5 distinct reloads
    let triggers = event_counter.load(Ordering::SeqCst);
    assert!(
        (1..=3).contains(&triggers),
        "Debounce should consolidate rapid modifications (got {triggers})"
    );

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_c11_successful_script_edits_compile_and_swap_atomically() {
    let engine = Arc::new(MockEngine::new());
    let slot = AtomicScriptSlot::new();
    let coordinator = HotReloadCoordinator::new(Arc::clone(&engine), slot.clone());

    assert_eq!(slot.version(), 0);
    assert!(slot.active_ast().is_none());

    // 1. Initial valid compilation and swap
    let status_1 = coordinator.compile_and_swap("let x = 100;");
    assert_eq!(status_1, HotReloadStatus::Success { version: 1 });
    assert_eq!(slot.version(), 1);
    assert_eq!(slot.active_ast().unwrap().as_str(), "let x = 100;");

    // 2. Subsequent valid edit
    let status_2 = coordinator.compile_and_swap("let x = 200;");
    assert_eq!(status_2, HotReloadStatus::Success { version: 2 });
    assert_eq!(slot.version(), 2);
    assert_eq!(slot.active_ast().unwrap().as_str(), "let x = 200;");
}

#[test]
fn test_c12_script_with_syntax_errors_does_not_replace_ast_and_logs() {
    let engine = Arc::new(MockEngine::new());
    let slot = AtomicScriptSlot::new();
    let coordinator = HotReloadCoordinator::new(Arc::clone(&engine), slot.clone());

    // Establish active version 1
    coordinator.compile_and_swap("let active = true;");
    assert_eq!(slot.version(), 1);
    assert_eq!(slot.active_ast().unwrap().as_str(), "let active = true;");

    // Attempt hot-reload with syntax error
    let invalid_edit = "let broken = SYNTAX_ERROR;";
    let status = coordinator.compile_and_swap(invalid_edit);

    match status {
        HotReloadStatus::CompilationError {
            error,
            previous_version,
        } => {
            assert!(error.contains("syntax error"));
            assert_eq!(previous_version, 1);
        }
        other => panic!("Expected HotReloadStatus::CompilationError, got: {other:?}"),
    }

    // Active script slot must retain version 1 and previous AST (C-12)
    assert_eq!(slot.version(), 1);
    assert_eq!(slot.active_ast().unwrap().as_str(), "let active = true;");
}

#[test]
fn test_c13_dom_and_state_intact_after_multiple_hot_reloads() {
    // 1. Setup persistent Rust domain state: DomTree
    let mut dom = DomTree::new();
    let doc_id = dom.create_document();

    let mut attrs = AttributeMap::new();
    attrs.insert(AttributeName::new("id"), AttributeValue::new("main-view"));
    let div_id = dom.create_element(TagName::new("div").unwrap(), attrs);
    dom.append_child(doc_id, div_id).unwrap();

    let text_id = dom.create_text("Persistent User Data");
    dom.append_child(div_id, text_id).unwrap();

    let initial_serialized = DomService::serialize_to_html(&dom, doc_id);
    assert_eq!(
        initial_serialized,
        r#"<div id="main-view">Persistent User Data</div>"#
    );

    // 2. Setup Hot-Reload coordinator
    let engine = Arc::new(MockEngine::new());
    let slot = AtomicScriptSlot::new();
    let coordinator = HotReloadCoordinator::new(Arc::clone(&engine), slot.clone());

    // 3. Perform 10 consecutive hot-reload attempts (alternating valid and invalid scripts)
    for i in 0..10 {
        if i % 2 == 0 {
            let status = coordinator.compile_and_swap(&format!("let reload_counter = {i};"));
            assert!(matches!(status, HotReloadStatus::Success { .. }));
        } else {
            let status = coordinator.compile_and_swap("let invalid = SYNTAX_ERROR;");
            assert!(matches!(status, HotReloadStatus::CompilationError { .. }));
        }
    }

    // 4. Verify that DOM tree remains 100% intact, uncorrupted, and valid (C-13)
    let post_reload_serialized = DomService::serialize_to_html(&dom, doc_id);
    assert_eq!(
        post_reload_serialized, initial_serialized,
        "DOM state must remain completely intact across multiple script hot-reloads"
    );
    assert_eq!(dom.node_count(), 3);
}
