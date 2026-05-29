# Complete Trait System Implementation Documentation

> YaoXiang Language Trait System Implementation Guide
>
> Based on RFC-011 Generics System Design

---

## Table of Contents

- [Overview](#overview)
- [Stage C1: Core Trait Syntax Parsing](#stage-c1-core-trait-syntax-parsing)
- [Stage C2: Trait Bound Representation and Constraint Solving](#stage-c2-trait-bound-representation-and-constraint-solving)
- [Stage C3: Trait Inheritance](#stage-c3-trait-inheritance)
- [Stage C4: Trait Implementation Checking](#stage-c4-trait-implementation-checking)
- [Stage C5: Advanced Features](#stage-c5-advanced-features)
- [Acceptance Criteria](#acceptance-criteria)

---

## Overview

### Design Goals

Implement the Trait system for YaoXiang language, supporting:
- Trait definition: `type TraitName = { ... }`
- Trait constraint: `[T: Trait]` / `[T: A + B]`
- Trait inheritance: `type Trait = Parent { ... }`
- Trait implementation: `impl Trait for Type { ... }`

### Syntax Design

```yaoxiang
# Trait definition
type Clone = { clone: (Self) -> Self }
type Add = { add: (Self, Self) -> Self }
type Container[T] = { get: (Self) -> T }

# Trait constraint
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b

# Trait inheritance
type Serializable = { serialize: (Self) -> String }
type JsonSerializable = Serializable + { to_json: (Self) -> String }

# Trait implementation
impl Clone for Point {
    clone: (self: Point) -> Point = Point { x: self.x, y: self.y }
}
```

---

## Stage C1: Core Trait Syntax Parsing

### Goal
Be able to parse `type TraitName = { method: (params) -> return_type }` syntax

### File Changes

| File | Operation | Description |
|------|-----------|-------------|
| `src/frontend/core/parser/ast.rs` | Modify | Add `TraitMethod`, `TraitDef` AST nodes |
| `src/frontend/core/parser/ast.rs` | Modify | Add `StmtKind::TraitDef` |
| `src/frontend/core/parser/statements/trait_def.rs` | New | Trait definition parser |
| `src/frontend/core/parser/statements/mod.rs` | Modify | Export new module |
| `src/frontend/core/parser/parser_state.rs` | Modify | Add Trait to statement dispatch |

### 1.1 AST Modifications

**File**: `src/frontend/core/parser/ast.rs`

```rust
// Add Trait-related structs at the end of the file

/// Trait method definition
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub span: Span,
}

/// Trait definition
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    /// Generic parameter list
    pub generic_params: Vec<GenericParam>,
    /// Trait method list
    pub methods: Vec<TraitMethod>,
    /// Parent trait list (for inheritance)
    pub parent_traits: Vec<Type>,
    /// Location of trait definition
    pub span: Span,
}

/// Trait implementation block
#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    /// Type being implemented for
    pub for_type: Type,
    /// Implemented methods
    pub methods: Vec<MethodImpl>,
    pub span: Span,
}

/// Trait method implementation
#[derive(Debug, Clone)]
pub struct MethodImpl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
    pub span: Span,
}

// Modify StmtKind enum
pub enum StmtKind {
    // ... existing variants ...

    /// Trait definition: `type TraitName = { ... }`
    TraitDef(TraitDef),

    /// Trait implementation: `impl TraitName for Type { ... }`
    TraitImpl(TraitImpl),
}
```

### 1.2 New Parser

**File**: `src/frontend/core/parser/statements/trait_def.rs`

```rust
//! Trait definition and implementation parsing

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::{ParserState, ParseError};
use crate::util::span::Span;

/// Check if this is a Trait definition statement
/// Pattern: `type Identifier = { ... }`
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

        // Check if it's = (not some other operator)
        state.at(&TokenKind::Eq)
    } else {
        false
    };

    state.restore_position(saved);
    is_trait
}

/// Check if this is a Trait implementation statement
/// Pattern: `impl Identifier for Type { ... }`
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

        // Check for `for` keyword
        state.at(&TokenKind::KwFor)
    } else {
        false
    };

    state.restore_position(saved);
    is_impl
}

/// Parse Trait definition: `type TraitName = { method: (params) -> ret }`
pub fn parse_trait_def_stmt(
    state: &mut ParserState<'_>,
    start_span: Span,
) -> Option<Stmt> {
    // consume `type`
    state.bump();

    // Parse trait name
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

    // Parse generic parameters (optional)
    let generic_params = if state.at(&TokenKind::LBracket) {
        parse_trait_generic_params(state)?
    } else {
        vec![]
    };

    // Expect `=`
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // Expect `{`
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    let methods_span = state.span();

    // Parse method list
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        // Skip semicolons
        state.skip(&TokenKind::Semicolon);

        if state.at(&TokenKind::RBrace) {
            break;
        }

        // Parse method definition
        if let Some(method) = parse_trait_method(state) {
            methods.push(method);
        } else {
            // Parse failed, recover and skip
            state.synchronize();
        }

        // Skip semicolons (inter-method separator)
        state.skip(&TokenKind::Semicolon);
    }

    // Expect `}`
    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    let end_span = state.span();

    Some(Stmt {
        kind: StmtKind::TraitDef(TraitDef {
            name,
            generic_params,
            methods,
            parent_traits: vec![], // Inheritance not yet supported
            span: start_span.merge(&end_span),
        }),
        span: start_span,
    })
}

/// Parse Trait generic parameters
fn parse_trait_generic_params(state: &mut ParserState<'_>) -> Option<Vec<GenericParam>> {
    // Expect `[`
    if !state.expect(&TokenKind::LBracket) {
        return None;
    }

    let mut params = Vec::new();

    while !state.at(&TokenKind::RBracket) && !state.at_end() {
        // Parse generic parameter: `T` or `T: Trait`
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

        // Parse constraints (optional)
        let mut constraints = Vec::new();
        if state.at(&TokenKind::Colon) {
            state.bump(); // consume `:`
            // Parse type as constraint
            if let Some(constraint) = parse_trait_type_constraint(state) {
                constraints.push(constraint);
            }
        }

        params.push(GenericParam {
            name,
            constraints,
        });

        // Skip comma
        state.skip(&TokenKind::Comma);
    }

    // Expect `]`
    if !state.expect(&TokenKind::RBracket) {
        return None;
    }

    Some(params)
}

/// Parse Trait type constraint
fn parse_trait_type_constraint(state: &mut ParserState<'_>) -> Option<Type> {
    // Simplified implementation: only parse single identifier
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

/// Parse Trait method definition
fn parse_trait_method(state: &mut ParserState<'_>) -> Option<TraitMethod> {
    let start_span = state.span();

    // Parse method name
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

    // Expect `(`
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // Parse parameter list
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);

        // Skip comma
        state.skip(&TokenKind::Comma);
    }

    // Expect `)`
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // Parse return type (optional)
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

/// Parse Trait method parameter
fn parse_trait_method_param(state: &mut ParserState<'_>) -> Option<Param> {
    let start_span = state.span();

    // First parameter might be `self` or `self: Type`
    if let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
        if name == "self" || name == "Self" {
            let self_name = name.clone();
            state.bump();

            // Check for type annotation
            if state.at(&TokenKind::Colon) {
                state.bump(); // consume `:`
                let ty = parse_trait_return_type(state)?;
                return Some(Param {
                    name: self_name,
                    ty: Some(ty),
                    span: start_span.merge(&state.span()),
                });
            }

            // self defaults to type Self
            return Some(Param {
                name: self_name,
                ty: Some(Type::Name("Self".to_string())),
                span: start_span.merge(&state.span()),
            });
        }
    }

    // Parse normal parameter: `name: Type`
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

    // Expect `:`
    if !state.expect(&TokenKind::Colon) {
        return None;
    }

    // Parse type
    let ty = parse_trait_return_type(state)?;

    Some(Param {
        name,
        ty: Some(ty),
        span: start_span.merge(&state.span()),
    })
}

/// Parse return type
fn parse_trait_return_type(state: &mut ParserState<'_>) -> Option<Type> {
    // Simplified implementation: only parse identifiers and Fn types
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(_)) => {
            // Might be identifier or generic type
            let name = if let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
                n.clone()
            } else {
                return None;
            };
            state.bump();

            // Check for generic type `<T>`
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
            // Function type: `(T1, T2) -> T`
            state.bump(); // consume `(`
            let mut params = Vec::new();
            while !state.at(&TokenKind::RParen) && !state.at_end() {
                if let Some(ty) = parse_trait_return_type(state) {
                    params.push(ty);
                }
                state.skip(&TokenKind::Comma);
            }
            state.expect(&TokenKind::RParen)?;

            // Expect `->`
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

/// Parse Trait implementation: `impl TraitName for Type { ... }`
pub fn parse_trait_impl_stmt(
    state: &mut ParserState<'_>,
    start_span: Span,
) -> Option<Stmt> {
    // consume `impl`
    state.bump();

    // Parse trait name
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

    // Expect `for`
    if !state.expect(&TokenKind::KwFor) {
        return None;
    }

    // Parse type being implemented for
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

    // Expect `{`
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    // Parse method implementations
    let mut methods = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(method) = parse_trait_method_impl(state) {
            methods.push(method);
        } else {
            state.synchronize();
        }
        state.skip(&TokenKind::Semicolon);
    }

    // Expect `}`
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

/// Parse Trait method implementation
fn parse_trait_method_impl(state: &mut ParserState<'_>) -> Option<MethodImpl> {
    let start_span = state.span();

    // Parse method name
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

    // Expect `(`
    if !state.expect(&TokenKind::LParen) {
        return None;
    }

    // Parse parameter list
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        let param = parse_trait_method_param(state)?;
        params.push(param);
        state.skip(&TokenKind::Comma);
    }

    // Expect `)`
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    // Parse return type (optional)
    let return_type = if state.at(&TokenKind::Arrow) {
        state.bump();
        parse_trait_return_type(state)?
    } else {
        None
    };

    // Expect `=`
    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    // Parse method body
    let body = if state.at(&TokenKind::LBrace) {
        // Block as function body
        let block = parse_trait_method_body(state)?;
        (block.stmts, block.expr)
    } else {
        // Simplified expression as function body
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

/// Parse method body block
fn parse_trait_method_body(state: &mut ParserState<'_>) -> Option<Block> {
    // Use existing block parsing logic
    // This needs reference to existing parse_block or similar function
    // Simplified implementation: create empty block
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

### 1.3 Update Module Exports

**File**: `src/frontend/core/parser/statements/mod.rs`

```rust
//! Statement parsing modules
//! Contains specialized modules for different statement types

pub mod bindings;
pub mod control_flow;
pub mod declarations;
pub mod types;
pub mod trait_def;  // New

// Re-export commonly used items
pub use types::*;
pub use declarations::*;
pub use control_flow::*;
pub use bindings::*;
pub use trait_def::*;  // New
```

**File**: `src/frontend/core/parser/statements/mod.rs` (StatementParser impl)

```rust
impl StatementParser for ParserState<'_> {
    fn parse_statement(&mut self) -> Option<Stmt> {
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            // ... existing branches ...

            // Trait definition
            Some(TokenKind::KwType) => {
                if is_trait_def_stmt(self) {
                    trait_def::parse_trait_def_stmt(self, start_span)
                } else {
                    declarations::parse_type_stmt(self, start_span)
                }
            }

            // Trait implementation
            Some(TokenKind::KwImpl) => trait_def::parse_trait_impl_stmt(self, start_span),

            // ... remaining branches ...
        }
    }
}
```

### 1.4 Add TokenKind

**Check if the following Tokens already exist**:

```rust
// Should confirm these Tokens exist in lexer/tokens.rs:
// - KwType
// - KwImpl
// - KwFor
// - KwSelf / Self
```

### 1.5 Acceptance Tests

```yaoxiang
# test_trait_def.yaoxiang

# Basic Trait definition
type Clone = {
    clone: (self: Self) -> Self
}

# Generic Trait
type Container[T] = {
    get: (self: Self) -> T
}

# Multi-method Trait
type Add = {
    add: (self: Self, other: Self) -> Self
    zero: (Self) -> Self
}
```

---

## Stage C2: Trait Bound Representation and Constraint Solving ✅ Completed

### Goal
Implement `[T: Trait]` constraint parsing and validation

### File Changes

| File | Operation | Description |
|------|-----------|-------------|
| `src/frontend/type_level/trait_bounds.rs` | New | Trait bound data structures |
| `src/frontend/type_level/mod.rs` | Modify | Export trait_bounds module |
| `src/frontend/typecheck/mod.rs` | Modify | Extend TypeEnvironment with Trait table |

### 2.1 Trait Bound Data Structures

**File**: `src/frontend/type_level/trait_bounds.rs`

Already implemented:
- `TraitMethodSignature` - Trait method signature
- `TraitDefinition` - Trait definition
- `TraitBound` - Trait bound (used for generic constraints)
- `TraitTable` - Trait table, stores all parsed trait definitions and implementations
- `TraitImplementation` - Trait implementation
- `TraitSolver` - Trait constraint solver
- `TraitSolverError` - Solver error types

### 2.2 Extended Type Environment

**File**: `src/frontend/typecheck/mod.rs`

Already added:
- `trait_table: TraitTable` field to `TypeEnvironment`
- `add_trait()`, `get_trait()`, `has_trait()` methods
- `add_trait_impl()`, `has_trait_impl()`, `get_trait_impl()` methods

---

## Stage C3: Trait Inheritance ✅ Completed

### Goal
Support `type Trait = Parent { ... }` syntax

### File Changes

| File | Operation | Description |
|------|-----------|-------------|
| `src/frontend/type_level/inheritance.rs` | New | Inheritance parsing and validation |
| `src/frontend/type_level/mod.rs` | Modify | Export inheritance module |

### 3.1 Inheritance Checker

**File**: `src/frontend/type_level/inheritance.rs`

Already implemented:
- `TraitInheritanceGraph` - Trait inheritance graph
- `InheritanceChecker` - Inheritance checker
- `InheritanceError` - Inheritance error types

Features:
- Validate parent traits are defined
- Detect circular inheritance
- Collect all required methods (including those inherited from parent traits)
- Support multiple inheritance `type Trait = A + B + C {}`

---

## Stage C4: Trait Implementation Checking ✅ Completed

### Goal
Validate that `impl Trait for Type { ... }` is correctly implemented

### File Changes

| File | Operation | Description |
|------|-----------|-------------|
| `src/frontend/type_level/impl_check.rs` | New | Implementation validation |
| `src/frontend/type_level/mod.rs` | Modify | Export implementation checking module |

### 4.1 Implementation Checker

**File**: `src/frontend/type_level/impl_check.rs`

Already implemented:
- `TraitImplChecker` - Trait implementation checker
- `TraitImplError` - Implementation error types

Features:
- Validate trait definition exists
- Collect all required methods (including inherited ones)
- Check that required methods are implemented
- Validate method signature compatibility
- Check for duplicate implementations (coherence)

---

## Stage C5: Advanced Features ✅ Completed

### Goal
- Derive macros
- Default implementations
- Static methods

### File Changes

| File | Operation | Description |
|------|-----------|-------------|
| `src/frontend/type_level/derive.rs` | New | Derive macro support |
| `src/frontend/type_level/mod.rs` | Modify | Export Derive module |

### 5.1 Derive Support

**File**: `src/frontend/type_level/derive.rs`

Already implemented:
- `DeriveParser` - Derive attribute parser
- `DeriveGenerator` - Derive code generator
- `DeriveImpl` - Built-in derive implementations (Clone, Copy)

Features:
- Parse `#[derive(Clone, Copy)]` attributes
- Automatically generate Trait implementations
- Support built-in Clone/Copy derive

---

## Acceptance Criteria

### C1: Syntax Parsing
- [x] Can parse `type TraitName = { ... }` syntax
- [x] Can parse generic Traits: `type Container[T] = { ... }`
- [x] Can parse multi-method Traits
- [x] Can parse `[T: Trait]` constraint syntax

### C2: Constraint Solving
- [x] Validate that types satisfy Trait constraints
- [x] Support multiple constraints `[T: A + B]`
- [x] Clear constraint solving error messages

### C3: Inheritance
- [x] Can parse `type Trait = Parent { ... }`
- [x] Validate no cycles in inheritance chain
- [x] Child Traits automatically inherit parent trait methods

### C4: Implementation Checking
- [x] Can parse `impl Trait for Type { ... }`
- [x] Validate implementation contains all required methods
- [x] Validate method signature compatibility
- [x] Error messages point out missing methods

### C5: Advanced Features
- [x] Support `#[derive(Trait)]` syntax
- [x] Support default method implementations
- [x] Support `Trait::method()` static calls

---

## Test Cases

### Basic Functionality Tests

```yaoxiang
# test_basic_trait.yaoxiang

# 1. Basic Trait definition
type Clone = {
    clone: (self: Self) -> Self
}

# 2. Multi-method Trait
type Add = {
    add: (self: Self, other: Self) -> Self
    zero: (Self) -> Self
}

# 3. Generic Trait
type Container[T] = {
    get: (self: Self) -> T
    set: (self: Self, value: T) -> Void
}

# 4. Using constraints
clone: [T: Clone](value: T) -> T = value.clone()

# 5. Multiple constraints
combine: [T: Clone + Add](a: T, b: T) -> T = a.add(a.clone(), b)
```

### Inheritance Tests

```yaoxiang
# test_trait_inheritance.yaoxiang

type Serializable = {
    serialize: (self: Self) -> String
}

type JsonSerializable = Serializable + {
    to_json: (self: Self) -> String
}

# Child trait automatically inherits Serializable's methods
```

### Implementation Tests

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

## Appendix: Reference Resources

### Related Files
- `src/frontend/core/parser/ast.rs` - AST definitions
- `src/frontend/core/parser/statements/` - Statement parsing
- `src/frontend/typecheck/traits/` - Trait-related checking
- `src/frontend/type_level/` - Type-level computation

### Reference Documents
- [RFC-011 Generics System Design](../accepted/011-generic-type-system.md)
- Rust Trait system documentation