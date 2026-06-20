# r2.hw.led — general LED / indicator capability (driver-informed draft for specs/core)

Driver-informed: the DFR1195 mono LED (GPIO21) heartbeat is working on real hardware — that's the ground
truth. Per Roy/supervisor: a **small, SEMANTIC** vocabulary (status *meanings*, not raw blink codes),
`kind:mono|rgb` descriptor, device-agnostic + calm. **CRITICAL: the LED path is INDEPENDENT of the display**
— it must signal OTA/status when the screen is down, so it cannot depend on the render plugin or any sentant
being up. Split: hive owns the device drivers; composer owns the StatusDisplay sentant; **specs/core
canonicalize this trait + descriptor**.

## Capability — one semantic command

The node exposes a single indicator command: **set the semantic STATUS**. The device driver maps that status
to the concrete rendering (a blink pattern on a mono LED, colour+pattern on RGB, a coloured dot on a browser
canvas). The caller says **WHAT** (the status meaning); the driver owns **HOW** (the per-device expression).

```
CMD_SET_STATUS { status: u8 }      # tiny — a single semantic enum value (R2-CBOR int or raw byte)
```

No raw blink/colour codes on the wire — that keeps it device-agnostic and calm (a node with an RGB LED, a
mono LED, or a screen-dot all honour the same `ota` status, each in its own idiom).

## Semantic status vocabulary (small + calm — status MEANINGS)

| status | meaning | mono default | rgb default |
|---|---|---|---|
| `ok` | all-well, operating normally | gentle heartbeat (lub-dub) | green heartbeat |
| `joining` | provisioning / not yet a TG member | slow even blink | amber slow pulse |
| `ota` | firmware update in progress | fast even blink | cyan fast |
| `error` | fault / needs attention | rapid burst (SOS-like) | red |
| `identify` | locate-this-node (operator ping) | solid for N seconds | white solid |
| `idle` / `off` | quiescent / display-only mode | off | off |

Calm-tech: `ok` is subtle (a slow heartbeat you barely notice); `error`/`identify` demand attention. Six
meanings, not a blink-code zoo. New statuses are added deliberately, not per-app.

## Descriptor (device-agnostic capability advertisement)

```
r2.hw.led:
  kind:       "mono" | "rgb"          # the indicator's physical nature
  statuses:   [ok, joining, ota, error, identify, idle]   # the set this driver renders
  dimmable:   bool                    # PWM brightness available
  colours:    { ok: green, ota: cyan, ... }   # rgb only — per-status colour slots, overridable
```
- **DFR1195:** `{ kind: mono, statuses: [...], dimmable: true (GPIO21 PWM) }`.
- **workshop WS2812 boards:** `{ kind: rgb, statuses, colours: {...} }`.
- **browser hive:** `{ kind: rgb (canvas dot) }`.
Same capability, per-board drivers — the LoRaRadio / display pattern.

## CRITICAL — independent of the display (Roy)

The whole point is signalling **when the screen is down** (during OTA, on a render failure, before the UI is
up). So the LED capability is a **separate, always-available output**, not routed through the render pipeline:

- **Firmware-direct base statuses.** The boot path sets `joining`→`ok`; the **OTA receiver** sets `ota` while
  flashing (and the display may legitimately be off then); the panic/error path sets `error` — all
  **without** any display plugin or sentant. The LED is the last thing standing.
- The StatusDisplay sentant MAY *refine* the status (richer policy from node telemetry), but `r2.hw.led` does
  **not depend** on the display render plugin or the sentant being alive.
- Two independent output capabilities (`r2.hw.led`, `r2.hw.display`); one sentant drives both, but each works
  standalone. (Firmware implication, already noted for hive: initialise + drive the LED *before/around* the
  display so a display fault never silences the LED.)

## For specs/core to canonicalize

- The `r2.hw.led` trait (set-status command) + the descriptor shape above + the status enum (with R2-FNV
  hashes for the command/status if they ride the event bus).
- Same general-vs-device split ratified for LoRaRadio + display. hive supplies per-board drivers (DFR1195
  mono done — `ok`=heartbeat; mapping the rest is small); composer's sentant selects status; this trait is
  the contract between them.
