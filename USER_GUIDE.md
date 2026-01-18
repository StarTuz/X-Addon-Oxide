# X-Addon-Oxide User Guide

Welcome to X-Addon-Oxide! This guide will help you get the most out of your new addon manager.

## Initial Setup

When you first launch X-Addon-Oxide, ensure the path to your **X-Plane 11/12** installation is correct in the top-right corner. Click **Set** if you need to browse for a different directory.

## Managing Addons

The sidebar on the left organizes your library into four categories:

### ✈️ Aircraft

- **Viewing**: Browse your aircraft in a hierarchical tree.
- **Preview**: Selecting an aircraft will show its icon (if available) in the preview panel to the right.
- **Folders**: Expand folders to see different variants or liveries.

### 🏔️ Scenery

- **Map View**: Scenery packages are plotted as green dots on the world map.
- **Toggling**: Use the **Enable/Disable** buttons on scenery cards. This modifies your `scenery_packs.ini` instantly.
- **Inspector**: Hover over a card to see details in the Inspector Panel, including tile coordinates and airport counts.

> [!NOTE]
> **Expected Behavior**: When running "Smart Sort", X-ADOX adds helpful headers and spacing to your `scenery_packs.ini`. However, X-Plane 12 automatically "sanitizes" this file on load, removing all comments and blank lines. **This is normal**; your custom sorting order remains perfectly intact, even if the visual formatting is removed by the sim.

### 🧩 Plugins

- **Enabling/Disabling**: Use the checkbox next to each plugin name.
- **How it works**: Disabled plugins are moved to a `plugins (disabled)` folder, preventing X-Plane from loading them without deleting your files.

### 👥 CSLs (Common Shape Library)

- Works identically to Plugins. Toggle checkboxes to manage which CSLs are active for your online flying sessions (VATSIM/IVAO).

## UI Features

- **Neon Indicators**: When a category is active, the icon and side bar glow in its specific color.
- **Hover Effects**: Panels will subtly glow when you hover over them, indicating they are interactive.
- **Scroll Bars**: Large lists of addons are easily navigable with smooth scrolling.

## Building & Installation

For Linux users, we recommend using the **AppImage** generated via Docker to ensure it runs on any distribution.

```bash
# To build your own AppImage:
./scripts/build_appimage.sh
```

---
*Developed with ❤️ for the X-Plane Community.*
