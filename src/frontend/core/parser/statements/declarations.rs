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

use crate::ast::*;
use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::parse_msg;
use crate::frontend::core::parser::{NameKind, ParserState, BP_ASSIGN, BP_LOWEST};
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::util::span::Span;

// Import from sibling modules
use super::types::{parse_fn_type_with_names, parse_type_annotation_required};

/// 该形参标注是否为**类型位**（编译期擦除，不占运行期形参位）：具体元类型
/// `Type`（`(T: Type)`）与**类型级函数**（`(F: (T: Type) -> Type)`——构造器变量，
/// RFC-010 元类型层级 Type1/Type2+）。
///
/// 判据与 `ast::classify_generic_params` / `lower/curry.rs` 的 `split_curry` /
/// `is_type_param_ann` **必须同源**：此前此处缺类型级函数分支，同一份声明在
/// parser 眼里 F 是值参数、在类型检查/IR 眼里是类型参数，于是「声明占几个运行期
/// 形参位」两处答案不同（WBS 6.8.2a 的 E1112 元数对照当场把这种声明误判成
/// 「实现形参超出声明」）。判据复用 `ast::is_type_level_fn` 单点。
fn is_type_param_annotation(ty: Option<&Type>) -> bool {
    match ty {
        Some(Type::MetaType { .. }) => true,
        Some(Type::Name { name, .. }) => name == "Type",
        Some(Type::Fn {
            params,
            return_type,
        }) => crate::ast::is_type_level_fn(params, return_type),
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
    state.skip_balanced_parens();

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

/// 声明形式的总派发器：`target` 已被调用者解析成表达式，本函数只按语义顺序
/// 把声明各部分交给各段函数，自身不做任何一种形态的解析（WBS 8.7.1）。
///
/// 顺序即语义：
/// 1. `reject_legacy_call_syntax` —— 拒绝已移除的 `name(...) -> ...` 旧语法；
/// 2. `resolve_annotation` —— `:` 之后到底是签名还是普通类型标注；
/// 3. `check_declaration_legality` —— 纯注解声明后面的 token 是否合法；
/// 4. `register_type_returning_predicate` —— 登记「返回 Type」的谓词名；
/// 5. 无 `=` → `report_missing_initializer`（声明缺初值）；
/// 6. `try_parse_type_body_assignment` —— 返回 Type 的声明先试类型体；
/// 7. 签名声明 → `parse_fn_signature_value`（内部再分「作者写了实现头」与
///    「只写表达式体」两种 value 构造）；
/// 8. 类型定义 → `parse_meta_type_definition`，其余 → `parse_plain_initializer`。
fn parse_assign_after_target(
    state: &mut ParserState<'_>,
    target: Expr,
    is_mut: bool,
    span: Span,
) -> Option<Stmt> {
    // RFC-010: 检测并拒绝旧语法: name(Type, ...) -> Ret = ...
    if reject_legacy_call_syntax(state) {
        return None;
    }

    // `:` 之后的标注（第 2 段）：`(annotation, is_fn_signature)`。
    let (annotation, is_fn_signature) = resolve_annotation(state)?;

    if !check_declaration_legality(state, annotation.as_ref()) {
        return None;
    }

    register_type_returning_predicate(state, annotation.as_ref(), &target);

    // ── 可选初始化 ──
    if !state.skip(&TokenKind::Eq) {
        report_missing_initializer(state, span);
        return None;
    }

    // RFC-010: 返回 Type 时，先尝试类型体解析（字段/枚举）
    // 失败则 fall through 到表达式分支（精化约束）
    if let Some(stmt) = try_parse_type_body_assignment(state, &target, annotation.as_ref(), span) {
        return Some(stmt);
    }

    // Case 1: RFC-010 函数签名（注解是 Type::Fn）
    if is_fn_signature {
        if let Some(stmt) =
            parse_fn_signature_value(state, &target, annotation.as_ref(), is_mut, span)
        {
            return Some(stmt);
        }
    }

    // Case 2: 类型定义 `Point: Type = { ... }`
    if matches!(annotation.as_ref(), Some(Type::MetaType { .. })) {
        if let Expr::Var(name, _) = &target {
            return parse_meta_type_definition(state, name, annotation, span);
        }
    }

    // Case 3: 普通变量初始化 (with or without type annotation)
    parse_plain_initializer(state, target, annotation, is_mut, span)
}

/// 标注消歧（第 2 段）：`:` 之后是 RFC-010 命名形参签名、普通类型标注，
/// 还是已移除的旧签名 `(Type, Type) -> Ret`。
///
/// 返回 `(annotation, is_fn_signature)`：前者是解析出的标注（`:` 缺席时为
/// `None`），后者标记是否走了命名形参签名——它决定 `=` 右侧按「函数声明」
/// 还是「普通初始化」解释。
///
/// 三层前瞻一处收口：`(` 内首 token 形态 → 像不像命名形参表 → 配平后
/// 是否跟 `->`；配平扫描统一走 `ParserState::skip_balanced_parens`。
/// 旧签名形态在此报「已不再支持」并返回 `None`。
fn resolve_annotation(state: &mut ParserState<'_>) -> Option<(Option<Type>, bool)> {
    let plain = |annotation: Type| (Some(annotation), false);

    if !state.at(&TokenKind::Colon) {
        return Some((None, false));
    }
    state.bump(); // consume ':'

    // RFC-010: name: (a: Int, b: Int) -> Ret = body (参数名在签名中)
    // 特征: 冒号后面跟着函数类型 (param: Type, ...) -> Ret
    if !state.at(&TokenKind::LParen) {
        return parse_type_annotation_required(state).map(plain);
    }

    // 第 1 层前瞻：`(` 内首 token 的形态决定「像不像命名形参表」。
    let looks_like_named_params = match state.peek().map(|t| &t.kind) {
        Some(TokenKind::RParen) | Some(TokenKind::KwMut) => true,
        Some(TokenKind::Identifier(_)) => matches!(
            state.peek_nth(2).map(|t| &t.kind),
            Some(TokenKind::Colon) | Some(TokenKind::Comma) | Some(TokenKind::RParen)
        ),
        _ => false,
    };

    // 第 2 层前瞻：括号配平后跟 `->`——两条同时成立才是 RFC-010 签名。
    let saved = state.save_position();
    let is_rfc010 = if looks_like_named_params {
        state.skip_balanced_parens();
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
        return Some((Some(type_annotation), true));
    }

    // 第 3 层前瞻：`(Type, Type) -> Ret` 是已移除的旧签名形态。
    let saved_check = state.save_position();
    state.skip_balanced_parens();
    let is_old_fn_syntax = state.at(&TokenKind::Arrow);
    state.restore_position(saved_check);

    if is_old_fn_syntax {
        state.error(parse_msg(
            state,
            "Old function syntax '(Type, Type) -> Ret' is no longer supported. \
                 Use RFC-010 syntax with named parameters: '(param: Type, ...) -> Ret'. \
                 Example: 'add: (a: Int, b: Int) -> Int = a + b'"
                .to_string(),
        ));
        return None;
    }

    parse_type_annotation_required(state).map(plain)
}

/// 声明合法性（第 3 段）：纯注解声明后只拦明确无效的后续 token。
///
/// `name: Type` 后无 `=` 时，`(` / `=>` / `,` 判非法；
/// Identifier 不拦截——它可能是下一个顶层语句（#168: 纯函数声明）。
fn check_declaration_legality(
    state: &mut ParserState<'_>,
    annotation: Option<&Type>,
) -> bool {
    if annotation.is_none() {
        return true;
    }
    let is_invalid = state.at(&TokenKind::LParen)
        || state.at(&TokenKind::FatArrow)
        || state.at(&TokenKind::Comma);
    if !is_invalid || state.at(&TokenKind::Eq) {
        return true;
    }
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
    false
}

/// 语义副作用（第 4 段）：RFC-027 把「返回 Type」的声明登记为**谓词**，供后续
/// 标注里的谓词应用判定。
///
/// 判据与类型体解析同源（`Type::Fn` 返回 `MetaType` / `Type`）。
fn register_type_returning_predicate(
    state: &mut ParserState<'_>,
    annotation: Option<&Type>,
    target: &Expr,
) {
    if let (Some(Type::Fn { return_type, .. }), Expr::Var(name, _)) = (annotation, target) {
        let returns_type = matches!(return_type.as_ref(), Type::MetaType { .. })
            || matches!(return_type.as_ref(), Type::Name { name: n, .. } if n == "Type");
        if returns_type {
            state.declare_predicate(name);
        }
    }
}

/// 声明缺初值（第 5 段）。
///
/// spec §3.2 `LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr`
/// 要求 `= Expr`——纯注解声明不合文法。旧实现把它当「纯声明」放行，
/// 但它没有值也没有可用的类型（仅用于延迟初始化，而本语言无此语义）。
fn report_missing_initializer(
    state: &mut ParserState<'_>,
    span: Span,
) {
    state.error(
        ErrorCodeDefinition::invalid_syntax(
            "declaration requires an initializer — write `x: Int = value`",
        )
        .at(span)
        .build(),
    );
}

/// 返回 `Type` 的声明先试类型体（第 6 段）。
///
/// 只有 `Struct` 才是类型体；`ConstExpr` 等是表达式，走后面的初始化分支。
/// 解析失败（如 `{ x > 0 }`）或形态不是 Struct 时**还原位置**返回 `None`，
/// 由调用方继续按表达式分支处理（诊断已在解析中发射，不回滚）。
fn try_parse_type_body_assignment(
    state: &mut ParserState<'_>,
    target: &Expr,
    annotation: Option<&Type>,
    span: Span,
) -> Option<Stmt> {
    let (Some(Type::Fn { return_type, .. }), Expr::Var(name, _)) = (annotation, target) else {
        return None;
    };
    if !matches!(return_type.as_ref(), Type::MetaType { .. }) {
        return None;
    }

    let saved = state.save_position();
    if let Some(definition) = parse_type_definition(state) {
        // 只有 Struct 才是类型体；ConstExpr 等是表达式，走 Assign
        if matches!(&definition, Type::Struct { .. }) {
            state.skip(&TokenKind::Semicolon);
            return Some(type_definition_stmt(
                name,
                annotation.cloned(),
                definition,
                span,
            ));
        }
    }
    // 类型体解析失败（如 { x > 0 }）→ 回退，走表达式分支
    state.restore_position(saved);
    None
}

/// 函数签名声明的初值构造（第 7 段入口）：只看 `=` 右侧是什么形态。
///
/// 作者写了实现头（lambda）→ 实现头即 value；只写了表达式 / 块 → 形参表从签名
/// **合成**；解析失败 → 还原位置返回 `None`，调用方继续按类型定义 /
/// 普通初始化处理（既有行为：会再解析一次）。
fn parse_fn_signature_value(
    state: &mut ParserState<'_>,
    target: &Expr,
    annotation: Option<&Type>,
    is_mut: bool,
    span: Span,
) -> Option<Stmt> {
    let saved = state.save_position();
    match state.parse_expression(BP_LOWEST) {
        Some(Expr::Lambda {
            params: lambda_params,
            body,
            ..
        }) => Some(build_author_lambda_value(
            state,
            target,
            annotation,
            lambda_params,
            body,
            is_mut,
            span,
        )),
        Some(expr) => Some(build_synthesized_lambda_value(
            state, target, annotation, expr, is_mut, span,
        )),
        None => {
            state.restore_position(saved);
            None
        }
    }
}

/// 作者写了实现头时的 value 构造（第 7 段分支 A）：值形态始终是「作者写的
/// 实现头 + 作者写的 body」，只有括号返回位才试「返回闭包」包装。
fn build_author_lambda_value(
    state: &mut ParserState<'_>,
    target: &Expr,
    annotation: Option<&Type>,
    lambda_params: Vec<Param>,
    body: Box<Block>,
    is_mut: bool,
    span: Span,
) -> Stmt {
    let impl_head_params: Vec<Param> = lambda_params.clone();

    // RFC-004 括号语义：返回位置是 `Paren` ⇔ 链条在此终止。
    // 作者写的 lambda（`(a) => a + 1`）就是**被返回的函数值**，
    // 不是本函数的参数列表。故本函数只取第一组签名参数，
    // body 为“返回这个 lambda”。
    // 不加此判会让 `a` 被合并进外层，`f` 变成接受 `a` 而非返回闭包。
    // RFC-027 §3：具名括号（`-> (r: P(r))`）与 `Paren` 同款——
    // 括号同样声明「这是完整类型（值）」，链条到此终止。
    let paren_return = matches!(
        annotation,
        Some(Type::Fn { return_type, .. }) if return_type.is_paren_complete()
    );
    if paren_return {
        if let Some(stmt) = try_wrap_paren_return_lambda(
            state,
            target,
            annotation,
            &lambda_params,
            &body,
            is_mut,
            span,
        ) {
            return stmt;
        }
        // 首组全为类型参数：落穿到标准 Lambda value 路径
    }

    // 构建 Lambda value（形态 = 作者写的实现头 + 作者写的 body）
    let value = Expr::Lambda {
        params: impl_head_params,
        body: body.clone(),
        span,
    };
    state.skip(&TokenKind::Semicolon);
    assign_stmt(target, annotation.cloned(), value, is_mut, span)
}

/// 括号返回位上「作者写的 lambda 是不是被返回的函数值」的判定（第 7 段判据）。
///
/// 是 → 返回外层函数自己的**值**形参表（包装时用它构造外层 value）；否 → `None`
///（作者 lambda 即外层函数体，调用方落穿到标准 Lambda value 路径）。
/// 判据只读声明侧事实（签名首组、返回位内层组），不看 body。
fn returned_value_own_params(
    annotation: Option<&Type>,
    lambda_params: &[Param],
) -> Option<Vec<Param>> {
    // 本函数自己的**值**参数个数 = 第一组签名参数中非类型参数的个数
    // （`(T: Type)` 这类类型参数编译期擦除，不占运行时参数位）。
    let own_param_count = match annotation {
        Some(Type::Fn { params, .. }) => params
            .iter()
            // 与 reshape 前口径一致：只有带类型的形参参与计数
            .filter(|p| p.ty.is_some())
            .filter(|p| !is_type_param_annotation(p.ty.as_ref()))
            .count(),
        _ => 0,
    };
    let extracted_params = annotation
        .map(crate::ast::Type::fn_params_flat)
        .unwrap_or_default();
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
    let first_group_has_type_params = match annotation {
        Some(Type::Fn { params, .. }) => params
            .iter()
            .any(|p| is_type_param_annotation(p.ty.as_ref())),
        _ => false,
    };
    // 被返回值的参数组 = 声明返回位内层 Fn 的值参数个数
    //（`(x: Int) -> Int` 的 `(x: Int)` 组；类型参数位擦除不计）。
    // 内层不是 Fn（如 `-> (Int)`、`-> (&Float, &Point)`）时为
    // None——个数判据不适用，保持既有行为。
    let inner_fn_value_param_count = match annotation {
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
                        .filter(|p| !is_type_param_annotation(p.ty.as_ref()))
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
    let author_lambda_is_outer =
        matches!(inner_fn_value_param_count, Some(n) if n != lambda_params.len());
    if author_lambda_is_outer || (own_params.is_empty() && first_group_has_type_params) {
        return None;
    }
    Some(own_params)
}

/// 括号返回位上的「返回闭包」包装（第 7 段分支 A′）：判据见
/// `returned_value_own_params`，本函数只做构造。
fn try_wrap_paren_return_lambda(
    state: &mut ParserState<'_>,
    target: &Expr,
    annotation: Option<&Type>,
    lambda_params: &[Param],
    body: &Block,
    is_mut: bool,
    span: Span,
) -> Option<Stmt> {
    let own_params = returned_value_own_params(annotation, lambda_params)?;

    let inner = Expr::Lambda {
        params: lambda_params.to_vec(),
        body: Box::new(body.clone()),
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
    Some(assign_stmt(
        target,
        annotation.cloned(),
        value,
        is_mut,
        span,
    ))
}

/// 从签名**合成**实现头的形参表（第 7 段判据）。
///
/// 块体 / 表达式体没有作者写的形参表：这里按声明合成它。
///
/// 取哪一层、层内哪些是运行期值位，都由 `ast` 的单一实现回答
///（`Type::impl_head_runtime_params`，与 typecheck 的体内绑定同一个
/// helper——WBS 6.8.2 / D54 F1 收口，6.8.3 F1d）。parser 不再自己写
/// 过滤：层选择（括号返回位 ⇒ 首组 / 其余 ⇒ 最内层组）、类型位、
/// 多约束并集、被当编译期常量用的名字都在那里判；本处只注入「是不是
/// 约束名」这一个探针问答（L2 没有约束名字表）。
fn synthesize_signature_params(
    state: &ParserState<'_>,
    annotation: Option<&Type>,
) -> Vec<Param> {
    annotation
        .map(|ann| {
            // 两个问答的可用性不同，别把「不知道」当「是」（WBS 6.8.4-1c F-2）：
            // - `is_const_param_type`：探针给出**真答案**（权威表在 L3 的
            //   `sema::probe::CONST_PARAM_TYPES`）；
            // - `is_type_name`：L2 拿不到「已声明类型名」表，`kind_of` 把
            //   「用户声明类型」与「无法定种」都答成 `UserType`，据此判「是类型」
            //   会把每个候选都 skip 掉 ⇒ 一个编译期值参数都剔不掉 ⇒ 合成头里
            //   仍带着它们 ⇒ E1112 误报。故此处取保守可靠值 `false`：
            //   分类只在**已声明类型与形参名同形**时可能过度，而那种形态的
            //   权威判定在 typecheck（它有真命名空间，E1011/E1112 都按它算）。
            //   不在 L2 复制 L3 名表（禁令一/二）：这里只表态「不提供该知识」。
            let cls = crate::ast::ParamClassifier {
                is_const_param_type: &|n| state.probe().is_const_param_type(n),
                is_type_name: &|_| false,
                is_predicate: &|n| state.kind_of(n) == NameKind::Predicate,
            };
            ann.impl_head_runtime_params(&|n| state.probe().is_constraint(n), &cls)
        })
        .unwrap_or_default()
}

/// 只写表达式 / 块体时的 value 构造（第 7 段分支 B）：形参表由
/// `synthesize_signature_params` 从签名合成，body 就是作者写的表达式（块体原样）。
fn build_synthesized_lambda_value(
    state: &mut ParserState<'_>,
    target: &Expr,
    annotation: Option<&Type>,
    expr: Expr,
    is_mut: bool,
    span: Span,
) -> Stmt {
    state.skip(&TokenKind::Semicolon);

    let value_params: Vec<Param> = synthesize_signature_params(state, annotation);

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
    assign_stmt(target, annotation.cloned(), value, is_mut, span)
}

/// 类型定义 `Point: Type = { ... }`（第 8 段之一）。
///
/// 含 `Type: Type = Type` 这条既有彩蛋；`parse_type_definition` 失败时
/// 整条声明按 `None` 收口（既有行为：不落穿到普通初始化）。
fn parse_meta_type_definition(
    state: &mut ParserState<'_>,
    name: &str,
    annotation: Option<Type>,
    span: Span,
) -> Option<Stmt> {
    // Easter egg: Type: Type = Type
    if name == "Type" {
        let saved_easter = state.save_position();
        if let Some(TokenKind::Identifier(val)) = state.current().map(|t| &t.kind) {
            if val == "Type" {
                state.bump();
                state.skip(&TokenKind::Semicolon);
                return Some(type_definition_stmt(
                    "Type",
                    annotation,
                    Type::MetaType {
                        name_span: Span::dummy(),
                        args: Vec::new(),
                    },
                    span,
                ));
            }
        }
        state.restore_position(saved_easter);
    }

    let definition = parse_type_definition(state)?;
    state.skip(&TokenKind::Semicolon);
    Some(type_definition_stmt(name, annotation, definition, span))
}

/// 普通变量初始化（第 8 段之一）：`[mut] name[: Type] = expr`。
///
/// 块体初值原样作为 value（L3 的类型检查识别为函数定义）。
fn parse_plain_initializer(
    state: &mut ParserState<'_>,
    target: Expr,
    annotation: Option<Type>,
    is_mut: bool,
    span: Span,
) -> Option<Stmt> {
    let initializer = match state.parse_expression(BP_LOWEST) {
        Some(expr) => expr,
        None => {
            // 深度闸拒绝（E0019 已发射）时跳过恢复诊断（issue #540 恰 1 条判据）
            if !state.is_depth_rejected() {
                state.error(parse_msg(
                    state,
                    "Expected expression after '='".to_string(),
                ));
            }
            return None;
        }
    };

    state.skip(&TokenKind::Semicolon);

    // x = { ... } → Block 作为 value，L3 的类型检查识别为函数定义
    Some(assign_stmt(&target, annotation, initializer, is_mut, span))
}

/// 拒绝已移除的旧调用语法 `name(Type, ...) -> Ret = ...`（第 1 段）。
///
/// 旧语法特征：标识符后面直接跟着 `(`，且配平后紧跟 `->`。命中即发射
/// 「旧函数语法已不再支持」并返回 `true`（调用方按 `None` 收口）。
fn reject_legacy_call_syntax(state: &mut ParserState<'_>) -> bool {
    if !state.at(&TokenKind::LParen) {
        return false;
    }
    let saved = state.save_position();
    state.skip_balanced_parens();
    let is_old_syntax = state.at(&TokenKind::Arrow);
    state.restore_position(saved);
    if !is_old_syntax {
        return false;
    }
    state.error(parse_msg(
        state,
        "旧函数语法已不再支持，请使用新语法: name: (param: Type, ...) -> Ret = body".to_string(),
    ));
    true
}

/// 构造一条赋值语句（本文件五处同形构造的唯一出口）。
fn assign_stmt(
    target: &Expr,
    type_annotation: Option<Type>,
    value: Expr,
    is_mut: bool,
    span: Span,
) -> Stmt {
    Stmt {
        kind: StmtKind::Assign {
            target: Box::new(target.clone()),
            type_annotation,
            value: Some(Box::new(value)),
            is_mut,
            span,
        },
        span,
        plan_id: PlanId::UNPLANNED,
    }
}

/// 构造一条类型定义语句（本文件三处同形构造的唯一出口）。
fn type_definition_stmt(
    name: &str,
    type_annotation: Option<Type>,
    definition: Type,
    span: Span,
) -> Stmt {
    Stmt {
        kind: StmtKind::TypeDefinition {
            name: name.to_string(),
            type_annotation,
            definition,
        },
        span,
        plan_id: PlanId::UNPLANNED,
    }
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
                // 深度闸拒绝（E0019 已发射）时跳过恢复诊断（issue #540 恰 1 条判据）
                if !state.is_depth_rejected() {
                    state.error(parse_msg(
                        state,
                        "Expected expression after mut".to_string(),
                    ));
                }
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

        // syntax.md §3.2.1：解构赋值只保留括号式 `(a, b) = e`；裸形态取消。
        // 报一次可操作诊断后仍按解构继续解析（避免级联），诊断使编译失败。
        state.error(parse_msg(
            state,
            "解构赋值必须写成 `(a, b) = e`：裸形态 `a, b = e` 已取消",
        ));

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
                // 深度闸拒绝（E0019 已发射）时跳过恢复诊断（issue #540 恰 1 条判据）
                if !state.is_depth_rejected() {
                    state.error(parse_msg(
                        state,
                        "Expected expression after '=' in tuple destructuring".to_string(),
                    ));
                }
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
                // 深度闸拒绝（E0019 已发射）时跳过恢复诊断（issue #540 恰 1 条判据）
                if !state.is_depth_rejected() {
                    state.error(parse_msg(
                        state,
                        "Expected expression after '='".to_string(),
                    ));
                }
                return None;
            }
        };

        state.skip(&TokenKind::Semicolon);

        return Some(assign_stmt(&target, None, value, is_mut, span));
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
            // 深度闸拒绝（E0019 已发射）时跳过恢复诊断（issue #540 恰 1 条判据）
            if !state.is_depth_rejected() {
                state.error(parse_msg(
                    state,
                    "Expected expression after '=' in tuple destructuring".to_string(),
                ));
            }
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
