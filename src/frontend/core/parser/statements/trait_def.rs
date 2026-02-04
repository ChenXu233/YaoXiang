//! Trait 定义和实现解析
//!
//! 实现 RFC-011 Trait 系统解析：
//! - `type TraitName = { method: (params) -> ret }`
//! - `impl TraitName for Type { ... }`

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::{
    TraitMethod, TraitDef, TraitImpl, MethodImpl, Stmt, StmtKind, Param, Type, Block, GenericParam,
};
use crate::frontend::core::parser::{ParserState, ParseError};
use crate::util::span::Span;

/// 解析 Trait 定义: `type TraitName = { method: (params) -> ret }`
pub fn parse_trait_def_stmt(
    state: &mut ParserState<'_>,
    _start_span: Span,
) -> Option<Stmt> {
    // consume `type`
    state.bump();

    // 解析 Trait 名称
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected trait name after 'type'".to_string(),
            ));
            return None;
        }
    };

    // 解析泛型参数（可选）
    let generic_params = if state.at(&TokenKind::LBracket) {
        parse_trait_generic_params(state)?
    } else {
        vec![]
    };

    // 期望 `=`
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // 解析父 Trait（可选，继承语法）
    let parent_traits = parse_trait_parents(state)?;

    // 期望 `{`
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    // 解析方法列表
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        // 跳过分号
        state.skip(&TokenKind::Semicolon);

        if state.at(&TokenKind::RBrace) {
            break;
        }

        // 解析方法定义
        if let Some(method) = parse_trait_method(state) {
            methods.push(method);
        } else {
            // 解析失败，恢复并跳过
            state.synchronize();
        }

        // 跳过分号（方法间分隔符）
        state.skip(&TokenKind::Semicolon);
    }

    // 期望 `}`
    if state.expect(&TokenKind::RBrace) {
    } else {
        return None;
    }

    Some(Stmt {
        kind: StmtKind::TraitDef(TraitDef {
            name,
            generic_params,
            methods,
            parent_traits,
            span: state.span(),
        }),
        span: state.span(),
    })
}

/// 解析 Trait 泛型参数
fn parse_trait_generic_params(state: &mut ParserState<'_>) -> Option<Vec<GenericParam>> {
    // 期望 `[`
    if !state.expect(&TokenKind::LBracket) {
        return None;
    }

    let mut params = Vec::new();

    while !state.at(&TokenKind::RBracket) && !state.at_end() {
        // 解析泛型参数: `T` 或 `T: Trait`
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                state.bump();
                name
            }
            _ => {
                state.error(ParseError::Message(
                    "Expected generic parameter name".to_string(),
                ));
                return None;
            }
        };

        // 解析约束（可选）
        let mut constraints = Vec::new();
        if state.at(&TokenKind::Colon) {
            state.bump(); // consume `:`
                          // 解析类型作为约束
            if let Some(constraint) = parse_trait_type_constraint(state) {
                constraints.push(constraint);
            }
        }

        params.push(GenericParam { name, constraints });

        // 跳过逗号
        state.skip(&TokenKind::Comma);
    }

    // 期望 `]`
    if !state.expect(&TokenKind::RBracket) {
        return None;
    }

    Some(params)
}

/// 解析 Trait 父 Trait（继承）
fn parse_trait_parents(state: &mut ParserState<'_>) -> Option<Vec<Type>> {
    let mut parents = Vec::new();

    // 检查是否有继承语法 `:` 或 `+`
    if state.at(&TokenKind::Colon) || state.at(&TokenKind::Plus) {
        // consume `:` 或第一个 `+`
        state.bump();

        // 解析第一个父 Trait
        if let Some(parent) = parse_trait_type_constraint(state) {
            parents.push(parent);
        }

        // 解析更多父 Trait（使用 `+` 分隔）
        while state.at(&TokenKind::Plus) {
            state.bump(); // consume `+`
            if let Some(parent) = parse_trait_type_constraint(state) {
                parents.push(parent);
            }
        }
    }

    Some(parents)
}

/// 解析 Trait 类型约束
fn parse_trait_type_constraint(state: &mut ParserState<'_>) -> Option<Type> {
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();

            // 检查是否是泛型类型 `<T, U>`
            if state.at(&TokenKind::Lt) {
                state.bump(); // consume `<`
                let mut args = Vec::new();
                while !state.at(&TokenKind::Gt) && !state.at_end() {
                    if let Some(arg) = parse_trait_type_constraint(state) {
                        args.push(arg);
                    }
                    state.skip(&TokenKind::Comma);
                }
                if !state.expect(&TokenKind::Gt) {
                    return None;
                }
                return Some(Type::Generic { name, args });
            }

            Some(Type::Name(name))
        }
        _ => {
            state.error(ParseError::Message("Expected type constraint".to_string()));
            None
        }
    }
}

/// 解析 Trait 方法定义
fn parse_trait_method(state: &mut ParserState<'_>) -> Option<TraitMethod> {
    // 解析方法名
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected method name in trait".to_string(),
            ));
            return None;
        }
    };

    // 期望 `(`
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // 解析参数列表
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);

        // 跳过逗号
        state.skip(&TokenKind::Comma);
    }

    // 期望 `)`
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // 解析返回类型（可选）
    let return_type = if state.at(&TokenKind::Arrow) {
        state.bump(); // consume `->`
        Some(parse_trait_return_type(state)?)
    } else {
        None
    };

    Some(TraitMethod {
        name,
        params,
        return_type,
        span: state.span(),
    })
}

/// 解析 Trait 方法参数
fn parse_trait_method_param(state: &mut ParserState<'_>) -> Option<Param> {
    // 第一个参数可能是 `self` 或 `self: Type`
    if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
        if name == "self" || name == "Self" {
            let self_name = name.clone();
            state.bump();

            // 检查是否有类型注解
            if state.at(&TokenKind::Colon) {
                state.bump(); // consume `:`
                let ty = parse_trait_return_type(state)?;
                return Some(Param {
                    name: self_name,
                    ty: Some(ty),
                    span: state.span(),
                });
            }

            // self 默认类型为 Self
            return Some(Param {
                name: self_name,
                ty: Some(Type::Name("Self".to_string())),
                span: state.span(),
            });
        }
    }

    // 解析普通参数: `name: Type`
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message("Expected parameter name".to_string()));
            return None;
        }
    };

    // 期望 `:`
    if !state.expect(&TokenKind::Colon) {
        return None;
    }

    // 解析类型
    let ty = parse_trait_return_type(state)?;

    Some(Param {
        name,
        ty: Some(ty),
        span: state.span(),
    })
}

/// 解析返回类型
fn parse_trait_return_type(state: &mut ParserState<'_>) -> Option<Type> {
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(_)) => {
            // 解析标识符
            let name = if let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
                n.clone()
            } else {
                return None;
            };
            state.bump();

            // 检查是否是泛型类型 `<T>`
            if state.at(&TokenKind::Lt) {
                state.bump(); // consume `<`
                let mut args = Vec::new();
                while !state.at(&TokenKind::Gt) && !state.at_end() {
                    if let Some(arg) = parse_trait_return_type(state) {
                        args.push(arg);
                    }
                    state.skip(&TokenKind::Comma);
                }
                if !state.expect(&TokenKind::Gt) {
                    return None;
                }
                return Some(Type::Generic { name, args });
            }

            // 检查是否是 Void 类型
            if name == "Void" {
                return Some(Type::Void);
            }

            Some(Type::Name(name))
        }
        Some(TokenKind::LParen) => {
            // 函数类型: `(T1, T2) -> T`
            state.bump(); // consume `(`
            let mut params = Vec::new();
            while !state.at(&TokenKind::RParen) && !state.at_end() {
                if let Some(ty) = parse_trait_return_type(state) {
                    params.push(ty);
                }
                state.skip(&TokenKind::Comma);
            }
            if !state.expect(&TokenKind::RParen) {
                return None;
            }

            // 期望 `->`
            if !state.expect(&TokenKind::Arrow) {
                return None;
            }

            let ret = parse_trait_return_type(state)?;

            Some(Type::Fn {
                params,
                return_type: Box::new(ret),
            })
        }
        _ => {
            state.error(ParseError::Message("Expected return type".to_string()));
            None
        }
    }
}

/// 解析 Trait 实现: `impl TraitName for Type { ... }`
/// 注意: impl 不是关键字，使用标识符检测
pub fn parse_trait_impl_stmt(
    state: &mut ParserState<'_>,
    _start_span: Span,
) -> Option<Stmt> {
    // consume `impl` 标识符
    state.bump();

    // 解析 Trait 名称
    let trait_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected trait name after 'impl'".to_string(),
            ));
            return None;
        }
    };

    // 期望 `for`
    if !state.expect(&TokenKind::KwFor) {
        return None;
    }

    // 解析实现针对的类型
    let for_type = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            Type::Name(name)
        }
        _ => {
            state.error(ParseError::Message("Expected type after 'for'".to_string()));
            return None;
        }
    };

    // 期望 `{`
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    // 解析方法实现
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(method) = parse_trait_method_impl(state) {
            methods.push(method);
        } else {
            state.synchronize();
        }
        state.skip(&TokenKind::Semicolon);
    }

    // 期望 `}`
    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    Some(Stmt {
        kind: StmtKind::TraitImpl(TraitImpl {
            trait_name,
            for_type,
            methods,
            span: state.span(),
        }),
        span: state.span(),
    })
}

/// 解析 Trait 方法实现
fn parse_trait_method_impl(state: &mut ParserState<'_>) -> Option<MethodImpl> {
    // 解析方法名
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message("Expected method name".to_string()));
            return None;
        }
    };

    // 期望 `(`
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // 解析参数列表
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);
        state.skip(&TokenKind::Comma);
    }

    // 期望 `)`
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // 解析返回类型（可选）
    let return_type = if state.at(&TokenKind::Arrow) {
        state.bump();
        Some(parse_trait_return_type(state)?)
    } else {
        None
    };

    // 期望 `=`
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // 解析方法体
    let body = if state.at(&TokenKind::LBrace) {
        // 块作为函数体 - Block.expr 已经是 Option<Box<Expr>>
        let block = parse_method_body(state)?;
        (block.stmts, block.expr)
    } else {
        // 简化的表达式作为函数体
        let expr = state.parse_expression(0);
        (Vec::new(), expr.map(Box::new))
    };

    Some(MethodImpl {
        name,
        params,
        return_type,
        body,
        span: state.span(),
    })
}

/// 解析方法体块
fn parse_method_body(state: &mut ParserState<'_>) -> Option<Block> {
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    let mut stmts = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(stmt) = state.parse_statement() {
            stmts.push(stmt);
        } else {
            state.bump();
        }
    }

    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    Some(Block {
        stmts,
        expr: None,
        span: state.span(),
    })
}

/// 检测是否是 Trait 定义语句
/// 模式: `type Identifier = { ... }`
pub fn is_trait_def_stmt(state: &mut ParserState<'_>) -> bool {
    if !matches!(state.current().map(|t| &t.kind), Some(TokenKind::KwType)) {
        return false;
    }

    let saved = state.save_position();
    state.bump(); // consume `type`

    let is_trait = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump(); // consume identifier

        // 检查是否是 `=`（或者是 `:` 继承语法）
        state.at(&TokenKind::Eq) || state.at(&TokenKind::Colon)
    } else {
        false
    };

    state.restore_position(saved);
    is_trait
}

/// 检测是否是 Trait 实现语句
/// 模式: `impl Identifier for Type { ... }`
/// 由于 `impl` 不是关键字，需要检查标识符
pub fn is_trait_impl_stmt(state: &mut ParserState<'_>) -> bool {
    if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
        if name != "impl" {
            return false;
        }
    } else {
        return false;
    }

    let saved = state.save_position();
    state.bump(); // consume `impl`

    let is_impl = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump(); // consume trait name

        // 检查是否是 `for` 关键字
        state.at(&TokenKind::KwFor)
    } else {
        false
    };

    state.restore_position(saved);
    is_impl
}
