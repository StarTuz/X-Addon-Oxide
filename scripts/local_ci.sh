#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "Starting Local CI Pipeline..."

# 1. Build
echo -e "${GREEN}Step 1: Building Release...${NC}"
cargo build --release -p x-adox-gui

# 2. Package (Linux only for now)
echo -e "${GREEN}Step 2: Packaging (Linux/AppImage)...${NC}"
if ! cargo packager --version &> /dev/null; then
    echo "cargo-packager not found. Installing..."
    cargo install cargo-packager --locked
fi

# Run packager
# We build only AppImage locally to save time and verify config structure
cargo packager --release -p x-adox-gui --formats appimage

# 3. Verify Artifacts
echo -e "${GREEN}Step 3: Verifying Artifacts...${NC}"
if [ -d "dist" ]; then
    echo "Contents of dist/:"
    ls -R dist/
    
    # Check for AppImage
    if find dist -name "*.AppImage" | grep -q .; then
        echo -e "${GREEN}✓ AppImage generated successfully.${NC}"
    else
        echo -e "${RED}✗ AppImage not found in dist/.${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ dist/ directory not found.${NC}"
    exit 1
fi

echo -e "${GREEN}Local CI Pipeline Passed Successfully!${NC}"
