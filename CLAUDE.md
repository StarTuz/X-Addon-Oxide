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

Rust stable toolchain (nightly not required). Release profile uses `lto = "thin"`, `strip = true`, `panic = "abort"` — note that `panic = "abort"` means no stack unwinding in release builds, so `catch_unwind` won't work and panics are immediate process termination.

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
- `migration.rs` - Unified migration engine for legacy `heuristics.json` and pin data (v2.4.0+; extended in 2.4.4)
- `profiles.rs` - Profile management for switching hangar configurations (root-specific isolation)
- `cache.rs` - Disk-backed caching for scenery bounds and metadata (mtime-based invalidation, versioned schema)
- `archive.rs` - `UnifiedArchiveReader` for listing `.zip`, `.7z`, and `.rar` contents (metadata-only for preview)
- `logbook.rs` - X-Plane Pilot.txt parsing (character-perfect for X-Plane 12)
- `apt_dat.rs` - Parser for X-Plane `apt.dat` airport data files (runways, coordinates, ICAO codes, datum row 1302)
- `groups.rs` - User-defined tag/group management for scenery packs (persisted per-config)
- `flight_gen.rs` - Flight plan generation: airport matching, route building, failure logging, multi-format export. `AirportPool` is the public type for pre-indexed airport sets; use `generate_flight_with_pool()` for repeated generation without re-scanning. Bundled data assets (`flight_context_bundle.json`, `flight_context_pois_overlay.json`, `icao_to_wikipedia.csv`) are embedded via `include_bytes!` and loaded by `get_bundled_flight_context()`, `get_poi_overlay()`, `get_icao_to_wikipedia()`.
  - **Guardrail design**: Only two hard constraints — helipad/seaplane-base type matching (helicopter ↔ heliport, floatplane ↔ seaplane base), and keyword-driven surface preference. Keywords `grass`/`unpaved` → Soft, `tarmac`/`asphalt` → Hard, `water`/`seaplane`/`floatplane` → Water (seaplane bases only). Runway length and aircraft-type distance limits are intentionally absent — users control range via keywords (`short`/`quick`, `long haul`, `2 hour flight`) and can swap aircraft after export. Default distance when no keyword given: 10–5000nm.
  - **`endpoints_explicit`**: Distance constraints are relaxed (2nm–20000nm) only when both endpoints are _point_ types (ICAO or NearCity). Region-to-Region pairs keep normal constraints so random picks stay geographically sensible.
  - **Seed airports**: Stored in `data/seed_airports.json` (embedded JSON), loaded once via `OnceLock`. Used as fallback when no pack airports cover a region/city. `seeds_for_constraint()` helper centralises the fallback lookup for both origin and destination.
  - **`FlightPlan`** has `time: Option<TimeKeyword>` and `weather: Option<WeatherKeyword>` fields (parsed from NLP input). These are display/export hints — they do not filter airports. `calculate_solar_time()` derives `TimeKeyword` from longitude + UTC.
- `weather.rs` - `WeatherEngine` fetches real-time METAR data and maps observed conditions to `WeatherKeyword` (Storm, Rain, Cloudy, Clear, etc.). Called by `generate_flight_from_prompt()` when the prompt contains a weather or time keyword. Results used for flight context display in the GUI.
- `data/` - Embedded binary assets: `flight_context_bundle.json` (63 airport history snippets), `flight_context_pois_overlay.json` (curated POIs for EGLL/LIRF), `icao_to_wikipedia.csv` (ICAO→Wikipedia title map for ~16k airports), `seed_airports.json` (~139 region keys including geographic features — Alps, Himalayas, PacIsles sub-regions etc. — fallback airports for flight gen)
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
- **Flight preferences** (schema v11): `flight_origin_prefs`, `flight_dest_prefs`, `flight_last_success` in `heuristics.json`; used by flight gen to prefer airports/remember last flight for region-based prompts
- Lower score = higher priority (inverted from category scores)

**Scoring hierarchy** (lower score = higher priority in `scenery_packs.ini`):

| Category | Score | Notes |
|---|---|---|
| Custom Airports / Named Airports | 10 | e.g., "Charles De Gaulle" packs |
| Airport Overlays | 12 | e.g., FlyTampa overlays |
| Global Airports | 13 | **CRITICAL ANCHOR**: must be above SimHeaven |
| Landmarks | 14 | Official X-Plane Landmarks |
| City Enhancements | 16 | Generic city enhancement packs |
| SimHeaven / X-World | 20 | **MUST BE BELOW GLOBAL AIRPORTS** — exclusion zones hide terminals otherwise |
| Libraries | 40 | Position-independent; never flag as ordering issue |
| Ortho/Photo | 50+ | |
| Mesh | 60+ | |

> When modifying these rules, always run: `cargo test -p x-adox-bitnet --test ordering_guardrails`

**`geo/` module** — `RegionIndex` backed by bundled `regions.json`. Provides `Region` (with one or more `BoundingBox` spans) and fuzzy `search(query)` for resolving natural-language region names (e.g., "British Isles", "Alaska") to bounding boxes used by flight gen for filtering airports.

### x-adox-gui

Iced framework (v0.13) with Elm-like message-driven architecture. `App` struct holds all state; `Message` enum drives updates. **`main.rs` is ~11383 lines** — always use targeted Grep/Read with line ranges, never read the whole file at once.

**Key landmarks in `main.rs`** (use these to navigate):

- `enum Message` (~line 167) — all message variants, grouped by feature
- `struct App` (~line 592) — all application state fields
- `fn update()` (~line 1356) — message handling / business logic dispatch
- `fn view_flight_context_window()` (~line 1116) — detached draggable flight context panel
- `fn subscription()` (~line 4910) — event subscriptions (timers, keyboard)
- `fn view()` (~line 5028) — top-level view routing by tab
- `fn view_scenery()` (~line 7311) — scenery tab layout
- `fn view_scenery_basket()` (~line 7735) — scenery basket panel (selection, bulk toggle)
- `fn view_addon_list()` (~line 8882) — reusable list for plugins/CSLs
- `fn view_aircraft_tree()` (~line 9147) — aircraft tree with smart view

- Tab navigation: Scenery, Aircraft, Plugins, CSLs, FlightGenerator, Heuristics, Issues, Utilities, Settings
- `map.rs` - Interactive world map with tile management and diagnostic health scores (respects `show_health_scores` filter)
- `style.rs` - Dark theme with neon glow effects and animated splash screen (driven by `animation_time` state)
- `flight_gen_gui.rs` - Chat-based flight plan generator UI (natural language input, Regenerate, format selection, export; "Remember this flight", "Prefer this origin/destination" persist to BitNet). Also contains Wikipedia/Wikidata POI fetch functions — see `docs/FLIGHT_CONTEXT.md` for the full architecture of the Flight Context system (bundled data + per-ICAO cache + live API fetch). Uses a **sub-state pattern**: `FlightGenState` struct + its own `Message` enum live here; the main `App` holds `flight_gen: FlightGenState` and delegates `Message::FlightGen(msg)` to it in `update()`. See also `docs/` for other design docs (`FLIGHT_GEN_AIRPORT_SOURCES.md`, `FLIGHT_GEN_RESEARCH_AND_PLAN.md`).
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
- **Archive Preview & Installation**:
  - `ArchivePreviewState` in `App`: Manages selected entries, progress, and robust options (`flatten`, `use_subfolder`).
  - `get_installation_paths()`: Centrally calculates destinations and handles script redirection for all archive types.
  - Messages: `Message::ConfirmArchiveInstall`, `Message::ToggleFlatten`, `Message::ToggleUseSubfolder`.
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
4. **Stable Hashing**: Use FNV-1a (deterministic) for installation-specific config paths.
5. **Pre-Push CI**: You **MUST** run `./scripts/local_ci.sh` before every push to ensure build stability and functional correctness. Non-local CI failures because `local_ci.sh` was skipped are unacceptable.

**Pin migration**: Old versions stored pins globally in `heuristics.json`. New versions store them per-profile in `profiles.json`. `ProfileCollection::sync_with_heuristics()` handles the migration.

## Scenery INI Sync Flow

Startup uses a **two-phase load** to keep the UI responsive even on cold-start (no cache) with Windows Defender active:

**Phase 1 — `SceneryManager::load_quick()`** (fires `Message::SceneryLoaded`, < 500ms):

1. Read existing INI entries (preserves order and raw_path)
2. Scan filesystem for folders via `discovery.rs` (filesystem order, no sorting)
3. Reconcile: match discovered folders to INI entries by name/path
4. Heuristic classify all packs + load any cached airport/tile data. **Uncached packs get empty airports/tiles** — no disk I/O. The `library.txt` presence check (Library category healing in `mod.rs`) is also skipped in quick mode to avoid blocking on slow/locked paths (Windows Defender, network drives); Phase 2 re-classifies.
5. Loading overlay dismisses once all subsystems complete; scenery list is immediately usable.

**Phase 2 — `SceneryManager::load_with_progress(cb)`** (fires `Message::SceneryDeepScanComplete`):

1. Full parallel disk scan for uncached packs: `discover_airports_in_pack`, `discover_tiles_in_pack`
2. Progress reported via `cb(0.0..=1.0)` → `Message::SceneryProgress` → slim progress bar in scenery tab
3. `SceneryDeepScanComplete` merges airports/tiles/categories into `self.packs` (preserving user status changes), re-runs `merge_custom_airports()` and validation, saves cache.
4. On Windows: if deep scan took > 10s → shows a one-time dismissable AV exclusion tip banner (`av_tip_dismissed` persisted in `scan_config.json`).

**`Refresh` uses the full scan** (`load_packs` → `load_with_progress`) since the user explicitly requested it.

Special case: `*GLOBAL_AIRPORTS*` is a virtual INI tag for X-Plane's built-in global airports.

## Scenery Classification Pipeline

Classification is a 3-stage pipeline across multiple files — understanding this flow is critical for category-related changes:

1. **`classifier.rs`** — Name-based heuristic classification. Uses regex patterns on folder names to assign initial `SceneryCategory` (e.g., `Airport`, `Mesh`, `Overlay`, `Library`).
2. **`mod.rs` (post-discovery promotion)** — Content-aware "healing" overrides classifier results by inspecting actual files (`library.txt` → Library, `apt.dat` → Airport, DSF tiles → Mesh). Has a protected category list — check it when adding new categories.
3. **`validator.rs`** — Order validation using resolved `pack.category` (not raw names). Detects issues like SimHeaven below Global Airports, mesh-above-overlay conflicts. Libraries are position-independent and should not be flagged.

### Sorting & Header Invariants (Agent Warning)

- **Rule Name Persistence**: The `scenery_packs.ini` section headers are derived directly from BitNet rule names (`# Rule Name`). 
- **NO Unasked Mapping**: Do NOT introduce "canonical" or "unified" header mappings (e.g., merging "Named Airports" into "Airports") without explicit USER approval. This can split sections if not applied perfectly across both the sorter and writer.
- **Header Tie-Breaker**: Always use the matched rule name as the secondary sort key after the priority score. This ensures items in the same section stay together.
- **Preserve Scan Data**: The `airports`, `tiles`, and `region` fields on `SceneryPack` must be preserved during `reconcile_with_external_packs`. Clearing them triggers non-deterministic sorting flips.

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

Files: `heuristics.json` (sorting rules, pins, aircraft overrides, **flight preferences** — schema v11), `scan_config.json` (exclusions, inclusions, `av_tip_dismissed`), `icon_overrides.json`

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
- **Flight gen tests**: `flight_gen_stress.rs` runs randomized prompts; set `STRESS_SEED=<u64>` for reproducible failures. Run with `--nocapture` to see per-iteration output. `flight_gen_robustness.rs` and `flight_gen_test.rs` cover deterministic edge cases.
- **Stress tests are ignored by default** — run explicitly: `STRESS_SEED=12345 cargo test -p x-adox-core --test flight_gen_stress -- --include-ignored --nocapture`

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
