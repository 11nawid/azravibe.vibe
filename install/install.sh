#!/bin/sh
# Azra one-command installer for Linux/macOS.
#   curl -fsSL https://raw.githubusercontent.com/11nawid/azravibe.vibe/main/install/install.sh | sh
set -eu

REPO="11nawid/azravibe.vibe"
case "$(uname -s)" in
  Linux)  ASSET="azra-linux-x86_64.tar.gz" ;;
  Darwin) ASSET="azra-macos-x86_64.tar.gz" ;;
  *) echo "Unsupported platform: $(uname -s)" >&2; exit 1 ;;
esac

DEST="$HOME/.azra/bin"
TMP="$(mktemp -d)"

echo "Finding the latest Azra release..."
PATTERN="azra-$(uname -s | tr '[:upper:]' '[:lower:]' | sed 's/darwin/macos/')-x86_64"
case "$(uname -s)" in Linux) PATTERN="azra-linux-x86_64" ;; Darwin) PATTERN="azra-macos-x86_64" ;; esac
API="https://api.github.com/repos/$REPO/releases/latest"
URL=$(curl -fsSL "$API" | grep -o "\"browser_download_url\": \"[^\"]*${PATTERN}[^\"]*tar.gz\"" | head -n 1 | cut -d '"' -f 4 || true)
if [ -z "$URL" ]; then
  URL="https://github.com/$REPO/releases/latest/download/$ASSET"
fi

echo "Downloading Azra..."
curl -fsSL "$URL" -o "$TMP/azra.tar.gz"
mkdir -p "$DEST"
tar -xzf "$TMP/azra.tar.gz" -C "$DEST"
chmod +x "$DEST/azra" 2>/dev/null || true

"$DEST/azra" setup-path || true
export PATH="$DEST:$PATH"

"$DEST/azra" --version || true

echo "Installing the Azra VS Code extension..."
"$DEST/azra" setup-vscode || echo "Extension setup skipped; run 'azra setup-vscode' later."

rm -rf "$TMP"
echo "Done! Run: azra run app.vibe"
