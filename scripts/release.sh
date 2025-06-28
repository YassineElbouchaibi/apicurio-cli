#!/bin/bash

# Release script for Apicurio CLI
# Usage: ./scripts/release.sh <version>

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.0"
    exit 1
fi

VERSION=$1

# Validate version format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: Version must be in format x.y.z (e.g., 0.1.0)"
    exit 1
fi

echo "Preparing release v$VERSION..."

# Check if working directory is clean
if ! git diff-index --quiet HEAD --; then
    echo "Error: Working directory is not clean. Please commit or stash changes."
    exit 1
fi

# Check if we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "Error: Must be on main branch to create release. Currently on: $CURRENT_BRANCH"
    exit 1
fi

# Update version in Cargo.toml
echo "Updating version in Cargo.toml..."
sed -i.bak "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update Cargo.lock
echo "Updating Cargo.lock..."
cargo check

# Update changelog
echo "Please update CHANGELOG.md to move items from [Unreleased] to [$VERSION] - $(date +%Y-%m-%d)"
echo "Press enter when changelog is updated..."
read -r

# Run tests
echo "Running tests..."
cargo test

# Run quality checks
echo "Running quality checks..."
cargo fmt --check
cargo clippy -- -D warnings

# Commit version bump
echo "Committing version bump..."
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: bump version to $VERSION"

# Create and push tag
echo "Creating and pushing tag v$VERSION..."
git tag "v$VERSION"
git push origin main
git push origin "v$VERSION"

echo "✅ Release v$VERSION created successfully!"
echo ""
echo "Next steps:"
echo "1. GitHub Actions will automatically build and publish the release"
echo "2. Monitor the Actions tab for build status"
echo "3. The crate will be published to crates.io automatically"
echo "4. Binary releases will be available on GitHub"
echo ""
echo "Release URL: https://github.com/YassineElbouchaibi/apicurio-cli/releases/tag/v$VERSION"
