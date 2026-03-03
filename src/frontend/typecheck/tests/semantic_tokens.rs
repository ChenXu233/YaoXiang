use crate::frontend::core::parser::ast;
use crate::frontend::typecheck::semantic_db::SemanticTokenModifier;
use crate::frontend::typecheck::semantic_db::SemanticTokenType;
use crate::frontend::typecheck::TypeChecker;
use crate::util::span::{Position, Span};

fn dummy_span() -> Span {
    Span::new(Position::dummy(), Position::dummy())
}

#[test]
fn test_var_reassignment_not_marked_declaration() {
    let mut checker = TypeChecker::new("file:///test.yx");

    let stmt1 = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "x".to_string(),
            name_span: dummy_span(),
            type_annotation: None,
            initializer: Some(Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(1),
                dummy_span(),
            ))),
            is_mut: false,
        },
        span: dummy_span(),
    };

    // Same surface syntax as declaration, but semantically it's reassignment.
    let stmt2 = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "x".to_string(),
            name_span: dummy_span(),
            type_annotation: None,
            initializer: Some(Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(2),
                dummy_span(),
            ))),
            is_mut: false,
        },
        span: dummy_span(),
    };

    let module = ast::Module {
        items: vec![stmt1, stmt2],
        span: dummy_span(),
    };

    let result = checker.check_module_collect_all(&module).unwrap();
    let tokens = result.semantic_db.get_tokens("file:///test.yx").unwrap();
    let xs: Vec<_> = tokens.iter().filter(|t| t.name == "x").collect();

    assert_eq!(xs.len(), 2);
    assert!(xs[0]
        .modifiers
        .contains(&SemanticTokenModifier::Declaration));
    assert!(!xs[1]
        .modifiers
        .contains(&SemanticTokenModifier::Declaration));
    assert_eq!(xs[1].modifiers, vec![SemanticTokenModifier::Mutable]);
}

#[test]
fn test_mut_var_declaration_marked_declaration_assignment_not() {
    let mut checker = TypeChecker::new("file:///test.yx");

    let stmt1 = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "y".to_string(),
            name_span: dummy_span(),
            type_annotation: None,
            initializer: Some(Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(1),
                dummy_span(),
            ))),
            is_mut: true,
        },
        span: dummy_span(),
    };

    let stmt2 = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "y".to_string(),
            name_span: dummy_span(),
            type_annotation: None,
            initializer: Some(Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(2),
                dummy_span(),
            ))),
            is_mut: false,
        },
        span: dummy_span(),
    };

    let module = ast::Module {
        items: vec![stmt1, stmt2],
        span: dummy_span(),
    };

    let result = checker.check_module_collect_all(&module).unwrap();
    let tokens = result.semantic_db.get_tokens("file:///test.yx").unwrap();
    let ys: Vec<_> = tokens.iter().filter(|t| t.name == "y").collect();

    assert_eq!(ys.len(), 2);
    assert_eq!(ys[0].modifiers, vec![SemanticTokenModifier::Declaration]);
    assert_eq!(ys[1].modifiers, vec![SemanticTokenModifier::Mutable]);
}

#[test]
fn test_type_annotation_emits_type_token() {
    let mut checker = TypeChecker::new("file:///test.yx");

    let stmt = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "x".to_string(),
            name_span: dummy_span(),
            type_annotation: Some(ast::Type::Name {
                name: "Int".to_string(),
                span: dummy_span(),
            }),
            initializer: None,
            is_mut: false,
        },
        span: dummy_span(),
    };

    let module = ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };

    let result = checker.check_module_collect_all(&module).unwrap();
    let tokens = result.semantic_db.get_tokens("file:///test.yx").unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| t.token_type == SemanticTokenType::Type && t.name == "Int"),
        "expected a Type token for Int"
    );
}

#[test]
fn test_meta_type_annotation_emits_type_token_for_type_keyword() {
    let mut checker = TypeChecker::new("file:///test.yx");

    let stmt = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "T".to_string(),
            name_span: dummy_span(),
            type_annotation: Some(ast::Type::MetaType {
                name_span: dummy_span(),
                args: vec![],
            }),
            initializer: None,
            is_mut: false,
        },
        span: dummy_span(),
    };

    let module = ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };

    let result = checker.check_module_collect_all(&module).unwrap();
    let tokens = result.semantic_db.get_tokens("file:///test.yx").unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| t.token_type == SemanticTokenType::Type && t.name == "Type"),
        "expected a Type token for meta-type keyword Type"
    );
}

#[test]
fn test_use_emits_segmented_namespace_tokens_on_path_parts() {
    let mut checker = TypeChecker::new("file:///test.yx");

    let stmt = ast::Stmt {
        kind: ast::StmtKind::Use {
            path: "std.io".to_string(),
            path_span: dummy_span(),
            path_parts: vec![
                ast::SpannedIdent {
                    name: "std".to_string(),
                    span: dummy_span(),
                },
                ast::SpannedIdent {
                    name: "io".to_string(),
                    span: dummy_span(),
                },
            ],
            items: None,
            alias: None,
        },
        span: dummy_span(),
    };

    let module = ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };

    let result = checker.check_module_collect_all(&module).unwrap();
    let tokens = result.semantic_db.get_tokens("file:///test.yx").unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| t.token_type == SemanticTokenType::Namespace && t.name == "std"),
        "expected a Namespace token for first use path segment std"
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.token_type == SemanticTokenType::Namespace && t.name == "io"),
        "expected a Namespace token for second use path segment io"
    );
}

#[test]
fn test_variant_constructors_emit_enum_member_tokens_and_usages() {
    let mut checker = TypeChecker::new("file:///test.yx");

    let typedef = ast::Stmt {
        kind: ast::StmtKind::TypeDef {
            name: "Result".to_string(),
            name_span: dummy_span(),
            definition: ast::Type::Variant(vec![
                ast::VariantDef {
                    name: "Ok".to_string(),
                    name_span: dummy_span(),
                    params: vec![(
                        None,
                        ast::Type::Name {
                            name: "Int".to_string(),
                            span: dummy_span(),
                        },
                    )],
                    span: dummy_span(),
                },
                ast::VariantDef {
                    name: "Err".to_string(),
                    name_span: dummy_span(),
                    params: vec![(
                        None,
                        ast::Type::Name {
                            name: "String".to_string(),
                            span: dummy_span(),
                        },
                    )],
                    span: dummy_span(),
                },
            ]),
            generic_params: vec![],
        },
        span: dummy_span(),
    };

    let module = ast::Module {
        items: vec![typedef],
        span: dummy_span(),
    };

    let result = checker.check_module_collect_all(&module).unwrap();
    let tokens = result.semantic_db.get_tokens("file:///test.yx").unwrap();

    assert!(
        tokens
            .iter()
            .any(|t| t.token_type == SemanticTokenType::EnumMember && t.name == "Ok"),
        "expected EnumMember token for Ok constructor definition or usage"
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.token_type == SemanticTokenType::EnumMember && t.name == "Err"),
        "expected EnumMember token for Err constructor definition"
    );
}
