//! 线性扫描寄存器分配器（7b.4）：把 SSA 值（`ValueId`）映射到 u8 帧槽。
//!
//! 设计依据：04-ssa.md §寄存器分配器（自研线性扫描、无寄存器文件 ⇒ 退化为
//! 「按活跃区间首次可用槽位分配」）。槽空间分两段：
//!
//! - `[0, named_base)`：**具名槽**（形参 / env / 具名绑定），由 lowering 单调发放；
//! - `[pool_base, frame_slots)`：**SSA 值槽**，由本模块按活跃区间复用。
//!
//! 活跃区间用 7b.3 的块级 CFG（`BasicBlock.successors`）做后向数据流；`Phi` 的
//! `dst` 定义在汇合块入口、`incoming` 的每个 `(P, v)` 是**前驱 P 出口的使用**——
//! 与 translator 的块入口 Move 组同源（提交信息）。
//!
//! 本模块**不持有 span / Diagnostic**：预算超限返回 [`Budget`]，由调用方
//! （`lower/scope.rs` 的 `finalize_code_body`）用既有 E3014 发射（码号、文案、
//! span 面全部不变）。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::middle::ir::ir::{BasicBlock, Instruction, Operand, ValueId, ValueInfo};

/// 分配结果。
#[derive(Debug)]
pub(in crate::middle) struct Allocation {
    /// 与 `values` 同索引：`ValueId` → 帧槽（值槽 ≥ 具名基址，且 < `frame_slots`）
    pub value_slots: Vec<usize>,
    /// `locals` 表长度 = 具名基址 + 同时活跃值峰值（含 pinned 占用的具名槽）
    pub frame_slots: usize,
}

/// 预算超限（`required` 是分配后的帧槽数，`limit` 是 u8/255 上限）。
#[derive(Debug)]
pub(in crate::middle) struct Budget {
    pub required: usize,
    pub limit: usize,
}

/// 一个值的活跃区间（闭区间，单位 = 函数内展平的指令下标）。
#[derive(Debug, Clone, Copy)]
struct Interval {
    start: usize,
    end: usize,
}

/// 把每个 SSA 值分配到帧槽：`frame_slots = named_base + max_live`。
///
/// `pinned` 是「这个值必须落在该具名槽」（`slot_for_operand` 耦合改成的钉子，
/// 见提交信息）：pinned 值固定落其具名槽，其余值只用
/// `pool_base` 以上的自由槽，两段不交叉。
///
/// 返回 `Err(Budget)` 当 `frame_slots > limit`；调用方转 E3014。
pub(in crate::middle) fn allocate_value_slots(
    blocks: &[BasicBlock],
    values: &[ValueInfo],
    named_base: usize,
    pinned: &[(ValueId, usize)],
    limit: usize,
) -> Result<Allocation, Budget> {
    let value_count = values.len();

    // ── 1. 布局：块区间（展平下标）与 label → 块下标 ──────────────
    let mut block_start: Vec<usize> = Vec::with_capacity(blocks.len());
    let mut block_end: Vec<Option<usize>> = Vec::with_capacity(blocks.len());
    let mut label_to_block: HashMap<usize, usize> = HashMap::new();
    let mut cursor = 0usize;
    for (index, block) in blocks.iter().enumerate() {
        label_to_block.insert(block.label, index);
        block_start.push(cursor);
        cursor += block.instructions.len();
        block_end.push(
            cursor
                .checked_sub(1)
                .filter(|_| !block.instructions.is_empty()),
        );
    }
    let total_instrs = cursor;

    // ── 2. 定义点 / 使用点 / Phi 入边 ──────────────
    let mut def_sites: Vec<Option<usize>> = vec![None; value_count];
    let mut use_sites: Vec<Vec<usize>> = vec![Vec::new(); value_count];
    // (前驱块下标, 值)：该值必须在前驱出口活着
    let mut phi_edge_values: Vec<HashSet<u32>> = vec![HashSet::new(); blocks.len()];
    let mut in_block_uses: Vec<HashSet<u32>> = vec![HashSet::new(); blocks.len()];
    let mut in_block_defs: Vec<HashSet<u32>> = vec![HashSet::new(); blocks.len()];
    let mut first_def_in_block: Vec<BTreeMap<u32, usize>> = vec![BTreeMap::new(); blocks.len()];

    for (index, block) in blocks.iter().enumerate() {
        for (offset, instruction) in block.instructions.iter().enumerate() {
            let at = block_start[index] + offset;
            match instruction {
                Instruction::Phi { dst, incoming, .. } => {
                    // Phi dst 定义在块入口：定义点取块首指令下标
                    if let Operand::Value(id) = dst {
                        let id = id.0;
                        let head = block_start[index];
                        def_sites[id as usize] = Some(
                            def_sites[id as usize].map_or(head, |existing| existing.min(head)),
                        );
                        in_block_defs[index].insert(id);
                        first_def_in_block[index].entry(id).or_insert(head);
                    }
                    for (predecessor_label, source) in incoming {
                        let Operand::Value(id) = source else {
                            continue;
                        };
                        if let Some(&predecessor) = label_to_block.get(predecessor_label) {
                            phi_edge_values[predecessor].insert(id.0);
                        }
                    }
                }
                _ => {
                    for operand in instruction.def_operands() {
                        if let Operand::Value(id) = operand {
                            def_sites[id.0 as usize] = Some(
                                def_sites[id.0 as usize].map_or(at, |existing| existing.min(at)),
                            );
                            in_block_defs[index].insert(id.0);
                            first_def_in_block[index].entry(id.0).or_insert(at);
                        }
                    }
                    for operand in instruction.use_operands() {
                        if let Operand::Value(id) = operand {
                            use_sites[id.0 as usize].push(at);
                            in_block_uses[index].insert(id.0);
                        }
                    }
                }
            }
        }
    }

    // ── 3. 块级活跃性：live_in = gen ∪ (live_out − kill) ──────────────
    // gen 只收**向上暴露**的使用（同块内先于定义的使用）——否则单块函数体里
    // 每个值都会被拉到块首，复用会完全失效。
    let mut gen: Vec<HashSet<u32>> = vec![HashSet::new(); blocks.len()];
    for index in 0..blocks.len() {
        for &value in &in_block_uses[index] {
            let first_def = first_def_in_block[index].get(&value).copied();
            let exposed = block_start[index]..=block_end[index].unwrap_or(block_start[index]);
            let visible = use_sites[value as usize]
                .iter()
                .copied()
                .filter(|at| exposed.contains(at))
                .min();
            if let Some(at) = visible {
                if first_def.is_none_or(|def| at < def) {
                    gen[index].insert(value);
                }
            }
        }
    }
    let mut live_in: Vec<HashSet<u32>> = gen.clone();
    let mut live_out: Vec<HashSet<u32>> = vec![HashSet::new(); blocks.len()];
    loop {
        let mut changed = false;
        for index in (0..blocks.len()).rev() {
            let mut out: HashSet<u32> = phi_edge_values[index].clone();
            for successor in &blocks[index].successors {
                if let Some(&next) = label_to_block.get(successor) {
                    out.extend(live_in[next].iter().copied());
                }
            }
            let mut input: HashSet<u32> = gen[index].clone();
            for &value in out.iter() {
                if !in_block_defs[index].contains(&value) {
                    input.insert(value);
                }
            }
            if out != live_out[index] || input != live_in[index] {
                live_out[index] = out;
                live_in[index] = input;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // ── 4. 区间：start = 定义点（含 Phi 块首），end = 最后使用 / 出口块末 ──────
    let mut intervals: Vec<Interval> = vec![Interval { start: 0, end: 0 }; value_count];
    for value in 0..value_count {
        let def_site = def_sites[value].unwrap_or(0);
        let mut start = def_site;
        let mut end = def_site;
        for &at in &use_sites[value] {
            start = start.min(at);
            end = end.max(at);
        }
        intervals[value] = Interval { start, end };
    }
    // 回边兜底：使用早于定义、或活进「定义之前的块」⇒ 区间保守扩到函数末
    let mut wrap_count = 0usize;
    for index in 0..blocks.len() {
        for &value in &live_in[index] {
            let value_index = value as usize;
            if def_sites[value_index].is_some_and(|def| block_start[index] < def) {
                wrap_count += 1;
                if total_instrs > 0 {
                    intervals[value_index].end = intervals[value_index].end.max(total_instrs - 1);
                }
            } else {
                intervals[value_index].start = intervals[value_index].start.min(block_start[index]);
            }
        }
        for &value in &live_out[index] {
            let value_index = value as usize;
            if let Some(end) = block_end[index] {
                intervals[value_index].end = intervals[value_index].end.max(end);
            }
        }
    }
    let _ = wrap_count; // 契约 B：回边形态由收口按实测登记（lowering 现状不产）

    // ── 5. pinned 值固定落具名槽；其余值只用 pool_base 以上的自由槽 ──────
    let mut pinned_slot_of: HashMap<usize, usize> = HashMap::new();
    let mut pool_base = named_base;
    for (value, slot) in pinned {
        pinned_slot_of.insert(value.0 as usize, *slot);
        pool_base = pool_base.max(slot + 1);
    }

    let mut order: Vec<usize> = (0..value_count)
        .filter(|value| !pinned_slot_of.contains_key(value))
        .collect();
    order.sort_by_key(|value| (intervals[*value].start, *value));

    let mut value_slots = vec![pool_base; value_count];
    let mut active: Vec<(usize, usize)> = Vec::new();
    let mut free: BTreeSet<usize> = BTreeSet::new();
    let mut next_slot = pool_base;
    for value in order {
        let interval = intervals[value];
        let released: Vec<usize> = active
            .iter()
            .filter(|(end, _)| *end < interval.start)
            .map(|(_, slot)| *slot)
            .collect();
        if !released.is_empty() {
            active.retain(|(end, _)| *end >= interval.start);
            free.extend(released);
        }
        let slot = match free.iter().next().copied() {
            Some(slot) => {
                free.remove(&slot);
                slot
            }
            None => {
                let slot = next_slot;
                next_slot += 1;
                slot
            }
        };
        value_slots[value] = slot;
        active.push((interval.end, slot));
    }
    for (value, slot) in pinned_slot_of {
        value_slots[value] = slot;
    }

    let frame_slots = next_slot.max(pool_base).max(named_base);
    if frame_slots > limit {
        return Err(Budget {
            required: frame_slots,
            limit,
        });
    }
    Ok(Allocation {
        value_slots,
        frame_slots,
    })
}
/// 打破 translator Phi Move 组的**真并行拷贝环**（7b.3 的临时槽未落地的补偿）。
///
/// translator 的方案 A 把每个汇合块入口的 Phi 组按前驱展开为一串 Move；它明确
/// 拒绝「前序 Move 写过的槽被后序 Move 读」的环（7b.3 留待临时槽）。本函数在
/// 值槽分配之后检查同一判据：把环中某个 Phi dst 的槽改到 frame_slots 的新
/// 顶端槽（该槽不属于任何现有 Move 源/目的），等价于标准 SSA 销毁时插入临时槽，
/// 不改变 Phi 的并行语义。pinned 到具名槽的 Phi dst 不动；若环全由 pinned 值
/// 构成则保留原状，由 translator 响亮失败。
pub(in crate::middle) fn break_phi_copy_cycles(
    blocks: &[BasicBlock],
    value_slots: &mut [usize],
    frame_slots: &mut usize,
    named_base: usize,
    limit: usize,
) -> Result<(), Budget> {
    let mut groups: BTreeMap<(usize, usize), Vec<(usize, usize)>> = BTreeMap::new();
    for block in blocks {
        for instruction in &block.instructions {
            let Instruction::Phi { dst, incoming, .. } = instruction else {
                continue;
            };
            let Operand::Value(dst) = dst else {
                continue;
            };
            for (pred, src) in incoming {
                let Operand::Value(src) = src else {
                    continue;
                };
                groups
                    .entry((block.label, *pred))
                    .or_default()
                    .push((dst.0 as usize, src.0 as usize));
            }
        }
    }

    loop {
        let mut changed = false;
        for moves in groups.values() {
            let mut slots: Vec<(usize, usize)> = Vec::with_capacity(moves.len());
            let mut valid = true;
            for (dst, src) in moves {
                let (Some(&dst_slot), Some(&src_slot)) =
                    (value_slots.get(*dst), value_slots.get(*src))
                else {
                    valid = false;
                    break;
                };
                slots.push((dst_slot, src_slot));
            }
            if !valid {
                continue;
            }

            let offending = slots.iter().enumerate().find_map(|(index, (dst_slot, _))| {
                if !slots[index + 1..]
                    .iter()
                    .any(|(_, src_slot)| src_slot == dst_slot)
                {
                    return None;
                }
                let (dst_value, _) = moves[index];
                if value_slots[dst_value] < named_base {
                    None
                } else {
                    Some(dst_value)
                }
            });
            let Some(dst_value) = offending else {
                continue;
            };
            let new_slot = *frame_slots;
            if new_slot >= limit {
                return Err(Budget {
                    required: new_slot + 1,
                    limit,
                });
            }
            value_slots[dst_value] = new_slot;
            *frame_slots += 1;
            changed = true;
            break;
        }
        if !changed {
            return Ok(());
        }
    }
}
