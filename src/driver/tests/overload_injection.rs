//! 跨单元方法候选注入面的直驱钉测试（WBS 4.9.7 / issue #464，修复轮 F1 + F3）
//!
//! 判据来源：RFC-029f §跨单元方法可达性归因 + t21 复审 F1（blocker）/ F3（low）。
//!
//! - **F1（名字归属）**：注入面必须按「名字 ← 提供模块」**逐对**取候选——只并入
//!   「键的提供模块 == 该类型名前缀在该单元 scope 里的提供模块」的候选组。跨对组合
//!   （名字集合 × 模块集合）会把同编译集里另一模块定义的同名类型候选并进同一键位
//!   ⇒ 调用点 2 候选 ⇒ 按「多候选无法区分即报错」规则误报 E1107——而源程序里每个
//!   名字只由一个模块带进作用域、接收者类型唯一，修复前是能编译能跑的。
//!   本文件把这一形态钉在驱动级：`use a.{Temp}` + `use b.{Other}`，a 亦定义
//!   Other、b 亦定义 Temp、方法同名同序 ⇒ check 收集面**不得**出现 E1107。
//! - **F3（三表等长）**：声明期登记与跨单元注入共用同一落表点，候选 / 混编 IR 名 /
//!   声明期形参名三张平行表逐位对齐（错位 = 调用点拿别人的名字派发）。

use std::path::{Path, PathBuf};

use crate::driver::{Driver, DriverError, DriverOutcome, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::types::MonoType;

/// 合法 manifest（[package] 可解析 ⇒ 角色上下文可用）；入口 main.yx
const MANIFEST: &str = "[package]\nname = \"t\"\nversion = \"0.1.0\"\n";

/// 落一个多文件项目夹具（带 manifest），入口 main.yx
fn write_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(dir.path().join("yaoxiang.toml"), MANIFEST).expect("write manifest");
    let mut entry = PathBuf::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write project file");
        if *name == "main.yx" {
            entry = path;
        }
    }
    assert!(entry.is_file(), "夹具必须含入口 main.yx");
    (dir, entry)
}

/// 以 Check 形态跑 Driver（与 `yaoxiang check` 同一入口形态）
fn run_check(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::Check,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

/// 全部检查面里的诊断码（按文件顺序展平）
fn all_codes(outcome: &DriverOutcome) -> Vec<String> {
    outcome
        .check_diagnostics
        .iter()
        .flat_map(|(_, ds)| ds.iter().map(|d| d.code.clone()))
        .collect()
}

/// F1（WBS 4.9.7 修复轮）：名字归属逐对过滤——每个名字只由一个模块带进作用域时，
/// 另一模块定义的同名类型候选**不得**并进同一键位。
#[test]
fn test_cross_unit_injection_filters_candidates_by_name_owner() {
    // Arrange：a 提供 Temp（并被它带进作用域）且另定义 Other；b 提供 Other 且另定义 Temp；
    // 两边同名同序方法都在，接收者类型在 main 里唯一（Temp ← a、Other ← b）
    let a = "Temp: Type = { v: Float }\nOther: Type = { v: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    self.v + 1.0\n}\n\nOther.to: (self: &Other) -> Float = (self) => {\n    self.v + 2.0\n}\n";
    let b = "Temp: Type = { v: Float }\nOther: Type = { v: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    self.v + 100.0\n}\n\nOther.to: (self: &Other) -> Float = (self) => {\n    self.v + 10.0\n}\n";
    let main = "use a.{Temp}\nuse b.{Other}\n\nmain = () => {\n    t = Temp(1.0)\n    o = Other(2.0)\n    x: Float = t.to()\n    y: Float = o.to()\n    assert.assert(x == 2.0, \"a.Temp.to\")\n    assert.assert(y == 12.0, \"b.Other.to\")\n}\n";
    let (_dir, entry) = write_project(&[("a.yx", a), ("b.yx", b), ("main.yx", main)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：修复前该形态 2×E1107（Temp.to / Other.to 各 2 候选）；修复后一条都不能有
    let codes = all_codes(&outcome);
    assert!(
        !codes.iter().any(|c| c == "E1107"),
        "名字归属逐对过滤必须消除同键候选误并（E1107 歧义误报），实际: {codes:?}"
    );
    assert!(
        outcome.failure.is_none(),
        "该形态修复前可编译运行，修复后不得变成硬失败: {:?}",
        outcome.failure
    );
}

/// F1 负对照：**两个模块都定义同名类型、但方法名不同** ⇒ 键位不重合，多候选误并不会发生。
#[test]
fn test_cross_unit_injection_distinct_method_names_stay_green() {
    // Arrange：a 的方法名 to、b 的方法名 tob（同名类型 Temp/Other 两边都有）
    let a = "Temp: Type = { v: Float }\nOther: Type = { v: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    self.v + 1.0\n}\n\nOther.to: (self: &Other) -> Float = (self) => {\n    self.v + 2.0\n}\n";
    let b = "Temp: Type = { v: Float }\nOther: Type = { v: Float }\n\nTemp.tob: (self: &Temp) -> Float = (self) => {\n    self.v + 100.0\n}\n\nOther.tob: (self: &Other) -> Float = (self) => {\n    self.v + 10.0\n}\n";
    let main = "use a.{Temp}\nuse b.{Other}\n\nmain = () => {\n    t = Temp(1.0)\n    o = Other(2.0)\n    x: Float = t.to()\n    y: Float = o.tob()\n    assert.assert(x == 2.0, \"a.Temp.to\")\n    assert.assert(y == 12.0, \"b.Other.tob\")\n}\n";
    let (_dir, entry) = write_project(&[("a.yx", a), ("b.yx", b), ("main.yx", main)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert
    let codes = all_codes(&outcome);
    assert!(
        !codes.iter().any(|c| c == "E1107"),
        "方法名不同 ⇒ 键位不重合，不得出现歧义误报，实际: {codes:?}"
    );
}

/// F3（WBS 4.9.7 修复轮）：候选三元组（候选 / 混编 IR 名 / 声明期形参名）的**同序落表**
/// 与**等长不变式**——声明期登记与跨单元注入共用同一落表点，快照出口校验等长；
/// 错位不得静默流到别的编译单元（错位 = 调用点拿别人的名字派发）。
#[test]
fn test_method_overload_triple_stays_aligned_across_registration_and_injection() {
    // Arrange：声明期登记两个同名候选（签名不同 ⇒ 各自占一个声明序）
    let mut source = TypeEnvironment::new();
    let first = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Float(64)),
    };
    let second = MonoType::Fn {
        params: vec![MonoType::make_string()],
        return_type: Box::new(MonoType::Float(64)),
    };
    source.add_method_overload_with_params("Temp.to", first.clone(), vec!["x".to_string()]);
    source.add_method_overload_with_params("Temp.to", second.clone(), vec!["s".to_string()]);
    // 同签名重复登记不占新声明序（既有去重口径不得因改写而变）
    source.add_method_overload_with_params("Temp.to", first.clone(), vec!["x".to_string()]);

    // Act：快照（跨单元通路的出口）→ 注入另一个环境（通路的入口）
    let snapshot = source.method_overload_snapshot();
    let mut target = TypeEnvironment::new();
    target.inject_method_overloads(snapshot.clone());

    // Assert：逐位对齐——候选序 = 声明序、混编名 0=裸名其后 #序、形参名同位
    let (key, group) = &snapshot[0];
    assert_eq!(key, "Temp.to", "快照键 = 登记键，实际: {key}");
    assert_eq!(
        group.candidates,
        vec![first, second],
        "候选必须按声明序排列（重复登记不得占新序），实际: {:?}",
        group.candidates
    );
    assert_eq!(
        group.ir_names,
        vec!["Temp.to".to_string(), "Temp.to#1".to_string()],
        "混编名必须与候选同序（0=裸名、其后 #序），实际: {:?}",
        group.ir_names
    );
    assert_eq!(
        group.param_names,
        vec![vec!["x".to_string()], vec!["s".to_string()]],
        "声明期形参名必须与候选同序，实际: {:?}",
        group.param_names
    );
    // 注入端与快照端逐表一致（跨单元通路的往返不得改序、不得丢位）
    let injected = target.method_overload_snapshot();
    assert_eq!(
        injected.len(),
        1,
        "注入后目标环境应恰有 1 个键位，实际: {:?}",
        injected.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>()
    );
    assert_eq!(
        injected[0].1,
        group.clone(),
        "注入不得改变三元组的顺序与配对"
    );
}
