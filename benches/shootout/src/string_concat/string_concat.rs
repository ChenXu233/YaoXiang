// string_concat — 字符串拼接（Rust 对照实现）
//
// ⚠ 这里的 `.clone()` 是**刻意**的，不是笔误：Rust 的 `String + &str` 会复用原
// 缓冲（摊还 O(1)），而 yaoxiang / python 的 `s = s + frag` 每轮重建整串（O(n²)）。
// 不显式 clone，比值就退化成「复杂度差异」而非「同形状实现的常数差」。

use std::env;

fn main() {
    let limit: usize = env::var("BENCH_INPUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);

    let mut s = String::new();
    for _ in 0..limit {
        s = s.clone() + "Hello";
        s = s.clone() + " ";
        s = s.clone() + "World";
        s = s.clone() + "!";
    }

    println!("{}", s);
}
