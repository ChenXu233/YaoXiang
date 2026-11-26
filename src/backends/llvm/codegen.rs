//! 已证明子集的 LLVM 发射。类型与能力判定全部来自 `analysis`；
//! 值走独立映射，控制流按 `BasicBlock` 建 LLVM 基本块（Phi 仍由 analysis 拒绝）。

use std::collections::HashMap;
use std::fmt::Display;

use anyhow::{anyhow, bail, Result};
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, PointerValue,
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

struct RuntimeFunctions<'ctx> {
    add: FunctionValue<'ctx>,
    sub: FunctionValue<'ctx>,
    mul: FunctionValue<'ctx>,
    div: FunctionValue<'ctx>,
    rem: FunctionValue<'ctx>,
    neg: FunctionValue<'ctx>,
    assert: FunctionValue<'ctx>,
}

pub(super) struct Emitter<'ctx> {
    context: &'ctx inkwell::context::Context,
    module: Module<'ctx>,
    builder: inkwell::builder::Builder<'ctx>,
    analysis: ModuleAnalysis,
    source_name: String,
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
            self.emit_cfg(
                value,
                &signature,
                blocks,
                &self.analysis.functions[&function.name].clone(),
            )?;
        }

        let init_block = BasicBlock {
            label: 0,
            instructions: ir.init.clone(),
            successors: Vec::new(),
        };
        self.emit_cfg(
            init,
            &init_signature,
            std::slice::from_ref(&init_block),
            &self.analysis.functions["__yx_module_init"].clone(),
        )?;
        self.emit_c_main(init)?;
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
    ) -> Result<()> {
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
        let mut flat_to_block = HashMap::new();
        let mut flat = 0;
        for (ordinal, block) in blocks.iter().enumerate() {
            for _ in &block.instructions {
                flat_to_block.insert(flat, ordinal);
                flat += 1;
            }
        }

        self.builder.position_at_end(llvm_blocks[0]);
        let mut locals = HashMap::new();
        let mut values = HashMap::new();
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

        for (ordinal, block) in blocks.iter().enumerate() {
            self.builder.position_at_end(llvm_blocks[ordinal]);
            let mut terminated = false;
            // lower 在显式返回后追加的仅含 Ret(None) 无前驱块是标量函数的
            // 不可达填充；生成 unreachable，不产生类型错误的 ret void。
            if ordinal > 0
                && signature.return_type != ScalarType::Void
                && block.instructions.len() == 1
                && matches!(block.instructions[0], Instruction::Ret { value: None, .. })
            {
                llvm(self.builder.build_unreachable())?;
                continue;
            }
            for instruction in &block.instructions {
                match instruction {
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
                        if self.emit_instruction(
                            &mut locals,
                            &mut values,
                            signature,
                            instruction,
                            analysis,
                        )? {
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
        }
        Ok(())
    }

    fn emit_instruction(
        &mut self,
        mut locals: &mut HashMap<usize, Slot<'ctx>>,
        values: &mut HashMap<u32, Value<'ctx>>,
        signature: &OwnedSignature,
        instruction: &Instruction,
        analysis: &FunctionAnalysis,
    ) -> Result<bool> {
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
                let predicate = match instruction {
                    Instruction::Eq { .. } => IntPredicate::EQ,
                    Instruction::Ne { .. } => IntPredicate::NE,
                    Instruction::Lt { .. } => IntPredicate::SLT,
                    Instruction::Le { .. } => IntPredicate::SLE,
                    Instruction::Gt { .. } => IntPredicate::SGT,
                    Instruction::Ge { .. } => IntPredicate::SGE,
                    _ => unreachable!(),
                };
                let compared = llvm(
                    self.builder
                        .build_int_compare(predicate, left, right, "compare"),
                )?;
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
            Instruction::Drop { .. } => {
                // AOT-1 的标量与只读字符串常量没有析构义务。
            }
            Instruction::Phi { .. } => {
                bail!("Phi mapping is deferred to batch 3.5 (AOT-3.5 boundary)")
            }
            Instruction::Jmp { .. } | Instruction::JmpIf { .. } | Instruction::JmpIfNot { .. } => {
                bail!("control-flow instruction reached instruction emitter")
            }
            other => bail!("unsupported instruction reached LLVM emission: {other:?}"),
        }
        Ok(matches!(instruction, Instruction::Ret { .. }))
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

    fn emit_c_main(
        &self,
        init: FunctionValue<'ctx>,
    ) -> Result<()> {
        let main_type = self.context.i32_type().fn_type(&[], false);
        let main = self.module.add_function("main", main_type, None);
        let entry = self.context.append_basic_block(main, "entry");
        self.builder.position_at_end(entry);
        llvm(self.builder.build_call(init, &[], ""))?;
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
                let slot = locals[slot];
                self.load_slot(slot)
            }
            Operand::Value(id) => values
                .get(&id.0)
                .copied()
                .ok_or_else(|| anyhow!("ValueId({}) has no LLVM definition at read time", id.0)),
            Operand::Global(index) => {
                let ty = self.analysis.globals[index];
                let Some(pointer) = self.globals.get(index).copied() else {
                    // AOT-3.1 known gap（阶段 1 支持集账本）：Global 缺指针时
                    // 当前静默返回 Void；后续批次改为带位置的显式拒绝。
                    return Ok(Value::void());
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

    fn load_slot(
        &self,
        slot: Slot<'ctx>,
    ) -> Result<Value<'ctx>> {
        let Some(pointer) = slot.pointer else {
            // AOT-3.1 known gap（阶段 1 支持集账本）：无指针槽位当前静默返回
            // Void；后续批次改为带位置的显式拒绝。
            return Ok(Value::void());
        };
        let repr = llvm(self.builder.build_load(
            llvm_type(self.context, slot.ty).expect("void has no pointer"),
            pointer,
            "load",
        ))?;
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
        let pointer = if value.ty == ScalarType::Void {
            None
        } else if let Some(existing) = locals.get(index).and_then(|slot| slot.pointer) {
            Some(existing)
        } else {
            Some(llvm(self.builder.build_alloca(
                llvm_type(self.context, value.ty).expect("void handled above"),
                &format!("slot.{index}"),
            ))?)
        };
        if let (Some(pointer), Some(repr)) = (pointer, value.repr) {
            llvm(self.builder.build_store(pointer, repr))?;
        }
        let expected = analysis.locals.get(index).copied();
        if expected != Some(value.ty) {
            bail!(
                "LLVM slot {index} type {:?} disagrees with analysis {expected:?}",
                value.ty
            )
        }
        locals.insert(
            *index,
            Slot {
                ty: value.ty,
                pointer,
            },
        );
        Ok(())
    }

    fn location_args(
        &mut self,
        span: Span,
    ) -> Result<Vec<BasicMetadataValueEnum<'ctx>>> {
        let file = self.global_string(&self.source_name.clone())?;
        Ok(vec![
            file.as_pointer_value().into(),
            self.context
                .i64_type()
                .const_int(self.source_name.len() as u64, false)
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

fn llvm_type(
    context: &inkwell::context::Context,
    ty: ScalarType,
) -> Option<BasicTypeEnum<'_>> {
    match ty {
        ScalarType::Void => None,
        ScalarType::Int => Some(context.i64_type().into()),
        ScalarType::Bool => Some(context.bool_type().into()),
        ScalarType::String => Some(
            context
                .struct_type(
                    &[
                        context.ptr_type(AddressSpace::default()).into(),
                        context.i64_type().into(),
                    ],
                    false,
                )
                .into(),
        ),
    }
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
    RuntimeFunctions {
        add: module.add_function("__yx_rt_add_i64", i64_ty.fn_type(&binary, false), None),
        sub: module.add_function("__yx_rt_sub_i64", i64_ty.fn_type(&binary, false), None),
        mul: module.add_function("__yx_rt_mul_i64", i64_ty.fn_type(&binary, false), None),
        div: module.add_function("__yx_rt_div_i64", i64_ty.fn_type(&binary, false), None),
        rem: module.add_function("__yx_rt_mod_i64", i64_ty.fn_type(&binary, false), None),
        neg: module.add_function("__yx_rt_neg_i64", i64_ty.fn_type(&unary, false), None),
        assert,
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
