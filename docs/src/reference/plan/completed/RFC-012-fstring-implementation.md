# RFC-012 F-String 模板字符串实现计划

> **状态**: ✅ 已完成
> **基于 RFC**: RFC-012 F-String Template Strings
> **转换策略**: 统一转换为 `format()` 调用
> **完成日期**: 2025-07

---

## 实现目标

为 YaoXiang 语言添加 f-string 模板字符串语法糖支持：

```yaoxiang
// 变量插值
name = "Alice"
greeting = f"Hello {name}"        // → format("Hello {}", name)

// 表达式插值
x = 10
y = 20
result = f"Sum: {x + y}"         // → format("Sum: {}", x + y)

// 格式化说明符
pi = 3.14159
s = f"Pi: {pi:.2f}"              // → format("Pi: {:.2f}", pi)

// 多重插值
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"
```

---

## 架构设计

### 核心原则

1. **统一转换策略** - 所有 f-string 统一转换为 `format()` 调用
2. **编译时语法糖** - 不新增运行时能力，纯前端处理
3. **常量求值扩展** - 在 IR 层扩展常量求值，支持编译时计算

### 数据流

```
源代码 (f"...")
    ↓
Lexer: 识别 f" 前缀
    ↓
Parser: 解析插值表达式
    ↓
AST: 新增 FString 节点
    ↓
TypeCheck: 验证表达式类型
    ↓
Codegen: 转换为 format() 调用
    ↓
IR/目标代码
```

---

## 实现步骤

### Phase 1: Lexer 词法分析

**目标**: 识别 f-string 语法

**文件**: `src/frontend/core/lexer/`

**修改内容**:

1. **tokens.rs** - 新增 token 类型
   ```rust
   // 新增 FStringLiteral token（存储原始 f-string 内容）
   FStringLiteral(String),
   ```

2. **tokenizer.rs** - 识别 f" 前缀
   ```rust
   // 在 next_token() 中添加
   '"' => {
       // 检查前一个字符是否是 'f'
       // 如果是，调用 scan_fstring()
       // 否则调用 scan_string()
   }
   ```

3. **literals.rs** - 实现 f-string 扫描
   ```rust fn scan_fstring
   pub(lexer: &mut Lexer<'_>) -> Option<Token> {
       // 扫描 f"..." 内容
       // 解析 {expression} 插值
       // 返回 FStringLiteral(String)
   }
   ```

**验收标准**:
- [x] `f"hello"` 被识别为 FStringLiteral token
- [x] `f"Hello {name}"` 正确解析插值边界
- [x] 报错: 未闭合的 `{` 给出明确错误信息（`UnterminatedFStringInterpolation`）

---

### Phase 2: Parser 语法分析

**目标**: 解析 f-string 为 AST 节点

**文件**: `src/frontend/core/parser/`

**修改内容**:

1. **ast.rs** - 新增 AST 节点
   ```rust
   pub enum Expr {
       // ... existing ...
       /// F-string 模板字符串
       FString {
           segments: Vec<FStringSegment>,  // 文本段和插值表达式
           span: Span,
       },
   }

   pub enum FStringSegment {
       /// 文本片段
       Text(String),
       /// 插值表达式
       Interpolation {
           expr: Box<Expr>,
           format_spec: Option<String>,  // 可选的格式化说明符
       },
   }
   ```

2. **pratt/nud.rs** - 解析 f-string 字面量
   ```rust
   // 在 nud 表中添加
   TokenKind::FStringLiteral(_) => Some((BP_HIGHEST, Self::parse_fstring)),

   fn parse_fstring(&mut self) -> Option<Expr> {
       // 将 FStringLiteral 字符串解析为 FString AST 节点
   }
   ```

**验收标准**:
- [x] `f"hello"` 解析为 `Expr::FString { segments: [Text("hello")] }`
- [x] `f"hello {x}"` 正确解析插值表达式
- [x] `f"Pi: {pi:.2f}"` 正确解析格式说明符

---

### Phase 3: TypeCheck 类型检查

**目标**: 验证插值表达式类型

**文件**: `src/frontend/typecheck/inference/`

**修改内容**:

1. **expressions.rs** - 类型推断
   ```rust
   // 新增 f-string 类型推断
   fn infer_fstring(&mut self, segments: &[FStringSegment]) -> Result<MonoType> {
       // f-string 总是返回 String 类型
       // 验证每个插值表达式的类型是否实现了 Stringable trait
   }
   ```

2. **约束生成**（如需要）
   ```rust
   // 对于插值表达式，添加 Stringable 约束
   ```

**验收标准**:
- [x] `f"{42}"` 类型为 String
- [x] `f"{some_int}"` 正确验证 Int → Stringable
- [ ] 报错: 不支持 Stringable 的类型给出明确错误（待 trait 系统完善后实现）

---

### Phase 4: Codegen 代码生成

**目标**: 转换为 format() 调用

**文件**: `src/middle/core/ir_gen.rs` 或新建 `fstring.rs`

**修改内容**:

1. **转换为 format() 调用**
   ```rust
   // 示例转换
   f"Hello {name}" → format("Hello {}", name)
   f"Pi: {pi:.2f}" → format("Pi: {:.2f}", pi)
   ```

2. **IR 生成**
   ```rust
   fn gen_fstring(&mut self, segments: &[FStringSegment]) -> Operand {
       // 构建 format 调用
       // format_str: "Hello {}"
       // args: [name]
   }
   ```

**验收标准**:
- [x] `f"hello"` 生成正确的 format 调用
- [x] `f"x = {x}"` 正确传递参数
- [x] `f"Pi: {pi:.2f}"` 格式化说明符正确传递

---

### Phase 5: 常量求值优化

**目标**: 编译时常量计算

**文件**: `src/middle/core/ir_gen.rs`

**修改内容**:

1. **扩展 eval_const_expr**
   ```rust
   fn eval_const_expr(&self, expr: &Expr) -> Option<ConstValue> {
       match expr {
           // 现有
           Expr::Lit(lit) => eval_literal(lit),

           // 新增: 递归求值 f-string
           Expr::FString { segments } => {
               let mut result = String::new();
               for seg in segments {
                   match seg {
                       FStringSegment::Text(s) => result.push_str(s),
                       FStringSegment::Interpolation { expr, .. } => {
                           // 递归求值表达式
                           let val = self.eval_const_expr(expr)?;
                           result.push_str(&val.to_string());
                       }
                   }
               }
               Some(ConstValue::String(result))
           }

           // 现有: 支持 format() 常量调用
           Expr::Call { func, args } if is_const_format(func) => {
               self.eval_const_format(args)
           }
       }
   }
   ```

2. **常量注入**
   ```rust
   // 在 gen_expr 中
   if let Some(const_val) = self.eval_const_expr(expr) {
       // 直接使用常量值，无需生成运行时调用
       return Operand::Const(const_val);
   }
   ```

**验收标准**:
- [x] `f"hello"` 编译时求值为常量 "hello"
- [x] `f"x = {1+2}"` 编译时求值为 "x = 3"
- [x] 非常量插值正确生成运行时调用

---

## 测试设计

### 单元测试

#### 1. Lexer 测试

**文件**: `src/frontend/core/lexer/tests/fstring.rs` (新建)

```rust
#[test]
fn test_fstring_basic() {
    let mut lexer = Lexer::new(r#"f"hello""#);
    let token = lexer.next_token().unwrap();
    assert!(matches!(token.kind, TokenKind::FStringLiteral(_)));
}

#[test]
fn test_fstring_with_interpolation() {
    let mut lexer = Lexer::new(r#"f"hello {name}""#);
    let token = lexer.next_token().unwrap();
    // 验证 token 内容包含插值标记
}

#[test]
fn test_fstring_unclosed_brace_error() {
    let mut lexer = Lexer::new(r#"f"hello {name""#);
    // 验证报错信息
}
```

#### 2. Parser 测试

**文件**: `src/frontend/core/parser/tests/fstring.rs` (新建)

```rust
#[test]
fn test_parse_fstring_text() {
    let tokens = tokenize(r#"f"hello""#);
    let ast = parse(tokens);
    assert_matches!(ast, Expr::FString { segments, .. }
        if segments.len() == 1
    );
}

#[test]
fn test_parse_fstring_interpolation() {
    let tokens = tokenize(r#"f"hello {name}""#);
    let ast = parse(tokens);
    // 验证 segments = [Text("hello "), Interpolation(Var("name"))]
}

#[test]
fn test_parse_fstring_format_spec() {
    let tokens = tokenize(r#"f"Pi: {pi:.2f}""#);
    let ast = parse(tokens);
    // 验证 format_spec = Some(".2f")
}
```

#### 3. TypeCheck 测试

**文件**: `src/frontend/typecheck/tests/fstring.rs` (新建)

```rust
#[test]
fn test_fstring_type_int() {
    let code = r#"
        x = 10
        s = f"value: {x}"
    "#;
    check_types(code);
}

#[test]
fn test_fstring_type_not_stringable() {
    let code = r#"
        struct NotStringable
        x = NotStringable()
        s = f"value: {x}"  // 应该报错
    "#;
    check_type_error(code, "does not implement Stringable");
}
```

#### 4. Codegen 测试

**文件**: `tests/integration/fstring.rs` (新建)

```rust
#[test]
fn test_fstring_basic() {
    let result = run(r#"
        print(f"hello world")
    "#);
    assert_eq!(result, "hello world");
}

#[test]
fn test_fstring_interpolation() {
    let result = run(r#"
        name = "Alice"
        print(f"Hello {name}")
    "#);
    assert_eq!(result, "Hello Alice");
}

#[test]
fn test_fstring_format_spec() {
    let result = run(r#"
        pi = 3.14159
        print(f"Pi: {pi:.2f}")
    "#);
    assert_eq!(result, "Pi: 3.14");
}

#[test]
fn test_fstring_expression() {
    let result = run(r#"
        x = 10
        y = 20
        print(f"{x} + {y} = {x + y}")
    "#);
    assert_eq!(result, "10 + 20 = 30");
}

#[test]
fn test_fstring_const_eval() {
    let result = run(r#"
        x = f"hello {1+2}"
        print(x)
    "#);
    // 常量求值结果
    assert_eq!(result, "hello 3");
}
```

### 集成测试

```rust
// 测试实际场景
#[test]
fn test_fstring_logging() {
    let code = r#"
        log(level: String, msg: String) = () => {
            timestamp = "2024-01-01"
            print(f"[{timestamp}] {level}: {msg}")
        }
        log("INFO", "system started")
    "#;
    // 期望输出: [2024-01-01] INFO: system started
}

#[test]
fn test_fstring_json_like() {
    let code = r#"
        name = "Alice"
        age = 30
        print(f"{ {"name": "{name}", "age": {age}} }")
    "#;
    // 期望输出: { "name": "Alice", "age": 30 }
}
```

---

## 关键文件清单

| 文件 | 修改类型 | 说明 |
|------|---------|------|
| `src/frontend/core/lexer/tokens.rs` | 修改 | 新增 FStringLiteral |
| `src/frontend/core/lexer/tokenizer.rs` | 修改 | 识别 f" 前缀 |
| `src/frontend/core/lexer/literals.rs` | 修改 | 扫描 f-string |
| `src/frontend/core/parser/ast.rs` | 修改 | 新增 FString 节点 |
| `src/frontend/core/parser/pratt/nud.rs` | 修改 | 解析 f-string |
| `src/frontend/typecheck/inference/expressions.rs` | 修改 | 类型推断 |
| `src/middle/core/ir_gen.rs` | 修改 | 代码生成 + 常量求值 |
| `src/frontend/core/lexer/tests/fstring.rs` | 新增 | Lexer 测试 |
| `src/frontend/core/parser/tests/fstring.rs` | 新增 | Parser 测试 |
| `src/frontend/typecheck/tests/fstring.rs` | 新增 | TypeCheck 测试 |
| `tests/integration/fstring.rs` | 新增 | 集成测试 |

---

## 依赖与风险

### 依赖

- **已有**: `format()` 函数 (`src/std/string.rs`)
- **已有**: 常量求值框架 (`ir_gen.rs::eval_const_expr`)
- **无需**: 新增外部依赖

### 风险

1. **嵌套花括号解析**: `{ { x } }` 场景
   - 解决: RFC 规定限制嵌套使用

2. **格式说明符复杂度**
   - 解决: 复用现有 format 函数解析逻辑

---

## 里程碑

- [x] Phase 1: Lexer 识别 f-string
- [x] Phase 2: Parser 解析为 AST
- [x] Phase 3: TypeCheck 类型验证
- [x] Phase 4: Codegen 转换为 format()
- [x] Phase 5: 常量求值优化
- [x] 完整测试覆盖（27 个测试: 10 词法器 + 6 解析器 + 4 类型检查 + 7 集成）

---

## 附录

### 参考实现

- Python f-strings: https://docs.python.org/3/tutorial/inputoutput.html
- Rust format!: https://doc.rust-lang.org/std/macro.format.html

### 相关 RFC

- RFC-012: F-String Template Strings (本文档基于)

---

## 实现记录

### 实际修改文件

| 文件 | 修改类型 | 具体变更 |
|------|---------|---------|
| `src/frontend/core/lexer/tokens.rs` | 修改 | 新增 `FStringLiteral(String)` token 及 `UnterminatedFStringInterpolation` 错误 |
| `src/frontend/core/lexer/tokenizer.rs` | 修改 | `scan_identifier()` 中检测 `f"` 前缀并调用 `scan_fstring()` |
| `src/frontend/core/lexer/literals.rs` | 修改 | 新增 `scan_fstring()` 函数 (~180行)，支持 `{}` 插值、`{{`/`}}` 转义、嵌套大括号深度追踪 |
| `src/frontend/core/lexer/mod.rs` | 修改 | `log_token()` 中新增 FStringLiteral 分支；引入 fstring 测试模块 |
| `src/frontend/core/parser/ast.rs` | 修改 | 新增 `FString` AST 节点及 `FStringSegment` 枚举 |
| `src/frontend/core/parser/pratt/nud.rs` | 修改 | 新增 `parse_fstring()`、`parse_fstring_segments()`、`split_format_spec()` |
| `src/frontend/typecheck/inference/expressions.rs` | 修改 | `infer_expr()` 中新增 `Expr::FString` 分支，返回 `MonoType::String` |
| `src/middle/core/ir_gen.rs` | 修改 | `get_expr_span()`、`eval_const_expr()`、`generate_expr_ir()` 三处新增 FString 处理 |
| `src/frontend/core/lexer/tests/fstring.rs` | 新增 | 10 个词法器测试 |
| `src/frontend/core/parser/tests/fstring.rs` | 新增 | 6 个解析器测试 |
| `src/frontend/typecheck/tests/fstring.rs` | 新增 | 4 个类型检查测试 |
| `tests/integration/fstring.rs` | 新增 | 7 个端到端集成测试 |
| `tests/integration.rs` | 修改 | 注册 fstring 集成测试模块 |

### 实现要点

1. **词法器**: f-string 整体存为单个 `FStringLiteral` token，`{}` 插值标记保留在字符串内容中
2. **解析器**: `parse_fstring_segments()` 将原始内容拆分为 `Text`/`Interpolation` 段，插值表达式使用完整的 lexer+parser 重新解析
3. **代码生成**: 转换为 `std.string.format()` 调用，使用位置占位符 `{0}`, `{1}` 等；格式说明符如 `{0:.2f}` 直接传递
4. **常量优化**: 当所有插值表达式都是编译时常量（且无格式说明符），整个 f-string 在编译时折叠为常量字符串
