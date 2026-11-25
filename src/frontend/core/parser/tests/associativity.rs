//! 表达式结合性 / 优先级语言级用例 — 基于语言规范 syntax §2.2 等
//!
//! 判据面：只经公共入口 [`parse_with_probe`] 断言**语言级 AST 形状**，不引用 Pratt
//! 内部符号（不出现 pratt 模块路径、绑定力常量，也不直接构造或调用解析器的状态类型）——
//! WBS 8.4 双解析器差分与 8.6 删 Pratt 之后本文件必须原样存活，守的是 C5 等价面
//! 「合法程序的 AST 形状不变」。
//!
//! 条款来源（逐条标注在用例的 Arrange 注释上）：
//! - SPEC syntax §2.2：运算符优先级表的级号、「结合性」列与 Range 绑定力注
//! - SPEC syntax §2.8：`MatchArm ::= Pattern ('|' Pattern)*`（或模式收链）
//! - SPEC syntax §2.10：`Lambda ::= ... '=>' Expr`（体是完整 Expr ⇒ 右贪婪）
//! - SPEC syntax §2.11：「`?` 是后缀运算符，优先级与 `.` 同级」
//!
//! 三扇门（coding-rules 7.2）：本文件全部走**条款**门（上列规范节）。
//! 「合并后的层内绑定力严格递增、无重复」不在本文件断言：按 coding-rules 7.1，这类断言
//! 冻的是实现选择（换一个把同层数值重排的正确实现就会红），属特征化、不能当门禁；
//! 层级关系改由下面的语言级 AST 形状逐条守。
//!
//! `as` 与 `*`、相等与比较的相对层级（SPEC syntax §2.2 级 2/4/9）曾是两条已登记缺陷
//! （09a §P8 8.8 六 D1–D3）；WBS 8.3.1 修复后由下面的用例钉住：`as` 高于 `*`、
//! 转换目标类型不吞其后中缀、`==`/`!=` 与 `<`/`>`/`<=`/`>=` 同级左结合。

use crate::ast::{BinOp, Block, Expr, Param, Pattern, StmtKind, Type};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::{parse_with_probe, NullProbe};

/// 解析唯一一条 `r = <expr>` 夹具，取回赋值右侧的表达式。
///
/// 夹具恒取赋值形态：语句位 `=` 直接产 `StmtKind::Assign`，右侧按全表达式解析，
/// 断言面因此只落在表达式的结合性上，不掺语句解析差异（RFC-038 换行规则等）。
fn assigned_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("词法失败: {e:?}\n源码:\n{source}"));
    let parsed = parse_with_probe(&tokens, &NullProbe);
    assert!(
        !parsed.has_errors,
        "夹具必须无诊断地解析通过（实际诊断: {:?}）\n源码:\n{source}",
        parsed.errors
    );
    assert_eq!(
        parsed.module.items.len(),
        1,
        "夹具必须恰好一条语句\n源码:\n{source}"
    );
    match &parsed.module.items[0].kind {
        StmtKind::Assign {
            value: Some(value), ..
        } => (**value).clone(),
        other => panic!("夹具必须是 r = <expr> 赋值语句，实际 {other:?}\n源码:\n{source}"),
    }
}

/// 断言二元节点的算子，并返回其 (左, 右) 操作数。
///
/// `clause` 是本次断言所守的规范条款（SPEC 节号），直接进失败消息——断言红时
/// 不必回查用例就知道红的是哪条性质。
fn expect_bin_op<'a>(
    expr: &'a Expr,
    expected: BinOp,
    clause: &str,
    source: &str,
) -> (&'a Expr, &'a Expr) {
    match expr {
        Expr::BinOp {
            op, left, right, ..
        } => {
            assert_eq!(
                *op, expected,
                "{clause}：算子应为 {expected:?}，实际 {op:?}（源码: {source:?}）"
            );
            (left.as_ref(), right.as_ref())
        }
        other => panic!("{clause}：期望二元表达式，实际 {other:?}（源码: {source:?}）"),
    }
}

/// 断言表达式是给定名字的变量引用。
fn expect_var(
    expr: &Expr,
    expected: &str,
    clause: &str,
    source: &str,
) {
    match expr {
        Expr::Var(name, _) => assert_eq!(
            name, expected,
            "{clause}：变量名应为 {expected:?}（源码: {source:?}）"
        ),
        other => panic!("{clause}：期望变量 {expected:?}，实际 {other:?}（源码: {source:?}）"),
    }
}

/// 断言表达式是给定值的整数字面量。
fn expect_int_lit(
    expr: &Expr,
    expected: i128,
    clause: &str,
    source: &str,
) {
    match expr {
        Expr::Lit(Literal::Int(value), _) => assert_eq!(
            *value, expected,
            "{clause}：字面量应为 {expected}（源码: {source:?}）"
        ),
        other => panic!("{clause}：期望整数字面量 {expected}，实际 {other:?}（源码: {source:?}）"),
    }
}

/// 断言表达式是 lambda，返回其形参表与体块。
fn expect_lambda<'a>(
    expr: &'a Expr,
    clause: &str,
    source: &str,
) -> (&'a [Param], &'a Block) {
    match expr {
        Expr::Lambda { params, body, .. } => (params, body),
        other => panic!("{clause}：期望 lambda 表达式，实际 {other:?}（源码: {source:?}）"),
    }
}

/// 断言块体恰好一条 `return <expr>` 语句，返回被返回的表达式。
fn expect_single_return<'a>(
    block: &'a Block,
    clause: &str,
    source: &str,
) -> &'a Expr {
    assert_eq!(
        block.stmts.len(),
        1,
        "{clause}：lambda 体应只有一条 return 语句\n源码:\n{source}"
    );
    match &block.stmts[0].kind {
        StmtKind::Return(Some(expr)) => expr.as_ref(),
        other => panic!("{clause}：期望 return 表达式，实际 {other:?}（源码: {source:?}）"),
    }
}

#[test]
fn test_sub_is_left_associative() {
    // Arrange：同级链 a - b - c——SPEC syntax §2.2 级 5（+ -）结合性「左到右」
    let source = "r = a - b - c";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层 - 的左操作数才是内层 a - b（右结合实现会落在右操作数上）
    let (lhs, rhs) = expect_bin_op(&expr, BinOp::Sub, "§2.2 级 5 左结合：外层必须是 -", source);
    let (inner_lhs, inner_rhs) = expect_bin_op(
        lhs,
        BinOp::Sub,
        "§2.2 级 5 左结合：外层左操作数必须是内层 a - b",
        source,
    );
    expect_var(inner_lhs, "a", "§2.2 级 5 左结合：内层左操作数", source);
    expect_var(inner_rhs, "b", "§2.2 级 5 左结合：内层右操作数", source);
    expect_var(
        rhs,
        "c",
        "§2.2 级 5 左结合：外层右操作数必须是最右端的 c",
        source,
    );
}

#[test]
fn test_div_is_left_associative() {
    // Arrange：同级链 a / b / c——SPEC syntax §2.2 级 4（* / %）结合性「左到右」
    let source = "r = a / b / c";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层 / 的左操作数才是内层 a / b
    let (lhs, rhs) = expect_bin_op(&expr, BinOp::Div, "§2.2 级 4 左结合：外层必须是 /", source);
    let (inner_lhs, inner_rhs) = expect_bin_op(
        lhs,
        BinOp::Div,
        "§2.2 级 4 左结合：外层左操作数必须是内层 a / b",
        source,
    );
    expect_var(inner_lhs, "a", "§2.2 级 4 左结合：内层左操作数", source);
    expect_var(inner_rhs, "b", "§2.2 级 4 左结合：内层右操作数", source);
    expect_var(
        rhs,
        "c",
        "§2.2 级 4 左结合：外层右操作数必须是最右端的 c",
        source,
    );
}

#[test]
fn test_mul_binds_tighter_than_add() {
    // Arrange：a + b * c——SPEC syntax §2.2 级 4（乘除）高于级 5（加减）
    let source = "r = a + b * c";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是 +，右操作数整块是 b * c
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Add,
        "§2.2 级 4 高于级 5：外层必须是 +",
        source,
    );
    expect_var(lhs, "a", "§2.2 级 4 高于级 5：+ 左操作数", source);
    let (inner_lhs, inner_rhs) = expect_bin_op(
        rhs,
        BinOp::Mul,
        "§2.2 级 4 高于级 5：+ 右操作数必须是 b * c",
        source,
    );
    expect_var(inner_lhs, "b", "§2.2 级 4 高于级 5：* 左操作数", source);
    expect_var(inner_rhs, "c", "§2.2 级 4 高于级 5：* 右操作数", source);
}

#[test]
fn test_comparison_binds_looser_than_addition_on_both_operands() {
    // Arrange：a + b < c + d——SPEC syntax §2.2 级 9（比较）低于级 5（加减），
    // 两侧都须先收成算术表达式再比较
    let source = "r = a + b < c + d";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是比较，左右操作数各是完整的加法表达式
    let (lhs, rhs) = expect_bin_op(&expr, BinOp::Lt, "§2.2 级 9 低于级 5：外层必须是 <", source);
    let (left_sum_lhs, left_sum_rhs) = expect_bin_op(
        lhs,
        BinOp::Add,
        "§2.2 级 9 低于级 5：< 左操作数必须是 a + b",
        source,
    );
    expect_var(
        left_sum_lhs,
        "a",
        "§2.2 级 9 低于级 5：左加法左操作数",
        source,
    );
    expect_var(
        left_sum_rhs,
        "b",
        "§2.2 级 9 低于级 5：左加法右操作数",
        source,
    );
    let (right_sum_lhs, right_sum_rhs) = expect_bin_op(
        rhs,
        BinOp::Add,
        "§2.2 级 9 低于级 5：< 右操作数必须是 c + d",
        source,
    );
    expect_var(
        right_sum_lhs,
        "c",
        "§2.2 级 9 低于级 5：右加法左操作数",
        source,
    );
    expect_var(
        right_sum_rhs,
        "d",
        "§2.2 级 9 低于级 5：右加法右操作数",
        source,
    );
}

#[test]
fn test_shift_binds_tighter_than_bitwise_ops_on_both_operands() {
    // Arrange：SPEC syntax §2.2 级 7（<< >>）高于级 8（& | ^）——
    // 两个方向各一条：位运算在左、移位在右，以及移位在左、位运算在右
    let with_bit_on_left = "r = a & b << c";
    let with_shift_on_left = "r = a << b & c";

    // Act
    let bit_on_left = assigned_expr(with_bit_on_left);
    let shift_on_left = assigned_expr(with_shift_on_left);

    // Assert：两条都收成位移在内层的位运算
    let (lhs, rhs) = expect_bin_op(
        &bit_on_left,
        BinOp::BitAnd,
        "§2.2 级 7 高于级 8：外层是 &",
        with_bit_on_left,
    );
    expect_var(
        lhs,
        "a",
        "§2.2 级 7 高于级 8：位运算左操作数",
        with_bit_on_left,
    );
    let (shift_lhs, shift_rhs) = expect_bin_op(
        rhs,
        BinOp::Shl,
        "§2.2 级 7 高于级 8：& 右操作数必须是 b << c",
        with_bit_on_left,
    );
    expect_var(
        shift_lhs,
        "b",
        "§2.2 级 7 高于级 8：位移左操作数",
        with_bit_on_left,
    );
    expect_var(
        shift_rhs,
        "c",
        "§2.2 级 7 高于级 8：位移右操作数",
        with_bit_on_left,
    );

    let (shift_lhs, rhs) = expect_bin_op(
        &shift_on_left,
        BinOp::BitAnd,
        "§2.2 级 7 高于级 8：外层仍是 &",
        with_shift_on_left,
    );
    let (inner_lhs, inner_rhs) = expect_bin_op(
        shift_lhs,
        BinOp::Shl,
        "§2.2 级 7 高于级 8：& 左操作数必须是 a << b",
        with_shift_on_left,
    );
    expect_var(
        inner_lhs,
        "a",
        "§2.2 级 7 高于级 8：位移左操作数",
        with_shift_on_left,
    );
    expect_var(
        inner_rhs,
        "b",
        "§2.2 级 7 高于级 8：位移右操作数",
        with_shift_on_left,
    );
    expect_var(
        rhs,
        "c",
        "§2.2 级 7 高于级 8：& 右操作数",
        with_shift_on_left,
    );
}

#[test]
fn test_bitwise_level_is_left_associative_across_operators() {
    // Arrange：a | b ^ c——SPEC syntax §2.2 级 8 的 & | ^ 同层、结合性「左到右」
    let source = "r = a | b ^ c";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是后出现的 ^，其左操作数是先收成的 a | b
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::BitXor,
        "§2.2 级 8 左结合：同层后出现的 ^ 在外层",
        source,
    );
    let (inner_lhs, inner_rhs) = expect_bin_op(
        lhs,
        BinOp::BitOr,
        "§2.2 级 8 左结合：^ 左操作数必须是 a | b",
        source,
    );
    expect_var(inner_lhs, "a", "§2.2 级 8 左结合：| 左操作数", source);
    expect_var(inner_rhs, "b", "§2.2 级 8 左结合：| 右操作数", source);
    expect_var(rhs, "c", "§2.2 级 8 左结合：^ 右操作数", source);
}

#[test]
fn test_range_upper_bound_swallows_addition() {
    // Arrange：0..n + 2——SPEC syntax §2.2 的 Range 绑定力注（.. 左 6 低于加法级，
    // 右操作数按加法级解析 ⇒ 上界是算术表达式）；§2.12 RangeExpr ::= Expr '..' Expr
    let source = "r = 0..n + 2";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是 ..，上界整块是 n + 2
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Range,
        "§2.2 Range 注：外层必须是 ..（上界吞加法）",
        source,
    );
    expect_int_lit(lhs, 0, "§2.2 Range 注：下界保持字面量 0", source);
    let (upper_lhs, upper_rhs) = expect_bin_op(
        rhs,
        BinOp::Add,
        "§2.2 Range 注：上界必须是 n + 2（若上界只收 n，+ 2 会跑出 Range）",
        source,
    );
    expect_var(upper_lhs, "n", "§2.2 Range 注：上界加法左操作数", source);
    expect_int_lit(upper_rhs, 2, "§2.2 Range 注：上界加法右操作数", source);
}

#[test]
fn test_range_chain_is_left_associative() {
    // Arrange：a..b..c——SPEC syntax §2.2 Range 绑定力注（step 形态 (a..b)..c）
    let source = "r = a..b..c";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层 .. 的左操作数才是内层 a..b（右结合会得到 Range 套 Range 的另一侧）
    let (lhs, rhs) = expect_bin_op(&expr, BinOp::Range, "§2.2 Range 注：外层必须是 ..", source);
    let (inner_lhs, inner_rhs) = expect_bin_op(
        lhs,
        BinOp::Range,
        "§2.2 Range 注：左操作数必须是内层 a..b（step 形态的左结合）",
        source,
    );
    expect_var(inner_lhs, "a", "§2.2 Range 注：内层下界", source);
    expect_var(inner_rhs, "b", "§2.2 Range 注：内层上界", source);
    expect_var(rhs, "c", "§2.2 Range 注：外层上界是步长分量 c", source);
}

#[test]
fn test_lambda_body_is_right_greedy() {
    // Arrange：(x) => x + 1——SPEC syntax §2.10 Lambda ::= '(' ParamList? ')' '=>' Expr，
    // 体是完整 Expr，故 => 右侧的全部表达式都属体（右贪婪），不参与外层运算
    let source = "r = (x) => x + 1";

    // Act
    let expr = assigned_expr(source);

    // Assert：整个右侧是 lambda，且体内是完整的 x + 1
    let (params, body) = expect_lambda(&expr, "§2.10：赋值右侧整体必须是 lambda", source);
    assert_eq!(
        params.len(),
        1,
        "§2.10：单形参 lambda 应恰有一个形参（源码: {source:?}）"
    );
    assert_eq!(
        params[0].name, "x",
        "§2.10：形参名应为 x（源码: {source:?}）"
    );
    let returned = expect_single_return(body, "§2.10：lambda 体应以 return 收尾", source);
    let (lhs, rhs) = expect_bin_op(
        returned,
        BinOp::Add,
        "§2.10 右贪婪：如果体只收 x，+ 1 会落在 lambda 之外",
        source,
    );
    expect_var(lhs, "x", "§2.10 右贪婪：体内加法左操作数", source);
    expect_int_lit(rhs, 1, "§2.10 右贪婪：体内加法右操作数", source);
}

#[test]
fn test_assignment_is_right_associative() {
    // Arrange：(a = b = c)——SPEC syntax §2.2 级 12（=）结合性「右到左」；
    // 括号让它落在表达式位，避免与语句位绑定的 r = ... 混淆
    let source = "r = (a = b = c)";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层 = 的右操作数才是内层 b = c
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Assign,
        "§2.2 级 12 右结合：外层必须是 =",
        source,
    );
    expect_var(lhs, "a", "§2.2 级 12 右结合：外层左操作数", source);
    let (inner_lhs, inner_rhs) = expect_bin_op(
        rhs,
        BinOp::Assign,
        "§2.2 级 12 右结合：外层右操作数必须是内层 b = c",
        source,
    );
    expect_var(inner_lhs, "b", "§2.2 级 12 右结合：内层左操作数", source);
    expect_var(inner_rhs, "c", "§2.2 级 12 右结合：内层右操作数", source);
}

#[test]
fn test_cast_target_stops_at_call_suffix_boundary() {
    // Arrange：f(x) as Int——SPEC syntax §2.2 级 1（() [] .）高于级 2（as）：
    // 转换的操作数是整条调用链，目标类型是单个类型名
    let source = "r = f(x) as Int";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是转换，操作数是调用、目标类型是类型名 Int
    match &expr {
        Expr::Cast {
            expr, target_type, ..
        } => {
            match expr.as_ref() {
                Expr::Call { func, args, .. } => {
                    expect_var(func, "f", "§2.2 级 1 高于级 2：转换的操作数是整条调用", source);
                    assert_eq!(
                        args.len(),
                        1,
                        "§2.2 级 1 高于级 2：f(x) 应恰有一个实参（源码: {source:?}）"
                    );
                    expect_var(&args[0], "x", "§2.2 级 1 高于级 2：调用实参", source);
                }
                other => panic!(
                    "§2.2 级 1 高于级 2：转换操作数必须是调用 f(x)，实际 {other:?}（源码: {source:?}）"
                ),
            }
            assert!(
                matches!(target_type, Type::Name { name, .. } if name == "Int"),
                "§2.2 级 1 高于级 2：转换目标必须是类型名 Int，实际 {target_type:?}（源码: {source:?}）"
            );
        }
        other => panic!("§2.2 级 1 高于级 2：外层必须是转换，实际 {other:?}（源码: {source:?}）"),
    }
}

#[test]
fn test_try_postfix_binds_tighter_than_addition() {
    // Arrange：f(x)? + 1——SPEC syntax §2.11「? 是后缀运算符，优先级与 . 同级」，
    // 即 SPEC §2.2 级 1，高于级 5（加法）
    let source = "r = f(x)? + 1";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是 +，左操作数整块是 f(x)?
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Add,
        "§2.11 级 1 高于级 5：外层必须是 +",
        source,
    );
    expect_int_lit(rhs, 1, "§2.11：+ 右操作数", source);
    match lhs {
        Expr::Try { expr, .. } => match expr.as_ref() {
            Expr::Call { func, args, .. } => {
                expect_var(func, "f", "§2.11：? 作用在调用结果上", source);
                assert_eq!(
                    args.len(),
                    1,
                    "§2.11：f(x) 应恰有一个实参（源码: {source:?}）"
                );
                expect_var(&args[0], "x", "§2.11：调用实参", source);
            }
            other => panic!("§2.11：? 的操作数必须是调用 f(x)，实际 {other:?}（源码: {source:?}）"),
        },
        other => panic!("§2.11：+ 左操作数必须是 f(x)?，实际 {other:?}（源码: {source:?}）"),
    }
}

#[test]
fn test_call_chain_postfix_binds_tighter_than_addition() {
    // Arrange：a.b(c)[0] + 1——SPEC syntax §2.2 级 1（() [] .）左到右、
    // 高于级 5（加法）；§2.3 / §2.4 / §2.5 分别是调用、成员访问、索引访问
    let source = "r = a.b(c)[0] + 1";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是 +，左操作数整条后缀链收成 a.b(c)[0]
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Add,
        "§2.2 级 1 高于级 5：外层必须是 +",
        source,
    );
    expect_int_lit(rhs, 1, "§2.2 级 1：+ 右操作数", source);
    match lhs {
        Expr::Index { expr, index, .. } => {
            expect_int_lit(index, 0, "§2.2 级 1：索引下标", source);
            match expr.as_ref() {
                Expr::Call { func, args, .. } => {
                    assert_eq!(
                        args.len(),
                        1,
                        "§2.2 级 1：a.b(c) 应恰有一个实参（源码: {source:?}）"
                    );
                    expect_var(&args[0], "c", "§2.2 级 1：调用实参", source);
                    match func.as_ref() {
                        Expr::FieldAccess { expr, field, .. } => {
                            expect_var(expr, "a", "§2.2 级 1：成员访问的接收者", source);
                            assert_eq!(field, "b", "§2.2 级 1：成员名应为 b（源码: {source:?}）");
                        }
                        other => panic!(
                            "§2.2 级 1：调用目标必须是 a.b，实际 {other:?}（源码: {source:?}）"
                        ),
                    }
                }
                other => panic!(
                    "§2.2 级 1：索引目标必须是调用 a.b(c)，实际 {other:?}（源码: {source:?}）"
                ),
            }
        }
        other => {
            panic!("§2.2 级 1：+ 左操作数必须是 a.b(c)[0]，实际 {other:?}（源码: {source:?}）")
        }
    }
}

#[test]
fn test_pattern_or_chain_collects_all_alternatives() {
    // Arrange：1 | 2 | 3 三备选——SPEC syntax §2.8
    // MatchArm ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','（臂尾逗号是文法的一部分）
    let source = r#"r = match x { 1 | 2 | 3 -> "n", _ -> "o", }"#;

    // Act
    let expr = assigned_expr(source);

    // Assert：首臂是三项或模式（收链成一条 Or，而非嵌套或模式/位运算）
    let arms = match &expr {
        Expr::Match { arms, .. } => arms,
        other => panic!("§2.8：夹具右侧必须是 match，实际 {other:?}（源码: {source:?}）"),
    };
    assert_eq!(arms.len(), 2, "§2.8：夹具应有两臂（源码: {source:?}）");
    let alternatives = match &arms[0].pattern {
        Pattern::Or(alternatives) => alternatives,
        other => panic!("§2.8：首臂必须是或模式 1 | 2 | 3，实际 {other:?}（源码: {source:?}）"),
    };
    assert_eq!(
        alternatives.len(),
        3,
        "§2.8：或模式应收齐三个备选（源码: {source:?}）"
    );
    assert!(
        matches!(&alternatives[0], Pattern::Literal(Literal::Int(1))),
        "§2.8：首个备选应为字面量 1，实际 {:?}（源码: {source:?}）",
        alternatives[0]
    );
    assert!(
        matches!(&alternatives[1], Pattern::Literal(Literal::Int(2))),
        "§2.8：第二个备选应为字面量 2，实际 {:?}（源码: {source:?}）",
        alternatives[1]
    );
    assert!(
        matches!(&alternatives[2], Pattern::Literal(Literal::Int(3))),
        "§2.8：第三个备选应为字面量 3，实际 {:?}（源码: {source:?}）",
        alternatives[2]
    );
}

#[test]
fn test_grouped_sub_expression_overrides_left_associativity() {
    // Arrange：a - (b - c)——SPEC syntax §2.1 表达式分类的括号分组（§2.2 的
    // 结合性规则只在未加括号处生效）。本用例同时给 a - b - c 那条断言作**区分力对照**：
    // 同一断言面在两种分组下给出不同形状，说明左结合断言不是恒真
    let source = "r = a - (b - c)";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层 - 的右操作数才是内层 b - c
    let (lhs, rhs) = expect_bin_op(&expr, BinOp::Sub, "§2.1 括号分组：外层必须是 -", source);
    expect_var(lhs, "a", "§2.1 括号分组：外层左操作数", source);
    let (inner_lhs, inner_rhs) = expect_bin_op(
        rhs,
        BinOp::Sub,
        "§2.1 括号分组：外层右操作数必须是括号内的 b - c",
        source,
    );
    expect_var(inner_lhs, "b", "§2.1 括号分组：内层左操作数", source);
    expect_var(inner_rhs, "c", "§2.1 括号分组：内层右操作数", source);
}

/// SPEC syntax §2.2 级 2 高于级 4：`as` 的目标类型不得吞掉其后的乘号。
///
/// 病史：09a §P8 8.8 六 D1——修复前 `x as Int * y` 收成
/// `Cast { target_type: ConstExpr(Int * y) }`（值级中缀整条进了类型位）。
#[test]
fn test_cast_target_does_not_swallow_following_multiplication() {
    // Arrange
    let source = "r = x as Int * y";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是乘法；左操作数是转换（目标类型是单个 Int），右操作数是 y
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Mul,
        "§2.2 级 2 高于级 4：as 不得吞掉其后的 *",
        source,
    );
    match lhs {
        Expr::Cast {
            expr: inner,
            target_type,
            ..
        } => {
            expect_var(inner, "x", "§2.2 级 2：转换操作数", source);
            assert!(
                matches!(target_type, Type::Name { name, .. } if name == "Int"),
                "§2.6 TypeCast 只吃一个类型：目标类型必须是 Int，实际 {target_type:?}（源码: {source:?}）"
            );
        }
        other => panic!(
            "§2.2 级 2 高于级 4：左操作数必须是转换 x as Int，实际 {other:?}（源码: {source:?}）"
        ),
    }
    expect_var(rhs, "y", "§2.2 级 2 高于级 4：乘法右操作数", source);
}

/// SPEC syntax §2.2 级 2 高于级 4：`x * y as Int` 的转换只包住 y。
///
/// 病史：09a §P8 8.8 六 D2——修复前收成 `Cast(Mul(x, y), Int)`。
#[test]
fn test_cast_binds_tighter_than_multiplication_on_the_right_operand() {
    // Arrange
    let source = "r = x * y as Int";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层乘法；右操作数是转换（操作数 y）
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Mul,
        "§2.2 级 2 高于级 4：转换必须紧贴右操作数",
        source,
    );
    expect_var(lhs, "x", "§2.2 级 2 高于级 4：乘法左操作数", source);
    match rhs {
        Expr::Cast {
            expr: inner,
            target_type,
            ..
        } => {
            expect_var(inner, "y", "§2.2 级 2：转换操作数", source);
            assert!(
                matches!(target_type, Type::Name { name, .. } if name == "Int"),
                "§2.6 TypeCast 只吃一个类型：目标类型必须是 Int，实际 {target_type:?}（源码: {source:?}）"
            );
        }
        other => panic!(
            "§2.2 级 2 高于级 4：右操作数必须是转换 y as Int，实际 {other:?}（源码: {source:?}）"
        ),
    }
}

/// SPEC syntax §2.2 级 9：`==` `!=` `<` `>` `<=` `>=` 同级、左到右。
///
/// 病史：09a §P8 8.8 六 D3——修复前 `1 == 2 < 3` 收成 `Eq(1, Lt(2, 3))`
/// （相等层比比较层松，与「同级左结合」相反）。
#[test]
fn test_equality_and_comparison_share_one_left_associative_level() {
    // Arrange：两种顺序都必须在同一级上左结合
    let equal_first = "r = (1 == 2 < 3)";
    let less_first = "r = (1 < 2 == 3)";

    // Act
    let equal_first_expr = assigned_expr(equal_first);
    let less_first_expr = assigned_expr(less_first);

    // Assert：`1 == 2 < 3` → Lt(Eq(1,2), 3)
    let (lhs, rhs) = expect_bin_op(
        &equal_first_expr,
        BinOp::Lt,
        "§2.2 级 9 同级左结合：外层必须是右侧的 <",
        equal_first,
    );
    let (inner_lhs, inner_rhs) = expect_bin_op(
        lhs,
        BinOp::Eq,
        "§2.2 级 9 同级左结合：左操作数必须是先算的 ==",
        equal_first,
    );
    expect_int_lit(inner_lhs, 1, "§2.2 级 9：== 左操作数", equal_first);
    expect_int_lit(inner_rhs, 2, "§2.2 级 9：== 右操作数", equal_first);
    expect_int_lit(rhs, 3, "§2.2 级 9：< 右操作数", equal_first);

    // Assert：`1 < 2 == 3` → Eq(Lt(1,2), 3)
    let (lhs, rhs) = expect_bin_op(
        &less_first_expr,
        BinOp::Eq,
        "§2.2 级 9 同级左结合：外层必须是右侧的 ==",
        less_first,
    );
    let (inner_lhs, inner_rhs) = expect_bin_op(
        lhs,
        BinOp::Lt,
        "§2.2 级 9 同级左结合：左操作数必须是先算的 <",
        less_first,
    );
    expect_int_lit(inner_lhs, 1, "§2.2 级 9：< 左操作数", less_first);
    expect_int_lit(inner_rhs, 2, "§2.2 级 9：< 右操作数", less_first);
    expect_int_lit(rhs, 3, "§2.2 级 9：== 右操作数", less_first);
}

/// SPEC syntax §2.6 `TypeCast ::= Expr 'as' TypeExpr`：`&mut T` 形态的目标类型
/// 同样只能吃一个类型——`* y` 属于外层乘法（D1 的引用形态）。
///
/// 病史：09a §P8 8.8 六 D1；复审 t29 F3——修复前 `x as &mut Int * y` 收成
/// `Cast { target_type: Ref { mutable: true, inner: ConstExpr(Int * y) } }`。
#[test]
fn test_cast_to_mut_ref_target_does_not_swallow_multiplication() {
    // Arrange
    let source = "r = x as &mut Int * y";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层是乘法，左操作数是转换，目标类型是 &mut Int
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Mul,
        "§2.6 目标类型不得吞其后中缀：&mut Int 之后必须回到乘法",
        source,
    );
    match lhs {
        Expr::Cast {
            expr: inner,
            target_type,
            ..
        } => {
            expect_var(inner, "x", "§2.6：转换操作数", source);
            match target_type {
                Type::Ref {
                    mutable: true,
                    inner: inner_ty,
                    ..
                } => assert!(
                    matches!(inner_ty.as_ref(), Type::Name { name, .. } if name == "Int"),
                    "§2.6：&mut 的内层必须是 Int，实际 {inner_ty:?}（源码: {source:?}）"
                ),
                other => {
                    panic!("§2.6：目标类型必须是 &mut Int，实际 {other:?}（源码: {source:?}）")
                }
            }
        }
        other => panic!("§2.6：左操作数必须是转换，实际 {other:?}（源码: {source:?}）"),
    }
    expect_var(rhs, "y", "§2.6：乘法右操作数", source);
}

/// SPEC syntax §2.6：函数类型 cast 目标的返回位同样不吞其后中缀。
///
/// 病史：复审 t29 F3——修复前 `x as (a: Int) -> Int * y` 的返回位吞掉 `* y`。
#[test]
fn test_cast_to_fn_type_target_does_not_swallow_multiplication() {
    // Arrange
    let source = "r = x as (a: Int) -> Int * y";

    // Act
    let expr = assigned_expr(source);

    // Assert：外层乘法；左操作数是转换，目标类型是函数类型
    let (lhs, rhs) = expect_bin_op(
        &expr,
        BinOp::Mul,
        "§2.6 目标类型不得吞其后中缀：函数类型返回位之后必须回到乘法",
        source,
    );
    match lhs {
        Expr::Cast {
            expr: inner,
            target_type,
            ..
        } => {
            expect_var(inner, "x", "§2.6：转换操作数", source);
            match target_type {
                Type::Fn { return_type, .. } => assert!(
                    matches!(return_type.as_ref(), Type::Name { name, .. } if name == "Int"),
                    "§2.6：函数类型的返回位必须是 Int，实际 {return_type:?}（源码: {source:?}）"
                ),
                other => panic!("§2.6：目标类型必须是函数类型，实际 {other:?}（源码: {source:?}）"),
            }
        }
        other => panic!("§2.6：左操作数必须是转换，实际 {other:?}（源码: {source:?}）"),
    }
    expect_var(rhs, "y", "§2.6：乘法右操作数", source);
}

/// 「表行被消费」的变异锚：赋值的算子必须同时等于语义常量与表行声明。
///
/// 若 parse_assign 退回硬编 `BinOp::Assign`（不经表），把表行 op 改成别的算子时
/// 第二条断言会红——这正是复审 t29 F1 要求的「经生产 parse 路径的消费证据」。
#[test]
fn test_assign_infix_operator_is_consumed_from_the_table() {
    // Arrange：`r = a = b` 的右侧是表达式位赋值；表行声明是唯一期望源
    use crate::frontend::core::lexer::tokens::TokenKind;
    use crate::frontend::core::parser::pratt::precedence::infix_ast_op;
    let table_op = infix_ast_op(&TokenKind::Eq).expect("表必须声明 = 的算子");
    let source = "r = a = b";

    // Act
    let expr = assigned_expr(source);

    // Assert：生产 parse 产出的算子 = 语义常量 = 表行声明（两向都钉住）
    let Expr::BinOp { op, left, .. } = &expr else {
        panic!("表达式位赋值必须产 BinOp，实际 {expr:?}（源码: {source:?}）");
    };
    assert_eq!(
        *op,
        BinOp::Assign,
        "表达式位赋值必须产 Assign（源码: {source:?}）"
    );
    assert_eq!(
        *op, table_op,
        "生产 parse 的算子必须等于表行声明——表行被消费（源码: {source:?}）"
    );
    expect_var(left, "a", "赋值左操作数", source);
}
