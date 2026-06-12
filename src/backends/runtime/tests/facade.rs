//! 运行时门面测试
//!
//! 测试覆盖内容：
//! - Runtime 的创建和配置
//! - 标准运行时和完整运行时的行为
//! - 任务的并行执行
//! - 资源序列化
//! - 协作式时间片

use crate::backends::runtime::engine::{sv, TaskMeta, TaskOutcome, TaskPoll, TaskResult};
use crate::backends::runtime::facade::{
    Runtime, RuntimeConfig, RuntimeMode, TaskFn, CoopTaskFn,
};
use crate::backends::common::value::TaskId;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn ok_i32(v: i32) -> TaskResult {
    Ok(sv(v))
}

#[test]
fn standard_and_full_match_for_workers_1() {
    let mut std_rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        ..RuntimeConfig::default()
    })
    .unwrap();
    let mut full_rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Full,
        workers: 1,
        work_stealing: false,
    })
    .unwrap();

    let a1 = std_rt
        .spawn(TaskMeta::default(), Box::new(|| ok_i32(1)))
        .unwrap();
    let b1 = std_rt
        .spawn(
            TaskMeta {
                deps: vec![a1],
                ..TaskMeta::default()
            },
            Box::new(|| ok_i32(2)),
        )
        .unwrap();

    let a2 = full_rt
        .spawn(TaskMeta::default(), Box::new(|| ok_i32(1)))
        .unwrap();
    let b2 = full_rt
        .spawn(
            TaskMeta {
                deps: vec![a2],
                ..TaskMeta::default()
            },
            Box::new(|| ok_i32(2)),
        )
        .unwrap();

    std_rt.drive_until(Some(b1)).unwrap();
    full_rt.drive_until(Some(b2)).unwrap();

    assert!(matches!(std_rt.outcome(b1), Some(TaskOutcome::Ok(_))));
    assert!(matches!(full_rt.outcome(b2), Some(TaskOutcome::Ok(_))));
}

#[test]
fn full_runtime_runs_tasks_in_parallel_when_workers_gt_1() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Full,
        workers: 2,
        work_stealing: true,
    })
    .unwrap();

    let (started_tx, started_rx) = crossbeam::channel::unbounded::<std::thread::ThreadId>();
    let (cont_tx, cont_rx) = crossbeam::channel::unbounded::<()>();

    let _t1 = rt
        .spawn(
            TaskMeta::default(),
            Box::new({
                let started_tx = started_tx.clone();
                let cont_rx = cont_rx.clone();
                move || {
                    started_tx.send(std::thread::current().id()).unwrap();
                    cont_rx.recv().unwrap();
                    ok_i32(1)
                }
            }),
        )
        .unwrap();
    let t2 = rt
        .spawn(
            TaskMeta::default(),
            Box::new({
                let started_tx = started_tx.clone();
                let cont_rx = cont_rx.clone();
                move || {
                    started_tx.send(std::thread::current().id()).unwrap();
                    cont_rx.recv().unwrap();
                    ok_i32(2)
                }
            }),
        )
        .unwrap();

    // Drive in another thread so the test thread can observe starts.
    let handle = std::thread::spawn(move || rt.drive_until(Some(t2)).unwrap());

    let first = match started_rx.recv_timeout(Duration::from_secs(1)) {
        Ok(v) => v,
        Err(e) => {
            let _ = cont_tx.send(());
            let _ = cont_tx.send(());
            let _ = handle.join();
            panic!("failed to observe first task start: {e}");
        }
    };
    let second = match started_rx.recv_timeout(Duration::from_secs(1)) {
        Ok(v) => v,
        Err(e) => {
            let _ = cont_tx.send(());
            let _ = cont_tx.send(());
            let _ = handle.join();
            panic!("failed to observe second task start: {e}");
        }
    };

    // Release both tasks (even if they were started sequentially, this avoids deadlocks).
    cont_tx.send(()).unwrap();
    cont_tx.send(()).unwrap();

    handle.join().unwrap();

    assert_ne!(
        first, second,
        "expected tasks to start on different threads"
    );
}

#[test]
fn full_runtime_serializes_tasks_with_same_resource_key() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Full,
        workers: 2,
        work_stealing: true,
    })
    .unwrap();

    let (started_tx, started_rx) = crossbeam::channel::unbounded::<TaskId>();
    let (cont_tx, cont_rx) = crossbeam::channel::unbounded::<()>();

    let _t1 = rt
        .spawn(
            TaskMeta {
                resources: vec!["io".into()],
                ..TaskMeta::default()
            },
            Box::new({
                let started_tx = started_tx.clone();
                let cont_rx = cont_rx.clone();
                move || {
                    started_tx.send(TaskId(1)).unwrap();
                    cont_rx.recv().unwrap();
                    ok_i32(1)
                }
            }),
        )
        .unwrap();
    let t2 = rt
        .spawn(
            TaskMeta {
                resources: vec!["io".into()],
                ..TaskMeta::default()
            },
            Box::new({
                let started_tx = started_tx.clone();
                let cont_rx = cont_rx.clone();
                move || {
                    started_tx.send(TaskId(2)).unwrap();
                    cont_rx.recv().unwrap();
                    ok_i32(2)
                }
            }),
        )
        .unwrap();

    // Drive in another thread so the test thread can observe starts.
    let handle = std::thread::spawn(move || rt.drive_until(Some(t2)).unwrap());

    let first = match started_rx.recv_timeout(Duration::from_secs(1)) {
        Ok(v) => v,
        Err(e) => {
            let _ = cont_tx.send(());
            let _ = cont_tx.send(());
            let _ = handle.join();
            panic!("failed to observe first task start: {e}");
        }
    };

    // The second task should not start until the first one finishes.
    if started_rx.recv_timeout(Duration::from_millis(100)).is_ok() {
        let _ = cont_tx.send(());
        let _ = cont_tx.send(());
        let _ = handle.join();
        panic!("expected resource serialization to prevent concurrent start");
    }

    cont_tx.send(()).unwrap();

    let second = match started_rx.recv_timeout(Duration::from_secs(1)) {
        Ok(v) => v,
        Err(e) => {
            let _ = cont_tx.send(());
            let _ = handle.join();
            panic!("failed to observe second task start: {e}");
        }
    };

    cont_tx.send(()).unwrap();
    handle.join().unwrap();

    assert_eq!(first, TaskId(1));
    assert_eq!(second, TaskId(2));
}

#[test]
fn work_stealing_toggle_does_not_change_correctness() {
    fn run(work_stealing: bool) -> TaskOutcome {
        let mut rt = Runtime::new(RuntimeConfig {
            mode: RuntimeMode::Full,
            workers: 2,
            work_stealing,
        })
        .unwrap();

        let a = rt
            .spawn(TaskMeta::default(), Box::new(|| ok_i32(1)))
            .unwrap();
        let b = rt
            .spawn(
                TaskMeta {
                    deps: vec![a],
                    ..TaskMeta::default()
                },
                Box::new(|| ok_i32(2)),
            )
            .unwrap();
        rt.await_task(b).unwrap()
    }

    let a = run(false);
    let b = run(true);

    match (a, b) {
        (TaskOutcome::Ok(x), TaskOutcome::Ok(y)) => {
            assert!(Arc::ptr_eq(&x, &y) || x.downcast_ref::<i32>() == y.downcast_ref::<i32>());
        }
        (oa, ob) => panic!("unexpected outcomes: {oa:?} vs {ob:?}"),
    }
}

#[test]
fn standard_runtime_coop_tasks_time_slice_fairly() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 1,
        work_stealing: false,
    })
    .unwrap();

    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let a = rt
        .spawn_coop(
            TaskMeta {
                label: Some("a".into()),
                ..TaskMeta::default()
            },
            Box::new({
                let order = order.clone();
                let mut remaining = 3usize;
                move |time_slice_enabled| {
                    order.lock().unwrap().push("a");
                    if time_slice_enabled {
                        remaining = remaining.saturating_sub(1);
                        if remaining == 0 {
                            TaskPoll::Ready(ok_i32(1))
                        } else {
                            TaskPoll::Pending
                        }
                    } else {
                        remaining = 0;
                        TaskPoll::Ready(ok_i32(1))
                    }
                }
            }),
        )
        .unwrap();

    let b = rt
        .spawn_coop(
            TaskMeta {
                label: Some("b".into()),
                ..TaskMeta::default()
            },
            Box::new({
                let order = order.clone();
                let mut remaining = 3usize;
                move |time_slice_enabled| {
                    order.lock().unwrap().push("b");
                    if time_slice_enabled {
                        remaining = remaining.saturating_sub(1);
                        if remaining == 0 {
                            TaskPoll::Ready(ok_i32(1))
                        } else {
                            TaskPoll::Pending
                        }
                    } else {
                        remaining = 0;
                        TaskPoll::Ready(ok_i32(1))
                    }
                }
            }),
        )
        .unwrap();

    rt.drive_until(None).unwrap();

    assert_eq!(*order.lock().unwrap(), vec!["a", "b", "a", "b", "a", "b"]);
    assert!(matches!(rt.outcome(a), Some(TaskOutcome::Ok(_))));
    assert!(matches!(rt.outcome(b), Some(TaskOutcome::Ok(_))));
}
