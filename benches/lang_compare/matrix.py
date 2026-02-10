#!/usr/bin/env python3
"""Matrix multiply - Python"""

def create_matrix(size: int):
    return [[i * j for j in range(size)] for i in range(size)]

def multiply(a, b, size: int):
    result = [[0 for _ in range(size)] for _ in range(size)]
    for i in range(size):
        for j in range(size):
            s = 0
            for k in range(size):
                s += a[i][k] * b[k][j]
            result[i][j] = s
    return result

def main():
    size = 20
    a = create_matrix(size)
    b = create_matrix(size)
    c = multiply(a, b, size)
    print(c[0][0])

if __name__ == "__main__":
    main()
