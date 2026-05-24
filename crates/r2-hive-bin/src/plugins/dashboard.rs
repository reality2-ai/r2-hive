//! Dashboard plugin — HTML status page and static assets.
//!
//! Routes:
//!   GET /           — HTML dashboard
//!   GET /relay.svg  — Logo (legacy name for backward compat)

use std::sync::Arc;

use axum::response::{Html, IntoResponse, Response};
use axum::http::header;
use axum::routing::get;
use axum::Router;

use crate::hive::HiveState;

/// Plugin routes for the dashboard.
pub fn routes() -> Router<Arc<HiveState>> {
    Router::new()
        .route("/", get(dashboard))
        .route("/relay.svg", get(relay_svg))
}

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("../../static/dashboard.html"))
}

async fn relay_svg() -> Response {
    let svg = include_str!("../../static/relay.svg");
    ([(header::CONTENT_TYPE, "image/svg+xml")], svg).into_response()
}
