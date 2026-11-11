//! 中间表示层
//!
//! 编译器中间层，负责从AST到字节码的转换过程。
//!
//! ## 架构
//!
//! 采用三层清晰分层设计：
//!
//! 1. **bytecode/**: 字节码域（D1 6.7.1b）
//!    - opcode 词表、.42 容器格式与序列化、字节码 IR
//!
//! 2. **ir/**: IR 定义与生成器（D1 6.7.1c：`core/` 改名 `ir/`，目录名说明职责）
//!    - SSA IR 定义（指令 / 操作数 / 基本块 / 模块结构）
//!    - IR 生成器（AST → IR）、规范化、verify
//!
//! 3. **passes/**: 编译器各个阶段
//!    - mono/: 泛型单态化
//!    - module/: 模块系统
//!    - codegen/: 代码生成
//!    - tests/: 统一测试套件
//!
//!    （生命周期检查不在 middle 层：RFC-009/009a 令牌生命期分析
//!    住 frontend/core/sema/layers/ownership.rs）
//!
//! 4. **对外接口**: 统一的API导出

#![allow(ambiguous_glob_reexports)]

// 字节码域（opcode 词表 + .42 容器/序列化 + 字节码 IR）
pub mod bytecode;

// IR 定义与生成器（D1 6.7.1c：core/ → ir/）
pub mod ir;

// 编译器各个阶段
pub mod passes;

// 对外导出
pub use ir::ir::*;
pub use ir::ir_gen::*;
pub use passes::mono::*;
pub use passes::codegen;

// 特别导出：monomorphize的实例化相关类型
pub use passes::mono::instance::*;
