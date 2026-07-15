# r2-dataplane scaffold (hive-drafted, for core to integrate into r2-core)

The **API-contract scaffold** for the shared no_std RX data-plane + keepalive core (#24).
Drafted by hive (we own the body); **core registers it** as a r2-core workspace member +
wires the per-platform call-site (core owns r2-core edits + the integration seam).

## What this is / isn't
- **IS**: the stable public contract — `PhyMask`, `RxDisposition`, `DataPlane`, and the two
  entry points `handle_rx_frame` + `poll_keepalive`. Core can register the crate + wire the
  nRF54-LR2021 call-site against these signatures **now — no bench needed** (the high-leverage
  unblock for #24).
- **ISN'T**: the bodies. They're `todo!()` and land **POST-BENCH**, factored from the
  bench-validated DFR io_task RX pipeline (so nRF54 reuses proven code, not pre-bench churn).

## The seam (core pieces ↔ hive body)
- `r2-dataplane` deps = core's `r2-route` (plan_forward / Observation / DedupCache) +
  `r2-wire` (decode_extended / classify_extended_full) + `r2-trust` (GroupHmac). It does
  **NOT** dep `r2-dispatch` (std/above-L4-L5) — `deliver_out` is a **raw channel push**; the
  consumer composes it (MCU → local engine; host → r2-dispatch's DeliverOnly).
- The **platform** task owns I/O (radio/channels/LED/clock/windows) + calls
  `handle_rx_frame`/`poll_keepalive`. `PhyMask` bit ↔ TX-channel mapping is the **platform
  adapter** (keeps this crate PHY-agnostic).

## Open at integration (core's call)
1. **Delivery mechanism**: hive drafted these files; how do you want the content in r2-core —
   take this lib.rs/Cargo.toml as a patch and commit, or scaffold the crate skeleton yourself
   and hive fills the body? (Either works; the public contract above is the stable part.)
2. **Internal type alignment**: `DataPlane`'s fields (the exact `RouteEngine` generics /
   `DedupCache<N>` sizing) finalize against your r2-route API — shown as the contract shape.
3. **Buffer protocol**: `relay_out`/`deliver_out` lengths are returned in `RxDisposition`
   (`relay_len`/`deliver_len`); confirm that matches the nRF54 call-site you wire.
