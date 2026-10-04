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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn ok_i32(v: i32) -> TaskResult {
    Ok(sv(v))
}

fn err_str(msg: &'static str) -> TaskResult {
    Err(sv(msg))
}

/// 挂一个依赖 deps 的任务，返回它的 id。
fn spawn_depending_on(
    rt: &mut LocalRuntime,
    deps: &[TaskId],
) -> TaskId {
    rt.spawn(TaskMeta {
        deps: deps.to_vec(),
        ..TaskMeta::default()
    })
    .unwrap_or_else(|e| panic!("spawn depending on {deps:?} failed: {e}"))
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

/// 驱动 rt 直到 target，并把真正被调度到的任务按顺序记入 order。
fn drive_recording(
    rt: &mut LocalRuntime,
    target: Option<TaskId>,
    order: &mut Vec<TaskId>,
    table: &HashMap<TaskId, TaskResult>,
) {
    rt.drive_until(target, |id| {
        order.push(id);
        table
            .get(&id)
            .cloned()
            .unwrap_or_else(|| panic!("no canned result for task {id:?}"))
    })
    .unwrap_or_else(|e| panic!("drive_until({target:?}) failed: {e}"));
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
fn diamond_dependency_respects_partial_order() {
    let mut rt = LocalRuntime::new();
    let a = spawn_depending_on(&mut rt, &[]);
    let b = spawn_depending_on(&mut rt, &[a]);
    let c = spawn_depending_on(&mut rt, &[a]);
    let d = spawn_depending_on(&mut rt, &[b, c]);

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> = [
        (a, ok_i32(1)),
        (b, ok_i32(2)),
        (c, ok_i32(3)),
        (d, ok_i32(4)),
    ]
    .into();
    drive_recording(&mut rt, Some(d), &mut order, &table);

    let pos = |id: TaskId| order.iter().position(|x| *x == id).unwrap();
    assert!(pos(a) < pos(b));
    assert!(pos(a) < pos(c));
    assert!(pos(b) < pos(d));
    assert!(pos(c) < pos(d));
}

#[test]
fn island_tasks_do_not_block_main_chain() {
    let mut rt = LocalRuntime::new();
    let a = rt.spawn(TaskMeta::default()).unwrap();
    let b = rt
        .spawn(TaskMeta {
            deps: vec![a],
            ..TaskMeta::default()
        })
        .unwrap();
    let c = rt.spawn(TaskMeta::default()).unwrap();

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> =
        [(a, ok_i32(1)), (b, ok_i32(2)), (c, ok_i32(3))].into();

    rt.drive_until(None, |id| {
        order.push(id);
        table.get(&id).cloned().unwrap()
    })
    .unwrap();

    assert!(rt.is_complete(a));
    assert!(rt.is_complete(b));
    assert!(rt.is_complete(c));
    assert!(order.contains(&c));
}

#[test]
fn drive_until_target_does_not_run_island_tasks() {
    let mut rt = LocalRuntime::new();
    let a = rt.spawn(TaskMeta::default()).unwrap();
    let b = rt
        .spawn(TaskMeta {
            deps: vec![a],
            ..TaskMeta::default()
        })
        .unwrap();
    let c = rt.spawn(TaskMeta::default()).unwrap();

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> =
        [(a, ok_i32(1)), (b, ok_i32(2)), (c, ok_i32(3))].into();

    rt.drive_until(Some(b), |id| {
        order.push(id);
        table.get(&id).cloned().unwrap()
    })
    .unwrap();

    assert_eq!(order, vec![a, b]);
    assert!(rt.is_complete(a));
    assert!(rt.is_complete(b));
    assert!(!rt.is_complete(c));
}

#[test]
fn multiple_failed_deps_are_merged_into_cancel_reason() {
    let mut rt = LocalRuntime::new();
    let a = rt.spawn(TaskMeta::default()).unwrap();
    let b = rt.spawn(TaskMeta::default()).unwrap();
    let c = rt
        .spawn(TaskMeta {
            deps: vec![a, b],
            ..TaskMeta::default()
        })
        .unwrap();

    let table: HashMap<TaskId, TaskResult> =
        [(a, err_str("a")), (b, err_str("b")), (c, ok_i32(0))].into();

    rt.drive_until(None, |id| table.get(&id).cloned().unwrap())
        .unwrap();

    assert!(matches!(rt.outcome(a), Some(TaskOutcome::Err(_))));
    assert!(matches!(rt.outcome(b), Some(TaskOutcome::Err(_))));

    let (primary, others) = match rt.outcome(c) {
        Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed { primary, others })) => {
            (*primary, others.clone())
        }
        other => panic!("unexpected outcome for c: {other:?}"),
    };

    assert_eq!(others.len(), 1);
    let mut all = vec![primary];
    all.extend(others);
    all.sort_by_key(|id| id.0);
    assert_eq!(all, vec![a, b]);
}

#[test]
fn failure_cancels_dependents() {
    let mut rt = LocalRuntime::new();
    let a = rt.spawn(TaskMeta::default()).unwrap();
    let b = rt
        .spawn(TaskMeta {
            deps: vec![a],
            ..TaskMeta::default()
        })
        .unwrap();

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> = [(a, err_str("boom")), (b, ok_i32(2))].into();

    rt.drive_until(Some(b), |id| {
        order.push(id);
        table.get(&id).cloned().unwrap()
    })
    .unwrap();

    assert_eq!(order, vec![a]);
    assert!(matches!(rt.outcome(a), Some(TaskOutcome::Err(_))));
    assert!(matches!(
        rt.outcome(b),
        Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed { primary, .. }))
            if *primary == a
    ));
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
fn resource_keys_serialize_tasks() {
    let mut rt = LocalRuntime::new();
    let r: ResourceKey = "io".into();

    let t1 = spawn_with_resource(&mut rt, &r);
    let t2 = spawn_with_resource(&mut rt, &r);
    let t3 = spawn_with_resource(&mut rt, &r);

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> =
        [(t1, ok_i32(1)), (t2, ok_i32(2)), (t3, ok_i32(3))].into();
    drive_recording(&mut rt, Some(t3), &mut order, &table);

    assert_eq!(order, vec![t1, t2, t3]);
}

#[test]
fn resource_serialization_does_not_propagate_failure() {
    let mut rt = LocalRuntime::new();
    let r: ResourceKey = "io".into();

    let t1 = rt
        .spawn(TaskMeta {
            resources: vec![r.clone()],
            ..TaskMeta::default()
        })
        .unwrap();
    let t2 = rt
        .spawn(TaskMeta {
            resources: vec![r.clone()],
            ..TaskMeta::default()
        })
        .unwrap();

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> = [(t1, err_str("boom")), (t2, ok_i32(2))].into();

    rt.drive_until(Some(t2), |id| {
        order.push(id);
        table.get(&id).cloned().unwrap()
    })
    .unwrap();

    assert_eq!(order, vec![t1, t2]);
    assert!(matches!(rt.outcome(t1), Some(TaskOutcome::Err(_))));
    assert!(matches!(rt.outcome(t2), Some(TaskOutcome::Ok(_))));
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

#[test]
fn single_task_can_finish_in_one_poll_without_slicing_overhead() {
    let mut rt = LocalRuntime::new();
    let a = rt.spawn(TaskMeta::default()).unwrap();

    let polls = AtomicUsize::new(0);
    let mut remaining = 5usize;

    rt.drive_until_polled(None, |id, time_slice_enabled| {
        assert_eq!(id, a);
        assert!(!time_slice_enabled);
        polls.fetch_add(1, Ordering::Relaxed);

        // No competitors: finish in one go.
        assert_eq!(remaining, 5);
        remaining = 0;
        TaskPoll::Ready(ok_i32(1))
    })
    .unwrap();

    assert_eq!(polls.load(Ordering::Relaxed), 1);
    assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
}

#[test]
fn yielded_task_can_be_cancelled_between_slices() {
    let mut rt = LocalRuntime::new();
    let a = rt.spawn(TaskMeta::default()).unwrap();
    let b = rt
        .spawn(TaskMeta {
            deps: vec![a],
            ..TaskMeta::default()
        })
        .unwrap();

    let next = rt.next_ready().unwrap();
    assert_eq!(next, a);
    rt.mark_running(next).unwrap();
    rt.yield_now(next, Duration::ZERO).unwrap();

    rt.cancel(a).unwrap();

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
