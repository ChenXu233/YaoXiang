# Task 7.4: 闭包单态化 - 实现计划

> **状态**: ✅ 已完成
> **实现文件**: [closure.rs](../../../../src/middle/monomorphize/closure.rs)
> **测试文件**: [closure_monomorphize.rs](../../../../src/middle/monomorphize/tests/closure_monomorphize.rs)
> **依赖**: task-07-03 (函数单态化)
> **预估工作量**: 3-4 天

## 背景

闭包（Lambda）是"带捕获环境的函数"。闭包单态化比函数单态化多一个维度：**捕获变量的类型组合**。

```yaoxiang
make_adder = (x: Int) => (y: Int) => x + y  # x 被捕获

# 单态化后需要为每种 x 的类型生成代码
make_adder_int = (x: Int) => (y: Int) => x + y
make_adder_f64 = (x: Float64) => (y: Float64) => x + y
```

## 核心洞察

```
闭包单态化 = 函数单态化 + 捕获变量处理

复用现有代码：
- substitute_types: 类型替换逻辑
- SpecializationKey: 缓存键（扩展支持捕获变量）
- should_specialize: 特化上限控制
```

## 数据结构设计

### 1. ClosureId (新增)

```rust
// instance.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClosureId {
    name: String,              // 闭包名称
    capture_types: Vec<MonoType>,  // 捕获变量的类型列表
}

impl ClosureId {
    pub fn new(name: String, capture_types: Vec<MonoType>) -> Self
    pub fn specialized_name(&self) -> String  // 如 "closure_123_int64_string"
}
```

### 2. ClosureInstance (新增)

```rust
// instance.rs
#[derive(Debug, Clone)]
pub struct ClosureInstance {
    pub id: ClosureId,
    pub generic_id: GenericClosureId,
    pub capture_vars: Vec<CaptureVariable>,  // 捕获变量详情
    pub body_ir: FunctionIR,                  // 闭包体的 IR
}

pub struct CaptureVariable {
    pub name: String,
    pub mono_type: MonoType,
    pub value: Operand,  // 捕获的值
}
```

### 3. GenericClosureId (新增)

```rust
// instance.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericClosureId {
    /// 生成闭包的泛型函数名（如 "make_adder"）
    name: String,
    /// 泛型参数（如 ["T"]）
    type_params: Vec<String>,
    /// 捕获变量名称（用于调试）
    capture_names: Vec<String>,
}
```

## 核心 API 设计

### Monomorphizer 扩展

```rust
impl Monomorphizer {
    /// 单态化闭包（主入口）
    pub fn monomorphize_closure(
        &mut self,
        generic_id: &GenericClosureId,
        type_args: &[MonoType],
        capture_types: &[MonoType],
    ) -> Option<ClosureId>;

    /// 检查闭包是否已单态化
    pub fn is_closure_monomorphized(&self, ...) -> bool;

    /// 获取已实例化的闭包
    pub fn get_instantiated_closure(&self, id: &ClosureId) -> Option<&ClosureInstance>;

    /// 获取已单态化的闭包数量
    pub fn instantiated_closure_count(&self) -> usize;
}
```

## 实现步骤

### Phase 1: 数据结构（Day 1）

| 文件 | 改动 |
|------|------|
| `instance.rs` | 添加 `ClosureId`, `ClosureInstance`, `GenericClosureId` |
| `mod.rs` | `Monomorphizer` 添加闭包相关字段 |

**新增字段到 Monomorphizer**:
```rust
pub struct Monomorphizer {
    // ... 现有字段 ...

    // ==================== 闭包单态化相关 ====================
    instantiated_closures: HashMap<ClosureId, ClosureInstance>,
    closure_specialization_cache: HashMap<ClosureSpecializationKey, ClosureId>,
    generic_closures: HashMap<GenericClosureId, ClosureIR>,
    next_closure_id: usize,
}
```

### Phase 2: 核心逻辑（Day 2）

| 方法 | 职责 |
|------|------|
| `monomorphize_closure()` | 主入口：查缓存 → 检查上限 → 实例化 |
| `instantiate_closure()` | 生成闭包 IR，处理捕获变量 |
| `substitute_closure_body()` | 替换闭包体中的泛型参数 |
| `extract_capture_types()` | 从闭包环境提取捕获变量类型 |

**关键算法：闭包实例化**

```rust
fn instantiate_closure(
    &mut self,
    generic_id: &GenericClosureId,
    type_args: &[MonoType],
    capture_types: &[MonoType],
) -> Option<ClosureInstance> {
    // 1. 构建类型替换映射 (TypeVar -> 具体类型)
    let type_map = self.build_type_map(generic_id, type_args)?;

    // 2. 替换闭包签名中的泛型参数
    let new_signature = self.substitute_signature(&generic_id.signature, &type_map)?;

    // 3. 替换闭包体中的泛型参数
    let new_body = self.substitute_closure_body(&generic_id.body, &type_map)?;

    // 4. 处理捕获变量（类型替换 + 值传递）
    let capture_vars = self.process_capture_vars(&generic_id.capture_vars, &type_map)?;

    Ok(ClosureInstance {
        id: ClosureId::new(..., capture_types.to_vec()),
        generic_id: generic_id.clone(),
        capture_vars,
        body_ir: new_body,
    })
}
```

### Phase 3: 测试（Day 3）

**测试用例**：

| 测试名 | 描述 |
|--------|------|
| `test_simple_closure` | 简单闭包单态化 |
| `test_closure_with_captures` | 带捕获变量的闭包 |
| `test_closure_multiple_captures` | 多捕获变量 |
| `test_closure_cache_hit` | 缓存命中 |
| `test_closure_different_types` | 不同类型生成不同实例 |
| `test_closure_monomorphized_count` | 统计验证 |
| `test_closure_nested` | 嵌套闭包 |
| `test_closure_as_fn_param` | 闭包作为函数参数传递 |

**测试文件**: `tests/closure_monomorphize.rs`

### Phase 4: 集成与文档（Day 4）

1. 更新 `tests/mod.rs` 导出
2. 更新 `instance.rs` 导出
3. 写任务文档 `task-07-04-closure-monomorphize.md`
4. 运行完整测试套件

## 复用与扩展

### 复用现有代码

| 已有实现 | 复用方式 |
|---------|---------|
| `substitute_types()` | 直接用于闭包体类型替换 |
| `should_specialize()` | 复用特化上限控制 |
| `SpecializationKey` | 扩展为 `ClosureSpecializationKey` |

### 扩展点

```rust
// 扩展 SpecializationKey 支持闭包
pub struct ClosureSpecializationKey {
    pub name: String,           // 闭包名
    pub type_args: Vec<MonoType>,  // 类型参数
    pub capture_types: Vec<MonoType>,  // 捕获变量类型（新增）
}
```

## 性能分析

### 闭包单态化 vs 动态分派

| 场景 | 动态分派 | 单态化 |
|------|---------|--------|
| `map(list, closure)` | 虚表查找 + 间接调用 | 直接调用 |
| 热路径闭包 | 无法内联 | 完全内联 |

**性能提升预估**：x10 ~ x100（闭包作为泛型参数的场景）

### 缓存策略

```
闭包缓存 = 类型参数组合 × 捕获变量类型组合

示例：
- make_adder<Int> 捕获 (Int) → 一个实例
- make_adder<Float64> 捕获 (Float64) → 一个实例
- make_adder<String> 捕获 (String) → 一个实例
```

## 文件清单

| 文件 | 状态 | 说明 |
|------|------|------|
| `instance.rs` | 修改 | 添加闭包相关结构 |
| `mod.rs` | 修改 | 添加闭包 API 和字段 |
| `tests/closure_monomorphize.rs` | 新增 | 15+ 测试用例 |
| `tests/mod.rs` | 修改 | 导出新模块 |
| `task-07-04-closure-monomorphize.md` | 新增 | 任务文档 |

## 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| 捕获变量生命周期管理 | 复用所有权模型（ref/Arc） |
| 嵌套闭包复杂度 | 分阶段实现，先做单层 |
| 测试覆盖不全 | 集成测试 + 属性测试 |

## 验收标准

- [ ] 15+ 单元测试通过
- [ ] 集成测试通过
- [ ] 类型替换正确（参数/返回值/局部变量）
- [ ] 捕获变量正确传递
- [ ] 缓存机制工作正常
- [ ] 特化上限控制工作
- [ ] 文档完整

## 时间线

```
Day 1: 数据结构设计 + 实现
Day 2: 核心逻辑实现
Day 3: 测试用例编写
Day 4: 集成 + 文档 + 修复
```

---

## 相关文档

- [task-07-03-fn-monomorphize.md](./task-07-03-fn-monomorphize.md)
- [009-ownership-model.md](../../design/accepted/009-ownership-model.md)
