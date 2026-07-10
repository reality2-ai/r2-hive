//! Runtime-MECHANICS de-risk spike (Milestone M3, ABI-independent).
//!
//! Proves the wasmtime std-host backend works END-TO-END with a PROVISIONAL ABI:
//! load → instantiate → call a module export (surface #1) → the module calls a host
//! import (surface #2) → linear-memory I/O both ways. This de-risks the D.1 std runtime
//! choice (wasmtime) WITHOUT committing to the frozen R2-PLUGIN §12.4 ABI (Milestone M1) —
//! the signatures here are placeholders; the flow is what M1 will not change.
//!
//! The guest is written inline in WebAssembly text (no external toolchain): on `execute` it
//! (1) calls `host_emit(data_ptr, data_len)` — proving surface #2 — then (2) copies the input
//! into the response buffer capped at `out_cap` and returns the written length — surface #1.
#![cfg(feature = "std-wasmtime")]

use r2_wasm_host::WasmHost;

/// Minimal guest exercising both surfaces. Mirrors the provisional `execute` shape
/// `(command, data_ptr, data_len, out_ptr, out_cap) -> written_len`.
const SPIKE_WAT: &str = r#"
(module
  (import "r2_host" "host_emit" (func $host_emit (param i32 i32)))
  (memory (export "memory") 1)
  (func (export "execute")
    (param $cmd i32) (param $dptr i32) (param $dlen i32) (param $optr i32) (param $ocap i32)
    (result i32)
    (local $i i32)
    (local $n i32)
    ;; surface #2: hand the host the input bytes
    (call $host_emit (local.get $dptr) (local.get $dlen))
    ;; n = min(dlen, ocap)  — respect the caller-provided buffer cap
    (local.set $n
      (select (local.get $dlen) (local.get $ocap)
              (i32.le_u (local.get $dlen) (local.get $ocap))))
    ;; copy data[0..n] -> out[0..n]
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (i32.store8
          (i32.add (local.get $optr) (local.get $i))
          (i32.load8_u (i32.add (local.get $dptr) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    ;; surface #1: return the written length
    (local.get $n))
)
"#;

#[test]
fn mechanics_spike_roundtrip() {
    let host = WasmHost::new().expect("build wasmtime engine");
    let module = host.load(SPIKE_WAT.as_bytes()).expect("compile spike module");
    let input = b"r2-linkable-base";

    let outcome = host.execute(&module, 0x01, input).expect("execute");

    // surface #1: the module wrote the input back into the host-provided response buffer.
    assert_eq!(outcome.output, input, "module response should echo input");
    // surface #2: the module called host_emit exactly once with the full input.
    assert_eq!(outcome.emitted, vec![input.to_vec()], "host_emit should have fired with input");
}

#[test]
fn respects_out_cap_truncation() {
    // Input (200 B) longer than the provisional OUT_CAP (128 B): surface #1 output is capped,
    // but surface #2 (host_emit) still receives the FULL payload — they are independent paths.
    let host = WasmHost::new().unwrap();
    let module = host.load(SPIKE_WAT.as_bytes()).unwrap();
    let input = vec![0xABu8; 200];

    let outcome = host.execute(&module, 0x02, &input).unwrap();

    assert_eq!(outcome.output.len(), 128, "response must be capped at out_cap");
    assert_eq!(outcome.output, vec![0xABu8; 128]);
    assert_eq!(outcome.emitted, vec![input.clone()], "host_emit sees the full uncapped input");
}

#[test]
fn rejects_malformed_module() {
    let host = WasmHost::new().unwrap();
    // Not valid wasm/wat — load must fail fast (validation), never silently.
    assert!(host.load(b"this is not wasm").is_err(), "malformed module must be rejected");
}
