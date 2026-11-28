//! Type annotation parsing
//!
//! Implements parsing for:
//! - Type annotations: `name: Type`
//! - Function types: `(Params) -> ReturnType`
//! - Struct types: `{ field: Type }`
//! - Tuple types: `(T, U, V)`
//! - Named struct types: `Name(x: Type, y: Type)`
//! - Constructor types: `Name(Type1, Type2)` — the ONLY generic application syntax
//! - Meta types: `Type`

use crate::ast::StructField;
use crate::ast::*;
use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::parse_msg;
use crate::frontend::core::parser::{ParserState, BP_LOWEST};
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::util::span::Span;

impl ParserState<'_> {
    /// Parse a type annotation
    pub fn parse_type_annotation(&mut self) -> Option<Type> {
        parse_type_annotation(self)
    }

    /// 严格版类型标注（`:`/`as` 之后类型必需的位置专用）
    pub fn parse_type_annotation_required(&mut self) -> Option<Type> {
        parse_type_annotation_required(self)
    }

    /// 解析 `as` 的目标类型（D1：目标类型不吞其后值级中缀，故禁止顶层常量续接）。
    pub fn parse_cast_target_type(&mut self) -> Option<Type> {
        parse_cast_target_type(self)
    }
}

/// 严格变体：仅用于调用方已消费 `:` 或 `as` 的位置（类型必需）。
/// 解析失败时报 E0010 并消费肇事 token，避免非法 token 掉进宽容版的
/// `_ => None` 后静默吞掉整个标注/绑定（#250 残余缺口：当时只修了 `?`）。
/// 探测型调用点（while-let / if-let / try_parse_typed_param_list）继续用宽容版。
pub fn parse_type_annotation_required(state: &mut ParserState<'_>) -> Option<Type> {
    if let Some(ty) = parse_type_annotation(state) {
        return Some(ty);
    }
    // 深度闸拒绝（issue #540 修复轮）：E0019 已在拒绝点发射，此处再报
    // E0010 是同因双诊断——保持「恰 1 条正式诊断」，与 expect() 的压制
    // 语义同构。标志在每条语句开始时复位，不影响后续语句的正常恢复。
    if state.is_depth_rejected() {
        return None;
    }
    let span = state.span();
    let found = state
        .current()
        .map(|t| format!("{:?}", t.kind))
        .unwrap_or_else(|| "EOF".to_string());
    state.error(
        ErrorCodeDefinition::expected_token("a type", &found)
            .at(span)
            .build(),
    );
    state.bump();
    None
}

/// Parse type annotation
///
/// 解析递归深度闸的**类型汇点**（issue #540 修复轮）：Ptr/Ref/泛型构造/
/// 具名 struct/函数类型/元组/类型体各臂的自递归与互递归都回入本函数，
/// 全部经同一 enter/exit 计数（与语句/表达式两汇点同机制、同一
/// [`MAX_PARSE_DEPTH`](crate::frontend::core::parser::parser_state::MAX_PARSE_DEPTH)
/// 常数，不另设第二套计数）。实测登记（1MB 参照线程，debug 构建）：
/// 类型递归崩溃起点 struct 386 / generic 595 / Ptr 928 / Ref 930 / Fn 930
/// 层——全部 ≤ 闸值 1024，按「类型闸值 ≥ 实测崩溃起点」判据并入单闸；
/// 闸上（≥1025 层）发射 E0019 并以 `None` 逐层弹出。
pub fn parse_type_annotation(state: &mut ParserState<'_>) -> Option<Type> {
    parse_type_annotation_with(state, true)
}

/// 类型转换（`as`）的目标类型：**禁止**顶层常量表达式续接。
///
/// D1（09a §P8 8.8 六）：类型位的标识符 / 字面量种子会经同一个 Pratt 中缀循环
/// 续接，于是「x as Int * y」的目标类型被读成「Int * y」。规范 §2.6 的
/// TypeCast ::= Expr 'as' TypeExpr 只吃一个类型，值级中缀属于外层表达式；
/// 类型级常量表达式（Array(Int, N + 1)、Assert(N > 0)）只出现在实参位，
/// 因此续接由「是否在实参位」这一个开关决定，而不是每个调用点各自判断。
pub fn parse_cast_target_type(state: &mut ParserState<'_>) -> Option<Type> {
    parse_type_annotation_with(state, false)
}

/// parse_type_annotation 的实际实现：allow_const_expr 决定顶层种子是否允许
/// 值级中缀续接（见上一条注释的 D1 说明）。
fn parse_type_annotation_with(
    state: &mut ParserState<'_>,
    allow_const_expr: bool,
) -> Option<Type> {
    if !state.enter_nested_parse() {
        return None;
    }
    let result = parse_type_annotation_nested(state, allow_const_expr);
    state.exit_nested_parse();
    result
}

/// [`parse_type_annotation`] 的闸内本体（原实现逐字保留）。
fn parse_type_annotation_nested(
    state: &mut ParserState<'_>,
    allow_const_expr: bool,
) -> Option<Type> {
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Star) => {
            // Raw pointer type: *T
            state.bump(); // consume '*'
            let inner_type = Box::new(parse_type_annotation_with(state, allow_const_expr)?);
            Some(Type::Ptr(inner_type))
        }
        Some(TokenKind::Ampersand) => {
            // &T — immutable reference type
            state.bump(); // consume &
            let inner = parse_type_annotation_with(state, allow_const_expr)?;
            let span = state.span();
            Some(Type::Ref {
                mutable: false,
                inner: Box::new(inner),
                span,
            })
        }
        Some(TokenKind::MutRef) => {
            // &mut T — mutable reference type
            state.bump(); // consume &mut
            let inner = parse_type_annotation_with(state, allow_const_expr)?;
            let span = state.span();
            Some(Type::Ref {
                mutable: true,
                inner: Box::new(inner),
                span,
            })
        }
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            let name_span = state.span();
            state.bump();
            // `Type[T]` and `Type<T>` are rejected.
            if name == "Type" {
                // Reject old Type[T] or Type<T> syntax
                if state.at(&TokenKind::LBracket) || state.at(&TokenKind::Lt) {
                    state.error(parse_msg(state,
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
                state.error(parse_msg(state, format!(
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
            // RFC-027 §14.5：类型位单层证据字段路径 `b.w`。V1 只允许 `ident.ident`；
            // 多级路径在这里也先按嵌套 `FieldAccess` 解析，由转换点拒绝（T03）。
            // 复用下方同一个 `Type::ConstExpr` 构造点（类型表门禁 T1 的构造点计数不含新站）。
            let mut field_path_seed: Option<Expr> = None;
            if state.at(&TokenKind::Dot)
                && matches!(
                    state.peek().map(|t| &t.kind),
                    Some(TokenKind::Identifier(_))
                )
            {
                let mut expr = Expr::Var(name.clone(), name_span);
                while state.at(&TokenKind::Dot) {
                    state.bump(); // '.'
                    let Some(TokenKind::Identifier(field)) = state.current().map(|t| &t.kind)
                    else {
                        break;
                    };
                    let field = field.clone();
                    state.bump();
                    // 字段名 span 不参与语义；基名 span 供诊断定位（与值级读取不同，值级另有发射点）
                    expr = Expr::FieldAccess {
                        expr: Box::new(expr),
                        field,
                        span: name_span,
                    };
                }
                field_path_seed = Some(expr);
            }
            // 后视检查：如果不是类型语法专用的 token，则作为表达式解析（blacklist）
            if field_path_seed.is_some()
                || (allow_const_expr && type_position_expr_continues(state))
            {
                let seed = field_path_seed.unwrap_or_else(|| Expr::Var(name.clone(), name_span));
                let expr = parse_type_position_const_expr(state, seed, allow_const_expr);
                return Some(Type::ConstExpr(Box::new(expr)));
            }
            Some(Type::Name {
                name,
                span: name_span,
            })
        }
        Some(TokenKind::LParen) => {
            // 形态分流（唯一判据 = 括号配平后是不是 `->`）：
            // 1. 一组形参 + 箭头 ⇒ 函数类型——与**声明签名**共用同一个产生式
            //    （`Param ::= Identifier ':' TypeExpr | TypeExpr`，RFC-039 D53）；
            // 2. 其余 ⇒ 元组 / 完整类型括号 / 返回位具名括号（RFC-004 / RFC-027 §3）。
            //
            // 此处已在类型递归闸内（本函数由 `parse_type_annotation_with` 计数），
            // 故直接进闸内本体，不二次计数。
            if at_fn_param_group(state) {
                // 形参位口径随所在类型位走：普通类型位用签名口径（类型必需、
                // 常量续接照常）；`as` 目标位（`allow_const_expr == false`，D1）
                // 用宽容 + 禁续接口径，否则 `x as (a: Int) -> Int * y` 的
                // `* y` 会被目标类型吞成返回位常量表达式。
                let mode = if allow_const_expr {
                    ParamTyMode::Signature
                } else {
                    ParamTyMode::SignatureNoConst
                };
                let (params, return_type) = parse_fn_type_with_names_nested(state, mode)?;
                return Some(Type::Fn {
                    params,
                    return_type,
                });
            }

            let saved = state.save_position();
            state.bump(); // consume '('

            // 非箭头形态仍复用**同一个**形参项产生式（探针口径）：多项元组
            // 丢弃名字（元组元素没有形参名），单项具名 = 返回位精化 binder。
            // 探针口径不消费 `mut`、不报类型诊断——失败一律回退元组解析。
            let mut params = match parse_param_items(state, ParamTyMode::Probe { allow_const_expr })
            {
                Some(params) if state.skip(&TokenKind::RParen) => params,
                _ => {
                    // 深度闸拒绝弹出期（issue #540 修复轮）：元组回退重试同样只
                    // 会以错位起点重扫残骸——直退，残骸交给语句层的 drain。
                    if state.is_depth_rejected() {
                        return None;
                    }
                    state.restore_position(saved);
                    return parse_tuple_type(state);
                }
            };

            // Not a function type, just a tuple / paren
            if params.len() == 1 {
                let param = params.pop().expect("len == 1");
                let named = param.is_named();
                let param_span = param.span;
                let inner = Box::new(param.ty?);
                if named {
                    // 单具名参数（`(r: IsPositive(r + 1))`）= RFC-027 §3 的返回位
                    // 具名精化：binder 名必须留存（其余语义与 `Paren` 同构）。
                    Some(Type::NamedParen {
                        param: param.name,
                        param_span,
                        inner,
                    })
                } else {
                    // 单元素括号 = 完整类型（RFC-004 柯里化语义）：
                    // 保留为 `Paren`，让 split_curry 知道此处链条终止。
                    // 此前直接剥掉，导致 `() -> ((a:Int)->Int)` 与
                    // `() -> (a:Int)->Int` 解析成同一个 AST（括号无意义）。
                    Some(Type::Paren(inner))
                }
            } else {
                // 元组元素没有形参名：名字槽丢弃，只留类型。
                Some(Type::Tuple(
                    params.into_iter().filter_map(|p| p.ty).collect(),
                ))
            }
        }
        Some(TokenKind::LBrace) => parse_struct_type(state),
        Some(TokenKind::LBracket) => {
            // RFC-010: `[T, U](params) -> Ret` syntax is removed.
            state.error(parse_msg(
                state,
                "Old generic function type syntax '[T, U](params) -> Ret' is no longer supported. \
                 Use RFC-010 syntax: '(T: Type, U: Type) -> ((params) -> Ret)'"
                    .to_string(),
            ));
            None
        }
        // `?` 是 try 运算符（表达式级），不能作为类型起始（#250）
        Some(TokenKind::Question) => {
            let span = state.span();
            state.bump(); // 消费掉 `?`，避免错误恢复在原地打转
            state.error(
                ErrorCodeDefinition::invalid_syntax(
                    "'?' 是 try 运算符，不是合法的类型。\
                     参数类型请使用类型名（如 Int、String、Any），或省略类型标注",
                )
                .at(span)
                .build(),
            );
            None
        }
        // 整数字面量类型：IsPositive(5) 中的 5；也支持算术表达式（`Array(Int, 2+3)`）
        Some(TokenKind::IntLiteral(n)) => {
            let n = *n;
            let span = state.span();
            state.bump();
            let expr = parse_type_position_const_expr(
                state,
                Expr::Lit(crate::frontend::core::lexer::tokens::Literal::Int(n), span),
                allow_const_expr,
            );
            Some(Type::ConstExpr(Box::new(expr)))
        }
        // 负整数字面量类型：IsPositive(-1) 中的 -1；同样接中缀（`-1+2`）
        Some(TokenKind::Minus) => {
            let span = state.span();
            state.bump();
            if let Some(TokenKind::IntLiteral(n)) = state.current().map(|t| &t.kind) {
                let n = *n;
                state.bump();
                let seed = Expr::UnOp {
                    op: crate::ast::UnOp::Neg,
                    expr: Box::new(Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(n),
                        span,
                    )),
                    span,
                };
                let expr = parse_type_position_const_expr(state, seed, allow_const_expr);
                Some(Type::ConstExpr(Box::new(expr)))
            } else {
                None
            }
        }
        // Note: fn type uses (Params) -> ReturnType syntax, not `fn` keyword
        _ => None,
    }
}

/// 种子后面紧跟的是「表达式续接」还是类型 / 绑定分隔符？
///
/// 黑名单与标识符种子路径的既有判据同源：`=`（绑定的初始化符）、`.`（类型名路径）、
/// `as`（转换）都**不**开启续接。少了这道闸门，`n: 3 = 3`（单值字面量类型标注）
/// 会被读成 `3 = 3` 的赋值表达式，绑定丢掉初始化器。
fn type_position_expr_continues(state: &ParserState<'_>) -> bool {
    state.infix_info().is_some()
        && !matches!(
            state.current().map(|t| &t.kind),
            Some(TokenKind::Dot | TokenKind::Eq | TokenKind::KwAs)
        )
}

/// 类型位常量表达式的**中缀续接**：从已解析的种子（标识符 / 整数字面量 /
/// 负字面量）继续走 Pratt 的 led 链，返回完整表达式。
///
/// 中缀解析的**唯一实现**是 `ParserState::parse_infix_from`（语句位表达式同一条
/// 链），本函数不复制第二份运算符解析。三种种子因此同构：`N+1`、`2+3`、`-1+2`
/// 都产出完整的 `BinOp` 树（issue #543：此前字面量种子解析完字面量就返回，中缀
/// 算符留给外层 `expect(RParen)` 报成「expected RParen, found Plus」）。
///
/// 续接失败（`2+` 这类真语法错）时发 E0012 并**保留种子**做错误恢复：
/// 诊断已经发出（不是静默），外层仍能闭合括号，不会再多报一条误导性的
/// 「expected a type, found RParen」。
fn parse_type_position_const_expr(
    state: &mut ParserState<'_>,
    seed: Expr,
    allow_const_expr: bool,
) -> Expr {
    if !allow_const_expr || !type_position_expr_continues(state) {
        return seed;
    }
    match state.parse_infix_from(seed.clone(), BP_LOWEST) {
        Some(expr) => expr,
        None => {
            // 深度闸拒绝（issue #540 修复轮）：E0019 已发射，同因双诊断压制，
            // 与 expect() 的弹出期压制同构。
            if !state.is_depth_rejected() {
                state.error(parse_msg(
                    state,
                    "类型位的常量表达式不完整：运算符缺少右操作数",
                ));
            }
            seed
        }
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

    // RFC-010: Result/Option 不 lower 成专用 AST 节点——它们是 std.result /
    // std.option 定义的记录式和类型（0e91d9f1 起编译器不再预置注册，
    // 变体集随 `use` 导入，见 type-system.md §4.2），与用户自定义泛型类型
    // 走同一 `Type::Generic` 路径。类型表示仍为
    // `Generic{"Result"/"Option", args}`（MonoType 层不变）。
    Some(Type::Generic {
        name,
        name_span,
        args,
    })
}

/// 形参项 / 形参列表里**类型子表达式**的解析口径。
///
/// 私有实现口径，不是新概念：三态各对应既有的一个类型解析入口
/// （`parse_type_annotation_required` / `parse_type_annotation_with` 的两种开关）。
#[derive(Clone, Copy)]
pub(super) enum ParamTyMode {
    /// 签名位（声明签名 / 箭头命中的类型位）：类型必需（失败报 E0010 并消费
    /// 肇事 token），允许类型级常量表达式续接（`Array(Int, N + 1)`）。
    Signature,
    /// `as` 目标类型里的形参位：类型宽容（失败静默，由调用方回退），且
    /// **禁止**常量表达式续接——D1：`x as (a: Int) -> Int * y` 的 `* y` 属值级，
    /// 不得被目标类型吞掉。
    SignatureNoConst,
    /// 类型位试探（元组 / 返回位具名括号）：不消费 `mut`、失败静默；
    /// 常量续接按字段开关（与 `parse_type_annotation_with` 同一语义）。
    Probe { allow_const_expr: bool },
}

impl ParamTyMode {
    fn parse(
        self,
        state: &mut ParserState<'_>,
    ) -> Option<Type> {
        match self {
            Self::Signature => parse_type_annotation_required(state),
            Self::SignatureNoConst => parse_type_annotation_with(state, false),
            Self::Probe { allow_const_expr } => parse_type_annotation_with(state, allow_const_expr),
        }
    }

    /// 是否在**形参位**（形参列表项）：只有形参位消费 `mut` 与 `+` 约束。
    /// 元组/括号类型位的 `(mut T, U)`、`(r: A + B)` 不是形参表，保持既有否定。
    fn is_signature(self) -> bool {
        matches!(self, Self::Signature | Self::SignatureNoConst)
    }
}

/// 当前位置是否是「一组形参 + 箭头」：`(` 且配平后紧跟 `->`。
///
/// **唯一实现点**。三处前瞻共用：类型位的 `(` 分支、柯里组
/// （`(a: Int) -> (Int) -> Int` 的返回位）、括号返回位
/// （`-> ((a: Int) -> Int)`）。判据只问「配平后是不是箭头」——形参项是
/// **命名**（`a: Int`）还是**裸类型**（`Int`）不参与判定：D53 之后两者都是形参，
/// 按首 token 形态分流会让 `(Int, Int) -> Int` 与 `(List(Int)) -> Int` 走不同分支。
pub(super) fn at_fn_param_group(state: &mut ParserState<'_>) -> bool {
    if !state.at(&TokenKind::LParen) {
        return false;
    }
    let saved = state.save_position();
    state.skip_balanced_parens();
    let after_is_arrow = state.at(&TokenKind::Arrow);
    state.restore_position(saved);
    after_is_arrow
}

/// 单个形参项（**唯一**产生式实现，RFC-039 D53 / SPEC type-system §3.5）：
///
/// ```text
/// Param ::= 'mut'? Identifier ':' TypeExpr    // 带名：名字进契约
///         | 'mut'? TypeExpr                   // 未命名带型参数：约束按位置生效
/// ```
///
/// 分流判据只有「标识符后面跟不跟 `:`」这一处。裸类型项产出
/// `name = ""` 的形参（[`Param::anonymous`] 的同一形态）——名字属于实现
/// （lambda 头自带，RFC-007 §47），AST 不伪造名字。
///
/// `mut` 只在**形参位**口径（`Signature` / `SignatureNoConst`）消费：`(mut x: Int)`
/// 是形参，而元组/括号**类型位**的 `(mut T, U)` 不是形参表，保持既有
/// 「`mut` 不是类型起始」的语法否定。
fn parse_param_item(
    state: &mut ParserState<'_>,
    mode: ParamTyMode,
) -> Option<Param> {
    let span = state.span();
    let is_mut = if mode.is_signature() {
        state.skip(&TokenKind::KwMut)
    } else {
        false
    };

    if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
        if matches!(state.peek().map(|t| &t.kind), Some(TokenKind::Colon)) {
            let name = name.clone();
            state.bump(); // 形参名
            state.skip(&TokenKind::Colon);
            let parsed_type = mode.parse(state)?;
            let ty = if mode.is_signature() && state.at(&TokenKind::Plus) {
                // `T: Clone + Add`：多约束用 `Type::Tuple` 容器承载，
                // `extract_generic_params` 会解包成多条约束（既有口径，未改）。
                let mut all_types = vec![parsed_type];
                while state.skip(&TokenKind::Plus) {
                    if let Some(extra_type) = mode.parse(state) {
                        all_types.push(extra_type);
                    } else {
                        break;
                    }
                }
                Type::Tuple(all_types)
            } else {
                wrap_literal_type_if_needed(name.clone(), parsed_type)
            };
            return Some(Param {
                name,
                ty: Some(ty),
                is_mut,
                quantity: None,
                span,
            });
        }
    }

    // 裸类型项：未命名带型参数（类型约束按位置生效，名字由实现提供）。
    let ty = mode.parse(state)?;
    Some(Param {
        name: String::new(),
        ty: Some(ty),
        is_mut,
        quantity: None,
        span,
    })
}

/// 形参列表（`(` 已消费，`)` 留给调用方）：`Param (',' Param)* ','?`。
fn parse_param_items(
    state: &mut ParserState<'_>,
    mode: ParamTyMode,
) -> Option<Vec<Param>> {
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        params.push(parse_param_item(state, mode)?);
        if !state.skip(&TokenKind::Comma) {
            break;
        }
    }
    Some(params)
}

/// Parse function type with parameter names: `(a: Int, b: Int) -> Int`
/// Returns (Vec<Param>, return_type)
/// This is for RFC-010 unified syntax: `name: (a: Int, b: Int) -> Ret = body`
/// Also supports const generic literal types: `factorial: [n: Int](n: n) -> Int`
///
/// 解析递归深度闸的**柯里组汇点**（issue #540 修复轮）：curry 组与括号
/// 返回位的自递归（`-> (a: Int) -> (b: Int) -> …`）不经
/// [`parse_type_annotation`]，须在本入口过同一 enter/exit 计数——否则深
/// 柯里链是真递归栈膨胀的绕闸入口。
pub(super) fn parse_fn_type_with_names(
    state: &mut ParserState<'_>,
    mode: ParamTyMode,
) -> Option<(Vec<Param>, Box<Type>)> {
    if !state.enter_nested_parse() {
        return None;
    }
    let result = parse_fn_type_with_names_nested(state, mode);
    state.exit_nested_parse();
    result
}

/// [`parse_fn_type_with_names`] 的闸内本体。
///
/// 形参项的产生式在 [`parse_param_item`]（唯一实现）：`Param ::= Identifier ':' TypeExpr | TypeExpr`
/// （RFC-039 D53）。此处只负责列表与尾部的 `)` / `->` / 返回位。
fn parse_fn_type_with_names_nested(
    state: &mut ParserState<'_>,
    mode: ParamTyMode,
) -> Option<(Vec<Param>, Box<Type>)> {
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    let params = parse_param_items(state, mode)?;

    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // Expect ->
    if !state.expect(&TokenKind::Arrow) {
        return None;
    }
    // 返回位后面的柯里组（`(n: N) -> (Int) -> Int`）：判据 = 括号配平后紧跟 `->`，
    // 与类型位的 `(` 分支共用 [`at_fn_param_group`]——返回类型本身可能是元组
    // `(T, U)`（#253），故必须前瞻箭头。返回值收敛为**两元组**（6.5.2 commit ③）：
    // 第一组形参 + 返回类型；后续各组同栖在嵌套 `Type::Fn.params` 里，调用方经
    // `Type::fn_params_flat` 拍平取得全部 curry 形参——不再旁路返回同一份数据。
    if at_fn_param_group(state) {
        // 前瞻已命中箭头：内层失败即收口（回退只会以错位起点重扫残骸）。
        // 深度闸拒绝时内层已报 E0019——残骸交给语句层的 drain_broken_construct。
        let (inner_params, inner_ret) = parse_fn_type_with_names(state, mode)?;
        let nested_fn = Type::Fn {
            params: inner_params,
            return_type: inner_ret,
        };
        return Some((params, Box::new(nested_fn)));
    }
    // RFC-004 括号语义：`-> ((a: Int) -> Int)` —— 外层括号声明“返回值是函数”，
    // 链条到此终止（`split_curry` 遇 `Paren` 停下）。判定走同一个
    // [`at_fn_param_group`]：只有「括号内是形参组 + 箭头」才是括号返回位，
    // 元组返回位 `-> ((Int, Int))` 因此不再误入内层解析（既有 E0010 误报的根因）。
    let saved_paren = state.save_position();
    if state.skip(&TokenKind::LParen) && at_fn_param_group(state) {
        if let Some((inner_params, inner_ret)) = parse_fn_type_with_names(state, mode) {
            if state.skip(&TokenKind::RParen) {
                let inner_fn = Type::Fn {
                    params: inner_params,
                    return_type: inner_ret,
                };
                return Some((params, Box::new(Type::Paren(Box::new(inner_fn)))));
            }
        }
        // 内层解析失败：诊断已发射（含深度闸的 E0019），恢复后按普通返回类型
        // 再解析只会以错位起点重扫残骸并重复报错——收口，残骸交给语句层 drain。
        return None;
    }
    state.restore_position(saved_paren);

    // `->` 后返回类型按签名口径解析（`Signature` = 严格版，非法 token 报错而非
    // 静默吞掉整个签名；`SignatureNoConst` = `as` 目标位，禁止常量续接）。
    let return_type = Box::new(mode.parse(state)?);
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

    let mut body: Vec<TypeBodyItem> = Vec::new();
    if !state.at(&TokenKind::RBrace) {
        while let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
            let name = name.clone();
            let name_start = state.save_position();
            state.bump();

            // 匿名类型表达式: Identifier 后接跟 '('（如 Assert(N > 0)）
            if state.at(&TokenKind::LParen) {
                state.restore_position(name_start);
                let ty = parse_type_annotation(state)?;
                body.push(TypeBodyItem::Expr(ty));
                if !state.skip(&TokenKind::Comma) {
                    break;
                }
                continue;
            }

            // 检查下一个 token 是否是 mut 或冒号
            let is_mut = state.skip(&TokenKind::KwMut);

            // 检查下一个 token 是否是冒号
            if state.at(&TokenKind::Colon) {
                // 有冒号: 字段声明或匿名函数绑定
                state.bump(); // consume ':'

                // 注解的起点——匿名函数绑定取不到函数类型时，诊断落在这里
                let annotation_span = state.span();
                let field_type = parse_type_annotation(state)?;

                // 检查是否有位置绑定: [positions]
                let positions = parse_optional_binding_positions(state);

                if state.skip(&TokenKind::Eq) {
                    if let Some(pos) = positions {
                        // 匿名函数绑定: name: FnType[pos] = lambda
                        //
                        // 形参与返回类型由**注解**提供：注解不是函数类型时没有可用的真实
                        // 类型，只能报诊断并放弃该绑定——不得伪造返回值（WBS 6.6.7；
                        // 6.3 的 `Type::Name{"()"}` 兜底正是被删掉的伪造载体）。
                        let Some((params, return_type)) = extract_fn_type_info(&field_type) else {
                            state.error(
                                ErrorCodeDefinition::invalid_syntax(&format!(
                                    "Anonymous function binding '{name}' needs a function type annotation before [positions] (the lambda's parameter and return types come from it)"
                                ))
                                .at(annotation_span)
                                .build(),
                            );
                            return None;
                        };
                        let body_expr = state.parse_expression(BP_LOWEST)?;
                        body.push(TypeBodyItem::Binding(TypeBodyBinding {
                            name,
                            kind: BindingKind::Anonymous {
                                params,
                                return_type: Box::new(return_type),
                                positions: pos,
                                body: Box::new(body_expr),
                            },
                        }));
                    } else {
                        // 默认值字段: name: Type = expression
                        let default_expr = state.parse_expression(BP_LOWEST)?;
                        body.push(TypeBodyItem::Field(StructField::with_default(
                            name,
                            is_mut,
                            field_type,
                            default_expr,
                        )));
                    }
                } else {
                    // 普通字段: name: Type
                    // 注意：不再硬编码 "Assert" — 所有 name: Type 形式一律作为字段。
                    // 匿名类型表达式 Assert(N > 0) 已由前面的 LParen 前瞻分支处理。
                    body.push(TypeBodyItem::Field(StructField::new(
                        name, is_mut, field_type,
                    )));
                }
            } else if state.skip(&TokenKind::Eq) {
                // 无冒号但有等号: 外部函数绑定 name = function[positions] 或默认绑定 name = function
                let func_name = match state.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(n)) => n.clone(),
                    _ => {
                        state.error(parse_msg(
                            state,
                            format!("Expected function name after '=' in binding '{}'", name),
                        ));
                        return None;
                    }
                };
                state.bump(); // consume function name

                // RFC-004: 尝试解析位置绑定 [positions]，如果没有则为默认绑定
                if state.at(&TokenKind::LBracket) {
                    let positions = parse_binding_positions(state).ok()?;
                    body.push(TypeBodyItem::Binding(TypeBodyBinding {
                        name,
                        kind: BindingKind::External {
                            function: func_name,
                            positions,
                        },
                    }));
                } else {
                    // 默认绑定: name = function（自动查找第一个类型匹配位置）
                    body.push(TypeBodyItem::Binding(TypeBodyBinding {
                        name,
                        kind: BindingKind::DefaultExternal {
                            function: func_name,
                        },
                    }));
                }
            } else if is_mut {
                // mut 后面没有冒号是语法错误
                state.error(parse_msg(
                    state,
                    format!("Expected ':' after 'mut' in field '{}'", name),
                ));
                return None;
            } else if state.at(&TokenKind::Dot)
                && matches!(
                    state.peek().map(|t| &t.kind),
                    Some(TokenKind::Identifier(_))
                )
            {
                // 类型体里的证据字段路径（形如 { bb.w }，RFC-027 §14.5）：整条路径作为
                // 类型体表达式项，交汇到 parse_type_annotation 的同一入口（不另写解析）。
                state.restore_position(name_start);
                let ty = parse_type_annotation(state)?;
                body.push(TypeBodyItem::Expr(ty));
            } else if matches!(
                state.current().map(|t| &t.kind),
                Some(
                    TokenKind::Gt
                        | TokenKind::Lt
                        | TokenKind::Ge
                        | TokenKind::Le
                        | TokenKind::EqEq
                        | TokenKind::Neq
                )
            ) {
                // 精化约束表达式: x > 0
                state.restore_position(name_start);
                let ty = parse_type_annotation(state)?;
                body.push(TypeBodyItem::Expr(ty));
            } else {
                // 接口约束: InterfaceName
                body.push(TypeBodyItem::Interface(name));
            }

            // 跳过逗号，如果不是逗号则结束循环
            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }

    if !state.skip(&TokenKind::RBrace) {
        // 未找到闭合 }——内容不是合法类型体
        return None;
    }

    Some(Type::Struct { body })
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
        state.error(parse_msg(
            state,
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
                // RFC-004 位置绑定。字面量载荷是 i128（词法 parse::<i128> 恒非负）：
                // - i64 域内：精确落位——合法位置逐字零变化（#563 语义边界）；
                // - 取负在 i128 域做（checked_neg 恒成功），#560 同族 panic
                //   （`[-i64::MIN 形态]` 修前在此裸 `-i64::MIN` 溢出，debug 构建
                //   exit 101）随之消除；`[-2^63]` 精确得 i64::MIN，与 ast 侧
                //   extract_binding_positions 的取负口径一致；
                // - 超出 i64 表示域：AST 位置域是 i64，原文无法承载——以哨兵
                //   桥接交 checker 的 E1064 判域（#563 甲案：正溢出取 i64::MAX
                //   必越参数上界、负溢出取 i64::MIN 归一化后仍负，两条都必落
                //   normalize_binding_positions 的 E1064）。哨兵文面显示的是
                //   哨兵值而非原字面量，裁决心智与 ast 侧「超域拒绝」一致。
                let magnitude = *n;
                let signed: i128 = if is_negative {
                    magnitude.checked_neg().unwrap_or(i64::MIN as i128)
                } else {
                    magnitude
                };
                let value = if signed > i64::MAX as i128 {
                    i64::MAX
                } else if signed < i64::MIN as i128 {
                    i64::MIN
                } else {
                    signed as i64
                };
                positions.push(value);
                state.bump();
                state.skip(&TokenKind::Comma);
            }
            _ => {
                state.error(parse_msg(
                    state,
                    "Expected integer position in binding".to_string(),
                ));
                return Err(());
            }
        }
    }

    if !state.at(&TokenKind::RBracket) {
        state.error(parse_msg(
            state,
            "Expected ']' after binding positions".to_string(),
        ));
        return Err(());
    }
    state.bump(); // consume ']'

    Ok(positions)
}

/// 剥掉类型位置的**透明包装**：RFC-004 的 `Paren` 与 RFC-027 §3 的 `NamedParen`。
///
/// 两层在类型检查与 lower 里都透明（`From<ast::Type> for MonoType` 直接递归内层），
/// 所以「注解究竟是什么类型」必须剥到底再判。匿名函数绑定尤其依赖这一点：裸函数类型
/// 后面跟 `[pos]` 会被当成索引 / const 泛型实参，文档记载的写法一定是
/// `((a: Int) -> Bool)[0]`——注解是 `Paren(Fn)`，不是 `Fn`。
fn unwrap_transparent_type(mut ty: &Type) -> &Type {
    loop {
        match ty {
            Type::Paren(inner) => ty = inner,
            Type::NamedParen { inner, .. } => ty = inner,
            other => return other,
        }
    }
}

/// 从函数类型中提取形参与返回类型。
///
/// 返回 `None` = 注解（剥掉透明括号后）**不是**函数类型：匿名函数绑定的形参与返回
/// 类型由注解提供，没有函数类型就没有可用的真实类型，调用方必须报诊断并放弃该绑定
/// （WBS 6.6.7）。
///
/// **不得**改成任何「占位返回类型」兜底——6.3 曾把 `Type::Void` 兜底换成
/// `Type::Name{"()"}`：那既不是真实类型（下游会当成用户类型或 `Void` 的类型写法），
/// 也让 `semantic_tokens` 的 `Name` 臂为这个 dummy 节点多记一枚 token。该路径
/// **语法可达**（`{ cb: (Int)[0] = 1 }`：位置绑定要求函数类型注解，语法本身不拦），
/// 故按诊断处理，而不是 `unreachable!`。
fn extract_fn_type_info(ty: &Type) -> Option<(Vec<Param>, Type)> {
    let Type::Fn {
        params,
        return_type,
    } = unwrap_transparent_type(ty)
    else {
        return None;
    };
    let param_list = params
        .iter()
        .enumerate()
        .map(|(i, p)| Param {
            name: format!("arg{}", i),
            ty: p.ty.clone(),
            is_mut: false,
            quantity: None,
            span: Span::dummy(),
        })
        .collect();
    Some((param_list, *return_type.clone()))
}
