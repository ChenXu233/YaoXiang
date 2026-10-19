#!/usr/bin/env python3
"""list_ops — 列表操作（Python 对照实现）

与 yaoxiang / rust 版是**同一算法、同一操作序列**：建 1000 元素列表 → 过滤偶数 →
每个 x2 → 取首元素累加 limit 次。

三种语言都用**摊还 O(1) 追加**（yx `list.push` / python `append` / rust `push`），
故该对照的复杂度是对齐的——比值可以读作「同形状实现在各语言的常数差」。
（历史：yx 侧曾被迫用 `nums = nums + [j]` 逐次复制，那是当年的绕行，已改回 push。）
"""

import os


def main():
    limit = int(os.environ.get("BENCH_INPUT", "100"))
    total = 0
    for _ in range(limit):
        nums = []
        for j in range(1000):
            nums.append(j)

        filtered = []
        for n in nums:
            if n % 2 == 0:
                filtered.append(n)

        doubled = []
        for n in filtered:
            doubled.append(n * 2)

        total += doubled[0]

    print(total)


if __name__ == "__main__":
    main()
