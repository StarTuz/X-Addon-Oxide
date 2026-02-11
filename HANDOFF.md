# Handoff: Version 2.4.0 (Manuals, Map Rendering, UI Fixes)

**Status**: Released v2.4.0
**Major Themes**: Interactive Scenery Order + Bulk Lifecycle + Security Hardening

## Recent Changes

- **Interactive Drag-and-Drop**: Full DND support in `x-adox-gui` with physical INI parity.
- **Stateful Bulk Toggle**: Dynamic action button (Disable/Enable/Toggle) with premium glows. Smart backend logic flips individual states.
- **Architecture**: Migration logic for pins/heuristics moved to `x-adox-core` for better separation of concerns.
- **Security**: Hardened GitHub Action workflows (pinned SHAs, restricted triggers).
- **Parity**: Removed alphabetical discovery sorting to favor simulation-natural order.

## Critical Context

1. **Determinism**: Alphabetical sort in discovery is gone. We now respect the raw filesystem order (DiscoveryOrder) to mirror X-Plane 12 behavior.
2. **Persistence**: The Scenery Basket uses the same `save_scenery_packs` logic as direct toggles/DND.
3. **Hardening**: PR Agent is now strictly configured in `.pr_agent.toml` with action-level security.

## Next Steps

- Final verification of the released branch.
- Prepare production build assets (AppImage/MSI).
