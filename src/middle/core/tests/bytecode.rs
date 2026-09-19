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
/// 为何不用「重编码比对」：编码方向只存在于 `translator`（IR → 字节码），
/// `BytecodeInstr` **没有**回编码方法，故无法从解出的指令再编码回来对照。
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

    // dst=0x11AA, src=0x22BB——两值不同，dst/src 写反即被抓
    match decode_one(opcode::RC_NEW, vec![0xAA, 0x11, 0xBB, 0x22]) {
        BytecodeInstr::RcNew { dst, src } => {
            assert_eq!(dst, Reg(0x11AA), "RcNew.dst 位置错误（可能 dst/src 写反）");
            assert_eq!(src, Reg(0x22BB), "RcNew.src 位置错误");
        }
        other => panic!("应为 RcNew，实际 {other:?}"),
    }

    // ArcDrop: 单字段 src
    match decode_one(opcode::ARC_DROP, vec![0xCD, 0xAB]) {
        BytecodeInstr::ArcDrop { src } => assert_eq!(src, Reg(0xABCD), "ArcDrop.src 位置错误"),
        other => panic!("应为 ArcDrop，实际 {other:?}"),
    }

    // StringConcat: dst + str1 + str2（三字段，顺序易错）
    match decode_one(
        opcode::STRING_CONCAT,
        vec![0x01, 0x00, 0x02, 0x00, 0x03, 0x00],
    ) {
        BytecodeInstr::StringConcat { dst, str1, str2 } => {
            assert_eq!(dst, Reg(1), "StringConcat.dst 位置错误");
            assert_eq!(str1, Reg(2), "StringConcat.str1 位置错误");
            assert_eq!(str2, Reg(3), "StringConcat.str2 位置错误");
        }
        other => panic!("应为 StringConcat，实际 {other:?}"),
    }

    // Cast: dst + src + target_type_id（u16 类型号）
    match decode_one(opcode::CAST, vec![0x01, 0x00, 0x02, 0x00, 0x34, 0x12]) {
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

    // BoundsCheck: array + index
    match decode_one(opcode::BOUNDS_CHECK, vec![0x07, 0x00, 0x09, 0x00]) {
        BytecodeInstr::BoundsCheck { array, index } => {
            assert_eq!(array, Reg(7), "BoundsCheck.array 位置错误");
            assert_eq!(index, Reg(9), "BoundsCheck.index 位置错误");
        }
        other => panic!("应为 BoundsCheck，实际 {other:?}"),
    }

    // WeakUpgrade: dst + src
    match decode_one(opcode::WEAK_UPGRADE, vec![0x05, 0x00, 0x06, 0x00]) {
        BytecodeInstr::WeakUpgrade { dst, src } => {
            assert_eq!(dst, Reg(5), "WeakUpgrade.dst 位置错误");
            assert_eq!(src, Reg(6), "WeakUpgrade.src 位置错误");
        }
        other => panic!("应为 WeakUpgrade，实际 {other:?}"),
    }
}
