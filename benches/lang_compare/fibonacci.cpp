// Fibonacci iterative - C++

#include <iostream>

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
    long long result = 0;
    for (int i = 0; i < 10000; i++) {
        result = fibonacci(1000);
    }
    return 0;
}
