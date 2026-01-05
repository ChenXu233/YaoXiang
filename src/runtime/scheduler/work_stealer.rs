//! Work stealing for load balancing across worker threads.
//!
//! This module implements a work-stealing scheduler that allows idle workers
//! to steal tasks from busy workers' local queues.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};
use std::thread;

use super::queue::TaskQueue;
use super::task::Task;

/// Statistics about work stealing operations.
#[derive(Debug, Default)]
pub struct StealStats {
    /// Number of successful steals.
    pub steal_successes: AtomicUsize,
    /// Number of failed steal attempts.
    pub steal_failures: AtomicUsize,
    /// Total number of steal attempts.
    pub total_attempts: AtomicUsize,
    /// Total tasks stolen.
    pub tasks_stolen: AtomicUsize,
}

impl StealStats {
    /// Record a successful steal.
    #[inline]
    pub fn record_success(
        &self,
        count: usize,
    ) {
        self.steal_successes.fetch_add(1, Ordering::SeqCst);
        self.tasks_stolen.fetch_add(count, Ordering::SeqCst);
        self.total_attempts.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a failed steal attempt.
    #[inline]
    pub fn record_failure(&self) {
        self.steal_failures.fetch_add(1, Ordering::SeqCst);
        self.total_attempts.fetch_add(1, Ordering::SeqCst);
    }

    /// Get success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        let total = self.total_attempts.load(Ordering::SeqCst);
        if total == 0 {
            return 1.0;
        }
        let successes = self.steal_successes.load(Ordering::SeqCst);
        successes as f64 / total as f64
    }
}

/// Strategy for stealing tasks from other workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealStrategy {
    /// Steal from the back of victim's queue (reduces contention).
    Back,
    /// Steal from the front of victim's queue (FIFO).
    Front,
    /// Randomly choose between front and back.
    Random,
}

/// Simple work stealer with minimal dependencies.
#[derive(Debug)]
pub struct WorkStealer {
    /// All worker queues (for stealing).
    queues: Arc<RwLock<Vec<Arc<TaskQueue>>>>,
    /// Current worker ID (thread-local).
    current_worker: AtomicUsize,
    /// Stealing strategy.
    strategy: StealStrategy,
    /// Statistics.
    stats: Arc<StealStats>,
    /// Random state for victim selection.
    rng_state: AtomicUsize,
}

impl WorkStealer {
    /// Create a new work stealer with the given number of workers.
    #[inline]
    pub fn new(num_workers: usize) -> Self {
        let mut queues = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            queues.push(Arc::new(TaskQueue::new()));
        }

        Self {
            queues: Arc::new(RwLock::new(queues)),
            current_worker: AtomicUsize::new(0),
            strategy: StealStrategy::Back,
            stats: Arc::new(StealStats::default()),
            rng_state: AtomicUsize::new(1),
        }
    }

    /// Get the number of workers.
    #[inline]
    pub fn num_workers(&self) -> usize {
        self.queues.read().unwrap().len()
    }

    /// Register the current thread as a worker with the given ID.
    #[inline]
    pub fn register_worker(
        &self,
        worker_id: usize,
    ) {
        self.current_worker.store(worker_id, Ordering::SeqCst);
    }

    /// Get the current worker ID.
    #[inline]
    pub fn current_worker(&self) -> usize {
        self.current_worker.load(Ordering::SeqCst)
    }

    /// Get the local queue for the current worker.
    #[inline]
    pub fn local_queue(&self) -> Arc<TaskQueue> {
        let worker_id = self.current_worker();
        self.queues.read().unwrap()[worker_id].clone()
    }

    /// Try to get a task from the local queue first.
    #[inline]
    pub fn try_local(&self) -> Option<Arc<Task>> {
        let queue = self.local_queue();
        queue.pop_front()
    }

    /// Steal a single task from a random victim.
    pub fn steal_random(&self) -> Option<Arc<Task>> {
        let num_workers = self.num_workers();
        if num_workers == 0 {
            return None;
        }

        let mut attempts = 0;
        let max_attempts = num_workers.min(8); // Limit attempts to avoid livelock

        while attempts < max_attempts {
            let victim_id = self.random_worker();
            if victim_id == self.current_worker() {
                attempts += 1;
                continue;
            }

            if let Some(task) = self.steal_from(victim_id) {
                self.stats.record_success(1);
                return Some(task);
            }
            attempts += 1;
        }

        self.stats.record_failure();
        None
    }

    /// Steal multiple tasks from random victims (batch stealing).
    pub fn steal_batch(
        &self,
        max_count: usize,
    ) -> Vec<Arc<Task>> {
        let mut stolen = Vec::with_capacity(max_count);
        let num_workers = self.num_workers();

        if num_workers == 0 {
            return stolen;
        }

        let victim_count = (num_workers / 4).max(1).min(num_workers);

        for _ in 0..victim_count {
            if stolen.len() >= max_count {
                break;
            }

            let victim_id = self.random_worker();
            if victim_id == self.current_worker() {
                continue;
            }

            while stolen.len() < max_count {
                if let Some(task) = self.steal_from(victim_id) {
                    stolen.push(task);
                } else {
                    break;
                }
            }
        }

        if !stolen.is_empty() {
            self.stats.record_success(stolen.len());
        } else {
            self.stats.record_failure();
        }

        stolen
    }

    /// Steal from a specific victim's queue.
    fn steal_from(
        &self,
        victim_id: usize,
    ) -> Option<Arc<Task>> {
        let queues = self.queues.read().unwrap();
        if victim_id >= queues.len() {
            return None;
        }

        let queue = &queues[victim_id];
        match self.strategy {
            StealStrategy::Back => queue.pop_back(),
            StealStrategy::Front => queue.pop_front(),
            StealStrategy::Random => {
                if self.next_rand() % 2 == 0 {
                    queue.pop_back()
                } else {
                    queue.pop_front()
                }
            },
        }
    }

    /// Get a random worker ID different from current.
    #[inline]
    fn random_worker(&self) -> usize {
        let num_workers = self.num_workers();
        if num_workers == 0 {
            return 0;
        }

        let mut rng = self.next_rand();
        let mut victim = rng % num_workers;

        // Make sure we don't pick ourselves
        while victim == self.current_worker() && num_workers > 1 {
            rng = rng.wrapping_add(12345);
            victim = rng % num_workers;
        }

        victim
    }

    /// Simple LCG random number generator.
    #[inline]
    fn next_rand(&self) -> usize {
        // LCG parameters (from Numerical Recipes)
        let state = self.rng_state.fetch_add(1, Ordering::SeqCst);
        state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
    }

    /// Get steal statistics.
    #[inline]
    pub fn stats(&self) -> &Arc<StealStats> {
        &self.stats
    }

    /// Set the stealing strategy.
    #[inline]
    pub fn set_strategy(
        &mut self,
        strategy: StealStrategy,
    ) {
        self.strategy = strategy;
    }

    /// Get the inner queues (for testing).
    #[inline]
    pub fn queues(&self) -> &Arc<RwLock<Vec<Arc<TaskQueue>>>> {
        &self.queues
    }
}

impl Default for WorkStealer {
    fn default() -> Self {
        let num_cpus = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(num_cpus)
    }
}
