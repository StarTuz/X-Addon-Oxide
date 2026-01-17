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

## Building from Source

X-Addon-Oxide is built using Rust and the Iced GUI library.

### Linux (AppImage)

We provide a Docker-based build process for maximum compatibility:

```bash
./scripts/build_appimage.sh
```

### Regular Build

```bash
cargo build --release
```

## Contributing

See [GitHub](https://github.com/StarTuz/X-Addon-Oxide) for the latest source and issues.
