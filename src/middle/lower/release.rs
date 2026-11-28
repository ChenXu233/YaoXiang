//! #591 阶段一：**值**（SSA 临时值）的释放事实。
//!
//! 释放事实此前只覆盖具名变量（`ReleasePlan` 的值是变量名，`emit_release_drops`
//! 只产 `Operand::Local`）。SSA 临时值——字符串拼接、容器构造、调用结果——没有
//! 任何释放点，AOT 的 RC / 字符串所有权 / 容器资源语义因此被阻塞。
//!
//! 本模块给出**最小可证子集**：只处理**直线函数**（整个函数体只有一个基本块，
//! 没有分支/回边/`Phi`）。在这类函数里「某个值的最后一次使用」是全局唯一的点，
//! 释放它不会早于任何使用——把 `Drop(Value)` 插在该指令之后即可。
//!
//! 多块函数（含 `if`/循环/`match`/`spawn`）**一律不发**：那些形态需要 CFG 上的
//! 活跃性判据（回边上的使用、`Phi` 的 incoming 前驱使用），提前释放属 D 类
//! 静默错值。先把直线子集做对，CFG 另作增量（#591 的验收要求）。
//!
//! 语义面：`DROP` 在解释器里是 NOP（`backends/interpreter/.../ops/control.rs`），
//! 故本事实对 VM 语料零影响；它服务的是 AOT 侧的所有权/RC 与槽位复用。
//!
//! 事实口径：**「此后 IR 不再读取该值」**——不是「该对象此刻被销毁」。后端按自己的
//! 所有权模型消费它（RC 把 `Drop(v)` 读作「v 少一个持有者」；标量路径是 NOP）。
//! 因此前置条件是后端拷贝语义（`Load`/`Move`/`Store`）让每个持有者各持一份引用，
//! 否则释放点必须由后端自己再收紧——这条前提写在这里，不埋在实现里。

use crate::frontend::core::types::trait_data::TraitTable;
use crate::frontend::core::types::MonoType;
use crate::middle::ir::ir::{BasicBlock, Instruction, Operand, ValueId, ValueInfo};
use crate::middle::lower::AstToIrGenerator;

impl AstToIrGenerator {
    /// 直线函数内的值释放事实：对每个「最后一次使用落在本块」的值，在使用点之后
    /// 插一条 `Drop(Value)`。
    ///
    /// 只在 `blocks.len() == 1` 时生效（见模块文档）。插入位置是**使用点之后**：
    /// regalloc 的槽位复用从该值的下一个区间才开始，Drop 读到的仍是该值。
    pub(in crate::middle) fn emit_value_release_drops(
        &mut self,
        blocks: &mut [BasicBlock],
        values: &[ValueInfo],
    ) {
        if blocks.len() != 1 {
            return;
        }
        let span = self.cur_span;
        let block = &mut blocks[0];

        // 1. 每个值的最后一次使用下标（单块函数即全函数的使用点）
        let mut last_use: Vec<Option<usize>> = vec![None; values.len()];
        for (idx, instr) in block.instructions.iter().enumerate() {
            for op in instr.use_operands() {
                if let Operand::Value(id) = op {
                    if let Some(slot) = last_use.get_mut(id.0 as usize) {
                        *slot = Some(idx);
                    }
                }
            }
        }

        // 2. 逐值判定：类型需释放 + 最后一次使用不是 `Ret`（返回值随返回逸出）
        let mut drops_at: Vec<(usize, Vec<ValueId>)> = Vec::new();
        for (vid, slot) in last_use.iter().enumerate() {
            let Some(idx) = *slot else { continue };
            if matches!(block.instructions[idx], Instruction::Ret { .. }) {
                continue;
            }
            let Some(info) = values.get(vid) else {
                continue;
            };
            if !needs_release(&info.ty) {
                continue;
            }
            match drops_at.last_mut() {
                Some((last_idx, ids)) if *last_idx == idx => ids.push(ValueId(vid as u32)),
                _ => drops_at.push((idx, vec![ValueId(vid as u32)])),
            }
        }

        // 3. 从后往前插，下标不漂移；同一使用点的多个值按 id 升序（确定性）
        for (idx, mut ids) in drops_at.into_iter().rev() {
            ids.sort_unstable_by_key(|id| id.0);
            for id in ids.into_iter().rev() {
                block.instructions.insert(
                    idx + 1,
                    Instruction::Drop {
                        src: Operand::Value(id),
                        span,
                    },
                );
            }
        }
    }
}

/// 该类型的值是否需要释放（拥有堆资源）。
///
/// 判据取**编译器已认定的原语值类型**（`TraitData::is_primitive_value_type`：
/// Int/Float/Bool/Char 由编译器复制），再排除无资源的位：`Void`/`Never`/字面量/
/// 元类型/推断变量、借用形态 `Ref`、以及显式未知 `TypeRef("?")`（不猜）。
fn needs_release(ty: &MonoType) -> bool {
    match ty {
        MonoType::Void
        | MonoType::Never
        | MonoType::Literal { .. }
        | MonoType::MetaType { .. }
        | MonoType::TypeVar(_) => false,
        // 借用形态：所有权不在值本身（释放由所有者负责）
        MonoType::Ref { .. } => false,
        // 显式未知：不猜
        MonoType::TypeRef(name) if name == "?" => false,
        other => !TraitTable::is_primitive_value_type(other),
    }
}
