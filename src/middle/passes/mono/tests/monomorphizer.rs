//! Monomorphizer 核心逻辑测试 — 对应 src/middle/passes/mono/mod.rs
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 泛型函数特化
//! RFC-011 §4.4: 泛型类型单态化（Issue #197）
//! 规范: docs/src/rfc/accepted/011-generic-type-system.md
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
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
};
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
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
                        span: Span::dummy(),
                    },
                    Instruction::Ret {
                        value: Some(Operand::Local(0)),
                        span: Span::dummy(),
                    },
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![LocalSlot::temp(param_type.clone())],
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
        return_type: MonoType::make_tuple(vec![t.clone(), t.clone()]),
        generic_params: Some(vec!["T".to_string()]),
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Load {
                        dst: Operand::Local(0),
                        src: Operand::Arg(0),
                        span: Span::dummy(),
                    },
                    Instruction::Load {
                        dst: Operand::Local(1),
                        src: Operand::Arg(1),
                        span: Span::dummy(),
                    },
                    Instruction::Ret {
                        value: Some(Operand::Local(0)),
                        span: Span::dummy(),
                    },
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![LocalSlot::temp(t.clone()), LocalSlot::temp(t.clone())],
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

/// 创建单基本块的函数 IR——`main`/`wrapper` 等测试夹具的公共骨架。
fn make_fn_ir(
    name: &str,
    params: Vec<MonoType>,
    return_type: MonoType,
    generic_params: Option<Vec<String>>,
    instructions: Vec<Instruction>,
    locals: Vec<LocalSlot>,
) -> FunctionIR {
    FunctionIR {
        def: None,
        name: name.to_string(),
        params,
        return_type,
        generic_params,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
            locals,
        },
    }
}

/// 创建 `main` 函数 IR（无参数、非泛型）。
fn make_main_ir(
    return_type: MonoType,
    instructions: Vec<Instruction>,
    locals: Vec<LocalSlot>,
) -> FunctionIR {
    make_fn_ir("main", vec![], return_type, None, instructions, locals)
}

/// 创建只含给定函数的模块 IR。
fn module_with(functions: Vec<FunctionIR>) -> ModuleIR {
    ModuleIR {
        functions,
        ..Default::default()
    }
}

/// 创建默认源码位置的 `Call` 指令（func 为常量函数名）。
fn call_instr(
    dst: Option<Operand>,
    func_name: &str,
    args: Vec<Operand>,
) -> Instruction {
    call_instr_at(dst, func_name, args, Span::default())
}

/// 创建带指定源码位置的 `Call` 指令（func 为常量函数名）。
fn call_instr_at(
    dst: Option<Operand>,
    func_name: &str,
    args: Vec<Operand>,
    span: Span,
) -> Instruction {
    Instruction::Call {
        dst,
        func: Operand::Const(ConstValue::String(func_name.to_string())),
        args,
        span,
        def: None,
    }
}

/// 创建对 `identity` 的调用指令（结果写入 Local(0)，源码位置默认）。
fn identity_call(arg: Operand) -> Instruction {
    call_instr(Some(Operand::Local(0)), "identity", vec![arg])
}

/// 创建 `Ret` 指令（`value` 为 None 时无返回值）。
fn ret_instr(value: Option<Operand>) -> Instruction {
    Instruction::Ret {
        value,
        span: Span::dummy(),
    }
}

/// 创建单类型参数的实例化请求（`params` 为泛型参数名）。
fn request(
    name: &str,
    params: &[&str],
    arg: MonoType,
    span: Span,
) -> InstantiationRequest {
    let params: Vec<String> = params.iter().map(|p| p.to_string()).collect();
    InstantiationRequest::new(
        GenericFunctionId::new(name.to_string(), params),
        vec![arg],
        span,
    )
}

/// 创建对 `identity` 的单类型参数实例化请求（源码位置为默认值）。
fn identity_request(arg: MonoType) -> InstantiationRequest {
    request("identity", &["T"], arg, Span::default())
}

/// `instr` 是否为「调用名为 `expected` 的函数」的 `Call` 指令。
fn is_call_to(
    instr: &Instruction,
    expected: &str,
) -> bool {
    matches!(
        instr,
        Instruction::Call { func: callee, .. }
        if *callee == Operand::Const(ConstValue::String(expected.to_string()))
    )
}

/// 从模块中按名字取出函数。
fn find_fn<'a>(
    module: &'a ModuleIR,
    name: &str,
    msg: &str,
) -> &'a FunctionIR {
    module
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{msg}"))
}

/// 创建 `line` 行上 `start_col`~`end_col` 的源码区间。
fn span_at(
    line: usize,
    start_col: usize,
    start_offset: usize,
    end_col: usize,
    end_offset: usize,
) -> Span {
    Span::new(
        crate::util::span::Position {
            line,
            column: start_col,
            offset: start_offset,
        },
        crate::util::span::Position {
            line,
            column: end_col,
            offset: end_offset,
        },
    )
}

/// `List<inner>` 泛型类型。
fn list_of(inner: MonoType) -> MonoType {
    MonoType::Generic {
        name: "List".to_string(),
        args: vec![inner],
    }
}

/// `TypeRef(name)`——泛型体内引用参数名时的占位类型。
fn type_ref(name: &str) -> MonoType {
    MonoType::TypeRef(name.to_string())
}

/// 把请求登记到 `container` 名下的占位（deferred）桶。
fn defer_request(
    mono: &mut Monomorphizer,
    container: &str,
    req: InstantiationRequest,
) {
    mono.deferred
        .entry(container.to_string())
        .or_default()
        .push(req);
}

/// 创建 `grow<T>` 函数 IR：单基本块内以 `List(T)` 局部变量递归调用自身。
fn make_grow_ir() -> FunctionIR {
    let t = MonoType::TypeVar(TypeVar::new(0));
    make_fn_ir(
        "grow",
        vec![t.clone()],
        t.clone(),
        Some(vec!["T".to_string()]),
        vec![
            call_instr(Some(Operand::Local(1)), "grow", vec![Operand::Local(0)]),
            ret_instr(Some(Operand::Local(1))),
        ],
        vec![LocalSlot::temp(list_of(t))],
    )
}

/// 创建 `main` 函数 IR：两处 `identity` 调用（span 各异）+ 无值 `Ret`。
fn main_with_two_identity_calls(
    span_int: Span,
    span_str: Span,
) -> FunctionIR {
    let calls = vec![
        (span_int, Operand::Const(ConstValue::Int(42)), 0),
        (
            span_str,
            Operand::Const(ConstValue::String("hello".to_string())),
            1,
        ),
    ];
    let locals = vec![
        LocalSlot::temp(MonoType::Int(64)),
        LocalSlot::temp(MonoType::make_string()),
    ];
    let instructions: Vec<Instruction> = calls
        .into_iter()
        .map(|(span, arg, dst)| {
            call_instr_at(Some(Operand::Local(dst)), "identity", vec![arg], span)
        })
        .chain(std::iter::once(ret_instr(None)))
        .collect();
    make_main_ir(MonoType::Void, instructions, locals)
}

/// 创建 `double_identity(int64)` 特化函数 IR：体内按 `inner_span` 嵌套调用 `identity`。
fn make_double_identity_ir(inner_span: Span) -> FunctionIR {
    make_fn_ir(
        "double_identity(int64)",
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        None,
        vec![
            call_instr_at(
                Some(Operand::Local(1)),
                "identity",
                vec![Operand::Arg(0)],
                inner_span,
            ),
            ret_instr(Some(Operand::Local(1))),
        ],
        vec![LocalSlot::temp(MonoType::Int(64))],
    )
}

/// 创建泛型类型定义 IR：`TypeName: (param: Type) -> Type = { fields...: param }`。
fn make_generic_type_decl_ir(
    type_name: &str,
    param: &str,
    fields: &[&str],
) -> FunctionIR {
    let body_items: Vec<TypeBodyItem> = fields
        .iter()
        .map(|name| {
            TypeBodyItem::Field(StructField {
                name: (*name).to_string(),
                ty: AstType::Name {
                    name: param.to_string(),
                    span: Span::dummy(),
                },
                default: None,
                is_mut: false,
            })
        })
        .collect();

    FunctionIR {
        def: None,
        name: type_name.to_string(),
        params: vec![MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        }],
        return_type: MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: Vec::new(),
        },
        generic_params: Some(vec![param.to_string()]),
        body: FunctionBody::TypeDecl {
            definition: AstType::Struct { body: body_items },
        },
    }
}

/// 取出 `FunctionBody::TypeDecl` 中 `Struct` 的字段列表。
fn type_decl_fields(body: &FunctionBody) -> Vec<&StructField> {
    let FunctionBody::TypeDecl { definition } = body else {
        panic!("body 应是 TypeDecl");
    };
    let AstType::Struct { body } = definition else {
        panic!("definition 应是 Struct");
    };
    body.iter()
        .enumerate()
        .map(|(idx, item)| match item {
            TypeBodyItem::Field(f) => f,
            _ => panic!("body[{idx}] 应是 Field"),
        })
        .collect()
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
        func.locals()[0].ty,
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
        vec![MonoType::make_string()],
        Span::default(),
    );

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_some(), "特化应该成功");
    let func = result.unwrap();
    assert_eq!(func.name, "identity(string)");
    assert_eq!(func.params[0], MonoType::make_string());
    assert_eq!(func.return_type, MonoType::make_string());
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
        MonoType::make_tuple(vec![MonoType::Float(64), MonoType::Float(64)])
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
        vec![MonoType::Int(64), MonoType::make_string()],
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
    let generic = make_fn_ir(
        "first",
        vec![MonoType::make_list(t.clone())],
        t,
        Some(vec!["T".to_string()]),
        vec![ret_instr(None)],
        vec![LocalSlot::temp(MonoType::make_list(MonoType::TypeVar(
            TypeVar::new(0),
        )))],
    );
    let mut mono = Monomorphizer::new();
    mono.generic_functions.insert("first".to_string(), generic);
    let req = request("first", &["T"], MonoType::make_string(), Span::default());

    // Act
    let result = mono.specialize_function(&req);

    // Assert
    assert!(result.is_some(), "特化应该成功");
    let func = result.unwrap();
    assert_eq!(func.params[0], MonoType::make_list(MonoType::make_string()));
    assert_eq!(func.return_type, MonoType::make_string());
    assert_eq!(
        func.locals()[0].ty,
        MonoType::make_list(MonoType::make_string())
    );
}

// ==================== #335 路径 A：fire_deferred / 占位求值测试 ====================

/// 占位请求（TypeRef(参数名) 实参）在所在泛型函数特化时按 name_map 求值入队
#[test]
fn test_fire_deferred_evaluates_placeholder_args() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    // 占位请求：g 的实参为所在泛型函数 f 的参数名 T（TypeRef 形态）
    let req = InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::TypeRef("T".to_string())],
        Span::default(),
    );
    mono.deferred.entry("f".to_string()).or_default().push(req);

    // name_map：f 特化时 T → int64
    let name_map: std::collections::HashMap<String, MonoType> =
        [("T".to_string(), MonoType::Int(64))].into_iter().collect();

    // Act
    mono.fire_deferred("f", "f(int64)", &name_map, 1);

    // Assert：求值后的请求入队，占位被替换为具体类型
    assert_eq!(mono.pending_queue.len(), 1, "占位请求应求值入队");
    let fired = mono.pending_queue.front().unwrap();
    assert_eq!(
        fired.type_args,
        vec![MonoType::Int(64)],
        "TypeRef(\"T\") 占位应被 name_map 求值为 int64"
    );
    assert_eq!(fired.containing_fn.as_deref(), Some("f(int64)"));
    assert_eq!(fired.depth, 1);
}

/// 无占位桶的容器：fire_deferred 无操作
#[test]
fn test_fire_deferred_unknown_container_noop() {
    let mut mono = Monomorphizer::new();
    let name_map: std::collections::HashMap<String, MonoType> =
        [("T".to_string(), MonoType::Int(64))].into_iter().collect();

    mono.fire_deferred("nonexistent", "f(int64)", &name_map, 1);

    assert!(mono.pending_queue.is_empty(), "无桶不应产生请求");
}

// ==================== replace_call_sites 测试 ====================

#[test]
fn test_replace_call_sites_replaces_generic_call_in_main() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let main_func = make_main_ir(
        MonoType::Void,
        vec![
            identity_call(Operand::Const(ConstValue::Int(42))),
            ret_instr(Some(Operand::Local(0))),
        ],
        vec![LocalSlot::temp(MonoType::Int(64))],
    );
    let mut module = module_with(vec![main_func]);
    mono.site_requests = vec![identity_request(MonoType::Int(64))];

    // Act
    mono.replace_call_sites(&mut module);

    // Assert
    let main_func = &module.functions[0];
    assert!(
        is_call_to(&main_func.blocks()[0].instructions[0], "identity(int64)"),
        "Call 指令的 func 应该被替换为特化函数名 identity(int64)"
    );
}

#[test]
fn test_replace_call_sites_skips_generic_functions() {
    // Arrange
    let mut mono = Monomorphizer::new();

    let wrapper_func = make_fn_ir(
        "wrapper",
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
        Some(vec!["T".to_string()]),
        vec![
            call_instr(Some(Operand::Local(0)), "identity", vec![Operand::Arg(0)]),
            ret_instr(Some(Operand::Local(0))),
        ],
        vec![LocalSlot::temp(MonoType::TypeVar(TypeVar::new(0)))],
    );
    let mut module = module_with(vec![wrapper_func]);
    mono.site_requests = vec![identity_request(MonoType::Int(64))];

    // Act
    mono.replace_call_sites(&mut module);

    // Assert
    let wrapper = &module.functions[0];
    assert!(
        is_call_to(&wrapper.blocks()[0].instructions[0], "identity"),
        "泛型函数内的调用不应被替换"
    );
}

#[test]
fn test_replace_call_sites_no_matching_request_does_not_replace() {
    // Arrange
    let mut mono = Monomorphizer::new();

    let main_func = make_main_ir(
        MonoType::Void,
        vec![call_instr(None, "foo", vec![])],
        vec![],
    );
    let mut module = module_with(vec![main_func]);
    mono.site_requests = vec![identity_request(MonoType::Int(64))];

    // Act
    mono.replace_call_sites(&mut module);

    // Assert
    let main_func = &module.functions[0];
    assert!(
        is_call_to(&main_func.blocks()[0].instructions[0], "foo"),
        "不匹配的调用不应被替换"
    );
}

// ==================== monomorphize 端到端测试 ====================

#[test]
fn test_monomorphize_end_to_end_specializes_and_replaces_calls() {
    // Arrange
    let identity = make_identity_ir();
    let main_func = make_main_ir(
        MonoType::Int(64),
        vec![
            identity_call(Operand::Const(ConstValue::Int(42))),
            ret_instr(Some(Operand::Local(0))),
        ],
        vec![LocalSlot::temp(MonoType::Int(64))],
    );
    let module = module_with(vec![identity, main_func]);
    let mut mono = Monomorphizer::new();
    let requests = [identity_request(MonoType::Int(64))];

    // Act
    let result = mono.monomorphize(&module, &requests).unwrap();

    // Assert: 应有 2 个函数：main（调用已替换）+ identity(int64)
    assert_eq!(result.functions.len(), 2);

    // Assert: main 中的调用应被替换为 identity(int64)
    let main_out = find_fn(&result, "main", "模块中应存在 main 函数");
    assert!(
        is_call_to(&main_out.blocks()[0].instructions[0], "identity(int64)"),
        "main 中的调用应被替换为 identity(int64)"
    );

    // Assert: 特化函数存在且泛型标记已清除
    let specialized = find_fn(&result, "identity(int64)", MISSING_SPECIALIZED);
    assert!(
        specialized.generic_params.is_none(),
        "特化函数的泛型标记应已清除"
    );
}

/// 「identity(int64) 特化函数应存在」的失败文案。
const MISSING_SPECIALIZED: &str = "应该存在 identity(int64) 特化函数";

/// 机制（3.4.8-R1+ / t3）：删除判据按**泛型名**（site_requests 的名字集合），
/// 调用点改写判据按 **(containing_fn, 名, span)** 三元键。同一泛型的两个调用点
/// 里只要有一个没有请求键（请求因符号化实参进 deferred 桶未排水，或该调用点
/// 从未产出请求），删除原件就让那个调用点悬空——运行期按原名查表落空（E6006）。
///
/// 规范：RFC-011 §4 泛型函数特化。实证现场：src/std/tests/list_ops.yx:53 的
/// `list.is_empty(list.empty(Int))`——该泛型的请求只存在于同一函数内的另一调用点。
#[test]
fn test_monomorphize_keeps_original_generic_when_call_site_lacks_request_key() {
    // Arrange：泛型 std.list.is_empty + main 里两个调用点（仅前者有请求键）
    let requested_span = span_at(10, 5, 100, 12, 107);
    let unrequested_span = span_at(20, 5, 200, 12, 207);
    let generic = make_fn_ir(
        "std.list.is_empty",
        vec![MonoType::Generic {
            name: "Vec".to_string(),
            args: vec![MonoType::TypeVar(TypeVar::new(0))],
        }],
        MonoType::Bool,
        Some(vec!["A".to_string()]),
        vec![ret_instr(Some(Operand::Const(ConstValue::Bool(true))))],
        vec![],
    );
    let main_func = make_main_ir(
        MonoType::Void,
        vec![
            call_instr_at(None, "std.list.is_empty", vec![], requested_span),
            call_instr_at(None, "std.list.is_empty", vec![], unrequested_span),
            ret_instr(None),
        ],
        vec![],
    );
    let module = module_with(vec![generic, main_func]);
    let mut covered_request = request(
        "std.list.is_empty",
        &["A"],
        MonoType::Int(64),
        requested_span,
    );
    covered_request.containing_fn = Some("main".to_string());
    let requests = [covered_request];

    // Act
    let result = Monomorphizer::new()
        .monomorphize(&module, &requests)
        .expect("单态化不应失败");

    // Assert 1：有请求键的调用点改写成特化名
    let main_out = find_fn(&result, "main", "模块中应存在 main 函数");
    assert!(
        is_call_to(
            &main_out.blocks()[0].instructions[0],
            "std.list.is_empty(int64)"
        ),
        "有请求键的调用点应改写成特化名 std.list.is_empty(int64)"
    );
    // Assert 2：无请求键的调用点保持原名（该调用点不在改写映射里）
    assert!(
        is_call_to(&main_out.blocks()[0].instructions[1], "std.list.is_empty"),
        "无请求键的调用点应保持原名 std.list.is_empty"
    );
    // Assert 3：原件必须保留——删除会让 Assert 2 的调用点悬空（运行期 E6006）
    assert!(
        result
            .functions
            .iter()
            .any(|f| f.name == "std.list.is_empty"),
        "存在未覆盖调用点时不得删除原件（否则运行期按原名查表落空 E6006）"
    );
}

/// 三个泛型原件（`std.list.is_empty` / `.len` / `.pop`）各带一个「有请求键」与一个
/// 「无请求键」调用点——三者都会被 t3 闭合闸放回，用于检验放回段的顺序纪律。
fn restore_three_generics_case() -> (ModuleIR, Vec<InstantiationRequest>) {
    let mut functions = Vec::new();
    let mut instructions = Vec::new();
    let mut requests = Vec::new();
    for (name, covered_span, uncovered_span) in [
        (
            "std.list.is_empty",
            span_at(10, 5, 100, 12, 107),
            span_at(11, 5, 120, 12, 127),
        ),
        (
            "std.list.len",
            span_at(20, 5, 200, 12, 207),
            span_at(21, 5, 220, 12, 227),
        ),
        (
            "std.list.pop",
            span_at(30, 5, 300, 12, 307),
            span_at(31, 5, 320, 12, 327),
        ),
    ] {
        functions.push(make_fn_ir(
            name,
            vec![MonoType::Generic {
                name: "Vec".to_string(),
                args: vec![MonoType::TypeVar(TypeVar::new(0))],
            }],
            MonoType::Bool,
            Some(vec!["A".to_string()]),
            vec![ret_instr(Some(Operand::Const(ConstValue::Bool(true))))],
            vec![],
        ));
        instructions.push(call_instr_at(None, name, vec![], covered_span));
        instructions.push(call_instr_at(None, name, vec![], uncovered_span));
        let mut req = request(name, &["A"], MonoType::Int(64), covered_span);
        req.containing_fn = Some("main".to_string());
        requests.push(req);
    }
    instructions.push(ret_instr(None));
    functions.push(make_main_ir(MonoType::Void, instructions, vec![]));
    (module_with(functions), requests)
}

/// 输出里「被放回的原件」的名字序列（特化副本名带 `(...)` 后缀，故同名者必为原件）。
fn restored_name_sequence(module: &ModuleIR) -> Vec<String> {
    const ORIGINALS: [&str; 3] = ["std.list.is_empty", "std.list.len", "std.list.pop"];
    module
        .functions
        .iter()
        .map(|f| f.name.clone())
        .filter(|n| ORIGINALS.contains(&n.as_str()))
        .collect()
}

/// 顺序纪律（3.4.8-R1++ / F1）：闭合闸放回的原件必须**按函数名排序**追加，与
/// `build_output` 的特化函数追加同款（mod.rs 明文：「HashMap 迭代顺序随进程随机化…
/// 按名排序固定顺序」）。按 `HashSet` 迭代序追加会让同一份源码产出不同的函数
/// 序号（.42 字节不可复现、dump 不可复现）。
#[test]
fn test_restore_gate_orders_restored_generics_by_name() {
    // Arrange：三个泛型原件都因「存在未覆盖调用点」被放回
    let (module, requests) = restore_three_generics_case();

    // Act
    let result = Monomorphizer::new()
        .monomorphize(&module, &requests)
        .expect("单态化不应失败");

    // Assert：强形式——顺序即契约，不依赖 HashSet 迭代序
    assert_eq!(
        restored_name_sequence(&result),
        vec!["std.list.is_empty", "std.list.len", "std.list.pop"],
        "放回的原件必须按函数名排序追加（对齐 build_output 的排序纪律）"
    );
}

/// 跨次可复现（3.4.8-R1++ / F1）：同一进程内对同一输入多次单态化，输出函数表顺序
/// 必须逐元素一致——否则同一源码的函数序号随 HashSet 随机种子漂移。
#[test]
fn test_restore_gate_output_order_is_stable_across_runs() {
    // Arrange：同一输入；≥2 次（取 8 次以消除 HashSet 顺序偶然一致的判别力缺口）
    let (module, requests) = restore_three_generics_case();
    let mut orders: Vec<Vec<String>> = Vec::new();

    // Act
    for _ in 0..8 {
        let out = Monomorphizer::new()
            .monomorphize(&module, &requests)
            .expect("单态化不应失败");
        orders.push(out.functions.iter().map(|f| f.name.clone()).collect());
    }

    // Assert：逐元素一致
    let first = &orders[0];
    for (i, order) in orders.iter().enumerate() {
        assert_eq!(
            order, first,
            "第 {i} 次单态化的函数表顺序与首次不一致：{order:?} vs {first:?}"
        );
    }
}

/// 泛型 `std.list.is_empty` **只**被一个「非改写面」站点（`MakeClosure` / `TailCall`）
/// 按名引用，且该站点在请求集合里有键（span 对齐）——用于钉「非改写面站点一律计
/// 未覆盖」。返回（模块, 请求）。
fn non_rewritable_ref_case(
    reference: Instruction,
    reference_span: Span,
) -> (ModuleIR, Vec<InstantiationRequest>) {
    let generic = make_fn_ir(
        "std.list.is_empty",
        vec![MonoType::Generic {
            name: "Vec".to_string(),
            args: vec![MonoType::TypeVar(TypeVar::new(0))],
        }],
        MonoType::Bool,
        Some(vec!["A".to_string()]),
        vec![ret_instr(Some(Operand::Const(ConstValue::Bool(true))))],
        vec![],
    );
    let main_func = make_main_ir(
        MonoType::Void,
        vec![reference, ret_instr(None)],
        vec![LocalSlot::temp(MonoType::Bool)],
    );
    let mut req = request(
        "std.list.is_empty",
        &["A"],
        MonoType::Int(64),
        reference_span,
    );
    req.containing_fn = Some("main".to_string());
    (module_with(vec![generic, main_func]), vec![req])
}

/// 指令形态不对称（3.4.8-R1+++ / t6）：扫描面（`by_name_call_target`）覆盖
/// `Call`/`TailCall`/`MakeClosure`，改写面（`replace_calls_in_function`）**只**处理
/// `Call`。`MakeClosure` 臂若照常查映射键，「有键」会被误判为「已覆盖」——原件被删
/// 而该站点永不被改写 → 悬空（t3 同缺陷类）。钉住：一律计未覆盖 → 原件保留。
#[test]
fn test_restore_gate_keeps_generic_referenced_only_by_make_closure() {
    // Arrange：仅 MakeClosure 站点按名引用，且该站点有对齐的请求键
    let reference_span = span_at(10, 5, 100, 12, 107);
    let reference = Instruction::MakeClosure {
        dst: Operand::Local(0),
        func: "std.list.is_empty".to_string(),
        def: None,
        env: vec![],
        span: reference_span,
    };
    let (module, requests) = non_rewritable_ref_case(reference, reference_span);

    // Act
    let result = Monomorphizer::new()
        .monomorphize(&module, &requests)
        .expect("单态化不应失败");

    // Assert：原件保留（MakeClosure 站点不会被改写，删除即悬空）
    assert!(
        result
            .functions
            .iter()
            .any(|f| f.name == "std.list.is_empty"),
        "MakeClosure 按名目标一律计未覆盖：原件必须保留（修复前「有键→判覆盖→被删」）"
    );
}

/// 同上缺陷类：`TailCall` 也不在改写面内，同样一律计未覆盖（注释早已如此声明，
/// 修复前代码却仍查映射键）。钉住行为与声明一致。
#[test]
fn test_restore_gate_keeps_generic_referenced_only_by_tail_call() {
    // Arrange：仅 TailCall 站点按名引用，且该站点有对齐的请求键
    let reference_span = span_at(20, 5, 200, 12, 207);
    let reference = Instruction::TailCall {
        func: Operand::Const(ConstValue::String("std.list.is_empty".to_string())),
        args: vec![],
        def: None,
        span: reference_span,
    };
    let (module, requests) = non_rewritable_ref_case(reference, reference_span);

    // Act
    let result = Monomorphizer::new()
        .monomorphize(&module, &requests)
        .expect("单态化不应失败");

    // Assert：原件保留（TailCall 站点不会被改写，删除即悬空）
    assert!(
        result
            .functions
            .iter()
            .any(|f| f.name == "std.list.is_empty"),
        "TailCall 按名目标一律计未覆盖：原件必须保留（修复前「有键→判覆盖→被删」）"
    );
}

// ==================== specialize_type 测试 (Issue #197 类型单态化) ====================

#[test]
fn test_specialize_generic_struct_substitutes_type_params() {
    // Arrange: Pair<T> 类型定义注册到 generic_types, 实例化为 Pair(Int)
    let mut mono = Monomorphizer::new();
    mono.generic_types
        .insert("Pair".to_string(), make_pair_type_ir(&["first", "second"]));
    let req = request("Pair", &["T"], MonoType::Int(64), Span::default());

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
    let fields = type_decl_fields(&result.body);
    assert_eq!(fields.len(), 2, "Pair 应有两个字段");
    assert_eq!(fields[0].name, "first", "第一个字段名应为 first");
    assert!(
        matches!(&fields[0].ty, AstType::Int(64)),
        "first 字段类型应为 Int(64)，实际为 {:?}",
        fields[0].ty
    );
    assert_eq!(fields[1].name, "second", "第二个字段名应为 second");
    assert!(
        matches!(&fields[1].ty, AstType::Int(64)),
        "second 字段类型应为 Int(64)，实际为 {:?}",
        fields[1].ty
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
        vec![MonoType::Int(64), MonoType::make_string()],
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
    mono.generic_types.insert(
        "List".to_string(),
        make_generic_type_decl_ir("List", "T", &["data"]),
    );

    // 构造嵌套泛型引用：List(List(Int))
    let nested_ty = list_of(list_of(MonoType::Int(64)));

    // Act: 从嵌套类型收集引用
    mono.collect_generic_type_refs(&nested_ty, 0);

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
    // Arrange: 小写参数名 t: Type 不应靠大写启发式识别
    let mut mono = Monomorphizer::new();
    mono.generic_types.insert(
        "Small".to_string(),
        make_generic_type_decl_ir("Small", "t", &["value"]),
    );
    let req = request("Small", &["t"], MonoType::make_string(), Span::default());

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

// ==================== #335 多实例化按调用点分发 ====================

/// #335 回归：同一泛型函数的两个实例化，各调用点必须改写到各自特化版本。
/// 旧实现按泛型名单键映射，后写覆盖前写——两处调用都会指向最后一个请求
/// 的特化（字节码实证：两处 CallStatic func_id 相同）。
#[test]
fn test_replace_call_sites_multi_instantiation_dispatches_per_site() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    // 两个调用点：不同源码位置（span 不同）
    let span_int = span_at(3, 9, 40, 22, 53);
    let span_str = span_at(4, 9, 60, 25, 76);
    let main_func = main_with_two_identity_calls(span_int, span_str);
    let mut module = module_with(vec![main_func]);

    // 两个实例化请求：各来自不同调用点（span 对应）
    mono.site_requests = vec![
        request("identity", &["T"], MonoType::Int(64), span_int),
        request("identity", &["T"], MonoType::make_string(), span_str),
    ];

    // Act
    mono.replace_call_sites(&mut module);

    // Assert：每个调用点改写到各自的特化，不再互相覆盖
    let instrs = &module.functions[0].blocks()[0].instructions;
    assert!(
        is_call_to(&instrs[0], "identity(int64)"),
        "第一处调用应特化为 identity(int64)"
    );
    assert!(
        is_call_to(&instrs[1], "identity(string)"),
        "第二处调用应特化为 identity(string)，不得被第一处覆盖"
    );
}

/// #335：嵌套泛型调用（特化体内的泛型调用）按自身 span 改写
#[test]
fn test_replace_call_sites_rewrites_nested_calls_in_specialized_body() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    // 特化函数（generic_params 已清除）体内对 identity 的嵌套调用，
    // span 与嵌套请求的 source_location 同源
    let inner_span = span_at(2, 30, 31, 42, 43);
    let wrapper = make_double_identity_ir(inner_span);
    let mut module = module_with(vec![wrapper]);
    mono.site_requests = vec![request("identity", &["T"], MonoType::Int(64), inner_span)];

    // Act
    mono.replace_call_sites(&mut module);

    // Assert
    let instrs = &module.functions[0].blocks()[0].instructions;
    assert!(
        is_call_to(&instrs[0], "identity(int64)"),
        "特化体内嵌套调用应按 (泛型名, span) 命中改写"
    );
}

// ==================== #335 深度/规模分离 ====================

/// #335：合法大规模泛型（150 个互不相同的实例化）必须编译通过。
/// 旧实现 depth 计数器混用「已处理总数」，超过 max_depth(100) 即误报
/// 无限递归。
#[test]
fn test_scale_over_hundred_instantiations_compiles() {
    // Arrange
    let mut mono = Monomorphizer::new();
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let main_func = make_main_ir(
        MonoType::Void,
        vec![
            identity_call(Operand::Const(ConstValue::Int(1))),
            ret_instr(None),
        ],
        vec![LocalSlot::temp(MonoType::Int(64))],
    );
    let module = module_with(vec![main_func]);

    // 150 个互不相同的实例化请求（Int(1)..Int(150) 宽度作区分维度）
    let requests: Vec<InstantiationRequest> = (1..=150)
        .map(|n| identity_request(MonoType::Int(n)))
        .collect();

    // Act
    let result = mono.monomorphize(&module, &requests);

    // Assert：全部实例化成功，超过旧 max_depth(100) 不再误报
    let output = result.expect("150 个不同实例化应全部成功");
    let specialized = output
        .functions
        .iter()
        .filter(|f| f.name.starts_with("identity("))
        .count();
    assert_eq!(specialized, 150, "应生成 150 个特化版本");
}

/// #335：真正的无限类型增长递归仍被深度上限拦截——
/// grow(T) 的体内以 List(T) 类型的局部变量递归调用自身，
/// 实例化链 grow(Int) → grow(List(Int)) → grow(List(List(Int))) → …
/// 类型逐层增长永不重复，规模上限管不住，必须由链深保护拦截。
#[test]
fn test_type_growing_recursion_still_blocked_by_depth() {
    // Arrange
    let mut mono = Monomorphizer::with_max_depth(5);
    mono.generic_functions
        .insert("grow".to_string(), make_grow_ir());

    let mut requests = vec![request("grow", &["T"], MonoType::Int(64), Span::default())];
    // 路径 A：grow 体内对自身（实参 List(T)）的调用由 typecheck 记录为
    // 占位请求（实参含 TypeRef("T")），挂在 grow 名下随每次特化求值
    requests[0].containing_fn = Some("grow".to_string());
    let placeholder = request("grow", &["T"], list_of(type_ref("T")), Span::default());
    defer_request(&mut mono, "grow", placeholder);

    let module = module_with(Vec::new());

    // Act
    let result = mono.monomorphize(&module, &requests);

    // Assert：链深超过 5 被拦（此时已处理实例化数远小于规模上限 10000）
    let err = result.expect_err("无限类型增长递归必须被深度保护拦截");
    assert!(
        err.message.contains("深度") || err.message.contains("递归"),
        "错误应指向递归链深度，实际: {}",
        err.message
    );
}

/// #335：实例化总数上限独立生效
#[test]
fn test_scale_cap_independent_of_depth() {
    // Arrange
    let mut mono = Monomorphizer::with_max_depth(100).with_max_instantiations(10);
    mono.generic_functions
        .insert("identity".to_string(), make_identity_ir());

    let module = ModuleIR {
        functions: Vec::new(),
        ..Default::default()
    };
    let requests: Vec<InstantiationRequest> = (1..=20)
        .map(|n| {
            InstantiationRequest::new(
                GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
                vec![MonoType::Int(n)],
                Span::default(),
            )
        })
        .collect();

    // Act
    let result = mono.monomorphize(&module, &requests);

    // Assert：每条请求 depth 都是 0（链深不超限），超的是总数上限
    let err = result.expect_err("超过总数上限必须报错");
    assert!(
        err.message.contains("总数") || err.message.contains("上限"),
        "错误应指向实例化总数上限，实际: {}",
        err.message
    );
}
