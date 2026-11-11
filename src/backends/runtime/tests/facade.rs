//! 运行时门面测试
//!
//! 测试覆盖内容：
//! - Runtime 的创建和配置
//! - 标准运行时的行为
//! - 任务的并行执行
//! - 资源序列化
//! - 协作式时间片

use crate::backends::runtime::engine::{sv, TaskMeta, TaskOutcome, TaskPoll, TaskResult};
use crate::backends::runtime::facade::{CoopTaskFn, Runtime, RuntimeConfig, RuntimeMode};
use crate::backends::common::value::TaskId;
use std::sync::{Arc, Mutex};

fn ok_i32(v: i32) -> TaskResult {
    Ok(sv(v))
}

/// 标准模式运行时夹具（workers 个 worker 线程）。
fn standard_runtime(workers: usize) -> Runtime {
    Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers,
    })
    .unwrap_or_else(|e| panic!("standard runtime with {workers} workers should build: {e}"))
}

/// 构造「每轮记录一次 tag，第三轮完成」的协作任务体。
///
/// 有时间片时逐轮递减，独占时一次跑完——正是用例要观察的公平性差异。
fn coop_recorder(
    order: Arc<Mutex<Vec<&'static str>>>,
    tag: &'static str,
    value: i32,
) -> CoopTaskFn {
    let mut remaining = 3usize;
    Box::new(move |time_slice_enabled| {
        order
            .lock()
            .unwrap_or_else(|e| panic!("order lock poisoned: {e}"))
            .push(tag);
        if time_slice_enabled {
            remaining = remaining.saturating_sub(1);
            if remaining == 0 {
                TaskPoll::Ready(ok_i32(value))
            } else {
                TaskPoll::Pending
            }
        } else {
            remaining = 0;
            TaskPoll::Ready(ok_i32(value))
        }
    })
}

/// 在 rt 上挂一个带 tag 标签的协作任务，返回它的 id。
fn spawn_coop_recorder(
    rt: &mut Runtime,
    tag: &'static str,
    value: i32,
    order: Arc<Mutex<Vec<&'static str>>>,
) -> TaskId {
    let meta = TaskMeta {
        label: Some(tag.into()),
        ..TaskMeta::default()
    };
    rt.spawn_coop(meta, coop_recorder(order, tag, value))
        .unwrap_or_else(|e| panic!("spawn_coop({tag}) should succeed: {e}"))
}

#[test]
fn standard_runtime_coop_tasks_time_slice_fairly() {
    let mut rt = standard_runtime(1);
    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let a = spawn_coop_recorder(&mut rt, "a", 1, order.clone());
    let b = spawn_coop_recorder(&mut rt, "b", 2, order.clone());

    rt.drive_until(None).unwrap();

    assert_eq!(*order.lock().unwrap(), vec!["a", "b", "a", "b", "a", "b"]);
    assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
    assert!(matches!(rt.outcome(b), Some(TaskOutcome::Ok(_))));
}

#[test]
fn standard_runtime_nested_spawn() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 2,
    })
    .unwrap();

    let a = rt
        .spawn(
            TaskMeta::default(),
            Box::new(|handle| {
                // Nested spawn: spawn a child task from within a task.
                let child_id = handle
                    .spawn(TaskMeta::default(), Box::new(|_h| ok_i32(42)))
                    .unwrap();
                // Note: In a real scenario, the child would be tracked and awaited.
                // For this test, we just verify the spawn succeeds.
                ok_i32(child_id.0 as i32)
            }),
        )
        .unwrap();

    rt.drive_until(Some(a)).unwrap();

    assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
}
