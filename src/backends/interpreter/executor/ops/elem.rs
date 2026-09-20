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
                // 元组值：`NewTuple` 产出 `RuntimeValue::Tuple(handle)`，索引即位置。
                // 此前只处理 `Struct{fields: HeapValue::Tuple}`（另一条构造路径），
                // 导致 `t.0` 静默写成 Void（#279 同类：不得静默）。
                if let RuntimeValue::Tuple(handle) = &obj {
                    let guard = handle.lock();
                    if let crate::backends::common::HeapValue::Tuple(items) = &*guard {
                        if (*field_idx as usize) < items.len() {
                            let v = items[*field_idx as usize].clone();
                            drop(guard);
                            self.call_stack[fi].set_slot(dst.0 as usize, v);
                            self.call_stack[fi].advance();
                            return Ok(StepOutcome::Continue);
                        }
                    }
                    drop(guard);
                    return Err(ExecutorError::type_only(format!(
                        "tuple index {} out of range",
                        field_idx
                    )));
                }
                // 以下三条路径此前都是**静默失败**：不匹配就不写 dst，留下 Void，
                // 调用方拿到 void 却不报错（#279 同类：不得静默）。
                // 典型触发：内置 `List(T)` 的值传给名为 `List` 的库结构体的 `&List(A)`
                // 形参——类型检查按名放行（同名），运行时形状不同，取 `.length` 得 void。
                // 内置集合值（`RuntimeValue::List/Array`）：`GetField` 的 field_idx 就是
                // 元素下标。命中此处说明静态类型被当成了记录型（例如库里的 `List`
                // 与内置 `List` 同名），按下标读元素而不是静默留 Void。
                if let RuntimeValue::List(h) | RuntimeValue::Array(h) = &obj {
                    let guard = h.lock();
                    if let crate::backends::common::HeapValue::List(items)
                    | crate::backends::common::HeapValue::Array(items) = &*guard
                    {
                        if (*field_idx as usize) < items.len() {
                            let v = items[*field_idx as usize].clone();
                            drop(guard);
                            self.call_stack[fi].set_slot(dst.0 as usize, v);
                            self.call_stack[fi].advance();
                            return Ok(StepOutcome::Continue);
                        }
                    }
                    drop(guard);
                    return Err(ExecutorError::type_only(format!(
                        "field index {} out of range on list/array value",
                        field_idx
                    )));
                }
                if let RuntimeValue::Struct { fields, .. } = obj {
                    let guard = fields.lock();
                    match &*guard {
                        crate::backends::common::HeapValue::Tuple(items) => {
                            if (*field_idx as usize) < items.len() {
                                let v = items[*field_idx as usize].clone();
                                drop(guard);
                                self.call_stack[fi].set_slot(dst.0 as usize, v);
                            } else {
                                let len = items.len();
                                drop(guard);
                                return Err(ExecutorError::type_only(format!(
                                    "tuple index {} out of range (len {})",
                                    field_idx, len
                                )));
                            }
                        }
                        crate::backends::common::HeapValue::List(items)
                        | crate::backends::common::HeapValue::Array(items) => {
                            if (*field_idx as usize) < items.len() {
                                let v = items[*field_idx as usize].clone();
                                drop(guard);
                                self.call_stack[fi].set_slot(dst.0 as usize, v);
                            } else {
                                let len = items.len();
                                drop(guard);
                                return Err(ExecutorError::type_only(format!(
                                    "field index {} out of range (len {})",
                                    field_idx, len
                                )));
                            }
                        }
                        _ => {
                            drop(guard);
                            return Err(ExecutorError::type_only(format!(
                                "GetField index {} on non-record heap value",
                                field_idx
                            )));
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
                // `Vec(T).length = n`：把缓冲调整到 n 个元素（截断或补 Void）。
                // 运行时用 `RuntimeValue::List/Array` 表示；按**运行时值类型**与
                // 结构体字段写区分，故与 `Struct` 分支不冲突（见 resolve_field_index）。
                if let RuntimeValue::List(h) | RuntimeValue::Array(h) = &obj {
                    let n = match &val {
                        RuntimeValue::Int(i) if *i >= 0 => *i as usize,
                        _ => {
                            return Err(ExecutorError::type_only(
                                "length assignment expects a non-negative Int",
                            ));
                        }
                    };
                    let mut guard = h.lock();
                    match &mut *guard {
                        crate::backends::common::HeapValue::List(items)
                        | crate::backends::common::HeapValue::Array(items) => {
                            items.resize(n, RuntimeValue::Void);
                        }
                        _ => {
                            drop(guard);
                            return Err(ExecutorError::type_only(
                                "length assignment on a non-buffer handle",
                            ));
                        }
                    }
                    drop(guard);
                    self.call_stack[fi].advance();
                    return Ok(StepOutcome::Continue);
                }
                if let RuntimeValue::Struct { fields, .. } = obj {
                    let mut guard = fields.lock();
                    if let crate::backends::common::HeapValue::Tuple(items) = &mut *guard {
                        if (*field_idx as usize) < items.len() {
                            items[*field_idx as usize] = val;
                        } else {
                            let len = items.len();
                            drop(guard);
                            return Err(ExecutorError::type_only(format!(
                                "struct field index {} out of range (len {})",
                                field_idx, len
                            )));
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
                // 长度读取：String 与容器（List/Array/Tuple/Dict）共用本指令。
                // 容器长度是缓冲的当前长度（`HeapValue::len()`）；String 按字节长。
                // 注：`Vec(T)` 运行时以 `RuntimeValue::List` 表示（同为可增长缓冲的唯一存储），
                // 二者仅类型层区分，待分配/扩容原语落地后另立堆变体。
                let len = match self.force_slot(fi, *src)? {
                    RuntimeValue::String(s) => s.len() as i64,
                    RuntimeValue::List(h)
                    | RuntimeValue::Array(h)
                    | RuntimeValue::Tuple(h)
                    | RuntimeValue::Dict(h) => h.lock().len() as i64,
                    // 非长度载体：显式报错，不静默返回 0（#279/#281 同款）
                    _ => {
                        return Err(ExecutorError::type_only(
                            "length is not defined for this value type".to_string(),
                        ))
                    }
                };
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Int(len));
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
