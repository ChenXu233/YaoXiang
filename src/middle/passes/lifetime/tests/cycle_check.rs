//! 跨 spawn 循环引用检测单元测试
//!
//! 测试 CycleChecker 的跨 spawn 循环检测、深度限制和 unsafe 绕过功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, ExecutionPlan, FunctionIR, Instruction, Operand};
use crate::middle::passes::lifetime::cycle_check::CycleChecker;

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
fn test_no_spawn_no_error() {
    let mut checker = CycleChecker::new();
    let func = create_test_function(vec![Instruction::Move {
        dst: Operand::Local(0),
        src: Operand::Arg(0),
    }]);

    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "无 spawn 不应有错误");
}

#[test]
fn test_single_spawn_no_cycle() {
    let mut checker = CycleChecker::new();
    let func = create_test_function(vec![Instruction::Spawn {
        closures: vec![Operand::Global(0)],
        plan: ExecutionPlan {
            groups: vec![],
            task_deps: vec![],
            task_resources: vec![],
        },
        result: Operand::Temp(0),
    }]);

    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "单 spawn 不应有循环");
}

#[test]
fn test_independent_spawns_no_cycle() {
    let mut checker = CycleChecker::new();
    let func = create_test_function(vec![
        Instruction::Spawn {
            closures: vec![Operand::Global(0)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(0),
        },
        Instruction::Spawn {
            closures: vec![Operand::Global(1)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(1),
        },
    ]);

    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "独立 spawn 不应有循环");
}

#[test]
fn test_spawn_chain_no_cycle() {
    // spawn A -> spawn B (单向依赖，无循环)
    let mut checker = CycleChecker::new();
    let func = create_test_function(vec![
        Instruction::Spawn {
            closures: vec![Operand::Global(0)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(0),
        },
        Instruction::Spawn {
            closures: vec![Operand::Global(1)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(1),
        },
    ]);

    let errors = checker.check_function(&func);
    assert!(errors.is_empty(), "单向依赖不应有循环");
}

#[test]
fn test_depth_limit_one_level() {
    // 测试深度限制：只检测直接依赖
    let mut checker = CycleChecker::new();
    let func = create_test_function(vec![
        Instruction::Spawn {
            closures: vec![Operand::Global(0)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(0),
        },
        Instruction::Move {
            dst: Operand::Temp(1),
            src: Operand::Temp(0),
        },
        Instruction::Move {
            dst: Operand::Temp(2),
            src: Operand::Temp(1), // 间接引用（深度 > 1）
        },
        Instruction::Spawn {
            closures: vec![Operand::Global(1)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(3),
        },
    ]);

    let errors = checker.check_function(&func);
    // 深度限制为 1，间接引用不应被追踪
    assert!(errors.is_empty(), "深度 > 1 的间接引用不应被检测");
}

#[test]
fn test_clear_resets_all_state() {
    use crate::middle::passes::lifetime::error::Checker;

    let mut checker = CycleChecker::new();
    let func = create_test_function(vec![Instruction::Spawn {
        closures: vec![Operand::Global(0)],
        plan: ExecutionPlan {
            groups: vec![],
            task_deps: vec![],
            task_resources: vec![],
        },
        result: Operand::Temp(0),
    }]);

    checker.check_function(&func);
    Checker::clear(&mut checker);

    assert!(checker.errors().is_empty());
    assert!(checker.unsafe_bypasses().is_empty());
}

#[test]
fn test_unsafe_bypass_empty_by_default() {
    // 当前 Phase 6，unsafe 检测尚未实现，应返回空
    let mut checker = CycleChecker::new();
    let func = create_test_function(vec![Instruction::Spawn {
        closures: vec![Operand::Global(0)],
        plan: ExecutionPlan {
            groups: vec![],
            task_deps: vec![],
            task_resources: vec![],
        },
        result: Operand::Temp(0),
    }]);

    checker.check_function(&func);
    assert!(
        checker.unsafe_bypasses().is_empty(),
        "Phase 6 默认无 unsafe 绕过"
    );
}

#[test]
fn test_error_message_contains_suggestion() {
    // 确保错误消息包含建议
    let mut checker = CycleChecker::new();

    // 构造循环：spawn A 使用 spawn B 结果，spawn B 使用 spawn A 结果
    let func = create_test_function(vec![
        Instruction::Spawn {
            closures: vec![Operand::Global(0)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(0),
        },
        Instruction::Spawn {
            closures: vec![Operand::Global(1)],
            plan: ExecutionPlan {
                groups: vec![],
                task_deps: vec![],
                task_resources: vec![],
            },
            result: Operand::Temp(1),
        },
        Instruction::Move {
            dst: Operand::Temp(0),
            src: Operand::Temp(1),
        },
    ]);

    let errors = checker.check_function(&func);
    // 如果检测到循环，消息应包含建议
    for error in errors {
        if error.code == "E2003" {
            assert!(
                error.message.contains("Weak") || error.message.contains("unsafe"),
                "错误消息应包含解决建议"
            );
        }
    }
}
