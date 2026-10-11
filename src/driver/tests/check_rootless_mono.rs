//! rootless 多文件 check 的单态化命名同源钉测试 — 基于 02-stage-contract §3 注记 #14 第③点
//!
//! 形态：**无项目根**（standalone / Script 语义）的多文件 check。该形态下
//! `ir_generation_check` 对每个单元传 `module_key=None`（src/driver/arms.rs），
//! 于是 ir_gen 跳过 `qualify_names`——单元 IR 的**定义键是扁平名**（`f`）；
//! 而 `Resolver` 产出的调用名与 typecheck 产出的实例化请求名都是**限定名**
//! （`lib.f`）。聚合（`aggregate_instantiation_requests`）必须把请求名锚定到
//! 定义表，mono 才会真解析：良构程序静默，病态泛型递归在 check 编译期报 E3005。
//!
//! 判据来源：WBS 3.4.8 ③（02 §3 注记 #14 第③点）；t3 独立验证登记的 F1 缺口。

use std::path::{Path, PathBuf};

use crate::driver::{Driver, DriverError, DriverOutcome, Program, ProgramKind, Stage, Unit};
use crate::frontend::config::CompileConfig;

/// 落一个**无 manifest** 的多文件夹具（rootless / Script 语义）：入口约定
/// `main.yx`。无 manifest 是 4.2.5 裁决 A 的 standalone 语义开关——
/// `ir_generation_check` 据此传 `module_key=None`。
fn make_rootless_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let mut entry = PathBuf::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write project file");
        if *name == "main.yx" {
            entry = path;
        }
    }
    assert!(entry.is_file(), "夹具必须含 main.yx（入口约定）");
    (dir, entry)
}

/// 同夹具但**带 manifest**（Bin 语义：`module_key=Some`，E3023 开关打开）——
/// 用来证明「Script 语义未被打开 E3023」的断言不是空转。
fn make_bin_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\n",
    )
    .expect("write manifest failed");
    let mut entry = PathBuf::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write project file");
        if *name == "main.yx" {
            entry = path;
        }
    }
    assert!(entry.is_file(), "夹具必须含 main.yx（入口约定）");
    (dir, entry)
}

/// 以 Check 形态跑 Driver（与 `yaoxiang check` 同一入口形态）。
fn run_check(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::Check,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

/// 某文件条目里的诊断码列表（按路径后缀匹配；无条目 = panic，不静默返回空表）。
fn check_codes(
    outcome: &DriverOutcome,
    suffix: &str,
) -> Vec<String> {
    outcome
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.ends_with(suffix))
        .map(|(_, ds)| ds.iter().map(|d| d.code.clone()).collect())
        .unwrap_or_else(|| {
            panic!(
                "check 收集通道必须有 {suffix} 的条目: {:?}",
                outcome
                    .check_diagnostics
                    .iter()
                    .map(|(p, _)| p.display().to_string())
                    .collect::<Vec<_>>()
            )
        })
}

/// F1 红判据（修复前必须为红）：rootless 多文件下的病态泛型递归
/// `f(x) = f([x])` 必须在 **check 编译期**报 E3005——聚合请求名（限定名
/// `lib.f`）与单元 IR 定义键（扁平名 `f`）不对齐时，mono 按「native 泛型
/// 无 IR 定义」静默 skip，check 报 0 error 后以类型擦除执行到进程爆栈。
#[test]
fn test_driver_run_check_rootless_multi_file_pathological_recursion_reports_e3005() {
    // Arrange：lib 定义类型逐层增长的自递归泛型，入口跨单元限定调用它
    let (_dir, entry) = make_rootless_project(&[
        (
            "lib.yx",
            "f: (T: Type) -> (x: T) -> T = (x) => {\n    return f([x])\n}\n",
        ),
        ("main.yx", "use lib\nv = lib.f(1)\n"),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert：mono 真解析（E3005 进收集通道——不是硬中止，也不是空转）
    assert!(
        outcome.failure.is_none(),
        "rootless check 的 mono 失败是收集诊断，不得走硬中止: {:?}",
        outcome.failure
    );
    let codes = check_codes(&outcome, "main.yx");
    assert_eq!(
        codes.iter().filter(|c| c.as_str() == "E3005").count(),
        1,
        "rootless 多文件病态泛型递归必须报恰好一条 E3005（请求名须锚定扁平定义键）: {codes:?}"
    );
    assert!(
        !outcome
            .skipped
            .iter()
            .any(|s| s.stage == Stage::Monomorphization),
        "有实例化请求时 mono 不得记跳过（空转即假绿）: {:?}",
        outcome.skipped
    );
}

/// 良构 rootless 多文件泛型程序：check 必须通过，且诊断集与 F1 修复前**逐项一致**
/// （修复只让 mono 真解析，不给原先成功的程序添诊断）。对照方法：断言直接钉住
/// 修复前实测的码列表——修复后若 mono 误报（E3005/E3018）或新增警告，本条红。
#[test]
fn test_driver_run_check_rootless_multi_file_well_formed_generic_stays_clean() {
    // Arrange：跨单元泛型调用（类型参数只出现在借用内层 &T）
    let (_dir, entry) = make_rootless_project(&[
        ("lib.yx", "pick: (T: Type) -> (x: &T) -> T = (x) => x\n"),
        ("main.yx", "use lib\nv = lib.pick(7)\n"),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "良构 rootless 多文件程序不得硬中止: {:?}",
        outcome.failure
    );
    assert_eq!(
        check_codes(&outcome, "main.yx"),
        vec!["W1004"],
        "入口诊断集必须与修复前一致（只应保留未使用变量警告）"
    );
    assert_eq!(
        check_codes(&outcome, "lib.yx"),
        Vec::<String>::new(),
        "lib 单元诊断集必须与修复前一致（干净）"
    );
}

/// Script 语义守卫：rootless 多文件的顶层可执行语句必须合法（standalone =
/// 顶层语句即程序体）——修复 F1 **不得**以「让 rootless 也走限定名」的方式
/// 连带打开 E3023。对照：同一对源文件加 manifest（Bin 语义）时 E3023 必须出现，
/// 证明本断言不是空转（E3023 开关确实由 module_key 控制）。
#[test]
fn test_driver_run_check_rootless_multi_file_keeps_script_semantics() {
    // Arrange：入口有顶层调用语句（Script 程序体），lib 提供被调函数
    let files: &[(&str, &str)] = &[
        ("lib.yx", "helper: () -> Void = {}\n"),
        ("main.yx", "use lib\nlib.helper()\n"),
    ];
    let (_rootless_dir, rootless_entry) = make_rootless_project(files);
    let (_bin_dir, bin_entry) = make_bin_project(files);

    // Act
    let rootless = run_check(&rootless_entry).expect("driver run failed");
    let bin = run_check(&bin_entry).expect("driver run failed");

    // Assert：Script 形不得有 E3023；Bin 形必须报 E3023（开关对照）
    let mut rootless_codes = check_codes(&rootless, "main.yx");
    rootless_codes.extend(check_codes(&rootless, "lib.yx"));
    assert!(
        !rootless_codes.iter().any(|c| c == "E3023"),
        "rootless 多文件 check 必须保持 Script 语义（顶层语句合法）: {rootless_codes:?}"
    );
    let bin_codes = check_codes(&bin, "main.yx");
    assert!(
        bin_codes.iter().any(|c| c == "E3023"),
        "对照组（有 manifest 的 Bin 语义）必须报 E3023——否则本守卫空转: {bin_codes:?}"
    );
}
