# Task 1.4: 注释处理

> **优先级**: P0
> **状态**: ✅ 已完成

## 功能描述

识别并跳过源代码中的注释。

## 注释类型

### 单行注释

```yaoxiang
// 这是一个单行注释
x = 42  // 行尾注释
```

### 多行注释

```yaoxiang
/*
 * 这是一个多行注释
 * 可以跨越多行
 */
```

### 嵌套注释（可选）

```yaoxiang
/*
外层 /* 内层 */ 继续 */
```

## 验收测试

```yaoxiang
# test_comments.yx

// 单行注释不应影响代码
x = 42  // 这是一个注释
assert(x == 42)

/*
 * 多行注释
 * 第二行
 */
y = 100
assert(y == 100)

# 注释中的代码不应执行
// z = 999
# w = 888

/*
result = 12345  // 被注释的代码
assert(result != 12345)  // 不会执行
*/

print("All comment tests passed!")
```

## 相关文件

- **mod.rs**: skip_whitespace_and_comments() 实现
