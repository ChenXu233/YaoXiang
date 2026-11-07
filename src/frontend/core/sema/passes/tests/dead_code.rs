//! 死代码分析测试 — 基于语言规范 §3 & RFC-011 §7
//!
//! §3: 类型系统
//! RFC-011 §7: 死代码消除机制
//! RFC-029g: `pub` 关键字已删除；未引用的顶层绑定一律参与判定，
//! 唯一豁免口是包内引用池（RFC-029f，跨文件消费者不可见）

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::passes::dead_code::{DeadCodeAnalyzer, DeadCodeWarning};
use crate::ast::{Block, Module, PlanId, Stmt, StmtKind, Expr, Type};
use crate::util::span::Span;

/// `main = (p: Point) => {}`——Point 仅出现在参数类型注解位置。
fn module_with_point_param_annotation() -> Module {
    Module {
        items: vec![Stmt {
            kind: StmtKind::Assign {
                target: Box::new(Expr::Var("main".to_string(), Span::dummy())),
                type_annotation: None,
                value: Some(Box::new(Expr::Lambda {
                    params: vec![crate::ast::Param {
                        name: "p".to_string(),
                        ty: Some(Type::Name {
                            name: "Point".to_string(),
                            span: Span::dummy(),
                        }),
                        is_mut: false,
                        span: Span::dummy(),
                    }],
                    body: Box::new(Block {
                        stmts: vec![],
                        span: Span::dummy(),
                    }),
                    span: Span::dummy(),
                })),
                is_mut: false,
                span: Span::dummy(),
            },
            span: Span::dummy(),
            plan_id: PlanId::UNPLANNED,
        }],
        span: Span::dummy(),
    }
}

/// 构造一个 Assign 语句（函数 / 方法）
fn make_binding(
    name: &str,
    type_name: Option<&str>,
    body_stmts: Vec<Stmt>,
) -> Stmt {
    let target = if let Some(tn) = type_name {
        Box::new(Expr::FieldAccess {
            expr: Box::new(Expr::Var(tn.to_string(), Span::dummy())),
            field: name.to_string(),
            span: Span::dummy(),
        })
    } else {
        Box::new(Expr::Var(name.to_string(), Span::dummy()))
    };
    Stmt {
        kind: StmtKind::Assign {
            target,
            type_annotation: None,
            value: Some(Box::new(Expr::Lambda {
                params: vec![],
                body: Box::new(Block {
                    stmts: body_stmts,
                    span: Span::dummy(),
                }),
                span: Span::dummy(),
            })),
            is_mut: false,
            span: Span::dummy(),
        },
        span: Span::dummy(),
        plan_id: PlanId::UNPLANNED,
    }
}

// Happy path 测试

#[test]
fn test_dead_code_analyzer_creation() {
    // Arrange & Act
    let _analyzer = DeadCodeAnalyzer::new();

    // Assert - 应该成功创建
}

#[test]
fn test_function_without_reference_is_not_reachable() {
    // Arrange: 没有任何引用的顶层函数不是可达根（RFC-029g 删除 pub 后无豁免）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("unreferenced_fn", None, vec![])],
        span: Span::dummy(),
    };
    analyzer.collect_entry_points_and_definitions(&ast);

    // Act
    let reachable = analyzer.compute_reachability(&ast);

    // Assert
    assert!(
        !reachable.contains("unreferenced_fn"),
        "RFC-029g 后修饰符不再产生可达根，实际可达集: {:?}",
        reachable
    );
}

// Error path 测试

#[test]
fn test_to_diagnostics_converts_warnings() {
    // Arrange
    let analyzer = DeadCodeAnalyzer::new();
    let warnings = vec![DeadCodeWarning {
        code: "W1001".to_string(),
        message: "Unused function: 'foo'".to_string(),
        span: Span::dummy(),
    }];

    // Act
    let diagnostics = analyzer.to_diagnostics(&warnings);

    // Assert
    assert_eq!(diagnostics.len(), 1, "应将 1 个警告转换为 1 个诊断信息");
}

#[test]
fn test_analyze_many_functions() {
    // Arrange: 生成大量函数，验证分析器性能和正确性
    let mut analyzer = DeadCodeAnalyzer::new();
    let items: Vec<Stmt> = (0..100)
        .map(|i| make_binding(&format!("func_{}", i), None, vec![]))
        .collect();
    let ast = Module {
        items,
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert: RFC-029g 后无引用即报——100 个函数各产生一条 W1001
    assert_eq!(
        warnings.len(),
        100,
        "未引用的顶层函数应逐个报告，实际: {:?}",
        warnings.len()
    );
}

#[test]
fn test_collect_ident_refs_covers_param_type_annotations() {
    // Arrange: `main = (p: Point) => p`——Point 出现在参数类型注解位置，
    // 普通 Var 走查不可见，collect_ident_refs 必须覆盖
    //（W1003 依赖此兜底：导入类型仅用于注解时不误报）
    let ast = module_with_point_param_annotation();

    // Act
    let refs = DeadCodeAnalyzer::collect_ident_refs(&ast);

    // Assert
    assert!(
        refs.contains("Point"),
        "参数类型注解中的类型名应被收集，实际: {:?}",
        refs
    );
    assert!(
        !refs.contains("main"),
        "顶层定义目标名不应被收集为引用（定义不是使用）",
    );
}

// RFC-029f：包内引用池是唯一豁免口（跨文件消费者在本文件不可见，宁漏报）

use std::collections::HashSet as StdHashSet;

#[test]
fn test_project_refs_hit_exempts_definition() {
    // Arrange: 提供包内引用池且命中该定义
    let mut analyzer = DeadCodeAnalyzer::new();
    analyzer.set_cross_file_refs(StdHashSet::from(["used_across_files".to_string()]));
    let ast = Module {
        items: vec![make_binding("used_across_files", None, vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.is_empty(),
        "引用池命中的定义应豁免，实际: {:?}",
        warnings
    );
}

/// 解析源码为模块（语法必须正确，否则测试本身有错）
fn parse_module(source: &str) -> Module {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parse_result = parse(&tokens);
    assert!(
        !parse_result.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parse_result.errors
    );
    parse_result.module
}

/// WBS 3.20.3 / issue #505：算子派发命中的方法组**整组按活**——
/// `u[0]` / `u == t` 的具名决议（`OperatorDispatch`）不产生 D20 重载决议
/// 条目，AST 引用收集里也没有 `index`/`equal` 短名 ⇒ 修复前两方法误报
/// W1005（红态实测）。driver 把派发组喂进来后不得再报。
#[test]
fn test_operator_dispatched_group_keeps_methods_alive_w1005() {
    // Arrange: 类型体内接口声明 + 两方法定义 + main 只写算子表达式
    //（AST 引用里没有 index/equal 短名——派发决议在 typecheck 侧）
    let ast = parse_module(
        r#"
Temp: Type = {
    v: Float,
    Index(Temp, Int, Float),
    Equal(Temp, Temp),
}
Temp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {
    self.v + 1.0
}
Temp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {
    true
}
main = () => {
    u = Temp(19.0)
    println(u[0])
    println(u == u)
}
"#,
    );

    // Act: 不喂派发组（修复前形态）→ 对照；喂派发组 → 修复后形态
    let mut analyzer = DeadCodeAnalyzer::new();
    let before = analyzer.analyze(&ast);
    let mut analyzer = DeadCodeAnalyzer::new();
    analyzer.set_operator_dispatched_groups(StdHashSet::from([
        "Temp.index".to_string(),
        "Temp.equal".to_string(),
    ]));
    let after = analyzer.analyze(&ast);

    // Assert: 修复前两方法各报一条 W1005（红态本身也在断言里钉住）；
    // 喂入派发组后两条全部消失
    let before_hits: Vec<&str> = before
        .iter()
        .filter(|w| w.code == "W1005")
        .map(|w| {
            if w.message.contains("Temp.index") {
                "Temp.index"
            } else {
                "Temp.equal"
            }
        })
        .collect();
    assert!(
        before_hits.contains(&"Temp.index") && before_hits.contains(&"Temp.equal"),
        "无派发组时 index/equal 应各报一条 W1005（红态基线），实际: {before:?}"
    );
    assert!(
        after.iter().all(|w| w.code != "W1005"),
        "算子派发命中的方法组不得报 W1005，实际: {after:?}"
    );
}

/// WBS 3.20.3 / issue #505 守卫（不得放宽判据）：接口声明了、方法定义了，
/// 但**没有**算子派发作用于该类型 ⇒ 两方法仍须如实报 W1005——派发组为空时
/// 判定与修复前完全一致。
#[test]
fn test_operator_interface_declared_without_dispatch_still_reports_w1005() {
    // Arrange: 同构类型与方法，main 只读字段（无 u[0] / u == t）
    let ast = parse_module(
        r#"
Temp: Type = {
    v: Float,
    Index(Temp, Int, Float),
    Equal(Temp, Temp),
}
Temp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {
    self.v + 1.0
}
Temp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {
    true
}
main = () => {
    u = Temp(1.0)
    println(u.v)
}
"#,
    );

    // Act: 派发组为空（接口从未被算子使用）
    let mut analyzer = DeadCodeAnalyzer::new();
    let warnings = analyzer.analyze(&ast);

    // Assert: 两方法照报 W1005
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1005" && w.message.contains("Temp.index")),
        "无算子派发时 Temp.index 仍应报 W1005，实际: {warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1005" && w.message.contains("Temp.equal")),
        "无算子派发时 Temp.equal 仍应报 W1005，实际: {warnings:?}"
    );
}
