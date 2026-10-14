//! #452 复审 F1：命名实参调用**编译期必响亮**（先红后绿判据）
//!
//! 规范来源:
//! - 用户 2026-10-14 裁决⑥（重载候选选择必须把命名实参计入匹配，禁止静默择一）
//! - #452 自订契约（冲突/缺参一律编译期报错）
//! - 批次 t9 / t3 复审 F1（多候选零命中时命名实参被静默接受）
//!
//! 红态（改动前，主仓真实序列实测）：含命名实参的方法调用在**决议零命中**时
//! 整段跳过元数与逐参类型检查——`check` exit=0、`run` 打印错值或 void；
//! 同调用点的位置实参形态却正确报 E1002/E1010（形态不对称）。
//!
//! 本文件把这些形态收成**仓内夹具**（外部探针包不入库），逐条断言编译期错误码。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;

/// 编译期诊断码集合（真实序列：lex → parse → typecheck）。
fn compile_codes(source: &str) -> Vec<String> {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<named-args-loud>");
    let tr = checker.check_module(&parsed.module);
    for d in tr.diagnostics.iter() {
        eprintln!("LOUD_DIAG {} :: {}", d.code, d.message);
    }
    tr.diagnostics.iter().map(|d| d.code.clone()).collect()
}

/// 双候选夹具（Int / String 首值形参）：命名实参与位置实参都要能被判成不相容。
const TWO_CANDS: &str = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, n: Int, a: Int) -> Int = (self, n, a) => { return n * 100 + a }
Temp.pick: (self: &Temp, n: String, a: Int) -> Int = (self, n, a) => { return a }
"#;

/// 单候选夹具：元数唯一，形参名可判定。
const ONE_CAND: &str = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, n: Int, a: Int) -> Int = (self, n, a) => { return n * 100 + a }
"#;

/// 编译期错误码判据表：(用例名, 夹具头, main 体, 期望码)
const CASES: &[(&str, &str, &str, &str)] = &[
    // ── F1 红态四探针（多候选零命中：命名实参类型错 / 缺参；带注解变体同静默）
    (
        "named_wrong_type_two_cands",
        TWO_CANDS,
        "return t.pick(n = true, a = 2)",
        "E1002",
    ),
    (
        "named_missing_arg_two_cands",
        TWO_CANDS,
        "r: Int = t.pick(a = 2)\n    print(r)\n    return 0",
        "E1010",
    ),
    (
        "annotated_named_wrong_type_two_cands",
        TWO_CANDS,
        "r: Int = t.pick(n = true, a = 2)\n    print(r)\n    return r",
        "E1002",
    ),
    (
        "annotated_named_missing_two_cands",
        TWO_CANDS,
        "r: Int = t.pick(a = 2)\n    print(r)\n    return r",
        "E1010",
    ),
    // ── 单候选同族（同一管道收窄后的落点）
    (
        "named_wrong_type_one_cand_mixed",
        ONE_CAND,
        "r = t.pick(1, a = \"x\")\n    print(r)\n    return 0",
        "E1002",
    ),
    (
        "named_wrong_type_one_cand_named_only",
        ONE_CAND,
        "r = t.pick(n = \"x\", a = 2)\n    print(r)\n    return 0",
        "E1002",
    ),
    // ── 未知名 / 撞槽位 / 顺序非法：同一批错误码（E1014 / E1111）
    (
        "unknown_name_two_cands",
        TWO_CANDS,
        "r = t.pick(z = 1)\n    return 0",
        "E1014",
    ),
    (
        "mixed_extraneous_named_two_cands",
        TWO_CANDS,
        "r = t.pick(1, b = 2)\n    return 0",
        "E1014",
    ),
    (
        "slot_conflict_two_cands",
        TWO_CANDS,
        "r = t.pick(a = 2, 1)\n    return 0",
        "E1111",
    ),
];

/// 命名实参的每一种不相容形态都必须在**编译期**报错（不得静默接受）。
#[test]
fn test_named_args_incompatible_shapes_error_at_compile_time() {
    for (label, head, body, expected) in CASES {
        // Arrange
        let source =
            format!("{head}main: () -> Int = {{\n    t = Temp(1)\n    {body}\n}}\nmain()\n");

        // Act
        let codes = compile_codes(&source);

        // Assert
        assert!(
            codes.iter().any(|c| c == expected),
            "{label}: 期望编译期 {expected}，实际诊断 {codes:?}（静默接受即 F1 红态）"
        );
    }
}

/// 位置实参对照组（p4/p9/p10 形态）不得回退：元数/类型错仍报原码。
#[test]
fn test_positional_only_incompatible_shapes_still_error() {
    // Arrange
    let cases: &[(&str, &str, &str)] = &[
        ("positional_wrong_type", "return t.pick(true, 2)", "E1002"),
        ("positional_missing", "r = t.pick(1)\n    return 0", "E1010"),
        (
            "positional_extra",
            "r = t.pick(1, 2, 3)\n    return 0",
            "E1010",
        ),
    ];
    for (label, body, expected) in cases {
        let source =
            format!("{TWO_CANDS}main: () -> Int = {{\n    t = Temp(1)\n    {body}\n}}\nmain()\n");

        // Act
        let codes = compile_codes(&source);

        // Assert
        assert!(
            codes.iter().any(|c| c == expected),
            "{label}: 位置实参形态必须报 {expected}，实际 {codes:?}"
        );
    }
}

/// 命中面不得回退：形态正确的命名/混合/纯命名调用编译期零诊断。
#[test]
fn test_named_args_matching_shapes_stay_green() {
    // Arrange
    let cases: &[(&str, &str)] = &[
        ("positional", "return t.pick(1, 2)"),
        ("mixed", "return t.pick(1, a = 2)"),
        ("named_decl_order", "return t.pick(n = 1, a = 2)"),
        ("named_reversed", "return t.pick(a = 2, n = 1)"),
    ];
    for (label, body) in cases {
        let source =
            format!("{ONE_CAND}main: () -> Int = {{\n    t = Temp(1)\n    {body}\n}}\nmain()\n");

        // Act
        let codes = compile_codes(&source);

        // Assert
        assert!(
            codes.is_empty(),
            "{label}: 命中面必须零诊断（不得因收窄守卫而误报），实际 {codes:?}"
        );
    }
}
