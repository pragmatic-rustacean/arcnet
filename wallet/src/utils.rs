use std::{backtrace, panic, path::PathBuf};

use crate::core::{Config, Core, FeeConfig, FeeType, Recipient};
use anyhow::Result;
use tracing::*;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing to save logs in the logs/ folder
pub fn setup_tracing() -> Result<()> {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "wallet.log");
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(file_appender))
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::TRACE.into()))
        .init();

    Ok(())
}

/// Makes sure that tracing is able to log panics occuring in the wallet.
pub fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = backtrace::Backtrace::force_capture();
        error!("Application panicked");
        error!("Error info: {:#?}", panic_info);
        error!("Backtrace info: {:#?}", backtrace);
    }));
}

/// A dummy configuration generator function.
pub fn generate_dummy_config(path: PathBuf) -> Result<()> {
    debug!("Generating dummy config");
    let dummy_config = Config {
        keys: vec![],
        contacts: vec![
            Recipient {
                name: "Niko".to_string(),
                key: PathBuf::from("rose.pub.poem"),
            },
            Recipient {
                name: "James".to_string(),
                key: PathBuf::from("niko.pub.poem"),
            },
        ],
        default_node: "127.0.0.1:9000".to_string(),
        fee_config: FeeConfig {
            fee_type: FeeType::Percent,
            value: 0.1,
        },
    };
    let config_pr = toml::to_string_pretty(&dummy_config)?;
    std::fs::write(&path, config_pr)?;
    println!("Dummy config generated at : {}", path.display());
    info!("Dummy config generated at {}", path.display());

    Ok(())
}

fn sats_to_arc(sats: u64) -> String {
    let arc = sats as f64 / 100_000_000.0;
    format!("{} Arc", arc)
}

pub fn big_mode_arc(core: &Core) -> String {
    text_to_ascii_art::to_art(sats_to_arc(core.get_balance()), "bold", 3, 3, 2)
        .expect("failed miserably to display ascii art")
        .to_string()
}
