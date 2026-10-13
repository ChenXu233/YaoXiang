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
    Obligation, UndrainedCause, IR_GENERATION_DELIVERED, OBLIGATION_UNDRAINED_CODE,
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

/// 三档表第一档：**空义务不报**——16 条全空时零发现（且结算确实跑了）。
#[test]
fn test_obligation_settlement_is_quiet_for_empty_obligations() {
    // Arrange：16 个义务字段全空的合成产物
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
        settlement.pairs, 16,
        "有 typecheck 产物的单元必须参与全部 16 条义务的结算"
    );
    assert_eq!(
        settlement.empty, 16,
        "16 条义务全部走第一档（空义务）: {settlement:?}"
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
        settlement
            .findings
            .iter()
            .all(|f| matches!(f.cause, UndrainedCause::RedList { .. })),
        "真实程序里除红名单外不得有未交付义务: {:?}",
        settlement.findings
    );
    assert!(
        settlement.pairs == 16 && settlement.empty > 0,
        "结算必须真的跑过（16 对、有空义务档）: {settlement:?}"
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
        settlement
            .findings
            .iter()
            .all(|f| matches!(f.cause, UndrainedCause::RedList { .. })),
        "摘掉 IrGeneration 交付标记必须让本例变红: {:?}",
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
        settlement
            .findings
            .iter()
            .all(|f| matches!(f.cause, UndrainedCause::RedList { .. })),
        "摘掉 mono 交付标记必须让本例变红: {:?}",
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
        settlement
            .findings
            .iter()
            .all(|f| matches!(f.cause, UndrainedCause::RedList { .. })),
        "摘掉 proof 交付标记必须让本例变红: {:?}",
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

/// 红名单（裁定丙 · 承担面 ①）：审计面必须带 WBS id 与零消费者证据，且**不外发**。
#[test]
fn test_obligation_red_list_is_in_audit_face_but_not_externalized() {
    // Arrange
    let outcome = run_single(SUM_TYPE_SOURCE);

    // Act
    let finding = outcome
        .obligations
        .findings
        .iter()
        .find(|f| f.obligation == Obligation::VariantCtorCalls);

    // Assert：审计面每条带清账任务 id 与证据指针
    let finding = finding.expect("variant_ctor_calls 非空必须在审计面报到（不静默）");
    match finding.cause {
        UndrainedCause::RedList { wbs, evidence } => {
            assert_eq!(wbs, "3.4.4", "清账任务 id 必须是 WBS 3.4.4");
            assert!(
                evidence.contains("detect_variant_ctor_call"),
                "证据指针必须指向 IR 侧形态检测替代: {evidence}"
            );
        }
        other => panic!("红名单字段的原因必须是 RedList: {other:?}"),
    }
    assert!(
        !outcome
            .result
            .warnings
            .iter()
            .any(|d| d.code == OBLIGATION_UNDRAINED_CODE),
        "红名单不进用户可见诊断流（裁定丙）: {:?}",
        outcome.result.warnings
    );
}

/// 结算诊断的**外发**（裁定丙 · 非红名单）：未交付义务必须进本形态诊断通道。
#[test]
fn test_obligation_undrained_is_externalized_as_w1090() {
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
        .warnings
        .iter()
        .find(|d| d.code == OBLIGATION_UNDRAINED_CODE)
        .expect("非红名单未交付义务必须外发 W1090");
    assert!(
        diagnostic.message.contains("escaped_refs"),
        "诊断必须点名义务字段: {}",
        diagnostic.message
    );
}

/// 结算清单与 16 条义务的静态事实一致（字段名/交付集无漂移）。
#[test]
fn test_obligation_inventory_is_complete_and_drift_free() {
    // Arrange
    let all: Vec<Obligation> = Obligation::ALL.to_vec();

    // Act
    let mut fields: Vec<&str> = all.iter().map(|o| o.field_name()).collect();
    fields.sort_unstable();
    fields.dedup();

    // Assert
    assert_eq!(all.len(), 16, "02 §义务字段表共 16 条义务");
    assert_eq!(fields.len(), 16, "字段名必须逐条唯一");
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

/// 红名单三条：清账任务 id 与「消费者为零」证据指针（2026-10-13 captain 裁定）。
#[test]
fn test_obligation_red_list_entries_carry_cleanup_ids_and_evidence() {
    // Arrange：红名单条目（字段 / 清账 WBS / 证据指针）
    let red: Vec<(Obligation, &str, &str)> = Obligation::ALL
        .iter()
        .filter_map(|o| o.red_list().map(|e| (*o, e.wbs, e.evidence)))
        .collect();

    // Act
    let mut ids: Vec<&str> = red.iter().map(|(_, wbs, _)| *wbs).collect();
    ids.sort_unstable();
    ids.dedup();

    // Assert
    assert_eq!(
        red.len(),
        3,
        "红名单恰好三条（variant_ctor_calls / method_overload_ir_names / interface_impl_registry）: {red:?}"
    );
    assert_eq!(
        ids,
        vec!["3.4.4", "3.4.5", "3.4.9"],
        "三条清账任务 id 必须与 09-WBS 登记一致"
    );
    for (obligation, _, evidence) in &red {
        assert!(
            evidence.contains("证据命令"),
            "{} 必须带可复核的「消费者为零」证据指针: {evidence}",
            obligation.field_name()
        );
    }
}

/// W1090 已注册、W 级、span 豁免（结算点没有用户可指的源码位置）。
#[test]
fn test_obligation_code_is_registered_as_span_exempt_warning() {
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
        Severity::Warning,
        "4.3.1 只外发 W 级（D5 观测期，升 E 归 4.3.2）"
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
    //         按文件名过滤——C2 判据的主语是「同源文件」）
    let run_keys = audit_keys_for_file(&run_outcome, "main.yx");
    assert!(
        !run_keys.is_empty(),
        "审计面必须非空（否则比对是空转）: {:?}",
        run_outcome.obligations
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
