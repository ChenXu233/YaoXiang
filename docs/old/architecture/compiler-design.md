# YaoXiang 编译器设计文档

> 版本：v2.0.0
> 状态：正式
> 作者：沫郁酱
> 日期：2025-01-04

---

## 目录

1. [概述](#一概述)
2. [前端设计](#二前端设计)
3. [中端设计](#三中端设计)
4. [类型系统实现](#四类型系统实现)
5. [代码生成器设计](#五代码生成器设计)
6. [优化策略](#六优化策略)
7. [错误处理](#七错误处理)
8. [性能考虑](#八性能考虑)

---

## 一、概述

YaoXiang 编译器采用现代分层架构设计，支持从源代码到字节码的完整编译流程。编译器设计的核心目标是：

- **高性能**：零成本抽象，编译时优化
- **可扩展性**：易于添加新语言特性和优化
- **可调试性**：清晰的中间表示和错误信息
- **AI友好**：严格的结构化语法，明确的语义

### 编译阶段总览

```
源代码
  ↓
┌─────────────────────────────────────────────────────────┐
│                     前端 (Frontend)                      │
├─────────────────────────────────────────────────────────┤
│  词法分析 → Token 流                                    │
│  ↓                                                      │
│  语法分析 → AST (抽象语法树)                            │
│  ↓                                                      │
│  类型检查 → 带类型的 AST + 类型约束                     │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│                     中端 (Middle)                        │
├─────────────────────────────────────────────────────────┤
│  中间表示 → IR (SSA 形式)                               │
│  ↓                                                      │
│  优化 → 优化后的 IR                                     │
│  ↓                                                      │
│  泛型单态化 → 具体 IR                                   │
│  ↓                                                      │
│  逃逸分析 → 内存分配策略                                │
│  ↓                                                      │
│  生命周期分析                                           │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│                     后端 (Backend)                       │
├─────────────────────────────────────────────────────────┤
│  字节码生成 → Instruction 流                            │
│  ↓                                                      │
│  输出 → 字节码文件 (.42)                                │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│                     运行时 (Runtime)                     │
├─────────────────────────────────────────────────────────┤
│  虚拟机执行 → 执行字节码                                │
│  调度器管理 → 并作任务调度                              │
│  内存管理 → 栈/堆分配                                   │
└─────────────────────────────────────────────────────────┘
```

### 核心文件结构

```
src/frontend/
├── lexer/
│   ├── mod.rs            # 词法分析器入口
│   ├── tokens.rs         # Token 定义
│   └── tests/
├── parser/
│   ├── mod.rs            # 解析器入口
│   ├── ast.rs            # AST 节点定义
│   ├── expr.rs           # 表达式解析
│   ├── nud.rs            # 前缀解析（Pratt Parser）
│   ├── led.rs            # 中缀解析（Pratt Parser）
│   ├── stmt.rs           # 语句解析
│   ├── state.rs          # 解析器状态
│   ├── type_parser.rs    # 类型解析
│   └── tests/
└── typecheck/
    ├── mod.rs            # 类型检查入口
    ├── types.rs          # 类型定义
    ├── infer.rs          # 类型推断
    ├── check.rs          # 类型验证
    ├── specialize.rs     # 泛型特化
    ├── errors.rs         # 类型错误
    └── tests/

src/middle/
├── mod.rs
├── ir.rs                 # 中间表示
├── optimizer.rs          # 优化器
├── codegen/              # 代码生成器
│   ├── mod.rs
│   ├── bytecode.rs
│   ├── expr.rs
│   ├── stmt.rs
│   ├── control_flow.rs
│   ├── loop_gen.rs
│   ├── switch.rs
│   ├── closure.rs
│   ├── generator.rs
│   └── tests/
├── monomorphize/         # 单态化
├── escape_analysis/      # 逃逸分析
└── lifetime/             # 生命周期分析
```

---

## 二、前端设计

### 2.1 词法分析器 (Lexer)

#### 2.1.1 设计原则

- **高性能**：单次扫描，O(n) 时间复杂度
- **精确位置**：记录每个 Token 的行号、列号
- **容错性**：遇到错误时尽可能继续扫描

#### 2.1.2 实现细节

**核心文件**：`src/frontend/lexer/mod.rs`, `src/frontend/lexer/tokens.rs`

```rust
// Token 定义（tokens.rs）
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // 标识符和字面量
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),
    BoolLiteral(bool),

    // 关键字
    Type, Pub, Use, Spawn, Ref, Mut,
    If, Elif, Else, Match, While, For, Return,

    // 符号
    LParen, RParen, LBrace, RBrace,
    Comma, Colon, Semicolon,

    // 运算符
    Equal, Arrow, Plus, Minus, Star, Slash,
    Eq, Ne, Lt, Le, Gt, Ge,

    // 特殊
    Eof,
}

// 字面量类型
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
}

// 词法分析器（mod.rs）
pub struct Lexer {
    input: String,
    chars: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self {
            input: input.clone(),
            chars: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        if self.is_at_end() {
            return Ok(Token::Eof);
        }

        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        match self.peek() {
            // 标识符和关键字
            c if c.is_alphabetic() || c == '_' => {
                self.read_identifier()
            }

            // 数字字面量
            c if c.is_digit(10) => {
                self.read_number()
            }

            // 字符串
            '"' => {
                self.read_string()
            }

            // 符号
            '(' => { self.advance(); Ok(Token::LParen) }
            ')' => { self.advance(); Ok(Token::RParen) }
            '{' => { self.advance(); Ok(Token::LBrace) }
            '}' => { self.advance(); Ok(Token::RBrace) }
            '=' => {
                self.advance();
                if self.peek() == '>' {
                    self.advance();
                    Ok(Token::Arrow)
                } else {
                    Ok(Token::Equal)
                }
            }

            // 运算符
            '+' => { self.advance(); Ok(Token::Plus) }
            '-' => { self.advance(); Ok(Token::Minus) }
            '*' => { self.advance(); Ok(Token::Star) }
            '/' => { self.advance(); Ok(Token::Slash) }

            // 未知字符
            _ => {
                let ch = self.advance();
                Err(LexError::UnexpectedCharacter(ch))
            }
        }
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.position;
        while !self.is_at_end() {
            let c = self.peek();
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let ident: String = self.chars[start..self.position].iter().collect();

        // 关键字检查
        match ident.as_str() {
            "type" => Token::Type,
            "pub" => Token::Pub,
            "use" => Token::Use,
            "spawn" => Token::Spawn,
            "ref" => Token::Ref,
            "mut" => Token::Mut,
            "if" => Token::If,
            "elif" => Token::Elif,
            "else" => Token::Else,
            "match" => Token::Match,
            "while" => Token::While,
            "for" => Token::For,
            "return" => Token::Return,
            "true" => Token::BoolLiteral(true),
            "false" => Token::BoolLiteral(false),
            _ => Token::Identifier(ident),
        }
    }
}
```

#### 2.1.3 词法错误处理

```rust
#[derive(Debug, Error)]
pub enum LexError {
    #[error("Unexpected character: {0}")]
    UnexpectedCharacter(char),

    #[error("Unclosed string literal")]
    UnclosedString,

    #[error("Invalid number format: {0}")]
    InvalidNumber(String),

    #[error("Invalid escape sequence: {0}")]
    InvalidEscape(String),
}
```

### 2.2 语法分析器 (Parser)

#### 2.2.1 设计原则

- **左递归消除**：使用 Pratt Parser 处理表达式
- **清晰的优先级**：显式定义运算符优先级
- **错误恢复**：遇到错误时跳过到安全位置
- **状态管理**：使用 `ParserState` 跟踪解析上下文

#### 2.2.2 Pratt Parser 实现

**核心文件**：`src/frontend/parser/mod.rs`, `src/frontend/parser/nud.rs`, `src/frontend/parser/led.rs`

```rust
// 表达式优先级
#[derive(PartialOrd, PartialEq, Clone, Copy)]
enum Precedence {
    Lowest,
    Equals,      // == !=
    LessGreater, // < > <= >=
    Add,         // + -
    Multiply,    // * /
    Unary,       // ! -
    Call,        // function(x)
    Index,       // array[i]
    Field,       // obj.field
}

// 解析器状态
pub struct ParserState {
    // 当前作用域级别
    scope_depth: usize,

    // 标签（用于 break/continue）
    loop_labels: Vec<String>,

    // 是否在函数定义中
    in_function: bool,

    // 函数返回类型（用于 return 检查）
    expected_return_type: Option<Type>,
}

// 解析器
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    state: ParserState,
}

impl Parser {
    /// 创建新解析器
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            state: ParserState::default(),
        }
    }

    /// 解析入口
    pub fn parse(&mut self) -> Result<Module, ParseError> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            if let Some(item) = self.parse_item()? {
                items.push(item);
            }
        }
        Ok(Module { items })
    }

    /// 解析模块项（函数、类型定义、变量等）
    fn parse_item(&mut self) -> Result<Option<Item>, ParseError> {
        match self.current_token() {
            Token::Function => self.parse_function().map(Some),
            Token::Type => self.parse_type_def().map(Some),
            Token::Let => self.parse_let_statement().map(|s| Some(Item::Stmt(s))),
            Token::Use => self.parse_use().map(Some),
            _ => {
                // 尝试解析为表达式语句
                if self.is_expr_start(self.current_token()) {
                    let expr = self.parse_expression(Precedence::Lowest)?;
                    self.consume(Token::Semicolon, "Expected ';'")?;
                    Ok(Some(Item::Stmt(Stmt::Expr(expr))))
                } else {
                    Err(ParseError::UnexpectedToken(
                        self.current_token().clone(),
                        "expected function, type, or let statement".to_string(),
                    ))
                }
            }
        }
    }

    /// 表达式解析入口
    pub fn parse_expression(&mut self, prec: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() && prec <= self.get_precedence() {
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    /// 前缀解析（nud.rs）
    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        match self.current_token() {
            Token::Integer(n) => {
                self.advance();
                Ok(Expr::Lit(Literal::Int(*n), self.span()))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expr::Lit(Literal::Float(*f), self.span()))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr::Lit(Literal::String(s.clone()), self.span()))
            }
            Token::BoolLiteral(b) => {
                self.advance();
                Ok(Expr::Lit(Literal::Bool(*b), self.span()))
            }
            Token::Identifier(name) => {
                self.advance();
                Ok(Expr::Var(name.clone(), self.span()))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression(Precedence::Lowest)?;
                self.consume(Token::RParen, "Expected ')'")?;
                Ok(expr)
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_expression(Precedence::Unary)?;
                Ok(Expr::UnOp {
                    op: UnOp::Negate,
                    expr: Box::new(expr),
                    span: self.span(),
                })
            }
            Token::Bang => {
                self.advance();
                let expr = self.parse_expression(Precedence::Unary)?;
                Ok(Expr::UnOp {
                    op: UnOp::Not,
                    expr: Box::new(expr),
                    span: self.span(),
                })
            }
            Token::If => self.parse_if(),
            Token::Match => self.parse_match(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Return => self.parse_return(),
            _ => Err(ParseError::UnexpectedToken(
                self.current_token().clone(),
                "Expected expression".to_string(),
            )),
        }
    }

    /// 中缀解析（led.rs）
    fn parse_infix(&mut self, left: Expr) -> Result<Expr, ParseError> {
        match self.current_token() {
            Token::Plus | Token::Minus | Token::Star | Token::Slash => {
                let op = self.parse_binary_op()?;
                self.advance();
                let prec = self.get_precedence();
                let right = self.parse_expression(prec)?;
                Ok(Expr::BinOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    span: self.span(),
                })
            }
            Token::LParen => {
                self.advance();
                let args = self.parse_argument_list()?;
                Ok(Expr::Call {
                    func: Box::new(left),
                    args,
                    span: self.span(),
                })
            }
            Token::LBracket => {
                self.advance();
                let index = self.parse_expression(Precedence::Lowest)?;
                self.consume(Token::RBracket, "Expected ']'")?;
                Ok(Expr::Index {
                    expr: Box::new(left),
                    index: Box::new(index),
                    span: self.span(),
                })
            }
            Token::Dot => {
                self.advance();
                let field = match self.current_token() {
                    Token::Identifier(name) => {
                        let name = name.clone();
                        self.advance();
                        name
                    }
                    _ => return Err(ParseError::ExpectedField),
                };
                Ok(Expr::FieldAccess {
                    expr: Box::new(left),
                    field,
                    span: self.span(),
                })
            }
            _ => Err(ParseError::UnexpectedToken(
                self.current_token().clone(),
                "Expected infix operator".to_string(),
            )),
        }
    }

    /// 运算符优先级映射
    fn get_precedence(&self) -> Precedence {
        match self.current_token() {
            Token::Eq | Token::Ne => Precedence::Equals,
            Token::Lt | Token::Le | Token::Gt | Token::Ge => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Add,
            Token::Star | Token::Slash => Precedence::Multiply,
            Token::LParen => Precedence::Call,
            Token::LBracket => Precedence::Index,
            Token::Dot => Precedence::Field,
            _ => Precedence::Lowest,
        }
    }
}
```

#### 2.2.3 语句解析

```rust
impl Parser {
    /// 解析 let 语句
    fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // 消耗 'let'

        let name = match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::ExpectedIdentifier),
        };

        let ty = if self.current_token() == &Token::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(Token::Equal, "Expected '='")?;
        let value = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::Semicolon, "Expected ';'")?;

        Ok(Stmt::Let {
            name,
            ty,
            value,
        })
    }

    /// 解析函数定义
    fn parse_function(&mut self) -> Result<Item, ParseError> {
        let span = self.span();
        self.advance(); // 消耗 'fn'

        let name = match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::ExpectedIdentifier),
        };

        // 解析参数列表
        self.consume(Token::LParen, "Expected '('")?;
        let mut params = Vec::new();
        while !self.matches(Token::RParen) {
            if !params.is_empty() {
                self.consume(Token::Comma, "Expected ','")?;
            }
            let param_name = match self.current_token() {
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                }
                _ => return Err(ParseError::ExpectedIdentifier),
            };
            self.consume(Token::Colon, "Expected ':'")?;
            let param_type = self.parse_type()?;
            params.push(Param {
                name: param_name,
                ty: param_type,
            });
        }
        self.consume(Token::RParen, "Expected ')'")?;

        // 返回类型
        let return_type = if self.matches(Token::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // 函数体
        let body = self.parse_block()?;

        Ok(Item::Function(Function {
            name,
            params,
            return_type,
            body,
            span,
        }))
    }

    /// 解析 if 语句
    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        self.advance(); // 消耗 'if'

        let condition = Box::new(self.parse_expression(Precedence::Lowest)?);
        let then_branch = Box::new(self.parse_block()?);

        let mut elif_branches = Vec::new();
        let mut else_branch: Option<Box<Expr>> = None;

        while self.matches(Token::Elif) {
            let elif_condition = Box::new(self.parse_expression(Precedence::Lowest)?);
            let elif_body = Box::new(self.parse_block()?);
            elif_branches.push(ElifBranch {
                condition: elif_condition,
                body: elif_body,
            });
        }

        if self.matches(Token::Else) {
            else_branch = Some(Box::new(self.parse_block()?));
        }

        Ok(Expr::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            span,
        })
    }

    /// 解析 match 表达式
    fn parse_match(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        self.advance(); // 消耗 'match'

        let expr = Box::new(self.parse_expression(Precedence::Lowest)?);
        self.consume(Token::LBrace, "Expected '{'")?;

        let mut arms = Vec::new();
        while !self.matches(Token::RBrace) {
            let pattern = self.parse_pattern()?;
            self.consume(Token::Arrow, "Expected '=>'")?;
            let arm_body = self.parse_expression(Precedence::Lowest)?;
            self.consume(Token::Comma, "Expected ','")?;

            arms.push(MatchArm {
                pattern,
                body: arm_body,
            });
        }

        Ok(Expr::Match { expr, arms, span })
    }
}
```

### 2.3 类型解析器

**核心文件**：`src/frontend/parser/type_parser.rs`

```rust
impl Parser {
    /// 解析类型
    pub fn parse_type(&mut self) -> Result<Type, ParseError> {
        match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();

                // 检查泛型参数
                if self.matches(Token::LAngle) {
                    let mut args = Vec::new();
                    while !self.matches(Token::RAngle) {
                        if !args.is_empty() {
                            self.consume(Token::Comma, "Expected ','")?;
                        }
                        args.push(self.parse_type()?);
                    }
                    Ok(Type::Generic { name, args })
                } else if self.peek() == Token::LBrace {
                    // 结构体类型
                    self.parse_struct_type(&name)
                } else {
                    Ok(Type::Name(name))
                }
            }
            Token::LParen => {
                // 元组类型
                self.advance();
                let mut types = Vec::new();
                while !self.matches(Token::RParen) {
                    if !types.is_empty() {
                        self.consume(Token::Comma, "Expected ','")?;
                    }
                    types.push(self.parse_type()?);
                }
                Ok(Type::Tuple(types))
            }
            Token::LBracket => {
                // 列表类型
                self.advance();
                let elem_type = self.parse_type()?;
                self.consume(Token::RBracket, "Expected ']'")?;
                Ok(Type::List(Box::new(elem_type)))
            }
            Token::Fn => self.parse_fn_type(),
            _ => Err(ParseError::ExpectedType),
        }
    }

    /// 解析函数类型
    fn parse_fn_type(&mut self) -> Result<Type, ParseError> {
        let span = self.span();
        self.advance(); // 消耗 'fn'

        self.consume(Token::LParen, "Expected '('")?;
        let mut params = Vec::new();
        while !self.matches(Token::RParen) {
            if !params.is_empty() {
                self.consume(Token::Comma, "Expected ','")?;
            }
            params.push(self.parse_type()?);
        }

        let return_type = if self.matches(Token::Arrow) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        Ok(Type::Fn {
            params,
            return_type,
            is_async: false,
        })
    }
}
```

### 2.4 AST 定义

**核心文件**：`src/frontend/parser/ast.rs`

```rust
// 模块
#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}

// 模块项
#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    TypeDef(TypeDef),
    Stmt(Stmt),
}

// 函数
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Expr,
    pub span: Span,
}

// 参数
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

// 类型定义
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

// 枚举变体
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Type>,
    pub span: Span,
}

// 表达式
#[derive(Debug, Clone)]
pub enum Expr {
    // 字面量
    Lit(Literal, Span),

    // 变量
    Var(String, Span),

    // 二元运算
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },

    // 一元运算
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
        span: Span,
    },

    // 函数调用
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },

    // 条件表达式
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        elif_branches: Vec<ElifBranch>,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },

    // 模式匹配
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    // 循环
    While {
        condition: Box<Expr>,
        body: Box<Expr>,
        label: Option<String>,
        span: Span,
    },

    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Expr>,
        label: Option<String>,
        span: Span,
    },

    // 块表达式
    Block(Block),

    // 返回
    Return(Option<Box<Expr>>, Span),

    // 中断
    Break(Option<String>, Span),
    Continue(Option<String>, Span),

    // 类型转换
    Cast {
        expr: Box<Expr>,
        target_type: Type,
        span: Span,
    },

    // 复合类型
    Tuple(Vec<Expr>, Span),
    List(Vec<Expr>, Span),
    Dict(Vec<(Expr, Expr)>, Span),

    // 索引访问
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },

    // 字段访问
    FieldAccess {
        expr: Box<Expr>,
        field: String,
        span: Span,
    },
}

// 语句
#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
    },
    Expr(Expr),
    // ... 其他语句
}

// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Negate,  // -
    Not,     // !
    Ref,     // &
    Deref,   // *
}

// 匹配臂
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

// 模式
#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Literal),
    Identifier(String),
    Tuple(Vec<Pattern>),
    List(Vec<Pattern>),
    Dict(Vec<(Pattern, Pattern)>),
    Wildcard,
}

// 块
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>,
}
```

---

## 三、中端设计

### 3.1 中间表示 (IR)

**核心文件**：`src/middle/ir.rs`

#### 3.1.1 IR 设计原则

- **SSA 形式**：每个变量只赋值一次
- **控制流图**：基本块和跳转指令
- **类型信息**：所有操作都有明确的类型
- **平台无关**：不依赖特定架构

#### 3.1.2 IR 定义

```rust
// IR 模块
#[derive(Debug, Clone)]
pub struct ModuleIR {
    pub types: Vec<Type>,
    pub constants: Vec<ConstValue>,
    pub globals: Vec<GlobalIR>,
    pub functions: Vec<FunctionIR>,
}

// 全局变量
#[derive(Debug, Clone)]
pub struct GlobalIR {
    pub name: String,
    pub ty: MonoType,
    pub init: Option<ConstValue>,
}

// 函数 IR
#[derive(Debug, Clone)]
pub struct FunctionIR {
    pub name: String,
    pub params: Vec<(String, MonoType)>,
    pub return_type: MonoType,
    pub locals: Vec<LocalIR>,
    pub blocks: Vec<BasicBlock>,
}

// 局部变量
#[derive(Debug, Clone)]
pub struct LocalIR {
    pub name: String,
    pub ty: MonoType,
}

// 基本块
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: u32,
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
}

// 操作数
#[derive(Debug, Clone)]
pub enum Operand {
    Local(usize),    // 局部变量
    Temp(usize),     // 临时寄存器
    Arg(usize),      // 参数
    Const(ConstValue), // 常量
    Global(usize),   // 全局变量
}

// IR 指令
#[derive(Debug, Clone)]
pub enum Instruction {
    // 移动和加载
    Move { dst: Operand, src: Operand },
    Load { dst: Operand, src: Operand },
    Store { dst: Operand, src: Operand },

    // 算术运算
    Add { dst: Operand, lhs: Operand, rhs: Operand },
    Sub { dst: Operand, lhs: Operand, rhs: Operand },
    Mul { dst: Operand, lhs: Operand, rhs: Operand },
    Div { dst: Operand, lhs: Operand, rhs: Operand },
    Mod { dst: Operand, lhs: Operand, rhs: Operand },
    Neg { dst: Operand, src: Operand },

    // 比较
    Eq { dst: Operand, lhs: Operand, rhs: Operand },
    Ne { dst: Operand, lhs: Operand, rhs: Operand },
    Lt { dst: Operand, lhs: Operand, rhs: Operand },
    Le { dst: Operand, lhs: Operand, rhs: Operand },
    Gt { dst: Operand, lhs: Operand, rhs: Operand },
    Ge { dst: Operand, lhs: Operand, rhs: Operand },

    // 控制流
    Jmp(u32),
    JmpIf(Operand, u32),
    JmpIfNot(Operand, u32),
    Ret(Option<Operand>),

    // 函数调用
    Call { dst: Option<Operand>, func: Operand, args: Vec<Operand> },
    TailCall { func: Operand, args: Vec<Operand> },

    // 内存操作
    Alloc { dst: Operand, size: Operand },
    Free(Operand),
    HeapAlloc { dst: Operand, type_id: u32 },

    // 字段操作
    LoadField { dst: Operand, src: Operand, field: u32 },
    StoreField { dst: Operand, field: u32, src: Operand },

    // 索引操作
    LoadIndex { dst: Operand, src: Operand, index: Operand },
    StoreIndex { dst: Operand, index: Operand, src: Operand },

    // 类型操作
    Cast { dst: Operand, src: Operand, target_type: u32 },
    TypeTest(Operand, u32),

    // 并发
    Spawn { func: Operand },
    Await(Operand),
    Yield,

    // 闭包
    MakeClosure { dst: Operand, func: Operand, env: Vec<Operand> },

    // 栈操作
    Push(Operand),
    Pop(Operand),
    Dup,
    Swap,
}

// 常量值
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Char(char),
    String(String),
    Unit,
    Tuple(Vec<ConstValue>),
    List(Vec<ConstValue>),
}
```

### 3.2 优化器

**核心文件**：`src/middle/optimizer.rs`

```rust
/// 优化器
pub struct Optimizer {
    // 优化配置
    config: OptimizerConfig,
}

#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    pub enable_constant_folding: bool,
    pub enable_dce: bool,             // 死代码消除
    pub enable_cse: bool,             // 公共子表达式消除
    pub enable_algebraic_simplify: bool, // 代数简化
}

impl Optimizer {
    pub fn new(config: Option<OptimizerConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
        }
    }

    /// 优化 IR 模块
    pub fn optimize(&self, module: &mut ModuleIR) {
        for func in module.functions.iter_mut() {
            self.optimize_function(func);
        }
    }

    fn optimize_function(&self, func: &mut FunctionIR) {
        for block in func.blocks.iter_mut() {
            if self.config.enable_constant_folding {
                self.constant_folding(block);
            }
            if self.config.enable_dce {
                self.dead_code_elimination(block);
            }
            if self.config.enable_algebraic_simplify {
                self.algebraic_simplify(block);
            }
        }
    }

    /// 常量折叠
    fn constant_folding(&self, block: &mut BasicBlock) {
        let mut changed = true;
        while changed {
            changed = false;
            for instr in block.instructions.iter_mut() {
                if let Some(new_instr) = self.fold_instruction(instr) {
                    *instr = new_instr;
                    changed = true;
                }
            }
        }
    }

    fn fold_instruction(&self, instr: &Instruction) -> Option<Instruction> {
        match instr {
            Instruction::Add { dst, lhs, rhs } => {
                if let (Operand::Const(ConstValue::Int(l)),
                        Operand::Const(ConstValue::Int(r))) = (lhs, rhs) {
                    return Some(Instruction::Load {
                        dst: *dst,
                        src: Operand::Const(ConstValue::Int(l + r)),
                    });
                }
                None
            }
            // ... 其他折叠规则
            _ => None,
        }
    }

    /// 死代码消除
    fn dead_code_elimination(&self, block: &mut BasicBlock) {
        // 收集活跃变量
        let mut live_vars = std::collections::HashSet::new();
        // 从后向前扫描，标记活跃变量
        for instr in block.instructions.iter().rev() {
            self.update_live_vars(instr, &mut live_vars);
        }

        // 移除不影响活跃变量的 Store
        block.instructions.retain(|instr| {
            !matches!(instr, Instruction::Store { .. }) ||
            self.is_store_needed(instr, &live_vars)
        });
    }

    /// 代数简化
    fn algebraic_simplify(&self, block: &mut BasicBlock) {
        for instr in block.instructions.iter_mut() {
            match instr {
                Instruction::Mul { dst, lhs, rhs } => {
                    // x * 1 -> x
                    if self.is_one(lhs) {
                        *instr = Instruction::Move { dst: *dst, src: *rhs };
                    } else if self.is_one(rhs) {
                        *instr = Instruction::Move { dst: *dst, src: *lhs };
                    }
                    // x * 0 -> 0
                    else if self.is_zero(lhs) || self.is_zero(rhs) {
                        *instr = Instruction::Load {
                            dst: *dst,
                            src: Operand::Const(ConstValue::Int(0)),
                        };
                    }
                }
                // ... 其他简化规则
                _ => {}
            }
        }
    }
}
```

### 3.3 泛型单态化

**核心文件**：`src/middle/monomorphize/mod.rs`, `src/middle/monomorphize/instance.rs`

```rust
/// 单态化器
pub struct Monomorphizer {
    // 实例缓存：(泛型函数ID, 类型参数列表) -> 具体函数ID
    instances: HashMap<(usize, Vec<MonoType>), usize>,
    // 待处理的泛型函数调用
    pending_calls: Vec<(usize, usize, Vec<MonoType>)>, // (调用位置, 函数ID, 类型参数)
}

impl Monomorphizer {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            pending_calls: Vec::new(),
        }
    }

    /// 单态化模块
    pub fn monomorphize(&mut self, module: &mut ModuleIR) {
        // 1. 收集所有泛型函数调用
        self.collect_generic_calls(module);

        // 2. 生成单态化实例
        while let Some((_call_site, generic_id, type_args)) = self.pending_calls.pop() {
            self.get_or_create_instance(module, generic_id, &type_args);
        }
    }

    fn collect_generic_calls(&mut self, module: &ModuleIR) {
        for (func_idx, func) in module.functions.iter().enumerate() {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if let Instruction::Call { func: callee, args, .. } = instr {
                        // 检查被调用函数是否是泛型
                        if let Operand::Const(ConstValue::Int(fid)) = callee {
                            let callee_func = &module.functions[*fid as usize];
                            if !callee_func.type_params.is_empty() {
                                // 推断类型参数
                                let type_args = self.infer_type_args(module, callee, args);
                                self.pending_calls.push((func_idx, *fid as usize, type_args));
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_or_create_instance(
        &mut self,
        module: &mut ModuleIR,
        generic_id: usize,
        type_args: &[MonoType],
    ) -> usize {
        let key = (generic_id, type_args.to_vec());

        if let Some(&instance_id) = self.instances.get(&key) {
            return instance_id;
        }

        // 创建新实例
        let generic = module.functions[generic_id].clone();
        let instance = self.instantiate(&generic, type_args);

        let instance_id = module.functions.len();
        module.functions.push(instance);
        self.instances.insert(key, instance_id);

        instance_id
    }

    fn instantiate(&self, generic: &FunctionIR, type_args: &[MonoType]) -> FunctionIR {
        // 创建类型替换映射
        let mut subst = HashMap::new();
        for (param, arg) in generic.type_params.iter().zip(type_args.iter()) {
            subst.insert(param.name.clone(), arg.clone());
        }

        // 替换函数签名
        let params = generic.params.iter()
            .map(|(name, _)| {
                let new_name = self.fresh_name();
                (new_name, subst.get(name).cloned().unwrap_or(MonoType::Unit))
            })
            .collect();

        let return_type = subst.get("Return")
            .cloned()
            .unwrap_or(generic.return_type.clone());

        // 替换指令中的类型
        let mut new_blocks = Vec::new();
        for mut block in generic.blocks.clone() {
            for instr in block.instructions.iter_mut() {
                self.substitute_types(instr, &subst);
            }
            new_blocks.push(block);
        }

        FunctionIR {
            name: format!("{}_{}", generic.name, self.monomorphized_name(type_args)),
            params,
            return_type,
            locals: generic.locals.clone(),
            blocks: new_blocks,
        }
    }

    fn monomorphized_name(&self, type_args: &[MonoType]) -> String {
        type_args.iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join("_")
    }
}
```

### 3.4 逃逸分析

**核心文件**：`src/middle/escape_analysis/mod.rs`

```rust
/// 逃逸分析结果
#[derive(Debug, Clone)]
pub struct EscapeAnalysisResult {
    // 每个局部变量的分配位置
    pub allocations: HashMap<usize, AllocationSite>,
    // 需要保存到堆的变量
    pub heap_vars: HashSet<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationSite {
    Stack,    // 栈分配
    Heap,     // 堆分配
    Register, // 寄存器（不分配）
}

/// 逃逸分析器
pub struct EscapeAnalyzer {
    // 函数图
    func_graph: FuncGraph,
    // 调用图
    call_graph: CallGraph,
}

impl EscapeAnalyzer {
    pub fn analyze(&self, module: &ModuleIR) -> EscapeAnalysisResult {
        let mut result = EscapeAnalysisResult {
            allocations: HashMap::new(),
            heap_vars: HashSet::new(),
        };

        // 分析每个函数
        for func in &module.functions {
            self.analyze_function(func, module, &mut result);
        }

        result
    }

    fn analyze_function(
        &self,
        func: &FunctionIR,
        module: &ModuleIR,
        result: &mut EscapeAnalysisResult,
    ) {
        for (local_idx, local) in func.locals.iter().enumerate() {
            // 默认假设栈分配
            result.allocations.insert(local_idx, AllocationSite::Stack);

            // 分析局部变量的使用
            for block in &func.blocks {
                for instr in &block.instructions {
                    self.analyze_instr(local_idx, instr, func, module, result);
                }
            }
        }
    }

    fn analyze_instr(
        &self,
        local_idx: usize,
        instr: &Instruction,
        func: &FunctionIR,
        module: &ModuleIR,
        result: &mut EscapeAnalysisResult,
    ) {
        match instr {
            // 函数调用可能导致逃逸
            Instruction::Call { args, .. } => {
                for arg in args {
                    if let Operand::Local(id) = arg {
                        if *id == local_idx {
                            // 检查被调用函数是否存储参数
                            // 如果是，可能逃逸到堆
                            result.allocations.insert(local_idx, AllocationSite::Heap);
                            result.heap_vars.insert(local_idx);
                        }
                    }
                }
            }

            // 返回值可能携带局部变量
            Instruction::Ret(Some(val)) => {
                if let Operand::Local(id) = val {
                    if *id == local_idx {
                        // 返回值逃逸
                        result.allocations.insert(local_idx, AllocationSite::Heap);
                        result.heap_vars.insert(local_idx);
                    }
                }
            }

            // 存储到全局变量逃逸
            Instruction::Store { dst: Operand::Global(_), src } => {
                if let Operand::Local(id) = src {
                    if *id == local_idx {
                        result.allocations.insert(local_idx, AllocationSite::Heap);
                        result.heap_vars.insert(local_idx);
                    }
                }
            }

            // 字段存储可能导致逃逸
            Instruction::StoreField { dst, .. } => {
                if let Operand::Local(id) = dst {
                    if *id == local_idx {
                        result.allocations.insert(local_idx, AllocationSite::Heap);
                        result.heap_vars.insert(local_idx);
                    }
                }
            }

            _ => {}
        }
    }
}
```

### 3.5 生命周期分析

**核心文件**：`src/middle/lifetime/mod.rs`

```rust
/// 生命周期分析器
pub struct LifetimeAnalyzer {
    // 变量生命周期
    lifetimes: HashMap<usize, Lifetime>,
    // 借用关系图
    borrow_graph: BorrowGraph,
}

#[derive(Debug, Clone)]
pub struct Lifetime {
    pub start: usize,  // 开始指令索引
    pub end: usize,    // 结束指令索引
    pub kind: LifetimeKind,
}

#[derive(Debug, Clone)]
pub enum LifetimeKind {
    /// 完整所有权
    Owned,
    /// 借用（不可变）
    Borrowed,
    /// 可变借用
    BorrowedMut,
    /// 生命周期参数
    Parameter(String),
}

impl LifetimeAnalyzer {
    pub fn analyze(&self, func: &FunctionIR) -> LifetimeAnalysisResult {
        let mut analysis = LifetimeAnalysisResult {
            lifetimes: HashMap::new(),
            borrow_constraints: Vec::new(),
            errors: Vec::new(),
        };

        // 1. 计算每个变量的活跃范围
        self.compute_lifetimes(func, &mut analysis);

        // 2. 检查借用规则
        self.check_borrow_rules(func, &mut analysis);

        analysis
    }

    fn compute_lifetimes(&self, func: &FunctionIR, analysis: &mut LifetimeAnalysisResult) {
        // 使用活跃变量分析确定生命周期
        let mut live_in = HashMap::new();
        let mut live_out = HashMap::new();

        // 初始化所有变量为活跃
        for (idx, _) in func.locals.iter().enumerate() {
            live_in.insert(idx, false);
            live_out.insert(idx, false);
        }

        // 迭代直到固定点
        let mut changed = true;
        while changed {
            changed = false;
            for (block_idx, block) in func.blocks.iter().enumerate() {
                for (instr_idx, instr) in block.instructions.iter().enumerate() {
                    let global_idx = self.instr_global_index(block_idx, instr_idx);

                    // 计算 live_out
                    let mut out_live = HashSet::new();
                    if let Some(term) = &block.terminator {
                        self.add_term_live_vars(term, &mut out_live);
                    }

                    // 计算 live_in
                    let mut in_live = out_live.clone();
                    self.add_instr_live_vars(instr, &mut in_live);

                    // 更新
                    if live_in.insert(global_idx, in_live).is_some_and(|old| old != in_live) {
                        changed = true;
                    }
                }
            }
        }
    }
}
```

---

## 四、类型系统实现

### 4.1 类型定义

**核心文件**：`src/frontend/typecheck/types.rs`

```rust
/// 类型变量（用于类型推断）
///
/// 每个类型变量有一个唯一的索引，用于在类型环境中追踪
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(usize);

impl TypeVar {
    pub fn new(index: usize) -> Self {
        TypeVar(index)
    }

    pub fn index(&self) -> usize {
        self.0
    }
}

/// 类型绑定的状态（union-find 结构）
///
/// 使用 union-find 模式实现类型变量的绑定和查找
#[derive(Debug, Clone)]
pub enum TypeBinding {
    /// 未绑定，可接受任何类型
    Unbound,
    /// 已绑定到具体类型
    Bound(MonoType),
    /// 链接到另一个类型变量（用于路径压缩）
    Link(TypeVar),
}

/// 单态类型（具体类型）
///
/// 不包含类型变量的具体类型，用于类型检查的最终结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonoType {
    /// 空类型
    Void,
    /// 布尔类型
    Bool,
    /// 整数类型（宽度）
    Int(usize),
    /// 浮点类型（宽度）
    Float(usize),
    /// 字符类型
    Char,
    /// 字符串类型
    String,
    /// 字节数组
    Bytes,
    /// 结构体类型
    Struct(StructType),
    /// 枚举类型
    Enum(EnumType),
    /// 元组类型
    Tuple(Vec<MonoType>),
    /// 列表类型
    List(Box<MonoType>),
    /// 字典类型
    Dict(Box<MonoType>, Box<MonoType>),
    /// 集合类型
    Set(Box<MonoType>),
    /// 函数类型
    Fn {
        /// 参数类型列表
        params: Vec<MonoType>,
        /// 返回类型
        return_type: Box<MonoType>,
        /// 是否异步
        is_async: bool,
    },
    /// 类型变量（推断中）
    TypeVar(TypeVar),
    /// 类型引用（如自定义类型名）
    TypeRef(String),
}

/// 结构体类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: MonoType,
}

/// 枚举类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<MonoType>,
}

/// 多态类型（带泛型参数）
#[derive(Debug, Clone)]
pub struct PolyType {
    /// 类型参数
    pub type_params: Vec<String>,
    /// 类型主体
    pub body: MonoType,
}

/// 类型约束求解器
#[derive(Debug)]
pub struct TypeConstraintSolver {
    /// 类型变量绑定（union-find）
    bindings: Vec<TypeBinding>,
    /// 类型变量计数器
    next_var: usize,
    /// 类型上下文
    context: TypeContext,
}

/// 类型上下文
#[derive(Debug, Clone)]
pub struct TypeContext {
    /// 变量类型
    pub vars: HashMap<String, PolyType>,
    /// 结构体定义
    pub structs: HashMap<String, StructType>,
    /// 枚举定义
    pub enums: HashMap<String, EnumType>,
    /// 类型参数（用于泛型函数）
    pub type_params: Vec<String>,
}

impl MonoType {
    /// 检查是否是数值类型
    pub fn is_numeric(&self) -> bool {
        matches!(self, MonoType::Int(_) | MonoType::Float(_))
    }

    /// 检查是否是可索引类型
    pub fn is_indexable(&self) -> bool {
        matches!(
            self,
            MonoType::List(_) | MonoType::Dict(_, _) | MonoType::String | MonoType::Tuple(_)
        )
    }

    /// 获取类型的字符串描述
    pub fn type_name(&self) -> String {
        match self {
            MonoType::Void => "void".to_string(),
            MonoType::Bool => "bool".to_string(),
            MonoType::Int(n) => format!("int{}", n),
            MonoType::Float(n) => format!("float{}", n),
            MonoType::Char => "char".to_string(),
            MonoType::String => "string".to_string(),
            MonoType::Bytes => "bytes".to_string(),
            MonoType::Struct(s) => s.name.clone(),
            MonoType::Enum(e) => e.name.clone(),
            MonoType::Tuple(types) => {
                format!("({})", types.iter().map(|t| t.type_name()).collect::<Vec<_>>().join(", "))
            }
            MonoType::List(t) => format!("List<{}>", t.type_name()),
            MonoType::Dict(k, v) => format!("Dict<{}, {}>", k.type_name(), v.type_name()),
            MonoType::Set(t) => format!("Set<{}>", t.type_name()),
            MonoType::Fn { params, return_type, .. } => {
                let params_str = params.iter().map(|t| t.type_name()).collect::<Vec<_>>().join(", ");
                format!("fn({}) -> {}", params_str, return_type.type_name())
            }
            MonoType::TypeVar(v) => format!("t{}", v.index()),
            MonoType::TypeRef(name) => name.clone(),
        }
    }
}
```

### 4.2 类型推断

**核心文件**：`src/frontend/typecheck/infer.rs`

```rust
/// 类型推断器
///
/// 负责推断表达式的类型并收集类型约束
#[derive(Debug)]
pub struct TypeInferrer<'a> {
    /// 类型约束求解器
    solver: &'a mut TypeConstraintSolver,
    /// 变量环境栈：每一层是一个作用域
    scopes: Vec<HashMap<String, PolyType>>,
    /// 循环标签栈（用于 break/continue）
    loop_labels: Vec<String>,
    /// 返回类型栈：每一层对应一个函数，存储该函数中所有 return 语句的表达式类型
    return_types: Vec<Vec<MonoType>>,
}

impl<'a> TypeInferrer<'a> {
    /// 创建新的类型推断器
    pub fn new(solver: &'a mut TypeConstraintSolver) -> Self {
        TypeInferrer {
            solver,
            scopes: vec![HashMap::new()], // Global scope
            loop_labels: Vec::new(),
            return_types: Vec::new(),
        }
    }

    /// 推断表达式的类型
    pub fn infer_expr(&mut self, expr: &ast::Expr) -> TypeResult<MonoType> {
        match &expr {
            ast::Expr::Lit(lit, span) => self.infer_literal(lit, *span),
            ast::Expr::Var(name, span) => self.infer_var(name, *span),
            ast::Expr::BinOp { op, left, right, span } => self.infer_binop(op, left, right, *span),
            ast::Expr::UnOp { op, expr, span } => self.infer_unop(op, expr, *span),
            ast::Expr::Call { func, args, span } => self.infer_call(func, args, *span),
            ast::Expr::If { condition, then_branch, elif_branches, else_branch, span } =>
                self.infer_if(condition, then_branch, elif_branches, else_branch.as_deref(), *span),
            ast::Expr::Match { expr, arms, span } => self.infer_match(expr, arms, *span),
            ast::Expr::While { condition, body, .., span } => self.infer_while(condition, body, *span),
            ast::Expr::For { var, iterable, body, span } => self.infer_for(var, iterable, body, *span),
            ast::Expr::Block(block) => self.infer_block(block, true),
            ast::Expr::Return(expr, span) => self.infer_return(expr.as_deref(), *span),
            ast::Expr::Break(label, span) => self.infer_break(label.as_deref(), *span),
            ast::Expr::Continue(label, span) => self.infer_continue(label.as_deref(), *span),
            ast::Expr::Cast { expr, target_type, span } => self.infer_cast(expr, target_type, *span),
            ast::Expr::Tuple(exprs, span) => self.infer_tuple(exprs, *span),
            ast::Expr::List(exprs, span) => self.infer_list(exprs, *span),
            ast::Expr::Dict(pairs, span) => self.infer_dict(pairs, *span),
            ast::Expr::Index { expr, index, span } => self.infer_index(expr, index, *span),
            ast::Expr::FieldAccess { expr, field, span } => self.infer_field_access(expr, field, *span),
        }
    }

    /// 推断字面量的类型
    fn infer_literal(&mut self, lit: &Literal, span: Span) -> TypeResult<MonoType> {
        let ty = match lit {
            Literal::Int(_) => MonoType::Int(64),
            Literal::Float(_) => MonoType::Float(64),
            Literal::Bool(_) => MonoType::Bool,
            Literal::Char(_) => MonoType::Char,
            Literal::String(_) => MonoType::String,
        };
        Ok(ty)
    }

    /// 推断变量的类型
    fn infer_var(&mut self, name: &str, span: Span) -> TypeResult<MonoType> {
        let poly = self.get_var(name).cloned();

        if let Some(poly) = poly {
            // 实例化多态类型
            let ty = self.solver.instantiate(&poly);
            Ok(ty)
        } else {
            Err(TypeError::UnknownVariable {
                name: name.to_string(),
                span,
            })
        }
    }

    /// 推断二元运算的类型
    fn infer_binop(&mut self, op: &BinOp, left: &ast::Expr, right: &ast::Expr, span: Span) -> TypeResult<MonoType> {
        let left_ty = self.infer_expr(left)?;
        let right_ty = self.infer_expr(right)?;

        match op {
            // 算术运算
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                // 检查操作数类型是否兼容
                self.solver.unify(&left_ty, &right_ty, span)?;

                // 返回数值类型
                if left_ty.is_numeric() {
                    Ok(left_ty)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: "numeric type".to_string(),
                        found: left_ty.type_name(),
                        span,
                    })
                }
            }

            // 比较运算
            BinOp::Eq | BinOp::Ne => {
                self.solver.unify(&left_ty, &right_ty, span)?;
                Ok(MonoType::Bool)
            }

            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.solver.unify(&left_ty, &right_ty, span)?;
                if left_ty.is_numeric() {
                    Ok(MonoType::Bool)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: "numeric or comparable type".to_string(),
                        found: left_ty.type_name(),
                        span,
                    })
                }
            }

            // 逻辑运算
            BinOp::And | BinOp::Or => {
                self.solver.unify(&left_ty, &MonoType::Bool, span)?;
                self.solver.unify(&right_ty, &MonoType::Bool, span)?;
                Ok(MonoType::Bool)
            }
        }
    }

    /// 推断函数调用类型
    fn infer_call(&mut self, func: &ast::Expr, args: &[ast::Expr], span: Span) -> TypeResult<MonoType> {
        let func_ty = self.infer_expr(func)?;

        match &func_ty {
            MonoType::Fn { params, return_type, .. } => {
                // 检查参数数量
                if params.len() != args.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: params.len(),
                        found: args.len(),
                        span,
                    });
                }

                // 推断参数类型并检查兼容性
                for (arg, expected_ty) in args.iter().zip(params.iter()) {
                    let arg_ty = self.infer_expr(arg)?;
                    self.solver.unify(&arg_ty, expected_ty, span)?;
                }

                Ok(*return_type.clone())
            }

            MonoType::TypeVar(var) => {
                // 创建新的类型变量作为返回类型
                let return_ty = MonoType::TypeVar(self.solver.fresh_var());

                // 为函数类型创建类型变量
                let param_tys: Vec<MonoType> = args.iter()
                    .map(|_| MonoType::TypeVar(self.solver.fresh_var()))
                    .collect();

                let func_ty = MonoType::Fn {
                    params: param_tys.clone(),
                    return_type: Box::new(return_ty.clone()),
                    is_async: false,
                };

                // 绑定函数类型
                self.solver.bind_typevar(var, &func_ty, span)?;

                // 约束参数类型
                for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                    let arg_ty = self.infer_expr(arg)?;
                    self.solver.unify(&arg_ty, param_ty, span)?;
                }

                Ok(return_ty)
            }

            _ => Err(TypeError::NotCallable {
                ty: func_ty.type_name(),
                span,
            })
        }
    }

    /// 推断 if 表达式类型
    fn infer_if(
        &mut self,
        condition: &ast::Expr,
        then_branch: &ast::Expr,
        elif_branches: &[ast::ElifBranch],
        else_branch: Option<&ast::Expr>,
        span: Span,
    ) -> TypeResult<MonoType> {
        // 条件必须是 Bool
        let cond_ty = self.infer_expr(condition)?;
        self.solver.unify(&cond_ty, &MonoType::Bool, span)?;

        // 推断 then 分支类型
        let then_ty = self.infer_expr(then_branch)?;

        // 处理 elif 分支
        let mut current_ty = then_ty;
        for elif in elif_branches {
            let elif_cond_ty = self.infer_expr(&elif.condition)?;
            self.solver.unify(&elif_cond_ty, &MonoType::Bool, span)?;
            let elif_ty = self.infer_expr(&elif.body)?;
            current_ty = self.solver.unify_types(&current_ty, &elif_ty, span)?;
        }

        // 处理 else 分支
        if let Some(else_expr) = else_branch {
            let else_ty = self.infer_expr(else_expr)?;
            current_ty = self.solver.unify_types(&current_ty, &else_ty, span)?;
        } else {
            // 如果没有 else 分支，类型必须是 Void
            self.solver.unify(&current_ty, &MonoType::Void, span)?;
        }

        Ok(current_ty)
    }

    /// 推断 match 表达式类型
    fn infer_match(&mut self, expr: &ast::Expr, arms: &[ast::MatchArm], span: Span) -> TypeResult<MonoType> {
        let expr_ty = self.infer_expr(expr)?;

        // 推断第一个分支的类型作为基础
        let first_arm_ty = self.infer_expr(&arms[0].body)?;

        // 检查所有分支类型是否兼容
        let mut result_ty = first_arm_ty;
        for arm in arms.iter() {
            // 检查模式是否匹配表达式类型
            self.infer_pattern(&arm.pattern, &expr_ty)?;
            let arm_ty = self.infer_expr(&arm.body)?;
            result_ty = self.solver.unify_types(&result_ty, &arm_ty, span)?;
        }

        Ok(result_ty)
    }

    /// 推断函数定义类型
    pub fn infer_fn_def(&mut self, func: &ast::Function) -> TypeResult<MonoType> {
        // 创建新的类型变量作为返回类型
        let return_ty = self.solver.fresh_var();

        // 创建新的作用域
        self.scopes.push(HashMap::new());

        // 为参数创建类型变量并添加到作用域
        let param_types: Vec<MonoType> = func.params.iter().map(|p| {
            let ty = if let Some(ty) = &p.ty {
                self.solver.type_from_ast(ty)
            } else {
                MonoType::TypeVar(self.solver.fresh_var())
            };
            self.scopes.last_mut().unwrap().insert(
                p.name.clone(),
                PolyType {
                    type_params: vec![],
                    body: ty.clone(),
                },
            );
            ty
        }).collect();

        // 进入函数作用域（收集 return 类型）
        self.enter_function();

        // 推断函数体类型
        let body_ty = self.infer_expr(&func.body)?;

        // 获取所有 return 语句的类型
        let return_type = if let Some(expected) = &func.return_type {
            let expected_ty = self.solver.type_from_ast(expected);
            self.solver.unify(&body_ty, &expected_ty, func.span)?;
            expected_ty
        } else {
            // 如果没有指定返回类型，使用推断的返回类型
            let ret_types = self.exit_function();
            if ret_types.is_empty() {
                MonoType::Void
            } else {
                // 统一所有 return 类型
                let mut unified = ret_types[0].clone();
                for ty in ret_types.iter().skip(1) {
                    unified = self.solver.unify_types(&unified, ty, func.span)?;
                }
                unified
            }
        };

        // 恢复作用域
        self.scopes.pop();

        // 创建函数类型
        Ok(MonoType::Fn {
            params: param_types,
            return_type: Box::new(return_ty),
            is_async: false,
        })
    }
}
```

### 4.3 类型约束求解

```rust
impl TypeConstraintSolver {
    /// 创建新的类型变量
    pub fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        self.bindings.push(TypeBinding::Unbound);
        var
    }

    /// 绑定类型变量
    pub fn bind_typevar(&mut self, var: &TypeVar, ty: &MonoType, span: Span) -> TypeResult<()> {
        // 循环检查
        if self.occurs_in(var, ty) {
            return Err(TypeError::InfiniteType {
                var: var.index(),
                span,
            });
        }

        self.bindings[var.index()] = TypeBinding::Bound(ty.clone());
        Ok(())
    }

    /// 合一两个类型
    pub fn unify(&mut self, t1: &MonoType, t2: &MonoType, span: Span) -> TypeResult<()> {
        // 解析类型变量
        let t1 = self.resolve(t1);
        let t2 = self.resolve(t2);

        match (t1, t2) {
            // 相同类型
            (a, b) if a == b => Ok(()),

            // 类型变量
            (MonoType::TypeVar(v), ty) => self.bind_typevar(&v, &ty, span),
            (ty, MonoType::TypeVar(v)) => self.bind_typevar(&v, &ty, span),

            // 函数类型
            (MonoType::Fn { params: p1, return_type: r1, .. },
             MonoType::Fn { params: p2, return_type: r2, .. }) => {
                if p1.len() != p2.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: p1.len(),
                        found: p2.len(),
                        span,
                    });
                }

                // 统一参数类型
                for (a, b) in p1.iter().zip(p2.iter()) {
                    self.unify(a, b, span)?;
                }

                // 统一返回类型
                self.unify(r1.as_ref(), r2.as_ref(), span)
            }

            // 列表类型
            (MonoType::List(t1), MonoType::List(t2)) => {
                self.unify(t1.as_ref(), t2.as_ref(), span)
            }

            // 字典类型
            (MonoType::Dict(k1, v1), MonoType::Dict(k2, v2)) => {
                self.unify(k1.as_ref(), k2.as_ref(), span)?;
                self.unify(v1.as_ref(), v2.as_ref(), span)
            }

            // 元组类型
            (MonoType::Tuple(t1), MonoType::Tuple(t2)) if t1.len() == t2.len() => {
                for (a, b) in t1.iter().zip(t2.iter()) {
                    self.unify(a, b, span)?;
                }
                Ok(())
            }

            // 类型引用
            (MonoType::TypeRef(n1), MonoType::TypeRef(n2)) if n1 == n2 => Ok(()),

            // 结构体类型
            (MonoType::Struct(s1), MonoType::Struct(s2)) if s1.name == s2.name => {
                if s1.fields.len() != s2.fields.len() {
                    return Err(TypeError::StructFieldMismatch {
                        struct_name: s1.name,
                        expected: s1.fields.len(),
                        found: s2.fields.len(),
                        span,
                    });
                }

                for (f1, f2) in s1.fields.iter().zip(s2.fields.iter()) {
                    self.unify(&f1.ty, &f2.ty, span)?;
                }
                Ok(())
            }

            _ => Err(TypeError::TypeMismatch {
                expected: t1.type_name(),
                found: t2.type_name(),
                span,
            })
        }
    }

    /// 解析类型（跟随类型变量链接）
    fn resolve(&self, ty: &MonoType) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => {
                match self.bindings[v.index()] {
                    TypeBinding::Bound(ref bound) => self.resolve(bound),
                    TypeBinding::Link(v2) => self.resolve(&MonoType::TypeVar(v2)),
                    TypeBinding::Unbound => ty.clone(),
                }
            }
            _ => ty.clone(),
        }
    }

    /// 循环检查
    fn occurs_in(&self, var: &TypeVar, ty: &MonoType) -> bool {
        match ty {
            MonoType::TypeVar(v) => {
                if *v == *var {
                    true
                } else {
                    match self.bindings[v.index()] {
                        TypeBinding::Bound(ref bound) => self.occurs_in(var, bound),
                        TypeBinding::Link(v2) => self.occurs_in(var, &MonoType::TypeVar(v2)),
                        TypeBinding::Unbound => false,
                    }
                }
            }
            MonoType::Fn { params, return_type, .. } => {
                params.iter().any(|p| self.occurs_in(var, p)) ||
                self.occurs_in(var, return_type)
            }
            MonoType::List(t) => self.occurs_in(var, t),
            MonoType::Dict(k, v) => self.occurs_in(var, k) || self.occurs_in(var, v),
            MonoType::Tuple(ts) => ts.iter().any(|t| self.occurs_in(var, t)),
            _ => false,
        }
    }

    /// 从 AST 类型转换
    pub fn type_from_ast(&self, ast_type: &Type) -> MonoType {
        match ast_type {
            Type::Name(name) => MonoType::TypeRef(name.clone()),
            Type::Int(n) => MonoType::Int(*n),
            Type::Float(n) => MonoType::Float(*n),
            Type::Char => MonoType::Char,
            Type::String => MonoType::String,
            Type::Bool => MonoType::Bool,
            Type::Void => MonoType::Void,
            Type::List(elem) => MonoType::List(Box::new(self.type_from_ast(elem))),
            Type::Dict(key, value) => MonoType::Dict(
                Box::new(self.type_from_ast(key)),
                Box::new(self.type_from_ast(value)),
            ),
            Type::Tuple(types) => MonoType::Tuple(
                types.iter().map(|t| self.type_from_ast(t)).collect(),
            ),
            Type::Fn { params, return_type, is_async } => MonoType::Fn {
                params: params.iter().map(|t| self.type_from_ast(t)).collect(),
                return_type: Box::new(self.type_from_ast(return_type)),
                is_async: *is_async,
            },
            Type::Generic { name, args } => {
                let mono_args = args.iter().map(|t| self.type_from_ast(t)).collect();
                MonoType::TypeRef(format!("{}<{}>", name, mono_args.iter().map(|t| t.type_name()).collect::<Vec<_>>().join(", ")))
            }
        }
    }
}
```

---

## 五、代码生成器设计

### 5.1 字节码格式

**核心文件**：`src/middle/codegen/bytecode.rs`

```rust
/// 字节码文件格式
#[derive(Debug, Clone)]
pub struct BytecodeFile {
    pub header: BytecodeHeader,
    pub type_table: Vec<MonoType>,
    pub const_pool: Vec<ConstValue>,
    pub code_section: CodeSection,
}

/// 文件头
#[derive(Debug, Clone)]
pub struct BytecodeHeader {
    /// 魔数：0x59584243 ("YXBC")
    pub magic: u32,
    /// 字节码版本
    pub version: u32,
    /// 标志位
    pub flags: u32,
    /// 入口点函数索引
    pub entry_point: u32,
    /// 区块数量
    pub section_count: u32,
    /// 文件大小
    pub file_size: u32,
    /// 校验和
    pub checksum: u32,
}

/// 代码段
#[derive(Debug, Clone)]
pub struct CodeSection {
    pub functions: Vec<FunctionCode>,
}

/// 函数代码
#[derive(Debug, Clone)]
pub struct FunctionCode {
    pub name: String,
    pub params: Vec<(String, MonoType)>,
    pub return_type: MonoType,
    pub local_count: usize,
    pub instructions: Vec<BytecodeInstruction>,
}

/// 字节码指令
#[derive(Debug, Clone)]
pub struct BytecodeInstruction {
    pub opcode: TypedOpcode,
    pub operands: Vec<u8>,
}

impl BytecodeInstruction {
    pub fn new(opcode: TypedOpcode, operands: Vec<u8>) -> Self {
        Self { opcode, operands }
    }

    /// 编码指令
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = vec![self.opcode as u8];
        bytes.extend(&self.operands);
        bytes
    }

    /// 获取操作数作为 u8
    pub fn operand_u8(&self, index: usize) -> u8 {
        self.operands[index]
    }

    /// 获取操作数作为 u16
    pub fn operand_u16(&self, index: usize) -> u16 {
        u16::from_le_bytes([self.operands[index], self.operands[index + 1]])
    }

    /// 获取操作数作为 u32
    pub fn operand_u32(&self, index: usize) -> u32 {
        u32::from_le_bytes([
            self.operands[index],
            self.operands[index + 1],
            self.operands[index + 2],
            self.operands[index + 3],
        ])
    }
}

/// 类型化操作码
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypedOpcode {
    // 特殊操作
    Nop = 0x00,
    Mov = 0x01,
    Drop = 0x02,

    // 整数运算 (64位)
    I64Add = 0x10,
    I64Sub = 0x11,
    I64Mul = 0x12,
    I64Div = 0x13,
    I64Rem = 0x14,
    I64Neg = 0x15,

    // 比较运算 (64位)
    I64Eq = 0x20,
    I64Ne = 0x21,
    I64Lt = 0x22,
    I64Le = 0x23,
    I64Gt = 0x24,
    I64Ge = 0x25,

    // 内存操作
    StackAlloc = 0x30,
    HeapAlloc = 0x31,
    LoadConst = 0x32,
    LoadElement = 0x33,
    StoreElement = 0x34,
    GetField = 0x35,
    SetField = 0x36,

    // 控制流
    Jmp = 0x40,
    JmpIf = 0x41,
    JmpIfNot = 0x42,
    Return = 0x43,
    ReturnValue = 0x44,

    // 函数调用
    Call = 0x50,
    TailCall = 0x51,

    // 类型操作
    Cast = 0x60,
    TypeCheck = 0x61,

    // 闭包
    MakeClosure = 0x70,
    LoadEnv = 0x71,

    // 异步操作
    Spawn = 0x80,
    Yield = 0x81,
    Await = 0x82,
}
```

### 5.2 代码生成器

**核心文件**：`src/middle/codegen/mod.rs`

```rust
/// 代码生成器
///
/// 将中间表示（IR）转换为类型化字节码。
/// 核心设计原则：
/// 1. 类型化指令：每条指令携带明确的类型信息
/// 2. 寄存器架构：所有操作在寄存器上进行
/// 3. 单态化输出：泛型已在编译期展开
pub struct CodegenContext {
    /// 当前模块
    module: ModuleIR,

    /// 符号表
    symbol_table: SymbolTable,

    /// 常量池
    constant_pool: ConstantPool,

    /// 字节码缓冲区
    bytecode: Vec<u8>,

    /// 当前函数
    current_function: Option<FunctionIR>,

    /// 寄存器分配器
    register_allocator: RegisterAllocator,

    /// 标签生成器
    label_generator: LabelGenerator,

    /// 逃逸分析结果
    escape_analysis: Option<EscapeAnalysisResult>,

    /// 字节码偏移追踪
    code_offsets: HashMap<usize, usize>,

    /// 跳转表
    jump_tables: HashMap<u16, JumpTable>,

    /// 函数索引
    function_indices: HashMap<String, usize>,

    /// 配置
    config: CodegenConfig,

    /// 当前作用域级别
    scope_level: usize,

    /// 当前循环标签 (loop_label, end_label)
    current_loop_label: Option<(usize, usize)>,
}

impl CodegenContext {
    /// 创建新的代码生成上下文
    pub fn new(module: ModuleIR) -> Self {
        let mut ctx = CodegenContext {
            module,
            symbol_table: SymbolTable::new(),
            constant_pool: ConstantPool::new(),
            bytecode: Vec::new(),
            current_function: None,
            register_allocator: RegisterAllocator::new(),
            label_generator: LabelGenerator::new(),
            escape_analysis: None,
            code_offsets: HashMap::new(),
            jump_tables: HashMap::new(),
            function_indices: HashMap::new(),
            config: CodegenConfig::default(),
            scope_level: 0,
            current_loop_label: None,
        };

        // 为所有函数建立索引
        for (idx, func) in ctx.module.functions.iter().enumerate() {
            ctx.function_indices.insert(func.name.clone(), idx);
        }

        ctx
    }

    /// 生成字节码
    pub fn generate(&mut self) -> Result<BytecodeFile, CodegenError> {
        // 1. 生成常量池
        let const_pool = std::mem::take(&mut self.constant_pool.constants);

        // 2. 生成代码段
        let mut code_section = CodeSection {
            functions: Vec::new(),
        };

        // 克隆函数以避免借用问题
        let functions = self.module.functions.clone();
        for func in functions {
            self.generate_function(&func, &mut code_section)?;
        }

        // 3. 生成类型表
        let type_table: Vec<MonoType> = self.module.types.iter().map(|t| self.type_from_ast(t)).collect();

        // 4. 生成文件头
        let header = self.generate_header();

        Ok(BytecodeFile {
            header,
            type_table,
            const_pool,
            code_section,
        })
    }

    /// 生成函数
    fn generate_function(
        &mut self,
        func: &FunctionIR,
        code_section: &mut CodeSection,
    ) -> Result<(), CodegenError> {
        self.current_function = Some(func.clone());
        self.register_allocator = RegisterAllocator::new();

        // 生成函数体
        let instructions = self.generate_instructions(func)?;

        code_section.functions.push(FunctionCode {
            name: func.name.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone(),
            instructions,
            local_count: func.locals.len(),
        });

        Ok(())
    }

    /// 生成函数指令
    fn generate_instructions(&mut self, func: &FunctionIR) -> Result<Vec<BytecodeInstruction>, CodegenError> {
        let mut instructions = Vec::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                let bytecode_instr = self.translate_instruction(instr)?;
                instructions.push(bytecode_instr);
            }
        }

        Ok(instructions)
    }

    /// 翻译 IR 指令为字节码指令
    fn translate_instruction(&mut self, instr: &Instruction) -> Result<BytecodeInstruction, CodegenError> {
        use Instruction::*;

        match instr {
            // 移动和加载
            Move { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(TypedOpcode::Mov, vec![dst_reg, src_reg]))
            }

            Load { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                match src {
                    Operand::Const(const_val) => {
                        let const_idx = self.add_constant(const_val.clone());
                        let idx_bytes = (const_idx as u16).to_le_bytes();
                        Ok(BytecodeInstruction::new(TypedOpcode::LoadConst, vec![dst_reg, idx_bytes[0], idx_bytes[1]]))
                    }
                    _ => {
                        let src_reg = self.operand_to_reg(src)?;
                        Ok(BytecodeInstruction::new(TypedOpcode::Mov, vec![dst_reg, src_reg]))
                    }
                }
            }

            // 算术运算
            Add { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(TypedOpcode::I64Add, vec![dst_reg, lhs_reg, rhs_reg]))
            }

            // ... 其他算术运算类似

            // 控制流
            Jmp(target) => {
                let offset = *target as i32;
                let bytes = offset.to_le_bytes();
                Ok(BytecodeInstruction::new(TypedOpcode::Jmp, bytes.to_vec()))
            }

            JmpIf(cond, target) => {
                let cond_reg = self.operand_to_reg(cond)?;
                let offset = *target as i32;
                let offset_bytes = (offset as i16).to_le_bytes();
                Ok(BytecodeInstruction::new(TypedOpcode::JmpIf, vec![cond_reg, offset_bytes[0], offset_bytes[1]]))
            }

            Ret(value) => {
                if let Some(v) = value {
                    let reg = self.operand_to_reg(v)?;
                    Ok(BytecodeInstruction::new(TypedOpcode::ReturnValue, vec![reg]))
                } else {
                    Ok(BytecodeInstruction::new(TypedOpcode::Return, vec![]))
                }
            }

            // ... 其他指令
            _ => Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![])),
        }
    }

    /// 将操作数转换为寄存器编号
    fn operand_to_reg(&self, operand: &Operand) -> Result<u8, CodegenError> {
        match operand {
            Operand::Local(id) => Ok(*id as u8),
            Operand::Temp(id) => Ok(*id as u8),
            Operand::Arg(id) => Ok(*id as u8),
            _ => Err(CodegenError::InvalidOperand),
        }
    }
}
```

---

## 六、优化策略

### 6.1 编译时优化

#### 6.1.1 常量折叠与传播

```rust
// 在 IR 层面进行
// 原始：a = 2 + 3; b = a * 4
// 优化：直接计算 a = 5; b = 20

pub fn constant_folding(ir: &mut ModuleIR) {
    for func in ir.functions.iter_mut() {
        for block in func.blocks.iter_mut() {
            let mut constant_values = HashMap::new();

            for instr in block.instructions.iter_mut() {
                match instr {
                    Instruction::Const(c) => {
                        // 记录常量
                    }
                    Instruction::Binary { op, left, right, dest } => {
                        if let (Operand::Const(l), Operand::Const(r)) = (left, right) {
                            if let Some(result) = self.compute_const(op, l, r) {
                                *instr = Instruction::Load {
                                    dst: *dest,
                                    src: Operand::Const(result),
                                };
                                constant_values.insert(*dest, result);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
```

#### 6.1.2 内联优化

```rust
pub fn inline_functions(&self, ir: &mut ModuleIR) {
    // 对于小函数，直接内联到调用点
    for func in ir.functions.iter() {
        if func.is_inlineable() && func.size() < INLINE_THRESHOLD {
            // 查找所有调用点
            // 替换为函数体
        }
    }
}
```

### 6.2 运行时优化

#### 6.2.1 内联缓存 (Inline Caches)

```rust
// vm/inline_cache.rs
pub struct InlineCache {
    pub caches: HashMap<(TypeId, String), FuncId>,
}

impl InlineCache {
    pub fn lookup(&mut self, type_id: TypeId, method: &str) -> Option<FuncId> {
        let key = (type_id, method.to_string());

        if let Some(cached) = self.caches.get(&key) {
            return Some(*cached);
        }

        // 未命中，查找并缓存
        if let Some(func) = self.find_method(type_id, method) {
            self.caches.insert(key, func);
            Some(func)
        } else {
            None
        }
    }
}
```

---

## 七、错误处理

### 7.1 错误类型层次

```rust
/// 编译错误
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Lexical error: {0}")]
    LexError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Code generation error: {0}")]
    CodegenError(String),
}

/// 类型错误
#[derive(Debug, Error)]
pub enum TypeError {
    #[error("Unknown variable: {name}")]
    UnknownVariable { name: String, span: Span },

    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String, span: Span },

    #[error("Arity mismatch: expected {expected}, found {found}")]
    ArityMismatch { expected: usize, found: usize, span: Span },

    #[error("Not callable: {ty}")]
    NotCallable { ty: String, span: Span },

    #[error("Infinite type: t{var}")]
    InfiniteType { var: usize, span: Span },

    #[error("Unknown type: {name}")]
    UnknownType { name: String, span: Span },
}
```

### 7.2 错误恢复策略

```rust
impl Parser {
    fn recover_from_error(&mut self) {
        // 跳过到下一个语句分隔符
        while !self.is_at_end() {
            match self.current_token() {
                Token::Semicolon | Token::RBrace | Token::Eof => break,
                _ => self.advance(),
            }
        }
    }
}
```

---

## 八、性能考虑

### 8.1 编译器性能

| 阶段 | 时间复杂度 | 空间复杂度 |
|-----|-----------|-----------|
| 词法分析 | O(n) | O(n) |
| 语法分析 | O(n) | O(n) |
| 类型推断 | O(n * m) | O(n + m) |
| 优化 | O(n * k) | O(n) |
| 代码生成 | O(n) | O(n) |

### 8.2 生成代码性能

- **字节码设计**：紧凑编码，使用变长编码
- **虚拟机优化**：使用 computed goto 或 switch 优化
- **JIT 编译**：热点代码生成机器码（未来扩展）

---

## 九、总结

YaoXiang 编译器采用现代编译器架构：

1. **前端**：Pratt Parser + Hindley-Milner 类型推断
2. **中端**：SSA IR + 多种优化通道
3. **后端**：类型化字节码 + 紧凑编码
4. **运行时**：高效的虚拟机 + 并作调度

**核心创新点**：

- **双层处理**：解析层宽松，类型检查层严格
- **位置绑定**：`[n]` 语法实现细粒度柯里化
- **并作模型**：自动并发，零认知负担
- **类型集中约定**：AI 友好的声明语法

**最后更新**：2025-01-04
