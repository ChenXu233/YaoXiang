//! 任务调度测试
//!
//! 测试覆盖内容：
//! - TaskId 的创建和属性
//! - TaskConfig 的配置
//! - TaskContext 的寄存器和栈操作
//! - TaskSpawner 的任务管理

use crate::backends::runtime::task::{
    Task, TaskId, TaskContext, TaskPriority, TaskConfig, TaskSpawner, TaskState, Scheduler,
    SchedulerStats, RuntimeError,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

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
