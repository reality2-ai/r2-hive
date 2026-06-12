//! Hot-plug watcher for CDC-ACM peripherals (Phase USB-3b).
//!
//! Periodically scans a directory (`/dev` in production) for new
//! `ttyACM*` / `ttyUSB*` device nodes and spawns a per-device session
//! task wrapping [`crate::usb_serial::run_session`]. When a device
//! disappears (cable unplugged, peripheral reset), the corresponding
//! session task reaches EOF on its own and ends; the watcher reaps
//! its [`tokio::task::JoinHandle`] on the next poll tick.
//!
//! ## Why polling and not inotify
//!
//! Polling at 1–2 Hz is simpler than wiring `inotify` for `/dev`,
//! handles the corner cases (slow USB enumeration, kernel
//! coalescing, CDC-ACM device-name reuse) the same way regardless of
//! kernel version, and adds no new dep. The cost is up to 2 seconds
//! of latency between cable insertion and pairing flow start, which
//! is well below what an operator notices.
//!
//! For deployments where reactive hot-plug matters (e.g. industrial
//! systems where pairing must complete in <500 ms), the watcher's
//! `scan_dir` API takes any `Path` so a future udev/inotify-based
//! variant can drop in as an alternative scanner.

#![cfg(target_os = "linux")]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::usb::{InMemoryLinkKeyStore, LinkKeyStore, UsbEvent, UsbSession};
use crate::usb_serial::{run_session, RawSerial, SessionControl};

/// Default polling interval. Production-tunable via `HotPlugWatcher::with_interval`.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(1500);

/// USB descriptor fields read from `/sys/class/tty/<name>/device/...`.
/// Used by [`UsbFilter`] to decide whether a given serial device is
/// an R2 peripheral the watcher should attempt to talk to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDescriptor {
    /// USB Vendor ID (`idVendor`).
    pub vid: u16,
    /// USB Product ID (`idProduct`).
    pub pid: u16,
    /// Manufacturer string from `iManufacturer`.
    pub manufacturer: Option<String>,
    /// Product string from `iProduct`.
    pub product: Option<String>,
}

/// Decision-maker for whether a `/dev/ttyACM*` device should be
/// treated as an R2 peripheral.
///
/// **Default-deny.** A bare-default `UsbFilter` permits *no* device.
/// The R2 firmware must set a recognised USB VID/PID so the host can
/// distinguish it from arbitrary serial peripherals (3D printers,
/// Arduino boards, USB-UART bridges) that happen to share `/dev`.
///
/// Three ways a device can be permitted:
///
/// 1. Its `(idVendor, idProduct)` is in `vid_pid_allowlist` — the
///    intended production path. Operators add a pair via
///    `--usb-vid-pid VID:PID` (repeatable) until R2 has an assigned
///    USB-IF VID, after which the project's canonical pair lands in
///    the compiled-in default.
/// 2. Its path is in `explicit_paths` — added by the operator via
///    `r2hive usb prepare /dev/ttyACMn` (Phase USB-4). One-shot
///    bypass for hardware whose VID/PID isn't yet on the list, e.g.
///    development bring-up of a new board.
/// 3. `allow_any` is true — the dev escape hatch. **Never** in
///    production; the `--usb-allow-any` CLI flag is conspicuously
///    flagged unsafe.
#[derive(Debug, Clone, Default)]
pub struct UsbFilter {
    /// Allowed `(vid, pid)` pairs. Empty by default.
    pub vid_pid_allowlist: Vec<(u16, u16)>,
    /// DEV/TEST ONLY: bypass the filter and try every CDC-ACM
    /// device. NEVER set in production.
    pub allow_any: bool,
    /// Per-path one-shot allowlist. Operator-driven via the future
    /// `r2hive usb prepare` command (Phase USB-4).
    pub explicit_paths: HashSet<PathBuf>,
}

impl UsbFilter {
    /// Returns true iff the watcher should open and SYNC at this
    /// device. Reads USB descriptors lazily — only when
    /// `vid_pid_allowlist` is consulted.
    pub fn permits(&self, path: &Path) -> bool {
        if self.allow_any {
            return true;
        }
        if self.explicit_paths.contains(path) {
            return true;
        }
        if self.vid_pid_allowlist.is_empty() {
            // No VID/PIDs configured and no explicit allow → deny.
            return false;
        }
        match device_descriptors(path) {
            Some(d) => self
                .vid_pid_allowlist
                .iter()
                .any(|(v, p)| *v == d.vid && *p == d.pid),
            None => false,
        }
    }
}

/// Read USB descriptor fields for a `/dev/ttyACM*` (or `ttyUSB*`)
/// path. Walks up `/sys/class/tty/<name>/device/...` until it finds
/// a directory containing both `idVendor` and `idProduct` — that's
/// the USB device node, distinct from the interface node directly
/// linked by `/sys/class/tty`.
///
/// Returns `None` if `/sys` isn't reachable, the device isn't a USB
/// peripheral, or the descriptor files are unreadable. Behaviour is
/// non-fatal — the filter just denies.
pub fn device_descriptors(serial_path: &Path) -> Option<UsbDescriptor> {
    let name = serial_path.file_name()?.to_str()?;
    let sys_link = Path::new("/sys/class/tty").join(name).join("device");
    let mut current = std::fs::canonicalize(&sys_link).ok()?;
    // USB topology: tty interface → USB interface → USB device. At
    // most a few levels.
    for _ in 0..6 {
        let vid_path = current.join("idVendor");
        let pid_path = current.join("idProduct");
        if vid_path.is_file() && pid_path.is_file() {
            let vid = read_hex_u16(&vid_path)?;
            let pid = read_hex_u16(&pid_path)?;
            let manufacturer = read_trimmed_string(&current.join("manufacturer"));
            let product = read_trimmed_string(&current.join("product"));
            return Some(UsbDescriptor {
                vid,
                pid,
                manufacturer,
                product,
            });
        }
        current = match current.parent() {
            Some(p) => p.to_path_buf(),
            None => return None,
        };
    }
    None
}

fn read_hex_u16(path: &Path) -> Option<u16> {
    let s = std::fs::read_to_string(path).ok()?;
    u16::from_str_radix(s.trim(), 16).ok()
}

fn read_trimmed_string(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Filename prefixes that look like CDC-ACM serial devices on Linux.
/// `ttyACM` is the canonical native USB-CDC name; `ttyUSB` covers
/// devices that go through a USB-UART bridge (CP2102, FTDI, etc).
const ACM_PREFIXES: &[&str] = &["ttyACM", "ttyUSB"];

/// Event surfaced from the hot-plug watcher to the operator side.
/// Each plugged-in dongle's session events get re-tagged with the
/// device path so a multi-dongle host knows which one each event
/// came from.
#[derive(Debug)]
pub enum HotPlugEvent {
    /// A new device was discovered and a session task spawned.
    /// Subsequent `Session` events from this `path` belong to that
    /// session.
    DeviceAttached { path: PathBuf },
    /// A previously-attached device's session task ended (cable
    /// unplugged, peripheral reset, or fatal protocol error). The
    /// watcher will spawn a fresh session if the same device path
    /// reappears.
    DeviceDetached { path: PathBuf },
    /// A [`UsbEvent`] from the session running on `path`.
    Session { path: PathBuf, event: UsbEvent },
    /// A device-open or session-spawn error. The watcher continues —
    /// a transient failure (permission denied while udev re-applies
    /// rules, etc.) doesn't kill the watcher.
    Error { path: PathBuf, message: String },
}

/// Status of a single tracked USB device, surfaced via the
/// [`UsbBringupHandle::status`] snapshot. Powers the
/// `r2.mgmt.usb.list` mgmt event and the `r2hive usb list` CLI.
#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub path: PathBuf,
    /// Last-known session state. `None` if the watcher saw the device
    /// but hasn't yet observed a session-state-changing event.
    pub session_state: Option<crate::usb::SessionState>,
    /// `hive_id_bytes` from CAPS once it arrives.
    pub hive_id_bytes: Option<crate::usb_pair::HiveIdBytes>,
    /// Firmware identifier from CAPS, for operator display.
    pub firmware_id: Option<String>,
    /// SAS code waiting for operator confirmation. `None` outside the
    /// `PairingAwaitingUser` window.
    pub pending_sas: Option<u32>,
    /// Operator-readable summary of the most recent error /
    /// pairing-failed reason, if any.
    pub last_error: Option<String>,
    /// Last-known USB descriptor (vid/pid, manufacturer, product).
    /// `None` for devices the watcher saw via `--usb-allow-any`
    /// without a `/sys` entry (e.g. test fixtures).
    pub descriptor: Option<UsbDescriptor>,
    /// CAPS-advertised transports (Phase USB-5). Populated when
    /// `UsbEvent::Caps` arrives so [`UsbBringupHandle::find_dongle_for_kind`]
    /// can resolve a transport-kind lookup to a path + local_id.
    pub advertised_transports: Option<Vec<crate::usb::TransportDescriptor>>,
}

impl DeviceStatus {
    fn fresh(path: PathBuf, descriptor: Option<UsbDescriptor>) -> Self {
        Self {
            path,
            session_state: None,
            hive_id_bytes: None,
            firmware_id: None,
            pending_sas: None,
            last_error: None,
            descriptor,
            advertised_transports: None,
        }
    }
}

/// Shareable handle to a running [`HotPlugWatcher`].
///
/// All operations are cheap clones; internally everything is held in
/// `Arc<RwLock<…>>`. The handle is what mgmt-event handlers (and
/// future applet RPCs) interact with.
#[derive(Clone)]
pub struct UsbBringupHandle {
    filter: Arc<std::sync::RwLock<UsbFilter>>,
    controls: Arc<std::sync::RwLock<HashMap<PathBuf, mpsc::Sender<SessionControl>>>>,
    statuses: Arc<std::sync::RwLock<HashMap<PathBuf, DeviceStatus>>>,
    link_keys: Arc<dyn LinkKeyStore>,
}

impl UsbBringupHandle {
    /// Add `path` to the explicit allowlist. The next watcher poll
    /// will pick it up and (assuming it's openable) start a session.
    /// This is what `r2hive usb prepare /dev/ttyACMn` does.
    pub fn prepare(&self, path: PathBuf) {
        self.filter
            .write()
            .expect("filter lock")
            .explicit_paths
            .insert(path);
    }

    /// Remove `path` from the explicit allowlist. Doesn't tear down a
    /// running session — that requires a separate cable disconnect or
    /// `unpair`. Mostly useful when the operator changes their mind
    /// about a device they prepared but haven't paired with yet.
    pub fn unprepare(&self, path: &Path) {
        self.filter
            .write()
            .expect("filter lock")
            .explicit_paths
            .remove(path);
    }

    /// Send a [`SessionControl::UserConfirms`] to the session on
    /// `path`. Used by the applet (and `r2hive usb confirm`) after
    /// the operator verifies the SAS code.
    pub async fn confirm(&self, path: &Path) -> bool {
        let tx = self
            .controls
            .read()
            .expect("controls lock")
            .get(path)
            .cloned();
        match tx {
            Some(tx) => tx.send(SessionControl::UserConfirms).await.is_ok(),
            None => false,
        }
    }

    /// Send a [`SessionControl::UserAborts`] to the session on
    /// `path`. Used when the operator notices the SAS codes don't
    /// match (or just changes their mind).
    pub async fn abort(&self, path: &Path) -> bool {
        let tx = self
            .controls
            .read()
            .expect("controls lock")
            .get(path)
            .cloned();
        match tx {
            Some(tx) => tx.send(SessionControl::UserAborts).await.is_ok(),
            None => false,
        }
    }

    /// Forget the link key for a previously-paired peripheral.
    /// Subsequent attaches with this `hive_id_bytes` will trigger fresh
    /// R2-PROVISION §5.3.4 first-attach pairing.
    pub fn unpair(&self, hive_id_bytes: &crate::usb_pair::HiveIdBytes) {
        self.link_keys.revoke(hive_id_bytes);
    }

    /// Snapshot of every currently-tracked device.
    pub fn status(&self) -> Vec<DeviceStatus> {
        let mut v: Vec<DeviceStatus> = self
            .statuses
            .read()
            .expect("status lock")
            .values()
            .cloned()
            .collect();
        v.sort_by(|a, b| a.path.cmp(&b.path));
        v
    }

    /// Send an R2-WIRE frame out via the dongle on `path`'s
    /// `local_id` transport (R2-USB §3.5). Returns `false` if no
    /// session is running for that path or its control channel is
    /// closed.
    pub async fn send_via_path(&self, path: &Path, local_id: u8, bytes: Vec<u8>) -> bool {
        let tx = self
            .controls
            .read()
            .expect("controls lock")
            .get(path)
            .cloned();
        match tx {
            Some(tx) => tx
                .send(SessionControl::SendWireFrame { local_id, bytes })
                .await
                .is_ok(),
            None => false,
        }
    }

    /// Look up a paired dongle that advertises a transport whose
    /// `kind` matches `wanted` (Phase USB-5). Returns `(path,
    /// local_id)` for the first match; if multiple dongles advertise
    /// the same transport kind, the lowest-sorted path wins —
    /// consistent and deterministic, but multi-dongle disambiguation
    /// will need its own treatment in a follow-up.
    pub fn find_dongle_for_kind(
        &self,
        wanted: &crate::usb::TransportKind,
    ) -> Option<(PathBuf, u8)> {
        let map = self.statuses.read().expect("status lock");
        let mut paths: Vec<&PathBuf> = map.keys().collect();
        paths.sort();
        for path in paths {
            let st = map.get(path)?;
            // Only paired (Active) devices count.
            if st.session_state != Some(crate::usb::SessionState::Active) {
                continue;
            }
            if let Some(transports) = &st.advertised_transports {
                for t in transports {
                    if transport_kind_matches(&t.kind, wanted) {
                        return Some((path.clone(), t.local_id));
                    }
                }
            }
        }
        None
    }
}

fn transport_kind_matches(
    a: &crate::usb::TransportKind,
    b: &crate::usb::TransportKind,
) -> bool {
    use crate::usb::TransportKind::*;
    match (a, b) {
        (Enumerated(x), Enumerated(y)) => x == y,
        (Named(x), Named(y)) => x == y,
        _ => false,
    }
}

/// Hot-plug watcher for USB-attached R2 peripherals.
///
/// Construct with [`HotPlugWatcher::new`], call [`HotPlugWatcher::run`]
/// inside a `tokio::spawn`, and consume `HotPlugEvent`s on the
/// receiver returned alongside. Use [`HotPlugWatcher::handle`] to
/// extract a [`UsbBringupHandle`] before spawning so the daemon's
/// mgmt-event handlers can reach into the watcher's state.
pub struct HotPlugWatcher {
    scan_dir: PathBuf,
    poll_interval: Duration,
    link_keys: Arc<dyn LinkKeyStore>,
    filter: Arc<std::sync::RwLock<UsbFilter>>,
    /// Per-device control channel — operator confirmations / aborts
    /// are routed through these.
    controls: Arc<std::sync::RwLock<HashMap<PathBuf, mpsc::Sender<SessionControl>>>>,
    /// Per-device session join handles.
    sessions: HashMap<PathBuf, JoinHandle<()>>,
    /// Per-device status snapshot (visible to mgmt callers).
    statuses: Arc<std::sync::RwLock<HashMap<PathBuf, DeviceStatus>>>,
    /// Re-emit channel — every per-session event (and watcher-level
    /// attach/detach) goes here.
    out: mpsc::Sender<HotPlugEvent>,
}

impl HotPlugWatcher {
    /// Build a watcher rooted at `scan_dir` (typically `/dev`) using
    /// the given link-key store. Returns the watcher and the event
    /// receiver.
    pub fn new(
        scan_dir: impl Into<PathBuf>,
        link_keys: Arc<dyn LinkKeyStore>,
    ) -> (Self, mpsc::Receiver<HotPlugEvent>) {
        let (tx, rx) = mpsc::channel(256);
        let w = Self {
            scan_dir: scan_dir.into(),
            poll_interval: DEFAULT_POLL_INTERVAL,
            link_keys,
            filter: Arc::new(std::sync::RwLock::new(UsbFilter::default())),
            controls: Arc::new(std::sync::RwLock::new(HashMap::new())),
            sessions: HashMap::new(),
            statuses: Arc::new(std::sync::RwLock::new(HashMap::new())),
            out: tx,
        };
        (w, rx)
    }

    /// Override the polling interval (default 1500 ms).
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Replace the [`UsbFilter`]. Default is fully restrictive (no
    /// device permitted) so callers MUST set this if they want any
    /// device opened.
    pub fn with_filter(self, filter: UsbFilter) -> Self {
        *self.filter.write().expect("filter lock") = filter;
        self
    }

    /// Cheap clone of a handle that can mutate the filter, route
    /// operator controls to running sessions, and read status
    /// snapshots from outside the watcher's task. Capture this
    /// **before** moving the watcher into `tokio::spawn(run)` —
    /// mgmt-event handlers (and applets) hold the handle to drive
    /// the watcher.
    pub fn handle(&self) -> UsbBringupHandle {
        UsbBringupHandle {
            filter: self.filter.clone(),
            controls: self.controls.clone(),
            statuses: self.statuses.clone(),
            link_keys: self.link_keys.clone(),
        }
    }

    /// Send a [`SessionControl`] (UserConfirms / UserAborts) to the
    /// session running on `path`. Returns `false` if no session is
    /// running for that path or the channel is closed.
    pub async fn send_control(&self, path: &Path, ctrl: SessionControl) -> bool {
        let tx = self
            .controls
            .read()
            .expect("controls lock")
            .get(path)
            .cloned();
        match tx {
            Some(tx) => tx.send(ctrl).await.is_ok(),
            None => false,
        }
    }

    /// Run the watcher's main loop. Returns when `out`'s receiver is
    /// dropped, signalling that the operator surface is gone.
    pub async fn run(mut self) {
        let mut tick = tokio::time::interval(self.poll_interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if self.out.is_closed() {
                return;
            }
            self.poll_once().await;
        }
    }

    async fn poll_once(&mut self) {
        // Discover currently-present devices.
        let present: Vec<PathBuf> = scan_acm_devices(&self.scan_dir);
        let known: std::collections::HashSet<_> = self.sessions.keys().cloned().collect();

        // Reap any session whose JoinHandle has finished. This
        // catches both clean disconnect (read EOF) and protocol
        // errors that closed the session.
        let mut to_remove = Vec::new();
        for (path, handle) in self.sessions.iter() {
            if handle.is_finished() {
                to_remove.push(path.clone());
            }
        }
        for path in to_remove {
            self.sessions.remove(&path);
            self.controls
                .write()
                .expect("controls lock")
                .remove(&path);
            self.statuses
                .write()
                .expect("status lock")
                .remove(&path);
            let _ = self
                .out
                .send(HotPlugEvent::DeviceDetached { path })
                .await;
        }

        // Spawn sessions for newly-attached devices that the filter
        // permits. Filtered-out devices are silently ignored — that
        // includes the common case of any random CDC-ACM device (3D
        // printer firmware, Arduino IDE, USB-UART bridges) the
        // operator hasn't explicitly opted into.
        let present_set: HashSet<_> = present.iter().cloned().collect();
        for path in &present {
            if known.contains(path) {
                continue;
            }
            // Snapshot the filter so we don't hold its lock across an
            // .await — the filter consults /sys reads which are sync
            // anyway.
            let permitted = {
                let f = self.filter.read().expect("filter lock");
                f.permits(path)
            };
            if !permitted {
                continue;
            }
            self.spawn_session(path.clone()).await;
        }

        // For devices that vanished from the filesystem but whose
        // session task hasn't finished yet (rare — usually the
        // session ends on read EOF first), don't force-kill. The
        // session will end when the next read returns EOF; we'll
        // reap it on the next poll.
        let _ = present_set;
    }

    async fn spawn_session(&mut self, path: PathBuf) {
        // Capture the descriptor (best-effort) so the status snapshot
        // can show vid/pid/manufacturer/product to operator UIs.
        let descriptor = device_descriptors(&path);
        let serial = match RawSerial::open(&path) {
            Ok(s) => s,
            Err(e) => {
                self.statuses
                    .write()
                    .expect("status lock")
                    .entry(path.clone())
                    .and_modify(|st| st.last_error = Some(format!("open: {e}")))
                    .or_insert_with(|| {
                        let mut s = DeviceStatus::fresh(path.clone(), descriptor.clone());
                        s.last_error = Some(format!("open: {e}"));
                        s
                    });
                let _ = self
                    .out
                    .send(HotPlugEvent::Error {
                        path: path.clone(),
                        message: format!("open {}: {e}", path.display()),
                    })
                    .await;
                return;
            }
        };

        // Seed the status entry now so `r2.mgmt.usb.list` shows the
        // device immediately rather than waiting for the first
        // SessionState transition.
        self.statuses
            .write()
            .expect("status lock")
            .insert(path.clone(), DeviceStatus::fresh(path.clone(), descriptor));

        let session = UsbSession::with_link_key_store(self.link_keys.clone());
        let (event_tx, mut event_rx) = mpsc::channel::<UsbEvent>(64);
        let (ctrl_tx, ctrl_rx) = mpsc::channel::<SessionControl>(8);

        // Forward per-session events through the watcher's tagged
        // channel AND fold them into the status snapshot map so
        // mgmt callers see the latest state at a glance.
        let out = self.out.clone();
        let path_for_events = path.clone();
        let statuses_for_events = self.statuses.clone();
        tokio::spawn(async move {
            while let Some(ev) = event_rx.recv().await {
                update_status_from_event(&statuses_for_events, &path_for_events, &ev);
                if out
                    .send(HotPlugEvent::Session {
                        path: path_for_events.clone(),
                        event: ev,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        // Run the session. The driver returns when the device EOFs or
        // the session closes; either way the JoinHandle finishes and
        // the next poll_once reaps it.
        let path_for_session = path.clone();
        let handle = tokio::spawn(async move {
            let _ = run_session(serial, session, event_tx, ctrl_rx).await;
            let _ = path_for_session;
        });

        let _ = self
            .out
            .send(HotPlugEvent::DeviceAttached { path: path.clone() })
            .await;
        self.controls
            .write()
            .expect("controls lock")
            .insert(path.clone(), ctrl_tx);
        self.sessions.insert(path, handle);
    }
}

/// Fold a per-session [`UsbEvent`] into the status snapshot map.
/// Intentionally narrow: only the few fields the operator surface
/// cares about (current state, hive_id_bytes once known, pending SAS,
/// last error) get tracked here. The full event stream still flows
/// through the [`HotPlugEvent`] channel for callers that want it.
fn update_status_from_event(
    statuses: &Arc<std::sync::RwLock<HashMap<PathBuf, DeviceStatus>>>,
    path: &Path,
    event: &UsbEvent,
) {
    let mut map = statuses.write().expect("status lock");
    let entry = map
        .entry(path.to_path_buf())
        .or_insert_with(|| DeviceStatus::fresh(path.to_path_buf(), None));
    match event {
        UsbEvent::SyncNegotiated { .. } => {
            entry.session_state = Some(crate::usb::SessionState::AwaitingCaps);
        }
        UsbEvent::Caps(caps) => {
            entry.hive_id_bytes = Some(caps.hive_id_bytes);
            entry.firmware_id = Some(caps.firmware_id.clone());
            entry.advertised_transports = Some(caps.transports.clone());
        }
        UsbEvent::PairingPrompt {
            hive_id_bytes,
            firmware_id,
            sas_code,
        } => {
            entry.hive_id_bytes = Some(*hive_id_bytes);
            entry.firmware_id = Some(firmware_id.clone());
            entry.pending_sas = Some(*sas_code);
            entry.session_state = Some(crate::usb::SessionState::PairingAwaitingUser);
        }
        UsbEvent::Paired { hive_id_bytes, .. } => {
            entry.hive_id_bytes = Some(*hive_id_bytes);
            entry.pending_sas = None;
            entry.session_state = Some(crate::usb::SessionState::Active);
            entry.last_error = None;
        }
        UsbEvent::PairingFailed { reason } => {
            entry.pending_sas = None;
            entry.session_state = Some(crate::usb::SessionState::Closed);
            entry.last_error = Some(reason.clone());
        }
        UsbEvent::Error(e) => {
            entry.session_state = Some(crate::usb::SessionState::Closed);
            entry.last_error = Some(format!("{e}"));
        }
        UsbEvent::WireFrame { .. } | UsbEvent::Control { .. } => {
            // Steady-state events; no status change.
        }
    }
}

/// Build a watcher with a default in-memory link-key store. Suitable
/// for first-deployment / dev rigs; production deployments should
/// pass a persistent store via [`HotPlugWatcher::new`].
pub fn watcher_with_default_store(
    scan_dir: impl Into<PathBuf>,
) -> (HotPlugWatcher, mpsc::Receiver<HotPlugEvent>) {
    HotPlugWatcher::new(scan_dir, Arc::new(InMemoryLinkKeyStore::new()))
}

/// Enumerate CDC-ACM devices in `dir`. Returns sorted absolute paths
/// for entries whose file name starts with one of [`ACM_PREFIXES`].
/// Non-existent or non-readable directories yield an empty list — the
/// watcher logs nothing in that case (a bare-metal rig may not have
/// `/dev` in the expected place).
pub fn scan_acm_devices(dir: &Path) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if ACM_PREFIXES.iter().any(|p| name.starts_with(p)) {
            out.push(entry.path());
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_acm_devices_returns_empty_for_missing_dir() {
        let v = scan_acm_devices(Path::new("/non/existent/dir"));
        assert!(v.is_empty());
    }

    #[test]
    fn scan_acm_devices_filters_by_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a mix of files. Only ttyACM*/ttyUSB* should show up.
        for name in &[
            "ttyACM0",
            "ttyACM1",
            "ttyUSB0",
            "ttyS0",
            "random_file",
            "null",
        ] {
            std::fs::write(tmp.path().join(name), b"").unwrap();
        }
        let v = scan_acm_devices(tmp.path());
        let names: Vec<String> = v
            .iter()
            .filter_map(|p| p.file_name()?.to_str().map(|s| s.to_string()))
            .collect();
        assert_eq!(names, vec!["ttyACM0", "ttyACM1", "ttyUSB0"]);
    }

    /// Watcher's poll loop emits DeviceAttached for new devices. We
    /// can't actually open `/dev/ttyACM*` in a unit test — opening a
    /// regular file in a tempdir as a CDC-ACM device fails at
    /// termios — so the test verifies the scan + spawn-attempt path
    /// up to (and including) the open-failure surfacing as a
    /// HotPlugEvent::Error. That's the right behaviour for a hot
    /// rig too: an unreadable device should log and the watcher
    /// should keep running.
    ///
    /// `allow_any: true` is required here because the test fixture
    /// is a regular file with no `/sys` entry — the default
    /// (production) filter would skip it.
    #[tokio::test]
    async fn watcher_surfaces_error_on_unopenable_device() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("ttyACM0"), b"").unwrap();

        let (watcher, mut rx) =
            watcher_with_default_store(tmp.path().to_path_buf());
        let filter = UsbFilter {
            allow_any: true,
            ..Default::default()
        };
        let watcher = watcher
            .with_interval(Duration::from_millis(50))
            .with_filter(filter);
        let handle = tokio::spawn(watcher.run());

        // Wait for the first poll.
        let mut got_event = None;
        for _ in 0..20 {
            match tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
                Ok(Some(ev)) => {
                    got_event = Some(ev);
                    break;
                }
                _ => continue,
            }
        }
        let event = got_event.expect("watcher should emit something");
        // Either an Error (more likely — termios fails on a regular
        // file) or a DeviceAttached event followed quickly by
        // session events.  Both prove the scan+spawn path ran.
        match event {
            HotPlugEvent::Error { path, .. } => {
                assert!(path.ends_with("ttyACM0"));
            }
            HotPlugEvent::DeviceAttached { path } => {
                assert!(path.ends_with("ttyACM0"));
            }
            other => panic!("unexpected event: {other:?}"),
        }

        // Tear down — drop the receiver so the watcher's run loop
        // exits.
        drop(rx);
        let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
    }

    #[test]
    fn default_filter_denies_everything() {
        let f = UsbFilter::default();
        assert!(!f.permits(Path::new("/dev/ttyACM0")));
        assert!(!f.permits(Path::new("/dev/ttyUSB0")));
        // Even a path that doesn't exist returns false (don't open
        // it).
        assert!(!f.permits(Path::new("/non/existent")));
    }

    #[test]
    fn allow_any_permits_everything() {
        let f = UsbFilter {
            allow_any: true,
            ..Default::default()
        };
        assert!(f.permits(Path::new("/dev/ttyACM0")));
        assert!(f.permits(Path::new("/dev/random")));
    }

    #[test]
    fn explicit_path_permits_only_that_path() {
        let mut f = UsbFilter::default();
        f.explicit_paths.insert(PathBuf::from("/dev/ttyACM0"));
        assert!(f.permits(Path::new("/dev/ttyACM0")));
        assert!(!f.permits(Path::new("/dev/ttyACM1")));
        assert!(!f.permits(Path::new("/dev/ttyUSB0")));
    }

    #[test]
    fn vid_pid_allowlist_alone_does_not_match_random_paths() {
        // /dev/ttyACM999 won't have a /sys entry, so even with a
        // VID/PID configured the filter denies — defensive against
        // a future bug where descriptor lookup silently returns
        // empty/match.
        let f = UsbFilter {
            vid_pid_allowlist: vec![(0x303A, 0x1001)],
            ..Default::default()
        };
        assert!(!f.permits(Path::new("/dev/ttyACM999")));
    }

    #[test]
    fn descriptor_reads_return_none_for_non_usb_paths() {
        // /dev/null exists on every Linux but isn't a USB device —
        // exercising the "path is fine but no /sys USB entry" branch.
        let d = device_descriptors(Path::new("/dev/null"));
        assert!(d.is_none(), "expected no descriptors for /dev/null");
    }

    #[tokio::test]
    async fn handle_prepare_extends_explicit_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let (watcher, _rx) = watcher_with_default_store(tmp.path().to_path_buf());
        let handle = watcher.handle();

        // Initially no path is permitted.
        assert!(!handle.filter.read().unwrap().permits(Path::new("/dev/ttyACM0")));

        // After prepare(), the path is in explicit_paths.
        handle.prepare(PathBuf::from("/dev/ttyACM0"));
        assert!(handle.filter.read().unwrap().permits(Path::new("/dev/ttyACM0")));

        // unprepare reverses it.
        handle.unprepare(Path::new("/dev/ttyACM0"));
        assert!(!handle.filter.read().unwrap().permits(Path::new("/dev/ttyACM0")));
    }

    #[tokio::test]
    async fn handle_status_starts_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let (watcher, _rx) = watcher_with_default_store(tmp.path().to_path_buf());
        let handle = watcher.handle();
        assert!(handle.status().is_empty());
    }

    #[tokio::test]
    async fn handle_unpair_revokes_link_key() {
        // Link-key store is the same Arc shared with the watcher.
        let store = Arc::new(InMemoryLinkKeyStore::new());
        let (watcher, _rx) =
            HotPlugWatcher::new(std::env::temp_dir(), store.clone());
        let handle = watcher.handle();

        let hive_id_bytes = [0xAA; 16];
        let link_key = [0xBB; 32];
        store.store(&hive_id_bytes, &link_key);
        assert!(store.lookup(&hive_id_bytes).is_some());

        handle.unpair(&hive_id_bytes);
        assert!(store.lookup(&hive_id_bytes).is_none());
    }

    #[tokio::test]
    async fn find_dongle_for_kind_returns_active_paired_dongle() {
        use crate::usb::{
            CapsFrame, SessionState, TransportDescriptor, TransportKind, UsbEvent,
        };

        let tmp = tempfile::tempdir().unwrap();
        let (watcher, _rx) = watcher_with_default_store(tmp.path().to_path_buf());
        let handle = watcher.handle();

        // Synthesize a paired dongle via direct status-map mutation.
        // (In production, the watcher's run loop folds events the same
        // way; we're exercising the lookup logic, not the event flow.)
        let path = PathBuf::from("/dev/ttyACM0");
        let caps = CapsFrame {
            hive_id_bytes: [0x55; 16],
            firmware_id: "fix".into(),
            firmware_version: 1,
            transports: vec![
                TransportDescriptor {
                    local_id: 0,
                    kind: TransportKind::Enumerated(1), // 1 = lora
                    region: Some("US915".into()),
                    properties_cbor: Vec::new(),
                },
                TransportDescriptor {
                    local_id: 2,
                    kind: TransportKind::Enumerated(2), // 2 = ble
                    region: None,
                    properties_cbor: Vec::new(),
                },
            ],
        };
        update_status_from_event(&handle.statuses, &path, &UsbEvent::Caps(caps));
        update_status_from_event(
            &handle.statuses,
            &path,
            &UsbEvent::Paired {
                hive_id_bytes: [0x55; 16],
                reconnect: false,
            },
        );
        // Verify the device is marked Active.
        let snap = handle
            .statuses
            .read()
            .unwrap()
            .get(&path)
            .cloned()
            .unwrap();
        assert_eq!(snap.session_state, Some(SessionState::Active));

        // LoRa lookup → returns the dongle's local_id 0.
        let lora = handle.find_dongle_for_kind(&TransportKind::Enumerated(1));
        assert_eq!(lora, Some((path.clone(), 0)));

        // BLE lookup → returns local_id 2.
        let ble = handle.find_dongle_for_kind(&TransportKind::Enumerated(2));
        assert_eq!(ble, Some((path.clone(), 2)));

        // WiFi lookup → no match (this dongle doesn't advertise it).
        let wifi = handle.find_dongle_for_kind(&TransportKind::Enumerated(3));
        assert_eq!(wifi, None);
    }

    #[tokio::test]
    async fn find_dongle_for_kind_skips_non_active_dongles() {
        use crate::usb::{
            CapsFrame, TransportDescriptor, TransportKind, UsbEvent,
        };

        let tmp = tempfile::tempdir().unwrap();
        let (watcher, _rx) = watcher_with_default_store(tmp.path().to_path_buf());
        let handle = watcher.handle();

        let path = PathBuf::from("/dev/ttyACM0");
        let caps = CapsFrame {
            hive_id_bytes: [0x55; 16],
            firmware_id: "fix".into(),
            firmware_version: 1,
            transports: vec![TransportDescriptor {
                local_id: 0,
                kind: TransportKind::Enumerated(1),
                region: Some("US915".into()),
                properties_cbor: Vec::new(),
            }],
        };
        // CAPS arrived but pairing not complete → should NOT match.
        update_status_from_event(&handle.statuses, &path, &UsbEvent::Caps(caps));
        let lora = handle.find_dongle_for_kind(&TransportKind::Enumerated(1));
        assert_eq!(
            lora, None,
            "find_dongle_for_kind must skip dongles that haven't completed pairing"
        );
    }

    #[tokio::test]
    async fn send_via_path_routes_through_session_control_channel() {
        // Wire up a fake control channel under a known path, observe
        // that send_via_path produces a SessionControl::SendWireFrame
        // on it. Doesn't touch a real serial port — purely tests the
        // routing logic.
        use crate::usb_serial::SessionControl;

        let tmp = tempfile::tempdir().unwrap();
        let (watcher, _rx) = watcher_with_default_store(tmp.path().to_path_buf());
        let handle = watcher.handle();
        let path = PathBuf::from("/dev/ttyACM0");

        // Inject a synthetic control sender into the bringup handle's
        // controls map.
        let (ctrl_tx, mut ctrl_rx) = mpsc::channel::<SessionControl>(4);
        handle
            .controls
            .write()
            .unwrap()
            .insert(path.clone(), ctrl_tx);

        let payload = vec![0xAA, 0xBB, 0xCC];
        let ok = handle.send_via_path(&path, 7, payload.clone()).await;
        assert!(ok);

        let ctrl = ctrl_rx.recv().await.expect("control msg arrived");
        match ctrl {
            SessionControl::SendWireFrame { local_id, bytes } => {
                assert_eq!(local_id, 7);
                assert_eq!(bytes, payload);
            }
            other => panic!("expected SendWireFrame, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn handle_confirm_returns_false_for_unknown_path() {
        let tmp = tempfile::tempdir().unwrap();
        let (watcher, _rx) = watcher_with_default_store(tmp.path().to_path_buf());
        let handle = watcher.handle();
        // No session running for this path → confirm/abort no-op
        // returning false rather than panicking.
        assert!(!handle.confirm(Path::new("/dev/ttyACM999")).await);
        assert!(!handle.abort(Path::new("/dev/ttyACM999")).await);
    }

    #[test]
    fn update_status_reflects_caps_and_paired_events() {
        use crate::usb::{CapsFrame, SessionState, TransportDescriptor, TransportKind, UsbEvent};
        let map: Arc<std::sync::RwLock<HashMap<PathBuf, DeviceStatus>>> =
            Arc::new(std::sync::RwLock::new(HashMap::new()));
        let path = PathBuf::from("/dev/ttyACM0");

        update_status_from_event(
            &map,
            &path,
            &UsbEvent::Caps(CapsFrame {
                hive_id_bytes: [0x55; 16],
                firmware_id: "fw".into(),
                firmware_version: 7,
                transports: vec![TransportDescriptor {
                    local_id: 0,
                    kind: TransportKind::Enumerated(1),
                    region: Some("US915".into()),
                    properties_cbor: Vec::new(),
                }],
            }),
        );
        let snap = map.read().unwrap().get(&path).cloned().unwrap();
        assert_eq!(snap.hive_id_bytes, Some([0x55; 16]));
        assert_eq!(snap.firmware_id.as_deref(), Some("fw"));

        update_status_from_event(
            &map,
            &path,
            &UsbEvent::PairingPrompt {
                hive_id_bytes: [0x55; 16],
                firmware_id: "fw".into(),
                sas_code: 488_092,
            },
        );
        let snap = map.read().unwrap().get(&path).cloned().unwrap();
        assert_eq!(snap.pending_sas, Some(488_092));
        assert_eq!(snap.session_state, Some(SessionState::PairingAwaitingUser));

        update_status_from_event(
            &map,
            &path,
            &UsbEvent::Paired {
                hive_id_bytes: [0x55; 16],
                reconnect: false,
            },
        );
        let snap = map.read().unwrap().get(&path).cloned().unwrap();
        assert_eq!(snap.pending_sas, None);
        assert_eq!(snap.session_state, Some(SessionState::Active));
    }

    #[test]
    fn default_poll_interval_is_reasonable() {
        // Sanity: don't accidentally ship a 1ms-poll watcher.
        assert!(DEFAULT_POLL_INTERVAL >= Duration::from_millis(500));
        assert!(DEFAULT_POLL_INTERVAL <= Duration::from_secs(5));
    }
}
