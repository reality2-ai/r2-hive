# R2-PLUGIN §12.4.3.1 — WASM projection (Level 2) — CANDIDATE for specs ratification

> **Status:** CANDIDATE (2026-07-10). Drafted by hive (host-implementability authority for the
> Level-2 wasm host, Milestone M3) for ratification by **specs** as a normative subsection
> **R2-PLUGIN §12.4.3.1**. Co-pinned with **core** (r2-forge module-emit authority). It is **not
> canon until specs ratifies** — this is the spec-first landing of the host↔module wire boundary
> so any conformant host and any r2-forge-emitted module interoperate (not a bilateral handshake).
>
> **Why normative:** §12.4.3 froze the **native `repr(C)`** `PluginVTableV1` (fn-pointers). A wasm
> module cannot return a native fn-pointer vtable, so Level 2 needs its own **byte-for-byte pinned
> projection** — wasm exports + a linear-memory result layout. Un-pinned, it drifts and a third
> implementation cannot conform. This subsection is that pin.
>
> **Refs:** R2-PLUGIN §12.4 (author trait) / §12.4.3 (frozen native ABI, the 4 rulings);
> proposal `docs/proposals/R2-LINKABLE-BASE-AND-VARIANT-MANAGEMENT-2026-07-10.md` §A.5.3 + Part C;
> host: `r2-hive/crates/r2-wasm-host` (spike `ca2b341` proves the mechanics); core §12.4.3 impl
> `f866c3f` + `abi_hash` KAT `329d708`.

## Principle
The projection is a **1:1 lowering of `PluginVTableV1` onto the wasm ABI**. The native `abi_version`
/`id` fields become wasm globals; each native fn-pointer becomes a wasm export taking/returning
`i32` with linear-memory `ptr`+`len`; the native `inst: *mut c_void` is **dropped** — a wasm module
instance **is** the plugin instance (§12.4.3 Ruling 3, one vtable per module), its state living in
its own linear memory. r2-forge emits these exports from a `Plugin` impl (the wasm analog of the
native `vtable_for`); hive's `r2-wasm-host` consumes them.

## Module export contract (a conformant Level-2 module MUST export exactly these)
| # | Export | Kind | Signature | Meaning |
|---|---|---|---|---|
| 1 | `memory` | memory | — | the module's linear memory (host reads/writes buffers here). |
| 2 | `__r2_abi_version` | global i32 | (immutable) | `abi_version: u32` (§12.4.3). Read **first**. |
| 3 | `__r2_abi_hash` | func | `(out_ptr: i32) -> ()` | writes the **full 32 B** `abi_hash` (SHA-256) into memory at `out_ptr`. |
| 4 | `__r2_plugin_id` | global i32 | (immutable) | `id: u8`. |
| 5 | `r2_init` | func | `(result_ptr: i32) -> ()` | runs `init`; writes an `AbiResult` image at `result_ptr`. Called once, post-load. |
| 6 | `r2_execute` | func | `(command: i32, data_ptr: i32, data_len: i32, result_ptr: i32) -> ()` | reads `data_len` bytes at `data_ptr`; writes an `AbiResult` image at `result_ptr`. |
| 7 | `r2_poll` | func | `(ev_hash_out_ptr: i32, buf_ptr: i32, cap: i32) -> i32` | writes ≤`cap` bytes at `buf_ptr` + the `u32` event hash at `ev_hash_out_ptr`; returns written len, or **-1 = None**. |
| 8 | `r2_name` | func | `(buf_ptr: i32, cap: i32) -> i32` | writes the name (≤`cap`) at `buf_ptr`; returns its length. |

`init`/`poll` are **required** exports (§12.4.3 Ruling 4); r2-forge emits a no-op default for
authors who do not implement them, so the host never has a missing-export/null-call surface.

## Result layout — the byte-exact native `AbiResult` image (136 B)
The `AbiResult` written at `result_ptr` is the **byte-for-byte image of the native `repr(C, u8)`
`AbiResult`** (§12.4.3), so there is exactly **one** layout and core's existing native 136 B
`AbiResult` KAT pins the wasm buffer too — no separate wasm encoding to maintain or drift. The host
supplies a result buffer of **≥ 136 B**. All multi-byte fields are little-endian (wasm memory + the
LE host agree). Layout (core-pinned, `repr(C, u8)`: `tag@0`, payload `@4` because `AbiError`'s `u32`
forces union-align 4):

```
offset 0   : tag u8         (0 = Ok, 1 = Err)
offset 1..4: padding
--- tag 0 (Ok) — mirrors AbiResponse { data:[u8;128], len:u16 } ---
offset 4..132  : data[128]
offset 132..134: len u16      (len ≤ 128)
--- tag 1 (Err) — mirrors AbiError { code:u32, desc:[u8;64], desc_len:u16 } ---
offset 4..8    : code u32
offset 8..72   : desc[64]
offset 72..74  : desc_len u16
--- total size 136 B (native size, KAT-pinned) ---
```

Field **order mirrors §12.4.3** (data-then-len; code, desc, desc_len) — an earlier hive draft
len-prefixed and mis-sized (132 B); corrected here to the native image per core's review.

## The abi_hash gate (two forms — targeting ≠ authorization, memo B.2.0)
- **Full 32 B** SHA-256 over the canonical schema — exported by the module (`__r2_abi_hash`),
  embedded in the host — is the fail-closed **load-gate exact-match** (§12.4.3 Ruling 2). v1 =
  `c37f504d4c2a9d8c1f5bc214aa229b4ae8c0d88897a49cce519814d8915a817e`.
- **8 B truncation** (`c37f504d4c2a9d8c`) rides only in the signed `UpdateHeader` / composer recipe
  as a **compat pre-filter** (§12.4.3 Ruling 1), authenticated by the header Ed25519 signature.
- A **monolithic** (non-module) image carries **all-zero** — no frozen-ABI dependency.

## Load gate (fail-closed, in B.2.0 check order)
1. **Authorization** — verify the module's signature under the TG update root (R2-UPDATE §2.4;
   who-may-load-a-module, memo B.4). A module is firmware; it gets firmware sovereignty.
2. **Compat** — read `__r2_abi_hash` (32 B); require `module == host` **exact**; refuse-to-instantiate
   on any mismatch (Ruling 2 — this is what keeps the base-version axis additive, Part C).
3. **Instantiate** — only then instantiate, read `__r2_abi_version`/`__r2_plugin_id`, call `r2_init`.

## Conformance + reversibility
- **Conformers:** hive `r2-wasm-host` (host) + r2-forge (module-emit). Core supplies the wasm-image
  `AbiResult` KAT (= the native 136 B KAT). hive's `ca2b341` spike already proves the export
  mechanism (ptr+len exports + linear-memory result buffers).
- **Reversibility:** a future projection is a **new** export set/version (e.g. `r2_execute` under a
  bumped `__r2_abi_version` → different `abi_hash`) alongside v1; exact-hash matching keeps a v1
  module fail-closed against a v2 host rather than mis-loading (mirrors §12.4.3 `PluginVTableV2`).

---
*Open for specs ratification as R2-PLUGIN §12.4.3.1. host (hive) drafts + implements the consumer;
core (r2-forge) co-pins + owns the wasm codegen; specs ratifies the normative text.*
