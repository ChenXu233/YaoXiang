//! 已证明子集的 LLVM 发射。类型与能力判定全部来自 `analysis`；
//! 值走独立映射，控制流按 `BasicBlock` 建 LLVM 基本块。
//!
//! 汇合映射（批次 3.5）：每个块的 `Phi` 在块首发射 LLVM phi，incoming 在前驱边
//! 发射时按前驱 label 登记；`Local` 槽位的 alloca 统一提到入口块，多前驱写—
//! 汇合读由入口块 alloca 支配，分支里只保留 store。

use std::collections::HashMap;
use std::fmt::Display;

use anyhow::{anyhow, bail, Result};
use inkwell::basic_block::BasicBlock as LlvmBlock;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, PhiValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};

use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand,
};
use crate::util::span::Span;

use super::analysis::{FunctionAnalysis, ModuleAnalysis, OwnedSignature, ScalarType};

#[derive(Clone, Copy)]
struct Slot<'ctx> {
    ty: ScalarType,
    pointer: Option<PointerValue<'ctx>>,
}

#[derive(Clone, Copy)]
struct Value<'ctx> {
    ty: ScalarType,
    repr: Option<BasicValueEnum<'ctx>>,
}

impl<'ctx> Value<'ctx> {
    fn void() -> Self {
        Self {
            ty: ScalarType::Void,
            repr: None,
        }
    }
}

/// 预建的块首 phi：incoming 在前驱边发射时按 label 登记。
struct PhiSlot<'ctx> {
    instruction_index: usize,
    id: u32,
    ty: ScalarType,
    phi: PhiValue<'ctx>,
}

struct RuntimeFunctions<'ctx> {
    add: FunctionValue<'ctx>,
    sub: FunctionValue<'ctx>,
    mul: FunctionValue<'ctx>,
    div: FunctionValue<'ctx>,
    rem: FunctionValue<'ctx>,
    neg: FunctionValue<'ctx>,
    assert: FunctionValue<'ctx>,
    /// `i64(i8*, i64, i8*, i64)`：内容判等，返回 1/0（显式长度，按字节比较）。
    str_eq: FunctionValue<'ctx>,
    /// `i64(i8*, i64, i8*, i64)`：字节字典序，返回 -1/0/1。
    str_cmp: FunctionValue<'ctx>,
    /// `void(location)`：位移量越界的失败入口（E6007 + 位置，noreturn）。
    shift_overflow: FunctionValue<'ctx>,
}

pub(super) struct Emitter<'ctx> {
    context: &'ctx inkwell::context::Context,
    module: Module<'ctx>,
    builder: inkwell::builder::Builder<'ctx>,
    analysis: ModuleAnalysis,
    source_name: String,
    source_files: Vec<String>,
    current_source: String,
    functions: HashMap<String, FunctionValue<'ctx>>,
    globals: HashMap<usize, PointerValue<'ctx>>,
    runtime: RuntimeFunctions<'ctx>,
    string_serial: usize,
}

impl<'ctx> Emitter<'ctx> {
    pub(super) fn new(
        context: &'ctx inkwell::context::Context,
        analysis: ModuleAnalysis,
        source_name: &str,
    ) -> Self {
        let module = context.create_module("yaoxiang_aot");
        let builder = context.create_builder();
        let runtime = declare_runtime(context, &module);
        Self {
            context,
            module,
            builder,
            analysis,
            source_name: source_name.to_string(),
            source_files: Vec::new(),
            current_source: source_name.to_string(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            runtime,
            string_serial: 0,
        }
    }

    pub(super) fn emit(
        mut self,
        ir: &ModuleIR,
    ) -> Result<Module<'ctx>> {
        self.source_files = ir.source_files.clone();
        self.declare_globals(ir)?;
        let reachable = self.analysis.reachable.clone();
        for function in ir
            .functions
            .iter()
            .filter(|function| reachable.contains(&function.name))
        {
            self.declare_function(function)?;
        }
        let init_signature = OwnedSignature {
            params: Vec::new(),
            return_type: ScalarType::Void,
        };
        let init = self.module.add_function(
            "__yx_module_init",
            function_type(self.context, &init_signature),
            Some(Linkage::Internal),
        );
        self.functions.insert("__yx_module_init".to_string(), init);

        for function in ir
            .functions
            .iter()
            .filter(|function| reachable.contains(&function.name))
        {
            let FunctionBody::Code { blocks, .. } = &function.body else {
                bail!("type declaration reached LLVM emission")
            };
            let signature = OwnedSignature {
                params: function
                    .params
                    .iter()
                    .map(ScalarType::from_mono)
                    .collect::<Result<Vec<_>>>()?,
                return_type: ScalarType::from_mono(&function.return_type)?,
            };
            let value = self.functions[&function.name];
            let function_source = ir
                .function_files
                .get(&function.name)
                .and_then(|file_id| ir.source_files.get(*file_id))
                .cloned()
                .unwrap_or_else(|| self.source_name.clone());
            self.emit_cfg(
                value,
                &signature,
                blocks,
                &self.analysis.functions[&function.name].clone(),
                Some(&function_source),
                None,
            )?;
        }

        let init_block = BasicBlock {
            label: 0,
            instructions: ir.init.clone(),
            successors: Vec::new(),
        };
        let init_source = self.source_name.clone();
        self.emit_cfg(
            init,
            &init_signature,
            std::slice::from_ref(&init_block),
            &self.analysis.functions["__yx_module_init"].clone(),
            Some(&init_source),
            Some(&ir.init_file_ids),
        )?;
        let entry = ir
            .entry_function
            .as_ref()
            .and_then(|name| self.functions.get(name).copied());
        self.emit_c_main(init, entry)?;
        Ok(self.module)
    }

    fn declare_globals(
        &mut self,
        ir: &ModuleIR,
    ) -> Result<()> {
        for global in &ir.globals {
            let ty = self
                .analysis
                .globals
                .get(&global.index)
                .copied()
                .map_or_else(|| ScalarType::from_mono(&global.ty), Ok)?;
            let Some(llvm_ty) = llvm_type(self.context, ty) else {
                continue;
            };
            let value = self.module.add_global(
                llvm_ty,
                Some(AddressSpace::default()),
                &format!("yx.global.{}", global.index),
            );
            value.set_initializer(&llvm_ty.const_zero());
            value.set_linkage(Linkage::Internal);
            self.globals.insert(global.index, value.as_pointer_value());
        }
        Ok(())
    }

    fn declare_function(
        &mut self,
        function: &FunctionIR,
    ) -> Result<()> {
        let signature = OwnedSignature {
            params: function
                .params
                .iter()
                .map(ScalarType::from_mono)
                .collect::<Result<Vec<_>>>()?,
            return_type: ScalarType::from_mono(&function.return_type)?,
        };
        let value = self.module.add_function(
            &mangle(&function.name),
            function_type(self.context, &signature),
            Some(Linkage::Internal),
        );
        self.functions.insert(function.name.clone(), value);
        Ok(())
    }

    fn emit_cfg(
        &mut self,
        function_value: FunctionValue<'ctx>,
        signature: &OwnedSignature,
        blocks: &[BasicBlock],
        analysis: &FunctionAnalysis,
        function_source: Option<&str>,
        init_file_ids: Option<&[usize]>,
    ) -> Result<()> {
        self.current_source = function_source.unwrap_or(&self.source_name).to_string();
        if blocks.is_empty() {
            let entry = self.context.append_basic_block(function_value, "entry");
            self.builder.position_at_end(entry);
            if signature.return_type == ScalarType::Void {
                llvm(self.builder.build_return(None))?;
                return Ok(());
            }
            bail!("scalar function without blocks reached LLVM emission")
        }

        let mut llvm_blocks = Vec::with_capacity(blocks.len());
        for block in blocks {
            llvm_blocks.push(
                self.context
                    .append_basic_block(function_value, &format!("bb{}", block.label)),
            );
        }
        // 每个 IR 块当前的「发射出口」：受检移位的守卫会把块切成 ok/fail/rest
        // 三段，出口随之变成 rest；后继块的 Phi incoming 必须登记真实出口。
        let mut block_exit = llvm_blocks.clone();
        let mut label_to_ordinal = HashMap::with_capacity(blocks.len());
        for (ordinal, block) in blocks.iter().enumerate() {
            label_to_ordinal.insert(block.label, ordinal);
        }
        let mut flat_to_block = HashMap::new();
        let mut flat = 0;
        for (ordinal, block) in blocks.iter().enumerate() {
            for _ in &block.instructions {
                flat_to_block.insert(flat, ordinal);
                flat += 1;
            }
        }

        // Phi 节点先于所有指令建立：前驱边可能在本块发射前就要登记 incoming，
        // 而 phi 节点必须位于所在块首（LLVM 的分组要求）。
        let mut values: HashMap<u32, Value<'ctx>> = HashMap::new();
        let mut block_phis: Vec<Vec<PhiSlot<'ctx>>> = Vec::with_capacity(blocks.len());
        for (ordinal, block) in blocks.iter().enumerate() {
            self.builder.position_at_end(llvm_blocks[ordinal]);
            let mut phis = Vec::new();
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                let Instruction::Phi { dst, .. } = instruction else {
                    break;
                };
                let Operand::Value(id) = dst else {
                    bail!("Phi destination must be an SSA value, got {dst:?}")
                };
                let ty = analysis.value_type(*id)?;
                let name = format!("phi.v{}", id.0);
                let phi = match llvm_type(self.context, ty) {
                    Some(BasicTypeEnum::IntType(int_type)) => {
                        llvm(self.builder.build_phi(int_type, &name))?
                    }
                    Some(BasicTypeEnum::StructType(struct_type)) => {
                        llvm(self.builder.build_phi(struct_type, &name))?
                    }
                    Some(_) => bail!("Phi ValueId({}) has no supported LLVM type", id.0),
                    // Void 汇合值没有 LLVM 形态：读取方一律得到 Void，不建节点。
                    None => {
                        values.insert(id.0, Value::void());
                        continue;
                    }
                };
                values.insert(
                    id.0,
                    Value {
                        ty,
                        repr: Some(phi.as_basic_value()),
                    },
                );
                phis.push(PhiSlot {
                    instruction_index,
                    id: id.0,
                    ty,
                    phi,
                });
            }
            block_phis.push(phis);
        }

        // Local 槽位的 alloca 统一提到入口块：多前驱写—汇合读时汇合点的 load
        // 由入口块 alloca 支配；分支里只保留 store。
        self.builder.position_at_end(llvm_blocks[0]);
        let mut slots: Vec<(usize, ScalarType)> = analysis
            .locals
            .iter()
            .map(|(slot, ty)| (*slot, *ty))
            .collect();
        slots.sort_unstable_by_key(|(slot, _)| *slot);
        let mut locals: HashMap<usize, Slot<'ctx>> = HashMap::with_capacity(slots.len());
        for (slot, ty) in slots {
            let pointer = match llvm_type(self.context, ty) {
                Some(llvm_ty) => Some(llvm(
                    self.builder.build_alloca(llvm_ty, &format!("slot.{slot}")),
                )?),
                None => None,
            };
            locals.insert(slot, Slot { ty, pointer });
        }

        for (index, ty) in signature.params.iter().enumerate() {
            let repr = function_value.get_nth_param(index as u32);
            let value = if *ty == ScalarType::Void {
                Value::void()
            } else {
                Value {
                    ty: *ty,
                    repr: Some(repr.ok_or_else(|| anyhow!("missing LLVM parameter {index}"))?),
                }
            };
            self.write_local(&mut locals, &Operand::Local(index), value, analysis)?;
        }

        let mut flat_index = 0usize;
        for (ordinal, block) in blocks.iter().enumerate() {
            self.builder.position_at_end(llvm_blocks[ordinal]);
            let mut terminated = false;
            // lower 在显式返回后追加的仅含 Ret(None) 无前驱块是标量函数的
            // 不可达填充；生成 unreachable，不产生类型错误的 ret void。
            if ordinal > 0
                && signature.return_type != ScalarType::Void
                && block.successors.is_empty()
                && block.instructions.len() == 1
                && matches!(block.instructions[0], Instruction::Ret { value: None, .. })
            {
                llvm(self.builder.build_unreachable())?;
                continue;
            }
            for instruction in &block.instructions {
                if let Some(file_ids) = init_file_ids {
                    if let Some(file_id) = file_ids.get(flat_index) {
                        if let Some(file) = self.source_files.get(*file_id) {
                            self.current_source = file.clone();
                        }
                    }
                }
                flat_index += 1;
                match instruction {
                    Instruction::Phi { .. } => {}
                    Instruction::Jmp { target, .. } => {
                        let target_block = flat_to_block
                            .get(target)
                            .copied()
                            .ok_or_else(|| anyhow!("jump target {target} has no basic block"))?;
                        llvm(
                            self.builder
                                .build_unconditional_branch(llvm_blocks[target_block]),
                        )?;
                        terminated = true;
                        break;
                    }
                    Instruction::JmpIf { cond, target, .. }
                    | Instruction::JmpIfNot { cond, target, .. } => {
                        let target_block = flat_to_block
                            .get(target)
                            .copied()
                            .ok_or_else(|| anyhow!("jump target {target} has no basic block"))?;
                        if blocks.get(ordinal + 1).is_none() {
                            bail!("conditional branch at last block has no fallthrough");
                        }
                        let fallthrough_ordinal = ordinal + 1;
                        let condition = self
                            .read(&locals, &values, cond, analysis)?
                            .repr
                            .ok_or_else(|| anyhow!("branch condition has no LLVM value"))?
                            .into_int_value();
                        if target_block == fallthrough_ordinal {
                            // 条件两条边落到同一基本块：IR 的 `successors` 已去重，
                            // 汇合块的 phi 也只登记一条 incoming；发射无条件分支，
                            // 避免 LLVM 看到两条边而 phi 少一条（verifier 会拒绝）。
                            llvm(
                                self.builder
                                    .build_unconditional_branch(llvm_blocks[fallthrough_ordinal]),
                            )?;
                            terminated = true;
                            break;
                        }
                        if matches!(instruction, Instruction::JmpIf { .. }) {
                            llvm(self.builder.build_conditional_branch(
                                condition,
                                llvm_blocks[target_block],
                                llvm_blocks[fallthrough_ordinal],
                            ))?;
                        } else {
                            llvm(self.builder.build_conditional_branch(
                                condition,
                                llvm_blocks[fallthrough_ordinal],
                                llvm_blocks[target_block],
                            ))?;
                        }
                        terminated = true;
                        break;
                    }
                    _ => {
                        let (flow_terminated, split_exit) = self.emit_instruction(
                            &mut locals,
                            &mut values,
                            signature,
                            instruction,
                            analysis,
                        )?;
                        if let Some(exit) = split_exit {
                            block_exit[ordinal] = exit;
                        }
                        if flow_terminated {
                            terminated = true;
                            break;
                        }
                    }
                }
            }
            if !terminated {
                if ordinal + 1 < blocks.len() {
                    llvm(
                        self.builder
                            .build_unconditional_branch(llvm_blocks[ordinal + 1]),
                    )?;
                } else if signature.return_type == ScalarType::Void {
                    llvm(self.builder.build_return(None))?;
                } else {
                    bail!("scalar function without return reached LLVM emission")
                }
            }
            self.emit_phi_incoming(
                ordinal,
                block_exit[ordinal],
                blocks,
                &label_to_ordinal,
                &block_phis,
                &locals,
                &values,
                analysis,
            )?;
        }
        Ok(())
    }

    /// 发射一条指令。返回 `(是否终结当前块, 新的发射出口块)`：受检移位会把
    /// 当前块切成 ok/fail/rest 三段，出口块随之改变（见 `emit_checked_shift`）。
    fn emit_instruction(
        &mut self,
        mut locals: &mut HashMap<usize, Slot<'ctx>>,
        values: &mut HashMap<u32, Value<'ctx>>,
        signature: &OwnedSignature,
        instruction: &Instruction,
        analysis: &FunctionAnalysis,
    ) -> Result<(bool, Option<LlvmBlock<'ctx>>)> {
        let mut split_exit: Option<LlvmBlock<'ctx>> = None;
        match instruction {
            Instruction::Move { dst, src, .. }
            | Instruction::Load { dst, src, .. }
            | Instruction::Store { dst, src, .. } => {
                let value = self.read(&*locals, values, src, analysis)?;
                self.write(&mut *locals, values, dst, value, analysis)?;
            }
            Instruction::Add {
                dst,
                lhs,
                rhs,
                span,
            }
            | Instruction::Sub {
                dst,
                lhs,
                rhs,
                span,
            }
            | Instruction::Mul {
                dst,
                lhs,
                rhs,
                span,
            }
            | Instruction::Div {
                dst,
                lhs,
                rhs,
                span,
            }
            | Instruction::Mod {
                dst,
                lhs,
                rhs,
                span,
            } => {
                let left = self
                    .read(&locals, values, lhs, analysis)?
                    .repr
                    .unwrap()
                    .into_int_value();
                let right = self
                    .read(&locals, values, rhs, analysis)?
                    .repr
                    .unwrap()
                    .into_int_value();
                let runtime = match instruction {
                    Instruction::Add { .. } => self.runtime.add,
                    Instruction::Sub { .. } => self.runtime.sub,
                    Instruction::Mul { .. } => self.runtime.mul,
                    Instruction::Div { .. } => self.runtime.div,
                    Instruction::Mod { .. } => self.runtime.rem,
                    _ => unreachable!(),
                };
                let mut args = vec![left.into(), right.into()];
                args.extend(self.location_args(*span)?);
                let call = llvm(self.builder.build_call(runtime, &args, "checked"))?;
                let value = Value {
                    ty: ScalarType::Int,
                    repr: call.try_as_basic_value().basic(),
                };
                self.write(&mut *locals, values, dst, value, analysis)?;
            }
            Instruction::Neg { dst, src, span } => {
                let value = self
                    .read(&locals, values, src, analysis)?
                    .repr
                    .unwrap()
                    .into_int_value();
                let mut args = vec![value.into()];
                args.extend(self.location_args(*span)?);
                let call = llvm(
                    self.builder
                        .build_call(self.runtime.neg, &args, "checked_neg"),
                )?;
                let result = Value {
                    ty: ScalarType::Int,
                    repr: call.try_as_basic_value().basic(),
                };
                self.write(&mut *locals, values, dst, result, analysis)?;
            }
            Instruction::Not { dst, src, .. } => {
                let value = self
                    .read(&locals, values, src, analysis)?
                    .repr
                    .unwrap()
                    .into_int_value();
                let result = Value {
                    ty: ScalarType::Bool,
                    repr: Some(llvm(self.builder.build_not(value, "not"))?.into()),
                };
                self.write(&mut *locals, values, dst, result, analysis)?;
            }
            Instruction::Eq { dst, lhs, rhs, .. }
            | Instruction::Ne { dst, lhs, rhs, .. }
            | Instruction::Lt { dst, lhs, rhs, .. }
            | Instruction::Le { dst, lhs, rhs, .. }
            | Instruction::Gt { dst, lhs, rhs, .. }
            | Instruction::Ge { dst, lhs, rhs, .. } => {
                let left = self.read(&locals, values, lhs, analysis)?;
                let right = self.read(&locals, values, rhs, analysis)?;
                let compared = if left.ty == ScalarType::String && right.ty == ScalarType::String {
                    // 字符串比较走运行时：显式长度、UTF-8 字节字典序。内容相等不等于
                    // 指针相等——两个内容相同的独立字面量是两个全局，必须比较相等。
                    let (lhs_ptr, lhs_len) = self.string_parts(left)?;
                    let (rhs_ptr, rhs_len) = self.string_parts(right)?;
                    let runtime = match instruction {
                        Instruction::Eq { .. } | Instruction::Ne { .. } => self.runtime.str_eq,
                        _ => self.runtime.str_cmp,
                    };
                    let call = llvm(self.builder.build_call(
                        runtime,
                        &[
                            lhs_ptr.into(),
                            lhs_len.into(),
                            rhs_ptr.into(),
                            rhs_len.into(),
                        ],
                        "str.compare",
                    ))?;
                    let ordering = call
                        .try_as_basic_value()
                        .basic()
                        .ok_or_else(|| anyhow!("string comparison runtime returned no value"))?
                        .into_int_value();
                    // eq 返回 1/0、cmp 返回 -1/0/1，两者都与 0 比较得到 Bool。
                    let predicate = match instruction {
                        Instruction::Eq { .. } => IntPredicate::NE,
                        Instruction::Ne { .. } => IntPredicate::EQ,
                        Instruction::Lt { .. } => IntPredicate::SLT,
                        Instruction::Le { .. } => IntPredicate::SLE,
                        Instruction::Gt { .. } => IntPredicate::SGT,
                        Instruction::Ge { .. } => IntPredicate::SGE,
                        _ => unreachable!(),
                    };
                    llvm(self.builder.build_int_compare(
                        predicate,
                        ordering,
                        self.context.i64_type().const_zero(),
                        "str.compare.result",
                    ))?
                } else {
                    let left = left
                        .repr
                        .ok_or_else(|| anyhow!("comparison operand has no LLVM value"))?
                        .into_int_value();
                    let right = right
                        .repr
                        .ok_or_else(|| anyhow!("comparison operand has no LLVM value"))?
                        .into_int_value();
                    let predicate = match instruction {
                        Instruction::Eq { .. } => IntPredicate::EQ,
                        Instruction::Ne { .. } => IntPredicate::NE,
                        Instruction::Lt { .. } => IntPredicate::SLT,
                        Instruction::Le { .. } => IntPredicate::SLE,
                        Instruction::Gt { .. } => IntPredicate::SGT,
                        Instruction::Ge { .. } => IntPredicate::SGE,
                        _ => unreachable!(),
                    };
                    llvm(
                        self.builder
                            .build_int_compare(predicate, left, right, "compare"),
                    )?
                };
                self.write(
                    &mut locals,
                    values,
                    dst,
                    Value {
                        ty: ScalarType::Bool,
                        repr: Some(compared.into()),
                    },
                    analysis,
                )?;
            }
            Instruction::Call {
                dst,
                func,
                args,
                def,
                span,
            } => {
                let call_values = args
                    .iter()
                    .map(|arg| self.read(&*locals, values, arg, analysis))
                    .collect::<Result<Vec<_>>>()?;
                let Operand::Const(ConstValue::String(name)) = func else {
                    bail!("indirect call reached LLVM emission")
                };
                if name == "std.assert.assert" {
                    self.emit_assert(call_values, *span)?;
                    if let Some(dst) = dst {
                        self.write(&mut *locals, values, dst, Value::void(), analysis)?;
                    }
                } else {
                    let resolved = def
                        .and_then(|id| self.analysis.definitions.get(&id.0))
                        .unwrap_or(name);
                    let target = self.functions[resolved];
                    let return_type = self.analysis.signatures[resolved].return_type;
                    let call_args = call_values
                        .iter()
                        .filter_map(|value| value.repr.map(Into::into))
                        .collect::<Vec<BasicMetadataValueEnum<'_>>>();
                    let call = llvm(self.builder.build_call(target, &call_args, "call"))?;
                    if let Some(dst) = dst {
                        let returned = if return_type == ScalarType::Void {
                            Value::void()
                        } else {
                            Value {
                                ty: return_type,
                                repr: call.try_as_basic_value().basic(),
                            }
                        };
                        self.write(&mut *locals, values, dst, returned, analysis)?;
                    }
                }
            }
            Instruction::Ret { value, .. } => {
                if signature.return_type == ScalarType::Void {
                    llvm(self.builder.build_return(None))?;
                } else {
                    let returned = self.read(
                        &locals,
                        values,
                        value.as_ref().expect("analysis checked scalar return"),
                        analysis,
                    )?;
                    llvm(
                        self.builder
                            .build_return(returned.repr.as_ref().map(|v| v as &dyn BasicValue<'_>)),
                    )?;
                }
            }
            Instruction::And { dst, lhs, rhs, .. }
            | Instruction::Or { dst, lhs, rhs, .. }
            | Instruction::Xor { dst, lhs, rhs, .. } => {
                // 位运算按二进制补码逐位处理，不做值范围检查（与 VM 同一语义）。
                let left = self
                    .read(&locals, values, lhs, analysis)?
                    .repr
                    .ok_or_else(|| anyhow!("bitwise operand has no LLVM value"))?
                    .into_int_value();
                let right = self
                    .read(&locals, values, rhs, analysis)?
                    .repr
                    .ok_or_else(|| anyhow!("bitwise operand has no LLVM value"))?
                    .into_int_value();
                let bits = match instruction {
                    Instruction::And { .. } => {
                        llvm(self.builder.build_and(left, right, "bitwise.and"))?
                    }
                    Instruction::Or { .. } => {
                        llvm(self.builder.build_or(left, right, "bitwise.or"))?
                    }
                    Instruction::Xor { .. } => {
                        llvm(self.builder.build_xor(left, right, "bitwise.xor"))?
                    }
                    _ => unreachable!(),
                };
                self.write(
                    &mut *locals,
                    values,
                    dst,
                    Value {
                        ty: ScalarType::Int,
                        repr: Some(bits.into()),
                    },
                    analysis,
                )?;
            }
            Instruction::Shl {
                dst,
                lhs,
                rhs,
                span,
                ..
            }
            | Instruction::Shr {
                dst,
                lhs,
                rhs,
                span,
                ..
            }
            | Instruction::Sar {
                dst,
                lhs,
                rhs,
                span,
                ..
            } => {
                let left = self
                    .read(&locals, values, lhs, analysis)?
                    .repr
                    .ok_or_else(|| anyhow!("shift lhs has no LLVM value"))?
                    .into_int_value();
                let right = self
                    .read(&locals, values, rhs, analysis)?
                    .repr
                    .ok_or_else(|| anyhow!("shift rhs has no LLVM value"))?
                    .into_int_value();
                // 移位种类决定发射哪条 LLVM 指令：左移 shl、逻辑右移 lshr、
                // 算术右移 ashr。`>>` 由前端落到 `Sar`（算术），`Shr` 是逻辑右移。
                let arithmetic_right = match instruction {
                    Instruction::Shl { .. } => None,
                    Instruction::Shr { .. } => Some(false),
                    Instruction::Sar { .. } => Some(true),
                    _ => unreachable!(),
                };
                let (shifted, exit) =
                    self.emit_checked_shift(left, right, arithmetic_right, *span)?;
                split_exit = Some(exit);
                self.write(
                    &mut *locals,
                    values,
                    dst,
                    Value {
                        ty: ScalarType::Int,
                        repr: Some(shifted.into()),
                    },
                    analysis,
                )?;
            }
            Instruction::Drop { .. } => {
                // AOT-1 的标量与只读字符串常量没有析构义务。
            }
            Instruction::Phi { .. } => {
                bail!("Phi instruction reached LLVM instruction emitter; Phi must be grouped at block head")
            }
            Instruction::Jmp { .. } | Instruction::JmpIf { .. } | Instruction::JmpIfNot { .. } => {
                bail!("control-flow instruction reached instruction emitter")
            }
            other => bail!("unsupported instruction reached LLVM emission: {other:?}"),
        }
        Ok((matches!(instruction, Instruction::Ret { .. }), split_exit))
    }

    /// 受检移位：位移量先截断成 `u32`（与 VM 的 `b as u32` 同口径），只有 `< 64`
    /// 才发射移位指令；越界进入带位置的运行时失败块（E6007），不静默 wrap。
    ///
    /// `arithmetic_right` 决定发射哪条指令：`None` = 左移 `shl`，`Some(false)` =
    /// 逻辑右移 `lshr`，`Some(true)` = 算术右移 `ashr`。前端把 `>>` 降到 `Sar`，
    /// 因此有符号右移走 `ashr`；`Shr` 是逻辑右移，留给未来的无符号右移运算符。
    ///
    /// 守卫把当前块切成三段：`shift.ok`（移位）→ `shift.rest`（本 IR 块的余下
    /// 指令与新出口）、`shift.fail`（失败入口 + `unreachable`）。调用方必须把
    /// 返回的出口块登记到 IR 块的发射出口上，否则后继块的 Phi incoming 会登记错
    /// 前驱。
    fn emit_checked_shift(
        &mut self,
        lhs: inkwell::values::IntValue<'ctx>,
        rhs: inkwell::values::IntValue<'ctx>,
        arithmetic_right: Option<bool>,
        span: Span,
    ) -> Result<(inkwell::values::IntValue<'ctx>, LlvmBlock<'ctx>)> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(|block| block.get_parent())
            .ok_or_else(|| anyhow!("shift guard needs an enclosing function"))?;
        let amount = llvm(self.builder.build_int_truncate(
            rhs,
            self.context.i32_type(),
            "shift.amount",
        ))?;
        let in_range = llvm(self.builder.build_int_compare(
            IntPredicate::ULT,
            amount,
            self.context.i32_type().const_int(64, false),
            "shift.in_range",
        ))?;
        let ok_block = self.context.append_basic_block(function, "shift.ok");
        let fail_block = self.context.append_basic_block(function, "shift.fail");
        let rest_block = self.context.append_basic_block(function, "shift.rest");
        llvm(
            self.builder
                .build_conditional_branch(in_range, ok_block, fail_block),
        )?;

        self.builder.position_at_end(ok_block);
        let amount = llvm(self.builder.build_int_z_extend(
            amount,
            self.context.i64_type(),
            "shift.amount.i64",
        ))?;
        let shifted = match arithmetic_right {
            None => llvm(self.builder.build_left_shift(lhs, amount, "shift.value"))?,
            // `sign_extend = true` 得 ashr（算术右移），`false` 得 lshr（逻辑右移）。
            Some(sign_extend) => {
                llvm(
                    self.builder
                        .build_right_shift(lhs, amount, sign_extend, "shift.value"),
                )?
            }
        };
        llvm(self.builder.build_unconditional_branch(rest_block))?;

        self.builder.position_at_end(fail_block);
        let args = self.location_args(span)?;
        llvm(
            self.builder
                .build_call(self.runtime.shift_overflow, &args, ""),
        )?;
        llvm(self.builder.build_unreachable())?;

        self.builder.position_at_end(rest_block);
        Ok((shifted, rest_block))
    }

    fn emit_assert(
        &mut self,
        values: Vec<Value<'ctx>>,
        span: Span,
    ) -> Result<()> {
        let condition = values[0].repr.unwrap().into_int_value();
        let condition = llvm(self.builder.build_int_z_extend(
            condition,
            self.context.i64_type(),
            "assert_condition",
        ))?;
        let message = if values.len() == 2 {
            values[1]
        } else {
            self.string_value("assert condition is false")?
        };
        let repr = message.repr.unwrap().into_struct_value();
        let pointer = llvm(
            self.builder
                .build_extract_value(repr, 0, "assert_message_ptr"),
        )?;
        let length = llvm(
            self.builder
                .build_extract_value(repr, 1, "assert_message_len"),
        )?;
        let mut args = vec![condition.into()];
        args.extend(self.location_args(span)?);
        args.push(pointer.into());
        args.push(length.into());
        llvm(self.builder.build_call(self.runtime.assert, &args, ""))?;
        Ok(())
    }

    /// 取 String 值的 `(ptr, len)` 两个字段。
    ///
    /// ABI 见设计文档「运行时 ABI 约定（首版）」：`String` 是 `{ i8*, i64 }`，
    /// `len` 是 UTF-8 **字节数**（不是码点数），字符串不以 NUL 结尾——运行时
    /// 一律按显式长度处理，禁止 `strlen`/`strcmp` 一类以 NUL 为界的调用。
    fn string_parts(
        &self,
        value: Value<'ctx>,
    ) -> Result<(BasicValueEnum<'ctx>, BasicValueEnum<'ctx>)> {
        let repr = value
            .repr
            .ok_or_else(|| anyhow!("string value has no LLVM representation"))?
            .into_struct_value();
        Ok((
            llvm(self.builder.build_extract_value(repr, 0, "str.ptr"))?,
            llvm(self.builder.build_extract_value(repr, 1, "str.len"))?,
        ))
    }

    fn emit_c_main(
        &self,
        init: FunctionValue<'ctx>,
        entry_function: Option<FunctionValue<'ctx>>,
    ) -> Result<()> {
        let main_type = self.context.i32_type().fn_type(&[], false);
        let main = self.module.add_function("main", main_type, None);
        let entry = self.context.append_basic_block(main, "entry");
        self.builder.position_at_end(entry);
        llvm(self.builder.build_call(init, &[], ""))?;
        if let Some(entry_function) = entry_function {
            llvm(self.builder.build_call(entry_function, &[], ""))?;
        }
        llvm(
            self.builder
                .build_return(Some(&self.context.i32_type().const_zero())),
        )?;
        Ok(())
    }

    fn read(
        &mut self,
        locals: &HashMap<usize, Slot<'ctx>>,
        values: &HashMap<u32, Value<'ctx>>,
        operand: &Operand,
        analysis: &FunctionAnalysis,
    ) -> Result<Value<'ctx>> {
        match operand {
            Operand::Const(ConstValue::Void) => Ok(Value::void()),
            Operand::Const(ConstValue::Bool(value)) => Ok(Value {
                ty: ScalarType::Bool,
                repr: Some(
                    self.context
                        .bool_type()
                        .const_int(u64::from(*value), false)
                        .into(),
                ),
            }),
            Operand::Const(ConstValue::Int(value)) => {
                let bits = if *value == (i64::MAX as i128) + 1 {
                    1u64 << 63
                } else {
                    i64::try_from(*value)
                        .map_err(|_| anyhow!("native AOT integer literal {value} is outside i64"))?
                        as u64
                };
                Ok(Value {
                    ty: ScalarType::Int,
                    repr: Some(self.context.i64_type().const_int(bits, true).into()),
                })
            }
            Operand::Const(ConstValue::String(value)) => self.string_value(value),
            Operand::Local(slot) | Operand::Arg(slot) => {
                let Some(slot) = locals.get(slot).copied() else {
                    bail!("local slot {slot} has no entry-block alloca")
                };
                // Void 没有 LLVM 形态：Void 槽位的读取按类型物化为 Void，与
                // `Const(ConstValue::Void)` 同一规则。判据由类型驱动，不看指针是否存在；
                // 非 Void 槽位缺指针走 `load_slot` 的硬失败（不变量被破坏）。
                if slot.ty == ScalarType::Void {
                    return Ok(Value::void());
                }
                self.load_slot(slot)
            }
            Operand::Value(id) => values
                .get(&id.0)
                .copied()
                .ok_or_else(|| anyhow!("ValueId({}) has no LLVM definition at read time", id.0)),
            Operand::Global(index) => {
                let ty = self
                    .analysis
                    .globals
                    .get(index)
                    .copied()
                    .ok_or_else(|| anyhow!("global slot {index} has no established type"))?;
                // 同上：Void 全局槽位没有 LLVM 形态，读取按类型物化为 Void。
                if ty == ScalarType::Void {
                    return Ok(Value::void());
                }
                let Some(pointer) = self.globals.get(index).copied() else {
                    // 非 Void 全局槽位缺指针只可能来自未声明的槽位：分析层对
                    // 「只读且未声明」的槽位报 `global slot N is not declared`，先写入
                    // 再读取的槽位在发射该写入时已按同一张表失败。这里显式失败，
                    // 不再静默物化 Void。
                    bail!(
                        "global slot {index} of type {ty:?} has no materialized LLVM pointer; \
                         a non-Void global is only materialized when the module declares it"
                    )
                };
                self.load_slot(Slot {
                    ty,
                    pointer: Some(pointer),
                })
            }
            other => bail!(
                "unsupported operand reached LLVM emission: {other:?}; locals={:?}",
                analysis.locals
            ),
        }
    }

    /// 读取非 Void 槽位。Void 槽位没有 LLVM 形态，由调用方按类型先行返回 Void；
    /// 因此缺指针意味着「非 Void 槽位没有入口块 alloca」——发射层不变量被破坏，
    /// 必须响亮失败，不静默物化 Void。
    fn load_slot(
        &self,
        slot: Slot<'ctx>,
    ) -> Result<Value<'ctx>> {
        let Some(pointer) = slot.pointer else {
            bail!(
                "slot of type {:?} has no entry-block alloca; only Void lacks an LLVM form",
                slot.ty
            )
        };
        let Some(llvm_ty) = llvm_type(self.context, slot.ty) else {
            bail!("slot of type {:?} has no LLVM form", slot.ty)
        };
        let repr = llvm(self.builder.build_load(llvm_ty, pointer, "load"))?;
        Ok(Value {
            ty: slot.ty,
            repr: Some(repr),
        })
    }

    fn write(
        &mut self,
        locals: &mut HashMap<usize, Slot<'ctx>>,
        values: &mut HashMap<u32, Value<'ctx>>,
        operand: &Operand,
        value: Value<'ctx>,
        analysis: &FunctionAnalysis,
    ) -> Result<()> {
        match operand {
            Operand::Value(id) => {
                // 生产端未定义值的物化（`ssa::materialize_undef` 的 `Load Void`）：
                // 显式未知类型的值只允许由 Void 源定义，发射层原样记录 Void。
                if value.ty == ScalarType::Void && analysis.is_explicit_unknown_value(*id)? {
                    values.insert(id.0, value);
                    return Ok(());
                }
                let expected = analysis.value_type(*id)?;
                if expected != value.ty {
                    bail!(
                        "ValueId({}) LLVM type {:?} disagrees with analysis {expected:?}",
                        id.0,
                        value.ty
                    )
                }
                values.insert(id.0, value);
                Ok(())
            }
            Operand::Global(index) => {
                if value.ty == ScalarType::Void {
                    return Ok(());
                }
                let pointer = self.globals[index];
                llvm(self.builder.build_store(pointer, value.repr.unwrap()))?;
                Ok(())
            }
            _ => self.write_local(locals, operand, value, analysis),
        }
    }

    fn write_local(
        &mut self,
        locals: &mut HashMap<usize, Slot<'ctx>>,
        operand: &Operand,
        value: Value<'ctx>,
        analysis: &FunctionAnalysis,
    ) -> Result<()> {
        let Operand::Local(index) = operand else {
            bail!("unsupported destination reached LLVM emission: {operand:?}")
        };
        let Some(slot) = locals.get(index).copied() else {
            bail!("local slot {index} has no entry-block alloca; analysis lacks its type")
        };
        if let (Some(pointer), Some(repr)) = (slot.pointer, value.repr) {
            llvm(self.builder.build_store(pointer, repr))?;
        }
        let expected = analysis.locals.get(index).copied();
        if expected != Some(value.ty) {
            bail!(
                "LLVM slot {index} type {:?} disagrees with analysis {expected:?}",
                value.ty
            )
        }
        Ok(())
    }

    /// 在前驱块发射完终结指令后，把这条边登记到后继块的 Phi incoming。
    ///
    /// `from` 是该 IR 块的**发射出口**：受检移位的守卫把块切成多段后，真实
    /// 分支指令在出口块而不在块首，Phi incoming 必须登记出口块。
    ///
    /// `incoming` 的键是 IR 块 label，源值在前驱出口已全部可用。显式未知的
    /// incoming（生产端 `Load Void` 物化）没有具体类型证据；分析层只放行没有
    /// 使用点的合并值（live 合并值带位置拒绝），所以这里按 Phi 的 dst 类型物化
    /// 为 `undef`，不猜类型。
    #[allow(clippy::too_many_arguments)]
    fn emit_phi_incoming(
        &mut self,
        from_ordinal: usize,
        from: LlvmBlock<'ctx>,
        blocks: &[BasicBlock],
        label_to_ordinal: &HashMap<usize, usize>,
        block_phis: &[Vec<PhiSlot<'ctx>>],
        locals: &HashMap<usize, Slot<'ctx>>,
        values: &HashMap<u32, Value<'ctx>>,
        analysis: &FunctionAnalysis,
    ) -> Result<()> {
        let predecessor_label = blocks[from_ordinal].label;
        for successor_label in &blocks[from_ordinal].successors {
            let Some(&to_ordinal) = label_to_ordinal.get(successor_label) else {
                bail!("phi edge target block label {successor_label} does not exist")
            };
            for phi_slot in &block_phis[to_ordinal] {
                let Instruction::Phi { incoming, .. } =
                    &blocks[to_ordinal].instructions[phi_slot.instruction_index]
                else {
                    bail!(
                        "phi bookkeeping disagrees with block {} instruction {}",
                        blocks[to_ordinal].label,
                        phi_slot.instruction_index
                    )
                };
                let Some((_, operand)) = incoming
                    .iter()
                    .find(|(label, _)| *label == predecessor_label)
                else {
                    bail!(
                        "Phi ValueId({}) in block {} has no incoming for predecessor {predecessor_label}",
                        phi_slot.id,
                        blocks[to_ordinal].label
                    )
                };
                let value = self.read(locals, values, operand, analysis)?;
                let repr = match value.repr {
                    Some(repr) if value.ty == phi_slot.ty => repr,
                    Some(_) => bail!(
                        "Phi ValueId({}) is {:?} but its incoming value from predecessor {predecessor_label} is {:?}",
                        phi_slot.id,
                        phi_slot.ty,
                        value.ty
                    ),
                    None => self.undef_value(phi_slot.ty).ok_or_else(|| {
                        anyhow!(
                            "Phi ValueId({}) has no LLVM type for undef materialization",
                            phi_slot.id
                        )
                    })?,
                };
                phi_slot
                    .phi
                    .add_incoming(&[(&repr as &dyn BasicValue<'ctx>, from)]);
            }
        }
        Ok(())
    }

    /// 按标量类型取 `undef`：用于没有类型证据的显式未知 incoming（仅死合流；
    /// live 合并值由分析层拒绝，不会走到这里）。
    fn undef_value(
        &self,
        ty: ScalarType,
    ) -> Option<BasicValueEnum<'ctx>> {
        match ty {
            ScalarType::Void => None,
            ScalarType::Int => Some(self.context.i64_type().get_undef().into()),
            ScalarType::Bool => Some(self.context.bool_type().get_undef().into()),
            ScalarType::String => Some(string_type(self.context).get_undef().into()),
        }
    }

    fn location_args(
        &mut self,
        span: Span,
    ) -> Result<Vec<BasicMetadataValueEnum<'ctx>>> {
        let file = self.global_string(&self.current_source.clone())?;
        Ok(vec![
            file.as_pointer_value().into(),
            self.context
                .i64_type()
                .const_int(self.current_source.len() as u64, false)
                .into(),
            self.context
                .i64_type()
                .const_int(span.start.line as u64, false)
                .into(),
            self.context
                .i64_type()
                .const_int(span.start.column as u64, false)
                .into(),
        ])
    }

    fn string_value(
        &mut self,
        value: &str,
    ) -> Result<Value<'ctx>> {
        let pointer = self.global_string(value)?;
        let repr = self.context.const_struct(
            &[
                pointer.as_pointer_value().into(),
                self.context
                    .i64_type()
                    .const_int(value.len() as u64, false)
                    .into(),
            ],
            false,
        );
        Ok(Value {
            ty: ScalarType::String,
            repr: Some(repr.into()),
        })
    }

    fn global_string(
        &mut self,
        value: &str,
    ) -> Result<inkwell::values::GlobalValue<'ctx>> {
        self.string_serial += 1;
        llvm(
            self.builder
                .build_global_string_ptr(value, &format!("yx.string.{}", self.string_serial)),
        )
    }
}

/// 标量到 LLVM 形态的映射。Void 是唯一没有值表示的标量——发射层「槽位缺指针只可能
/// 是 Void」的不可达证明依赖这条性质，判据为 tests/codegen.rs 的
/// test_only_void_scalar_has_no_llvm_form。
pub(super) fn llvm_type(
    context: &inkwell::context::Context,
    ty: ScalarType,
) -> Option<BasicTypeEnum<'_>> {
    match ty {
        ScalarType::Void => None,
        ScalarType::Int => Some(context.i64_type().into()),
        ScalarType::Bool => Some(context.bool_type().into()),
        ScalarType::String => Some(string_type(context).into()),
    }
}

fn string_type(context: &inkwell::context::Context) -> StructType<'_> {
    context.struct_type(
        &[
            context.ptr_type(AddressSpace::default()).into(),
            context.i64_type().into(),
        ],
        false,
    )
}

fn function_type<'ctx>(
    context: &'ctx inkwell::context::Context,
    signature: &OwnedSignature,
) -> inkwell::types::FunctionType<'ctx> {
    let params = signature
        .params
        .iter()
        .filter_map(|ty| llvm_type(context, *ty))
        .map(BasicMetadataTypeEnum::from)
        .collect::<Vec<_>>();
    llvm_type(context, signature.return_type).map_or_else(
        || context.void_type().fn_type(&params, false),
        |ty| ty.fn_type(&params, false),
    )
}

fn declare_runtime<'ctx>(
    context: &'ctx inkwell::context::Context,
    module: &Module<'ctx>,
) -> RuntimeFunctions<'ctx> {
    let i64_ty = context.i64_type();
    let ptr_ty = context.ptr_type(AddressSpace::default());
    let location = [ptr_ty.into(), i64_ty.into(), i64_ty.into(), i64_ty.into()];
    let binary = [
        i64_ty.into(),
        i64_ty.into(),
        location[0],
        location[1],
        location[2],
        location[3],
    ];
    let unary = [
        i64_ty.into(),
        location[0],
        location[1],
        location[2],
        location[3],
    ];
    let assert = module.add_function(
        "__yx_rt_assert",
        context.void_type().fn_type(
            &[
                i64_ty.into(),
                location[0],
                location[1],
                location[2],
                location[3],
                ptr_ty.into(),
                i64_ty.into(),
            ],
            false,
        ),
        None,
    );
    // 受检移位的失败入口：只带位置参数，运行期 noreturn。
    let shift_overflow = module.add_function(
        "__yx_rt_shift_overflow",
        context
            .void_type()
            .fn_type(&[location[0], location[1], location[2], location[3]], false),
        None,
    );
    // 字符串比较：两个 `{ptr, len}` 按值展开成四个标量参数，返回 i64。
    let string_compare = [ptr_ty.into(), i64_ty.into(), ptr_ty.into(), i64_ty.into()];
    let str_eq = module.add_function(
        "__yx_rt_str_eq",
        i64_ty.fn_type(&string_compare, false),
        None,
    );
    let str_cmp = module.add_function(
        "__yx_rt_str_cmp",
        i64_ty.fn_type(&string_compare, false),
        None,
    );
    RuntimeFunctions {
        add: module.add_function("__yx_rt_add_i64", i64_ty.fn_type(&binary, false), None),
        sub: module.add_function("__yx_rt_sub_i64", i64_ty.fn_type(&binary, false), None),
        mul: module.add_function("__yx_rt_mul_i64", i64_ty.fn_type(&binary, false), None),
        div: module.add_function("__yx_rt_div_i64", i64_ty.fn_type(&binary, false), None),
        rem: module.add_function("__yx_rt_mod_i64", i64_ty.fn_type(&binary, false), None),
        neg: module.add_function("__yx_rt_neg_i64", i64_ty.fn_type(&unary, false), None),
        assert,
        str_eq,
        str_cmp,
        shift_overflow,
    }
}

fn mangle(name: &str) -> String {
    let mut result = String::from("yx.fn.");
    for byte in name.as_bytes() {
        use std::fmt::Write as _;
        write!(result, "{byte:02x}").expect("writing into String cannot fail");
    }
    result
}

fn llvm<T, E: Display>(result: Result<T, E>) -> Result<T> {
    result.map_err(|e| anyhow!("LLVM emission failed: {e}"))
}
