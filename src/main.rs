use hyperion::{Module, Health, Result, load_config}; // lib.rs

struct Hello;

impl Module for Hello {
    fn name(&self) -> &str {
        "Hello"
    }

    fn start(&mut self) -> Result<()> {
        println!("[{}] start", self.name());
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        println!("[{}] stop", self.name());
        Ok(())
    }

    fn health(&self) -> Health {
        Health::Healthy
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    // Lifecycle demonstration
    // let mut module = Hello;
    // module.start()?;
    // println!("[{}] Health: {:?}", module.name(), module.health());
    // module.stop()?;
    // Ok(())

    // Config demonstration
    let cfg = load_config()?;
    println!("HYPERION v{}", VERSION);
    println!("config: log_level={}, data_dir={}", cfg.log_level, cfg.data_dir);
    Ok(())
}