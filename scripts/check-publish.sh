#!/bin/bash

# Pre-publish check script for Apicurio CLI
# This script validates that everything is ready for publishing

set -e

echo "🔍 Running pre-publish checks..."
echo ""

# Check if Cargo.toml has all required fields
echo "📋 Checking Cargo.toml metadata..."
if ! grep -q "authors.*Yassine El Bouchaibi" Cargo.toml; then
    echo "❌ Missing or incorrect author in Cargo.toml"
    exit 1
fi

if ! grep -q "repository.*github.com/YassineElbouchaibi/apicurio-cli" Cargo.toml; then
    echo "❌ Missing or incorrect repository URL in Cargo.toml"
    exit 1
fi

if ! grep -q "license.*MIT OR Apache-2.0" Cargo.toml; then
    echo "❌ Missing or incorrect license in Cargo.toml"
    exit 1
fi

echo "✅ Cargo.toml metadata looks good"
echo ""

# Check if README exists
echo "📖 Checking documentation..."
if [ ! -f "README.md" ]; then
    echo "❌ README.md is missing"
    exit 1
fi

if [ ! -f "CHANGELOG.md" ]; then
    echo "❌ CHANGELOG.md is missing"
    exit 1
fi

echo "✅ Documentation files present"
echo ""

# Check if code compiles
echo "🔨 Checking compilation..."
if ! cargo check --release; then
    echo "❌ Code does not compile"
    exit 1
fi

echo "✅ Code compiles successfully"
echo ""

# Check if tests pass
echo "🧪 Running tests..."
if ! cargo test; then
    echo "❌ Tests failed"
    exit 1
fi

echo "✅ Tests pass"
echo ""

# Check formatting
echo "🎨 Checking code formatting..."
if ! cargo fmt --check; then
    echo "❌ Code is not formatted. Run: cargo fmt"
    exit 1
fi

echo "✅ Code is properly formatted"
echo ""

# Check linting
echo "🔍 Running linter..."
if ! cargo clippy -- -D warnings; then
    echo "❌ Linter found issues"
    exit 1
fi

echo "✅ Linter checks pass"
echo ""

# Check if dry-run publish works
echo "📦 Testing publish (dry run)..."
if ! cargo publish --dry-run; then
    echo "❌ Publish dry run failed"
    exit 1
fi

echo "✅ Publish dry run successful"
echo ""

# Check for secrets that need to be set
echo "🔑 Checking required secrets..."
echo "Make sure these secrets are set in GitHub:"
echo "  - CARGO_TOKEN (for crates.io publishing)"
echo "  - CODECOV_TOKEN (optional, for code coverage)"
echo ""

# Check git status
echo "📝 Checking git status..."
if ! git diff-index --quiet HEAD --; then
    echo "⚠️  Working directory has uncommitted changes"
    echo "   Consider committing changes before release"
fi

if [ "$(git branch --show-current)" != "main" ]; then
    echo "⚠️  Not on main branch (currently on: $(git branch --show-current))"
    echo "   Consider switching to main branch for release"
fi

echo ""
echo "🎉 All checks passed! Ready for publishing."
echo ""
echo "To publish:"
echo "1. Run: ./scripts/release.sh <version>"
echo "2. Or manually: git tag v<version> && git push origin v<version>"
echo ""
echo "The GitHub Actions will handle:"
echo "- Building release binaries"
echo "- Publishing to crates.io"
echo "- Creating GitHub release"
