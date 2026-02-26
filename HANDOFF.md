# Handoff: Version 2.4.0+ (Scenery Tagging, Flight Gen Learning, Regenerate)

**Status**: v2.4.0 features + Flight Generator learning (3.1) and Regenerate
**Major Themes**: Scenery Personalization + Aircraft Discovery + **Flight Gen Learning** + UX Refinement

## Recent Changes

- **Archive Preview Mode (2.4.4)**: Interactive selection of files from `.zip`, `.7z`, and `.rar`. Added `ArchivePreviewState` to `App` and `UnifiedArchiveReader` to `x-adox-core`.
- **Robust Installation**: Added "Flatten Archive" (strips redundant root) and "Wrap in Folder" (forces subfolder) toggles. Unified script redirection for all archive types via `get_installation_paths`.
- **Flight Gen: Detached Context Window**: Implemented a floating, draggable window for the "History & context" panel. Introduced "Pop out" interaction in the inline panel. Refactored UI logic for reusability between inline and window modes.
- **Flight Gen: UX Density Polish**: Updated typography (13.5px, 1.6 line height), added emerald status badges, and implemented an adaptive height system (30-35vh).
- **Flight Gen: Context Modal**: Added a "Show full context" overlay for long snippets.
- **Flight Gen Learning (3.1)**: BitNet `heuristics.json` now stores flight preferences (schema v10): `flight_origin_prefs`, `flight_dest_prefs`, `flight_last_success`. GUI: “Remember this flight”, “Prefer this origin”, “Prefer this destination”. `generate_flight` takes optional `prefs` and prefers these when resolving region-based origin/destination. See `docs/FLIGHT_GEN_BITNET_LEARNING_OPPORTUNITY.md`; 3.2 (LLM/stronger AI) remains on roadmap.
- **Flight Gen Regenerate**: New **Regenerate** button re-runs the last user prompt for a new random outcome without re-typing.
- **Interactive Scenery Tagging**: Added ability to add/remove custom tags directly on scenery cards. Metadata is persisted to `scenery_groups.json`.
- **Generalized Scenery Grouping**: Introduced `SceneryViewMode` (Flat, Region, Tags). Refactored grouping logic to use a unified `scenery_groups` pipeline.
- **Per-Aircraft Variant Toggling**: Aircraft folders with multiple `.acf` files now display variants as indented sub-nodes. Variants can be toggled independently via file renaming (`.acf.disabled`).
- **Expansion Preservation**: Fixed a critical UX bug where the aircraft tree would collapse on every refresh; added snapshot/restore logic for `aircraft_expanded_paths`.
- **Interactive Drag-and-Drop**: Full DND support in `x-adox-gui` with physical INI parity.
- **Security Hardening**: GitHub Action workflows updated with pinned SHAs and restricted triggers.
- **Aircraft AI Heuristics Expansion**: Refined BitNet heuristics to accurately categorize missing helicopters (DF206, Sea King, Blackhawk, etc.) and prevented overlapping tags between `Turboprop` models and `Helicopter` turboshaft engines.
- **Aircraft Immediate Retagging**: Fixed a UI bug where manually overriding an aircraft's category required an app restart. It now instantly parses the tree in-memory via `retag_aircraft_tree` using an updated `BitNetModel` instance.
- **Aircraft Thumbnails**: Expanded the image engine to look for multiple variations of custom thumbnails (e.g., `[acf_name].png`, `[folder_name].jpg`, `icon_hq.png`) to support different aircraft developer schemas.
- **Qodo Review Automation**: Overhauled `.github/workflows/pr_agent.yml` and `.pr_agent.toml` to fully integrate `qodo-ai/pr-agent@main` on GitHub Actions, successfully migrating from GitHub App syntax to `github_action_config`.
- **macOS Universal Binary**: Fixed CI to build a universal (fat) binary for macOS via `lipo` (arm64 + x86_64). Previous builds were arm64-only because `macos-latest` switched to Apple Silicon but `cargo build` was never passed `--target`. Intel Macs now work correctly.

## Critical Context

1. **Tag Grouping Strategy**: Groups are based on the **Primary Tag** (first tag in the list) to avoid list duplication.
2. **Expansion State**: `App.aircraft_expanded_paths` is the source of truth for tree state during reloads. `collect_expanded_paths` must be called BEFORE `Refresh` for changes to survive the rebuild.
3. **INI Fidelity**: `SceneryPack.raw_path` remains the primary key for INI round-trips to preserve user formatting.

## Docs

- **Flight Gen**: `docs/FLIGHT_GEN_AIRPORT_SOURCES.md`, `docs/FLIGHT_GEN_RESEARCH_AND_PLAN.md`, `docs/FLIGHT_GEN_BITNET_LEARNING_OPPORTUNITY.md` (roadmap: 3.2 after 3.1).

## Next Steps

- **UI Polish**: Implement gradient fade cues for scrollables if Iced support improves.
- **Search Integration**: Allow searching for scenery by tag string.
- **Flight Gen 3.2**: Optional LLM/API for natural language and suggestions (post–3.1).
- **Production Build**: Generate latest AppImage assets for testing.
