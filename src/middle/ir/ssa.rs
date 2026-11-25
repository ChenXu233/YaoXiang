//! 7f.1 支配关系与 SSA 构造（WBS P7 / 04-ssa.md）。
//!
//! 输入是 `partition_blocks` 之后的 7b.3 产物（blocks + values），输出是
//! 每个 `Operand::Value` 单定义、且每个可达使用都被其定义块支配的新形态：
//!
//! - 计算可达块上的支配树与支配边界；
//! - 在迭代支配边界上插入 `Phi`（块首、incoming 覆盖全部前驱并按前驱 label 升序）；
//! - 按支配树重命名 `Operand::Value`；`Operand::Local` 保持存储语义，不改写；
//! - `Phi` dst 登记新的 `ValueId`/`ValueInfo`，`defining_block` 记真实块 label；
//! - 值钉子（`pins`）跟随原值的全部版本，保证 `Local` 槽与值槽的既有耦合不被切断。
//!
//! 无值表（`values.is_empty()`，即 value_mode=false 的模块 init / 顶层语句）直接跳过。
//!
//! 类型策略（04-ssa「值标识方式」的显式未知契约）：Phi 原值与全部输入均未知时
//! 保留 `TypeRef("?")`，不发明具体类型；已知输入彼此冲突才返回显式错误。
//! 未定义值统一规则：不可达块里的使用跳过；任何可达使用在某个路径上无定义的
//! `Operand::Value`（全函数 0 定义，或部分前驱未定义）都在入口物化一条
//! `Load Void`，类型记显式未知（等价既有帧槽默认 Void 语义，与 `04-ssa` 的
//! Local 存储模型一致；translator 不支持 Const Move，故不内联常量）。
//! 其余内部不一致（入口块需要 Phi、Phi 类型冲突等）返回显式错误，由
//! `lower/scope.rs` 收敛为既有 E3005，不猜类型、不写占位值。

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{BasicBlock, ConstValue, Instruction, Operand, ValueId, ValueInfo};
use crate::util::span::Span;

/// SSA 构造输出。
#[derive(Debug)]
pub struct SsaOutput {
    pub blocks: Vec<BasicBlock>,
    pub values: Vec<ValueInfo>,
    /// 原值钉子在新值表上的展开（同一槽可能对应原值/新定义/Phi 多个版本）。
    pub pins: Vec<(ValueId, usize)>,
}

/// SSA 构造失败（由调用方映射为 E3005，带源码位置）。
#[derive(Debug, Clone)]
pub struct SsaConstructionError {
    pub message: String,
    pub span: Option<Span>,
}

impl SsaConstructionError {
    fn at(
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DefSite {
    block: usize,
    instr: usize,
}

#[derive(Debug, Clone, Copy)]
struct UseSite {
    block: usize,
    instr: usize,
}

#[derive(Debug, Clone, Copy)]
struct PhiSite {
    block: usize,
    var: usize,
    dst: ValueId,
    span: Span,
}

fn is_unknown_type(ty: &MonoType) -> bool {
    matches!(ty, MonoType::TypeRef(name) if name == "?")
}

fn def_span(
    blocks: &[BasicBlock],
    def: DefSite,
) -> Span {
    blocks[def.block]
        .instructions
        .get(def.instr)
        .map(Instruction::span)
        .unwrap_or_else(Span::dummy)
}

fn adjust_jump_targets(
    blocks: &mut [BasicBlock],
    old_block_starts: &[usize],
    old_total: usize,
    head_insertions: &[usize],
) {
    let mut prefix = vec![0usize; blocks.len()];
    let mut inserted = 0usize;
    for (index, count) in head_insertions.iter().enumerate() {
        prefix[index] = inserted;
        inserted += *count;
    }
    for block in blocks {
        for instruction in &mut block.instructions {
            let target = match instruction {
                Instruction::Jmp { target, .. }
                | Instruction::JmpIf { target, .. }
                | Instruction::JmpIfNot { target, .. } => target,
                _ => continue,
            };
            if *target >= old_total {
                continue;
            }
            let target_block = old_block_starts
                .partition_point(|start| *start <= *target)
                .saturating_sub(1);
            *target += prefix.get(target_block).copied().unwrap_or(0);
        }
    }
}

fn use_span(
    blocks: &[BasicBlock],
    site: UseSite,
) -> Span {
    blocks[site.block]
        .instructions
        .get(site.instr)
        .map(Instruction::span)
        .unwrap_or_else(Span::dummy)
}

type DefUseSites = (Vec<Vec<DefSite>>, Vec<Vec<UseSite>>);

fn collect_defs_uses(
    blocks: &[BasicBlock],
    n_orig: usize,
) -> Result<DefUseSites, SsaConstructionError> {
    let mut defs: Vec<Vec<DefSite>> = vec![Vec::new(); n_orig];
    let mut uses: Vec<Vec<UseSite>> = vec![Vec::new(); n_orig];
    for (block_index, block) in blocks.iter().enumerate() {
        for (instr_index, instruction) in block.instructions.iter().enumerate() {
            for operand in instruction.def_operands() {
                let Operand::Value(id) = operand else {
                    continue;
                };
                if id.0 as usize >= n_orig {
                    return Err(SsaConstructionError::at(
                        instruction.span(),
                        format!("定义 ValueId({}) 越界（值表长 {n_orig}）", id.0),
                    ));
                }
                defs[id.0 as usize].push(DefSite {
                    block: block_index,
                    instr: instr_index,
                });
            }
            for operand in instruction.use_operands() {
                let Operand::Value(id) = operand else {
                    continue;
                };
                if id.0 as usize >= n_orig {
                    return Err(SsaConstructionError::at(
                        instruction.span(),
                        format!("使用 ValueId({}) 越界（值表长 {n_orig}）", id.0),
                    ));
                }
                uses[id.0 as usize].push(UseSite {
                    block: block_index,
                    instr: instr_index,
                });
            }
        }
    }
    Ok((defs, uses))
}

#[allow(clippy::too_many_arguments)]
fn materialize_undef(
    blocks: &mut [BasicBlock],
    entry_index: usize,
    values: &mut Vec<ValueInfo>,
    versions_by_var: &mut [Vec<ValueId>],
    var: usize,
    span: Span,
    cache: &mut HashMap<usize, ValueId>,
    head_insertions: &mut [usize],
) -> ValueId {
    if let Some(id) = cache.get(&var) {
        return *id;
    }
    let id = ValueId(values.len() as u32);
    // 物化的是入口 Load Void，类型不可知：保留显式未知，不冒充原值类型。
    values.push(ValueInfo {
        ty: MonoType::TypeRef("?".to_string()),
        debug_name: None,
        defining_block: blocks[entry_index].label,
    });
    versions_by_var[var].push(id);
    head_insertions[entry_index] += 1;
    blocks[entry_index].instructions.insert(
        0,
        Instruction::Load {
            dst: Operand::Value(id),
            src: Operand::Const(ConstValue::Void),
            span,
        },
    );
    cache.insert(var, id);
    id
}

/// 对函数体做 SSA 构造。`entry` 是入口块 label（生产路径恒 0）。
pub fn construct_ssa(
    mut blocks: Vec<BasicBlock>,
    mut values: Vec<ValueInfo>,
    entry: usize,
    pins: &[(ValueId, usize)],
) -> Result<SsaOutput, SsaConstructionError> {
    // 无值表路径（模块 init / 顶层语句）不构造，原样返回。
    if values.is_empty() {
        return Ok(SsaOutput {
            blocks,
            values,
            pins: pins.to_vec(),
        });
    }

    let n_blocks = blocks.len();
    let n_orig = values.len();
    let mut label_to_index: HashMap<usize, usize> = HashMap::new();
    for (index, block) in blocks.iter().enumerate() {
        if label_to_index.insert(block.label, index).is_some() {
            return Err(SsaConstructionError::at(
                Span::dummy(),
                format!("基本块 label {} 重复", block.label),
            ));
        }
    }
    let Some(&entry_index) = label_to_index.get(&entry) else {
        return Err(SsaConstructionError::at(
            Span::dummy(),
            format!("入口块 label {entry} 不存在"),
        ));
    };

    // 输入必须是无 Phi 的 7b.3 产物：构造器自己负责 Phi，不能吞掉半成品。
    for block in &blocks {
        if let Some(phi) = block
            .instructions
            .iter()
            .find(|instruction| matches!(instruction, Instruction::Phi { .. }))
        {
            return Err(SsaConstructionError::at(
                phi.span(),
                "SSA 构造输入已含 Phi；7f.1 只接受无 Phi 的 7b.3 产物",
            ));
        }
    }

    // 记录插入前的展平块起点：Phi / Load Void 插到块首会移动跳转目标的
    // 绝对指令下标，最后必须按“目标块之前插入了多少条”统一回填。
    let mut old_block_starts = vec![0usize; n_blocks];
    for index in 1..n_blocks {
        old_block_starts[index] =
            old_block_starts[index - 1] + blocks[index - 1].instructions.len();
    }
    let old_total = old_block_starts.last().copied().unwrap_or(0)
        + blocks.last().map_or(0, |block| block.instructions.len());
    let mut head_insertions = vec![0usize; n_blocks];

    // ── 1. 收集 def/use ─────────────────────────────
    let (mut defs, mut uses) = collect_defs_uses(&blocks, n_orig)?;

    // ── 2. 块级 CFG（前驱表保留全部结构前驱，供 Phi incoming 使用）──
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); n_blocks];
    for (index, block) in blocks.iter().enumerate() {
        for successor in &block.successors {
            let Some(&target) = label_to_index.get(successor) else {
                return Err(SsaConstructionError::at(
                    Span::dummy(),
                    format!("块 {} 的后继 label {successor} 不存在", block.label),
                ));
            };
            successors[index].push(target);
        }
    }
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); n_blocks];
    for (index, targets) in successors.iter().enumerate() {
        for &target in targets {
            predecessors[target].push(index);
        }
    }
    for preds in &mut predecessors {
        preds.sort_unstable();
        preds.dedup();
    }

    // ── 3. 可达性 / 支配树 / 支配边界 ────────────────
    let mut reachable: HashSet<usize> = HashSet::new();
    let mut work = vec![entry_index];
    while let Some(index) = work.pop() {
        if !reachable.insert(index) {
            continue;
        }
        work.extend(successors[index].iter().copied());
    }
    let mut order: Vec<usize> = reachable.iter().copied().collect();
    order.sort_unstable();

    // 全函数 0 定义且存在可达使用：旧 7b.3 读帧槽默认 Void；在入口物化一条
    // Load Void，使“隐式未定义”变成显式定义（不猜类型/值）。只有不可达
    // 使用的 0 定义值跳过——不可达块不参与 SSA 判据。部分前驱未定义由
    // Phi 填充阶段用同一 materialize_undef 处理。
    let undefined_used: Vec<usize> = (0..n_orig)
        .filter(|var| {
            defs[*var].is_empty()
                && uses[*var]
                    .iter()
                    .any(|site| reachable.contains(&site.block))
        })
        .collect();
    if !undefined_used.is_empty() {
        let mut prefix = Vec::new();
        for var in undefined_used {
            values[var].ty = MonoType::TypeRef("?".to_string());
            let span = use_span(&blocks, uses[var][0]);
            prefix.push(Instruction::Load {
                dst: Operand::Value(ValueId(var as u32)),
                src: Operand::Const(ConstValue::Void),
                span,
            });
        }
        head_insertions[entry_index] += prefix.len();
        blocks[entry_index].instructions.splice(0..0, prefix);
        let rebuilt = collect_defs_uses(&blocks, n_orig)?;
        defs = rebuilt.0;
        uses = rebuilt.1;
    }

    let mut dom: Vec<HashSet<usize>> = blocks.iter().map(|_| reachable.clone()).collect();
    dom[entry_index] = HashSet::from([entry_index]);
    let mut changed = true;
    while changed {
        changed = false;
        for &block in &order {
            if block == entry_index {
                continue;
            }
            let mut preds = predecessors[block]
                .iter()
                .copied()
                .filter(|pred| reachable.contains(pred));
            let Some(first) = preds.next() else {
                continue;
            };
            let mut next = dom[first].clone();
            for pred in preds {
                next = next.intersection(&dom[pred]).copied().collect();
            }
            next.insert(block);
            if next != dom[block] {
                dom[block] = next;
                changed = true;
            }
        }
    }

    let mut idom: Vec<Option<usize>> = vec![None; n_blocks];
    for &block in &order {
        if block == entry_index {
            continue;
        }
        let mut best: Option<usize> = None;
        for &candidate in &dom[block] {
            if candidate == block {
                continue;
            }
            let better = best.is_none_or(|current| {
                dom[candidate].len() > dom[current].len()
                    || (dom[candidate].len() == dom[current].len() && candidate < current)
            });
            if better {
                best = Some(candidate);
            }
        }
        idom[block] = best;
    }

    let mut frontier: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n_blocks];
    for &block in &order {
        let Some(idom_block) = idom[block] else {
            continue;
        };
        for &pred in &predecessors[block] {
            if !reachable.contains(&pred) {
                continue;
            }
            let mut runner = pred;
            while runner != idom_block {
                frontier[runner].insert(block);
                let Some(next) = idom[runner] else {
                    break;
                };
                runner = next;
            }
        }
    }

    // ── 4. 迭代支配边界 → Phi 站点 ─────────────────
    let mut phi_sites: Vec<(usize, usize)> = Vec::new();
    for var in 0..n_orig {
        let def_blocks: BTreeSet<usize> = defs[var]
            .iter()
            .filter(|def| reachable.contains(&def.block))
            .map(|def| def.block)
            .collect();
        let reachable_uses: Vec<UseSite> = uses[var]
            .iter()
            .copied()
            .filter(|use_site| reachable.contains(&use_site.block))
            .collect();

        if def_blocks.is_empty() {
            if let Some(use_site) = reachable_uses.first() {
                let use_span = blocks[use_site.block]
                    .instructions
                    .get(use_site.instr)
                    .map(Instruction::span)
                    .unwrap_or_else(Span::dummy);
                let defs_desc = defs[var]
                    .iter()
                    .map(|def| format!("block {} instr {}", blocks[def.block].label, def.instr))
                    .collect::<Vec<_>>()
                    .join("; ");
                let uses_desc = reachable_uses
                    .iter()
                    .map(|site| format!("block {} instr {}", blocks[site.block].label, site.instr))
                    .collect::<Vec<_>>()
                    .join("; ");
                let cfg_desc = blocks
                    .iter()
                    .map(|block| format!("{}->{:?}", block.label, block.successors))
                    .collect::<Vec<_>>()
                    .join(", ");
                let instrs_desc = blocks[use_site.block]
                    .instructions
                    .iter()
                    .enumerate()
                    .map(|(index, instruction)| format!("{index}: {instruction:?}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(SsaConstructionError::at(
                    use_span,
                    format!(
                        "ValueId({var}) 在块 {} 被使用，但可达块内没有定义（SSA 构造拒绝未定义使用）；定义位 [{defs_desc}]；可达使用 [{uses_desc}]；CFG [{cfg_desc}]；块内指令：\n{instrs_desc}",
                        blocks[use_site.block].label
                    ),
                ));
            }
            continue;
        }
        // 同一块内多次定义不需要 Phi；没有可达使用也不需要 Phi（只做定义拆分）。
        if def_blocks.len() <= 1 || reachable_uses.is_empty() {
            continue;
        }

        let mut work: Vec<usize> = def_blocks.iter().copied().collect();
        let mut phi_blocks: BTreeSet<usize> = BTreeSet::new();
        while let Some(block) = work.pop() {
            for &target in &frontier[block] {
                if target == entry_index {
                    return Err(SsaConstructionError::at(
                        def_span(&blocks, defs[var][0]),
                        "入口块需要 Phi，但 7b.3 方案 A 的 translator 禁止入口块 Phi",
                    ));
                }
                if phi_blocks.insert(target) {
                    work.push(target);
                }
            }
        }
        for block in phi_blocks {
            phi_sites.push((block, var));
        }
    }
    phi_sites.sort_by_key(|(block, var)| (blocks[*block].label, *var));

    // ── 5. 值版本：原值保留给首个定义，其余定义/Phi 分配新 ValueId ──
    let mut versions_by_var: Vec<Vec<ValueId>> =
        (0..n_orig).map(|var| vec![ValueId(var as u32)]).collect();
    let mut def_version: HashMap<(usize, usize, usize), ValueId> = HashMap::new();
    for var in 0..n_orig {
        for (def_index, def) in defs[var].iter().enumerate() {
            let version = if def_index == 0 {
                ValueId(var as u32)
            } else {
                let ty = values[var].ty.clone();
                let debug_name = values[var].debug_name.clone();
                let version = ValueId(values.len() as u32);
                values.push(ValueInfo {
                    ty,
                    debug_name,
                    defining_block: blocks[def.block].label,
                });
                versions_by_var[var].push(version);
                version
            };
            values[version.0 as usize].defining_block = blocks[def.block].label;
            def_version.insert((def.block, def.instr, var), version);
        }
    }

    let mut phi_at_block: HashMap<usize, Vec<(usize, ValueId)>> = HashMap::new();
    let mut phi_meta: Vec<PhiSite> = Vec::new();
    let mut phi_vars: HashSet<usize> = HashSet::new();
    for &(block, var) in &phi_sites {
        let ty = values[var].ty.clone();
        let debug_name = values[var].debug_name.clone();
        let dst = ValueId(values.len() as u32);
        values.push(ValueInfo {
            ty,
            debug_name,
            defining_block: blocks[block].label,
        });
        versions_by_var[var].push(dst);
        phi_at_block.entry(block).or_default().push((var, dst));
        phi_vars.insert(var);
        phi_meta.push(PhiSite {
            block,
            var,
            dst,
            span: def_span(&blocks, defs[var][0]),
        });
    }

    // ── 6. 按支配树重命名可达块内的 Value ─────────────
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n_blocks];
    for &block in &order {
        if block == entry_index {
            continue;
        }
        if let Some(parent) = idom[block] {
            children[parent].push(block);
        }
    }
    for list in &mut children {
        list.sort_unstable();
    }

    let mut current: Vec<Option<ValueId>> = vec![None; n_orig];
    let mut exit_versions: HashMap<(usize, usize), ValueId> = HashMap::new();

    enum Step {
        Enter(usize),
        Exit(Vec<(usize, Option<ValueId>)>),
    }
    let mut stack = vec![Step::Enter(entry_index)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(block) => {
                let mut changes: Vec<(usize, Option<ValueId>)> = Vec::new();
                if let Some(phis) = phi_at_block.get(&block) {
                    for &(var, dst) in phis {
                        changes.push((var, current[var]));
                        current[var] = Some(dst);
                    }
                }

                let block_label = blocks[block].label;
                for (instr_index, instruction) in blocks[block].instructions.iter_mut().enumerate()
                {
                    let span = instruction.span();
                    let def_ptrs: Vec<*const Operand> = instruction
                        .def_operands()
                        .iter()
                        .map(|operand| *operand as *const Operand)
                        .collect();

                    // 6.1 先重命名使用位（同指令内 def 之前读到的仍是旧版本）。
                    let mut failure: Option<SsaConstructionError> = None;
                    instruction.for_each_operand_mut(&mut |operand| {
                        if failure.is_some()
                            || def_ptrs
                                .iter()
                                .any(|ptr| std::ptr::eq(*ptr, operand as *const Operand))
                        {
                            return;
                        }
                        let Operand::Value(id) = operand else {
                            return;
                        };
                        let var = id.0 as usize;
                        let Some(version) = current[var] else {
                            failure = Some(SsaConstructionError::at(
                                span,
                                format!(
                                    "ValueId({var}) 在块 {block_label} 指令 {instr_index} 被使用，但没有支配该位置的定义"
                                ),
                            ));
                            return;
                        };
                        *operand = Operand::Value(version);
                    });
                    if let Some(error) = failure {
                        return Err(error);
                    }

                    // 6.2 再重命名定义位。
                    instruction.for_each_operand_mut(&mut |operand| {
                        if failure.is_some()
                            || !def_ptrs
                                .iter()
                                .any(|ptr| std::ptr::eq(*ptr, operand as *const Operand))
                        {
                            return;
                        }
                        let Operand::Value(id) = operand else {
                            return;
                        };
                        let var = id.0 as usize;
                        let Some(version) = def_version.get(&(block, instr_index, var)).copied()
                        else {
                            failure = Some(SsaConstructionError::at(
                                span,
                                format!(
                                    "ValueId({var}) 在块 {block_label} 指令 {instr_index} 的定义没有登记版本"
                                ),
                            ));
                            return;
                        };
                        changes.push((var, current[var]));
                        current[var] = Some(version);
                        *operand = Operand::Value(version);
                    });
                    if let Some(error) = failure {
                        return Err(error);
                    }
                }

                for &var in &phi_vars {
                    if let Some(version) = current[var] {
                        exit_versions.insert((block, var), version);
                    }
                }

                stack.push(Step::Exit(changes));
                for &child in children[block].iter().rev() {
                    stack.push(Step::Enter(child));
                }
            }
            Step::Exit(changes) => {
                for (var, previous) in changes.into_iter().rev() {
                    current[var] = previous;
                }
            }
        }
    }

    // ── 6.3 不可达块：不做支配 / Phi，但定义位仍必须换成唯一版本 ──
    // 全函数（含死代码）的单定义是 7f.2 verify 的判据；不可达块没有支配
    // 关系，故这里只做版本替换，不报“使用前无定义”（死代码没有运行时语义）。
    let mut unreachable_current: Vec<Option<ValueId>> = vec![None; n_orig];
    for (block, block_data) in blocks.iter_mut().enumerate() {
        if reachable.contains(&block) {
            continue;
        }
        for (instr_index, instruction) in block_data.instructions.iter_mut().enumerate() {
            let def_ptrs: Vec<*const Operand> = instruction
                .def_operands()
                .iter()
                .map(|operand| *operand as *const Operand)
                .collect();
            instruction.for_each_operand_mut(&mut |operand| {
                let is_def = def_ptrs
                    .iter()
                    .any(|ptr| std::ptr::eq(*ptr, operand as *const Operand));
                let Operand::Value(id) = operand else {
                    return;
                };
                let var = id.0 as usize;
                if is_def {
                    let Some(version) = def_version.get(&(block, instr_index, var)).copied() else {
                        return;
                    };
                    unreachable_current[var] = Some(version);
                    *operand = Operand::Value(version);
                } else if let Some(version) = unreachable_current[var] {
                    *operand = Operand::Value(version);
                }
            });
        }
    }

    // ── 7. 填 Phi incoming（全部前驱、按 label 升序）并定类型 ──
    let mut phi_incoming: HashMap<(usize, usize), Vec<(usize, Operand)>> = HashMap::new();
    let mut undef_cache: HashMap<usize, ValueId> = HashMap::new();
    for phi in &phi_meta {
        let mut predecessors_of_block = predecessors[phi.block].clone();
        predecessors_of_block.sort_by_key(|pred| blocks[*pred].label);

        let mut reachable_entries: Vec<(usize, ValueId)> = Vec::new();
        for &pred in &predecessors_of_block {
            if !reachable.contains(&pred) {
                continue;
            }
            let source = match exit_versions.get(&(pred, phi.var)).copied() {
                Some(source) => source,
                // 该前驱路径上未定义：旧路径读帧槽默认 Void，这里在入口
                // 显式补一个 Load Void 值（类型记显式未知），行为等价。
                None => materialize_undef(
                    &mut blocks,
                    entry_index,
                    &mut values,
                    &mut versions_by_var,
                    phi.var,
                    phi.span,
                    &mut undef_cache,
                    &mut head_insertions,
                ),
            };
            reachable_entries.push((blocks[pred].label, source));
        }
        let Some(first_reachable_source) = reachable_entries.first().map(|(_, source)| *source)
        else {
            return Err(SsaConstructionError::at(
                phi.span,
                format!("Phi（块 {}）没有任何可达前驱", blocks[phi.block].label),
            ));
        };

        let mut entries: Vec<(usize, Operand)> = reachable_entries
            .iter()
            .map(|(label, source)| (*label, Operand::Value(*source)))
            .collect();
        for &pred in &predecessors_of_block {
            if reachable.contains(&pred) {
                continue;
            }
            let unreachable_source = defs[phi.var]
                .iter()
                .find(|def| def.block == pred)
                .and_then(|def| def_version.get(&(def.block, def.instr, phi.var)).copied())
                .unwrap_or(first_reachable_source);
            entries.push((blocks[pred].label, Operand::Value(unreachable_source)));
        }
        entries.sort_by_key(|(label, _)| *label);

        let ty = resolve_phi_type(&values, phi.var, phi.span, &entries)?;
        values[phi.dst.0 as usize].ty = ty;
        phi_incoming.insert((phi.block, phi.var), entries);
    }

    // ── 8. 把 Phi 插到块首（同一块内按 var 升序；同块 Phi 前驱集一致）──
    let mut phi_blocks: Vec<usize> = phi_at_block.keys().copied().collect();
    phi_blocks.sort_by_key(|block| blocks[*block].label);
    for block in phi_blocks {
        let phis = phi_at_block.remove(&block).unwrap_or_default();
        head_insertions[block] += phis.len();
        for (position, (var, dst)) in phis.into_iter().enumerate() {
            let Some(incoming) = phi_incoming.remove(&(block, var)) else {
                return Err(SsaConstructionError::at(
                    Span::dummy(),
                    format!(
                        "Phi（块 {}，ValueId({var})）缺少 incoming",
                        blocks[block].label
                    ),
                ));
            };
            let span = phi_meta
                .iter()
                .find(|phi| phi.block == block && phi.var == var)
                .map(|phi| phi.span)
                .unwrap_or_else(Span::dummy);
            blocks[block].instructions.insert(
                position,
                Instruction::Phi {
                    dst: Operand::Value(dst),
                    incoming,
                    span,
                },
            );
        }
    }

    adjust_jump_targets(&mut blocks, &old_block_starts, old_total, &head_insertions);

    // ── 9. 值钉子跟随原值的全部版本 ─────────────────
    let mut out_pins: Vec<(ValueId, usize)> = Vec::new();
    let mut seen_pins: HashSet<(ValueId, usize)> = HashSet::new();
    for &(old, slot) in pins {
        let var = old.0 as usize;
        if var >= n_orig {
            return Err(SsaConstructionError::at(
                Span::dummy(),
                format!("值钉子指向越界 ValueId({})（值表长 {n_orig}）", old.0),
            ));
        }
        for &version in &versions_by_var[var] {
            if seen_pins.insert((version, slot)) {
                out_pins.push((version, slot));
            }
        }
    }

    Ok(SsaOutput {
        blocks,
        values,
        pins: out_pins,
    })
}

/// Phi 类型：优先原值表类型；原值未知时要求全部已知输入同型；仍不可得则显式报错。
pub(crate) fn resolve_phi_type(
    values: &[ValueInfo],
    var: usize,
    span: Span,
    entries: &[(usize, Operand)],
) -> Result<MonoType, SsaConstructionError> {
    let original = &values[var].ty;
    let mut incoming_type: Option<MonoType> = None;
    for (_, operand) in entries {
        let Operand::Value(id) = operand else {
            continue;
        };
        let ty = &values[id.0 as usize].ty;
        if is_unknown_type(ty) {
            continue;
        }
        match &incoming_type {
            None => incoming_type = Some(ty.clone()),
            Some(previous) if previous == ty => {}
            Some(previous) => {
                return Err(SsaConstructionError::at(
                    span,
                    format!("Phi（ValueId({var})）的输入类型不一致：{previous:?} vs {ty:?}"),
                ));
            }
        }
    }

    if !is_unknown_type(original) {
        if let Some(incoming) = &incoming_type {
            if incoming != original {
                return Err(SsaConstructionError::at(
                    span,
                    format!(
                        "Phi（ValueId({var})）输入类型 {incoming:?} 与值表类型 {original:?} 不一致"
                    ),
                ));
            }
        }
        return Ok(original.clone());
    }

    // 原值表为显式未知：已知输入同型则收敛到该类型；否则沿用仓库既有的
    // 显式未知标记（`TypeRef("?")`），不发明具体类型、不静默用 Int(64) 占位。
    Ok(incoming_type.unwrap_or_else(|| MonoType::TypeRef("?".to_string())))
}
