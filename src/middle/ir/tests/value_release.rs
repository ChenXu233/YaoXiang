//! #591 判据：**值**（SSA 临时值）的释放事实。
//!
//! 性质 =「直线函数里，每个需要释放的值恰有一个释放点，且该点不早于它的任何
//! 使用」；多块函数（含分支/回边/`Phi`）**不发**值释放事实（负控）。
//!
//! # 门依据（coding-rules 第七部分）
//!
//! **条款**：`docs/src/dev/architecture/04-ssa.md` 的槽位复用与 RFC-039 D20 的
//! 跨阶段事实口径——释放事实由生产侧（lower）给出，消费侧（AOT 所有权）不自行
//! 重算。**病史**：#591 记录「Drop 全为 local、无 Drop(Value)」，AOT 只能把 Drop
//! 当 NOP。**证明门**：换一个正确的替代实现（例如在 regalloc 的活跃区间上算释放
//! 点）不会让本条变红——断言的是「释放点存在且在使用之后」，不是实现位置。
//!
//! 边界（不在本条判据内）：多块函数的值释放（CFG 活跃性，属后续增量）；
//! `Ret` 处的值（随返回逸出，不释放）；原语值（Int/Float/Bool/Char，编译器复制，
//! 无资源可释放）。

use crate::frontend::core::types::trait_data::TraitTable;
use crate::middle::ir::ir::{FunctionBody, Instruction, ModuleIR, Operand, ValueId, ValueInfo};

use super::support::compile_ir;

/// `main` 的单块指令序列与值表（多块返回 None）。
fn single_block(ir: &ModuleIR) -> Option<(&Vec<Instruction>, &Vec<ValueInfo>)> {
    for function in &ir.functions {
        if function.name != "main" {
            continue;
        }
        let FunctionBody::Code { blocks, values, .. } = &function.body else {
            continue;
        };
        if blocks.len() != 1 {
            return None;
        }
        return Some((&blocks[0].instructions, values));
    }
    None
}

/// 该值在指令序列里的全部**读取**下标（`use_operands` 是使用点的唯一判据）。
///
/// `Drop` 自身也读它释放的那个值——但释放点正是本判据的被断言对象，故排除，
/// 否则「最后一次使用」会退化成「释放点自己」而判据空转。
fn read_positions(
    instructions: &[Instruction],
    vid: ValueId,
) -> Vec<usize> {
    let mut out = Vec::new();
    for (idx, instr) in instructions.iter().enumerate() {
        if matches!(instr, Instruction::Drop { .. }) {
            continue;
        }
        for op in instr.use_operands() {
            if matches!(op, Operand::Value(id) if *id == vid) {
                out.push(idx);
            }
        }
    }
    out
}

/// 直线函数：每个值释放点都紧跟在它最后一次读取之后，且此后不再读取。
#[test]
fn test_value_drops_follow_last_use_in_straight_line_function() {
    // Arrange：单块函数，`s` 的拼接结果是一次纯临时值（String，拥有堆资源）
    let ir = compile_ir(
        "use std.string\n\
         main: () -> Int = {\n\
             s = \"hello\" + \"world\"\n\
             string.len(s)\n\
         }\n",
    );
    let (instructions, values) = single_block(&ir).expect("夹具必须是单块函数");

    // Act：收集所有值释放点
    let drops: Vec<(usize, ValueId)> = instructions
        .iter()
        .enumerate()
        .filter_map(|(idx, instr)| match instr {
            Instruction::Drop {
                src: Operand::Value(id),
                ..
            } => Some((idx, *id)),
            _ => None,
        })
        .collect();

    // Assert：至少一条（判据不空转）
    assert!(
        !drops.is_empty(),
        "直线函数必须产出值释放事实，实际 0 条：{instructions:#?}"
    );
    for (drop_idx, vid) in drops {
        let reads = read_positions(instructions, vid);
        let last_read = reads.iter().copied().max().expect("被释放的值必须被读过");
        // 释放点在使用之后，且此后不再有该值的读取（同一使用点可有多条释放，
        // 组内顺序按 id 升序——故此处判「在使用之后」而不是「紧邻」）。
        assert!(
            drop_idx > last_read,
            "释放点不得早于任何使用（值 {vid:?}）：reads={reads:?}，drop={drop_idx}"
        );
        let later: Vec<usize> = reads.into_iter().filter(|r| *r > drop_idx).collect();
        assert!(
            later.is_empty(),
            "释放点之后不得再读取该值（值 {vid:?}）：later={later:?}，drop={drop_idx}"
        );
        // 原语值（Int/Float/Bool/Char）由编译器复制、无资源可释放——不得出现在释放点里。
        let info = values.get(vid.0 as usize).expect("被释放的值必须在值表里");
        assert!(
            !TraitTable::is_primitive_value_type(&info.ty),
            "原语值不得发释放点：值 {vid:?} 类型 {:?}",
            info.ty
        );
    }
}

/// 负控：多块函数（含 `while` 回边）不发值释放事实——提前释放属 D 类静默错值。
#[test]
fn test_value_drops_absent_for_multi_block_function() {
    // Arrange：函数体含循环（Jmp/JmpIf 切成多块），且同样有 String 临时值
    let ir = compile_ir(
        "use std.string\n\
         main: () -> Int = {\n\
             s = \"hello\" + \"world\"\n\
             mut i = 0\n\
             while i < 1 {\n\
                 i = i + 1\n\
             }\n\
             string.len(s)\n\
         }\n",
    );
    let mut blocks = 0usize;
    let mut value_drops = 0usize;
    for function in &ir.functions {
        if function.name != "main" {
            continue;
        }
        let FunctionBody::Code { blocks: body, .. } = &function.body else {
            continue;
        };
        blocks = body.len();
        for block in body {
            for instr in &block.instructions {
                if matches!(
                    instr,
                    Instruction::Drop {
                        src: Operand::Value(_),
                        ..
                    }
                ) {
                    value_drops += 1;
                }
            }
        }
    }

    // Assert：确是多块，且没有任何值释放事实
    assert!(blocks > 1, "夹具必须是多块函数，实际 {blocks} 块");
    assert_eq!(
        value_drops, 0,
        "多块函数不得发值释放事实（CFG 活跃性未做）：实际 {value_drops} 条"
    );
}
