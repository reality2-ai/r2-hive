//! # r2-wasm-host — the linkable-base wasm HOST (Milestone M3)
//!
//! **Why this crate exists (first-time reader):** Roy ruled (2026-07-10) to build the
//! *linkable base* — a stable core TN base that separately-compiled ensembles link into,
//! so we stop recompiling the whole world per variant. hive owns the **wasm HOST**: the
//! base hosts a wasm runtime and loads compiled ensemble modules as sandboxed `.wasm`
//! (Level 2 of the proposal). This is the **inverse** of `crates/r2-hive-wasm` (which is
//! R2 compiled *to* wasm to run in a browser); here R2 is the HOST and ensembles are guests.
//!
//! Canon/authority: `r2-specifications/docs/proposals/R2-LINKABLE-BASE-AND-VARIANT-MANAGEMENT-2026-07-10.md`
//! (A.2 Level-2, A.5 no_std verdicts, B.4 TG-gated load), R2-PLUGIN §12.4 (the ABI seed),
//! R2-COMPILE §11 / §12 OQ-4 (the attach-seams). Hive plan: `docs/WASM-HOST-LINKABLE-BASE-PREP.md`.
//!
//! ## The two surfaces (both = the frozen R2-PLUGIN §12.4 `repr(C)` lowering — Milestone M1)
//! 1. **Module EXPORTS** (module → base): the §12.4 `Plugin` trait lowered to the wasm ABI
//!    (`execute`/`init`/`poll`, all `i32` args + linear-memory `ptr`+`len`, out-params for
//!    written lengths). The base wraps a loaded module so that — from the engine's dispatch —
//!    a wasm module *is* a `Plugin`.
//! 2. **Host IMPORTS** (base → module): the `capabilities.requires` (§12.3) syscall surface
//!    the module may call back into (emit-event, time, RNG, log, declared capability handles).
//!
//! ## ⛔ ABI GATE (spec-first, non-negotiable)
//! Both surfaces are the **frozen §12.4 `repr(C)` ABI** (Milestone M1 — core drafting, specs
//! ratifies). **Nothing ABI-bound is frozen here ahead of that ratification.** [`provisional_abi`]
//! is a clearly-marked PLACEHOLDER whose only job is to prove host *mechanics* (load / instantiate
//! / call / host-import / linear-memory I/O). When specs pings the ratified §12.4 lowering, the
//! placeholder is replaced 1:1 — the mechanics proven here do not change, only the signatures.
//!
//! ## What is proven vs held
//! - **Proven now (ABI-independent):** the runtime backend works end-to-end (see the
//!   `mechanics_spike` test) — this de-risks the wasmtime choice (D.1 std runtime).
//! - **Held for M1:** the real host-import signatures, result struct layouts
//!   (`PluginResponse[128]`/`PluginError[64]` `repr(C)`, `repr(u8)` discriminants, `abi_version`).
//! - **Held for later (M3 tail):** browser backend (native WebAssembly, reuse r2-hive-wasm glue);
//!   esp32s3+PSRAM wasm3; the B.4 TG-gated load gate (verify module sig under the TG update root
//!   BEFORE instantiate — a module is firmware, gets firmware sovereignty).

/// The **PROVISIONAL** module-export/host-import shape used only to prove host mechanics.
///
/// ⚠ This is NOT the frozen ABI. It exists so the [`WasmHost`] can exercise a real
/// load→instantiate→call→host-import→memory round-trip before Milestone M1 lands. Every name
/// here is replaced by the ratified R2-PLUGIN §12.4 `repr(C)` lowering when specs ratifies it.
pub mod provisional_abi {
    /// Import module name a guest uses for the host-import surface (surface #2). Provisional.
    pub const HOST_IMPORT_MODULE: &str = "r2_host";
    /// Provisional host-import: `host_emit(ptr, len)` — the guest hands the host `len` bytes at
    /// linear-memory `ptr`. Stands in for the future §12.4 emit-event / capability calls.
    pub const HOST_EMIT: &str = "host_emit";
    /// Provisional module export the base calls (surface #1), shaped like the §12.4 `execute`
    /// lowering: `execute(command, data_ptr, data_len, out_ptr, out_cap) -> written_len`.
    pub const EXPORT_EXECUTE: &str = "execute";
    /// The linear memory the module must export (wasm convention).
    pub const EXPORT_MEMORY: &str = "memory";

    /// Linear-memory scratch offsets the host uses for the provisional call convention. Chosen
    /// inside the module's first 64 KiB page; the frozen ABI will define its own buffer discipline.
    pub const DATA_PTR: u32 = 1024;
    /// Where the host asks the module to write its response.
    pub const OUT_PTR: u32 = 8192;
    /// Response buffer cap the host advertises (mirrors §12.4's fixed 128 B response buffer intent).
    pub const OUT_CAP: u32 = 128;
}

#[cfg(feature = "std-wasmtime")]
mod wasmtime_backend {
    use crate::provisional_abi as abi;
    use anyhow::{anyhow, Context, Result};
    use wasmtime::{Caller, Engine, Extern, Instance, Linker, Module, Store};

    /// Host-side state carried per invocation. `emitted` captures every provisional `host_emit`
    /// call so a caller (and the mechanics spike) can prove the host-import path fired.
    #[derive(Default)]
    pub struct HostState {
        /// Byte payloads the guest handed the host via `host_emit`, in call order.
        pub emitted: Vec<Vec<u8>>,
    }

    /// The result of running a module's `execute` export once.
    #[derive(Debug, PartialEq, Eq)]
    pub struct ExecOutcome {
        /// Bytes the module wrote to its response buffer (surface #1 return).
        pub output: Vec<u8>,
        /// Payloads the module emitted via the host-import surface (surface #2), in order.
        pub emitted: Vec<Vec<u8>>,
    }

    /// The wasmtime-backed linkable-base host. Holds a shared [`Engine`]; modules are compiled
    /// once via [`WasmHost::load`] and can be executed repeatedly.
    ///
    /// Deps: `wasmtime` (Cranelift JIT + sandbox). Used-by: the base's ensemble dispatch (once
    /// the §12.4 ABI is frozen, a [`LoadedModule`] is wrapped as an `r2-engine::Plugin`).
    pub struct WasmHost {
        engine: Engine,
    }

    /// A compiled module ready to instantiate. Compilation (validation + Cranelift) happens once;
    /// each [`WasmHost::execute`] gets a fresh [`Store`]/[`Instance`] so guest state never leaks
    /// across invocations (isolation by construction).
    pub struct LoadedModule {
        module: Module,
    }

    impl WasmHost {
        /// Construct a host with the default (sandboxed) wasmtime engine.
        pub fn new() -> Result<Self> {
            Ok(Self { engine: Engine::default() })
        }

        /// Compile a module from a `.wasm` binary **or** WebAssembly text (`.wat`) — wasmtime's
        /// `wat` feature accepts both. Validation happens here; a malformed module fails fast.
        ///
        /// NOTE (B.4, held for later): the real loader MUST verify the module's signature under
        /// the TG update root BEFORE this compile/instantiate step — a module is firmware.
        pub fn load(&self, wasm_or_wat: &[u8]) -> Result<LoadedModule> {
            let module = Module::new(&self.engine, wasm_or_wat).context("compile wasm module")?;
            Ok(LoadedModule { module })
        }

        /// Run a module's provisional `execute` export once with `command` + `data`, providing the
        /// provisional `host_emit` import. Returns what the module wrote + what it emitted.
        ///
        /// This is the mechanics primitive; the frozen §12.4 lowering replaces the signatures, not
        /// the flow (write input → call export → module calls host imports → read response).
        pub fn execute(&self, module: &LoadedModule, command: u32, data: &[u8]) -> Result<ExecOutcome> {
            let mut store = Store::new(&self.engine, HostState::default());
            let mut linker = Linker::new(&self.engine);

            // Surface #2 (provisional): the guest calls host_emit(ptr,len) to hand us bytes.
            linker.func_wrap(
                abi::HOST_IMPORT_MODULE,
                abi::HOST_EMIT,
                |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<()> {
                    let mem = match caller.get_export(abi::EXPORT_MEMORY) {
                        Some(Extern::Memory(m)) => m,
                        _ => return Err(anyhow!("guest exports no linear memory")),
                    };
                    let (ptr, len) = (ptr as usize, len as usize);
                    let bytes = mem
                        .data(&caller)
                        .get(ptr..ptr.saturating_add(len))
                        .ok_or_else(|| anyhow!("host_emit ptr/len out of bounds"))?
                        .to_vec();
                    caller.data_mut().emitted.push(bytes);
                    Ok(())
                },
            )?;

            let instance: Instance = linker
                .instantiate(&mut store, &module.module)
                .context("instantiate module")?;
            let memory = instance
                .get_memory(&mut store, abi::EXPORT_MEMORY)
                .ok_or_else(|| anyhow!("module exports no memory"))?;

            // Write the input into the guest's linear memory (surface #1 in-params).
            memory
                .write(&mut store, abi::DATA_PTR as usize, data)
                .context("write input into guest memory")?;

            // Surface #1: execute(command, data_ptr, data_len, out_ptr, out_cap) -> written_len.
            let execute = instance
                .get_typed_func::<(i32, i32, i32, i32, i32), i32>(&mut store, abi::EXPORT_EXECUTE)
                .context("module missing execute export")?;
            let written = execute.call(
                &mut store,
                (
                    command as i32,
                    abi::DATA_PTR as i32,
                    data.len() as i32,
                    abi::OUT_PTR as i32,
                    abi::OUT_CAP as i32,
                ),
            )?;
            if written < 0 || written as u32 > abi::OUT_CAP {
                return Err(anyhow!("module returned invalid written_len {written}"));
            }

            let mut output = vec![0u8; written as usize];
            memory
                .read(&store, abi::OUT_PTR as usize, &mut output)
                .context("read response from guest memory")?;

            Ok(ExecOutcome { output, emitted: store.into_data().emitted })
        }
    }
}

#[cfg(feature = "std-wasmtime")]
pub use wasmtime_backend::{ExecOutcome, HostState, LoadedModule, WasmHost};
