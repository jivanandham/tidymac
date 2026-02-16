pub mod detector;
pub mod uninstaller;

pub use detector::{discover_apps, find_associated_files, AppSource, AssociatedFile, AssociatedKind, InstalledApp};
pub use uninstaller::{find_app_by_name, uninstall_app, UninstallReport};
