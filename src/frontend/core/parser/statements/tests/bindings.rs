//! 方法绑定测试 — 基于 RFC-004, RFC-010, 语言规范 §6.6-§6.8
//!
//! RFC-004 定义:
//!   Type.method = func          默认绑定（自动检测位置）
//!   Type.method = func[N]       单位置绑定
//!   Type.method = func[-1]      负数索引（最后一个参数）
//!   Type.method = func[0, 1]    多位置联合绑定
//!   Type.method = func[0, _, 2] 占位符绑定
//!
//! RFC-010 定义:
//!   Type.method: (params) -> Ret = { body }  方法定义
//!   { method = func[N] }                    类型体内绑定
//!   { method: (FnType)[N] = (params) => body }  匿名函数绑定

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::ast::{StmtKind, BindingKind, Type, TypeBodyBinding, TypeBodyItem};
use crate::frontend::core::parser::statements::bindings::{BindingParser, BindingPositionValidator};

fn parse_stmt(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors, "解析不应有错误");
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
}

#[test]
fn test_rfc004_triple_position() {
    let kind = parse_stmt("Point.calc = calculate[0, 1, 2]");
    assert!(matches!(&kind, StmtKind::Assign { .. }));
}

#[test]
fn test_rfc010_type_body_default_binding() {
    let kind = parse_stmt("Point: Type = { distance = distance }");
    if let StmtKind::TypeDefinition {
        definition: Type::Struct { body },
        ..
    } = &kind
    {
        let bindings: Vec<&TypeBodyBinding> = body
            .iter()
            .filter_map(|it| {
                if let TypeBodyItem::Binding(b) = it {
                    Some(b)
                } else {
                    None
                }
            })
            .collect();
        assert!(!bindings.is_empty(), "应有 bindings");
        assert!(
            matches!(bindings[0].kind, BindingKind::DefaultExternal { .. }),
            "binding kind 应为 DefaultExternal"
        );
    }
}

#[test]
fn test_rfc010_anonymous_binding() {
    let src = "Point: Type = { distance: ((a: Point, b: Point) -> Float)[0] = (a, b) => 0.0 }";
    let kind = parse_stmt(src);
    if let StmtKind::TypeDefinition {
        definition: Type::Struct { body },
        ..
    } = &kind
    {
        let bindings: Vec<&TypeBodyBinding> = body
            .iter()
            .filter_map(|it| {
                if let TypeBodyItem::Binding(b) = it {
                    Some(b)
                } else {
                    None
                }
            })
            .collect();
        assert!(!bindings.is_empty(), "应有 bindings");
        assert!(
            matches!(bindings[0].kind, BindingKind::Anonymous { .. }),
            "binding kind 应为 Anonymous"
        );
    }
}

#[test]
fn test_binding_parser_validate_ok() {
    let parser = BindingParser::new();
    assert!(parser
        .validate_binding_syntax("Point.method = value")
        .is_ok());
}

#[test]
fn test_binding_parser_validate_missing_eq() {
    let parser = BindingParser::new();
    assert!(
        parser.validate_binding_syntax("invalid").is_err(),
        "无效绑定语法应被拒绝"
    );
}

#[test]
fn test_binding_parser_parse_tokens() {
    let tokens = tokenize("Point.method = 42").unwrap();
    let parser = BindingParser::new();
    assert!(parser.parse_binding(&tokens, 0).is_ok());
}

#[test]
fn test_position_validator_ok() {
    let v = BindingPositionValidator::new(5);
    assert!(v.validate_positions(&[0, 1, 2, 3, 4]).is_ok());
}

#[test]
fn test_position_validator_exceeds() {
    let v = BindingPositionValidator::new(3);
    assert!(
        v.validate_positions(&[5]).is_err(),
        "超出范围的位置应被拒绝"
    );
}

#[test]
fn test_position_validator_syntax() {
    let v = BindingPositionValidator::new(5);
    assert!(
        v.validate_binding_syntax("func[0]").is_ok(),
        "func[0] 是合法绑定语法"
    );
    assert!(
        v.validate_binding_syntax("func").is_err(),
        "func 缺少位置索引应被拒绝"
    );
}
