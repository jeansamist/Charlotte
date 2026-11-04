//! PipeWire format negotiation: we always ask for interleaved f32 at the
//! internal rate/channel count. PipeWire's adapter converts (resamples,
//! remaps channels) every input and the output to match.

use pipewire as pw;
use pw::spa;

/// Serialized `EnumFormat` pod for F32LE / `rate` / `channels` (FL, FR, …).
pub fn raw_f32_format(rate: u32, channels: u32) -> Vec<u8> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(rate);
    info.set_channels(channels);
    let mut position = [0u32; spa::param::audio::MAX_CHANNELS];
    let layout = [
        spa::sys::SPA_AUDIO_CHANNEL_FL,
        spa::sys::SPA_AUDIO_CHANNEL_FR,
        spa::sys::SPA_AUDIO_CHANNEL_FC,
        spa::sys::SPA_AUDIO_CHANNEL_LFE,
        spa::sys::SPA_AUDIO_CHANNEL_RL,
        spa::sys::SPA_AUDIO_CHANNEL_RR,
    ];
    for (i, p) in layout.iter().take(channels as usize).enumerate() {
        position[i] = *p;
    }
    info.set_position(position);

    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(spa::pod::Object {
            type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
            id: spa::param::ParamType::EnumFormat.as_raw(),
            properties: info.into(),
        }),
    )
    .expect("serialize audio format pod")
    .0
    .into_inner()
}
