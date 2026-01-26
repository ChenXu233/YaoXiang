//! Iterator标准trait完整示例（RFC-011 Phase 3）
//!
//! 这个示例展示了如何使用GAT系统实现Iterator trait

use crate::frontend::parser::ast;
use crate::frontend::typecheck::gat::{GATEnvironment, GenericAssocType};
use crate::frontend::typecheck::traits::{TraitDef, TraitMethod, TraitEnvironment};
use crate::util::span::Span;

/// 创建Iterator trait的示例
pub fn create_iterator_trait() -> TraitDef {
    let span = Span::dummy();

    // 创建Iterator trait
    TraitDef {
        name: "Iterator".to_string(),
        generic_params: vec!["T".to_string()], // Iterator的类型参数
        methods: vec![
            TraitMethod {
                name: "next".to_string(),
                params: vec![("self".to_string(), ast::Type::Name("Self".to_string()))],
                return_type: Box::new(ast::Type::Generic {
                    name: "Option".to_string(),
                    args: vec![ast::Type::Generic {
                        name: "Self".to_string(),
                        args: vec![], // Item关联类型
                    }],
                }),
                span,
            },
            TraitMethod {
                name: "has_next".to_string(),
                params: vec![("self".to_string(), ast::Type::Name("Self".to_string()))],
                return_type: Box::new(ast::Type::Bool),
                span,
            },
        ],
        super_traits: vec![],
        assoc_types: vec![
            GenericAssocType {
                name: "Item".to_string(),
                host_params: vec!["T".to_string()], // Item的类型参数来自Iterator
                assoc_params: vec![],
                bounds: vec![],
                default_ty: Some(Box::new(ast::Type::Generic {
                    name: "Self".to_string(),
                    args: vec![],
                })),
                span,
            },
        ],
        span,
    }
}

/// 创建Iterable trait的示例
pub fn create_iterable_trait() -> TraitDef {
    let span = Span::dummy();

    // 创建Iterable trait
    TraitDef {
        name: "Iterable".to_string(),
        generic_params: vec!["T".to_string()],
        methods: vec![
            TraitMethod {
                name: "iter".to_string(),
                params: vec![("self".to_string(), ast::Type::Name("Self".to_string()))],
                return_type: Box::new(ast::Type::Generic {
                    name: "Iterator".to_string(),
                    args: vec![ast::Type::Generic {
                        name: "Self".to_string(),
                        args: vec![], // Item关联类型
                    }],
                }),
                span,
            },
        ],
        super_traits: vec![],
        assoc_types: vec![
            GenericAssocType {
                name: "Item".to_string(),
                host_params: vec!["T".to_string()],
                assoc_params: vec![],
                bounds: vec![],
                default_ty: Some(Box::new(ast::Type::Generic {
                    name: "Self".to_string(),
                    args: vec![],
                })),
                span,
            },
        ],
        span,
    }
}

/// 示例：Vec实现Iterable
pub fn vec_iterable_example() -> ast::Stmt {
    let span = Span::dummy();

    // Vec<T> 实现 Iterable<T>
    ast::Stmt::Impl {
        trait_ref: Some(ast::Type::Generic {
            name: "Iterable".to_string(),
            args: vec![ast::Type::Generic {
                name: "T".to_string(),
                args: vec![],
            }],
        }),
        for_type: ast::Type::Generic {
            name: "Vec".to_string(),
            args: vec![ast::Type::Generic {
                name: "T".to_string(),
                args: vec![],
            }],
        },
        methods: vec![
            // 实现 iter 方法
            ast::Stmt::Fn {
                name: "iter".to_string(),
                params: vec![ast::Param {
                    name: "self".to_string(),
                    ty: Some(ast::Type::Name("Self".to_string())),
                    span,
                }],
                return_type: Some(ast::Type::Generic {
                    name: "Iterator".to_string(),
                    args: vec![ast::Type::Generic {
                        name: "Self".to_string(),
                        args: vec![],
                    }],
                }),
                body: ast::Expr::Block(Box::new(ast::Block {
                    stmts: vec![],
                    expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Unit))),
                    span,
                })),
                span,
            },
        ],
        span,
    }
}

/// 示例：使用GAT的泛型函数
pub fn collect_function_example() -> ast::Stmt {
    let span = Span::dummy();

    // collect: [T, I: Iterable[T]](iter: I) -> Vec[T]
    ast::Stmt::Fn {
        name: "collect".to_string(),
        params: vec![ast::Param {
            name: "iter".to_string(),
            ty: Some(ast::Type::Generic {
                name: "I".to_string(),
                args: vec![],
            }),
            span,
        }],
        return_type: Some(ast::Type::Generic {
            name: "Vec".to_string(),
            args: vec![ast::Type::Generic {
                name: "T".to_string(),
                args: vec![],
            }],
        }),
        body: ast::Expr::Block(Box::new(ast::Block {
            stmts: vec![],
            expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Unit))),
            span,
        })),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator_trait_creation() {
        let iterator_trait = create_iterator_trait();
        assert_eq!(iterator_trait.name, "Iterator");
        assert_eq!(iterator_trait.generic_params.len(), 1);
        assert_eq!(iterator_trait.assoc_types.len(), 1);
        assert_eq!(iterator_trait.assoc_types[0].name, "Item");
    }

    #[test]
    fn test_iterable_trait_creation() {
        let iterable_trait = create_iterable_trait();
        assert_eq!(iterable_trait.name, "Iterable");
        assert_eq!(iterable_trait.generic_params.len(), 1);
        assert_eq!(iterable_trait.assoc_types.len(), 1);
    }

    #[test]
    fn test_gat_assoc_type_structure() {
        let iterator_trait = create_iterator_trait();
        let item_assoc = &iterator_trait.assoc_types[0];

        assert_eq!(item_assoc.name, "Item");
        assert_eq!(item_assoc.host_params, vec!["T"]);
        assert!(item_assoc.default_ty.is_some());
    }

    #[test]
    fn test_vec_iterable_example() {
        let impl_stmt = vec_iterable_example();

        // 检查impl结构
        if let ast::Stmt::Impl {
            trait_ref: Some(ref tr),
            for_type: ref ft,
            ..
        } = impl_stmt
        {
            assert!(matches!(tr, ast::Type::Generic { .. }));
            assert!(matches!(ft, ast::Type::Generic { .. }));
        } else {
            panic!("Expected impl statement");
        }
    }

    #[test]
    fn test_collect_function_example() {
        let fn_stmt = collect_function_example();

        // 检查函数结构
        if let ast::Stmt::Fn {
            name,
            params,
            return_type: Some(ref rt),
            ..
        } = fn_stmt
        {
            assert_eq!(name, "collect");
            assert_eq!(params.len(), 1);
            assert!(matches!(rt, ast::Type::Generic { .. }));
        } else {
            panic!("Expected fn statement");
        }
    }

    #[test]
    fn test_iterator_next_method() {
        let iterator_trait = create_iterator_trait();
        let next_method = &iterator_trait.methods[0];

        assert_eq!(next_method.name, "next");
        assert_eq!(next_method.params.len(), 1);
    }

    #[test]
    fn test_iterator_has_next_method() {
        let iterator_trait = create_iterator_trait();
        let has_next_method = &iterator_trait.methods[1];

        assert_eq!(has_next_method.name, "has_next");
        assert_eq!(has_next_method.params.len(), 1);
        assert!(matches!(has_next_method.return_type.as_ref(), ast::Type::Bool));
    }

    #[test]
    fn test_iterable_iter_method() {
        let iterable_trait = create_iterable_trait();
        let iter_method = &iterable_trait.methods[0];

        assert_eq!(iter_method.name, "iter");
        assert_eq!(iter_method.params.len(), 1);

        // 检查返回类型是 Iterator<Self>
        if let ast::Type::Generic { name, args } = iter_method.return_type.as_ref() {
            assert_eq!(name, "Iterator");
            assert_eq!(args.len(), 1);
        } else {
            panic!("Expected Iterator generic type");
        }
    }

    #[test]
    fn test_associated_type_in_function_type() {
        let iterator_trait = create_iterator_trait();
        let next_method = &iterator_trait.methods[0];

        // next 方法的返回类型应该是 Option<Item>
        if let ast::Type::Generic { name, args } = next_method.return_type.as_ref() {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);

            // Item 应该引用 Iterator 的关联类型
            if let ast::Type::Generic { name: item_name, .. } = &args[0] {
                assert_eq!(item_name, "Self");
            }
        }
    }
}
