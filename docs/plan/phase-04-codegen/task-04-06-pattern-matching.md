# Task 4.6: 模式匹配字节码

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

生成 match 表达式和模式匹配的字节码。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `MATCH` | 模式匹配 | |
| `MATCH_PAT` | 匹配模式 | |
| `PAT_TEST` | 测试模式 | 模式匹配测试 |

## 字节码格式

```rust
struct Match {
    expr_reg: Reg,
    arms: Vec<MatchArm>,
    result_reg: Reg,
}

struct MatchArm {
    pattern: Pattern,
    guard: Option<Reg>,  // 守卫条件
    body_start: Label,
}
```

## 模式类型

| 模式类型 | 字节码序列 |
|----------|-----------|
| 字面量 | `CONST` + `EQ` + `JUMP_IF_FALSE` |
| 标识符 | `JUMP`（总是匹配） |
| 构造器 | `PAT_TEST_CONSTRUCTOR` + 解包 |
| 元组 | `PAT_TEST_TUPLE` + 逐元素 |
| 通配符 | `JUMP` |

## 生成规则

### match 表达式
```yaoxiang
match opt {
    Some(n) => n * 2
    None => 0
}
```
生成字节码：
```
# 加载 opt 值
LOAD opt -> r1

# 测试 Some
PAT_TEST_CONSTRUCTOR r1, "Some" -> r2
JUMP_IF_FALSE r2 -> try_none
# 解包 Some
PAT_EXTRACT r1, "Some" -> n_reg
# 匹配成功，执行分支
n_reg -> r3
MUL r3, CONST(2) -> r4
JUMP -> match_end

try_none:
# 测试 None
PAT_TEST_CONSTRUCTOR r1, "None" -> r5
JUMP_IF_FALSE r5 -> match_fail
CONST 0 -> r6
JUMP -> match_end

match_fail:
# 无匹配 - 运行时错误
ERROR "NonExhaustivePatterns"

match_end:
```

## 验收测试

```yaoxiang
# test_pattern_matching_bytecode.yx

# 基础模式匹配
describe(n) = match n {
    0 => "zero"
    1 => "one"
    _ => "many"
}
assert(describe(0) == "zero")
assert(describe(5) == "many")

# 构造器模式
result = match Option::Some(42) {
    Some(n) => n * 2
    None => 0
}
assert(result == 84)

# 守卫模式
sign(n) = match n {
    x if x < 0 => "negative"
    0 => "zero"
    x => "positive"
}
assert(sign(-5) == "negative")

# 嵌套模式
result = match Result::Ok(Option::Some("value")) {
    Ok(Some(s)) => s
    _ => "default"
}
assert(result == "value")

print("Pattern matching bytecode tests passed!")
```

## 相关文件

- **bytecode.rs**: 模式匹配指令定义
- **generator.rs**: match 表达式生成逻辑
