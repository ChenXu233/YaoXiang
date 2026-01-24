//! 模块构建器
//!
//! 累积和管理多个语句，支持增量类型检查和符号表更新

use std::collections::HashMap;
use std::time::Instant;

use crate::Result;

/// 模块构建器
pub struct ModuleBuilder {
    /// 累积的语句
    statements: Vec<String>,
    /// 符号表
    symbols: HashMap<String, SymbolInfo>,
    /// 编译开始时间
    start_time: Instant,
    /// 语句计数
    statement_count: usize,
}

impl ModuleBuilder {
    /// 创建新的模块构建器
    pub fn new() -> Result<Self> {
        Ok(Self {
            statements: Vec::new(),
            symbols: HashMap::new(),
            start_time: Instant::now(),
            statement_count: 0,
        })
    }

    /// 添加语句
    pub fn add_statement(
        &mut self,
        statement: &str,
    ) -> Result<()> {
        let statement = statement.trim();
        if statement.is_empty() {
            return Ok(());
        }

        // 添加到语句列表
        self.statements.push(statement.to_string());
        self.statement_count += 1;

        // 增量类型检查（简化版，实际应该调用编译器）
        self.incremental_type_check(statement)?;

        // 更新符号表
        self.update_symbols(statement)?;

        Ok(())
    }

    /// 增量类型检查
    fn incremental_type_check(
        &self,
        statement: &str,
    ) -> Result<()> {
        // 这里是简化版实现
        // 实际应该：
        // 1. 解析语句
        // 2. 检查类型
        // 3. 验证与现有符号的兼容性

        // 简单的语法检查
        if statement.contains("let") {
            // 检查 let 语句
            self.check_let_statement(statement)?;
        }

        Ok(())
    }

    /// 检查 let 语句
    fn check_let_statement(
        &self,
        statement: &str,
    ) -> Result<()> {
        // 简化版：检查是否有语法错误
        if !statement.contains('=') {
            return Err(anyhow::anyhow!("Invalid let statement: missing '='"));
        }

        Ok(())
    }

    /// 更新符号表
    fn update_symbols(
        &mut self,
        statement: &str,
    ) -> Result<()> {
        // 提取变量名（简化版）
        if statement.starts_with("let") {
            if let Some(eq_pos) = statement.find('=') {
                let left = &statement[4..eq_pos].trim();
                let var_name = left.split_whitespace().next().unwrap_or("");
                if !var_name.is_empty() {
                    self.symbols.insert(
                        var_name.to_string(),
                        SymbolInfo {
                            name: var_name.to_string(),
                            defined_at: self.statement_count,
                            statement: statement.to_string(),
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// 获取语句数量
    pub fn statement_count(&self) -> usize {
        self.statement_count
    }

    /// 获取符号数量
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// 获取所有符号
    pub fn get_symbols(&self) -> &HashMap<String, SymbolInfo> {
        &self.symbols
    }

    /// 获取累积的代码
    pub fn get_code(&self) -> String {
        self.statements.join("\n")
    }

    /// 清空构建器
    pub fn reset(&mut self) -> Result<()> {
        self.statements.clear();
        self.symbols.clear();
        self.statement_count = 0;
        self.start_time = Instant::now();
        Ok(())
    }
}

/// 符号信息
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub defined_at: usize,
    pub statement: String,
}
