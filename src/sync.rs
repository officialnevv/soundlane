//! Clock and playout primitives, kept independent from cpal/Opus/WebRTC.

use std::collections::{BTreeMap, VecDeque};

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME_SAMPLES: u32 = 960; // 20 ms at 48 kHz

#[derive(Debug, Default, Clone)]
pub struct ClockSync { samples: VecDeque<(f64, f64)>, pub offset_seconds: Option<f64>, pub rtt_seconds: Option<f64> }

impl ClockSync {
    /// Records an NTP-like sample: host sent at t1, we received at t2, and answer at t3.
    pub fn add_sample(&mut self, t1: f64, t2: f64, t3: f64) {
        let rtt = t3 - t1;
        if rtt.is_sign_negative() { return; }
        self.samples.push_back((rtt, t2 - (t1 + t3) / 2.0));
        if self.samples.len() > 12 { self.samples.pop_front(); }
        if let Some((rtt, best)) = self.samples.iter().min_by(|a, b| a.0.total_cmp(&b.0)).copied() {
            self.offset_seconds = Some(self.offset_seconds.map_or(best, |old| old + 0.25 * (best - old)));
            self.rtt_seconds = Some(rtt);
        }
    }
}

#[derive(Debug, Default)]
pub struct JitterBuffer<T> { packets: BTreeMap<u32, T>, pub gaps: u64, pub resyncs: u64, next: Option<u32> }

impl<T> JitterBuffer<T> {
    pub fn push(&mut self, index: u32, packet: T) { self.packets.insert(index, packet); }
    pub fn pop_for(&mut self, expected: u32) -> Option<T> {
        match self.next {
            None => self.next = Some(expected),
            Some(position) if position.abs_diff(expected) > 8 => { self.next = Some(expected); self.resyncs += 1; }
            _ => {}
        }
        let position = self.next?;
        self.next = Some(position.wrapping_add(1));
        let packet = self.packets.remove(&position);
        if packet.is_none() { self.gaps += 1; }
        self.packets.retain(|index, _| *index + 600 >= position);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn clock_prefers_lowest_rtt() { let mut c = ClockSync::default(); c.add_sample(0.0, 1.0, 4.0); c.add_sample(10.0, 11.2, 12.0); assert!(c.offset_seconds.unwrap() > 0.9); }
    #[test] fn jitter_buffer_tracks_gap() { let mut b = JitterBuffer::default(); b.push(5, "a"); assert_eq!(b.pop_for(5), Some("a")); assert_eq!(b.pop_for(6), None); assert_eq!(b.gaps, 1); }
}
