//! 嵌套函数体的外层帧捕获判定 — RFC-009 §2.3 / SPEC §12.3 / WBS 3.6.3
//!
//! 规则（RFC-009 §2.3，2026-06-16 决议 + 2026-08-17 补充）：Lambda **不隐式捕获
//! 外层变量**；隐式捕获的编译错误码是 E1001（`concurrency.md` §3.4 同文）。
//! 判据是「该引用是否需要从**外层帧**取值」：
//! - 外层帧的值（函数参数 / const 泛型参数 / 块局部 / 语句绑定值）→ 需要 → 拒（E1001）
//! - 静态函数名 / 模块级绑定 / 编译期名（类型参数）→ 不需要 → 放行
//!
//! 覆盖（3.6.3 前的静默错值形态：不报错、算错值，或运行期 E6007）：
//! - 函数类型参数被嵌套函数体引用（t2 的 probe_g：打印 void）
//! - Int 参数被嵌套函数体引用（打印 6 而非 13）
//! - 两个嵌套函数各引用一个外层参数（运行期 E6007）
//!
//! 回归防线：模块级函数 / 模块级绑定 / 类型参数照常可用；lambda **表达式**体
//! （#254 创建点固化）行为不变；同块兄弟嵌套函数引用维持既有 E1001（先于本任务）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::util::diagnostic::Diagnostic;

/// 解析 + 类型检查，返回全部诊断（收集模式：不短路，逐条可查）。
fn check_source(source: &str) -> Vec<Diagnostic> {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("词法失败: {e:?}\n源码:\n{source}"));
    let result = parse(&tokens);
    assert!(result.errors.is_empty(), "解析失败: {:?}", result.errors);
    let mut checker = TypeChecker::new("test");
    checker.check_module(&result.module).diagnostics
}

/// 诊断 span 覆盖的源码文本——断言「定位指向引用处」。
fn span_text<'a>(
    source: &'a str,
    d: &Diagnostic,
) -> &'a str {
    let span = d
        .span
        .unwrap_or_else(|| panic!("诊断缺 span（无法定位）：{d:?}"));
    &source[span.start.offset..span.end.offset]
}

/// 取第一条 E1001。
fn first_e1001(diagnostics: &[Diagnostic]) -> &Diagnostic {
    diagnostics
        .iter()
        .find(|d| d.code == "E1001")
        .unwrap_or_else(|| panic!("应报 E1001（隐含捕获外层帧值），实际诊断: {diagnostics:?}"))
}

#[test]
fn test_nested_fn_function_param_capture_rejected_with_e1001() {
    // Arrange：嵌套函数体按名调用**外层函数类型参数** h（需从外层帧取值）
    let source = r#"
double: (x: Int) -> Int = (x) => x * 2
apply: (h: (x: Int) -> Int, x: Int) -> Int = {
    g = (y: Int) => h(y)
    g(x)
}
apply(double, 3)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：修复前静默打印 void；现在编译期 E1001，定位指向 `h` 的引用处
    let d = first_e1001(&diagnostics);
    assert_eq!(
        span_text(source, d),
        "h",
        "诊断应指向嵌套函数体里对 h 的引用位置，实际诊断: {d:?}"
    );
}

#[test]
fn test_nested_fn_value_param_capture_rejected_with_e1001() {
    // Arrange：嵌套函数体引用外层 **Int 参数** n（不限于函数类型）
    let source = r#"
apply: (n: Int) -> Int = {
    g = (y: Int) => y + n
    g(3)
}
apply(10)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：修复前静默打印 6（应为 13）；现在 E1001 指向 `n`
    let d = first_e1001(&diagnostics);
    assert_eq!(
        span_text(source, d),
        "n",
        "诊断应指向嵌套函数体里对 n 的引用位置，实际诊断: {d:?}"
    );
}

#[test]
fn test_nested_fn_multiple_param_captures_rejected_with_e1001() {
    // Arrange：两个嵌套函数各引用一个外层参数（修复前运行期 E6007）
    let source = r#"
outer: (a: Int, b: Int) -> Int = {
    add: (x: Int) -> Int = (x) => x + a
    scale: (x: Int) -> Int = (x) => x * b
    add(scale(2))
}
outer(10, 3)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：至少一条 E1001，且指向 a 或 b 的引用处
    let d = first_e1001(&diagnostics);
    let at = span_text(source, d);
    assert!(
        at == "a" || at == "b",
        "诊断应指向 a 或 b 的引用处，实际指向 {at:?}；全部诊断: {diagnostics:?}"
    );
}

#[test]
fn test_nested_fn_param_capture_same_code_and_message_as_local_capture() {
    // Arrange：同名的「参数捕获」与「块局部捕获」两份源码——后者是修复前就有的判据
    let param_capture = r#"
outer: (v: Int) -> Int = {
    g = (y: Int) => y + v
    g(1)
}
outer(10)
"#;
    let local_capture = r#"
outer: () -> Int = {
    v = 10
    g = (y: Int) => y + v
    g(1)
}
outer()
"#;

    // Act
    let from_param = check_source(param_capture);
    let from_local = check_source(local_capture);

    // Assert：二者同码（E1001）同文案（message 逐字相同，含变量名）——同一条判定
    let a = first_e1001(&from_param);
    let b = first_e1001(&from_local);
    assert_eq!(
        (a.code.as_str(), a.message.as_str()),
        (b.code.as_str(), b.message.as_str()),
        "参数捕获与块局部捕获必须同码同文案（同一条隐式捕获判定）"
    );
}

#[test]
fn test_nested_fn_module_level_function_reference_still_allowed() {
    // Arrange：嵌套函数体引用**模块级函数**（静态函数名，不需外层帧值）
    let source = r#"
inc: (x: Int) -> Int = (x) => x + 1
outer: (a: Int) -> Int = {
    wrap: (x: Int) -> Int = (x) => inc(x) + 1
    wrap(3)
}
outer(0)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：模块级名不需要帧值 → 不得被捕获屏障拦
    assert!(
        !diagnostics.iter().any(|d| d.code == "E1001"),
        "模块级函数引用应放行，实际诊断: {diagnostics:?}"
    );
}

#[test]
fn test_nested_fn_global_binding_reference_still_allowed() {
    // Arrange：嵌套函数体引用**模块级绑定值**（全局槽位，不是外层帧）
    let source = r#"
base: Int = 10
outer: (a: Int) -> Int = {
    wrap: (x: Int) -> Int = (x) => x + base
    wrap(3)
}
outer(0)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：模块级绑定读全局槽位 → 放行
    assert!(
        !diagnostics.iter().any(|d| d.code == "E1001"),
        "模块级绑定引用应放行，实际诊断: {diagnostics:?}"
    );
}

#[test]
fn test_nested_fn_outer_type_param_reference_allowed() {
    // Arrange：嵌套函数的**标注**引用外层**类型参数** T（编译期名，不占运行时帧）
    let source = r#"
outer: (T: Type, x: T) -> T = {
    id: (v: T) -> T = (v) => v
    id(x)
}
outer(Int, 7)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：类型参数不需帧值 → 不构成隐式捕获，放行
    assert!(
        !diagnostics.iter().any(|d| d.code == "E1001"),
        "外层类型参数引用应放行（编译期名不进运行时帧），实际诊断: {diagnostics:?}"
    );
}

#[test]
fn test_expression_lambda_param_capture_still_allowed() {
    // Arrange：**表达式位置**的 lambda 捕获参数（#254 创建点固化机制）
    let source = r#"
double: (x: Int) -> Int = (x) => x * 2
apply: (h: (x: Int) -> Int, x: Int) -> Int = ((y: Int) => h(y))(x)
apply(double, 3)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：表达式 lambda 的捕获走另一条已交付路径，本次不得改变其行为
    assert!(
        !diagnostics.iter().any(|d| d.code == "E1001"),
        "表达式位置 lambda 的捕获应保持可用（#254），实际诊断: {diagnostics:?}"
    );
}

#[test]
fn test_nested_fn_sibling_reference_rejected_e1001_pre_existing() {
    // Arrange：嵌套函数引用**同块兄弟嵌套函数**——兄弟名登记在外层块局部层
    let source = r#"
outer: (a: Int) -> Int = {
    add_one: (x: Int) -> Int = (x) => x + 1
    wrap: (x: Int) -> Int = (x) => add_one(x) + 1
    wrap(3)
}
outer(0)
"#;

    // Act
    let diagnostics = check_source(source);

    // Assert：**先于本任务**即报 E1001（外层块局部层在 enter_fn 时整体移出）；
    // 本次判据只处理「外层帧的值」面，不改变该既有行为（详见 3.6.3 报告 §兄弟函数口径）
    let d = first_e1001(&diagnostics);
    assert_eq!(
        span_text(source, d),
        "add_one",
        "兄弟嵌套函数引用维持既有 E1001 且定位指向引用处，实际诊断: {d:?}"
    );
}
