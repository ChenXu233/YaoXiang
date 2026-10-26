//! 钉测试：**跨单元互递归**的实例化链在多文件（merged IR）形态下受同一资源保护。
//!
//! 规范来源：
//! - 02-stage-contract §3 注记 #14 第 ④ 点——「资源保护随全程序 BFS 覆盖跨单元
//!   互递归」；注记 #14 同时记录：病态泛型递归 `f(x) = f([x])` 在无 mono 的
//!   多文件路径下让编译器进程爆栈（0xC00000FD），mono 深度闸是唯一编译期防线
//!   （「类型擦除兜底」经 RFC-033 反射裁决作废——`^^List(Int)` 需要实例化的真实身份）；
//! - 09-execution-wbs 3.4.8 行判据（C2/C3）：跨文件泛型矩阵与单文件等价；
//!   病态递归多文件 → 编译期 E3005（不再爆栈）。
//!
//! 与既有钉测试的分工：driver.rs 的 3.4.8 ② 组只钉**单单元自递归**
//!（`lib.f` 自调、实参类型逐层增长）。本文件的被测对象是**互递归**——
//! A 单元的函数以增长类型调 B 单元的函数、B 再反过来以增长类型调 A：深度链
//! 必须**跨单元**延续（deferred 桶键 = 对方单元的限定名）。若链在第一层断掉，
//! 原型会被 `restore_generics_with_uncovered_call_sites` 放回、以类型擦除执行到
//! 进程栈溢出（0xC00000FD）——本文件即该形态的活体判据。
//!
//! 两个方向都钉：保护不足（爆栈 / 静默擦除）与保护过度（良构跨单元泛型互调
//! 被误拒）。

use super::driver::{make_script_project, run_multi};
use crate::driver::{Driver, DriverOutcome, Program, ProgramKind, Unit};
use crate::frontend::config::{CompileConfig, MonoConfig};
use crate::frontend::module::orchestrator::OrchestratorError;
use std::path::Path;

// ==================== 夹具 ====================

/// 病态互递归两单元：`a.f` 以 `Vec(T)` 调 `b.g`，`b.g` 再以 `Vec(T)` 调 `a.f`。
///
/// 类型逐层增长（T → Vec(T) → Vec(Vec(T)) → …）且永不重复，规模上限管不住——
/// 只能由特化链深度闸拦截。
const PATHOLOGICAL_FILES: &[(&str, &str)] = &[
    (
        "a.yx",
        "use b\nf: (T: Type) -> (x: T) -> T = (x) => {\n    return b.g([x])\n}\n",
    ),
    (
        "b.yx",
        "use a\ng: (T: Type) -> (x: T) -> T = (x) => {\n    return a.f([x])\n}\n",
    ),
    ("main.yx", "use a\nv = a.f(1)\n"),
];

/// 良构跨单元泛型互调两单元（有限深度）：`a.f` 调 `b.g`，`b.g` 调
/// `std.list.first`——链在具体类型处收敛（三层、无类型增长）。
const WELL_FORMED_FILES: &[(&str, &str)] = &[
    (
        "a.yx",
        "use b\nf: (T: Type) -> (x: Vec(T)) -> T = (x) => {\n    return b.g(x)\n}\n",
    ),
    (
        "b.yx",
        "use std.list\ng: (T: Type) -> (x: Vec(T)) -> T = (x) => {\n    return list.first(x)\n}\n",
    ),
    ("main.yx", "use a\nv = a.f([7])\n"),
];

/// 以 MultiFile 形态跑 Driver，单态化上限按 `mono` 配置。
///
/// `run_multi` 固定用 `CompileConfig::default()`；本函数让同一夹具在不同
/// 上限下重跑，用来证明「报错位置随配置移动」——即拦截真的来自深度闸，
/// 而不是某个恰好也说「失败」的旁路。
fn run_multi_with_mono(
    entry: &Path,
    mono: MonoConfig,
) -> Result<DriverOutcome, crate::driver::DriverError> {
    let config = CompileConfig {
        mono,
        ..CompileConfig::default()
    };
    let program = Program::new(
        ProgramKind::MultiFile,
        vec![Unit::new("main", entry, "")],
        config,
    );
    Driver::new().run(program)
}

// ==================== 结构断言辅助 ====================

/// 取多文件故障通道里的 (诊断码, 消息) 列表；非 TypeCheck 故障返回空表。
fn failure_diagnostics(outcome: &DriverOutcome) -> Vec<(String, String)> {
    match &outcome.failure {
        Some(OrchestratorError::TypeCheck { diagnostics, .. }) => diagnostics
            .iter()
            .map(|d| (d.code.clone(), d.message.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// 跑合并 IR 的 codegen + 解释器，返回 (全部全局槽位值, 执行错误)。
fn run_codegen(module: crate::middle::ModuleIR) -> (Vec<String>, Option<String>) {
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = match ctx.generate() {
        Ok(f) => f,
        Err(d) => return (Vec::new(), Some(format!("codegen error: {d}"))),
    };
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(file);
    let mut interp = crate::backends::interpreter::Interpreter::new();
    use crate::backends::Executor as _;
    let exec = interp
        .execute_module(&bytecode_module)
        .err()
        .map(|e| format!("{e:?}"));
    let values = (0..16)
        .map_while(|i| interp.global_slot(i).map(|v| format!("{v:?}")))
        .collect();
    (values, exec)
}

// ==================== 负向：保护不足 ====================

/// 病态跨单元互递归必须**编译期**终止于 E3005（特化链深度超限）且不产出 IR。
///
/// 三态判据：E3005 = 编译期终止（本测试断言）；0xC00000FD = 进程栈溢出
/// （故障通道不是 TypeCheck，或测试进程直接崩）；静默成功 = 无故障且产出 IR
/// 并执行（`failure` 为 None 即命中）。红态：深度链在第一层断掉时
/// `failure` 变 None、E3005 消失，第一条断言即红。
#[test]
fn test_multi_file_cross_unit_mutual_recursion_reports_e3005() {
    // Arrange：两个单元互相以增长类型调用，入口只调一次（有限文本、无界类型）
    let (_dir, entry) = make_script_project(PATHOLOGICAL_FILES);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert①：故障走 TypeCheck 通道且诊断码为 E3005（不是栈溢出、不是静默）
    let diagnostics = failure_diagnostics(&outcome);
    assert!(
        diagnostics.iter().any(|(code, _)| code == "E3005"),
        "跨单元互递归必须在编译期报 E3005（特化链深度超限）；\
         无 E3005 即深度链断在第一层（静默擦除执行）或进程已爆栈。实际故障：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    // Assert②：消息指向特化链深度与默认上限 100（证明拦截来自深度闸）
    let (_, message) = diagnostics
        .iter()
        .find(|(code, _)| code == "E3005")
        .expect("E3005 诊断必须存在");
    assert!(
        message.contains("深度") && message.contains("100"),
        "E3005 消息必须指向特化链深度与上限 100（默认配置），实际：{message}"
    );
    // Assert③：mono 失败后不得产出 IR——否则下游会执行未特化的原件
    assert!(
        outcome.module.is_none(),
        "mono 失败后不得产出 IR（否则下游会执行未特化的原件）"
    );
}

/// 拦截位置随 `mono.max_depth` 配置移动——同一夹具在 150 上限下报出的
/// 当前深度是 151（默认 100 上限的版本见上一个测试）。
///
/// 这条把「E3005 来自深度闸」从旁证变成直证：若拦截来自别的机制（类型错误、
/// 规模上限、解析器），报错位置不会随该配置平移。
///
/// **上限取值说明**：本测试取 150（贴近默认 100，逐层计数肉眼可验）。
/// 上限抬到远超此处量级的形态（5000）另有一条判据
/// `test_multi_file_cross_unit_pathological_recursion_at_5000_reports_e3005`：
/// 修复前（WBS 3.10.1 / issue #447）该形态会在闸门之前耗尽主线程栈
/// （0xC00000FD）——深度闸的强度不能超过「闸门之前那条路径」的栈安全边界。
#[test]
fn test_multi_file_cross_unit_mutual_recursion_depth_follows_configured_max_depth() {
    // Arrange：同一病态夹具，深度上限抬到 150（总数上限 10_000 仍是上界，
    // 先命中的仍是深度闸）
    let (_dir, entry) = make_script_project(PATHOLOGICAL_FILES);
    let mono = MonoConfig {
        max_depth: 150,
        ..MonoConfig::default()
    };

    // Act
    let outcome = run_multi_with_mono(&entry, mono).expect("driver run failed");

    // Assert：诊断码仍为 E3005，且报出的当前深度 = 配置上限 + 1
    let diagnostics = failure_diagnostics(&outcome);
    let (_, message) = diagnostics
        .iter()
        .find(|(code, _)| code == "E3005")
        .unwrap_or_else(|| {
            panic!(
                "上限抬到 150 后仍须在编译期报 E3005（不是爆栈、不是静默）。实际故障：{:?}",
                outcome.failure.as_ref().map(|e| format!("{e}"))
            )
        });
    assert!(
        message.contains("150") && message.contains("151"),
        "E3005 必须报出配置上限 150 与当前深度 151（证明拦截确有深度闸、且逐层跨单元计数），实际：{message}"
    );
}

// ==================== 正向：保护过度 ====================

/// 良构跨单元泛型互调（有限深度）必须编译成功并执行出正确值。
///
/// 链：入口 `a.f([7])` → `b.g(Vec(Int))` → `std.list.first(Int)` → 7。
/// 三层各有跨单元/跨库的泛型实参，深度链有限（类型不增长），不得被深度闸误拒。
/// 红态：深度链误判为发散（E3005）或跨单元特化丢失时，第一条断言即红。
#[test]
fn test_multi_file_cross_unit_generic_chain_compiles_and_runs() {
    // Arrange：a 调 b、b 调 std.list，全链泛型、类型不增长
    let (_dir, entry) = make_script_project(WELL_FORMED_FILES);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert①：不得编译失败（保护过度会把良构链误拒成 E3005）
    assert!(
        outcome.failure.is_none(),
        "有限深度的跨单元泛型互调不得编译失败（深度闸保护过度）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let merged = outcome.module.expect("Linking+mono must produce IR");

    // Assert②：三个单元/库的各层都真的特化过——链跨单元延续的结构证据
    let names: Vec<String> = merged.functions.iter().map(|f| f.name.clone()).collect();
    for expected in ["a.f(int64)", "b.g(int64)", "std.list.first(int64)"] {
        assert!(
            names.iter().any(|n| n == expected),
            "跨单元泛型链必须逐层特化出 {expected}（缺一层即链在现场断掉）：{names:?}"
        );
    }
    // Assert③：参与特化的泛型原件已被特化版替换（不是静默退回类型擦除执行）
    for original in ["a.f", "b.g"] {
        assert!(
            !names.iter().any(|n| n == original),
            "全部调用点都被覆盖的泛型原件必须被删除（保留原件=退回类型擦除执行）：{names:?}"
        );
    }

    // Assert④：执行出正确值 7（跨单元泛型互调的语义与单文件同表达式一致）
    let (values, exec) = run_codegen(merged);
    assert!(
        exec.is_none(),
        "跨单元泛型链执行不得报错（E6006 = 特化版缺失、调用点悬空）：{exec:?}"
    );
    assert!(
        values.iter().any(|v| v == "Int(7)"),
        "a.f([7]) 应执行得 Int(7)，实际全局槽位值：{values:?}"
    );
}

/// 与单文件同表达式的等价判据（C2）：`f`/`g` 同处一模块时同一表达式
/// 必须产出同一可观测值——跨单元不改变语义。
#[test]
fn test_multi_file_cross_unit_generic_chain_matches_single_file_value() {
    // Arrange：把两单元的实现合并进一个 Script 模式单文件（顶层绑定即程序体）
    let single_source = "first: (T: Type) -> (x: Vec(T)) -> T = (x) => {\n    return x[0]\n}\ng: (T: Type) -> (x: Vec(T)) -> T = (x) => {\n    return first(x)\n}\nf: (T: Type) -> (x: Vec(T)) -> T = (x) => {\n    return g(x)\n}\nv = f([7])\n";
    let (_dir, entry) = make_script_project(&[("main.yx", single_source)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：单文件形态同表达式（经 mono 路径）同样得 Int(7)
    assert!(
        outcome.failure.is_none(),
        "单文件对照程序不得编译失败：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let (values, exec) = run_codegen(outcome.module.expect("must produce IR"));
    assert!(exec.is_none(), "单文件对照程序执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(7)"),
        "跨单元与单文件同表达式必须产出同一值 Int(7)，实际：{values:?}"
    );
}

/// 病态递归在默认套件里的深度档：越过修复前的原生栈崩点（约第 2500 层）——
/// 即「原生栈不再随特化深度增长」这条 3.9/3.10 修复在默认套件里仍被覆盖——
/// 同时把堆内存压在 16 GB 托管 runner 可承受的范围（5000 档实测峰值 15.6 GB，
/// 会让 CI 的测试作业被 OOM 杀掉，见 issue #449；那一档由下方 `#[ignore]` 用例保留）。
const PATHOLOGICAL_CI_DEPTH: usize = 2600;

/// 3.10.1 判据（CI 预算档）：病态泛型递归在越过旧崩点的深度上仍必须**编译期**
/// 终止于 E3005，且报出配置上限与当前深度——不是耗尽原生栈崩进程。
///
/// `cost:` 标记（#486 第 1 项）：峰值 8.11 GiB = 5000 档实测 15.6 GiB × 2600/5000
/// （3.19 MiB/层线性外推，**峰值未实测**；time=15s 为 2026-10-21 本机独占波实测 wall clock）。
/// `mode=exclusive` 表示这一档的内存装不下第二个实例——测试体开头据此取跨进程独占锁
/// （`test_support::exclusive_lock`），预算通道（`scripts/dev/testsuite.py`）据此把它
/// 从批量波里剔除、独占一波执行。
// cost: peak=8307MiB time=15s mode=exclusive source=extrapolated@issue-449-5000-tier-ratio
#[test]
fn test_multi_file_cross_unit_pathological_recursion_within_ci_budget_reports_e3005() {
    // Arrange：先拿独占锁（见上方 cost 标记），再抬深度上限到 CI 预算内的 PATHOLOGICAL_CI_DEPTH
    let _exclusive = crate::util::test_support::exclusive_lock(
        "test_multi_file_cross_unit_pathological_recursion_within_ci_budget_reports_e3005",
    );
    let (_dir, entry) = make_script_project(PATHOLOGICAL_FILES);
    let mono = MonoConfig {
        max_depth: PATHOLOGICAL_CI_DEPTH,
        ..MonoConfig::default()
    };

    // Act
    let outcome = run_multi_with_mono(&entry, mono).expect("driver run failed");

    // Assert：E3005 + 真实上限/当前深度（闸门真的走到该层，而不是崩栈）
    let diagnostics = failure_diagnostics(&outcome);
    let (_, message) = diagnostics
        .iter()
        .find(|(code, _)| code == "E3005")
        .unwrap_or_else(|| {
            panic!(
                "上限抬到 {PATHOLOGICAL_CI_DEPTH} 后仍须在编译期报 E3005（不是爆栈、不是静默）。实际故障：{:?}",
                outcome.failure.as_ref().map(|e| format!("{e}"))
            )
        });
    assert!(
        message.contains(&PATHOLOGICAL_CI_DEPTH.to_string())
            && message.contains(&(PATHOLOGICAL_CI_DEPTH + 1).to_string()),
        "E3005 必须报出配置上限 {PATHOLOGICAL_CI_DEPTH} 与当前深度 {}，实际：{message}",
        PATHOLOGICAL_CI_DEPTH + 1
    );
    assert!(
        outcome.module.is_none(),
        "mono 失败后不得产出 IR（否则下游会执行未特化的原件）"
    );
}

/// WBS 3.10.1 判据（生产默认档）：抬高 `mono.max_depth` 到 5000 后，病态泛型递归仍必须
/// **编译期**终止于 E3005（含真实上限 5000）——而不是在闸门之前耗尽原生栈崩进程
/// （修复前实测：编译到约第 2500 层 0xC00000FD）。
///
/// `#[ignore]` 的理由：本档实测 72.6 s / 峰值 15.6 GB（2026-10-13，issue #449），
/// 16 GB 的 GitHub 托管 runner 会被 OOM 杀掉（表现是测试作业被取消），故不进默认套件。
/// 显式运行：
/// `cargo test --lib --all-features driver::tests::mono_cross_unit::test_multi_file_cross_unit_pathological_recursion_at_5000 -- --ignored --exact`
///
/// `cost:` 标记（#486 第 1 项）：峰值 15.6 GiB / 72.6 s 为 issue #449 的**实测**（2026-10-13）。
/// `mode=exclusive`：15.6 GiB 装不下第二个实例——测试体开头取跨进程独占锁，手工并跑两路时
/// 后来者等待而不是把自己的 15.6 GiB 叠上去。
// cost: peak=15974MiB time=72.6s mode=exclusive source=measured@2026-10-13
#[test]
#[ignore = "5000 档实测 72.6 s / 15.6 GB，会 OOM 掉 16 GB 的 CI runner；默认套件由 PATHOLOGICAL_CI_DEPTH 档覆盖（issue #449）"]
fn test_multi_file_cross_unit_pathological_recursion_at_5000_reports_e3005() {
    // Arrange：先拿独占锁（见上方 cost 标记），再把深度上限抬到 5000（总数上限保持默认）
    let _exclusive = crate::util::test_support::exclusive_lock(
        "test_multi_file_cross_unit_pathological_recursion_at_5000_reports_e3005",
    );
    let (_dir, entry) = make_script_project(PATHOLOGICAL_FILES);
    let mono = MonoConfig {
        max_depth: 5000,
        ..MonoConfig::default()
    };

    // Act
    let outcome = run_multi_with_mono(&entry, mono).expect("driver run failed");

    // Assert：E3005 + 真实上限 5000/当前 5001（闸门真的走到 5000 层，而不是崩栈）
    let diagnostics = failure_diagnostics(&outcome);
    let (_, message) = diagnostics
        .iter()
        .find(|(code, _)| code == "E3005")
        .unwrap_or_else(|| {
            panic!(
                "上限抬到 5000 后仍须在编译期报 E3005（不是爆栈、不是静默）。实际故障：{:?}",
                outcome.failure.as_ref().map(|e| format!("{e}"))
            )
        });
    assert!(
        message.contains("5000") && message.contains("5001"),
        "E3005 必须报出配置上限 5000 与当前深度 5001，实际：{message}"
    );
    assert!(
        outcome.module.is_none(),
        "mono 失败后不得产出 IR（否则下游会执行未特化的原件）"
    );
}
