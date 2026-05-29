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
    /// Cancelled but waiting for control deps to finish before becoming complete.
    ///
    /// This preserves resource ordering even when cancellation happens before
    /// the task ever starts (or while an earlier task in the same resource chain
    /// is still running).
    Cancelling,
    Finished,
}

#[derive(Debug)]
struct TaskNode {
    meta: TaskMeta,
    /// All dependencies (hard + control), used for cycle/reachability checks.
    deps: Vec<TaskId>,
    /// Hard dependencies: failure/cancellation propagates to this task.
    hard_deps: Vec<TaskId>,
    /// Control dependencies: must complete before running, but do not propagate failures.
    control_deps: Vec<TaskId>,
    /// Dependents that have this task as a hard dependency.
    dependents_hard: Vec<TaskId>,
    /// Dependents that have this task as a control dependency.
    dependents_control: Vec<TaskId>,
    remaining_hard: usize,
    remaining_control: usize,
    status: TaskStatus,
    cancel_pending: Option<TaskCancelReason>,
    outcome: Option<TaskOutcome>,
    started_at: Option<Instant>,
    finished_at: Option<Instant>,
}

impl TaskNode {
    fn is_finished(&self) -> bool {
        matches!(self.status, TaskStatus::Finished)
    }

    fn is_runnable(&self) -> bool {
        matches!(self.status, TaskStatus::Pending)
            && self.remaining_hard == 0
            && self.remaining_control == 0
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
    #[error("Task is not runnable in its current state: {0:?}")]
    TaskNotRunnable(TaskId),
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
        meta: TaskMeta,
    ) -> Result<TaskId, RuntimeError> {
        let task_id = TaskId(self.next_id);
        self.next_id += 1;

        // Hard deps come from explicit metadata (data dependencies).
        let hard_deps = dedup_stable(meta.deps.iter().copied());

        // Control deps come from resource serialization (coarse-grained ordering).
        let mut control_deps = Vec::new();
        for key in &meta.resources {
            if let Some(prev) = self.resource_last.get(key) {
                control_deps.push(*prev);
            }
        }
        // Deduplicate control deps (keep order stable) and avoid duplicating hard deps.
        let hard_set: std::collections::HashSet<TaskId> = hard_deps.iter().copied().collect();
        control_deps.retain(|d| !hard_set.contains(d));
        let control_deps = dedup_stable(control_deps);

        // All deps for reachability/cycle checks.
        let deps = hard_deps
            .iter()
            .copied()
            .chain(control_deps.iter().copied())
            .collect::<Vec<_>>();

        // Validate deps and compute remaining counts; handle already-failed/cancelled hard deps.
        let mut remaining_hard = 0usize;
        let mut remaining_control = 0usize;
        let mut failed_hard = Vec::new();
        let mut cancelled_hard = Vec::new();

        for dep in &hard_deps {
            let Some(dep_node) = self.tasks.get(dep) else {
                return Err(RuntimeError::DependencyNotFound(*dep));
            };
            match dep_node.status {
                TaskStatus::Pending | TaskStatus::Running => remaining_hard += 1,
                TaskStatus::Cancelling => cancelled_hard.push(*dep),
                TaskStatus::Finished => match dep_node.outcome.as_ref() {
                    Some(TaskOutcome::Ok(_)) => {}
                    Some(TaskOutcome::Err(_)) => failed_hard.push(*dep),
                    Some(TaskOutcome::Cancelled(_)) => cancelled_hard.push(*dep),
                    None => remaining_hard += 1,
                },
            }
        }

        for dep in &control_deps {
            let Some(dep_node) = self.tasks.get(dep) else {
                return Err(RuntimeError::DependencyNotFound(*dep));
            };
            if !dep_node.is_finished() {
                remaining_control += 1;
            }
        }

        let cancel_pending = if !failed_hard.is_empty() {
            Some(TaskCancelReason::DependencyFailed {
                primary: failed_hard[0],
                others: failed_hard.iter().copied().skip(1).collect(),
            })
        } else if !cancelled_hard.is_empty() {
            Some(TaskCancelReason::DependencyCancelled {
                primary: cancelled_hard[0],
                others: cancelled_hard.iter().copied().skip(1).collect(),
            })
        } else {
            None
        };

        let node = TaskNode {
            meta: meta.clone(),
            deps: deps.clone(),
            hard_deps: hard_deps.clone(),
            control_deps: control_deps.clone(),
            dependents_hard: Vec::new(),
            dependents_control: Vec::new(),
            remaining_hard,
            remaining_control,
            status: match cancel_pending {
                None => TaskStatus::Pending,
                Some(_) if remaining_control > 0 => TaskStatus::Cancelling,
                Some(_) => TaskStatus::Finished,
            },
            cancel_pending: cancel_pending.clone(),
            outcome: match cancel_pending.clone() {
                None => None,
                Some(_) if remaining_control > 0 => None,
                Some(r) => Some(TaskOutcome::Cancelled(r)),
            },
            started_at: None,
            finished_at: match cancel_pending {
                None => None,
                Some(_) if remaining_control > 0 => None,
                Some(_) => Some(Instant::now()),
            },
        };
        self.tasks.insert(task_id, node);

        // Link dependents.
        for dep in hard_deps {
            if let Some(dep_node) = self.tasks.get_mut(&dep) {
                dep_node.dependents_hard.push(task_id);
            }
        }
        for dep in control_deps {
            if let Some(dep_node) = self.tasks.get_mut(&dep) {
                dep_node.dependents_control.push(task_id);
            }
        }

        // Update resource "last" pointers.
        for key in meta.resources {
            self.resource_last.insert(key, task_id);
        }

        // Enqueue if ready.
        if let Some(n) = self.tasks.get(&task_id) {
            if n.is_runnable() {
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

        let dep_status = self.tasks.get(&dep).map(|n| (n.status, n.outcome.clone()));

        // Apply the hard edge.
        {
            let task_node = self
                .tasks
                .get_mut(&task)
                .ok_or(RuntimeError::TaskNotFound(task))?;
            if matches!(
                task_node.status,
                TaskStatus::Cancelling | TaskStatus::Finished
            ) {
                return Err(RuntimeError::TaskAlreadyFinished(task));
            }
            if task_node.deps.contains(&dep) {
                return Ok(());
            }

            task_node.deps.push(dep);
            task_node.hard_deps.push(dep);

            if let Some((status, outcome)) = dep_status.as_ref() {
                match status {
                    TaskStatus::Pending | TaskStatus::Running => {
                        task_node.remaining_hard += 1;
                    }
                    TaskStatus::Cancelling => {}
                    TaskStatus::Finished => {
                        if outcome.is_none() {
                            task_node.remaining_hard += 1;
                        }
                    }
                }
            }
        }

        // Register hard dependent.
        if let Some(dep_node) = self.tasks.get_mut(&dep) {
            dep_node.dependents_hard.push(task);
        }

        // If the dependency is already failed/cancelled, cancel propagation should be immediate.
        let to_cancel = match dep_status {
            Some((TaskStatus::Cancelling, _)) => Some(TaskCancelReason::DependencyCancelled {
                primary: dep,
                others: Vec::new(),
            }),
            Some((TaskStatus::Finished, Some(TaskOutcome::Err(_)))) => {
                Some(TaskCancelReason::DependencyFailed {
                    primary: dep,
                    others: Vec::new(),
                })
            }
            Some((TaskStatus::Finished, Some(TaskOutcome::Cancelled(_)))) => {
                Some(TaskCancelReason::DependencyCancelled {
                    primary: dep,
                    others: Vec::new(),
                })
            }
            _ => None,
        };

        if let Some(reason) = to_cancel {
            self.propagate_cancel(dep, reason);
        } else if let Some(n) = self.tasks.get(&task) {
            if n.is_runnable() {
                self.ready.push_back(task);
            }
        }

        self.recompute_counts();
        Ok(())
    }

    /// Cancel a task (and all dependents).
    pub fn cancel(
        &mut self,
        task_id: TaskId,
    ) -> Result<(), RuntimeError> {
        let control_to_release = {
            let Some(node) = self.tasks.get_mut(&task_id) else {
                return Err(RuntimeError::TaskNotFound(task_id));
            };
            match node.status {
                TaskStatus::Pending => {
                    let reason = TaskCancelReason::Explicit;
                    if node.remaining_control > 0 {
                        node.status = TaskStatus::Cancelling;
                        node.cancel_pending = Some(reason);
                        node.outcome = None;
                        Vec::new()
                    } else {
                        node.outcome = Some(TaskOutcome::Cancelled(reason));
                        node.status = TaskStatus::Finished;
                        node.cancel_pending = None;
                        node.finished_at = Some(Instant::now());
                        node.dependents_control.clone()
                    }
                }
                TaskStatus::Running => return Err(RuntimeError::TaskNotCancellable(task_id)),
                TaskStatus::Cancelling | TaskStatus::Finished => return Ok(()),
            }
        };

        for t in control_to_release {
            self.on_control_dependency_satisfied(t);
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
                if node.is_runnable() {
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
            if !node.is_runnable() {
                continue;
            }

            if id == target || self.depends_on_for_completion(target, id) {
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
        if !node.is_runnable() {
            return Err(RuntimeError::TaskNotRunnable(task_id));
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
        if !matches!(node.status, TaskStatus::Running) {
            return Err(RuntimeError::TaskAlreadyFinished(task_id));
        }
        node.status = TaskStatus::Finished;
        node.outcome = Some(outcome.clone());
        node.cancel_pending = None;
        node.finished_at = Some(Instant::now());

        self.total_exec_time += exec_time;

        let hard = node.dependents_hard.clone();
        let control = node.dependents_control.clone();
        let _ = node;

        // Control deps are satisfied regardless of success/failure.
        for t in control {
            self.on_control_dependency_satisfied(t);
        }

        // Hard deps: only Ok satisfies; Err/Cancelled cancels dependents.
        match outcome {
            TaskOutcome::Ok(_) => {
                for t in hard {
                    self.on_hard_dependency_satisfied(t);
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

    fn on_hard_dependency_satisfied(
        &mut self,
        task: TaskId,
    ) {
        let Some(node) = self.tasks.get_mut(&task) else {
            return;
        };
        if node.is_finished() {
            return;
        }
        if node.remaining_hard > 0 {
            node.remaining_hard -= 1;
        }
        if node.is_runnable() {
            self.ready.push_back(task);
        }
    }

    fn on_control_dependency_satisfied(
        &mut self,
        task: TaskId,
    ) {
        let Some(node) = self.tasks.get_mut(&task) else {
            return;
        };

        if node.remaining_control > 0 {
            node.remaining_control -= 1;
        }

        match node.status {
            TaskStatus::Pending => {
                if node.is_runnable() {
                    self.ready.push_back(task);
                }
            }
            TaskStatus::Cancelling => {
                if node.remaining_control == 0 {
                    // Finalize cancellation and release its control dependents.
                    let reason = node
                        .cancel_pending
                        .take()
                        .unwrap_or(TaskCancelReason::Explicit);
                    node.status = TaskStatus::Finished;
                    node.outcome = Some(TaskOutcome::Cancelled(reason));
                    node.finished_at = Some(Instant::now());

                    let control = node.dependents_control.clone();
                    let _ = node;
                    for t in control {
                        self.on_control_dependency_satisfied(t);
                    }
                }
            }
            TaskStatus::Running | TaskStatus::Finished => {}
        }
    }

    fn propagate_cancel(
        &mut self,
        dep_task: TaskId,
        reason: TaskCancelReason,
    ) {
        let mut queue = VecDeque::new();
        if let Some(node) = self.tasks.get(&dep_task) {
            for &child in &node.dependents_hard {
                queue.push_back(child);
            }
        }

        while let Some(t) = queue.pop_front() {
            let Some(node) = self.tasks.get_mut(&t) else {
                continue;
            };

            match node.status {
                TaskStatus::Finished => {
                    // Merge "other" dependency ids for better diagnostics when possible.
                    if let Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed {
                        primary,
                        others,
                    })) = node.outcome.as_mut()
                    {
                        if matches!(reason, TaskCancelReason::DependencyFailed { .. })
                            && *primary != dep_task
                            && !others.contains(&dep_task)
                        {
                            others.push(dep_task);
                        }
                    }
                    if let Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyCancelled {
                        primary,
                        others,
                    })) = node.outcome.as_mut()
                    {
                        if matches!(reason, TaskCancelReason::DependencyCancelled { .. })
                            && *primary != dep_task
                            && !others.contains(&dep_task)
                        {
                            others.push(dep_task);
                        }
                    }
                    continue;
                }
                TaskStatus::Cancelling => {
                    // Merge reasons into pending cancellation if possible.
                    if let Some(existing) = node.cancel_pending.as_mut() {
                        if let TaskCancelReason::DependencyFailed { primary, others } = existing {
                            if matches!(reason, TaskCancelReason::DependencyFailed { .. })
                                && *primary != dep_task
                                && !others.contains(&dep_task)
                            {
                                others.push(dep_task);
                            }
                        }
                        if let TaskCancelReason::DependencyCancelled { primary, others } = existing
                        {
                            if matches!(reason, TaskCancelReason::DependencyCancelled { .. })
                                && *primary != dep_task
                                && !others.contains(&dep_task)
                            {
                                others.push(dep_task);
                            }
                        }
                    }
                    continue;
                }
                TaskStatus::Running => {
                    // Should be unreachable for hard deps; keep conservative.
                    continue;
                }
                TaskStatus::Pending => {}
            }

            let hard_children = node.dependents_hard.clone();

            // Cancel pending task (but keep ordering: wait for its control deps if any).
            if node.remaining_control > 0 {
                node.status = TaskStatus::Cancelling;
                node.cancel_pending = Some(reason.clone());
                node.outcome = None;
            } else {
                node.status = TaskStatus::Finished;
                node.outcome = Some(TaskOutcome::Cancelled(reason.clone()));
                node.cancel_pending = None;
                node.finished_at = Some(Instant::now());

                // Control dependents are released when this task becomes complete.
                let control = node.dependents_control.clone();
                let _ = node;
                for t in control {
                    self.on_control_dependency_satisfied(t);
                }
            }

            // Propagate further along hard edges.
            for child in hard_children {
                queue.push_back(child);
            }
        }
    }

    fn depends_on_for_completion(
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

        let mut stack = vec![task];
        let mut visited = std::collections::HashSet::new();
        while let Some(cur) = stack.pop() {
            if cur == dep {
                return true;
            }
            if !visited.insert(cur) {
                continue;
            }
            let Some(node) = self.tasks.get(&cur) else {
                continue;
            };

            match node.status {
                TaskStatus::Finished => {}
                TaskStatus::Cancelling => {
                    for &d in &node.control_deps {
                        stack.push(d);
                    }
                }
                TaskStatus::Pending | TaskStatus::Running => {
                    for &d in &node.deps {
                        stack.push(d);
                    }
                }
            }
        }
        false
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
                (TaskStatus::Cancelling, _) => pending += 1,
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

fn dedup_stable<I: IntoIterator<Item = TaskId>>(iter: I) -> Vec<TaskId> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in iter {
        if seen.insert(id) {
            out.push(id);
        }
    }
    out
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
    fn resource_serialization_does_not_propagate_failure() {
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

        let mut order = Vec::new();
        let table: HashMap<TaskId, TaskResult> = [(t1, err_str("boom")), (t2, ok_i32(2))].into();

        rt.drive_until(Some(t2), |id| {
            order.push(id);
            table.get(&id).cloned().unwrap()
        })
        .unwrap();

        assert_eq!(order, vec![t1, t2]);
        assert!(matches!(rt.outcome(t1), Some(TaskOutcome::Err(_))));
        assert!(matches!(rt.outcome(t2), Some(TaskOutcome::Ok(_))));
    }

    #[test]
    fn cancelled_task_waits_for_control_deps_to_preserve_resource_order() {
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

        let next = rt.next_ready().unwrap();
        assert_eq!(next, t1);
        rt.mark_running(next).unwrap();

        // Cancel the middle task while the first one is still running.
        rt.cancel(t2).unwrap();
        assert!(!rt.is_complete(t2));

        // If cancellation completed immediately, t3 would become runnable now (violating serialization).
        assert_eq!(rt.next_ready(), None);

        rt.complete(t1, TaskOutcome::Ok(sv(1)), Duration::ZERO)
            .unwrap();

        // Now that the control dep is satisfied, the cancelled task can become complete.
        assert!(rt.is_complete(t2));
        assert!(matches!(
            rt.outcome(t2),
            Some(TaskOutcome::Cancelled(TaskCancelReason::Explicit))
        ));

        let next = rt.next_ready().unwrap();
        assert_eq!(next, t3);
        rt.mark_running(next).unwrap();
        rt.complete(t3, TaskOutcome::Ok(sv(3)), Duration::ZERO)
            .unwrap();

        assert!(matches!(rt.outcome(t3), Some(TaskOutcome::Ok(_))));
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
