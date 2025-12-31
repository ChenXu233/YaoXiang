//! Task scheduler for concurrent execution

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

/// Task ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(usize);

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Waiting,
    Finished,
    Cancelled,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Task
#[derive(Debug)]
pub struct Task {
    id: TaskId,
    state: Mutex<TaskState>,
    priority: TaskPriority,
    stack_size: usize,
}

impl Task {
    /// Create a new task
    pub fn new(id: TaskId, priority: TaskPriority, stack_size: usize) -> Self {
        Self {
            id,
            state: Mutex::new(TaskState::Ready),
            priority,
            stack_size,
        }
    }

    /// Get task ID
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Get task state
    pub fn state(&self) -> TaskState {
        *self.state.lock().unwrap()
    }

    /// Set task state
    pub fn set_state(&self, state: TaskState) {
        *self.state.lock().unwrap() = state;
    }

    /// Get priority
    pub fn priority(&self) -> TaskPriority {
        self.priority
    }

    /// Get stack size
    pub fn stack_size(&self) -> usize {
        self.stack_size
    }
}

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Number of worker threads
    pub num_workers: usize,
    /// Default task stack size
    pub default_stack_size: usize,
    /// Work stealing batch size
    pub steal_batch: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        let num_cpus = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            num_workers: num_cpus,
            default_stack_size: 2 * 1024 * 1024,
            steal_batch: 4,
            max_queue_size: 1024,
        }
    }
}

/// Task scheduler
#[derive(Debug)]
pub struct Scheduler {
    /// Configuration
    config: SchedulerConfig,
    /// Global task queue
    global_queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    /// Per-worker local queues
    local_queues: Vec<Arc<Mutex<VecDeque<Arc<Task>>>>>,
    /// Worker threads
    workers: Vec<thread::JoinHandle<()>>,
    /// Running state
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self::with_config(SchedulerConfig::default())
    }

    /// Create scheduler with config
    pub fn with_config(config: SchedulerConfig) -> Self {
        let global_queue = Arc::new(Mutex::new(VecDeque::new()));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let mut local_queues = Vec::with_capacity(config.num_workers);
        for _ in 0..config.num_workers {
            local_queues.push(Arc::new(Mutex::new(VecDeque::new())));
        }

        let mut workers = Vec::with_capacity(config.num_workers);

        for i in 0..config.num_workers {
            let global_queue = global_queue.clone();
            let local_queue = local_queues[i].clone();
            let running = running.clone();

            let worker = thread::spawn(move || {
                Self::worker_loop(i, &global_queue, &local_queue, &running);
            });

            workers.push(worker);
        }

        Self {
            config,
            global_queue,
            local_queues,
            workers,
            running,
        }
    }

    /// Worker thread loop
    fn worker_loop(
        _id: usize,
        global_queue: &Arc<Mutex<VecDeque<Arc<Task>>>>,
        local_queue: &Arc<Mutex<VecDeque<Arc<Task>>>>,
        running: &Arc<std::sync::atomic::AtomicBool>,
    ) {
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            // Try to get a task
            if let Some(task) = Self::steal_or_get(global_queue, local_queue) {
                Self::execute_task(task);
            } else {
                // No task available, yield
                thread::sleep(Duration::from_millis(1));
            }
        }
    }

    /// Try to steal or get a task
    fn steal_or_get(
        global_queue: &Arc<Mutex<VecDeque<Arc<Task>>>>,
        local_queue: &Arc<Mutex<VecDeque<Arc<Task>>>>,
    ) -> Option<Arc<Task>> {
        // First try local queue
        if let Some(task) = local_queue.lock().unwrap().pop_front() {
            return Some(task);
        }

        // Then try global queue
        if let Some(task) = global_queue.lock().unwrap().pop_front() {
            return Some(task);
        }

        // Finally try to steal from other workers
        // This would need access to all local queues
        None
    }

    /// Execute a task
    fn execute_task(task: Arc<Task>) {
        task.set_state(TaskState::Running);
        // TODO: Execute task
        task.set_state(TaskState::Finished);
    }

    /// Spawn a new task
    pub fn spawn(&self, task: Arc<Task>) {
        self.global_queue.lock().unwrap().push_back(task);
    }

    /// Shutdown the scheduler
    pub fn shutdown(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);

        for worker in self.workers.drain(..) {
            worker.join().unwrap();
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.shutdown();
    }
}
