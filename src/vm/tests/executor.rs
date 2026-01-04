//! VM 单元测试
//!
//! 测试虚拟机执行器的配置、状态和值类型

use crate::vm::{VMError, VM, VMConfig, VMStatus, Value};
use crate::vm::executor::Opcode;
use crate::middle::ModuleIR;

#[cfg(test)]
mod vm_config_tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VMConfig::default();
        assert_eq!(config.stack_size, 64 * 1024);
        assert!(!config.enable_jit);
    }

    #[test]
    fn test_vm_config_custom() {
        let config = VMConfig {
            stack_size: 128 * 1024,
            enable_jit: true,
        };
        assert_eq!(config.stack_size, 128 * 1024);
        assert!(config.enable_jit);
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
        let mut vm = VM::new();
        // Create a minimal ModuleIR for testing
        let module = ModuleIR {
            types: vec![],
            constants: vec![],
            globals: vec![],
            functions: vec![],
        };
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
        let value = Value::Void;
        assert!(matches!(value, Value::Void));
    }

    #[test]
    fn test_value_bool() {
        let true_val = Value::Bool(true);
        let false_val = Value::Bool(false);
        assert!(matches!(true_val, Value::Bool(true)));
        assert!(matches!(false_val, Value::Bool(false)));
    }

    #[test]
    fn test_value_int() {
        let value = Value::Int(42);
        assert!(matches!(value, Value::Int(42)));
        let negative = Value::Int(-100);
        assert!(matches!(negative, Value::Int(-100)));
    }

    #[test]
    fn test_value_float() {
        let value = Value::Float(3.14);
        assert!(matches!(value, Value::Float(f) if (f - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_value_char() {
        let value = Value::Char('中');
        assert!(matches!(value, Value::Char('中')));
    }

    #[test]
    fn test_value_string() {
        let value = Value::String("hello".to_string());
        assert!(matches!(value, Value::String(s) if s == "hello"));
    }

    #[test]
    fn test_value_bytes() {
        let value = Value::Bytes(vec![1, 2, 3]);
        assert!(matches!(value, Value::Bytes(b) if b == vec![1, 2, 3]));
    }

    #[test]
    fn test_value_list() {
        let value = Value::List(vec![Value::Int(1), Value::Int(2)]);
        assert!(matches!(value, Value::List(list) if list.len() == 2));
    }

    #[test]
    fn test_value_clone() {
        let value = Value::Int(42);
        let cloned = value.clone();
        assert!(matches!(cloned, Value::Int(42)));
    }
}

#[cfg(test)]
mod opcode_tests {
    use super::*;

    #[test]
    fn test_opcode_values() {
        assert_eq!(Opcode::Nop as u8, 0x00);
        assert_eq!(Opcode::Push as u8, 0x01);
        assert_eq!(Opcode::Pop as u8, 0x02);
        assert_eq!(Opcode::Dup as u8, 0x03);
        assert_eq!(Opcode::Swap as u8, 0x04);
    }

    #[test]
    fn test_opcode_try_from_valid() {
        let result = Opcode::try_from(0x00);
        assert!(result.is_ok());
        assert!(matches!(result, Ok(Opcode::Nop)));
    }

    #[test]
    fn test_opcode_try_from_invalid() {
        assert!(Opcode::try_from(0xFF).is_err());
        assert!(matches!(Opcode::try_from(0xFF), Err(VMError::InvalidOpcode(0xFF))));
    }

    #[test]
    fn test_opcode_partial_eq() {
        assert_eq!(Opcode::Nop, Opcode::Nop);
        assert_ne!(Opcode::Nop, Opcode::Push);
    }

    #[test]
    fn test_opcode_debug() {
        let debug_output = format!("{:?}", Opcode::Nop);
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
