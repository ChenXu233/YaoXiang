//! translator `Phi` 臂（方案 A）判据 — 基于 04-ssa.md §「`Phi` 的字节码形态」
//! 方案 A 与 09a §P7 7b.3 契约 §二 第 2–4 条
//!
//! 方案 A：`Phi` 是 IR-only 伪指令，translator 按**前驱**把它展开为汇合块入口的
//! `Move { dst: vreg, src: incoming[i] }` 串；不新增 opcode、`.42` 格式零变更、
//! `executor/` 零改动。`Operand::Value` 经 7b.2 的 `value_slots` 解析为 `u8` 槽。
//!
//! 本文件五个锚：
//! 1. 单前驱：Move 串落在块入口、组内无跳转；
//! 2. 多前驱：运行时按被选前驱取到对应 incoming（菱形 CFG，两条入边都是跳转）；
//! 3. 临界边：条件跳转直达汇合块 ⇒ 该跳转必须重定向到本前驱的组（否则拿到错值）；
//! 4. `.42` 写读往返后字节码一致，且解释执行与直接翻译结果一致；
//! 5. 组内并行拷贝环响亮失败（E8001），不静默错值。
//!
//! `Phi` 生产点仍为 0：全部夹具都是**合成 IR**（不经 lowering）。

use crate::backends::Executor;
use crate::frontend::core::sema::MonoType;
use crate::middle::bytecode::file::BytecodeFile;
use crate::middle::bytecode::{BytecodeInstr, BytecodeModule};
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, GlobalSlot, Instruction, LocalSlot, ModuleIR,
    Operand, ValueId, ValueInfo,
};
use crate::middle::passes::codegen::CodegenContext;
use crate::util::span::Span;

// ── 辅助函数（规则 5.3：集中在文件顶部）──────────────

fn ssa_value(id: u32) -> Operand {
    Operand::Value(ValueId(id))
}

fn load_int(
    dst: Operand,
    n: i128,
) -> Instruction {
    Instruction::Load {
        dst,
        src: Operand::Const(ConstValue::Int(n)),
        span: Span::dummy(),
    }
}

fn load_bool(
    dst: Operand,
    value: bool,
) -> Instruction {
    Instruction::Load {
        dst,
        src: Operand::Const(ConstValue::Bool(value)),
        span: Span::dummy(),
    }
}

fn jmp(target: usize) -> Instruction {
    Instruction::Jmp {
        target,
        span: Span::dummy(),
    }
}

fn jmp_if(
    cond: Operand,
    target: usize,
) -> Instruction {
    Instruction::JmpIf {
        cond,
        target,
        span: Span::dummy(),
    }
}

fn store_global(
    index: usize,
    src: Operand,
) -> Instruction {
    Instruction::Store {
        dst: Operand::Global(index),
        src,
        span: Span::dummy(),
    }
}

fn ret_void() -> Instruction {
    Instruction::Ret {
        value: None,
        span: Span::dummy(),
    }
}

fn phi(
    dst: Operand,
    incoming: Vec<(usize, Operand)>,
) -> Instruction {
    Instruction::Phi {
        dst,
        incoming,
        span: Span::dummy(),
    }
}

/// 值表：`slots.len()` 条 `Int(64)` 值，定义块取 `blocks`（与 `value_slots` 同索引）。
fn value_table(
    defining_blocks: &[usize],
    slots: Vec<usize>,
) -> (Vec<ValueInfo>, Vec<usize>) {
    let values: Vec<ValueInfo> = defining_blocks
        .iter()
        .map(|block| ValueInfo {
            ty: MonoType::Int(64),
            debug_name: None,
            defining_block: *block,
        })
        .collect();
    (values, slots)
}

/// 单函数模块：`main` + 全局槽位 0（结果可经解释器读回）。
fn module_with_main(
    blocks: Vec<BasicBlock>,
    locals: Vec<LocalSlot>,
    values: Vec<ValueInfo>,
    value_slots: Vec<usize>,
) -> ModuleIR {
    let mut module = ModuleIR {
        functions: vec![FunctionIR {
            def: None,
            name: "main".to_string(),
            params: Vec::new(),
            return_type: MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks,
                entry: 0,
                locals,
                values,
                value_slots,
            },
        }],
        ..Default::default()
    };
    module.globals.push(GlobalSlot {
        name: "result".to_string(),
        ty: MonoType::Int(64),
        index: 0,
    });
    module.entry_function = Some("main".to_string());
    module
}

/// 单前驱夹具：块 0 直线落空到块 1，块 1 的 `Phi` 只有前驱 0。
///
/// 展平：0 `Load v0=42` / 1 `Phi v1 <- [0:v0]` / 2 `Store Global(0) <- v1` / 3 `Ret`
fn single_predecessor_module() -> ModuleIR {
    let (values, value_slots) = value_table(&[0, 1], vec![1, 2]);
    module_with_main(
        vec![
            BasicBlock {
                label: 0,
                instructions: vec![load_int(ssa_value(0), 42)],
                successors: vec![1],
            },
            BasicBlock {
                label: 1,
                instructions: vec![
                    phi(ssa_value(1), vec![(0, ssa_value(0))]),
                    store_global(0, ssa_value(1)),
                    ret_void(),
                ],
                successors: Vec::new(),
            },
        ],
        vec![
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
        ],
        values,
        value_slots,
    )
}

/// 菱形夹具：块 0 条件分支到块 1 / 块 2，两块各写一个值后**跳转**到汇合块 3。
///
/// 展平：0 `Load Local(0)` / 1 `JmpIf(->4)` / 2 `Load v0=10` / 3 `Jmp(->6)` /
/// 4 `Load v1=20` / 5 `Jmp(->6)` / 6 `Phi v2 <- [1:v0, 2:v1]` /
/// 7 `Store Global(0) <- v2` / 8 `Ret`
fn diamond_module(condition: bool) -> ModuleIR {
    let (values, value_slots) = value_table(&[1, 2, 3], vec![1, 2, 3]);
    module_with_main(
        vec![
            BasicBlock {
                label: 0,
                instructions: vec![
                    load_bool(Operand::Local(0), condition),
                    jmp_if(Operand::Local(0), 4),
                ],
                successors: vec![1, 2],
            },
            BasicBlock {
                label: 1,
                instructions: vec![load_int(ssa_value(0), 10), jmp(6)],
                successors: vec![3],
            },
            BasicBlock {
                label: 2,
                instructions: vec![load_int(ssa_value(1), 20), jmp(6)],
                successors: vec![3],
            },
            BasicBlock {
                label: 3,
                instructions: vec![
                    phi(ssa_value(2), vec![(1, ssa_value(0)), (2, ssa_value(1))]),
                    store_global(0, ssa_value(2)),
                    ret_void(),
                ],
                successors: Vec::new(),
            },
        ],
        vec![
            LocalSlot::temp(MonoType::Bool),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
        ],
        values,
        value_slots,
    )
}

/// 临界边夹具：块 0 的条件跳转**直达**汇合块 2；块 1 是落空块（块尾不是跳转，
/// 布局落空进汇合块），故汇合块的两条入边分别是「条件跳转边」与「落空边」。
///
/// 组序按落空前驱领首：前驱 1 的组在块入口（落空路径命中），前驱 0 的组在其后——
/// 条件跳转必须被重定向到自己的组，否则会执行前驱 1 的 Move 而拿到 20。
///
/// 展平：0 `Load Local(0)=条件` / 1 `Load v0=10` / 2 `JmpIf(->4)` /
/// 3 `Load v1=20` / 4 `Phi v2 <- [0:v0, 1:v1]` / 5 `Store Global(0) <- v2` / 6 `Ret`
fn critical_edge_module(condition: bool) -> ModuleIR {
    let (values, value_slots) = value_table(&[0, 1, 2], vec![1, 2, 3]);
    module_with_main(
        vec![
            BasicBlock {
                label: 0,
                instructions: vec![
                    load_bool(Operand::Local(0), condition),
                    load_int(ssa_value(0), 10),
                    jmp_if(Operand::Local(0), 4),
                ],
                successors: vec![1, 2],
            },
            BasicBlock {
                label: 1,
                instructions: vec![load_int(ssa_value(1), 20)],
                successors: vec![2],
            },
            BasicBlock {
                label: 2,
                instructions: vec![
                    phi(ssa_value(2), vec![(0, ssa_value(0)), (1, ssa_value(1))]),
                    store_global(0, ssa_value(2)),
                    ret_void(),
                ],
                successors: Vec::new(),
            },
        ],
        vec![
            LocalSlot::temp(MonoType::Bool),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
        ],
        values,
        value_slots,
    )
}

/// 并行拷贝环夹具：汇合块的第二个 `Phi` 以前一个 `Phi` 的 dst 为源 ⇒ 同组内
/// 「先写槽、后读槽」，顺序执行需要临时槽（本批没有）。
fn parallel_copy_cycle_module() -> ModuleIR {
    let (values, value_slots) = value_table(&[1, 1, 2, 2], vec![1, 2, 3, 4]);
    module_with_main(
        vec![
            BasicBlock {
                label: 0,
                instructions: vec![
                    load_bool(Operand::Local(0), true),
                    jmp_if(Operand::Local(0), 4),
                ],
                successors: vec![1, 2],
            },
            BasicBlock {
                label: 1,
                instructions: vec![load_int(ssa_value(0), 10), jmp(4)],
                successors: vec![3],
            },
            BasicBlock {
                label: 2,
                instructions: vec![load_int(ssa_value(1), 20), jmp(4)],
                successors: vec![3],
            },
            BasicBlock {
                label: 3,
                instructions: vec![
                    phi(ssa_value(2), vec![(1, ssa_value(0)), (2, ssa_value(1))]),
                    phi(ssa_value(3), vec![(1, ssa_value(2)), (2, ssa_value(2))]),
                    store_global(0, ssa_value(3)),
                    ret_void(),
                ],
                successors: Vec::new(),
            },
        ],
        vec![
            LocalSlot::temp(MonoType::Bool),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
            LocalSlot::temp(MonoType::Int(64)),
        ],
        values,
        value_slots,
    )
}

/// 直接翻译（不经 `.42`）：`CodegenContext` → `BytecodeModule`。
fn translate_direct(module: ModuleIR) -> BytecodeModule {
    let mut context = CodegenContext::new(module);
    let file = context.generate().expect("codegen 必须成功");
    BytecodeModule::from(file)
}

/// `.42` 写读往返后再解码：`write_to` → `read_from` → `BytecodeModule`。
fn translate_through_dot42(module: ModuleIR) -> BytecodeModule {
    let mut context = CodegenContext::new(module);
    let file = context.generate().expect("codegen 必须成功");
    let mut buffer = Vec::new();
    file.write_to(&mut buffer).expect(".42 写入必须成功");
    let mut cursor = std::io::Cursor::new(&buffer);
    let restored = BytecodeFile::read_from(&mut cursor).expect(".42 读取必须成功");
    BytecodeModule::from(restored)
}

/// 执行模块并读回全局槽位 0 的整数。
fn execute_result(module: &BytecodeModule) -> i64 {
    let mut interpreter = crate::backends::interpreter::Interpreter::new();
    interpreter.execute_module(module).expect("执行必须成功");
    match interpreter.global_slot(0) {
        Some(crate::backends::common::RuntimeValue::Int(value)) => *value,
        other => panic!("全局槽位 0 应为 Int，实际 {other:?}"),
    }
}

/// 提取 main 的字节码指令（解码后的形态）。
fn main_instructions(module: &BytecodeModule) -> &[BytecodeInstr] {
    &module
        .functions
        .first()
        .expect("往返模块必须有 main 函数")
        .instructions
}

/// 列出 main 中所有 `Mov { dst, src }` 的槽号对（方案 A 的 Move 串形态）。
fn mov_pairs(module: &BytecodeModule) -> Vec<(u16, u16)> {
    main_instructions(module)
        .iter()
        .filter_map(|instruction| match instruction {
            BytecodeInstr::Mov { dst, src } => Some((dst.index(), src.index())),
            _ => None,
        })
        .collect()
}

// ── 单前驱：Move 串落在块入口、组内无跳转 ──────────────

/// 单前驱 `Phi` 展开为一条 `Mov`（v1 槽 2 ← v0 槽 1），且不引入跳转——
/// 组内只有 Move 是全批最简形态（09a §P7 7b.3 契约 §二 第 2 条）。
#[test]
fn test_phi_single_predecessor_expands_to_move_without_jump() {
    // Arrange
    let module = translate_direct(single_predecessor_module());

    // Act
    let pairs = mov_pairs(&module);
    let has_jump = main_instructions(&module)
        .iter()
        .any(|instruction| matches!(instruction, BytecodeInstr::Jmp { .. }));

    // Assert
    assert_eq!(
        pairs,
        vec![(2, 1)],
        "单前驱 Phi 必须展开为恰好一条 Mov(槽 2 ← 槽 1)，实际 {pairs:?}"
    );
    assert!(!has_jump, "单前驱的 Phi 组内不得有跳转");
}

// ── 多前驱：运行时按被选前驱取值 ──────────────

/// 菱形 CFG 两条入边都是无条件跳转：条件成立取 20（块 2），否则取 10（块 1）。
#[test]
fn test_phi_multi_predecessor_selects_move_by_taken_edge() {
    // Arrange
    let taken_true = translate_direct(diamond_module(true));
    let taken_false = translate_direct(diamond_module(false));

    // Act
    let value_true = execute_result(&taken_true);
    let value_false = execute_result(&taken_false);

    // Assert
    assert_eq!(
        value_true, 20,
        "条件成立必须走块 2 的入边，Phi 取 v1=20，实际 {value_true}"
    );
    assert_eq!(
        value_false, 10,
        "条件不成立必须走块 1 的入边，Phi 取 v0=10，实际 {value_false}"
    );
    assert_eq!(
        mov_pairs(&taken_true).len(),
        2,
        "两个前驱各一条 Mov（汇合块入口的组串），实际 {:?}",
        mov_pairs(&taken_true)
    );
}

// ── 临界边：条件跳转重定向到自己的组 ──────────────

/// 条件跳转**直达**汇合块时，它必须被重定向到前驱 0 的组（组序里排第二）；
/// 若落在块入口会执行前驱 1 的 Move，条件成立也拿到 20。
#[test]
fn test_phi_conditional_edge_is_forwarded_to_own_group() {
    // Arrange
    let taken_true = translate_direct(critical_edge_module(true));
    let taken_false = translate_direct(critical_edge_module(false));

    // Act
    let value_true = execute_result(&taken_true);
    let value_false = execute_result(&taken_false);

    // Assert
    assert_eq!(
        value_true, 10,
        "条件成立经条件跳转边到达，Phi 必须取前驱 0 的 v0=10，实际 {value_true}"
    );
    assert_eq!(
        value_false, 20,
        "条件不成立经落空边到达，Phi 必须取前驱 1 的 v1=20，实际 {value_false}"
    );
}

// ── `.42` 往返与解释执行一致 ──────────────

/// `.42` 写读往返后 bytecode 逐条一致（opcode + 操作数），且解释执行结果与
/// 直接翻译一致——方案 A 不改 `.42` 格式，`Phi` 在 translator 阶段已消失。
#[test]
fn test_phi_bytecode_roundtrips_through_dot42() {
    // Arrange
    let direct = translate_direct(diamond_module(true));
    let roundtrip = translate_through_dot42(diamond_module(true));

    // Act — 逐条比较解码后的指令（含 opcode 与操作数；`BytecodeInstr` 不带
    // `PartialEq`，用 Debug 文本作等价判据，失败时可直接读差）
    let direct_codes = format!("{:?}", main_instructions(&direct));
    let roundtrip_codes = format!("{:?}", main_instructions(&roundtrip));
    let direct_value = execute_result(&direct);
    let roundtrip_value = execute_result(&roundtrip);

    // Assert
    assert_eq!(
        roundtrip_codes, direct_codes,
        ".42 往返必须保持字节码逐条一致（方案 A 不改格式）"
    );
    assert_eq!(
        roundtrip_value, direct_value,
        "解释执行经 .42 往返与直接翻译必须同值"
    );
}

// ── 并行拷贝环：响亮失败 ──────────────

/// 同组内出现「先写槽、后读槽」的并行拷贝环时，translator 必须返回 E8001
/// 内部错误（需要临时槽，而临时槽归 7b.4 的分配器），不静默错值。
#[test]
fn test_phi_group_parallel_copy_cycle_is_rejected() {
    // Arrange
    let mut context = CodegenContext::new(parallel_copy_cycle_module());

    // Act
    let result = context.generate();

    // Assert
    let diagnostic = result.expect_err("并行拷贝环必须被拒绝，不得静默生成错值字节码");
    assert!(
        diagnostic.message.contains("并行拷贝环"),
        "诊断必须点名并行拷贝环，实际 {}",
        diagnostic.message
    );
}
