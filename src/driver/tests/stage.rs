//! Stage 测试 — 基于 02-stage-contract §目标设计 1 / RFC-039 L1
//!
//! §1: Stage 为编译期可穷举枚举（12 变体），Stage::ALL 按数据依赖拓扑序
//! §1: StageScope 把「逐模块 vs 项目级」变成类型事实
//! 修订注记: Monomorphization 消费 IR 产物，拓扑序在 IrGeneration 之后

use crate::driver::stage::{Stage, StageScope};

/// 02 §1 的 12 变体全集——测试侧独立罗列，防实现侧漏加/多加。
const ALL_VARIANTS: [Stage; 12] = [
    Stage::VendorConsistency,
    Stage::Discovery,
    Stage::Registry,
    Stage::RoleClassification,
    Stage::Parsing,
    Stage::Typecheck,
    Stage::DeadCodeAnalysis,
    Stage::ProofExecution,
    Stage::GlobalSlotAlloc,
    Stage::IrGeneration,
    Stage::Monomorphization,
    Stage::Linking,
];

fn position_in_all(stage: Stage) -> usize {
    Stage::ALL
        .iter()
        .position(|s| *s == stage)
        .unwrap_or_else(|| panic!("{stage:?} must be present in Stage::ALL"))
}

#[test]
fn test_stage_all_covers_every_variant() {
    // Arrange（ALL_VARIANTS 即规范全集）

    // Act / Assert
    assert_eq!(Stage::ALL.len(), ALL_VARIANTS.len());
    for variant in ALL_VARIANTS {
        assert!(
            Stage::ALL.contains(&variant),
            "Stage::{variant:?} declared but missing from Stage::ALL"
        );
    }
}

#[test]
fn test_stage_all_has_no_duplicates() {
    // Arrange / Act: 逐对比较下标

    // Assert
    for (i, a) in Stage::ALL.iter().enumerate() {
        for (j, b) in Stage::ALL.iter().enumerate() {
            if i != j {
                assert!(
                    a != b,
                    "Stage::{a:?} appears twice in Stage::ALL ({i} and {j})"
                );
            }
        }
    }
}

#[test]
fn test_stage_all_topological_order_follows_data_dependencies() {
    // Arrange: 02 §1 的数据依赖边（上游必须先于下游）
    let dependency_edges: [(Stage, Stage); 7] = [
        (Stage::Discovery, Stage::Registry),
        (Stage::Registry, Stage::Parsing),
        (Stage::Parsing, Stage::Typecheck),
        (Stage::Typecheck, Stage::ProofExecution),
        (Stage::GlobalSlotAlloc, Stage::IrGeneration),
        // 修订注记：mono 消费 IR 产物（monomorphize(&ir, …)），必须在 IrGeneration 后
        (Stage::IrGeneration, Stage::Monomorphization),
        (Stage::IrGeneration, Stage::Linking),
    ];

    // Act / Assert
    for (upstream, downstream) in dependency_edges {
        assert!(
            position_in_all(upstream) < position_in_all(downstream),
            "topological order violated: {upstream:?} must precede {downstream:?} in Stage::ALL"
        );
    }
}

#[test]
fn test_stage_scope_classification() {
    // Arrange: 02 §1 表格的作用域划分
    let per_module = [
        Stage::Parsing,
        Stage::Typecheck,
        Stage::ProofExecution,
        Stage::IrGeneration,
    ];

    // Act / Assert
    for stage in ALL_VARIANTS {
        let expect_per_module = per_module.contains(&stage);
        if expect_per_module {
            assert!(
                matches!(stage.scope(), StageScope::PerModule),
                "Stage::{stage:?} should be PerModule"
            );
        } else {
            assert!(
                matches!(stage.scope(), StageScope::Project),
                "Stage::{stage:?} should be Project"
            );
        }
    }
}

#[test]
fn test_stage_slug_unique_and_nonempty() {
    // Arrange / Act: 收集全部 slug

    // Assert
    for (i, a) in ALL_VARIANTS.iter().enumerate() {
        assert!(!a.slug().is_empty(), "Stage::{a:?} slug must be nonempty");
        for b in ALL_VARIANTS.iter().skip(i + 1) {
            assert_ne!(
                a.slug(),
                b.slug(),
                "Stage::{a:?} and Stage::{b:?} share a slug"
            );
        }
    }
}
