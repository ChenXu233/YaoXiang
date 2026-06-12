//! 运行时引擎测试
//!
//! 测试覆盖内容：
//! - LocalRuntime 的任务调度
//! - 依赖关系和 DAG 执行
//! - 任务取消和失败传播
//! - 资源序列化
//! - 协作式时间片

use crate::backends::runtime::engine::{
    sv, LocalRuntime, RuntimeError, RuntimeStats, TaskCancelReason, TaskMeta, TaskOutcome,
    TaskPoll, TaskResult, ResourceKey,
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
    let a = rt.spawn(TaskMeta::default()).unwrap();
    let b = rt
        .spawn(TaskMeta {
            deps: vec![a],
            ..TaskMeta::default()
        })
        .unwrap();
    let c = rt
        .spawn(TaskMeta {
            deps: vec![a],
            ..TaskMeta::default()
        })
        .unwrap();
    let d = rt
        .spawn(TaskMeta {
            deps: vec![b, c],
            ..TaskMeta::default()
        })
        .unwrap();

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> = [
        (a, ok_i32(1)),
        (b, ok_i32(2)),
        (c, ok_i32(3)),
        (d, ok_i32(4)),
    ]
    .into();

    rt.drive_until(Some(d), |id| {
        order.push(id);
        table.get(&id).cloned().unwrap()
    })
    .unwrap();

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
        Some(TaskOutcome::Cancelled(TaskCancelReason::DependencyFailed {
            primary,
            others,
        })) => (*primary, others.clone()),
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
    let t3 = rt
        .spawn(TaskMeta {
            resources: vec![r.clone()],
            ..TaskMeta::default()
        })
        .unwrap();

    let mut order = Vec::new();
    let table: HashMap<TaskId, TaskResult> =
        [(t1, ok_i32(1)), (t2, ok_i32(2)), (t3, ok_i32(3))].into();

    rt.drive_until(Some(t3), |id| {
        order.push(id);
        table.get(&id).cloned().unwrap()
    })
    .unwrap();

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
    let t3 = rt
        .spawn(TaskMeta {
            resources: vec![r.clone()],
            ..TaskMeta::default()
        })
        .unwrap();

    let next = rt.next_ready().unwrap();
    assert_eq!(next, t1);
    rt.mark_running(next).unwrap();

    // Cancel the middle task while the first one is still running.
    rt.cancel(t2).unwrap();
    assert!(!rt.is_complete(t2));

    // If cancellation completed immediately, t3 would become runnable now (violating serialization).
    assert_eq!(rt.next_ready(), None);

    rt.complete(t1, TaskOutcome::Ok(sv(1)), Duration::ZERO)
        .unwrap();

    // Now that the control dep is satisfied, the cancelled task can become complete.
    assert!(rt.is_complete(t2));
    assert!(matches!(
        rt.outcome(t2),
        Some(TaskOutcome::Cancelled(TaskCancelReason::Explicit))
    ));

    let next = rt.next_ready().unwrap();
    assert_eq!(next, t3);
    rt.mark_running(next).unwrap();
    rt.complete(t3, TaskOutcome::Ok(sv(3)), Duration::ZERO)
        .unwrap();

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
    let a = rt
        .spawn(TaskMeta {
            label: Some("a".into()),
            ..TaskMeta::default()
        })
        .unwrap();
    let b = rt
        .spawn(TaskMeta {
            label: Some("b".into()),
            ..TaskMeta::default()
        })
        .unwrap();

    let mut remaining: HashMap<TaskId, usize> = [(a, 3usize), (b, 3usize)].into();
    let mut order = Vec::new();

    rt.drive_until_polled(None, |id, time_slice_enabled| {
        order.push(id);
        let r = remaining.get_mut(&id).unwrap();
        if *r == 0 {
            return TaskPoll::Ready(ok_i32(0));
        }

        if time_slice_enabled {
            *r -= 1;
            if *r == 0 {
                TaskPoll::Ready(ok_i32(1))
            } else {
                TaskPoll::Pending
            }
        } else {
            *r = 0;
            TaskPoll::Ready(ok_i32(1))
        }
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
