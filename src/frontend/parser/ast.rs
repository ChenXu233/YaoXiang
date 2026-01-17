//! Abstract Syntax Tree types

use super::super::lexer::tokens::Literal;
use crate::util::span::Span;

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Literal, Span),
    Var(String, Span),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    FnDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Box<Block>,
        is_async: bool,
        span: Span,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        elif_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
        span: Span,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    While {
        condition: Box<Expr>,
        body: Box<Block>,
        label: Option<String>,
        span: Span,
    },
    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Block>,
        label: Option<String>,
        span: Span,
    },
    Block(Block),
    Return(Option<Box<Expr>>, Span),
    Break(Option<String>, Span),
    Continue(Option<String>, Span),
    Cast {
        expr: Box<Expr>,
        target_type: Type,
        span: Span,
    },
    Tuple(Vec<Expr>, Span),
    List(Vec<Expr>, Span),
    ListComp {
        element: Box<Expr>,           // 元素表达式 x * x
        var: String,                  // 迭代变量名 x
        iterable: Box<Expr>,          // 可迭代对象
        condition: Option<Box<Expr>>, // 过滤条件 if x > 0
        span: Span,
    },
    Dict(Vec<(Expr, Expr)>, Span),
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    FieldAccess {
        expr: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Error propagation operator: `expr?`
    /// Generated when the user writes `expr?` - propagates Err/None upward
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    /// ref 关键字：创建 Arc（原子引用计数）
    /// `shared = ref p` 创建 p 的 Arc 副本
    Ref {
        expr: Box<Expr>,
        span: Span,
    },
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Range,
    Assign,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Pos,
    Not,
}

/// Statement
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

/// Statement kind
#[derive(Debug, Clone)]
pub enum StmtKind {
    Expr(Box<Expr>),
    /// Variable declaration: [mut] name[: type] [= expr]
    Var {
        name: String,
        type_annotation: Option<Type>,
        initializer: Option<Box<Expr>>,
        is_mut: bool,
    },
    /// For loop: `for item in iterable { body }`
    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Block>,
        label: Option<String>,
    },
    TypeDef {
        name: String,
        definition: Type,
    },
    Module {
        name: String,
        items: Vec<Stmt>,
    },
    Use {
        path: String,
        items: Option<Vec<String>>,
        alias: Option<String>,
    },
    /// Function definition: `name: Type = (params) => body`
    Fn {
        name: String,
        type_annotation: Option<Type>,
        params: Vec<Param>,
        body: (Vec<Stmt>, Option<Box<Expr>>),
    },
}

/// Variant constructor definition (for variant types)
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub params: Vec<(Option<String>, Type)>,
    pub span: Span,
}

/// Type
#[derive(Debug, Clone)]
pub enum Type {
    Name(String),
    Int(usize),
    Float(usize),
    Char,
    String,
    Bytes,
    Bool,
    Void,
    Struct(Vec<(String, Type)>),
    NamedStruct {
        name: String,
        fields: Vec<(String, Type)>,
    },
    Union(Vec<(String, Option<Type>)>),
    Enum(Vec<String>),
    /// Variant type: `type Color = red | green | blue` or `type Result = ok(T) | err(E)`
    Variant(Vec<VariantDef>),
    Tuple(Vec<Type>),
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Set(Box<Type>),
    Fn {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Generic {
        name: String,
        args: Vec<Type>,
    },
    Sum(Vec<Type>),
}

/// Block
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>,
    pub span: Span,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub span: Span,
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

/// Pattern
#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Identifier(String),
    Literal(Literal),
    Tuple(Vec<Pattern>),
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    Union {
        name: String,
        variant: String,
        pattern: Option<Box<Pattern>>,
    },
    Or(Vec<Pattern>),
    Guard {
        pattern: Box<Pattern>,
        condition: Expr,
    },
}

/// Module
#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Stmt>,
    pub span: Span,
}
