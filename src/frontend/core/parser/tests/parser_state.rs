//! Tests for ParserState: helpers, error tracking, save/restore.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{NullProbe, ParserState};
use crate::util::plan_id::PlanId;
use crate::frontend::core::lexer::tokens::TokenKind;
use crate::frontend::core::parser::parse_msg;

fn with_state<F>(
    source: &str,
    mut f: F,
) where
    F: FnMut(&mut ParserState<'_>),
{
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens, &NullProbe);
    f(&mut state);
}

#[test]
fn test_new_parser_state() {
    with_state("", |state| {
        assert!(state.at_end());
    });
}

#[test]
fn test_current_returns_first_token() {
    with_state("42", |state| {
        let tok = state.current();
        assert!(tok.is_some());
        assert_eq!(&tok.unwrap().kind, &TokenKind::IntLiteral(42));
    });
}

#[test]
fn test_peek_returns_second_token() {
    with_state("1 + 2", |state| {
        let peeked = state.peek();
        assert!(peeked.is_some());
        assert_eq!(&peeked.unwrap().kind, &TokenKind::Plus);
    });
}

#[test]
fn test_at_matches() {
    with_state("+", |state| {
        assert!(state.at(&TokenKind::Plus));
    });
}

#[test]
fn test_error_tracking() {
    with_state("x", |state| {
        state.error(parse_msg(state, "test error"));
        assert!(state.has_errors());
        assert_eq!(state.error_count(), 1);
    });
}

#[test]
fn test_save_restore_position() {
    with_state("a b c", |state| {
        state.bump(); // consume 'a'
        let saved = state.save_position();
        state.bump(); // consume 'b'
        state.restore_position(saved);
        assert_eq!(
            &state.current().unwrap().kind,
            &TokenKind::Identifier("b".into())
        );
    });
}

#[test]
fn test_nested_save_restore() {
    with_state("1 2 3", |state| {
        let s1 = state.save_position();
        state.bump();
        let s2 = state.save_position();
        state.bump();
        state.restore_position(s2);
        assert_eq!(&state.current().unwrap().kind, &TokenKind::IntLiteral(2));
        state.restore_position(s1);
        assert_eq!(&state.current().unwrap().kind, &TokenKind::IntLiteral(1));
    });
}

#[test]
fn test_can_start_stmt_semicolon() {
    with_state(";", |state| {
        assert!(!state.can_start_stmt());
    });
}

// ── 释放计划身份分配（RFC-039 D20 / WBS 4.5.1） ──────────────────

/// 合成语句（不经解析器）恒为未分配身份——它不属任何解析单元，也不构成释放计划义务。
#[test]
fn test_unplanned_statement_has_no_plan_id() {
    // Arrange：手工构造的合成语句（ir_gen 内建 / 直驱夹具同款）
    use crate::ast::{Expr, Stmt, StmtKind};
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Lit(
            crate::frontend::core::lexer::tokens::Literal::Int(1),
            crate::util::span::Span::dummy(),
        ))),
        span: crate::util::span::Span::dummy(),
        plan_id: PlanId::UNPLANNED,
    };

    // Act
    let planned = stmt.plan_id.is_planned();

    // Assert
    assert!(!planned, "合成语句不得持有已分配身份: {:?}", stmt.plan_id);
    assert_eq!(
        PlanId::UNPLANNED.to_string(),
        "plan#unplanned",
        "未分配身份的显示形态是诊断可读性的一部分"
    );
}

// ==================== 解析递归深度闸（issue #540） ====================
//
// 规范来源：RFC-013 错误码规范 E0019（表达式嵌套过深）+ WBS 3.20.30
// （issue #540：深嵌套表达式此前让编译器栈溢出崩溃 0xC00000FD 零诊断，
// D59 C 类崩溃面直修）+ 用户裁决（大栈线程消解 razor + 解析深度闸
// MAX_PARSE_DEPTH=1024）。
//
// 闸值与通过区的登记依据见 parser_state.rs 的 MAX_PARSE_DEPTH 文档；
// 深夹具一律放测试目录（本文件），不进语料（IR/语料差分零漂移）。

use crate::frontend::core::parser::parse;

/// 在生产同款栈大小上解析（编译线程栈常量单源 `COMPILE_STACK_SIZE`）。
///
/// 解析闸允许的最大解析峰值（闸值 1024 层 × 每层帧成本）超过测试线程
/// 默认栈，镜像 CLI 编排环境（main.rs 大栈线程）是这些夹具的运行前提。
fn parse_on_production_stack(source: String) -> crate::frontend::core::parser::ParseResult {
    std::thread::Builder::new()
        .stack_size(crate::COMPILE_STACK_SIZE)
        .spawn(move || {
            let tokens = tokenize(&source).expect("fixture must lex");
            parse(&tokens)
        })
        .expect("spawn parse thread")
        .join()
        .expect("parse thread must not panic")
}

fn error_codes(result: &crate::frontend::core::parser::ParseResult) -> Vec<String> {
    result.errors.iter().map(|d| d.code.clone()).collect()
}

#[test]
fn test_parse_deep_unary_chain_reports_single_e0019() {
    // Arrange：1100 层一元链（闸值 1024，修前 1MB 栈在 282 层即崩溃）
    let expr = format!("{}1", "- ".repeat(1100));
    let source = format!("main = () => {{\n    y = {expr}\n    print(y)\n}}\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert：恰 1 条诊断且为 E0019（拒绝点语义：指向超限的嵌套起点）
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深一元链必须报 E0019");
}

#[test]
fn test_parse_deep_call_nesting_reports_single_e0019() {
    // Arrange：1100 层调用嵌套。回归钉：拒绝残骸（未闭合的 `)` 序列）
    // 曾被块体恢复循环逐 token 重解析，产生 511 条 E0011 级联——
    // drain_broken_construct 的括号余额清理正是为它存在。
    let expr = format!("{}1{}", "id(".repeat(1100), ")".repeat(1100));
    let source = format!("id: (Int) -> Int = (x) => x\nmain = () => {{\n    y = {expr}\n}}\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断（禁止 E0011 级联）: {codes:?}");
    assert_eq!(codes[0], "E0019", "深调用嵌套必须报 E0019");
}

#[test]
fn test_parse_deep_block_nesting_reports_single_e0019() {
    // Arrange：1100 层块嵌套（闸值上曾无限打转：入口拒绝路径漏跑
    // 残骸清理，synchronize 的语句起点启发式零推进）
    let source = format!("main = () => {}1{}\n", "{ ".repeat(1100), " }".repeat(1100));

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断（禁止恢复级联）: {codes:?}");
    assert_eq!(codes[0], "E0019", "深块嵌套必须报 E0019");
}

#[test]
fn test_parse_deep_paren_grouping_reports_single_e0019() {
    // Arrange：1100 层括号分组——解析真递归但 AST 折叠为单层，
    // 是闸下限（括号 600 层通过）的登记形态
    let expr = format!("{}1{}", "(".repeat(1100), ")".repeat(1100));
    let source = format!("main = () => {{\n    y = {expr}\n}}\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深括号分组必须报 E0019");
}

#[test]
fn test_parse_block_chain_1000_below_gate_parses_clean() {
    // Arrange：1000 层块嵌套——闸值（1024）之下，修前在 1MB 栈上崩溃、
    // 大栈线程修后可编译的形态；闸不得触及它（原先成功程序逐字不变）
    let source = format!("main = () => {}1{}\n", "{ ".repeat(1000), " }".repeat(1000));

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert!(codes.is_empty(), "闸下形态必须零诊断: {codes:?}");
}

#[test]
fn test_parse_flat_binop_chain_bypasses_depth_gate() {
    // Arrange：3000 项扁平二元链——左结合中缀在 parser 是**迭代**（栈深
    // 有界），不经解析闸；它的资源防线是 IR 层既有 E3014 寄存器上限
    // （登记边界：解析闸不覆盖 parser 扁平形态）。本测试钉住这个设计
    // 决策：解析闸不得误伤迭代形态。
    let expr = "1 + ".repeat(3000) + "1";
    let source = format!("main = () => {{\n    y = {expr}\n}}\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert!(
        !codes.contains(&"E0019".to_string()),
        "扁平二元链不经解析闸（迭代形态）: {codes:?}"
    );
}
