//! Task scheduling for YaoXiang Runtime
//!
//! This module provides task-based concurrency support per RFC-008.
//! Memory management uses Arc (ref keyword in YaoXiang), no GC.
//!
//! ## Three-tier Runtime Support
//!
//! - **Embedded Runtime**: Immediate executor, no DAG, sync execution
//! - **Standard Runtime**: DAG scheduler, lazy evaluation, async/concurrent
//! - **Full Runtime**: + WorkStealer, parallel optimization

use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use std::fmt;

/// Type alias for shared sync values
pub type SyncValue = Arc<dyn std::any::Any + Send + Sync>;

/// Type alias for task result
pub type TaskResult = Result<SyncValue, SyncValue>;

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TaskId(usize);

impl TaskId {
    /// Create a new task ID
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Get the underlying value
    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Task({})", self.0)
    }
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TaskPriority {
    /// Low priority tasks
    Low = 0,
    /// Normal priority tasks (default)
    #[default]
    Normal = 1,
    /// High priority tasks
    High = 2,
    /// Critical priority tasks
    Critical = 3,
}

/// Task state
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    /// Task created but not started
    Pending,
    /// Task is currently executing
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with error
    Failed(String),
    /// Task was cancelled
    Cancelled,
}

/// Task configuration
#[derive(Debug, Clone, Default)]
pub struct TaskConfig {
    /// Task priority
    pub priority: TaskPriority,
    /// Task name for debugging
    pub name: String,
    /// Stack size in bytes (0 = default)
    pub stack_size: usize,
    /// Parent task ID (for task tree)
    pub parent_id: Option<TaskId>,
}

impl TaskConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set task priority
    pub fn with_priority(
        mut self,
        priority: TaskPriority,
    ) -> Self {
        self.priority = priority;
        self
    }

    /// Set task name
    pub fn with_name(
        mut self,
        name: impl Into<String>,
    ) -> Self {
        self.name = name.into();
        self
    }

    /// Set parent task
    pub fn with_parent(
        mut self,
        parent: TaskId,
    ) -> Self {
        self.parent_id = Some(parent);
        self
    }
}

/// A spawned task
///
/// Per RFC-009: Task boundary is the leak boundary.
/// Cycles within a task are allowed and will be released when task ends.
#[derive(Debug)]
pub struct Task {
    /// Unique task ID
    id: TaskId,
    /// Task configuration
    config: TaskConfig,
    /// Current state
    state: TaskState,
    /// Result storage (once completed)
    result: Option<TaskResult>,
}

impl Task {
    /// Create a new task
    pub fn new(
        id: TaskId,
        config: TaskConfig,
    ) -> Self {
        Self {
            id,
            config,
            state: TaskState::Pending,
            result: None,
        }
    }

    /// Get task ID
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Get task name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get priority
    pub fn priority(&self) -> TaskPriority {
        self.config.priority
    }

    /// Get current state
    pub fn state(&self) -> &TaskState {
        &self.state
    }

    /// Mark as running
    pub fn set_running(&mut self) {
        self.state = TaskState::Running;
    }

    /// Mark as completed with result
    pub fn set_completed<T: Send + Sync + 'static>(
        &mut self,
        value: T,
    ) {
        self.state = TaskState::Completed;
        self.result = Some(Ok(Arc::new(value)));
    }

    /// Mark as failed with error
    pub fn set_failed<E: std::fmt::Debug + Send + Sync + 'static>(
        &mut self,
        error: E,
    ) {
        self.state = TaskState::Failed(format!("{:?}", error));
        self.result = Some(Err(Arc::new(error)));
    }

    /// Get result if completed
    pub fn result(&self) -> Option<&TaskResult> {
        self.result.as_ref()
    }
}

/// Task context for async function execution
///
/// Contains all state needed to execute a task:
/// - Registers for bytecode interpreter
/// - Stack for function calls
/// - Task-local storage
#[derive(Debug, Default)]
pub struct TaskContext {
    /// Current task ID
    task_id: TaskId,
    /// Register file for the interpreter
    registers: Vec<Arc<dyn std::any::Any + Send + Sync>>,
    /// Stack for function calls
    stack: Vec<Arc<dyn std::any::Any + Send + Sync>>,
    /// Task-local storage (per RFC-008, not shared between tasks)
    locals: HashMap<usize, Arc<dyn std::any::Any + Send + Sync>>,
    /// Entry point IP (for stack unwinding)
    entry_ip: usize,
}

impl TaskContext {
    /// Create a new task context
    pub fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            registers: Vec::new(),
            stack: Vec::new(),
            locals: HashMap::new(),
            entry_ip: 0,
        }
    }

    /// Get task ID
    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    /// Get a register value
    pub fn get_register(
        &self,
        index: usize,
    ) -> Option<&Arc<dyn std::any::Any + Send + Sync>> {
        self.registers.get(index)
    }

    /// Set a register value
    pub fn set_register(
        &mut self,
        index: usize,
        value: Arc<dyn std::any::Any + Send + Sync>,
    ) {
        if index >= self.registers.len() {
            self.registers.resize(index + 1, Arc::new(()));
        }
        self.registers[index] = value;
    }

    /// Push onto stack
    pub fn push<T: Send + Sync + 'static>(
        &mut self,
        value: T,
    ) {
        self.stack.push(Arc::new(value));
    }

    /// Pop from stack
    pub fn pop<T: Send + Sync + 'static>(
        &mut self
    ) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.stack.pop()
    }

    /// Get stack top
    pub fn stack_top(&self) -> Option<&Arc<dyn std::any::Any + Send + Sync>> {
        self.stack.last()
    }

    /// Get a local value
    pub fn get_local(
        &self,
        key: usize,
    ) -> Option<&Arc<dyn std::any::Any + Send + Sync>> {
        self.locals.get(&key)
    }

    /// Set a local value
    pub fn set_local(
        &mut self,
        key: usize,
        value: Arc<dyn std::any::Any + Send + Sync>,
    ) {
        self.locals.insert(key, value);
    }

    /// Get entry IP
    pub fn entry_ip(&self) -> usize {
        self.entry_ip
    }

    /// Set entry IP
    pub fn set_entry_ip(
        &mut self,
        ip: usize,
    ) {
        self.entry_ip = ip;
    }
}

/// Scheduler statistics
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// Number of pending tasks
    pub pending_count: usize,
    /// Number of running tasks
    pub running_count: usize,
    /// Number of completed tasks
    pub completed_count: usize,
    /// Total tasks spawned
    pub total_spawned: usize,
    /// Average task execution time
    pub avg_execution_time: Duration,
}

/// Trait for scheduler implementations
///
/// Per RFC-008: Scheduler decouples via generics.
/// Different implementations:
/// - SingleThreadScheduler: async execution (num_workers=1)
/// - MultiThreadScheduler: parallel execution (num_workers>1)
pub trait Scheduler: Send + Sync {
    /// Spawn a new task
    fn spawn(
        &self,
        task: Arc<Task>,
        config: TaskConfig,
    ) -> TaskId;

    /// Await task completion and get result
    fn await_task(
        &self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError>;

    /// Spawn multiple tasks with dependencies
    fn spawn_with_deps(
        &self,
        task: Arc<Task>,
        config: TaskConfig,
        deps: &[TaskId],
    ) -> TaskId;

    /// Await multiple tasks
    fn await_all(
        &self,
        task_ids: &[TaskId],
    ) -> Result<(), RuntimeError>;

    /// Cancel a task
    fn cancel(
        &self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError>;

    /// Check if a task is complete
    fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool;

    /// Get scheduler statistics
    fn stats(&self) -> SchedulerStats;
}

/// Runtime error types
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// Task not found
    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),

    /// Task is still running
    #[error("Task still running: {0}")]
    TaskRunning(TaskId),

    /// Task was cancelled
    #[error("Task cancelled: {0}")]
    TaskCancelled(TaskId),

    /// Task failed with error
    #[error("Task failed: {0}")]
    TaskFailed(String),

    /// Scheduler error
    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Task spawner for managing task execution
///
/// Per RFC-008: Main entry point for creating tasks.
/// Uses Arc for shared state (ref keyword in YaoXiang).
#[derive(Debug, Default)]
pub struct TaskSpawner<S: Scheduler> {
    /// Scheduler implementation
    scheduler: Arc<S>,
}

impl<S: Scheduler> TaskSpawner<S> {
    /// Create a new task spawner with scheduler
    pub fn new(scheduler: Arc<S>) -> Self {
        Self { scheduler }
    }

    /// Spawn a new task with default config
    pub fn spawn(
        &mut self,
        task: Arc<Task>,
    ) -> TaskId {
        self.spawn_with_config(task, TaskConfig::default())
    }

    /// Spawn a new task with config
    pub fn spawn_with_config(
        &mut self,
        task: Arc<Task>,
        config: TaskConfig,
    ) -> TaskId {
        self.scheduler.spawn(task, config)
    }

    /// Spawn multiple tasks
    pub fn spawn_batch(
        &mut self,
        tasks: Vec<(Arc<Task>, TaskConfig)>,
    ) -> Vec<TaskId> {
        tasks
            .into_iter()
            .map(|(task, config)| self.spawn_with_config(task, config))
            .collect()
    }

    /// Await a single task
    pub fn await_task(
        &self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        self.scheduler.await_task(task_id)
    }

    /// Await multiple tasks
    pub fn await_all(
        &self,
        task_ids: &[TaskId],
    ) -> Result<(), RuntimeError> {
        self.scheduler.await_all(task_ids)
    }

    /// Check if task is complete
    pub fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool {
        self.scheduler.is_complete(task_id)
    }

    /// Cancel a task
    pub fn cancel(
        &self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        self.scheduler.cancel(task_id)
    }

    /// Get scheduler stats
    pub fn stats(&self) -> SchedulerStats {
        self.scheduler.stats()
    }

    /// Get scheduler reference
    pub fn scheduler(&self) -> &Arc<S> {
        &self.scheduler
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Simple scheduler for testing
    #[derive(Debug, Default)]
    struct TestScheduler {
        tasks: Arc<Mutex<HashMap<TaskId, Arc<Task>>>>,
        completed: Arc<Mutex<Vec<TaskId>>>,
    }

    impl Scheduler for TestScheduler {
        fn spawn(
            &self,
            task: Arc<Task>,
            config: TaskConfig,
        ) -> TaskId {
            let id = TaskId::new(config.name.parse().unwrap_or(0));
            self.tasks.lock().unwrap().insert(id, task);
            id
        }

        fn await_task(
            &self,
            _task_id: TaskId,
        ) -> Result<(), RuntimeError> {
            // Simulate waiting
            Ok(())
        }

        fn spawn_with_deps(
            &self,
            task: Arc<Task>,
            config: TaskConfig,
            _deps: &[TaskId],
        ) -> TaskId {
            self.spawn(task, config)
        }

        fn await_all(
            &self,
            _task_ids: &[TaskId],
        ) -> Result<(), RuntimeError> {
            Ok(())
        }

        fn cancel(
            &self,
            task_id: TaskId,
        ) -> Result<(), RuntimeError> {
            self.tasks.lock().unwrap().remove(&task_id);
            Ok(())
        }

        fn is_complete(
            &self,
            task_id: TaskId,
        ) -> bool {
            self.completed.lock().unwrap().contains(&task_id)
        }

        fn stats(&self) -> SchedulerStats {
            SchedulerStats::default()
        }
    }

    #[test]
    fn test_task_id() {
        let id = TaskId::new(42);
        assert_eq!(id.into_inner(), 42);
        assert_eq!(id.to_string(), "Task(42)");
    }

    #[test]
    fn test_task_config() {
        let config = TaskConfig::new()
            .with_priority(TaskPriority::High)
            .with_name("test_task");

        assert_eq!(config.priority, TaskPriority::High);
        assert_eq!(config.name, "test_task");
    }

    #[test]
    fn test_task_context() {
        let ctx = TaskContext::new(TaskId::new(1));
        assert_eq!(ctx.task_id(), TaskId::new(1));

        let mut ctx = TaskContext::new(TaskId::new(2));
        ctx.set_register(0, Arc::new(42));
        assert_eq!(
            ctx.get_register(0).map(|v| v.downcast_ref::<i32>()),
            Some(Some(&42))
        );

        ctx.push(100i32);
        assert_eq!(
            ctx.stack_top().map(|v| v.downcast_ref::<i32>()),
            Some(Some(&100))
        );
    }

    #[test]
    fn test_task_spawner() {
        let scheduler = Arc::new(TestScheduler::default());
        let mut spawner = TaskSpawner::new(scheduler);

        // Create a config with name "1" so the test scheduler extracts TaskId(1)
        let config = TaskConfig::new().with_name("1");
        let task = Arc::new(Task::new(TaskId::new(0), config.clone()));
        let id = spawner.spawn_with_config(task, config);

        assert_eq!(id, TaskId::new(1));
    }
}
