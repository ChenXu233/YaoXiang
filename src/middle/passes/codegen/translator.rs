//! IR 到字节码翻译器
//!
//! 将中间表示（IR）翻译为字节码指令。

use crate::backends::common::opcode;
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand,
};
use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::Reg;
use crate::middle::passes::codegen::emitter::Emitter;
use crate::middle::passes::codegen::operand::OperandResolver;
use crate::middle::passes::codegen::{BytecodeInstruction};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::{DebugSpan, FileId, Span};
use std::collections::HashMap;

/// FFI 函数元数据 — 机制/库/符号
#[derive(Debug, Clone)]
struct FfiFuncMeta {
    mechanism: String,
    lib: String,
    symbol: String,
}

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
    /// 闭包函数的索引偏移量（用于计算闭包函数在模块中的正确索引）
    closure_function_offset: Option<usize>,
    /// 函数名到索引的映射
    function_name_to_idx: Option<HashMap<String, usize>>,
    /// DefId 到函数表索引的映射（Stage 3：按 DefId 分发的主路径）
    function_def_to_idx: HashMap<crate::frontend::module::symbol::DefId, usize>,
    /// FFI 函数元数据缓存: func_name → mechanism/lib/symbol
    ffi_func_meta: HashMap<String, FfiFuncMeta>,

    /// 是否生成运行时调试信息（IP -> Span）
    generate_debug_info: bool,

    /// Debug info 关联的源文件 id（用于多文件/模块定位）
    source_file_id: FileId,
}

impl Translator {
    /// 创建新的翻译器
    pub fn new() -> Self {
        Translator {
            emitter: Emitter::new(),
            operand_resolver: OperandResolver::new(),
            current_function: None,
            ffi_func_meta: HashMap::new(),
            function_def_to_idx: HashMap::new(),
            closure_function_offset: None,
            function_name_to_idx: None,
            generate_debug_info: false,
            source_file_id: 0,
        }
    }

    pub fn set_generate_debug_info(
        &mut self,
        enable: bool,
    ) {
        self.generate_debug_info = enable;
    }

    pub fn set_source_file_id(
        &mut self,
        file_id: FileId,
    ) {
        self.source_file_id = file_id;
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
    ) -> Result<TranslatorOutput, Diagnostic> {
        // 消费 FFI 函数绑定 — 存储 mechanism/lib/symbol 元数据
        for binding in &module.ffi_bindings {
            if let crate::middle::core::ir::FfiBinding::FuncBinding {
                func_name,
                lib_id,
                symbol,
            } = binding
            {
                if let Some(lib) = module.ffi_libs.get(*lib_id) {
                    self.ffi_func_meta.insert(
                        func_name.clone(),
                        FfiFuncMeta {
                            mechanism: lib.mechanism.clone(),
                            lib: lib.lib_name.clone(),
                            symbol: symbol.clone(),
                        },
                    );
                }
            }
        }

        // 建立函数名到索引的映射
        let mut function_name_to_idx: HashMap<String, usize> = HashMap::new();
        for (idx, func) in module.functions.iter().enumerate() {
            function_name_to_idx.insert(func.name.clone(), idx);
        }
        // 建立 DefId 到索引的映射（Stage 3 主路径；FunctionIR.def 由 ir_gen 尾部 assign_defs 填充）
        self.function_def_to_idx = module
            .functions
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| f.def.map(|d| (d, idx)))
            .collect();

        // 注册闭包函数的索引偏移量（闭包函数从 module.functions.len() 开始）
        // 这样 translate_make_closure 就可以正确计算闭包函数的索引
        let closure_offset = module.functions.len();
        self.closure_function_offset = Some(closure_offset);
        self.function_name_to_idx = Some(function_name_to_idx);

        let mut code_section = super::CodeSection {
            functions: Vec::new(),
        };

        for func in &module.functions {
            // 多文件模式（#252）：按函数来源文件设置 debug span 的 file_id；
            // 单文件/未记录（mono 特化新名）回退 0（CLI SourceMap 首条目）。
            self.source_file_id =
                module.function_files.get(&func.name).copied().unwrap_or(0) as FileId;
            match &func.body {
                FunctionBody::TypeDecl { definition } => {
                    // 类型定义：从定义机械合成构造函数
                    let ctor = self.synthesize_constructor(func, definition)?;
                    code_section.functions.push(ctor);
                }
                FunctionBody::Code { .. } => {
                    // 普通函数：翻译指令
                    let func_code = self.translate_function(func)?;
                    code_section.functions.push(func_code);
                }
            }
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
    ) -> Result<super::FunctionCode, Diagnostic> {
        self.current_function = Some(func.clone());

        let mut instructions = Vec::new();
        let mut debug_map = HashMap::new();
        let mut ir_to_bytecode_map = HashMap::new();
        let mut pending_jumps: Vec<(usize, usize, u8)> = Vec::new(); // (bytecode_idx, target_ir_idx, opcode)
        let mut global_ir_index = 0;

        let (blocks_ref, locals_len) = match &func.body {
            FunctionBody::Code { blocks, locals, .. } => {
                (blocks.iter().collect::<Vec<_>>(), locals.len())
            }
            FunctionBody::TypeDecl { .. } => {
                return Err(Diagnostic::error(
                    "E_INTERNAL".to_string(),
                    "translate_function called on TypeDecl — should be handled by synthesize_constructor"
                        .to_string(),
                    "This is a compiler bug".to_string(),
                    None,
                ));
            }
        };

        for block in blocks_ref {
            for instr in &block.instructions {
                ir_to_bytecode_map.insert(global_ir_index, instructions.len());
                let current_bytecode_idx = instructions.len();

                if self.generate_debug_info {
                    if let Some(span) = Self::extract_span(instr) {
                        if !span.is_dummy() {
                            debug_map.insert(
                                current_bytecode_idx,
                                DebugSpan::new(self.source_file_id, span),
                            );
                        }
                    }
                }

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
            local_count: locals_len,
            debug_map,
        })
    }

    /// 从类型定义机械合成构造函数
    ///
    /// 等价于旧 ir_gen 的 `generate_struct_constructor_ir`，但在 codegen 阶段执行。
    /// 数据源从 `struct_definitions` HashMap 变为 `FunctionBody::TypeDecl { definition }`。
    fn synthesize_constructor(
        &mut self,
        type_func: &FunctionIR,
        definition: &crate::frontend::core::parser::ast::Type,
    ) -> Result<super::FunctionCode, Diagnostic> {
        let fields = definition.struct_fields();
        let struct_name = &type_func.name;

        if fields.is_empty() {
            // 无字段类型（如枚举）——返回空构造函数
            return Ok(super::FunctionCode {
                name: struct_name.clone(),
                params: Vec::new(),
                return_type: type_func.return_type.clone(),
                instructions: Vec::new(),
                local_count: 0,
                debug_map: HashMap::new(),
            });
        }

        // 构造函数 IR 指令序列：Load(每个字段) + CreateStruct + Ret
        let mut ir_instructions = Vec::new();
        let mut field_operands = Vec::new();

        for (i, _field) in fields.iter().enumerate() {
            let local_reg = i;
            ir_instructions.push(Instruction::Load {
                dst: Operand::Local(local_reg),
                src: Operand::Arg(i),
            });
            field_operands.push(Operand::Local(local_reg));
        }

        let result_reg = fields.len();
        ir_instructions.push(Instruction::CreateStruct {
            dst: Operand::Local(result_reg),
            type_name: struct_name.clone(),
            fields: field_operands,
        });
        ir_instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));

        // 创建临时 FunctionIR 用于 translate_instruction
        let param_types: Vec<MonoType> = fields.iter().map(|f| f.ty.clone().into()).collect();
        let mut locals: Vec<MonoType> = param_types.clone();
        locals.push(MonoType::TypeRef(struct_name.clone()));

        let temp_func = FunctionIR {
            name: struct_name.clone(),
            // 构造器与类型同一绑定身份：调用 Point(...) 解析到类型的 DefId
            def: type_func.def,
            params: param_types,
            return_type: type_func.return_type.clone(),
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions: ir_instructions.clone(),
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals.clone(),
            },
        };

        // 设置临时当前函数，调用 translate_function 完成翻译
        let saved_func = self.current_function.clone();
        let func_code = self.translate_function(&temp_func)?;
        self.current_function = saved_func;

        Ok(func_code)
    }

    fn extract_span(instr: &Instruction) -> Option<Span> {
        match instr {
            Instruction::Call { span, .. } => Some(*span),
            Instruction::CallVirt { span, .. } => Some(*span),
            Instruction::CallDyn { span, .. } => Some(*span),
            Instruction::Store { span, .. } => Some(*span),
            Instruction::StoreField { span, .. } => Some(*span),
            Instruction::StoreIndex { span, .. } => Some(*span),
            Instruction::Div { span, .. } => Some(*span),
            Instruction::Mod { span, .. } => Some(*span),
            Instruction::LoadField { span, .. } => Some(*span),
            Instruction::LoadIndex { span, .. } => Some(*span),
            _ => None,
        }
    }

    /// 从指令中提取跳转目标（如果是跳转指令）
    fn get_jump_target(instr: &Instruction) -> Option<(usize, u8)> {
        match instr {
            Instruction::Jmp(target) => Some((*target, opcode::JMP)),
            Instruction::JmpIf(_, target) => Some((*target, opcode::JMP_IF)),
            Instruction::JmpIfNot(_, target) => Some((*target, opcode::JMP_IF_NOT)),
            _ => None,
        }
    }

    /// 回填跳转偏移（实际实现）
    fn backfill_jumps_impl(
        instructions: &mut [BytecodeInstruction],
        ir_to_bytecode_map: &HashMap<usize, usize>,
        pending_jumps: &[(usize, usize, u8)],
    ) {
        for (bytecode_idx, target_ir_idx, opcode) in pending_jumps {
            if let Some(&target_bytecode_idx) = ir_to_bytecode_map.get(target_ir_idx) {
                // 计算相对偏移: target - current
                let offset = (target_bytecode_idx as i32) - (*bytecode_idx as i32);
                let bytes = offset.to_le_bytes();

                let instr = &mut instructions[*bytecode_idx];
                match *opcode {
                    opcode::JMP => {
                        // Jmp 操作数: [offset: i32]
                        instr.operands[0] = bytes[0];
                        instr.operands[1] = bytes[1];
                        instr.operands[2] = bytes[2];
                        instr.operands[3] = bytes[3];
                    }
                    opcode::JMP_IF | opcode::JMP_IF_NOT => {
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
    ) -> Result<BytecodeInstruction, Diagnostic> {
        use Instruction::*;

        match instr {
            Move { dst, src } => self.translate_move(dst, src),
            Load { dst, src } => self.translate_load(dst, src),
            Store { dst, src, .. } => self.translate_store(dst, src),

            Add { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_ADD, dst, lhs, rhs),
            Sub { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_SUB, dst, lhs, rhs),
            Mul { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_MUL, dst, lhs, rhs),
            Div { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_DIV, dst, lhs, rhs),
            Mod { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_REM, dst, lhs, rhs),

            And { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_AND, dst, lhs, rhs),
            Or { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_OR, dst, lhs, rhs),
            Xor { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_XOR, dst, lhs, rhs),
            Shl { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_SHL, dst, lhs, rhs),
            Shr { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_SHR, dst, lhs, rhs),
            Sar { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_SAR, dst, lhs, rhs),
            Neg { dst, src } => self.translate_unary_op(opcode::I64_NEG, dst, src),
            Not { dst, src } => self.translate_unary_op(opcode::I64_NEG, dst, src),

            Eq { dst, lhs, rhs } => {
                self.translate_compare(opcode::I64_EQ, opcode::I64_NE, dst, lhs, rhs)
            }
            Ne { dst, lhs, rhs } => {
                self.translate_compare(opcode::I64_NE, opcode::I64_EQ, dst, lhs, rhs)
            }
            Lt { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_LT, dst, lhs, rhs),
            Le { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_LE, dst, lhs, rhs),
            Gt { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_GT, dst, lhs, rhs),
            Ge { dst, lhs, rhs } => self.translate_binary_op(opcode::I64_GE, dst, lhs, rhs),

            Jmp(target) => self.translate_jmp(*target),
            JmpIf(cond, target) => self.translate_jmp_if(cond, *target),
            JmpIfNot(cond, target) => self.translate_jmp_if_not(cond, *target),
            Ret(value) => self.translate_ret(value),

            Call {
                dst,
                func,
                args,
                def,
                ..
            } => self.translate_call(dst, func, args, *def),
            CallVirt {
                dst,
                obj,
                method_name,
                args,
                ..
            } => self.translate_call_virt(dst, obj, method_name.as_str(), args),
            CallDyn {
                dst, func, args, ..
            } => self.translate_call_dyn(dst, func, args),
            TailCall { func, args, .. } => self.translate_tail_call(func, args),

            Alloc { dst, .. } => self.translate_alloc(dst),
            Free(_) => Ok(BytecodeInstruction::new(opcode::NOP, vec![])),
            AllocArray { dst, .. } => self.translate_alloc_array(dst),

            LoadField {
                dst, src, field, ..
            } => self.translate_load_field(dst, src, *field),
            StoreField {
                dst, field, src, ..
            } => self.translate_store_field(dst, *field, src),
            LoadIndex {
                dst, src, index, ..
            } => self.translate_load_index(dst, src, index),
            StoreIndex {
                dst, index, src, ..
            } => self.translate_store_index(dst, index, src),

            Cast { dst, src, .. } => self.translate_cast(dst, src),
            TypeTest(_, _) => Ok(BytecodeInstruction::new(opcode::TYPE_CHECK, vec![0, 0, 0])),

            Spawn {
                closures,
                plan,
                result,
            } => self.translate_spawn_multi(closures, plan, result),
            Yield => Ok(BytecodeInstruction::new(opcode::YIELD, vec![])),

            HeapAlloc { dst, .. } => self.translate_heap_alloc(dst),
            CreateStruct {
                dst,
                type_name,
                fields,
            } => self.translate_create_struct(dst, type_name, fields),
            NewDict { dst, keys, values } => self.translate_new_dict(dst, keys, values),
            NewTuple { dst, items } => self.translate_new_tuple(dst, items),
            MakeClosure {
                dst,
                func,
                def,
                env,
            } => self.translate_make_closure(dst, func, *def, env),
            Drop(operand) => self.translate_drop(operand),

            Push(operand) => self.translate_push(operand),
            Pop(operand) => self.translate_pop(operand),
            Dup => Ok(BytecodeInstruction::new(opcode::NOP, vec![])),
            Swap => Ok(BytecodeInstruction::new(opcode::NOP, vec![])),

            ArcNew { dst, src } => self.translate_arc_new(dst, src),
            RcNew { dst, src } => self.translate_rc_new(dst, src),
            ArcClone { dst, src } => self.translate_arc_clone(dst, src),
            ArcDrop(operand) => self.translate_arc_drop(operand),

            StringLength { dst, src } => self.translate_string_length(dst, src),
            StringConcat { dst, lhs, rhs } => self.translate_string_concat(dst, lhs, rhs),
            StringGetChar { dst, src, index } => self.translate_string_get_char(dst, src, index),
            StringFromInt { dst, src } => self.translate_string_from_int(dst, src),
            StringFromFloat { dst, src } => self.translate_string_from_float(dst, src),

            LoadUpvalue { dst, upvalue_idx } => self.translate_load_upvalue(dst, *upvalue_idx),
            StoreUpvalue { src, upvalue_idx } => self.translate_store_upvalue(src, *upvalue_idx),

            // unsafe 块和指针操作（暂不支持，跳过）
            UnsafeBlockStart | UnsafeBlockEnd => Ok(BytecodeInstruction::new(opcode::NOP, vec![])),
            PtrFromRef { .. } | PtrDeref { .. } | PtrStore { .. } | PtrLoad { .. } => {
                Ok(BytecodeInstruction::new(opcode::NOP, vec![]))
            }

            CloseUpvalue(operand) => self.translate_close_upvalue(operand),

            // spawn for: 从 List 寄存器动态读取闭包并 spawn
            Instruction::SpawnFromList {
                closures_list,
                plan,
                result,
            } => self.translate_spawn_from_list(closures_list, plan, result),
        }
    }

    // ===== 翻译辅助方法 =====

    fn translate_move(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        // 防御性：如果 dst 是 Local，走 StoreLocal 语义
        // （IR 层统一走 Store 后正常路径不会触发，但 if/match 表达式结果聚合的 Move{dst:Local} 仍可能存在）
        if let Operand::Local(local_idx) = dst {
            let src_reg = self.operand_resolver.to_reg(src)?;
            return Ok(BytecodeInstruction::new(
                opcode::STORE_LOCAL,
                vec![*local_idx as u8, src_reg],
            ));
        }
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::MOV,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_load(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        match src {
            Operand::Const(const_val) => {
                let const_idx = self.emitter.add_constant(const_val.clone());
                Ok(BytecodeInstruction::new(
                    opcode::LOAD_CONST,
                    vec![dst_reg, (const_idx as u16) as u8, (const_idx >> 8) as u8],
                ))
            }
            Operand::Local(local_idx) => Ok(BytecodeInstruction::new(
                opcode::LOAD_LOCAL,
                vec![dst_reg, *local_idx as u8],
            )),
            Operand::Arg(arg_idx) => Ok(BytecodeInstruction::new(
                opcode::LOAD_ARG,
                vec![dst_reg, *arg_idx as u8],
            )),
            _ => {
                let src_reg = self.operand_resolver.to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    opcode::MOV,
                    vec![dst_reg, src_reg],
                ))
            }
        }
    }

    fn translate_store(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        if let Operand::Local(local_idx) = dst {
            let src_reg = self.operand_resolver.to_reg(src)?;
            Ok(BytecodeInstruction::new(
                opcode::STORE_LOCAL,
                vec![*local_idx as u8, src_reg],
            ))
        } else {
            Err(ErrorCodeDefinition::codegen_invalid_operand("invalid operand").build())
        }
    }

    fn translate_binary_op(
        &mut self,
        opcode: u8,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let lhs_reg = self.operand_resolver.to_reg(lhs)?;
        let rhs_reg = self.operand_resolver.to_reg(rhs)?;

        Ok(BytecodeInstruction::new(
            opcode,
            vec![dst_reg, lhs_reg, rhs_reg],
        ))
    }

    /// 翻译比较操作，统一使用整数比较指令
    /// 注意：实际类型检查在运行时通过 executor.rs 的 exec_compare 完成
    fn translate_compare(
        &mut self,
        eq_opcode: u8,
        _ne_opcode: u8,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        // 统一使用整数比较指令，运行时通过 exec_compare 根据实际类型执行正确比较
        self.translate_binary_op(eq_opcode, dst, lhs, rhs)
    }

    fn translate_unary_op(
        &mut self,
        opcode: u8,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(opcode, vec![dst_reg, src_reg]))
    }

    fn translate_jmp(
        &mut self,
        _target: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        Ok(BytecodeInstruction::new(opcode::JMP, vec![0, 0, 0, 0]))
    }

    fn translate_jmp_if(
        &mut self,
        cond: &Operand,
        _target: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let cond_reg = self.operand_resolver.to_reg(cond)?;
        Ok(BytecodeInstruction::new(
            opcode::JMP_IF,
            vec![cond_reg, 0, 0, 0, 0],
        ))
    }

    fn translate_jmp_if_not(
        &mut self,
        cond: &Operand,
        _target: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let cond_reg = self.operand_resolver.to_reg(cond)?;
        Ok(BytecodeInstruction::new(
            opcode::JMP_IF_NOT,
            vec![cond_reg, 0, 0, 0, 0],
        ))
    }

    fn translate_ret(
        &mut self,
        value: &Option<Operand>,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        match value {
            Some(v) => {
                let reg = self.operand_resolver.to_reg(v)?;
                Ok(BytecodeInstruction::new(opcode::RETURN_VALUE, vec![reg]))
            }
            None => Ok(BytecodeInstruction::new(opcode::RETURN, vec![])),
        }
    }

    fn translate_call(
        &mut self,
        dst: &Option<Operand>,
        func: &Operand,
        args: &[Operand],
        def: Option<crate::frontend::module::symbol::DefId>,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };

        let func_name = match func {
            Operand::Const(ConstValue::String(name)) => Some(name.clone()),
            _ => None,
        };

        // 四类调用目标（按优先级）：
        // 0. DefId 命中（Stage 3 主路径：字节码函数含构造器）→ CallStatic 函数表索引
        // 1. ExternRef FFI（ffi_func_meta 携带 mechanism/lib/symbol）→ CallNative 带元数据
        // 2. 名字命中（测试手工构造 IR 无 def 的回退路径）→ CallStatic 函数表索引
        // 3. std 原生/外部名（def 未命中函数表）→ CallNative 无元数据按名 FFI
        let ffi_meta = func_name
            .as_ref()
            .and_then(|n| self.ffi_func_meta.get(n).cloned());
        let def_idx = def.and_then(|d| self.function_def_to_idx.get(&d).copied());
        let bytecode_idx = func_name.as_ref().and_then(|n| {
            self.function_name_to_idx
                .as_ref()
                .and_then(|m| m.get(n).copied())
        });

        let (opcode, func_id) = if let Some(idx) = def_idx {
            (opcode::CALL_STATIC, idx as u32)
        } else if ffi_meta.is_some() {
            let const_idx = self
                .emitter
                .add_constant(ConstValue::String(func_name.clone().unwrap()));
            (opcode::CALL_NATIVE, const_idx as u32)
        } else if let Some(idx) = bytecode_idx {
            (opcode::CALL_STATIC, idx as u32)
        } else {
            match func {
                Operand::Const(ConstValue::Int(i)) => (opcode::CALL_STATIC, *i as u32),
                Operand::Const(ConstValue::String(name)) => {
                    let const_idx = self.emitter.add_constant(ConstValue::String(name.clone()));
                    (opcode::CALL_NATIVE, const_idx as u32)
                }
                // 禁止静默回退到 0 号函数（#251 同类：兑底曾静默吞掉元组字面量）
                other => {
                    return Err(ErrorCodeDefinition::codegen_invalid_operand(&format!(
                        "unresolvable call target in translate_call: {:?}",
                        other
                    ))
                    .build());
                }
            }
        };
        let base_arg_reg = if let Some(first_arg) = args.first() {
            self.operand_resolver.to_reg(first_arg)?
        } else {
            0
        };
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(base_arg_reg);
        // 对 FFI 函数，在 func_name_idx 后追加 mechanism/lib/symbol 的常量池索引
        if let Some(meta) = &ffi_meta {
            let mech_idx = self
                .emitter
                .add_constant(ConstValue::String(meta.mechanism.clone()));
            let lib_idx = self
                .emitter
                .add_constant(ConstValue::String(meta.lib.clone()));
            let sym_idx = self
                .emitter
                .add_constant(ConstValue::String(meta.symbol.clone()));
            operands.extend_from_slice(&mech_idx.to_le_bytes()); // 4 bytes
            operands.extend_from_slice(&lib_idx.to_le_bytes()); // 4 bytes
            operands.extend_from_slice(&sym_idx.to_le_bytes()); // 4 bytes
        }
        operands.push(args.len() as u8);
        for arg in args {
            let arg_reg = self.operand_resolver.to_reg(arg)?;
            operands.extend_from_slice(&(arg_reg as u16).to_le_bytes());
        }

        Ok(BytecodeInstruction::new(opcode, operands))
    }

    fn translate_spawn_multi(
        &mut self,
        closures: &[Operand],
        plan: &crate::middle::core::ir::ExecutionPlan,
        result: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(result)?;

        // 编码 closures
        let closure_regs: Vec<u8> = closures
            .iter()
            .map(|c| self.operand_resolver.to_reg(c))
            .collect::<Result<Vec<u8>, _>>()?;

        let mut operands = Vec::new();

        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());

        // closures.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(closure_regs.len() as u32).to_le_bytes());

        // each closure register (2 bytes LE each)
        for reg in &closure_regs {
            operands.extend_from_slice(&(*reg as u16).to_le_bytes());
        }

        // task_deps.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_deps.len() as u32).to_le_bytes());

        // for each task: deps.len(4) + deps(4*each)
        for deps in &plan.task_deps {
            operands.extend_from_slice(&(deps.len() as u32).to_le_bytes());
            for &dep in deps {
                operands.extend_from_slice(&(dep as u32).to_le_bytes());
            }
        }

        // task_resources.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_resources.len() as u32).to_le_bytes());

        // for each task: res_count(4) + for each res: str_len(4) + str_bytes
        for res in &plan.task_resources {
            operands.extend_from_slice(&(res.len() as u32).to_le_bytes());
            for s in res {
                let bytes = s.as_bytes();
                operands.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                operands.extend_from_slice(bytes);
            }
        }

        Ok(BytecodeInstruction::new(opcode::SPAWN, operands))
    }

    fn translate_spawn_from_list(
        &mut self,
        closures_list: &Operand,
        plan: &crate::middle::core::ir::ExecutionPlan,
        result: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(result)?;
        let list_reg = self.operand_resolver.to_reg(closures_list)?;

        let mut operands = Vec::new();

        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());

        // closures_list (2 bytes LE)
        operands.extend_from_slice(&(list_reg as u16).to_le_bytes());

        // task_deps.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_deps.len() as u32).to_le_bytes());

        // for each task: deps.len(4) + deps(4*each)
        for deps in &plan.task_deps {
            operands.extend_from_slice(&(deps.len() as u32).to_le_bytes());
            for &dep in deps {
                operands.extend_from_slice(&(dep as u32).to_le_bytes());
            }
        }

        // task_resources.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_resources.len() as u32).to_le_bytes());

        // for each task: res_count(4) + for each res: str_len(4) + str_bytes
        for res in &plan.task_resources {
            operands.extend_from_slice(&(res.len() as u32).to_le_bytes());
            for s in res {
                let bytes = s.as_bytes();
                operands.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                operands.extend_from_slice(bytes);
            }
        }

        Ok(BytecodeInstruction::new(opcode::SPAWN_FROM_LIST, operands))
    }

    fn translate_call_virt(
        &mut self,
        dst: &Option<Operand>,
        obj: &Operand,
        method_name: &str,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
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
        Ok(BytecodeInstruction::new(opcode::CALL_VIRT, operands))
    }

    fn translate_call_dyn(
        &mut self,
        dst: &Option<Operand>,
        func: &Operand,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };
        let obj_reg = self.operand_resolver.to_reg(func)?;
        let mut arg_regs: Vec<Reg> = Vec::new();
        for arg in args {
            let reg = self.operand_resolver.to_reg(arg)? as u16;
            arg_regs.push(Reg(reg));
        }
        let mut operands = vec![dst_reg, obj_reg];
        operands.extend_from_slice(&0u16.to_le_bytes());
        for reg in &arg_regs {
            operands.push(reg.0 as u8);
        }
        operands.push(arg_regs.len() as u8);
        Ok(BytecodeInstruction::new(opcode::CALL_DYN, operands))
    }

    fn translate_tail_call(
        &mut self,
        func: &Operand,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
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
        Ok(BytecodeInstruction::new(opcode::TAIL_CALL, operands))
    }

    fn translate_alloc(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(opcode::STACK_ALLOC, vec![dst_reg]))
    }

    fn translate_alloc_array(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            opcode::NEW_LIST_WITH_CAP,
            vec![dst_reg, 0, 0],
        ))
    }

    fn translate_load_field(
        &mut self,
        dst: &Operand,
        src: &Operand,
        field: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let field_offset = field as u16;
        Ok(BytecodeInstruction::new(
            opcode::GET_FIELD,
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
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let field_offset = field as u16;
        Ok(BytecodeInstruction::new(
            opcode::SET_FIELD,
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
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        Ok(BytecodeInstruction::new(
            opcode::LOAD_ELEMENT,
            vec![dst_reg, src_reg, index_reg],
        ))
    }

    fn translate_store_index(
        &mut self,
        dst: &Operand,
        index: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STORE_ELEMENT,
            vec![dst_reg, index_reg, src_reg],
        ))
    }

    fn translate_cast(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::CAST,
            vec![dst_reg, src_reg, 0, 0],
        ))
    }

    fn translate_heap_alloc(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            opcode::HEAP_ALLOC,
            vec![dst_reg, 0, 0],
        ))
    }

    /// 翻译 CreateStruct 指令
    /// 格式: dst(1) + type_name_idx(4) + field_count(1) + fields(2*count)
    fn translate_create_struct(
        &mut self,
        dst: &Operand,
        type_name: &str,
        fields: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let name_idx = self
            .emitter
            .add_constant(ConstValue::String(type_name.to_string())) as u32;
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&name_idx.to_le_bytes());
        operands.push(fields.len() as u8);
        for field in fields {
            let field_reg = self.operand_resolver.to_reg(field)?;
            operands.extend_from_slice(&(field_reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::CREATE_STRUCT, operands))
    }

    /// 翻译 NewDict 指令
    /// 格式: dst(2) + pair_count(4) + keys(2*count) + values(2*count)
    fn translate_new_dict(
        &mut self,
        dst: &Operand,
        keys: &[Operand],
        values: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let pair_count = keys.len() as u32;
        let mut operands = Vec::new();
        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());
        // pair_count (4 bytes LE)
        operands.extend_from_slice(&pair_count.to_le_bytes());
        // key registers (2 bytes LE each)
        for key in keys {
            let key_reg = self.operand_resolver.to_reg(key)?;
            operands.extend_from_slice(&(key_reg as u16).to_le_bytes());
        }
        // value registers (2 bytes LE each)
        for val in values {
            let val_reg = self.operand_resolver.to_reg(val)?;
            operands.extend_from_slice(&(val_reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::NEW_DICT, operands))
    }

    /// 翻译 NewTuple 指令
    /// 格式: dst(2) + item_count(4) + items(2*count)
    fn translate_new_tuple(
        &mut self,
        dst: &Operand,
        items: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let item_count = items.len() as u32;
        let mut operands = Vec::new();
        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());
        // item_count (4 bytes LE)
        operands.extend_from_slice(&item_count.to_le_bytes());
        // item registers (2 bytes LE each)
        for item in items {
            let item_reg = self.operand_resolver.to_reg(item)?;
            operands.extend_from_slice(&(item_reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::NEW_TUPLE, operands))
    }

    fn translate_make_closure(
        &mut self,
        dst: &Operand,
        func_name: &str,
        def: Option<crate::frontend::module::symbol::DefId>,
        env: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        // DefId 主路径（Stage 3）；名字路径为测试手工构造 IR 的回退
        let func_id = if let Some(idx) = def.and_then(|d| self.function_def_to_idx.get(&d)) {
            *idx as u32
        } else {
            let name_to_idx = self.function_name_to_idx.as_ref().ok_or_else(|| {
                ErrorCodeDefinition::internal_error(
                    "translate_make_closure 调用时 function_name_to_idx 未初始化",
                )
                .build()
            })?;
            name_to_idx.get(func_name).copied().ok_or_else(|| {
                ErrorCodeDefinition::internal_error(&format!(
                    "闭包函数 '{}' 未在函数名映射表中注册",
                    func_name
                ))
                .build()
            })? as u32
        };
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(env.len() as u8);
        for op in env {
            let reg = self.operand_resolver.to_reg(op)?;
            operands.extend_from_slice(&(reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::MAKE_CLOSURE, operands))
    }

    fn translate_drop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::DROP, vec![reg]))
    }

    fn translate_push(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::MOV, vec![reg]))
    }

    fn translate_pop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::MOV, vec![reg]))
    }

    fn translate_arc_new(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::ARC_NEW,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_rc_new(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::RC_NEW,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_arc_clone(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::ARC_CLONE,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_arc_drop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::ARC_DROP, vec![reg]))
    }

    fn translate_string_length(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_LENGTH,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_string_concat(
        &mut self,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let lhs_reg = self.operand_resolver.to_reg(lhs)?;
        let rhs_reg = self.operand_resolver.to_reg(rhs)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_CONCAT,
            vec![dst_reg, lhs_reg, rhs_reg],
        ))
    }

    fn translate_string_get_char(
        &mut self,
        dst: &Operand,
        src: &Operand,
        index: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_GET_CHAR,
            vec![dst_reg, src_reg, index_reg],
        ))
    }

    fn translate_string_from_int(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_FROM_INT,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_string_from_float(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_FROM_FLOAT,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_load_upvalue(
        &mut self,
        dst: &Operand,
        upvalue_idx: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            opcode::LOAD_UPVALUE,
            vec![dst_reg, upvalue_idx as u8],
        ))
    }

    fn translate_store_upvalue(
        &mut self,
        src: &Operand,
        upvalue_idx: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STORE_UPVALUE,
            vec![src_reg, upvalue_idx as u8],
        ))
    }

    fn translate_close_upvalue(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::CLOSE_UPVALUE, vec![reg]))
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
