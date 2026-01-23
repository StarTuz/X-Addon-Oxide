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
| **Interactive World Map** | ✅ | ❌ |
| **Shadow Mesh Detection** | ✅ | ❌ |
| **Modern Dark GUI** | ✅ | ❌ |
| **Companion App Launcher** | ✅ | ❌ |
| **Automatic Logbook Sync** | ✅ | ❌ |

## Feature Highlights

### 🚀 Core Management

* **Non-Destructive Workflow**: Enable or disable Scenery, Aircraft, and Plugins with a single click. We never move your files destructively; we manage logical links to keep your simulator safe.
* **Direct Zip Install**: Install Aircraft, Scenery, and Plugins directly from their archives (`.zip`)—no manual unzipping required.
* **Shadow Mesh Detection**: Automatically identifies redundant mesh scenery that destroys load times, helping you optimize performance.
* **Profiles**: Create and switch between different hangar configurations (e.g., "IFR Online", "VFR Scenery Heavy") instantly.
* **Companion App Launcher**: Manage and launch external tools like SimBrief, Navigraph, or VATSIM clients directly from the Plugins tab.
* **Logbook & Utilities**: Automatically synced pilot logbook and live aircraft tracking in the Utilities tab.

### 🧠 AI & Visuals

* **AI Smart View**: Powered by our **offline** local **BitNet** heuristic model, your aircraft are automatically categorized (Airliner, Military, GA, Helicopter) without manual tagging. **0% Network Usage, 100% Privacy.** Now with cached grouping for instant switching.
* **World Map**: Visualize your entire scenery library on an interactive global map. See exactly where your coverage is.
* **Buttery Smooth UI**: Decoupled rendering and optimized parsers ensure the interface remains responsive even with thousands of scenery packs and aircraft.
* **Premium Experience**: A sleek, hardware-accelerated interface with dark mode, neon accents, and smooth animations.

### 📦 Deployment

* **Native Support**: We provide proper installers for **Windows** (MSI/EXE), **macOS** (DMG), and **Linux** (AppImage). No dependencies to hunt down.
* **Developer Friendly**: Hot-swap addons while the sim is running (plugin dependent) for rapid testing.

## Getting Started

1. **Set your X-Plane Path**: Point the app to your X-Plane installation directory.
2. **Explore your Addons**: Use the sidebar to navigate between Aircraft, Scenery, and Plugins.
3. **Smart Sorting**: Use "AI Smart View" in the Aircraft tab to see your fleet organized by role.
4. **Manual Control**: Right-click or use the dropdown in the Aircraft preview to set a manual category override.

For more detailed instructions, see the [Full User Guide](USER_GUIDE.md).

## Configuration

X-Addon-Oxide stores your profiles, tagged groups, and scenery backups in standard system locations:

* **Linux**: `~/.config/x-adox/`
* **Windows**: `%APPDATA%\X-Addon-Oxide\`
* **macOS**: `~/Library/Application Support/X-Addon-Oxide/`

## Installation

### Download Installers (Recommended)

Grab the latest professional installers from the [Releases](https://github.com/StarTuz/X-Addon-Oxide/releases) page:

* **Windows**: `.exe` (NSIS Installer)
* **macOS**: `.dmg` (Disk Image)
* **Linux**: `.tar.gz` (Binary tarball)

### Building from Source

#### Prerequisites

You will need the **Rust** toolchain installed ([rustup.rs](https://rustup.rs/)).

#### System Dependencies (Linux)

**Ubuntu / Debian:**

```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libfontconfig1-dev libwayland-dev libx11-dev libxkbcommon-dev libdbus-1-dev pkg-config
```

**Arch Linux:**

```bash
sudo pacman -S alsa-lib fontconfig wayland libx11 libxkbcommon dbus pkgconf
```

**Fedora:**

```bash
sudo dnf install alsa-lib-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-devel pkg-config
```

**openSUSE:**

```bash
sudo zypper install alsa-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-1-devel pkg-config
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

For a portable Linux distribution, you can build an AppImage using Docker to ensure compatibility across distributions (like Arch or Ubuntu):

```bash
chmod +x scripts/build_appimage.sh
./scripts/build_appimage.sh
```

This will create `X-Addon-Oxide-x86_64.AppImage` in the root directory.

## Contributing

See [GitHub](https://github.com/StarTuz/X-Addon-Oxide) for the latest source and issues.

---
*Developed with ❤️ for the X-Plane Community.*
