//! 死代码分析测试 — 基于语言规范 §3 & RFC-011 §7
//!
//! §3: 类型系统
//! RFC-011 §7: 死代码消除机制
//! RFC-029g: `pub` 关键字已删除；未引用的顶层绑定一律参与判定，
//! 唯一豁免口是包内引用池（RFC-029f，跨文件消费者不可见）

use crate::frontend::core::typecheck::passes::dead_code::{DeadCodeAnalyzer, DeadCodeWarning};
use crate::frontend::core::parser::ast::{Block, Module, Stmt, StmtKind, Expr, Type};
use crate::util::span::Span;

/// 私有类型 `Outer { inner: Inner }`——字段类型引用私有类型 Inner。
fn outer_with_inner_field() -> Stmt {
    use crate::frontend::core::parser::ast::{StructField, TypeBodyItem};
    Stmt {
        kind: StmtKind::TypeDefinition {
            name: "Outer".to_string(),
            signature_params: vec![],
            definition: Type::Struct {
                body: vec![TypeBodyItem::Field(StructField::new(
                    "inner".to_string(),
                    false,
                    Type::Name {
                        name: "Inner".to_string(),
                        span: Span::dummy(),
                    },
                ))],
            },
        },
        span: Span::dummy(),
    }
}

/// `main = (p: Point) => {}`——Point 仅出现在参数类型注解位置。
fn module_with_point_param_annotation() -> Module {
    Module {
        items: vec![Stmt {
            kind: StmtKind::Assign {
                target: Box::new(Expr::Var("main".to_string(), Span::dummy())),
                type_annotation: None,
                signature_params: vec![],
                value: Some(Box::new(Expr::Lambda {
                    params: vec![crate::frontend::core::parser::ast::Param {
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
        }],
        span: Span::dummy(),
    }
}

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
            is_mut: false,
            span: Span::dummy(),
        },
        span: Span::dummy(),
    }
}

/// 构造一个类型定义语句（`Name: Type = ...` 的真实语法形态，#321）
/// definition 用原生类型——不引用自身（自引用会被收集成使用，污染判定）
fn make_type_def(name: &str) -> Stmt {
    Stmt {
        kind: StmtKind::TypeDefinition {
            name: name.to_string(),
            signature_params: vec![],
            definition: Type::Int(64),
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
        items: vec![make_binding("main", None, vec![])],
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
        items: vec![make_binding("main", None, vec![])],
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

#[test]
fn test_compute_reachability_from_entry_point() {
    // Arrange: main 引用了 helper，两者都应可达
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", None, vec![make_call_stmt("helper")]),
            make_binding("helper", None, vec![]),
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
        items: vec![make_binding("dead_fn", None, vec![])],
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
            make_binding("main", None, vec![make_call_stmt("helper")]),
            make_binding("helper", None, vec![]),
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
        items: vec![make_type_def("DeadType")],
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
        items: vec![make_binding("render", Some("Widget"), vec![])],
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
fn test_unreferenced_defs_all_report() {
    // Arrange: 函数 / 类型 / 方法都无引用——RFC-029g 后没有任何豁免口
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("orphan_fn", None, vec![]),
            make_type_def("OrphanType"),
            make_binding("orphan_method", Some("OrphanType"), vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    let codes: Vec<&str> = warnings.iter().map(|w| w.code.as_str()).collect();
    assert!(
        codes.contains(&"W1001") && codes.contains(&"W1002") && codes.contains(&"W1005"),
        "函数 / 类型 / 方法三类未引用定义都应报告，实际: {:?}",
        warnings
    );
}

#[test]
fn test_referenced_variable_is_not_reported() {
    // Arrange: 变量被 main 引用 → 可达，不报（RFC-029g 后判定只看引用）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", None, vec![make_call_stmt("used_var")]),
            make_var("used_var"),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .all(|w| !(w.code == "W1004" && w.message.contains("used_var"))),
        "被引用的变量不应报 W1004，实际: {:?}",
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
fn test_mutual_reference_functions_reachable() {
    // Arrange: 函数 A 调用 B，B 调用 A，两者都通过 main 引用可达
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            // main 调用 func_a
            make_binding("main", None, vec![make_call_stmt("func_a")]),
            // func_a 调用 func_b
            make_binding("func_a", None, vec![make_call_stmt("func_b")]),
            // func_b 调用 func_a
            make_binding("func_b", None, vec![make_call_stmt("func_a")]),
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
fn test_unused_type_is_not_an_entry_point() {
    // Arrange: 未被引用的类型定义不再是可达根（RFC-029g 删除 pub 后无豁免）
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_type_def("UnusedType")],
        span: Span::dummy(),
    };
    analyzer.collect_entry_points_and_definitions(&ast);

    // Act
    let reachable = analyzer.compute_reachability(&ast);

    // Assert
    assert!(
        !reachable.contains("UnusedType"),
        "RFC-029g 后类型定义不因修饰符成为可达根，实际可达集: {:?}",
        reachable
    );
}

#[test]
fn test_method_binding_no_function_warning() {
    // Arrange: pub 方法绑定（Type.method）是对外接口，不应作为普通函数被报告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("render", Some("Widget"), vec![])],
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

// 引用收集完备性（#321 自查：漏收位置会造成新语义下的误报）

/// 构造 match 表达式语句：`match scrutinee { Name {..} => body }`
fn make_match_stmt(
    scrutinee: &str,
    pattern_type: &str,
    body_stmts: Vec<Stmt>,
) -> Stmt {
    use crate::frontend::core::parser::ast::{MatchArm, Pattern};
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Match {
            expr: Box::new(Expr::Var(scrutinee.to_string(), Span::dummy())),
            arms: vec![MatchArm {
                pattern: Pattern::Struct {
                    name: pattern_type.to_string(),
                    fields: vec![],
                },
                body: Block {
                    stmts: body_stmts,
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        })),
        span: Span::dummy(),
    }
}

/// 构造 spawn 表达式语句：`spawn { body }`
fn make_spawn_stmt(body_stmts: Vec<Stmt>) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Spawn {
            body: Box::new(Block {
                stmts: body_stmts,
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        })),
        span: Span::dummy(),
    }
}

/// 构造索引赋值语句：`target[...] = value`（无求值语义，仅引用形状）
fn make_index_assign_stmt(
    target: &str,
    value_var: &str,
) -> Stmt {
    Stmt {
        kind: StmtKind::Assign {
            target: Box::new(Expr::Index {
                expr: Box::new(Expr::Var(target.to_string(), Span::dummy())),
                index: Box::new(Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(0),
                    Span::dummy(),
                )),
                span: Span::dummy(),
            }),
            type_annotation: None,
            signature_params: vec![],
            value: Some(Box::new(Expr::Var(value_var.to_string(), Span::dummy()))),
            is_mut: false,
            span: Span::dummy(),
        },
        span: Span::dummy(),
    }
}

#[test]
fn test_type_referenced_in_match_pattern_is_alive() {
    // Arrange: 私有类型 Color 仅在 match 模式（Struct 模式头）中被引用
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_type_def("Color"),
            make_binding("main", None, vec![make_match_stmt("c", "Color", vec![])]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.iter().all(|w| !w.message.contains("Color")),
        "match 模式头引用的私有类型不应报 W1002，实际: {:?}",
        warnings
    );
}

#[test]
fn test_fn_referenced_in_spawn_body_is_alive() {
    // Arrange: 私有函数仅在 spawn 体中被调用
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("worker", None, vec![]),
            make_binding(
                "main",
                None,
                vec![make_spawn_stmt(vec![make_call_stmt("worker")])],
            ),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.iter().all(|w| !w.message.contains("worker")),
        "spawn 体中引用的私有函数不应报 W1001，实际: {:?}",
        warnings
    );
}

#[test]
fn test_var_written_via_index_assign_is_alive() {
    // Arrange: 顶层私有变量仅通过 `counts[i] = v` 复合目标写入使用
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_var("counts"),
            make_binding("main", None, vec![make_index_assign_stmt("counts", "item")]),
            make_var("item"),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.iter().all(|w| !w.message.contains("counts")),
        "经索引赋值写入的私有变量不应报 W1004，实际: {:?}",
        warnings
    );
}

#[test]
fn test_only_dead_root_reported_not_its_callees() {
    // Arrange: 有入口点时采用扁平可达性近似——死函数 ghost 调用 helper，
    // helper 被 AST 引用即视为可达（宁漏报不误报），只报死代码根 ghost
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", None, vec![]),
            make_binding("ghost", None, vec![make_call_stmt("helper")]),
            make_binding("helper", None, vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1001" && w.message.contains("ghost")),
        "未被引用的 ghost 应报 W1001，实际: {:?}",
        warnings
    );
    assert!(
        warnings.iter().all(|w| !w.message.contains("helper")),
        "有入口点时仅被死代码引用的 helper 视为可达（扁平近似只报根），实际: {:?}",
        warnings
    );
}

#[test]
fn test_no_entry_point_all_unused_private_defs_dead() {
    // Arrange: 模块无 main 且无 pub 定义——不存在任何可达根，
    // 全部私有定义自外部视角均不可达，传递性死代码整链报告
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("ghost", None, vec![make_call_stmt("helper")]),
            make_binding("helper", None, vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    let codes: Vec<&str> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1001" && w.message.contains("ghost")),
        "无入口点时 ghost 应报 W1001，实际: {:?}",
        codes
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1001" && w.message.contains("helper")),
        "无入口点时仅被死代码引用的 helper 亦为传递性死代码，应报 W1001，实际: {:?}",
        codes
    );
}

#[test]
fn test_private_type_used_in_type_body_is_alive() {
    // Arrange: 私有类型 Inner 仅在私有类型 Outer 的结构体字段类型中被引用
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_type_def("Inner"),
            outer_with_inner_field(),
            make_binding("main", None, vec![make_call_stmt("Outer")]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.iter().all(|w| !w.message.contains("Inner")),
        "被 Outer 字段类型引用的私有类型 Inner 不应报 W1002，实际: {:?}",
        warnings
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

// RFC-029g：`pub` 删除后，死码豁免只剩包内引用池一道 —— 角色不再改写 pub 政策

#[test]
fn test_unused_top_level_function_is_reported() {
    // Arrange: 无引用池（Script 视角）——main 之外的未引用函数无处豁免
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", None, vec![]),
            make_binding("dead_fn", None, vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1001" && w.message.contains("dead_fn")),
        "未使用的顶层函数应报 W1001，实际: {:?}",
        warnings
    );
}

#[test]
fn test_script_role_reports_unused_top_level_variable() {
    // Arrange: Script 角色（无跨文件引用池）——RFC-029g 后变量绑定同样不豁免
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", None, vec![]),
            make_var("dead_top_level_var"),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1004" && w.message.contains("dead_top_level_var")),
        "RFC-029g 后 Script 角色未使用的顶层变量应报 W1004（不再有豁免），实际: {:?}",
        warnings
    );
}

#[test]
fn test_used_top_level_function_is_not_reported() {
    // Arrange: 被 main 引用的顶层函数可达，不报
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", None, vec![make_call_stmt("used_fn")]),
            make_binding("used_fn", None, vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings.is_empty(),
        "被引用的顶层函数不应报，实际: {:?}",
        warnings
    );
}

#[test]
fn test_unused_type_definition_is_reported() {
    // Arrange: 未使用的类型定义没有修饰符可豁免 → W1002
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![
            make_binding("main", None, vec![]),
            make_type_def("DeadType"),
        ],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1002" && w.message.contains("DeadType")),
        "未使用的类型定义应报 W1002，实际: {:?}",
        warnings
    );
}

// RFC-029f：包内引用池是唯一豁免口（跨文件消费者在本文件不可见，宁漏报）

use std::collections::HashSet as StdHashSet;

#[test]
fn test_project_refs_hit_exempts_definition() {
    // Arrange: 提供包内引用池且命中该定义
    let mut analyzer = DeadCodeAnalyzer::new();
    analyzer.set_project_refs(StdHashSet::from(["used_across_files".to_string()]));
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

#[test]
fn test_project_refs_miss_reports_definition() {
    // Arrange: 引用池存在但未命中该定义 → 报
    let mut analyzer = DeadCodeAnalyzer::new();
    analyzer.set_project_refs(StdHashSet::from(["other_name".to_string()]));
    let ast = Module {
        items: vec![make_binding("orphan_fn", None, vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1001" && w.message.contains("orphan_fn")),
        "引用池未命中的定义应报 W1001，实际: {:?}",
        warnings
    );
}

#[test]
fn test_no_project_refs_keeps_file_local_judgement() {
    // Arrange: 单文件路径（无引用池）——RFC-029g 后不再有绝对豁免
    let mut analyzer = DeadCodeAnalyzer::new();
    let ast = Module {
        items: vec![make_binding("orphan_fn", None, vec![])],
        span: Span::dummy(),
    };

    // Act
    let warnings = analyzer.analyze(&ast);

    // Assert
    assert!(
        warnings
            .iter()
            .any(|w| w.code == "W1001" && w.message.contains("orphan_fn")),
        "无引用池时未引用的顶层绑定应报 W1001，实际: {:?}",
        warnings
    );
}
