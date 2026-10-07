//! Program 测试 — 基于 02-stage-contract §目标设计 3/5 / RFC-039 L1
//!
//! §3: Program.stages 只能来自 ProgramKind::stages() 的 6 个预定义组合
//! §5: Aggregation 是 Program 字段；各 kind 的默认聚合对应现状
//! 阶段表的期望内容从 02 §现状「11 处阶段覆盖不一致」表 + P3 止血后的
//! proof 接线推导（规范侧期望值，非实现侧反推）。

use crate::driver::program::{Aggregation, Program, ProgramKind};
use crate::driver::stage::Stage;
use crate::driver::unit::Unit;
use crate::frontend::config::CompileConfig;

const ALL_KINDS: [ProgramKind; 6] = [
    ProgramKind::SingleFile,
    ProgramKind::MultiFile,
    ProgramKind::Check,
    ProgramKind::Lsp,
    ProgramKind::Embedded,
    ProgramKind::WasmPlayground,
];

fn make_program(kind: ProgramKind) -> Program {
    Program::new(kind, Vec::new(), CompileConfig::default())
}

/// §3：每种 ProgramKind 的阶段表——期望值按 02 §现状逐路径归并。
fn expected_stages(kind: ProgramKind) -> &'static [Stage] {
    use Stage::{
        DeadCodeAnalysis, Discovery, GlobalSlotAlloc, IrGeneration, Linking, Monomorphization,
        Parsing, ProofExecution, Registry, RoleClassification, Typecheck, VendorConsistency,
    };
    match kind {
        // 单文件管线（pipeline.rs）：parse → typecheck（内嵌死代码门控）→ proof → IR → mono
        ProgramKind::SingleFile | ProgramKind::WasmPlayground => &[
            Parsing,
            Typecheck,
            DeadCodeAnalysis,
            ProofExecution,
            IrGeneration,
            Monomorphization,
        ],
        // compile_project：vendor → discover → registry → parse/typecheck/proof
        // → slot alloc → IR → link（现状无角色分类/死代码/单态化）
        ProgramKind::MultiFile => &[
            VendorConsistency,
            Discovery,
            Registry,
            Parsing,
            Typecheck,
            ProofExecution,
            GlobalSlotAlloc,
            IrGeneration,
            Linking,
        ],
        // check_project：compile 骨架 + 角色分类 + 死代码族，无 slot/IR/link。
        // 死代码按 Stage::ALL 拓扑序排在 proof 前（现状 proof 在前；
        // 两者无数据依赖，C2 集合语义下诊断集相同——02 §1 修订注记）
        ProgramKind::Check => &[
            VendorConsistency,
            Discovery,
            Registry,
            RoleClassification,
            Parsing,
            Typecheck,
            DeadCodeAnalysis,
            ProofExecution,
        ],
        // check_source_in_project：check 骨架但无角色分类/死代码（现状）
        ProgramKind::Lsp => &[
            VendorConsistency,
            Discovery,
            Registry,
            Parsing,
            Typecheck,
            ProofExecution,
        ],
        // compile_embedded_module：parse → typecheck → proof → 独立 IR
        ProgramKind::Embedded => &[Parsing, Typecheck, ProofExecution, IrGeneration],
    }
}

#[test]
fn test_program_stage_coverage() {
    // Arrange / Act / Assert: 每 kind 的阶段表等于规范期望（02 §3 验收判据）
    for kind in ALL_KINDS {
        assert_eq!(
            kind.stages(),
            expected_stages(kind),
            "ProgramKind::{kind:?} stage table diverges from 02-stage-contract §现状归并"
        );
    }
}

#[test]
fn test_program_stage_tables_are_subsets_of_stage_all() {
    // Arrange / Act / Assert: 任意 kind 的阶段表 ⊆ Stage::ALL 且无重复
    for kind in ALL_KINDS {
        let stages = kind.stages();
        for stage in stages {
            assert!(
                Stage::ALL.contains(stage),
                "ProgramKind::{kind:?} lists {stage:?} which is absent from Stage::ALL"
            );
        }
        for (i, a) in stages.iter().enumerate() {
            for b in stages.iter().skip(i + 1) {
                assert_ne!(a, b, "ProgramKind::{kind:?} lists Stage::{a:?} twice");
            }
        }
    }
}

#[test]
fn test_program_stage_tables_respect_topological_order() {
    // Arrange / Act / Assert: 每 kind 的阶段表是 Stage::ALL 顺序的子序列
    for kind in ALL_KINDS {
        let positions: Vec<usize> = kind
            .stages()
            .iter()
            .map(|s| {
                Stage::ALL
                    .iter()
                    .position(|a| a == s)
                    .unwrap_or_else(|| panic!("{s:?} must be in Stage::ALL"))
            })
            .collect();
        for pair in positions.windows(2) {
            assert!(
                pair[0] < pair[1],
                "ProgramKind::{kind:?} stage table violates Stage::ALL topological order"
            );
        }
    }
}

#[test]
fn test_program_stages_derive_from_kind() {
    // Arrange
    let program = make_program(ProgramKind::Check);

    // Act
    let stages = program.stages();

    // Assert: Program 的阶段表即 kind 的预定义组合（无自定义入口，§3 防退化）
    assert_eq!(stages, ProgramKind::Check.stages());
    assert_eq!(stages, expected_stages(ProgramKind::Check));
}

#[test]
fn test_program_kind_default_aggregation() {
    // Arrange: 02 §5 现状对应表
    let fail_fast = [
        ProgramKind::SingleFile,
        ProgramKind::MultiFile,
        ProgramKind::Embedded,
        ProgramKind::WasmPlayground,
    ];
    let collect_all = [ProgramKind::Check, ProgramKind::Lsp];

    // Act / Assert
    for kind in fail_fast {
        assert!(
            matches!(make_program(kind).aggregation, Aggregation::FailFast),
            "ProgramKind::{kind:?} should default to FailFast"
        );
    }
    for kind in collect_all {
        assert!(
            matches!(make_program(kind).aggregation, Aggregation::CollectAll),
            "ProgramKind::{kind:?} should default to CollectAll"
        );
    }
}

#[test]
fn test_program_with_aggregation_override() {
    // Arrange: LSP 单文件分支——SingleFile 形态 + CollectAll（02 §5）
    let program = make_program(ProgramKind::SingleFile).with_aggregation(Aggregation::CollectAll);

    // Act / Assert
    assert!(
        matches!(program.aggregation, Aggregation::CollectAll),
        "with_aggregation must override the kind default"
    );
    assert_eq!(
        program.stages(),
        expected_stages(ProgramKind::SingleFile),
        "aggregation override must not change the stage table"
    );
}

#[test]
fn test_program_units_passthrough() {
    // Arrange
    let units = vec![Unit::new("app", "app.yx", "main: () -> Void = {}")];

    // Act
    let program = Program::new(ProgramKind::SingleFile, units, CompileConfig::default());

    // Assert
    assert_eq!(program.units.len(), 1);
    assert_eq!(program.units[0].key, "app");
}
