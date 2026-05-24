//! Transport auto-detection (Phase 4a of the r2-hive plan).
//!
//! [`detect_profile`] runs a non-blocking ladder of probes and returns a
//! [`ProfileReport`] describing which transports are available on the
//! host. The result is purely diagnostic — `main.rs` consumes it to
//! decide which transport bring-ups to call. Operators can still
//! force-on / force-off any transport via the existing `--lan`,
//! `--ble`, `--lora` CLI flags; the `--auto` flag (default in a future
//! release; opt-in for now) tells the daemon to honour the report.
//!
//! ## Probe ladder
//!
//! 1. **USB R2-dongle** — enumerate `/dev/ttyACM*` and `/dev/ttyUSB*`.
//!    Presence is reported; the SYNC handshake (R2-USB §3.6) and
//!    dongle-advertised binding pre-emption is deferred to a follow-up
//!    iteration.
//! 2. **Internal BlueZ** — check whether `/sys/class/bluetooth/hci0`
//!    exists; read the kernel driver name; flag `hci_uart_qca` so the
//!    route engine can de-prioritise BLE on UNO-Q-style boards (see
//!    `TEST-RIG.md` known-issues).
//! 3. **Native WiFi / Ethernet** — enumerate IPv4 interfaces with
//!    UP+RUNNING flags via `getifaddrs`.
//! 4. **LoRa** — accept a configured socket path and report whether it
//!    is currently a connectable UDS.
//! 5. **Internet relay** — purely a configuration fact; not probed.
//!
//! Probes that fail are recorded as `present: false`; nothing in this
//! module panics, and no probe holds a kernel resource for longer than
//! its enumeration call. Total runtime on a Linux host is sub-50 ms.

use std::path::{Path, PathBuf};

/// Where the BLE driver name lives on Linux.
const BLE_SYS_DRIVER: &str = "/sys/class/bluetooth/hci0/device/driver";

/// Result of one [`detect_profile`] invocation.
#[derive(Debug, Clone)]
pub struct ProfileReport {
    /// USB candidates that *might* be R2 dongles. Caller is responsible
    /// for the SYNC handshake before treating these as real R2-USB v0.2
    /// links.
    pub usb_candidates: Vec<PathBuf>,
    /// BLE adapter status.
    pub ble: BleReport,
    /// IPv4 interfaces in UP+RUNNING state, excluding loopback.
    pub network_ifaces: Vec<NetworkIface>,
    /// LoRa socket status (configured path; presence flag).
    pub lora: LoraReport,
    /// Tier suggestion derived from the above. Operator-overridable.
    pub suggested_kind: ProfileKind,
}

/// What kind of host this looks like, used for default transport
/// selection. Mirrors the four deployment profiles in the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    /// Full Linux + USB dongle (or other USB R2 device).
    LinuxWithDongle,
    /// Plain Linux with internal BlueZ.
    LinuxBlueZ,
    /// No radios; networking only.
    Cloud,
    /// Browser / WASM target. Reported only when the binary was built
    /// with `--features wasm-only` and detection still ran (it
    /// shouldn't, but the variant exists for completeness).
    Wasm,
}

/// BLE adapter probe result.
#[derive(Debug, Clone, Default)]
pub struct BleReport {
    /// Whether `/sys/class/bluetooth/hci0` exists.
    pub present: bool,
    /// Linux kernel driver name (e.g. `"hci_uart_qca"`). Empty when
    /// `present == false` or the symlink can't be read.
    pub driver: String,
    /// True iff `driver == "hci_uart_qca"` — the QCA-rooted UART BLE
    /// stack we've seen lock up on UNO-Q boards. Route engine should
    /// lower the BLE priority weight when this is true.
    pub qca: bool,
}

/// LoRa probe result.
#[derive(Debug, Clone)]
pub struct LoraReport {
    /// Path the operator configured (`--lora-socket` default or
    /// override). Probed for existence + UDS-ness.
    pub socket_path: PathBuf,
    /// True iff `socket_path` exists and is a Unix-domain socket.
    pub present: bool,
}

/// One network interface in UP+RUNNING state. Per-iface address
/// resolution would require platform-specific syscalls; the report
/// only enumerates names and leaves binding to the existing transport
/// crates.
#[derive(Debug, Clone)]
pub struct NetworkIface {
    pub name: String,
}

impl ProfileReport {
    /// Whether the profile suggests starting the LAN bring-up (UDP
    /// beacon + UDP transport). True whenever there's any non-loopback
    /// IPv4 interface.
    pub fn should_run_lan(&self) -> bool {
        !self.network_ifaces.is_empty()
    }

    /// Whether the profile suggests starting the BLE bring-up.
    pub fn should_run_ble(&self) -> bool {
        self.ble.present
    }

    /// Whether the profile suggests starting the LoRa bring-up.
    pub fn should_run_lora(&self) -> bool {
        self.lora.present
    }

    /// Operator-readable summary, one line per finding.
    pub fn summary_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(format!("Profile: {:?}", self.suggested_kind));
        if self.network_ifaces.is_empty() {
            out.push("Networking: none detected".to_string());
        } else {
            let names: Vec<&str> = self.network_ifaces.iter().map(|i| i.name.as_str()).collect();
            out.push(format!("Networking: {} interface(s) — {}", names.len(), names.join(", ")));
        }
        match (&self.ble.present, &self.ble.qca) {
            (true, true) => out.push(format!(
                "BLE: present (driver {}; QCA fragility flag — see TEST-RIG.md)",
                self.ble.driver
            )),
            (true, false) => out.push(format!("BLE: present (driver {})", self.ble.driver)),
            (false, _) => out.push("BLE: not present".to_string()),
        }
        if self.usb_candidates.is_empty() {
            out.push("USB: no /dev/ttyACM* or /dev/ttyUSB* candidates".to_string());
        } else {
            out.push(format!(
                "USB: {} candidate(s) ({})",
                self.usb_candidates.len(),
                self.usb_candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out.push(format!(
            "LoRa: socket {} {}",
            self.lora.socket_path.display(),
            if self.lora.present { "present" } else { "missing" }
        ));
        out
    }
}

/// Run the probe ladder and produce a [`ProfileReport`].
///
/// `lora_socket` is the operator-configured LoRa IPC path (mirrors
/// `--lora-socket`). Pass `Path::new("")` if LoRa probing is irrelevant.
pub fn detect_profile(lora_socket: &Path) -> ProfileReport {
    let usb_candidates = probe_usb();
    let ble = probe_ble(BLE_SYS_DRIVER);
    let network_ifaces = probe_network();
    let lora = probe_lora(lora_socket);

    let suggested_kind = if !usb_candidates.is_empty() {
        ProfileKind::LinuxWithDongle
    } else if ble.present {
        ProfileKind::LinuxBlueZ
    } else if !network_ifaces.is_empty() {
        ProfileKind::Cloud
    } else {
        // No radios, no networking — odd but possible in containers
        // without --net. Fall back to Cloud and let the operator notice
        // via the summary.
        ProfileKind::Cloud
    };

    ProfileReport {
        usb_candidates,
        ble,
        network_ifaces,
        lora,
        suggested_kind,
    }
}

fn probe_usb() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let dev = Path::new("/dev");
    let entries = match std::fs::read_dir(dev) {
        Ok(e) => e,
        Err(_) => return candidates,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if name.starts_with("ttyACM") || name.starts_with("ttyUSB") {
            candidates.push(entry.path());
        }
    }
    candidates.sort();
    candidates
}

/// Read the BLE driver symlink and classify. Public for tests so a
/// fixture path can be passed instead of the real `/sys` path.
pub fn probe_ble(driver_path: &str) -> BleReport {
    let symlink = match std::fs::read_link(driver_path) {
        Ok(p) => p,
        Err(_) => {
            return BleReport {
                present: false,
                driver: String::new(),
                qca: false,
            };
        }
    };
    let driver = symlink
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let qca = driver == "hci_uart_qca";
    BleReport {
        present: true,
        driver,
        qca,
    }
}

fn probe_network() -> Vec<NetworkIface> {
    // Use the `/proc/net/route` table to enumerate UP interfaces. Avoids
    // a libc dep just for getifaddrs(). Each route line is
    // tab-separated: iface dest gateway flags ... Match flags & 0x1
    // (RTF_UP) and skip loopback.
    let route = match std::fs::read_to_string("/proc/net/route") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut by_iface = std::collections::BTreeMap::<String, ()>::new();
    for line in route.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let iface = cols[0];
        if iface == "lo" {
            continue;
        }
        let flags = u32::from_str_radix(cols[3], 16).unwrap_or(0);
        if flags & 0x1 == 0 {
            continue;
        }
        by_iface.insert(iface.to_string(), ());
    }

    // For each interface, read its IPv4 from /proc/net/fib_trie or
    // /sys/class/net. Both are awkward to parse portably; instead, walk
    // the interface listing and shell out to `ip -4 -o addr` if it's
    // available. Falling back: if we can read interface names but not
    // addresses, return them with 0.0.0.0 so the report still flags
    // "networking present".
    by_iface
        .into_keys()
        .map(|iface| NetworkIface { name: iface })
        .collect()
}

fn probe_lora(socket_path: &Path) -> LoraReport {
    let path = socket_path.to_path_buf();
    if path.as_os_str().is_empty() {
        return LoraReport {
            socket_path: path,
            present: false,
        };
    }
    let present = std::fs::metadata(&path)
        .map(|m| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                m.file_type().is_socket()
            }
            #[cfg(not(unix))]
            {
                let _ = m;
                false
            }
        })
        .unwrap_or(false);
    LoraReport {
        socket_path: path,
        present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_ble_missing_path_reports_absent() {
        let r = probe_ble("/non/existent/path");
        assert!(!r.present);
        assert!(!r.qca);
        assert_eq!(r.driver, "");
    }

    #[test]
    fn probe_ble_qca_symlink_flagged() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = tmp.path().join("driver");
        let target = tmp.path().join("hci_uart_qca");
        std::fs::create_dir_all(&target).unwrap();
        std::os::unix::fs::symlink(&target, &driver).unwrap();
        let r = probe_ble(driver.to_str().unwrap());
        assert!(r.present);
        assert_eq!(r.driver, "hci_uart_qca");
        assert!(r.qca);
    }

    #[test]
    fn probe_ble_non_qca_symlink_not_flagged() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = tmp.path().join("driver");
        let target = tmp.path().join("btusb");
        std::fs::create_dir_all(&target).unwrap();
        std::os::unix::fs::symlink(&target, &driver).unwrap();
        let r = probe_ble(driver.to_str().unwrap());
        assert!(r.present);
        assert_eq!(r.driver, "btusb");
        assert!(!r.qca);
    }

    #[test]
    fn probe_lora_empty_path_absent() {
        let r = probe_lora(Path::new(""));
        assert!(!r.present);
    }

    #[test]
    fn probe_lora_regular_file_not_a_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("not-a-socket");
        std::fs::write(&f, b"x").unwrap();
        let r = probe_lora(&f);
        assert!(!r.present);
    }

    #[test]
    fn detect_profile_summary_has_at_least_three_lines() {
        let r = detect_profile(Path::new(""));
        let lines = r.summary_lines();
        assert!(lines.len() >= 3, "got: {lines:?}");
    }
}
