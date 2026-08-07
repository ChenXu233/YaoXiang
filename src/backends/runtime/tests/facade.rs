//! 运行时门面测试
//!
//! 测试覆盖内容：
//! - Runtime 的创建和配置
//! - 标准运行时的行为
//! - 任务的并行执行
//! - 资源序列化
//! - 协作式时间片

use crate::backends::runtime::engine::{sv, TaskMeta, TaskOutcome, TaskPoll, TaskResult};
use crate::backends::runtime::facade::{Runtime, RuntimeConfig, RuntimeMode};
use crate::backends::common::value::TaskId;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn ok_i32(v: i32) -> TaskResult {
    Ok(sv(v))
}

#[test]
fn standard_runtime_runs_tasks_in_parallel_when_workers_gt_1() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 2,
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
                move |_h| {
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
                move |_h| {
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
fn standard_runtime_serializes_tasks_with_same_resource_key() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 2,
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
                move |_h| {
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
                move |_h| {
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
fn standard_runtime_coop_tasks_time_slice_fairly() {
    let mut rt = Runtime::new(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 1,
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
