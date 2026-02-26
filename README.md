# X-Addon-Oxide

The **Next-Gen** addon manager for [X-Plane Flight Simulator](http://www.x-plane.com).

X-Addon-Oxide is a free, open-source tool that brings modern design and AI intelligence to your flight sim hangar. Unlike traditional managers that simply list files, we provide a rich, visual experience with a non-destructive philosophy—organizing your library without risking your installation.

## Why X-Addon-Oxide?

| Feature | X-Addon-Oxide | Standard Managers |
| :--- | :---: | :---: |
| **Native Installers** (.exe, .dmg, AppImage) | ✅ | ⚠️ (Varies) |
| **Direct Zip Installation** | ✅ | ✅ |
| **Non-Destructive Toggling** | ✅ | ✅ |
| **Profile Management** | ✅ | ⚠️ (Varies) |
| **AI Auto-Categorization** (BitNet) | ✅ | ❌ |
| **Scenery Health Diagnostics** | ✅ | ❌ |
| **Interactive World Map** | ✅ | ❌ |
| **Shadow Mesh Detection** | ✅ | ❌ |
| **Natural Language Flight Planning** | ✅ | ❌ |
| **FlyWithLua Script Management** | ✅ | ❌ |
| **Companion App Launcher** | ✅ | ❌ |
| **Automatic Logbook Sync** | ✅ | ❌ |
| **Premium Animated Splash** | ✅ | ❌ |
| **Modern Dark GUI** | ✅ | ❌ |

## Feature Highlights

- **New for 2.4.4**:
  - **Archive Preview Mode & Robust Installation**: All `.zip`, `.7z`, and `.rar` archive installations now trigger an interactive preview modal. Support for **selective extraction**, **flattening** (stripping redundant top-level folders), and **wrapping** (forcing subfolder creation).
  - **Unified Script Redirection**: Detailed redirection logic for **FlyWithLua** and **XPPython3** is now consistent across all archive formats, ensuring scripts always land in the correct simulator directory.
  - **Destination Transparency**: The "Final Destination" path is now displayed in the preview modal for user verification before installation.
- **New for 2.4.3**:
  - **Rust 1.81+ Stability Fix**: Eliminated a scenery-sort panic triggered on Rust 1.81+ when the SimHeaven tiebreaker comparator returned non-transitive results. The `sort_by` comparator now satisfies total ordering in all SimHeaven / non-SimHeaven mixed comparisons. Also backports the `iced_graphics 0.14` `total_cmp()` fix for the renderer damage-grouping path (`damage.rs`) to prevent NaN-induced panics on the tiny_skia fallback renderer. Thanks to [@mmaechtel](https://github.com/mmaechtel) for both fixes.
- **New for 2.4.1**:
  - **Chinese (zh-CN) Flight Generator NLP**: Type flight prompts in Simplified Chinese — 「从北京到上海短途飞行下雨天使用A320在凌晨」 is parsed correctly into origin, destination, duration, weather, aircraft, and time keywords. Includes 80+ city/country aliases (18 new Chinese cities), full weather vocabulary (暴雨→storm, 大雨/小雨→rain, 凌晨→night, etc.), aircraft type hints (直升机, 波音, 空客), and grammatical particle stripping.
  - **Internationalized Flight Generator Chat UI**: The System/User chat labels and welcome message now respect the selected language — switch to Chinese in Settings and the entire Flight Generator interface follows immediately.
  - **Aircraft Tag Parser Fix**: The NLP aircraft capture regex now uses a `\bat\b` terminator, fixing prompts like "A320 at night" or "A320在凌晨" to correctly extract just the aircraft model rather than the trailing context.
- **New for 2.4.0**:
  - **Flight Generator — Weather & Time NLP**: Describe your flight in plain English including weather and time of day. "Stormy morning flight from EGLL to LFPG in a 737" will filter airports by live METAR conditions and pick a realistic departure time automatically. Supports storm, rain, snow, fog, clear, gusty, and calm weather; dawn, morning, noon, afternoon, evening, and night time slots.
  - **Seaplane & Water Routing**: Dedicated water surface keyword detection — "floatplane", "seaplane", or "amphibian" routes exclusively to seaplane bases, no hardcoded pack filters.
  - **FlyWithLua Script Management**: Enable and disable individual Lua scripts within FlyWithLua without touching the plugin itself. Scripts toggle between `Scripts/` and `Scripts (disabled)/` with a single click; a live enabled/total badge shows state at a glance.
  - **Performance Optimizations**: Airport pool now uses `HashMap` with pre-sized capacity (faster flight generation). Scenery cache writes compact JSON instead of pretty-printed (smaller disk footprint). Region index lookups are O(1) via a pre-built `HashMap` index (was O(n) linear scan).
  - **Brand Identity**: New X-Addon-Oxide wordmark logo integrated into the header toolbar, splash screen, and Settings About section.
  - **Professional User Manual**: Full PDF handbook ([X-Addon-Oxide-User-Manual.pdf](X-Addon-Oxide-User-Manual.pdf)) shipping alongside the app for offline reference.
  - **AppImage in CI**: Linux releases now ship a portable `.AppImage` as the primary download in addition to a plain binary tarball — no installation required on any Linux distribution.
- **New for 2.3.1**:
  - **Refined Log Analysis Export**: Export missing resource reports as CSV or TXT with selective checkboxes and a 'Select All / Deselect All' toggle.
  - **Strict Scenery Adherence**: The app now strictly follows your `scenery_packs.ini` and stops auto-adding unmanaged folders found on disk, giving you total control over your library.
  - **7z Archive Support**: Install Aircraft, Scenery, and Plugins directly from `.7z` files in addition to `.zip`.
- **New for 2.2.6**:
  - **Scenery Health Scores**: Diagnostic engine that analyzes metadata and folder structure to ensure your scenery is healthy. High scores (90-100%) indicate stable installations.
  - **Premium Loading Experience**: A completely overhauled splash screen with smooth pulsing animations and shimmer effects.
  - **Enhanced Logbook**: Not just a viewer—now includes robust filtering (Tail #, Aircraft Type, Circular flights, Duration) and character-perfect deletion that maintains strict X-Plane 12 formatting.
  - **Dynamic CSL Detection**: Automatically scans all installed plugins for CSL packages, supporting IVAO, xPilot, and more without hardcoded paths.
  - **Persistent Settings**: Your Map Filter selections (Health Scores, Global Airports, Ortho Coverage, etc.) are now saved and restored across sessions.
  - **Direct Launcher**: Launch X-Plane directly with custom arguments and support for multiple installations.
  - **Companion Apps**: Launch tools like Little Navmap directly from the manager.
- **Logbook Support**: Select a previous flight to visualize its magenta path on the global map. Now with bulk cleanup tools.

### 🚀 Core Management

- **Non-Destructive Workflow**: Enable or disable Scenery, Aircraft, and Plugins with a single click. We never move your files destructively; we manage logical links to keep your simulator safe.
- **Direct Zip & Multi-Format Install**: Install Aircraft, Scenery, and Plugins directly from `.zip`, `.7z`, or `.rar` archives.
- **Archive Preview Mode**: Preview and selectively extract contents from addons before they touch your sim. Includes **Flatten** and **Wrap in Folder** options to fix common nesting issues.
- **Shadow Mesh Detection**: Automatically identifies redundant mesh scenery that destroys load times, helping you optimize performance.
- **Profiles**: Create and switch between different hangar configurations (e.g., "IFR Online", "VFR Scenery Heavy") instantly.
- **Companion App Launcher**: Manage and launch external tools like SimBrief, Navigraph, or VATSIM clients directly from the Plugins tab.
- **Flight Generator**: Natural-language flight plans (e.g. "Stormy evening from London to Paris in a 737") with live METAR weather filtering, time-of-day preferences, seaplane/water routing, **Regenerate** for a new outcome, and BitNet learning ("Remember this flight", "Prefer this origin/destination").
- **FlyWithLua Scripts**: Per-script enable/disable for FlyWithLua without disabling the whole plugin — ideal for managing large script libraries.
- **Logbook & Utilities**: Automatically synced pilot logbook and live aircraft tracking in the Utilities tab.

### 🧠 AI & Visuals

- **AI Smart View**: Powered by our **offline** local **BitNet** heuristic model, your aircraft are automatically categorized (Airliner, Military, GA, Helicopter) without manual tagging. **0% Network Usage, 100% Privacy.** Now with cached grouping for instant switching.
- **World Map**: Visualize your entire scenery library on an interactive global map. See exactly where your coverage is.
- **Buttery Smooth UI**: Decoupled rendering and optimized parsers ensure the interface remains responsive even with thousands of scenery packs and aircraft.
- **Premium Experience**: A sleek, hardware-accelerated interface with dark mode, neon accents, smooth animations, and the X-Addon-Oxide wordmark logo in the header. Features an **Animated Splash Screen** with shimmering progress indicators.
- **Diagnostic Intelligence**: Built-in health checks that alert you to missing metadata or improper scenery classifications. See [HEALTH_SCORE.md](HEALTH_SCORE.md) for details.

### 📦 Deployment

- **Native Support**: We provide proper installers for **Windows** (MSI/EXE), **macOS** (DMG), and **Linux** (AppImage). No dependencies to hunt down.
- **Developer Friendly**: Hot-swap addons while the sim is running (plugin dependent) for rapid testing.

## Release Notes

### v2.4.4

- **Robust Installation Options**: Unified the installation bridge for all formats. Added flattening and wrapping toggles to resolve directory nesting bugs (e.g. `Plugins/Name/Name`).
- **Archive Preview Expansion**: Selective file extraction now supported for `.zip`, `.7z`, and `.rar`.
- **Redirection Parity**: Script-only packages now redirect to `FlyWithLua/Scripts` or `XPPython3/PythonPlugins` consistently across all supported archive types.

### v2.4.3

- **Rust 1.81+ Scenery Sort Stability**: Added missing `else`-branches to the SimHeaven tiebreaker in `sorter.rs` so `sort_by()` satisfies total ordering — fixes a panic on Rust 1.81+ (sort now enforces strict total order checks). Fixes [#2](https://github.com/StarTuz/X-Addon-Oxide/pull/2).
- **iced_graphics `total_cmp()` backport**: Vendor-patches `iced_graphics 0.13` with the upstream `iced 0.14` fix for `damage::group()` using `partial_cmp().unwrap_or(Equal)` on `f32` distances — NaN values could trigger a panic on the tiny_skia fallback renderer path. Replaced with `total_cmp()`. Contributed by [@mmaechtel](https://github.com/mmaechtel).

### v2.4.1

- **Chinese (zh-CN) Flight Generator NLP**: Full Chinese-language input support for the Flight Generator. Prompts like 「从北京到上海短途飞行下雨天使用A320在凌晨」 are parsed into origin, destination, duration, weather, aircraft, and time. Includes 80+ city/country aliases, weather intensity variants (暴雨→storm, 大雨/小雨→rain), time keywords (凌晨→night, 黄昏→dusk), aircraft type hints (直升机, 波音, 空客, 涡桨), vehicle connectors (搭乘/乘坐/使用/驾驶→"in a"), and grammatical particle stripping (在, 的, 了).
- **Internationalized Flight Generator Chat UI**: System/User labels and the welcome message now respect the selected language. Switching to Chinese in Settings updates the chat UI immediately — no restart required.
- **Aircraft Tag Parser Fix**: Added `\bat\b` as an ACF regex terminator so "A320 at night" and "A320在凌晨" (Chinese preprocessing maps 在→"at") correctly extract tag `a320` rather than `a320 at night`.

### v2.4.0

- **Flight Generator — Weather NLP**: Live METAR filtering via aviationweather.gov cache. Keywords: `stormy`, `rainy`, `snowy`, `foggy`, `gusty`, `calm`, `clear`.
- **Flight Generator — Time NLP**: Departure time preferences from natural language. Keywords: `dawn`, `morning`, `noon`, `afternoon`, `evening`, `night`, `midnight`.
- **Flight Generator — Seaplane Routing**: `water`/`seaplane`/`floatplane`/`amphibian` keywords route exclusively to seaplane bases.
- **Flight Generator — NLP Vocabulary Expanded**: 170+ city aliases, 154 geographic regions, 66 ICAO prefix mappings, aircraft modifier phrase stripping.
- **FlyWithLua Script Management**: Enable/disable individual Lua scripts without touching the plugin.
- **Performance**: `HashMap` airport pool with pre-sized capacity; compact JSON scenery cache; O(1) region index lookups.
- **Brand Identity**: New wordmark logo in header toolbar, splash screen, and Settings About page.
- **Professional PDF Manual**: Full [user handbook](X-Addon-Oxide-User-Manual.pdf) available for offline reference.
- **AppImage CI**: Linux releases now include a portable `.AppImage` as the primary download.
- **Code Quality**: UTF-8 safe string scanning in NLP parser; structured key=value logging throughout; non-blocking weather cache (no live network fetch during flight generation).

### v2.3.3

- **Interactive Drag-and-Drop**: Manually reorder your scenery library with intuitive drag handles, visual ghosting, and auto-scrolling.
- **Stateful Bulk Toggle**: A "Smart Toggle" button in the Scenery Basket that dynamically adapts (Disable/Enable/Toggle) based on your selection, with premium color-coded glowing effects (Red/Blue/Purple).
- **Security Hardening**: Hardened GitHub Action workflows with commit SHA pinning and restricted triggers to prevent unauthorized automation execution.
- **Enhanced Scenery Discovery**: Removed alphabetical sorting in discovery to respect natural filesystem and INI order.
- **Improved Migration**: Robust migration for legacy `heuristics.json` pins with automatic corruption detection and backups.

### v2.3.2

- **Content-Aware Scenery Sorting**: "Heals" misclassified scenery by analyzing standard X-Plane file structures.
- **Robust Developer Priority**: Major developers (Orbx, FlyTampa, Aerosoft) are now strictly pinned above generic city scenery.
- **Map Improvements**: Double-click to zoom, better hover priority, and correct draw order.
- **Improved Installation**: Native file dialogs on macOS/Linux.

## Getting Started

1. **Set your X-Plane Path**: Point the app to your X-Plane installation directory.
2. **Explore your Addons**: Use the sidebar to navigate between Aircraft, Scenery, and Plugins.
3. **Smart Sorting**: Use "AI Smart View" in the Aircraft tab to see your fleet organized by role.
4. **Manual Control**: Right-click or use the dropdown in the Aircraft preview to set a manual category override.

For detailed instructions, see the [User Manual (PDF)](X-Addon-Oxide-User-Manual.pdf).

## Configuration

X-Addon-Oxide stores your profiles, tagged groups, and scenery backups in standard system locations:

- **Linux**: `~/.config/x-adox/`
- **Windows**: `%APPDATA%\X-Addon-Oxide\`
- **macOS**: `~/Library/Application Support/X-Addon-Oxide/`

## Installation

### Download Installers (Recommended)

Grab the latest professional installers from the [Releases](https://github.com/StarTuz/X-Addon-Oxide/releases) page:

- **Windows**: `.exe` (NSIS Installer)
- **macOS**: `.dmg` (Disk Image)
- **Linux**: `.AppImage` (Portable — no install required) or `.tar.gz` (Binary tarball)

> **macOS — "App is Broken" / Gatekeeper warning**
>
> Because X-Addon-Oxide is currently unsigned (no Apple Developer certificate), macOS Gatekeeper may refuse to open it with a message like *"X-Addon-Oxide is broken and cannot be opened."*
>
> **One-time fix:** Open Terminal and run:
>
> ```bash
> xattr -cr /Applications/X-Addon-Oxide.app
> ```
>
> Then try launching again. If macOS still blocks it, go to **System Settings → Privacy & Security** and click **"Open Anyway"**.
>
> This is a known limitation until the project has a paid Apple Developer ID for code-signing and notarization. It does not indicate a problem with the app itself.

### Building from Source

#### Prerequisites

You will need the **Rust** toolchain installed ([rustup.rs](https://rustup.rs/)).

#### System Dependencies (Linux)

**Ubuntu / Debian:**

```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libfontconfig1-dev libwayland-dev libx11-dev libxkbcommon-dev libdbus-1-dev libgtk-3-dev pkg-config
```

**Arch Linux:**

```bash
sudo pacman -S alsa-lib fontconfig wayland libx11 libxkbcommon dbus gtk3 pkgconf
```

**Fedora:**

```bash
sudo dnf install alsa-lib-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-devel gtk3-devel pkg-config
```

**openSUSE:**

```bash
sudo zypper install alsa-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-1-devel gtk3-devel pkg-config
```

#### Steps

1. **Clone & Build:**

   ```bash
   git clone https://github.com/StarTuz/X-Addon-Oxide.git
   cd X-Addon-Oxide
   cargo build --release
   ```

2. **Run:**

   ```bash
   cargo run --release -p x-adox-gui
   ```

#### Building AppImage (Linux)

For a portable Linux distribution, you can build an AppImage using Docker to ensure compatibility across distributions:

```bash
chmod +x scripts/build_appimage.sh
./scripts/build_appimage.sh
```

This will create `X-Addon-Oxide-x86_64.AppImage` in the root directory.

## Roadmap

Planned for future releases:

- **Additional Language Support**: English and Simplified Chinese (zh-CN) ship with 219 UI keys each. Japanese, Korean, German, French, and other community-contributed locale files welcome — contributions open on GitHub.
- **X-Plane 12.2 compatibility**: Track new sim-level API changes as they land.

## Contributing

See [GitHub](https://github.com/StarTuz/X-Addon-Oxide) for the latest source and issues.

---
*Developed with ❤️ for the X-Plane Community.*

## Heritage & Attribution

X-Addon-Oxide is an advanced evolution of the original **xaddonmanager** project. We proudly acknowledge the foundational work by **Austin Goudge**, whose original vision for non-destructive X-Plane addon management made this tool possible.

For more details on our project's history and original authorship, please see [ATTRIBUTIONS.md](ATTRIBUTIONS.md).

## License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for the full text.

Copyright (c) 2020 Austin Goudge
Copyright (c) 2026 StarTuz
