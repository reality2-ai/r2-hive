//! Record store seam — the storing-backend hive's durable **record-of-truth**
//! (Roy-commissioned; see `docs/storing-backend-hive-scoping.md`).
//!
//! Modeled on the [`crate::identity`] storage seam: the trait + platform-neutral
//! types live here (`no_std` + `alloc`), and each platform supplies a concrete
//! impl in its layer — SQLite/Postgres on the always-on Linux backend hive, an
//! `InMemoryRecordStore` for tests, IndexedDB for a wasm hive. hive logic targets
//! the trait, never a concrete store.
//!
//! **Design — append-only log is the source of truth.** A business
//! record-of-truth must survive every client being offline and must never lose a
//! concurrent edit (plain last-write-wins does). So the durable substrate is an
//! **append-only event log** ([`StoredEvent`]); the current materialized record
//! is a *projection* of that log. This is also the **audit trail**: every
//! mutation is one immutable log entry carrying who/what/when. ("A sync peer that
//! never forgets" — core; "append the op-stream as a log" rather than only
//! materializing LWW.)
//!
//! **Scope boundary (supervisor GO, spec-first).** This module is the
//! *structural* seam only. The fields for attribution ([`Actor`]) and the log
//! shape are here, but the **authority / audit / access-scope ENFORCEMENT
//! semantics are spec-gated** — they wire in once specs ratifies the canon
//! (write-authority, `canSee`, proposal accept/reject rules). Nothing here
//! enforces; it only records.

use alloc::string::String;
use alloc::vec::Vec;

/// Monotonic, store-assigned sequence number for a log entry. Starts at 1; `0`
/// means "before the first entry" (use as a catch-up cursor).
pub type Seq = u64;

/// Whether a mutation was made by a human or an autonomous agent. Attribution is
/// recorded structurally now; the authority semantics it feeds are spec-gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    Human,
    Agent,
}

/// The proven actor behind a mutation. `id` is an opaque identity string (e.g. a
/// DEV_PK hex or a person id resolved from TG membership). The store persists it
/// so "who authorized this" is recoverable — the gap composer flagged (identity
/// proven at ingress but dropped from the audit row).
#[derive(Debug, Clone)]
pub struct Actor {
    pub id: String,
    pub kind: ActorKind,
}

/// A mutation to append to the durable log. The store assigns the [`Seq`].
#[derive(Debug, Clone)]
pub struct NewRecordEvent {
    /// Wall-clock seconds (from the platform clock) when the mutation occurred.
    pub timestamp: u64,
    /// The record/entity id this mutation targets.
    pub entity: String,
    /// The mutation verb — e.g. `create` / `update` / `delete` / `propose` /
    /// `apply`. Free-form at the seam; canon will pin the vocabulary.
    pub action: String,
    /// r2-fnv hash of the originating R2 event name (links the log row back to
    /// the event that produced it).
    pub event_hash: u32,
    /// The proven writer, if known. `None` until the actor-attribution path is
    /// wired (spec-gated); present-but-unenforced is the structural default.
    pub actor: Option<Actor>,
    /// Optional client-supplied idempotency key. When set, a second append with
    /// the same `op_id` MUST NOT create a duplicate entry — the store returns the
    /// already-assigned [`Seq`]. (Directly serves exactly-once-at-dispatch — the
    /// TN-L2-IT-AB-000 ruling: identity dedup, not a wall-clock window.)
    pub op_id: Option<String>,
    /// The op payload — typically the CBOR typed-op diff / record body.
    pub payload: Vec<u8>,
}

/// A persisted log entry: a [`NewRecordEvent`] with its assigned [`Seq`].
#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub seq: Seq,
    pub timestamp: u64,
    pub entity: String,
    pub action: String,
    pub event_hash: u32,
    pub actor: Option<Actor>,
    pub op_id: Option<String>,
    pub payload: Vec<u8>,
}

/// Platform-neutral error for [`RecordStore`] operations. Backends map their
/// native errors (rusqlite, io, etc.) into these so hive logic stays
/// platform-agnostic (mirrors [`crate::identity::StoreError`]).
#[derive(Debug)]
pub enum RecordError {
    /// The requested entity / sequence does not exist.
    NotFound,
    /// The mutation conflicts with store invariants (e.g. an authority/version
    /// rule once those land). Carries a human-readable reason.
    Conflict(String),
    /// Any backend failure (IO, db, serialization). Human-readable description.
    Backend(String),
}

impl core::fmt::Display for RecordError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RecordError::NotFound => write!(f, "record store: not found"),
            RecordError::Conflict(s) => write!(f, "record store: conflict: {s}"),
            RecordError::Backend(s) => write!(f, "record store: {s}"),
        }
    }
}

/// The durable record-of-truth seam. Implementations use interior mutability
/// (`&self`) so the store can be shared by the persistence sentant and readers
/// — same convention as [`crate::identity::IdentityStore`].
///
/// The append-only log is authoritative; [`get`](RecordStore::get) returns the
/// current projection. Enforcement (authority/scope) is NOT part of this
/// contract yet — it layers on once canon ratifies.
pub trait RecordStore {
    /// Append a mutation to the log, returning its assigned monotonic [`Seq`].
    /// If `event.op_id` is `Some` and already seen, returns the existing seq
    /// (idempotent) without appending a duplicate.
    fn append(&self, event: NewRecordEvent) -> Result<Seq, RecordError>;

    /// The current materialized payload for `entity` — the latest non-deleted
    /// write — or `None` if absent/deleted.
    fn get(&self, entity: &str) -> Result<Option<Vec<u8>>, RecordError>;

    /// Replay the log strictly after `since` (use `0` for the whole log) — for
    /// catch-up, projection rebuild, and audit export. Entries are in seq order.
    fn log_since(&self, since: Seq) -> Result<Vec<StoredEvent>, RecordError>;

    /// The highest assigned [`Seq`] (`0` if the log is empty) — a catch-up
    /// cursor for clients reconnecting after being offline.
    fn head_seq(&self) -> Result<Seq, RecordError>;
}

// ───────────────────────── Tests ─────────────────────────
//
// A RAM-backed reference impl exercises the seam standalone (core links `std`
// under test). It models the durable shape — append-only log authoritative,
// `get` a projection, op_id idempotency — that the platform-layer SQLite backend
// will mirror with crash-safe storage. The production `InMemoryRecordStore` (pub,
// for the persistence ensemble) lands in the bin alongside the SQLite impl.

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use std::collections::HashMap;
    use std::string::ToString;
    use std::sync::RwLock;

    const ACTION_DELETE: &str = "delete";

    struct InMemoryRecordStore {
        inner: RwLock<Inner>,
    }
    struct Inner {
        log: Vec<StoredEvent>,
        projection: HashMap<String, usize>,
        by_op: HashMap<String, Seq>,
    }
    impl InMemoryRecordStore {
        fn new() -> Self {
            Self {
                inner: RwLock::new(Inner {
                    log: Vec::new(),
                    projection: HashMap::new(),
                    by_op: HashMap::new(),
                }),
            }
        }
    }
    impl RecordStore for InMemoryRecordStore {
        fn append(&self, event: NewRecordEvent) -> Result<Seq, RecordError> {
            let mut g = self
                .inner
                .write()
                .map_err(|_| RecordError::Backend("lock poisoned".to_string()))?;
            if let Some(op_id) = &event.op_id {
                if let Some(&seq) = g.by_op.get(op_id) {
                    return Ok(seq); // idempotent
                }
            }
            let seq = g.log.len() as u64 + 1;
            let stored = StoredEvent {
                seq,
                timestamp: event.timestamp,
                entity: event.entity,
                action: event.action,
                event_hash: event.event_hash,
                actor: event.actor,
                op_id: event.op_id,
                payload: event.payload,
            };
            let idx = g.log.len();
            g.projection.insert(stored.entity.clone(), idx);
            if let Some(op_id) = &stored.op_id {
                g.by_op.insert(op_id.clone(), seq);
            }
            g.log.push(stored);
            Ok(seq)
        }
        fn get(&self, entity: &str) -> Result<Option<Vec<u8>>, RecordError> {
            let g = self
                .inner
                .read()
                .map_err(|_| RecordError::Backend("lock poisoned".to_string()))?;
            match g.projection.get(entity) {
                Some(&idx) => {
                    let e = &g.log[idx];
                    if e.action == ACTION_DELETE {
                        Ok(None)
                    } else {
                        Ok(Some(e.payload.clone()))
                    }
                }
                None => Ok(None),
            }
        }
        fn log_since(&self, since: Seq) -> Result<Vec<StoredEvent>, RecordError> {
            let g = self
                .inner
                .read()
                .map_err(|_| RecordError::Backend("lock poisoned".to_string()))?;
            Ok(g.log.iter().filter(|e| e.seq > since).cloned().collect())
        }
        fn head_seq(&self) -> Result<Seq, RecordError> {
            let g = self
                .inner
                .read()
                .map_err(|_| RecordError::Backend("lock poisoned".to_string()))?;
            Ok(g.log.len() as u64)
        }
    }

    fn ev(entity: &str, action: &str, payload: &[u8]) -> NewRecordEvent {
        NewRecordEvent {
            timestamp: 1_700_000_000,
            entity: entity.to_string(),
            action: action.to_string(),
            event_hash: 0xABCD_1234,
            actor: None,
            op_id: None,
            payload: payload.to_vec(),
        }
    }

    #[test]
    fn append_assigns_monotonic_seq_and_tracks_head() {
        let s = InMemoryRecordStore::new();
        assert_eq!(s.head_seq().unwrap(), 0);
        assert_eq!(s.append(ev("rec-1", "create", b"a")).unwrap(), 1);
        assert_eq!(s.append(ev("rec-2", "create", b"b")).unwrap(), 2);
        assert_eq!(s.head_seq().unwrap(), 2);
    }

    #[test]
    fn get_returns_latest_write() {
        let s = InMemoryRecordStore::new();
        s.append(ev("rec", "create", b"v1")).unwrap();
        assert_eq!(s.get("rec").unwrap().as_deref(), Some(&b"v1"[..]));
        s.append(ev("rec", "update", b"v2")).unwrap();
        assert_eq!(s.get("rec").unwrap().as_deref(), Some(&b"v2"[..]));
        assert_eq!(s.get("absent").unwrap(), None);
    }

    #[test]
    fn delete_tombstones_projection_but_keeps_log() {
        let s = InMemoryRecordStore::new();
        s.append(ev("rec", "create", b"v1")).unwrap();
        s.append(ev("rec", "delete", b"")).unwrap();
        assert_eq!(s.get("rec").unwrap(), None); // projection tombstoned
        assert_eq!(s.log_since(0).unwrap().len(), 2); // log retains both
    }

    #[test]
    fn op_id_makes_append_idempotent() {
        let s = InMemoryRecordStore::new();
        let mut a = ev("rec", "create", b"v1");
        a.op_id = Some("op-42".to_string());
        let mut a2 = ev("rec", "create", b"v1");
        a2.op_id = Some("op-42".to_string());
        let seq1 = s.append(a).unwrap();
        let seq2 = s.append(a2).unwrap();
        assert_eq!(seq1, seq2, "same op_id ⇒ same seq");
        assert_eq!(s.head_seq().unwrap(), 1, "no duplicate appended");
    }

    #[test]
    fn log_since_replays_from_cursor() {
        let s = InMemoryRecordStore::new();
        s.append(ev("a", "create", b"1")).unwrap();
        s.append(ev("b", "create", b"2")).unwrap();
        s.append(ev("c", "create", b"3")).unwrap();
        let tail = s.log_since(1).unwrap();
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].seq, 2);
        assert_eq!(tail[1].seq, 3);
    }

    #[test]
    fn actor_attribution_is_recorded_structurally() {
        let s = InMemoryRecordStore::new();
        let mut a = ev("rec", "create", b"v1");
        a.actor = Some(Actor {
            id: "dev-pk-abc".to_string(),
            kind: ActorKind::Agent,
        });
        s.append(a).unwrap();
        let entry = &s.log_since(0).unwrap()[0];
        let actor = entry.actor.as_ref().expect("actor persisted");
        assert_eq!(actor.id, "dev-pk-abc");
        assert_eq!(actor.kind, ActorKind::Agent);
    }

    #[test]
    fn usable_as_trait_object() {
        let store: Box<dyn RecordStore> = Box::new(InMemoryRecordStore::new());
        store.append(ev("rec", "create", b"x")).unwrap();
        assert_eq!(store.get("rec").unwrap().as_deref(), Some(&b"x"[..]));
    }
}
