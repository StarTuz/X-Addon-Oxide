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

# 2. Package (Current Platform)
echo -e "${GREEN}Step 2: Packaging...${NC}"
if ! cargo packager --version &> /dev/null; then
    echo "cargo-packager not found. Installing..."
    cargo install cargo-packager --locked
fi

# Run packager
# We build for the local platform (binary) to verify TOML structure
cargo packager --release -p x-adox-gui --formats binary


# 3. Verify Artifacts
echo -e "${GREEN}Step 3: Verifying Artifacts...${NC}"
if [ -d "dist" ]; then
    echo "Contents of dist/:"
    ls -R dist/
    
    # Check for binary
    if find dist -name "x-adox-gui" | grep -q .; then
        echo -e "${GREEN}✓ Binary generated successfully.${NC}"
    else
        echo -e "${RED}✗ Binary not found in dist/.${NC}"
        exit 1
    fi

else
    echo -e "${RED}✗ dist/ directory not found.${NC}"
    exit 1
fi

echo -e "${GREEN}Local CI Pipeline Passed Successfully!${NC}"
