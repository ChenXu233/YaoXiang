//! `std::ffi` 模块的单元测试
//!
//! 覆盖 `NativeBinding` 的创建、字段访问等功能。

use crate::std::ffi::NativeBinding;

#[test]
fn test_native_binding_creation() {
    let binding = NativeBinding::new("my_add", "my_add");
    assert_eq!(binding.func_name(), "my_add");
    assert_eq!(binding.native_symbol(), "my_add");
}

#[test]
fn test_native_binding_different_names() {
    let binding = NativeBinding::new("add", "math.add");
    assert_eq!(binding.func_name(), "add");
    assert_eq!(binding.native_symbol(), "math.add");
}
