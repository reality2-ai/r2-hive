//! Browser device-credential issuance and cookie verification for
//! web plugins (R2-PLUGIN §13.5).
//!
//! v0.1 implements the minimum the security model requires for static
//! GETs: a per-hive HMAC-SHA256 cookie tied to a device id minted at
//! provision time. The Ed25519 keypair half of §13.5 — the per-frame
//! HMAC envelope on WS channels — lives in the WS channel module
//! (Phase 3d follow-up) once channels are wired.
//!
//! Cookie format on the wire:
//!
//! ```text
//! r2_web_session=<base64url(device_id_16 || expiry_be_u64 || mac_32)>
//! ```
//!
//! `mac` = HMAC-SHA256(signing_key, `device_id_16 || expiry_be_u64`).
//! Cookie attributes set by the issuer: `Secure; HttpOnly;
//! SameSite=Strict; Path=/`. Default TTL: 24 hours.
//!
//! Provision word codes (one-time, 1-hour TTL by default) live in a
//! ledger separate from the TG-join ledger so a TG-join code can't be
//! replayed as a browser-provision code and vice versa.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// 16-byte opaque browser device identifier.
pub type DeviceId = [u8; 16];

const COOKIE_NAME: &str = "r2_web_session";
const COOKIE_BODY_LEN: usize = 16 + 8 + 32; // device_id + expiry + mac

const DEFAULT_PROVISION_TTL_SECS: u64 = 60 * 60; // 1 hour
const DEFAULT_COOKIE_TTL_SECS: u64 = 24 * 60 * 60; // 24 hours

/// A provisioned browser device.
#[derive(Debug, Clone)]
pub struct DeviceCredential {
    /// Random 16-byte identifier minted at provision time.
    pub device_id: DeviceId,
    /// Issue time (seconds since UNIX epoch).
    pub issued_at_secs: u64,
}

#[derive(Debug, Clone)]
struct ProvisionCode {
    expires_at_secs: u64,
    used: bool,
}

/// Why an auth check failed.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// No `r2_web_session` cookie was present on the request.
    #[error("no session cookie")]
    NoCookie,
    /// Cookie failed base64 decode or had the wrong length.
    #[error("cookie malformed")]
    BadCookie,
    /// Cookie HMAC did not match the recomputed value.
    #[error("cookie signature invalid")]
    BadSignature,
    /// Cookie's expiry is in the past.
    #[error("cookie expired")]
    Expired,
    /// Cookie was valid, but the browser device is no longer active.
    #[error("browser device revoked")]
    Revoked,
}

/// Why a provision-code redemption failed.
#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
    /// The supplied word code was never minted, or has been deleted.
    #[error("unknown word code")]
    UnknownCode,
    /// The code was minted but already redeemed once.
    #[error("word code already used")]
    AlreadyUsed,
    /// The code was minted but its TTL has elapsed.
    #[error("word code expired")]
    Expired,
}

/// Web-plugin browser auth registry. One instance per hive.
pub struct WebAuth {
    signing_key: [u8; 32],
    devices: RwLock<HashMap<DeviceId, DeviceCredential>>,
    provision_codes: RwLock<HashMap<String, ProvisionCode>>,
}

impl WebAuth {
    /// Build with an HMAC key derived from the hive's master secret
    /// (see `MasterSecret::derive_web_auth_key`).
    pub fn new(signing_key: [u8; 32]) -> Self {
        Self {
            signing_key,
            devices: RwLock::new(HashMap::new()),
            provision_codes: RwLock::new(HashMap::new()),
        }
    }

    /// Mint a fresh provision word code with the default 1-hour TTL.
    /// Returns the code in canonical hyphenated form (e.g.
    /// `"calm-orbit-cedar"`).
    pub fn mint_provision_code(&self) -> String {
        self.mint_provision_code_with_ttl(DEFAULT_PROVISION_TTL_SECS)
    }

    /// Variant with a caller-chosen TTL (for tests).
    pub fn mint_provision_code_with_ttl(&self, ttl_secs: u64) -> String {
        let words = random_three_word_code();
        let now = unix_now();
        let entry = ProvisionCode {
            expires_at_secs: now.saturating_add(ttl_secs),
            used: false,
        };
        self.provision_codes
            .write()
            .expect("provision lock")
            .insert(words.clone(), entry);
        words
    }

    /// Redeem a word code: mint a [`DeviceCredential`] and a cookie
    /// string that the caller MUST set on the response with
    /// `Set-Cookie: <returned_value>`.
    pub fn redeem_provision_code(
        &self,
        words: &str,
    ) -> Result<(DeviceCredential, String), ProvisionError> {
        let now = unix_now();
        {
            let mut codes = self.provision_codes.write().expect("provision lock");
            let entry = codes.get_mut(words).ok_or(ProvisionError::UnknownCode)?;
            if entry.used {
                return Err(ProvisionError::AlreadyUsed);
            }
            if entry.expires_at_secs <= now {
                return Err(ProvisionError::Expired);
            }
            entry.used = true;
        }

        let mut device_id = [0u8; 16];
        getrandom::getrandom(&mut device_id).expect("getrandom");
        let cred = DeviceCredential {
            device_id,
            issued_at_secs: now,
        };
        self.devices
            .write()
            .expect("devices lock")
            .insert(device_id, cred.clone());

        let cookie = self.issue_cookie(device_id, DEFAULT_COOKIE_TTL_SECS);
        Ok((cred, cookie))
    }

    /// Build the `Set-Cookie` value for a device id with the requested TTL.
    pub fn issue_cookie(&self, device_id: DeviceId, ttl_secs: u64) -> String {
        let now = unix_now();
        let expiry = now.saturating_add(ttl_secs);
        let body = encode_cookie_body(&self.signing_key, device_id, expiry);
        format!(
            "{name}={body}; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age={ttl_secs}",
            name = COOKIE_NAME,
            body = body,
            ttl_secs = ttl_secs,
        )
    }

    /// Verify that a `Cookie:` header value carries a valid session.
    /// `header_value` is the full `Cookie:` field as the browser sent
    /// it, including any other unrelated cookies.
    pub fn verify_cookie_header(&self, header_value: &str) -> Result<DeviceId, AuthError> {
        let body = extract_cookie(header_value).ok_or(AuthError::NoCookie)?;
        let device_id = verify_cookie_body(&self.signing_key, body)?;
        if !self.is_known_device(&device_id) {
            return Err(AuthError::Revoked);
        }
        Ok(device_id)
    }

    /// List currently-active devices (for status / debug surfaces).
    pub fn devices(&self) -> Vec<DeviceCredential> {
        self.devices
            .read()
            .expect("devices lock")
            .values()
            .cloned()
            .collect()
    }

    /// Revoke a device. Subsequent verifies on its cookie fail even
    /// though the cookie still passes cryptographic verification.
    pub fn revoke_device(&self, device_id: &DeviceId) {
        self.devices
            .write()
            .expect("devices lock")
            .remove(device_id);
    }

    /// Returns true if the device id is currently in the active ledger.
    pub fn is_known_device(&self, device_id: &DeviceId) -> bool {
        self.devices
            .read()
            .expect("devices lock")
            .contains_key(device_id)
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn encode_cookie_body(key: &[u8; 32], device_id: DeviceId, expiry: u64) -> String {
    let mut payload = [0u8; COOKIE_BODY_LEN];
    payload[..16].copy_from_slice(&device_id);
    payload[16..24].copy_from_slice(&expiry.to_be_bytes());
    let mac = compute_mac(key, &payload[..24]);
    payload[24..].copy_from_slice(&mac);
    URL_SAFE_NO_PAD.encode(payload)
}

fn verify_cookie_body(key: &[u8; 32], body: &str) -> Result<DeviceId, AuthError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(body.as_bytes())
        .map_err(|_| AuthError::BadCookie)?;
    if bytes.len() != COOKIE_BODY_LEN {
        return Err(AuthError::BadCookie);
    }
    let expected_mac = compute_mac(key, &bytes[..24]);
    if !constant_time_eq(&expected_mac, &bytes[24..]) {
        return Err(AuthError::BadSignature);
    }
    let mut expiry_bytes = [0u8; 8];
    expiry_bytes.copy_from_slice(&bytes[16..24]);
    let expiry = u64::from_be_bytes(expiry_bytes);
    if expiry <= unix_now() {
        return Err(AuthError::Expired);
    }
    let mut device_id = [0u8; 16];
    device_id.copy_from_slice(&bytes[..16]);
    Ok(device_id)
}

fn extract_cookie(header_value: &str) -> Option<&str> {
    for pair in header_value.split(';') {
        let pair = pair.trim();
        if let Some(rest) = pair.strip_prefix(&format!("{}=", COOKIE_NAME)) {
            return Some(rest);
        }
    }
    None
}

fn compute_mac(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(message);
    let result = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
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

/// 256-word list (BIP39-shaped — short, low-collision, no homophones).
/// 3 words → 256³ ≈ 1.7e7 codes; the 1-hour TTL plus single-use makes
/// online guessing infeasible. The list is intentionally small and
/// hand-checked for ambiguity rather than the full BIP39 2048 — the
/// shorter words are easier to dictate over voice.
const WORDS: &[&str] = &[
    "amber", "apple", "april", "arrow", "atlas", "azure", "banjo", "barn",
    "basil", "beach", "berry", "birch", "bison", "blaze", "bloom", "blue",
    "boat", "bold", "bone", "book", "boot", "borax", "brave", "bread",
    "brick", "brisk", "broad", "brook", "brown", "bud", "bulb", "calm",
    "candy", "canon", "cargo", "carve", "cedar", "chalk", "cheer", "chess",
    "chest", "choir", "cider", "cinder", "clay", "clear", "cliff", "cloud",
    "clove", "coast", "cobalt", "coin", "comet", "cool", "coral", "cosmic",
    "court", "cove", "craft", "crane", "crisp", "crown", "crux", "crystal",
    "cube", "cumin", "daisy", "dance", "delta", "denim", "depth", "diary",
    "dome", "doom", "drift", "drum", "duet", "dust", "early", "earth",
    "echo", "edge", "ember", "epic", "ether", "fable", "fair", "farm",
    "feast", "felt", "fern", "fig", "finch", "fjord", "flame", "flax",
    "flint", "flora", "flute", "foam", "fog", "forge", "frost", "gem",
    "gentle", "ginger", "glade", "glass", "glow", "gold", "grace", "grain",
    "grape", "grasp", "grass", "grin", "grit", "grove", "haiku", "hail",
    "halo", "harbor", "hare", "harp", "haste", "haven", "hazel", "heart",
    "hill", "hum", "hush", "ibis", "icicle", "ink", "iris", "ivory",
    "ivy", "jade", "jasper", "jewel", "joy", "kale", "keep", "kilo",
    "kind", "knot", "lake", "lapis", "larch", "lark", "lava", "lemon",
    "linen", "lion", "lotus", "lucid", "lunar", "lyric", "magma", "maple",
    "marble", "mauve", "meadow", "mesa", "mica", "mint", "mist", "moon",
    "moss", "muse", "myth", "navy", "nest", "noble", "nomad", "north",
    "nova", "oak", "oasis", "ocean", "olive", "onyx", "opal", "orbit",
    "otter", "owl", "ozone", "pearl", "peak", "petal", "pine", "pith",
    "plum", "pool", "poppy", "prism", "pure", "quail", "quartz", "quay",
    "quill", "quilt", "rain", "raven", "reed", "reef", "ridge", "rift",
    "ripe", "river", "rose", "ruby", "rust", "saffron", "sage", "sand",
    "satin", "scope", "sea", "seed", "shade", "sharp", "shell", "shine",
    "silk", "sky", "slate", "smile", "snow", "soft", "solar", "song",
    "spark", "spire", "spruce", "star", "steam", "stone", "storm", "swift",
    "tame", "thorn", "tide", "tiger", "topaz", "torch", "tower", "trail",
    "trout", "truth", "tulip", "vault", "vega", "verge", "violet", "wheat",
];

fn random_three_word_code() -> String {
    let mut buf = [0u8; 6]; // 3 × u16 indexes
    getrandom::getrandom(&mut buf).expect("getrandom");
    let pick = |i: usize| -> &'static str {
        let idx = u16::from_le_bytes([buf[i * 2], buf[i * 2 + 1]]) as usize;
        WORDS[idx % WORDS.len()]
    };
    format!("{}-{}-{}", pick(0), pick(1), pick(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth() -> WebAuth {
        WebAuth::new([0xAB; 32])
    }

    #[test]
    fn mint_redeem_round_trip() {
        let a = auth();
        let code = a.mint_provision_code_with_ttl(60);
        let (cred, cookie) = a.redeem_provision_code(&code).expect("redeem");
        assert!(a.is_known_device(&cred.device_id));
        let body = extract_cookie(&cookie).expect("cookie body in Set-Cookie");
        assert_eq!(
            verify_cookie_body(&[0xAB; 32], body).unwrap(),
            cred.device_id
        );
    }

    #[test]
    fn cookie_with_wrong_key_rejected() {
        let a = auth();
        let cookie = a.issue_cookie([1u8; 16], 60);
        let body = extract_cookie(&cookie).unwrap();
        let err = verify_cookie_body(&[0xCD; 32], body).unwrap_err();
        assert!(matches!(err, AuthError::BadSignature));
    }

    #[test]
    fn double_redeem_rejected() {
        let a = auth();
        let code = a.mint_provision_code_with_ttl(60);
        let _ = a.redeem_provision_code(&code).unwrap();
        let err = a.redeem_provision_code(&code).unwrap_err();
        assert!(matches!(err, ProvisionError::AlreadyUsed));
    }

    #[test]
    fn expired_code_rejected() {
        let a = auth();
        let code = a.mint_provision_code_with_ttl(0);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let err = a.redeem_provision_code(&code).unwrap_err();
        assert!(matches!(err, ProvisionError::Expired));
    }

    #[test]
    fn unknown_code_rejected() {
        let a = auth();
        let err = a.redeem_provision_code("not-a-real-code").unwrap_err();
        assert!(matches!(err, ProvisionError::UnknownCode));
    }

    #[test]
    fn extract_cookie_picks_the_right_one() {
        let header = "other=foo; r2_web_session=ABCDEF; another=bar";
        assert_eq!(extract_cookie(header), Some("ABCDEF"));
    }

    #[test]
    fn no_cookie_returns_no_cookie_error() {
        let a = auth();
        let err = a.verify_cookie_header("other=value").unwrap_err();
        assert!(matches!(err, AuthError::NoCookie));
    }

    #[test]
    fn revoked_device_cookie_is_rejected() {
        let a = auth();
        let code = a.mint_provision_code_with_ttl(60);
        let (cred, cookie) = a.redeem_provision_code(&code).expect("redeem");
        let pair = cookie.split(';').next().unwrap();
        assert_eq!(a.verify_cookie_header(pair).unwrap(), cred.device_id);

        a.revoke_device(&cred.device_id);
        let err = a.verify_cookie_header(pair).unwrap_err();
        assert!(matches!(err, AuthError::Revoked));
    }
}
