//! # r2-wasm-host — the linkable-base wasm HOST (Milestone M3)
//!
//! **Why this crate exists (first-time reader):** Roy ruled (2026-07-10) to build the
//! *linkable base* — a stable core TN base that separately-compiled ensembles link into,
//! so we stop recompiling the whole world per variant. hive owns the **wasm HOST**: the
//! base hosts a wasm runtime and loads compiled ensemble modules as sandboxed `.wasm`
//! (Level 2 of the proposal). This is the **inverse** of `crates/r2-hive-wasm` (which is
//! R2 compiled *to* wasm to run in a browser); here R2 is the HOST and ensembles are guests.
//!
//! ## Bound to RATIFIED R2-PLUGIN v0.7 §12.4.3.1 (specs `6ccd656`)
//! The [`abi`] module encodes the ratified **WASM projection (Level 2)** of the frozen
//! §12.4.3 `PluginVTableV1`: the 8 module exports, the 136 B native-image `AbiResult` layout
//! (little-endian, `tag@0`/payload`@4`, padding-zero-pinned), and the full-32 B `abi_hash`
//! load-gate. The earlier `provisional_abi` placeholder is retired — this is the real surface.
//! core's r2-forge emits conformant modules against the same text (native `vtable_for` thunks +
//! the padding-zero assertion); the wasm-image `AbiResult` KAT is core-supplied.
//!
//! Authority: `r2-specifications` R2-PLUGIN §12.4.3.1 (v0.7); proposal
//! `docs/proposals/R2-LINKABLE-BASE-AND-VARIANT-MANAGEMENT-2026-07-10.md`; hive plan
//! `docs/WASM-HOST-LINKABLE-BASE-PREP.md` + `docs/WASM-PROJECTION-12.4.3.1-CANDIDATE.md`.
//!
//! ## Load gate (§12.4.3.1, fail-closed, B.2.0 order)
//! 1. **Authorization** — verify the module signature under the TG update root (memo B.4).
//!    *Not yet wired here* — it is the ABI-independent B.4 TG-gated load gate (next build-now);
//!    [`WasmHost::instantiate`] documents the hook point.
//! 2. **Compat** — read `__r2_abi_hash` (full 32 B); require `module == host` **exact**
//!    (Ruling 2); refuse-to-instantiate on mismatch. **Implemented** ([`WasmHost::instantiate`]).
//! 3. **Instantiate** — only then run plugin logic (`r2_init`, then serve `r2_execute`/`r2_poll`).
//!
//! ## Known follow-on (flagged to specs/core): the host↔guest memory-region convention
//! §12.4.3.1 fixes the export *signatures* (`r2_execute(command, data_ptr, data_len, result_ptr)`)
//! but not **where** the host may place the `data_ptr`/`result_ptr` buffers inside the module's
//! linear memory. This host uses a fixed [`scratch`] region as a **provisional** convention for
//! the conformance spike; a real r2-forge module needs a pinned convention (an `__r2_alloc` export
//! or a module-reserved host-scratch region) — a §12.4.3.1 follow-on.

/// Ratified R2-PLUGIN v0.7 §12.4.3.1 constants — the WASM projection of `PluginVTableV1`.
pub mod abi {
    /// `abi_version` of the frozen ABI (v1). Read first (module exports it as a global).
    pub const ABI_VERSION: u32 = 1;
    /// Full 32 B `abi_hash` v1 — the fail-closed load-gate exact-match value (Ruling 2). The
    /// module exports it via `__r2_abi_hash`; the host embeds it here and compares byte-exact.
    /// First 8 B (`c37f504d4c2a9d8c`) are the `UpdateHeader`/recipe compat truncation (Ruling 1).
    pub const ABI_HASH_V1: [u8; 32] = [
        0xc3, 0x7f, 0x50, 0x4d, 0x4c, 0x2a, 0x9d, 0x8c, 0x1f, 0x5b, 0xc2, 0x14, 0xaa, 0x22, 0x9b,
        0x4a, 0xe8, 0xc0, 0xd8, 0x88, 0x97, 0xa4, 0x9c, 0xce, 0x51, 0x98, 0x14, 0xd8, 0x91, 0x5a,
        0x81, 0x7e,
    ];

    // ── the 8 module exports (§12.4.3.1) ──
    // NOTE (v0.8 ruling-direction, core 5447034 on real wasm32): abi_version + plugin_id are
    // VALUE-RETURNING FUNCS `() -> i32`, NOT globals — a Rust `pub static` compiles to a wasm
    // global holding the value's ADDRESS (a linear-memory offset), so a host reading the global
    // gets a pointer, not the value. Funcs are toolchain-independent + uniform with the other 6.
    pub const EXPORT_MEMORY: &str = "memory";
    pub const FUNC_ABI_VERSION: &str = "__r2_abi_version";
    pub const FUNC_ABI_HASH: &str = "__r2_abi_hash";
    pub const FUNC_PLUGIN_ID: &str = "__r2_plugin_id";
    pub const FUNC_INIT: &str = "r2_init";
    pub const FUNC_EXECUTE: &str = "r2_execute";
    pub const FUNC_POLL: &str = "r2_poll";
    pub const FUNC_NAME: &str = "r2_name";
    // §12.4.3.2 (R2-PLUGIN v0.9): the module-reserved host-scratch region accessors (funcs).
    pub const FUNC_SCRATCH_PTR: &str = "__r2_scratch_ptr";
    pub const FUNC_SCRATCH_LEN: &str = "__r2_scratch_len";

    /// Size of the `AbiResult` native image the host reads (§12.4.3.1: 136 B, padding pinned zero).
    pub const ABI_RESULT_LEN: usize = 136;
    pub const TAG_OK: u8 = 0;
    pub const TAG_ERR: u8 = 1;
    // AbiResult field offsets (tag@0, payload@4 — AbiError's u32 forces union-align 4):
    pub const OFF_OK_DATA: usize = 4; // ..132
    pub const OFF_OK_LEN: usize = 132; // u16
    pub const OFF_ERR_CODE: usize = 4; // u32
    pub const OFF_ERR_DESC: usize = 8; // ..72
    pub const OFF_ERR_DESC_LEN: usize = 72; // u16
    /// Max `AbiResponse` data length (§12.4.3 `[u8; 128]`).
    pub const MAX_OK_DATA: usize = 128;
    /// Max `AbiError` desc length (§12.4.3 `[u8; 64]`).
    pub const MAX_ERR_DESC: usize = 64;
}

/// §12.4.3.2 (R2-PLUGIN v0.9) host-scratch region layout — sub-buffer offsets **relative to
/// `__r2_scratch_ptr()`**. The host reads the region base+len at instantiate and places these five
/// non-overlapping fixed sub-buffers within it (the 1000 B floor = 32+136+64+256+512). `INPUT` is
/// the input sub-buffer start; its capacity is `scratch_len - INPUT` (the module may reserve more).
mod scratch {
    pub const MIN_LEN: u32 = 1004; // spec floor (§12.4.3.2 v0.10): 32+136+64+(4+256)+512
    pub const HASH: u32 = 0; // 32 B (__r2_abi_hash out)
    pub const RESULT: u32 = 32; // 136 B (AbiResult)
    pub const NAME: u32 = 168; // NAME_CAP
    pub const NAME_CAP: u32 = 64;
    pub const POLL_EV: u32 = 232; // 4 B (u32 event hash)
    pub const POLL_BUF: u32 = 236; // POLL_CAP
    pub const POLL_CAP: u32 = 256;
    pub const INPUT: u32 = 492; // execute input (poll ends at 236+256=492); cap = scratch_len - INPUT
}

/// A verified/parsed plugin result (host view of the 136 B `AbiResult` image).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginResult {
    /// `AbiResult::Ok(AbiResponse)` — the response bytes (`data[..len]`).
    Ok(alloc_vec::Vec<u8>),
    /// `AbiResult::Err(AbiError)` — error code + description bytes (`desc[..desc_len]`).
    Err { code: u32, desc: alloc_vec::Vec<u8> },
}

// std host: Vec is std::vec::Vec. (Kept behind an alias so a future no_std/alloc backend is a
// one-line change, matching the crate's multi-backend intent.)
mod alloc_vec {
    pub use std::vec::Vec;
}

/// Parse a 136 B `AbiResult` native image into a host [`PluginResult`] (§12.4.3.1 layout).
#[cfg(feature = "std-wasmtime")]
fn parse_abi_result(buf: &[u8; abi::ABI_RESULT_LEN]) -> anyhow::Result<PluginResult> {
    use anyhow::{anyhow, bail};
    match buf[0] {
        abi::TAG_OK => {
            let len = u16::from_le_bytes([buf[abi::OFF_OK_LEN], buf[abi::OFF_OK_LEN + 1]]) as usize;
            if len > abi::MAX_OK_DATA {
                bail!("AbiResult Ok len {len} exceeds 128");
            }
            Ok(PluginResult::Ok(buf[abi::OFF_OK_DATA..abi::OFF_OK_DATA + len].to_vec()))
        }
        abi::TAG_ERR => {
            let code = u32::from_le_bytes([
                buf[abi::OFF_ERR_CODE],
                buf[abi::OFF_ERR_CODE + 1],
                buf[abi::OFF_ERR_CODE + 2],
                buf[abi::OFF_ERR_CODE + 3],
            ]);
            let dlen =
                u16::from_le_bytes([buf[abi::OFF_ERR_DESC_LEN], buf[abi::OFF_ERR_DESC_LEN + 1]])
                    as usize;
            if dlen > abi::MAX_ERR_DESC {
                bail!("AbiResult Err desc_len {dlen} exceeds 64");
            }
            Ok(PluginResult::Err {
                code,
                desc: buf[abi::OFF_ERR_DESC..abi::OFF_ERR_DESC + dlen].to_vec(),
            })
        }
        other => Err(anyhow!("AbiResult tag {other} is neither Ok(0) nor Err(1)")),
    }
}

#[cfg(feature = "std-wasmtime")]
mod wasmtime_backend {
    use crate::{abi, parse_abi_result, scratch, PluginResult};
    use anyhow::{anyhow, bail, Context, Result};
    use wasmtime::{Engine, Instance, Memory, Module, Store};

    /// The wasmtime-backed linkable-base host. Holds a shared [`Engine`]; modules are compiled
    /// once via [`WasmHost::load`] then instantiated (with the §12.4.3.1 load gate) per use.
    pub struct WasmHost {
        engine: Engine,
    }

    /// A compiled (validated) module. Compilation runs no guest code.
    pub struct LoadedModule {
        module: Module,
    }

    /// A live plugin instance — the module has passed the abi_hash gate and had `r2_init` called.
    /// Each instance owns its [`Store`], so guest state never leaks across instances (isolation).
    pub struct PluginInstance {
        store: Store<()>,
        instance: Instance,
        memory: Memory,
        abi_version: u32,
        plugin_id: u8,
        /// §12.4.3.2 host-scratch region base (`__r2_scratch_ptr()`) — sub-buffers are `base + scratch::*`.
        scratch_base: u32,
        /// Region length (`__r2_scratch_len()`); in-region execute-input cap = `len - scratch::INPUT`.
        scratch_len: u32,
    }

    impl WasmHost {
        pub fn new() -> Result<Self> {
            Ok(Self { engine: Engine::default() })
        }

        /// Compile+validate a module from a `.wasm` binary or `.wat` text. No guest code runs.
        pub fn load(&self, wasm_or_wat: &[u8]) -> Result<LoadedModule> {
            Ok(LoadedModule {
                module: Module::new(&self.engine, wasm_or_wat).context("compile module")?,
            })
        }

        /// Instantiate a module through the §12.4.3.1 load gate, then call `r2_init`.
        ///
        /// Gate order (B.2.0): **(1)** [B.4 signature verify — TODO, the ABI-independent gate;
        /// callers MUST do it before this today] **(2)** `__r2_abi_hash` full-32 B exact-match
        /// (Ruling 2 — refuse on mismatch) **(3)** run `r2_init` (must return Ok). A v1 module
        /// declares no host imports (pure-compute), so it instantiates with an empty import set.
        pub fn instantiate(&self, module: &LoadedModule) -> Result<PluginInstance> {
            let mut store = Store::new(&self.engine, ());
            // v1 modules import nothing (the capability/host-import surface is a later increment).
            let instance = Instance::new(&mut store, &module.module, &[])
                .context("instantiate module")?;
            let memory = instance
                .get_memory(&mut store, abi::EXPORT_MEMORY)
                .ok_or_else(|| anyhow!("module exports no `{}`", abi::EXPORT_MEMORY))?;

            let abi_version = call_i32_func(&instance, &mut store, abi::FUNC_ABI_VERSION)? as u32;
            if abi_version != abi::ABI_VERSION {
                bail!("module abi_version {abi_version} != host v{}", abi::ABI_VERSION);
            }

            // §12.4.3.2: read the module-reserved host-scratch region (funcs) before placing buffers.
            let scratch_base = call_i32_func(&instance, &mut store, abi::FUNC_SCRATCH_PTR)? as u32;
            let scratch_len = call_i32_func(&instance, &mut store, abi::FUNC_SCRATCH_LEN)? as u32;
            if scratch_len < scratch::MIN_LEN {
                bail!(
                    "__r2_scratch_len() {scratch_len} below the §12.4.3.2 floor {}",
                    scratch::MIN_LEN
                );
            }
            let hash_off = (scratch_base + scratch::HASH) as usize;
            let result_off = (scratch_base + scratch::RESULT) as usize;

            // GATE (Ruling 2): the module writes its full 32 B abi_hash into the scratch region;
            // require exact-match. (Instantiation above is inert — a conformant module has no start.)
            let abi_hash_fn = instance
                .get_typed_func::<i32, ()>(&mut store, abi::FUNC_ABI_HASH)
                .context("module missing __r2_abi_hash")?;
            abi_hash_fn.call(&mut store, hash_off as i32)?;
            let mut got = [0u8; 32];
            memory.read(&store, hash_off, &mut got)?;
            if got != abi::ABI_HASH_V1 {
                bail!(
                    "abi_hash mismatch — refuse to instantiate (Ruling 2): module {} != host v1",
                    hex(&got)
                );
            }

            let plugin_id = (call_i32_func(&instance, &mut store, abi::FUNC_PLUGIN_ID)? & 0xff) as u8;

            // r2_init (required export) — must return Ok.
            let init = instance
                .get_typed_func::<i32, ()>(&mut store, abi::FUNC_INIT)
                .context("module missing r2_init")?;
            init.call(&mut store, result_off as i32)?;
            match read_result(&memory, &store, result_off)? {
                PluginResult::Ok(_) => {}
                PluginResult::Err { code, .. } => bail!("r2_init returned Err(code={code})"),
            }

            Ok(PluginInstance {
                store,
                instance,
                memory,
                abi_version,
                plugin_id,
                scratch_base,
                scratch_len,
            })
        }
    }

    impl PluginInstance {
        pub fn abi_version(&self) -> u32 {
            self.abi_version
        }
        pub fn plugin_id(&self) -> u8 {
            self.plugin_id
        }

        /// Absolute linear-memory offset of a scratch sub-buffer (`scratch_base + relative`).
        fn off(&self, rel: u32) -> usize {
            (self.scratch_base + rel) as usize
        }

        /// Call `r2_execute(command, data)` and return the parsed result. Input is placed in the
        /// §12.4.3.2 scratch region if it fits the in-region cap (`scratch_len - INPUT`), else
        /// fail-closed (the `__r2_alloc` escalation for oversized std/browser inputs is a follow-on;
        /// MCU modules never carry it — §12.4.3.2).
        pub fn execute(&mut self, command: u8, data: &[u8]) -> Result<PluginResult> {
            let input_cap = (self.scratch_len - scratch::INPUT) as usize;
            if data.len() > input_cap {
                bail!(
                    "execute input {} B exceeds in-region cap {} and __r2_alloc escalation is not wired",
                    data.len(),
                    input_cap
                );
            }
            let (input_off, result_off) = (self.off(scratch::INPUT), self.off(scratch::RESULT));
            self.memory
                .write(&mut self.store, input_off, data)
                .context("write execute input")?;
            let f = self
                .instance
                .get_typed_func::<(i32, i32, i32, i32), ()>(&mut self.store, abi::FUNC_EXECUTE)
                .context("module missing r2_execute")?;
            f.call(
                &mut self.store,
                (command as i32, input_off as i32, data.len() as i32, result_off as i32),
            )?;
            read_result(&self.memory, &self.store, result_off)
        }

        /// Call `r2_poll`; returns `Some((event_hash, payload))` or `None` (-1).
        pub fn poll(&mut self) -> Result<Option<(u32, Vec<u8>)>> {
            let (ev_off, buf_off) = (self.off(scratch::POLL_EV), self.off(scratch::POLL_BUF));
            let f = self
                .instance
                .get_typed_func::<(i32, i32, i32), i32>(&mut self.store, abi::FUNC_POLL)
                .context("module missing r2_poll")?;
            let n = f.call(&mut self.store, (ev_off as i32, buf_off as i32, scratch::POLL_CAP as i32))?;
            if n < 0 {
                return Ok(None);
            }
            let n = n as usize;
            if n > scratch::POLL_CAP as usize {
                bail!("r2_poll returned len {n} exceeding cap {}", scratch::POLL_CAP);
            }
            let mut ev = [0u8; 4];
            self.memory.read(&self.store, ev_off, &mut ev)?;
            let mut buf = vec![0u8; n];
            self.memory.read(&self.store, buf_off, &mut buf)?;
            Ok(Some((u32::from_le_bytes(ev), buf)))
        }

        /// Call `r2_name`; returns the module-reported name.
        pub fn name(&mut self) -> Result<String> {
            let name_off = self.off(scratch::NAME);
            let f = self
                .instance
                .get_typed_func::<(i32, i32), i32>(&mut self.store, abi::FUNC_NAME)
                .context("module missing r2_name")?;
            let n = f.call(&mut self.store, (name_off as i32, scratch::NAME_CAP as i32))?;
            if n < 0 || n as u32 > scratch::NAME_CAP {
                bail!("r2_name returned invalid len {n}");
            }
            let mut buf = vec![0u8; n as usize];
            self.memory.read(&self.store, name_off, &mut buf)?;
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }
    }

    fn read_result(memory: &Memory, store: &Store<()>, result_off: usize) -> Result<PluginResult> {
        let mut buf = [0u8; abi::ABI_RESULT_LEN];
        memory
            .read(store, result_off, &mut buf)
            .context("read AbiResult image")?;
        parse_abi_result(&buf)
    }

    /// Call a `() -> i32` value-returning export (§12.4.3.1 v0.8: abi_version / plugin_id are
    /// funcs, not globals — a Rust `pub static` global would export the value's ADDRESS, not the
    /// value; a func returns the value directly, toolchain-independent).
    fn call_i32_func(instance: &Instance, store: &mut Store<()>, name: &str) -> Result<i32> {
        instance
            .get_typed_func::<(), i32>(&mut *store, name)
            .with_context(|| format!("module missing `{name}` () -> i32"))?
            .call(&mut *store, ())
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
}

#[cfg(feature = "std-wasmtime")]
pub use wasmtime_backend::{LoadedModule, PluginInstance, WasmHost};
