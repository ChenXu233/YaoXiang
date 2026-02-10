// Fibonacci iterative - Go

package main

import "fmt"

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
    result := 0
    for i := 0; i < 10000; i++ {
        result = fibonacci(1000)
    }
    fmt.Println(result)
}
