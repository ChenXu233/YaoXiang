//! rootless（standalone / Script 语义）多文件 **run** 形态的编排钉测试 — 基于
//! WBS 4.10.3（4.2.5 裁决 A 的 run 半边）与 #413 T4 入口语义。
//!
//! 形态：无项目根（无 yaoxiang.toml）的入口文件经 Program{MultiFile} 编排运行。
//! 与 check 半边（check_rootless_mono.rs）同一家族的 run 侧判据：
//! - Script 无入口概念——Linking 不得给 rootless 合并 IR 设 entry_function
//!   （#413 T4：顶层语句即程序主体，自动调 main 会与显式 main() 双跑，#356）；
//! - Script 语义保持——顶层可执行语句不报 E3023（ir_gen 的 script_mode 由
//!   rootless 驱动，与限定名键解耦，同 3.4.8 ③ 的 check 半边）；
//! - Bin 形态对照——带 manifest 的项目入口语义不变（entry_function 照旧设置），
//!   证明 Script 侧断言不是空转。
//!
//! #467（WBS 4.10.4）：多单元初始化序列按 `use` 依赖拓扑序拼接——被依赖单元
//! 的初始化先于导入它的单元；无依赖边的单元保持发现顺序（发现期按路径字典序
//! 排序）；模块导入环内按路径字典序（确定性约定，不是语义保证）。拼接序在
//! Linking 臂（两形态共用）里生效，故 Script 与 Bin 同源——Bin 侧只额外保留
//! `entry_function`（入口调用仍由运行期排在全部 init 之后）。
//!
//! 判据来源：WBS 4.10.3 / 4.10.4；#413（T4 决议）；#356（防双跑）；#467
//! （初始化顺序）；RFC-029f 角色模型。

use std::path::Path;

use super::check_rootless_mono::{make_bin_project, make_rootless_project};
use crate::driver::{Driver, DriverError, DriverOutcome, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;
use crate::middle::ModuleIR;

/// 以 MultiFile 形态跑 Driver（与 `yaoxiang run` 同一入口形态，即 compile_project）。
fn run_multifile(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::MultiFile,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

#[test]
fn test_run_rootless_merged_ir_has_no_entry_function() {
    // Arrange - 无 manifest：main.yx 定义函数形态 main
    let (_dir, entry) = make_rootless_project(&[("main.yx", "main = () => { x = 1 }\n")]);

    // Act
    let outcome = run_multifile(&entry).expect("rootless MultiFile 编排应成功");

    // Assert - #413 T4：Script 无入口概念，merged IR 不得携带 entry_function
    let module = outcome.module.expect("成功编排必须产 merged IR");
    assert!(
        module.entry_function.is_none(),
        "Script（rootless）不得自动调 main：entry_function 应为 None，实际 {:?}",
        module.entry_function
    );
    assert!(
        module.functions.iter().any(|f| f.name == "main.main"),
        "函数本体仍应限定名存在于 IR 中（不被删除）：{:?}",
        module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_run_bin_project_merged_ir_keeps_entry_function() {
    // Arrange - 带 manifest 的 Bin 项目：入口语义不变（对照组）
    let (_dir, entry) = make_bin_project(&[("main.yx", "main = () => { x = 1 }\n")]);

    // Act
    let outcome = run_multifile(&entry).expect("Bin MultiFile 编排应成功");

    // Assert - Bin 照旧自动入口：entry_function 指向限定名 main
    let module = outcome.module.expect("成功编排必须产 merged IR");
    assert_eq!(
        module.entry_function.as_deref(),
        Some("main.main"),
        "Bin 项目入口应保持 {{entry_key}}.main（对照证明 Script 侧不是空转）"
    );
}

#[test]
fn test_run_rootless_top_level_statement_compiles() {
    // Arrange - 无 manifest：顶层可执行语句（Script 语义合法；Bin 语义是 E3023）
    let (_dir, entry) = make_rootless_project(&[("main.yx", "print(\"script-top-level\")\n")]);

    // Act
    let outcome = run_multifile(&entry);

    // Assert - Script 语义：顶层语句即程序主体，编排必须成功产 IR
    let outcome = outcome.unwrap_or_else(|e| {
        panic!("rootless 顶层可执行语句应正常编排（Script 语义），实际故障: {e:?}")
    });
    assert!(
        outcome.module.is_some(),
        "Script 顶层语句必须产出 merged IR（不得报 E3023）"
    );
}

/// merged IR 的初始化序列**实际执行的文件顺序**：按 `init_file_ids` 首次出现的
/// 下标序去重。`init_file_ids` 与 `init` 等长、每条指令记其所属 `source_files`
/// 下标，同一文件的条目是连续一段——#368 的 file_id 表在这里兼作顺序判据。
///
/// 没有初始化指令的单元不出现（它没有可排的段）：夹具必须给每个单元留一条顶层
/// **值**绑定，否则顺序判据落在空集上、断言退化成空转。
fn init_file_order(module: &ModuleIR) -> Vec<String> {
    let mut first_seen: Vec<usize> = Vec::new();
    for &file_idx in &module.init_file_ids {
        if !first_seen.contains(&file_idx) {
            first_seen.push(file_idx);
        }
    }
    first_seen
        .into_iter()
        .map(|file_idx| {
            let path = module.source_files.get(file_idx).unwrap_or_else(|| {
                panic!(
                    "init_file_ids 含越界下标 {file_idx}（source_files 长度 {}）",
                    module.source_files.len()
                )
            });
            Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone())
        })
        .collect()
}

/// #467 形态1（issue 最小复现）：入口顶层读依赖单元顶层值。
///
/// 依赖单元 `zlib.yx` 的路径字典序**晚于**入口 `main.yx`——修复前初始化
/// 序列按路径序拼接，入口先跑、读到 `void`（exit 0，无诊断）。本钉测试断言
/// 依赖单元先初始化。
#[test]
fn test_run_rootless_init_order_follows_use_topology() {
    // Arrange - 依赖单元的路径排序在入口之后（修复前 red 的决定性条件）
    let (_dir, entry) = make_rootless_project(&[
        ("zlib.yx", "base: Int = 41\n"),
        ("main.yx", "use zlib\nb: Int = zlib.base\n"),
    ]);

    // Act
    let outcome = run_multifile(&entry).expect("rootless MultiFile 编排应成功");
    let module = outcome.module.expect("成功编排必须产 merged IR");

    // Assert - #467：被 `use` 的单元初始化必须早于导入它的入口单元
    assert_eq!(
        init_file_order(&module),
        ["zlib.yx", "main.yx"],
        "rootless 多文件初始化必须按 use 依赖拓扑序：zlib.yx（被依赖）先于 main.yx（入口）"
    );
}

/// #467 Bin 形态对照：同一套拼接序对带 manifest 的项目同样成立（依赖单元仍先
/// 初始化），且入口语义不变——`entry_function` 照旧指向 `{entry_key}.main`，
/// 运行期由它决定入口在全部 init 之后调用（脚本 VM 先跑 `__yx_module_init`
/// 再跳 entry_point）。证明修复不是只对 rootless 生效的分叉。
#[test]
fn test_run_bin_project_init_order_follows_use_topology() {
    // Arrange - 带 manifest 的 Bin 项目；被依赖单元 zlib.yx 路径排在入口之后
    let (_dir, entry) = make_bin_project(&[
        ("zlib.yx", "base: Int = 41\n"),
        (
            "main.yx",
            // 入口单元的顶层**值**绑定：它的初始化落在入口的 init 段（函数绑定
            // 不进 init，入口就没有可排的段——顺序判据只能落在值绑定上）
            "use std.assert\nuse zlib.{base}\ncopied: Int = base\nmain = () => { assert.assert(copied == 41, \"copied\") }\n",
        ),
    ]);

    // Act
    let outcome = run_multifile(&entry).expect("Bin MultiFile 编排应成功");
    let module = outcome.module.expect("成功编排必须产 merged IR");

    // Assert - Bin 形态同样按 use 拓扑序初始化；入口函数照旧存在（角色门控未受
    // 初始化序调整影响）
    assert_eq!(
        init_file_order(&module),
        ["zlib.yx", "main.yx"],
        "Bin 项目的初始化序也必须按 use 依赖拓扑序：zlib.yx 先于 main.yx（#467）"
    );
    assert_eq!(
        module.entry_function.as_deref(),
        Some("main.main"),
        "初始化序调整不得动入口语义：Bin 的 entry_function 仍是 {{entry_key}}.main"
    );
}

/// #467 有环（zlib ↔ zother）：环内先后语义上不可判定，按路径字典序做**确定性
/// 约定**（不是语义保证——本批不新增诊断码，环升级为编译期错误属独立待裁项）。
///
/// 夹具刻意让**路径序与拓扑序不同**：入口 main.yx 的路径字典序排在环的两个
/// 成员之前，修复前（按发现序 = 路径序拼接）会先初始化入口、再初始化环内单元；
/// 修复后 SCC 整体排在入口之前。两态可分辨：把 Linking 臂的 `init_order`
/// 改回恒等（退回发现序）的变异试验会使本用例变红。
#[test]
fn test_run_rootless_init_order_cycle_is_deterministic_path_order() {
    // Arrange - 模块导入环 zlib ↔ zother；入口 main 只依赖 zlib
    let (_dir, entry) = make_rootless_project(&[
        // 路径序：main.yx < zlib.yx < zother.yx（入口排在环成员之前）
        ("zlib.yx", "use zother\nz_val: Int = 2\n"),
        ("zother.yx", "use zlib\no_val: Int = 1\n"),
        ("main.yx", "use zlib\nm_val: Int = zlib.z_val\n"),
    ]);

    // Act
    let outcome = run_multifile(&entry).expect("rootless MultiFile 编排应成功（环当前容忍）");
    let module = outcome.module.expect("成功编排必须产 merged IR");

    // Assert - SCC 缩点后入口最后；环内按路径字典序（zlib.yx < zother.yx）
    assert_eq!(
        init_file_order(&module),
        ["zlib.yx", "zother.yx", "main.yx"],
        "环 zlib↔zother 内按路径字典序（确定性约定），入口 main 排在被依赖 SCC 之后"
    );
}

/// #467 无依赖边：互不依赖的单元保持发现顺序（发现期按路径字典序排序）——
/// 稳定 tie-break，不静默乱序。
#[test]
fn test_run_rootless_init_order_independent_units_keep_path_order() {
    // Arrange - alib / zlib 互不依赖，入口都 use（两者都在入口之前初始化）
    let (_dir, entry) = make_rootless_project(&[
        ("zlib.yx", "z: Int = 3\n"),
        ("alib.yx", "a: Int = 1\n"),
        ("main.yx", "use zlib\nuse alib\nm: Int = alib.a\n"),
    ]);

    // Act
    let outcome = run_multifile(&entry).expect("rootless MultiFile 编排应成功");
    let module = outcome.module.expect("成功编排必须产 merged IR");

    // Assert - 无依赖关系的单元按发现顺序（=路径字典序）稳定排序，入口最后
    assert_eq!(
        init_file_order(&module),
        ["alib.yx", "zlib.yx", "main.yx"],
        "无依赖边的单元必须保持发现顺序（路径字典序）：alib.yx 先于 zlib.yx，入口最后"
    );
}
