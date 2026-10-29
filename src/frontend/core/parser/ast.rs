//! Abstract Syntax Tree types

use std::collections::HashSet;

pub use crate::frontend::core::lexer::tokens::Literal;
pub use crate::util::plan_id::PlanId;
use crate::util::span::Span;

use super::probe::{NameKind, TypeEnvProbe};

#[derive(Debug, Clone)]
pub struct SpannedIdent {
    pub name: String,
    pub span: Span,
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
        /// 调用点身份（RFC-039 D20 / WBS 4.5.2）：解析期由 `ParserState`
        /// 分配、随本调用节点走的 `PlanId`——重载决议（`overload_resolutions`）
        /// 的产出与消费读同一个节点上的同一个值，不再靠「Call 的 span 两处
        /// 各自算一遍」的位置巧合。
        ///
        /// 构造一律走 [`Expr::call`]（全仓唯一构造器）；解析器之外的合成调用
        /// 记 [`PlanId::UNPLANNED`]，消费侧不消费未分配身份（见
        /// `OverloadResolutions::get`）。
        plan_id: PlanId,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        else_if_branches: Vec<(Box<Expr>, Box<Block>)>,
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
        span: Span,
    },
    For {
        var: String,
        var_mut: bool, // 变量是否可变
        iterable: Box<Expr>,
        body: Box<Block>,
        span: Span,
    },
    /// spawn for 数据并行循环（RFC-024 §2.4）
    SpawnFor {
        var: String,
        var_mut: bool,
        iterable: Box<Expr>,
        body: Box<Block>,
        span: Span,
    },
    Block(Block),
    Return(Option<Box<Expr>>, Span),
    /// break 语句（#314：Python 风格定案——裸 break，无标签，仅语句位置）
    Break(Span),
    /// continue 语句（#314：裸 continue，无标签）
    Continue(Span),
    Cast {
        expr: Box<Expr>,
        target_type: Type,
        span: Span,
    },
    Tuple(Vec<Expr>, Span),
    List(Vec<Expr>, Span),
    ListComp {
        element: Box<Expr>,                 // 元素表达式 x * x
        generators: Vec<ListCompGenerator>, // 生成器子句，至少一个（parser 保证）
        span: Span,
    },
    Dict(Vec<(Expr, Expr)>, Span),
    /// membership 谓词（#299 §3）：`elem in container` → Bool
    /// 一等霍尔谓词：精化类型阶段作为编译期可证命题的基底
    In {
        elem: Box<Expr>,
        container: Box<Expr>,
        span: Span,
    },
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
    /// Borrow expression: `&expr` or `&mut expr`
    /// Creates a borrow token
    Borrow {
        mutable: bool,
        expr: Box<Expr>,
        span: Span,
    },
    /// unsafe 块：允许系统级操作
    /// `unsafe { *ptr = ... }`
    Unsafe {
        body: Box<Block>,
        span: Span,
    },
    /// Spawn a concurrent block: `spawn { ... }`
    ///
    /// RFC-024: The only parallel primitive in YaoXiang.
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

/// 列表推导式的生成器子句：`for <var> in <iterable> ('if' <condition>)?`
///
/// #401：单个推导式可携带多个子句（`[e for x in a for y in b if c]`），
/// 后续子句的 iterable/condition 可引用先前子句绑定的变量。
#[derive(Debug, Clone)]
pub struct ListCompGenerator {
    pub var: String,                  // 迭代变量名 x
    pub iterable: Box<Expr>,          // 可迭代对象
    pub condition: Option<Box<Expr>>, // 过滤条件 if x > 0
    pub span: Span,
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
    // #285: 位运算/移位（SPEC §2.2 级 7/8）
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
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
    /// 释放计划身份（RFC-039 D20 / WBS 4.5.1）：解析期由 `ParserState`
    /// **单调分配**、编译单元内唯一，随语句节点走。
    ///
    /// 它把 `ReleasePlan` 的键从「span 位置巧合」换成「AST 节点结构身份」：
    /// ownership 把释放点归属到释放边界所在的语句，ir_gen 在语句边界按同一
    /// 身份查表发射 `Instruction::Drop`。不经解析器的合成语句恒为
    /// [`PlanId::UNPLANNED`]（不构成释放计划义务）。
    pub plan_id: PlanId,
}

/// Statement kind
#[derive(Debug, Clone)]
pub enum StmtKind {
    Expr(Box<Expr>),
    /// 赋值语句：target = 值 或 target: 类型 = 值
    /// target 是表达式（Var("x")、FieldAccess(Var("File"), "open")、...）
    /// parser 不区分 target 能否当左值——由 L3 的类型检查判断
    /// type_annotation 是可选的类型标注
    /// value 是可选的初始化表达式（extern 绑定 / 方法绑定有，纯声明无）
    Assign {
        target: Box<Expr>,
        /// 类型标注；函数定义时它承载 `Type::Fn`——形参名与类型同栖在
        /// `Type::Fn.params`（`signature_fn_params` 是取用入口）。
        type_annotation: Option<Type>,
        value: Option<Box<Expr>>,
        is_mut: bool,
        span: Span,
    },
    /// For loop: `for [mut] item in iterable { body }`
    For {
        var: String,
        /// 变量名的源码位置（用于代码染色等）
        var_span: Span,
        var_mut: bool, // 变量是否可变
        iterable: Box<Expr>,
        body: Box<Block>,
    },
    /// 类型定义：`Point: Type = { x: Float, y: Float }`
    /// 或泛型：`Option: (T: Type) -> Type = { some(T) | none }`
    TypeDefinition {
        /// 类型名
        name: String,
        /// 声明签名：`(T: Type) -> Type`（`Point: Type` 记 `MetaType`）。
        /// 泛型形参从 `Type::Fn` 载体取（`signature_fn_params`），与 `Assign`
        /// 的 `type_annotation` 同一表示——不再另存并行形参表。
        type_annotation: Option<Type>,
        /// 解析后的类型体（必有）
        definition: Type,
    },
    /// Use statement: `use module.path` / `use module.{a, b}` /
    /// `use module as m` / `use module.{a as x}`（#245）
    Use {
        path: String,
        /// Module path span
        path_span: Span,
        /// Spans of each identifier in the module path (dot-separated)
        path_parts: Vec<SpannedIdent>,
        /// 花括号导入项（带名源位置，供 LSP 导入项跳转定位绑定）
        items: Option<Vec<SpannedIdent>>,
        /// 位置别名 `use path as m`（带名源位置，同上）
        alias: Option<Vec<SpannedIdent>>,
        /// 花括号内联别名（#245）：与 `items` 逐项对齐，`None` 表示该项无别名。
        /// `use lib.{helper as h}` → items=["helper"], item_aliases=[Some("h")]。
        /// 全项均无别名时为 `None`。
        item_aliases: Option<Vec<Option<SpannedIdent>>>,
    },
    /// If statement: `if condition { then_branch } elif branches else_branch`
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        else_if_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
        span: Span,
    },
    /// 元组解构赋值: `a, b = (1, 2)` 或 `(a, b) = (1, 2)`
    DestructureAssign {
        names: Vec<SpannedIdent>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Return statement: `return expr` or `return`
    Return(Option<Box<Expr>>),
    /// 错误恢复占位符：表示解析失败的语句
    ///
    /// 当解析器遇到无法解析的语句时，插入此占位符而非 panic。
    /// 类型检查器遇到此节点时应报告错误但不 panic。
    /// 用于 LSP 错误恢复场景。
    Error(Span),
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

/// 类型体项 — 类型定义 `{}` 是有序代码块，每项按序求值
///
/// parser 只按语法区分，不认识任何类型族名字。
#[derive(Debug, Clone)]
pub enum TypeBodyItem {
    /// `name: Type` 或 `name: Type = default` — 运行时数据字段
    Field(StructField),
    /// `name = function[pos]` 或 `name = function` 或匿名函数绑定 — 保留现有 TypeBodyBinding 语义
    Binding(TypeBodyBinding),
    /// 接口约束名 `InterfaceName`
    Interface(String),
    /// 匿名类型表达式 `Assert(N > 0)` / `SomeProofType(t)` — 结果未命名
    Expr(Type),
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

/// Generic parameter kind: Type parameter or Const parameter
#[derive(Debug, Clone)]
pub enum GenericParamKind {
    /// Type parameter: `T`
    Type,
    /// Const parameter: [N: Int]
    Const {
        /// The type of the const parameter (e.g., Int)
        const_type: Box<Type>,
    },
}

/// Generic parameter with constraints: `[T: Clone]` or `[N: Int]`
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub kind: GenericParamKind,
    pub constraints: Vec<Type>,
}
/// Generic parameter with name and constraints only (no Type/Const classification).
/// Used by consumers that only need structural info, not kind dispatch.
#[derive(Debug, Clone)]
pub struct GenericParamName {
    pub name: String,
    pub constraints: Vec<Type>,
}

/// Type
#[derive(Debug, Clone)]
pub enum Type {
    Name {
        name: String,
        span: Span,
    },
    Struct {
        /// 有序类型体项（字段/绑定/接口/表达式按源码顺序）
        body: Vec<TypeBodyItem>,
    },
    NamedStruct {
        name: String,
        name_span: Span,
        fields: Vec<StructField>,
    },
    Tuple(Vec<Type>),
    /// 函数类型：`(a: Int, b: Int) -> Ret`。
    ///
    /// `params` 复用 [`Param`]——名字与类型同栖（RFC-039 6.8.1）：
    /// 签名位置（`parse_fn_type_with_names`）与 lambda 头一样带真实名字；
    /// 纯类型位置/类型反解没有名字可写时用 [`Param::anonymous`] 合成
    /// （`name` 空串，`is_named()` 为 false）。**不得**再引入并行的
    /// 名字数组（如 `param_names: Vec<Option<String>>`）。
    Fn {
        params: Vec<Param>,
        return_type: Box<Type>,
    },
    Generic {
        name: String,
        name_span: Span,
        args: Vec<Type>,
    },
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
    /// Reference type: `&T` (immutable) or `&mut T` (mutable)
    /// Borrow token — zero-sized compile-time type
    Ref {
        mutable: bool,
        inner: Box<Type>,
        span: Span,
    },
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
    /// 编译期表达式（泛型参数位置的值表达式，如 Assert(N > 0) 中的 N > 0）
    ConstExpr(Box<Expr>),
    /// 括号类型：`(T)` —— **完整类型，链条终止**（RFC-004 柯里化语义）。
    ///
    /// 仅在「括号未构成函数参数组」时产生，即括号后不跟 `->`。因此：
    ///
    /// - `(a: Int) -> Int` 参数组：内层 `(...)` 后跟 `->` → **不**产生 `Paren`
    /// - `() -> ((a: Int) -> Int)` 返回位置裸括号 → **产生** `Paren`
    ///
    /// `split_curry` 遇 `Paren` 即停止拆层：括号声明“这是一个完整类型”，
    /// 它是函数的**值**，不是下一个参数组。
    ///
    /// 其余环节（类型检查 / 单态化 / 解释器）视 `Paren` 为透明，递归内层即可。
    Paren(Box<Type>),
    /// **具名括号**：`(r: P(...))` —— 只有一个具名参数、且括号后不跟 `->`。
    ///
    /// RFC-027 §3 的返回位精化靠它声明**返回形式参数名**（这里叫 binder）：
    ///
    /// - `f: (b: Int) -> (r: IsPositive(r + 1))` 的 `r` 就是 binder——
    ///   它「仅存在于类型签名中、仅被谓词引用，**不进入函数体作用域**」，
    ///   由 `return` 的值供给。故解析层必须留下这个名字：丢掉它，类型检查器
    ///   只能猜「约束里不在作用域的自由变量即返回值形参」，于是
    ///   `(r: P(m))` 会把未声明的 `m` 也当成 binder 静默代入（实测缺陷）。
    /// - 语义与 `Paren` 同构：`MonoType` 层透明（只递归内层）、
    ///   `split_curry` 到此终止（括号声明“这是一个完整类型”），只多携带 binder 名。
    ///
    /// 参数位的形参名仍照旧丢弃（由 `parse_fn_type_with_names` 的 `Param` 承载），
    /// 本节点只用于类型位置的单具名括号形态。
    NamedParen {
        /// 声明的具名（返回形式参数名）
        param: String,
        /// 名字的源码位置
        param_span: Span,
        /// 内层类型（谓词应用）
        inner: Box<Type>,
    },
}

impl Type {
    /// RFC-004 括号语义：`self` 是否为「括号声明的完整类型」（`Paren` / `NamedParen`）。
    ///
    /// 括号 = 柯里链条终止 = 该类型是函数的**值**，不是下一层参数组。
    /// 两处共用同一判定，防止判据漂移：
    /// - parser 的 paren_return 包装（`parser/statements/declarations.rs`）；
    /// - typecheck 的值级分解（`typecheck/inference/statements.rs`，issue #501）。
    pub fn is_paren_complete(&self) -> bool {
        matches!(self, Type::Paren(_) | Type::NamedParen { .. })
    }

    /// 从 Type::Struct 的 body 中提取字段引用（顺序保留）
    pub fn struct_fields(&self) -> Vec<&StructField> {
        match self {
            Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Field(f) = it {
                        Some(f)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 提取接口约束名引用
    pub fn struct_interfaces(&self) -> Vec<&String> {
        match self {
            Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Interface(s) = it {
                        Some(s)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 提取绑定引用
    pub fn struct_bindings(&self) -> Vec<&TypeBodyBinding> {
        match self {
            Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Binding(b) = it {
                        Some(b)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 收集类型树中出现的名字引用（类型名/绑定函数名/编译期表达式的标识符）。
    ///
    /// 供死代码分析（可达性）与未使用导入检测共享：
    /// 类型引用只出现在注解/类型体位置，普通表达式遍历看不到它们。
    /// 保守策略——拿不准的名字一律收集（宁可漏报也不误报死代码/未用导入）。
    pub fn collect_name_refs(
        &self,
        out: &mut HashSet<String>,
    ) {
        match self {
            Type::Name { name, .. } => {
                out.insert(name.clone());
            }
            Type::Struct { body } => {
                for item in body {
                    collect_type_body_item_refs(item, out);
                }
            }
            Type::NamedStruct { fields, .. } => {
                for f in fields {
                    f.ty.collect_name_refs(out);
                    if let Some(default) = &f.default {
                        collect_expr_ident_refs(default, out);
                    }
                }
            }
            Type::Tuple(items) => {
                for ty in items {
                    ty.collect_name_refs(out);
                }
            }
            Type::Fn {
                params,
                return_type,
            } => {
                for p in params {
                    if let Some(ty) = &p.ty {
                        ty.collect_name_refs(out);
                    }
                }
                return_type.collect_name_refs(out);
            }
            Type::Ptr(inner) => inner.collect_name_refs(out),
            Type::Generic { name, args, .. } => {
                out.insert(name.clone());
                for ty in args {
                    ty.collect_name_refs(out);
                }
            }
            Type::Literal { base_type, .. } => base_type.collect_name_refs(out),
            Type::Ref { inner, .. } => inner.collect_name_refs(out),
            Type::MetaType { args, .. } => {
                for ty in args {
                    ty.collect_name_refs(out);
                }
            }
            Type::ConstExpr(expr) => collect_expr_ident_refs(expr, out),
            _ => {}
        }
    }
}

/// 收集类型体项（字段/绑定/接口/匿名类型表达式）中的名字引用
fn collect_type_body_item_refs(
    item: &TypeBodyItem,
    out: &mut HashSet<String>,
) {
    match item {
        TypeBodyItem::Field(f) => {
            f.ty.collect_name_refs(out);
            if let Some(default) = &f.default {
                collect_expr_ident_refs(default, out);
            }
        }
        TypeBodyItem::Binding(b) => collect_binding_kind_refs(&b.kind, out),
        TypeBodyItem::Expr(ty) => ty.collect_name_refs(out),
        TypeBodyItem::Interface(_) => {}
    }
}

/// 收集类型体绑定（外部/匿名/默认绑定）中的名字引用
fn collect_binding_kind_refs(
    kind: &BindingKind,
    out: &mut HashSet<String>,
) {
    match kind {
        BindingKind::External { function, .. } | BindingKind::DefaultExternal { function } => {
            out.insert(function.clone());
        }
        BindingKind::Anonymous {
            params,
            return_type,
            body,
            ..
        } => {
            for p in params {
                if let Some(ty) = &p.ty {
                    ty.collect_name_refs(out);
                }
            }
            return_type.collect_name_refs(out);
            collect_expr_ident_refs(body, out);
        }
    }
}

/// 收集表达式中出现的标识符（仅 Var 叶子，不深入求值语义）。
///
/// 用于 ConstExpr 等表达式位置的名字保守收集；
/// 完整的引用分析在 L3 的类型检查与死代码分析各自的遍历中完成。
fn collect_expr_ident_refs(
    expr: &Expr,
    out: &mut HashSet<String>,
) {
    match expr {
        Expr::Var(name, _) => {
            out.insert(name.clone());
        }
        Expr::Call { func, args, .. } => {
            collect_expr_ident_refs(func, out);
            for arg in args {
                collect_expr_ident_refs(arg, out);
            }
        }
        Expr::FieldAccess { expr, .. } => collect_expr_ident_refs(expr, out),
        Expr::BinOp { left, right, .. } => {
            collect_expr_ident_refs(left, out);
            collect_expr_ident_refs(right, out);
        }
        Expr::UnOp { expr, .. } => collect_expr_ident_refs(expr, out),
        Expr::Cast { expr, .. } => collect_expr_ident_refs(expr, out),
        Expr::Index { expr, index, .. } => {
            collect_expr_ident_refs(expr, out);
            collect_expr_ident_refs(index, out);
        }
        Expr::Tuple(items, _) | Expr::List(items, _) => {
            for item in items {
                collect_expr_ident_refs(item, out);
            }
        }
        Expr::Dict(fields, _) => {
            for (key, value) in fields {
                collect_expr_ident_refs(key, out);
                collect_expr_ident_refs(value, out);
            }
        }
        _ => {}
    }
}

/// Block
///
/// All blocks use `return` for return values. No trailing expression semantics.
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Function parameter（形参：名字与类型同栖）
///
/// 三类使用点共用同一个结构（RFC-039 6.8.1）：lambda 头、函数签名的
/// `Type::Fn.params`、以及需要形参名/类型对的语义消费点。**唯一**的形参
/// 表示——不得新增 Parameter/Arg 第二结构，也不得用
/// `param_names: Vec<Option<String>>` 一类平行数组过渡。
///
/// 名字约定：有名字的位置（签名/lambda）`name` 是真实标识符；纯类型位置
/// （无名实参、类型反解合成）用 [`Param::anonymous`]，`name` 为空串，
/// 由 [`Param::is_named`] 判定。空串名字不参与任何按名查找。
#[derive(Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub is_mut: bool,
    pub span: Span,
}

impl Param {
    /// 合成**无名形参**：只有类型可写的位置（类型位置、类型反解）使用。
    pub fn anonymous(
        ty: Option<Type>,
        span: Span,
    ) -> Self {
        Self {
            name: String::new(),
            ty,
            is_mut: false,
            span,
        }
    }

    /// 由类型合成无名形参（span 取 dummy：合成形的源码位置不存在）。
    pub fn from_type(ty: Type) -> Self {
        Self::anonymous(Some(ty), Span::dummy())
    }

    /// 是否携带真实名字（合成/类型位形参为 `false`）。
    pub fn is_named(&self) -> bool {
        !self.name.is_empty()
    }
}

/// `Debug` 手写（不 derive）：**无名形参**按其所承载的 `Type` 打印——与
/// RFC-039 6.8.1 重塑前 `Type::Fn.params: Vec<Type>` 的 Debug 文本逐字一致。
/// 规范化 IR 快照的 `type_decl` 行打印 AST 类型的 Debug 文本（
/// `middle/core/normalize.rs`），纯表示重塑不得让快照文本漂移；有名形参
/// 按完整结构打印（名字是语义，必须可见）。
impl std::fmt::Debug for Param {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match (&self.ty, self.is_named(), self.is_mut) {
            (Some(ty), false, false) => std::fmt::Debug::fmt(ty, f),
            _ => f
                .debug_struct("Param")
                .field("name", &self.name)
                .field("ty", &self.ty)
                .field("is_mut", &self.is_mut)
                .field("span", &self.span)
                .finish(),
        }
    }
}

impl Type {
    /// 剥掉**类型位形参名**（纯语法装饰）后的副本：`Type::Fn.params` 的名字
    /// 归零并递归全树；`NamedParen.param`（返回位 binder）与匿名函数绑定的
    /// 值级形参名是语义，原样保留。
    ///
    /// 用途：IR 的 `FunctionBody::TypeDecl` 仍以 `ast::Type` 为载体（载体迁移
    /// 属 IR 形状议题，见 09 的 6.5.1 输入项 ②），而规范化 IR 快照按 `Debug`
    /// 文本比对——把名字带进 IR 会让纯表示重塑（6.8.1）引发已入库快照漂移。
    /// IR 不消费形参名，故入 IR 前归一为匿名形参。
    pub fn without_fn_param_names(&self) -> Type {
        fn strip_params(params: &[Param]) -> Vec<Param> {
            params
                .iter()
                .map(|p| Param {
                    name: String::new(),
                    ty: p.ty.as_ref().map(|t| t.without_fn_param_names()),
                    is_mut: p.is_mut,
                    span: p.span,
                })
                .collect()
        }
        fn strip_field(f: &StructField) -> StructField {
            StructField {
                name: f.name.clone(),
                is_mut: f.is_mut,
                ty: f.ty.without_fn_param_names(),
                default: f.default.clone(),
            }
        }
        fn strip_fields(fields: &[StructField]) -> Vec<StructField> {
            fields.iter().map(strip_field).collect()
        }
        fn strip_body(body: &[TypeBodyItem]) -> Vec<TypeBodyItem> {
            body.iter()
                .map(|item| match item {
                    TypeBodyItem::Field(f) => TypeBodyItem::Field(strip_field(f)),
                    TypeBodyItem::Binding(b) => TypeBodyItem::Binding(TypeBodyBinding {
                        name: b.name.clone(),
                        kind: match &b.kind {
                            // 值级形参名是语义（不是类型位装饰）：保留
                            BindingKind::Anonymous {
                                params,
                                return_type,
                                positions,
                                body,
                            } => BindingKind::Anonymous {
                                params: params.clone(),
                                return_type: Box::new(return_type.without_fn_param_names()),
                                positions: positions.clone(),
                                body: body.clone(),
                            },
                            other => other.clone(),
                        },
                    }),
                    TypeBodyItem::Interface(name) => TypeBodyItem::Interface(name.clone()),
                    TypeBodyItem::Expr(t) => TypeBodyItem::Expr(t.without_fn_param_names()),
                })
                .collect()
        }
        match self {
            Type::Struct { body } => Type::Struct {
                body: strip_body(body),
            },
            Type::NamedStruct {
                name,
                name_span,
                fields,
            } => Type::NamedStruct {
                name: name.clone(),
                name_span: *name_span,
                fields: strip_fields(fields),
            },
            Type::Tuple(types) => {
                Type::Tuple(types.iter().map(|t| t.without_fn_param_names()).collect())
            }
            Type::Fn {
                params,
                return_type,
            } => Type::Fn {
                params: strip_params(params),
                return_type: Box::new(return_type.without_fn_param_names()),
            },
            Type::Generic {
                name,
                name_span,
                args,
            } => Type::Generic {
                name: name.clone(),
                name_span: *name_span,
                args: args.iter().map(|t| t.without_fn_param_names()).collect(),
            },
            Type::Literal {
                name,
                name_span,
                base_type,
            } => Type::Literal {
                name: name.clone(),
                name_span: *name_span,
                base_type: Box::new(base_type.without_fn_param_names()),
            },
            Type::Ptr(inner) => Type::Ptr(Box::new(inner.without_fn_param_names())),
            Type::Ref {
                mutable,
                inner,
                span,
            } => Type::Ref {
                mutable: *mutable,
                inner: Box::new(inner.without_fn_param_names()),
                span: *span,
            },
            Type::MetaType { name_span, args } => Type::MetaType {
                name_span: *name_span,
                args: args.iter().map(|t| t.without_fn_param_names()).collect(),
            },
            Type::Paren(inner) => Type::Paren(Box::new(inner.without_fn_param_names())),
            // NamedParen.param 是返回位 binder（语义），不剥
            Type::NamedParen {
                param,
                param_span,
                inner,
            } => Type::NamedParen {
                param: param.clone(),
                param_span: *param_span,
                inner: Box::new(inner.without_fn_param_names()),
            },
            Type::Name { .. } | Type::ConstExpr(_) => self.clone(),
        }
    }

    /// 从 `Type::Fn` 链拍平出全部形参（curry 各组依次展开；`Paren`/
    /// `NamedParen` 视为透明继续内层）。
    ///
    /// 这是「语句声明里的原始签名形参」的**唯一实现**（RFC-039 6.5.2）：
    /// `Assign`/`TypeDefinition` 不再另存形参表，消费点一律经本方法取；
    /// 非函数类型返回空。`NamedParen` 的 binder 名不在此列——它由该节点
    /// 自身承载、由类型检查器显式解释（返回位精化）。
    pub fn fn_params_flat(&self) -> Vec<Param> {
        fn walk(
            ty: &Type,
            out: &mut Vec<Param>,
        ) {
            match ty {
                Type::Fn {
                    params,
                    return_type,
                } => {
                    out.extend(params.iter().cloned());
                    walk(return_type, out);
                }
                Type::Paren(inner) | Type::NamedParen { inner, .. } => walk(inner, out),
                _ => {}
            }
        }
        let mut out = Vec::new();
        walk(self, &mut out);
        out
    }
}

/// 语句的类型标注 → 拍平的签名形参（`Option` 形态的唯一入口）。
///
/// 非 `Type::Fn` 标注（普通值绑定、`Point: Type` 的元类型标注）返回空。
pub fn signature_fn_params(type_annotation: Option<&Type>) -> Vec<Param> {
    type_annotation
        .map(Type::fn_params_flat)
        .unwrap_or_default()
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

/// 可作 **const 泛型参数**的内置类型名（`(N: Int)` 的 `Int`）。
///
/// 语义 = 「能被内置名表解析」（`NameKind::Builtin`）∩「const 参数可接受的
/// 子集」。本表是**子集**的来源，全集在 L3 的内置名解析表
/// （`MonoType::from_builtin_name`）——两份知识在 `extract_generic_param_names`
/// 里显式接线（WBS 6.4.5），并由 `tools/type-tables` 的 T2 门禁逐名对账
/// （C1：本表 ⊆ 内置名表）。
///
/// 6.4 删掉了 10 个旧拼写（`I8` / `I16` / `I32` / `I64` / `U8` /
/// `U16` / `U32` / `U64` / `F32` / `F64`）：它们是 6.1 已删除的
/// 实现内拼写，留在表里会产生**无法 lower** 的 const 参数名（Uint 族在
/// `MonoType` 里没有变体，位宽缩写也不是内置名）。规范名（`Int8` …）不进本表
/// ——它们自始不在 const 参数的可接受集合里，加入会拓宽泛型参数登记，属行为变更。
pub const CONST_PARAM_TYPES: &[&str] = &["Int", "Bool", "Float", "Char", "String"];

/// 参数名是否在给定类型中被当作**类型引用**（`(N: Int) -> (n: N)` 的 `N`）。
///
/// **单一实现**（RFC-039 P6 6.4 / 03 §5.3）：本函数是原先两份同义实现
/// （本文件的 `name_used_as_type_in` 与 `statements/declarations.rs` 的
/// `name_used_as_type`）合并后的**唯一**版本，后者已改为调用本函数。
/// 合并口径 = 本文件版本（较新、修复更全）叠加上另一版的**谓词短路**：
///
/// - `Generic` 的**头部**同样是类型引用（`F(A)` 的 `F` 是构造器变量）——
///   本文件版本有、另一版没有；
/// - `Struct` / `NamedStruct` 的字段类型递归——本文件版本有、另一版落
///   `_ => false`；
/// - **谓词应用的实参位是值**（`Terminates(b)` 的 `b`），故头名被判定为谓词时
///   整个应用短路 false——另一版有、本文件版本缺（RFC-027a）。
///
/// `is_predicate` 是调用方注入的环境问答：parser 侧传「探针 ∪ 本遍已声明谓词」，
/// L3 侧传语义探针。两边都不是字符串常量表。
///
/// **已知共同缺口（本批刻意不动）**：`MetaType` / `ConstExpr` / `Paren` /
/// `NamedParen` 四个变体两版原先都落 `_ => false`。补它们会改变
/// 「const 参数是否被当类型用」的判定（进而改 IR 的 generic_params 与语料行为），
/// 属语义修正，须按 07 的分流判据另立一轮，不在 6.4 的 C3 面内。
pub fn name_used_as_type(
    name: &str,
    ty: &Type,
    is_predicate: &dyn Fn(&str) -> bool,
) -> bool {
    match ty {
        Type::Name { name: n, .. } => n == name,
        Type::Literal { name: n, .. } => n == name,
        // 头部位置同样是类型引用：`F(A)` 的应用头部 `F` 指的是**构造器变量**
        // （高阶类型参数，RFC-010 元类型层级的 Type1/Type2+ 值）——与 `F` 单独
        // 出现在类型位同义。此前只下钻实参位，头部被漏掉，于是类型体内的
        // `field: F(A)` 判不出 F 被当类型用，注解校验误报 E1003。
        //
        // 谓词应用反过来：头名是编译期谓词时**实参位是值**（`Terminates(b)`），
        // 整个应用不构成类型引用。
        Type::Generic {
            name: head, args, ..
        } => {
            if is_predicate(head) {
                return false;
            }
            head == name
                || args
                    .iter()
                    .any(|a| name_used_as_type(name, a, is_predicate))
        }
        Type::Fn {
            params,
            return_type,
        } => {
            params.iter().any(|p| {
                p.ty.as_ref()
                    .is_some_and(|t| name_used_as_type(name, t, is_predicate))
            }) || name_used_as_type(name, return_type, is_predicate)
        }
        Type::Ptr(inner) => name_used_as_type(name, inner, is_predicate),
        Type::Ref { inner, .. } => name_used_as_type(name, inner, is_predicate),
        Type::Tuple(types) => types
            .iter()
            .any(|t| name_used_as_type(name, t, is_predicate)),
        // 类型定义体：字段类型里的引用也算（`SafeArray = { data: Array(T, N) }`
        // 的 `N`）。此前缺这两个分支，const 泛型参数在定义体内被引用时
        // 判不出「被当类型用」，注解校验会把合法 const 参数误报未知名。
        Type::Struct { body } => body.iter().any(|item| match item {
            TypeBodyItem::Field(f) => name_used_as_type(name, &f.ty, is_predicate),
            TypeBodyItem::Expr(e) => name_used_as_type(name, e, is_predicate),
            _ => false,
        }),
        Type::NamedStruct { fields, .. } => fields
            .iter()
            .any(|f| name_used_as_type(name, &f.ty, is_predicate)),
        _ => false,
    }
}

/// 标注是否为**类型级函数**：`(T: Type) -> Type` —— 参数位与返回位都是 `Type`。
///
/// 这是高阶类型参数（构造器变量）的声明形态：F 的取值范围是类型构造器，
/// 而非某个具体类型（RFC-010 元类型层级 Type1/Type2+）。
pub fn is_type_level_fn(
    params: &[Param],
    return_type: &Type,
) -> bool {
    fn is_type(t: &Type) -> bool {
        match t {
            Type::MetaType { .. } => true,
            Type::Name { name, .. } => name == "Type",
            _ => false,
        }
    }
    !params.is_empty()
        && is_type(return_type)
        && params.iter().all(|p| p.ty.as_ref().is_some_and(is_type))
}

/// 从签名形参里抽出**结构可判定**的泛型参数（名字 + 约束）。
///
/// 三类来源：`Type`/`MetaType` 标注、`CONST_PARAM_TYPES` 里的内置类型名、
/// 约束名（探针判定的运算符接口）、类型级函数标注（构造器变量）。其余
/// `Name` 标注（用户类型的值参数）保守跳过——trait 表不在 L2，带 trait 约束的
/// 分类由 L3 的 `classify_generic_params` 另做。
pub fn extract_generic_param_names(
    params: &[Param],
    probe: &dyn TypeEnvProbe,
) -> Vec<GenericParamName> {
    params
        .iter()
        .filter_map(|p| {
            let ty = p.ty.as_ref()?;
            match ty {
                // (T: Type) — MetaType
                Type::MetaType { .. } => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: Vec::new(),
                }),
                Type::Name { name, .. } if name == "Type" => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: Vec::new(),
                }),
                // `(N: Int)` 是 const 泛型，但 `(a: Int)` 是普通值参数——
                // 两者类型标注都是 `Int`，仅凭类型无法区分。
                // 判据：参数名是否在**同一签名**的其它位置被当作类型引用
                //（如 `f: (N: Int) -> (n: N) -> Int` 的 `N`）。
                // 不做此判别会把 `plain: (a: Int) -> Int` 误标为泛型函数，
                // 单态化据此将其删除且永不重建（实测 E6006，#351）。
                // 双知识接线（WBS 6.4.5）：名字必须在 const 参数可接受集合内，**且**
                // 由探针确认为内置名（权威表在 L3 的 `from_builtin_name`）。
                // 只查本表的字符串比较会绕开探针的定种；只查探针则会把 `Int8` 等
                // 规范名也当 const 参数候选（拓宽登记 = 行为变更）。
                //
                // **不持有内置名表的探针回退单表判据**（6.4 修复轮）：L2 的默认探针
                // （BuiltinProbe / NullProbe）拿不到类型表，`kind_of` 永不返回
                // `Builtin`——若不放行，默认入口对 `(N: Int)` 会静默漏登记，与 6.4
                // 之前不一致。`knows_builtin_types() == false` 时按 `CONST_PARAM_TYPES`
                // 单表判定（= 6.4 前行为）；L3 语义探针持有名表，双知识仍全强度生效。
                Type::Name { name, .. }
                    if CONST_PARAM_TYPES.contains(&name.as_str())
                        && (probe.kind_of(name) == NameKind::Builtin
                            || !probe.knows_builtin_types()) =>
                {
                    let used_as_type = params.iter().any(|q| {
                        q.ty.as_ref().is_some_and(|qt| {
                            name_used_as_type(&p.name, qt, &|n| {
                                probe.kind_of(n) == NameKind::Predicate
                            })
                        })
                    });
                    if used_as_type {
                        Some(GenericParamName {
                            name: p.name.clone(),
                            constraints: Vec::new(),
                        })
                    } else {
                        None
                    }
                }
                // RFC-011 §5.2：约束形参（`T: Add`——标注为约束名）。
                // 是不是约束名由注入探针判定（权威表在 L3 的运算符接口表）；
                // 其余 Name（用户类型标注的值参数）仍保守跳过。
                Type::Name { name, .. } if probe.is_constraint(name) => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: vec![ty.clone()],
                }),
                // 高阶类型参数（构造器变量）：标注是类型级函数 `(T: Type) -> Type`。
                // 与 `classify_generic_params` 同一判据（is_type_level_fn），否则
                // 声明侧把它当类型参数、IR 侧却少登记一个泛型参数——单态化请求的
                // 类型实参数与 IR 的 generic_params 对不上，直接 E3018。
                Type::Fn {
                    params,
                    return_type,
                } if is_type_level_fn(params, return_type) => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: Vec::new(),
                }),
                Type::Name { .. } => {
                    // 无法确认是否为 trait → 保守不下泛型参数
                    None
                }
                // (T: Clone + Add) — 无法确认所有元素都是约束 → 保守忽略（L3 类型检查负责）
                Type::Tuple(_types) => None,
                _ => None,
            }
        })
        .collect()
}

/// 用 trait_table 判定泛型参数分类（替代 extract_generic_params 的旧大小写猜测）。
///
/// MetaType 或 Name("Type") → Type 参数（无约束）；
/// CONST_PARAM_TYPES 原始类型名 → Const 参数；
/// Name 且 `is_trait(name)` → Type 参数（约束=该 annotation）；
/// Tuple 且所有元素都是 trait → Type 参数（多约束）；
/// 其余 → 跳过（值参数）。
pub fn classify_generic_params(
    params: &[Param],
    is_trait: &dyn Fn(&str) -> bool,
) -> Vec<GenericParam> {
    params
        .iter()
        .filter_map(|p| {
            let ty = p.ty.as_ref()?;
            match ty {
                // (T: Type) — MetaType
                Type::MetaType { .. } => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: Vec::new(),
                }),
                Type::Name { name, .. } if name == "Type" => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: Vec::new(),
                }),
                Type::Name { name, .. } if CONST_PARAM_TYPES.contains(&name.as_str()) => {
                    Some(GenericParam {
                        name: p.name.clone(),
                        kind: GenericParamKind::Const {
                            const_type: Box::new(ty.clone()),
                        },
                        constraints: Vec::new(),
                    })
                }
                Type::Name { name, .. } if is_trait(name) => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: vec![ty.clone()],
                }),
                // 高阶类型参数（构造器变量）：标注是**类型级函数** `(T: Type) -> Type`——
                // 参数位与返回位都是 `Type`。F 的取值范围是类型构造器（RFC-010
                // 元类型层级 Type1/Type2+），故按类型参数登记，而不是值参数。
                Type::Fn {
                    params,
                    return_type,
                } if is_type_level_fn(params, return_type) => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: Vec::new(),
                }),
                // annotation 为具体类型（非 trait、非 CONST_PARAM_TYPES）→ 非泛型参数
                Type::Name { .. } => None,
                Type::Tuple(types) => {
                    let all_traits = types.iter().all(|t| match t {
                        Type::Name { name, .. } => is_trait(name),
                        _ => false,
                    });
                    if all_traits && !types.is_empty() {
                        Some(GenericParam {
                            name: p.name.clone(),
                            kind: GenericParamKind::Type,
                            constraints: types.clone(),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect()
}

impl Expr {
    /// 调用表达式的**唯一构造器**（RFC-039 D20 / WBS 4.5.2）。
    ///
    /// 全仓调用点一律经此构造：`func` 收 `Expr` 或 `Box<Expr>` 皆可（`Box` 有
    /// 恒等 `Into`）。`plan_id` 由解析器分配（`ParserState::alloc_plan_id`）；
    /// 解析器之外的合成调用（常量求值、IR 内建、测试夹具）记
    /// [`PlanId::UNPLANNED`]。
    pub fn call(
        func: impl Into<Box<Expr>>,
        args: Vec<Expr>,
        named_args: Vec<(String, Expr)>,
        span: Span,
        plan_id: PlanId,
    ) -> Expr {
        Expr::Call {
            func: func.into(),
            args,
            named_args,
            span,
            plan_id,
        }
    }

    /// 表达式的源码 span（穷举变体，新增变体时编译器强制补全）
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit(_, s)
            | Expr::Var(_, s)
            | Expr::Return(_, s)
            | Expr::Break(s)
            | Expr::Continue(s) => *s,
            Expr::BinOp { span, .. }
            | Expr::UnOp { span, .. }
            | Expr::Call { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::While { span, .. }
            | Expr::For { span, .. }
            | Expr::SpawnFor { span, .. }
            | Expr::Borrow { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::Index { span, .. }
            | Expr::Tuple(_, span)
            | Expr::List(_, span)
            | Expr::Cast { span, .. }
            | Expr::Try { span, .. } => *span,
            Expr::Block(block) => block.span,
            Expr::ListComp { span, .. }
            | Expr::Dict(_, span)
            | Expr::In { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Ref { span, .. }
            | Expr::Unsafe { span, .. }
            | Expr::Spawn { span, .. }
            | Expr::FString { span, .. }
            | Expr::Error(span) => *span,
        }
    }

    /// 解构 Assign 目标：`x` → (x, None)；`Type.field` → (field, Some(Type))；其他 → None
    pub fn receiver_parts(&self) -> Option<(String, Option<String>)> {
        match self {
            Expr::Var(n, _) => Some((n.clone(), None)),
            Expr::FieldAccess { expr, field, .. } => {
                if let Expr::Var(tn, _) = expr.as_ref() {
                    Some((field.clone(), Some(tn.clone())))
                } else {
                    Some((field.clone(), None))
                }
            }
            _ => None,
        }
    }

    /// 解构函数值：Lambda → (params, body)；Block → (空, body)；其他 → (空, 空)
    pub fn callable_parts(&self) -> (Vec<Param>, Vec<Stmt>) {
        match self {
            Expr::Lambda { params, body, .. } => (params.clone(), body.stmts.clone()),
            Expr::Block(block) => (Vec::new(), block.stmts.clone()),
            _ => (Vec::new(), Vec::new()),
        }
    }

    /// `name = <expr>` 中，`mut` 绑定是否为类型/方法的**元绑定**——
    /// 这类左值不引入变量，不适用 spec §4.3 的声明/赋值判定。
    ///
    /// 两种形态：
    /// - **类型定义绑定**（RFC-010）：`Db = unsafe { Db: Type = {...}; Db }`
    ///   块内定义类型、尾表达式交回类型名——编译期构造，无运行时绑定。
    /// - 调用方自有的其他形态（如 `Type.method = f` 方法绑定）由调用方判定。
    pub fn is_type_def_binding(value: Option<&Expr>) -> bool {
        let Some(Expr::Unsafe { body, .. }) = value else {
            return false;
        };
        // 块内**直接**包含类型定义，且尾表达式引用该类型名
        let mut def_names: Vec<&str> = Vec::new();
        for st in &body.stmts {
            if let crate::frontend::core::parser::ast::StmtKind::TypeDefinition { name, .. } =
                &st.kind
            {
                def_names.push(name.as_str());
            }
        }
        if def_names.is_empty() {
            return false;
        }
        matches!(
            body.stmts.last().map(|s| &s.kind),
            Some(crate::frontend::core::parser::ast::StmtKind::Expr(e))
                if matches!(e.as_ref(), Expr::Var(n, _) if def_names.contains(&n.as_str()))
        )
    }

    /// `name = <value>` 是**函数定义**还是**值绑定**？（B 方案）
    ///
    /// # 判据：注解
    ///
    /// | 情形 | 结果 | 依据 |
    /// | ---- | ---- | ---- |
    /// | `value` 是 `Lambda`（`=>`） | 函数 | `=>` 是显式函数构造子 |
    /// | 注解是 `Fn` | 函数 | 声明了函数类型 |
    /// | 注解是非 `Fn` 类型 | 值 | 注解即类型 |
    /// | **无注解** | **值** | **内容决定类型** |
    ///
    /// # 无注解时按内容推断
    ///
    /// 无注解的 `f = { 5 }` 是**值** `5`，不是函数——块的值由内容给出，
    /// 不由注解存在与否决定。要定义函数就写注解：`f: () -> Int = { 5 }`。
    /// 这条规则与 `d = {"a": 1}` 同源：类型由内容决定。
    pub fn block_binding_is_function(
        type_annotation: Option<&Type>,
        value: Option<&Expr>,
    ) -> bool {
        match value {
            // `=>` 是显式函数构造子，注释无关
            Some(Expr::Lambda { .. }) => true,
            // 块：只有 Fn 注解才是函数；无注解按值处理（内容决定类型）
            Some(Expr::Block(_)) => matches!(type_annotation, Some(Type::Fn { .. })),
            // 非块值的绑定不参与此判定
            _ => false,
        }
    }

    /// RFC-004: 从绑定语句 `Type.method = fn[pos]` 的 index 部分提取位置列表。
    /// 支持单个整数与整数元组；负索引（`-1` 从末尾计数）原样保留，
    /// 由调用方按被绑函数元数归一化。无法识别时回退 `[0]`（与既有行为一致）。
    pub fn extract_binding_positions(index: &Expr) -> Vec<i64> {
        fn one(e: &Expr) -> Option<i64> {
            match e {
                Expr::Lit(Literal::Int(n), _) => Some(*n as i64),
                Expr::UnOp {
                    op: UnOp::Neg,
                    expr,
                    ..
                } => match expr.as_ref() {
                    Expr::Lit(Literal::Int(n), _) => Some(-(*n as i64)),
                    _ => None,
                },
                _ => None,
            }
        }
        match index {
            Expr::Tuple(items, _) => {
                let parsed: Vec<i64> = items.iter().filter_map(one).collect();
                if parsed.is_empty() {
                    vec![0]
                } else {
                    parsed
                }
            }
            other => one(other).map(|p| vec![p]).unwrap_or_else(|| vec![0]),
        }
    }
}
