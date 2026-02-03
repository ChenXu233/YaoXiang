//! Declaration parsing - variables, types, imports, functions
//!
//! Implements parsing for:
//! - Variable declarations: `[mut] name[: type] [= expr];`
//! - Type definitions: `type Name = Type;`
//! - Use imports: `use path;` or `use path.{item1, item2};`
//! - Function definitions: `name: (ParamTypes) -> ReturnType = (params) => body;`

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::{ParserState, ParseError, BP_LOWEST};
use crate::util::span::Span;

/// 检测是否是旧的函数定义语法: `name(Types) -> ReturnType = ...`
/// 通过向前看来检测这种模式
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
    // 新语法在括号前有冒号: name: (Types) -> ...
    let is_old = state.at(&TokenKind::Arrow);

    // 恢复位置
    state.restore_position(saved);

    is_old
}

/// 跳过旧函数语法的整个声明
fn skip_old_function_syntax(state: &mut ParserState<'_>) {
    // 跳过直到找到分号或遇到新的语句开始
    while !state.at_end() {
        if state.at(&TokenKind::Semicolon) {
            state.bump();
            break;
        }
        // 遇到新语句的关键字时停止
        if matches!(
            state.current().map(|t| &t.kind),
            Some(TokenKind::KwType)
                | Some(TokenKind::KwUse)
                | Some(TokenKind::KwMut)
                | Some(TokenKind::KwReturn)
                | Some(TokenKind::KwBreak)
                | Some(TokenKind::KwContinue)
        ) {
            break;
        }
        state.bump();
    }
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

    // Extract params and body from lambda
    let (params, body_stmts, body_expr) = match &initializer {
        Expr::Lambda { params, body, .. } => {
            (params.clone(), body.stmts.clone(), body.expr.clone())
        }
        _ => {
            state.error(ParseError::Message(
                "Method body must be a lambda expression".to_string(),
            ));
            return None;
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

    // Parse type annotation and generic params
    // Two cases:
    // 1. name: TypeAnnotation = ...
    // 2. name[GenericParams]: TypeAnnotation = ...
    let (type_annotation, generic_params) = if state.at(&TokenKind::Colon) {
        state.bump(); // consume ':'

        // Check if this is a generic param declaration: [T: Clone] or <T: Clone]
        // These come right after ':'
        let has_generic_syntax = state.at(&TokenKind::LBracket) || state.at(&TokenKind::Lt);

        if has_generic_syntax {
            // Parse generic params
            let generic = parse_generic_params_with_constraints(state)?;

            // After generic params, the type annotation follows
            // It could be:
            // - (Params) -> ReturnType (function type)
            // - Other type
            // We DON'T expect another ':' here - the type annotation starts directly
            let type_ann = parse_type_annotation(state)?;
            (Some(type_ann), generic)
        } else {
            // No generic params, just parse type annotation
            let type_ann = parse_type_annotation(state)?;
            (Some(type_ann), Vec::new())
        }
    } else {
        (None, Vec::new())
    };

    // Check for invalid syntax after type annotation.
    // Valid tokens after type annotation: `=`, `;`, newline/EOF
    // Invalid tokens include:
    // 1. `(` without `=` - e.g., `name: Type (params) => body` (missing =)
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
        // Check if this might be a function definition with unified syntax:
        // name: (ParamTypes) -> ReturnType = (params) => body
        // OR with generic params: name[T: Constraint]: (ParamTypes) -> ReturnType = body
        let saved_position = state.save_position();
        let init_opt = state.parse_expression(BP_LOWEST);

        if let Some(initializer) = init_opt {
            if let Some(ref type_annotation) = type_annotation {
                if let Type::Fn {
                    params: type_params,
                    return_type: _,
                } = type_annotation
                {
                    // Case 1: Initializer is a lambda expression
                    if let Expr::Lambda { params, body, .. } = &initializer {
                        // Merge type information from type annotation with parameter names from lambda
                        let mut merged_params = Vec::new();

                        for (i, lambda_param) in params.iter().enumerate() {
                            if let Some(ty) = type_params.get(i) {
                                merged_params.push(Param {
                                    name: lambda_param.name.clone(),
                                    ty: Some(ty.clone()),
                                    span: lambda_param.span,
                                });
                            } else {
                                merged_params.push(lambda_param.clone());
                            }
                        }

                        state.skip(&TokenKind::Semicolon);
                        return Some(Stmt {
                            kind: StmtKind::Fn {
                                name,
                                generic_params,
                                type_annotation: type_annotation.clone(),
                                params: merged_params,
                                body: (body.stmts.clone(), body.expr.clone()),
                                is_pub: final_is_pub,
                            },
                            span,
                        });
                    }
                    // Case 2: Initializer is a simple expression and we have generic params
                    // This is a generic function with expression body: test: [T](x: T) -> T = x
                    else if !generic_params.is_empty() {
                        state.skip(&TokenKind::Semicolon);
                        // Create function with empty params and expression body
                        // Extract params from type annotation
                        let params = type_params
                            .iter()
                            .enumerate()
                            .map(|(i, ty)| Param {
                                name: format!("__param_{}", i),
                                ty: Some(ty.clone()),
                                span: Span::dummy(),
                            })
                            .collect();

                        return Some(Stmt {
                            kind: StmtKind::Fn {
                                name,
                                generic_params,
                                type_annotation: type_annotation.clone(),
                                params,
                                body: (Vec::new(), Some(Box::new(initializer))),
                                is_pub: final_is_pub,
                            },
                            span,
                        });
                    }
            }
        }

        // If not a function definition, restore and parse as regular variable
        state.restore_position(saved_position);
        state.clear_errors();

        // Note: generic_params is not used for variable declarations
        let _generic_params = generic_params;

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

/// Parse type definition statement: `type Name = Type;`
/// Supports:
/// - Simple type: `type Color = red`
/// - Union type: `type Color = red | green | blue`
/// - Generic union: `type Result[T, E] = ok(T) | err(E)`
/// - Struct type: `type Point = Point(x: Float, y: Float)`
pub fn parse_type_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'type'

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

    // Parse generic parameters: type Result[T, E] = ...
    let _generic_params = parse_type_generic_params(state)?;

    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    let definition = parse_type_definition(state)?;

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::TypeDef { name, definition },
        span,
    })
}

/// Parse generic parameters for type definition: [T, E] or <T, E>
fn parse_type_generic_params(state: &mut ParserState<'_>) -> Option<Vec<String>> {
    let open = if state.at(&TokenKind::LBracket) {
        state.bump();
        TokenKind::RBracket
    } else if state.at(&TokenKind::Lt) {
        state.bump();
        TokenKind::Gt
    } else {
        return Some(Vec::new());
    };

    let mut params = Vec::new();
    while !state.at(&open) && !state.at_end() {
        if let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
            params.push(n.clone());
            state.bump();
            state.skip(&TokenKind::Comma);
        } else {
            break;
        }
    }

    if !state.expect(&open) {
        return None;
    }

    Some(params)
}

/// Parse generic parameters with constraints: `[T: Clone]`
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

        // Parse constraint: `T: Clone`
        let mut constraints = Vec::new();
        if state.skip(&TokenKind::Colon) {
            if let Some(constraint) = parse_type_annotation(state) {
                constraints.push(constraint);
            }
        }

        params.push(GenericParam { name, constraints });
        state.skip(&TokenKind::Comma);
    }

    if !state.expect(&TokenKind::RBracket) {
        return None;
    }

    Some(params)
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
                            .map(|(n, t)| (Some(n.clone()), t.clone()))
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
                state.clear_errors();
            } else if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
                let saved_position2 = state.save_position();

                if let Some(stmt) =
                    parse_fn_stmt_with_name_simple(state, name.clone(), span, is_pub)
                {
                    state.skip(&TokenKind::Semicolon);
                    return Some(stmt);
                }

                state.restore_position(saved_position2);
                state.clear_errors();
            }

            // Not a function definition, parse as variable declaration (name = expr)
            // This is a variable declaration with type inference
            let initializer = state.parse_expression(BP_LOWEST)?;
            state.skip(&TokenKind::Semicolon);
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
        state.clear_errors();

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
            span: param_span,
        });
    }

    Some(params)
}

/// Parse type annotation
pub fn parse_type_annotation(state: &mut ParserState<'_>) -> Option<Type> {
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();

            // Check for generic parameters: Type<T> or Type[T]
            if state.at(&TokenKind::Lt) {
                return parse_generic_type(name, state);
            }
            if state.at(&TokenKind::LBracket) {
                return parse_generic_type_bracket(name, state);
            }

            // Check for function type: Type -> ReturnType (single param without parentheses)
            if state.at(&TokenKind::Arrow) {
                state.bump(); // consume '->'
                let return_type = Box::new(parse_type_annotation(state)?);
                return Some(Type::Fn {
                    params: vec![Type::Name(name)],
                    return_type,
                });
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

/// Parse named struct type: `Name(x: Type, y: Type)`
fn parse_named_struct_type(
    name: String,
    state: &mut ParserState<'_>,
) -> Option<Type> {
    state.bump(); // consume '('

    let mut fields = Vec::new();

    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let field_name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        if !state.expect(&TokenKind::Colon) {
            return None;
        }

        let field_type = parse_type_annotation(state)?;
        fields.push((field_name, field_type));

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

/// Parse function type: `(Params) -> ReturnType`
fn parse_fn_type(state: &mut ParserState<'_>) -> Option<Type> {
    // Note: fn type uses syntax (Params) -> ReturnType without `fn` keyword

    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    let mut param_types = Vec::new();

    if !state.at(&TokenKind::RParen) {
        while let Some(ty) = parse_type_annotation(state) {
            param_types.push(ty);

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

    Some(Type::Fn {
        params: param_types,
        return_type,
    })
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

/// Parse struct type like `{ field: Type }` or `Name(x: Type, y: Type)`
fn parse_struct_type(state: &mut ParserState<'_>) -> Option<Type> {
    state.skip(&TokenKind::LBrace);

    let mut fields = Vec::new();

    if !state.at(&TokenKind::RBrace) {
        while let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
            let name = name.clone();
            state.bump();

            state.skip(&TokenKind::Colon);

            let field_type = parse_type_annotation(state)?;

            fields.push((name, field_type));

            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }

    state.skip(&TokenKind::RBrace);

    Some(Type::Struct(fields))
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
