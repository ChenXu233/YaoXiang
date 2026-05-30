// E:/git/YaoXiang/src/util/diagnostic/session.rs

use std::path::PathBuf;
use anyhow::Result;

use crate::frontend::module::dep_graph::ModuleDependencyGraph;
use crate::frontend::module::cache::{ModuleCache, CacheMode};
use super::CheckResult;

/// 检查会话 — 管理增量检查状态
pub struct CheckSession {
    dep_graph: ModuleDependencyGraph,
    #[allow(dead_code)]
    cache: ModuleCache,
    all_files: Vec<PathBuf>,
}

impl Default for CheckSession {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckSession {
    pub fn new() -> Self {
        Self {
            dep_graph: ModuleDependencyGraph::new(),
            cache: ModuleCache::new(CacheMode::Development),
            all_files: Vec::new(),
        }
    }

    /// 全量检查
    pub fn check_all(
        &mut self,
        files: &[PathBuf],
    ) -> Result<CheckResult> {
        self.all_files = files.to_vec();

        // Parse all files and build dependency graph
        let parsed = super::parse_files_parallel(files)?;
        for (_, module_id, ast) in &parsed {
            self.dep_graph.build_from_ast(module_id, ast);
        }

        super::check_files_with_diagnostics(files)
    }

    /// 增量检查 — 只检查受影响的模块
    pub fn check_incremental(
        &mut self,
        changed_files: &[PathBuf],
    ) -> Result<CheckResult> {
        let affected = self.dep_graph.affected_modules(changed_files);

        if affected.is_empty() {
            return Ok(CheckResult::default());
        }

        let files_to_check: Vec<PathBuf> = affected
            .iter()
            .filter_map(|module_id| module_id.path.clone())
            .collect();

        if files_to_check.is_empty() {
            return Ok(CheckResult::default());
        }

        super::check_files_with_diagnostics(&files_to_check)
    }

    /// 获取依赖图引用
    pub fn dep_graph(&self) -> &ModuleDependencyGraph {
        &self.dep_graph
    }

    /// 获取所有已注册的文件列表
    pub fn all_files(&self) -> &[PathBuf] {
        &self.all_files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_session_new() {
        let session = CheckSession::new();
        assert!(session.all_files().is_empty());
    }

    #[test]
    fn test_check_all_single_file() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let file = dir.path().join("test.yx");
        std::fs::write(
            &file,
            r#"use std.io

main: () -> Void = {
    print("hello")
}
"#,
        )
        .expect("write file");

        let mut session = CheckSession::new();
        let result = session.check_all(&[file]).expect("check all");
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn test_check_incremental_no_changes() {
        let mut session = CheckSession::new();
        let result = session.check_incremental(&[]).expect("check incremental");
        assert_eq!(result.error_count, 0);
    }
}
