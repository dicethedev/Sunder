use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub event: AuditEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditEvent {
    SignRequest {
        key_name: String,
        message_hex: String,
        nodes_participated: Vec<usize>,
        success: bool,
    },
    KeyLoaded {
        key_name: String,
        scheme: String,
    },
    NodeStarted {
        node_index: usize,
        bind_addr: String,
    },
    SignFailed {
        key_name: String,
        reason: String,
    },
}

pub struct AuditLog {
    path: PathBuf,
    lock: Mutex<()>,
}

impl AuditLog {
    pub fn new(path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
            lock: Mutex::new(()),
        }
    }

    pub fn write(&self, event: AuditEvent) {
        let _guard = self.lock.lock().unwrap();

        let entry = AuditEntry {
            timestamp: chrono_now(),
            event,
        };

        if let Ok(line) = serde_json::to_string(&entry) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(file, "{}", line);
            }
        }
    }
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}
