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
    measure_verdicts_with_assumptions_of(source, measures, &[], solver)
}

/// 同 [`measure_verdicts_of`]，但注入函数前置条件（形参精化）。
///
/// 递减判定需要它：`a % b < b` 仅在 `b > 0` 时成立，而 `b > 0` 来自
/// 前置条件 `b >= 0` 与路径守卫 `b != 0` 两者。只给守卫不给前置条件时，
/// b 可取负（`mod` 取负号）→ 判不出。
fn measure_verdicts_with_assumptions_of(
    source: &str,
    measures: &[(&str, crate::frontend::core::types::const_data::ConstExpr)],
    param_assumptions: &[(
        &str,
        Vec<crate::frontend::core::types::const_data::ConstExpr>,
    )],
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
fn test_measure_obligation_disproved_when_solver_reports_sat() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(always_sat()));

    // Assert
    assert!(
        matches!(verdicts.as_slice(), [MeasureVerdict::Disproved { .. }]),
        "Sat 即存在反例，递减义务被判伪（可报 E4022）；实际: {verdicts:?}"
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
    assert!(
        matches!(verdicts.as_slice(), [MeasureVerdict::Disproved { .. }]),
        "n + 1 < n 存在反例，真实 Z3 应判为**判伪**（而非仅「未证明」）；\
         实际: {verdicts:?}"
    );
}

/// 无下界精化时，递减义务被判伪（RFC-027a:77）。
///
/// 与 `test_gcd_measure_decrease_proved_with_guard_and_precondition` 成对：
/// **同一源码、同一守卫、同一求解器，只差形参下界**。此处的 `b: Int` 无精化，
/// 下界无从导出，于是 `b` 可取负（`mod` 取负号）→ `a % b < b` 有反例 → 判伪。
///
/// 区分两种「不成立」（这条容易搞混）：
/// - **判伪**（本用例）：求解器给出反例 → 义务不成立 → 可报 E4022
/// - **判不出**（`NotProved`）：求解器 unknown → 不是义务为假，只给方向不拒绝
///   （RFC-027a:223）
///
/// 历史：本用例的前身锁定「守卫与良基性未接线」的边界。守卫已在路径守卫累积中
/// 落地，边界因此移到下界（前置条件）一侧。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_gcd_measure_decrease_disproved_without_lower_bound() {
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
    assert!(
        matches!(verdicts.as_slice(), [MeasureVerdict::Disproved { .. }]),
        "无下界精化时 b 可取负，a % b < b 有反例 → 判伪；实际: {verdicts:?}"
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
        .map(|(k, v)| (k.clone(), v.clone()))
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

/// 前置条件 `b >= 0`（形参精化 `NonNegative(b)` 代入后的形态）。
#[cfg(not(target_arch = "wasm32"))]
fn nonnegative_b() -> Vec<crate::frontend::core::types::const_data::ConstExpr> {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
    vec![ConstExpr::BinOp {
        op: BinOp::Ge,
        left: Box::new(ConstExpr::NamedVar("b".to_string())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    }]
}

/// RFC-027a §路径守卫（**真实 Z3**）—— gcd 的递减义务在守卫 + 前置条件下成立。
///
/// `if b == 0 { return a }` 的早返回使唯一递归调用点带上守卫 `b != 0`；配合
/// 前置条件 `b >= 0` 得 `b > 0`，于是 `a % b < b` 在 ℤ 上为真 → Unsat → 成立。
/// 这是 RFC-027a 的门面正例，也是「路径守卫累积」整套改动的验收点。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_gcd_measure_decrease_proved_with_guard_and_precondition() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "NonNegative: (x: Int) -> Type = { x >= 0 }\n\
                  gcd: (a: Int, b: NonNegative(b)) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_with_assumptions_of(
        source,
        &measures,
        &[("gcd", nonnegative_b())],
        Some(solver),
    );

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::Proved],
        "守卫 b != 0 加以前置条件 b >= 0 得 b > 0，a % b < b 应被判成立；实际: {verdicts:?}"
    );
}

/// 同一源码、同一求解器，但早返回的 then 分支**落回** → 无 `b != 0` 守卫。
///
/// 与上一个用例成对：两者一起才证明「守卫确实在做功」，而非判定恒真。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_gcd_measure_decrease_needs_guard() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange —— `if b == 0 { }` 不 return，后续语句不获 `b != 0`
    let source = "NonNegative: (x: Int) -> Type = { x >= 0 }\n\
                  gcd: (a: Int, b: NonNegative(b)) -> Terminates(b) = { \
                  if b == 0 { } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_with_assumptions_of(
        source,
        &measures,
        &[("gcd", nonnegative_b())],
        Some(solver),
    );

    // Assert
    assert!(
        matches!(verdicts.as_slice(), [MeasureVerdict::Disproved { .. }]),
        "无 `b != 0` 守卫时 b 可取 0，`a % b < b` 有反例 → 判伪；实际: {verdicts:?}"
    );
}

/// 恒返回 `Unknown` 的桩（语义：求解器判不了，**不等于**义务为假）
#[cfg(not(target_arch = "wasm32"))]
fn always_unknown() -> Box<dyn crate::frontend::core::typecheck::proof::smt::backend::Solver> {
    use crate::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTResult};
    use crate::frontend::core::typecheck::proof::smt::backend::Solver;

    #[derive(Debug)]
    struct AlwaysUnknown;
    impl Solver for AlwaysUnknown {
        fn solve(
            &self,
            _commands: &[SMTCommand],
            _timeout_ms: u64,
        ) -> SMTResult {
            SMTResult::Unknown {
                reason: "stub".to_string(),
            }
        }
    }
    Box::new(AlwaysUnknown)
}

/// **判不出 ≠ 判伪**（RFC-027a:223）—— 求解器 unknown 时须为 `NotProved` 而非 `Disproved`。
///
/// 与 `test_measure_obligation_disproved_when_solver_reports_sat` 成对：同一源码、
/// 同一递减义务，只差求解器答案（unknown / sat）。把 unknown 当判伪会违反
/// 「推不出时只给方向不拒绝」，进而在发射 E4022 时误伤。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_measure_obligation_not_proved_when_solver_reports_unknown() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return a } \
                  return gcd(b, a % b) }";
    let measures = [("gcd", ConstExpr::NamedVar("b".to_string()))];

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(always_unknown()));

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::NotProved],
        "unknown 只是判不了，不是义务为假，不得判为 Disproved；实际: {verdicts:?}"
    );
}

/// 递减义务判伪 + 良基性已证 → 发射 E4022（RFC-027a §义务生成）。
///
/// 这是 E4022 的**唯一**发射路径。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_measure_disproof_emits_e4022_when_well_founded() {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;
    use crate::frontend::core::typecheck::environment::TypeEnvironment;
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    // Arrange —— 测度 b：有下界精化（良基性可证），但递归实参不下降（b - 1 取代 b？）
    // 用递增实参 `b + 1` 使 `b + 1 < b` 判伪
    let source = "NonNegative: (x: Int) -> Type = { x >= 0 }\n\
                  f: (n: Int, b: NonNegative(b)) -> Terminates(b) = { \
                  if b == 0 { return n } \
                  return f(n, b + 1) }";
    let measures = [("f", ConstExpr::NamedVar("b".to_string()))];
    let assumptions = [(
        "f",
        vec![ConstExpr::BinOp {
            op: BinOp::Ge,
            left: Box::new(ConstExpr::NamedVar("b".to_string())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        }],
    )]
    .iter()
    .map(|(k, v)| (k.to_string(), v.clone()))
    .collect();
    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析应无错误: {:?}", parsed.errors);

    let mut checker = TerminationChecker::new()
        .set_measures(
            measures
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
        .set_param_assumptions(assumptions)
        .with_solver_owned(default_solver().expect("本用例需要 Z3"));
    let env = TypeEnvironment::new();

    // Act
    let results = checker.check_module(&parsed.module, &env);

    // Assert
    let codes: Vec<String> = results
        .into_iter()
        .filter_map(|r| r.into_result().err())
        .map(|d| d.code)
        .collect();
    assert_eq!(
        codes,
        vec!["E4022".to_string()],
        "良基性已证 + 递减判伪 应发射 E4022；实际: {codes:?}"
    );
}

/// 递减判伪但**良基性未证** → 不发射（ℤ 上不降 ≠ 不终止）。
///
/// 与上一个用例成对：同一源码、同一判伪结果，只差形参下界。少了这道门会
/// 在 `b` 可取负时误报——`gcd` 在 `b` 为负时照样终止。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_measure_disproof_silent_without_well_foundedness() {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;
    use crate::frontend::core::typecheck::environment::TypeEnvironment;
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange —— 同源码，但 b 无下界精化
    let source = "f: (n: Int, b: Int) -> Terminates(b) = { \
                  if b == 0 { return n } \
                  return f(n, b + 1) }";
    let measures = [("f", ConstExpr::NamedVar("b".to_string()))];
    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析应无错误: {:?}", parsed.errors);

    let mut checker = TerminationChecker::new()
        .set_measures(
            measures
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
        .with_solver_owned(default_solver().expect("本用例需要 Z3"));
    let env = TypeEnvironment::new();

    // Act
    let results = checker.check_module(&parsed.module, &env);

    // Assert
    assert!(
        results.is_empty(),
        "良基性未证时不得因递减判伪而报错（027a:223 只给方向不拒绝）；\
         实际结果数: {}",
        results.len()
    );
}

// ==================== RFC-027a §义务生成：循环回边的显式测度 ====================

/// 循环测度的构造捷径：`n - i`
fn measure_n_minus_i() -> crate::frontend::core::types::const_data::ConstExpr {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr};
    ConstExpr::BinOp {
        op: BinOp::Sub,
        left: Box::new(ConstExpr::NamedVar("n".to_string())),
        right: Box::new(ConstExpr::NamedVar("i".to_string())),
    }
}

/// RFC-027a §义务生成 —— 测度绑在**循环**上时须产生回边义务。
///
/// 形态取自 RFC-027 §6.9 的循环范例（`acc: Terminates(n - i) = while i < n {…}`）。
/// 义务与递归回边**同构**：`params` 是回边上被推进的变量、`call_args` 是其回边取值，
/// 于是判定侧能复用同一条 `m[params:=call_args] < m`。
///
/// 守卫必须含**循环条件**：它是回边可达的唯一假设，也是良基性下界的来源
///（`i < n` ⇒ `n - i > 0`）。缺它则循环测度的良基性恒推不出。
#[test]
fn test_loop_measure_generates_backedge_obligation() {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    // Arrange —— mut i 未带精化，故走的不是探索路径，只能是显式测度义务
    let source = "loop: (n: Int) -> Int = { \
                  mut i: Int = 0 \
                  acc: Terminates(n - i) = while i < n { i = i + 1; i } \
                  return acc }";

    // Act
    let obligations = measure_obligations_of(source, &[("acc", measure_n_minus_i())]);

    // Assert —— 恰一条回边义务，且原料齐备
    assert_eq!(
        obligations.len(),
        1,
        "循环回边应产生恰一条测度义务；实际: {obligations:?}"
    );
    let ob = &obligations[0];
    assert_eq!(
        ob.params,
        vec!["i".to_string()],
        "回边被推进的变量应作 params；实际: {:?}",
        ob.params
    );
    assert_eq!(
        ob.call_args,
        vec![ConstExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(ConstExpr::NamedVar("i".to_string())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        }],
        "回边取值应为赋值右侧 `i + 1`，由此得 `m[params:=call_args]`；实际: {:?}",
        ob.call_args
    );
    assert!(
        ob.guards
            .iter()
            .any(|g| matches!(g, ConstExpr::BinOp { op: BinOp::Lt, .. })),
        "守卫须含循环条件 `i < n`——它是下界来源；实际: {:?}",
        ob.guards
    );
}

/// RFC-027a §判定管线（**真实 Z3**）—— 循环测度真递减时须判成立。
///
/// `n - (i + 1) < n - i` 恒真（即 `-1 < 0`），与 `i`、`n` 取值无关。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_real_solver_proves_loop_measure_decrease() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;

    // Arrange
    let source = "loop: (n: Int) -> Int = { \
                  mut i: Int = 0 \
                  acc: Terminates(n - i) = while i < n { i = i + 1; i } \
                  return acc }";
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_of(source, &[("acc", measure_n_minus_i())], Some(solver));

    // Assert
    assert_eq!(
        verdicts,
        vec![MeasureVerdict::Proved],
        "`n - (i+1) < n - i` 恒真，真实 Z3 应判为成立；实际: {verdicts:?}"
    );
}

/// RFC-027a §判定管线（**真实 Z3**）—— 方向写反的循环测度须判**判伪**。
///
/// 反例守卫：与上一条成对。测度取 `i` 而体内 `i = i + 1`，于是 `i + 1 < i` 有反例。
/// 两条一起才钉死方向——只留一条时，判定接线接反也能绿。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_real_solver_rejects_increasing_loop_measure() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange —— 测度取 i，而 i 每轮递增
    let source = "loop: (n: Int) -> Int = { \
                  mut i: Int = 0 \
                  acc: Terminates(i) = while i < n { i = i + 1; i } \
                  return acc }";
    let measures = [("acc", ConstExpr::NamedVar("i".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver));

    // Assert
    assert!(
        matches!(verdicts.as_slice(), [MeasureVerdict::Disproved { .. }]),
        "`i + 1 < i` 有反例，应判**判伪**（而非仅「未证明」）；实际: {verdicts:?}"
    );
}

/// 测度式**未引用任何被赋值变量**时不得生成义务（防止误报）。
///
/// 若照生成，义务是 `m[params:=call_args] < m`，而 `m` 与被推进的变量无关，
/// 两侧**恒等** ⇒ `m < m` 恒假 ⇒ 判伪 ⇒ 报 E4022 **误报**。
/// 宁缺勿错：这种情况直接不生成。
#[test]
fn test_loop_measure_skipped_when_measure_ignores_assigned_vars() {
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange —— 测度 `n` 与循环变量 `i` 无关
    let source = "loop: (n: Int) -> Int = { \
                  mut i: Int = 0 \
                  acc: Terminates(n) = while i < n { i = i + 1; i } \
                  return acc }";
    let measures = [("acc", ConstExpr::NamedVar("n".to_string()))];

    // Act
    let obligations = measure_obligations_of(source, &measures);

    // Assert
    assert!(
        obligations.is_empty(),
        "测度未取到被赋值变量时义务恒为 `m < m`（恒假），生成即误报；实际: {obligations:?}"
    );
}

/// 赋值之间相互引用时不得生成义务（同时代入语义未定义）。
///
/// `i = i + 1; j = i` —— `j` 的回边取值 `i` 是**推进后**的 `i`。逐个代入会按次序
/// 得出 `n - i` 而非 `n - (i + 1)`，义务被判成一个无关的式子。宁缺勿错。
#[test]
fn test_loop_measure_skipped_on_mutual_assignment() {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr};

    // Arrange —— j 的右侧引用了同样被赋值的 i
    let source = "loop: (n: Int) -> Int = { \
                  mut i: Int = 0 \
                  mut j: Int = 0 \
                  acc: Terminates(n - j) = while i < n { i = i + 1; j = i; j } \
                  return acc }";
    let measure = ConstExpr::BinOp {
        op: BinOp::Sub,
        left: Box::new(ConstExpr::NamedVar("n".to_string())),
        right: Box::new(ConstExpr::NamedVar("j".to_string())),
    };
    let measures = [("acc", measure)];

    // Act
    let obligations = measure_obligations_of(source, &measures);

    // Assert
    assert!(
        obligations.is_empty(),
        "赋值间相互引用需同时代入语义，逐个代入次序错；实际: {obligations:?}"
    );
}

/// RFC-027a §良基性 —— 循环测度的下界来自**循环条件**（真实 Z3）。
///
/// 循环 `while i < n` 的守卫 `i < n` 导出 `n - i > 0`，故测度 `n - i` 良基。
/// 这条假设来自**回边义务的守卫**，而不是形参精化——本用例特意不给任何形参
/// 精化（`param_assumptions` 为空），于是它唯一钉住的就是「守卫进良基性假设」
/// 这一步：去掉那一步，本用例的判定会退成 `NotProved`（下界推不出）。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_real_solver_loop_measure_well_founded_from_loop_guard() {
    use crate::frontend::core::typecheck::proof::smt::backend::default_solver;

    // Arrange
    let source = "loop: (n: Int) -> Int = { \
                  mut i: Int = 0 \
                  acc: Terminates(n - i) = while i < n { i = i + 1; i } \
                  return acc }";
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act —— 刻意不给形参前置条件
    let verdicts =
        well_founded_verdict_of(source, &[("acc", measure_n_minus_i())], &[], Some(solver));

    // Assert
    let verdict = verdicts
        .iter()
        .find(|(name, _)| name == "acc")
        .map(|(_, v)| v.clone())
        .expect("测度表含 `acc`，良基性判定须含同名条目");
    assert_eq!(
        verdict,
        MeasureVerdict::Proved,
        "循环守卫 `i < n` 应导出 `n - i > 0` ⇒ 良基；若判定退为 NotProved，\
         说明守卫未进良基性假设；实际: {verdicts:?}"
    );
}
