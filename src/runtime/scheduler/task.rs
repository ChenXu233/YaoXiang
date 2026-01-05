//! Task definitions for the scheduler.
//!
//! This module defines tasks that can be scheduled and executed by the FlowScheduler.

use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex,
};
use std::thread;
use std::time::Duration;

/// Unique task identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

impl TaskId {
    /// Get the inner value.
    #[inline]
    pub fn inner(&self) -> usize {
        self.0
    }
}

impl From<usize> for TaskId {
    fn from(val: usize) -> Self {
        Self(val)
    }
}

impl From<TaskId> for usize {
    fn from(val: TaskId) -> Self {
        val.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "Task({})", self.0)
    }
}

/// Task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is waiting to be scheduled.
    Ready,
    /// Task is currently executing.
    Running,
    /// Task is waiting for dependencies.
    Waiting,
    /// Task has completed successfully.
    Finished,
    /// Task has failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
}

impl TaskState {
    /// Convert from u8 (for atomic storage).
    #[inline]
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => TaskState::Ready,
            1 => TaskState::Running,
            2 => TaskState::Waiting,
            3 => TaskState::Finished,
            4 => TaskState::Failed,
            5 => TaskState::Cancelled,
            _ => TaskState::Ready,
        }
    }

    /// Convert to u8 (for atomic storage).
    #[inline]
    pub fn as_u8(&self) -> u8 {
        match self {
            TaskState::Ready => 0,
            TaskState::Running => 1,
            TaskState::Waiting => 2,
            TaskState::Finished => 3,
            TaskState::Failed => 4,
            TaskState::Cancelled => 5,
        }
    }
}

/// Task priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Default)]
pub enum TaskPriority {
    /// Low priority tasks.
    Low = 0,
    /// Normal priority (default).
    #[default]
    Normal = 1,
    /// High priority tasks.
    High = 2,
    /// Critical priority tasks.
    Critical = 3,
}

/// A task that can be scheduled for execution.
pub struct Task {
    /// Unique task ID.
    id: TaskId,
    /// Task name for debugging.
    name: String,
    /// Current state (atomic for thread-safe access).
    state: AtomicU8,
    /// Priority of the task.
    priority: TaskPriority,
    /// Stack size for the task (if spawning a new thread).
    stack_size: usize,
    /// The actual work to execute.
    executor: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    /// Execution duration (for statistics).
    exec_duration: Mutex<Option<Duration>>,
}

impl std::fmt::Debug for Task {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("state", &self.state())
            .field("priority", &self.priority)
            .field("stack_size", &self.stack_size)
            .finish()
    }
}

impl Task {
    /// Create a new task with the given ID and executor.
    pub fn new<F>(
        id: TaskId,
        priority: TaskPriority,
        stack_size: usize,
        executor: F,
    ) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            id,
            name: format!("Task({})", id.inner()),
            state: AtomicU8::new(TaskState::Ready as u8),
            priority,
            stack_size,
            executor: Mutex::new(Some(Box::new(executor))),
            exec_duration: Mutex::new(None),
        }
    }

    /// Create a simple task (without executor, for testing).
    pub fn simple(
        id: TaskId,
        priority: TaskPriority,
        stack_size: usize,
    ) -> Self {
        Self {
            id,
            name: format!("Task({})", id.inner()),
            state: AtomicU8::new(TaskState::Ready as u8),
            priority,
            stack_size,
            executor: Mutex::new(None),
            exec_duration: Mutex::new(None),
        }
    }

    /// Get the task ID.
    #[inline]
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Get the task name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current state.
    #[inline]
    pub fn state(&self) -> TaskState {
        TaskState::from_u8(self.state.load(Ordering::SeqCst))
    }

    /// Set the task state.
    #[inline]
    pub fn set_state(
        &self,
        state: TaskState,
    ) {
        self.state.store(state.as_u8(), Ordering::SeqCst);
    }

    /// Get the priority.
    #[inline]
    pub fn priority(&self) -> TaskPriority {
        self.priority
    }

    /// Get the stack size.
    #[inline]
    pub fn stack_size(&self) -> usize {
        self.stack_size
    }

    /// Check if the task is ready to run.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.state() == TaskState::Ready
    }

    /// Check if the task is currently running.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.state() == TaskState::Running
    }

    /// Check if the task is finished.
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.state() == TaskState::Finished
    }

    /// Take the executor closure from the task.
    #[inline]
    pub fn take_executor(&self) -> Option<Box<dyn FnOnce() + Send>> {
        self.executor.lock().unwrap().take()
    }

    /// Record the execution duration.
    #[inline]
    pub fn record_duration(
        &self,
        duration: Duration,
    ) {
        *self.exec_duration.lock().unwrap() = Some(duration);
    }

    /// Get the execution duration.
    #[inline]
    pub fn exec_duration(&self) -> Option<Duration> {
        *self.exec_duration.lock().unwrap()
    }
}

/// Configuration for task execution.
#[derive(Debug, Clone)]
pub struct TaskConfig {
    /// Default stack size for tasks.
    pub default_stack_size: usize,
    /// Maximum stack size for tasks.
    pub max_stack_size: usize,
    /// Whether to use native threads or green threads.
    pub use_native_threads: bool,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            default_stack_size: 2 * 1024 * 1024, // 2MB
            max_stack_size: 64 * 1024 * 1024,    // 64MB
            use_native_threads: true,
        }
    }
}

/// Task builder for constructing tasks with various options.
#[derive(Debug, Default)]
pub struct TaskBuilder {
    name: Option<String>,
    priority: TaskPriority,
    stack_size: Option<usize>,
}

impl TaskBuilder {
    /// Create a new task builder.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the task name.
    #[inline]
    pub fn name(
        mut self,
        name: impl Into<String>,
    ) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the task priority.
    #[inline]
    pub fn priority(
        mut self,
        priority: TaskPriority,
    ) -> Self {
        self.priority = priority;
        self
    }

    /// Set the stack size.
    #[inline]
    pub fn stack_size(
        mut self,
        size: usize,
    ) -> Self {
        self.stack_size = Some(size);
        self
    }

    /// Build the task with the given ID and executor.
    pub fn build<F>(
        self,
        id: TaskId,
        executor: F,
    ) -> Task
    where
        F: FnOnce() + Send + 'static,
    {
        let stack_size = self.stack_size.unwrap_or_else(|| {
            thread::available_parallelism()
                .map(|n| n.get() * 1024 * 1024)
                .unwrap_or(2 * 1024 * 1024)
        });

        let name = self.name.unwrap_or_else(|| format!("Task({})", id.inner()));

        Task {
            id,
            name,
            state: AtomicU8::new(TaskState::Ready as u8),
            priority: self.priority,
            stack_size,
            executor: Mutex::new(Some(Box::new(executor))),
            exec_duration: Mutex::new(None),
        }
    }
}

/// Iterator for generating task IDs.
#[derive(Debug)]
pub struct TaskIdGenerator {
    next_id: usize,
}

impl TaskIdGenerator {
    /// Create a new task ID generator.
    #[inline]
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    /// Generate the next task ID.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        TaskId(id)
    }
}

impl Default for TaskIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}
