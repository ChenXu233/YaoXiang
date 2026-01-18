# Task 7.2: 类型单态化

> **优先级**: P0
> **状态**: ✅ 已完成
> **依赖**: task-07-01
> **完成日期**: 2026-01-18

## 功能描述

将泛型类型实例化为具体类型，支持：
- 泛型结构体单态化 (`struct Point[T] { x: T, y: T }`)
- 泛型枚举单态化 (`enum Option[T] { Some(T), None }`)
- Arc 类型处理（所有权模型要求）
- 类型缓存避免重复单态化

## 验证结果

```bash
cargo test --lib monomorphize::tests
# test result: ok. 68 passed; 0 failed
#   - type_monomorphize: 13 tests
#   - instance: 55 tests (含类型相关测试)
```

## 实现位置

| 文件 | 说明 |
|------|------|
| `src/middle/monomorphize/mod.rs` | 类型单态化核心实现（668-1293行） |
| `src/middle/monomorphize/tests/type_monomorphize.rs` | 类型单态化单元测试（13项） |
| `src/middle/monomorphize/tests/instance.rs` | 数据结构单元测试（55项） |

## 核心 API

```rust
impl Monomorphizer {
    /// 单态化泛型类型
    pub fn monomorphize_type(
        &mut self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> Option<MonoType>

    /// 实例化具体类型
    fn instantiate_type(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
        generic_type: &MonoType,
    ) -> Option<MonoType>

    /// 递归替换类型中的泛型参数
    fn substitute_type_args(
        &self,
        ty: &MonoType,
        type_args: &[MonoType],
        type_params: &[String],
    ) -> MonoType

    /// 注册单态化后的类型
    pub fn register_monomorphized_type(
        &mut self,
        mono_type: MonoType,
    ) -> TypeId
}
```

## 类型单态化示例

```yaoxiang
# 泛型类型定义
type Option[T] = some(T) | none
type Point[T] = struct { x: T, y: T }

# 单态化后
type Option_Int = some(Int) | none
type Option_String = some(String) | none
type Point_Int = struct { x: Int, y: Int }
type Point_String = struct { x: String, y: String }
```

## Arc 类型处理（所有权模型）

```rust
# Arc 类型在单态化时保持不变，递归替换内部类型
type Shared[T] = struct { data: ref T }

# 单态化后
type Shared_Int = struct { data: ref Int }
# Arc(ref Int) 保持 Arc，ref T 中的 T 被替换为 Int
```

## 测试覆盖

### 类型单态化测试（13项）

| 测试 | 说明 |
|------|------|
| `test_monomorphize_simple_struct` | 简单结构体单态化 |
| `test_monomorphize_multi_param_struct` | 多参数结构体单态化 |
| `test_monomorphize_enum` | 枚举类型单态化 |
| `test_monomorphize_with_arc` | Arc 类型处理 |
| `test_monomorphize_nested_arc` | 嵌套 Arc 类型 |
| `test_monomorphize_list_type` | 列表类型单态化 |
| `test_monomorphize_dict_type` | 字典类型单态化 |
| `test_type_cache` | 类型缓存功能 |
| `test_type_id_generation` | 类型 ID 生成 |
| `test_monomorphize_nonexistent_type` | 不存在类型处理 |
| `test_register_monomorphized_type` | 类型注册 |
| `test_monomorphize_nested_struct` | 嵌套结构体单态化 |
| `test_generic_type_param_count` | 泛型参数计数 |

### 实例数据结构测试（55项）

| 模块 | 测试数 | 说明 |
|------|--------|------|
| `specialization_key_tests` | 10 | 特化缓存键基础功能 |
| `specialization_key_edge_cases` | 9 | 复杂类型（tuple/dict/set/fn/range） |
| `generic_function_id_tests` | 7 | 泛型函数 ID |
| `function_id_tests` | 6 | 函数 ID |
| `instantiation_request_tests` | 5 | 实例化请求 |
| `function_instance_tests` | 8 | 函数实例 |
| `type_id_tests` | 7 | 类型 ID（含 HashMap 集成） |
| `type_instance_tests` | 4 | 类型实例 |
| `generic_type_id_tests` | 5 | 泛型类型 ID |

## 与所有权模型的集成

根据 `docs/design/accepted/009-ownership-model.md`：

1. **`ref` 关键字** → `MonoType::Arc` 在单态化时保持 Arc 结构
2. 内部类型递归替换，外部 Arc 包装不变
3. 支持 `ref T` 作为泛型参数的场景
