//! rootless 多文件 check 的单态化命名同源钉测试 — 基于 02-stage-contract §3 注记 #14 第③点
//!
//! 形态：**无项目根**（standalone / Script 语义）的多文件 check。3.4.8 ③ 回归
//! 修复后，`ir_generation_check` 逐单元传**单元键**限定名 + `with_script_mode(true)`
//! 保持 Script 语义（限定名开关 ⊥ Script/Bin 开关，3.4.8 ③ 回归修复）。于是三者
//! 同源：IR 定义键、IR 调用名、聚合请求名都是 `{单元键}.{名字}`——mono 真解析
//!（良构静默、病态泛型递归编译期报 E3005），合并也不会把跨单元同名函数判成
//!「模块限定名冲突」。
//!
//! 判据来源：WBS 3.4.8 ③（02 §3 注记 #14 第③点）；t3 独立验证登记的 F1 缺口；
//! t11 回归（rootless 未限定名 → 跨单元同名合并报冲突，文档示例门禁 10 项红）。

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
/// 顶层语句即程序体）。3.4.8 ③ 回归修复让 rootless **走限定名**（units 需按单元键
/// 限定才能合并），但 Script/Bin 语义由 `with_script_mode` 独立承载——不得连带
/// 打开 E3023。对照：同一对源文件加 manifest（Bin 语义）时 E3023 必须出现，
/// 证明本断言不是空转（E3023 开关确实由 Script/Bin 标志控制）。
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
/// 跨单元同名（用户模块 ↔ 嵌入 std 模块）：用户文件里的 `map` 与 `<std/list>` 的
/// `map` 同名——各单元 IR 定义键按单元键限定后合并无冲突（t11 回归的 4/10 形状）。
/// 判别力：rootless 不限定名时 `link_module_irs` 报「模块限定名冲突 `map`
///（exNN.yx 与 <std/list>）」，`outcome.failure` 变 Some → 第一条断言红。
#[test]
fn test_driver_run_check_rootless_cross_unit_name_clashes_with_std_stays_clean() {
    // Arrange：入口定义 `map`（与 std.list 的 map 同名）并调用 std.list 的 len
    let (_dir, entry) = make_rootless_project(&[(
        "main.yx",
        "use std.list\nmap: (xs: Vec(Int)) -> Int = (xs) => list.len(xs)\nv = map([1, 2])\n",
    )]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert：不得有冲突故障，也不得引入错误诊断
    assert!(
        outcome.failure.is_none(),
        "用户 `map` 与 <std/list>.map 不得被判成模块限定名冲突: {:?}",
        outcome.failure
    );
    let codes = check_codes(&outcome, "main.yx");
    assert!(
        !codes.iter().any(|c| c.starts_with('E')),
        "同名不得引入错误诊断: {codes:?}"
    );
}

/// 跨单元同名（用户模块 ↔ 用户模块）：两个文件各定义 `add`——限定名后共存，
/// 各自调用落在各自定义上（与 project 形态一致）。判别力：不限定名时合并报
///「模块限定名冲突 `add`（lib.yx 与 util.yx）」，第一条断言红。
#[test]
fn test_driver_run_check_rootless_cross_unit_name_clashes_across_user_modules_stays_clean() {
    // Arrange：lib 与 util 各定义同名 `add`（不同签名，各自被调用）
    let (_dir, entry) = make_rootless_project(&[
        ("lib.yx", "add: (a: Int, b: Int) -> Int = (a, b) => a + b\n"),
        ("util.yx", "add: (s: String) -> String = (s) => s\n"),
        (
            "main.yx",
            "use lib\nuse util\nv = lib.add(1, 2)\nw = util.add(\"x\")\n",
        ),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "两个用户模块的同名函数不得被判成模块限定名冲突: {:?}",
        outcome.failure
    );
    let mut codes = check_codes(&outcome, "main.yx");
    codes.extend(check_codes(&outcome, "lib.yx"));
    codes.extend(check_codes(&outcome, "util.yx"));
    assert!(
        !codes.iter().any(|c| c.starts_with('E')),
        "同名不得引入错误诊断: {codes:?}"
    );
}

/// 同一文件内的顶层同名重绑定（Script 语义：后者生效）：IR 只留一份定义，合并不
/// 再把「同一模块内的重绑定」误报成「模块限定名冲突（同一路径 ×2）」。
/// 判别力：t11 红态实测（文档示例 ex11 形状）→ Collision + check 退出码 1，
/// 第一条断言红；若重绑定语义反了，`driver.rs` 的取值钉板（后者 = 2）会红。
#[test]
fn test_driver_run_check_rootless_redeclared_function_stays_clean() {
    // Arrange：同一文件里 `add` 声明两次（docs「完整形式 / 简短形式」示例形状）
    let (_dir, entry) = make_rootless_project(&[(
        "main.yx",
        "add: (a: Int, b: Int) -> Int = a + b\nadd = (a, b) => a + b\n",
    )]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "同一模块内的顶层重绑定不得被判成模块限定名冲突: {:?}",
        outcome.failure
    );
    let codes = check_codes(&outcome, "main.yx");
    assert!(
        !codes.iter().any(|c| c.starts_with('E')),
        "重绑定不得引入错误诊断: {codes:?}"
    );
}
