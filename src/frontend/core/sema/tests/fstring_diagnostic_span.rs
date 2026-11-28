//! f-string 插值内诊断的码序列与位置钉（issue #574 / WBS 8.2.3 聚焦复审
//! F3）：「诊断集逐条不变，只允许位置变化」这一 D59 A 类授权面前提，此前
//! 只存在于提交信息的手工对比——本文件把它钉成自动化断言：多行 f-string
//! 插值内未知变量的诊断**码序列恰为 [E1001]**（无额外码混入），且诊断位置
//! 指到插值内容实际所在源行（> 1，不再 line=1）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;

/// 解析源码 → 模块（夹具自身错误必须显式 panic）。
fn parse_module(source: &str) -> crate::ast::Module {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("词法失败: {e:?}\n源码:\n{source}"));
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析失败: {:?}", parsed.errors);
    parsed.module
}

#[test]
fn test_multiline_fstring_interpolation_diagnostic_keeps_code_sequence_and_absolute_line() {
    // Arrange：**多行 f-string**——三引号 f-string 起于第 2 行，插值 {nope}
    // 内容在第 3 行（未知变量 nope）。修前该诊断 line=1（相对插值文本）；
    // 码序列若混入额外码即说明诊断集变化（本批只许动位置）。
    let source = "main: () -> Void = () => {\n    print(f\"\"\"\nx={nope}\n\"\"\")\n}\nmain()";

    // Act
    let module = parse_module(source);
    let mut checker = TypeChecker::new("fstring-span-probe");
    let result = checker.check_module(&module, &TypeLevelNamespace::scan_one(&module));

    // Assert：诊断码序列恰为 [E1001]（码/条数不变——本批只许动位置）
    let codes: Vec<String> = result.diagnostics.iter().map(|d| d.code.clone()).collect();
    assert!(
        codes == vec!["E1001".to_string()],
        "插值内未知变量的诊断码序列必须恰为 [E1001]，实际: {codes:?}"
    );

    // Assert：位置指到插值内容实际所在源行（第 3 行），不再 line=1
    let diag = result
        .diagnostics
        .first()
        .expect("码序列断言已保证至少一条诊断");
    let span = diag
        .span
        .as_ref()
        .expect("E1001 诊断必须携带 span（位置断言的前提）");
    assert!(
        span.start.line == 3,
        "插值内诊断必须指到源文件第 3 行（f-string 起于第 2 行、插值在第 3 行），实际 line={}",
        span.start.line
    );
}
