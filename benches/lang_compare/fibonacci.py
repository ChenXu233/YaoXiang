#!/usr/bin/env python3
"""Fibonacci iterative - Python"""

def fibonacci(n: int) -> int:
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b

def main():
    result = 0
    for _ in range(10000):
        result = fibonacci(1000)
    return result

if __name__ == "__main__":
    main()
