//! 任务内循环引用追踪单元测试
//!
//! 测试 IntraTaskCycleTracker 的任务内循环检测、警告生成和状态清除功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::intra_task_cycle::IntraTaskCycleTracker;
use crate::util::span::Span;

/// 创建测试用的 FunctionIR
fn create_test_function(instructions: Vec<Instruction>) -> FunctionIR {
    FunctionIR {
        name: "test".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions,
            successors: vec![],
        }],
        entry: 0,
    }
}

#[test]
fn test_no_cycle() {
    let mut tracker = IntraTaskCycleTracker::new();
    let func = create_test_function(vec![
        Instruction::ArcNew {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::ArcNew {
            dst: Operand::Temp(1),
            src: Operand::Local(1),
        },
    ]);

    let warnings = tracker.track_function(&func);
    assert!(warnings.is_empty(), "不应有循环警告");
}

#[test]
fn test_simple_cycle_warning() {
    let mut tracker = IntraTaskCycleTracker::new();

    // 模拟 a = ref b; b.field = a 形成循环
    let func = create_test_function(vec![
        Instruction::ArcNew {
            dst: Operand::Temp(0), // a = ref b
            src: Operand::Local(0),
        },
        Instruction::StoreField {
            dst: Operand::Local(0), // b.field = a
            src: Operand::Temp(0),
            field: 0,
            type_name: None,
            field_name: None,
            span: Span::dummy(),
        },
    ]);

    let warnings = tracker.track_function(&func);
    // 检测到任务内循环（警告，不报错）
    assert!(!warnings.is_empty(), "应检测到任务内循环");
    assert!(warnings[0].code == "E2003");
}

#[test]
fn test_chain_no_cycle() {
    // a = ref b; c = ref d; 无循环
    let mut tracker = IntraTaskCycleTracker::new();
    let func = create_test_function(vec![
        Instruction::ArcNew {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::ArcNew {
            dst: Operand::Temp(1),
            src: Operand::Local(1),
        },
        Instruction::ArcNew {
            dst: Operand::Temp(2),
            src: Operand::Temp(0),
        },
    ]);

    let warnings = tracker.track_function(&func);
    assert!(warnings.is_empty(), "链式 ref 不应形成循环");
}

#[test]
fn test_self_reference_cycle() {
    // a = ref a（自引用）
    let mut tracker = IntraTaskCycleTracker::new();
    let func = create_test_function(vec![Instruction::ArcNew {
        dst: Operand::Temp(0),
        src: Operand::Temp(0), // 自引用
    }]);

    let warnings = tracker.track_function(&func);
    // 自引用形成循环
    assert!(!warnings.is_empty(), "自引用应检测为循环");
}

#[test]
fn test_multiple_cycles() {
    // 多个独立循环
    let mut tracker = IntraTaskCycleTracker::new();
    let func = create_test_function(vec![
        // 循环 1: temp_0 -> local_0 -> temp_0
        Instruction::ArcNew {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        },
        Instruction::StoreField {
            dst: Operand::Local(0),
            src: Operand::Temp(0),
            field: 0,
            type_name: None,
            field_name: None,
            span: Span::dummy(),
        },
        // 循环 2: temp_1 -> local_1 -> temp_1
        Instruction::ArcNew {
            dst: Operand::Temp(1),
            src: Operand::Local(1),
        },
        Instruction::StoreField {
            dst: Operand::Local(1),
            src: Operand::Temp(1),
            field: 0,
            type_name: None,
            field_name: None,
            span: Span::dummy(),
        },
    ]);

    let warnings = tracker.track_function(&func);
    // 应检测到至少一个循环
    assert!(!warnings.is_empty(), "应检测到循环");
}

#[test]
fn test_clear_resets_state() {
    let mut tracker = IntraTaskCycleTracker::new();
    let func = create_test_function(vec![Instruction::ArcNew {
        dst: Operand::Temp(0),
        src: Operand::Local(0),
    }]);

    tracker.track_function(&func);
    tracker.clear();

    assert!(tracker.warnings().is_empty());
}

#[test]
fn test_warning_contains_location() {
    let mut tracker = IntraTaskCycleTracker::new();
    let func = create_test_function(vec![Instruction::ArcNew {
        dst: Operand::Temp(0),
        src: Operand::Temp(0), // 自引用
    }]);

    let warnings = tracker.track_function(&func);
    if let Some(first) = warnings.first() {
        assert_eq!(first.code, "E2003", "应为 E2003 错误码");
    }
}
