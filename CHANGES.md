# TidyMac 1.1.0 — safety rework and new features

## Critical fixes (could delete the wrong data)
- **Cache hits deleted whole folders.** Items served from the scan cache had no
  entry list, and the cleaner then removed the item's root (e.g. all of
  `~/Library/Caches`). Cached items are now display-only (`from_cache`) and the
  cleaner never removes a root — only entries inside it.
- **Docker target** walked `Docker.raw` and `~/.docker` (credentials) as files.
  Removed; Docker cleanup now only uses `docker … prune`.
- **Large-file scan** ran on every scan, ignored the profile, didn't prune hidden
  folders/`~/Library`, and was bulk-cleanable. Now opt-in, pruned, review-only.
- **Three deletion paths bypassed safety** (uninstaller, leftovers, privacy).
  Everything goes through `cleaner::deleter::execute` now.
- **App matching** used substrings and `matches[0]`. Now exact bundle id / path /
  unique exact name; name-only matches require confirmation.
- **Safety guard** compared raw strings. Now canonicalized, case-insensitive,
  root-scoped allowlist plus denylist (Keychains, iCloud Drive, Mail, Messages,
  Safari, Photos libraries, credentials…), with per-flow policies.
- **"Clean leftovers"** on an installed app deleted Application Support,
  Preferences and Containers (resets the app). Now only rebuildable data.
- **GUI empty selection / stale scan**: an empty selection meant "clean all" and
  the GUI rescanned before cleaning. Now the GUI cleans exactly the reviewed scan
  (by `scan_id`, minus unticked entries); empty selection removes nothing.

## Correctness
- node_modules finder never found anything; venv finder accepted `.env` files.
- One protected path aborted the whole clean (now skipped and reported).
- Double counting between ~/Library/Caches and per-tool targets.
- Dead targets (/var/log, /var/folders) replaced by your own `$TMPDIR`.
- Soft delete reported "freed" space; now "staged" with expiry.
- Manifest written incrementally with a crash journal; atomic writes; unique
  session ids; `is_dir` recorded before the move.
- Simulators via `xcrun simctl`, Go cache via `go clean -modcache`, sbt keeps
  settings, read-only trees handled.
- `--max-safety` / `--categories` now work; `validate_clean_operation` enforced.
- `launchctl` uses the real uid; startup toggles need an exact label.
- Full Disk Access detected via TCC.db; containers labelled as TCC, not SIP.
- Tools found outside the GUI's minimal PATH (Homebrew, Docker, Go).

## Build & signing
- Removed fake/weakening entitlements; strict rpaths (no CWD-relative paths).
- `build-dmg.sh` auto-selects a real signing identity, bundles the CLI, optional
  notarization. Sentry, jwalk and ignore removed.
- CI fails on warnings, Clippy and rustfmt; all test files and Swift tests run.

## New
- Disk map (treemap) and System Data explainer — `tidymac map`, `system-data`.
- APFS clone-aware sizes everywhere (scan, map, duplicates).
- Leftovers of uninstalled apps — `tidymac apps orphans`.
- Homebrew cask `zap` paths and `pkgutil` receipts in app info/uninstall.
- Custom `[[rules]]` in profiles (see `profiles/custom_example.toml`).
- Duplicates in the GUI with keep-one-copy removal; `tidymac dup --remove`.
- Scheduled dry-run reports via launchd — `tidymac schedule …`.
- Menu bar extra; privacy clean and "empty undo area" in the GUI.

## Known gaps
- Settings sliders for stale days / retention are stored by the GUI but not yet
  written to `~/.tidymac/config.toml` (use `tidymac config set`).
- None of this was compiled where it was written — run the checks below.
