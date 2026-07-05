//! Word codes plugin — temporary three-word invitation code mappings.
//!
//! Key holder's browser registers {words → tg_hash + join_code} via POST.
//! Joiner's browser looks up via GET. Single-use, 5-minute TTL.
//!
//! Routes:
//!   POST /word-code       — register {words, tg_hash, join_code}
//!   GET  /word-code/{words} — lookup and consume

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::Mutex;

use crate::hive::HiveState;

const WORD_CODE_TTL_SECS: u64 = 300; // 5 minutes

struct WordCodeEntry {
    tg_hash: String,
    join_code: String,
    created_at: Instant,
}

/// Word code store — owned by HiveState, accessed via Arc.
pub struct WordCodeStore {
    codes: Mutex<HashMap<String, WordCodeEntry>>,
}

impl WordCodeStore {
    /// Empty store.
    ///
    /// **Used-by:** `HiveState::new` (one store per daemon).
    pub fn new() -> Self {
        WordCodeStore {
            codes: Mutex::new(HashMap::new()),
        }
    }

    /// Register {words → tg_hash + join_code} with the 5-minute TTL.
    ///
    /// **Used-by:** the POST route below (key-holder's browser) and
    /// `main.rs`'s UDP `WC:` sideband (LAN-proximity propagation).
    pub async fn register(&self, words: String, tg_hash: String, join_code: String) {
        let mut codes = self.codes.lock().await;
        let now = Instant::now();
        codes.retain(|_, v| now.duration_since(v.created_at).as_secs() < WORD_CODE_TTL_SECS);
        codes.insert(words, WordCodeEntry { tg_hash, join_code, created_at: now });
    }

    /// Single-use lookup: returns and CONSUMES the mapping (a second
    /// lookup misses), honouring the TTL.
    ///
    /// **Used-by:** the GET route below (joiner's browser).
    pub async fn lookup(&self, words: &str) -> Option<(String, String)> {
        let mut codes = self.codes.lock().await;
        let now = Instant::now();
        if let Some(entry) = codes.remove(words) {
            if now.duration_since(entry.created_at).as_secs() < WORD_CODE_TTL_SECS {
                return Some((entry.tg_hash, entry.join_code));
            }
        }
        None
    }
}

/// Plugin routes for word codes.
pub fn routes() -> Router<Arc<HiveState>> {
    Router::new()
        .route("/word-code", post(register_word_code))
        .route("/word-code/{words}", get(lookup_word_code))
}

async fn register_word_code(
    State(state): State<Arc<HiveState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let words = body.get("words").and_then(|v| v.as_str()).unwrap_or("");
    let tg_hash = body.get("tg_hash").and_then(|v| v.as_str()).unwrap_or("");
    let join_code = body.get("join_code").and_then(|v| v.as_str()).unwrap_or("");

    if words.is_empty() || tg_hash.is_empty() || join_code.is_empty() {
        return (StatusCode::BAD_REQUEST, "missing fields").into_response();
    }

    state.word_codes.register(words.to_string(), tg_hash.to_string(), join_code.to_string()).await;
    log::info!("word code registered: {} -> tg:{}", words, &tg_hash[..8.min(tg_hash.len())]);

    // Broadcast word code to LAN peers (proximity-limited, TTL=1)
    #[cfg(feature = "transport-udp")]
    if let Some(udp) = state.udp_transport.read().await.as_ref() {
        let msg = format!("WC:{}:{}:{}", words, tg_hash, join_code);
        use r2_discovery::AsyncTransport;
        let _ = udp.send(0, msg.as_bytes()).await; // broadcast to all UDP peers
        log::debug!("word code broadcast to LAN peers: {}", words);
    }

    ([(header::CONTENT_TYPE, "application/json")], r#"{"ok":true}"#).into_response()
}

async fn lookup_word_code(
    State(state): State<Arc<HiveState>>,
    Path(words): Path<String>,
) -> Response {
    match state.word_codes.lookup(&words).await {
        Some((tg_hash, join_code)) => {
            let json = format!(r#"{{"tg_hash":"{}","join_code":"{}"}}"#, tg_hash, join_code);
            ([(header::CONTENT_TYPE, "application/json")], json).into_response()
        }
        None => {
            (StatusCode::NOT_FOUND, "word code not found or expired").into_response()
        }
    }
}
