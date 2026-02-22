//! Frontend Compilation Pipeline
//!
//! 编译器，包含前端模块词法分析、语法分析、类型检查和 IR 生成。
//! 提供细粒度的事件系统，用于 IDE 和 LSP 集成支持。
//!
//! # 模块结构
//!
//! - [`core`] - 核心算法层（词法分析器、解析器、类型系统）
//! - [`typecheck`] - 类型检查层
//! - [`type_level`] - 类型级计算（RFC-011）
//! - [`config`] - 编译配置
//! - [`pipeline`] - 编译流水线
//! - [`events`] - 事件系统
//!
//! # 快速开始
//!
//! ```ignore
//! use yaoxiang::frontend::{Compiler, CompileConfig};
//!
//! let config = CompileConfig::new();
//! let mut compiler = Compiler::with_config(config);
//!
//! match compiler.compile("test.yao", "let x = 42;") {
//!     Ok(ir) => println!("Compiled successfully!"),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! # 事件订阅
//!
//! 编译器支持事件订阅，可以用来实现 IDE 进度显示和诊断报告：
//!
//! ```ignore
//! use yaoxiang::frontend::events::{EventBus, EventSubscriber};
//!
//! struct ProgressLogger;
//!
//! impl EventSubscriber for ProgressLogger {
//!     fn event_types(&self) -> Vec<EventType> {
//!         vec![EventType::Phase, EventType::Progress]
//!     }
//!
//!     fn on_event(&self, event: &dyn Any, metadata: &EventMetadata) {
//!         println!("Event: {}, seq: {}", event.name(), metadata.sequence);
//!     }
//! }
//!
//! let event_bus = EventBus::new();
//! event_bus.subscribe(ProgressLogger);
//! ```

// 重新导出公共 API
// =================

// 核心模块（词法分析器、解析器、类型系统）
pub mod core;

// 通用模块系统
pub mod module;

// 类型检查层
pub mod typecheck;

// 类型级计算（RFC-011）
pub mod type_level;

// 诊断系统
pub use crate::util::diagnostic;

// 编译配置
pub mod config;

// 编译流水线
pub mod pipeline;

// 事件系统
pub mod events;

// 编译器核心（事件驱动）
pub mod compiler;

// 重新导出类型
// =============

// 编译器
pub use compiler::Compiler;

// 编译配置
pub use config::{CompileConfig, OptLevel, DiagLevel, FeatureFlags, ErrorRecoveryStrategy};

// 编译流水线
pub use pipeline::{Pipeline, PipelineState, CompilationResult};

// 编译结果
pub use compiler::CompileError;

// 事件类型
pub use events::*;
