//! 并发测试（含嵌套 spawn）
//!
//! 测试 Standard 运行时的并发执行、依赖排序和嵌套 spawn 功能。

use std::sync::{Arc, Mutex};

use crate::backends::runtime::engine::{sv, TaskMeta};
use crate::backends::runtime::facade::{Runtime, RuntimeConfig, RuntimeMode, TaskFn};

/// 标准模式运行时夹具（workers 个 worker 线程）。
fn standard_runtime(workers: usize) -> Runtime {
    Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers,
    })
    .unwrap_or_else(|e| panic!("standard runtime with {workers} workers should build: {e}"))
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
