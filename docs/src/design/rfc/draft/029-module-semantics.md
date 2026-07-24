---
title: "RFC-029: 模块语义系统"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-07-14（重写：删除兼容性章节，开放问题拆入子 RFC）"
---

# RFC-029: 模块语义系统

## 摘要

将模块系统接入编译管线，实现多文件编译和包级可见性控制。

**核心原则**：类型检查器只查询预构建的模块注册表，不碰磁盘。模块图在类型检查之前完整构建。

**不包含**：缓存、文件监听、热重载、增量重编译。这些是编译生命周期优化，属于后续独立 RFC。

## 动机

### 当前问题

1. **编译器只支持单文件**：`Compiler::compile(name, source)` 无法处理跨文件依赖
2. **导出规则互相打架**：类型自动导出、常量自动导出、方法自动导出、函数检查 `pub`——四套例外
3. **两套模块解析器**：`frontend/module/resolver.rs` 和 `package/source/module_resolver.rs` 搜索顺序不同
4. **类型检查器耦合文件加载**：草案曾要求 `use` 在类型检查时触发 `ModuleLoader::load()`

### 设计目标

- 一个项目可以编译多个 `.yx` 文件
- `use` 语句的语义清晰、无歧义
- 可见性规则统一为一条
- 单文件模式继续工作，不要求 `yaoxiang.toml`
- 类型检查器是纯逻辑，不执行文件 I/O

## 提案

### 1. 模块身份与路径解析

#### 模块定义

一个**模块**是一个 `.yx` 文件。模块路径是点分隔的命名路径，对应文件系统位置。

```
math.geometry → src/math/geometry.yx
             → src/math/geometry/mod.yx
             → src/math/geometry/index.yx
```

一个**包**是一个带 `yaoxiang.toml` 的项目，包含多个模块。包是唯一封装边界。

#### 路径解析规则

查找顺序（唯一规则，取代现有两套解析器）：

1. **标准库**：`std` 或 `std.*` → 内置模块，从 `ModuleRegistry` 查询
2. **vendor 目录**：`.yaoxiang/vendor/<pkg>-*/src/` → 依赖包
3. **当前文件相对路径**：相对于当前 `.yx` 文件所在目录
4. **项目 src 目录**：`<project_root>/src/`

文件定位尝试顺序：

```
base/name.yx
base/name/mod.yx
base/name/index.yx
```

找到第一个存在的文件即停止。如果 `name.yx` 和 `name/mod.yx` 同时存在，报错：

```
模块路径歧义：`math.geometry` 同时匹配：
  src/math/geometry.yx
  src/math/geometry/mod.yx
请删除其中一个。
```

#### 统一解析器

消灭现有两套 `ModuleResolver`。保留 `frontend/module/resolver.rs` 作为唯一实现，删除 `package/source/module_resolver.rs`。`YXPATH` 环境变量支持合并到唯一解析器中。

### 2. 导入语义

#### 语法形式

```yaoxiang
use math.geometry                          # 模块命名空间
use math.geometry as geo                   # 模块命名空间别名
use math.geometry.{Point}                  # 选择性导入
use math.geometry.{Point, distance}        # 多项选择性导入
use math.geometry.{Point as P}             # 选择性导入带别名
use math.geometry.{Point as P, distance as dist}  # 多项带别名
```

#### 语义

所有导入形式都是**编译期名字解析规则**，不是运行时引用拷贝。导入的名字指向模块导出表中的声明身份。

| 语法 | 绑定到当前作用域 | 使用方式 |
|------|-----------------|----------|
| `use path` | path 的最后一段作为模块命名空间 | `geometry.Point` |
| `use path as alias` | alias 作为模块命名空间 | `alias.Point` |
| `use path.{item}` | item 本身 | `item` |
| `use path.{item as alias}` | alias 本身 | `alias` |

#### 删除的语法

- ~~`from path use item`~~：不采用 Python from-import 形式
- ~~`use path.*`~~：通配导入带来冲突风险，模块命名空间导入已经足够
- ~~`use path.{a, b} as c, d`~~：平行列表按位置配对是脆弱数据结构，别名必须跟在每个声明后面：`use path.{a as c, b as d}`

#### 路径语义

`use path` 中的 `path` 始终是**模块路径**，不是声明。找不到模块直接报错：

```
未找到模块 `math.geometry.Point`。
如果 `Point` 是模块 `math.geometry` 中的声明，请使用：
use math.geometry.{Point}
```

不做"先找完整模块，失败后把最后一段当声明"的 fallback。

#### 导入冲突

同名导入直接报错，不静默覆盖：

```
名字 `Point` 导入冲突：
  math.geometry.Point
  graphics.geometry.Point
请改用选择性导入或模块命名空间别名。
```

### 3. 可见性

#### 规则

包是唯一封装边界。模块不承担权限边界。

| 写法 | 当前包内 | 其他包 |
|------|:--------:|:------:|
| 默认（无 `pub`） | ✅ | ❌ |
| `pub` | ✅ | ✅ |

**一条规则，适用于所有顶层声明**：类型、函数、常量、方法。

消灭现有代码的四套例外：

- ~~类型定义始终导出~~ → 同一规则
- ~~常量自动导出~~ → 同一规则
- ~~方法自动导出~~ → 同一规则
- ~~只有函数检查 `pub`~~ → 同一规则

#### 数据结构

替换 AST 中的 `is_pub: bool` 为：

```rust
pub enum Visibility {
    Package,  // 默认：当前包可见
    Public,   // pub：所有包可见
}
```

#### 导出表

每个模块维护两张表：

- **PackageSymbols**：包内完整符号表，包含所有顶层声明
- **PublicExports**：提供给其他包的 `pub` 声明子集

同包 `use` 查询 `PackageSymbols`；跨包 `use` 只能查询 `PublicExports`。

跨包引用非 `pub` 声明直接报错：

```
模块 `math.geometry` 的 `internalHelper` 不可见。
它不是 pub 声明，只能在 `math` 包内用。
```

### 4. 项目编译流程

#### 编译管线

```
项目入口
  → 读取 yaoxiang.toml 获取入口文件
  → 从入口递归解析 use 语句，发现所有依赖模块
  → 构建模块依赖图（ModuleDependencyGraph）
  → 检测循环依赖
  → 拓扑排序
  → 按顺序对每个模块执行：词法分析 → 语法分析 → 提取导出
  → 构建 ModuleRegistry（包含所有模块的导出表）
  → 按拓扑顺序对每个模块执行类型检查（查询 ModuleRegistry）
  → 生成多个 ModuleIR
  → 聚合诊断
```

类型检查器**只查询**预构建的 `ModuleRegistry`，不执行文件加载、不碰磁盘。

#### 入口文件选择

优先级：

1. `[run].main`（如果存在）
2. `[[bin]]` 第一项的 `path`
3. `[lib].path`
4. `src/main.yx`（约定默认）

单文件模式不需要 `yaoxiang.toml`，直接编译给定文件。

#### 循环依赖

```
检测到循环依赖：
  math.geometry → math.transform → math.geometry
```

循环依赖是编译错误，不做特殊处理。

#### 错误聚合

多文件编译的错误按模块拓扑顺序聚合。每个错误标注来源模块和文件位置：

```
错误：模块 `math.geometry` 中：
  src/math/geometry.yx:12:5
  类型 `Circle` 未定义

错误：模块 `app.main` 中：
  src/main.yx:3:1
  模块 `math.geometry` 不可见
```

### 5. 编译器改动

| 组件 | 改动 |
|------|------|
| `compiler.rs` | 新增 `compile_project(project_root)` 方法 |
| `pipeline.rs` | 保持单模块编译职责，不变成上帝对象 |
| `typecheck/checker.rs` | `use` 语句查询 `ModuleRegistry`，不触发文件加载 |
| `typecheck/inference/statements.rs` | 同上，`process_use_stmt` 只查询不加载 |
| `frontend/module/resolver.rs` | 合并 `package/source/module_resolver.rs` 的 YXPATH 支持，成为唯一解析器 |
| `frontend/module/loader.rs` | 扩展：支持递归发现、构建完整模块图 |
| `frontend/module/dep_graph.rs` | 已实现，复用拓扑排序和循环检测 |
| `frontend/module/registry.rs` | 已实现，复用导出表查询 |
| `frontend/module/cache.rs` | 已实现，本 RFC 不接入编译管线 |
| `frontend/module/hot_reload.rs` | 已实现，本 RFC 不接入编译管线 |
| AST `is_pub: bool` | 替换为 `Visibility` 枚举 |
| `package/source/module_resolver.rs` | 删除，职责合并到 `frontend/module/resolver.rs` |

## 实现策略

### 阶段划分

**Phase 1：统一模块解析**
1. 合并两套 `ModuleResolver`，删除 `package/source/module_resolver.rs`
2. 支持 `YXPATH` 环境变量
3. 模块路径歧义检测

**Phase 2：可见性数据结构**
4. AST `is_pub: bool` → `Visibility` 枚举
5. 解析器支持 `pub` 关键字映射到 `Visibility::Public`
6. `ModuleLoader::extract_exports` 统一使用 `Visibility` 判断导出

**Phase 3：项目编译入口**
7. `compiler.rs` 新增 `compile_project(project_root)` 方法
8. 从入口递归发现模块，构建 `ModuleDependencyGraph`
9. 拓扑排序，按顺序加载模块并提取导出
10. 构建完整 `ModuleRegistry`
11. 按拓扑顺序类型检查每个模块
12. 生成多个 `ModuleIR`，聚合诊断

**Phase 4：导入语法**
13. 实现 `use path.{item as alias}` 语法
14. 消灭路径末尾 fallback 猜测

### 依赖关系

- RFC-014（包管理器）— 名来自 `yaoxiang.toml`，vendor 目录结构
- RFC-011（泛型系统）— trait 是结构化类型，不涉及模块归属
- RFC-009（所有权模型）— 模块导入是编译期名字解析，不涉及运行时引用拷贝

## 子 RFC 规划

以下子 RFC 在**预计规划中**，尚未开始起草：

| 子 RFC | 能力（预计） | 前提条件（预计） |
|--------|-------------|-----------------|
| 029a | 模块缓存与增量重编译 | 模块图和导出表稳定 |
| 029b | 文件监听与热重载 | 029a 的缓存失效机制 |
| 029c | 重导出（`pub use`） | 导出表和可见性规则落地 |
| 029d | CLI 参数 `--entry` 覆盖入口选择 | 项目编译入口可用 |
| 029e | 多文件诊断 `--json` 输出格式 | 诊断聚合机制可用 |
| — | `pub(package)` 模块私有可见性 | 当前无现实需求，暂不纳入 |
| — | 工作空间多包编译 | 由 RFC-014c 承载 |

## 参考文献

- [RFC-009: 所有权模型](accepted/009-ownership-model.md) — Move 语义，导入是编译期名字解析
- [RFC-011: 泛型类型系统](accepted/011-generic-type-system.md) — 结构化类型定义
- [RFC-014: 包管理系统设计](accepted/014-package-manager.md) — 包名来源、vendor 目录
- [RFC-015: 配置系统](accepted/015-configuration-system.md) — `yaoxiang.toml` 字段定义