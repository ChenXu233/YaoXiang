# Task 5.1: 基础逃逸分析

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

分析变量是否逃逸到当前作用域之外。

## 逃逸类型

| 类型 | 说明 | 分配策略 |
|------|------|----------|
| `NoEscape` | 不逃逸 | 栈分配 |
| `Escapes` | 逃逸到外部 | 堆分配 |
| `Captured` | 被闭包捕获 | 堆分配 |
| `Unknown` | 未知 | 保守堆分配 |

## 分析算法

```rust
struct EscapeAnalyzer {
    /// 当前作用域
    scope: Scope,
    /// 变量逃逸信息
    escape_info: HashMap<Var, EscapeInfo>,
    /// 闭包捕获变量
    captured_vars: HashSet<Var>,
}

enum EscapeInfo {
    NoEscape,
    Escapes {
        reason: EscapeReason,
    },
    Captured {
        by_closure: FunctionId,
    },
}
```

## 逃逸规则

```yaoxiang
fn local_var() {
    x = 42           # 不逃逸，栈分配
    print(x)
}

fn return_var() -> Int {
    x = 42           # 逃逸（返回），堆分配
    return x
}

fn closure_capture() {
    x = 42
    f = || x + 1     # x 被捕获，堆分配
    f()
}

fn global_escape() {
    x = [1, 2, 3]
    global_ptr = &x  # 逃逸到全局，堆分配
}
```

## 相关文件

- **mod.rs**: EscapeAnalyzer
- **analysis.rs**: 分析逻辑
