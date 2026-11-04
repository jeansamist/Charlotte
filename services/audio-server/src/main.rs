use audio_server::{app, config};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "audio-server", version, about = "Charlotte multi-source Bluetooth audio mixer")]
struct Cli {
    /// Config file (default: /etc/charlotte/audio-server.toml, then ./config/default.toml).
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Overrides logging.level (trace, debug, info, warn, error). RUST_LOG also works.
    #[arg(long, global = true)]
    log_level: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the mixer (default).
    Run {
        /// Add N simulated sine-wave inputs (test without Bluetooth devices).
        #[arg(long, default_value_t = 0)]
        simulate: usize,
        /// Do not capture Bluetooth sources.
        #[arg(long)]
        no_bluetooth: bool,
        /// Override output.device (a PipeWire node.name, or "default").
        #[arg(long)]
        output: Option<String>,
        /// Seconds between status reports.
        #[arg(long, default_value_t = 5)]
        status_interval: u64,
    },
    /// List PipeWire audio nodes and how the mixer classifies them.
    ListNodes,
    /// Validate the configuration and print it.
    CheckConfig,
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (mut cfg, file) = config::load(cli.config.as_deref())?;
    if let Some(level) = cli.log_level {
        cfg.logging.level = level;
    }
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("audio_server={}", cfg.logging.level)));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
    tracing::info!(
        config = %file.as_ref().map_or("<built-in defaults>".into(), |p| p.display().to_string()),
        version = env!("CARGO_PKG_VERSION"),
        "audio-server starting"
    );

    match cli.command.unwrap_or(Command::Run {
        simulate: 0,
        no_bluetooth: false,
        output: None,
        status_interval: 5,
    }) {
        Command::Run {
            simulate,
            no_bluetooth,
            output,
            status_interval,
        } => {
            if let Some(o) = output {
                cfg.output.device = o;
            }
            cfg.validate()?;
            app::run(
                cfg,
                app::RunOptions {
                    simulate,
                    no_bluetooth,
                    status_interval: Duration::from_secs(status_interval.max(1)),
                },
            )
        }
        Command::ListNodes => app::list_nodes(),
        Command::CheckConfig => {
            println!("{cfg:#?}");
            println!("configuration OK");
            Ok(())
        }
    }
}
