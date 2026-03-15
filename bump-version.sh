#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new_version>"
    echo "  Version format: YYYY.M.D or YYYY.M.D-N"
    echo "  Example: $0 2026.3.15"
    exit 1
fi

NEW_VERSION="$1"

if ! echo "$NEW_VERSION" | grep -qE '^[0-9]{4}\.[0-9]{1,2}\.[0-9]{1,2}(-[0-9]+)?$'; then
    echo "Error: Invalid version format: $NEW_VERSION"
    echo "Expected: YYYY.M.D or YYYY.M.D-N"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

OLD_VERSION=$(cat VERSION)
echo "Bumping version: ${OLD_VERSION} -> ${NEW_VERSION}"

# Update VERSION file
echo "$NEW_VERSION" > VERSION

# Update Cargo.toml workspace version
sed -i "s/^version = \"${OLD_VERSION}\"/version = \"${NEW_VERSION}\"/" Cargo.toml

echo "Updated:"
echo "  VERSION"
echo "  Cargo.toml"
echo ""
echo "Done. Run 'cargo check' to verify."
