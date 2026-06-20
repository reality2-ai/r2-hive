# r2.hw.display — descriptor refinements from the ST7735S driver (for specs)

specs accepted the driver-side draft. composer's base descriptor:
`{kind, width, height, colour, text_rows, backlight(0/1/2), power_cut(0/1)}`. From driving the **real
ST7735S** (160×80 RGB565) on the DFR1195, here's what to refine — grounded in working hardware, not theory.

## ADD these fields (the sentant/plugin genuinely needs them)

1. **`text_cols` (u8)** — usable character columns. The StatusDisplay sentant formats `label:value` lines and
   must truncate/elide to fit horizontally; it already adapts to `text_rows` (top-N) — `text_cols` completes
   it for the horizontal fit. ST7735S 160 px / FONT_6X10 (6 px) ≈ **26 cols**.
2. **`min_update_ms` (u16)** — the minimum interval between renders the panel sustains. Calm-tech renders
   on-change, but change-bursts must be rate-limited to what SPI + panel handle. ST7735S @ 20 MHz: a full
   160×80×16-bit frame ≈ 13 ms on the wire; a sensible calm floor is **~100 ms**. Stops a chatty sentant from
   thrashing the bus / flickering.
3. **`partial_update` (bool)** — the controller supports addressable-window writes (rewrite only changed rows,
   not full clear+redraw). Pairs with render-on-change for efficiency + flicker-free updates. ST7735S = **true**.

## CONFIRM / set for the DFR1195 panel

`kind=st7735s, width=160, height=80, colour=rgb565, text_rows=8 (80/10), text_cols=26, backlight=2 (dimmable —
GPIO16 is PWM-capable), power_cut=1 (controller power rail can be fully cut), min_update_ms=100,
partial_update=1`.

## KEEP OUT of the capability descriptor — driver / board-profile details, NOT sentant-facing

To keep the sentant device-agnostic, these per-panel quirks belong in the **board profile**, not the
capability contract (the sentant must never need them):
- **rotation** — the panel is native 80×160; the driver rotates 90° to present a logical 160×80. The sentant
  only ever sees the logical surface.
- **addressable-window offset** — this panel needs a **(26, 1)** column/row offset; pure driver internal.
- **power polarity** — **GPIO48 controller power is ACTIVE-LOW** (the costly board fact). Driver/board-profile.
- **SPI freq/pins** (20 MHz, Mode 0; MOSI11/SCK12/CS17/DC14/RST15/BL16/PWR48) — board profile.

Rationale: the capability descriptor is the **sentant↔plugin contract** (the logical surface + what to honour);
per-panel electrical/geometry quirks are the **driver's** job. This keeps R2-HMI clean and the sentant portable
across panels (an ST7789 or a bigger ST7735 differs only in descriptor numbers + driver, not in the contract).

## LED descriptor (already drafted)

See `docs/r2-hw-led-capability-proposal.md`: `CMD_SET_STATUS{status}`; vocabulary
`all_well(heartbeat) / joining / ota / error / identify / idle`; descriptor `kind:mono|rgb` +
`available_patterns` + (rgb) per-status `colour` slots. DFR1195 = `{kind:mono, dimmable}` (GPIO21 software-PWM
heartbeat validated on hardware). Matches your list; `idle` is the off/quiescent state. The two descriptors
share the same shape: **a small semantic surface + a `kind` that selects the device idiom.**
