# X-Addon-Oxide

The advanced addon manager for [X-Plane Flight Simulator](http://www.x-plane.com).

X-Addon-Oxide is a powerful, cross-platform tool designed for both flight sim enthusiasts and addon developers. It provides a modern, fast, and visually stunning interface to manage your Custom Scenery, Aircraft, Plugins, and CSLs.

## Key Features

- **🚀 Mod Management**: Effortlessly enable or disable Plugins and CSLs with a single click.
- **🗺️ World Map**: View all your installed scenery packages on an interactive global map.
- **✈️ AI Smart View**: Automatically categorizes aircraft using a built-in BitNet heuristic model (Airliners, Military, GA, etc.).
- **🕵️ Shadow Mesh Detection**: Identification of redundant mesh scenery that negatively impacts load times.
- **🔧 User Overrides**: Manually override AI aircraft categories and set **Custom Aircraft Icons** to perfectly organize your hangar.
- **🛡️ Folder Exclusions**: Exclude specific aircraft folders from the scan to keep your library clean.
- **✨ Premium UI**: A sleek, dark-themed interface with neon glow effects and reactive hover feedback.
- **📦 Multi-Platform**: Native installers for Windows (NSIS), macOS (DMG), and Linux (AppImage/Binary).
- **🛠️ Developer Friendly**: Quickly toggle addons for testing without manual file renaming.

## Getting Started

1. **Set your X-Plane Path**: Point the app to your X-Plane installation directory.
2. **Explore your Addons**: Use the sidebar to navigate between Aircraft, Scenery, and Plugins.
3. **Smart Sorting**: Use "AI Smart View" in the Aircraft tab to see your fleet organized by role.
4. **Manual Control**: Right-click or use the dropdown in the Aircraft preview to set a manual category override.

For more detailed instructions, see the [Full User Guide](USER_GUIDE.md).

## Installation

### Download Installers (Recommended)

Grab the latest professional installers from the [Releases](https://github.com/StarTuz/X-Addon-Oxide/releases) page:

- **Windows**: `.exe` (NSIS Installer)
- **macOS**: `.dmg` (Disk Image)
- **Linux**: `.tar.gz` (Binary tarball)

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
