//! 并发测试（含嵌套 spawn）
//!
//! 测试 Standard 运行时的并发执行、依赖排序和嵌套 spawn 功能。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::backends::runtime::engine::{sv, TaskMeta};
use crate::backends::runtime::facade::{Runtime, RuntimeConfig, RuntimeMode, SpawnHandle, TaskFn};

#[test]
fn standard_runtime_concurrent_execution() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 4,
        work_stealing: false,
    })
    .unwrap();

    let order = Arc::new(Mutex::new(Vec::new()));
    let mut task_ids = Vec::new();

    for i in 0..4 {
        let order = order.clone();
        let task: TaskFn = Box::new(move |_h| {
            std::thread::sleep(Duration::from_millis(100));
            order.lock().unwrap().push(i);
            Ok(sv(i))
        });
        let id = rt.spawn(TaskMeta::default(), task).unwrap();
        task_ids.push(id);
    }

    let start = Instant::now();
    rt.drive_until(None).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(350),
        "Expected concurrent execution, took {:?}",
        elapsed
    );

    for id in task_ids {
        assert!(rt.is_complete(id));
    }
}

#[test]
fn standard_runtime_dependency_ordering() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 2,
        work_stealing: false,
    })
    .unwrap();

    let order = Arc::new(Mutex::new(Vec::new()));

    let order_a = order.clone();
    let task_a: TaskFn = Box::new(move |_h| {
        order_a.lock().unwrap().push("A");
        Ok(sv(1))
    });
    let id_a = rt.spawn(TaskMeta::default(), task_a).unwrap();

    let order_b = order.clone();
    let task_b: TaskFn = Box::new(move |_h| {
        order_b.lock().unwrap().push("B");
        Ok(sv(2))
    });
    let meta_b = TaskMeta {
        deps: vec![id_a],
        resources: vec![],
        label: None,
    };
    rt.spawn(meta_b, task_b).unwrap();

    rt.drive_until(None).unwrap();

    let order = order.lock().unwrap();
    assert_eq!(*order, vec!["A", "B"]);
}

#[test]
fn standard_runtime_nested_spawn() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 4,
        work_stealing: false,
    })
    .unwrap();

    let results = Arc::new(Mutex::new(Vec::new()));

    let results_clone = results.clone();
    let outer_task: TaskFn = Box::new(move |handle: &SpawnHandle| {
        let r1 = results_clone.clone();
        let inner_a: TaskFn = Box::new(move |_h| {
            std::thread::sleep(Duration::from_millis(50));
            r1.lock().unwrap().push("inner_a");
            Ok(sv("a"))
        });

        let r2 = results_clone.clone();
        let inner_b: TaskFn = Box::new(move |_h| {
            std::thread::sleep(Duration::from_millis(50));
            r2.lock().unwrap().push("inner_b");
            Ok(sv("b"))
        });

        let _id_a = handle.spawn(TaskMeta::default(), inner_a).unwrap();
        let _id_b = handle.spawn(TaskMeta::default(), inner_b).unwrap();

        results_clone.lock().unwrap().push("outer");
        Ok(sv("outer_done"))
    });

    let id = rt.spawn(TaskMeta::default(), outer_task).unwrap();
    rt.drive_until(Some(id)).unwrap();

    assert!(rt.is_complete(id));
    let results = results.lock().unwrap();
    assert!(results.contains(&"outer"));
}
