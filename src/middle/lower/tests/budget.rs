//! 7b.4 go/no-go 直驱判据 — 基于 09a §P7 7b.4 契约 §二 B 与用户裁决 ②
//!
//! 两条越界语料（`src/std/tests/string_ops.yx` 的 354 槽、`tests/yaoxiang/
//! 01-syntax/control-flow/list_comp.yx` 的 323 槽，都是**不回收时的单调槽数**）
//! 在 7b.4 之后必须靠活跃区间复用把 `frame_slots = 具名基址 + 同时活跃值`
//! 压回 ≤255，否则实现轮 go/no-go 失败、不得改语料或加 spill。
//!
//! 判据（证明门）：分配器换一个正确实现（如更精细的图着色）也必须满足
//! `locals.len() ≤ 255`；换回「每值独占一槽」的实现必红（帧槽会随值总数涨回去）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{FunctionBody, ModuleIR};
use crate::middle::lower::AstToIrGenerator;

/// 走 lex→parse→typecheck→lower 真实生成序列，返回 ModuleIR。
fn lower_source(
    file_name: &str,
    source: &str,
) -> ModuleIR {
    let tokens = tokenize(source).expect("语料必须能 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "{file_name}: 语料必须无解析错误：{:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new(file_name);
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "{file_name}: 语料必须无类型错误：{:?}",
        type_result.diagnostics
    );
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .expect("语料必须能生成 IR")
}

/// 一个语料的帧槽画像：最大帧槽 + 是否出现「值槽复用」的函数。
struct FrameProfile {
    max_locals: usize,
    total_values: usize,
    total_named: usize,
    reuse_functions: usize,
}

fn frame_profile(module: &ModuleIR) -> FrameProfile {
    let mut profile = FrameProfile {
        max_locals: 0,
        total_values: 0,
        total_named: 0,
        reuse_functions: 0,
    };
    for function in &module.functions {
        let FunctionBody::Code { locals, values, .. } = &function.body else {
            continue;
        };
        let named = locals.iter().filter(|slot| slot.name.is_some()).count();
        profile.max_locals = profile.max_locals.max(locals.len());
        profile.total_values += values.len();
        profile.total_named += named;
        if named + values.len() > locals.len() {
            profile.reuse_functions += 1;
        }
    }
    profile
}

/// 两条越界语料的 go/no-go：任何函数体的帧槽数必须 ≤255，且确实发生复用。
#[test]
fn test_corpus_over_budget_forms_fit_after_allocation() {
    // Arrange
    let cases = [
        (
            "src/std/tests/string_ops.yx",
            include_str!("../../../../src/std/tests/string_ops.yx"),
        ),
        (
            "tests/yaoxiang/01-syntax/control-flow/list_comp.yx",
            include_str!("../../../../tests/yaoxiang/01-syntax/control-flow/list_comp.yx"),
        ),
    ];

    // Act & Assert
    for (name, source) in cases {
        let module = lower_source(name, source);
        let profile = frame_profile(&module);
        assert!(
            profile.max_locals <= 255,
            "{name}: 分配后最大帧槽 {} 必须 ≤255（具名 {} / 值 {}）",
            profile.max_locals,
            profile.total_named,
            profile.total_values
        );
        assert!(
            profile.total_values > 0,
            "{name}: 非空断言——语料必须真的产出 SSA 值（否则判据空转）"
        );
        assert!(
            profile.reuse_functions > 0,
            "{name}: 复用实证——必须存在「具名 + 值 > 帧槽」的函数体，实际 0 个"
        );
    }
}
