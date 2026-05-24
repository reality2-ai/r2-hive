//! Wayfinder plugins — modular capabilities that extend the mesh participant.
//!
//! Each plugin provides axum routes that get merged into the wayfinder's
//! HTTP router. Plugins are composable and feature-gated.
//!
//! ## Plugin Pattern
//!
//! A plugin is a function that returns an axum `Router` fragment:
//!
//! ```ignore
//! pub fn routes() -> Router<Arc<WayfinderState>> { ... }
//! ```
//!
//! The wayfinder main.rs merges all plugin routes into the app router.
//! Plugins access shared state through axum's `State` extractor.

pub mod word_codes;
pub mod dashboard;
