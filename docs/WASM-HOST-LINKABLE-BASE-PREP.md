# WASM HOST — Linkable-Base Prep (hive workstream)

> **Status:** PREP / design memo (2026-07-10). Roy ruled GO on building the linkable base (via
> supervisor); hive owns the **wasm host**. This doc is the hive-side plan + the buildable-now vs
> ABI-gated split. It is **not canon** — it mirrors and builds against the specs proposal, which is
> the authority. **Coordinate under specs.**
>
> **The gate (do not skip):** the whole ABI-bound surface is **BLOCKED on specs freezing the
> R2-PLUGIN §12.4 `Plugin` trait to a `repr(C)` ABI** (A.5.3 lowering: fn-pointer table +
> `repr(C)`/`repr(u8)` result types + caller-provided buffers with explicit written-lengths + an
> `abi_version` field). Prep the ABI-independent scaffolding now; build the host-import surface
> against the frozen ABI when it lands.

## References (authority = specs)
- **Proposal (design authority):** `r2-specifications/docs/proposals/R2-LINKABLE-BASE-AND-VARIANT-MANAGEMENT-2026-07-10.md`
  — esp. **A.2** (Level 1/2/3 coupling), **A.4/A.5** (target × level matrix + core's no_std feasibility,
  the MEASURED nrf52840 RAM map), **A.5.3** (the §12.4 → `repr(C)` ABI lowering), **A.5.4** (Level-0
  hot-path/crypto never modularized), **B.4** (TG-gated module-load = firmware sovereignty),
  **D.1/D.3** (per-target recommendation + migration steps 4–5 = hive's wasm-host work).
- **R2-PLUGIN §12.4** — the `Plugin` trait author-contract (the ABI seed); §12.6 trust boundary
  ("a future WASM mode would sit alongside NIF as a second isolated mode"); §11 (WASM target, future).
- **R2-COMPILE §11** (hybrid mode — compiled core + one runtime-updateable slot) / **§12 OQ-4**
  (pico runtime hosting 2–3 compiled sentants) — the canon attach-seams the host lands at.
- Hive canon mirror: [HIVE-ARCHITECTURE-CANON.md](HIVE-ARCHITECTURE-CANON.md) (device-composition layering).

## 1. What we are building (and what we are NOT)
**Building:** a **wasm HOST** — the stable core TN base **hosts a wasm runtime** and **loads compiled
ensemble modules as `.wasm`**, sandboxed, calling a host-import surface (Level 2, A.2). This is the
mechanism that lets the R2-UPDATE §1.3 four-layer partial-update model stop collapsing to a full image
on MCU (Part C) and gives LoRa OTA a payload small enough to drip (B.6).

**NOT this:** the existing `crates/r2-hive-wasm` is **R2 compiled *to* wasm** (the `/proof` hive runs
the R2 stack inside a browser). That is the *inverse* — R2-as-wasm-guest, not R2-hosting-wasm-guests.
The proposal calls this out explicitly (A.2 table note). The new host is **greenfield**; it may *reuse*
`r2-hive-wasm`'s browser plumbing (wasm-bindgen glue, the WasmHive API) but the hosting mechanism is new.

## 2. The two ABI surfaces (both derive from the frozen §12.4 lowering)
A wasm ensemble module and the base meet across **two** surfaces — both are the `repr(C)` lowering of
the §12.4 contract, projected onto the wasm ABI (i32 args, linear-memory `ptr+len`, no borrowed Rust
crossings — which is exactly what A.5.3 already demands, so the lowering is *wasm-friendly by design*):

1. **Module EXPORTS (module → base contract)** = the §12.4 trait, lowered:
   `execute(command:u32, data_ptr:u32, data_len:u32, out_ptr:u32, out_cap:u32) -> written_len:u32`
   plus `init`, `poll` (out-param form of `Option<(u32,&[u8])>`), `name`/`id` (written-length form).
   The base wraps a loaded module in a Rust `Plugin` impl that trampolines into these exports — so
   from the engine's dispatch, **a wasm module IS a `Plugin`** and plugs into existing dispatch.
2. **Host IMPORTS (base → module capabilities)** = the `capabilities.requires` (§12.3) syscall surface
   the module may call back into: emit-event, current-time, RNG, log, and the capability handles the
   module declared. **This is the "host-import surface" supervisor named** — the load-bearing thing to
   get right, and the thing to co-design with specs so the frozen ABI is host-implementable in wasm.

**Both are frozen by the same §12.4 lowering + `abi_version`.** Until that lands I use a clearly-marked
**PROVISIONAL** placeholder ABI to prove host *mechanics* only, and swap it for the frozen one — no
ABI-bound code ships against an unfrozen surface (rework guard).

## 3. Target phasing (D.3 steps 4–5; core A.5 verdicts are binding)
1. **`std` host (Linux/Pi) FIRST** — wasmtime (sandbox + near-native JIT). Lowest-risk, prove the
   host-import surface here (D.3 step 4). Attach at R2-COMPILE §11.
2. **Browser** — the runtime *is* the host (native `WebAssembly`); reuse `r2-hive-wasm` browser glue.
   Native fit (D.1). std + browser share the Rust host-logic; only the runtime backend differs.
3. **esp32s3 + PSRAM** — pilot **wasm3** (interpreter; no JIT/AOT on MCU — W^X/no MMU). Linear memory
   in **PSRAM** to avoid contending for internal SRAM (A.5.1). First MCU linkable base (§12 OQ-4 pico).
4. **nrf52840 = exactly ONE bounded (≤32 KB), boot-reserved, wasm3-only slot** (core A.5.1 — general
   multi-module hosting is REFUTED; the binding constraint is 64 KB-page *contiguity* on the MMU-less
   allocator-backed heap, not total RAM). The slot's linear-memory block is **statically reserved at
   boot** (R2-COMPILE §11 "one slot"). Do not assume a general host fits.
5. **Level 0 (compiled-in) is never a module** (core A.5.4): route-forward/dedup/wire/CBOR/FNV, the
   radio io_task/SX1262 driver (hard-real-time), and per-frame HMAC/HKDF/Ed25519 (constant-time —
   sandbox timing variance is a side-channel). The wasm host is for **orchestration-class** logic only.

## 4. Runtime selection (conjecture — to be confirmed by a spike)
| Target | Runtime | Why |
|---|---|---|
| std host | **wasmtime** | mature, sandbox, near-native JIT, `Engine`/`Store`/`Linker` host-import model fits surface #2 cleanly. (wasmer is the fallback.) |
| browser | **native `WebAssembly`** via wasm-bindgen | the browser is the host; no embedded runtime. |
| esp32s3/nrf52840 | **wasm3** | ~64 KB flash, avoids WAMR-fast's bytecode-in-RAM (core A.5.1 — decisive on a 256 KB part); interpreter-only is fine (MCU can't JIT). |

Single Rust host crate, runtime behind a trait/backend seam so std↔browser↔wasm3 share the load /
verify / capability-broker logic; only the instantiate+call backend differs per target.

## 5. Buildable NOW (ABI-independent) vs GATED on the frozen ABI
**NOW (no rework risk):**
- New crate skeleton `r2-wasm-host` (std + browser feature seams; runtime-backend trait).
- **Runtime de-risk spike:** wasmtime loads + instantiates + calls a trivial module, module calls a
  host import, memory read/write across linear memory — proves host *mechanics* (D.3 step 4 groundwork).
- **TG-gated load gate (B.4)** scaffolding: verify the module's signature under the TG update root
  (reuse r2-trust cert / r2-update verify) **before** instantiate — a module is firmware; it gets
  firmware sovereignty. This is security-critical and ABI-independent.
- The `Plugin`-trampoline *shape* (surface #1) as an abstraction, against a PROVISIONAL placeholder ABI.
- Capability-broker design (surface #2): which `capabilities.requires` handles the host exposes.

**GATED on the frozen §12.4 `repr(C)` ABI (A.5.3):**
- The actual host-import function signatures + result struct layouts (`PluginResponse[128]`/
  `PluginError[64]` repr(C), `repr(u8)` discriminants, `abi_version` field).
- The final module-export signatures the base calls.
- Anything a third-party module author would compile against (must be the frozen surface, versioned).

## 6. Security gate (B.4 — non-negotiable, security-paramount)
- **Loading a module IS a code-update** → the on-device loader **MUST** verify the module signature
  under the TG's update root **before** mapping/instantiating (R2-UPDATE §1.3 module payload types
  `plugin_module 0x07` presuppose this gate). Fail-closed.
- **wasm sandbox = blast-radius control** (linear-memory bounds, no raw pointers, capability-style
  imports); signatures give *provenance* only. This is exactly why Level 2 (wasm) beats Level 3
  (native) — and why the host must never hand a module a raw capability it did not declare + get
  authorized for.
- The gate protects the **remote/OTA load path**; the physical wired path stays the re-commission
  bypass (B.4.1) — out of hive-wasm-host scope.

## 7. Coordination with specs (the gate owner)
hive is the **consumer** of the frozen ABI, so hive has concrete implementability input:
- Confirm the §12.4 lowering's `ptr+len` / caller-provided-buffer shape maps 1:1 onto wasm exports
  (it does — A.5.3's "no borrowed `&str`/`Option`/tuple crossings" is already the wasm requirement).
- Confirm the `abi_version` gate location + the fixed buffer sizes (128 B/64 B) survive as wasm
  linear-memory conventions.
- Ask the **freeze status/timeline** for §12.4 → `repr(C)` (D.4 R2-PLUGIN row).
- Offer the host-import (surface #2) capability list as feedback into the freeze so it is
  host-implementable, not just author-expressible.

## 8. Next steps (this workstream)
1. **[coordinate]** specs: ABI freeze status + host-implementability input (surface #2). ← sent.
2. **[build-now]** wasmtime runtime-mechanics spike (load/instantiate/call/host-import/memory) on std —
   PROVISIONAL ABI, proves the host works end-to-end.
3. **[build-now]** `r2-wasm-host` crate skeleton + the TG-gated load gate (B.4).
4. **[gated]** bind surfaces #1/#2 to the frozen §12.4 `repr(C)` ABI when specs freezes it.
5. **[later]** browser backend (reuse r2-hive-wasm glue); then esp32s3+PSRAM wasm3 pilot.

*Parallel to + separate from the RAK bench/beacon work. State trail in [RESUME.md](../RESUME.md).*
