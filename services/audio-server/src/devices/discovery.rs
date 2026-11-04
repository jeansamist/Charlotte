//! Classifies PipeWire nodes so we only capture what we should.
//!
//! The important rule: never capture our own nodes, or the mixed output would
//! feed back into the mixer.

pub const APP_NAME: &str = "charlotte-audio-server";
pub const OUTPUT_NODE_NAME: &str = "charlotte-mixer-out";
const OWN_NODE_PREFIX: &str = "charlotte-mixer-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// A Bluetooth device streaming to us, exposed as a source → mixer input.
    BluetoothSource,
    /// A Bluetooth device streaming to us, but routed straight to the speakers
    /// (`bluez5.media-source-role = playback`, the PipeWire default). Not captured.
    BluetoothPlayback,
    /// Physical output (ALSA or Bluetooth speaker).
    OutputSink,
    /// Virtual output (loopback, null sink, filter…).
    VirtualSink,
    /// Any other capture source (microphones…). Not captured automatically.
    OtherSource,
    /// Created by this program.
    Own,
    Other,
}

impl NodeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::BluetoothSource => "bluetooth-source",
            Self::BluetoothPlayback => "bluetooth-playback",
            Self::OutputSink => "output",
            Self::VirtualSink => "virtual-output",
            Self::OtherSource => "source",
            Self::Own => "own",
            Self::Other => "other",
        }
    }
}

/// `get` returns a node property (e.g. `get("media.class")`).
pub fn classify<'a>(get: impl Fn(&str) -> Option<&'a str>) -> NodeKind {
    let class = get("media.class").unwrap_or("");
    let name = get("node.name").unwrap_or("");

    if get("application.name") == Some(APP_NAME) || name.starts_with(OWN_NODE_PREFIX) {
        return NodeKind::Own;
    }
    let api = get("device.api");
    let is_bt = api == Some("bluez5") || name.starts_with("bluez_input.");

    match class {
        "Audio/Source" if is_bt => NodeKind::BluetoothSource,
        "Stream/Output/Audio" if is_bt => NodeKind::BluetoothPlayback,
        "Audio/Sink" => {
            let physical = matches!(api, Some("alsa") | Some("bluez5"));
            if physical && !name.contains("snd_aloop") {
                NodeKind::OutputSink
            } else {
                NodeKind::VirtualSink
            }
        }
        c if c.starts_with("Audio/Source") => NodeKind::OtherSource,
        _ => NodeKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn kind(pairs: &[(&'static str, &'static str)]) -> NodeKind {
        let m: HashMap<&str, &'static str> = pairs.iter().copied().collect();
        classify(|k| m.get(k).copied())
    }

    #[test]
    fn bluetooth_input_is_captured() {
        assert_eq!(
            kind(&[
                ("media.class", "Audio/Source"),
                ("node.name", "bluez_input.AA_BB_CC_DD_EE_FF.2"),
                ("device.api", "bluez5"),
            ]),
            NodeKind::BluetoothSource
        );
    }

    #[test]
    fn bluetooth_in_playback_role_is_flagged() {
        assert_eq!(
            kind(&[
                ("media.class", "Stream/Output/Audio"),
                ("node.name", "bluez_input.AA_BB_CC_DD_EE_FF.2"),
            ]),
            NodeKind::BluetoothPlayback
        );
    }

    #[test]
    fn own_nodes_are_never_inputs() {
        assert_eq!(
            kind(&[
                ("media.class", "Audio/Source"),
                ("node.name", "charlotte-mixer-out"),
                ("device.api", "bluez5"),
            ]),
            NodeKind::Own
        );
        assert_eq!(
            kind(&[("application.name", APP_NAME), ("media.class", "Stream/Input/Audio")]),
            NodeKind::Own
        );
    }

    #[test]
    fn sinks() {
        assert_eq!(
            kind(&[
                ("media.class", "Audio/Sink"),
                ("node.name", "alsa_output.pci-0000_00_1f.3.analog-stereo"),
                ("device.api", "alsa"),
            ]),
            NodeKind::OutputSink
        );
        assert_eq!(
            kind(&[
                ("media.class", "Audio/Sink"),
                ("node.name", "alsa_output.platform-snd_aloop.0.analog-stereo"),
                ("device.api", "alsa"),
            ]),
            NodeKind::VirtualSink
        );
    }

    #[test]
    fn microphones_are_not_bluetooth_sources() {
        assert_eq!(
            kind(&[
                ("media.class", "Audio/Source"),
                ("node.name", "alsa_input.pci.mic"),
                ("device.api", "alsa"),
            ]),
            NodeKind::OtherSource
        );
    }
}
