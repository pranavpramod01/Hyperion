use serde::Deserialize;
use tracing_subscriber::{fmt, EnvFilter};

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

// Initialize logging based on the env or config. Prefer HYPERION_LOG if set or cfg.log_level. from config. Fallback to "info".
pub fn init_telemetry(cfg: &Config) -> Result<()> {
    // Determine the log level filter.
    let level_from_env = std::env::var("HYPERION_LOG").ok();
    let filter = match level_from_env {
        Some(s) => EnvFilter::try_new(s),
        None => EnvFilter::try_new(cfg.log_level.clone()),
    }
    .unwrap_or_else(|_| EnvFilter::new("info"));

    // Set up the subscriber with the filter and a formatter.
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();

    // Emit a small startup line.
    tracing::info!("telemetry initialized");
    Ok(())
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

// Axiom: Minimal runtime shell
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

    struct Demo { running: bool }
    impl Module for Demo {
        fn name(&self) -> &str { "demo" }
        fn start(&mut self) -> Result<()> { self.running = true; Ok(()) }
        fn stop(&mut self) -> Result<()> { self.running = false; Ok(()) }
        fn health(&self) -> Health {
            if self.running { Health::Healthy } else { Health::Degraded { reason: "stopped".into() } }
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

    // Test configuration loading from a valid file.
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

    // Test telemetry initialization.
    #[test]
    fn telemetry_init_smoke() {
        let cfg = Config { log_level: "debug".into(), data_dir: "data".into() };
        let _ = init_telemetry(&cfg);
        tracing::debug!("debug line after init");
    }

    // Test runtime with multiple modules.
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