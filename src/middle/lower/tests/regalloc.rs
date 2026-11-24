//! 线性扫描分配器单元测试 — 基于任务 7b.4 的提交信息
//!
//! 判据（证明门）：分配器的**性质**是不变量而非当前输出——
//! 1. 活跃区间不重叠的值必须能共享槽（复用是删掉三处回收的前提；换一个正确的
//!    替代实现（如更精细的图着色）也必须满足本条）；
//! 2. 同时活跃的值**绝不能**共享槽（槽语义的正确性底线）；
//! 3. `Phi` 的入边是**前驱出口**的使用，故源值必须活到前驱块末（7b.3 的
//!    translator 展开依赖同一语义）；
//! 4. 值钉子固定落具名槽；5. 预算闸按 `frame_slots` 判。

use crate::middle::ir::ir::{BasicBlock, ConstValue, Instruction, Operand, ValueId, ValueInfo};
use crate::middle::lower::regalloc::{allocate_value_slots, Allocation};
use crate::util::span::Span;

// ── 辅助函数（规则 5.3：集中在文件顶部）──────────────

fn value(id: u32) -> Operand {
    Operand::Value(ValueId(id))
}

fn load_value(
    id: u32,
    n: i128,
) -> Instruction {
    Instruction::Load {
        dst: value(id),
        src: Operand::Const(ConstValue::Int(n)),
        span: Span::dummy(),
    }
}

fn add_values(
    dst: u32,
    lhs: u32,
    rhs: u32,
) -> Instruction {
    Instruction::Add {
        dst: value(dst),
        lhs: value(lhs),
        rhs: value(rhs),
        span: Span::dummy(),
    }
}

fn ret_value(id: u32) -> Instruction {
    Instruction::Ret {
        value: Some(value(id)),
        span: Span::dummy(),
    }
}

fn ret_void() -> Instruction {
    Instruction::Ret {
        value: None,
        span: Span::dummy(),
    }
}

fn jmp(target: usize) -> Instruction {
    Instruction::Jmp {
        target,
        span: Span::dummy(),
    }
}

fn block(
    label: usize,
    instructions: Vec<Instruction>,
    successors: Vec<usize>,
) -> BasicBlock {
    BasicBlock {
        label,
        instructions,
        successors,
    }
}

fn values(n: usize) -> Vec<ValueInfo> {
    (0..n)
        .map(|_| ValueInfo {
            ty: crate::frontend::core::sema::MonoType::Int(64),
            debug_name: None,
            defining_block: 0,
        })
        .collect()
}

fn allocate(
    blocks: &[BasicBlock],
    value_count: usize,
    named_base: usize,
    pinned: &[(ValueId, usize)],
    limit: usize,
) -> Result<Allocation, crate::middle::lower::regalloc::Budget> {
    allocate_value_slots(blocks, &values(value_count), named_base, pinned, limit)
}

// ── 判据 1：互不重叠的值共享槽 ──────────────

/// 两段先后使用、区间不相交的临时值必须落同一槽：`frame_slots` 只按
/// 「具名基址 + 同时活跃数」计，而不是按值总数计（复用承重）。
#[test]
fn test_allocator_reuses_slot_for_disjoint_intervals() {
    // Arrange：v0/v1 在指令 0-1 内用完，v2/v3 在 2-4 内用完
    let blocks = vec![block(
        0,
        vec![
            load_value(0, 1),
            add_values(1, 0, 0),
            load_value(2, 2),
            add_values(3, 2, 2),
            ret_value(3),
        ],
        Vec::new(),
    )];

    // Act
    let allocation = allocate(&blocks, 4, 2, &[], 255).expect("预算充足");

    // Assert
    assert_eq!(
        allocation.frame_slots, 4,
        "复用后帧槽 = 具名基址 2 + 同时活跃 2，实际 {}",
        allocation.frame_slots
    );
    assert_eq!(
        allocation.value_slots[2], allocation.value_slots[0],
        "区间不相交的 v2 应复用 v0 的槽：{:?}",
        allocation.value_slots
    );
    assert_eq!(
        allocation.value_slots[3], allocation.value_slots[1],
        "区间不相交的 v3 应复用 v1 的槽：{:?}",
        allocation.value_slots
    );
}

// ── 判据 2：同时活跃的值不得共享槽 ──────────────

/// 活过另一值全段的值必须占据不同槽——槽语义的正确性底线。
#[test]
fn test_allocator_keeps_overlapping_values_in_distinct_slots() {
    // Arrange：v0 在 0 定义、3 使用（跨越 v1/v2 的整个生命周期）
    let blocks = vec![block(
        0,
        vec![
            load_value(0, 1),
            load_value(1, 2),
            add_values(2, 1, 1),
            add_values(3, 0, 2),
            ret_value(3),
        ],
        Vec::new(),
    )];

    // Act
    let allocation = allocate(&blocks, 4, 2, &[], 255).expect("预算充足");

    // Assert
    assert_ne!(
        allocation.value_slots[0], allocation.value_slots[1],
        "长活跃的 v0 不得与 v1 共享槽：{:?}",
        allocation.value_slots
    );
    assert_ne!(
        allocation.value_slots[0], allocation.value_slots[2],
        "长活跃的 v0 不得与 v2 共享槽：{:?}",
        allocation.value_slots
    );
}

// ── 判据 3：Phi 入边是前驱出口的使用 ──────────────

/// `Phi` 的 `incoming` 源只在汇合块被引用，但它必须活到**前驱块末**：
/// 否则 translator 在前驱出口展开的 Move 组会读到已被复用掉的槽。
#[test]
fn test_allocator_keeps_phi_edge_source_live_to_predecessor_end() {
    // Arrange：块 0 定义 v0（只经 Phi 入边被用），块 1 是汇合块
    let blocks = vec![
        block(
            0,
            vec![
                load_value(0, 7),
                load_value(2, 8),
                add_values(3, 2, 2),
                jmp(4),
            ],
            vec![1],
        ),
        block(
            1,
            vec![
                Instruction::Phi {
                    dst: value(1),
                    incoming: vec![(0, value(0))],
                    span: Span::dummy(),
                },
                ret_value(1),
            ],
            Vec::new(),
        ),
    ];

    // Act
    let allocation = allocate(&blocks, 4, 2, &[], 255).expect("预算充足");

    // Assert
    assert_ne!(
        allocation.value_slots[0], allocation.value_slots[2],
        "Phi 入边源 v0 必须活到块 0 末（与 v2 不重叠），实际 {:?}",
        allocation.value_slots
    );
}

// ── 判据 4：值钉子固定落具名槽 ──────────────

/// pinned 值固定落其具名槽（`slot_for_operand` 耦合改成的钉子），
/// 且不占自由槽池：`frame_slots` 仍等于具名基址（无其他值）。
#[test]
fn test_allocator_pins_value_to_named_slot() {
    // Arrange：v0 钉在具名槽 1
    let blocks = vec![block(0, vec![load_value(0, 3), ret_value(0)], Vec::new())];

    // Act
    let allocation = allocate(&blocks, 1, 2, &[(ValueId(0), 1)], 255).expect("预算充足");

    // Assert
    assert_eq!(
        allocation.value_slots[0], 1,
        "pinned 值必须落具名槽 1，实际 {}",
        allocation.value_slots[0]
    );
    assert_eq!(
        allocation.frame_slots, 2,
        "pinned 值不占自由槽池，帧槽应仍为具名基址 2，实际 {}",
        allocation.frame_slots
    );
}

// ── 判据 5：预算闸按 frame_slots 判 ──────────────

/// 帧槽超过上限即返 `Budget`（调用方转 E3014）；未超则成功——判据是
/// **分配后帧槽**，不是值总数（值总数 300 但活跃 2 的形态必须成功）。
#[test]
fn test_allocator_budget_is_judged_by_frame_slots_not_value_count() {
    // Arrange：300 个值、但任意时刻只有 2 个同时活跃
    let mut instructions = Vec::new();
    for id in 0..150u32 {
        instructions.push(load_value(id * 2, i128::from(id)));
        instructions.push(add_values(id * 2 + 1, id * 2, id * 2));
    }
    instructions.push(ret_void());
    let blocks = vec![block(0, instructions, Vec::new())];

    // Act：值 300 个、具名基址 2 ⇒ 帧槽 4，远小于上限
    let allocation = allocate(&blocks, 300, 2, &[], 255).expect("活跃数小，必须装得下");
    // Act：具名基址本身越界 ⇒ 必须报预算
    let over = allocate(&blocks, 300, 254, &[], 255);

    // Assert
    assert!(
        allocation.frame_slots <= 255,
        "值总数 300 但活跃数小，帧槽必须 ≤255，实际 {}",
        allocation.frame_slots
    );
    assert!(
        over.is_err(),
        "具名基址 254 + 两个同时活跃值 ⇒ 帧槽 256 必须触发预算"
    );
    assert_eq!(
        over.err().map(|budget| (budget.required, budget.limit)),
        Some((256, 255)),
        "预算报告必须给 required/limit"
    );
}
