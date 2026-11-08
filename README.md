<div align="center">

# 🧹 TidyMac

### Developer-Aware, Privacy-First macOS Cleanup & Storage Intelligence

**Reclaim 20–50 GB of hidden developer cache space with mathematical safety, APFS awareness, and a native SwiftUI interface.**

[![CI](https://github.com/jivanandham/tidymac/actions/workflows/ci.yml/badge.svg)](https://github.com/jivanandham/tidymac/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Homebrew](https://img.shields.io/badge/brew_install-tidymac-FBB040?logo=homebrew&logoColor=white)](https://github.com/jivanandham/homebrew-tidymac)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Swift](https://img.shields.io/badge/Swift-5.9%2B-F05138?logo=swift&logoColor=white)](https://swift.org)
[![macOS](https://img.shields.io/badge/macOS-14%2B_Sonoma-000000?logo=apple&logoColor=white)](https://www.apple.com/macos/)

<br>

```
┌────────────────────────────────────────────────────────┐
│  $ tidymac scan --profile developer                    │
│                                                        │
│  🔍 Scanning developer caches & build artifacts...     │
│  ████████████████████████████████████░░  88%           │
│                                                        │
│  📦 Xcode DerivedData                  12.3 GB         │
│  🐳 Docker reclaimable (prune)          8.1 GB         │
│  📁 node_modules (stale projects)       5.7 GB         │
│  🐍 Python virtualenvs                  3.2 GB         │
│  🦀 Cargo target directories            2.8 GB         │
│  🍺 Homebrew download cache             1.4 GB         │
│  ─────────────────────────────────────────────         │
│  💾 Total Reclaimable:                 33.5 GB         │
└────────────────────────────────────────────────────────┘
```

</div>

---

## ⚡ Quick Install

### Homebrew (Recommended)

```bash
brew tap jivanandham/tidymac && brew install tidymac
```

### Download DMG Installer

Download the latest signed release from [Releases](https://github.com/jivanandham/tidymac/releases) or build it locally:
```bash
git clone https://github.com/jivanandham/tidymac.git
cd tidymac && ./scripts/build-dmg.sh
# Creates build/TidyMac.dmg
```

<details>
<summary>📦 Build from source</summary>

#### Build CLI (Rust)
```bash
git clone https://github.com/jivanandham/tidymac.git
cd tidymac
cargo build --release
sudo cp target/release/tidymac /usr/local/bin/
```

#### Run SwiftUI App (Xcode / SwiftPM)
```bash
cd tidymac
./scripts/build-ffi.sh
cd TidyMac && swift run TidyMacApp
```

</details>

---

## 💡 Why TidyMac?

> **Standard cleaners treat every Mac like an office laptop.**  
> They wipe browser cookies and user caches, but leave tens of gigabytes of stale build artifacts, unlinked package trees, orphaned containers, and simulator runtimes untouched.

<table>
<tr>
<td width="50%">

### 🎯 Developer-Aware
- **14+ Developer Ecosystems**: Xcode DerivedData, iOS Simulators (via `xcrun simctl`), Android SDK/NDK, Cargo targets, Gradle, Maven, sbt, pip, poetry, conda, Go module caches, Homebrew, and Docker.
- **Smart Stale Project Detection**: Distinguishes active workspaces from abandoned projects.
- **System Data Explainer**: Uncovers what constitutes macOS "System Data" (snapshots, Xcode caches, swap, sleep images).

</td>
<td width="50%">

### 🛡️ Ironclad Safety
- **Single Deletion Engine**: Every clean, uninstall, and privacy wipe goes through a single safety pipeline with canonical path validation.
- **Root-Safe & Non-Destructive**: Never deletes a target folder's root (e.g., `~/Library/Caches`). Only purges verified children.
- **Protected Paths**: System roots, Keychains, iCloud Drive, Mail, Messages, Safari, Photos, and `.ssh` are permanently blocked.
- **7-Day Undo Window**: Soft delete moves files to `~/.tidymac/staging` before permanent purge.
- **100% Offline & Private**: Zero network calls, zero analytics, zero telemetry.

</td>
</tr>
</table>

---

## 📊 Feature Comparison

| Capability | TidyMac | CleanMyMac X / 5 | DaisyDisk | AppCleaner |
| :--- | :---: | :---: | :---: | :---: |
| **Developer Caches (Xcode, Simulators, Cargo, Go, npm)** | ✅ Built-in (14+) | ⚠️ Basic Xcode only | ❌ Manual | ❌ |
| **Combined Native GUI (SwiftUI) & Scriptable CLI** | ✅ Single engine | ❌ GUI only | ❌ GUI only | ❌ GUI only |
| **Safety Guardrails & Blocklists (Keychains, Mail, Cloud)** | ✅ Strict root-scoped | ⚠️ Partial | ❌ | ⚠️ Partial |
| **7-Day Soft-Delete & Staging Undo** | ✅ `tidymac undo` | ⚠️ Trash only | ❌ | ⚠️ Trash only |
| **APFS Clone-Aware Accurate Sizing** | ✅ | ❌ | ❌ | ❌ |
| **Visual Treemap / Disk Map** | ✅ Drill-down | ⚠️ Bubble graph | ✅ Starburst | ❌ |
| **Orphaned App Leftovers Detection** | ✅ Receipt-aware | ✅ | ❌ | ⚠️ Interactive only |
| **Zero Telemetry / 100% Offline** | ✅ Open source | ❌ Cloud telemetry | ❌ Tracking | ✅ |
| **License & Price** | **Free (MIT)** | ~$40 / yr | $9.99 | Free (Closed) |

---

## 🖥️ Native macOS App (SwiftUI)

TidyMac provides a high-performance native macOS interface connected directly to the Rust core via zero-overhead C FFI bindings:

| View | Purpose |
| :--- | :--- |
| 📊 **Dashboard** | Storage overview, quick stats, and one-click maintenance triggers |
| 🔍 **Scan & Clean** | Categorized review of system, developer, and application caches with per-item toggles |
| 🗺️ **Disk Map** | Interactive APFS-aware squarified treemap with drill-down exploration |
| 👥 **Duplicates** | 4-pass duplicate scanner with perceptual image similarity matching |
| 📱 **App Uninstaller** | Comprehensive application footprint review and clean uninstaller |
| 🧩 **Leftovers** | Detect and reclaim residue from previously uninstalled applications |
| 🐳 **Docker** | Safe daemon-integrated disk usage audit and pruning |
| 🔒 **Privacy** | Cookie, local storage, history, and cache cleaner for 8 major browsers |
| 📜 **History & Undo** | Review prior cleaning sessions with instant single-click restoration |
| ⚙️ **Settings** | Safety thresholds, custom profiles, scheduled runs, and exclusion lists |

---

## ⌨️ Command Line Interface

### 🔍 Scanning
```bash
# Standard scan across user caches and temporary data
tidymac scan

# Scan with developer profile (Xcode, Docker, node_modules, etc.)
tidymac scan --profile developer

# Deep scan with individual file paths
tidymac scan --profile deep --detailed

# Output raw JSON for automation or scripting
tidymac scan --format json
```

### 🧹 Cleaning & Recovery
```bash
# Dry run: preview what would be cleaned without modifying anything
tidymac clean --profile developer --dry-run

# Clean with 7-day undo staging safety net (default)
tidymac clean --profile developer

# Hard clean (permanently delete immediately)
tidymac clean --profile quick --hard

# Restore the most recent cleanup session
tidymac undo --last

# List recovery sessions or restore a specific session
tidymac undo --list
tidymac undo --session <session-id>
```

### 🗺️ Storage Analysis & Treemap
```bash
# Largest directories in your home folder
tidymac map

# Drill down into a specific directory up to 3 levels deep
tidymac map ~/Library --depth 3 --top 20

# Explain hidden macOS "System Data" components
tidymac system-data
```

### 👥 Duplicate Detection
```bash
# Find exact duplicate files (fast 3-pass hash)
tidymac dup ~/Documents

# Find visually similar photos using perceptual image hashing
tidymac dup ~/Pictures --perceptual

# Detailed duplicate report with paths
tidymac dup ~/Downloads --detailed
```

### 📱 Applications & Leftovers
```bash
# List installed applications sorted by total footprint
tidymac apps list --sort size

# Inspect an application's associated containers, caches, and preferences
tidymac apps info "Slack"

# Completely remove an application and its associated files
tidymac apps remove "Slack" --dry-run
tidymac apps remove com.tinyspeck.slackmacgap

# Discover residue left behind by uninstalled applications
tidymac apps orphans

# Safely clean orphaned leftover files
tidymac apps orphans --clean com.example.oldapp
```

### 🔒 Privacy Audit
```bash
# Scan browser privacy data across 8 major browsers
tidymac privacy scan

# Preview privacy data cleanup (cookies, cache, history, trackers)
tidymac privacy clean --dry-run
```
*Supported browsers: Chrome, Safari, Firefox, Arc, Brave, Edge, Opera, and Vivaldi.*

### ⏰ Scheduling & Maintenance
```bash
# Install a weekly dry-run report via user launchd
tidymac schedule install --profile developer --every weekly

# Check scheduled report status
tidymac schedule status

# Remove scheduled background runs
tidymac schedule remove

# Purge expired items from the 7-day staging area
tidymac purge --expired
```

---

## 🎛️ Cleanup Profiles

TidyMac comes with four built-in profiles and supports fully customized user rules:

| Profile | Level | Scope | Typical Use Case |
| :--- | :---: | :--- | :--- |
| 🟢 `quick` | Conservative | User caches, system logs, trash, `$TMPDIR` | Daily upkeep |
| 🟡 `developer` | Balanced | Dev tool caches, DerivedData, build directories, Docker | Post-coding sessions |
| 🟠 `creative` | Balanced | Audio/video scratch disks, render caches, image previews | Media editing workflows |
| 🔴 `deep` | Thorough | Complete scan including large files and unlinked runtimes | Monthly deep recovery |

Custom profile definitions can be placed in `~/.tidymac/profiles/<name>.toml`:
```toml
name = "custom"
description = "Personal workflow rules"

[[rules]]
path = "~/Downloads"
min_age_days = 30
min_size_mb = 100
extensions = ["dmg", "pkg", "iso"]
```

---

## 🏗️ Architecture & Internals

```
┌────────────────────────────────────────────────────────┐
│               Native SwiftUI Frontend                  │
│   Dashboard │ Scan │ Map │ Duplicates │ Apps │ Privacy │
├────────────────────────────────────────────────────────┤
│                 TidyMacBridge.swift                    │  ← Swift ↔ C FFI
├────────────────────────────────────────────────────────┤
│               tidymac.h / module.modulemap             │  ← C Declarations
├────────────────────────────────────────────────────────┤
│                 libtidymac.dylib                       │  ← Compiled Rust Core
├────────────────────────────────────────────────────────┤
│                    Rust Core Engine                    │
│   • scanner (Walker, DevDetector, Targets, Caching)    │
│   • cleaner (Deleter, Staging, Manifest, Safety)       │
│   • duplicates (SizeGroup, FastHash, SHA-256, pHash)   │
│   • apps (Detector, Receipts, Uninstaller, Orphans)    │
│   • viz (Treemap, APFS Clones, System Data)            │
│   • privacy (Browsers, Cookies, Storage, Cleaner)      │
└────────────────────────────────────────────────────────┘
```

### Project Structure

```
tidymac/
├── Cargo.toml               # Rust dependencies and configuration
├── src/                     # Core Rust implementation
│   ├── main.rs              # CLI entry point and subcommands
│   ├── lib.rs               # Library root
│   ├── ffi.rs               # C FFI exports for SwiftUI
│   ├── apps/                # App discovery, receipts, uninstaller, orphans
│   ├── cleaner/             # Safety-checked deleter, staging, purge, undo
│   ├── cli/                 # Argument parsing and formatting
│   ├── common/              # APFS helpers, permissions, config, safety guard
│   ├── duplicates/          # 4-pass duplicate & perceptual image matching
│   ├── privacy/             # Multi-browser privacy scanner & cleaner
│   ├── profiles/            # Built-in and custom profile parsers
│   ├── scanner/             # Walker, developer detection, target catalog
│   ├── startup/             # Login item manager
│   └── viz/                 # Squarified treemap & System Data inspector
├── TidyMac/                 # Native SwiftUI Application
│   ├── Package.swift        # Swift package manifest
│   ├── Sources/             # SwiftUI views, view models, and bridge
│   ├── Libraries/           # FFI dylib, C headers, and modulemap
│   └── Tests/               # Swift test suite
├── ffi/                     # C header definitions
├── profiles/                # Default profile presets
├── scripts/                 # Build, FFI, and DMG packaging scripts
└── tests/                   # Rust integration tests
```

---

## 🛠️ Development & Testing

### Running Tests
```bash
# Run Rust unit and integration test suite
cargo test

# Run Swift bridge and model tests
cd TidyMac && swift test
```

### Full DMG Build Pipeline
```bash
# Builds native Rust FFI dylib, compiles Swift app, code signs, and produces DMG
./scripts/build-dmg.sh
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE). Built with ❤️ for the macOS developer community.
