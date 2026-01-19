#!/bin/bash
set -e

# X-ADOX Automated Installation Script

echo "--- X-ADOX System Installation ---"

# Step 1: Build
echo "Building release binaries..."
cargo build --release --workspace

# Step 2: Install Binaries
echo "Installing binaries to /usr/local/bin/..."
sudo cp target/release/x-adox-gui /usr/local/bin/
sudo cp target/release/x-adox-cli /usr/local/bin/

# Step 3: Desktop Integration
echo "Installing desktop entry..."
sudo cp crates/x-adox-gui/assets/xam-addon-oxide.desktop /usr/share/applications/x-adox.desktop

# Step 4: Icons
echo "Installing icons..."
sudo mkdir -p /usr/share/icons/hicolor/1024x1024/apps
sudo cp assets/packaging/icon_1024.png /usr/share/icons/hicolor/1024x1024/apps/x-adox-gui.png
# Also install to a standard fallback location
sudo mkdir -p /usr/share/icons/hicolor/256x256/apps
magick assets/packaging/icon_1024.png -resize 256x256 icon_256.png
sudo mv icon_256.png /usr/share/icons/hicolor/256x256/apps/x-adox-gui.png

echo "----------------------------------"
echo "Installation Complete!"
echo "You can now launch X-ADOX from your application menu or run 'x-adox-gui'."
