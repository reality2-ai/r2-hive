//! End-to-end test for R2-PLUGIN §13.5 browser provision + cookie gate.
//!
//! 1. Mounts a tiny bundle.
//! 2. Installs a `WebAuth` registry.
//! 3. Asserts that a cookie-less GET against the bundle is 401 (Accept: */*)
//!    or a 303 redirect to /r2/web/provision (Accept: text/html).
//! 4. Mints a provision word code, redeems it via POST, captures the
//!    Set-Cookie value, and replays the cookie on a fresh GET that
//!    succeeds.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use r2_def::{WebChannelDef, WebPluginManifest};
use r2_hive::hive::HiveState;
use r2_hive::web::{serve_web_plugin, web_provision_get, web_provision_post};
use r2_hive::web_auth::WebAuth;

fn make_app(state: Arc<HiveState>) -> Router {
    Router::new()
        .route("/ensemble/{*rest}", get(serve_web_plugin))
        .route("/plugin/{*rest}", get(serve_web_plugin))
        .route(
            "/r2/web/provision",
            get(web_provision_get).post(web_provision_post),
        )
        .with_state(state)
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

fn manifest(name: &str, mount: Option<&str>, bundle: &str) -> WebPluginManifest {
    WebPluginManifest {
        name: name.to_string(),
        bundle: bundle.to_string(),
        mount: mount.map(|s| s.to_string()),
        channels: Vec::<WebChannelDef>::new(),
        subscriptions: Vec::new(),
        graphql_schema: None,
        csp: None, // parser fills restrictive_default; mount path defaults defensively
    }
}

fn make_bundle(tmp: &std::path::Path, body: &[u8]) {
    let bundle = tmp.join("ui");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("index.html"), body).unwrap();
}

async fn setup_with_auth() -> (Arc<HiveState>, SocketAddr, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    make_bundle(tmp.path(), b"<h1>guarded</h1>");
    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    state
        .web_plugins
        .mount("guarded", &m, tmp.path())
        .expect("mount");
    state.set_web_auth(Arc::new(WebAuth::new([0x42u8; 32])));
    let addr = spawn_app(state.clone()).await;
    (state, addr, tmp)
}

#[tokio::test]
async fn cookieless_get_with_html_accept_redirects_to_provision() {
    let (_state, addr, _tmp) = setup_with_auth().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let resp = client
        .get(format!("http://{}/ensemble/guarded", addr))
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 303);
    let loc = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.starts_with("/r2/web/provision?return="));
}

#[tokio::test]
async fn cookieless_get_with_api_accept_returns_401() {
    let (_state, addr, _tmp) = setup_with_auth().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/ensemble/guarded", addr))
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let www = resp
        .headers()
        .get("www-authenticate")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(www.starts_with("R2-Provision realm=\"guarded\""));
}

#[tokio::test]
async fn provision_redeem_then_authenticated_get_succeeds() {
    let (state, addr, _tmp) = setup_with_auth().await;
    // Mint a code via the WebAuth API directly (the mgmt event path is
    // covered by a separate test).
    let auth = state.web_auth().expect("auth");
    let code = auth.mint_provision_code_with_ttl(60);

    // Redeem it via the form-encoded POST endpoint.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let resp = client
        .post(format!("http://{}/r2/web/provision", addr))
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(format!("code={}&return=/ensemble/guarded", code))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 303, "expected redirect after provision");
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .expect("cookie set")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.contains("r2_web_session="));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));

    // Replay the cookie. reqwest needs the bare `name=value` for Cookie.
    let body_pair = set_cookie
        .split(';')
        .next()
        .unwrap()
        .trim()
        .to_string();
    let resp2 = client
        .get(format!("http://{}/ensemble/guarded", addr))
        .header(reqwest::header::COOKIE, body_pair)
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), 200);
    let body = resp2.text().await.unwrap();
    assert!(body.contains("guarded"));
}

#[tokio::test]
async fn provision_with_bad_code_returns_400() {
    let (_state, addr, _tmp) = setup_with_auth().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/r2/web/provision", addr))
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body("code=not-a-real-code")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn missing_web_auth_fails_closed_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    make_bundle(tmp.path(), b"<h1>closed</h1>");
    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    let m = manifest("ui", None, "ui/");
    state.web_plugins.mount("closed", &m, tmp.path()).unwrap();
    let addr = spawn_app(state.clone()).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/ensemble/closed", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);
}

/// DEV-BUILD-ONLY (R2-BUILDMODE §5.1): tests the dev bypass itself; the
/// setter does not exist in prod builds. The other tests in this file are
/// the PROD-relevant fail-closed assertions and run in both modes.
#[cfg(feature = "dev")]
#[tokio::test]
async fn explicit_dev_mode_serves_with_warning_header() {
    let tmp = tempfile::tempdir().unwrap();
    make_bundle(tmp.path(), b"<h1>open</h1>");
    let state = Arc::new(HiveState::new(0xCAFEBEEF, 64, 4));
    state.set_web_dev_mode(true);
    let m = manifest("ui", None, "ui/");
    state.web_plugins.mount("open", &m, tmp.path()).unwrap();
    let addr = spawn_app(state.clone()).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/ensemble/open", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let dev = resp
        .headers()
        .get("x-r2-web-auth")
        .expect("dev-mode marker")
        .to_str()
        .unwrap();
    assert_eq!(dev, "dev-mode");
}

#[tokio::test]
async fn revoked_cookie_is_rejected() {
    let (state, addr, _tmp) = setup_with_auth().await;
    let auth = state.web_auth().expect("auth");
    let code = auth.mint_provision_code_with_ttl(60);
    let (cred, set_cookie) = auth.redeem_provision_code(&code).unwrap();
    let cookie_pair = set_cookie.split(';').next().unwrap().trim().to_string();

    auth.revoke_device(&cred.device_id);

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/ensemble/guarded", addr))
        .header(reqwest::header::COOKIE, cookie_pair)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}
