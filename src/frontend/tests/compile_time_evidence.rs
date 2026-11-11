//! 编译期值参数的**证据判定**（RFC-039 D62 / 6.9.2-B，issue #533）
//!
//! q=0 形参的实参必须是编译期可确定的值。改动前 `collect_const_arg_instantiation_request`
//! 用 `Option` 把「非目标调用」与「目标调用缺证据」混成一个 `None`：后者被静默按运行期
//! 传参——`f(y)` 里内层 `n: N` 的约束从未生效，且特化永不发生。本轮把判定拆成三态
//! （非目标 / 证据齐备 / 缺证据），缺证据在调用点该实参处报 E1113。
//!
//! 本文件是**登记-消费直驱**测试：声明面的调用点形参名表（`checker` 的签名收集 →
//! `StatementChecker` → 表达式推断器）与消费面的证据判定，同一条用例里首尾相扣——
//! 手工构造登记数据不算证据。

use crate::frontend::compiler::Compiler;

/// R-B1：`y` 是运行期形参 ⇒ `N` 没有编译期证据。
const MISSING_EVIDENCE: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "k: (y: Int, z: Int) -> Int = f(y)(z)\n",
    "main: () -> Void = {\n",
    "    r: Int = k(42, 42)\n",
    "}\n",
);

/// 混合声明的第二层：`N` 仍是 q=0，实参 `y` 无证据（与纯 const 泛型同一判据）。
const MIXED_DECL_MISSING_EVIDENCE: &str = concat!(
    "g: (x: Int, N: Int) -> (n: N) -> Int = (n) => n\n",
    "k: (y: Int) -> Int = g(3, y)(42)\n",
    "main: () -> Void = {\n",
    "    r: Int = k(42)\n",
    "}\n",
);

/// 合法形态：字面量 / 值环境 / 单值表达式（复用既有 Evaluator）。
const EVIDENCE_FORMS: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "main: () -> Void = {\n",
    "    x: Int = 42\n",
    "    k: Int = 43\n",
    "    a: Int = f(42)(42)\n",
    "    b: Int = f(x)(42)\n",
    "    c: Int = f(k - 1)(42)\n",
    "}\n",
);

/// 命名实参指向 q=0 形参（实参无证据）：证据判定照判——「不支持特化」不是跳过理由。
const NAMED_CONST_ARG_WITHOUT_EVIDENCE: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "k: (y: Int) -> Int = f(N = y)\n",
    "main: () -> Void = {\n",
    "    r: Int = k(42)\n",
    "}\n",
);

/// 命名实参指向 q=0 形参（实参有证据）：证据齐备后落回既有的实参名校验——
/// 可命名表取自实现头视图（只含 `n`），故 `N = 42` 报 E1014（**既有判据**，本批不改）。
const NAMED_CONST_ARG_WITH_EVIDENCE: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "k: (y: Int) -> Int = f(N = 42)(42)\n",
    "main: () -> Void = {\n",
    "    r: Int = k(42)\n",
    "}\n",
);

/// R-B1：缺证据的 q=0 实参必须报 E1113，位置指向**该实参**而不是调用整体。
#[test]
fn test_const_arg_missing_evidence_is_reported() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("const_arg_missing_evidence.yx", MISSING_EVIDENCE);

    // Assert
    let error = result.expect_err("运行期形参实参不构成编译期证据，应拒绝");
    let diagnostic = error
        .diagnostic()
        .expect("E1113 必须携带结构化诊断（不是自由文本）");
    assert_eq!(
        diagnostic.code.as_str(),
        "E1113",
        "缺编译期证据应报 E1113（不得借用 E1092/E1010 或只发 warning）；实际: {}",
        error.message()
    );
    assert!(
        diagnostic.message.contains("'N'"),
        "诊断必须点名缺证据的编译期值参数 N；实际: {}",
        diagnostic.message
    );
    assert_eq!(
        diagnostic.span.map(|span| span.start.line),
        Some(2),
        "诊断应落在第 2 行 `f(y)` 的实参 y 处（不是声明处、不是调用整体）"
    );
}

/// 混合声明（q=0 不在首层）同样要判证据——「不支持特化」不是跳过检查的理由。
#[test]
fn test_const_arg_mixed_declaration_missing_evidence_is_reported() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("const_arg_mixed_missing.yx", MIXED_DECL_MISSING_EVIDENCE);

    // Assert
    let error = result.expect_err("混合声明的 q=0 形参缺证据同样应拒绝");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1113"),
        "混合形态应报同一个 E1113；实际: {}",
        error.message()
    );
}

/// R-B2：三类证据形态都必须通过（合法形态不得因证据判定而变红）。
#[test]
fn test_const_arg_evidence_forms_are_accepted() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("const_arg_evidence_forms.yx", EVIDENCE_FORMS);

    // Assert
    assert!(
        result.is_ok(),
        "字面量 / 值环境 / 单值表达式三类证据都应通过；实际: {:?}",
        result.err().map(|e| e.message().to_string())
    );
}

/// 命名形态**不跳过**证据检查：`f(N = y)` 的 y 无证据 ⇒ E1113。
///
/// 披露（诊断码平移）：该形态改动前报 E1014（旧判定对命名实参整体 `return None`，
/// 让后面的实参名校验接手）。现在证据判定先跑，故同一非法程序换了个**更贴切**的码；
/// 两种形态都是拒绝，且 `N = 42`（证据齐备）仍保持 E1014——见下一条测试。
#[test]
fn test_const_arg_named_target_without_evidence_still_reports_e1113() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile(
        "const_arg_named_without_evidence.yx",
        NAMED_CONST_ARG_WITHOUT_EVIDENCE,
    );

    // Assert
    let error = result.expect_err("按名给 q=0 形参传运行期值应被拒绝");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1113"),
        "命名形态同样要过证据检查（不得因「不支持特化」跳过）；实际: {}",
        error.message()
    );
}

/// 命名形态 + 证据齐备：落回既有实参名校验（E1014，可用名表取自实现头视图）。
/// 本批**不改**这条判据——它属「可命名表该用哪个视图」的既有问题。
#[test]
fn test_const_arg_named_target_with_evidence_falls_back_to_existing_reject() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile(
        "const_arg_named_with_evidence.yx",
        NAMED_CONST_ARG_WITH_EVIDENCE,
    );

    // Assert
    let error = result.expect_err("q=0 形参名不在命名实参的可用名表里");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1014"),
        "证据齐备后应落回既有的未知名判定 E1014；实际: {}",
        error.message()
    );
}

/// RFC-011 §4.1 / SPEC §7.1 逐字收录的规范示例（递归 const 泛型）。
///
/// 体内 `k: N` 是**单值钉住**的形参（N 是编译期值参数）：`k - 1` 的值特化时才定，
/// 但编译期可求值。6.9.2-B 首版只认 `MonoType::Literal{v}`，把这条规范形态判成
/// 「无证据」——成功程序回归（t20 F1）。
const RECURSIVE_FACTORIAL: &str = concat!(
    "factorial: (N: Int) -> (k: N) -> Int = {\n",
    "    return match k {\n",
    "        0 => 1,\n",
    "        _ => k * factorial(k - 1)(k - 1)\n",
    "    }\n",
    "}\n",
    "main: () -> Void = {\n",
    "    r: Int = factorial(5)(5)\n",
    "}\n",
);

/// 钉住形参与字面量构成的表达式（`k + 1`）也是证据；`id(k)` 这种调用结果不是。
const PINNED_EXPRESSION: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "g: (N: Int) -> (k: N) -> Int = {\n",
    "    return f(k + 1)(42)\n",
    "}\n",
    "main: () -> Void = {\n",
    "    r: Int = g(41)(41)\n",
    "}\n",
);

const PINNED_PARAM_BUT_CALL_RESULT: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "id: (x: Int) -> Int = x\n",
    "g: (N: Int) -> (k: N) -> Int = {\n",
    "    return f(id(k))(42)\n",
    "}\n",
    "main: () -> Void = {\n",
    "    r: Int = g(41)(41)\n",
    "}\n",
);

/// 反例：**普通运行期形参**不构成证据（k: Int，不是 `k: N`）。
const NON_PINNED_PARAM: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "h: (k: Int) -> Int = f(k)(42)\n",
    "main: () -> Void = {\n",
    "    r: Int = h(42)\n",
    "}\n",
);

/// F3 钉测试（tripwire，勿读成「正确行为」）：**方法面**（`接收者.方法(...)` 的
/// `FieldAccess` 形态）当前**完全不判**编译期证据——`b.get(y)` 里 `y` 是运行期
/// 形参，却不报 E1113。
///
/// 事实：6.9.2-B 的槽位表只登记**函数名**，调用点 `callee_generic_name` 对「值接收者
/// 的方法调用」返回 None（只认 `Var` 与模块限定的 `FieldAccess`），故整段判定走
/// `NotTarget`。方法面的 q=0 事实（`Type.method` 声明 + `Export` 侧）不在本批覆盖面。
/// 修好后本条必须改成「报 E1113」，在那之前不得把它当作「该形态合法」的证据。
const METHOD_FACE_CONST_ARG: &str = concat!(
    "Box: Type = { v: Int }\n",
    "Box.get: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "main: () -> Void = {\n",
    "    b: Box = Box(v = 1)\n",
    "    y: Int = 3\n",
    "    r: Int = b.get(y)(y)\n",
    "}\n",
);

/// 规范示例必须编译通过（改前因 E1113 红）。
#[test]
fn test_const_generic_recursive_factorial_compiles() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("recursive_factorial.yx", RECURSIVE_FACTORIAL);

    // Assert
    assert!(
        result.is_ok(),
        "RFC-011 §4.1 的规范示例必须通过（单值钉住的 k 是编译期已知的）；实际: {:?}",
        result.err().map(|e| e.message().to_string())
    );
}

/// 钉住形参参与的**纯运算**表达式构成证据。
#[test]
fn test_const_arg_pinned_param_expression_is_evidence() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("pinned_expression.yx", PINNED_EXPRESSION);

    // Assert
    assert!(
        result.is_ok(),
        "`k + 1`（k 被单值钉住）应构成证据；实际: {:?}",
        result.err().map(|e| e.message().to_string())
    );
}

/// 钉住形参**经函数调用**的结果不构成证据（不是纯运算）。
#[test]
fn test_const_arg_call_result_is_not_evidence() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("pinned_but_call.yx", PINNED_PARAM_BUT_CALL_RESULT);

    // Assert
    let error = result.expect_err("`id(k)` 是调用结果，不能证明编译期可求值");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1113"),
        "非纯运算的表达式仍应报 E1113；实际: {}",
        error.message()
    );
}

/// 反例：普通运行期形参不构成证据（钉住面不得被放宽成「任何形参都算」）。
#[test]
fn test_const_arg_non_pinned_param_is_not_evidence() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("non_pinned_param.yx", NON_PINNED_PARAM);

    // Assert
    let error = result.expect_err("`k: Int` 是普通运行期形参，不构成编译期证据");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1113"),
        "非钉住形参应报 E1113；实际: {}",
        error.message()
    );
}

/// F3 tripwire：方法面当前**零诊断**（未覆盖，见上方说明）。
#[test]
fn test_method_bound_const_generic_argument_is_currently_unchecked() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("method_face_const_arg.yx", METHOD_FACE_CONST_ARG);

    // Assert — tripwire：方法面当前不判证据（`callee_generic_name` 对值接收者返回 None）
    assert!(
        result.is_ok(),
        "tripwire：方法面（接收者.方法）当前不判编译期证据；若这里开始失败，说明方法面已被接管——请把断言改成「必须报 E1113」并删掉本 tripwire。实际: {:?}",
        result.err().map(|e| e.message().to_string())
    );
}

/// F1'（t23）：`match` 臂把**同名** `k` 绑到运行期值——钉住身份不得随名字继承。
///
/// `g` 的形参 `k: N` 是单值钉住的（可作证据），但臂内 `k` 是新绑定（运行期值）：
/// 按名判会把 `k - 1` 当编译期证据放行（运行期 q=8、按名证据取到的是被遮蔽的
/// 钉住形参），按绑定判必须报 E1113。
const PINNED_PARAM_SHADOWED_BY_ARM: &str = concat!(
    "id: (x: Int) -> Int = x\n",
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "g: (N: Int) -> (k: N) -> (r: Int) -> Int = {\n",
    "    return match r {\n",
    "        k => f(k - 1)(k - 1)\n",
    "    }\n",
    "}\n",
    "main: () -> Void = {\n",
    "    v: Int = g(41)(41)(id(9))\n",
    "}\n",
);

/// F1'（t23）：值环境名被臂内同名绑定遮蔽——`const_bindings` 不得按名摊平。
///
/// 全局 `k0 = 7` 是值环境常量；臂内 `k0` 是运行期绑定（无 const_value）。按名摊平
/// 会让外层 7 顶上来，把 `k0 - 1` 算成 6（运行期是 8）；按绑定取最内层则无值 ⇒ E1113。
const VALUE_ENV_SHADOWED_BY_ARM: &str = concat!(
    "id: (x: Int) -> Int = x\n",
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "k0: Int = 7\n",
    "g: (r: Int) -> Int = {\n",
    "    return match r {\n",
    "        k0 => f(k0 - 1)(k0 - 1)\n",
    "    }\n",
    "}\n",
    "main: () -> Void = {\n",
    "    v: Int = g(id(9))\n",
    "}\n",
);

/// 遮蔽钉住形参的绑定不是编译期证据（E1113，位置指向该实参）。
#[test]
fn test_const_arg_pinned_param_shadowed_by_match_arm_is_not_evidence() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("pinned_param_shadowed.yx", PINNED_PARAM_SHADOWED_BY_ARM);

    // Assert
    let error = result.expect_err("臂内同名绑定是运行期值，不得继承钉住身份");
    let diagnostic = error.diagnostic().expect("E1113 必须携带结构化诊断");
    assert_eq!(
        diagnostic.code.as_str(),
        "E1113",
        "遮蔽后的 `k - 1` 不是证据，应报 E1113；实际: {}",
        error.message()
    );
    assert_eq!(
        diagnostic.span.map(|span| span.start.line),
        Some(5),
        "诊断应落在第 5 行 `f(k - 1)` 的实参处"
    );
}

/// 遮蔽值环境名的绑定不是编译期证据（E1113）——`const_bindings` 按最内层绑定取。
#[test]
fn test_const_arg_value_env_shadowed_by_match_arm_is_not_evidence() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("value_env_shadowed.yx", VALUE_ENV_SHADOWED_BY_ARM);

    // Assert
    let error = result.expect_err("臂内 k0 没有编译期值，外层 7 不得顶上来");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1113"),
        "遮蔽后的 `k0 - 1` 不可求值，应报 E1113；实际: {}",
        error.message()
    );
}

/// 6.9.2-C1 ⑥：**闭项求值失败**（`1 / 0`）必须拒绝。
///
/// 旧行为（B 步）：`1 / 0` 结构上是编译期项，检查期不求值 ⇒ 静默接受，运行期才炸。
/// C1 起：归一化返回 `Disproved` ⇒ 与「缺证据」同一出口（E1113）拒绝（D59 C 类）。
const CLOSED_TERM_THAT_FAILS: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "main: () -> Void = {\n",
    "    v: Int = f(1 / 0)(1)\n",
    "}\n",
);

/// 闭项求值失败 ⇒ E1113（不得静默接受、不得当 `Unproven` 放行）。
#[test]
fn test_const_arg_closed_term_failure_is_reported() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("closed_term_fails.yx", CLOSED_TERM_THAT_FAILS);

    // Assert
    let error = result.expect_err("`1 / 0` 是闭项但求值失败，必须拒绝");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1113"),
        "闭项求值失败应与缺证据同码 E1113 拒绝；实际: {}",
        error.message()
    );
}

/// 值环境证据的**单值收紧**（6.8.7）必须保持：`f(x)(5)`（`x: Int = 42`）报 E1002。
///
/// 这是 B 步的基线行为（t18 实测），C1 首版一度退化成「0 诊断」：请求里放的是
/// **原始项** `NamedVar("x")`，而 `GenericArg::as_type()` 手里没有环境（空环境里该名
/// 未绑定 ⇒ 求值失败 ⇒ **`Disproved`** ⇒ 无类型位投影）⇒ 请求未建 ⇒ 返回类型没被换成
/// `int64::42`。
/// C1 的约定是「请求只放闭项 ⇒ 项即其值」，即放归一化后的 `Lit(42)`，收紧随之恢复。
/// 本条是**语义断言**（不是 tripwire）：值环境与字面量走同一条收紧路径。
const VALUE_ENV_SINGLE_VALUE: &str = concat!(
    "f: (N: Int) -> (n: N) -> Int = (n) => n\n",
    "main: () -> Void = {\n",
    "    x: Int = 42\n",
    "    v: Int = f(x)(5)\n",
    "}\n",
);

/// 值环境证据同样触发单值收紧（`5 ≠ 42` ⇒ E1002）。
#[test]
fn test_const_arg_value_env_keeps_single_value_enforcement() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("value_env_single_value.yx", VALUE_ENV_SINGLE_VALUE);

    // Assert
    let error = result.expect_err("x = 42 的实参 5 与单值类型 int64::42 不符，应报 E1002");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E1002"),
        "值环境证据与字面量走同一条单值收紧；实际: {}",
        error.message()
    );
}
