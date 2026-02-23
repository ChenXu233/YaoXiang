//! 编译世界（World）
//!
//! 管理编译器状态和符号索引，为 LSP 提供语义分析支持。

use crate::frontend::core::lexer::symbols::SymbolIndex;

/// 编译世界
///
/// 聚合编译器运行所需的全局状态：
/// - 符号索引（用于 go-to-definition、find-references 等）
/// - 编译器实例管理
///
/// 后续阶段将扩展为包含：
/// - 增量编译管道
/// - 依赖图
/// - 类型环境快照
#[derive(Debug, Default)]
pub struct World {
    /// 全局符号索引
    symbol_index: SymbolIndex,
}

impl World {
    /// 创建新的编译世界
    pub fn new() -> Self {
        Self {
            symbol_index: SymbolIndex::new(),
        }
    }

    /// 获取符号索引（不可变）
    pub fn symbol_index(&self) -> &SymbolIndex {
        &self.symbol_index
    }

    /// 获取符号索引（可变）
    pub fn symbol_index_mut(&mut self) -> &mut SymbolIndex {
        &mut self.symbol_index
    }

    /// 移除某个文件的所有符号（文件关闭或重新解析时调用）
    pub fn remove_file_symbols(
        &mut self,
        file_path: &str,
    ) {
        self.symbol_index.remove_file(file_path);
    }

    /// 获取索引中的符号总数
    pub fn symbol_count(&self) -> usize {
        self.symbol_index.symbol_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_new() {
        let world = World::new();
        assert_eq!(world.symbol_count(), 0);
    }
}
