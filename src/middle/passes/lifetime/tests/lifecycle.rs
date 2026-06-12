//! 变量生命周期追踪器单元测试
//!
//! 测试 LifecycleTracker 的变量生命周期追踪、问题检测和事件收集功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::lifecycle::{LifecycleIssueKind, LifecycleTracker};
use crate::util::span::Span;

fn make_simple_function() -> FunctionIR {
    FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
            successors: vec![],
        }],
        entry: 0,
    }
}

#[test]
fn test_track_created() {
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = make_simple_function();
    tracker.analyze_function(&func);
    // 参数应该被追踪
    assert!(tracker.get_lifecycle(&Operand::Arg(0)).is_some());
}

#[test]
fn test_detect_never_used() {
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Alloc {
                    dst: Operand::Local(0),
                    size: Operand::Const(ConstValue::Int(4)),
                },
                Instruction::Drop(Operand::Local(0)),
            ],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    let issues = tracker.detect_issues();
    let never_used = issues
        .iter()
        .any(|i| i.kind == LifecycleIssueKind::NeverUsed);
    assert!(never_used);
}

#[test]
fn test_multiple_consume() {
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Move {
                    dst: Operand::Temp(0),
                    src: Operand::Arg(0),
                },
                Instruction::Move {
                    dst: Operand::Temp(1),
                    src: Operand::Arg(0),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    let issues = tracker.detect_issues();
    let multiple_consume = issues
        .iter()
        .any(|i| i.kind == LifecycleIssueKind::MultipleConsume);
    assert!(multiple_consume);
}

#[test]
fn test_track_store_instruction() {
    // 测试 Store 指令的创建和消费追踪
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Alloc {
                    dst: Operand::Local(0),
                    size: Operand::Const(ConstValue::Int(4)),
                },
                Instruction::Store {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                    span: Span::dummy(),
                },
            ],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    // Local 0 应该被创建
    let local_lifecycle = tracker.get_lifecycle(&Operand::Local(0));
    assert!(local_lifecycle.is_some());
    assert!(local_lifecycle.unwrap().creation_location.is_some());
}

#[test]
fn test_track_call_instruction() {
    // 测试 Call 指令的参数消费和返回值创建
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Call {
                    dst: Some(Operand::Temp(0)),
                    func: Operand::Global(0), // 使用索引表示函数
                    args: vec![Operand::Arg(0)],
                    span: Span::dummy(),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    // 参数应该被消费
    let arg_lifecycle = tracker.get_lifecycle(&Operand::Arg(0));
    assert!(arg_lifecycle.is_some());
    assert!(arg_lifecycle.unwrap().is_consumed());
    // 返回值应该被创建
    let ret_lifecycle = tracker.get_lifecycle(&Operand::Temp(0));
    assert!(ret_lifecycle.is_some());
    assert!(ret_lifecycle.unwrap().creation_location.is_some());
}

#[test]
fn test_track_drop_without_consume() {
    // 测试检测未消费就释放
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Alloc {
                    dst: Operand::Local(0),
                    size: Operand::Const(ConstValue::Int(4)),
                },
                // 不使用直接 Drop
                Instruction::Drop(Operand::Local(0)),
            ],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    let issues = tracker.detect_issues();
    let drop_without_consume = issues
        .iter()
        .any(|i| i.kind == LifecycleIssueKind::DropWithoutConsume);
    assert!(drop_without_consume);
}

#[test]
fn test_get_all_lifecycles() {
    // 测试获取所有生命周期信息
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    let lifecycles = tracker.get_all_lifecycles();
    // 应该包含参数
    assert!(!lifecycles.is_empty());
}

#[test]
fn test_events_collection() {
    // 测试事件收集
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    let events = tracker.events();
    // 应该有事件记录
    assert!(!events.is_empty());
}

#[test]
fn test_lifecycle_info_is_consumed() {
    // 测试 is_consumed 方法
    let mut tracker = LifecycleTracker::new("test".to_string());
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(0)],
        return_type: MonoType::Int(0),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Move {
                    dst: Operand::Temp(0),
                    src: Operand::Arg(0),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };
    tracker.analyze_function(&func);
    let arg_lifecycle = tracker.get_lifecycle(&Operand::Arg(0)).unwrap();
    assert!(arg_lifecycle.is_consumed());
}

#[test]
fn test_unknown_operand_returns_none() {
    // 测试未知操作数返回 None
    let tracker = LifecycleTracker::new("test".to_string());
    let lifecycle = tracker.get_lifecycle(&Operand::Temp(999));
    assert!(lifecycle.is_none());
}
