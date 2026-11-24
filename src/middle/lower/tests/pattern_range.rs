//! 范围模式 IR 判定链测试 — issue #405，SPEC syntax §2.8「范围模式」
//!
//! 判据（条款门）：SPEC syntax §2.8 定案模式位 `lo..hi` 为**双闭区间判定**
//! `lo <= x <= hi`（Int 域）。本测试钉住它的 IR 落地形态：`Ge(reg, lo)` +
//! `Le(reg, hi)` 双比较 + 各自 `JmpIfNot` 失败跳下一臂——与字面量臂的
//! `Eq` 判定链同构的「内联测试链」（RFC-010b 臂语义：失败落下一臂）。
//! 换一个不发射这两条比较（或只发射一端）的实现，本测试必红。
//!
//! 落位说明（test-specification 规则 1.1）：`compile_pattern`（`lower/expr/`
//! `control.rs`）私有于生成器，测试走其唯一公共驱动面——完整管线
//! parse → typecheck → `AstToIrGenerator::generate_module_ir`（与 IR 快照
//! `src/middle/ir/tests/snapshot.rs::normalize_corpus_file` 同一条生成序列）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::{SemanticProbe, TypeLevelNamespace};
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;

/// 跑完整管线生成模块 IR（script 模式顶层绑定——顶层语句编译进 `init` 序列，
/// ir.rs `ModuleIR.init`：「Script 模式下它本身就是程序」）。
fn generate_ir(source: &str) -> Vec<Instruction> {
    let tokens = tokenize(source).expect("tokenize failed");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(!parsed.has_errors, "parse failed: {:?}", parsed.errors);
    let mut checker = TypeChecker::new("pattern_range_test");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "typecheck failed: {:?}",
        type_result.diagnostics
    );
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    let module_ir = generator
        .generate_module_ir(&parsed.module)
        .expect("IR generation failed");
    module_ir.init
}

/// 指令序列中是否存在「Load(Int(lo)) → Ge → JmpIfNot → Load(Int(hi)) → Le →
/// JmpIfNot」的连续窗口（范围模式判定链的本体）。
fn contains_range_chain(
    instructions: &[Instruction],
    lo: i128,
    hi: i128,
) -> bool {
    let windows = instructions.windows(6);
    for w in windows {
        let [Instruction::Load {
            src: Operand::Const(ConstValue::Int(lo_const)),
            ..
        }, Instruction::Ge { .. }, Instruction::JmpIfNot { .. }, Instruction::Load {
            src: Operand::Const(ConstValue::Int(hi_const)),
            ..
        }, Instruction::Le { .. }, Instruction::JmpIfNot { .. }] = w
        else {
            continue;
        };
        if *lo_const == lo && *hi_const == hi {
            return true;
        }
    }
    false
}

#[test]
fn test_range_pattern_compiles_to_closed_ge_le_chain() {
    // Arrange — issue #405 夹具形态：Int scrutinee + 范围臂 + 字面量臂 + 通配兜底
    let source = r#"
grade = match 85 {
    80..89 => 1,
    90 => 2,
    _ => 0,
}
print(grade)
"#;

    // Act
    let instructions = generate_ir(source);

    // Assert — 双闭判定链两端的端点常量与指令形态逐一在位
    assert!(
        contains_range_chain(&instructions, 80, 89),
        "range arm should compile to Load(80)+Ge+JmpIfNot+Load(89)+Le+JmpIfNot, got:\n{:?}",
        instructions
    );
    // 字面量/通配臂的既有形态不受影响：Eq 判定链仍在（兜底臂是绑定不发射比较）
    assert!(
        instructions
            .iter()
            .any(|i| matches!(i, Instruction::Eq { .. })),
        "fallback-free literal chain should keep its Eq test, got:\n{:?}",
        instructions
    );
}

#[test]
fn test_range_pattern_negative_bounds_compile_to_negative_constants() {
    // Arrange — 负端点（parser 归一化为负 Int 常量）
    let source = r#"
r = match -5 {
    -5..-1 => 1,
    _ => 0,
}
print(r)
"#;

    // Act
    let instructions = generate_ir(source);

    // Assert — 负端点以负 Int 常量进入 Ge/Le 链
    assert!(
        contains_range_chain(&instructions, -5, -1),
        "negative range arm should compile to Load(-5)+Ge+JmpIfNot+Load(-1)+Le+JmpIfNot, got:\n{:?}",
        instructions
    );
}
