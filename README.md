# cosmic-caffeine

> Status: **proof of concept**. Tray icon toggles a real logind inhibit
> lock; settings GUI persists preferences via `cosmic_config`. No
> keyboard shortcut yet, no app-aware "block while window X is focused"
> yet.

A Wayland-native idle/sleep inhibitor with a `StatusNotifierItem` tray
icon and a libcosmic settings GUI. Built to fill a gap in the COSMIC
desktop — works under any DE that consumes
`org.kde.StatusNotifierItem` (KDE, Sway+waybar, Hyprland, COSMIC).

Sibling project to [`tb-tray`](https://github.com/atayozcan/tb-tray)
and [`cosmic-clip`](https://github.com/atayozcan/cosmic-clip); shares
the [`cosmic-tray-app`](https://github.com/atayozcan/cosmic-tray-app)
helper crate for paths, autostart, and the single-binary
`--settings`-re-exec pattern.

## What it does

- Click the cup → acquires
  `org.freedesktop.login1.Manager.Inhibit("idle:sleep", "block")`
- Click again → drops the lock
- Tray menu: pick "On for 5 / 30 / 60 min" or "On indefinitely";
  also exposes Settings… and Quit
- Settings GUI: default duration, choose what to inhibit (idle, sleep,
  or both), notification on toggle, autostart

## What it does not (yet) do

- No global hotkey
- No "auto-on while application X is running" rules
- No fractional minutes / seconds-level granularity
- No D-Bus interface for other apps to drive it

These are POC limits, not by-design. PRs welcome.

## How the inhibit actually works

We take **two** locks in tandem, because no single API covers both
"don't suspend" and "don't blank the screen" on a typical Wayland
session:

- `org.freedesktop.login1.Manager.Inhibit` on the **system bus**
  returns a file descriptor; holding the FD blocks automatic suspend.
  This is the same mechanism `systemd-inhibit(1)` uses.
- `org.freedesktop.ScreenSaver.Inhibit` on the **session bus**
  (Wayland compositors, X screensavers, and most desktops honor it)
  returns a cookie tied to the bus connection; holding the connection
  blocks screen blanking / lock-on-idle. cosmic-comp, mutter, kwin,
  and sway all manage blanking themselves and don't act on logind's
  `idle` inhibit class — so the ScreenSaver call is what actually
  keeps the screen on.

Both locks are dropped when the user toggles caffeine off (or the
duration timer fires). Missing ScreenSaver service is a non-fatal
no-op; the suspend lock still works.

## Install

```sh
git clone https://github.com/atayozcan/cosmic-caffeine
git clone https://github.com/atayozcan/cosmic-tray-app  # sibling lib (path dep)
cd cosmic-caffeine
./install.sh
```

That installs:

| Path | What |
| --- | --- |
| `~/.local/bin/cosmic-caffeine` | the binary (daemon + settings GUI in one) |
| `~/.local/share/icons/hicolor/scalable/apps/cosmic-caffeine{,-active}-symbolic.svg` | tray icons |
| `~/.local/share/applications/cosmic-caffeine.desktop` | app-menu launcher |

Per-user, no root needed. The launcher's `Exec=` is templated with
the absolute binary path at install time so it keeps working even
when your desktop session's PATH doesn't include `~/.local/bin`. The
script cleans up artifacts from earlier installs (the obsolete
second `cosmic-caffeine-settings` binary, its launcher, etc.) before
laying down the new files, and (re)starts the daemon so the new
version is live immediately.

To uninstall:

```sh
./uninstall.sh
```

### Build deps (Arch)

```sh
pkexec pacman -S --needed rust pkgconf libxkbcommon wayland mesa \
    vulkan-icd-loader fontconfig freetype2
```

## Run

```sh
cosmic-caffeine &
```

Or enable the *Start cosmic-caffeine on login* toggle in settings.

## Config

Stored via `cosmic_config` at
`~/.config/cosmic/io.github.atayozcan.CosmicCaffeine/v1/`, one
RON-encoded file per field. Fields:

| Field | Type | Default | What |
| --- | --- | --- | --- |
| `default_minutes` | u32 | `0` | Click-default duration; 0 = indefinite |
| `inhibit_idle` | bool | `true` | Block screen blanking / lock-on-idle |
| `inhibit_sleep` | bool | `true` | Block automatic suspend |
| `notify_on_toggle` | bool | `false` | Show a notification on toggle |

Changes from the settings GUI propagate to the running daemon
without a restart — config is re-read on each tray click via
`cosmic_config`.

## License

MIT — see `LICENSE`.
