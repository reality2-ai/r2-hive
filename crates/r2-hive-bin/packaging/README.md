# r2-hive packaging

Operator-facing artefacts for shipping r2-hive on real systems.

```
packaging/
├── defaults/hive.toml                   # documented config example
├── systemd/r2-hive.service              # per-user systemd unit
└── dbus/org.reality2.Hive.service       # D-Bus activation file
```

## systemd

The unit at [`systemd/r2-hive.service`](systemd/r2-hive.service) is a
**per-user** unit (`WantedBy=default.target`), suitable for
`systemctl --user enable --now r2-hive`. Distros packaging r2-hive
should install it into `$prefix/lib/systemd/user/r2-hive.service`
(e.g. `/usr/lib/systemd/user/r2-hive.service`).

The unit uses `Type=notify` and `WatchdogSec=30s`. Both require the
binary to be built with `--features systemd`:

```sh
cargo build --release --features systemd
```

A binary built without the feature will still run under the unit, but
systemd will time out waiting for `READY=1` and the watchdog won't
function. Drop both `Type=notify` and `WatchdogSec=` — keep
`Type=simple` — if you need to ship a binary that doesn't link
`sd-notify`.

The unit hardens the daemon's runtime: `ProtectSystem=strict`,
`ProtectKernelModules=true`, `MemoryDenyWriteExecute=true`,
`RestrictNamespaces=true`, and a `SystemCallFilter` covering
`@system-service @network-io` minus `@privileged @resources`. None
of these block r2-hive's normal operation; they exist to limit the
blast radius of an in-process compromise.

## D-Bus activation

[`dbus/org.reality2.Hive.service`](dbus/org.reality2.Hive.service)
lets desktop tray applets and local clients **activate** the daemon
on demand:

```sh
dbus-send --session --print-reply \
    --dest=org.reality2.Hive \
    /org/reality2/Hive \
    org.freedesktop.DBus.Peer.Ping
```

This triggers the systemd user unit named in `SystemdService=`. Once
the daemon is up, the applet talks to it over the management UDS
at `${XDG_RUNTIME_DIR}/r2tgd.sock` — D-Bus is the activation
transport only, not the data path.

Install path:

- **System-wide:** `$prefix/share/dbus-1/services/org.reality2.Hive.service`
- **Per-user:** `~/.local/share/dbus-1/services/org.reality2.Hive.service`

The Cosmic and KDE first-boot applets (Phase 3f / 3g) will use this
to start the daemon if it isn't already running when the user opens
the tray icon.

## Distribution packaging

The `debian/`, `rpm/`, `archlinux/`, and `homebrew/` manifests for
Phase 7 will reference these files directly — there's no per-distro
fork. Each manifest installs `r2-hive` to `/usr/bin/`, the systemd
unit to `/usr/lib/systemd/user/`, the D-Bus service file to
`/usr/share/dbus-1/services/`, and the example config to
`/usr/share/r2/hive.toml`.
