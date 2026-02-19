//! Declaration parsing - variables, types, imports, functions
//!
//! Implements parsing for:
//! - Variable declarations: `[mut] name[: type] [= expr];`
//! - Type definitions: `Name: Type = Type;`
//! - Use imports: `use path;` or `use path.{item1, item2};`
//! - Function definitions: `name: (ParamTypes) -> ReturnType = (params) => body;`
//! - Mutability fields: `type Point = { x: Float, mut y: Float }`

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::{StructField, *};
use crate::frontend::core::parser::{ParserState, ParseError, BP_LOWEST};
use crate::util::span::Span;
use std::collections::HashSet;

/// Const parameter primitive types
const CONST_PARAM_TYPES: &[&str] = &[
    "Int", "Bool", "Float", "I8", "I16", "I32", "I64", "U8", "U16", "U32", "U64", "F32", "F64",
    "Char", "String",
];

/// Lazy-initialized set of const parameter types for O(1) lookup
fn const_param_type_set() -> &'static HashSet<&'static str> {
    use std::sync::OnceLock;
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| CONST_PARAM_TYPES.iter().copied().collect())
}

/// Extract parameter name from a Type for generic_params
/// Returns None if the type is complex (e.g., MetaType) and doesn't have a simple name
fn extract_type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Name(name) => Some(name.clone()),
        // For MetaType like Type[T], extract T's name
        Type::MetaType { args } => {
            if args.len() == 1 {
                extract_type_name(&args[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 检测是否是旧函数定义语法: identifier( 后跟类型参数和 ->
/// 旧语法示例: add(Int, Int) -> Int = (a, b) => a + b
/// 已被弃用，移除支持
fn is_old_function_syntax(state: &mut ParserState<'_>) -> bool {
    // 保存当前位置
    let saved = state.save_position();

    // 跳过标识符
    state.bump();

    // 检查是否是 (
    if !state.at(&TokenKind::LParen) {
        state.restore_position(saved);
        return false;
    }
    state.bump(); // consume '('

    // 跳过括号内的内容，计算括号深度
    let mut paren_depth = 1;
    while paren_depth > 0 && !state.at_end() {
        if state.at(&TokenKind::LParen) {
            paren_depth += 1;
        } else if state.at(&TokenKind::RParen) {
            paren_depth -= 1;
        }
        state.bump();
    }

    // 检查括号后是否是 ->，这是旧语法的特征
    // 旧语法: add(Int, Int) -> Int = (a, b) => a + b
    //          ^^^^^^^^ 这里的箭头是类型注解的一部分
    // 注意: 旧语法中 -> 后面是返回类型，然后才是 =
    let is_old = state.at(&TokenKind::Arrow);

    // 恢复位置
    state.restore_position(saved);

    is_old
}

/// 跳过旧函数语法的整个声明（已移除支持，保持兼容性）
fn skip_old_function_syntax(_state: &mut ParserState<'_>) {
    // 旧语法已移除，此函数不再需要
}

/// 检测是否是方法绑定语法: `Type.method: (Params) -> ReturnType`
/// 模式: Identifier . Identifier :
fn is_method_bind_syntax(state: &mut ParserState<'_>) -> bool {
    let saved = state.save_position();

    // 检查是否是 Identifier (类型名)
    let has_type_name = matches!(
        state.current().map(|t| &t.kind),
        Some(TokenKind::Identifier(_))
    );

    if has_type_name {
        state.bump(); // consume type name

        // 检查是否是点号
        let has_dot = state.at(&TokenKind::Dot);
        if has_dot {
            state.bump(); // consume dot

            // 检查是否是 Identifier (方法名)
            let has_method_name = matches!(
                state.current().map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
            );

            if has_method_name {
                state.bump(); // consume method name

                // 检查是否是冒号 (类型注解开始)
                let has_colon = state.at(&TokenKind::Colon);
                state.restore_position(saved);
                return has_colon;
            }
        }
    }

    state.restore_position(saved);
    false
}

/// Parse method binding statement: `Type.method: (Params) -> ReturnType = (params) => body`
/// Example: `Point.draw: (Point, Surface) -> Void = (self, surface) => { ... }`
pub fn parse_method_bind_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // Parse type name
    let type_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => {
            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
            return None;
        }
    };
    state.bump(); // consume type name

    // Expect dot
    if !state.expect(&TokenKind::Dot) {
        return None;
    }

    // Parse method name
    let method_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => {
            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
            return None;
        }
    };
    state.bump(); // consume method name

    // Expect colon
    if !state.expect(&TokenKind::Colon) {
        return None;
    }

    // Parse method type annotation
    let method_type = match parse_type_annotation(state) {
        Some(t) => t,
        None => {
            state.error(ParseError::Message(
                "Expected type annotation after ':' in method binding".to_string(),
            ));
            return None;
        }
    };

    // Expect equals sign
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // Parse method body - should be a lambda expression: (params) => body
    // We need to parse this as an expression and extract the lambda
    let initializer = match state.parse_expression(BP_LOWEST) {
        Some(expr) => expr,
        None => {
            state.error(ParseError::Message(
                "Expected method body after '=' in method binding".to_string(),
            ));
            return None;
        }
    };

    // Parse method body
    // RFC-010 支持三种形式：
    // 1. 旧语法: (self, surface) => { ... }      - Lambda 表达式
    // 2. 新语法: { ... }                          - 代码块（参数已在签名中声明）
    // 3. 直接表达式: value                        - 表达式（无 return）
    let (params, body_stmts, body_expr) = match &initializer {
        Expr::Lambda { params, body, .. } => {
            // 旧语法：提取 Lambda 的参数和体
            (params.clone(), body.stmts.clone(), body.expr.clone())
        }
        Expr::Block(block) => {
            // RFC-010 新语法：代码块体，参数已在签名中声明
            (Vec::new(), block.stmts.clone(), block.expr.clone())
        }
        expr => {
            // 直接表达式形式
            (Vec::new(), Vec::new(), Some(Box::new(expr.clone())))
        }
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::MethodBind {
            type_name,
            method_name,
            method_type,
            params,
            body: (body_stmts, body_expr),
        },
        span,
    })
}

/// Parse variable declaration: `[mut] [pub] name[: type] [= expr];`
/// Function definition: `[pub] name: (ParamTypes) -> ReturnType = (params) => body;`
/// Generic function: `[pub] name[T: Clone]: (ParamTypes) -> ReturnType = (params) => body;`
pub fn parse_var_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    parse_var_stmt_with_pub(state, span, None)
}

/// Parse variable declaration with optional pre-detected pub modifier
fn parse_var_stmt_with_pub(
    state: &mut ParserState<'_>,
    span: Span,
    pre_detected_pub: Option<bool>,
) -> Option<Stmt> {
    // Check for mutability
    let is_mut = state.skip(&TokenKind::KwMut);

    // Check for pub modifier (only if not already detected)
    // 如果 pre_detected_pub 为 Some(true)，说明 pub 已被调用者消费
    let final_is_pub = if pre_detected_pub == Some(true) {
        // pub already consumed by caller, skip detection but keep true
        state.skip(&TokenKind::KwPub);
        true
    } else {
        state.skip(&TokenKind::KwPub)
    };

    // Parse variable name (identifier)
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => {
            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
            return None;
        }
    };
    state.bump();

    // RFC-010: 检测并拒绝旧语法: name(Type, ...) -> Ret = ...
    // 旧语法特征: 标识符后面直接跟着 (，没有 :
    if state.at(&TokenKind::LParen) {
        // 这看起来像旧函数语法，尝试确认
        let saved = state.save_position();
        state.bump(); // consume '('

        // 跳过括号内容
        let mut paren_depth = 1;
        while paren_depth > 0 && !state.at_end() {
            if state.at(&TokenKind::LParen) {
                paren_depth += 1;
            } else if state.at(&TokenKind::RParen) {
                paren_depth -= 1;
            }
            state.bump();
        }

        // 检查括号后是否是 -> (旧语法的特征)
        let is_old_syntax = state.at(&TokenKind::Arrow);
        state.restore_position(saved);

        if is_old_syntax {
            // 拒绝旧语法
            state.error(ParseError::Message(
                "旧函数语法已不再支持，请使用新语法: name: (param: Type, ...) -> Ret = body"
                    .to_string(),
            ));
            return None;
        }
    }

    // RFC-010 新语法: name: (a: Int, b: Int) -> Ret = body (参数名在签名中)
    // 特征: 冒号后面跟着函数类型 (param: Type, ...) -> Ret
    // 包括空参数 () 也属于 RFC-010 语法
    let (type_annotation, generic_params, fn_params) = if state.at(&TokenKind::Colon) {
        state.bump(); // consume ':'

        // Check if this is a generic param declaration: [T: Clone] or <T: Clone>
        // These come right after ':'
        let has_generic_syntax = state.at(&TokenKind::LBracket) || state.at(&TokenKind::Lt);

        if has_generic_syntax {
            // Parse generic params first
            let generic = parse_generic_params_with_constraints(state)?;

            // After generic params, check for RFC-010 new syntax: () or (param: Type, ...) -> Ret
            if state.at(&TokenKind::LParen) {
                // Look ahead to check if this is RFC-010 function syntax
                // Key: must have -> after the closing paren to be a function type
                // RFC-010/RFC-007 syntax:
                //   - (name: Type, ...) - named params with types
                //   - (name, name, ...) - named params without types (HM inference)
                // Old syntax:
                //   - (Type, Type, ...) - types only (NO param names)
                //
                // To distinguish: params are lowercase, types are Uppercase
                let saved = state.save_position();
                state.bump(); // consume '('

                // Check if this is RFC-010/RFC-007 compatible:
                // - Empty params () is always compatible
                // - Identifier followed by ':' is a named param with type
                // - Identifier followed by ',' or ')' could be either:
                //   - param name (lowercase) -> RFC-007 style
                //   - type name (Uppercase) -> old syntax
                let looks_like_named_params = if state.at(&TokenKind::RParen) {
                    // Empty params () is always RFC-010 compatible
                    true
                } else if state.at(&TokenKind::KwMut) {
                    // mut keyword signals a named parameter
                    true
                } else if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
                    let first_char = name.chars().next().unwrap_or('A');
                    let next = state.peek().map(|t| &t.kind);
                    // RFC-010: ':' after param name (e.g., a: Int)
                    // RFC-007 HM style: lowercase identifier followed by ',' or ')' (e.g., (a, b))
                    // Old syntax: Uppercase identifier (type) followed by ',' or ')' (e.g., (Int, Int))
                    matches!(next, Some(TokenKind::Colon))
                        || (first_char.is_lowercase()
                            && matches!(next, Some(TokenKind::Comma) | Some(TokenKind::RParen)))
                } else {
                    false
                };

                // Second check: skip to closing paren and check for ->
                let is_rfc010 = if looks_like_named_params {
                    let mut paren_depth = 1;
                    while paren_depth > 0 && !state.at_end() {
                        if state.at(&TokenKind::LParen) {
                            paren_depth += 1;
                        } else if state.at(&TokenKind::RParen) {
                            paren_depth -= 1;
                        }
                        state.bump();
                    }
                    state.at(&TokenKind::Arrow)
                } else {
                    false
                };

                state.restore_position(saved);

                if is_rfc010 {
                    // RFC-010 new syntax: () or (a: Int, b: Int) -> Ret
                    let (fn_params_parsed, return_type) = parse_fn_type_with_names(state)?;

                    // Build function type for type_annotation
                    let param_types: Vec<Type> = fn_params_parsed
                        .iter()
                        .filter_map(|p| p.ty.clone())
                        .collect();

                    let type_annotation = Type::Fn {
                        params: param_types,
                        return_type: return_type.clone(),
                    };

                    (Some(type_annotation), generic, Some(fn_params_parsed))
                } else {
                    // Check if this looks like old function syntax: (Type, Type) -> Ret
                    // If so, reject it with a helpful error message
                    let saved_check = state.save_position();
                    state.bump(); // consume '('
                    let mut paren_depth = 1;
                    while paren_depth > 0 && !state.at_end() {
                        if state.at(&TokenKind::LParen) {
                            paren_depth += 1;
                        } else if state.at(&TokenKind::RParen) {
                            paren_depth -= 1;
                        }
                        state.bump();
                    }
                    let is_old_fn_syntax = state.at(&TokenKind::Arrow);
                    state.restore_position(saved_check);

                    if is_old_fn_syntax {
                        // Old function syntax detected - reject it
                        state.error(ParseError::Message("Old function syntax '(Type, Type) -> Ret' is no longer supported. \
                             Use RFC-010 syntax with named parameters: '(param: Type, ...) -> Ret'. \
                             Example: 'add: (a: Int, b: Int) -> Int = a + b'".to_string()));
                        return None;
                    }

                    // Not a function type, parse as normal type annotation
                    let type_ann = parse_type_annotation(state)?;
                    (Some(type_ann), generic, None)
                }
            } else {
                // Not a function type, parse as normal type annotation
                let type_ann = parse_type_annotation(state)?;
                (Some(type_ann), generic, None)
            }
        } else {
            // No generic params, check for RFC-010 new syntax
            if state.at(&TokenKind::LParen) {
                // Look ahead to check if this is RFC-010 function syntax
                // Key: must have -> after the closing paren to be a function type
                // RFC-010/RFC-007 syntax:
                //   - (name: Type, ...) - named params with types
                //   - (name, name, ...) - named params without types (HM inference)
                // Old syntax:
                //   - (Type, Type, ...) - types only (NO param names)
                //
                // To distinguish: params are lowercase, types are Uppercase
                let saved = state.save_position();
                state.bump(); // consume '('

                // Check if this is RFC-010/RFC-007 compatible:
                // - Empty params () is always compatible
                // - Identifier followed by ':' is a named param with type
                // - Identifier followed by ',' or ')' could be either:
                //   - param name (lowercase) -> RFC-007 style
                //   - type name (Uppercase) -> old syntax
                let looks_like_named_params = if state.at(&TokenKind::RParen) {
                    // Empty params () is always RFC-010 compatible
                    true
                } else if state.at(&TokenKind::KwMut) {
                    // mut keyword signals a named parameter
                    true
                } else if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
                    let first_char = name.chars().next().unwrap_or('A');
                    let next = state.peek().map(|t| &t.kind);
                    // RFC-010: ':' after param name (e.g., a: Int)
                    // RFC-007 HM style: lowercase identifier followed by ',' or ')' (e.g., (a, b))
                    // Old syntax: Uppercase identifier (type) followed by ',' or ')' (e.g., (Int, Int))
                    matches!(next, Some(TokenKind::Colon))
                        || (first_char.is_lowercase()
                            && matches!(next, Some(TokenKind::Comma) | Some(TokenKind::RParen)))
                } else {
                    false
                };

                // Second check: skip to closing paren and check for ->
                let is_rfc010 = if looks_like_named_params {
                    let mut paren_depth = 1;
                    while paren_depth > 0 && !state.at_end() {
                        if state.at(&TokenKind::LParen) {
                            paren_depth += 1;
                        } else if state.at(&TokenKind::RParen) {
                            paren_depth -= 1;
                        }
                        state.bump();
                    }
                    state.at(&TokenKind::Arrow)
                } else {
                    false
                };

                state.restore_position(saved);

                if is_rfc010 {
                    // RFC-010 new syntax: () or (a: Int, b: Int) -> Ret
                    let (fn_params_parsed, return_type) = parse_fn_type_with_names(state)?;

                    // Build function type for type_annotation
                    let param_types: Vec<Type> = fn_params_parsed
                        .iter()
                        .filter_map(|p| p.ty.clone())
                        .collect();

                    let type_annotation = Type::Fn {
                        params: param_types,
                        return_type: return_type.clone(),
                    };

                    (Some(type_annotation), Vec::new(), Some(fn_params_parsed))
                } else {
                    // Check if this looks like old function syntax: (Type, Type) -> Ret
                    // If so, reject it with a helpful error message
                    let saved_check = state.save_position();
                    state.bump(); // consume '('
                    let mut paren_depth = 1;
                    while paren_depth > 0 && !state.at_end() {
                        if state.at(&TokenKind::LParen) {
                            paren_depth += 1;
                        } else if state.at(&TokenKind::RParen) {
                            paren_depth -= 1;
                        }
                        state.bump();
                    }
                    let is_old_fn_syntax = state.at(&TokenKind::Arrow);
                    state.restore_position(saved_check);

                    if is_old_fn_syntax {
                        // Old function syntax detected - reject it
                        state.error(ParseError::Message("Old function syntax '(Type, Type) -> Ret' is no longer supported. \
                             Use RFC-010 syntax with named parameters: '(param: Type, ...) -> Ret'. \
                             Example: 'add: (a: Int, b: Int) -> Int = a + b'".to_string()));
                        return None;
                    }

                    // Not a function type, parse as normal type annotation
                    let type_ann = parse_type_annotation(state)?;
                    (Some(type_ann), Vec::new(), None)
                }
            } else {
                // Not a function type, parse as normal type annotation
                let type_ann = parse_type_annotation(state)?;
                (Some(type_ann), Vec::new(), None)
            }
        }
    } else {
        (None, Vec::new(), None)
    };

    // Check for invalid syntax after type annotation.
    // Valid tokens after type annotation: `=`, `;`, newline/EOF
    // Invalid tokens include:
    // 1. `(` without `=` - e.g., `name: Type(params) => body` (missing =)
    // 2. `=>` without `=` - e.g., `name: Type(params) => body` (type parser consumed params)
    // 3. `,` - e.g., `name: Int, Int -> Int` (invalid type syntax with bare comma)
    // 4. Identifier - e.g., `name: (Int)Int -> Int` (invalid type syntax)
    if type_annotation.is_some() {
        let is_invalid = state.at(&TokenKind::LParen)
            || state.at(&TokenKind::FatArrow)
            || state.at(&TokenKind::Comma)
            || matches!(
                state.current().map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
            );
        if is_invalid && !state.at(&TokenKind::Eq) {
            let span = state.current().map(|t| t.span).unwrap_or_else(Span::dummy);
            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span,
            });
            return None;
        }
    }

    // Optional initializer
    if state.skip(&TokenKind::Eq) {
        // RFC-010 新语法: name: (a: Int, b: Int) -> Ret = body
        // body 可以是表达式或 lambda

        // Case 1: 有 fn_params (RFC-010 新语法)
        if let Some(ref extracted_params) = fn_params {
            // body 可以是表达式或 lambda
            let saved = state.save_position();
            let init = state.parse_expression(BP_LOWEST);

            match init {
                Some(Expr::Lambda {
                    params: lambda_params,
                    body,
                    ..
                }) => {
                    // Lambda 形式: name: (a: Int, b: Int) -> Int = (a, b) => a + b
                    // RFC-007: 签名参数名和 lambda 参数名必须匹配

                    // 检查参数数量
                    if lambda_params.len() != extracted_params.len() {
                        state.error(ParseError::Message(format!(
                            "Parameter count mismatch: signature has {} parameters, lambda has {}",
                            extracted_params.len(),
                            lambda_params.len()
                        )));
                        return None;
                    }

                    // 检查参数名匹配
                    for (i, (sig_param, lambda_param)) in extracted_params
                        .iter()
                        .zip(lambda_params.iter())
                        .enumerate()
                    {
                        if sig_param.name != lambda_param.name {
                            state.error(ParseError::Message(format!(
                                "Parameter name mismatch at position {}: signature has '{}', lambda has '{}'. \
                                 RFC-007 requires matching parameter names, or omit the lambda head entirely.",
                                i + 1,
                                sig_param.name,
                                lambda_param.name
                            )));
                            return None;
                        }
                    }

                    // 合并类型信息
                    let mut merged = Vec::new();
                    for (i, extracted) in extracted_params.iter().enumerate() {
                        if let Some(lambda_p) = lambda_params.get(i) {
                            merged.push(Param {
                                name: lambda_p.name.clone(),
                                ty: extracted.ty.clone(),
                                is_mut: lambda_p.is_mut,
                                span: lambda_p.span,
                            });
                        } else {
                            merged.push(extracted.clone());
                        }
                    }
                    state.skip(&TokenKind::Semicolon);
                    return Some(Stmt {
                        kind: StmtKind::Fn {
                            name,
                            generic_params,
                            type_annotation: type_annotation.clone(),
                            params: merged,
                            body: (body.stmts.clone(), body.expr.clone()),
                            is_pub: final_is_pub,
                        },
                        span,
                    });
                }
                Some(expr) => {
                    // 直接表达式: name: (a: Int, b: Int) -> Int = a + b
                    state.skip(&TokenKind::Semicolon);

                    // RFC-010: Check if expr is a Block expression
                    // If so, use the block's statements and expression
                    let body = if let Expr::Block(block) = &expr {
                        (block.stmts.clone(), block.expr.clone())
                    } else {
                        (Vec::new(), Some(Box::new(expr)))
                    };

                    return Some(Stmt {
                        kind: StmtKind::Fn {
                            name,
                            generic_params,
                            type_annotation: type_annotation.clone(),
                            params: extracted_params.clone(),
                            body,
                            is_pub: final_is_pub,
                        },
                        span,
                    });
                }
                None => {
                    state.restore_position(saved);
                }
            }
        }

        // Case 2: Variable with initializer (no fn_params - not a function definition)
        // Note: Old function syntax (Type, Type) -> Ret is now rejected at type annotation parsing stage

        // RFC-010: Check if type annotation is MetaType (`Type` or `Type[T]`)
        // If so, this is a type definition: `Name: Type = { ... }` or `Name: Type[T] = { ... }`
        if let Some(Type::MetaType { args: meta_args }) = &type_annotation {
            // RFC-010 Easter Egg: Type: Type = Type
            // 检测 `Type: Type = Type` 彩蛋（用户尝试定义 Type 自身）
            if name == "Type" {
                // 尝试解析表达式
                let saved_easter = state.save_position();
                if let Some(TokenKind::Identifier(val)) = state.current().map(|t| &t.kind) {
                    if val == "Type" {
                        state.bump();
                        state.skip(&TokenKind::Semicolon);
                        // 返回一个特殊的 TypeDef 表示彩蛋
                        // 编译器后续阶段会检测并输出禅意消息
                        // 保留 meta_args 以便类型检查器区分 E1090 和 E1091
                        return Some(Stmt {
                            kind: StmtKind::TypeDef {
                                name: "Type".to_string(),
                                definition: Type::MetaType { args: Vec::new() },
                                generic_params: meta_args
                                    .iter()
                                    .filter_map(extract_type_name)
                                    .collect(),
                            },
                            span,
                        });
                    }
                }
                state.restore_position(saved_easter);
            }

            // Extract generic parameter names from meta_args (Vec<Type>) for StmtKind::TypeDef
            let generic_params_for_type: Vec<String> =
                meta_args.iter().filter_map(extract_type_name).collect();

            let definition = parse_type_definition(state)?;
            state.skip(&TokenKind::Semicolon);
            return Some(Stmt {
                kind: StmtKind::TypeDef {
                    name,
                    definition,
                    generic_params: generic_params_for_type,
                },
                span,
            });
        }

        let initializer = match state.parse_expression(BP_LOWEST) {
            Some(expr) => Some(Box::new(expr)),
            None => {
                // Failed to parse initializer expression
                state.error(ParseError::Message(format!(
                    "Expected expression after '=' for variable '{}'",
                    name
                )));
                return None;
            }
        };

        state.skip(&TokenKind::Semicolon);

        // RFC-010: Check if initializer is a Block expression
        // If so, convert to function definition: name = { ... } => name: () -> Void = { ... }
        if let Some(ref init_expr) = initializer {
            if let Expr::Block(block) = init_expr.as_ref() {
                return Some(Stmt {
                    kind: StmtKind::Fn {
                        name,
                        generic_params: Vec::new(),
                        type_annotation: None, // Will be inferred
                        params: Vec::new(),
                        body: (block.stmts.clone(), block.expr.clone()),
                        is_pub: final_is_pub,
                    },
                    span,
                });
            }
        }

        return Some(Stmt {
            kind: StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut,
            },
            span,
        });
    }

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Var {
            name,
            type_annotation,
            initializer: None,
            is_mut,
        },
        span,
    })
}

/// Parse generic parameters with constraints: `[T: Clone]` or `[N: Int]`
/// Supports:
/// - Type parameters: `[T]` or `[T: Clone]`
/// - Const parameters: `[N: Int]` - const generic with type annotation
/// - Platform parameter: `[P: X86_64]` - RFC-011 platform specialization
pub fn parse_generic_params_with_constraints(
    state: &mut ParserState<'_>
) -> Option<Vec<GenericParam>> {
    if !state.at(&TokenKind::LBracket) {
        return Some(Vec::new());
    }
    state.bump(); // consume '['

    let mut params = Vec::new();

    while !state.at(&TokenKind::RBracket) && !state.at_end() {
        // Parse parameter name
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        // RFC-011: Check for reserved platform parameter P
        // P is reserved for platform specialization: [P: X86_64]
        let is_platform_param = name == "P";

        // Parse constraint: `T: Clone` or type annotation: `N: Int`
        let mut constraints = Vec::new();

        // Check for colon followed by type (could be either constraint or const type)
        if state.skip(&TokenKind::Colon) {
            if let Some(constraint_or_type) = parse_type_annotation(state) {
                // RFC-011: Platform parameter handling
                // [P: X86_64] - P is reserved, constraint is the platform type
                if is_platform_param {
                    // P is a special platform parameter, constraint is the platform type
                    params.push(GenericParam {
                        name,
                        kind: GenericParamKind::Platform, // Platform-specific parameter
                        constraints: vec![constraint_or_type],
                    });
                }
                // Determine if this is a Const parameter or a constraint
                // Const parameter: [N: Int] where Int is the const's type
                // Type constraint: [T: Clone] where Clone is a trait/type bound
                // We distinguish by whether the type is a simple type name (const) or a trait-like type
                else if is_const_param_type(&constraint_or_type) {
                    // This is a Const parameter: [N: Int]
                    params.push(GenericParam {
                        name,
                        kind: GenericParamKind::Const {
                            const_type: Box::new(constraint_or_type),
                        },
                        constraints,
                    });
                } else {
                    // This is a type parameter with constraint: [T: Clone]
                    constraints.push(constraint_or_type);
                    params.push(GenericParam {
                        name,
                        kind: GenericParamKind::Type,
                        constraints,
                    });
                }
            } else {
                // Fallback: type parameter without specific constraint
                if is_platform_param {
                    // Platform parameter without constraint matches any platform
                    params.push(GenericParam {
                        name,
                        kind: GenericParamKind::Platform,
                        constraints,
                    });
                } else {
                    params.push(GenericParam {
                        name,
                        kind: GenericParamKind::Type,
                        constraints,
                    });
                }
            }
        } else {
            // No colon: type parameter without constraint: [T]
            // Platform parameter without constraint matches any platform
            params.push(GenericParam {
                name,
                kind: if is_platform_param {
                    GenericParamKind::Platform
                } else {
                    GenericParamKind::Type
                },
                constraints,
            });
        }

        state.skip(&TokenKind::Comma);
    }

    if !state.expect(&TokenKind::RBracket) {
        return None;
    }

    Some(params)
}

/// Check if a type annotation represents a Const parameter type
/// Const parameter types are simple named types like Int, Bool, Float
/// Type constraints are trait-like types
fn is_const_param_type(ty: &Type) -> bool {
    match ty {
        Type::Name(name) => const_param_type_set().contains(name.as_str()),
        Type::Literal { .. } => true,
        _ => false,
    }
}

/// Parse type definition (handles union types with |)
fn parse_type_definition(state: &mut ParserState<'_>) -> Option<Type> {
    let first_type = parse_type_annotation(state)?;

    if state.at(&TokenKind::Pipe) {
        let mut types = vec![first_type];
        while state.skip(&TokenKind::Pipe) {
            types.push(parse_type_annotation(state)?);
        }

        // Check if all types are variant-like
        let all_variants = types.iter().all(|t| {
            matches!(
                t,
                Type::Name(_) | Type::Generic { .. } | Type::NamedStruct { .. }
            )
        });

        if all_variants {
            // Convert to Variant
            let mut variants = Vec::new();
            for ty in types.iter() {
                match ty {
                    Type::Generic { name, args } => {
                        let params = args.iter().map(|a| (None, a.clone())).collect();
                        variants.push(VariantDef {
                            name: name.clone(),
                            params,
                            span: state.span(),
                        });
                    }
                    Type::NamedStruct { name, fields } => {
                        let params = fields
                            .iter()
                            .map(|f| (Some(f.name.clone()), f.ty.clone()))
                            .collect();
                        variants.push(VariantDef {
                            name: name.clone(),
                            params,
                            span: state.span(),
                        });
                    }
                    Type::Name(name) => {
                        variants.push(VariantDef {
                            name: name.clone(),
                            params: Vec::new(),
                            span: state.span(),
                        });
                    }
                    _ => unreachable!(),
                }
            }
            return Some(Type::Variant(variants));
        } else {
            return Some(Type::Sum(types));
        }
    }

    Some(first_type)
}

/// Parse use import statement: `use path;` or `use path.{item1, item2};` or `use path as alias;`
pub fn parse_use_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'use'

    let path = parse_use_path(state)?;

    // Parse import items: use path.{item1, item2};
    let items = if state.skip(&TokenKind::LBrace) {
        let mut items = Vec::new();
        while !state.at(&TokenKind::RBrace) && !state.at_end() {
            match state.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => {
                    items.push(n.clone());
                    state.bump();
                    state.skip(&TokenKind::Comma);
                }
                Some(TokenKind::KwPub) => {
                    // Skip 'pub' in import items
                    state.bump();
                }
                _ => break,
            }
        }
        state.expect(&TokenKind::RBrace);
        Some(items)
    } else {
        None
    };

    // Parse alias: use path as alias;
    let alias = if state.skip(&TokenKind::KwAs) {
        match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => {
                let a = n.clone();
                state.bump();
                Some(a)
            }
            _ => None,
        }
    } else {
        None
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Use { path, items, alias },
        span,
    })
}

/// Parse use path (dot-separated identifiers)
fn parse_use_path(state: &mut ParserState<'_>) -> Option<String> {
    let mut parts = Vec::new();

    while let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
        parts.push(n.clone());
        state.bump();
        if !state.skip(&TokenKind::Dot) {
            break;
        }
    }

    if parts.is_empty() {
        state.error(ParseError::UnexpectedToken {
            found: state
                .current()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Eof),
            span: state.span(),
        });
        None
    } else {
        Some(parts.join("."))
    }
}

/// Parse statement starting with identifier: function definition or expression or variable declaration
/// Syntax:
/// - `pub name = (params) => body` - pub 函数定义，自动绑定到类型
/// - `name = (params) => body` - 函数定义
/// - `name = expr` - 变量声明（如果没有类型注解）
/// - `name: type = expr` - 变量声明（带类型注解）
/// - `name expr` - 表达式语句
///
/// 旧语法 `name(types) -> type = ...` 被明确拒绝
pub fn parse_identifier_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // 首先检测并拒绝旧语法: identifier( 后跟类型参数和 ->
    // 旧语法示例: add(Int, Int) -> Int = (a, b) => a + b
    // 必须在获取 next 之前检查，避免借用冲突
    if let Some(TokenKind::LParen) = state.peek().map(|t| &t.kind) {
        // 检查这是否是旧的函数定义语法
        if is_old_function_syntax(state) {
            state.error(ParseError::Message(
                "旧语法已弃用。请使用新语法: `name: (Types) -> ReturnType = (params) => body`"
                    .to_string(),
            ));
            // 跳过整个旧语法声明
            skip_old_function_syntax(state);
            return None;
        }
    }

    // 检测方法绑定语法: Type.method: ...
    if is_method_bind_syntax(state) {
        return parse_method_bind_stmt(state, span);
    }

    // 检测 pub 关键字
    // 先检查当前 token 是否是 pub
    let is_pub = if matches!(state.current().map(|t| &t.kind), Some(TokenKind::KwPub)) {
        state.bump(); // 消费 pub
        true
    } else {
        false
    };

    // 获取当前 token（应该是标识符）
    let next = state.peek();

    // Check if identifier is followed by =
    if matches!(next.map(|t| &t.kind), Some(TokenKind::Eq)) {
        let saved_position = state.save_position();
        let err_count = state.error_count();

        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                state.error(ParseError::UnexpectedToken {
                    found: state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span: state.span(),
                });
                return None;
            }
        };
        state.bump(); // consume identifier

        // Check if = is followed by (
        if state.at(&TokenKind::Eq) {
            state.bump(); // consume =

            // If = is followed by (, try to parse as function definition
            if state.at(&TokenKind::LParen) {
                if let Some(stmt) = parse_fn_stmt_with_name(state, name.clone(), span, is_pub) {
                    state.skip(&TokenKind::Semicolon);
                    return Some(stmt);
                }
                state.restore_position(saved_position);
                state.truncate_errors(err_count);
            } else if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
                let saved_position2 = state.save_position();
                let err_count2 = state.error_count();

                if let Some(stmt) =
                    parse_fn_stmt_with_name_simple(state, name.clone(), span, is_pub)
                {
                    state.skip(&TokenKind::Semicolon);
                    return Some(stmt);
                }

                state.restore_position(saved_position2);
                state.truncate_errors(err_count2);
            }

            // Not a function definition, parse as variable declaration (name = expr)
            // This is a variable declaration with type inference
            let initializer = state.parse_expression(BP_LOWEST)?;
            state.skip(&TokenKind::Semicolon);

            // RFC-010: Check if initializer is a Block expression
            // If so, convert to function definition: name = { ... } => name: () -> Void = { ... }
            if let Expr::Block(block) = &initializer {
                return Some(Stmt {
                    kind: StmtKind::Fn {
                        name,
                        generic_params: Vec::new(),
                        type_annotation: None, // Will be inferred
                        params: Vec::new(),
                        body: (block.stmts.clone(), block.expr.clone()),
                        is_pub,
                    },
                    span,
                });
            }

            return Some(Stmt {
                kind: StmtKind::Var {
                    name,
                    type_annotation: None,
                    initializer: Some(Box::new(initializer)),
                    is_mut: false,
                },
                span,
            });
        }

        state.restore_position(saved_position);
        state.truncate_errors(err_count);

        // Fallback to expression statement
        return parse_expr_stmt(state, span);
    }

    // Check for variable declaration: identifier followed by :
    if matches!(next.map(|t| &t.kind), Some(TokenKind::Colon)) {
        // 传递已检测到的 is_pub 给 parse_var_stmt
        return parse_var_stmt_with_pub(state, span, Some(is_pub));
    }

    // Otherwise, parse as expression
    parse_expr_stmt(state, span)
}

/// Parse function definition with already parsed name
/// Handles: `[pub] name = (params) => body`
pub fn parse_fn_stmt_with_name(
    state: &mut ParserState<'_>,
    name: String,
    span: Span,
    is_pub: bool,
) -> Option<Stmt> {
    if !state.expect(&TokenKind::LParen) {
        return None;
    }
    let params = parse_fn_params(state)?;
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    if !state.expect(&TokenKind::FatArrow) {
        return None;
    }

    let (stmts, expr) = parse_fn_body(state)?;

    Some(Stmt {
        kind: StmtKind::Fn {
            name,
            generic_params: Vec::new(),
            type_annotation: None,
            params,
            body: (stmts, expr),
            is_pub,
        },
        span,
    })
}

/// Parse function definition with already parsed name (simple form)
/// Handles: `[pub] name = param => body` (single param without parentheses)
pub fn parse_fn_stmt_with_name_simple(
    state: &mut ParserState<'_>,
    name: String,
    span: Span,
    is_pub: bool,
) -> Option<Stmt> {
    let param_span = state.span();
    let param_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    if !state.expect(&TokenKind::FatArrow) {
        return None;
    }

    let (stmts, expr) = parse_fn_body(state)?;

    Some(Stmt {
        kind: StmtKind::Fn {
            name,
            generic_params: Vec::new(),
            type_annotation: None,
            params: vec![Param {
                name: param_name,
                ty: None,
                is_mut: false,
                span: param_span,
            }],
            body: (stmts, expr),
            is_pub,
        },
        span,
    })
}

/// Parse function body (expression or block)
fn parse_fn_body(state: &mut ParserState<'_>) -> Option<(Vec<Stmt>, Option<Box<Expr>>)> {
    if state.at(&TokenKind::LBrace) {
        if !state.expect(&TokenKind::LBrace) {
            return None;
        }
        let body = parse_block_body_impl(state)?;
        if !state.expect(&TokenKind::RBrace) {
            return None;
        }
        Some(body)
    } else {
        let expr = state.parse_expression(BP_LOWEST)?;
        Some((Vec::new(), Some(Box::new(expr))))
    }
}

/// Parse block body implementation (shared helper)
fn parse_block_body_impl(state: &mut ParserState<'_>) -> Option<(Vec<Stmt>, Option<Box<Expr>>)> {
    let mut stmts = Vec::new();

    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(stmt) = state.parse_statement() {
            stmts.push(stmt);
        } else {
            state.synchronize();
        }
    }

    let expr = if !state.at(&TokenKind::RBrace) {
        state.parse_expression(BP_LOWEST)
    } else {
        None
    };

    Some((stmts, expr.map(Box::new)))
}

/// Parse function parameters: `(param1: Type, param2: Type)`
pub fn parse_fn_params(state: &mut ParserState<'_>) -> Option<Vec<Param>> {
    let mut params = Vec::new();

    while !state.at(&TokenKind::RParen) && !state.at_end() {
        if !params.is_empty() && !state.expect(&TokenKind::Comma) {
            return None;
        }

        if state.at(&TokenKind::RParen) {
            break;
        }

        let param_span = state.span();

        // Handle '...' for variadic parameters
        let _is_variadic = state.skip(&TokenKind::DotDotDot);

        // Check for mut keyword
        let is_mut = state.skip(&TokenKind::KwMut);

        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        let ty = if state.skip(&TokenKind::Colon) {
            parse_type_annotation(state)
        } else {
            None
        };

        params.push(Param {
            name,
            ty,
            is_mut,
            span: param_span,
        });
    }

    Some(params)
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
            state.bump();

            // RFC-010: Check if this is the meta-type keyword `Type` or `Type[T]` or `Type<T>`
            // Supports infinite universe levels: Type[Type[T]] → Type2, etc.
            if name == "Type" {
                // Check for generic parameters: Type[T] or Type[K, V] or Type[Type[T]]
                if state.at(&TokenKind::LBracket) {
                    state.bump(); // consume '['
                    let mut args = Vec::new();
                    while !state.at(&TokenKind::RBracket) && !state.at_end() {
                        // Recursively parse each parameter as a Type
                        if let Some(param_type) = parse_type_annotation(state) {
                            args.push(param_type);
                            state.skip(&TokenKind::Comma);
                        } else {
                            break;
                        }
                    }
                    state.skip(&TokenKind::RBracket);
                    return Some(Type::MetaType { args });
                }
                // Also support angle bracket syntax: Type<T> or Type<K, V>
                if state.at(&TokenKind::Lt) {
                    state.bump(); // consume '<'
                    let mut args = Vec::new();
                    while !state.at(&TokenKind::Gt) && !state.at_end() {
                        // Recursively parse each parameter as a Type
                        if let Some(param_type) = parse_type_annotation(state) {
                            args.push(param_type);
                            state.skip(&TokenKind::Comma);
                        } else {
                            break;
                        }
                    }
                    state.skip(&TokenKind::Gt);
                    return Some(Type::MetaType { args });
                }
                return Some(Type::MetaType { args: Vec::new() });
            }

            // Check for generic parameters: Type<T> or Type[T]
            if state.at(&TokenKind::Lt) {
                return parse_generic_type(name, state);
            }
            if state.at(&TokenKind::LBracket) {
                return parse_generic_type_bracket(name, state);
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
                    return parse_named_struct_type(name, state);
                } else {
                    // Parse as generic constructor: Name(Type1, Type2)
                    return parse_constructor_type(name, state);
                }
            }

            Some(Type::Name(name))
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
            // This could be a function type with generic params: [T, U](params) -> ReturnType
            // Or it could be a tuple/list type starting with bracket
            let saved = state.save_position();
            state.bump(); // consume '['

            // Check if this looks like a function type: [...](...) -> ...
            // We need to see if there's a ']' followed by '('
            let looks_like_fn_type = state.at(&TokenKind::RBracket) || {
                // Try to parse one element, then check if ']' and '(' follow
                if parse_type_annotation(state).is_some() {
                    state.at(&TokenKind::RBracket) && {
                        // Peek after ']'
                        let saved2 = state.save_position();
                        state.bump(); // consume ']'
                        let result = state.at(&TokenKind::LParen);
                        state.restore_position(saved2);
                        result
                    }
                } else {
                    false
                }
            };

            if looks_like_fn_type {
                // It's a function type with generic params, reparse from scratch
                state.restore_position(saved);
                state.bump(); // consume '['

                // Parse generic param types inside [...]
                let mut param_types = Vec::new();
                if !state.at(&TokenKind::RBracket) {
                    while let Some(ty) = parse_type_annotation(state) {
                        param_types.push(ty);
                        if !state.skip(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                if !state.expect(&TokenKind::RBracket) {
                    return None;
                }

                // Now expect (params) -> ReturnType
                if !state.expect(&TokenKind::LParen) {
                    return None;
                }

                let mut fn_param_types = Vec::new();
                if !state.at(&TokenKind::RParen) {
                    while let Some(ty) = parse_type_annotation(state) {
                        fn_param_types.push(ty);
                        if !state.skip(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                if !state.expect(&TokenKind::RParen) {
                    return None;
                }

                if !state.expect(&TokenKind::Arrow) {
                    return None;
                }

                let return_type = Box::new(parse_type_annotation(state)?);

                // Return function type with parsed params
                Some(Type::Fn {
                    params: fn_param_types,
                    return_type,
                })
            } else {
                // Not a function type, try to parse as tuple type
                state.restore_position(saved);
                // But since we don't handle [T, U, V] tuple syntax, return None
                None
            }
        }
        // Note: fn type uses (Params) -> ReturnType syntax, not `fn` keyword
        _ => None,
    }
}

/// Parse generic type like `Vec[T]` or `HashMap[K, V]`
fn parse_generic_type(
    name: String,
    state: &mut ParserState<'_>,
) -> Option<Type> {
    state.skip(&TokenKind::Lt); // consume '<'

    let mut args = Vec::new();

    if !state.at(&TokenKind::Gt) {
        while let Some(arg) = parse_type_annotation(state) {
            args.push(arg);

            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }

    state.skip(&TokenKind::Gt); // consume '>'

    Some(Type::Generic { name, args })
}

/// Parse generic type with bracket syntax: `Type[T, U]`
fn parse_generic_type_bracket(
    name: String,
    state: &mut ParserState<'_>,
) -> Option<Type> {
    state.skip(&TokenKind::LBracket); // consume '['

    let mut args = Vec::new();

    if !state.at(&TokenKind::RBracket) {
        while let Some(arg) = parse_type_annotation(state) {
            args.push(arg);

            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }

    state.skip(&TokenKind::RBracket); // consume ']'

    Some(Type::Generic { name, args })
}

/// Parse named struct type: `Name(x: Type, y: Type)` or `Name(mut x: Type, y: Type)`
fn parse_named_struct_type(
    name: String,
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

    Some(Type::NamedStruct { name, fields })
}

/// Parse constructor type: `Name(Type1, Type2)`
fn parse_constructor_type(
    name: String,
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

    Some(Type::Generic { name, args })
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
/// e.g., for `factorial: [n: Int](n: n)`, when parsing the second `n`:
/// - name = "n" (parameter name)
/// - parsed_type = Type::Name("n")
///   Result: Type::Literal { name: "n", base_type: Type::Name("Int") }
fn wrap_literal_type_if_needed(
    param_name: String,
    parsed_type: Type,
) -> Type {
    // Check if the type is a simple identifier with the same name as the parameter
    // This indicates a const generic literal type reference
    match &parsed_type {
        Type::Name(type_name) if type_name == &param_name => {
            // This is a literal type reference: the parameter value is used as a type
            Type::Literal {
                name: param_name.clone(),
                base_type: Box::new(Type::Name(param_name)),
            }
        }
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
/// RFC-010 支持两种语法：
/// - 普通字段: `type Point = { x: Float, mut y: Float }`
/// - 接口约束: `type Point = { x: Float, Drawable, Serializable }`
/// - 可变字段: `type Point = { mut x: Float, y: Float }`
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
                // 无冒号但有等号: 外部函数绑定 name = function[positions]
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

                // 解析位置绑定 [positions]
                let positions = parse_binding_positions(state).ok()?;
                bindings.push(TypeBodyBinding {
                    name,
                    kind: BindingKind::External {
                        function: func_name,
                        positions,
                    },
                });
            } else if is_mut {
                // mut 后面没有冒号是语法错误
                state.error(ParseError::Message(format!(
                    "Expected ':' after 'mut' in field '{}'",
                    name
                )));
                return None;
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

    Some(Type::Struct { fields, bindings })
}

/// 解析可选的位置绑定: `[0]` 或 `[0, 1]`
/// 只在下一个 token 是 `[` 且内容是整数时才消费
fn parse_optional_binding_positions(state: &mut ParserState<'_>) -> Option<Vec<usize>> {
    if !state.at(&TokenKind::LBracket) {
        return None;
    }

    // 前瞻检查: 确认 `[` 后面是整数字面量
    let saved = state.save_position();
    state.bump(); // consume '['

    match state.current().map(|t| &t.kind) {
        Some(TokenKind::IntLiteral(_)) => {
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

/// 解析位置绑定: `[0]` 或 `[0, 1]`（必须存在）
fn parse_binding_positions(state: &mut ParserState<'_>) -> Result<Vec<usize>, ()> {
    if !state.at(&TokenKind::LBracket) {
        state.error(ParseError::Message(
            "Expected '[' for binding position".to_string(),
        ));
        return Err(());
    }
    state.bump(); // consume '['

    let mut positions = Vec::new();
    while !state.at(&TokenKind::RBracket) && !state.at_end() {
        match state.current().map(|t| &t.kind) {
            Some(TokenKind::IntLiteral(n)) => {
                positions.push(*n as usize);
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

/// Parse a constructor: `Name` or `Name(params)`
pub fn parse_constructor(state: &mut ParserState<'_>) -> Option<VariantDef> {
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => {
            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
            return None;
        }
    };
    state.bump();

    let params = if state.at(&TokenKind::LParen) {
        parse_constructor_params(state)?
    } else {
        Vec::new()
    };

    Some(VariantDef {
        name,
        params,
        span: state.span(),
    })
}

/// Parse constructor parameters: (x: Type, y: Type) or generic args: (Type1, Type2)
fn parse_constructor_params(state: &mut ParserState<'_>) -> Option<Vec<(Option<String>, Type)>> {
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    let has_named_params = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(_)) => {
            matches!(state.peek().map(|t| &t.kind), Some(TokenKind::Colon))
        }
        _ => false,
    };

    let mut params = Vec::new();

    if has_named_params {
        while !state.at(&TokenKind::RParen) && !state.at_end() {
            let name = match state.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => break,
            };
            state.bump();

            if !state.expect(&TokenKind::Colon) {
                return None;
            }

            let ty = match parse_type_annotation(state) {
                Some(t) => t,
                None => break,
            };

            params.push((Some(name), ty));

            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    } else {
        while !state.at(&TokenKind::RParen) && !state.at_end() {
            let ty = match parse_type_annotation(state) {
                Some(t) => t,
                None => break,
            };

            params.push((None, ty));

            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }

    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    Some(params)
}

/// Parse expression statement
pub fn parse_expr_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    if let Some(expr) = state.parse_expression(BP_LOWEST) {
        state.skip(&TokenKind::Semicolon);
        Some(Stmt {
            kind: StmtKind::Expr(Box::new(expr)),
            span,
        })
    } else {
        state.bump();
        None
    }
}
