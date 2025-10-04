use hyperion::{Module, Health, Result}; // lib.rs

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

fn main() -> Result<()> {
    // Lifecycle demonstration
    let mut module = Hello;
    module.start()?;
    println!("[{}] Health: {:?}", module.name(), module.health());
    module.stop()?;
    Ok(())
}