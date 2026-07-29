---
title: 'RFC-029: 模块语义系统'
status: '已接受'
author: '晨煦'
created: '2026-06-13'
updated: '2026-07-30（重写：基于审核讨论，确立模块=record语义，删除可见性机制）'
issue: '#232'
---

# RFC-029: 模块语义系统

## 摘要

将模块系统接入编译管线，实现多文件编译。

**核心定义**：模块 = 一个 `.yx` 文件的所有顶层绑定。模块的类型 = 那些绑定的类型（推断）。
`use` = record 解构。没有 `pub`，没有 `private`，没有 `export`，没有可见性机制。

**核心原则**：

- 类型检查器只查询预构建的 ModuleRegistry，不碰磁盘
- 包内文件合并为单一编译单元（AST 拼接），包内循环引用自然允许
- Registry 按需加载：只装从入口沿 `use` 可达的模块

**不包含**：缓存、文件监听、热重载、增量重编译、跨包循环依赖处理。

## 动机

### 当前问题

1. **编译器只支持单文件**：`Pipeline::run(name, source)` 接收一个字符串，无法处理跨文件依赖
2. **`use` 只能解析 std 模块**：本地文件间的 `use` 全部报 "Unknown variable"（#232）
3. **模块解析器放错了位置**：唯一的路径解析逻辑在 `package/source/module_resolver.rs`，`frontend/module/resolver.rs` 实际是编译期谓词正格化（RFC-027）

### 设计目标

- 一个项目可以编译多个 `.yx` 文件
- `use` 语句的语义清晰：record 解构，不是特殊机制
- 单文件继续工作，不要求 `yaoxiang.toml`
- 管线（`Pipeline`）零改动，多文件支持是编排层的事
- 不引入新关键字、新 AST 节点、新概念

## 提案

### 1. 模块 = record of bindings

一个**模块**是一个 `.yx` 文件的所有顶层绑定。

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }
distance: (a: Point, b: Point) -> Float = { ... }
```

这个模块的内容就是 `{ Point: Type, distance: (Point, Point) -> Float }`。

模块不是特殊实体。它是 `name: type = value` 模型的一个实例——碰巧在文件边界上定义的
record。模块的类型从绑定推断，永远不需要显式标注。

`use` 引入的绑定**也是**模块内容的一部分：

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

`math` 模块的内容 = `{ Point: Type, distance: (Point, Point) -> Float }`。
外面 `use math.{Point}` 能拿到。`use math.geometry.{Point}` 也能拿到。
两个路径指向同一个绑定。

### 2. use = record 解构

所有 `use` 形式都是 record 字段访问 + 绑定：

```yaoxiang
use math.geometry.{Point, distance}
```

等价于：

```yaoxiang
Point = math.geometry.Point
distance = math.geometry.distance
```

| 语法 | 语义 |
|------|------|
| `use path.{item}` | 取 `path` record 的 `item` 字段，绑定到当前作用域 |
| `use path.{a, b}` | 取多个字段 |
| `use path` | 取 `path` record 本身，绑定到最后一段名字 |
| `use path as alias` | 取 `path` record 本身，绑定到 `alias` |

#### 不存在的语法

- ~~`use path.*`~~：通配导入。不需要，显式列出绑定。
- ~~`from path use item`~~：Python 式。不采用。
- ~~`use path.{item as alias}`~~：花括号内别名。Phase 4 可选，不阻塞 #232。

#### 导入冲突

同名绑定直接报错：

```
名字 `Point` 冲突：
  math.geometry.Point
  graphics.shapes.Point
请使用不同名字或模块别名。
```

### 3. 可见性：不存在

**本 RFC 不引入任何可见性机制。** 所有顶层绑定对所有能写出路径的代码可见。

这是有意的设计决策，不是遗漏。

#### 设计依据

| 想表达的 | 怎么做 | 机制 |
|----------|--------|------|
| "这是 API" | 放在发布的包里 | 分发 |
| "这是内部的" | 放在不发布的包里 | 分发 |
| "这是函数内部的" | 写在函数体里 | 作用域 |

三层全是已有机制：包、文件、作用域。不需要新东西。

#### 为什么不要 `pub`

- 不想让别人用的东西，不应该放在顶层（放局部作用域）
- 多个文件共享的辅助函数，放在独立的不发布的包里
- "防不防得住"和"该不该有信号"是两件事。当前阶段没有第三方生态，信号无意义
- 门不上锁。走门是礼貌，翻墙是自由。语言不管礼貌

#### 将来

当生态成熟需要强制边界时，可通过独立 RFC 引入。加限制是向后兼容的（默认公开 →
显式标记为内部）。但本 RFC 不预设这个方向，也不承诺它会到来。

### 4. 路径解析

#### 模块路径 → 文件

```
use math.geometry.{Point}
```

查找顺序（唯一规则）：

1. **Registry 已注册模块**：`math.geometry` 已在 Registry 中 → 直接使用
2. **标准库**：`std` 或 `std.*` → 内置模块
3. **项目 src 目录**：`<project_root>/src/math/geometry.yx`
4. **vendor 目录**：`.yaoxiang/vendor/<pkg>-*/src/`（将来）

文件定位尝试顺序：

```
base/name.yx
base/name/mod.yx
```

找到第一个即停止。两者同时存在 → 报错：

```
模块路径歧义：`math.geometry` 同时匹配：
  src/math/geometry.yx
  src/math/geometry/mod.yx
请删除其中一个。
```

#### mod.yx = 目录入口（约定）

`mod.yx` 是目录的入口文件。`use math` 时加载 `src/math/mod.yx`。

这是**约定，不是强制**。用户可以直接 `use math.geometry` 穿透到子文件。
`mod.yx` 是"推荐入口"（门牌号），不是"唯一入口"（锁）。

#### 统一解析器

当前唯一的路径解析逻辑在 `package/source/module_resolver.rs`。
将其移至 `frontend/module/resolver.rs`（替换当前名不副实的谓词正格化文件，
谓词正格化移至 `frontend/core/types/eval/`）。

### 5. 项目编译流程

#### 路径 A：合并 AST

包内所有文件合并为**单一编译单元**。管线零改动。

```
编排器（Pipeline 之上）：
  1. 确定入口文件
  2. 解析入口文件的 use 语句（只读 use 行，不解析函数体）
  3. 沿 use 路径发现文件，加入队列
  4. 对队列中的文件，解析它们的 use 语句
  5. 重复 3-4 直到队列为空（按需发现）
  6. 逐个完整解析所有发现的文件 → 多个 AST
  7. 合并为一个 Module（所有顶层 items 拼接，Span 保留来源文件）
  8. 喂给 Pipeline::run()（管线不知道有多个文件）
```

#### 包内循环引用：允许

因为所有文件合并为一个 AST，包内文件互相 `use` 等价于同一个文件内的互引：

```yaoxiang
// tree.yx
use node.{Node}
Tree: Type = { root: Node }

// node.yx
use tree.{Tree}
Node: Type = { value: Int, parent: Tree }
```

合并后就是一个 AST 里两个互相引用的类型定义。编译器本来就支持。

#### 包间循环：再说

包是分发单位，包间需要拓扑序。当前无第三方包生态，暂不处理。
遇到时直接报错即可。

#### 入口文件选择

优先级：

1. `[run].main`（yaoxiang.toml）
2. `[[bin]]` 第一项的 `path`
3. `src/main.yx`（约定默认）

无 `yaoxiang.toml` 时：直接编译给定文件。Registry 里只有 std。
这不是"单文件模式"——是"发现结果为空的自然结果"。

#### Registry 按需加载

Registry 的内容 = 从入口沿 `use` 可达的所有模块。
不可达的模块不解析、不注册、不存在。
**这不是优化，是定义。**

### 6. std 模块与用户模块同质

对 typecheck 来说，`use std.io.{println}` 和 `use math.geometry.{Point}` 操作完全一样：

1. 在 Registry 里找到模块 record
2. 取字段
3. 绑定到当前作用域

来源（Std / User / Vendor）是元数据，不影响解析逻辑。
native 函数的特殊处理推迟到 IR gen / codegen 层。

## 编译器改动

| 组件 | 改动 |
|------|------|
| `frontend/module/resolver.rs` | **重写**：当前是谓词正格化（RFC-027），移至 `frontend/core/types/eval/`。此文件改为真正的模块路径解析（从 `package/source/module_resolver.rs` 迁移） |
| `frontend/module/mod.rs` | 扩展：补充合并 AST 所需的来源文件追踪（Span 带文件名） |
| `frontend/module/registry.rs` | 扩展：支持注册用户模块（当前只注册 std） |
| `frontend/module/orchestrator.rs` | **新建**：多文件编排器（发现 → 解析 → 合并 → 调用 Pipeline） |
| `frontend/pipeline.rs` | **不改** |
| `frontend/core/parser/statements/imports.rs` | 不改（`use` 解析已实现） |
| `package/source/module_resolver.rs` | **删除**，逻辑迁移到 `frontend/module/resolver.rs` |
| `frontend/core/typecheck/` | `use` 处理改为查 Registry（当前只查 std） |
| AST `is_pub: bool` | **不动**。本 RFC 不涉及可见性 |

### 不存在的文件（RFC 旧版声称"已实现"但实际不存在）

- ~~`frontend/module/loader.rs`~~ — 不存在，职责由编排器承担
- ~~`frontend/module/dep_graph.rs`~~ — 不存在，包内不需要拓扑排序（合并 AST）
- ~~`frontend/module/cache.rs`~~ — 不存在，属于子 RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — 不存在，属于子 RFC 029b

## 实现策略

### Phase 1：统一路径解析

1. 将谓词正格化从 `frontend/module/resolver.rs` 移至 `frontend/core/types/eval/`
2. 将 `package/source/module_resolver.rs` 的路径解析逻辑迁移到 `frontend/module/resolver.rs`
3. 模块路径歧义检测（`name.yx` 和 `name/mod.yx` 同时存在 → 报错）

### Phase 2：多文件编排器

4. 新建 `frontend/module/orchestrator.rs`
5. 实现按需发现（从入口沿 use 递归）
6. 实现 AST 合并（多文件 items 拼接，Span 带来源文件）
7. `compiler.rs` 新增 `compile_project(project_root)` 调用编排器

### Phase 3：use 名字解析

8. typecheck 的 `process_use_stmt` 改为查 Registry（不再只查 std）
9. 导入冲突检测（同名报错）
10. E2E 测试：多文件项目 `use` 本地模块

### Phase 4（可选，不阻塞 #232）

11. `use path.{item as alias}` 花括号内别名
12. vendor 目录解析（配合 RFC-014）

### 依赖关系

- RFC-014（包管理器）— `yaoxiang.toml` 字段、vendor 目录结构（Phase 4 才需要）
- 无其他前置依赖

## 子 RFC 规划

| 子 RFC | 能力 | 前提 |
|--------|------|------|
| 029a | 模块缓存与增量重编译 | 编排器稳定 |
| 029b | 文件监听与热重载 | 029a |
| 029d | CLI `--entry` 覆盖入口 | 编排器可用 |
| 029e | 多文件诊断 `--json` 输出 | 诊断聚合 |

已删除：~~029c（重导出）~~ — 不需要。`use` 就是重导出，没有 "pub use" 概念。

## 设计决策记录

| 决策 | 结论 | 日期 | 依据 |
|------|------|------|------|
| 模块是什么 | 文件顶层绑定的 record | 2026-07-30 | RFC-010 `name: type = value` 统一模型 |
| `use` 语义 | record 解构 | 2026-07-30 | 不引入新机制，复用已有 record 语义 |
| 可见性 | 不存在 | 2026-07-30 | 作用域 + 分发边界覆盖所有场景，不需要新关键字 |
| `pub` 关键字 | 不要 | 2026-07-30 | "不想让别人用就别放顶层/别发布那个包" |
| mod.yx 语义 | 目录入口（约定，非强制） | 2026-07-30 | Python `__init__.py` 模型：门不上锁 |
| 模块类型标注 | 不需要 | 2026-07-30 | 内部绑定已自带类型，标注是冗余 |
| 包内循环 | 允许（合并 AST） | 2026-07-30 | 路径 A：管线零改动，Rust crate 内模型 |
| 包间循环 | 暂不处理 | 2026-07-30 | 无第三方生态，遇到报错即可 |
| Registry 加载 | 按需（只装可达模块） | 2026-07-30 | 不是优化，是定义 |
| 单文件 vs 项目 | 同一机制 | 2026-07-30 | Registry 内容不同，查找逻辑相同 |

## 参考文献

- [RFC-010: 统一类型语法](../accepted/010-unified-type-syntax.md) — `name: type = value` 模型
- [RFC-009: 所有权模型](../accepted/009-ownership-model.md) — 导入是编译期名字解析
- [RFC-011: 泛型类型系统](../accepted/011-generic-type-system.md) — 结构化类型
- [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) — 包名、vendor 目录
- [RFC-026: FFI 核心机制](../accepted/026-ffi-core-mechanism.md) — StdModule 注册
- [RFC-030: assert 断言机制](../accepted/030-assert-mechanism.md) — StdModule 统一注册先例
