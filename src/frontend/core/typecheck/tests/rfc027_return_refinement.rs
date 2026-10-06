//! RFC-027 §3 返回位精化（后置条件）—— 返回形式参数的**声明名**识别与谓词实参代入
//!
//! §3 把返回位精化定义为与形参位**同一条规则**的两侧：`形参名: 谓词调用(实参)`，
//! 区别仅在于值由调用方提供还是由 `return` 提供。返回形式参数「仅存在于类型签名
//! 中、仅被谓词引用，**不进入函数体作用域**」。
//!
//! 本文件钉住三条此前实测偏差的语义：
//! 1. 返回形式参数按**声明的名字**识别（解析层保留名字并传给类型检查器，见
//!    `Type::NamedParen`），不再靠「约束里不在作用域的自由变量即返回值形参」猜；
//! 2. 谓词应用的实参**真正代入**约束——`(r: IsPositive(r + 100))` 的约束是
//!    `r + 100 > 0`，而不是退化成 `IsPositive(<const-expr>)` 再被当成 `IsPositive(r)`；
//! 3. 除声明名之外、不在作用域的自由变量报未定义标识符（E1001），不得被静默
//!    代入返回值（`(r: Eq2(m, r))` 的 `m`）。
//!
//! 另钉住 RFC-027 §2 的返回位**多参数谓词**边界：实参含变量时证明内核执行不了
//!（`try_proof_fn_call` 要实参全部可取值 ⇒ 空证明调用集），报 E2031，其 `{var}` 槽
//! 取**标识符**（声明名，或裸形态下 RFC-027 §2 的规范名 `result`），
//! 不得把约束挂到形参名上，
//! 替代写法由 E2031 的本地化 help 给出；实参全为字面量时仍生成证明函数调用交
//! 编译期执行。
//!
//! 端到端对照语料：`tests/yaoxiang/02-type-system/return_refinement_arg_expr.yx`、
//! `tests/yaoxiang/06-compile-errors/return_refinement_arg_expr_err.yx`、
//! `tests/yaoxiang/06-compile-errors/return_refinement_undeclared_var_err.yx`、
//! `tests/yaoxiang/06-compile-errors/return_refinement_multi_arg_predicate_err.yx`、
//! `tests/yaoxiang/06-compile-errors/return_refinement_multi_arg_literal_err.yx`。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::TypeCheckResult;
use crate::frontend::core::types::const_data::ConstValue;
use crate::util::diagnostic::Diagnostic;

/// §3 的一元谓词：`x > 0`。
const IS_POSITIVE: &str = "IsPositive: (x: Int) -> Type = { x > 0 }\n";

/// §2 的二元谓词（RFC-027 招牌后置条件形态）：`s == n * 2`。
const SUM_UP_TO: &str = "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n";

/// §3 的二元证明函数：`a == b`（非单参数谓词定义，走证明函数路径）。
const EQ2: &str = "Eq2: (a: Int, b: Int) -> Type = { a == b }\n";

/// 解析并类型检查一段源码（词法/语法错误折算成 E0001 诊断）。
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

/// 断言诊断里出现指定错误码。
fn assert_has_code(
    result: &TypeCheckResult,
    code: &str,
    why: &str,
) {
    assert!(
        result.diagnostics.iter().any(|d| d.code == code),
        "{why}；期望 {code}，实际: {:#?}",
        result.diagnostics
    );
}

/// 取第一条指定 code 的诊断；不存在即 panic，并带上全部诊断供定位。
///
/// 「按码取诊断」的判空样板在本文件多处重复，统一收到这里，用例侧只留断言本身。
fn expect_diagnostic_with_code<'a>(
    diagnostics: &'a [Diagnostic],
    code: &str,
) -> &'a Diagnostic {
    diagnostics
        .iter()
        .find(|d| d.code == code)
        .unwrap_or_else(|| panic!("应报 {code}；实际诊断: {diagnostics:#?}"))
}

/// 文本是否含 CJK（中日韩文字与全角标点）。
///
/// 用来钉住「诊断文案里的自然语言只能来自 locales」：code 侧只传标识符
///（声明名 / 规范名 `result`）与约束表达式，故 en 渲染下不应出现 CJK。
fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(
            c as u32,
            0x3000..=0x303F | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xFF00..=0xFFEF
        )
    })
}

/// 读 `locales/en.json` 里某错误码的本地化 help。
///
/// 与 i18n 同源：`src/util/i18n/mod.rs` 用 `include_str!` 嵌的就是这个文件，
/// 故断言「help 回落本地化文案」在下游翻译更新时会随之更新。
fn en_locale_help(code: &str) -> String {
    let raw: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../locales/en.json"))
            .expect("locales/en.json 应可解析为 JSON");
    raw[code]["help"]
        .as_str()
        .unwrap_or_else(|| panic!("locales/en.json 缺少 {code}.help"))
        .to_string()
}

/// §3：谓词实参是**表达式**时按实参代入约束，可证者为零诊断。
///
/// `d1: () -> (r: IsPositive(r + 100)) = { return -5 }`：约束是 `r + 100 > 0`，
/// 代入返回值 `-5` 得 `-5 + 100 > 0`（95 > 0）——成立，故零诊断。
/// 修复前实参被降级成占位 `<const-expr>`，约束退化成 `-5 > 0` 而误报。
#[test]
fn test_return_binder_argument_expression_substituted_proves_clean() {
    // Arrange
    let source = format!("{IS_POSITIVE}d1: () -> (r: IsPositive(r + 100)) = {{ return -5 }}");

    // Act / Assert
    assert_clean(
        &source,
        "约束 -5 + 100 > 0 成立：谓词实参 r + 100 必须真正代入，而非当成 r",
    );
}

/// §3：谓词实参代入后为假 ⇒ E4018，且反例/约束文本里是**代入后的式子**。
///
/// `d2: () -> (r: IsPositive(r - 100)) = { return 5 }`：约束 `5 - 100 > 0`（-95 > 0）
/// 为假。修复前实参丢失，约束退化成 `5 > 0` 而静默通过。
#[test]
fn test_return_binder_argument_expression_substituted_violation_reports_e4018() {
    // Arrange
    let source = format!("{IS_POSITIVE}d2: () -> (r: IsPositive(r - 100)) = {{ return 5 }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert_has_code(
        &result,
        "E4018",
        "约束 5 - 100 > 0 为假：代入实参后必须判伪",
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("5 - 100")),
        "诊断应给出**代入后**的约束 `5 - 100 > 0`（实参真正代入的证据）；实际: {:#?}",
        result.diagnostics
    );
}

/// §3：返回形式参数只认**声明的名字**；未声明的自由变量报未定义标识符。
///
/// `g: () -> (r: Eq2(m, r)) = { return 7 }`：声明的返回形式参数是 `r`，
/// `m` 既不在签名里、也不在作用域里 ⇒ E1001。
/// 修复前 `m` 与 `r` 都被当成返回形式参数代入 `7`，约束退化成 `7 == 7` 恒真而静默通过。
#[test]
fn test_return_binder_undeclared_free_variable_reports_unknown_identifier() {
    // Arrange
    let source = format!("{EQ2}g: () -> (r: Eq2(m, r)) = {{ return 7 }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert_has_code(
        &result,
        "E1001",
        "未声明的自由变量 m 不得被当成返回形式参数，应报未定义标识符",
    );
    assert!(
        result.diagnostics.iter().any(|d| d.message.contains("m")),
        "未定义标识符诊断应点名 m；实际: {:#?}",
        result.diagnostics
    );
}

/// §3：声明的返回形式参数之外，**作用域内**的自由变量用自己的符号（不代返回值）。
///
/// `f: (b: IsPositive(b)) -> (r: IsPositive(b + r)) = { b + 1 }`：约束 `b + r > 0`，
/// 只有声明的 `r` 代入返回值 `b + 1` ⇒ `b + (b + 1) > 0`，在 Γ={b>0} 下成立。
#[test]
fn test_return_binder_substitutes_only_declared_name() {
    // Arrange
    let source =
        format!("{IS_POSITIVE}f: (b: IsPositive(b)) -> (r: IsPositive(b + r)) = {{ b + 1 }}");

    // Act / Assert
    assert_clean(
        &source,
        "约束里的 b 是形参（用自己的符号），只有声明的 r 代入返回值",
    );
}

/// §3：裸精化返回位（未声明 binder）按**形参符号**判定，不代入返回值。
///
/// `f: (b: Int) -> IsPositive(b + 1) = { b + 1 }`：约束 `b + 1 > 0` 与返回值无关，
/// b 无下界 ⇒ 有反例（b = -1）⇒ E4018。
#[test]
fn test_bare_return_refinement_checked_over_params_reports_e4018() {
    // Arrange
    let source = format!("{IS_POSITIVE}f: (b: Int) -> IsPositive(b + 1) = {{ b + 1 }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert_has_code(
        &result,
        "E4018",
        "裸形态的约束只涉及形参 b，b 无下界时有反例（b = -1）",
    );
}

// ============ RFC-027 §2：返回位多参数谓词（含变量实参暂不支持，给明确诊断） ============

/// §2/§3：返回位多参数谓词的实参含**变量**时，诊断对象是**返回位**——
/// E2031 的 `{var}` 槽传 locale 中性标识（有声明 binder 就传它的名字 `r`，
/// 不再挂到形参 `b`），自然语言措辞全部来自 locales（help 回落 E2031 本地化文案）。
///
/// `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }` 语义上完全正确
///（r = b * 2 确实满足 s == n * 2），但证明内核只执行实参**全部可取值**
///（字面量或已绑定变量）的证明函数调用——`b` 是形参、无界，故判不出。
#[test]
fn test_multi_arg_predicate_with_variable_arg_points_at_return_position() {
    // Arrange —— 钉住 en 本地化（默认即 en；显式设定使断言与环境配置无关）
    crate::util::i18n::set_lang_from_string("en".to_string());
    let source = format!("{SUM_UP_TO}f: (b: Int) -> (r: SumUpTo(b, r)) = {{ return b * 2 }}");

    // Act
    let result = check_source(&source);

    // Assert
    let diag = expect_diagnostic_with_code(&result.diagnostics, "E2031");
    assert!(
        diag.message.contains("'r'"),
        "{{var}} 槽应是声明的 binder 名 `r`（此前误挂到形参 b 上）；实际: {:#?}",
        diag.message
    );
    assert!(
        !diag.message.contains("'b'"),
        "不得再把约束挂到形参名上；实际: {:#?}",
        diag.message
    );
    assert!(
        !contains_cjk(&diag.message) && !contains_cjk(&diag.help),
        "code 侧只传中性标识，诊断文案不得含 code 写死的自然语言；实际 message={:?} help={:?}",
        diag.message,
        diag.help
    );
    assert_eq!(
        diag.help,
        en_locale_help("E2031"),
        "help 应回落 E2031 的本地化文案（不再由 code 侧 with_help 覆盖）"
    );
}

/// §2：裸精化返回位（未声明 binder）的 `{var}` 取**规范标识符** `result`。
///
/// RFC-027 §2 把返回值形参命名为 `result`（spec: `-> (result: IsMax(T, arr, result))`，
/// 「`result` 是返回值形参，值由 `return` 提供」），裸形态 `-> P(...)` 即其简写——
/// 故这里不是英文散文，而是合法的标识符。
#[test]
fn test_bare_multi_arg_predicate_points_at_return_position() {
    // Arrange
    crate::util::i18n::set_lang_from_string("en".to_string());
    let source = format!("{SUM_UP_TO}f: (b: Int) -> SumUpTo(b, b) = {{ b }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert_has_code(&result, "E2031", "裸形态的多元谓词实参含形参，判不出");
    let diag = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E2031")
        .expect("已断言存在 E2031");
    assert!(
        diag.message.contains("'result'") && !diag.message.contains("'b'"),
        "裸形态 {{var}} 应是规范标识符 `result`（不是形参 b、也不是散文）；实际: {:#?}",
        diag.message
    );
    assert!(
        !contains_cjk(&diag.message),
        "裸形态文案同样不得含 code 写死的自然语言；实际: {:#?}",
        diag.message
    );
}

/// §2：实参**全可取值**（字面量 + 由 `return` 代入的 binder）时不得走「暂不支持」，
/// 而要生成证明函数调用，交 pipeline 编译期执行（其判 false 时报 E4018）。
///
/// `f: () -> (r: SumUpTo(3, r)) = { return 7 }` ⇒ 实参 [Int(3), Int(7)]。
/// E4018 由 pipeline 在 `check_module` 之后报，故端到端由语料
/// `06-compile-errors/return_refinement_multi_arg_literal_err.yx` 覆盖，
/// 本测试钉住「调用被生成且实参为 [Int(3), Int(7)]」这一步。
#[test]
fn test_multi_arg_predicate_with_literal_arg_collects_proof_call() {
    // Arrange
    let source = format!("{SUM_UP_TO}f: () -> (r: SumUpTo(3, r)) = {{ return 7 }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E2031"),
        "实参可取值时不得报「返回位暂不支持」；实际: {:#?}",
        result.diagnostics
    );
    let call = result
        .proof_calls()
        .iter()
        .find(|c| c.func_name == "SumUpTo")
        .unwrap_or_else(|| {
            panic!(
                "应生成 SumUpTo 的证明函数调用；实际: {:#?}",
                result.proof_calls()
            )
        });
    assert!(
        matches!(
            call.args.as_slice(),
            [ConstValue::Int(3), ConstValue::Int(7)]
        ),
        "证明函数实参应是代入返回值后的 [Int(3), Int(7)]；实际: {:?}",
        call.args
    );
}

/// §2：字面量实参**成立**时不报任何诊断——证明函数编译期执行为真即通过。
///
/// `f: () -> (r: SumUpTo(3, r)) = { return 6 }` ⇒ 证明函数调用 `SumUpTo(3, 6)`
///（6 == 3 * 2）为真，故 `check_module` 零诊断。与上一条成对：
/// 「暂不支持」诊断不得吃掉实参全部可取值（真能判）的多元谓词应用。
#[test]
fn test_multi_arg_predicate_with_literal_arg_satisfied_is_clean() {
    // Arrange
    let source = format!("{SUM_UP_TO}f: () -> (r: SumUpTo(3, r)) = {{ return 6 }}");

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "6 == 3 * 2 成立，应零诊断；实际: {:#?}",
        result.diagnostics
    );
    let call = result
        .proof_calls()
        .iter()
        .find(|c| c.func_name == "SumUpTo")
        .expect("应生成 SumUpTo 的证明函数调用交编译期执行");
    assert!(
        matches!(
            call.args.as_slice(),
            [ConstValue::Int(3), ConstValue::Int(6)]
        ),
        "证明函数实参应是 [Int(3), Int(6)]；实际: {:?}",
        call.args
    );
}
