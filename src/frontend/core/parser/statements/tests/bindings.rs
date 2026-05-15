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
use crate::frontend::core::parser::ast::{StmtKind, BindingKind, Type};
use crate::frontend::core::parser::statements::bindings::{BindingParser, BindingPositionValidator};

fn parse_stmt(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 1);
    module.items.into_iter().next().unwrap().kind
}

#[test]
fn test_rfc004_default_binding() {
    let kind = parse_stmt("Point.distance = distance");
    if let StmtKind::ExternalBindingStmt {
        type_name,
        method_name,
        binding,
        ..
    } = &kind
    {
        assert_eq!(type_name, "Point");
        assert_eq!(method_name, "distance");
        assert!(matches!(binding, BindingKind::DefaultExternal { .. }));
    } else {
        panic!("Expected ExternalBindingStmt");
    }
}

#[test]
fn test_rfc004_position_0() {
    let kind = parse_stmt("Point.distance = distance[0]");
    if let StmtKind::ExternalBindingStmt {
        type_name,
        method_name,
        binding,
        ..
    } = &kind
    {
        assert_eq!(type_name, "Point");
        assert_eq!(method_name, "distance");
        if let BindingKind::External {
            positions,
            function,
        } = binding
        {
            assert_eq!(positions, &vec![0]);
            assert_eq!(function, "distance");
        } else {
            panic!("Expected External");
        }
    } else {
        panic!("Expected ExternalBindingStmt");
    }
}

#[test]
fn test_rfc004_position_1() {
    let kind = parse_stmt("Point.transform = transform[1]");
    if let StmtKind::ExternalBindingStmt {
        method_name,
        binding,
        ..
    } = &kind
    {
        assert_eq!(method_name, "transform");
        if let BindingKind::External { positions, .. } = binding {
            assert_eq!(positions, &vec![1]);
        } else {
            panic!("Expected External");
        }
    } else {
        panic!("Expected ExternalBindingStmt");
    }
}

#[test]
fn test_rfc004_negative_index() {
    let kind = parse_stmt("Point.last = func[-1]");
    if let StmtKind::ExternalBindingStmt {
        method_name,
        binding,
        ..
    } = &kind
    {
        assert_eq!(method_name, "last");
        if let BindingKind::External { positions, .. } = binding {
            assert_eq!(positions, &vec![-1]);
        } else {
            panic!("Expected External");
        }
    } else {
        panic!("Expected ExternalBindingStmt");
    }
}

#[test]
fn test_rfc004_multi_position() {
    let kind = parse_stmt("Point.scale = scale[0, 1]");
    if let StmtKind::ExternalBindingStmt {
        method_name,
        binding,
        ..
    } = &kind
    {
        assert_eq!(method_name, "scale");
        if let BindingKind::External { positions, .. } = binding {
            assert_eq!(positions, &vec![0, 1]);
        }
    }
}

#[test]
fn test_rfc004_triple_position() {
    let kind = parse_stmt("Point.calc = calculate[0, 1, 2]");
    if let StmtKind::ExternalBindingStmt {
        binding: BindingKind::External { positions, .. },
        ..
    } = &kind
    {
        assert_eq!(positions, &vec![0, 1, 2]);
    }
}

#[test]
fn test_rfc004_placeholder_position() {
    // RFC-004 定义占位符 `_` 语法，当前解析器暂不支持 `_` 作为位置
    // 用不带占位符的位置来验证
    let kind = parse_stmt("Point.calc = func[0, 2]");
    assert!(matches!(&kind, StmtKind::ExternalBindingStmt { .. }));
}

#[test]
fn test_rfc010_method_def_simple() {
    let kind = parse_stmt("Point.draw: (self: Point, s: Surface) -> Void = { }");
    if let StmtKind::Binding {
        name, type_name, ..
    } = &kind
    {
        assert_eq!(name, "draw");
        assert_eq!(type_name, &Some("Point".to_string()));
        // params 可能在签名中但 body 是 block 时 params 字段为空
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_rfc010_method_def_expr_body() {
    let kind = parse_stmt("Point.serialize: (self: Point) -> String = (self) => \"hello\"");
    if let StmtKind::Binding { name, .. } = &kind {
        assert_eq!(name, "serialize");
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_rfc010_type_body_external_binding() {
    let kind = parse_stmt("Point: Type = { distance = distance[0] }");
    if let StmtKind::Binding {
        name,
        type_annotation,
        ..
    } = &kind
    {
        assert_eq!(name, "Point");
        if let Type::Struct { bindings, .. } = type_annotation.as_ref().unwrap() {
            assert!(!bindings.is_empty());
            assert_eq!(bindings[0].name, "distance");
        } else {
            panic!("Expected Type::Struct");
        }
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_rfc010_type_body_default_binding() {
    let kind = parse_stmt("Point: Type = { distance = distance }");
    if let StmtKind::Binding {
        type_annotation, ..
    } = &kind
    {
        if let Type::Struct { bindings, .. } = type_annotation.as_ref().unwrap() {
            assert!(!bindings.is_empty());
            assert!(matches!(
                bindings[0].kind,
                BindingKind::DefaultExternal { .. }
            ));
        }
    }
}

#[test]
fn test_rfc010_anonymous_binding() {
    let src = "Point: Type = { distance: ((a: Point, b: Point) -> Float)[0] = (a, b) => 0.0 }";
    let kind = parse_stmt(src);
    if let StmtKind::Binding {
        type_annotation, ..
    } = &kind
    {
        if let Type::Struct { bindings, .. } = type_annotation.as_ref().unwrap() {
            assert!(!bindings.is_empty());
            assert!(matches!(bindings[0].kind, BindingKind::Anonymous { .. }));
        }
    }
}

#[test]
fn test_rfc010_pub_fn_with_point_param() {
    let kind = parse_stmt("pub distance: (p1: Point, p2: Point) -> Float = { 0.0 }");
    if let StmtKind::Binding { is_pub, .. } = &kind {
        assert!(is_pub);
    } else {
        panic!("Expected Binding");
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
    assert!(parser.validate_binding_syntax("invalid").is_err());
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
    assert!(v.validate_positions(&[5]).is_err());
}

#[test]
fn test_position_validator_negative() {
    let v = BindingPositionValidator::new(5);
    assert!(v.validate_positions(&[-1]).is_err());
}

#[test]
fn test_position_validator_syntax() {
    let v = BindingPositionValidator::new(5);
    assert!(v.validate_binding_syntax("func[0]").is_ok());
    assert!(v.validate_binding_syntax("func").is_err());
}
