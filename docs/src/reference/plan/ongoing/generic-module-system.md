# YaoXiang 通用模块系统重构计划

## 1. 概述

当前 YaoXiang 的模块系统是针对 `std` 内置模块的特殊实现，存在多处硬编码，不支持用户自定义模块。本文档描述如何实现通用的模块系统。

## 2. 验收标准

### 2.1 必须支持的功能

| 功能 | 语法示例 | 说明 |
|------|---------|------|
| 模块导入 | `use my_module` | 从文件系统加载模块 |
| 选择性导入 | `use my_module.{func1, func2}` | 只导入指定函数 |
| 模块别名 | `use my_module as m` | 使用别名访问 |
| 模块调用 | `my_module.func()` | 通过模块调用函数 |
| 子模块 | `my_module.sub.func()` | 访问子模块 |
| 模块属性 | `my_module.CONST` | 访问模块常量 |

### 2.2 std 模块兼容性

重构后必须保持与当前相同的行为：

| 语法 | 示例 | 状态 |
|------|------|------|
| `std.io.print()` | `use std.io` + `std.io.println("x")` | ✅ 必须保持 |
| `print()` (选择性导入) | `use std.io.{print}` + `print("x")` | ✅ 必须保持 |
| `io.print()` (模块导入) | `use std.{io}` + `io.println("x")` | ✅ 必须保持 |
| `std.math.PI` | 常量访问 | ✅ 必须保持 |

### 2.3 模块加载规则

```
// 模块搜索路径（相对于当前文件）
1. ./my_module.yx           // 当前目录
2. ./my_module/mod.yx       // 子目录
3. ./my_module/index.yx     // index 文件

// std 模块特殊处理
// std 模块位于 src/std/，始终可用
```

## 3. 测试需求

### 3.1 功能测试

```yaoxiang
// test_user_module.yx
// 假设存在 my_module.yx 文件

// 1. 基本模块导入
use my_module
main = {
    my_module.greet()  // 调用模块函数
}

// 2. 选择性导入
use my_module.{add, sub}
main = {
    add(1, 2)
}

// 3. 模块别名
use my_module as m
main = {
    m.greet()
}

// 4. 子模块
use my_module.utils
main = {
    my_module.utils.help()
}

// 5. 嵌套导入
use my_module.{sub1, sub2.sub3}
```

### 3.2 std 模块兼容性测试

所有现有的测试用例必须继续通过：

```yaoxiang
// 现有语法测试
use std.io
main = { std.io.println("test") }

use std.io.{print}
main = { print("test") }

use std.{io}
main = { io.println("test") }

use std.math
main = { std.math.PI }
```

### 3.3 边界情况测试

- 空模块导入
- 模块循环依赖（应报错）
- 不存在的模块（应报错）
- 重复导入
- 导入冲突

## 4. 现有代码分析

### 4.1 代码异味位置

| 文件 | 行号 | 问题 |
|------|------|------|
| `src/std/mod.rs` | 29-100 | 硬编码的 `get_module_exports`，只处理 std 模块 |
| `src/frontend/typecheck/mod.rs` | 593-622 | 硬编码的 std 模块处理逻辑 |
| `src/frontend/typecheck/inference/expressions.rs` | 515-598 | 硬编码的 `io\|math\|net\|concurrent` 列表 |
| `src/middle/core/ir_gen.rs` | 65-131 | 硬编码的命名空间处理 |

### 4.2 关键结构分析

#### 4.2.1 ModuleExport (src/std/mod.rs)

```rust
pub struct ModuleExport {
    pub short_name: &'static str,      // 短名称
    pub qualified_name: &'static str, // 完整路径
    pub signature: &'static str,      // 函数签名
}
```

**问题**: 只用于 std 模块，需要泛化为通用模块导出

#### 4.2.2 NativeDeclaration (src/std/io.rs 等)

```rust
pub struct NativeDeclaration {
    pub name: &'static str,
    pub native_name: &'static str,
    pub signature: &'static str,
    pub doc: &'static str,
    pub implemented: bool,
}
```

**说明**: `NativeDeclaration` 是为 FFI（Foreign Function Interface）设计的，专门用于 YaoXiang 调用 Rust 函数。用户通过 std.ffi `native` 函数实现与 Rust 的互操作。

**不需要改动**: 用户模块使用 YaoXiang 本身编写，不需要 NativeDeclaration。模块系统只需加载 YaoXiang 源文件即可。

**设计分离**：
| 模块类型 | 实现方式 | 加载方式 |
|---------|---------|----------|
| FFI 模块 | Rust + NativeDeclaration | 内置注册 |
| 用户模块 | YaoXiang 源文件 | 文件系统加载 |

#### 4.2.3 StmtKind::Use (AST)

```rust
StmtKind::Use {
    path: String,           // 模块路径
    items: Option<Vec<String>>, // 导入项
    alias: Option<String>,     // 别名
}
```

**现状**: 解析器已支持完整语法，类型检查未完全实现

## 5. 实施计划

### 5.1 阶段 1: 设计通用模块接口

**目标**: 定义通用的模块注册和查询接口

**可能需要更改的文件**:
- 新增 `src/frontend/module.rs` - 模块系统核心接口

**设计内容**:
```rust
// 模块导出项
pub struct ModuleExport {
    pub name: String,           // 导出名称
    pub full_path: String,     // 完整路径
    pub kind: ExportKind,      // 函数/常量/子模块
    pub type_info: TypeInfo,   // 类型信息
}

// 模块注册表
pub trait ModuleRegistry {
    fn get_module(&self, path: &str) -> Option<Module>;
    fn register_module(&mut self, path: String, module: Module);
}

// 模块加载器
pub trait ModuleLoader {
    fn load_module(&self, path: &str) -> Result<Module, ModuleError>;
}
```

### 5.2 阶段 2: 实现模块加载器

**目标**: 从文件系统加载用户模块

**可能需要更改的文件**:
- 新增 `src/frontend/module/loader.rs` - 文件系统模块加载
- 修改 `src/frontend/compiler.rs` - 集成模块加载

**实现内容**:
- 实现模块搜索路径
- 实现模块解析（AST 生成）
- 实现模块缓存

### 5.3 阶段 3: 重构类型检查

**目标**: 使用通用模块系统替代硬编码

**可能需要更改的文件**:
- `src/frontend/typecheck/mod.rs` - 使用通用模块接口
- `src/frontend/typecheck/inference/expressions.rs` - 移除硬编码
- `src/std/mod.rs` - 实现 ModuleRegistry trait

### 5.4 阶段 4: 重构 IR 生成

**目标**: 移除硬编码的命名空间处理

**可能需要更改的文件**:
- `src/middle/core/ir_gen.rs` - 使用通用模块接口

### 5.5 阶段 5: std 模块适配

**目标**: 确保 std 模块兼容

**可能需要更改的文件**:
- `src/std/mod.rs` - 适配新的模块接口

### 5.6 阶段 6: 测试和验证

**目标**: 确保所有功能正常

**可能需要新增的文件**:
- `tests/modules/` - 模块系统测试

## 6. 架构设计（草稿）

```
┌─────────────────────────────────────────────────────────────────┐
│                        编译器前端                                 │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │   Parser     │───▶│ TypeChecker  │───▶│  IR Gen      │   │
│  └──────────────┘    └──────────────┘    └──────────────┘   │
│         │                   │                   │              │
│         ▼                   ▼                   ▼              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │                   Module System                       │    │
│  ├──────────────────────────────────────────────────────┤    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │    │
│  │  │  Registry   │  │  Loader     │  │  Resolver   │ │    │
│  │  │  (模块注册)  │  │  (加载器)   │  │  (名称解析)  │ │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘ │    │
│  └──────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## 7. 循环依赖处理

### 7.1 设计决策：主动报错

模块系统采用**主动报错**策略，检测到循环依赖时立即终止编译并显示清晰错误信息。

**理由**：
- 明确：用户立即知道问题所在
- 有帮助：报错信息可显示循环路径
- 符合直觉：类似 Rust/Go 等语言的做法

### 7.2 检测算法

```rust
// 1. 构建依赖图
//    每个模块 -> 它依赖的模块列表

// 2. 拓扑排序（Kahn 算法）
//    - 入度为0的节点优先
//    - 无法排序 = 存在循环

// 3. 如果排序失败，报告循环依赖
```

### 7.3 循环类型

#### 直接循环
```yaoxiang
// a.yx
use b

// b.yx
use a  // Error: 循环依赖 a <-> b
```

#### 间接循环
```yaoxiang
// a.yx
use b

// b.yx
use c

// c.yx
use a  // Error: 间接循环 a -> b -> c -> a
```

#### 自引用
```yaoxiang
// a.yx
use a  // Error: 自引用
```

### 7.4 报错信息示例

```yaoxiang
error[E1001]: cyclic dependency detected
  --> a.yx:1:1
   |
1  | use b
   | ^^^^^^
   |
note: dependency path: a -> b -> c -> a
help: break the cycle by removing one of these imports
```

### 7.5 实现位置

- **模块加载器** (`src/frontend/module/loader.rs`)
- 在模块图构建完成后立即检测
- 编译前期检测，避免后续工作浪费

## 8. 其他限制

### 8.1 模块版本

当前不考虑模块版本管理。

### 8.2 条件编译

当前不考虑条件编译（feature flags）。

## 9. 模块缓存策略

### 9.1 设计目标

参考 RFC-014 的项目结构，支持开发时的热重载：

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/
        ├── foo-1.2.3/
        │   └── src/
        │       └── foo.yx
        └── bar-0.5.0/
```

### 9.2 缓存类型

| 缓存类型 | 时机 | 策略 |
|---------|------|------|
| **编译时缓存** | 编译期间 | 内存缓存，同一编译单元内复用 |
| **开发缓存** | `yaoxiang run` 运行时 | 文件系统缓存，文件变化自动重载 |
| **发布缓存** | 发布构建 | 锁定版本，不监听文件变化 |

### 9.3 热重载机制

```rust
// 开发模式：文件监听
struct HotReloader {
    watcher: notify::Watcher,  // 文件系统监听
}

// 触发条件：
// 1. 依赖的 .yx 文件发生变化
// 2. yaoxiang.toml 或 yaoxiang.lock 变化

// 重载策略：
// - 增量重编译变化的模块
// - 重新构建依赖图
// - 检测循环依赖
```

### 9.4 实现位置

- **模块缓存**：`src/frontend/module/cache.rs`
- **热重载**：`src/frontend/module/hot_reload.rs`
- **集成点**：`src/frontend/compiler.rs`

---

## 10. 编译错误信息优化

### 10.1 设计目标

参考 Rust 的错误信息详细程度，提供清晰、有帮助的错误提示。

### 10.2 错误类型和示例

#### 10.2.1 模块未找到

```yaoxiang
error[E1001]: module not found: 'my_module'
  --> main.yx:1:1
   |
1  | use my_module
   | ^^^^^^^^^^^^^^
   |
help: check if the file exists at one of these locations:
  - ./my_module.yx
  - ./my_module/index.yx
  - ./.yaoxiang/vendor/my_module-*/src/my_module.yx
note: you may need to add 'my_module' to your dependencies in yaoxiang.toml
```

#### 10.2.2 函数未导出

```yaoxiang
error[E1002]: export not found: 'undefined_func'
  --> main.yx:3:5
   |
3  | my_module.undefined_func()
   |     ^^^^^^^^^^^^^^^^^^^^
   |
help: 'my_module' exports these functions:
  - greet(name: String) -> String
  - add(a: i64, b: i64) -> i64
  - sub(a: i64, b: i64) -> i64
```

#### 10.2.3 循环依赖

```yaoxiang
error[E1003]: cyclic dependency detected
  --> a.yx:1:1
   |
1  | use b
   | ^^^^^
   |
note: dependency path: a -> b -> c -> a
help: break the cycle by removing one of these imports
```

#### 10.2.4 路径解析错误

```yaoxiang
error[E1004]: invalid module path
  --> main.yx:1:1
   |
1  | use ./relative/path
   | ^^^^^^^^^^^^^^^^^^
   |
help: use absolute module names (e.g., use my_module) or configure in yaoxiang.toml
```

### 10.3 实现位置

- **错误定义**：`src/util/diagnostic/error_codes.rs`
- **错误生成**：各模块系统组件

---

## 11. 模块路径解析规则

### 11.1 设计依据

参考 RFC-014 第 125-135 行的模块解析顺序设计。

### 11.2 路径语法

| 语法 | 示例 | 说明 |
|------|------|------|
| 标准路径 | `use foo.bar` | 搜索 vendor/ 和 src/ |
| 子模块 | `use foo.bar.baz` | 嵌套模块 |
| 当前模块 | `use .` | 当前模块（self） |
| 父模块 | `use ..` | 父模块（parent） |

**不支持**：
- 相对路径（`use ./utils`）
- 绝对路径（`use /usr/local/lib`）
- 字符串包名（`use "@org/package"`）

### 11.3 搜索顺序

```
use foo.bar.baz;

查找顺序:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/ 目录)
2. ./src/foo/bar/baz.yx                      (本地模块)
3. $YXPATH/foo/bar/baz.yx                    (全局路径，预留)
4. $YXLIB/std/foo/bar/baz.yx                 (标准库)
```

### 11.4 std 模块特殊处理

```
use std.io          -> 映射到 $YXLIB/std/io/
use std.math        -> 映射到 $YXLIB/std/math/
```

### 11.5 实现位置

- **模块解析器**：`src/frontend/module/resolver.rs`
- **路径搜索**：`src/frontend/module/loader.rs`

---

## 12. 统一接口设计

### 12.1 设计目标

std 模块和用户模块使用统一的模块接口，便于扩展。

### 12.2 核心接口

```rust
// 模块
pub trait Module {
    fn path(&self) -> &str;
    fn exports(&self) -> &HashMap<String, Export>;
}

// 模块注册表
pub trait ModuleRegistry {
    fn get(&self, path: &str) -> Option<Box<dyn Module>>;
    fn register(&mut self, path: String, module: Box<dyn Module>);
}

// 模块加载器
pub trait ModuleLoader {
    fn load(&self, path: &str) -> Result<Box<dyn Module>, ModuleError>;
}
```

### 12.3 具体实现

| 实现 | 用途 |
|------|------|
| `StdModule` | std 标准库（内置） |
| `UserModule` | 用户定义的模块（文件加载） |
| `VendorModule` | .yaoxiang/vendor/ 依赖 |
| `CompositeRegistry` | 组合多个注册表，按搜索顺序查询 |

### 12.4 组合注册表

```rust
struct CompositeRegistry {
    // 搜索顺序：前一个优先
    std: StdModule,
    vendor: VendorRegistry,
    user: FileModuleRegistry,
}

impl ModuleRegistry for CompositeRegistry {
    fn get(&self, path: &str) -> Option<Box<dyn Module>> {
        // 按顺序尝试每个注册表
        self.std.get(path)
            .or_else(|| self.vendor.get(path))
            .or_else(|| self.user.get(path))
    }
}
```

### 12.5 实现位置

- **接口定义**：`src/frontend/module/mod.rs`
- **std 实现**：`src/frontend/module/std.rs`
- **文件加载**：`src/frontend/module/file.rs`
- **组合器**：`src/frontend/module/registry.rs`

---

## 13. 实施检查清单

- [ ] 模块缓存策略
- [ ] 热重载机制
- [x] 编译错误信息（E5001-E5007 已定义）
- [x] 模块路径解析（`src/frontend/module/resolver.rs`）
- [x] 统一模块接口（`src/frontend/module/mod.rs`）
- [x] std 模块适配（`src/frontend/module/registry.rs` + `src/std/mod.rs`）
- [x] 循环依赖检测（`src/frontend/module/loader.rs` Kahn 算法）
- [x] 类型检查重构 - 移除硬编码（`src/frontend/typecheck/mod.rs`）
- [x] IR 生成重构 - 移除硬编码（`src/middle/core/ir_gen.rs`）
- [x] 表达式推断重构 - 移除硬编码（`src/frontend/typecheck/inference/expressions.rs`）
- [ ] 用户模块文件加载（解析 .yx 文件提取导出项）
- [ ] RFC-014 集成（vendor/、yaoxiang.toml）

---

## 14. 已完成的实施记录

### 14.1 阶段 1：通用模块接口（已完成 ✅）

**新增文件**：
- `src/frontend/module/mod.rs` - 模块系统核心类型定义（`Export`, `ModuleInfo`, `ModuleSource`, `ModuleError` 等）
- `src/frontend/module/registry.rs` - 统一模块注册表（`ModuleRegistry`），自动发现注册 std 模块
- `src/frontend/module/resolver.rs` - 模块路径解析器（`ModuleResolver`），支持 std/vendor/用户模块搜索
- `src/frontend/module/loader.rs` - 模块加载器（`ModuleLoader`），支持循环依赖检测

**设计决策**：
- 使用数据驱动（`ModuleInfo` + `HashMap`）而非 trait 对象，简化接口
- `ModuleRegistry::with_std()` 从各 std 模块的 `native_declarations()` 自动发现导出项
- `ModuleSource` 枚举区分 `Std`/`User`/`Vendor` 模块来源

### 14.2 阶段 2：硬编码消除（已完成 ✅）

**修改的文件和消除的硬编码**：

| 文件 | 消除的硬编码 | 替代方案 |
|------|------------|---------|
| `src/std/mod.rs` | `match module_path { "std" => ..., "std.io" => ... }` | 委托给 `ModuleRegistry::with_std()` |
| `src/frontend/typecheck/mod.rs` | `let std_modules = ["std.io", "std.math", ...]` | 通过 `registry.std_submodule_names()` 动态获取 |
| `src/frontend/typecheck/mod.rs` | `crate::std::get_module_exports(path)` 在 Use 处理中 | 通过 `self.env.module_registry.get(path)` 查询 |
| `src/frontend/typecheck/inference/expressions.rs` | `matches!(name.as_str(), "io" \| "math" \| "net" \| "concurrent")` | 通过 `std_submodules` 列表动态判断 |
| `src/middle/core/ir_gen.rs` | `use crate::std::{concurrent, io, math, net}` + 手工构建映射 | 通过 `ModuleRegistry::with_std()` 自动生成 `NATIVE_NAMES`/`SHORT_TO_QUALIFIED`/`STD_SUBMODULES` |

### 14.3 阶段 3：错误码定义（已完成 ✅）

**新增错误码**：
- `E5005` - 无效的模块路径
- `E5006` - 重复导入
- `E5007` - 模块导出提示

### 14.4 验证结果

- ✅ 全部 1464 测试通过（1428 单元 + 30 文档 + 6 集成，0 失败）
- ✅ 运行示例文件正常输出
- ✅ `cargo check` 无编译错误

---

**注意**: 用户模块的 .yx 文件解析和 vendor 集成尚待实现，需要编译器支持多文件编译。
