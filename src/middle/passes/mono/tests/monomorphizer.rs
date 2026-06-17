//! Monomorphizer 核心逻辑测试 — 对应 src/middle/passes/mono/mod.rs
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 泛型函数特化

use crate::frontend::core::typecheck::MonoType;
use crate::frontend::core::types::var::TypeVar;
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::middle::passes::mono::instance::{
    GenericFunctionId, InstantiationRequest, SpecializationKey,
};
use crate::middle::passes::mono::Monomorphizer;
use crate::util::span::Span;

// ==================== 辅助函数 ====================

/// 创建简单的泛型 identity 函数 IR
/// fn identity<T>(x: T) -> T { return x; }
fn make_identity_ir() -> FunctionIR {
    let param_type = MonoType::TypeVar(TypeVar::new(0));
    FunctionIR {
        name: "identity".to_string(),
        params: vec![param_type.clone()],
        return_type: param_type.clone(),
        locals: vec![param_type.clone()],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Load {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: Some(vec!["T".to_string()]),
    }
}

/// 创建泛型 swap 函数 IR
/// fn swap<T>(a: T, b: T) -> (T, T)
fn make_swap_ir() -> FunctionIR {
    let t = MonoType::TypeVar(TypeVar::new(0));
    FunctionIR {
        name: "swap".to_string(),
        params: vec![t.clone(), t.clone()],
        return_type: MonoType::Tuple(vec![t.clone(), t.clone()]),
        locals: vec![t.clone(), t.clone()],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Load {
                    dst: Operand::Local(0),
                    src: Operand::Arg(0),
                },
                Instruction::Load {
                    dst: Operand::Local(1),
                    src: Operand::Arg(1),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: Some(vec!["T".to_string()]),
    }
}

// ==================== specialize_function 测试 ====================

#[test]
fn test_specialize_identity_with_int() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let req = InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_some(), "特化应该成功");
    let func = result.unwrap();
    assert_eq!(func.name, "identity(int64)");
    assert_eq!(func.params.len(), 1);
    assert_eq!(func.params[0], MonoType::Int(64), "参数类型应为 Int(64)");
    assert_eq!(func.return_type, MonoType::Int(64), "返回类型应为 Int(64)");
    assert_eq!(
        func.locals[0],
        MonoType::Int(64),
        "局部变量类型应为 Int(64)"
    );
    assert!(func.generic_params.is_none(), "泛型标记应已清除");
    assert_eq!(func.blocks.len(), 1);
    assert_eq!(func.blocks[0].instructions.len(), 2);
}

#[test]
fn test_specialize_identity_with_string() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let req = InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::String],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_some(), "特化应该成功");
    let func = result.unwrap();
    assert_eq!(func.name, "identity(string)");
    assert_eq!(func.params[0], MonoType::String);
    assert_eq!(func.return_type, MonoType::String);
    assert!(func.generic_params.is_none());
}

#[test]
fn test_specialize_swap_with_float() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("swap".to_string(), make_swap_ir());

    let req = InstantiationRequest::new(
        GenericFunctionId::new("swap".to_string(), vec!["T".to_string()]),
        vec![MonoType::Float(64)],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_some(), "特化应该成功");
    let func = result.unwrap();
    assert_eq!(func.name, "swap(float64)");
    assert_eq!(func.params.len(), 2);
    assert_eq!(func.params[0], MonoType::Float(64));
    assert_eq!(func.params[1], MonoType::Float(64));
    assert_eq!(
        func.return_type,
        MonoType::Tuple(vec![MonoType::Float(64), MonoType::Float(64)])
    );
    assert!(func.generic_params.is_none());
}

#[test]
fn test_specialize_missing_generic_function_returns_none() {
    // Arrange
    let mono = Monomorphizer::new();
    let req = InstantiationRequest::new(
        GenericFunctionId::new("nonexistent".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_none(), "不存在的泛型函数应返回 None");
}

#[test]
fn test_specialize_type_args_mismatch_returns_none() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let req = InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64), MonoType::String],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_none(), "类型参数数量不匹配应返回 None");
}

#[test]
fn test_specialize_non_generic_function_returns_none() {
    // Arrange
    let mut mono = Monomorphizer::new();
    let func = FunctionIR {
        name: "add".to_string(),
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: MonoType::Int(64),
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: None,
    };
    mono.generic_functions.insert("add".to_string(), func);

    let req = InstantiationRequest::new(
        GenericFunctionId::new("add".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(
        result.is_none(),
        "非泛型函数特化应返回 None（generic_params 为 None）"
    );
}

#[test]
fn test_specialize_with_generic_type_args_replaces_inner_types() {
    // Arrange
    let t = MonoType::TypeVar(TypeVar::new(0));
    let list_t = MonoType::List(Box::new(t.clone()));

    let generic = FunctionIR {
        name: "first".to_string(),
        params: vec![list_t],
        return_type: t,
        locals: vec![MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(0))))],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(None)],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: Some(vec!["T".to_string()]),
    };

    let mut mono = Monomorphizer::new();
    mono.generic_functions.insert("first".to_string(), generic);

    let req = InstantiationRequest::new(
        GenericFunctionId::new("first".to_string(), vec!["T".to_string()]),
        vec![MonoType::String],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_some(), "特化应该成功");
    let func = result.unwrap();
    assert_eq!(func.params[0], MonoType::List(Box::new(MonoType::String)));
    assert_eq!(func.return_type, MonoType::String);
    assert_eq!(func.locals[0], MonoType::List(Box::new(MonoType::String)));
}

// ==================== scan_for_new_calls 测试 ====================

#[test]
fn test_scan_for_new_calls_no_generic_calls_leaves_queue_empty() {
    // Arrange
    let mut mono = Monomorphizer::new();
    let func = FunctionIR {
        name: "simple".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Ret(None)],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: None,
    };

    // Act
    mono.scan_for_new_calls(&func);

    // Assert
    assert!(mono.pending_queue.is_empty(), "无泛型调用时队列应为空");
}

#[test]
fn test_scan_for_new_calls_with_generic_call_enqueues_request() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let func = FunctionIR {
        name: "wrapper(Int)".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Call {
                    dst: Some(Operand::Local(0)),
                    func: Operand::Const(ConstValue::String("identity".to_string())),
                    args: vec![Operand::Arg(0)],
                    span: Span::default(),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: None,
    };

    // Act
    mono.scan_for_new_calls(&func);

    // Assert
    assert_eq!(mono.pending_queue.len(), 1, "应该有一个新的实例化请求");
    let pending = &mono.pending_queue[0];
    assert_eq!(pending.generic_id().name(), "identity");
    assert_eq!(pending.type_args().len(), 1);
    assert_eq!(pending.type_args()[0], MonoType::Int(64));
}

#[test]
fn test_scan_for_new_calls_duplicate_prevented_by_processed_set() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    mono.processed.insert(SpecializationKey::new(
        "identity".to_string(),
        vec![MonoType::Int(64)],
    ));

    let func = FunctionIR {
        name: "dup_check".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Call {
                    dst: Some(Operand::Local(0)),
                    func: Operand::Const(ConstValue::String("identity".to_string())),
                    args: vec![Operand::Arg(0)],
                    span: Span::default(),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: None,
    };

    // Act
    mono.scan_for_new_calls(&func);

    // Assert
    assert!(
        mono.pending_queue.is_empty(),
        "已处理的请求不应重复加入队列"
    );
}

// ==================== operand_to_type_hint 测试 ====================

#[test]
fn test_operand_to_type_hint_resolves_arg_local_and_const() {
    // Arrange
    let mono = Monomorphizer::new();
    let func = FunctionIR {
        name: "test".to_string(),
        params: vec![MonoType::Int(64), MonoType::String],
        return_type: MonoType::Void,
        locals: vec![MonoType::Bool, MonoType::Float(64)],
        blocks: vec![],
        entry: 0,
        generic_params: None,
    };

    // Assert: Arg(0) -> Int(64)
    assert_eq!(
        mono.operand_to_type_hint(&Operand::Arg(0), &func),
        Some(MonoType::Int(64))
    );

    // Assert: Arg(1) -> String
    assert_eq!(
        mono.operand_to_type_hint(&Operand::Arg(1), &func),
        Some(MonoType::String)
    );

    // Assert: Arg(99) -> None (越界)
    assert_eq!(mono.operand_to_type_hint(&Operand::Arg(99), &func), None);

    // Assert: Local(0) -> Bool
    assert_eq!(
        mono.operand_to_type_hint(&Operand::Local(0), &func),
        Some(MonoType::Bool)
    );

    // Assert: Const(Int) -> Int(64)
    assert_eq!(
        mono.operand_to_type_hint(&Operand::Const(ConstValue::Int(42)), &func),
        Some(MonoType::Int(64))
    );

    // Assert: Const(String) -> String
    assert_eq!(
        mono.operand_to_type_hint(
            &Operand::Const(ConstValue::String("hello".to_string())),
            &func,
        ),
        Some(MonoType::String)
    );
}

// ==================== replace_call_sites 测试 ====================

#[test]
fn test_replace_call_sites_replaces_generic_call_in_main() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let main_func = FunctionIR {
        name: "main".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Call {
                    dst: Some(Operand::Local(0)),
                    func: Operand::Const(ConstValue::String("identity".to_string())),
                    args: vec![Operand::Const(ConstValue::Int(42))],
                    span: Span::default(),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: None,
    };

    let mut module = ModuleIR {
        functions: vec![main_func],
        ..Default::default()
    };

    let requests = vec![InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    )];

    // Act
    mono.replace_call_sites(&mut module, &requests);

    // Assert
    let main_func = &module.functions[0];
    assert!(
        matches!(
            &main_func.blocks[0].instructions[0],
            Instruction::Call { func: callee, .. }
            if *callee == Operand::Const(ConstValue::String("identity(int64)".to_string()))
        ),
        "Call 指令的 func 应该被替换为特化函数名 identity(int64)"
    );
}

#[test]
fn test_replace_call_sites_skips_generic_functions() {
    // Arrange
    let mono = Monomorphizer::new();

    let wrapper_func = FunctionIR {
        name: "wrapper".to_string(),
        params: vec![MonoType::TypeVar(TypeVar::new(0))],
        return_type: MonoType::TypeVar(TypeVar::new(0)),
        locals: vec![MonoType::TypeVar(TypeVar::new(0))],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Call {
                    dst: Some(Operand::Local(0)),
                    func: Operand::Const(ConstValue::String("identity".to_string())),
                    args: vec![Operand::Arg(0)],
                    span: Span::default(),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: Some(vec!["T".to_string()]),
    };

    let mut module = ModuleIR {
        functions: vec![wrapper_func],
        ..Default::default()
    };

    let requests = vec![InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    )];

    // Act
    mono.replace_call_sites(&mut module, &requests);

    // Assert
    let wrapper = &module.functions[0];
    assert!(
        matches!(
            &wrapper.blocks[0].instructions[0],
            Instruction::Call { func: callee, .. }
            if *callee == Operand::Const(ConstValue::String("identity".to_string()))
        ),
        "泛型函数内的调用不应被替换"
    );
}

#[test]
fn test_replace_call_sites_no_matching_request_does_not_replace() {
    // Arrange
    let mono = Monomorphizer::new();

    let main_func = FunctionIR {
        name: "main".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        locals: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![Instruction::Call {
                dst: None,
                func: Operand::Const(ConstValue::String("foo".to_string())),
                args: vec![],
                span: Span::default(),
            }],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: None,
    };

    let mut module = ModuleIR {
        functions: vec![main_func],
        ..Default::default()
    };

    let requests = vec![InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    )];

    // Act
    mono.replace_call_sites(&mut module, &requests);

    // Assert
    let main_func = &module.functions[0];
    assert!(
        matches!(
            &main_func.blocks[0].instructions[0],
            Instruction::Call { func: callee, .. }
            if *callee == Operand::Const(ConstValue::String("foo".to_string()))
        ),
        "不匹配的调用不应被替换"
    );
}

// ==================== monomorphize 端到端测试 ====================

#[test]
fn test_monomorphize_end_to_end_specializes_and_replaces_calls() {
    // Arrange
    let identity = make_identity_ir();

    let main_func = FunctionIR {
        name: "main".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        locals: vec![MonoType::Int(64)],
        blocks: vec![BasicBlock {
            label: 0,
            instructions: vec![
                Instruction::Call {
                    dst: Some(Operand::Local(0)),
                    func: Operand::Const(ConstValue::String("identity".to_string())),
                    args: vec![Operand::Const(ConstValue::Int(42))],
                    span: Span::default(),
                },
                Instruction::Ret(Some(Operand::Local(0))),
            ],
            successors: Vec::new(),
        }],
        entry: 0,
        generic_params: None,
    };

    let module = ModuleIR {
        functions: vec![identity, main_func],
        ..Default::default()
    };

    let mut mono = Monomorphizer::new();
    let requests = vec![InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    )];

    // Act
    let result = mono.monomorphize(&module, &requests);

    // Assert: 应有 2 个函数：main（调用已替换）+ identity(int64)
    assert_eq!(result.functions.len(), 2);

    // Assert: main 中的调用应被替换为 identity(int64)
    let main_out = result.functions.iter().find(|f| f.name == "main").unwrap();
    assert!(
        matches!(
            &main_out.blocks[0].instructions[0],
            Instruction::Call { func: callee, .. }
            if *callee == Operand::Const(ConstValue::String("identity(int64)".to_string()))
        ),
        "main 中的调用应被替换为 identity(int64)"
    );

    // Assert: 特化函数存在且泛型标记已清除
    let specialized = result
        .functions
        .iter()
        .find(|f| f.name == "identity(int64)")
        .expect("应该存在 identity(int64) 特化函数");
    assert!(
        specialized.generic_params.is_none(),
        "特化函数的泛型标记应已清除"
    );
}
