use crate::{Result, Scheduler, Vaultline, Event};
use clap::{Parser, Subcommand};

// CLI definition
#[derive(Parser, Debug)]
#[command(name = "halodeck", author, version, about = "Hyperion CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show scheduler status (queue depth, leased count)
    Status,

    // Show recent events from vaultline
    Logs {
        #[arg(long, default_value_t = 20)]
        tail: usize,
    },

    // Submit a new job to queue
    Submit {
        kind: String,
        payload: String,
    },
}

impl Cli {
    pub fn run(self, sched: &mut Scheduler, vault: &mut Vaultline) -> Result<()> {
        match self.command {
            Command::Status => {
                let depth = sched.depth();
                let leased = sched.leased_count();
                tracing::info!(depth, leased, "status");
                let _ = vault.append(Event::now("halodeck", "info", format!("status depth={depth} leased={leased}")));
                Ok(())
            }
            Command::Logs { tail } => {
                for event in vault.tail(tail) {
                    println!(
                        "{} [{}] {}: {}",
                        event.ts_ms, event.level, event.source, event.message
                    );
                }
                Ok(())
            }
            Command::Submit { kind, payload } => {
                let id = sched.enqueue(kind.clone(), payload.clone());
                tracing::info!(id, kind = %kind, "submitted job");
                let _ = vault.append(Event::now("halodeck", "info", format!("submitted job {id} to {kind}")));
                Ok(())
            }
        }
    }
}

// Testing CLI parsing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_status() {
        let args = vec!["halodeck", "status"];
        let cli = Cli::parse_from(args);
        match cli.command {
            Command::Status => {}
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_cli_logs() {
        let args = vec!["halodeck", "logs", "--tail", "10"];
        let cli = Cli::parse_from(args);
        match cli.command {
            Command::Logs { tail } => assert_eq!(tail, 10),
            _ => panic!("Expected Logs command"),
        }
    }

    #[test]
    fn test_cli_submit() {
        let args = vec!["halodeck", "submit", "email", "Welcome to Hyperion!"];
        let cli = Cli::parse_from(args);
        match cli.command {
            Command::Submit { kind, payload } => {
                assert_eq!(kind, "email");
                assert_eq!(payload, "Welcome to Hyperion!");
            }
            _ => panic!("Expected Submit command"),
        }
    }
}
