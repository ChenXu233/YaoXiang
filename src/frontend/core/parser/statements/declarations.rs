//! Declaration parsing - variables, types, function signatures (identifier stmt)
//!
//! Implements parsing for:
//! - Variable declarations: `[mut] name[: type] [= expr];`
//! - Type definitions: `Name: Type = Type;` (simple form)
//! - Identifier statement dispatch (routes to function/var/expression parsing)
//! - Mutability fields in type body
//! - Constructor parsing: `Name` or `Name(params)`
//! - Expression statement fallback
//!
//! Sub-modules:
//! - `types` - Type annotation parsing (annotations, generic params, struct/enum types)
//! - `functions` - Function definition parsing (fn stmts with params and body)
//! - `imports` - Use/import statement parsing

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::{ParserState, BP_LOWEST};
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::frontend::core::parser::parse_msg;
use crate::util::span::Span;

// Import from sibling modules
use super::functions::{parse_fn_stmt_with_name, parse_fn_stmt_with_name_simple};
use super::types::{parse_type_annotation, parse_fn_type_with_names, parse_binding_positions};

#[allow(dead_code)]
fn fn_returns_meta_type(type_annotation: Option<&Type>) -> bool {
    matches!(
        type_annotation,
        Some(Type::Fn {
            return_type,
            ..
        }) if matches!(return_type.as_ref(), Type::MetaType { .. })
    )
}

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

    // 检查是否是 Identifier (类型名) — 类型名必须以大写字母开头
    let has_type_name = matches!(
        state.current().map(|t| &t.kind),
        Some(TokenKind::Identifier(name)) if name.chars().next().is_some_and(|c| c.is_uppercase())
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

/// RFC-004: 检测是否是外部绑定语句语法: `Type.method = function[pos]`
/// 模式: Identifier . Identifier = (没有冒号)
fn is_external_binding_syntax(state: &mut ParserState<'_>) -> bool {
    let saved = state.save_position();

    // 检查是否是 Identifier (类型名) — 类型名必须以大写字母开头
    let has_type_name = matches!(
        state.current().map(|t| &t.kind),
        Some(TokenKind::Identifier(name)) if name.chars().next().is_some_and(|c| c.is_uppercase())
    );

    if has_type_name {
        state.bump(); // consume type name

        if state.at(&TokenKind::Dot) {
            state.bump(); // consume dot

            let has_method_name = matches!(
                state.current().map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
            );

            if has_method_name {
                state.bump(); // consume method name

                // 检查是否是等号 (不是冒号)
                let has_eq = state.at(&TokenKind::Eq);
                state.restore_position(saved);
                return has_eq;
            }
        }
    }

    state.restore_position(saved);
    false
}

/// RFC-004: 解析外部绑定语句: `Type.method = function[pos]` 或 `Type.method = function`
pub fn parse_external_binding_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // Parse type name
    let type_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump(); // consume type name

    state.bump(); // consume '.'

    // Parse method name
    let method_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump(); // consume method name

    state.bump(); // consume '='

    // Parse function name
    let func_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => {
            state.error(parse_msg(format!(
                "Expected function name after '=' in external binding '{}.{}'",
                type_name, method_name
            )));
            return None;
        }
    };
    state.bump(); // consume function name

    // 解析可选的位置绑定 [positions]
    let binding = if state.at(&TokenKind::LBracket) {
        let positions = parse_binding_positions(state).ok()?;
        BindingKind::External {
            function: func_name,
            positions,
        }
    } else {
        BindingKind::DefaultExternal {
            function: func_name,
        }
    };

    Some(Stmt {
        kind: StmtKind::ExternalBindingStmt {
            type_name,
            method_name,
            binding,
        },
        span,
    })
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
            state.error(
                ErrorCodeDefinition::unexpected_token(&format!(
                    "{:?}",
                    state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof)
                ))
                .at(state.span())
                .build(),
            );
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
            state.error(
                ErrorCodeDefinition::unexpected_token(&format!(
                    "{:?}",
                    state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof)
                ))
                .at(state.span())
                .build(),
            );
            return None;
        }
    };
    state.bump(); // consume method name

    // Expect colon
    if !state.expect(&TokenKind::Colon) {
        return None;
    }

    // Parse method type annotation - use parse_fn_type_with_names to preserve param names
    // This returns (Vec<Param>, Box<Type>) where Param has name and ty.
    let (method_fn_params, method_return_type) = match parse_fn_type_with_names(state) {
        Some(result) => result,
        None => {
            state.error(parse_msg(
                "Expected function type annotation after ':' in method binding, e.g. (self: Type, ...) -> Ret".to_string(),
            ));
            return None;
        }
    };

    // Build Type::Fn from the parsed params (types only, as Type::Fn doesn't store names)
    let method_param_types: Vec<Type> = method_fn_params
        .iter()
        .filter_map(|p| p.ty.clone())
        .collect();
    let method_type = Type::Fn {
        params: method_param_types,
        return_type: method_return_type,
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
            state.error(parse_msg(
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
    let (params, body_stmts) = match &initializer {
        Expr::Lambda { params, body, .. } => {
            // 旧语法：提取 Lambda 的参数和体
            (params.clone(), body.stmts.clone())
        }
        Expr::Block(block) => {
            // RFC-010 新语法：代码块体，参数已在签名中声明
            // 使用从签名中解析出的方法参数（method_fn_params），
            // 这样类型检查器可以正确地将参数添加到作用域
            (method_fn_params.clone(), block.stmts.clone())
        }
        expr => {
            // 直接表达式形式
            (
                method_fn_params.clone(),
                vec![Stmt {
                    kind: StmtKind::Expr(Box::new(expr.clone())),
                    span: state.span(),
                }],
            )
        }
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Binding {
            name: method_name,
            type_name: Some(type_name),
            method_type: Some(method_type),
            signature_params: Vec::new(),
            type_annotation: None,

            params,
            body: body_stmts,
            is_pub: false,
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
    let (name, name_span) = match state.current() {
        Some(t) => match &t.kind {
            TokenKind::Identifier(n) => (n.clone(), t.span),
            _ => {
                state.error(
                    ErrorCodeDefinition::unexpected_token(&format!(
                        "{:?}",
                        state
                            .current()
                            .map(|t| t.kind.clone())
                            .unwrap_or(TokenKind::Eof)
                    ))
                    .at(state.span())
                    .build(),
                );
                return None;
            }
        },
        None => {
            state.error(
                ErrorCodeDefinition::unexpected_token(&format!("{:?}", TokenKind::Eof))
                    .at(state.span())
                    .build(),
            );
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
            state.error(parse_msg(
                "旧函数语法已不再支持，请使用新语法: name: (param: Type, ...) -> Ret = body"
                    .to_string(),
            ));
            return None;
        }
    }

    // RFC-010 新语法: name: (a: Int, b: Int) -> Ret = body (参数名在签名中)
    // 特征: 冒号后面跟着函数类型 (param: Type, ...) -> Ret
    // 包括空参数 () 也属于 RFC-010 语法
    let (type_annotation, fn_params) = if state.at(&TokenKind::Colon) {
        state.bump(); // consume ':'

        // RFC-010: name: type = value. Generic params (T: Type) extracted from parsed fn type.
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

                (Some(type_annotation), Some(fn_params_parsed))
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
                    state.error(parse_msg("Old function syntax '(Type, Type) -> Ret' is no longer supported. \
                             Use RFC-010 syntax with named parameters: '(param: Type, ...) -> Ret'. \
                             Example: 'add: (a: Int, b: Int) -> Int = a + b'".to_string()));
                    return None;
                }

                // Not a function type, parse as normal type annotation
                let type_ann = parse_type_annotation(state)?;
                (Some(type_ann), None)
            }
        } else {
            let type_ann = parse_type_annotation(state)?;
            (Some(type_ann), None)
        }
    } else {
        (None, None)
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
            state.error(
                ErrorCodeDefinition::unexpected_token(&format!(
                    "{:?}",
                    state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof)
                ))
                .at(span)
                .build(),
            );
            return None;
        }
    }

    // Optional initializer
    if state.skip(&TokenKind::Eq) {
        // RFC-010 新语法: name: (a: Int, b: Int) -> Ret = body
        // body 可以是表达式或 lambda

        // Check if this is a generic type constructor definition:
        //   name: (T: Type, ...) -> Type = { ... }
        // When the return type is Type::MetaType (i.e. `-> Type`),
        // treat it as a type constructor, not a function.
        if let Some(Type::Fn { return_type, .. }) = &type_annotation {
            if matches!(return_type.as_ref(), Type::MetaType { .. }) {
                // This is a type constructor with generic params

                let definition = parse_type_definition(state)?;
                state.skip(&TokenKind::Semicolon);
                return Some(Stmt {
                    kind: StmtKind::Binding {
                        name,
                        type_name: None,
                        method_type: None,
                        signature_params: fn_params.clone().unwrap_or_default(),
                        type_annotation: Some(definition),

                        params: Vec::new(),
                        body: Vec::new(),
                        is_pub: final_is_pub,
                    },
                    span,
                });
            }
        }

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
                    //
                    // For generic functions like `map: (T: Type, R: Type) -> (...) = (list, f) => ...`
                    // or `clone: (T: Clone) -> ((value: T) -> T) = (value) => ...`,
                    // the value-level parameter names are in the return type's function type,
                    // not in the first parameter group. Filter out type params (uppercase names)
                    // before matching, since type parameter names follow the uppercase convention.
                    let value_params: Vec<&Param> = extracted_params
                        .iter()
                        .filter(|p| {
                            let first_char = p.name.chars().next().unwrap_or('a');
                            first_char.is_lowercase()
                        })
                        .collect();

                    // If there are value params in the signature, verify they match lambda.
                    // If ALL params are type params (generic function), skip matching
                    // since the real value params are embedded in the return type.
                    if !value_params.is_empty() {
                        if lambda_params.len() != value_params.len() {
                            state.error(parse_msg(format!(
                                "Parameter count mismatch: signature has {} parameters, lambda has {}",
                                value_params.len(),
                                lambda_params.len()
                            )));
                            return None;
                        }

                        for (i, (sig_param, lambda_param)) in
                            value_params.iter().zip(lambda_params.iter()).enumerate()
                        {
                            if sig_param.name != lambda_param.name {
                                state.error(parse_msg(format!(
                                    "Parameter name mismatch at position {}: signature has '{}', lambda has '{}'. \
                                     RFC-007 requires matching parameter names, or omit the lambda head entirely.",
                                    i + 1,
                                    sig_param.name,
                                    lambda_param.name
                                )));
                                return None;
                            }
                        }
                    }

                    // 值参数对齐：签名值参数（剔除 Type 级泛型参数）与 lambda 参数 zip。
                    // 仅剔除 Type 级泛型（MetaType/Type/大写约束名）；Const 泛型（如 N: Int）
                    // 仍是值级参数，保留 Int 标注。与 extract_generic_params 的 Type 分类同源。
                    let generic_params_list = extract_generic_params(extracted_params);
                    let generic_names: std::collections::HashSet<&str> = generic_params_list
                        .iter()
                        .filter(|gp| {
                            matches!(
                                gp.kind,
                                crate::frontend::core::parser::ast::GenericParamKind::Type
                            )
                        })
                        .map(|p| p.name.as_str())
                        .collect();
                    let value_sig: Vec<&Param> = extracted_params
                        .iter()
                        .filter(|p| !generic_names.contains(p.name.as_str()))
                        .collect();
                    // 全泛型签名时（值参数在内层返回类型，如 map: (T: Type) -> ((x: T) -> R)）
                    // value_sig 为空，params 直接取 lambda 参数（标注 None，HM 推断）；
                    // 否则 RFC-007 校验已保证 value_sig 与 lambda_params 等长。
                    let merged: Vec<Param> = if value_sig.is_empty() {
                        lambda_params.to_vec()
                    } else {
                        value_sig
                            .iter()
                            .zip(lambda_params.iter())
                            .map(|(sig, lam)| Param {
                                name: lam.name.clone(),
                                ty: sig.ty.clone(),
                                is_mut: lam.is_mut,
                                span: lam.span,
                            })
                            .collect()
                    };
                    state.skip(&TokenKind::Semicolon);
                    return Some(Stmt {
                        kind: StmtKind::Binding {
                            name,
                            type_name: None,
                            method_type: None,
                            signature_params: extracted_params.clone(),
                            type_annotation: type_annotation.clone(),
                            params: merged,
                            body: body.stmts.clone(),
                            is_pub: final_is_pub,
                        },
                        span,
                    });
                }
                Some(expr) => {
                    // 直接表达式: name: (a: Int, b: Int) -> Int = a + b
                    state.skip(&TokenKind::Semicolon);

                    // RFC-010: Check if expr is a Block expression
                    // If so, use the block's statements
                    let body = if let Expr::Block(block) = &expr {
                        block.stmts.clone()
                    } else {
                        vec![Stmt {
                            kind: StmtKind::Expr(Box::new(expr)),
                            span: state.span(),
                        }]
                    };

                    let generic_params_list = extract_generic_params(extracted_params);
                    let generic_names: std::collections::HashSet<&str> = generic_params_list
                        .iter()
                        .filter(|gp| {
                            matches!(
                                gp.kind,
                                crate::frontend::core::parser::ast::GenericParamKind::Type
                            )
                        })
                        .map(|p| p.name.as_str())
                        .collect();
                    let value_params: Vec<Param> = extracted_params
                        .iter()
                        .filter(|p| !generic_names.contains(p.name.as_str()))
                        .cloned()
                        .collect();
                    return Some(Stmt {
                        kind: StmtKind::Binding {
                            name,
                            type_name: None,
                            method_type: None,
                            signature_params: extracted_params.clone(),
                            params: value_params,
                            type_annotation: type_annotation.clone(),
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
        if let Some(Type::MetaType { args: _, .. }) = &type_annotation {
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
                            kind: StmtKind::Binding {
                                name: "Type".to_string(),
                                type_name: None,
                                method_type: None,
                                signature_params: Vec::new(),
                                type_annotation: Some(Type::MetaType {
                                    name_span: Span::dummy(),
                                    args: Vec::new(),
                                }),

                                params: Vec::new(),
                                body: Vec::new(),
                                is_pub: false,
                            },
                            span,
                        });
                    }
                }
                state.restore_position(saved_easter);
            }

            let definition = parse_type_definition(state)?;
            state.skip(&TokenKind::Semicolon);
            return Some(Stmt {
                kind: StmtKind::Binding {
                    name,
                    type_name: None,
                    method_type: None,
                    signature_params: Vec::new(),
                    type_annotation: Some(definition),

                    params: Vec::new(),
                    body: Vec::new(),
                    is_pub: false,
                },
                span,
            });
        }

        let initializer = match state.parse_expression(BP_LOWEST) {
            Some(expr) => Some(Box::new(expr)),
            None => {
                // Failed to parse initializer expression
                state.error(parse_msg(format!(
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
                    kind: StmtKind::Binding {
                        name,
                        type_name: None,
                        method_type: None,
                        signature_params: Vec::new(),
                        // 保留变量的显式类型注解，供后续类型检查做 Void/Int 等一致性校验。
                        type_annotation: type_annotation.clone(),

                        params: Vec::new(),
                        body: block.stmts.clone(),
                        is_pub: final_is_pub,
                    },
                    span,
                });
            }
        }

        return Some(Stmt {
            kind: StmtKind::Var {
                name,
                name_span,
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
            name_span,
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
fn parse_type_definition(state: &mut ParserState<'_>) -> Option<Type> {
    let first_type = parse_type_annotation(state)?;

    // 检查是否有 | 符号（不允许使用不带花括号的枚举语法）
    if state.at(&TokenKind::Pipe) {
        state.error(parse_msg(
            "Enum variants must use brace syntax: `{ red | green | blue }` instead of `red | green | blue`".to_string(),
        ));
        return None;
    }

    Some(first_type)
}
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
            state.error(parse_msg(
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

    // RFC-004: 检测外部绑定语句: Type.method = function[pos]
    if is_external_binding_syntax(state) {
        return parse_external_binding_stmt(state, span);
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

        let name_span = state.current().map(|t| t.span);
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                state.error(
                    ErrorCodeDefinition::unexpected_token(&format!(
                        "{:?}",
                        state
                            .current()
                            .map(|t| t.kind.clone())
                            .unwrap_or(TokenKind::Eof)
                    ))
                    .at(state.span())
                    .build(),
                );
                return None;
            }
        };
        let name_span = name_span.unwrap_or_else(Span::dummy);
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
                    kind: StmtKind::Binding {
                        name,
                        type_name: None,
                        method_type: None,
                        signature_params: Vec::new(),
                        type_annotation: None, // Will be inferred

                        params: Vec::new(),
                        body: block.stmts.clone(),
                        is_pub,
                    },
                    span,
                });
            }

            return Some(Stmt {
                kind: StmtKind::Var {
                    name,
                    name_span,
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

    // Check for tuple destructuring: identifier, identifier, ... = expr
    // This must be checked BEFORE the `:` check since `,` comes first
    if matches!(next.map(|t| &t.kind), Some(TokenKind::Comma)) {
        // Save position in case this turns out to not be destructuring
        let saved = state.save_position();
        let err_count = state.error_count();

        let first_token = state.current().unwrap();
        let first_name = SpannedIdent {
            name: match &first_token.kind {
                TokenKind::Identifier(n) => n.clone(),
                _ => {
                    state.restore_position(saved);
                    state.truncate_errors(err_count);
                    return parse_expr_stmt(state, span);
                }
            },
            span: first_token.span,
        };
        state.bump(); // consume first identifier

        let mut names = vec![first_name];

        // Collect comma-separated identifiers
        while state.skip(&TokenKind::Comma) {
            let tok = match state.current() {
                Some(t) if matches!(&t.kind, TokenKind::Identifier(_)) => t.clone(),
                _ => {
                    state.restore_position(saved);
                    state.truncate_errors(err_count);
                    return parse_expr_stmt(state, span);
                }
            };
            names.push(SpannedIdent {
                name: match tok.kind {
                    TokenKind::Identifier(n) => n,
                    _ => unreachable!(),
                },
                span: tok.span,
            });
            state.bump(); // consume identifier
        }

        // Expect `=`
        if !state.at(&TokenKind::Eq) {
            state.restore_position(saved);
            state.truncate_errors(err_count);
            return parse_expr_stmt(state, span);
        }
        state.bump(); // consume `=`

        // Parse RHS expression
        let rhs = match state.parse_expression(BP_LOWEST) {
            Some(expr) => expr,
            None => {
                state.error(parse_msg(
                    "Expected expression after '=' in tuple destructuring".to_string(),
                ));
                return None;
            }
        };

        state.skip(&TokenKind::Semicolon);

        return Some(Stmt {
            kind: StmtKind::DestructureAssign {
                names,
                rhs: Box::new(rhs),
                span,
            },
            span,
        });
    }

    // Check for variable declaration: identifier followed by :
    if matches!(next.map(|t| &t.kind), Some(TokenKind::Colon)) {
        // 传递已检测到的 is_pub 给 parse_var_stmt
        return parse_var_stmt_with_pub(state, span, Some(is_pub));
    }

    // Otherwise, parse as expression
    parse_expr_stmt(state, span)
}
pub fn parse_constructor(state: &mut ParserState<'_>) -> Option<VariantDef> {
    let name_span = state.span();
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => {
            state.error(
                ErrorCodeDefinition::unexpected_token(&format!(
                    "{:?}",
                    state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof)
                ))
                .at(state.span())
                .build(),
            );
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
        name_span,
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

/// Parse parenthesized tuple destructuring: `(a, b) = expr`
/// Called when the current token is `(`.
/// Falls back to expression statement if this is not a destructuring pattern.
pub fn parse_paren_destructure_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    let saved = state.save_position();
    let err_count = state.error_count();

    state.bump(); // consume `(`

    // Check if next token is an identifier (first name in destructuring)
    let first_token = match state.current() {
        Some(t) if matches!(&t.kind, TokenKind::Identifier(_)) => t.clone(),
        _ => {
            // Not a destructuring pattern, fall back to expression
            state.restore_position(saved);
            state.truncate_errors(err_count);
            return parse_expr_stmt(state, span);
        }
    };
    let first_name = SpannedIdent {
        name: match &first_token.kind {
            TokenKind::Identifier(n) => n.clone(),
            _ => unreachable!(),
        },
        span: first_token.span,
    };
    state.bump(); // consume first identifier

    let mut names = vec![first_name];

    // Collect remaining comma-separated identifiers
    while state.at(&TokenKind::Comma) {
        state.bump(); // consume `,`
        let tok = match state.current() {
            Some(t) if matches!(&t.kind, TokenKind::Identifier(_)) => t.clone(),
            _ => {
                // Not a destructuring pattern after comma
                state.restore_position(saved);
                state.truncate_errors(err_count);
                return parse_expr_stmt(state, span);
            }
        };
        names.push(SpannedIdent {
            name: match tok.kind {
                TokenKind::Identifier(n) => n,
                _ => unreachable!(),
            },
            span: tok.span,
        });
        state.bump(); // consume identifier
    }

    // Expect `)`
    if !state.at(&TokenKind::RParen) {
        state.restore_position(saved);
        state.truncate_errors(err_count);
        return parse_expr_stmt(state, span);
    }
    state.bump(); // consume `)`

    // Expect `=`
    if !state.at(&TokenKind::Eq) {
        state.restore_position(saved);
        state.truncate_errors(err_count);
        return parse_expr_stmt(state, span);
    }
    state.bump(); // consume `=`

    // Parse RHS expression
    let rhs = match state.parse_expression(BP_LOWEST) {
        Some(expr) => expr,
        None => {
            state.error(parse_msg(
                "Expected expression after '=' in tuple destructuring".to_string(),
            ));
            return None;
        }
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::DestructureAssign {
            names,
            rhs: Box::new(rhs),
            span,
        },
        span,
    })
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
