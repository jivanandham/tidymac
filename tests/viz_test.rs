//! Integration tests for the viz / storage visualization module.

#[test]
fn test_analyze_disk_usage_returns_data() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    // The root filesystem always has capacity > 0
    assert!(
        usage.total_capacity > 0,
        "Total disk capacity should be > 0"
    );
    assert_eq!(usage.mount_point, "/");
}

#[test]
fn test_disk_usage_used_leq_total() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    assert!(
        usage.used <= usage.total_capacity,
        "used ({}) should not exceed total_capacity ({})",
        usage.used,
        usage.total_capacity
    );
}

#[test]
fn test_disk_usage_available_leq_total() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    assert!(
        usage.available <= usage.total_capacity,
        "available ({}) should not exceed total_capacity ({})",
        usage.available,
        usage.total_capacity
    );
}

#[test]
fn test_categories_sorted_by_size_descending() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    let sizes: Vec<u64> = usage.categories.iter().map(|c| c.size).collect();
    let mut sorted = sizes.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(
        sizes, sorted,
        "Categories should be sorted by size descending"
    );
}

#[test]
fn test_every_category_has_non_empty_name() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    for cat in &usage.categories {
        assert!(!cat.name.is_empty(), "Category name should not be empty");
    }
}

#[test]
fn test_every_category_path_exists() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    for cat in &usage.categories {
        assert!(
            cat.path.exists(),
            "Category '{}' path {:?} should exist",
            cat.name,
            cat.path
        );
    }
}

#[test]
fn test_print_viz_does_not_panic() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    // Just ensure the rendering function doesn't panic
    tidymac::viz::storage::print_viz(&usage);
}

#[test]
fn test_print_viz_json_does_not_panic() {
    let usage = tidymac::viz::storage::analyze_disk_usage();
    tidymac::viz::storage::print_viz_json(&usage);
}
