//! 容器与聚合指令族：堆分配 / 列表 / 数组 / 字典 / 元组 / 变体 / 元素读取
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::value::TypeId;
use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::{BytecodeInstr, Reg};
use crate::backends::interpreter::executor::debug::index_arg_at;

impl Interpreter {
    /// 执行「容器与聚合」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_container(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
            BytecodeInstr::HeapAlloc { dst, type_id: _ } => {
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(Vec::new()));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Tuple(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewListWithCap { dst, capacity } => {
                let handle = self.heap.allocate(crate::backends::common::HeapValue::List(
                    Vec::with_capacity(*capacity as usize),
                ));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::List(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewArray { dst, count } => {
                // #299 §2：定长数组——元素默认 Void 占位，后续由字面量/store 填充
                let items = vec![RuntimeValue::Void; *count as usize];
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Array(items));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Array(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewDict { dst, keys, values } => {
                let mut map = std::collections::HashMap::new();
                for (key_reg, val_reg) in keys.iter().zip(values.iter()) {
                    let key = self.call_stack[fi]
                        .get_slot(key_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    let val = self.call_stack[fi]
                        .get_slot(val_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    map.insert(key, val);
                }
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Dict(map));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Dict(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewTuple { dst, items } => {
                let mut tuple_items = Vec::with_capacity(items.len());
                for item_reg in items {
                    let item = self.call_stack[fi]
                        .get_slot(item_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    tuple_items.push(item);
                }
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(tuple_items));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Tuple(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewRange {
                dst,
                start,
                end,
                step,
            } => {
                // #302：三标量内联记录，构造点已拦 step=0（字面量）；动态零走 std.range.contains 显式错误
                let read = |r: &Reg| -> i64 {
                    match self.call_stack[fi].get_slot(r.0 as usize) {
                        Some(RuntimeValue::Int(n)) => *n,
                        _ => 0,
                    }
                };
                let (s, e, p) = (read(start), read(end), read(step));
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::Range {
                        start: s,
                        end: e,
                        step: p,
                    },
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // RFC-011a §6: 包装具体值为存在类型变体（Animal$Group.Dog(payload)）
            BytecodeInstr::CreateVariant {
                dst,
                // RFC-010: group 携带和类型名——确定性派生类型身份，
                // 使跨构造路径（新机制 / std native）的值可比较、可穷尽
                group_idx,
                variant,
                payload,
            } => {
                let payload_val = self.force_slot(fi, *payload)?;
                let group = self.const_string(*group_idx);
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::Enum {
                        type_id: TypeId::from_sum_type_name(&group),
                        variant_id: *variant,
                        payload: Box::new(payload_val),
                    },
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // RFC-011a §6: 变体号提取。守卫：obj 必须是变体值——漏包装在此显式
            // 报错（四层防御第③层），绝不静默产出错误数据
            BytecodeInstr::VariantTag {
                dst,
                obj,
                group_idx,
            } => {
                let obj_val = self.force_slot(fi, *obj)?;
                let group = self.const_string(*group_idx);
                let tag = match &obj_val {
                    RuntimeValue::Enum { variant_id, .. } => *variant_id as i64,
                    other => {
                        let actual = format!("{:?}", other.value_type_simple());
                        return Err(ExecutorError::runtime_only(format!(
                                "存在类型分发收到未包装的值（期望 {} 的变体，实际是 {}）——                             值未经包装进入存在类型位置，属编译器包装点遗漏",
                                group, actual
                            )));
                    }
                };
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Int(tag));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // RFC-011a §6: 变体负载提取（守卫同 VariantTag）
            BytecodeInstr::VariantPayload {
                dst,
                obj,
                group_idx,
            } => {
                let obj_val = self.force_slot(fi, *obj)?;
                let group = self.const_string(*group_idx);
                let payload = match obj_val {
                    RuntimeValue::Enum { payload, .. } => *payload,
                    other => {
                        let actual = format!("{:?}", other.value_type_simple());
                        return Err(ExecutorError::runtime_only(format!(
                                "存在类型分发收到未包装的值（期望 {} 的变体，实际是 {}）——                             值未经包装进入存在类型位置，属编译器包装点遗漏",
                                group, actual
                            )));
                    }
                };
                self.call_stack[fi].set_slot(dst.0 as usize, payload);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // #299 §3: membership 谓词——命中 true / 未命中 false，不报错（问 vs 断言）
            BytecodeInstr::Contains {
                dst,
                elem,
                container,
            } => {
                let e = self.force_slot(fi, *elem)?;
                let cval = self.force_slot(fi, *container)?;
                let found = match &cval {
                        RuntimeValue::List(h) => matches!(
                            &*h.lock(),
                            crate::backends::common::HeapValue::List(items)
                                if items.contains(&e)
                        ),
                        RuntimeValue::Array(h) => matches!(
                            &*h.lock(),
                            crate::backends::common::HeapValue::Array(items)
                                if items.contains(&e)
                        ),
                        RuntimeValue::Tuple(h) => matches!(
                            &*h.lock(),
                            crate::backends::common::HeapValue::Tuple(items)
                                if items.contains(&e)
                        ),
                        RuntimeValue::Dict(h) => matches!(
                            &*h.lock(),
                            crate::backends::common::HeapValue::Dict(map)
                                if map.contains_key(&e)
                        ),
                        RuntimeValue::String(s) => match &e {
                            RuntimeValue::String(sub) => s.contains(sub.as_ref()),
                            RuntimeValue::Char(ch) => match char::from_u32(*ch) {
                                Some(c) => s.contains(c),
                                // 无效码点不可能存在于合法 Char 值，落到即运行时数据损坏
                                None => {
                                    return Err(ExecutorError::runtime_only(format!(
                                        "internal: invalid Char code point {ch} in 'in' check"
                                    )))
                                }
                            },
                            // #300 B 项：非 String/Char 探 String 是类型层漏拦，不静默 false
                            _ => {
                                return Err(ExecutorError::runtime_only(format!(
                                    "internal: 'in' on String with non-string element ({e}) — typecheck missed this"
                                )))
                            }
                        },
                        // #300 B 项：非容器落到这里是类型层漏拦（白名单已拦），
                        // 静默 false 是 #279 同款疾病，改为内部错误
                        _ => {
                            return Err(ExecutorError::runtime_only(format!(
                                "internal: 'in' on non-container value ({cval}) — typecheck missed this"
                            )))
                        }
                    };
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Bool(found));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadElement { dst, array, index } => {
                let arr = self.force_slot(fi, *array)?;
                let idx_value = self.force_slot(fi, *index)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "list", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::List(items) = &*handle.lock() {
                            if idx < items.len() {
                                self.call_stack[fi].set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Tuple(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "tuple", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::Tuple(items) = &*handle.lock() {
                            if idx < items.len() {
                                self.call_stack[fi].set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "array", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::Array(items) = &*handle.lock() {
                            if idx < items.len() {
                                self.call_stack[fi].set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Dict(handle) => {
                        if let crate::backends::common::HeapValue::Dict(map) = &*handle.lock() {
                            match map.get(&idx_value) {
                                Some(value) => {
                                    self.call_stack[fi].set_slot(dst.0 as usize, value.clone())
                                }
                                // #299：缺键不再静默返回 void（同 #279 方向）
                                None => {
                                    return Err(ExecutorError::KeyNotFound {
                                        key: format!("{}", idx_value),
                                        stack: Some(self.capture_stack()),
                                    });
                                }
                            }
                        }
                    }
                    // #299：不可索引类型（如 String）不再静默 void
                    _ => {
                        return Err(ExecutorError::type_only(
                            "indexing not supported on this value type",
                        ))
                    }
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreElement {
                array,
                index,
                value,
            } => {
                let arr = self.force_slot(fi, *array)?;
                let idx_value = self.force_slot(fi, *index)?;
                let val = self.force_slot(fi, *value)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "list", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::List(items) = &mut *handle.lock()
                        {
                            if idx < items.len() {
                                items[idx] = val;
                            } else if idx == items.len() {
                                items.push(val);
                            } else {
                                // #279：越界写不再静默丢弃；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名（#360）
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "array", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::Array(items) =
                            &mut *handle.lock()
                        {
                            if idx < items.len() {
                                items[idx] = val;
                            } else {
                                // #279：越界写不再静默丢弃；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名（#360）
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Dict(handle) => {
                        if let crate::backends::common::HeapValue::Dict(map) = &mut *handle.lock() {
                            map.insert(idx_value, val);
                        }
                    }
                    _ => {}
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_container 收到非本族指令: {other:?}"
            ))),
        }
    }
}
