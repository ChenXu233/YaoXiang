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
//!
//! 「整数字面量保真」段（issue [#561](https://github.com/ChenXuXu233/YaoXiang/issues/561)，
//! 病史门）：修前 Atom 分支以 `parse::<i64>` 识别数字并经 `Z3_mk_int(n as i32)`
//! 构造——超过 i64 的字面量（i128::MAX，39 位）parse 失败被当成自由变量，
//! 可 parse 但超 i32 的字面量被截断；一元负号经 `App("-")` 单参数翻译成
//! `Z3_mk_sub`，而 `Z3_mk_sub` 官方语义（`args[0] - ... - args[n-1]`）单参数
//! 是恒等，负号丢失。三类失真都改变约束真值判定（issue #559 直评面实测：
//! 恒真约束被假拒绝为 E4018 带伪变量反例 `170...727 = (- 1)`）。以下锚
//! 以 `Assert(not constraint) + CheckSat` 编码（与 `check_predicate` 第 3 级
//! 同向）：Unsat = constraint 在真实整数语义下恒真。

#[cfg(not(target_arch = "wasm32"))]
use crate::frontend::core::sema::proof::smt::ast::{SMTCommand, SMTExpr, SMTResult, SMTSort};
#[cfg(not(target_arch = "wasm32"))]
use crate::frontend::core::sema::proof::smt::backend::{Solver, default_solver};

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

/// `default_solver()` 新建的独立实例确实可求解（非占位对象）
///
/// RFC-039 D58 后它是**测试与独立场景**的实例工厂；生产路径走
/// `backend.rs` 的进程级共享口 `with_shared_solver`（唯一后端选择点是其内部
/// 的 `new_backend`，两个入口共用）。
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

// ---- 整数字面量保真锚（issue #561，病史门；编码与 check_predicate 第 3 级同向）----

/// 构造「constraint 恒真 ⇒ Unsat」判定查询：`Assert(not constraint)` + CheckSat。
/// Unsat = 取反目标不可满足 = constraint 对所有赋值成立（Proved 方向）。
#[cfg(not(target_arch = "wasm32"))]
fn always_true_query(constraint: SMTExpr) -> Vec<SMTCommand> {
    vec![
        SMTCommand::Assert(SMTExpr::App("not".into(), vec![constraint])),
        SMTCommand::CheckSat,
    ]
}

/// 39 位字面量（i128::MAX）进约束比较必须按真实整数语义判定恒真（issue #561）
///
/// 病史：修前 `170141183460469231731687303715884105727` parse::<i64> 失败被
/// 当成自由变量 → 恒真约束 `MAX > 0` 被假拒绝为 Sat（issue #559 直评面
/// E4018 带伪变量反例的机理，探针实测反例 `170...727 = (- 1)`）。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_smt_i128_max_literal_comparison_marks_true_constraint_unsat() {
    // Arrange — 约束 i128::MAX > 0：真实整数语义恒真
    let constraint = SMTExpr::App(
        ">".into(),
        vec![
            SMTExpr::Atom(i128::MAX.to_string()),
            SMTExpr::Atom("0".into()),
        ],
    );
    let commands = always_true_query(constraint);

    // Act
    let Some(solver) = default_solver() else {
        return; // Z3 不可用（本机无库）：判向锚无法观测，跳过
    };
    let result = solver.solve(&commands, 1000);

    // Assert — 恒真约束必须 unsat；修前字面量落自由变量 → sat（假拒绝）
    assert!(
        matches!(result, SMTResult::Unsat),
        "i128::MAX > 0 恒真，取反必须 unsat（修前 39 位字面量落自由变量判 sat）, 实际: {:?}",
        result
    );
}

/// 负 39 位字面量（i128::MIN + 1 = -i128::MAX 的十进制串）进比较必须精确（issue #561）
///
/// 病史：修前带负号的 39 位串 parse::<i64> 失败落自由变量。`-MAX < 0`
/// 真实恒真 → 取反必须 unsat。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_smt_i128_min_plus_one_literal_comparison_marks_true_constraint_unsat() {
    // Arrange — 约束 (-i128::MAX) < 0：真实整数语义恒真（MIN+1 形态，负号进 Atom）
    let constraint = SMTExpr::App(
        "<".into(),
        vec![
            SMTExpr::Atom((-i128::MAX).to_string()),
            SMTExpr::Atom("0".into()),
        ],
    );
    let commands = always_true_query(constraint);

    // Act
    let Some(solver) = default_solver() else {
        return;
    };
    let result = solver.solve(&commands, 1000);

    // Assert
    assert!(
        matches!(result, SMTResult::Unsat),
        "(-i128::MAX) < 0 恒真，取反必须 unsat（修前负 39 位串落自由变量判 sat）, 实际: {:?}",
        result
    );
}

/// 超 i32 但可 parse 为 i64 的字面量不得截断——截断会翻转蕴含判向（issue #561）
///
/// 病史：修前 4000000000 `as i32` 截断为 -294967296。真实语义下假设
/// `x > 4e9` 蕴含 `x > 0`（蕴含查询编码：假设 + 取反目标 → Unsat）；截断后
/// 假设变成 `x > -294967296`（负数），不再蕴含目标 → Sat（伪反例）。
/// 注意两侧同截断的比较不翻转（同减 2³² 保序）——翻转需一侧截断一侧不截断，
/// 蕴含对（假设 4e9 截断、目标 0 不截断）正是该形态。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_smt_literal_beyond_i32_is_not_truncated_in_implication() {
    // Arrange — 假设 (x > 4000000000)，取反目标 (x <= 0)：真实语义矛盾 → unsat
    let commands = vec![
        SMTCommand::Assert(SMTExpr::App(
            ">".into(),
            vec![
                SMTExpr::Atom("x".into()),
                SMTExpr::Atom("4000000000".into()),
            ],
        )),
        SMTCommand::Assert(SMTExpr::App(
            "not".into(),
            vec![SMTExpr::App(
                ">".into(),
                vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("0".into())],
            )],
        )),
        SMTCommand::CheckSat,
    ];

    // Act
    let Some(solver) = default_solver() else {
        return;
    };
    let result = solver.solve(&commands, 1000);

    // Assert — x > 4e9 蕴含 x > 0，取反目标与假设矛盾 → unsat；
    // 修前假设截断为负数 → sat（假拒绝，伪反例落在负数区间）
    assert!(
        matches!(result, SMTResult::Unsat),
        "x > 4e9 蕴含 x > 0，蕴含查询必须 unsat（修前 4e9 截断为 -294967296 使蕴含失效判 sat）, 实际: {:?}",
        result
    );
}

/// 一元负号必须取负——Z3_mk_sub 单参数官方语义是恒等，负号曾整体丢失（issue #561）
///
/// 病史：修前 `App("-", [x])` 翻译成 `Z3_mk_sub(1, [x])`，按官方语义
/// `args[0] - ... - args[n-1]` 单参数退化为恒等——`(- 5) > 0` 被翻译成
/// `5 > 0` 恒真，取反后 unsat（假接受）。真实语义恒假 → 取反恒真 → 必须判 sat。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_smt_unary_minus_marks_false_constraint_sat() {
    // Arrange — 约束 (- 5) > 0：真实整数语义恒假
    let constraint = SMTExpr::App(
        ">".into(),
        vec![
            SMTExpr::App("-".into(), vec![SMTExpr::Atom("5".into())]),
            SMTExpr::Atom("0".into()),
        ],
    );
    let commands = always_true_query(constraint);

    // Act
    let Some(solver) = default_solver() else {
        return;
    };
    let result = solver.solve(&commands, 1000);

    // Assert — 恒假约束取反后可满足（sat）；修前负号丢失判 unsat（假接受）
    assert!(
        matches!(result, SMTResult::Sat { .. }),
        "(- 5) > 0 恒假，取反必须 sat（修前 App(-) 单参数负号丢失判 unsat）, 实际: {:?}",
        result
    );
}

/// 带负号的 i32 域小字面量经 numeral 通道后判向不变（回归对照锚，修前修后均绿）
///
/// numeral 化后负数字面量串（如 "-5"）改走 Z3_mk_numeral——本锚钉住该通道
/// 与原 Z3_mk_int 通道在 i32 域判向等价。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_smt_negative_small_literal_comparison_regression() {
    // Arrange — 约束 (-5) > 0：恒假（负号在 Atom 串内，非一元负号形态）
    let constraint = SMTExpr::App(
        ">".into(),
        vec![SMTExpr::Atom("-5".into()), SMTExpr::Atom("0".into())],
    );
    let commands = always_true_query(constraint);

    // Act
    let Some(solver) = default_solver() else {
        return;
    };
    let result = solver.solve(&commands, 1000);

    // Assert — 恒假约束取反后可满足；numeral 通道与原 mk_int 通道判向一致
    assert!(
        matches!(result, SMTResult::Sat { .. }),
        "(-5) > 0 恒假，取反必须 sat（numeral 通道不得改变 i32 域判向）, 实际: {:?}",
        result
    );
}
