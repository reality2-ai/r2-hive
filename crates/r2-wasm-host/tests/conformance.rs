//! §12.4.3.1 CONFORMANCE test (Milestone M3) — bound to the RATIFIED WASM projection.
//!
//! Exercises `r2-wasm-host` against a hand-written §12.4.3.1-conformant guest (inline WAT, no
//! external toolchain): the 8 module exports, the 136 B native-image `AbiResult`, and the
//! full-32 B `__r2_abi_hash` load gate. It replaces the earlier provisional-ABI mechanics spike;
//! the real r2-forge-emitted module (core) will be interop-tested against this same host next.
#![cfg(feature = "std-wasmtime")]

use r2_wasm_host::{PluginResult, WasmHost};

/// A §12.4.3.1-conformant pure-compute "echo" plugin (id=7, abi_version=1). Exports the 8 required
/// symbols; `r2_execute` copies input into `AbiResult::Ok.data` (capped at 128); `__r2_abi_hash`
/// exports the v1 hash. The 32 B hash lives in a module data section at offset 0 (module-owned);
/// the host's scratch offsets (>=256) do not collide with it.
const CONFORMANT_WAT: &str = r#"
(module
  (memory (export "memory") 1)
  ;; v0.8: abi_version + plugin_id are value-returning funcs () -> i32 (not globals)
  (func (export "__r2_abi_version") (result i32) (i32.const 1))
  (func (export "__r2_plugin_id") (result i32) (i32.const 7))
  ;; §12.4.3.2: host-scratch region reserved at [16384, 16384+2048) — module never touches it.
  (func (export "__r2_scratch_ptr") (result i32) (i32.const 16384))
  (func (export "__r2_scratch_len") (result i32) (i32.const 2048))
  ;; ABI_HASH_WASM (32 B, R2-PLUGIN §12.4.3.3 v0.14 split) at offset 0 — 5b6c9317beca30c553a0bb2ff0fd69c5ba704097957d0a75cf8a6b20161c9717
  (data (i32.const 0) "\5b\6c\93\17\be\ca\30\c5\53\a0\bb\2f\f0\fd\69\c5\ba\70\40\97\95\7d\0a\75\cf\8a\6b\20\16\1c\97\17")
  ;; __r2_abi_hash(out_ptr): copy the 32 B hash from [0..32] to out_ptr
  (func (export "__r2_abi_hash") (param $out i32)
    (local $i i32)
    (block $d (loop $l
      (br_if $d (i32.ge_u (local.get $i) (i32.const 32)))
      (i32.store8 (i32.add (local.get $out) (local.get $i)) (i32.load8_u (local.get $i)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $l))))
  ;; r2_init(result_ptr): write AbiResult::Ok(empty) — tag 0, len 0
  (func (export "r2_init") (param $r i32)
    (i32.store8 (local.get $r) (i32.const 0))
    (i32.store16 (i32.add (local.get $r) (i32.const 132)) (i32.const 0)))
  ;; r2_execute(cmd, dptr, dlen, rptr): AbiResult::Ok, data = input[0..min(dlen,128)]
  (func (export "r2_execute") (param $cmd i32) (param $dptr i32) (param $dlen i32) (param $rptr i32)
    (local $n i32) (local $i i32)
    (i32.store8 (local.get $rptr) (i32.const 0)) ;; tag Ok
    (local.set $n (select (local.get $dlen) (i32.const 128) (i32.le_u (local.get $dlen) (i32.const 128))))
    (local.set $i (i32.const 0))
    (block $d (loop $l
      (br_if $d (i32.ge_u (local.get $i) (local.get $n)))
      (i32.store8 (i32.add (i32.add (local.get $rptr) (i32.const 4)) (local.get $i))
                  (i32.load8_u (i32.add (local.get $dptr) (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $l)))
    (i32.store16 (i32.add (local.get $rptr) (i32.const 132)) (local.get $n))) ;; len = n
  ;; r2_poll(ev_out, buf, cap) -> i32 : no events -> -1
  (func (export "r2_poll") (param $ev i32) (param $buf i32) (param $cap i32) (result i32)
    (i32.const -1))
  ;; r2_name(buf, cap) -> i32 : "spike" (5 bytes)
  (func (export "r2_name") (param $buf i32) (param $cap i32) (result i32)
    (i32.store8 (local.get $buf) (i32.const 115))                       ;; s
    (i32.store8 (i32.add (local.get $buf) (i32.const 1)) (i32.const 112)) ;; p
    (i32.store8 (i32.add (local.get $buf) (i32.const 2)) (i32.const 105)) ;; i
    (i32.store8 (i32.add (local.get $buf) (i32.const 3)) (i32.const 107)) ;; k
    (i32.store8 (i32.add (local.get $buf) (i32.const 4)) (i32.const 101)) ;; e
    (i32.const 5))
)
"#;

/// Same module but with one ABI_HASH_WASM byte flipped (first byte 5b→00) — the load gate MUST refuse it (Ruling 2).
const WRONG_HASH_WAT: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "__r2_abi_version") (result i32) (i32.const 1))
  (func (export "__r2_plugin_id") (result i32) (i32.const 7))
  (func (export "__r2_scratch_ptr") (result i32) (i32.const 16384))
  (func (export "__r2_scratch_len") (result i32) (i32.const 2048))
  (data (i32.const 0) "\00\6c\93\17\be\ca\30\c5\53\a0\bb\2f\f0\fd\69\c5\ba\70\40\97\95\7d\0a\75\cf\8a\6b\20\16\1c\97\17")
  (func (export "__r2_abi_hash") (param $out i32)
    (local $i i32)
    (block $d (loop $l
      (br_if $d (i32.ge_u (local.get $i) (i32.const 32)))
      (i32.store8 (i32.add (local.get $out) (local.get $i)) (i32.load8_u (local.get $i)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $l))))
  (func (export "r2_init") (param $r i32)
    (i32.store8 (local.get $r) (i32.const 0))
    (i32.store16 (i32.add (local.get $r) (i32.const 132)) (i32.const 0)))
  (func (export "r2_execute") (param $cmd i32) (param $dptr i32) (param $dlen i32) (param $rptr i32) (nop))
  (func (export "r2_poll") (param $ev i32) (param $buf i32) (param $cap i32) (result i32) (i32.const -1))
  (func (export "r2_name") (param $buf i32) (param $cap i32) (result i32) (i32.const 0))
)
"#;

#[test]
fn conformant_module_binds_and_executes() {
    let host = WasmHost::new().expect("engine");
    let module = host.load(CONFORMANT_WAT.as_bytes()).expect("compile");
    let mut inst = host.instantiate(&module).expect("instantiate (abi_hash gate + init)");

    assert_eq!(inst.abi_version(), 1);
    assert_eq!(inst.plugin_id(), 7);
    assert_eq!(inst.name().unwrap(), "spike");

    let input = b"r2-linkable-base";
    match inst.execute(0x01, input).expect("execute") {
        PluginResult::Ok(data) => assert_eq!(data, input, "Ok.data echoes input"),
        other => panic!("expected Ok, got {other:?}"),
    }
    assert_eq!(inst.poll().unwrap(), None, "no pending events");
}

#[test]
fn execute_truncates_at_128() {
    let host = WasmHost::new().unwrap();
    let module = host.load(CONFORMANT_WAT.as_bytes()).unwrap();
    let mut inst = host.instantiate(&module).unwrap();

    let input = vec![0xABu8; 200];
    match inst.execute(0x02, &input).unwrap() {
        PluginResult::Ok(data) => {
            assert_eq!(data.len(), 128, "AbiResponse data caps at 128");
            assert_eq!(data, vec![0xABu8; 128]);
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn wrong_abi_hash_is_refused() {
    // Ruling 2: exact-match load gate. A module whose __r2_abi_hash != host v1 MUST NOT instantiate.
    let host = WasmHost::new().unwrap();
    let module = host.load(WRONG_HASH_WAT.as_bytes()).expect("compiles fine (validation passes)");
    match host.instantiate(&module) {
        Ok(_) => panic!("mismatched abi_hash must refuse to instantiate (Ruling 2)"),
        Err(e) => assert!(
            e.to_string().contains("abi_hash mismatch"),
            "error should name the abi_hash gate, got: {e}"
        ),
    }
}

#[test]
fn malformed_module_rejected() {
    let host = WasmHost::new().unwrap();
    assert!(host.load(b"this is not wasm").is_err(), "malformed module must fail compile");
}
