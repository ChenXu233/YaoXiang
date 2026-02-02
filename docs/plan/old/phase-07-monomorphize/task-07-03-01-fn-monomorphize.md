# Task 7.3: 函数单态化

> **优先级**: P0
> **状态**: ✅ 已完成
> **依赖**: task-07-01, task-07-02
> **完成日期**: 2025-01-18

## 功能描述

将泛型函数实例化为具体函数。

## 函数单态化示例

```yaoxiang
# 泛型函数
map:[T, U](List[T], (T) -> U) -> List[U] = (list, f) => {
    match list {
        List(head, tail) => List(f(head), map(tail, f)),
        empty => empty,
    }
}

# 单态化后
map_int_string:(List[Int], (Int) -> String) -> List[String] = (list, f) => {
    match list {
        List(head, tail) => List(f(head), map_int_string(tail, f)),
        empty => empty,
    }
}
```

## 实现说明

**核心策略**：复用现有的 `instantiate_function` 和 `substitute_types`，新增公开 API。

### 主要 API

```rust
/// 单态化泛型函数（新增公开入口）
pub fn monomorphize_function(
    &mut self,
    generic_id: &GenericFunctionId,
    type_args: &[MonoType],
) -> Option<FunctionId>

/// 检查是否已单态化
pub fn is_function_monomorphized(&self, ...) -> bool

/// 获取已实例化的函数IR
pub fn get_instantiated_function(&self, ...) -> Option<&FunctionIR>

/// 获取已实例化的函数数量
pub fn instantiated_function_count(&self) -> usize
```

### 复用已有实现

| 已有实现 | 用途 |
|---------|------|
| `instantiate_function` | 函数实例化逻辑 |
| `substitute_types` | 类型替换 |
| `SpecializationKey` | 缓存键 |
| `should_specialize` | 特化上限控制 |

## 相关文件

- **[mod.rs](src/middle/monomorphize/mod.rs)**: 单态化器（新增 API）
- **[tests/fn_monomorphize.rs](src/middle/monomorphize/tests/fn_monomorphize.rs)**: 函数单态化测试（新增）
- **instance.rs**: 实例化请求与缓存键（已存在）

## 测试覆盖

- 简单泛型函数单态化
- 多参数泛型函数
- 同一泛型函数多次单态化
- 缓存命中验证
- 检查函数是否已单态化
- 获取已实例化的函数
- 不存在的泛型函数
- 非泛型函数
- 特化上限
- 不同类型参数顺序产生不同实例
- 单态化后的函数IR参数类型替换
