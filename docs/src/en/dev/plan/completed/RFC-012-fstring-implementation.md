# RFC-012 F-String Template Strings Implementation Plan

> **Status**: ✅ Completed
> **Based on RFC**: RFC-012 F-String Template Strings
> **Conversion Strategy**: Unified conversion to `format()` calls
> **Completion Date**: 2025-07

---

## Implementation Goals

Add f-string template string syntactic sugar support for YaoXiang language:

```yaoxiang
// Variable interpolation
name = "Alice"
greeting = f"Hello {name}"        // → format("Hello {}", name)

// Expression interpolation
x = 10
y = 20
result = f"Sum: {x + y}"         // → format("Sum: {}", x + y)

// Format specifiers
pi = 3.14159
s = f"Pi: {pi:.2f}"              // → format("Pi: {:.2f}", pi)

// Multiple interpolations
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"
```

---

## Architecture Design

### Core Principles

1. **Unified Conversion Strategy** - All f-strings are converted to `format()` calls
2. **Compile-time Syntactic Sugar** - No new runtime capabilities, pure frontend processing
3. **Constant Evaluation Extension** - Extend constant evaluation at IR layer for compile-time computation

### Data Flow

```
Source Code (f"...")
    ↓
Lexer: Recognize f" prefix
    ↓
Parser: Parse interpolation expressions
    ↓
AST: New FString node
    ↓
TypeCheck: Validate expression types
    ↓
Codegen: Convert to format() calls
    ↓
IR/Target Code
```

---

## Implementation Steps

### Phase 1: Lexer Tokenization

**Goal**: Recognize f-string syntax

**File**: `src/frontend/core/lexer/`

**Modifications**:

1. **tokens.rs** - Add new token type
   ```rust
   // New FStringLiteral token (stores raw f-string content)
   FStringLiteral(String),
   ```

2. **tokenizer.rs** - Recognize f" prefix
   ```rust
   // Add in next_token()
   '"' => {
       // Check if previous character is 'f'
       // If so, call scan_fstring()
       // Otherwise call scan_string()
   }
   ```

3. **literals.rs** - Implement f-string scanning
   ```rust fn scan_fstring
   pub(lexer: &mut Lexer<'_>) -> Option<Token> {
       // Scan f"..." content
       // Parse {expression} interpolations
       // Return FStringLiteral(String)
   }
   ```

**Acceptance Criteria**:
- [x] `f"hello"` is recognized as FStringLiteral token
- [x] `f"Hello {name}"` correctly parses interpolation boundaries
- [x] Error: Unclosed `{` gives clear error message (`UnterminatedFStringInterpolation`)

---

### Phase 2: Parser Syntax Analysis

**Goal**: Parse f-string into AST nodes

**File**: `src/frontend/core/parser/`

**Modifications**:

1. **ast.rs** - Add new AST node
   ```rust
   pub enum Expr {
       // ... existing ...
       /// F-string template string
       FString {
           segments: Vec<FStringSegment>,  // Text segments and interpolation expressions
           span: Span,
       },
   }

   pub enum FStringSegment {
       /// Text fragment
       Text(String),
       /// Interpolation expression
       Interpolation {
           expr: Box<Expr>,
           format_spec: Option<String>,  // Optional format specifier
       },
   }
   ```

2. **pratt/nud.rs** - Parse f-string literal
   ```rust
   // Add to nud table
   TokenKind::FStringLiteral(_) => Some((BP_HIGHEST, Self::parse_fstring)),

   fn parse_fstring(&mut self) -> Option<Expr> {
       // Parse FStringLiteral string into FString AST node
   }
   ```

**Acceptance Criteria**:
- [x] `f"hello"` parses to `Expr::FString { segments: [Text("hello")] }`
- [x] `f"hello {x}"` correctly parses interpolation expression
- [x] `f"Pi: {pi:.2f}"` correctly parses format specifier

---

### Phase 3: TypeCheck Type Checking

**Goal**: Validate interpolation expression types

**File**: `src/frontend/typecheck/inference/`

**Modifications**:

1. **expressions.rs** - Type inference
   ```rust
   // Add f-string type inference
   fn infer_fstring(&mut self, segments: &[FStringSegment]) -> Result<MonoType> {
       // f-string always returns String type
       // Validate that each interpolation expression's type implements Stringable trait
   }
   ```

2. **Constraint Generation** (if needed)
   ```rust
   // For interpolation expressions, add Stringable constraint
   ```

**Acceptance Criteria**:
- [x] `f"{42}"` has type String
- [x] `f"{some_int}"` correctly validates Int → Stringable
- [ ] Error: Types not supporting Stringable give clear error (to be implemented after trait system is complete)

---

### Phase 4: Codegen Code Generation

**Goal**: Convert to format() calls

**File**: `src/middle/core/ir_gen.rs` or new `fstring.rs`

**Modifications**:

1. **Convert to format() calls**
   ```rust
   // Example conversions
   f"Hello {name}" → format("Hello {}", name)
   f"Pi: {pi:.2f}" → format("Pi: {:.2f}", pi)
   ```

2. **IR Generation**
   ```rust
   fn gen_fstring(&mut self, segments: &[FStringSegment]) -> Operand {
       // Build format call
       // format_str: "Hello {}"
       // args: [name]
   }
   ```

**Acceptance Criteria**:
- [x] `f"hello"` generates correct format call
- [x] `f"x = {x}"` correctly passes parameters
- [x] `f"Pi: {pi:.2f}"` format specifier correctly passed

---

### Phase 5: Constant Evaluation Optimization

**Goal**: Compile-time constant computation

**File**: `src/middle/core/ir_gen.rs`

**Modifications**:

1. **Extend eval_const_expr**
   ```rust
   fn eval_const_expr(&self, expr: &Expr) -> Option<ConstValue> {
       match expr {
           // Existing
           Expr::Lit(lit) => eval_literal(lit),

           // New: Recursively evaluate f-string
           Expr::FString { segments } => {
               let mut result = String::new();
               for seg in segments {
                   match seg {
                       FStringSegment::Text(s) => result.push_str(s),
                       FStringSegment::Interpolation { expr, .. } => {
                           // Recursively evaluate expression
                           let val = self.eval_const_expr(expr)?;
                           result.push_str(&val.to_string());
                       }
                   }
               }
               Some(ConstValue::String(result))
           }

           // Existing: Support format() constant calls
           Expr::Call { func, args } if is_const_format(func) => {
               self.eval_const_format(args)
           }
       }
   }
   ```

2. **Constant Injection**
   ```rust
   // In gen_expr
   if let Some(const_val) = self.eval_const_expr(expr) {
       // Use constant value directly, no runtime call needed
       return Operand::Const(const_val);
   }
   ```

**Acceptance Criteria**:
- [x] `f"hello"` evaluates to constant "hello" at compile time
- [x] `f"x = {1+2}"` evaluates to "x = 3" at compile time
- [x] Non-constant interpolations correctly generate runtime calls

---

## Test Design

### Unit Tests

#### 1. Lexer Tests

**File**: `src/frontend/core/lexer/tests/fstring.rs` (new)

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
    // Verify token content includes interpolation markers
}

#[test]
fn test_fstring_unclosed_brace_error() {
    let mut lexer = Lexer::new(r#"f"hello {name""#);
    // Verify error message
}
```

#### 2. Parser Tests

**File**: `src/frontend/core/parser/tests/fstring.rs` (new)

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
    // Verify segments = [Text("hello "), Interpolation(Var("name"))]
}

#[test]
fn test_parse_fstring_format_spec() {
    let tokens = tokenize(r#"f"Pi: {pi:.2f}""#);
    let ast = parse(tokens);
    // Verify format_spec = Some(".2f")
}
```

#### 3. TypeCheck Tests

**File**: `src/frontend/typecheck/tests/fstring.rs` (new)

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
        s = f"value: {x}"  // Should error
    "#;
    check_type_error(code, "does not implement Stringable");
}
```

#### 4. Codegen Tests

**File**: `tests/integration/fstring.rs` (new)

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
    // Constant evaluation result
    assert_eq!(result, "hello 3");
}
```

### Integration Tests

```rust
// Test actual scenarios
#[test]
fn test_fstring_logging() {
    let code = r#"
        log(level: String, msg: String) = () => {
            timestamp = "2024-01-01"
            print(f"[{timestamp}] {level}: {msg}")
        }
        log("INFO", "system started")
    "#;
    // Expected output: [2024-01-01] INFO: system started
}

#[test]
fn test_fstring_json_like() {
    let code = r#"
        name = "Alice"
        age = 30
        print(f"{{"name": "{name}", "age": {age}}}")
    "#;
    // Expected output: { "name": "Alice", "age": 30 }
}
```

---

## Key File Checklist

| File | Modification Type | Description |
|------|-------------------|-------------|
| `src/frontend/core/lexer/tokens.rs` | Modify | Add FStringLiteral |
| `src/frontend/core/lexer/tokenizer.rs` | Modify | Recognize f" prefix |
| `src/frontend/core/lexer/literals.rs` | Modify | Scan f-string |
| `src/frontend/core/parser/ast.rs` | Modify | Add FString node |
| `src/frontend/core/parser/pratt/nud.rs` | Modify | Parse f-string |
| `src/frontend/typecheck/inference/expressions.rs` | Modify | Type inference |
| `src/middle/core/ir_gen.rs` | Modify | Code generation + constant evaluation |
| `src/frontend/core/lexer/tests/fstring.rs` | New | Lexer tests |
| `src/frontend/core/parser/tests/fstring.rs` | New | Parser tests |
| `src/frontend/typecheck/tests/fstring.rs` | New | TypeCheck tests |
| `tests/integration/fstring.rs` | New | Integration tests |

---

## Dependencies and Risks

### Dependencies

- **Existing**: `format()` function (`src/std/string.rs`)
- **Existing**: Constant evaluation framework (`ir_gen.rs::eval_const_expr`)
- **Not needed**: New external dependencies

### Risks

1. **Nested brace parsing**: `{ { x } }` scenario
   - Solution: RFC specifies limitations on nested usage

2. **Format specifier complexity**
   - Solution: Reuse existing format function parsing logic

---

## Milestones

- [x] Phase 1: Lexer recognizes f-string
- [x] Phase 2: Parser parses to AST
- [x] Phase 3: TypeCheck type validation
- [x] Phase 4: Codegen converts to format()
- [x] Phase 5: Constant evaluation optimization
- [x] Full test coverage (27 tests: 10 lexer + 6 parser + 4 typecheck + 7 integration)

---

## Appendix

### Reference Implementations

- Python f-strings: https://docs.python.org/3/tutorial/inputoutput.html
- Rust format!: https://doc.rust-lang.org/std/macro.format.html

### Related RFCs

- RFC-012: F-String Template Strings (this document is based on)

---

## Implementation Log

### Actual Modified Files

| File | Modification Type | Specific Changes |
|------|-------------------|------------------|
| `src/frontend/core/lexer/tokens.rs` | Modify | Added `FStringLiteral(String)` token and `UnterminatedFStringInterpolation` error |
| `src/frontend/core/lexer/tokenizer.rs` | Modify | In `scan_identifier()`, detect `f"` prefix and call `scan_fstring()` |
| `src/frontend/core/lexer/literals.rs` | Modify | Added `scan_fstring()` function (~180 lines), supports `{}` interpolation, `{` `}` escaping, nested brace depth tracking |
| `src/frontend/core/lexer/mod.rs` | Modify | Added FStringLiteral branch in `log_token()`; introduced fstring test module |
| `src/frontend/core/parser/ast.rs` | Modify | Added `FString` AST node and `FStringSegment` enum |
| `src/frontend/core/parser/pratt/nud.rs` | Modify | Added `parse_fstring()`, `parse_fstring_segments()`, `split_format_spec()` |
| `src/frontend/typecheck/inference/expressions.rs` | Modify | Added `Expr::FString` branch in `infer_expr()`, returns `MonoType::String` |
| `src/middle/core/ir_gen.rs` | Modify | Added FString handling in `get_expr_span()`, `eval_const_expr()`, `generate_expr_ir()` |
| `src/frontend/core/lexer/tests/fstring.rs` | New | 10 lexer tests |
| `src/frontend/core/parser/tests/fstring.rs` | New | 6 parser tests |
| `src/frontend/typecheck/tests/fstring.rs` | New | 4 typecheck tests |
| `tests/integration/fstring.rs` | New | 7 end-to-end integration tests |
| `tests/integration.rs` | Modify | Registered fstring integration test module |

### Implementation Highlights

1. **Lexer**: f-string is stored as a single `FStringLiteral` token, `{}` interpolation markers are preserved in string content
2. **Parser**: `parse_fstring_segments()` splits raw content into `Text`/`Interpolation` segments, interpolation expressions are re-parsed using full lexer+parser
3. **Code generation**: Convert to `std.string.format()` calls, use positional placeholders `{0}`, `{1}`, etc.; format specifiers like `{0:.2f}` are passed directly
4. **Constant optimization**: When all interpolation expressions are compile-time constants (and no format specifiers), the entire f-string is folded into a constant string at compile time