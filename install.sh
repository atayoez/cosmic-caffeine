#!/usr/bin/env bash
# cosmic-caffeine installer — places binaries, icons, and desktop entries
# under $XDG_DATA_HOME (or ~/.local/share). Per-user, no root required.
#
# Usage:
#   ./install.sh             # builds release + installs both binaries
#   ./install.sh --uninstall # removes everything this script wrote

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
cd "$SCRIPT_DIR"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"
APPS_DIR="$DATA_DIR/applications"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"

uninstall() {
    echo "cosmic-caffeine: uninstalling..."
    rm -f "$BIN_DIR/cosmic-caffeine" "$BIN_DIR/cosmic-caffeine-settings"
    rm -f "$ICON_DIR/cosmic-caffeine-symbolic.svg" "$ICON_DIR/cosmic-caffeine-active-symbolic.svg"
    rm -f "$APPS_DIR/cosmic-caffeine.desktop" "$APPS_DIR/cosmic-caffeine-settings.desktop"
    rm -f "$AUTOSTART_DIR/cosmic-caffeine.desktop"
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$DATA_DIR/icons/hicolor" 2>/dev/null || true
    fi
    echo "cosmic-caffeine: uninstalled."
}

if [[ "${1:-}" == "--uninstall" ]]; then
    uninstall
    exit 0
fi

echo "cosmic-caffeine: building (cargo build --release)..."
cargo build --release

mkdir -p "$BIN_DIR" "$ICON_DIR" "$APPS_DIR"

install -m 0755 target/release/cosmic-caffeine "$BIN_DIR/cosmic-caffeine"
install -m 0755 target/release/cosmic-caffeine-settings "$BIN_DIR/cosmic-caffeine-settings"
install -m 0644 resources/icons/cosmic-caffeine-symbolic.svg "$ICON_DIR/cosmic-caffeine-symbolic.svg"
install -m 0644 resources/icons/cosmic-caffeine-active-symbolic.svg "$ICON_DIR/cosmic-caffeine-active-symbolic.svg"

sed "s|@BIN@|$BIN_DIR/cosmic-caffeine|" resources/cosmic-caffeine.desktop \
    > "$APPS_DIR/cosmic-caffeine.desktop"
sed "s|@BIN@|$BIN_DIR/cosmic-caffeine-settings|" resources/cosmic-caffeine-settings.desktop \
    > "$APPS_DIR/cosmic-caffeine-settings.desktop"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$DATA_DIR/icons/hicolor" 2>/dev/null || true
fi

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "cosmic-caffeine: warning: $BIN_DIR is not in PATH; add it to your shell rc." ;;
esac

cat <<EOF
cosmic-caffeine: installed.

  Daemon:   $BIN_DIR/cosmic-caffeine
  Settings: $BIN_DIR/cosmic-caffeine-settings
  Icons:    $ICON_DIR/cosmic-caffeine{,-active}-symbolic.svg
  Launchers: $APPS_DIR/cosmic-caffeine{,-settings}.desktop

Next steps:
  - Run 'cosmic-caffeine' to start the tray daemon.
  - Click the cup to toggle idle/sleep inhibition.
  - Run 'cosmic-caffeine-settings' to set default duration / autostart.
EOF
