//! Translator 一元指令编码往返测试 — 基于 RFC-039 决议登记 D33 与字节码域文档
//!
//! RFC-039 D33：`UnaryOp::Not` 静默退化为 `I64_NEG` → **修**，`opcode()` 必须
//! 区分 `op` 字段（「静默改变语义是最危险的一类」）。
//!
//! issue #562：translator 对 `Instruction::Not` 与 `Instruction::Neg` 同发
//! `opcode::I64_NEG`，解码段把该码一律还原为 `UnaryOp::Neg`——编码/解码
//! 往返把 Not 改写为取负。语言规范限定 `!` 操作数为 Bool（`infer_unary`
//! 非 Bool 即报 E1051），现行执行答案靠解释器 Neg-on-Bool 补偿臂兜住；
//! 修复 = Not 换专用指令码 `I64_NOT`，解码还原 `UnaryOp::Not`。
//!
//! 本文件三条锚：
//! - 往返保真（主锚，先红后绿）：IR Not → 编码 → .42 序列化 → 解码，
//!   `op` 字段必须保真为 Not；
//! - 端到端语义（钉住执行面）：Not(Bool) 经序列化往返后执行仍返回逻辑非；
//! - Neg 对照锚：Neg 编码/解码行为逐字不变（修前修后都绿）。

use crate::backends::Executor;
use crate::frontend::core::sema::MonoType;
use crate::middle::bytecode::opcode;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
};
use crate::middle::passes::codegen::CodegenContext;
use crate::util::span::Span;

// -- 辅助函数（规则 5.3：集中在文件顶部）--

/// 构造一个「r0 = bool 常量，r1 = 一元运算(r0)，r1 写入全局槽位 0」的单函数模块。
///
/// `use_not = true` 时一元运算为 `Instruction::Not`（逻辑非 `!x` 的 IR 形态），
/// 否则为 `Instruction::Neg`（取负 `-x` 的 IR 形态）。
/// 结果经全局槽位观察（`execute_module` 装载常量池并执行入口，槽位值可读回）。
fn unary_module(use_not: bool) -> ModuleIR {
    let unary = if use_not {
        Instruction::Not {
            dst: Operand::Local(1),
            src: Operand::Local(0),
            span: Span::default(),
        }
    } else {
        Instruction::Neg {
            dst: Operand::Local(1),
            src: Operand::Local(0),
            span: Span::default(),
        }
    };
    let mut module = ModuleIR::default();
    module.globals.push(crate::middle::ir::ir::GlobalSlot {
        name: "result".to_string(),
        ty: MonoType::Bool,
        index: 0,
    });
    module.entry_function = Some("main".to_string());
    module.functions.push(FunctionIR {
        def: None,
        name: "main".to_string(),
        params: Vec::new(),
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Load {
                        dst: Operand::Local(0),
                        src: Operand::Const(ConstValue::Bool(true)),
                        span: Span::default(),
                    },
                    unary,
                    Instruction::Store {
                        dst: Operand::Global(0),
                        src: Operand::Local(1),
                        span: Span::default(),
                    },
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![
                LocalSlot::temp(MonoType::Bool),
                LocalSlot::temp(MonoType::Bool),
            ],
        },
    });
    module
}

/// 编译为 BytecodeFile，做 .42 内存缓冲往返（write_to → read_from），
/// 再经 `From<BytecodeFile> for BytecodeModule` 解码——即所有执行入口
/// （源码 run / .42 直跑）共用的有损面。
fn generate_roundtrip_module(use_not: bool) -> crate::middle::bytecode::BytecodeModule {
    let module = unary_module(use_not);
    let mut ctx = CodegenContext::new(module);
    let file = ctx.generate().expect("codegen should succeed");

    let mut buf = Vec::new();
    file.write_to(&mut buf).expect("write_to should succeed");
    let mut cursor = std::io::Cursor::new(&buf);
    let restored = crate::middle::bytecode::file::BytecodeFile::read_from(&mut cursor)
        .expect("read_from should succeed");
    crate::middle::bytecode::BytecodeModule::from(restored)
}

/// 取解码后 main 函数的第一条一元指令。
fn first_unary_instr(
    module: &crate::middle::bytecode::BytecodeModule
) -> crate::middle::bytecode::BytecodeInstr {
    use crate::middle::bytecode::BytecodeInstr;
    let func = module
        .functions
        .first()
        .expect("roundtrip module should have one function");
    let mut found: Option<BytecodeInstr> = None;
    for instr in &func.instructions {
        if matches!(instr, BytecodeInstr::UnaryOp { .. }) {
            found = Some(instr.clone());
            break;
        }
    }
    found.expect("decoded instructions should contain a UnaryOp")
}

// -- 往返保真（主锚）--

/// 主锚（RFC-039 D33 / issue #562）：IR `Instruction::Not` 经编码 → .42
/// 序列化往返 → 解码后，`op` 字段必须保真为 `UnaryOp::Not`。
///
/// 缺陷形态：Not 与 Neg 同发 `I64_NEG`，解码一律还原 Neg——往返把逻辑非
/// 改写为取负，`UnaryOp::Not` 变体在真实管线中成死形态。修复后 Not 走
/// 专用指令码 `I64_NOT`，解码还原 Not。
#[test]
fn test_not_instruction_roundtrip_decode_preserves_not_op() {
    // Arrange & Act
    let module = generate_roundtrip_module(true);

    // Assert
    let instr = first_unary_instr(&module);
    assert!(
        matches!(
            instr,
            crate::middle::bytecode::BytecodeInstr::UnaryOp {
                op: crate::middle::bytecode::UnaryOp::Not,
                ..
            }
        ),
        "IR Not 经编码/序列化/解码往返后必须还原为 UnaryOp::Not，实际 {instr:?}"
    );
}

/// 对照锚：IR `Instruction::Neg` 编码为 `I64_NEG`、解码还原 `UnaryOp::Neg`
/// ——修复只换 Not 的码，Neg 全链行为逐字不变（修前修后都绿）。
#[test]
fn test_neg_instruction_roundtrip_decode_preserves_neg_op() {
    // Arrange & Act
    let module = generate_roundtrip_module(false);

    // Assert
    let instr = first_unary_instr(&module);
    assert!(
        matches!(
            instr,
            crate::middle::bytecode::BytecodeInstr::UnaryOp {
                op: crate::middle::bytecode::UnaryOp::Neg,
                ..
            }
        ),
        "IR Neg 经编码/序列化/解码往返后必须还原为 UnaryOp::Neg，实际 {instr:?}"
    );
}

/// 编码级锚（D33）：Not 的编码 opcode 必须是专用码 `I64_NOT` 且不得与
/// `I64_NEG` 同码。
#[test]
fn test_not_instruction_encodes_to_dedicated_opcode() {
    // Arrange
    let module = unary_module(true);
    let mut ctx = CodegenContext::new(module);
    let file = ctx.generate().expect("codegen should succeed");

    // Act：取编码层 main 的全部指令 opcode
    let func = file
        .code_section
        .functions
        .first()
        .expect("encoded file should have one function");
    let opcodes: Vec<u8> = func.instructions.iter().map(|i| i.opcode).collect();

    // Assert
    assert!(
        opcodes.contains(&opcode::I64_NOT),
        "IR Not 必须编码为 I64_NOT({:#x})，实际指令流 {opcodes:#x?}",
        opcode::I64_NOT
    );
    assert!(
        !opcodes.contains(&opcode::I64_NEG),
        "IR Not 不得再与 Neg 同码 I64_NEG({:#x})，实际指令流 {opcodes:#x?}",
        opcode::I64_NEG
    );
}

// -- 端到端语义（执行面钉住）--

/// 端到端锚：IR `Not(Bool(true))` 经 .42 序列化往返 + 解码 + 解释器执行，
/// 全局槽位 0 记录 `Bool(false)`（逻辑非语义；语言规范 `!` 限定 Bool 域）。
///
/// 该形态在缺陷存在时也答对（解释器 Neg-on-Bool 补偿臂）——本锚钉住
/// 「修复后端到端语义不变」，与往返保真锚（形态面）互补。
#[test]
fn test_not_bool_executes_logically_through_serialized_module() {
    // Arrange
    let module = generate_roundtrip_module(true);
    let mut interp = crate::backends::interpreter::Interpreter::new();

    // Act
    interp
        .execute_module(&module)
        .expect("execute_module should succeed");

    // Assert
    match interp.global_slot(0) {
        Some(crate::backends::common::RuntimeValue::Bool(false)) => {}
        other => panic!("!true 经 .42 往返执行后槽位应为 Bool(false)，实际 {other:?}"),
    }
}
