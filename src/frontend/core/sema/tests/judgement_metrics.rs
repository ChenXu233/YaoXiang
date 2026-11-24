//! 判定点量测（RFC-039 D62 / 6.9.3d 契约第 4 条）
//!
//! 为什么需要它：契约要求「一次判定一份预算」与「decide/defeq 调用面」是**实测数字**，
//! 不是估计。计数器只在测试构建里存在（`judgement::metrics`，生产零开销）。
//!
//! 运行方式（人工量测，不进 fast/full 预算）：
//! `cargo test --lib test_judgement_metrics_over_corpus -- --ignored --exact --nocapture`

use std::path::{Path, PathBuf};

use crate::frontend::compiler::Compiler;
use crate::frontend::core::sema::judgement::metrics;

/// 递归收集语料目录下的 `.yx` 文件（按路径排序，输出可复现）。
fn corpus_files(
    dir: &Path,
    out: &mut Vec<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            corpus_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "yx") {
            out.push(path);
        }
    }
}

/// 全量语料编译一次，打印判定点量测。
// cost: peak=1GiB time=180s mode=manual source=extrapolated@corpus-348-compile-run
#[test]
#[ignore = "cost: 全量语料编译（约 2-3 分钟），仅人工量测契约第 4 条的数字"]
fn test_judgement_metrics_over_corpus() {
    // Arrange
    metrics::reset();
    let mut files = Vec::new();
    corpus_files(Path::new("tests/yaoxiang"), &mut files);
    assert!(!files.is_empty(), "语料目录必须存在（否则量测数字无意义）");

    // Act
    let mut compiled = 0usize;
    for file in &files {
        let Ok(source) = std::fs::read_to_string(file) else {
            continue;
        };
        let mut compiler = Compiler::new();
        let _ = compiler.compile(&file.to_string_lossy(), &source);
        compiled += 1;
    }

    // Assert
    let (decide_calls, defeq_attempts, proved, steps_total, steps_max) = metrics::snapshot();
    println!(
        "JUDGEMENT_METRICS files={compiled} decide={decide_calls} defeq={defeq_attempts} \
         proved={proved} steps_total={steps_total} steps_max={steps_max}"
    );
    assert_eq!(compiled, files.len(), "每个语料文件都应编译一次");
    assert!(
        decide_calls >= defeq_attempts,
        "进入判等的次数不可能超过判定次数（decide={decide_calls} defeq={defeq_attempts}）"
    );
}
