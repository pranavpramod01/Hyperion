use crate::module::{Result};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// Minimum shape for logging and auditing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub ts_ms: u128,
    pub source: String,
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub kv: serde_json::Value,
}

impl Event {
    // Create a new event with current timestamp and empty kv.
    pub fn now<S: Into<String>, L: Into<String>, M: Into<String>>(source: S, level: L, message: M) -> Self {
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Self {
            ts_ms,
            source: source.into(),
            level: level.into(),
            message: message.into(),
            kv: serde_json::Value::Null,
        }
    }
}

// Append-only event log with file backing.
pub struct Vaultline {
    mem: Vec<Event>,
    file: Option<PathBuf>,
}

impl Vaultline {
    // Open or create a vaultline at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        // Ensure the file exists, but do not eagerly load it into memory here.
        // Use `load_from_disk` when the caller explicitly wants to populate memory.
        let _file = OpenOptions::new().create(true).append(true).read(true).open(&path)?;

        Ok(Self { mem: Vec::new(), file: Some(path) })
    }

    // Append a new event to the vaultline.
    pub fn append(&mut self, event: Event) -> Result<()> {
        // Keep an in-memory copy of the original event (do not mutate the caller's event)
        self.mem.push(event.clone());

        // If file-backed, append an NDJSON line using the original event (do not normalize
        // here so that stored events match what the caller provided).
        if let Some(ref path) = self.file {
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            let line = serde_json::to_string(&event)?;
            use std::io::Write as _;
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;

            // Durability if env set
            if std::env::var("HYPERION_STRICT_DURABILITY").as_deref() == Ok("1") {
                file.sync_all()?;
            }
        }

        Ok(())
    }

    // Retrieve the last n events in memory.
    pub fn tail(&self, n: usize) -> Vec<&Event> {
        let len = self.mem.len();
        let start = len.saturating_sub(n);
        self.mem[start..].iter().collect()
    }

    // Create an in-memory only vaultline (no file).
    pub fn new_in_memory() -> Self {
        Self { mem: Vec::new(), file: None }
    }

    pub fn normalize_event(ev: &mut Event) {
        if ev.level.is_empty() {
            ev.level = "info".to_string();
        }
        if ev.source.is_empty() {
            ev.source = "unknown".to_string();
        }
        if ev.message.is_empty() {
            ev.message = "(no message)".to_string();
        }

        if ev.ts_ms == 0 {
            ev.ts_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
        }  

        if ev.kv.is_null() {
            ev.kv = serde_json::Value::Object(serde_json::Map::new());
        }
    }

    // Load events from disk into memory (if file exists).
    pub fn load_from_disk(&mut self) -> Result<usize> {
        let Some(ref path) = self.file else { return Ok(0) };
        if !path.exists() { return Ok(0) }
        let f = OpenOptions::new().read(true).open(path)?;
        let mut added = 0usize;
        for line in BufReader::new(f).lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            if let Ok(ev) = serde_json::from_str::<Event>(&line) {
                self.mem.push(ev);
                added += 1;
            }
        }
        Ok(added)
    }

    // Retrieve all events in memory.
    pub fn all(&self) -> &[Event] {
        &self.mem
    }
}

// Unit tests for Vaultline
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let ev = Event::now("test_source", "info", "This is a test");
        assert_eq!(ev.source, "test_source");
        assert_eq!(ev.level, "info");
        assert_eq!(ev.message, "This is a test");
        assert!(ev.ts_ms > 0);
        assert!(ev.kv.is_null());
    }

    #[test]
    fn test_vaultline_in_memory() {
        let mut vault = Vaultline::new_in_memory();
        assert_eq!(vault.all().len(), 0);

        let ev1 = Event::now("src1", "info", "First event");
        vault.append(ev1.clone()).unwrap();
        assert_eq!(vault.all().len(), 1);
        assert_eq!(vault.all()[0], ev1);

        let ev2 = Event::now("src2", "error", "Second event");
        vault.append(ev2.clone()).unwrap();
        assert_eq!(vault.all().len(), 2);
        assert_eq!(vault.all()[1], ev2);

        let tail = vault.tail(1);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0], &ev2);

        let tail_all = vault.tail(5);
        assert_eq!(tail_all.len(), 2);
    }

    #[test]
    fn test_vaultline_file_backed() {
        let temp_dir = std::env::temp_dir();
        let log_path = temp_dir.join(format!("vaultline_test_{}.log", std::process::id()));
        if log_path.exists() {
            let _ = std::fs::remove_file(&log_path);
        }

        let mut vault = Vaultline::new(&log_path).unwrap();
        assert_eq!(vault.all().len(), 0);

        let ev1 = Event::now("file_src1", "info", "File event 1");
        vault.append(ev1.clone()).unwrap();
        assert_eq!(vault.all().len(), 1);

        let ev2 = Event::now("file_src2", "warn", "File event 2");
        vault.append(ev2.clone()).unwrap();
        assert_eq!(vault.all().len(), 2);

        // Reload from disk
        let mut vault_reload = Vaultline::new(&log_path).unwrap();
        assert_eq!(vault_reload
            .load_from_disk()
            .unwrap(), 2);
        assert_eq!(vault_reload.all().len(), 2);
        assert_eq!(vault_reload.all()[0], ev1);
        assert_eq!(vault_reload.all()[1], ev2);
        let _ = std::fs::remove_file
            (&log_path);
    }   
    #[test]
    fn test_event_normalization() {
        let mut ev = Event {
            ts_ms: 0,
            source: "".into(),
            level: "".into(),
            message: "".into(),
            kv: serde_json::Value::Null,
        };
        Vaultline::normalize_event(&mut ev);
        assert_eq!(ev.source, "unknown");
        assert_eq!(ev.level, "info");
        assert_eq!(ev.message, "(no message)");
        assert!(ev.ts_ms > 0);
        assert!(ev.kv.is_object() && ev.kv.as_object().unwrap().is_empty());
    }
}