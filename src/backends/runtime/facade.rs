//! High-level runtime facade (Embedded / Standard / Full).
//! High-level runtime facade (Embedded / Standard / Full).
//!
//! This layer stays decoupled from interpreter/compiler internals: it schedules
//! generic tasks and returns type-erased (`Any`) payloads.

use std::collections::HashMap;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam::channel::{Receiver, Sender};

use crate::backends::common::value::TaskId;

use super::engine::{sv, LocalRuntime, RuntimeError, RuntimeStats, TaskMeta, TaskOutcome, TaskResult};

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
    /// Worker count for Full runtime.
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
pub type TaskFn = Box<dyn FnOnce() -> TaskResult + Send + 'static>;

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
    Standard(StandardRuntime),
    Full(FullRuntime),
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Result<Self, RuntimeFacadeError> {
        let inner = match config.mode {
            RuntimeMode::Embedded => RuntimeInner::Embedded(EmbeddedRuntime::default()),
            RuntimeMode::Standard => RuntimeInner::Standard(StandardRuntime::default()),
            RuntimeMode::Full => {
                if config.workers == 0 {
                    return Err(RuntimeFacadeError::InvalidConfig("workers must be >= 1"));
                }
                RuntimeInner::Full(FullRuntime::new(config.workers, config.work_stealing)?)
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
            RuntimeInner::Standard(rt) => Ok(rt.spawn(meta, task)?),
            RuntimeInner::Full(rt) => Ok(rt.spawn(meta, task)?),
        }
    }

    pub fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeFacadeError> {
        match &mut self.inner {
            RuntimeInner::Embedded(rt) => rt.cancel(task_id),
            RuntimeInner::Standard(rt) => rt.cancel(task_id)?,
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
            RuntimeInner::Standard(rt) => rt.outcome(task_id).cloned(),
            RuntimeInner::Full(rt) => rt.outcome(task_id).cloned(),
        }
    }

    pub fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool {
        match &self.inner {
            RuntimeInner::Embedded(rt) => rt.is_complete(task_id),
            RuntimeInner::Standard(rt) => rt.is_complete(task_id),
            RuntimeInner::Full(rt) => rt.is_complete(task_id),
        }
    }

    pub fn stats(&self) -> RuntimeStats {
        match &self.inner {
            RuntimeInner::Embedded(rt) => rt.stats(),
            RuntimeInner::Standard(rt) => rt.stats(),
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
            RuntimeInner::Standard(rt) => rt.drive_until(target)?,
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
        let result = task();
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
// Standard Runtime (single-thread DAG)
// ============================================================================

#[derive(Default)]
struct StandardRuntime {
    graph: LocalRuntime,
    tasks: HashMap<TaskId, TaskFn>,
}

impl StandardRuntime {
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

    fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        self.graph.cancel(task_id)?;
        self.tasks.remove(&task_id);
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
        loop {
            if let Some(t) = target {
                if self.graph.is_complete(t) {
                    self.prune_finished_tasks();
                    return Ok(());
                }
            }

            let Some(next) = self.graph.next_ready() else {
                if let Some(t) = target {
                    if !self.graph.is_complete(t) {
                        return Err(RuntimeError::DeadlockOrCycle(t));
                    }
                }
                self.prune_finished_tasks();
                return Ok(());
            };

            self.graph.mark_running(next)?;
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

            let start = Instant::now();
            let result = task();
            let exec_time = start.elapsed();

            match result {
                Ok(v) => self.graph.complete(next, TaskOutcome::Ok(v), exec_time)?,
                Err(e) => self.graph.complete(next, TaskOutcome::Err(e), exec_time)?,
            }

            self.prune_finished_tasks();
        }
    }

    fn prune_finished_tasks(&mut self) {
        let finished: Vec<TaskId> = self
            .tasks
            .keys()
            .copied()
            .filter(|id| self.graph.is_complete(*id))
            .collect();
        for id in finished {
            self.tasks.remove(&id);
        }
    }
}

// ============================================================================
// Full Runtime (multi-thread)
// ============================================================================

struct FullRuntime {
    graph: LocalRuntime,
    tasks: HashMap<TaskId, TaskFn>,
    work_tx: Sender<WorkItem>,
    done_rx: Receiver<WorkResult>,
    threads: Vec<JoinHandle<()>>,
    workers: usize,
}

struct WorkItem {
    id: TaskId,
    task: TaskFn,
}

type WorkResult = (TaskId, TaskResult, Duration);

impl FullRuntime {
    fn new(
        workers: usize,
        _work_stealing: bool,
    ) -> Result<Self, RuntimeFacadeError> {
        let (done_tx, done_rx) = crossbeam::channel::unbounded::<WorkResult>();
        let (work_tx, work_rx) = crossbeam::channel::unbounded::<WorkItem>();
        let threads = spawn_worker_threads(workers, work_rx, done_tx);

        Ok(Self {
            graph: LocalRuntime::new(),
            tasks: HashMap::new(),
            work_tx,
            done_rx,
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
            return Ok(id);
        }
        self.tasks.insert(id, task);
        Ok(id)
    }

    fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        self.graph.cancel(task_id)?;
        self.tasks.remove(&task_id);
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
    ) -> Result<(), RuntimeFacadeError> {
        let mut in_flight = 0usize;

        loop {
            if let Some(t) = target {
                if self.graph.is_complete(t) {
                    self.prune_finished_tasks();
                    return Ok(());
                }
            }

            while in_flight < self.workers {
                let Some(next) = self.graph.next_ready() else {
                    break;
                };
                self.graph.mark_running(next)?;
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
                self.submit(next, task)?;
                in_flight += 1;
            }

            if in_flight == 0 {
                if let Some(t) = target {
                    if !self.graph.is_complete(t) {
                        return Err(RuntimeError::DeadlockOrCycle(t).into());
                    }
                }
                self.prune_finished_tasks();
                return Ok(());
            }

            let (id, result, exec_time) = self.done_rx.recv().map_err(|e| {
                RuntimeFacadeError::WorkerPool(format!("result channel closed: {e}"))
            })?;
            in_flight = in_flight.saturating_sub(1);

            match result {
                Ok(v) => self.graph.complete(id, TaskOutcome::Ok(v), exec_time)?,
                Err(e) => self.graph.complete(id, TaskOutcome::Err(e), exec_time)?,
            }

            self.prune_finished_tasks();
        }
    }

    fn submit(
        &self,
        id: TaskId,
        task: TaskFn,
    ) -> Result<(), RuntimeFacadeError> {
        self.work_tx
            .send(WorkItem { id, task })
            .map_err(|e| RuntimeFacadeError::WorkerPool(format!("{e}")))
    }

    fn prune_finished_tasks(&mut self) {
        let finished: Vec<TaskId> = self
            .tasks
            .keys()
            .copied()
            .filter(|id| self.graph.is_complete(*id))
            .collect();
        for id in finished {
            self.tasks.remove(&id);
        }
    }
}

impl Drop for FullRuntime {
    fn drop(&mut self) {
        let (dummy_tx, _dummy_rx) = crossbeam::channel::unbounded::<WorkItem>();
        let old = std::mem::replace(&mut self.work_tx, dummy_tx);
        drop(old);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

fn spawn_worker_threads(
    workers: usize,
    work_rx: Receiver<WorkItem>,
    done_tx: Sender<WorkResult>,
) -> Vec<JoinHandle<()>> {
    let mut threads = Vec::with_capacity(workers);
    for _ in 0..workers {
        let work_rx = work_rx.clone();
        let done_tx = done_tx.clone();
        threads.push(std::thread::spawn(move || {
            while let Ok(item) = work_rx.recv() {
                let start = Instant::now();
                let result = (item.task)();
                let exec_time = start.elapsed();
                let _ = done_tx.send((item.id, result, exec_time));
            }
        }));
    }
    threads
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn ok_i32(v: i32) -> TaskResult {
        Ok(sv(v))
    }

    #[test]
    fn standard_and_full_match_for_workers_1() {
        let mut std_rt = Runtime::new(RuntimeConfig {
            mode: RuntimeMode::Standard,
            ..RuntimeConfig::default()
        })
        .unwrap();
        let mut full_rt = Runtime::new(RuntimeConfig {
            mode: RuntimeMode::Full,
            workers: 1,
            work_stealing: false,
        })
        .unwrap();

        let a1 = std_rt
            .spawn(TaskMeta::default(), Box::new(|| ok_i32(1)))
            .unwrap();
        let b1 = std_rt
            .spawn(
                TaskMeta {
                    deps: vec![a1],
                    ..TaskMeta::default()
                },
                Box::new(|| ok_i32(2)),
            )
            .unwrap();

        let a2 = full_rt
            .spawn(TaskMeta::default(), Box::new(|| ok_i32(1)))
            .unwrap();
        let b2 = full_rt
            .spawn(
                TaskMeta {
                    deps: vec![a2],
                    ..TaskMeta::default()
                },
                Box::new(|| ok_i32(2)),
            )
            .unwrap();

        std_rt.drive_until(Some(b1)).unwrap();
        full_rt.drive_until(Some(b2)).unwrap();

        assert!(matches!(std_rt.outcome(b1), Some(TaskOutcome::Ok(_))));
        assert!(matches!(full_rt.outcome(b2), Some(TaskOutcome::Ok(_))));
    }

    #[test]
    fn full_runtime_runs_tasks_in_parallel_when_workers_gt_1() {
        let mut rt = Runtime::new(RuntimeConfig {
            mode: RuntimeMode::Full,
            workers: 2,
            work_stealing: true,
        })
        .unwrap();

        let (started_tx, started_rx) = crossbeam::channel::unbounded::<std::thread::ThreadId>();
        let (cont_tx, cont_rx) = crossbeam::channel::unbounded::<()>();

        let _t1 = rt
            .spawn(
                TaskMeta::default(),
                Box::new({
                    let started_tx = started_tx.clone();
                    let cont_rx = cont_rx.clone();
                    move || {
                        started_tx.send(std::thread::current().id()).unwrap();
                        cont_rx.recv().unwrap();
                        ok_i32(1)
                    }
                }),
            )
            .unwrap();
        let t2 = rt
            .spawn(
                TaskMeta::default(),
                Box::new({
                    let started_tx = started_tx.clone();
                    let cont_rx = cont_rx.clone();
                    move || {
                        started_tx.send(std::thread::current().id()).unwrap();
                        cont_rx.recv().unwrap();
                        ok_i32(2)
                    }
                }),
            )
            .unwrap();

        // Drive in another thread so the test thread can observe starts.
        let handle = std::thread::spawn(move || rt.drive_until(Some(t2)).unwrap());

        let first = match started_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(v) => v,
            Err(e) => {
                let _ = cont_tx.send(());
                let _ = cont_tx.send(());
                let _ = handle.join();
                panic!("failed to observe first task start: {e}");
            }
        };
        let second = match started_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(v) => v,
            Err(e) => {
                let _ = cont_tx.send(());
                let _ = cont_tx.send(());
                let _ = handle.join();
                panic!("failed to observe second task start: {e}");
            }
        };

        // Release both tasks (even if they were started sequentially, this avoids deadlocks).
        cont_tx.send(()).unwrap();
        cont_tx.send(()).unwrap();

        handle.join().unwrap();

        assert_ne!(
            first, second,
            "expected tasks to start on different threads"
        );
    }

    #[test]
    fn work_stealing_toggle_does_not_change_correctness() {
        fn run(work_stealing: bool) -> TaskOutcome {
            let mut rt = Runtime::new(RuntimeConfig {
                mode: RuntimeMode::Full,
                workers: 2,
                work_stealing,
            })
            .unwrap();

            let a = rt
                .spawn(TaskMeta::default(), Box::new(|| ok_i32(1)))
                .unwrap();
            let b = rt
                .spawn(
                    TaskMeta {
                        deps: vec![a],
                        ..TaskMeta::default()
                    },
                    Box::new(|| ok_i32(2)),
                )
                .unwrap();
            rt.await_task(b).unwrap()
        }

        let a = run(false);
        let b = run(true);

        match (a, b) {
            (TaskOutcome::Ok(x), TaskOutcome::Ok(y)) => {
                assert!(Arc::ptr_eq(&x, &y) || x.downcast_ref::<i32>() == y.downcast_ref::<i32>());
            }
            (oa, ob) => panic!("unexpected outcomes: {oa:?} vs {ob:?}"),
        }
    }
}
