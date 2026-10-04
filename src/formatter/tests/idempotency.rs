//! Formatter 幂等性回归 — 原则 1（`docs/src/design/formatter/index.md:26`）：对已格式化的代码再次格式化，输出必须与输入完全相同。
//!
//! 历史缺陷（B1 调研实测）：注释区间上界用**行号闭区间**，于是「与语句同一行的行末注释」
//! 既被当作前导注释、又被当作行末注释输出 → 每格式化一次多一份副本，文件行数无界增长；
//! 另有「块内最后一条语句与闭合 `}` 同行时 prev_end_line 多 +1」→ 紧随其后的注释被删除。
//! 两类形态都收敛到同一个触发条件：**单行语句 + 同行行末注释**（与 lambda 无关）。
//!
//! 规范来源：`docs/src/dev/test-specification.md` 规则 1.1（单文件模块 → 父级 tests/）、
//! 规则 3.1（命名 `test_<what>_<scenario>`）、规则 4.1（Arrange / Act / Assert 三段）。

use crate::formatter::{format_source, FormatOptions};

/// 格式化一例输入；失败时给出输入全文，便于定位。
fn format_or_panic(source: &str) -> String {
    format_source(source, &FormatOptions::default())
        .unwrap_or_else(|e| panic!("用例输入应可格式化，实际失败：{e}\n--- 输入 ---\n{source}"))
}

/// 幂等性用例表：(场景, 输入)。
///
/// 覆盖历史触发形态：顶层绑定 / 块内语句 / 块内非首条 / lambda 块体末行 / 块起始行注释 /
/// lambda 作实参；并含两个对照（注释独占一行、块外注释 + 后续语句）必须保持稳定。
const IDEMPOTENT_CASES: &[(&str, &str)] = &[
    (
        "顶层 lambda 绑定 + 行末注释",
        "double = (x) => x * 2 // c\n",
    ),
    (
        "顶层多参 lambda 绑定 + 行末注释",
        "add = (a, b) => a + b // c\n",
    ),
    ("顶层语句 + 行末注释", "b = 3 // c\n"),
    (
        "块内赋值 + 行末注释",
        "main: () -> Void = {\n    b = 3 // c\n}\n",
    ),
    (
        "块内非首条赋值 + 行末注释",
        "main: () -> Void = {\n    a = 1\n    b = 3 // c\n}\n",
    ),
    (
        "块内 lambda 绑定 + 行末注释",
        "main: () -> Void = {\n    f = (x) => x * 2 // c\n}\n",
    ),
    (
        "lambda 块体末行 + 行末注释",
        "f = (a) => {\n    a + 1 // last\n}\n",
    ),
    (
        "块起始行注释（注释被移入块内）",
        "main: () -> Void = { // open\n    b = 3\n}\n",
    ),
    (
        "lambda 作实参 + 行末注释",
        "use std.list\n\nmain: () -> Void = {\n    mut r = list.map([1, 2], x => x * 2) // c\n}\n",
    ),
    (
        "块内注释独占行（对照：本来就稳定）",
        "main: () -> Void = {\n    // own line\n    b = 3\n}\n",
    ),
    (
        "块外注释 + 后续语句（对照：注释不得被删除）",
        "a: (Int) -> Int = (x) => {\n    return x\n}\n// keep me\nb: (Int) -> Int = (y) => y\n",
    ),
];

/// 注释守恒用例表：(场景, 输入, 注释标记, 期望输出中出现次数)。
const CONSERVATION_CASES: &[(&str, &str, &str, usize)] = &[
    (
        "顶层行末注释只出现一次",
        "double = (x) => x * 2 // c\n",
        "// c",
        1,
    ),
    (
        "块内行末注释只出现一次",
        "main: () -> Void = {\n    b = 3 // c\n}\n",
        "// c",
        1,
    ),
    (
        "块内非首条语句的行末注释只出现一次",
        "main: () -> Void = {\n    a = 1\n    b = 3 // c\n}\n",
        "// c",
        1,
    ),
    (
        "块外注释不因块折叠而丢失",
        "a: (Int) -> Int = (x) => {\n    return x\n}\n// keep me\nb: (Int) -> Int = (y) => y\n",
        "// keep me",
        1,
    ),
    (
        "块起始行注释在输出中保留一次",
        "main: () -> Void = { // open\n    b = 3\n}\n",
        "// open",
        1,
    ),
];

#[test]
fn test_format_second_pass_is_fixed_point_on_comment_cases() {
    for &(scenario, input) in IDEMPOTENT_CASES {
        // Arrange
        // （输入即样例；期望值来自「格式化是幂等的」这一规范原则，而非当前实现输出）

        // Act：对同一输入连续格式化三次
        let first = format_or_panic(input);
        let second = format_or_panic(&first);
        let third = format_or_panic(&second);

        // Assert：第一次之后即为不动点（不是「多跑几次才收敛」）
        assert_eq!(
            first, second,
            "第二次格式化改变了输出（不幂等）：{scenario}\n--- p1 ---\n{first}--- p2 ---\n{second}"
        );
        assert_eq!(
            second, third,
            "第三次格式化仍在改动（未收敛）：{scenario}\n--- p2 ---\n{second}--- p3 ---\n{third}"
        );
    }
}

#[test]
fn test_format_keeps_each_comment_exactly_once() {
    for &(scenario, input, marker, expected) in CONSERVATION_CASES {
        // Arrange
        // （期望次数由输入中该注释的出现次数决定：每处注释在输出中恰好保留一次）

        // Act
        let output = format_or_panic(input);

        // Assert
        assert_eq!(
            output.matches(marker).count(),
            expected,
            "注释 {marker} 应恰好出现 {expected} 次（丢失或重复）：{scenario}\n--- 输出 ---\n{output}"
        );
    }
}
