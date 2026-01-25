//! 中间表示层
//!
//! 编译器中间层，负责从AST到字节码的转换过程。
//!
//! ## 架构
//!
//! 采用三层清晰分层设计：
//!
//! 1. **core/**: 核心数据结构
//!    - IR定义、字节码格式、模块结构
//!    - IR生成器
//!
//! 2. **passes/**: 编译器各个阶段
//!    - lifetime/: 生命周期检查
//!    - mono/: 泛型单态化
//!    - module/: 模块系统
//!    - codegen/: 代码生成
//!    - tests/: 统一测试套件
//!
//! 3. **对外接口**: 统一的API导出

#![allow(ambiguous_glob_reexports)]

// 核心数据结构
pub mod core;

// 编译器各个阶段
pub mod passes;

// 对外导出
pub use core::ir::*;
pub use core::bytecode;
pub use core::ir_gen::*;
pub use passes::lifetime::*;
pub use passes::mono::*;
pub use passes::module::*;
pub use passes::codegen;

// 特别导出：monomorphize的实例化相关类型
pub use passes::mono::instance::*;
