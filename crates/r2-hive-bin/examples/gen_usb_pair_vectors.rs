//! Generator for `r2-usb-pair-vectors.json` — the USB pairing crypto
//! conformance fixtures. Emits byte-exact vectors from r2-hive's CANONICAL
//! crypto helpers (`r2_hive::usb_pair`, X25519 / SHA-256 / HKDF-SHA256 /
//! HMAC-SHA256) so the fixture can never drift from the implementation it
//! pins. The deterministic inputs match `usb_pair.rs`'s `fixed()` test block.
//!
//! Run: `cargo run --example gen_usb_pair_vectors`
//! Writes `r2-specifications/testing/test-vectors/r2-usb-pair-vectors.json`.
//!
//! GOVERNING SPEC: pending specs confirmation. The impl cites "R2-HIVE §6.4",
//! but specs ruled R2-HIVE is not a spec — the real home is the provisioning /
//! USB crypto contract (R2-PROVISION / R2-USB / R2-TRUST primitives). specs
//! confirms the governing spec + verifies these are spec-driven before landing.

use std::fs;
use std::path::PathBuf;

use r2_hive::usb_pair::{
    commitment, link_key, public_key_from_secret, reconnect_tag, sas_code, shared_secret,
    verify_commitment, verify_reconnect_tag,
};

fn unhex<const N: usize>(s: &str) -> [u8; N] {
    let bytes: Vec<u8> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect();
    let mut a = [0u8; N];
    a.copy_from_slice(&bytes);
    a
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    // Deterministic pinned inputs (== usb_pair.rs `fixed()`).
    let sk_host = unhex::<32>("1111111111111111111111111111111111111111111111111111111111111111");
    let sk_periph = unhex::<32>("2222222222222222222222222222222222222222222222222222222222222222");
    let nonce_host = unhex::<32>("3333333333333333333333333333333333333333333333333333333333333333");
    let nonce_periph = unhex::<32>("4444444444444444444444444444444444444444444444444444444444444444");
    let hive_id = unhex::<16>("55555555555555555555555555555555");
    let nonce_rc = unhex::<16>("66666666666666666666666666666666");

    // Derived via the canonical helpers.
    let pk_host = public_key_from_secret(&sk_host);
    let pk_periph = public_key_from_secret(&sk_periph);
    let z = shared_secret(&sk_host, &pk_periph);
    let commit = commitment(&pk_periph, &nonce_periph);
    let sas = sas_code(&z, &pk_host, &pk_periph, &nonce_host, &nonce_periph);
    let lk = link_key(&z, &pk_host, &pk_periph, &nonce_host, &nonce_periph, &hive_id);
    let tag = reconnect_tag(&lk, &nonce_rc, &hive_id);

    let mut bad_pk = pk_periph;
    bad_pk[0] ^= 0x01;
    let mut bad_key = lk;
    bad_key[0] ^= 0x01;
    let mut bad_nonce = nonce_rc;
    bad_nonce[0] ^= 0x01;

    let vectors = serde_json::json!([
        {"id":"UP1","category":"x25519","description":"eph public key from host secret","secret":hex(&sk_host),"public":hex(&pk_host)},
        {"id":"UP2","category":"x25519","description":"eph public key from peripheral secret","secret":hex(&sk_periph),"public":hex(&pk_periph)},
        {"id":"UP3","category":"x25519","description":"ECDH shared secret Z (agrees both sides)","host_secret":hex(&sk_host),"peer_public":hex(&pk_periph),"shared_secret":hex(&z)},
        {"id":"UP4","category":"commitment","description":"SHA-256(eph_pk_peripheral || nonce_peripheral)","eph_pk_peripheral":hex(&pk_periph),"nonce_peripheral":hex(&nonce_periph),"commitment":hex(&commit)},
        {"id":"UP5","category":"commitment","description":"verify_commitment accepts correct reveal","commitment":hex(&commit),"eph_pk_peripheral":hex(&pk_periph),"nonce_peripheral":hex(&nonce_periph),"expect":verify_commitment(&commit,&pk_periph,&nonce_periph)},
        {"id":"UP6","category":"commitment","description":"verify_commitment rejects substituted pk","commitment":hex(&commit),"eph_pk_peripheral":hex(&bad_pk),"nonce_peripheral":hex(&nonce_periph),"expect":verify_commitment(&commit,&bad_pk,&nonce_periph)},
        {"id":"UP7","category":"sas","description":"6-digit SAS code (HKDF-SHA256, % 1_000_000)","shared_secret":hex(&z),"eph_pk_host":hex(&pk_host),"eph_pk_peripheral":hex(&pk_periph),"nonce_host":hex(&nonce_host),"nonce_peripheral":hex(&nonce_periph),"sas_code":sas,"rendered":format!("{sas:06}")},
        {"id":"UP8","category":"linkkey","description":"32-byte link key (HKDF-SHA256)","shared_secret":hex(&z),"eph_pk_host":hex(&pk_host),"eph_pk_peripheral":hex(&pk_periph),"nonce_host":hex(&nonce_host),"nonce_peripheral":hex(&nonce_periph),"hive_id_bytes":hex(&hive_id),"link_key":hex(&lk)},
        {"id":"UP9","category":"reconnect","description":"reconnect tag HMAC-SHA256(link_key, label||nonce_rc||hive_id)[..16]","link_key":hex(&lk),"nonce_rc":hex(&nonce_rc),"hive_id_bytes":hex(&hive_id),"reconnect_tag":hex(&tag)},
        {"id":"UP10","category":"reconnect","description":"verify_reconnect_tag accepts correct tag","reconnect_tag":hex(&tag),"link_key":hex(&lk),"nonce_rc":hex(&nonce_rc),"hive_id_bytes":hex(&hive_id),"expect":verify_reconnect_tag(&tag,&lk,&nonce_rc,&hive_id)},
        {"id":"UP11","category":"reconnect","description":"verify_reconnect_tag rejects wrong link key","reconnect_tag":hex(&tag),"link_key":hex(&bad_key),"nonce_rc":hex(&nonce_rc),"hive_id_bytes":hex(&hive_id),"expect":verify_reconnect_tag(&tag,&bad_key,&nonce_rc,&hive_id)},
        {"id":"UP12","category":"reconnect","description":"verify_reconnect_tag rejects wrong nonce","reconnect_tag":hex(&tag),"link_key":hex(&lk),"nonce_rc":hex(&bad_nonce),"hive_id_bytes":hex(&hive_id),"expect":verify_reconnect_tag(&tag,&lk,&bad_nonce,&hive_id)},
    ]);

    let doc = serde_json::json!({
        "spec": "R2-PROVISION",
        "version": "0.1",
        "description": "USB pairing crypto conformance vectors (X25519 / SHA-256 / \
            HKDF-SHA256 / HMAC-SHA256). GENERATED from r2-hive's canonical usb_pair \
            helpers via examples/gen_usb_pair_vectors.rs — do not hand-edit. \
            Governing spec pending specs confirmation: impl cites R2-HIVE §6.4 (which \
            specs ruled is not a spec); real home is the provisioning/USB crypto \
            contract. Inputs are deterministic test values; keys are TEST-ONLY.",
        "vectors": vectors,
    });

    let out: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..", "..", "..", "r2-specifications", "testing", "test-vectors",
        "r2-usb-pair-vectors.json",
    ]
    .iter()
    .collect();

    let json = serde_json::to_string_pretty(&doc).expect("serialize");
    fs::write(&out, json + "\n").unwrap_or_else(|e| panic!("write {out:?}: {e}"));
    println!("wrote {} usb-pair vectors to {}", doc["vectors"].as_array().unwrap().len(), out.display());
}
