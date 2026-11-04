//! Runtime wiring: PipeWire main loop, node discovery, inputs, output, status.
//!
//! Threads:
//! - main thread: PipeWire main loop (registry events, stream setup, status timer).
//! - PipeWire data thread (real-time): capture callbacks + output callback.
//! - one thread per simulated source (when `--simulate N`).

use crate::audio::engine::{new_engine, EngineHandle};
use crate::audio::input::{connect_capture, CaptureStream};
use crate::audio::output::connect_playback;
use crate::audio::sim::SimSource;
use crate::config::Config;
use crate::devices::discovery::{classify, NodeKind};
use crate::devices::registry::{DeviceRegistry, NodeInfo};
use crate::monitoring::ProcessSampler;
use anyhow::Context as _;
use pipewire as pw;
use pw::registry::GlobalObject;
use pw::spa::utils::dict::DictRef;
use pw::types::ObjectType;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Number of simulated sine-wave inputs to add.
    pub simulate: usize,
    /// Do not capture Bluetooth sources (useful with `--simulate`).
    pub no_bluetooth: bool,
    pub status_interval: Duration,
}

struct State {
    cfg: Config,
    core: pw::core::CoreRc,
    engine: EngineHandle,
    devices: DeviceRegistry,
    registry: pw::registry::RegistryRc,
    /// PipeWire node id → capture stream.
    captures: HashMap<u32, CaptureStream>,
    /// Bluetooth sources whose full properties we are fetching before capturing.
    probes: HashMap<u32, (pw::node::Node, pw::node::NodeListener)>,
    /// Bluetooth sources we decided not to capture (e.g. headset microphone proxies).
    ignored: HashSet<u32>,
    sims: Vec<SimSource>,
    sampler: ProcessSampler,
    last_cycles: u64,
}

pub fn run(cfg: Config, opts: RunOptions) -> anyhow::Result<()> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None).context("create PipeWire main loop")?;
    let context = pw::context::ContextRc::new(&mainloop, None).context("create PipeWire context")?;
    let core = context
        .connect_rc(None)
        .context("connect to PipeWire (is the user's pipewire service running?)")?;
    let registry = core.get_registry_rc().context("get PipeWire registry")?;

    if cfg.bluetooth.enabled {
        setup_bluetooth_identity(&cfg);
        if let Err(e) = crate::bluetooth::agent::spawn_agent(
            &cfg.bluetooth.adapter,
            cfg.bluetooth.auto_accept_trusted_devices,
        ) {
            warn!(error = %e, "cannot start pairing agent thread");
        }
    }

    let (handle, engine) = new_engine(&cfg.audio);
    let _playback = connect_playback(&core, engine, &cfg).context("create output stream")?;
    info!(
        output = %cfg.output.device,
        rate = cfg.audio.sample_rate,
        quantum = cfg.audio.buffer_size,
        latency_ms = cfg.audio.input_latency_ms,
        "mixer output started"
    );

    let state = Rc::new(RefCell::new(State {
        cfg: cfg.clone(),
        core: core.clone(),
        engine: handle,
        devices: DeviceRegistry::default(),
        registry: registry.clone(),
        captures: HashMap::new(),
        probes: HashMap::new(),
        ignored: HashSet::new(),
        sims: Vec::new(),
        sampler: ProcessSampler::default(),
        last_cycles: 0,
    }));

    // Simulated inputs: tones on a pentatonic scale so the mix is audible.
    {
        let mut st = state.borrow_mut();
        const FREQS: [f32; 10] = [220.0, 261.6, 293.7, 329.6, 392.0, 440.0, 523.3, 587.3, 659.3, 784.0];
        for i in 0..opts.simulate {
            let freq = FREQS[i % FREQS.len()];
            let (slot, producer) = st
                .engine
                .add_input(&format!("sim-{i} ({freq} Hz)"))
                .context("add simulated input")?;
            let sim = SimSource::spawn(producer, freq, 0.2, cfg.audio.sample_rate, cfg.audio.channels as usize);
            st.sims.push(sim);
            info!(slot, freq, "simulated input added");
        }
    }

    let capture_bt = cfg.bluetooth.enabled && cfg.bluetooth.auto_connect_sources && !opts.no_bluetooth;
    let _registry_listener = registry
        .add_listener_local()
        .global({
            let state = state.clone();
            move |g| on_global(&state, g, capture_bt)
        })
        .global_remove({
            let state = state.clone();
            move |id| on_global_remove(&state, id)
        })
        .register();

    let weak = mainloop.downgrade();
    let _sigint = mainloop.loop_().add_signal_local(pw::loop_::Signal::INT, move || {
        if let Some(l) = weak.upgrade() {
            info!("SIGINT received, shutting down");
            l.quit();
        }
    });
    let weak = mainloop.downgrade();
    let _sigterm = mainloop.loop_().add_signal_local(pw::loop_::Signal::TERM, move || {
        if let Some(l) = weak.upgrade() {
            info!("SIGTERM received, shutting down");
            l.quit();
        }
    });

    let timer = mainloop.loop_().add_timer({
        let state = state.clone();
        move |_| report(&mut state.borrow_mut())
    });
    let _ = timer.update_timer(Some(Duration::from_secs(2)), Some(opts.status_interval));

    mainloop.run();

    // Graceful shutdown: stop inputs first, then the output.
    let mut st = state.borrow_mut();
    st.probes.clear();
    st.captures.clear();
    st.sims.clear();
    info!("stopped");
    Ok(())
}

fn setup_bluetooth_identity(cfg: &Config) {
    use crate::bluetooth::{apply_identity, is_audio_class, major_class_name};
    let bt = &cfg.bluetooth;
    match apply_identity(&bt.adapter, &bt.name) {
        Ok(a) => {
            info!(
                adapter = %bt.adapter,
                address = %a.address,
                name = %a.alias,
                renamed = a.renamed,
                class = format!("0x{:06x} ({})", a.class, major_class_name(a.class)),
                powered = a.powered,
                "Bluetooth identity"
            );
            if !is_audio_class(a.class) {
                warn!(
                    "Bluetooth class is {} — Macs and phones will not list \"{}\" as a speaker. \
                     Run services/audio-server/scripts/setup-bluetooth.sh once (sudo).",
                    major_class_name(a.class),
                    a.alias
                );
            }
            if !a.powered {
                warn!("Bluetooth adapter {} is powered off", bt.adapter);
            }
        }
        Err(e) => warn!(adapter = %bt.adapter, error = %e, "cannot configure Bluetooth adapter"),
    }
}

fn on_global(state: &Rc<RefCell<State>>, g: &GlobalObject<&DictRef>, capture_bt: bool) {
    if g.type_ != ObjectType::Node {
        return;
    }
    let Some(props) = g.props else { return };
    let kind = classify(|k| props.get(k));
    let name = props.get("node.name").unwrap_or("").to_owned();
    let description = props
        .get("node.description")
        .or_else(|| props.get("node.nick"))
        .unwrap_or(&name)
        .to_owned();
    let media_class = props.get("media.class").unwrap_or("").to_owned();

    let mut st = state.borrow_mut();
    st.devices.insert(NodeInfo {
        id: g.id,
        name: name.clone(),
        description: description.clone(),
        media_class,
        kind,
    });

    match kind {
        NodeKind::BluetoothSource if capture_bt => {
            // Registry globals carry only a few properties. Fetch the full node info
            // before deciding: WirePlumber also creates `bluez5.loopback` sources that
            // stand for a headset's microphone, and capturing one would switch the
            // headset to the low-quality HFP profile.
            let registry = st.registry.clone();
            drop(st);
            let node: pw::node::Node = match registry.bind(g) {
                Ok(n) => n,
                Err(e) => {
                    error!(id = g.id, error = %e, "cannot inspect Bluetooth source");
                    return;
                }
            };
            let id = g.id;
            let weak: Weak<RefCell<State>> = Rc::downgrade(state);
            let listener = node
                .add_listener_local()
                .info(move |info| {
                    let Some(state) = weak.upgrade() else { return };
                    let Ok(mut st) = state.try_borrow_mut() else { return };
                    if st.captures.contains_key(&id) || st.ignored.contains(&id) {
                        return; // already decided (info is re-sent on every change)
                    }
                    let loopback = info
                        .props()
                        .and_then(|p| p.get("bluez5.loopback"))
                        .is_some_and(|v| v == "true");
                    if loopback {
                        info!(id, device = %description, "ignoring headset microphone proxy (bluez5.loopback)");
                        st.ignored.insert(id);
                    } else {
                        start_capture(&mut st, id, &name, &description);
                    }
                })
                .register();
            state.borrow_mut().probes.insert(id, (node, listener));
        }
        NodeKind::BluetoothSource => {
            info!(id = g.id, device = %description, "Bluetooth source available (auto-capture disabled)");
        }
        NodeKind::BluetoothPlayback => {
            warn!(
                id = g.id,
                device = %description,
                "Bluetooth audio is playing directly to the speakers, not through the mixer. \
                 Install packaging/wireplumber/51-charlotte-bluez.conf and reconnect the device."
            );
        }
        NodeKind::OutputSink | NodeKind::VirtualSink => {
            info!(id = g.id, output = %name, kind = kind.label(), "output available");
        }
        _ => {}
    }
}

fn start_capture(st: &mut State, id: u32, name: &str, description: &str) {
    let (slot, producer) = match st.engine.add_input(description) {
        Ok(v) => v,
        Err(e) => {
            error!(device = %description, error = %e, "cannot add Bluetooth input");
            st.ignored.insert(id);
            return;
        }
    };
    match connect_capture(&st.core, name, slot, producer, &st.cfg.audio) {
        Ok(cap) => {
            info!(id, slot, device = %description, node = %name, "Bluetooth source connected → mixer input");
            st.captures.insert(id, cap);
        }
        Err(e) => {
            error!(device = %description, error = %e, "cannot capture Bluetooth source");
            let _ = st.engine.remove_input(slot);
            st.ignored.insert(id);
        }
    }
}

fn on_global_remove(state: &Rc<RefCell<State>>, id: u32) {
    let mut st = state.borrow_mut();
    st.probes.remove(&id);
    st.ignored.remove(&id);
    if let Some(cap) = st.captures.remove(&id) {
        let slot = cap.slot;
        drop(cap);
        if let Err(e) = st.engine.remove_input(slot) {
            warn!(slot, error = %e, "remove input");
        }
        info!(id, slot, node = %st.devices.get(id).map_or("?", |n| n.name.as_str()), "Bluetooth source disconnected → input removed");
    }
    st.devices.remove(id);
}

fn report(st: &mut State) {
    st.engine.collect_garbage();
    // Drop node probes once a decision was made (never from inside their own callback).
    let decided: Vec<u32> = st
        .probes
        .keys()
        .filter(|id| st.captures.contains_key(id) || st.ignored.contains(id))
        .copied()
        .collect();
    for id in decided {
        st.probes.remove(&id);
    }
    let m = st.engine.master().clone();
    let cycles = m.cycles.load(Relaxed);
    if cycles == st.last_cycles {
        warn!("output is not running (no audio cycles) — is the output device available and not suspended?");
    }
    st.last_cycles = cycles;

    let proc_stats = st.sampler.sample();
    info!(
        inputs = st.engine.channels().count(),
        master_peak_db = %db(m.peak.get()),
        limiter_gain = format!("{:.2}", m.limiter_gain.get()),
        xruns = m.xruns.load(Relaxed),
        cpu = proc_stats.map(|p| format!("{:.1}%", p.cpu_percent)).unwrap_or_default(),
        rss_mib = proc_stats.map(|p| format!("{:.1}", p.rss_kib as f64 / 1024.0)).unwrap_or_default(),
        "status"
    );
    let samples_per_ms = (st.cfg.audio.sample_rate as f32 * st.cfg.audio.channels as f32) / 1000.0;
    for (slot, c) in st.engine.channels() {
        info!(
            slot,
            name = %c.name,
            streaming = c.streaming.load(Relaxed),
            peak_db = %db(c.peak.get()),
            gain = c.gain.get(),
            muted = c.muted.load(Relaxed),
            buffered_ms = format!("{:.0}", c.buffered.load(Relaxed) as f32 / samples_per_ms),
            underruns = c.underruns.load(Relaxed),
            overruns = c.overruns.load(Relaxed),
            drift_skips = c.drift_skips.load(Relaxed),
            "  input"
        );
    }
}

fn db(linear: f32) -> String {
    if linear <= 1e-6 {
        "-inf".into()
    } else {
        format!("{:.1}", 20.0 * linear.log10())
    }
}

/// Prints every audio node PipeWire knows, with our classification.
pub fn list_nodes() -> anyhow::Result<()> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None).context("connect to PipeWire")?;
    let registry = core.get_registry_rc()?;

    let nodes: Rc<RefCell<Vec<NodeInfo>>> = Rc::default();
    let _l = registry
        .add_listener_local()
        .global({
            let nodes = nodes.clone();
            move |g| {
                if g.type_ != ObjectType::Node {
                    return;
                }
                let Some(p) = g.props else { return };
                let media_class = p.get("media.class").unwrap_or("").to_owned();
                if !media_class.contains("Audio") {
                    return;
                }
                let name = p.get("node.name").unwrap_or("").to_owned();
                nodes.borrow_mut().push(NodeInfo {
                    id: g.id,
                    description: p.get("node.description").unwrap_or(&name).to_owned(),
                    kind: classify(|k| p.get(k)),
                    name,
                    media_class,
                });
            }
        })
        .register();

    let weak = mainloop.downgrade();
    let timer = mainloop.loop_().add_timer(move |_| {
        if let Some(l) = weak.upgrade() {
            l.quit();
        }
    });
    let _ = timer.update_timer(Some(Duration::from_millis(500)), None);
    mainloop.run();

    println!("{:<5} {:<20} {:<22} {:<55} DESCRIPTION", "ID", "KIND", "MEDIA CLASS", "NODE NAME");
    for n in nodes.borrow().iter() {
        println!(
            "{:<5} {:<20} {:<22} {:<55} {}",
            n.id,
            n.kind.label(),
            n.media_class,
            n.name,
            n.description
        );
    }
    Ok(())
}
