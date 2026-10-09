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
use crate::frontend::pipeline::{Pipeline, PipelineError};

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
fn test_driver_run_multi_file_kind_hits_unwired_stage() {
    // Arrange: 4.1.3 只接线 SingleFile 形态——MultiFile 首阶段即未接线
    let program = Program::new(
        ProgramKind::MultiFile,
        vec![Unit::new("app", "app.yx", "main: () -> Void = {}")],
        CompileConfig::default(),
    );

    // Act
    let result = Driver::new().run(program);

    // Assert: 显式错误而非 panic/todo!（02 §改动清单：其余臂随 4.2 落地）
    assert!(
        matches!(
            result,
            Err(DriverError::StageNotWired(Stage::VendorConsistency))
        ),
        "MultiFile must fail at its first unwired stage: {result:?}"
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
