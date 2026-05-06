# cosmic-caffeine

> Status: **proof of concept**. Tray icon toggles a real logind inhibit
> lock; settings GUI persists preferences. No keyboard shortcut yet,
> no app-aware "block while window X is focused" yet.

A Wayland-native idle/sleep inhibitor with a `StatusNotifierItem` tray
icon and a libcosmic settings GUI. Built to fill a gap in the COSMIC
desktop — works under any DE that consumes
`org.kde.StatusNotifierItem` (KDE, Sway+waybar, Hyprland, COSMIC).

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

systemd-logind exposes
[`org.freedesktop.login1.Manager.Inhibit`](https://www.freedesktop.org/wiki/Software/systemd/inhibit/)
on the system bus. Calling it returns a file descriptor that holds the
lock — close the FD and logind cancels it. We hold the FD on the tray
state for as long as the user wants caffeine on. This is the same
mechanism `systemd-inhibit(1)` uses, just driven from inside the
daemon over `zbus`.

## Install

```sh
git clone https://github.com/atayozcan/cosmic-caffeine
cd cosmic-caffeine
./install.sh
```

That installs:

| Path | What |
| --- | --- |
| `~/.local/bin/cosmic-caffeine` | tray daemon |
| `~/.local/bin/cosmic-caffeine-settings` | libcosmic settings GUI |
| `~/.local/share/icons/hicolor/scalable/apps/cosmic-caffeine{,-active}-symbolic.svg` | tray icons |
| `~/.local/share/applications/cosmic-caffeine{,-settings}.desktop` | app-menu launchers |

`./install.sh --uninstall` removes everything the installer wrote.

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

`~/.config/cosmic-caffeine/config.toml`:

```toml
default_minutes  = 0      # 0 = indefinite
inhibit_idle     = true
inhibit_sleep    = true
notify_on_toggle = false
```

## License

MIT — see `LICENSE`.
