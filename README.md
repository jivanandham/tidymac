# 🧹 TidyMac

**A developer-aware, privacy-first Mac cleanup utility built in Rust with a native SwiftUI interface.**

[![CI](https://github.com/jivanandham/tidymac/actions/workflows/ci.yml/badge.svg)](https://github.com/jivanandham/tidymac/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Swift](https://img.shields.io/badge/Swift-5.9%2B-F05138.svg)](https://swift.org)
[![macOS](https://img.shields.io/badge/macOS-14%2B-000000.svg)](https://www.apple.com/macos/)

---

## Why TidyMac?

Mainstream cleaners like CleanMyMac X, OnyX, and AppCleaner treat every Mac the same. They don't understand developer workflows — and that's where **20–50GB of reclaimable space** hides.

TidyMac was built for developers. It intelligently detects and cleans **14+ developer tool caches** (Xcode DerivedData, Docker images, `node_modules`, Python venvs, Homebrew, pip, Cargo, CocoaPods, Gradle, and more), while providing safety features no other cleaner offers.

### Key Differentiators

| Feature | TidyMac | CleanMyMac X | AppCleaner |
|---------|---------|-------------|------------|
| Developer cache detection | ✅ 14+ tools | ❌ | ❌ |
| Native SwiftUI GUI | ✅ | N/A | N/A |
| Dry-run before cleaning | ✅ | ❌ | ❌ |
| Undo with 7-day recovery | ✅ | ❌ | ❌ |
| Smart profiles | ✅ 4 presets | ❌ | ❌ |
| Perceptual image dedup | ✅ | ❌ | ❌ |
| CLI with JSON output | ✅ | ❌ | ❌ |
| Privacy audit | ✅ | Partial | ❌ |
| 100% offline, zero telemetry | ✅ | ❌ | ✅ |
| Free & open source | ✅ | $40/yr | ✅ |

---

## Quick Start

### Option 1: CLI (Rust)

```bash
# Clone & build
git clone https://github.com/jivanandham/tidymac.git
cd tidymac
cargo build --release

# Install to PATH
cp target/release/tidymac /usr/local/bin/

# Initialize & scan
tidymac config init
tidymac scan --profile developer
```

### Option 2: Native macOS App (SwiftUI)

```bash
# Clone & build the Rust FFI library first
git clone https://github.com/jivanandham/tidymac.git
cd tidymac
cargo build --release

# Build the FFI bridge (copies dylib + headers)
./scripts/build-ffi.sh

# Run the SwiftUI app
cd TidyMac
swift run TidyMacApp
```

> **Note:** The SwiftUI app requires macOS 14+ (Sonoma) and Swift 5.9+.

---

## SwiftUI App

TidyMac includes a **native macOS SwiftUI application** that wraps the Rust core engine via C FFI bindings.

### App Views

| View | Description |
|------|-------------|
| **Dashboard** | Overview of disk usage and quick actions |
| **Scan** | Run and visualize scan results |
| **Apps** | Browse installed apps and their footprint |
| **Docker** | Manage Docker images, containers, and volumes |
| **Privacy** | Audit browser cookies, history, and trackers |
| **History** | Review past cleanup sessions with undo |
| **Settings** | Configure profiles, thresholds, and preferences |

### Architecture

The SwiftUI app communicates with the Rust backend through a C FFI bridge:

```
┌──────────────────────────────┐
│     SwiftUI App (TidyMac)    │
│  Dashboard │ Scan │ Apps │…  │
├──────────────────────────────┤
│     TidyMacBridge.swift      │  ← Swift ↔ C bridge
├──────────────────────────────┤
│    TidyMacFFI (C headers)    │  ← module.modulemap + tidymac.h
├──────────────────────────────┤
│   libtidymac.dylib (Rust)    │  ← Compiled Rust core
├──────────────────────────────┤
│       Rust Core Engine       │
│  Scanner │ Cleaner │ Hasher  │
│  Privacy │ Profiles │ Apps   │
└──────────────────────────────┘
```

---

## CLI Usage

### Scanning

```bash
tidymac scan                              # Quick scan (caches, temp, trash)
tidymac scan --profile developer          # Developer-focused scan
tidymac scan --profile deep --detailed    # Deep scan with file paths
tidymac scan --format json                # JSON output for scripting
```

### Cleaning

```bash
tidymac clean --profile developer         # Soft delete (7-day undo window)
tidymac clean --profile quick --hard      # Permanent deletion
tidymac clean --dry-run                   # Preview what would be cleaned
```

### Undo

```bash
tidymac undo --last                       # Restore last cleanup
tidymac undo --list                       # View all recovery sessions
tidymac undo --session 2026-02-15T19-30-00
```

### Duplicate Detection

```bash
tidymac dup ~/Documents                   # Find exact duplicates
tidymac dup ~/Pictures --perceptual       # Find visually similar photos
tidymac dup ~/Downloads --detailed        # Show file paths per group
```

The duplicate finder uses a **4-pass pipeline** for speed:

```
Pass 1: Group by file size ──► eliminates ~95% instantly
Pass 2: Quick hash (4KB)   ──► eliminates ~4% more
Pass 3: Full SHA-256       ──► confirms exact duplicates
Pass 4: Perceptual hash    ──► finds visually similar images (optional)
```

### App Management

```bash
tidymac apps list --sort size             # List apps by total footprint
tidymac apps info Slack                   # See app + all associated files
tidymac apps remove Slack --dry-run       # Preview complete removal
```

### Startup Items

```bash
tidymac startup list                      # Show all login/startup items
tidymac startup disable com.docker.helper # Disable a startup item
tidymac startup enable com.docker.helper  # Re-enable it
```

### Privacy Audit

```bash
tidymac privacy scan                      # Full privacy audit
tidymac privacy scan --browsers           # Browser data only
tidymac privacy clean --dry-run           # Preview privacy cleanup
```

Detects and audits: Chrome, Firefox, Safari, Brave, Edge, Arc, Vivaldi, Opera — cookies, history, local storage, cache, and tracking data.

### Storage Visualization

```bash
tidymac viz                               # Visual disk usage breakdown
tidymac viz --format json                 # JSON for dashboards
```

### Staging Management

```bash
tidymac purge --expired                   # Free expired staging space
tidymac purge --install-auto              # Auto-purge daily at 3am
tidymac status                            # Overview of everything
```

---

## Smart Profiles

| Profile | Aggression | Targets | Use Case |
|---------|-----------|---------|----------|
| `quick` | Low | Caches, temp, trash | Daily maintenance |
| `developer` | Medium | + All dev caches (14 tools) | After coding sessions |
| `creative` | Medium | + Render files, previews | After creative projects |
| `deep` | High | Everything + large files | Monthly deep clean |

Create custom profiles in `~/.tidymac/profiles/custom.toml`.

---

## Project Structure

```
tidymac/
├── Cargo.toml               # Rust package manifest
├── src/                     # Rust core engine
│   ├── main.rs              # CLI entry point & command routing
│   ├── lib.rs               # Library root (FFI-ready for SwiftUI)
│   ├── ffi.rs               # C FFI exports
│   ├── cli/                 # Argument parsing & output formatting
│   ├── scanner/             # File system scanning engine
│   ├── cleaner/             # Cleaning with staging & undo
│   ├── duplicates/          # Duplicate detection (SHA-256 + perceptual)
│   ├── apps/                # App uninstaller
│   ├── startup/             # Startup items manager
│   ├── privacy/             # Privacy dashboard
│   ├── viz/                 # Storage visualization
│   ├── profiles/            # Smart profile system
│   └── common/              # Shared utilities & safety guards
├── ffi/                     # C header for FFI bridge
│   └── tidymac.h
├── TidyMac/                 # SwiftUI macOS app
│   ├── Package.swift        # Swift Package Manager manifest
│   ├── Sources/             # Swift source files
│   │   ├── TidyMacApp.swift # App entry point
│   │   ├── ContentView.swift
│   │   ├── Bridge/          # Rust FFI bridge layer
│   │   ├── ViewModels/      # App state management
│   │   └── Views/           # SwiftUI views (7 screens)
│   ├── Libraries/           # FFI artifacts (dylib, header, modulemap)
│   ├── Info.plist
│   └── TidyMac.entitlements
├── profiles/                # Built-in cleanup profiles (TOML)
├── scripts/                 # Build & release scripts
│   ├── build-ffi.sh         # Build Rust → Swift FFI bridge
│   ├── build-dmg.sh         # Package as .dmg installer
│   ├── release.sh           # Full release pipeline
│   └── install.sh           # CLI installation script
├── build/                   # Pre-built app bundle & DMG
├── tests/                   # Integration & unit tests
└── .github/workflows/       # CI/CD pipelines
```

---

## Building

### Prerequisites

- **macOS 14+** (Sonoma or later)
- **Rust 1.70+** — [Install via rustup](https://rustup.rs)
- **Swift 5.9+** — Included with Xcode 15+
- **Full Disk Access** recommended (for scanning Mail, Safari data)

### Build CLI Only

```bash
cargo build --release
```

### Build SwiftUI App

```bash
# 1. Build Rust library
cargo build --release

# 2. Generate FFI artifacts
./scripts/build-ffi.sh

# 3. Build & run the app
cd TidyMac && swift run TidyMacApp
```

### Build DMG Installer

```bash
./scripts/build-dmg.sh
# Output: build/TidyMac.dmg
```

---

## Testing

```bash
cargo test                    # Run all 81 tests
cargo test --lib              # Unit tests only
cargo test --test cli_test    # CLI integration tests
cargo test --test hasher_test # Duplicate detection tests
```

| Test Suite | Tests | Coverage |
|-----------|-------|----------|
| Unit (format, safety) | 12 | Formatting, protected paths, validation |
| CLI integration | 23 | Every command, error cases, JSON output |
| Hasher | 9 | 3-pass pipeline, edge cases, grouping |
| Scanner | 30 | Config, profiles, walker, permissions |
| Staging | 7 | Manifest, staging, restore, serialization |
| **Total** | **81** | |

---

## Roadmap

- [x] **Rust CLI** — Full-featured command-line interface
- [x] **SwiftUI GUI** — Native macOS app wrapping the Rust core via FFI
- [ ] **Incremental scan caching** — Only re-scan changed files
- [ ] **Docker integration** — `docker system prune` integration
- [x] **Homebrew formula** — `brew tap jivanandham/tidymac && brew install tidymac`
- [ ] **Scheduled cleanup** — Automated profiles via launchd
- [ ] **Menu bar app** — Quick-access from the macOS menu bar

---

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

```bash
# Fork & clone
git clone https://github.com/<your-username>/tidymac.git
cd tidymac

# Run tests
cargo test

# Submit a PR
```

---

## License

MIT — Built by [Jeeva](https://github.com/jivanandham)
