//! Task scheduler for concurrent execution
//!
//! This module provides the FlowScheduler, a dependency-aware task scheduler
//! that integrates with the DAG module for parallel execution.

pub mod queue;
pub mod task;
pub mod work_stealer;

pub use queue::{PriorityTaskQueue, TaskQueue};
pub use task::{Task, TaskBuilder, TaskConfig, TaskId, TaskIdGenerator, TaskPriority, TaskState};
pub use work_stealer::{StealStats, StealStrategy, WorkStealer};

use std::collections::{HashSet, VecDeque};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Condvar, Mutex, RwLock,
};
use std::thread;
use std::time::Duration;

use crate::runtime::dag::{ComputationDAG, DAGError, DAGNodeKind, NodeId};

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Number of worker threads.
    pub num_workers: usize,
    /// Default task stack size.
    pub default_stack_size: usize,
    /// Work stealing batch size.
    pub steal_batch: usize,
    /// Maximum queue size per worker.
    pub max_queue_size: usize,
    /// Whether to use work stealing.
    pub use_work_stealing: bool,
    /// Idle timeout before yielding.
    pub idle_timeout: Duration,
    /// Statistics collection enabled.
    pub enable_stats: bool,
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
            use_work_stealing: true,
            idle_timeout: Duration::from_millis(1),
            enable_stats: false,
        }
    }
}

/// Scheduler statistics.
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// Total tasks scheduled.
    pub tasks_scheduled: AtomicUsize,
    /// Total tasks completed.
    pub tasks_completed: AtomicUsize,
    /// Total tasks stolen.
    pub tasks_stolen: AtomicUsize,
    /// Total steal attempts.
    pub steal_attempts: AtomicUsize,
    /// Total successful steals.
    pub steal_success: AtomicUsize,
    /// Total execution time in microseconds.
    pub total_exec_time_us: AtomicUsize,
    /// Peak number of running tasks.
    pub peak_parallelism: AtomicUsize,
}

impl SchedulerStats {
    /// Record a scheduled task.
    #[inline]
    pub fn record_scheduled(&self) {
        self.tasks_scheduled.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a completed task.
    #[inline]
    pub fn record_completed(&self, duration_us: usize) {
        self.tasks_completed.fetch_add(1, Ordering::SeqCst);
        self.total_exec_time_us
            .fetch_add(duration_us, Ordering::SeqCst);
    }

    /// Record a steal.
    #[inline]
    pub fn record_steal(&self, success: bool) {
        self.tasks_stolen.fetch_add(1, Ordering::SeqCst);
        self.steal_attempts.fetch_add(1, Ordering::SeqCst);
        if success {
            self.steal_success.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Update parallelism.
    #[inline]
    pub fn update_parallelism(&self, current: usize) {
        loop {
            let peak = self.peak_parallelism.load(Ordering::SeqCst);
            if current <= peak {
                break;
            }
            if self
                .peak_parallelism
                .compare_exchange(peak, current, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Get steal success rate.
    pub fn steal_success_rate(&self) -> f64 {
        let attempts = self.steal_attempts.load(Ordering::SeqCst);
        if attempts == 0 {
            return 1.0;
        }
        self.steal_success.load(Ordering::SeqCst) as f64 / attempts as f64
    }
}

/// Dependency-aware scheduler that integrates with DAG.
///
/// FlowScheduler coordinates task execution based on dependency relationships
/// tracked in the ComputationDAG, enabling parallel execution of independent tasks.
#[derive(Debug)]
pub struct FlowScheduler {
    /// Configuration.
    config: SchedulerConfig,
    /// Computation DAG for dependency tracking (protected by mutex).
    dag: Arc<Mutex<ComputationDAG>>,
    /// Work stealer for load balancing (shared via Arc).
    work_stealer: Arc<WorkStealer>,
    /// Per-worker local queues (shared with work stealer).
    worker_queues: Arc<RwLock<Vec<Arc<TaskQueue>>>>,
    /// Global ready queue (DAG nodes ready to execute).
    ready_queue: Arc<Mutex<VecDeque<NodeId>>>,
    /// Worker threads.
    workers: Vec<thread::JoinHandle<()>>,
    /// Running state.
    running: Arc<AtomicBool>,
    /// Condition variable for waking workers.
    wake_condvar: Arc<Condvar>,
    /// Statistics.
    stats: Arc<SchedulerStats>,
    /// Task ID generator.
    task_id_generator: Mutex<TaskIdGenerator>,
    /// Completed DAG nodes to track dependency readiness.
    completed_nodes: Arc<Mutex<HashSet<NodeId>>>,
}

impl FlowScheduler {
    /// Create a new flow scheduler with default config.
    #[inline]
    pub fn new() -> Self {
        Self::with_config(SchedulerConfig::default())
    }

    /// Create a flow scheduler with custom configuration.
    pub fn with_config(config: SchedulerConfig) -> Self {
        let num_workers = config.num_workers;
        let running = Arc::new(AtomicBool::new(true));
        let wake_condvar = Arc::new(Condvar::new());

        // Create DAG for dependency tracking
        let dag = Arc::new(Mutex::new(ComputationDAG::new()));

        // Create work stealer
        let work_stealer = Arc::new(WorkStealer::new(num_workers));

        // Share the internal queues managed by the work stealer
        let worker_queues = work_stealer.queues().clone();

        // Create global ready queue for DAG nodes
        let ready_queue = Arc::new(Mutex::new(VecDeque::new()));

        // Create statistics
        let stats = Arc::new(SchedulerStats::default());

        // Create task ID generator
        let task_id_generator = Mutex::new(TaskIdGenerator::new());

        // Track completed DAG nodes for dependency resolution
        let completed_nodes = Arc::new(Mutex::new(HashSet::new()));

        // Spawn worker threads
        let workers = Self::spawn_workers(
            num_workers,
            &running,
            &wake_condvar,
            &work_stealer,
            &worker_queues,
            &ready_queue,
            &dag,
            &stats,
            &completed_nodes,
            &config,
        );

        Self {
            config,
            dag,
            work_stealer,
            worker_queues,
            ready_queue,
            workers,
            running,
            wake_condvar,
            stats,
            task_id_generator,
            completed_nodes,
        }
    }

    /// Spawn worker threads.
    fn spawn_workers(
        num_workers: usize,
        running: &Arc<AtomicBool>,
        wake_condvar: &Arc<Condvar>,
        work_stealer: &Arc<WorkStealer>,
        worker_queues: &Arc<RwLock<Vec<Arc<TaskQueue>>>>,
        ready_queue: &Arc<Mutex<VecDeque<NodeId>>>,
        dag: &Arc<Mutex<ComputationDAG>>,
        stats: &Arc<SchedulerStats>,
        completed_nodes: &Arc<Mutex<HashSet<NodeId>>>,
        config: &SchedulerConfig,
    ) -> Vec<thread::JoinHandle<()>> {
        let mut workers = Vec::with_capacity(num_workers);

        for worker_id in 0..num_workers {
            let running = running.clone();
            let wake_condvar = wake_condvar.clone();
            let work_stealer = work_stealer.clone();
            let worker_queues = worker_queues.clone();
            let ready_queue = ready_queue.clone();
            let dag = dag.clone();
            let stats = stats.clone();
            let completed_nodes = completed_nodes.clone();
            let config = config.clone();

            let worker = thread::Builder::new()
                .name(format!("flow-worker-{}", worker_id))
                .spawn(move || {
                    Self::worker_loop(
                        worker_id,
                        &running,
                        &wake_condvar,
                        &work_stealer,
                        &worker_queues,
                        &ready_queue,
                        &dag,
                        &stats,
                        &completed_nodes,
                        &config,
                    );
                })
                .expect("Failed to spawn worker thread");

            workers.push(worker);
        }

        workers
    }

    /// Worker thread main loop.
    fn worker_loop(
        worker_id: usize,
        running: &Arc<AtomicBool>,
        _wake_condvar: &Arc<Condvar>,
        work_stealer: &Arc<WorkStealer>,
        worker_queues: &Arc<RwLock<Vec<Arc<TaskQueue>>>>,
        ready_queue: &Arc<Mutex<VecDeque<NodeId>>>,
        dag: &Arc<Mutex<ComputationDAG>>,
        stats: &Arc<SchedulerStats>,
        completed_nodes: &Arc<Mutex<HashSet<NodeId>>>,
        config: &SchedulerConfig,
    ) {
        // Register with work stealer
        work_stealer.register_worker(worker_id);

        // Keep a handle to worker queues (currently unused but retained for future placement strategies)
        let _ = worker_queues;

        while running.load(Ordering::SeqCst) {
            // 1. Try to get a task from local queue
            if let Some(task) = work_stealer.try_local() {
                Self::execute_task(task, stats);
                continue;
            }

            // 2. Try to get a ready DAG node
            if let Some(node_id) = Self::try_ready_queue(ready_queue) {
                Self::execute_node(node_id, dag, ready_queue, completed_nodes, stats);
                continue;
            }

            // 3. Try work stealing (if enabled)
            if config.use_work_stealing {
                let stolen = work_stealer.steal_batch(config.steal_batch);
                if !stolen.is_empty() {
                    for task in stolen {
                        stats.record_steal(true);
                        Self::execute_task(task, stats);
                    }
                    continue;
                } else {
                    stats.record_steal(false);
                }
            }

            // 4. No work available, wait
            let guard = ready_queue.lock().unwrap();
            drop(guard);
            // Wait with timeout
            std::thread::sleep(config.idle_timeout);
        }
    }

    /// Try to get a ready DAG node.
    #[inline]
    fn try_ready_queue(ready_queue: &Arc<Mutex<VecDeque<NodeId>>>) -> Option<NodeId> {
        let mut queue = ready_queue.lock().unwrap();
        queue.pop_front()
    }

    /// Execute a task.
    fn execute_task(task: Arc<Task>, stats: &Arc<SchedulerStats>) {
        task.set_state(TaskState::Running);

        let start = std::time::Instant::now();

        // Execute the task
        if let Some(executor) = task.take_executor() {
            executor();
        }

        let duration_us = start.elapsed().as_micros() as usize;
        task.set_state(TaskState::Finished);
        stats.record_completed(duration_us);

        // Update parallelism
        stats.update_parallelism(1);
    }

    /// Execute a DAG node.
    fn execute_node(
        node_id: NodeId,
        dag: &Arc<Mutex<ComputationDAG>>,
        ready_queue: &Arc<Mutex<VecDeque<NodeId>>>,
        completed_nodes: &Arc<Mutex<HashSet<NodeId>>>,
        stats: &Arc<SchedulerStats>,
    ) {
        let node_info = {
            let dag_guard = dag.lock().unwrap();
            dag_guard.get_node(node_id).ok().cloned()
        };

        if let Some(node) = node_info {
            let start = std::time::Instant::now();

            // Execute node (placeholder - actual execution would be handled by VM)
            let _ = node.kind();

            let duration_us = start.elapsed().as_micros() as usize;
            stats.record_completed(duration_us);

            // Mark node as completed
            {
                let mut completed = completed_nodes.lock().unwrap();
                completed.insert(node_id);
            }

            // Check dependents; if all dependencies done, enqueue them
            if !node.dependents().is_empty() {
                let dependents = node.dependents().to_vec();
                let completed_snapshot = completed_nodes.lock().unwrap().clone();
                let dag_guard = dag.lock().unwrap();
                for dependent in dependents {
                    if let Ok(dep_node) = dag_guard.get_node(dependent) {
                        let ready = dep_node
                            .dependencies()
                            .iter()
                            .all(|dep| completed_snapshot.contains(dep));
                        if ready {
                            ready_queue.lock().unwrap().push_back(dependent);
                        }
                    }
                }
            }
        }
    }

    /// Submit a task without dependencies.
    pub fn spawn(&self, task: Arc<Task>) {
        self.stats.record_scheduled();

        // Round-robin to workers for load distribution
        let worker_id =
            self.task_id_generator.lock().unwrap().next().inner() % self.config.num_workers;

        Self::enqueue_task(&self.worker_queues, worker_id, task);

        // Wake up the worker
        self.wake_condvar.notify_one();
    }

    /// Submit a task with a specific worker preference.
    pub fn spawn_on(&self, worker_id: usize, task: Arc<Task>) {
        self.stats.record_scheduled();

        Self::enqueue_task(&self.worker_queues, worker_id, task);

        self.wake_condvar.notify_one();
    }

    /// Enqueue a task onto a specific worker queue.
    fn enqueue_task(
        worker_queues: &Arc<RwLock<Vec<Arc<TaskQueue>>>>,
        worker_id: usize,
        task: Arc<Task>,
    ) {
        let queues = worker_queues.read().unwrap();
        if worker_id < queues.len() {
            queues[worker_id].push(task);
        }
    }

    /// Add an edge between DAG nodes.
    pub fn add_edge(&self, from: NodeId, to: NodeId) -> Result<(), DAGError> {
        let mut dag = self.dag.lock().unwrap();
        let result = dag.add_edge(from, to);

        if result.is_ok() {
            // Node now has a dependency, ensure it is not marked ready prematurely
            let mut queue = self.ready_queue.lock().unwrap();
            queue.retain(|id| *id != to);
        }

        result
    }

    /// Add a node to the DAG and schedule it if ready.
    pub fn schedule_node(&self, kind: DAGNodeKind, dependencies: &[NodeId]) -> NodeId {
        let mut dag = self.dag.lock().unwrap();
        let node_id = dag
            .add_node(kind)
            .expect("Failed to add node to DAG");

        for &dependency in dependencies {
            dag.add_edge(dependency, node_id)
                .expect("Failed to add dependency edge");
        }

        // If there are no dependencies, mark as ready
        if dependencies.is_empty() {
            self.ready_queue.lock().unwrap().push_back(node_id);
        }

        // Wake up workers to process the new node
        self.wake_condvar.notify_all();

        node_id
    }

    /// Add a constant node to the DAG.
    #[inline]
    pub fn add_constant(&self, value: String) -> NodeId {
        self.schedule_node(DAGNodeKind::Constant { value }, &[])
    }

    /// Add a compute node to the DAG.
    #[inline]
    pub fn add_compute(&self, name: String, dependencies: &[NodeId]) -> NodeId {
        self.schedule_node(DAGNodeKind::Compute { name }, dependencies)
    }

    /// Add a parallel block node to the DAG.
    #[inline]
    pub fn add_parallel_block(&self, num_exprs: usize) -> NodeId {
        self.schedule_node(DAGNodeKind::ParallelBlock { num_exprs }, &[])
    }

    /// Add a data parallel node to the DAG.
    #[inline]
    pub fn add_data_parallel(
        &self,
        iterator_id: NodeId,
        body_id: NodeId,
        num_iterations: usize,
    ) -> NodeId {
        self.schedule_node(
            DAGNodeKind::DataParallel {
                iterator_id,
                body_id,
                num_iterations,
            },
            &[iterator_id, body_id],
        )
    }

    /// Get statistics.
    #[inline]
    pub fn stats(&self) -> &Arc<SchedulerStats> {
        &self.stats
    }

    /// Get the number of workers.
    #[inline]
    pub fn num_workers(&self) -> usize {
        self.config.num_workers
    }

    /// Get the computation DAG.
    #[inline]
    pub fn dag(&self) -> &Arc<Mutex<ComputationDAG>> {
        &self.dag
    }

    /// Check if the scheduler is running.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Shutdown the scheduler.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        // Wake up all workers
        self.wake_condvar.notify_all();

        // Wait for workers to finish
        for worker in self.workers.drain(..) {
            worker.join().expect("Worker thread panicked");
        }
    }
}

impl Default for FlowScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for FlowScheduler {
    fn drop(&mut self) {
        if self.is_running() {
            self.shutdown();
        }
    }
}
