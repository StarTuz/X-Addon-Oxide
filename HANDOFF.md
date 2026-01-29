# Handoff: Version 2.3.3 (Drag-and-Drop)

**Status**: Released v2.3.3
**Core Feature**: Interactive Scenery Drag-and-Drop

## Recent Changes

- **Drag-and-Drop**: Implemented full DND support in `x-adox-gui`.
  - **Parity**: Uses `save_scenery_packs` to write direct memory state to `scenery_packs.ini`, ensuring what you see is what you get.
  - **Sorting**: We REMOVED the alphabetical sorting from `discovery.rs` to allow the simulation's natural loading order (filesystem dependent) to be the baseline, preventing "fighting" with valid INI orders.
- **Documentation**: Updated README, User Guide, and Architecture notes.

## Critical Context

1. **Determinism**: The previous "Alphabetical Sort" in discovery was a mistake. X-Plane reads directories in filesystem order (mostly). Our app must respect `scenery_packs.ini` above all else.
2. **Persistence**: Dragging an item *immediately* triggers a save. This is intentional to prevent data loss, but ensure `scenery_packs.ini` doesn't get locked by X-Plane if the sim is running (we handle this gracefully via `std::fs` but be aware).

## Next Steps

- Verify Linux AppImage build (CI usually handles this).
- Monitor for reports of "fighting" sort orders if users have manually edited their INI in weird ways.
