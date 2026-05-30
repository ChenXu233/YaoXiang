//! Formatter 基准测试 — 基于测试规范 §10

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use yaoxiang::formatter::{format_source, FormatOptions};

fn bench_format_small(c: &mut Criterion) {
    let source = "let x = 1\nlet y = 2\nlet z = x + y";
    c.bench_function("format_small", |b| {
        b.iter(|| format_source(black_box(source), &FormatOptions::default()))
    });
}

fn bench_format_medium(c: &mut Criterion) {
    let source = r#"
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    let result = fibonacci(10)
    print(result)
}
"#;
    c.bench_function("format_medium", |b| {
        b.iter(|| format_source(black_box(source), &FormatOptions::default()))
    });
}

criterion_group!(benches, bench_format_small, bench_format_medium);
criterion_main!(benches);
