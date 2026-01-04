# YaoXiang 编译器设计文档

> 版本：v1.0.0
> 状态：正式
> 作者：晨煦
> 日期：2025-01-04

---

## 目录

1. [概述](#一概述)
2. [前段设计](#二前段设计)
3. [中端设计](#三中端设计)
4. [后端设计](#四后端设计)
5. [类型系统实现](#五类型系统实现)
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
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│                     后端 (Backend)                      │
├─────────────────────────────────────────────────────────┤
│  字节码生成 → Instruction 流                            │
│  ↓                                                      │
│  字节码优化 → 优化的指令流                              │
│  ↓                                                      │
│  输出 → 字节码文件                                      │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│                     运行时 (Runtime)                    │
├─────────────────────────────────────────────────────────┤
│  虚拟机执行 → 执行字节码                                │
│  调度器管理 → 并作任务调度                              │
│  内存管理 → 栈/堆分配                                   │
└─────────────────────────────────────────────────────────┘
```

---

## 二、前段设计

### 2.1 词法分析器 (Lexer)

#### 2.1.1 设计原则
- **高性能**：单次扫描，O(n) 时间复杂度
- **精确位置**：记录每个 Token 的行号、列号
- **容错性**：遇到错误时尽可能继续扫描

#### 2.1.2 实现细节

```rust
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

#### 2.1.3 Token 定义

```rust
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
```

### 2.2 语法分析器 (Parser)

#### 2.2.1 设计原则
- **左递归消除**：使用 Pratt Parser 处理表达式
- **清晰的优先级**：显式定义运算符优先级
- **错误恢复**：遇到错误时跳过到安全位置

#### 2.2.2 Pratt Parser 实现

```rust
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

// 表达式优先级
#[derive(PartialOrd, PartialEq)]
enum Precedence {
    Lowest,
    Equals,      // == !=
    LessGreater, // < > <= >=
    Add,         // + -
    Multiply,    // * /
    Unary,       // ! -
    Call,        // function(x)
    Index,       // array[i]
}

impl Parser {
    // 表达式解析入口
    pub fn parse_expression(&mut self, prec: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;
        
        while !self.is_at_end() && prec <= self.get_precedence() {
            left = self.parse_infix(left)?;
        }
        
        Ok(left)
    }
    
    // 前缀解析（处理单目运算符、字面量、括号等）
    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        match self.current_token() {
            Token::Integer(n) => {
                self.advance();
                Ok(Expr::Literal(Literal::Integer(n)))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expr::Literal(Literal::Float(f)))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            Token::BoolLiteral(b) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(b)))
            }
            Token::Identifier(name) => {
                self.advance();
                Ok(Expr::Identifier(name))
            }
            Token::LParen => {
                self.advance(); // 消耗 '('
                let expr = self.parse_expression(Precedence::Lowest)?;
                self.consume(Token::RParen, "Expected ')'")?;
                Ok(expr)
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_expression(Precedence::Unary)?;
                Ok(Expr::Unary {
                    op: UnaryOp::Negate,
                    operand: Box::new(expr),
                })
            }
            Token::Bang => {
                self.advance();
                let expr = self.parse_expression(Precedence::Unary)?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(expr),
                })
            }
            _ => {
                Err(ParseError::UnexpectedToken(
                    self.current_token().clone(),
                    "Expected expression".to_string(),
                ))
            }
        }
    }
    
    // 中缀解析（处理二目运算符、函数调用、索引等）
    fn parse_infix(&mut self, left: Expr) -> Result<Expr, ParseError> {
        match self.current_token() {
            Token::Plus | Token::Minus | Token::Star | Token::Slash => {
                let op = self.parse_binary_op()?;
                self.advance();
                let prec = self.get_precedence();
                let right = self.parse_expression(prec)?;
                Ok(Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                })
            }
            Token::LParen => {
                self.advance(); // 消耗 '('
                let args = self.parse_argument_list()?;
                Ok(Expr::Call {
                    func: Box::new(left),
                    args,
                })
            }
            Token::LBracket => {
                self.advance(); // 消耗 '['
                let index = self.parse_expression(Precedence::Lowest)?;
                self.consume(Token::RBracket, "Expected ']'")?;
                Ok(Expr::Index {
                    expr: Box::new(left),
                    index: Box::new(index),
                })
            }
            _ => Err(ParseError::UnexpectedToken(
                self.current_token().clone(),
                "Expected infix operator".to_string(),
            )),
        }
    }
    
    // 运算符优先级映射
    fn get_precedence(&self) -> Precedence {
        match self.current_token() {
            Token::Eq | Token::Ne => Precedence::Equals,
            Token::Lt | Token::Le | Token::Gt | Token::Ge => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Add,
            Token::Star | Token::Slash => Precedence::Multiply,
            Token::LParen => Precedence::Call,
            Token::LBracket => Precedence::Index,
            _ => Precedence::Lowest,
        }
    }
}
```

#### 2.2.3 语句解析

```rust
impl Parser {
    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match self.current_token() {
            Token::Let => self.parse_let_statement(),
            Token::Function => self.parse_function_statement(),
            Token::Type => self.parse_type_statement(),
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::For => self.parse_for_statement(),
            Token::Return => self.parse_return_statement(),
            Token::Spawn => self.parse_spawn_statement(),
            _ => {
                // 表达式语句
                let expr = self.parse_expression(Precedence::Lowest)?;
                self.consume(Token::Semicolon, "Expected ';'")?;
                Ok(Stmt::Expr(expr))
            }
        }
    }
    
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
        
        Ok(Stmt::Let { name, ty, value })
    }
}
```

### 2.3 类型检查器

#### 2.3.1 类型定义

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // 原子类型
    Void,
    Bool,
    Int,
    Uint,
    Float,
    String,
    
    // 复合类型
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    
    // 泛型
    Generic {
        name: String,
        args: Vec<Type>,
    },
    
    // 构造器（联合类型）
    Constructor {
        name: String,
        args: Vec<Type>,
    },
    
    // 类型变量（用于推断）
    Variable(TypeVar),
    
    // 未知（用于错误恢复）
    Unknown,
}

// 类型变量，使用整数 ID 标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub u32);

// 类型方案（泛型包装）
#[derive(Debug, Clone)]
pub struct TypeScheme {
    pub vars: Vec<TypeVar>,
    pub body: Type,
}
```

#### 2.3.2 类型推断算法 (Hindley-Milner)

```rust
pub struct TypeChecker {
    // 当前类型环境：变量名 -> 类型方案
    env: HashMap<String, TypeScheme>,
    
    // 约束收集器
    constraints: Vec<Constraint>,
    
    // 类型变量计数器
    next_var: u32,
}

// 类型约束
#[derive(Debug, Clone)]
pub enum Constraint {
    // 两个类型必须相等
    Equal(Type, Type),
    
    // 类型变量必须满足某些约束
    Variable(TypeVar, Type),
}

impl TypeChecker {
    // 表达式类型推断
    pub fn infer_expr(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {
            Expr::Literal(Literal::Integer(_)) => Ok(Type::Int),
            Expr::Literal(Literal::Float(_)) => Ok(Type::Float),
            Expr::Literal(Literal::Bool(_)) => Ok(Type::Bool),
            Expr::Literal(Literal::String(_)) => Ok(Type::String),
            
            Expr::Identifier(name) => {
                let scheme = self.env.get(name)
                    .ok_or_else(|| TypeError::UndefinedVariable(name.clone()))?;
                Ok(self.instantiate(scheme))
            }
            
            Expr::Binary { left, op, right } => {
                let left_ty = self.infer_expr(left)?;
                let right_ty = self.infer_expr(right)?;
                
                // 添加约束：左右操作数类型必须匹配
                self.add_constraint(Constraint::Equal(left_ty.clone(), right_ty));
                
                // 根据运算符确定返回类型
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        // 数值运算要求都是数字
                        self.add_constraint(Constraint::Equal(left_ty.clone(), Type::Int));
                        self.add_constraint(Constraint::Equal(left_ty.clone(), Type::Float));
                        Ok(left_ty)
                    }
                    BinOp::Eq | BinOp::Ne => Ok(Type::Bool),
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => Ok(Type::Bool),
                }
            }
            
            Expr::Lambda { params, body } => {
                // 为每个参数创建新类型变量
                let param_tys: Vec<Type> = params.iter()
                    .map(|_| self.fresh_var())
                    .collect();
                
                // 创建新的作用域
                let mut new_env = self.env.clone();
                for (param, ty) in params.iter().zip(param_tys.iter()) {
                    new_env.insert(
                        param.name.clone(),
                        TypeScheme { vars: vec![], body: ty.clone() },
                    );
                }
                
                // 保存当前环境
                let old_env = std::mem::replace(&mut self.env, new_env);
                let ret_ty = self.infer_expr(body)?;
                self.env = old_env;
                
                Ok(Type::Function {
                    params: param_tys,
                    ret: Box::new(ret_ty),
                })
            }
            
            Expr::Call { func, args } => {
                let func_ty = self.infer_expr(func)?;
                let arg_tys: Vec<Type> = args.iter()
                    .map(|arg| self.infer_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                
                // 创建函数类型模板
                let param_tys: Vec<Type> = arg_tys.iter()
                    .map(|_| self.fresh_var())
                    .collect();
                let ret_ty = self.fresh_var();
                
                // 约束：函数类型必须匹配
                self.add_constraint(Constraint::Equal(
                    func_ty,
                    Type::Function {
                        params: param_tys.clone(),
                        ret: Box::new(ret_ty.clone()),
                    },
                ));
                
                // 约束：参数类型必须匹配
                for (arg, param) in arg_tys.iter().zip(param_tys.iter()) {
                    self.add_constraint(Constraint::Equal(arg.clone(), param.clone()));
                }
                
                Ok(ret_ty)
            }
            
            _ => Err(TypeError::Unimplemented),
        }
    }
    
    // 实例化泛型类型（将泛型变量替换为具体变量）
    fn instantiate(&self, scheme: &TypeScheme) -> Type {
        let mut subst = HashMap::new();
        for var in &scheme.vars {
            subst.insert(*var, self.fresh_var());
        }
        self.apply_substitution(&scheme.body, &subst)
    }
    
    // 创建新的类型变量
    fn fresh_var(&mut self) -> Type {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        Type::Variable(var)
    }
    
    // 添加约束
    fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }
    
    // 求解约束（统一算法）
    pub fn solve_constraints(&mut self) -> Result<Substitution, TypeError> {
        let mut substitution = HashMap::new();
        
        while let Some(constraint) = self.constraints.pop() {
            match constraint {
                Constraint::Equal(t1, t2) => {
                    let new_subst = self.unify(&t1, &t2)?;
                    // 应用新替换到现有约束
                    self.constraints = self.constraints.iter()
                        .map(|c| self.apply_constraint(c, &new_subst))
                        .collect();
                    // 合并替换
                    for (var, ty) in new_subst {
                        substitution.insert(var, self.apply_substitution(&ty, &substitution));
                    }
                }
                Constraint::Variable(var, ty) => {
                    let new_subst = HashMap::from([(var, ty)]);
                    self.constraints = self.constraints.iter()
                        .map(|c| self.apply_constraint(c, &new_subst))
                        .collect();
                    for (v, t) in new_subst {
                        substitution.insert(v, self.apply_substitution(&t, &substitution));
                    }
                }
            }
        }
        
        Ok(substitution)
    }
    
    // 统一算法（解决类型相等）
    fn unify(&self, t1: &Type, t2: &Type) -> Result<Substitution, TypeError> {
        match (t1, t2) {
            // 相同类型自动统一
            (a, b) if a == b => Ok(HashMap::new()),
            
            // 类型变量统一
            (Type::Variable(var), ty) => {
                if self.occurs_check(*var, ty) {
                    Err(TypeError::InfiniteType)
                } else {
                    Ok(HashMap::from([(*var, ty.clone())]))
                }
            }
            (ty, Type::Variable(var)) => {
                if self.occurs_check(*var, ty) {
                    Err(TypeError::InfiniteType)
                } else {
                    Ok(HashMap::from([(*var, ty.clone())]))
                }
            }
            
            // 函数类型统一
            (
                Type::Function { params: p1, ret: r1 },
                Type::Function { params: p2, ret: r2 },
            ) => {
                if p1.len() != p2.len() {
                    return Err(TypeError::ArityMismatch);
                }
                
                let mut subst = HashMap::new();
                for (param1, param2) in p1.iter().zip(p2.iter()) {
                    let param_subst = self.unify(param1, param2)?;
                    // 应用并合并
                    for (var, ty) in param_subst {
                        subst.insert(var, self.apply_substitution(&ty, &subst));
                    }
                }
                
                let ret_subst = self.unify(r1, r2)?;
                for (var, ty) in ret_subst {
                    subst.insert(var, self.apply_substitution(&ty, &subst));
                }
                
                Ok(subst)
            }
            
            // 构造器类型统一
            (
                Type::Constructor { name: n1, args: a1 },
                Type::Constructor { name: n2, args: a2 },
            ) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return Err(TypeError::TypeMismatch);
                }
                
                let mut subst = HashMap::new();
                for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                    let arg_subst = self.unify(arg1, arg2)?;
                    for (var, ty) in arg_subst {
                        subst.insert(var, self.apply_substitution(&ty, &subst));
                    }
                }
                
                Ok(subst)
            }
            
            // 无法统一
            _ => Err(TypeError::TypeMismatch),
        }
    }
    
    // 循环检查（防止无限类型）
    fn occurs_check(&self, var: TypeVar, ty: &Type) -> bool {
        match ty {
            Type::Variable(v) => *v == var,
            Type::Function { params, ret } => {
                params.iter().any(|p| self.occurs_check(var, p)) ||
                self.occurs_check(var, ret)
            }
            Type::Constructor { args, .. } => {
                args.iter().any(|a| self.occurs_check(var, a))
            }
            _ => false,
        }
    }
    
    // 应用替换
    fn apply_substitution(&self, ty: &Type, subst: &Substitution) -> Type {
        match ty {
            Type::Variable(var) => {
                if let Some(replacement) = subst.get(var) {
                    replacement.clone()
                } else {
                    ty.clone()
                }
            }
            Type::Function { params, ret } => {
                Type::Function {
                    params: params.iter()
                        .map(|p| self.apply_substitution(p, subst))
                        .collect(),
                    ret: Box::new(self.apply_substitution(ret, subst)),
                }
            }
            Type::Constructor { name, args } => {
                Type::Constructor {
                    name: name.clone(),
                    args: args.iter()
                        .map(|a| self.apply_substitution(a, subst))
                        .collect(),
                }
            }
            _ => ty.clone(),
        }
    }
    
    fn apply_constraint(&self, constraint: &Constraint, subst: &Substitution) -> Constraint {
        match constraint {
            Constraint::Equal(t1, t2) => {
                Constraint::Equal(
                    self.apply_substitution(t1, subst),
                    self.apply_substitution(t2, subst),
                )
            }
            Constraint::Variable(var, ty) => {
                Constraint::Variable(*var, self.apply_substitution(ty, subst))
            }
        }
    }
}
```

---

## 三、中端设计

### 3.1 中间表示 (IR)

#### 3.1.1 IR 设计原则
- **SSA 形式**：每个变量只赋值一次
- **控制流图**：基本块和跳转指令
- **类型信息**：所有操作都有明确的类型
- **平台无关**：不依赖特定架构

#### 3.1.2 IR 定义

```rust
// 基本块
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
}

// 指令（无跳转）
#[derive(Debug, Clone)]
pub enum Instruction {
    // 值操作
    Const(Constant),
    Binary {
        op: BinaryOp,
        left: Value,
        right: Value,
        dest: VarId,
    },
    Unary {
        op: UnaryOp,
        operand: Value,
        dest: VarId,
    },
    
    // 内存操作
    Load {
        source: Value,  // 可能是全局、局部、或指针
        dest: VarId,
    },
    Store {
        source: Value,
        dest: Value,    // 存储位置
    },
    
    // 函数调用
    Call {
        func: FuncId,
        args: Vec<Value>,
        dest: Option<VarId>,  // 无返回值时为 None
    },
    
    // 类型操作
    Cast {
        value: Value,
        target_type: Type,
        dest: VarId,
    },
    
    // 并发
    Spawn {
        func: FuncId,
        args: Vec<Value>,
        dest: VarId,  // 返回任务 ID
    },
}

// 终结符（控制流转移）
#[derive(Debug, Clone)]
pub enum Terminator {
    // 无条件跳转
    Jump(BlockId),
    
    // 条件跳转
    Branch {
        condition: Value,
        then_block: BlockId,
        else_block: BlockId,
    },
    
    // 返回
    Return(Option<Value>),
    
    // 异常传播
    Unreachable,
}

// 值（操作数）
#[derive(Debug, Clone)]
pub enum Value {
    // 变量（SSA）
    Var(VarId),
    
    // 常量
    Constant(Constant),
    
    // 全局符号
    Global(GlobalId),
    
    // 函数引用
    Function(FuncId),
}

// 常量
#[derive(Debug, Clone)]
pub enum Constant {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,  // 空值
}

// ID 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalId(u32);
```

#### 3.1.3 IR 构建

```rust
pub struct IRBuilder {
    current_block: BlockId,
    blocks: HashMap<BlockId, BasicBlock>,
    var_counter: u32,
}

impl IRBuilder {
    pub fn new() -> Self {
        let entry = BlockId(0);
        Self {
            current_block: entry,
            blocks: HashMap::new(),
            var_counter: 0,
        }
    }
    
    pub fn emit_binary(
        &mut self,
        op: BinaryOp,
        left: Value,
        right: Value,
    ) -> VarId {
        let dest = self.fresh_var();
        let instr = Instruction::Binary { op, left, right, dest };
        self.add_instruction(instr);
        dest
    }
    
    pub fn emit_call(
        &mut self,
        func: FuncId,
        args: Vec<Value>,
    ) -> Option<VarId> {
        // 检查函数返回类型
        let ret_ty = self.get_func_return_type(func);
        let dest = if ret_ty != Type::Void {
            Some(self.fresh_var())
        } else {
            None
        };
        
        let instr = Instruction::Call { func, args, dest };
        self.add_instruction(instr);
        dest
    }
    
    fn fresh_var(&mut self) -> VarId {
        let id = VarId(self.var_counter);
        self.var_counter += 1;
        id
    }
    
    fn add_instruction(&mut self, instr: Instruction) {
        if let Some(block) = self.blocks.get_mut(&self.current_block) {
            block.instructions.push(instr);
        }
    }
}
```

### 3.2 优化器

#### 3.2.1 优化通道

```rust
pub struct Optimizer {
    passes: Vec<Box<dyn OptimizationPass>>,
}

pub trait OptimizationPass {
    fn name(&self) -> &str;
    fn run(&self, ir: &mut IRModule) -> bool;  // 返回是否修改
}

// 常量折叠
struct ConstantFoldingPass;

impl OptimizationPass for ConstantFoldingPass {
    fn name(&self) -> &str { "constant_folding" }
    
    fn run(&self, ir: &mut IRModule) -> bool {
        let mut modified = false;
        
        for func in ir.functions.iter_mut() {
            for block in func.blocks.iter_mut() {
                let mut new_instructions = Vec::new();
                
                for instr in block.instructions.iter() {
                    if let Instruction::Binary { op, left, right, dest } = instr {
                        if let (Value::Constant(l), Value::Constant(r)) = (left, right) {
                            // 可以在编译时计算
                            if let Some(result) = self.fold_constants(op, l, r) {
                                new_instructions.push(Instruction::Const(result));
                                modified = true;
                                continue;
                            }
                        }
                    }
                    new_instructions.push(instr.clone());
                }
                
                block.instructions = new_instructions;
            }
        }
        
        modified
    }
}

// 死代码消除
struct DeadCodeEliminationPass;

impl OptimizationPass for DeadCodeEliminationPass {
    fn name(&self) -> &str { "dead_code_elimination" }
    
    fn run(&self, ir: &mut IRModule) -> bool {
        let mut modified = false;
        let mut used_vars = HashSet::new();
        
        // 收集所有被使用的变量
        for func in ir.functions.iter() {
            for block in func.blocks.iter() {
                for instr in block.instructions.iter() {
                    match instr {
                        Instruction::Binary { left, right, .. } => {
                            if let Value::Var(v) = left { used_vars.insert(*v); }
                            if let Value::Var(v) = right { used_vars.insert(*v); }
                        }
                        Instruction::Call { args, .. } => {
                            for arg in args {
                                if let Value::Var(v) = arg {
                                    used_vars.insert(*v);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // 移除未使用的赋值
        for func in ir.functions.iter_mut() {
            for block in func.blocks.iter_mut() {
                let mut new_instructions = Vec::new();
                
                for instr in block.instructions.iter() {
                    if let Instruction::Binary { dest, .. } = instr {
                        if !used_vars.contains(dest) {
                            modified = true;
                            continue;  // 移除
                        }
                    }
                    new_instructions.push(instr.clone());
                }
                
                block.instructions = new_instructions;
            }
        }
        
        modified
    }
}
```

### 3.3 泛型单态化

```rust
pub struct Monomorphizer {
    // 泛型函数实例化缓存
    instances: HashMap<(FuncId, Vec<Type>), FuncId>,
}

impl Monomorphizer {
    pub fn monomorphize(&mut self, ir: &mut IRModule) {
        // 1. 收集所有泛型函数调用
        let mut calls_to_replace = Vec::new();
        
        for func in ir.functions.iter() {
            for block in func.blocks.iter() {
                for (idx, instr) in block.instructions.iter().enumerate() {
                    if let Instruction::Call { func: func_id, args, .. } = instr {
                        if let Some(generic_func) = ir.functions.get(func_id.0 as usize) {
                            if generic_func.type_params.len() > 0 {
                                // 推断类型参数
                                let type_args = self.infer_type_args(generic_func, args);
                                calls_to_replace.push((
                                    func_id.clone(),
                                    type_args,
                                    idx,
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        // 2. 为每个调用生成具体版本
        for (generic_id, type_args, _) in calls_to_replace {
            let instance_id = self.get_or_create_instance(ir, generic_id, type_args);
            // 替换调用指令...
        }
    }
    
    fn get_or_create_instance(
        &mut self,
        ir: &mut IRModule,
        generic_id: FuncId,
        type_args: Vec<Type>,
    ) -> FuncId {
        let key = (generic_id, type_args.clone());
        
        if let Some(cached) = self.instances.get(&key) {
            return *cached;
        }
        
        // 创建新实例
        let generic_func = &ir.functions[generic_id.0 as usize];
        let instance = self.instantiate_function(generic_func, type_args.clone());
        
        let instance_id = FuncId(ir.functions.len() as u32);
        ir.functions.push(instance);
        
        self.instances.insert(key, instance_id);
        instance_id
    }
    
    fn instantiate_function(&self, func: &Function, type_args: Vec<Type>) -> Function {
        // 1. 创建类型替换映射
        let mut subst = HashMap::new();
        for (param, arg) in func.type_params.iter().zip(type_args.iter()) {
            subst.insert(*param, arg.clone());
        }
        
        // 2. 替换函数体中的所有类型
        let mut new_func = func.clone();
        new_func.type_params.clear();  // 不再是泛型
        
        // 3. 替换指令中的类型和变量
        for block in new_func.blocks.iter_mut() {
            for instr in block.instructions.iter_mut() {
                self.substitute_types(instr, &subst);
            }
        }
        
        new_func
    }
}
```

### 3.4 逃逸分析

```rust
pub struct EscapeAnalyzer {
    // 值的分配点
    allocations: HashMap<VarId, AllocationSite>,
}

#[derive(Debug, Clone)]
pub enum AllocationSite {
    // 栈分配（不会逃逸）
    Stack,
    
    // 堆分配（可能逃逸）
    Heap,
    
    // 不确定（需要进一步分析）
    Unknown,
}

impl EscapeAnalyzer {
    pub fn analyze(&mut self, ir: &IRModule) -> HashMap<VarId, AllocationSite> {
        let mut results = HashMap::new();
        
        for func in ir.functions.iter() {
            for block in func.blocks.iter() {
                for instr in block.instructions.iter() {
                    match instr {
                        // 函数调用可能导致逃逸
                        Instruction::Call { args, dest, .. } => {
                            if let Some(d) = dest {
                                // 如果函数可能存储参数到全局，arg 会逃逸
                                // 如果 dest 返回的是指针，也可能逃逸
                                if self.may_escape(func, args) {
                                    results.insert(*d, AllocationSite::Heap);
                                }
                            }
                        }
                        
                        // 全局存储导致逃逸
                        Instruction::Store { dest, .. } => {
                            if let Value::Global(_) = dest {
                                // 源值会逃逸
                            }
                        }
                        
                        // 其他指令...
                        _ => {}
                    }
                }
            }
        }
        
        // 传播分析：如果一个值被赋值给会逃逸的变量，它也逃逸
        self.propagate_escaping(&mut results, ir);
        
        results
    }
    
    fn may_escape(&self, func: &Function, args: &[Value]) -> bool {
        // 简单分析：检查函数是否是外部函数或接受函数指针
        // 更复杂的分析需要函数摘要
        true
    }
    
    fn propagate_escaping(&self, results: &mut HashMap<VarId, AllocationSite>, ir: &IRModule) {
        // 如果 var1 = var2 且 var2 逃逸，则 var1 也逃逸
        // 需要多次迭代直到收敛
    }
}
```

---

## 四、后端设计

### 4.1 字节码生成

#### 4.1.1 指令选择策略

```rust
pub struct BytecodeGenerator {
    instructions: Vec<Instruction>,
    constant_pool: Vec<Constant>,
    function_table: Vec<FunctionHeader>,
}

impl BytecodeGenerator {
    pub fn from_ir(&mut self, ir: &IRModule) {
        for func in ir.functions.iter() {
            self.generate_function(func);
        }
    }
    
    fn generate_function(&mut self, func: &Function) {
        let func_idx = self.function_table.len();
        self.function_table.push(FunctionHeader {
            name: func.name.clone(),
            arity: func.params.len() as u8,
            entry_point: self.instructions.len() as u32,
        });
        
        for block in func.blocks.iter() {
            for instr in block.instructions.iter() {
                self.generate_instruction(instr);
            }
            
            if let Some(term) = &block.terminator {
                self.generate_terminator(term);
            }
        }
    }
    
    fn generate_instruction(&mut self, instr: &Instruction) {
        match instr {
            Instruction::Const(c) => {
                let idx = self.add_constant(c.clone());
                self.emit(Instruction::PushConstant(idx));
            }
            
            Instruction::Binary { op, left, right, dest } => {
                // 加载左操作数
                self.load_value(left);
                self.load_value(right);
                
                // 执行运算
                match op {
                    BinaryOp::Add => self.emit(Instruction::Add),
                    BinaryOp::Sub => self.emit(Instruction::Sub),
                    BinaryOp::Mul => self.emit(Instruction::Mul),
                    BinaryOp::Div => self.emit(Instruction::Div),
                    _ => {}
                }
                
                // 存储结果
                self.store_var(*dest);
            }
            
            Instruction::Call { func, args, dest } => {
                for arg in args {
                    self.load_value(arg);
                }
                self.emit(Instruction::Call(func.0));
                if let Some(d) = dest {
                    self.store_var(*d);
                }
            }
            
            Instruction::Spawn { func, args, dest } => {
                for arg in args {
                    self.load_value(arg);
                }
                self.emit(Instruction::Spawn(func.0));
                self.store_var(*dest);
            }
            
            _ => {}
        }
    }
    
    fn generate_terminator(&mut self, term: &Terminator) {
        match term {
            Terminator::Jump(block) => {
                // 需要先计算目标地址
                let addr = self.get_block_address(*block);
                self.emit(Instruction::Jump(addr));
            }
            Terminator::Branch { condition, then_block, else_block } => {
                self.load_value(condition);
                let then_addr = self.get_block_address(*then_block);
                let else_addr = self.get_block_address(*else_block);
                self.emit(Instruction::JumpIf(then_addr));  // 简化
                self.emit(Instruction::Jump(else_addr));
            }
            Terminator::Return(value) => {
                if let Some(val) = value {
                    self.load_value(val);
                }
                self.emit(Instruction::Return);
            }
            Terminator::Unreachable => {
                self.emit(Instruction::Halt);
            }
        }
    }
    
    fn load_value(&mut self, value: &Value) {
        match value {
            Value::Var(var) => {
                self.emit(Instruction::LoadLocal(var.0));
            }
            Value::Constant(c) => {
                let idx = self.add_constant(c.clone());
                self.emit(Instruction::PushConstant(idx));
            }
            Value::Global(glob) => {
                self.emit(Instruction::LoadGlobal(glob.0));
            }
            Value::Function(func) => {
                let idx = self.add_constant(Constant::FunctionRef(func.0));
                self.emit(Instruction::PushConstant(idx));
            }
        }
    }
    
    fn store_var(&mut self, var: VarId) {
        self.emit(Instruction::StoreLocal(var.0));
    }
    
    fn add_constant(&mut self, c: Constant) -> u32 {
        if let Some(idx) = self.constant_pool.iter().position(|existing| existing == &c) {
            idx as u32
        } else {
            let idx = self.constant_pool.len();
            self.constant_pool.push(c);
            idx as u32
        }
    }
    
    fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }
}
```

### 4.2 字节码优化

```rust
pub struct BytecodeOptimizer {
    // 简化的优化
    // 1. 常量传播
    // 2. 死代码消除
    // 3. 跳转优化
    // 4. 尾调用优化
}

impl BytecodeOptimizer {
    pub fn optimize(&self, instructions: &mut Vec<Instruction>) {
        self.remove_unreachable_code(instructions);
        self.optimize_jumps(instructions);
        self.inline_constants(instructions);
    }
    
    // 移除不可达代码
    fn remove_unreachable_code(&self, instructions: &mut Vec<Instruction>) {
        let mut reachable = vec![true; instructions.len()];
        
        // 标记所有跳转目标
        for (i, instr) in instructions.iter().enumerate() {
            match instr {
                Instruction::Jump(target) => {
                    if (*target as usize) < instructions.len() {
                        reachable[*target as usize] = true;
                    }
                }
                Instruction::JumpIf(target) => {
                    if (*target as usize) < instructions.len() {
                        reachable[*target as usize] = true;
                    }
                }
                Instruction::Return | Instruction::Halt => {
                    // 后续指令不可达，除非有标签
                    for j in (i + 1)..instructions.len() {
                        reachable[j] = false;
                    }
                }
                _ => {}
            }
        }
        
        // 移除不可达指令
        let mut new_instructions = Vec::new();
        for (i, instr) in instructions.iter().enumerate() {
            if reachable[i] {
                new_instructions.push(instr.clone());
            }
        }
        *instructions = new_instructions;
    }
    
    // 优化跳转（消除冗余跳转）
    fn optimize_jumps(&self, instructions: &mut Vec<Instruction>) {
        for i in 0..instructions.len() {
            if let Instruction::Jump(target) = instructions[i] {
                // 如果跳转到跳转指令，直接跳到最后目标
                if let Instruction::Jump(target2) = instructions[target as usize] {
                    instructions[i] = Instruction::Jump(target2);
                }
            }
        }
    }
    
    // 常量内联
    fn inline_constants(&self, instructions: &mut Vec<Instruction>) {
        let mut constants = HashMap::new();
        
        for i in 0..instructions.len() {
            match &instructions[i] {
                Instruction::PushConstant(idx) => {
                    // 跟踪常量值
                    // 如果后面紧跟 StoreLocal，记录映射
                    if i + 1 < instructions.len() {
                        if let Instruction::StoreLocal(var) = instructions[i + 1] {
                            constants.insert(var, *idx);
                        }
                    }
                }
                Instruction::LoadLocal(var) => {
                    // 如果这个变量是常量，直接替换为 PushConstant
                    if let Some(idx) = constants.get(var) {
                        instructions[i] = Instruction::PushConstant(*idx);
                    }
                }
                _ => {}
            }
        }
    }
}
```

---

## 五、类型系统实现

### 5.1 类型推断的完整流程

```
1. 收集约束
   ├─ 遍历 AST
   ├─ 为每个表达式分配类型变量
   └─ 生成约束（例如：a + b 推出 a: Int, b: Int, result: Int）

2. 求解约束
   ├─ 使用合一算法
   ├─ 处理类型变量替换
   └─ 检查一致性

3. 应用结果
   ├─ 替换所有类型变量
   ├─ 验证无类型错误
   └─ 生成带类型的 IR
```

### 5.2 泛型处理

```rust
pub struct GenericInstantiation {
    // 泛型函数定义
    generic: Function,
    
    // 类型参数映射
    type_args: HashMap<String, Type>,
}

impl GenericInstantiation {
    pub fn instantiate(&self) -> Function {
        // 1. 创建替换表
        let mut substitutor = TypeSubstitutor::new();
        for (param, arg) in &self.type_args {
            substitutor.add_mapping(param, arg);
        }
        
        // 2. 替换函数签名
        let new_params = self.generic.params.iter()
            .map(|p| substitutor.substitute(&p.ty))
            .collect();
        
        let new_ret_ty = substitutor.substitute(&self.generic.return_ty);
        
        // 3. 替换函数体
        let new_body = self.instantiate_body(&self.generic.body, &substitutor);
        
        Function {
            name: self.generic.name.clone(),
            type_params: vec![],  // 不再是泛型
            params: new_params,
            return_ty: new_ret_ty,
            body: new_body,
        }
    }
}
```

### 5.3 子类型和多态

```rust
// 支持子类型关系
pub enum TypeRelation {
    // T1 是 T2 的子类型
    Subtype(Type, Type),
    
    // T1 和 T2 等价
    Equivalent(Type, Type),
}

impl Subtyping {
    pub fn is_subtype(&self, sub: &Type, sup: &Type) -> bool {
        match (sub, sup) {
            // 相同类型
            (a, b) if a == b => true,
            
            // 底类型是所有类型的子类型
            (Type::Bottom, _) => true,
            
            // 顶类型是所有类型的父类型
            (_, Type::Top) => true,
            
            // 函数的逆变参数，协变返回
            (
                Type::Function { params: p1, ret: r1 },
                Type::Function { params: p2, ret: r2 },
            ) => {
                // 参数逆变：p2 <: p1
                // 返回协变：r1 <: r2
                p2.len() == p1.len() &&
                p2.iter().zip(p1.iter()).all(|(a, b)| self.is_subtype(b, a)) &&
                self.is_subtype(r1, r2)
            }
            
            // 联合类型
            (Type::Union(types), sup) => {
                types.iter().all(|t| self.is_subtype(t, sup))
            }
            
            // 其他情况
            _ => false,
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

pub fn constant_folding(ir: &mut IRModule) {
    for func in ir.functions.iter_mut() {
        for block in func.blocks.iter_mut() {
            let mut constant_values = HashMap::new();
            
            for instr in block.instructions.iter_mut() {
                match instr {
                    Instruction::Const(c) => {
                        // 记录常量
                    }
                    Instruction::Binary { op, left, right, dest } => {
                        if let (Value::Constant(l), Value::Constant(r)) = (left, right) {
                            if let Some(result) = self.compute_const(op, l, r) {
                                *instr = Instruction::Const(result);
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
pub fn inline_functions(&self, ir: &mut IRModule) {
    // 对于小函数，直接内联到调用点
    for func in ir.functions.iter() {
        if func.is_inlineable() && func.size() < INLINE_THRESHOLD {
            // 查找所有调用点
            // 替换为函数体
        }
    }
}
```

#### 6.1.3 循环优化

```rust
// 循环不变量外提
pub fn loop_invariant_code_motion(&self, ir: &mut IRModule) {
    // 识别循环
    // 找出循环中不变的表达式
    // 移到循环前
}

// 循环展开
pub fn loop_unrolling(&self, ir: &mut IRModule) {
    // 对于小循环，可以展开
}
```

### 6.2 运行时优化

#### 6.2.1 内联缓存 (Inline Caches)

```rust
pub struct InlineCache {
    // 缓存类型检查结果
    type_cache: HashMap<VarId, Type>,
    
    // 缓存方法查找
    method_cache: HashMap<(TypeId, String), FuncId>,
}

impl InlineCache {
    pub fn get_method(&mut self, type_id: TypeId, method_name: &str) -> Option<FuncId> {
        let key = (type_id, method_name.to_string());
        
        if let Some(cached) = self.method_cache.get(&key) {
            return Some(*cached);
        }
        
        // 未命中，查找并缓存
        if let Some(func) = self.find_method(type_id, method_name) {
            self.method_cache.insert(key, func);
            Some(func)
        } else {
            None
        }
    }
}
```

#### 6.2.2 JIT 编译（未来扩展）

```rust
// 将热点代码动态编译为机器码
pub struct JITCompiler {
    // 热点检测
    hotness: HashMap<FuncId, u32>,
    
    // 机器码生成
    code_cache: HashMap<FuncId, *const u8>,
}

impl JITCompiler {
    pub fn record_call(&mut self, func_id: FuncId) {
        let count = self.hotness.entry(func_id).or_insert(0);
        *count += 1;
        
        if *count > JIT_THRESHOLD {
            self.compile_to_native(func_id);
        }
    }
}
```

---

## 七、错误处理

### 7.1 错误类型层次

```rust
#[derive(Debug)]
pub enum CompileError {
    // 词法错误
    Lex(LexError),
    
    // 语法错误
    Parse(ParseError),
    
    // 类型错误
    Type(TypeError),
    
    // 代码生成错误
    CodeGen(CodeGenError),
    
    // 语义错误
    Semantic(SemanticError),
}

#[derive(Debug)]
pub enum LexError {
    UnexpectedCharacter(char),
    UnclosedString,
    InvalidNumber,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(Token, String),
    ExpectedIdentifier,
    ExpectedExpression,
    UnclosedDelimiter,
}

#[derive(Debug)]
pub enum TypeError {
    UndefinedVariable(String),
    TypeMismatch,
    ArityMismatch,
    InfiniteType,
    Unimplemented,
    ConstraintUnsatisfied(String),
}

#[derive(Debug)]
pub enum SemanticError {
    DuplicateDefinition(String),
    WrongNumberOfArguments,
    InvalidSpawnContext,
    MutabilityViolation,
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

impl TypeChecker {
    fn error_recovery(&mut self, expr: &Expr) -> Type {
        // 返回未知类型，允许继续检查
        Type::Unknown
    }
}
```

### 7.3 错误报告

```rust
pub struct ErrorReporter {
    source: String,
    diagnostics: Vec<Diagnostic>,
}

impl ErrorReporter {
    pub fn report(&self, error: &CompileError, span: Span) {
        let diagnostic = Diagnostic {
            severity: Severity::Error,
            message: error.to_string(),
            location: self.get_location(span),
            snippet: self.get_snippet(span),
            suggestion: self.get_suggestion(error),
        };
        
        self.diagnostics.push(diagnostic);
    }
    
    pub fn print(&self) {
        for diag in &self.diagnostics {
            println!("{}", diag.format());
        }
    }
}
```

---

## 八、性能考虑

### 8.1 编译器性能

#### 8.1.1 时间复杂度优化
- **词法分析**：O(n) 单次扫描
- **语法分析**：O(n) Pratt Parser
- **类型推断**：O(n * m)，其中 m 是约束数量
- **优化**：可配置的优化级别

#### 8.1.2 空间复杂度优化
- **增量编译**：只重新编译修改的模块
- **缓存**：AST、IR、字节码缓存
- **内存池**：减少分配开销

### 8.2 生成代码性能

#### 8.2.1 字节码设计
- **紧凑编码**：使用变长编码
- **直接操作**：减少间接访问
- **内联缓存**：加速动态分发

#### 8.2.2 虚拟机优化
- **解释器循环**：使用 computed goto 或 switch 优化
- **栈缓存**：热点栈位置
- **JIT 编译**：热点代码生成机器码

### 8.3 并发性能

#### 8.3.1 并作调度
- **工作窃取**：负载均衡
- **任务窃取**：小粒度任务
- **无锁队列**：减少竞争

#### 8.3.2 内存分配
- **线程本地存储**：减少竞争
- **分配器选择**：根据场景选择
- **零拷贝**：尽可能避免复制

---

## 九、总结

YaoXiang 编译器采用现代编译器架构，从前端到后端都有清晰的设计原则：

1. **前端**：Pratt Parser + Hindley-Milner 类型推断
2. **中端**：SSA IR + 多种优化通道
3. **后端**：字节码生成 + 紧凑编码
4. **运行时**：高效的虚拟机 + 并作调度

主要创新点：
- **双层处理**：解析层宽松，类型检查层严格
- **位置绑定**：`[n]` 语法实现细粒度柯里化
- **并作模型**：自动并发，零认知负担
- **类型集中约定**：AI 友好的声明语法

**最后更新**：2025-01-04
