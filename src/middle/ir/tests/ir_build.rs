//! IR 手工构造共享助手（禁令三单点）：verify / snapshot 判据测试的
//! FunctionIR / ModuleIR 构造器。单块函数：块内指令序即展平序。

use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FrameId, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR,
    Operand, ValueId, ValueInfo,
};
use crate::util::span::Span;

pub fn temp_slot(ty: MonoType) -> LocalSlot {
    LocalSlot::temp(ty)
}

pub fn named_slot(
    name: &str,
    ty: MonoType,
) -> LocalSlot {
    LocalSlot {
        name: Some(name.to_string()),
        ty,
        scope_depth: 0,
        quantity: None,
        origin: None,
    }
}

/// 具名槽，且**归属指定生成帧**（不变量 7「内层隔离」的负控/正控夹具用）。
pub fn named_slot_in_frame(
    name: &str,
    ty: MonoType,
    origin: FrameId,
) -> LocalSlot {
    LocalSlot {
        name: Some(name.to_string()),
        ty,
        scope_depth: 0,
        quantity: None,
        origin: Some(origin),
    }
}

pub fn func(
    name: &str,
    params: Vec<MonoType>,
    return_type: MonoType,
    locals: Vec<LocalSlot>,
    instrs: Vec<Instruction>,
) -> FunctionIR {
    func_with_blocks(
        name,
        params,
        return_type,
        locals,
        Vec::new(),
        vec![block(0, instrs, Vec::new())],
    )
}

/// 单块函数体（块 label、指令序、块级后继由调用方给定）。
pub fn block(
    label: usize,
    instructions: Vec<Instruction>,
    successors: Vec<usize>,
) -> BasicBlock {
    BasicBlock {
        label,
        instructions,
        successors,
    }
}

/// 多块函数体构造器（Phi 前驱判据用）：`values` 为空表即 7b.1b 生产形态。
pub fn func_with_blocks(
    name: &str,
    params: Vec<MonoType>,
    return_type: MonoType,
    locals: Vec<LocalSlot>,
    values: Vec<ValueInfo>,
    blocks: Vec<BasicBlock>,
) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        def: None,
        params,
        return_type,
        generic_params: None,
        body: FunctionBody::Code {
            blocks,
            entry: 0,
            locals,
            values,
        },
    }
}

/// 一条 `ValueInfo`（`defining_block` 由调用方给，类型取 `ty`）。
pub fn value_info(
    ty: MonoType,
    defining_block: usize,
) -> ValueInfo {
    ValueInfo {
        ty,
        debug_name: None,
        defining_block,
    }
}

/// `Operand::Value` 的简写（测试夹具专用；生产构造点 0）。
pub fn ssa_value(id: u32) -> Operand {
    Operand::Value(ValueId(id))
}

pub fn phi_instr(
    dst: Operand,
    incoming: Vec<(usize, Operand)>,
) -> Instruction {
    Instruction::Phi {
        dst,
        incoming,
        span: Span::dummy(),
    }
}

pub fn jmp(target: usize) -> Instruction {
    Instruction::Jmp {
        target,
        span: Span::dummy(),
    }
}

pub fn module_with(functions: Vec<FunctionIR>) -> ModuleIR {
    ModuleIR {
        functions,
        ..Default::default()
    }
}

pub fn int_const(n: i128) -> Operand {
    Operand::Const(ConstValue::Int(n))
}

pub fn move_instr(
    dst: usize,
    src: Operand,
) -> Instruction {
    Instruction::Move {
        dst: Operand::Local(dst),
        src,
        span: Span::dummy(),
    }
}

pub fn ret_void() -> Instruction {
    Instruction::Ret {
        value: None,
        span: Span::dummy(),
    }
}
