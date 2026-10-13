//! 义务账本测试 —— 基于 02-stage-contract §2「义务账本：Obligations +
//! assert_drained()」/ RFC-039 D5 / WBS 4.3.1。
//!
//! 钉住的判据（逐条对应契约与用户裁决）：
//! - 三档结算表：空义务不报 / 有交付标记通过 / 非空且无标记必报；
//! - S1（2026-10-13 用户裁决）：消费者阶段不在本 ProgramKind 阶段表 → 不判欠；
//! - 跳过豁免：SkipReason::UpstreamFailed / ConfigDisabled 不报；
//! - 交付标记覆盖三个交付点：IrGeneration / ProofExecution / Monomorphization
//!   ——每处都有一条用例在标记被摘掉时变红（drained_kinds 提供非空证据）；
//! - 外发（裁定丙）：**非红名单**的未交付义务外发 W1090；红名单三字段
//!   只在审计面（DriverOutcome.obligations.findings）出现，不进用户可见诊断流，
//!   其「不静默」由审计面 / check-obligations.py 门禁 / 02 §2 注记三处承担；
//! - C2：同一源文件在 run（SingleFile）与 check（Check）下按键归一后一致。
//!
//! 红态证据（先红后绿，WBS 4.3.1 验收）：本文件先以 #[ignore] 骨架落库，并在
//! 「唯一结算点未接线」时运行留档（target/impl43-evidence/red-skeleton.txt）；
//! 转绿后另做三处「摘掉交付标记」的变异测试留档。

use crate::driver::obligations::{
    Obligation, UndrainedCause, IR_GENERATION_DELIVERED, NON_OBLIGATION_FIELDS,
    OBLIGATION_UNDRAINED_CODE,
};
use crate::driver::state::{SkipReason, State};
use crate::driver::{Driver, DriverOutcome, Program, ProgramKind, Stage, Unit};
use crate::frontend::config::CompileConfig;
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::core::types::MonoType;
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
use crate::util::diagnostic::{ErrorCodeDefinition, Severity};
use crate::util::span::Span;

/// 造一个带指定义务的合成 typecheck 产物（结构体字面量需要 typecheck 树内的
/// pub(super) proof_calls，树外只能走 Default + 赋值）。
fn type_result_with(build: impl FnOnce(&mut TypeCheckResult)) -> TypeCheckResult {
    let mut result = TypeCheckResult::default();
    build(&mut result);
    result
}

/// 造一个带合成产物的 State（不跑阶段表，直接钉结算纯函数）。
fn state_with(
    kind: ProgramKind,
    result: TypeCheckResult,
) -> State {
    let program = Program::new(
        kind,
        vec![Unit::new("test", "test.yx", "")],
        CompileConfig::default(),
    );
    let mut state = State::new(program);
    state.units[0].type_result = Some(result);
    state
}

/// 单文件形态跑 Driver（与 yaoxiang run 同一入口形态）。
fn run_single(source: &str) -> DriverOutcome {
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("test", "test.yx", source)],
        CompileConfig::default(),
    );
    Driver::new()
        .run(program)
        .expect("driver run must not fail structurally")
}

/// 和类型夹具：必须让 variant_ctor_calls / sum_types 等义务非空。
const SUM_TYPE_SOURCE: &str =
    "Color: Type = {\n    red: () -> Color,\n}\n\nmain: () -> Void = {\n    c = Color.red()\n}\n";

/// 泛型调用夹具：必须让 instantiation_requests 非空（Monomorphization 义务）。
const GENERIC_CALL_SOURCE: &str =
    "id: (T: Type) -> (x: T) -> T = (x) => x\n\nmain: () -> Void = {\n    y = id(1)\n}\n";

/// 证明义务夹具：可判定的证明函数（6 == 3 * 2 成立）——proof_calls 非空且执行通过。
const PROOF_CALL_SOURCE: &str = "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n\nf: () -> (r: SumUpTo(3, r)) = {\n    return 6\n}\n\nmain: () -> Void = {}\n";

/// 某形态下 W1090 的（归一化）诊断键集合：路径分隔符统一。
fn normalized_obligation_diagnostics(outcome: &DriverOutcome) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for d in outcome.result.warnings.iter().chain(
        outcome
            .check_diagnostics
            .iter()
            .flat_map(|(_, ds)| ds.iter()),
    ) {
        if d.code == OBLIGATION_UNDRAINED_CODE {
            keys.push(d.message.replace('\\', "/"));
        }
    }
    keys.sort();
    keys
}

/// 审计面里属于某文件的结算键（C2 的「同源文件」比对面）。
fn audit_keys_for_file(
    outcome: &DriverOutcome,
    file_name: &str,
) -> Vec<String> {
    let mut keys: Vec<String> = outcome
        .obligations
        .findings
        .iter()
        .filter(|f| f.path.file_name().is_some_and(|n| n == file_name))
        .map(|f| f.key())
        .collect();
    keys.sort();
    keys
}

/// 三档表第一档：**空义务不报**——13 条全空时零发现（且结算确实跑了）。
#[test]
fn test_obligation_settlement_is_quiet_for_empty_obligations() {
    // Arrange：13 个义务字段全空的合成产物
    let state = state_with(ProgramKind::SingleFile, TypeCheckResult::default());

    // Act
    let settlement = state.settle_obligations();

    // Assert
    assert!(
        settlement.is_drained(),
        "空义务不得产任何结算发现: {:?}",
        settlement.findings
    );
    assert_eq!(
        settlement.pairs, 13,
        "有 typecheck 产物的单元必须参与全部 13 条义务的结算"
    );
    assert_eq!(
        settlement.empty, 13,
        "13 条义务全部走第一档（空义务）: {settlement:?}"
    );
}

/// 端到端：能编译的程序里，除红名单外**不应有未交付**（其余义务要么空、要么已交付）。
#[test]
fn test_obligation_settlement_has_no_undelivered_in_real_program() {
    // Arrange：最小程序
    let outcome = run_single("main: () -> Void = {}");

    // Act
    let settlement = &outcome.obligations;

    // Assert：只有红名单条目可以出现——NotDelivered 意味着交付点没接线
    assert!(
        settlement.is_drained(),
        "红名单归零后，健康程序的结算必须零发现: {:?}",
        settlement.findings
    );
    assert!(
        settlement.pairs == 13 && settlement.empty > 0,
        "结算必须真的跑过（13 对、有空义务档）: {settlement:?}"
    );
}

/// 交付标记（IrGeneration 交付点）：和类型的 sum_types 等义务必须被排空。
#[test]
fn test_obligation_delivery_mark_drains_ir_generation_obligations() {
    // Arrange：和类型程序——sum_types 是非红名单的 IrGeneration 义务
    let outcome = run_single(SUM_TYPE_SOURCE);

    // Act
    let settlement = &outcome.obligations;

    // Assert：证据 1 = 该义务真的非空且经标记通过（否则本例空转）；
    //        证据 2 = 除红名单外零未交付（摘掉该交付标记即在此变红）
    assert!(
        outcome.result.is_success(),
        "夹具必须编译成功: {:?}",
        outcome.result.errors
    );
    assert!(
        settlement.drained_kinds.contains(&Obligation::SumTypes),
        "和类型程序的 sum_types 必须经 IrGeneration 交付点通过: {settlement:?}"
    );
    assert!(
        settlement.is_drained(),
        "摘掉 IrGeneration 交付标记必须让本例变红（健康态应零发现）: {:?}",
        settlement.findings
    );
}

/// 交付标记（Monomorphization 交付点）：实例化请求必须被排空。
#[test]
fn test_obligation_delivery_mark_drains_instantiation_requests() {
    // Arrange：泛型调用程序
    let outcome = run_single(GENERIC_CALL_SOURCE);

    // Act
    let settlement = &outcome.obligations;

    // Assert
    assert!(
        outcome.result.is_success(),
        "夹具必须编译成功: {:?}",
        outcome.result.errors
    );
    assert!(
        settlement
            .drained_kinds
            .contains(&Obligation::InstantiationRequests),
        "泛型调用的实例化请求必须经 Monomorphization 交付点通过: {settlement:?}"
    );
    assert!(
        settlement.is_drained(),
        "摘掉 mono 交付标记必须让本例变红（健康态应零发现）: {:?}",
        settlement.findings
    );
}

/// 交付标记（ProofExecution 交付点）：证明义务必须被排空。
#[test]
fn test_obligation_delivery_mark_drains_proof_calls() {
    // Arrange：可判定的证明函数
    let outcome = run_single(PROOF_CALL_SOURCE);

    // Act
    let settlement = &outcome.obligations;

    // Assert
    assert!(
        outcome.result.is_success(),
        "可判定的证明必须编译成功: {:?}",
        outcome.result.errors
    );
    assert!(
        settlement.drained_kinds.contains(&Obligation::ProofCalls),
        "证明义务必须经 ProofExecution 交付点通过（真值 6 == 3 * 2）: {settlement:?}"
    );
    assert!(
        settlement.is_drained(),
        "摘掉 proof 交付标记必须让本例变红（健康态应零发现）: {:?}",
        settlement.findings
    );
}

/// 三档表第三档（机制级）：义务非空、消费者阶段已运行、**没有交付标记** → 必报。
#[test]
fn test_obligation_without_delivery_mark_is_reported() {
    // Arrange：合成产物含 escaped_refs（IrGeneration 义务），不标记
    let state = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.escaped_refs.insert("v".to_string());
        }),
    );

    // Act
    let settlement = state.settle_obligations();

    // Assert
    let finding = settlement
        .findings
        .iter()
        .find(|f| f.obligation == Obligation::EscapedRefs)
        .expect("非空且未交付的 escaped_refs 必须有一条结算发现");
    assert!(
        matches!(finding.cause, UndrainedCause::NotDelivered),
        "无标记的未交付原因是 NotDelivered: {:?}",
        finding.cause
    );
}

/// 三档表第二档（机制级）：同一条义务打上交付标记后 → 通过。
#[test]
fn test_obligation_with_delivery_mark_is_drained() {
    // Arrange：与上一条同一产物，区别只在交付点打了标记
    let mut state = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.escaped_refs.insert("v".to_string());
        }),
    );
    state.units[0].delivered.deliver(Obligation::EscapedRefs);

    // Act
    let settlement = state.settle_obligations();

    // Assert
    assert!(
        !settlement
            .findings
            .iter()
            .any(|f| f.obligation == Obligation::EscapedRefs),
        "打了交付标记的义务不得再报: {:?}",
        settlement.findings
    );
    assert!(
        settlement.drained_kinds.contains(&Obligation::EscapedRefs),
        "排空种类里必须留下该义务: {settlement:?}"
    );
}

/// S1（用户裁决）：消费者阶段不在本 ProgramKind 阶段表 → 不判欠。
#[test]
fn test_obligation_consumer_stage_absent_is_not_owed() {
    // Arrange：Lsp 阶段表没有 IrGeneration（program.rs:112-119）
    let state = state_with(
        ProgramKind::Lsp,
        type_result_with(|r| {
            r.escaped_refs.insert("v".to_string());
        }),
    );

    // Act
    let settlement = state.settle_obligations();

    // Assert
    assert!(
        !settlement
            .findings
            .iter()
            .any(|f| f.obligation == Obligation::EscapedRefs),
        "Lsp 无 IrGeneration 阶段——该义务不判欠: {:?}",
        settlement.findings
    );
    assert!(
        settlement.consumer_stage_absent > 0,
        "S1 豁免计数必须非零: {settlement:?}"
    );
}

/// 跳过豁免（配置关闭）：mono 关闭时不判欠 instantiation_requests。
#[test]
fn test_obligation_config_disabled_skip_is_not_owed() {
    // Arrange：请求非空，但消费者阶段被记为 ConfigDisabled
    let mut state = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.instantiation_requests.push(InstantiationRequest::new(
                GenericFunctionId::new("f".to_string(), vec!["T".to_string()]),
                vec![MonoType::Int(64)],
                Span::default(),
            ));
        }),
    );
    state.record_skipped(Stage::Monomorphization, SkipReason::ConfigDisabled);

    // Act
    let settlement = state.settle_obligations();

    // Assert
    assert!(
        !settlement
            .findings
            .iter()
            .any(|f| f.obligation == Obligation::InstantiationRequests),
        "配置关闭导致的跳过不是丢弃: {:?}",
        settlement.findings
    );
    assert!(
        settlement.consumer_stage_skipped > 0,
        "跳过豁免计数必须非零: {settlement:?}"
    );
}

/// 跳过豁免（上游失败）：IrGeneration 因上游失败未执行时不判欠。
#[test]
fn test_obligation_upstream_failed_skip_is_not_owed() {
    // Arrange
    let mut state = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.escaped_refs.insert("v".to_string());
        }),
    );
    state.record_skipped(
        Stage::IrGeneration,
        SkipReason::UpstreamFailed(Stage::Parsing),
    );

    // Act
    let settlement = state.settle_obligations();

    // Assert
    assert!(
        !settlement
            .findings
            .iter()
            .any(|f| f.obligation == Obligation::EscapedRefs),
        "上游失败导致的跳过不是丢弃: {:?}",
        settlement.findings
    );
}

/// 守恒式：pairs = 空 + S1 豁免 + 跳过豁免 + 已交付 + 发现数（结算不丢账）。
#[test]
fn test_obligation_settlement_counters_are_conserved() {
    // Arrange
    let outcome = run_single(PROOF_CALL_SOURCE);

    // Act
    let s = &outcome.obligations;

    // Assert
    assert_eq!(
        s.pairs,
        s.empty
            + s.consumer_stage_absent
            + s.consumer_stage_skipped
            + s.delivered
            + s.findings.len(),
        "结算计数必须守恒（否则有义务既没通过也没报）: {s:?}"
    );
}

/// 红名单归零（3.4.9 清账收口）：健康程序的审计面零发现、且不外发任何 W1090。
///
/// 红名单为空后 `UndrainedCause::RedList` 无活体入口——本用例钉住「没有任何
/// 字段被当作零消费者登记」这一事实。红名单**机制**仍在（门禁判据 4 的陈旧
/// 检测与 `--selftest` 的 stale-red-list 夹具继续覆盖其检测面）。
#[test]
fn test_obligation_red_list_is_empty_and_audit_face_is_silent() {
    // Arrange
    let outcome = run_single(SUM_TYPE_SOURCE);

    // Act
    let findings = &outcome.obligations.findings;
    let externalized = outcome
        .result
        .warnings
        .iter()
        .any(|d| d.code == OBLIGATION_UNDRAINED_CODE);

    // Assert
    assert!(
        findings.is_empty(),
        "红名单归零后健康程序的审计面必须零发现: {findings:?}"
    );
    assert!(
        !externalized,
        "健康程序不得外发 W1090: {:?}",
        outcome.result.warnings
    );
}

/// 结算诊断的**外发**（4.3.2 同码升 E）：未交付义务进**错误通道**、触发即失败。
#[test]
fn test_obligation_undrained_is_externalized_as_error() {
    // Arrange：非红名单义务非空、交付标记缺席（等价于交付点被摘掉）
    let mut state = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.escaped_refs.insert("v".to_string());
        }),
    );

    // Act
    state.assert_drained();

    // Assert
    assert_eq!(
        state.obligations.emitted, 1,
        "恰好一条未交付义务应外发: {:?}",
        state.obligations
    );
    let diagnostic = state
        .errors
        .iter()
        .filter_map(|e| e.diagnostic())
        .find(|d| d.code == OBLIGATION_UNDRAINED_CODE)
        .expect("未交付义务必须进错误通道（E 级）");
    assert!(
        diagnostic.message.contains("escaped_refs"),
        "诊断必须点名义务字段: {}",
        diagnostic.message
    );
    assert_eq!(
        diagnostic.severity,
        Severity::Error,
        "4.3.2 同码升 E：发射级别必须是 error"
    );
    assert!(
        state.warnings.is_empty(),
        "E 级不得落进警告通道（否则不计入 error_count、退出码不为非零）: {:?}",
        state.warnings
    );
}

/// 同码升 E 的多文件形态：未交付义务另记 OrchestratorError 故障（触发即编译失败）。
#[test]
fn test_obligation_undrained_fails_orchestrated_compilation() {
    // Arrange：MultiFile 形态（非 pipeline 通道）的合成产物 + 交付标记缺席
    let mut state = state_with(
        ProgramKind::MultiFile,
        type_result_with(|r| {
            r.escaped_refs.insert("v".to_string());
        }),
    );

    // Act
    state.assert_drained();

    // Assert：错误通道 + failure 双落位（compile_project 契约读 failure）
    assert!(
        state
            .errors
            .iter()
            .filter_map(|e| e.diagnostic())
            .any(|d| d.code == OBLIGATION_UNDRAINED_CODE && d.severity == Severity::Error),
        "多文件形态也必须进错误通道: {:?}",
        state.errors
    );
    assert!(
        state.failure.is_some(),
        "多文件形态必须记 OrchestratorError 故障（否则 result.errors 无人消费）: {:?}",
        state.obligations
    );
}

/// C2（机制级 · 防空转）：同一条未交付义务在 run 与 check 形态下产出的键必须一致。
///
/// 端到端侧健康程序两侧都静默（见上一条用例）——本用例用合成 State 造出
/// 「非空且未交付」的义务，证明两侧都**会**报，且报的键逐字相同。
#[test]
fn test_obligation_settlement_keys_agree_across_run_and_check_kinds() {
    // Arrange：同一份合成产物（escaped_refs 非空、交付标记缺席），两种形态各一份
    let make = |kind: ProgramKind| {
        state_with(
            kind,
            type_result_with(|r| {
                r.escaped_refs.insert("v".to_string());
            }),
        )
        .settle_obligations()
    };

    // Act
    let run_settlement = make(ProgramKind::SingleFile);
    let check_settlement = make(ProgramKind::Check);

    // Assert：两侧都非空（否则比对是空转），且键集逐字一致
    assert!(
        !run_settlement.keys().is_empty() && !check_settlement.keys().is_empty(),
        "同一未交付义务在两种形态下都必须产出发现: run={run_settlement:?} check={check_settlement:?}"
    );
    assert_eq!(
        run_settlement.keys(),
        check_settlement.keys(),
        "run 与 check 形态对同一义务的结算键必须一致（C2）"
    );
}

/// 结算清单与 13 条义务的静态事实一致（字段名/交付集无漂移）。
#[test]
fn test_obligation_inventory_is_complete_and_drift_free() {
    // Arrange
    let all: Vec<Obligation> = Obligation::ALL.to_vec();

    // Act
    let mut fields: Vec<&str> = all.iter().map(|o| o.field_name()).collect();
    fields.sort_unstable();
    fields.dedup();

    // Assert
    assert_eq!(
        all.len(),
        13,
        "02 §义务字段表在 3.4.4/3.4.5/3.4.9 清账后共 13 条义务"
    );
    assert_eq!(fields.len(), 13, "字段名必须逐条唯一");
    let ir_fields: Vec<Obligation> = all
        .iter()
        .copied()
        .filter(|o| o.consumer_stage() == Stage::IrGeneration && o.red_list().is_none())
        .collect();
    assert_eq!(
        IR_GENERATION_DELIVERED,
        ir_fields.as_slice(),
        "IrGeneration 交付集必须等于该阶段的非红名单义务集（防两端漂移）"
    );
}

/// 红名单为空（3.4.4 / 3.4.5 / 3.4.9 三条全部清账收口）。
///
/// 一条登记（哪怕只是占位）都必须是一次**显式裁决**——它会要求连带更新本用例
/// 与门禁判据 4；因此「悄悄加回一条」在本用例下必然红。
#[test]
fn test_obligation_red_list_is_empty_after_cleanup() {
    // Arrange：红名单条目（字段 / 清账 WBS / 证据指针）
    let red: Vec<(Obligation, &str, &str)> = Obligation::ALL
        .iter()
        .filter_map(|o| o.red_list().map(|e| (*o, e.wbs, e.evidence)))
        .collect();

    // Act
    let red_fields: Vec<&str> = red.iter().map(|(o, _, _)| o.field_name()).collect();

    // Assert
    assert!(
        red_fields.is_empty(),
        "3.4.4/3.4.5/3.4.9 清账后红名单必须为空（重新登记须走显式裁决）: {red_fields:?}"
    );
}

/// 非义务字段登记：与义务清单互补、理由写实（门禁判据 1 的第三处）。
#[test]
fn test_non_obligation_registry_is_wellformed() {
    // Arrange
    let obligation_fields: Vec<&str> = Obligation::ALL.iter().map(|o| o.field_name()).collect();
    let entries = NON_OBLIGATION_FIELDS;

    // Act
    let mut names: Vec<&str> = entries.iter().map(|e| e.field).collect();
    names.sort_unstable();
    let before_dedup = names.len();
    names.dedup();

    // Assert
    assert!(
        !entries.is_empty(),
        "非义务登记不得为空（8 条在其中，含 3.4.9 改判的 interface_impl_registry）"
    );
    assert_eq!(
        names.len(),
        before_dedup,
        "非义务登记不得有重复字段: {names:?}"
    );
    for entry in entries {
        assert!(
            !obligation_fields.contains(&entry.field),
            "{} 不能同时是义务与非义务（两表必须互补）",
            entry.field
        );
        assert!(
            entry.reason.len() >= 12,
            "{} 的理由必须写实测消费面（不少于 12 字）: {:?}",
            entry.field,
            entry.reason
        );
    }
}

/// W1090 已注册、span 豁免、**发射级别为 error**（4.3.2 同码升 E 的终态）。
#[test]
fn test_obligation_code_is_span_exempt_and_emits_error_severity() {
    // Arrange
    let definition = ErrorCodeDefinition::find(OBLIGATION_UNDRAINED_CODE);
    let finding = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.escaped_refs.insert("v".to_string());
        }),
    )
    .settle_obligations()
    .findings
    .into_iter()
    .next()
    .expect("未交付义务必须产出一条发现");

    // Act
    let definition = definition.expect("W1090 必须在注册表里（codes/w1xxx.rs）");

    // Assert
    assert!(
        definition.span_exempt,
        "义务结算无用户可指的位置——必须登记 span_exempt（不许塞 dummy span）"
    );
    assert_eq!(
        finding.to_diagnostic().severity,
        Severity::Error,
        "4.3.2 同码升 E（显式 .severity(Error)，码号仍是 W1090）——D5 终态"
    );
    assert_eq!(
        finding.to_diagnostic().code,
        OBLIGATION_UNDRAINED_CODE,
        "升 E 不改码号（不新增 E 码：同一事实两个码即平行表示）"
    );
}

/// C2：同源文件在 run（SingleFile）与 check（Check）下按键归一后一致。
#[test]
fn test_obligation_diagnostics_agree_across_run_and_check() {
    // Arrange：同一份磁盘源文件，两个入口形态各跑一次
    let dir = tempfile::tempdir().expect("tempdir creation failed");
    let entry = dir.path().join("main.yx");
    std::fs::write(&entry, SUM_TYPE_SOURCE).expect("write fixture failed");
    let run_program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("main", &entry, SUM_TYPE_SOURCE)],
        CompileConfig::default(),
    );
    let check_program = Program::new(
        ProgramKind::Check,
        vec![Unit::new("main", &entry, String::new())],
        CompileConfig::default(),
    );

    // Act
    let run_outcome = Driver::new()
        .run(run_program)
        .expect("run entry must not fail structurally");
    let check_outcome = Driver::new()
        .run(check_program)
        .expect("check entry must not fail structurally");

    // Assert：① 审计面按键归一后一致（同源文件；check 会额外带上 <std/...> 单元，
    //         按文件名过滤——C2 判据的主语是「同源文件」）。
    //         红名单归零后健康程序两侧都**零发现**：这不是空转——下面同时断言
    //         两侧结算都真的跑过（pairs ≥ 义务条数），即「静默是算出来的静默」。
    let run_keys = audit_keys_for_file(&run_outcome, "main.yx");
    assert!(
        run_outcome.obligations.pairs >= 13 && check_outcome.obligations.pairs >= 13,
        "两侧结算都必须真的跑过（pairs ≥ 13）: run={:?} check={:?}",
        run_outcome.obligations,
        check_outcome.obligations
    );
    assert_eq!(
        run_keys,
        audit_keys_for_file(&check_outcome, "main.yx"),
        "run 与 check 对同一源文件的义务结算键必须一致（C2）"
    );
    // ② 外发诊断集同样一致（本夹具下两侧都为空——红名单不外发）
    assert_eq!(
        normalized_obligation_diagnostics(&run_outcome),
        normalized_obligation_diagnostics(&check_outcome),
        "run 与 check 的义务外发诊断集必须一致（C2）"
    );
}
