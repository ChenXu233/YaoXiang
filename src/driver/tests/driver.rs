//! Driver 测试 — 基于 02-stage-contract §目标设计 3/4 / RFC-039 L1
//!
//! §3: Driver 是 Program 的唯一解释器，dispatch 对 Stage 穷尽 match
//! §4: 跳过由拓扑判定并记录原因（4.1.3：仅 DriverOutcome.skipped 内部记录，
//!     不外发诊断——外发随 4.3 义务账本）
//! 验收锚点（09-WBS 4.1.3）：Pipeline::run 改走 Driver 后，单文件路径
//! 诊断集与退出码逐字节相同。下列用例逐条钉住 pipeline.rs 原函数体的
//! 可观测行为：错误分类映射、parse 只取首错、死代码开关与 typecheck 警告
//! 的耦合门控、FailFast 首错即中止。

use crate::driver::{
    Aggregation, Driver, DriverError, DriverOutcome, Program, ProgramKind, SkipReason, Stage, Unit,
};
use crate::frontend::config::CompileConfig;
use crate::frontend::module::orchestrator::OrchestratorError;
use crate::frontend::pipeline::{PipelineError};
use std::path::{Path, PathBuf};

/// 构造 SingleFile 程序并跑 Driver。
fn run_single(
    source: &str,
    config: CompileConfig,
) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("test", "test.yx", source)],
        config,
    );
    Driver::new().run(program)
}

/// 取某阶段的跳过原因（未跳过 = None）。
fn skip_reason(
    outcome: &DriverOutcome,
    stage: Stage,
) -> Option<SkipReason> {
    outcome
        .skipped
        .iter()
        .find(|s| s.stage == stage)
        .map(|s| s.reason)
}

#[test]
fn test_driver_run_success_skips_obligation_free_stages() {
    // Arrange: 合法源——无证明义务、无泛型实例化请求
    let source = "main: () -> Void = {}";

    // Act
    let outcome = run_single(source, CompileConfig::default()).expect("driver run failed");

    // Assert: 成功产 IR；无义务的阶段被记录为 NoObligations 跳过（仅内部记录）
    assert!(
        outcome.result.is_success(),
        "valid source must compile: {:?}",
        outcome.result.errors
    );
    assert!(
        matches!(
            skip_reason(&outcome, Stage::ProofExecution),
            Some(SkipReason::NoObligations)
        ),
        "proof stage must record NoObligations skip: {:?}",
        outcome.skipped
    );
    assert!(
        matches!(
            skip_reason(&outcome, Stage::Monomorphization),
            Some(SkipReason::NoObligations)
        ),
        "mono stage must record NoObligations skip: {:?}",
        outcome.skipped
    );
    assert!(
        skip_reason(&outcome, Stage::DeadCodeAnalysis).is_none(),
        "dead-code stage is enabled by default and must run: {:?}",
        outcome.skipped
    );
}

#[test]
fn test_driver_run_typecheck_error_maps_and_aborts() {
    // Arrange: 类型不匹配（Int 绑定 String 字面量）
    let source = "let x: Int = \"hello\";";

    // Act
    let outcome = run_single(source, CompileConfig::default()).expect("driver run failed");

    // Assert: 诊断全部归入 TypeCheck；FailFast 首错即中止——下游阶段根本
    // 不被访问（02 §3 草图的 break 语义），故 skipped 为空
    assert!(!outcome.result.is_success(), "type error must fail the run");
    assert!(
        !outcome.result.errors.is_empty(),
        "type error must produce diagnostics"
    );
    for e in &outcome.result.errors {
        assert!(
            matches!(e, PipelineError::TypeCheck(_)),
            "typecheck failure must map to TypeCheck: {e:?}"
        );
    }
    assert!(outcome.result.ir.is_none(), "aborted run yields no IR");
    assert!(
        outcome.skipped.is_empty(),
        "FailFast aborts before visiting downstream stages: {:?}",
        outcome.skipped
    );
}

#[test]
fn test_driver_run_dead_code_disabled_drops_all_warnings() {
    // Arrange: 同上，但关闭死代码开关——pipeline.rs:275-278 现状门控把
    // typecheck 警告一并丢弃（耦合必须忠实复刻，逐字节判据）
    let source = "main: () -> Void = {}\nunused_helper: () -> Void = {}";
    let config = CompileConfig::default().with_dead_code_enabled(false);

    // Act
    let outcome = run_single(source, config).expect("driver run failed");

    // Assert: 警告清零 + 记录 ConfigDisabled 跳过（内部）
    assert!(
        outcome.result.is_success(),
        "dead-code gate must not turn into errors: {:?}",
        outcome.result.errors
    );
    assert!(
        outcome.result.warnings.is_empty(),
        "dead_code.enabled=false drops the whole warning stream: {:?}",
        outcome.result.warnings
    );
    assert!(
        matches!(
            skip_reason(&outcome, Stage::DeadCodeAnalysis),
            Some(SkipReason::ConfigDisabled)
        ),
        "dead-code stage must record ConfigDisabled skip: {:?}",
        outcome.skipped
    );
}

#[test]
fn test_driver_run_mono_disabled_records_config_skip() {
    // Arrange: 关闭单态化开关
    let source = "main: () -> Void = {}";
    let config = CompileConfig {
        mono: crate::frontend::config::MonoConfig {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // Act
    let outcome = run_single(source, config).expect("driver run failed");

    // Assert
    assert!(
        outcome.result.is_success(),
        "mono gate must not affect success: {:?}",
        outcome.result.errors
    );
    assert!(
        matches!(
            skip_reason(&outcome, Stage::Monomorphization),
            Some(SkipReason::ConfigDisabled)
        ),
        "mono stage must record ConfigDisabled skip: {:?}",
        outcome.skipped
    );
}

#[test]
fn test_driver_run_empty_units_rejected() {
    // Arrange: 无编译单元的程序是调用方 bug——显式错误，不许静默 vacuous success
    let program = Program::new(
        ProgramKind::SingleFile,
        Vec::new(),
        CompileConfig::default(),
    );

    // Act
    let result = Driver::new().run(program);

    // Assert
    assert!(
        matches!(result, Err(DriverError::EmptyUnits)),
        "empty units must be rejected: {result:?}"
    );
}

// ============ 3.4.8 ③ 回归修复：Script 顶层同名重绑定（后者生效）============

/// Script 模式（无 manifest）的顶层同名重绑定：**后者生效**——被遮蔽的前一个定义
/// 从函数表移除（单文件扁平函数表按序插入的语义；实测 `run` 输出 2）。
/// 不这么做时同一模块里会出现两份同名 IR，Linking 的合并校验把「同一模块内的
/// 重绑定」误报成「模块限定名冲突（同一路径 ×2）」并打红整个 check
///（3.4.8 ③ 回归，文档示例门禁 10 项中 6 项即此形状）。
#[test]
fn test_driver_run_script_rebinding_keeps_last_definition() {
    // Arrange：同一顶层名 `f` 先声明（带注解）再重绑定——docs「完整形式 / 简短
    // 形式」示例形状；typecheck 接受该形状（无注解的重复绑定会报 E2002，那是
    // 另一条既有判据，不经 IR）
    let source = "f: () -> Int = () => 1\nf = () => 2\nv = f()\n";

    // Act
    let outcome = run_single(source, CompileConfig::default()).expect("driver run failed");

    // Assert：函数表只剩后者；执行取到的也是后者（2）
    assert!(
        outcome.result.is_success(),
        "Script 重绑定程序必须编译成功: {:?}",
        outcome.result.errors
    );
    let ir = outcome
        .result
        .ir
        .expect("single-file script must produce IR");
    let definitions = ir.functions.iter().filter(|f| f.name == "f").count();
    assert_eq!(
        definitions,
        1,
        "Script 重绑定后者生效：函数表不得留两份 f: {:?}",
        ir.functions
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>()
    );
    let value = run_merged_slot0(ir);
    assert_eq!(
        value, "Int(2)",
        "被遮蔽的前定义不得再生效：v = f() 应取后者（2），实际 {value:?}"
    );
}

// ===================== 4.2.1 多文件形态（A1/B1/C3 裁决，2026-10-09）=====================

/// 在临时目录落一个多文件项目（yaoxiang.toml + 给定 .yx），返回 (dir 守卫, 入口路径)。
///
/// 清单必须是**合法**的（含 `[package] version`）：WBS 3.23.1 / #503 起，
/// 存在但不可读的清单报 E3024 并中止 RoleClassification 之后的死代码分析——
/// 缺 version 的清单会把本组判据（W1001/per-file 收集）的触发前提整个换掉。
fn make_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest failed");
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).expect("write fixture failed");
    }
    let entry = dir.path().join("main.yx");
    (dir, entry)
}

/// 以 MultiFile 形态跑 Driver（入口单元为占位——Discovery 臂负责展开）。
pub(super) fn run_multi(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::MultiFile,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

// ─────────────────────────────────────────────────────────────────────────────
// WBS 3.4.8 ②（2026-10-10）：多文件单态化臂接线——行为判据
// ─────────────────────────────────────────────────────────────────────────────

/// 多文件项目夹具：写盘但**不放 manifest**——入口按 Script 角色编译，
/// 顶层绑定即程序体（值绑定 `v = …` 在初始化期求值，可经解释器全局槽位观察）。
/// 带 manifest 的 Bin 角色还要求 main 绑定（E3020），与本组判据无关。
pub(super) fn make_script_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let mut entry = PathBuf::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write project file");
        if *name == "main.yx" {
            entry = path;
        }
    }
    (dir, entry)
}

// ─────────────────────────────────────────────────────────────────────────────
// t3 F1：多文件 IR 自洽——模块顶层初始化序列的调用名也必须限定
// ─────────────────────────────────────────────────────────────────────────────

/// F1 红判据（t3 p9/p10 的驱动侧复现）：多文件下**模块顶层调用本模块顶层函数**
/// 时，函数定义被 `qualify_names` 限定（`main.main.helper`）、而 `ir.init` 里的
/// 调用名此前保持裸名 → 运行期按裸名查扁平函数表落空（E6006）。
/// 与泛型无关：这里 `helper` 是普通非泛型函数。
#[test]
fn test_driver_run_multi_file_top_level_call_to_local_function_runs() {
    // Arrange：single.yx 顶层绑定直接调用同文件普通函数（无 use、无泛型）
    let (_dir, entry) =
        make_script_project(&[("main.yx", "helper: () -> Int = () => { 7 }\nv = helper()\n")]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：编译成功且 init 段真的把 v 算出来了（Int(7)）
    assert!(
        outcome.failure.is_none(),
        "顶层调用本文件函数不得编译失败：{:?}",
        outcome.failure
    );
    let value = run_merged_slot0(outcome.module.expect("must produce IR"));
    assert!(
        value == "Int(7)",
        "多文件顶层调用本文件函数应得 Int(7)（F1 前是 E6006：函数表缺失裸名 helper），实际：{value:?}"
    );
}

/// F1 结构判据：合并 IR 里 init 段的调用名必须与函数表键**同源**（都带模块键）。
/// 这条不依赖运行期，直接钉「IR 自洽」——只改函数体不改 init 就会在此红。
#[test]
fn test_driver_run_multi_file_init_calls_use_qualified_names() {
    // Arrange：同一项目形态，另加一个被顶层调用的函数
    let (_dir, entry) =
        make_script_project(&[("main.yx", "helper: () -> Int = () => { 7 }\nv = helper()\n")]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    let merged = outcome.module.expect("must produce IR");

    // Assert：init 里出现的调用名都在函数表里（自洽），且裸名 helper 不存在
    let function_names: Vec<String> = merged.functions.iter().map(|f| f.name.clone()).collect();
    let init_calls: Vec<String> =
        merged
            .init
            .iter()
            .filter_map(|i| match i {
                crate::middle::ir::ir::Instruction::Call {
                    func:
                        crate::middle::ir::ir::Operand::Const(
                            crate::middle::ir::ir::ConstValue::String(n),
                        ),
                    ..
                } => Some(n.clone()),
                _ => None,
            })
            .collect();
    assert!(
        !init_calls.is_empty(),
        "夹具应至少有一条 init 调用（否则判据空转）：{init_calls:?}"
    );
    assert!(
        !init_calls.iter().any(|n| n == "helper"),
        "init 段不得残留裸名 helper——函数表里只有 main.helper，裸名即 E6006：{init_calls:?}"
    );
    assert!(
        init_calls.iter().all(|n| function_names.contains(n)),
        "init 段调用名必须都在函数表里（同源）：init={init_calls:?} funcs={function_names:?}"
    );
}
/// 多文件编译产物跑完 codegen + 解释器后，全局槽位 0 的值。
/// 判据与 codegen 全局槽位测试同源：`v` 是入口顶层值绑定，槽位由
/// `allocate_global_slots` 分配（本项目内它是 0 号）。
/// 多文件产物跑完 codegen + 解释器后，**全部**全局槽位的值（调试字符串）。
/// 槽位号由 `allocate_global_slots` 按单元发现序分配，本组用例不依赖具体号，
/// 故取全集断言「值出现过」。
pub(super) fn run_merged_slot_values(module: crate::middle::ModuleIR) -> Vec<String> {
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = ctx.generate().expect("codegen must succeed");
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(file);
    let mut interp = crate::backends::interpreter::Interpreter::new();
    use crate::backends::Executor as _;
    interp
        .execute_module(&bytecode_module)
        .expect("interpreter must execute merged module");
    (0..16)
        .map_while(|i| interp.global_slot(i).map(|v| format!("{v:?}")))
        .collect()
}

fn run_merged_slot0(module: crate::middle::ModuleIR) -> String {
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = ctx.generate().expect("codegen must succeed");
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(file);
    let mut interp = crate::backends::interpreter::Interpreter::new();
    use crate::backends::Executor as _;
    interp
        .execute_module(&bytecode_module)
        .expect("interpreter must execute merged module");
    format!(
        "{:?}",
        interp.global_slot(0).expect("global slot 0 must exist")
    )
}

/// 3.4.8 ②：病态泛型递归 `f(x) = f([x])` 在多文件下必须在**编译期**报
/// E3005（特化链深度超限）——mono 介入前该程序以类型擦除执行到进程栈
/// 溢出（0xC00000FD）。断言：有故障 + 诊断码 E3005 + 阶段为 mono。
#[test]
fn test_driver_run_multi_file_pathological_generic_recursion_reports_e3005() {
    // Arrange：lib.f 递归调用自身、实参类型逐层增长（T → Vec(T) → …）
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "f: (T: Type) -> (x: T) -> T = (x) => {\n    return f([x])\n}\n",
        ),
        ("main.yx", "use lib\nv = lib.f(1)\n"),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：编译期 E3005，不是栈溢出、也不是静默成功
    let Some(OrchestratorError::TypeCheck { diagnostics, .. }) = &outcome.failure else {
        panic!(
            "病态泛型递归必须编译期失败（E3005），实际：{:?}",
            outcome.failure.as_ref().map(|e| format!("{e}"))
        );
    };
    assert!(
        diagnostics.iter().any(|d| d.code == "E3005"),
        "诊断必须是 E3005（特化链深度超限）：{diagnostics:?}"
    );
    assert!(
        outcome.module.is_none(),
        "mono 失败后不得产出 IR（否则下游会执行未特化的原件）"
    );
}

#[test]
fn test_driver_run_multi_file_proof_error_yields_to_typecheck_error() {
    // Arrange（A1 裁决钉板）：aaa.yx 带一个会失败的证明义务（返回位注解），
    // zzz.yx 带类型错误。发现序 aaa < main < zzz——旧交错顺序会在 aaa 的
    // typecheck 通过后立刻执行其 proof 并报 E4018；A1 阶段分离后，
    // ProofExecution 在全部 typecheck 之后，zzz 的类型错误必须先报
    let (_dir, entry) = make_project(&[
        (
            "aaa.yx",
            "IsLe: (n: Int, r: Int) -> Type = { r < n }\nf: () -> (r: IsLe(3, r)) = {\n    return 7\n}\n",
        ),
        ("main.yx", "use aaa.{f}\nuse zzz.{bad}\nmain: () -> Void = {}\n"),
        ("zzz.yx", "bad: Int = \"nope\"\n"),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert: 报 zzz 的 typecheck 错误而非 aaa 的 E4018
    let Some(OrchestratorError::TypeCheck {
        path, diagnostics, ..
    }) = &outcome.failure
    else {
        panic!(
            "expected OrchestratorError::TypeCheck, got {:?}",
            outcome.failure
        );
    };
    assert!(
        path.ends_with("zzz.yx"),
        "A1: typecheck failure must precede proof failure: {path}"
    );
    assert!(
        !diagnostics.iter().any(|d| d.code == "E4018"),
        "proof error must not surface before typecheck stage completes: {diagnostics:?}"
    );
}

#[test]
fn test_driver_run_multi_file_parse_error_hard_aborts() {
    // Arrange：broken.yx 语法错误——compile 路径（FailFast）parse 错误
    // 硬中止是**正确语义**（坏文件产不出 IR），本钉板长期有效；Check
    // 路径的降级由 test_driver_run_check_parse_error_collected_per_file
    // 钉住（4.10.1 已修复，2026-10-09）
    let (_dir, entry) = make_project(&[
        ("broken.yx", "let = ;\n"),
        ("main.yx", "use broken.{x}\nmain: () -> Void = {}\n"),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        matches!(&outcome.failure, Some(OrchestratorError::Parse { path, .. }) if path.ends_with("broken.yx")),
        "parse error must hard-abort as OrchestratorError::Parse: {:?}",
        outcome.failure
    );
}

// ===================== 4.2.2 Check 形态（顺序归一 / E3020 归属裁决，2026-10-09）=====================
// 原未接线钉板 `test_driver_run_check_kind_hits_unwired_role_classification`
// 随本批接线完成退役——Check 形态全接线后的行为由下列用例钉住。

/// 以 Check 形态跑 Driver（占位入口单元，Discovery 臂沿 use 图展开）。
fn run_check(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::Check,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

/// 取某文件在 check 通道里的诊断（按路径后缀匹配——发现集含嵌入 std 虚拟路径）。
fn check_diags_of<'a>(
    outcome: &'a DriverOutcome,
    suffix: &str,
) -> Option<&'a [crate::util::diagnostic::Diagnostic]> {
    outcome
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.ends_with(suffix))
        .map(|(_, d)| d.as_slice())
}

/// manifest 声明面带 `[[bin]] main.yx` 的项目夹具（文件无 main 绑定，
/// 另有一个未使用函数供顺序断言用）。
fn make_declared_bin_project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n\n[[bin]]\nname = \"app\"\npath = \"main.yx\"\n",
    )
    .expect("write manifest failed");
    std::fs::write(dir.path().join("main.yx"), "helper: () -> Void = {}\n")
        .expect("write fixture failed");
    let entry = dir.path().join("main.yx");
    (dir, entry)
}

#[test]
fn test_driver_run_check_diagnostic_order_follows_stage_topology() {
    // Arrange: 顺序归一裁决（2026-10-09 用户定夺，02 §1 修订注记 #2 扩展）——
    // 文件内诊断顺序 = 阶段拓扑序：E3020（RoleClassification）先于
    // W1001（DeadCodeAnalysis）；诊断集合不变，仅顺序归一
    let (_dir, entry) = make_declared_bin_project();

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    let diags = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(
        codes,
        ["E3020", "W1001"],
        "文件内诊断必须按阶段拓扑序归一（E3020 → W1001）: {codes:?}"
    );
}

#[test]
fn test_driver_run_check_parse_error_collected_per_file() {
    // Arrange（4.10.1 修复钉板，rust 式收集语义，2026-10-09 用户裁决）——
    // 本测试由 test_driver_run_check_parse_error_hard_aborts 显式翻转而来：
    // broken.yx 语法错误不再整体硬中止，parse 失败降级为逐文件诊断收集；
    // 带病文件退出编译单元（方案 B：AST 槽 None，registry/typecheck 等
    // 下游全跳过），导入方报模块未找到（E5001）
    let (_dir, entry) = make_project(&[
        ("broken.yx", "let = ;\n"),
        ("main.yx", "use broken.{x}\nmain: () -> Void = {}\n"),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "check 路径 parse 错误不得再走故障通道: {:?}",
        outcome.failure
    );
    let broken = check_diags_of(&outcome, "broken.yx").expect("entry for broken.yx");
    assert!(
        broken
            .iter()
            .any(|d| matches!(d.severity, crate::util::diagnostic::Severity::Error)),
        "带病文件必须收 Error 级 parse 诊断: {broken:?}"
    );
    let main = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        main.iter().any(|d| d.code == "E5001"),
        "导入带病模块应报模块未找到（方案 B 语义）: {main:?}"
    );
}

#[test]
fn test_driver_run_check_proof_error_collected_per_file() {
    // Arrange（A1 对照钉板）：aaa.yx 带一个会失败的证明义务（返回位注解），
    // zzz.yx 带类型错误。compile 路径下 zzz 的类型错误掩盖 aaa 的 E4018
    //（A1 首报归一）；check 是 CollectAll——两者都必须按文件收集到位
    let (_dir, entry) = make_project(&[
        (
            "aaa.yx",
            "IsLe: (n: Int, r: Int) -> Type = { r < n }\nf: () -> (r: IsLe(3, r)) = {\n    return 7\n}\n",
        ),
        ("main.yx", "use aaa.{f}\nuse zzz.{bad}\nmain: () -> Void = {}\n"),
        ("zzz.yx", "bad: Int = \"nope\"\n"),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "proof/类型错误都是收集诊断，不得硬中止: {:?}",
        outcome.failure
    );
    let aaa = check_diags_of(&outcome, "aaa.yx").expect("entry for aaa.yx");
    assert!(
        aaa.iter().any(|d| d.code == "E4018"),
        "aaa.yx 的证明失败必须进收集通道: {aaa:?}"
    );
    let zzz = check_diags_of(&outcome, "zzz.yx").expect("entry for zzz.yx");
    assert!(!zzz.is_empty(), "zzz.yx 的类型错误必须进收集通道: {zzz:?}");
}

// ===================== 4.2.3 Lsp 形态（按文件来源分流裁决，2026-10-09）=====================

/// 以 Lsp 形态跑 Driver：占位入口单元携带内存缓冲区源码（脏缓冲区），
/// Discovery 重建单元集时入口单元的源码以缓冲区为准。
fn run_lsp(
    entry: &Path,
    buffer: &str,
) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::Lsp,
        vec![Unit::new("main", entry, buffer)],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

#[test]
fn test_driver_run_lsp_typechecks_buffer_with_disk_registry() {
    // Arrange: 磁盘项目干净；缓冲区把 add_one 的实参改成错误类型——LSP
    // 契约：被编辑内容参与 typecheck，磁盘 lib 的签名照常解析（编辑器
    // 里的跨文件结果与 yaoxiang check 一致）
    let (_dir, entry) = make_project(&[
        (
            "lib.yx",
            "add_one: (x: Int) -> Int = (x) => {\n    return x + 1\n}\n",
        ),
        (
            "main.yx",
            "use lib.{add_one}\nmain = () => {\n    y = add_one(1)\n}\n",
        ),
    ]);
    let buffer = "use lib.{add_one}\nmain = () => {\n    y = add_one(\"nope\")\n}\n";

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "LSP 诊断收集不得走故障通道: {:?}",
        outcome.failure
    );
    let main = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        main.iter().any(|d| d.code == "E1002"),
        "缓冲区的类型错误必须被收集: {main:?}"
    );
    assert!(
        !main.iter().any(|d| d.code == "E5001"),
        "磁盘 lib 的签名必须照常解析（不得误报模块未找到）: {main:?}"
    );
}

#[test]
fn test_driver_run_lsp_buffer_parse_error_keeps_partial_ast() {
    // Arrange（编辑器哲学钉板）：缓冲区末尾有一处语法错误（E0012），但
    // 幸存语句的类型错误（E1002）仍应被检出——残缺 AST 继续 typecheck，
    // 而非整体丢弃（与 Check 路径的方案 B 按文件来源分流）
    let (_dir, entry) = make_project(&[("main.yx", "main: () -> Void = {}\n")]);
    let buffer = "main: Int = \"nope\"\nlet = ;\n";

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "缓冲区 parse 错误不得硬中止: {:?}",
        outcome.failure
    );
    let main = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    let codes: Vec<&str> = main.iter().map(|d| d.code.as_str()).collect();
    assert!(
        codes.contains(&"E0012"),
        "缓冲区 parse 错误必须收集: {codes:?}"
    );
    assert!(
        codes.contains(&"E1002"),
        "幸存语句必须继续 typecheck（编辑器哲学）: {codes:?}"
    );
}

#[test]
fn test_driver_run_lsp_disk_parse_error_degrades_like_check() {
    // Arrange（LSP 硬中止怪癖修复钉板）：磁盘上 lib.yx 语法错误——旧行为
    // build_registry_from 硬中止，handler 静默退回单文件路径（跨文件解析
    // 能力丧失）；裁决后磁盘文件同 Check 方案 B（收集 + 退出编译单元），
    // 缓冲区照常检查，导入方报模块未找到
    let (_dir, entry) = make_project(&[
        ("lib.yx", "let = ;\n"),
        ("main.yx", "use lib.{x}\nmain: () -> Void = {}\n"),
    ]);
    let buffer = "use lib.{x}\nmain: () -> Void = {}\n";

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "磁盘文件 parse 错误不得硬中止 LSP: {:?}",
        outcome.failure
    );
    let lib = check_diags_of(&outcome, "lib.yx").expect("entry for lib.yx");
    assert!(
        lib.iter().any(|d| d.code == "E0012"),
        "磁盘带病文件的 parse 诊断入其条目: {lib:?}"
    );
    let main = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        main.iter().any(|d| d.code == "E5001"),
        "导入带病磁盘模块报模块未找到（方案 B 收敛）: {main:?}"
    );
}

#[test]
fn test_driver_run_lsp_buffer_lex_error_collected() {
    // Arrange（编辑器哲学延伸）：缓冲区词法失败（未终结字符串）——无
    // tokens 可 parse，收词法诊断并跳过 typecheck，不再硬中止（旧行为
    // 经 OrchestratorError::Parse 让 handler 退回单文件路径）
    let (_dir, entry) = make_project(&[("main.yx", "main: () -> Void = {}\n")]);
    let buffer = "main = () => {\n    s = \"unterminated\n}\n";

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "缓冲区词法错误不得硬中止: {:?}",
        outcome.failure
    );
    let main = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        main.iter()
            .any(|d| matches!(d.severity, crate::util::diagnostic::Severity::Error)),
        "词法错误必须以 Error 级收集: {main:?}"
    );
}

// ===================== 4.2.4 Embedded 形态 =====================

/// 以 Embedded 形态跑 Driver（调用方共享注册表随程序注入，#94）。
/// 嵌入源是编译期常量——无需临时目录。
fn run_embedded(key: &str) -> Result<DriverOutcome, DriverError> {
    let source =
        crate::std::yx_sources::embedded_std_source(key).expect("embedded std source must exist");
    let program = Program::new(
        ProgramKind::Embedded,
        vec![Unit::new(
            key,
            format!("<{}>", key.replace('.', "/")),
            source,
        )],
        CompileConfig::default(),
    )
    .with_shared_registry(crate::frontend::module::registry::ModuleRegistry::with_std());
    Driver::new().run(program)
}

#[test]
fn test_driver_run_embedded_compiles_embedded_std_module() {
    // Arrange: 嵌入 std 模块 std.list（#117 硬切换后 for 循环脱糖依赖
    // iter/has_next/next）作为独立编译单元——EMBEDDED_STAGES 四臂
    //（无 Registry/GlobalSlotAlloc/Linking，registry 由调用方注入共享）

    // Act
    let outcome = run_embedded("std.list").expect("driver run failed");

    // Assert: 无故障；产出独立 ModuleIR，迭代协议函数带限定名在产物中
    assert!(
        outcome.failure.is_none(),
        "嵌入 std 模块编译不得失败: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("Embedded 形态必须产出 ModuleIR");
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    for expected in ["std.list.iter", "std.list.has_next", "std.list.next"] {
        assert!(
            names.contains(&expected),
            "迭代协议函数 {expected} 必须在产物中: {names:?}"
        );
    }
}

// ===================== 4.2.5 standalone 统一（裁决 A，2026-10-09）=====================

#[test]
fn test_driver_run_check_standalone_entry_warns_dead_code() {
    // Arrange: 无 manifest 的 standalone 文件——警告面只覆盖入口文件本身
    //（check_single_file 的单文件语义直译：Script 角色 + 无项目引用池）；
    // 被带入的邻旁文件只收错误、不警告（宁漏勿误）
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    std::fs::write(
        dir.path().join("main.yx"),
        "dead = (x: Int) => x\nmain = () => { x = 1 }\n",
    )
    .expect("write fixture failed");
    let entry = dir.path().join("main.yx");

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "standalone check 不得走故障通道: {:?}",
        outcome.failure
    );
    let diags = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        diags.iter().any(|d| d.code == "W1001"),
        "standalone 入口文件的死代码警告不得丢失（4.2.5 防回归）: {diags:?}"
    );
}

#[test]
fn test_driver_run_check_standalone_resolves_relative_use() {
    // Arrange（裁决 A 钉板）：无 manifest 的单文件带相对 use——Discovery
    // 沿导入者目录解析邻旁文件（与 rustc 单文件 mod 解析对齐）；旧
    // check_single_file 路径报 E5001
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    std::fs::write(
        dir.path().join("b.yx"),
        "f: (x: Int) -> Int = (x) => x + 1\n",
    )
    .expect("write fixture failed");
    std::fs::write(
        dir.path().join("main.yx"),
        "use b.{f}\nmain = () => {\n    y = f(1)\n}\n",
    )
    .expect("write fixture failed");
    let entry = dir.path().join("main.yx");

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    let main = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        !main.iter().any(|d| d.code == "E5001"),
        "standalone 相对导入不得再报模块未找到: {main:?}"
    );
    assert!(
        check_diags_of(&outcome, "b.yx").is_some(),
        "邻旁文件必须进发现集（每个发现文件都有条目）"
    );
}

#[test]
fn test_driver_run_check_embedded_std_keeps_merged_surface() {
    // Arrange（4.2.5 施工发现的潜伏缺陷钉板）：同时使用 std.result 与
    // std.test——with_std() 已按「native + yx 表面合并」注册嵌入模块，
    // Registry 臂若重复收获注册会把 native 半面（result.is_err 等）整体
    // 顶掉（std.test 内部调用 result.is_err → 误报 E1043）
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    std::fs::write(
        dir.path().join("main.yx"),
        "use std.result\nuse std.test\n\nmain: () -> Void = {}\n",
    )
    .expect("write fixture failed");
    let entry = dir.path().join("main.yx");

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert: 全部条目的诊断集合都不得出现 E1043
    let all: Vec<&str> = outcome
        .check_diagnostics
        .iter()
        .flat_map(|(_, ds)| ds.iter().map(|d| d.code.as_str()))
        .collect();
    assert!(
        !all.contains(&"E1043"),
        "嵌入 std 的合并表面不得被重复注册顶掉: {all:?}"
    );
}

// ===================== 4.2.6 SingleFile+CollectAll 形态（LSP 单文件兜底）=====================

/// 以 SingleFile + CollectAll 跑 Driver（LSP 单文件兜底路径的形态，
/// 02 §3 入口表——pipeline 的 FailFast 契约不受影响）。
fn run_single_collect_all(source: &str) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("test", "test.yx", source)],
        CompileConfig::default(),
    )
    .with_aggregation(Aggregation::CollectAll);
    Driver::new().run(program)
}

#[test]
fn test_driver_run_single_file_collect_all_typechecks_partial_ast() {
    // Arrange（编辑器哲学在单文件兜底的同构，4.2.3 裁决延伸）：一处语法
    // 错误 + 一个幸存语句的类型错误——残缺 AST 继续 typecheck
    let source = "main: Int = \"nope\"\nlet = ;\n";

    // Act
    let outcome = run_single_collect_all(source).expect("driver run failed");

    // Assert
    let codes: Vec<String> = outcome
        .result
        .errors
        .iter()
        .filter_map(|e| e.diagnostic().map(|d| d.code))
        .collect();
    assert!(
        codes.contains(&"E0012".to_string()),
        "parse 错误必须收集: {codes:?}"
    );
    assert!(
        codes.contains(&"E1002".to_string()),
        "幸存语句必须继续 typecheck（编辑器哲学）: {codes:?}"
    );
}

// ===================== 4.2.7 collect_all 重复诊断去重 =====================

// ─────────────────────────────────────────────────────────────────────────────

/// check 形态下良构泛型项目必须**无多余错误诊断**：mono 消费合并 IR 只为
/// 特化，成功即静默；IR 也不因加阶段而变成 check 的对外产物。判别力：
/// mono 在 check 误报（E3018/E3005）时第一条断言红；请求表限定化缺口让
/// mono 空转时第二条断言红；IR 经 DriverOutcome.module 外发时第三条红。
#[test]
fn test_driver_run_check_generic_project_passes_without_extra_diagnostics() {
    // Arrange：项目根 + 跨单元泛型调用（lib.pick 的类型参数只在借用内层 &T）
    let (_dir, entry) = make_project(&[
        ("lib.yx", "pick: (T: Type) -> (x: &T) -> T = (x) => x\n"),
        (
            "main.yx",
            "use lib\nmain: () -> Void = {\n    v = lib.pick(7)\n}\n",
        ),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "良构泛型项目不得硬中止: {:?}",
        outcome.failure
    );
    let errors: Vec<String> = outcome
        .check_diagnostics
        .iter()
        .flat_map(|(_, ds)| {
            ds.iter()
                .filter(|d| matches!(d.severity, crate::util::diagnostic::Severity::Error))
                .map(|d| format!("{}: {}", d.code, d.message))
        })
        .collect();
    assert!(
        errors.is_empty(),
        "良构泛型项目在 check 形态不得有多余错误诊断: {errors:?}"
    );
    assert!(
        !outcome
            .skipped
            .iter()
            .any(|s| s.stage == Stage::Monomorphization),
        "有实例化请求时 mono 必须真跑（NoObligations 即空转）: {:?}",
        outcome.skipped
    );
    assert!(
        outcome.module.is_none(),
        "check 的 IR 只驻内存供合并与 mono 消费，不得外发为产物（3.4.8 ③）"
    );
}

/// mono 失败不得丢掉其余文件已收集的诊断（check 的收集语义保持）。判别力：
/// mono 若走 fail_with 硬失败，outcome.failure 变 Some（第一条断言红）；
/// 若收集通道在 mono 失败时丢弃已收条目，zzz 的 E1002 消失（第三条断言红）。
#[test]
fn test_driver_run_check_mono_failure_keeps_other_file_diagnostics() {
    // Arrange：项目根 + lib 病态泛型递归（mono 失败）+ zzz 类型错误（另一文件的诊断）
    let (_dir, entry) = make_project(&[
        (
            "lib.yx",
            "f: (T: Type) -> (x: T) -> T = (x) => {\n    return f([x])\n}\n",
        ),
        ("zzz.yx", "bad: Int = \"oops\"\n"),
        (
            "main.yx",
            "use lib\nuse zzz.{bad}\nmain: () -> Void = {\n    v = lib.f(1)\n    w = bad\n}\n",
        ),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "mono 失败在 check 形态不得转为硬中止: {:?}",
        outcome.failure
    );
    let main = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        main.iter().any(|d| d.code == "E3005"),
        "mono 的 E3005 必须进入口文件的收集条目: {main:?}"
    );
    let zzz = check_diags_of(&outcome, "zzz.yx").expect("entry for zzz.yx");
    assert!(
        zzz.iter().any(|d| d.code == "E1002"),
        "mono 失败不得丢掉其余文件的诊断: {zzz:?}"
    );
}

/// 无实例化请求时 mono 在 check 形态记 NoObligations 跳过——与 run 路径同
/// 口径（阶段记录不自相矛盾：没跑 = 明确记录原因，而不是静默缺失）。
/// 判别力：mono 未接线（阶段表缺它）或在 check 记成 UpstreamFailed/失败时红。
#[test]
fn test_driver_run_check_records_mono_skip_when_no_requests() {
    // Arrange：干净的非泛型项目
    let (_dir, entry) = make_project(&[("main.yx", "main: () -> Void = {}\n")]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "干净项目不得硬中止: {:?}",
        outcome.failure
    );
    assert!(
        matches!(
            skip_reason(&outcome, Stage::Monomorphization),
            Some(SkipReason::NoObligations)
        ),
        "无实例化请求时 mono 必须记 NoObligations（同 run 路径口径）: {:?}",
        outcome.skipped
    );
}
