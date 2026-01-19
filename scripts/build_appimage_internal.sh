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
mkdir -p AppDir/usr/share/icons/hicolor/512x512/apps
cp assets/packaging/icon_512.png AppDir/usr/share/icons/hicolor/512x512/apps/x-adox-gui.png

# Run linuxdeploy
linuxdeploy --appdir AppDir \
    --desktop-file crates/x-adox-gui/assets/xam-addon-oxide.desktop \
    --icon-file assets/packaging/icon_512.png \
    --output appimage

echo "AppImage build complete!"
