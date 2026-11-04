//! Process CPU / memory sampling (from /proc) for status reports and benchmarks.

use std::time::Instant;

pub struct ProcessSampler {
    last_ticks: Option<u64>,
    last_at: Instant,
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessStats {
    /// CPU usage since the previous sample, in % of one core.
    pub cpu_percent: f32,
    pub rss_kib: u64,
}

/// Linux reports CPU time in clock ticks; 100/s on all mainstream x86_64 kernels.
const CLK_TCK: f64 = 100.0;

impl Default for ProcessSampler {
    fn default() -> Self {
        Self {
            last_ticks: None,
            last_at: Instant::now(),
        }
    }
}

impl ProcessSampler {
    pub fn sample(&mut self) -> Option<ProcessStats> {
        let ticks = cpu_ticks()?;
        let now = Instant::now();
        let cpu = match self.last_ticks {
            Some(prev) => {
                let secs = now.duration_since(self.last_at).as_secs_f64().max(1e-3);
                ((ticks.saturating_sub(prev)) as f64 / CLK_TCK / secs * 100.0) as f32
            }
            None => 0.0,
        };
        self.last_ticks = Some(ticks);
        self.last_at = now;
        Some(ProcessStats {
            cpu_percent: cpu,
            rss_kib: rss_kib().unwrap_or(0),
        })
    }
}

fn cpu_ticks() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // Fields after the ")" of the command name; utime and stime are fields 14 and 15.
    let rest = &stat[stat.rfind(')')? + 2..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    Some(f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?)
}

fn rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
