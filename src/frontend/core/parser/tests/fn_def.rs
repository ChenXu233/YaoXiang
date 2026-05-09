//! RFC-007 函数定义解析器测试
//!
//! 测试 RFC-007 统一语法：name: (params) -> Return = body
//!
//! 语法规则：
//! - 完整形式：name: (a: Type, b: Type) -> Ret = (a, b) => { return ... }
//! - 省略 Lambda 头：name: (a: Type, b: Type) -> Ret = { return ... }
//! - 直接表达式：name: (a: Type, b: Type) -> Ret = expression
//! - 最简形式：name = (a, b) => { return ... }
//! - 泛型函数：name: [T](x: T) -> T = x
//! - 方法绑定：Type.method: (Type, ...) -> Ret = (params) => { ... }
//!
//! RFC-010 统一语法: name: (params) -> Return = body
//! - 参数名在签名中声明: `(a: Int, b: Int)`
//! - Lambda 头可省略（签名已声明参数）

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast::{StmtKind};
use crate::frontend::core::parser::parse;

#[cfg(test)]
mod fn_def_tests {
    use super::*;

    // ======== RFC-010 函数定义测试 ========

    #[test]
    fn test_parse_fn_def_no_params() {
        // RFC-010: main: () -> Void = {}
        let source = "main: () -> Void = {}";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding { name, params, .. } => {
                assert_eq!(name, "main");
                assert!(params.is_empty());
            }
            _ => panic!("Expected Binding statement, got {:?}", module.items[0].kind),
        }
    }

    #[test]
    fn test_parse_fn_def_with_params() {
        // RFC-010: 参数名在签名中，表达式体
        let source = "add: (a: Int, b: Int) -> Int = a + b";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "a");
                assert_eq!(params[1].name, "b");
            }
            _ => panic!("Expected Binding statement"),
        }
    }

    #[test]
    fn test_parse_fn_def_block_body() {
        // RFC-010: 代码块体
        let source = "add: (a: Int, b: Int) -> Int = { return a + b }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();
        assert_eq!(module.items.len(), 1);
    }

    // ======== RFC-011 泛型函数测试 ========

    #[test]
    fn test_parse_generic_param_with_constraint() {
        // RFC-010: (T: Clone) -> ((x: T) -> T)
        let source = "clone: (T: Clone) -> ((x: T) -> T) = x";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                generic_params,
                ..
            } => {
                assert_eq!(name, "clone");
                assert_eq!(generic_params.len(), 1);
                assert_eq!(generic_params[0].name, "T");
                assert_eq!(generic_params[0].constraints.len(), 1);
                match &generic_params[0].constraints[0] {
                    crate::frontend::core::parser::ast::Type::Name { name, .. } => {
                        assert_eq!(name, "Clone");
                    }
                    _ => panic!("Expected Type::Name for constraint"),
                }
            }
            _ => panic!("Expected Binding statement, got {:?}", module.items[0].kind),
        }
    }

    #[test]
    fn test_parse_multiple_generic_params_with_constraints() {
        // RFC-010: (T: Clone, U: Serializable) -> ((a: T, b: U) -> (T, U))
        let source = "pair: (T: Clone, U: Serializable) -> ((a: T, b: U) -> (T, U)) = (a, b)";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                generic_params,
                ..
            } => {
                assert_eq!(name, "pair");
                assert_eq!(generic_params.len(), 2);
                assert_eq!(generic_params[0].name, "T");
                assert_eq!(generic_params[0].constraints.len(), 1);
                assert_eq!(generic_params[1].name, "U");
                assert_eq!(generic_params[1].constraints.len(), 1);
            }
            _ => panic!("Expected Binding statement"),
        }
    }

    #[test]
    fn test_parse_generic_param_without_constraint() {
        // RFC-010: (T: Type) -> ((value: T) -> T)
        let source = "identity: (T: Type) -> ((value: T) -> T) = value";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                generic_params,
                ..
            } => {
                assert_eq!(name, "identity");
                assert_eq!(generic_params.len(), 1);
                assert_eq!(generic_params[0].name, "T");
                assert!(generic_params[0].constraints.is_empty());
            }
            _ => panic!("Expected Binding statement"),
        }
    }

    // ======== RFC-010 方法绑定测试 ========

    #[test]
    fn test_parse_method_bind_basic() {
        // RFC-010: Point.draw: (self: Point, surface: Surface) -> Void = { ... }
        let source = "Point.draw: (self: Point, surface: Surface) -> Void = { }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                params,
                ..
            } => {
                assert_eq!(name, "draw");
                assert_eq!(*type_name, Some("Point".to_string()));
                // RFC-010: params 从签名解析，method_type 存储完整签名
                if let Some(crate::frontend::core::parser::ast::Type::Fn {
                    params: type_params,
                    ..
                }) = method_type
                {
                    assert_eq!(type_params.len(), 2); // self: Point, surface: Surface
                } else {
                    panic!("Expected Fn type for method_type, got {:?}", method_type);
                }
                // RFC-010: body 是代码块，params 为空
                assert_eq!(params.len(), 0);
            }
            _ => panic!(
                "Expected Binding statement for method, got {:?}",
                module.items[0].kind
            ),
        }
    }

    #[test]
    fn test_parse_method_bind_with_expression_body() {
        // RFC-010: Point.serialize: (self: Point) -> String = "..."
        // 参数从签名解析，method_type 存储签名
        let source = "Point.serialize: (self: Point) -> String = \"Point(${self.x}, ${self.y})\"";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                body: (stmts, expr),
                ..
            } => {
                assert_eq!(name, "serialize");
                assert_eq!(*type_name, Some("Point".to_string()));
                // RFC-010: params 从签名解析，存储在 method_type 中
                // method_type 应该是 Fn 类型
                match method_type {
                    Some(crate::frontend::core::parser::ast::Type::Fn { params, .. }) => {
                        assert_eq!(params.len(), 1); // self: Point
                    }
                    _ => panic!("Expected Fn type for method_type"),
                }
                assert!(stmts.is_empty());
                assert!(expr.is_some());
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    #[test]
    fn test_parse_method_bind_with_block_body() {
        // RFC-010: Point.distance: (self: Point, other: Point) -> Float = { ... }
        let source = r#"
            Point.distance: (self: Point, other: Point) -> Float = {
                dx = self.x - other.x
                dy = self.y - other.y
                return (dx * dx + dy * dy).sqrt()
            }
        "#;
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                body: (stmts, _expr),
                ..
            } => {
                assert_eq!(name, "distance");
                assert_eq!(*type_name, Some("Point".to_string()));
                // RFC-010: method_type 存储完整签名
                if let Some(crate::frontend::core::parser::ast::Type::Fn {
                    params: type_params,
                    ..
                }) = method_type
                {
                    assert_eq!(type_params.len(), 2);
                    // type_params 是 Vec<Type>，检查类型名称
                    match &type_params[0] {
                        crate::frontend::core::parser::ast::Type::Name { name, .. } => {
                            assert_eq!(name, "Point")
                        }
                        _ => panic!("Expected Point type for first param"),
                    }
                } else {
                    panic!("Expected Fn type");
                }
                // Should have statements (dx = ..., dy = ...)
                assert!(stmts.len() >= 2);
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    #[test]
    fn test_parse_method_bind_no_params() {
        // RFC-010: Point.reset: () -> Void = { ... }
        let source = "Point.reset: () -> Void = { x = 0 }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                params,
                ..
            } => {
                assert_eq!(name, "reset");
                assert_eq!(*type_name, Some("Point".to_string()));
                // RFC-010: Check method type has 0 params
                if let Some(crate::frontend::core::parser::ast::Type::Fn {
                    params: type_params,
                    ..
                }) = method_type
                {
                    assert_eq!(type_params.len(), 0);
                } else {
                    panic!("Expected Fn type for method_type");
                }
                assert_eq!(params.len(), 0);
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    #[test]
    fn test_parse_method_bind_complex_types() {
        // RFC-010: List.map: (self: List(T), mapper: (T) -> U) -> List(U) = { ... }
        let source = "List.map: (self: List(T), mapper: (T) -> U) -> List(U) = { }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                params,
                ..
            } => {
                assert_eq!(name, "map");
                assert_eq!(*type_name, Some("List".to_string()));
                // RFC-010: method_type 存储完整签名
                if let Some(crate::frontend::core::parser::ast::Type::Fn {
                    params: type_params,
                    ..
                }) = method_type
                {
                    assert_eq!(type_params.len(), 2);
                    // type_params 是 Vec<Type>，检查类型名称
                    // 第一个参数应该是 List[T]
                    match &type_params[0] {
                        crate::frontend::core::parser::ast::Type::Generic { name, .. } => {
                            assert_eq!(name, "List")
                        }
                        _ => panic!(
                            "Expected Generic List type for first param, got {:?}",
                            type_params[0]
                        ),
                    }
                    // 第二个参数应该是 (T) -> U 函数类型
                    match &type_params[1] {
                        crate::frontend::core::parser::ast::Type::Fn { .. } => {}
                        _ => panic!(
                            "Expected Fn type for mapper param, got {:?}",
                            type_params[1]
                        ),
                    }
                } else {
                    panic!("Expected Fn type");
                }
                assert_eq!(params.len(), 0); // RFC-010: body 是代码块
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    #[test]
    fn test_parse_method_bind_with_method_call_in_body() {
        // RFC-010: Point.add: (self: Point, other: Point) -> Point = { ... }
        let source = r#"
            Point.add: (self: Point, other: Point) -> Point = {
                result = Point(self.x + other.x, self.y + other.y)
                result
            }
        "#;
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                body: (stmts, expr),
                ..
            } => {
                assert_eq!(name, "add");
                assert_eq!(*type_name, Some("Point".to_string()));
                // RFC-010: params 从签名解析
                match method_type {
                    Some(crate::frontend::core::parser::ast::Type::Fn {
                        params: type_params,
                        ..
                    }) => {
                        assert_eq!(type_params.len(), 2);
                    }
                    _ => panic!("Expected Fn type"),
                }
                assert!(stmts.len() >= 1);
                assert!(expr.is_some());
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    #[test]
    fn test_parse_method_bind_tuple_return() {
        // RFC-010: Point.decompose: (self: Point) -> (Float, Float) = (self.x, self.y)
        let source = "Point.decompose: (self: Point) -> (Float, Float) = (self.x, self.y)";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                body: (_stmts, expr),
                ..
            } => {
                assert_eq!(name, "decompose");
                assert_eq!(*type_name, Some("Point".to_string()));
                // Verify it parsed as Fn type
                if let Some(crate::frontend::core::parser::ast::Type::Fn { return_type, .. }) =
                    method_type
                {
                    match **return_type {
                        crate::frontend::core::parser::ast::Type::Tuple(_) => {}
                        _ => panic!("Expected Tuple return type"),
                    }
                } else {
                    panic!("Expected Fn type for method_type");
                }
                // RFC-010: 直接表达式形式
                assert!(expr.is_some());
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    #[test]
    fn test_parse_method_bind_and_function_together() {
        // Parse method binding and function definition together
        let source = r#"
            Point.x: (self: Point) -> Float = self.x

            get_value: (x: Int) -> Int = x + 1
        "#;
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 2);

        // First item should be Binding (method)
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                ..
            } => {
                assert_eq!(name, "x");
                assert_eq!(*type_name, Some("Point".to_string()));
                // RFC-010: Verify method_type has correct params
                match method_type {
                    Some(crate::frontend::core::parser::ast::Type::Fn {
                        params: type_params,
                        ..
                    }) => {
                        assert_eq!(type_params.len(), 1);
                    }
                    _ => panic!("Expected Fn type"),
                }
            }
            _ => panic!("Expected Binding statement (method) as first item"),
        }

        // Second item should be Binding (function)
        match &module.items[1].kind {
            StmtKind::Binding { name, .. } => {
                assert_eq!(name, "get_value");
            }
            _ => panic!("Expected Binding statement as second item"),
        }
    }

    #[test]
    fn test_parse_method_bind_with_if_in_body() {
        // RFC-010: if 表达式使用花括号语法
        let source = "Number.sign: (self: Number) -> String = if self.value > 0 { \"positive\" } else { \"non-positive\" }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                body: (stmts, expr),
                ..
            } => {
                assert_eq!(name, "sign");
                assert_eq!(*type_name, Some("Number".to_string()));
                assert!(stmts.is_empty());
                assert!(expr.is_some());
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    #[test]
    fn test_parse_method_bind_option_type() {
        // RFC-010: Optional.get_or_default: (self: Optional(T), default: T) -> T = { ... }
        let source = "Optional.get_or_default: (self: Optional(T), default: T) -> T = { return self.value or default }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_name,
                method_type,
                params,
                ..
            } => {
                assert_eq!(name, "get_or_default");
                assert_eq!(*type_name, Some("Optional".to_string()));
                // RFC-010: body 是代码块，params 为空（参数信息在 method_type 中）
                assert_eq!(params.len(), 0);
                // 验证方法类型签名有 2 个参数
                if let Some(ref fn_type) = method_type {
                    if let crate::frontend::core::parser::ast::Type::Fn {
                        params: type_params,
                        ..
                    } = fn_type
                    {
                        assert_eq!(type_params.len(), 2);
                    } else {
                        panic!("Expected Fn type for method_type");
                    }
                } else {
                    panic!("Expected Some for method_type");
                }
            }
            _ => panic!("Expected Binding statement for method"),
        }
    }

    // ======== RFC-010 接口约束语法测试 ========

    #[test]
    fn test_parse_struct_type_with_interface_constraint() {
        // RFC-010: Point: Type = { x: Float, y: Float, Drawable, Serializable }
        let source = "Point: Type = { x: Float, y: Float, Drawable, Serializable }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_annotation,
                generic_params,
                ..
            } => {
                assert_eq!(name, "Point");
                assert!(generic_params.is_empty());
                match type_annotation {
                    Some(crate::frontend::core::parser::ast::Type::Struct { fields, .. }) => {
                        assert_eq!(fields.len(), 2);
                        assert_eq!(fields[0].name, "x");
                        assert_eq!(fields[1].name, "y");
                    }
                    _ => panic!("Expected Struct type, got {:?}", type_annotation),
                }
            }
            _ => panic!("Expected Binding statement for type"),
        }
    }

    #[test]
    fn test_parse_struct_type_only_interfaces() {
        // RFC-010: EmptyType: Type = { Drawable, Serializable }
        let source = "EmptyType: Type = { Drawable, Serializable }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_annotation,
                generic_params,
                ..
            } => {
                assert_eq!(name, "EmptyType");
                assert!(generic_params.is_empty());
                match type_annotation {
                    Some(crate::frontend::core::parser::ast::Type::Struct { fields, .. }) => {
                        assert!(fields.is_empty()); // 只有接口约束
                    }
                    _ => panic!("Expected Struct type"),
                }
            }
            _ => panic!("Expected Binding statement for type"),
        }
    }

    #[test]
    fn test_parse_empty_struct_type() {
        // RFC-010: EmptyType: Type = {}
        let source = "EmptyType: Type = {}";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_annotation,
                generic_params,
                ..
            } => {
                assert_eq!(name, "EmptyType");
                assert!(generic_params.is_empty());
                match type_annotation {
                    Some(crate::frontend::core::parser::ast::Type::Struct { fields, .. }) => {
                        assert!(fields.is_empty());
                    }
                    _ => panic!("Expected Struct type"),
                }
            }
            _ => panic!("Expected Binding statement for type"),
        }
    }

    #[test]
    fn test_parse_interface_definition() {
        // RFC-010: Drawable: Type = { draw: (self: Self, surface: Surface) -> Void }
        let source = "Drawable: Type = { draw: (self: Self, surface: Surface) -> Void }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_annotation,
                ..
            } => {
                assert_eq!(name, "Drawable");
                match type_annotation {
                    Some(crate::frontend::core::parser::ast::Type::Struct { fields, .. }) => {
                        assert_eq!(fields.len(), 1);
                        assert_eq!(fields[0].name, "draw");
                    }
                    _ => panic!("Expected Struct type"),
                }
            }
            _ => panic!("Expected Binding statement for type"),
        }
    }

    #[test]
    fn test_parse_interface_definition_with_self() {
        // RFC-010: Serializable: Type = { serialize: (self: Self) -> String }
        let source = "Serializable: Type = { serialize: (self: Self) -> String }";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
        match &module.items[0].kind {
            StmtKind::Binding {
                name,
                type_annotation,
                ..
            } => {
                assert_eq!(name, "Serializable");
                match type_annotation {
                    Some(crate::frontend::core::parser::ast::Type::Struct { fields, .. }) => {
                        assert_eq!(fields.len(), 1);
                        assert_eq!(fields[0].name, "serialize");
                    }
                    _ => panic!("Expected Struct type"),
                }
            }
            _ => panic!("Expected Binding statement for type"),
        }
    }

    #[test]
    fn test_parse_generic_type_definition() {
        // RFC-010: Type alias for concrete types
        let source = "IntList: Type = List(Int)";
        let tokens = tokenize(source).unwrap();
        let module = parse(&tokens).unwrap();

        assert_eq!(module.items.len(), 1);
    }
}
