use crate::module::{Module, Result, Health};

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