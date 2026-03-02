//! Integration tests for the startup items manager module.

#[test]
fn test_startup_kind_display() {
    use tidymac::startup::manager::StartupKind;
    assert_eq!(StartupKind::UserLaunchAgent.to_string(), "User Agent");
    assert_eq!(StartupKind::SystemLaunchAgent.to_string(), "System Agent");
    assert_eq!(StartupKind::SystemLaunchDaemon.to_string(), "System Daemon");
}

#[test]
fn test_discover_startup_items_returns_vec() {
    // Smoke test: should never panic
    let items = tidymac::startup::manager::discover_startup_items();
    // Validate basic invariants for any discovered items
    for item in &items {
        assert!(
            !item.name.is_empty(),
            "Startup item name should not be empty"
        );
        assert!(
            !item.label.is_empty(),
            "Startup item label should not be empty"
        );
        assert!(
            item.path.exists(),
            "Plist path should exist: {:?}",
            item.path
        );
    }
}

#[test]
fn test_startup_items_sorted_by_name() {
    let items = tidymac::startup::manager::discover_startup_items();
    // discover_startup_items() is documented to return items sorted by name
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "Startup items should be sorted by name");
}

#[test]
fn test_find_item_by_name_empty_query() {
    let items = tidymac::startup::manager::discover_startup_items();
    // An empty string matches everything
    let found = tidymac::startup::manager::find_item_by_name(&items, "");
    assert_eq!(
        found.len(),
        items.len(),
        "Empty query should match all items"
    );
}

#[test]
fn test_find_item_by_name_no_match() {
    let items = tidymac::startup::manager::discover_startup_items();
    let found =
        tidymac::startup::manager::find_item_by_name(&items, "__totally_nonexistent_label_zyxwv__");
    assert!(
        found.is_empty(),
        "Should find no items for a non-existent label"
    );
}

#[test]
fn test_find_item_by_name_case_insensitive() {
    let items = tidymac::startup::manager::discover_startup_items();
    if let Some(first) = items.first() {
        let upper = first.name.to_uppercase();
        let lower = first.name.to_lowercase();
        let found_upper = tidymac::startup::manager::find_item_by_name(&items, &upper);
        let found_lower = tidymac::startup::manager::find_item_by_name(&items, &lower);
        // Both queries should find at least the same item
        assert!(!found_upper.is_empty());
        assert!(!found_lower.is_empty());
    }
}
