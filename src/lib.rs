// Health status for any running component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

// Error type placeholder.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

// Minimal lifecycle trait for components.
pub trait Module {
    // Return the name of the module.
    fn name(&self) -> &str;

    // Start the module.
    fn start(&mut self) -> Result<()>;
    // Stop the module.
    fn stop(&mut self) -> Result<()>;

    // Check the health status of the module.
    fn health(&self) -> Health {
        Health::Healthy
    }
}

// Test module implementation.
#[cfg(test)]
mod tests {
    use super::*;

    struct TestModule {
        name: String,
        is_running: bool,
    }

    impl TestModule {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                is_running: false,
            }
        }
    }

    impl Module for TestModule {
        fn name(&self) -> &str {
            &self.name
        }

        fn start(&mut self) -> Result<()> {
            if self.is_running {
                return Err("Module already running".into());
            }
            self.is_running = true;
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            if !self.is_running {
                return Err("Module not running".into());
            }
            self.is_running = false;
            Ok(())
        }

        fn health(&self) -> Health {
            if self.is_running {
                Health::Healthy
            } else {
                Health::Unhealthy { reason: "Not running".into() }
            }
        }
    }

    #[test]
    fn test_module_lifecycle() {
        let mut module = TestModule::new("TestModule");

        assert_eq!(module.name(), "TestModule");
        assert_eq!(module.health(), Health::Unhealthy { reason: "Not running".into() });

        module.start().unwrap();
        assert_eq!(module.health(), Health::Healthy);

        module.stop().unwrap();
        assert_eq!(module.health(), Health::Unhealthy { reason: "Not running".into() });
    }
}