---
title: "RFC-029: 模块语义系统"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-06-13（移除孤儿规则/连贯性检查，聚焦模块接入编译管线）"
---

# RFC-029: 模块语义系统

## 摘要

将模块系统接入编译管线，实现多文件编译、模块级可见性控制和热重载。**不引入孤儿规则或连贯性检查**——YaoXiang 的 trait 是结构化类型（RFC-011 §2.1），不需要 Rust 风格的名义 impl 归属追踪。

## 动机

### 当前问题

模块系统的**物理层**（加载、解析、缓存、依赖图、热重载）已完整实现（`frontend/module/`），但**未接入编译管线**：

- `pipeline.rs` 只接受单个源字符串，不支持多文件项目
- `use` 语句在类型检查时无法实际加载模块
- `ModuleCache`、`HotReloader`、`VendorLoader` 已实现但无调用方
- 标准库 native 函数通过 `ModuleRegistry::with_std()` 硬编码注册，未走通用模块加载路径

### 为什么不需要孤儿规则

RFC-011 定义 trait 为结构化类型：

```yaoxiang
Clone: Type = { clone: (Self) -> Self }
```

- **没有 `impl Trait for Type`** — 方法直接定义在类型上
- **没有孤儿规则** — 任何模块都可以给自己的类型加方法
- **没有连贯性检查** — 方法是类型结构的一部分，不按名义匹配

因此 `TraitImplementation` 不需要 `defined_in` 或 `module` 字段。相关 issue #46 和 #73 已关闭。

## 提案

### 核心设计

**两层接入**：

```text
┌─────────────────────────────────────────────────┐
│  编译管线接入 (Pipeline Integration)             │  ← 新增：多文件编译、模块加载
├─────────────────────────────────────────────────┤
│  可见性 (Visibility)                             │  ← 新增：pub / 默认（模块内可见）
└─────────────────────────────────────────────────┘
```

### 1. 多文件编译

编译器入口从单文件扩展为项目目录：

```rust
/// 编译项目（而非单个文件）
pub fn compile_project(&mut self, project_root: &Path) -> Result<Vec<ModuleIR>, CompileError> {
    // 1. 读取 yaoxiang.toml 获取入口文件
    // 2. 从入口文件递归加载依赖模块
    // 3. 拓扑排序依赖图
    // 4. 按顺序编译每个模块
    // 5. 跨模块类型检查（use 语句解析）
}
```

接入点：`compiler.rs` 新增 `compile_project` 方法，内部使用 `ModuleLoader` 加载模块。

### 2. use 语句的模块解析

当前 `statements.rs` 有 `ModuleRegistry` 但只做注册查询。需要扩展为实际加载：

```yaoxiang
# 当前：use 语句在类型检查时无法找到模块
use math.geometry.Point  # ❌ ModuleRegistry 中没有 math.geometry

# 目标：use 语句触发模块加载
use math.geometry.Point  # ✅ ModuleLoader 加载 math/geometry.yx，提取 Point 导出
```

实现路径：
1. `use` 语句触发 `ModuleLoader::load()`
2. 加载结果注册到 `ModuleRegistry`
3. 类型检查器从 `ModuleRegistry` 查询导出类型

### 3. 可见性系统

```yaoxiang
# math/geometry.yx
pub type Point = { x: Int, y: Int }       # pub = 其他模块可用
type InternalState = { cache: Int }        # 默认 = 仅 geometry 模块内可用

pub Point.distance: (self: Point, other: Point) -> Float = {
    # ...
}
```

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

类型检查器在跨模块引用时检查可见性。

### 4. 模块缓存

`ModuleCache` 已实现 LRU/TTL 缓存策略。接入编译管线后：
- 首次编译：加载 + 编译 + 缓存
- 后续编译：命中缓存则跳过
- 文件变更：`HotReloader` 自动失效脏缓存

### 5. 热重载接入

`HotReloader` 已完整实现（`frontend/module/hot_reload.rs`），需要接入编译管线：

```rust
// 在编译管线启动时
let mut reloader = HotReloader::new(project_root, config, cache.clone());
let mut event_rx = reloader.start()?;

// 在异步主循环中
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        for module in &event.affected_modules {
            pipeline.recompile_module(module).await;
        }
    }
});
```

## 编译器改动

| 组件 | 改动 |
|------|------|
| `compiler.rs` | 新增 `compile_project` 方法 |
| `pipeline.rs` | 支持多模块编译、模块缓存查询 |
| `typecheck/inference/statements.rs` | `use` 语句触发模块加载 |
| `typecheck/mod.rs` | 从通用模块路径注册 native 函数（替代硬编码） |
| `frontend/module/loader.rs` | 已实现，无需改动 |
| `frontend/module/cache.rs` | 已实现，无需改动 |
| `frontend/module/hot_reload.rs` | 已实现，无需改动 |
| AST 层面 | `pub` 关键字对类型的可见性标注（如尚未支持） |

## 实现策略

### 阶段划分

**Phase 1：多文件编译入口**
1. `compiler.rs` 新增 `compile_project(project_root)` 方法
2. 使用 `ModuleLoader` 从入口文件递归加载依赖
3. 使用 `ModuleDependencyGraph` 拓扑排序
4. 按顺序调用现有单文件编译流程

**Phase 2：use 语句模块解析**
5. `statements.rs` 中 `use` 语句触发 `ModuleLoader::load()`
6. 加载结果注册到 `ModuleRegistry`
7. 导出类型在类型检查时可用

**Phase 3：可见性**
8. AST 层面解析 `pub` 对类型的标注
9. 类型检查器在跨模块引用时检查可见性

**Phase 4：缓存与热重载**
10. `pipeline.rs` 接入 `ModuleCache`
11. `pipeline.rs` 接入 `HotReloader`
12. 增量重编译只处理受影响模块

### 依赖关系

- RFC-014（包管理器）— 包名来自 `yaoxiang.toml`
- RFC-011（泛型系统）— trait 是结构化类型，不涉及模块归属

## 开放问题

- [ ] 默认可见性是"模块内"还是"包内"？（Rust 默认模块内，Go 默认包内）
- [ ] 是否需要 `pub(crate)` 级别？
- [ ] 热重载是否需要支持跨模块依赖链重编译？
- [ ] 多文件编译的错误报告如何聚合？

---

## 参考文献

- [RFC-011: 泛型类型系统](accepted/011-generic-type-system.md) — 结构化类型定义
- [RFC-014: 包管理系统设计](accepted/014-package-manager.md) — 包名来源
