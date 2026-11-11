//! RFC-027 §6.1 精化类型的**透明性**测试
//!
//! 精化类型 `{x: T | p(x)}` 是**编译期约束**，不是独立的运行时类型：一个
//! `NonNegative(b)` 的值**就是**一个 `Int` 的值，凡接受 `Int` 处皆应接受它。
//! 本节钉住这条基础规则——「精化类型可当它的基类型用」。
//!
//! 背景：绑定位的同类规则已落地（`c280ea86`，语料
//! `tests/yaoxiang/02-type-system/refinement/refined_variable_uses_base_type.yx`），但
//! **形参位不成立**：`b: NonNegative(b)` 在体内是名义类型，于是
//! 1) 体内 `b` 当值用报 E1012/E1002，2) 任何调用方传入基类型报 E1002
//!    （连合法的 `f(5)` 都被拒——不是谓词校验，是个坏类型）。
//!
//! 本文件只断言 `check_module` 能看到的东西。证明调用（E4018）由 `pipeline.rs`
//! 在 `check_module` **之后**执行，故精化**违反**的用例在 `.yx` 层，见
//! `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx`。
//!
//! 文件末段另钉住 RFC-027 §3.3/§3.4 的**多形参关系谓词**调用点义务（#532）：
//! `n: Eq(n, N)` 的自由变量是多个声明形参名，按声明序与实参按位对齐后整体代入，
//! 判定仍走同一条 `check_predicate` 管道。端到端对照语料
//! `02-type-system/refinement/refined_relation_arg.yx`、
//! `06-compile-errors/refinement/refined_relation_arg_violates_err.yx`、
//! `06-compile-errors/refinement/refined_relation_arg_order_err.yx`、
//! `06-compile-errors/refinement/refined_relation_arg_named_err.yx`。
//!
//! 形参约束引用**另一个**形参（`b: IsPositive(a)`）时，槽位按**自由变量名**解析，
//! 不用被约束位的实参顶替（文末「形参约束引用另一个形参」一节钉住两侧与边界）。
//!
//! 命名实参的槽位绑定不在此重判：消费共享权威层
//! `inference::arg_binding`（位置实参占前导槽、命名实参按名入槽）。文末「本版边界」
//! 一节钉住四条**未修面**（绑定位开约束 / 嵌套调用 / 柯里声明 / 模块级常量引用），
//! 每条都断言「现在不报」——它们归 6.9.4 的 Γ 等式桥与后续遍历器工作。

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::types::TypeCheckResult;
use crate::frontend::core::types::const_data::ConstValue;
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
    checker.check_module(&module, &TypeLevelNamespace::scan_one(&module))
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
/// `tests/yaoxiang/06-compile-errors/refinement/refined_param_arg_violates_err.yx`。
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

// ==================== RFC-027 §3.3/§3.4：多形参关系谓词的调用点义务（#532） ====================

/// §2/§3 的二元关系谓词：`a == b`。
///
/// 与一元谓词不同，它**不进**编译期谓词注册表（注册只收单参数定义），因此形参
/// 约束保持不透明应用 `Eq(n, N)`——这正是此前多实参谓词的调用点义务被漏掉的形态。
const EQ: &str = "Eq: (a: Int, b: Int) -> Type = { a == b }\n";

/// §2 的非交换二元关系：`s == n * 2`——用来钉住「形参—实参按位对齐」。
const SUM_UP_TO: &str = "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n";

/// 声明序形参名 ↔ 实参**按位对齐**：`f(42, 7)` 的约束 `Eq(n, N)` 代入后是
/// `Eq(7, 42)`（`n` 在声明里是第二位、`N` 是第一位）。
///
/// `check_module` 只产出证明义务（E4018 由 pipeline 编译期执行后发射），故本用例
/// 钉住「义务被生成且实参按声明位置替换」；端到端拒绝见语料
/// `tests/yaoxiang/06-compile-errors/refinement/refined_relation_arg_violates_err.yx`。
#[test]
fn test_relation_call_arg_substituted_in_declaration_order() {
    // Arrange —— N 为第一位、n 为第二位，实参 (42, 7) 因此应落到 (N, n)
    let source = format!(
        "{EQ}f: (N: Int, n: Eq(n, N)) -> Int = {{ n }}\n\
         main: () -> Void = {{\n    x: Int = f(42, 7)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    let call = result
        .proof_calls()
        .iter()
        .find(|c| c.func_name == "Eq")
        .unwrap_or_else(|| {
            panic!(
                "多实参关系谓词应生成 Eq 的证明函数调用；实际: {:#?}",
                result.proof_calls()
            )
        });
    assert!(
        matches!(
            call.args.as_slice(),
            [ConstValue::Int(7), ConstValue::Int(42)]
        ),
        "约束 Eq(n, N) 代入实参后应是 Eq(7, 42)（n 是第二位、N 是第一位）；实际: {:?}",
        call.args
    );
}

/// 合法关系调用：`f(42, 42)` 的义务是闭合的 `Eq(42, 42)`，`check_module` 零诊断
///（判定结果交 pipeline 执行证明函数，真即通过）。
#[test]
fn test_relation_call_arg_satisfied_is_clean() {
    // Arrange
    let source = format!(
        "{EQ}f: (N: Int, n: Eq(n, N)) -> Int = {{ n }}\n\
         main: () -> Void = {{\n    x: Int = f(42, 42)\n    print(x)\n}}"
    );

    // Act
    let result = assert_clean(&source, "42 == 42 成立，关系谓词的调用点义务应零诊断");

    // Assert —— 义务确实被生成（不是「没查所以没报」）
    let call = result
        .proof_calls()
        .iter()
        .find(|c| c.func_name == "Eq")
        .expect("应生成 Eq 的证明函数调用交编译期执行");
    assert!(
        matches!(
            call.args.as_slice(),
            [ConstValue::Int(42), ConstValue::Int(42)]
        ),
        "证明函数实参应是 [Int(42), Int(42)]；实际: {:?}",
        call.args
    );
}

/// 非交换关系的**顺序**判据：`g(3, 6)` 满足 `s == n * 2`，只有把第二实参
/// 代入 `s` 才成立（错接成 `SumUpTo(6, 3)` 会得 `3 == 12` 而判伪）。
#[test]
fn test_relation_call_arg_noncommutative_order_is_clean() {
    // Arrange
    let source = format!(
        "{SUM_UP_TO}g: (n: Int, s: SumUpTo(n, s)) -> Int = {{ s }}\n\
         main: () -> Void = {{\n    y: Int = g(3, 6)\n    print(y)\n}}"
    );

    // Act
    let result = assert_clean(
        &source,
        "6 == 3 * 2 成立；形参与实参错接会变成 SumUpTo(6, 3) 而误报",
    );

    // Assert
    let call = result
        .proof_calls()
        .iter()
        .find(|c| c.func_name == "SumUpTo")
        .expect("应生成 SumUpTo 的证明函数调用");
    assert!(
        matches!(
            call.args.as_slice(),
            [ConstValue::Int(3), ConstValue::Int(6)]
        ),
        "证明函数实参应是 [Int(3), Int(6)]；实际: {:?}",
        call.args
    );
}

/// **边界** —— 符号实参（`f(N, n)`，透传前一层的形参）不送检：代入后是开约束
///（`Eq(N, n)`），本轮把「证不出」当「已证伪」会误拒正确程序。开约束的证据面
/// 归 6.9.4 的 Γ 等式桥；本用例是那条边界的履行哨。
#[test]
fn test_relation_call_arg_symbolic_form_generates_no_obligation() {
    // Arrange —— 透传包装：约束里的两个自由变量都是调用方的形参
    let source = format!(
        "{EQ}f: (N: Int, n: Eq(n, N)) -> Int = {{ n }}\n\
         g: (N: Int, n: Eq(n, N)) -> Int = {{ f(N, n) }}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "符号实参本轮保留既有边界（不判、不误报）；实际: {:#?}",
        result.diagnostics
    );
    assert!(
        !result.proof_calls().iter().any(|c| c.func_name == "Eq"),
        "开约束（Eq(N, n)）不应生成证明义务；若此处变红说明 Γ 等式桥已落地，\
         请把断言改为「按证据判定」。实际: {:?}",
        result.proof_calls()
    );
}

/// **边界** —— 实参转不出编译期表达式（`f(42, g(7))`）时不生成义务：本路径要求
/// 全部必需实参可形式化，缺一即保留既有边界（与一元路径同一条 tripwire 口径）。
#[test]
fn test_relation_call_arg_unconvertible_generates_no_obligation() {
    // Arrange
    let source = format!(
        "{EQ}h: (x: Int) -> Int = {{ x }}\n\
         f: (N: Int, n: Eq(n, N)) -> Int = {{ n }}\n\
         main: () -> Void = {{\n    x: Int = f(42, h(7))\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        !result.proof_calls().iter().any(|c| c.func_name == "Eq"),
        "实参不可形式化时不生成义务；若此处变红说明已落地更强的形式化要求，\
         请把断言改为「报错」。实际: {:?}",
        result.proof_calls()
    );
}

// ==================== 命名实参：槽位绑定归共享权威层（#532） ====================

/// 命名实参按**形参名**入槽，与书写序无关：`f(n = 7, N = 42)` 的约束 `Eq(n, N)`
/// 代入后是 `Eq(7, 42)`，而不是按书写序的 `Eq(42, 7)`。
///
/// 槽位绑定消费共享权威层 `inference::arg_binding`（IR 的实参重排同源），
/// 调用点义务与命名/位置两种实参形态因此同一判据。
#[test]
fn test_relation_call_arg_named_arguments_bound_by_param_name() {
    // Arrange —— 书写序 (n, N) 与声明序 (N, n) 相反
    let source = format!(
        "{EQ}f: (N: Int, n: Eq(n, N)) -> Int = {{ n }}\n\
         main: () -> Void = {{\n    x: Int = f(n = 7, N = 42)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    let call = result
        .proof_calls()
        .iter()
        .find(|c| c.func_name == "Eq")
        .unwrap_or_else(|| {
            panic!(
                "命名实参调用应生成 Eq 的证明函数调用；实际: {:#?}",
                result.proof_calls()
            )
        });
    assert!(
        matches!(
            call.args.as_slice(),
            [ConstValue::Int(7), ConstValue::Int(42)]
        ),
        "Eq(n, N) 代入命名实参后应是 Eq(7, 42)（按名入槽，与书写序无关）；实际: {:?}",
        call.args
    );
}

/// 命名实参的合法关系调用：`g(s = 6, n = 3)` 的义务是 `SumUpTo(3, 6)`，
/// `check_module` 零诊断——非交换关系上按书写序错接会得 `SumUpTo(6, 3)` 而误报。
#[test]
fn test_relation_call_arg_named_arguments_satisfied_is_clean() {
    // Arrange
    let source = format!(
        "{SUM_UP_TO}g: (n: Int, s: SumUpTo(n, s)) -> Int = {{ s }}\n\
         main: () -> Void = {{\n    y: Int = g(s = 6, n = 3)\n    print(y)\n}}"
    );

    // Act
    let result = assert_clean(
        &source,
        "6 == 3 * 2 成立；命名实参若按书写序入槽会变成 SumUpTo(6, 3) 而误报",
    );

    // Assert
    let call = result
        .proof_calls()
        .iter()
        .find(|c| c.func_name == "SumUpTo")
        .expect("应生成 SumUpTo 的证明函数调用");
    assert!(
        matches!(
            call.args.as_slice(),
            [ConstValue::Int(3), ConstValue::Int(6)]
        ),
        "证明函数实参应是 [Int(3), Int(6)]；实际: {:?}",
        call.args
    );
}

/// 一元精化形参的**命名实参**同样受调用点义务约束：`divide(a = 0, b = -5)` 的
/// `b > 0` 代入 -5 即判伪，`check_module` 直接报 E4018（与位置实参形态同判）。
#[test]
fn test_unary_call_arg_named_arguments_checked() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}divide: (a: Int, b: IsPositive(b)) -> Int = {{ a }}\n\
         main: () -> Void = {{\n    x: Int = divide(a = 0, b = -5)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "命名实参 -5 违反 b > 0，应报 E4018；实际: {:#?}",
        result.diagnostics
    );
}

// ==================== 本版边界（#532 未修面登记） ====================
//
// 四条边界都由本节的「现在不报」用例钉住，任何一条变成「报错」都说明对应形态
// 已被接管——届时把断言改为「报错」并同步 09a 的边界登记，不得直接删掉用例。

/// 边界①-a：**绑定位**的多元开约束不判。`u: (...) -> Int = h(n, s)` 的实参是
/// 符号（n、s 未绑定），代入后仍是开约束（`SumUpTo(n, s)`），缺值闸门拒绝送检。
///
/// 对照：同形的一元路径**会**交判定管道找反例（见下一条与语料
/// `06-compile-errors/refinement/call_arg_no_evidence_err.yx`）——这个分叉是
/// 已登记的边界，不是等价语义。
#[test]
fn test_relation_call_arg_binding_position_open_constraint_is_not_checked() {
    // Arrange
    let source = format!(
        "{SUM_UP_TO}h: (n: Int, s: SumUpTo(n, s)) -> Int = {{ s }}\n\
         u: (n: Int, s: Int) -> Int = h(n, s)\n\
         main: () -> Void = {{\n    x: Int = u(3, 6)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E4018"),
        "多元开约束本轮不送检（证据面归 6.9.4）；若此处变红说明已接管，请改断言为报错。\
         实际: {:#?}",
        result.diagnostics
    );
    assert!(
        !result
            .proof_calls()
            .iter()
            .any(|c| c.func_name == "SumUpTo"),
        "开约束不应生成证明义务；实际: {:?}",
        result.proof_calls()
    );
}

/// 边界①-b：一元开约束**仍**报 E4018（含反例）——与上一条成对，钉住两条路径的
/// 语义分叉。与语料 `call_arg_no_evidence_err.yx` 同形。
#[test]
fn test_unary_call_arg_open_constraint_still_reports_e4018() {
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
        "一元开约束 y > 0 应判伪并报 E4018；实际: {:#?}",
        result.diagnostics
    );
}

/// 边界②：**嵌套调用**形态不判。调用点义务只覆盖裸调用语句与赋值 RHS 两种形态，
/// `print(f(42, 7))` 里的内层调用没有遍历器可达，故静默通过。
#[test]
fn test_relation_call_arg_nested_call_form_is_not_checked() {
    // Arrange
    let source = format!(
        "{EQ}f: (N: Int, n: Eq(n, N)) -> Int = {{ n }}\n\
         main: () -> Void = {{\n    print(f(42, 7))\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E4018"),
        "嵌套调用形态本轮不判；若此处变红说明已接入通用遍历器，请改断言为报错。实际: {:#?}",
        result.diagnostics
    );
    assert!(
        !result.proof_calls().iter().any(|c| c.func_name == "Eq"),
        "嵌套调用不应生成证明义务；实际: {:?}",
        result.proof_calls()
    );
}

/// 边界③：**柯里声明**的内层组不判。`curried(42)(7)` 的被调是「调用表达式」，
/// 且注册签名只承载外层形参组——内层 `n: Eq(n, N)` 当前没有义务。
#[test]
fn test_relation_call_arg_curried_declaration_is_not_checked() {
    // Arrange
    let source = format!(
        "{EQ}curried: (N: Int) -> (n: Eq(n, N)) -> Int = {{ n }}\n\
         main: () -> Void = {{\n    x: Int = curried(42)(7)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E4018"),
        "柯里声明的内层组本轮不判；若此处变红说明已接管，请改断言为报错。实际: {:#?}",
        result.diagnostics
    );
    assert!(
        !result.proof_calls().iter().any(|c| c.func_name == "Eq"),
        "柯里形态不应生成证明义务；实际: {:?}",
        result.proof_calls()
    );
}

/// 边界④：约束引用**模块级常量**（非形参名）不判。`Eq(n, LIMIT)` 的 `LIMIT`
/// 不是调用的形参名，本路径无从把它与实参对齐，故保留既有边界。
#[test]
fn test_relation_call_arg_module_constant_free_var_is_not_checked() {
    // Arrange
    let source = format!(
        "{EQ}LIMIT: Int = 11\n\
         bounded: (n: Eq(n, LIMIT)) -> Int = {{ n }}\n\
         main: () -> Void = {{\n    x: Int = bounded(10)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E4018"),
        "约束引用非形参名（模块级常量）本轮不判；若此处变红说明已接管，请改断言为报错。\
         实际: {:#?}",
        result.diagnostics
    );
    assert!(
        !result.proof_calls().iter().any(|c| c.func_name == "Eq"),
        "模块级常量形态不应生成证明义务；实际: {:?}",
        result.proof_calls()
    );
}

// ==================== 形参约束引用**另一个**形参（#532，F5） ====================
//
// `b: IsPositive(a)` 的约束属于 `a`：判定必须按**自由变量名**解析实参槽位，
// 不得用「被约束位 b 的实参」顶替。以下五条把这条判据的两侧与两条边界都钉住。

/// 位置实参：`h(1, -5)` 的约束是 `a > 0`，`a = 1` 满足；`b = -5` 与本约束无关。
#[test]
fn test_param_constraint_on_other_param_positional_is_clean() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}h: (a: Int, b: IsPositive(a)) -> Int = {{ a }}\n\
         main: () -> Void = {{\n    x: Int = h(1, -5)\n    print(x)\n}}"
    );

    // Act / Assert
    assert_clean(
        &source,
        "约束属 a（a > 0），a = 1 满足；拿 b 的实参 -5 顶替会产生伪反例",
    );
}

/// 命名实参：`h(a = 1, b = -5)` 与位置形态同判。
#[test]
fn test_param_constraint_on_other_param_named_is_clean() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}h: (a: Int, b: IsPositive(a)) -> Int = {{ a }}\n\
         main: () -> Void = {{\n    x: Int = h(a = 1, b = -5)\n    print(x)\n}}"
    );

    // Act / Assert
    assert_clean(&source, "命名实参按形参名入槽，约束仍属 a（a = 1 满足）");
}

/// 乱序命名实参：`h(b = -5, a = 1)`——书写序不影响槽位（按名入槽）。
#[test]
fn test_param_constraint_on_other_param_named_reordered_is_clean() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}h: (a: Int, b: IsPositive(a)) -> Int = {{ a }}\n\
         main: () -> Void = {{\n    x: Int = h(b = -5, a = 1)\n    print(x)\n}}"
    );

    // Act / Assert
    assert_clean(
        &source,
        "乱序命名实参按形参名入槽；书写序不得影响约束的实参来源",
    );
}

/// 真违反：`h(-5, 1)` 的 `a = -5` 使 `a > 0` 为假 ⇒ E4018。
///
/// 修复前这一形态**静默通过**（拿 b 的实参 1 顶替 a ⇒ `1 > 0` 为真），
/// 与「a 合法、b 非法」的误报正好相反。
#[test]
fn test_param_constraint_on_other_param_violation_reported() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}h: (a: Int, b: IsPositive(a)) -> Int = {{ a }}\n\
         main: () -> Void = {{\n    x: Int = h(-5, 1)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E4018"),
        "a = -5 使约束 a > 0 为假，应报 E4018；实际: {:#?}",
        result.diagnostics
    );
}

/// 边界：约束的自由变量不是声明形参名（模块级常量 `IsPositive(LIMIT)`）⇒ 不判。
///
/// 本路径无从把 `LIMIT` 与实参对齐；拿被约束位的实参顶替会把判定交给错误的值。
#[test]
fn test_param_constraint_on_module_constant_is_not_checked() {
    // Arrange
    let source = format!(
        "{IS_POSITIVE}LIMIT: Int = 5\n\
         h: (a: Int, b: IsPositive(LIMIT)) -> Int = {{ a }}\n\
         main: () -> Void = {{\n    x: Int = h(1, -5)\n    print(x)\n}}"
    );

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E4018"),
        "自由变量不是声明形参名（模块级常量）时保留既有边界（不判）；\
         若此处变红说明该形态已被接管，请改断言为按常量值判定。实际: {:#?}",
        result.diagnostics
    );
}
