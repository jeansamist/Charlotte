//! State shared between the PipeWire main thread and the HTTP API thread.
//!
//! Gain / mute / peaks are atomics inside `ChannelControl` and `MasterControl`,
//! so the API changes them directly. Lists (inputs, outputs) are small and change
//! rarely; a plain `Mutex` is fine (never touched by the real-time thread).

use crate::audio::engine::MasterControl;
use crate::audio::mixer::ChannelControl;
use crate::monitoring::ProcessSampler;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct InputEntry {
    pub slot: usize,
    /// PipeWire node id (None for simulated inputs).
    pub node_id: Option<u32>,
    pub node_name: String,
    pub device: String,
    pub kind: &'static str,
    pub control: Arc<ChannelControl>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub kind: &'static str,
}

pub struct Shared {
    pub started_at: u64,
    pub version: &'static str,
    pub master: Arc<MasterControl>,
    pub inputs: Mutex<Vec<InputEntry>>,
    pub outputs: Mutex<Vec<OutputEntry>>,
    pub sampler: Mutex<ProcessSampler>,
    pub bluetooth_adapter: Option<String>,
    /// Interleaved samples per millisecond (for buffer → ms conversions).
    pub samples_per_ms: f32,
}

impl Shared {
    pub fn add_input(&self, e: InputEntry) {
        let mut v = self.inputs.lock().unwrap();
        v.retain(|x| x.slot != e.slot);
        v.push(e);
        v.sort_by_key(|x| x.slot);
    }

    pub fn remove_input(&self, slot: usize) {
        self.inputs.lock().unwrap().retain(|x| x.slot != slot);
    }

    pub fn add_output(&self, e: OutputEntry) {
        let mut v = self.outputs.lock().unwrap();
        v.retain(|x| x.id != e.id);
        v.push(e);
        v.sort_by_key(|x| x.id);
    }

    pub fn remove_output(&self, id: u32) {
        self.outputs.lock().unwrap().retain(|x| x.id != id);
    }
}
