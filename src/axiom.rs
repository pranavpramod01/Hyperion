use crate::module::{Module, Result, Health};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::thread;
use std::time::Duration;

/// Minimal runtime shell that manages registered modules.
pub struct Runtime {
    modules: Vec<Box<dyn Module>>,
}

impl Runtime {
    pub fn new() -> Self { Self { modules: Vec::new() } }

    pub fn register<M: Module + 'static>(&mut self, m: M) {
        self.modules.push(Box::new(m));
    }

    /// Start all modules in registration order.
    pub fn start_all(&mut self) -> Result<()> {
        for m in self.modules.iter_mut() { m.start()?; }
        Ok(())
    }

    /// Stop all modules in reverse order.
    pub fn stop_all(&mut self) -> Result<()> {
        for m in self.modules.iter_mut().rev() { m.stop()?; }
        Ok(())
    }

    /// Aggregate health (first non-Healthy wins).
    pub fn overall_health(&self) -> Health {
        for m in &self.modules {
            match m.health() {
                Health::Healthy => continue,
                other => return other,
            }
        }
        Health::Healthy
    }

    // Start modules and block until Ctrl-C, then stop modules. Returns Ok even if the Ctrl-C handler was already installed elsewhere.
    pub fn run_until_ctrlc(&mut self) -> Result<()> {
        // Start modules
        self.start_all()?;
        tracing::info!("runtime: started; press Ctrl-C to stop");

        // Setup Ctrl-C handler
        let shutdown = Arc::new(AtomicBool::new(false));

        // Clone for handler
        {
            let flag = shutdown.clone();
            let _ = ctrlc::set_handler(move || {
                flag.store(true, Ordering::SeqCst);
            });
        }

        // Wait for signal
        while !shutdown.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
        }

        // Stop modules
        tracing::info!("runtime: shutting down");
        self.stop_all()?;
        tracing::info!("runtime: stopped");
        Ok(())
    }

    // Check if Ctrl-C has been pressed (non-blocking); returns true if shutdown requested.
    pub fn poll_ctrl_c(shutdown: &AtomicBool) -> bool {
        shutdown.swap(false, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Demo { running: bool }
    impl Module for Demo {
        fn name(&self) -> &str { "demo" }
        fn start(&mut self) -> Result<()> { self.running = true; Ok(()) }
        fn stop(&mut self) -> Result<()> { self.running = false; Ok(()) }
        fn health(&self) -> Health {
            if self.running { Health::Healthy } else { Health::Degraded { reason: "stopped".into() } }
        }
    }

    #[test]
    fn runtime_lifecycle() {
        let mut rt = Runtime::new();
        rt.register(Demo { running: false });

        assert!(matches!(rt.overall_health(), Health::Degraded { .. }));

        rt.start_all().unwrap();
        assert!(matches!(rt.overall_health(), Health::Healthy));

        rt.stop_all().unwrap();
        assert!(matches!(rt.overall_health(), Health::Degraded { .. }));
    }
}