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
/// F1 回归（WBS 4.9.9 修复轮 2）：**别名 + 裸名导入同名类型**——候选名字表按
/// 「目标名」归并后被收窄规则③整名跳过，调用方拿不到名字表。此时 IR 侧不得
/// 静默丢弃命名实参，必须响亮拒绝（既有 E1014）。
///
/// 先红后绿的定义（**红 = 静默丢参**，不是「编译失败」）：
/// - 修复前（第一次修复之后）：check exit=0、run exit=1，运行期
///   `E6007 type mismatch in binary operation Add`（b.yx 的 `self.v + step_b`
///   收不到 step_b）——命名实参被丢弃，编译期不报；
/// - 修复后：check exit=1，`error [E1014] Function bump has no parameter named
///   step_b (available: )`。
///
/// 对照（同批）：位置实参形态（p13x）与「别名导入的是另一类型」（p13d）修复
/// 前后都照常 0/0——本判据只改「拿不到名字表 + 有命名实参」这一格。
#[test]
fn test_cross_unit_named_arg_mixed_alias_bare_import_rejects_loudly() {
    // Arrange：两模块各定义同名类型 Temp（方法形参名不同），调用方别名导入 a 的、
    // 裸名导入 b 的——裸名 Temp 上的命名实参调用应响亮拒绝
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
    assert!(
        no_hard_failure,
        "IR 级诊断必须收进逐文件通道，不得硬中止（其余文件诊断不得丢）：{codes:?}"
    );
    assert!(
        codes.iter().any(|code| code == "E1014"),
        "拿不到形参名表时命名实参必须响亮拒绝（既有 E1014），不得静默丢弃；实际诊断：{codes:?}"
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
