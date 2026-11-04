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
    /// Above this many buffered samples, old audio is dropped (drift control).
    max_fill: usize,
    primed: bool,
}

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
            // Allow 2x the target before skipping, to avoid constant corrections.
            max_fill: (prefill * 2).min(capacity) / channels * channels,
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

        // Clock drift: the source runs slightly faster than our output.
        // Drop the excess, keeping `prefill` samples of latency.
        if avail > self.max_fill {
            let skip = (avail - self.prefill) / self.channels * self.channels;
            if let Ok(chunk) = self.inner.read_chunk(skip) {
                chunk.commit_all();
            }
            control.drift_skips.fetch_add(1, Relaxed);
            avail -= skip;
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
    fn drift_skip_keeps_latency_bounded() {
        let (mut p, mut c, ctl) = make(128, 8); // max_fill = 16
        p.push(&(0..40).map(|i| i as f32).collect::<Vec<_>>());
        let mut out = [0.0f32; 4];
        assert_eq!(c.read(&mut out, &ctl), ReadStatus::Full);
        // 40 buffered > 16: skip 32 → keep the newest 8, read 4 of them.
        assert_eq!(out, [32.0, 33.0, 34.0, 35.0]);
        assert_eq!(ctl.drift_skips.load(Relaxed), 1);
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
