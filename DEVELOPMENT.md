# Development Guide

This document provides guidance for contributors working on this repository.

## Project Overview

X-Addon-Oxide is a cross-platform addon manager for X-Plane Flight Simulator (versions 11/12). It manages Custom Scenery, Aircraft, Plugins, and CSLs with an interactive world map and smart categorization.

## Build Commands

```bash
# Build all crates
cargo build --release

# Run GUI application
cargo run --release -p x-adox-gui

# Run CLI
cargo run --release -p x-adox-cli -- --root /path/to/x-plane list

# Run all tests
cargo test

# Test specific crate
cargo test -p x-adox-core
cargo test -p x-adox-bitnet

# Build AppImage (Linux via Docker)
./scripts/build_appimage.sh

# Local CI pipeline (build + test). MANDATORY BEFORE EVERY PUSH.
./scripts/local_ci.sh
```

## The Golden Rule

> [!IMPORTANT]
> **Always run `./scripts/local_ci.sh` before every push.**
>
> This script builds the release binary and runs all unit and integration tests. It ensures that your changes haven't broken the build or introduced regressions. Committing and pushing without running local CI is the primary cause of build failures in GitHub Actions.

## CI/CD

GitHub Actions builds on push to main and on tags:

- **Windows**: NSIS installer (`.exe`) via cargo-packager
- **macOS**: DMG + App bundle via cargo-packager
- **Linux**: Binary tarball (`.tar.gz`) - AppImage built separately via Docker

Releases are created automatically when pushing a version tag:

```bash
git tag v2.4.0
git push origin v2.4.0
```

**Security**: All GitHub Action workflows are hardened using semantic version pinning (via [commit SHAs](https://docs.github.com/en/actions/using-workflows/workflow-security-hardening-for-github-actions#using-third-party-actions)) and restricted event triggers.

Artifacts are collected to `dist/{platform}/` and uploaded to GitHub Releases.

## Architecture

The project is a Rust workspace with 4 crates:

```text
crates/
├── x-adox-core/     # Core business logic: addon discovery, scenery management, plugin toggling
├── x-adox-gui/      # Iced-based GUI with tab navigation, world map, and dark theme
├── x-adox-cli/      # CLI interface: list, enable, disable, smart-sort commands
└── x-adox-bitnet/   # Heuristics engine for scenery priority scoring and aircraft classification
```

**Data Flow**: GUI/CLI → Core (discovery, management) → BitNet (scoring/classification)

### Key Modules in x-adox-core

- `lib.rs` - XPlaneManager: locates X-Plane installation, parses Log.txt
- `discovery.rs` - Scans Aircraft/, Custom Scenery/, plugins/, CSLs
- `management.rs` - Enables/disables plugins and aircraft; handles **Bulk State updates**.
- `scenery/` - SceneryManager, INI parsing, classification, smart sorting.
- `migration.rs` - (v2.4.0) Unified migration engine for legacy `heuristics.json` and pin data.

### x-adox-bitnet

Rules-based heuristics engine that:

- Scores scenery packs (0-100) for smart sorting. Detailed logic in [HEALTH_SCORE.md](HEALTH_SCORE.md).
- Classifies aircraft by engine type and category
- Supports manual priority overrides (sticky sort)
- **Flight preferences** (schema v10): `flight_origin_prefs`, `flight_dest_prefs`, `flight_last_success` in `heuristics.json`; flight gen uses these when resolving region-based origin/destination

#### BitNet Sorting Rules (Critical)

The sorting hierarchy is strictly defined to prevent scenery conflicts. Future changes **MUST** output this order:

| Priority | Category | Score | Notes |
| :--- | :--- | :--- | :--- |
| **Top** | **Custom Airports** | **10** | Includes `Named Airports` (e.g., "Charles De Gaulle") |
| | **Airport Overlays** | **12** | e.g., FlyTampa Overlays (Must be above Global) |
| | **Global Airports** | **13** | **CRITICAL ANCHOR**: Must be above SimHeaven |
| | **Landmarks** | **14** | Official X-Plane Landmarks |
| | **City Enhancements** | **16** | Generic enhancements (Riga, London, etc.) |
| | **SimHeaven / X-World** | **20** | **MUST BE BELOW GLOBAL AIRPORTS** to avoid exclusion zones hiding terminals |
| | **Libraries** | **40** | |
| | **Ortho/Photo** | **50+** | |
| **Bottom** | **Mesh** | **60+** | |

**Regression Testing**: Always run `cargo test -p x-adox-bitnet --test ordering_guardrails` when modifying these rules.

### x-adox-gui

Iced framework with Elm-like message-driven architecture:

- Tab navigation: Scenery, Aircraft, Plugins (includes Companion Manager), **Flight Generator**, Utilities (Logbook/Map), Heuristics, Issues, Settings
- `flight_gen_gui.rs` - Flight Gen: natural language prompt, **Regenerate** (same prompt, new outcome), export (FMS 11/12, LNM, SimBrief), **Remember this flight** / **Prefer this origin** / **Prefer this destination** (persist to BitNet)
- `map.rs` - Interactive world map showing scenery locations and live flight tracking
- `style.rs` - Dark theme with neon glow effects
- **Folder Exclusions**: Manage scanning scope via Settings (gear icon in Aircraft tab)
- **Aircraft Icon Overrides**: Manually set high-res icons for specific aircraft
- **Companion Manager**: Add and launch external simulator tools from the Plugins tab
- **Pilot Utilities**: Live Logbook monitoring and interactive flight path mapping

## X-Plane Path Conventions

- Scenery config: `$XPLANE_ROOT/Custom Scenery/scenery_packs.ini`
- Disabled addons use suffix: `Aircraft (Disabled)/`, `plugins (disabled)/`
- Config storage (`~/.config/x-adox/X-Addon-Oxide/`):
  - `heuristics.json`: Sorting rules, pins, aircraft overrides, flight preferences (schema v10)
  - `scan_config.json`: Folder exclusions/inclusions
  - `icon_overrides.json`: Manual aircraft icon paths

## Error Handling

Custom error types using `thiserror::Error` in each crate (XamError, SceneryError, AptDatError). Use `anyhow::Result` for general fallback.

## Commit Style

Follow conventional commits: `feat:`, `fix:`, `chore:`, `ci:`, `docs:`, `release:`
