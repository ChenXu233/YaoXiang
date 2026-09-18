//! 闭包与类型指令族：MakeClosure / TypeOf / Cast / TypeCheck / Throw
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::value::FunctionId;
use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::BytecodeInstr;

impl Interpreter {
    /// 执行「闭包与类型」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_misc(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
            BytecodeInstr::MakeClosure {
                dst,
                func: func_idx,
                env,
            } => {
                let func_id = if (*func_idx as usize) < self.image.functions_by_id.len() {
                    FunctionId(*func_idx)
                } else {
                    eprintln!(
                        "[warn] Closure: function index {} out of range ({}), fallback to id 0",
                        func_idx,
                        self.image.functions_by_id.len()
                    );
                    FunctionId(0)
                };
                let captured_env: Vec<RuntimeValue> = env
                    .iter()
                    .map(|r| {
                        self.call_stack[fi]
                            .get_slot(r.0 as usize)
                            .cloned()
                            .unwrap_or(RuntimeValue::Void)
                    })
                    .collect();
                let closure =
                    RuntimeValue::Function(crate::backends::common::value::FunctionValue {
                        func_id,
                        env: captured_env,
                    });
                self.call_stack[fi].set_slot(dst.0 as usize, closure);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Type operations ──────────────────────────────────
            BytecodeInstr::TypeOf { dst, src } => {
                let val = self.force_slot(fi, *src)?;
                let type_name: &str = match &val {
                    RuntimeValue::Void => "Void",
                    RuntimeValue::Bool(_) => "Bool",
                    RuntimeValue::Int(_) => "Int",
                    RuntimeValue::Float(_) => "Float",
                    RuntimeValue::Char(_) => "Char",
                    RuntimeValue::String(_) => "String",
                    RuntimeValue::Bytes(_) => "Bytes",
                    RuntimeValue::Tuple(_) => "Tuple",
                    RuntimeValue::Range { .. } => "Range",
                    RuntimeValue::Array(_) => "Array",
                    RuntimeValue::List(_) => "List",
                    RuntimeValue::Dict(_) => "Dict",
                    RuntimeValue::Struct { .. } => "Struct",
                    RuntimeValue::Enum { .. } => "Enum",
                    RuntimeValue::Function(_) => "Function",
                    RuntimeValue::Arc(_) => "Arc",
                    RuntimeValue::Weak(_) => "Weak",
                    RuntimeValue::Async(_) => "Async",
                    RuntimeValue::Ptr { .. } => "Ptr",
                    RuntimeValue::OpaqueHandle { .. } => "OpaqueHandle",
                };
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::String(std::sync::Arc::from(type_name)),
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Cast {
                dst,
                src,
                target_type_id,
            } => {
                let val = self.force_slot(fi, *src)?;
                let result = match (val, *target_type_id) {
                    (RuntimeValue::Int(n), 1) => RuntimeValue::Float(n as f64),
                    (RuntimeValue::Float(f), 0) => RuntimeValue::Int(f as i64),
                    (RuntimeValue::Int(n), 2) => RuntimeValue::Bool(n != 0),
                    (RuntimeValue::Bool(b), 0) => RuntimeValue::Int(if b { 1 } else { 0 }),
                    (v, _) => v,
                };
                self.call_stack[fi].set_slot(dst.0 as usize, result);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::TypeCheck { value, type_id } => {
                let val = self.force_slot(fi, *value)?;
                let actual_id: u16 = match val {
                    RuntimeValue::Int(_) => 0,
                    RuntimeValue::Float(_) => 1,
                    RuntimeValue::Bool(_) => 2,
                    RuntimeValue::String(_) => 3,
                    RuntimeValue::Char(_) => 4,
                    RuntimeValue::Void => 5,
                    _ => u16::MAX,
                };
                if actual_id != *type_id && *type_id != u16::MAX {
                    let stack = self.capture_stack();
                    return Err(ExecutorError::runtime(
                        format!(
                            "Type mismatch: expected type_id {}, got {}",
                            type_id, actual_id
                        ),
                        stack,
                    ));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Error handling ───────────────────────────────────
            BytecodeInstr::Throw { error: _ } => {
                let stack = self.capture_stack();
                Err(ExecutorError::runtime(
                    "User thrown error".to_string(),
                    stack,
                ))
            }
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_misc 收到非本族指令: {other:?}"
            ))),
        }
    }
}
