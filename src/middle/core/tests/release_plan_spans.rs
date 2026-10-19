//! WBS 4.5.1 / RFC-039 D20 · D41：ReleasePlan「产出 ⊆ 消费」判据（键 = PlanId）
//!
//! 规范来源:
//! - docs/src/dev/architecture/07-equivalence-oracle.md §修复正确性漏洞的专门判据
//!   （test_release_plan_spans_consumed：ownership 产出的键集合 ⊆ IR 生成消费
//!   的键集合；**无白名单，差集必须为空**——D41）
//! - docs/src/rfc/accepted/039-compiler-architecture.md D20（span 失配导致 Drop
//!   静默丢失，是最隐蔽的一类缺陷）/ D41
//!
//! 判据形态（4.5.1 起按 PlanId 结算）：全语料（tests/yaoxiang/ +
//! src/std/tests/，剔 expect: compile-error 与 // skip:——不到达 IR 生成的文件
//! 不在本合同适用范围）逐文件走 lex→parse→typecheck→ir_gen **真实生成序列**，
//! 结算 `ReleasePlan` 自己的两侧事实：产出键（`drops`）与消费台账
//! （`consume` 记账，消费与读取是同一次调用）。差集 = `ReleasePlan::unconsumed()`。
//!
//! 测试名沿用 D41 契约 id（判据在 RFC 与 07 里以 `test_release_plan_spans_consumed`
//! 登记）；键已不是 span，而是**解析期分配、随 AST 节点走的 `PlanId`**。
//!
//! # 红态证据（先红后绿的「红」）
//!
//! 修前实测（2026-10-13，本判据带 `#[ignore]` 常驻，命令
//! `cargo test --lib release_plan_spans -- --ignored`）：
//! **examined=173（有释放义务且走通全链的语料文件）、未消费产出键 533 个、
//! 无义务跳过 38 个**——173 个文件**每一个**都有差集（差集非空文件数 = examined）。
//!（更早一次实测见 2026-10-06 记录：168 文件 / 373 键；语料增长后复现同一形态。）
//!
//! 根因（逐案核实）：ownership 按「最后使用**表达式**的 span」或「作用域退出
//! 时 current_span」记键，ir_gen 按「**语句** span」查表——两个跨层各自持有
//! 的位置量对不上，表达式级键永不命中（例：
//! tests/yaoxiang/99-demos/error_move_after_use.yx:14 的字符串实参 span）。
//! 另有尾语句路径（`generate_tail_stmt_ir` 提前返回）与嵌套块语句从不查表。
//!
//! # 处置（D20 已落地）
//!
//! 键改为 `PlanId`（解析期单调分配、编译单元内唯一、随 `Stmt` 走）；ownership 把
//! 释放点归属到释放边界所在的语句；ir_gen 在**每条语句边界**（函数体 / curry 层 /
//! 方法体 / lambda 体 / 嵌套块 / 尾语句路径）经 `ReleasePlan::consume` 取条目——
//! 取即记账。义务账本（4.3，唯一结算点）读同一份账，差集非空即报 W1090。
//!
//! 范围说明：多文件路径（orchestrator 逐文件 AstToIrGenerator）共用同一观测点，
//! 但生成器不跨编排边界暴露，本判据先覆盖单文件 pipeline 语料面；多文件面随
//! P4 统一 Driver 自然并入（D20 迁移时同一观测点复用）。

use std::path::Path;

use super::corpus::{corpus_files, reaches_ir_generation};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::util::plan_id::PlanId;

/// 修前实测的证据面（有释放义务且走通全链的语料文件数，见模块头「红态证据」：
/// 173 = 差集非空文件数 = examined）。断言用它钉住「证据面不许缩小」——
/// corpus 收缩必须显式改这里并说明原因。
const PRE_FIX_EXAMINED: usize = 173;

/// 逐文件走真实生成序列，返回 (产出键数, 未被消费的产出键)。
///
/// 返回 None 的三类情形（合同不适用，非判据失败）：生成前置阶段（lex/parse/
/// typecheck/ir_gen）失败；文件无释放义务（产出键集为空——不构成本判据证据）。
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
    let produced = type_result.release_plan.drops.len();
    if produced == 0 {
        return None;
    }
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    generator.generate_module_ir(&parsed.module).ok()?;
    drop(generator);
    // 消费台账与计划同体（生成器持有的是克隆，账是同一个）——直接读计划自己的两侧事实
    let unconsumed = type_result.release_plan.unconsumed();
    Some((produced, unconsumed))
}

/// 真实链路的直驱探针结果：产出键数 / 未消费键 / 计划里的释放变量 / IR 里的 Drop。
struct PlanProbe {
    produced: usize,
    unconsumed: Vec<PlanId>,
    released: Vec<String>,
    dropped_names: Vec<String>,
}

/// 源码 → 真实生成序列（lex → parse → typecheck → ir_gen）的直驱探针。
fn compile_plan_probe(source: &str) -> PlanProbe {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<plan-probe>");
    let type_result = checker.check_module(&parsed.module);
    assert!(
        type_result.diagnostics.is_empty(),
        "夹具必须通过类型检查: {:?}",
        type_result.diagnostics
    );
    let produced = type_result.release_plan.drops.len();
    let mut released: Vec<String> = type_result
        .release_plan
        .drops
        .values()
        .flatten()
        .cloned()
        .collect();
    released.sort();
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    let ir = generator
        .generate_module_ir(&parsed.module)
        .expect("夹具必须生成 IR");
    drop(generator);
    let unconsumed = type_result.release_plan.unconsumed();
    let dropped_names = dropped_local_names(&ir);
    PlanProbe {
        produced,
        unconsumed,
        released,
        dropped_names,
    }
}

/// IR 里 `Instruction::Drop` 指向的局部变量名（槽位表反查；无名槽位标 `<temp N>`）。
fn dropped_local_names(ir: &crate::middle::core::ir::ModuleIR) -> Vec<String> {
    use crate::middle::core::ir::{FunctionBody, Instruction, Operand};
    let mut names = Vec::new();
    for func in &ir.functions {
        let FunctionBody::Code { blocks, locals, .. } = &func.body else {
            continue;
        };
        for block in blocks {
            for inst in &block.instructions {
                if let Instruction::Drop {
                    src: Operand::Local(idx),
                    ..
                } = inst
                {
                    names.push(
                        locals
                            .get(*idx)
                            .and_then(|slot| slot.name.clone())
                            .unwrap_or_else(|| format!("<temp {idx}>")),
                    );
                }
            }
        }
    }
    names
}

/// 直驱（生产端 → 消费端同批交付）：真实链路走一遍——释放计划必须**全数被消费**，
/// 且消费真的产出 `Instruction::Drop` 指令（结算不得空转）。
#[test]
fn test_release_plan_consumption_is_direct_drive() {
    // Arrange：局部变量 + 嵌套块 + 尾表达式（三处语句边界都在场）
    let source = concat!(
        "f: (a: Int) -> Int = {\n",
        "    x = a + 1\n",
        "    if x > 0 {\n",
        "        y = x\n",
        "    }\n",
        "    x\n",
        "}\n",
        "main: () -> Void = {\n",
        "    z = f(1)\n",
        "}\n",
        "main()\n",
    );

    // Act
    let probe = compile_plan_probe(source);

    // Assert
    assert!(
        probe.produced > 0,
        "夹具必须产出释放计划条目（否则本用例无证据）: produced={}",
        probe.produced
    );
    assert!(
        probe.unconsumed.is_empty(),
        "产出键必须全部被 IR 生成消费（D41 差集为空）: {:?}",
        probe.unconsumed
    );
    assert!(
        !probe.dropped_names.is_empty(),
        "消费必须真的发射 Drop 指令（否则只是记账空转）: released={:?}",
        probe.released
    );
}

/// 嵌套块语句边界也消费（D20 的块内半边）：块内作用域退出的 Drop 必须**真的发射**
/// ——4.5.1 之前 `generate_block_ir` 从不查表，这类 Drop 静默丢失。
#[test]
fn test_release_plan_nested_block_scope_drop_is_emitted() {
    // Arrange：if 分支体内声明的 y——其释放边界在块内，不在函数体末尾
    let source = concat!(
        "f: (a: Int) -> Int = {\n",
        "    x = a + 1\n",
        "    if x > 0 {\n",
        "        y = x + 1\n",
        "    }\n",
        "    x\n",
        "}\n",
        "main: () -> Void = {\n",
        "    z = f(1)\n",
        "}\n",
        "main()\n",
    );

    // Act
    let probe = compile_plan_probe(source);

    // Assert
    assert!(
        probe.released.contains(&"y".to_string()),
        "块内声明的 y 必须在释放计划里（作用域退点归属块内最后一条语句）: released={:?}",
        probe.released
    );
    assert!(
        probe.unconsumed.is_empty(),
        "块内语句的计划条目也必须有消费点: {:?}",
        probe.unconsumed
    );
    assert!(
        probe.dropped_names.contains(&"y".to_string()),
        "块内作用域退出的 Drop 必须真的发射（4.5.1 前此路径从不查表）: dropped={:?}",
        probe.dropped_names
    );
}

#[test]
fn test_release_plan_spans_consumed() {
    // Arrange：语料发现（compile-error / skip 不到达 ir_gen，剔出合同面）
    let mut examined = 0usize;
    let mut skipped_no_obligation = 0usize;
    let mut consumed_total = 0usize;
    let mut violations: Vec<String> = Vec::new();

    // Act：逐文件按 PlanId 求差集（产出键 − 已消费键）
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
            skipped_no_obligation += 1;
            continue;
        };
        examined += 1;
        consumed_total += produced - unconsumed.len();
        if !unconsumed.is_empty() {
            violations.push(format!(
                "{relative}: {produced} 个产出键中 {} 个未被消费: {unconsumed:?}",
                unconsumed.len()
            ));
        }
    }

    // Assert：D41——无白名单，差集必须为空
    assert!(
        examined >= PRE_FIX_EXAMINED,
        "判据证据面不许缩小：本次 examined={examined}，修前实测 {PRE_FIX_EXAMINED} 个文件"
    );
    assert!(
        consumed_total > 0,
        "结算不得空转：语料中应至少有一个产出键被 IR 生成真实消费（consumed_total={consumed_total}）"
    );
    assert!(
        violations.is_empty(),
        "ReleasePlan 产出键未被 IR 生成消费（D41 差集必须为空；examined={examined}, \
         消费键={consumed_total}, 无义务跳过={skipped_no_obligation}）：\n{}",
        violations.join("\n")
    );
}
