//! 类型判等的生产判定点（RFC-039 D62 / 6.9.3d，issue #535）
//!
//! 判据锚：RFC-027 §12.1（判定点先统一；统一失败后允许改用判等，但必须先回滚统一
//! 尝试产生的部分绑定）、RFC-011 §4.1（类型位常量的字面量形态）、RFC-039 D62 的
//! I2（纯判等）与 I7（接线真实，生产判定点而非单测可达）。
//!
//! 本文件是**生产入口直驱**测试：经 `Compiler::compile` 走 `Stage::Typecheck` 的真实
//! 判定点。单元级 `defeq` 调用不算证据（契约第 2 条：禁止以单测可达冒充用户可达）。

use crate::frontend::compiler::Compiler;

/// δ 别名（`Five: Type = 5`）在**调用实参位**与字面量类型 `int64::5` 相遇。
///
/// 声明位不参与：`Array(Int, Five)` 作变量注解时，容器由 `check_var_stmt_inner` 的
/// 元素槽/长度槽阶梯定型，容器本身不比 unify，故本形态必须落在实参位（见 09a §P6
/// 6.9.3d 的探针结论）。
const ALIAS_ACTUAL_ARG: &str = concat!(
    "Five: Type = 5\n",
    "\n",
    "take: (a: Array(Int, 5)) -> Int = (a) => 0\n",
    "\n",
    "main: () -> Void = {\n",
    "    b: Array(Int, Five) = [1, 2, 3, 4, 5]\n",
    "    r: Int = take(b)\n",
    "}\n",
);

/// 负控：`Idx` 的 δ 体是 `Array(Int, 5)`，而初始化器是 `Vec(Int)`——两侧归约后
/// 仍不相等，判定点必须保留 `unify` 的原文诊断。
const ALIAS_DISPROVED: &str = concat!(
    "Idx: Type = Array(Int, 5)\n",
    "\n",
    "main: () -> Void = {\n",
    "    x: Idx = [1, 2, 3, 4, 5]\n",
    "}\n",
);

/// 类型位算术在 `From<ast::Type>` 的 `ConstExpr` 臂即折叠成字面量类型，
/// 与 `int64::5` 在 unify 之前已同形（6.9.3d 之前的既有行为，不得回退）。
const LITERAL_ARITH: &str = concat!(
    "take5: (a: Array(Int, 5)) -> Int = (a) => 0\n",
    "\n",
    "main: () -> Void = {\n",
    "    b: Array(Int, 2 + 3) = [1, 2, 3, 4, 5]\n",
    "    r: Int = take5(b)\n",
    "}\n",
);

/// 变量声明位的阶梯形态（元素槽/长度槽定型）必须保持放行——原先成功的程序不得改判。
const ALIAS_DECL_LADDER: &str = concat!(
    "Five: Type = 5\n",
    "\n",
    "main: () -> Void = {\n",
    "    b: Array(Int, Five) = [1, 2, 3, 4, 5]\n",
    "}\n",
);

/// 别名环：两侧 ground、δ 归约失败 ⇒ 用户可见 **E4023**（不是 E1002）。
///
/// 这是契约第 5 条「E4023 用户可达」的直驱判据，同时覆盖第 6 条的 subject 门禁：
/// 报文必须带 `l ≡ r` 形态的命题（`defeq` 自身给的 `subject` 是 `None`，
/// 命题与位置由判定点覆写）。
const ALIAS_CYCLE_E4023: &str = concat!(
    "A: Type = B\n",
    "B: Type = A\n",
    "\n",
    "take: (a: Array(Int, 5)) -> Int = (a) => 0\n",
    "\n",
    "main: () -> Void = {\n",
    "    b: Array(Int, A) = [1, 2, 3, 4, 5]\n",
    "    r: Int = take(b)\n",
    "}\n",
);

/// 契约第 1 条主锚（先红后绿）：δ 别名实参与字面量类型实参经生产判定点判等。
///
/// 红态（改动前）实测：`E1002 Expected type 'Array(int64, int64::5)', found type
/// 'Array(int64, Five)'`——两侧 ground、仅表示不同，正是 RFC-027 §12.1 允许改用
/// 判等的形态。
#[test]
fn test_alias_actual_arg_is_judged_equal() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("alias_actual_arg.yx", ALIAS_ACTUAL_ARG);

    // Assert
    assert!(
        result.is_ok(),
        "δ 别名实参应经判定点判等通过；实际: {}",
        result
            .err()
            .map(|e| e.message().to_string())
            .unwrap_or_default()
    );
}

/// 契约第 1 条负控：判等只救表示差，不救真不等——`Disproved` 必须回退到 unify 原文。
#[test]
fn test_alias_disproved_keeps_unify_diagnostic() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("alias_disproved.yx", ALIAS_DISPROVED);

    // Assert
    let error = result.expect_err("Array(Int,5) 与 Vec(Int) 归约后仍不等，必须拒绝");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1002"),
        "Disproved 应保留 unify 的 E1002 原文（不得改报 E4023）；实际: {}",
        error.message()
    );
}

/// 既有行为钉子：类型位算术在进入判定点之前已折叠，`2+3` 与 `5` 同形。
#[test]
fn test_literal_arith_in_type_position_same_as_literal() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("literal_arith.yx", LITERAL_ARITH);

    // Assert
    assert!(
        result.is_ok(),
        "类型位 2+3 折叠后应与字面量 5 同形（不得回退折叠）；实际: {}",
        result
            .err()
            .map(|e| e.message().to_string())
            .unwrap_or_default()
    );
}

/// 既有行为钉子：变量声明位的容器阶梯不受判定点影响（原先成功 ⇒ 仍成功）。
#[test]
fn test_alias_decl_ladder_stays_accepted() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("alias_decl_ladder.yx", ALIAS_DECL_LADDER);

    // Assert
    assert!(
        result.is_ok(),
        "声明位容器走元素槽/长度槽阶梯，判定点接入不得改判为错；实际: {}",
        result
            .err()
            .map(|e| e.message().to_string())
            .unwrap_or_default()
    );
}

/// 契约第 5 条：别名环 ⇒ E4023（命题 + 原因 + 位置齐全）。
#[test]
fn test_alias_cycle_reports_e4023_with_proposition_and_position() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("alias_cycle_e4023.yx", ALIAS_CYCLE_E4023);

    // Assert
    let error = result.expect_err("别名环无法归约：应报「无法证明」而不是「不等」");
    let diagnostic = error
        .diagnostic()
        .expect("E4023 必须携带结构化诊断（不是自由文本）");
    assert_eq!(
        diagnostic.code.as_str(),
        "E4023",
        "归约失败不得被站点改写成 E1002；实际: {}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains("≡"),
        "命题必须是类型等式形态（判定点覆写 subject）；实际: {}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains("A") && diagnostic.message.contains("int64::5"),
        "命题两侧都必须是真实类型；实际: {}",
        diagnostic.message
    );
    assert_eq!(
        diagnostic.span.map(|span| span.start.line),
        Some(8),
        "位置必须落在判定点（`take(b)` 的实参 b），不是声明处"
    );
}
