# RFC-010 Unified Type Syntax - Pending Implementation Documentation

> **Created**: 2026-02-03
> **Status**: Pending Implementation
> **Based on RFC**: RFC-010 Unified Type Syntax

## Overview

This document describes the parts of the RFC-010 unified type syntax design that are not yet implemented or are only partially implemented, serving as a reference guide for subsequent development.

---

## 1. Method Binding Syntax Parsing

### 1.1 Problem Description

RFC-010 designed the `Type.method: (Type, ...) -> ReturnType = ...` method definition syntax, but the parser currently lacks support for this syntax.

**Expected Syntax**:
```yaoxiang
# Type method definition
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}
```

**Current State**:
- AST has `MethodBind` node definition (`src/frontend/core/parser/ast.rs:184-195`)
- Parser `declarations.rs` lacks corresponding syntax parsing logic

### 1.2 Required Modifications

#### 1.2.1 Modify `parse_type_annotation` or Add New Parsing Function

Add method binding syntax recognition in `src/frontend/core/parser/statements/declarations.rs`:

```rust
/// Check if this is method binding syntax: `Type.method: (Params) -> ReturnType`
fn is_method_bind_syntax(state: &mut ParserState<'_>) -> bool {
    let saved = state.save_position();

    // Check for dot-separated type name and method name
    // E.g., Point.draw: (Point, Surface) -> Void = ...
    let has_dot_method = if let Some(TokenKind::Identifier(_)) = state.current().map(|t| &t.kind) {
        state.bump();
        state.at(&TokenKind::Dot)
    } else {
        false
    };

    state.restore_position(saved);
    has_dot_method
}
```

#### 1.2.2 Add Method Binding Parsing Function

```rust
/// Parse method binding: `Type.method: (Params) -> ReturnType = (params) => body`
pub fn parse_method_bind_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // Parse type name
    let type_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // Consume dot
    state.expect(&TokenKind::Dot)?;

    // Parse method name
    let method_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    // Consume colon
    state.expect(&TokenKind::Colon)?;

    // Parse method type
    let method_type = parse_type_annotation(state)?;

    // Consume equals sign
    state.expect(&TokenKind::Eq)?;

    // Parse method body
    let body = parse_fn_body(state)?;

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::MethodBind {
            type_name,
            method_name,
            method_type,
            params: body.0,
            body: body.1,
        },
        span,
    })
}
```

### 1.3 Test Cases

```rust
#[test]
fn test_method_bind_parsing() {
    let code = r#"
        Point.draw: (Point, Surface) -> Void = (self, surface) => {
            surface.plot(self.x, self.y)
        }
    "#;

    let ast = parse(code).unwrap();
    assert!(matches!(
        ast.items[0].kind,
        StmtKind::MethodBind {
            type_name: ref n,
            method_name: ref m,
            ..
        } if n == "Point" && m == "draw"
    ));
}
```

---

## 2. Pub Auto-Binding Mechanism

### 2.1 Problem Description

RFC-010 designed the `pub` auto-binding mechanism: when a function is declared with `pub`, the compiler should automatically bind it to a type defined in the same file.

**Expected Behavior**:
```yaoxiang
# Using pub declaration, compiler automatically binds to Point.distance
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Equivalent to:
Point.distance = distance[0]

# Invocation methods
d1 = distance(p1, p2)      # Functional style
d2 = p1.distance(p2)       # OOP syntax sugar
```

**Current State**: No related implementation

### 2.2 Required Modifications

#### 2.2.1 Modify Parser to Recognize Pub Functions

In the `parse_identifier_stmt` function in `src/frontend/core/parser/statements/declarations.rs`:

```rust
/// Parse statement starting with identifier
pub fn parse_identifier_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // Check for pub declaration
    let is_pub = state.skip(&TokenKind::KwPub);

    // Subsequent logic...

    // Return with pub status marked
    Some(Stmt {
        kind: StmtKind::Fn {
            name,
            type_annotation,
            params,
            body,
            is_pub,  // New field
        },
        span,
    })
}
```

#### 2.2.2 Add New AST Fields

Modify `StmtKind::Fn` in `src/frontend/core/parser/ast.rs`:

```rust
/// Function definition: `name: Type = (params) => body`
pub struct FnStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
    pub is_pub: bool,  // New: whether to auto-bind to type
    pub auto_bind_type: Option<String>,  // New: target type for auto-binding
}
```

#### 2.2.3 Implement Auto-Binding in Type Checking Phase

In `src/frontend/typecheck/inference/statements.rs`:

```rust
/// Handle function definition with pub auto-binding
fn infer_fn_stmt(
    &mut self,
    stmt: &Stmt,
    env: &mut TypeEnvironment,
) -> TypeResult<MonoType> {
    match &stmt.kind {
        StmtKind::Fn { name, params, return_type, body, is_pub, .. } => {
            // Build function type
            let fn_type = self.infer_fn_type(params, return_type.as_ref())?;

            if *is_pub {
                // Try to auto-bind to types defined in the same file
                if let Some(target_type) = self.find_target_type_for_pub(name, params) {
                    self.bind_method_to_type(&target_type, name, &fn_type)?;
                }
            }

            // Register to environment
            env.add_var(name.clone(), PolyType::mono(fn_type));

            Ok(MonoType::Void)
        }
        _ => unreachable!(),
    }
}

/// Find the target type a pub function should bind to
fn find_target_type_for_pub(
    &self,
    fn_name: &str,
    params: &[Param],
) -> Option<String> {
    // Rule: the type name of the first parameter is the binding target
    // E.g., distance: (Point, Point) -> Float binds to Point
    if let Some(first_param) = params.first() {
        if let Some(ref ty) = first_param.ty {
            return Some(self.type_to_string(ty));
        }
    }
    None
}
```

### 2.3 Test Cases

```rust
#[test]
fn test_pub_auto_bind() {
    let code = r#"
        type Point = {
            x: Float,
            y: Float
        }

        pub distance: (Point, Point) -> Float = (p1, p2) => {
            dx = p1.x - p2.x
            dy = p1.y - p2.y
            (dx * dx + dy * dy).sqrt()
        }
    "#;

    let type_env = typecheck(code).unwrap();

    // Check if Point.distance method is bound
    let point_type = type_env.get_type("Point").unwrap();
    assert!(point_type.methods.contains_key("distance"));
}
```

---

## 3. Generic Constraint Syntax Parsing

### 3.1 Problem Description

RFC-010 integrates with RFC-011 generic system, supporting `[T: Constraint]` constraint syntax.

**Expected Syntax**:
```yaoxiang
# Generic function with constraint
clone: [T: Clone](value: T) -> T = value.clone()

# Multiple constraints (ampersand syntax not yet supported)
# process: [T: Drawable & Serializable](item: T) -> String = { ... }

# Angle bracket syntax
identity: <T: Clone>(value: T) -> T = value
```

**Current State**: ✅ Implemented

### 3.2 Required Modifications

#### 3.2.1 Modify Generic Parameter Parsing

In `src/frontend/core/parser/statements/declarations.rs`:

```rust
/// Generic parameter structure
pub struct GenericParam {
    pub name: String,
    pub constraints: Vec<MonoType>,  // Constraint list
}

/// Parse generic parameters: `[T, U]` or `[T: Clone, U: Serializable]`
pub fn parse_generic_params_with_constraints(
    state: &mut ParserState<'_>,
) -> Option<Vec<GenericParam>> {
    let open = if state.at(&TokenKind::LBracket) {
        state.bump();
        TokenKind::RBracket
    } else if state.at(&TokenKind::Lt) {
        state.bump();
        TokenKind::Gt
    } else {
        return Some(Vec::new());
    };

    let mut params = Vec::new();

    while !state.at(&open) && !state.at_end() {
        // Parse parameter name
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        // Parse constraints
        let mut constraints = Vec::new();
        if state.skip(&TokenKind::Colon) {
            loop {
                let constraint = parse_type_annotation(state)?;
                constraints.push(constraint);

                if !state.skip(&TokenKind::Amp) {
                    break;
                }
            }
        }

        params.push(GenericParam { name, constraints });
        state.skip(&TokenKind::Comma);
    }

    if !state.expect(&open) {
        return None;
    }

    Some(params)
}
```

#### 3.2.2 Modify Type Definition and Function Definition

Add generic parameters to `StmtKind::Fn`:

```rust
/// Function definition with generic params
pub struct FnStmt {
    pub name: String,
    pub generic_params: Vec<GenericParam>,  // Added
    pub type_annotation: Option<Type>,
    pub params: Vec<Param>,
    pub body: (Vec<Stmt>, Option<Box<Expr>>),
}
```

#### 3.2.3 Implement Constraint Validation in Type Checking

In `src/frontend/typecheck/checking/bounds.rs`:

```rust
/// Check if generic parameters satisfy constraints
pub fn check_generic_param_constraints(
    &self,
    param: &GenericParam,
    arg_type: &MonoType,
) -> Result<(), TypeError> {
    for constraint in &param.constraints {
        if !self.check_constraint(arg_type, constraint)? {
            return Err(TypeError::ConstraintNotSatisfied {
                param_name: param.name.clone(),
                constraint_name: constraint.type_name(),
                arg_type: arg_type.type_name(),
            });
        }
    }
    Ok(())
}
```

### 3.3 Test Cases

```rust
#[test]
fn test_generic_constraint_parsing() {
    let code = r#"
        clone: [T: Clone](value: T) -> T = value.clone()
    "#;

    let ast = parse(code).unwrap();
    match &ast.items[0].kind {
        StmtKind::Fn { generic_params, .. } => {
            assert_eq!(generic_params.len(), 1);
            assert_eq!(generic_params[0].name, "T");
            assert_eq!(generic_params[0].constraints.len(), 1);
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_generic_constraint_checking() {
    let code = r#"
        type Point = { x: Float, y: Float }

        # Point does not implement Clone, should error
        clone: [T: Clone](value: T) -> T = value.clone()
    "#;

    let result = typecheck(code);
    assert!(result.is_err());
}
```

---

## 4. Complete Implementation Priority

| Priority | Feature | Scope | Status |
|----------|---------|-------|--------|
| **P0** | Method binding syntax parsing | Parser | Pending Implementation |
| **P1** | Pub auto-binding mechanism | Parser + Type Checking | Pending Implementation |
| **P2** | Generic constraint syntax | Parser + Type Checking | ✅ Completed |

---

## 5. Related File List

### 5.1 Files Requiring Modification

| File Path | Modification Content |
|-----------|---------------------|
| `src/frontend/core/parser/ast.rs` | Add `GenericParam` struct, add `generic_params` field to `StmtKind::Fn` |
| `src/frontend/core/parser/statements/declarations.rs` | Add `parse_generic_params_with_constraints`, modify `parse_var_stmt`, extend `parse_type_annotation` |
| `src/frontend/typecheck/checking/mod.rs` | Add `generic_params` field matching |
| `src/frontend/typecheck/inference/statements.rs` | Add `generic_params` field matching |
| `src/frontend/typecheck/inference/expressions.rs` | Add `generic_params` field matching |
| `src/middle/core/ir_gen.rs` | Add `generic_params` field matching |

### 5.2 Files to Add

| File Path | Description |
|-----------|-------------|
| `src/frontend/core/parser/statements/method_bind.rs` | Method binding parsing logic (pending implementation) |
| `src/frontend/typecheck/checking/auto_bind.rs` | Auto-binding checking logic (pending implementation) |

---

## 6. Acceptance Criteria

### 6.1 Method Binding
- [ ] Can parse `Type.method: (Params) -> ReturnType = ...` syntax
- [ ] AST correctly generates `MethodBind` node
- [ ] Type checking correctly binds methods to types

### 6.2 Pub Auto-Binding
- [ ] `pub fn` can be correctly recognized
- [ ] Can auto-bind to the first parameter's type
- [ ] Supports `p.method()` syntax sugar invocation

### 6.3 Generic Constraints
- [x] Can parse `[T: Clone]` syntax
- [ ] Type checking can verify if constraints are satisfied (pending implementation)
- [ ] Error messages clearly indicate missing constraints (pending implementation)