#!/usr/bin/env python3
"""
YaoXiang 语言性能对比脚本

对比 YaoXiang 与 Python、Rust、Go、C++ 的性能表现。
"""

import subprocess
import json
import time
import os
import sys
from dataclasses import dataclass
from typing import Dict, List, Optional
import tempfile

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)


@dataclass
class BenchmarkResult:
    name: str
    yaoxiang_ms: float
    python_ms: float
    rust_ms: float
    cpp_ms: float
    go_ms: float

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "yaoxiang": self.yaoxiang_ms,
            "python": self.python_ms,
            "rust": self.rust_ms,
            "cpp": self.cpp_ms,
            "go": self.go_ms,
        }


def run_command(cmd: List[str], timeout: int = 60) -> tuple:
    """运行命令并返回 (stdout, stderr, returncode)"""
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired:
        return "", "Timeout", -1
    except Exception as e:
        return "", str(e), -1


def get_yaoxiang_path() -> str:
    """获取 yaoxiang 可执行文件路径"""
    release_path = os.path.join(PROJECT_ROOT, "target", "release", "yaoxiang")
    debug_path = os.path.join(PROJECT_ROOT, "target", "debug", "yaoxiang")

    if os.path.exists(release_path):
        return release_path
    elif os.path.exists(debug_path):
        return debug_path
    else:
        return "yaoxiang"  # 假设在 PATH 中


def benchmark_language(code: str, lang: str, iterations: int = 100) -> float:
    """运行指定语言的代码并计时"""
    temp_file = None
    try:
        if lang == "python":
            ext = ".py"
            cmd = [sys.executable]
        elif lang == "rust":
            ext = ".rs"
            cmd = ["rustc", "-O"]
        elif lang == "cpp":
            ext = ".cpp"
            cmd = ["g++", "-O2"]
        elif lang == "go":
            ext = ".go"
            cmd = ["go", "build", "-o"]
        elif lang == "yaoxiang":
            ext = ".yx"
            cmd = [get_yaoxiang_path()]
        else:
            raise ValueError(f"Unknown language: {lang}")

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=ext, delete=False
        ) as f:
            f.write(code)
            f.flush()
            temp_file = f.name

        # 特殊处理：Go 需要先编译再运行
        if lang == "go":
            bin_file = temp_file + ".bin"
            compile_result = run_command(["go", "build", "-o", bin_file, temp_file])
            if compile_result[2] != 0:
                return float("inf")
            cmd = [bin_file]
        # C++ 和 Rust 需要编译
        elif lang in ["cpp", "rust"]:
            bin_file = temp_file + ".bin"
            compile_cmd = cmd + ["-o", bin_file, temp_file]
            compile_result = run_command(compile_cmd)
            if compile_result[2] != 0:
                return float("inf")
            cmd = [bin_file]
        else:
            cmd.append(temp_file)

        # 运行基准测试
        start = time.perf_counter()
        for _ in range(iterations):
            run_command(cmd)
        elapsed = time.perf_counter() - start
        return (elapsed / iterations) * 1000  # 转换为毫秒

    finally:
        if temp_file and os.path.exists(temp_file):
            os.unlink(temp_file)
        # 清理编译产物
        if temp_file:
            for ext in [".bin", ".exe"]:
                bin_file = temp_file + ext
                if os.path.exists(bin_file):
                    os.unlink(bin_file)


# ============ 基准测试程序 ============

BENCHMARKS = {
    "fibonacci_iterative": {
        "name": "斐波那契迭代 (n=30, 1000次)",
        "python": '''
def fib(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

for _ in range(1000):
    fib(30)
''',
        "rust": '''
fn fib(n: u64) -> u64 {
    if n <= 1 { return n }
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 0..n {
        let temp = a;
        a = b;
        b = temp + b;
    }
    a
}

fn main() {
    for _ in 0..1000 {
        fib(30);
    }
}
''',
        "cpp": '''
#include <iostream>
long long fib(int n) {
    if (n <= 1) return n;
    long long a = 0, b = 1;
    for (int i = 0; i < n; i++) {
        long long temp = a;
        a = b;
        b = temp + b;
    }
    return a;
}

int main() {
    for (int i = 0; i < 1000; i++) {
        fib(30);
    }
    return 0;
}
''',
        "go": '''
package main

import "fmt"

func fib(n int) int {
    if n <= 1 {
        return n
    }
    a, b := 0, 1
    for i := 0; i < n; i++ {
        a, b = b, a+b
    }
    return a
}

func main() {
    for i := 0; i < 1000; i++ {
        fib(30)
    }
}
''',
        "yaoxiang": '''
fib: (n: Int) -> Int = {
    if n <= 1 { return n }
    let mut a = 0
    let mut b = 1
    for _ in 0..n {
        let temp = a
        a = b
        b = temp + b
    }
    a
}

main: () -> Void = {
    for _ in 0..1000 {
        fib(30)
    }
}
''',
    },
    "fibonacci_recursive": {
        "name": "斐波那契递归 (n=20)",
        "python": '''
def fib(n):
    if n <= 1:
        return n
    return fib(n-1) + fib(n-2)

fib(20)
''',
        "rust": '''
fn fib(n: u64) -> u64 {
    if n <= 1 { n } else { fib(n-1) + fib(n-2) }
}

fn main() {
    fib(20);
}
''',
        "cpp": '''
int fib(int n) {
    if (n <= 1) return n;
    return fib(n-1) + fib(n-2);
}

int main() {
    fib(20);
    return 0;
}
''',
        "go": '''
package main

import "fmt"

func fib(n int) int {
    if n <= 1 {
        return n
    }
    return fib(n-1) + fib(n-2)
}

func main() {
    fib(20)
    fmt.Println(fib(20))
}
''',
        "yaoxiang": '''
fib: (n: Int) -> Int = if n <= 1 { n } else { fib(n-1) + fib(n-2) }

main: () -> Void = {
    fib(20)
}
''',
    },
    "matrix_multiply": {
        "name": "矩阵乘法 (20x20, 50次)",
        "python": '''
def mat_mul(a, b):
    n = len(a)
    result = [[0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            s = 0
            for k in range(n):
                s += a[i][k] * b[k][j]
            result[i][j] = s
    return result

n = 20
a = [[i * j for j in range(n)] for i in range(n)]
b = [[i + j for j in range(n)] for i in range(n)]

for _ in range(50):
    mat_mul(a, b)
''',
        "rust": '''
fn mat_mul(a: &Vec<Vec<i64>>, b: &Vec<Vec<i64>>, n: usize) -> Vec<Vec<i64>> {
    let mut result = vec![vec![0i64; n]; n];
    for i in 0..n {
        for j in 0..n {
            let mut s = 0i64;
            for k in 0..n {
                s += a[i][k] * b[k][j];
            }
            result[i][j] = s;
        }
    }
    result
}

fn main() {
    let n = 20usize;
    let mut a = vec![vec![0i64; n]; n];
    let mut b = vec![vec![0i64; n]; n];
    for i in 0..n {
        for j in 0..n {
            a[i][j] = (i * j) as i64;
            b[i][j] = (i + j) as i64;
        }
    }
    for _ in 0..50 {
        mat_mul(&a, &b, n);
    }
}
''',
        "cpp": '''
#include <iostream>
#include <vector>
using namespace std;

vector<vector<long long>> mat_mul(const vector<vector<long long>>& a,
                                  const vector<vector<long long>>& b) {
    int n = a.size();
    vector<vector<long long>> result(n, vector<long long>(n, 0));
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            long long s = 0;
            for (int k = 0; k < n; k++) {
                s += a[i][k] * b[k][j];
            }
            result[i][j] = s;
        }
    }
    return result;
}

int main() {
    int n = 20;
    vector<vector<long long>> a(n, vector<long long>(n));
    vector<vector<long long>> b(n, vector<long long>(n));
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            a[i][j] = i * j;
            b[i][j] = i + j;
        }
    }
    for (int t = 0; t < 50; t++) {
        mat_mul(a, b);
    }
    return 0;
}
''',
        "go": '''
package main

import "fmt"

func matMul(a, b [][]int) [][]int {
    n := len(a)
    result := make([][]int, n)
    for i := 0; i < n; i++ {
        result[i] = make([]int, n)
        for j := 0; j < n; j++ {
            var s int
            for k := 0; k < n; k++ {
                s += a[i][k] * b[k][j]
            }
            result[i][j] = s
        }
    }
    return result
}

func main() {
    n := 20
    a := make([][]int, n)
    b := make([][]int, n)
    for i := 0; i < n; i++ {
        a[i] = make([]int, n)
        b[i] = make([]int, n)
        for j := 0; j < n; j++ {
            a[i][j] = i * j
            b[i][j] = i + j
        }
    }
    for t := 0; t < 50; t++ {
        matMul(a, b)
    }
    fmt.Println("done")
}
''',
        "yaoxiang": '''
mat_mul: (a: [[Int]], b: [[Int]]) -> [[Int]] = {
    let n = len(a)
    let result = [[0; n]; n]
    for i in 0..n {
        for j in 0..n {
            let mut sum = 0
            for k in 0..n {
                sum = sum + a[i][k] * b[k][j]
            }
            result[i][j] = sum
        }
    }
    result
}

main: () -> Void = {
    let n = 20
    let a = [[i * j; n]; n]
    let b = [[i + j; n]; n]
    for _ in 0..50 {
        mat_mul(a, b)
    }
}
''',
    },
    "list_operations": {
        "name": "列表操作 (1000次创建和遍历)",
        "python": '''
for _ in range(1000):
    lst = [i for i in range(100)]
    total = sum(lst)
''',
        "rust": '''
fn main() {
    for _ in 0..1000 {
        let lst: Vec<i64> = (0..100).collect();
        let total: i64 = lst.iter().sum();
    }
}
''',
        "cpp": '''
#include <iostream>
int main() {
    for (int t = 0; t < 1000; t++) {
        int lst[100];
        int total = 0;
        for (int i = 0; i < 100; i++) {
            lst[i] = i;
            total += i;
        }
    }
    return 0;
}
''',
        "go": '''
package main

func main() {
    for _ := 0; i < 1000; i++ {
        lst := make([]int, 100)
        var total int
        for i := 0; i < 100; i++ {
            lst[i] = i
            total += i
        }
    }
}
''',
        "yaoxiang": '''
main: () -> Void = {
    for _ in 0..1000 {
        let lst = [i; 100]
        let sum = 0
        for x in lst {
            sum = sum + x
        }
        sum
    }
}
''',
    },
    "string_concat": {
        "name": "字符串拼接 (1000次)",
        "python": '''
for _ in range(1000):
    s = ""
    for i in range(100):
        s += str(i) + ","
''',
        "rust": '''
fn main() {
    for _ in 0..1000 {
        let mut s = String::new();
        for i in 0..100 {
            s.push_str(&i.to_string());
            s.push(',');
        }
    }
}
''',
        "cpp": '''
#include <iostream>
#include <string>
int main() {
    for (int t = 0; t < 1000; t++) {
        std::string s;
        for (int i = 0; i < 100; i++) {
            s += std::to_string(i);
            s += ",";
        }
    }
    return 0;
}
''',
        "go": '''
package main

func main() {
    for _ := 0; i < 1000; i++ {
        var s string
        for i := 0; i < 100; i++ {
            s += string(rune(i))
        }
    }
}
''',
        "yaoxiang": '''
main: () -> Void = {
    for _ in 0..1000 {
        let mut s = ""
        for i in 0..100 {
            s = s + str(i) + ","
        }
    }
}
''',
    },
}


def run_benchmarks() -> List[BenchmarkResult]:
    """运行所有基准测试"""
    results = []
    iterations = 100

    for key, bench in BENCHMARKS.items():
        print(f"Running benchmark: {bench['name']}...")

        result = BenchmarkResult(
            name=bench["name"],
            yaoxiang_ms=benchmark_language(bench["yaoxiang"], "yaoxiang", iterations),
            python_ms=benchmark_language(bench["python"], "python", iterations),
            rust_ms=benchmark_language(bench["rust"], "rust", iterations),
            cpp_ms=benchmark_language(bench["cpp"], "cpp", iterations),
            go_ms=benchmark_language(bench["go"], "go", iterations),
        )
        results.append(result)

        print(f"  YaoXiang: {result.yaoxiang_ms:.2f}ms")
        print(f"  Python:   {result.python_ms:.2f}ms")
        print(f"  Rust:     {result.rust_ms:.2f}ms")
        print(f"  C++:      {result.cpp_ms:.2f}ms")
        print(f"  Go:       {result.go_ms:.2f}ms")

    return results


def save_results(results: List[BenchmarkResult], output_path: str):
    """保存结果到 JSON 文件"""
    output = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "iterations": 100,
        "benchmarks": [r.to_dict() for r in results],
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\nResults saved to: {output_path}")


def main():
    """主函数"""
    import argparse

    parser = argparse.ArgumentParser(
        description="YaoXiang Language Performance Comparison"
    )
    parser.add_argument(
        "--output", "-o", default="compare_results.json", help="Output JSON file path"
    )
    parser.add_argument(
        "--iterations", "-i", type=int, default=100, help="Number of iterations"
    )
    args = parser.parse_args()

    print("=" * 60)
    print("YaoXiang Language Performance Comparison")
    print("=" * 60)
    print(f"Running with {args.iterations} iterations per test...\n")

    results = run_benchmarks()
    save_results(results, args.output)

    print("\n" + "=" * 60)
    print("Done!")
    print("=" * 60)


if __name__ == "__main__":
    main()
