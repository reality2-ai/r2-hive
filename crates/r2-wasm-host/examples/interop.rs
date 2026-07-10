//! True-e2e L2 interop runner (Milestone M3 capstone). Loads a **real r2-forge-emitted** Echo
//! `.wasm` (path via argv[1]) into `r2-wasm-host` and verifies it **byte-exact** against the shared
//! conformance vectors in r2-core `crates/r2-engine/WASM-INTEROP.md` (R2-PLUGIN v0.10 §12.4.3.1/.2).
//!
//! Run: `cargo run --example interop -- /path/to/echo.wasm`
//!
//! Proves the whole L2 mechanism end-to-end: emit (core r2-forge) → load → inert-instantiate →
//! read `__r2_scratch_ptr()`/`len()` → `__r2_abi_hash` 32 B exact-match load gate → `r2_init`/
//! `r2_execute` → the 136 B `AbiResult` image the host reads from the scratch region == the vector.
use r2_wasm_host::{PluginResult, WasmHost};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: interop <echo.wasm>");
    let wasm = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let host = WasmHost::new().expect("engine");
    let module = host.load(&wasm).expect("compile echo.wasm");
    // instantiate applies the §12.4.3.1 load gate: abi_version==1, scratch>=1004, __r2_abi_hash
    // 32 B exact-match vs host v1, r2_init Ok. A failure here IS the load gate doing its job.
    let mut inst = host
        .instantiate(&module)
        .expect("instantiate — load gate (scratch read + abi_hash exact + init Ok)");

    let mut fails = 0u32;
    macro_rules! chk {
        ($name:expr, $cond:expr, $got:expr) => {{
            let ok = $cond;
            println!("[{}] {:<38} {}", if ok { "PASS" } else { "FAIL" }, $name, $got);
            if !ok {
                fails += 1;
            }
        }};
    }

    chk!("abi_version == 1", inst.abi_version() == 1, format!("= {}", inst.abi_version()));
    chk!("plugin_id == 0x2a", inst.plugin_id() == 0x2a, format!("= 0x{:02x}", inst.plugin_id()));
    chk!("scratch_len >= 1004", inst.scratch_len() >= 1004, format!("= {}", inst.scratch_len()));

    // r2_execute(0x01, [0xAA,0xBB]) → canonical 136 B AbiResult image (tag0, data@4=01aabb, len@132=3).
    let mut expected = [0u8; 136];
    expected[4] = 0x01;
    expected[5] = 0xAA;
    expected[6] = 0xBB;
    expected[132] = 3; // len u16 LE = 3
    let img = inst.execute_raw(0x01, &[0xAA, 0xBB]).expect("execute_raw");
    chk!("execute AbiResult image byte-exact", img == expected, format!("= {}", hex(&img)));

    match inst.execute(0x01, &[0xAA, 0xBB]).expect("execute") {
        PluginResult::Ok(d) => {
            chk!("execute parsed Ok([01,AA,BB])", d == vec![0x01, 0xAA, 0xBB], format!("= {:02x?}", d))
        }
        other => chk!("execute parsed Ok", false, format!("= {other:?}")),
    }

    let name = inst.name().expect("name");
    chk!("name == echo", name == "echo", format!("= {name:?}"));
    let poll = inst.poll().expect("poll");
    chk!("poll == None", poll.is_none(), format!("= {poll:?}"));

    println!();
    if fails == 0 {
        println!("INTEROP GREEN — real r2-forge Echo module round-trips byte-exact against r2-wasm-host.");
    } else {
        println!("INTEROP RED — {fails} check(s) failed.");
        std::process::exit(1);
    }
}
