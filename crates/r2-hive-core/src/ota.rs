//! OTA firmware-receive seam — the portable, `no_std` half of the over-the-air
//! firmware update receiver (R2-HIVE north-star: protocol + state machine in
//! hive-core; flash I/O and the transport are platform code).
//!
//! Wire protocol (matches the std reference
//! `r2-core/platforms/esp32/src/ota_tcp.rs`, port 21043):
//! - `CMD_START` (0x01): then a 36-byte preamble = `image_len:u32 LE` +
//!   `sha256:[u8;32]`, then the firmware byte stream.
//! - `CMD_QUERY` (0x02): device build info (handled by the platform layer).
//! - Reply: `[status:u8][msg_len:u16 LE][msg:utf8]`.
//!
//! Reply-status contract (composer, `OTA-REPLY-STATUS-CONTRACT.md`): status is
//! `0x00`=SUCCESS (msg `OK`, emitted only after sha256-match + write +
//! set-boot) or `0x01`=ERROR (msg = `<CODE>[ detail]`, CODE one of
//! [`OtaError`]). Status bytes stay 0x00/0x01 (R2-UPDATE RESP_OK/ERR); the
//! CODE rides in the message.
//!
//! What's portable here: command/status constants, preamble parsing, the
//! `image_len ≤ slot_capacity` TOO_BIG bound-check (done BEFORE any write —
//! DFR1195 is a 4 MB part with ~1.5 MB slots, smaller than the 8 MB devkitc),
//! the streaming SHA-256 + length accounting, and the SUCCESS/ERROR reply
//! encoding. The platform supplies the bytes (embassy-net TCP) and the
//! [`FirmwareSink`] (esp-storage / esp_ota_* on the device; a buffer on host).

use sha2::{Digest, Sha256};

/// TCP port for OTA firmware transfer and device query (`0x5233` + 1).
pub const OTA_PORT: u16 = 21043;

/// Command byte: begin a firmware transfer (preamble + stream follows).
pub const CMD_START: u8 = 0x01;
/// Command byte: query device build info.
pub const CMD_QUERY: u8 = 0x02;

/// Reply status byte: success (R2-UPDATE RESP_OK).
pub const STATUS_OK: u8 = 0x00;
/// Reply status byte: error (R2-UPDATE RESP_ERR); CODE rides in the message.
pub const STATUS_ERROR: u8 = 0x01;

/// Length of the `CMD_START` preamble after the command byte:
/// `image_len:u32 LE` (4) + `sha256:[u8;32]` (32).
pub const PREAMBLE_LEN: usize = 36;

/// The parsed `CMD_START` preamble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtaPreamble {
    /// Declared firmware image length in bytes.
    pub image_len: u32,
    /// Expected SHA-256 of the whole image.
    pub sha256: [u8; 32],
}

impl OtaPreamble {
    /// Parse the 36-byte preamble (`image_len:u32 LE` + `sha256:[u8;32]`).
    /// Returns [`OtaError::Short`] if the buffer is too small.
    pub fn parse(buf: &[u8]) -> Result<Self, OtaError> {
        if buf.len() < PREAMBLE_LEN {
            return Err(OtaError::Short);
        }
        let image_len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&buf[4..36]);
        Ok(Self { image_len, sha256 })
    }
}

/// OTA failure CODEs — the `<CODE>` half of an ERROR reply message
/// (OTA-REPLY-STATUS-CONTRACT). All map to reply status `0x01`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtaError {
    /// Preamble could not be read/parsed.
    Preamble,
    /// Declared image is larger than the inactive slot capacity. Checked
    /// BEFORE any flash write.
    TooBig,
    /// The staged image failed its header/magic validation at finalize.
    BadMagic,
    /// Streamed bytes did not match the declared SHA-256.
    ShaMismatch,
    /// A flash write failed.
    WriteFail,
    /// No inactive OTA slot is available to stage into / boot from.
    NoSlot,
    /// The stream ended before `image_len` bytes arrived (or a chunk would
    /// overrun it).
    Short,
}

impl OtaError {
    /// The stable CODE token for the reply message.
    pub fn code(self) -> &'static str {
        match self {
            OtaError::Preamble => "PREAMBLE",
            OtaError::TooBig => "TOO_BIG",
            OtaError::BadMagic => "BAD_MAGIC",
            OtaError::ShaMismatch => "SHA_MISMATCH",
            OtaError::WriteFail => "WRITE_FAIL",
            OtaError::NoSlot => "NO_SLOT",
            OtaError::Short => "SHORT",
        }
    }
}

/// Encode a reply frame `[status:u8][msg_len:u16 LE][msg:utf8]` into `out`.
/// Returns the number of bytes written, or `None` if `out` is too small. No
/// allocation (MCU-friendly).
pub fn encode_reply(status: u8, msg: &str, out: &mut [u8]) -> Option<usize> {
    let msg = msg.as_bytes();
    let total = 3 + msg.len();
    if out.len() < total || msg.len() > u16::MAX as usize {
        return None;
    }
    out[0] = status;
    out[1..3].copy_from_slice(&(msg.len() as u16).to_le_bytes());
    out[3..total].copy_from_slice(msg);
    Some(total)
}

/// Encode the SUCCESS reply (`status 0x00`, msg `OK`).
pub fn encode_ok(out: &mut [u8]) -> Option<usize> {
    encode_reply(STATUS_OK, "OK", out)
}

/// Encode an ERROR reply (`status 0x01`, msg = the failure CODE). Detail text,
/// if any, is the caller's concern; the CODE alone is contract-conformant.
pub fn encode_error(err: OtaError, out: &mut [u8]) -> Option<usize> {
    encode_reply(STATUS_ERROR, err.code(), out)
}

/// Flash-side sink for a staged firmware image — the OTA storage seam. The
/// device impl wraps `esp_ota_begin`/`esp_ota_write`/`esp_ota_end` +
/// `set_boot_partition` (esp-storage); a host impl can buffer to RAM/file for
/// testing. The receiver bound-checks against [`slot_capacity`] before
/// [`begin`].
///
/// [`slot_capacity`]: FirmwareSink::slot_capacity
/// [`begin`]: FirmwareSink::begin
pub trait FirmwareSink {
    /// Capacity of the inactive OTA slot in bytes. The receiver rejects images
    /// larger than this with [`OtaError::TooBig`] before any write.
    fn slot_capacity(&self) -> u32;

    /// Open the inactive slot for an image of `image_len` bytes
    /// (`esp_ota_begin`). [`OtaError::NoSlot`] if none is available.
    fn begin(&mut self, image_len: u32) -> Result<(), OtaError>;

    /// Append a chunk to the staged image (`esp_ota_write`).
    /// [`OtaError::WriteFail`] on flash error.
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), OtaError>;

    /// Validate the staged image header (`esp_ota_end` → [`OtaError::BadMagic`])
    /// and mark it as the boot slot (`set_boot_partition` →
    /// [`OtaError::NoSlot`]). After this returns `Ok`, the platform may reboot.
    fn finalize(&mut self) -> Result<(), OtaError>;

    /// Discard the staged image (`esp_ota_abort`). Best-effort; infallible.
    fn abort(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Awaiting `start`.
    Idle,
    /// `begin` succeeded; streaming chunks.
    Streaming,
    /// Terminal — succeeded or failed; no further input accepted.
    Closed,
}

/// Drives a single OTA transfer over a [`FirmwareSink`]: bound-check → begin →
/// stream (hashing) → verify SHA-256 → finalize. Platform-agnostic; the host
/// loop feeds it bytes from the transport and sends the [`reply`] it produces.
///
/// [`reply`]: OtaReceiver::finish
pub struct OtaReceiver<S: FirmwareSink> {
    sink: S,
    hasher: Sha256,
    expected_len: u32,
    expected_sha: [u8; 32],
    received: u32,
    state: State,
}

impl<S: FirmwareSink> OtaReceiver<S> {
    /// Wrap a sink, ready to receive one image.
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            hasher: Sha256::new(),
            expected_len: 0,
            expected_sha: [0u8; 32],
            received: 0,
            state: State::Idle,
        }
    }

    /// Bytes accepted so far.
    pub fn received(&self) -> u32 {
        self.received
    }

    /// Begin a transfer from a parsed preamble: bound-check the declared size
    /// against the slot ([`OtaError::TooBig`]) BEFORE opening the slot, then
    /// `begin`. On any error the receiver is closed and the sink aborted.
    pub fn start(&mut self, preamble: OtaPreamble) -> Result<(), OtaError> {
        if self.state != State::Idle {
            return self.fail(OtaError::Preamble);
        }
        // TOO_BIG must be decided before any write (DFR1195 4 MB / ~1.5 MB slot).
        if preamble.image_len > self.sink.slot_capacity() {
            return self.fail(OtaError::TooBig);
        }
        if let Err(e) = self.sink.begin(preamble.image_len) {
            return self.fail(e);
        }
        self.expected_len = preamble.image_len;
        self.expected_sha = preamble.sha256;
        self.received = 0;
        self.hasher = Sha256::new();
        self.state = State::Streaming;
        Ok(())
    }

    /// Feed a chunk of firmware bytes: written to the sink and folded into the
    /// running hash. Rejects bytes that would overrun the declared length
    /// ([`OtaError::Short`] is reserved for *under*-run; overrun is a protocol
    /// fault reported as [`OtaError::WriteFail`]). On error the receiver is
    /// closed and the sink aborted.
    pub fn feed(&mut self, chunk: &[u8]) -> Result<(), OtaError> {
        if self.state != State::Streaming {
            return self.fail(OtaError::WriteFail);
        }
        let new_total = self.received as u64 + chunk.len() as u64;
        if new_total > self.expected_len as u64 {
            return self.fail(OtaError::WriteFail);
        }
        if let Err(e) = self.sink.write_chunk(chunk) {
            return self.fail(e);
        }
        self.hasher.update(chunk);
        self.received += chunk.len() as u32;
        Ok(())
    }

    /// Complete the transfer: require the full image, verify SHA-256, then
    /// finalize (header check + set-boot). Returns `Ok` only when the image is
    /// staged and bootable — the contract's SUCCESS precondition. On any error
    /// the receiver is closed and the sink aborted.
    pub fn finish(&mut self) -> Result<(), OtaError> {
        if self.state != State::Streaming {
            return self.fail(OtaError::Short);
        }
        if self.received != self.expected_len {
            return self.fail(OtaError::Short);
        }
        let digest = core::mem::replace(&mut self.hasher, Sha256::new()).finalize();
        if digest.as_slice() != self.expected_sha {
            return self.fail(OtaError::ShaMismatch);
        }
        if let Err(e) = self.sink.finalize() {
            return self.fail(e);
        }
        self.state = State::Closed;
        Ok(())
    }

    /// Borrow the underlying sink (e.g. for the platform to reboot after a
    /// successful [`finish`]).
    pub fn sink(&self) -> &S {
        &self.sink
    }

    fn fail(&mut self, err: OtaError) -> Result<(), OtaError> {
        self.sink.abort();
        self.state = State::Closed;
        Err(err)
    }
}

// ───────────────────────── Tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// RAM-backed sink that records the staged image and can inject failures.
    struct MockSink {
        capacity: u32,
        staged: Vec<u8>,
        begun: bool,
        finalized: bool,
        aborted: bool,
        fail_begin: Option<OtaError>,
        fail_write: Option<OtaError>,
        fail_finalize: Option<OtaError>,
    }

    impl MockSink {
        fn with_capacity(capacity: u32) -> Self {
            Self {
                capacity,
                staged: Vec::new(),
                begun: false,
                finalized: false,
                aborted: false,
                fail_begin: None,
                fail_write: None,
                fail_finalize: None,
            }
        }
    }

    impl FirmwareSink for MockSink {
        fn slot_capacity(&self) -> u32 {
            self.capacity
        }
        fn begin(&mut self, _image_len: u32) -> Result<(), OtaError> {
            if let Some(e) = self.fail_begin {
                return Err(e);
            }
            self.begun = true;
            Ok(())
        }
        fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), OtaError> {
            if let Some(e) = self.fail_write {
                return Err(e);
            }
            self.staged.extend_from_slice(chunk);
            Ok(())
        }
        fn finalize(&mut self) -> Result<(), OtaError> {
            if let Some(e) = self.fail_finalize {
                return Err(e);
            }
            self.finalized = true;
            Ok(())
        }
        fn abort(&mut self) {
            self.aborted = true;
        }
    }

    fn sha_of(data: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(data);
        let d = h.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&d);
        out
    }

    fn preamble_bytes(len: u32, sha: [u8; 32]) -> [u8; PREAMBLE_LEN] {
        let mut buf = [0u8; PREAMBLE_LEN];
        buf[0..4].copy_from_slice(&len.to_le_bytes());
        buf[4..36].copy_from_slice(&sha);
        buf
    }

    #[test]
    fn happy_path_stages_image_and_finalizes() {
        let image = b"firmware-image-bytes-0123456789".to_vec();
        let pre = OtaPreamble::parse(&preamble_bytes(image.len() as u32, sha_of(&image))).unwrap();

        let mut rx = OtaReceiver::new(MockSink::with_capacity(1_500_000));
        rx.start(pre).expect("start");
        // Feed in two chunks to exercise streaming.
        rx.feed(&image[..10]).expect("chunk 1");
        rx.feed(&image[10..]).expect("chunk 2");
        rx.finish().expect("finish ok");

        assert_eq!(rx.received(), image.len() as u32);
        assert_eq!(rx.sink().staged, image);
        assert!(rx.sink().finalized);
        assert!(!rx.sink().aborted);
    }

    #[test]
    fn too_big_is_rejected_before_begin() {
        let pre = OtaPreamble {
            image_len: 2_000_000,
            sha256: [0u8; 32],
        };
        let mut rx = OtaReceiver::new(MockSink::with_capacity(1_500_000));
        assert_eq!(rx.start(pre), Err(OtaError::TooBig));
        // begin() must NOT have been called, and the sink aborted.
        assert!(!rx.sink().begun);
        assert!(rx.sink().aborted);
    }

    #[test]
    fn sha_mismatch_aborts() {
        let image = b"the-real-image".to_vec();
        let mut wrong = sha_of(&image);
        wrong[0] ^= 0xFF;
        let pre = OtaPreamble {
            image_len: image.len() as u32,
            sha256: wrong,
        };
        let mut rx = OtaReceiver::new(MockSink::with_capacity(1_000_000));
        rx.start(pre).unwrap();
        rx.feed(&image).unwrap();
        assert_eq!(rx.finish(), Err(OtaError::ShaMismatch));
        assert!(!rx.sink().finalized);
        assert!(rx.sink().aborted);
    }

    #[test]
    fn underrun_is_short() {
        let image = b"0123456789".to_vec();
        let pre = OtaPreamble {
            image_len: image.len() as u32,
            sha256: sha_of(&image),
        };
        let mut rx = OtaReceiver::new(MockSink::with_capacity(1_000_000));
        rx.start(pre).unwrap();
        rx.feed(&image[..4]).unwrap();
        assert_eq!(rx.finish(), Err(OtaError::Short));
        assert!(rx.sink().aborted);
    }

    #[test]
    fn overrun_is_rejected() {
        let pre = OtaPreamble {
            image_len: 4,
            sha256: [0u8; 32],
        };
        let mut rx = OtaReceiver::new(MockSink::with_capacity(1_000_000));
        rx.start(pre).unwrap();
        assert_eq!(rx.feed(b"toolong"), Err(OtaError::WriteFail));
        assert!(rx.sink().aborted);
    }

    #[test]
    fn write_failure_propagates() {
        let image = b"abcd".to_vec();
        let pre = OtaPreamble {
            image_len: image.len() as u32,
            sha256: sha_of(&image),
        };
        let mut sink = MockSink::with_capacity(1_000_000);
        sink.fail_write = Some(OtaError::WriteFail);
        let mut rx = OtaReceiver::new(sink);
        rx.start(pre).unwrap();
        assert_eq!(rx.feed(&image), Err(OtaError::WriteFail));
        assert!(rx.sink().aborted);
    }

    #[test]
    fn no_slot_at_begin() {
        let mut sink = MockSink::with_capacity(1_000_000);
        sink.fail_begin = Some(OtaError::NoSlot);
        let pre = OtaPreamble {
            image_len: 100,
            sha256: [0u8; 32],
        };
        let mut rx = OtaReceiver::new(sink);
        assert_eq!(rx.start(pre), Err(OtaError::NoSlot));
        assert!(rx.sink().aborted);
    }

    #[test]
    fn bad_magic_at_finalize() {
        let image = b"img".to_vec();
        let mut sink = MockSink::with_capacity(1_000_000);
        sink.fail_finalize = Some(OtaError::BadMagic);
        let pre = OtaPreamble {
            image_len: image.len() as u32,
            sha256: sha_of(&image),
        };
        let mut rx = OtaReceiver::new(sink);
        rx.start(pre).unwrap();
        rx.feed(&image).unwrap();
        assert_eq!(rx.finish(), Err(OtaError::BadMagic));
        assert!(rx.sink().aborted);
    }

    #[test]
    fn preamble_parse_round_trips_and_rejects_short() {
        let sha = sha_of(b"x");
        let buf = preamble_bytes(0xDEAD_BEEF, sha);
        let pre = OtaPreamble::parse(&buf).unwrap();
        assert_eq!(pre.image_len, 0xDEAD_BEEF);
        assert_eq!(pre.sha256, sha);
        assert_eq!(OtaPreamble::parse(&buf[..10]), Err(OtaError::Short));
    }

    #[test]
    fn reply_encoding_matches_wire_shape() {
        let mut out = [0u8; 64];

        let n = encode_ok(&mut out).unwrap();
        assert_eq!(out[0], STATUS_OK);
        assert_eq!(u16::from_le_bytes([out[1], out[2]]), 2);
        assert_eq!(&out[3..n], b"OK");

        let n = encode_error(OtaError::TooBig, &mut out).unwrap();
        assert_eq!(out[0], STATUS_ERROR);
        assert_eq!(u16::from_le_bytes([out[1], out[2]]), 7);
        assert_eq!(&out[3..n], b"TOO_BIG");

        // Too-small buffer is reported, not panicked.
        let mut tiny = [0u8; 2];
        assert_eq!(encode_ok(&mut tiny), None);
    }

    #[test]
    fn every_error_has_a_nonempty_code() {
        for e in [
            OtaError::Preamble,
            OtaError::TooBig,
            OtaError::BadMagic,
            OtaError::ShaMismatch,
            OtaError::WriteFail,
            OtaError::NoSlot,
            OtaError::Short,
        ] {
            assert!(!e.code().is_empty());
        }
    }
}
