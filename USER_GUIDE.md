# X-Addon-Oxide User Guide

Welcome to X-Addon-Oxide! This guide will help you get the most out of your new addon manager. The Map with the dots represent your custom airports.

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
- **Map Filter Settings**: Use the "Map Filter" collapsible menu to toggle specific data layers (Custom Airports, Global Airports, Mesh & Terrain, etc.).
- **Persistence**: Your map filter selections are automatically saved and will be restored every time you launch the application.
- **Toggling**: Use the **Enable/Disable** buttons on scenery cards. This modifies your `scenery_packs.ini` instantly.
- **Inspector**: Hover over a card to see details in the Inspector Panel, including tile coordinates and airport counts.
- **Interactive Sorting**: Hover over a scenery card to reveal **Move Up** and **Move Down** arrows. Manual reorders instantly create a **Pin** (Red Icon) which the AI will honor forever.
- **Smart Pinning**: Any manual adjustment "teaches" the AI your preferred order. Pinned items are highlighted with a red glow and icon.
- **Clear All Pins**: If you want to undo your manual reorders, use the button at the top of the Scenery list to revert to the default AI logic.
- **Stable Sort**: When using "Smart Sort", the application preserves your manual reorders (pins) for items with tied scores. This ensures that your manual efforts (Pinning) are respected 1:1 and never overwritten by alphabetical tie-breaking.
- **Scenery Health Diagnostics**: A diagnostic score is calculated for every pack based on its contents and metadata.
  - **Scores**: Range from **EXCELLENT (90-100%)** to **CRITICAL (<40%)**.
  - **CEILING**: Note that 90% is the practical "Perfect" ceiling for standard custom addons, as 100% is reserved for system/internal library files.
  - **Visibility**: Display can be toggled via the **Map Filter** menu under the "Utilities" section. Hover over an airport dot to see the parent package's health status instantly.
  - **Details**: See the technical breakdown in `HEALTH_SCORE.md`.

> [!IMPORTANT]
> **Applying Changes**: Manual reorders and pins update the AI's internal logic immediately. However, to write the new order to your actual X-Plane `scenery_packs.ini` file, you must click the **Smart Sort** button and then **Apply Changes** in the Simulation Report.
> [!NOTE]
> **Dynamic Section Headers**: The comment headers in `scenery_packs.ini` (like `# Airports`, `# Libraries`) are generated dynamically based on your rule names in Edit Sort. Packs that don't match any specific rule get generic fallback headers. X-Plane ignores these comments - only the order matters.

### 🧩 Plugins

- **Enabling/Disabling**: Use the checkbox next to each plugin name.
- **How it works**: Disabled plugins are moved to a `plugins (disabled)` folder, preventing X-Plane from loading them without deleting your files.

### ✈️ CSLs (Common Shape Library)

- **Dynamic Scanning**: X-Addon-Oxide automatically scans all your installed plugins (like IVAO_CSL, xPilot, or LiveTraffic) for CSL packages. No manual path configuration required.
- **Toggling**: Enable or disable specific CSL libraries to manage visibility during online operations. Disabled libraries are moved to a protected `CSL (disabled)` subfolder.

### 🛠️ Utilities

- **Pilot Logbook**: Automatically syncs your `X-Plane Pilot.txt` entries into a searchable list.
  - **Filtering**: Use the search bar to filter by Tail Number or Aircraft Type. Toggle "Circular Flights" to find flights that returned to the same airport.
  - **Cleanup & Deletion**: Delete individual entries or use bulk selection to clean your logbook.
  - **Formatting Safety**: Deletions are "character-perfect"—the application meticulously preserves X-Plane's strict fixed-width formatting, including headers, column alignment, and the `99` EOF marker.
  - **Backups**: Every edit automatically creates a `.bak` backup of your original logbook file for safety.
- **Live Map**: Track your aircraft's latest position and historical flight paths on the global interactive map.
- **Performance**: The Utilities engine is optimized to handle large log files without affecting simulator performance.

### 🚀 Companion Apps (managed in Plugins tab)

- **One-Click Launch**: Store and launch your favorite external flight sim tools (SimBrief, Navigraph, VATSIM clients) directly from within X-Addon-Oxide.
- **Auto-Naming**: The manager will attempt to suggest a name based on the executable you select.
- **Centralized**: No more hunting through your desktop for different flight planning or online tools.

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

## Screenshots V2

Quite a few differences with V2 vs V1. First, if you  

Set X-Plane directory:

<img width="756" height="52" alt="image" src="https://github.com/user-attachments/assets/e61e931a-1fba-4282-8184-40ed8313a353" />

### Aircraft

 Unchecking will move them to  X-Plane-12/Aircraft (Disabled) (or whereever you installed X-Plane)

   Folder view

<img width="1030" height="791" alt="image" src="https://github.com/user-attachments/assets/e3c82d58-fa43-405b-bc2c-787278029363" />

  Smart View

<img width="1030" height="791" alt="image" src="https://github.com/user-attachments/assets/2269b2c5-71a2-4035-a9eb-dbb75341c7a5" />

### Scenery

Overall screenshot of Scenery

<img width="1021" height="798" alt="image" src="https://github.com/user-attachments/assets/1a4bbeb7-7f5b-41c4-800f-b23a89639378" />

<img width="994" height="98" alt="image" src="https://github.com/user-attachments/assets/436577cd-8065-40dd-8336-2a929b922119" />

Install - Install a scenery package from Zip

Delete - Delete a selected scenery package.

Refresh - Refresh scenery packages

Smart Sort - BitNet trained to sort your scenery.ini files, up to ten backups saved.

Need to apply to have to effect.

<img width="993" height="786" alt="image" src="https://github.com/user-attachments/assets/d3d6149d-0ab5-41b1-91cb-85ef278c962a" />

Edit Sort - You have full control over the Heuristics with JSON editing if you're not satisfied. Share with others, and also import. Default will reset to shipped settings

<img width="993" height="786" alt="image" src="https://github.com/user-attachments/assets/799a6d3e-b2f0-41e3-983b-adceaca68624" />

### Plugins

Overall picture

<img width="1045" height="781" alt="image" src="https://github.com/user-attachments/assets/ff43f609-fdff-46e8-8d18-9828db99bd48" />

Install - Install plugins from Zip.

Delete - Remove any plugins

Refresh - Refresh if you installed new ones and they aren't showing

Profile:

<img width="634" height="101" alt="image" src="https://github.com/user-attachments/assets/a1d3d922-186e-47f1-be8e-fff29f0913ff" />

Multiple profile support if you want to load/unload certain plugins

Mentioning disabling, unchecking any plugin will disable it (X-Plane-12/Resources/Plugins (disabled).

<img width="1027" height="769" alt="image" src="https://github.com/user-attachments/assets/348aaa85-336d-4e0d-89c2-54872c202051" />

### CSLs (untested)

<img width="1027" height="769" alt="image" src="https://github.com/user-attachments/assets/93b7f7d9-7f8f-4b85-b33f-c8b32bc99c78" />

### Issues

If X-Plane complains about missing scenery objects a scane will provide you with the list, so you can track/install them outside of the Sim

### V2.2.0 -> Added a Utlities section and some UI enhancements

**Premium Animated Loading Screen**: When starting up or switching installations, you will see a dynamic splash screen.

- **Pulsing Logo**: Shows the app is active and processing.
- **Shimmer Progress**: The progress bar shimmers even if scanning is just starting, providing immediate feedback.
- **Breathing Background**: A subtle depth-shifting background for a more modern experience.

<img width="1026" height="794" alt="image" src="https://github.com/user-attachments/assets/102ec509-e264-4b54-9e75-9ee492de293a" />

Launch X-Plane, arguments supported along with multiple copy supported:

<img width="936" height="179" alt="image" src="https://github.com/user-attachments/assets/56e13715-5fd7-4b39-90dc-cf486fd64a1e" />

Utilities Section:

<img width="1080" height="804" alt="image" src="https://github.com/user-attachments/assets/20d39048-3796-478c-88d4-23ab1a7b0483" />

Companion App

Show Manager brings up a sub menu:

<img width="909" height="207" alt="image" src="https://github.com/user-attachments/assets/2e2ca034-7297-4caf-aa2e-f7f5bd56441e" />

Added Little Navmap earlier and launched it:

<img width="2362" height="859" alt="image" src="https://github.com/user-attachments/assets/6b5aac57-3b19-4bae-ad7a-ce62ee7844f9" />

Logbook:

Before selecting a flight.

<img width="1091" height="873" alt="image" src="https://github.com/user-attachments/assets/f7e382aa-9934-4e87-824c-f62a74872192" />

Afer selecting and zoomed in.

<img width="1091" height="873" alt="image" src="https://github.com/user-attachments/assets/c6b81c2e-224c-4ca7-9de3-1969fecffd37" />

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
