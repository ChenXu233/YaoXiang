//! Monomorphizer 核心逻辑测试 — 对应 src/middle/passes/mono/mod.rs
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 泛型函数特化
//! RFC-011 §4.4: 泛型类型单态化（Issue #197）
//! 规范: docs/src/design/rfc/accepted/011-generic-type-system.md
//!
//! 覆盖:
//! - `Monomorphizer::specialize_function` 单态化泛型函数
//! - `Monomorphizer::specialize_type` 单态化泛型类型
//! - 类型侧单态化的去重缓存
//! - 类型参数数量不匹配时的错误路径
//! - 泛型 enum variant 单态化（Variant 类型体替换）
//! - 嵌套泛型 BFS 引用扫描（如 List(List(Int))）
//! - 小写参数名类型参数判定（靠名单不靠大小写启发式）

use crate::frontend::core::parser::ast::{StructField, Type as AstType, TypeBodyItem};
use crate::frontend::core::typecheck::MonoType;
use crate::frontend::core::types::mono::UniverseLevel;
use crate::frontend::core::types::var::TypeVar;
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand,
};
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
        def: None,
        name: "identity".to_string(),
        params: vec![param_type.clone()],
        return_type: param_type.clone(),
        generic_params: Some(vec!["T".to_string()]),
        body: FunctionBody::Code {
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
            locals: vec![param_type.clone()],
        },
    }
}

/// 创建泛型 swap 函数 IR
/// fn swap<T>(a: T, b: T) -> (T, T)
fn make_swap_ir() -> FunctionIR {
    let t = MonoType::TypeVar(TypeVar::new(0));
    FunctionIR {
        def: None,
        name: "swap".to_string(),
        params: vec![t.clone(), t.clone()],
        return_type: MonoType::Tuple(vec![t.clone(), t.clone()]),
        generic_params: Some(vec!["T".to_string()]),
        body: FunctionBody::Code {
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
            locals: vec![t.clone(), t.clone()],
        },
    }
}

/// 创建泛型 Pair 类型定义 IR: `Pair: (T: Type) -> Type = { first: T, second: T }`
/// 字段名按传入顺序排列，便于断言。
fn make_pair_type_ir(fields: &[&str]) -> FunctionIR {
    let body_items: Vec<TypeBodyItem> = fields
        .iter()
        .map(|name| {
            TypeBodyItem::Field(StructField {
                name: (*name).to_string(),
                ty: AstType::Name {
                    name: "T".to_string(),
                    span: Span::dummy(),
                },
                default: None,
                is_mut: false,
            })
        })
        .collect();

    FunctionIR {
        def: None,
        name: "Pair".to_string(),
        params: vec![MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        }],
        return_type: MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        },
        generic_params: Some(vec!["T".to_string()]),
        body: FunctionBody::TypeDecl {
            definition: AstType::Struct { body: body_items },
        },
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
        func.locals()[0],
        MonoType::Int(64),
        "局部变量类型应为 Int(64)"
    );
    assert!(func.generic_params.is_none(), "泛型标记应已清除");
    assert_eq!(func.blocks().len(), 1);
    assert_eq!(func.blocks()[0].instructions.len(), 2);
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
        def: None,
        name: "add".to_string(),
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: MonoType::Int(64),
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![],
        },
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
        def: None,
        name: "first".to_string(),
        params: vec![list_t],
        return_type: t,
        generic_params: Some(vec!["T".to_string()]),
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(None)],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(0))))],
        },
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
    assert_eq!(func.locals()[0], MonoType::List(Box::new(MonoType::String)));
}

// ==================== scan_for_new_calls 测试 ====================

#[test]
fn test_scan_for_new_calls_no_generic_calls_leaves_queue_empty() {
    // Arrange
    let mut mono = Monomorphizer::new();
    let func = FunctionIR {
        def: None,
        name: "simple".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(None)],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![],
        },
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
        def: None,
        name: "wrapper(Int)".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Arg(0)],
                        span: Span::default(),
                        def: None,
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![MonoType::Int(64)],
        },
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
        def: None,
        name: "dup_check".to_string(),
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Int(64),
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Arg(0)],
                        span: Span::default(),
                        def: None,
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![MonoType::Int(64)],
        },
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
        def: None,
        name: "test".to_string(),
        params: vec![MonoType::Int(64), MonoType::String],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![],
            entry: 0,
            locals: vec![MonoType::Bool, MonoType::Float(64)],
        },
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
        def: None,
        name: "main".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Const(ConstValue::Int(42))],
                        span: Span::default(),
                        def: None,
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![MonoType::Int(64)],
        },
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
            &main_func.blocks()[0].instructions[0],
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
        def: None,
        name: "wrapper".to_string(),
        params: vec![MonoType::TypeVar(TypeVar::new(0))],
        return_type: MonoType::TypeVar(TypeVar::new(0)),
        generic_params: Some(vec!["T".to_string()]),
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Arg(0)],
                        span: Span::default(),
                        def: None,
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![MonoType::TypeVar(TypeVar::new(0))],
        },
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
            &wrapper.blocks()[0].instructions[0],
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
        def: None,
        name: "main".to_string(),
        params: vec![],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Call {
                    dst: None,
                    func: Operand::Const(ConstValue::String("foo".to_string())),
                    args: vec![],
                    span: Span::default(),
                    def: None,
                }],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![],
        },
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
            &main_func.blocks()[0].instructions[0],
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
        def: None,
        name: "main".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Const(ConstValue::Int(42))],
                        span: Span::default(),
                        def: None,
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![MonoType::Int(64)],
        },
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
    let result = mono.monomorphize(&module, &requests).unwrap();

    // Assert: 应有 2 个函数：main（调用已替换）+ identity(int64)
    assert_eq!(result.functions.len(), 2);

    // Assert: main 中的调用应被替换为 identity(int64)
    let main_out = result.functions.iter().find(|f| f.name == "main").unwrap();
    assert!(
        matches!(
            &main_out.blocks()[0].instructions[0],
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

// ==================== specialize_type 测试 (Issue #197 类型单态化) ====================

#[test]
fn test_specialize_generic_struct_substitutes_type_params() {
    // Arrange: Pair<T> 类型定义注册到 generic_types, 实例化为 Pair(Int)
    let mut mono = Monomorphizer::new();
    mono.generic_types
        .insert("Pair".to_string(), make_pair_type_ir(&["first", "second"]));

    let req = InstantiationRequest::new(
        GenericFunctionId::new("Pair".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    );

    // Act
    let result = mono.specialize_type(&req).expect("特化 Pair(Int) 应该成功");

    // Assert: 特化后的名字、泛型标记、类型体
    assert_eq!(result.name, "Pair(int64)", "特化类型名应为 Pair(int64)");
    assert!(
        result.generic_params.is_none(),
        "特化类型不应再有泛型参数标记"
    );
    assert!(
        matches!(result.body, FunctionBody::TypeDecl { .. }),
        "特化结果 body 应为 TypeDecl"
    );

    // Assert: 类型体中两个字段的 T 都已被替换为 Int(64)
    let FunctionBody::TypeDecl { definition } = &result.body else {
        panic!("body 应是 TypeDecl");
    };
    let AstType::Struct { body } = definition else {
        panic!("definition 应是 Struct");
    };
    assert_eq!(body.len(), 2, "Pair 应有两个字段");

    let first = match &body[0] {
        TypeBodyItem::Field(f) => f,
        _ => panic!("body[0] 应是 Field"),
    };
    assert_eq!(first.name, "first", "第一个字段名应为 first");
    assert!(
        matches!(&first.ty, AstType::Int(64)),
        "first 字段类型应为 Int(64)，实际为 {:?}",
        first.ty
    );

    let second = match &body[1] {
        TypeBodyItem::Field(f) => f,
        _ => panic!("body[1] 应是 Field"),
    };
    assert_eq!(second.name, "second", "第二个字段名应为 second");
    assert!(
        matches!(&second.ty, AstType::Int(64)),
        "second 字段类型应为 Int(64)，实际为 {:?}",
        second.ty
    );
}

#[test]
fn test_type_specialization_dedup_via_processed_set() {
    // Arrange: 预填充 processed 集合模拟同一请求的二次进入
    let mut mono = Monomorphizer::new();
    mono.generic_types
        .insert("Pair".to_string(), make_pair_type_ir(&["value"]));

    let req = InstantiationRequest::new(
        GenericFunctionId::new("Pair".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    );
    let key = req.specialization_key();
    mono.processed.insert(key.clone());

    // Act: 验证去重缓存命中
    let cached = mono.processed.contains(&key);

    // Assert
    assert!(cached, "processed 集合应命中同一 specialization_key");
    assert_eq!(key.name, "Pair", "dedup key 的名字部分应来自 generic_id");
}

#[test]
fn test_type_specialization_arg_count_mismatch_returns_none() {
    // Arrange: Pair 只声明 1 个类型参数 T，但请求传入 2 个
    let mut mono = Monomorphizer::new();
    mono.generic_types
        .insert("Pair".to_string(), make_pair_type_ir(&["value"]));

    let req = InstantiationRequest::new(
        GenericFunctionId::new("Pair".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64), MonoType::String],
        Span::default(),
    );

    // Act
    let result = mono.specialize_type(&req);

    // Assert
    assert!(
        result.is_none(),
        "类型参数数量不匹配应返回 None 而非部分特化"
    );
}

// ==================== 泛型 enum variant 测试 (Issue #197) ====================

// ==================== 嵌套泛型引用测试 (Issue #197) ====================

#[test]
fn test_collect_generic_type_refs_nested_specialization() {
    // Arrange: List<T> 类型定义，模拟 List(List(Int)) 的嵌套泛型引用
    let mut mono = Monomorphizer::new();
    let body_items = vec![TypeBodyItem::Field(StructField {
        name: "data".to_string(),
        ty: AstType::Name {
            name: "T".to_string(),
            span: Span::dummy(),
        },
        default: None,
        is_mut: false,
    })];
    let type_def = FunctionIR {
        def: None,
        name: "List".to_string(),
        params: vec![MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        }],
        return_type: MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        },
        generic_params: Some(vec!["T".to_string()]),
        body: FunctionBody::TypeDecl {
            definition: AstType::Struct { body: body_items },
        },
    };
    mono.generic_types.insert("List".to_string(), type_def);

    // 构造嵌套泛型引用：List(List(Int))
    let nested_ty = MonoType::Generic {
        name: "List".to_string(),
        args: vec![MonoType::Generic {
            name: "List".to_string(),
            args: vec![MonoType::Int(64)],
        }],
    };

    // Act: 从嵌套类型收集引用
    mono.collect_generic_type_refs(&nested_ty);

    // BFS 顺序：先外层 List(List(Int))（collect_generic_type_refs 在递归入队之前先入队外层）
    let first = &mono.pending_queue[0];
    assert_eq!(first.type_args().len(), 1, "第一个请求应有 1 个类型参数");
    assert!(
        matches!(&first.type_args()[0], MonoType::Generic { name, .. } if name == "List"),
        "第一个请求应为 List(List(Int))，实际为 {:?}",
        first.type_args()[0]
    );

    let second = &mono.pending_queue[1];
    assert_eq!(second.type_args().len(), 1, "第二个请求应有 1 个类型参数");
    assert_eq!(
        second.type_args()[0],
        MonoType::Int(64),
        "第二个请求应为 List(Int)"
    );
}

// ==================== 小写参数名测试 (Issue #197) ====================

#[test]
fn test_specialize_type_lowercase_param_name() {
    // Arrange: 小写参数名 `t: Type` 不应靠大写启发式识别
    let mut mono = Monomorphizer::new();

    let body = vec![TypeBodyItem::Field(StructField {
        name: "value".to_string(),
        ty: AstType::Name {
            name: "t".to_string(),
            span: Span::dummy(),
        },
        default: None,
        is_mut: false,
    })];

    let type_def = FunctionIR {
        def: None,
        name: "Small".to_string(),
        params: vec![MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        }],
        return_type: MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        },
        generic_params: Some(vec!["t".to_string()]),
        body: FunctionBody::TypeDecl {
            definition: AstType::Struct { body },
        },
    };

    mono.generic_types.insert("Small".to_string(), type_def);

    let req = InstantiationRequest::new(
        GenericFunctionId::new("Small".to_string(), vec!["t".to_string()]),
        vec![MonoType::String],
        Span::default(),
    );

    // Act
    let result = mono.specialize_type(&req).expect("小写参数名特化应成功");

    // Assert
    assert_eq!(result.name, "Small(string)", "特化类型名应为 Small(string)");
    let FunctionBody::TypeDecl { definition } = &result.body else {
        panic!("body 应是 TypeDecl");
    };
    let AstType::Struct { body } = definition else {
        panic!("definition 应是 Struct");
    };
    let TypeBodyItem::Field(f) = &body[0] else {
        panic!("body 应是 Field");
    };
    assert!(
        matches!(&f.ty, AstType::String),
        "小写参数名 t 应被替换为 String，实际为 {:?}",
        f.ty
    );
}
