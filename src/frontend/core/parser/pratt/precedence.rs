//! Pratt 解析的绑定力与算子属性：**唯一声明处**（SPEC syntax.md §2.2）。
//!
//! 本模块是一张声明式表的家：每个运算符只有一行，行里同时给出
//!   · 触发 token；
//!   · 左 / 右绑定力（右值由结合性派生：左结合 = left + 1，右结合 = left）；
//!   · 对应的 AST 表示（`ast::BinOp`；非算术算子为 `None`）；
//!   · 解析策略（普通二元 / 赋值 / `in` / 移位 / 调用 / 字段 / 索引 / `?` / `as` / lambda）。
//!
//! `led.rs` 的 `infix_info` 与 `parse_binary` 都只查这张表，不再各自列一遍
//! token → 绑定力或 token → 算子；`nud.rs` 的前缀层同样查 [`PREFIX_OPS`]。
//! 两套 `BinOp` 表示（`ast::BinOp` 与 `types::const_data::BinOp`）的对应关系由
//! 本表的 `op` 字段派生、由 `pratt/tests/operator_sources.rs` 逐算子互校。

use crate::ast::BinOp;
use crate::frontend::core::lexer::tokens::TokenKind;

// ---------------------------------------------------------------------------
// 绑定力层级（数值越大绑定越紧；括号里的「级 N」就是 SPEC §2.2 的优先级行）
// ---------------------------------------------------------------------------

/// 表达式 / 子表达式的入口绑定力。
pub const BP_LOWEST: u8 = 0;
/// 级 13：`=`（右结合）。
pub const BP_ASSIGN: u8 = 1;
/// 级 11：`or`。
pub const BP_OR: u8 = 2;
/// 级 10：`and`。
pub const BP_AND: u8 = 3;
/// 级 9：`==` `!=` `<` `>` `<=` `>=` `in`（同一级、左结合）。
pub const BP_EQUALITY: u8 = 4;
/// 级 8：`&` `|` `^`。
pub const BP_BITWISE: u8 = 5;
/// 级 7：`<<` `>>`。
pub const BP_SHIFT: u8 = 6;
/// 级 6：`..` 的左绑定力（右操作数按 [`BP_RANGE_RIGHT`] 解析：吞加法、不吞同级 `..`）。
pub const BP_RANGE_LEFT: u8 = 6;
/// 级 6：`..` 的右绑定力。
pub const BP_RANGE_RIGHT: u8 = 7;
/// 级 5：`+` `-`。
pub const BP_ADD: u8 = 7;
/// 级 4：`*` `/` `%`。
pub const BP_MUL: u8 = 8;
/// 级 3：一元前缀 `!` `-` `+`（右结合：操作数按 `BP_UNARY + 1` 解析）。
pub const BP_UNARY: u8 = 9;
/// 级 2：`as`（必须高于 `BP_MUL`、低于 `BP_CALL`——D2 的判据）。
pub const BP_CAST: u8 = 9;
/// 级 1：`()` `[]` `.` `?`（调用 / 字段 / 索引 / 错误传播）。
pub const BP_CALL: u8 = 10;
/// 原子（字面量 / 标识符 / 分组 / 块 / if / match …）的绑定力。
pub const BP_HIGHEST: u8 = 11;
/// 单参 lambda 中缀 `=>`：只在表达式顶层触发，体按 [`BP_LOWEST`] 解析。
pub const BP_LAMBDA: u8 = 11;

/// 模式位探测阈值（issue #405）：高于任何中缀绑定力，使 `lo..hi` 范围模式能被
/// 整体探测后再按模式解释，而不是先被 `..` 拆走。
pub const BP_PATTERN_PROBE: u8 = 12;

// ---------------------------------------------------------------------------
// 中缀 / 后缀算子表（唯一声明）
// ---------------------------------------------------------------------------

/// 中缀算子的解析策略：决定由哪个 `parse_*` 函数消费该 token。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfixStrategy {
    /// 普通二元（`+` `*` `==` `&` `..` …）。
    Binary,
    /// `=`（右结合赋值）。
    Assign,
    /// `in`（成员谓词，产出 `Expr::In`）。
    In,
    /// `<<` / `>>`（两枚相邻同种 token 合成一枚算子）。
    Shift,
    /// `(`（调用）。
    Call,
    /// `.`（字段访问）。
    Field,
    /// `[`（索引）。
    Index,
    /// `?`（错误传播）。
    Try,
    /// `as`（类型转换）。
    Cast,
    /// `=>`（单参 lambda）。
    Lambda,
}

/// 一行中缀算子声明（TokenKind 非 Copy，故本结构只 Clone）。
#[derive(Clone, Debug)]
pub struct InfixOp {
    /// 触发该行的 token。
    pub token: TokenKind,
    /// 左绑定力。
    pub left_bp: u8,
    /// 右绑定力（左结合 = left + 1；右结合 = left）。
    pub right_bp: u8,
    /// AST 表示；`None` 表示该算子不产出 `ast::BinOp`（由策略函数产出别的节点）。
    pub op: Option<BinOp>,
    /// 解析策略。
    pub strategy: InfixStrategy,
    /// 是否需要两枚相邻同种 token（`<<` / `>>`）。
    pub double_token: bool,
}

/// 中缀算子表——**绑定力与 AST 映射的唯一声明处**。
///
/// `right_bp` 的消费范围（复审 t29 F10）：只有会自己解析右操作数/容器的策略消费它——
/// [`InfixStrategy::Binary`]（`parse_binary`）、[`InfixStrategy::In`]（`parse_in` 取该行
/// `left_bp` 作容器下限）、[`InfixStrategy::Shift`]（`parse_shift`）与
/// [`InfixStrategy::Assign`]（`parse_assign`）。`Call` / `Field` / `Index` / `Try` /
/// `Cast` / `Lambda` 自带解析入口、**不使用**传入的右绑定力（`_left_bp` 被忽略）；
/// 它们的 `right_bp` 只作为表内自洽值存在，由 `precedence_inline` 的一致性断言守着。
pub const INFIX_OPS: &[InfixOp] = &[
    InfixOp {
        token: TokenKind::Eq,
        left_bp: BP_ASSIGN,
        right_bp: BP_ASSIGN,
        op: Some(BinOp::Assign),
        strategy: InfixStrategy::Assign,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Or,
        left_bp: BP_OR,
        right_bp: BP_OR + 1,
        op: Some(BinOp::Or),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::And,
        left_bp: BP_AND,
        right_bp: BP_AND + 1,
        op: Some(BinOp::And),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::EqEq,
        left_bp: BP_EQUALITY,
        right_bp: BP_EQUALITY + 1,
        op: Some(BinOp::Eq),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Neq,
        left_bp: BP_EQUALITY,
        right_bp: BP_EQUALITY + 1,
        op: Some(BinOp::Neq),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Lt,
        left_bp: BP_EQUALITY,
        right_bp: BP_EQUALITY + 1,
        op: Some(BinOp::Lt),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Le,
        left_bp: BP_EQUALITY,
        right_bp: BP_EQUALITY + 1,
        op: Some(BinOp::Le),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Gt,
        left_bp: BP_EQUALITY,
        right_bp: BP_EQUALITY + 1,
        op: Some(BinOp::Gt),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Ge,
        left_bp: BP_EQUALITY,
        right_bp: BP_EQUALITY + 1,
        op: Some(BinOp::Ge),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::KwIn,
        left_bp: BP_EQUALITY,
        right_bp: BP_EQUALITY + 1,
        op: None,
        strategy: InfixStrategy::In,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Ampersand,
        left_bp: BP_BITWISE,
        right_bp: BP_BITWISE + 1,
        op: Some(BinOp::BitAnd),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Pipe,
        left_bp: BP_BITWISE,
        right_bp: BP_BITWISE + 1,
        op: Some(BinOp::BitOr),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Caret,
        left_bp: BP_BITWISE,
        right_bp: BP_BITWISE + 1,
        op: Some(BinOp::BitXor),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Lt,
        left_bp: BP_SHIFT,
        right_bp: BP_SHIFT + 1,
        op: Some(BinOp::Shl),
        strategy: InfixStrategy::Shift,
        double_token: true,
    },
    InfixOp {
        token: TokenKind::Gt,
        left_bp: BP_SHIFT,
        right_bp: BP_SHIFT + 1,
        op: Some(BinOp::Shr),
        strategy: InfixStrategy::Shift,
        double_token: true,
    },
    InfixOp {
        token: TokenKind::DotDot,
        left_bp: BP_RANGE_LEFT,
        right_bp: BP_RANGE_RIGHT,
        op: Some(BinOp::Range),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Plus,
        left_bp: BP_ADD,
        right_bp: BP_ADD + 1,
        op: Some(BinOp::Add),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Minus,
        left_bp: BP_ADD,
        right_bp: BP_ADD + 1,
        op: Some(BinOp::Sub),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Star,
        left_bp: BP_MUL,
        right_bp: BP_MUL + 1,
        op: Some(BinOp::Mul),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Slash,
        left_bp: BP_MUL,
        right_bp: BP_MUL + 1,
        op: Some(BinOp::Div),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Percent,
        left_bp: BP_MUL,
        right_bp: BP_MUL + 1,
        op: Some(BinOp::Mod),
        strategy: InfixStrategy::Binary,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::KwAs,
        left_bp: BP_CAST,
        right_bp: BP_CAST + 1,
        op: None,
        strategy: InfixStrategy::Cast,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::FatArrow,
        left_bp: BP_LAMBDA,
        right_bp: 1,
        op: None,
        strategy: InfixStrategy::Lambda,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::LParen,
        left_bp: BP_CALL,
        right_bp: BP_CALL + 1,
        op: None,
        strategy: InfixStrategy::Call,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Dot,
        left_bp: BP_CALL,
        right_bp: BP_CALL + 1,
        op: None,
        strategy: InfixStrategy::Field,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::LBracket,
        left_bp: BP_CALL,
        right_bp: BP_CALL + 1,
        op: None,
        strategy: InfixStrategy::Index,
        double_token: false,
    },
    InfixOp {
        token: TokenKind::Question,
        left_bp: BP_CALL,
        right_bp: BP_CALL + 1,
        op: None,
        strategy: InfixStrategy::Try,
        double_token: false,
    },
];

/// 查中缀算子行：`double_token` 参与匹配，专门消解 `<<` / `>>` 的双行。
///
/// `Lt` / `Gt` 在表里各有两行：比较（单 token）与移位（两枚相邻同种 token）。
/// 调用方已知下一个 token 是否为同种时传 `double_token = true`，否则传 `false`；
/// 表是唯一事实源，`double_token` 不是「第二份映射」而是该行语法的组成部分。
pub fn infix_op_matching(
    kind: &TokenKind,
    double_token: bool,
) -> Option<&'static InfixOp> {
    INFIX_OPS
        .iter()
        .find(|entry| entry.token == *kind && entry.double_token == double_token)
}

/// 查单 token 中缀算子行（等价 `infix_op_matching(kind, false)`）。
pub fn infix_op(kind: &TokenKind) -> Option<&'static InfixOp> {
    infix_op_matching(kind, false)
}

/// 查单 token 中缀算子的 AST 表示（`in` / `as` / `=>` / 后缀算子返回 `None`）。
pub fn infix_ast_op(kind: &TokenKind) -> Option<BinOp> {
    infix_op(kind).and_then(|entry| entry.op)
}

/// 查双 token 中缀算子的 AST 表示（`<<` / `>>` 的 Shl / Shr）。
pub fn infix_double_ast_op(kind: &TokenKind) -> Option<BinOp> {
    infix_op_matching(kind, true).and_then(|entry| entry.op)
}

// ---------------------------------------------------------------------------
// 前缀算子表
// ---------------------------------------------------------------------------

/// 前缀算子的解析策略。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixStrategy {
    /// `!` `-` `+` `*`（一元 / 解引用）。
    Unary,
    /// `&` `&mut`（借用）。
    Borrow,
}

/// 一行前缀算子声明（TokenKind 非 Copy，故本结构只 Clone）。
#[derive(Clone, Debug)]
pub struct PrefixOp {
    pub token: TokenKind,
    pub bp: u8,
    pub strategy: PrefixStrategy,
}

/// 前缀算子表——前缀绑定力与策略的唯一声明处。
pub const PREFIX_OPS: &[PrefixOp] = &[
    PrefixOp {
        token: TokenKind::Minus,
        bp: BP_UNARY,
        strategy: PrefixStrategy::Unary,
    },
    PrefixOp {
        token: TokenKind::Plus,
        bp: BP_UNARY,
        strategy: PrefixStrategy::Unary,
    },
    PrefixOp {
        token: TokenKind::Not,
        bp: BP_UNARY,
        strategy: PrefixStrategy::Unary,
    },
    PrefixOp {
        token: TokenKind::Star,
        bp: BP_UNARY,
        strategy: PrefixStrategy::Unary,
    },
    PrefixOp {
        token: TokenKind::Ampersand,
        bp: BP_UNARY,
        strategy: PrefixStrategy::Borrow,
    },
    PrefixOp {
        token: TokenKind::MutRef,
        bp: BP_UNARY,
        strategy: PrefixStrategy::Borrow,
    },
];

/// 查前缀算子行。
pub fn prefix_op(kind: &TokenKind) -> Option<&'static PrefixOp> {
    PREFIX_OPS.iter().find(|entry| entry.token == *kind)
}
