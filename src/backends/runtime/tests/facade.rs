//! 运行时门面测试
//!
//! 测试覆盖内容：
//! - Runtime 的创建和配置
//! - 标准运行时的行为
//! - 任务的并行执行
//! - 资源序列化
//! - 协作式时间片

use crate::backends::runtime::engine::{sv, TaskMeta, TaskOutcome, TaskPoll, TaskResult};
use crate::backends::runtime::facade::{CoopTaskFn, Runtime, RuntimeConfig, RuntimeMode, TaskFn};
use crate::backends::common::value::TaskId;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

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

/// 「启动栅栏」：任务启动时上报一个 token，随后阻塞到测试线程放行。
///
/// 用它把「任务确实启动了」「两个任务是否真的并行」变成测试线程可观察的事件，
/// 而不是靠 sleep 猜时序。收尾（release / finish）一律带上下文失败：既要有序
/// 收掉阻塞中的任务与驱动线程，也不允许把驱动线程的失败静默吞掉。
struct StartFence<T> {
    started_tx: crossbeam_channel::Sender<T>,
    started_rx: crossbeam_channel::Receiver<T>,
    release_tx: crossbeam_channel::Sender<()>,
    release_rx: crossbeam_channel::Receiver<()>,
    driver: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> StartFence<T> {
    fn new() -> Self {
        let (started_tx, started_rx) = crossbeam_channel::unbounded();
        let (release_tx, release_rx) = crossbeam_channel::unbounded();
        Self {
            started_tx,
            started_rx,
            release_tx,
            release_rx,
            driver: None,
        }
    }

    /// 构造「上报 token，再阻塞到放行，最后返回 value」的任务体。
    fn task(
        &self,
        token: impl FnOnce() -> T + Send + 'static,
        value: i32,
    ) -> TaskFn {
        let started = self.started_tx.clone();
        let release = self.release_rx.clone();
        Box::new(move |_h| {
            started
                .send(token())
                .unwrap_or_else(|_| panic!("start observer must outlive the task"));
            release
                .recv()
                .unwrap_or_else(|_| panic!("task must be released before it can finish"));
            ok_i32(value)
        })
    }

    /// 在独立线程里驱动 rt 直到 target 完成；测试线程据此观察启动事件。
    fn drive(
        &mut self,
        mut rt: Runtime,
        target: TaskId,
    ) {
        self.driver = Some(std::thread::spawn(move || {
            rt.drive_until(Some(target))
                .unwrap_or_else(|e| panic!("drive_until({target:?}) failed: {e}"));
        }));
    }

    /// 放行 count 个任务。
    ///
    /// 接收端由夹具自身持有（release_rx），每个任务闭包另持一份，所以发送本不该失败；
    /// 真失败就必须带上下文炸出来——静默跳过会让用例在「任务根本没放行」的情况下
    /// 继续往下跑到断言。
    fn release(
        &self,
        count: usize,
    ) {
        for _ in 0..count {
            self.release_tx
                .send(())
                .unwrap_or_else(|e| panic!("release channel closed: {e:?}"));
        }
    }

    /// 回收驱动线程，并把驱动线程里的 panic 一并传播出来。
    ///
    /// 曾经把它写成丢弃返回值的 join：那会吞掉驱动线程的 panic——只要两次启动上报
    /// 都已观察到，用例就会在「驱动其实失败了」的情况下照样变绿。收尾必须承重。
    fn finish(&mut self) {
        if let Some(driver) = self.driver.take() {
            driver
                .join()
                .unwrap_or_else(|e| panic!("driver thread panicked: {e:?}"));
        }
    }

    /// 等一次启动上报；超时就先放行 releases 个任务、回收驱动线程，再 panic。
    fn await_start(
        &mut self,
        label: &str,
        releases: usize,
    ) -> T {
        match self.started_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(token) => token,
            Err(e) => {
                self.release(releases);
                self.finish();
                panic!("failed to observe {label} task start: {e}");
            }
        }
    }

    /// 断言 timeout 内没有任何任务启动；若启动了，收尾后以 message panic。
    fn assert_no_start_within(
        &mut self,
        timeout: Duration,
        releases: usize,
        message: &str,
    ) {
        if self.started_rx.recv_timeout(timeout).is_ok() {
            self.release(releases);
            self.finish();
            panic!("{message}");
        }
    }
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
fn standard_runtime_runs_tasks_in_parallel_when_workers_gt_1() {
    let mut rt = standard_runtime(2);
    let mut fence = StartFence::<std::thread::ThreadId>::new();

    let _t1 = rt
        .spawn(
            TaskMeta::default(),
            fence.task(|| std::thread::current().id(), 1),
        )
        .unwrap();
    let t2 = rt
        .spawn(
            TaskMeta::default(),
            fence.task(|| std::thread::current().id(), 2),
        )
        .unwrap();

    // Drive in another thread so the test thread can observe starts.
    fence.drive(rt, t2);

    let first = fence.await_start("first", 2);
    let second = fence.await_start("second", 2);

    // Release both tasks (even if they were started sequentially, this avoids deadlocks).
    fence.release(2);
    fence.finish();

    assert_ne!(
        first, second,
        "expected tasks to start on different threads"
    );
}

#[test]
fn standard_runtime_serializes_tasks_with_same_resource_key() {
    let mut rt = standard_runtime(2);
    let mut fence = StartFence::<TaskId>::new();
    let io = TaskMeta {
        resources: vec!["io".into()],
        ..TaskMeta::default()
    };

    let _t1 = rt.spawn(io.clone(), fence.task(|| TaskId(1), 1)).unwrap();
    let t2 = rt.spawn(io, fence.task(|| TaskId(2), 2)).unwrap();

    // Drive in another thread so the test thread can observe starts.
    fence.drive(rt, t2);

    let first = fence.await_start("first", 2);

    // The second task should not start until the first one finishes.
    fence.assert_no_start_within(
        Duration::from_millis(100),
        2,
        "expected resource serialization to prevent concurrent start",
    );

    fence.release(1);

    let second = fence.await_start("second", 1);

    fence.release(1);
    fence.finish();

    assert_eq!(first, TaskId(1));
    assert_eq!(second, TaskId(2));
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
