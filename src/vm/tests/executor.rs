//! VM 单元测试
//!
//! 测试虚拟机执行器的配置、状态和值类型

use crate::runtime::value::RuntimeValue;
use crate::vm::executor::{VMConfig, VMStatus, VM};
use crate::vm::opcode::TypedOpcode;
use std::sync::Arc;

#[cfg(test)]
mod vm_config_tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VMConfig::default();
        assert_eq!(config.stack_size, 64 * 1024);
        assert_eq!(config.max_call_depth, 1024);
        assert!(!config.trace_execution);
    }

    #[test]
    fn test_vm_config_custom() {
        let config = VMConfig {
            stack_size: 128 * 1024,
            max_call_depth: 2048,
            trace_execution: true,
        };
        assert_eq!(config.stack_size, 128 * 1024);
        assert_eq!(config.max_call_depth, 2048);
        assert!(config.trace_execution);
    }

    #[test]
    fn test_vm_config_clone() {
        let config = VMConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.stack_size, config.stack_size);
    }
}

#[cfg(test)]
mod vm_tests {
    use super::*;

    #[test]
    fn test_vm_new() {
        let vm = VM::new();
        assert_eq!(vm.status(), VMStatus::Ready);
        assert!(vm.error().is_none());
    }

    #[test]
    fn test_vm_default() {
        let vm = VM::default();
        assert_eq!(vm.status(), VMStatus::Ready);
    }

    #[test]
    fn test_vm_new_with_config() {
        let config = VMConfig::default();
        let vm = VM::new_with_config(config);
        assert_eq!(vm.status(), VMStatus::Ready);
    }

    #[test]
    fn test_vm_execute_module() {
        use crate::middle::codegen::bytecode::CompiledModule;
        use crate::middle::ir::ModuleIR;

        let mut vm = VM::new();
        // Create a minimal CompiledModule for testing
        let module = CompiledModule::from_ir(&ModuleIR {
            types: vec![],
            globals: vec![],
            functions: vec![],
        });
        let result = vm.execute_module(&module);
        assert!(result.is_ok());
        assert_eq!(vm.status(), VMStatus::Finished);
    }

    #[test]
    fn test_vm_debug() {
        let vm = VM::new();
        let debug_output = format!("{:?}", vm);
        assert!(debug_output.contains("VM"));
    }
}

#[cfg(test)]
mod value_tests {
    use super::*;

    #[test]
    fn test_value_void() {
        let value = RuntimeValue::Unit;
        assert!(matches!(value, RuntimeValue::Unit));
    }

    #[test]
    fn test_value_bool() {
        let true_val = RuntimeValue::Bool(true);
        let false_val = RuntimeValue::Bool(false);
        assert!(matches!(true_val, RuntimeValue::Bool(true)));
        assert!(matches!(false_val, RuntimeValue::Bool(false)));
    }

    #[test]
    fn test_value_int() {
        let value = RuntimeValue::Int(42);
        assert!(matches!(value, RuntimeValue::Int(42)));
        let negative = RuntimeValue::Int(-100);
        assert!(matches!(negative, RuntimeValue::Int(-100)));
    }

    #[test]
    fn test_value_float() {
        let value = RuntimeValue::Float(3.14);
        assert!(matches!(value, RuntimeValue::Float(f) if (f - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_value_char() {
        let value = RuntimeValue::Char('中' as u32);
        assert!(matches!(value, RuntimeValue::Char(c) if c == '中' as u32));
    }

    #[test]
    fn test_value_string() {
        let value = RuntimeValue::String(Arc::from("hello"));
        assert!(matches!(value, RuntimeValue::String(s) if s.as_ref() == "hello"));
    }

    #[test]
    fn test_value_bytes() {
        let value = RuntimeValue::Bytes(Arc::from(vec![1, 2, 3]));
        assert!(matches!(value, RuntimeValue::Bytes(b) if b.as_ref() == vec![1, 2, 3]));
    }

    #[test]
    fn test_value_list() {
        let value = RuntimeValue::List(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]);
        assert!(matches!(value, RuntimeValue::List(list) if list.len() == 2));
    }

    #[test]
    fn test_value_clone() {
        let value = RuntimeValue::Int(42);
        let cloned = value.clone();
        assert!(matches!(cloned, RuntimeValue::Int(42)));
    }
}

#[cfg(test)]
mod opcode_tests {
    use super::*;

    #[test]
    fn test_opcode_values() {
        assert_eq!(TypedOpcode::Nop as u8, 0x00);
        assert_eq!(TypedOpcode::Return as u8, 0x01);
        assert_eq!(TypedOpcode::ReturnValue as u8, 0x02);
        assert_eq!(TypedOpcode::Jmp as u8, 0x03);
    }

    #[test]
    fn test_opcode_try_from_valid() {
        let result = TypedOpcode::try_from(0x00);
        assert!(result.is_ok());
        assert!(matches!(result, Ok(TypedOpcode::Nop)));
    }

    #[test]
    fn test_opcode_try_from_invalid() {
        // 0x7E 未被使用（ArcDrop 是 0x7C），应该返回错误
        let result = TypedOpcode::try_from(0x7E);
        assert!(result.is_err());
    }

    #[test]
    fn test_opcode_partial_eq() {
        assert_eq!(TypedOpcode::Nop, TypedOpcode::Nop);
        assert_ne!(TypedOpcode::Nop, TypedOpcode::Return);
    }

    #[test]
    fn test_opcode_debug() {
        let debug_output = format!("{:?}", TypedOpcode::Nop);
        assert!(debug_output.contains("Nop"));
    }
}

#[cfg(test)]
mod vm_status_tests {
    use super::*;

    #[test]
    fn test_vm_status_values() {
        assert_eq!(VMStatus::Ready as u8, 0);
        assert_eq!(VMStatus::Running as u8, 1);
        assert_eq!(VMStatus::Finished as u8, 2);
        assert_eq!(VMStatus::Error as u8, 3);
    }

    #[test]
    fn test_vm_status_partial_eq() {
        assert_eq!(VMStatus::Ready, VMStatus::Ready);
        assert_ne!(VMStatus::Ready, VMStatus::Running);
    }

    #[test]
    fn test_vm_status_debug() {
        let debug_output = format!("{:?}", VMStatus::Ready);
        assert!(debug_output.contains("Ready"));
    }
}

// =====================
// 指令执行集成测试
// =====================

#[cfg(test)]
mod instruction_execution_tests {

    mod opcode_encoding_tests {
        use crate::vm::opcode::TypedOpcode;

        #[test]
        fn test_i32_opcodes_exist() {
            // 验证 I32 操作码存在（不检查具体编码，只验证变体存在）
            let _ = TypedOpcode::I32Add;
            let _ = TypedOpcode::I32Sub;
            let _ = TypedOpcode::I32Mul;
            let _ = TypedOpcode::I32Div;
            let _ = TypedOpcode::I32Rem;
            let _ = TypedOpcode::I32Neg;
        }

        #[test]
        fn test_i32_bitwise_opcodes_exist() {
            let _ = TypedOpcode::I32And;
            let _ = TypedOpcode::I32Or;
            let _ = TypedOpcode::I32Xor;
            let _ = TypedOpcode::I32Shl;
            let _ = TypedOpcode::I32Sar;
            let _ = TypedOpcode::I32Shr;
        }

        #[test]
        fn test_i64_bitwise_opcodes_exist() {
            let _ = TypedOpcode::I64And;
            let _ = TypedOpcode::I64Or;
            let _ = TypedOpcode::I64Xor;
            let _ = TypedOpcode::I64Shl;
            let _ = TypedOpcode::I64Sar;
            let _ = TypedOpcode::I64Shr;
        }

        #[test]
        fn test_f32_opcodes_exist() {
            let _ = TypedOpcode::F32Add;
            let _ = TypedOpcode::F32Sub;
            let _ = TypedOpcode::F32Mul;
            let _ = TypedOpcode::F32Div;
            let _ = TypedOpcode::F32Rem;
            let _ = TypedOpcode::F32Sqrt;
            let _ = TypedOpcode::F32Neg;
        }

        #[test]
        fn test_f32_comparison_opcodes_exist() {
            let _ = TypedOpcode::F32Eq;
            let _ = TypedOpcode::F32Ne;
            let _ = TypedOpcode::F32Lt;
            let _ = TypedOpcode::F32Le;
            let _ = TypedOpcode::F32Gt;
            let _ = TypedOpcode::F32Ge;
        }

        #[test]
        fn test_f64_rem_exists() {
            let _ = TypedOpcode::F64Rem;
        }

        #[test]
        fn test_string_opcodes_exist() {
            let _ = TypedOpcode::StringLength;
            let _ = TypedOpcode::StringConcat;
            let _ = TypedOpcode::StringEqual;
            let _ = TypedOpcode::StringGetChar;
            let _ = TypedOpcode::StringFromInt;
            let _ = TypedOpcode::StringFromFloat;
        }

        #[test]
        fn test_closure_opcodes_exist() {
            let _ = TypedOpcode::MakeClosure;
            let _ = TypedOpcode::LoadUpvalue;
            let _ = TypedOpcode::StoreUpvalue;
            let _ = TypedOpcode::CloseUpvalue;
        }

        #[test]
        fn test_exception_opcodes_exist() {
            let _ = TypedOpcode::TryBegin;
            let _ = TypedOpcode::TryEnd;
            let _ = TypedOpcode::Throw;
            let _ = TypedOpcode::Rethrow;
        }

        #[test]
        fn test_memory_opcodes_exist() {
            let _ = TypedOpcode::StackAlloc;
            let _ = TypedOpcode::HeapAlloc;
            let _ = TypedOpcode::GetField;
            let _ = TypedOpcode::SetField;
        }

        #[test]
        fn test_arc_opcodes_exist() {
            let _ = TypedOpcode::ArcNew;
            let _ = TypedOpcode::ArcClone;
            let _ = TypedOpcode::ArcDrop;
        }

        #[test]
        fn test_type_opcodes_exist() {
            let _ = TypedOpcode::TypeCheck;
            let _ = TypedOpcode::TypeOf;
            let _ = TypedOpcode::Cast;
        }

        #[test]
        fn test_control_flow_opcodes_exist() {
            // Phase 1: 控制流指令
            let _ = TypedOpcode::Switch; // 0x06 - 多分支跳转
            let _ = TypedOpcode::LoopStart; // 0x07 - 循环开始
            let _ = TypedOpcode::LoopInc; // 0x08 - 循环递增
            let _ = TypedOpcode::TailCall; // 0x09 - 尾调用
            let _ = TypedOpcode::Yield; // 0x0A - 协程让出
            let _ = TypedOpcode::Label; // 0x0B - 标签定义
        }
    }

    mod opcode_parsing_tests {
        use crate::vm::opcode::TypedOpcode;

        #[test]
        fn test_parse_known_opcodes() {
            // 验证已知操作码可以正确解析
            assert!(TypedOpcode::try_from(0x00).is_ok()); // Nop
            assert!(TypedOpcode::try_from(0x01).is_ok()); // Return
            assert!(TypedOpcode::try_from(0x03).is_ok()); // Jmp
            assert!(TypedOpcode::try_from(0x10).is_ok()); // Mov
        }

        #[test]
        fn test_parse_i32_opcodes() {
            // 验证 I32 操作码解析
            assert!(TypedOpcode::try_from(0x30).is_ok()); // I32Add
            assert!(TypedOpcode::try_from(0x31).is_ok()); // I32Sub
            assert!(TypedOpcode::try_from(0x32).is_ok()); // I32Mul
            assert!(TypedOpcode::try_from(0x33).is_ok()); // I32Div
            assert!(TypedOpcode::try_from(0x34).is_ok()); // I32Rem
            assert!(TypedOpcode::try_from(0x35).is_ok()); // I32Neg
        }

        #[test]
        fn test_parse_i32_bitwise_opcodes() {
            assert!(TypedOpcode::try_from(0x36).is_ok()); // I32And
            assert!(TypedOpcode::try_from(0x37).is_ok()); // I32Or
            assert!(TypedOpcode::try_from(0x38).is_ok()); // I32Xor
            assert!(TypedOpcode::try_from(0x39).is_ok()); // I32Shl
            assert!(TypedOpcode::try_from(0x3A).is_ok()); // I32Sar
            assert!(TypedOpcode::try_from(0x3B).is_ok()); // I32Shr
        }

        #[test]
        fn test_parse_f32_opcodes() {
            assert!(TypedOpcode::try_from(0x50).is_ok()); // F32Add
            assert!(TypedOpcode::try_from(0x51).is_ok()); // F32Sub
            assert!(TypedOpcode::try_from(0x52).is_ok()); // F32Mul
            assert!(TypedOpcode::try_from(0x53).is_ok()); // F32Div
            assert!(TypedOpcode::try_from(0x54).is_ok()); // F32Rem
        }

        #[test]
        fn test_parse_f32_comparison_opcodes() {
            assert!(TypedOpcode::try_from(0x6C).is_ok());
            assert!(TypedOpcode::try_from(0x6D).is_ok());
            assert!(TypedOpcode::try_from(0x6E).is_ok());
            assert!(TypedOpcode::try_from(0x6F).is_ok());
            assert!(TypedOpcode::try_from(0x70).is_ok());
            assert!(TypedOpcode::try_from(0x71).is_ok());
        }

        #[test]
        fn test_parse_string_opcodes() {
            assert!(TypedOpcode::try_from(0x90).is_ok());
            assert!(TypedOpcode::try_from(0x91).is_ok());
            assert!(TypedOpcode::try_from(0x92).is_ok());
            assert!(TypedOpcode::try_from(0x93).is_ok());
            assert!(TypedOpcode::try_from(0x94).is_ok());
            assert!(TypedOpcode::try_from(0x95).is_ok());
        }

        #[test]
        fn test_parse_closure_opcodes() {
            assert!(TypedOpcode::try_from(0x83).is_ok()); // MakeClosure
            assert!(TypedOpcode::try_from(0x84).is_ok());
            assert!(TypedOpcode::try_from(0x85).is_ok());
            assert!(TypedOpcode::try_from(0x86).is_ok());
        }

        #[test]
        fn test_parse_exception_opcodes() {
            assert!(TypedOpcode::try_from(0xA0).is_ok());
            assert!(TypedOpcode::try_from(0xA1).is_ok());
            assert!(TypedOpcode::try_from(0xA2).is_ok());
            assert!(TypedOpcode::try_from(0xA3).is_ok());
        }

        #[test]
        fn test_parse_memory_opcodes() {
            assert!(TypedOpcode::try_from(0x72).is_ok());
            assert!(TypedOpcode::try_from(0x73).is_ok());
            assert!(TypedOpcode::try_from(0x75).is_ok());
            assert!(TypedOpcode::try_from(0x76).is_ok());
        }

        #[test]
        fn test_parse_arc_opcodes() {
            assert!(TypedOpcode::try_from(0x7A).is_ok()); // ArcNew
            assert!(TypedOpcode::try_from(0x7B).is_ok()); // ArcClone
            assert!(TypedOpcode::try_from(0x7C).is_ok()); // ArcDrop
        }

        #[test]
        fn test_parse_type_opcodes() {
            assert!(TypedOpcode::try_from(0xC0).is_ok()); // TypeCheck
            assert!(TypedOpcode::try_from(0xC1).is_ok()); // Cast
        }

        #[test]
        fn test_parse_control_flow_opcodes() {
            // Phase 1: 控制流指令解析测试
            assert!(TypedOpcode::try_from(0x06).is_ok()); // Switch
            assert!(TypedOpcode::try_from(0x07).is_ok()); // LoopStart
            assert!(TypedOpcode::try_from(0x08).is_ok()); // LoopInc
            assert!(TypedOpcode::try_from(0x09).is_ok()); // TailCall
            assert!(TypedOpcode::try_from(0x0A).is_ok()); // Yield
            assert!(TypedOpcode::try_from(0x0B).is_ok()); // Label
        }

        #[test]
        fn test_invalid_opcode_returns_error() {
            // 验证无效操作码返回错误
            assert!(TypedOpcode::try_from(0x7E).is_err()); // 未使用
        }
    }

    mod opcode_name_tests {
        use crate::vm::opcode::TypedOpcode;

        #[test]
        fn test_i32_opcode_names() {
            assert_eq!(TypedOpcode::I32Add.name(), "I32Add");
            assert_eq!(TypedOpcode::I32Sub.name(), "I32Sub");
            assert_eq!(TypedOpcode::I32Mul.name(), "I32Mul");
            assert_eq!(TypedOpcode::I32Div.name(), "I32Div");
            assert_eq!(TypedOpcode::I32Rem.name(), "I32Rem");
            assert_eq!(TypedOpcode::I32Neg.name(), "I32Neg");
        }

        #[test]
        fn test_f32_opcode_names() {
            assert_eq!(TypedOpcode::F32Add.name(), "F32Add");
            assert_eq!(TypedOpcode::F32Sub.name(), "F32Sub");
            assert_eq!(TypedOpcode::F32Mul.name(), "F32Mul");
            assert_eq!(TypedOpcode::F32Div.name(), "F32Div");
            assert_eq!(TypedOpcode::F32Rem.name(), "F32Rem");
            assert_eq!(TypedOpcode::F32Sqrt.name(), "F32Sqrt");
        }

        #[test]
        fn test_f32_comparison_names() {
            assert_eq!(TypedOpcode::F32Eq.name(), "F32Eq");
            assert_eq!(TypedOpcode::F32Ne.name(), "F32Ne");
            assert_eq!(TypedOpcode::F32Lt.name(), "F32Lt");
            assert_eq!(TypedOpcode::F32Le.name(), "F32Le");
            assert_eq!(TypedOpcode::F32Gt.name(), "F32Gt");
            assert_eq!(TypedOpcode::F32Ge.name(), "F32Ge");
        }

        #[test]
        fn test_string_opcode_names() {
            assert_eq!(TypedOpcode::StringLength.name(), "StringLength");
            assert_eq!(TypedOpcode::StringConcat.name(), "StringConcat");
            assert_eq!(TypedOpcode::StringEqual.name(), "StringEqual");
            assert_eq!(TypedOpcode::StringGetChar.name(), "StringGetChar");
        }

        #[test]
        fn test_exception_opcode_names() {
            assert_eq!(TypedOpcode::TryBegin.name(), "TryBegin");
            assert_eq!(TypedOpcode::TryEnd.name(), "TryEnd");
            assert_eq!(TypedOpcode::Throw.name(), "Throw");
            assert_eq!(TypedOpcode::Rethrow.name(), "Rethrow");
        }

        #[test]
        fn test_closure_opcode_names() {
            assert_eq!(TypedOpcode::MakeClosure.name(), "MakeClosure");
            assert_eq!(TypedOpcode::LoadUpvalue.name(), "LoadUpvalue");
            assert_eq!(TypedOpcode::StoreUpvalue.name(), "StoreUpvalue");
            assert_eq!(TypedOpcode::CloseUpvalue.name(), "CloseUpvalue");
        }

        #[test]
        fn test_control_flow_opcode_names() {
            // Phase 1: 控制流指令名称测试
            assert_eq!(TypedOpcode::Switch.name(), "Switch");
            assert_eq!(TypedOpcode::LoopStart.name(), "LoopStart");
            assert_eq!(TypedOpcode::LoopInc.name(), "LoopInc");
            assert_eq!(TypedOpcode::TailCall.name(), "TailCall");
            assert_eq!(TypedOpcode::Yield.name(), "Yield");
            assert_eq!(TypedOpcode::Label.name(), "Label");
        }
    }
}
