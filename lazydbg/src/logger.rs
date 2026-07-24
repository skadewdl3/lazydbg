use std::collections::VecDeque;
use std::sync::{Arc, OnceLock, RwLock};

use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;

// ---------- Storage ----------

pub struct LogEntry {
    pub level: Level,
    pub target: String,
    pub message: String,
}

pub struct LogStore {
    inner: RwLock<VecDeque<LogEntry>>,
    capacity: usize,
}

pub type SharedLogStore = Arc<LogStore>;

impl LogStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    pub fn push(&self, entry: LogEntry) {
        let mut guard = self.inner.write().unwrap();
        if guard.len() >= self.capacity {
            guard.pop_front();
        }
        guard.push_back(entry);
    }

    /// Snapshot for a widget to render — cheap clone of the entries it needs.
    pub fn snapshot(&self) -> Vec<String> {
        self.inner
            .read()
            .unwrap()
            .iter()
            .map(|e| format!("[{}] {}: {}", e.level, e.target, e.message))
            .collect()
    }
}

// ---------- Global access ----------

static LOG_STORE: OnceLock<SharedLogStore> = OnceLock::new();

/// Call this once at startup, before or alongside subscriber init.
pub fn global_log_store() -> SharedLogStore {
    LOG_STORE
        .get_or_init(|| Arc::new(LogStore::new(1000)))
        .clone()
}

// ---------- The tracing Layer ----------

pub struct LoggingLayer {
    logs: SharedLogStore,
}

impl LoggingLayer {
    pub fn new(logs: SharedLogStore) -> Self {
        Self { logs }
    }
}

/// Pulls the `message` field (and falls back to debug-formatting others)
/// out of a tracing Event into a plain String.
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else if self.message.is_empty() {
            // fallback: capture something even if there's no "message" field
            self.message = format!("{}={:?}", field.name(), value);
        }
    }
}

impl<S> Layer<S> for LoggingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        self.logs.push(LogEntry {
            level: *event.metadata().level(),
            target: event.metadata().target().to_string(),
            message: visitor.message,
        });
    }
}

// ---------- Wiring it up ----------

pub fn init_logging() -> SharedLogStore {
    let store = global_log_store();

    let ui_layer = LoggingLayer::new(store.clone()).with_filter(LevelFilter::DEBUG);

    tracing_subscriber::registry().with(ui_layer).init();

    store
}
