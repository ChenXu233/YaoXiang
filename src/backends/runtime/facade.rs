//! High-level runtime facade (Embedded / Standard / Full).
//!
//! This layer stays decoupled from interpreter/compiler internals: it schedules
//! generic tasks and returns type-erased (`Any`) payloads.

use std::collections::HashMap;
#[cfg(not(feature = "wasm"))]
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[cfg(not(feature = "wasm"))]
use crossbeam::channel::{Receiver, Sender};

use crate::backends::common::value::TaskId;

use super::engine::{
    sv, LocalRuntime, RuntimeError, RuntimeStats, TaskMeta, TaskOutcome, TaskPoll, TaskResult,
};

/// Runtime mode (three-tier per RFC-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    /// Immediate execution, no DAG scheduling.
    Embedded,
    /// Single-thread DAG scheduling.
    Standard,
    /// Multi-thread execution + (optional) work-stealing.
    Full,
}

/// Runtime configuration.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub mode: RuntimeMode,
    /// Worker count for Standard and Full runtimes.
    pub workers: usize,
    /// Enable work-stealing for Full runtime.
    pub work_stealing: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mode: RuntimeMode::Embedded,
            workers: 1,
            work_stealing: false,
        }
    }
}

/// A generic runnable task for Standard/Full runtimes.
///
/// The `SpawnHandle` parameter allows nested spawning from within tasks.
pub type TaskFn = Box<dyn FnOnce(&SpawnHandle) -> TaskResult + Send + 'static>;

/// A cooperative task that can yield (`Pending`) for fair time-slicing.
///
/// The boolean parameter indicates whether time-slicing is enabled (i.e. there
/// are other runnable tasks).
#[cfg(not(feature = "wasm"))]
pub type CoopTaskFn = Box<dyn FnMut(bool) -> TaskPoll + Send + 'static>;

// ============================================================================
// SpawnHandle — wasm-compatible stub vs full crossbeam version
// ============================================================================

#[cfg(feature = "wasm")]
/// Handle passed to tasks for nested spawning (no-op in wasm).
pub struct SpawnHandle;

#[cfg(feature = "wasm")]
impl SpawnHandle {
    pub fn noop() -> Self {
        SpawnHandle
    }
}

#[cfg(not(feature = "wasm"))]
/// Worker thread message sent back to the main thread.
enum WorkerMessage {
    /// Task completed.
    Completed {
        id: TaskId,
        result: TaskResult,
        exec_time: Duration,
    },
    /// Task requests to spawn a child task (nested spawn).
    SpawnRequest {
        meta: TaskMeta,
        task: TaskFn,
        respond: Sender<TaskId>,
    },
}

#[cfg(not(feature = "wasm"))]
/// Handle passed to tasks for nested spawning.
///
/// Tasks can use this to spawn child tasks that become part of the runtime's DAG.
pub struct SpawnHandle {
    tx: Sender<WorkerMessage>,
}

#[cfg(not(feature = "wasm"))]
impl SpawnHandle {
    pub fn spawn(
        &self,
        meta: TaskMeta,
        task: TaskFn,
    ) -> Result<TaskId, RuntimeError> {
        let (respond_tx, respond_rx) = crossbeam::channel::bounded(1);
        self.tx
            .send(WorkerMessage::SpawnRequest {
                meta,
                task,
                respond: respond_tx,
            })
            .map_err(|_| RuntimeError::DeadlockOrCycle(TaskId(0)))?;
        respond_rx
            .recv()
            .map_err(|_| RuntimeError::DeadlockOrCycle(TaskId(0)))
    }

    pub fn noop() -> Self {
        let (tx, _rx) = crossbeam::channel::unbounded();
        Self { tx }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeFacadeError {
    #[error(transparent)]
    Engine(#[from] RuntimeError),
    #[error("Invalid runtime config: {0}")]
    InvalidConfig(&'static str),
    #[error("Worker pool error: {0}")]
    WorkerPool(String),
}

/// A reusable runtime facade that can be embedded in the interpreter now,
/// and later reused by the compiler/AOT backend.
pub struct Runtime {
    inner: RuntimeInner,
}

enum RuntimeInner {
    Embedded(EmbeddedRuntime),
    #[cfg(not(feature = "wasm"))]
    Standard(StandardRuntime),
    #[cfg(not(feature = "wasm"))]
    Full(FullRuntime),
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Result<Self, RuntimeFacadeError> {
        let inner = match config.mode {
            RuntimeMode::Embedded => RuntimeInner::Embedded(EmbeddedRuntime::default()),
            #[cfg(not(feature = "wasm"))]
            RuntimeMode::Standard => {
                if config.workers == 0 {
                    return Err(RuntimeFacadeError::InvalidConfig("workers must be >= 1"));
                }
                RuntimeInner::Standard(StandardRuntime::new(config.workers)?)
            }
            #[cfg(not(feature = "wasm"))]
            RuntimeMode::Full => {
                if config.workers == 0 {
                    return Err(RuntimeFacadeError::InvalidConfig("workers must be >= 1"));
                }
                RuntimeInner::Full(FullRuntime::new(config.workers, config.work_stealing)?)
            }
            #[cfg(feature = "wasm")]
            _ => {
                return Err(RuntimeFacadeError::InvalidConfig(
                    "only Embedded runtime is supported in wasm",
                ));
            }
        };
        Ok(Self { inner })
    }

    pub fn spawn(
        &mut self,
        meta: TaskMeta,
        task: TaskFn,
    ) -> Result<TaskId, RuntimeFacadeError> {
        match &mut self.inner {
            RuntimeInner::Embedded(rt) => Ok(rt.spawn(meta, task)),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Standard(rt) => Ok(rt.spawn(meta, task)?),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Full(rt) => Ok(rt.spawn(meta, task)?),
        }
    }

    #[cfg(not(feature = "wasm"))]
    pub fn spawn_coop(
        &mut self,
        meta: TaskMeta,
        task: CoopTaskFn,
    ) -> Result<TaskId, RuntimeFacadeError> {
        match &mut self.inner {
            RuntimeInner::Embedded(_) => Err(RuntimeFacadeError::InvalidConfig(
                "embedded runtime does not support cooperative tasks",
            )),
            RuntimeInner::Standard(rt) => Ok(rt.spawn_coop(meta, task)?),
            RuntimeInner::Full(rt) => Ok(rt.spawn_coop(meta, task)?),
        }
    }

    pub fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeFacadeError> {
        match &mut self.inner {
            RuntimeInner::Embedded(rt) => rt.cancel(task_id),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Standard(rt) => rt.cancel(task_id)?,
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Full(rt) => rt.cancel(task_id)?,
        }
        Ok(())
    }

    pub fn outcome(
        &self,
        task_id: TaskId,
    ) -> Option<TaskOutcome> {
        match &self.inner {
            RuntimeInner::Embedded(rt) => rt.outcome(task_id).cloned(),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Standard(rt) => rt.outcome(task_id).cloned(),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Full(rt) => rt.outcome(task_id).cloned(),
        }
    }

    pub fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool {
        match &self.inner {
            RuntimeInner::Embedded(rt) => rt.is_complete(task_id),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Standard(rt) => rt.is_complete(task_id),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Full(rt) => rt.is_complete(task_id),
        }
    }

    pub fn stats(&self) -> RuntimeStats {
        match &self.inner {
            RuntimeInner::Embedded(rt) => rt.stats(),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Standard(rt) => rt.stats(),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Full(rt) => rt.stats(),
        }
    }

    /// Drive until `target` completes; if `None`, drive until no more runnable tasks.
    pub fn drive_until(
        &mut self,
        target: Option<TaskId>,
    ) -> Result<(), RuntimeFacadeError> {
        match &mut self.inner {
            RuntimeInner::Embedded(rt) => rt.drive_until(target),
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Standard(rt) => rt.drive_until(target)?,
            #[cfg(not(feature = "wasm"))]
            RuntimeInner::Full(rt) => rt.drive_until(target)?,
        }
        Ok(())
    }

    pub fn await_task(
        &mut self,
        task_id: TaskId,
    ) -> Result<TaskOutcome, RuntimeFacadeError> {
        self.drive_until(Some(task_id))?;
        self.outcome(task_id).ok_or(RuntimeFacadeError::WorkerPool(
            "task has no outcome".to_string(),
        ))
    }
}

// ============================================================================
// Embedded Runtime
// ============================================================================

#[derive(Debug, Default)]
struct EmbeddedRuntime {
    next_id: usize,
    outcomes: HashMap<TaskId, TaskOutcome>,
    total_exec_time: Duration,
    stats: RuntimeStats,
}

impl EmbeddedRuntime {
    fn spawn(
        &mut self,
        meta: TaskMeta,
        task: TaskFn,
    ) -> TaskId {
        let task_id = TaskId(self.next_id);
        self.next_id += 1;

        // Embedded runtime: no DAG. Dependencies are not supported here.
        // Hosts should select Standard/Full if they need deps/resources.
        if !meta.deps.is_empty() || !meta.resources.is_empty() {
            self.outcomes.insert(
                task_id,
                TaskOutcome::Err(sv("embedded runtime does not support deps/resources")),
            );
            self.stats.failed_count += 1;
            self.stats.total_spawned += 1;
            return task_id;
        }

        let start = Instant::now();
        let result = task(&SpawnHandle::noop());
        let exec_time = start.elapsed();
        self.total_exec_time += exec_time;

        let outcome = match result {
            Ok(v) => TaskOutcome::Ok(v),
            Err(e) => TaskOutcome::Err(e),
        };
        match outcome {
            TaskOutcome::Ok(_) => self.stats.completed_count += 1,
            TaskOutcome::Err(_) => self.stats.failed_count += 1,
            TaskOutcome::Cancelled(_) => self.stats.cancelled_count += 1,
        }
        self.stats.total_spawned += 1;
        let finished =
            self.stats.completed_count + self.stats.failed_count + self.stats.cancelled_count;
        if finished > 0 {
            self.stats.avg_execution_time = self.total_exec_time / (finished as u32);
        }

        self.outcomes.insert(task_id, outcome);
        task_id
    }

    fn cancel(
        &mut self,
        _task_id: TaskId,
    ) {
        // Embedded tasks run immediately; cancellation is a no-op for now.
    }

    fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool {
        self.outcomes.contains_key(&task_id)
    }

    fn outcome(
        &self,
        task_id: TaskId,
    ) -> Option<&TaskOutcome> {
        self.outcomes.get(&task_id)
    }

    fn stats(&self) -> RuntimeStats {
        self.stats.clone()
    }

    fn drive_until(
        &mut self,
        _target: Option<TaskId>,
    ) {
        // Embedded runtime executes at spawn time.
    }
}

// ============================================================================
// Standard Runtime (thread pool DAG) — not available in wasm
// ============================================================================

#[cfg(not(feature = "wasm"))]
/// A work item to be processed by a worker thread.
struct WorkItem {
    id: TaskId,
    task: TaskFn,
    spawn_handle: SpawnHandle,
}

#[cfg(not(feature = "wasm"))]
struct StandardRuntime {
    graph: LocalRuntime,
    tasks: HashMap<TaskId, TaskFn>,
    coop_tasks: HashMap<TaskId, CoopTaskFn>,
    work_tx: Sender<WorkItem>,
    msg_tx: Sender<WorkerMessage>,
    msg_rx: Receiver<WorkerMessage>,
    threads: Vec<JoinHandle<()>>,
    workers: usize,
}

#[cfg(not(feature = "wasm"))]
impl StandardRuntime {
    fn new(workers: usize) -> Result<Self, RuntimeFacadeError> {
        let (msg_tx, msg_rx) = crossbeam::channel::unbounded::<WorkerMessage>();
        let (work_tx, threads) = spawn_worker_threads(workers, msg_tx.clone());

        Ok(Self {
            graph: LocalRuntime::new(),
            tasks: HashMap::new(),
            coop_tasks: HashMap::new(),
            work_tx,
            msg_tx,
            msg_rx,
            threads,
            workers,
        })
    }

    fn spawn(
        &mut self,
        meta: TaskMeta,
        task: TaskFn,
    ) -> Result<TaskId, RuntimeError> {
        let id = self.graph.spawn(meta)?;
        if self.graph.is_complete(id) {
            // Pre-cancelled due to failed/cancelled deps.
            return Ok(id);
        }
        self.tasks.insert(id, task);
        Ok(id)
    }

    fn spawn_coop(
        &mut self,
        meta: TaskMeta,
        task: CoopTaskFn,
    ) -> Result<TaskId, RuntimeError> {
        let id = self.graph.spawn(meta)?;
        if self.graph.is_complete(id) {
            // Pre-cancelled due to failed/cancelled deps.
            return Ok(id);
        }
        self.coop_tasks.insert(id, task);
        Ok(id)
    }

    fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        self.graph.cancel(task_id)?;
        self.tasks.remove(&task_id);
        self.coop_tasks.remove(&task_id);
        self.prune_finished_tasks();
        Ok(())
    }

    fn outcome(
        &self,
        task_id: TaskId,
    ) -> Option<&TaskOutcome> {
        self.graph.outcome(task_id)
    }

    fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool {
        self.graph.is_complete(task_id)
    }

    fn stats(&self) -> RuntimeStats {
        self.graph.stats()
    }

    fn drive_until(
        &mut self,
        target: Option<TaskId>,
    ) -> Result<(), RuntimeError> {
        let mut in_flight = 0usize;

        loop {
            if let Some(t) = target {
                if self.graph.is_complete(t) {
                    self.prune_finished_tasks();
                    return Ok(());
                }
            }

            // Dispatch ready tasks to the thread pool.
            while in_flight < self.workers {
                let Some(next) = (match target {
                    Some(t) => self
                        .graph
                        .next_ready_for(t)
                        .or_else(|| self.graph.next_ready()),
                    None => self.graph.next_ready(),
                }) else {
                    break;
                };

                self.graph.mark_running(next)?;

                // Check if it's a cooperative task.
                if let Some(task) = self.coop_tasks.get_mut(&next) {
                    let time_slice_enabled = self.graph.stats().pending_count > 0;
                    let start = Instant::now();
                    let polled = task(time_slice_enabled);
                    let exec_time = start.elapsed();

                    match polled {
                        TaskPoll::Ready(result) => match result {
                            Ok(v) => self.graph.complete(next, TaskOutcome::Ok(v), exec_time)?,
                            Err(e) => self.graph.complete(next, TaskOutcome::Err(e), exec_time)?,
                        },
                        TaskPoll::Pending => self.graph.yield_now(next, exec_time)?,
                    }
                    continue;
                }

                // Regular task: send to thread pool.
                let task = match self.tasks.remove(&next) {
                    Some(t) => t,
                    None => {
                        self.graph.complete(
                            next,
                            TaskOutcome::Err(sv("task payload missing")),
                            Duration::ZERO,
                        )?;
                        continue;
                    }
                };

                let spawn_handle = SpawnHandle {
                    tx: self.msg_tx.clone(),
                };
                self.work_tx
                    .send(WorkItem {
                        id: next,
                        task,
                        spawn_handle,
                    })
                    .map_err(|_| RuntimeError::DeadlockOrCycle(next))?;
                in_flight += 1;
            }

            if in_flight == 0 {
                if let Some(t) = target {
                    if !self.graph.is_complete(t) {
                        return Err(RuntimeError::DeadlockOrCycle(t));
                    }
                }
                self.prune_finished_tasks();
                return Ok(());
            }

            // Receive messages from worker threads.
            let msg = self
                .msg_rx
                .recv()
                .map_err(|_| RuntimeError::DeadlockOrCycle(target.unwrap_or(TaskId(0))))?;

            match msg {
                WorkerMessage::Completed {
                    id,
                    result,
                    exec_time,
                } => {
                    in_flight = in_flight.saturating_sub(1);
                    match result {
                        Ok(v) => self.graph.complete(id, TaskOutcome::Ok(v), exec_time)?,
                        Err(e) => self.graph.complete(id, TaskOutcome::Err(e), exec_time)?,
                    }
                }
                WorkerMessage::SpawnRequest {
                    meta,
                    task,
                    respond,
                } => {
                    let id = self.graph.spawn(meta)?;
                    if self.graph.is_complete(id) {
                        let _ = respond.send(id);
                    } else {
                        self.tasks.insert(id, task);
                        let _ = respond.send(id);
                    }
                    // Continue loop — the new task may be ready for dispatch.
                    // in_flight > 0 at this point (the spawning worker is still
                    // in-flight), so the in_flight == 0 deadlock check won't
                    // fire. The next dispatch iteration will pick up the new task.
                }
            }

            self.prune_finished_tasks();
        }
    }

    fn prune_finished_tasks(&mut self) {
        let finished_once: Vec<TaskId> = self
            .tasks
            .keys()
            .copied()
            .filter(|id| self.graph.is_complete(*id))
            .collect();
        for id in finished_once {
            self.tasks.remove(&id);
        }

        let finished_coop: Vec<TaskId> = self
            .coop_tasks
            .keys()
            .copied()
            .filter(|id| self.graph.is_complete(*id))
            .collect();
        for id in finished_coop {
            self.coop_tasks.remove(&id);
        }
    }
}

#[cfg(not(feature = "wasm"))]
impl Drop for StandardRuntime {
    fn drop(&mut self) {
        // Close the work channel to signal workers to exit.
        let (dummy_tx, _dummy_rx) = crossbeam::channel::unbounded::<WorkItem>();
        let old = std::mem::replace(&mut self.work_tx, dummy_tx);
        drop(old);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

// ============================================================================
// Full Runtime (delegates to StandardRuntime) — not available in wasm
// ============================================================================

#[cfg(not(feature = "wasm"))]
struct FullRuntime {
    standard: StandardRuntime,
    // TODO: WorkStealer for load balancing
}

#[cfg(not(feature = "wasm"))]
impl FullRuntime {
    fn new(
        workers: usize,
        _work_stealing: bool,
    ) -> Result<Self, RuntimeFacadeError> {
        Ok(Self {
            standard: StandardRuntime::new(workers)?,
        })
    }

    fn spawn(
        &mut self,
        meta: TaskMeta,
        task: TaskFn,
    ) -> Result<TaskId, RuntimeError> {
        self.standard.spawn(meta, task)
    }

    fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        self.standard.cancel(task_id)
    }

    fn outcome(
        &self,
        task_id: TaskId,
    ) -> Option<&TaskOutcome> {
        self.standard.outcome(task_id)
    }

    fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool {
        self.standard.is_complete(task_id)
    }

    fn stats(&self) -> RuntimeStats {
        self.standard.stats()
    }

    fn drive_until(
        &mut self,
        target: Option<TaskId>,
    ) -> Result<(), RuntimeError> {
        self.standard.drive_until(target)
    }

    fn spawn_coop(
        &mut self,
        meta: TaskMeta,
        task: CoopTaskFn,
    ) -> Result<TaskId, RuntimeError> {
        self.standard.spawn_coop(meta, task)
    }
}

// ============================================================================
// Worker thread pool — not available in wasm
// ============================================================================

#[cfg(not(feature = "wasm"))]
fn spawn_worker_threads(
    workers: usize,
    msg_tx: Sender<WorkerMessage>,
) -> (Sender<WorkItem>, Vec<JoinHandle<()>>) {
    let (work_tx, work_rx) = crossbeam::channel::unbounded::<WorkItem>();
    let mut threads = Vec::with_capacity(workers);
    for _ in 0..workers {
        let work_rx = work_rx.clone();
        let msg_tx = msg_tx.clone();
        threads.push(std::thread::spawn(move || {
            while let Ok(item) = work_rx.recv() {
                let start = Instant::now();
                let result = (item.task)(&item.spawn_handle);
                let exec_time = start.elapsed();
                if msg_tx
                    .send(WorkerMessage::Completed {
                        id: item.id,
                        result,
                        exec_time,
                    })
                    .is_err()
                {
                    break; // Main thread dropped msg_rx — exit worker.
                }
            }
        }));
    }
    (work_tx, threads)
}
