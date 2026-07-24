// Fibonacci iterative - Go
// Reads BENCH_INPUT env var for input size, defaults to 1000

package main

import (
    "fmt"
    "os"
    "strconv"
)

func fibonacci(n int) int {
    if n <= 1 {
        return n
    }
    a, b := 0, 1
    for i := 2; i <= n; i++ {
        a, b = b, a+b
    }
    return b
}

func main() {
    n := 1000
    if env := os.Getenv("BENCH_INPUT"); env != "" {
        if v, err := strconv.Atoi(env); err == nil && v > 0 {
            n = v
        }
    }

    result := 0
    for i := 0; i < 10000; i++ {
        result = fibonacci(n)
    }
    fmt.Println(result)
}