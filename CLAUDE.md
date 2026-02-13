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

Rust stable toolchain (nightly not required). Release profile uses `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"` — note that `panic = "abort"` means no stack unwinding in release builds, so `catch_unwind` won't work and panics are immediate process termination.

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

- `lib.rs` - Path normalization, config root detection, X-Plane install registry lookup, **stable hashing** (FNV-1a)
- `discovery.rs` - Scans Aircraft/, Custom Scenery/, plugins/, CSLs
- `management.rs` - Enables/disables plugins and aircraft via "(Disabled)" suffix folders
- `profiles.rs` - Profile management for switching hangar configurations (root-specific isolation)
- `cache.rs` - Disk-backed caching for scenery bounds and metadata (mtime-based invalidation, versioned schema)
- `logbook.rs` - X-Plane Pilot.txt parsing (character-perfect for X-Plane 12)
- `apt_dat.rs` - Parser for X-Plane `apt.dat` airport data files (runways, coordinates, ICAO codes, datum row 1302)
- `groups.rs` - User-defined tag/group management for scenery packs (persisted per-config)
- `flight_gen.rs` - Flight plan generation: airport matching (includes "British Isles" region support), route building, failure logging, multi-format export
- `scenery/` - SceneryManager, INI parsing, classification, smart sorting, validation
  - `ini_handler.rs` - Reads/writes `scenery_packs.ini` with raw_path round-trip preservation
  - `sorter.rs` - Smart sort using stable `sort_by` to preserve manual pins
  - `classifier.rs` - Heuristic categorization, content-aware "healing" of misclassifications
  - `validator.rs` - Scenery order validation (e.g., SimHeaven below Global Airports detection)
  - `dsf_peek.rs` - Minimal DSF binary parser for scenery type identification (uncompressed DSFs only)

### x-adox-bitnet

Rules-based heuristics engine (not ML despite the name) that:

- Scores scenery packs (0-100) for smart sorting with 16 `SceneryCategory` variants (defined in `scenery/mod.rs`, includes virtual `Group`)
- Classifies aircraft by engine type and category using regex pattern matching
- Parses natural language flight prompts via `flight_prompt.rs` / `parser.rs` (e.g., "London to Paris in a 737")
- Supports manual priority overrides (sticky sort / pins)
- Lower score = higher priority (inverted from category scores)

### x-adox-gui

Iced framework (v0.13) with Elm-like message-driven architecture. `App` struct holds all state; `Message` enum drives updates. **`main.rs` is ~10480 lines** — always use targeted Grep/Read with line ranges, never read the whole file at once.

**Key landmarks in `main.rs`** (use these to navigate):

- `enum Message` (~line 167) — all message variants, grouped by feature
- `struct App` (~line 571) — all application state fields
- `fn update()` (~line 1148) — message handling / business logic dispatch
- `fn subscription()` (~line 4301) — event subscriptions (timers, keyboard)
- `fn view()` (~line 4387) — top-level view routing by tab
- `fn view_scenery()` (~line 6584) — scenery tab layout
- `fn view_scenery_basket()` (~line 6899) — scenery basket panel (selection, bulk toggle)
- `fn view_addon_list()` (~line 8020) — reusable list for plugins/CSLs
- `fn view_aircraft_tree()` (~line 8285) — aircraft tree with smart view

- Tab navigation: Scenery, Aircraft, Plugins, CSLs, FlightGenerator, Heuristics, Issues, Utilities, Settings
- `map.rs` - Interactive world map with tile management and diagnostic health scores (respects `show_health_scores` filter)
- `style.rs` - Dark theme with neon glow effects and animated splash screen (driven by `animation_time` state)
- `flight_gen_gui.rs` - Chat-based flight plan generator UI (natural language input, format selection, export)
- **Drag-and-Drop**:
  - Parity-first design: Drops trigger physical move + pin + save to `scenery_packs.ini`
  - The `save_scenery_packs` helper does a "dumb write" of exact GUI state, bypassing the SceneryManager load/merge cycle for responsiveness
  - Visuals: Grip handles, drop gaps, ghost overlay, auto-scroll (`AbsoluteOffset`)
  - State managed via `DragContext` struct in `main.rs`
- **Scenery Tagging & Grouping**:
  - `SceneryViewMode` (Flat, Region, Tags) controls grouping in `view_scenery`.
  - Tags are interactive: `+` button for adding, `x` for removal.
  - State: `scenery_tag_focus` (active input), `new_tag_input` (text buffer).
  - Persistence: Managed via `groups.rs` and saved to `scenery_groups.json`.
- **Aircraft Variants**:
  - `AircraftNode` holds `variants: Vec<AcfVariant>`.
  - Variants are rendered as indented children in the tree.
  - Toggling renamed `.acf` files; expansion state preserved via `aircraft_expanded_paths` snapshots.
- **Stateful Bulk Toggle**:
  - Detection: View cross-references `selected_basket_items` with `App.packs` to count enabled/disabled items.
  - States: **Disable Selected** (all enabled, ACCENT_RED), **Enable Selected** (all disabled, ACCENT_BLUE), **Toggle Selected** (mixed, ACCENT_PURPLE).
  - Logic: `BulkToggledSelectedBasket` flips each pack's state individually.
  - Concurrency: Button must be `on_press(None)` when `scenery_is_saving` is true to prevent race conditions during I/O.
- **Companion Apps**: External tools (SimBrief, Navigraph) managed in Plugins tab
- **Logbook/Utilities**: Flight path visualization on map, bulk cleanup tools

## Critical Invariants

1. **No Alphabetical Sort in Discovery**: `discovery.rs` must NOT sort results alphabetically. It uses `read_dir()` filesystem order to match X-Plane's own scenery discovery behavior. Alphabetical sorting breaks parity between the app's view and X-Plane's actual load order.

2. **Stable Sort for Pins**: `sorter.rs` relies on Rust's stable `sort_by` to preserve the relative position of manually pinned entries. Using an unstable sort would scramble user-arranged order.

3. **INI Round-Trip Fidelity**: `SceneryPack.raw_path` stores the exact original string from `scenery_packs.ini`. Writes must use `raw_path` when available to preserve the user's original format (absolute paths, custom prefixes, backslash conventions). Never normalize raw_path on write.

4. **No Auto-Add to INI**: Folders discovered on disk but absent from `scenery_packs.ini` remain unmanaged. The app never auto-adds them — the user must run X-Plane once to generate the INI entry. This ensures strict parity with X-Plane's view.

5. **Non-Destructive Philosophy**: Disabled addons are moved to `(Disabled)` folders, never deleted. Original files are always preserved. All operations must be reversible.

## Root-Specific Config Isolation

Config directories are isolated per X-Plane installation using a hash of the install path, stored under `installs/{hash}/`.

**How it works** (in `lib.rs`):

1. **Normalize** (`normalize_install_path`): Resolves the install path against X-Plane's own registry files (`x-plane_install_12.txt`, `x-plane_install_11.txt`) to handle symlinks, trailing slashes, and case variations
2. **Hash** (`calculate_stable_hash`): Uses FNV-1a (deterministic across restarts, unlike Rust's `DefaultHasher`) → 16-char hex string
3. **Migrate** (`get_scoped_config_root`): If a legacy-hash directory exists but no stable-hash directory, moves/copies config automatically. Handles cross-device moves (EXDEV fallback to copy+delete)

**Pin migration**: Old versions stored pins globally in `heuristics.json`. New versions store them per-profile in `profiles.json`. `ProfileCollection::sync_with_heuristics()` handles the migration.

## Scenery INI Sync Flow

When the SceneryManager loads (`scenery/mod.rs`):

1. Read existing INI entries (preserves order and raw_path)
2. Scan filesystem for folders via `discovery.rs` (filesystem order, no sorting)
3. Reconcile: match discovered folders to INI entries by name/path
4. Sync paths if filesystem differs from INI (handles case/whitespace variations)
5. Classify using BitNet rules, then "heal" misclassifications by inspecting actual content (apt.dat, DSF tiles)
6. Parallel processing via `rayon` for expensive operations (apt.dat parsing, DSF inspection)

Special case: `*GLOBAL_AIRPORTS*` is a virtual INI tag for X-Plane's built-in global airports.

## Scenery Classification Pipeline

Classification is a 3-stage pipeline across multiple files — understanding this flow is critical for category-related changes:

1. **`classifier.rs`** — Name-based heuristic classification. Uses regex patterns on folder names to assign initial `SceneryCategory` (e.g., `Airport`, `Mesh`, `Overlay`, `Library`).
2. **`mod.rs` (post-discovery promotion)** — Content-aware "healing" overrides classifier results by inspecting actual files (`library.txt` → Library, `apt.dat` → Airport, DSF tiles → Mesh). Has a protected category list — check it when adding new categories.
3. **`validator.rs`** — Order validation using resolved `pack.category` (not raw names). Detects issues like SimHeaven below Global Airports, mesh-above-overlay conflicts. Libraries are position-independent and should not be flagged.

## X-Plane Integration Points

- Scenery config: `$XPLANE_ROOT/Custom Scenery/scenery_packs.ini`
- Global airports: `Global Scenery/Global Airports/Earth nav data/apt.dat`
- Disabled addons use suffix pattern: `Aircraft (Disabled)/`, `plugins (disabled)/`
- Logs: `Log.txt` for error detection
- Logbook: `Pilot.txt` (character-perfect parsing required for X-Plane 12 compatibility)

## Known Risks

- **X-Plane File Locking**: Accessing `scenery_packs.ini` while X-Plane is writing to it (e.g., on sim exit) can cause conflicts. Race conditions are possible if the user drags scenery while the sim is closing.

## Error Handling

Custom error types using `thiserror::Error` per crate (XamError, SceneryError, AptDatError). Use `anyhow::Result` for general fallback.

## Config Storage

- Linux: `~/.config/x-adox/X-Addon-Oxide/`
- Windows: `%APPDATA%\X-Addon-Oxide\`
- macOS: `~/Library/Application Support/X-Addon-Oxide/`

Files: `heuristics.json`, `scan_config.json`, `icon_overrides.json`

Per-installation configs live in `installs/{hash}/` subdirectories (see Root-Specific Config Isolation above).

## Commit Rules

**IMPORTANT**: Do NOT execute `git commit` without explicit user verification. Present changes first, ask for approval, then commit only after user confirms. See also `.agent/rules/commit_verification.md`.

**Pre-commit checklist:**

1. Run `cargo test` and show the actual terminal output (not just a claim)
2. Show `git diff --stat` to user
3. Wait for explicit user approval
4. Only then commit

**Verification red flags (never do these):**

- Saying "tests passed" without showing output
- Claiming "verified" without evidence
- Committing changes without running the test suite
- Creating test files that don't compile
- A test that doesn't appear in output **DID NOT RUN**

## Commit Style

Follow conventional commits: `feat:`, `fix:`, `chore:`, `ci:`, `docs:`, `release:`, `Logic:`, `UI:`

## Testing Notes

- Use `X_ADOX_CONFIG_DIR` env var to override config directory in tests
- Tests in `x-adox-core` may create temp X-Plane directory structures
- GUI crate has no unit tests (visual testing only)
- Regression tests use naming convention `regression_*.rs` in `crates/x-adox-core/tests/`
- If you create a new test file, run it explicitly: `cargo test -p <crate> --test <filename>`
- **Env var tests must serialize**: `X_ADOX_CONFIG_DIR` is process-global. Tests that call `set_var("X_ADOX_CONFIG_DIR", ...)` must acquire a shared `static ENV_MUTEX: Mutex<()>` to avoid racing. See `regression_hashing_migration.rs` for the pattern.

## CI/CD

GitHub Actions (`ci.yml`) builds on push to main and on version tags:

- Matrix: Linux, Windows, macOS (all x86_64)
- Packages via `cargo-packager`: NSIS installer (Windows), DMG (macOS), tarball (Linux)
- Releases created automatically from `v*` tags

## Linux System Dependencies

For building on Linux, install these packages first:

**Ubuntu/Debian**: `sudo apt-get install -y libasound2-dev libfontconfig1-dev libwayland-dev libx11-dev libxkbcommon-dev libdbus-1-dev libgtk-3-dev pkg-config`

**Arch**: `sudo pacman -S alsa-lib fontconfig wayland libx11 libxkbcommon dbus gtk3 pkgconf`

**Fedora**: `sudo dnf install alsa-lib-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-devel gtk3-devel pkg-config`

**openSUSE**: `sudo zypper install alsa-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-1-devel gtk3-devel pkg-config`
