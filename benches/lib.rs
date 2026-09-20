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
            // 用 wrapping_mul：100! 远超 i64，普通 `*=` 在 debug 下直接 panic
            // （`cargo test --bench` 会触发），且溢出行为并非本基准测量目标。
            // 本基准测的是乘法指令的 CPU 成本，结果值无意义。
            let mut r = 1i64;
            for i in 1..100i64 {
                r = r.wrapping_mul(i);
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
    let source = std::fs::read_to_string("benches/shootout/src/fibonacci/fibonacci.yx")
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
    let source = std::fs::read_to_string("benches/shootout/src/matrix/matrix.yx")
        .expect("Cannot read matrix.yx");

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
    let source = std::fs::read_to_string("benches/shootout/src/list_ops/list_ops.yx")
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
    let source = std::fs::read_to_string("benches/shootout/src/string_concat/string_concat.yx")
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
// 解释器热路径基准 — 只测「执行」段，不含编译/codegen
// ============================================================================
//
// 为什么不用 `yaoxiang::run()`：`run_with_source_name`（src/lib.rs:104）把
// 编译 + codegen + 执行三件事串在一起，解释器信号会被编译开销稀释，无法
// 判断热路径优化的真实收益。此处预编译一次，`iter()` 内只跑执行段。
//
// 为什么每次迭代新建 `Interpreter`：`execute_module` 会累积状态
// （`constants.extend` + `functions_by_id.push`，src/backends/interpreter/
// executor/execute.rs:23-29），复用同一实例会让函数表不断膨胀、测量失真。
//
// 为什么源码里不写 `println` 且以 `main()` 结尾：
//   - `println` 走 `native_println` 直写 stdout（src/std/io.rs:143），污染测量；
//   - 入口语义在 9916518b 后只认 `module.entry_function`，Script 模式下
//     顶层语句/绑定本身不构成入口——必须以 `main()` 显式调用结尾，
//     否则会静默测到零工作量（本 bench 用返回值断言防此类回归）。

use yaoxiang::backends::Executor;
use yaoxiang::frontend::Compiler;
use yaoxiang::middle::passes::codegen::CodegenContext;
use yaoxiang::Interpreter;

/// 编译 YaoXiang 源码为 BytecodeModule（bench 外只跑一次）
fn compile_to_module(source: &str) -> yaoxiang::middle::bytecode::BytecodeModule {
    let mut compiler = Compiler::new();
    let module = compiler
        .compile_with_source("<bench>", source)
        .expect("bench source failed to compile");
    let mut ctx = CodegenContext::new(module);
    let bytecode_file = ctx.generate().expect("bench codegen failed");
    yaoxiang::middle::bytecode::BytecodeModule::from(bytecode_file)
}

/// 跑一次执行段，返回第 0 号全局槽位的值（用于防静默空跑断言）
///
/// 为何不用入口返回值：`execute_module` 返回 `()`（src/backends/mod.rs:358），
/// 且 `main()` 的调用发生在模块初始化序列（`__yx_module_init`）中，入口函数
/// 自身只返回 Void。顶层绑定的求值结果落在全局槽位，此处从 `global_slot(0)` 读回。
fn execute_once(module: &yaoxiang::middle::bytecode::BytecodeModule) -> yaoxiang::RuntimeValue {
    let mut interp = Interpreter::new();
    interp
        .execute_module(module)
        .expect("bench execution failed");
    interp
        .global_slot(0)
        .cloned()
        .expect("bench: 全局槽位 0 不存在（bench 可能静默空跑）")
}

/// 递归 fib — 测函数调用路径（`Frame` 深拷贝 + 每条指令的栈帧信息分配）
///
/// n=27 约 196418；调用次数约 63 万，单次执行量级适合 Criterion 采样。
/// 顶层绑定 `result: Int = fib(27)` 使结果落入全局槽位 0 供断言读取。
const SRC_FIB_RECURSIVE: &str = r#"
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n
  }
  return fib(n - 1) + fib(n - 2)
}

result: Int = fib(27)

main: () -> Void = {
  result
}
main()
"#;

/// 稳态循环 — 测每条指令的固定税（Frame memmove + 指令 clone）
///
/// 10⁷ 次迭代；无函数调用、无堆分配，信号纯净。
const SRC_LOOP_STEADY: &str = r#"
accumulate: () -> Int = {
  mut s = 0
  mut i = 0
  while i < 10000000 {
    s = s + i
    i = i + 1
  }
  s
}

result: Int = accumulate()

main: () -> Void = {
  result
}
main()
"#;

fn bench_interp_fib_recursive(c: &mut Criterion) {
    let module = compile_to_module(SRC_FIB_RECURSIVE);

    // 防静默空跑：确认递归真的跑了（fib(27) = 196418）
    // 若入口语义变更使 main() 不再被调用，此处立即失败而非静默测 0
    match execute_once(&module) {
        yaoxiang::RuntimeValue::Int(n) => assert_eq!(n, 196418, "fib(27) 结果不符"),
        other => panic!("全局槽位 0 非预期 Int，实际 {other:?}（bench 可能静默空跑）"),
    }

    c.bench_function("interp_fib_recursive_27", |b| {
        b.iter(|| execute_once(&module))
    });
}

fn bench_interp_loop_steady(c: &mut Criterion) {
    let module = compile_to_module(SRC_LOOP_STEADY);

    // 防静默空跑：循环确实跑完（sum(0..10⁷) = 49999995000000）
    match execute_once(&module) {
        yaoxiang::RuntimeValue::Int(n) => {
            assert_eq!(n, 49_999_995_000_000, "循环累加结果不符")
        }
        other => panic!("全局槽位 0 非预期 Int，实际 {other:?}（bench 可能静默空跑）"),
    }

    c.bench_function("interp_loop_steady_10m", |b| {
        b.iter(|| execute_once(&module))
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
    targets = bench_yaoxiang_fibonacci, bench_yaoxiang_matrix, bench_yaoxiang_string_concat, bench_yaoxiang_list_ops
);

criterion_group!(
    name = interpreter;
    config = Criterion::default().sample_size(30);
    targets = bench_fibonacci_rust, bench_matrix_rust
);

// 热路径基准：样本量压到 10（单次执行秒级，全量采样过慢）
criterion_group!(
    name = hotpath;
    config = Criterion::default().sample_size(10);
    targets = bench_interp_fib_recursive, bench_interp_loop_steady
);

criterion_main!(micro, yaoxiang, interpreter, hotpath);

// TODO: 添加更多基准测试，例如编译器效率测试、内存使用基准等。修复语言原始问题等。
