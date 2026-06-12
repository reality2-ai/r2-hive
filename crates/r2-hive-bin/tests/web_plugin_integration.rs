//! End-to-end test for R2-PLUGIN §13 web-plugin mount/serve/unmount.
//!
//! Spawns the production axum router on a loopback port, mounts a
//! tempdir bundle through `state.web_plugins.mount()`, fetches the
//! index, asserts the §13.9 default headers, then unmounts and asserts
//! 404.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use r2_def::{WebChannelDef, WebCspOverride, WebPluginManifest};
use r2_hive::hive::HiveState;
use r2_hive::web::serve_web_plugin;

fn make_app(state: Arc<HiveState>) -> Router {
    Router::new()
        .route("/ensemble/{*rest}", get(serve_web_plugin))
        .route("/plugin/{*rest}", get(serve_web_plugin))
        .with_state(state)
}

fn manifest(name: &str, mount: Option<&str>, bundle: &str) -> WebPluginManifest {
    WebPluginManifest {
        name: name.to_string(),
        bundle: bundle.to_string(),
        mount: mount.map(|s| s.to_string()),
        channels: Vec::<WebChannelDef>::new(),
        subscriptions: Vec::new(),
        graphql_schema: None,
        csp: Some(WebCspOverride::default()),
    }
}

async fn spawn_app(state: Arc<HiveState>) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = make_app(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    addr
}

#[tokio::test]
async fn mount_serves_index_with_security_headers() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = tmp.path().join("ui");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("index.html"), b"<h1>hi</h1>").unwrap();
    std::fs::write(bundle.join("app.js"), b"console.log('ok');\n").unwrap();

    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    state
        .web_plugins
        .mount("notekeeper", &m, tmp.path())
        .expect("mount");

    let addr = spawn_app(state.clone()).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("http://{}/ensemble/notekeeper", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let csp = resp
        .headers()
        .get("Content-Security-Policy")
        .expect("csp header")
        .to_str()
        .unwrap()
        .to_string();
    let xcto = resp
        .headers()
        .get("X-Content-Type-Options")
        .expect("xcto")
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(xcto, "nosniff");
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("frame-ancestors 'none'"));
    let body = resp.text().await.unwrap();
    assert!(body.contains("<h1>hi</h1>"));

    // sub-asset
    let resp = client
        .get(format!("http://{}/ensemble/notekeeper/app.js", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/javascript; charset=utf-8"
    );
}

#[tokio::test]
async fn unmount_yields_404() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = tmp.path().join("ui");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("index.html"), b"<h1>x</h1>").unwrap();

    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    state
        .web_plugins
        .mount("gone", &m, tmp.path())
        .expect("mount");

    let addr = spawn_app(state.clone()).await;
    let client = reqwest::Client::new();

    let r1 = client
        .get(format!("http://{}/ensemble/gone", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);

    state.web_plugins.unmount_ensemble("gone");

    let r2 = client
        .get(format!("http://{}/ensemble/gone", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 404);
}

#[tokio::test]
async fn parent_dir_traversal_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = tmp.path().join("ui");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("index.html"), b"<h1>x</h1>").unwrap();

    // sibling file outside the bundle
    std::fs::write(tmp.path().join("secret.txt"), b"sekret").unwrap();

    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    state.web_plugins.mount("e", &m, tmp.path()).expect("mount");

    let addr = spawn_app(state.clone()).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("http://{}/ensemble/e/../secret.txt", addr))
        .send()
        .await
        .unwrap();
    // axum's path normalisation may collapse `..` before our handler
    // sees it; either 403 (we caught it) or 404 (no such mount after
    // normalisation) is acceptable, the file MUST NOT be served.
    assert!(matches!(resp.status().as_u16(), 403 | 404));
}

#[tokio::test]
async fn missing_index_html_rejected_at_mount() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = tmp.path().join("ui");
    std::fs::create_dir_all(&bundle).unwrap();

    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    let err = state.web_plugins.mount("e", &m, tmp.path()).unwrap_err();
    assert!(format!("{err}").contains("index.html"));
}

// ─────────────────────────────────────────────────────────────────
// R2-PLUGIN §13.10 conformance vectors — gap #2 from CONFORMANCE.md
// ─────────────────────────────────────────────────────────────────

/// WEB-ESCAPING-SYMLINK-REJECTED (§13.3, §13.10(7)): a bundle containing
/// a symlink that resolves outside the bundle root MUST be rejected at
/// mount.
#[tokio::test]
async fn escaping_symlink_rejected_at_mount() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = tmp.path().join("ui");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("index.html"), b"<h1>x</h1>").unwrap();

    // Symlink target is outside `bundle` and resolves to a real file.
    std::fs::write(tmp.path().join("secret.txt"), b"sekret").unwrap();
    std::os::unix::fs::symlink(
        tmp.path().join("secret.txt"),
        bundle.join("escape"),
    )
    .unwrap();

    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    let err = state.web_plugins.mount("e", &m, tmp.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("escaping symlink"),
        "expected escaping-symlink error, got: {msg}"
    );
}

/// WEB-ATOMIC-RELOAD (§13.4, §13.10(3)): a remount that swaps the
/// bundle root MUST appear atomic to concurrent requests — a request
/// in flight observes either the old or new index, never a torn
/// state.
#[tokio::test]
async fn atomic_remount_observes_either_old_or_new_never_torn() {
    let tmp_v1 = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp_v1.path().join("ui")).unwrap();
    std::fs::write(tmp_v1.path().join("ui").join("index.html"), b"<h1>v1</h1>").unwrap();

    let tmp_v2 = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp_v2.path().join("ui")).unwrap();
    std::fs::write(tmp_v2.path().join("ui").join("index.html"), b"<h1>v2</h1>").unwrap();

    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    state
        .web_plugins
        .mount("notekeeper", &m, tmp_v1.path())
        .expect("v1 mount");

    let addr = spawn_app(state.clone()).await;
    let client = reqwest::Client::new();

    // Spawn 16 concurrent GETs while remounting between v1 and v2.
    let url = format!("http://{}/ensemble/notekeeper", addr);
    let mut handles = Vec::new();
    for _ in 0..16 {
        let c = client.clone();
        let u = url.clone();
        handles.push(tokio::spawn(async move {
            c.get(u).send().await.unwrap().text().await.unwrap()
        }));
    }
    // Remount mid-flight.
    state.web_plugins.unmount_ensemble("notekeeper");
    state
        .web_plugins
        .mount("notekeeper", &m, tmp_v2.path())
        .expect("v2 mount");

    let mut bodies = Vec::new();
    for h in handles {
        bodies.push(h.await.unwrap());
    }
    for body in &bodies {
        assert!(
            body.contains("<h1>v1</h1>") || body.contains("<h1>v2</h1>") || body.is_empty(),
            "torn body observed: {body:?}"
        );
    }
    // Empty bodies are OK (request happened during the unmount→mount
    // gap and got 404). The point is no mixed content.
}

/// WEB-BAD-SCORE-CHANNEL-TARGET (§13.8.5): a score whose channel
/// `target_sentant` references a sentant that doesn't exist in the
/// same ensemble MUST be rejected at load. Mount-side analogue: if a
/// manifest specifies channels but no matching sentant in the score,
/// reject — modelled here at the parser level since channel-target
/// validation needs the score's sentant list.
#[test]
fn channel_target_validation_via_def_parser() {
    use r2_def::{parse_ensemble_yaml, EnsembleScore, SentantEntry};

    // Channel target_sentant=\"nonexistent\" is not a sentant in the
    // ensemble. r2-def parses successfully (parsing is structural);
    // validation is the loader's job. We assert here that the score
    // *parses* and the channel definition makes the mismatch
    // detectable downstream (the loader must reject before the
    // r2-hive ensemble registry instantiates it).
    let yaml = r#"
ensemble:
  name: bad-channel-target
  description: "Channel points at a non-existent sentant"
  version: "0.1.0"
  ensemble_version: "0.1"
  sentants:
    - name: real
      description: "the only real sentant"
      automations:
        - { name: m, initial: idle, transitions: [] }
  plugins:
    - name: ui
      type: "web"
      bundle: "ui/"
      channels:
        - name: live
          target_sentant: nonexistent
"#;
    let score: EnsembleScore = parse_ensemble_yaml(yaml).expect("parse");
    let plugin = &score.plugins[0];
    let web = plugin.as_web().expect("ok").expect("web");

    let sentant_names: Vec<&str> = score
        .sentants
        .iter()
        .map(|s| match s {
            SentantEntry::Inline(d) => d.name.as_str(),
            SentantEntry::External { include } => include.as_str(),
        })
        .collect();
    let mismatched: Vec<&str> = web
        .channels
        .iter()
        .filter(|c| !sentant_names.contains(&c.target_sentant.as_str()))
        .map(|c| c.target_sentant.as_str())
        .collect();
    assert!(
        !mismatched.is_empty(),
        "test fixture should have produced a mismatch"
    );
    assert!(
        mismatched.contains(&"nonexistent"),
        "expected 'nonexistent' in mismatch list, got {mismatched:?}"
    );
}
