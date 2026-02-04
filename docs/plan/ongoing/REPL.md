# REPL 实现文档

## 概述

REPL（Read-Eval-Print Loop）是 YaoXiang 语言的交互式解释器，为开发者提供即时反馈的编程环境。用户可以直接输入代码片段并立即查看执行结果，极大提升原型开发和语言学习效率。

REPL 模块位于 `src/backends/dev/repl.rs`，与开发 Shell（`src/backends/dev/shell.rs`）紧密集成，共同构成 YaoXiang 的交互式开发环境。REPL 专注于代码评估，而 Shell 提供更丰富的开发命令集，包括文件管理、调试等功能。

当前 REPL 实现支持多行输入、表达式完整性检测、历史记录管理、特殊命令处理等核心功能。代码评估通过集成前端编译器实现，支持实时语法检查和错误报告。

## 架构设计

### 模块位置与依赖关系

```
src/backends/dev/
├── repl.rs         # REPL 主模块
├── shell.rs        # 开发 Shell（包装 REPL）
└── debugger.rs     # 调试器集成

src/backends/common/
├── value.rs        # RuntimeValue 运行时值类型
├── heap.rs         # 堆内存管理
└── mod.rs          # 公共模块导出

src/backends/interpreter/
├── executor.rs     # 指令执行器
├── frames.rs       # 调用帧管理
└── registers.rs    # 寄存器管理
```

REPL 模块依赖以下核心类型：`RuntimeValue` 提供统一的运行时值表示，`Interpreter` 负责代码执行，`Compiler` 处理源代码编译。模块间通过清晰定义的接口通信，确保各组件可独立测试和演进。

### 核心数据结构

#### REPLConfig 配置结构

```rust
#[derive(Debug, Clone)]
pub struct REPLConfig {
    /// 标准提示符，显示在每行输入开头
    pub prompt: String,
    /// 多行输入提示符，用于块结构续行
    pub multi_line_prompt: String,
    /// 语法高亮开关（保留接口，未来可集成终端高亮库）
    pub syntax_highlight: bool,
    /// 自动缩进开关（保留接口，需结合输入库实现）
    pub auto_indent: bool,
    /// 历史记录最大条目数，防止内存无限增长
    pub history_size: usize,
}
```

配置结构采用可扩展设计，`syntax_highlight` 和 `auto_indent` 字段为未来增强预留接口。当前标准提示符为 `">> "`，多行提示符为 `".. "`，历史记录默认存储 1000 条条目。

#### REPLResult 评估结果枚举

```rust
#[derive(Debug)]
pub enum REPLResult {
    /// 评估产生实际值，需打印显示
    Value(RuntimeValue),
    /// 评估无返回值（unit 类型）
    Ok,
    /// 评估过程发生错误
    Error(String),
    /// 用户主动退出（:quit 或 Ctrl-D）
    Exit,
}
```

结果枚举清晰区分四种评估 outcome，便于主循环分别处理。`Value` 包装实际运行时值，`Error` 携带人类可读的错误信息，`Exit` 信号通知主循环终止。

#### REPL 主结构

```rust
#[derive(Debug)]
pub struct REPL {
    /// REPL 配置
    config: REPLConfig,
    /// 代码解释器实例
    interpreter: Interpreter,
    /// 已输入历史（支持上下箭头遍历）
    history: Vec<String>,
    /// 当前输入缓冲区（多行续接用）
    buffer: String,
    /// 当前行计数（0 表示新表达式开始）
    line_count: usize,
}
```

REPL 结构将配置、内部状态、执行引擎分离。`buffer` 存储用户正在输入的多行表达式，`line_count` 追踪续行状态，两者共同实现块结构（如函数定义、if 表达式）的多行输入支持。

## 工作流程

### 主循环流程

```
┌─────────────────────────────────────────────────────┐
│                    REPL.run()                        │
├─────────────────────────────────────────────────────┤
│  1. 打印欢迎信息                                     │
│  2. 进入主循环                                       │
│     ┌─────────────────────────────────────────────┐ │
│     │  read_line()                                │ │
│     │  ├─ 显示提示符                               │ │
│     │  ├─ 读取一行输入                             │ │
│     │  ├─ 检测特殊命令（:quit, :help 等）          │ │
│     │  └─ 添加到历史记录                           │ │
│     ├─ 判断表达式完整性                             │ │
│     │  └─ 不完整 → 继续 read_line()                │ │
│     └─ 完整 → evaluate()                           │
│         ├─ 包装代码为完整函数                       │
│         ├─ 调用 Compiler 编译                       │
│         └─ 返回结果                                 │
│  3. 处理结果并循环或退出                             │
└─────────────────────────────────────────────────────┘
```

主循环遵循经典的 REPL 模式：读取输入、评估代码、打印结果。关键设计决策是将表达式完整性判断与输入读取分离，允许用户在多行输入过程中逐步构建复杂表达式。

### 表达式完整性检测

`is_complete()` 方法通过统计括号、花括号、方括号的配对状态判断表达式是否完整。算法考虑字符串转义，确保不会将字符串内部的括号误判为表达式分隔符。

```rust
fn is_complete(&self, code: &str) -> bool {
    let mut braces = 0;   // { }
    let mut brackets = 0; // [ ]
    let mut parens = 0;   // ( )
    let mut in_string = false;
    let mut escaped = false;

    for c in code.chars() {
        // 处理转义字符
        if escaped { escaped = false; continue; }
        if c == '\\' { escaped = true; continue; }

        // 处理字符串
        if c == '"' { in_string = !in_string; continue; }

        // 非字符串区域计数
        if !in_string {
            match c {
                '{' => braces += 1,
                '}' => { if braces == 0 { return true; } braces -= 1; }
                '[' => brackets += 1,
                ']' => { if brackets == 0 { return true; } brackets -= 1; }
                '(' => parens += 1,
                ')' => { if parens == 0 { return true; } parens -= 1; }
                _ => {}
            }
        }
    }

    braces == 0 && brackets == 0 && parens == 0 && !in_string && !escaped
}
```

完整性检测的边界情况处理值得注意：当遇到不匹配的闭合括号时（如 `}` 但 braces 已为 0），方法立即返回 `true`，表示前序表达式已完整。这允许用户输入不完整的续行并继续输入。

### 代码评估流程

```rust
fn evaluate(&mut self, code: &str) -> Result<REPLResult, io::Error> {
    // 1. 将代码包装为完整函数
    let wrapped = format!(
        "main() -> () = () => {{\n{}\n}}",
        code
    );

    // 2. 调用前端编译器
    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source("<repl>", &wrapped) {
        Ok(_module) => Ok(REPLResult::Ok),
        Err(e) => {
            // 3. 错误处理（精简输出）
            let lines: Vec<&str> = error_msg.lines().collect();
            if lines.len() > 2 {
                Ok(REPLResult::Error(
                    lines[lines.len() - 2..].join("\n")
                ))
            } else {
                Ok(REPLResult::Error(error_msg))
            }
        }
    }
}
```

代码包装策略将用户输入嵌入 `main() -> () = () => { ... }` 函数中。这种设计确保无论用户输入表达式还是语句，都能被编译器正确处理。包装后的代码通过标准编译流程，最终可被解释器执行。

错误输出经过精简处理，移除文件上下文行，只保留核心错误信息，使交互体验更友好。

## 配置与定制

### 默认配置

```rust
impl Default for REPLConfig {
    fn default() -> Self {
        Self {
            prompt: ">> ".to_string(),
            multi_line_prompt: ".. ".to_string(),
            syntax_highlight: true,
            auto_indent: true,
            history_size: 1000,
        }
    }
}
```

### 自定义配置示例

```rust
let config = REPLConfig {
    prompt: "yx> ".to_string(),
    multi_line_prompt: "... ".to_string(),
    syntax_highlight: false,  // 禁用语法高亮
    auto_indent: false,       // 禁用自动缩进
    history_size: 500,        // 减少历史记录
};

let mut repl = REPL::with_config(config);
```

### 创建方式

```rust
// 使用默认配置
let repl = REPL::new();

// 使用自定义配置
let repl = REPL::with_config(custom_config);
```

## 特殊命令参考

### 可用命令列表

| 命令 | 别名 | 功能描述 |
|------|------|----------|
| `:quit` | `:q` | 退出 REPL，返回上层 Shell |
| `:help` | `:h` | 显示可用命令列表 |
| `:clear` | `:c` | 清除当前缓冲区（放弃未完成的输入） |
| `:history` | `:hist` | 显示历史记录（带行号） |

### 命令处理流程

```rust
fn handle_command(&mut self, command: &str) -> Result<REPLResult, io::Error> {
    match command {
        ":quit" | ":q" => Ok(REPLResult::Exit),
        ":help" | ":h" => {
            // 显示帮助信息
            Ok(REPLResult::Ok)
        }
        ":clear" | ":c" => {
            // 清空缓冲区
            self.buffer.clear();
            self.line_count = 0;
            Ok(REPLResult::Ok)
        }
        ":history" | ":hist" => {
            // 遍历显示历史
            for (i, line) in self.history.iter().enumerate() {
                tlog!(info, /* ... */);
            }
            Ok(REPLResult::Ok)
        }
        _ => {
            // 未知命令提示
            Ok(REPLResult::Ok)
        }
    }
}
```

## 与 DevShell 的集成

### Shell 调用 REPL

```rust
":repl" | "repl" => {
    if let Err(e) = self.repl.run() {
        ShellResult::Error(format!("REPL error: {}", e))
    } else {
        ShellResult::Success
    }
}
```

### 状态流转

```
Shell ──:repl──> REPL.run()
                    │
                    ├─ :quit ──> 返回 Shell
                    └─ Ctrl-D ──> 返回 Shell
```

### Shell 辅助功能

Shell 额外提供以下 REPL 无法直接访问的功能：

| Shell 命令 | 功能描述 |
|-----------|----------|
| `:cd <path>` | 切换工作目录 |
| `:pwd` | 显示当前目录 |
| `:ls [path]` | 列出目录内容 |
| `:run <file>` | 运行文件并计时 |
| `:load <file>` | 加载文件到环境 |
| `:debug <file>` | 启动调试模式 |
| `:break <fn> <offset>` | 设置断点 |

## 技术实现细节

### 输入行读取

```rust
fn read_line(&mut self) -> Result<REPLResult, io::Error> {
    // 1. 确定提示符（单行或多行）
    let prompt = if self.line_count == 0 {
        &self.config.prompt
    } else {
        &self.config.multi_line_prompt
    };

    // 2. 打印提示符并刷新输出缓冲
    tlog!(debug, MSG::ReplPrompt, &prompt.to_string());
    io::stdout().flush()?;

    // 3. 读取标准输入
    let mut line = String::new();
    let stdin = io::stdin();

    if stdin.read_line(&mut line)? == 0 {
        return Ok(REPLResult::Exit);  // Ctrl-D 检测
    }

    // 4. 处理命令或添加到缓冲区
    // ...
}
```

### 历史记录管理

```rust
// 仅非空行加入历史
if !line.is_empty() {
    self.history.push(line.clone());
}

// 历史记录用于提供上下箭头遍历
//（注：当前实现存储到 Vec，终端交互需结合 readline 库）
```

历史记录仅存储非空行，避免大量空白行占用空间。未来可集成 `rustyline` 或 `liner` 等成熟库，提供真正的交互式历史浏览。

### 文件加载

```rust
pub fn load_file(&mut self, path: &Path) -> Result<REPLResult, io::Error> {
    let source = std::fs::read_to_string(path)?;

    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source(&path.display().to_string(), &source) {
        Ok(_module) => Ok(REPLResult::Ok),
        Err(e) => Ok(REPLResult::Error(format!("{}", e))),
    }
}
```

`load_file()` 复用编译器接口，支持加载并执行 `.yx` 源文件。文件路径使用 `Path` 类型，确保跨平台兼容性。

### i18n 消息映射

REPL 使用统一的消息系统（`src/util/i18n/mod.rs`），所有用户可见文本通过 `MSG` 枚举访问：

```rust
MSG::ReplWelcome     // 欢迎信息
MSG::ReplHelp        // 帮助信息
MSG::ReplError       // 错误前缀
MSG::ReplValue       // 值输出格式
MSG::ReplPrompt      // 提示符格式
// ...
```

这种设计支持多语言扩展，只需提供不同语言的 `MSG` 实现。

## 测试覆盖

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_repl_new() {
        let repl = REPL::new();
        assert!(repl.history.is_empty());
    }

    #[test]
    fn test_repl_is_complete() {
        let repl = REPL::new();

        // 完整表达式
        assert!(repl.is_complete("1 + 2"));
        assert!(repl.is_complete("let x = 42"));
        assert!(repl.is_complete("fn foo() { 1 }"));

        // 不完整表达式
        assert!(!repl.is_complete("fn foo() {"));
        assert!(!repl.is_complete("if true {"));
        assert!(!repl.is_complete("{"));
    }
}
```

## 未来演进方向

### 短期增强

1. **集成 readline 库**：提供真正的交互式编辑（Emacs/Vi 模式、增量搜索）
2. **语法高亮**：集成 ANSI 转义序列高亮，支持关键字、数字、字符串
3. **Tab 自动补全**：符号补全、函数参数提示
4. **多行编辑**：支持通过 `(` 或 `{` 触发的智能续行

### 中期目标

1. **增量类型检查**：仅对变更部分重新检查，避免全量编译延迟
2. **内联结果打印**：对复杂值提供摘要，`:expand` 查看详情
3. **持久化历史**：跨会话保留历史记录
4. **模块导入**：REPL 内直接导入已编译模块

### 长期愿景

1. **Jupyter Kernel**：提供 IPython/Jupyter 集成
2. **图形 REPL**：可视化数据结构、调用栈、时间线
3. **远程 REPL**：通过网络 REPL 调试远程进程
4. **性能剖析**：`:profile` 命令输出执行时间、内存分配统计

## 相关文件索引

| 文件 | 职责 |
|------|------|
| `src/backends/dev/repl.rs` | REPL 主模块 |
| `src/backends/dev/shell.rs` | 开发 Shell |
| `src/backends/dev/debugger.rs` | 调试器 |
| `src/backends/common/value.rs` | 运行时值类型 |
| `src/backends/common/heap.rs` | 堆内存管理 |
| `src/backends/interpreter/mod.rs` | 解释器模块 |
| `src/util/i18n/mod.rs` | 国际化消息 |
| `src/frontend/Compiler.rs` | 编译器前端 |
