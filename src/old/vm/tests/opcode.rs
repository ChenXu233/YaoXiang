//! TypedOpcode 单元测试
//!
//! 测试字节码操作码的定义和行为

use crate::vm::opcode::TypedOpcode;

#[cfg(test)]
mod name_tests {
    use super::*;

    /// 测试 TypedOpcode 名称
    #[test]
    fn test_opcode_name_nop() {
        assert_eq!(TypedOpcode::Nop.name(), "Nop");
    }

    #[test]
    fn test_opcode_name_return() {
        assert_eq!(TypedOpcode::Return.name(), "Return");
    }

    #[test]
    fn test_opcode_name_i64_add() {
        assert_eq!(TypedOpcode::I64Add.name(), "I64Add");
    }

    #[test]
    fn test_opcode_name_f64_mul() {
        assert_eq!(TypedOpcode::F64Mul.name(), "F64Mul");
    }

    #[test]
    fn test_opcode_name_invalid() {
        assert_eq!(TypedOpcode::Invalid.name(), "Invalid");
    }

    /// 测试所有主要指令名称
    #[test]
    fn test_all_opcode_names() {
        let opcodes = [
            TypedOpcode::Nop,
            TypedOpcode::Return,
            TypedOpcode::ReturnValue,
            TypedOpcode::Jmp,
            TypedOpcode::JmpIf,
            TypedOpcode::JmpIfNot,
            TypedOpcode::Switch,
            TypedOpcode::LoopStart,
            TypedOpcode::LoopInc,
            TypedOpcode::TailCall,
            TypedOpcode::Yield,
            TypedOpcode::Label,
            TypedOpcode::Mov,
            TypedOpcode::LoadConst,
            TypedOpcode::LoadLocal,
            TypedOpcode::StoreLocal,
            TypedOpcode::LoadArg,
            TypedOpcode::I64Add,
            TypedOpcode::I64Sub,
            TypedOpcode::I64Mul,
            TypedOpcode::I64Div,
            TypedOpcode::I64Rem,
            TypedOpcode::I64And,
            TypedOpcode::I64Or,
            TypedOpcode::I64Xor,
            TypedOpcode::I64Shl,
            TypedOpcode::I64Sar,
            TypedOpcode::I64Shr,
            TypedOpcode::I64Neg,
            TypedOpcode::I64Load,
            TypedOpcode::I64Store,
            TypedOpcode::I64Const,
            TypedOpcode::I32Add,
            TypedOpcode::F64Add,
            TypedOpcode::F64Sub,
            TypedOpcode::F64Mul,
            TypedOpcode::F64Div,
            TypedOpcode::F64Sqrt,
            TypedOpcode::F32Add,
            TypedOpcode::F32Mul,
            TypedOpcode::I64Eq,
            TypedOpcode::I64Ne,
            TypedOpcode::I64Lt,
            TypedOpcode::I64Le,
            TypedOpcode::I64Gt,
            TypedOpcode::I64Ge,
            TypedOpcode::F64Eq,
            TypedOpcode::StackAlloc,
            TypedOpcode::HeapAlloc,
            TypedOpcode::GetField,
            TypedOpcode::SetField,
            TypedOpcode::LoadElement,
            TypedOpcode::StoreElement,
            TypedOpcode::NewListWithCap,
            TypedOpcode::CallStatic,
            TypedOpcode::CallVirt,
            TypedOpcode::CallDyn,
            TypedOpcode::MakeClosure,
            TypedOpcode::LoadUpvalue,
            TypedOpcode::StoreUpvalue,
            TypedOpcode::CloseUpvalue,
            TypedOpcode::StringLength,
            TypedOpcode::StringConcat,
            TypedOpcode::StringEqual,
            TypedOpcode::TryBegin,
            TypedOpcode::TryEnd,
            TypedOpcode::Throw,
            TypedOpcode::Rethrow,
            TypedOpcode::BoundsCheck,
            TypedOpcode::TypeCheck,
            TypedOpcode::Cast,
            TypedOpcode::TypeOf,
            TypedOpcode::Custom0,
            TypedOpcode::Invalid,
        ];

        for opcode in opcodes {
            let name = opcode.name();
            assert!(!name.is_empty(), "Opcode {:?} has empty name", opcode);
        }
    }
}

#[cfg(test)]
mod numeric_op_tests {
    use super::*;

    /// 测试 is_numeric_op
    #[test]
    fn test_is_numeric_op_i64() {
        assert!(TypedOpcode::I64Add.is_numeric_op());
        assert!(TypedOpcode::I64Sub.is_numeric_op());
        assert!(TypedOpcode::I64Mul.is_numeric_op());
        assert!(TypedOpcode::I64Div.is_numeric_op());
    }

    #[test]
    fn test_is_numeric_op_f64() {
        assert!(TypedOpcode::F64Add.is_numeric_op());
        assert!(TypedOpcode::F64Mul.is_numeric_op());
        assert!(!TypedOpcode::Nop.is_numeric_op());
    }

    #[test]
    fn test_is_numeric_op_false() {
        assert!(!TypedOpcode::Nop.is_numeric_op());
        assert!(!TypedOpcode::Return.is_numeric_op());
        assert!(!TypedOpcode::LoadConst.is_numeric_op());
    }
}

#[cfg(test)]
mod integer_op_tests {
    use super::*;

    /// 测试 is_integer_op
    #[test]
    fn test_is_integer_op() {
        assert!(TypedOpcode::I64Add.is_integer_op());
        assert!(TypedOpcode::I32Mul.is_integer_op());
        assert!(TypedOpcode::I64And.is_integer_op());
        assert!(!TypedOpcode::F64Add.is_integer_op());
    }
}

#[cfg(test)]
mod float_op_tests {
    use super::*;

    /// 测试 is_float_op
    #[test]
    fn test_is_float_op() {
        assert!(TypedOpcode::F64Add.is_float_op());
        assert!(TypedOpcode::F32Mul.is_float_op());
        assert!(TypedOpcode::F64Sqrt.is_float_op());
        assert!(!TypedOpcode::I64Add.is_float_op());
    }
}

#[cfg(test)]
mod load_op_tests {
    use super::*;

    /// 测试 is_load_op
    #[test]
    fn test_is_load_op() {
        assert!(TypedOpcode::LoadConst.is_load_op());
        assert!(TypedOpcode::LoadLocal.is_load_op());
        assert!(TypedOpcode::I64Load.is_load_op());
        assert!(TypedOpcode::LoadElement.is_load_op());
        assert!(!TypedOpcode::Nop.is_load_op());
    }
}

#[cfg(test)]
mod store_op_tests {
    use super::*;

    /// 测试 is_store_op
    #[test]
    fn test_is_store_op() {
        assert!(TypedOpcode::StoreLocal.is_store_op());
        assert!(TypedOpcode::I64Store.is_store_op());
        assert!(TypedOpcode::SetField.is_store_op());
        assert!(!TypedOpcode::Nop.is_store_op());
    }
}

#[cfg(test)]
mod call_op_tests {
    use super::*;

    /// 测试 is_call_op
    #[test]
    fn test_is_call_op() {
        assert!(TypedOpcode::CallStatic.is_call_op());
        assert!(TypedOpcode::CallVirt.is_call_op());
        assert!(TypedOpcode::CallDyn.is_call_op());
        assert!(!TypedOpcode::Nop.is_call_op());
    }
}

#[cfg(test)]
mod return_op_tests {
    use super::*;

    /// 测试 is_return_op
    #[test]
    fn test_is_return_op() {
        assert!(TypedOpcode::Return.is_return_op());
        assert!(TypedOpcode::ReturnValue.is_return_op());
        assert!(TypedOpcode::TailCall.is_return_op());
        assert!(!TypedOpcode::Nop.is_return_op());
    }
}

#[cfg(test)]
mod jump_op_tests {
    use super::*;

    /// 测试 is_jump_op
    #[test]
    fn test_is_jump_op() {
        assert!(TypedOpcode::Jmp.is_jump_op());
        assert!(TypedOpcode::JmpIf.is_jump_op());
        assert!(TypedOpcode::Switch.is_jump_op());
        assert!(!TypedOpcode::Nop.is_jump_op());
    }
}

#[cfg(test)]
mod operand_count_tests {
    use super::*;

    #[test]
    fn test_operand_count_zero() {
        assert_eq!(TypedOpcode::Nop.operand_count(), 0);
        assert_eq!(TypedOpcode::Return.operand_count(), 0);
        assert_eq!(TypedOpcode::Yield.operand_count(), 0);
        assert_eq!(TypedOpcode::Invalid.operand_count(), 0);
    }

    #[test]
    fn test_operand_count_one() {
        assert_eq!(TypedOpcode::ReturnValue.operand_count(), 1);
        assert_eq!(TypedOpcode::Throw.operand_count(), 1);
        assert_eq!(TypedOpcode::TryBegin.operand_count(), 1);
    }

    #[test]
    fn test_operand_count_two() {
        assert_eq!(TypedOpcode::Mov.operand_count(), 2);
        assert_eq!(TypedOpcode::LoadConst.operand_count(), 2);
        assert_eq!(TypedOpcode::I64Neg.operand_count(), 2);
    }

    #[test]
    fn test_operand_count_three() {
        assert_eq!(TypedOpcode::I64Add.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Mul.operand_count(), 3);
        assert_eq!(TypedOpcode::I64Load.operand_count(), 3);
    }

    #[test]
    fn test_operand_count_i64_comparison() {
        // I64 比较指令需要 3 个操作数: dst, lhs, rhs
        assert_eq!(TypedOpcode::I64Eq.operand_count(), 3);
        assert_eq!(TypedOpcode::I64Ne.operand_count(), 3);
        assert_eq!(TypedOpcode::I64Lt.operand_count(), 3);
        assert_eq!(TypedOpcode::I64Le.operand_count(), 3);
        assert_eq!(TypedOpcode::I64Gt.operand_count(), 3);
        assert_eq!(TypedOpcode::I64Ge.operand_count(), 3);
    }

    #[test]
    fn test_operand_count_f64_comparison() {
        // F64 比较指令需要 3 个操作数: dst, lhs, rhs
        assert_eq!(TypedOpcode::F64Eq.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Ne.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Lt.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Le.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Gt.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Ge.operand_count(), 3);
    }

    #[test]
    fn test_operand_count_four() {
        assert_eq!(TypedOpcode::LoadElement.operand_count(), 4);
        assert_eq!(TypedOpcode::StringConcat.operand_count(), 4);
    }

    #[test]
    fn test_operand_count_five() {
        // 函数调用指令需要 5 个操作数: dst, func_id, base_arg_reg, arg_count, (reserved)
        assert_eq!(TypedOpcode::CallStatic.operand_count(), 5);
        assert_eq!(TypedOpcode::CallVirt.operand_count(), 5);
        assert_eq!(TypedOpcode::CallDyn.operand_count(), 5);
    }

    #[test]
    fn test_operand_count_memory() {
        // 内存 Load/Store 指令需要 3 个操作数
        assert_eq!(TypedOpcode::I64Load.operand_count(), 3);
        assert_eq!(TypedOpcode::I32Load.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Load.operand_count(), 3);
        assert_eq!(TypedOpcode::F32Load.operand_count(), 3);

        assert_eq!(TypedOpcode::I64Store.operand_count(), 3);
        assert_eq!(TypedOpcode::I32Store.operand_count(), 3);
        assert_eq!(TypedOpcode::F64Store.operand_count(), 3);
        assert_eq!(TypedOpcode::F32Store.operand_count(), 3);
    }

    #[test]
    fn test_operand_count_jump() {
        // JmpIfNot 需要 2 个操作数: cond_reg (u8), offset (i16)
        assert_eq!(TypedOpcode::JmpIfNot.operand_count(), 2);
        // Switch 需要 3 个操作数: reg (u8), table_idx (u16), default_offset (i16)
        assert_eq!(TypedOpcode::Switch.operand_count(), 3);
    }

    #[test]
    fn test_operand_count_call() {
        // TailCall 需要 4 个操作数: func_id, base_arg, arg_count
        assert_eq!(TypedOpcode::TailCall.operand_count(), 4);
        // MakeClosure 需要 4 个操作数: dst, func_id, env_size, upvalue_count
        assert_eq!(TypedOpcode::MakeClosure.operand_count(), 4);
    }

    #[test]
    fn test_operand_count_control_flow() {
        // Switch 需要 3 个操作数: reg (u8), table_idx (u16), default_offset (i16)
        assert_eq!(TypedOpcode::Switch.operand_count(), 3);
        // Label 需要 1 个操作数: label_id (u8)
        assert_eq!(TypedOpcode::Label.operand_count(), 1);
        // LoopStart 需要 4 个操作数: start_reg, end_reg, step_reg, exit_offset
        assert_eq!(TypedOpcode::LoopStart.operand_count(), 4);
        // LoopInc 需要 3 个操作数: dst, current, step
        assert_eq!(TypedOpcode::LoopInc.operand_count(), 3);
        // Yield 无操作数
        assert_eq!(TypedOpcode::Yield.operand_count(), 0);
        // TailCall 需要 4 个操作数
        assert_eq!(TypedOpcode::TailCall.operand_count(), 4);
    }
}

#[cfg(test)]
mod try_from_tests {
    use super::*;

    /// 测试 TryFrom
    #[test]
    fn test_try_from_valid() {
        assert_eq!(TypedOpcode::try_from(0x00), Ok(TypedOpcode::Nop));
        assert_eq!(TypedOpcode::try_from(0x20), Ok(TypedOpcode::I64Add));
        assert_eq!(TypedOpcode::try_from(0x40), Ok(TypedOpcode::F64Add));
        assert_eq!(TypedOpcode::try_from(0xFF), Ok(TypedOpcode::Invalid));
    }

    #[test]
    fn test_try_from_control_flow_opcodes() {
        // 控制流指令 (0x06-0x0B)
        assert_eq!(TypedOpcode::try_from(0x06), Ok(TypedOpcode::Switch));
        assert_eq!(TypedOpcode::try_from(0x07), Ok(TypedOpcode::LoopStart));
        assert_eq!(TypedOpcode::try_from(0x08), Ok(TypedOpcode::LoopInc));
        assert_eq!(TypedOpcode::try_from(0x09), Ok(TypedOpcode::TailCall));
        assert_eq!(TypedOpcode::try_from(0x0A), Ok(TypedOpcode::Yield));
        assert_eq!(TypedOpcode::try_from(0x0B), Ok(TypedOpcode::Label));
    }

    #[test]
    fn test_try_from_invalid() {
        assert!(TypedOpcode::try_from(0x0F).is_err());
        assert!(TypedOpcode::try_from(0x1F).is_err());
        // 0x7E 未被使用（已使用的编码: 0x72-0x7C）
        assert!(TypedOpcode::try_from(0x7E).is_err());
        // 0x89-0x8F 是函数调用操作码，已被使用
        assert!(TypedOpcode::try_from(0x89).is_err());
        assert!(TypedOpcode::try_from(0x96).is_err());
    }
}

#[cfg(test)]
mod display_trait_tests {
    use super::*;

    /// 测试 Display trait
    #[test]
    fn test_display_trait() {
        assert_eq!(format!("{}", TypedOpcode::Nop), "Nop");
        assert_eq!(format!("{}", TypedOpcode::I64Add), "I64Add");
        assert_eq!(format!("{}", TypedOpcode::CallStatic), "CallStatic");
    }
}

#[cfg(test)]
mod partial_eq_tests {
    use super::*;

    /// 测试 PartialEq
    #[test]
    fn test_partial_eq() {
        assert_eq!(TypedOpcode::Nop, TypedOpcode::Nop);
        assert_ne!(TypedOpcode::Nop, TypedOpcode::Return);
    }
}

#[cfg(test)]
mod debug_trait_tests {
    use super::*;

    /// 测试 Debug
    #[test]
    fn test_debug_trait() {
        let debug = format!("{:?}", TypedOpcode::Nop);
        assert!(debug.contains("Nop"));
    }
}
