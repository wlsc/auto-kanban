#!/bin/bash

set -e  # Exit on any error

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map architecture names
case "$ARCH" in
  x86_64)
    ARCH="x64"
    ;;
  arm64|aarch64)
    ARCH="arm64"
    ;;
  *)
    echo "⚠️  Warning: Unknown architecture $ARCH, using as-is"
    ;;
esac

# Map OS names
case "$OS" in
  linux)
    OS="linux"
    ;;
  darwin)
    OS="macos"
    ;;
  *)
    echo "⚠️  Warning: Unknown OS $OS, using as-is"
    ;;
esac

PLATFORM="${OS}-${ARCH}"

# Set CARGO_TARGET_DIR if not defined
if [ -z "$CARGO_TARGET_DIR" ]; then
  CARGO_TARGET_DIR="target"
fi

echo "🔍 Detected platform: $PLATFORM"
echo "🔧 Using target directory: $CARGO_TARGET_DIR"

# Set API base URL for remote features
export AK_SHARED_API_BASE="https://api.autokanban.dev"
export VITE_AK_SHARED_API_BASE="https://api.autokanban.dev"

echo "🧹 Cleaning previous builds..."
rm -rf npx-cli/dist
mkdir -p npx-cli/dist/$PLATFORM

echo "🔨 Building frontend..."
(cd frontend && npm run build)

echo "🔨 Building Rust binaries..."
cargo build --release --manifest-path Cargo.toml
cargo build --release --bin mcp_task_server --manifest-path Cargo.toml

echo "📦 Creating distribution package..."

# Copy the main binary
cp ${CARGO_TARGET_DIR}/release/server auto-kanban
zip -q auto-kanban.zip auto-kanban
rm -f auto-kanban 
mv auto-kanban.zip npx-cli/dist/$PLATFORM/auto-kanban.zip

# Copy the MCP binary
cp ${CARGO_TARGET_DIR}/release/mcp_task_server auto-kanban-mcp
zip -q auto-kanban-mcp.zip auto-kanban-mcp
rm -f auto-kanban-mcp
mv auto-kanban-mcp.zip npx-cli/dist/$PLATFORM/auto-kanban-mcp.zip

# Copy the Review CLI binary
cp ${CARGO_TARGET_DIR}/release/review auto-kanban-review
zip -q auto-kanban-review.zip auto-kanban-review
rm -f auto-kanban-review
mv auto-kanban-review.zip npx-cli/dist/$PLATFORM/auto-kanban-review.zip

echo "✅ Build complete!"
echo "📁 Files created:"
echo "   - npx-cli/dist/$PLATFORM/auto-kanban.zip"
echo "   - npx-cli/dist/$PLATFORM/auto-kanban-mcp.zip"
echo "   - npx-cli/dist/$PLATFORM/auto-kanban-review.zip"
echo ""
echo "🚀 To test locally, run:"
echo "   cd npx-cli && node bin/cli.js"
