//! 求解器后端抽象测试 — 基于 RFC-027 §8
//!
//! §8: 「不做通用求解器抽象层——SMT-LIB 就是抽象层。未来如果 CVC5 在特定
//! 理论上突破，切换只需换二进制，不需改编译器代码。」
//! 本文件验证该可替换性：上层只依赖 `Solver` trait，替换后端不需要改动
//! `predicate.rs` / `ownership.rs` / `termination.rs`。
//!
//! 回归性质：本抽象落地前，三个消费层直接持有 `Z3Backend` 具体类型
//! （`Z3_INSTANCE: Mutex<Z3Backend>`、`z3: &'static Z3Backend`、
//! `Z3Backend::new()`），任何非 Z3 后端都无法接入。以下用例在旧结构下
//! 无法编译。

#[cfg(not(target_arch = "wasm32"))]
use crate::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTExpr, SMTResult, SMTSort};
#[cfg(not(target_arch = "wasm32"))]
use crate::frontend::core::typecheck::proof::smt::backend::{Solver, default_solver};

/// 一个完全不含 Z3 的 `Solver` 实现——固定返回 `Unsat`。
///
/// 存在性本身就是断言：若 trait 泄漏了任何 Z3 专有类型（context 句柄、
/// `Z3_ast` 等），本类型无法实现 `Solver`。
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct AlwaysUnsatSolver {
    /// 记录收到的查询次数，供断言验证命令确实流经 trait 边界
    calls: std::cell::Cell<usize>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Solver for AlwaysUnsatSolver {
    fn solve(
        &self,
        _commands: &[SMTCommand],
        _timeout_ms: u64,
    ) -> SMTResult {
        self.calls.set(self.calls.get() + 1);
        SMTResult::Unsat
    }
}

/// 求解器后端可以在不触碰 Z3 的情况下实现并替换（RFC-027 §8 可替换性）
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_solver_trait_accepts_non_z3_backend() {
    // Arrange — 构造一个与 Z3 无关的后端
    let backend = AlwaysUnsatSolver {
        calls: std::cell::Cell::new(0),
    };
    let commands = vec![
        SMTCommand::DeclareConst("x".into(), SMTSort::Int),
        SMTCommand::CheckSat,
    ];

    // Act — 经 trait 对象调用（消费层的实际使用形态）
    let dyn_backend: &dyn Solver = &backend;
    let result = dyn_backend.solve(&commands, 100);

    // Assert
    assert!(
        matches!(result, SMTResult::Unsat),
        "非 Z3 后端应能经 Solver trait 返回结果，实际得到 {:?}",
        result
    );
    assert_eq!(backend.calls.get(), 1, "命令应流经 trait 边界到达后端实现");
}

/// `Solver` 必须能装入 `Mutex<Box<dyn Solver>>`——`predicate.rs` 的静态共享形态
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_solver_object_is_send_for_static_sharing() {
    // Arrange — 静态共享要求 Box<dyn Solver> 满足 Send
    let boxed: Box<dyn Solver> = Box::new(AlwaysUnsatSolver {
        calls: std::cell::Cell::new(0),
    });

    // Act
    let shared = std::sync::Mutex::new(boxed);
    let guard = shared.lock().unwrap();
    let result = guard.solve(&[SMTCommand::CheckSat], 50);

    // Assert
    assert!(
        matches!(result, SMTResult::Unsat),
        "经 Mutex 共享的后端应正常工作，实际得到 {:?}",
        result
    );
}

/// `default_solver()` 是唯一后端选择点；返回的后端确实可求解（非占位对象）
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_default_solver_returns_usable_backend() {
    // Arrange — 一个必然 unsat 的查询：目标取反后自相矛盾
    let commands = vec![
        SMTCommand::DeclareConst("x".into(), SMTSort::Int),
        SMTCommand::Assert(SMTExpr::App(
            "not".into(),
            vec![SMTExpr::App(
                ">=".into(),
                vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("x".into())],
            )],
        )),
        SMTCommand::CheckSat,
    ];

    // Act
    let Some(solver) = default_solver() else {
        // Z3 未安装时跳过：本用例验证的是「返回的后端可用」，不是强制依赖 Z3
        return;
    };
    let result = solver.solve(&commands, 1000);

    // Assert — 后端必须真求解，而不是恒返回 Unknown 的占位实现
    assert!(
        matches!(result, SMTResult::Unsat),
        "default_solver 返回的后端应能正确求解必然 unsat 的查询，实际得到 {:?}",
        result
    );
}
