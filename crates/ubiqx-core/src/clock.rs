use chrono::{DateTime, Local, Utc};

/// Source of "now". Injected everywhere so that segmentation, scheduling and insights can be
/// tested deterministically.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;

    /// Local wall-clock time (used for "report at 18:00" style rules and for day boundaries).
    fn now_local(&self) -> DateTime<Local> {
        self.now().with_timezone(&Local)
    }
}

/// The real system clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A clock that returns a fixed, adjustable instant. Intended for tests.
#[derive(Debug, Clone)]
pub struct FixedClock(pub std::sync::Arc<std::sync::Mutex<DateTime<Utc>>>);

impl FixedClock {
    pub fn new(at: DateTime<Utc>) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(at)))
    }

    pub fn set(&self, at: DateTime<Utc>) {
        *self.0.lock().expect("clock poisoned") = at;
    }

    pub fn advance(&self, d: chrono::Duration) {
        let mut g = self.0.lock().expect("clock poisoned");
        *g += d;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().expect("clock poisoned")
    }
}
