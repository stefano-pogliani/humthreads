use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

use crate::ThreadScope;

/// Fake a `ThreadScope` for use in tests.
#[derive(Default)]
pub struct MockThreadScope {
    activity: Arc<Mutex<Option<String>>>,
    shutdown: Arc<AtomicBool>,
}

impl MockThreadScope {
    pub fn new() -> MockThreadScope {
        MockThreadScope {
            activity: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns a `ThreadScope` reflecting the state of this mock.
    pub fn scope(&self) -> ThreadScope {
        ThreadScope::new(Arc::clone(&self.activity), Arc::clone(&self.shutdown))
    }

    /// Set the shutdown state that `ThreadScope::should_shutdown` will return.
    pub fn set_shutdown(&self, value: bool) {
        self.shutdown.store(value, Ordering::Relaxed);
    }
}
