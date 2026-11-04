//! Mixing primitives and per-channel controls.
//!
//! `output[n] = limiter(master * sum(gain[i] * input[i][n]))`
//!
//! Controls are atomics so the control thread (API, CLI) can change gain/mute
//! while the real-time thread reads them without locks.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};

/// Lock-free f32 cell (stored as bits).
#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    #[inline]
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Relaxed))
    }
    #[inline]
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Relaxed)
    }
}

/// State of one mixer input, shared between control and audio threads.
#[derive(Debug)]
pub struct ChannelControl {
    pub name: String,
    pub gain: AtomicF32,
    pub muted: AtomicBool,
    /// Peak of the last block, before gain (0..=1+).
    pub peak: AtomicF32,
    /// True while the input delivers audio to the mixer.
    pub streaming: AtomicBool,
    /// Mixer wanted audio but the input buffer was empty.
    pub underruns: AtomicU64,
    /// Input buffer was full and samples were dropped.
    pub overruns: AtomicU64,
    /// Samples skipped to bring latency back down (clock drift).
    pub drift_skips: AtomicU64,
    /// Interleaved samples currently buffered.
    pub buffered: AtomicU32,
}

impl ChannelControl {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            gain: AtomicF32::new(1.0),
            muted: AtomicBool::new(false),
            peak: AtomicF32::new(0.0),
            streaming: AtomicBool::new(false),
            underruns: AtomicU64::new(0),
            overruns: AtomicU64::new(0),
            drift_skips: AtomicU64::new(0),
            buffered: AtomicU32::new(0),
        }
    }
}

/// `dst[i] += src[i] * gain`
#[inline]
pub fn accumulate(dst: &mut [f32], src: &[f32], gain: f32) {
    for (d, s) in dst.iter_mut().zip(src) {
        *d += s * gain;
    }
}

#[inline]
pub fn apply_gain(buf: &mut [f32], gain: f32) {
    if gain != 1.0 {
        for s in buf {
            *s *= gain;
        }
    }
}

#[inline]
pub fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

/// Replaces NaN/inf samples (corrupt input) with silence.
#[inline]
pub fn sanitize(buf: &mut [f32]) {
    for s in buf {
        if !s.is_finite() {
            *s = 0.0;
        }
    }
}

/// True if every sample is below `threshold` (about -60 dBFS for 0.001).
#[inline]
pub fn is_silent(buf: &[f32], threshold: f32) -> bool {
    buf.iter().all(|s| s.abs() < threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulate_sums_with_gain() {
        let mut out = vec![0.0f32; 4];
        accumulate(&mut out, &[1.0, 1.0, 1.0, 1.0], 0.5);
        accumulate(&mut out, &[0.2, -0.2, 0.0, 1.0], 1.0);
        assert_eq!(out, vec![0.7, 0.3, 0.5, 1.5]);
    }

    #[test]
    fn gain_and_peak() {
        let mut b = vec![0.5, -0.8, 0.1];
        apply_gain(&mut b, 0.5);
        assert_eq!(b, vec![0.25, -0.4, 0.05]);
        assert_eq!(peak(&b), 0.4);
    }

    #[test]
    fn silence_detection() {
        assert!(is_silent(&[0.0, 0.0005, -0.0009], 0.001));
        assert!(!is_silent(&[0.0, 0.01], 0.001));
    }

    #[test]
    fn sanitize_removes_invalid() {
        let mut b = vec![f32::NAN, 0.5, f32::NEG_INFINITY];
        sanitize(&mut b);
        assert_eq!(b, vec![0.0, 0.5, 0.0]);
    }

    #[test]
    fn atomic_f32_roundtrip() {
        let a = AtomicF32::new(0.25);
        a.set(-1.5);
        assert_eq!(a.get(), -1.5);
    }
}
