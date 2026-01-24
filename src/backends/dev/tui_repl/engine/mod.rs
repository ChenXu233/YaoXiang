//! 增量编译引擎
/// 增量编译引擎
///
/// 提供高性能的增量编译能力，支持状态保持和缓存优化
pub mod compiler;
pub mod module_builder;
pub mod profiler;
pub mod symbol_cache;

pub use compiler::IncrementalCompiler;
pub use module_builder::ModuleBuilder;
pub use symbol_cache::SymbolCache;
pub use profiler::Profiler;
