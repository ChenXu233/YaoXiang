//! 代码生成上下文单元测试
//!
//! 测试 CodegenContext 的基本创建和功能。

use crate::middle::core::ir::ModuleIR;
use crate::middle::passes::codegen::mod_::CodegenContext;

#[test]
fn test_basic_codegen_context() {
    let module = ModuleIR::default();
    let ctx = CodegenContext::new(module);
    assert_eq!(ctx.module.functions.len(), 0);
}
