//! R2-HIVE §6.4 pairing — pure cryptographic helpers (Phase USB-2).
//!
//! These functions implement the byte-pinned crypto contract from
//! `r2-specifications/specs/r2-core/R2-HIVE.md` §§6.4.1–6.4.6 and the
//! test vectors at `r2-usb-pair-vectors.json`. They are deterministic
//! (no CSPRNG), allocation-free aside from heap-clearable scratch
//! buffers, and depend only on `x25519-dalek`, `sha2`, `hkdf`, and
//! `hmac`. The pairing state machine in [`crate::usb::session`] glues
//! these helpers to the wire I/O.
//!
//! Algorithm pin per §6.4.1:
//!
//! - X25519 (RFC 7748) for key agreement.
//! - SHA-256 for the commitment.
//! - HKDF-SHA256 for the SAS and link key.
//! - HMAC-SHA256 (truncated to 16 bytes) for reconnect tags.
//!
//! Domain-separation labels (don't change without bumping the v1 in
//! the label):
//!
//! - `b"r2-usb-pair-sas-v1"` — HKDF salt for the 6-digit verification code.
//! - `b"r2-usb-pair-linkkey-v1"` — HKDF salt for the 32-byte link key.
//! - `b"r2-usb-reconnect-v1"` — HMAC message prefix for reconnect.

use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

/// 32-byte X25519 public key.
pub type PublicKey32 = [u8; 32];
/// 32-byte X25519 raw secret key (pre-clamp; clamping happens inside
/// `x25519-dalek` on first scalar mult).
pub type SecretKey32 = [u8; 32];
/// 32-byte SHA-256 commitment.
pub type Commitment = [u8; 32];
/// 32-byte X25519 shared secret.
pub type SharedSecret = [u8; 32];
/// 32-byte HMAC-SHA256 link key.
pub type LinkKey = [u8; 32];
/// 32-byte commit/SAS nonce.
pub type Nonce32 = [u8; 32];
/// 16-byte device identifier from CAPS field 0.
pub type HiveIdBytes = [u8; 16];
/// 16-byte reconnect challenge nonce.
pub type ReconnectNonce = [u8; 16];
/// 16-byte truncated HMAC tag for reconnect responses.
pub type ReconnectTag = [u8; 16];

const SAS_LABEL: &[u8] = b"r2-usb-pair-sas-v1";
const LINK_KEY_LABEL: &[u8] = b"r2-usb-pair-linkkey-v1";
const RECONNECT_LABEL: &[u8] = b"r2-usb-reconnect-v1";

/// Compute the host's X25519 public key from a raw secret. The
/// `x25519-dalek` `StaticSecret::from(bytes)` clamps internally on
/// first scalar mult per RFC 7748.
pub fn public_key_from_secret(sk: &SecretKey32) -> PublicKey32 {
    let sec = StaticSecret::from(*sk);
    let pk = PublicKey::from(&sec);
    *pk.as_bytes()
}

/// X25519 ECDH: compute the shared secret `Z` from one side's secret
/// and the other side's public key. Both sides arrive at the same
/// value.
pub fn shared_secret(self_sk: &SecretKey32, peer_pk: &PublicKey32) -> SharedSecret {
    let sec = StaticSecret::from(*self_sk);
    let pk = PublicKey::from(*peer_pk);
    let z = sec.diffie_hellman(&pk);
    *z.as_bytes()
}

/// SHA-256(eph_pk_peripheral ‖ nonce_peripheral). The peripheral's
/// commitment, sent in `PAIR_COMMIT` (msg_type 5).
pub fn commitment(eph_pk_peripheral: &PublicKey32, nonce_peripheral: &Nonce32) -> Commitment {
    let mut h = Sha256::new();
    h.update(eph_pk_peripheral);
    h.update(nonce_peripheral);
    let digest = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Verify the peripheral's commitment matches what `PAIR_REVEAL`
/// disclosed. Constant-time equality.
pub fn verify_commitment(
    expected: &Commitment,
    eph_pk_peripheral: &PublicKey32,
    nonce_peripheral: &Nonce32,
) -> bool {
    let computed = commitment(eph_pk_peripheral, nonce_peripheral);
    constant_time_eq(expected, &computed)
}

/// Derive the 6-digit verification code per §6.4.4.
///
/// Both sides compute the same `sas_code` from the shared secret and
/// the four committed values; the host displays it in its pairing
/// UI; the peripheral renders it (display, USB-CDC, blink — per
/// §6.4.8). User confirms they match.
///
/// Returns the 6-digit code in `0..=999_999`. Render as `{:06}` to
/// preserve leading zeros.
pub fn sas_code(
    z: &SharedSecret,
    eph_pk_host: &PublicKey32,
    eph_pk_peripheral: &PublicKey32,
    nonce_host: &Nonce32,
    nonce_peripheral: &Nonce32,
) -> u32 {
    let mut info = [0u8; 32 + 32 + 32 + 32];
    info[..32].copy_from_slice(eph_pk_host);
    info[32..64].copy_from_slice(eph_pk_peripheral);
    info[64..96].copy_from_slice(nonce_host);
    info[96..128].copy_from_slice(nonce_peripheral);

    let hk = Hkdf::<Sha256>::new(Some(SAS_LABEL), z);
    let mut okm = [0u8; 4];
    hk.expand(&info, &mut okm).expect("HKDF expand 4 bytes");
    let u = u32::from_be_bytes(okm);
    u % 1_000_000
}

/// Derive the long-term link key per §6.4.5. Stored on both sides
/// keyed by `hive_id_bytes`; survives reboots; survives OTA.
pub fn link_key(
    z: &SharedSecret,
    eph_pk_host: &PublicKey32,
    eph_pk_peripheral: &PublicKey32,
    nonce_host: &Nonce32,
    nonce_peripheral: &Nonce32,
    hive_id_bytes: &HiveIdBytes,
) -> LinkKey {
    let mut info = [0u8; 32 + 32 + 32 + 32 + 16];
    info[..32].copy_from_slice(eph_pk_host);
    info[32..64].copy_from_slice(eph_pk_peripheral);
    info[64..96].copy_from_slice(nonce_host);
    info[96..128].copy_from_slice(nonce_peripheral);
    info[128..].copy_from_slice(hive_id_bytes);

    let hk = Hkdf::<Sha256>::new(Some(LINK_KEY_LABEL), z);
    let mut out = [0u8; 32];
    hk.expand(&info, &mut out).expect("HKDF expand 32 bytes");
    out
}

/// Compute the reconnect HMAC per §6.4.6.
///
/// `tag = HMAC-SHA256(link_key, b"r2-usb-reconnect-v1" || nonce_rc || hive_id_bytes)[..16]`.
///
/// Both host and peripheral compute this; constant-time compare.
pub fn reconnect_tag(
    link_key: &LinkKey,
    nonce_rc: &ReconnectNonce,
    hive_id_bytes: &HiveIdBytes,
) -> ReconnectTag {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(link_key)
        .expect("HMAC-SHA256 accepts any key length");
    mac.update(RECONNECT_LABEL);
    mac.update(nonce_rc);
    mac.update(hive_id_bytes);
    let full = mac.finalize().into_bytes();
    let mut tag = [0u8; 16];
    tag.copy_from_slice(&full[..16]);
    tag
}

/// Verify a reconnect tag in constant time.
pub fn verify_reconnect_tag(
    expected: &ReconnectTag,
    link_key: &LinkKey,
    nonce_rc: &ReconnectNonce,
    hive_id_bytes: &HiveIdBytes,
) -> bool {
    let computed = reconnect_tag(link_key, nonce_rc, hive_id_bytes);
    constant_time_eq(expected, &computed)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------
// Tests — replay every byte from r2-usb-pair-vectors.json against the
// pure crypto helpers. If any of these fail, either the spec or this
// implementation has drifted; either way it's a stop-the-line bug.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn arr<const N: usize>(s: &str) -> [u8; N] {
        let v = hex(s);
        assert_eq!(v.len(), N, "hex {s:?} not {N} bytes");
        let mut a = [0u8; N];
        a.copy_from_slice(&v);
        a
    }

    /// Pinned test inputs from r2-usb-pair-vectors.json::fixed_inputs.
    /// All vectors below are byte-equal with the JSON file's
    /// derived_values block.
    fn fixed() -> (
        SecretKey32,
        SecretKey32,
        Nonce32,
        Nonce32,
        HiveIdBytes,
        ReconnectNonce,
    ) {
        (
            arr::<32>("1111111111111111111111111111111111111111111111111111111111111111"),
            arr::<32>("2222222222222222222222222222222222222222222222222222222222222222"),
            arr::<32>("3333333333333333333333333333333333333333333333333333333333333333"),
            arr::<32>("4444444444444444444444444444444444444444444444444444444444444444"),
            arr::<16>("55555555555555555555555555555555"),
            arr::<16>("66666666666666666666666666666666"),
        )
    }

    #[test]
    fn pinned_eph_pk_host() {
        let (sk_host, _, _, _, _, _) = fixed();
        assert_eq!(
            public_key_from_secret(&sk_host),
            arr::<32>("7b4e909bbe7ffe44c465a220037d608ee35897d31ef972f07f74892cb0f73f13")
        );
    }

    #[test]
    fn pinned_eph_pk_peripheral() {
        let (_, sk_periph, _, _, _, _) = fixed();
        assert_eq!(
            public_key_from_secret(&sk_periph),
            arr::<32>("0faa684ed28867b97f4a6a2dee5df8ce974e76b7018e3f22a1c4cf2678570f20")
        );
    }

    #[test]
    fn pinned_shared_secret_z() {
        let (sk_host, sk_periph, _, _, _, _) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z_host = shared_secret(&sk_host, &pk_periph);
        let z_periph = shared_secret(&sk_periph, &pk_host);
        assert_eq!(z_host, z_periph, "ECDH must agree on both sides");
        assert_eq!(
            z_host,
            arr::<32>("9e004098efc091d4ec2663b4e9f5cfd4d7064571690b4bea97ab146ab9f35056")
        );
    }

    #[test]
    fn pinned_commitment_matches_spec() {
        let (_, sk_periph, _, nonce_periph, _, _) = fixed();
        let pk_periph = public_key_from_secret(&sk_periph);
        assert_eq!(
            commitment(&pk_periph, &nonce_periph),
            arr::<32>("63036b4d1ce9e73c19dfbcdd3238cada9ae44f3186a2139b7ecf47aa0f41625e")
        );
    }

    #[test]
    fn verify_commitment_accepts_correct_inputs() {
        let (_, sk_periph, _, nonce_periph, _, _) = fixed();
        let pk_periph = public_key_from_secret(&sk_periph);
        let c = commitment(&pk_periph, &nonce_periph);
        assert!(verify_commitment(&c, &pk_periph, &nonce_periph));
    }

    #[test]
    fn verify_commitment_rejects_substituted_pk() {
        let (_, sk_periph, _, nonce_periph, _, _) = fixed();
        let pk_periph = public_key_from_secret(&sk_periph);
        let c = commitment(&pk_periph, &nonce_periph);
        // An attacker who substitutes a different pk can't pass.
        let mut other = pk_periph;
        other[0] ^= 0x01;
        assert!(!verify_commitment(&c, &other, &nonce_periph));
    }

    #[test]
    fn pinned_sas_code() {
        let (sk_host, sk_periph, nonce_host, nonce_periph, _, _) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z = shared_secret(&sk_host, &pk_periph);
        let code = sas_code(&z, &pk_host, &pk_periph, &nonce_host, &nonce_periph);
        assert_eq!(code, 488_092);
    }

    #[test]
    fn sas_code_rendered_as_six_digits() {
        // Render contract: zero-padded six digits. 488092 doesn't need
        // padding, but the spec mandates the format so a future code
        // like 7 still renders as "000007" — exercise that too.
        assert_eq!(format!("{:06}", 488_092u32), "488092");
        assert_eq!(format!("{:06}", 7u32), "000007");
    }

    #[test]
    fn pinned_link_key() {
        let (sk_host, sk_periph, nonce_host, nonce_periph, hive_id_bytes, _) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z = shared_secret(&sk_host, &pk_periph);
        let lk = link_key(
            &z,
            &pk_host,
            &pk_periph,
            &nonce_host,
            &nonce_periph,
            &hive_id_bytes,
        );
        assert_eq!(
            lk,
            arr::<32>("386667c282a123f2847ef829386561bbebe5d02f2132ffe96a9f40d2c31c43cb")
        );
    }

    #[test]
    fn pinned_reconnect_tag() {
        let (sk_host, sk_periph, nonce_host, nonce_periph, hive_id_bytes, nonce_rc) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z = shared_secret(&sk_host, &pk_periph);
        let lk = link_key(
            &z,
            &pk_host,
            &pk_periph,
            &nonce_host,
            &nonce_periph,
            &hive_id_bytes,
        );
        let tag = reconnect_tag(&lk, &nonce_rc, &hive_id_bytes);
        assert_eq!(tag, arr::<16>("2f62edaaa469424d5a5da5630b06967b"));
    }

    #[test]
    fn verify_reconnect_tag_accepts_correct() {
        let (sk_host, sk_periph, nonce_host, nonce_periph, hive_id_bytes, nonce_rc) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z = shared_secret(&sk_host, &pk_periph);
        let lk = link_key(
            &z,
            &pk_host,
            &pk_periph,
            &nonce_host,
            &nonce_periph,
            &hive_id_bytes,
        );
        let tag = reconnect_tag(&lk, &nonce_rc, &hive_id_bytes);
        assert!(verify_reconnect_tag(&tag, &lk, &nonce_rc, &hive_id_bytes));
    }

    #[test]
    fn verify_reconnect_tag_rejects_wrong_link_key() {
        let (sk_host, sk_periph, nonce_host, nonce_periph, hive_id_bytes, nonce_rc) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z = shared_secret(&sk_host, &pk_periph);
        let lk = link_key(
            &z,
            &pk_host,
            &pk_periph,
            &nonce_host,
            &nonce_periph,
            &hive_id_bytes,
        );
        let tag = reconnect_tag(&lk, &nonce_rc, &hive_id_bytes);
        let mut wrong = lk;
        wrong[0] ^= 0x01;
        assert!(!verify_reconnect_tag(&tag, &wrong, &nonce_rc, &hive_id_bytes));
    }

    #[test]
    fn verify_reconnect_tag_rejects_wrong_nonce() {
        let (sk_host, sk_periph, nonce_host, nonce_periph, hive_id_bytes, nonce_rc) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z = shared_secret(&sk_host, &pk_periph);
        let lk = link_key(
            &z,
            &pk_host,
            &pk_periph,
            &nonce_host,
            &nonce_periph,
            &hive_id_bytes,
        );
        let tag = reconnect_tag(&lk, &nonce_rc, &hive_id_bytes);
        let mut other_nonce = nonce_rc;
        other_nonce[0] ^= 0x01;
        assert!(!verify_reconnect_tag(
            &tag,
            &lk,
            &other_nonce,
            &hive_id_bytes
        ));
    }
}
