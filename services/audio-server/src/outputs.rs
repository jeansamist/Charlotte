//! Output selection. The mixer's playback stream follows PipeWire's default
//! sink, so choosing an output means changing the default sink (via `wpctl`,
//! part of WirePlumber).

use std::process::Command;

/// `node.name` of the current default sink, if any.
pub fn default_sink_name() -> Option<String> {
    let out = Command::new("wpctl")
        .args(["inspect", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()?;
    parse_node_name(&String::from_utf8_lossy(&out.stdout))
}

/// Makes node `id` the default sink.
pub fn set_default_sink(id: u32) -> std::io::Result<()> {
    let status = Command::new("wpctl").args(["set-default", &id.to_string()]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("wpctl set-default {id} failed: {status}")))
    }
}

fn parse_node_name(inspect: &str) -> Option<String> {
    inspect.lines().find_map(|l| {
        let l = l.trim_start_matches(|c: char| c.is_whitespace() || c == '*').trim();
        l.strip_prefix("node.name = ")
            .map(|v| v.trim_matches('"').to_owned())
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_wpctl_inspect() {
        let s = "id 99, type PipeWire:Interface:Node\n    media.class = \"Audio/Sink\"\n  * node.name = \"bluez_output.41_42.1\"\n";
        assert_eq!(super::parse_node_name(s).as_deref(), Some("bluez_output.41_42.1"));
    }
}
