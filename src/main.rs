use std::path::Path;
use std::time::Duration;
use clap::Parser;

use hyperion::{
    init_telemetry, load_config, Result, Runtime, Module, Health,
    Vaultline, Event, Scheduler, HaloCli,
};

// Simple demo module
struct Hello { running: bool }
impl Module for Hello {
    fn name(&self) -> &str { "hello" }
    fn start(&mut self) -> Result<()> { self.running = true; tracing::info!("[{}] start", self.name()); Ok(()) }
    fn stop(&mut self) -> Result<()> { self.running = false; tracing::info!("[{}] stop", self.name()); Ok(()) }
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

    // Vaultline (file-backed)
    let log_path = Path::new(&cfg.data_dir).join("event.log");
    let mut vault = Vaultline::new(&log_path)?;
    let _ = vault.load_from_disk()?;

    // Scheduler (in-memory)
    let mut sched = Scheduler::new();

    // CLI path
    if let Ok(cli) = HaloCli::try_parse() {
        cli.run(&mut sched, &mut vault)?;
        return Ok(());
    }

    // Basic vaultline demo
    vault.append(Event::now("axiom", "info", "runtime starting"))?;
    vault.append(Event::now("hello", "info", "hello module warming up"))?;
    if let Some(last) = vault.tail(1).into_iter().next() {
        tracing::info!(last_level = %last.level, last_source = %last.source, last_msg = %last.message, "vaultline tail(1)");
    }

    // Epoch demo
    let _j1 = sched.enqueue("email", "Welcome to Hyperion!");
    let _j2 = sched.enqueue("email", "Send follow-up");
    if let Some(job) = sched.dequeue("email", Duration::from_secs(5)) {
        tracing::info!(id = job.id, kind = %job.kind, payload = %job.payload, "dequeued job");
        sched.complete(job.id)?;
        vault.append(Event::now("epoch", "info", format!("completed job {}", job.id)))?;
    }
    tracing::info!(depth = sched.depth(), leased = sched.leased_count(), "scheduler status");

    // Runtime demo
    let mut rt = Runtime::new();
    rt.register(Hello { running: false });
    rt.start_all()?;
    tracing::info!(overall = ?rt.overall_health(), "runtime health after start");
    rt.stop_all()?;
    tracing::info!(overall = ?rt.overall_health(), "runtime health after stop");

    Ok(())
}
