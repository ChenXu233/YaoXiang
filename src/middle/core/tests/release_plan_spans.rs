//! WBS 2.4.4 / RFC-039 D41：ReleasePlan span 键「产出 ⊆ 消费」判据
//!
//! 规范来源:
//! - docs/src/dev/architecture/07-equivalence-oracle.md §修复正确性漏洞的专门判据
//!   （test_release_plan_spans_consumed：ownership 产出的 Span 集合 ⊆ IR 构造消费
//!   的 Span 集合；**无白名单，差集必须为空**——D41，空不了说明 ReleasePlan 契约
//!   有缺陷，修契约见 D20）
//! - docs/src/rfc/accepted/039-compiler-architecture.md D20（span 失配导致 Drop
//!   静默丢失，是最隐蔽的一类缺陷）/ D41
//!
//! 判据形态：全语料（tests/yaoxiang/ + src/std/tests/，剔 expect: compile-error
//! 与 // skip:——不到达 IR 生成的文件不在本合同适用范围）逐文件走
//! lex→parse→typecheck→ir_gen **真实生成序列**，对
//! `TypeCheckResult.release_plan.drops` 键集（产出）与生成器实际消费键集
//! （D41 观测点 `release_plan_consumed_spans`）求差集。
//!
//! 红态证据（2026-10-06 实测，`cargo test --lib release_plan_spans -- --ignored`）：
//! **168 个语料文件、373 个产出键未被消费**——ownership 按「最后使用**表达式**
//! span」记键（如 `assert.assert(true, "all passed")` 的字符串实参 span），
//! ir_gen 按「**语句** span」查表，表达式级键永不命中，Drop 静默丢失。
//! 逐案核实：tests/yaoxiang/99-demos/error_move_after_use.yx:14 的未消费键即
//! 字符串实参 `"all passed"` 的 span。分布：02-type-system 64、01-syntax 28、
//! 05-ownership 19、99-demos 13、04-concurrency 11、std/tests 10、03-semantics 10、
//! 06-compile-errors 7、03-modules 6（文件数）。
//!
//! 处置（D41）：契约缺陷成立，修契约见 D20——**归 P4**（Span 键 → PlanId，
//! 改变约 168 个文件的 IR 行为面，须等 C2 判据与语料差分就位）。本判据带
//! #[ignore] 常驻复现红态；P4/D20 落地时移除属性即转绿。
//!
//! 范围说明：多文件路径（orchestrator 逐文件 AstToIrGenerator）共用同一观测点，
//! 但生成器不跨编排边界暴露，本判据先覆盖单文件 pipeline 语料面；多文件面随
//! P4 统一 Driver 自然并入（D20 迁移时同一观测点复用）。

use std::collections::HashSet;
use std::path::Path;

use super::corpus::{corpus_files, reaches_ir_generation};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::util::span::Span;

/// 逐文件走真实生成序列，返回 (产出键集, 消费键集)。
///
/// 返回 None 的三类情形（合同不适用，非判据失败）：生成前置阶段（lex/parse/
/// typecheck/ir_gen）失败；文件无释放义务（产出键集为空——不构成本判据证据）。
fn produced_vs_consumed(path: &Path) -> Option<(HashSet<Span>, HashSet<Span>)> {
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
    let produced: HashSet<Span> = type_result.release_plan.drops.keys().cloned().collect();
    if produced.is_empty() {
        return None;
    }
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    generator.generate_module_ir(&parsed.module).ok()?;
    let consumed = generator.release_plan_consumed_spans().clone();
    Some((produced, consumed))
}

#[test]
#[ignore = "red-by-design（WBS 2.4.4/D41）：ReleasePlan span 键失配 373 处实测；修契约归 P4/D20（PlanId）"]
fn test_release_plan_spans_consumed() {
    // Arrange：语料发现（compile-error / skip 不到达 ir_gen，剔出合同面）
    let mut examined = 0usize;
    let mut skipped_no_obligation = 0usize;
    let mut violations: Vec<String> = Vec::new();

    // Act：逐文件求差集
    for file in corpus_files() {
        if !reaches_ir_generation(&file) {
            continue;
        }
        let relative = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&file)
            .display()
            .to_string();
        let Some((produced, consumed)) = produced_vs_consumed(&file) else {
            skipped_no_obligation += 1;
            continue;
        };
        examined += 1;
        let diff: Vec<&Span> = produced.difference(&consumed).collect();
        if !diff.is_empty() {
            violations.push(format!(
                "{relative}: {} 个产出键未被消费: {:?}",
                diff.len(),
                diff
            ));
        }
    }

    // Assert：D41——无白名单，差集必须为空
    assert!(
        examined > 0,
        "语料中应至少有一个含释放义务的文件构成本判据证据"
    );
    assert!(
        violations.is_empty(),
        "ReleasePlan 产出键未被 IR 构造消费（D41 差集必须为空；examined={examined}, \
         无义务跳过={skipped_no_obligation}）：\n{}",
        violations.join("\n")
    );
}
