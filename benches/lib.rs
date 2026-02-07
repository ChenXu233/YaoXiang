//! # YaoXiang 性能基准测试
//!
//! 使用 Criterion.rs 进行性能基准测试。
//!
//! ## 基准测试分组
//! - `micro`: Rust 微基准测试（CPU 运算）
//! - `yaoxiang`: YaoXiang 解释器性能测试
//! - `interpreter`: 解释器性能测试
//! - `codegen`: 编译器效率测试
//!
//! ## 使用方法
//! ```bash
//! cargo bench          # 运行所有
//! cargo bench micro    # 只运行微基准
//! cargo bench yaoxiang # 只运行 YaoXiang 测试
//! ```

use criterion::{criterion_group, criterion_main, Criterion};

// ============================================================================
// Micro Benchmarks - Rust 底层运算基准
// ============================================================================

fn bench_add(c: &mut Criterion) {
    c.bench_function("add", |b| {
        b.iter(|| {
            let mut r = 0i64;
            for i in 0..1000 {
                r += i;
            }
            r
        })
    });
}

fn bench_mul(c: &mut Criterion) {
    c.bench_function("mul", |b| {
        b.iter(|| {
            let mut r = 1i64;
            for i in 1..100 {
                r *= i;
            }
            r
        })
    });
}

fn bench_vec_push(c: &mut Criterion) {
    c.bench_function("vec_push", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..1000 {
                v.push(i);
            }
            v
        })
    });
}

fn bench_fibonacci_rust(c: &mut Criterion) {
    c.bench_function("fibonacci_iterative_rust", |b| {
        b.iter(|| {
            let mut a = 0i64;
            let mut b_val = 1i64;
            for _ in 0..20 {
                let temp = a + b_val;
                a = b_val;
                b_val = temp;
            }
            a
        })
    });
}

fn bench_matrix_rust(c: &mut Criterion) {
    c.bench_function("matrix_multiply_10x10_rust", |b| {
        b.iter(|| {
            let size = 10;
            let mut a = vec![vec![0i64; size]; size];
            let mut b = vec![vec![0i64; size]; size];
            let mut c = vec![vec![0i64; size]; size];

            for i in 0..size {
                for j in 0..size {
                    a[i][j] = (i + j) as i64;
                    b[i][j] = (i * j) as i64;
                }
            }

            for i in 0..size {
                for j in 0..size {
                    let mut sum = 0i64;
                    for k in 0..size {
                        sum += a[i][k] * b[k][j];
                    }
                    c[i][j] = sum;
                }
            }
            c
        })
    });
}

// ============================================================================
// YaoXiang Interpreter Benchmarks - YaoXiang 解释器性能
// ============================================================================

fn bench_yaoxiang_fibonacci(c: &mut Criterion) {
    let source = std::fs::read_to_string("benches/yx_benchmarks/fibonacci.yx")
        .expect("Cannot read fibonacci.yx");

    // 禁用日志以减少噪音
    let _ = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::ERROR)
        .try_init();

    c.bench_function("yaoxiang_fibonacci_iterative", |b| {
        b.iter(|| {
            yaoxiang::run(&source).expect("YaoXiang execution failed");
        })
    });
}

fn bench_yaoxiang_matrix(c: &mut Criterion) {
    let source =
        std::fs::read_to_string("benches/yx_benchmarks/matrix.yx").expect("Cannot read matrix.yx");

    let _ = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::ERROR)
        .try_init();

    c.bench_function("yaoxiang_matrix_multiply_20x20", |b| {
        b.iter(|| {
            yaoxiang::run(&source).expect("YaoXiang execution failed");
        })
    });
}

fn bench_yaoxiang_list_ops(c: &mut Criterion) {
    let source = std::fs::read_to_string("benches/yx_benchmarks/list_ops.yx")
        .expect("Cannot read list_ops.yx");

    let _ = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::ERROR)
        .try_init();

    c.bench_function("yaoxiang_list_operations", |b| {
        b.iter(|| {
            yaoxiang::run(&source).expect("YaoXiang execution failed");
        })
    });
}

fn bench_yaoxiang_string_concat(c: &mut Criterion) {
    let source = std::fs::read_to_string("benches/yx_benchmarks/string_concat.yx")
        .expect("Cannot read string_concat.yx");

    let _ = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::ERROR)
        .try_init();

    c.bench_function("yaoxiang_string_concat", |b| {
        b.iter(|| {
            yaoxiang::run(&source).expect("YaoXiang execution failed");
        })
    });
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    name = micro;
    config = Criterion::default().sample_size(50);
    targets = bench_add, bench_mul, bench_vec_push
);

criterion_group!(
    name = yaoxiang;
    config = Criterion::default().sample_size(10);
    targets = bench_yaoxiang_fibonacci, bench_yaoxiang_matrix, bench_yaoxiang_list_ops, bench_yaoxiang_string_concat
);

criterion_group!(
    name = interpreter;
    config = Criterion::default().sample_size(30);
    targets = bench_fibonacci_rust, bench_matrix_rust
);

criterion_main!(micro, yaoxiang, interpreter);

// TODO: 添加更多基准测试，例如编译器效率测试、内存使用基准等。修复语言原始问题等。
