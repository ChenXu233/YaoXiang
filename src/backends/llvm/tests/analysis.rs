//! AOT 分析层单元测试 — 基于 RFC-018 与 `llvm-aot-execution.md` 阶段 1
//!
//! 覆盖 `ValueInfo.ty` 值证据、显式未知/不支持类型、`verify_ssa` 入口、
//! `Phi` 的类型证据与 incoming 判定，以及批次 3.5 收窄后的 `Local` 汇合读边界；
//! 批次 4.1 起覆盖 `String` 六种比较的接受集与混型拒绝边界（依据设计文档
//! 「运行时 ABI 约定（首版）」与「支持集」）。目录模块按 test-specification
//! 1.1/1.4 声明。

use crate::backends::llvm::analysis::analyze_module;
use crate::frontend::core::sema::MonoType;
use crate::frontend::core::types::identity::Nominal;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand, ValueId,
    ValueInfo,
};
use crate::util::span::Span;

/// 带真实行/列的 span：拒绝诊断必须能定位到源码位置。
fn at(
    line: usize,
    column: usize,
) -> Span {
    let position = crate::util::span::Position::new(line, column);
    Span::new(position, position)
}

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

fn load_local(
    slot: usize,
    src: Operand,
) -> Instruction {
    Instruction::Load {
        dst: Operand::Local(slot),
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
            ty: MonoType::TypeRef(Nominal::flat("?").expect("'?' is a valid bare type name")),
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
        // 容器类型仍在支持集外（`Float` 自批次 4.5 起已是标量支持集成员，不能再当
        // 「不支持类型」的样例）。
        vec![ValueInfo {
            ty: MonoType::Generic {
                name: Nominal::flat("List").expect("List is a valid bare type name"),
                args: vec![MonoType::Int(64)],
            },
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
        detail.contains("ValueId(0)") && detail.contains("List"),
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
fn test_native_phi_accepts_known_merge_type() {
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
    let analysis =
        analyze_module(&module, "phi.yx").expect("a Phi with known incoming types is supported");

    // Assert
    assert_eq!(
        analysis.functions["f"].values[0],
        MonoType::Int(64),
        "Phi destination type must come from ValueInfo.ty"
    );
}

/// 生产端 `materialize_undef` 的汇合形态：入口 `Load Void`（显式未知）作为
/// 一个前驱的 Phi incoming，另一个前驱给已知 Int。
fn phi_with_unknown_incoming_module() -> ModuleIR {
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
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Value(ValueId(0)),
                            src: Operand::Const(ConstValue::Void),
                            span: Span::dummy(),
                        },
                        Instruction::JmpIfNot {
                            cond: Operand::Const(ConstValue::Bool(true)),
                            target: 4,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![1, 2],
                },
                BasicBlock {
                    label: 1,
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Value(ValueId(1)),
                            src: Operand::Const(ConstValue::Int(5)),
                            span: Span::dummy(),
                        },
                        Instruction::Jmp {
                            target: 4,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![3],
                },
                BasicBlock {
                    label: 2,
                    instructions: vec![],
                    successors: vec![3],
                },
                BasicBlock {
                    label: 3,
                    instructions: vec![
                        Instruction::Phi {
                            dst: Operand::Value(ValueId(2)),
                            incoming: vec![
                                (1, Operand::Value(ValueId(1))),
                                (2, Operand::Value(ValueId(0))),
                            ],
                            span: at(6, 9),
                        },
                        Instruction::Ret {
                            value: Some(Operand::Value(ValueId(2))),
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![],
                },
            ],
            entry: 0,
            locals: vec![],
            values: vec![
                ValueInfo {
                    ty: MonoType::TypeRef(
                        Nominal::flat("?").expect("'?' is a valid bare type name"),
                    ),
                    debug_name: None,
                    defining_block: 0,
                },
                ValueInfo {
                    ty: MonoType::Int(64),
                    debug_name: None,
                    defining_block: 1,
                },
                ValueInfo {
                    ty: MonoType::Int(64),
                    debug_name: None,
                    defining_block: 3,
                },
            ],
            value_slots: vec![],
        },
    };
    module_with(function)
}

#[test]
fn test_native_phi_live_unknown_incoming_is_rejected() {
    // Arrange：显式未知 incoming 进入一个被 `Ret` 使用的 Phi——合并值 live，
    // 按 dst 类型物化 undef 会成为可观察的未定义值。
    let module = phi_with_unknown_incoming_module();

    // Act
    let error = analyze_module(&module, "phi_unknown.yx")
        .err()
        .expect("a live Phi merging an explicit unknown incoming must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("ValueId(2)") && detail.contains("explicitly unknown"),
        "live unknown-incoming Phi diagnostic must name the value and the merge: {detail}"
    );
    assert!(
        detail.contains("phi_unknown.yx:6:9"),
        "live unknown-incoming Phi diagnostic must carry the Phi position: {detail}"
    );
    assert!(
        detail.contains("Void") && detail.contains("undef"),
        "diagnostic must explain the VM frame-slot default / undef boundary: {detail}"
    );
}

/// 死的合流值 + 显式未知 incoming：Phi 的 dst 在本函数没有任何使用点，
/// 因此 `undef` 不可观察；该形态保留在支持集内（生产端 while 短路累加器
/// 的循环头 Phi 即此类）。
fn dead_unknown_incoming_phi_module() -> ModuleIR {
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
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Value(ValueId(0)),
                            src: Operand::Const(ConstValue::Void),
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
                        Instruction::Phi {
                            dst: Operand::Value(ValueId(1)),
                            incoming: vec![(0, Operand::Value(ValueId(0)))],
                            span: at(6, 9),
                        },
                        Instruction::Ret {
                            value: Some(Operand::Const(ConstValue::Int(7))),
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![],
                },
            ],
            entry: 0,
            locals: vec![],
            values: vec![
                ValueInfo {
                    ty: MonoType::TypeRef(
                        Nominal::flat("?").expect("'?' is a valid bare type name"),
                    ),
                    debug_name: None,
                    defining_block: 0,
                },
                ValueInfo {
                    ty: MonoType::Int(64),
                    debug_name: None,
                    defining_block: 1,
                },
            ],
            value_slots: vec![],
        },
    };
    module_with(function)
}

#[test]
fn test_native_phi_dead_unknown_incoming_is_allowed() {
    // Arrange：显式未知 incoming 进入一个没有使用点的 Phi——undef 不可观察，
    // 不触发 live 边界（与生产端 while 短路合流的复现形态一致）。
    let module = dead_unknown_incoming_phi_module();

    // Act
    let analysis = analyze_module(&module, "phi_dead_unknown.yx")
        .expect("a dead Phi with explicit unknown incoming stays inside the support set");

    // Assert
    assert_eq!(
        analysis.functions["f"].values[1],
        MonoType::Int(64),
        "dead Phi destination type must still come from ValueInfo.ty"
    );
}

/// 两块 Phi 夹具：块 0 写入 `entry_instructions` 后 `Jmp` 到块 1；块 1 块首放
/// 给定 `Phi`，随后 `Ret` 其 dst。`Jmp` 的目标是块 1 的展平起点（Phi 下标）。
fn two_block_phi_module(
    entry_instructions: Vec<Instruction>,
    incoming: Vec<(usize, Operand)>,
    phi_type: MonoType,
) -> ModuleIR {
    let phi_index = entry_instructions.len() + 1;
    let mut entry = entry_instructions;
    entry.push(Instruction::Jmp {
        target: phi_index,
        span: Span::dummy(),
    });
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
                    instructions: entry,
                    successors: vec![1],
                },
                BasicBlock {
                    label: 1,
                    instructions: vec![
                        Instruction::Phi {
                            dst: Operand::Value(ValueId(0)),
                            incoming,
                            span: at(6, 9),
                        },
                        ret_value(0),
                    ],
                    successors: vec![],
                },
            ],
            entry: 0,
            // Local(0) 是 `test_native_phi_local_incoming_is_rejected` 的写入槽；
            // 其余夹具不用它，注册槽位不改变分析结论。
            locals: vec![crate::middle::ir::ir::LocalSlot::temp(MonoType::Int(64))],
            values: vec![ValueInfo {
                ty: phi_type,
                debug_name: None,
                defining_block: 1,
            }],
            value_slots: vec![],
        },
    };
    module_with(function)
}

#[test]
fn test_native_phi_entry_block_without_incoming_is_rejected() {
    // Arrange：入口块没有前驱。verify_ssa 允许 incoming 为空的 Phi（前驱数 0 ==
    // incoming 数 0），但 LLVM phi 至少要一条 incoming；分析层必须发射前拒绝。
    let function = base_function(
        MonoType::Int(64),
        vec![ValueInfo {
            ty: MonoType::Int(64),
            debug_name: None,
            defining_block: 0,
        }],
        vec![
            Instruction::Phi {
                dst: Operand::Value(ValueId(0)),
                incoming: Vec::new(),
                span: at(3, 1),
            },
            ret_value(0),
        ],
    );
    let module = module_with(function);

    // Act
    let error = analyze_module(&module, "phi_entry.yx")
        .err()
        .expect("a Phi without incoming edges must be rejected before emission");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("ValueId(0)") && detail.contains("no incoming"),
        "empty-incoming Phi diagnostic must name the value and the reason: {detail}"
    );
    assert!(
        detail.contains("phi_entry.yx:3:1"),
        "empty-incoming Phi diagnostic must carry the source position: {detail}"
    );
}

#[test]
fn test_native_phi_local_incoming_is_rejected() {
    // Arrange：Phi 的一个输入是 Local 槽（不是 SSA 值）。生产端不会产出这种形态；
    // 发射层无法在前驱终结指令之后插入 load，必须显式拒绝而不是让 LLVM 兜底。
    let module = two_block_phi_module(
        vec![load_local(0, Operand::Const(ConstValue::Int(5)))],
        vec![(0, Operand::Local(0))],
        MonoType::Int(64),
    );

    // Act
    let error = analyze_module(&module, "phi_local.yx")
        .err()
        .expect("a Phi incoming from a Local slot must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("must be an SSA value or constant"),
        "Local-incoming Phi diagnostic must name the supported incoming forms: {detail}"
    );
    assert!(
        detail.contains("phi_local.yx:6:9"),
        "Local-incoming Phi diagnostic must carry the Phi position: {detail}"
    );
}

#[test]
fn test_native_phi_unknown_destination_is_rejected() {
    // Arrange：Phi dst 的值表类型是显式未知 TypeRef("?")。
    let module = two_block_phi_module(
        vec![],
        vec![(0, Operand::Const(ConstValue::Int(1)))],
        MonoType::TypeRef(Nominal::flat("?").expect("'?' is a valid bare type name")),
    );

    // Act
    let error = analyze_module(&module, "phi_unknown_dst.yx")
        .err()
        .expect("a Phi destination with explicit unknown type must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("ValueId(0)") && detail.contains("TypeRef(\"?\")"),
        "unknown Phi destination diagnostic must name value id and original type: {detail}"
    );
}

#[test]
fn test_native_phi_known_type_mismatch_is_rejected() {
    // Arrange：Phi dst 的 ValueInfo 是 Int(64)，前驱给 Bool 常量。
    let module = two_block_phi_module(
        vec![],
        vec![(0, Operand::Const(ConstValue::Bool(true)))],
        MonoType::Int(64),
    );

    // Act
    let error = analyze_module(&module, "phi_type_mismatch.yx")
        .err()
        .expect("a known-type Phi incoming mismatch must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("Bool") && detail.contains("Int"),
        "Phi incoming type mismatch diagnostic must name both types: {detail}"
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
        span: at(9, 5),
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
fn test_native_local_multi_predecessor_write_is_allowed() {
    // Arrange：两臂各写 Local(0)，汇合后读；批次 3.5 起由入口块 alloca 支持。
    let module = local_merge_module(true);

    // Act
    let analysis = analyze_module(&module, "merge_multi.yx")
        .expect("multi-predecessor Local merge is inside the batch 3.5 support set");

    // Assert
    assert!(
        analysis.functions.contains_key("merge"),
        "merged Local function should be analyzed"
    );
}

#[test]
fn test_native_local_read_without_write_on_path_is_rejected() {
    // Arrange：单臂写、另一条路径直接读；VM 帧槽默认值（Void）无法在带类型
    // alloca 上物化，收窄后的边界仍必须显式拒绝。
    let module = local_merge_module(false);

    // Act
    let error = analyze_module(&module, "merge_single.yx")
        .err()
        .expect("a Local read on a path without a write must stay rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("Local(0)") && detail.contains("never written"),
        "narrowed boundary diagnostic must name Local and the reason: {detail}"
    );
    assert!(
        detail.contains("merge_single.yx:9:5"),
        "narrowed boundary diagnostic must carry the read source position: {detail}"
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

/// String 类型的值表项：批次 4.1 的六种比较都要求两侧取到同一具体类型。
fn string_value_info(block: usize) -> ValueInfo {
    ValueInfo {
        ty: MonoType::Generic {
            name: Nominal::flat("String").expect("String is a valid bare type name"),
            args: Vec::new(),
        },
        debug_name: None,
        defining_block: block,
    }
}

fn bool_value_info(block: usize) -> ValueInfo {
    ValueInfo {
        ty: MonoType::Bool,
        debug_name: None,
        defining_block: block,
    }
}

#[test]
fn test_native_string_equality_comparison_is_accepted() {
    // Arrange：两个 String 值的 Eq/Ne，结果类型是 Bool（内容比较，不比较指针）。
    let function = base_function(
        MonoType::Bool,
        vec![
            string_value_info(0),
            string_value_info(0),
            bool_value_info(0),
            bool_value_info(0),
        ],
        vec![
            load_value(0, Operand::Const(ConstValue::String("lhs".to_string()))),
            load_value(1, Operand::Const(ConstValue::String("rhs".to_string()))),
            Instruction::Eq {
                dst: Operand::Value(ValueId(2)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(2, 5),
            },
            Instruction::Ne {
                dst: Operand::Value(ValueId(3)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(3, 5),
            },
            ret_value(2),
        ],
    );
    let module = module_with(function);

    // Act
    let analysis = analyze_module(&module, "string_eq.yx")
        .expect("String equality comparison should be inside the support set");

    // Assert
    assert_eq!(
        analysis.functions["f"].values[2],
        MonoType::Bool,
        "String equality must produce a Bool value"
    );
}

#[test]
fn test_native_string_ordering_comparison_is_accepted() {
    // Arrange：两个 String 值的 Lt/Le/Gt/Ge，结果类型是 Bool（字节字典序）。
    let function = base_function(
        MonoType::Bool,
        vec![
            string_value_info(0),
            string_value_info(0),
            bool_value_info(0),
            bool_value_info(0),
            bool_value_info(0),
            bool_value_info(0),
        ],
        vec![
            load_value(0, Operand::Const(ConstValue::String("abc".to_string()))),
            load_value(1, Operand::Const(ConstValue::String("abd".to_string()))),
            Instruction::Lt {
                dst: Operand::Value(ValueId(2)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(2, 5),
            },
            Instruction::Le {
                dst: Operand::Value(ValueId(3)),
                lhs: Operand::Value(ValueId(1)),
                rhs: Operand::Value(ValueId(0)),
                span: at(3, 5),
            },
            Instruction::Gt {
                dst: Operand::Value(ValueId(4)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(4, 5),
            },
            Instruction::Ge {
                dst: Operand::Value(ValueId(5)),
                lhs: Operand::Value(ValueId(1)),
                rhs: Operand::Value(ValueId(0)),
                span: at(5, 5),
            },
            ret_value(3),
        ],
    );
    let module = module_with(function);

    // Act
    let analysis = analyze_module(&module, "string_ordering.yx")
        .expect("String ordering comparison should be inside the support set");

    // Assert
    assert_eq!(
        analysis.functions["f"].values[5],
        MonoType::Bool,
        "String ordering must produce a Bool value"
    );
}

#[test]
fn test_native_string_mixed_comparison_is_rejected_with_location() {
    // Arrange：String 与 Int 混型判等必须带位置拒绝，不能被当成 String 比较放行。
    let function = base_function(
        MonoType::Bool,
        vec![
            string_value_info(0),
            ValueInfo {
                ty: MonoType::Int(64),
                debug_name: None,
                defining_block: 0,
            },
            bool_value_info(0),
        ],
        vec![
            load_value(0, Operand::Const(ConstValue::String("lhs".to_string()))),
            load_value(1, Operand::Const(ConstValue::Int(1))),
            Instruction::Eq {
                dst: Operand::Value(ValueId(2)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(7, 13),
            },
            ret_value(2),
        ],
    );
    let module = module_with(function);

    // Act
    let error = analyze_module(&module, "string_mixed.yx")
        .err()
        .expect("mixed String/Int comparison must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("string_mixed.yx:7:13"),
        "rejection must carry the source location: {detail}"
    );
    assert!(
        detail.contains("String") && detail.contains("Int"),
        "rejection must name both operand types: {detail}"
    );
}

fn int_value_info(block: usize) -> ValueInfo {
    ValueInfo {
        ty: MonoType::Int(64),
        debug_name: None,
        defining_block: block,
    }
}

#[test]
fn test_native_int_bitwise_operands_are_accepted() {
    // Arrange：And/Or/Xor 两侧 Int → Int（批次 4.2；按位语义，不检查值范围）。
    let function = base_function(
        MonoType::Int(64),
        vec![
            int_value_info(0),
            int_value_info(0),
            int_value_info(0),
            int_value_info(0),
            int_value_info(0),
        ],
        vec![
            load_value(0, Operand::Const(ConstValue::Int(6))),
            load_value(1, Operand::Const(ConstValue::Int(2))),
            Instruction::And {
                dst: Operand::Value(ValueId(2)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(2, 5),
            },
            Instruction::Or {
                dst: Operand::Value(ValueId(3)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(3, 5),
            },
            Instruction::Xor {
                dst: Operand::Value(ValueId(4)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(4, 5),
            },
            ret_value(4),
        ],
    );
    let module = module_with(function);

    // Act
    let analysis = analyze_module(&module, "int_bitwise.yx")
        .expect("Int bitwise operands should be inside the support set");

    // Assert
    assert_eq!(
        analysis.functions["f"].values[4],
        MonoType::Int(64),
        "bitwise result must be an Int value"
    );
}

#[test]
fn test_native_shift_operands_are_accepted() {
    // Arrange：Shl/Shr/Sar 两侧 Int → Int（批次 4.2；越界只由运行时失败路径报错）。
    let function = base_function(
        MonoType::Int(64),
        vec![
            int_value_info(0),
            int_value_info(0),
            int_value_info(0),
            int_value_info(0),
            int_value_info(0),
        ],
        vec![
            load_value(0, Operand::Const(ConstValue::Int(1024))),
            load_value(1, Operand::Const(ConstValue::Int(2))),
            Instruction::Shl {
                dst: Operand::Value(ValueId(2)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(2, 5),
            },
            Instruction::Shr {
                dst: Operand::Value(ValueId(3)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(3, 5),
            },
            Instruction::Sar {
                dst: Operand::Value(ValueId(4)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(4, 5),
            },
            ret_value(4),
        ],
    );
    let module = module_with(function);

    // Act
    let analysis = analyze_module(&module, "int_shift.yx")
        .expect("Int shift operands should be inside the support set");

    // Assert
    assert_eq!(
        analysis.functions["f"].values[4],
        MonoType::Int(64),
        "shift result must be an Int value"
    );
}

#[test]
fn test_native_shift_mixed_operand_is_rejected_with_location() {
    // Arrange：Int 与 Bool 混型的移位必须带位置拒绝。
    let function = base_function(
        MonoType::Int(64),
        vec![
            int_value_info(0),
            ValueInfo {
                ty: MonoType::Bool,
                debug_name: None,
                defining_block: 0,
            },
            int_value_info(0),
        ],
        vec![
            load_value(0, Operand::Const(ConstValue::Int(1))),
            load_value(1, Operand::Const(ConstValue::Bool(true))),
            Instruction::Shl {
                dst: Operand::Value(ValueId(2)),
                lhs: Operand::Value(ValueId(0)),
                rhs: Operand::Value(ValueId(1)),
                span: at(5, 11),
            },
            ret_value(2),
        ],
    );
    let module = module_with(function);

    // Act
    let error = analyze_module(&module, "shift_mixed.yx")
        .err()
        .expect("mixed Int/Bool shift must be rejected");

    // Assert
    let detail = error.to_string();
    assert!(
        detail.contains("shift_mixed.yx:5:11"),
        "rejection must carry the source location: {detail}"
    );
    assert!(
        detail.contains("Int") && detail.contains("Bool"),
        "rejection must name both operand types: {detail}"
    );
}
