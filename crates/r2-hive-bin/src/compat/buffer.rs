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
    pub fn new(capacity: usize) -> Self {
        RingBuffer {
            frames: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
        }
    }

    pub fn push(&mut self, data: Vec<u8>, timestamp: u64) {
        if self.frames.len() >= self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back(BufferedFrame { timestamp, data });
    }

    pub fn since(&self, timestamp: u64) -> impl Iterator<Item = &BufferedFrame> {
        self.frames.iter().filter(move |f| f.timestamp > timestamp)
    }

    pub fn oldest_timestamp(&self) -> u64 {
        self.frames.front().map(|f| f.timestamp).unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }
}
