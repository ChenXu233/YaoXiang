//! FlowScheduler 单元测试

use crate::runtime::scheduler::{FlowScheduler, Scheduler, SchedulerConfig, Task, TaskId, TaskPriority};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

#[test]
fn test_flow_scheduler_creation() {
    let scheduler = FlowScheduler::new();
    assert!(scheduler.is_running());
    assert!(scheduler.num_workers() > 0);
}

#[test]
fn test_flow_scheduler_spawn() {
    let scheduler = Arc::new(FlowScheduler::new());
    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(4));

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let scheduler = scheduler.clone();
            let counter = counter.clone();
            let barrier = barrier.clone();

            thread::spawn(move || {
                barrier.wait();

                let task = Arc::new(Task::new(
                    TaskId(i),
                    TaskPriority::Normal,
                    1024,
                    move || {
                        counter.fetch_add(1, Ordering::SeqCst);
                    },
                ));

                scheduler.spawn(task);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Give workers time to execute
    thread::sleep(Duration::from_millis(100));

    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn test_flow_scheduler_shutdown() {
    let mut scheduler = FlowScheduler::new();
    assert!(scheduler.is_running());

    scheduler.shutdown();
    assert!(!scheduler.is_running());
}

#[test]
fn test_flow_scheduler_dag_integration() {
    use crate::runtime::dag::{ComputationDAG, DAGNodeKind};

    let scheduler = FlowScheduler::new();

    // Add some constants
    let a = scheduler.add_constant("1".to_string());
    let b = scheduler.add_constant("2".to_string());

    // Add compute node that depends on a and b
    let c = scheduler.add_compute("add".to_string(), &[a, b]);

    // Add edge from a to c (redundant, but should work)
    scheduler.add_edge(a, c).ok();

    assert!(scheduler.dag().contains_node(a));
    assert!(scheduler.dag().contains_node(b));
    assert!(scheduler.dag().contains_node(c));
}

#[test]
fn test_flow_scheduler_stats() {
    let scheduler = FlowScheduler::new();

    // Spawn some tasks
    for i in 0..10 {
        let task = Arc::new(Task::new(TaskId(i), TaskPriority::Normal, 1024, || {}));
        scheduler.spawn(task);
    }

    let stats = scheduler.stats();
    assert_eq!(stats.tasks_scheduled.load(Ordering::SeqCst), 10);
}

#[test]
fn test_flow_scheduler_parallel() {
    let scheduler = Arc::new(FlowScheduler::new());
    let barrier = Arc::new(Barrier::new(4));
    let completed = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let scheduler = scheduler.clone();
            let barrier = barrier.clone();
            let completed = completed.clone();

            thread::spawn(move || {
                barrier.wait();

                for i in 0..100 {
                    let task = Arc::new(Task::new(
                        TaskId(i),
                        TaskPriority::Normal,
                        1024,
                        move || {
                            // Simulate some work
                            let _ = (0..1000).fold(0, |acc, x| acc ^ x);
                        },
                    ));
                    scheduler.spawn(task);
                }

                completed.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Wait for completion
    thread::sleep(Duration::from_millis(500));

    assert_eq!(completed.load(Ordering::SeqCst), 4);
}
