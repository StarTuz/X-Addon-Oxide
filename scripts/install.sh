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
sudo mkdir -p /usr/share/icons/hicolor/scalable/apps
sudo cp crates/x-adox-gui/assets/icons/aircraft.svg /usr/share/icons/hicolor/scalable/apps/x-adox-gui.svg

echo "----------------------------------"
echo "Installation Complete!"
echo "You can now launch X-ADOX from your application menu or run 'x-adox-gui'."
