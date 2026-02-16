//! # TidyMac
//!
//! A developer-aware, privacy-first Mac cleanup utility.
//!
//! TidyMac scans your Mac for junk files, developer caches, duplicates,
//! and leftover application data. It features:
//!
//! - **Developer-Aware Cleaning**: Xcode, Docker, node_modules, venv, Homebrew, and more
//! - **Safety-First**: Dry-run by default, soft-delete with 7-day undo window
//! - **Smart Profiles**: Quick, Developer, Creative, and Deep Clean presets
//! - **Perceptual Duplicate Detection**: Find visually similar images
//! - **CLI as Unix Citizen**: JSON output, pipe-friendly, cron-schedulable
//! - **Privacy Dashboard**: Audit browser cookies, tracking data, local databases
//! - **100% Offline**: Zero telemetry, no accounts, no cloud

pub mod cli;
pub mod cleaner;
pub mod common;
pub mod duplicates;
pub mod apps;
pub mod ffi;
pub mod startup;
pub mod privacy;
pub mod profiles;
pub mod scanner;
pub mod viz;
