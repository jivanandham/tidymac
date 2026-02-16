pub mod grouper;
pub mod hasher;
pub mod perceptual;
pub mod resolver;

pub use grouper::{find_duplicates, DupConfig, DupResults};
pub use perceptual::{MatchType, SimilarFile, SimilarGroup};
pub use resolver::{resolve_all, resolve_group, ResolveStrategy, ResolvedGroup};
