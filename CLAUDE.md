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

# Run a single test by name
cargo test -p x-adox-core test_name_here

# Local CI pipeline (build + test + verify binary)
./scripts/local_ci.sh

# Full test suite with crate-by-crate checks
./scripts/test_all.sh

# Build AppImage (Linux via Docker)
./scripts/build_appimage.sh

# Create release (triggers GitHub Actions CI)
git tag v2.x.x && git push origin v2.x.x
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

- `lib.rs` - Path normalization, config root detection, X-Plane install registry lookup
- `discovery.rs` - Scans Aircraft/, Custom Scenery/, plugins/, CSLs
- `management.rs` - Enables/disables plugins and aircraft via "(Disabled)" suffix folders
- `profiles.rs` - Profile management for switching hangar configurations (root-specific isolation)
- `cache.rs` - Disk-backed caching for scenery bounds and metadata
- `logbook.rs` - X-Plane Pilot.txt parsing (character-perfect for X-Plane 12)
- `scenery/` - SceneryManager, INI parsing, classification, smart sorting, validation

### x-adox-bitnet

Rules-based heuristics engine (not ML) that:

- Scores scenery packs (0-100) for smart sorting with 13 SceneryCategory variants
- Classifies aircraft by engine type and category
- Supports manual priority overrides (sticky sort)

### x-adox-gui

Iced framework (v0.13) with Elm-like message-driven architecture. `App` struct holds all state; `Message` enum drives updates.

- Tab navigation: Scenery, Aircraft, Plugins, CSLs, Heuristics, Issues, Utilities, Settings
- `map.rs` - Interactive world map with tile management and diagnostic health scores (respects `show_health_scores` filter)
- `style.rs` - Dark theme with neon glow effects and animated splash screen (driven by `animation_time` state)
- **Drag-and-Drop**:
  - Parity-first design: Drops trigger physical move + pin + save to `scenery_packs.ini`
  - Visuals: Grip handles, drop gaps, ghost overlay, auto-scroll (`AbsoluteOffset`)
  - Determinism: `discovery.rs` must NOT sort alphabetically (uses filesystem order to match X-Plane)
- **Companion Apps**: External tools (SimBrief, Navigraph) managed in Plugins tab
- **Logbook/Utilities**: Flight path visualization on map, bulk cleanup tools

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

Config directories are **root-specific** (isolated per X-Plane installation) using a hash of the install path.

## Commit Rules

**IMPORTANT**: Do NOT execute `git commit` without explicit user verification. Present changes first, ask for approval, then commit only after user confirms.

**Pre-commit checklist:**
1. Run `cargo test` and show passing output
2. Show `git diff --stat` to user
3. Wait for explicit user approval
4. Only then commit

## Commit Style

Follow conventional commits: `feat:`, `fix:`, `chore:`, `ci:`, `docs:`, `release:`, `Logic:`, `UI:`

## Testing Notes

- Use `X_ADOX_CONFIG_DIR` env var to override config directory in tests
- Tests in `x-adox-core` may create temp X-Plane directory structures
- GUI crate has no unit tests (visual testing only)

## Verification Requirements

**CRITICAL**: Agents must provide proof of verification, not just claims.

**Before claiming tests pass:**
- Run `cargo test` and show the actual terminal output
- If you create a new test file, run it explicitly: `cargo test -p <crate> --test <filename>`
- A test that doesn't appear in output **DID NOT RUN**
- Compilation errors mean tests **DID NOT PASS** - do not claim otherwise

**Before committing:**
- Run `cargo test` and show output proving all tests pass
- Show `git diff --stat` of exactly what will be committed
- If tests fail to compile, fix them before claiming "done"

**Red flags (never do these):**
- Saying "tests passed" without showing output
- Claiming "verified" without evidence
- Committing changes without running the test suite
- Creating test files that don't compile

## Non-Destructive Philosophy

Disabled addons are moved to `(Disabled)` folders, never deleted. Original files are always preserved.

## Linux System Dependencies

For building on Linux, install these packages first:

**Ubuntu/Debian**: `sudo apt-get install -y libasound2-dev libfontconfig1-dev libwayland-dev libx11-dev libxkbcommon-dev libdbus-1-dev pkg-config`

**Arch**: `sudo pacman -S alsa-lib fontconfig wayland libx11 libxkbcommon dbus pkgconf`

**Fedora**: `sudo dnf install alsa-lib-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-devel pkg-config`

**openSUSE**: `sudo zypper install alsa-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-1-devel pkg-config`
