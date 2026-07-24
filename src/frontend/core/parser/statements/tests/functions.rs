//! Function definition parsing tests — based on spec §6.1–§6.4

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::{Expr, StmtKind};

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
fn test_fn_typed_params() {
    let kind = parse_fn("add: (a: Int, b: Int) -> Int = (a, b) => a + b");
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
    assert!(params[0].ty.is_some(), "参数应有类型标注");
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
