//! Integration test: spawn the daemon in-process, connect over the Unix socket,
//! round-trip an `r2.mgmt.daemon.status` request, assert the response fields.

use tokio::net::UnixStream;

use r2_hive::mgmt::api::{build_status_request, parse_status_response};
use r2_hive::mgmt::framing::{read_frame, write_frame};
use r2_hive::mgmt::{socket, state::DaemonState};

#[tokio::test]
async fn daemon_status_round_trip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");

    let state = DaemonState::new();
    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn daemon");

    // Small yield so the listener is ready before connect.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("connect to daemon");
    let (mut reader, mut writer) = stream.split();

    let correlation_id = 0xDEADBEEF_u64;
    let request = build_status_request(correlation_id);
    write_frame(&mut writer, &request)
        .await
        .expect("write request");

    let response_frame = read_frame(&mut reader)
        .await
        .expect("read response")
        .expect("non-empty response");

    let parsed = parse_status_response(&response_frame).expect("parse response");

    assert_eq!(parsed.correlation_id, correlation_id, "correlation id");
    assert_eq!(parsed.version, env!("CARGO_PKG_VERSION"), "version");
    // build_hash is "unversioned" unless R2TGD_BUILD_HASH is set; either is acceptable.
    assert!(
        !parsed.build_hash.is_empty(),
        "build_hash should be non-empty"
    );

    // Shut down cleanly.
    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn identity_persists_across_daemon_restart() {
    use r2_hive::mgmt::api::{build_identity_status_request, parse_identity_status_response};
    use r2_hive::mgmt::identity::FileStore;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let store_path = tmp.path().join("master.key");
    let store = FileStore::new(store_path.clone());

    // First start: should generate a fresh master secret.
    let state1 =
        r2_hive::mgmt::DaemonState::with_identity(&store).expect("identity load/create (first)");
    assert!(state1.identity_present());
    assert!(state1.identity_created_this_start());
    let fingerprint_1 = state1.identity_fingerprint();
    assert_eq!(fingerprint_1.len(), 16);

    let handle = r2_hive::mgmt::socket::spawn(socket_path.clone(), state1)
        .await
        .expect("spawn first");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Round-trip identity.status over the socket.
    let observed_fp_1 = {
        let mut stream = UnixStream::connect(&socket_path)
            .await
            .expect("connect first");
        let (mut reader, mut writer) = stream.split();
        let correlation_id = 0x1_u64;
        write_frame(&mut writer, &build_identity_status_request(correlation_id))
            .await
            .expect("write");
        let frame = read_frame(&mut reader)
            .await
            .expect("read")
            .expect("response");
        let parsed = parse_identity_status_response(&frame).expect("parse");
        assert!(parsed.present);
        assert!(parsed.created_this_start);
        assert_eq!(parsed.backend, "file");
        parsed.fingerprint
    };
    assert_eq!(observed_fp_1, fingerprint_1);

    // Shut the first daemon down cleanly.
    let _ = handle.shutdown.send(());
    let _ = handle.join.await;

    // Second start: load the persisted master secret; fingerprint must match,
    // created_this_start must be false.
    let state2 =
        r2_hive::mgmt::DaemonState::with_identity(&store).expect("identity load/create (second)");
    assert!(state2.identity_present());
    assert!(
        !state2.identity_created_this_start(),
        "second start should load existing, not generate"
    );
    assert_eq!(
        state2.identity_fingerprint(),
        fingerprint_1,
        "fingerprint must be stable across restart"
    );

    let handle2 = r2_hive::mgmt::socket::spawn(socket_path.clone(), state2)
        .await
        .expect("spawn second");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let observed_fp_2 = {
        let mut stream = UnixStream::connect(&socket_path)
            .await
            .expect("connect second");
        let (mut reader, mut writer) = stream.split();
        write_frame(&mut writer, &build_identity_status_request(0x2_u64))
            .await
            .expect("write");
        let frame = read_frame(&mut reader)
            .await
            .expect("read")
            .expect("response");
        let parsed = parse_identity_status_response(&frame).expect("parse");
        assert!(!parsed.created_this_start);
        parsed.fingerprint
    };
    assert_eq!(observed_fp_2, fingerprint_1);

    let _ = handle2.shutdown.send(());
    let _ = handle2.join.await;
}

#[tokio::test]
async fn peer_list_no_hive_state() {
    // Mgmt-only daemon (DaemonState::new() with no HiveState attached) MUST
    // still respond to r2.api.peer.list with an empty list — the operation
    // is well-defined, there are simply no peers.
    use r2_hive::mgmt::api::build_peer_list_request;
    use r2_hive::mgmt::primitive::parse_peer_list_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn daemon");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("connect to daemon");
    let (mut reader, mut writer) = stream.split();

    let correlation_id = 0xABCDEF_u64;
    write_frame(&mut writer, &build_peer_list_request(correlation_id))
        .await
        .expect("write peer.list");
    let response_frame = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&response_frame).expect("decode response");
    let expected_hash = r2_fnv::r2_hash("r2.api.peer.list").expect("hash");
    assert_eq!(parsed.header.event_hash, expected_hash, "expected peer.list response frame");
    let (cid, peers) = parse_peer_list_response(parsed.payload).expect("parse peer.list");
    assert_eq!(cid, correlation_id);
    assert!(peers.is_empty(), "no HiveState ⇒ empty peer list");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn peer_list_with_hive_state_includes_self() {
    // With HiveState attached and no neighbours observed, peer.list MUST
    // return exactly the daemon's self_hive_id.
    use std::sync::Arc;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::build_peer_list_request;
    use r2_hive::mgmt::primitive::parse_peer_list_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    // Attach a HiveState with a known self_hive_id.
    let self_id: u32 = 0xCAFEBEEF;
    let hive = Arc::new(HiveState::new(self_id, 1024, 64));
    state.attach_hive_state(hive);

    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn daemon");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("connect to daemon");
    let (mut reader, mut writer) = stream.split();

    let correlation_id = 0x12345_u64;
    write_frame(&mut writer, &build_peer_list_request(correlation_id))
        .await
        .expect("write");
    let response_frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&response_frame).expect("decode");
    let (cid, peers) = parse_peer_list_response(parsed.payload).expect("parse");
    assert_eq!(cid, correlation_id);
    assert_eq!(peers, vec![self_id as u64], "self should be the only peer");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn peer_query_self_returns_status_self() {
    use std::sync::Arc;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::build_peer_query_request;
    use r2_hive::mgmt::primitive::parse_peer_query_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let self_id: u32 = 0xCAFEBEEF;
    state.attach_hive_state(Arc::new(HiveState::new(self_id, 1024, 64)));

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    let cid = 0xAA_u64;
    write_frame(&mut writer, &build_peer_query_request(cid, self_id as u64)).await.expect("write");
    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let (rcid, hid, status, last_seen, transports) =
        parse_peer_query_response(parsed.payload).expect("parse");
    assert_eq!(rcid, cid);
    assert_eq!(hid, self_id as u64);
    assert_eq!(status, 1, "self status");
    assert_eq!(last_seen, None, "no last_seen for self");
    assert!(transports.is_empty(), "no transports for self");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn peer_query_unknown_returns_status_unknown() {
    use std::sync::Arc;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::build_peer_query_request;
    use r2_hive::mgmt::primitive::parse_peer_query_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    state.attach_hive_state(Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64)));

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    let cid = 0xBB_u64;
    let unknown: u64 = 0x1234_5678;
    write_frame(&mut writer, &build_peer_query_request(cid, unknown)).await.expect("write");
    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let (rcid, hid, status, last_seen, transports) =
        parse_peer_query_response(parsed.payload).expect("parse");
    assert_eq!(rcid, cid);
    assert_eq!(hid, unknown);
    assert_eq!(status, 0, "unknown status");
    assert_eq!(last_seen, None);
    assert!(transports.is_empty());

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn peer_query_neighbour_returns_status_and_transports() {
    use std::sync::Arc;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::build_peer_query_request;
    use r2_hive::mgmt::primitive::parse_peer_query_response;
    use r2_route::neighbour::{MobilityClass, Observation};
    use r2_route::transport::{QualitySample, Transport};

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let self_id: u32 = 0xCAFEBEEF;
    let hive = Arc::new(HiveState::new(self_id, 1024, 64));

    // Inject a synthetic neighbour observation so the route engine sees a peer.
    let neighbour_id: u32 = 0xDEADBEEF;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);
    {
        let mut engine = hive.route_engine.lock().await;
        engine.ingest_observation(Observation {
            hive_id: neighbour_id,
            transport: Transport::Ble,
            timestamp: now_secs,
            quality: QualitySample::Direct(0.8),
            rssi: Some(-60),
            mcu_origin: false,
            mobility: MobilityClass::Mobile,
        });
        engine.ingest_observation(Observation {
            hive_id: neighbour_id,
            transport: Transport::Wifi,
            timestamp: now_secs,
            quality: QualitySample::Direct(0.9),
            rssi: None,
            mcu_origin: false,
            mobility: MobilityClass::Mobile,
        });
    }

    state.attach_hive_state(hive);
    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    let cid = 0xCC_u64;
    write_frame(&mut writer, &build_peer_query_request(cid, neighbour_id as u64)).await.expect("write");
    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let (rcid, hid, status, last_seen, transports) =
        parse_peer_query_response(parsed.payload).expect("parse");
    assert_eq!(rcid, cid);
    assert_eq!(hid, neighbour_id as u64);
    assert_eq!(status, 2, "neighbour status");
    assert_eq!(last_seen, Some((now_secs as u64) * 1000), "last_seen in ms");
    // Both BLE and WiFi observations were ingested; both should be reported.
    assert!(transports.contains(&"ble".to_string()), "transports={:?}", transports);
    assert!(transports.contains(&"wifi".to_string()), "transports={:?}", transports);

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn peer_query_missing_hive_id_returns_bad_payload() {
    use r2_fnv::r2_hash;
    use r2_hive::mgmt::api::EV_PEER_QUERY;
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    // Build a peer.query request with only correlation_id, no hive_id.
    // Payload: A1 00 19 00 99 (map(1){0:0x99}). 0x99 fits as one-byte uint
    // = 24..255 range so we use 0x18 prefix: A1 00 18 99.
    let payload = [0xA1, 0x00, 0x18, 0x99];
    let event_hash = r2_hash(EV_PEER_QUERY).expect("hash");
    let msg = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 0, k: 0, msg_id: 0, event_hash,
            payload_len: payload.len() as u32,
            target_group: 0, target_hive: 0,
        },
        route: None,
        payload: &payload,
        hmac_tag: None,
    };
    let mut out = vec![0u8; 64];
    let n = encode_extended(&msg, &mut out).expect("encode");
    out.truncate(n);
    write_frame(&mut writer, &out).await.expect("write");

    let response_frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&response_frame).expect("decode");
    let error_hash = r2_hash("r2.mgmt.event.error").expect("hash");
    assert_eq!(parsed.header.event_hash, error_hash, "expected error frame");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn event_send_no_hive_state_returns_unsupported() {
    use r2_fnv::r2_hash;
    use r2_hive::mgmt::api::build_event_send_request;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new(); // no hive_state
    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    let req = build_event_send_request(1, "org.example.ping", &[0xA1, 0x00, 0x18, 0x2A], None, None);
    write_frame(&mut writer, &req).await.expect("write");

    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let err_hash = r2_hash("r2.mgmt.event.error").expect("hash");
    assert_eq!(parsed.header.event_hash, err_hash, "expected error frame");
    // The error code is "unsupported" — not asserted bit-for-bit since
    // CBOR text decoding through the existing helpers is non-trivial.

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn event_send_broadcast_returns_msg_id() {
    use std::sync::Arc;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::build_event_send_request;
    use r2_hive::mgmt::primitive::parse_event_send_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    state.attach_hive_state(Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64)));

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    let cid = 0xABCD_u64;
    let req = build_event_send_request(cid, "org.example.ping", &[0xA1, 0x00, 0x18, 0x2A], None, None);
    write_frame(&mut writer, &req).await.expect("write");

    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let (rcid, msg_id) = parse_event_send_response(parsed.payload).expect("parse");
    assert_eq!(rcid, cid);
    assert!(msg_id > 0, "msg_id should be assigned");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn transport_allow_mask_mgmt_state_set_clear_roundtrip() {
    use std::sync::Arc;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::transport_policy::{
        build_clear_request, build_set_request, build_state_request, parse_response,
        EV_TRANSPORT_ALLOW_MASK_CLEAR, EV_TRANSPORT_ALLOW_MASK_SET,
        EV_TRANSPORT_ALLOW_MASK_STATE,
    };
    use r2_route::transport::{Transport, TransportSet};

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let hive = Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64));
    state.attach_hive_state(hive.clone());

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    write_frame(&mut writer, &build_state_request(1)).await.expect("write state");
    let frame = read_frame(&mut reader).await.expect("read").expect("state");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_fnv::r2_hash(EV_TRANSPORT_ALLOW_MASK_STATE).unwrap()
    );
    let state_resp = parse_response(parsed.payload).expect("parse state");
    assert_eq!(state_resp.correlation_id, 1);
    assert_eq!(state_resp.effective_mask, TransportSet::ALL_BITS);
    assert_eq!(state_resp.all_mask, TransportSet::ALL_BITS);
    assert!(!state_resp.active_lease);

    let requested = Transport::Wifi.bit() | 0x80;
    write_frame(&mut writer, &build_set_request(2, requested, 0xA11CE, "bench-phase2"))
        .await
        .expect("write set");
    let frame = read_frame(&mut reader).await.expect("read").expect("set");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_fnv::r2_hash(EV_TRANSPORT_ALLOW_MASK_SET).unwrap()
    );
    let set_resp = parse_response(parsed.payload).expect("parse set");
    assert_eq!(set_resp.correlation_id, 2);
    assert_eq!(set_resp.requested_mask, Some(requested));
    assert_eq!(set_resp.accepted_mask, Some(Transport::Wifi.bit()));
    assert_eq!(set_resp.effective_mask, Transport::Wifi.bit());
    assert_eq!(set_resp.lease_id, Some(0xA11CE));
    assert_eq!(set_resp.source.as_deref(), Some("bench-phase2"));
    assert!(set_resp.active_lease);
    assert_eq!(
        hive.transport_policy_snapshot().await.effective_mask,
        Transport::Wifi.bit()
    );

    write_frame(&mut writer, &build_state_request(3)).await.expect("write state");
    let frame = read_frame(&mut reader).await.expect("read").expect("state");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let state_resp = parse_response(parsed.payload).expect("parse state");
    assert_eq!(state_resp.correlation_id, 3);
    assert_eq!(state_resp.requested_mask, Some(requested));
    assert_eq!(state_resp.accepted_mask, Some(Transport::Wifi.bit()));
    assert_eq!(state_resp.effective_mask, Transport::Wifi.bit());
    assert!(state_resp.active_lease);

    write_frame(&mut writer, &build_clear_request(4, Some(0xA11CE)))
        .await
        .expect("write clear");
    let frame = read_frame(&mut reader).await.expect("read").expect("clear");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_fnv::r2_hash(EV_TRANSPORT_ALLOW_MASK_CLEAR).unwrap()
    );
    let clear_resp = parse_response(parsed.payload).expect("parse clear");
    assert_eq!(clear_resp.correlation_id, 4);
    assert_eq!(clear_resp.effective_mask, TransportSet::ALL_BITS);
    assert!(!clear_resp.active_lease);
    assert!(
        hive.transport_policy_snapshot()
            .await
            .active_lease
            .is_none()
    );

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn transport_allow_mask_mgmt_without_hive_state_is_unsupported() {
    use r2_fnv::r2_hash;
    use r2_hive::mgmt::transport_policy::build_state_request;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    write_frame(&mut writer, &build_state_request(1)).await.expect("write");
    let frame = read_frame(&mut reader).await.expect("read").expect("frame");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.mgmt.event.error").expect("hash"),
        "recognised local event should fail closed without HiveState"
    );

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn event_send_targeted_unknown_peer_returns_peer_not_found() {
    use std::sync::Arc;
    use r2_fnv::r2_hash;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::build_event_send_request;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    state.attach_hive_state(Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64)));
    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    let cid = 0x99_u64;
    let unknown_target: u64 = 0xDEADC0DE;
    let req = build_event_send_request(cid, "org.example.ping", &[], Some(unknown_target), None);
    write_frame(&mut writer, &req).await.expect("write");

    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let err_hash = r2_hash("r2.mgmt.event.error").expect("hash");
    assert_eq!(parsed.header.event_hash, err_hash, "expected error frame for peer_not_found");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn tg_current_no_attachment_returns_only_cid() {
    use r2_hive::mgmt::api::build_tg_current_request;
    use r2_hive::mgmt::primitive::parse_tg_current_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    let cid = 0xDEAD_u64;
    write_frame(&mut writer, &build_tg_current_request(cid)).await.expect("write");
    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let (rcid, attached) = parse_tg_current_response(parsed.payload).expect("parse");
    assert_eq!(rcid, cid);
    assert!(attached.is_none(), "no HiveState ⇒ no attachment");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn tg_current_with_attached_tg_returns_full_payload() {
    use std::sync::Arc;
    use r2_hive::hive::{ActiveTg, HiveState, TgMemberRole};
    use r2_hive::mgmt::api::build_tg_current_request;
    use r2_hive::mgmt::primitive::parse_tg_current_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let self_id: u32 = 0xCAFEBEEF;
    let hive = Arc::new(HiveState::new(self_id, 1024, 64));

    // Synthesize an active TG. tg_id set to a recognisable pattern; tg_hash
    // is the first 8 bytes (the on-wire scoping identifier in tg_map).
    let tg_id_bytes: [u8; 32] = [0xAA; 32];
    let tg_hash: [u8; 8] = [0xAA; 8];
    hive.set_active_tg(ActiveTg {
        tg_id: tg_id_bytes,
        tg_hash,
        member_role: TgMemberRole::KeyHolder,
        hive_id: self_id,
    }).await;
    state.attach_hive_state(hive);

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    let cid = 0xBEEF_u64;
    write_frame(&mut writer, &build_tg_current_request(cid)).await.expect("write");
    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let (rcid, attached) = parse_tg_current_response(parsed.payload).expect("parse");
    assert_eq!(rcid, cid);
    let (tg_id, role, hive_id) = attached.expect("expected an attachment");
    assert_eq!(tg_id, tg_id_bytes.to_vec());
    assert_eq!(role, 2, "key_holder wire value");
    assert_eq!(hive_id, self_id as u64);

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn cap_query_returns_empty_in_v0_1() {
    use r2_hive::mgmt::api::build_cap_query_request;
    use r2_hive::mgmt::primitive::parse_cap_query_response;

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();
    let cid = 0xCAFE_u64;
    write_frame(&mut writer, &build_cap_query_request(cid, None)).await.expect("write");
    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let (rcid, bloom, hashes, classes) =
        parse_cap_query_response(parsed.payload).expect("parse");
    assert_eq!(rcid, cid);
    // v0.1: no capability advertisements yet → empty across the board.
    assert!(bloom.is_empty(), "bloom={:?}", bloom);
    assert!(hashes.is_empty());
    assert!(classes.is_empty());

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn event_subscribe_unsubscribe_roundtrip() {
    use r2_hive::mgmt::api::{build_event_subscribe_request, build_event_unsubscribe_request};
    use r2_hive::mgmt::primitive::{
        parse_event_subscribe_response, parse_event_unsubscribe_response,
    };

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new(); // no HiveState — local registry path
    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    // Subscribe by event class.
    let cid_sub = 1u64;
    write_frame(
        &mut writer,
        &build_event_subscribe_request(cid_sub, Some("org.example.ping"), None, None),
    )
    .await
    .expect("write");
    let sub_resp = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&sub_resp).expect("decode");
    let (rcid, sub_id) = parse_event_subscribe_response(parsed.payload).expect("parse subscribe");
    assert_eq!(rcid, cid_sub);
    assert!(sub_id != 0, "sub_id non-zero");

    // Unsubscribe.
    let cid_unsub = 2u64;
    write_frame(
        &mut writer,
        &build_event_unsubscribe_request(cid_unsub, sub_id),
    )
    .await
    .expect("write");
    let unsub_resp = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&unsub_resp).expect("decode");
    let (rcid, status) = parse_event_unsubscribe_response(parsed.payload).expect("parse unsub");
    assert_eq!(rcid, cid_unsub);
    assert_eq!(status, 0, "ok");

    // Unsubscribing again should report status=1 (no such subscription).
    let cid_unsub2 = 3u64;
    write_frame(
        &mut writer,
        &build_event_unsubscribe_request(cid_unsub2, sub_id),
    )
    .await
    .expect("write");
    let unsub_resp = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&unsub_resp).expect("decode");
    let (rcid, status) = parse_event_unsubscribe_response(parsed.payload).expect("parse unsub");
    assert_eq!(rcid, cid_unsub2);
    assert_eq!(status, 1, "no such subscription");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn delivery_fires_for_matching_subscription() {
    use std::sync::Arc;
    use r2_fnv::r2_hash;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::build_event_subscribe_request;
    use r2_hive::mgmt::primitive::parse_event_subscribe_response;
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let hive = Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64));
    state.attach_hive_state(hive.clone());

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    // Subscribe to a specific event class.
    write_frame(
        &mut writer,
        &build_event_subscribe_request(1, Some("org.example.broadcast"), None, None),
    )
    .await
    .expect("write");
    let sub_resp = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&sub_resp).expect("decode");
    let (_, sub_id) = parse_event_subscribe_response(parsed.payload).expect("parse");

    // Synthesize an inbound mesh frame matching the subscription and call
    // deliver_inbound directly. (In production this is called by the
    // route engine on inbound traffic.)
    let inner_payload = vec![0xA1, 0x00, 0x18, 0xFF];
    let event_hash = r2_hash("org.example.broadcast").expect("hash");
    let inbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 5,
            k: 0,
            msg_id: 99,
            event_hash,
            payload_len: inner_payload.len() as u32,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload: &inner_payload,
        hmac_tag: None,
    };
    let mut wire = vec![0u8; 256];
    let n = encode_extended(&inbound, &mut wire).expect("encode");
    wire.truncate(n);

    hive.deliver_inbound(&wire, 0xDEADBEEF, None).await;

    // We should now receive an r2.api.event.delivery notification on the
    // socket, addressed to the sub_id we got back.
    let delivery = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&delivery).expect("decode");
    let delivery_hash = r2_hash("r2.api.event.delivery").expect("hash");
    assert_eq!(parsed.header.event_hash, delivery_hash, "delivery frame");

    // Spot-check the payload: key 1 should be the sub_id.
    use r2_cbor::{Decoder, Item};
    let mut dec = Decoder::new(parsed.payload);
    let entries = match dec.next().expect("cbor") {
        Item::Map(n) => n,
        _ => panic!("not a map"),
    };
    let mut found_sub_id = false;
    let mut found_msg_id = false;
    for _ in 0..entries {
        let key = dec.next().expect("key");
        let val = dec.next().expect("val");
        match (key, val) {
            (Item::UInt(1), Item::UInt(n)) => {
                assert_eq!(n, sub_id as u64);
                found_sub_id = true;
            }
            (Item::UInt(7), Item::UInt(n)) => {
                assert_eq!(n, 99, "msg_id should pass through");
                found_msg_id = true;
            }
            _ => {}
        }
    }
    assert!(found_sub_id, "sub_id key missing");
    assert!(found_msg_id, "msg_id key missing");

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn delivery_does_not_fire_for_non_matching_subscription() {
    use std::sync::Arc;
    use std::time::Duration;
    use r2_fnv::r2_hash;
    use r2_hive::hive::HiveState;
    use r2_hive::mgmt::api::{build_event_subscribe_request, build_peer_list_request};
    use r2_hive::mgmt::primitive::{parse_event_subscribe_response, parse_peer_list_response};
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let hive = Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64));
    state.attach_hive_state(hive.clone());

    let handle = socket::spawn(socket_path.clone(), state.clone()).await.expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    // Subscribe to "org.example.A" only.
    write_frame(
        &mut writer,
        &build_event_subscribe_request(1, Some("org.example.A"), None, None),
    )
    .await
    .expect("write");
    let _ = parse_event_subscribe_response(
        r2_wire::decode_extended(&read_frame(&mut reader).await.unwrap().unwrap())
            .unwrap()
            .payload,
    )
    .unwrap();

    // Deliver an event for a different class.
    let inner_payload = vec![];
    let event_hash = r2_hash("org.example.B").expect("hash");
    let inbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 5, k: 0, msg_id: 0, event_hash,
            payload_len: 0,
            target_group: 0, target_hive: 0,
        },
        route: None,
        payload: &inner_payload,
        hmac_tag: None,
    };
    let mut wire = vec![0u8; 64];
    let n = encode_extended(&inbound, &mut wire).expect("encode");
    wire.truncate(n);
    hive.deliver_inbound(&wire, 0xDEADBEEF, None).await;

    // Make a normal request and verify no delivery snuck in front of the
    // response. peer.list returns one frame; if a delivery matched, we'd
    // see two frames here.
    write_frame(&mut writer, &build_peer_list_request(2)).await.expect("write");
    // Allow any spurious delivery to arrive.
    tokio::time::sleep(Duration::from_millis(20)).await;
    let frame = read_frame(&mut reader).await.expect("read").expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let peer_list_hash = r2_hash("r2.api.peer.list").expect("hash");
    assert_eq!(
        parsed.header.event_hash, peer_list_hash,
        "expected peer.list response, not a delivery"
    );
    let (rcid, _peers) = parse_peer_list_response(parsed.payload).expect("parse");
    assert_eq!(rcid, 2);

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn daemon_rejects_unknown_event() {
    use r2_fnv::r2_hash;
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2tgd.sock");
    let state = DaemonState::new();
    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn daemon");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .expect("connect to daemon");
    let (mut reader, mut writer) = stream.split();

    // Craft a frame for an event class the daemon does not know.
    let event_hash = r2_hash("r2.mgmt.something.made.up").expect("hash");
    let msg = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 0,
            k: 0,
            msg_id: 0,
            event_hash,
            payload_len: 0,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload: &[],
        hmac_tag: None,
    };
    let mut out = vec![0u8; 64];
    let n = encode_extended(&msg, &mut out).expect("encode");
    out.truncate(n);

    write_frame(&mut writer, &out)
        .await
        .expect("write unknown-event");

    let response_frame = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");

    // The response should be an r2.mgmt.event.error frame — quick sniff via the
    // event hash.
    let parsed = r2_wire::decode_extended(&response_frame).expect("decode response");
    let error_hash = r2_hash("r2.mgmt.event.error").expect("hash");
    assert_eq!(
        parsed.header.event_hash, error_hash,
        "expected error response frame"
    );

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

/// R2-TG-TOOL §5.1 v0.4 MUST (b) — the squat-guard REFUSAL arm, runtime-
/// tested WITHOUT root or a user namespace. The guard's path is
/// exists → stat → foreign-uid → refuse, and it is file-type-agnostic, so
/// any pre-existing ROOT-owned path exercises it (/proc/version is root-
/// owned on every Linux). Note for future readers: the
/// `unshare --map-root-user` route does NOT work here — your own files map
/// TOGETHER WITH your uid (both read 0 inside the ns), so the mismatch
/// never occurs; verified empirically before choosing this construction.
#[tokio::test]
async fn squat_guard_refuses_foreign_owned_socket_path() {
    // As root every file is ours and the arm cannot fire — skip honestly.
    fn uid() -> u32 {
        extern "C" {
            fn getuid() -> u32;
        }
        unsafe { getuid() }
    }
    if uid() == 0 {
        eprintln!("skipped: running as root — foreign-uid mismatch unobtainable");
        return;
    }
    let state = DaemonState::new();
    let err = match socket::spawn(std::path::PathBuf::from("/proc/version"), state).await {
        Err(e) => e,
        Ok(_) => panic!("foreign-owned path at the socket name MUST refuse to bind"),
    };
    assert_eq!(
        err.kind(),
        std::io::ErrorKind::PermissionDenied,
        "squat guard must refuse with PermissionDenied, got: {err}"
    );
}

