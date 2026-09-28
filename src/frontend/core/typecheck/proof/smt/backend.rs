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
//! 生命周期、参数设置或其他状态。
//!
//! # 线程约束
//!
//! `Z3Backend` 内部用 `RefCell` 缓存，`RefCell` 是 `Send` 但不 `Sync`，且 Z3
//! 的 FFI 约定要求同一 context 不能被并发访问。因此 `Solver` 只要求
//! `Send`（满足 `Mutex<Box<dyn Solver>>` 静态共享），**不要求 `Sync`**——
//! 声称 `Sync` 会让 `&Z3Backend` 被多线程并发调用 `solve`，造成 Z3 context
//! 与 `RefCell` 的数据竞争。并发访问必须经 `Mutex` 串行化。

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
}

/// 构造默认求解器后端
///
/// 这是**唯一**的后端选择点：换求解器（如 CVC5）只需改这里，
/// 上层代码（`predicate.rs` / `ownership.rs` / `termination.rs`）无感知。
///
/// # 返回
///
/// - `Some(backend)`：后端可用
/// - `None`：后端不可用（如 Z3 未安装）。调用方应保守降级
///
/// 注意选择 `Option` 而非 `Result`：调用方的降级动作不做区分
/// （无论初始化失败还是 unknown 都是保守方向），额外错误类型无收益。
///
/// wasm32 下无 FFI 后端，返回 `None`（消费方走文本路径或保守降级）。
#[cfg(not(target_arch = "wasm32"))]
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
