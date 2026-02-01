#!/bin/bash
set -e

echo "🚀 Running X-Addon-Oxide Test Suite..."

echo "📦 Checking core library..."
cargo check -p x-adox-core
cargo test -p x-adox-core

echo "📦 Checking bitnet heuristics..."
cargo check -p x-adox-bitnet
cargo test -p x-adox-bitnet

echo "📦 Checking GUI application..."
cargo check -p x-adox-gui

echo "🧪 Running all tests..."
cargo test --all-targets

echo "✅ All checks passed!"
