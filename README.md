# X-Addon-Oxide

The advanced addon manager for [X-Plane Flight Simulator](http://www.x-plane.com).

X-Addon-Oxide is a powerful, cross-platform tool designed for both flight sim enthusiasts and addon developers. It provides a modern, fast, and visually stunning interface to manage your Custom Scenery, Aircraft, Plugins, and CSLs.

## Key Features

- **🚀 Mod Management**: Effortlessly enable or disable Plugins and CSLs with a single click.
- **🗺️ World Map**: View all your installed scenery packages on an interactive global map.
- **✈️ Aircraft Preview**: Instantly view aircraft icons and technical details before you fly.
- **✨ Premium UI**: A sleek, dark-themed interface with neon glow effects and reactive hover feedback.
- **📦 Multi-Platform**: Built with Rust for high performance on Linux, Windows, and macOS.
- **🛠️ Developer Friendly**: Quickly toggle addons for testing without manual file renaming.

## Getting Started

1. **Set your X-Plane Path**: Point the app to your X-Plane installation directory.
2. **Explore your Addons**: Use the sidebar to navigate between Aircraft, Scenery, and Plugins.
3. **Manage Status**: Use checkboxes for Plugins/CSLs and toggle buttons for Scenery.

For more detailed instructions, see the [Full User Guide](USER_GUIDE.md).

## Installation & Building

### Prerequisites

You will need the **Rust** toolchain installed. If you don't have it, you can get it from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### System Dependencies (Linux)

Before building, ensure you have the necessary development libraries installed.

**Ubuntu/Debian:**

```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libfontconfig1-dev libwayland-dev libx11-dev libxkbcommon-dev libdbus-1-dev
```

**Fedora:**

```bash
sudo dnf install alsa-lib-devel fontconfig-devel wayland-devel libX11-devel libxkbcommon-devel dbus-devel
```

**Arch Linux:**

```bash
sudo pacman -S alsa-lib fontconfig wayland libx11 libxkbcommon dbus
```

### Building from Source

1. **Clone the repository:**

   ```bash
   git clone https://github.com/StarTuz/X-Addon-Oxide.git
   cd X-Addon-Oxide
   ```

2. **Build the project:**

   ```bash
   cargo build --release
   ```

3. **Run the application:**

   ```bash
   ./target/release/x-adox-gui
   ```

### Building as AppImage (Recommended for Linux)

We provide a Docker-based build process to ensure maximum compatibility across different Linux distributions:

```bash
./scripts/build_appimage.sh
```

## Contributing

See [GitHub](https://github.com/StarTuz/X-Addon-Oxide) for the latest source and issues.
