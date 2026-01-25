//! 统一测试套件
//!
//! 包含middle层所有模块的集成测试。

pub mod codegen;
pub mod lifetime;
pub mod module;
pub mod mono;

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// 集成测试示例
    #[test]
    fn test_middle_layer_integration() {
        // 这里可以添加跨模块的集成测试
    }
}
