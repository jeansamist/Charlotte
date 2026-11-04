//! The mixing engine.
//!
//! Split in two halves:
//! - [`Engine`] lives in the real-time output callback. It reads every input
//!   buffer, mixes, limits and fills the output. It never allocates, blocks or locks.
//! - [`EngineHandle`] lives on the control thread. It adds/removes inputs by
//!   sending commands through a lock-free queue, and frees removed inputs that the
//!   engine sends back (so memory is never freed on the real-time thread).

use super::buffer::{input_buffer, InputConsumer, InputProducer, ReadStatus};
use super::limiter::{hard_clip, Limiter};
use super::mixer::{self, AtomicF32, ChannelControl};
use crate::config::AudioConfig;
use crate::errors::EngineError;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

/// Largest block processed in one pass (frames). Larger requests are split.
pub const MAX_BLOCK_FRAMES: usize = 8192;

/// Master section state, shared with the control thread.
#[derive(Debug)]
pub struct MasterControl {
    pub gain: AtomicF32,
    pub muted: AtomicBool,
    pub peak: AtomicF32,
    pub limiter_gain: AtomicF32,
    pub limiter_enabled: AtomicBool,
    /// Number of output callbacks run (useful to check the engine is alive).
    pub cycles: AtomicU64,
    /// Output callbacks where no buffer was available.
    pub xruns: AtomicU64,
}

struct Slot {
    consumer: InputConsumer,
    control: Arc<ChannelControl>,
}

enum Command {
    Add(usize, Box<Slot>),
    Remove(usize),
}

/// Real-time half. Owned by the output stream callback.
pub struct Engine {
    channels: usize,
    slots: Vec<Option<Box<Slot>>>,
    scratch: Vec<f32>,
    mix: Vec<f32>,
    commands: rtrb::Consumer<Command>,
    trash: rtrb::Producer<Box<Slot>>,
    limiter: Limiter,
    master: Arc<MasterControl>,
}

/// Control half. Owned by the main thread.
pub struct EngineHandle {
    commands: rtrb::Producer<Command>,
    trash: rtrb::Consumer<Box<Slot>>,
    used: Vec<Option<Arc<ChannelControl>>>,
    master: Arc<MasterControl>,
    buffer_capacity: usize,
    prefill: usize,
    channels: usize,
}

/// Creates both halves from the audio configuration.
pub fn new_engine(cfg: &AudioConfig) -> (EngineHandle, Engine) {
    let channels = cfg.channels as usize;
    let max = cfg.max_inputs;
    let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(max * 2 + 8);
    let (trash_tx, trash_rx) = rtrb::RingBuffer::new(max * 2 + 8);
    let master = Arc::new(MasterControl {
        gain: AtomicF32::new(cfg.master_volume),
        muted: AtomicBool::new(false),
        peak: AtomicF32::new(0.0),
        limiter_gain: AtomicF32::new(1.0),
        limiter_enabled: AtomicBool::new(cfg.limiter_enabled),
        cycles: AtomicU64::new(0),
        xruns: AtomicU64::new(0),
    });
    let prefill = cfg.prefill_samples();
    // Room for 4x the latency target, and at least 4 large blocks.
    let buffer_capacity = (prefill * 4).max(cfg.buffer_size as usize * channels * 8);

    let engine = Engine {
        channels,
        slots: (0..max).map(|_| None).collect(),
        scratch: vec![0.0; MAX_BLOCK_FRAMES * channels],
        mix: vec![0.0; MAX_BLOCK_FRAMES * channels],
        commands: cmd_rx,
        trash: trash_tx,
        limiter: Limiter::new(cfg.sample_rate, cfg.limiter_threshold, 80.0),
        master: master.clone(),
    };
    let handle = EngineHandle {
        commands: cmd_tx,
        trash: trash_rx,
        used: (0..max).map(|_| None).collect(),
        master,
        buffer_capacity,
        prefill,
        channels,
    };
    (handle, engine)
}

impl Engine {
    /// Fills `out` (interleaved) with the next block of mixed audio.
    pub fn process(&mut self, out: &mut [f32]) {
        self.apply_commands();
        self.master.cycles.fetch_add(1, Relaxed);
        let block = MAX_BLOCK_FRAMES * self.channels;
        for chunk in out.chunks_mut(block) {
            self.process_block(chunk);
        }
    }

    /// Call when the output had no buffer to fill this cycle.
    pub fn note_xrun(&self) {
        self.master.xruns.fetch_add(1, Relaxed);
    }

    fn apply_commands(&mut self) {
        while let Ok(cmd) = self.commands.pop() {
            match cmd {
                Command::Add(i, slot) => {
                    if let Some(old) = self.slots[i].replace(slot) {
                        let _ = self.trash.push(old);
                    }
                }
                Command::Remove(i) => {
                    if let Some(old) = self.slots[i].take() {
                        old.control.streaming.store(false, Relaxed);
                        old.control.peak.set(0.0);
                        let _ = self.trash.push(old);
                    }
                }
            }
        }
    }

    fn process_block(&mut self, out: &mut [f32]) {
        let n = out.len();
        let mix = &mut self.mix[..n];
        mix.fill(0.0);

        for slot in self.slots.iter_mut().flatten() {
            let src = &mut self.scratch[..n];
            let ctl = &*slot.control;
            let status = slot.consumer.read(src, ctl);
            match status {
                ReadStatus::Buffering => {
                    ctl.streaming.store(false, Relaxed);
                    ctl.peak.set(0.0);
                    continue;
                }
                ReadStatus::Underrun(_) => {
                    ctl.underruns.fetch_add(1, Relaxed);
                    ctl.streaming.store(false, Relaxed);
                }
                ReadStatus::Full => ctl.streaming.store(true, Relaxed),
            }
            mixer::sanitize(src);
            ctl.peak.set(mixer::peak(src));
            if !ctl.muted.load(Relaxed) {
                mixer::accumulate(mix, src, ctl.gain.get());
            }
        }

        let master_gain = if self.master.muted.load(Relaxed) {
            0.0
        } else {
            self.master.gain.get()
        };
        mixer::apply_gain(mix, master_gain);
        if self.master.limiter_enabled.load(Relaxed) {
            self.limiter.process(mix, self.channels);
            self.master.limiter_gain.set(self.limiter.gain());
        }
        hard_clip(mix);
        self.master.peak.set(mixer::peak(mix));
        out.copy_from_slice(mix);
    }
}

impl EngineHandle {
    /// Registers a new input. Returns its slot and the producer to feed it.
    pub fn add_input(&mut self, name: &str) -> Result<(usize, InputProducer), EngineError> {
        self.collect_garbage();
        let slot = self
            .used
            .iter()
            .position(Option::is_none)
            .ok_or(EngineError::NoFreeSlot(self.used.len()))?;
        let control = Arc::new(ChannelControl::new(name));
        let (producer, consumer) =
            input_buffer(self.buffer_capacity, self.prefill, self.channels, control.clone());
        self.commands
            .push(Command::Add(
                slot,
                Box::new(Slot {
                    consumer,
                    control: control.clone(),
                }),
            ))
            .map_err(|_| EngineError::QueueFull)?;
        self.used[slot] = Some(control);
        Ok((slot, producer))
    }

    /// Removes an input. Its memory is freed later by [`Self::collect_garbage`].
    pub fn remove_input(&mut self, slot: usize) -> Result<(), EngineError> {
        if self.used.get(slot).map_or(true, Option::is_none) {
            return Err(EngineError::UnknownSlot(slot));
        }
        self.commands
            .push(Command::Remove(slot))
            .map_err(|_| EngineError::QueueFull)?;
        self.used[slot] = None;
        Ok(())
    }

    /// Frees inputs the engine has released. Call periodically from the control thread.
    pub fn collect_garbage(&mut self) {
        while let Ok(slot) = self.trash.pop() {
            drop(slot);
        }
    }

    pub fn channel(&self, slot: usize) -> Option<&Arc<ChannelControl>> {
        self.used.get(slot).and_then(Option::as_ref)
    }

    pub fn channels(&self) -> impl Iterator<Item = (usize, &Arc<ChannelControl>)> {
        self.used
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
    }

    pub fn master(&self) -> &Arc<MasterControl> {
        &self.master
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AudioConfig {
        AudioConfig {
            input_latency_ms: 5, // 240 frames = 480 samples prefill
            master_volume: 1.0,
            limiter_enabled: false,
            ..AudioConfig::default()
        }
    }

    #[test]
    fn empty_engine_outputs_silence() {
        let (_h, mut e) = new_engine(&cfg());
        let mut out = vec![1.0f32; 512];
        e.process(&mut out);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn mixes_two_inputs_with_gain_and_mute() {
        let (mut h, mut e) = new_engine(&cfg());
        let (a, mut pa) = h.add_input("a").unwrap();
        let (b, mut pb) = h.add_input("b").unwrap();
        // 900 samples: above the 480-sample prefill, below the 960 drift limit.
        pa.push(&vec![0.25; 900]);
        pb.push(&vec![0.5; 900]);
        h.channel(a).unwrap().gain.set(2.0);

        let mut out = vec![0.0f32; 256];
        e.process(&mut out);
        assert!(out.iter().all(|s| (*s - 1.0).abs() < 1e-6), "{:?}", &out[..4]);

        h.channel(b).unwrap().muted.store(true, Relaxed);
        e.process(&mut out);
        assert!(out.iter().all(|s| (*s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn limiter_prevents_clipping_with_many_sources() {
        let mut c = cfg();
        c.limiter_enabled = true;
        c.limiter_threshold = 0.9;
        let (mut h, mut e) = new_engine(&c);
        let mut producers: Vec<_> = (0..6)
            .map(|i| h.add_input(&format!("s{i}")).unwrap().1)
            .collect();
        for p in &mut producers {
            p.push(&vec![0.8; 2048]);
        }
        let mut out = vec![0.0f32; 1024];
        e.process(&mut out);
        assert!(out.iter().all(|s| s.abs() <= 0.9 + 1e-6));
        assert!(h.master().peak.get() > 0.5);
    }

    #[test]
    fn removing_input_mid_stream_keeps_others_playing() {
        let (mut h, mut e) = new_engine(&cfg());
        let (a, mut pa) = h.add_input("a").unwrap();
        let (_b, mut pb) = h.add_input("b").unwrap();
        pa.push(&vec![0.1; 900]);
        pb.push(&vec![0.2; 900]);
        let mut out = vec![0.0f32; 256];
        e.process(&mut out);
        h.remove_input(a).unwrap();
        drop(pa);
        e.process(&mut out);
        assert!(out.iter().all(|s| (*s - 0.2).abs() < 1e-6));
        h.collect_garbage();
        // Slot is free again.
        assert_eq!(h.add_input("c").unwrap().0, a);
    }

    #[test]
    fn slot_exhaustion_is_an_error() {
        let mut c = cfg();
        c.max_inputs = 1;
        let (mut h, _e) = new_engine(&c);
        h.add_input("a").unwrap();
        assert!(matches!(h.add_input("b"), Err(EngineError::NoFreeSlot(1))));
    }

    #[test]
    fn master_mute_and_gain() {
        let (mut h, mut e) = new_engine(&cfg());
        let (_a, mut pa) = h.add_input("a").unwrap();
        pa.push(&vec![0.5; 900]);
        h.master().gain.set(0.5);
        let mut out = vec![0.0f32; 256];
        e.process(&mut out);
        assert!(out.iter().all(|s| (*s - 0.25).abs() < 1e-6));
        h.master().muted.store(true, Relaxed);
        e.process(&mut out);
        assert!(out.iter().all(|s| *s == 0.0));
    }
}
