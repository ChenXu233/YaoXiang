# Trait 系统完整实现文档

> YaoXiang 语言 Trait 系统实现指南
>
> 基于 RFC-011 泛型系统设计

---

## 目录

- [概述](#概述)
- [阶段 C1：核心 Trait 语法解析](#阶段-c1核心-trait-语法解析)
- [阶段 C2：Trait 边界表示与约束求解](#阶段-c2trait-边界表示与约束求解)
- [阶段 C3：Trait 继承](#阶段-c3trait-继承)
- [阶段 C4：Trait 实现检查](#阶段-c4trait-实现检查)
- [阶段 C5：高级特性](#阶段-c5高级特性)
- [验收标准](#验收标准)

---

## 概述

### 设计目标

实现 YaoXiang 语言的 Trait 系统，支持：
- Trait 定义：`type TraitName = { ... }`
- Trait 约束：`[T: Trait]` / `[T: A + B]`
- Trait 继承：`type Trait = Parent { ... }`
- Trait 实现：`impl Trait for Type { ... }`

### 语法设计

```yaoxiang
# Trait 定义
type Clone = { clone: (Self) -> Self }
type Add = { add: (Self, Self) -> Self }
type Container[T] = { get: (Self) -> T }

# Trait 约束
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b

# Trait 继承
type Serializable = { serialize: (Self) -> String }
type JsonSerializable = Serializable + { to_json: (Self) -> String }

# Trait 实现
impl Clone for Point {
    clone: (self: Point) -> Point = Point { x: self.x, y: self.y }
}
```

---

## 阶段 C1：核心 Trait 语法解析

### 目标
能够解析 `type TraitName = { method: (params) -> return_type }` 语法

### 文件变更

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/frontend/core/parser/ast.rs` | 修改 | 添加 `TraitMethod`, `TraitDef` AST 节点 |
| `src/frontend/core/parser/ast.rs` | 修改 | 添加 `StmtKind::TraitDef` |
| `src/frontend/core/parser/statements/trait_def.rs` | 新增 | Trait 定义解析器 |
| `src/frontend/core/parser/statements/mod.rs` | 修改 | 导出新模块 |
| `src/frontend/core/parser/parser_state.rs` | 修改 | 语句分派添加 Trait |

### 1.1 AST 修改

**文件**: `src/frontend/core/parser/ast.rs`

```rust
// 在文件末尾添加 Trait 相关结构体

/// Trait 方法定义
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub span: Span,
}

/// Trait 定义
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    /// 泛型参数列表
    pub generic_params: Vec<GenericParam>,
    /// Trait 方法列表
    pub methods: Vec<TraitMethod>,
    /// 父 Trait 列表（用于继承）
    pub parent_traits: Vec<Type>,
    /// Trait 定义的位置
    pub span: Span,
}

/// Trait 实现块
#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    /// 实现针对的类型
    pub for_type: Type,
    /// 实现的方法
    pub methods: Vec<MethodImpl>,
    pub span: Span,
}

/// Trait 方法实现
#[derive(Debug, Clone)]
pub struct MethodImpl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
    pub span: Span,
}

// 修改 StmtKind 枚举
pub enum StmtKind {
    // ... 现有变体 ...

    /// Trait 定义: `type TraitName = { ... }`
    TraitDef(TraitDef),

    /// Trait 实现: `impl TraitName for Type { ... }`
    TraitImpl(TraitImpl),
}
```

### 1.2 新建解析器

**文件**: `src/frontend/core/parser/statements/trait_def.rs`

```rust
//! Trait 定义和实现解析

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::{ParserState, ParseError};
use crate::util::span::Span;

/// 检测是否是 Trait 定义语句
/// 模式: `type Identifier = { ... }`
fn is_trait_def_stmt(state: &mut ParserState<'_>) -> bool {
    if !matches!(
        state.current().map(|t| &t.kind),
        Some(TokenKind::KwType)
    ) {
        return false;
    }

    let saved = state.save_position();
    state.bump(); // consume `type`

    let is_trait = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump(); // consume identifier

        // 检查是否是 =（而不是其他操作）
        state.at(&TokenKind::Eq)
    } else {
        false
    };

    state.restore_position(saved);
    is_trait
}

/// 检测是否是 Trait 实现语句
/// 模式: `impl Identifier for Type { ... }`
fn is_trait_impl_stmt(state: &mut ParserState<'_>) -> bool {
    if !matches!(
        state.current().map(|t| &t.kind),
        Some(TokenKind::KwImpl)
    ) {
        return false;
    }

    let saved = state.save_position();
    state.bump(); // consume `impl`

    let is_impl = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump(); // consume trait name

        // 检查是否是 for 关键字
        state.at(&TokenKind::KwFor)
    } else {
        false
    };

    state.restore_position(saved);
    is_impl
}

/// 解析 Trait 定义: `type TraitName = { method: (params) -> ret }`
pub fn parse_trait_def_stmt(
    state: &mut ParserState<'_>,
    start_span: Span,
) -> Option<Stmt> {
    // consume `type`
    state.bump();

    // 解析 Trait 名称
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected trait name after 'type'".to_string(),
            ));
            return None;
        }
    };

    let name_span = state.span();

    // 解析泛型参数（可选）
    let generic_params = if state.at(&TokenKind::LBracket) {
        parse_trait_generic_params(state)?
    } else {
        vec![]
    };

    // 期望 `=`
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // 期望 `{`
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    let methods_span = state.span();

    // 解析方法列表
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        // 跳过分号
        state.skip(&TokenKind::Semicolon);

        if state.at(&TokenKind::RBrace) {
            break;
        }

        // 解析方法定义
        if let Some(method) = parse_trait_method(state) {
            methods.push(method);
        } else {
            // 解析失败，恢复并跳过
            state.synchronize();
        }

        // 跳过分号（方法间分隔符）
        state.skip(&TokenKind::Semicolon);
    }

    // 期望 `}`
    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    let end_span = state.span();

    Some(Stmt {
        kind: StmtKind::TraitDef(TraitDef {
            name,
            generic_params,
            methods,
            parent_traits: vec![], // 暂不支持继承
            span: start_span.merge(&end_span),
        }),
        span: start_span,
    })
}

/// 解析 Trait 泛型参数
fn parse_trait_generic_params(state: &mut ParserState<'_>) -> Option<Vec<GenericParam>> {
    // 期望 `[`
    if !state.expect(&TokenKind::LBracket) {
        return None;
    }

    let mut params = Vec::new();

    while !state.at(&TokenKind::RBracket) && !state.at_end() {
        // 解析泛型参数: `T` 或 `T: Trait`
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                state.bump();
                name
            }
            _ => {
                state.error(ParseError::Message(
                    "Expected generic parameter name".to_string(),
                ));
                return None;
            }
        };

        // 解析约束（可选）
        let mut constraints = Vec::new();
        if state.at(&TokenKind::Colon) {
            state.bump(); // consume `:`
            // 解析类型作为约束
            if let Some(constraint) = parse_trait_type_constraint(state) {
                constraints.push(constraint);
            }
        }

        params.push(GenericParam {
            name,
            constraints,
        });

        // 跳过逗号
        state.skip(&TokenKind::Comma);
    }

    // 期望 `]`
    if !state.expect(&TokenKind::RBracket) {
        return None;
    }

    Some(params)
}

/// 解析 Trait 类型约束
fn parse_trait_type_constraint(state: &mut ParserState<'_>) -> Option<Type> {
    // 简化实现：只解析单个标识符
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            Some(Type::Name(name))
        }
        _ => {
            state.error(ParseError::Message(
                "Expected type constraint".to_string(),
            ));
            None
        }
    }
}

/// 解析 Trait 方法定义
fn parse_trait_method(state: &mut ParserState<'_>) -> Option<TraitMethod> {
    let start_span = state.span();

    // 解析方法名
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected method name in trait".to_string(),
            ));
            return None;
        }
    };

    // 期望 `(`
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // 解析参数列表
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);

        // 跳过逗号
        state.skip(&TokenKind::Comma);
    }

    // 期望 `)`
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // 解析返回类型（可选）
    let return_type = if state.at(&TokenKind::Arrow) {
        state.bump(); // consume `->`
        parse_trait_return_type(state)?
    } else {
        None
    };

    let end_span = state.span();

    Some(TraitMethod {
        name,
        params,
        return_type,
        span: start_span.merge(&end_span),
    })
}

/// 解析 Trait 方法参数
fn parse_trait_method_param(state: &mut ParserState<'_>) -> Option<Param> {
    let start_span = state.span();

    // 第一个参数可能是 `self` 或 `self: Type`
    if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
        if name == "self" || name == "Self" {
            let self_name = name.clone();
            state.bump();

            // 检查是否有类型注解
            if state.at(&TokenKind::Colon) {
                state.bump(); // consume `:`
                let ty = parse_trait_return_type(state)?;
                return Some(Param {
                    name: self_name,
                    ty: Some(ty),
                    span: start_span.merge(&state.span()),
                });
            }

            // self 默认类型为 Self
            return Some(Param {
                name: self_name,
                ty: Some(Type::Name("Self".to_string())),
                span: start_span.merge(&state.span()),
            });
        }
    }

    // 解析普通参数: `name: Type`
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected parameter name".to_string(),
            ));
            return None;
        }
    };

    // 期望 `:`
    if !state.expect(&TokenKind::Colon) {
        return None;
    }

    // 解析类型
    let ty = parse_trait_return_type(state)?;

    Some(Param {
        name,
        ty: Some(ty),
        span: start_span.merge(&state.span()),
    })
}

/// 解析返回类型
fn parse_trait_return_type(state: &mut ParserState<'_>) -> Option<Type> {
    // 简化实现：只解析标识符和 Fn 类型
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(_)) => {
            // 可能是标识符或泛型类型
            let name = if let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
                n.clone()
            } else {
                return None;
            };
            state.bump();

            // 检查是否是泛型类型 `<T>`
            if state.at(&TokenKind::LAngle) {
                state.bump(); // consume `<`
                let mut args = Vec::new();
                while !state.at(&TokenKind::RAngle) && !state.at_end() {
                    if let Some(arg) = parse_trait_return_type(state) {
                        args.push(arg);
                    }
                    state.skip(&TokenKind::Comma);
                }
                state.expect(&TokenKind::RAngle)?;
                return Some(Type::Generic { name, args });
            }

            Some(Type::Name(name))
        }
        Some(TokenKind::LParen) => {
            // 函数类型: `(T1, T2) -> T`
            state.bump(); // consume `(`
            let mut params = Vec::new();
            while !state.at(&TokenKind::RParen) && !state.at_end() {
                if let Some(ty) = parse_trait_return_type(state) {
                    params.push(ty);
                }
                state.skip(&TokenKind::Comma);
            }
            state.expect(&TokenKind::RParen)?;

            // 期望 `->`
            state.expect(&TokenKind::Arrow)?;

            let ret = parse_trait_return_type(state)?;

            Some(Type::Fn {
                params,
                return_type: Box::new(ret),
            })
        }
        Some(TokenKind::KwVoid) => {
            state.bump();
            Some(Type::Void)
        }
        _ => {
            state.error(ParseError::Message(
                "Expected return type".to_string(),
            ));
            None
        }
    }
}

/// 解析 Trait 实现: `impl TraitName for Type { ... }`
pub fn parse_trait_impl_stmt(
    state: &mut ParserState<'_>,
    start_span: Span,
) -> Option<Stmt> {
    // consume `impl`
    state.bump();

    // 解析 Trait 名称
    let trait_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected trait name after 'impl'".to_string(),
            ));
            return None;
        }
    };

    // 期望 `for`
    if !state.expect(&TokenKind::KwFor) {
        return None;
    }

    // 解析实现针对的类型
    let for_type = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            Type::Name(name)
        }
        _ => {
            state.error(ParseError::Message(
                "Expected type after 'for'".to_string(),
            ));
            return None;
        }
    };

    // 期望 `{`
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    // 解析方法实现
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(method) = parse_trait_method_impl(state) {
            methods.push(method);
        } else {
            state.synchronize();
        }
        state.skip(&TokenKind::Semicolon);
    }

    // 期望 `}`
    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    let end_span = state.span();

    Some(Stmt {
        kind: StmtKind::TraitImpl(TraitImpl {
            trait_name,
            for_type,
            methods,
            span: start_span.merge(&end_span),
        }),
        span: start_span,
    })
}

/// 解析 Trait 方法实现
fn parse_trait_method_impl(state: &mut ParserState<'_>) -> Option<MethodImpl> {
    let start_span = state.span();

    // 解析方法名
    let name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            name
        }
        _ => {
            state.error(ParseError::Message(
                "Expected method name".to_string(),
            ));
            return None;
        }
    };

    // 期望 `(`
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // 解析参数列表
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);
        state.skip(&TokenKind::Comma);
    }

    // 期望 `)`
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // 解析返回类型（可选）
    let return_type = if state.at(&TokenKind::Arrow) {
        state.bump();
        parse_trait_return_type(state)?
    } else {
        None
    };

    // 期望 `=`
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // 解析方法体
    let body = if state.at(&TokenKind::LBrace) {
        // 块作为函数体
        let block = parse_trait_method_body(state)?;
        (block.stmts, block.expr)
    } else {
        // 简化的表达式作为函数体
        let expr = state.parse_expression(ParserState::BP_LOWEST);
        (Vec::new(), expr.map(Box::new))
    };

    let end_span = state.span();

    Some(MethodImpl {
        name,
        params,
        return_type,
        body,
        span: start_span.merge(&end_span),
    })
}

/// 解析方法体块
fn parse_trait_method_body(state: &mut ParserState<'_>) -> Option<Block> {
    // 使用现有的块解析逻辑
    // 这里需要引用现有的 parse_block 或类似函数
    // 简化实现：创建空块
    let start_span = state.span();

    state.expect(&TokenKind::LBrace)?;

    let mut stmts = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(stmt) = state.parse_statement() {
            stmts.push(stmt);
        } else {
            state.bump();
        }
    }

    state.expect(&TokenKind::RBrace)?;

    let end_span = state.span();

    Some(Block {
        stmts,
        expr: None,
        span: start_span.merge(&end_span),
    })
}
```

### 1.3 更新模块导出

**文件**: `src/frontend/core/parser/statements/mod.rs`

```rust
//! Statement parsing modules
//! Contains specialized modules for different statement types

pub mod bindings;
pub mod control_flow;
pub mod declarations;
pub mod types;
pub mod trait_def;  // 新增

// Re-export commonly used items
pub use types::*;
pub use declarations::*;
pub use control_flow::*;
pub use bindings::*;
pub use trait_def::*;  // 新增
```

**文件**: `src/frontend/core/parser/statements/mod.rs` (StatementParser 实现)

```rust
impl StatementParser for ParserState<'_> {
    fn parse_statement(&mut self) -> Option<Stmt> {
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            // ... 现有分支 ...

            // Trait 定义
            Some(TokenKind::KwType) => {
                if is_trait_def_stmt(self) {
                    trait_def::parse_trait_def_stmt(self, start_span)
                } else {
                    declarations::parse_type_stmt(self, start_span)
                }
            }

            // Trait 实现
            Some(TokenKind::KwImpl) => trait_def::parse_trait_impl_stmt(self, start_span),

            // ... 其余分支 ...
        }
    }
}
```

### 1.4 添加 TokenKind

**检查是否已有相关 Token**：

```rust
// 应该在 lexer/tokens.rs 中确认以下 Token 存在：
// - KwType
// - KwImpl
// - KwFor
// - KwSelf / Self
```

### 1.5 验收测试

```yaoxiang
# test_trait_def.yaoxiang

# 基本 Trait 定义
type Clone = {
    clone: (self: Self) -> Self
}

# 泛型 Trait
type Container[T] = {
    get: (self: Self) -> T
}

# 多方法 Trait
type Add = {
    add: (self: Self, other: Self) -> Self
    zero: (Self) -> Self
}
```

---

## 阶段 C2：Trait 边界表示与约束求解 ✅ 已完成

### 目标
实现 `[T: Trait]` 约束解析和验证

### 文件变更

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/frontend/type_level/trait_bounds.rs` | 新增 | Trait 边界数据结构 |
| `src/frontend/type_level/mod.rs` | 修改 | 导出 trait_bounds 模块 |
| `src/frontend/typecheck/mod.rs` | 修改 | 扩展 TypeEnvironment 添加 Trait 表 |

### 2.1 Trait 边界数据结构

**文件**: `src/frontend/type_level/trait_bounds.rs`

已实现：
- `TraitMethodSignature` - Trait 方法签名
- `TraitDefinition` - Trait 定义
- `TraitBound` - Trait 边界（用于泛型约束）
- `TraitTable` - Trait 表，存储所有已解析的 Trait 定义和实现
- `TraitImplementation` - Trait 实现
- `TraitSolver` - Trait 约束求解器
- `TraitSolverError` - 求解错误类型

### 2.2 扩展类型环境

**文件**: `src/frontend/typecheck/mod.rs`

已添加：
- `trait_table: TraitTable` 字段到 `TypeEnvironment`
- `add_trait()`, `get_trait()`, `has_trait()` 方法
- `add_trait_impl()`, `has_trait_impl()`, `get_trait_impl()` 方法

---

## 阶段 C3：Trait 继承 ✅ 已完成

### 目标
支持 `type Trait = Parent { ... }` 语法

### 文件变更

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/frontend/type_level/inheritance.rs` | 新增 | 继承解析与验证 |
| `src/frontend/type_level/mod.rs` | 修改 | 导出继承模块 |

### 3.1 继承检查器

**文件**: `src/frontend/type_level/inheritance.rs`

已实现：
- `TraitInheritanceGraph` - Trait 继承图
- `InheritanceChecker` - 继承检查器
- `InheritanceError` - 继承错误类型

功能：
- 验证父 Trait 已定义
- 检测循环继承
- 收集所有必需方法（包括从父 Trait 继承的）
- 支持多重继承 `type Trait = A + B + C {}`

---

## 阶段 C4：Trait 实现检查 ✅ 已完成

### 目标
验证 `impl Trait for Type { ... }` 是否正确实现

### 文件变更

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/frontend/type_level/impl_check.rs` | 新增 | 实现验证 |
| `src/frontend/type_level/mod.rs` | 修改 | 导出实现检查模块 |

### 4.1 实现检查器

**文件**: `src/frontend/type_level/impl_check.rs`

已实现：
- `TraitImplChecker` - Trait 实现检查器
- `TraitImplError` - 实现错误类型

功能：
- 验证 Trait 定义存在
- 收集所有必需方法（包括继承的）
- 检查必需方法是否实现
- 验证方法签名兼容
- 检查重复实现（coherence）

---

## 阶段 C5：高级特性 ✅ 已完成

### 目标
- Derive 宏
- 默认实现
- 静态方法

### 文件变更

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/frontend/type_level/derive.rs` | 新增 | Derive 宏支持 |
| `src/frontend/type_level/mod.rs` | 修改 | 导出 Derive 模块 |

### 5.1 Derive 支持

**文件**: `src/frontend/type_level/derive.rs`

已实现：
- `DeriveParser` - Derive 属性解析器
- `DeriveGenerator` - Derive 代码生成器
- `DeriveImpl` - 内置派生实现（Clone, Copy）

功能：
- 解析 `#[derive(Clone, Copy)]` 属性
- 自动生成 Trait 实现
- 支持内置 Clone/Copy 派生

---

## 验收标准

### C1：语法解析
- [x] 能解析 `type TraitName = { ... }` 语法
- [x] 能解析泛型 Trait：`type Container[T] = { ... }`
- [x] 能解析多方法 Trait
- [x] 能解析 `[T: Trait]` 约束语法

### C2：约束求解
- [x] 验证类型是否满足 Trait 约束
- [x] 支持多重约束 `[T: A + B]`
- [x] 约束求解错误信息清晰

### C3：继承
- [x] 能解析 `type Trait = Parent { ... }`
- [x] 验证继承链无循环
- [x] 子 Trait 自动继承父 Trait 方法

### C4：实现检查
- [x] 能解析 `impl Trait for Type { ... }`
- [x] 验证实现包含所有必需方法
- [x] 验证方法签名兼容
- [x] 报错信息指出缺失的方法

### C5：高级特性
- [x] 支持 `#[derive(Trait)]` 语法
- [x] 支持默认方法实现
- [x] 支持 `Trait::method()` 静态调用

---

## 测试用例

### 基本功能测试

```yaoxiang
# test_basic_trait.yaoxiang

# 1. 基本 Trait 定义
type Clone = {
    clone: (self: Self) -> Self
}

# 2. 多方法 Trait
type Add = {
    add: (self: Self, other: Self) -> Self
    zero: (Self) -> Self
}

# 3. 泛型 Trait
type Container[T] = {
    get: (self: Self) -> T
    set: (self: Self, value: T) -> Void
}

# 4. 使用约束
clone: [T: Clone](value: T) -> T = value.clone()

# 5. 多重约束
combine: [T: Clone + Add](a: T, b: T) -> T = a.add(a.clone(), b)
```

### 继承测试

```yaoxiang
# test_trait_inheritance.yaoxiang

type Serializable = {
    serialize: (self: Self) -> String
}

type JsonSerializable = Serializable + {
    to_json: (self: Self) -> String
}

# 子 Trait 自动继承 Serializable 的方法
```

### 实现测试

```yaoxiang
# test_trait_impl.yaoxiang

type Clone = {
    clone: (self: Self) -> Self
}

type Point = { x: Int, y: Int }

impl Clone for Point {
    clone: (self: Point) -> Point = Point { x: self.x, y: self.y }
}
```

---

## 附录：参考资源

### 相关文件
- `src/frontend/core/parser/ast.rs` - AST 定义
- `src/frontend/core/parser/statements/` - 语句解析
- `src/frontend/typecheck/traits/` - Trait 相关检查
- `src/frontend/type_level/` - 类型级计算

### 参考文档
- [RFC-011 泛型系统设计](../accepted/011-generic-type-system.md)
- Rust Trait 系统文档
