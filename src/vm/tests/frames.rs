//! Frame 单元测试
//!
//! 测试虚拟机调用帧的创建和行为

use crate::vm::frames::Frame;
use crate::vm::Value;

#[cfg(test)]
mod frame_tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let frame = Frame::new(
            "test_function".to_string(),
            100,
            50,
            vec![],
        );
        assert_eq!(frame.name, "test_function");
        assert_eq!(frame.return_addr, 100);
        assert_eq!(frame.saved_fp, 50);
        assert!(frame.locals.is_empty());
    }

    #[test]
    fn test_frame_with_locals() {
        let locals = vec![
            Value::Int(1),
            Value::Int(2),
            Value::Bool(true),
        ];
        let frame = Frame::new("func".to_string(), 0, 0, locals);
        assert_eq!(frame.locals.len(), 3);
        assert!(matches!(frame.locals[0], Value::Int(1)));
    }

    #[test]
    fn test_frame_debug() {
        let frame = Frame::new("test".to_string(), 10, 5, vec![]);
        let debug_output = format!("{:?}", frame);
        assert!(debug_output.contains("test"));
    }
}
