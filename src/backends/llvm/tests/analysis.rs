//! AOT 分析层单元测试 — 基于 RFC-018 与 `llvm-aot-execution.md` 阶段 1
//!
//! 覆盖 `ValueInfo.ty` 值证据、显式未知/不支持类型、`verify_ssa` 入口与
//! `Phi` 的 AOT-3.5 边界；目录模块按 test-specification 1.1/1.4 声明。

use crate::backends::llvm::analysis::analyze_module;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand, ValueId,
    ValueInfo,
};
use crate::util::span::Span;

fn base_function(
    return_type: MonoType,
    values: Vec<ValueInfo>,
    instructions: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: "f".to_string(),
        def: None,
        params: vec![],
        return_type,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: vec![],
            }],
            entry: 0,
            locals: vec![],
            values,
            value_slots: vec![],
        },
    }
}

fn module_with(function: FunctionIR) -> ModuleIR {
    let mut module = ModuleIR::default();
    module.functions.push(function);
    module.init.push(Instruction::Call {
        dst: None,
        func: Operand::Const(ConstValue::String("f".to_string())),
        args: vec![],
        def: None,
        span: Span::dummy(),
    });
    module
}

fn load_value(
    id: u32,
    src: Operand,
) -> Instruction {
    Instruction::Load {
        dst: Operand::Value(ValueId(id)),
        src,
        span: Span::dummy(),
    }
}

fn ret_value(id: u32) -> Instruction {
    Instruction::Ret {
        value: Some(Operand::Value(ValueId(id))),
        span: Span::dummy(),
    }
}

#[test]
fn test_native_value_type_known_scalar_passes() {
    // Arrange
    let function = base_function(
        MonoType::Int(64),
        vec![ValueInfo {
            ty: MonoType::Int(64),
            debug_name: None,
            defining_block: 0,
        }],
        vec![
            load_value(0, Operand::Const(ConstValue::Int(1))),
            ret_value(0),
        ],
    );
    let module = module_with(function);

    // Act
    let analysis =
        analyze_module(&module, "value_known.yx").expect("known scalar value should be accepted");

    // Assert
    assert_eq!(
        analysis.functions["f"].values[0],
        MonoType::Int(64),
        "known value type must be carried from ValueInfo"
    );
}

#[test]
fn test_native_explicit_unknown_value_is_rejected() {
    // Arrange
    let function = base_function(
        MonoType::Int(64),
        vec![ValueInfo {
            ty: MonoType::TypeRef("?".to_string()),
            debug_name: None,
            defining_block: 0,
        }],
        vec![
            load_value(0, Operand::Const(ConstValue::Int(1))),
            ret_value(0),
        ],
    );
    let module = module_with(function);

    // Act
    let error = analyze_module(&module, "value_unknown.yx")
        .err()
        .expect("explicit unknown value type must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("ValueId(0)") && detail.contains("TypeRef(\"?\")"),
        "diagnostic must include value id and original type: {detail}"
    );
}

#[test]
fn test_native_unsupported_value_type_is_rejected() {
    // Arrange
    let function = base_function(
        MonoType::Int(64),
        vec![ValueInfo {
            ty: MonoType::Float(64),
            debug_name: None,
            defining_block: 0,
        }],
        vec![
            load_value(0, Operand::Const(ConstValue::Int(1))),
            ret_value(0),
        ],
    );
    let module = module_with(function);

    // Act
    let error = analyze_module(&module, "value_unsupported.yx")
        .err()
        .expect("unsupported value type must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("ValueId(0)") && detail.contains("Float(64)"),
        "diagnostic must include value id and unsupported type: {detail}"
    );
}

#[test]
fn test_native_multi_definition_value_is_rejected_by_verify_ssa() {
    // Arrange
    let function = base_function(
        MonoType::Int(64),
        vec![ValueInfo {
            ty: MonoType::Int(64),
            debug_name: None,
            defining_block: 0,
        }],
        vec![
            load_value(0, Operand::Const(ConstValue::Int(1))),
            load_value(0, Operand::Const(ConstValue::Int(2))),
            ret_value(0),
        ],
    );
    let module = module_with(function);

    // Act
    let error = analyze_module(&module, "value_double.yx")
        .err()
        .expect("multi-definition value must fail SSA entry verification");

    // Assert
    assert!(
        error.to_string().contains("verification"),
        "diagnostic must identify SSA verification: {error}"
    );
}

#[test]
fn test_native_phi_is_rejected_with_batch_35_boundary() {
    // Arrange: block 0 jumps to block 1, which contains a valid Phi.
    let function = FunctionIR {
        name: "f".to_string(),
        def: None,
        params: vec![],
        return_type: MonoType::Int(64),
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![
                BasicBlock {
                    label: 0,
                    instructions: vec![Instruction::Jmp {
                        target: 1,
                        span: Span::dummy(),
                    }],
                    successors: vec![1],
                },
                BasicBlock {
                    label: 1,
                    instructions: vec![
                        Instruction::Phi {
                            dst: Operand::Value(ValueId(0)),
                            incoming: vec![(0, Operand::Const(ConstValue::Int(1)))],
                            span: Span::dummy(),
                        },
                        ret_value(0),
                    ],
                    successors: vec![],
                },
            ],
            entry: 0,
            locals: vec![],
            values: vec![ValueInfo {
                ty: MonoType::Int(64),
                debug_name: None,
                defining_block: 1,
            }],
            value_slots: vec![],
        },
    };
    let module = module_with(function);

    // Act
    let error = analyze_module(&module, "phi.yx")
        .err()
        .expect("Phi must remain outside AOT-3.1");

    // Assert
    assert!(
        error.to_string().contains("3.5"),
        "diagnostic must name the AOT-3.5 boundary: {error}"
    );
}

fn local_merge_module(two_arms: bool) -> ModuleIR {
    let condition = Instruction::JmpIfNot {
        cond: Operand::Const(ConstValue::Bool(true)),
        target: 3,
        span: Span::dummy(),
    };
    let write = |value: i128| Instruction::Load {
        dst: Operand::Local(0),
        src: Operand::Const(ConstValue::Int(value)),
        span: Span::dummy(),
    };
    let jump = |target: usize| Instruction::Jmp {
        target,
        span: Span::dummy(),
    };
    let read = Instruction::Load {
        dst: Operand::Value(ValueId(0)),
        src: Operand::Local(0),
        span: Span::dummy(),
    };
    let ret = Instruction::Ret {
        value: None,
        span: Span::dummy(),
    };
    let blocks = if two_arms {
        vec![
            BasicBlock {
                label: 0,
                instructions: vec![condition],
                successors: vec![1, 2],
            },
            BasicBlock {
                label: 1,
                instructions: vec![write(42), jump(5)],
                successors: vec![3],
            },
            BasicBlock {
                label: 2,
                instructions: vec![write(0), jump(5)],
                successors: vec![3],
            },
            BasicBlock {
                label: 3,
                instructions: vec![read, ret],
                successors: vec![],
            },
        ]
    } else {
        vec![
            BasicBlock {
                label: 0,
                instructions: vec![condition],
                successors: vec![1, 2],
            },
            BasicBlock {
                label: 1,
                instructions: vec![write(42), jump(3)],
                successors: vec![2],
            },
            BasicBlock {
                label: 2,
                instructions: vec![read, ret],
                successors: vec![],
            },
        ]
    };
    let function = FunctionIR {
        name: "merge".to_string(),
        def: None,
        params: vec![],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks,
            entry: 0,
            locals: vec![crate::middle::ir::ir::LocalSlot::temp(MonoType::Int(64))],
            values: vec![ValueInfo {
                ty: MonoType::Int(64),
                debug_name: None,
                defining_block: if two_arms { 3 } else { 2 },
            }],
            value_slots: vec![],
        },
    };
    let mut module = ModuleIR::default();
    module.functions.push(function);
    module.init.push(Instruction::Call {
        dst: None,
        func: Operand::Const(ConstValue::String("merge".to_string())),
        args: vec![],
        def: None,
        span: Span::dummy(),
    });
    module
}

fn local_sequential_module() -> ModuleIR {
    let function = FunctionIR {
        name: "sequential".to_string(),
        def: None,
        params: vec![],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Load {
                        dst: Operand::Local(0),
                        src: Operand::Const(ConstValue::Int(1)),
                        span: Span::dummy(),
                    },
                    Instruction::Load {
                        dst: Operand::Value(ValueId(0)),
                        src: Operand::Local(0),
                        span: Span::dummy(),
                    },
                    Instruction::Ret {
                        value: None,
                        span: Span::dummy(),
                    },
                ],
                successors: vec![],
            }],
            entry: 0,
            locals: vec![crate::middle::ir::ir::LocalSlot::temp(MonoType::Int(64))],
            values: vec![ValueInfo {
                ty: MonoType::Int(64),
                debug_name: None,
                defining_block: 0,
            }],
            value_slots: vec![],
        },
    };
    let mut module = ModuleIR::default();
    module.functions.push(function);
    module.init.push(Instruction::Call {
        dst: None,
        func: Operand::Const(ConstValue::String("sequential".to_string())),
        args: vec![],
        def: None,
        span: Span::dummy(),
    });
    module
}

fn local_loop_module() -> ModuleIR {
    let function = FunctionIR {
        name: "loop".to_string(),
        def: None,
        params: vec![],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![
                BasicBlock {
                    label: 0,
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Local(0),
                            src: Operand::Const(ConstValue::Int(0)),
                            span: Span::dummy(),
                        },
                        Instruction::Jmp {
                            target: 2,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![1],
                },
                BasicBlock {
                    label: 1,
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Local(0),
                            src: Operand::Const(ConstValue::Int(1)),
                            span: Span::dummy(),
                        },
                        Instruction::Load {
                            dst: Operand::Value(ValueId(0)),
                            src: Operand::Local(0),
                            span: Span::dummy(),
                        },
                        Instruction::Jmp {
                            target: 2,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![1],
                },
            ],
            entry: 0,
            locals: vec![crate::middle::ir::ir::LocalSlot::temp(MonoType::Int(64))],
            values: vec![ValueInfo {
                ty: MonoType::Int(64),
                debug_name: None,
                defining_block: 1,
            }],
            value_slots: vec![],
        },
    };
    let mut module = ModuleIR::default();
    module.functions.push(function);
    module.init.push(Instruction::Call {
        dst: None,
        func: Operand::Const(ConstValue::String("loop".to_string())),
        args: vec![],
        def: None,
        span: Span::dummy(),
    });
    module
}

#[test]
fn test_native_local_merge_boundary_rejects_multi_predecessor_write() {
    // Arrange
    let module = local_merge_module(true);

    // Act
    let error = analyze_module(&module, "merge_multi.yx")
        .err()
        .expect("multi-predecessor Local merge must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("AOT-3.5") && detail.contains("Local(0)"),
        "multi-arm Local merge diagnostic must name AOT-3.5 and Local: {detail}"
    );
}

#[test]
fn test_native_local_merge_boundary_rejects_single_arm_write() {
    // Arrange
    let module = local_merge_module(false);

    // Act
    let error = analyze_module(&module, "merge_single.yx")
        .err()
        .expect("single-arm Local write must not dominate a merged read");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("AOT-3.5") && detail.contains("Local(0)"),
        "single-arm Local merge diagnostic must name AOT-3.5 and Local: {detail}"
    );
}

#[test]
fn test_native_local_sequential_write_is_allowed() {
    // Arrange
    let module = local_sequential_module();

    // Act
    let analysis = analyze_module(&module, "sequential.yx")
        .expect("a dominating sequential Local write should be accepted");

    // Assert
    assert!(
        analysis.functions.contains_key("sequential"),
        "sequential Local function should be analyzed"
    );
}

#[test]
fn test_native_local_loop_repeated_write_is_allowed() {
    // Arrange
    let module = local_loop_module();

    // Act
    let analysis = analyze_module(&module, "loop_write.yx")
        .expect("loop back-edge writes reusing a dominating slot should be accepted");

    // Assert
    assert!(
        analysis.functions.contains_key("loop"),
        "loop Local function should be analyzed"
    );
}
