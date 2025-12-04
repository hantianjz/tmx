#!/bin/bash
set -e

# Script to update the Homebrew formula in homebrew-tap repository
# Usage: ./update-homebrew-formula.sh <version> [tap-repo-path]
#
# Arguments:
#   version: The version/tag to update to (e.g., v0.1.0 or 0.1.0)
#   tap-repo-path: Path to the homebrew-tap repository (optional, defaults to ./homebrew-tap)
#
# Environment variables:
#   REPO_OWNER: GitHub repository owner (default: hantianjz)
#   REPO_NAME: GitHub repository name (default: tmx)
#   FORMULA_NAME: Homebrew formula name (default: tmx)

# Default values
REPO_OWNER="${REPO_OWNER:-hantianjz}"
REPO_NAME="${REPO_NAME:-tmx}"
FORMULA_NAME="${FORMULA_NAME:-tmx}"

# Check arguments
if [ $# -lt 1 ]; then
    echo "Usage: $0 <version> [tap-repo-path]"
    echo "Example: $0 v0.1.0"
    echo "Example: $0 0.1.0 ../homebrew-tap"
    exit 1
fi

TAG="$1"
TAP_REPO_PATH="${2:-./homebrew-tap}"

# Remove 'v' prefix if present for version
VERSION="${TAG#v}"

# Ensure TAG has 'v' prefix for GitHub URLs
if [[ ! "$TAG" =~ ^v ]]; then
    TAG="v${TAG}"
fi

echo "Updating Homebrew formula for ${REPO_NAME} to version ${VERSION}"
echo "Repository: ${REPO_OWNER}/${REPO_NAME}"
echo "Tag: ${TAG}"
echo "Tap repository path: ${TAP_REPO_PATH}"

# Download the release tarball to calculate SHA256
TARBALL_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/archive/refs/tags/${TAG}.tar.gz"
echo "Downloading tarball from: ${TARBALL_URL}"

TEMP_DIR=$(mktemp -d)
trap "rm -rf ${TEMP_DIR}" EXIT

curl -sL "$TARBALL_URL" -o "${TEMP_DIR}/${REPO_NAME}.tar.gz"

if [ ! -s "${TEMP_DIR}/${REPO_NAME}.tar.gz" ]; then
    echo "Error: Failed to download tarball or file is empty"
    exit 1
fi

SHA256=$(shasum -a 256 "${TEMP_DIR}/${REPO_NAME}.tar.gz" | awk '{print $1}')
echo "Calculated SHA256: ${SHA256}"

# Determine formula file path
FORMULA_FILE="${TAP_REPO_PATH}/Formula/${FORMULA_NAME}.rb"
FORMULA_DIR=$(dirname "$FORMULA_FILE")

# Create Formula directory if it doesn't exist
mkdir -p "$FORMULA_DIR"

echo "Updating formula at: ${FORMULA_FILE}"

# Create or update the formula
cat > "$FORMULA_FILE" << EOF
class Tmx < Formula
  desc "Terminal multiplexer wrapper and session manager"
  homepage "https://github.com/${REPO_OWNER}/${REPO_NAME}"
  url "${TARBALL_URL}"
  sha256 "${SHA256}"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/tmx", "--version"
  end
end
EOF

echo "Formula updated successfully!"
echo ""
echo "Next steps:"
echo "1. Review the changes: cat ${FORMULA_FILE}"
echo "2. Commit the changes: cd ${TAP_REPO_PATH} && git add ${FORMULA_FILE} && git commit -m 'Update ${FORMULA_NAME} to ${VERSION}'"
echo "3. Push to GitHub: cd ${TAP_REPO_PATH} && git push"
