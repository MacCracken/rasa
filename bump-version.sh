#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new_version>"
    echo "  Version format: YYYY.M.D or YYYY.M.D-N"
    echo "  Example: $0 2026.3.18"
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

# 1. VERSION file
echo "$NEW_VERSION" > VERSION
echo "  Updated VERSION"

# 2. Cargo.toml workspace version
sed -i "s/^version = \"${OLD_VERSION}\"/version = \"${NEW_VERSION}\"/" Cargo.toml
echo "  Updated Cargo.toml"

# 3. Cargo.lock (regenerate from updated Cargo.toml)
cargo generate-lockfile --quiet 2>/dev/null || true
echo "  Updated Cargo.lock"

# 4. .agnos-agent.json
if [ -f .agnos-agent.json ]; then
    sed -i "s/\"version\": \"[0-9]\{4\}\.[0-9]*\.[0-9]*\"/\"version\": \"${NEW_VERSION}\"/" .agnos-agent.json
    echo "  Updated .agnos-agent.json"
fi

# 5. docs/development/roadmap.md
if [ -f docs/development/roadmap.md ]; then
    sed -i "s/> \*\*Version\*\*: [0-9]\{4\}\.[0-9]*\.[0-9]*/> **Version**: ${NEW_VERSION}/" docs/development/roadmap.md
    echo "  Updated docs/development/roadmap.md"
fi

echo ""
echo "Done. Version is now ${NEW_VERSION}."
echo "Run 'cargo check' to verify."
