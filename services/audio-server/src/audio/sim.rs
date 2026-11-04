//! Simulated audio inputs: sine tones fed through the same ring buffers as real
//! Bluetooth inputs. Used to test and benchmark the mixer without devices.

use super::buffer::InputProducer;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct SimSource {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SimSource {
    /// Starts a thread producing a sine tone at `freq` Hz, in real time.
    pub fn spawn(
        mut producer: InputProducer,
        freq: f32,
        amplitude: f32,
        sample_rate: u32,
        channels: usize,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let thread = std::thread::Builder::new()
            .name(format!("sim-{freq}Hz"))
            .spawn(move || {
                let step = std::f32::consts::TAU * freq / sample_rate as f32;
                let mut phase = 0.0f32;
                let start = Instant::now();
                let mut produced: u64 = 0; // frames
                let mut block = vec![0.0f32; 4800 * channels];
                while !stop2.load(Relaxed) {
                    // Produce exactly as many frames as real time requires.
                    let due = (start.elapsed().as_secs_f64() * sample_rate as f64) as u64;
                    let frames = (due - produced).min(4800) as usize;
                    if frames > 0 {
                        let buf = &mut block[..frames * channels];
                        for frame in buf.chunks_exact_mut(channels) {
                            let v = phase.sin() * amplitude;
                            frame.fill(v);
                            phase = (phase + step) % std::f32::consts::TAU;
                        }
                        producer.push(buf);
                        produced += frames as u64;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
            .expect("spawn simulated source thread");
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for SimSource {
    fn drop(&mut self) {
        self.stop.store(true, Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
