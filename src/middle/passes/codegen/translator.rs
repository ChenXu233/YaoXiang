//! IR 到字节码翻译器
//!
//! 将中间表示（IR）翻译为字节码指令。

use crate::backends::common::Opcode;
use crate::middle::core::ir::{ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::middle::passes::codegen::emitter::Emitter;
use crate::middle::passes::codegen::operand::OperandResolver;
use crate::middle::passes::codegen::{BytecodeInstruction, CodegenError};
use std::collections::{HashMap, HashSet};

/// IR 到字节码翻译器
///
/// 职责：
/// - 将 Instruction 翻译为 BytecodeInstruction
/// - 管理翻译过程中的状态
/// - 处理跳转偏移回填
#[derive(Debug)]
pub struct Translator {
    /// 字节码发射器
    emitter: Emitter,
    /// 操作数解析器
    operand_resolver: OperandResolver,
    /// 当前函数
    current_function: Option<FunctionIR>,
    /// 已注册的 native 函数名集合
    native_functions: HashSet<String>,
}

impl Translator {
    /// 创建新的翻译器
    pub fn new() -> Self {
        let mut native_functions = HashSet::new();

        // 从 std 模块自动发现 native 函数
        for (short_name, native_name) in crate::std::all_native_names() {
            native_functions.insert(native_name.to_string());
            native_functions.insert(short_name.to_string());
        }

        Translator {
            emitter: Emitter::new(),
            operand_resolver: OperandResolver::new(),
            current_function: None,
            native_functions,
        }
    }

    /// 注册一个 native 函数名
    pub fn register_native(
        &mut self,
        name: &str,
    ) {
        self.native_functions.insert(name.to_string());
    }

    /// 检查函数是否是 native 函数
    pub fn is_native(
        &self,
        name: &str,
    ) -> bool {
        self.native_functions.contains(name)
    }

    /// 添加常量（用于测试）
    pub fn test_add_constant(
        &mut self,
        value: ConstValue,
    ) -> usize {
        self.emitter.add_constant(value)
    }

    /// 翻译模块
    pub fn translate_module(
        &mut self,
        module: &ModuleIR,
    ) -> Result<TranslatorOutput, CodegenError> {
        // 注册用户声明的 native 函数绑定
        // 这些来自 `my_func: Type = Native("symbol")` 声明
        for binding in &module.native_bindings {
            // 注册函数名（使调用点生成 CallNative）
            self.register_native(binding.func_name());
            // 也注册 native symbol（若与函数名不同）
            if binding.func_name() != binding.native_symbol() {
                self.register_native(binding.native_symbol());
            }
        }

        let mut code_section = super::CodeSection {
            functions: Vec::new(),
        };

        for func in &module.functions {
            let func_code = self.translate_function(func)?;
            code_section.functions.push(func_code);
        }

        let const_pool = self.emitter.take_constant_pool();

        Ok(TranslatorOutput {
            code_section,
            const_pool,
        })
    }

    /// 翻译单个函数
    fn translate_function(
        &mut self,
        func: &FunctionIR,
    ) -> Result<super::FunctionCode, CodegenError> {
        self.current_function = Some(func.clone());

        let mut instructions = Vec::new();
        let mut ir_to_bytecode_map = HashMap::new();
        let mut pending_jumps: Vec<(usize, usize, Opcode)> = Vec::new(); // (bytecode_idx, target_ir_idx, opcode)
        let mut global_ir_index = 0;

        for block in func.blocks.iter() {
            for instr in &block.instructions {
                ir_to_bytecode_map.insert(global_ir_index, instructions.len());
                let current_bytecode_idx = instructions.len();

                // 检查是否是跳转指令，记录待回填信息
                if let Some((target, opcode)) = Self::get_jump_target(instr) {
                    pending_jumps.push((current_bytecode_idx, target, opcode));
                }

                global_ir_index += 1;

                let bytecode_instr = self.translate_instruction(instr)?;
                instructions.push(bytecode_instr);
            }
        }

        ir_to_bytecode_map.insert(global_ir_index, instructions.len());

        // 回填跳转偏移
        Self::backfill_jumps_impl(&mut instructions, &ir_to_bytecode_map, &pending_jumps);

        Ok(super::FunctionCode {
            name: func.name.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone(),
            instructions,
            local_count: func.locals.len(),
        })
    }

    /// 从指令中提取跳转目标（如果是跳转指令）
    fn get_jump_target(instr: &Instruction) -> Option<(usize, Opcode)> {
        match instr {
            Instruction::Jmp(target) => Some((*target, Opcode::Jmp)),
            Instruction::JmpIf(_, target) => Some((*target, Opcode::JmpIf)),
            Instruction::JmpIfNot(_, target) => Some((*target, Opcode::JmpIfNot)),
            _ => None,
        }
    }

    /// 回填跳转偏移（实际实现）
    fn backfill_jumps_impl(
        instructions: &mut [BytecodeInstruction],
        ir_to_bytecode_map: &HashMap<usize, usize>,
        pending_jumps: &[(usize, usize, Opcode)],
    ) {
        for (bytecode_idx, target_ir_idx, opcode) in pending_jumps {
            if let Some(&target_bytecode_idx) = ir_to_bytecode_map.get(target_ir_idx) {
                // 计算相对偏移: target - current
                let offset = (target_bytecode_idx as i32) - (*bytecode_idx as i32);
                let bytes = offset.to_le_bytes();

                let instr = &mut instructions[*bytecode_idx];
                match opcode {
                    Opcode::Jmp => {
                        // Jmp 操作数: [offset: i32]
                        instr.operands[0] = bytes[0];
                        instr.operands[1] = bytes[1];
                        instr.operands[2] = bytes[2];
                        instr.operands[3] = bytes[3];
                    }
                    Opcode::JmpIf | Opcode::JmpIfNot => {
                        // JmpIf/JmpIfNot 操作数: [cond_reg: u8, offset: i32]
                        instr.operands[1] = bytes[0];
                        instr.operands[2] = bytes[1];
                        instr.operands[3] = bytes[2];
                        instr.operands[4] = bytes[3];
                    }
                    _ => {}
                }
            }
        }
    }

    /// 翻译单条 IR 指令
    fn translate_instruction(
        &mut self,
        instr: &Instruction,
    ) -> Result<BytecodeInstruction, CodegenError> {
        use Instruction::*;

        match instr {
            Move { dst, src } => self.translate_move(dst, src),
            Load { dst, src } => self.translate_load(dst, src),
            Store { dst, src, .. } => self.translate_store(dst, src),

            Add { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Add, dst, lhs, rhs),
            Sub { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Sub, dst, lhs, rhs),
            Mul { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Mul, dst, lhs, rhs),
            Div { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Div, dst, lhs, rhs),
            Mod { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Rem, dst, lhs, rhs),

            And { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64And, dst, lhs, rhs),
            Or { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Or, dst, lhs, rhs),
            Xor { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Xor, dst, lhs, rhs),
            Shl { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Shl, dst, lhs, rhs),
            Shr { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Shr, dst, lhs, rhs),
            Sar { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Sar, dst, lhs, rhs),
            Neg { dst, src } => self.translate_unary_op(Opcode::I64Neg, dst, src),

            Eq { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Eq, dst, lhs, rhs),
            Ne { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Ne, dst, lhs, rhs),
            Lt { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Lt, dst, lhs, rhs),
            Le { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Le, dst, lhs, rhs),
            Gt { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Gt, dst, lhs, rhs),
            Ge { dst, lhs, rhs } => self.translate_binary_op(Opcode::I64Ge, dst, lhs, rhs),

            Jmp(target) => self.translate_jmp(*target),
            JmpIf(cond, target) => self.translate_jmp_if(cond, *target),
            JmpIfNot(cond, target) => self.translate_jmp_if_not(cond, *target),
            Ret(value) => self.translate_ret(value),

            Call { dst, func, args } => self.translate_call(dst, func, args),
            CallVirt {
                dst,
                obj,
                method_name,
                args,
            } => self.translate_call_virt(dst, obj, method_name.as_str(), args),
            CallDyn { dst, func, args } => self.translate_call_dyn(dst, func, args),
            TailCall { func, args } => self.translate_tail_call(func, args),

            Alloc { dst, .. } => self.translate_alloc(dst),
            Free(_) => Ok(BytecodeInstruction::new(Opcode::Nop, vec![])),
            AllocArray { dst, .. } => self.translate_alloc_array(dst),

            LoadField { dst, src, field } => self.translate_load_field(dst, src, *field),
            StoreField {
                dst, field, src, ..
            } => self.translate_store_field(dst, *field, src),
            LoadIndex { dst, src, index } => self.translate_load_index(dst, src, index),
            StoreIndex {
                dst, index, src, ..
            } => self.translate_store_index(dst, index, src),

            Cast { dst, src, .. } => self.translate_cast(dst, src),
            TypeTest(_, _) => Ok(BytecodeInstruction::new(Opcode::TypeCheck, vec![0, 0, 0])),

            Spawn { .. } => Ok(BytecodeInstruction::new(Opcode::Nop, vec![])),
            Yield => Ok(BytecodeInstruction::new(Opcode::Yield, vec![])),

            HeapAlloc { dst, .. } => self.translate_heap_alloc(dst),
            MakeClosure { dst, func, .. } => self.translate_make_closure(dst, *func),
            Drop(operand) => self.translate_drop(operand),

            Push(operand) => self.translate_push(operand),
            Pop(operand) => self.translate_pop(operand),
            Dup => Ok(BytecodeInstruction::new(Opcode::Nop, vec![])),
            Swap => Ok(BytecodeInstruction::new(Opcode::Nop, vec![])),

            ArcNew { dst, src } => self.translate_arc_new(dst, src),
            ArcClone { dst, src } => self.translate_arc_clone(dst, src),
            ArcDrop(operand) => self.translate_arc_drop(operand),
            ShareRef { dst, src } => self.translate_share_ref(dst, src),

            StringLength { dst, src } => self.translate_string_length(dst, src),
            StringConcat { dst, lhs, rhs } => self.translate_string_concat(dst, lhs, rhs),
            StringGetChar { dst, src, index } => self.translate_string_get_char(dst, src, index),
            StringFromInt { dst, src } => self.translate_string_from_int(dst, src),
            StringFromFloat { dst, src } => self.translate_string_from_float(dst, src),

            LoadUpvalue { dst, upvalue_idx } => self.translate_load_upvalue(dst, *upvalue_idx),
            StoreUpvalue { src, upvalue_idx } => self.translate_store_upvalue(src, *upvalue_idx),

            // unsafe 块和指针操作（暂不支持，跳过）
            UnsafeBlockStart | UnsafeBlockEnd => Ok(BytecodeInstruction::new(Opcode::Nop, vec![])),
            PtrFromRef { .. } | PtrDeref { .. } | PtrStore { .. } | PtrLoad { .. } => {
                Ok(BytecodeInstruction::new(Opcode::Nop, vec![]))
            }

            CloseUpvalue(operand) => self.translate_close_upvalue(operand),
        }
    }

    // ===== 翻译辅助方法 =====

    fn translate_move(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::Mov,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_load(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        match src {
            Operand::Const(const_val) => {
                let const_idx = self.emitter.add_constant(const_val.clone());
                Ok(BytecodeInstruction::new(
                    Opcode::LoadConst,
                    vec![dst_reg, (const_idx as u16) as u8, (const_idx >> 8) as u8],
                ))
            }
            Operand::Local(local_idx) => Ok(BytecodeInstruction::new(
                Opcode::LoadLocal,
                vec![dst_reg, *local_idx as u8],
            )),
            Operand::Arg(arg_idx) => Ok(BytecodeInstruction::new(
                Opcode::LoadArg,
                vec![dst_reg, *arg_idx as u8],
            )),
            _ => {
                let src_reg = self.operand_resolver.to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    Opcode::Mov,
                    vec![dst_reg, src_reg],
                ))
            }
        }
    }

    fn translate_store(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        if let Operand::Local(local_idx) = dst {
            let src_reg = self.operand_resolver.to_reg(src)?;
            Ok(BytecodeInstruction::new(
                Opcode::StoreLocal,
                vec![*local_idx as u8, src_reg],
            ))
        } else {
            Err(CodegenError::InvalidOperand)
        }
    }

    fn translate_binary_op(
        &mut self,
        opcode: Opcode,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let lhs_reg = self.operand_resolver.to_reg(lhs)?;
        let rhs_reg = self.operand_resolver.to_reg(rhs)?;

        if matches!(opcode, Opcode::I64Add) {
            Ok(BytecodeInstruction::new(
                opcode,
                vec![dst_reg, 0, lhs_reg, 0, rhs_reg, 0],
            ))
        } else {
            Ok(BytecodeInstruction::new(
                opcode,
                vec![dst_reg, lhs_reg, rhs_reg],
            ))
        }
    }

    fn translate_unary_op(
        &mut self,
        opcode: Opcode,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(opcode, vec![dst_reg, src_reg]))
    }

    fn translate_jmp(
        &mut self,
        _target: usize,
    ) -> Result<BytecodeInstruction, CodegenError> {
        Ok(BytecodeInstruction::new(Opcode::Jmp, vec![0, 0, 0, 0]))
    }

    fn translate_jmp_if(
        &mut self,
        cond: &Operand,
        _target: usize,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let cond_reg = self.operand_resolver.to_reg(cond)?;
        Ok(BytecodeInstruction::new(
            Opcode::JmpIf,
            vec![cond_reg, 0, 0, 0, 0],
        ))
    }

    fn translate_jmp_if_not(
        &mut self,
        cond: &Operand,
        _target: usize,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let cond_reg = self.operand_resolver.to_reg(cond)?;
        Ok(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![cond_reg, 0, 0, 0, 0],
        ))
    }

    fn translate_ret(
        &mut self,
        value: &Option<Operand>,
    ) -> Result<BytecodeInstruction, CodegenError> {
        match value {
            Some(v) => {
                let reg = self.operand_resolver.to_reg(v)?;
                Ok(BytecodeInstruction::new(Opcode::ReturnValue, vec![reg]))
            }
            None => Ok(BytecodeInstruction::new(Opcode::Return, vec![])),
        }
    }

    fn translate_call(
        &mut self,
        dst: &Option<Operand>,
        func: &Operand,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };

        // 检查是否是 native 函数调用
        let func_name = match func {
            Operand::Const(ConstValue::String(name)) => Some(name.clone()),
            _ => None,
        };

        // 命名空间解析：如果函数名不是已知的 native 函数，
        // 尝试通过短名称映射获取完整名称
        let is_native = func_name
            .as_ref()
            .map(|n| self.is_native(n))
            .unwrap_or(false);

        let func_id = match func {
            Operand::Const(ConstValue::Int(i)) => *i as u32,
            Operand::Const(ConstValue::String(name)) => {
                let const_idx = self.emitter.add_constant(ConstValue::String(name.clone()));
                const_idx as u32
            }
            _ => 0,
        };
        let base_arg_reg = if let Some(first_arg) = args.first() {
            self.operand_resolver.to_reg(first_arg)?
        } else {
            0
        };
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(base_arg_reg);
        operands.push(args.len() as u8);
        for arg in args {
            let arg_reg = self.operand_resolver.to_reg(arg)?;
            operands.extend_from_slice(&(arg_reg as u16).to_le_bytes());
        }

        // 根据是否是 native 函数选择操作码
        let opcode = if is_native {
            Opcode::CallNative
        } else {
            Opcode::CallStatic
        };

        Ok(BytecodeInstruction::new(opcode, operands))
    }

    fn translate_call_virt(
        &mut self,
        dst: &Option<Operand>,
        obj: &Operand,
        method_name: &str,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };
        let obj_reg = self.operand_resolver.to_reg(obj)?;
        let name_idx = self
            .emitter
            .add_constant(ConstValue::String(method_name.to_owned())) as u16;
        let base_arg_reg = if let Some(first_arg) = args.first() {
            self.operand_resolver.to_reg(first_arg)?
        } else {
            0
        };
        let mut operands = vec![dst_reg, obj_reg];
        operands.extend_from_slice(&name_idx.to_le_bytes());
        operands.push(base_arg_reg);
        operands.push(args.len() as u8);
        Ok(BytecodeInstruction::new(Opcode::CallVirt, operands))
    }

    fn translate_call_dyn(
        &mut self,
        dst: &Option<Operand>,
        _func: &Operand,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };
        let func_idx = self.emitter.add_constant(ConstValue::Int(0));
        let func_handle = func_idx as u16;
        let base_arg_reg = if let Some(first_arg) = args.first() {
            self.operand_resolver.to_reg(first_arg)?
        } else {
            0
        };
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&func_handle.to_le_bytes());
        operands.push(base_arg_reg);
        operands.push(args.len() as u8);
        Ok(BytecodeInstruction::new(Opcode::CallDyn, operands))
    }

    fn translate_tail_call(
        &mut self,
        func: &Operand,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, CodegenError> {
        let func_id = match func {
            Operand::Const(ConstValue::Int(i)) => *i as u32,
            _ => 0,
        };
        let base_arg_reg = if let Some(first_arg) = args.first() {
            self.operand_resolver.to_reg(first_arg)?
        } else {
            0
        };
        let mut operands = vec![];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(base_arg_reg);
        operands.push(args.len() as u8);
        Ok(BytecodeInstruction::new(Opcode::TailCall, operands))
    }

    fn translate_alloc(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(Opcode::StackAlloc, vec![dst_reg]))
    }

    fn translate_alloc_array(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            Opcode::NewListWithCap,
            vec![dst_reg, 0, 0],
        ))
    }

    fn translate_load_field(
        &mut self,
        dst: &Operand,
        src: &Operand,
        field: usize,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let field_offset = field as u16;
        Ok(BytecodeInstruction::new(
            Opcode::GetField,
            vec![
                dst_reg,
                src_reg,
                (field_offset & 0xFF) as u8,
                (field_offset >> 8) as u8,
            ],
        ))
    }

    fn translate_store_field(
        &mut self,
        dst: &Operand,
        field: usize,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let field_offset = field as u16;
        Ok(BytecodeInstruction::new(
            Opcode::SetField,
            vec![
                dst_reg,
                (field_offset & 0xFF) as u8,
                (field_offset >> 8) as u8,
                src_reg,
            ],
        ))
    }

    fn translate_load_index(
        &mut self,
        dst: &Operand,
        src: &Operand,
        index: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        Ok(BytecodeInstruction::new(
            Opcode::LoadElement,
            vec![dst_reg, src_reg, index_reg],
        ))
    }

    fn translate_store_index(
        &mut self,
        dst: &Operand,
        index: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::StoreElement,
            vec![dst_reg, index_reg, src_reg],
        ))
    }

    fn translate_cast(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::Cast,
            vec![dst_reg, src_reg, 0, 0],
        ))
    }

    fn translate_heap_alloc(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            Opcode::HeapAlloc,
            vec![dst_reg, 0, 0],
        ))
    }

    fn translate_make_closure(
        &mut self,
        dst: &Operand,
        func: usize,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let func_id = func as u32;
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(0);
        Ok(BytecodeInstruction::new(Opcode::MakeClosure, operands))
    }

    fn translate_drop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(Opcode::Drop, vec![reg]))
    }

    fn translate_push(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(Opcode::Mov, vec![reg]))
    }

    fn translate_pop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(Opcode::Mov, vec![reg]))
    }

    fn translate_arc_new(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::ArcNew,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_arc_clone(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::ArcClone,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_arc_drop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(Opcode::ArcDrop, vec![reg]))
    }

    fn translate_share_ref(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        // ShareRef: 用于线程本地共享，需要类型是 Sync
        // TODO: 实现完整的 ShareRef 操作码支持
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        // 临时实现：使用 Nop，后续需要添加专门的 Opcode
        let _ = src_reg;
        Ok(BytecodeInstruction::new(Opcode::Nop, vec![dst_reg]))
    }

    fn translate_string_length(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::StringLength,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_string_concat(
        &mut self,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let lhs_reg = self.operand_resolver.to_reg(lhs)?;
        let rhs_reg = self.operand_resolver.to_reg(rhs)?;
        Ok(BytecodeInstruction::new(
            Opcode::StringConcat,
            vec![dst_reg, lhs_reg, rhs_reg],
        ))
    }

    fn translate_string_get_char(
        &mut self,
        dst: &Operand,
        src: &Operand,
        index: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        Ok(BytecodeInstruction::new(
            Opcode::StringGetChar,
            vec![dst_reg, src_reg, index_reg],
        ))
    }

    fn translate_string_from_int(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::StringFromInt,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_string_from_float(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::StringFromFloat,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_load_upvalue(
        &mut self,
        dst: &Operand,
        upvalue_idx: usize,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            Opcode::LoadUpvalue,
            vec![dst_reg, upvalue_idx as u8],
        ))
    }

    fn translate_store_upvalue(
        &mut self,
        src: &Operand,
        upvalue_idx: usize,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            Opcode::StoreUpvalue,
            vec![src_reg, upvalue_idx as u8],
        ))
    }

    fn translate_close_upvalue(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, CodegenError> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(Opcode::CloseUpvalue, vec![reg]))
    }
}

impl Default for Translator {
    fn default() -> Self {
        Translator::new()
    }
}

/// 翻译器输出
pub struct TranslatorOutput {
    pub code_section: super::CodeSection,
    pub const_pool: Vec<ConstValue>,
}
