#!/bin/bash
# Build script for RustyDB - Compiles for both Linux and Windows
set -e

echo "========================================"
echo "  RustyDB Multi-Platform Build Script"
echo "========================================"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Load cargo environment
source "$HOME/.cargo/env"

# Clean previous builds (optional)
if [ "$1" == "clean" ]; then
    echo -e "${BLUE}Cleaning previous builds...${NC}"
    cargo clean
    rm -rf builds/
    echo ""
fi

# Build for Linux
echo -e "${BLUE}Building for Linux (x86_64-unknown-linux-gnu)...${NC}"
cargo build --release --target x86_64-unknown-linux-gnu
echo -e "${GREEN}✓ Linux build complete${NC}"
echo ""

# Build for Windows
echo -e "${BLUE}Building for Windows (x86_64-pc-windows-gnu)...${NC}"
cargo build --release --target x86_64-pc-windows-gnu
echo -e "${GREEN}✓ Windows build complete${NC}"
echo ""

# Organize binaries
echo -e "${BLUE}Organizing binaries...${NC}"
mkdir -p builds/{linux,windows}
cp target/x86_64-unknown-linux-gnu/release/rusty-db-{cli,server} builds/linux/
cp target/x86_64-pc-windows-gnu/release/rusty-db-{cli,server}.exe builds/windows/
chmod +x builds/linux/*
echo -e "${GREEN}✓ Binaries organized in builds/ directory${NC}"
echo ""

# Show results
echo "========================================"
echo "  Build Summary"
echo "========================================"
tree -h builds/ 2>/dev/null || ls -lhR builds/
echo ""
echo -e "${GREEN}Build completed successfully!${NC}"
echo ""
echo "Linux binaries:"
echo "  - builds/linux/rusty-db-server"
echo "  - builds/linux/rusty-db-cli"
echo ""
echo "Windows binaries:"
echo "  - builds/windows/rusty-db-server.exe"
echo "  - builds/windows/rusty-db-cli.exe"
