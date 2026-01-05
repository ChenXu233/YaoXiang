# Task 4.9: 逃逸分析集成

> **优先级**: P2
> **状态**: ⏳ 待实现

## 功能描述

与逃逸分析模块集成，优化内存分配策略。

## 集成机制

```rust
struct EscapeInfo {
    escapes: bool,        // 是否逃逸到外部
    captured_by: Vec<String>,  // 被哪些闭包捕获
    modified: bool,       // 是否被修改
}
```

## 字节码优化

| 场景 | 分配策略 | 字节码变化 |
|------|----------|-----------|
| 不逃逸且不捕获 | 栈分配 | `ALLOCA` 代替 `ALLOC` |
| 逃逸到外部 | 堆分配 | 保留 `ALLOC` |
| 被闭包捕获 | 堆分配 | 添加 `CAPTURE` 指令 |
| 可变且不逃逸 | 栈分配+逃逸检查 | `ALLOCA` + `CHECK_ESCAPE` |

## 生成规则

### 栈分配优化
```yaoxiang
fn foo() {
    x = 42  # x 不逃逸，栈分配
    bar(x)
}
```
生成字节码（优化后）：
```
ALLOCA 42 -> r1  # 栈分配
LOAD r1 -> r2
CALL bar(r2)
```

### 逃逸检查
```yaoxiang
fn process() {
    data = [1, 2, 3]  # data 可能逃逸
    if condition {
        return data  # 逃逸
    }
    # data 不逃逸，栈分配
    sum(data)
}
```
生成字节码（带逃逸检查）：
```
# 初始假设栈分配
ALLOCA [1, 2, 3] -> r1

# 检查是否逃逸
TEST_ESCAPE r1 -> escaped
JUMP_IF_TRUE escaped -> heap_alloc

# 栈分配路径
JUMP -> continue

heap_alloc:
# 逃逸，改为堆分配
ALLOC [1, 2, 3] -> r1

continue:
```

## 验收测试

```yaoxiang
# test_escape_analysis.yx

# 不逃逸 - 栈分配
fn local_only() {
    x = compute_value()  # 栈分配优化
    process(x)
    # x 在此处释放
}

# 可能逃逸 - 堆分配
fn may_escape() -> List[Int] {
    data = [1, 2, 3]
    return data  # data 逃逸，堆分配
}

# 闭包捕获 - 堆分配
fn create_closure() {
    x = 42
    closure = || x + 1  # x 被捕获，堆分配
    call(closure)
}

print("Escape analysis integration tests passed!")
```

## 相关文件

- **generator.rs**: 逃逸分析集成逻辑
- **escape.rs**: 逃逸分析模块
