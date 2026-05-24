//! End-to-end test for path-based `r2.mgmt.ensemble.load` with a web
//! plugin: load a score from disk, the daemon mounts the bundle, GETs
//! return the bundle, then `ensemble.stop` unmounts and GETs return 404.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tokio::net::UnixStream;

use r2_def::SentantDef;
use r2_engine::{ActionBuf, Event, Sentant, StateId};
use r2_ensemble::{BoxedSentant, LoadError, SentantFactory};
use r2_fnv::r2_hash;
use r2_hive::hive::HiveState;
use r2_hive::mgmt::ensemble::{build_load_request_from_path, build_stop_request};
use r2_hive::mgmt::framing::{read_frame, write_frame};
use r2_hive::mgmt::{socket, state::DaemonState};

struct NoopSentant {
    class_hash: u32,
}
impl Sentant for NoopSentant {
    fn handle_event(&mut self, _ev: &Event, _b: &mut ActionBuf) {}
    fn state(&self) -> StateId { 0 }
    fn class_hash(&self) -> u32 { self.class_hash }
    fn name(&self) -> &str { "noop" }
    fn subscriptions(&self) -> &[u32] { &[] }
}

struct NoopFactory(Arc<AtomicU32>);
impl SentantFactory for NoopFactory {
    fn build(&self, def: &SentantDef) -> Result<BoxedSentant, LoadError> {
        let cls = def.class.as_deref().unwrap_or(&def.name);
        let class_hash =
            r2_hash(cls).map_err(|_| LoadError::BadEventClass(cls.into()))?;
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(NoopSentant { class_hash }))
    }
}

const SCORE: &str = r#"
ensemble:
  name: web-load-test
  description: web-plugin auto-mount fixture
  version: "0.1.0"
  ensemble_version: "0.1"
  sentants:
    - name: stub
      class: nz.test.stub
      description: stub
      automations:
        - name: main
          initial: idle
          transitions: []
  plugins:
    - name: ui
      type: "web"
      bundle: "ui/"
"#;

#[tokio::test]
async fn ensemble_load_path_mounts_web_plugin_and_stop_unmounts() {
    // Build the score + bundle on disk.
    let tmp = tempfile::tempdir().expect("tempdir");
    let score_path = tmp.path().join("ensemble.yaml");
    std::fs::write(&score_path, SCORE).unwrap();
    let bundle_dir = tmp.path().join("ui");
    std::fs::create_dir_all(&bundle_dir).unwrap();
    std::fs::write(bundle_dir.join("index.html"), b"<h1>web-load-ok</h1>").unwrap();

    // Daemon + hive + factory.
    let hive = Arc::new(HiveState::new(0xCAFE_BABE, 64, 16));
    hive.ensembles
        .register_factory(Arc::new(NoopFactory(Arc::new(AtomicU32::new(0)))));

    let socket_path = tmp.path().join("r2-hive.sock");
    let daemon = DaemonState::new();
    daemon.attach_hive_state(hive.clone());
    let _handle = socket::spawn(socket_path.clone(), daemon)
        .await
        .expect("spawn daemon");

    // Spawn the HTTP listener that fronts web plugins.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http_addr = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/ensemble/{*rest}", get(r2_hive::web::serve_web_plugin))
        .route("/plugin/{*rest}", get(r2_hive::web::serve_web_plugin))
        .with_state(hive.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Send `r2.mgmt.ensemble.load` with `path` set.
    let req = build_load_request_from_path(
        1,
        "yaml",
        score_path.to_str().expect("utf8"),
    );
    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut r, mut w) = stream.split();
    write_frame(&mut w, &req).await.expect("write load");
    let resp = read_frame(&mut r)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.ensemble.load").unwrap(),
        "expected load response, not error"
    );

    // Web plugin should now be mounted.
    let mounts = hive.web_plugins.mounts();
    assert!(
        mounts.iter().any(|m| m == "/ensemble/web-load-test"),
        "expected /ensemble/web-load-test mount, got {mounts:?}"
    );

    // GET the index over HTTP.
    let client = reqwest::Client::new();
    let r1 = client
        .get(format!("http://{}/ensemble/web-load-test", http_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    let body = r1.text().await.unwrap();
    assert!(body.contains("web-load-ok"), "unexpected body: {body}");

    // Stop the ensemble; the mount should disappear.
    let stop = build_stop_request(2, "web-load-test");
    write_frame(&mut w, &stop).await.expect("write stop");
    let _ = read_frame(&mut r).await.expect("read");
    assert!(hive.web_plugins.mounts().is_empty(), "expected unmount on stop");

    let r2 = client
        .get(format!("http://{}/ensemble/web-load-test", http_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 404);
}
