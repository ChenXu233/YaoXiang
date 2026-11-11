//! WBS 6.8.2a / D54：**声明驱动的 Fn 标注 ↔ 函数体对照**（形参维度 + 出口统一）
//!
//! 背景：D54（2026-10-05）报告的「同一语义不同语法、check 绿运行期红」在
//! 出口维度已于 #501 修复轮（48e25ca6）堵上，**剩余面在形参维度**：`=>` 后的
//! 实现头与标注的运行时形参位之间，旧行为按**名字交集**合并，交集为空就整段
//! 跳过——标注的形参类型从未到达实现头。
//!
//! 本文件把 D54 的残留矩阵收成仓内夹具（外部探针目录不入库），逐条断言：
//! - ① 形参按位驱动：同一语义的三种语法（同名箭头 / 异名箭头 / block 体）
//!   必须给出**同一条诊断**；声明类型按位置到达实现头，改名不再改变判据。
//! - ② 出口两叉收一叉：箭头糖（StmtKind::Return）与显式 return
//!   （Expr::Return）都是 E1012、都指向注解。
//! - ③ 注册形态同源：命名实参的可用名取**声明**，实现头改名不影响调用侧。
//! - ④ 形参标注冲突：实现头自带标注与声明不符 → E1011（旧行为是静默覆盖）。
//!
//! 判据来源：WBS 6.8.2a 任务契约 + D54；E1112 为本轮新增的对外码
//! （实现形参个数超出声明）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;

/// 编译期诊断 (码, 行号) 序列（真实序列：lex → parse → typecheck）。
fn compile_diags(source: &str) -> Vec<(String, usize)> {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<fn-annotation-binding>");
    let tr = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    for d in tr.diagnostics.iter() {
        eprintln!("BIND_DIAG {} @{:?} :: {}", d.code, d.span, d.message);
    }
    tr.diagnostics
        .iter()
        .map(|d| (d.code.clone(), d.span.map_or(0, |s| s.start.line)))
        .collect()
}

/// 编译期诊断码集合。
fn compile_codes(source: &str) -> Vec<String> {
    compile_diags(source).into_iter().map(|(c, _)| c).collect()
}

/// 夹具：一份「同语义、三种语法」的三个函数定义。
///
/// 三个函数都是「一个 Int 形参、体内把字符串加进去」：
/// - `same`：箭头 lambda，形参同名（旧行为：标注类型按位覆盖 → E1002）
/// - `renamed`：箭头 lambda，形参改名为 b（旧行为：交集为空 → 静默放行，
///   运行期 E6007）
/// - `blocky`：block 体（形参由 parser 按标注合成 → E1002）
const THREE_SYNTAX: &str = r#"
same: (a: Int) -> Int = (a) => a + "s"
renamed: (a: Int) -> Int = (b) => b + "s"
blocky: (a: Int) -> Int = { a + "s" }
"#;

#[test]
fn test_fn_annotation_param_type_drives_every_syntax_form() {
    // Arrange：同语义、三种语法（同名箭头 / 异名箭头 / block 体）
    // Act
    let codes = compile_codes(THREE_SYNTAX);
    // Assert：三格必须是同一条诊断（旧行为：改名那一格 check 绿、运行期 E6007）
    assert_eq!(
        codes.iter().filter(|c| *c == "E1002").count(),
        3,
        "三种语法都必须报 E1002（形参类型按位来自声明）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_declared_types_bind_positionally_when_renamed() {
    // Arrange：第一个形参的类型只有声明里有（实现头不带标注）
    const SRC: &str = r#"
f: (a: Int, b: String) -> String = (x, y) => x
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：x 必须是 Int（按位取自声明），返回位是 String → E1012；
    // 旧行为：形参退回 fresh var，x 自由 unify 成 String，check 静默通过。
    assert!(
        codes.iter().any(|c| c == "E1012"),
        "改名形参必须按位拿到声明类型（x: Int ≠ String）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_named_argument_uses_declared_name() {
    // Arrange：声明名 a、实现头名 b，调用点用**声明名**传命名实参
    const SRC: &str = r#"
f: (a: Int) -> Int = (b) => b
main: () -> Void = {
    x: Int = f(a = 7)
}
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：可用名取声明（③）——旧行为报 E1014（available: b）
    assert!(
        !codes.iter().any(|c| c == "E1014"),
        "命名实参必须按**声明**名绑定：{codes:?}"
    );
    assert!(
        codes.is_empty(),
        "声明名与实现头名不同不应产生诊断：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_named_argument_positional_equivalent() {
    // Arrange：同一函数两种调用形态（位置 / 命名）必须同样放行
    const SRC: &str = r#"
f: (a: Int, b: Int) -> Int = (x, y) => x + y
main: () -> Void = {
    p: Int = f(1, 2)
    q: Int = f(b = 2, a = 1)
}
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert
    assert!(
        codes.is_empty(),
        "命名与位置调用应同样放行（同一绑定判据）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_impl_param_count_exceeds_declaration() {
    // Arrange：声明 0 个运行期形参，实现写了 1 个（旧行为：check 绿、运行期 void）
    const SRC: &str = r#"
f: () -> Int = (a: Int) => a
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：多出的实现形参没有声明位可绑 → 新增对外码 E1112
    assert!(
        codes.iter().any(|c| c == "E1112"),
        "实现形参个数超出声明必须报 E1112：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_impl_fewer_params_is_filled_from_declaration() {
    // Arrange：声明 1 个形参、实现不写形参（体内不引用它）
    const SRC: &str = r#"
f: (a: Int) -> Int = () => 1
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：反向不报——缺席位由声明名补齐，函数照常按声明元数调用
    assert!(
        codes.is_empty(),
        "实现少写形参不是错误（缺席位由声明补名）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_curry_layer_params_stay_visible() {
    // Arrange：柯里外层值参数 t 只在体内引用（#295 的参数固化）
    const SRC: &str = r#"
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：按位驱动不得把柯里外层位算进「实现形参」——否则误报 E1112/E1001
    assert!(
        codes.is_empty(),
        "柯里外层值参数必须仍在体内可见，且不占实现形参位：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_const_generic_curry_still_passes() {
    // Arrange：const 泛型柯里（(N: Int) -> (n: N) -> Int），实现头只写内层
    const SRC: &str = r#"
f: (N: Int) -> (n: N) -> Int = (n) => n
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：const 位不占运行期形参位——元数对照必须按同一口径（不得误报 E1112）
    assert!(codes.is_empty(), "const 泛型柯里不应产生诊断：{codes:?}");
}

#[test]
fn test_fn_annotation_paren_return_closure_stays_valid() {
    // Arrange：括号返回（返回闭包）+ 闭包包装形态
    const SRC: &str = r#"
mk: () -> ((a: Int) -> Int) = (a) => a + 1
make_adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：括号语义（RFC-004）不受按位驱动影响
    assert!(
        codes.is_empty(),
        "括号返回/闭包包装形态不应产生诊断：{codes:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// WBS 6.8.3：实现头只承载作者写的形参（parser 停止填声明类型）
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fn_annotation_curry_impl_arity_counts_flattened_slots() {
    // Arrange — 元数对照按**拍平的运行期值位**（声明「调用要按序给几个值」）：
    // 2 层柯里的拍平值位是 2 位，实现头写 2 位合法、3 位才多；平铺声明只有 1 位。
    // 6.8.3 曾试按「实现头所绑那一层」（最内层组）对照，会把既有可运行写法
    // `add3: (a: Int) -> (b: Int) -> (c: Int) -> Int = (a, b, c) => ...` 判成 E1112
    //（tests/integration/curry_codegen.rs / curry_execution.rs 各有一条判据）。
    //
    // 反过来也**不得**把 `curry_two` 当成「绑定成功」的证据：柯里声明配多形参实现头
    // 的绑定本身是既有缺口（体内一引用那几位就报错——内部一致性错误 / E1053 / E1012，
    // 见 09a §P6 6.8.3 的登记）。本用例只钉 E1112 的元数口径，不涉绑定。
    const SRC: &str = r#"
flat: (a: Int) -> Int = (x, y) => 1
curry_two: (A: Int) -> (B: String) -> Int = (x, y) => 1
curry_three: (A: Int) -> (B: String) -> Int = (x, y, z) => 1
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert — flat（1 位写 2）与 curry_three（2 位写 3）各一条，curry_two 合法
    assert_eq!(
        codes.iter().filter(|c| *c == "E1112").count(),
        2,
        "平铺 1 位写 2 位、柯里 2 位写 3 位各报一条；柯里 2 位写 2 位不报：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_curry_impl_within_slots_still_passes() {
    // Arrange — 同一柯里声明，实现头少写（1 位、0 位）
    const SRC: &str = r#"
one: (A: Int) -> (B: String) -> Int = (x) => 1
none: (A: Int) -> (B: String) -> Int = () => 1
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert — 反向（实现少于声明）不报：缺席位由 #295 用声明名补齐
    assert!(
        codes.is_empty(),
        "实现头不超过声明值位时不得报 E1112：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_borrow_param_value_return_aligned() {
    // Arrange — 声明形参是借用、返回位是**同一个**类型变量：`&T` 与 `T` 形不同而
    // 语义同（借用是编译期零大小类型，值位是同一个值）。两种出口形态都要覆盖：
    // 箭头体（尾表达式）与块体（显式 return）。
    const SRC: &str = r#"
pick: (T: Type) -> (x: &T) -> T = (x) => x
pick_block: (T: Type) -> (x: &T) -> T = { x }
pick_ret: (T: Type) -> (x: &T) -> T = { return x }
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert — 出口对照必须认「借用形 ↔ 值形同形态」，不得误报 E1012。
    //
    // 因果链订正（WBS 6.8.3 验证轮实测）：本条**不是**递归修复的承重判据。修复前
    // （函数体形参取值级签名替换结果）`unify(&t, t)` 会建自指绑定、`expand_type`
    // 无限自递归；6.8.3 把体内绑定改成取声明标注原文（`TypeRef` 名义类型）后，出口
    // unify 直接成功、走不到那条 bind（闸掉入口对齐与 occurs check 的 Ref 臂，全量
    // 用例仍绿）。故本条的职责只剩**语义**：借用形与值形在值位是同一个值，三种出口
    // 形态都不得报错。判据承重在单元级（`types::tests::mono` 的同形态真值表）。
    assert!(
        codes.is_empty(),
        "借用形参的返回值必须与值形返回位相容（不得误报 E1012）：{codes:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-1（WBS 6.8.3 复审阻塞项 → WBS 6.8.4-1 口径收口）：本层返回位**是**类型位置
// ─────────────────────────────────────────────────────────────────────────────
//
// 判据口径 = SPEC `type-system.md` §7.1「两条序列」：名字在**任一** Type 子树（含本层
// 返回位）被引用 ⇒ 编译期值参数 ⇒ 不进**实现头绑定序列**（形参 − 类型参数 − 编译期值
// 参数）；调用点实参序列另算（编译期值参数仍占实参位）。故下列形态的实现头不得绑定 `a`：
// 实现头写了 1 位而声明只有 0 个可绑位 ⇒ **E1112**。
//
// 判据落在 typecheck 层（IR 规范化拿不到该形态的文本，进不了语料快照）；箭头体与块体
// 两种合成路径必须**同码**。

#[test]
fn test_param_used_in_own_return_type_still_rejects_bad_body() {
    // Arrange — 声明返回位用到了本层的值形参（依赖类型），体却直接返回那个值。
    // `Vec` / `Option` / `Array` / 嵌套四种同形写法都要覆盖，箭头体与块体各一份。
    const SRC: &str = r#"
vec_arrow: (a: Int) -> Vec(a) = (a) => a
vec_block: (a: Int) -> Vec(a) = { a }
opt_arrow: (a: Int) -> Option(a) = (a) => a
arr_arrow: (a: Int) -> Array(a, 2) = (a) => a
nested_arrow: (a: Int) -> Vec(Vec(a)) = (a) => a
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert — 五格都必须报 **E1112**（实现头位数 > 实现头绑定序列长度）：SPEC §7.1
    // 把本层返回位的类型实参算作「类型位置」⇒ `a` 是编译期值参数 ⇒ 声明没有可绑位，
    // 而实现头写了 1 位。判据仍是「必须被拒」——静默通过才是静默错答。
    assert_eq!(
        codes.iter().filter(|c| *c == "E1112").count(),
        5,
        "返回位用到本层值形参时，实现头多出的位必须报 E1112（不得静默通过）：{codes:?}"
    );
}

#[test]
fn test_param_used_in_own_return_type_keeps_annotation_alignment() {
    // Arrange — 作者标注与声明**逐字一致**，且声明返回位用到了本层形参：
    // `b` 的值参与 `Vec(a)`、返回位也是 `Vec(a)`。
    const SRC: &str = r#"
arrow: (a: Int, b: Vec(a)) -> Vec(a) = (a: Int, b: Vec(a)) => b
block: (a: Int, b: Vec(a)) -> Vec(a) = { b }
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert — 两格都报 **E1112**（不是 E1011）：按 SPEC §7.1，`a` 被 `Vec(a)` 引用 ⇒
    // 实现头绑定序列只剩 `b` 一位，而实现头写了 2 位。作者标注的逐字一致性不再能救它
    // （标注对的是声明原文，位次对的是绑定序列）——诊断码单点，不得混成 E1011。
    assert_eq!(
        codes.iter().filter(|c| *c == "E1112").count(),
        2,
        "逐字一致标注 + 依赖类型值形参必须报 E1112，且不得混进 E1011：{codes:?}"
    );
}
// ─────────────────────────────────────────────────────────────────────────────
// F1f（WBS 6.8.3）：依赖类型值形参 / 编译期 const 位的按位对齐——typecheck 层判据
// ─────────────────────────────────────────────────────────────────────────────
//
// 这两条形态进不了语料：`Vec(a)` 的 IR 规范化拿不到文本，快照判据会判「跳过即红」
//（09a §P6 6.8.2 判据③ 的登记）。故判据落在 **typecheck 层**：本组只跑
// lex → parse → typecheck，不碰 IR。

#[test]
fn test_dependent_value_param_stays_runtime_slot() {
    // Arrange — `a` 的**值**参与 `b` 的类型（依赖类型），它仍是运行期值位；
    // 实现头的两条合成路径（箭头体 / 块体）都必须保住两位并按位对上声明。
    const SRC: &str = r#"
arrow: (a: Int, b: Vec(a)) -> Int = (x, y) => x
block: (a: Int, b: Vec(a)) -> Int = { a }
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert — 两格都报 **E1112**：`a` 的值参与 `b` 的标注类型（`Vec(a)`），按 SPEC §7.1
    // 它是编译期值参数 ⇒ 实现头绑定序列只有 `b` 一位，实现头的两位无处可绑。
    assert_eq!(
        codes.iter().filter(|c| *c == "E1112").count(),
        2,
        "依赖类型的值形参按 §7.1 不占实现头绑定位，多出的位必须报 E1112：{codes:?}"
    );
}

#[test]
fn test_const_generic_param_takes_no_impl_head_slot() {
    // Arrange — `N` 出现在**返回位**的类型位置 ⇒ 编译期 const 位，不占实现头形参；
    // 块体合成出来的头必须只保留值位（`N` 在体内仍可引用，由 const 参数链提供）。
    const SRC: &str = r#"
f: (N: Int, s: String) -> Int = { N }
g: (N: Int) -> (n: N) -> Int = { n }
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert — 零诊断：N 若被算进实现头，位次前移会把 s/String 对到 N/Int 上报 E1011。
    assert!(
        codes.is_empty(),
        "编译期 const 位不得占实现头形参位、值位必须按位对齐：{codes:?}"
    );
}
// ─────────────────────────────────────────────────────────────────────────────
// E1011 判据的**求解器沙箱**边界（WBS 6.8.2b 退役面核对 + 复审关注点）
// ─────────────────────────────────────────────────────────────────────────────
//
// 形参标注冲突用「求解器克隆沙箱」判定：沙箱里的统一结论**不回写**主求解器。
// 两个方向都必须钉住：
// - 不泄漏：沙箱若把声明的类型变量先绑到实现头的 TypeRef 上，体内 return 的
//   对照会拿到错口径（`pick: (T: Type) -> (x: &T) -> T = (x) => x` 会被误报 E1012）；
// - 不吞错：沙箱只是「兼容性探测」，真实的类型错误仍必须照常报出。

#[test]
fn test_fn_annotation_conflict_sandbox_does_not_leak_into_solver() {
    // Arrange：声明返回 T、形参 &T，体内直接返回形参（T 是类型参数的新鲜变量）
    const SRC: &str = r#"
pick: (T: Type) -> (x: &T) -> T = (x) => x
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：沙箱探测后主求解器仍保持「T 未绑定」——不得冒出错口径的 E1012
    //（若泄漏，T 会被绑成 &T，随后 return 的对照必然报 E1012）
    assert!(
        !codes.iter().any(|c| c == "E1012"),
        "冲突判定的沙箱统一不得泄漏进主求解器（否则误报 E1012）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_conflict_sandbox_still_reports_real_error() {
    // Arrange：同一泛型形态，但返回位写死 Int、体内返回 &T——真实错误
    const SRC: &str = r#"
pick: (T: Type) -> (x: &T) -> Int = (x) => x
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：沙箱不吞错——返回值与声明返回位的真实不符仍报 E1012
    assert!(
        codes.iter().any(|c| c == "E1012"),
        "沙箱只做兼容性探测，真实返回位错误必须照常报 E1012：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_param_conflict_keeps_declared_type_for_callers() {
    // Arrange：定义处形参标注冲突（E1011），调用处按**声明**类型传错实参
    const SRC: &str = r#"
f: (a: Int) -> Int = (a: String) => a
main: () -> Void = {
    x: Int = f("hello")
}
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：两处真实错误都在（定义 E1011 + 调用 E1002），且注册形态仍是声明类型
    //（若冲突判定污染了注册类型，调用点这里就不会再报 E1002）
    assert!(
        codes.iter().any(|c| c == "E1011"),
        "定义处的形参标注冲突必须报 E1011：{codes:?}"
    );
    assert!(
        codes.iter().any(|c| c == "E1002"),
        "调用点仍按声明类型判实参（E1002）——注册形态不得被冲突判定污染：{codes:?}"
    );
}
