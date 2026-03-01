# LSP 定义解析问题与解决方案

> **任务**：修复 LSP 跳转定义功能
> **日期**：2026-02-28
> **状态**：待处理
> **发现时间**：2026-02-28

---

## 概述

本文档记录 LSP 服务器中与"跳转到定义"（Go to Definition）功能相关的问题及解决方案。

---

## 问题 1：Use 语句导入的符号无法定位

### 问题描述

`use std.list` 语句导入的模块符号（如 `push`、`pop` 等）无法通过跳转定义功能找到。

### 根因分析

**位置**：`src/lsp/world.rs:98-169`

```rust
// update_index_from_ast 只处理这些语句类型
StmtKind::Var { ... }      // ✅ 处理
StmtKind::Fn { ... }       // ✅ 处理
StmtKind::TypeDef { ... }  // ✅ 处理
StmtKind::MethodBind { ... } // ✅ 处理
StmtKind::Use { ... }      // ❌ 没有处理！
```

**流程问题**：
1. 用户代码使用 `use std.list` 导入模块
2. 类型检查阶段会将导入的符号注册到 `TypeEnvironment`
3. 但 `update_index_from_ast` 只解析 AST 顶层语句，**不处理 Use 语句**
4. 因此 Use 导入的符号没有进入 `SymbolIndex`
5. LSP 跳转定义时找不到这些符号

### 影响

- 用户使用 `use std.list` 后，无法跳转到 `std.list.push` 等函数的定义
- 补全功能也无法提供标准库函数的定义

---

## 问题 2：同名函数跳转到错误位置

### 问题描述

当同一个函数名出现在多个文件中时，跳转定义会返回所有同名定义，无法精确跳转到正确的位置。

### 根因分析

**位置**：`src/lsp/handlers/definition.rs:51-66`

```rust
// 当前实现：只用名字匹配，返回所有同名定义
let symbols = world.symbol_index().find_by_name(&ident.name);

let locations: Vec<Location> = symbols
    .iter()
    .filter_map(|sym| { ... })  // 只要名字相同就返回，不考虑上下文
    .collect();
```

**问题**：
1. `SymbolIndex` 仅包含顶层符号（Var, Fn, TypeDef, MethodBind）
2. 查找时只按名字匹配，不考虑：
   - 符号的类型（变量 vs 函数）
   - 符号的作用域（局部 vs 全局）
   - 调用时的类型上下文
3. 返回所有同名定义，由客户端选择（通常是第一个）

### 影响

- 多个文件中存在同名函数时，跳转可能跳到错误的位置
- 需要用户手动选择正确的定义

---

## 问题 3：局部变量和函数参数无法定位

### 问题描述

函数内部定义的局部变量和函数参数无法通过跳转定义功能找到。

### 根因分析

**位置**：`src/lsp/world.rs:107-168`

```rust
// update_index_from_ast 只处理模块顶层语句
for stmt in &module.items {
    match &stmt.kind {
        StmtKind::Var { name, .. } => {
            // 只处理顶层变量
        }
        StmtKind::Fn { name, params, .. } => {
            // 只处理函数定义，不处理函数体的参数和局部变量
        }
        // ...
    }
}
```

**问题**：
1. `SymbolIndex` 只从**模块顶层语句**提取符号
2. 函数参数、局部变量等**嵌套作用域中的符号**没有进入索引
3. LSP 跳转定义时无法找到这些符号

---

## 问题 4：标准库没有 YaoXiang 源文件

### 问题描述

标准库（std.list、std.io 等）使用 Rust 实现，没有对应的 .yx 源文件。

### 根因分析

**当前架构**：

```
用户代码 (.yx)              标准库 (Rust)
      │                         │
      ▼                         ▼
  解析 → AST             StdModule → NativeExport
      │                         │
      ▼                         ▼
 SymbolIndex ◄──────── ModuleRegistry
                           (LSP 不可见)
```

**问题**：
1. 标准库通过 `StdModule` trait 注册到 `ModuleRegistry`
2. 每个模块有 `NativeExport` 列表，包含：
   - `name`: 短名称（如 "push"）
   - `native_name`: 完全限定名（如 "std.list.push"）
   - `signature`: 函数签名（如 "(list: List, item: Any) -> List"）
   - **注意**：没有 `Span`（Rust 函数没有 YaoXiang 源码位置）
3. LSP 服务器不知道 `ModuleRegistry` 的存在
4. 因此无法定位标准库函数的定义

---

## 解决方案

### 方案 A：标准库 YaoXiang 接口文件

**核心思想**：为标准库创建 YaoXiang 接口文件，使用 ExternalBindingStmt 绑定到 Rust 函数。

**文件结构**：
```
~/.yaoxiang/std/                    # 安装目录（全局标准库）
├── list.yx                          # list 模块接口
│   push: (list: List, item: Any) -> List = ...
│   pop: (list: List) -> Any = ...
├── io.yx                            # io 模块接口
│   print: (...args) -> () = ...
│   println: (...args) -> () = ...
│   read_line: () -> String = ...
│   read_file: (path: String) -> String = ...
│   write_file: (path: String, content: String) -> Bool = ...
│   append_file: (path: String, content: String) -> Bool = ...
│   format_fallback: (value: Any, type_name: String) -> String = ...
├── dict.yx
├── string.yx
└── ...

项目目录/
├── main.yx
└── .yaoxiang/
    └── vendor/
        └── std/                     # 项目本地标准库（覆盖全局）
            └── list.yx              # 可选：覆盖全局的 list 接口
```

**接口文件格式**：
```yaoxiang
// io.yx - 标准库 IO 模块接口
// 仅供 LSP 跳转和类型查看，不参与实际执行


print: (...args) -> () = {
    // 输出到标准输出
    ... // 实现由 Rust 提供
}


println: (...args) -> () = {
    // 输出到标准输出并换行
    ...
}


read_line: () -> String = {
    // 从标准输入读取一行
    ...
}

read_file: (path: String) -> String = {
    /* 读取文件内容
    @param path 文件路径 */
    ...
}

write_file: (path: String, content: String) -> Bool = {
    /* 写入文件内容，覆盖原有内容
    @param path 文件路径
    @param content 文件内容
    @return 是否成功 */
    ...
}


append_file: (path: String, content: String) -> Bool = {
    /* 追加写入文件内容
    @param path 文件路径
    @param content 文件内容
    @return 是否成功 */
    ...
}

format_fallback: (value: Any, type_name: String) -> String = {
    /* 将任意类型格式化为字符串
    @param value 任意值
    @param type_name 值的类型名称
    @return 格式化后的字符串 */
    ...
}
```

**语法说明**：
- 等号右边用 `...` 表示跳过实际实现
- 如果需要添加函数文档，使用块语法：
  ```yaoxiang
  print: (...args) -> () = {
      // 注释文档
      ...
  }
  ```

**模块查找顺序**（类似 Python）：
```
use std.list 查找顺序：
1. 项目目录/.yaoxiang/vendor/std/list.yx  ← 优先（存在则使用）
2. ~/.yaoxiang/std/list.yx               ← 回退（默认）
```

**优点**：
- 用户代码和标准库使用同一套语法
- LSP 可以直接解析这些文件提供跳转和补全
- 标准库接口本身就是文档
- 易于维护
- 支持项目本地覆盖全局标准库

**实现步骤**：

1. **自动生成工具**：编写代码生成工具，从 Rust 代码中的 `NativeExport` 自动生成 `.yx` 接口文件
   - 输入：`src/std/io.rs` 中的 `NativeExport` 定义
   - 输出：`.yaoxiang/std/io.yx` 接口文件
   - 生成规则：
     - `name` → 函数名
     - `signature` → 类型注解
     - `native_name` → 不写入（仅用于 Rust 绑定）

2. **集成到构建流程**：在 Cargo 构建时自动运行生成工具
   ```rust
   // build.rs 或独立的生成脚本
   fn main() {
       // 读取 src/std/*.rs
       // 解析 NativeExport 定义
       // 生成 .yaoxiang/std/*.yx 文件
   }
   ```

3. **修改模块解析逻辑**：支持双路径查找（项目优先 → 全局回退）

4. **修改 LSP 服务器**：加载接口文件到符号索引，提供跳转和补全

---

### 方案 B：修复同名函数精确匹配

**核心思想**：利用 SemanticDB 中的类型信息和作用域信息进行精确匹配。

**当前已有的资源**：
- `SemanticDB`：包含更精确的符号信息（类型、作用域）
- `symbol_defs`：符号名 → 定义位置列表
- `symbol_refs`：符号名 → 引用位置列表

**实现步骤**：
1. 修改 `handle_definition` 函数，优先使用 `SemanticDB` 进行查找
2. 利用光标位置的上下文（表达式类型、作用域）进行精确匹配
3. 如果 `SemanticDB` 中找不到，再回退到 `SymbolIndex`

---

### 方案 C：处理局部变量和函数参数

**核心思想**：将函数内部的符号也加入索引。

**实现步骤**：
1. 修改 `update_index_from_ast`，遍历函数体
2. 提取函数参数和局部变量
3. 为每个符号记录其作用域层级
4. 查找时利用作用域信息进行过滤

---

## 建议优先级

| 优先级 | 问题 | 解决方案 |
|--------|------|----------|
| P1 | 同名函数跳转错误 | 方案 B：使用 SemanticDB 精确匹配 |
| P2 | 局部变量无法定位 | 方案 C：扩展符号索引范围 |
| P3 | Use 导入符号无法定位 | 方案 A：创建标准库接口文件 |
| P4 | 标准库无法定位 | 方案 A：创建标准库接口文件 |

---

## 相关代码位置

| 文件 | 说明 |
|------|------|
| `src/lsp/handlers/definition.rs` | 跳转定义处理函数 |
| `src/lsp/world.rs` | 符号索引更新逻辑 |
| `src/frontend/core/lexer/symbols.rs` | SymbolIndex 定义 |
| `src/frontend/typecheck/semantic_db.rs` | SemanticDB 定义 |
| `src/frontend/typecheck/mod.rs` | 类型检查器（处理 Use 语句） |
| `src/std/mod.rs` | 标准库模块定义 |
| `src/frontend/module/registry.rs` | 模块注册表 |

---

## 实现过程

### 问题依赖关系

```
┌─────────────────────────────────────────────────────────────┐
│                        实现依赖图                           │
└─────────────────────────────────────────────────────────────┘

问题 4：标准库接口文件
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 1. 自动生成工具（从 NativeExport → .yx 文件）      │
    │  │ 2. 模块双路径查找（项目 → 全局）                    │
    │  │ 3. LSP 加载接口文件到符号索引                       │
    │  └─────────────────────────────────────────────────────┘
    ▼
问题 1：Use 语句符号定位
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 4. update_index_from_ast 处理 Use 语句              │
    │  │ 5. 将 use 导入的符号加入索引                       │
    │  └─────────────────────────────────────────────────────┘
    ▼
问题 2：同名函数跳转错误  ←─┐
    │                         │
    │  ┌──────────────────┐   │
    │  │ 6. 使用 SemanticDB│   │
    │  │    精确匹配      │   │
    │  └──────────────────┘   │
    │                         │
问题 3：局部变量无法定位  ──┘
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 7. 扩展符号索引范围（遍历函数体）                  │
    │  │ 8. 记录作用域层级                                   │
    │  └─────────────────────────────────────────────────────┘
    ▼
   全部完成
```
---

## 实现优先级

| 顺序 | 问题 | 复杂度 | 理由 |
|------|------|--------|------|
| 1 | 问题 4：标准库接口 | 中 | 基础设施，其他问题可能依赖 |
| 2 | 问题 1：Use 符号定位 | 低 | 修复后标准库跳转可用 |
| 3 | 问题 2：同名函数跳转 | 中 | 改进查找算法 |
| 4 | 问题 3：局部变量 | 高 | 需要遍历函数体 |

---

## 参考

- RFC-004：柯里化方法的多位置联合绑定设计（ExternalBindingStmt）
- [Language Server Protocol 规范](https://microsoft.github.io/language-server-protocol/)
