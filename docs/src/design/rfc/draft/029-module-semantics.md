---
title: "RFC-029: 模块语义系统"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-06-13"
---

# RFC-029: 模块语义系统

## 摘要

为 YaoXiang 的类型系统和 trait 系统引入模块归属追踪和可见性规则，使孤儿规则、一致性检查、跨模块类型正确性成为可能。同时将已实现但未接入的热重载模块接入编译管线。

## 动机

### 为什么需要这个特性/变更？

模块系统的**物理层**（加载、解析、缓存、依赖图、热重载）已完整实现。但**语义层**完全空白：

- `TraitImplementation` 不知道属于哪个模块 → 孤儿规则无法实现（#46、#73）
- `StructType`/`EnumType` 不记录定义位置 → 跨模块一致性检查不可能
- 类型检查器不知道"我现在在编译哪个模块" → 无法判断 impl 是本地还是外部
- 所有导出都是公开的 → 没有访问控制

这些问题在 #46（Trait 连贯性检查）和 #73（TraitImplementation span）中暴露出来。

### 当前的问题

```yaoxiang
# module_a.yx
type Point = { x: Int, y: Int }

# module_b.yx
use module_a.Point

# 为别人的类型实现别人的 trait —— 应该被拒绝，但编译器不知道
Point.clone: (self: Point) -> Point = {
    return Point(self.x, self.y)
}
```

当前编译器无法判断这个 `clone` 实现是否合法，因为：
1. `TraitImplementation` 没有记录"这个 impl 在 module_b 里定义"
2. `StructType` 没有记录"Point 在 module_a 里定义"
3. 没有孤儿规则来验证归属

## 提案

### 核心设计

**三层数据扩展**：

```text
┌─────────────────────────────────────────────────┐
│  编译上下文 (CompilationContext)                 │  ← 新增：当前正在编译的模块路径
├─────────────────────────────────────────────────┤
│  类型定义归属 (StructType.defined_in)            │  ← 新增：记录类型在哪个模块定义
│  TraitImplementation.defined_in                  │  ← 新增：记录 impl 在哪个模块定义
│  TraitDefinition.defined_in                      │  ← 新增：记录 trait 在哪个模块定义
├─────────────────────────────────────────────────┤
│  可见性 (Visibility)                             │  ← 新增：pub / 默认（模块内可见）
└─────────────────────────────────────────────────┘
```

### 示例

```yaoxiang
# math/geometry.yx
pub type Point = { x: Int, y: Int }       # pub = 其他模块可用
type InternalState = { cache: Int }        # 默认 = 仅 geometry 模块内可用

pub Point.distance: (self: Point, other: Point) -> Float = {
    # ...
}

# main.yx
use math.geometry.Point
use math.geometry.Point.distance   # ✅ pub 导出
use math.geometry.InternalState    # ❌ 编译错误：InternalState 不可见
```

### 语法变化

| 之前 | 之后 | 说明 |
|------|------|------|
| `type Point = {...}` | `pub type Point = {...}` | 类型可见性 |
| `Point.clone: ...` | `Point.clone: ...` | impl 语法不变，归属由编译上下文自动推断 |

**关键设计：impl 的归属不需要用户手动标注。** impl 在哪个文件里写，就属于哪个模块。编译上下文自动携带当前模块路径。

## 详细设计

### 1. 编译上下文

```rust
/// 编译上下文 — 贯穿整个编译管线
pub struct CompilationContext {
    /// 当前正在编译的模块路径（如 "math.geometry"）
    pub current_module: String,
    /// 当前文件路径
    pub current_file: PathBuf,
    /// 当前包名（来自 yaoxiang.toml）
    pub package_name: String,
}
```

编译管线在进入每个模块时设置 `current_module`，类型检查器通过上下文获取。

### 2. 类型定义归属

```rust
pub struct StructType {
    pub name: String,
    pub fields: Vec<(String, MonoType)>,
    // ... 现有字段 ...

    /// 新增：定义该类型的模块路径
    pub defined_in: Option<String>,
}

pub struct EnumType {
    pub name: String,
    pub variants: Vec<String>,
    /// 新增：定义该枚举的模块路径
    pub defined_in: Option<String>,
}

pub struct TraitDefinition {
    pub name: String,
    pub methods: HashMap<String, TraitMethodSignature>,
    // ... 现有字段 ...

    /// 新增：定义该 trait 的模块路径
    pub defined_in: Option<String>,
}
```

使用 `Option<String>` 而非 `String`，确保向后兼容——现有代码不填也不会 break。

### 3. TraitImplementation 归属

```rust
pub struct TraitImplementation {
    pub trait_name: String,
    pub for_type_name: String,
    pub methods: HashMap<String, MonoType>,

    /// 新增：定义该 impl 的模块路径
    pub defined_in: Option<String>,
    /// 新增：定义位置（用于错误报告）
    pub span: Option<Span>,
}
```

### 4. 可见性

```rust
/// 可见性级别
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// 公开 — 所有模块可访问
    Public,
    /// 默认 — 仅定义模块内可见
    Module,
}
```

在 AST 层面，`pub` 关键字已有 parser 支持（需要验证）。类型检查器在跨模块引用时检查可见性。

### 5. 孤儿规则

```
impl Trait for Type 合法 ⟺
    Trait 定义在当前模块 OR Type 定义在当前模块
```

实现位置：`CoherenceChecker::check_orphan_rule`。

```rust
fn check_orphan_rule(&self, impl_: &TraitImplementation, ctx: &CompilationContext) -> bool {
    let trait_defined_here = self.trait_table
        .get_trait(&impl_.trait_name)
        .and_then(|t| t.defined_in.as_ref())
        .map_or(false, |m| m == &ctx.current_module);

    let type_defined_here = /* 查 StructType.defined_in */ == &ctx.current_module;

    trait_defined_here || type_defined_here
}
```

### 6. 热重载接入

`HotReloader` 已完整实现（`frontend/module/hot_reload.rs`），需要接入编译管线：

```rust
// 在编译管线启动时
let mut reloader = HotReloader::new(project_root, config, cache.clone());
let mut event_rx = reloader.start()?;

// 在异步主循环中
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        // 只重编译受影响的模块
        for module in &event.affected_modules {
            pipeline.recompile_module(module).await;
        }
    }
});
```

接入点在 `frontend/pipeline.rs` 的 `run_and_cache` 方法中。

## 编译器改动

| 组件 | 改动 |
|------|------|
| `types/trait_data.rs` | `TraitImplementation`、`TraitDefinition` 添加 `defined_in`、`span` |
| `types/mono.rs` | `StructType`、`EnumType` 添加 `defined_in` |
| `typecheck/environment.rs` | `add_trait_impl` 填充 `defined_in` |
| `typecheck/checker.rs` | 从编译上下文获取 `current_module` |
| `typecheck/traits/coherence.rs` | 实现 `check_orphan_rule` |
| `frontend/pipeline.rs` | 接入 `HotReloader` |
| `frontend/module/loader.rs` | `extract_exports` 填充 `defined_in` |
| AST 层面 | `pub` 关键字对类型的可见性标注 |

## 向后兼容性

- 所有新字段都是 `Option` 类型，不填时行为不变
- 现有代码不写 `pub` 等同于 `Visibility::Module`（默认模块内可见）
- 热重载是可选功能，不影响现有编译流程

## 权衡

### 优点

- 解锁孤儿规则（#46 依赖）
- 解锁冲突实现位置报告（#73 依赖）
- 解锁跨模块可见性检查
- 热重载能力从"已实现"变为"可用"

### 缺点

- 每个类型定义和 impl 都需要填充 `defined_in`，增加编译器代码量
- 可见性检查需要在跨模块引用时插入检查点

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| 不追踪归属 | 继续让孤儿规则空着 | 语言正确性缺陷，用户写冲突 impl 不报错 |
| 用文件路径代替模块路径 | `defined_in: PathBuf` | 路径不稳定（重命名/移动），模块路径是逻辑标识 |
| 显式 crate 标注 | 类似 Rust 的 `extern crate` | YaoXiang 设计目标是简洁，模块路径自动推断足够 |

## 实现策略

### 阶段划分

**Phase 1：数据基础**（无行为变化）
1. `TraitImplementation` 添加 `defined_in: Option<String>` + `span: Option<Span>`
2. `TraitDefinition` 添加 `defined_in: Option<String>`
3. `StructType`、`EnumType` 添加 `defined_in: Option<String>`
4. 所有构造点填入 `None`（保持现有行为）

**Phase 2：编译上下文**
5. 定义 `CompilationContext` 结构
6. 编译管线在模块入口设置 `current_module`
7. 类型检查器从上下文读取 `current_module`

**Phase 3：归属填充**
8. `loader.rs::extract_exports` 填充 `defined_in`
9. `environment.rs::add_trait_impl` 从上下文填充 `defined_in`
10. 结构体/枚举定义处填充 `defined_in`

**Phase 4：孤儿规则**
11. `CoherenceChecker::check_orphan_rule` 实现归属检查
12. 编译时对非法 impl 报错

**Phase 5：可见性**
13. AST 层面解析 `pub` 对类型的标注
14. 类型检查器在跨模块引用时检查可见性

**Phase 6：热重载接入**
15. `pipeline.rs` 接入 `HotReloader`
16. 增量重编译只处理受影响模块

### 依赖关系

- #46（Trait 连贯性检查）— Phase 4 完成后可关闭
- #73（TraitImplementation span）— Phase 1 完成后可关闭
- RFC-014（包管理器）— 包名来自 `yaoxiang.toml`

### 风险

- Phase 3 涉及大量构造点修改，容易遗漏
- 可见性规则可能与现有跨模块代码冲突（需要迁移期）

## 开放问题

- [ ] 默认可见性是"模块内"还是"包内"？（Rust 默认是模块内，Go 默认是包内）
- [ ] 是否需要 `pub(crate)` 级别？
- [ ] 孤儿规则是否允许"孤儿豁免"（类似 Rust 的 `#[fundamental]`）？
- [ ] 热重载是否需要支持跨模块依赖链重编译？

---

## 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 归属用模块路径而非文件路径 | 模块路径是逻辑标识，不受文件移动影响 | 2026-06-13 | 晨煦 |
| impl 归属自动推断 | 不需要用户标注，编译上下文自动携带 | 2026-06-13 | 晨煦 |
| 所有新字段用 Option | 向后兼容，不强制填充 | 2026-06-13 | 晨煦 |
| 孤儿规则用模块级而非包级 | 与 Rust 对齐，更严格 | 2026-06-13 | 晨煦 |

---

## 参考文献

- [RFC-014: 包管理系统设计](accepted/014-package-manager.md)
- [RFC-009: 所有权模型设计](accepted/009-ownership-model.md)
- [RFC-011: 泛型类型系统](accepted/011-generic-type-system.md)
- Rust RFC 2451 — Re-exporting and visibility
- Rust Orphan Rules — https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
