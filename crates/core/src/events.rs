//! A transport-free event bus.
//!
//! The device, capture, calibration, firmware and download workers all run on
//! their own threads. They report progress through this type instead of a
//! window handle, so the core stays free of any UI toolkit. The application
//! installs a sink that feeds its own queue and redraws from there.

use std::{
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use serde::Serialize;
use serde_json::Value;

type Sink = Arc<dyn Fn(&'static str, Value) + Send + Sync>;

/// A cloneable handle that forwards typed payloads to the application's sink.
#[derive(Clone)]
pub struct Events {
    sink: Sink,
    dropped: Arc<AtomicU64>,
}

impl Events {
    /// Builds a handle from a sink. The sink must not block: it is called from
    /// capture and flash worker threads.
    pub fn new(sink: impl Fn(&'static str, Value) + Send + Sync + 'static) -> Self {
        Self {
            sink: Arc::new(sink),
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    /// A handle that discards everything, for tests and headless use.
    pub fn noop() -> Self {
        Self::new(|_, _| {})
    }

    /// Serializes `payload` and hands it to the sink. Failures are counted and
    /// dropped: a progress update must never abort a worker thread.
    pub fn emit<T: Serialize>(&self, name: &'static str, payload: T) {
        let value = match serde_json::to_value(payload) {
            Ok(value) => value,
            Err(error) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                tracing::warn!("failed to serialize {name}: {error}");
                return;
            }
        };
        (self.sink)(name, value);
    }

    /// How many payloads could not be serialized, for diagnostics.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl fmt::Debug for Events {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Events")
            .field("dropped", &self.dropped())
            .finish_non_exhaustive()
    }
}

/// The type the workers carry; named after the handle it replaces so the
/// capture, calibration, firmware and download code reads the same as before.
pub type AppHandle = Events;
