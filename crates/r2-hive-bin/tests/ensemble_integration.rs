//! Integration tests for `r2.mgmt.ensemble.*` over the Unix socket.
//!
//! Spins up a daemon with a HiveState attached, registers a tiny Rust
//! sentant factory, then drives load → list → info → stop through the
//! mgmt socket using the public `build_*_request` helpers.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use tokio::net::UnixStream;

use r2_cbor::{Decoder, Item};
use r2_def::SentantDef;
use r2_engine::{ActionBuf, Event, Sentant, StateId};
use r2_ensemble::{BoxedSentant, LoadError, SentantFactory};
use r2_fnv::r2_hash;
use r2_hive::hive::HiveState;
use r2_hive::mgmt::ensemble::{
    build_info_request, build_list_request, build_load_request, build_stop_request,
};
use r2_hive::mgmt::framing::{read_frame, write_frame};
use r2_hive::mgmt::{socket, state::DaemonState};

/// Bare echo sentant just so the registry has something to instantiate.
struct Echo {
    class_hash: u32,
    subs: Vec<u32>,
}
impl Sentant for Echo {
    fn handle_event(&mut self, _ev: &Event, _b: &mut ActionBuf) {}
    fn state(&self) -> StateId { 0 }
    fn class_hash(&self) -> u32 { self.class_hash }
    fn name(&self) -> &str { "echo" }
    fn subscriptions(&self) -> &[u32] { &self.subs }
}

struct EchoFactory {
    builds: Arc<AtomicU32>,
}
impl SentantFactory for EchoFactory {
    fn build(&self, def: &SentantDef) -> Result<BoxedSentant, LoadError> {
        if !def.name.starts_with("echo") {
            return Err(LoadError::NoFactory {
                name: def.name.clone(),
                reason: "EchoFactory only".into(),
            });
        }
        let cls = def.class.as_deref().unwrap_or(&def.name);
        let class_hash =
            r2_hash(cls).map_err(|_| LoadError::BadEventClass(cls.into()))?;
        let mut subs = Vec::new();
        for a in &def.automations {
            for t in &a.transitions {
                if let Ok(h) = r2_hash(&t.event) {
                    if !subs.contains(&h) {
                        subs.push(h);
                    }
                }
            }
        }
        self.builds.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(Echo { class_hash, subs }))
    }
}

const SCORE: &str = r#"
ensemble:
  name: notekeeper-test
  description: integration fixture
  version: "0.1.0"
  ensemble_version: "0.1"
  sentants:
    - name: echo
      class: nz.test.echo
      description: test sentant
      automations:
        - name: main
          transitions:
            - event: note.create
              from: "*"
"#;

struct Setup {
    handle: socket::ServerHandle,
    socket_path: std::path::PathBuf,
    builds: Arc<AtomicU32>,
    hive: Arc<HiveState>,
}

async fn setup() -> Setup {
    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    // Leak the tempdir so the socket path survives the test (the
    // tokio::spawn'd listener owns it).
    std::mem::forget(tmp);

    let hive = Arc::new(HiveState::new(0xCAFE_BABE, 64, 16));
    let builds = Arc::new(AtomicU32::new(0));
    hive.ensembles.register_factory(Arc::new(EchoFactory {
        builds: builds.clone(),
    }));

    let daemon = DaemonState::new();
    daemon.attach_hive_state(hive.clone());
    let handle = socket::spawn(socket_path.clone(), daemon)
        .await
        .expect("spawn daemon");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Setup {
        handle,
        socket_path,
        builds,
        hive,
    }
}

async fn round_trip(
    socket_path: &std::path::Path,
    request: Vec<u8>,
) -> Vec<u8> {
    let mut stream = UnixStream::connect(socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    write_frame(&mut writer, &request).await.expect("write");
    read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty")
}

fn extract_uint(payload: &[u8], target_key: u64) -> Option<u64> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let k = dec.next().ok()?;
        let v = dec.next().ok()?;
        if let Item::UInt(kk) = k {
            if kk == target_key {
                if let Item::UInt(n) = v {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn extract_text(payload: &[u8], target_key: u64) -> Option<String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let k = dec.next().ok()?;
        let v = dec.next().ok()?;
        if let Item::UInt(kk) = k {
            if kk == target_key {
                if let Item::Text(s) = v {
                    return std::str::from_utf8(s).ok().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

#[tokio::test]
async fn ensemble_load_list_info_stop_round_trip() {
    let s = setup().await;
    let sock = s.socket_path.clone();
    let builds = s.builds.clone();
    let hive = s.hive.clone();
    let handle = s.handle;

    // 1. load
    let resp = round_trip(&sock, build_load_request(1, "yaml", SCORE)).await;
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.ensemble.load").unwrap(),
        "load response event class"
    );
    assert_eq!(extract_text(parsed.payload, 1).as_deref(), Some("notekeeper-test"));
    assert_eq!(extract_uint(parsed.payload, 3), Some(1)); // sentant_count
    assert_eq!(builds.load(Ordering::SeqCst), 1);

    // 2. list
    let resp = round_trip(&sock, build_list_request(2)).await;
    let parsed = r2_wire::decode_extended(&resp).expect("decode list");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.ensemble.list").unwrap()
    );

    // 3. info
    let resp = round_trip(&sock, build_info_request(3, "notekeeper-test")).await;
    let parsed = r2_wire::decode_extended(&resp).expect("decode info");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.ensemble.info").unwrap()
    );
    assert_eq!(extract_uint(parsed.payload, 2), Some(0)); // status: Healthy

    // 4. stop
    let resp = round_trip(&sock, build_stop_request(4, "notekeeper-test")).await;
    let parsed = r2_wire::decode_extended(&resp).expect("decode stop");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.ensemble.stop").unwrap(),
        "stop response event class"
    );
    assert_eq!(hive.ensembles.list().len(), 0);

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn ensemble_load_returns_error_for_garbage() {
    let s = setup().await;
    let sock = s.socket_path.clone();
    let handle = s.handle;

    let resp = round_trip(&sock, build_load_request(1, "yaml", "not yaml: : :")).await;
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.event.error").unwrap()
    );

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn ensemble_info_for_unknown_returns_not_loaded_error() {
    let s = setup().await;
    let sock = s.socket_path.clone();
    let handle = s.handle;

    let resp = round_trip(&sock, build_info_request(1, "does-not-exist")).await;
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.event.error").unwrap()
    );
    assert_eq!(extract_text(parsed.payload, 1).as_deref(), Some("not_loaded"));

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}
