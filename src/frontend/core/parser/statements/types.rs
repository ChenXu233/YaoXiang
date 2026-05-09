//! Type annotation parsing and extension trait
//!
//! Implements parsing for:
//! - Type annotations: `name: Type`
//! - Function types: `(Params) -> ReturnType`
//! - Struct types: `{ field: Type }`
//! - Enum types: `{ Variant1 | Variant2 }`
//! - Tuple types: `(T, U, V)`
//! - Named struct types: `Name(x: Type, y: Type)`
//! - Constructor types: `Name(Type1, Type2)` — the ONLY generic application syntax
//! - Meta types: `Type`
//!
//! Also provides `TypeStatementParser` trait so callers can use
//! `state.parse_type_annotation()` instead of the free function.

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ast::StructField;
use crate::frontend::core::parser::{ParserState, ParseError, BP_LOWEST};
use crate::util::span::Span;

/// Extension trait providing `.parse_type_annotation()` on ParserState.
pub trait TypeStatementParser {
    /// Parse a type annotation
    fn parse_type_annotation(&mut self) -> Option<Type>;
}

impl TypeStatementParser for ParserState<'_> {
    fn parse_type_annotation(&mut self) -> Option<Type> {
        parse_type_annotation(self)
    }
}

/// Const parameter primitive types
pub(crate) const CONST_PARAM_TYPES: &[&str] = &[
    "Int", "Bool", "Float", "I8", "I16", "I32", "I64", "U8", "U16", "U32", "U64", "F32", "F64",
    "Char", "String",
];

#[allow(dead_code)]
fn looks_like_parenthesized_lambda(state: &mut ParserState<'_>) -> bool {
    if !state.at(&TokenKind::LParen) {
        return false;
    }

    let saved = state.save_position();
    state.bump();

    let mut depth = 1;
    while depth > 0 && !state.at_end() {
        if state.at(&TokenKind::LParen) {
            depth += 1;
        } else if state.at(&TokenKind::RParen) {
            depth -= 1;
        }
        state.bump();
    }

    let is_lambda = depth == 0 && state.at(&TokenKind::FatArrow);
    state.restore_position(saved);
    is_lambda
}

/// Parse type annotation
pub fn parse_type_annotation(state: &mut ParserState<'_>) -> Option<Type> {
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Star) => {
            // Raw pointer type: *T
            state.bump(); // consume '*'
            let inner_type = Box::new(parse_type_annotation(state)?);
            Some(Type::Ptr(inner_type))
        }
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            let name_span = state.span();
            state.bump();

            // RFC-010: `Type` is the meta-type keyword. Only () syntax is allowed.
            // `Type[T]` and `Type<T>` are rejected.
            if name == "Type" {
                // Reject old Type[T] or Type<T> syntax
                if state.at(&TokenKind::LBracket) || state.at(&TokenKind::Lt) {
                    state.error(ParseError::Message(
                        "Old 'Type[...]' or 'Type<...>' syntax is no longer supported. \
                         Use 'Type' alone for the meta-type, or '(T: Type, ...) -> Type' for type constructors."
                            .to_string(),
                    ));
                    return None;
                }
                return Some(Type::MetaType {
                    name_span,
                    args: Vec::new(),
                });
            }

            // Check for old curried function type: Type -> ReturnType (single param without parentheses)
            // This is OLD SYNTAX and should be rejected!
            // RFC-010 requires: (param: Type) -> ReturnType
            if state.at(&TokenKind::Arrow) {
                state.error(ParseError::Message(format!(
                    "Old curried function syntax '{} -> ...' is no longer supported. \
                     Use RFC-010 syntax with named parameters: '(param: {}) -> ReturnType'. \
                     Example: 'inc: (x: Int) -> Int = x => x + 1' instead of 'inc: Int -> Int = ...'",
                    name, name
                )));
                return None;
            }

            // Check for constructor/struct type: Name(params) or Name(x: Type, ...)
            if state.at(&TokenKind::LParen) {
                // Look ahead to determine if this is a named struct (has field names)
                // or a generic constructor (just types)
                let saved = state.save_position();
                state.bump(); // consume '('

                // Check if the first thing looks like "identifier:" (named field)
                let has_named_fields =
                    if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
                        matches!(state.peek().map(|t| &t.kind), Some(TokenKind::Colon))
                    } else {
                        false
                    };

                state.restore_position(saved);

                if has_named_fields {
                    // Parse as named struct: Name(x: Type, y: Type)
                    return parse_named_struct_type(name, name_span, state);
                } else {
                    // Parse as generic constructor: Name(Type1, Type2)
                    return parse_constructor_type(name, name_span, state);
                }
            }

            // Check for old angle bracket generic syntax: Name<Args>
            if state.at(&TokenKind::Lt) {
                state.error(ParseError::Message(
                    "Old 'Name<...>' angle bracket syntax is no longer supported. \
                     Use '()' application syntax: 'Name(Type1, Type2)'"
                        .to_string(),
                ));
                return None;
            }

            Some(Type::Name {
                name,
                span: name_span,
            })
        }
        Some(TokenKind::Lt) => {
            // RFC-010: angle bracket syntax `Type<Args>` is rejected
            state.error(ParseError::Message(
                "Old 'Type<...>' angle bracket syntax is no longer supported. \
                 Use '()' application syntax: 'Name(Type1, Type2)'"
                    .to_string(),
            ));
            None
        }
        Some(TokenKind::LParen) => {
            // This could be either:
            // 1. A tuple type: (T, U, V)
            // 2. A function type: (Params) -> ReturnType where Params may have names like (value: T)
            // We need to look ahead to check for ->

            let saved = state.save_position();

            // Try to parse as the parameter list of a function type
            state.bump(); // consume '('

            // Special case: check if first token is Identifier followed by Colon
            // This indicates named parameters like (value: T, x: Int)
            let has_named_params =
                if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
                    matches!(state.peek().map(|t| &t.kind), Some(TokenKind::Colon))
                } else {
                    false
                };

            let mut param_types = Vec::new();

            if has_named_params {
                // Parse named parameters: (name: Type, name: Type, ...) -> ReturnType
                // For function type annotation, we only care about the types
                while !state.at(&TokenKind::RParen) && !state.at_end() {
                    // Skip parameter name
                    if let Some(TokenKind::Identifier(_name)) = state.current().map(|t| &t.kind) {
                        state.bump(); // consume name

                        // Expect colon and type
                        if !state.skip(&TokenKind::Colon) {
                            break;
                        }

                        // Parse the type
                        if let Some(ty) = parse_type_annotation(state) {
                            param_types.push(ty);
                        } else {
                            break;
                        }

                        // Skip comma if present
                        if !state.skip(&TokenKind::Comma) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            } else {
                // Parse types without names: (Type, Type, ...) -> ReturnType
                if !state.at(&TokenKind::RParen) {
                    while let Some(ty) = parse_type_annotation(state) {
                        param_types.push(ty);

                        if !state.skip(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
            }

            if !state.skip(&TokenKind::RParen) {
                state.restore_position(saved);
                return parse_tuple_type(state);
            }

            // Check if followed by ->
            if state.at(&TokenKind::Arrow) {
                state.bump(); // consume '->'
                let return_type = Box::new(parse_type_annotation(state)?);
                return Some(Type::Fn {
                    params: param_types,
                    return_type,
                });
            }

            // Not a function type, just a tuple
            if param_types.len() == 1 {
                // Single element in parentheses, not a tuple
                Some(param_types.pop().unwrap())
            } else {
                Some(Type::Tuple(param_types))
            }
        }
        Some(TokenKind::LBrace) => parse_struct_type(state),
        Some(TokenKind::LBracket) => {
            // RFC-010: `[T, U](params) -> Ret` syntax is removed.
            state.error(ParseError::Message(
                "Old generic function type syntax '[T, U](params) -> Ret' is no longer supported. \
                 Use RFC-010 syntax: '(T: Type, U: Type) -> ((params) -> Ret)'"
                    .to_string(),
            ));
            None
        }
        // Note: fn type uses (Params) -> ReturnType syntax, not `fn` keyword
        _ => None,
    }
}

/// Parse named struct type: `Name(x: Type, y: Type)` or `Name(mut x: Type, y: Type)`
fn parse_named_struct_type(
    name: String,
    name_span: Span,
    state: &mut ParserState<'_>,
) -> Option<Type> {
    state.bump(); // consume '('

    let mut fields = Vec::new();

    while !state.at(&TokenKind::RParen) && !state.at_end() {
        // 检查是否有关键字 mut
        let is_mut = state.skip(&TokenKind::KwMut);

        let field_name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        if !state.expect(&TokenKind::Colon) {
            return None;
        }

        let field_type = parse_type_annotation(state)?;
        fields.push(StructField::new(field_name, is_mut, field_type));

        if !state.skip(&TokenKind::Comma) {
            break;
        }
    }

    state.expect(&TokenKind::RParen);

    Some(Type::NamedStruct {
        name,
        name_span,
        fields,
    })
}

/// Parse constructor type: `Name(Type1, Type2)`
fn parse_constructor_type(
    name: String,
    name_span: Span,
    state: &mut ParserState<'_>,
) -> Option<Type> {
    state.bump(); // consume '('

    let mut args = Vec::new();

    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let arg = parse_type_annotation(state)?;
        args.push(arg);

        if !state.skip(&TokenKind::Comma) {
            break;
        }
    }

    state.expect(&TokenKind::RParen);

    // Lower well-known generic types to dedicated AST nodes.
    match (name.as_str(), args.len()) {
        ("Option", 1) => Some(Type::Option(Box::new(args.into_iter().next()?))),
        ("Result", 2) => {
            let mut it = args.into_iter();
            let ok = it.next()?;
            let err = it.next()?;
            Some(Type::Result(Box::new(ok), Box::new(err)))
        }
        _ => Some(Type::Generic {
            name,
            name_span,
            args,
        }),
    }
}

/// Parse function type with parameter names: `(a: Int, b: Int) -> Int`
/// Returns (Vec<Param>, return_type)
/// This is for RFC-010 unified syntax: `name: (a: Int, b: Int) -> Ret = body`
/// Also supports const generic literal types: `factorial: [n: Int](n: n) -> Int`
pub fn parse_fn_type_with_names(state: &mut ParserState<'_>) -> Option<(Vec<Param>, Box<Type>)> {
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    let mut params = Vec::new();

    // Check for empty params: ()
    if !state.at(&TokenKind::RParen) {
        while !state.at(&TokenKind::RParen) && !state.at_end() {
            // Skip comma between params
            if !params.is_empty() && !state.skip(&TokenKind::Comma) {
                break;
            }

            let param_span = state.span();

            // Check for mut keyword
            let is_mut = state.skip(&TokenKind::KwMut);

            // Parse parameter name
            let name = match state.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => break,
            };
            state.bump();

            // RFC-007: 类型标注是可选的（HM 推断）
            // 检查是否有冒号和类型
            let ty = if state.skip(&TokenKind::Colon) {
                // Parse type annotation
                // Check if this is a literal type (const parameter reference)
                // e.g., (n: n) where n is a const generic parameter
                let parsed_type = parse_type_annotation(state)?;

                // Wrap in Literal if it's an identifier matching the parameter name
                // This handles const generic literal types like (n: n)
                Some(wrap_literal_type_if_needed(name.clone(), parsed_type))
            } else {
                // 无类型标注，HM 推断
                None
            };

            params.push(Param {
                name,
                ty,
                is_mut,
                span: param_span,
            });
        }
    }

    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // Expect ->
    if !state.expect(&TokenKind::Arrow) {
        return None;
    }

    let return_type = Box::new(parse_type_annotation(state)?);

    Some((params, return_type))
}

/// Wrap a type annotation as a literal type if it's a const parameter reference
fn wrap_literal_type_if_needed(
    param_name: String,
    parsed_type: Type,
) -> Type {
    match &parsed_type {
        Type::Name { name, span } if name == &param_name => Type::Literal {
            name: param_name.clone(),
            name_span: *span,
            base_type: Box::new(Type::Name {
                name: param_name,
                span: *span,
            }),
        },
        _ => parsed_type,
    }
}

/// Parse tuple type like `(T, U, V)`
fn parse_tuple_type(state: &mut ParserState<'_>) -> Option<Type> {
    state.skip(&TokenKind::LParen);

    let mut types = Vec::new();

    if !state.at(&TokenKind::RParen) {
        while let Some(ty) = parse_type_annotation(state) {
            types.push(ty);

            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }

    state.skip(&TokenKind::RParen);

    Some(Type::Tuple(types))
}

/// Parse struct type like `{ field: Type }` or `{ field: Type, InterfaceName }`
fn parse_struct_type(state: &mut ParserState<'_>) -> Option<Type> {
    state.skip(&TokenKind::LBrace);

    let mut fields = Vec::new();
    let mut bindings = Vec::new();
    let mut interfaces = Vec::new();

    if !state.at(&TokenKind::RBrace) {
        while let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
            let name = name.clone();
            state.bump();

            // 检查下一个 token 是否是 mut 或冒号
            let is_mut = state.skip(&TokenKind::KwMut);

            // 检查下一个 token 是否是冒号
            if state.at(&TokenKind::Colon) {
                // 有冒号: 字段声明或匿名函数绑定
                state.bump(); // consume ':'
                let field_type = parse_type_annotation(state)?;

                // 检查是否有位置绑定: [positions]
                let positions = parse_optional_binding_positions(state);

                if state.skip(&TokenKind::Eq) {
                    if let Some(pos) = positions {
                        // 匿名函数绑定: name: FnType[pos] = lambda
                        let body_expr = state.parse_expression(BP_LOWEST)?;
                        let (params, return_type) = extract_fn_type_info(&field_type);
                        bindings.push(TypeBodyBinding {
                            name,
                            kind: BindingKind::Anonymous {
                                params,
                                return_type: Box::new(return_type),
                                positions: pos,
                                body: Box::new(body_expr),
                            },
                        });
                    } else {
                        // 默认值字段: name: Type = expression
                        let default_expr = state.parse_expression(BP_LOWEST)?;
                        fields.push(StructField::with_default(
                            name,
                            is_mut,
                            field_type,
                            default_expr,
                        ));
                    }
                } else {
                    // 普通字段: name: Type
                    fields.push(StructField::new(name, is_mut, field_type));
                }
            } else if state.skip(&TokenKind::Eq) {
                // 无冒号但有等号: 外部函数绑定 name = function[positions] 或默认绑定 name = function
                let func_name = match state.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(n)) => n.clone(),
                    _ => {
                        state.error(ParseError::Message(format!(
                            "Expected function name after '=' in binding '{}'",
                            name
                        )));
                        return None;
                    }
                };
                state.bump(); // consume function name

                // RFC-004: 尝试解析位置绑定 [positions]，如果没有则为默认绑定
                if state.at(&TokenKind::LBracket) {
                    let positions = parse_binding_positions(state).ok()?;
                    bindings.push(TypeBodyBinding {
                        name,
                        kind: BindingKind::External {
                            function: func_name,
                            positions,
                        },
                    });
                } else {
                    // 默认绑定: name = function（自动查找第一个类型匹配位置）
                    bindings.push(TypeBodyBinding {
                        name,
                        kind: BindingKind::DefaultExternal {
                            function: func_name,
                        },
                    });
                }
            } else if is_mut {
                // mut 后面没有冒号是语法错误
                state.error(ParseError::Message(format!(
                    "Expected ':' after 'mut' in field '{}'",
                    name
                )));
                return None;
            } else if state.at(&TokenKind::Pipe) || state.at(&TokenKind::LParen) {
                // 枚举变体: red | green | blue 或 ok(Int) | err(String)
                // 回退一个 token，从头开始解析枚举
                state.restore_position(state.save_position() - 1);
                return parse_enum_variants_in_braces(state);
            } else {
                // 接口约束: InterfaceName
                interfaces.push(name);
            }

            // 跳过逗号，如果不是逗号则结束循环
            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }

    state.skip(&TokenKind::RBrace);

    Some(Type::Struct {
        fields,
        bindings,
        interfaces,
    })
}

/// 解析花括号内的枚举变体: { red | green | blue }
fn parse_enum_variants_in_braces(state: &mut ParserState<'_>) -> Option<Type> {
    let first_variant = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            let name_span = state.span();
            state.bump();

            // 检查是否有参数类型: ok(Int)
            let params = if state.at(&TokenKind::LParen) {
                state.bump(); // consume '('
                let mut params = Vec::new();
                while !state.at(&TokenKind::RParen) && !state.at_end() {
                    if let Some(param_type) = parse_type_annotation(state) {
                        params.push((None, param_type));
                        state.skip(&TokenKind::Comma);
                    } else {
                        break;
                    }
                }
                state.skip(&TokenKind::RParen);
                params
            } else {
                Vec::new()
            };

            VariantDef {
                name,
                name_span,
                params,
                span: state.span(),
            }
        }
        _ => {
            state.error(ParseError::Message("Expected variant name".to_string()));
            return None;
        }
    };

    // 收集后续变体
    let mut variants = vec![first_variant];
    while state.skip(&TokenKind::Pipe) {
        match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                let name_span = state.span();
                state.bump();

                // 检查是否有参数类型: ok(Int)
                let params = if state.at(&TokenKind::LParen) {
                    state.bump(); // consume '('
                    let mut params = Vec::new();
                    while !state.at(&TokenKind::RParen) && !state.at_end() {
                        if let Some(param_type) = parse_type_annotation(state) {
                            params.push((None, param_type));
                            state.skip(&TokenKind::Comma);
                        } else {
                            break;
                        }
                    }
                    state.skip(&TokenKind::RParen);
                    params
                } else {
                    Vec::new()
                };

                variants.push(VariantDef {
                    name,
                    name_span,
                    params,
                    span: state.span(),
                });
            }
            _ => {
                state.error(ParseError::Message(
                    "Expected variant name after '|'".to_string(),
                ));
                break;
            }
        }
    }

    state.skip(&TokenKind::RBrace);

    Some(Type::Variant(variants))
}

/// 解析可选的位置绑定: `[0]` 或 `[0, 1]`
fn parse_optional_binding_positions(state: &mut ParserState<'_>) -> Option<Vec<i64>> {
    if !state.at(&TokenKind::LBracket) {
        return None;
    }

    // 前瞻检查: 确认 `[` 后面是整数字面量或负号
    let saved = state.save_position();
    state.bump(); // consume '['

    match state.current().map(|t| &t.kind) {
        Some(TokenKind::IntLiteral(_)) | Some(TokenKind::Minus) => {
            state.restore_position(saved);
            // 是位置绑定，解析之
            parse_binding_positions(state).ok()
        }
        _ => {
            // 不是位置绑定（可能是泛型参数）
            state.restore_position(saved);
            None
        }
    }
}

/// 解析位置绑定: `[0]` 或 `[0, 1]` 或 `[-1]`（必须存在）
pub(crate) fn parse_binding_positions(state: &mut ParserState<'_>) -> Result<Vec<i64>, ()> {
    if !state.at(&TokenKind::LBracket) {
        state.error(ParseError::Message(
            "Expected '[' for binding position".to_string(),
        ));
        return Err(());
    }
    state.bump(); // consume '['

    let mut positions = Vec::new();
    while !state.at(&TokenKind::RBracket) && !state.at_end() {
        // RFC-004: 支持负数索引 `[-1]`
        let is_negative = state.at(&TokenKind::Minus);
        if is_negative {
            state.bump(); // consume '-'
        }

        match state.current().map(|t| &t.kind) {
            Some(TokenKind::IntLiteral(n)) => {
                let value = *n as i64;
                positions.push(if is_negative { -value } else { value });
                state.bump();
                state.skip(&TokenKind::Comma);
            }
            _ => {
                state.error(ParseError::Message(
                    "Expected integer position in binding".to_string(),
                ));
                return Err(());
            }
        }
    }

    if !state.at(&TokenKind::RBracket) {
        state.error(ParseError::Message(
            "Expected ']' after binding positions".to_string(),
        ));
        return Err(());
    }
    state.bump(); // consume ']'

    Ok(positions)
}

/// 从函数类型中提取参数和返回类型
fn extract_fn_type_info(ty: &Type) -> (Vec<Param>, Type) {
    match ty {
        Type::Fn {
            params,
            return_type,
        } => {
            let param_list = params
                .iter()
                .enumerate()
                .map(|(i, p)| Param {
                    name: format!("arg{}", i),
                    ty: Some(p.clone()),
                    is_mut: false,
                    span: Span::dummy(),
                })
                .collect();
            (param_list, *return_type.clone())
        }
        _ => (Vec::new(), Type::Void),
    }
}
