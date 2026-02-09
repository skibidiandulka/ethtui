# ethtui

`ethtui` is a small TUI that shows Ethernet link status and IP information in a layout similar to
other pythops TUIs (e.g. `impala`, `bluetui`).

## Goals

- Robust on "Ethernet-only" machines (desktops, VMs)
- No root required for read-only status
- Works with Omarchy's current integration model (run in terminal via `xdg-terminal-exec`)

## Data Sources

To stay robust and avoid parsing shell output, `ethtui` reads:

- `/sys/class/net/*` for link state, carrier, MAC, speed
- `/proc/net/route` for IPv4 default gateway
- `getifaddrs(3)` (via `if-addrs`) for IP addresses
- `/etc/resolv.conf` for DNS servers

## Run

```bash
cargo run
```

Keys:

- `j`/`k` or arrows: move selection
- `r`: refresh
- `q` or `Esc`: quit

## Omarchy Integration

Omarchy typically launches TUIs with:

```bash
omarchy-launch-or-focus-tui ethtui
```

This opens the terminal with `--app-id=org.omarchy.ethtui`, allowing consistent window rules.

