//! WBS 4.9.9 — **跨单元方法**的命名实参形参名表贯通：驱动级直驱钉测试。
//!
//! 规范来源：SPEC syntax §2.3（命名实参 `name = value`；带名参数**名字进契约、
//! 以声明为准**）+ type-system §3.5（带名参数可被命名实参引用）+ RFC-011a §3
//!（方法级重载候选的决议与混编 IR 名）+ RFC-029（跨文件方法调用）。
//!
//! 机制形态（coding-rules 第六部分「登记-消费」型）：声明期形参名由**定义单元**
//! 登记进 `TypeEnvironment::method_overload_param_names`，经
//! `method_overload_snapshot` → `ModuleInfo::method_overloads` 跨单元携带；
//! 消费端是 IR 生成的 `AstToIrGenerator::lookup_param_info`（按定义单元限定名
//! `{module}.{Type}.{method}[#N]` 取候选组的形参名表）。本文件**不手工构造**任何
//! 注册数据：全部经 Driver 的真实多文件通路（Discovery → Registry → Typecheck →
//! GlobalSlotAlloc → IrGeneration → Linking）产出——生产端与消费端同批直驱；
//! 消费端测试手工构造登记数据即静默通道（review 打回）。
//!
//! 修复前红态（第一次修复前，实测）：`error [E1014] Function 'scale_by' has no
//! parameter named 'k' (available: )`——IR 生成侧只按「本单元 `fn_param_names`」
//! 与「模块导出面」查名字，而方法键的首段是**类型名**不是模块键，导出面对它恒不命中。
//!
//! 修复轮 2（t2-F1 回归）：候选名字表按「目标名」归并后，**别名 + 裸名导入同名
//! 类型**（`use a.{Temp as X}` + `use b.{Temp}`）会被收窄规则③整名跳过 ⇒ 调用方
//! 没有候选名字表。第一次修复此时把**编译期响亮 E1014** 放成了**运行期静默丢参**
//! （check=0、run=1 运行期 E6007）。本文件后三条用例把「不静默丢弃命名实参」补齐
//! 到 IR 侧的兜底：拿不到名字表就响亮拒绝（既有 E1014，available 与共享判据同一
//! 口径），覆盖绑定臂 / 无决议普通方法臂 / 整体模块导入形态。

use std::path::Path;

use super::driver::{make_script_project, run_merged_slot_values, run_multi};
use crate::driver::{Driver, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};

/// 以 Check 形态跑多文件脚本项目 → （逐文件诊断码展平，是否未硬中止）。
///
/// 选 Check 形态的理由：IR 生成期诊断（E1014 只由 lower 产生）在 Check 形态
/// 收进逐文件通道，判据直接读码——不必依赖 CLI 退出码，也覆盖「硬中止会丢掉
/// 其余文件诊断」的契约（`outcome.failure.is_none()`）。
fn check_codes(entry: &Path) -> (Vec<String>, bool) {
    let program = Program::new(
        ProgramKind::Check,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    let outcome = Driver::new()
        .run(program)
        .expect("driver run must not error out");
    let codes = outcome
        .check_diagnostics
        .iter()
        .flat_map(|(_, diagnostics)| diagnostics.iter().map(|d| d.code.clone()))
        .collect();
    (codes, outcome.failure.is_none())
}

/// 合并 IR 里**被调名以 `suffix` 结尾**的调用 → `(被调名, 实参数)`（函数体 + init 段）。
///
/// 方法调用在 IR 里就是普通 `Call`：接收者占 `args[0]`，值实参从 `args[1]` 起
///（`lower/expr/call.rs` 的方法臂）。按被调名而不是源码位置筛查，判据与
/// 「派发名 = 决议出的混编名」这条既有契约同源。
fn calls_ending_with(
    module: &crate::middle::ModuleIR,
    suffix: &str,
) -> Vec<(String, usize)> {
    module
        .functions
        .iter()
        .flat_map(|function| function.all_instructions())
        .chain(module.init.iter())
        .filter_map(|instruction| match instruction {
            Instruction::Call {
                func: Operand::Const(ConstValue::String(name)),
                args,
                ..
            } => Some((name.clone(), args.len())),
            _ => None,
        })
        .filter(|(name, _)| name.ends_with(suffix))
        .collect()
}

/// 单候选跨单元方法：命名实参按**声明期形参名**入槽，接收者与命名实参都进 IR 调用。
///
/// 判别力：名字表拿不到时 IR 生成侧在 `bind_args_to_params` 报 E1014（编译中止、
/// 无合并 IR ⇒ 第一条断言红）；名字表拿对但实参序重建错时调用只带接收者（第二条红）；
/// 实参进 IR 但槽位错位时求值不是 6.0（第三条红）。
#[test]
fn test_cross_unit_method_named_arg_binds_declaration_parameter_name() {
    // Arrange
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "Point: Type = { x: Float, y: Float }\n\nPoint.scale_by: (self: &Point, k: Float) -> Float = {\n    return self.x * k\n}\n",
        ),
        (
            "main.yx",
            "use lib.{Point}\n\np = Point(3.0, 4.0)\nv = p.scale_by(k = 2.0)\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "跨单元方法的命名实参不得编译失败（修复前是 E1014 available 为空）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let calls = calls_ending_with(&module, "Point.scale_by");
    assert!(
        calls.iter().any(|(_, argc)| *argc == 2),
        "方法调用必须带接收者 + 命名实参两个实参（命名实参不得被丢弃），实际调用：{calls:?}"
    );
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(6.0)"),
        "self.x * k = 3.0 * 2.0 应求值为 Float(6.0)，实际全局槽位：{values:?}"
    );
}

/// 内联别名导入（`use lib.{Point as P}`）：候选注入面按**目标名**配对，
/// 命名实参同样按声明期形参名入槽。
///
/// 判别力：注入面按本地别名配对时别名导入的候选永远注入不进来 ⇒ 命名实参被静默
/// 丢弃（修复前实测运行期 E6007 `type mismatch in binary operation Mul`）——
/// 第一条断言红；别名路径若绕过候选决议，实参数同样不足两个（第二条红）。
#[test]
fn test_cross_unit_method_named_arg_alias_import_binds_parameter_name() {
    // Arrange
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "Point: Type = { x: Float, y: Float }\n\nPoint.scale_by: (self: &Point, k: Float) -> Float = {\n    return self.x * k\n}\n",
        ),
        (
            "main.yx",
            "use lib.{Point as P}\n\np = P(3.0, 4.0)\nv = p.scale_by(k = 2.0)\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "别名导入的跨单元方法命名实参不得编译失败：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let calls = calls_ending_with(&module, "Point.scale_by");
    assert!(
        calls.iter().any(|(_, argc)| *argc == 2),
        "别名导入形态同样必须带接收者 + 命名实参两个实参，实际调用：{calls:?}"
    );
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(6.0)"),
        "别名导入形态 self.x * k 应求值为 Float(6.0)，实际全局槽位：{values:?}"
    );
}

/// 两候选重载：命名实参按**形参名**筛候选（k → Float 候选、n → Int 候选），
/// 位置实参按实参类型决议——四条调用各两次落到两个候选上。
///
/// 判别力：名字表缺失 ⇒ 编译中止（第一条红）；按名筛候选若退化成「总取首候选」，
/// 命名 `n` 与位置 `3` 的调用都会落到 Float 候选（裸名 3 次 / 声明序 1 一次，
/// 第二条按次数分布红）；实参序错位则 `Int(30)` 求不出来（第四条红）。
#[test]
fn test_cross_unit_method_named_arg_selects_overload_by_parameter_name() {
    // Arrange：两个候选形参名互不相同（k: Float / n: Int），四处调用
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "Point: Type = { x: Float, y: Float }\n\nPoint.scale_by: (self: &Point, k: Float) -> Float = {\n    return self.x * k\n}\n\nPoint.scale_by: (self: &Point, n: Int) -> Int = {\n    return n * 10\n}\n",
        ),
        (
            "main.yx",
            "use lib.{Point}\n\np = Point(3.0, 4.0)\na = p.scale_by(k = 2.0)\nb = p.scale_by(n = 3)\nc = p.scale_by(2.0)\nd = p.scale_by(3)\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "两候选重载的命名实参调用不得编译失败：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    // 混编名是候选序的载体：裸名 = 声明序 0（Float 候选），后缀 #1 = 声明序 1（Int 候选）。
    let first_candidate = calls_ending_with(&module, "Point.scale_by");
    let second_candidate = calls_ending_with(&module, "Point.scale_by#1");
    assert!(
        first_candidate.iter().all(|(_, argc)| *argc == 2)
            && second_candidate.iter().all(|(_, argc)| *argc == 2),
        "四处调用都必须带接收者 + 一个值实参，实际：裸名 {first_candidate:?} / 声明序 1 {second_candidate:?}"
    );
    assert!(
        first_candidate.len() == 2 && second_candidate.len() == 2,
        "命名与位置两侧都要各命中两个候选（裸名 2 次、声明序 1 两次），实际：裸名 {first_candidate:?} / 声明序 1 {second_candidate:?}"
    );
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(6.0)"),
        "Float 候选（self.x * k）应求值为 Float(6.0)，实际全局槽位：{values:?}"
    );
    assert!(
        values.iter().any(|value| value == "Int(30)"),
        "Int 候选（n * 10）应求值为 Int(30)，实际全局槽位：{values:?}"
    );
}
/// F1 修复轮 3（t3 复审）：**混合别名 + 裸名导入同名类型**——候选按**裸名提供方**
/// 注入（`local == target` 优先）。裸名侧（b.Temp）的命名实参按 b 的声明名入槽：
/// 编译通过、值正确（2 + 3 = 5.0）。
///
/// 三态（同一夹具，见报告）：pre-t1 `1e05dc3e` = 0/0；修复轮 2 `091b835b` = E1002
/// （规则③把裸名提供方候选一起跳过，位置实参重载退回单绑定）；本修复后 = 0/0。
/// 名字层歧义的消费侧判据（WBS 4.9.9 修复轮 3 / 复审轮；用户裁决 A 口径）：**混合别名 +
/// 裸名导入同名类型**时，该类型上的**命名实参一律响亮拒绝**（E1014，共享判据的空表出口）。
///
/// 机制：候选注入面按提供方优先级只取裸名提供方的候选，而调用键前缀是定义侧类型名
/// ——两个同名类型共用同一个键：typecheck 会用裸名提供方的签名匹配，IR 却按接收者
/// 归属派发到别名模块的方法；形参名相同、类型不同时就是**静默错值**（实测别名侧
/// `x.bump(n = 2.0)` 曾打出 200.0：别名模块的方法体以 Float 执行 Int 参数）。
///
/// 三态（同一夹具，见报告）：pre-t1 `1e05dc3e` = check=1 E1014；第一次修复
/// `02ed42d2` = check=0/run=1（运行期 E6007）；中间一轮 `5edfb53e` = check=0/run=0
/// （静默打出 200.0）；本修复后 = check=1 E1014（响亮）。
#[test]
fn test_mixed_alias_bare_import_named_arg_rejects_loudly() {
    // Arrange：a 的 Temp 别名导入为 X（方法形参名 step_a），b 的同名 Temp 裸名导入
    // （方法形参名 step_b）；调用落在裸名导入的 b.Temp 上
    let (_dir, entry) = make_script_project(&[
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, step_a: Float) -> Float = {\n    return self.v + step_a\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, step_b: Float) -> Float = {\n    return self.v + step_b\n}\n",
        ),
        (
            "main.yx",
            "use a.{Temp as X}\nuse b.{Temp}\n\nt = Temp(2.0)\nv = t.bump(step_b = 3.0)\n",
        ),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1014"),
        "名字层歧义类型上的命名实参必须响亮拒绝（既有 E1014），不得静默错派/静默丢弃：{codes:?}"
    );
}
/// F1 修复轮 3（t3 复审）·别名侧边界：别名导入的 a.Temp 实例与裸名侧共用同一个
/// 裸名候选键，用 a 的形参名（step_a）调用必须**响亮拒绝**（E1014），不得静默
/// 错派到 b 的方法（「避免静默错派」= 别名侧评估结论）。
#[test]
fn test_mixed_alias_bare_import_alias_side_named_arg_rejects_loudly() {
    // Arrange：同一混合导入形态，调用落在别名导入的 X（= a.Temp）上，用 a 的形参名
    let (_dir, entry) = make_script_project(&[
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, step_a: Float) -> Float = {\n    return self.v + step_a\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, step_b: Float) -> Float = {\n    return self.v + step_b\n}\n",
        ),
        (
            "main.yx",
            "use a.{Temp as X}\nuse b.{Temp}\n\nx = X(1.0)\nv = x.bump(step_a = 5.0)\n",
        ),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1014"),
        "别名侧用 a 的形参名（step_a）必须响亮拒绝（既有 E1014），不得静默错派到 b 的方法：{codes:?}"
    );
}

/// F1 回归：**RFC-004 绑定臂**（类型体内 `shrink = shrink_impl[0]`）在被绑函数的
/// 形参名表不可得时同样响亮拒绝。
///
/// 注意边界：本形态的**位置实参**调用另有既有跨单元绑定缺陷（被绑函数名未限定
/// ⇒ 运行期 E6006），与本判据无关；本测试只钉「命名实参不静默丢弃」。
#[test]
fn test_cross_unit_named_arg_binding_arm_rejects_loudly() {
    // Arrange：类型体内绑定方法，被绑函数定义在另一个单元
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "Widget: Type = {\n    v: Float,\n    shrink = shrink_impl[0],\n}\n\nshrink_impl: (self: &Widget, delta: Float) -> Float = {\n    return self.v - delta\n}\n",
        ),
        (
            "main.yx",
            "use lib.{Widget}\n\nw = Widget(5.0)\nv = w.shrink(delta = 2.0)\n",
        ),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1014"),
        "绑定臂拿不到被绑函数的形参名表时，命名实参必须响亮拒绝（既有 E1014）：{codes:?}"
    );
}

/// F1 回归：**整体模块导入**（`use lib` + `lib.Circle(...)`）——收窄规则①不为
/// 整体导入产候选条目，调用方没有名字表；命名实参此前被静默丢弃（运行期
/// E6007），现同样响亮拒绝。
#[test]
fn test_cross_unit_named_arg_whole_module_import_rejects_loudly() {
    // Arrange
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "Circle: Type = { r: Float }\n\nCircle.area_with: (self: &Circle, factor: Float) -> Float = {\n    return self.r * self.r * factor\n}\n",
        ),
        (
            "main.yx",
            "use lib\n\nc = lib.Circle(2.0)\nv = c.area_with(factor = 3.0)\n",
        ),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1014"),
        "整体模块导入形态没有名字表时，命名实参必须响亮拒绝（既有 E1014）：{codes:?}"
    );
}
/// 名字层歧义下**位置实参保留注入候选**（4.9.8 的派发归因，判据必须继续 0/0）：同名
/// Temp 由 a（别名导入）与 b（裸名导入）两个模块提供，b 侧有两个同名重载；位置实参
/// 分别按 Float / Int 命中，且 IR 按接收者所属单元派发。
///
/// 三态（同一夹具，见报告）：pre-t1 `1e05dc3e` = 0/0；第一次修复后 `02ed42d2`
/// = check=1 E1002（规则③整名跳过，候选全丢）；中间一轮与本修复后 = 0/0（值断言
/// 5.0 / 30 证明派发与候选都正确）。
#[test]
fn test_mixed_alias_bare_import_keeps_bare_provider_overloads() {
    // Arrange：a 提供 Temp（别名导入为 X），b 提供同名 Temp 且有两个同名重载；
    // 调用只落在裸名导入的 b.Temp 上，位置实参分别命中 Float / Int 候选
    let (_dir, entry) = make_script_project(&[
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = {\n    return self.v\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, step: Float) -> Float = {\n    return self.v + step\n}\n\nTemp.bump: (self: &Temp, n: Int) -> Int = {\n    return n * 10\n}\n",
        ),
        (
            "main.yx",
            "use std.assert\nuse a.{Temp as X}\nuse b.{Temp}\n\nx = X(1.0)\nt = Temp(2.0)\ntotal = x.v + t.v\nf = t.bump(3.0)\ni = t.bump(3)\nassert.assert(total == 3.0, \"both imports used\")\nassert.assert(f == 5.0, \"Float overload: 2+3 = 5.0\")\nassert.assert(i == 30, \"Int overload: 3*10 = 30\")\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "名字层歧义下位置实参必须保留注入候选（4.9.8 判据）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(5.0)"),
        "位置实参 3.0 必须命中 Float 候选（2+3 = 5.0）并派发到 b.Temp，实际全局槽位：{values:?}"
    );
    assert!(
        values.iter().any(|value| value == "Int(30)"),
        "位置实参 3 必须命中 Int 候选（3*10 = 30）并派发到 b.Temp，实际全局槽位：{values:?}"
    );
}

/// F2 回归（t3 复审，4.9.9 修复轮 3）：**闭包 / 函数体内嵌套 lambda 调用**的命名实参
/// 保留旧行为（静默忽略，不因兜底变成编译期拒绝）。
///
/// 零参闭包 `f = () => 7; g = f(x = 1)` 在兜底前编译通过且结果正确（`g == 7`），
/// 按 07 的入口判据属「原先成功」——改判编译期错误是 D59 D 类，超出 4.9.9 目标。
/// 故 IR 兜底只作用于**声明函数名**，函数体内 lambda 绑定（`nested_functions`）不进兜底；
/// 「命名实参被静默忽略」登记为已知缺口（D60 ② 该报未报，A 类），见 09 的 4.9.9 行。
#[test]
fn test_dynamic_call_named_arg_keeps_legacy_behavior() {
    // Arrange：零参闭包 + 命名实参（旧行为：忽略命名实参、返回 7）；
    // 一参闭包 + 命名实参（旧行为：同样忽略，本判据只固定「不变成编译期拒绝」）
    let (_dir, entry) = make_script_project(&[(
        "main.yx",
        "use std.assert\n\nmain = () => {\n    f = () => 7\n    g = f(x = 1)\n    h = (a: Int) => a\n    k = h(a = 2)\n    assert.assert(g == 7, \"zero-param closure ignores named arg\")\n    return g\n}\n\nr = main()\n",
    )]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "闭包 / 嵌套 lambda 调用的命名实参必须保留旧行为，不得被兜底改判编译期拒绝：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Int(7)"),
        "零参闭包的命名实参按旧行为被忽略：f(x = 1) 返回 Int(7)，实际全局槽位：{values:?}"
    );
}

/// F3 回归（t3 复审，4.9.9 修复轮 3）：**内置容器构造**对非 `len` 的命名实参响亮拒绝。
///
/// `generate_builtin_container_ir` 原先只认 `len`，其余命名实参既不入槽也不拒绝
/// （`Vec(Int)(foo = 3)` 修复前后都 check=0 ⇒ D60 禁止的静默择一）。修复轮 3 复用
/// 共享兜底的空表出口报既有 E1014；`len` 形态不受影响（正控见下一条）。
#[test]
fn test_builtin_container_ctor_rejects_unknown_named_arg() {
    // Arrange：非 len 命名实参
    let (_dir, entry) = make_script_project(&[("main.yx", "v = Vec(Int)(foo = 3)\n")]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1014"),
        "容器构造的未知命名实参必须响亮拒绝（既有 E1014），不得静默忽略：{codes:?}"
    );
}

/// F3 正控：`Vec(Int)(len = 3)` 仍按槽位分配走通（不得被兜底误伤）。
/// F3 正控（含**值断言**）：`Vec(Int)(len = 3)` 仍按槽位分配走通——`len` 被静默忽略
/// 时该正控也必须能红，故断言 `v.length == 3`（脚本内 assert + 全局槽位 `Int(3)` 双证）。
#[test]
fn test_builtin_container_ctor_len_named_arg_still_passes() {
    // Arrange
    let (_dir, entry) = make_script_project(&[(
        "main.yx",
        "use std.assert\n\nv = Vec(Int)(len = 3)\nw = v.length\nassert.assert(w == 3, \"Vec(Int)(len = 3) allocates 3 slots\")\n",
    )]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "len 是容器构造的合法命名实参，不得被兜底改判编译期错误：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Int(3)"),
        "Vec(Int)(len = 3) 必须真的分配 3 个槽位（v.length == 3），实际全局槽位：{values:?}"
    );
}

/// 名字层歧义下**顶层闭包值绑定**的命名实参保留旧行为（顶层与函数体内一视同仁）。
///
/// 顶层 `f = () => 7; g = f(x = 1)` 在兜底之前编译通过且 `g == 7`（脚本内断言 + 槽位
/// `Int(7)` 双证）；只按 `nested_functions` 排除时顶层闭包不在该列表 ⇒ 被兜底改判
/// E1014（D59 D 类，已由本判据钉死）。
#[test]
fn test_top_level_closure_named_arg_keeps_legacy_behavior() {
    // Arrange：顶层（模块体）闭包 + 命名实参
    let (_dir, entry) = make_script_project(&[(
        "main.yx",
        "use std.assert\n\nf = () => 7\ng = f(x = 1)\nassert.assert(g == 7, \"top-level zero-param closure ignores named arg\")\n",
    )]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "顶层闭包值绑定的命名实参必须保留旧行为，不得被兜底改判编译期拒绝：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Int(7)"),
        "顶层零参闭包的命名实参按旧行为被忽略：f(x = 1) 返回 Int(7)，实际全局槽位：{values:?}"
    );
}

/// 具名嵌套函数声明（Fn 注解 + 声明形参）是**声明函数**，与闭包值、块体声明函数一样
/// **不进**共享兜底——命名实参在这些形态上保留旧行为（被忽略），不得因为兜底变严格而
/// 改判原先成功的程序（D59 D 类）。
///
/// 本例的函数体**忽略形参**，旧行为 = 编译通过、返回 7（命名实参被忽略）。该形态登记
/// 为已知缺口（D60 ② 该报未报，A 类），与闭包值、块体声明函数同列；函数体**消费**形参
/// 的同形态旧行为是运行期 E6007（同样保留，不在此断言）。
#[test]
fn test_named_nested_function_named_arg_keeps_legacy_behavior() {
    // Arrange：函数体内具名嵌套函数（body 忽略形参）+ 命名实参
    let (_dir, entry) = make_script_project(&[(
        "main.yx",
        "main = () => {\n    g: (a: Int, b: Int) -> Int = {\n        7\n    }\n    r = g(a = 1, b = 2)\n    return r\n}\n\nr = main()\n",
    )]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "具名嵌套函数声明的命名实参必须保留旧行为（不得被兜底改判编译期拒绝）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Int(7)"),
        "具名嵌套函数的命名实参按旧行为被忽略：g(a = 1, b = 2) 返回 Int(7)，实际全局槽位：{values:?}"
    );
}
/// 边界①：**两个裸名提供方冲突**——导入层既有判据已响亮拒绝（E5006 duplicate import），
/// 本判据固定它不被 4.9.9 的收窄/兜底改动静默掉。
#[test]
fn test_conflicting_bare_imports_reject_duplicate_type_name() {
    // Arrange：两个模块都用裸名导入同名类型
    let (_dir, entry) = make_script_project(&[
        ("a.yx", "Temp: Type = { v: Float }\n"),
        ("b.yx", "Temp: Type = { v: Float }\n"),
        ("main.yx", "use a.{Temp}\nuse b.{Temp}\n\nt = Temp(2.0)\n"),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E5006"),
        "两个裸名提供方冲突必须在导入层响亮拒绝（既有 E5006），不得被静默择一：{codes:?}"
    );
}

/// **不变量判据**（复审轮要求，裁决口径 b）：typecheck 采用的候选集与 IR 实际派发目标
/// 必须同源。用可观测形式表达：
/// - **歧义形态 + 命名实参**：check 必红（E1014）——名表来源不可信，不许静默；
/// - **歧义形态 + 位置实参**：check/run 必须 0/0，且值断言证明 IR 派发到**接收者所属
///   模块**（a 的 bump 返回 n*100，b 的 bump 返回 n*10，两实现可区分）；
/// - **非歧义形态**（仅别名 / 仅裸名单一提供方）：check/run 0/0 且派发到正确模块。
///
/// 该判据对三棵树都成立：pre-t1 与中间坏态的「歧义 + 命名实参」不红（静默丢参/错值），
/// 本修复后成立。位置实参一侧的候选/派发同源性风险另行由 4.9.10 的探针矩阵评估。
#[test]
fn test_method_identity_invariant_ambiguous_named_red_positional_green() {
    // Arrange：a.Temp.bump 返回 n*100（Int），b.Temp.bump 返回 v+step（Float）——
    // 两个实现的值可区分，故「派发到哪个模块」可观测
    // 两侧签名**相同**（`bump(n: Int) -> Int`），实现可区分（a: n*100，b: n*10）：
    // 这样位置实参的 typecheck 不会因签名差异先报错，值断言能直接证明 IR 的派发归属。
    let modules: [(&str, &str); 2] = [
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Int) -> Int = {\n    return n * 100\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Int) -> Int = {\n    return n * 10\n}\n",
        ),
    ];
    let alias_only = make_script_project(&[
        modules[0],
        (
            "main.yx",
            "use std.assert\nuse a.{Temp as X}\n\nx = X(1.0)\nr = x.bump(2)\nassert.assert(r == 200, \"alias-only dispatches to a.Temp.bump\")\n",
        ),
    ]);
    let bare_named = make_script_project(&[
        modules[0],
        modules[1],
        (
            "main.yx",
            "use a.{Temp as X}\nuse b.{Temp}\n\nt = Temp(2.0)\ns = t.bump(n = 2)\n",
        ),
    ]);
    let ambiguous_positional = make_script_project(&[
        modules[0],
        modules[1],
        (
            "main.yx",
            "use std.assert\nuse a.{Temp as X}\nuse b.{Temp}\n\nx = X(1.0)\nt = Temp(2.0)\nr = x.bump(2)\ns = t.bump(2)\nassert.assert(r == 200, \"alias receiver dispatches to a.Temp.bump\")\nassert.assert(s == 20, \"bare receiver dispatches to b.Temp.bump\")\n",
        ),
    ]);

    // Act
    let (named_codes, named_ok) = check_codes(&bare_named.1);
    let alias_outcome = run_multi(&alias_only.1).expect("driver run failed");
    let positional_outcome = run_multi(&ambiguous_positional.1).expect("driver run failed");

    // Assert
    assert!(named_ok, "IR 级诊断必须收进逐文件通道：{named_codes:?}");
    assert!(
        named_codes.iter().any(|code| code == "E1014"),
        "歧义形态 + 命名实参必须响亮拒绝（既有 E1014），不得静默错派：{named_codes:?}"
    );
    assert!(
        alias_outcome.failure.is_none(),
        "仅别名单一提供方必须 0/0（不得被歧义判据误伤）：{:?}",
        alias_outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let values = run_merged_slot_values(alias_outcome.module.expect("合并 IR"));
    assert!(
        values.iter().any(|value| value == "Int(200)"),
        "仅别名形态必须派发到 a.Temp.bump（2*100 = 200），实际全局槽位：{values:?}"
    );
    assert!(
        positional_outcome.failure.is_none(),
        "歧义形态 + 位置实参必须 0/0（4.9.8 派发归因）：{:?}",
        positional_outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let values = run_merged_slot_values(positional_outcome.module.expect("合并 IR"));
    assert!(
        values.iter().any(|value| value == "Int(200)"),
        "歧义形态位置实参：别名侧接收者必须派发到 a.Temp.bump（2*100 = 200），实际全局槽位：{values:?}"
    );
    assert!(
        values.iter().any(|value| value == "Int(20)"),
        "歧义形态位置实参：裸名侧接收者必须派发到 b.Temp.bump（2*10 = 20），实际全局槽位：{values:?}"
    );
}
/// 名字层歧义 + **位置实参**：两侧候选签名不一致时同样响亮拒绝（既有 E1107）——
/// 判据是各提供方对 `{Type}.{method}` 的候选签名集合是否完全一致
///（`ambiguous_method_signature_conflict`）。
///
/// 实测静默错值三例（见 4.9.10 的探针矩阵）：形参类型不同 ⇒ 常量体返回 111 而非
/// 2.0；元数不同 ⇒ 多出的实参被丢、返回 300 而非 7；返回类型不同 ⇒ 返回 200 而非
/// 2.0。签名一致时（4.9.8 夹具）位置实参仍 0/0 且派发正确（见不变量判据）。
#[test]
fn test_mixed_alias_bare_import_positional_signature_conflict_rejects_loudly() {
    // Arrange：a 的 Temp 别名导入为 X（bump 收 Int），b 的同名 Temp 裸名导入（bump 收 Float）
    let (_dir, entry) = make_script_project(&[
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Int) -> Int = {\n    return 111\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Float) -> Float = {\n    return n\n}\n",
        ),
        (
            "main.yx",
            "use a.{Temp as X}\nuse b.{Temp}\n\nx = X(1.0)\nr = x.bump(2.0)\n",
        ),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1107"),
        "歧义形态 + 位置实参 + 候选签名不一致必须响亮拒绝（既有 E1107），不得静默错值：{codes:?}"
    );
}

/// 同源判据的**放行**一侧（D59 D 类修复）：裸名侧接收者（owner == winner）的位置实参
/// 即使两个提供方的候选签名不同，也照常按接收者所属模块派发——typecheck 与 IR 用同
/// 一模块的候选，不存在错值。此前「全局签名比较」把这一侧也拒了（实测 0/0 打印 4.0
/// 被误拒为 E1107）。
#[test]
fn test_bare_side_positional_with_differing_signatures_dispatches_to_owner() {
    // Arrange：a 的 bump 收 Int（常量体 111），b 的 bump 收 Float（v + n）；裸名导入 b
    let (_dir, entry) = make_script_project(&[
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Int) -> Int = {\n    return 111\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Float) -> Float = {\n    return self.v + n\n}\n",
        ),
        (
            "main.yx",
            "use std.assert\nuse a.{Temp as X}\nuse b.{Temp}\n\nx = X(1.0)\nt = Temp(2.0)\nr = t.bump(2.0)\nassert.assert(r == 4.0, \"bare side dispatches to b.Temp.bump\")\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "裸名侧（owner == winner）的位置实参必须放行（differing signatures 也不拒）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(4.0)"),
        "裸名侧必须派发到 b.Temp.bump（2.0 + 2.0 = 4.0），实际全局槽位：{values:?}"
    );
}

/// 同源判据的**拒绝**一侧：别名侧接收者（owner != winner）且两模块候选**元数**不同 ⇒
/// 响亮拒绝（既有 E1107）。实测旧行为：变异前（全局签名比较之前）check 通过、多出的
/// 实参被丢，返回 300 而非 7。
#[test]
fn test_alias_side_positional_arity_mismatch_rejects_loudly() {
    // Arrange：a 的 bump 单参（n * 100），b 的 bump 双参（p + q）；别名侧调用两参
    let (_dir, entry) = make_script_project(&[
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Int) -> Int = {\n    return n * 100\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, p: Int, q: Int) -> Int = {\n    return p + q\n}\n",
        ),
        (
            "main.yx",
            "use a.{Temp as X}\nuse b.{Temp}\n\nx = X(1.0)\nr = x.bump(3, 4)\n",
        ),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1107"),
        "别名侧元数不一致的候选必须响亮拒绝（既有 E1107），不得丢实参静默错值：{codes:?}"
    );
}

/// 同源判据的**拒绝**一侧：别名侧接收者 + 两模块候选**返回类型**不同 ⇒ 响亮拒绝
///（既有 E1107）。实测旧行为：check 通过、返回 200（a 的 Int 实现）而非 b 的 2.0。
#[test]
fn test_alias_side_positional_return_type_mismatch_rejects_loudly() {
    // Arrange：a 的 bump 返回 Int（n*100），b 的 bump 同形参返回 Float（2.0）
    let (_dir, entry) = make_script_project(&[
        (
            "a.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Int) -> Int = {\n    return n * 100\n}\n",
        ),
        (
            "b.yx",
            "Temp: Type = { v: Float }\n\nTemp.bump: (self: &Temp, n: Int) -> Float = {\n    return 2.0\n}\n",
        ),
        (
            "main.yx",
            "use a.{Temp as X}\nuse b.{Temp}\n\nx = X(1.0)\nr = x.bump(2)\n",
        ),
    ]);

    // Act
    let (codes, no_hard_failure) = check_codes(&entry);

    // Assert
    assert!(no_hard_failure, "IR 级诊断必须收进逐文件通道：{codes:?}");
    assert!(
        codes.iter().any(|code| code == "E1107"),
        "别名侧返回类型不一致的候选必须响亮拒绝（既有 E1107），不得静默返回别家实现的值：{codes:?}"
    );
}
