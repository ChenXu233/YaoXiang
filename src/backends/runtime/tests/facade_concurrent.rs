//! 并发测试（含嵌套 spawn）
//!
//! 测试 Standard 运行时的并发执行、依赖排序和嵌套 spawn 功能。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::backends::runtime::engine::{sv, TaskMeta};
use crate::backends::runtime::facade::{Runtime, RuntimeConfig, RuntimeMode, SpawnHandle, TaskFn};

/// 标准模式运行时夹具（workers 个 worker 线程）。
fn standard_runtime(workers: usize) -> Runtime {
    Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers,
    })
    .unwrap_or_else(|e| panic!("standard runtime with {workers} workers should build: {e}"))
}

/// 「睡 50ms 后把 tag 记入 results，再返回 value」的内层任务体。
fn recording_task(
    results: Arc<Mutex<Vec<&'static str>>>,
    tag: &'static str,
    value: &'static str,
) -> TaskFn {
    Box::new(move |_h| {
        std::thread::sleep(Duration::from_millis(50));
        results
            .lock()
            .unwrap_or_else(|e| panic!("results lock poisoned: {e}"))
            .push(tag);
        Ok(sv(value))
    })
}

#[test]
fn standard_runtime_concurrent_execution() {
    let mut rt = standard_runtime(4);

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
    let mut rt = standard_runtime(2);

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
    let mut rt = standard_runtime(4);
    let results = Arc::new(Mutex::new(Vec::new()));

    let outer_results = results.clone();
    let outer_task: TaskFn = Box::new(move |handle: &SpawnHandle| {
        let _id_a = handle
            .spawn(
                TaskMeta::default(),
                recording_task(outer_results.clone(), "inner_a", "a"),
            )
            .unwrap();
        let _id_b = handle
            .spawn(
                TaskMeta::default(),
                recording_task(outer_results.clone(), "inner_b", "b"),
            )
            .unwrap();

        outer_results.lock().unwrap().push("outer");
        Ok(sv("outer_done"))
    });

    let id = rt.spawn(TaskMeta::default(), outer_task).unwrap();
    rt.drive_until(Some(id)).unwrap();

    assert!(rt.is_complete(id));
    let results = results.lock().unwrap();
    assert!(results.contains(&"outer"));
}
