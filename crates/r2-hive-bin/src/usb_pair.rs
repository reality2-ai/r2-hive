//! R2-PROVISION §5.3.4 USB pairing — pure cryptographic helpers (Phase USB-2).
//!
//! These functions implement the byte-pinned crypto contract from
//! `r2-specifications/specs/r2-core/R2-PROVISION.md` §5.3.4 ("USB Pairing (Wired,
//! MITM-Protected)", v0.6, the ratified canonical home — formerly R2-HIVE §6.4)
//! and the test vectors at `r2-usb-pair-vectors.json` (UP1–UP12, conformance-
//! bound). They are deterministic (no CSPRNG), allocation-free aside from
//! heap-clearable scratch buffers, and depend only on `x25519-dalek`, `sha2`,
//! `hkdf`, and `hmac`. The pairing state machine in [`crate::usb::UsbSession`]
//! glues these helpers to the wire I/O.
//!
//! Algorithm pin per §5.3.4 ("Key agreement and commitment" step 1 +
//! the closing Conformance block):
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
/// 16-byte truncated HMAC key-confirmation tag for the terminal handshake
/// (PAIR_CONFIRM msg 7 / PAIR_DONE msg 8 / PAIR_ACK msg 14). 16 B to match [`ReconnectTag`].
pub type ConfirmTag = [u8; 16];

const SAS_LABEL: &[u8] = b"r2-usb-pair-sas-v1";
const LINK_KEY_LABEL: &[u8] = b"r2-usb-pair-linkkey-v1";
const RECONNECT_LABEL: &[u8] = b"r2-usb-reconnect-v1";
/// Direction-separated terminal key-confirmation labels (R2-PROVISION §5.3.4 v0.42, UP14).
/// PAIR_CONFIRM(7)=host, PAIR_DONE(8)=peripheral, PAIR_ACK(14)=host — so a tag minted for one
/// message can never verify as another. Byte-identical across Profile A and B (terminal transcript
/// is generation-independent — hive impl-source owner confirmed, v0.41 re-lock).
const CONFIRM_HOST_LABEL: &[u8] = b"r2-usb-pair-confirm-host-v1";
const CONFIRM_PERIPHERAL_LABEL: &[u8] = b"r2-usb-pair-confirm-peripheral-v1";
const ACK_HOST_LABEL: &[u8] = b"r2-usb-pair-ack-host-v1";

/// Compute the host's X25519 public key from a raw secret. The
/// `x25519-dalek` `StaticSecret::from(bytes)` clamps internally on
/// first scalar mult per RFC 7748.
pub fn public_key_from_secret(sk: &SecretKey32) -> PublicKey32 {
    let sec = StaticSecret::from(*sk);
    let pk = PublicKey::from(&sec);
    *pk.as_bytes()
}

/// X25519 ECDH: compute the shared secret `Z` from one side's secret
/// and the other side's public key. Both sides arrive at the same value.
///
/// **R2-PROVISION §5.3.4 non-contributory reject (UP14, defense-in-depth MUST):** returns `None` when
/// the peer's key is zero/low-order so that `Z` is non-contributory (all-zero). Accepting it would make
/// the SAS + link_key attacker-influenced; the caller MUST treat `None` as a `bad_key` pairing abort and
/// derive nothing. Both host and peripheral apply this before any SAS / link_key / terminal-MAC step.
pub fn shared_secret(self_sk: &SecretKey32, peer_pk: &PublicKey32) -> Option<SharedSecret> {
    let sec = StaticSecret::from(*self_sk);
    let pk = PublicKey::from(*peer_pk);
    let z = sec.diffie_hellman(&pk);
    if !z.was_contributory() {
        return None;
    }
    Some(*z.as_bytes())
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

/// Derive the 6-digit verification code per R2-PROVISION §5.3.4 (SAS
/// verification).
///
/// Both sides compute the same `sas_code` from the shared secret and
/// the four committed values; the host displays it in its pairing
/// UI; the peripheral renders it (display, USB-CDC, blink — per the same
/// SAS verification paragraph). User confirms they match.
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

/// Derive the long-term link key per R2-PROVISION §5.3.4 (Link key). Stored on
/// both sides keyed by `hive_id_bytes`; survives reboots; survives OTA.
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

/// Compute the reconnect HMAC per R2-PROVISION §5.3.4 (Reconnect).
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

/// Terminal key-confirmation MAC over the pairing transcript (R2-PROVISION §5.3.4 v0.42 — UP14).
///
/// `tag = HMAC-SHA256(link_key, label || eph_pk_host || eph_pk_peripheral || nonce_host || nonce_peripheral)[..16]`
///
/// The 128 B transcript = the SAME four committed values that bind [`sas_code`] / [`link_key`], keyed by
/// the link key (K1). `label` is direction-separated (see the CONFIRM/DONE/ACK label consts). Each side
/// persists the link key ONLY after constant-time-verifying the peer's tag (defeats the empty-CONFIRM/DONE
/// key-confirmation desync). Callers use [`confirm_tag`] / [`done_tag`] / [`ack_tag`].
fn terminal_tag(
    link_key: &LinkKey,
    label: &[u8],
    eph_pk_host: &PublicKey32,
    eph_pk_peripheral: &PublicKey32,
    nonce_host: &Nonce32,
    nonce_peripheral: &Nonce32,
) -> ConfirmTag {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(link_key)
        .expect("HMAC-SHA256 accepts any key length");
    mac.update(label);
    mac.update(eph_pk_host);
    mac.update(eph_pk_peripheral);
    mac.update(nonce_host);
    mac.update(nonce_peripheral);
    let full = mac.finalize().into_bytes();
    let mut tag = [0u8; 16];
    tag.copy_from_slice(&full[..16]);
    tag
}

/// PAIR_CONFIRM (msg 7, host→peripheral) key-confirmation tag — [`terminal_tag`] w/ the host label.
pub fn confirm_tag(
    link_key: &LinkKey,
    eph_pk_host: &PublicKey32,
    eph_pk_peripheral: &PublicKey32,
    nonce_host: &Nonce32,
    nonce_peripheral: &Nonce32,
) -> ConfirmTag {
    terminal_tag(link_key, CONFIRM_HOST_LABEL, eph_pk_host, eph_pk_peripheral, nonce_host, nonce_peripheral)
}

/// PAIR_DONE (msg 8, peripheral→host) key-confirmation tag — [`terminal_tag`] w/ the peripheral label.
pub fn done_tag(
    link_key: &LinkKey,
    eph_pk_host: &PublicKey32,
    eph_pk_peripheral: &PublicKey32,
    nonce_host: &Nonce32,
    nonce_peripheral: &Nonce32,
) -> ConfirmTag {
    terminal_tag(link_key, CONFIRM_PERIPHERAL_LABEL, eph_pk_host, eph_pk_peripheral, nonce_host, nonce_peripheral)
}

/// PAIR_ACK (msg 14, host→peripheral) finalizer key-confirmation tag — [`terminal_tag`] w/ the ack label.
pub fn ack_tag(
    link_key: &LinkKey,
    eph_pk_host: &PublicKey32,
    eph_pk_peripheral: &PublicKey32,
    nonce_host: &Nonce32,
    nonce_peripheral: &Nonce32,
) -> ConfirmTag {
    terminal_tag(link_key, ACK_HOST_LABEL, eph_pk_host, eph_pk_peripheral, nonce_host, nonce_peripheral)
}

/// Verify a terminal key-confirmation tag in constant time. `tag_fn` is the direction's tag builder
/// ([`confirm_tag`] / [`done_tag`] / [`ack_tag`]). Returns true iff `expected` matches.
pub fn verify_terminal_tag(
    expected: &ConfirmTag,
    tag_fn: fn(&LinkKey, &PublicKey32, &PublicKey32, &Nonce32, &Nonce32) -> ConfirmTag,
    link_key: &LinkKey,
    eph_pk_host: &PublicKey32,
    eph_pk_peripheral: &PublicKey32,
    nonce_host: &Nonce32,
    nonce_peripheral: &Nonce32,
) -> bool {
    let computed = tag_fn(link_key, eph_pk_host, eph_pk_peripheral, nonce_host, nonce_peripheral);
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

    /// Profile A (R2-PROVISION §5.3.4 v0.42) — terminal MACs (UP14) + gen-free reconnect (UP18) +
    /// non-contributory reject, byte-exact vs the landed origin vectors (r2-usb-pair-vectors 0.10).
    #[test]
    fn profile_a_terminal_reconnect_and_reject_vectors() {
        let (sk_host, sk_periph, nonce_host, nonce_periph, hive_id, nonce_rc) = fixed();
        let pk_host = public_key_from_secret(&sk_host);
        let pk_periph = public_key_from_secret(&sk_periph);
        let z = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
        let k1 = link_key(&z, &pk_host, &pk_periph, &nonce_host, &nonce_periph, &hive_id);
        // Canonical link_key K1 (UP8 / UP18).
        assert_eq!(
            k1,
            arr::<32>("386667c282a123f2847ef829386561bbebe5d02f2132ffe96a9f40d2c31c43cb")
        );
        // UP14 terminal MACs (CONFIRM→DONE→ACK), keyed by K1 over the 128 B transcript.
        assert_eq!(
            confirm_tag(&k1, &pk_host, &pk_periph, &nonce_host, &nonce_periph),
            arr::<16>("4e4c5ff286e30e55bd71c2efdc869f4e")
        );
        assert_eq!(
            done_tag(&k1, &pk_host, &pk_periph, &nonce_host, &nonce_periph),
            arr::<16>("08ba274f802d982844df255fe5a68be8")
        );
        assert_eq!(
            ack_tag(&k1, &pk_host, &pk_periph, &nonce_host, &nonce_periph),
            arr::<16>("1ec03c3d79e1f6f19b6a797d24142d72")
        );
        // UP18 Profile-A gen-free reconnect tag (the existing reconnect_tag helper).
        assert_eq!(
            reconnect_tag(&k1, &nonce_rc, &hive_id),
            arr::<16>("2f62edaaa469424d5a5da5630b06967b")
        );
        // UP14 non-contributory reject: an all-zero peer key forces Z=0 → None (→ bad_key abort).
        assert!(shared_secret(&sk_host, &[0u8; 32]).is_none());
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
        let z_host = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
        let z_periph = shared_secret(&sk_periph, &pk_host).expect("contributory Z");
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
        let z = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
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
        let z = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
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
        let z = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
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
        let z = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
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
        let z = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
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
        let z = shared_secret(&sk_host, &pk_periph).expect("contributory Z");
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
