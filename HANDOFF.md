# Handoff: Version 2.4.0 (Scenery Tagging, Grouping & Aircraft Variants)

**Status**: Released v2.4.0 Features Implementations
**Major Themes**: Scenery Personalization + Advanced Aircraft Discovery + UX Refinement

## Recent Changes

- **Interactive Scenery Tagging**: Added ability to add/remove custom tags directly on scenery cards. Metadata is persisted to `scenery_groups.json`.
- **Generalized Scenery Grouping**: Introduced `SceneryViewMode` (Flat, Region, Tags). Refactored grouping logic to use a unified `scenery_groups` pipeline.
- **Per-Aircraft Variant Toggling**: Aircraft folders with multiple `.acf` files now display variants as indented sub-nodes. Variants can be toggled independently via file renaming (`.acf.disabled`).
- **Expansion Preservation**: Fixed a critical UX bug where the aircraft tree would collapse on every refresh; added snapshot/restore logic for `aircraft_expanded_paths`.
- **Interactive Drag-and-Drop**: Full DND support in `x-adox-gui` with physical INI parity.
- **Security Hardening**: GitHub Action workflows updated with pinned SHAs and restricted triggers.

## Critical Context

1. **Tag Grouping Strategy**: Groups are based on the **Primary Tag** (first tag in the list) to avoid list duplication.
2. **Expansion State**: `App.aircraft_expanded_paths` is the source of truth for tree state during reloads. `collect_expanded_paths` must be called BEFORE `Refresh` for changes to survive the rebuild.
3. **INI Fidelity**: `SceneryPack.raw_path` remains the primary key for INI round-trips to preserve user formatting.

## Next Steps

- **UI Polish**: Refine the visual spacing of the new `+` tag button.
- **Search Integration**: Allow searching for scenery by tag string.
- **Production Build**: Generate latest AppImage assets for testing.
