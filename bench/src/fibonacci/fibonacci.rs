//! Fibonacci iterative - Rust
//! Reads BENCH_INPUT env var for input size, defaults to 1000

use std::env;

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    let mut a = 0;
    let mut b = 1;
    for _ in 2..=n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    b
}

fn main() {
    let n: u64 = env::var("BENCH_INPUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let mut result = 0;
    for _ in 0..10000 {
        result = fibonacci(n);
    }
    println!("{}", result);
}