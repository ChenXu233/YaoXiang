//! 终止检查器单元测试
//!
//! 规范来源：
//! - RFC-027《编译期谓词与统一静态验证》§7「循环终止性证明」
//!   - §7.2 策略 1：线性秩函数自动合成（SMT 验证）—— 见文末 tripwire
//!   - §7.3 策略 2：谓词违反计数（框架占位）
//!   - §7.5 策略 4：乘法缩放度量
//! - 语言规范 §控制流「while 循环」

use crate::frontend::core::typecheck::layers::termination::TerminationChecker;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::parser::ast::{BinOp, Block, Expr, Literal, Stmt, StmtKind};
use crate::util::span::Span;

// ==================== 测试辅助函数 ====================

fn dummy_span() -> Span {
    Span::default()
}

/// 创建 `i += delta` 表达式 (`i = i + delta`)
fn make_increment(
    var: &str,
    delta: i128,
) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(Expr::Var(var.to_string(), dummy_span())),
            right: Box::new(Expr::BinOp {
                op: BinOp::Add,
                left: Box::new(Expr::Var(var.to_string(), dummy_span())),
                right: Box::new(Expr::Lit(Literal::Int(delta), dummy_span())),
                span: dummy_span(),
            }),
            span: dummy_span(),
        })),
        span: dummy_span(),
    }
}

/// 创建 `i -= delta` 表达式
fn make_decrement(
    var: &str,
    delta: i128,
) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(Expr::Var(var.to_string(), dummy_span())),
            right: Box::new(Expr::BinOp {
                op: BinOp::Sub,
                left: Box::new(Expr::Var(var.to_string(), dummy_span())),
                right: Box::new(Expr::Lit(Literal::Int(delta), dummy_span())),
                span: dummy_span(),
            }),
            span: dummy_span(),
        })),
        span: dummy_span(),
    }
}

/// 创建 `i < n` 条件
fn make_lt_condition(
    var: &str,
    bound: &str,
) -> Box<Expr> {
    Box::new(Expr::BinOp {
        op: BinOp::Lt,
        left: Box::new(Expr::Var(var.to_string(), dummy_span())),
        right: Box::new(Expr::Var(bound.to_string(), dummy_span())),
        span: dummy_span(),
    })
}

/// 创建 `i > 0` 条件
fn make_gt_condition(
    var: &str,
    bound: i128,
) -> Box<Expr> {
    Box::new(Expr::BinOp {
        op: BinOp::Gt,
        left: Box::new(Expr::Var(var.to_string(), dummy_span())),
        right: Box::new(Expr::Lit(Literal::Int(bound), dummy_span())),
        span: dummy_span(),
    })
}

/// 创建 `while cond { body }` 表达式
fn make_while(
    condition: Box<Expr>,
    body_stmts: Vec<Stmt>,
) -> Box<Expr> {
    Box::new(Expr::While {
        condition,
        body: Box::new(Block {
            stmts: body_stmts,
            span: dummy_span(),
        }),
        span: dummy_span(),
    })
}

/// 创建 for 循环表达式
fn make_for(
    var: &str,
    iterable: Box<Expr>,
    body_stmts: Vec<Stmt>,
) -> Box<Expr> {
    Box::new(Expr::For {
        var: var.to_string(),
        var_mut: false,
        iterable,
        body: Box::new(Block {
            stmts: body_stmts,
            span: dummy_span(),
        }),
        span: dummy_span(),
    })
}

/// 运行终止检查器，**并把给定变量标记为已精化**（进 RFC-027 §7 验证模式）
///
/// §7：裸 `while` 不进验证模式，只有度量变量带精化标注时才生成终止义务。
/// 未标注精化的循环用 `run_check`（期望「不检查」）；要验证「检查确实发生」
/// 必须用本函数显式声明精化，否则测的是门控而非检查逻辑。
fn run_check_with_refined(
    expr: &Expr,
    refined: &[&str],
) -> Vec<ProofResult> {
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(expr.clone())),
        span: dummy_span(),
    };
    let module = crate::frontend::core::parser::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::typecheck::environment::TypeEnvironment::new();
    let vars: std::collections::HashSet<String> = refined.iter().map(|s| s.to_string()).collect();
    let mut checker = TerminationChecker::new().set_refined_vars(vars);
    checker.check_module(&module, &env)
}

/// 运行终止检查器
fn run_check(expr: &Expr) -> Vec<ProofResult> {
    // 将表达式包装为模块语句
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(expr.clone())),
        span: dummy_span(),
    };
    let module = crate::frontend::core::parser::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::typecheck::environment::TypeEnvironment::new();
    let mut checker = TerminationChecker::new();
    checker.check_module(&module, &env)
}

/// 运行终止检查器，并注入一个**确定性**桩求解器
///
/// 桩恒返回 `Unsat`（即「候选在所有路径上严格递减」）。这样用例不依赖
/// 本机是否安装 Z3，也就能在 CI 上稳定判定策略 1 的可用性。
#[cfg(not(target_arch = "wasm32"))]
fn run_check_with_stub_solver(expr: &Expr) -> Vec<ProofResult> {
    use crate::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTResult};
    use crate::frontend::core::typecheck::proof::smt::backend::Solver;

    /// 恒「验证通过」的桩：任何候选都被判定为严格递减
    #[derive(Debug)]
    struct AlwaysUnsat;
    impl Solver for AlwaysUnsat {
        fn solve(
            &self,
            _commands: &[SMTCommand],
            _timeout_ms: u64,
        ) -> SMTResult {
            SMTResult::Unsat
        }
    }
    static ALWAYS_UNSAT: AlwaysUnsat = AlwaysUnsat;

    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(expr.clone())),
        span: dummy_span(),
    };
    let module = crate::frontend::core::parser::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::typecheck::environment::TypeEnvironment::new();
    let mut checker = TerminationChecker::new().with_solver(&ALWAYS_UNSAT);
    checker.check_module(&module, &env)
}

/// 同 `run_check_with_stub_solver`，并声明给定变量已精化（进 §7 验证模式）
#[cfg(not(target_arch = "wasm32"))]
fn run_check_with_stub_solver_refined(
    expr: &Expr,
    refined: &[&str],
) -> Vec<ProofResult> {
    use crate::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTResult};
    use crate::frontend::core::typecheck::proof::smt::backend::Solver;

    #[derive(Debug)]
    struct AlwaysUnsat;
    impl Solver for AlwaysUnsat {
        fn solve(
            &self,
            _commands: &[SMTCommand],
            _timeout_ms: u64,
        ) -> SMTResult {
            SMTResult::Unsat
        }
    }
    static ALWAYS_UNSAT: AlwaysUnsat = AlwaysUnsat;

    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(expr.clone())),
        span: dummy_span(),
    };
    let module = crate::frontend::core::parser::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::typecheck::environment::TypeEnvironment::new();
    let vars: std::collections::HashSet<String> = refined.iter().map(|s| s.to_string()).collect();
    let mut checker = TerminationChecker::new()
        .with_solver(&ALWAYS_UNSAT)
        .set_refined_vars(vars);
    checker.check_module(&module, &env)
}

// ==================== 测试：循环终止 ====================

#[test]
fn test_while_increment_to_bound_terminates() {
    // while i < n { i += 1 }
    let while_expr = make_while(make_lt_condition("i", "n"), vec![make_increment("i", 1)]);

    let results = run_check(&while_expr);
    // 所有结果应该都是 Proved（或者空——没有 while 循环外的其他检查）
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results, got unproven: {:?}",
        unproven
    );
}

#[test]
fn test_while_decrement_to_lower_bound_terminates() {
    // while i > 0 { i -= 1 }
    let while_expr = make_while(make_gt_condition("i", 0), vec![make_decrement("i", 1)]);

    let results = run_check(&while_expr);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results, got unproven: {:?}",
        unproven
    );
}

#[test]
fn test_while_constant_condition_fails() {
    // while true { x = 1 } — 没有递减的循环变量
    let body_stmt = Box::new(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(Expr::Var("x".to_string(), dummy_span())),
            right: Box::new(Expr::Lit(Literal::Int(1), dummy_span())),
            span: dummy_span(),
        })),
        span: dummy_span(),
    });
    let while_expr = Box::new(Expr::While {
        condition: Box::new(Expr::Lit(Literal::Bool(true), dummy_span())),
        body: Box::new(Block {
            stmts: vec![*body_stmt],
            span: dummy_span(),
        }),
        span: dummy_span(),
    });

    // §7：体中是 `x = 1`（非递减），但传入精化集合 {x} 使其进验证模式
    let results = run_check_with_refined(&while_expr, &["x"]);
    assert!(
        !results.is_empty(),
        "Expected unproven result for non-terminating loop"
    );
    assert!(
        matches!(&results[0], ProofResult::Unproven { .. }),
        "Expected Unproven, got: {:?}",
        results[0]
    );
}

/// RFC-027 §7：**裸 `while` 不进验证模式**，不生成任何终止义务
///
/// 回归性质：此前实现对所有循环无差别要求终止证明，导致
/// `mut i: Int = 0; while true { x = 1 }` 这类无精化标注的循环被误报 E4021。
/// §7 明确「裸 `while` 不进验证模式」——无精化 → 不检查 → 无诊断。
#[test]
fn test_bare_while_without_refinement_generates_no_obligation() {
    // Arrange — 同一个非终止循环，但**不传**精化集合
    let body_stmt = Box::new(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(Expr::Var("x".to_string(), dummy_span())),
            right: Box::new(Expr::Lit(Literal::Int(1), dummy_span())),
            span: dummy_span(),
        })),
        span: dummy_span(),
    });
    let while_expr = Box::new(Expr::While {
        condition: Box::new(Expr::Lit(Literal::Bool(true), dummy_span())),
        body: Box::new(Block {
            stmts: vec![*body_stmt],
            span: dummy_span(),
        }),
        span: dummy_span(),
    });

    // Act — 不经 set_refined_vars（空集 = 无精化）
    let results = run_check(&while_expr);

    // Assert — 不进验证模式，故无任何终止义务（不应出现 Unproven）
    assert!(
        results.iter().all(|r| r.is_proved()),
        "裸 while 按 §7 不进验证模式，不应生成 Unproven。实际: {:?}",
        results
    );
}

#[test]
fn test_while_no_assignment_fails() {
    // while i < n { print(i) } — 循环体内没有修改 i
    let body_stmt = Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Var("i".to_string(), dummy_span()))),
        span: dummy_span(),
    };
    let while_expr = make_while(make_lt_condition("i", "n"), vec![body_stmt]);

    let results = run_check_with_refined(&while_expr, &["i"]);
    assert!(
        !results.is_empty(),
        "Expected unproven result for loop with no progress"
    );
    assert!(
        matches!(&results[0], ProofResult::Unproven { .. }),
        "Expected Unproven, got: {:?}",
        results[0]
    );
}

#[test]
fn test_while_decrement_wrong_direction_fails() {
    // while i < n { i -= 1 } — i 递减但上界是 n，方向错误
    let while_expr = make_while(make_lt_condition("i", "n"), vec![make_decrement("i", 1)]);

    let results = run_check_with_refined(&while_expr, &["i"]);
    assert!(
        !results.is_empty(),
        "Expected unproven result: i decreases when it should increase toward bound"
    );
    assert!(
        matches!(&results[0], ProofResult::Unproven { .. }),
        "Expected Unproven, got: {:?}",
        results[0]
    );
}

#[test]
fn test_while_increment_by_two_terminates() {
    // while i < n { i += 2 }
    let while_expr = make_while(make_lt_condition("i", "n"), vec![make_increment("i", 2)]);

    let results = run_check(&while_expr);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results for i += 2, got unproven: {:?}",
        unproven
    );
}

// ==================== 测试：for 循环 ====================

#[test]
fn test_for_loop_trivially_terminates() {
    // for x in range { print(x) }
    let body_stmt = Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Var("x".to_string(), dummy_span()))),
        span: dummy_span(),
    };
    let for_expr = make_for(
        "x",
        Box::new(Expr::Var("range".to_string(), dummy_span())),
        vec![body_stmt],
    );

    let results = run_check(&for_expr);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results for for-loop, got unproven: {:?}",
        unproven
    );
}

// ==================== 测试：嵌套循环 ====================

#[test]
fn test_nested_while_both_terminating() {
    // 外层 while i < n { i += 1 }
    let inner_while = make_while(make_lt_condition("j", "m"), vec![make_increment("j", 1)]);
    let inner_stmt = Stmt {
        kind: StmtKind::Expr(inner_while),
        span: dummy_span(),
    };
    let body_stmts = vec![make_increment("i", 1), inner_stmt];
    let outer_while = make_while(make_lt_condition("i", "n"), body_stmts);

    let results = run_check(&outer_while);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results for nested terminating loops, got unproven: {:?}",
        unproven
    );
}

#[test]
fn test_nested_while_inner_fails() {
    // 外层终止，内层不终止。
    //
    // §7：内层必须有带精化的度量变量才会进验证模式——空体 `while true {}`
    // 无变量可言，按 §7 不生成义务，故此处用 `while j > 0 { j = j + 1 }`：
    // 有度量变量 j、方向错误故不终止，且声明 j 已精化。
    let inner_while = Box::new(Expr::While {
        condition: make_gt_condition("j", 0),
        body: Box::new(Block {
            stmts: vec![make_increment("j", 1)],
            span: dummy_span(),
        }),
        span: dummy_span(),
    });
    let inner_stmt = Stmt {
        kind: StmtKind::Expr(inner_while),
        span: dummy_span(),
    };
    let outer_while = make_while(
        make_lt_condition("i", "n"),
        vec![make_increment("i", 1), inner_stmt],
    );

    let results = run_check_with_refined(&outer_while, &["i", "j"]);
    // 外层终止(Proved)但内层不终止(Unproven) → 至少 1 个 Unproven
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        !unproven.is_empty(),
        "Expected at least one Unproven result for inner non-terminating loop"
    );
}

// ==================== 策略 1 可用性 tripwire（#377）====================

/// 策略 1 当前不产出任何可用度量，故 `i < j { i += 1; j -= 1 }` 无法被证明
///
/// RFC-027 §7 把「四种策略都无法自动拿下」的循环交给策略 1（线性秩函数自动
/// 合成），其典型例子正是本用例的形状。但实测策略 1 恒空手，两处独立缺陷：
///
/// - **(a) 输入为空**：`check_while_loop` 的 `bound_is_loop_invariant` 过滤会
///   剔除「在循环体内被赋值的边界变量」，而这类循环的边界按定义就在体内被改
///   → `bounds` 为空 → 零候选。
/// - **(b) 判定符号相反**：`generate_rank_candidates` 产出的 `delta` 恒为 `+1`，
///   而 `verify_rank_candidate` 构造 `m' = m + delta` 后断言 `not (m' < m)`；
///   `m + 1 < m` 恒假 → 求解器返回 Sat → 验证恒失败。
///
/// 本用例**端到端**锁定「该形状当前不被证明」这一事实，并用恒返回 `Unsat`
/// （即无条件判定「严格递减成立」）的桩求解器，使之不依赖本机 Z3、CI 上稳定。
/// 桩都救不回来，正说明缺陷在度量合成侧而非求解器侧。
///
/// 修好 (a)（或 (a)+(b)）后此处会失败——届时请把断言翻转成「该循环被证明」
/// 并更新用例名，不要删除用例。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_strategy1_cannot_prove_dual_variable_loop_currently() {
    // Arrange — i < j { i += 1; j -= 1 }：边界 j 在循环体内被赋值
    let while_expr = make_while(
        make_lt_condition("i", "j"),
        vec![make_increment("i", 1), make_decrement("j", 1)],
    );

    // Act — 注入恒 Unsat（「候选严格递减」）的桩求解器。
    // 声明 i/j 已精化，使该循环进 §7 验证模式——否则测的是门控而非策略 1。
    let results = run_check_with_stub_solver_refined(&while_expr, &["i", "j"]);

    // Assert — 即便求解器无条件认可任何候选，该循环仍未被证明
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        !unproven.is_empty(),
        "策略 1 当前恒空手（#377 缺陷 a：边界过滤清空输入；缺陷 b：delta 符号相反），
         故恒 Unsat 桩求解器下该循环仍不应被证明。
         若此处开始通过，说明策略 1 已可用——请翻转断言并更新用例名。results={:?}",
        results
    );
}

/// 终止策略 2（谓词违反计数）是框架占位，恒不产出度量（RFC-027 §7.3）
///
/// 实现注释标注「框架占位」：完整实现需 parser 支持 forall 量词语法，而语言
/// 层面尚无该语法。本用例与上一条同形——两者合起来说明「策略 1 空手 + 策略 2
/// 占位」时该形状无任何自动证明路径。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_strategy2_violation_count_is_placeholder_not_usable() {
    // Arrange
    let while_expr = make_while(
        make_lt_condition("i", "j"),
        vec![make_increment("i", 1), make_decrement("j", 1)],
    );

    // Act — 声明 i/j 精化以进 §7 验证模式（否则门控直接放行，测不到策略 2）
    let results = run_check_with_stub_solver_refined(&while_expr, &["i", "j"]);

    // Assert — 无任何策略成立
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        !unproven.is_empty(),
        "策略 2 当前为框架占位（需 forall 量词），不应产出度量。
         若此处开始通过，说明策略 2 已实装——请更新断言与用例名。results={:?}",
        results
    );
}
