//! Integration tests for `r2.api.service.{advertise,retract}`
//! (R2-HOST-API §5.2, R2-PLUGIN §5).

use std::sync::Arc;

use tokio::net::UnixStream;

use r2_cbor::{Decoder, Item};
use r2_fnv::r2_hash;
use r2_hive::hive::HiveState;
use r2_hive::mgmt::api::{build_service_advertise_request, build_service_retract_request};
use r2_hive::mgmt::framing::{read_frame, write_frame};
use r2_hive::mgmt::{socket, state::DaemonState};
use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

fn extract_uint(payload: &[u8], target: u64) -> Option<u64> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let k = dec.next().ok()?;
        let v = dec.next().ok()?;
        if let Item::UInt(kk) = k {
            if kk == target {
                if let Item::UInt(n) = v {
                    return Some(n);
                }
            }
        }
    }
    None
}

#[tokio::test]
async fn service_advertise_returns_high_bit_id() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2-hive.sock");
    let state = DaemonState::new();
    state.attach_hive_state(Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64)));
    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    write_frame(
        &mut writer,
        &build_service_advertise_request(1, "org.example.echo"),
    )
    .await
    .expect("write");

    let resp = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.api.service.advertise").unwrap(),
        "advertise response event class"
    );
    let cid = extract_uint(parsed.payload, 0).unwrap();
    let service_id = extract_uint(parsed.payload, 1).unwrap();
    assert_eq!(cid, 1);
    assert!(
        service_id & 0x8000_0000 != 0,
        "service_id must have high bit set: 0x{:08X}",
        service_id
    );

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn service_retract_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2-hive.sock");
    let state = DaemonState::new();
    state.attach_hive_state(Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64)));
    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    // 1. advertise
    write_frame(
        &mut writer,
        &build_service_advertise_request(1, "org.example.echo"),
    )
    .await
    .expect("write");
    let resp = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    let service_id = extract_uint(parsed.payload, 1).unwrap() as u32;

    // 2. first retract — succeeds
    write_frame(&mut writer, &build_service_retract_request(2, service_id))
        .await
        .expect("write");
    let resp = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.api.service.retract").unwrap()
    );
    assert_eq!(extract_uint(parsed.payload, 1), Some(service_id as u64));

    // 3. second retract on the same id — also succeeds (idempotent)
    write_frame(&mut writer, &build_service_retract_request(3, service_id))
        .await
        .expect("write");
    let resp = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.api.service.retract").unwrap()
    );

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn advertised_service_receives_matching_event_delivery() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2-hive.sock");
    let state = DaemonState::new();
    let hive = Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64));
    state.attach_hive_state(hive.clone());
    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    // Advertise a service for "org.example.run".
    write_frame(
        &mut writer,
        &build_service_advertise_request(1, "org.example.run"),
    )
    .await
    .expect("write");
    let resp = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    let service_id = extract_uint(parsed.payload, 1).unwrap() as u32;
    assert!(service_id & 0x8000_0000 != 0);

    // Synthesize an inbound mesh frame for "org.example.run".
    let payload = vec![0xA1, 0x00, 0x18, 0x2A];
    let event_hash = r2_hash("org.example.run").unwrap();
    let inbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 5,
            k: 0,
            msg_id: 7,
            event_hash,
            payload_len: payload.len() as u32,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload: &payload,
        hmac_tag: None,
    };
    let mut wire = vec![0u8; 256];
    let n = encode_extended(&inbound, &mut wire).expect("encode");
    wire.truncate(n);
    hive.deliver_inbound(&wire, 0xDEADBEEF, None).await;

    // The service registration should receive a delivery on this connection.
    let delivery = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&delivery).expect("decode");
    assert_eq!(
        parsed.header.event_hash,
        r2_hash("r2.api.event.delivery").unwrap(),
        "expected delivery frame"
    );
    // sub_id field on the delivery is the service_id with high bit set.
    let observed_id = extract_uint(parsed.payload, 1).unwrap() as u32;
    assert_eq!(observed_id, service_id);

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}

#[tokio::test]
async fn retracted_service_no_longer_receives_deliveries() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let socket_path = tmp.path().join("r2-hive.sock");
    let state = DaemonState::new();
    let hive = Arc::new(HiveState::new(0xCAFEBEEF, 1024, 64));
    state.attach_hive_state(hive.clone());
    let handle = socket::spawn(socket_path.clone(), state.clone())
        .await
        .expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut stream = UnixStream::connect(&socket_path).await.expect("connect");
    let (mut reader, mut writer) = stream.split();

    // Advertise then immediately retract.
    write_frame(
        &mut writer,
        &build_service_advertise_request(1, "org.example.gone"),
    )
    .await
    .expect("write");
    let resp = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&resp).expect("decode");
    let service_id = extract_uint(parsed.payload, 1).unwrap() as u32;

    write_frame(&mut writer, &build_service_retract_request(2, service_id))
        .await
        .expect("write");
    let _ = read_frame(&mut reader).await.expect("read").unwrap();

    // Now deliver an event of the same class.
    let event_hash = r2_hash("org.example.gone").unwrap();
    let payload = vec![];
    let inbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 5,
            k: 0,
            msg_id: 0,
            event_hash,
            payload_len: 0,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload: &payload,
        hmac_tag: None,
    };
    let mut wire = vec![0u8; 64];
    let n = encode_extended(&inbound, &mut wire).expect("encode");
    wire.truncate(n);
    hive.deliver_inbound(&wire, 0xDEADBEEF, None).await;

    // Issue a normal request; if a delivery had been queued we would
    // see two frames here. We expect exactly one (the response to our
    // request).
    use r2_hive::mgmt::api::build_status_request;
    write_frame(&mut writer, &build_status_request(99))
        .await
        .expect("write");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let frame = read_frame(&mut reader)
        .await
        .expect("read")
        .expect("non-empty");
    let parsed = r2_wire::decode_extended(&frame).expect("decode");
    let status_hash = r2_hash("r2.mgmt.daemon.status").unwrap();
    assert_eq!(
        parsed.header.event_hash, status_hash,
        "expected daemon.status response, not a delivery — got 0x{:08X}",
        parsed.header.event_hash
    );

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
}
