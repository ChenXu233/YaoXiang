//! 进程级共享求解器（RFC-039 D58）测试 — WBS 4.4.2
//!
//! 规范来源：
//! - RFC-039 决议登记 D58：SMT 求解器获取面统一为**进程级共享单例** +
//!   `with_shared_solver` 闭包口；三个消费点（predicate / termination /
//!   ownership）全部改走共享口；`default_solver()` 仅保留给测试与独立场景。
//!   禁令：闭包内禁止再次获取（`Mutex` 不可重入，嵌套即死锁）。
//! - WBS 4.4.2 验收：生产路径 `default_solver()` 调用点归零；缓存命中率
//!   跨三消费点可观测（读取口 `shared_solver_thread_consumer_stats` /
//!   `shared_solver_cache_stats`；进程累计口径的
//!   `shared_solver_consumer_stats` 保留为该口径的读取口）。
//! - RFC-027 §8：上层只依赖 `Solver::solve`；缓存计数是可选观测面。
//!
//! 「实测数字」的取法：`cargo test --lib shared -- --nocapture` 打印三消费点
//! 的获取/命中/未命中。这些数字是「三点共享同一实例」的可复核证据——缓存
//! 计数是**实例内**的（见 `z3_backend.rs`），三点若各自新建实例，跨点累计
//! 不可能出现。数字取自**当前线程的观测镜像**（`backend.rs` 的
//! `THREAD_CONSUMER_STATS`，issue #514）：编译线程内「编译前 / 编译后」
//! 差分，恰为本次编译的精确增量——进程累计口径的差分窗口会被并发落账
//! 劈开（全量并发跑间歇红），线程口径天然免于该污染。

use std::sync::mpsc;
use std::time::Duration;

use crate::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTSort};
use crate::frontend::core::typecheck::proof::smt::backend::{
    default_solver, shared_solver_cache_stats, shared_solver_thread_consumer_stats,
    with_shared_solver, ConsumerStats, SolverConsumer,
};

/// 探针超时上限：嵌套获取若退化成死锁，用例须**失败**而不是挂住 `cargo test`。
const PROBE_TIMEOUT: Duration = Duration::from_secs(120);

/// 三消费点的**当前线程**观测镜像读数（按 `SolverConsumer::ALL` 顺序）。
///
/// 镜像只被当前线程写（`backend.rs` 的 `THREAD_CONSUMER_STATS`），线程内
/// 差分不含其他测试线程的落账——这是判据②读数在并发下稳定的前提。
fn thread_consumer_stats_snapshot() -> Vec<ConsumerStats> {
    SolverConsumer::ALL
        .iter()
        .copied()
        .map(shared_solver_thread_consumer_stats)
        .collect()
}

/// 累计增量（后 - 前）。
fn stats_delta(
    before: ConsumerStats,
    after: ConsumerStats,
) -> ConsumerStats {
    ConsumerStats {
        acquisitions: after.acquisitions - before.acquisitions,
        hits: after.hits - before.hits,
        misses: after.misses - before.misses,
    }
}

/// 单次编译：词法 → 语法 → 类型检查（含 termination/ownership/predicate 三消费点）。
///
/// 返回诊断数：探针源码应零诊断（有诊断说明探针形状漂了，读数不可信）。
fn check_source_once(source: &str) -> usize {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;
    use crate::frontend::core::typecheck::checker::TypeChecker;

    let tokens = tokenize(source).expect("探针源码应可词法分析");
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "探针源码应可解析: {:?}", parsed.errors);
    let mut checker = TypeChecker::new("yx44-shared-probe");
    checker.check_module(&parsed.module).diagnostics.len()
}
/// D58 禁令的执行机制：闭包内再次获取共享求解器 → panic（而不是死锁）
///
/// 探针带超时保护：`Mutex` 不可重入，若重入检测缺失，嵌套获取会永久阻塞在
/// `lock()` 上——本用例用 `recv_timeout` 把「挂死」变成「失败」。
#[test]
fn test_shared_solver_nested_acquisition_panics_instead_of_deadlocking() {
    // Arrange — 子线程内做嵌套获取，主线程带超时等待其结论
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_shared_solver(SolverConsumer::Predicate, |_| {
                with_shared_solver(SolverConsumer::Ownership, |_| ())
            })
        }));
        let message = match outcome {
            Ok(()) => String::new(),
            Err(payload) => payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_default(),
        };
        let _ = tx.send(message);
    });

    // Act — 超时保护：挂死 = 失败（不是「等一会儿看看」）
    let message = rx.recv_timeout(PROBE_TIMEOUT).unwrap_or_else(|_| {
        panic!("嵌套获取必须 panic 而非死锁（D58 禁令），{PROBE_TIMEOUT:?} 内未返回")
    });

    // Assert
    assert!(
        message.contains("D58"),
        "重入检测的 panic 消息须点名 D58 禁令，实际: {message}"
    );
}

/// `default_solver()` 的实例与共享实例**隔离**：缓存计数是实例内的
///
/// 这条是判据②的反面对照：独立实例的计数从 0 起、只随自己的求解增长。
/// 若计数是进程级全局量，「跨三点累计」就不再是单例的证据——本用例钉死
/// 计数归属实例，从而让「三点增量可见于共享实例」成为单例的可复核证据。
#[test]
fn test_shared_solver_default_instance_counters_are_instance_internal() {
    // Arrange — 独立实例；Z3 缺失时跳过（同 cache_bound 用例口径）
    let Some(fresh) = default_solver() else {
        return;
    };
    let commands = vec![
        SMTCommand::DeclareConst("yx44_fresh".into(), SMTSort::Int),
        SMTCommand::CheckSat,
    ];

    // Act — 同一查询两次：独立实例自己累计 1 未命中 + 1 命中
    let first = fresh.solve(&commands, 1000);
    let second = fresh.solve(&commands, 1000);

    // Assert
    assert_eq!(
        fresh.cache_stats(),
        Some((1, 1)),
        "独立实例的计数必须从 0 起、只记自己的求解（计数是实例内的）"
    );
    assert_eq!(
        format!("{first:?}"),
        format!("{second:?}"),
        "缓存命中返回的结果必须与真求解一致"
    );
}
/// 触发三消费点的探针源码：同一次编译内三点各自发出**重复查询**。
///
/// 重复是刻意的——缓存是实例内的，单次编译内出现「命中」的唯一途径是同形
/// 查询再发一次；命中因此是「三点共享同一实例」的可观测证据。
///
/// - 精化谓词：`yx44_pos_a` / `yx44_pos_b` 后置条件 `yx44_r >= 0` 由形参
///   前置 `yx44_b >= 0` 蕴含（level 2b，同一查询两条 → 1 miss + 1 hit）
/// - 终止性：`yx44_rec_a` / `yx44_rec_b` 测度义务与良基性同形
///   （`yx44_n - 7` / `yx44_n >= 0` → 2 miss + 2 hit）
/// - 所有权：`yx44_own_a` / `yx44_own_b` 的回边切断查询同形（1 miss + 1 hit）
///
/// 变量名带 `yx44_` 前缀、位移取 `7`：与其他用例的查询不重合——测试并发
/// 共享同一单例，同名同形的查询会互相命中（那是共享的正常表现，但会让
/// 本用例的「未命中」读数不确定）。
const THREE_CONSUMER_SOURCE: &str = "NonNegative: (x: Int) -> Type = { x >= 0 }\n\
Point: Type = { x: Int, y: Int }\n\
yx44_pos_a: (yx44_b: NonNegative(yx44_b)) -> (yx44_r: NonNegative(yx44_r)) = { yx44_b + 7 }\n\
yx44_pos_b: (yx44_b: NonNegative(yx44_b)) -> (yx44_r: NonNegative(yx44_r)) = { yx44_b + 7 }\n\
yx44_rec_a: (yx44_n: Int) -> Terminates(yx44_n) = {\n\
if yx44_n == 0 { return 0 }\n\
return yx44_rec_a(yx44_n - 7) }\n\
yx44_rec_b: (yx44_n: Int) -> Terminates(yx44_n) = {\n\
if yx44_n == 0 { return 0 }\n\
return yx44_rec_b(yx44_n - 7) }\n\
bump: (pt: &mut Point) -> Void = { pt.x = pt.x + 1 }\n\
yx44_own_a: () -> Void = {\n\
p = Point(1, 2)\n\
view = &p\n\
mut i = 0\n\
while i < 11 {\n\
x = view.x\n\
i = i + 1\n\
if i == 11 { bump(p) }\n\
} }\n\
yx44_own_b: () -> Void = {\n\
p = Point(1, 2)\n\
view = &p\n\
mut i = 0\n\
while i < 11 {\n\
x = view.x\n\
i = i + 1\n\
if i == 11 { bump(p) }\n\
} }\n";
/// 同一编译内三消费点全部经共享口，且**各自**产生 hit/miss（WBS 4.4.2 判据②）
///
/// 探针在子线程里跑编译、主线程带超时等待：三点在共享口上串行不嵌套
/// （判据④）——嵌套会自锁，超时即失败，不挂死。
///
/// 读数源（issue #514）：两组读数全部在**编译线程内**、紧贴编译前后取，
/// 经 channel 带回。三点读数取自当前线程的观测镜像（只含本线程落账），
/// 实例读数的差分窗口是编译时长本身——共享实例的计数器单调递增，窗口内
/// 其他线程的贡献只会让实例增量更大，比对方向（`>=`）不受影响。修前以
/// 进程累计口径做差分：三点账的落账发生在实例计数累加之后（`backend.rs`
/// 的 `with_shared_solver` 里 `drop(guard)` 之后），两读数的快照窗口可以把
/// 其他测试的一笔获取劈开（进三点增量、不进实例增量），判据间歇失败——
/// 修前实测 `cargo test --lib typecheck` 全量并发连续 3 轮全红（如
/// 「实例增量 (4, 5) vs 三点之和 (4, 7)」）。
#[test]
fn test_shared_solver_serves_three_consumers_in_one_compilation() {
    // Act — **一次**编译（子线程 + 超时保护）；两组读数同线程紧贴编译前后取
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let before = thread_consumer_stats_snapshot();
        let total_before = shared_solver_cache_stats();
        let diagnostics = check_source_once(THREE_CONSUMER_SOURCE);
        let after = thread_consumer_stats_snapshot();
        let total_after = shared_solver_cache_stats();
        let _ = tx.send((diagnostics, before, total_before, after, total_after));
    });
    let (diagnostics, before, total_before, after, total_after) = rx
        .recv_timeout(PROBE_TIMEOUT)
        .unwrap_or_else(|_| {
            panic!(
                "三消费点须在一次编译内串行完成（D58：闭包内禁止再次获取），{PROBE_TIMEOUT:?} 内未返回"
            )
        });

    // Assert — 编译干净，且三点各自经共享口拿到 hit/miss
    assert_eq!(diagnostics, 0, "探针源码应零诊断，否则读数形状漂了");
    let mut readout: Vec<String> = Vec::new();
    let mut hits_from_consumers = 0u64;
    let mut misses_from_consumers = 0u64;
    for ((consumer, base), now) in SolverConsumer::ALL.iter().copied().zip(before).zip(after) {
        let delta = stats_delta(base, now);
        readout.push(format!(
            "{}: acq={} hit={} miss={}",
            consumer.name(),
            delta.acquisitions,
            delta.hits,
            delta.misses
        ));
        hits_from_consumers += delta.hits;
        misses_from_consumers += delta.misses;
        assert!(
            delta.acquisitions >= 1,
            "消费点 {} 未经共享口获取求解器；读数: {readout:?}",
            consumer.name()
        );
        assert!(
            delta.misses >= 1,
            "消费点 {} 未产生未命中（首查必须真求解）；读数: {readout:?}",
            consumer.name()
        );
        assert!(
            delta.hits >= 1,
            "消费点 {} 未产生命中（同形查询应命中共享缓存）；读数: {readout:?}",
            consumer.name()
        );
    }
    if let (Some((hits_before, misses_before)), Some((hits_after, misses_after))) =
        (total_before, total_after)
    {
        assert!(
            hits_after - hits_before >= hits_from_consumers
                && misses_after - misses_before >= misses_from_consumers,
            "共享实例自身计数增量须覆盖三点增量之和（三点确实共用这一实例）：\
             实例增量 ({}, {}) vs 三点之和 ({hits_from_consumers}, {misses_from_consumers})",
            hits_after - hits_before,
            misses_after - misses_before
        );
    }
    println!(
        "WBS 4.4.2 读数（一次编译）: {} | 共享实例累计 hit/miss: {total_after:?}",
        readout.join(" | ")
    );
}
