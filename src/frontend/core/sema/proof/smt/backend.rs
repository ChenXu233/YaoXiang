//! 求解器后端抽象 — 基于 RFC-027 §8
//!
//! RFC §8 要求编译器不绑定特定求解器 API：内部翻译目标为 SMT-LIB 2.6，
//! 换求解器不应触及上层代码。本模块提供**最小**抽象面：上层只依赖
//! `Solver::solve`，后端实现（Z3 FFI 等）藏在 trait 之后。
//!
//! # 为什么抽象面这么窄
//!
//! `SMTCommand` 与 `SMTResult`（见 `ast.rs`）是纯数据类型，不含任何求解器
//! 专有类型。因此后端差异完全被 `solve` 的签名吃掉，无需暴露 context
//! 生命周期、参数设置或其他状态。唯一的例外是只读观测方法
//! [`Solver::cache_stats`]——它有默认实现（`None`），不构成后端必须履行的职责。
//!
//! # 线程约束
//!
//! `Z3Backend` 内部用 `RefCell` 缓存，`RefCell` 是 `Send` 但不 `Sync`，且 Z3
//! 的 FFI 约定要求同一 context 不能被并发访问。因此 `Solver` 只要求
//! `Send`（满足 `Mutex<Box<dyn Solver>>` 静态共享），**不要求 `Sync`**——
//! 声称 `Sync` 会让 `&Z3Backend` 被多线程并发调用 `solve`，造成 Z3 context
//! 与 `RefCell` 的数据竞争。并发访问必须经 `Mutex` 串行化。
//!
//! # 获取面：进程级共享单例（RFC-039 D58）
//!
//! 生产路径**唯一**的获取口是 [`with_shared_solver`]：`LazyLock<Mutex<Option<Box<dyn Solver>>>>`
//! 持进程级单例，闭包内借用求解器。三个消费点——精化谓词（`layers/predicate.rs`）、
//! 终止判定（`layers/termination.rs`，由 `checker.rs` 在共享口内注入）、
//! 所有权回边切断（`layers/ownership.rs`）——全部走这一口。
//!
//! 单例化的意义在**缓存可观测**：缓存是实例内的（`z3_backend.rs`），每消费点
//! 新建实例会让命中率计数与 `CACHE_CAP` 空转（WBS 4.4.2）。共享实例下
//! [`shared_solver_cache_stats`] 给出实例累计的命中/未命中数字，
//! [`shared_solver_consumer_stats`] / [`shared_solver_thread_consumer_stats`]
//! 给出按消费点切片的观测账（进程累计 / 当前线程两个口径）。
//!
//! [`default_solver`]（新建实例）只保留给测试与独立场景。
//!
//! **D58 禁令**：闭包内禁止再次获取共享求解器——`Mutex` 不可重入，嵌套即死锁；
//! 调用链 termination / ownership / predicate 串行不嵌套。禁令由
//! [`with_shared_solver`] 的同线程重入检测机制化：违反时 panic 而非挂死。
//!
//! # 观测账的读写面契约（WBS 4.4.4；2026-10-24 用户裁决 ① 保留）
//!
//! [`ConsumerStats`] / `CONSUMER_STATS` 是**可观测判据的载体**，不是业务状态。
//! 三条契约：
//!
//! 1. **写入面**：生产路径**每次经共享口获取求解器**记一次原子累加
//!    （`with_shared_solver` → `record_consumer_use`：`acquisitions` +1，
//!    `hits`/`misses` 按后端缓存计数增量累加）。
//! 2. **读取面只有** `proof/smt/tests/shared.rs`——生产代码**零调用**
//!    （进程累计口 `shared_solver_consumer_stats` 与线程镜像口
//!    `shared_solver_thread_consumer_stats` 的调用点全仓均只在该测试文件；
//!    自 #514 起判据读数走镜像口，进程累计口保留为该口径的读取口）。
//! 3. [`SolverConsumer::ALL`] 是**测试专用常量**，生产代码不得依赖。
//!
//! 为什么保留：4.4.2（D58 共享单例）的验收判据以这份读数为载体——「三点共享
//! 同一实例、命中率按消费点可观测」只能靠它证明；删掉它就得同时改写 4.4.2 的
//! 验收判据，故用户 2026-10-24 裁决 ① 保留，并把上面三条固化为契约
//! （issue [#475](https://github.com/ChenXu233/YaoXiang/issues/475)）。

use std::cell::Cell;
use std::sync::{LazyLock, Mutex};

use super::ast::{SMTCommand, SMTResult};

/// SMT 求解器后端
///
/// 实现者负责自身的初始化与生命周期管理；调用方只发命令、收结果。
///
/// # 错误处理约定
///
/// 求解失败（超时、理论不完整、初始化失败）**不是**错误返回，而是
/// `SMTResult::Unknown`。这是刻意的：RFC-027 把 SMT 定位为**精度层**
/// 而非 soundness 依赖，未知一律保守降级，由上层决定降级方向。
pub trait Solver: Send + std::fmt::Debug {
    /// 发送 SMT 命令序列并返回求解结果
    ///
    /// # 参数
    ///
    /// - `commands`: 命令序列，通常由 `translate::translate_constraint` 产出
    /// - `timeout_ms`: 本条查询的时间上限（毫秒）
    ///
    /// # 返回
    ///
    /// - `Unsat`: 目标在所有假设下成立（前提、终止性、切断等判定成立）
    /// - `Sat { model }`: 存在反例，`model` 携带赋值
    /// - `Unknown { reason }`: 无法判定，调用方应保守降级
    fn solve(
        &self,
        commands: &[SMTCommand],
        timeout_ms: u64,
    ) -> SMTResult;

    /// 缓存命中/未命中计数（命中, 未命中），后端无缓存统计时返回 `None`。
    ///
    /// 观测面而非求解面：默认实现返回 `None`，因此「有缓存」不是后端必须
    /// 履行的职责（RFC-027 §8 的抽象面仍只保证 `solve`）。Z3 后端给出进程
    /// 累计值（见 `z3_backend.rs` 的缓存契约），WBS 4.4.2 的
    /// 「缓存命中率可观测」经共享口的读取口 [`shared_solver_cache_stats`] 落地。
    fn cache_stats(&self) -> Option<(u64, u64)> {
        None
    }
}

/// 构造一个新的求解器后端实例——**唯一**的后端选择点（RFC-027 §8）。
///
/// 换求解器（如 CVC5）只需改这里，上层代码（`predicate.rs` / `ownership.rs` /
/// `termination.rs`）无感知。
///
/// # 返回
///
/// - `Some(backend)`：后端可用
/// - `None`：后端不可用（如 Z3 未安装）。调用方应保守降级
///
/// 注意选择 `Option` 而非 `Result`：调用方的降级动作不做区分
/// （无论初始化失败还是 unknown 都是保守方向），额外错误类型无收益。
///
/// wasm32 同样有 FFI 后端（#435：libz3.a 经 build.rs 静态链接进 wasm 产物，
/// Emscripten 预编译资产由 z3-wasm release 供给）——全平台统一走 Z3。
///
/// 私有：进程内只有两处需要「新实例」——共享单例 [`SHARED_SOLVER`] 的引导
/// （进程内一次）与 [`default_solver`]（测试与独立场景）。
fn new_backend() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}

/// 新建一个**独立**的求解器后端实例（测试与独立场景专用，RFC-039 D58）。
///
/// 生产路径**不得**调用本函数：每次调用新建实例，缓存计数与 `CACHE_CAP`
/// 随实例空转（缓存是实例内的，见 `z3_backend.rs`），且多实例并存不符合
/// 「同一 context 不并发访问」的约定。生产路径走进程级共享口
/// [`with_shared_solver`]。
///
/// 用途：需要与进程单例**隔离**的实例时——测试要观察「独立实例的计数不污染
/// 共享计数」，正是单例语义的反面对照。
pub fn default_solver() -> Option<Box<dyn Solver>> {
    new_backend()
}
// ==================== 进程级共享单例（RFC-039 D58） ====================

/// SMT 消费点标签（WBS 4.4.2：命中率**按消费点**可观测）。
///
/// 标签只参与观测——不改变获取语义（仍是单口、单实例、`Mutex` 串行）。
/// 命中率按消费点分开统计，是为了让「三点共享同一实例」这件事可读：
/// 三点各自的计数若来自各自的实例，跨点累计就不可能成立。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverConsumer {
    /// 精化谓词证明（`layers/predicate.rs`）
    Predicate,
    /// 终止性判定（`layers/termination.rs`，经 `checker.rs` 注入）
    Termination,
    /// 所有权回边切断判定（`layers/ownership.rs`）
    Ownership,
}

impl SolverConsumer {
    /// 全部消费点，按固定顺序（predicate → termination → ownership）。
    ///
    /// 顺序即 `TypeChecker::check_module` 里的实际执行顺序（见 02 §S5），
    /// 供探针/报告按序输出。
    ///
    /// **测试专用常量**：生产代码不得依赖本常量（使用点全仓只在
    /// `proof/smt/tests/shared.rs`）——契约见模块头「观测账的读写面契约」。
    pub const ALL: [SolverConsumer; 3] = [
        SolverConsumer::Predicate,
        SolverConsumer::Termination,
        SolverConsumer::Ownership,
    ];

    /// 稳定短名（探针输出与报告引用用；不随枚举变体顺序漂移）。
    pub fn name(self) -> &'static str {
        match self {
            SolverConsumer::Predicate => "predicate",
            SolverConsumer::Termination => "termination",
            SolverConsumer::Ownership => "ownership",
        }
    }

    /// 观测表下标——与 [`ConsumerStats`] 的存储顺序一一对应。
    fn index(self) -> usize {
        match self {
            SolverConsumer::Predicate => 0,
            SolverConsumer::Termination => 1,
            SolverConsumer::Ownership => 2,
        }
    }
}

/// 单个消费点的进程累计观测值。
///
/// `hits`/`misses` 是**进入共享口期间**后端缓存计数的增量（一次获取可能
/// 发出多条查询——终止判定在一次获取里跑完所有测度义务），故
/// `hits + misses` 不小于 `acquisitions` 恒成立。
///
/// 写入面 = 每次经共享口获取求解器记一次；读取面**只有**
/// `proof/smt/tests/shared.rs`（生产代码零调用）——契约见模块头
/// 「观测账的读写面契约」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConsumerStats {
    /// 经共享口进入求解器的次数
    pub acquisitions: u64,
    /// 这些获取期间后端缓存命中的次数
    pub hits: u64,
    /// 这些获取期间后端缓存未命中（真求解）的次数
    pub misses: u64,
}
/// 进程级共享求解器实例（D58 唯一实例）。
///
/// `Mutex` 是必须的而非可选的：`Solver` 只 `Send` 不 `Sync`（见模块顶部
/// 线程约束），并发访问必须串行化。首次使用时惰性初始化——`default_solver`
/// 只在此时被调用一次。
static SHARED_SOLVER: LazyLock<Mutex<Option<Box<dyn Solver>>>> =
    LazyLock::new(|| Mutex::new(new_backend()));

/// 消费点观测表（进程累计）。
///
/// 与 [`SHARED_SOLVER`] 分开上锁：观测计数是纯增量，不参与求解串行化。
/// 锁序单向（先求解器锁、后观测锁），读取口只取观测锁，无环。
///
/// 生产侧**零读取**：读取口（进程累计口 `shared_solver_consumer_stats` 与
/// 线程镜像口 `shared_solver_thread_consumer_stats`）的调用点全仓只在
/// `proof/smt/tests/shared.rs`——契约见模块头「观测账的读写面契约」。
/// 线程切片镜像 [`THREAD_CONSUMER_STATS`] 与本表同点同值累加。
static CONSUMER_STATS: Mutex<[ConsumerStats; 3]> = Mutex::new(
    [ConsumerStats {
        acquisitions: 0,
        hits: 0,
        misses: 0,
    }; 3],
);

thread_local! {
    /// 本线程是否正处在 [`with_shared_solver`] 的闭包内。
    ///
    /// D58 禁令的执行机制：`Mutex` 不可重入，闭包内再次获取即死锁——
    /// 死锁没有诊断信息且挂死整个编译/测试进程；本标志把违反变成显式 panic。
    static IN_SHARED_SOLVER: Cell<bool> = const { Cell::new(false) };

    /// 当前线程的观测镜像——进程账 [`CONSUMER_STATS`] 的**线程切片**（issue #514）。
    ///
    /// 写入点与进程账**同点同值**：[`record_consumer_use`] 把同一份增量同时
    /// 累加进进程账与本线程镜像。镜像只被当前线程写、只被当前线程读，因此
    /// 线程内「编译前 / 编译后」两次读数的差分恰好等于**本线程该时段**经
    /// 共享口发生的贡献——并发下其他线程的落账不可见。
    ///
    /// 为什么需要它（issue #514）：4.4.2 判据②「实例增量 ≥ 三点之和」原本以
    /// 进程账差分为读数源；进程账的落账发生在实例计数累加**之后**
    /// （[`with_shared_solver`] 里 `drop(guard)` 之后，两步之间无原子性），
    /// 并发下其他测试的一笔获取可以被读取方的快照窗口劈开——实例计数落在
    /// 窗口前缘之前、落账落在前缘之后——三点增量被推高而实例增量不含它，
    /// 判据间歇失败（单跑绿、全量并发红）。线程切片使差分天然免于跨线程
    /// 落账污染，判据本体（阈值与比对方向）不变。
    ///
    /// **测试专用读数通道**：读取面只有 `proof/smt/tests/shared.rs`，生产
    /// 代码不得依赖——与模块头「观测账的读写面契约」同规。
    static THREAD_CONSUMER_STATS: Cell<[ConsumerStats; 3]> = const {
        Cell::new([ConsumerStats {
            acquisitions: 0,
            hits: 0,
            misses: 0,
        }; 3])
    };
}
/// 在**进程级共享求解器**上执行 `f`——生产路径唯一的求解器获取口（D58）。
///
/// `f` 收到 `None` 表示后端不可用（Z3 缺失/初始化失败），调用方按各自的
/// 保守方向降级（谓词与终止判定 → 未知/未判定；所有权 → 回边穿越）。
///
/// # D58 禁令（闭包内禁止再次获取）
///
/// `Mutex` 不可重入：闭包内再次调用本函数（直接或经另一个消费点间接）
/// 会自锁死。调用链 termination / ownership / predicate 在共享口上
/// **串行不嵌套**——三点由 `TypeChecker::check_module` 顺序调用，彼此无调用关系。
///
/// 禁令不是只写在注释里：本函数检测同线程重入并 panic（消息含 `D58`），
/// 使违反变成可诊断的失败而不是无信息的挂死。跨线程并发不受影响——
/// 不同线程各自进入，由 `Mutex` 串行化等待。
///
/// # 消费点标签
///
/// `consumer` 只用于观测（WBS 4.4.2：命中率按消费点可观测），不改变获取
/// 语义——单一实例、唯一获取口、`Mutex` 串行的语义与标签无关。
pub fn with_shared_solver<R>(
    consumer: SolverConsumer,
    f: impl FnOnce(Option<&dyn Solver>) -> R,
) -> R {
    /// 闭包退出（含 panic 展开）时复位重入标志。
    struct ResetOnDrop;

    impl Drop for ResetOnDrop {
        fn drop(&mut self) {
            IN_SHARED_SOLVER.with(|flag| flag.set(false));
        }
    }

    IN_SHARED_SOLVER.with(|flag| {
        assert!(
            !flag.get(),
            "D58 禁令违反：with_shared_solver 闭包内不得再次获取共享求解器（Mutex 不可重入，嵌套即死锁；调用链 termination/ownership/predicate 必须串行不嵌套）"
        );
        flag.set(true);
    });
    let _reset = ResetOnDrop;

    let guard = SHARED_SOLVER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let before = guard.as_deref().and_then(Solver::cache_stats);
    let result = f(guard.as_deref());
    let after = guard.as_deref().and_then(Solver::cache_stats);
    drop(guard);

    record_consumer_use(consumer, before, after);
    result
}
/// 记一次消费点观测：获取次数 +1，命中/未命中按后端计数增量累加。
fn record_consumer_use(
    consumer: SolverConsumer,
    before: Option<(u64, u64)>,
    after: Option<(u64, u64)>,
) {
    let (hits, misses) = match (before, after) {
        (Some((hits_before, misses_before)), Some((hits_after, misses_after))) => (
            hits_after.saturating_sub(hits_before),
            misses_after.saturating_sub(misses_before),
        ),
        // 后端无缓存统计（trait 默认实现）或求解器不可用：只记获取次数
        _ => (0, 0),
    };

    let mut stats = CONSUMER_STATS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let entry = &mut stats[consumer.index()];
    entry.acquisitions += 1;
    entry.hits += hits;
    entry.misses += misses;
    drop(stats);

    // 同一份增量记进当前线程的镜像（写入点与进程账同点同值，见
    // THREAD_CONSUMER_STATS 的注释）——Cell 无锁，与进程账锁无交互。
    THREAD_CONSUMER_STATS.with(|mirror| {
        let mut slots = mirror.get();
        let entry = &mut slots[consumer.index()];
        entry.acquisitions += 1;
        entry.hits += hits;
        entry.misses += misses;
        mirror.set(slots);
    });
}

/// 共享实例自身的进程累计缓存计数（命中, 未命中）。
///
/// 读取口（WBS 4.4.2）。返回 `None` 有两种情形：共享实例不可用（Z3 缺失），
/// 或后端未实现缓存统计（[`Solver::cache_stats`] 默认实现）。
///
/// **不得在 [`with_shared_solver`] 闭包内调用**：读取口与获取口共用同一把锁，
/// 闭包内调用即自锁（与 D58 禁令同源，重入检测只覆盖获取口）。
pub fn shared_solver_cache_stats() -> Option<(u64, u64)> {
    let guard = SHARED_SOLVER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.as_deref().and_then(Solver::cache_stats)
}

/// 单个消费点的**进程累计**观测值（WBS 4.4.2：命中率按消费点可观测）。
///
/// 与 [`shared_solver_cache_stats`] 的差别：这里只统计**经共享口**发生的
/// 求解，且按消费点分开；与共享实例计数共同构成「三点共享同一实例」的
/// 读数（三点增量之和 = 共享实例计数器的增量，前提是没有绕过获取口的
/// 直接调用）。
///
/// 与 [`shared_solver_thread_consumer_stats`] 的差别：本函数是**进程累计**
/// 口径（含其他线程的贡献），后者是**当前线程**口径。#514 起 4.4.2 判据的
/// 读数走线程口径（进程口径的差分窗口会被并发落账劈开，见
/// [`THREAD_CONSUMER_STATS`] 注释）；本函数保留为进程累计口径的读取口，
/// 当前全仓零调用。
///
/// 读取面契约：调用方**只有** `proof/smt/tests/shared.rs`，生产代码不得
/// 调用——见模块头「观测账的读写面契约」。
pub fn shared_solver_consumer_stats(consumer: SolverConsumer) -> ConsumerStats {
    let stats = CONSUMER_STATS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    stats[consumer.index()]
}

/// 当前线程的观测镜像累计值（WBS 4.4.2；issue #514 的并发安全读数通道）。
///
/// 与 [`shared_solver_consumer_stats`] 的差别：这里只累计**当前线程**经
/// 共享口发生的获取（镜像见 [`THREAD_CONSUMER_STATS`]）。在执行编译的
/// 线程内对「编译前 / 编译后」各取一次、做差，即得**本次编译**的精确
/// 增量；进程累计口径下并发其他线程的落账会进入差分窗口，线程口径不会。
///
/// 读取面只有 `proof/smt/tests/shared.rs`，生产代码不得调用——与
/// 「观测账的读写面契约」同规。
pub fn shared_solver_thread_consumer_stats(consumer: SolverConsumer) -> ConsumerStats {
    THREAD_CONSUMER_STATS.with(|mirror| mirror.get()[consumer.index()])
}
