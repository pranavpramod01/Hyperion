use tracing_subscriber::{fmt, filter::EnvFilter};
use crate::config::Config;
use crate::module::Result;

/// Initialize global logging based on env or config.
/// Order: HYPERION_LOG env -> cfg.log_level -> "info"
pub fn init_telemetry(cfg: &Config) -> Result<()> {
    let level_from_env = std::env::var("HYPERION_LOG").ok();
    let filter = match level_from_env {
        Some(s) => EnvFilter::try_new(s),
        None => EnvFilter::try_new(cfg.log_level.clone()),
    }.unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();

    tracing::info!("telemetry initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn init_smoke() {
        let cfg = Config { log_level: "debug".into(), data_dir: "data".into() };
        let _ = init_telemetry(&cfg); // should not panic
        tracing::debug!("debug after init");
    }
}