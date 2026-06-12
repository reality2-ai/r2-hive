//! Host-side R2-USB v2 framer (Phase USB-1).
//!
//! Consumes a CDC-ACM byte stream from a USB-attached peripheral and:
//!
//! 1. Performs the SYNC handshake (R2-USB §3.3).
//! 2. Parses the peripheral's CAPS frame (R2-USB §3.6).
//! 3. Demultiplexes type-byte-tagged frames (R2-USB §3.5):
//!    - `0x00..=0xFB` → R2-WIRE frame for that `local_id`'s transport
//!    - `0xFE` → CAPS (re-issued, e.g. on hot-plug; §3.6)
//!    - `0xFF` → control frame (§3.7) — error reports, log lines,
//!      transport state changes, and (in milestone 2) pairing
//!      messages 4–11.
//!
//! This module owns the *protocol* state machine. Serial I/O wiring
//! (`/dev/ttyACM*` open, termios raw mode, hot-plug watcher) lives in a
//! follow-up sub-module so the protocol can be tested without
//! hardware.
//!
//! # Scope: this is **R2-USB v2 only** — not the Tier 2 bus
//!
//! R2 has two wired CPU↔MCU bridge protocols and they are NOT the
//! same thing. Implementers reading this module: please don't
//! generalise it.
//!
//! - **R2-USB v2 peripheral mode** — what this module implements.
//!   Tier 3 host always on, USB-attached MCU is a thin radio
//!   appliance with no R2-WIRE stack. Length-prefix + type-byte
//!   demux + CAPS + R2-PROVISION §5.3.4 SAS pairing.
//! - **R2-HW §4 MCU-SBC bus** — *separate* protocol for Tier 2
//!   power-managed nodes (off-grid solar, bespoke CPU+MCU boards).
//!   SPI or UART + WAKE GPIO, distinct framing
//!   (`0x52 0x32 ‖ CMD ‖ LEN ‖ payload ‖ CRC-16/CCITT`),
//!   distinct vocabulary (WAKE / PACKET / STATUS / LOG / TRANSMIT
//!   / CONFIG / SLEEP / SET_TIMER), MCU runs L1–L4 autonomously,
//!   SBC duty-cycles. Implementing it would be a separate
//!   `mcu_sbc.rs` module — *not* an extension of this one.
//!
//! See `docs/CONFORMANCE.md` "Two wired-bridge architectures" for
//! the side-by-side. Wireless-attached devices are always full
//! hives — no third bridge protocol; standard R2-TRUST + R2-WIRE
//! over the radio link.
//!
//! # Pairing (milestone scope)
//!
//! Phase USB-1 implements the *auto-pair stub*: any peripheral that
//! completes SYNC + CAPS is treated as authorised, and its advertised
//! transports are registered with the host. The full R2-PROVISION §5.3.4 challenge-
//! response (commit/reveal SAS, link-key derivation, reconnect HMAC)
//! lands in Phase USB-2.

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

use r2_cbor::{Decoder, Encoder, Item, Value};

use crate::usb_pair::{
    self, Commitment, HiveIdBytes, LinkKey, Nonce32, PublicKey32, ReconnectNonce, SecretKey32,
    SharedSecret,
};

/// SYNC magic (`R2` in ASCII, little-endian on the wire).
pub const SYNC_MAGIC: u16 = 0x5232;

/// R2-USB protocol version this host implementation prefers. Negotiated
/// down per §3.3 if the peripheral advertises a lower version.
pub const PREFERRED_VERSION: u8 = 2;

/// Maximum payload length we accept on a single framed read. Matches
/// R2-WIRE compact-mode max body (R2-WIRE §3.2.1) plus headroom for
/// the type byte and CAPS overhead. A peripheral that advertises a
/// frame larger than this is malformed.
pub const MAX_PAYLOAD: usize = 4096;

/// Type bytes per R2-USB §3.5.
pub const TYPE_CAPS: u8 = 0xFE;
pub const TYPE_CONTROL: u8 = 0xFF;
/// `local_id` values (0x00..0xFB) are valid R2-WIRE tagged frames.
pub const TYPE_LOCAL_ID_MAX: u8 = 0xFB;

// ---------------------------------------------------------------------
// Public events surface
// ---------------------------------------------------------------------

/// Output of [`UsbSession::ingest_bytes`]. The caller acts on each
/// event in turn.
#[derive(Debug, Clone)]
pub enum UsbEvent {
    /// SYNC handshake completed. `version` is the negotiated effective
    /// version (`min(host, peripheral)` per §3.3).
    SyncNegotiated { version: u8, flags: u8 },
    /// CAPS frame received and parsed (v2 only). Contains the
    /// peripheral's `hive_id_bytes`, firmware identifier/version, and the
    /// list of transports it offers.
    Caps(CapsFrame),
    /// User confirmation required (R2-PROVISION §5.3.4, SAS verification). The caller MUST
    /// render `sas_code` as a 6-digit decimal (`{:06}`) and ask the
    /// operator to compare with what the peripheral displays. Then
    /// call [`UsbSession::user_confirms`] or
    /// [`UsbSession::user_aborts`] within the §5.3.4 user-confirm
    /// timeout (60 s).
    PairingPrompt {
        hive_id_bytes: [u8; 16],
        firmware_id: String,
        sas_code: u32,
    },
    /// Pairing or reconnect completed; the peripheral is now
    /// authenticated and CAPS-advertised transports MAY be activated.
    /// `reconnect = true` when a stored link key was reused;
    /// `reconnect = false` when a fresh §5.3.4 first-attach pairing
    /// just completed.
    Paired {
        hive_id_bytes: [u8; 16],
        reconnect: bool,
    },
    /// Pairing or reconnect failed. Session is closed; the operator
    /// surface (UI / log) should report the reason verbatim — these
    /// match the R2-PROVISION §5.3.4 (message vocabulary pending ratification)
    /// `PAIR_ABORT` reason vocabulary.
    PairingFailed { reason: String },
    /// An R2-WIRE frame addressed to the transport identified by
    /// `local_id`. The bytes are the R2-WIRE frame body verbatim — no
    /// type byte, no length prefix.
    WireFrame { local_id: u8, bytes: Vec<u8> },
    /// A control frame per §3.7. `msg_type` matches the §3.7
    /// vocabulary (`1` = error report, `2` = log line, `3` = transport
    /// state change, `4..=11` = pairing messages — handled internally
    /// by the pairing state machine and NOT surfaced through this
    /// variant during pairing).
    Control { msg_type: u64, body: Vec<u8> },
    /// Protocol violation. The caller MUST close the link.
    Error(UsbError),
}

/// Why the framer rejected a frame.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UsbError {
    /// Length prefix exceeds [`MAX_PAYLOAD`].
    #[error("frame too large: {0} > {max}", max = MAX_PAYLOAD)]
    FrameTooLarge(usize),
    /// SYNC frame had wrong magic or wrong size.
    #[error("malformed SYNC: {0}")]
    BadSync(&'static str),
    /// Peripheral negotiated a version we don't support.
    #[error("unsupported version: peripheral={0}")]
    UnsupportedVersion(u8),
    /// Type byte 0xFC or 0xFD (reserved per §3.5) appeared on the wire.
    #[error("reserved type byte 0x{0:02X}")]
    ReservedType(u8),
    /// CAPS frame failed CBOR decode or required-field validation.
    #[error("malformed CAPS: {0}")]
    BadCaps(&'static str),
    /// A frame arrived in the wrong session state.
    #[error("unexpected frame in {state:?}")]
    OutOfOrder { state: SessionState },
}

/// Parsed CAPS frame.
#[derive(Debug, Clone)]
pub struct CapsFrame {
    pub hive_id_bytes: [u8; 16],
    pub firmware_id: String,
    pub firmware_version: u64,
    pub transports: Vec<TransportDescriptor>,
}

/// One transport descriptor inside CAPS.transports[].
#[derive(Debug, Clone)]
pub struct TransportDescriptor {
    pub local_id: u8,
    /// Kind: integer for the canonical enum (Appendix A) or string for
    /// experimental / vendor-specific transports.
    pub kind: TransportKind,
    pub region: Option<String>,
    /// Properties map left as the raw CBOR body bytes; later phases
    /// can decode per-kind schemas. Empty Vec when CAPS omitted the
    /// field.
    pub properties_cbor: Vec<u8>,
}

/// Transport kind from CAPS field 1.
#[derive(Debug, Clone)]
pub enum TransportKind {
    /// Integer enum per R2-USB Appendix A (1..8 = lora/ble/wifi/eth/
    /// zigbee/802154/nrf24/thread; 9..99 reserved; 100+ experimental).
    Enumerated(u64),
    /// Text kind name for experimental / vendor transports.
    Named(String),
}

// ---------------------------------------------------------------------
// Session state machine
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Have not sent or received any SYNC yet.
    Initial,
    /// Local SYNC has been emitted; waiting for the peripheral's SYNC.
    SyncSent,
    /// Peripheral's SYNC parsed, version negotiated as v2, awaiting CAPS.
    AwaitingCaps,
    /// CAPS arrived; host sent `RECONNECT_CHALLENGE`; awaiting
    /// `RECONNECT_RESPONSE` (R2-PROVISION §5.3.4, Reconnect).
    Reconnecting,
    /// Host sent `PAIR_HELLO_HOST` (R2-PROVISION §5.3.4, message vocabulary
    /// pending ratification); awaiting `PAIR_COMMIT`.
    PairingHelloSent,
    /// Got `PAIR_COMMIT`; awaiting `PAIR_REVEAL`.
    PairingCommitReceived,
    /// Got `PAIR_REVEAL`, computed Z + SAS; awaiting user confirmation
    /// via [`UsbSession::user_confirms`] / [`UsbSession::user_aborts`].
    PairingAwaitingUser,
    /// User confirmed; sent `PAIR_CONFIRM`; awaiting `PAIR_DONE`.
    PairingConfirmSent,
    /// Operating normally — frames flow.
    Active,
    /// Closed (after fatal error or explicit close). No further events.
    Closed,
}

/// Pairing mode (R2-PROVISION §5.3.4 vs Phase USB-1 dev-only auto-pair).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingMode {
    /// **DEFAULT in production.** Full R2-PROVISION §5.3.4 challenge-response on
    /// first attach, HMAC reconnect on subsequent attaches.
    Strict,
    /// Phase USB-1 stub: any peripheral that completes SYNC + CAPS is
    /// trusted. For development rigs only — do not ship.
    AutoPairUnsafe,
}

/// In-flight first-attach pairing state.
#[derive(Debug)]
struct PairingFlow {
    eph_sk_host: SecretKey32,
    eph_pk_host: PublicKey32,
    nonce_host: Nonce32,
    hive_id_bytes: HiveIdBytes,
    firmware_id: String,
    commit: Option<Commitment>,
    eph_pk_peripheral: Option<PublicKey32>,
    nonce_peripheral: Option<Nonce32>,
    z: Option<SharedSecret>,
    sas_code: Option<u32>,
    pending_link_key: Option<LinkKey>,
}

impl Drop for PairingFlow {
    fn drop(&mut self) {
        // Best-effort wipe — actual zeroize would require zeroize on
        // every field; the secrets are already short-lived.
        self.eph_sk_host = [0u8; 32];
        self.nonce_host = [0u8; 32];
        if let Some(z) = self.z.as_mut() {
            *z = [0u8; 32];
        }
        if let Some(lk) = self.pending_link_key.as_mut() {
            *lk = [0u8; 32];
        }
    }
}

/// In-flight reconnect state.
#[derive(Debug)]
struct ReconnectFlow {
    hive_id_bytes: HiveIdBytes,
    nonce_rc: ReconnectNonce,
    link_key: LinkKey,
}

impl Drop for ReconnectFlow {
    fn drop(&mut self) {
        self.link_key = [0u8; 32];
    }
}

/// Persistent store of `(hive_id_bytes → link_key)` pairings. Implementations
/// hold the keys at the host-user-session scope (see R2-HIVE §3 for the
/// per-user identity model). The session interacts via this trait so
/// production deployments can swap in a file-backed or keyring-backed
/// store without changing the protocol code.
pub trait LinkKeyStore: Send + Sync {
    fn lookup(&self, hive_id_bytes: &HiveIdBytes) -> Option<LinkKey>;
    fn store(&self, hive_id_bytes: &HiveIdBytes, link_key: &LinkKey);
    fn revoke(&self, hive_id_bytes: &HiveIdBytes);
}

/// In-memory link-key store. Suitable for tests and for processes that
/// don't need persistence across restarts. Production deployments
/// should swap in a file-backed store rooted under the host-user
/// identity directory.
#[derive(Debug, Default)]
pub struct InMemoryLinkKeyStore {
    inner: RwLock<std::collections::HashMap<HiveIdBytes, LinkKey>>,
}

impl InMemoryLinkKeyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LinkKeyStore for InMemoryLinkKeyStore {
    fn lookup(&self, hive_id_bytes: &HiveIdBytes) -> Option<LinkKey> {
        self.inner.read().unwrap().get(hive_id_bytes).copied()
    }
    fn store(&self, hive_id_bytes: &HiveIdBytes, link_key: &LinkKey) {
        self.inner
            .write()
            .unwrap()
            .insert(*hive_id_bytes, *link_key);
    }
    fn revoke(&self, hive_id_bytes: &HiveIdBytes) {
        self.inner.write().unwrap().remove(hive_id_bytes);
    }
}

/// Bytes-in / events-out R2-USB session.
pub struct UsbSession {
    state: SessionState,
    /// Negotiated protocol version. 0 until SYNC completes.
    version: u8,
    /// Inbound byte buffer — bytes accumulated from the wire that
    /// haven't yet formed a complete length-prefixed frame.
    inbox: VecDeque<u8>,
    /// Bytes the caller should write to the peripheral. Drained by
    /// [`UsbSession::take_outbound`].
    outbox: VecDeque<u8>,
    /// Captured CAPS, set after first AwaitingCaps→Active transition.
    /// Re-issued CAPS on hot-plug update this in place.
    caps: Option<CapsFrame>,
    /// Pairing mode (Strict by default; AutoPairUnsafe for dev rigs).
    mode: PairingMode,
    /// Link-key store consulted on attach (Strict mode only).
    link_keys: Arc<dyn LinkKeyStore>,
    /// In-flight first-attach pairing flow (Strict mode).
    pairing: Option<PairingFlow>,
    /// In-flight reconnect flow (Strict mode).
    reconnecting: Option<ReconnectFlow>,
    /// Test-injected ephemeral keypair seed (None ⇒ getrandom).
    test_eph_sk_host: Option<SecretKey32>,
    test_nonce_host: Option<Nonce32>,
    test_nonce_rc: Option<ReconnectNonce>,
}

impl Default for UsbSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UsbSession {
    /// Build a fresh session in **Strict** pairing mode with an
    /// in-memory link-key store. The host MUST call [`Self::send_sync`]
    /// once after construction to enqueue the outbound SYNC.
    pub fn new() -> Self {
        Self::with_link_key_store(Arc::new(InMemoryLinkKeyStore::new()))
    }

    /// Build a Strict-mode session with a caller-supplied link-key
    /// store (file-backed or keyring-backed in production).
    pub fn with_link_key_store(store: Arc<dyn LinkKeyStore>) -> Self {
        Self {
            state: SessionState::Initial,
            version: 0,
            inbox: VecDeque::new(),
            outbox: VecDeque::new(),
            caps: None,
            mode: PairingMode::Strict,
            link_keys: store,
            pairing: None,
            reconnecting: None,
            test_eph_sk_host: None,
            test_nonce_host: None,
            test_nonce_rc: None,
        }
    }

    /// **Dev/test only.** Build a session that auto-pairs every
    /// peripheral that completes SYNC + CAPS — the Phase USB-1 stub
    /// behaviour. NEVER use in production — anyone who plugs a USB
    /// device in becomes trusted.
    pub fn new_auto_pair_unsafe() -> Self {
        let mut s = Self::new();
        s.mode = PairingMode::AutoPairUnsafe;
        s
    }

    /// Test-only: inject deterministic ephemeral keys + nonces so
    /// pairing flows reproduce the canonical vectors byte-for-byte.
    /// In production, `getrandom` fills these.
    #[cfg(test)]
    pub fn inject_test_keys(
        &mut self,
        eph_sk_host: SecretKey32,
        nonce_host: Nonce32,
        nonce_rc: ReconnectNonce,
    ) {
        self.test_eph_sk_host = Some(eph_sk_host);
        self.test_nonce_host = Some(nonce_host);
        self.test_nonce_rc = Some(nonce_rc);
    }

    /// Enqueue our outbound SYNC frame. Idempotent: subsequent calls
    /// after the first are no-ops.
    pub fn send_sync(&mut self) {
        if self.state != SessionState::Initial {
            return;
        }
        let payload = encode_sync_payload(PREFERRED_VERSION, 0);
        let frame = encode_length_prefixed(&payload);
        self.outbox.extend(frame);
        self.state = SessionState::SyncSent;
    }

    /// Drain any pending outbound bytes.
    pub fn take_outbound(&mut self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.outbox.len());
        out.extend(self.outbox.drain(..));
        out
    }

    /// Push an outbound R2-WIRE frame to be carried over the dongle's
    /// `local_id` transport (R2-USB §3.5). The session wraps `bytes`
    /// with the type-byte prefix and length-prefix framing. Only
    /// valid in the [`SessionState::Active`] state — pre-pairing
    /// frames are silently dropped (the peripheral would reject them
    /// per R2-PROVISION §5.3.4 (message vocabulary pending ratification) anyway).
    ///
    /// Returns `true` if the frame was queued, `false` if the session
    /// is not yet authorised to send wire data.
    pub fn send_wire_frame(&mut self, local_id: u8, bytes: &[u8]) -> bool {
        if !matches!(self.state, SessionState::Active) {
            return false;
        }
        if local_id > TYPE_LOCAL_ID_MAX {
            return false;
        }
        let mut payload = Vec::with_capacity(1 + bytes.len());
        payload.push(local_id);
        payload.extend_from_slice(bytes);
        self.outbox.extend(encode_length_prefixed(&payload));
        true
    }

    /// Feed bytes received from the peripheral. Returns zero or more
    /// [`UsbEvent`]s.
    pub fn ingest_bytes(&mut self, bytes: &[u8]) -> Vec<UsbEvent> {
        if matches!(self.state, SessionState::Closed) {
            return Vec::new();
        }
        self.inbox.extend(bytes);
        let mut events = Vec::new();
        while let Some(payload) = self.try_pop_frame(&mut events) {
            self.dispatch_payload(&payload, &mut events);
            if matches!(self.state, SessionState::Closed) {
                break;
            }
        }
        events
    }

    /// Current state — exposed for tests and ops surfaces.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Most recent CAPS frame, if any.
    pub fn caps(&self) -> Option<&CapsFrame> {
        self.caps.as_ref()
    }

    // ---- internal --------------------------------------------------

    fn try_pop_frame(&mut self, events: &mut Vec<UsbEvent>) -> Option<Vec<u8>> {
        if self.inbox.len() < 2 {
            return None;
        }
        let len_lo = self.inbox[0];
        let len_hi = self.inbox[1];
        let payload_len = u16::from_le_bytes([len_lo, len_hi]) as usize;
        if payload_len > MAX_PAYLOAD {
            self.fatal(UsbError::FrameTooLarge(payload_len), events);
            return None;
        }
        if self.inbox.len() < 2 + payload_len {
            return None; // wait for more bytes
        }
        // Drop the length prefix and copy the payload.
        self.inbox.drain(..2);
        let payload: Vec<u8> = self.inbox.drain(..payload_len).collect();
        Some(payload)
    }

    fn dispatch_payload(&mut self, payload: &[u8], events: &mut Vec<UsbEvent>) {
        match self.state {
            SessionState::SyncSent => self.dispatch_sync(payload, events),
            // Once SYNC negotiates, every subsequent frame is typed
            // (CAPS, control, R2-WIRE). The pairing state machine
            // dispatches msg_type 4..=11 control frames internally.
            SessionState::AwaitingCaps
            | SessionState::Active
            | SessionState::Reconnecting
            | SessionState::PairingHelloSent
            | SessionState::PairingCommitReceived
            | SessionState::PairingAwaitingUser
            | SessionState::PairingConfirmSent => self.dispatch_typed(payload, events),
            SessionState::Initial | SessionState::Closed => {
                self.fatal(
                    UsbError::OutOfOrder {
                        state: self.state,
                    },
                    events,
                );
            }
        }
    }

    fn dispatch_sync(&mut self, payload: &[u8], events: &mut Vec<UsbEvent>) {
        match decode_sync_payload(payload) {
            Ok((version, flags)) => {
                let negotiated = version.min(PREFERRED_VERSION);
                if negotiated == 0 || negotiated > PREFERRED_VERSION {
                    self.fatal(UsbError::UnsupportedVersion(version), events);
                    return;
                }
                self.version = negotiated;
                events.push(UsbEvent::SyncNegotiated {
                    version: negotiated,
                    flags,
                });
                self.state = if negotiated >= 2 {
                    SessionState::AwaitingCaps
                } else {
                    // v1 = legacy peer mode. The peripheral runs a full
                    // R2-WIRE stack; frames are not type-byte tagged.
                    // Phase USB-1 doesn't implement the v1 path —
                    // calling code SHOULD reject v1 peripherals until
                    // the legacy decoder lands. Enter Active here so
                    // that any frames the peripheral emits round-trip
                    // through `WireFrame { local_id: 0, bytes }` for
                    // diagnostics; production deployments will close
                    // the link on this branch.
                    SessionState::Active
                };
            }
            Err(e) => self.fatal(UsbError::BadSync(e), events),
        }
    }

    fn dispatch_typed(&mut self, payload: &[u8], events: &mut Vec<UsbEvent>) {
        if payload.is_empty() {
            self.fatal(UsbError::BadSync("empty typed frame"), events);
            return;
        }
        // v1 legacy mode: no type byte; whole payload is R2-WIRE.
        if self.version < 2 {
            events.push(UsbEvent::WireFrame {
                local_id: 0,
                bytes: payload.to_vec(),
            });
            return;
        }
        let type_byte = payload[0];
        let body = &payload[1..];
        match type_byte {
            0xFC | 0xFD => {
                self.fatal(UsbError::ReservedType(type_byte), events);
            }
            TYPE_CAPS => match parse_caps(body) {
                Ok(caps) => {
                    self.caps = Some(caps.clone());
                    events.push(UsbEvent::Caps(caps.clone()));
                    if matches!(self.state, SessionState::AwaitingCaps) {
                        self.on_caps_received(&caps, events);
                    }
                    // Re-issued CAPS in Active state is a hot-plug
                    // notification (§5.4); the host's transport
                    // inventory updates but we don't restart pairing.
                }
                Err(e) => self.fatal(UsbError::BadCaps(e), events),
            },
            TYPE_CONTROL => match parse_control(body) {
                Ok((msg_type, body_bytes)) => {
                    if self.is_pairing_msg_type(msg_type) {
                        self.dispatch_pairing_frame(msg_type, &body_bytes, events);
                    } else {
                        events.push(UsbEvent::Control {
                            msg_type,
                            body: body_bytes,
                        });
                    }
                }
                Err(e) => self.fatal(UsbError::BadCaps(e), events),
            },
            id if id <= TYPE_LOCAL_ID_MAX => {
                // R2-WIRE frames are only valid on a paired link.
                // Pre-pairing, drop silently per R2-PROVISION §5.3.4 / R2-USB §5.4.
                if !matches!(self.state, SessionState::Active) {
                    return;
                }
                events.push(UsbEvent::WireFrame {
                    local_id: id,
                    bytes: body.to_vec(),
                });
            }
            // 0xFC / 0xFD already handled; everything else falls here
            // only via unreachable patterns.
            _ => unreachable!(),
        }
    }

    fn is_pairing_msg_type(&self, msg_type: u64) -> bool {
        // R2-USB §3.7: msg_type 4..=11 carry pairing flow per
        // R2-PROVISION §5.3.4 (message vocabulary / Reconnect). Always interpret as pairing while
        // pairing or reconnecting state is in flight; in Active
        // (already paired) drop them quietly — the peripheral
        // shouldn't be sending pairing frames after pairing is done.
        matches!(msg_type, 4..=11)
            && matches!(
                self.state,
                SessionState::AwaitingCaps
                    | SessionState::Reconnecting
                    | SessionState::PairingHelloSent
                    | SessionState::PairingCommitReceived
                    | SessionState::PairingAwaitingUser
                    | SessionState::PairingConfirmSent
            )
    }

    /// Choose between reconnect (known hive_id_bytes, stored link key) and
    /// first-attach pairing (unknown hive_id_bytes) — or, in
    /// AutoPairUnsafe mode, skip both and go straight to Active.
    fn on_caps_received(&mut self, caps: &CapsFrame, events: &mut Vec<UsbEvent>) {
        match self.mode {
            PairingMode::AutoPairUnsafe => {
                self.state = SessionState::Active;
                events.push(UsbEvent::Paired {
                    hive_id_bytes: caps.hive_id_bytes,
                    reconnect: false,
                });
            }
            PairingMode::Strict => {
                if let Some(link_key) = self.link_keys.lookup(&caps.hive_id_bytes) {
                    self.start_reconnect(caps.hive_id_bytes, link_key, events);
                } else {
                    self.start_pairing(caps.hive_id_bytes, caps.firmware_id.clone(), events);
                }
            }
        }
    }

    fn start_reconnect(
        &mut self,
        hive_id_bytes: HiveIdBytes,
        link_key: LinkKey,
        events: &mut Vec<UsbEvent>,
    ) {
        let nonce_rc = self.test_nonce_rc.unwrap_or_else(|| {
            let mut n = [0u8; 16];
            getrandom::getrandom(&mut n).expect("getrandom");
            n
        });
        self.reconnecting = Some(ReconnectFlow {
            hive_id_bytes,
            nonce_rc,
            link_key,
        });
        let frame = build_pair_msg(
            9, // RECONNECT_CHALLENGE
            &[(1u64, CborField::Bytes(&nonce_rc))],
        );
        self.outbox.extend(encode_length_prefixed(&frame));
        self.state = SessionState::Reconnecting;
        let _ = events; // no event emitted at challenge time; user sees Paired or PairingFailed
    }

    fn start_pairing(
        &mut self,
        hive_id_bytes: HiveIdBytes,
        firmware_id: String,
        events: &mut Vec<UsbEvent>,
    ) {
        let eph_sk_host = self.test_eph_sk_host.unwrap_or_else(|| {
            let mut sk = [0u8; 32];
            getrandom::getrandom(&mut sk).expect("getrandom");
            sk
        });
        let nonce_host = self.test_nonce_host.unwrap_or_else(|| {
            let mut n = [0u8; 32];
            getrandom::getrandom(&mut n).expect("getrandom");
            n
        });
        let eph_pk_host = usb_pair::public_key_from_secret(&eph_sk_host);
        let frame = build_pair_msg(
            4, // PAIR_HELLO_HOST
            &[
                (1u64, CborField::Bytes(&eph_pk_host)),
                (2u64, CborField::Bytes(&nonce_host)),
            ],
        );
        self.outbox.extend(encode_length_prefixed(&frame));
        self.pairing = Some(PairingFlow {
            eph_sk_host,
            eph_pk_host,
            nonce_host,
            hive_id_bytes,
            firmware_id,
            commit: None,
            eph_pk_peripheral: None,
            nonce_peripheral: None,
            z: None,
            sas_code: None,
            pending_link_key: None,
        });
        self.state = SessionState::PairingHelloSent;
        let _ = events;
    }

    fn dispatch_pairing_frame(
        &mut self,
        msg_type: u64,
        body: &[u8],
        events: &mut Vec<UsbEvent>,
    ) {
        match (msg_type, self.state) {
            (5, SessionState::PairingHelloSent) => self.handle_pair_commit(body, events),
            (6, SessionState::PairingCommitReceived) => self.handle_pair_reveal(body, events),
            (8, SessionState::PairingConfirmSent) => self.handle_pair_done(body, events),
            (10, SessionState::Reconnecting) => self.handle_reconnect_response(body, events),
            (11, _) => self.handle_pair_abort(body, events),
            _ => {
                // Any other pairing msg_type in any unexpected state
                // is a protocol violation. Per R2-PROVISION §5.3.4 (message
                // vocabulary pending ratification), abort with a
                // protocol_error reason.
                self.send_abort("protocol_error");
                self.fail_pairing("protocol_error", events);
            }
        }
    }

    fn handle_pair_commit(&mut self, body: &[u8], events: &mut Vec<UsbEvent>) {
        let commit = match extract_bstr_field::<32>(body, 1) {
            Some(c) => c,
            None => {
                self.send_abort("protocol_error");
                return self.fail_pairing("bad_commit", events);
            }
        };
        if let Some(p) = self.pairing.as_mut() {
            p.commit = Some(commit);
        }
        self.state = SessionState::PairingCommitReceived;
    }

    fn handle_pair_reveal(&mut self, body: &[u8], events: &mut Vec<UsbEvent>) {
        let pk_periph = match extract_bstr_field::<32>(body, 1) {
            Some(p) => p,
            None => {
                self.send_abort("protocol_error");
                return self.fail_pairing("bad_reveal", events);
            }
        };
        let nonce_periph = match extract_bstr_field::<32>(body, 2) {
            Some(n) => n,
            None => {
                self.send_abort("protocol_error");
                return self.fail_pairing("bad_reveal", events);
            }
        };
        let p = match self.pairing.as_mut() {
            Some(p) => p,
            None => return self.fail_pairing("internal", events),
        };
        let commit = match p.commit {
            Some(c) => c,
            None => return self.fail_pairing("internal", events),
        };
        if !usb_pair::verify_commitment(&commit, &pk_periph, &nonce_periph) {
            self.send_abort("commit_mismatch");
            return self.fail_pairing("commit_mismatch", events);
        }
        let z = usb_pair::shared_secret(&p.eph_sk_host, &pk_periph);
        let sas = usb_pair::sas_code(
            &z,
            &p.eph_pk_host,
            &pk_periph,
            &p.nonce_host,
            &nonce_periph,
        );
        let lk = usb_pair::link_key(
            &z,
            &p.eph_pk_host,
            &pk_periph,
            &p.nonce_host,
            &nonce_periph,
            &p.hive_id_bytes,
        );
        p.eph_pk_peripheral = Some(pk_periph);
        p.nonce_peripheral = Some(nonce_periph);
        p.z = Some(z);
        p.sas_code = Some(sas);
        p.pending_link_key = Some(lk);
        let hive_id_bytes = p.hive_id_bytes;
        let firmware_id = p.firmware_id.clone();
        self.state = SessionState::PairingAwaitingUser;
        events.push(UsbEvent::PairingPrompt {
            hive_id_bytes,
            firmware_id,
            sas_code: sas,
        });
    }

    fn handle_pair_done(&mut self, _body: &[u8], events: &mut Vec<UsbEvent>) {
        let p = match self.pairing.take() {
            Some(p) => p,
            None => return self.fail_pairing("internal", events),
        };
        let lk = match p.pending_link_key {
            Some(lk) => lk,
            None => return self.fail_pairing("internal", events),
        };
        self.link_keys.store(&p.hive_id_bytes, &lk);
        let hive_id_bytes = p.hive_id_bytes;
        drop(p);
        events.push(UsbEvent::Paired {
            hive_id_bytes,
            reconnect: false,
        });
        self.state = SessionState::Active;
    }

    fn handle_reconnect_response(&mut self, body: &[u8], events: &mut Vec<UsbEvent>) {
        let tag = match extract_bstr_field::<16>(body, 1) {
            Some(t) => t,
            None => {
                self.send_abort("protocol_error");
                return self.fail_pairing("bad_response", events);
            }
        };
        let r = match self.reconnecting.take() {
            Some(r) => r,
            None => return self.fail_pairing("internal", events),
        };
        if !usb_pair::verify_reconnect_tag(&tag, &r.link_key, &r.nonce_rc, &r.hive_id_bytes) {
            self.send_abort("reconnect_failed");
            return self.fail_pairing("reconnect_failed", events);
        }
        let hive_id_bytes = r.hive_id_bytes;
        drop(r);
        events.push(UsbEvent::Paired {
            hive_id_bytes,
            reconnect: true,
        });
        self.state = SessionState::Active;
    }

    fn handle_pair_abort(&mut self, body: &[u8], events: &mut Vec<UsbEvent>) {
        let reason = extract_tstr_field(body, 1).unwrap_or_else(|| "unknown".to_string());
        self.fail_pairing(&reason, events);
    }

    /// Operator confirms the SAS code matched on the peripheral.
    /// Valid only in `PairingAwaitingUser`.
    pub fn user_confirms(&mut self) {
        if !matches!(self.state, SessionState::PairingAwaitingUser) {
            return;
        }
        let confirm = build_pair_msg(7 /* PAIR_CONFIRM */, &[]);
        self.outbox.extend(encode_length_prefixed(&confirm));
        self.state = SessionState::PairingConfirmSent;
    }

    /// Operator rejects the SAS code (mismatch, change of mind, or
    /// 60 s timeout per R2-PROVISION §5.3.4, SAS verification). Sends `PAIR_ABORT { reason: "user_aborted" }`
    /// and closes.
    pub fn user_aborts(&mut self) {
        if !matches!(self.state, SessionState::PairingAwaitingUser) {
            return;
        }
        self.send_abort("user_aborted");
        // Don't emit PairingFailed yet — wait for the peripheral's
        // ACK or timeout. Simpler model: emit it now and close.
        let mut events = Vec::new();
        self.fail_pairing("user_aborted", &mut events);
        // Caller's next ingest_bytes() call won't see these, so
        // surface them via outbox-readable state. For symmetry with
        // other error paths, callers SHOULD observe state == Closed
        // after user_aborts() and treat any pending UsbEvent as
        // moot.
    }

    fn send_abort(&mut self, reason: &str) {
        let frame = build_pair_msg(
            11, // PAIR_ABORT
            &[(1u64, CborField::Text(reason))],
        );
        self.outbox.extend(encode_length_prefixed(&frame));
    }

    fn fail_pairing(&mut self, reason: &str, events: &mut Vec<UsbEvent>) {
        events.push(UsbEvent::PairingFailed {
            reason: reason.to_string(),
        });
        self.pairing = None;
        self.reconnecting = None;
        self.state = SessionState::Closed;
    }

    fn fatal(&mut self, err: UsbError, events: &mut Vec<UsbEvent>) {
        events.push(UsbEvent::Error(err));
        self.state = SessionState::Closed;
    }
}

// ---------------------------------------------------------------------
// SYNC and length-prefix codec
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// §3.7 control-frame builders for pairing — type 0xFF, body is an
// integer-keyed CBOR map per R2-PROVISION §5.3.4 (message vocabulary pending ratification).
// ---------------------------------------------------------------------

enum CborField<'a> {
    Bytes(&'a [u8]),
    Text(&'a str),
}

/// Build a §3.7 control frame body that carries a R2-PROVISION §5.3.4 pairing
/// message. Returns `[0xFF] || cbor({0: msg_type, 1: {fields...}})`
/// suitable for [`encode_length_prefixed`].
fn build_pair_msg(msg_type: u64, fields: &[(u64, CborField<'_>)]) -> Vec<u8> {
    // Body of the control frame: {0: msg_type, 1: {pairing-body...}}
    let mut buf = vec![0u8; 2 + 64 + 64 * fields.len()];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("control map");
        enc.kv(0, &Value::UInt(msg_type)).expect("msg_type");
        enc.uint(1).expect("body key");
        enc.map(fields.len()).expect("body map");
        for (k, f) in fields {
            enc.uint(*k).expect("field key");
            match f {
                CborField::Bytes(b) => enc.value(&Value::Bytes(b)).expect("bytes"),
                CborField::Text(s) => enc.value(&Value::Text(s)).expect("text"),
            }
        }
        enc.len()
    };
    // Prepend the type byte (0xFF = control).
    let mut out = Vec::with_capacity(1 + used);
    out.push(TYPE_CONTROL);
    out.extend_from_slice(&buf[..used]);
    out
}

/// Extract a fixed-size byte string under integer key `target_key`
/// from a CBOR map. Returns `None` if the key is missing or the
/// value isn't a byte string of the expected length.
fn extract_bstr_field<const N: usize>(body: &[u8], target_key: u64) -> Option<[u8; N]> {
    let mut dec = Decoder::new(body);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(k) = key {
            if k == target_key {
                if let Item::Bytes(b) = val {
                    if b.len() == N {
                        let mut a = [0u8; N];
                        a.copy_from_slice(b);
                        return Some(a);
                    }
                }
                return None;
            }
        }
    }
    None
}

/// Extract a UTF-8 text string under integer key `target_key`.
fn extract_tstr_field(body: &[u8], target_key: u64) -> Option<String> {
    let mut dec = Decoder::new(body);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(k) = key {
            if k == target_key {
                if let Item::Text(s) = val {
                    return std::str::from_utf8(s).ok().map(|s| s.to_string());
                }
                return None;
            }
        }
    }
    None
}

fn encode_sync_payload(version: u8, flags: u8) -> [u8; 4] {
    let mut out = [0u8; 4];
    out[..2].copy_from_slice(&SYNC_MAGIC.to_le_bytes());
    out[2] = version;
    out[3] = flags;
    out
}

fn decode_sync_payload(payload: &[u8]) -> Result<(u8, u8), &'static str> {
    if payload.len() != 4 {
        return Err("payload not 4 bytes");
    }
    let magic = u16::from_le_bytes([payload[0], payload[1]]);
    if magic != SYNC_MAGIC {
        return Err("magic mismatch");
    }
    Ok((payload[2], payload[3]))
}

/// Wrap a payload with a 2-byte LE length prefix per R2-USB §3.2.
pub fn encode_length_prefixed(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 2);
    out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Build a complete v2 SYNC frame (length prefix + payload). Used by
/// tests and by the production session via [`UsbSession::send_sync`].
pub fn build_sync_frame(version: u8, flags: u8) -> Vec<u8> {
    encode_length_prefixed(&encode_sync_payload(version, flags))
}

// ---------------------------------------------------------------------
// CAPS parser
// ---------------------------------------------------------------------

fn parse_caps(body: &[u8]) -> Result<CapsFrame, &'static str> {
    let mut dec = Decoder::new(body);
    let entries = match dec.next().map_err(|_| "CAPS not CBOR")? {
        Item::Map(n) => n,
        _ => return Err("CAPS root not a map"),
    };

    let mut hive_id_bytes: Option<[u8; 16]> = None;
    let mut firmware_id: Option<String> = None;
    let mut firmware_version: Option<u64> = None;
    let mut transports: Option<Vec<TransportDescriptor>> = None;

    for _ in 0..entries {
        let key = match dec.next().map_err(|_| "CAPS key decode")? {
            Item::UInt(k) => k,
            _ => return Err("CAPS non-integer key"),
        };
        match key {
            0 => match dec.next().map_err(|_| "CAPS hive_id_bytes decode")? {
                Item::Bytes(b) if b.len() == 16 => {
                    let mut id = [0u8; 16];
                    id.copy_from_slice(b);
                    hive_id_bytes = Some(id);
                }
                _ => return Err("CAPS hive_id_bytes not bstr/16"),
            },
            1 => match dec.next().map_err(|_| "CAPS firmware_id decode")? {
                Item::Text(s) => {
                    firmware_id = Some(
                        std::str::from_utf8(s)
                            .map_err(|_| "CAPS firmware_id not utf8")?
                            .to_string(),
                    );
                }
                _ => return Err("CAPS firmware_id not tstr"),
            },
            2 => match dec.next().map_err(|_| "CAPS firmware_version decode")? {
                Item::UInt(v) => firmware_version = Some(v),
                _ => return Err("CAPS firmware_version not uint"),
            },
            3 => {
                transports = Some(parse_transports(&mut dec)?);
            }
            _ => {
                // Unknown CAPS keys are forward-compatible per spec —
                // skip the value.
                let _ = skip_one(&mut dec);
            }
        }
    }

    let hive_id_bytes = hive_id_bytes.ok_or("CAPS missing hive_id_bytes")?;
    let firmware_id = firmware_id.ok_or("CAPS missing firmware_id")?;
    let firmware_version = firmware_version.ok_or("CAPS missing firmware_version")?;
    let transports = transports.ok_or("CAPS missing transports")?;
    if transports.is_empty() || transports.len() > 16 {
        return Err("CAPS transports must be 1..16");
    }
    // local_id uniqueness check (§3.5 + §5.4)
    let mut seen = [false; 256];
    for t in &transports {
        if seen[t.local_id as usize] {
            return Err("duplicate local_id in CAPS");
        }
        seen[t.local_id as usize] = true;
        if t.local_id > TYPE_LOCAL_ID_MAX {
            return Err("local_id >= 0xFC reserved");
        }
    }
    Ok(CapsFrame {
        hive_id_bytes,
        firmware_id,
        firmware_version,
        transports,
    })
}

fn parse_transports(dec: &mut Decoder<'_>) -> Result<Vec<TransportDescriptor>, &'static str> {
    let n = match dec.next().map_err(|_| "transports decode")? {
        Item::Array(n) => n,
        _ => return Err("transports not an array"),
    };
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(parse_transport(dec)?);
    }
    Ok(out)
}

fn parse_transport(dec: &mut Decoder<'_>) -> Result<TransportDescriptor, &'static str> {
    let entries = match dec.next().map_err(|_| "transport decode")? {
        Item::Map(n) => n,
        _ => return Err("transport not a map"),
    };
    let mut local_id: Option<u8> = None;
    let mut kind: Option<TransportKind> = None;
    let mut region: Option<String> = None;
    let properties_cbor: Vec<u8> = Vec::new();
    for _ in 0..entries {
        let key = match dec.next().map_err(|_| "transport key")? {
            Item::UInt(k) => k,
            _ => return Err("transport non-integer key"),
        };
        match key {
            0 => match dec.next().map_err(|_| "local_id decode")? {
                Item::UInt(v) if v <= 0xFB => local_id = Some(v as u8),
                _ => return Err("local_id out of range"),
            },
            1 => match dec.next().map_err(|_| "kind decode")? {
                Item::UInt(v) => kind = Some(TransportKind::Enumerated(v)),
                Item::Text(s) => {
                    kind = Some(TransportKind::Named(
                        std::str::from_utf8(s)
                            .map_err(|_| "kind not utf8")?
                            .to_string(),
                    ));
                }
                _ => return Err("kind not uint/tstr"),
            },
            2 => match dec.next().map_err(|_| "region decode")? {
                Item::Text(s) => {
                    region = Some(
                        std::str::from_utf8(s)
                            .map_err(|_| "region not utf8")?
                            .to_string(),
                    );
                }
                _ => return Err("region not tstr"),
            },
            3 => {
                // Properties are kind-specific (Appendix A); the host
                // doesn't need them to register a transport binding
                // for milestone 1. Skip without capturing — a later
                // phase will plumb per-kind decoders here.
                let _ = skip_one(dec);
            }
            _ => {
                let _ = skip_one(dec);
            }
        }
    }
    let local_id = local_id.ok_or("transport missing local_id")?;
    let kind = kind.ok_or("transport missing kind")?;
    Ok(TransportDescriptor {
        local_id,
        kind,
        region,
        properties_cbor,
    })
}

fn skip_one(dec: &mut Decoder<'_>) -> Result<(), &'static str> {
    match dec.next().map_err(|_| "skip decode")? {
        Item::Map(n) => {
            for _ in 0..n {
                skip_one(dec)?; // key
                skip_one(dec)?; // value
            }
        }
        Item::Array(n) => {
            for _ in 0..n {
                skip_one(dec)?;
            }
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Control frame parser (§3.7)
// ---------------------------------------------------------------------

fn parse_control(body: &[u8]) -> Result<(u64, Vec<u8>), &'static str> {
    let mut dec = Decoder::new(body);
    let entries = match dec.next().map_err(|_| "control not CBOR")? {
        Item::Map(n) => n,
        _ => return Err("control root not a map"),
    };
    let mut msg_type: Option<u64> = None;
    let mut body_start: Option<usize> = None;
    let mut body_end: Option<usize> = None;
    for _ in 0..entries {
        let key = match dec.next().map_err(|_| "control key")? {
            Item::UInt(k) => k,
            _ => return Err("control non-integer key"),
        };
        match key {
            0 => match dec.next().map_err(|_| "msg_type decode")? {
                Item::UInt(v) => msg_type = Some(v),
                _ => return Err("msg_type not uint"),
            },
            1 => {
                body_start = Some(dec.position());
                skip_one(&mut dec)?;
                body_end = Some(dec.position());
            }
            _ => {
                let _ = skip_one(&mut dec);
            }
        }
    }
    let mt = msg_type.ok_or("control missing msg_type")?;
    // body is the slice we constructed the Decoder from, so
    // dec.position()-based offsets index into `body` directly.
    let body_bytes = match (body_start, body_end) {
        (Some(s), Some(e)) if e <= body.len() => body[s..e].to_vec(),
        _ => Vec::new(),
    };
    Ok((mt, body_bytes))
}

// ---------------------------------------------------------------------
// Tests — replay the public r2-usb-vectors.json fixtures.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        let s = s.replace(' ', "").replace('\n', "");
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn sync_frame_round_trips_v2() {
        // TV2 from r2-usb-vectors.json — v2 SYNC.
        let frame = build_sync_frame(2, 0);
        assert_eq!(hex("040032520200"), frame);
    }

    #[test]
    fn host_session_sends_v2_sync_on_construction() {
        let mut s = UsbSession::new();
        s.send_sync();
        let out = s.take_outbound();
        assert_eq!(hex("040032520200"), out);
        assert_eq!(s.state(), SessionState::SyncSent);
    }

    #[test]
    fn negotiates_v2_when_peripheral_responds_v2() {
        let mut s = UsbSession::new();
        s.send_sync();
        let _ = s.take_outbound();
        let evs = s.ingest_bytes(&hex("040032520200"));
        assert_eq!(evs.len(), 1);
        match &evs[0] {
            UsbEvent::SyncNegotiated { version, flags } => {
                assert_eq!(*version, 2);
                assert_eq!(*flags, 0);
            }
            other => panic!("expected SyncNegotiated, got {other:?}"),
        }
        assert_eq!(s.state(), SessionState::AwaitingCaps);
    }

    #[test]
    fn negotiates_down_to_v1_when_peripheral_responds_v1() {
        // TV7 in r2-usb-vectors.json — mixed-version handshake.
        let mut s = UsbSession::new();
        s.send_sync();
        let _ = s.take_outbound();
        let evs = s.ingest_bytes(&hex("040032520100"));
        match &evs[0] {
            UsbEvent::SyncNegotiated { version, .. } => assert_eq!(*version, 1),
            other => panic!("expected SyncNegotiated v1, got {other:?}"),
        }
    }

    #[test]
    fn malformed_sync_closes_session() {
        let mut s = UsbSession::new();
        s.send_sync();
        let _ = s.take_outbound();
        // Wrong magic (expect 0x5232, supply 0xFFFF).
        let evs = s.ingest_bytes(&hex("0400FFFF0200"));
        assert_eq!(evs.len(), 1);
        assert!(matches!(evs[0], UsbEvent::Error(UsbError::BadSync(_))));
        assert_eq!(s.state(), SessionState::Closed);
    }

    /// TV3 from r2-usb-vectors.json — minimal CAPS for a LoRa-only
    /// peripheral (region=AU915, properties.chip=sx1262).
    const TV3_CAPS_FRAME: &str =
        "3300FEA40050000102030405060708090A0B0C0D0E0F016372327002010381A4000001010265415539313503A10066737831323632";

    #[test]
    fn parses_minimal_caps() {
        // AutoPairUnsafe: this Phase USB-1 test exercises the framer,
        // not pairing. Strict-mode pairing is exercised in the
        // `pairing_*` tests below.
        let mut s = UsbSession::new_auto_pair_unsafe();
        s.send_sync();
        let _ = s.take_outbound();
        let _ = s.ingest_bytes(&hex("040032520200"));
        let evs = s.ingest_bytes(&hex(TV3_CAPS_FRAME));
        let caps = match evs.into_iter().next() {
            Some(UsbEvent::Caps(c)) => c,
            other => panic!("expected Caps, got {other:?}"),
        };
        assert_eq!(
            caps.hive_id_bytes,
            hex("000102030405060708090A0B0C0D0E0F").as_slice()
        );
        assert_eq!(caps.firmware_version, 1);
        assert_eq!(caps.transports.len(), 1);
        let t = &caps.transports[0];
        assert_eq!(t.local_id, 0);
        assert!(matches!(t.kind, TransportKind::Enumerated(1))); // 1 = lora
        assert_eq!(t.region.as_deref(), Some("AU915"));
        assert_eq!(s.state(), SessionState::Active);
    }

    /// TV5 — tagged data frame on local_id=0 wraps an R2-WIRE evt.
    /// wire_hex: 1100 00 53A1B2424D3E4C1A2B3C4DA10018EA  (length=0x11=17 bytes)
    /// payload after type byte: 53A1B2424D3E4C1A2B3C4DA10018EA  (16 bytes)
    #[test]
    fn delivers_wire_frame_after_caps() {
        // AutoPairUnsafe: see comment in parses_minimal_caps.
        let mut s = UsbSession::new_auto_pair_unsafe();
        s.send_sync();
        let _ = s.take_outbound();
        let _ = s.ingest_bytes(&hex("040032520200"));
        let _ = s.ingest_bytes(&hex(TV3_CAPS_FRAME));
        // TV5 from r2-usb-vectors.json (wire_hex pinned).
        let evs = s.ingest_bytes(&hex("1100000053A1B2424D3E4C1A2B3C4DA10018EA"));
        let ev = evs.into_iter().last().expect("frame emitted");
        match ev {
            UsbEvent::WireFrame { local_id, bytes } => {
                assert_eq!(local_id, 0);
                assert_eq!(bytes, hex("0053A1B2424D3E4C1A2B3C4DA10018EA"));
            }
            other => panic!("expected WireFrame, got {other:?}"),
        }
    }

    /// TV5 again, but delivered before CAPS — should be dropped per §5.4.
    #[test]
    fn drops_wire_frame_before_caps() {
        let mut s = UsbSession::new();
        s.send_sync();
        let _ = s.take_outbound();
        let _ = s.ingest_bytes(&hex("040032520200"));
        let evs = s.ingest_bytes(&hex("1100000053A1B2424D3E4C1A2B3C4DA10018EA"));
        assert!(evs.is_empty(), "no events emitted: {evs:?}");
    }

    /// TV6 — control frame (msg_type=1 error report). msg_type=1 is
    /// not a pairing message (those are 4..=11), so it surfaces as
    /// UsbEvent::Control even during pairing. AutoPairUnsafe used
    /// here for symmetry with the other framer tests.
    #[test]
    fn parses_control_frame() {
        let mut s = UsbSession::new_auto_pair_unsafe();
        s.send_sync();
        let _ = s.take_outbound();
        let _ = s.ingest_bytes(&hex("040032520200"));
        let _ = s.ingest_bytes(&hex(TV3_CAPS_FRAME));
        let evs = s.ingest_bytes(&hex("1500FFA2000101A200190101016974656D706F72617279"));
        let ctrl = evs
            .into_iter()
            .find(|e| matches!(e, UsbEvent::Control { .. }))
            .expect("control event emitted");
        match ctrl {
            UsbEvent::Control { msg_type, body } => {
                assert_eq!(msg_type, 1);
                assert!(!body.is_empty());
            }
            _ => unreachable!(),
        }
    }

    /// TV11 — malformed CBOR in CAPS body (single 0xFF byte).
    #[test]
    fn malformed_caps_closes_session() {
        let mut s = UsbSession::new();
        s.send_sync();
        let _ = s.take_outbound();
        let _ = s.ingest_bytes(&hex("040032520200"));
        let evs = s.ingest_bytes(&hex("0200FEFF"));
        assert!(matches!(
            evs.last(),
            Some(UsbEvent::Error(UsbError::BadCaps(_)))
        ));
        assert_eq!(s.state(), SessionState::Closed);
    }

    /// TV12 — fragmented SYNC across reads. Length prefix arrives in
    /// one read, payload in the next. The framer holds bytes until a
    /// complete frame is available.
    #[test]
    fn fragmented_sync_reassembles() {
        let mut s = UsbSession::new();
        s.send_sync();
        let _ = s.take_outbound();
        let evs1 = s.ingest_bytes(&hex("0400"));
        assert!(evs1.is_empty());
        let evs2 = s.ingest_bytes(&hex("32520200"));
        assert!(matches!(evs2[0], UsbEvent::SyncNegotiated { version: 2, .. }));
    }

    /// TV9 — NEGATIVE: CAPS with local_id=0xFE (collides with reserved
    /// type byte). MUST be rejected.
    #[test]
    fn caps_with_reserved_local_id_rejected() {
        // Build a minimal CAPS where the single transport has local_id=0xFE.
        // CBOR: A4 00 50 <16 bytes> 01 61 78 02 01 03 81 A2 00 18 FE 01 01
        // Decoded:
        //   {0: bstr(16) deviceid, 1: tstr(1) "x", 2: 1,
        //    3: [{0: 0xFE, 1: 1}]}
        let mut body = Vec::new();
        body.push(0xFE); // type byte = CAPS
        // CAPS map (4 entries): a4
        body.push(0xa4);
        // 0: hive_id_bytes bstr(16)
        body.extend_from_slice(&hex("0050") );
        body.extend_from_slice(&[0u8; 16]);
        // 1: firmware_id tstr(1) "x"
        body.extend_from_slice(&hex("016178"));
        // 2: firmware_version 1
        body.extend_from_slice(&hex("0201"));
        // 3: transports array(1) of one descriptor
        body.extend_from_slice(&hex("0381"));
        // descriptor map(2): {0: 0xFE, 1: 1}
        body.extend_from_slice(&hex("a20018FE0101"));
        let len = (body.len() as u16).to_le_bytes();
        let mut frame = Vec::new();
        frame.extend_from_slice(&len);
        frame.extend_from_slice(&body);

        let mut s = UsbSession::new();
        s.send_sync();
        let _ = s.take_outbound();
        let _ = s.ingest_bytes(&hex("040032520200"));
        let evs = s.ingest_bytes(&frame);
        assert!(matches!(
            evs.last(),
            Some(UsbEvent::Error(UsbError::BadCaps(_)))
        ));
    }

    // ─────────────────────────────────────────────────────────────────
    // Phase USB-2 — R2-PROVISION §5.3.4 pairing flow, end-to-end against the
    // deterministic r2-usb-pair-vectors.json inputs.
    //
    // These tests use a CAPS frame whose hive_id_bytes matches the
    // synthetic 0x55-filled fixture used in r2-usb-pair-vectors.json.
    // The DFR1195 in production will have its own hive_id_bytes derived
    // from its eFuse MAC per R2-USB §3.6.1; the protocol shape is
    // identical.
    // ─────────────────────────────────────────────────────────────────

    fn pinned_keys() -> ([u8; 32], [u8; 32], [u8; 16]) {
        let mut sk = [0u8; 32];
        for b in &mut sk {
            *b = 0x11;
        }
        let mut nh = [0u8; 32];
        for b in &mut nh {
            *b = 0x33;
        }
        let mut rc = [0u8; 16];
        for b in &mut rc {
            *b = 0x66;
        }
        (sk, nh, rc)
    }

    /// Build a CAPS frame whose hive_id_bytes is the all-0x55 fixture
    /// from r2-usb-pair-vectors.json. The peripheral's CAPS would
    /// contain the dongle's actual eFuse-MAC-derived hive_id_bytes; for
    /// tests we hand-pin to match the vector file.
    fn caps_frame_with_fixture_hive_id_bytes() -> Vec<u8> {
        // {0: bstr(16) hive_id_bytes=0x55*16, 1: "fix", 2: 1, 3: [{0:0, 1:1}]}
        let mut body = Vec::new();
        body.push(0xFE); // type = CAPS
        body.push(0xA4); // map(4)
        body.push(0x00);
        body.push(0x50); // bstr(16)
        body.extend_from_slice(&[0x55u8; 16]);
        body.push(0x01);
        body.push(0x63); // tstr(3)
        body.extend_from_slice(b"fix");
        body.push(0x02);
        body.push(0x01); // uint 1
        body.push(0x03);
        body.push(0x81); // array(1)
        body.push(0xA2); // map(2)
        body.push(0x00);
        body.push(0x00); // local_id = 0
        body.push(0x01);
        body.push(0x01); // kind = 1 (lora)

        let len = (body.len() as u16).to_le_bytes();
        let mut frame = Vec::new();
        frame.extend_from_slice(&len);
        frame.extend_from_slice(&body);
        frame
    }

    /// Walk a session up through SYNC negotiation. Returns the bytes
    /// the host emitted (SYNC frame) which the test discards.
    fn sync_v2(s: &mut UsbSession) {
        s.send_sync();
        let _ = s.take_outbound();
        let _ = s.ingest_bytes(&hex("040032520200"));
    }

    #[test]
    fn pairing_unknown_device_emits_pair_hello_host() {
        let (sk, nh, rc) = pinned_keys();
        let mut s = UsbSession::new();
        s.inject_test_keys(sk, nh, rc);
        sync_v2(&mut s);

        // Peripheral's CAPS arrives — unknown hive_id_bytes → first-attach.
        let _ = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        assert_eq!(s.state(), SessionState::PairingHelloSent);

        // Outbound bytes: PAIR_HELLO_HOST control frame with our
        // pinned ephemeral PK + nonce. The control-frame body (after
        // type byte) is `{0: 4, 1: {1: bstr/32 eph_pk_host, 2: bstr/32 nonce_host}}`.
        let out = s.take_outbound();
        // Body must include the pinned host PK derived from sk=0x11*32:
        let pk_host_pinned =
            hex("7b4e909bbe7ffe44c465a220037d608ee35897d31ef972f07f74892cb0f73f13");
        assert!(
            window_contains(&out, &pk_host_pinned),
            "outbound bytes missing pinned eph_pk_host"
        );
    }

    #[test]
    fn full_pairing_handshake_byte_equals_pinned_vectors() {
        let (sk, nh, rc) = pinned_keys();
        let mut s = UsbSession::new();
        s.inject_test_keys(sk, nh, rc);
        sync_v2(&mut s);

        // 1. CAPS in (unknown device) → host emits PAIR_HELLO_HOST.
        let evs = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        let _ = s.take_outbound();
        assert!(evs.iter().any(|e| matches!(e, UsbEvent::Caps(_))));

        // 2. Peripheral's PAIR_COMMIT — pinned commit_p value.
        // Frame: length-prefix(2) || 0xFF || cbor({0:5, 1:{1: bstr/32 commit_p}})
        let commit_p = hex("63036b4d1ce9e73c19dfbcdd3238cada9ae44f3186a2139b7ecf47aa0f41625e");
        let pair_commit = build_pair_msg(5, &[(1u64, CborField::Bytes(&commit_p))]);
        let frame = encode_length_prefixed(&pair_commit);
        let evs = s.ingest_bytes(&frame);
        assert!(evs.is_empty(), "PAIR_COMMIT shouldn't surface events");
        assert_eq!(s.state(), SessionState::PairingCommitReceived);

        // 3. Peripheral's PAIR_REVEAL — pinned eph_pk_periph + nonce_periph.
        let pk_periph =
            hex("0faa684ed28867b97f4a6a2dee5df8ce974e76b7018e3f22a1c4cf2678570f20");
        let nonce_periph = vec![0x44u8; 32];
        let pair_reveal = build_pair_msg(
            6,
            &[
                (1u64, CborField::Bytes(&pk_periph)),
                (2u64, CborField::Bytes(&nonce_periph)),
            ],
        );
        let frame = encode_length_prefixed(&pair_reveal);
        let evs = s.ingest_bytes(&frame);
        assert_eq!(s.state(), SessionState::PairingAwaitingUser);
        let prompt = evs
            .into_iter()
            .find_map(|e| match e {
                UsbEvent::PairingPrompt {
                    sas_code,
                    hive_id_bytes,
                    firmware_id,
                } => Some((sas_code, hive_id_bytes, firmware_id)),
                _ => None,
            })
            .expect("PairingPrompt emitted");
        assert_eq!(
            prompt.0, 488_092,
            "SAS code must match pinned vector value"
        );
        assert_eq!(prompt.1, [0x55u8; 16]);
        assert_eq!(prompt.2, "fix");

        // 4. User confirms — host emits PAIR_CONFIRM.
        s.user_confirms();
        let _ = s.take_outbound();
        assert_eq!(s.state(), SessionState::PairingConfirmSent);

        // 5. Peripheral's PAIR_DONE — host stores link_key and
        //    transitions to Active.
        let pair_done = build_pair_msg(8, &[]);
        let frame = encode_length_prefixed(&pair_done);
        let evs = s.ingest_bytes(&frame);
        assert!(evs.iter().any(|e| matches!(
            e,
            UsbEvent::Paired { reconnect: false, .. }
        )));
        assert_eq!(s.state(), SessionState::Active);
    }

    #[test]
    fn reconnect_with_known_hive_id_bytes_succeeds() {
        let (sk, nh, rc) = pinned_keys();
        let store = Arc::new(InMemoryLinkKeyStore::new());
        // Pre-populate the store with the pinned link_key for the
        // fixture hive_id_bytes, simulating a prior successful pairing.
        let hive_id_bytes = [0x55u8; 16];
        let link_key_bytes =
            hex("386667c282a123f2847ef829386561bbebe5d02f2132ffe96a9f40d2c31c43cb");
        let mut lk = [0u8; 32];
        lk.copy_from_slice(&link_key_bytes);
        store.store(&hive_id_bytes, &lk);

        let mut s = UsbSession::with_link_key_store(store);
        s.inject_test_keys(sk, nh, rc);
        sync_v2(&mut s);

        // CAPS in → known hive_id_bytes → host emits RECONNECT_CHALLENGE.
        let _ = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        let _ = s.take_outbound();
        assert_eq!(s.state(), SessionState::Reconnecting);

        // Peripheral computes the pinned reconnect tag.
        let tag = hex("2f62edaaa469424d5a5da5630b06967b");
        let resp = build_pair_msg(10, &[(1u64, CborField::Bytes(&tag))]);
        let frame = encode_length_prefixed(&resp);
        let evs = s.ingest_bytes(&frame);
        assert!(evs.iter().any(|e| matches!(
            e,
            UsbEvent::Paired { reconnect: true, .. }
        )));
        assert_eq!(s.state(), SessionState::Active);
    }

    #[test]
    fn reconnect_with_wrong_tag_fails_session() {
        let (sk, nh, rc) = pinned_keys();
        let store = Arc::new(InMemoryLinkKeyStore::new());
        store.store(&[0x55u8; 16], &[0u8; 32]); // wrong link key
        let mut s = UsbSession::with_link_key_store(store);
        s.inject_test_keys(sk, nh, rc);
        sync_v2(&mut s);

        let _ = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        let _ = s.take_outbound();

        // Peripheral sends a tag computed against the pinned link key,
        // but the host has the wrong one stored → fails.
        let tag = hex("2f62edaaa469424d5a5da5630b06967b");
        let resp = build_pair_msg(10, &[(1u64, CborField::Bytes(&tag))]);
        let frame = encode_length_prefixed(&resp);
        let evs = s.ingest_bytes(&frame);
        assert!(evs.iter().any(|e| matches!(
            e,
            UsbEvent::PairingFailed { reason }
                if reason == "reconnect_failed"
        )));
        assert_eq!(s.state(), SessionState::Closed);
    }

    #[test]
    fn pair_abort_from_peripheral_fails_session() {
        let (sk, nh, rc) = pinned_keys();
        let mut s = UsbSession::new();
        s.inject_test_keys(sk, nh, rc);
        sync_v2(&mut s);
        let _ = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        let _ = s.take_outbound();

        // Peripheral aborts pairing.
        let abort = build_pair_msg(11, &[(1u64, CborField::Text("user_aborted"))]);
        let frame = encode_length_prefixed(&abort);
        let evs = s.ingest_bytes(&frame);
        assert!(evs.iter().any(|e| matches!(
            e,
            UsbEvent::PairingFailed { reason } if reason == "user_aborted"
        )));
        assert_eq!(s.state(), SessionState::Closed);
    }

    #[test]
    fn commit_mismatch_aborts_with_correct_reason() {
        let (sk, nh, rc) = pinned_keys();
        let mut s = UsbSession::new();
        s.inject_test_keys(sk, nh, rc);
        sync_v2(&mut s);
        let _ = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        let _ = s.take_outbound();

        // Peripheral commits to one value but reveals different inputs.
        let bogus_commit = vec![0xAAu8; 32];
        let pair_commit = build_pair_msg(5, &[(1u64, CborField::Bytes(&bogus_commit))]);
        let _ = s.ingest_bytes(&encode_length_prefixed(&pair_commit));
        let _ = s.take_outbound();

        let pk_periph =
            hex("0faa684ed28867b97f4a6a2dee5df8ce974e76b7018e3f22a1c4cf2678570f20");
        let nonce_periph = vec![0x44u8; 32];
        let pair_reveal = build_pair_msg(
            6,
            &[
                (1u64, CborField::Bytes(&pk_periph)),
                (2u64, CborField::Bytes(&nonce_periph)),
            ],
        );
        let evs = s.ingest_bytes(&encode_length_prefixed(&pair_reveal));
        assert!(evs.iter().any(|e| matches!(
            e,
            UsbEvent::PairingFailed { reason } if reason == "commit_mismatch"
        )));
        assert_eq!(s.state(), SessionState::Closed);
        // Outbound bytes should also include a PAIR_ABORT
        // {reason: "commit_mismatch"} frame the host sent before closing.
        let out = s.take_outbound();
        assert!(window_contains(&out, b"commit_mismatch"));
    }

    // ─────────────────────────────────────────────────────────────────
    // Phase USB-5 — host-driven outbound wire frames through the dongle.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn send_wire_frame_rejects_pre_active_state() {
        let mut s = UsbSession::new_auto_pair_unsafe();
        // Initial state — should reject.
        assert!(!s.send_wire_frame(0, b"hello"));
    }

    #[test]
    fn send_wire_frame_in_active_pushes_typed_frame() {
        let mut s = UsbSession::new_auto_pair_unsafe();
        sync_v2(&mut s);
        let _ = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        let _ = s.take_outbound();
        assert_eq!(s.state(), SessionState::Active);

        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        assert!(s.send_wire_frame(2, &payload));
        let out = s.take_outbound();
        // Frame layout: length(2 LE) || type=0x02 || payload(4)
        assert_eq!(out, hex("050002deadbeef"));
    }

    #[test]
    fn send_wire_frame_rejects_reserved_local_id() {
        let mut s = UsbSession::new_auto_pair_unsafe();
        sync_v2(&mut s);
        let _ = s.ingest_bytes(&caps_frame_with_fixture_hive_id_bytes());
        let _ = s.take_outbound();
        // 0xFC, 0xFD reserved per R2-USB §3.5; 0xFE = CAPS, 0xFF = control.
        assert!(!s.send_wire_frame(0xFC, b"x"));
        assert!(!s.send_wire_frame(0xFE, b"x"));
        assert!(!s.send_wire_frame(0xFF, b"x"));
    }

    fn window_contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }
}
