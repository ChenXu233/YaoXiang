//! Standard Concurrent library

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Spawn a new thread
pub fn spawn<F, T>(f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    thread::spawn(f)
}

/// Sleep for duration
pub fn sleep(duration: Duration) {
    thread::sleep(duration);
}

/// Get current thread ID (format as string for portability)
pub fn thread_id() -> String {
    format!("{:?}", thread::current().id())
}

/// Create a mutex
pub fn mutex_new<T>(value: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(value))
}

/// Lock mutex
pub fn mutex_lock<T>(mutex: &Arc<Mutex<T>>) -> std::sync::LockResult<std::sync::MutexGuard<'_, T>> {
    mutex.lock()
}

/// Atomic bool (placeholder)
pub struct AtomicBool {
    value: std::sync::atomic::AtomicUsize,
}

impl AtomicBool {
    /// Create new atomic bool
    pub fn new(value: bool) -> Self {
        Self {
            value: std::sync::atomic::AtomicUsize::new(if value { 1 } else { 0 }),
        }
    }

    /// Load value
    pub fn load(&self) -> bool {
        self.value.load(std::sync::atomic::Ordering::SeqCst) != 0
    }

    /// Store value
    pub fn store(
        &self,
        value: bool,
    ) {
        self.value.store(
            if value { 1 } else { 0 },
            std::sync::atomic::Ordering::SeqCst,
        );
    }
}

/// Atomic usize (placeholder)
pub struct AtomicUsize {
    value: std::sync::atomic::AtomicUsize,
}

impl AtomicUsize {
    /// Create new atomic usize
    pub fn new(value: usize) -> Self {
        Self {
            value: std::sync::atomic::AtomicUsize::new(value),
        }
    }

    /// Load value
    pub fn load(&self) -> usize {
        self.value.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Store value
    pub fn store(
        &self,
        value: usize,
    ) {
        self.value.store(value, std::sync::atomic::Ordering::SeqCst);
    }

    /// Add and return old value
    pub fn fetch_add(
        &self,
        delta: usize,
    ) -> usize {
        self.value
            .fetch_add(delta, std::sync::atomic::Ordering::SeqCst)
    }
}
