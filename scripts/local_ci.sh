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

# 2. Run Tests
echo -e "${GREEN}Step 2: Running Tests...${NC}"
cargo test

# 3. Verify Binary
echo -e "${GREEN}Step 3: Verifying Binary...${NC}"
BINARY="target/release/x-adox-gui"
if [ -f "$BINARY" ]; then
    echo -e "${GREEN}✓ Binary exists at $BINARY${NC}"
    ls -lh "$BINARY"
else
    echo -e "${RED}✗ Binary not found at $BINARY${NC}"
    exit 1
fi

echo -e "${GREEN}Local CI Pipeline Passed Successfully!${NC}"
