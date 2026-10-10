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
use crate::frontend::pipeline::{Pipeline, PipelineError};
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
fn test_driver_run_lex_error_maps_to_lex_parse() {
    // Arrange: 含非法字符的源
    let source = "let x = @;";

    // Act
    let outcome = run_single(source, CompileConfig::default()).expect("driver run failed");

    // Assert: pipeline 现状——词法错误归入 PipelineError::LexParse
    assert!(!outcome.result.is_success(), "lex error must fail the run");
    assert_eq!(
        outcome.result.error_count, 1,
        "exactly one lex error expected: {:?}",
        outcome.result.errors
    );
    assert!(
        matches!(outcome.result.errors[0], PipelineError::LexParse(_)),
        "lex failure must map to LexParse: {:?}",
        outcome.result.errors
    );
    assert!(outcome.result.ir.is_none(), "failure yields no IR");
}

#[test]
fn test_driver_run_parse_error_reports_first_error_only() {
    // Arrange: 两处独立语法错误——pipeline.rs:248-255 现状只取首个 parse 错误
    let source = "let = ;\nlet = ;";

    // Act
    let outcome = run_single(source, CompileConfig::default()).expect("driver run failed");

    // Assert: 无论 parser 检出几处，对外只发首错（逐字节相同判据的一部分）
    assert_eq!(
        outcome.result.error_count, 1,
        "only the first parse error may surface: {:?}",
        outcome.result.errors
    );
    assert!(
        matches!(outcome.result.errors[0], PipelineError::LexParse(_)),
        "parse failure must map to LexParse: {:?}",
        outcome.result.errors
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
fn test_driver_run_dead_code_enabled_reports_unused_function() {
    // Arrange: 一个未被引用的顶层函数（main 是入口豁免）
    let source = "main: () -> Void = {}\nunused_helper: () -> Void = {}";

    // Act
    let outcome = run_single(source, CompileConfig::default()).expect("driver run failed");

    // Assert: 默认开启死代码分析——W1001 出现在警告里
    assert!(
        outcome.result.is_success(),
        "unused function is a warning, not an error: {:?}",
        outcome.result.errors
    );
    assert!(
        outcome.result.warnings.iter().any(|d| d.code == "W1001"),
        "W1001 expected for unused_helper: {:?}",
        outcome.result.warnings
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

#[test]
fn test_pipeline_run_delegates_to_driver() {
    // Arrange: 走 pub 入口 Pipeline——确认 shim 语义不变（编译成功 + 计时落字段）
    let mut pipeline = Pipeline::new(CompileConfig::default());

    // Act
    let result = pipeline.run("test.yx", "main: () -> Void = {}");

    // Assert
    assert!(
        result.is_success(),
        "pipeline shim must preserve success semantics: {:?}",
        result.errors
    );
    let _aggregation_witness: Aggregation = Aggregation::FailFast; // SingleFile 默认聚合
}

// ===================== 4.2.1 多文件形态（A1/B1/C3 裁决，2026-10-09）=====================

/// 在临时目录落一个多文件项目（yaoxiang.toml + 给定 .yx），返回 (dir 守卫, 入口路径)。
fn make_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\n",
    )
    .expect("write manifest failed");
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).expect("write fixture failed");
    }
    let entry = dir.path().join("main.yx");
    (dir, entry)
}

/// 以 MultiFile 形态跑 Driver（入口单元为占位——Discovery 臂负责展开）。
fn run_multi(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::MultiFile,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

#[test]
fn test_driver_run_multi_file_success_links_modules() {
    // Arrange: lib.yx 出一个函数，main.yx 经 use 导入并调用（夹具语法同
    // tests/yaoxiang-multifile/use-type-and-function）
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

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert: 无故障；Linking 产物含双方限定名函数
    assert!(
        outcome.failure.is_none(),
        "clean project must not fail: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("Linking must produce merged IR");
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"main.main"),
        "entry main must be linked as main.main: {names:?}"
    );
    assert!(
        names.contains(&"lib.add_one"),
        "lib function must be linked as lib.add_one: {names:?}"
    );
}

#[test]
fn test_driver_run_multi_file_typecheck_error_surfaces_file_contract() {
    // Arrange: lib.yx 顶层类型错误——compile_project 现状契约 =
    // OrchestratorError::TypeCheck{ path 指向出错文件, diagnostics 原样 }
    let (_dir, entry) = make_project(&[
        ("lib.yx", "bad: Int = \"oops\"\n"),
        ("main.yx", "use lib.{bad}\nmain: () -> Void = {}\n"),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
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
        path.ends_with("lib.yx"),
        "error must point at the failing file: {path}"
    );
    assert!(
        !diagnostics.is_empty(),
        "TypeCheck variant must carry the diagnostics"
    );
}

#[test]
fn test_driver_run_multi_file_missing_main_rejected() {
    // Arrange: Bin 角色（有 manifest）但无 main 绑定——#388 定案必须编译期拒绝
    let (_dir, entry) = make_project(&[("main.yx", "helper: () -> Void = {}\n")]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert: E3020 bin_missing_main
    let Some(OrchestratorError::TypeCheck { diagnostics, .. }) = &outcome.failure else {
        panic!(
            "expected OrchestratorError::TypeCheck, got {:?}",
            outcome.failure
        );
    };
    assert!(
        diagnostics.iter().any(|d| d.code == "E3020"),
        "missing main must yield E3020: {diagnostics:?}"
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

#[test]
fn test_driver_run_check_collects_per_file_diagnostics() {
    // Arrange: lib.yx 类型错误 + main.yx 干净——Check 契约 = 逐文件收集
    // 诊断而非硬中止（与 compile 路径 FailFast 对立的另一面）
    let (_dir, entry) = make_project(&[
        ("lib.yx", "bad: Int = \"oops\"\n"),
        ("main.yx", "use lib.{bad}\nmain = () => {\n    y = bad\n}\n"),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert: 无故障通道；每个发现文件都有条目（干净文件为空 Vec）
    assert!(
        outcome.failure.is_none(),
        "check 诊断收集不得走故障通道: {:?}",
        outcome.failure
    );
    let lib_diags = check_diags_of(&outcome, "lib.yx").expect("lib.yx must have a per-file entry");
    assert!(
        !lib_diags.is_empty(),
        "lib.yx 的类型错误必须进收集通道: {lib_diags:?}"
    );
    let main_diags =
        check_diags_of(&outcome, "main.yx").expect("main.yx must have a per-file entry");
    assert!(
        main_diags.is_empty(),
        "干净文件的条目应为空 Vec（check_project 现状契约）: {main_diags:?}"
    );
}

#[test]
fn test_driver_run_check_collect_all_continues_past_first_error() {
    // Arrange: aaa.yx 与 zzz.yx 各带一个类型错误——compile 路径在首个出错
    // 文件即中止（FailFast），check 必须两份都收集（CollectAll 语义）
    let (_dir, entry) = make_project(&[
        ("aaa.yx", "bad_a: Int = \"oops\"\n"),
        (
            "main.yx",
            "use aaa.{bad_a}\nuse zzz.{bad_z}\nmain = () => {\n    y = bad_a\n    z = bad_z\n}\n",
        ),
        ("zzz.yx", "bad_z: Int = \"nope\"\n"),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "CollectAll 不得因单文件错误硬中止: {:?}",
        outcome.failure
    );
    let aaa = check_diags_of(&outcome, "aaa.yx").expect("entry for aaa.yx");
    assert!(!aaa.is_empty(), "aaa.yx 的类型错误必须被收集: {aaa:?}");
    let zzz = check_diags_of(&outcome, "zzz.yx").expect("entry for zzz.yx");
    assert!(
        !zzz.is_empty(),
        "zzz.yx 的类型错误必须同样被收集（不停留在首错）: {zzz:?}"
    );
}

#[test]
fn test_driver_run_check_role_aware_dead_code_w1001() {
    // Arrange: Bin 角色（有 manifest + main 绑定）文件里的未使用顶层函数——
    // RFC-029f 角色感知死代码经 RoleClassification → DeadCodeAnalysis 链路
    // 产出（夹具同 cli_e2e test_e2e_check_bin_unused_fn_reports）
    let (_dir, entry) = make_project(&[(
        "main.yx",
        "dead_api = (x: Int) => x\nmain = () => { x = 1 }\n",
    )]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    let diags = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        diags.iter().any(|d| d.code == "W1001"),
        "Bin 角色未使用顶层函数应报 W1001: {diags:?}"
    );
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
fn test_driver_run_check_declared_bin_missing_main_reports_e3020() {
    // Arrange: #388 定案 check 半边判据（surfaces.bins 声明面）——
    // 声明为 bin 的文件缺 main 绑定报 E3020，进逐文件收集通道而非故障通道
    let (_dir, entry) = make_declared_bin_project();

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "E3020 是收集诊断而非硬中止: {:?}",
        outcome.failure
    );
    let diags = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    assert!(
        diags.iter().any(|d| d.code == "E3020"),
        "声明面 bin 缺 main 必须报 E3020: {diags:?}"
    );
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
fn test_driver_run_check_parse_error_does_not_mask_other_files() {
    // Arrange: broken.yx parse 失败与 zzz.yx 类型错误并存——rust 式收集
    // 语义（2026-10-09 裁决）：所有文件的错误一次性收齐，谁也不掩盖谁
    let (_dir, entry) = make_project(&[
        ("broken.yx", "let = ;\n"),
        (
            "main.yx",
            "use broken.{x}\nuse zzz.{bad}\nmain = () => {\n    y = bad\n}\n",
        ),
        ("zzz.yx", "bad: Int = \"nope\"\n"),
    ]);

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "CollectAll 不得因 parse 失败中止: {:?}",
        outcome.failure
    );
    let broken = check_diags_of(&outcome, "broken.yx").expect("entry for broken.yx");
    assert!(
        !broken.is_empty(),
        "broken.yx 的 parse 错误必须被收集: {broken:?}"
    );
    let zzz = check_diags_of(&outcome, "zzz.yx").expect("entry for zzz.yx");
    assert!(
        !zzz.is_empty(),
        "zzz.yx 的类型错误不得被 parse 失败掩盖: {zzz:?}"
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
fn test_driver_run_single_file_collect_all_collects_parse_errors() {
    // Arrange: 两处语法错误——CollectAll 语义应收全量（编辑器要标出所有
    // 语法错误）；pipeline 的 FailFast 首错形态不受影响
    let source = "x = @ @\ny = @ @\n"; // 探针实证：4 条 parse 错误

    // Act
    let outcome = run_single_collect_all(source).expect("driver run failed");

    // Assert
    assert!(
        outcome.result.errors.len() >= 2,
        "CollectAll 应收齐全部 parse 错误（非首错）: {:?}",
        outcome.result.errors
    );
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

#[test]
fn test_driver_run_check_dedupes_collect_all_diagnostics() {
    // Arrange: 带标注函数体内的类型错误——收集点把首错同时放进
    // collected_errors 与 Err 返回通道，4.2.5 基线实证此类 fixture
    // 的同码同 span 同消息诊断重复报告（64 个语料条目）
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    std::fs::write(
        dir.path().join("main.yx"),
        "main: () -> Void = {\n  x = 1 + \"a\"\n}\n",
    )
    .expect("write fixture failed");
    let entry = dir.path().join("main.yx");

    // Act
    let outcome = run_check(&entry).expect("driver run failed");

    // Assert: E1002 恰好一份（去重键 = 码 + span + 消息）
    let diags = check_diags_of(&outcome, "main.yx").expect("entry for main.yx");
    let count = diags.iter().filter(|d| d.code == "E1002").count();
    assert_eq!(
        count, 1,
        "同一诊断事实不得重复报告（4.2.7 去重）: {diags:?}"
    );
}
