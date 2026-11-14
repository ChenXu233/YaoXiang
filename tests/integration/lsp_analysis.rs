//! LSP 分析面深表达式锚（issue #540 / WBS 3.20.30）
//!
//! 规范来源：RFC-013 错误码规范 E0019 + D59 C 类崩溃面直修授权（用户裁决：
//! 编排层大栈线程 + 解析深度闸 1024）。
//!
//! LSP 形态的阶段表（Parsing → Registry → Typecheck → ProofExecution）在
//! 编辑器路径上同样吃深递归：修前 didOpen 深表达式（300 项）实测进程
//! 栈溢出（0xC00000FD，探针留档 target/yx-scratch/3.20.30-probe/——不入库）。
//! 修复两半在此面各锚一半：
//! - 解析闸（E0019）：超限树在解析期即被拒绝，LSP 专属走查
//!   （semantic_tokens 等）不再接触深树——本文件主锚；
//! - 大栈线程：LSP 服务器循环跑在 `COMPILE_STACK_SIZE` 线程上
//!   （main.rs 编排包裹），子进程级行为由 cli_e2e 的崩溃面锚覆盖。

#![cfg(feature = "cli")]

use std::path::PathBuf;
use tempfile::TempDir;
use yaoxiang::frontend::config::CompileConfig;
use yaoxiang::frontend::module::orchestrator::analyze_source_in_project;

#[test]
fn test_lsp_analysis_deep_expression_reports_e0019_without_crash() {
    // Arrange：1100 层一元链（闸值 1024）。修前 didOpen 该形态进程崩溃；
    // 修后解析闸拒绝并给出恰 1 条 E0019。分析走调用方线程——测试线程默认
    // 栈装不下解析闸允许的解析峰值，故镜像生产编排环境在测试内起
    // COMPILE_STACK_SIZE 线程（常量单源，见 lib.rs）。
    let expr = format!("{}1", "- ".repeat(1100));
    let source = format!("main = () => {{\n    y = {expr}\n    print(y)\n}}\n");
    let dir = TempDir::new().expect("temp dir");
    let path: PathBuf = dir.path().join("lsp_deep_unary.yx");
    std::fs::write(&path, &source).expect("write fixture");

    // Act
    let analysis = std::thread::Builder::new()
        .stack_size(yaoxiang::COMPILE_STACK_SIZE)
        .spawn(move || analyze_source_in_project(&path, &source, CompileConfig::default()))
        .expect("spawn analysis thread")
        .join()
        .expect("analysis thread must not panic")
        .expect("analysis must not fail orchestrally");

    // Assert：恰 1 条 E0019，无崩溃。残缺 AST 上可能再有少量降级诊断
    // （LSP 形态的既有设计：parse 错误收集后保留残缺 AST 继续 typecheck，
    // 见 driver parsing_lsp 臂）——但 E0019 自身必须恰 1 条。
    let e0019_count = analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == "E0019")
        .count();
    assert_eq!(e0019_count, 1, "恰 1 条 E0019: {:?}", analysis.diagnostics);
}
