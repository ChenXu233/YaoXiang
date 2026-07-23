// Fibonacci iterative - C++
// Reads BENCH_INPUT env var for input size, defaults to 1000

#include <iostream>
#include <cstdlib>

long long fibonacci(int n) {
    if (n <= 1) return n;
    long long a = 0, b = 1;
    for (int i = 2; i <= n; i++) {
        long long temp = a + b;
        a = b;
        b = temp;
    }
    return b;
}

int main() {
    const char* env = std::getenv("BENCH_INPUT");
    int n = env ? std::atoi(env) : 1000;
    if (n <= 0) n = 1000;

    long long result = 0;
    for (int i = 0; i < 10000; i++) {
        result = fibonacci(n);
    }
    std::cout << result << std::endl;
    return 0;
}