pub mod detector;
pub mod orphans;
pub mod receipts;
pub mod uninstaller;

pub use detector::{
    discover_apps, find_associated_files, AppSource, AssociatedFile, AssociatedKind, InstalledApp, MatchKind,
};
pub use orphans::{find_orphans, OrphanGroup};
pub use receipts::{install_info, InstallInfo, PackageReceipt};
pub use uninstaller::{
    find_app_by_name, resolve_app, uninstall_app, uninstall_app_with, AppLookup, UninstallOptions, UninstallReport,
};
