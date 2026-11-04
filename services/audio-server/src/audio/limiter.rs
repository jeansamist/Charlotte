//! Peak limiter: instant attack, exponential release.
//!
//! Guarantees `|output| <= threshold` for finite input, so several loud
//! sources mixed together never clip. Real-time safe: no allocation, no locks.

pub struct Limiter {
    threshold: f32,
    release_coef: f32,
    gain: f32,
}

impl Limiter {
    /// `release_ms`: time for the gain to recover ~63% of the way back to 1.0.
    pub fn new(sample_rate: u32, threshold: f32, release_ms: f32) -> Self {
        let release_samples = (release_ms / 1000.0 * sample_rate as f32).max(1.0);
        Self {
            threshold,
            release_coef: (-1.0 / release_samples).exp(),
            gain: 1.0,
        }
    }

    /// Current gain reduction (1.0 = not limiting).
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Processes interleaved frames in place.
    pub fn process(&mut self, buf: &mut [f32], channels: usize) {
        for frame in buf.chunks_exact_mut(channels) {
            let peak = frame.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            let target = if peak > self.threshold {
                self.threshold / peak
            } else {
                1.0
            };
            if target < self.gain {
                self.gain = target; // instant attack
            } else {
                self.gain = target + (self.gain - target) * self.release_coef;
            }
            // The release can overshoot the ceiling for one frame; re-check.
            let g = if peak * self.gain > self.threshold {
                self.threshold / peak
            } else {
                self.gain
            };
            for s in frame.iter_mut() {
                *s *= g;
            }
        }
    }
}

/// Final safety net: replaces NaN/inf and hard-clamps to [-1, 1].
#[inline]
pub fn hard_clip(buf: &mut [f32]) {
    for s in buf {
        *s = if s.is_finite() { s.clamp(-1.0, 1.0) } else { 0.0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_signal_untouched() {
        let mut l = Limiter::new(48_000, 0.95, 50.0);
        let mut buf = vec![0.5, -0.5, 0.25, -0.25];
        l.process(&mut buf, 2);
        assert_eq!(buf, vec![0.5, -0.5, 0.25, -0.25]);
    }

    #[test]
    fn loud_signal_never_exceeds_threshold() {
        let mut l = Limiter::new(48_000, 0.9, 50.0);
        // three full-scale sources summed = 3.0
        let mut buf: Vec<f32> = (0..4800)
            .map(|i| 3.0 * (i as f32 * 0.05).sin())
            .collect();
        l.process(&mut buf, 2);
        assert!(buf.iter().all(|s| s.abs() <= 0.9 + 1e-6));
    }

    #[test]
    fn gain_recovers_after_peak() {
        let mut l = Limiter::new(48_000, 0.9, 10.0);
        let mut loud = vec![2.0f32; 2];
        l.process(&mut loud, 2);
        assert!(l.gain() < 0.5);
        let mut quiet = vec![0.1f32; 48_000]; // 0.5 s
        l.process(&mut quiet, 2);
        assert!(l.gain() > 0.99);
    }

    #[test]
    fn hard_clip_sanitizes() {
        let mut buf = vec![f32::NAN, f32::INFINITY, 1.5, -2.0, 0.3];
        hard_clip(&mut buf);
        assert_eq!(buf, vec![0.0, 0.0, 1.0, -1.0, 0.3]);
    }
}
