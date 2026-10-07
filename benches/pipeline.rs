//! 管线冒烟基准（WBS 2.3.3 / 07-equivalence-oracle §145：性能基线入库）
//!
//! 07 §145 要求的三个冒烟基准及其归属：
//! - **全语料编译耗时**：本文件 `corpus/corpus_compile_frontend`
//! - **典型程序解释吞吐**：`benches/lib.rs` hotpath 组
//!   （`interp_fib_recursive_27` / `interp_loop_steady_10m`）已覆盖，不重复建设
//! - **CLI 冷启动**：本文件 `cli/cli_cold_start_hello`
//!
//! 用途（07 §145）：语料差分保证「行为相同」，不保证「耗时相同」。本基线是
//! 后续管线类改动（删槽位复用、Phi 展开为 Move 串、Driver 间接层，P4/P7/P8）
//! 的性能判据——回归 >10% 且 PR 无说明则拦（07 §182 check-perf-regression 预留）。
//!
//! ## 口径
//!
//! - 语料面与 IR 判据同口径（`src/middle/core/tests/corpus.rs`）：
//!   `tests/yaoxiang/**` + `src/std/tests/**`，skip 与 `expect: compile-error`
//!   不到达 IR 生成，不属编译耗时基线面。判定语义复用 `TestFileSpec` 单点（禁令三）。
//! - 每文件独立 `Compiler::new()`——语料文件编译相互隔离，
//!   复用实例会把跨文件状态带进测量。
//! - 源文本在 `iter` 外一次性读入（规范 11.2：输入不得在迭代内准备）。

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use yaoxiang::frontend::Compiler;
use yaoxiang::util::test_markers::{Expectation, TestFileSpec};

// ============================================================================
// 语料收集（bench crate 与 lib 测试、examples 是三个 crate，不可跨用，
// 就近复制收集逻辑——与 corpus.rs / corpus_probe.rs 同一先例形态）
// ============================================================================

fn collect_yx(
    dir: &Path,
    out: &mut Vec<PathBuf>,
) {
    if !dir.is_dir() {
        return;
    }
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yx(&path, out);
        } else if path.extension().is_some_and(|e| e == "yx") {
            out.push(path);
        }
    }
}

/// 语料文件全集（tests/yaoxiang/ + src/std/tests/，排序保证确定性）。
fn corpus_files() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_yx(&manifest.join("tests").join("yaoxiang"), &mut files);
    collect_yx(&manifest.join("src").join("std").join("tests"), &mut files);
    files.sort();
    files
}

/// IR 判据口径：skip / expect: compile-error 的文件不在基线面。
fn reaches_ir_generation(path: &Path) -> bool {
    let spec = TestFileSpec::parse(path);
    spec.skip_reason.is_none() && !matches!(spec.expectation, Expectation::CompileError(_))
}

// ============================================================================
// 基准：全语料编译耗时（前端 → IR 生成，不含 codegen/执行）
// ============================================================================

fn bench_corpus_compile_frontend(c: &mut Criterion) {
    let corpus: Vec<(PathBuf, String)> = corpus_files()
        .into_iter()
        .filter(|p| reaches_ir_generation(p))
        .map(|p| {
            let source = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("语料读取失败 {}: {e}", p.display()));
            (p, source)
        })
        .collect();
    assert!(
        corpus.len() > 200,
        "语料收集异常：仅 {} 个文件（预期 >200）——基线无意义",
        corpus.len()
    );

    c.bench_function("corpus_compile_frontend", |b| {
        b.iter(|| {
            let mut compiled = 0usize;
            for (path, source) in &corpus {
                let mut compiler = Compiler::new();
                let result = compiler.compile_with_source(&path.display().to_string(), source);
                black_box(&result);
                if result.is_ok() {
                    compiled += 1;
                }
            }
            black_box(compiled)
        })
    });
}

// ============================================================================
// 基准：CLI 冷启动（进程 spawn + 编译 + 执行最简程序）
// ============================================================================

fn bench_cli_cold_start_hello(c: &mut Criterion) {
    let binary = env!("CARGO_BIN_EXE_yaoxiang-rs");
    let hello = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("yaoxiang")
        .join("00-smoke")
        .join("hello.yx");
    // 防静默空跑：冷启动基准的前提是目标程序确实能跑通
    let probe = Command::new(binary)
        .arg("run")
        .arg(&hello)
        .output()
        .expect("CLI 冷启动探针 spawn 失败");
    assert!(
        probe.status.success(),
        "hello.yx 探针运行失败（退出码 {:?}）——冷启动基线无意义",
        probe.status.code()
    );

    c.bench_function("cli_cold_start_hello", |b| {
        b.iter(|| {
            let output = Command::new(black_box(binary))
                .arg("run")
                .arg(black_box(&hello))
                .output()
                .expect("CLI 冷启动 spawn 失败");
            black_box(output.status.code())
        })
    });
}

// ============================================================================
// Criterion Groups（规范 12.2：按主题分组，不挤在一个组里）
// ============================================================================

criterion_group!(
    name = corpus;
    config = Criterion::default().sample_size(10);
    targets = bench_corpus_compile_frontend
);

criterion_group!(
    name = cli;
    config = Criterion::default().sample_size(10);
    targets = bench_cli_cold_start_hello
);

criterion_main!(corpus, cli);
