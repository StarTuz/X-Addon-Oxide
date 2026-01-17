#!/bin/bash
set -e

# This script is intended to be run inside the docker container
export APPIMAGE_EXTRACT_AND_RUN=1

echo "Building Rust project..."
cargo build --release --bin x-adox-gui

echo "Preparing AppDir..."
mkdir -p AppDir/usr/bin
cp target/release/x-adox-gui AppDir/usr/bin/x-adox-gui

# Copy icons
mkdir -p AppDir/usr/share/icons/hicolor/scalable/apps
cp crates/xam_gui/assets/icons/aircraft.svg AppDir/usr/share/icons/hicolor/scalable/apps/x-adox-gui.svg

# Run linuxdeploy
linuxdeploy --appdir AppDir \
    --desktop-file crates/xam_gui/assets/xam-addon-oxide.desktop \
    --icon-file crates/xam_gui/assets/icons/aircraft.svg \
    --output appimage

echo "AppImage build complete!"
