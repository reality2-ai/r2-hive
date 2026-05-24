//! CDC-ACM serial I/O wrapper for R2-USB v2 (Phase USB-3a).
//!
//! Opens a `/dev/ttyACM*` (or any compatible CDC-ACM character device)
//! in raw, non-blocking mode and presents an `AsyncRead + AsyncWrite`
//! handle that integrates with `tokio`. The protocol layer in
//! [`crate::usb::UsbSession`] consumes the bytes and produces
//! [`UsbEvent`](crate::usb::UsbEvent)s; this module owns the wire I/O
//! and nothing else.
//!
//! [`run_session`] is the canonical loop: feeds inbound bytes into
//! the session, flushes outbound bytes back to the device, surfaces
//! events on a channel, and listens for operator confirmations on a
//! control channel.
//!
//! # Platform scope
//!
//! Linux-only — the termios bit layout and the `cfmakeraw` shim are
//! platform-specific. macOS/Windows hosts run `r2-hive` against
//! Internet/BLE transports without USB-attached peripherals; if/when
//! USB-attached peripherals on macOS/Windows become a goal, this
//! module gets per-platform `#[cfg]` blocks.

#![cfg(target_os = "linux")]

use std::fs::OpenOptions;
use std::io;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use tokio::io::{unix::AsyncFd, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::sync::mpsc;

use crate::usb::{SessionState, UsbEvent, UsbSession};

/// Tokio-async wrapper around a CDC-ACM serial device in raw mode.
///
/// Doesn't impose its own buffering — `tokio::io::AsyncRead` /
/// `AsyncWrite` are the surface, so callers can wrap with
/// `BufReader` / `BufWriter` if they want, or feed bytes straight
/// through to a [`UsbSession`].
pub struct RawSerial {
    fd: AsyncFd<OwnedFd>,
}

impl RawSerial {
    /// Open a CDC-ACM device, configure termios for raw byte I/O, and
    /// register it with the tokio runtime for async readiness
    /// notifications.
    ///
    /// Termios settings applied:
    /// - `cfmakeraw` (no canonical mode, no echo, no signals, no input
    ///   processing).
    /// - `VMIN=1, VTIME=0` so reads return as soon as a single byte is
    ///   available.
    /// - Baud rate is left untouched — CDC-ACM ignores it on the wire.
    /// - Local mode (`CLOCAL`) and reader (`CREAD`) bits are forced on
    ///   so a missing modem-control signal can't block reads.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
            .open(path)?;
        let fd: OwnedFd = file.into();
        configure_termios(&fd)?;
        let fd = AsyncFd::new(fd)?;
        Ok(Self { fd })
    }

    fn raw_fd(&self) -> RawFd {
        self.fd.get_ref().as_raw_fd()
    }
}

fn configure_termios(fd: &OwnedFd) -> io::Result<()> {
    // SAFETY: fd is a valid open file descriptor; termios is
    // POD-style; cfmakeraw only writes its argument; tcgetattr/tcsetattr
    // are documented thread-safe on the same fd.
    unsafe {
        let mut t: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd.as_raw_fd(), &mut t) != 0 {
            return Err(io::Error::last_os_error());
        }
        libc::cfmakeraw(&mut t);
        // Force CREAD (reader on) and CLOCAL (ignore modem control)
        // so we don't block on DCD or similar signals that CDC-ACM
        // doesn't drive.
        t.c_cflag |= libc::CREAD | libc::CLOCAL;
        t.c_cc[libc::VMIN] = 1;
        t.c_cc[libc::VTIME] = 0;
        if libc::tcsetattr(fd.as_raw_fd(), libc::TCSANOW, &t) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

impl AsyncRead for RawSerial {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let raw = self.raw_fd();
        loop {
            let mut guard = ready!(self.fd.poll_read_ready(cx))?;
            let unfilled = buf.initialize_unfilled();
            let result = guard.try_io(|_| -> io::Result<usize> {
                // SAFETY: raw is a valid open fd; `unfilled` is a
                // valid mutable byte buffer; libc::read is the
                // documented sys-call for non-blocking reads.
                let n = unsafe {
                    libc::read(raw, unfilled.as_mut_ptr() as *mut libc::c_void, unfilled.len())
                };
                if n < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            });
            match result {
                Ok(Ok(n)) => {
                    buf.advance(n);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(e)) => return Poll::Ready(Err(e)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for RawSerial {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let raw = self.raw_fd();
        loop {
            let mut guard = ready!(self.fd.poll_write_ready(cx))?;
            let result = guard.try_io(|_| -> io::Result<usize> {
                // SAFETY: raw is a valid open fd; `buf` is a valid
                // byte slice; libc::write is the documented
                // non-blocking write sys-call.
                let n = unsafe {
                    libc::write(raw, buf.as_ptr() as *const libc::c_void, buf.len())
                };
                if n < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            });
            match result {
                Ok(r) => return Poll::Ready(r),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Serial port writes are unbuffered at this layer; nothing to flush.
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // No graceful-close protocol on a serial port; close on Drop.
        Poll::Ready(Ok(()))
    }
}

// ---------------------------------------------------------------------
// Session driver
// ---------------------------------------------------------------------

/// Operator-side control input to a running session, plus internal
/// out-of-band asks (host wants to send a wire frame through this
/// dongle's radio).
#[derive(Debug, Clone)]
pub enum SessionControl {
    /// Operator confirmed the SAS code shown in a [`UsbEvent::PairingPrompt`]
    /// — proceed with [`UsbSession::user_confirms`].
    UserConfirms,
    /// Operator rejected the SAS code or the §6.4 60-second confirmation
    /// timeout elapsed — abort with [`UsbSession::user_aborts`].
    UserAborts,
    /// Send an R2-WIRE frame out via this dongle's `local_id`
    /// transport (Phase USB-5). The session wraps the bytes in the
    /// R2-USB §3.5 type-byte framing.
    SendWireFrame { local_id: u8, bytes: Vec<u8> },
}

/// Drive a [`UsbSession`] over an `AsyncRead + AsyncWrite` transport.
///
/// Generic over the transport so tests can use `tokio::io::duplex`
/// pairs and production uses [`RawSerial`].
///
/// Loop semantics:
///
/// 1. Send the initial v2 SYNC frame.
/// 2. Repeatedly: flush any outbound bytes the session has produced;
///    `select!` on (a) inbound bytes from the transport and (b) a
///    control message from the operator surface.
/// 3. Inbound bytes are fed into [`UsbSession::ingest_bytes`]. Each
///    [`UsbEvent`] is forwarded on `event_tx`.
/// 4. The loop exits when the transport closes (read returns 0), the
///    session transitions to [`SessionState::Closed`], or
///    `event_tx`'s receiver is dropped.
///
/// Returns the final session state. On clean disconnect, that's
/// [`SessionState::Closed`] (or `Active` if the peripheral simply
/// stopped sending).
pub async fn run_session<S>(
    transport: S,
    mut session: UsbSession,
    event_tx: mpsc::Sender<UsbEvent>,
    mut control_rx: mpsc::Receiver<SessionControl>,
) -> io::Result<SessionState>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    tokio::pin!(transport);
    session.send_sync();

    let mut buf = [0u8; 4096];
    loop {
        // Drain any pending outbound bytes the session has produced.
        let outbox = session.take_outbound();
        if !outbox.is_empty() {
            transport.write_all(&outbox).await?;
        }
        if matches!(session.state(), SessionState::Closed) {
            return Ok(SessionState::Closed);
        }

        tokio::select! {
            // Inbound bytes from the peripheral.
            read = transport.read(&mut buf) => {
                let n = read?;
                if n == 0 {
                    // Peripheral disconnected (cable unplugged or
                    // firmware reset). Session may have been mid-flow;
                    // surface its final state.
                    return Ok(session.state());
                }
                let evs = session.ingest_bytes(&buf[..n]);
                for ev in evs {
                    if event_tx.send(ev).await.is_err() {
                        // Listener dropped; tear down gracefully.
                        return Ok(session.state());
                    }
                }
            }
            // Operator control input (user confirm / abort, or
            // host-driven wire-frame outbound).
            ctrl = control_rx.recv() => {
                match ctrl {
                    Some(SessionControl::UserConfirms) => session.user_confirms(),
                    Some(SessionControl::UserAborts) => session.user_aborts(),
                    Some(SessionControl::SendWireFrame { local_id, bytes }) => {
                        let _ = session.send_wire_frame(local_id, &bytes);
                    }
                    None => {
                        // Control sender dropped — tear down gracefully.
                        return Ok(session.state());
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// Tests — drive a session over a tokio duplex pair (no hardware needed)
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usb::{InMemoryLinkKeyStore, UsbEvent};
    use std::sync::Arc;
    use tokio::io::duplex;

    fn hex(s: &str) -> Vec<u8> {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// Smoke test: a session over a duplex pair consumes the
    /// peripheral's SYNC + CAPS frames and emits a Paired event in
    /// AutoPairUnsafe mode. Verifies the run_session loop's
    /// ingest/flush/event-forward plumbing is correct end-to-end.
    #[tokio::test]
    async fn run_session_drives_session_over_duplex() {
        let (host_side, mut peripheral_side) = duplex(4096);

        let mut session = UsbSession::new_auto_pair_unsafe();
        // Inject deterministic test inputs (not strictly needed for
        // auto-pair, but prevents getrandom from dirtying tests).
        session.inject_test_keys([0x11; 32], [0x33; 32], [0x66; 16]);

        let (event_tx, mut event_rx) = mpsc::channel(32);
        let (_control_tx, control_rx) = mpsc::channel(4);

        // Driver task: reads from host_side, writes to host_side.
        let driver = tokio::spawn(async move {
            run_session(host_side, session, event_tx, control_rx).await
        });

        // Peripheral simulator: receive host's SYNC, send back its own.
        let mut buf = [0u8; 64];
        let n = peripheral_side.read(&mut buf).await.unwrap();
        assert!(n >= 6, "host SYNC should be 6 bytes (got {n})");
        assert_eq!(&buf[..6], &hex("040032520200")[..]);

        // Send peripheral's v2 SYNC reply.
        peripheral_side
            .write_all(&hex("040032520200"))
            .await
            .unwrap();

        // Send a minimal CAPS frame (TV3-style, lora_us915, hive_id_bytes all-0xAA).
        // Frame: length-prefix(2) || 0xFE (CAPS) || cbor map.
        let mut caps_body = Vec::new();
        caps_body.push(0xFE);
        caps_body.push(0xA4); // map(4)
        caps_body.push(0x00);
        caps_body.push(0x50);
        caps_body.extend_from_slice(&[0xAA; 16]);
        caps_body.push(0x01);
        caps_body.push(0x63);
        caps_body.extend_from_slice(b"sim");
        caps_body.push(0x02);
        caps_body.push(0x01);
        caps_body.push(0x03);
        caps_body.push(0x81);
        caps_body.push(0xA2);
        caps_body.push(0x00);
        caps_body.push(0x00);
        caps_body.push(0x01);
        caps_body.push(0x01);
        let len = (caps_body.len() as u16).to_le_bytes();
        let mut caps_frame = Vec::new();
        caps_frame.extend_from_slice(&len);
        caps_frame.extend_from_slice(&caps_body);
        peripheral_side.write_all(&caps_frame).await.unwrap();

        // Collect events for a moment.
        let mut got_sync = false;
        let mut got_caps = false;
        let mut got_paired = false;
        for _ in 0..6 {
            match tokio::time::timeout(
                std::time::Duration::from_millis(200),
                event_rx.recv(),
            )
            .await
            {
                Ok(Some(UsbEvent::SyncNegotiated { version, .. })) => {
                    assert_eq!(version, 2);
                    got_sync = true;
                }
                Ok(Some(UsbEvent::Caps(_))) => got_caps = true,
                Ok(Some(UsbEvent::Paired { reconnect, .. })) => {
                    assert!(!reconnect);
                    got_paired = true;
                    break;
                }
                Ok(Some(other)) => panic!("unexpected event: {other:?}"),
                Ok(None) => break,
                Err(_) => break,
            }
        }
        assert!(got_sync, "SyncNegotiated should fire");
        assert!(got_caps, "Caps should fire");
        assert!(got_paired, "Paired should fire (auto-pair)");

        // Drop the peripheral side to signal EOF.
        drop(peripheral_side);
        let final_state = driver.await.expect("driver join").expect("driver ok");
        // After EOF, state is whatever the session was in last —
        // Active for auto-pair after CAPS.
        assert_eq!(final_state, SessionState::Active);
    }

    /// Pairing flow: host's `PairingPrompt` is forwarded; user
    /// confirmation via the control channel advances the session.
    #[tokio::test]
    async fn pairing_user_confirms_via_control_channel() {
        let (host_side, mut peripheral_side) = duplex(4096);

        let store = Arc::new(InMemoryLinkKeyStore::new());
        let mut session = UsbSession::with_link_key_store(store);
        session.inject_test_keys([0x11; 32], [0x33; 32], [0x66; 16]);

        let (event_tx, mut event_rx) = mpsc::channel(32);
        let (control_tx, control_rx) = mpsc::channel(4);

        let driver = tokio::spawn(async move {
            run_session(host_side, session, event_tx, control_rx).await
        });

        // 1. Eat host's SYNC, reply v2.
        let mut buf = [0u8; 64];
        let _ = peripheral_side.read(&mut buf).await.unwrap();
        peripheral_side
            .write_all(&hex("040032520200"))
            .await
            .unwrap();

        // 2. Send a CAPS with the all-0x55 fixture hive_id_bytes (matches
        //    pinned vectors).
        let mut caps_body = Vec::new();
        caps_body.push(0xFE);
        caps_body.push(0xA4);
        caps_body.push(0x00);
        caps_body.push(0x50);
        caps_body.extend_from_slice(&[0x55; 16]);
        caps_body.push(0x01);
        caps_body.push(0x63);
        caps_body.extend_from_slice(b"fix");
        caps_body.push(0x02);
        caps_body.push(0x01);
        caps_body.push(0x03);
        caps_body.push(0x81);
        caps_body.push(0xA2);
        caps_body.push(0x00);
        caps_body.push(0x00);
        caps_body.push(0x01);
        caps_body.push(0x01);
        let len = (caps_body.len() as u16).to_le_bytes();
        let mut caps_frame = Vec::new();
        caps_frame.extend_from_slice(&len);
        caps_frame.extend_from_slice(&caps_body);
        peripheral_side.write_all(&caps_frame).await.unwrap();

        // Wait for the host's PAIR_HELLO_HOST to arrive. duplex
        // delivers all available bytes per read, so one read is
        // enough — the host writes ~73 bytes for PAIR_HELLO_HOST.
        let mut hello_buf = vec![0u8; 256];
        let n = peripheral_side.read(&mut hello_buf).await.unwrap();
        assert!(
            n >= 60,
            "expected PAIR_HELLO_HOST (~73 bytes), got {n}"
        );
        // Peripheral computes commit_p over the pinned values. We
        // can use the precomputed value from the test vectors.
        let commit_p =
            hex("63036b4d1ce9e73c19dfbcdd3238cada9ae44f3186a2139b7ecf47aa0f41625e");
        // Build PAIR_COMMIT control frame: 0xFF || cbor({0:5, 1:{1: bstr/32 commit_p}})
        let mut commit_body = vec![0xFF, 0xA2, 0x00, 0x05, 0x01, 0xA1, 0x01, 0x58, 0x20];
        commit_body.extend_from_slice(&commit_p);
        let len = (commit_body.len() as u16).to_le_bytes();
        let mut commit_frame = Vec::new();
        commit_frame.extend_from_slice(&len);
        commit_frame.extend_from_slice(&commit_body);
        peripheral_side.write_all(&commit_frame).await.unwrap();

        // PAIR_REVEAL: pinned eph_pk_peripheral + nonce_peripheral
        let pk_periph = hex("0faa684ed28867b97f4a6a2dee5df8ce974e76b7018e3f22a1c4cf2678570f20");
        let nonce_periph = vec![0x44; 32];
        let mut reveal_body = vec![0xFF, 0xA2, 0x00, 0x06, 0x01, 0xA2];
        reveal_body.extend_from_slice(&[0x01, 0x58, 0x20]);
        reveal_body.extend_from_slice(&pk_periph);
        reveal_body.extend_from_slice(&[0x02, 0x58, 0x20]);
        reveal_body.extend_from_slice(&nonce_periph);
        let len = (reveal_body.len() as u16).to_le_bytes();
        let mut reveal_frame = Vec::new();
        reveal_frame.extend_from_slice(&len);
        reveal_frame.extend_from_slice(&reveal_body);
        peripheral_side.write_all(&reveal_frame).await.unwrap();

        // Expect a PairingPrompt event with sas_code = 488092.
        let mut got_prompt = false;
        for _ in 0..10 {
            match tokio::time::timeout(
                std::time::Duration::from_millis(200),
                event_rx.recv(),
            )
            .await
            {
                Ok(Some(UsbEvent::PairingPrompt { sas_code, .. })) => {
                    assert_eq!(sas_code, 488_092);
                    got_prompt = true;
                    break;
                }
                Ok(Some(_)) => continue, // SyncNegotiated, Caps, etc.
                _ => break,
            }
        }
        assert!(got_prompt, "PairingPrompt should fire with sas=488092");

        // User confirms via the control channel.
        control_tx.send(SessionControl::UserConfirms).await.unwrap();

        // Read the host's PAIR_CONFIRM (single-byte body), reply with PAIR_DONE.
        let mut confirm_buf = vec![0u8; 16];
        let _ = peripheral_side.read(&mut confirm_buf).await.unwrap();
        let pair_done_body = vec![0xFF, 0xA2, 0x00, 0x08, 0x01, 0xA0];
        let len = (pair_done_body.len() as u16).to_le_bytes();
        let mut done_frame = Vec::new();
        done_frame.extend_from_slice(&len);
        done_frame.extend_from_slice(&pair_done_body);
        peripheral_side.write_all(&done_frame).await.unwrap();

        // Expect Paired event.
        let mut got_paired = false;
        for _ in 0..6 {
            match tokio::time::timeout(
                std::time::Duration::from_millis(200),
                event_rx.recv(),
            )
            .await
            {
                Ok(Some(UsbEvent::Paired { reconnect, .. })) => {
                    assert!(!reconnect);
                    got_paired = true;
                    break;
                }
                Ok(_) => continue,
                _ => break,
            }
        }
        assert!(got_paired, "Paired event should fire after PAIR_DONE");

        drop(peripheral_side);
        let _ = driver.await;
    }
}
