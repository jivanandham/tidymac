<div align="center">

# 🧹 TidyMac

### Developer-Aware, Privacy-First Mac Cleanup Utility

**Reclaim 20–50GB of hidden developer cache space with intelligence, safety, and style.**

[![CI](https://github.com/jivanandham/tidymac/actions/workflows/ci.yml/badge.svg)](https://github.com/jivanandham/tidymac/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Homebrew](https://img.shields.io/badge/brew_install-tidymac-FBB040?logo=homebrew&logoColor=white)](https://github.com/jivanandham/homebrew-tidymac)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Swift](https://img.shields.io/badge/Swift-5.9%2B-F05138?logo=swift&logoColor=white)](https://swift.org)
[![macOS](https://img.shields.io/badge/macOS-14%2B_Sonoma-000000?logo=apple&logoColor=white)](https://www.apple.com/macos/)

<br>

```
┌─────────────────────────────────────────────┐
│  $ tidymac scan --profile developer         │
│                                             │
│  🔍 Scanning developer caches...           │
│  ████████████████████████████░░  87%        │
│                                             │
│  📦 Xcode DerivedData     12.3 GB          │
│  🐳 Docker images          8.1 GB          │
│  📁 node_modules           5.7 GB          │
│  🐍 Python venvs           3.2 GB          │
│  📦 Cargo target           2.8 GB          │
│  🍺 Homebrew cache         1.4 GB          │
│  ─────────────────────────────────          │
│  💾 Total reclaimable:    33.5 GB          │
└─────────────────────────────────────────────┘
```

</div>

---

## ⚡ Install in 10 Seconds

```bash
brew tap jivanandham/tidymac && brew install tidymac
```

<details>
<summary>📦 Other installation methods</summary>

### From Source (Rust)
```bash
git clone https://github.com/jivanandham/tidymac.git
cd tidymac && cargo build --release
cp target/release/tidymac /usr/local/bin/
```

### Native macOS App (SwiftUI)
```bash
git clone https://github.com/jivanandham/tidymac.git
cd tidymac
cargo build --release && ./scripts/build-ffi.sh
cd TidyMac && swift run TidyMacApp
```

### DMG Installer
```bash
git clone https://github.com/jivanandham/tidymac.git
cd tidymac && ./scripts/build-dmg.sh
# → build/TidyMac.dmg
```

</details>

---

## 🔥 Why TidyMac?

> **Mainstream cleaners treat every Mac the same.**
> They don't understand developer workflows — and that's where the real space waste lives.

<table>
<tr>
<td width="50%">

### 🎯 Built for Developers

- **14+ dev tool caches** detected automatically
- Xcode DerivedData, Docker, `node_modules`, Python venvs, Cargo, CocoaPods, Gradle, Homebrew, pip, and more
- Understands your workflow — won't delete active project files

</td>
<td width="50%">

### 🛡️ Safety First

- **Dry-run mode** — preview before cleaning
- **7-day undo** — soft delete moves items to `~/.tidymac/staging`; recover with `tidymac undo --last`
  (space is freed when the session expires or with `tidymac purge`)
- **One deletion engine** — every removal (cleans, uninstalls, privacy, duplicates) is checked by the
  same safety guard right before it happens: canonical paths, case-insensitive, must be inside the
  scanned folder, never a folder root
- **Protected paths** — system folders, Keychains, iCloud Drive, Mail, Messages, Photos libraries,
  `~/.ssh` and other credentials are never deletable; `~/Documents` & co. only for single files you pick
- **In-use detection** — anything belonging to a running app is skipped
- **No network code** — no telemetry, no crash reporter, no update pings

</td>
</tr>
</table>

### How TidyMac Compares

Based on each tool's documented features (late 2026). ◐ = partial.

| Feature | TidyMac | CleanMyMac 5 | Mole | AppCleaner |
|:--------|:-------:|:------------:|:----:|:----------:|
| Developer caches (Xcode, simulators via `simctl`, npm, Cargo, Go, Docker via `docker prune`…) | ✅ | ◐ Xcode | ✅ | ❌ |
| Native GUI **and** scriptable CLI on one engine | ✅ | GUI only | CLI (+ paid GUI) | GUI only |
| Review list before cleaning | ✅ per entry | ✅ | ✅ `--dry-run` | ✅ |
| Undo after cleaning | ✅ staging + `tidymac undo` | ◐ some items to Trash | ❌ permanent | ◐ Trash |
| APFS clone-aware "actually freed" sizes | ✅ | ❌ | ❌ | ❌ |
| Custom rules in profile files | ✅ `[[rules]]` | ❌ | ◐ whitelist | ❌ |
| Leftovers of uninstalled apps | ✅ | ✅ | ✅ | ❌ |
| Disk map (treemap) | ✅ | ✅ | ◐ TUI | ❌ |
| JSON output / scheduled reports | ✅ | ❌ | ✅ | ❌ |
| No network code | ✅ | ❌ | ✅ | ✅ |
| Price | Free, MIT | ~$40/yr | Free (CLI) | Free |

--------|:-------:|:---------:|:----------:|
| Developer cache detection (14+ tools) | ✅ | ❌ | ❌ |
| Native SwiftUI + CLI | ✅ | GUI only | GUI only |
| Dry-run before cleaning | ✅ | ❌ | ❌ |
| Undo with 7-day recovery | ✅ | ❌ | ❌ |
| Smart cleanup profiles | ✅ 4 presets | ❌ | ❌ |
| Perceptual image dedup | ✅ | ❌ | ❌ |
| JSON/scripting output | ✅ | ❌ | ❌ |
| Privacy audit (8 browsers) | ✅ | Partial | ❌ |
| 100% offline, zero telemetry | ✅ | ❌ | ✅ |
| Open source | ✅ Free | $40/yr | ✅ Free |

---

## 🚀 Quick Start

```bash
# Initialize TidyMac
tidymac config init

# Your first scan — see what's reclaimable
tidymac scan --profile developer

# Preview what would be cleaned (dry-run)
tidymac clean --profile developer --dry-run

# Clean with 7-day undo safety net
tidymac clean --profile developer
```

---

## 📖 Commands

<details>
<summary><b>🔍 Scanning</b> — Find reclaimable space</summary>

```bash
tidymac scan                              # Quick scan (caches, temp, trash)
tidymac scan --profile developer          # Developer-focused scan
tidymac scan --profile deep --detailed    # Deep scan with file paths
tidymac scan --format json                # JSON output for scripting
```

</details>

<details>
<summary><b>🧹 Cleaning</b> — Reclaim disk space</summary>

```bash
tidymac clean --profile developer         # Soft delete (7-day undo window)
tidymac clean --profile quick --hard      # Permanent deletion
tidymac clean --dry-run                   # Preview what would be cleaned
```

</details>

<details>
<summary><b>↩️ Undo</b> — Recover cleaned files</summary>

```bash
tidymac undo --last                       # Restore last cleanup
tidymac undo --list                       # View all recovery sessions
tidymac undo --session 2026-02-15T19-30-00-123-ab12
```

</details>

<details>
<summary><b>🔎 Duplicate Detection</b> — Find duplicate files & images</summary>

```bash
tidymac dup ~/Documents                   # Find exact duplicates
tidymac dup ~/Pictures --perceptual       # Find visually similar photos
tidymac dup ~/Downloads --detailed        # Show file paths per group
```

**4-pass pipeline for speed:**
```
Pass 1: Group by file size ──► eliminates ~95% instantly
Pass 2: Quick hash (4KB)   ──► eliminates ~4% more
Pass 3: Full SHA-256       ──► confirms exact duplicates
Pass 4: Perceptual hash    ──► finds visually similar images (optional)
```

</details>

<details>
<summary><b>📱 App Management</b> — Find and remove app leftovers</summary>

```bash
tidymac apps list --sort size             # List apps by total footprint
tidymac apps info Slack                   # See app + all associated files
tidymac apps remove Slack --dry-run       # Preview complete removal
tidymac apps remove com.tinyspeck.slackmacgap   # Exact bundle id also works
# Names must match exactly; folders matched only by name need --include-name-matches
```

</details>

<details>
<summary><b>⚙️ Startup Items</b> — Manage login items</summary>

```bash
tidymac startup list                      # Show all login/startup items
tidymac startup disable com.docker.helper # Disable a startup item
tidymac startup enable com.docker.helper  # Re-enable it
```

</details>

<details>
<summary><b>🔒 Privacy Audit</b> — Scan browser data & trackers</summary>

```bash
tidymac privacy scan                      # Full privacy audit
tidymac privacy scan --browsers           # Browser data only
tidymac privacy clean --dry-run           # Preview privacy cleanup
```

Supports: Chrome, Firefox, Safari, Brave, Edge, Arc, Vivaldi, Opera — cookies, history, local storage, cache, and tracking data.

</details>

<details>
<summary><b>🗺 Disk Map & System Data</b> — Where did my space go?</summary>

```bash
tidymac map                               # Biggest folders in your home folder
tidymac map ~/Library --depth 3 --top 20  # Drill into a folder
tidymac system-data                       # Explain macOS "System Data"
```

</details>

<details>
<summary><b>🧩 Leftovers of uninstalled apps</b></summary>

```bash
tidymac apps orphans                              # List leftovers (conservative)
tidymac apps orphans --clean com.example.oldapp   # Remove them (undoable)
```

</details>

<details>
<summary><b>⏰ Scheduled reports</b> — Dry runs on a schedule, never deletes</summary>

```bash
tidymac schedule install --profile developer --every weekly
tidymac schedule status
tidymac schedule remove
```

</details>

<details>
<summary><b>📊 Storage Visualization</b> — Disk usage overview</summary>

```bash
tidymac viz                               # Visual disk usage breakdown
tidymac viz --format json                 # JSON for dashboards
tidymac status                            # Overview of everything
```

</details>

<details>
<summary><b>🐳 Docker</b> — Manage and prune Docker resources</summary>

```bash
tidymac docker                            # Show Docker disk usage
tidymac docker --prune                    # Prune dangling images, containers, volumes
tidymac docker --prune --dry-run          # Preview what would be reclaimed
```

</details>

<details>
<summary><b>🔥 Purge</b> — Manage staging area and auto-purge</summary>

```bash
tidymac purge --expired                   # Purge expired sessions from staging
tidymac purge --all                       # Permanently purge ALL sessions
tidymac purge --install-auto              # Install launchd plist for daily auto-purge
```

</details>

<details>
<summary><b>⚙️ Configuration</b> — Manage TidyMac settings</summary>

```bash
tidymac config init                       # Initialize config at ~/.tidymac
tidymac config show                       # Show current configuration
tidymac config set stale_days 30          # Update a configuration value
tidymac config clear-cache                # Clear the scan cache
```

</details>

---

## 🎛️ Smart Profiles

| Profile | Aggression | What It Cleans | Best For |
|:--------|:----------:|:---------------|:---------|
| 🟢 `quick` | Low | Caches, temp files, trash | Daily maintenance |
| 🟡 `developer` | Medium | + 14 dev tool caches | After coding sessions |
| 🟠 `creative` | Medium | + Render files, previews | After creative projects |
| 🔴 `deep` | High | Everything + large files | Monthly deep clean |

> 💡 Create custom profiles in `~/.tidymac/profiles/<name>.toml`, including your own
> `[[rules]]` (paths, minimum age/size, extensions). See `profiles/custom_example.toml`.
> `tidymac clean` only includes **Safe** items unless you pass `--max-safety caution`.

---

## 🖥️ Native macOS App (SwiftUI)

TidyMac includes a **native SwiftUI application** that wraps the high-performance Rust core via C FFI bindings.

| View | Description |
|:-----|:------------|
| 📊 **Dashboard** | Disk usage overview and quick actions |
| 🔍 **Scan** | Run and visualize scan results |
| 📱 **Apps** | Browse installed apps and their footprint |
| 🐳 **Docker** | Manage Docker images, containers, and volumes |
| 🔒 **Privacy** | Audit browser cookies, history, and trackers |
| 📜 **History** | Review past cleanup sessions with undo |
| ⚙️ **Settings** | Configure profiles, thresholds, and preferences |

<details>
<summary><b>🏗️ Architecture: How Swift Talks to Rust</b></summary>

```
┌──────────────────────────────────────────┐
│          SwiftUI App (TidyMac)           │
│   Dashboard │ Scan │ Apps │ Privacy │…   │
├──────────────────────────────────────────┤
│         TidyMacBridge.swift              │  ← Swift ↔ C bridge
├──────────────────────────────────────────┤
│       TidyMacFFI (C headers)             │  ← module.modulemap + tidymac.h
├──────────────────────────────────────────┤
│      libtidymac.dylib (Rust)             │  ← Compiled Rust core
├──────────────────────────────────────────┤
│          Rust Core Engine                │
│  Scanner │ Cleaner │ Hasher │ Privacy    │
│  Profiles │ Apps │ Startup │ Viz         │
└──────────────────────────────────────────┘
```

</details>

---

## 🏗️ Project Structure

<details>
<summary>Click to expand</summary>

```
tidymac/
├── Cargo.toml               # Rust package manifest
├── src/                     # ⚙️ Rust core engine
│   ├── main.rs              # CLI entry point & command routing
│   ├── lib.rs               # Library root (FFI-ready)
│   ├── ffi.rs               # C FFI exports for SwiftUI
│   ├── cli/                 # Argument parsing & output formatting
│   ├── scanner/             # File system scanning engine
│   ├── cleaner/             # Cleaning with staging & undo
│   ├── duplicates/          # Duplicate detection (SHA-256 + perceptual)
│   ├── apps/                # App discovery & uninstaller
│   ├── startup/             # Startup items manager
│   ├── privacy/             # Privacy audit (8 browsers)
│   ├── viz/                 # Storage visualization
│   ├── profiles/            # Smart profile system
│   └── common/              # Shared utilities & safety guards
├── ffi/                     # 🔗 C header for FFI bridge
│   └── tidymac.h
├── TidyMac/                 # 🖥️ SwiftUI macOS app
│   ├── Package.swift        # Swift Package Manager manifest
│   ├── Sources/             # Swift source files (7 views)
│   └── Libraries/           # FFI artifacts (dylib, header, modulemap)
├── profiles/                # 📋 Built-in cleanup profiles (TOML)
├── scripts/                 # 🛠️ Build & release automation
├── tests/                   # ✅ Integration & unit tests
└── .github/workflows/       # 🔄 CI/CD pipelines
```

</details>

---

## 🔧 Building from Source

### Prerequisites

| Requirement | Version | Notes |
|:------------|:--------|:------|
| macOS | 14+ (Sonoma) | Required for SwiftUI app |
| Rust | 1.70+ | [Install via rustup](https://rustup.rs) |
| Swift | 5.9+ | Included with Xcode 15+ |
| Full Disk Access | — | Recommended for complete scanning |

```bash
# CLI only
cargo build --release

# SwiftUI app
cargo build --release && ./scripts/build-ffi.sh
cd TidyMac && swift run TidyMacApp

# DMG installer
./scripts/build-dmg.sh
```

---

## ✅ Testing

```bash
cargo test --all-targets      # Rust unit + integration tests
cd TidyMac && swift test      # Swift model/treemap tests (after ./scripts/build-ffi.sh)
```

CI fails on any test failure, Clippy warning, or formatting difference.

### Signing for personal use

macOS ties Full Disk Access to the app's code signature. An ad-hoc signed build
loses the permission on every rebuild, so `build-dmg.sh` picks up your
**Apple Development** certificate automatically (free: Xcode › Settings ›
Accounts › Manage Certificates › +). Set `CODESIGN_IDENTITY` to override and
`NOTARY_PROFILE` to notarize with a Developer ID.

---

## 🗺️ Roadmap

- [x] 🦀 **Rust CLI** — Full-featured command-line interface
- [x] 🖥️ **SwiftUI GUI** — Native macOS app via Rust FFI
- [x] 🍺 **Homebrew formula** — `brew tap jivanandham/tidymac && brew install tidymac`
- [x] ⚡ **Incremental scan caching** — Only re-scan changed files
- [x] 🐳 **Docker integration** — `docker system prune` integration
- [x] ⏰ **Scheduled reports** — Dry-run profiles via launchd
- [x] 📌 **Menu bar extra** — Free space and quick actions
- [x] 🗺 **Disk map** — Treemap with drill-down
- [x] 🧩 **Orphaned leftovers** — With Homebrew zap and pkgutil receipts

---

## 🤝 Contributing

Contributions are welcome! Here's how to get started:

```bash
# 1. Fork & clone
git clone https://github.com/<your-username>/tidymac.git
cd tidymac

# 2. Run tests
cargo test

# 3. Make your changes & submit a PR
```

---

<div align="center">

## 📄 License

MIT — Built with ❤️ by [Jeeva](https://github.com/jivanandham)

**If TidyMac saved you disk space, give it a ⭐!**

</div>
