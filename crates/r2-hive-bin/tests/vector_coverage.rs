//! Vector-coverage regression check (gap #3 from `docs/CONFORMANCE.md`).
//!
//! For each canonical vector file r2-hive's tests reference, the
//! "expected" map below pins:
//!   - the minimum total vector count in the upstream file (so an
//!     upstream removal doesn't silently shrink our denominator), and
//!   - the minimum number of vector IDs referenced from r2-hive's
//!     test sources (so a refactor that drops a fixture doesn't go
//!     unnoticed).
//!
//! When the conformance posture changes intentionally, edit these
//! numbers and the matching documentation in CONFORMANCE.md in the
//! same change.

use std::collections::HashSet;
use std::path::PathBuf;

const VECTORS_ROOT: &str = "../../../r2-specifications/testing/test-vectors";
const TEST_SOURCES: &[&str] = &[
    "src/usb.rs",
    "tests/host_api_conformance.rs",
    "tests/web_plugin_integration.rs",
    "tests/web_auth_integration.rs",
    "tests/web_plugin_load.rs",
    "tests/ensemble_integration.rs",
    "tests/service_integration.rs",
    "tests/mgmt_integration.rs",
];

/// Per-spec coverage targets.
struct Target {
    /// File name under VECTORS_ROOT.
    file: &'static str,
    /// Lower bound on total vectors in the upstream file (regardless
    /// of whether r2-hive references them all).
    min_upstream: usize,
    /// Lower bound on vector IDs referenced from r2-hive test sources.
    /// 0 means r2-hive doesn't reference vectors by ID — coverage is
    /// behavioural rather than fixture-based.
    min_referenced: usize,
}

const TARGETS: &[Target] = &[
    Target {
        file: "r2-host-api-vectors.json",
        min_upstream: 28,
        // host_api_conformance.rs replays every vector by iterating
        // the JSON; no ID-by-ID references.
        min_referenced: 0,
    },
    Target {
        file: "r2-usb-vectors.json",
        min_upstream: 13,
        // src/usb.rs cites TV1, TV2, TV3, TV5, TV6, TV7, TV9, TV11, TV12.
        min_referenced: 9,
    },
    Target {
        file: "r2-usb-pair-vectors.json",
        min_upstream: 10,
        // Pinned but not yet replayed by host code (Phase USB-2).
        min_referenced: 0,
    },
    Target {
        file: "r2-plugin-web-vectors.json",
        // 9 numbered vectors plus the spec/fixtures overhead.
        min_upstream: 9,
        // web_plugin_integration covers WEB-MOUNT-AND-FETCH (mount_serves_),
        // WEB-UNMOUNT-ON-STOP (unmount_yields_404),
        // WEB-CSP-DEFAULT-FORBIDS-INLINE-SCRIPT (mount_serves_),
        // WEB-AUTH-401-WITHOUT-COOKIE (web_auth_integration),
        // WEB-ATOMIC-RELOAD (atomic_remount_),
        // WEB-ESCAPING-SYMLINK-REJECTED (escaping_symlink_),
        // WEB-BAD-SCORE-CHANNEL-TARGET (channel_target_validation_via_def_parser).
        // Tests don't cite vector IDs in source — count is 0 here,
        // but min_upstream tracks that the spec hasn't shrunk.
        min_referenced: 0,
    },
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn vectors_path(file: &str) -> PathBuf {
    workspace_root().join(VECTORS_ROOT).join(file)
}

fn read_test_sources() -> String {
    let mut out = String::new();
    for f in TEST_SOURCES {
        let p = workspace_root().join(f);
        if let Ok(s) = std::fs::read_to_string(&p) {
            out.push_str(&s);
            out.push('\n');
        } else {
            // Missing source files are not a hard error — the suite
            // may have been pruned. The min_referenced floors below
            // catch it if a file's vectors became unreachable.
            eprintln!("vector_coverage: missing source {p:?}");
        }
    }
    out
}

fn count_total_vectors(file: &str) -> usize {
    let p = vectors_path(file);
    let s = match std::fs::read_to_string(&p) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let v: serde_json::Value = serde_json::from_str(&s).expect("vector file is JSON");
    let mut count = 0;
    if let Some(arr) = v.get("vectors").and_then(|v| v.as_array()) {
        count += arr.len();
    }
    // Files that nest vectors under different keys (e.g. r2-fnv splits
    // them across conformance_vectors / canonicalisation_vectors); for
    // the four files this regression test cares about, the top-level
    // "vectors" array is the canonical container.
    count
}

fn count_referenced_ids_for(file: &str, sources: &str) -> usize {
    // Read the JSON, list every vector ID, and count how many appear
    // verbatim in any of the test source files. This is how we detect
    // a refactor that removes a vector citation by name.
    let p = vectors_path(file);
    let s = match std::fs::read_to_string(&p) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let v: serde_json::Value = serde_json::from_str(&s).expect("vector file is JSON");
    let arr = match v.get("vectors").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return 0,
    };
    let mut ids: HashSet<String> = HashSet::new();
    for entry in arr {
        if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
            ids.insert(id.to_string());
        }
    }
    ids.iter().filter(|id| sources.contains(id.as_str())).count()
}

#[test]
fn vectors_root_exists() {
    let root = workspace_root().join(VECTORS_ROOT);
    assert!(
        root.is_dir(),
        "expected vectors root at {root:?} — adjust VECTORS_ROOT if r2-specifications is elsewhere"
    );
}

#[test]
fn upstream_vector_counts_have_not_regressed() {
    for t in TARGETS {
        let n = count_total_vectors(t.file);
        assert!(
            n >= t.min_upstream,
            "{}: upstream vector count regressed: {n} < {min}",
            t.file,
            min = t.min_upstream
        );
    }
}

#[test]
fn referenced_vector_ids_have_not_dropped() {
    let sources = read_test_sources();
    for t in TARGETS {
        if t.min_referenced == 0 {
            continue;
        }
        let n = count_referenced_ids_for(t.file, &sources);
        let sources_searched = TEST_SOURCES.join(", ");
        assert!(
            n >= t.min_referenced,
            "{file}: referenced-IDs regressed: {n} < {min} (sources searched: {sources_searched})",
            file = t.file,
            min = t.min_referenced,
        );
    }
}

/// Soft check: list known-but-unreferenced vectors so the human
/// reviewing CONFORMANCE.md sees the gap. Doesn't fail — running with
/// `--nocapture` shows the list.
#[test]
fn list_unreferenced_vectors() {
    let sources = read_test_sources();
    for t in TARGETS {
        let p = vectors_path(t.file);
        let s = match std::fs::read_to_string(&p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: serde_json::Value = match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let arr = match v.get("vectors").and_then(|v| v.as_array()) {
            Some(a) => a,
            None => continue,
        };
        let mut missing = Vec::new();
        for entry in arr {
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                if !sources.contains(id) {
                    missing.push(id.to_string());
                }
            }
        }
        if !missing.is_empty() {
            println!(
                "{}: {} vector(s) not referenced by ID in tests: {:?}",
                t.file,
                missing.len(),
                missing
            );
        }
    }
}
