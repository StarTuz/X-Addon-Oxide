# Expert Briefing: X-Addon-Oxide v2.4.0

## System Architecture

### Core Components

- **Drag-and-Drop Engine**:
  - **Logic**: `crates/x-adox-gui/src/main.rs`. State managed via `DragContext` struct.
  - **Persistence**: Hybrid approach. Physical move in memory (`self.packs`), BitNet Pinning (`heuristics.json`: scenery overrides, aircraft overrides, **flight preferences** — origin/dest prefs, last success), and Immediate File Write (`scenery_packs.ini` via `save_scenery_packs`).
  - **Parity**: We strictly mirror the order of `self.packs` to the INI file.

### Critical Invariants

1. **No Alphabetical Sort**: `discovery.rs` MUST NOT sort alphabetically. It must yield files in the order the OS returns them (or purely filesystem order) if we are to respect "undefined" loading orders correctly, or at least not enforce an order that contradicts the user's explicit INI file relative positions during discovery merging.
2. **Stable Sorting**: `sorter.rs` relies on Rust's `sort_by` being stable to preserve manual pins.
3. **Parity**: The `save_scenery_packs` helper bypasses the typical "SceneryManager load/merge" cycle to perform a "dumb write" of the exact GUI state. This is crucial for DND to feel responsive and accurate.

## Known Risks

- **X-Plane Locking**: Accessing `scenery_packs.ini` while X-Plane is writing to it (e.g. on exit) can cause conflicts. X-Addon-Oxide handles this via simpler error reporting but race conditions are possible if the user is dragging while the sim is closing.

## Development Environment

- **Rust**: Nightly not required, stable is fine.
- **Iced**: v0.13. Requires standard Linux libs (`libkbcommon`, `wayland`, etc.).
