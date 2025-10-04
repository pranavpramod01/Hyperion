use serde::Deserialize;
use crate::module::Result;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

fn default_log_level() -> String { "info".to_string() }
fn default_data_dir() -> String { "data".to_string() }

/// Load configuration from `HYPERION_CONFIG` (TOML) if set, otherwise `config.toml`.
/// If the file doesn't exist, return safe defaults.
pub fn load_config() -> Result<Config> {
    let config_path = std::env::var("HYPERION_CONFIG").unwrap_or_else(|_| "config.toml".into());
    if std::path::Path::new(&config_path).exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    } else {
        Ok(Config { log_level: default_log_level(), data_dir: default_data_dir() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing() {
        // Keep unsafe if your toolchain insists.
        unsafe { std::env::set_var("HYPERION_CONFIG", "___does_not_exist___hyperion.toml"); }
        let cfg = load_config().unwrap();
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.data_dir, "data");
    }

    #[test]
    fn reads_file() {
        let path = std::env::temp_dir().join(format!("hyperion_cfg_{}.toml", std::process::id()));
        std::fs::write(&path, r#"log_level = "debug"
data_dir = "test_data""#).unwrap();

        unsafe { std::env::set_var("HYPERION_CONFIG", &path); }
        let cfg = load_config().unwrap();
        assert_eq!(cfg.log_level, "debug");
        assert_eq!(cfg.data_dir, "test_data");

        let _ = std::fs::remove_file(path);
    }
}