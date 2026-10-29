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
use crate::frontend::core::parser::{NameKind, ParserState, BP_LOWEST, BP_ASSIGN};
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::frontend::core::parser::parse_msg;
use crate::util::span::Span;

// Import from sibling modules
use super::types::{parse_type_annotation_required, parse_fn_type_with_names};

/// 判断 annotation 是否为类型参数标注（`Type` / `MetaType`）。纯结构，不看名字。
fn is_type_param_annotation(ty: Option<&Type>) -> bool {
    match ty {
        Some(Type::MetaType { .. }) => true,
        Some(Type::Name { name, .. }) => name == "Type",
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

/// Parse variable declaration: `[mut] name[: type] [= expr];`
/// Function definition: `name: (ParamTypes) -> ReturnType = (params) => body;`
/// Generic function: `name[T: Clone]: (ParamTypes) -> ReturnType = (params) => body;`
pub fn parse_var_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    parse_var_stmt_with_mut(state, span)
}

/// Parse variable declaration after the leading `mut`, if any.
/// target 已被调用者解析为表达式；此函数处理 `:` 类型标注 / `=` 初始化 部分。
fn parse_assign_after_target(
    state: &mut ParserState<'_>,
    target: Expr,
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
                state,
                "旧函数语法已不再支持，请使用新语法: name: (param: Type, ...) -> Ret = body"
                    .to_string(),
            ));
            return None;
        }
    }

    // ── 解析类型标注 ──
    // RFC-010: name: (a: Int, b: Int) -> Ret = body (参数名在签名中)
    // 特征: 冒号后面跟着函数类型 (param: Type, ...) -> Ret
    let (type_annotation, is_fn_signature) = if state.at(&TokenKind::Colon) {
        state.bump(); // consume ':'

        if state.at(&TokenKind::LParen) {
            // Look ahead to check if this is RFC-010 function syntax
            let saved = state.save_position();
            state.bump(); // consume '('

            let looks_like_named_params =
                if state.at(&TokenKind::RParen) || state.at(&TokenKind::KwMut) {
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
                let (fn_params_parsed, return_type) = parse_fn_type_with_names(state)?;

                // 名字与类型同栖（RFC-039 6.8.1）：形参原样进 Type::Fn.params，
                // 不再有第二份 curry 旁路（6.5.2 commit ③ 收敛返回值）。
                let type_annotation = Type::Fn {
                    params: fn_params_parsed,
                    return_type,
                };
                (Some(type_annotation), true)
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
                    state.error(parse_msg(state, "Old function syntax '(Type, Type) -> Ret' is no longer supported. \
                             Use RFC-010 syntax with named parameters: '(param: Type, ...) -> Ret'. \
                             Example: 'add: (a: Int, b: Int) -> Int = a + b'".to_string()));
                    return None;
                }

                let type_ann = parse_type_annotation_required(state)?;
                (Some(type_ann), false)
            }
        } else {
            let type_ann = parse_type_annotation_required(state)?;
            (Some(type_ann), false)
        }
    } else {
        (None, false)
    };

    // 纯声明: `name: Type` 后无 `=`，直接跟换行/分号/新语句
    // 仅拦截明确无效的后续 token（LParen/FatArrow/Comma）
    // Identifier 不拦截——它可能是下一个顶层语句（#168: 纯函数声明）
    if type_annotation.is_some() {
        let is_invalid = state.at(&TokenKind::LParen)
            || state.at(&TokenKind::FatArrow)
            || state.at(&TokenKind::Comma);
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

    // RFC-027：登记「返回 Type」的声明为**谓词**，供后续标注里的谓词应用判定。
    // 判据与下方类型体解析同源（`Type::Fn` 返回 `MetaType`/`Type`）。
    if let (Some(Type::Fn { return_type, .. }), Expr::Var(name, _)) = (&type_annotation, &target) {
        let returns_type = matches!(return_type.as_ref(), Type::MetaType { .. })
            || matches!(return_type.as_ref(), Type::Name { name: n, .. } if n == "Type");
        if returns_type {
            state.declare_predicate(name);
        }
    }

    // ── 可选初始化 ──
    if state.skip(&TokenKind::Eq) {
        // RFC-010: 返回 Type 时，先尝试类型体解析（字段/枚举）
        // 失败则 fall through 到表达式分支（精化约束）
        if let Some(Type::Fn { return_type, .. }) = &type_annotation {
            if matches!(return_type.as_ref(), Type::MetaType { .. }) {
                if let Expr::Var(name, _) = &target {
                    let saved = state.save_position();
                    if let Some(definition) = parse_type_definition(state) {
                        // 只有 Struct 才是类型体；ConstExpr 等是表达式，走 Assign
                        if matches!(&definition, Type::Struct { .. }) {
                            state.skip(&TokenKind::Semicolon);
                            return Some(Stmt {
                                kind: StmtKind::TypeDefinition {
                                    name: name.clone(),
                                    type_annotation: type_annotation.clone(),
                                    definition,
                                },
                                span,
                                plan_id: PlanId::UNPLANNED,
                            });
                        }
                    }
                    // 类型体解析失败（如 { x > 0 }）→ 回退，走表达式分支
                    state.restore_position(saved);
                }
            }
        }

        // Case 1: RFC-010 函数签名（注解是 Type::Fn）
        if is_fn_signature {
            // 拍平的签名形参从注解的 Type::Fn 链取得（6.5.2：唯一来源）
            let extracted_params = type_annotation
                .as_ref()
                .map(crate::frontend::core::parser::ast::Type::fn_params_flat)
                .unwrap_or_default();
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
                            state.error(parse_msg(state, format!(
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
                                state.error(parse_msg(state, format!(
                                "Parameter name mismatch at position {}: signature has '{}', lambda has '{}'.",
                                i + 1,
                                sig_param.name,
                                lambda_param.name
                            )));
                                return None;
                            }
                        }
                    }

                    // 值参数对齐（按位置）：上面已校验签名与 lambda 的值参数
                    // **逐位同名**且个数一致，故按位置取类型等价于按名查表。
                    // 6.5.2 commit ③ 删掉「按名重建 HashMap」这层派生——名字
                    // 曾同时存在于签名与 lambda 头两处，只需这一份位置对齐。
                    let merged: Vec<Param> = if value_params.is_empty() {
                        lambda_params.to_vec()
                    } else {
                        lambda_params
                            .iter()
                            .zip(value_params.iter())
                            .map(|(lam, sig)| Param {
                                name: lam.name.clone(),
                                ty: sig.ty.clone(),
                                is_mut: lam.is_mut,
                                span: lam.span,
                            })
                            .collect()
                    };

                    // RFC-004 括号语义：返回位置是 `Paren` ⇔ 链条在此终止。
                    // 作者写的 lambda（`(a) => a + 1`）就是**被返回的函数值**，
                    // 不是本函数的参数列表。故本函数只取第一组签名参数，
                    // body 为“返回这个 lambda”。
                    // 不加此判会让 `a` 被合并进外层，`f` 变成接受 `a` 而非返回闭包。
                    // RFC-027 §3：具名括号（`-> (r: P(r))`）与 `Paren` 同款——
                    // 括号同样声明「这是完整类型（值）」，链条到此终止。
                    let paren_return = matches!(
                        type_annotation.as_ref(),
                        Some(Type::Fn { return_type, .. }) if return_type.is_paren_complete()
                    );
                    if paren_return {
                        // 本函数自己的**值**参数个数 = 第一组签名参数中非类型参数的个数
                        // （`(T: Type)` 这类类型参数编译期擦除，不占运行时参数位）。
                        let own_param_count = match type_annotation.as_ref() {
                            Some(Type::Fn { params, .. }) => params
                                .iter()
                                // 与 reshape 前口径一致：只有带类型的形参参与计数
                                .filter(|p| p.ty.is_some())
                                .filter(|p| !is_type_param_annotation(p.ty.as_ref()))
                                .count(),
                            _ => 0,
                        };
                        let own_params: Vec<Param> = extracted_params
                            .iter()
                            .filter(|p| !is_type_param_annotation(p.ty.as_ref()))
                            .take(own_param_count)
                            .cloned()
                            .collect();
                        // 首组**全是类型参数**（`(T: Type, E: Type) -> ((self: &R) -> Bool)`）：
                        // 类型位擦除后本层不占运行时参数位，paren 层就是值层——
                        // 作者写的 lambda 本身即值级函数，不做「返回闭包」包装。
                        // 运行时闭包形态（`make_adder: (a: Int) -> ((b: Int) -> Int)`）
                        // 首组含值参数，仍走下面的包装。
                        let first_group_has_type_params = match type_annotation.as_ref() {
                            Some(Type::Fn { params, .. }) => params
                                .iter()
                                .any(|p| is_type_param_annotation(p.ty.as_ref())),
                            _ => false,
                        };
                        // 被返回值的参数组 = 声明返回位内层 Fn 的值参数个数
                        //（`(x: Int) -> Int` 的 `(x: Int)` 组；类型参数位擦除不计）。
                        // 内层不是 Fn（如 `-> (Int)`、`-> (&Float, &Point)`）时为
                        // None——个数判据不适用，保持既有行为。
                        let inner_fn_value_param_count = match type_annotation.as_ref() {
                            Some(Type::Fn { return_type, .. }) => {
                                let inner = match return_type.as_ref() {
                                    Type::Paren(t) => Some(t.as_ref()),
                                    Type::NamedParen { inner, .. } => Some(inner.as_ref()),
                                    _ => None,
                                };
                                inner.and_then(|t| match t {
                                    Type::Fn { params, .. } => Some(
                                        params
                                            .iter()
                                            .filter(|t| !is_type_param_annotation(Some(t)))
                                            .count(),
                                    ),
                                    _ => None,
                                })
                            }
                            _ => None,
                        };
                        // 作者 lambda 的参数组与内层 Fn 参数组**不对齐**时，作者写
                        // 的 lambda 就是外层函数本体——体里的 return / 尾表达式
                        // 供给被返回的函数值，不再包装。包装会把它变成「被返回的
                        // 值」，凭空多出一层与声明不符的函数形态（issue #501 形态
                        // B：`() => { return inc }` 被包成 `() => () => { return inc }`，
                        // g()(1) 的运行时形态与 E1012 的 found 口径同时错位）。
                        // 对齐时（含内层非 Fn）保持既有包装：作者 lambda 即被返回
                        // 的函数值（`mk: () -> ((a: Int) -> Int) = (a) => a + 1`、
                        // `mk_block: () -> (() -> Int) = { () => 7 }`、运行时闭包
                        // `make_adder: (a: Int) -> ((b: Int) -> Int) = (b) => a + b`）。
                        let author_lambda_is_outer = matches!(
                            inner_fn_value_param_count,
                            Some(n) if n != lambda_params.len()
                        );
                        if !author_lambda_is_outer
                            && (!own_params.is_empty() || !first_group_has_type_params)
                        {
                            let inner = Expr::Lambda {
                                params: merged,
                                body: body.clone(),
                                span,
                            };
                            // 合成体语句：不经过 parse_statement，就地发放身份
                            let plan_id = state.alloc_plan_id();
                            let value = Expr::Lambda {
                                params: own_params,
                                body: Box::new(Block {
                                    stmts: vec![Stmt {
                                        kind: StmtKind::Expr(Box::new(inner)),
                                        span,
                                        plan_id,
                                    }],
                                    span,
                                }),
                                span,
                            };
                            state.skip(&TokenKind::Semicolon);
                            return Some(Stmt {
                                kind: StmtKind::Assign {
                                    target: Box::new(target),
                                    type_annotation,
                                    value: Some(Box::new(value)),
                                    is_mut,
                                    span,
                                },
                                span,
                                plan_id: PlanId::UNPLANNED,
                            });
                        }
                        // 首组全为类型参数：落穿到标准 Lambda value 路径
                    }

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
                            value: Some(Box::new(value)),
                            is_mut,
                            span,
                        },
                        span,
                        plan_id: PlanId::UNPLANNED,
                    });
                }
                Some(expr) => {
                    // 直接表达式: name: (a: Int, b: Int) -> Int = a + b
                    state.skip(&TokenKind::Semicolon);

                    // 值参数判定。
                    //
                    // RFC-027a：`name_used_as_type` 会把「类型应用实参位」也算作
                    // 类型引用，于是 `-> Terminates(b)` 使 `b` 被当作 const 泛型参数
                    // 从 lambda params 中剔除（实测退化为 `[a]`，函数体引用 `b` 报
                    // E3006）。此处排除谓词应用。
                    //
                    // 判据的两个来源都不是 parser 自己的名字表：
                    // ① 注入探针（`state.probe()`）——内置谓词与环境已知谓词；
                    // ② 本遍已解析的声明累积（`declare_predicate`）。
                    // `ParserState::is_predicate_app` 把两者合成一个问答。
                    let value_params: Vec<Param> = extracted_params
                        .iter()
                        .filter(|p| {
                            if is_type_param_annotation(p.ty.as_ref()) {
                                return false;
                            }
                            // RFC-011 §5.2：约束形参（`T: Add`——标注名为运算符
                            // 接口/约束名，非具体类型）是类型位，不占运行时参数。
                            // 是不是约束名由注入探针判定（L2 不再持有该名字表）。
                            if let Some(Type::Name { name: n, .. }) = &p.ty {
                                if state.probe().is_constraint(n) {
                                    return false;
                                }
                            }
                            let used_as_const = type_annotation.as_ref().is_some_and(|ann| {
                                name_used_as_type(&p.name, ann, &|n| {
                                    state.kind_of(n) == NameKind::Predicate
                                })
                            });
                            !used_as_const
                        })
                        .cloned()
                        .collect();

                    // 构建 Lambda value（参数从签名提取，body 是表达式）
                    // 合成体语句：不经过 parse_statement，就地发放身份
                    let plan_id = state.alloc_plan_id();
                    let body_block = if let Expr::Block(block) = &expr {
                        block.clone()
                    } else {
                        Block {
                            stmts: vec![Stmt {
                                kind: StmtKind::Expr(Box::new(expr.clone())),
                                span: state.span(),
                                plan_id,
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
                            value: Some(Box::new(value)),
                            is_mut,
                            span,
                        },
                        span,
                        plan_id: PlanId::UNPLANNED,
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
                                    type_annotation: type_annotation.clone(),
                                    definition: Type::MetaType {
                                        name_span: Span::dummy(),
                                        args: Vec::new(),
                                    },
                                },
                                span,
                                plan_id: PlanId::UNPLANNED,
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
                        type_annotation: type_annotation.clone(),
                        definition,
                    },
                    span,
                    plan_id: PlanId::UNPLANNED,
                });
            }
        }

        // Case 3: 普通变量初始化 (with or without type annotation)
        let initializer = match state.parse_expression(BP_LOWEST) {
            Some(expr) => expr,
            None => {
                state.error(parse_msg(
                    state,
                    "Expected expression after '='".to_string(),
                ));
                return None;
            }
        };

        state.skip(&TokenKind::Semicolon);

        // x = { ... } → Block 作为 value，L3 的类型检查识别为函数定义
        return Some(Stmt {
            kind: StmtKind::Assign {
                target: Box::new(target),
                type_annotation,
                value: Some(Box::new(initializer)),
                is_mut,
                span,
            },
            span,
            plan_id: PlanId::UNPLANNED,
        });
    }

    // 语法错误：`x: Int` 无初值
    //
    // spec §3.2 `LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr`
    // 要求 `= Expr`——纯注解声明不合文法。旧实现把它当「纯声明」放行，
    // 但它没有值也没有可用的类型（仅用于延迟初始化，而本语言无此语义）。
    state.error(
        ErrorCodeDefinition::invalid_syntax(
            "declaration requires an initializer — write `x: Int = value`",
        )
        .at(span)
        .build(),
    );
    None
}

/// Parse generic parameters with constraints: `[T: Clone]` or `[N: Int]`
/// Supports:
/// - Type parameters: `[T]` or `[T: Clone]`
/// - Const parameters: `[N: Int]` - const generic with type annotation
/// - Platform parameter: `[P: X86_64]` - RFC-011 platform specialization
fn parse_type_definition(state: &mut ParserState<'_>) -> Option<Type> {
    let first_type = parse_type_annotation_required(state)?;

    // 检查是否有 | 符号（不允许使用不带花括号的枚举语法）
    if state.at(&TokenKind::Pipe) {
        state.error(parse_msg(state,
            "Enum variants must use brace syntax: `{ red | green | blue }` instead of `red | green | blue`".to_string(),
        ));
        return None;
    }

    Some(first_type)
}
/// Parse variable declaration after the leading `mut`, if any.
/// This is the entry point when `:` is detected after an identifier.
fn parse_var_stmt_with_mut(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    let is_mut = state.skip(&TokenKind::KwMut);

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

    parse_assign_after_target(state, target, is_mut, span)
}

pub fn parse_identifier_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // 首先检测并拒绝旧语法: identifier( 后跟类型参数和 ->
    if let Some(TokenKind::LParen) = state.peek().map(|t| &t.kind) {
        if is_old_function_syntax(state) {
            state.error(parse_msg(
                state,
                "旧语法已弃用。请使用新语法: `name: (Types) -> ReturnType = (params) => body`"
                    .to_string(),
            ));
            skip_old_function_syntax(state);
            return None;
        }
    }

    // 检测 mut 关键字
    let is_mut = state.skip(&TokenKind::KwMut);

    // ── 解析 target 表达式 ──
    // 使用 BP_ASSIGN + 1 在 `=` 处停止，但 `.` 和 `[` 是 BP_CALL=9 所以会被解析
    let target = match state.parse_expression(BP_ASSIGN + 1) {
        Some(expr) => expr,
        None => {
            // 解析失败，可能是旧语法残留
            if is_mut {
                state.error(parse_msg(
                    state,
                    "Expected expression after mut".to_string(),
                ));
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
                plan_id: PlanId::UNPLANNED,
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
                plan_id: PlanId::UNPLANNED,
            });
        }
        state.bump(); // consume `=`

        let rhs = match state.parse_expression(BP_LOWEST) {
            Some(expr) => expr,
            None => {
                state.error(parse_msg(
                    state,
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
            plan_id: PlanId::UNPLANNED,
        });
    }

    // ── `:` 类型标注 ──
    if state.at(&TokenKind::Colon) {
        return parse_assign_after_target(state, target, is_mut, span);
    }

    // ── `=` 初始化 ──
    if state.at(&TokenKind::Eq) {
        state.bump(); // consume '='

        let value = match state.parse_expression(BP_LOWEST) {
            Some(expr) => expr,
            None => {
                state.error(parse_msg(
                    state,
                    "Expected expression after '='".to_string(),
                ));
                return None;
            }
        };

        state.skip(&TokenKind::Semicolon);

        return Some(Stmt {
            kind: StmtKind::Assign {
                target: Box::new(target),
                type_annotation: None,
                value: Some(Box::new(value)),
                is_mut,
                span,
            },
            span,
            plan_id: PlanId::UNPLANNED,
        });
    }

    // ── 否则: 表达式语句 ──
    state.skip(&TokenKind::Semicolon);

    if is_mut {
        // mut 后面不是赋值或类型标注，报错
        state.error(parse_msg(
            state,
            "Expected ':' or '=' after mut identifier".to_string(),
        ));
        return None;
    }

    Some(Stmt {
        kind: StmtKind::Expr(Box::new(target)),
        span,
        plan_id: PlanId::UNPLANNED,
    })
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

    let rhs = match state.parse_expression(BP_LOWEST) {
        Some(expr) => expr,
        None => {
            state.error(parse_msg(
                state,
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
        plan_id: PlanId::UNPLANNED,
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
            plan_id: PlanId::UNPLANNED,
        })
    } else {
        // 表达式解析失败：报错而非静默吞掉 token（#258 审计：块内 `, 5` 等
        // token 曾被 bump 静默吃掉，整条链无任何诊断）
        let found = state
            .current()
            .map(|t| format!("{:?}", t.kind))
            .unwrap_or_else(|| "EOF".to_string());
        state.error(
            ErrorCodeDefinition::unexpected_token(&found)
                .at(state.span())
                .build(),
        );
        state.bump();
        None
    }
}
