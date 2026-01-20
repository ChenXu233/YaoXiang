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
        // 0x74 未被使用（SetField 是 0x75），应该返回错误
        let result = TypedOpcode::try_from(0x74);
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
