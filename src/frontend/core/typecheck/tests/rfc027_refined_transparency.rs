//! RFC-027 §6.1 精化类型的**透明性**测试
//!
//! 精化类型 `{x: T | p(x)}` 是**编译期约束**，不是独立的运行时类型：一个
//! `NonNegative(b)` 的值**就是**一个 `Int` 的值，凡接受 `Int` 处皆应接受它。
//! 本节钉住这条基础规则——「精化类型可当它的基类型用」。
//!
//! 背景：绑定位的同类规则已落地（`c280ea86`，语料
//! `tests/yaoxiang/02-type-system/refined_variable_uses_base_type.yx`），但
//! **形参位不成立**：`b: NonNegative(b)` 在体内是名义类型，于是
//! 1) 体内 `b` 当值用报 E1012/E1002，2) 任何调用方传入基类型报 E1002
//!    （连合法的 `f(5)` 都被拒——不是谓词校验，是个坏类型）。
//!
//! 本文件只断言 `check_module` 能看到的东西。证明调用（E4018）由 `pipeline.rs`
//! 在 `check_module` **之后**执行，故精化**违反**的用例在 `.yx` 层，见
//! `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx`。

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::TypeCheckResult;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::util::diagnostic::Diagnostic;

/// 下界谓词：`x >= 0`。源码内定义（语言里没有内建的非负类型）。
const NON_NEGATIVE: &str = "NonNegative: (x: Int) -> Type = { x >= 0 }\n";

/// 解析并类型检查
fn check_source(source: &str) -> TypeCheckResult {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(e) => {
            return TypeCheckResult {
                diagnostics: vec![Diagnostic::error(
                    "E0001".to_string(),
                    format!("词法错误: {}", e),
                    String::new(),
                    None,
                )],
                ..Default::default()
            };
        }
    };
    let module = match parse(&tokens) {
        result if result.has_errors => {
            return TypeCheckResult {
                diagnostics: vec![Diagnostic::error(
                    "E0001".to_string(),
                    format!("解析错误: {:?}", result.errors),
                    String::new(),
                    None,
                )],
                ..Default::default()
            };
        }
        result => result.module,
    };
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module)
}

/// 断言零诊断。
///
/// 断言「零诊断」而非「无某码」：E1002 / E1012 是同一个病的两个出口，
/// 只堵一个会让测试在另一处漏绿。
fn assert_clean(
    source: &str,
    why: &str,
) -> TypeCheckResult {
    let result = check_source(source);
    assert!(
        result.diagnostics.is_empty(),
        "{why}；期望零诊断，实际: {:#?}",
        result.diagnostics
    );
    result
}

/// 返回位：精化形参当作基类型返回。
#[test]
fn test_param_refinement_transparent_in_return_position() {
    // Arrange
    let source = format!("{NON_NEGATIVE}f: (b: NonNegative(b)) -> Int = {{ b }}");

    // Act / Assert
    assert_clean(
        &source,
        "精化形参 b: NonNegative(b) 的值就是一个 Int，直接返回它不应报 E1012",
    );
}

/// 操作数位：精化形参参与算术。
#[test]
fn test_param_refinement_transparent_in_operator() {
    // Arrange
    let source = format!("{NON_NEGATIVE}f: (b: NonNegative(b)) -> Int = {{ b + 1 }}");

    // Act / Assert
    assert_clean(&source, "精化形参参与算术不应报 E1002（期望基类型 Int）");
}

/// 相等比较：精化形参与基类型比较。
#[test]
fn test_param_refinement_transparent_in_equality() {
    // Arrange
    let source = {
        let f = "f: (b: NonNegative(b)) -> Int = { if b == 0 { 1 } else { 0 } }";
        format!("{NON_NEGATIVE}{f}")
    };

    // Act / Assert
    assert_clean(
        &source,
        "精化形参与基类型比较不应报 E1101（refined 未实现 Equal）",
    );
}

/// 调用实参位：精化形参传给接受基类型的函数。
#[test]
fn test_param_refinement_transparent_as_call_argument() {
    // Arrange
    let source = {
        let f = "g: (x: Int) -> Int = { x }";
        let h = "h: (b: NonNegative(b)) -> Int = { g(b) }";
        format!("{NON_NEGATIVE}{f}\n{h}")
    };

    // Act / Assert
    assert_clean(&source, "精化形参传入接受 Int 的函数不应报类型不符");
}

/// 调用方一侧：以基类型实参调用精化形参的函数。
///
/// 这是修复前最刺眼的一处——**合法的** `f(5)` 被拒。
#[test]
fn test_refined_fn_accepts_base_type_argument() {
    // Arrange
    let source = {
        let f = "f: (b: NonNegative(b)) -> Int = { b }";
        let m = "main: () -> Void = {\n    z: Int = f(5)\n}";
        format!("{NON_NEGATIVE}{f}\n{m}")
    };

    // Act / Assert
    assert_clean(
        &source,
        "精化形参的函数应接受基类型实参；修复前连 f(5) 都报 E1002",
    );
}

/// 调用点实参对形参精化的义务：违反时直接判伪并报 E4018（RFC-027 §3.4）。
///
/// 谓词定义注册（#377-3）后，形参约束是**谓词体**（`b >= 0` 而非不透明应用），
/// 常量实参可直接折叠判伪，故诊断在 `check_module` 内即产生（不必等 pipeline
/// 执行证明函数）。端到端见
/// `tests/yaoxiang/06-compile-errors/refined_param_arg_violates_err.yx`。
#[test]
fn test_call_arg_refinement_violation_judged_directly() {
    // Arrange — f(-5) 违反 b >= 0
    let source = {
        let f = "f: (b: NonNegative(b)) -> Int = { 0 }";
        let m = "main: () -> Void = {\n    z: Int = f(-5)\n}";
        format!("{NON_NEGATIVE}{f}\n{m}")
    };

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "调用点传入违反形参精化的常量应报 E4018；实际: {:#?}",
        result.diagnostics
    );
}

/// **Tripwire** —— 实参**转不出常量表达式**时不生成义务（与上一条的边界）。
///
/// 上一条（裸变量实参）已按 §3.4 强半场拒绝；本条是**剩余边界**：实参是调用
///（`g(1)`）时构造不出代入式，无从判定，故仍放过。它不等于强半场未启用，
/// 而是「可形式化的实参都已要求证据；不可形式化的尚未」。
///
/// 谁让调用实参也开始判（如内联/证明函数），本用例会变红，提醒把断言改为「报错」。
#[test]
fn test_call_arg_unconvertible_still_generates_no_obligation() {
    // Arrange — 实参是形参（无静态值）
    let source = {
        let g = "g: (x: Int) -> Int = { x }";
        let f = "f: (b: NonNegative(b)) -> Int = { 0 }";
        let m = "main: () -> Void = {\n    z: Int = f(g(1))\n}";
        format!("{NON_NEGATIVE}{g}\n{f}\n{m}")
    };

    // Act
    let result = check_source(&source);

    // Assert — 当前无义务；强半场启用后应改为断言报错
    assert!(
        !result
            .proof_calls()
            .iter()
            .any(|c| c.func_name == "NonNegative"),
        "实参非编译期常量时本版不生成义务；若此处变红说明已落地静态证据要求，\
         请把断言改为「报错」。实际: {:?}",
        result.proof_calls()
    );
}

// ==================== RFC-027 §3.4：强半场的两条证据来源 ====================

/// §3.4 —— 实参**无任何静态证据**时必须拒绝（强半场）。
///
/// `divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)`：`y` 无界、也
/// 无精化标注，验证条件 `{} ⇒ { y > 0 }` 有反例（y = 0）⇒ E4018。
/// 这是 §3.4「YaoXiang 不接受运行时值直接进入精化类型参数而不提供静态证据」
/// 的直接落地（#395 第二行）。
#[test]
fn test_call_arg_without_evidence_is_rejected() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}divide: (a: Int, b: IsPositive(b)) -> Int = {{ a }}\n\
         u: (x: Int, y: Int) -> Int = divide(x, y)"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "实参无静态证据应报 E4018（§3.4 强半场）；实际: {:#?}",
        result.diagnostics
    );
}

/// §3.4 —— 实参**自身的精化标注**是证据（#395 第三行）。
///
/// `via_param: (n: IsPositive(n)) -> Int = divide(0, n)`：证据来自形参前置条件。
/// 与上一条成对——同一调用形态，只差 `n` 是否带精化标注。
#[test]
fn test_call_arg_evidenced_by_own_param_refinement() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}divide: (a: Int, b: IsPositive(b)) -> Int = {{ a }}\n\
         via_param: (n: IsPositive(n)) -> Int = divide(0, n)"
    );

    // Assert
    assert_clean(
        &source,
        "实参 n 自身带 IsPositive 标注，即 §3.3 的静态证据，应零诊断",
    );
}

/// §3.4 —— 局部变量的精化标注是证据（#395 第三行的局部形态）。
///
/// 与上一条的区别：证据不在形参前置条件里，而在**实参变量自己的声明**上。
/// 判定时把该约束注入局部作用域作假设，查完即弹。
#[test]
fn test_call_arg_evidenced_by_local_var_refinement() {
    // Arrange —— 初值是指针未定的调用，故只能靠变量精化标注而非字面量值；
    // 调用写成**赋值右侧**：嵌套在 `print(...)` 里的调用不进检查
    //（#395 另注：检查位置仅语句层两种形态），写成嵌套形式会让本用例丧失判别力。
    let source = format!(
        "{IS_POSITIVE}g: (x: Int) -> Int = {{ x }}\n\
         divide: (a: Int, b: IsPositive(b)) -> Int = {{ a }}\n\
         main: () -> Void = {{\n    n: IsPositive(n) = g(1)\n    z: Int = divide(0, n)\n    print(z)\n}}"
    );

    // Assert
    assert_clean(
        &source,
        "实参 n 的声明带精化标注，应作为证据；否则会误报（值未知时折不出常量）",
    );
}

/// §3.4 —— `if` 守卫是证据（§3.4 帮助文案自己举的例子）。
///
/// `if y > 0 { divide(x, y) }`：守卫 `y > 0` 正是精化实参所需的证据。
#[test]
fn test_call_arg_evidenced_by_if_guard() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}divide: (a: Int, b: IsPositive(b)) -> Int = {{ a }}\n\
         guarded: (x: Int, y: Int) -> Int = {{\n\
         \x20   if y > 0 {{\n        z: Int = divide(x, y)\n        return z\n    }}\n    0\n}}"
    );

    // Assert
    assert_clean(
        &source,
        "`if y > 0` 分支内 y 有界，守卫应作证据；不注入守卫即误报",
    );
}

/// §3.4 —— 无守卫时同一调用仍须拒绝（守卫要有、不能白送）。
///
/// 与上一条成对：同一调用、同一函数，只差 `if` 守卫。
#[test]
fn test_call_arg_without_guard_is_rejected() {
    // Arrange —— 无守卫
    let source = format!(
        "{IS_POSITIVE}divide: (a: Int, b: IsPositive(b)) -> Int = {{ a }}\n\
         unguarded: (x: Int, y: Int) -> Int = {{\n    z: Int = divide(x, y)\n    z\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "无守卫时 y 无界，应报 E4018；实际: {:#?}",
        result.diagnostics
    );
}

// ==================== RFC-027 §3：返回点精化义务（后置条件） ====================

/// §3 的谓词定义：`IsPositive: (x: Int) -> Type = { x > 0 }`
const IS_POSITIVE: &str = "IsPositive: (x: Int) -> Type = { x > 0 }\n";

/// RFC-027 §3 —— 违反后置条件必须报 E4018，并给出反例。
///
/// `bad: (b: IsPositive(b)) -> (r: IsPositive(r)) = { b - 1 }`：
/// 约束 `r > 0` 代入返回值 `b - 1` 得 `b - 1 > 0`，在 Γ={b>0} 下**有反例**（b = 1）。
///
/// 这是此前最大的静默漏洞：返回位精化**完全未被校验**，违反后置条件的函数零诊断。
#[test]
fn test_return_postcondition_violation_reported() {
    // Arrange
    let source =
        format!("{IS_POSITIVE}bad: (b: IsPositive(b)) -> (r: IsPositive(r)) = {{ b - 1 }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "违反后置条件 `r > 0` 应报 E4018；实际: {:#?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("b = 1") || d.message.contains("b = (1)")),
        "反例应给出使约束为假的形参取值（b = 1，此时返回值为 0）；实际: {:#?}",
        result.diagnostics
    );
}

/// RFC-027 §3 —— 后置条件由**形参前置条件**得证时零诊断。
///
/// 与上一条成对：同一谓词、同一返回位形态，只差函数体。
/// `b + 1 > 0` 在 Γ={b>0} 下恒真，故成立。缺 Γ 注入则本用例会（误）报错——
/// 两条一起才钉死「假设集真的进了判定」。
#[test]
fn test_return_postcondition_proved_from_param_refinement_is_clean() {
    // Arrange
    let source =
        format!("{IS_POSITIVE}succ: (b: IsPositive(b)) -> (r: IsPositive(r)) = {{ b + 1 }}");

    // Assert
    assert_clean(
        &source,
        "Γ 含形参精化 `b > 0` 时，`b + 1 > 0` 成立，后置条件应零诊断",
    );
}

/// RFC-027 §3 —— 裸精化返回位（约束只涉及形参）同样在返回点验证。
///
/// `f: (b: Int) -> IsPositive(b + 1) = { b + 1 }`：约束 `b + 1 > 0` 与返回值无关，
/// 它是「返回类型在 b ≤ -1 时为空」的断言；b 无下界故有反例 ⇒ 拒绝。
///
/// 这正是 §3「统一性」的体现：两个形态同一条规则，只是绑定来源不同——
/// 具名形态（`-> (r: P(...))`）的声明名绑 `return` 的值，裸形态没有声明名，
/// 约束里的 `b` 是形参 ⇒ 用自己的符号（见 rfc027_return_refinement.rs）。
#[test]
fn test_bare_return_refinement_checked_over_params() {
    // Arrange
    let source = format!("{IS_POSITIVE}f: (b: Int) -> IsPositive(b + 1) = {{ b + 1 }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "`b + 1 > 0` 在 b 无下界时有反例（b = -1），应报 E4018；实际: {:#?}",
        result.diagnostics
    );
}

/// RFC-027 §3 —— 显式 `return`（非尾表达式）同样在返回点验证。
///
/// RFC-010a：块的值 = 尾表达式，故尾表达式是**隐式** `return`；显式 `return`
/// 是另一条出口。两条出口都要查——只查其一即漏。
#[test]
fn test_explicit_return_postcondition_is_checked() {
    // Arrange —— 走显式 return 这条出口
    let source = format!(
        "{IS_POSITIVE}f: (b: IsPositive(b)) -> (r: IsPositive(r)) = {{\n    return b - 1\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "显式 return 违反后置条件应同样报 E4018；实际: {:#?}",
        result.diagnostics
    );
}

/// **Tripwire** —— 返回表达式转不出常量表达式时**不判**（保守：不误报）。
///
/// 后置条件的验证走符号代入：把返回值形参在约束里代入返回表达式的常量形式。
/// 返回表达式含调用（`g(b)`）时转不出，此时无从构造代入式，**放过**而非猜。
/// 谁让它开始判（如补上调用的内联/证明函数），本用例会变红，提醒把断言改为报错。
#[test]
fn test_return_refinement_not_checked_when_expression_unconvertible() {
    // Arrange —— 返回表达式是调用，转不出常量表达式
    let source = format!(
        "{IS_POSITIVE}g: (x: Int) -> Int = {{ x }}\n\
         f: (b: IsPositive(b)) -> (r: IsPositive(r)) = {{ g(b - 1) }}"
    );

    // Assert
    assert_clean(
        &source,
        "返回表达式转不出常量表达式时无从代入，本版保守放过",
    );
}
