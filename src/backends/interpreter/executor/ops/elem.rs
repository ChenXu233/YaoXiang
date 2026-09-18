//! 字段与元素指令族：StoreElement / GetField / SetField / CreateStruct / 边界检查 / 字符串
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::BytecodeInstr;

impl Interpreter {
    /// 执行「字段与元素」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_elem(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
            BytecodeInstr::GetField {
                dst,
                src,
                field_idx,
            } => {
                let obj = self.force_slot(fi, *src)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let crate::backends::common::HeapValue::Tuple(items) = &*fields.lock() {
                        if (*field_idx as usize) < items.len() {
                            self.call_stack[fi]
                                .set_slot(dst.0 as usize, items[*field_idx as usize].clone());
                        }
                    }
                } else if let RuntimeValue::Range { start, end, step } = obj {
                    // #302：Range 具名字段（start=0, end=1, step=2）
                    let v = match field_idx {
                        0 => start,
                        1 => end,
                        _ => step,
                    };
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Int(v));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::SetField {
                src,
                field_idx,
                value,
            } => {
                let obj = self.force_slot(fi, *src)?;
                let val = self.force_slot(fi, *value)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let crate::backends::common::HeapValue::Tuple(items) = &mut *fields.lock() {
                        if (*field_idx as usize) < items.len() {
                            items[*field_idx as usize] = val;
                        }
                    }
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::CreateStruct {
                dst,
                type_name,
                fields,
            } => {
                let field_values: Vec<RuntimeValue> = fields
                    .iter()
                    .map(|reg| {
                        self.call_stack[fi]
                            .get_slot(reg.0 as usize)
                            .cloned()
                            .unwrap_or(RuntimeValue::Void)
                    })
                    .collect();
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(field_values));
                let vtable = self.build_vtable(type_name);
                let struct_val = RuntimeValue::Struct {
                    type_id: crate::backends::common::value::TypeId(0),
                    fields: handle,
                    vtable,
                };
                self.call_stack[fi].set_slot(dst.0 as usize, struct_val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::BoundsCheck { array, index } => {
                let arr = self.force_slot(fi, *array)?;
                let idx = self.force_slot(fi, *index)?.to_int().unwrap_or(-1);
                let len = match &arr {
                    RuntimeValue::List(h) | RuntimeValue::Tuple(h) | RuntimeValue::Array(h) => {
                        match &*h.lock() {
                            crate::backends::common::HeapValue::List(list) => list.len() as i64,
                            crate::backends::common::HeapValue::Tuple(t) => t.len() as i64,
                            _ => -1,
                        }
                    }
                    _ => -1,
                };
                if idx < 0 || idx >= len {
                    let stack = self.capture_stack();
                    // #280：越界用专用码 E6003（原 E6007 通用）
                    return Err(ExecutorError::index_out_of_bounds(
                        len.max(0) as usize,
                        idx,
                        Some(stack),
                    ));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── String operations ────────────────────────────────
            BytecodeInstr::StringConcat { dst, str1, str2 } => {
                let s1: String = match self.force_slot(fi, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_slot(fi, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::String(format!("{}{}", s1, s2).into()),
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringLength { dst, src } => {
                let s: String = match self.force_slot(fi, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Int(s.len() as i64));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringEqual { dst, str1, str2 } => {
                let s1: String = match self.force_slot(fi, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_slot(fi, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::Int(if s1 == s2 { 1 } else { 0 }),
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringGetChar { dst, src, index } => {
                let s: String = match self.force_slot(fi, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let result = s
                    .chars()
                    .nth(index.0 as usize)
                    .map(|c| RuntimeValue::Char(c as u32))
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, result);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromInt { dst, src } => {
                let val = self.force_slot(fi, *src)?.to_int().ok_or_else(|| {
                    ExecutorError::type_error(
                        "StringFromInt 操作数不是 Int 类型",
                        self.capture_stack(),
                    )
                })?;
                self.call_stack[fi]
                    .set_slot(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromFloat { dst, src } => {
                let val = self.force_slot(fi, *src)?.to_float().ok_or_else(|| {
                    ExecutorError::type_error(
                        "StringFromFloat 操作数不是 Float 类型",
                        self.capture_stack(),
                    )
                })?;
                self.call_stack[fi]
                    .set_slot(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_elem 收到非本族指令: {other:?}"
            ))),
        }
    }
}
