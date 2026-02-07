//! Fibonacci iterative - Rust

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
    let mut result = 0;
    for _ in 0..10000 {
        result = fibonacci(1000);
    }
}
