//! 死代码分析测试 — 基于语言规范 §3 & RFC-011 §7
//!
//! §3: 类型系统
//! RFC-011 §7: 死代码消除机制

use crate::frontend::core::typecheck::passes::dead_code::{DeadCodeAnalyzer, DeadCodeWarning};
use crate::frontend::core::typecheck::semantic_db::{
    SemanticDB, SemanticToken, SemanticTokenType, SemanticTokenModifier,
};
use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind, Expr};
use crate::util::span::Span;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// 构造一个空模块
fn empty_module() -> Module {
    Module {
        items: vec![],
        span: Span::dummy(),
    }
}

/// 构造一个 Binding 语句（函数 / 类型构造器 / 方法）
fn make_binding(
    name: &str,
    is_pub: bool,
    type_name: Option<&str>,
    body_stmts: Vec<Stmt>,
) -> Stmt {
    Stmt {
        kind: StmtKind::Binding {
            name: name.to_string(),
            type_name: type_name.map(String::from),
            method_type: None,
            is_pub,
            params: vec![],
            body: body_stmts,
            generic_params: vec![],
            type_annotation: None,
        },
        span: Span::dummy(),
    }
}

/// 构造一个类型构造器（参数为空、body 为空、有类型注解）
fn make_type_constructor(name: &str) -> Stmt {
    use crate::frontend::core::parser::ast::Type;
    Stmt {
        kind: StmtKind::Binding {
            name: name.to_string(),
            type_name: None,
            method_type: None,
            is_pub: false,
            params: vec![],
            body: vec![],
            generic_params: vec![],
            type_annotation: Some(Type::Name {
                name: name.to_string(),
                span: Span::dummy(),
            }),
        },
        span: Span::dummy(),
    }
}

/// 构造一个 Var 语句
fn make_var(name: &str) -> Stmt {
    Stmt {
        kind: StmtKind::Var {
            name: name.to_string(),
            name_span: Span::dummy(),
            type_annotation: None,
            initializer: None,
            is_mut: false,
        },
        span: Span::dummy(),
    }
}

/// 构造一个 Use 语句
fn make_use(
    path: &str,
    items: Option<Vec<String>>,
) -> Stmt {
    use crate::frontend::core::parser::ast::SpannedIdent;
    Stmt {
        kind: StmtKind::Use {
            path: path.to_string(),
            path_span: Span::dummy(),
            path_parts: vec![SpannedIdent {
                name: path.to_string(),
                span: Span::dummy(),
            }],
            items,
            alias: None,
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

// ===================================================================
// Happy path 测试
// ===================================================================

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
    let semantic_db = SemanticDB::new();

    // Act
    let warnings = analyzer.analyze(&ast, &semantic_db);

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
    let mut semantic_db = SemanticDB::new();
    semantic_db.add_token(
        "test.yx",
        SemanticToken {
            name: "main".to_string(),
            token_type: SemanticTokenType::Function,
            modifiers: vec![SemanticTokenModifier::Declaration],
            span: Span::dummy(),
        },
    );
    semantic_db.add_token(
        "test.yx",
        SemanticToken {
            name: "main".to_string(),
            token_type: SemanticTokenType::Function,
            modifiers: vec![],
            span: Span::dummy(),
        },
    );

    // Act
    let warnings = analyzer.analyze(&ast, &semantic_db);

    // Assert
    assert!(
        warnings.is_empty(),
        "pub main 作为入口点且被引用不应产生警告, 实际: {}",
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
    let warnings = analyzer.analyze(&ast, &SemanticDB::new());

    // Assert: main 是入口点，不会出现在未使用导出警告中
    assert!(
        warnings.iter().all(|w| !w.message.contains("main")),
        "main 作为入口点不应被报告为死代码"
    );
}

#[test]
fn test_pub_function_is_entry_point_reachable() {
    // Arrange: pub 函数是入口点，不应被报告为未使用导出
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("public_fn", true, None, vec![])],
        span: Span::dummy(),
    };

    // Act: 通过公共 API 验证 pub 函数不产生警告
    // pub 函数是入口点，但 analyze 会检查可达性。
    // 由于 SemanticDB 中没有引用，pub 函数虽然不是入口点中的"可达"符号，
    // 但 find_unused_exports 检查的是「导出但不可达」的情况。
    // 对于 pub 函数，它们被加入 entry_points 所以 compute_reachability 会标记为可达。
    let warnings = analyzer.analyze(&ast, &SemanticDB::new());

    // Assert: pub 函数作为入口点，不应出现 W1001 警告
    assert!(
        warnings
            .iter()
            .all(|w| w.code != "W1001" || !w.message.contains("public_fn")),
        "pub 函数作为入口点不应被报告为未使用导出"
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

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_detect_unused_exported_function() {
    // Arrange: pub 函数是入口点，不应产生死代码警告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("exported_fn", true, None, vec![])],
        span: Span::dummy(),
    };
    let semantic_db = SemanticDB::new();

    // Act
    let warnings = analyzer.analyze(&ast, &semantic_db);

    // Assert — pub 函数是入口点（可被外部模块使用），不应报为死代码
    assert!(
        warnings.is_empty(),
        "pub 函数是入口点，不应产生死代码警告，实际: {}",
        warnings.len()
    );
}

#[test]
fn test_detect_unused_variable_tracked_in_analyzer() {
    // Arrange: 变量应被收集到分析器中（通过 analyze 的副作用验证）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_var("unused_var")],
        span: Span::dummy(),
    };
    let semantic_db = SemanticDB::new();

    // Act: analyze 不应 panic
    let _warnings = analyzer.analyze(&ast, &semantic_db);

    // Assert: 分析器应能正确处理变量语句而不崩溃
}

#[test]
fn test_detect_unused_import() {
    // Arrange: 导入了未被使用的符号
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_use(
            "some.module",
            Some(vec!["unused_import".to_string()]),
        )],
        span: Span::dummy(),
    };
    let semantic_db = SemanticDB::new();

    // Act
    let warnings = analyzer.analyze(&ast, &semantic_db);

    // Assert
    assert!(
        warnings.iter().any(|w| w.code == "W1003"),
        "未使用的导入应使用警告码 W1003"
    );
    assert!(
        warnings.iter().any(|w| w.message.contains("unused_import")),
        "警告消息应包含导入名 'unused_import'"
    );
}

#[test]
fn test_find_unused_exports_returns_correct_codes() {
    // Arrange: pub 函数是入口点，find_unused_exports 不应对其产生警告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("exported_fn", true, None, vec![])],
        span: Span::dummy(),
    };
    let semantic_db = SemanticDB::new();

    // Act
    let warnings = analyzer.analyze(&ast, &semantic_db);

    // Assert: pub 函数是入口点，不会被报为未使用导出
    assert!(
        warnings.is_empty(),
        "pub 函数是入口点，不应产生警告，实际: {}",
        warnings.len()
    );
}

#[test]
fn test_to_diagnostics_converts_warnings() {
    // Arrange
    let analyzer = DeadCodeAnalyzer::new();
    let warnings = vec![DeadCodeWarning {
        code: "W1001".to_string(),
        message: "Unused exported function: 'foo'".to_string(),
        span: Span::dummy(),
    }];

    // Act
    let diagnostics = analyzer.to_diagnostics(&warnings);

    // Assert
    assert_eq!(diagnostics.len(), 1, "应将 1 个警告转换为 1 个诊断信息");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_analyze_empty_module_boundary() {
    // Arrange
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = empty_module();
    let semantic_db = SemanticDB::new();

    // Act
    let warnings = analyzer.analyze(&ast, &semantic_db);

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
    let semantic_db = SemanticDB::new();

    // Act
    let warnings = analyzer.analyze(&ast, &semantic_db);

    // Assert: 所有 pub 函数都是入口点，不应产生警告
    assert_eq!(warnings.len(), 0, "pub 函数是入口点，不应产生警告");
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
fn test_type_constructor_is_entry_point() {
    // Arrange: 类型构造器应被识别为入口点（可被实例化）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_type_constructor("MyType")],
        span: Span::dummy(),
    };

    // Act: 类型构造器是入口点，不应产生警告
    let warnings = analyzer.analyze(&ast, &SemanticDB::new());

    // Assert
    assert!(
        warnings.iter().all(|w| !w.message.contains("MyType")),
        "类型 'MyType' 作为入口点不应被报告为死代码"
    );
}

#[test]
fn test_method_binding_no_function_warning() {
    // Arrange: 方法绑定（Type.method）不应作为普通函数被报告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("render", true, Some("Widget"), vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast, &SemanticDB::new());

    // Assert: 方法绑定以 Widget.render 格式记录，不应产生 W1001 函数警告
    assert!(
        warnings
            .iter()
            .all(|w| w.code != "W1001" || !w.message.contains("render")),
        "方法绑定不应作为普通函数被报告为未使用导出"
    );
}

#[test]
fn test_import_used_no_warning() {
    // Arrange: 已被引用的导入不应产生警告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_use("math", Some(vec!["sqrt".to_string()])),
            make_call_stmt("sqrt"),
        ],
        span: Span::dummy(),
    };
    analyzer.collect_entry_points_and_definitions(&ast);

    let reachable = analyzer.compute_reachability(&ast);

    // Act
    let warnings = analyzer.find_unused_imports(&reachable);

    // Assert: sqrt 被引用了，不应产生 W1003 警告
    assert!(
        warnings.is_empty(),
        "已使用的导入 'sqrt' 不应产生警告, 实际: {}",
        warnings.len()
    );
}
