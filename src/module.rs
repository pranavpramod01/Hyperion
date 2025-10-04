#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

/// Common error/result aliases used across the crate.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

/// Minimal lifecycle every component should implement.
pub trait Module {
    fn name(&self) -> &str;
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn health(&self) -> Health { Health::Healthy }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestModule {
        name: String,
        is_running: bool,
    }

    impl TestModule {
        fn new(name: &str) -> Self {
            Self { name: name.to_string(), is_running: false }
        }
    }

    impl Module for TestModule {
        fn name(&self) -> &str { &self.name }
        fn start(&mut self) -> Result<()> {
            if self.is_running { return Err("Module already running".into()); }
            self.is_running = true;
            Ok(())
        }
        fn stop(&mut self) -> Result<()> {
            if !self.is_running { return Err("Module not running".into()); }
            self.is_running = false;
            Ok(())
        }
        fn health(&self) -> Health {
            if self.is_running { Health::Healthy } else { Health::Unhealthy { reason: "Not running".into() } }
        }
    }

    #[test]
    fn module_lifecycle() {
        let mut m = TestModule::new("TestModule");
        assert_eq!(m.name(), "TestModule");
        assert_eq!(m.health(), Health::Unhealthy { reason: "Not running".into() });
        m.start().unwrap();
        assert_eq!(m.health(), Health::Healthy);
        m.stop().unwrap();
        assert_eq!(m.health(), Health::Unhealthy { reason: "Not running".into() });
    }
}