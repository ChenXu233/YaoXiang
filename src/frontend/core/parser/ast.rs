//! Abstract Syntax Tree types

pub use crate::frontend::core::lexer::tokens::Literal;
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
    /// unsafe 块：允许系统级操作
    /// `unsafe { *ptr = ... }`
    Unsafe {
        body: Box<Block>,
        span: Span,
    },
    /// Lambda expression: (params) => body
    /// Used for RFC-007 function syntax: name = (params) => body
    Lambda {
        params: Vec<Param>,
        body: Box<Block>,
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
    /// Dereference: `*ptr`
    Deref,
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
    /// Type definition: `RFC-010: `Name: Type = { ... }`
    TypeDef {
        name: String,
        definition: Type,
        /// RFC-010: Generic type parameters from `Type[T]` or `Type[K, V]`
        generic_params: Vec<String>,
    },
    /// Use statement: `use module.path`
    Use {
        path: String,
        items: Option<Vec<String>>,
        alias: Option<String>,
    },
    /// Function definition: `name: Type = (params) => body`
    /// With pub modifier: `pub name: Type = (params) => body` - auto-binds to first param type
    Fn {
        name: String,
        generic_params: Vec<GenericParam>,
        type_annotation: Option<Type>,
        params: Vec<Param>,
        body: (Vec<Stmt>, Option<Box<Expr>>),
        is_pub: bool, // 是否公开导出并自动绑定到类型
    },
    /// Method binding: `Type.method: (Type, ...) -> ReturnType = (params) => body`
    MethodBind {
        /// 类型名称
        type_name: String,
        /// 方法名称
        method_name: String,
        /// 方法类型（包含 self 参数）
        method_type: Type,
        /// 方法参数（不包含 self）
        params: Vec<Param>,
        /// 方法体
        body: (Vec<Stmt>, Option<Box<Expr>>),
    },
    /// If statement: `if condition { then_branch } elif branches else_branch`
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        elif_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
        span: Span,
    },
}

/// Variant constructor definition (for variant types)
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub params: Vec<(Option<String>, Type)>,
    pub span: Span,
}

/// 结构体字段定义
///
/// 用于表示类型定义中的字段，包含可变性标记
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub is_mut: bool,
    pub ty: Type,
}

impl StructField {
    /// 创建新的结构体字段
    pub fn new(
        name: String,
        is_mut: bool,
        ty: Type,
    ) -> Self {
        Self { name, is_mut, ty }
    }
}

/// Generic parameter kind: Type parameter, Const parameter, or Platform parameter
#[derive(Debug, Clone)]
pub enum GenericParamKind {
    /// Type parameter: [T]
    Type,
    /// Const parameter: [N: Int]
    Const {
        /// The type of the const parameter (e.g., Int)
        const_type: Box<Type>,
    },
    /// Platform parameter: [P] or [P: X86_64]
    /// RFC-011: P is reserved for platform specialization
    Platform,
}

/// Generic parameter with constraints: `[T: Clone]` or `[N: Int]`
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub kind: GenericParamKind,
    pub constraints: Vec<Type>,
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
    Struct(Vec<StructField>),
    NamedStruct {
        name: String,
        fields: Vec<StructField>,
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
    /// 关联类型访问（如 T::Item）
    AssocType {
        /// 宿主类型
        host_type: Box<Type>,
        /// 关联类型名称
        assoc_name: String,
        /// 关联类型参数（如果有关联类型也是泛型的）
        assoc_args: Vec<Type>,
    },
    Sum(Vec<Type>),
    /// Literal type: a compile-time constant value used as a type
    /// e.g., "5" in `[n: Int](n: n)` - n is a literal type "5"
    /// Used for const generics with literal value parameters
    Literal {
        /// The literal name (e.g., "5")
        name: String,
        /// The underlying type (e.g., Int)
        base_type: Box<Type>,
    },
    /// Raw pointer type: `*T`
    /// Only usable inside unsafe blocks
    Ptr(Box<Type>),
    /// Meta-type: `Type` or `Type[T]` or `Type[K, V]`
    /// RFC-010: Used in unified syntax `Name: Type = { ... }`
    /// `Type` is the only meta-type keyword in the language
    /// Supports infinite universe levels: `Type[Type[T]]` → Type2, etc.
    MetaType {
        /// Generic type parameters (empty for plain `Type`)
        /// e.g., `Type[T]` has args = [T], `Type[K, V]` has args = [K, V]
        /// e.g., `Type[Type[T]]` has args = [MetaType { args: [T] }]
        args: Vec<Type>,
    },
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
    pub body: Block,
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
        /// 字段模式列表：(字段名, 是否可变, 模式)
        fields: Vec<(String, bool, Box<Pattern>)>,
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

impl Default for Module {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            span: Span::dummy(),
        }
    }
}
