---
title: "X-Addon-Oxide User Manual"
author: "StarTuz"
date: "February 2026"
---

<style>
  body { font-family: "Helvetica Neue", Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; max-width: 800px; margin: 0 auto; }
  h1, h2, h3 { color: #2c3e50; }
  h1 { border-bottom: 2px solid #3498db; padding-bottom: 10px; margin-top: 40px; }
  h2 { border-bottom: 1px solid #ecf0f1; padding-bottom: 5px; margin-top: 30px; }
  .cover-page { text-align: center; padding-top: 100px; page-break-after: always; }
  .cover-title { font-size: 48px; font-weight: bold; margin-bottom: 10px; color: #2c3e50; }
  .cover-subtitle { font-size: 24px; color: #7f8c8d; margin-bottom: 50px; }
  .cover-version { font-size: 18px; color: #95a5a6; margin-top: 20px; }
  .page-break { page-break-after: always; }
  img { max-width: 100%; border: 1px solid #ddd; border-radius: 4px; box-shadow: 0 2px 5px rgba(0,0,0,0.1); margin: 20px 0; }
  code { background-color: #f8f9fa; padding: 2px 4px; border-radius: 3px; font-family: monospace; }
  pre { background-color: #f8f9fa; padding: 15px; border-radius: 5px; overflow-x: auto; border: 1px solid #e9ecef; }
  blockquote { border-left: 4px solid #3498db; margin: 0; padding-left: 15px; color: #555; background: #ecf6fd; padding: 10px; }
  table { width: 100%; border-collapse: collapse; margin: 20px 0; }
  th, td { border: 1px solid #ddd; padding: 12px; text-align: left; }
  th { background-color: #f2f2f2; color: #333; }
  .note { background-color: #fff3cd; border: 1px solid #ffeeba; padding: 10px; border-radius: 5px; color: #856404; margin: 20px 0; }
  .tip { background-color: #d4edda; border: 1px solid #c3e6cb; padding: 10px; border-radius: 5px; color: #155724; margin: 20px 0; }
</style>

<div class="cover-page">
  <img src="assets/packaging/icon_512.png" width="200" height="200" alt="App Icon" style="border:none; box-shadow:none;">
  <div class="cover-title">X-Addon-Oxide</div>
  <div class="cover-subtitle">The Modern Addon Manager for X-Plane 11 & 12</div>
  <div class="cover-version">Version 2.4.0</div>
  <div class="cover-version">User Manual</div>
</div>

# Introduction

Welcome to **X-Addon-Oxide**, the next-generation addon manager for X-Plane. Designed for performance and ease of use, it helps you organize your aircraft, scenery, plugins, and more with powerful AI-assisted features.

**Key Features:**

* **Smart Sorting**: AI-powered scenery sorting (BitNet engine) that respects X-Plane's strict loading order.
* **Map Visualization**: Interactive world map showing your installed scenery coverage.
* **Plugin Management**: Enable/disable plugins without deleting files.
* **Utilities**: Built-in Logbook editor, Companion App launcher, and more.
* **Performance**: Native application (Rust) for blazing fast speeds.

<div class="page-break"></div>

# Getting Started

## Installation

X-Addon-Oxide provides native installers for all major platforms:

* **Windows**: `.exe` installer (NSIS).
* **macOS**: `.dmg` disk image.
* **Linux**: `.AppImage` (portable) or `.deb`/`.rpm`.

## Initial Setup

1. Launch the application.
2. On the first run, you will be prompted to locate your **X-Plane Root Directory** (e.g., `C:\X-Plane 11` or `/home/user/X-Plane 12`).
3. Click **Set** button in the top-right corner if the auto-detection fails.

<img src="https://github.com/user-attachments/assets/e61e931a-1fba-4282-8184-40ed8313a353" alt="Setup Screen">

The application effectively manages these folders:

* `Aircraft/`
* `Custom Scenery/`
* `Resources/plugins/`
* `Output/logbooks/`

<div class="page-break"></div>

# Aircraft Manager

The Aircraft tab offers a comprehensive view of your hangar.

## Smart View

Toggle the **Smart View** switch to automatically categorize your aircraft by type (Airliner, GA, Helicopter, Military) using the integrated heuristics engine.

<img src="https://github.com/user-attachments/assets/2269b2c5-71a2-4035-a9eb-dbb75341c7a5" alt="Aircraft Smart View">

### Manual Overrides

If an aircraft is miscategorized:

1. Select the aircraft in the list.
2. In the preview pane, use the **Set Category** dropdown.
3. Your choice is saved and persists across restarts.

### Custom Icons

You can personalize your hangar by setting custom icons:

1. Select an aircraft.
2. Click the **Change Icon** button.
3. Choose any `.png` or `.jpg` image.

## PDF Manuals (New in v2.4.0)

Access your aircraft documentation instantly.

* Look for the blue **Book Icon** (📖) next to an aircraft name.
* Clicking it will launch the PDF manual directly in your default viewer.
* If multiple manuals are found, the folder containing them will open.

<div class="page-break"></div>

# Scenery Manager

The Scenery tab is the heart of X-Addon-Oxide, offering powerful tools to manage your `scenery_packs.ini`.

<img src="https://github.com/user-attachments/assets/1a4bbeb7-7f5b-41c4-800f-b23a89639378" alt="Scenery Manager Overview">

## Map View

* **Green Dots**: Custom airports installed.
* **Blue Tiles**: Orthophoto/Mesh coverage.
* **Inspector**: Hover over any map feature to see details in the Inspector Panel.

## Sorting & Ordering

X-Plane loads scenery in a specific order (top to bottom). X-Addon-Oxide helps you manage this:

1. **Smart Sort**: Click this button to let the AI organize your entire library based on known rules (Airports > Libraries > Mesh).
2. **Manual Reordering**: Drag and drop rows to reorder them manually.
3. **Pinning**: Manual moves are "pinned" (marked with a red icon). The Smart Sort will respect these pins in future runs.

### Layering Rules (The "Golden Rules")

X-Addon-Oxide enforces strict layering rules to prevent graphical glitches (like missing airport terminals). The most critical rule to know is:

> **Global Airports** must be **ABOVE** **SimHeaven / X-World**.

* **Why?** SimHeaven packs contain "exclusion zones" that hide default X-Plane scenery. If SimHeaven loads *above* Global Airports, it will hide the default terminals, leaving you with empty aprons.
* **Smart Sort**: Automatically handles this by placing Global Airports at **Score 13** and SimHeaven at **Score 20**.
* **Manual Sorting**: If you manually move SimHeaven *above* Global Airports, the **Simulation Report** will flag this as a "Critical" error to warn you.

<div class="tip">
<strong>Tip:</strong> Always remember to click <strong>Apply Changes</strong> after sorting to write the new order to your <code>scenery_packs.ini</code> file.
</div>

## Health Score

Each scenery pack is analyzed for potential issues.

* **Excellent (90-100%)**: Contains expected files (apt.dat, dsf).
* **Warning (<50%)**: Missing critical files or structure issues.
* Hover over an airport on the map to see its health status.

<div class="page-break"></div>

# Plugins & CSLs

Manage your plugins and online traffic models (CSL) with ease.

## Plugin Management

* **Toggle**: Use the checkbox to enable/disable plugins.
* **Safety**: Disabled plugins are moved to a `(disabled)` folder, keeping your installation clean.

<img src="https://github.com/user-attachments/assets/ff43f609-fdff-46e8-8d18-9828db99bd48" alt="Plugin Manager">

## CSL (Common Shape Library)

For online flying (VATSIM/IVAO), CSLs are crucial. X-Addon-Oxide scans your installation for CSL packages and allows you to toggle them individually.

<div class="page-break"></div>

# Utilities

## Pilot Logbook

A powerful editor for your `X-Plane Pilot.txt`.

* **Search & Filter**: Find flights by tail number, aircraft type, or date.
* **Clean Up**: Delete individual or multiple entries. The file format is strictly preserved.
* **Backups**: Automatic backups (`.bak`) are created before any changes.

<img src="https://github.com/user-attachments/assets/f7e382aa-9934-4e87-824c-f62a74872192" alt="Logbook Editor">

## Companion Apps

Launch your essential flight tools from one place.

1. Go to **Plugins** tab -> **Companion Apps**.
2. Add executables (e.g., SimBrief Downloader, Navigraph Charts, vPilot).
3. Launch them directly from X-Addon-Oxide before your flight.

<div class="page-break"></div>

# Troubleshooting

## Missing Scenery Objects?

Use the **Issues** tab to scan your `Log.txt` after a flight.

* The app identifies missing library assets.
* Export a report to CSV/TXT to find and install the missing dependencies.

## Log Files

X-Addon-Oxide logs its own activity to:

* **Windows**: `%APPDATA%\x-adox\X-Addon-Oxide\x-adox.log`
* **Linux**: `~/.config/x-adox/X-Addon-Oxide/x-adox.log`
* **macOS**: `~/Library/Application Support/com.x-adox.X-Addon-Oxide/x-adox.log`

---

*This handbook was generated for X-Addon-Oxide v2.4.0.*
