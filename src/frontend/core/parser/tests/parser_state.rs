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

#[test]
fn test_parse_deep_fstring_interpolation_reports_single_e0019() {
    // Arrange：f-string 内嵌 1100 层括号——外层语句深度 + 内层解析深度合计
    // 超闸（1024）。修前（#540 修复笔）内层闸拒绝推进的是内嵌解析实例
    // 自己的 errors，实例随「字面文本回退」被整体丢弃——程序编译成功、
    // 运行输出字面插值源文本（D59 C 类静默给错答案）。修后内层拒绝必须
    // 传导到外层诊断集。
    let expr = format!("{}1{}", "(".repeat(1100), ")".repeat(1100));
    let source = format!("main = () => {{\n    s = f\"{{{expr}}}\"\n    print(s)\n}}\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert：恰 1 条诊断且为 E0019（同因不得双诊断）
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深内插必须经内层闸传导 E0019: {codes:?}");
}

#[test]
fn test_parse_shallow_fstring_interpolation_still_interpolates() {
    // Arrange：浅内插对照——内层闸不触发，R1 传导路径不得误伤既有插值
    let source = "main = () => {\n    s = f\"{1 + 1}\"\n}\n";

    // Act
    let result = parse_on_production_stack(source.to_string());

    // Assert
    let codes = error_codes(&result);
    assert!(codes.is_empty(), "浅内插零变化: {codes:?}");
}

#[test]
fn test_parse_deep_generic_type_annotation_reports_single_e0019() {
    // Arrange：1100 层嵌套泛型构造类型 `A(A(…A(Int)…))`。实测登记（WBS
    // 3.20.30 t1r 探针，1MB 参照线程）：类型标注递归崩溃起点 595——闸上
    // 深度在 1MB 栈上全部崩溃，闸值必须拦在崩溃之前。修前类型递归不经
    // 解析闸（parser_state.rs 的两汇点不含它），闸上形态在生产 64MB 栈
    // 编译成功、无任何诊断（真递归栈膨胀无人拦）。
    let ty = format!("{}Int{}", "A(".repeat(1100), ")".repeat(1100));
    let source = format!("x: {ty} = 1\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert：恰 1 条诊断且为 E0019
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深类型标注必须报 E0019: {codes:?}");
}

#[test]
fn test_parse_deep_struct_type_annotation_reports_single_e0019() {
    // Arrange：1100 层嵌套 struct 类型 `{a: {a: …{a: Int}…}}`——实测登记
    // 同上，崩溃起点 386（类型面最深崩溃形态，帧成本最高）。
    let ty = format!("{}Int{}", "{a: ".repeat(1100), "}".repeat(1100));
    let source = format!("x: {ty} = 1\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深 struct 类型必须报 E0019: {codes:?}");
}

#[test]
fn test_parse_deep_ptr_type_annotation_reports_single_e0019() {
    // Arrange：1100 层裸指针链 `***…Int`——实测登记同上，崩溃起点 928。
    let ty = format!("{}Int", "*".repeat(1100));
    let source = format!("x: {ty} = 1\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深 Ptr 链必须报 E0019: {codes:?}");
}

#[test]
fn test_parse_deep_ref_type_annotation_reports_single_e0019() {
    // Arrange：1100 层 `&mut ` 引用链——实测登记同上，崩溃起点 930。
    let ty = format!("{}Int", "&mut ".repeat(1100));
    let source = format!("x: {ty} = 1\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深 Ref 链必须报 E0019: {codes:?}");
}

#[test]
fn test_parse_deep_fn_type_annotation_reports_single_e0019() {
    // Arrange：1100 层嵌套函数类型 `() -> () -> …Int`——实测登记同上，
    // 崩溃起点 930。
    let ty = format!("{}Int", "() -> ".repeat(1100));
    let source = format!("x: {ty} = 1\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深 Fn 类型必须报 E0019: {codes:?}");
}

#[test]
fn test_parse_deep_curry_signature_reports_single_e0019() {
    // Arrange：1100 层柯里组签名 `f: (x: Int) -> (x: Int) -> …Int`——
    // parse_fn_type_with_names 的 curry 组自递归同样不经解析闸。
    let ty = format!("{}Int", "(x: Int) -> ".repeat(1100));
    let source = format!("f: {ty} Int = 1\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深柯里组签名必须报 E0019: {codes:?}");
}

#[test]
fn test_parse_deep_pattern_nesting_reports_single_e0019() {
    // Arrange：1100 层嵌套 struct 模式 `P{a: P{a: …1…}}`（match 臂）——
    // parse_pattern 字段臂自递归每层的表达式探针 enter/exit 成对、不累积
    // 计数，模式递归栈不受闸保护（实测登记同上，1MB 崩溃起点 980）。
    let pat = format!("{}1{}", "P{a: ".repeat(1100), "}".repeat(1100));
    let source = format!("main = () => {{\n    x = match 1 {{ {pat} => 1, _ => 0 }}\n}}\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert_eq!(codes.len(), 1, "恰 1 条诊断: {codes:?}");
    assert_eq!(codes[0], "E0019", "深模式嵌套必须报 E0019: {codes:?}");
}

#[test]
fn test_parse_struct_type_800_below_gate_parses_clean() {
    // Arrange：800 层嵌套 struct 类型——闸值（1024）之下、生产 64MB 栈
    // 上修前即编译成功的形态（razor 对照：修前修后零变化；1MB 参照栈
    // 崩溃起点 386 之上由大栈线程兜底，与表达式面 block-1000 同构）。
    let ty = format!("{}Int{}", "{a: ".repeat(800), "}".repeat(800));
    let source = format!("x: {ty} = 1\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert!(codes.is_empty(), "闸下类型形态必须零诊断: {codes:?}");
}

#[test]
fn test_parse_pattern_800_below_gate_parses_clean() {
    // Arrange：800 层嵌套 struct 模式——闸下形态 razor 对照。
    let pat = format!("{}1{}", "P{a: ".repeat(800), "}".repeat(800));
    let source = format!("main = () => {{\n    x = match 1 {{ {pat} => 1, _ => 0 }}\n}}\n");

    // Act
    let result = parse_on_production_stack(source);

    // Assert
    let codes = error_codes(&result);
    assert!(codes.is_empty(), "闸下模式形态必须零诊断: {codes:?}");
}

#[test]
fn test_parse_deep_block_mid_body_reports_single_e0019() {
    // Arrange：main 体中部 1100 层深块 + 后续正常语句。修前实测留档
    // （WBS 3.20.30 t1r 探针）：E0019 ×3 级联——深块拒后 drain 停在余额
    // 归零的闭合 `}`，块体恢复循环的 synchronize 不识别 RBrace、把外层
    // 闭合逐个吃掉，后续语句在闸值上逐条重新进闸被拒、逐语句重发 E0019。
    let source = format!(
        "main = () => {{\n    {}1{}\n    y = 1 + 1\n    print(y)\n}}\n",
        "{ ".repeat(1100),
        " }".repeat(1100)
    );

    // Act
    let result = parse_on_production_stack(source);

    // Assert：恰 1 条 E0019（后续语句不得级联重发）
    let codes = error_codes(&result);
    assert_eq!(
        codes.iter().filter(|c| c.as_str() == "E0019").count(),
        1,
        "恰 1 条 E0019: {codes:?}"
    );
    assert_eq!(codes.len(), 1, "禁止同因级联与恢复噪声: {codes:?}");
}

#[test]
fn test_parse_shallow_block_recovery_still_reports_e0011_once() {
    // Arrange：块内普通语法错误（非深度闸，`::` 标签语法已移除的语句起点
    // 形态）——synchronize 是共享恢复机制，RBrace 停步点不得改变普通路径
    // 的既有诊断形态（E0011 指向肇事行）。
    let source = "main = () => {\n    break ::123\n    y = 1\n}\n";

    // Act
    let result = parse_on_production_stack(source.to_string());

    // Assert：E0011 恰 1 条（既有恢复面锚，形态同 error_recovery 的
    // break-with-label 锚）
    let codes = error_codes(&result);
    assert_eq!(
        codes.iter().filter(|c| c.as_str() == "E0011").count(),
        1,
        "普通恢复路径必须恰报 1 条 E0011: {codes:?}"
    );
}
