//! 终止检查器单元测试
//!
//! 规范来源：
//! - RFC-027《编译期谓词与统一静态验证》§7「循环终止性证明」
//!   - §7.2 策略 1：线性秩函数自动合成（SMT 验证）—— 双变量形状与健全性边界
//!   - §7.3 策略 2：谓词违反计数（框架占位）
//!   - §7.5 策略 4：乘法缩放度量
//! - 语言规范 §控制流「while 循环」

use crate::frontend::core::sema::layers::termination::{MeasureVerdict, TerminationChecker};
use crate::frontend::core::sema::proof::verdict::{ProofFailure, ProofResult, ProofResultExt};
use crate::ast::{BinOp, Block, Expr, Literal, PlanId, Stmt, StmtKind};
use crate::frontend::core::types::const_data::{BinOp as ConstBinOp, ConstExpr};
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
        plan_id: PlanId::UNPLANNED,
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
        plan_id: PlanId::UNPLANNED,
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
        plan_id: PlanId::UNPLANNED,
    };
    let module = crate::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::sema::environment::TypeEnvironment::new();
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
        plan_id: PlanId::UNPLANNED,
    };
    let module = crate::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::sema::environment::TypeEnvironment::new();
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
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(expr.clone())),
        span: dummy_span(),
        plan_id: PlanId::UNPLANNED,
    };
    let module = crate::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::sema::environment::TypeEnvironment::new();
    let vars: std::collections::HashSet<String> = refined.iter().map(|s| s.to_string()).collect();
    // 桩求解器与 always_unsat() 是同一个：复用，而不是在这里再定义一遍。
    // D58 后检查器只**借用**求解器：桩的生存期须覆盖 checker，故先绑定再借。
    let solver = always_unsat();
    let mut checker = TerminationChecker::new()
        .with_solver(Some(solver.as_ref()))
        .set_refined_vars(vars);
    checker.check_module(&module, &env)
}

/// `ConstExpr` 命名变量的构造捷径——测度夹具里反复出现 `NamedVar(名字.to_string())`。
fn named_var(name: &str) -> ConstExpr {
    ConstExpr::NamedVar(name.to_string())
}

/// 把 `&[(&str, V)]` 夹具表转成 checker 需要的拥有型 `HashMap<String, V>`。
///
/// 测度表（RFC-027a §2）与形参假设表（§良基性）共用这一形状。
fn owned_table<V: Clone>(entries: &[(&str, V)]) -> std::collections::HashMap<String, V> {
    entries
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect()
}

/// 构造注入了测度表、形参假设与（可选）求解器的终止检查器。
#[cfg(not(target_arch = "wasm32"))]
fn checker_with_measures<'a>(
    measures: &[(&str, ConstExpr)],
    param_assumptions: &[(&str, Vec<ConstExpr>)],
    solver: Option<&'a dyn crate::frontend::core::sema::proof::smt::backend::Solver>,
) -> TerminationChecker<'a> {
    // D58：求解器是共享实例的借用（生命周期参数化），不再注入所有权——
    // 注入所有权要求每个消费点各自新建实例，正是本任务要消除的形态。
    TerminationChecker::new()
        .set_measures(owned_table(measures))
        .set_param_assumptions(owned_table(param_assumptions))
        .with_solver(solver)
}

/// 解析源码 → 断言解析无错 → 返回模块；词法与语法失败都带上下文 panic。
fn checked_module(source: &str) -> crate::ast::Module {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse;

    let tokens = tokenize(source).unwrap_or_else(|e| panic!("词法分析应成功: {e:?}"));
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析应无错误: {:?}", parsed.errors);
    parsed.module
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
        plan_id: PlanId::UNPLANNED,
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
        matches!(&results[0], Err(ProofFailure::Unproven { .. })),
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
        plan_id: PlanId::UNPLANNED,
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
        plan_id: PlanId::UNPLANNED,
    };
    let while_expr = make_while(make_lt_condition("i", "n"), vec![body_stmt]);

    let results = run_check_with_refined(&while_expr, &["i"]);
    assert!(
        !results.is_empty(),
        "Expected unproven result for loop with no progress"
    );
    assert!(
        matches!(&results[0], Err(ProofFailure::Unproven { .. })),
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
        matches!(&results[0], Err(ProofFailure::Unproven { .. })),
        "Expected Unproven, got: {:?}",
        results[0]
    );
}

// ==================== 测试：for 循环 ====================

// ==================== 测试：嵌套循环 ====================

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
        plan_id: PlanId::UNPLANNED,
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

// ==================== 策略 1：线性秩函数自动合成（RFC-027 §7.2）====================

/// 策略 1：`i < j { i += 1; j -= 1 }` 由「边界自己也在动」的线性秩函数拿下
///
/// 该形状此前被两处缺陷挡住（#377），修法即本用例钉住的契约：
///
/// - **(a) 输入曾被清空**：`bound_is_loop_invariant` 过滤会剔除「在循环体内被赋值
///   的边界变量」，而需要秩函数的形状，其边界按定义就在体内被改 ⇒ 候选恒零。
///   修法：策略 1 改吃**未过滤**的边界集合，健全性改由逐赋值验证承担。
/// - **(b) 判定符号曾相反**：旧 `verify_rank_candidate` 把 `m'` 写成 `v + delta`
///   （`delta` 恒 `+1`）再断言 `not (m' < m)` ⇒ 恒可满足 ⇒ 恒验证失败。修法：
///   按 Direction 推导测度变化——`m = bound - v` ⇒ `Δm = δbound - δv`
///   （本例 `δi = +1`、`δj = -1` ⇒ `Δm = -1 - 1 = -2 < 0`）。
///
/// 用恒返回 `Unsat`（无条件认可「候选严格递减」）的桩求解器，使本用例不依赖
/// 本机 Z3、CI 上稳定。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_strategy1_proves_dual_variable_loop() {
    // Arrange — i < j { i += 1; j -= 1 }：边界 j 在循环体内被赋值
    let while_expr = make_while(
        make_lt_condition("i", "j"),
        vec![make_increment("i", 1), make_decrement("j", 1)],
    );

    // Act — 声明 i/j 已精化，使该循环进 §7 验证模式（否则测的是门控而非策略 1）
    let results = run_check_with_stub_solver_refined(&while_expr, &["i", "j"]);

    // Assert
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "`i < j {{ i += 1; j -= 1 }}` 的秩函数 `j - i` 每轮减 2，应被证明终止；
         仍有未证明：{unproven:?}（results={results:?}）"
    );
}

// ==================== 策略 1b：标志循环（guarded exit）====================

// ==================== 策略 1b 健全性：守卫块重绑 / 未知重绑定 ====================

// ==================== 测试：显式测度注入（RFC-027a §2，T2）====================

/// RFC-027a §2 —— 注入的显式测度须可按被标注名精确取回。
///
/// T2 的交付是「测度可提取、可注入」；本用例锁住注入侧的契约：键是被标注的
/// 绑定/函数名，值是测度的无损 `ConstExpr`。**复合测度不得被压成单变量**——
/// 压扁后 SMT 会拿到错误的测度（`n - i` 变成 `n`），义务生成随之错误。
#[test]
fn test_injected_explicit_measures_are_retrievable() {
    // Arrange —— 测度表：单变量 `b` 与复合 `n - i`
    let measures = [("gcd", named_var("b")), ("acc", measure_n_minus_i())];

    // Act
    let checker = TerminationChecker::new().set_measures(owned_table(&measures));

    // Assert
    assert_eq!(
        checker.measures().get("gcd"),
        Some(&named_var("b")),
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

// ==================== 测试：测度义务的 SMT 判定（RFC-027a §判定管线，T4）====================

/// 解析源码 → 注入测度 → 可选注入求解器 → 跑终止检查 → 取判定结果
#[cfg(not(target_arch = "wasm32"))]
fn measure_verdicts_of(
    source: &str,
    measures: &[(&str, ConstExpr)],
    solver: Option<&dyn crate::frontend::core::sema::proof::smt::backend::Solver>,
) -> Vec<MeasureVerdict> {
    measure_verdicts_with_assumptions_of(source, measures, &[], solver)
}

/// 解析源码 → 装配（测度表 + 前置条件 + 可选求解器）→ 跑一遍终止检查。
///
/// 「递减判定」与「良基性判定」只差最后读哪张结果表，装配过程完全一致。
#[cfg(not(target_arch = "wasm32"))]
fn run_measure_checker<'a>(
    source: &str,
    measures: &[(&str, ConstExpr)],
    param_assumptions: &[(&str, Vec<ConstExpr>)],
    solver: Option<&'a dyn crate::frontend::core::sema::proof::smt::backend::Solver>,
) -> TerminationChecker<'a> {
    use crate::frontend::core::sema::environment::TypeEnvironment;

    let module = checked_module(source);
    // 装配这一步与 checker_with_measures 是同一件事：复用，不另写一份。
    let mut checker = checker_with_measures(measures, param_assumptions, solver);
    checker.check_module(&module, &TypeEnvironment::new());
    checker
}

/// 同 [`measure_verdicts_of`]，但注入函数前置条件（形参精化）。
///
/// 递减判定需要它：`a % b < b` 仅在 `b > 0` 时成立，而 `b > 0` 来自
/// 前置条件 `b >= 0` 与路径守卫 `b != 0` 两者。只给守卫不给前置条件时，
/// b 可取负（`mod` 取负号）→ 判不出。
#[cfg(not(target_arch = "wasm32"))]
fn measure_verdicts_with_assumptions_of(
    source: &str,
    measures: &[(&str, ConstExpr)],
    param_assumptions: &[(&str, Vec<ConstExpr>)],
    solver: Option<&dyn crate::frontend::core::sema::proof::smt::backend::Solver>,
) -> Vec<MeasureVerdict> {
    run_measure_checker(source, measures, param_assumptions, solver)
        .measure_verdicts()
        .to_vec()
}

/// 恒返回 `Unsat` 的桩（语义：所有取值下测度都严格递减）
#[cfg(not(target_arch = "wasm32"))]
fn always_unsat() -> Box<dyn crate::frontend::core::sema::proof::smt::backend::Solver> {
    use crate::frontend::core::sema::proof::smt::ast::{SMTCommand, SMTResult};
    use crate::frontend::core::sema::proof::smt::backend::Solver;

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
fn always_sat() -> Box<dyn crate::frontend::core::sema::proof::smt::backend::Solver> {
    use crate::frontend::core::sema::proof::smt::ast::{SMTCommand, SMTModel, SMTResult};
    use crate::frontend::core::sema::proof::smt::backend::Solver;

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
    // 桩求解器先绑定：检查器只**借用**求解器（D58 生命周期参数化）
    let solver = always_unsat();

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver.as_ref()));

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
    // 桩求解器先绑定：检查器只**借用**求解器（D58 生命周期参数化）
    let solver = always_sat();

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver.as_ref()));

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
    use crate::frontend::core::sema::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange
    let source = "f: (n: Int) -> Terminates(n) = { \
                  if n == 0 { return 0 } \
                  return f(n - 1) }";
    let measures = [("f", ConstExpr::NamedVar("n".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver.as_ref()));

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
    use crate::frontend::core::sema::proof::smt::backend::default_solver;
    use crate::frontend::core::types::const_data::ConstExpr;

    // Arrange — f(n + 1)：n + 1 < n 有反例
    let source = "f: (n: Int) -> Terminates(n) = { \
                  if n == 0 { return 0 } \
                  return f(n + 1) }";
    let measures = [("f", ConstExpr::NamedVar("n".to_string()))];
    let solver = default_solver().expect("本用例需要 Z3（default_solver）");

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver.as_ref()));

    // Assert
    assert!(
        matches!(verdicts.as_slice(), [MeasureVerdict::Disproved { .. }]),
        "n + 1 < n 存在反例，真实 Z3 应判为**判伪**（而非仅「未证明」）；\
         实际: {verdicts:?}"
    );
}

/// 良基性判定的 harness（RFC-027a §良基性）。
///
/// 与 [`measure_verdicts_of`] 并列：那条判「递减」，这条判「测度落在自然数上」
/// （`m >= 0`）。参数假设即函数前置条件（`b: NonNegative(b)` → `b >= 0`）。
#[cfg(not(target_arch = "wasm32"))]
/// 良基性判定的 harness（RFC-027a §良基性）。
///
/// 与 [`measure_verdicts_of`] 并列：那条判「递减」，这条判「测度落在自然数上」
/// （`m >= 0`）。参数假设即函数前置条件（`b: NonNegative(b)` → `b >= 0`）。
#[cfg(not(target_arch = "wasm32"))]
fn well_founded_verdict_of(
    source: &str,
    measures: &[(&str, ConstExpr)],
    param_assumptions: &[(&str, Vec<ConstExpr>)],
    solver: Option<&dyn crate::frontend::core::sema::proof::smt::backend::Solver>,
) -> Vec<(String, MeasureVerdict)> {
    let checker = run_measure_checker(source, measures, param_assumptions, solver);
    let mut out: Vec<(String, MeasureVerdict)> = checker
        .well_founded_verdicts()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
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

/// 恒返回 `Unknown` 的桩（语义：求解器判不了，**不等于**义务为假）
#[cfg(not(target_arch = "wasm32"))]
fn always_unknown() -> Box<dyn crate::frontend::core::sema::proof::smt::backend::Solver> {
    use crate::frontend::core::sema::proof::smt::ast::{SMTCommand, SMTResult};
    use crate::frontend::core::sema::proof::smt::backend::Solver;

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
    // 桩求解器先绑定：检查器只**借用**求解器（D58 生命周期参数化）
    let solver = always_unknown();

    // Act
    let verdicts = measure_verdicts_of(source, &measures, Some(solver.as_ref()));

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
    use crate::frontend::core::sema::environment::TypeEnvironment;
    use crate::frontend::core::sema::proof::smt::backend::default_solver;

    // Arrange —— 测度 b：有下界精化（良基性可证），但递增实参 `b + 1` 使 `b + 1 < b` 判伪
    let source = "NonNegative: (x: Int) -> Type = { x >= 0 }\n\
                  f: (n: Int, b: NonNegative(b)) -> Terminates(b) = { \
                  if b == 0 { return n } \
                  return f(n, b + 1) }";
    let solver = default_solver().expect("本用例需要 Z3");
    let mut checker = checker_with_measures(
        &[("f", named_var("b"))],
        &[("f", nonnegative_b())],
        Some(solver.as_ref()),
    );
    let module = checked_module(source);

    // Act
    let results = checker.check_module(&module, &TypeEnvironment::new());

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

// ==================== WBS 3.4.2：Unjudged 义务计数（求解器缺失的静默通道）====================
// 规范来源：RFC-027a §义务生成 + 02-stage-contract §41-52「失败不产生信号」类缺陷。
// T4 现状「只判定并记录，不发射诊断」——count_unjudged_obligations 是信号取出口，
// 计数 > 0 时 checker 发射 W1081。

#[test]
fn test_count_unjudged_obligations_counts_both_kinds() {
    use crate::frontend::core::sema::layers::termination::count_unjudged_obligations;

    // Arrange: 递减判定 2 条（1 Unjudged + 1 Proved）+ 良基性 2 条（1 Unjudged + 1 NotProved）
    let measure = vec![MeasureVerdict::Unjudged, MeasureVerdict::Proved];
    let mut well_founded = std::collections::HashMap::new();
    well_founded.insert("f".to_string(), MeasureVerdict::Unjudged);
    well_founded.insert("g".to_string(), MeasureVerdict::NotProved);

    // Act
    let count = count_unjudged_obligations(&measure, &well_founded);

    // Assert
    assert_eq!(count, 2, "两条 Unjudged（递减 1 + 良基性 1）必须都计入");
}
