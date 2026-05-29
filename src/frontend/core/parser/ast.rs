//! Abstract Syntax Tree types

pub use crate::frontend::core::lexer::tokens::Literal;
use crate::util::span::Span;

#[derive(Debug, Clone)]
pub struct SpannedIdent {
    pub name: String,
    pub span: Span,
}

/// Evaluation strategy annotation (RFC-001/008).
///
/// Used by `@block/@auto/@eager` on function signatures or blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvalMode {
    Block,
    Auto,
    Eager,
}

/// Semantic category for unified binding statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingSemanticKind {
    /// `TypeName.method = ...`
    Method,
    /// Type definition / type constructor lowered form.
    TypeConstructor,
    /// Function-like binding (including block sugar and lambda forms).
    Function,
}

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
        /// RFC-010: 命名参数 `Point(x=1, y=2)`
        named_args: Vec<(String, Expr)>,
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
        var_mut: bool, // 变量是否可变
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
    /// Evaluation strategy annotation for a block: `@block { ... }` / `@auto { ... }` / `@eager { ... }`
    Eval {
        mode: EvalMode,
        body: Box<Block>,
        span: Span,
    },
    /// Spawn a concurrent block: `spawn { ... }`
    ///
    /// Only valid inside `@block` scope (enforced by type checker / compiler).
    Spawn {
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
    /// RFC-012: F-string template literal
    /// `f"Hello {name}"` → FString { segments: [Text("Hello "), Interpolation { expr, format_spec }] }
    FString {
        segments: Vec<FStringSegment>,
        span: Span,
    },
    /// 错误恢复占位符：表示解析失败的表达式
    ///
    /// 当解析器遇到无法解析的表达式时，插入此占位符而非 panic。
    /// 类型检查器遇到此节点时应报告错误但不 panic。
    /// 用于 LSP 错误恢复场景。
    Error(Span),
}

/// RFC-012: F-string segment
#[derive(Debug, Clone)]
pub enum FStringSegment {
    /// Plain text fragment
    Text(String),
    /// Interpolation expression with optional format specifier
    Interpolation {
        expr: Box<Expr>,
        format_spec: Option<String>,
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
    /// Variable declaration: `mut` name `:` type `=` expr
    Var {
        name: String,
        /// 变量名的源码位置（用于代码染色等）
        /// Span of the `Type` meta-keyword identifier.
        name_span: Span,
        type_annotation: Option<Type>,
        initializer: Option<Box<Expr>>,
        is_mut: bool,
    },
    /// For loop: `for [mut] item in iterable { body }`
    For {
        var: String,
        /// 变量名的源码位置（用于代码染色等）
        var_span: Span,
        var_mut: bool, // 变量是否可变
        iterable: Box<Expr>,
        body: Box<Block>,
        label: Option<String>,
    },
    /// Unified binding: combines Fn, TypeDef, and MethodBind
    /// Used for RFC-022: Unified binding syntax
    Binding {
        /// Binding name (function name, type name, or method name)
        name: String,
        /// Optional type name for method binding
        type_name: Option<String>,
        /// Method type (for method binding)
        method_type: Option<Type>,
        /// Generic type parameters
        generic_params: Vec<GenericParam>,
        /// Type annotation / return type
        type_annotation: Option<Type>,
        /// Evaluation strategy annotation (`@block/@auto/@eager`) on this function.
        eval: Option<EvalMode>,
        /// Parameters (for functions and methods)
        params: Vec<Param>,
        /// Body: (prelude statements, optional tail expression)
        body: (Vec<Stmt>, Option<Box<Expr>>),
        /// Whether this binding is public
        is_pub: bool,
    },
    /// Use statement: `use module.path` or `use module.{a, b} as c, d`
    Use {
        path: String,
        /// Module path span
        path_span: Span,
        /// Spans of each identifier in the module path (dot-separated)
        path_parts: Vec<SpannedIdent>,
        items: Option<Vec<String>>,
        alias: Option<Vec<String>>,
    },
    /// If statement: `if condition { then_branch } elif branches else_branch`
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        elif_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
        span: Span,
    },
    /// RFC-004: 外部绑定语句: `Type.method = function[positions]`
    ExternalBindingStmt {
        type_name: String,
        method_name: String,
        binding: BindingKind,
    },
    /// 错误恢复占位符：表示解析失败的语句
    ///
    /// 当解析器遇到无法解析的语句时，插入此占位符而非 panic。
    /// 类型检查器遇到此节点时应报告错误但不 panic。
    /// 用于 LSP 错误恢复场景。
    Error(Span),
}

/// Variant constructor definition (for variant types)
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<(Option<String>, Type)>,
    pub span: Span,
}

/// 结构体字段定义
///
/// 用于表示类型定义中的字段，包含可变性标记和可选默认值
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub is_mut: bool,
    pub ty: Type,
    /// 可选的默认值表达式
    pub default: Option<Box<Expr>>,
}

impl StructField {
    /// 创建新的结构体字段（无默认值）
    pub fn new(
        name: String,
        is_mut: bool,
        ty: Type,
    ) -> Self {
        Self {
            name,
            is_mut,
            ty,
            default: None,
        }
    }

    /// 创建带默认值的结构体字段
    pub fn with_default(
        name: String,
        is_mut: bool,
        ty: Type,
        default: Expr,
    ) -> Self {
        Self {
            name,
            is_mut,
            ty,
            default: Some(Box::new(default)),
        }
    }
}

/// 类型体内置绑定
///
/// 在类型定义体内绑定方法到字段
#[derive(Debug, Clone)]
pub struct TypeBodyBinding {
    pub name: String,
    pub kind: BindingKind,
}

/// 绑定方式
#[derive(Debug, Clone)]
pub enum BindingKind {
    /// 外部函数绑定: `name = function[positions]`
    External {
        function: String,
        positions: Vec<i64>,
    },
    /// 匿名函数绑定: `name: ((params) -> Return)[position] = ((params) => body)`
    Anonymous {
        params: Vec<Param>,
        return_type: Box<Type>,
        positions: Vec<i64>,
        body: Box<Expr>,
    },
    /// RFC-004: 默认绑定（无位置）: `name = function`
    /// 自动查找函数参数中第一个类型匹配的位置
    DefaultExternal { function: String },
}

/// Generic parameter kind: Type parameter, Const parameter, or Platform parameter
#[derive(Debug, Clone)]
pub enum GenericParamKind {
    /// Type parameter: `T`
    Type,
    /// Const parameter: [N: Int]
    Const {
        /// The type of the const parameter (e.g., Int)
        const_type: Box<Type>,
    },
    /// Platform parameter: `P` or `P: X86_64`
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
    Name {
        name: String,
        span: Span,
    },
    Int(usize),
    Float(usize),
    Char,
    String,
    Bytes,
    Bool,
    Void,
    Struct {
        fields: Vec<StructField>,
        bindings: Vec<TypeBodyBinding>,
        /// RFC-010: 接口约束列表
        interfaces: Vec<String>,
    },
    NamedStruct {
        name: String,
        name_span: Span,
        fields: Vec<StructField>,
    },
    Union(Vec<(String, Option<Type>)>),
    Enum(Vec<String>),
    /// Variant type: `type Color = red | green | blue` or `type Result = ok(T) | err(E)`
    Variant(Vec<VariantDef>),
    Tuple(Vec<Type>),
    Fn {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Generic {
        name: String,
        name_span: Span,
        args: Vec<Type>,
    },
    /// 关联类型访问（如 T::Item）
    AssocType {
        /// 宿主类型
        host_type: Box<Type>,
        /// 关联类型名称
        assoc_name: String,
        assoc_name_span: Span,
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
        name_span: Span,
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
        /// `Type` 关键字的源码位置
        name_span: Span,
        /// Generic type parameters (empty for plain `Type`)
        /// e.g., `Type[T]` has args = `T`, `Type[K, V]` has args = `K, V`
        /// e.g., `Type[Type[T]]` has args = `MetaType { args: T }`
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
    pub is_mut: bool,
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

/// Returns true if a type is the `Type` meta keyword form.
pub fn is_meta_type(ty: &Type) -> bool {
    matches!(ty, Type::MetaType { .. })
}

/// Returns true if a type annotation semantically returns `Type`.
pub fn type_annotation_returns_meta_type(ty: &Type) -> bool {
    match ty {
        Type::MetaType { .. } => true,
        Type::Fn { return_type, .. } => matches!(return_type.as_ref(), Type::MetaType { .. }),
        _ => false,
    }
}

/// Classify a unified binding into method/type-constructor/function semantics.
///
/// The parser lowers type constructors into bindings with empty params/body and
/// concrete type annotations, so downstream phases can use one stable predicate.
pub fn classify_binding_semantic_kind(
    type_name: Option<&String>,
    type_annotation: Option<&Type>,
    params: &[Param],
    body_stmts: &[Stmt],
    body_expr: Option<&Expr>,
) -> BindingSemanticKind {
    if type_name.is_some() {
        return BindingSemanticKind::Method;
    }

    // 解析器会把类型构造器降级为"空 params + 空 body + concrete type annotation"形态。
    // 例如 `Id: (T: Type) -> Type = { x: T }` 在 AST 中 type_annotation 已是 Struct，
    // 不再保留 `-> Type` 的函数签名，因此这里必须按降级后的形状判断。
    if type_annotation.is_some()
        && params.is_empty()
        && body_stmts.is_empty()
        && body_expr.is_none()
    {
        return BindingSemanticKind::TypeConstructor;
    }

    BindingSemanticKind::Function
}
