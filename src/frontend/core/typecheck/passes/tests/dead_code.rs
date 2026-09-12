//! 死代码分析测试 — 基于语言规范 §3 & RFC-011 §7
//!
//! §3: 类型系统
//! RFC-011 §7: 死代码消除机制
//! #321 定案 B: pub 定义是对外接口永不报；仅私有定义参与死代码判定

use crate::frontend::core::typecheck::passes::dead_code::{DeadCodeAnalyzer, DeadCodeWarning};
use crate::frontend::core::parser::ast::{Block, Module, Stmt, StmtKind, Expr, Type};
use crate::util::span::Span;

// Helpers

/// 构造一个空模块
fn empty_module() -> Module {
    Module {
        items: vec![],
        span: Span::dummy(),
    }
}

/// 构造一个 Assign 语句（函数 / 方法）
fn make_binding(
    name: &str,
    is_pub: bool,
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
            signature_params: vec![],
            value: Some(Box::new(Expr::Lambda {
                params: vec![],
                body: Box::new(Block {
                    stmts: body_stmts,
                    span: Span::dummy(),
                }),
                span: Span::dummy(),
            })),
            is_pub,
            is_mut: false,
            span: Span::dummy(),
        },
        span: Span::dummy(),
    }
}

/// 构造一个类型定义语句（`Name: Type = ...` 的真实语法形态，#321）
fn make_type_def(
    name: &str,
    is_pub: bool,
) -> Stmt {
    Stmt {
        kind: StmtKind::TypeDefinition {
            name: name.to_string(),
            signature_params: vec![],
            definition: Type::Name {
                name: name.to_string(),
                span: Span::dummy(),
            },
            is_pub,
        },
        span: Span::dummy(),
    }
}

/// 构造一个普通变量绑定（无值）
fn make_var(name: &str) -> Stmt {
    Stmt {
        kind: StmtKind::Assign {
            target: Box::new(Expr::Var(name.to_string(), Span::dummy())),
            type_annotation: None,
            signature_params: vec![],
            value: None,
            is_pub: false,
            is_mut: false,
            span: Span::dummy(),
        },
        span: Span::dummy(),
    }
}

/// 构造一个带类型注解的变量绑定（`x: Int`——曾被他构造器启发式误分类为类型，#321）
fn make_annotated_var(name: &str) -> Stmt {
    Stmt {
        kind: StmtKind::Assign {
            target: Box::new(Expr::Var(name.to_string(), Span::dummy())),
            type_annotation: Some(Type::Name {
                name: "Int".to_string(),
                span: Span::dummy(),
            }),
            signature_params: vec![],
            value: None,
            is_pub: false,
            is_mut: false,
            span: Span::dummy(),
        },
        span: Span::dummy(),
    }
}

/// 构造一个引用某符号的 Call 表达式语句
fn make_call_stmt(name: &str) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Call {
            func: Box::new(Expr::Var(name.to_string(), Span::dummy())),
            args: vec![],
            named_args: vec![],
            span: Span::dummy(),
        })),
        span: Span::dummy(),
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
fn test_analyze_empty_module_produces_no_warnings() {
    // Arrange
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = empty_module();

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.is_empty(),
        "空模块不应产生任何死代码警告, 实际: {}",
        warnings.len()
    );
}

#[test]
fn test_analyze_active_function_no_warning() {
    // Arrange: main 函数是入口点且被引用，不应产生警告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("main", true, None, vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.is_empty(),
        "pub main 作为入口点不应产生警告, 实际: {}",
        warnings.len()
    );
}

#[test]
fn test_main_function_is_entry_point_reachable() {
    // Arrange: main 函数作为入口点应始终可达（不产生死代码警告）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("main", false, None, vec![])],
        span: Span::dummy(),
    };

    // Act: 通过 analyze 的公共 API 验证 main 不产生警告
    let warnings = analyzer.analyze(&ast);

    // Assert: main 是入口点，不会出现在死代码警告中
    assert!(
        warnings.iter().all(|w| !w.message.contains("main")),
        "main 作为入口点不应被报告为死代码"
    );
}

#[test]
fn test_pub_function_is_entry_point_reachable() {
    // Arrange: pub 函数是对外接口（#321 定案 B），不应被报告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("public_fn", true, None, vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert: pub 函数作为入口点，不应出现 W1001 警告
    assert!(
        warnings
            .iter()
            .all(|w| w.code != "W1001" || !w.message.contains("public_fn")),
        "pub 函数是对外接口，不应被报告为死代码"
    );
}

#[test]
fn test_compute_reachability_from_entry_point() {
    // Arrange: main 引用了 helper，两者都应可达
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", false, None, vec![make_call_stmt("helper")]),
            make_binding("helper", false, None, vec![]),
        ],
        span: Span::dummy(),
    };
    analyzer.collect_entry_points_and_definitions(&ast);

    // Act
    let reachable = analyzer.compute_reachability(&ast);

    // Assert
    assert!(reachable.contains("main"), "main 应可达");
    assert!(reachable.contains("helper"), "被 main 引用的 helper 应可达");
}

// 私有死代码正向用例（#321 定案 B：仅私有定义报告）

#[test]
fn test_unused_private_function_reports_w1001() {
    // Arrange: 私有函数无任何引用 → W1001
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("dead_fn", false, None, vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1001" && w.message.contains("dead_fn")),
        "未被引用的私有函数应报 W1001，实际: {:?}",
        warnings
    );
}

#[test]
fn test_private_fn_used_by_reachable_fn_no_warning() {
    // Arrange: helper 被 main 引用 → 可达，不报
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", false, None, vec![make_call_stmt("helper")]),
            make_binding("helper", false, None, vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.is_empty(),
        "被 main 引用的私有 helper 不应报 W1001，实际: {:?}",
        warnings
    );
}

#[test]
fn test_unused_private_variable_reports_w1004() {
    // Arrange: 私有变量无任何引用 → W1004
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_var("dead_var")],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1004" && w.message.contains("dead_var")),
        "未被引用的私有变量应报 W1004，实际: {:?}",
        warnings
    );
}

#[test]
fn test_annotated_private_variable_reports_w1004() {
    // Arrange: 带类型注解的私有变量（`dead_var: Int`）也是变量，不是类型——
    // 曾被他构造器启发式误分类进入口点而永不报告（#321 M2 复盘）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_annotated_var("dead_var")],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1004" && w.message.contains("dead_var")),
        "带注解的未引用私有变量应报 W1004，实际: {:?}",
        warnings
    );
}

#[test]
fn test_unused_private_type_reports_w1002() {
    // Arrange: 私有类型定义无任何引用 → W1002
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_type_def("DeadType", false)],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1002" && w.message.contains("DeadType")),
        "未被引用的私有类型应报 W1002，实际: {:?}",
        warnings
    );
}

#[test]
fn test_unused_private_method_reports_w1005() {
    // Arrange: 私有方法绑定无任何引用 → W1005
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("render", false, Some("Widget"), vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1005" && w.message.contains("Widget.render")),
        "未被引用的私有方法应报 W1005，实际: {:?}",
        warnings
    );
}

#[test]
fn test_pub_defs_never_warn() {
    // Arrange: pub 函数/类型/变量/方法都是对外接口，即使"无引用"也不报
    //（对外消费方是否使用超出单文件分析边界，宁静默不误报）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("pub_fn", true, None, vec![]),
            make_type_def("PubType", true),
            make_binding("pub_method", true, Some("PubType"), vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.is_empty(),
        "pub 定义是对外接口，不应报死代码警告，实际: {:?}",
        warnings
    );
}

#[test]
fn test_pub_variable_never_warns() {
    // Arrange: pub 变量是对外接口
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![Stmt {
            kind: StmtKind::Assign {
                target: Box::new(Expr::Var("pub_var".to_string(), Span::dummy())),
                type_annotation: None,
                signature_params: vec![],
                value: None,
                is_pub: true,
                is_mut: false,
                span: Span::dummy(),
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.iter().all(|w| w.code != "W1004"),
        "pub 变量是对外接口，不应报 W1004，实际: {:?}",
        warnings
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

// Boundary 测试

#[test]
fn test_analyze_empty_module_boundary() {
    // Arrange
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = empty_module();

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(warnings.is_empty(), "空模块不应产生任何警告");
}

#[test]
fn test_analyze_many_functions() {
    // Arrange: 生成大量函数，验证分析器性能和正确性
    let mut analyzer = DeadCodeAnalyzer::new();
    let items: Vec<Stmt> = (0..100)
        .map(|i| make_binding(&format!("func_{}", i), true, None, vec![]))
        .collect();
    let ast = Module {
        items,
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert: 所有 pub 函数都是对外接口，不应产生警告
    assert_eq!(warnings.len(), 0, "pub 函数是对外接口，不应产生警告");
}

#[test]
fn test_mutual_reference_functions_reachable() {
    // Arrange: 函数 A 调用 B，B 调用 A，两者都通过 main 引用可达
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            // main 调用 func_a
            make_binding("main", false, None, vec![make_call_stmt("func_a")]),
            // func_a 调用 func_b
            make_binding("func_a", false, None, vec![make_call_stmt("func_b")]),
            // func_b 调用 func_a
            make_binding("func_b", false, None, vec![make_call_stmt("func_a")]),
        ],
        span: Span::dummy(),
    };
    analyzer.collect_entry_points_and_definitions(&ast);

    // Act
    let reachable = analyzer.compute_reachability(&ast);

    // Assert: 三者都应可达
    assert!(reachable.contains("main"), "main 应可达");
    assert!(reachable.contains("func_a"), "被 main 引用的 func_a 应可达");
    assert!(
        reachable.contains("func_b"),
        "被 func_a 引用的 func_b 应可达"
    );
}

#[test]
fn test_pub_type_is_entry_point() {
    // Arrange: pub 类型是对外接口（可达根），不应产生警告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_type_def("PubType", true)],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.iter().all(|w| !w.message.contains("PubType")),
        "pub 类型是对外接口，不应被报告为死代码"
    );
}

#[test]
fn test_method_binding_no_function_warning() {
    // Arrange: pub 方法绑定（Type.method）是对外接口，不应作为普通函数被报告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("render", true, Some("Widget"), vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert: 方法绑定是对外接口，不应产生 W1001 函数警告
    assert!(
        warnings
            .iter()
            .all(|w| w.code != "W1001" || !w.message.contains("render")),
        "pub 方法绑定是对外接口，不应被报告为未使用函数"
    );
}
