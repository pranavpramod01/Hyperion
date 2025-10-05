pub mod module;
pub mod config;
pub mod telemetry;
pub mod axiom;
pub mod vaultline;
pub mod epoch;
pub mod halodeck;

// Re-export key items for easier access
pub use module::{Health, Module, Result, Error};
pub use config::{Config, load_config};
pub use telemetry::init_telemetry;
pub use vaultline::{Vaultline, Event};
pub use axiom::Runtime;
pub use epoch::{Scheduler, Job};
pub use halodeck::{Cli as HaloCli, Command as HaloCommand};