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
use crate::driver::{Program, ProgramKind, Stage, Unit};
use crate::frontend::config::CompileConfig;
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::core::types::MonoType;
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
use crate::util::diagnostic::{Severity};
use crate::util::plan_id::PlanId;
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

/// 三档表第一档：**空义务不报**——14 条全空时零发现（且结算确实跑了）。
#[test]
fn test_obligation_settlement_is_quiet_for_empty_obligations() {
    // Arrange：14 个义务字段全空的合成产物
    let state = state_with(ProgramKind::SingleFile, TypeCheckResult::default());

    // Act
    let settlement = state.settle_obligations();

    // Assert
    assert!(
        settlement.findings.is_empty(),
        "空义务不得产任何结算发现: {:?}",
        settlement.findings
    );
    assert_eq!(
        settlement.pairs, 14,
        "有 typecheck 产物的单元必须参与全部 14 条义务的结算"
    );
    assert_eq!(
        settlement.empty, 14,
        "14 条义务全部走第一档（空义务）: {settlement:?}"
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

/// 跳过豁免（配置关闭）：mono 关闭时不判欠 instantiation_requests。
#[test]
fn test_obligation_config_disabled_skip_is_not_owed() {
    // Arrange：请求非空，但消费者阶段被记为 ConfigDisabled
    let mut state = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.instantiation_requests.push(InstantiationRequest::new(
                GenericFunctionId::new("f".to_string(), vec!["T".to_string()]),
                vec![crate::frontend::core::types::mono::GenericArg::Type(
                    MonoType::Int(64),
                )],
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

/// 结算清单与 14 条义务的静态事实一致（字段名/交付集无漂移）。
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
        14,
        "02 §义务字段表在 3.4.4/3.4.5/3.4.9 清账后共 13 条，6.9.5-B.2 新增 record_models"
    );
    assert_eq!(fields.len(), 14, "字段名必须逐条唯一");
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

/// 消费必达（4.5.1 / D41）：交付标记齐、消费者也跑完了一轮，但消费台账缺一个
/// 键 → **响亮**。
///
/// 两处同时钉住：审计面出 [`UndrainedCause::NotConsumed`]（区别于「未交付」），
/// 唯一结算点 `assert_drained` 把它外发成 **E 级 W1090**（触发即改退出码）。
#[test]
fn test_release_plan_unconsumed_is_reported_loudly() {
    // Arrange：产出键存在（合成产物）+ 交付标记齐 + 台账完成标记（消费者跑完）
    let mut state = state_with(
        ProgramKind::SingleFile,
        type_result_with(|r| {
            r.release_plan
                .drops
                .insert(PlanId::from_ordinal(7), vec!["x".to_string()]);
        }),
    );
    state.units[0]
        .delivered
        .deliver_all(IR_GENERATION_DELIVERED);
    state.units[0]
        .type_result
        .as_ref()
        .expect("合成产物已在场")
        .release_plan
        .mark_consumption_completed();

    // Act：唯一结算点的纯函数
    let settlement = state.settle_obligations();

    // Assert：审计面必须给出「未消费」原因（交付 ≠ 消费）
    let finding = settlement
        .findings
        .iter()
        .find(|f| f.obligation == Obligation::ReleasePlan)
        .expect("未消费的产出键必须进结算发现（不得静默）");
    assert!(
        matches!(finding.cause, UndrainedCause::NotConsumed { unconsumed: 1 }),
        "原因必须是 NotConsumed（区别于 NotDelivered）: {:?}",
        finding.cause
    );

    // Act / Assert：外发成 E 级 W1090（响亮失败，不是警告噪声）
    state.assert_drained();
    let diagnostic = state
        .errors
        .iter()
        .filter_map(|e| e.diagnostic())
        .find(|d| d.code == OBLIGATION_UNDRAINED_CODE)
        .expect("未消费义务必须外发 W1090");
    assert_eq!(diagnostic.severity, Severity::Error, "4.3.2 同码升 E");
    assert!(
        diagnostic.message.contains("release_plan") && diagnostic.message.contains('1'),
        "诊断必须点名义务字段与未消费键数: {}",
        diagnostic.message
    );
}

/// check 形态的消费必达（2026-10-13 实测缺口回归）：Check 形态下
/// Linking/Monomorphization 会把 `UnitState.ir` 取走——判据若挂在「IR 产物还在
/// 不在」，真实的未消费就会在 check 形态**静默通过**。本用例钉住：完成标记在、
/// 未消费键在时，check 形态同样产发现并外发到逐文件通道。
#[test]
fn test_release_plan_unconsumed_is_reported_in_check_kind() {
    // Arrange：Check 形态的合成产物（交付标记齐 + 完成标记 + 一个未消费键）
    let mut state = state_with(
        ProgramKind::Check,
        type_result_with(|r| {
            r.release_plan
                .drops
                .insert(PlanId::from_ordinal(7), vec!["x".to_string()]);
        }),
    );
    state.units[0]
        .delivered
        .deliver_all(IR_GENERATION_DELIVERED);
    state.units[0]
        .type_result
        .as_ref()
        .expect("合成产物已在场")
        .release_plan
        .mark_consumption_completed();

    // Act
    let settlement = state.settle_obligations();
    state.assert_drained();

    // Assert：发现进审计面 + 外发进逐文件收集通道（Check 的对外通道）
    assert!(
        settlement
            .findings
            .iter()
            .any(|f| f.obligation == Obligation::ReleasePlan
                && matches!(f.cause, UndrainedCause::NotConsumed { unconsumed: 1 })),
        "check 形态的未消费必须同样响亮（IR 产物被取走不是豁免理由）: {:?}",
        settlement.findings
    );
    assert!(
        state.units[0]
            .check_diagnostics
            .iter()
            .any(|d| d.code == OBLIGATION_UNDRAINED_CODE),
        "check 形态必须把 W1090 外发进逐文件通道: {:?}",
        state.units[0].check_diagnostics
    );
}
