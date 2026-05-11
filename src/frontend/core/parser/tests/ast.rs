//! Tests for all AST type definitions: Expr, StmtKind, Type, Pattern, operators.

use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast::{
    self, BindingKind, BindingSemanticKind, BinOp, Block, EvalMode, Expr, FStringSegment,
    GenericParam, GenericParamKind, MatchArm, Param, Pattern, SpannedIdent, Stmt, StmtKind,
    StructField, Type, TypeBodyBinding, UnOp, VariantDef, classify_binding_semantic_kind,
    is_meta_type, type_annotation_returns_meta_type,
};
use crate::util::span::Span;

// ============================================================================
// Expr variants
// ============================================================================

#[test]
fn test_expr_lit() {
    let span = Span::dummy();
    assert!(matches!(Expr::Lit(Literal::Int(42), span), Expr::Lit(..)));
    assert!(matches!(
        Expr::Lit(Literal::Float(3.14), span),
        Expr::Lit(..)
    ));
    assert!(matches!(
        Expr::Lit(Literal::String("hi".into()), span),
        Expr::Lit(..)
    ));
    assert!(matches!(
        Expr::Lit(Literal::Bool(true), span),
        Expr::Lit(..)
    ));
    assert!(matches!(Expr::Lit(Literal::Char('a'), span), Expr::Lit(..)));
}

#[test]
fn test_expr_var() {
    let expr = Expr::Var("x".into(), Span::dummy());
    if let Expr::Var(name, _) = &expr {
        assert_eq!(name, "x");
    } else {
        panic!("Expected Expr::Var");
    }
}

#[test]
fn test_expr_binop() {
    let expr = Expr::BinOp {
        op: BinOp::Add,
        left: Box::new(Expr::Lit(Literal::Int(1), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(2), Span::dummy())),
        span: Span::dummy(),
    };
    if let Expr::BinOp { op, .. } = &expr {
        assert_eq!(*op, BinOp::Add);
    } else {
        panic!("Expected Expr::BinOp");
    }
}

#[test]
fn test_expr_unop() {
    let expr = Expr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(Expr::Lit(Literal::Int(1), Span::dummy())),
        span: Span::dummy(),
    };
    if let Expr::UnOp { op, .. } = &expr {
        assert_eq!(*op, UnOp::Neg);
    } else {
        panic!("Expected Expr::UnOp");
    }
}

#[test]
fn test_expr_call() {
    let expr = Expr::Call {
        func: Box::new(Expr::Var("f".into(), Span::dummy())),
        args: vec![Expr::Lit(Literal::Int(1), Span::dummy())],
        named_args: vec![("x".into(), Expr::Lit(Literal::Int(2), Span::dummy()))],
        span: Span::dummy(),
    };
    if let Expr::Call {
        args, named_args, ..
    } = &expr
    {
        assert_eq!(args.len(), 1);
        assert_eq!(named_args.len(), 1);
        assert_eq!(named_args[0].0, "x");
    } else {
        panic!("Expected Expr::Call");
    }
}

#[test]
fn test_expr_if() {
    let expr = Expr::If {
        condition: Box::new(Expr::Lit(Literal::Bool(true), Span::dummy())),
        then_branch: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::dummy(),
        }),
        elif_branches: vec![],
        else_branch: None,
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::If { .. }));
}

#[test]
fn test_expr_match() {
    let expr = Expr::Match {
        expr: Box::new(Expr::Lit(Literal::Int(1), Span::dummy())),
        arms: vec![MatchArm {
            pattern: Pattern::Wildcard,
            body: Block {
                stmts: vec![],
                expr: None,
                span: Span::dummy(),
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };
    if let Expr::Match { arms, .. } = &expr {
        assert_eq!(arms.len(), 1);
    } else {
        panic!("Expected Expr::Match");
    }
}

#[test]
fn test_expr_block() {
    let block = Block {
        stmts: vec![],
        expr: Some(Box::new(Expr::Lit(Literal::Int(1), Span::dummy()))),
        span: Span::dummy(),
    };
    let expr = Expr::Block(block);
    if let Expr::Block(b) = &expr {
        assert!(b.expr.is_some());
    } else {
        panic!("Expected Expr::Block");
    }
}

#[test]
fn test_expr_lambda() {
    let expr = Expr::Lambda {
        params: vec![Param {
            name: "x".into(),
            ty: None,
            is_mut: false,
            span: Span::dummy(),
        }],
        body: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    if let Expr::Lambda { params, .. } = &expr {
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "x");
    } else {
        panic!("Expected Expr::Lambda");
    }
}

#[test]
fn test_expr_list() {
    let expr = Expr::List(
        vec![Expr::Lit(Literal::Int(1), Span::dummy())],
        Span::dummy(),
    );
    if let Expr::List(items, _) = &expr {
        assert_eq!(items.len(), 1);
    } else {
        panic!("Expected Expr::List");
    }
}

#[test]
fn test_expr_tuple() {
    let expr = Expr::Tuple(
        vec![
            Expr::Lit(Literal::Int(1), Span::dummy()),
            Expr::Lit(Literal::String("a".into()), Span::dummy()),
        ],
        Span::dummy(),
    );
    if let Expr::Tuple(items, _) = &expr {
        assert_eq!(items.len(), 2);
    } else {
        panic!("Expected Expr::Tuple");
    }
}

#[test]
fn test_expr_index() {
    let expr = Expr::Index {
        expr: Box::new(Expr::Var("arr".into(), Span::dummy())),
        index: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::Index { .. }));
}

#[test]
fn test_expr_field_access() {
    let expr = Expr::FieldAccess {
        expr: Box::new(Expr::Var("obj".into(), Span::dummy())),
        field: "field".into(),
        span: Span::dummy(),
    };
    if let Expr::FieldAccess { field, .. } = &expr {
        assert_eq!(field, "field");
    } else {
        panic!("Expected Expr::FieldAccess");
    }
}

#[test]
fn test_expr_return() {
    let expr = Expr::Return(
        Some(Box::new(Expr::Lit(Literal::Int(1), Span::dummy()))),
        Span::dummy(),
    );
    assert!(matches!(expr, Expr::Return(Some(..), _)));

    let expr_void = Expr::Return(None, Span::dummy());
    assert!(matches!(expr_void, Expr::Return(None, _)));
}

#[test]
fn test_expr_try() {
    let expr = Expr::Try {
        expr: Box::new(Expr::Var("x".into(), Span::dummy())),
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::Try { .. }));
}

#[test]
fn test_expr_ref() {
    let expr = Expr::Ref {
        expr: Box::new(Expr::Var("x".into(), Span::dummy())),
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::Ref { .. }));
}

#[test]
fn test_expr_unsafe() {
    let expr = Expr::Unsafe {
        body: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::Unsafe { .. }));
}

#[test]
fn test_expr_spawn() {
    let expr = Expr::Spawn {
        body: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::Spawn { .. }));
}

#[test]
fn test_expr_fstring() {
    let expr = Expr::FString {
        segments: vec![
            FStringSegment::Text("hello ".into()),
            FStringSegment::Interpolation {
                expr: Box::new(Expr::Var("name".into(), Span::dummy())),
                format_spec: None,
            },
        ],
        span: Span::dummy(),
    };
    if let Expr::FString { segments, .. } = &expr {
        assert_eq!(segments.len(), 2);
    } else {
        panic!("Expected Expr::FString");
    }
}

#[test]
fn test_expr_error() {
    let expr = Expr::Error(Span::dummy());
    assert!(matches!(expr, Expr::Error(_)));
}

// ============================================================================
// StmtKind variants
// ============================================================================

#[test]
fn test_stmtkind_var() {
    let stmt = Stmt {
        kind: StmtKind::Var {
            name: "x".into(),
            name_span: Span::dummy(),
            type_annotation: Some(Type::Name {
                name: "Int".into(),
                span: Span::dummy(),
            }),
            initializer: Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
            is_mut: false,
        },
        span: Span::dummy(),
    };
    if let StmtKind::Var { name, is_mut, .. } = &stmt.kind {
        assert_eq!(name, "x");
        assert!(!is_mut);
    } else {
        panic!("Expected StmtKind::Var");
    }
}

#[test]
fn test_stmtkind_binding() {
    let stmt = Stmt {
        kind: StmtKind::Binding {
            name: "add".into(),
            type_name: None,
            method_type: None,
            generic_params: vec![],
            type_annotation: None,
            eval: None,
            params: vec![],
            body: (vec![], None),
            is_pub: false,
        },
        span: Span::dummy(),
    };
    if let StmtKind::Binding { name, .. } = &stmt.kind {
        assert_eq!(name, "add");
    } else {
        panic!("Expected StmtKind::Binding");
    }
}

#[test]
fn test_stmtkind_use() {
    let stmt = Stmt {
        kind: StmtKind::Use {
            path: "std.io".into(),
            path_span: Span::dummy(),
            path_parts: vec![
                SpannedIdent {
                    name: "std".into(),
                    span: Span::dummy(),
                },
                SpannedIdent {
                    name: "io".into(),
                    span: Span::dummy(),
                },
            ],
            items: Some(vec!["println".into()]),
            alias: None,
        },
        span: Span::dummy(),
    };
    if let StmtKind::Use { path, .. } = &stmt.kind {
        assert_eq!(path, "std.io");
    } else {
        panic!("Expected StmtKind::Use");
    }
}

// ============================================================================
// Type variants
// ============================================================================

#[test]
fn test_type_name() {
    let t = Type::Name {
        name: "Int".into(),
        span: Span::dummy(),
    };
    if let Type::Name { name, .. } = &t {
        assert_eq!(name, "Int");
    } else {
        panic!("Expected Type::Name");
    }
}

#[test]
fn test_type_primitives() {
    assert!(matches!(Type::Int(64), Type::Int(64)));
    assert!(matches!(Type::Float(64), Type::Float(64)));
    assert!(matches!(Type::Char, Type::Char));
    assert!(matches!(Type::String, Type::String));
    assert!(matches!(Type::Bool, Type::Bool));
    assert!(matches!(Type::Void, Type::Void));
}

#[test]
fn test_type_fn() {
    let t = Type::Fn {
        params: vec![Type::Name {
            name: "Int".into(),
            span: Span::dummy(),
        }],
        return_type: Box::new(Type::Name {
            name: "String".into(),
            span: Span::dummy(),
        }),
    };
    if let Type::Fn { params, .. } = &t {
        assert_eq!(params.len(), 1);
    } else {
        panic!("Expected Type::Fn");
    }
}

#[test]
fn test_type_tuple() {
    let t = Type::Tuple(vec![Type::Int(64), Type::Bool]);
    if let Type::Tuple(ts) = &t {
        assert_eq!(ts.len(), 2);
    } else {
        panic!("Expected Type::Tuple");
    }
}

#[test]
fn test_type_meta_type() {
    let t = Type::MetaType {
        name_span: Span::dummy(),
        args: vec![],
    };
    assert!(is_meta_type(&t));
    assert!(type_annotation_returns_meta_type(&t));
}

#[test]
fn test_type_generic() {
    let t = Type::Generic {
        name: "List".into(),
        name_span: Span::dummy(),
        args: vec![Type::Name {
            name: "Int".into(),
            span: Span::dummy(),
        }],
    };
    if let Type::Generic { name, args, .. } = &t {
        assert_eq!(name, "List");
        assert_eq!(args.len(), 1);
    } else {
        panic!("Expected Type::Generic");
    }
}

#[test]
fn test_type_variant() {
    let t = Type::Variant(vec![
        VariantDef {
            name: "red".into(),
            name_span: Span::dummy(),
            params: vec![],
            span: Span::dummy(),
        },
        VariantDef {
            name: "green".into(),
            name_span: Span::dummy(),
            params: vec![],
            span: Span::dummy(),
        },
    ]);
    if let Type::Variant(variants) = &t {
        assert_eq!(variants.len(), 2);
    } else {
        panic!("Expected Type::Variant");
    }
}

#[test]
fn test_type_meta_type_nested() {
    let inner = Type::MetaType {
        name_span: Span::dummy(),
        args: vec![],
    };
    let outer = Type::MetaType {
        name_span: Span::dummy(),
        args: vec![Type::MetaType {
            name_span: Span::dummy(),
            args: vec![inner],
        }],
    };
    assert!(is_meta_type(&outer));
    // Verify nested MetaType has args
    if let Type::MetaType { args, .. } = &outer {
        assert_eq!(args.len(), 1);
    }
}

// ============================================================================
// Pattern variants
// ============================================================================

#[test]
fn test_pattern_wildcard() {
    assert!(matches!(Pattern::Wildcard, Pattern::Wildcard));
}

#[test]
fn test_pattern_literal() {
    let pat = Pattern::Literal(Literal::Int(42));
    assert!(matches!(pat, Pattern::Literal(_)));
}

#[test]
fn test_pattern_identifier() {
    let pat = Pattern::Identifier("x".into());
    if let Pattern::Identifier(name) = &pat {
        assert_eq!(name, "x");
    }
}

#[test]
fn test_pattern_guard() {
    let pat = Pattern::Guard {
        pattern: Box::new(Pattern::Wildcard),
        condition: Expr::Lit(Literal::Bool(true), Span::dummy()),
    };
    assert!(matches!(pat, Pattern::Guard { .. }));
}

// ============================================================================
// Operator exhaustiveness
// ============================================================================

#[test]
fn test_binop_all_variants() {
    // Verify all BinOp variants exist and are constructable
    let ops = vec![
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Mod,
        BinOp::Eq,
        BinOp::Neq,
        BinOp::Lt,
        BinOp::Le,
        BinOp::Gt,
        BinOp::Ge,
        BinOp::And,
        BinOp::Or,
        BinOp::Range,
        BinOp::Assign,
    ];
    assert_eq!(ops.len(), 15);
}

#[test]
fn test_unop_all_variants() {
    let ops = vec![UnOp::Neg, UnOp::Pos, UnOp::Not, UnOp::Deref];
    assert_eq!(ops.len(), 4);
}

// ============================================================================
// classify_binding_semantic_kind
// ============================================================================

#[test]
fn test_classify_method() {
    let kind = classify_binding_semantic_kind(Some(&"Point".to_string()), None, &[], &[], None);
    assert_eq!(kind, BindingSemanticKind::Method);
}

#[test]
fn test_classify_type_constructor() {
    let kind = classify_binding_semantic_kind(
        None,
        Some(&Type::MetaType {
            name_span: Span::dummy(),
            args: vec![],
        }),
        &[],
        &[],
        None,
    );
    assert_eq!(kind, BindingSemanticKind::TypeConstructor);
}

#[test]
fn test_classify_function() {
    let kind = classify_binding_semantic_kind(
        None,
        None,
        &[Param {
            name: "x".into(),
            ty: None,
            is_mut: false,
            span: Span::dummy(),
        }],
        &[],
        None,
    );
    assert_eq!(kind, BindingSemanticKind::Function);
}

// ============================================================================
// StructField
// ============================================================================

#[test]
fn test_struct_field_new() {
    let field = StructField::new(
        "x".into(),
        false,
        Type::Name {
            name: "Int".into(),
            span: Span::dummy(),
        },
    );
    assert_eq!(field.name, "x");
    assert!(!field.is_mut);
    assert!(field.default.is_none());
}

#[test]
fn test_struct_field_with_default() {
    let field = StructField::with_default(
        "x".into(),
        false,
        Type::Float(64),
        Expr::Lit(Literal::Float(0.0), Span::dummy()),
    );
    assert_eq!(field.name, "x");
    assert!(field.default.is_some());
}

// ============================================================================
// GenericParam
// ============================================================================

#[test]
fn test_generic_param_type_kind() {
    let gp = GenericParam {
        name: "T".into(),
        kind: GenericParamKind::Type,
        constraints: vec![],
    };
    assert_eq!(gp.name, "T");
    assert!(matches!(gp.kind, GenericParamKind::Type));
}

#[test]
fn test_generic_param_const_kind() {
    let gp = GenericParam {
        name: "N".into(),
        kind: GenericParamKind::Const {
            const_type: Box::new(Type::Int(64)),
        },
        constraints: vec![],
    };
    assert!(matches!(gp.kind, GenericParamKind::Const { .. }));
}

#[test]
fn test_generic_param_platform_kind() {
    let gp = GenericParam {
        name: "P".into(),
        kind: GenericParamKind::Platform,
        constraints: vec![],
    };
    assert!(matches!(gp.kind, GenericParamKind::Platform));
}

// ============================================================================
// FStringSegment
// ============================================================================

#[test]
fn test_fstring_segment_text() {
    let seg = FStringSegment::Text("hello".into());
    if let FStringSegment::Text(s) = &seg {
        assert_eq!(s, "hello");
    }
}

#[test]
fn test_fstring_segment_interpolation() {
    let seg = FStringSegment::Interpolation {
        expr: Box::new(Expr::Var("x".into(), Span::dummy())),
        format_spec: Some(".2f".into()),
    };
    if let FStringSegment::Interpolation { format_spec, .. } = &seg {
        assert_eq!(format_spec, &Some(".2f".into()));
    }
}

// ============================================================================
// EvalMode
// ============================================================================

#[test]
fn test_eval_mode_all() {
    assert!(matches!(EvalMode::Block, EvalMode::Block));
    assert!(matches!(EvalMode::Auto, EvalMode::Auto));
    assert!(matches!(EvalMode::Eager, EvalMode::Eager));
}

// ============================================================================
// BindingKind
// ============================================================================

#[test]
fn test_binding_kind_external() {
    let bk = BindingKind::External {
        function: "func".into(),
        positions: vec![0, 1],
    };
    if let BindingKind::External { positions, .. } = &bk {
        assert_eq!(positions.len(), 2);
    }
}

#[test]
fn test_binding_kind_anonymous() {
    let bk = BindingKind::Anonymous {
        params: vec![],
        return_type: Box::new(Type::Void),
        positions: vec![0],
        body: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
    };
    assert!(matches!(bk, BindingKind::Anonymous { .. }));
}

#[test]
fn test_binding_kind_default_external() {
    let bk = BindingKind::DefaultExternal {
        function: "func".into(),
    };
    assert!(matches!(bk, BindingKind::DefaultExternal { .. }));
}

// ============================================================================
// TypeBodyBinding
// ============================================================================

#[test]
fn test_type_body_binding() {
    let tbb = TypeBodyBinding {
        name: "distance".into(),
        kind: BindingKind::DefaultExternal {
            function: "dist".into(),
        },
    };
    assert_eq!(tbb.name, "distance");
}

// ============================================================================
// VariantDef
// ============================================================================

#[test]
fn test_variant_def_no_params() {
    let vd = VariantDef {
        name: "red".into(),
        name_span: Span::dummy(),
        params: vec![],
        span: Span::dummy(),
    };
    assert_eq!(vd.name, "red");
    assert!(vd.params.is_empty());
}

#[test]
fn test_variant_def_with_params() {
    let vd = VariantDef {
        name: "ok".into(),
        name_span: Span::dummy(),
        params: vec![(Some("value".into()), Type::Int(64))],
        span: Span::dummy(),
    };
    assert_eq!(vd.params.len(), 1);
}

// ============================================================================
// Helper function tests
// ============================================================================

#[test]
fn test_is_meta_type_false() {
    assert!(!is_meta_type(&Type::Name {
        name: "Int".into(),
        span: Span::dummy()
    }));
}

#[test]
fn test_type_annotation_returns_meta_type_fn() {
    let t = Type::Fn {
        params: vec![],
        return_type: Box::new(Type::MetaType {
            name_span: Span::dummy(),
            args: vec![],
        }),
    };
    assert!(type_annotation_returns_meta_type(&t));
}

#[test]
fn test_type_annotation_returns_meta_type_false() {
    let t = Type::Fn {
        params: vec![],
        return_type: Box::new(Type::Int(64)),
    };
    assert!(!type_annotation_returns_meta_type(&t));
}

// ============================================================================
// Module default
// ============================================================================

#[test]
fn test_module_default() {
    let m = ast::Module::default();
    assert!(m.items.is_empty());
}

// ============================================================================
// Additional expression types
// ============================================================================

#[test]
fn test_expr_while() {
    let expr = Expr::While {
        condition: Box::new(Expr::Lit(Literal::Bool(true), Span::dummy())),
        body: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::dummy(),
        }),
        label: None,
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::While { .. }));
}

#[test]
fn test_expr_for() {
    let expr = Expr::For {
        var: "i".into(),
        var_mut: false,
        iterable: Box::new(Expr::Var("items".into(), Span::dummy())),
        body: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::dummy(),
        }),
        label: None,
        span: Span::dummy(),
    };
    if let Expr::For { var, .. } = &expr {
        assert_eq!(var, "i");
    }
}

#[test]
fn test_expr_listcomp() {
    let expr = Expr::ListComp {
        element: Box::new(Expr::BinOp {
            op: BinOp::Mul,
            left: Box::new(Expr::Var("x".into(), Span::dummy())),
            right: Box::new(Expr::Var("x".into(), Span::dummy())),
            span: Span::dummy(),
        }),
        var: "x".into(),
        iterable: Box::new(Expr::Var("items".into(), Span::dummy())),
        condition: Some(Box::new(Expr::BinOp {
            op: BinOp::Gt,
            left: Box::new(Expr::Var("x".into(), Span::dummy())),
            right: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
            span: Span::dummy(),
        })),
        span: Span::dummy(),
    };
    if let Expr::ListComp { var, condition, .. } = &expr {
        assert_eq!(var, "x");
        assert!(condition.is_some());
    }
}

#[test]
fn test_expr_dict() {
    let expr = Expr::Dict(
        vec![(
            Expr::Lit(Literal::String("key".into()), Span::dummy()),
            Expr::Lit(Literal::Int(42), Span::dummy()),
        )],
        Span::dummy(),
    );
    if let Expr::Dict(pairs, _) = &expr {
        assert_eq!(pairs.len(), 1);
    }
}

#[test]
fn test_expr_cast() {
    let expr = Expr::Cast {
        expr: Box::new(Expr::Lit(Literal::Int(42), Span::dummy())),
        target_type: Type::Name {
            name: "Float".into(),
            span: Span::dummy(),
        },
        span: Span::dummy(),
    };
    assert!(matches!(expr, Expr::Cast { .. }));
}

#[test]
fn test_expr_fndef() {
    let expr = Expr::FnDef {
        name: "add".into(),
        params: vec![],
        return_type: Some(Type::Int(64)),
        body: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::dummy(),
        }),
        is_async: false,
        span: Span::dummy(),
    };
    if let Expr::FnDef { name, .. } = &expr {
        assert_eq!(name, "add");
    }
}

#[test]
fn test_expr_eval_modes() {
    let body = Box::new(Block {
        stmts: vec![],
        expr: None,
        span: Span::dummy(),
    });
    for mode in [EvalMode::Block, EvalMode::Auto, EvalMode::Eager] {
        let expr = Expr::Eval {
            mode,
            body: body.clone(),
            span: Span::dummy(),
        };
        assert!(matches!(expr, Expr::Eval { .. }));
    }
}
