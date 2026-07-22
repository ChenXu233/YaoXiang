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
use crate::frontend::core::parser::{ParserState, BP_LOWEST, BP_ASSIGN};
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::frontend::core::parser::parse_msg;
use crate::util::span::Span;

// Import from sibling modules
use super::types::{parse_type_annotation, parse_fn_type_with_names};

/// 判断 annotation 是否为类型参数标注（`Type` / `MetaType`）。纯结构，不看名字。
fn is_type_param_annotation(ty: Option<&Type>) -> bool {
    match ty {
        Some(Type::MetaType { .. }) => true,
        Some(Type::Name { name, .. }) => name == "Type",
        _ => false,
    }
}

/// 结构判定：参数名是否在给定类型中作为类型引用（`Type::Name`）出现。
/// 用于识别 const 泛型参数——如 `(n: N)` 里的 N 被当类型用。不看大小写。
fn name_used_as_type(
    name: &str,
    ty: &Type,
) -> bool {
    match ty {
        Type::Name { name: n, .. } => n == name,
        Type::Generic { args, .. } => args.iter().any(|a| name_used_as_type(name, a)),
        Type::Fn {
            params,
            return_type,
        } => {
            params.iter().any(|p| name_used_as_type(name, p))
                || name_used_as_type(name, return_type)
        }
        Type::Option(inner) | Type::Ptr(inner) => name_used_as_type(name, inner),
        Type::Ref { inner, .. } => name_used_as_type(name, inner),
        Type::Result(a, b) => name_used_as_type(name, a) || name_used_as_type(name, b),
        Type::Tuple(types) | Type::Sum(types) => types.iter().any(|t| name_used_as_type(name, t)),
        Type::Literal { name: n, .. } => n == name,
        _ => false,
    }
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

/// Parse variable declaration: `[mut] [pub] name[: type] [= expr];`
/// Function definition: `[pub] name: (ParamTypes) -> ReturnType = (params) => body;`
/// Generic function: `[pub] name[T: Clone]: (ParamTypes) -> ReturnType = (params) => body;`
pub fn parse_var_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    parse_var_stmt_with_pub(state, span, None)
}

/// Parse variable declaration with optional pre-detected pub modifier.
/// target 已被调用者解析为表达式；此函数处理 `:` 类型标注 / `=` 初始化 部分。
fn parse_assign_after_target(
    state: &mut ParserState<'_>,
    target: Expr,
    is_pub: bool,
    is_mut: bool,
    span: Span,
) -> Option<Stmt> {
    // RFC-010: 检测并拒绝旧语法: name(Type, ...) -> Ret = ...
    // 旧语法特征: 标识符后面直接跟着 (，没有 :
    if state.at(&TokenKind::LParen) {
        let saved = state.save_position();
        state.bump();
        let mut paren_depth = 1;
        while paren_depth > 0 && !state.at_end() {
            if state.at(&TokenKind::LParen) {
                paren_depth += 1;
            } else if state.at(&TokenKind::RParen) {
                paren_depth -= 1;
            }
            state.bump();
        }
        let is_old_syntax = state.at(&TokenKind::Arrow);
        state.restore_position(saved);
        if is_old_syntax {
            state.error(parse_msg(
                "旧函数语法已不再支持，请使用新语法: name: (param: Type, ...) -> Ret = body"
                    .to_string(),
            ));
            return None;
        }
    }

    // ── 解析类型标注 ──
    // RFC-010: name: (a: Int, b: Int) -> Ret = body (参数名在签名中)
    // 特征: 冒号后面跟着函数类型 (param: Type, ...) -> Ret
    let (type_annotation, fn_params) = if state.at(&TokenKind::Colon) {
        state.bump(); // consume ':'

        if state.at(&TokenKind::LParen) {
            // Look ahead to check if this is RFC-010 function syntax
            let saved = state.save_position();
            state.bump(); // consume '('

            let looks_like_named_params = if state.at(&TokenKind::RParen) {
                true
            } else if state.at(&TokenKind::KwMut) {
                true
            } else if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
                let next = state.peek().map(|t| &t.kind);
                matches!(next, Some(TokenKind::Colon))
                    || matches!(next, Some(TokenKind::Comma) | Some(TokenKind::RParen))
            } else {
                false
            };

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
                let (fn_params_parsed, all_curry_params, return_type) =
                    parse_fn_type_with_names(state)?;

                let param_types: Vec<Type> = fn_params_parsed
                    .iter()
                    .filter_map(|p| p.ty.clone())
                    .collect();
                let type_annotation = Type::Fn {
                    params: param_types,
                    return_type: return_type.clone(),
                };
                (Some(type_annotation), Some(all_curry_params))
            } else {
                // Check old function syntax: (Type, Type) -> Ret
                let saved_check = state.save_position();
                state.bump();
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
                    state.error(parse_msg("Old function syntax '(Type, Type) -> Ret' is no longer supported. \
                             Use RFC-010 syntax with named parameters: '(param: Type, ...) -> Ret'. \
                             Example: 'add: (a: Int, b: Int) -> Int = a + b'".to_string()));
                    return None;
                }

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

    // Check for invalid syntax after type annotation
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

    // ── 可选初始化 ──
    if state.skip(&TokenKind::Eq) {
        // Check if this is a generic type constructor definition:
        //   name: (T: Type, ...) -> Type = { ... }
        if let Some(Type::Fn { return_type, .. }) = &type_annotation {
            if matches!(return_type.as_ref(), Type::MetaType { .. }) {
                // 类型构造器
                if let Expr::Var(name, _) = &target {
                    let definition = parse_type_definition(state)?;
                    state.skip(&TokenKind::Semicolon);
                    return Some(Stmt {
                        kind: StmtKind::TypeDefinition {
                            name: name.clone(),
                            signature_params: fn_params.clone().unwrap_or_default(),
                            definition,
                            is_pub,
                        },
                        span,
                    });
                }
            }
        }

        // Case 1: 有 fn_params (RFC-010 新语法)
        if let Some(ref extracted_params) = fn_params {
            let saved = state.save_position();
            let init = state.parse_expression(BP_LOWEST);

            match init {
                Some(Expr::Lambda {
                    params: lambda_params,
                    body,
                    ..
                }) => {
                    // Lambda 形式: name: (a: Int, b: Int) -> Int = (a, b) => a + b
                    let lambda_names: std::collections::HashSet<&str> =
                        lambda_params.iter().map(|p| p.name.as_str()).collect();
                    let value_params: Vec<&Param> = extracted_params
                        .iter()
                        .filter(|p| lambda_names.contains(p.name.as_str()))
                        .collect();

                    if !value_params.is_empty() {
                        if lambda_params.len() != value_params.len() {
                            state.error(parse_msg(format!(
                            "Parameter count mismatch: signature has {} value parameters, lambda has {}",
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
                                "Parameter name mismatch at position {}: signature has '{}', lambda has '{}'.",
                                i + 1,
                                sig_param.name,
                                lambda_param.name
                            )));
                                return None;
                            }
                        }
                    }

                    // 值参数对齐
                    let merged: Vec<Param> = if value_params.is_empty() {
                        lambda_params.to_vec()
                    } else {
                        let sig_by_name: std::collections::HashMap<&str, &Param> = extracted_params
                            .iter()
                            .map(|p| (p.name.as_str(), p))
                            .collect();
                        lambda_params
                            .iter()
                            .map(|lam| {
                                let sig =
                                    sig_by_name.get(lam.name.as_str()).copied().unwrap_or(lam);
                                Param {
                                    name: lam.name.clone(),
                                    ty: sig.ty.clone(),
                                    is_mut: lam.is_mut,
                                    span: lam.span,
                                }
                            })
                            .collect()
                    };

                    // 构建 Lambda value（包含 merged params 和 body）
                    let value = Expr::Lambda {
                        params: merged,
                        body: body.clone(),
                        span,
                    };
                    state.skip(&TokenKind::Semicolon);
                    return Some(Stmt {
                        kind: StmtKind::Assign {
                            target: Box::new(target),
                            type_annotation,
                            signature_params: extracted_params.clone(),
                            value: Some(Box::new(value)),
                            is_pub,
                            is_mut,
                            span,
                        },
                        span,
                    });
                }
                Some(expr) => {
                    // 直接表达式: name: (a: Int, b: Int) -> Int = a + b
                    state.skip(&TokenKind::Semicolon);

                    // 值参数判定
                    let value_params: Vec<Param> = extracted_params
                        .iter()
                        .filter(|p| {
                            if is_type_param_annotation(p.ty.as_ref()) {
                                return false;
                            }
                            let used_as_const = type_annotation
                                .as_ref()
                                .is_some_and(|ann| name_used_as_type(&p.name, ann));
                            !used_as_const
                        })
                        .cloned()
                        .collect();

                    // 构建 Lambda value（参数从签名提取，body 是表达式）
                    let body_block = if let Expr::Block(block) = &expr {
                        block.clone()
                    } else {
                        Block {
                            stmts: vec![Stmt {
                                kind: StmtKind::Expr(Box::new(expr.clone())),
                                span: state.span(),
                            }],
                            span: state.span(),
                        }
                    };
                    let value = Expr::Lambda {
                        params: value_params,
                        body: Box::new(body_block),
                        span,
                    };
                    return Some(Stmt {
                        kind: StmtKind::Assign {
                            target: Box::new(target),
                            type_annotation,
                            signature_params: extracted_params.clone(),
                            value: Some(Box::new(value)),
                            is_pub,
                            is_mut,
                            span,
                        },
                        span,
                    });
                }
                None => {
                    state.restore_position(saved);
                }
            }
        }

        // Case 2: 类型定义 `Point: Type = { ... }`
        if let Some(Type::MetaType { .. }) = &type_annotation {
            if let Expr::Var(name, _) = &target {
                // Easter egg: Type: Type = Type
                if name == "Type" {
                    let saved_easter = state.save_position();
                    if let Some(TokenKind::Identifier(val)) = state.current().map(|t| &t.kind) {
                        if val == "Type" {
                            state.bump();
                            state.skip(&TokenKind::Semicolon);
                            return Some(Stmt {
                                kind: StmtKind::TypeDefinition {
                                    name: "Type".to_string(),
                                    signature_params: Vec::new(),
                                    definition: Type::MetaType {
                                        name_span: Span::dummy(),
                                        args: Vec::new(),
                                    },
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
                    kind: StmtKind::TypeDefinition {
                        name: name.clone(),
                        signature_params: Vec::new(),
                        definition,
                        is_pub: false,
                    },
                    span,
                });
            }
        }

        // Case 3: 普通变量初始化 (with or without type annotation)
        let initializer = match state.parse_expression(BP_LOWEST) {
            Some(expr) => expr,
            None => {
                state.error(parse_msg("Expected expression after '='".to_string()));
                return None;
            }
        };

        state.skip(&TokenKind::Semicolon);

        // x = { ... } → Block 作为 value，typechecker 识别为函数定义
        return Some(Stmt {
            kind: StmtKind::Assign {
                target: Box::new(target),
                type_annotation,
                signature_params: Vec::new(),
                value: Some(Box::new(initializer)),
                is_pub,
                is_mut,
                span,
            },
            span,
        });
    }

    // 无初始化: `x: Int` 或 `x` (纯声明)
    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Assign {
            target: Box::new(target),
            type_annotation,
            signature_params: Vec::new(),
            value: None,
            is_pub,
            is_mut,
            span,
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
/// Parse variable declaration with optional pre-detected pub modifier.
/// This is the entry point when `:` is detected after an identifier.
fn parse_var_stmt_with_pub(
    state: &mut ParserState<'_>,
    span: Span,
    pre_detected_pub: Option<bool>,
) -> Option<Stmt> {
    let is_mut = state.skip(&TokenKind::KwMut);

    let final_is_pub = if pre_detected_pub == Some(true) {
        state.skip(&TokenKind::KwPub);
        true
    } else {
        state.skip(&TokenKind::KwPub)
    };

    // Parse target as expression (identifier → Var)
    let target = match state.current() {
        Some(t) => match &t.kind {
            TokenKind::Identifier(n) => {
                let name = n.clone();
                let name_span = t.span;
                state.bump();
                Expr::Var(name, name_span)
            }
            _ => {
                let cur_kind = state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof);
                state.error(
                    ErrorCodeDefinition::unexpected_token(&format!("{:?}", cur_kind))
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

    parse_assign_after_target(state, target, final_is_pub, is_mut, span)
}

pub fn parse_identifier_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // 首先检测并拒绝旧语法: identifier( 后跟类型参数和 ->
    if let Some(TokenKind::LParen) = state.peek().map(|t| &t.kind) {
        if is_old_function_syntax(state) {
            state.error(parse_msg(
                "旧语法已弃用。请使用新语法: `name: (Types) -> ReturnType = (params) => body`"
                    .to_string(),
            ));
            skip_old_function_syntax(state);
            return None;
        }
    }

    // 检测 pub 关键字
    let is_pub = if matches!(state.current().map(|t| &t.kind), Some(TokenKind::KwPub)) {
        state.bump();
        true
    } else {
        false
    };

    // 检测 mut 关键字
    let is_mut = state.skip(&TokenKind::KwMut);

    // ── 解析 target 表达式 ──
    // 使用 BP_ASSIGN + 1 在 `=` 处停止，但 `.` 和 `[` 是 BP_CALL=9 所以会被解析
    let target = match state.parse_expression(BP_ASSIGN + 1) {
        Some(expr) => expr,
        None => {
            // 解析失败，可能是旧语法残留
            if is_pub || is_mut {
                state.error(parse_msg("Expected expression after pub/mut".to_string()));
                return None;
            }
            return parse_expr_stmt(state, span);
        }
    };

    // ── 元组解构: target, identifier, ... = expr ──
    // target 后面跟逗号说明可能是元组解构
    if state.at(&TokenKind::Comma) {
        let saved = state.save_position();
        let err_count = state.error_count();

        // 从 target 提取第一个名字
        let first_name = if let Expr::Var(n, s) = &target {
            SpannedIdent {
                name: n.clone(),
                span: *s,
            }
        } else {
            // target 不是 Var，不是解构
            state.restore_position(saved);
            state.truncate_errors(err_count);
            // 回退为表达式语句
            state.skip(&TokenKind::Semicolon);
            return Some(Stmt {
                kind: StmtKind::Expr(Box::new(target)),
                span,
            });
        };

        state.bump(); // consume ','

        let mut names = vec![first_name];

        while !state.at_end() {
            let tok = match state.current() {
                Some(t) if matches!(&t.kind, TokenKind::Identifier(_)) => t.clone(),
                _ => break,
            };
            names.push(SpannedIdent {
                name: match &tok.kind {
                    TokenKind::Identifier(n) => n.clone(),
                    _ => break,
                },
                span: tok.span,
            });
            state.bump();

            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }

        // Expect `=`
        if !state.at(&TokenKind::Eq) {
            state.restore_position(saved);
            state.truncate_errors(err_count);
            // Fallback: treat as expression statement
            state.skip(&TokenKind::Semicolon);
            return Some(Stmt {
                kind: StmtKind::Expr(Box::new(target)),
                span,
            });
        }
        state.bump(); // consume `=`

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

    // ── `:` 类型标注 ──
    if state.at(&TokenKind::Colon) {
        return parse_assign_after_target(state, target, is_pub, is_mut, span);
    }

    // ── `=` 初始化 ──
    if state.at(&TokenKind::Eq) {
        state.bump(); // consume '='

        // Parse value expression
        let value = match state.parse_expression(BP_LOWEST) {
            Some(expr) => expr,
            None => {
                state.error(parse_msg("Expected expression after '='".to_string()));
                return None;
            }
        };

        state.skip(&TokenKind::Semicolon);

        return Some(Stmt {
            kind: StmtKind::Assign {
                target: Box::new(target),
                type_annotation: None,
                signature_params: Vec::new(),
                value: Some(Box::new(value)),
                is_pub,
                is_mut,
                span,
            },
            span,
        });
    }

    // ── 否则: 表达式语句 ──
    state.skip(&TokenKind::Semicolon);

    if is_pub || is_mut {
        // pub/mut 后面不是赋值或类型标注，报错
        state.error(parse_msg(
            "Expected ':' or '=' after pub/mut identifier".to_string(),
        ));
        return None;
    }

    Some(Stmt {
        kind: StmtKind::Expr(Box::new(target)),
        span,
    })
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
