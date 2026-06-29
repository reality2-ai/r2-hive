//! Transport integration — hive's multi-transport send path over the REAL
//! r2-discovery UDP-LAN sockets (core D3a, `52b0e4e`).
//!
//! Unlike core's own `udp_lan` unit tests (which exercise the transport in
//! isolation), these drive the wiring hive owns: `HiveState::set_udp_transport`
//! + the `send_to_hive_via` fallback chain (Internet→Wifi→Ble) actually putting
//! bytes on a real socket that a second transport receives. This converts
//! "compiles against core's real API" into "proven to round-trip".
//!
//! UDP-LAN is reached through the `Wifi` transport slot (Internet = WebSocket),
//! so a successful UDP send reports `Transport::Wifi`.

use std::sync::Arc;

use r2_discovery::bindings::udp_lan::UdpLanTransport;
use r2_discovery::{AsyncTransport, LinkQuality, PeerMap};
use r2_hive::hive::HiveState;
use r2_route::transport::{Transport, TransportSet};

const ID_A: u32 = 0x0000_00AA;
const ID_B: u32 = 0x0000_00BB;

/// A `HiveState` with a registered UDP-LAN transport sends a frame that a
/// second, independent UDP-LAN transport receives — over real loopback
/// sockets — and the route engine reports it went out over `Wifi`.
#[tokio::test]
async fn hive_send_round_trips_over_real_udp_lan() {
    // Two real transports on ephemeral loopback ports.
    let udp_a = Arc::new(UdpLanTransport::bind("127.0.0.1:0").await.expect("bind A"));
    let udp_b = Arc::new(UdpLanTransport::bind("127.0.0.1:0").await.expect("bind B"));

    // Cross-register addresses (in production this is fed by R2-DISCOVERY §3
    // resolution): A must know where B is to send; B must know A to reverse-
    // resolve the datagram source to A's canonical hive id.
    udp_a
        .peers()
        .add_peer(ID_B, udp_b.local_addr().to_string(), LinkQuality::default());
    udp_b
        .peers()
        .add_peer(ID_A, udp_a.local_addr().to_string(), LinkQuality::default());

    // Hive A owns udp_a via the same registration path main.rs uses for --lan.
    let state = HiveState::new(ID_A, 64, 16);
    state.set_udp_transport(udp_a.clone()).await;

    // Send through the multi-transport fallback. WS has no peers, so it falls
    // through to the Wifi (UDP-LAN) slot.
    let frame = b"r2-wire-frame-over-udp";
    let used = state.send_to_hive_via(ID_B, None, frame).await;
    assert_eq!(
        used,
        Some(Transport::Wifi),
        "send should succeed over the UDP-LAN (Wifi) transport"
    );

    // The peer receives the exact bytes, with the source resolved to A's id.
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), udp_b.recv())
        .await
        .expect("recv did not time out")
        .expect("recv ok");
    assert_eq!(got.data, frame, "peer receives the frame verbatim");
    assert_eq!(
        got.source_hive, ID_A,
        "reverse lookup resolves the datagram source to A's canonical id"
    );
}

/// With no transports able to reach the destination (no WS peers, no UDP/BLE
/// transport set), the fallback chain exhausts cleanly and reports failure
/// rather than hanging or panicking.
#[tokio::test]
async fn hive_send_with_no_transports_returns_none() {
    let state = HiveState::new(ID_A, 64, 16);
    let used = state.send_to_hive_via(ID_B, None, b"frame").await;
    assert_eq!(used, None, "no reachable transport ⇒ send reports failure");
}

/// A `Wifi` hint still resolves to the UDP-LAN transport: the hint is honoured
/// first in the attempt order, and the frame still lands on the peer socket.
#[tokio::test]
async fn wifi_hint_routes_over_udp_lan() {
    let udp_a = Arc::new(UdpLanTransport::bind("127.0.0.1:0").await.expect("bind A"));
    let udp_b = Arc::new(UdpLanTransport::bind("127.0.0.1:0").await.expect("bind B"));
    udp_a
        .peers()
        .add_peer(ID_B, udp_b.local_addr().to_string(), LinkQuality::default());

    let state = HiveState::new(ID_A, 64, 16);
    state.set_udp_transport(udp_a.clone()).await;

    let used = state
        .send_to_hive_via(ID_B, Some(Transport::Wifi), b"hinted")
        .await;
    assert_eq!(used, Some(Transport::Wifi));

    let got = tokio::time::timeout(std::time::Duration::from_secs(2), udp_b.recv())
        .await
        .expect("recv did not time out")
        .expect("recv ok");
    assert_eq!(got.data, b"hinted");
}

/// The host fallback path is not allowed to bypass the node-local
/// `transport_allow_mask`. This covers locally originated sends that do not go
/// through `RouteEngine::plan_forward` first.
#[tokio::test]
async fn transport_allow_mask_filters_host_send_before_physical_egress() {
    let udp_a = Arc::new(UdpLanTransport::bind("127.0.0.1:0").await.expect("bind A"));
    let udp_b = Arc::new(UdpLanTransport::bind("127.0.0.1:0").await.expect("bind B"));
    udp_a
        .peers()
        .add_peer(ID_B, udp_b.local_addr().to_string(), LinkQuality::default());

    let state = HiveState::new(ID_A, 64, 16);
    state.set_udp_transport(udp_a.clone()).await;

    let ack = state
        .set_transport_policy_lease(77, "transport-integration-test".to_string(), Transport::Internet.bit())
        .await;
    assert_eq!(ack.requested_mask, Transport::Internet.bit());
    assert_eq!(ack.accepted_mask, Transport::Internet.bit());

    let used = state
        .send_to_hive_via(ID_B, Some(Transport::Wifi), b"masked")
        .await;
    assert_eq!(used, None, "disabled Wifi must not be used");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), udp_b.recv())
            .await
            .is_err(),
        "masked send should not reach the UDP-LAN socket"
    );

    let snapshot = state.clear_transport_policy().await;
    assert_eq!(snapshot.effective_mask, TransportSet::ALL_BITS);
    assert!(snapshot.active_lease.is_none());

    let used = state
        .send_to_hive_via(ID_B, Some(Transport::Wifi), b"unmasked")
        .await;
    assert_eq!(used, Some(Transport::Wifi));
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), udp_b.recv())
        .await
        .expect("recv did not time out")
        .expect("recv ok");
    assert_eq!(got.data, b"unmasked");
}
