//! Precedence handling for Pratt parser
//!
//! This module defines binding power levels and precedence rules with RFC-010/011 support.

/// Binding power levels for Pratt parser
pub const BP_LOWEST: u8 = 0;
pub const BP_ASSIGN: u8 = 1;
pub const BP_LOGICAL_OR: u8 = 2;
pub const BP_LOGICAL_AND: u8 = 3;
pub const BP_EQUALITY: u8 = 4;
pub const BP_COMPARISON: u8 = 5;
pub const BP_TERM: u8 = 6;
pub const BP_FACTOR: u8 = 7;
pub const BP_UNARY: u8 = 8;
pub const BP_CAST: u8 = 8;
pub const BP_CALL: u8 = 9;
pub const BP_HIGHEST: u8 = 10;

/// Additional binding power levels for infix operators
// #299 §3: Range 提到高绑定力——'x in 1..10' 中 range 需作为整体成为 in 的右操作数，
// 且高于逻辑/比较层级（数学惯例：区间是紧绑定构造）
pub const BP_RANGE: u8 = 7;
pub const BP_OR: u8 = 1;
pub const BP_AND: u8 = 2;
pub const BP_EQ: u8 = 3;
pub const BP_CMP: u8 = 4;
// #285: SPEC §2.2 相对顺序 乘除(4) > 加减(5) > 移位(7) > 位运算(8) > 比较(9)
pub const BP_BIT: u8 = 5;
pub const BP_SHIFT: u8 = 6;
pub const BP_ADD: u8 = 7;
pub const BP_MUL: u8 = 8;

/// `..` 的左绑定力（#300 F/I 项：左 6 低于加法 7；右操作数按加法级解析，
/// 故 `0..n+2` = `0..(n+2)`、`a..b..c` = `(a..b)..c`）。
pub const BP_RANGE_L: u8 = 6;

/// `..` 的右绑定力（与 `BP_RANGE` 同值：右操作数按加法级解析，吞加法不吞同级 `..`）。
pub const BP_RANGE_R: u8 = BP_RANGE;

/// 模式位探测表达式的阈值（issue #405）：高于任何中缀绑定力，使 `lo..hi`
/// 范围模式能被整体探测后再按模式解释，而不是先被 `..` 拆走。
pub const BP_PATTERN_PROBE: u8 = 12;
