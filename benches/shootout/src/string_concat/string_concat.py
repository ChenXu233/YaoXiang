#!/usr/bin/env python3
"""string_concat — 字符串拼接（Python 对照实现）

⚠ 刻意写成 `s = s + "Hello"` 而不是惯用的 `s += "Hello"`：
CPython 对引用计数为 1 的字符串在 `+=` 上有原地优化（摊还 O(1)），而 yaoxiang 的
`+` 每轮**重建整串**（O(n²)）。要对照就得同形状——否则这张表里量到的是复杂度
差异，不是语言差异。
"""

import os


def main():
    limit = int(os.environ.get("BENCH_INPUT", "500"))
    s = ""
    for _ in range(limit):
        s = s + "Hello"
        s = s + " "
        s = s + "World"
        s = s + "!"

    print(s)


if __name__ == "__main__":
    main()
