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
fn test_driver_run_check_kind_hits_unwired_role_classification() {
    // Arrange: 4.2.1 后 MultiFile 全接线；Check 形态的 RoleClassification
    // 臂随 4.2.2 落地——跑到那里必须显式报错而非 panic/todo!
    let (_dir, entry) = make_project(&[("main.yx", "main: () -> Void = {}\n")]);
    let program = Program::new(
        ProgramKind::Check,
        vec![Unit::new("main", &entry, "")],
        CompileConfig::default(),
    );

    // Act
    let result = Driver::new().run(program);

    // Assert
    assert!(
        matches!(
            result,
            Err(DriverError::StageNotWired(Stage::RoleClassification))
        ),
        "Check must fail at its first unwired stage: {result:?}"
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
    // Arrange（4.10.1 怪癖钉板）：broken.yx 语法错误——现状多文件路径
    // parse 错误是 OrchestratorError::Parse 硬中止（`?` 传播），不是逐文件
    // 诊断；本测试钉住现状，4.10.1 修复时必须显式翻转
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
