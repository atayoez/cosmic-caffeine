#!/usr/bin/env bash
# cosmic-caffeine installer — places the panel-applet binary, settings
# binary, and icons under $XDG_DATA_HOME (or ~/.local/share). Per-user,
# no root required.
#
# cosmic-caffeine is a cosmic-panel applet, NOT a daemon. The panel
# spawns the binary as needed; this installer only deposits files.
#
# Cleans up artifacts from previous SNI/daemon installs (autostart
# entries, the old .desktop launcher, etc.) so the upgrade is hands-off.
#
# Usage:
#   ./install.sh             # build + install
#   ./install.sh --uninstall # remove everything this script wrote

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
cd "$SCRIPT_DIR"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"
APPS_DIR="$DATA_DIR/applications"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"

OWNED_FILES=(
    "$BIN_DIR/cosmic-caffeine"
    "$BIN_DIR/cosmic-caffeine-settings"
    "$ICON_DIR/cosmic-caffeine-symbolic.svg"
    "$ICON_DIR/cosmic-caffeine-active-symbolic.svg"
    "$APPS_DIR/cosmic-caffeine.desktop"
    "$APPS_DIR/cosmic-caffeine-settings.desktop"
    "$APPS_DIR/io.github.atayozcan.CosmicCaffeine.desktop"
    "$AUTOSTART_DIR/cosmic-caffeine.desktop"
)

clean_old_artifacts() {
    local removed=0
    for f in "${OWNED_FILES[@]}"; do
        if [[ -e "$f" ]]; then
            rm -f "$f" && removed=$((removed + 1))
        fi
    done
    if (( removed > 0 )); then
        echo "cosmic-caffeine: cleaned up $removed stale file(s)."
    fi
}

refresh_caches() {
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$DATA_DIR/icons/hicolor" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi
}

# Stop any running cosmic-caffeine (old SNI daemon and/or applet).
# pkill -x matches by the 15-char comm name "cosmic-caffein" for every
# invocation regardless of args, so this catches both old and new modes.
stop_running() {
    if pgrep -x cosmic-caffein >/dev/null 2>&1; then
        pkill -x cosmic-caffein 2>/dev/null || true
        for _ in 1 2 3 4 5; do
            pgrep -x cosmic-caffein >/dev/null 2>&1 || return 0
            sleep 0.2
        done
        pkill -9 -x cosmic-caffein 2>/dev/null || true
        sleep 0.2
    fi
}

uninstall() {
    echo "cosmic-caffeine: uninstalling..."
    stop_running
    clean_old_artifacts
    refresh_caches
    echo "cosmic-caffeine: uninstalled. (Remove the applet from your panel via cosmic-settings.)"
}

if [[ "${1:-}" == "--uninstall" ]]; then
    uninstall
    exit 0
fi

echo "cosmic-caffeine: building (cargo build --release)..."
cargo build --release

stop_running

echo "cosmic-caffeine: cleaning previous install..."
clean_old_artifacts

mkdir -p "$BIN_DIR" "$ICON_DIR" "$APPS_DIR"

install -m 0755 target/release/cosmic-caffeine "$BIN_DIR/cosmic-caffeine"
install -m 0755 target/release/cosmic-caffeine-settings "$BIN_DIR/cosmic-caffeine-settings"
install -m 0644 resources/icons/cosmic-caffeine-symbolic.svg "$ICON_DIR/cosmic-caffeine-symbolic.svg"
install -m 0644 resources/icons/cosmic-caffeine-active-symbolic.svg "$ICON_DIR/cosmic-caffeine-active-symbolic.svg"

# cosmic-panel discovers applets by their APP_ID-named .desktop file
# in $XDG_DATA_HOME/applications.
sed "s|@BIN@|$BIN_DIR/cosmic-caffeine|g" resources/cosmic-caffeine.desktop \
    > "$APPS_DIR/io.github.atayozcan.CosmicCaffeine.desktop"
chmod 0644 "$APPS_DIR/io.github.atayozcan.CosmicCaffeine.desktop"

refresh_caches

cat <<EOF
cosmic-caffeine: installed.

  Applet:   $BIN_DIR/cosmic-caffeine
  Settings: $BIN_DIR/cosmic-caffeine-settings
  Manifest: $APPS_DIR/io.github.atayozcan.CosmicCaffeine.desktop

To attach to the panel:
  cosmic-settings → Panel → <Top|Bottom|Dock> → Add Applet → Caffeine

To uninstall: ./install.sh --uninstall
EOF
