//! 代码生成器单元测试
//!
//! 测试字节码生成功能

#[cfg(test)]
mod opcode_tests {
    use yaoxiang::vm::opcode::Opcode;

    #[test]
    fn test_opcode_name() {
        assert_eq!(Opcode::Nop.name(), "Nop");
        assert_eq!(Opcode::I64Add.name(), "I64Add");
        assert_eq!(Opcode::F64Mul.name(), "F64Mul");
        assert_eq!(Opcode::Return.name(), "Return");
    }

    #[test]
    fn test_opcode_is_numeric() {
        assert!(Opcode::I64Add.is_numeric_op());
        assert!(Opcode::F64Add.is_numeric_op());
        assert!(!Opcode::Nop.is_numeric_op());
        assert!(!Opcode::Return.is_numeric_op());
    }

    #[test]
    fn test_opcode_is_integer() {
        assert!(Opcode::I64Add.is_integer_op());
        assert!(Opcode::I32Mul.is_integer_op());
        assert!(!Opcode::F64Add.is_integer_op());
    }

    #[test]
    fn test_opcode_is_float() {
        assert!(Opcode::F64Add.is_float_op());
        assert!(Opcode::F32Mul.is_float_op());
        assert!(!Opcode::I64Add.is_float_op());
    }

    #[test]
    fn test_opcode_is_load() {
        assert!(Opcode::LoadConst.is_load_op());
        assert!(Opcode::LoadLocal.is_load_op());
        assert!(!Opcode::Nop.is_load_op());
    }

    #[test]
    fn test_opcode_is_store() {
        assert!(Opcode::StoreLocal.is_store_op());
        assert!(!Opcode::Nop.is_store_op());
    }

    #[test]
    fn test_opcode_is_call() {
        assert!(Opcode::CallStatic.is_call_op());
        assert!(Opcode::CallVirt.is_call_op());
        assert!(Opcode::CallDyn.is_call_op());
        assert!(!Opcode::Nop.is_call_op());
    }

    #[test]
    fn test_opcode_try_from_valid() {
        assert_eq!(Opcode::try_from(0x00), Ok(Opcode::Nop));
        assert_eq!(Opcode::try_from(0x20), Ok(Opcode::I64Add));
        assert_eq!(Opcode::try_from(0x40), Ok(Opcode::F64Add));
        assert_eq!(Opcode::try_from(0xFF), Ok(Opcode::Invalid));
    }

    #[test]
    fn test_opcode_try_from_invalid() {
        assert!(Opcode::try_from(0x09).is_err());
        assert!(Opcode::try_from(0x1F).is_err());
        assert!(Opcode::try_from(0x2F).is_err());
    }

    #[test]
    fn test_opcode_operand_count() {
        assert_eq!(Opcode::Nop.operand_count(), 0);
        assert_eq!(Opcode::Return.operand_count(), 0);
        assert_eq!(Opcode::Mov.operand_count(), 2);
        assert_eq!(Opcode::I64Add.operand_count(), 3);
        assert_eq!(Opcode::CallStatic.operand_count(), 4);
    }
}

#[cfg(test)]
mod monomorphize_tests {
    use yaoxiang::middle::monomorphize::{GenericFunctionId, SpecializationKey};

    #[test]
    fn test_generic_function_id() {
        let id = GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]);
        assert_eq!(id.name(), "identity");
        assert_eq!(id.signature(), "identity<T>");
    }

    #[test]
    fn test_specialization_key() {
        let key = SpecializationKey::new(
            "identity".to_string(),
            vec![yaoxiang::frontend::typecheck::MonoType::Int(64)],
        );
        assert_eq!(key.to_string(), "identity<int64>");
    }

    #[test]
    fn test_specialization_key_no_args() {
        let key = SpecializationKey::new("main".to_string(), vec![]);
        assert_eq!(key.to_string(), "main");
    }
}

#[cfg(test)]
mod escape_analysis_tests {
    use yaoxiang::middle::escape_analysis::{Allocation, EscapeAnalysisResult, LocalId};

    #[test]
    fn test_local_id() {
        let id = LocalId::new(5);
        assert_eq!(id.index(), 5);
    }

    #[test]
    fn test_escape_analysis_result() {
        let mut result = EscapeAnalysisResult::default();
        result.mark_stack_allocated(LocalId::new(0));
        result.mark_heap_allocated(LocalId::new(1));

        assert!(result.should_stack_allocate(LocalId::new(0)));
        assert!(!result.should_stack_allocate(LocalId::new(1)));
        assert!(result.should_heap_allocate(LocalId::new(1)));
        assert!(!result.should_heap_allocate(LocalId::new(0)));

        assert_eq!(result.get_allocation(LocalId::new(0)), Allocation::Stack);
        assert_eq!(result.get_allocation(LocalId::new(1)), Allocation::Heap);
        assert_eq!(result.get_allocation(LocalId::new(2)), Allocation::Undecided);
    }

    #[test]
    fn test_result_builder() {
        let builder = EscapeAnalysisResultBuilder::new();
        let result = builder
            .mark_stack_allocated(LocalId::new(0))
            .mark_heap_allocated(LocalId::new(1))
            .build();

        assert!(result.should_stack_allocate(LocalId::new(0)));
        assert!(result.should_heap_allocate(LocalId::new(1)));
    }

    #[test]
    fn test_result_builder_batch() {
        let mut builder = EscapeAnalysisResultBuilder::new();
        builder.mark_stack_allocated_batch(vec![LocalId::new(0), LocalId::new(2)]);

        let result = builder.build();
        assert!(result.should_stack_allocate(LocalId::new(0)));
        assert!(result.should_stack_allocate(LocalId::new(2)));
    }
}

#[cfg(test)]
mod codegen_tests {
    use yaoxiang::middle::ir::{ConstValue, FunctionIR, ModuleIR, Operand};

    #[test]
    fn test_function_ir() {
        let func = FunctionIR::new("test");
        assert_eq!(func.name, "test");
    }

    #[test]
    fn test_module_ir() {
        let module = ModuleIR::default();
        assert!(module.functions.is_empty());
        assert!(module.types.is_empty());
        assert!(module.constants.is_empty());
    }

    #[test]
    fn test_operand_variants() {
        let local = Operand::Local(0);
        let temp = Operand::Temp(1);
        let arg = Operand::Arg(2);
        let global = Operand::Global(3);
        let label = Operand::Label(4);

        // Just ensure they can be created
        assert!(matches!(local, Operand::Local(0)));
        assert!(matches!(temp, Operand::Temp(1)));
        assert!(matches!(arg, Operand::Arg(2)));
        assert!(matches!(global, Operand::Global(3)));
        assert!(matches!(label, Operand::Label(4)));
    }

    #[test]
    fn test_const_value() {
        let void = ConstValue::Void;
        let boolean = ConstValue::Bool(true);
        let integer = ConstValue::Int(42);
        let float = ConstValue::Float(3.14);
        let string = ConstValue::String("hello".to_string());

        assert!(matches!(void, ConstValue::Void));
        assert!(matches!(boolean, ConstValue::Bool(true)));
        assert!(matches!(integer, ConstValue::Int(42)));
        assert!(matches!(float, ConstValue::Float(_)));
        assert!(matches!(string, ConstValue::String(s) if s == "hello"));
    }
}

#[cfg(test)]
mod bytecode_instruction_tests {
    use yaoxiang::middle::codegen::BytecodeInstruction;
    use yaoxiang::vm::opcode::Opcode;

    #[test]
    fn test_bytecode_instruction_new() {
        let instr = BytecodeInstruction::new(Opcode::I64Add, vec![0, 1, 2]);
        assert_eq!(instr.opcode, Opcode::I64Add);
        assert_eq!(instr.operands, vec![0, 1, 2]);
    }

    #[test]
    fn test_bytecode_instruction_encode() {
        let instr = BytecodeInstruction::new(Opcode::I64Add, vec![0, 1, 2]);
        let bytes = instr.encode();
        assert_eq!(bytes[0], Opcode::I64Add as u8);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 1);
        assert_eq!(bytes[3], 2);
    }

    #[test]
    fn test_bytecode_instruction_encode_empty() {
        let instr = BytecodeInstruction::new(Opcode::Return, vec![]);
        let bytes = instr.encode();
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], Opcode::Return as u8);
    }
}
