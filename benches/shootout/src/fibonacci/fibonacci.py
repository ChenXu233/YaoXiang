#!/usr/bin/env python3
"""Fibonacci iterative - Python
Reads BENCH_INPUT env var for input size, defaults to 1000"""

import os

def fibonacci(n: int) -> int:
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b

def main():
    n = int(os.environ.get("BENCH_INPUT", "1000"))
    result = 0
    for _ in range(10000):
        result = fibonacci(n)
    print(result)

if __name__ == "__main__":
    main()