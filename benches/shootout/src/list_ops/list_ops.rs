// list_ops — 列表操作（Rust 对照实现）
//
// 与 yaoxiang / python 版是**同一算法、同一操作序列**：建 1000 元素列表 →
// 过滤偶数 → 每个 x2 → 取首元素累加 limit 次。
//
// 三种语言都用**摊还 O(1) 追加**（yx `list.push` / python `append` / rust `push`），
// 故该对照的复杂度是对齐的——比值可以读作「同形状实现在各语言的常数差」。

use std::env;

fn main() {
    let limit: usize = env::var("BENCH_INPUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let mut total: i64 = 0;
    for _ in 0..limit {
        let mut nums: Vec<i64> = Vec::new();
        for j in 0..1000i64 {
            nums.push(j);
        }

        let mut filtered: Vec<i64> = Vec::new();
        for n in &nums {
            if n % 2 == 0 {
                filtered.push(*n);
            }
        }

        let mut doubled: Vec<i64> = Vec::new();
        for n in &filtered {
            doubled.push(n * 2);
        }

        total += doubled[0];
    }

    println!("{}", total);
}
