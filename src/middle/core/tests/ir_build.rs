//! IR 手工构造共享助手（禁令三单点）：verify / snapshot 判据测试的
//! FunctionIR / ModuleIR 构造器。单块函数：块内指令序即展平序。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
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
    }
}

pub fn func(
    name: &str,
    params: Vec<MonoType>,
    return_type: MonoType,
    locals: Vec<LocalSlot>,
    instrs: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        def: None,
        params,
        return_type,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: instrs,
                successors: Vec::new(),
            }],
            entry: 0,
            locals,
        },
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
