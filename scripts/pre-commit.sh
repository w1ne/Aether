#!/bin/bash
set -e

echo "🔍 Running Pre-Commit Checks..."

# 1. Format Check
echo "🎨 Checking Formatting..."
cargo fmt -- --check

# 2. Clippy
echo "📎 Running Clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "✅ All checks passed!"
