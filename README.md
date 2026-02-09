# ethtui

`ethtui` is a small TUI that shows Ethernet link status and IP information in a layout similar to
other pythops TUIs (e.g. `impala`, `bluetui`).

## Goals

- Robust on "Ethernet-only" machines (desktops, VMs)
- No root required for read-only status
- Works with Omarchy's current integration model (run in terminal via `xdg-terminal-exec`)

## Scope

- Lists physical, non-wifi interfaces only (`/sys/class/net/*/device`, excluding `wireless/phy80211`).
- Read-only status always works without privileges.
- DHCP actions are best-effort and depend on your network stack (see below).

## Installation

### Binary release

Download the `ethtui-vX.Y.Z-x86_64.tar.gz` artifact from GitHub Releases and install `ethtui` to a
directory in your `$PATH` (e.g. `~/.local/bin`).

### Build from source

```bash
git clone https://github.com/skibidiandulka/ethtui
cd ethtui
cargo build --release
./target/release/ethtui
```

## Data Sources

To stay robust and avoid parsing shell output, `ethtui` reads:

- `/sys/class/net/*` for link state, carrier, MAC, speed
- `/proc/net/route` for IPv4 default gateway
- `getifaddrs(3)` (via `if-addrs`) for IP addresses
- `/etc/resolv.conf` for DNS servers

## Usage

Minimum terminal size is `80x24`.

Keys (vim-style, plus arrows):

- `j`/`k` or `↑`/`↓`: move selection
- `r`: refresh
- `n`: renew DHCP (best-effort)
- `q` or `Esc`: quit

## DHCP Renew Notes

When you press `n`, `ethtui` runs `networkctl renew <iface>` and shows a before/after snapshot
in-app. If nothing changes, it may still have renewed the lease (it's common for IP/GW/DNS to stay
the same).

If you are not running as root and `networkctl` requires privileges, `ethtui` will try `sudo -n`
(non-interactive). If that fails, you will see an error popup.

## Omarchy Integration

Omarchy typically launches TUIs with:

```bash
omarchy-launch-or-focus-tui ethtui
```

This opens the terminal with `--app-id=org.omarchy.ethtui`, allowing consistent window rules.
