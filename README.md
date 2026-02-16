# 🧹 TidyMac

**A developer-aware, privacy-first Mac cleanup utility built in Rust.**

[![CI](https://github.com/jeevakrishnasamy/tidymac/actions/workflows/ci.yml/badge.svg)](https://github.com/jeevakrishnasamy/tidymac/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

---

## Why TidyMac?

Mainstream cleaners like CleanMyMac X, OnyX, and AppCleaner treat every Mac the same. They don't understand developer workflows — and that's where 20–50GB of reclaimable space hides.

TidyMac was built for developers. It intelligently detects and cleans **14+ developer tool caches** (Xcode DerivedData, Docker images, `node_modules`, Python venvs, Homebrew, pip, Cargo, CocoaPods, Gradle, and more), while providing safety features no other cleaner offers.

### Key Differentiators

| Feature | TidyMac | CleanMyMac X | AppCleaner |
|---------|---------|-------------|------------|
| Developer cache detection | ✅ 14+ tools | ❌ | ❌ |
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

```bash
# Build from source
git clone https://github.com/jeevakrishnasamy/tidymac.git
cd tidymac
cargo build --release

# Install to PATH
cp target/release/tidymac /usr/local/bin/

# Initialize
tidymac config init

# Your first scan
tidymac scan --profile developer
```

---

## Usage

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

The duplicate finder uses a **3-pass pipeline** for speed:

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

## Architecture

```
┌─────────────────────────────────────────────────┐
│                 TidyMac CLI                     │
│  ┌───────┐ ┌───────┐ ┌─────┐ ┌──────┐ ┌─────┐ │
│  │ Scan  │ │ Clean │ │ Dup │ │ Apps │ │ ... │ │
│  └───┬───┘ └───┬───┘ └──┬──┘ └──┬───┘ └──┬──┘ │
│      │         │        │       │        │     │
│  ┌───┴─────────┴────────┴───────┴────────┴───┐ │
│  │            Core Engine Layer              │ │
│  │  Scanner │ Cleaner │ Hasher │ Profiles    │ │
│  │  Walker  │ Staging │ Perceptual │ Config  │ │
│  │  DevDetect│ Manifest│ Resolver │ Safety   │ │
│  └───────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────┐ │
│  │         macOS Integration Layer           │ │
│  │  File System │ plist │ launchctl │ Perms  │ │
│  └───────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**Tech Stack:** Rust + clap + walkdir + rayon + sha2 + image_hasher + plist + colored + indicatif

**Safety:** Protected path detection prevents accidental deletion of `~`, `/System`, `~/Documents`, `~/.ssh`, etc. — even if a scanner bug mislabels them.

---

## Project Structure

```
src/
├── main.rs              # CLI entry point & command routing (919 lines)
├── lib.rs               # Library root (FFI-ready for SwiftUI)
├── cli/                 # Argument parsing & output formatting
│   ├── args.rs          # clap derive definitions
│   └── output.rs        # Human, JSON, and quiet formatters
├── scanner/             # File system scanning engine
│   ├── targets.rs       # 30+ scan target definitions
│   ├── walker.rs        # Parallel file walker (rayon)
│   └── dev_detector.rs  # Developer tool detection
├── cleaner/             # Cleaning with staging & undo
│   ├── engine.rs        # Dry-run, soft-delete, hard-delete
│   ├── staging.rs       # Soft-delete with file preservation
│   ├── manifest.rs      # JSON operation logging
│   └── purger.rs        # Auto-purge expired sessions
├── duplicates/          # Duplicate detection
│   ├── hasher.rs        # SHA-256 quick & full hashing
│   ├── perceptual.rs    # Image similarity (dHash)
│   ├── grouper.rs       # 4-pass pipeline orchestrator
│   └── resolver.rs      # Smart keep/remove strategies
├── apps/                # App uninstaller
│   ├── detector.rs      # App discovery + plist parsing
│   └── uninstaller.rs   # Safe removal with dry-run
├── startup/             # Startup items manager
│   └── manager.rs       # LaunchAgent/Daemon management
├── privacy/             # Privacy dashboard
│   ├── browsers.rs      # 10 browser profile scanner
│   └── trackers.rs      # Tracking domain database
├── viz/                 # Storage visualization
│   └── storage.rs       # Disk usage bar charts
├── profiles/            # Smart profile system
│   └── loader.rs        # Built-in + custom TOML profiles
└── common/              # Shared utilities
    ├── config.rs         # TOML configuration
    ├── format.rs         # Size/path/duration formatting
    ├── permissions.rs    # SIP & FDA detection
    ├── safety.rs         # Protected path guards
    └── errors.rs         # Error types
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

- [ ] **SwiftUI GUI** — Native macOS app wrapping the Rust CLI via FFI
- [ ] **Incremental scan caching** — Only re-scan changed files
- [ ] **Docker integration** — `docker system prune` integration
- [ ] **Homebrew formula** — `brew install tidymac`
- [ ] **Scheduled cleanup** — Automated profiles via launchd

---

## Requirements

- macOS 12+ (Monterey or later)
- Rust 1.70+ (for building)
- Full Disk Access recommended (for Mail, Safari data)

---

## License

MIT — Built by [Jeeva Krishna Samy](https://github.com/jeevakrishnasamy)
