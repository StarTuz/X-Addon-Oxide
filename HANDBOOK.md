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
* **Map Visualization**: Interactive world map showing your installed scenery coverage with health scores and filters.
* **Plugin Management**: Enable/disable plugins and individual Lua scripts without deleting files.
* **Flight Generator**: Natural-language flight plans (e.g. "London to Paris in a 737"), with weather and time-of-day filtering, multiple export formats, and learning ("Remember this flight") persisted via BitNet.
* **Aircraft Installer**: Drop a zip archive directly into X-Addon-Oxide and it extracts to the right folder.
* **Profiles**: Switch between multiple hangar configurations (e.g. Summer, Winter) without reinstalling anything.
* **Utilities**: Built-in Logbook editor, Companion App launcher, Issues scanner, and more.
* **Performance**: Native application (Rust) for blazing-fast startup even with large libraries.

<div class="page-break"></div>

# Getting Started

## Installation

X-Addon-Oxide provides native installers for all major platforms:

* **Windows**: `.exe` installer (NSIS).
* **macOS**: `.dmg` disk image.
* **Linux**: `.AppImage` (portable) or `.deb` / `.rpm`.

## Initial Setup

1. Launch the application.
2. On the first run you will be prompted to locate your **X-Plane Root Directory** (e.g. `C:\X-Plane 12` or `/home/user/X-Plane 12`).
3. Click **Browse…** in the top toolbar if auto-detection fails, then select your X-Plane folder.
4. Once set, the path is shown in the toolbar dropdown. Use the dropdown to switch between multiple X-Plane installations if you have more than one.

The application manages these folders inside your X-Plane root:

* `Aircraft/`
* `Custom Scenery/`
* `Resources/plugins/`
* `Output/logbooks/`

<div class="page-break"></div>

# Profiles

Profiles let you maintain multiple addon configurations for the same X-Plane installation — for example a **Default** (everything enabled) and a **Winter** profile (only winter mesh active).

<img src="pictures/multipleprofilesupport.png" alt="Profile dropdown showing Default and Winter">

* **Switch**: Click the **Profile** dropdown in the top toolbar and select any profile.
* **Create**: Click the **+** button next to the dropdown to add a new profile.
* **Rename / Delete**: Use the pencil and trash icons next to the dropdown.

Each profile stores its own set of enabled/disabled scenery packs and aircraft. Switching profiles immediately re-writes your `scenery_packs.ini` to reflect the chosen configuration.

<div class="tip">
<strong>Tip:</strong> Create a <em>Performance</em> profile with all orthophoto mesh disabled for faster loading on slower machines.
</div>

<div class="page-break"></div>

# Aircraft Manager

The Aircraft tab offers a comprehensive view of your hangar.

<img src="pictures/aircraftnowinstalled.png" alt="Aircraft Library with installed aircraft">

## Installing Aircraft

You can install a new aircraft directly from a zip archive without leaving the app:

1. Click **Install…** in the top toolbar.

   <img src="pictures/install.png" alt="Install button in toolbar">

2. A file browser opens — select the zip archive for the aircraft you want to install.

   <img src="pictures/installexample.png" alt="File browser with zip selected">

3. Choose the destination folder inside your `Aircraft/` directory.

   <img src="pictures/destination.png" alt="Destination folder picker">

4. The archive is extracted with a live progress bar.

   <img src="pictures/installprogress.png" alt="Extracting progress bar">

5. Click **Refresh** when extraction completes and the aircraft appears in the library.

## Searching Aircraft

Use the search bar at the top of the Aircraft Library to filter by name. The tree stays collapsed for matching folders.

<img src="pictures/searchaircraft.png" alt="Aircraft search for 707 showing variants">

Results are shown as a collapsible tree: the folder is the parent node and individual `.acf` variants appear as children.

## Smart View

Toggle the **AI Smart View** button to automatically categorize your aircraft by type (Airliner, GA, Helicopter, Military, Prop, Jet…) using the integrated heuristics engine.

<img src="pictures/aismartview.png" alt="Aircraft Smart View categories">

If an aircraft is miscategorized, select it and use the **Set Category** dropdown in the Inspector Panel.

## Custom Icons

Personalize your hangar with custom icons:

1. Select an aircraft in the list.
2. Click the **Change Icon** button in the Inspector Panel.

   <img src="pictures/changeicon.png" alt="Change Icon button in inspector">

3. A file chooser opens — pick any `.png` or `.jpg` image.

   <img src="pictures/iconselect.png" alt="Select Custom Aircraft Icon dialog">

4. The icon updates immediately in the list.

   <img src="pictures/iconchanged.png" alt="Aircraft with custom red icon applied">

## PDF Manuals

Look for the blue **Book Icon** (📖) next to an aircraft name. Clicking it launches the PDF manual in your default viewer. If multiple manuals exist, the folder opens instead.

<div class="page-break"></div>

# Scenery Manager

The Scenery tab is the heart of X-Addon-Oxide — it manages your `scenery_packs.ini` and gives you full visibility over your installed scenery.

## Searching Scenery

Type in the **Search scenery…** bar to filter the list by name or ICAO code. Search is instant and case-insensitive.

<img src="pictures/searchscenery1.png" alt="Scenery search for EGLL showing Heathrow and related packs">

The results show all packs whose name contains your query — airports, mesh, overlays, and libraries alike.

<img src="pictures/searchscenery2.png" alt="Scenery search showing mixed result types">

## Enabling & Disabling Scenery

Each scenery row has a **DISABLE** / **ENABLE** toggle button on the right. Disabled packs are marked with `SCENERY_PACK_DISABLED` in your `scenery_packs.ini` — X-Plane skips them on next launch without removing any files.

<img src="pictures/disable.png" alt="Scenery Library with disable buttons">

After disabling a pack, the INI entry changes from `SCENERY_PACK` to `SCENERY_PACK_DISABLED`:

<img src="pictures/disabledinscenery.ini.png" alt="scenery_packs.ini showing SCENERY_PACK_DISABLED">

To re-enable, click **ENABLE**:

<img src="pictures/renable.png" alt="Disabled pack showing ENABLE button">

The INI entry reverts to a normal `SCENERY_PACK` line:

<img src="pictures/renabledinsceneryini.png" alt="scenery_packs.ini with pack re-enabled">

<div class="note">
<strong>Note:</strong> Disabling scenery only changes the INI flag. Files remain on disk and can be re-enabled at any time.
</div>

## Tagging Scenery

Assign custom tags to any scenery pack for quick filtering.

1. Click the **+** tag button on a scenery row.

   <img src="pictures/scenertag.png" alt="Tag plus button on scenery row">

2. Type a tag name in the input field that appears.

   <img src="pictures/scenertag1.png" alt="Tag input showing 'French Airport'">

3. Press **Enter** — the tag badge appears on the row. Click **×** on the badge to remove it.

   <img src="pictures/scenertag2.png" alt="French Airport tag applied with remove button">

## View Modes

Use the **view dropdown** (top right of the Scenery Library) to switch between grouping modes:

<img src="pictures/groupbytag.png" alt="View dropdown showing Flat View, Group by Region, Group by Tag options">

| Mode | Description |
|---|---|
| **Flat View** | All packs in one list (loading order) |
| **Group by Region** | Packs grouped by continent/region |
| **Group by Tag** | Packs grouped by your custom tags |
| **Group by Map Enhancement** | Ortho, mesh, overlay packs separated |
| **Group by AutoOrtho** | AutoOrtho tile sets separated |

### Group by Tag

<img src="pictures/showtagresult.png" alt="Group by Tag showing French Airport (1 packs) and Untagged (218 packs)">

Each tag forms a collapsible group header with a count. **Untagged** collects all packs without tags.

### Group by Region

<img src="pictures/groupbyregion.png" alt="Group by Region showing Africa, Asia, Europe, North America, Oceania, Other/Global, South America">

Packs are sorted into continents based on their airport coordinates. Each group has **Add to Bucket** and **Disable All** actions.

## Sorting & Ordering

X-Plane loads scenery top-to-bottom. X-Addon-Oxide gives you several tools to control this:

1. **Smart Sort**: Click to let the AI organize your entire library based on known rules (Custom Airports → Overlays → Libraries → Mesh).
2. **Manual Drag & Drop**: Drag the grip handle (⠿) on the left of any row to reorder manually.
3. **Pinning**: Manual moves are auto-pinned (red pin icon). Smart Sort respects pins in future runs.
4. **Edit Sort**: Opens a text editor for fine-grained pin and rule overrides.

<img src="pictures/editsort.png" alt="Edit Sort view">

### The Golden Rule

> **Global Airports** must load **ABOVE** SimHeaven / X-World overlay packs.

SimHeaven packs include exclusion zones that hide default X-Plane buildings. If SimHeaven loads above Global Airports, terminals disappear. Smart Sort handles this automatically.

<div class="tip">
<strong>Tip:</strong> After any reorder, the changes are written to <code>scenery_packs.ini</code> immediately — there is no separate "Apply" step.
</div>

## Health Score

Each scenery pack is analysed for structural completeness. The score (0–100%) is shown on the interactive map and in the Inspector Panel. Hover over an airport dot on the map to see its score and pack details.

<div class="page-break"></div>

# World Map

The interactive map at the bottom of the screen shows your installed scenery coverage and responds to clicks and hover.

<img src="pictures/worldmap1.png" alt="World map showing coverage dots">

* **Green dots**: Custom airports from your installed scenery packs.
* **Blue tiles**: Orthophoto / mesh coverage areas.
* **Inspector Panel**: Click or hover any feature to see ICAO, name, type, coordinates, and pack health.

<img src="pictures/worldmap2inspct.png" alt="Map zoomed into Scandinavia with airport inspector">

## Zooming & Panning

Scroll to zoom, click-drag to pan. The map renders OpenStreetMap tiles at higher zoom levels so you can inspect exact airport placement.

<img src="pictures/worldmapzoomedsweden.png" alt="Map filter panel open showing Custom Airports, Enhancements, Global Airports checkboxes">

## Map Filters

Click the **Map Filter ▾** button to toggle which layers are visible:

| Filter | Shows |
|---|---|
| Custom Airports | Your installed airport scenery dots |
| Enhancements (Small) | Overlay / enhancement packs |
| Global Airports | Built-in X-Plane airport dots |
| Ortho Coverage | Orthophoto tile footprints |
| OrthoMasters (Grid) | Grid lines for ortho sets |
| Regional Overlays | Regional mesh overlays |
| Flight Paths | Logbook flight tracks |
| Scenery Health Scores | Colour-coded health overlay |

## Scenery Health Scores

Enable **Scenery Health Scores** in the map filter to colour each airport dot by its structural score:

<img src="pictures/sceneryhealth.png" alt="Map with health scores showing coloured airport dots">

* **Green**: Excellent (>80%) — expected files present.
* **Orange/Yellow**: Warning — some files missing or unusual structure.
* **Red**: Poor — critical files absent.

<div class="page-break"></div>

# Plugins & CSLs

## Plugin Management

The Plugins tab lists all plugins found in `Resources/plugins/`. Use the checkbox on any row to enable or disable a plugin. Disabled plugins are moved to a `(disabled)` sub-folder — they are never deleted.

## FlyWithLua Script Management

If you use **FlyWithLua**, X-Addon-Oxide discovers all `.lua` scripts inside its `Scripts/` and `Scripts (disabled)/` folders and displays them as expandable sub-rows under the FlyWithLua plugin entry.

<img src="pictures/pluginsupportwithluanested.png" alt="Plugin Library with FlyWithLua expanded showing individual lua scripts">

* Click the **▶** arrow on the FlyWithLua row to expand/collapse the script list.
* The badge shows `enabled / total` script counts (e.g. `2/12`).
* Check or uncheck individual scripts to move them between `Scripts/` and `Scripts (disabled)/` without touching the plugin itself.

## CSL (Common Shape Library)

For online flying (VATSIM / IVAO), CSLs are essential for traffic models. X-Addon-Oxide scans your installation for CSL packages and lets you toggle them individually.

<img src="pictures/cslsupport.png" alt="CSL tab">

<div class="page-break"></div>

# Flight Generator

The Flight Gen tab generates complete flight plans from natural-language prompts. It uses a global airport database (~38 000 airports) and — when network is available — live METAR weather data.

<img src="pictures/flightgeneration1.png" alt="Flight Generator tab with prompt input">

## Writing a Prompt

Type anything into the **Ask for a flight…** box and press **Send** or **Enter**. The engine extracts:

* **Origin / Destination**: ICAO codes, city names, country names, or regions (e.g. "Europe", "Alaska").
* **Aircraft**: Manufacturer, model, or category (e.g. "Piper", "A320", "helicopter").
* **Duration**: Keywords like `short`, `1 hour`, `long haul`, `transatlantic`.
* **Time of day**: `dawn`, `daytime`, `sunset`, `night`.
* **Weather**: `storm`, `clear`, `fog`, `snow`, `gusty`, `calm`, `rain`.
* **Surface**: `grass`, `water` / `seaplane`, `paved`.
* **Flight type**: `bush`, `backcountry`, `regional`.

### Example Prompts

| Prompt | What it generates |
|---|---|
| `London to Paris in a 737` | EGLL → LFPG range, Boeing tag |
| `One hour flight during daytime with a Piper` | ~120 nm, PA-28 family, airports currently in solar day |
| `Flight for one hour storm` | ~100 nm, airports with active METAR thunderstorm |
| `One hour flight at dawn` | ~450 nm (Boeing used), airports in solar dawn window |
| `Bush flight in Alaska with a floatplane` | Remote strips, seaplane bases only |

<img src="pictures/flighdaytimehour.png" alt="Daytime Piper flight result: WAOO to WRBS, 120nm">

<img src="pictures/flight1hrstorm.png" alt="Storm flight result: FYOT to FYKL Namibia, 102nm">

<img src="pictures/onehourfltdawn.png" alt="Dawn flight result: EDGM to LDOC, Boeing 737-800, 449nm">

## Regenerate

Click **Regenerate** to get a different airport pair for the same prompt without retyping. Each press picks a new random result.

## Export Formats

After a flight is generated, four export buttons appear:

| Button | Format | Use with |
|---|---|---|
| **FMS 11** | X-Plane 11 `.fms` | X-Plane 11 built-in FMS |
| **FMS 12** | X-Plane 12 `.fms` | X-Plane 12 built-in FMS, Toliss, Zibo |
| **LNM** | Little NavMap `.lnmpln` | Little NavMap route planning |
| **SimBrief** | Opens SimBrief website | Full dispatch briefing |

<img src="pictures/fms12example.png" alt="Flight Gen result with export buttons visible">

### FMS 12 Walkthrough

1. Click **FMS 12** — a system save dialog opens, pre-navigated to `Output/FMS plans/`.

   <img src="pictures/fms12save.png" alt="Save FMS 12 dialog pointing to X-Plane Output folder">

2. Save the file (the name is pre-filled with origin and destination ICAO).
3. In X-Plane, open the FMC and go to **F-PLN** → **CO RTE** and enter the route.

   <img src="pictures/fms12insim.png" alt="X-Plane FMC SEC INIT page">

4. The route loads with the correct departure and destination.

   <img src="pictures/fms12insim2.png" alt="FMC showing KLAX/KSFO">

   <img src="pictures/fms12insim3.png" alt="FMC F-PLN showing KLAX→KSFO complete route">

## History & Context

Click **History & Context** to open a side panel showing:

* A Wikipedia summary for the **origin airport** and surrounding area.
* Points of interest near the airport.
* Buttons to **Remember this flight** (persists origin/destination as preferred airports) and **Prefer this origin / destination**.

<img src="pictures/historycontextdeparture.png" alt="History & Context panel showing airport background and Remember this flight button">

<img src="pictures/LNMlittlenav.png" alt="Little NavMap with a generated flight plan">

## Edit Dictionary (NLP Customisation)

The **Edit Dictionary** button opens the NLP JSON editor — a live editor where you can teach the flight engine custom vocabulary without touching code.

<img src="pictures/editflightregenjson.png" alt="NLP Dictionary JSON Editor">

The dictionary supports six rule categories:

| Category | Controls |
|---|---|
| `aircraft_rules` | Maps phrases to aircraft tags, optional distance/speed limits |
| `time_rules` | Maps phrases to solar time-of-day windows |
| `weather_rules` | Maps phrases to METAR weather conditions |
| `surface_rules` | Maps phrases to runway surface preferences |
| `flight_type_rules` | Maps phrases to bush / regional flight types |
| `duration_rules` | Maps phrases to distance-range envelopes |

Click **▶ Valid Values Reference** inside the editor for an inline cheatsheet of all accepted `mapped_value` options.

Use **Import** / **Export** to back up or share your dictionary. Use **Reset to Defaults** to restore the factory vocabulary.

For the full NLP schema reference, see `docs/NLP_DICTIONARY.md`.

<div class="page-break"></div>

# Utilities

## Pilot Logbook

A built-in editor for your `X-Plane Pilot.txt` file.

<img src="pictures/logbook.png" alt="Logbook Editor">

* **Search & Filter**: Find flights by tail number, aircraft type, or date.
* **Delete**: Remove individual or bulk entries. The file format is strictly preserved for X-Plane compatibility.
* **Show Route**: Click any logbook entry to highlight the flight path on the world map.

<img src="pictures/logbookshowroute.png" alt="Logbook with route highlighted on map">

## Companion Apps

Launch your essential flight tools from one place.

1. Go to **Plugins** tab → **Companion Apps** section.
2. Add executables (e.g. SimBrief Downloader, Navigraph Charts, vPilot).
3. Launch them directly before your flight.

## Launch X-Plane with Arguments

The **Launch** button in the toolbar starts X-Plane directly from X-Addon-Oxide. The **Launch args** field lets you pass command-line arguments (e.g. `--safe_mode=Plugin`).

<img src="pictures/launchxpdirectlywithargs.png" alt="Launch args field with X-Plane arguments">

<div class="page-break"></div>

# Issues Dashboard

The **Issues** tab scans your `X-Plane Log.txt` for errors and presents them in a structured report.

<img src="pictures/issueswithresults.png" alt="Issues Dashboard showing X-Plane Log Analysis with missing resource errors">

## Running a Scan

1. Fly a session in X-Plane so `Log.txt` is freshly written.
2. Switch to the **Issues** tab in X-Addon-Oxide.
3. Click **Select All** or pick individual items and click **Scan**.

## Understanding Results

Each issue entry shows:

* **Error type** (e.g. "Missing Resource: weapons.png").
* **Source file** — the scenery pack or plugin that triggered the error.
* **Affected Scenery Pack** — the pack to check or remove.

<div class="tip">
<strong>Tip:</strong> Use the Issues Dashboard after installing a large batch of airports to catch missing library dependencies before your next flight.
</div>

## Export

Results can be exported to CSV or TXT for sharing with addon developers or forum posts.

<div class="page-break"></div>

# Settings

The Settings tab controls application-wide behaviour.

<img src="pictures/settings1.png" alt="Settings tab page 1">
<img src="pictures/settings2.png" alt="Settings tab page 2">

Key options include:

* **Exclude Paths**: Folders to skip during discovery (e.g. large texture libraries you don't want listed).
* **Include Paths**: Additional custom paths to scan outside your X-Plane root.
* **Scan Config**: Deep-scan behaviour and Windows AV exclusion guidance.

<div class="page-break"></div>

# Troubleshooting

## Missing Scenery Objects?

Use the **Issues** tab (see above) to scan your `Log.txt` after a flight and identify missing library assets.

## Scenery Not Showing in X-Plane?

1. Check the **Scenery** tab — verify the pack shows as **Active** (blue dot) and is not `Disabled`.
2. Confirm the loading order — click **Smart Sort** if you recently installed new packs.
3. Restart X-Plane after any scenery change.

## Wrong Scenery Order After Edit?

If you manually edited `scenery_packs.ini` outside the app, click **Refresh** to reload the current file state.

## Log Files

X-Addon-Oxide logs its own activity to:

* **Windows**: `%APPDATA%\x-adox\X-Addon-Oxide\x-adox.log`
* **Linux**: `~/.config/x-adox/X-Addon-Oxide/x-adox.log`
* **macOS**: `~/Library/Application Support/com.x-adox.X-Addon-Oxide/x-adox.log`

---

*This handbook was generated for X-Addon-Oxide v2.4.0.*
