pub mod deleter;
pub mod engine;
pub mod manifest;
pub mod purger;
pub mod staging;

pub use deleter::{execute, CleanMode, CleanReport, DeleteOptions, DeleteRequest};
pub use engine::{check_staging_health, clean, clean_with, CleanOptions, StagingHealth};
pub use manifest::{CleanManifest, ManifestItem, SessionSummary, SkippedEntry};
pub use purger::{purge_all, purge_expired, purge_session, PurgeReport};
pub use staging::{restore_session, RestoreReport};
