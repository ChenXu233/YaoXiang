//! 终止检查器单元测试
//!
//! 规范来源：
//! - RFC-027《编译期谓词与统一静态验证》§7「循环终止性证明」
//!   - §7.2 策略 1：线性秩函数自动合成（SMT 验证）—— 见文末 tripwire
//!   - §7.3 策略 2：谓词违反计数（框架占位）
//!   - §7.5 策略 4：乘法缩放度量
//! - 语言规范 §控制流「while 循环」

use crate::frontend::core::typecheck::layers::termination::{
    MeasureObligation, MeasureVerdict, TerminationChecker,
};
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

/// 运行终止检查器，并注入一个**确定性**桩求解器，同时声明给定变量已精化
///
/// 桩恒返回 `Unsat`（即「候选在所有路径上严格递减」）。这样用例不依赖
/// 本机是否安装 Z3，也就能在 CI 上稳定判定策略 1 的可用性。
///
/// §7：裸 `while` 不进验证模式，只有度量变量带精化标注时才生成终止义务。
/// 未标注精化的循环用 `run_check`（期望「不检查」）；要验证「检查确实发生」
/// 必须用本函数显式声明精化，否则测的是门控而非检查逻辑。
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
        .with_solver_owned(Box::new(AlwaysUnsat))
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

// ==================== 测试：显式测度注入（RFC-027a §2，T2）====================

/// RFC-027a §2 —— 注入的显式测度须可按被标注名精确取回。
///
/// T2 的交付是「测度可提取、可注入」；本用例锁住注入侧的契约：键是被标注的
/// 绑定/函数名，值是测度的无损 `ConstExpr`。**复合测度不得被压成单变量**——
/// 压扁后 SMT 会拿到错误的测度（`n - i` 变成 `n`），义务生成随之错误。
#[test]
fn test_injected_explicit_measures_are_retrievable() {
    use crate::frontend::core::types::const_data::{BinOp as ConstBinOp, ConstExpr};

    // Arrange
    let mut measures = std::collections::HashMap::new();
    measures.insert("gcd".to_string(), ConstExpr::NamedVar("b".to_string()));
    measures.insert(
        "acc".to_string(),
        ConstExpr::BinOp {
            op: ConstBinOp::Sub,
            left: Box::new(ConstExpr::NamedVar("n".to_string())),
            right: Box::new(ConstExpr::NamedVar("i".to_string())),
        },
    );

    // Act
    let checker = TerminationChecker::new().set_measures(measures);

    // Assert
    assert_eq!(
        checker.measures().get("gcd"),
        Some(&ConstExpr::NamedVar("b".to_string())),
        "函数名键应取回其返回类型位的测度"
    );
    let compound = checker.measures().get("acc").expect("acc 的测度应可取回");
    assert!(
        matches!(
            compound,
            ConstExpr::BinOp {
                op: ConstBinOp::Sub,
                ..
            }
        ),
        "复合测度 n - i 应保留 BinOp(Sub) 结构，不得退化为单变量；实际: {compound:?}"
    );
    assert_eq!(
        checker.measures().len(),
        2,
        "只应注入两个测度，实际: {:?}",
        checker.measures()
    );
}

/// RFC-027a §2 —— 未注入时测度表为空（不得凭空产生测度）。
#[test]
fn test_no_measures_injected_means_empty_table() {
    // Arrange / Act
    let checker = TerminationChecker::new();

    // Assert
    assert!(
        checker.measures().is_empty(),
        "未调用 set_measures 时测度表应为空（默认不检查任何显式测度），\
         实际: {:?}",
        checker.measures()
    );
}

// ==================== 测试：显式测度的递归义务（RFC-027a §义务生成，T3）====================

/// 解析源码 → 注入测度表 → 跑终止检查 → 取生成的测度义务
fn measure_obligations_of(
    source: &str,
    measures: &[(&str, crate::frontend::core::types::const_data::ConstExpr)],
) -> Vec<MeasureObligation> {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;
    use crate::frontend::core::typecheck::environment::TypeEnvironment;

    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "解析应无错误，实际: {:?}",
        parsed.errors
    );
    let table = measures
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    let env = TypeEnvironment::new();
    let mut checker = TerminationChecker::new().set_measures(table);
    checker.check_module(&parsed.module, &env);
    checker.measure_obligations().to_vec()
}

/// RFC-027a §义务生成 —— 带显式测度的函数，其**自递归调用**须产生测度义务。
///
/// 这是本期交付面（§五 风险 4：函数形态的非结构递归，gcd）。义务原料 = 形参 +
/// 测度 + 调用实参——三者齐备，T4 才能构造 `m[形参:=实参] < m` 并送 SMT。
#[test]
fn test_recursive_self_call_generates_measure_obligation() {
    use crate::frontend::core::types::const_data::{BinOp as ConstBinOp, ConstExpr};

    // Arrange — gcd 形态：测度 b，自调用 gcd(b, a % b)
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];

    // Act
    let obligations = measure_obligations_of(source, &measures);

    // Assert
    assert_eq!(
        obligations.len(),
        1,
        "gcd 有 1 处自递归调用，应生成 1 条测度义务；实际: {obligations:?}"
    );
    let ob = &obligations[0];
    assert_eq!(ob.fn_name, "gcd", "义务应归属发起递归的函数");
    assert_eq!(
        ob.params,
        vec!["a".to_string(), "b".to_string()],
        "义务须带上形参表，T4 才能做「形参 := 实参」替换"
    );
    assert_eq!(
        ob.measure,
        ConstExpr::NamedVar("b".to_string()),
        "义务须带显式测度原式"
    );
    assert_eq!(
        ob.call_args.len(),
        2,
        "调用实参 gcd(b, a % b) 应有两个，实际: {:?}",
        ob.call_args
    );
    assert_eq!(
        ob.call_args[0],
        ConstExpr::NamedVar("b".to_string()),
        "首个实参应为 NamedVar(b)"
    );
    assert!(
        matches!(
            ob.call_args[1],
            ConstExpr::BinOp {
                op: ConstBinOp::Mod,
                ..
            }
        ),
        "次个实参应为 a % b 的 BinOp(Mod) 结构（不得丢表达式），实际: {:?}",
        ob.call_args[1]
    );
}

/// RFC-027a §义务生成 —— **无显式测度**的函数即便自递归也不得产生义务。
///
/// 反例守卫：否则任何递归函数都会被显式测度路径接管，绕过 RFC-027 §7 的
/// 「无测度 → 硬边界」语义。
#[test]
fn test_recursive_without_explicit_measure_generates_no_obligation() {
    // Arrange — 同样是自递归，但测度表为空
    let source = "f: (n: Int) -> Int = { \
                  if n == 0 { return 0 } \
                  return f(n - 1) }";

    // Act
    let obligations = measure_obligations_of(source, &[]);

    // Assert
    assert!(
        obligations.is_empty(),
        "未声明显式测度的函数不应走显式测度路径；实际: {obligations:?}"
    );
}

/// RFC-027a §义务生成 —— 调用**其它**函数不是自递归，不产生义务。
///
/// 反例守卫：只按「函数名 == 当前函数名」判定自调用；否则任何函数调用都会被
/// 误当回边（互递归需 SCC，本期不做——见计划 §五 非目标）。
#[test]
fn test_call_to_other_function_generates_no_obligation() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange — gcd 有测度，但调用的是别的函数
    let source = "helper: (x: Int) -> Int = { x }\n\
                  gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return helper(b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];

    // Act
    let obligations = measure_obligations_of(source, &measures);

    // Assert
    assert!(
        obligations.is_empty(),
        "对 helper 的调用不是 gcd 的自递归，不应产生测度义务；实际: {obligations:?}"
    );
}

/// RFC-027a §义务生成 —— 有测度但体内无自调用（纯迭代实现）时零义务。
#[test]
fn test_measured_function_without_self_call_generates_no_obligation() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "g: (a: Int, b: Int) -> Terminates(b) = { a + b }";
    let measures = [("g", ConstExpr::NamedVar("b".to_string()))];

    // Act
    let obligations = measure_obligations_of(source, &measures);

    // Assert
    assert!(
        obligations.is_empty(),
        "体内无自递归调用时不应产生义务；实际: {obligations:?}"
    );
}

// ==================== 测试：测度义务的 SMT 判定（RFC-027a §判定管线，T4）====================

/// 解析源码 → 注入测度 → 可选注入求解器 → 跑终止检查 → 取判定结果
#[cfg(not(target_arch = "wasm32"))]
fn measure_verdicts_of(
    source: &str,
    measures: &[(&str, crate::frontend::core::types::const_data::ConstExpr)],
    solver: Option<Box<dyn crate::frontend::core::typecheck::proof::smt::backend::Solver>>,
) -> Vec<MeasureVerdict> {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;
    use crate::frontend::core::typecheck::environment::TypeEnvironment;

    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析应无错误: {:?}", parsed.errors);
    let table = measures
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    let mut checker = TerminationChecker::new().set_measures(table);
    if let Some(s) = solver {
        checker = checker.with_solver_owned(s);
    }
    let env = TypeEnvironment::new();
    checker.check_module(&parsed.module, &env);
    checker.measure_verdicts().to_vec()
}

/// 恒返回 `Unsat` 的桩（语义：所有取值下测度都严格递减）
#[cfg(not(target_arch = "wasm32"))]
fn always_unsat() -> Box<dyn crate::frontend::core::typecheck::proof::smt::backend::Solver> {
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
    Box::new(AlwaysUnsat)
}

/// 恒返回 `Sat` 的桩（语义：存在反例，测度未严格递减）
#[cfg(not(target_arch = "wasm32"))]
fn always_sat() -> Box<dyn crate::frontend::core::typecheck::proof::smt::backend::Solver> {
    use crate::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTModel, SMTResult};
    use crate::frontend::core::typecheck::proof::smt::backend::Solver;

    #[derive(Debug)]
    struct AlwaysSat;
    impl Solver for AlwaysSat {
        fn solve(
            &self,
            _commands: &[SMTCommand],
            _timeout_ms: u64,
        ) -> SMTResult {
            SMTResult::Sat {
                model: SMTModel {
                    assignments: vec![],
                },
            }
        }
    }
    Box::new(AlwaysSat)
}

/// RFC-027a §判定管线 —— 义务经 SMT 判定，Unsat → 「严格递减成立」。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_measure_obligation_proved_when_solver_reports_unsat() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange — gcd 形态，测度 b；恒 Unsat 桩
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(always_unsat()));

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::Proved],
        "Unsat 应判为严格递减成立；实际: {verdicts:?}"
    );
}

/// RFC-027a §判定管线 —— Sat（求解器给出反例）时**不得**宣称成立。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_measure_obligation_not_proved_when_solver_reports_sat() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(always_sat()));

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::NotProved],
        "Sat = 有反例，测度未严格递减，不得判为成立；实际: {verdicts:?}"
    );
}

/// RFC-027a §判定管线 —— 无求解器（wasm / Z3 缺失）时义务**不判定**。
///
/// 方向性：返回 `Unjudged` 而非 `Proved`——把「没判」当成「成立」会静默
/// 放行未证明的程序。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_measure_obligation_unjudged_without_solver() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "f: (n: Int) -> Terminates(n) = { \
                  if n == 0 { return 0 } \
                  return f(n - 1) }";
    let measures = [("f", ConstExpr::NamedVar("n".to_string()))];

    // Act — 不注入求解器
    let verdicts = measure_verdicts_of(source, &measures, None);

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::Unjudged],
        "无求解器时不应宣称任何判定；实际: {verdicts:?}"
    );
}

/// RFC-027a §判定管线（**真实 Z3**）—— 真递减的测度必须被判为成立。
///
/// `f(n - 1)` 的测度 `n`：代入后 `n - 1 < n` 恒真 → Unsat → 成立。
/// 本用例断言的是**数学**，不是埋管：桩解算器无法区分真假递减。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_real_solver_proves_genuinely_decreasing_measure() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "f: (n: Int) -> Terminates(n) = { \
                  if n == 0 { return 0 } \
                  return f(n - 1) }";
    let measures = [("f", ConstExpr::NamedVar("n".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver));

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::Proved],
        "n - 1 < n 恒真，真实 Z3 应判为严格递减成立；实际: {verdicts:?}"
    );
}

/// RFC-027a §判定管线（**真实 Z3**）—— 递增的测度必须被判为**不成立**。
///
/// 反例守卫：若判定接线接反（把 Sat 当成立），本用例与上一个会同时绿——
/// 两者一起才钉死方向。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_real_solver_rejects_increasing_measure() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange — f(n + 1)：n + 1 < n 有反例
    let source = "f: (n: Int) -> Terminates(n) = { \
                  if n == 0 { return 0 } \
                  return f(n + 1) }";
    let measures = [("f", ConstExpr::NamedVar("n".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver));

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::NotProved],
        "n + 1 < n 存在反例，真实 Z3 应判为不成立；实际: {verdicts:?}"
    );
}

/// **tripwire（T4 已知边界）** —— gcd 的递减义务当前被判为**不成立**。
///
/// 原因：义务判定只代入实参，**不采集路径守卫**，也不加良基性假设。
/// RFC-027a §示例要求：
/// 1. 路径守卫 `b != 0`（来自 `if b == 0 { return a }` 之后）
/// 2. 良基性 `b >= 0`（来自形参精化 `NonNegative(b)`）
///
/// 二者齐备才能断言 `b > 0`，进而 `a % b < b` 成立。当前两者都缺，Z3 会给出
/// 反例（`b` 取负）→ `NotProved`。
///
/// **T5 必须先把守卫与良基性接上再发射 E4022**——否则 gcd 这个正例会报错。
/// 修好之后本用例会失败：届时应更新本断言与注释，而不是删掉它。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_gcd_measure_decrease_unprovable_until_guards_wired() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver));

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::NotProved],
        "守卫与良基性未接线前，gcd 的 a % b < b 判不出（b 可取负）——本断言锁定该边界，\
         提醒 T5 不得在此之前发射 E4022；实际: {verdicts:?}"
    );
}

/// 良基性判定的 harness（RFC-027a §良基性）。
///
/// 与 [`measure_verdicts_of`] 并列：那条判「递减」，这条判「测度落在自然数上」
/// （`m >= 0`）。参数假设即函数前置条件（`b: NonNegative(b)` → `b >= 0`）。
#[cfg(not(target_arch = "wasm32"))]
fn well_founded_verdict_of(
    source: &str,
    measures: &[(&str, crate::frontend::core::types::const_data::ConstExpr)],
    param_assumptions: &[(
        &str,
        Vec<crate::frontend::core::types::const_data::ConstExpr>,
    )],
    solver: Option<Box<dyn crate::frontend::core::typecheck::proof::smt::backend::Solver>>,
) -> Vec<(String, MeasureVerdict)> {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;
    use crate::frontend::core::typecheck::environment::TypeEnvironment;

    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析应无错误: {:?}", parsed.errors);
    let table = measures
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    let assumptions = param_assumptions
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    let mut checker = TerminationChecker::new()
        .set_measures(table)
        .set_param_assumptions(assumptions);
    if let Some(s) = solver {
        checker = checker.with_solver_owned(s);
    }
    let env = TypeEnvironment::new();
    checker.check_module(&parsed.module, &env);
    let mut out: Vec<(String, MeasureVerdict)> = checker
        .well_founded_verdicts()
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// RFC-027a §良基性（**真实 Z3**）—— 前置条件给出下界时测度良基。
///
/// `b: NonNegative(b)` 代入后的约束是 `b >= 0`，正是良基性目标本身，故
/// 假设 + 目标取反 → Unsat → 成立。**这是它与非良基对照的区别所在**：
/// 同一测度、同一求解器，只差这条假设。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_real_solver_well_founded_from_param_refinement() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange —— gcd 的标准形态：b 带下界精化
    let source = "NonNegative: (x: Int) -> Type = { x >= 0 }\n\
                  gcd: (a: Int, b: NonNegative(b)) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];
    // 前置条件：b >= 0（谓词体代入后的形态）
    let assumptions = [(
        "gcd",
        vec![ConstExpr::BinOp {
            op: crate::frontend::core::types::const_data::BinOp::Ge,
            left: Box::new(ConstExpr::NamedVar("b".to_string())),
            right: Box::new(ConstExpr::Lit(
                crate::frontend::core::types::const_data::ConstValue::Int(0),
            )),
        }],
    )];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = well_founded_verdict_of(source, &measures, &assumptions, Some(solver));

    // Assert
    assert_eq!(
        verdicts,
        vec![("gcd".to_string(), MeasureVerdict::Proved)],
        "b >= 0 作为前置条件时，测度 b 的良基性应被证明（假设即目标）；实际: {verdicts:?}"
    );
}

/// 同一测度、同一求解器，**去掉前置条件**即判不出（ℤ 上 `<` 不良基）。
///
/// 与上一个用例成对：两者一起才证明「假设确实在做功」，而非判定接线恒真。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_real_solver_well_founded_needs_param_refinement() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange —— 同源码，但 b 无下界精化
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act —— 无前置条件
    let verdicts = well_founded_verdict_of(source, &measures, &[], Some(solver));

    // Assert
    assert_eq!(
        verdicts,
        vec![("gcd".to_string(), MeasureVerdict::NotProved)],
        "b 可取负（-1,-2,… 在 ℤ 上无限下降），无下界精化时良基性不得被宣称成立；\
         实际: {verdicts:?}"
    );
}

/// 无求解器时不宣称任何良基性判定（不得把「未判」当「成立」）。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_well_founded_unjudged_without_solver() {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    // Arrange
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];
    let assumptions = [(
        "gcd",
        vec![ConstExpr::BinOp {
            op: BinOp::Ge,
            left: Box::new(ConstExpr::NamedVar("b".to_string())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        }],
    )];

    // Act —— 不注入求解器
    let verdicts = well_founded_verdict_of(source, &measures, &assumptions, None);

    // Assert
    assert_eq!(
        verdicts,
        vec![("gcd".to_string(), MeasureVerdict::Unjudged)],
        "无求解器时不得宣称良基性；实际: {verdicts:?}"
    );
}
