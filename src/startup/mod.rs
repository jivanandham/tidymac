pub mod manager;

pub use manager::{
    disable_item, discover_startup_items, enable_item, find_item_by_name,
    StartupItem, StartupKind,
};
