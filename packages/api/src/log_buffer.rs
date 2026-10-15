//! LogBuffer - Buffer en memoria para capturar eventos de tracing recientes

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LogBuffer {
    capacity: usize,
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
        }
    }

    pub fn push(&self, level: &str, message: String) {
        if let Ok(mut lock) = self.entries.lock() {
            if lock.len() >= self.capacity {
                lock.pop_front();
            }
            lock.push_back(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: level.to_string(),
                message,
            });
        }
    }

    pub fn get_recent(&self) -> Vec<LogEntry> {
        if let Ok(lock) = self.entries.lock() {
            lock.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

pub struct LogLayer {
    pub buffer: LogBuffer,
}

impl<S: Subscriber> Layer<S> for LogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = event.metadata().level().as_str();

        struct MessageVisitor(String);
        impl Visit for MessageVisitor {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{:?}", value);
                } else if self.0.is_empty() {
                    self.0 = format!("{}: {:?}", field.name(), value);
                }
            }
        }

        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        let clean_msg = visitor.0.trim_matches('"').to_string();
        if !clean_msg.is_empty() {
            self.buffer.push(level, clean_msg);
        }
    }
}
