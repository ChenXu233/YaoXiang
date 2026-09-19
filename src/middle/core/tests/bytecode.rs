//! Bytecode IR unit tests (RFC-009 v9 Borrow/Release token opcodes)
//!
//! Covers:
//! - Instruction size calculations
//! - Register and label Display formatting
//! - Borrow/Release opcode and size mapping
//! - Borrow/Release round-trip encode-decode via `build_and_decode`
//! - MonoType::Ref -> IrType::Void conversion

use crate::middle::core::bytecode::{BytecodeInstr, BytecodeModule, Label, Reg};
use std::collections::HashMap;
use crate::middle::core::ir::Type as IrType;
use crate::backends::common::opcode;
use crate::frontend::core::typecheck::MonoType;
use crate::middle::passes::codegen::bytecode::BytecodeInstruction;

// Instruction Size

#[test]
fn test_nop_size_is_one_byte() {
    // Arrange
    let nop = BytecodeInstr::Nop;
    // Act
    let size = nop.size();
    // Assert
    assert_eq!(
        size, 1,
        "Nop instruction should occupy exactly 1 byte (opcode only)"
    );
}

#[test]
fn test_mov_size_is_five_bytes() {
    // Arrange
    let mov = BytecodeInstr::Mov {
        dst: Reg(0),
        src: Reg(1),
    };
    // Act
    let size = mov.size();
    // Assert
    assert_eq!(
        size, 5,
        "Mov instruction should occupy 5 bytes: opcode(1) + dst(2) + src(2)"
    );
}

// Display Formatting

#[test]
fn test_reg_display_format() {
    // Arrange / Act / Assert
    assert_eq!(
        format!("{}", Reg(0)),
        "r0",
        "Reg(0) should display as \"r0\""
    );
    assert_eq!(
        format!("{}", Reg(15)),
        "r15",
        "Reg(15) should display as \"r15\""
    );
}

#[test]
fn test_label_display_format() {
    // Arrange / Act / Assert
    assert_eq!(
        format!("{}", Label(0)),
        "L0",
        "Label(0) should display as \"L0\""
    );
    assert_eq!(
        format!("{}", Label(10)),
        "L10",
        "Label(10) should display as \"L10\""
    );
}

// Borrow/Release Opcode Mapping

#[test]
fn test_borrow_immutable_opcode_is_borrow() {
    // Arrange
    let instr = BytecodeInstr::Borrow {
        dst: Reg(1),
        src: Reg(2),
        mutable: false,
    };
    // Act
    let opcode = instr.opcode();
    // Assert
    assert_eq!(
        opcode,
        opcode::BORROW,
        "Immutable Borrow should map to opcode::BORROW"
    );
}

#[test]
fn test_borrow_mutable_opcode_is_borrow() {
    // Arrange
    let instr = BytecodeInstr::Borrow {
        dst: Reg(1),
        src: Reg(2),
        mutable: true,
    };
    // Act
    let opcode = instr.opcode();
    // Assert
    assert_eq!(
        opcode,
        opcode::BORROW,
        "Mutable Borrow should map to opcode::BORROW"
    );
}

#[test]
fn test_borrow_size_is_six_bytes() {
    // Arrange
    let instr = BytecodeInstr::Borrow {
        dst: Reg(1),
        src: Reg(2),
        mutable: true,
    };
    // Act
    let size = instr.size();
    // Assert: opcode(1) + dst(2) + src(2) + mutable(1) = 6
    assert_eq!(
        size, 6,
        "Borrow instruction should occupy 6 bytes: opcode(1)+dst(2)+src(2)+mutable(1)"
    );
}

#[test]
fn test_release_opcode_is_release() {
    // Arrange
    let instr = BytecodeInstr::Release { src: Reg(3) };
    // Act
    let opcode = instr.opcode();
    // Assert
    assert_eq!(
        opcode,
        opcode::RELEASE,
        "Release should map to opcode::RELEASE"
    );
}

#[test]
fn test_release_size_is_three_bytes() {
    // Arrange
    let instr = BytecodeInstr::Release { src: Reg(3) };
    // Act
    let size = instr.size();
    // Assert: opcode(1) + src(2) = 3
    assert_eq!(
        size, 3,
        "Release instruction should occupy 3 bytes: opcode(1)+src(2)"
    );
}

// Borrow/Release Round-trip Tests

/// Helper: build a minimal BytecodeFile with one function containing
/// the given raw BytecodeInstructions, then decode via `From<BytecodeFile>`.
fn build_and_decode(instrs: Vec<BytecodeInstruction>) -> BytecodeModule {
    use crate::middle::passes::codegen::bytecode as bcfile;
    let func = bcfile::FunctionCode {
        name: "test_fn".to_string(),
        params: vec![],
        return_type: crate::frontend::core::typecheck::MonoType::Void,
        instructions: instrs,
        local_count: 0,
        local_names: HashMap::new(),
        debug_map: std::collections::HashMap::new(),
    };
    let file = bcfile::BytecodeFile {
        header: bcfile::FileHeader::default(),
        type_table: vec![],
        const_pool: vec![],
        code_section: bcfile::CodeSection {
            functions: vec![func],
        },
        vtables: vec![],
        debug_section: None,
    };
    BytecodeModule::from(file)
}

/// Helper: assert that the single decoded instruction is `Borrow`
/// with the expected `dst`, `src`, and `mutable` fields.
fn assert_borrow_instr(
    instr: &BytecodeInstr,
    expected_dst: Reg,
    expected_src: Reg,
    expected_mutable: bool,
) {
    match instr {
        BytecodeInstr::Borrow { dst, src, mutable } => {
            assert_eq!(
                *dst, expected_dst,
                "Borrow dst should be {:?}",
                expected_dst
            );
            assert_eq!(
                *src, expected_src,
                "Borrow src should be {:?}",
                expected_src
            );
            assert_eq!(
                *mutable, expected_mutable,
                "Borrow mutable should be {}",
                expected_mutable
            );
        }
        other => panic!("Expected Borrow instruction, got {:?}", other),
    }
}

/// Helper: assert that the single decoded instruction is `Release`
/// with the expected `src` field.
fn assert_release_instr(
    instr: &BytecodeInstr,
    expected_src: Reg,
) {
    match instr {
        BytecodeInstr::Release { src } => {
            assert_eq!(
                *src, expected_src,
                "Release src should be {:?}",
                expected_src
            );
        }
        other => panic!("Expected Release instruction, got {:?}", other),
    }
}

#[test]
fn test_borrow_roundtrip_immutable() {
    // Arrange: encode Borrow dst=1, src=2, mutable=false
    let encoded = BytecodeInstruction::new(
        opcode::BORROW,
        vec![1, 0, 2, 0, 0], // dst=1 LE, src=2 LE, mutable=false
    );
    // Act
    let module = build_and_decode(vec![encoded]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        module.functions.len(),
        1,
        "Module should contain exactly 1 function"
    );
    assert_eq!(
        instrs.len(),
        1,
        "Function should contain exactly 1 instruction"
    );
    assert_borrow_instr(&instrs[0], Reg(1), Reg(2), false);
}

#[test]
fn test_borrow_roundtrip_mutable() {
    // Arrange: encode Borrow dst=1, src=2, mutable=true
    let encoded = BytecodeInstruction::new(
        opcode::BORROW,
        vec![1, 0, 2, 0, 1], // dst=1 LE, src=2 LE, mutable=true
    );
    // Act
    let module = build_and_decode(vec![encoded]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        instrs.len(),
        1,
        "Function should contain exactly 1 instruction"
    );
    assert_borrow_instr(&instrs[0], Reg(1), Reg(2), true);
}

#[test]
fn test_release_roundtrip() {
    // Arrange: encode Release src=3
    let encoded = BytecodeInstruction::new(
        opcode::RELEASE,
        vec![3, 0], // src=3 LE
    );
    // Act
    let module = build_and_decode(vec![encoded]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        instrs.len(),
        1,
        "Function should contain exactly 1 instruction"
    );
    assert_release_instr(&instrs[0], Reg(3));
}

#[test]
fn test_borrow_release_combined_roundtrip() {
    // Arrange: Borrow(dst=5, src=10, mutable=true) followed by Release(src=5)
    let borrow_instr = BytecodeInstruction::new(
        opcode::BORROW,
        vec![5, 0, 10, 0, 1], // dst=5, src=10, mutable=true
    );
    let release_instr = BytecodeInstruction::new(
        opcode::RELEASE,
        vec![5, 0], // src=5
    );
    // Act
    let module = build_and_decode(vec![borrow_instr, release_instr]);
    let instrs = &module.functions[0].instructions;
    // Assert
    assert_eq!(
        instrs.len(),
        2,
        "Function should contain exactly 2 instructions"
    );
    assert_borrow_instr(&instrs[0], Reg(5), Reg(10), true);
    assert_release_instr(&instrs[1], Reg(5));
}

// MonoType::Ref -> IrType Conversion

#[test]
fn test_ref_type_maps_to_void_ir_type() {
    // Arrange
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };
    // Act
    let ir_type: IrType = ref_ty.into();
    // Assert: Ref is ZST, should map to Void
    assert!(
        matches!(ir_type, IrType::Void),
        "Immutable Ref<i64> should map to IrType::Void (ZST has no runtime repr)"
    );
}

#[test]
fn test_ref_type_mutable_maps_to_void_ir_type() {
    // Arrange
    let ref_ty = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::make_string()),
    };
    // Act
    let ir_type: IrType = ref_ty.into();
    // Assert: Ref is ZST regardless of mutability
    assert!(
        matches!(ir_type, IrType::Void),
        "Mutable Ref<String> should map to IrType::Void (ZST has no runtime repr)"
    );
}

// ── 编解码漂移护栏 ────────────────────────────────────────────

/// 断言「编码器能产出的每个 opcode，解码器都有对应分支」。
///
/// 背景：解码器曾只覆盖 57/83 个 opcode，其余落到静默兜底变 `Nop`——
/// `RC_NEW`(0x89) 即因此让弱引用功能静默失效。该兜底现已改为 panic，
/// 本测试把「漂移」提前到编译/测试期暴露，而非运行到某个用例才炸。
///
/// 做法：遍历 `BytecodeInstr::opcode()` 的映射（编码方向），对每个 opcode
/// 构造一条最小指令再解码，断言解出的**指令种类与编码前一致**（不是 Nop）。
#[test]
fn test_every_opcode_roundtrips_not_silently_nop() {
    use crate::backends::common::opcode;
    use crate::middle::bytecode::{BinaryOp, BytecodeInstr, CompareOp, Label, Reg};

    // (指令, 期望的 opcode) —— 覆盖解码器现支持的全部指令种类
    let cases: Vec<(BytecodeInstr, u8)> = vec![
        (BytecodeInstr::Nop, opcode::NOP),
        (
            BytecodeInstr::Mov {
                dst: Reg(1),
                src: Reg(2),
            },
            opcode::MOV,
        ),
        (
            BytecodeInstr::LoadConst {
                dst: Reg(1),
                const_idx: 0,
            },
            opcode::LOAD_CONST,
        ),
        (
            BytecodeInstr::LoadLocal {
                dst: Reg(1),
                local_idx: 0,
            },
            opcode::LOAD_LOCAL,
        ),
        (
            BytecodeInstr::StoreLocal {
                local_idx: 0,
                src: Reg(1),
            },
            opcode::STORE_LOCAL,
        ),
        (
            BytecodeInstr::LoadArg {
                dst: Reg(1),
                arg_idx: 0,
            },
            opcode::LOAD_ARG,
        ),
        (
            BytecodeInstr::LoadGlobal {
                dst: Reg(1),
                global_idx: 0,
            },
            opcode::LOAD_GLOBAL,
        ),
        (
            BytecodeInstr::StoreGlobal {
                global_idx: 0,
                src: Reg(1),
            },
            opcode::STORE_GLOBAL,
        ),
        (
            BytecodeInstr::BinaryOp {
                dst: Reg(1),
                lhs: Reg(2),
                rhs: Reg(3),
                op: BinaryOp::Add,
            },
            opcode::I64_ADD,
        ),
        (
            BytecodeInstr::Compare {
                dst: Reg(1),
                lhs: Reg(2),
                rhs: Reg(3),
                cmp: CompareOp::Eq,
            },
            opcode::I64_EQ,
        ),
        (BytecodeInstr::Return, opcode::RETURN),
        (
            BytecodeInstr::ReturnValue { value: Reg(0) },
            opcode::RETURN_VALUE,
        ),
        (BytecodeInstr::Jmp { target: Label(0) }, opcode::JMP),
        (
            BytecodeInstr::JmpIf {
                cond: Reg(0),
                target: Label(0),
            },
            opcode::JMP_IF,
        ),
        (
            BytecodeInstr::JmpIfNot {
                cond: Reg(0),
                target: Label(0),
            },
            opcode::JMP_IF_NOT,
        ),
        // 引用计数族（本次修复的重点）
        (
            BytecodeInstr::ArcNew {
                dst: Reg(1),
                src: Reg(2),
            },
            opcode::ARC_NEW,
        ),
        (
            BytecodeInstr::RcNew {
                dst: Reg(1),
                src: Reg(2),
            },
            opcode::RC_NEW,
        ),
        (
            BytecodeInstr::ArcClone {
                dst: Reg(1),
                src: Reg(2),
            },
            opcode::ARC_CLONE,
        ),
        (BytecodeInstr::ArcDrop { src: Reg(1) }, opcode::ARC_DROP),
        (
            BytecodeInstr::WeakNew {
                dst: Reg(1),
                src: Reg(2),
            },
            opcode::WEAK_NEW,
        ),
        (
            BytecodeInstr::WeakUpgrade {
                dst: Reg(1),
                src: Reg(2),
            },
            opcode::WEAK_UPGRADE,
        ),
        (
            BytecodeInstr::Borrow {
                dst: Reg(1),
                src: Reg(2),
                mutable: false,
            },
            opcode::BORROW,
        ),
        (BytecodeInstr::Release { src: Reg(1) }, opcode::RELEASE),
        // 内存与类型
        (BytecodeInstr::Drop { value: Reg(0) }, opcode::DROP),
        (
            BytecodeInstr::StackAlloc {
                dst: Reg(0),
                size: 1,
            },
            opcode::STACK_ALLOC,
        ),
        (
            BytecodeInstr::HeapAlloc {
                dst: Reg(0),
                type_id: 0,
            },
            opcode::HEAP_ALLOC,
        ),
        (
            BytecodeInstr::CloseUpvalue { src: Reg(0) },
            opcode::CLOSE_UPVALUE,
        ),
        (
            BytecodeInstr::Cast {
                dst: Reg(0),
                src: Reg(1),
                target_type_id: 0,
            },
            opcode::CAST,
        ),
        (
            BytecodeInstr::TypeCheck {
                value: Reg(0),
                type_id: 0,
            },
            opcode::TYPE_CHECK,
        ),
        (
            BytecodeInstr::TypeOf {
                dst: Reg(0),
                src: Reg(1),
            },
            opcode::TYPE_OF,
        ),
        (
            BytecodeInstr::BoundsCheck {
                array: Reg(0),
                index: Reg(1),
            },
            opcode::BOUNDS_CHECK,
        ),
        // 字符串
        (
            BytecodeInstr::StringLength {
                dst: Reg(0),
                src: Reg(1),
            },
            opcode::STRING_LENGTH,
        ),
        (
            BytecodeInstr::StringConcat {
                dst: Reg(0),
                str1: Reg(1),
                str2: Reg(2),
            },
            opcode::STRING_CONCAT,
        ),
        (
            BytecodeInstr::StringEqual {
                dst: Reg(0),
                str1: Reg(1),
                str2: Reg(2),
            },
            opcode::STRING_EQUAL,
        ),
        (
            BytecodeInstr::StringGetChar {
                dst: Reg(0),
                src: Reg(1),
                index: Reg(2),
            },
            opcode::STRING_GET_CHAR,
        ),
        (
            BytecodeInstr::StringFromInt {
                dst: Reg(0),
                src: Reg(1),
            },
            opcode::STRING_FROM_INT,
        ),
        (
            BytecodeInstr::StringFromFloat {
                dst: Reg(0),
                src: Reg(1),
            },
            opcode::STRING_FROM_FLOAT,
        ),
        // 异常
        (BytecodeInstr::TryEnd, opcode::TRY_END),
        (BytecodeInstr::Throw { error: Reg(0) }, opcode::THROW),
    ];

    for (instr, expected_opcode) in cases {
        // Arrange: 按 operand_len 表填充合法长度的操作数
        let opcode_val = instr.opcode();
        assert_eq!(
            opcode_val, expected_opcode,
            "编码方向 opcode 不符：{instr:?}"
        );
        let operand_len = instr.size();
        // 用非零且可区分的字节填充：全填 0 无法发现「字段顺序写反」这类
        // 错位——各字段都解出 0 时看似正确。此处按位置递增，
        // 使每个字段解出的值互不相同，错位即被下方断言捕获。
        let operands: Vec<u8> = (0..operand_len)
            .map(|i| (i as u8).wrapping_mul(3).wrapping_add(1))
            .collect();
        let encoded = BytecodeInstruction::new(opcode_val, operands);

        // Act
        let module = build_and_decode(vec![encoded]);
        let decoded = &module.functions[0].instructions;

        // Assert: 解出的指令种类与编码前一致（不得是静默 Nop 兜底）
        assert_eq!(
            decoded.len(),
            1,
            "{instr:?} 解码后指令数应为 1，实际 {}",
            decoded.len()
        );
        assert_eq!(
            std::mem::discriminant(&decoded[0]),
            std::mem::discriminant(&instr),
            "opcode 0x{opcode_val:02X} 解码后指令种类变了——编码器与解码器漂移。\
             编码前 {instr:?}，解码后 {:?}",
            decoded[0]
        );
    }
}

/// 字段级校验：解码器把各字段放在正确位置。
///
/// 上方的种类比对无法发现「字段顺序写反」——例如 `RcNew { dst, src }` 若
/// 按 `src, dst` 解码，种类仍是 `RcNew` 但值已互换。本测试用互不相同的
/// 操作数值逐字段断言。
///
/// **字节布局以编码器为准**（`translator.rs` 的 `translate_*`）：
/// 编码器经 `to_reg` 产出寄存器号，上限 255（`operand.rs` 的
/// register_overflow 检查），故**寄存器恒为 1 字节**；类型号/长度等
/// 非寄存器字段才是 2 字节小端。
/// 注：`BytecodeInstr::size()` 的注释（如 "dst(2)"）与实际编码不符，
/// **不可**作为字节布局依据——曾据此实现导致全部字段错位。
#[test]
fn test_decode_places_fields_in_correct_positions() {
    use crate::backends::common::opcode;
    use crate::middle::bytecode::{BytecodeInstr, Reg};

    // 辅助：按字节向量解码出单条指令
    fn decode_one(
        op: u8,
        operands: Vec<u8>,
    ) -> BytecodeInstr {
        let module = build_and_decode(vec![BytecodeInstruction::new(op, operands)]);
        module.functions[0].instructions[0].clone()
    }

    // RcNew: dst(1) + src(1)——两值不同，dst/src 写反即被抓
    match decode_one(opcode::RC_NEW, vec![0x11, 0x22]) {
        BytecodeInstr::RcNew { dst, src } => {
            assert_eq!(dst, Reg(0x11), "RcNew.dst 位置错误（可能 dst/src 写反）");
            assert_eq!(src, Reg(0x22), "RcNew.src 位置错误");
        }
        other => panic!("应为 RcNew，实际 {other:?}"),
    }

    // ArcDrop: 单字段 src（1 字节）
    match decode_one(opcode::ARC_DROP, vec![0xCD]) {
        BytecodeInstr::ArcDrop { src } => {
            assert_eq!(src, Reg(0xCD), "ArcDrop.src 位置错误")
        }
        other => panic!("应为 ArcDrop，实际 {other:?}"),
    }

    // ArcNew: dst(1) + src(1)
    match decode_one(opcode::ARC_NEW, vec![0x05, 0x06]) {
        BytecodeInstr::ArcNew { dst, src } => {
            assert_eq!(dst, Reg(5), "ArcNew.dst 位置错误");
            assert_eq!(src, Reg(6), "ArcNew.src 位置错误");
        }
        other => panic!("应为 ArcNew，实际 {other:?}"),
    }

    // WeakUpgrade: dst(1) + src(1)
    match decode_one(opcode::WEAK_UPGRADE, vec![0x07, 0x08]) {
        BytecodeInstr::WeakUpgrade { dst, src } => {
            assert_eq!(dst, Reg(7), "WeakUpgrade.dst 位置错误");
            assert_eq!(src, Reg(8), "WeakUpgrade.src 位置错误");
        }
        other => panic!("应为 WeakUpgrade，实际 {other:?}"),
    }

    // Drop: value(1)
    match decode_one(opcode::DROP, vec![0x0A]) {
        BytecodeInstr::Drop { value } => {
            assert_eq!(value, Reg(10), "Drop.value 位置错误")
        }
        other => panic!("应为 Drop，实际 {other:?}"),
    }

    // Cast: dst(1) + src(1) + target_type_id(2, 小端)
    match decode_one(opcode::CAST, vec![0x01, 0x02, 0x34, 0x12]) {
        BytecodeInstr::Cast {
            dst,
            src,
            target_type_id,
        } => {
            assert_eq!(dst, Reg(1), "Cast.dst 位置错误");
            assert_eq!(src, Reg(2), "Cast.src 位置错误");
            assert_eq!(target_type_id, 0x1234, "Cast.target_type_id 位置错误");
        }
        other => panic!("应为 Cast，实际 {other:?}"),
    }

    // HeapAlloc: dst(1) + type_id(2, 小端)
    match decode_one(opcode::HEAP_ALLOC, vec![0x03, 0x78, 0x56]) {
        BytecodeInstr::HeapAlloc { dst, type_id } => {
            assert_eq!(dst, Reg(3), "HeapAlloc.dst 位置错误");
            assert_eq!(type_id, 0x5678, "HeapAlloc.type_id 位置错误");
        }
        other => panic!("应为 HeapAlloc，实际 {other:?}"),
    }

    // BoundsCheck: array(1) + index(1)
    match decode_one(opcode::BOUNDS_CHECK, vec![0x07, 0x09]) {
        BytecodeInstr::BoundsCheck { array, index } => {
            assert_eq!(array, Reg(7), "BoundsCheck.array 位置错误");
            assert_eq!(index, Reg(9), "BoundsCheck.index 位置错误");
        }
        other => panic!("应为 BoundsCheck，实际 {other:?}"),
    }

    // StringConcat: dst(1) + str1(1) + str2(1)
    match decode_one(opcode::STRING_CONCAT, vec![0x01, 0x02, 0x03]) {
        BytecodeInstr::StringConcat { dst, str1, str2 } => {
            assert_eq!(dst, Reg(1), "StringConcat.dst 位置错误");
            assert_eq!(str1, Reg(2), "StringConcat.str1 位置错误");
            assert_eq!(str2, Reg(3), "StringConcat.str2 位置错误");
        }
        other => panic!("应为 StringConcat，实际 {other:?}"),
    }
}

/// 解码覆盖面哨兵：`opcode.rs` 定义的每个 opcode 都应有解码分支。
///
/// 背景：解码器曾只覆盖 57/83 个 opcode，其余落到静默兜底变 `Nop`——
/// `RC_NEW`(0x89) 即因此让弱引用功能静默失效（见 #D2）。兜底改为 panic 后，
/// 遗漏会表现为运行到该指令才崩，本测试把它提前到测试期。
///
/// **已知例外**：`SWITCH` 无编码器（`translator.rs` 不产出）、`ir_gen` 也不构造
/// 对应的 `Instruction::Switch`——`match` 语句编译为 `I64Eq` + `JmpIfNot` 链
/// （实测 dump 确认）。故它既无编码格式也无解码需求，列入白名单。
/// 白名单外的任何遗漏都会让本测试失败。
#[test]
fn test_all_opcodes_have_decode_branch() {
    // 从源码读取 opcode.rs 的定义与 bytecode.rs 的解码分支，
    // 断言前者是后者的子集（白名单除外）。
    let opcode_src = include_str!("../../../backends/common/opcode.rs");
    let decoder_src = include_str!("../bytecode.rs");

    let defined: std::collections::BTreeSet<String> = opcode_src
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub const "))
        .filter_map(|rest| rest.split_once(':'))
        .map(|(name, _)| name.trim().to_string())
        .collect();

    let decoded: std::collections::BTreeSet<String> = decoder_src
        .match_indices("opcode::")
        .filter_map(|(i, _)| {
            let rest = &decoder_src[i + "opcode::".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            // 只认「匹配臂」：后随 ` =>`
            if rest[name.len()..].trim_start().starts_with("=>") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    // 已知死路径：无编码器亦无 IR 产出
    const WHITELIST: &[&str] = &["SWITCH"];

    let missing: Vec<&String> = defined
        .iter()
        .filter(|op| !decoded.contains(*op) && !WHITELIST.contains(&op.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "以下 opcode 已定义但无解码分支（会落到 panic 兜底，应在 bytecode.rs 补分支或加入白名单）：{missing:?}\n\
         已定义 {} 个，有解码分支 {} 个",
        defined.len(),
        decoded.len()
    );
}

/// 编码器覆盖面哨兵：记录**无 translator 编码器**的 opcode 全集。
///
/// 与解码覆盖面哨兵互补——那个查「定义 vs 解码」，本测试查「定义 vs 编码」。
///
/// 这些 opcode 的解码器与执行器大多已实现，但 `translator.rs` 不产出它们
/// （`ir_gen` 前端不构造对应 IR），故属**预留路径**而非缺陷。本测试的作用：
/// 1. 把「未启用路径」的全集固化为断言——新增此类 opcode 时会被发现
/// 2. 若将来接通某个前端，该 opcode 应从此表**移除**（表示已启用），
///    使这张表始终是「当前未启用」的准确快照
///
/// 判定方式：扫描 `translator.rs` 是否出现 `opcode::NAME`。这是近似判据，
/// 只会在「误报为已编码」时使断言更严（要求移除表项），不会漏报。
#[test]
fn test_opcodes_without_encoder_are_documented() {
    // Arrange: 读取 opcode 定义与编码器源码
    let opcode_src = include_str!("../../../backends/common/opcode.rs");
    let translator_src = include_str!("../../passes/codegen/translator.rs");

    let defined: std::collections::BTreeSet<String> = opcode_src
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub const "))
        .filter_map(|rest| rest.split_once(':'))
        .map(|(name, _)| name.trim().to_string())
        .collect();

    // Act: 找出 translator.rs 中未出现的 opcode
    let unencoded: std::collections::BTreeSet<String> = defined
        .iter()
        .filter(|name| !translator_src.contains(&format!("opcode::{name}")))
        .cloned()
        .collect();

    // Assert: 与下方快照完全一致。
    // 每个条目都经核查确认「解码/执行已实现、前端未接通」，详见 plan 文档 D8。
    const EXPECTED_UNENCODED: &[&str] = &[
        // 借用令牌（RFC-009 v9）：解码器 + 执行器 + 往返测试齐备
        "BORROW",
        "RELEASE",
        // 越界检查：有解码 + 执行，ir_gen 不产出
        "BOUNDS_CHECK",
        // 异常族（throw/try）：有解码 + 执行，ir_gen 不产出
        "THROW",
        "TRY_BEGIN",
        "TRY_END",
        // 弱引用：有解码 + 执行
        "WEAK_NEW",
        "WEAK_UPGRADE",
        // 字符串相等：有解码 + 执行，当前走别处
        "STRING_EQUAL",
        // 运行时类型查询：有解码 + 执行
        "TYPE_OF",
        // 多路分支：**无解码器**（唯一一个），已在解码哨兵白名单
        "SWITCH",
        // 占位 opcode：仅在显示名表出现，非真实指令
        "LABEL",
    ];

    let expected: std::collections::BTreeSet<String> =
        EXPECTED_UNENCODED.iter().map(|s| s.to_string()).collect();

    let newly_unencoded: Vec<&String> = unencoded.difference(&expected).collect();
    let newly_encoded: Vec<&String> = expected.difference(&unencoded).collect();

    assert!(
        newly_unencoded.is_empty(),
        "以下 opcode 无 translator 编码器但未登记——若是新预留路径请加入          EXPECTED_UNENCODED 并核查解码/执行是否齐备；若是缺陷请补编码器：{newly_unencoded:?}"
    );
    assert!(
        newly_encoded.is_empty(),
        "以下 opcode 已出现在 translator.rs 中（疑似已接通前端），请从          EXPECTED_UNENCODED 移除，使本表保持为「当前未启用」的准确快照：{newly_encoded:?}"
    );
}
