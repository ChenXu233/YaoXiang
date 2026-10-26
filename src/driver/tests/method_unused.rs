//! 方法死代码判据的驱动级钉测试（WBS 4.9.6 / issue #450，用户 2026-10-16 裁决 A；
//! 跨单元归因 WBS 4.9.7 / issue #464）
//!
//! 判据（D 类修复：删掉误报即改变诊断对外面）：
//! ① 无项目根路径下**被调用**的方法（含重载决议生效）不得报 W1005；
//! ② 同名重载**各自独立判定**——逐定义一条、不互相覆盖（旧实现 `all_defs` 按
//!    full name 键控，两个重载只剩一条，于是「2 个重载报 1 条」）；
//! ③ **未被调用**的方法仍须如实报 W1005（不得借修复放宽判据）。
//!
//! 跨单元（4.9.7 / #464，可达形态 = 调用方与定义方同处编译集）：
//! ④ 名字组有决议条目时按**定义单元**判定：调用方拼不出另一单元的类型名
//!    （`use b` 只把该文件带进编译集）⇒ 那一单元未被调用的同名同序定义如实报
//!    W1005。修复前回落池的**名字级**豁免（`RefPool::field_names` 短名全局平铺）
//!    ⇒ 不报（红态实测见下）；
//! ⑤ 误报防线：两单元各含同名同序方法且**都被调用** ⇒ 两侧都不报。内联别名
//!    （`use b.{Temp as TempB}`）与模块限定构造（`b.Temp(..)`）两种形态的调用点
//!    键都是**定义侧**裸名（实测都落 `Temp.to`），而 t19 的注入面按收窄规则不注入
//!    它们 ⇒ 归因必须按「记录方能拼出该类型名的模块集合」并入
//!    （`helpers.rs::typed_owners` 规则②），否则真实被调用的定义被误报；
//! ⑥ 键作用域双向：同单元未被命中的兄弟候选仍报（不过窄）；另一单元的未调用
//!    定义不被同名兄弟的调用豁免（不过宽）；
//! ⑦ 驱动侧读取是**纯读**：经 `OverloadResolutions::iter`，不触达 `get`、不写
//!    消费台账——否则 W1090「消费必达」结算会被驱动侧的读取掩盖（读一次 = 记一次消费）。
//!
//! 红态（4.9.6 修复前，实测）：语料 `tests/yaoxiang/02-type-system/functions/
//! method_call_args.yx` 3 条 W1005（三个方法都被调用）、`method_overload.yx`
//! 1 条（两个重载都被调用，却只报 1 条）、`06-compile-errors/call-args/
//! method_overload_ambiguous_err.yx` 1 条（两个重载各占一条定义，只报 1 条）。
//! 夹具形态与语料同构：**无 manifest**（rootless / Script 语义，无包内引用池）。
//!
//! 红态（4.9.7 修复前，实测）：④ 的三文件夹具 `check main.yx` 只报 a.yx 的
//! `Temp.other`（:8）与 b.yx 的未使用类型 W1002，**b.yx 的未调用 `Temp.to` 不报**
//! ——跨单元同名同序的定义靠池的名字级豁免蒙混过关。
//! 残余边界（登记，宁漏报方向）：单独 `check <定义文件>`（调用方不在单元集）时
//! 没有决议条目 ⇒ 回落名字级豁免 ⇒ 不报；调用方只写 `b.Temp(..)` 构造而从不调用
//! 其方法时，该单元的同名同序方法也不报（归因按名字面的**可能**归属并入）。

use std::path::PathBuf;

use super::check_rootless_mono::{check_codes, make_rootless_project, run_check};
use super::driver::{make_script_project, run_merged_slot_values, run_multi};
use super::ref_pool::assert_warnings;
use crate::driver::helpers::resolved_method_marks;
use crate::frontend::core::typecheck::types::OverloadResolutions;
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::util::plan_id::PlanId;

/// 带 `[run].main` 声明面的多文件夹具——`yaoxiang check main.yx` 的直驱等价形态：
/// 池按 manifest 声明面的包目标装配（`helpers.rs::collect_package_refs`），警告面
/// 覆盖包内文件（`DeadCodeAnalysis` 臂的 `in_project` 门控）。
fn make_declared_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n\n[run]\nmain = \"main.yx\"\n",
    )
    .expect("write manifest");
    let mut entry = PathBuf::new();
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).expect("write project file");
        if *name == "main.yx" {
            entry = dir.path().join(name);
        }
    }
    assert!(entry.is_file(), "夹具必须含 main.yx（入口约定）");
    (dir, entry)
}

/// 跨单元同名同序夹具（④ 主验收，可达形态）：a.yx 的 `Temp.to` 有调用点（:4）、
/// `Temp.other` 没有（:8）；b.yx 的同名同序 `Temp.to` 没有（:4）。入口只用
/// `use a.{Temp}` 命名类型，`use b` 只把 b 带进编译集 ⇒ 调用方拼不出 b 的类型名
/// （这正是「跨单元同名同序的未调用定义」此前漏报的形态）。
const CROSS_UNIT_A: &str = "// a.yx —— 被调用的 Temp.to（:4）+ 未被调用的 Temp.other（:8）\nTemp: Type = { v: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    self.v + 1.0\n}\n\nTemp.other: (self: &Temp) -> Float = (self) => {\n    self.v\n}\n";

/// 同名同序、无调用点的 `Temp.to`（:4）——b 只被入口的 `use b` 带进编译集。
const CROSS_UNIT_B: &str = "// b.yx —— 同名同序、无调用点的 Temp.to（:4）\nTemp: Type = { v: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    self.v * 10.0\n}\n";

/// ④ 的入口：类型只经 a 的裸名命名，b 只进编译集（拼不出 b 的类型名）。
const CROSS_UNIT_MAIN_BARE: &str = "// main.yx —— 只用 a 的 Temp 命名类型；use b 只把 b 带进编译集\nuse std.assert\nuse a.{Temp}\nuse b\n\nmain = () => {\n    t = Temp(1.0)\n    x: Float = t.to()\n    assert.assert(x == 2.0, \"a.to -> v+1\")\n}\n";

/// ⑤ 的入口（内联别名形态）：a 经裸名、b 经 `TempB` 别名——两侧都被调用。
const CROSS_UNIT_MAIN_ALIAS: &str = "// main.yx —— 两个类型各自被调用：a 经裸名、b 经内联别名\nuse std.assert\nuse a.{Temp}\nuse b.{Temp as TempB}\n\nmain = () => {\n    t = Temp(1.0)\n    x: Float = t.to()\n    u = TempB(2.0)\n    y: Float = u.to()\n    assert.assert(x == 2.0, \"a.to -> v+1\")\n    assert.assert(y == 20.0, \"b.to -> v*10\")\n}\n";

/// ⑤ 的入口（模块限定构造形态）：a 经裸名、b 经 `b.Temp(..)`——两侧都被调用。
const CROSS_UNIT_MAIN_QUALIFIED: &str = "// main.yx —— 两个类型各自被调用：a 经裸名、b 经模块限定构造\nuse std.assert\nuse a.{Temp}\nuse b\n\nmain = () => {\n    t = Temp(1.0)\n    x: Float = t.to()\n    u = b.Temp(2.0)\n    y: Float = u.to()\n    assert.assert(x == 2.0, \"a.to -> v+1\")\n    assert.assert(y == 20.0, \"b.to -> v*10\")\n}\n";

/// ⑥ 的定义侧：三个同名候选（声明序 0/1/2 → :4 / :8 / :12）+ 未被调用的 other（:16）。
const CROSS_UNIT_OVERLOAD_A: &str = "// a.yx —— 三个同名候选，只命中声明序 0（:4）；other 未被调用（:16）\nTemp: Type = { v: Float }\n\nTemp.to: (self: &Temp, n: Int) -> Int = (self, n) => {\n    n\n}\n\nTemp.to: (self: &Temp, s: String) -> String = (self, s) => {\n    s\n}\n\nTemp.to: (self: &Temp, f: Float) -> Float = (self, f) => {\n    f\n}\n\nTemp.other: (self: &Temp) -> Float = (self) => {\n    self.v\n}\n";

/// ⑥ 的入口：调用点只命中 a 的声明序 0 候选（Int）。
const CROSS_UNIT_OVERLOAD_MAIN: &str = "// main.yx —— 调用点只命中 a 的声明序 0 候选（Int）\nuse std.assert\nuse a.{Temp}\nuse b\n\nmain = () => {\n    t = Temp(1.0)\n    x: Int = t.to(1)\n    assert.assert(x == 1, \"a.to#0\")\n}\n";

/// 4.9.6 / #450 ①：被调用的方法不得报 W1005（无项目根路径）。
#[test]
fn test_called_method_is_not_reported_unused() {
    // Arrange：Temp.get 被 main 调用（同文件、无 manifest ⇒ 只按文件内定义-使用图判定）
    let src = "Temp: Type = { value: Float }\n\nTemp.get: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n    x = t.get()\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：方法被调用 ⇒ 无 W1005（类型与函数也不该报）
    let codes = check_codes(&outcome, "main.yx");
    assert!(
        !codes.iter().any(|c| c == "W1005"),
        "被调用的方法不得报 W1005，实际: {codes:?}"
    );
}

/// 4.9.6 / #450 ②：同名重载各自独立判定——两个都未被调用时**各报一条**。
#[test]
fn test_same_named_overloads_report_independently() {
    // Arrange：Temp.to 两个重载都未被调用（逐定义一条，不互相覆盖）
    let src = "Temp: Type = { value: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nTemp.to: (self: &Temp) -> String = (self) => {\n    return \"hot\"\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：两个重载各报一条 W1005（旧实现按 full name 键控只剩一条）
    let codes = check_codes(&outcome, "main.yx");
    let unused_methods = codes.iter().filter(|c| *c == "W1005").count();
    assert_eq!(
        unused_methods, 2,
        "两个同名重载都未被调用时必须各报一条 W1005，实际: {codes:?}"
    );
}

/// 4.9.6 / #450 ③：未被调用的方法仍须如实报 W1005（修复不得放宽判据）。
#[test]
fn test_uncalled_method_still_reported() {
    // Arrange：Temp.get 无任何调用点
    let src = "Temp: Type = { value: Float }\n\nTemp.get: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：未被调用的方法必须报 W1005
    let codes = check_codes(&outcome, "main.yx");
    assert!(
        codes.iter().any(|c| c == "W1005"),
        "未被调用的方法必须报 W1005，实际: {codes:?}"
    );
}

/// 4.9.6 / #450 ①+③ 逐候选（验收③「含未被重载决议命中」）：
/// 一组同名重载**仅一个被调用**时，未被决议命中的兄弟重载仍须报 W1005——
/// 且必须报**正确的那一个**（行号断言 = 声明序 ↔ 决议名 #k 的配对实测）。
#[test]
fn test_partially_called_overload_group_reports_the_unhit_sibling() {
    // Arrange：两个 Temp.to 重载（第 3 行 ->Float 声明序 0、第 7 行 ->String 声明序 1），
    // 调用点带类型注解 ⇒ 重载决议生效（D20 决议表记下被选中的那一个）
    let call_first = "Temp: Type = { value: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nTemp.to: (self: &Temp) -> String = (self) => {\n    return \"hot\"\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n    a: Float = t.to()\n}\n";
    let call_second = "Temp: Type = { value: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nTemp.to: (self: &Temp) -> String = (self) => {\n    return \"hot\"\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n    b: String = t.to()\n}\n";
    let (_d1, first_entry) = make_rootless_project(&[("main.yx", call_first)]);
    let (_d2, second_entry) = make_rootless_project(&[("main.yx", call_second)]);

    // Act：分别调用声明序 0 / 声明序 1 的候选
    let out_first = run_check(&first_entry).expect("check 必须返回产物");
    let out_second = run_check(&second_entry).expect("check 必须返回产物");

    // Assert：命中 ->Float（:3）时兄弟 ->String（:7）报；命中 ->String（:7）时兄弟
    // ->Float（:3）报——行号方向即配对方向，配错会在此翻红
    assert_warnings(&out_first, "main.yx", &[("W1005", 7)], &[("W1005", 3)]);
    assert_warnings(&out_second, "main.yx", &[("W1005", 3)], &[("W1005", 7)]);
}

/// 4.9.7 / #464 ④（主验收，可达形态）：跨单元同名同序的**未调用**定义如实报 W1005。
/// 修复前该定义靠池的名字级豁免（短名平铺的 `to`）蒙混过关——红态实测为「不报」。
#[test]
fn test_cross_unit_uncalled_same_named_method_is_reported() {
    // Arrange：a 的 Temp.to 有调用点、Temp.other 没有；b 的同名同序 Temp.to 没有
    let (_dir, entry) = make_declared_project(&[
        ("a.yx", CROSS_UNIT_A),
        ("b.yx", CROSS_UNIT_B),
        ("main.yx", CROSS_UNIT_MAIN_BARE),
    ]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：b 的未调用定义必须报；a 的被调用定义不报、a 的未调用兄弟（:8）仍报
    assert_warnings(&outcome, "b.yx", &[("W1005", 4)], &[]);
    assert_warnings(&outcome, "a.yx", &[("W1005", 8)], &[("W1005", 4)]);
}

/// 4.9.7 / #464 ⑤ 误报防线（内联别名形态）：两单元各含同名同序方法且**都被调用**
/// ⇒ 两侧都不报。别名调用点的键是**定义侧**裸名（实测落 `Temp.to`），而该模块按
/// t19 收窄规则不进注入面 ⇒ 归因必须按记录方名字面并入（`typed_owners` 规则②）。
#[test]
fn test_cross_unit_same_named_method_called_through_alias_is_not_reported() {
    // Arrange：a 经裸名调用、b 经 `TempB` 别名调用（同名同序、两侧都活）
    let (_dir, entry) = make_declared_project(&[
        ("a.yx", CROSS_UNIT_A),
        ("b.yx", CROSS_UNIT_B),
        ("main.yx", CROSS_UNIT_MAIN_ALIAS),
    ]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：两侧被调用的 Temp.to（:4）都不得报（同单元未被调用的 other 仍报 :8）
    assert_warnings(&outcome, "a.yx", &[("W1005", 8)], &[("W1005", 4)]);
    assert_warnings(&outcome, "b.yx", &[], &[("W1005", 4)]);
}

/// 4.9.7 / #464 ⑤ 误报防线（模块限定构造形态）：`b.Temp(..)` 之后的 `.to()` 同样落
/// 定义侧键名 ⇒ 该模块必须并入可能归属，两侧都不报。
#[test]
fn test_cross_unit_same_named_method_called_through_qualified_path_is_not_reported() {
    // Arrange：a 经裸名调用、b 经 `b.Temp(..)` 构造后调用（同名同序、两侧都活）
    let (_dir, entry) = make_declared_project(&[
        ("a.yx", CROSS_UNIT_A),
        ("b.yx", CROSS_UNIT_B),
        ("main.yx", CROSS_UNIT_MAIN_QUALIFIED),
    ]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：两侧被调用的 Temp.to（:4）都不得报（同单元未被调用的 other 仍报 :8）
    assert_warnings(&outcome, "a.yx", &[("W1005", 8)], &[("W1005", 4)]);
    assert_warnings(&outcome, "b.yx", &[], &[("W1005", 4)]);
}

/// 4.9.7 / #464 ⑥ 键作用域双向：a 的三个同名候选只命中声明序 0 ⇒ 未被命中的两个
/// 兄弟仍报（不过窄）；b 的同名同序定义没有任何调用点、调用方也拼不出 b 的类型名
/// ⇒ 不得被 a 的调用跨单元豁免（不过宽）。
#[test]
fn test_cross_unit_key_scope_reports_unhit_siblings_without_cross_unit_exemption() {
    // Arrange：a 侧三候选只命中声明序 0；b 侧同名同序定义无人调用
    let (_dir, entry) = make_declared_project(&[
        ("a.yx", CROSS_UNIT_OVERLOAD_A),
        ("b.yx", CROSS_UNIT_B),
        ("main.yx", CROSS_UNIT_OVERLOAD_MAIN),
    ]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：a 的未命中兄弟（:8 / :12）与未调用 other（:16）报、被命中的（:4）不报；
    // b 的未调用同名同序定义（:4）也报（跨单元不互相豁免）
    assert_warnings(
        &outcome,
        "a.yx",
        &[("W1005", 8), ("W1005", 12), ("W1005", 16)],
        &[("W1005", 4)],
    );
    assert_warnings(&outcome, "b.yx", &[("W1005", 4)], &[]);
}

/// 4.9.7 / #464 ⑦ 驱动侧读取是纯读：读归因标记**不触达** `OverloadResolutions::get`
/// ⇒ 决议条目仍在未消费差集里——否则「消费必达」结算（W1090）会被驱动侧的读取掩盖
/// （读一次 = 记一次消费）。空 `units`/`asts`：本条只钉读取面本身。
#[test]
fn test_resolved_method_marks_read_leaves_consumption_ledger_untouched() {
    // Arrange：一条已登记、尚无消费者（ir_gen 未跑）的决议条目
    let call = PlanId::from_ordinal(7);
    let mut result = TypeCheckResult::default();
    result.overload_resolutions =
        OverloadResolutions::from_entries([(call, "Temp.to".to_string())]);

    // Act：驱动侧唯一读取面
    let marks = resolved_method_marks(&[], &[], &[Some(&result)]);

    // Assert：读取面命中该条目（不是空转）；台账的未消费差集不变（纯读）
    assert!(
        marks.groups.contains("Temp.to"),
        "读取面必须命中决议条目（否则本条断言空转）: {:?}",
        marks.groups
    );
    assert_eq!(
        result.overload_resolutions.unconsumed(),
        vec![call],
        "驱动侧读取不得计入消费台账（否则掩盖 W1090 结算）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// WBS 3.20.3 / issue #505：算子派发调用的接口方法不得报 W1005
// ─────────────────────────────────────────────────────────────────────────────
//
// 判据（issue #505 终态判据，D 类：删误报即改变警告面）：
// ① 类型体内 `Index(Temp, Int, Float)` / `Equal(Temp, Temp)` 声明 + 算子表达式
//    `u[0]` / `u == t` 实际派发调用（run 值正确）⇒ `Temp.index` / `Temp.equal`
//    不报 W1005——派发的具名决议是 `OperatorDispatch { type_name, method }`
//    （RFC-011b，独立于 D20 重载决议表的义务通道），修复前死代码分析只消费
//    后者 ⇒ 两形态（单文件 2 条 / 跨单元 4 条）全部误报（红态实测）；
// ② 守卫（不得借修复放宽）：接口声明了但算子从未作用于该类型 ⇒ 照报 W1005；
// ③ 值级：跨单元派发的运行值保持 20.0 / false（修复只动警告面，IR 零漂移）。
//
// 红态（修复前，实测）：① 的单文件 `check` 报 2 条 W1005（Temp.index :6 /
// Temp.equal :9）；跨单元（issue 原文夹具）报 4 条（a.yx 与 b.yx 各 :6 / :9）；
// `run` 值始终正确（20.0 / false）——方法确实被调用，警告为误报。

/// issue #505 跨单元夹具的 a 侧（v+1.0 / true）：W1005 红态落在 :6 与 :9。
const OP_DISPATCH_A: &str = "// a.yx —— Index/Equal 接口 + 两方法（被 main 的算子表达式派发调用）\nTemp: Type = {\n    v: Float,\n    Index(Temp, Int, Float),\n    Equal(Temp, Temp),\n}\nTemp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {\n    self.v + 1.0\n}\nTemp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {\n    true\n}\n";

/// issue #505 跨单元夹具的 b 侧（v*10.0 / false）：同名同序，红态同 :6 / :9。
const OP_DISPATCH_B: &str = "// b.yx —— 同构接口与两方法（值面不同：v*10.0 / false）\nTemp: Type = {\n    v: Float,\n    Index(Temp, Int, Float),\n    Equal(Temp, Temp),\n}\nTemp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {\n    self.v * 10.0\n}\nTemp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {\n    false\n}\n";

/// issue #505 跨单元入口：`u[0]` / `u == t` 两个派发点（红态 = a/b 各 2 条 W1005）。
const OP_DISPATCH_MAIN: &str = "// main.yx —— 两个算子表达式分别派发 b.Temp.index / Equal(Temp, Temp)\nuse a.{Temp}\nuse b.{Temp as TempB}\n\nmain = () => {\n    t = Temp(1.0)\n    u = TempB(2.0)\n    println(u[0])\n    println(u == t)\n}\n";

/// ① 跨单元主验收：两单元四个接口方法全被算子派发调用 ⇒ check 诊断集不含
/// W1005（修复前 4 条，红态实测）。清单带 `[run].main` 声明面（issue 原文
/// 「清单合法、含 version」）⇒ 警告面覆盖包内文件。
#[test]
fn test_operator_dispatched_methods_not_reported_unused_cross_unit() {
    // Arrange：issue #505 原文最小复现（a/b 两单元 + main + 声明面清单）
    let (_dir, entry) = make_declared_project(&[
        ("a.yx", OP_DISPATCH_A),
        ("b.yx", OP_DISPATCH_B),
        ("main.yx", OP_DISPATCH_MAIN),
    ]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：a/b 的 index/equal（首行是注释 ⇒ 定义在 :7/:10）都被派发调用
    // ⇒ 四条 W1005 全部消失
    assert_warnings(&outcome, "a.yx", &[], &[("W1005", 7), ("W1005", 10)]);
    assert_warnings(&outcome, "b.yx", &[], &[("W1005", 7), ("W1005", 10)]);
}

/// ① 单文件主验收：同构夹具单文件内 `u[0]` / `u == u` 派发调用 ⇒ 不报 W1005
/// （修复前 2 条，红态实测）。rootless 单文件语义（Script 不接池，判定只看
/// 文件内定义-使用图 + 算子派发组）。
#[test]
fn test_operator_dispatched_methods_not_reported_unused_single_file() {
    // Arrange：类型 + 接口声明 + 两方法 + main 内两个算子表达式，同文件
    let src = "Temp: Type = {\n    v: Float,\n    Index(Temp, Int, Float),\n    Equal(Temp, Temp),\n}\nTemp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {\n    self.v + 1.0\n}\nTemp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {\n    true\n}\nmain = () => {\n    u = Temp(19.0)\n    println(u[0])\n    println(u == u)\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：被派发调用的两个方法（:6/:9）不得报 W1005
    assert_warnings(&outcome, "main.yx", &[], &[("W1005", 6), ("W1005", 9)]);
}

/// ② 守卫（不得放宽判据）：接口声明了、方法定义了，但 main 从不用
/// `u[0]` / `u == t`（只读字段）⇒ 两方法仍须如实报 W1005（:6/:9）。
/// 修复前修复后都必须绿——翻红即修复把判据放宽了。
#[test]
fn test_operator_interface_declared_without_use_still_reports_unused_method() {
    // Arrange：与单文件主验收同构，去掉全部算子表达式（改读字段）
    let src = "Temp: Type = {\n    v: Float,\n    Index(Temp, Int, Float),\n    Equal(Temp, Temp),\n}\nTemp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {\n    self.v + 1.0\n}\nTemp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {\n    true\n}\nmain = () => {\n    u = Temp(1.0)\n    println(u.v)\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：无算子派发 ⇒ 接口方法照报 W1005
    assert_warnings(&outcome, "main.yx", &[("W1005", 6), ("W1005", 9)], &[]);
}

/// ③ 值级：跨单元派发的运行值保持 20.0（b.Temp.index = v*10.0）与 false
/// （b.Temp.equal）——修复只动警告面，不触 IR 与派发（修前修后都绿；
/// 修复若改变行为则在此翻红）。
#[test]
fn test_operator_dispatch_cross_unit_run_values_unchanged() {
    // Arrange：rootless Script 形态（顶层绑定即程序体，值进全局槽位可观察）
    let main = "use a.{Temp}\nuse b.{Temp as TempB}\n\nt = Temp(1.0)\nu = TempB(2.0)\nindexed = u[0]\neq = u == t\n";
    let (_dir, entry) = make_script_project(&[
        ("a.yx", OP_DISPATCH_A),
        ("b.yx", OP_DISPATCH_B),
        ("main.yx", main),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：编译无故障，槽位值含 Float(20.0) 与 Bool(false)
    assert!(
        outcome.failure.is_none(),
        "跨单元算子派发不得编译失败: {:?}",
        outcome.failure
    );
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(20.0)"),
        "u[0] 必须派发 b.Temp.index（v*10.0 = 20.0）: {values:?}"
    );
    assert!(
        values.iter().any(|v| v == "Bool(false)"),
        "u == t 必须派发 b.Temp.equal（false）: {values:?}"
    );
}
