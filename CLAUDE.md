# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

X-Addon-Oxide is a cross-platform addon manager for X-Plane Flight Simulator (versions 11/12). It manages Custom Scenery, Aircraft, Plugins, and CSLs with an interactive world map and AI-powered categorization.

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

# Local CI pipeline (build + test)
./scripts/local_ci.sh
```

## CI/CD

GitHub Actions builds on push to main and on tags:

- **Windows**: NSIS installer (`.exe`) via cargo-packager
- **macOS**: DMG + App bundle via cargo-packager
- **Linux**: Binary tarball (`.tar.gz`) - AppImage built separately via Docker

Releases are created automatically when pushing a version tag:

```bash
git tag v2.1.5
git push origin v2.1.5
```

Artifacts are collected to `dist/{platform}/` and uploaded to GitHub Releases.

## Architecture

The project is a Rust workspace with 4 crates:

```
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
- `management.rs` - Enables/disables plugins and aircraft by moving to "(Disabled)" folders
- `scenery/` - SceneryManager, INI parsing, classification, smart sorting, validation

### x-adox-bitnet

Rules-based heuristics engine that:

- Scores scenery packs (0-100) for smart sorting
- Classifies aircraft by engine type and category
- Supports manual priority overrides (sticky sort)

### x-adox-gui

Iced framework with Elm-like message-driven architecture:

- Tab navigation: Scenery, Aircraft, Plugins, CSLs, Heuristics, Issues, Settings (Aircraft context)
- `map.rs` - Interactive world map showing scenery locations
- `style.rs` - Dark theme with neon glow effects
- **Folder Exclusions**: Manage scanning scope via Settings (gear icon in Aircraft tab)
- **Aircraft Icon Overrides**: Manually set high-res icons for specific aircraft

## X-Plane Path Conventions

- Scenery config: `$XPLANE_ROOT/Custom Scenery/scenery_packs.ini`
- Disabled addons use suffix: `Aircraft (Disabled)/`, `plugins (disabled)/`
- Config storage (`~/.config/x-adox/X-Addon-Oxide/`):
  - `heuristics.json`: AI sorting rules and tags
  - `scan_config.json`: Folder exclusions/inclusions
  - `icon_overrides.json`: Manual aircraft icon paths

## Error Handling

Custom error types using `thiserror::Error` in each crate (XamError, SceneryError, AptDatError). Use `anyhow::Result` for general fallback.

## Commit Style

Follow conventional commits: `feat:`, `fix:`, `chore:`, `ci:`, `docs:`, `release:`
