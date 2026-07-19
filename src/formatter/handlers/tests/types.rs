//! 类型格式化处理器测试
//!
//! 对应 formatter 规范 §9 (type annotations)

use crate::formatter::context::FormatContext;
use crate::formatter::handlers::types::format_type;
use crate::formatter::options::FormatOptions;
use crate::formatter::source_map::SourceMap;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast::*;
use crate::util::span::Span;

fn default_ctx() -> FormatContext {
    FormatContext::new(FormatOptions::default())
}

fn default_source_map() -> SourceMap {
    SourceMap::build("")
}

#[test]
fn test_format_type_int() {
    assert_eq!(
        format_type(&Type::Int(32), &default_ctx(), &default_source_map()),
        "i32"
    );
    assert_eq!(
        format_type(&Type::Int(64), &default_ctx(), &default_source_map()),
        "i64"
    );
}

#[test]
fn test_format_type_float() {
    assert_eq!(
        format_type(&Type::Float(32), &default_ctx(), &default_source_map()),
        "f32"
    );
    assert_eq!(
        format_type(&Type::Float(64), &default_ctx(), &default_source_map()),
        "f64"
    );
}

#[test]
fn test_format_type_bool() {
    assert_eq!(
        format_type(&Type::Bool, &default_ctx(), &default_source_map()),
        "Bool"
    );
}

#[test]
fn test_format_type_string() {
    assert_eq!(
        format_type(&Type::String, &default_ctx(), &default_source_map()),
        "String"
    );
}

#[test]
fn test_format_type_char() {
    assert_eq!(
        format_type(&Type::Char, &default_ctx(), &default_source_map()),
        "Char"
    );
}

#[test]
fn test_format_type_void() {
    assert_eq!(
        format_type(&Type::Void, &default_ctx(), &default_source_map()),
        "()"
    );
}

#[test]
fn test_format_type_tuple() {
    let ty = Type::Tuple(vec![Type::Int(32), Type::Bool]);
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "(i32, Bool)"
    );
}

#[test]
fn test_format_type_option() {
    let ty = Type::Option(Box::new(Type::Int(32)));
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "i32?"
    );
}

#[test]
fn test_format_type_fn() {
    let ty = Type::Fn {
        params: vec![Type::Int(32), Type::Bool],
        return_type: Box::new(Type::String),
    };
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "(i32, Bool) -> String"
    );
}

#[test]
fn test_format_type_ref() {
    let ty = Type::Ref {
        mutable: false,
        inner: Box::new(Type::Int(32)),
        span: Span::dummy(),
    };
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "&i32"
    );
}

#[test]
fn test_format_type_mut_ref() {
    let ty = Type::Ref {
        mutable: true,
        inner: Box::new(Type::Int(32)),
        span: Span::dummy(),
    };
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "&mut i32"
    );
}

#[test]
fn test_format_type_ptr() {
    let ty = Type::Ptr(Box::new(Type::Int(32)));
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "*i32"
    );
}

#[test]
fn test_format_type_name() {
    let ty = Type::Name {
        name: "MyType".to_string(),
        span: Span::dummy(),
    };
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "MyType"
    );
}

#[test]
fn test_format_type_enum() {
    let ty = Type::Enum(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "A | B | C"
    );
}

#[test]
fn test_format_type_sum() {
    let ty = Type::Sum(vec![Type::Int(32), Type::Bool]);
    assert_eq!(
        format_type(&ty, &default_ctx(), &default_source_map()),
        "i32 + Bool"
    );
}

#[test]
fn test_format_type_const_expr() {
    // 覆盖: formatter 规范 §9 — 编译期表达式类型须还原为源码
    // 验证: Type::ConstExpr(N < 100) 格式化为 "N < 100"，而非 "<const-expr>" 占位符
    let expr = Expr::BinOp {
        op: BinOp::Lt,
        left: Box::new(Expr::Var("N".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(100), Span::dummy())),
        span: Span::dummy(),
    };
    let ty = Type::ConstExpr(Box::new(expr));

    let result = format_type(&ty, &default_ctx(), &default_source_map());

    assert_eq!(result, "N < 100", "ConstExpr 应还原为源码表达式而非占位符");
}

#[test]
fn test_format_type_generic_with_const_expr_arg() {
    // 覆盖: formatter 规范 §9 — 泛型参数位置的编译期表达式
    // 验证: Assert(N < 100) 整体格式化为 "Assert(N < 100)"
    let expr = Expr::BinOp {
        op: BinOp::Lt,
        left: Box::new(Expr::Var("N".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(100), Span::dummy())),
        span: Span::dummy(),
    };
    let ty = Type::Generic {
        name: "Assert".to_string(),
        name_span: Span::dummy(),
        args: vec![Type::ConstExpr(Box::new(expr))],
    };

    let result = format_type(&ty, &default_ctx(), &default_source_map());

    assert_eq!(
        result, "Assert(N < 100)",
        "Generic 内的 ConstExpr 应还原为源码表达式"
    );
}

#[test]
fn test_format_struct_preserves_body_order() {
    // 覆盖: formatter 规范 §9 — 类型体是有序代码块，按声明顺序输出
    // 验证: Field / Expr / Field 混合 body 不打乱顺序，Expr 项不丢失
    let gt_expr = Expr::BinOp {
        op: BinOp::Gt,
        left: Box::new(Expr::Var("N".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
        span: Span::dummy(),
    };
    let body = vec![
        TypeBodyItem::Field(StructField::new("x".to_string(), false, Type::Int(32))),
        TypeBodyItem::Expr(Type::Generic {
            name: "Assert".to_string(),
            name_span: Span::dummy(),
            args: vec![Type::ConstExpr(Box::new(gt_expr))],
        }),
        TypeBodyItem::Field(StructField::new("y".to_string(), false, Type::Int(32))),
    ];
    let ty = Type::Struct { body };

    let result = format_type(&ty, &default_ctx(), &default_source_map());

    assert_eq!(
        result, "{ x: i32, Assert(N > 0), y: i32 }",
        "类型体项应保持声明顺序，匿名 Expr 项不得丢弃"
    );
}

#[test]
fn test_format_struct_merges_consecutive_interfaces() {
    // 覆盖: formatter 规范 §9 — 连续 Interface 项合并为 impl 子句
    // 验证: 连续接口合并输出 "impl Eq, Hash"，且位置保持在首个接口出现处
    let body = vec![
        TypeBodyItem::Interface("Eq".to_string()),
        TypeBodyItem::Interface("Hash".to_string()),
        TypeBodyItem::Field(StructField::new("x".to_string(), false, Type::Int(32))),
    ];
    let ty = Type::Struct { body };

    let result = format_type(&ty, &default_ctx(), &default_source_map());

    assert_eq!(
        result, "{ impl Eq, Hash, x: i32 }",
        "连续 Interface 项应合并为一个 impl 子句"
    );
}

#[test]
fn test_format_type_body_binding_external() {
    // 覆盖: formatter 规范 §9 — 类型体外部绑定还原为 name = function[pos] 语法
    // 验证: External 带 positions 输出位置列表；空 positions 退化为 name = function
    let with_pos = Type::Struct {
        body: vec![TypeBodyItem::Binding(TypeBodyBinding {
            name: "get".to_string(),
            kind: BindingKind::External {
                function: "array_get".to_string(),
                positions: vec![0, 2],
            },
        })],
    };
    let no_pos = Type::Struct {
        body: vec![TypeBodyItem::Binding(TypeBodyBinding {
            name: "get".to_string(),
            kind: BindingKind::External {
                function: "array_get".to_string(),
                positions: vec![],
            },
        })],
    };

    assert_eq!(
        format_type(&with_pos, &default_ctx(), &default_source_map()),
        "{ get = array_get[0, 2] }",
        "External 绑定应输出 positions 列表"
    );
    assert_eq!(
        format_type(&no_pos, &default_ctx(), &default_source_map()),
        "{ get = array_get }",
        "空 positions 应退化为无位置形式"
    );
}

#[test]
fn test_format_type_body_binding_default_and_anonymous() {
    // 覆盖: formatter 规范 §9 — 类型体绑定 DefaultExternal 与 Anonymous 形态
    // 验证: DefaultExternal 输出 name = function；Anonymous 输出完整签名与 lambda
    let default_binding = Type::Struct {
        body: vec![TypeBodyItem::Binding(TypeBodyBinding {
            name: "get".to_string(),
            kind: BindingKind::DefaultExternal {
                function: "array_get".to_string(),
            },
        })],
    };
    let anonymous = Type::Struct {
        body: vec![TypeBodyItem::Binding(TypeBodyBinding {
            name: "get".to_string(),
            kind: BindingKind::Anonymous {
                params: vec![Param {
                    name: "i".to_string(),
                    ty: Some(Type::Name {
                        name: "Int".to_string(),
                        span: Span::dummy(),
                    }),
                    is_mut: false,
                    span: Span::dummy(),
                }],
                return_type: Box::new(Type::Name {
                    name: "T".to_string(),
                    span: Span::dummy(),
                }),
                positions: vec![0],
                body: Box::new(Expr::Var("item".to_string(), Span::dummy())),
            },
        })],
    };

    assert_eq!(
        format_type(&default_binding, &default_ctx(), &default_source_map()),
        "{ get = array_get }",
        "DefaultExternal 绑定应输出 name = function"
    );
    assert_eq!(
        format_type(&anonymous, &default_ctx(), &default_source_map()),
        "{ get: ((i: Int) -> T)[0] = ((i: Int) => item) }",
        "Anonymous 绑定应输出完整签名与 lambda 体"
    );
}
#[test]
fn test_format_generic_type_params_rules() {
    // 覆盖: RFC-010 泛型类型定义 — 参数签名重建须带类型标注
    // 验证: Type 无约束 → "T: Type"；Type 带约束 → "T: Clone"；Const → "N: Int"；Platform → "P"
    use crate::formatter::handlers::common::format_generic_type_params;

    let type_no_constraint = vec![GenericParam {
        name: "T".to_string(),
        kind: GenericParamKind::Type,
        constraints: vec![],
    }];
    let type_with_constraint = vec![GenericParam {
        name: "T".to_string(),
        kind: GenericParamKind::Type,
        constraints: vec![Type::Name {
            name: "Clone".to_string(),
            span: Span::dummy(),
        }],
    }];
    let const_param = vec![GenericParam {
        name: "N".to_string(),
        kind: GenericParamKind::Const {
            const_type: Box::new(Type::Name {
                name: "Int".to_string(),
                span: Span::dummy(),
            }),
        },
        constraints: vec![],
    }];
    let platform = vec![GenericParam {
        name: "P".to_string(),
        kind: GenericParamKind::Platform,
        constraints: vec![],
    }];

    assert_eq!(
        format_generic_type_params(&type_no_constraint, &default_ctx(), &default_source_map()),
        "(T: Type)",
        "Type 参数无约束应补 Type 标注"
    );
    assert_eq!(
        format_generic_type_params(&type_with_constraint, &default_ctx(), &default_source_map()),
        "(T: Clone)",
        "Type 参数有约束应输出约束名"
    );
    assert_eq!(
        format_generic_type_params(&const_param, &default_ctx(), &default_source_map()),
        "(N: Int)",
        "Const 参数应输出 const_type 标注"
    );
    assert_eq!(
        format_generic_type_params(&platform, &default_ctx(), &default_source_map()),
        "(P)",
        "Platform 参数无约束时不输出标注"
    );
}
