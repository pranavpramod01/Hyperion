use hyperion::{init_telemetry, load_config, Module, Runtime, Health, Result};

struct Hello {
    running: bool,
}

impl Module for Hello {
    fn name(&self) -> &str { "hello" }
    fn start(&mut self) -> Result<()> {
        self.running = true;
        tracing::info!("[{}] start", self.name());
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.running = false;
        tracing::info!("[{}] stop", self.name());
        Ok(())
    }
    fn health(&self) -> Health {
        if self.running { Health::Healthy } else { Health::Degraded { reason: "stopped".into() } }
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    // Load config + init logs
    let cfg = load_config()?;
    init_telemetry(&cfg)?;

    tracing::info!(version = VERSION, data_dir = %cfg.data_dir, "HYPERION starting");

    // Build a tiny runtime and register one demo module
    let mut rt = Runtime::new();
    rt.register(Hello { running: false });

    // Start -> report health -> stop
    rt.start_all()?;
    tracing::info!(rt_overall = format!("{:?}", rt.overall_health()), "runtime health after start");

    rt.stop_all()?;
    tracing::info!(rt_overall = format!("{:?}", rt.overall_health()), "runtime health after stop");

    Ok(())
}
