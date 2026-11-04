//! Per-input ring buffer between a capture callback (producer) and the mixer
//! callback (consumer). Wraps `rtrb`: lock-free, preallocated, real-time safe.

use super::mixer::ChannelControl;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

/// Writing side, owned by a capture stream (or a simulated source).
pub struct InputProducer {
    inner: rtrb::Producer<f32>,
    control: Arc<ChannelControl>,
    channels: usize,
}

/// Reading side, owned by the mixer engine.
pub struct InputConsumer {
    inner: rtrb::Consumer<f32>,
    channels: usize,
    /// Samples to buffer before playing (latency target).
    prefill: usize,
    /// Above this many buffered samples, old audio is dropped at once (emergency only).
    max_fill: usize,
    /// Dead band around `prefill` before the gentle drift correction acts.
    band: f32,
    /// Smoothed buffer level (samples). Bursty Bluetooth delivery makes the
    /// instantaneous level jump; corrections follow the average.
    level: f32,
    primed: bool,
}

/// Smoothing of the buffer level (≈ 1 s with 10 ms blocks).
const LEVEL_ALPHA: f32 = 0.01;

/// Creates a buffer that holds `capacity` interleaved samples.
pub fn input_buffer(
    capacity: usize,
    prefill: usize,
    channels: usize,
    control: Arc<ChannelControl>,
) -> (InputProducer, InputConsumer) {
    let (p, c) = rtrb::RingBuffer::new(capacity);
    let prefill = prefill.min(capacity / 2) / channels * channels;
    (
        InputProducer {
            inner: p,
            control,
            channels,
        },
        InputConsumer {
            inner: c,
            channels,
            prefill,
            // Gentle correction handles normal drift; jumps only after a long stall.
            max_fill: (prefill * 4).min(capacity) / channels * channels,
            band: (prefill / 4).max(channels * 64) as f32,
            level: prefill as f32,
            primed: false,
        },
    )
}

impl InputProducer {
    /// Pushes interleaved samples; drops what does not fit (counted as overrun).
    pub fn push(&mut self, samples: &[f32]) -> usize {
        self.push_iter(samples.len(), samples.iter().copied())
    }

    /// Pushes `len` samples from an iterator (lets callers decode bytes without a copy).
    pub fn push_iter(&mut self, len: usize, mut it: impl Iterator<Item = f32>) -> usize {
        let free = self.inner.slots() / self.channels * self.channels;
        let n = len.min(free);
        if n < len {
            self.control.overruns.fetch_add(1, Relaxed);
        }
        if n == 0 {
            return 0;
        }
        if let Ok(mut chunk) = self.inner.write_chunk(n) {
            let (a, b) = chunk.as_mut_slices();
            for s in a.iter_mut().chain(b.iter_mut()) {
                *s = it.next().unwrap_or(0.0);
            }
            chunk.commit_all();
        }
        n
    }

    pub fn control(&self) -> &Arc<ChannelControl> {
        &self.control
    }
}

/// Result of [`InputConsumer::read`].
#[derive(Debug, PartialEq, Eq)]
pub enum ReadStatus {
    /// Still filling the latency buffer; `dst` is silence.
    Buffering,
    /// `dst` is fully filled with audio.
    Full,
    /// Ran dry; `dst` is partly audio (count) then silence. Will re-buffer.
    Underrun(usize),
}

impl InputConsumer {
    /// Fills `dst` (interleaved) from the buffer. Never blocks, never allocates.
    pub fn read(&mut self, dst: &mut [f32], control: &ChannelControl) -> ReadStatus {
        let mut avail = self.inner.slots();
        control.buffered.store(avail as u32, Relaxed);

        if !self.primed {
            if avail < self.prefill.max(dst.len()) {
                dst.fill(0.0);
                return ReadStatus::Buffering;
            }
            self.primed = true;
        }

        // Emergency: far too much audio queued (e.g. after a stall). Jump back to
        // the latency target.
        if avail > self.max_fill {
            let skip = (avail - self.prefill) / self.channels * self.channels;
            if let Ok(chunk) = self.inner.read_chunk(skip) {
                chunk.commit_all();
            }
            control.drift_skips.fetch_add(1, Relaxed);
            avail -= skip;
            self.level = avail as f32;
        }

        // Clock drift: the sender's clock is slightly faster or slower than the
        // output's. Remove or repeat ONE frame per block (≈0.2% speed change),
        // which is inaudible, until the average level is back near the target.
        self.level += (avail as f32 - self.level) * LEVEL_ALPHA;
        let ch = self.channels;
        let target = self.prefill as f32;
        if dst.len() >= 2 * ch {
            if self.level > target + self.band && avail >= dst.len() + ch {
                self.read_dropping_frame(dst);
                control.slips.fetch_add(1, Relaxed);
                return ReadStatus::Full;
            }
            if self.level < target - self.band && avail >= dst.len() {
                self.read_repeating_frame(dst);
                control.slips.fetch_add(1, Relaxed);
                return ReadStatus::Full;
            }
        }

        let n = dst.len().min(avail) / self.channels * self.channels;
        if n > 0 {
            if let Ok(chunk) = self.inner.read_chunk(n) {
                let (a, b) = chunk.as_slices();
                dst[..a.len()].copy_from_slice(a);
                dst[a.len()..n].copy_from_slice(b);
                chunk.commit_all();
            }
        }
        if n < dst.len() {
            dst[n..].fill(0.0);
            self.primed = false;
            return ReadStatus::Underrun(n);
        }
        ReadStatus::Full
    }

    /// Fills `dst` from `dst.len() + 1 frame` of input: the last two frames are
    /// averaged into one, so the stream advances one frame faster.
    fn read_dropping_frame(&mut self, dst: &mut [f32]) {
        let (n, ch) = (dst.len(), self.channels);
        let Ok(chunk) = self.inner.read_chunk(n + ch) else { return };
        let (a, b) = chunk.as_slices();
        let get = |i: usize| if i < a.len() { a[i] } else { b[i - a.len()] };
        for (i, d) in dst[..n - ch].iter_mut().enumerate() {
            *d = get(i);
        }
        for c in 0..ch {
            dst[n - ch + c] = 0.5 * (get(n - ch + c) + get(n + c));
        }
        chunk.commit_all();
    }

    /// Fills `dst` from `dst.len() - 1 frame` of input and repeats the last
    /// frame, so the stream advances one frame slower.
    fn read_repeating_frame(&mut self, dst: &mut [f32]) {
        let (n, ch) = (dst.len(), self.channels);
        let Ok(chunk) = self.inner.read_chunk(n - ch) else { return };
        let (a, b) = chunk.as_slices();
        dst[..a.len()].copy_from_slice(a);
        dst[a.len()..n - ch].copy_from_slice(b);
        chunk.commit_all();
        let (head, tail) = dst.split_at_mut(n - ch);
        tail.copy_from_slice(&head[n - 2 * ch..]);
    }

    /// True if the producer side was dropped (input stream destroyed).
    pub fn is_abandoned(&self) -> bool {
        self.inner.is_abandoned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(cap: usize, prefill: usize) -> (InputProducer, InputConsumer, Arc<ChannelControl>) {
        let ctl = Arc::new(ChannelControl::new("t"));
        let (p, c) = input_buffer(cap, prefill, 2, ctl.clone());
        (p, c, ctl)
    }

    #[test]
    fn buffers_until_prefill() {
        let (mut p, mut c, ctl) = make(64, 8);
        let mut out = [9.0f32; 4];
        p.push(&[1.0; 4]);
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Buffering);
        assert_eq!(out, [0.0; 4]);
        p.push(&[1.0; 4]);
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Full);
        assert_eq!(out, [1.0; 4]);
    }

    #[test]
    fn underrun_pads_with_silence_and_rebuffers() {
        let (mut p, mut c, ctl) = make(64, 4);
        p.push(&[0.5; 6]);
        let mut out = [0.0f32; 4];
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Full);
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Underrun(2));
        assert_eq!(out, [0.5, 0.5, 0.0, 0.0]);
        p.push(&[0.5; 2]);
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Buffering);
    }

    #[test]
    fn overrun_drops_and_counts() {
        let (mut p, _c, ctl) = make(8, 4);
        assert_eq!(p.push(&[1.0; 12]), 8);
        assert_eq!(ctl.overruns.load(Relaxed), 1);
    }

    #[test]
    fn emergency_skip_after_a_stall() {
        let (mut p, mut c, ctl) = make(256, 8); // max_fill = 32
        p.push(&(0..80).map(|i| i as f32).collect::<Vec<_>>());
        let mut out = [0.0f32; 4];
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Full);
        // 80 buffered > 32: skip 72 → keep the newest 8, read 4 of them.
        assert_eq!(out, [72.0, 73.0, 74.0, 75.0]);
        assert_eq!(ctl.drift_skips.load(Relaxed), 1);
    }

    /// Simulates a sender whose clock is `ppm` faster (or slower if negative)
    /// than the output; returns (final level in samples, slips, emergency skips).
    fn simulate_drift(ppm: i64) -> (usize, u64, u64) {
        let ch = 2;
        let block = 512 * ch; // 512 frames per callback
        let prefill = 2880 * ch; // 60 ms
        let ctl = Arc::new(ChannelControl::new("t"));
        let (mut p, mut c) = input_buffer(prefill * 8, prefill, ch, ctl.clone());
        let mut out = vec![0.0f32; block];
        let mut owed = 0i64; // extra frames ×1e6
        let mut val = 0.0f32;
        // 10 minutes of audio.
        for _ in 0..(48_000 * 600 / 512) {
            owed += 512 * ppm;
            let extra = owed / 1_000_000;
            owed -= extra * 1_000_000;
            let frames = (512 + extra) as usize;
            let data: Vec<f32> = (0..frames * ch).map(|_| { val += 1.0; val }).collect();
            p.push(&data);
            c.read(&mut out, &ctl);
        }
        (c.inner.slots(), ctl.slips.load(Relaxed), ctl.drift_skips.load(Relaxed))
    }

    #[test]
    fn fast_sender_is_corrected_without_jumps() {
        let (level, slips, skips) = simulate_drift(300); // 300 ppm, worse than real devices
        assert_eq!(skips, 0, "no audible jump");
        assert!(slips > 0);
        let prefill = 2880 * 2;
        assert!(level.abs_diff(prefill) < prefill / 2, "level {level} stays near {prefill}");
    }

    #[test]
    fn slow_sender_is_corrected_without_underruns() {
        let (level, slips, skips) = simulate_drift(-300);
        assert_eq!(skips, 0);
        assert!(slips > 0);
        let prefill = 2880 * 2;
        assert!(level.abs_diff(prefill) < prefill / 2, "level {level} stays near {prefill}");
    }

    #[test]
    fn dropping_a_frame_averages_the_boundary() {
        let ctl = Arc::new(ChannelControl::new("t"));
        let (mut p, mut c) = input_buffer(64, 4, 2, ctl);
        p.push(&[1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 5.0, 5.0]);
        let mut out = [0.0f32; 6];
        c.read_dropping_frame(&mut out);
        assert_eq!(out, [1.0, 1.0, 2.0, 2.0, 4.0, 4.0]); // frames 3 and 4 merged
    }

    #[test]
    fn repeating_a_frame_holds_the_last_value() {
        let ctl = Arc::new(ChannelControl::new("t"));
        let (mut p, mut c) = input_buffer(64, 4, 2, ctl);
        p.push(&[1.0, 1.0, 2.0, 2.0]);
        let mut out = [0.0f32; 6];
        c.read_repeating_frame(&mut out);
        assert_eq!(out, [1.0, 1.0, 2.0, 2.0, 2.0, 2.0]);
    }

    #[test]
    fn wraparound_reads_are_contiguous() {
        let (mut p, mut c, ctl) = make(8, 4); // max_fill = 8: no drift skip
        let mut out = [0.0f32; 6];
        p.push(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        c.read(&mut out, &ctl);
        p.push(&[7.0, 8.0, 9.0, 10.0, 11.0, 12.0]); // wraps
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Full);
        assert_eq!(out, [7.0, 8.0, 9.0, 10.0, 11.0, 12.0]);
    }
}
