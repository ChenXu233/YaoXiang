//! Task queue for the scheduler
//!
//! Multi-producer, multi-consumer task queue with priority support.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::task::{Task, TaskId, TaskPriority, TaskState};

/// A thread-safe task queue supporting multiple producers and consumers.
#[derive(Debug)]
pub struct TaskQueue {
    /// Inner deque protected by mutex
    inner: Arc<Mutex<VecDeque<Arc<Task>>>>,
}

impl TaskQueue {
    /// Create a new empty task queue.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Push a task to the back of the queue.
    #[inline]
    pub fn push(&self, task: Arc<Task>) {
        let mut inner = self.inner.lock().unwrap();
        inner.push_back(task);
    }

    /// Push a task to the front of the queue (high priority).
    #[inline]
    pub fn push_front(&self, task: Arc<Task>) {
        let mut inner = self.inner.lock().unwrap();
        inner.push_front(task);
    }

    /// Pop a task from the front of the queue.
    #[inline]
    pub fn pop_front(&self) -> Option<Arc<Task>> {
        let mut inner = self.inner.lock().unwrap();
        inner.pop_front()
    }

    /// Pop a task from the back of the queue (for work stealing).
    #[inline]
    pub fn pop_back(&self) -> Option<Arc<Task>> {
        let mut inner = self.inner.lock().unwrap();
        inner.pop_back()
    }

    /// Peek at the front task without removing it.
    #[inline]
    pub fn peek_front(&self) -> Option<Arc<Task>> {
        let inner = self.inner.lock().unwrap();
        inner.front().cloned()
    }

    /// Get the number of tasks in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.len()
    }

    /// Check if the queue is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.is_empty()
    }

    /// Get a clone of the inner Arc for sharing.
    #[inline]
    pub fn inner(&self) -> Arc<Mutex<VecDeque<Arc<Task>>>> {
        self.inner.clone()
    }
}

impl Clone for TaskQueue {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority-aware task queue.
///
/// Tasks are ordered by priority, with higher priority tasks executed first.
#[derive(Debug)]
pub struct PriorityTaskQueue {
    /// High priority queue
    high: Arc<Mutex<VecDeque<Arc<Task>>>>,
    /// Normal priority queue
    normal: Arc<Mutex<VecDeque<Arc<Task>>>>,
    /// Low priority queue
    low: Arc<Mutex<VecDeque<Arc<Task>>>>,
}

impl PriorityTaskQueue {
    /// Create a new priority task queue.
    pub fn new() -> Self {
        Self {
            high: Arc::new(Mutex::new(VecDeque::new())),
            normal: Arc::new(Mutex::new(VecDeque::new())),
            low: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Push a task with the given priority.
    pub fn push(&self, task: Arc<Task>, priority: TaskPriority) {
        match priority {
            TaskPriority::Critical | TaskPriority::High => {
                self.high.lock().unwrap().push_back(task);
            }
            TaskPriority::Normal => {
                self.normal.lock().unwrap().push_back(task);
            }
            TaskPriority::Low => {
                self.low.lock().unwrap().push_back(task);
            }
        }
    }

    /// Pop the highest priority task available.
    pub fn pop(&self) -> Option<Arc<Task>> {
        // Try high priority first
        if let Some(task) = self.high.lock().unwrap().pop_front() {
            return Some(task);
        }
        // Then normal priority
        if let Some(task) = self.normal.lock().unwrap().pop_front() {
            return Some(task);
        }
        // Finally low priority
        self.low.lock().unwrap().pop_front()
    }

    /// Pop from back (for work stealing).
    pub fn pop_back(&self) -> Option<Arc<Task>> {
        // Try low priority first (less important tasks stolen first)
        if let Some(task) = self.low.lock().unwrap().pop_back() {
            return Some(task);
        }
        // Then normal priority
        if let Some(task) = self.normal.lock().unwrap().pop_back() {
            return Some(task);
        }
        // Finally high priority
        self.high.lock().unwrap().pop_back()
    }

    /// Get total number of tasks.
    pub fn len(&self) -> usize {
        let high_len = self.high.lock().unwrap().len();
        let normal_len = self.normal.lock().unwrap().len();
        let low_len = self.low.lock().unwrap().len();
        high_len + normal_len + low_len
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.high.lock().unwrap().is_empty()
            && self.normal.lock().unwrap().is_empty()
            && self.low.lock().unwrap().is_empty()
    }
}

impl Default for PriorityTaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PriorityTaskQueue {
    fn clone(&self) -> Self {
        Self {
            high: self.high.clone(),
            normal: self.normal.clone(),
            low: self.low.clone(),
        }
    }
}

