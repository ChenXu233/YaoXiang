//! 运行时引擎测试
//!
//! 测试覆盖内容：
//! - LocalRuntime 的任务调度
//! - 依赖关系和 DAG 执行
//! - 任务取消和失败传播
//! - 资源序列化
//! - 协作式时间片

use crate::backends::runtime::engine::{
    sv, LocalRuntime, RuntimeError, TaskCancelReason, TaskMeta, TaskOutcome, TaskPoll, TaskResult,
    ResourceKey,
};
use crate::backends::common::value::TaskId;
use std::collections::HashMap;
use std::time::Duration;

fn ok_i32(v: i32) -> TaskResult {
    Ok(sv(v))
}

/// 挂一个带 label 的任务，返回它的 id。
fn spawn_labelled(
    rt: &mut LocalRuntime,
    label: &'static str,
) -> TaskId {
    rt.spawn(TaskMeta {
        label: Some(label.into()),
        ..TaskMeta::default()
    })
    .unwrap_or_else(|e| panic!("spawn of {label} failed: {e}"))
}

/// 挂一个占用资源 key 的任务，返回它的 id。
fn spawn_with_resource(
    rt: &mut LocalRuntime,
    key: &ResourceKey,
) -> TaskId {
    rt.spawn(TaskMeta {
        resources: vec![key.clone()],
        ..TaskMeta::default()
    })
    .unwrap_or_else(|e| panic!("spawn with resource {key:?} failed: {e}"))
}

/// 断言 expected 就是下一个就绪任务，并把它置为运行中。
fn run_next_ready(
    rt: &mut LocalRuntime,
    expected: TaskId,
) {
    let next = rt
        .next_ready()
        .unwrap_or_else(|| panic!("expected {expected:?} to be ready, found none"));
    assert_eq!(next, expected);
    rt.mark_running(next)
        .unwrap_or_else(|e| panic!("mark_running({next:?}) failed: {e}"));
}

/// 以零执行时间把 id 收尾，避免把时序引入结果断言。
fn complete_now(
    rt: &mut LocalRuntime,
    id: TaskId,
    outcome: TaskOutcome,
) {
    rt.complete(id, outcome, Duration::ZERO)
        .unwrap_or_else(|e| panic!("complete({id:?}) failed: {e}"));
}

/// 三轮预算的轮询策略：有时间片就逐轮扣减，独占时一次跑完。
fn budgeted_poll(
    remaining: &mut HashMap<TaskId, usize>,
    id: TaskId,
    time_slice_enabled: bool,
) -> TaskPoll {
    let left = remaining
        .get_mut(&id)
        .unwrap_or_else(|| panic!("polled an unknown task {id:?}"));
    if *left == 0 {
        return TaskPoll::Ready(ok_i32(0));
    }
    if time_slice_enabled {
        *left -= 1;
        if *left == 0 {
            TaskPoll::Ready(ok_i32(1))
        } else {
            TaskPoll::Pending
        }
    } else {
        *left = 0;
        TaskPoll::Ready(ok_i32(1))
    }
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
fn cancelled_task_waits_for_control_deps_to_preserve_resource_order() {
    let mut rt = LocalRuntime::new();
    let r: ResourceKey = "io".into();
    let t1 = spawn_with_resource(&mut rt, &r);
    let t2 = spawn_with_resource(&mut rt, &r);
    let t3 = spawn_with_resource(&mut rt, &r);

    run_next_ready(&mut rt, t1);

    // Cancel the middle task while the first one is still running.
    rt.cancel(t2).unwrap();
    assert!(!rt.is_complete(t2));

    // If cancellation completed immediately, t3 would become runnable now (violating serialization).
    assert_eq!(rt.next_ready(), None);

    complete_now(&mut rt, t1, TaskOutcome::Ok(sv(1)));

    // Now that the control dep is satisfied, the cancelled task can become complete.
    assert!(rt.is_complete(t2));
    assert!(matches!(
        rt.outcome(t2),
        Some(TaskOutcome::Cancelled(TaskCancelReason::Explicit))
    ));

    run_next_ready(&mut rt, t3);
    complete_now(&mut rt, t3, TaskOutcome::Ok(sv(3)));

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
    let a = spawn_labelled(&mut rt, "a");
    let b = spawn_labelled(&mut rt, "b");

    let mut remaining: HashMap<TaskId, usize> = [(a, 3usize), (b, 3usize)].into();
    let mut order = Vec::new();

    rt.drive_until_polled(None, |id, time_slice_enabled| {
        order.push(id);
        budgeted_poll(&mut remaining, id, time_slice_enabled)
    })
    .unwrap();

    assert_eq!(order, vec![a, b, a, b, a, b]);
    assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
    assert!(matches!(rt.outcome(b), Some(TaskOutcome::Ok(_))));
}
