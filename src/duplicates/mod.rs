pub mod grouper;
pub mod hasher;
pub mod perceptual;
pub mod resolver;

pub use grouper::{find_duplicates, DupConfig, DupResults};
pub use perceptual::{MatchType, SimilarFile, SimilarGroup};
pub use resolver::{resolve_all, resolve_group, ResolveStrategy, ResolvedGroup};

/// JSON representation of duplicate results (shared by the CLI and the GUI).
pub fn results_to_json(results: &DupResults) -> serde_json::Value {
    let group_json = |g: &SimilarGroup| {
        serde_json::json!({
            "match_type": format!("{}", g.match_type),
            "wasted_bytes": g.wasted_bytes,
            "wasted_formatted": crate::common::format::format_size(g.wasted_bytes),
            "members": g.members.iter().map(|m| {
                serde_json::json!({
                    "path": m.path.display().to_string(),
                    "size_bytes": m.size_bytes,
                    "size_formatted": crate::common::format::format_size(m.size_bytes),
                    "similarity": m.similarity,
                })
            }).collect::<Vec<_>>(),
        })
    };
    serde_json::json!({
        "files_scanned": results.files_scanned,
        "duration_secs": results.duration_secs,
        "total_groups": results.total_groups,
        "total_wasted": results.total_wasted,
        "total_wasted_formatted": crate::common::format::format_size(results.total_wasted),
        "total_duplicates": results.total_duplicates,
        "exact_groups": results.exact_groups.iter().map(group_json).collect::<Vec<_>>(),
        "similar_groups": results.similar_groups.iter().map(group_json).collect::<Vec<_>>(),
        "errors": results.errors,
    })
}
