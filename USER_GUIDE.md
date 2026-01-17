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

## Screenshots v1:

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/0de14117-4513-4044-96c1-478d6d675ac6" />

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/080eb9a3-280f-4191-b5f6-7ee40aea4049" />

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/b0a2a56c-b10c-4e67-a695-182b0540ddf3" />

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/64b8b55b-5852-49d3-a3c6-907b5478593d" />




---
*Developed with ❤️ for the X-Plane Community.*
