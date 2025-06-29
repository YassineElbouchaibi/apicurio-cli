#!/bin/bash

# Release script for Apicurio CLI
# Usage: ./scripts/release.sh <version|patch|minor|major>

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version|patch|minor|major>"
    echo "Examples:"
    echo "  $0 0.1.0      # Set specific version"
    echo "  $0 patch      # Bump patch version (0.1.3 -> 0.1.4)"
    echo "  $0 minor      # Bump minor version (0.1.3 -> 0.2.0)"
    echo "  $0 major      # Bump major version (0.1.3 -> 1.0.0)"
    exit 1
fi

INPUT=$1

# Function to get current version from Cargo.toml
get_current_version() {
    grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/'
}

# Function to bump version based on type
bump_version() {
    local current_version=$1
    local bump_type=$2
    
    IFS='.' read -r major minor patch <<< "$current_version"
    
    case "$bump_type" in
        "major")
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        "minor")
            minor=$((minor + 1))
            patch=0
            ;;
        "patch")
            patch=$((patch + 1))
            ;;
        *)
            echo "Error: Invalid bump type: $bump_type"
            exit 1
            ;;
    esac
    
    echo "$major.$minor.$patch"
}

# Determine the target version
if [[ "$INPUT" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    # Specific version provided
    VERSION=$INPUT
elif [[ "$INPUT" =~ ^(patch|minor|major)$ ]]; then
    # Bump type provided
    CURRENT_VERSION=$(get_current_version)
    if [ -z "$CURRENT_VERSION" ]; then
        echo "Error: Could not determine current version from Cargo.toml"
        exit 1
    fi
    echo "Current version: $CURRENT_VERSION"
    VERSION=$(bump_version "$CURRENT_VERSION" "$INPUT")
    echo "Bumping $INPUT version: $CURRENT_VERSION -> $VERSION"
else
    echo "Error: Argument must be either a version number (x.y.z) or bump type (patch|minor|major)"
    echo "Examples:"
    echo "  $0 0.1.0      # Set specific version"
    echo "  $0 patch      # Bump patch version"
    echo "  $0 minor      # Bump minor version"
    echo "  $0 major      # Bump major version"
    exit 1
fi

echo "Preparing release v$VERSION..."

# Validate final version format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: Final version is not in correct format: $VERSION"
    exit 1
fi

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
