//! The mixing engine.
//!
//! Split in two halves:
//! - [`Engine`] lives in the real-time output callback. It reads every input
//!   buffer, mixes, limits and fills the output. It never allocates, blocks or locks.
//! - [`EngineHandle`] lives on the control thread. It adds/removes inputs by
//!   sending commands through a lock-free queue, and frees removed inputs that the
//!   engine sends back (so memory is never freed on the real-time thread).
//!
//! Smart mixing ("ducking"): the input that most recently **started making sound**
//! is in the foreground. Every other input is lowered to `ducking.level_db` with a
//! smooth fade. When the foreground input has been silent for `hold_ms`, the
//! previous one becomes foreground again and fades back to its level.

use super::buffer::{input_buffer, InputConsumer, InputProducer, ReadStatus};
use super::limiter::{hard_clip, Limiter};
use super::mixer::{self, AtomicF32, ChannelControl};
use crate::config::{AudioConfig, DuckingConfig};
use crate::errors::EngineError;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

/// Largest block processed in one pass (frames). Larger requests are split.
pub const MAX_BLOCK_FRAMES: usize = 8192;

/// Time constant for user gain / mute changes (avoids clicks).
const GAIN_SMOOTH_MS: f32 = 15.0;

/// Master section state, shared with the control thread.
#[derive(Debug)]
pub struct MasterControl {
    pub gain: AtomicF32,
    pub muted: AtomicBool,
    pub peak: AtomicF32,
    pub limiter_gain: AtomicF32,
    pub limiter_enabled: AtomicBool,
    pub ducking_enabled: AtomicBool,
    /// Level of background inputs while another one plays (dB, ≤ 0).
    pub ducking_level_db: AtomicF32,
    /// Number of output callbacks run (useful to check the engine is alive).
    pub cycles: AtomicU64,
    /// Output callbacks where no buffer was available.
    pub xruns: AtomicU64,
}

/// Per-input activity tracking for ducking (real-time side only).
#[derive(Default)]
struct Activity {
    active: bool,
    /// Frame clock when the input became active (newest = foreground).
    since: u64,
    /// Frame clock of the last block above the threshold.
    last_loud: u64,
}

struct Slot {
    consumer: InputConsumer,
    control: Arc<ChannelControl>,
    activity: Activity,
    /// Smoothed ducking gain (1 = not ducked).
    duck: f32,
    /// Smoothed user gain (volume × mute).
    gain: f32,
}

enum Command {
    Add(usize, Box<Slot>),
    Remove(usize),
}

/// Ducking parameters converted to the engine's units.
struct DuckParams {
    threshold: f32,
    hold_frames: u64,
    attack_k: f32,
    release_k: f32,
}

/// One-pole smoothing coefficient for a time constant in ms.
fn smooth_k(ms: f32, sample_rate: u32) -> f32 {
    1.0 - (-1.0 / (ms.max(0.1) / 1000.0 * sample_rate as f32)).exp()
}

pub fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
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
    duck: DuckParams,
    gain_k: f32,
    /// Frames processed since start.
    clock: u64,
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

/// Creates both halves from the configuration.
pub fn new_engine(cfg: &AudioConfig, ducking: &DuckingConfig) -> (EngineHandle, Engine) {
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
        ducking_enabled: AtomicBool::new(ducking.enabled),
        ducking_level_db: AtomicF32::new(ducking.level_db),
        cycles: AtomicU64::new(0),
        xruns: AtomicU64::new(0),
    });
    let prefill = cfg.prefill_samples();
    // Room for 4x the latency target, and at least 8 large blocks.
    let buffer_capacity = (prefill * 4).max(cfg.buffer_size as usize * channels * 8);
    let rate = cfg.sample_rate;

    let engine = Engine {
        channels,
        slots: (0..max).map(|_| None).collect(),
        scratch: vec![0.0; MAX_BLOCK_FRAMES * channels],
        mix: vec![0.0; MAX_BLOCK_FRAMES * channels],
        commands: cmd_rx,
        trash: trash_tx,
        limiter: Limiter::new(rate, cfg.limiter_threshold, 80.0),
        master: master.clone(),
        duck: DuckParams {
            threshold: db_to_linear(ducking.threshold_db),
            hold_frames: ducking.hold_ms as u64 * rate as u64 / 1000,
            attack_k: smooth_k(ducking.attack_ms as f32, rate),
            release_k: smooth_k(ducking.release_ms as f32, rate),
        },
        gain_k: smooth_k(GAIN_SMOOTH_MS, rate),
        clock: 0,
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

/// Root mean square of a block (all channels).
fn rms(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
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
                        old.control.active.store(false, Relaxed);
                        old.control.peak.set(0.0);
                        let _ = self.trash.push(old);
                    }
                }
            }
        }
    }

    /// Index of the foreground input: the active one that started most recently.
    fn foreground(&self) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().filter(|s| s.activity.active).map(|s| (i, s.activity.since)))
            .max_by_key(|&(i, since)| (since, i))
            .map(|(i, _)| i)
    }

    fn process_block(&mut self, out: &mut [f32]) {
        let n = out.len();
        let ch = self.channels;
        let frames = (n / ch) as u64;
        let clock = self.clock;

        // Ducking decision from the activity seen so far (one block of delay).
        let ducking = self.master.ducking_enabled.load(Relaxed);
        let fg = if ducking { self.foreground() } else { None };
        let duck_level = db_to_linear(self.master.ducking_level_db.get().min(0.0));
        let (gain_k, d) = (self.gain_k, &self.duck);

        let mix = &mut self.mix[..n];
        mix.fill(0.0);

        for (i, slot) in self.slots.iter_mut().enumerate() {
            let Some(slot) = slot else { continue };
            let src = &mut self.scratch[..n];
            let ctl = &*slot.control;
            let status = slot.consumer.read(src, ctl);
            match status {
                ReadStatus::Buffering => {
                    ctl.streaming.store(false, Relaxed);
                }
                ReadStatus::Underrun(_) => {
                    ctl.underruns.fetch_add(1, Relaxed);
                    ctl.streaming.store(false, Relaxed);
                }
                ReadStatus::Full => ctl.streaming.store(true, Relaxed),
            }
            mixer::sanitize(src);
            ctl.peak.set(mixer::peak(src));

            // Activity: became loud → active (and newest); quiet for hold → inactive.
            let a = &mut slot.activity;
            if rms(src) > d.threshold {
                if !a.active {
                    a.active = true;
                    a.since = clock;
                }
                a.last_loud = clock + frames;
            } else if a.active && clock.saturating_sub(a.last_loud) > d.hold_frames {
                a.active = false;
            }
            ctl.active.store(a.active, Relaxed);

            let duck_target = match fg {
                Some(f) if f != i => duck_level,
                _ => 1.0,
            };
            let gain_target = if ctl.muted.load(Relaxed) { 0.0 } else { ctl.gain.get() };
            let duck_k = if duck_target < slot.duck { d.attack_k } else { d.release_k };

            for (dst, frame) in mix.chunks_exact_mut(ch).zip(src.chunks_exact(ch)) {
                slot.duck += (duck_target - slot.duck) * duck_k;
                slot.gain += (gain_target - slot.gain) * gain_k;
                let g = slot.duck * slot.gain;
                for (o, s) in dst.iter_mut().zip(frame) {
                    *o += s * g;
                }
            }
            ctl.duck_gain.set(slot.duck);
            ctl.ducked.store(duck_target < 1.0, Relaxed);
        }
        self.clock += frames;

        let master_gain = if self.master.muted.load(Relaxed) {
            0.0
        } else {
            self.master.gain.get()
        };
        mixer::apply_gain(mix, master_gain);
        if self.master.limiter_enabled.load(Relaxed) {
            self.limiter.process(mix, ch);
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
                    activity: Activity::default(),
                    duck: 1.0,
                    gain: 1.0,
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

    const BLOCK: usize = 256; // samples = 128 stereo frames

    fn cfg() -> AudioConfig {
        AudioConfig {
            input_latency_ms: 5, // 240 frames = 480 samples prefill
            master_volume: 1.0,
            limiter_enabled: false,
            ..AudioConfig::default()
        }
    }

    fn no_duck() -> DuckingConfig {
        DuckingConfig {
            enabled: false,
            ..DuckingConfig::default()
        }
    }

    /// Feeds each producer one block of `levels[i]` (None = nothing) and processes one block.
    fn step(e: &mut Engine, ps: &mut [InputProducer], levels: &[Option<f32>]) -> Vec<f32> {
        for (p, l) in ps.iter_mut().zip(levels) {
            if let Some(v) = l {
                p.push(&[*v; BLOCK]);
            }
        }
        let mut out = vec![0.0f32; BLOCK];
        e.process(&mut out);
        out
    }

    fn run(e: &mut Engine, ps: &mut [InputProducer], levels: &[Option<f32>], blocks: usize) -> Vec<f32> {
        let mut out = Vec::new();
        for _ in 0..blocks {
            out = step(e, ps, levels);
        }
        out
    }

    fn approx(out: &[f32], v: f32, tol: f32) -> bool {
        out.iter().all(|s| (s - v).abs() < tol)
    }

    #[test]
    fn empty_engine_outputs_silence() {
        let (_h, mut e) = new_engine(&cfg(), &no_duck());
        let mut out = vec![1.0f32; 512];
        e.process(&mut out);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn mixes_two_inputs_with_gain_and_mute() {
        let (mut h, mut e) = new_engine(&cfg(), &no_duck());
        let (a, pa) = h.add_input("a").unwrap();
        let (b, pb) = h.add_input("b").unwrap();
        let mut ps = [pa, pb];
        h.channel(a).unwrap().gain.set(2.0);
        let out = run(&mut e, &mut ps, &[Some(0.25), Some(0.5)], 40);
        assert!(approx(&out, 1.0, 1e-3), "{:?}", &out[..4]);

        h.channel(b).unwrap().muted.store(true, Relaxed);
        let out = run(&mut e, &mut ps, &[Some(0.25), Some(0.5)], 40);
        assert!(approx(&out, 0.5, 1e-3), "{:?}", &out[..4]);
    }

    #[test]
    fn limiter_prevents_clipping_with_many_sources() {
        let mut c = cfg();
        c.limiter_enabled = true;
        c.limiter_threshold = 0.9;
        let (mut h, mut e) = new_engine(&c, &no_duck());
        let mut ps: Vec<_> = (0..6).map(|i| h.add_input(&format!("s{i}")).unwrap().1).collect();
        let out = run(&mut e, &mut ps, &[Some(0.8); 6], 10);
        assert!(out.iter().all(|s| s.abs() <= 0.9 + 1e-6));
        assert!(h.master().peak.get() > 0.5);
    }

    #[test]
    fn removing_input_mid_stream_keeps_others_playing() {
        let (mut h, mut e) = new_engine(&cfg(), &no_duck());
        let (a, pa) = h.add_input("a").unwrap();
        let (_b, pb) = h.add_input("b").unwrap();
        let mut ps = vec![pa, pb];
        run(&mut e, &mut ps, &[Some(0.1), Some(0.2)], 10);
        h.remove_input(a).unwrap();
        ps.remove(0);
        let out = run(&mut e, &mut ps, &[Some(0.2)], 10);
        assert!(approx(&out, 0.2, 1e-4));
        h.collect_garbage();
        assert_eq!(h.add_input("c").unwrap().0, a); // slot is free again
    }

    #[test]
    fn slot_exhaustion_is_an_error() {
        let mut c = cfg();
        c.max_inputs = 1;
        let (mut h, _e) = new_engine(&c, &no_duck());
        h.add_input("a").unwrap();
        assert!(matches!(h.add_input("b"), Err(EngineError::NoFreeSlot(1))));
    }

    #[test]
    fn master_mute_and_gain() {
        let (mut h, mut e) = new_engine(&cfg(), &no_duck());
        let (_a, pa) = h.add_input("a").unwrap();
        let mut ps = [pa];
        h.master().gain.set(0.5);
        let out = run(&mut e, &mut ps, &[Some(0.5)], 10);
        assert!(approx(&out, 0.25, 1e-4));
        h.master().muted.store(true, Relaxed);
        let out = step(&mut e, &mut ps, &[Some(0.5)]);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn ducking_lowers_previous_source_then_restores_it() {
        let duck = DuckingConfig {
            enabled: true,
            level_db: -20.0, // ×0.1
            threshold_db: -40.0,
            hold_ms: 100,
            attack_ms: 5,
            release_ms: 5,
        };
        let (mut h, mut e) = new_engine(&cfg(), &duck);
        let (a, pa) = h.add_input("mac").unwrap();
        let (b, pb) = h.add_input("phone").unwrap();
        let mut ps = [pa, pb];

        // The Mac plays alone: full level.
        let out = run(&mut e, &mut ps, &[Some(0.5), None], 60);
        assert!(approx(&out, 0.5, 1e-3), "alone: {:?}", &out[..2]);

        // The phone starts: it takes the foreground, the Mac is lowered to 0.05.
        let out = run(&mut e, &mut ps, &[Some(0.5), Some(0.3)], 60);
        assert!(approx(&out, 0.05 + 0.3, 2e-3), "ducked: {:?}", &out[..2]);
        assert!(h.channel(a).unwrap().ducked.load(Relaxed));
        assert!(!h.channel(b).unwrap().ducked.load(Relaxed));

        // The phone stops: after the hold time the Mac returns to its level.
        let out = run(&mut e, &mut ps, &[Some(0.5), None], 120);
        assert!(approx(&out, 0.5, 2e-3), "restored: {:?}", &out[..2]);
        assert!(!h.channel(a).unwrap().ducked.load(Relaxed));
    }

    #[test]
    fn ducking_disabled_keeps_levels() {
        let (mut h, mut e) = new_engine(&cfg(), &no_duck());
        let (_a, pa) = h.add_input("a").unwrap();
        let (_b, pb) = h.add_input("b").unwrap();
        let mut ps = [pa, pb];
        run(&mut e, &mut ps, &[Some(0.2), None], 30);
        let out = run(&mut e, &mut ps, &[Some(0.2), Some(0.3)], 60);
        assert!(approx(&out, 0.5, 1e-3));
    }
}
