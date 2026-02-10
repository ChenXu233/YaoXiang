# YaoXiang 错误提示系统设计文档

## 1. 概述

本文档描述了 YaoXiang 编程语言的语法错误和运行时错误栈提示系统的设计方案。目标是提供与 Rust 编译器相媲美的友好错误提示，帮助开发者快速定位和修复问题。

### 1.1 设计目标

- **用户友好性**：错误信息清晰、易懂，包含代码片段和下划线标注
- **可操作性**：提供修复建议和相关文档链接
- **完整性**：覆盖语法错误、类型错误、运行时错误
- **一致性**：统一的错误格式和输出风格

### 1.2 错误分类

```
错误类型
├── 语法错误 (Syntax Error)
│   ├── 词法错误 (Lexer Error)
│   └── 解析错误 (Parser Error)
├── 类型错误 (Type Error)
├── 语义错误 (Semantic Error)
└── 运行时错误 (Runtime Error)
    ├── 断言错误 (Assertion Error)
    ├── 空指针错误 (Null Pointer Error)
    ├── 索引越界 (Index Out of Bounds)
    ├── 除零错误 (Division by Zero)
    └── 栈溢出 (Stack Overflow)
```

## 2. 错误输出格式规范

### 2.1 通用错误格式

```
error[E0001]: Type mismatch: expected `Int`, found `String`
  --> examples/test.yx:5:12
   |
 5 | let x: Int = "hello";
   |            ^^^^^^^^ expected `Int`, found `String`
   |
   = help: Type mismatch between `Int` and `String`
   = note: Strings cannot be assigned to integer variables
   = note: Consider using `to_int()` method to convert: `"123".to_int()`
   = see https://docs.yaoxiang.dev/errors/E0001 for more information

error[E0002]: Unknown variable `undefined_var`
  --> examples/test.yx:10:8
   |
10 | print(undefined_var);
   |      ^^^^^^^^^^^^^^ not found in this scope
   |
   = help: Variables must be defined before use
   = note: Did you mean to define it with `let`?
   = see https://docs.yaoxiang.dev/errors/E0002 for more information
```

### 2.2 运行时错误栈格式

```
thread 'main' panicked at 'index out of bounds':
  --> examples/test.yx:15:5
   |
15 | let arr = [1, 2, 3];
   |     ^^^ defined here
   |
RuntimeError: Index out of bounds (index: 5, size: 3)
  --> examples/test.yx:20:10
   |
20 | arr[5];
   |     ^ index 5 is out of bounds for array of size 3
   |
Stack trace:
  ┌─────────────────────────────────────┐
  │ examples/test.yx:18:1               │
  │ fn main() {                         │
  │   test_array_access(); // ← called here
  │ }                                   │
  ├─────────────────────────────────────┤
  │ examples/test.yx:12:5               │
  │ fn test_array_access() {            │
  │   let arr = [1, 2, 3];              │
  │   arr[5]; // ← panic here           │
  │ }                                   │
  └─────────────────────────────────────┘

note: run with `RUST_BACKTRACE=1` environment variable to display a full backtrace.
```

### 2.3 多行错误格式

```
error[E0015]: Cannot infer type for parameter `x`
  --> examples/test.yx:3:12
   |
 3 | fn double(x) { // Cannot infer type for parameter `x`
   |            ^ help: consider adding a type annotation: `x: Int`
   |
   = note: Parameters must have explicit types or be inferrable from usage
   = see https://docs.yaoxiang.dev/errors/E0015 for more information
```

## 3. 错误代码规范

### 3.1 错误代码前缀

| 前缀 | 含义 | 范围 |
|------|------|------|
| E0001-E0999 | 语法错误 | 词法和解析 |
| E1001-E1999 | 类型错误 | 类型检查 |
| E2001-E2999 | 语义错误 | 名称解析、作用域 |
| E3001-E3999 | 编译错误 | 代码生成 |
| R4001-R4999 | 运行时错误 | 执行时错误 |

### 3.2 错误代码示例

```yaml
errors:
  E0001:
    title: "Type mismatch"
    description: "Types do not match in an expression"
    category: "Type Error"
    severity: "error"
    examples:
      - code: |
          let x: Int = "hello";
        output: |
          error[E0001]: Type mismatch: expected `Int`, found `String`
      - code: |
          fn add(a: Int, b: Int): Int { a + b }
          add("one", 2);
        output: |
          error[E0001]: Type mismatch: expected `Int`, found `String`
    fixes:
      - "Add type conversion using `.to_int()`"
      - "Change the variable type to match"
    links:
      - "https://docs.yaoxiang.dev/types"
      - "https://docs.yaoxiang.dev/errors/E0001"
```

## 4. 核心组件设计

### 4.1 Diagnostic 结构

```rust
// src/util/diagnostic.rs

/// 诊断信息的严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,      // 错误 - 阻止编译/运行
    Warning,    // 警告 - 可能有问题
    Info,       // 信息 - 有用的提示
    Hint,       // 建议 - 修复建议
}

/// 错误代码信息
#[derive(Debug, Clone)]
pub struct ErrorCode {
    pub code: String,
    pub category: String,
    pub severity: Severity,
    pub message: String,
    pub explanation: String,
    pub suggestions: Vec<String>,
    pub examples: Vec<Example>,
    pub links: Vec<String>,
}

/// 代码片段显示
#[derive(Debug, Clone)]
pub struct CodeSnippet {
    pub file: String,
    pub span: Span,
    pub code: String,
    pub highlights: Vec<Highlight>,
    pub labels: Vec<Label>,
}

/// 高亮区域
#[derive(Debug, Clone)]
pub struct Highlight {
    pub span: Span,
    pub style: HighlightStyle,
}

/// 标签（用于错误说明）
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle,
}

/// 标签样式
#[derive(Debug, Clone, Copy)]
pub enum LabelStyle {
    Primary,      // 主要错误位置
    Secondary,    // 次要位置
    Note,         // 注释
    Help,         // 帮助信息
}

/// 完整的诊断信息
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub snippets: Vec<CodeSnippet>,
    pub related: Vec<Diagnostic>,
    pub suggestions: Vec<String>,
    pub explanation: Option<String>,
    pub backtrace: Option<Backtrace>,
}

/// 运行时栈帧信息
#[derive(Debug, Clone)]
pub struct Frame {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub function: String,
    pub span: Span,
}

/// 栈跟踪信息
#[derive(Debug, Clone)]
pub struct Backtrace {
    pub frames: Vec<Frame>,
    pub panic_message: String,
    pub thread_name: String,
}
```

### 4.2 错误格式化器

```rust
// src/util/diagnostic.rs

/// 错误格式化器
pub struct DiagnosticFormatter {
    /// 是否启用颜色
    pub use_colors: bool,
    /// 是否显示完整栈跟踪
    pub full_backtrace: bool,
    /// 是否显示帮助链接
    pub show_help: bool,
    /// 是否显示代码片段
    pub show_snippets: bool,
    /// 终端宽度
    pub terminal_width: usize,
}

impl DiagnosticFormatter {
    /// 格式化诊断信息为字符串
    pub fn format(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();
        
        // 1. 错误头
        output.push_str(&self.format_header(diagnostic));
        
        // 2. 代码片段
        if self.show_snippets {
            output.push_str(&self.format_snippets(diagnostic));
        }
        
        // 3. 帮助信息
        if self.show_help {
            output.push_str(&self.format_help(diagnostic));
        }
        
        // 4. 相关错误
        if !diagnostic.related.is_empty() {
            output.push_str(&self.format_related(diagnostic));
        }
        
        output
    }

    /// 格式化错误头
    fn format_header(&self, diagnostic: &Diagnostic) -> String {
        let severity_label = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "help",
        };
        
        format!("{}{}[{}]: {}\n", 
            self.color(severity_label, diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        )
    }

    /// 格式化代码片段
    fn format_snippets(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();
        
        for snippet in &diagnostic.snippets {
            output.push_str(&format!("  --> {}:{}:{}\n",
                snippet.file,
                snippet.span.start.line,
                snippet.span.start.column));
            
            // 代码行
            let lines: Vec<&str> = snippet.code.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let line_num = snippet.span.start.line + i;
                output.push_str(&format!("{:>4} │ {}\n", line_num, line));
            }
            
            // 高亮和标签
            output.push_str(&self.format_labels(snippet));
        }
        
        output
    }

    /// 格式化标签
    fn format_labels(&self, snippet: &CodeSnippet) -> String {
        let mut output = String::new();
        
        for label in &snippet.labels {
            output.push_str(&format!("  {}", label.message));
        }
        
        output
    }

    /// 格式化栈跟踪
    fn format_backtrace(&self, backtrace: &Backtrace) -> String {
        let mut output = String::new();
        
        output.push_str("Stack trace:\n");
        
        for (i, frame) in backtrace.frames.iter().enumerate() {
            let indicator = if i == 0 { "→" } else { " " };
            output.push_str(&format!("{:>3}{} {}:{} in {}\n",
                i + 1,
                indicator,
                frame.file,
                frame.line,
                frame.function));
        }
        
        output
    }
}
```

### 4.3 颜色主题

```rust
// src/util/diagnostic.rs

/// 终端颜色
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    Reset,
}

/// ANSI 颜色代码
impl Color {
    pub fn to_ansi(self) -> &'static str {
        match self {
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
            Color::BrightBlack => "\x1b[90m",
            Color::Reset => "\x1b[0m",
        }
    }
}

/// 格式化器颜色配置
impl DiagnosticFormatter {
    fn color(&self, role: &str, severity: Severity) -> String {
        if !self.use_colors {
            return String::new();
        }
        
        let color = match (role, severity) {
            ("error", _) => Color::Red,
            ("warning", _) => Color::Yellow,
            ("info", _) => Color::Blue,
            ("help", _) => Color::Cyan,
            ("note", _) => Color::Magenta,
            _ => Color::Reset,
        };
        
        color.to_ansi().to_string()
    }
}
```

## 5. 错误类型定义

### 5.1 语法错误

```rust
// src/frontend/parser/errors.rs

#[derive(Debug, Error, Clone)]
pub enum SyntaxError {
    #[error("Unexpected token: expected {expected:?}, found {found:?}")]
    UnexpectedToken {
        expected: Vec<TokenKind>,
        found: TokenKind,
        span: Span,
    },
    
    #[error("Unclosed delimiter: {delimiter}")]
    UnclosedDelimiter {
        delimiter: String,
        span: Span,
        open_span: Span,
    },
    
    #[error("Invalid escape sequence: {sequence}")]
    InvalidEscapeSequence {
        sequence: String,
        span: Span,
    },
    
    #[error("Invalid number literal: {literal}")]
    InvalidNumberLiteral {
        literal: String,
        span: Span,
    },
    
    #[error("Unexpected end of file")]
    UnexpectedEOF {
        span: Span,
        expected: Vec<TokenKind>,
    },
}
```

### 5.2 类型错误

```rust
// src/frontend/typecheck/errors.rs

#[derive(Debug, Error, Clone)]
pub enum TypeError {
    #[error("Type mismatch: expected {expected:?}, found {found:?}")]
    TypeMismatch {
        expected: MonoType,
        found: MonoType,
        span: Span,
        expected_span: Option<Span>,
    },
    
    #[error("Unknown variable: {name}")]
    UnknownVariable {
        name: String,
        span: Span,
        candidates: Vec<String>, // 相似变量名建议
    },
    
    #[error("Unknown type: {name}")]
    UnknownType {
        name: String,
        span: Span,
        candidates: Vec<String>, // 相似类型名建议
    },
    
    #[error("Field not found: {field} in {ty}")]
    FieldNotFound {
        field: String,
        ty: MonoType,
        span: Span,
        available_fields: Vec<String>,
    },
    
    #[error("Method not found: {method} on {ty}")]
    MethodNotFound {
        method: String,
        ty: MonoType,
        span: Span,
        available_methods: Vec<String>,
    },
}
```

### 5.3 运行时错误

```rust
// src/middle/errors.rs

#[derive(Debug, Error, Clone)]
pub enum RuntimeError {
    #[error("Index out of bounds: index {index} is out of bounds for {size}")]
    IndexOutOfBounds {
        index: i128,
        size: usize,
        span: Span,
    },
    
    #[error("Division by zero")]
    DivisionByZero {
        span: Span,
    },
    
    #[error("Null pointer dereference")]
    NullPointerDereference {
        span: Span,
        accessed_field: Option<String>,
    },
    
    #[error("Assertion failed: {condition}")]
    AssertionFailed {
        condition: String,
        span: Span,
    },
    
    #[error("Stack overflow: maximum call depth exceeded")]
    StackOverflow {
        depth: usize,
        max_depth: usize,
        span: Span,
    },
    
    #[error("Recursion detected: function {function} calls itself")]
    RecursionDetected {
        function: String,
        span: Span,
        call_chain: Vec<String>,
    },
}
```

## 6. 增强功能

### 6.1 智能建议系统

```rust
/// 相似名称建议器
pub struct SuggestionEngine {
    /// 编辑距离阈值
    max_distance: usize,
}

impl SuggestionEngine {
    /// 查找相似的变量/类型名
    pub fn suggest_similar(
        &self,
        name: &str,
        candidates: &[String],
    ) -> Vec<String> {
        candidates
            .iter()
            .filter(|c| {
                let distance = levenshtein_distance(name, c);
                distance > 0 && distance <= self.max_distance
            })
            .cloned()
            .collect()
    }
    
    /// 建议正确的关键字
    pub fn suggest_keyword(&self, name: &str) -> Option<String> {
        let keywords = ["let", "fn", "if", "else", "for", "while", "return", "struct"];
        self.suggest_similar(name, &keywords.iter().map(|s| s.to_string()).collect())
            .first()
            .cloned()
    }
}
```

### 6.2 错误代码文档生成

```rust
/// 错误代码文档生成器
pub struct ErrorDocGenerator {
    /// 错误定义
    errors: HashMap<String, ErrorCode>,
}

impl ErrorDocGenerator {
    /// 生成 Markdown 文档
    pub fn generate_markdown(&self) -> String {
        let mut doc = String::new();
        
        doc.push_str("# Error Reference\n\n");
        doc.push_str("This document lists all error codes and their meanings.\n\n");
        
        for (code, error) in &self.errors {
            doc.push_str(&format!("## {}\n\n", code));
            doc.push_str(&format!("**Category**: {}\n\n", error.category));
            doc.push_str(&format!("**Severity**: {:?}\n\n", error.severity));
            doc.push_str(&format!("{}\n\n", error.explanation));
            
            if !error.examples.is_empty() {
                doc.push_str("### Examples\n\n");
                for example in &error.examples {
                    doc.push_str("```yaoxiang\n");
                    doc.push_str(&example.code);
                    doc.push_str("\n```\n\n");
                    doc.push_str("```\n");
                    doc.push_str(&example.output);
                    doc.push_str("\n```\n\n");
                }
            }
            
            if !error.fixes.is_empty() {
                doc.push_str("### Possible Fixes\n\n");
                for fix in &error.fixes {
                    doc.push_str(&format!("- {}\n", fix));
                }
                doc.push_str("\n");
            }
        }
        
        doc
    }
}
```

### 6.3 交互式错误修复

```rust
/// 交互式修复建议
pub struct FixApplicator {
    /// 文件路径
    path: PathBuf,
    /// 源代码
    source: String,
}

impl FixApplicator {
    /// 应用修复建议
    pub fn apply_fix(&mut self, fix: &Fix) -> Result<(), Error> {
        match fix {
            Fix::AddTypeAnnotation { span, type_str } => {
                self.insert_at(span.end, format!(": {}", type_str))
            }
            Fix::ChangeType { span, new_type } => {
                self.replace_span(span, new_type.clone())
            }
            Fix::Rename { old_name, new_name } => {
                self.rename_variable(old_name, new_name)
            }
        }
    }
    
    /// 建议可能的修复
    pub fn suggest_fixes(&self, error: &Diagnostic) -> Vec<Fix> {
        let mut fixes = Vec::new();
        
        for suggestion in &error.suggestions {
            // 解析建议并生成 Fix 对象
            if let Some(fix) = self.parse_suggestion(suggestion) {
                fixes.push(fix);
            }
        }
        
        fixes
    }
}
```

## 7. 命令行集成

### 7.1 编译时错误输出

```rust
// src/main.rs

fn main() {
    let matches = clap::Command::new("yaoxiang")
        .arg(clap::Arg::new("file").required(true))
        .arg(clap::Arg::new("verbose").short('v').long("verbose"))
        .arg(clap::Arg::new("color").short('C').long("color").value_parser(["auto", "always", "never"]))
        .get_matches();
    
    // 加载源文件
    let source = std::fs::read_to_string(matches.get_one::<String>("file")).unwrap();
    
    // 编译
    let result = compile(&source);
    
    match result {
        Ok(_) => println!("Compilation successful!"),
        Err(errors) => {
            let formatter = DiagnosticFormatter {
                use_colors: should_use_color(&matches),
                show_snippets: true,
                show_help: true,
                ..Default::default()
            };
            
            for error in errors {
                println!("{}", formatter.format(&error));
            }
            
            std::process::exit(1);
        }
    }
}
```

### 7.2 运行时错误输出

```rust
// src/middle/executor.rs

impl Executor {
    /// 执行并处理错误
    pub fn run(&mut self, bytecode: &[Bytecode]) -> Result<Value, VMError> {
        // 设置 panic hook 以捕获运行时错误
        std::panic::set_hook(Box::new(|info| {
            let backtrace = Backtrace::capture();
            let diagnostic = Self::create_runtime_diagnostic(info, &backtrace);
            
            let formatter = DiagnosticFormatter {
                use_colors: true,
                show_snippets: true,
                show_help: true,
                ..Default::default()
            };
            
            eprintln!("{}", formatter.format(&diagnostic));
        }));
        
        // 执行字节码...
    }
    
    /// 创建运行时错误诊断
    fn create_runtime_diagnostic(panic_info: &PanicInfo, backtrace: &Backtrace) -> Diagnostic {
        // 从 panic 信息和栈跟踪构建诊断
    }
}
```

## 8. 集成开发环境支持

### 8.1 LSP 诊断协议

```rust
//  Language Server Protocol 诊断输出

pub struct LSPServer;

impl LSPServer {
    /// 发布诊断信息
    fn publish_diagnostics(&self, uri: lsp::Url, diagnostics: Vec<Diagnostic>) {
        let lsp_diagnostics: Vec<lsp::Diagnostic> = diagnostics
            .iter()
            .map(|d| self.to_lsp_diagnostic(d))
            .collect();
        
        self.client.publish_diagnostics(uri, lsp_diagnostics, None);
    }
    
    /// 转换为 LSP 诊断
    fn to_lsp_diagnostic(&self, diagnostic: &Diagnostic) -> lsp::Diagnostic {
        lsp::Diagnostic {
            range: self.span_to_range(diagnostic.span),
            severity: Some(self.severity_to_lsp(diagnostic.severity)),
            code: Some(lsp::NumberOrString::String(diagnostic.code.clone())),
            message: diagnostic.message.clone(),
            related_information: diagnostic.related.iter()
                .map(|r| self.to_related_information(r))
                .collect(),
            ..Default::default()
        }
    }
}
```

### 8.2 错误代码快速修复

```json
// LSP Code Action 响应示例
{
  "version": 3,
  "commands": [
    {
      "title": "Add type annotation",
      "command": "yaoxiang.applyFix",
      "arguments": [
        {
          "file": "examples/test.yx",
          "range": {
            "start": { "line": 3, "character": 12 },
            "end": { "line": 3, "character": 12 }
          },
          "fix": {
            "kind": "insert",
            "text": ": Int"
          }
        }
      ]
    }
  ]
}
```

## 9. 配置文件

### 9.1 诊断配置

```yaml
# .yaoxiang/config.yaml 或 pyproject.toml 中的 yaoxiang 配置

diagnostic:
  # 是否显示颜色
  colors: "auto"
  
  # 是否显示代码片段
  show_snippets: true
  
  # 是否显示帮助链接
  show_help: true
  
  # 是否启用智能建议
  suggestions: true
  
  # 最大相似建议数量
  max_suggestions: 5
  
  # 错误代码文档路径
  error_docs_url: "https://docs.yaoxiang.dev/errors"
```

## 10. 实施计划

### 阶段一：基础架构（Week 1-2）

- [ ] 完善 `Diagnostic` 结构定义
- [ ] 实现 `CodeSnippet` 和 `Label` 显示
- [ ] 添加 `DiagnosticFormatter` 格式化器
- [ ] 实现基础颜色支持

### 阶段二：错误类型增强（Week 3-4）

- [ ] 为所有错误类型添加 span 和建议
- [ ] 实现智能建议引擎（相似名称）
- [ ] 添加错误代码文档生成器
- [ ] 增强运行时栈跟踪

### 阶段三：用户体验优化（Week 5-6）

- [ ] 添加交互式修复建议
- [ ] 实现 LSP 诊断协议支持
- [ ] 添加配置文件支持
- [ ] 创建错误参考文档网站

### 阶段四：测试和完善（Week 7-8）

- [ ] 编写单元测试
- [ ] 创建错误提示测试用例
- [ ] 收集用户反馈并优化
- [ ] 完善错误代码文档

## 11. 附录

### 11.1 完整错误代码列表

| 代码 | 名称 | 类别 | 严重程度 |
|------|------|------|----------|
| E0001 | TypeMismatch | 类型错误 | error |
| E0002 | UnknownVariable | 名称解析 | error |
| E0003 | UnknownType | 名称解析 | error |
| E0004 | ArityMismatch | 类型错误 | error |
| E0005 | RecursiveType | 类型错误 | error |
| E0006 | UnsupportedOp | 类型错误 | error |
| E0007 | GenericConstraint | 类型错误 | error |
| E0008 | InfiniteType | 类型错误 | error |
| E0009 | UnboundTypeVar | 类型错误 | error |
| E0010 | UnknownLabel | 名称解析 | error |
| E0011 | FieldNotFound | 类型错误 | error |
| E0012 | IndexOutOfBounds | 运行时错误 | error |
| E0013 | CallError | 类型错误 | error |
| E0014 | AssignmentError | 类型错误 | error |
| E0015 | InferenceError | 类型错误 | error |
| E0016 | CannotInferParamType | 类型错误 | error |

### 11.2 相关资源

- [Rust 编译器错误提示](https://doc.rust-lang.org/error_codes/error-index.html)
- [Clang 诊断格式](https://clang.llvm.org/diagnostics.html)
- [GCC 错误消息格式](https://gcc.gnu.org/onlinedocs/gcc/Diagnostic-Message-Formatting-Options.html)
- [Language Server Protocol - Diagnostic](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#diagnostic)

---

## 12. 详细错误示例集

本节提供 YaoXiang 各种错误的详细示例，包括源代码、错误输出和修复建议。

### 12.1 语法错误示例

#### 示例 1: 意外的令牌

**源代码：**
```yaoxiang
// examples/syntax/unexpected_token.yx
fn main() {
    let x = 10;
    let y = 20;
    x + y = 30;  // 错误：不能给表达式赋值
}
```

**错误输出：**
```
error[E0101]: Unexpected token
  --> examples/syntax/unexpected_token.yx:5:5
   |
 5 |     x + y = 30;
   |     ^^^^^^^
   |
   = help: The left-hand side of an assignment must be a variable or field
   = note: You cannot assign to an expression like `x + y`
   = note: Did you mean to use `==` for comparison?
   = see https://docs.yaoxiang.dev/errors/E0101 for more information
```

**修复建议：**
```yaoxiang
fn main() {
    let x = 10;
    let y = 20;
    // 如果要比较：
    let result = x + y;
    // 或者：
    // if x + y == 30 { ... }
}
```

---

#### 示例 2: 未闭合的括号

**源代码：**
```yaoxiang
// examples/syntax/unclosed_paren.yx
fn calculate(a: Int, b: Int) -> Int {
    return (a + b * 2;
}

fn main() {
    let x = calculate(10, 20);
}
```

**错误输出：**
```
error[E0102]: Unclosed delimiter
  --> examples/syntax/unclosed_paren.yx:2:12
   |
 2 |     return (a + b * 2;
   |            ^ unclosed '(' here
   |
 3 | }
   | ^ expected ')' to match this
   |
   = help: The closing parenthesis ')' is missing
   = note: The function body has unbalanced parentheses
   = see https://docs.yaoxiang.dev/errors/E0102 for more information
```

**修复建议：**
```yaoxiang
fn calculate(a: Int, b: Int) -> Int {
    return (a + b) * 2;  // 添加闭合括号
}
```

---

#### 示例 3: 无效的转义序列

**源代码：**
```yaoxiang
// examples/syntax/invalid_escape.yx
fn main() {
    let path = "C:\\Users\\Name\\Documents\test";  // 无效的 \t
    print(path);
}
```

**错误输出：**
```
error[E0103]: Invalid escape sequence
  --> examples/syntax/invalid_escape.yx:3:24
   |
 3 |     let path = "C:\\Users\\Name\\Documents\test";
   |                        ^^^^^^^^
   |                        help: Invalid escape sequence '\t'
   |
   = note: Valid escape sequences are: \\n, \\t, \\r, \\, \", '\\'
   = note: To represent a literal backslash, use '\\\\'
   = help: Did you mean to use a tab character or two backslashes?
   = see https://docs.yaoxiang.dev/errors/E0103 for more information
```

**修复建议：**
```yaoxiang
fn main() {
    let path = "C:\\Users\\Name\\Documents\\test";  // 使用双反斜杠
    print(path);
}
```

---

### 12.2 类型错误示例

#### 示例 1: 类型不匹配

**源代码：**
```yaoxiang
// examples/type/mismatch.yx
struct Point {
    x: Int,
    y: Int,
}

fn main() {
    let p: Point = Point { x: 10, y: 20 };
    let s: String = p;  // 错误：Point 不能转换为 String
    print(s);
}
```

**错误输出：**
```
error[E0001]: Type mismatch
  --> examples/type/mismatch.yx:10:15
   |
 10 |     let s: String = p;
   |               ^^^^^^^
   |               expected `String`, found `Point`
   |
   = help: Cannot convert `Point` to `String` automatically
   = note: Struct types cannot be implicitly converted to strings
   = help: Consider using string interpolation or a to_string() method
   = example: `let s = "Point(" + p.x.to_string() + ", " + p.y.to_string() + ")"`
   = see https://docs.yaoxiang.dev/errors/E0001 for more information
```

**修复建议：**
```yaoxiang
fn main() {
    let p: Point = Point { x: 10, y: 20 };
    let s = "Point(" + p.x.to_string() + ", " + p.y.to_string() + ")";
    print(s);
}
```

---

#### 示例 2: 参数数量不匹配

**源代码：**
```yaoxiang
// examples/type/arity.yx
fn add(a: Int, b: Int) -> Int {
    return a + b;
}

fn main() {
    let result = add(10, 20, 30);  // 错误：参数过多
}
```

**错误输出：**
```
error[E0004]: Arity mismatch
  --> examples/type/arity.yx:7:20
   |
 7 |     let result = add(10, 20, 30);
   |                    ^^^^^^^^^^^^
   |                    expected 2 arguments, found 3
   |
   = help: Function `add` takes 2 parameters
   = note: Definition: `fn add(a: Int, b: Int) -> Int`
   = note: You can define a three-parameter version: `fn add3(a, b, c) => a + b + c`
   = see https://docs.yaoxiang.dev/errors/E0004 for more information
```

**修复建议：**
```yaoxiang
fn main() {
    let result = add(10, 20);  // 移除多余的参数
    // 或者定义一个新函数
    // fn add3(a: Int, b: Int, c: Int) -> Int { a + b + c }
}
```

---

#### 示例 3: 未知字段

**源代码：**
```yaoxiang
// examples/type/unknown_field.yx
struct Rectangle {
    width: Int,
    height: Int,
}

fn main() {
    let rect = Rectangle { width: 10, height: 20 };
    let area = rect.width * rect.area;  // 错误：Rectangle 没有 area 字段
}
```

**错误输出：**
```
error[E0011]: Unknown field `area` in `Rectangle`
  --> examples/type/unknown_field.yx:9:31
   |
 9 |     let area = rect.width * rect.area;
   |                               ^^^^^
   |                               help: Field `area` not found
   |
   = note: Available fields for `Rectangle` are: `width`, `height`
   = help: Did you mean to compute the area?
   = example: `rect.width * rect.height`
   = see https://docs.yaoxiang.dev/errors/E0011 for more information
```

**修复建议：**
```yaoxiang
fn main() {
    let rect = Rectangle { width: 10, height: 20 };
    let area = rect.width * rect.height;  // 使用正确的字段
}
```

---

### 12.3 名称解析错误示例

#### 示例 1: 未知变量

**源代码：**
```yaoxiang
// examples/scope/unknown_variable.yx
fn main() {
    let result = calculate_sum(10, 20);
    print(result);
}
```

**错误输出：**
```
error[E0002]: Unknown variable `calculate_sum`
  --> examples/scope/unknown_variable.yx:3:22
   |
 3 |     let result = calculate_sum(10, 20);
   |                      ^^^^^^^^^^^^
   |                      not found in this scope
   |
   = help: No function named `calculate_sum` is defined
   = note: Similar functions in scope: none
   = help: Did you mean to define this function first?
   = see https://docs.yaoxiang.dev/errors/E0002 for more information

warning[W0001]: Unused variable `result`
  --> examples/scope/unknown_variable.yx:2:13
   |
 2 |     let result = ...;
   |              ^^^^^^
```

**修复建议：**
```yaoxiang
fn calculate_sum(a: Int, b: Int) -> Int {
    return a + b;
}

fn main() {
    let result = calculate_sum(10, 20);
    print(result);
}
```

---

#### 示例 2: 变量名拼写错误

**源代码：**
```yaoxiang
// examples/scope/typo.yx
fn main() {
    let user_name = "Alice";
    print(userage);  // 错误：userage 未定义（应为 user_name）
}
```

**错误输出：**
```
error[E0002]: Unknown variable `userage`
  --> examples/scope/typo.yx:4:11
   |
 4 |     print(userage);
   |           ^^^^^^^
   |           not found in this scope
   |
   = help: Similar variables in scope: `user_name`
   = note: Did you mean `user_name`? (edit distance: 1)
   = help: Press Ctrl+. to auto-fix this typo
   = see https://docs.yaoxiang.dev/errors/E0002 for more information
```

**修复建议：**
```yaoxiang
fn main() {
    let user_name = "Alice";
    print(user_name);  // 修正拼写
}
```

---

### 12.4 运行时错误示例

#### 示例 1: 索引越界

**源代码：**
```yaoxiang
// examples/runtime/index_bounds.yx
fn main() {
    let numbers = [1, 2, 3, 4, 5];
    print(numbers[10]);  // 运行时错误：索引 10 超出数组大小 5
}
```

**运行时错误输出：**
```
thread 'main' panicked at 'index out of bounds':

RuntimeError: Index out of bounds (index: 10, size: 5)
  --> examples/runtime/index_bounds.yx:4:14
   |
 4 |     print(numbers[10]);
   |              ^^^^ index 10 is out of bounds for array of size 5
   |
   = note: Array `numbers` has 5 elements (indices 0-4)
   = help: Valid indices are: 0, 1, 2, 3, 4
   = help: Consider using bounds checking: `if index < numbers.len() { ... }`
   = see https://docs.yaoxiang.dev/errors/R4001 for more information

Stack trace:
  1 → examples/runtime/index_bounds.yx:2:1  in fn main()
```

**修复建议：**
```yaoxiang
fn main() {
    let numbers = [1, 2, 3, 4, 5];
    let index = 2;  // 使用有效索引
    if index < numbers.len() {
        print(numbers[index]);
    } else {
        print("Index out of bounds");
    }
}
```

---

#### 示例 2: 除零错误

**源代码：**
```yaoxiang
// examples/runtime/divide_zero.yx
fn main() {
    let x = 10;
    let y = 0;
    let result = x / y;  // 运行时错误：除以零
    print(result);
}
```

**运行时错误输出：**
```
thread 'main' panicked at 'division by zero':

RuntimeError: Division by zero
  --> examples/runtime/divide_zero.yx:5:18
   |
 5 |     let result = x / y;
   |                  ^^^^ division by zero
   |
   = note: Variable `y` is 0 at this point
   = help: Add a check before division: `if y != 0 { x / y } else { ... }`
   = see https://docs.yaoxiang.dev/errors/R4002 for more information

Stack trace:
  1 → examples/runtime/divide_zero.yx:2:1  in fn main()
```

**修复建议：**
```yaoxiang
fn main() {
    let x = 10;
    let y = 0;
    if y == 0 {
        print("Error: Cannot divide by zero");
    } else {
        let result = x / y;
        print(result);
    }
}
```

---

#### 示例 3: 递归深度溢出

**源代码：**
```yaoxiang
// examples/runtime/recursion.yx
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() {
    let result = fibonacci(1000);  // 递归深度过大
}
```

**运行时错误输出：**
```
thread 'main' panicked at 'maximum call depth exceeded':

RuntimeError: Stack overflow (depth: 1024, max_depth: 1024)
  --> examples/runtime/recursion.yx:3:12
   |
 3 |     return fibonacci(n - 1) + fibonacci(n - 2);
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |              recursive call here
   |
   = note: Maximum recursion depth is 1024
   = help: The function `fibonacci` is called recursively 1024 times
   = help: Consider using an iterative approach instead
   = see https://docs.yaoxiang.dev/errors/R4005 for more information

Stack trace (first 10 frames):
  1 → examples/runtime/recursion.yx:9:1   in fn main()
  2 → examples/runtime/recursion.yx:3:1   in fn fibonacci()
  3 → examples/runtime/recursion.yx:3:1   in fn fibonacci()
  ...
 1024 → examples/runtime/recursion.yx:2:1   in fn fibonacci()
```

**修复建议：**
```yaoxiang
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n;
    }
    let a = 0;
    let b = 1;
    let i = 2;
    while i <= n {
        let temp = a + b;
        a = b;
        b = temp;
        i = i + 1;
    }
    return b;
}

fn main() {
    let result = fibonacci(1000);
    print(result);
}
```

---

### 12.5 泛型错误示例

#### 示例 1: 泛型约束违反

**源代码：**
```yaoxiang
// examples/generic/constraint.yx
struct Container<T: Addable> {
    value: T,
}

fn main() {
    let c = Container { value: "hello" };
}

// Addable 是用于数值类型的特征
```

**错误输出：**
```
error[E0007]: Generic constraint violated
  --> examples/generic/constraint.yx:7:26
   |
 7 |     let c = Container { value: "hello" };
   |                          ^^^^^^^^^^^^^^
   |                          type `String` does not satisfy `Addable`
   |
   = note: Generic parameter `T` has constraint `Addable`
   = help: `String` does not implement the `Addable` trait
   = note: Types that implement `Addable`: `Int`, `Float`, `Double`
   = see https://docs.yaoxiang.dev/errors/E0007 for more information
```

**修复建议：**
```yaoxiang
fn main() {
    let c = Container { value: 42 };  // 使用 Int 类型
    // 或者重新定义 Container 不需要 Addable 约束
}
```

---

### 12.6 警告示例

#### 示例 1: 未使用的变量

**源代码：**
```yaoxiang
// examples/warning/unused_var.yx
fn main() {
    let unused_variable = 100;
    let used_variable = 200;
    print(used_variable);
}
```

**警告输出：**
```
warning[W0001]: Unused variable `unused_variable`
  --> examples/warning/unused_var.yx:3:9
   |
 3 |     let unused_variable = 100;
   |     ^^^^^^^^^^^^^^^^^^^
   |
   = note: Variables starting with `_` are allowed to be unused
   = help: Did you mean to use it? Prefix with `_` to silence this warning
   = example: `let _unused_variable = 100;`
```

**修复建议：**
```yaoxiang
fn main() {
    let _unused_variable = 100;  // 使用下划线前缀
    let used_variable = 200;
    print(used_variable);
}
```

---

### 12.7 复杂多错误示例

**源代码：**
```yaoxiang
// examples/complex/multiple_errors.yx
fn procces_data(data: List<Int>) -> Int {  // 拼写错误: procces -> process
    let total = 0;
    for item in data {
        total = total + item;
    }
    return total;
}

fn main() {
    let numbers = [1, 2, 3, "four", 5];  // 类型混合
    let result = procces_data(numbers);   // 函数名拼写错误
    print(ressult);                        // 变量名拼写错误
}
```

**完整输出：**
```
error[E0002]: Unknown function `procces_data`
  --> examples/complex/multiple_errors.yx:11:16
   |
 11 |     let result = procces_data(numbers);
   |                  ^^^^^^^^^^^^
   |                  not found in this scope
   |
   = help: Similar function defined at line 2: `procces_data`
   = note: Did you mean `procces_data`? (typo in function name)
   = help: Press Ctrl+. to rename function to `process_data`

error[E0001]: Type mismatch
  --> examples/complex/multiple_errors.yx:10:24
   |
 10 |     let numbers = [1, 2, 3, "four", 5];
   |                        ^^^^^^^
   |                        expected `Int`, found `String`
   |
   = note: Array elements must all have the same type
   = help: Remove the string or convert it to Int
   = example: `["four".to_int()]`

error[E0002]: Unknown variable `ressult`
  --> examples/complex/multiple_errors.yx:12:11
   |
 12 |     print(ressult);
   |           ^^^^^^^
   |           not found in this scope
   |
   = help: Similar variable defined at line 10: `result`
   = note: Did you mean `result`?

2 errors found.

Compilation failed.
```

**修复后的代码：**
```yaoxiang
fn process_data(data: List<Int>) -> Int {
    let total = 0;
    for item in data {
        total = total + item;
    }
    return total;
}

fn main() {
    let numbers = [1, 2, 3, 4, 5];
    let result = process_data(numbers);
    print(result);
}
```

---

### 12.8 完整项目错误示例

**项目结构：**
```
my_project/
├── main.yx
├── utils.yx
└── math.yx
```

**main.yx:**
```yaoxiang
import "./utils";
import "./math";

fn main() {
    let result = math.add(10, 20);
    print_utils_message(result);
}
```

**错误输出：**
```
error[E0002]: Unknown import `./utils`
  --> main.yx:2:1
   |
 2 | import "./utils";
   | ^^^^^^^^^^^^^^^
   |           help: File not found: /path/to/my_project/utils.yx
   |
   = note: Import paths are resolved relative to the importing file
   = help: Did you mean `./utils.yx`?
   = help: Available files in directory: `math.yx`

error[E0002]: Unknown import `./math`
  --> main.yx:3:1
   |
 3 | import "./math";
   | ^^^^^^^^^^^^^^
   |           note: File found but no exports
   = note: The file `./math.yx` exists but doesn't export `add`
   = help: Add `export fn add(a, b) => a + b` to math.yx

error[E0002]: Unknown function `print_utils_message`
  --> main.yx:7:5
   |
 7 |     print_utils_message(result);
   |     ^^^^^^^^^^^^^^^^^^^^
   |     not found in this scope
   |
   = note: Imported items from `./utils`:
   =   - `format_output` (function)
   =   - `validate_input` (function)
   = help: Did you mean `format_output`?

3 errors found.

Compilation failed.
```

---

## 13. 测试用例

### 13.1 错误提示测试框架

```rust
// tests/diagnostic.rs

#[cfg(test)]
mod error_formatting_tests {
    use crate::util::diagnostic::{DiagnosticFormatter, Diagnostic, Severity};
    use crate::util::span::{Span, Position};
    
    #[test]
    fn test_type_mismatch_format() {
        let source = r#"let x: Int = "hello";"#;
        
        let diagnostic = Diagnostic::error(
            "E0001".to_string(),
            "Type mismatch: expected `Int`, found `String`".to_string(),
            Span::new(
                Position::new(1, 9),
                Position::new(1, 20)
            ),
        );
        
        let formatter = DiagnosticFormatter::default();
        let output = formatter.format(&diagnostic);
        
        assert!(output.contains("error[E0001]"));
        assert!(output.contains("Type mismatch"));
        assert!(output.contains("let x"));
    }
    
    #[test]
    fn test_unknown_variable_with_suggestion() {
        let diagnostic = Diagnostic::error(
            "E0002".to_string(),
            "Unknown variable `userage`".to_string(),
            Span::dummy(),
        );
        
        let suggestions = vec!["user_name".to_string()];
        
        // 验证建议是否包含在输出中
        let formatter = DiagnosticFormatter::default();
        let output = formatter.format(&diagnostic);
        
        assert!(output.contains("user_name"));
    }
}
```

## 15. 类型推断规则与错误提示

本节详细说明 YaoXiang 的类型推断规则，以及如何为每种推断场景生成友好的错误提示。

### 15.1 类型推断分类总览

#### 完整标注场景（PASS）

| #   | 语法示例                                    | 参数     | 返回   | 解析 | 类型检查 | 说明           |
|-----|---------------------------------------------|----------|--------|------|----------|----------------|
| 1   | `add: (Int, Int) -> Int = (a, b) => a + b`  | ✓ Int    | ✓ Int  | ✓    | ✓ PASS   | 标准完整标注   |
| 2   | `inc: Int -> Int = x => x + 1`              | ✓ Int    | ✓ Int  | ✓    | ✓ PASS   | 单参数完整标注 |
| 3   | `log: (String) -> Void = (msg) => print(msg)` | ✓ String | ✓ Void | ✓    | ✓ PASS   | Void 返回      |
| 4   | `get_val: () -> Int = () => 42`             | ✓ ()     | ✓ Int  | ✓    | ✓ PASS   | 无参完整标注   |
| 5   | `empty: () -> Void = () => {}`              | ✓ ()     | ✓ Void | ✓    | ✓ PASS   | 空函数体       |

#### 新推断场景（无类型标注）

| #   | 语法示例                        | 参数     | 返回      | 解析 | 类型检查 | 说明          |
|-----|---------------------------------|----------|-----------|------|----------|---------------|
| 6   | `main = () => {}`               | 推断 ()  | 推断 Void | ✓    | ✓ PASS   | 空块 → Void   |
| 7   | `get_num = () => 42`            | 推断 ()  | 推断 Int  | ✓    | ✓ PASS   | 表达式 → 类型 |
| 8   | `add = (a, b) => a + b`         | 无法推断 | 推断 Int  | ✓    | ✗ 拒绝   | 参数无类型    |
| 9   | `square(x) = x * x`             | 无法推断 | 推断 Int  | ✓    | ✗ 拒绝   | 参数无类型    |
| 10  | `foo = x => x`                  | 无法推断 | 无法推断  | ✓    | ✗ 拒绝   | 全无法推断    |
| 11  | `print_msg = (msg) => print(msg)` | 无法推断 | 推断 Void | ✓    | ✗ 拒绝   | 参数无类型    |

#### 旧语法推断场景

| #   | 语法示例                        | 参数       | 返回      | 解析 | 类型检查 | 说明             |
|-----|---------------------------------|------------|-----------|------|----------|------------------|
| 12  | `empty3() = () => {}`           | 推断 ()    | 推断 Void | ✓    | ✓ PASS   | 旧语法空函数     |
| 13  | `get_random() = () => 42`       | 推断 ()    | 推断 Int  | ✓    | ✓ PASS   | 旧语法有返回值   |
| 14  | `square2(Int) = (x) => x * x`   | ✓ Int      | 推断 Int  | ✓    | ✓ PASS   | 旧语法有参数类型 |
| 15  | `mul(Int, Int) = (a, b) => a * b` | ✓ Int, Int | 推断 Int  | ✓    | ✓ PASS   | 旧语法完整参数   |

#### return 语句场景

| #   | 语法示例                                              | 参数     | 返回     | 解析 | 类型检查 | 说明            |
|-----|-------------------------------------------------------|----------|----------|------|----------|-----------------|
| 16  | `add: (Int, Int) -> Int = (a, b) => { return a + b; }` | ✓        | ✓        | ✓    | ✓ PASS   | 标准 + return   |
| 17  | `add = (a, b) => { return a + b; }`                   | 无法推断 | 无法推断 | ✓    | ✗ 拒绝   | 参数无类型      |
| 18  | `get = () => { return 42; }`                          | 推断 ()  | 推断 Int | ✓    | ✓ PASS   | return 显式返回 |
| 19  | `early: Int -> Int = (x) => { if x < 0 { return 0; } x }` | ✓ | ✓ | ✓ | ✓ PASS | 早期 return |

### 15.2 类型推断规则核心逻辑

```rust
// src/frontend/typecheck/infer.rs

/// 类型推断引擎
pub struct TypeInferenceEngine {
    /// 当前语言设置
    locale: Locale,
    /// 推断上下文
    context: InferenceContext,
}

impl TypeInferenceEngine {
    /// 推断函数类型
    pub fn infer_function_type(&self, func: &Function) -> InferenceResult<Type> {
        // 1. 推断参数类型
        let param_types = self.infer_params(&func.params)?;
        
        // 2. 推断返回类型
        let return_type = self.infer_return_type(&func.return_type, &func.body)?;
        
        // 3. 构建函数类型
        Ok(Type::Function {
            params: param_types,
            return_type: Box::new(return_type),
        })
    }
    
    /// 推断参数类型
    fn infer_params(&self, params: &[Param]) -> InferenceResult<Vec<Type>> {
        params
            .iter()
            .map(|p| {
                if let Some(ty) = &p.type_annotation {
                    // 有类型标注，直接使用
                    Ok(self.resolve_type(ty)?)
                } else {
                    // 无类型标注，尝试从上下文推断
                    self.infer_param_from_context(p)
                }
            })
            .collect()
    }
    
    /// 从上下文推断参数类型
    fn infer_param_from_context(&self, param: &Param) -> InferenceResult<Type> {
        // Lambda 参数无法从上下文推断类型
        // 因为参数类型必须在调用前确定
        Err(TypeError::CannotInferParamType {
            name: param.name.clone(),
            span: param.span,
            reason: InferenceFailureReason::LambdaParamNoContext,
        })
    }
    
    /// 推断返回类型
    fn infer_return_type(&self, annotation: &Option<Type>, body: &Expr) -> InferenceResult<Type> {
        match (annotation, body) {
            // 有返回类型标注
            (Some(ty), _) => Ok(self.resolve_type(ty)?),
            
            // 无标注，有 return expr
            (None, Expr::Return { expr, .. }) => {
                self.infer_type(expr)
            }
            
            // 无标注，有表达式（无 return）
            (None, Expr::Block(block)) if block.has_return() => {
                self.infer_type_from_return_block(block)
            }
            
            // 无标注，无 return，有表达式
            (None, expr) if !is_block(expr) => {
                self.infer_type(expr)
            }
            
            // 无标注，无 return，有空块 {}
            (None, Expr::Block(block)) if block.is_empty() => {
                Ok(Type::Void)
            }
            
            // 无标注，无 return，无表达式 → 无法推断
            (None, Expr::Block(block)) if !block.has_return() => {
                Err(TypeError::CannotInferReturnType {
                    span: block.span,
                    reason: InferenceFailureReason::NoReturnInBlock,
                })
            }
            
            _ => unreachable!(),
        }
    }
}
```

### 15.3 友好错误提示设计

#### 推断失败错误消息模板

```rust
// src/util/diagnostic/messages.rs

/// 多语种错误消息模板
pub struct InferenceErrorMessages;

impl InferenceErrorMessages {
    /// 参数无法推断（Lambda 参数）
    pub fn param_cannot_infer(name: &str, context: &str) -> MessageTemplate {
        MessageTemplate {
            zh: format!("参数 `{}` 无法推断类型：{}", name, context),
            en: format!("Parameter `{}` cannot have its type inferred: {}", name, context),
            ja: format!("パラメータ `{}` の型を推測できません：{}", name, context),
        }
    }
    
    /// 返回类型无法推断
    pub fn return_cannot_infer(span: Span) -> MessageTemplate {
        MessageTemplate {
            zh: "无法推断返回类型：函数体没有 return 语句且不是表达式".to_string(),
            en: "Cannot infer return type: function body has no return statement and is not an expression".to_string(),
            ja: "戻り値の型を推測できません：関数本体に return 文がなく、式でもありません".to_string(),
        }
    }
    
    /// 建议添加类型标注
    pub fn suggest_type_annotation(name: &str, suggested_type: &str) -> MessageTemplate {
        MessageTemplate {
            zh: format!("建议给参数 `{}` 添加类型标注：`{}`", name, suggested_type),
            en: format!("Consider adding a type annotation for parameter `{}`: `{}`", name, suggested_type),
            ja: format!("パラメータ `{}` に型注釈を追加建议你：{}", name, suggested_type),
        }
    }
}

/// 推断失败原因
pub enum InferenceFailureReason {
    /// Lambda 参数无法从上下文推断
    LambdaParamNoContext,
    /// 块中没有 return 语句
    NoReturnInBlock,
    /// 空块无法推断返回类型
    EmptyBlockReturn,
    /// 循环中的 break 无法推断类型
    BreakInLoop,
}

impl InferenceFailureReason {
    /// 获取解释文本
    pub fn explanation(&self) -> MessageTemplate {
        match self {
            Self::LambdaParamNoContext => MessageTemplate {
                zh: "Lambda 参数的类型无法从使用上下文推断，因为参数类型必须在函数定义时确定".to_string(),
                en: "Lambda parameter types cannot be inferred from usage context because parameter types must be known at function definition time".to_string(),
                ja: "Lambda パラメータの型は、使用コンテキストから推測できません。パラメータの型は関数定義時に確定する必要があるためです".to_string(),
            },
            Self::NoReturnInBlock => MessageTemplate {
                zh: "如果函数体是代码块且没有 return 语句，编译器无法确定返回类型".to_string(),
                en: "If the function body is a code block without a return statement, the compiler cannot determine the return type".to_string(),
                ja: "関数本体が return 文のないコードブロックの場合、コンパイラは戻り値の型を決定できません".to_string(),
            },
            _ => MessageTemplate::default(),
        }
    }
}
```

### 15.4 具体场景的错误提示示例

#### 场景 8: 参数无法推断

**源代码：**
```yaoxiang
add = (a, b) => a + b
```

**友好错误输出：**
```
error[E0016]: Cannot infer type for parameter `a`
  --> example.yx:1:10
   |
 1 | add = (a, b) => a + b
   |          ^ parameter `a` has no type annotation
   |
   = note: Lambda parameters cannot be inferred from context
   = help: Add type annotation to both parameters:
   = example: `add: (Int, Int) -> Int = (a: Int, b: Int) => a + b`
   = help: Or use old syntax: `add(Int, Int) = (a, b) => a + b`
   = see https://docs.yaoxiang.dev/errors/E0016 for more information
   = see https://docs.yaoxiang.dev/types#function-types for function type syntax
```

**多语种版本：**
```
[zh] error[E0016]: 无法推断参数 `a` 的类型
[en] error[E0016]: Cannot infer type for parameter `a`
[ja] error[E0016]: パラメータ `a` の型を推測できません
```

---

#### 场景 10: 参数和返回都无法推断

**源代码：**
```yaoxiang
foo = x => x
```

**友好错误输出：**
```
error[E0016]: Cannot infer type for parameter `x` and return type
  --> example.yx:1:8
   |
 1 | foo = x => x
   |        ^ parameter `x` has no type annotation
   |
   = note: Neither parameter types nor return type can be inferred
   = help: This function takes one parameter and returns it unchanged
   = help: Add type annotation:
   = example: `foo: (T) -> T = <T> (x: T) => x`
   = example: `foo: (Int) -> Int = (x: Int) => x`
   = see https://docs.yaoxiang.dev/errors/E0016 for more information
```

---

#### 场景 11: 参数无类型但有上下文

**源代码：**
```yaoxiang
print_msg = (msg) => print(msg)
```

**友好错误输出：**
```
error[E0016]: Cannot infer type for parameter `msg`
  --> example.yx:1:14
   |
 1 | print_msg = (msg) => print(msg)
   |              ^^^
   |
   = note: Although `msg` is used in `print(msg)`, parameter types cannot be inferred from function calls
   = help: The `print` function accepts `String`, but this is a hint, not a guarantee
   = help: Add explicit type annotation:
   = example: `print_msg: (String) -> Void = (msg: String) => print(msg)`
   = see https://docs.yaoxiang.dev/errors/E0016 for more information
```

---

#### 场景 17: 参数无类型的 return 函数

**源代码：**
```yaoxiang
add = (a, b) => { return a + b; }
```

**友好错误输出：**
```
error[E0016]: Cannot infer type for parameters `a` and `b`
  --> example.yx:1:8
   |
 1 | add = (a, b) => { return a + b; }
   |        ^^^^^^
   |
   = note: Parameter types cannot be inferred even with return statement
   = help: Add parameter type annotations:
   = example: `add: (Int, Int) -> Int = (a: Int, b: Int) => { return a + b; }`
   = note: Return type can be inferred from `a + b` (which is `Int`)
   = see https://docs.yaoxiang.dev/errors/E0016 for more information
```

---

#### 块无 return 的情况

**源代码：**
```yaoxiang
get_val = () => { let x = 42; }
```

**友好错误输出：**
```
error[E0017]: Cannot infer return type for function
  --> example.yx:1:12
   |
 1 | get_val = () => { let x = 42; }
   |            ^^^^^^^^^^^^^^^^^^^^
   |
   = note: Function body is a block without return statement
   = help: If you want to return Void, use an empty block `{}`
   = help: If you want to return a value, add return statement:
   = example: `get_val = () => { let x = 42; return x; }`
   = help: Or simplify to expression: `get_val = () => 42`
   = see https://docs.yaoxiang.dev/errors/E0017 for more information
```

## 16. 多语种支持（i18n）模块化设计

### 16.1 模块架构

```
src/util/diagnostic/
├── mod.rs
├── locale.rs           # 语言环境管理
├── messages/           # 消息模板目录
│   ├── mod.rs
│   ├── zh_CN.rs        # 简体中文
│   ├── en_US.rs        # 美国英语
│   └── ja_JP.rs        # 日语
├── templates.rs        # 消息模板定义
└── formatter.rs        # 格式化器
```

### 16.2 语言环境管理

```rust
// src/util/diagnostic/locale.rs

/// 支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    ZhCN,    // 简体中文
    EnUS,    // 美国英语
    EnGB,    // 英国英语
    JaJP,    // 日语
    KoKR,    // 韩语
    EsES,    // 西班牙语
}

/// 语言标签
impl From<&str> for Language {
    fn from(s: &str) -> Self {
        match s {
            "zh-CN" | "zh_CN" | "zh" => Language::ZhCN,
            "en-US" | "en_US" | "en" => Language::EnUS,
            "ja-JP" | "ja_JP" | "ja" => Language::JaJP,
            "ko-KR" | "ko_KR" | "ko" => Language::KoKR,
            "es-ES" | "es_ES" | "es" => Language::EsES,
            _ => Language::EnUS,  // 默认语言
        }
    }
}

/// 语言环境配置
#[derive(Debug, Clone)]
pub struct Locale {
    /// 当前语言
    language: Language,
    /// 是否使用本地化消息
    use_native: bool,
    /// 是否显示英文原文
    show_original: bool,
}

impl Default for Locale {
    fn default() -> Self {
        Locale {
            language: Language::from("zh-CN"),  // 默认中文
            use_native: true,
            show_original: false,
        }
    }
}

impl Locale {
    /// 从环境变量创建
    pub fn from_env() -> Self {
        let lang = std::env::var("YAOXIANG_LANG")
            .unwrap_or_else(|_| "zh-CN".to_string());
        
        Self {
            language: Language::from(&lang),
            ..Default::default()
        }
    }
    
    /// 从命令行参数创建
    pub fn from_args(args: &ArgMatches) -> Self {
        let lang = args.get_one::<String>("lang")
            .map(|s| s.as_str())
            .unwrap_or("zh-CN");
        
        Self {
            language: Language::from(lang),
            ..Default::default()
        }
    }
}
```

### 16.3 消息模板系统

```rust
// src/util/diagnostic/templates.rs

/// 多语种消息模板
#[derive(Debug, Clone)]
pub struct MessageTemplate {
    pub zh: String,
    pub en: String,
    pub ja: String,
    // 可以继续添加其他语言...
}

impl Default for MessageTemplate {
    fn default() -> Self {
        MessageTemplate {
            zh: String::new(),
            en: String::new(),
            ja: String::new(),
        }
    }
}

impl MessageTemplate {
    /// 根据语言获取消息
    pub fn get(&self, locale: Locale) -> &str {
        match locale.language {
            Language::ZhCN => &self.zh,
            Language::EnUS | Language::EnGB => &self.en,
            Language::JaJP => &self.ja,
            _ => &self.en,
        }
    }
    
    /// 格式化消息
    pub fn format(&self, locale: Locale, args: &[(&str, &str)]) -> String {
        let mut message = self.get(locale).to_string();
        
        for (key, value) in args {
            message = message.replace(&format!("{{{{{}}}}}", key), value);
        }
        
        message
    }
    
    /// 获取带原文的完整消息
    pub fn get_with_original(&self, locale: Locale) -> String {
        let native = self.get(locale);
        let original = self.get(Locale { language: Language::EnUS, ..locale });
        
        if locale.show_original && native != original {
            format!("{} ({})", native, original)
        } else {
            native.to_string()
        }
    }
}

/// 错误代码到消息的映射
pub struct ErrorMessages {
    /// 错误代码映射
    messages: HashMap<String, MessageTemplate>,
}

impl ErrorMessages {
    /// 加载所有错误消息
    pub fn load() -> Self {
        let mut messages = HashMap::new();
        
        // 加载基础错误消息
        messages.insert("E0001".to_string(), MessageTemplate {
            zh: "类型不匹配：期望 `{}`，实际找到 `{}"".to_string(),
            en: "Type mismatch: expected `{}`, found `{}`".to_string(),
            ja: "型の不一致：期待 `{}`、实际は `{}`".to_string(),
        });
        
        messages.insert("E0016".to_string(), MessageTemplate {
            zh: "无法推断参数 `{}` 的类型".to_string(),
            en: "Cannot infer type for parameter `{}`".to_string(),
            ja: "パラメータ `{}` の型を推測できません".to_string(),
        });
        
        // ... 更多错误消息
        
        ErrorMessages { messages }
    }
    
    /// 获取错误消息
    pub fn get(&self, code: &str) -> Option<&MessageTemplate> {
        self.messages.get(code)
    }
}
```

### 16.4 消息模块化加载

```rust
// src/util/diagnostic/messages/mod.rs

/// 基础错误消息
pub mod base_errors {
    use super::MessageTemplate;
    
    pub const TYPE_MISMATCH: MessageTemplate = MessageTemplate {
        zh: "类型不匹配：期望 `{expected}`，实际找到 `{found}`".to_string(),
        en: "Type mismatch: expected `{expected}`, found `{found}`".to_string(),
        ja: "型の不一致：期待 `{expected}`、实际は `{found}`".to_string(),
    };
    
    pub const UNKNOWN_VARIABLE: MessageTemplate = MessageTemplate {
        zh: "未知变量 `{name}`".to_string(),
        en: "Unknown variable `{name}`".to_string(),
        ja: "未知の変数 `{name}`".to_string(),
    };
}

/// 类型推断错误消息
pub mod type_inference_errors {
    use super::MessageTemplate;
    
    pub const CANNOT_INFER_PARAM: MessageTemplate = MessageTemplate {
        zh: "无法推断参数 `{name}` 的类型：{reason}".to_string(),
        en: "Cannot infer type for parameter `{name}`: {reason}".to_string(),
        ja: "パラメータ `{name}` の型を推測できません：{reason}".to_string(),
    };
    
    pub const CANNOT_INFER_RETURN: MessageTemplate = MessageTemplate {
        zh: "无法推断返回类型：{reason}".to_string(),
        en: "Cannot infer return type: {reason}".to_string(),
        ja: "戻り値の型を推測できません：{reason}".to_string(),
    };
    
    pub const LAMBDA_PARAM_CONTEXT: MessageTemplate = MessageTemplate {
        zh: "Lambda 参数无法从上下文推断类型".to_string(),
        en: "Lambda parameter types cannot be inferred from context".to_string(),
        ja: "Lambda パラメータの型はコンテキストから推測できません".to_string(),
    };
}

/// 运行时错误消息
pub mod runtime_errors {
    use super::MessageTemplate;
    
    pub const INDEX_OUT_OF_BOUNDS: MessageTemplate = MessageTemplate {
        zh: "索引越界：索引 {index} 超出数组大小 {size}".to_string(),
        en: "Index out of bounds: index {index} is out of bounds for size {size}".to_string(),
        ja: "インデックスエラー：インデックス {index} はサイズ {size} を超えています".to_string(),
    };
    
    pub const DIVISION_BY_ZERO: MessageTemplate = MessageTemplate {
        zh: "除以零错误".to_string(),
        en: "Division by zero".to_string(),
        ja: "零での除算エラー".to_string(),
    };
}

/// 建议消息
pub mod suggestions {
    use super::MessageTemplate;
    
    pub const ADD_TYPE_ANNOTATION: MessageTemplate = MessageTemplate {
        zh: "建议添加类型标注：{annotation}".to_string(),
        en: "Consider adding a type annotation: {annotation}".to_string(),
        ja: "型注釈を追加建议你：{annotation}".to_string(),
    };
    
    pub const CHECK_DOCS: MessageTemplate = MessageTemplate {
        zh: "请参阅文档：{url}".to_string(),
        en: "See documentation: {url}".to_string(),
        ja: "ドキュメントを参照してください：{url}".to_string(),
    };
}
```

### 16.5 格式化器集成多语种

```rust
// src/util/diagnostic/formatter.rs

/// 多语种诊断格式化器
pub struct I18nDiagnosticFormatter {
    /// 语言环境
    locale: Locale,
    /// 错误消息库
    messages: Arc<ErrorMessages>,
    /// 是否显示原文
    show_original: bool,
}

impl I18nDiagnosticFormatter {
    /// 格式化错误消息
    pub fn format_error(&self, error: &Diagnostic) -> String {
        let message_template = self.messages.get(&error.code);
        
        match message_template {
            Some(template) => {
                // 格式化参数
                let args = self.extract_args(error);
                template.format(self.locale, &args)
            }
            None => {
                // 使用默认消息
                error.message.clone()
            }
        }
    }
    
    /// 格式化帮助信息
    pub fn format_help(&self, help: &Help) -> String {
        match help {
            Help::Suggestion(text) => {
                let template = suggestions::ADD_TYPE_ANNOTATION;
                template.format(self.locale, &[("annotation", text)])
            }
            Help::Link(url) => {
                let template = suggestions::CHECK_DOCS;
                template.format(self.locale, &[("url", url)])
            }
        }
    }
    
    /// 获取当前语言
    pub fn locale(&self) -> Locale {
        self.locale
    }
}

/// CLI 集成示例
fn create_formatter(locale: Locale) -> I18nDiagnosticFormatter {
    I18nDiagnosticFormatter {
        locale,
        messages: Arc::new(ErrorMessages::load()),
        show_original: locale.show_original,
    }
}
```

### 16.6 配置文件中的语言设置

```yaml
# .yaoxiang/config.yaml

diagnostic:
  # 语言设置 (zh-CN, en-US, ja-JP, ko-KR, es-ES)
  language: "zh-CN"
  
  # 是否显示原文（双语模式）
  show_original: false
  
  # 是否使用本地化消息
  use_native: true
```

### 16.7 命令行语言参数

```bash
# 使用中文（默认）
yaoxiang run main.yx

# 使用英文
yaoxiang run main.yx --lang en-US

# 双语模式（中文 + 英文原文）
yaoxiang run main.yx --lang zh-CN --show-original

# 使用日语
yaoxiang run main.yx --lang ja-JP

# 设置环境变量
export YAOXIANG_LANG=zh-CN
yaoxiang run main.yx
```

### 16.8 语言文件热加载

```rust
// 动态加载语言包
pub struct LanguagePackLoader {
    /// 已加载的语言包
    loaded_packs: HashMap<Language, LanguagePack>,
}

impl LanguagePackLoader {
    /// 加载语言包
    pub fn load(&mut self, language: Language) -> Result<&LanguagePack, Error> {
        if let Some(pack) = self.loaded_packs.get(&language) {
            return Ok(pack);
        }
        
        // 从文件加载语言包
        let pack = LanguagePack::from_file(&self.get_pack_path(language))?;
        self.loaded_packs.insert(language, pack);
        
        Ok(self.loaded_packs.get(&language).unwrap())
    }
    
    /// 获取语言包文件路径
    fn get_pack_path(&self, language: Language) -> PathBuf {
        let lang_code = match language {
            Language::ZhCN => "zh-CN",
            Language::EnUS => "en-US",
            Language::JaJP => "ja-JP",
            _ => "en-US",
        };
        
        PathBuf::from(format!("resources/langs/{}.json", lang_code))
    }
}

/// 语言包 JSON 格式示例
/* resources/langs/zh-CN.json
{
  "meta": {
    "language": "zh-CN",
    "version": "1.0.0",
    "last_updated": "2024-01-01"
  },
  "messages": {
    "E0001": {
      "title": "类型不匹配",
      "description": "类型不匹配错误",
      "examples": [
        {
          "code": "let x: Int = \"hello\";",
          "output": "期望 `Int`，实际找到 `String`"
        }
      ],
      "help": [
        "检查类型标注是否正确",
        "使用类型转换方法"
      ]
    }
  }
}
*/
```

## 17. 性能考虑

### 17.1 错误格式化性能优化

- **延迟格式化**：只有在需要显示时才格式化详细信息
- **代码片段缓存**：避免重复读取和格式化相同文件
- **并发行格式化**：多线程处理多个错误
- **内存池**：重用诊断对象减少分配

### 17.2 栈跟踪收集优化

- **条件收集**：仅在启用调试模式时收集完整栈跟踪
- **帧采样**：对深度递归进行帧采样
- **压缩存储**：使用整数偏移代替完整文件名
