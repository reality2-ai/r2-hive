//! Legacy relay compatibility layer.
//!
//! Provides backward-compatible WebSocket handshake (HELLO/WELCOME),
//! frame catchup buffer, and signaling protocol for existing clients
//! (e.g. Notekeeper in the browser).

pub mod buffer;
pub mod handshake;
pub mod protocol;
