//! Capture stream: one PipeWire input stream per mixer input (e.g. one per
//! connected phone). Its real-time callback copies samples into the input's
//! ring buffer and does nothing else.

use super::buffer::InputProducer;
use super::format::raw_f32_format;
use crate::config::AudioConfig;
use crate::devices::discovery::APP_NAME;
use pipewire as pw;
use pw::properties::properties;
use pw::spa::pod::Pod;
use tracing::{debug, warn};

struct CaptureData {
    producer: InputProducer,
}

/// A live capture stream. Dropping it disconnects from PipeWire.
pub struct CaptureStream {
    // Field order matters: the listener must be dropped before the stream.
    _listener: pw::stream::StreamListener<CaptureData>,
    stream: pw::stream::StreamRc,
    pub slot: usize,
    pub target: String,
}

impl Drop for CaptureStream {
    fn drop(&mut self) {
        let _ = self.stream.disconnect();
    }
}

/// Connects a capture stream to the PipeWire node named `target` and feeds `producer`.
pub fn connect_capture(
    core: &pw::core::CoreRc,
    target: &str,
    slot: usize,
    producer: InputProducer,
    audio: &AudioConfig,
) -> Result<CaptureStream, pw::Error> {
    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::APP_NAME => APP_NAME,
        // Never follow the default source if `target` disappears.
        *pw::keys::NODE_DONT_RECONNECT => "true",
    };
    props.insert(*pw::keys::NODE_NAME, format!("charlotte-mixer-in-{slot}"));
    props.insert(
        *pw::keys::NODE_DESCRIPTION,
        format!("Charlotte mixer input {slot}"),
    );
    props.insert(
        *pw::keys::NODE_LATENCY,
        format!("{}/{}", audio.buffer_size, audio.sample_rate),
    );
    props.insert(*pw::keys::TARGET_OBJECT, target);

    let stream = pw::stream::StreamRc::new(core.clone(), &format!("mixer-in-{slot}"), props)?;
    let name = target.to_owned();

    let listener = stream
        .add_local_listener_with_user_data(CaptureData { producer })
        .state_changed(move |_, _, old, new| {
            debug!(input = %name, ?old, ?new, "capture stream state");
            if let pw::stream::StreamState::Error(e) = new {
                warn!(input = %name, error = %e, "capture stream error");
            }
        })
        .process(|stream, data| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(d) = datas.first_mut() else {
                return;
            };
            let offset = d.chunk().offset() as usize;
            let size = d.chunk().size() as usize;
            if let Some(bytes) = d.data() {
                let end = (offset + size).min(bytes.len());
                let bytes = &bytes[offset.min(end)..end];
                let n = bytes.len() / 4;
                data.producer.push_iter(
                    n,
                    bytes
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
                );
            }
        })
        .register()?;

    let format = raw_f32_format(audio.sample_rate, audio.channels);
    let mut params = [Pod::from_bytes(&format).expect("valid format pod")];
    stream.connect(
        pw::spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    Ok(CaptureStream {
        _listener: listener,
        stream,
        slot,
        target: target.to_owned(),
    })
}
