//! Function definition parsing tests — based on spec §6.1–§6.4

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::ast::{Expr, StmtKind};

fn parse_fn(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors, "解析不应有错误");
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
}

#[test]
fn test_fn_no_params_block_body() {
    let kind = parse_fn("f = () => { return 1 }");
    if let StmtKind::Assign { target, value, .. } = &kind {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        let (params, _body) = if let Some(v) = value {
            if let Expr::Lambda { params, body, .. } = v.as_ref() {
                (params.clone(), body.stmts.clone())
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };
        assert_eq!(name, "f");
        assert!(params.is_empty(), "无参数函数 params 应为空");
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_fn_one_param_expr_body() {
    let kind = parse_fn("inc = (x) => x + 1");
    if let StmtKind::Assign { target, value, .. } = &kind {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        let (params, _body) = if let Some(v) = value {
            if let Expr::Lambda { params, body, .. } = v.as_ref() {
                (params.clone(), body.stmts.clone())
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };
        assert_eq!(name, "inc");
        assert_eq!(params.len(), 1);
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_fn_multi_param() {
    let kind = parse_fn("add = (a, b) => a + b");
    let StmtKind::Assign { value, .. } = &kind else {
        panic!("Expected Assign");
    };
    let params = if let Some(v) = value {
        if let Expr::Lambda { params, .. } = v.as_ref() {
            params.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].name, "a");
    assert_eq!(params[1].name, "b");
}

#[test]
fn test_fn_signature_params_carry_types() {
    // 覆盖: WBS 6.8.3 — 类型标注停在**声明**里；实现头只保留作者写的东西
    let kind = parse_fn("add: (a: Int, b: Int) -> Int = (a, b) => a + b");
    let StmtKind::Assign {
        value,
        type_annotation,
        ..
    } = &kind
    else {
        panic!("Expected Assign");
    };
    // Act：声明侧（Type::Fn 链）与实现头（lambda params）分别取值
    let signature = type_annotation
        .as_ref()
        .map(crate::ast::Type::fn_params_flat)
        .unwrap_or_default();
    let params = if let Some(v) = value {
        if let Expr::Lambda { params, .. } = v.as_ref() {
            params.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    // Assert：声明的形参带类型；实现头的同名形参不再被补上类型
    assert_eq!(signature.len(), 2, "声明侧应有 2 位形参");
    assert!(
        signature.iter().all(|p| p.ty.is_some()),
        "声明侧形参必须带类型标注，got: {:?}",
        signature.iter().map(|p| &p.ty).collect::<Vec<_>>()
    );
    assert_eq!(params.len(), 2, "实现头应保留作者写的 2 位形参");
    assert!(
        params.iter().all(|p| p.ty.is_none()),
        "实现头不得被 parser 补上声明类型（6.8.3 起由消费点取声明），got: {:?}",
        params.iter().map(|p| &p.ty).collect::<Vec<_>>()
    );
}

#[test]
fn test_fn_mut_param() {
    let kind = parse_fn("inc: (mut x: Int) -> Int = (mut x) => x + 1");
    let StmtKind::Assign { value, .. } = &kind else {
        panic!("Expected Assign");
    };
    let params = if let Some(v) = value {
        if let Expr::Lambda { params, .. } = v.as_ref() {
            params.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    assert!(params[0].is_mut, "mut 参数应被识别");
}

/// 旧函数定义语法的块体形态 `f: () -> Int = { 5 }`：带 Fn 注解的块被收编为
/// `Lambda`（空参数表 + 块体），不构造已删除的 FnDef 变体（RFC-039 6.5.1）。
#[test]
fn test_fn_block_body_with_fn_annotation_parses_to_lambda() {
    // Arrange
    let kind = parse_fn("f: () -> Int = { 5 }");

    // Act
    let StmtKind::Assign { target, value, .. } = &kind else {
        panic!("Expected Assign");
    };

    // Assert
    assert!(
        matches!(target.as_ref(), Expr::Var(name, _) if name == "f"),
        "左值应为变量 f，实际: {target:?}"
    );
    assert!(
        matches!(
            value.as_deref(),
            Some(Expr::Lambda { params, .. }) if params.is_empty()
        ),
        "块体函数定义的 value 应为 Lambda（空参数表），实际: {value:?}"
    );
}
