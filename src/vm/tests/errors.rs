//! VMError 单元测试
//!
//! 测试虚拟机错误的定义和 Display 实现

use crate::vm::errors::{VMError, VMResult};

#[cfg(test)]
mod error_display_tests {
    use super::*;

    #[test]
    fn test_vm_error_invalid_opcode() {
        let error = VMError::InvalidOpcode(0xFF);
        assert_eq!(format!("{}", error), "Invalid opcode: 255");
    }

    #[test]
    fn test_vm_error_stack_underflow() {
        let error = VMError::StackUnderflow;
        assert_eq!(format!("{}", error), "Stack underflow");
    }

    #[test]
    fn test_vm_error_stack_overflow() {
        let error = VMError::StackOverflow;
        assert_eq!(format!("{}", error), "Stack overflow");
    }

    #[test]
    fn test_vm_error_invalid_operand() {
        let error = VMError::InvalidOperand;
        assert_eq!(format!("{}", error), "Invalid operand");
    }

    #[test]
    fn test_vm_error_division_by_zero() {
        let error = VMError::DivisionByZero;
        assert_eq!(format!("{}", error), "Division by zero");
    }

    #[test]
    fn test_vm_error_type_error() {
        let error = VMError::TypeError("expected int, got float".to_string());
        assert_eq!(format!("{}", error), "Type error: expected int, got float");
    }

    #[test]
    fn test_vm_error_index_out_of_bounds() {
        let error = VMError::IndexOutOfBounds;
        assert_eq!(format!("{}", error), "Index out of bounds");
    }

    #[test]
    fn test_vm_error_key_not_found() {
        let error = VMError::KeyNotFound;
        assert_eq!(format!("{}", error), "Key not found");
    }

    #[test]
    fn test_vm_error_uninitialized_variable() {
        let error = VMError::UninitializedVariable;
        assert_eq!(format!("{}", error), "Uninitialized variable");
    }

    #[test]
    fn test_vm_error_call_stack_overflow() {
        let error = VMError::CallStackOverflow;
        assert_eq!(format!("{}", error), "Call stack overflow");
    }

    #[test]
    fn test_vm_error_runtime_error() {
        let error = VMError::RuntimeError("null pointer dereference".to_string());
        assert_eq!(
            format!("{}", error),
            "Runtime error: null pointer dereference"
        );
    }

    #[test]
    fn test_vm_error_out_of_memory() {
        let error = VMError::OutOfMemory;
        assert_eq!(format!("{}", error), "Out of memory");
    }

    #[test]
    fn test_vm_error_invalid_state() {
        let error = VMError::InvalidState("invalid pc".to_string());
        assert_eq!(format!("{}", error), "Invalid state: invalid pc");
    }
}

#[cfg(test)]
mod vm_result_tests {
    use super::*;

    #[test]
    fn test_vm_result_ok() {
        let result: VMResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_vm_result_err() {
        let result: VMResult<i32> = Err(VMError::StackOverflow);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMError::StackOverflow));
    }
}
