#!/bin/sh
# installer/unix/install.sh
#
# Installs Liphia for the current user on Linux (x86_64) and macOS (Apple
# silicon), without root:
#
#   curl -fsSL https://raw.githubusercontent.com/shferreira-lab/liphia/main/installer/unix/install.sh | sh
#
# A specific version:
#
#   curl -fsSL https://raw.githubusercontent.com/shferreira-lab/liphia/main/installer/unix/install.sh | LIPHIA_VERSION=2.0.0 sh
#
# What it does: finds the engine release (tags vX.Y.Z; package releases
# are skipped), downloads liphia-<version>-<platform>.tar.gz, installs the
# executables to ~/.liphia/bin and adds that folder to PATH in the shell
# startup files. Running it again upgrades in place.

set -eu

REPO="shferreira-lab/liphia"
LIPHIA_HOME="${LIPHIA_HOME:-$HOME/.liphia}"
BIN_DIR="$LIPHIA_HOME/bin"

fail() {
    echo "liphia install: $*" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || fail "'$1' is required"
}

need curl
need tar

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)            PLATFORM="linux-x86_64" ;;
    Darwin-arm64)            PLATFORM="macos-aarch64" ;;
    Darwin-x86_64)           fail "Intel Macs are not supported yet (only Apple silicon)" ;;
    *)                       fail "unsupported platform: $(uname -s) $(uname -m)" ;;
esac

# Releases are listed newest first; engine tags are "v<semver>".
if [ -n "${LIPHIA_VERSION:-}" ]; then
    VERSION="${LIPHIA_VERSION#v}"
else
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases?per_page=50" \
        | grep -o '"tag_name": *"v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*"' \
        | head -n 1 \
        | sed 's/.*"v\([^"]*\)"/\1/')
    [ -n "$VERSION" ] || fail "no Liphia engine release found in $REPO"
fi

NAME="liphia-$VERSION-$PLATFORM"
URL="https://github.com/$REPO/releases/download/v$VERSION/$NAME.tar.gz"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Installing Liphia $VERSION"
echo "  from $URL"
echo "  to   $LIPHIA_HOME"

curl -fsSL "$URL" -o "$TMP/$NAME.tar.gz" || fail "download failed: $URL"
tar -xzf "$TMP/$NAME.tar.gz" -C "$TMP"

mkdir -p "$BIN_DIR"
for exe in liphia liphia-gui; do
    if [ -f "$TMP/$NAME/$exe" ]; then
        cp "$TMP/$NAME/$exe" "$BIN_DIR/$exe"
        chmod +x "$BIN_DIR/$exe"
    fi
done
for doc in LICENSE.txt THIRD_PARTY_LICENSES.txt README.md; do
    [ -f "$TMP/$NAME/$doc" ] && cp "$TMP/$NAME/$doc" "$LIPHIA_HOME/"
done

# Binaries fetched with curl carry no quarantine flag, but clear it anyway
# in case the archive was downloaded by a browser before.
if [ "$(uname -s)" = "Darwin" ]; then
    xattr -d com.apple.quarantine "$BIN_DIR/liphia" 2>/dev/null || true
    xattr -d com.apple.quarantine "$BIN_DIR/liphia-gui" 2>/dev/null || true
fi

# PATH: one marked line per startup file, added only once. Files are
# created for the user's own shell and only updated for the others when
# they already exist.
LINE="export PATH=\"$BIN_DIR:\$PATH\"  # liphia"
add_line() {
    file="$1"
    create="$2"
    if [ ! -f "$file" ] && [ "$create" != "yes" ]; then
        return
    fi
    if [ -f "$file" ] && grep -q "# liphia" "$file"; then
        return
    fi
    mkdir -p "$(dirname "$file")"
    printf '\n%s\n' "$LINE" >> "$file"
    echo "  added $BIN_DIR to PATH in $file"
}

SHELL_NAME=$(basename "${SHELL:-sh}")
[ "$SHELL_NAME" = "bash" ] && add_line "$HOME/.bashrc" yes || add_line "$HOME/.bashrc" no
[ "$SHELL_NAME" = "zsh" ] && add_line "$HOME/.zshrc" yes || add_line "$HOME/.zshrc" no
add_line "$HOME/.profile" no

FISH_CONF="$HOME/.config/fish/conf.d/liphia.fish"
if [ "$SHELL_NAME" = "fish" ] && [ ! -f "$FISH_CONF" ]; then
    mkdir -p "$(dirname "$FISH_CONF")"
    echo "fish_add_path $BIN_DIR  # liphia" > "$FISH_CONF"
    echo "  added $BIN_DIR to PATH in $FISH_CONF"
fi

echo ""
"$BIN_DIR/liphia" version
echo ""
echo "Liphia $VERSION is installed. Open a new terminal, or run:"
echo "  export PATH=\"$BIN_DIR:\$PATH\""
