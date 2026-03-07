# X-Addon-Oxide

The addon manager for [X-Plane Flight Simulator](http://www.x-plane.com) that treats your sim library like a real working hangar.

X-Addon-Oxide is a free, open-source desktop app for organizing scenery, aircraft, plugins, CSLs, and utility workflows across X-Plane 11 and 12. Its focus is practical: keep large addon libraries readable, sortable, and safe to manage without breaking your simulator layout.

Instead of acting like a bare file browser, X-Addon-Oxide gives you visual structure, health checks, map context, profile switching, archive-first installation, and an offline heuristic layer that helps classify and organize what you already own. The result is less time wrestling with folders and `scenery_packs.ini`, and more time actually flying.

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

## What It Does

- **Manages your library without destructive file handling**: Toggle scenery, aircraft, plugins, CSLs, and scripts while keeping your X-Plane installation readable and recoverable.
- **Installs addons directly from archives**: Open `.zip`, `.7z`, and `.rar` packages, preview their contents, flatten bad folder nesting, wrap missing top-level folders, and confirm the final destination before extraction.
- **Understands scenery order and addon health**: Detects ordering problems, shadowed mesh, weak classifications, and structural issues that lead to broken airports or slow loads.
- **Gives you a visual overview of your sim**: Browse your library on an interactive world map, inspect profiles, and track what is installed where.
- **Helps with flight ideas, not just file management**: The flight generator is designed to suggest plausible routes from rough prompts like `EGLL to Spain`, `north from EGLL around 100nm`, or `stormy evening flight in a turboprop`, whether you already know your departure, want a random departure, want a random destination, or just want a good starting point before refining in Little Navmap, SimBrief, FMS export, or your own planning workflow.
- **Keeps useful sim-adjacent tools in one place**: FlyWithLua script toggling, companion app launching, logbook utilities, and profile-specific preferences are built into the main app instead of scattered across separate tools.

## Recent Highlights

- **Flight generator improvements**: Better natural-language parsing for direction, distance, weather, time of day, Chinese input, aircraft matching, and airport context.
- **Archive preview and safer installs**: Selective extraction, flatten/wrap controls, and clearer destination handling across `.zip`, `.7z`, and `.rar`.
- **Lower-friction script management**: Fine-grained FlyWithLua script enable/disable without turning off the entire plugin.
- **Performance and stability work**: Faster airport lookups, lighter cache output, reduced idle CPU usage, and fixes for Rust 1.81+ sorting edge cases.
- **Cross-platform packaging**: Native Windows installers, macOS bundles/DMGs, Linux AppImage releases, and a full offline handbook.

### 🚀 Core Management

- **Scenery, aircraft, plugins, and CSL control**: Enable, disable, organize, and inspect your library from one place.
- **Archive-first installation**: Install directly from downloads instead of manually sorting folders before every addon.
- **Profile switching**: Maintain different simulator setups for IFR, VFR, online flying, ortho-heavy regions, or test environments.
- **Scenery diagnostics**: Catch ordering problems, bad classifications, and mesh conflicts before they show up in-sim.
- **Plugin-side utilities**: Launch companion apps, manage FlyWithLua scripts, and keep common sim support tasks nearby.
- **Flight suggestion workflow**: Generate route ideas from natural language, including random departures or destinations, regenerate alternatives, then export to FMS, Little Navmap, or SimBrief.
- **Logbook tools**: Inspect, filter, map, and clean logbook entries without losing X-Plane formatting.

### 🧠 AI & Visuals

- **Offline BitNet categorization**: Aircraft and scenery can be grouped intelligently without sending your library to a service.
- **Interactive map view**: See your scenery footprint globally instead of reading folders in isolation.
- **Diagnostics with context**: Health scores, validation messages, and classification hints explain what looks wrong and why.
- **Fast desktop UI**: Built to stay responsive even with large libraries, heavy scenery collections, and thousands of discovered items.
- **Polished presentation**: Animated loading, localized UI, icon handling, and a more modern desktop feel than typical sim managers.

### 📦 Deployment

- **Native distribution**: Windows installers, macOS app bundles/DMGs, and Linux AppImage releases are all supported.
- **Portable where it matters**: Linux AppImage builds work without distro-specific setup.
- **Useful for tinkerers too**: Rapid addon iteration and profile testing make it practical for scenery authors, plugin users, and heavy library curators.

## Release Notes

### v2.4.7

- **Flight Generator — Direction & Distance NLP**: Added support for directional prompts such as `north`, `southwest`, `northbound`, and `towards the east`, plus bare distance phrases like `100km`, `70nm`, and mixed-unit ranges.
- **Flight Generator — Better Aircraft Matching**: Improved fuzzy aircraft detection so prompts are more tolerant of common shorthand, connectors, and model phrasing. This includes fixes for phrases like `on a 737` and better handling of aircraft names near destination text.
- **Flight Generator — Geographic Prompt Expansion**: Added broader geographic feature and alias support so rough prompts based on cities, regions, and location names resolve more reliably.
- **Flight Generator — Runway Metadata Awareness**: Airport and aircraft metadata now carries runway length and surface requirements more consistently, allowing the generator to surface better warnings without over-rejecting route ideas.
- **NLP Robustness & Data Cleanup**: Externalized more location alias data, reduced aircraft false-positives, and tightened parser behavior around ambiguous prompt fragments.

### v2.4.5

- **Performance Fix (GUI Idle CPU)**: Resolved a major performance regression where the GUI would consume 20-30% CPU even when idle. Fixed the perpetual 16ms animation timer in the subscription guard.
- **Map Initialization**: Auto-initialize map zoom after scenery loading completes, eliminating the need for a background timer to wait for the first map render.

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

### v2.4.0 (previous release)

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
