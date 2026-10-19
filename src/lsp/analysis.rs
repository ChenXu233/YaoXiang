//! LSP 文档分析（WBS 4.6.1）：诊断与语义数据出自**同一次编译**。
//!
//! 4.6.1 前，didOpen/didChange 触发**两趟**编译：诊断走 Driver 统一管线
//! （handlers/diagnostics.rs），语义数据走 server.rs 的手工
//! lex→parse→单文件 typecheck（无模块注册表，项目内 `use` 解析失败、
//! 跨文件引用无 `resolves_to`）。本模块把两趟并成一趟：
//!
//! ```text
//! 项目内文件 → orchestrator::analyze_source_in_project（Lsp 形态）
//! 单文件兜底 → Program{SingleFile, CollectAll} + Driver
//!                          ↓ 同一趟编译
//!            (Diagnostic[], SemanticDB)——后者经 remap_file_paths
//!            把 OS 路径键改写为规范化 file URI 键（uri.rs）
//! ```
//!
//! 语义数据面契约（driver/state.rs `DriverOutcome.semantic_db`）：目标
//! 文件的全量语义信息 + 非目标文件的顶层导出定义面；错误文件同样保留
//! （CollectAll——染色/跳转在打字中间态不消失，编辑器哲学同 4.2.6）。

use tracing::{debug, warn};

use crate::driver::{Aggregation, Driver, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;
use crate::frontend::core::typecheck::semantic_db::SemanticDB;
use crate::util::diagnostic::Diagnostic;

use super::uri::{path_to_uri, uri_to_path};

/// 分析一份打开的文档：一次编译同时产出诊断与语义数据（规范化 URI 键）。
pub(crate) fn analyze_document(
    uri: &str,
    content: &str,
) -> (Vec<Diagnostic>, SemanticDB) {
    let path = uri_to_path(uri);
    if let Some(p) = &path {
        use crate::frontend::module::orchestrator;
        if orchestrator::find_project_root(p).is_some() {
            match orchestrator::analyze_source_in_project(p, content) {
                Ok(analysis) => {
                    debug!(
                        "项目分析完成: {} ({} 条诊断)",
                        uri,
                        analysis.diagnostics.len()
                    );
                    let mut db = analysis.semantic_db;
                    db.remap_file_paths(&|f| path_to_uri(std::path::Path::new(f)));
                    return (analysis.diagnostics, db);
                }
                // 磁盘不可读等 → 退回单文件路径（同 4.2.3 的降级链）
                Err(e) => {
                    warn!("项目分析失败，退回单文件: {} - {}", uri, e);
                }
            }
        }
    }

    // 单文件兜底：与 CLI 同一 Driver（SingleFile + CollectAll，4.2.6）
    let unit_path = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| uri.to_string());
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("<lsp>", unit_path, content)],
        CompileConfig::default(),
    )
    .with_aggregation(Aggregation::CollectAll);
    let outcome = match Driver::new().run(program) {
        Ok(outcome) => outcome,
        Err(e) => {
            // 结构性不可达（SingleFile 恒 1 单元、各臂全接线）——防御：空结果
            warn!("driver 运行失败: {} - {}", uri, e);
            return (Vec::new(), SemanticDB::new());
        }
    };
    let diagnostics: Vec<Diagnostic> = outcome
        .result
        .errors
        .iter()
        .filter_map(|e| e.diagnostic())
        .chain(outcome.result.warnings.iter().cloned())
        .collect();
    let mut db = outcome.semantic_db;
    db.remap_file_paths(&|f| path_to_uri(std::path::Path::new(f)));

    debug!("单文件分析完成: {} ({} 条诊断)", uri, diagnostics.len());
    (diagnostics, db)
}
