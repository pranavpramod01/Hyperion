use serde::Deserialize;

// Health status for any running component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

// Configuration structure with defaults.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

fn default_log_level() -> String { "info".to_string() }
fn default_data_dir() -> String { "data".to_string() }

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

// Load configurations from a file (HYPERION_CONFIG if set). Defaults if not found.
pub fn load_config() -> Result<Config> {
    let config_path = std::env::var("HYPERION_CONFIG").unwrap_or_else(|_| "config.toml".into());
    if std::path::Path::new(&config_path).exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    } else {
        Ok(Config {
            log_level: default_log_level(),
            data_dir: default_data_dir(),
        })
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

    // Test lifecycle and health checks.
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

    // Test configuration loading with defaults.
    #[test]
    fn load_config_defaults_when_missing() {
        // Point to a non-existent file to ensure defaults are used.
        unsafe {
            std::env::set_var("HYPERION_CONFIG", "___does_not_exist___hyperion.toml");
        }
        let cfg = load_config().expect("config should load with defaults");
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.data_dir, "data");
    }

    #[test]
    fn load_config_from_file() {
        // Create a temporary TOML file.
        let path = std::env::temp_dir().join(format!("hyperion_cfg_{}.toml", std::process::id()));
        std::fs::write(
            &path,
            r#"
            log_level = "debug"
            data_dir  = "test_data"
            "#,
        ).unwrap();

        unsafe {
            std::env::set_var("HYPERION_CONFIG", &path);
        }
        let cfg = load_config().expect("config should parse");
        assert_eq!(cfg.log_level, "debug");
        assert_eq!(cfg.data_dir, "test_data");

        let _ = std::fs::remove_file(&path);
    }
}