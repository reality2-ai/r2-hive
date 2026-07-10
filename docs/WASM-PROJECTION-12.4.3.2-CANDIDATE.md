# R2-PLUGIN §12.4.3.2 — host↔guest memory convention (Level 2) — CANDIDATE

> **Status:** CANDIDATE (2026-07-10). Drafted by hive (host, Milestone M3) after binding
> `r2-wasm-host` to §12.4.3.1 surfaced this gap. Flow (specs' decision): **co-pinned by hive
> (host) + core (r2-forge module-emit) → specs ratifies** as R2-PLUGIN **§12.4.3.2** (same as
> §12.4.3.1). Not canon until specs ratifies. Incorporates specs' design lean (module-reserved
> host-scratch region, MCU-first) and answers its open question (one convention vs tiered).
>
> **Refs:** R2-PLUGIN §12.4.3 / §12.4.3.1 (the ratified WASM projection); proposal §A.5 (the
> nrf52840 one-bounded-≤32 KB-slot verdict — the binding target); host `r2-hive/crates/r2-wasm-host`
> (`6a13ec8`, bound to §12.4.3.1 using a *provisional* fixed-offset scratch — this subsection
> replaces that with the real convention).

## The gap
§12.4.3.1 pins the export **signatures** — `r2_execute(command, data_ptr, data_len, result_ptr)`,
`__r2_abi_hash(out_ptr)`, `r2_poll(ev_hash_out_ptr, buf_ptr, cap)`, `r2_name(buf_ptr, cap)` — but
**not where** the host may place those `*_ptr` buffers inside the module's linear memory. A naive
fixed host offset can clobber the module's own statics/heap/stack; without a convention, host and
module cannot share memory safely, and a third impl cannot conform.

## The convention (unified baseline + optional escalation)
**Answer to specs' open question: NOT strictly tiered — one baseline that works everywhere (incl.
the nrf52840 bounded slot), plus an optional escalation for roomy hosts.** The baseline alone is
sufficient on MCU (no allocator, deterministic bounds); std/browser MAY additionally offer dynamic
allocation.

### REQUIRED — the module-reserved host-scratch region
A conformant Level-2 module MUST export two **value-returning funcs** (`() -> i32`, **not globals** —
uniform with `__r2_abi_version`/`__r2_plugin_id` per the v0.8 metadata-funcs resolution: a Rust
`pub static` global exports the value's *address*, not the value) delimiting a linear-memory region
it **reserves for the host** and guarantees never to read or write except as the host's buffer target:
| Export | Kind | Meaning |
|---|---|---|
| `__r2_scratch_ptr` | func `() -> i32` | start offset of the host-scratch region. |
| `__r2_scratch_len` | func `() -> i32` | length (bytes) of the region. |

- The host owns `[__r2_scratch_ptr(), __r2_scratch_ptr() + __r2_scratch_len())` for the lifetime of
  the instance and places all §12.4.3.1 call buffers there.
- **Pinned buffer bounds (co-pin proposal; caps are the required minimums, r2-forge may size larger):**
  `__r2_abi_hash` out = **32 B**; `AbiResult` result slot = **136 B** (fixed, §12.4.3.1);
  `r2_name` out cap `NAME_CAP` = **64 B**; `r2_poll` payload cap `POLL_CAP` = **256 B**;
  `r2_execute` input margin `INPUT_MIN` = **512 B** (the in-region input ceiling before escalation).
- **Sizing invariant (r2-forge enforces):** `__r2_scratch_len()` MUST be ≥ the **fixed-buffer floor**
  `32 + 136 + NAME_CAP + POLL_CAP + INPUT_MIN` (= **1000 B** at the pinned caps). r2-forge sizes the
  reservation per target: bounded on MCU (fits the ≤32 KB slot), larger on std/browser.
- **Owner + lifetime:** the **host** owns the region for the instance's lifetime; buffers within it
  are **transient per call** (host writes input before a call, reads the result after). The module
  MUST NOT retain or read host-written bytes in the region across calls, and MUST NOT read the
  non-selected `AbiResult` branch bytes.
- **Determinism + security:** the region is module-declared and bounded, so the host never guesses;
  there is **no allocator** and hence no allocator attack surface on MCU; placement is fully
  deterministic (KAT-pinnable). **Sandbox-safe by construction:** the host writes only through the
  wasm engine's bounds-checked linear memory — a ptr/len outside the module's memory **traps** (the
  engine bounds-checks every access), so even a host bug is a trap, never silent corruption; the
  module-declared region is the *correctness* layer atop that *safety* floor.

### OPTIONAL — dynamic allocation (std/browser flexibility)
A module MAY additionally export:
| Export | Kind | Meaning |
|---|---|---|
| `__r2_alloc` | func `(size: i32) -> i32` | returns a linear-memory ptr for `size` bytes, or `0` on failure. |
| `__r2_free` | func `(ptr: i32, size: i32)` | releases a prior `__r2_alloc` block. |

- If present, the host uses `__r2_alloc` for buffers that exceed the reserved region (large
  `r2_execute` inputs on std/browser).
- **MCU modules OMIT these** (per proposal §A.5: no allocator on the bounded slot — no attack
  surface, no mid-run contiguous-allocation failure on the MMU-less heap).

### Host buffer-placement algorithm (normative)
1. Fixed-size buffers — `__r2_abi_hash` out (32 B), `AbiResult` (136 B), `r2_name` out, `r2_poll`
   ev+buf — are ALWAYS placed in the reserved region (the sizing invariant guarantees they fit).
2. `r2_execute` input (`data`): if it fits the reserved region's input margin, place it there; ELSE
   if `__r2_alloc` is exported, allocate + place + `__r2_free` after; ELSE **fail-closed** with
   "input exceeds module scratch and module exports no `__r2_alloc`" (never a silent clobber).

## Conformance
- **Host** (`r2-wasm-host`): read `__r2_scratch_ptr`/`__r2_scratch_len` at instantiate; place all
  §12.4.3.1 buffers per the algorithm; prefer `__r2_alloc` only when input overflows the region.
  (Replaces the provisional fixed-offset scratch in `6a13ec8`.)
- **Module** (r2-forge): reserve the region (a `static` byte array the linker places + the two
  globals point at) sized per target; emit `__r2_alloc`/`__r2_free` only for std/browser targets.
- **KAT:** a reserved-region-conformant module (fixed `__r2_scratch_ptr`/`len`) — core supplies,
  as with the §12.4.3.1 wasm-image KAT.

## Why this over pure `__r2_alloc`
An `__r2_alloc`-only convention would force an allocator into every module — exactly the attack
surface + MMU-less-heap fragility the proposal §A.5 refuses on the nrf52840 slot. The reserved
region is the Occam-simplest thing that works on the tightest target, and `__r2_alloc` rides on top
only where the roominess is real — so one convention spans MCU→std→browser without tiering the host.

---
*Open for specs ratification as R2-PLUGIN §12.4.3.2. hive (host) + core (r2-forge) co-pin; specs
ratifies. hive holds re-binding r2-wasm-host to this until ratified (spec-first).*
