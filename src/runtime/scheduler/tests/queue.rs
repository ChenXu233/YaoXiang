//! TaskQueue 单元测试

use crate::runtime::scheduler::{Task, TaskId, TaskPriority, TaskQueue, PriorityTaskQueue};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

#[test]
fn test_task_queue_basic() {
    let queue = TaskQueue::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_task_queue_push_pop() {
    let queue = TaskQueue::new();
    let task = Arc::new(Task::new(TaskId(1), TaskPriority::Normal, 1024, || {}));

    queue.push(task.clone());
    assert_eq!(queue.len(), 1);

    let popped = queue.pop_front().unwrap();
    assert_eq!(popped.id(), task.id());
    assert!(queue.is_empty());
}

#[test]
fn test_task_queue_pop_back() {
    let queue = TaskQueue::new();
    let task1 = Arc::new(Task::new(TaskId(1), TaskPriority::Normal, 1024, || {}));
    let task2 = Arc::new(Task::new(TaskId(2), TaskPriority::Normal, 1024, || {}));

    queue.push(task1.clone());
    queue.push(task2.clone());

    // pop_back should return task2 (last pushed)
    let popped = queue.pop_back().unwrap();
    assert_eq!(popped.id(), TaskId(2));
}

#[test]
fn test_task_queue_clone() {
    let queue = TaskQueue::new();
    let queue2 = queue.clone();

    let task = Arc::new(Task::new(TaskId(1), TaskPriority::Normal, 1024, || {}));
    queue.push(task.clone());

    // Both should see the same task
    assert_eq!(queue.len(), 1);
    assert_eq!(queue2.len(), 1);
}

#[test]
fn test_priority_task_queue() {
    let queue = PriorityTaskQueue::new();

    let low = Arc::new(Task::new(TaskId(1), TaskPriority::Low, 1024, || {}));
    let high = Arc::new(Task::new(TaskId(2), TaskPriority::High, 1024, || {}));
    let normal = Arc::new(Task::new(TaskId(3), TaskPriority::Normal, 1024, || {}));

    // Push in random order
    queue.push(low.clone(), TaskPriority::Low);
    queue.push(high.clone(), TaskPriority::High);
    queue.push(normal.clone(), TaskPriority::Normal);

    // First should be high priority
    let first = queue.pop().unwrap();
    assert_eq!(first.id(), TaskId(2));

    // Then normal
    let second = queue.pop().unwrap();
    assert_eq!(second.id(), TaskId(3));

    // Finally low
    let third = queue.pop().unwrap();
    assert_eq!(third.id(), TaskId(1));
}

#[test]
fn test_task_queue_thread_safety() {
    let queue = Arc::new(TaskQueue::new());
    let counter = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let queue = queue.clone();
            let counter = counter.clone();

            thread::spawn(move || {
                for j in 0..100 {
                    let task =
                        Arc::new(Task::new(TaskId(i * 100 + j), TaskPriority::Normal, 1024, || {}));
                    queue.push(task);
                }
                counter.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 10);
    assert_eq!(queue.len(), 1000);
}
