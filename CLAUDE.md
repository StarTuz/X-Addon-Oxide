# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

# Local CI pipeline (build + test + verify binary)
./scripts/local_ci.sh

# Build AppImage (Linux via Docker)
./scripts/build_appimage.sh
```

## Architecture

Rust workspace with 4 crates:

```
crates/
├── x-adox-core/     # Core business logic: addon discovery, scenery management, plugin toggling
├── x-adox-gui/      # Iced-based GUI with tab navigation, world map, dark theme
├── x-adox-cli/      # CLI interface: list, enable, disable, smart-sort commands
└── x-adox-bitnet/   # Heuristics engine for scenery scoring and aircraft classification
```

**Data Flow**: GUI/CLI → Core (discovery, management) → BitNet (scoring/classification)

### x-adox-core Key Modules

- `lib.rs` - XPlaneManager: locates X-Plane installation, parses Log.txt
- `discovery.rs` - Scans Aircraft/, Custom Scenery/, plugins/, CSLs
- `management.rs` - Enables/disables plugins and aircraft via "(Disabled)" suffix folders
- `scenery/` - SceneryManager, INI parsing, classification, smart sorting, validation

### x-adox-bitnet

Rules-based heuristics engine (not ML) that:

- Scores scenery packs (0-100) for smart sorting with 13 SceneryCategory variants
- Classifies aircraft by engine type and category
- Supports manual priority overrides (sticky sort)

### x-adox-gui

Iced framework (v0.13) with Elm-like message-driven architecture:

- Tab navigation: Scenery, Aircraft, Plugins, CSLs, Heuristics, Issues, Utilities, Settings
- `map.rs` - Interactive world map with tile management and diagnostic health scores (respects `show_health_scores` filter)
- `style.rs` - Dark theme with neon glow effects and animated splash screen (driven by `animation_time` state)
- **Drag-and-Drop**:
  - Parity-first design: Drops trigger physical move + pin + save to `scenery_packs.ini`
  - Visuals: Grip handles, drop gaps, ghost overlay, auto-scroll (`AbsoluteOffset`)
  - Determinism: `discovery.rs` must NOT sort alphabetically (uses filesystem order to match X-Plane)

## X-Plane Integration Points

- Scenery config: `$XPLANE_ROOT/Custom Scenery/scenery_packs.ini`
- Global airports: `Global Scenery/Global Airports/Earth nav data/apt.dat`
- Disabled addons use suffix pattern: `Aircraft (Disabled)/`, `plugins (disabled)/`
- Logs: `Log.txt` for error detection
- Logbook: `Pilot.txt` (character-perfect parsing required for X-Plane 12 compatibility)

## Error Handling

Custom error types using `thiserror::Error` per crate (XamError, SceneryError, AptDatError). Use `anyhow::Result` for general fallback.

## Config Storage

- Linux: `~/.config/x-adox/X-Addon-Oxide/`
- Windows: `%APPDATA%\X-Addon-Oxide\`
- macOS: `~/Library/Application Support/X-Addon-Oxide/`

Files: `heuristics.json`, `scan_config.json`, `icon_overrides.json`

## Commit Rules

**IMPORTANT**: Do NOT execute `git commit` without explicit user verification. Present changes first, ask for approval, then commit only after user confirms.

## Commit Style

Follow conventional commits: `feat:`, `fix:`, `chore:`, `ci:`, `docs:`, `release:`, `Logic:`, `UI:`

## Non-Destructive Philosophy

Disabled addons are moved to `(Disabled)` folders, never deleted. Original files are always preserved.
