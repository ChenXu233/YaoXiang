//! Send/Sync 检查器单元测试

use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
use crate::middle::lifetime::SendSyncChecker;

/// 测试：基本类型总是 Send
#[test]
fn test_primitives_are_send() {
    let checker = SendSyncChecker::new();

    assert!(checker.is_send(&MonoType::Void));
    assert!(checker.is_send(&MonoType::Bool));
    assert!(checker.is_send(&MonoType::Int(64)));
    assert!(checker.is_send(&MonoType::Float(64)));
    assert!(checker.is_send(&MonoType::Char));
    assert!(checker.is_send(&MonoType::String));
    assert!(checker.is_send(&MonoType::Bytes));
}

/// 测试：基本类型总是 Sync
#[test]
fn test_primitives_are_sync() {
    let checker = SendSyncChecker::new();

    assert!(checker.is_sync(&MonoType::Void));
    assert!(checker.is_sync(&MonoType::Bool));
    assert!(checker.is_sync(&MonoType::Int(64)));
    assert!(checker.is_sync(&MonoType::Float(64)));
    assert!(checker.is_sync(&MonoType::Char));
    assert!(checker.is_sync(&MonoType::String));
    assert!(checker.is_sync(&MonoType::Bytes));
}

/// 测试：列表类型
#[test]
fn test_list_types() {
    let checker = SendSyncChecker::new();

    let int_list = MonoType::List(Box::new(MonoType::Int(64)));
    assert!(checker.is_send(&int_list));
    // 列表默认不是 Sync（除非是 Arc[List[T]]）
    assert!(!checker.is_sync(&int_list));
}

/// 测试：元组类型
#[test]
fn test_tuple_types() {
    let checker = SendSyncChecker::new();

    let tuple = MonoType::Tuple(vec![MonoType::Int(64), MonoType::Float(64)]);
    assert!(checker.is_send(&tuple));
    assert!(checker.is_sync(&tuple));

    let tuple_with_ref = MonoType::Tuple(vec![MonoType::Int(64), MonoType::String]);
    assert!(checker.is_send(&tuple_with_ref));
}

/// 测试：Arc 类型总是 Send + Sync
#[test]
fn test_arc_is_send_sync() {
    let checker = SendSyncChecker::new();

    let arc_int = MonoType::Arc(Box::new(MonoType::Int(64)));
    assert!(checker.is_send(&arc_int));
    assert!(checker.is_sync(&arc_int));

    let arc_struct = MonoType::Arc(Box::new(MonoType::Struct(
        crate::frontend::typecheck::StructType {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MonoType::Int(64)),
                ("y".to_string(), MonoType::Int(64)),
            ],
        },
    )));
    assert!(checker.is_send(&arc_struct));
    assert!(checker.is_sync(&arc_struct));
}

/// 测试：结构体类型
#[test]
fn test_struct_types() {
    let checker = SendSyncChecker::new();

    let point = MonoType::Struct(crate::frontend::typecheck::StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Int(64)),
            ("y".to_string(), MonoType::Float(64)),
        ],
    });
    assert!(checker.is_send(&point));
    assert!(checker.is_sync(&point));
}

/// 测试：函数类型
#[test]
fn test_fn_types() {
    let checker = SendSyncChecker::new();

    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Int(64)),
        is_async: false,
    };
    assert!(checker.is_send(&fn_type));
}

/// 测试：Dict 类型
#[test]
fn test_dict_types() {
    let checker = SendSyncChecker::new();

    let dict = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(64)));
    assert!(checker.is_send(&dict));
    assert!(!checker.is_sync(&dict));
}

/// 测试：Set 类型
#[test]
fn test_set_types() {
    let checker = SendSyncChecker::new();

    let set = MonoType::Set(Box::new(MonoType::String));
    assert!(checker.is_send(&set));
    assert!(!checker.is_sync(&set));
}

/// 测试：Range 类型
#[test]
fn test_range_types() {
    let checker = SendSyncChecker::new();

    let range = MonoType::Range {
        elem_type: Box::new(MonoType::Int(64)),
    };
    assert!(checker.is_send(&range));
    assert!(checker.is_sync(&range));
}

/// 测试：无 spawn 指令时无错误
#[test]
fn test_no_spawn_no_errors() {
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        is_async: false,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Move {
                    dst: Operand::Temp(0),
                    src: Operand::Local(0),
                },
                Instruction::Ret(Some(Operand::Temp(0))),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = SendSyncChecker::new();
    let errors = checker.check_function(&func);

    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
}

/// 测试：spawn 无捕获变量的闭包
#[test]
fn test_spawn_closure_no_captured() {
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 创建一个无捕获的闭包
                Instruction::MakeClosure {
                    dst: Operand::Local(0),
                    func: 0,     // 假设闭包函数索引为 0
                    env: vec![], // 无捕获变量
                },
                // spawn 这个闭包
                Instruction::Spawn {
                    func: Operand::Local(0),
                    args: vec![],
                    result: Operand::Local(2),
                },
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = SendSyncChecker::new();
    let errors = checker.check_function(&func);

    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
}

/// 测试：spawn 捕获 Send 类型的闭包
#[test]
fn test_spawn_closure_captures_send() {
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Void,
        is_async: false,
        locals: vec![MonoType::Int(64)], // local_0: Int (Send)
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                // 捕获 local_0 (Int 类型，Send)
                Instruction::MakeClosure {
                    dst: Operand::Local(1),
                    func: 0,
                    env: vec![Operand::Local(0)],
                },
                Instruction::Spawn {
                    func: Operand::Local(1),
                    args: vec![],
                    result: Operand::Local(2),
                },
                Instruction::Ret(None),
            ],
            successors: vec![],
        }],
        entry: 0,
    };

    let mut checker = SendSyncChecker::new();
    let errors = checker.check_function(&func);

    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
}

/// 测试：checker 可以创建和清空
#[test]
fn test_checker_lifecycle() {
    let mut checker = SendSyncChecker::new();
    assert!(checker.errors().is_empty());

    checker.clear();
    assert!(checker.errors().is_empty());
}
