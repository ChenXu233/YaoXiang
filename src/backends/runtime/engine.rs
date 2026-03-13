//! Decoupled runtime scheduler core (DAG + dependencies + cancellation).
//!
//! This module intentionally does **not** depend on the interpreter or compiler
//! internals. It only manages:
//! - Task lifecycle
//! - Dependency edges (DAG)
//! - Resource serialization (coarse-grained control edges)
//! - Failure propagation and cancellation

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::backends::common::value::TaskId;

/// Shared, thread-safe payload used by the runtime.
pub type SyncValue = Arc<dyn Any + Send + Sync>;

/// Task result used by the runtime.
pub type TaskResult = Result<SyncValue, SyncValue>;

/// Result of polling/running a cooperative task slice.
#[derive(Debug)]
pub enum TaskPoll {
    /// The task finished and produced its final result.
    Ready(TaskResult),
    /// The task yielded; it should be re-queued for later execution.
    Pending,
}

/// Helper to build a [`SyncValue`].
pub fn sv<T: Any + Send + Sync + 'static>(value: T) -> SyncValue {
    Arc::new(value)
}

/// A stable key used to serialize side-effectful operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceKey(Arc<str>);

impl ResourceKey {
    pub fn new(key: impl Into<Arc<str>>) -> Self {
        Self(key.into())
    }
}

impl From<&'static str> for ResourceKey {
    fn from(value: &'static str) -> Self {
        Self::new(Arc::<str>::from(value))
    }
}

/// Per-task metadata owned by the scheduler.
#[derive(Debug, Clone, Default)]
pub struct TaskMeta {
    /// Explicit dependencies. The task won't run until all dependencies finish successfully.
    pub deps: Vec<TaskId>,
    /// Resource keys used by this task. Tasks sharing the same key are serialized.
    pub resources: Vec<ResourceKey>,
    /// Optional label for debugging.
    pub label: Option<Arc<str>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskCancelReason {
    /// Explicit cancellation.
    Explicit,
    /// Cancelled due to failed dependencies.
    DependencyFailed {
        primary: TaskId,
        others: Vec<TaskId>,
    },
    /// Cancelled due to cancelled dependencies.
    DependencyCancelled {
        primary: TaskId,
        others: Vec<TaskId>,
    },
}

#[derive(Debug, Clone)]
pub enum TaskOutcome {
    Ok(SyncValue),
    Err(SyncValue),
    Cancelled(TaskCancelReason),
}

impl TaskOutcome {
    pub fn is_finished(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskStatus {
    Pending,
    Running,
    Finished,
}

#[derive(Debug)]
struct TaskNode {
    meta: TaskMeta,
    deps: Vec<TaskId>,
    dependents: Vec<TaskId>,
    remaining_deps: usize,
    status: TaskStatus,
    outcome: Option<TaskOutcome>,
    started_at: Option<Instant>,
    finished_at: Option<Instant>,
}

impl TaskNode {
    fn is_finished(&self) -> bool {
        matches!(self.status, TaskStatus::Finished)
    }
}

#[derive(Debug, Default, Clone)]
pub struct RuntimeStats {
    pub pending_count: usize,
    pub running_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub cancelled_count: usize,
    pub total_spawned: usize,
    pub avg_execution_time: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Task not found: {0:?}")]
    TaskNotFound(TaskId),
    #[error("Dependency not found: {0:?}")]
    DependencyNotFound(TaskId),
    #[error("Cycle detected when adding dependency: {task:?} depends on {dep:?}")]
    CycleDetected { task: TaskId, dep: TaskId },
    #[error("No ready tasks but target is not finished: {0:?}")]
    DeadlockOrCycle(TaskId),
    #[error("Task already finished: {0:?}")]
    TaskAlreadyFinished(TaskId),
    #[error("Task is not cancellable in its current state: {0:?}")]
    TaskNotCancellable(TaskId),
    #[error("Task is not yieldable in its current state: {0:?}")]
    TaskNotYieldable(TaskId),
}

/// A single-threaded runtime graph.
///
/// The runtime is "decoupled": it does not execute tasks by itself. Instead, it
/// exposes a `drive_*` API that lets the host (e.g. the interpreter) execute
/// ready tasks and then report outcomes back.
#[derive(Debug, Default)]
pub struct LocalRuntime {
    next_id: usize,
    tasks: HashMap<TaskId, TaskNode>,
    ready: VecDeque<TaskId>,
    resource_last: HashMap<ResourceKey, TaskId>,
    total_exec_time: Duration,
    stats: RuntimeStats,
}

impl LocalRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stats(&self) -> RuntimeStats {
        self.stats.clone()
    }

    pub fn is_complete(
        &self,
        task_id: TaskId,
    ) -> bool {
        self.tasks
            .get(&task_id)
            .map(|n| n.is_finished())
            .unwrap_or(false)
    }

    pub fn outcome(
        &self,
        task_id: TaskId,
    ) -> Option<&TaskOutcome> {
        self.tasks.get(&task_id).and_then(|n| n.outcome.as_ref())
    }

    pub fn meta(
        &self,
        task_id: TaskId,
    ) -> Option<&TaskMeta> {
        self.tasks.get(&task_id).map(|n| &n.meta)
    }

    /// Create a task node and return its id.
    pub fn spawn(
        &mut self,
        mut meta: TaskMeta,
    ) -> Result<TaskId, RuntimeError> {
        let task_id = TaskId(self.next_id);
        self.next_id += 1;

        // Resource serialization: add dependency on the last task that used the same resource.
        for key in &meta.resources {
            if let Some(prev) = self.resource_last.get(key) {
                meta.deps.push(*prev);
            }
        }

        // Deduplicate deps (keep order stable).
        let mut deps = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for d in meta.deps.iter().copied() {
            if seen.insert(d) {
                deps.push(d);
            }
        }

        // Validate deps and compute remaining count; handle already-failed deps.
        let mut remaining_deps = 0usize;
        let mut failed_deps = Vec::new();
        let mut cancelled_deps = Vec::new();
        for dep in &deps {
            let Some(dep_node) = self.tasks.get(dep) else {
                return Err(RuntimeError::DependencyNotFound(*dep));
            };
            match dep_node.outcome.as_ref() {
                None => remaining_deps += 1,
                Some(TaskOutcome::Ok(_)) => {}
                Some(TaskOutcome::Err(_)) => failed_deps.push(*dep),
                Some(TaskOutcome::Cancelled(_)) => cancelled_deps.push(*dep),
            }
        }

        let outcome = if !failed_deps.is_empty() {
            Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed {
                primary: failed_deps[0],
                others: failed_deps.iter().copied().skip(1).collect(),
            }))
        } else if !cancelled_deps.is_empty() {
            Some(TaskOutcome::Cancelled(
                TaskCancelReason::DependencyCancelled {
                    primary: cancelled_deps[0],
                    others: cancelled_deps.iter().copied().skip(1).collect(),
                },
            ))
        } else {
            None
        };

        let node = TaskNode {
            meta: meta.clone(),
            deps: deps.clone(),
            dependents: Vec::new(),
            remaining_deps,
            status: if outcome.is_some() {
                TaskStatus::Finished
            } else {
                TaskStatus::Pending
            },
            outcome,
            started_at: None,
            finished_at: None,
        };
        self.tasks.insert(task_id, node);

        // Link dependents.
        for dep in deps {
            if let Some(dep_node) = self.tasks.get_mut(&dep) {
                dep_node.dependents.push(task_id);
            }
        }

        // Update resource "last" pointers.
        for key in meta.resources {
            self.resource_last.insert(key, task_id);
        }

        // Enqueue if ready.
        if let Some(n) = self.tasks.get(&task_id) {
            if matches!(n.status, TaskStatus::Pending) && n.remaining_deps == 0 {
                self.ready.push_back(task_id);
            }
        }

        self.stats.total_spawned += 1;
        self.recompute_counts();
        Ok(task_id)
    }

    /// Add a dependency edge `dep -> task`.
    pub fn add_dependency(
        &mut self,
        task: TaskId,
        dep: TaskId,
    ) -> Result<(), RuntimeError> {
        if task == dep {
            return Err(RuntimeError::CycleDetected { task, dep });
        }
        let Some(_) = self.tasks.get(&task) else {
            return Err(RuntimeError::TaskNotFound(task));
        };
        let Some(_) = self.tasks.get(&dep) else {
            return Err(RuntimeError::DependencyNotFound(dep));
        };

        // Adding dep -> task creates a cycle if `dep` already depends on `task`.
        if self.is_reachable(dep, task) {
            return Err(RuntimeError::CycleDetected { task, dep });
        }

        // Apply the edge.
        let dep_outcome = self.tasks.get(&dep).and_then(|n| n.outcome.clone());
        {
            let task_node = self
                .tasks
                .get_mut(&task)
                .ok_or(RuntimeError::TaskNotFound(task))?;
            if task_node.is_finished() {
                return Err(RuntimeError::TaskAlreadyFinished(task));
            }
            if task_node.deps.iter().any(|d| *d == dep) {
                return Ok(());
            }
            task_node.deps.push(dep);

            match dep_outcome.as_ref() {
                None => {
                    task_node.remaining_deps += 1;
                }
                Some(TaskOutcome::Ok(_)) => {}
                Some(TaskOutcome::Err(_)) => {
                    task_node.outcome =
                        Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed {
                            primary: dep,
                            others: Vec::new(),
                        }));
                    task_node.status = TaskStatus::Finished;
                    task_node.remaining_deps = 0;
                }
                Some(TaskOutcome::Cancelled(_)) => {
                    task_node.outcome = Some(TaskOutcome::Cancelled(
                        TaskCancelReason::DependencyCancelled {
                            primary: dep,
                            others: Vec::new(),
                        },
                    ));
                    task_node.status = TaskStatus::Finished;
                    task_node.remaining_deps = 0;
                }
            }
        }

        // Register dependent.
        if let Some(dep_node) = self.tasks.get_mut(&dep) {
            dep_node.dependents.push(task);
        }

        self.recompute_counts();
        Ok(())
    }

    /// Cancel a task (and all dependents).
    pub fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        let Some(node) = self.tasks.get_mut(&task_id) else {
            return Err(RuntimeError::TaskNotFound(task_id));
        };
        match node.status {
            TaskStatus::Pending => {
                node.outcome = Some(TaskOutcome::Cancelled(TaskCancelReason::Explicit));
                node.status = TaskStatus::Finished;
                node.finished_at = Some(Instant::now());
            }
            TaskStatus::Running => return Err(RuntimeError::TaskNotCancellable(task_id)),
            TaskStatus::Finished => return Ok(()),
        }

        self.propagate_cancel(
            task_id,
            TaskCancelReason::DependencyCancelled {
                primary: task_id,
                others: Vec::new(),
            },
        );
        self.recompute_counts();
        Ok(())
    }

    /// Pop the next runnable task id.
    pub fn next_ready(&mut self) -> Option<TaskId> {
        while let Some(id) = self.ready.pop_front() {
            if let Some(node) = self.tasks.get(&id) {
                if matches!(node.status, TaskStatus::Pending) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Pop the next runnable task id that is required to finish `target`.
    ///
    /// This prioritizes the main dependency chain and prevents unrelated "island"
    /// tasks from delaying `drive_until(Some(target))`.
    pub fn next_ready_for(
        &mut self,
        target: TaskId,
    ) -> Option<TaskId> {
        let mut skipped = Vec::new();
        while let Some(id) = self.ready.pop_front() {
            let Some(node) = self.tasks.get(&id) else {
                continue;
            };
            if !matches!(node.status, TaskStatus::Pending) {
                continue;
            }

            if id == target || self.depends_on(target, id) {
                for s in skipped {
                    self.ready.push_back(s);
                }
                return Some(id);
            }

            skipped.push(id);
        }

        for s in skipped {
            self.ready.push_back(s);
        }
        None
    }

    /// Returns true if `task` transitively depends on `dep`.
    pub fn depends_on(
        &self,
        task: TaskId,
        dep: TaskId,
    ) -> bool {
        if task == dep {
            return true;
        }
        if !self.tasks.contains_key(&task) || !self.tasks.contains_key(&dep) {
            return false;
        }
        self.is_reachable(task, dep)
    }

    pub fn mark_running(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        let Some(node) = self.tasks.get_mut(&task_id) else {
            return Err(RuntimeError::TaskNotFound(task_id));
        };
        if node.is_finished() {
            return Err(RuntimeError::TaskAlreadyFinished(task_id));
        }
        node.status = TaskStatus::Running;
        node.started_at = Some(Instant::now());
        self.recompute_counts();
        Ok(())
    }

    pub fn complete(
        &mut self,
        task_id: TaskId,
        outcome: TaskOutcome,
        exec_time: Duration,
    ) -> Result<(), RuntimeError> {
        let Some(node) = self.tasks.get_mut(&task_id) else {
            return Err(RuntimeError::TaskNotFound(task_id));
        };
        if node.is_finished() {
            return Err(RuntimeError::TaskAlreadyFinished(task_id));
        }
        node.status = TaskStatus::Finished;
        node.outcome = Some(outcome.clone());
        node.finished_at = Some(Instant::now());

        self.total_exec_time += exec_time;

        // Propagate to dependents.
        let dependents = node.dependents.clone();
        match outcome {
            TaskOutcome::Ok(_) => {
                for dep_task in dependents {
                    self.on_dependency_satisfied(dep_task, task_id);
                }
            }
            TaskOutcome::Err(_) => {
                self.propagate_cancel(
                    task_id,
                    TaskCancelReason::DependencyFailed {
                        primary: task_id,
                        others: Vec::new(),
                    },
                );
            }
            TaskOutcome::Cancelled(_) => {
                self.propagate_cancel(
                    task_id,
                    TaskCancelReason::DependencyCancelled {
                        primary: task_id,
                        others: Vec::new(),
                    },
                );
            }
        }

        self.recompute_counts();
        Ok(())
    }

    /// Yield a running task and re-queue it to run again later.
    ///
    /// This is the building block for fairness/time-slicing (RFC-001 A8).
    pub fn yield_now(
        &mut self,
        task_id: TaskId,
        exec_time: Duration,
    ) -> Result<(), RuntimeError> {
        let Some(node) = self.tasks.get_mut(&task_id) else {
            return Err(RuntimeError::TaskNotFound(task_id));
        };
        if node.is_finished() {
            return Err(RuntimeError::TaskAlreadyFinished(task_id));
        }
        if !matches!(node.status, TaskStatus::Running) {
            return Err(RuntimeError::TaskNotYieldable(task_id));
        }

        node.status = TaskStatus::Pending;
        node.started_at = None;
        self.ready.push_back(task_id);

        self.total_exec_time += exec_time;
        self.recompute_counts();
        Ok(())
    }

    /// Drive cooperative tasks until:
    /// - `target` is finished (if provided), or
    /// - there are no more ready tasks (if target is None).
    ///
    /// The executor returns [`TaskPoll::Pending`] to yield the current task slice.
    /// When yielding, the scheduler will re-queue the task behind other runnable tasks,
    /// enabling fair progress for multiple long-running loops.
    pub fn drive_until_polled<F>(
        &mut self,
        target: Option<TaskId>,
        mut poll: F,
    ) -> Result<(), RuntimeError>
    where
        F: FnMut(TaskId, bool) -> TaskPoll,
    {
        loop {
            if let Some(t) = target {
                if self.is_complete(t) {
                    return Ok(());
                }
            } else if self.ready.is_empty() {
                return Ok(());
            }

            let Some(next) = (match target {
                Some(t) => self.next_ready_for(t),
                None => self.next_ready(),
            }) else {
                if let Some(t) = target {
                    return Err(RuntimeError::DeadlockOrCycle(t));
                }
                return Ok(());
            };

            self.mark_running(next)?;

            let time_slice_enabled = !self.ready.is_empty();
            let start = Instant::now();
            let polled = poll(next, time_slice_enabled);
            let exec_time = start.elapsed();

            match polled {
                TaskPoll::Ready(result) => match result {
                    Ok(v) => self.complete(next, TaskOutcome::Ok(v), exec_time)?,
                    Err(e) => self.complete(next, TaskOutcome::Err(e), exec_time)?,
                },
                TaskPoll::Pending => self.yield_now(next, exec_time)?,
            }
        }
    }

    /// Drive the scheduler until:
    /// - `target` is finished (if provided), or
    /// - there are no more ready tasks (if target is None).
    pub fn drive_until<F>(
        &mut self,
        target: Option<TaskId>,
        mut executor: F,
    ) -> Result<(), RuntimeError>
    where
        F: FnMut(TaskId) -> TaskResult,
    {
        loop {
            if let Some(t) = target {
                if self.is_complete(t) {
                    return Ok(());
                }
            } else if self.ready.is_empty() {
                return Ok(());
            }

            let Some(next) = (match target {
                Some(t) => self.next_ready_for(t),
                None => self.next_ready(),
            }) else {
                if let Some(t) = target {
                    return Err(RuntimeError::DeadlockOrCycle(t));
                }
                return Ok(());
            };

            self.mark_running(next)?;
            let start = Instant::now();
            let result = executor(next);
            let exec_time = start.elapsed();

            match result {
                Ok(v) => self.complete(next, TaskOutcome::Ok(v), exec_time)?,
                Err(e) => self.complete(next, TaskOutcome::Err(e), exec_time)?,
            }
        }
    }

    fn on_dependency_satisfied(
        &mut self,
        task: TaskId,
        _dep: TaskId,
    ) {
        let Some(node) = self.tasks.get_mut(&task) else {
            return;
        };
        if node.is_finished() {
            return;
        }
        if node.remaining_deps > 0 {
            node.remaining_deps -= 1;
        }
        if node.remaining_deps == 0 {
            self.ready.push_back(task);
        }
    }

    fn propagate_cancel(
        &mut self,
        dep_task: TaskId,
        reason: TaskCancelReason,
    ) {
        let mut queue = VecDeque::new();
        if let Some(node) = self.tasks.get(&dep_task) {
            for &child in &node.dependents {
                queue.push_back(child);
            }
        }

        while let Some(t) = queue.pop_front() {
            let Some(node) = self.tasks.get_mut(&t) else {
                continue;
            };
            if node.is_finished() {
                // Merge "other" dependency ids for better diagnostics when possible.
                if let Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed {
                    primary,
                    others,
                })) = node.outcome.as_mut()
                {
                    if matches!(reason, TaskCancelReason::DependencyFailed { .. }) {
                        if *primary != dep_task && !others.contains(&dep_task) {
                            others.push(dep_task);
                        }
                    }
                }
                if let Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyCancelled {
                    primary,
                    others,
                })) = node.outcome.as_mut()
                {
                    if matches!(reason, TaskCancelReason::DependencyCancelled { .. }) {
                        if *primary != dep_task && !others.contains(&dep_task) {
                            others.push(dep_task);
                        }
                    }
                }
                continue;
            }

            node.status = TaskStatus::Finished;
            node.outcome = Some(TaskOutcome::Cancelled(reason.clone()));
            node.finished_at = Some(Instant::now());

            // Propagate further.
            for &child in &node.dependents {
                queue.push_back(child);
            }
        }
    }

    fn is_reachable(
        &self,
        start: TaskId,
        target: TaskId,
    ) -> bool {
        let mut stack = vec![start];
        let mut visited = std::collections::HashSet::new();
        while let Some(cur) = stack.pop() {
            if cur == target {
                return true;
            }
            if !visited.insert(cur) {
                continue;
            }
            if let Some(node) = self.tasks.get(&cur) {
                for &d in &node.deps {
                    stack.push(d);
                }
            }
        }
        false
    }

    fn recompute_counts(&mut self) {
        let mut pending = 0usize;
        let mut running = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;
        let mut cancelled = 0usize;

        for node in self.tasks.values() {
            match (node.status, node.outcome.as_ref()) {
                (TaskStatus::Pending, _) => pending += 1,
                (TaskStatus::Running, _) => running += 1,
                (TaskStatus::Finished, Some(TaskOutcome::Ok(_))) => completed += 1,
                (TaskStatus::Finished, Some(TaskOutcome::Err(_))) => failed += 1,
                (TaskStatus::Finished, Some(TaskOutcome::Cancelled(_))) => cancelled += 1,
                (TaskStatus::Finished, None) => {}
            }
        }

        self.stats.pending_count = pending;
        self.stats.running_count = running;
        self.stats.completed_count = completed;
        self.stats.failed_count = failed;
        self.stats.cancelled_count = cancelled;
        self.stats.total_spawned = self.stats.total_spawned.max(self.tasks.len());

        let finished = completed + failed + cancelled;
        if finished > 0 {
            self.stats.avg_execution_time = self.total_exec_time / (finished as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn ok_i32(v: i32) -> TaskResult {
        Ok(sv(v))
    }

    fn err_str(msg: &'static str) -> TaskResult {
        Err(sv(msg))
    }

    #[test]
    fn linear_dependency_executes_in_order() {
        let mut rt = LocalRuntime::new();
        let a = rt
            .spawn(TaskMeta {
                label: Some("a".into()),
                ..TaskMeta::default()
            })
            .unwrap();
        let b = rt
            .spawn(TaskMeta {
                deps: vec![a],
                label: Some("b".into()),
                ..TaskMeta::default()
            })
            .unwrap();

        let mut order = Vec::new();
        let table: HashMap<TaskId, TaskResult> = [(a, ok_i32(1)), (b, ok_i32(2))].into();
        rt.drive_until(Some(b), |id| {
            order.push(id);
            table.get(&id).cloned().unwrap()
        })
        .unwrap();

        assert_eq!(order, vec![a, b]);
        assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
        assert!(matches!(rt.outcome(b), Some(TaskOutcome::Ok(_))));
    }

    #[test]
    fn diamond_dependency_respects_partial_order() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt
            .spawn(TaskMeta {
                deps: vec![a],
                ..TaskMeta::default()
            })
            .unwrap();
        let c = rt
            .spawn(TaskMeta {
                deps: vec![a],
                ..TaskMeta::default()
            })
            .unwrap();
        let d = rt
            .spawn(TaskMeta {
                deps: vec![b, c],
                ..TaskMeta::default()
            })
            .unwrap();

        let mut order = Vec::new();
        let table: HashMap<TaskId, TaskResult> = [
            (a, ok_i32(1)),
            (b, ok_i32(2)),
            (c, ok_i32(3)),
            (d, ok_i32(4)),
        ]
        .into();

        rt.drive_until(Some(d), |id| {
            order.push(id);
            table.get(&id).cloned().unwrap()
        })
        .unwrap();

        let pos = |id: TaskId| order.iter().position(|x| *x == id).unwrap();
        assert!(pos(a) < pos(b));
        assert!(pos(a) < pos(c));
        assert!(pos(b) < pos(d));
        assert!(pos(c) < pos(d));
    }

    #[test]
    fn island_tasks_do_not_block_main_chain() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt
            .spawn(TaskMeta {
                deps: vec![a],
                ..TaskMeta::default()
            })
            .unwrap();
        let c = rt.spawn(TaskMeta::default()).unwrap();

        let mut order = Vec::new();
        let table: HashMap<TaskId, TaskResult> =
            [(a, ok_i32(1)), (b, ok_i32(2)), (c, ok_i32(3))].into();

        rt.drive_until(None, |id| {
            order.push(id);
            table.get(&id).cloned().unwrap()
        })
        .unwrap();

        assert!(rt.is_complete(a));
        assert!(rt.is_complete(b));
        assert!(rt.is_complete(c));
        assert!(order.contains(&c));
    }

    #[test]
    fn drive_until_target_does_not_run_island_tasks() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt
            .spawn(TaskMeta {
                deps: vec![a],
                ..TaskMeta::default()
            })
            .unwrap();
        let c = rt.spawn(TaskMeta::default()).unwrap();

        let mut order = Vec::new();
        let table: HashMap<TaskId, TaskResult> =
            [(a, ok_i32(1)), (b, ok_i32(2)), (c, ok_i32(3))].into();

        rt.drive_until(Some(b), |id| {
            order.push(id);
            table.get(&id).cloned().unwrap()
        })
        .unwrap();

        assert_eq!(order, vec![a, b]);
        assert!(rt.is_complete(a));
        assert!(rt.is_complete(b));
        assert!(!rt.is_complete(c));
    }

    #[test]
    fn multiple_failed_deps_are_merged_into_cancel_reason() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt.spawn(TaskMeta::default()).unwrap();
        let c = rt
            .spawn(TaskMeta {
                deps: vec![a, b],
                ..TaskMeta::default()
            })
            .unwrap();

        let table: HashMap<TaskId, TaskResult> =
            [(a, err_str("a")), (b, err_str("b")), (c, ok_i32(0))].into();

        rt.drive_until(None, |id| table.get(&id).cloned().unwrap())
            .unwrap();

        assert!(matches!(rt.outcome(a), Some(TaskOutcome::Err(_))));
        assert!(matches!(rt.outcome(b), Some(TaskOutcome::Err(_))));

        let (primary, others) = match rt.outcome(c) {
            Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed {
                primary,
                others,
            })) => (*primary, others.clone()),
            other => panic!("unexpected outcome for c: {other:?}"),
        };

        assert_eq!(others.len(), 1);
        let mut all = vec![primary];
        all.extend(others);
        all.sort_by_key(|id| id.0);
        assert_eq!(all, vec![a, b]);
    }

    #[test]
    fn failure_cancels_dependents() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt
            .spawn(TaskMeta {
                deps: vec![a],
                ..TaskMeta::default()
            })
            .unwrap();

        let mut order = Vec::new();
        let table: HashMap<TaskId, TaskResult> = [(a, err_str("boom")), (b, ok_i32(2))].into();

        rt.drive_until(Some(b), |id| {
            order.push(id);
            table.get(&id).cloned().unwrap()
        })
        .unwrap();

        assert_eq!(order, vec![a]);
        assert!(matches!(rt.outcome(a), Some(TaskOutcome::Err(_))));
        assert!(matches!(
            rt.outcome(b),
            Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed { primary, .. }))
                if *primary == a
        ));
    }

    #[test]
    fn explicit_cancel_cancels_dependents() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt
            .spawn(TaskMeta {
                deps: vec![a],
                ..TaskMeta::default()
            })
            .unwrap();

        rt.cancel(a).unwrap();

        let mut ran = Vec::new();
        rt.drive_until(None, |id| {
            ran.push(id);
            ok_i32(0)
        })
        .unwrap();

        assert!(ran.is_empty());
        assert!(matches!(
            rt.outcome(a),
            Some(TaskOutcome::Cancelled(TaskCancelReason::Explicit))
        ));
        assert!(matches!(
            rt.outcome(b),
            Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyCancelled { primary, .. }))
                if *primary == a
        ));
    }

    #[test]
    fn resource_keys_serialize_tasks() {
        let mut rt = LocalRuntime::new();
        let r: ResourceKey = "io".into();
        let t1 = rt
            .spawn(TaskMeta {
                resources: vec![r.clone()],
                ..TaskMeta::default()
            })
            .unwrap();
        let t2 = rt
            .spawn(TaskMeta {
                resources: vec![r.clone()],
                ..TaskMeta::default()
            })
            .unwrap();
        let t3 = rt
            .spawn(TaskMeta {
                resources: vec![r.clone()],
                ..TaskMeta::default()
            })
            .unwrap();

        let mut order = Vec::new();
        let table: HashMap<TaskId, TaskResult> =
            [(t1, ok_i32(1)), (t2, ok_i32(2)), (t3, ok_i32(3))].into();

        rt.drive_until(Some(t3), |id| {
            order.push(id);
            table.get(&id).cloned().unwrap()
        })
        .unwrap();

        assert_eq!(order, vec![t1, t2, t3]);
    }

    #[test]
    fn detects_cycle_when_adding_dependency() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt.spawn(TaskMeta::default()).unwrap();

        rt.add_dependency(a, b).unwrap();
        let err = rt.add_dependency(b, a).unwrap_err();
        assert!(matches!(err, RuntimeError::CycleDetected { .. }));
    }

    #[test]
    fn cooperative_time_slicing_is_fair_for_two_long_tasks() {
        let mut rt = LocalRuntime::new();
        let a = rt
            .spawn(TaskMeta {
                label: Some("a".into()),
                ..TaskMeta::default()
            })
            .unwrap();
        let b = rt
            .spawn(TaskMeta {
                label: Some("b".into()),
                ..TaskMeta::default()
            })
            .unwrap();

        let mut remaining: HashMap<TaskId, usize> = [(a, 3usize), (b, 3usize)].into();
        let mut order = Vec::new();

        rt.drive_until_polled(None, |id, time_slice_enabled| {
            order.push(id);
            let r = remaining.get_mut(&id).unwrap();
            if *r == 0 {
                return TaskPoll::Ready(ok_i32(0));
            }

            if time_slice_enabled {
                *r -= 1;
                if *r == 0 {
                    TaskPoll::Ready(ok_i32(1))
                } else {
                    TaskPoll::Pending
                }
            } else {
                *r = 0;
                TaskPoll::Ready(ok_i32(1))
            }
        })
        .unwrap();

        assert_eq!(order, vec![a, b, a, b, a, b]);
        assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
        assert!(matches!(rt.outcome(b), Some(TaskOutcome::Ok(_))));
    }

    #[test]
    fn single_task_can_finish_in_one_poll_without_slicing_overhead() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();

        let polls = AtomicUsize::new(0);
        let mut remaining = 5usize;

        rt.drive_until_polled(None, |id, time_slice_enabled| {
            assert_eq!(id, a);
            assert!(!time_slice_enabled);
            polls.fetch_add(1, Ordering::Relaxed);

            // No competitors: finish in one go.
            assert_eq!(remaining, 5);
            remaining = 0;
            TaskPoll::Ready(ok_i32(1))
        })
        .unwrap();

        assert_eq!(polls.load(Ordering::Relaxed), 1);
        assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
    }

    #[test]
    fn yielded_task_can_be_cancelled_between_slices() {
        let mut rt = LocalRuntime::new();
        let a = rt.spawn(TaskMeta::default()).unwrap();
        let b = rt
            .spawn(TaskMeta {
                deps: vec![a],
                ..TaskMeta::default()
            })
            .unwrap();

        let next = rt.next_ready().unwrap();
        assert_eq!(next, a);
        rt.mark_running(next).unwrap();
        rt.yield_now(next, Duration::ZERO).unwrap();

        rt.cancel(a).unwrap();

        assert!(matches!(
            rt.outcome(a),
            Some(TaskOutcome::Cancelled(TaskCancelReason::Explicit))
        ));
        assert!(matches!(
            rt.outcome(b),
            Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyCancelled { primary, .. }))
                if *primary == a
        ));
    }
}
