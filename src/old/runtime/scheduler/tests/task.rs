//! Task 单元测试

use crate::runtime::scheduler::{Task, TaskBuilder, TaskId, TaskIdGenerator, TaskPriority, TaskState};

#[test]
fn test_task_creation() {
    let task = Task::new(TaskId(1), TaskPriority::Normal, 1024, || {});
    assert_eq!(task.id(), TaskId(1));
    assert_eq!(task.state(), TaskState::Ready);
    assert_eq!(task.priority(), TaskPriority::Normal);
}

#[test]
fn test_task_state_transitions() {
    let task = Task::new(TaskId(1), TaskPriority::Normal, 1024, || {});

    assert_eq!(task.state(), TaskState::Ready);

    task.set_state(TaskState::Running);
    assert_eq!(task.state(), TaskState::Running);

    task.set_state(TaskState::Finished);
    assert_eq!(task.state(), TaskState::Finished);
}

#[test]
fn test_task_executor() {
    let executed = std::sync::Arc::new(std::sync::Mutex::new(false));
    let exec_clone = executed.clone();

    let task = Task::new(TaskId(1), TaskPriority::Normal, 1024, move || {
        *exec_clone.lock().unwrap() = true;
    });

    assert!(task.take_executor().is_some());
    assert!(task.take_executor().is_none()); // Should be None after taking
}

#[test]
fn test_task_priority_ordering() {
    assert!(TaskPriority::Low < TaskPriority::Normal);
    assert!(TaskPriority::Normal < TaskPriority::High);
    assert!(TaskPriority::High < TaskPriority::Critical);
}

#[test]
fn test_task_builder() {
    let task = TaskBuilder::new()
        .name("test_task")
        .priority(TaskPriority::High)
        .stack_size(4096)
        .build(TaskId(42), || {});

    assert_eq!(task.id(), TaskId(42));
    assert_eq!(task.name(), "test_task");
    assert_eq!(task.priority(), TaskPriority::High);
    assert_eq!(task.stack_size(), 4096);
}

#[test]
fn test_task_id_generator() {
    let mut gen = TaskIdGenerator::new();
    assert_eq!(gen.next(), TaskId(0));
    assert_eq!(gen.next(), TaskId(1));
    assert_eq!(gen.next(), TaskId(2));
}

#[test]
fn test_task_state_from_u8() {
    assert_eq!(TaskState::from_u8(0), TaskState::Ready);
    assert_eq!(TaskState::from_u8(1), TaskState::Running);
    assert_eq!(TaskState::from_u8(2), TaskState::Waiting);
    assert_eq!(TaskState::from_u8(3), TaskState::Finished);
    assert_eq!(TaskState::from_u8(4), TaskState::Failed);
    assert_eq!(TaskState::from_u8(5), TaskState::Cancelled);
}
