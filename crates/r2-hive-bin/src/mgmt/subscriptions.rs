//! Subscription registry for the R2-HOST-API primitive surface.
//!
//! See R2-HOST-API §4. A connection registers a subscription via
//! `r2.api.event.subscribe` and receives matching events as
//! `r2.api.event.delivery` notifications — and §7.5.4 deliver-gate rejects as
//! `r2.api.event.delivery.denied` (R2-HOST-API §3.2.1). Subscriptions are torn
//! down when the connection closes.
//!
//! Channel-isolation guidance: an unfiltered subscription receives BOTH
//! deliveries and denies on one bounded channel (a forged-frame flood can
//! crowd deliveries out). Deny consumers should subscribe filtered to the
//! denied class; delivery consumers should filter by their own event class.
//!
//! Phase 1 ships the registry skeleton with the data model and ID allocator;
//! the actual delivery wiring lands when Phase 1 hooks into HiveState's
//! inbound frame paths.
//!
//! ## Interlinks + canon
//!
//! One registry per mgmt connection, created in
//! `HiveState::register_subscriber` (called by `socket.rs` and `ws.rs`);
//! mutated by the subscribe/unsubscribe handlers in `primitive.rs`; read by
//! `HiveState::{deliver_inbound, deny_inbound}` when fanning notifications
//! out. Canon: R2-HOST-API §4 (subscription mechanics), §3.2/§3.2.1
//! (delivery + denied), §5.2 (high-bit synthetic ids) —
//! `r2-specifications/specs/r2-core/R2-HOST-API.md`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

/// A subscription's filter — see R2-HOST-API §3.2 (event.subscribe payload
/// keys 1–4).
#[derive(Debug, Clone, Default)]
pub struct SubscriptionFilter {
    /// OPTIONAL exact match against incoming event class string (canonical form).
    pub event_class: Option<String>,
    /// OPTIONAL exact match against incoming event hash. Mutually exclusive
    /// with `event_class` per §3.2.
    pub event_hash: Option<u32>,
    /// OPTIONAL only match events from this hive_id.
    pub from_hive: Option<u64>,
    /// OPTIONAL only match events from this TG (8-byte TG-hash). v0.1
    /// default = active TG.
    pub from_tg: Option<[u8; 8]>,
}

impl SubscriptionFilter {
    /// True iff this filter has no match keys (broadcast subscription per §3.2).
    pub fn is_broadcast(&self) -> bool {
        self.event_class.is_none()
            && self.event_hash.is_none()
            && self.from_hive.is_none()
            && self.from_tg.is_none()
    }
}

/// One active subscription on one connection.
///
/// Most subscriptions are passive observers (created by
/// `r2.api.event.subscribe`). When `service_class` is `Some`, the entry
/// is a *service-sentant registration* (R2-PLUGIN §5, R2-HOST-API §5.2):
/// the connection claims to *handle* events of that class on this hive,
/// not merely to observe them. Service registrations carry the same
/// fanout machinery as subscriptions but their IDs are allocated from
/// the reserved high-bit (`& 0x8000_0000 != 0`) ID space so callers can
/// distinguish them.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub sub_id: u32,
    pub filter: SubscriptionFilter,
    /// `Some(class)` iff this entry is a service-sentant registration.
    pub service_class: Option<String>,
}

/// Per-connection subscription registry. A connection holds zero or more
/// subscriptions; each is identified by an ID unique within the connection.
/// Subscription IDs are released when the connection closes (the registry
/// goes out of scope with the connection).
pub struct SubscriptionRegistry {
    next_id: AtomicU32,
    subs: HashMap<u32, Subscription>,
}

impl SubscriptionRegistry {
    /// Empty per-connection registry.
    ///
    /// **Used-by:** `HiveState::register_subscriber` (one per mgmt
    /// connection, UDS and WS alike).
    pub fn new() -> Self {
        Self {
            // start at 1 so we can reserve high-bit IDs for synthetic
            // service-sentant deliveries per R2-HOST-API §5.2.
            next_id: AtomicU32::new(1),
            subs: HashMap::new(),
        }
    }

    /// Register a new subscription, returning its sub_id.
    pub fn add(&mut self, filter: SubscriptionFilter) -> u32 {
        let sub_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // Mask off the high bit (reserved for service-sentant deliveries).
        let sub_id = sub_id & 0x7FFF_FFFF;
        self.subs.insert(
            sub_id,
            Subscription {
                sub_id,
                filter,
                service_class: None,
            },
        );
        sub_id
    }

    /// Register a service-sentant for the given event class. Returns a
    /// service_id with the high bit set (R2-HOST-API §5.2).
    pub fn add_service(&mut self, class: &str, event_hash: u32) -> u32 {
        let raw = self.next_id.fetch_add(1, Ordering::Relaxed);
        let service_id = (raw & 0x7FFF_FFFF) | 0x8000_0000;
        let filter = SubscriptionFilter {
            event_class: Some(class.to_string()),
            event_hash: Some(event_hash),
            from_hive: None,
            from_tg: None,
        };
        self.subs.insert(
            service_id,
            Subscription {
                sub_id: service_id,
                filter,
                service_class: Some(class.to_string()),
            },
        );
        service_id
    }

    /// Remove a subscription or service registration by ID. Returns
    /// `true` if an entry was removed, `false` if the ID didn't match
    /// anything.
    pub fn remove(&mut self, sub_id: u32) -> bool {
        self.subs.remove(&sub_id).is_some()
    }

    /// Iterate over service registrations only (entries where
    /// `service_class` is set).
    pub fn services(&self) -> impl Iterator<Item = &Subscription> {
        self.subs.values().filter(|s| s.service_class.is_some())
    }

    /// Iterate over all active subscriptions.
    pub fn iter(&self) -> impl Iterator<Item = &Subscription> {
        self.subs.values()
    }

    /// Number of active subscriptions.
    pub fn len(&self) -> usize {
        self.subs.len()
    }

    /// True if there are no active subscriptions.
    pub fn is_empty(&self) -> bool {
        self.subs.is_empty()
    }
}

impl Default for SubscriptionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove_subscription() {
        let mut reg = SubscriptionRegistry::new();
        assert!(reg.is_empty());

        let id = reg.add(SubscriptionFilter {
            event_class: Some("org.example.ping".to_string()),
            ..Default::default()
        });
        assert_eq!(reg.len(), 1);

        // Subscription IDs are non-zero and never have the high bit set.
        assert!(id != 0);
        assert!(id & 0x8000_0000 == 0);

        assert!(reg.remove(id));
        assert!(reg.is_empty());

        // Removing again returns false.
        assert!(!reg.remove(id));
    }

    #[test]
    fn broadcast_filter_detection() {
        let f = SubscriptionFilter::default();
        assert!(f.is_broadcast());

        let f = SubscriptionFilter {
            event_class: Some("foo".into()),
            ..Default::default()
        };
        assert!(!f.is_broadcast());

        let f = SubscriptionFilter {
            from_hive: Some(0xCAFE),
            ..Default::default()
        };
        assert!(!f.is_broadcast());
    }

    #[test]
    fn unique_ids() {
        let mut reg = SubscriptionRegistry::new();
        let id1 = reg.add(SubscriptionFilter::default());
        let id2 = reg.add(SubscriptionFilter::default());
        let id3 = reg.add(SubscriptionFilter::default());
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }
}
