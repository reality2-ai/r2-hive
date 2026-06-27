//! Web plugin registry — implements R2-PLUGIN §13 (web plugin type).
//!
//! Per-request lookup against an `RwLock<HashMap<mount, MountedBundle>>`,
//! so mount/unmount are atomic from a concurrent request's point of view
//! (§13.4). axum routes under `/ensemble/*` and `/plugin/*` fall through
//! to [`serve_web_plugin`], which resolves the mount, opens the file
//! relative to its bundle root, and applies the §13.9 default headers.
//!
//! Authentication (§13.5) is enforced by this module. Browser device
//! cookies are verified against the active device ledger; missing auth
//! fails closed unless the operator explicitly starts the daemon with the
//! development bypass.
//!
//! WebSocket channels (§13.6) live in a separate module (`web_ws.rs`,
//! TODO) once auth is in place. For now `mount()` records channel
//! metadata so a later wiring step can pick it up without a registry
//! migration.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use r2_def::{CspPolicy, WebChannelDef, WebPluginManifest};

use crate::hive::HiveState;

/// One mounted web plugin — a bundle directory served at a URL prefix.
#[derive(Debug)]
pub struct MountedBundle {
    /// Owning ensemble id (for unmount).
    pub ensemble: String,
    /// Plugin name within the ensemble.
    pub plugin: String,
    /// Mount path (e.g. `/ensemble/notekeeper`). Always begins with `/`,
    /// never has a trailing `/`.
    pub mount: String,
    /// Filesystem directory containing `index.html`. Always absolute.
    pub bundle_root: PathBuf,
    /// The plugin's Content-Security-Policy (R2-WEB v0.6 §3.4): the authored
    /// policy, or `CspPolicy::restrictive_default` when the manifest omitted it.
    pub csp: CspPolicy,
    /// WebSocket channel definitions. Currently recorded only — wiring
    /// happens in Phase 3d once browser auth is in place.
    pub channels: Vec<WebChannelDef>,
}

/// Registry of mounted web plugins. Lookups are read-locked; mounts and
/// unmounts take the write lock for an instant.
#[derive(Debug, Default)]
pub struct WebPluginRegistry {
    /// Indexed by mount path (no trailing slash). The same plugin can
    /// only mount once at the same path; `mount` rejects duplicates.
    by_mount: RwLock<HashMap<String, Arc<MountedBundle>>>,
    /// Reverse index: ensemble id → mount paths owned by that ensemble.
    by_ensemble: RwLock<HashMap<String, Vec<String>>>,
}

/// Why a [`WebPluginRegistry::mount`] call failed.
#[derive(Debug, thiserror::Error)]
pub enum MountError {
    /// `bundle_root` does not exist or is not a readable directory.
    #[error("bundle root {0} is not a readable directory")]
    BundleRootMissing(PathBuf),
    /// `index.html` is missing from `bundle_root` (R2-PLUGIN §13.3).
    #[error("bundle {0} has no index.html (R2-PLUGIN §13.3)")]
    NoIndexHtml(PathBuf),
    /// A symlink inside the bundle resolves outside the bundle root
    /// (R2-PLUGIN §13.3).
    #[error("bundle {0} contains an escaping symlink (R2-PLUGIN §13.3)")]
    EscapingSymlink(PathBuf),
    /// Another plugin is already mounted at this path.
    #[error("mount path {0} is already taken")]
    AlreadyMounted(String),
}

impl WebPluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mount a web plugin's bundle at the path derived from `manifest`
    /// (or the ensemble default if `manifest.mount` is `None`).
    ///
    /// `score_dir` is the directory the score was loaded from; the
    /// manifest's `bundle` is resolved relative to it.
    pub fn mount(
        &self,
        ensemble: &str,
        manifest: &WebPluginManifest,
        score_dir: &Path,
    ) -> Result<String, MountError> {
        let mount_path = manifest
            .mount
            .clone()
            .unwrap_or_else(|| format!("/ensemble/{}", ensemble));
        let mount_path = mount_path.trim_end_matches('/').to_string();
        let bundle_root = score_dir.join(&manifest.bundle);
        let bundle_root = bundle_root.canonicalize().map_err(|_| {
            MountError::BundleRootMissing(bundle_root.clone())
        })?;
        if !bundle_root.is_dir() {
            return Err(MountError::BundleRootMissing(bundle_root));
        }
        if !bundle_root.join("index.html").is_file() {
            return Err(MountError::NoIndexHtml(bundle_root));
        }
        verify_no_escaping_symlinks(&bundle_root)?;

        {
            let by_mount = self.by_mount.read().expect("registry lock");
            if by_mount.contains_key(&mount_path) {
                return Err(MountError::AlreadyMounted(mount_path));
            }
        }

        let bundle = Arc::new(MountedBundle {
            ensemble: ensemble.to_string(),
            plugin: manifest.name.clone(),
            mount: mount_path.clone(),
            bundle_root,
            // Manifest CSP is always Some (the parser fills restrictive_default
            // when omitted); default defensively so a UI is never served without one.
            csp: manifest.csp.clone().unwrap_or_else(CspPolicy::restrictive_default),
            channels: manifest.channels.clone(),
        });

        {
            let mut by_mount = self.by_mount.write().expect("registry lock");
            by_mount.insert(mount_path.clone(), bundle);
        }
        {
            let mut by_ens = self.by_ensemble.write().expect("registry lock");
            by_ens
                .entry(ensemble.to_string())
                .or_default()
                .push(mount_path.clone());
        }

        Ok(mount_path)
    }

    /// Remove every mount belonging to `ensemble`.
    pub fn unmount_ensemble(&self, ensemble: &str) {
        let mounts = self
            .by_ensemble
            .write()
            .expect("registry lock")
            .remove(ensemble)
            .unwrap_or_default();
        let mut by_mount = self.by_mount.write().expect("registry lock");
        for m in mounts {
            by_mount.remove(&m);
        }
    }

    /// Resolve a request URI path to a mounted bundle and the relative
    /// asset path within the bundle. Returns `None` if no mount covers
    /// the URI.
    pub fn resolve<'a>(&self, uri_path: &'a str) -> Option<(Arc<MountedBundle>, &'a str)> {
        let by_mount = self.by_mount.read().expect("registry lock");
        for (mount, bundle) in by_mount.iter() {
            if let Some(rest) = match_mount(mount, uri_path) {
                return Some((Arc::clone(bundle), rest));
            }
        }
        None
    }

    /// All currently-mounted paths (mostly for tests / status JSON).
    pub fn mounts(&self) -> Vec<String> {
        self.by_mount
            .read()
            .expect("registry lock")
            .keys()
            .cloned()
            .collect()
    }
}

/// Returns `Some(rest)` if `uri_path` is `mount` or `mount/...`. The
/// returned `rest` does NOT begin with a slash; for the bare mount it
/// is the empty string.
fn match_mount<'a>(mount: &str, uri_path: &'a str) -> Option<&'a str> {
    if let Some(stripped) = uri_path.strip_prefix(mount) {
        if stripped.is_empty() {
            Some("")
        } else if let Some(rest) = stripped.strip_prefix('/') {
            Some(rest)
        } else {
            // mount is "/ensemble/foo" and uri is "/ensemble/foobar" — not a match.
            None
        }
    } else {
        None
    }
}

fn verify_no_escaping_symlinks(root: &Path) -> Result<(), MountError> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| MountError::BundleRootMissing(root.to_path_buf()))?;
    let mut stack = vec![canonical_root.clone()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|_| MountError::BundleRootMissing(dir.clone()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let meta = entry.metadata().ok();
            if let Ok(real) = path.canonicalize() {
                if !real.starts_with(&canonical_root) {
                    return Err(MountError::EscapingSymlink(path));
                }
                if meta.map(|m| m.is_dir()).unwrap_or(false) {
                    stack.push(real);
                }
            } else {
                // Broken symlink — treat as escaping.
                return Err(MountError::EscapingSymlink(path));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// axum handler
// ---------------------------------------------------------------------


/// Fallback handler for `/ensemble/*` and `/plugin/*` URIs. Serves
/// static assets from the matching mounted bundle, applying §13.9
/// security headers and §13.5 auth gating.
pub async fn serve_web_plugin(
    State(hive): State<Arc<HiveState>>,
    AxumPath(_path): AxumPath<String>,
    headers: HeaderMap,
    req: axum::extract::Request,
) -> Response {
    let uri_path = req.uri().path();
    let Some((bundle, rest)) = hive.web_plugins.resolve(uri_path) else {
        return (StatusCode::NOT_FOUND, "no such mount").into_response();
    };

    // R2-PLUGIN §13.5 — gate static GETs on the session cookie. Missing
    // auth fails closed unless the operator explicitly enabled web-dev-mode.
    let dev_mode = if let Some(auth) = hive.web_auth() {
        let cookie_header = headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        match auth.verify_cookie_header(cookie_header) {
            Ok(_device_id) => false,
            Err(_) => return unauthenticated_response(&bundle.ensemble, &headers, uri_path),
        }
    } else if hive.web_dev_mode() {
        true
    } else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "web auth not configured",
        )
            .into_response();
    };

    // Resolve the asset path. Empty rest -> index.html.
    let rest = if rest.is_empty() { "index.html" } else { rest };
    let mut path = PathBuf::from(rest);
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return (StatusCode::FORBIDDEN, "path escapes bundle").into_response();
    }
    if path.as_os_str().is_empty() {
        path = PathBuf::from("index.html");
    }

    let full = bundle.bundle_root.join(&path);
    let canonical = match full.canonicalize() {
        Ok(p) => p,
        Err(_) => return (StatusCode::NOT_FOUND, "no such asset").into_response(),
    };
    if !canonical.starts_with(&bundle.bundle_root) {
        return (StatusCode::FORBIDDEN, "path escapes bundle").into_response();
    }
    if canonical.is_dir() {
        // Directory listing not allowed; fall back to index.html if
        // the URI points exactly at a directory.
        let with_index = canonical.join("index.html");
        if with_index.is_file() {
            return serve_file(&bundle, with_index, dev_mode).await;
        }
        return (StatusCode::NOT_FOUND, "no such asset").into_response();
    }
    serve_file(&bundle, canonical, dev_mode).await
}

async fn serve_file(bundle: &MountedBundle, path: PathBuf, dev_mode: bool) -> Response {
    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::NOT_FOUND, "no such asset").into_response(),
    };
    let mime = mime_for(&path);
    let csp = render_csp(&bundle.csp);
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    h.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    h.insert("Referrer-Policy", HeaderValue::from_static("same-origin"));
    h.insert(
        "Cross-Origin-Opener-Policy",
        HeaderValue::from_static("same-origin"),
    );
    h.insert(
        "Cross-Origin-Embedder-Policy",
        HeaderValue::from_static("require-corp"),
    );
    h.insert(
        "Permissions-Policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    if let Ok(v) = HeaderValue::from_str(&csp) {
        h.insert("Content-Security-Policy", v);
    }
    if dev_mode {
        h.insert("X-R2-Web-Auth", HeaderValue::from_static("dev-mode"));
    }
    resp
}

fn unauthenticated_response(
    ensemble: &str,
    headers: &HeaderMap,
    uri_path: &str,
) -> Response {
    // Browser-friendly: redirect to provision when the request looks
    // like a navigation (Accept: text/html). Other clients get 401.
    let accepts_html = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/html"))
        .unwrap_or(false);
    if accepts_html {
        let target = format!(
            "/r2/web/provision?return={}",
            urlencoded(uri_path)
        );
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::SEE_OTHER;
        if let Ok(v) = HeaderValue::from_str(&target) {
            resp.headers_mut().insert(header::LOCATION, v);
        }
        return resp;
    }
    let mut resp = Response::new(Body::from("authentication required"));
    *resp.status_mut() = StatusCode::UNAUTHORIZED;
    let realm = format!("R2-Provision realm=\"{}\"", ensemble);
    if let Ok(v) = HeaderValue::from_str(&realm) {
        resp.headers_mut().insert(header::WWW_AUTHENTICATE, v);
    }
    resp
}

fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

/// Render a [`CspPolicy`] (R2-WEB v0.6 §3.4) directive map into a
/// Content-Security-Policy header value: `directive src1 src2; directive2 ...`.
/// `directives` is a `BTreeMap`, so the output is deterministically ordered. A
/// directive with no sources renders as the bare directive (valid CSP, e.g.
/// `upgrade-insecure-requests`).
fn render_csp(policy: &CspPolicy) -> String {
    let mut out = String::new();
    for (directive, sources) in &policy.directives {
        if !out.is_empty() {
            out.push_str("; ");
        }
        out.push_str(directive);
        for s in sources {
            out.push(' ');
            out.push_str(s);
        }
    }
    out
}

// ---------------------------------------------------------------------
// /r2/web/provision endpoint
// ---------------------------------------------------------------------

/// Browser provision endpoint (R2-PLUGIN §13.5).
///
/// `GET /r2/web/provision?return=...` renders a minimal HTML form so a
/// person can paste the operator-issued word code. The form submits
/// `POST /r2/web/provision` with `code=<words>&return=<path>`. On
/// success the hive sets the session cookie and redirects to
/// `<return>` (or `/` if absent).
///
/// `POST` accepts either `application/x-www-form-urlencoded` (HTML
/// form) or `application/json` (`{ "code": "<words>" }`) for headless
/// clients.
pub async fn web_provision_get(
    State(_hive): State<Arc<HiveState>>,
    req: axum::extract::Request,
) -> Response {
    let return_to = parse_query_param(req.uri().query().unwrap_or(""), "return")
        .unwrap_or_else(|| "/".to_string());
    let html = format!(
        r##"<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>R2 — pair this device</title>
<style>
  body {{ font-family: system-ui, sans-serif; max-width: 32rem; margin: 4rem auto; padding: 0 1rem; }}
  label {{ display: block; margin-top: 1rem; }}
  input[type=text] {{ width: 100%; padding: .5rem; font-size: 1rem; box-sizing: border-box; }}
  button {{ margin-top: 1rem; padding: .5rem 1rem; font-size: 1rem; }}
  .hint {{ color: #666; font-size: .9em; margin-top: .25rem; }}
</style>
</head><body>
<h1>Pair this browser</h1>
<p>Enter the three-word code from the operator. Run <code>r2hive web provision</code> on the daemon host to mint one.</p>
<form method="post" action="/r2/web/provision">
  <input type="hidden" name="return" value="{return_to_attr}">
  <label>Word code
    <input type="text" name="code" autocomplete="off" placeholder="e.g. amber-orbit-cedar" required>
  </label>
  <p class="hint">The code is single-use and expires after 1 hour.</p>
  <button type="submit">Pair</button>
</form>
</body></html>
"##,
        return_to_attr = html_escape(&return_to)
    );
    let mut resp = Response::new(Body::from(html));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp.headers_mut().insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    resp
}

/// POST `/r2/web/provision` — redeem a word code, set the session
/// cookie, redirect (form) or return JSON (API).
pub async fn web_provision_post(
    State(hive): State<Arc<HiveState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let Some(auth) = hive.web_auth() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "web auth not configured",
        )
            .into_response();
    };

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let (code, return_to) = if content_type.starts_with("application/json") {
        match serde_json::from_slice::<ProvisionRequest>(&body) {
            Ok(p) => (p.code, p.return_to.unwrap_or_else(|| "/".into())),
            Err(_) => return (StatusCode::BAD_REQUEST, "bad json").into_response(),
        }
    } else {
        // form-urlencoded
        let s = match std::str::from_utf8(&body) {
            Ok(s) => s,
            Err(_) => return (StatusCode::BAD_REQUEST, "bad form").into_response(),
        };
        let code = parse_query_param(s, "code");
        let ret = parse_query_param(s, "return").unwrap_or_else(|| "/".to_string());
        match code {
            Some(c) => (c, ret),
            None => return (StatusCode::BAD_REQUEST, "missing code").into_response(),
        }
    };

    match auth.redeem_provision_code(&code) {
        Ok((_cred, set_cookie)) => {
            let mut resp = Response::new(Body::empty());
            // JSON callers want JSON; form callers want a redirect.
            if content_type.starts_with("application/json") {
                let json = serde_json::json!({"status": "ok", "return": return_to})
                    .to_string();
                resp = Response::new(Body::from(json));
                resp.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
            } else {
                *resp.status_mut() = StatusCode::SEE_OTHER;
                if let Ok(v) = HeaderValue::from_str(&return_to) {
                    resp.headers_mut().insert(header::LOCATION, v);
                }
            }
            if let Ok(v) = HeaderValue::from_str(&set_cookie) {
                resp.headers_mut().insert(header::SET_COOKIE, v);
            }
            resp
        }
        Err(e) => {
            let status = StatusCode::BAD_REQUEST;
            let body = format!("provision failed: {e}");
            (status, body).into_response()
        }
    }
}

#[derive(serde::Deserialize)]
struct ProvisionRequest {
    code: String,
    #[serde(rename = "return")]
    return_to: Option<String>,
}

fn parse_query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next()?;
        let v = it.next().unwrap_or("");
        if k == key {
            return Some(url_decode(v));
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let h = std::str::from_utf8(&bytes[i + 1..i + 3])
                    .ok()
                    .and_then(|h| u8::from_str_radix(h, 16).ok());
                match h {
                    Some(b) => {
                        out.push(b);
                        i += 3;
                    }
                    None => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("txt") => "text/plain; charset=utf-8",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(name: &str, mount: Option<&str>, bundle: &str) -> WebPluginManifest {
        WebPluginManifest {
            name: name.to_string(),
            bundle: bundle.to_string(),
            mount: mount.map(|s| s.to_string()),
            channels: Vec::new(),
            subscriptions: Vec::new(),
            graphql_schema: None,
            csp: None,
        }
    }

    #[test]
    fn match_mount_exact_and_subpath() {
        assert_eq!(match_mount("/ensemble/foo", "/ensemble/foo"), Some(""));
        assert_eq!(
            match_mount("/ensemble/foo", "/ensemble/foo/app.js"),
            Some("app.js")
        );
        assert_eq!(match_mount("/ensemble/foo", "/ensemble/foobar"), None);
        assert_eq!(match_mount("/ensemble/foo", "/other"), None);
    }

    #[test]
    fn mount_then_resolve() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("ui");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("index.html"), b"<h1>hi</h1>").unwrap();
        let reg = WebPluginRegistry::new();
        let m = manifest("ui", None, "ui/");
        reg.mount("notekeeper", &m, tmp.path()).unwrap();

        let (got, rest) = reg.resolve("/ensemble/notekeeper").unwrap();
        assert_eq!(got.ensemble, "notekeeper");
        assert_eq!(rest, "");

        reg.unmount_ensemble("notekeeper");
        assert!(reg.resolve("/ensemble/notekeeper").is_none());
    }

    #[test]
    fn mount_rejects_missing_index_html() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("ui");
        std::fs::create_dir_all(&bundle).unwrap();
        let reg = WebPluginRegistry::new();
        let m = manifest("ui", None, "ui/");
        let err = reg.mount("e", &m, tmp.path()).unwrap_err();
        assert!(matches!(err, MountError::NoIndexHtml(_)));
    }

    #[test]
    fn render_csp_emits_directive_map_in_header_form() {
        let mut directives = std::collections::BTreeMap::new();
        directives.insert(
            "script-src".to_string(),
            vec!["'self'".to_string(), "https://cdn.example.com".to_string()],
        );
        directives.insert("object-src".to_string(), vec!["'none'".to_string()]);
        let out = render_csp(&CspPolicy { directives });
        assert!(out.contains("script-src 'self' https://cdn.example.com"));
        assert!(out.contains("object-src 'none'"));
        // BTreeMap order ⇒ object-src (o) before script-src (s), joined by "; ".
        assert_eq!(out, "object-src 'none'; script-src 'self' https://cdn.example.com");
    }

    #[test]
    fn restrictive_default_renders_a_safe_policy() {
        let out = render_csp(&CspPolicy::restrictive_default());
        assert!(out.contains("default-src 'self'"));
        assert!(out.contains("object-src 'none'"));
    }
}
