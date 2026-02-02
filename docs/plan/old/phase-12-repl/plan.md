# REPL 实现计划

## 目标
实现交互式终端，支持：
- 多行输入
- 状态保持（函数/变量跨调用保留）
- 历史命令
- 所有语言特性

## 核心问题

### 问题 1：状态保持
当前 `run()` 每次调用都创建全新 `Compiler`：
```rust
pub fn run(source: &str) -> Result<()> {
    let mut compiler = frontend::Compiler::new();  // 每次新建！
    let module = compiler.compile(source)?;
    // ...
}
```
**需要**：持久化的 `Compiler` 实例

### 问题 2：多行输入
检测语句是否完整：
- `fn foo() {` → 继续输入
- `}` → 执行

检测方法：
1. 括号计数：`{` `(` `[` +1，`)` `]` `-1`
2. 引号状态：跟踪 `"` 是否闭合
3. 关键字检测：行尾是否在关键字后

### 问题 3：累积 IR
每次输入需要合并到已有模块：
```rust
repl.execute("let x = 1")  // 添加到现有 IR
repl.execute("print(x)")   // 使用之前的定义
```

## 方案设计

### 1. 可重用 Compiler
```rust
pub struct Compiler {
    type_env: typecheck::TypeEnvironment,  // 保持状态
}
```

### 2. REPL 结构
```rust
pub struct REPL {
    compiler: Compiler,
    input_buffer: String,
    paren_depth: usize,      // 括号深度
    in_string: bool,         // 是否在字符串内
    history: Vec<String>,    // 命令历史
}
```

### 3. 输入处理流程
```
读取行 → 检测完整性 → 不完整继续收集 → 完整则编译执行
```

### 4. 错误处理
- 单行语法错误：清空 buffer，重新开始
- 多行结构错误：提示 "unclosed bracket"

## 实现步骤

### Phase 1: 基础 REPL
- [ ] 创建 `src/repl.rs`
- [ ] 实现 `REPL::new()`
- [ ] 实现 `REPL::read_line()`（单行）
- [ ] 实现 `REPL::step(&str)` 执行单行
- [ ] 添加 `main.rs` 的 REPL 入口

### Phase 2: 多行支持
- [ ] 实现括号计数检测
- [ ] 实现引号状态跟踪
- [ ] 实现 "..." 提示符

### Phase 3: 状态保持
- [ ] 修改 `Compiler` 可重用
- [ ] REPL 累积符号表
- [ ] 累积 IR（可选：增量编译）

### Phase 4: 交互体验
- [ ] 命令历史（`rustyline` 或手写）
- [ ] `:type <expr>` 查看类型
- [ ] `:load <file>` 加载文件
- [ ] `:clear` 清空状态

## 关键文件变更

| 文件 | 变更 |
|------|------|
| `src/lib.rs` | 添加 `pub mod repl` |
| `src/repl.rs` | 新建，REPL 实现 |
| `src/frontend/mod.rs` | `Compiler` 持久化 |
| `src/main.rs` | REPL 入口 |

## 风险点

1. **符号表膨胀**：长期运行可能导致内存增长
   - 方案：添加 `:reset` 命令

2. **增量编译复杂度**：当前 AST → IR 是整体流程
   - 方案：先支持单文件累积，复杂增量后续

3. **类型推断循环**：重复定义需检测冲突
   - 方案：覆盖前警告或报错

## 外部依赖

推荐使用 `rustyline`：
```toml
rustyline = { version = "13", features = ["derive"] }
```

## 优先级

P0: 基础单行 REPL（可执行代码）
P1: 多行输入支持
P2: 状态保持
P3: 历史/补全（锦上添花）
