//! # r2-hive-core
//!
//! The **platform-agnostic** half of the hive (R2-HIVE north-star: ONE hive
//! codebase everywhere = these no_std crates + a thin per-platform host/board
//! layer). This crate holds hive logic that has **no platform/runtime
//! dependency** — no tokio, no axum, no std networking — so the same code runs on
//! Linux/cloud, ESP32-S3/DFR1195 (esp-hal/embassy), Uno-Q, and the wasm browser
//! hive. It is built on r2-core's no_std/alloc protocol crates (r2-wire, r2-route,
//! r2-fnv) and is itself `#![no_std]` + `alloc`.
//!
//! Platform layers (e.g. `r2-hive-bin` on Linux) depend on this crate and supply
//! the platform-specific pieces (async runtime, sockets, board drivers, storage).
//!
//! First module: [`sync_host`] — the sync host-loop transport seam + the routing
//! core (`route_inbound_sync`) the MCU firmware runs (R2-DISCOVERY §5 sync tier).
//! Verified `#![no_std]` here; further seams (Platform, transport) migrate in as
//! the convergence proceeds.

#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod ensemble;
pub mod identity;
pub mod ota;
pub mod platform;
pub mod sync_host;
pub mod transport_seam;
