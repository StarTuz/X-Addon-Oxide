# Handoff: Version 2.4.0+ (Scenery Tagging, Flight Gen Learning, Regenerate)

**Status**: v2.4.0 features + Flight Generator learning (3.1) and Regenerate
**Major Themes**: Scenery Personalization + Aircraft Discovery + **Flight Gen Learning** + UX Refinement

## Recent Changes

- **Flight Gen Learning (3.1)**: BitNet `heuristics.json` now stores flight preferences (schema v10): `flight_origin_prefs`, `flight_dest_prefs`, `flight_last_success`. GUI: “Remember this flight”, “Prefer this origin”, “Prefer this destination”. `generate_flight` takes optional `prefs` and prefers these when resolving region-based origin/destination. See `docs/FLIGHT_GEN_BITNET_LEARNING_OPPORTUNITY.md`; 3.2 (LLM/stronger AI) remains on roadmap.
- **Flight Gen Regenerate**: New **Regenerate** button re-runs the last user prompt for a new random outcome without re-typing.
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

## Docs

- **Flight Gen**: `docs/FLIGHT_GEN_AIRPORT_SOURCES.md`, `docs/FLIGHT_GEN_RESEARCH_AND_PLAN.md`, `docs/FLIGHT_GEN_BITNET_LEARNING_OPPORTUNITY.md` (roadmap: 3.2 after 3.1).

## Next Steps

- **UI Polish**: Refine the visual spacing of the new `+` tag button.
- **Search Integration**: Allow searching for scenery by tag string.
- **Flight Gen 3.2**: Optional LLM/API for natural language and suggestions (post–3.1).
- **Production Build**: Generate latest AppImage assets for testing.
