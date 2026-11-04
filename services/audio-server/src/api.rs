//! HTTP API (axum) for the Charlotte gateway. Runs on its own thread with its
//! own Tokio runtime, fully isolated from the real-time audio thread.
//!
//! **No authentication**: bind it to 127.0.0.1 only. The gateway (which has
//! login) is the only intended client.

use crate::bluetooth;
use crate::shared::Shared;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing::{error, info};

type AppState = Arc<Shared>;

pub struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

fn bad_request(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}
fn not_found(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::NOT_FOUND, msg.into())
}
fn internal(msg: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, msg.to_string())
}

type ApiResult = Result<Json<Value>, ApiError>;

pub fn spawn(shared: Arc<Shared>, host: String, port: u16) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new().name("api".into()).spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => return error!(error = %e, "cannot start API runtime"),
        };
        rt.block_on(async move {
            let addr = format!("{host}:{port}");
            let listener = match tokio::net::TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => return error!(%addr, error = %e, "cannot bind HTTP API"),
            };
            info!(%addr, "HTTP API listening");
            if let Err(e) = axum::serve(listener, router(shared)).await {
                error!(error = %e, "HTTP API stopped");
            }
        });
    })
}

pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/metrics", get(metrics))
        .route("/api/mixer", get(mixer))
        .route("/api/mixer/channels", get(mixer))
        .route("/api/mixer/channels/{slot}/volume", put(channel_volume))
        .route("/api/mixer/channels/{slot}/mute", put(channel_mute))
        .route("/api/mixer/master/volume", put(master_volume))
        .route("/api/mixer/master/mute", put(master_mute))
        .route("/api/mixer/ducking", put(set_ducking))
        .route("/api/outputs", get(outputs))
        .route("/api/outputs/current", get(outputs).put(set_output))
        .route("/api/bluetooth", get(bt_status))
        .route("/api/bluetooth/name", put(bt_name))
        .route("/api/bluetooth/pairing", post(bt_pairing_open).delete(bt_pairing_close))
        .route("/api/bluetooth/devices/{address}", delete(bt_forget))
        .with_state(shared)
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn health(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": s.version,
        "started_at": s.started_at,
        "uptime_s": now().saturating_sub(s.started_at),
        "cycles": s.master.cycles.load(Relaxed),
        "xruns": s.master.xruns.load(Relaxed),
        "inputs": s.inputs.lock().unwrap().len(),
    }))
}

async fn metrics(State(s): State<AppState>) -> Json<Value> {
    let p = s.sampler.lock().unwrap().sample();
    Json(json!({
        "cpu_percent": p.map(|p| p.cpu_percent),
        "rss_kib": p.map(|p| p.rss_kib),
        "cycles": s.master.cycles.load(Relaxed),
        "xruns": s.master.xruns.load(Relaxed),
    }))
}

fn mixer_json(s: &Shared) -> Value {
    let m = &s.master;
    let channels: Vec<Value> = s
        .inputs
        .lock()
        .unwrap()
        .iter()
        .map(|i| {
            let c = &i.control;
            json!({
                "slot": i.slot,
                "name": i.device,
                "node": i.node_name,
                "kind": i.kind,
                "volume": c.gain.get(),
                "muted": c.muted.load(Relaxed),
                "peak": c.peak.get(),
                "streaming": c.streaming.load(Relaxed),
                "buffered_ms": c.buffered.load(Relaxed) as f32 / s.samples_per_ms,
                "underruns": c.underruns.load(Relaxed),
                "overruns": c.overruns.load(Relaxed),
                "drift_skips": c.drift_skips.load(Relaxed),
                "slips": c.slips.load(Relaxed),
                "active": c.active.load(Relaxed),
                "ducked": c.ducked.load(Relaxed),
                "duck_gain": c.duck_gain.get(),
            })
        })
        .collect();
    json!({
        "master": {
            "volume": m.gain.get(),
            "muted": m.muted.load(Relaxed),
            "peak": m.peak.get(),
            "limiter_gain": m.limiter_gain.get(),
            "limiter_enabled": m.limiter_enabled.load(Relaxed),
        },
        "ducking": {
            "enabled": m.ducking_enabled.load(Relaxed),
            "level_db": m.ducking_level_db.get(),
        },
        "channels": channels,
    })
}

async fn mixer(State(s): State<AppState>) -> Json<Value> {
    Json(mixer_json(&s))
}

#[derive(Deserialize)]
struct VolumeBody {
    volume: f32,
}
#[derive(Deserialize)]
struct MuteBody {
    muted: bool,
}

fn check_volume(v: f32, max: f32) -> Result<f32, ApiError> {
    if v.is_finite() && (0.0..=max).contains(&v) {
        Ok(v)
    } else {
        Err(bad_request(format!("volume must be between 0 and {max}")))
    }
}

fn with_channel(s: &Shared, slot: usize, f: impl FnOnce(&crate::shared::InputEntry)) -> Result<(), ApiError> {
    let inputs = s.inputs.lock().unwrap();
    let e = inputs
        .iter()
        .find(|i| i.slot == slot)
        .ok_or_else(|| not_found(format!("no mixer channel {slot}")))?;
    f(e);
    Ok(())
}

async fn channel_volume(State(s): State<AppState>, Path(slot): Path<usize>, Json(b): Json<VolumeBody>) -> ApiResult {
    let v = check_volume(b.volume, 2.0)?;
    with_channel(&s, slot, |e| e.control.gain.set(v))?;
    Ok(Json(mixer_json(&s)))
}

async fn channel_mute(State(s): State<AppState>, Path(slot): Path<usize>, Json(b): Json<MuteBody>) -> ApiResult {
    with_channel(&s, slot, |e| e.control.muted.store(b.muted, Relaxed))?;
    Ok(Json(mixer_json(&s)))
}

async fn master_volume(State(s): State<AppState>, Json(b): Json<VolumeBody>) -> ApiResult {
    s.master.gain.set(check_volume(b.volume, 1.0)?);
    Ok(Json(mixer_json(&s)))
}

async fn master_mute(State(s): State<AppState>, Json(b): Json<MuteBody>) -> ApiResult {
    s.master.muted.store(b.muted, Relaxed);
    Ok(Json(mixer_json(&s)))
}

#[derive(Deserialize)]
struct DuckingBody {
    enabled: Option<bool>,
    level_db: Option<f32>,
}

async fn set_ducking(State(s): State<AppState>, Json(b): Json<DuckingBody>) -> ApiResult {
    if let Some(l) = b.level_db {
        if !(l.is_finite() && (-60.0..=0.0).contains(&l)) {
            return Err(bad_request("level_db must be between -60 and 0"));
        }
        s.master.ducking_level_db.set(l);
    }
    if let Some(e) = b.enabled {
        s.master.ducking_enabled.store(e, Relaxed);
    }
    Ok(Json(mixer_json(&s)))
}

/// The mixer plays to PipeWire's default sink; switching output = changing it.
async fn outputs(State(s): State<AppState>) -> ApiResult {
    let current = tokio::task::spawn_blocking(crate::outputs::default_sink_name)
        .await
        .map_err(internal)?;
    let list = s.outputs.lock().unwrap().clone();
    Ok(Json(json!({ "current": current, "outputs": list })))
}

#[derive(Deserialize)]
struct OutputBody {
    name: String,
}

async fn set_output(State(s): State<AppState>, Json(b): Json<OutputBody>) -> ApiResult {
    let id = s
        .outputs
        .lock()
        .unwrap()
        .iter()
        .find(|o| o.name == b.name)
        .map(|o| o.id)
        .ok_or_else(|| not_found(format!("unknown output {:?}", b.name)))?;
    tokio::task::spawn_blocking(move || crate::outputs::set_default_sink(id))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    info!(output = %b.name, "output changed");
    outputs(State(s)).await
}

fn adapter(s: &Shared) -> Result<String, ApiError> {
    s.bluetooth_adapter
        .clone()
        .ok_or_else(|| ApiError(StatusCode::CONFLICT, "Bluetooth is disabled in the configuration".into()))
}

async fn bt_status(State(s): State<AppState>) -> ApiResult {
    let a = adapter(&s)?;
    let st = tokio::task::spawn_blocking(move || bluetooth::status(&a))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    Ok(Json(serde_json::to_value(st).map_err(internal)?))
}

#[derive(Deserialize)]
struct NameBody {
    name: String,
}

async fn bt_name(State(s): State<AppState>, Json(b): Json<NameBody>) -> ApiResult {
    let name = b.name.trim().to_owned();
    if name.is_empty() || name.len() > 248 {
        return Err(bad_request("name must be 1 to 248 bytes long"));
    }
    let a = adapter(&s)?;
    let n = name.clone();
    tokio::task::spawn_blocking(move || bluetooth::set_name(&a, &n))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    info!(%name, "Bluetooth name changed");
    bt_status(State(s)).await
}

#[derive(Deserialize)]
struct PairingBody {
    #[serde(default = "default_pairing_secs")]
    seconds: u32,
}
fn default_pairing_secs() -> u32 {
    180
}

async fn bt_pairing_open(State(s): State<AppState>, Json(b): Json<PairingBody>) -> ApiResult {
    if !(30..=900).contains(&b.seconds) {
        return Err(bad_request("seconds must be between 30 and 900"));
    }
    let a = adapter(&s)?;
    let secs = b.seconds;
    tokio::task::spawn_blocking(move || bluetooth::set_pairing_window(&a, Some(secs)))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    info!(seconds = secs, "pairing window opened");
    bt_status(State(s)).await
}

async fn bt_pairing_close(State(s): State<AppState>) -> ApiResult {
    let a = adapter(&s)?;
    tokio::task::spawn_blocking(move || bluetooth::set_pairing_window(&a, None))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    bt_status(State(s)).await
}

async fn bt_forget(State(s): State<AppState>, Path(address): Path<String>) -> ApiResult {
    let a = adapter(&s)?;
    let addr = address.clone();
    tokio::task::spawn_blocking(move || bluetooth::forget_device(&a, &addr))
        .await
        .map_err(internal)?
        .map_err(|e| match e {
            bluetooth::BluetoothError::InvalidAddress(_) => bad_request(e.to_string()),
            e => internal(e),
        })?;
    info!(%address, "Bluetooth device forgotten");
    bt_status(State(s)).await
}
