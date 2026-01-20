# X-Addon-Oxide User Guide

Welcome to X-Addon-Oxide! This guide will help you get the most out of your new addon manager.

## Initial Setup

When you first launch X-Addon-Oxide, ensure the path to your **X-Plane 11/12** installation is correct in the top-right corner. Click **Set** if you need to browse for a different directory.

## Managing Addons

The sidebar on the left organizes your library into four categories:

### ✈️ Aircraft

- **Viewing**: Browse your aircraft in a hierarchical tree.
- **AI Smart View**: Toggle the "Smart View" switch to automatically group aircraft by their role (Airliners, General Aviation, Military, etc.) using our BitNet heuristic engine.
- **Manual Overrides**: If the AI misidentifies an aircraft, you can manually set its category.
  - Select the aircraft in the tree.
  - In the preview pane, use the **Set AI Category Manually** dropdown.
  - The change is saved instantly and persists across restarts.
- **Custom Aircraft Icons**: You can manually set a custom icon for any aircraft.
  - Select the aircraft in the tree.
  - In the preview pane, click the **Change Icon** button next to the aircraft image.
  - Browse for a `.png`, `.jpg`, `.jpeg`, or `.webp` file.
- **Preview**: Selecting an aircraft shows its icon (if available) and technical tags.

### 🛡️ Settings & Exclusions

Accessible via the **Gear Icon** in the Aircraft toolbar (when Smart View is enabled), the Settings panel allows you to manage your library scan:

- **Folder Exclusions**: Prevent the scanner from searching specific subdirectories.
  - Click **Add Exclusion Folder** to browse for a path to ignore.
  - This is useful for hiding "Generic" or "Static" aircraft libraries that clutter your smart view.
  - Click **Remove** to restore a folder to the scan.
  - Changes require a **Refresh** to take effect in the views.

### 🏔️ Scenery

- **Map View**: Scenery packages are plotted as green dots on the world map.
- **Toggling**: Use the **Enable/Disable** buttons on scenery cards. This modifies your `scenery_packs.ini` instantly.
- **Inspector**: Hover over a card to see details in the Inspector Panel, including tile coordinates and airport counts.
- **Interactive Sorting**: Hover over a scenery card to reveal **Move Up** and **Move Down** arrows. Manual reorders instantly create a **Pin** (Red Icon) which the AI will honor forever.
- **Smart Pinning**: Any manual adjustment "teaches" the AI your preferred order. Pinned items are highlighted with a red glow and icon.
- **Clear All Pins**: If you want to undo your manual reorders, use the button at the top of the Scenery list to revert to the default AI logic.

> [!IMPORTANT]
> **Applying Changes**: Manual reorders and pins update the AI's internal logic immediately. However, to write the new order to your actual X-Plane `scenery_packs.ini` file, you must click the **Smart Sort** button and then **Apply Changes** in the Simulation Report.

### 🧩 Plugins

- **Enabling/Disabling**: Use the checkbox next to each plugin name.
- **How it works**: Disabled plugins are moved to a `plugins (disabled)` folder, preventing X-Plane from loading them without deleting your files.

### 👥 CSLs (Common Shape Library)

- Works identically to Plugins. Toggle checkboxes to manage which CSLs are active for your online flying sessions (VATSIM/IVAO).

## UI Features

- **Neon Indicators**: When a category is active, the icon and sidebar glow in its specific color.
- **Window Icon**: The application now features a custom high-resolution window icon for better visibility in your taskbar.
- **Hover Effects**: Panels will subtly glow when you hover over them, indicating they are interactive.

## Installation

We recommend using the official installers (NSIS for Windows, DMG for macOS) for the best experience.

## Screenshots v1

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/0de14117-4513-4044-96c1-478d6d675ac6" />

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/080eb9a3-280f-4191-b5f6-7ee40aea4049" />

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/b0a2a56c-b10c-4e67-a695-182b0540ddf3" />

<img width="1030" height="799" alt="image" src="https://github.com/user-attachments/assets/64b8b55b-5852-49d3-a3c6-907b5478593d" />
## Uninstallation

If you ever need to remove X-Addon-Oxide, follow these simple steps:

### Windows

1. Open **Settings** > **Apps** > **Apps & Features** (or **Installed Apps**).
2. Locate **X-Addon-Oxide** in the list.
3. Click **Uninstall**. The uninstaller will offer to remove your configuration files if you wish.

### macOS

1. Open your **Applications** folder.
2. Drag **X-Addon-Oxide.app** to the **Trash**.
3. To remove local configuration (optional): Delete the folder at `~/Library/Application Support/com.x-adox.X-Addon-Oxide`.

### Linux (AppImage)

Simply delete the `.AppImage` file. If you integrated it with your desktop (e.g., using `appimaged`), use your desktop's "Remove" feature.

To remove local configuration (optional): Delete the folder at `~/.config/x-adox/X-Addon-Oxide`.

### Linux (Manual/Binary Install)

1. Delete the binaries and desktop entry:
   - `sudo rm /usr/local/bin/x-adox-gui`
   - `sudo rm /usr/share/applications/xam-addon-oxide.desktop`
   - `sudo rm -rf /usr/share/icons/hicolor/*/apps/x-adox-gui.png`

---
*Developed with ❤️ for the X-Plane Community.*
