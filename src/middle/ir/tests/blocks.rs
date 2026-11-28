//! 基本块切分与块级 CFG 判据 — 基于 04-ssa.md §目标设计「`Phi` 作为新
//! `Instruction` 变体」三条构造约束之二
//!
//! 04 §目标设计：`Phi` 的 `incoming` 键是**前驱块 label**，前驱集由
//! `BasicBlock.successors` 反推（`11 号不变量 `Phi` 一致性` / 07 §第一层」）。
//! 故块必须是真基本块、出边必须真实——本文件的判据分两层：
//! 1. `partition_blocks` 的切分与出边推导（直线体 / 条件分支双边 + 落空 /
//!    循环回边 / 真自环 / 越界目标 / 空体）；
//! 2. **真实 lowering 产物**上的块级 CFG（while 循环体切成多块、存在分支出边
//!    与回边），以及该数据被 `verify_ssa` 的块级前驱判据实际消费。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{
    partition_blocks, ConstValue, FunctionBody, Instruction, ModuleIR, Operand, ValueId,
};
use crate::middle::ir::verify::{verify_ssa, InvariantKind};
use crate::middle::lower::AstToIrGenerator;
use crate::util::span::Span;

// ── 辅助函数（规则 5.3：集中在文件顶部）──────────────

/// `Load dst <- Const(Int(n))`。
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

/// `JmpIfNot cond -> target`（成立时跳转；不成立时落空到布局下一块）。
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

fn jmp(target: usize) -> Instruction {
    Instruction::Jmp {
        target,
        span: Span::dummy(),
    }
}

fn ret_void() -> Instruction {
    Instruction::Ret {
        value: None,
        span: Span::dummy(),
    }
}

/// 走 lex→parse→typecheck→lower 真实生成序列，返回 ModuleIR。
fn lower_source(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("测试源码必须能 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "测试源码必须无解析错误：{:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("blocks_test.yx");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "测试源码必须无类型错误：{:?}",
        type_result.diagnostics
    );
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .expect("测试源码必须能生成 IR")
}

/// 由 `successors` 反推块级前驱（与 `verify_ssa` 的 `block_predecessors` 同义，
/// 测试侧独立复算，避免用被测实现自证）。
fn predecessors(
    blocks: &[crate::middle::ir::ir::BasicBlock]
) -> std::collections::HashMap<usize, std::collections::HashSet<usize>> {
    let mut table: std::collections::HashMap<usize, std::collections::HashSet<usize>> =
        std::collections::HashMap::new();
    for block in blocks {
        for successor in &block.successors {
            table.entry(*successor).or_default().insert(block.label);
        }
    }
    table
}

/// while 循环体：至少一个回边 + 一个条件分支，用于验证真实 lowering 产物的块级 CFG。
const WHILE_SOURCE: &str = "
main: () -> Void = {
    mut i = 0
    while i < 3 {
        i = i + 1
    }
}
";

// ── 切分与出边推导（合成指令流）──────────────

/// 直线体不切分：单块、label 0、无出边——这是「块切分不进快照 / 不改字节码」
/// 的结构前提（既有语料里大量函数体是直线体）。
#[test]
fn test_partition_blocks_straight_line_body_is_one_block() {
    // Arrange
    let instructions = vec![load_int(Operand::Local(0), 1), ret_void()];

    // Act
    let blocks = partition_blocks(instructions);

    // Assert
    assert_eq!(
        blocks.len(),
        1,
        "直线体必须只有一块，实际 {} 块",
        blocks.len()
    );
    assert_eq!(blocks[0].label, 0, "唯块的 label 必须是 0（entry）");
    assert_eq!(blocks[0].instructions.len(), 2, "块内指令数必须与输入一致");
    assert!(
        blocks[0].successors.is_empty(),
        "直线体没有块级出边，实际 {:?}",
        blocks[0].successors
    );
}

/// 条件跳转切出「目标块 + 落空块」两条边：出边是块 label（不是指令下标），
/// 且落空边来自布局（`JmpIfNot` 不成立时执行下一条指令所属块）。
#[test]
fn test_partition_blocks_conditional_jump_splits_target_and_fallthrough() {
    // Arrange — 0:Load 1:JmpIfNot(->3) 2:Load 3:Ret
    let instructions = vec![
        load_int(Operand::Local(0), 1),
        jmp_if_not(Operand::Local(0), 3),
        load_int(Operand::Local(1), 2),
        ret_void(),
    ];

    // Act
    let blocks = partition_blocks(instructions);

    // Assert
    let labels: Vec<usize> = blocks.iter().map(|block| block.label).collect();
    assert_eq!(
        labels,
        vec![0, 1, 2],
        "leader 切分应得到三块，实际 {labels:?}"
    );
    assert_eq!(
        blocks[0].successors,
        vec![1, 2],
        "条件跳转块必须有目标块 2 与落空块 1 两条边，实际 {:?}",
        blocks[0].successors
    );
    assert_eq!(
        blocks[1].successors,
        vec![2],
        "落空块必须以布局落空连到块 2，实际 {:?}",
        blocks[1].successors
    );
    assert!(
        blocks[2].successors.is_empty(),
        "Ret 结尾的块没有出边，实际 {:?}",
        blocks[2].successors
    );
    assert_eq!(
        blocks
            .iter()
            .map(|block| block.instructions.len())
            .sum::<usize>(),
        4,
        "切分不得增删指令（跳转目标坐标因此保持不变）"
    );
}

/// 循环回边：`Jmp` 目标是函数首条指令 ⇒ 边指向块 0；块级前驱因此非空，
/// 这是 `verify_ssa` 前驱判据在真实 IR 上的数据来源。
#[test]
fn test_partition_blocks_loop_back_edge_points_to_head_block() {
    // Arrange — 0:Load 1:JmpIfNot(->4) 2:Load 3:Jmp(->0) 4:Ret
    let instructions = vec![
        load_int(Operand::Local(0), 1),
        jmp_if_not(Operand::Local(0), 4),
        load_int(Operand::Local(1), 2),
        jmp(0),
        ret_void(),
    ];

    // Act
    let blocks = partition_blocks(instructions);

    // Assert
    assert_eq!(
        blocks.len(),
        3,
        "循环体应切成三块，实际 {} 块",
        blocks.len()
    );
    assert_eq!(
        blocks[1].successors,
        vec![0],
        "回边必须指向块 0（循环头），实际 {:?}",
        blocks[1].successors
    );
    assert_eq!(
        predecessors(&blocks).get(&0),
        Some(&std::collections::HashSet::from([1])),
        "块 0 的块级前驱必须由回边反推出 {{1}}"
    );
}

/// 单条 `Jmp` 自指成**真自环**（块的唯一指令把控制交回本块块首）。
#[test]
fn test_partition_blocks_self_jump_becomes_self_edge() {
    // Arrange
    let instructions = vec![jmp(0)];

    // Act
    let blocks = partition_blocks(instructions);

    // Assert
    assert_eq!(
        blocks.len(),
        1,
        "自指跳转只有一块，实际 {} 块",
        blocks.len()
    );
    assert_eq!(
        blocks[0].successors,
        vec![0],
        "自指跳转必须形成块 0 的自环，实际 {:?}",
        blocks[0].successors
    );
}

/// 越界目标不产生边：目标合法性是 `verify` 的「jump 目标存在」判据职责，
/// `partition_blocks` 只做块图推导（避免两处各判一次、口径分叉）。
#[test]
fn test_partition_blocks_out_of_range_jump_adds_no_edge() {
    // Arrange
    let instructions = vec![jmp(9)];

    // Act
    let blocks = partition_blocks(instructions);

    // Assert
    assert_eq!(blocks.len(), 1, "越界目标不得切出新块");
    assert!(
        blocks[0].successors.is_empty(),
        "越界目标不得产生块级出边，实际 {:?}",
        blocks[0].successors
    );
}

/// 空体保持「一块、label 0、无指令、无出边」——渲染与翻译路径对空体的既有
/// 形态不变（`normalize` 对空块不输出任何行）。
#[test]
fn test_partition_blocks_empty_body_keeps_single_empty_block() {
    // Arrange & Act
    let blocks = partition_blocks(Vec::new());

    // Assert
    assert_eq!(
        blocks.len(),
        1,
        "空体必须保持单块形态，实际 {} 块",
        blocks.len()
    );
    assert_eq!(blocks[0].label, 0, "空块的 label 必须是 0");
    assert!(
        blocks[0].instructions.is_empty(),
        "空块不得有指令，实际 {} 条",
        blocks[0].instructions.len()
    );
    assert!(
        blocks[0].successors.is_empty(),
        "空块没有出边，实际 {:?}",
        blocks[0].successors
    );
}

// ── 真实 lowering 产物上的块级 CFG 与消费端 ──────────────

/// 真 lowering 的 while 函数体必须切成多块，并同时具备**分支出边**与**回边**：
/// 块级 CFG 自此在真实 IR 上有数据（7b.1b 登记的前置条件落地）。
#[test]
fn test_lowered_while_body_has_real_block_successors() {
    // Arrange
    let module = lower_source(WHILE_SOURCE);
    let main = module
        .functions
        .iter()
        .find(|func| func.name == "main")
        .expect("WHILE_SOURCE 必须产出 main 函数");

    // Act
    let blocks = main.blocks();

    // Assert
    assert!(
        blocks.len() >= 2,
        "while 循环体必须切成多块，实际 {} 块",
        blocks.len()
    );
    assert!(
        blocks.iter().any(|block| block.successors.len() >= 2),
        "循环条件块必须有两条出边（目标 + 落空），实际 {:?}",
        blocks
            .iter()
            .map(|block| (&block.label, &block.successors))
            .collect::<Vec<_>>()
    );
    assert!(
        blocks
            .iter()
            .any(|block| block.successors.iter().any(|succ| *succ <= block.label)),
        "循环体必须有回边（后继 label ≤ 本块 label），实际 {:?}",
        blocks
            .iter()
            .map(|block| (&block.label, &block.successors))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        blocks
            .iter()
            .map(|block| block.instructions.len())
            .sum::<usize>(),
        main.all_instructions().count(),
        "切分不得增删指令（展平序与总数守恒）"
    );
    assert!(
        !predecessors(blocks).is_empty(),
        "真实 lowering 产物的块级前驱不得为空（否则 verify_ssa 的块级判据无数据）"
    );
}

/// 块级前驱判据的**消费端**证据：在真实 lowering 产物里插入 `incoming` 为空的
/// `Phi`，`verify_ssa` 必须按**真实 successors**报出前驱数差异（前驱集非空）。
///
/// 这是「登记-消费」闭环：successors 由 `partition_blocks` 生产，前驱判据消费它；
/// 若 successors 退回空表，本断言会因「前驱=[]」而失败。
#[test]
fn test_ssa_phi_on_lowered_blocks_reports_real_predecessor_count() {
    // Arrange — 找 main 里第一个有前驱的块，在块首注入 incoming 为空的 Phi
    let mut module = lower_source(WHILE_SOURCE);
    let main_index = module
        .functions
        .iter()
        .position(|func| func.name == "main")
        .expect("WHILE_SOURCE 必须产出 main 函数");
    let (block_index, predecessor_labels) = match &module.functions[main_index].body {
        FunctionBody::Code { blocks, .. } => {
            let table = predecessors(blocks);
            let (index, labels) = blocks
                .iter()
                .enumerate()
                .find_map(|(index, block)| {
                    let labels = table.get(&block.label)?;
                    (!labels.is_empty()).then(|| (index, labels.clone()))
                })
                .expect("真实 lowering 产物必须存在有前驱的块");
            (index, labels)
        }
        FunctionBody::TypeDecl { .. } => panic!("main 必须是 Code 体"),
    };
    if let FunctionBody::Code { blocks, .. } = &mut module.functions[main_index].body {
        blocks[block_index].instructions.insert(
            0,
            Instruction::Phi {
                dst: Operand::Value(ValueId(0)),
                incoming: Vec::new(),
                span: Span::dummy(),
            },
        );
    }

    // Act
    let error = verify_ssa(&module).expect_err("incoming 为空的 Phi 必被块级前驱判据拒绝");

    // Assert
    let messages: Vec<&String> = error
        .violations
        .iter()
        .filter(|violation| violation.invariant == InvariantKind::PhiConsistency)
        .map(|violation| &violation.message)
        .collect();
    assert!(
        messages.iter().any(|message| message.contains("前驱数")),
        "必须报前驱数差异（前驱集 {predecessor_labels:?}），实际 {messages:?}"
    );
    assert!(
        messages.iter().all(|message| !message.contains("前驱=[]")),
        "前驱集不得为空——为空说明 successors 仍是空表（本批的登记-消费链断了），实际 {messages:?}"
    );
}
