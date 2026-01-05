# Task 4.4: 函数调用字节码

> **优先级**: P0
> **状态**: ⏳ 待实现

## 功能描述

生成函数调用和返回的字节码，包括参数传递和返回值处理。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `CALL` | 函数调用 | |
| `CALL_INDIRECT` | 间接调用 | 函数指针调用 |
| `ENTER` | 进入函数 | 函数序言 |
| `LEAVE` | 离开函数 | 函数尾声 |
| `LOAD_PARAM` | 加载参数 | 加载函数参数 |

## 字节码格式

```rust
struct Call {
    func: FunctionRef,      // 函数引用
    args: Vec<Reg>,         // 参数寄存器
    result: Option<Reg>,    // 返回值寄存器
}

struct Enter {
    frame_size: usize,      // 栈帧大小
    local_count: usize,     // 局部变量数量
}

struct Leave {
    return_reg: Option<Reg>,
}
```

## 生成规则

### 直接调用
```yaoxiang
result = add(a, b)
```
生成字节码：
```
LOAD a -> r1
LOAD b -> r2
CALL add(r1, r2) -> r3
STORE r3 -> result
```

### 函数序言/尾声
```yaoxiang
fn foo(x: Int, y: Int): Int {
    z = x + y
    return z
}
```
生成字节码：
```
foo:
ENTER frame_size=3, local_count=1
LOAD_PARAM 0 -> r1  # x
LOAD_PARAM 1 -> r2  # y
ADD r1, r2 -> r3
STORE r3 -> z
RETURN r3
LEAVE
```

## 验收测试

```yaoxiang
# test_function_call_bytecode.yx

# 基本函数调用
add(a, b) = a + b
assert(add(1, 2) == 3)

# 嵌套调用
f(g(x)) = g(x) * 2
double(n) = n * 2
assert(f(double(5)) == 20)

# 递归
fact(n) = if n <= 1 { 1 } else { n * fact(n - 1) }
assert(fact(5) == 120)

# 高阶函数
apply(f, x) = f(x)
assert(apply(add, 5) == 8)

print("Function call bytecode tests passed!")
```

## 相关文件

- **bytecode.rs**: 函数调用指令定义
- **generator.rs**: 函数调用生成逻辑
