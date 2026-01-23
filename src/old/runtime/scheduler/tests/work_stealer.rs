//! WorkStealer 单元测试

use crate::runtime::scheduler::{Task, TaskId, TaskPriority, WorkStealer};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_work_stealer_creation() {
    let stealer = WorkStealer::new(4);
    assert_eq!(stealer.num_workers(), 4);
}

#[test]
fn test_work_stealer_empty() {
    let stealer = WorkStealer::new(4);

    // Should return None when no tasks
    assert!(stealer.try_local().is_none());
    assert!(stealer.steal_random().is_none());
}

#[test]
fn test_work_stealer_local_push_pop() {
    let stealer = WorkStealer::new(4);
    stealer.register_worker(0);

    let task = Arc::new(Task::simple(TaskId(1), TaskPriority::Normal, 1024));
    stealer.local_queue().push(task.clone());

    // Should be able to get it from local
    let popped = stealer.try_local().unwrap();
    assert_eq!(popped.id().inner(), 1);
}

#[test]
fn test_work_stealer_steal_from_other() {
    let stealer = WorkStealer::new(2);
    stealer.register_worker(0);

    // Add task to worker 1's queue
    let task = Arc::new(Task::simple(TaskId(42), TaskPriority::Normal, 1024));
    {
        let queues = stealer.queues().read().unwrap();
        queues[1].push(task);
    }

    // Worker 0 should be able to steal it
    let stolen = stealer.steal_random().unwrap();
    assert_eq!(stolen.id().inner(), 42);
}

#[test]
fn test_work_stealer_batch() {
    let stealer = WorkStealer::new(2);
    stealer.register_worker(0);

    // Add 10 tasks to worker 1's queue
    for i in 0..10 {
        let task = Arc::new(Task::simple(TaskId(i), TaskPriority::Normal, 1024));
        let queues = stealer.queues().read().unwrap();
        queues[1].push(task);
    }

    // Steal 5 tasks
    let batch = stealer.steal_batch(5);
    assert_eq!(batch.len(), 5);
}

#[test]
fn test_work_stealer_concurrent() {
    let stealer = Arc::new(WorkStealer::new(4));
    let barrier = Arc::new(Barrier::new(4));
    let tasks_per_thread = 100;

    let handles: Vec<_> = (0..4)
        .map(|worker_id| {
            let stealer = stealer.clone();
            let barrier = barrier.clone();

            thread::spawn(move || {
                stealer.register_worker(worker_id);
                barrier.wait();

                // Push tasks to local queue
                for i in 0..tasks_per_thread {
                    let task = Arc::new(Task::simple(
                        TaskId(worker_id * 100 + i),
                        TaskPriority::Normal,
                        1024,
                    ));
                    stealer.local_queue().push(task);
                }

                // Try to steal tasks
                let mut stolen = 0;
                while stolen < tasks_per_thread {
                    if stealer.steal_random().is_some() {
                        stolen += 1;
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have some steal activity
    let stats = stealer.stats();
    assert!(stats.total_attempts.load(std::sync::atomic::Ordering::SeqCst) > 0);
}

#[test]
fn test_steal_stats() {
    use std::sync::atomic::Ordering;

    let stealer = WorkStealer::new(2);
    stealer.register_worker(0);

    // Add task to worker 1
    let task = Arc::new(Task::simple(TaskId(1), TaskPriority::Normal, 1024));
    let queues = stealer.queues().read().unwrap();
    queues[1].push(task.clone());

    // Steal it
    let _ = stealer.steal_random();

    let stats = stealer.stats();
    assert_eq!(stats.steal_successes.load(Ordering::SeqCst), 1);
    assert_eq!(stats.tasks_stolen.load(Ordering::SeqCst), 1);

    // Try to steal from empty queue
    let _ = stealer.steal_random();
    assert_eq!(stats.steal_failures.load(Ordering::SeqCst), 1);
}
