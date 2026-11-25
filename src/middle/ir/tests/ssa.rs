//! 7f.1 支配关系与 SSA 构造判据（WBS P7 / 04-ssa.md §SSA 形态定义）。
//!
//! 覆盖：单块多定义、then/else 汇合、循环回边、嵌套汇合、不可达块、无值表跳过、
//! Phi incoming 前驱顺序、显式未知类型传播、已知输入冲突报错、不可达使用跳过、
//! 可达且 0 定义值的 Load Void 物化，以及真实 lowering 产物上的
//! “每个可达 Operand::Value 单定义且使用被定义支配”断言。

use crate::frontend::compiler::Compiler;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{
    partition_blocks, BasicBlock, ConstValue, FunctionBody, Instruction, Operand, ValueId,
    ValueInfo,
};
use crate::middle::ir::ssa::{construct_ssa, resolve_phi_type};
use crate::util::span::Span;

fn int_ty() -> MonoType {
    MonoType::Int(64)
}

fn bool_ty() -> MonoType {
    MonoType::Bool
}

fn value_info(ty: MonoType) -> ValueInfo {
    ValueInfo {
        ty,
        debug_name: None,
        defining_block: 0,
    }
}

fn load(
    dst: Operand,
    value: ConstValue,
) -> Instruction {
    Instruction::Load {
        dst,
        src: Operand::Const(value),
        span: Span::dummy(),
    }
}

fn add(
    dst: Operand,
    lhs: Operand,
    rhs: Operand,
) -> Instruction {
    Instruction::Add {
        dst,
        lhs,
        rhs,
        span: Span::dummy(),
    }
}

fn jmp(target: usize) -> Instruction {
    Instruction::Jmp {
        target,
        span: Span::dummy(),
    }
}

fn jmp_if_not(
    cond: Operand,
    target: usize,
) -> Instruction {
    Instruction::JmpIfNot {
        cond,
        target,
        span: Span::dummy(),
    }
}

fn ret(value: Operand) -> Instruction {
    Instruction::Ret {
        value: Some(value),
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

fn label_to_index(blocks: &[BasicBlock]) -> std::collections::HashMap<usize, usize> {
    blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.label, index))
        .collect()
}

/// 可达块上的支配集（与 verify 的指令级支配口径独立：本助手只判块级支配）。
fn dominators(
    blocks: &[BasicBlock],
    entry: usize,
) -> (
    std::collections::HashSet<usize>,
    Vec<std::collections::HashSet<usize>>,
) {
    let labels = label_to_index(blocks);
    let entry_index = labels[&entry];
    let mut reachable = std::collections::HashSet::new();
    let mut work = vec![entry_index];
    while let Some(index) = work.pop() {
        if !reachable.insert(index) {
            continue;
        }
        for successor in &blocks[index].successors {
            work.push(labels[successor]);
        }
    }

    let all: std::collections::HashSet<usize> = reachable.clone();
    let mut dom: Vec<std::collections::HashSet<usize>> =
        blocks.iter().map(|_| all.clone()).collect();
    dom[entry_index] = std::collections::HashSet::from([entry_index]);
    let mut changed = true;
    while changed {
        changed = false;
        for &index in &reachable {
            if index == entry_index {
                continue;
            }
            let mut preds = blocks
                .iter()
                .enumerate()
                .filter(|(_, block)| block.successors.contains(&blocks[index].label))
                .map(|(pred, _)| pred)
                .filter(|pred| reachable.contains(pred));
            let Some(first) = preds.next() else {
                continue;
            };
            let mut next = dom[first].clone();
            for pred in preds {
                next = next.intersection(&dom[pred]).copied().collect();
            }
            next.insert(index);
            if next != dom[index] {
                dom[index] = next;
                changed = true;
            }
        }
    }
    (reachable, dom)
}

/// 判据：每个可达 Operand::Value 单定义；可达使用被定义支配；Phi 覆盖全部前驱且
/// incoming 按前驱 label 升序；defining_block 与真实定义块一致。
fn assert_ssa(
    blocks: &[BasicBlock],
    values: &[ValueInfo],
    entry: usize,
) {
    let labels = label_to_index(blocks);
    let mut defs: Vec<Option<(usize, usize)>> = vec![None; values.len()];
    for (block_index, block) in blocks.iter().enumerate() {
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            for operand in instruction.def_operands() {
                let Operand::Value(id) = operand else {
                    continue;
                };
                assert!(
                    (id.0 as usize) < values.len(),
                    "定义 ValueId({}) 越界（值表长 {}）",
                    id.0,
                    values.len()
                );
                assert!(
                    defs[id.0 as usize].is_none(),
                    "ValueId({}) 被定义了不止一次（第二次在块 {} 指令 {}）",
                    id.0,
                    block.label,
                    instruction_index
                );
                defs[id.0 as usize] = Some((block_index, instruction_index));
            }
            for operand in instruction.use_operands() {
                let Operand::Value(id) = operand else {
                    continue;
                };
                assert!(
                    (id.0 as usize) < values.len(),
                    "使用 ValueId({}) 越界（值表长 {}）",
                    id.0,
                    values.len()
                );
            }
        }
    }

    for (id, def) in defs.iter().enumerate() {
        if let Some((block_index, _)) = def {
            assert_eq!(
                values[id].defining_block, blocks[*block_index].label,
                "ValueId({id}) 的 defining_block 必须是真实定义块 {}，实际 {}",
                blocks[*block_index].label, values[id].defining_block
            );
        }
    }

    let (reachable, dom) = dominators(blocks, entry);
    for &block_index in &reachable {
        let block = &blocks[block_index];
        let mut predecessors: Vec<usize> = blocks
            .iter()
            .filter(|candidate| candidate.successors.contains(&block.label))
            .map(|candidate| candidate.label)
            .collect();
        predecessors.sort_unstable();

        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            let is_phi = matches!(instruction, Instruction::Phi { .. });
            if !is_phi {
                for operand in instruction.use_operands() {
                    let Operand::Value(id) = operand else {
                        continue;
                    };
                    let (def_block, def_instruction) = defs[id.0 as usize]
                        .expect("可达使用必须有定义（SSA 构造必须拒绝未定义使用）");
                    let same_block_after =
                        def_block == block_index && def_instruction < instruction_index;
                    assert!(
                        dom[block_index].contains(&def_block) || same_block_after,
                        "块 {} 指令 {} 使用的 ValueId({}) 未被定义块 {} 支配",
                        block.label,
                        instruction_index,
                        id.0,
                        blocks[def_block].label
                    );
                }
            }

            if let Instruction::Phi { incoming, .. } = instruction {
                let mut labels_in: Vec<usize> = incoming.iter().map(|(label, _)| *label).collect();
                assert_eq!(
                    labels_in, predecessors,
                    "块 {} 的 Phi incoming 必须覆盖全部前驱且升序：期望 {predecessors:?}，实际 {labels_in:?}",
                    block.label
                );
                labels_in.sort_unstable();
                assert_eq!(
                    labels_in, predecessors,
                    "块 {} 的 Phi incoming 排序后仍须等于前驱集",
                    block.label
                );

                // Phi 的 incoming 在**前驱出口**使用：支配关系应在该前驱上判，
                // 不是在汇合块上判。不可达前驱不参与支配判定（与 verify 同口径）。
                for (pred_label, operand) in incoming {
                    let Operand::Value(id) = operand else {
                        continue;
                    };
                    let Some(&pred_index) = labels.get(pred_label) else {
                        panic!("Phi 前驱 label {pred_label} 不存在");
                    };
                    if !reachable.contains(&pred_index) {
                        continue;
                    }
                    let (def_block, def_instruction) =
                        defs[id.0 as usize].expect("Phi incoming 的源必须有定义");
                    let same_block_before_end = def_block == pred_index
                        && def_instruction < blocks[pred_index].instructions.len();
                    assert!(
                        dom[pred_index].contains(&def_block) || same_block_before_end,
                        "Phi 前驱 {} 出口使用的 ValueId({}) 未被定义块 {} 支配",
                        blocks[pred_index].label,
                        id.0,
                        blocks[def_block].label
                    );
                }
            }
        }
    }
}

// ── 单块多定义 ─────────────────────────────

#[test]
fn test_ssa_single_block_multiple_definitions_renames_uses() {
    // Arrange
    let instructions = vec![
        load(Operand::Value(ValueId(0)), ConstValue::Int(1)),
        add(
            Operand::Value(ValueId(0)),
            Operand::Value(ValueId(0)),
            Operand::Const(ConstValue::Int(2)),
        ),
        ret(Operand::Value(ValueId(0))),
    ];
    let blocks = partition_blocks(instructions);
    let values = vec![value_info(int_ty())];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("单块多定义必须能构造 SSA");

    // Assert
    assert_ssa(&output.blocks, &output.values, 0);
    let ret_value = match output.blocks[0].instructions.last() {
        Some(Instruction::Ret {
            value: Some(value), ..
        }) => value.clone(),
        other => panic!("单块必须以 Ret 结尾，实际 {other:?}"),
    };
    assert!(
        !matches!(ret_value, Operand::Value(ValueId(0))),
        "第二次定义之后的使用必须重命名到新版本，实际仍是原 ValueId(0)"
    );
}

// ── then/else 汇合与 incoming 顺序 ─────────────

#[test]
fn test_ssa_then_else_merge_inserts_phi() {
    // Arrange：0 Load v0=true / 1 JmpIfNot v0 -> 4 / 2 Load v1=1 / 3 Jmp -> 5
    //          4 Load v1=2 / 5 Ret v1；块 3 是汇合块，前驱 1、2
    let instructions = vec![
        load(Operand::Value(ValueId(0)), ConstValue::Bool(true)),
        jmp_if_not(Operand::Value(ValueId(0)), 4),
        load(Operand::Value(ValueId(1)), ConstValue::Int(1)),
        jmp(5),
        load(Operand::Value(ValueId(1)), ConstValue::Int(2)),
        ret(Operand::Value(ValueId(1))),
    ];
    let blocks = partition_blocks(instructions);
    let values = vec![value_info(bool_ty()), value_info(int_ty())];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("then/else 汇合必须能构造 SSA");

    // Assert
    assert_ssa(&output.blocks, &output.values, 0);
    let merge = output
        .blocks
        .iter()
        .find(|block| block.label == 3)
        .expect("then/else 汇合块 label 必须是 3");
    assert!(
        matches!(merge.instructions.first(), Some(Instruction::Phi { .. })),
        "汇合块首必须是 Phi，实际 {:?}",
        merge.instructions.first()
    );
    let phi_type = output
        .values
        .iter()
        .find(|info| info.defining_block == 3)
        .expect("Phi 必须登记 ValueInfo 并记真实定义块")
        .ty
        .clone();
    assert_eq!(phi_type, int_ty(), "Phi 类型必须从两个 Int 输入推断");

    // Phi 插到块首会移动展平下标；跳到汇合块/入口的 Jmp 目标必须同步平移。
    let merge_position = output
        .blocks
        .iter()
        .position(|block| block.label == 3)
        .expect("汇合块必须存在");
    let merge_flat_start: usize = output.blocks[..merge_position]
        .iter()
        .map(|block| block.instructions.len())
        .sum();
    let then_jmp_target = output.blocks[1]
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            Instruction::Jmp { target, .. } => Some(*target),
            _ => None,
        })
        .expect("then 分支必须以 Jmp 到汇合块");
    assert_eq!(
        then_jmp_target, merge_flat_start,
        "Phi 插入后 Jmp 目标必须指向汇合块的新展平起点"
    );
}

#[test]
fn test_ssa_phi_incoming_sorted_by_predecessor_label() {
    // Arrange：与 then/else 同形，汇合块前驱 label 为 1、2（必须升序输出）
    let instructions = vec![
        load(Operand::Value(ValueId(0)), ConstValue::Bool(true)),
        jmp_if_not(Operand::Value(ValueId(0)), 4),
        load(Operand::Value(ValueId(1)), ConstValue::Int(1)),
        jmp(5),
        load(Operand::Value(ValueId(1)), ConstValue::Int(2)),
        ret(Operand::Value(ValueId(1))),
    ];
    let blocks = partition_blocks(instructions);
    let values = vec![value_info(bool_ty()), value_info(int_ty())];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("汇合必须能构造 SSA");

    // Assert
    let phi = output.blocks[3]
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            Instruction::Phi { incoming, .. } => Some(incoming),
            _ => None,
        })
        .expect("汇合块必须含 Phi");
    let labels: Vec<usize> = phi.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels, vec![1, 2], "Phi incoming 必须按前驱 label 升序");
}

// ── 循环回边 ───────────────────────────────

#[test]
fn test_ssa_loop_backedge_inserts_phi() {
    // Arrange：0 Load v0=0 / 1 Jmp -> 2 / 2 Lt v1 v0,3 / 3 JmpIfNot v1 -> 6
    //          4 Add v0 v0,1 / 5 Jmp -> 2 / 6 Ret v0
    // 头块 1（label 1）有前驱 0 与 2（回边），v0 需要 Phi。
    let instructions = vec![
        load(Operand::Value(ValueId(0)), ConstValue::Int(0)),
        jmp(2),
        Instruction::Lt {
            dst: Operand::Value(ValueId(1)),
            lhs: Operand::Value(ValueId(0)),
            rhs: Operand::Const(ConstValue::Int(3)),
            span: Span::dummy(),
        },
        jmp_if_not(Operand::Value(ValueId(1)), 6),
        add(
            Operand::Value(ValueId(0)),
            Operand::Value(ValueId(0)),
            Operand::Const(ConstValue::Int(1)),
        ),
        jmp(2),
        ret(Operand::Value(ValueId(0))),
    ];
    let blocks = partition_blocks(instructions);
    let values = vec![value_info(int_ty()), value_info(bool_ty())];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("循环回边必须能构造 SSA");

    // Assert
    assert_ssa(&output.blocks, &output.values, 0);
    let header = output
        .blocks
        .iter()
        .find(|block| block.label == 1)
        .expect("循环头 label 必须是 1");
    let phi = header
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            Instruction::Phi { incoming, .. } => Some(incoming),
            _ => None,
        })
        .expect("循环头必须插入 Phi");
    let labels: Vec<usize> = phi.iter().map(|(label, _)| *label).collect();
    assert_eq!(labels, vec![0, 2], "循环头 Phi 必须覆盖前驱 0 与回边 2");
}

// ── 嵌套汇合 ───────────────────────────────

#[test]
fn test_ssa_nested_merge_inserts_phis() {
    // Arrange：内层汇合块 5（前驱 3、4）与外层汇合块 6（前驱 2、5）都需要 Phi。
    let blocks = vec![
        block(
            0,
            vec![
                load(Operand::Value(ValueId(2)), ConstValue::Bool(true)),
                jmp_if_not(Operand::Value(ValueId(2)), 0),
            ],
            vec![1, 2],
        ),
        block(
            1,
            vec![
                load(Operand::Value(ValueId(3)), ConstValue::Bool(true)),
                jmp_if_not(Operand::Value(ValueId(3)), 0),
            ],
            vec![3, 4],
        ),
        block(
            2,
            vec![load(Operand::Value(ValueId(0)), ConstValue::Int(10))],
            vec![6],
        ),
        block(
            3,
            vec![load(Operand::Value(ValueId(0)), ConstValue::Int(20))],
            vec![5],
        ),
        block(
            4,
            vec![load(Operand::Value(ValueId(0)), ConstValue::Int(30))],
            vec![5],
        ),
        block(
            5,
            vec![add(
                Operand::Value(ValueId(1)),
                Operand::Value(ValueId(0)),
                Operand::Const(ConstValue::Int(1)),
            )],
            vec![6],
        ),
        block(6, vec![ret(Operand::Value(ValueId(0)))], vec![]),
    ];
    let values = vec![
        value_info(int_ty()),
        value_info(int_ty()),
        value_info(bool_ty()),
        value_info(bool_ty()),
    ];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("嵌套汇合必须能构造 SSA");

    // Assert
    assert_ssa(&output.blocks, &output.values, 0);
    for label in [5, 6] {
        let merge = output
            .blocks
            .iter()
            .find(|block| block.label == label)
            .expect("嵌套汇合块必须存在");
        assert!(
            matches!(merge.instructions.first(), Some(Instruction::Phi { .. })),
            "块 {label} 首必须是 Phi，实际 {:?}",
            merge.instructions.first()
        );
    }
}

// ── 不可达块 / 无值表 ───────────────────────

#[test]
fn test_ssa_unreachable_block_is_ignored() {
    // Arrange：块 1 不可达（入口 0 直接 Ret），块 1 内有一个值定义与使用。
    let blocks = vec![
        block(
            0,
            vec![Instruction::Ret {
                value: None,
                span: Span::dummy(),
            }],
            vec![],
        ),
        block(
            1,
            vec![
                load(Operand::Value(ValueId(0)), ConstValue::Int(7)),
                ret(Operand::Value(ValueId(0))),
            ],
            vec![],
        ),
    ];
    let values = vec![value_info(int_ty())];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("不可达块不得让构造失败");

    // Assert
    assert_ssa(&output.blocks, &output.values, 0);
    assert_eq!(
        output.blocks[1].instructions.len(),
        2,
        "不可达块不得被插入/删除指令"
    );
    assert!(
        !output.blocks[1]
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Phi { .. })),
        "不可达块不得插入 Phi"
    );
}

#[test]
fn test_ssa_skips_function_without_values() {
    // Arrange
    let blocks = vec![block(
        0,
        vec![Instruction::Ret {
            value: None,
            span: Span::dummy(),
        }],
        vec![],
    )];

    // Act
    let output = construct_ssa(blocks, Vec::new(), 0, &[]).expect("无值表路径必须跳过构造");

    // Assert
    assert_eq!(output.values.len(), 0, "无值表路径不得新增值");
    assert_eq!(output.blocks[0].instructions.len(), 1, "无值表路径不得改块");
}

// ── 显式未知 / 已知冲突 / 未定义物化 ─────────

#[test]
fn test_ssa_phi_unknown_type_is_propagated_explicitly() {
    // Arrange：汇合值的定义来自 Local（lower 不写 LocalSlot.ty），两个版本都未知。
    let instructions = vec![
        load(Operand::Value(ValueId(1)), ConstValue::Bool(true)),
        jmp_if_not(Operand::Value(ValueId(1)), 4),
        Instruction::Move {
            dst: Operand::Value(ValueId(0)),
            src: Operand::Local(0),
            span: Span::dummy(),
        },
        jmp(5),
        Instruction::Move {
            dst: Operand::Value(ValueId(0)),
            src: Operand::Local(1),
            span: Span::dummy(),
        },
        ret(Operand::Value(ValueId(0))),
    ];
    let blocks = partition_blocks(instructions);
    let values = vec![
        value_info(MonoType::TypeRef("?".to_string())),
        value_info(bool_ty()),
    ];

    // Act
    let output = construct_ssa(blocks, values, 0, &[])
        .expect("显式未知是既有契约，不得因类型未知而拒绝生成");

    // Assert
    let phi = output
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .find_map(|instruction| match instruction {
            Instruction::Phi {
                dst: Operand::Value(id),
                ..
            } => Some(*id),
            _ => None,
        })
        .expect("then/else 汇合必须插入 Phi");
    assert_eq!(
        output.values[phi.0 as usize].ty,
        MonoType::TypeRef("?".to_string()),
        "原值与全部输入均未知时，Phi 必须保留显式未知，不得发明具体类型"
    );
}

#[test]
fn test_ssa_phi_conflicting_known_inputs_reports_error() {
    // Arrange：原值未知，但两个输入已带互斥的已知类型。
    let values = vec![
        value_info(MonoType::TypeRef("?".to_string())),
        value_info(int_ty()),
        value_info(bool_ty()),
    ];
    let entries = vec![
        (0, Operand::Value(ValueId(1))),
        (1, Operand::Value(ValueId(2))),
    ];

    // Act
    let error = resolve_phi_type(&values, 0, Span::dummy(), &entries)
        .expect_err("已知输入类型冲突必须显式报错");

    // Assert
    assert!(
        error.message.contains("不一致"),
        "冲突错误消息必须点名类型不一致，实际 {}",
        error.message
    );
}

#[test]
fn test_ssa_unreachable_use_is_skipped() {
    // Arrange：ValueId(0) 全函数无定义，唯一使用在不可达块 1。
    let blocks = vec![
        block(
            0,
            vec![Instruction::Ret {
                value: None,
                span: Span::dummy(),
            }],
            vec![],
        ),
        block(
            1,
            vec![Instruction::Move {
                dst: Operand::Value(ValueId(1)),
                src: Operand::Value(ValueId(0)),
                span: Span::dummy(),
            }],
            vec![],
        ),
    ];
    let values = vec![
        value_info(MonoType::TypeRef("?".to_string())),
        value_info(MonoType::TypeRef("?".to_string())),
    ];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("不可达使用不得让构造失败");

    // Assert
    assert_eq!(
        output.blocks[0].instructions.len(),
        1,
        "不可达使用不得触发入口 Load Void 物化"
    );
    assert!(
        !output.blocks[0]
            .instructions
            .iter()
            .any(|instruction| matches!(
                instruction,
                Instruction::Load {
                    src: Operand::Const(ConstValue::Void),
                    ..
                }
            )),
        "不可达使用跳过时入口不得出现 Load Void"
    );
}

#[test]
fn test_ssa_partial_predecessor_undefined_is_materialized() {
    // Arrange：v0 在块 1/2 有定义，块 3 无定义；块 4 是汇合块（前驱 1/2/3）。
    // 块 3 路径上的 v0 必须入口物化 Load Void，而非报错或猜值。
    let blocks = vec![
        block(0, vec![], vec![1, 2, 3]),
        block(
            1,
            vec![load(Operand::Value(ValueId(0)), ConstValue::Int(1))],
            vec![4],
        ),
        block(
            2,
            vec![load(Operand::Value(ValueId(0)), ConstValue::Int(2))],
            vec![4],
        ),
        block(3, vec![], vec![4]),
        block(4, vec![ret(Operand::Value(ValueId(0)))], vec![]),
    ];
    let values = vec![value_info(int_ty())];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("部分前驱未定义必须能物化");

    // Assert
    assert_ssa(&output.blocks, &output.values, 0);
    let phi_block = output
        .blocks
        .iter()
        .find(|block| block.label == 4)
        .expect("汇合块必须存在");
    let incoming = phi_block
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            Instruction::Phi { incoming, .. } => Some(incoming),
            _ => None,
        })
        .expect("汇合块必须插入 Phi");
    assert_eq!(
        incoming.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
        vec![1, 2, 3],
        "Phi incoming 必须覆盖三个前驱"
    );
    let (_, missing_source) = incoming
        .iter()
        .find(|(label, _)| *label == 3)
        .expect("块 3 的 incoming 必须存在");
    let Operand::Value(missing_id) = missing_source else {
        panic!("块 3 的 incoming 必须是物化出的 Value，实际 {missing_source:?}");
    };
    assert_eq!(
        output.values[missing_id.0 as usize].ty,
        MonoType::TypeRef("?".to_string()),
        "部分前驱物化值必须记显式未知类型"
    );
    assert_eq!(
        output.values[missing_id.0 as usize].defining_block, 0,
        "部分前驱物化必须定义在入口块"
    );
    assert!(
        matches!(
            output.blocks[0].instructions.first(),
            Some(Instruction::Load {
                dst: Operand::Value(id),
                src: Operand::Const(ConstValue::Void),
                ..
            }) if *id == *missing_id
        ),
        "入口首条指令必须是块 3 incoming 的 Load Void，实际 {:?}",
        output.blocks[0].instructions.first()
    );
}

#[test]
fn test_ssa_reachable_zero_def_value_is_materialized() {
    // Arrange：ValueId(0) 全函数 0 定义，但在可达块 0 被 Ret 使用。
    let blocks = vec![block(0, vec![ret(Operand::Value(ValueId(0)))], vec![])];
    let values = vec![value_info(MonoType::TypeRef("?".to_string()))];

    // Act
    let output = construct_ssa(blocks, values, 0, &[]).expect("可达 0 定义值必须能物化");

    // Assert
    assert_eq!(
        output.blocks[0].instructions.len(),
        2,
        "可达 0 定义值必须在入口补恰好一条 Load Void"
    );
    assert!(
        matches!(
            &output.blocks[0].instructions[0],
            Instruction::Load {
                dst: Operand::Value(ValueId(0)),
                src: Operand::Const(ConstValue::Void),
                ..
            }
        ),
        "入口首条指令必须是 ValueId(0) 的 Load Void，实际 {:?}",
        output.blocks[0].instructions[0]
    );
    assert_eq!(
        output.values[0].defining_block, 0,
        "物化值必须登记真实定义块 label"
    );
    assert_ssa(&output.blocks, &output.values, 0);
}

// ── 真实 lowering 产物 ─────────────────────

#[test]
fn test_lowered_control_flow_values_are_single_definition_and_dominated() {
    // Arrange：if 表达式尾值（多定义）+ while 循环 + 直接调用，走真实生产入口。
    let source = r#"
use std.assert

choose: (n: Int) -> Int = {
    if n == 0 { 1 } else { 2 }
}

count: () -> Int = {
    mut i = 0
    while i < 3 { i = i + 1 }
    i
}

main: () -> Void = {
    assert.assert(choose(0) == 1, "choose 0")
    assert.assert(choose(1) == 2, "choose 1")
    assert.assert(count() == 3, "count")
}

main()
"#;
    let ir = Compiler::new()
        .compile("ssa_probe.yx", source)
        .expect("SSA 探针源码必须能编译");

    // Act & Assert：逐 FunctionBody::Code 断言可达 SSA 形态。
    let mut checked = 0usize;
    for function in &ir.functions {
        if let FunctionBody::Code { blocks, values, .. } = &function.body {
            assert_ssa(blocks, values, function.entry());
            checked += 1;
        }
    }
    assert!(
        checked >= 3,
        "必须至少检查 choose/count/main 三个函数体，实际 {checked}"
    );
}
