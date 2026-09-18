//! Debugger implementation for YaoXiang bytecode interpreter
//!
//! This module contains the DebuggableExecutor trait implementation and the
//! core stepping engine (step_one / execute_instr / run_until_stop).

use crate::backends::{DebuggableExecutor, ExecutorError, ExecutorResult};
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{BytecodeInstr, ConstValue, Label};
use super::executor::Interpreter;

/// 族方法返回类型：`Some(outcome)` 表示已处理，`None` 表示不属于该族。
///
/// 用 `Option` 包裹而非直接返回 outcome，是为了让 `execute_instr` 的分派
/// 链可短路——但实际实现中分派由 `match` 完成，族方法总是命中，
/// 故此处直接返回 `ExecutorResult<StepOutcome>`。
pub(super) type ExecResult = crate::backends::ExecutorResult<StepOutcome>;

/// Outcome of a single instruction execution.
pub(super) enum StepOutcome {
    /// Instruction executed normally; continue to next.
    Continue,
    /// Function returned (frame already popped).
    Returned,
}

/// Reason the debugger stopped execution.
pub(super) enum StopReason {
    Breakpoint,
    Returned,
    Completed,
}

/// #279：索引值 → usize；非 Int 报类型错误，负数报运行时错误（不再静默当 0）
/// #300 D 项：len 为真实容器长度，负索引诊断携带原始 i64（不再是 max=0 哨兵）
/// 索引值 → usize；非 Int 报类型错误，负数报运行时错误（不再静默当 0）。
///
/// `index_slot` 记录索引所在的寄存器号，供诊断层反查局部变量名——
/// `a[k]` 越界时告诉用户“k 这个变量越界了”比只给数值有用。
/// 传 None 表示索引来自编译器临时寄存器（无源码名）。
pub(super) fn index_arg_at(
    idx: &RuntimeValue,
    what: &str,
    len: usize,
    index_slot: Option<usize>,
) -> ExecutorResult<usize> {
    let i = idx
        .to_int()
        .ok_or_else(|| ExecutorError::type_only(format!("{what} index must be an Int")))?;
    usize::try_from(i).map_err(|_| {
        // #299 §4: 负索引归并到 IndexOutOfBounds（E6003），不再落通用 E6007
        ExecutorError::IndexOutOfBounds {
            max: len,
            index: i,
            index_slot,
            stack: None,
        }
    })
}

impl Interpreter {
    /// 从常量池解析字符串（变体组名等）
    pub(in crate::backends::interpreter::executor) fn const_string(
        &self,
        idx: u16,
    ) -> String {
        self.image
            .constants
            .get(idx as usize)
            .and_then(|c| {
                if let ConstValue::String(s) = c {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    /// Decode a Label into a signed offset for relative jumps.
    pub(in crate::backends::interpreter::executor) fn decode_label_offset(label: Label) -> i32 {
        i32::from_le_bytes([
            label.0 as u8,
            (label.0 >> 8) as u8,
            (label.0 >> 16) as u8,
            (label.0 >> 24) as u8,
        ])
    }

    /// Execute a single instruction. The core of the stepping engine.
    ///
    /// ## 帧原地驻留（不再 pop/push）
    ///
    /// 此前实现把顶部帧从 `call_stack` 弹出、执行、再压回：在帧上执行需要
    /// `&mut self`（改堆/运行时），而帧存于 `self.call_stack`，二者借用冲突，
    /// 故只能先弹出。代价是每条指令搬一次 `Frame`（含 3 个 `Vec` 与槽位数组）。
    ///
    /// 现改为按索引就地访问：所有对帧的读写都是 `self.call_stack[fi]` 形式的
    /// 瞬时借用，与 `&mut self.heap` 等顺序执行、互不重叠，因此无需弹出。
    /// 附带修正：调用者帧全程留在栈上，栈帧捕获可拿到完整调用链。
    ///
    /// 指令数据统一经 `image` 读取——`Frame` 只持 `func_id`，不按值携带函数体。
    pub(super) fn step_one(&mut self) -> ExecutorResult<StepOutcome> {
        if self.call_stack.is_empty() {
            return Ok(StepOutcome::Returned);
        }
        let fi = self.call_stack.len() - 1;

        let fid = self.call_stack[fi].func_id as usize;
        if fid >= self.image.functions_by_id.len() {
            let stack = self.capture_stack();
            return Err(ExecutorError::function_not_found(
                format!("Frame holds invalid func_id {fid}"),
                stack,
            ));
        }

        if self.call_stack[fi].ip >= self.image.functions_by_id[fid].instructions.len() {
            self.call_stack.pop();
            return Ok(StepOutcome::Returned);
        }

        let ip = self.call_stack[fi].ip;
        let instr = self.image.functions_by_id[fid].instructions[ip].clone();
        self.execute_instr(fi, &instr)
    }

    /// Execute until a stop condition (breakpoint, return, or completion).
    pub(super) fn run_until_stop(&mut self) -> ExecutorResult<StopReason> {
        loop {
            if self.has_breakpoint() {
                return Ok(StopReason::Breakpoint);
            }
            match self.step_one()? {
                StepOutcome::Continue => {}
                StepOutcome::Returned => {
                    if self.call_stack.is_empty() {
                        return Ok(StopReason::Completed);
                    }
                    return Ok(StopReason::Returned);
                }
            }
        }
    }

    /// Execute a single instruction on the frame at `call_stack[fi]`.
    ///
    /// This is the instruction dispatcher — all instruction logic lives here.
    ///
    /// 帧不再作为 `&mut Frame` 传入，而是用索引定位：所有对帧的访问都是
    /// `self.call_stack[fi]` 的瞬时借用，因此 `&mut self`（堆、运行时、
    /// 甚至嵌套调用时向同一 `call_stack` 压帧）始终可用。
    fn execute_instr(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> ExecutorResult<StepOutcome> {
        // 按指令族分派：各族实现见 `ops/` 模块。
        // 本表的族归属与 `ops/*.rs` 各自的 match 臂严格一致——不一致会导致
        // 运行时报「收到非本族指令」。新增指令时两处需同步。
        match instr {
            // ── 控制流（ops/control.rs）──────────────────────────
            BytecodeInstr::Nop
            | BytecodeInstr::Yield
            | BytecodeInstr::Drop { .. }
            | BytecodeInstr::Release { .. }
            | BytecodeInstr::StackAlloc { .. }
            | BytecodeInstr::TryBegin { .. }
            | BytecodeInstr::TryEnd
            | BytecodeInstr::ArcDrop { .. }
            | BytecodeInstr::CloseUpvalue { .. }
            | BytecodeInstr::Return
            | BytecodeInstr::ReturnValue { .. }
            | BytecodeInstr::Jmp { .. }
            | BytecodeInstr::JmpIf { .. }
            | BytecodeInstr::JmpIfNot { .. }
            | BytecodeInstr::Switch { .. } => self.exec_control(fi, instr),

            // ── 寄存器与槽位（ops/reg.rs）────────────────────────
            BytecodeInstr::Mov { .. }
            | BytecodeInstr::LoadConst { .. }
            | BytecodeInstr::LoadLocal { .. }
            | BytecodeInstr::StoreLocal { .. }
            | BytecodeInstr::LoadArg { .. }
            | BytecodeInstr::LoadGlobal { .. }
            | BytecodeInstr::StoreGlobal { .. }
            | BytecodeInstr::LoadUpvalue { .. }
            | BytecodeInstr::StoreUpvalue { .. } => self.exec_reg(fi, instr),

            // ── 算术与比较（ops/arith.rs）────────────────────────
            BytecodeInstr::BinaryOp { .. }
            | BytecodeInstr::UnaryOp { .. }
            | BytecodeInstr::Compare { .. } => self.exec_arith(fi, instr),

            // ── 调用与并发（ops/call.rs）─────────────────────────
            BytecodeInstr::CallStatic { .. }
            | BytecodeInstr::CallNative { .. }
            | BytecodeInstr::CallVirt { .. }
            | BytecodeInstr::CallDyn { .. }
            | BytecodeInstr::Spawn { .. }
            | BytecodeInstr::SpawnFromList { .. } => self.exec_call(fi, instr),

            // ── 容器与聚合（ops/container.rs）────────────────────
            BytecodeInstr::HeapAlloc { .. }
            | BytecodeInstr::NewListWithCap { .. }
            | BytecodeInstr::NewArray { .. }
            | BytecodeInstr::NewDict { .. }
            | BytecodeInstr::NewTuple { .. }
            | BytecodeInstr::NewRange { .. }
            | BytecodeInstr::CreateVariant { .. }
            | BytecodeInstr::VariantTag { .. }
            | BytecodeInstr::VariantPayload { .. }
            | BytecodeInstr::Contains { .. }
            | BytecodeInstr::LoadElement { .. }
            | BytecodeInstr::StoreElement { .. } => self.exec_container(fi, instr),

            // ── 字段与元素（ops/elem.rs）─────────────────────────
            BytecodeInstr::GetField { .. }
            | BytecodeInstr::SetField { .. }
            | BytecodeInstr::CreateStruct { .. }
            | BytecodeInstr::BoundsCheck { .. }
            | BytecodeInstr::StringConcat { .. }
            | BytecodeInstr::StringLength { .. }
            | BytecodeInstr::StringEqual { .. }
            | BytecodeInstr::StringGetChar { .. }
            | BytecodeInstr::StringFromInt { .. }
            | BytecodeInstr::StringFromFloat { .. } => self.exec_elem(fi, instr),

            // ── 引用计数（ops/arc.rs）────────────────────────────
            BytecodeInstr::ArcNew { .. }
            | BytecodeInstr::RcNew { .. }
            | BytecodeInstr::ArcClone { .. }
            | BytecodeInstr::WeakNew { .. }
            | BytecodeInstr::WeakUpgrade { .. }
            | BytecodeInstr::Borrow { .. } => self.exec_arc(fi, instr),

            // ── 闭包与类型（ops/misc.rs）─────────────────────────
            BytecodeInstr::MakeClosure { .. }
            | BytecodeInstr::TypeOf { .. }
            | BytecodeInstr::Cast { .. }
            | BytecodeInstr::TypeCheck { .. }
            | BytecodeInstr::Throw { .. } => self.exec_misc(fi, instr),
        }
    }
}

impl DebuggableExecutor for Interpreter {
    fn set_breakpoint(
        &mut self,
        offset: usize,
    ) {
        self.breakpoints.insert(offset, ());
    }

    fn remove_breakpoint(
        &mut self,
        offset: usize,
    ) {
        self.breakpoints.remove(&offset);
    }

    fn has_breakpoint(&self) -> bool {
        if let Some(frame) = self.call_stack.last() {
            self.breakpoints.contains_key(&frame.ip)
        } else {
            false
        }
    }

    fn step(&mut self) -> ExecutorResult<()> {
        self.step_one()?;
        Ok(())
    }

    fn step_over(&mut self) -> ExecutorResult<()> {
        let depth = self.call_stack.len();
        // Execute the current instruction
        self.step_one()?;
        // If it was a function call, wait for it to complete
        while self.call_stack.len() > depth {
            match self.run_until_stop()? {
                StopReason::Breakpoint | StopReason::Completed => return Ok(()),
                StopReason::Returned => {}
            }
        }
        Ok(())
    }

    fn step_out(&mut self) -> ExecutorResult<()> {
        let depth = self.call_stack.len();
        loop {
            match self.run_until_stop()? {
                StopReason::Breakpoint | StopReason::Completed => return Ok(()),
                StopReason::Returned => {
                    if self.call_stack.len() < depth {
                        return Ok(());
                    }
                }
            }
        }
    }

    fn run(&mut self) -> ExecutorResult<()> {
        loop {
            match self.run_until_stop()? {
                StopReason::Breakpoint | StopReason::Completed => return Ok(()),
                StopReason::Returned => {
                    if self.call_stack.is_empty() {
                        return Ok(());
                    }
                }
            }
        }
    }

    fn current_ip(&self) -> usize {
        self.call_stack.last().map(|f| f.ip).unwrap_or(0)
    }

    fn current_function(&self) -> Option<&str> {
        self.call_stack
            .last()
            .and_then(|f| self.image.function_name(f.func_id as usize))
    }

    fn breakpoints(&self) -> Vec<usize> {
        self.breakpoints.keys().copied().collect()
    }
}
