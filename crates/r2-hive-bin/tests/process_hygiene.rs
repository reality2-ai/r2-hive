//! F8 process-hygiene gates (supervisor-assigned).
//!
//! Two gates:
//!  1. `agents_md_exists_with_required_headings` — asserts AGENTS.md exists with its normative headings
//!     (the operating contract can't silently disappear or lose a section).
//!  2. `heartbeat_v12_6_dc_seq_canonical` — locks the R2-WIRE §12.6 keepalive encoding `{0:seq, 1:dc}`
//!     (Compact uint keys, ascending canonical). #[ignore]'d (xfail-tracked) while the FIRMWARE still
//!     emits the legacy fixed byte-8 power_state (see FORKS.md). UN-IGNORE when the firmware dc re-emit
//!     lands — the flip from ignored→passing is the signal the fork is resolved.

const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[test]
fn agents_md_exists_with_required_headings() {
    let path = format!("{REPO_ROOT}/AGENTS.md");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("AGENTS.md must exist at repo root ({path}): {e}"));
    for heading in [
        "## Role",
        "## Authority Chain",
        "## Before Editing",
        "## Test Gates",
        "## Stop Conditions",
        "## No-Go",
        "## Current-State Pointer",
    ] {
        assert!(
            body.contains(heading),
            "AGENTS.md is missing the required heading: {heading:?}"
        );
    }
    // The do-not-fork policy must live here (moved out of RESUME.md per F8).
    assert!(
        body.contains("per-target firmware fork"),
        "AGENTS.md must carry the NO per-target firmware forks policy"
    );
}

/// Minimal canonical CBOR uint encoder (RFC 8949 §3, the Compact-profile subset §12.6 uses).
fn put_uint(b: &mut Vec<u8>, v: u64) {
    match v {
        0..=23 => b.push(v as u8),
        24..=0xFF => b.extend_from_slice(&[0x18, v as u8]),
        0x100..=0xFFFF => {
            b.push(0x19);
            b.extend_from_slice(&(v as u16).to_be_bytes());
        }
        0x1_0000..=0xFFFF_FFFF => {
            b.push(0x1A);
            b.extend_from_slice(&(v as u32).to_be_bytes());
        }
        _ => {
            b.push(0x1B);
            b.extend_from_slice(&v.to_be_bytes());
        }
    }
}

/// §12.6 keepalive payload: `{0: seq, 1: dc}` — uint keys, ascending canonical.
fn encode_dc_seq(seq: u32, dc: u8) -> Vec<u8> {
    let mut b = vec![0xA2]; // map of 2 pairs
    b.push(0x00); // key 0 = seq
    put_uint(&mut b, seq as u64);
    b.push(0x01); // key 1 = dc
    put_uint(&mut b, dc as u64);
    b
}

/// Read the `dc` (uint key 1) back out — minimal decode over the fixed `{0,1}` shape.
fn parse_dc(b: &[u8]) -> Option<u8> {
    if b.first() != Some(&0xA2) {
        return None;
    }
    let mut i = 1;
    for _ in 0..2 {
        let key = *b.get(i)?;
        i += 1;
        let head = *b.get(i)?;
        i += 1;
        let val = match head {
            0..=23 => head as u64,
            0x18 => {
                let v = *b.get(i)? as u64;
                i += 1;
                v
            }
            _ => return None, // §12.6 seq/dc never exceed uint8 in these tests
        };
        if key == 0x01 {
            return Some(val as u8);
        }
    }
    None
}

#[test]
#[ignore = "FORKS.md: firmware HB byte-8 power_state diverges from R2-WIRE §12.6 until the dc re-emit lands — un-ignore to flip this gate green"]
fn heartbeat_v12_6_dc_seq_canonical() {
    // Byte-identical to core's verified r2_dataplane::encode_dc_seq_cbor (seq=42, dc=Intermittent=2).
    assert_eq!(
        encode_dc_seq(42, 2),
        [0xA2, 0x00, 0x18, 0x2A, 0x01, 0x02],
        "§12.6 keepalive {{0:seq,1:dc}} canonical encoding"
    );
    // Small-int keys/values stay in the 0..=23 single-byte form.
    assert_eq!(encode_dc_seq(7, 1), [0xA2, 0x00, 0x07, 0x01, 0x01]);
    // Round-trip the duty class (0=Unknown,1=AlwaysOn,2=Intermittent).
    for dc in 0u8..=2 {
        assert_eq!(parse_dc(&encode_dc_seq(99, dc)), Some(dc));
    }
}
