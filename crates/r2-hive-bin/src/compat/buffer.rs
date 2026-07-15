//! Ring buffer for frame catchup — stores recent frames per trust group.
//!
//! Per R2-TRANSPORT-RELAY §4: volatile, default 1000 frames per trust group.
//! Clients can request frames since a timestamp to catch up after reconnect.

use std::collections::VecDeque;

/// A buffered frame with its receive timestamp.
pub struct BufferedFrame {
    pub timestamp: u64,
    pub data: Vec<u8>,
}

/// Bounded ring buffer of recent R2-WIRE frames.
pub struct RingBuffer {
    frames: VecDeque<BufferedFrame>,
    capacity: usize,
}

impl RingBuffer {
    /// Fixed-capacity ring (capacity = `--buffer-size`, default 1000).
    ///
    /// **Used-by:** `hive.rs::register_tg_peer` (one ring per TG compat entry).
    pub fn new(capacity: usize) -> Self {
        RingBuffer {
            frames: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
        }
    }

    /// Append a frame with its UNIX timestamp, evicting the oldest at capacity.
    ///
    /// **Used-by:** `hive.rs::buffer_frame` (legacy broadcast path).
    pub fn push(&mut self, data: Vec<u8>, timestamp: u64) {
        if self.frames.len() >= self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back(BufferedFrame { timestamp, data });
    }

    /// Frames at/after `timestamp`, oldest first (the catchup read).
    ///
    /// **Used-by:** `hive.rs::catchup_frames`.
    pub fn since(&self, timestamp: u64) -> impl Iterator<Item = &BufferedFrame> {
        self.frames.iter().filter(move |f| f.timestamp > timestamp)
    }

    /// Timestamp of the oldest retained frame (0 when empty) — tells a
    /// client how far back catchup can reach.
    ///
    /// **Used-by:** `hive.rs::buffer_oldest`.
    pub fn oldest_timestamp(&self) -> u64 {
        self.frames.front().map(|f| f.timestamp).unwrap_or(0)
    }

    /// Number of frames currently retained.
    ///
    /// **Used-by:** this file's tests (capacity/eviction assertions).
    pub fn len(&self) -> usize {
        self.frames.len()
    }
}
