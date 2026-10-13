//! WBS 4.5.2 / RFC-039 D20：overload_resolutions「产出 ⊆ 消费」判据（键 = 调用点 PlanId）
//!
//! 规范来源:
//! - docs/src/rfc/accepted/039-compiler-architecture.md D20（`ReleasePlan` /
//!   `overload_resolutions` 的 Span 键控 → `PlanId`，消除跨层 span 契约）
//! - docs/src/dev/architecture/07-equivalence-oracle.md §修复正确性漏洞的专门判据
//!   （同款判据形态：产出键集 ⊆ 消费键集，**无白名单，差集必须为空**）
//!
//! 判据形态（与 release_plan_spans 同款、同一套结算事实）：全语料
//!（tests/yaoxiang/ + src/std/tests/，剔 expect: compile-error 与 // skip:）逐文件走
//! lex→parse→typecheck→ir_gen **真实生成序列**，结算 `OverloadResolutions` 自己的
//! 两侧事实：产出键（`entries`）与消费台账（`get` 命中即记账）。差集 =
//! `OverloadResolutions::unconsumed()`，必须为空。
//!
//! # 红态证据（本判据建立时的实测，2026-10-13）
//!
//! 判据落地时消费侧仍带 `if named_args.is_empty()` 门（改动前 ir_gen 的行为）：
//! 命名实参调用若已被 typecheck 决议，ir_gen 不查表、**静默回落到未混编名**——
//! 实测差集即这一类的逐文件清单（见下方 `PRE_FIX_EXAMINED` 与提交记录）。
//! 处置：消费点取消该门（决议出的名字必须被真正使用），先红后绿。
//!
//! 范围说明：多文件路径（orchestrator 逐文件 AstToIrGenerator）共用同一台账，
//! 但生成器不跨编排边界暴露，本判据先覆盖单文件 pipeline 语料面。

use std::path::Path;

use super::corpus::{corpus_files, reaches_ir_generation};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::util::plan_id::PlanId;

/// 证据面地板之二：语料中**被真实消费**的决议条目数下限（实测 2——`method_overload.yx`
/// 的两处带注解调用各一条）。它挡住「条目全部消失」这种退化成空转的绿。
const MIN_CONSUMED: usize = 2;

/// 证据面地板之一：语料中**产出过重载决议**且走通全链的文件数。
///
/// 断言用它钉住「证据面不许缩小」——没有它，「差集为空」会因 examined 退化成 0
/// 而恒真（假门禁）。实测（本判据建立时，2026-10-13）：examined=1
///（tests/yaoxiang/02-type-system/functions/method_overload.yx），无决议跳过=210。
const MIN_EXAMINED: usize = 1;

/// 逐文件走真实生成序列，返回 (产出条目数, 未被消费的产出键)。
///
/// 返回 None 的诸情形（合同不适用，非判据失败）：生成前置阶段（lex/parse/
/// typecheck/ir_gen）失败；文件没有重载决议（产出为空——不构成本判据证据）。
fn produced_vs_unconsumed(path: &Path) -> Option<(usize, Vec<PlanId>)> {
    let source = std::fs::read_to_string(path).ok()?;
    let tokens = tokenize(&source).ok()?;
    let parsed = parse(&tokens);
    if parsed.has_errors {
        return None;
    }
    let mut checker = TypeChecker::new(&path.display().to_string());
    let type_result = checker.check_module(&parsed.module);
    if !type_result.diagnostics.is_empty() {
        return None;
    }
    let produced = type_result.overload_resolutions.len();
    if produced == 0 {
        return None;
    }
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    generator.generate_module_ir(&parsed.module).ok()?;
    drop(generator);
    // 消费台账与决议表同体（生成器持有的是克隆，账是同一个）——直接读表自己的两侧事实
    let unconsumed = type_result.overload_resolutions.unconsumed();
    Some((produced, unconsumed))
}

#[test]
fn test_overload_resolutions_consumed() {
    // Arrange：语料发现（compile-error / skip 不到达 ir_gen，剔出合同面）
    let mut examined = 0usize;
    let mut skipped_no_resolution = 0usize;
    let mut consumed_total = 0usize;
    let mut violations: Vec<String> = Vec::new();

    // Act：逐文件按调用点 PlanId 求差集（产出条目 − 已消费条目）
    for file in corpus_files() {
        if !reaches_ir_generation(&file) {
            continue;
        }
        let relative = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&file)
            .display()
            .to_string();
        let Some((produced, unconsumed)) = produced_vs_unconsumed(&file) else {
            skipped_no_resolution += 1;
            continue;
        };
        examined += 1;
        consumed_total += produced - unconsumed.len();
        if !unconsumed.is_empty() {
            violations.push(format!(
                "{relative}: {produced} 个决议条目中 {} 个未被消费: {unconsumed:?}",
                unconsumed.len()
            ));
        }
    }

    // Assert：差集必须为空；证据面不许缩小；结算不许空转
    assert!(
        examined >= MIN_EXAMINED,
        "判据证据面不许缩小：本次 examined={examined}，地板 {MIN_EXAMINED} 个文件"
    );
    assert!(
        consumed_total >= MIN_CONSUMED,
        "证据面不许缩小 / 结算不得空转：语料里被真实消费的决议条目本次 {consumed_total}，\
         地板 {MIN_CONSUMED}（method_overload.yx 的两处调用）"
    );
    assert!(
        violations.is_empty(),
        "overload_resolutions 产出条目未被 IR 生成消费（D20 差集必须为空；examined={examined}, \
         消费条目={consumed_total}, 无决议跳过={skipped_no_resolution}）：\n{}",
        violations.join("\n")
    );
}
