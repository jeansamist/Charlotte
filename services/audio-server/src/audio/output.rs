//! Playback stream: the single mixed output. Its real-time callback asks the
//! [`Engine`] for the next block and writes it to the PipeWire buffer.

use super::engine::{Engine, MAX_BLOCK_FRAMES};
use super::format::raw_f32_format;
use crate::config::Config;
use crate::devices::discovery::{APP_NAME, OUTPUT_NODE_NAME};
use pipewire as pw;
use pw::properties::properties;
use pw::spa::pod::Pod;
use tracing::{info, warn};

struct PlaybackData {
    engine: Engine,
    buf: Vec<f32>,
    channels: usize,
}

pub struct PlaybackStream {
    _listener: pw::stream::StreamListener<PlaybackData>,
    stream: pw::stream::StreamRc,
}

impl Drop for PlaybackStream {
    fn drop(&mut self) {
        let _ = self.stream.disconnect();
    }
}

pub fn connect_playback(
    core: &pw::core::CoreRc,
    engine: Engine,
    cfg: &Config,
) -> Result<PlaybackStream, pw::Error> {
    let audio = &cfg.audio;
    let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "Music",
        *pw::keys::APP_NAME => APP_NAME,
        *pw::keys::NODE_NAME => OUTPUT_NODE_NAME,
        *pw::keys::NODE_DESCRIPTION => "Charlotte mixer output",
    };
    props.insert(
        *pw::keys::NODE_LATENCY,
        format!("{}/{}", audio.buffer_size, audio.sample_rate),
    );
    if cfg.output.device != "default" {
        props.insert(*pw::keys::TARGET_OBJECT, cfg.output.device.as_str());
    }

    let stream = pw::stream::StreamRc::new(core.clone(), "mixer-out", props)?;
    let channels = audio.channels as usize;
    let data = PlaybackData {
        engine,
        buf: vec![0.0; MAX_BLOCK_FRAMES * channels],
        channels,
    };

    let listener = stream
        .add_local_listener_with_user_data(data)
        .state_changed(|_, _, old, new| {
            info!(?old, ?new, "output stream state");
            if let pw::stream::StreamState::Error(e) = new {
                warn!(error = %e, "output stream error");
            }
        })
        .process(|stream, data| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                data.engine.note_xrun();
                return;
            };
            let requested = buffer.requested() as usize;
            let datas = buffer.datas_mut();
            let Some(d) = datas.first_mut() else {
                return;
            };
            let ch = data.channels;
            let stride = 4 * ch;
            let frames = match d.data() {
                Some(bytes) => {
                    let cap = bytes.len() / stride;
                    let frames = if requested > 0 { requested.min(cap) } else { cap }
                        .min(data.buf.len() / ch);
                    let out = &mut data.buf[..frames * ch];
                    data.engine.process(out);
                    for (dst, s) in bytes.chunks_exact_mut(4).zip(out.iter()) {
                        dst.copy_from_slice(&s.to_le_bytes());
                    }
                    frames
                }
                None => 0,
            };
            let chunk = d.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = stride as i32;
            *chunk.size_mut() = (frames * stride) as u32;
        })
        .register()?;

    let format = raw_f32_format(audio.sample_rate, audio.channels);
    let mut params = [Pod::from_bytes(&format).expect("valid format pod")];
    stream.connect(
        pw::spa::utils::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    Ok(PlaybackStream {
        _listener: listener,
        stream,
    })
}
