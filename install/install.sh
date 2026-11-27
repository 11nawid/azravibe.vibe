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

URL="https://github.com/$REPO/releases/latest/download/$ASSET"
DEST="$HOME/.azra/bin"
TMP="$(mktemp -d)"

echo "Downloading Azra..."
curl -fsSL "$URL" -o "$TMP/azra.tar.gz"
mkdir -p "$DEST"
tar -xzf "$TMP/azra.tar.gz" -C "$DEST"
chmod +x "$DEST/azra" "$DEST/azra-lsp" 2>/dev/null || true

case ":$PATH:" in
  *":$DEST:"*) ;;
  *)
    for rc in "$HOME/.profile" "$HOME/.bashrc" "$HOME/.zshrc"; do
      [ -f "$rc" ] || touch "$rc"
      printf '\nexport PATH="$HOME/.azra/bin:$PATH" # Azra\n' >> "$rc"
    done
    echo "Added $DEST to PATH. Restart your shell (or run: export PATH=\"$DEST:\$PATH\")."
    ;;
esac

"$DEST/azra" --version || true

if command -v code >/dev/null 2>&1; then
  VSIX="$DEST/azra-vibe.vsix"
  if [ -f "$VSIX" ]; then
    echo "Installing the Azra VS Code extension..."
    code --install-extension "$VSIX" --force >/dev/null 2>&1 || true
    echo "VS Code extension installed."
  fi
else
  echo "VS Code CLI not found; highlighting can be set up later from editors/vscode."
fi

rm -rf "$TMP"
echo "Done! Run: azra run app.vibe"
