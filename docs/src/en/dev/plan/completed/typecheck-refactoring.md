# Type Checking Flow Complete Refactoring Plan

> **Status**: ✅ Completed  
> **Completion Date**: 2025-07  
> **Test Results**: All 1469 tests passed (1434 + 30 + 5), 0 failures

## Core Objectives

Completely eliminate technical debt, making the type checking flow clear, concise, and extensible, while maintaining good collaboration with existing feature modules.

---

## Existing Module Structure Analysis (Pre-Refactoring)

> The following is the pre-refactoring structure, for reference only.

```
src/frontend/typecheck/
├── mod.rs                      # Unified entry point
├── checking/                   # ❌ Problem: Overlapping responsibilities with inference
│   ├── mod.rs                 # BodyChecker, AssignmentChecker, SubtypeChecker...
│   ├── assignment.rs
│   ├── bounds.rs
│   ├── compatibility.rs
│   └── subtyping.rs
├── inference/                  # ❌ Problem: Overlapping responsibilities with checking
│   ├── mod.rs
│   ├── expressions.rs          # ExprInferrer
│   ├── generics.rs
│   ├── patterns.rs
│   └── statements.rs
├── specialization/             # ✅ Kept (independent feature)
├── traits/                     # ✅ Kept (independent feature)
├── gat/                        # ✅ Kept (independent feature)
├── tests/                      # ✅ Kept
├── overload.rs                 # ✅ Kept (independent feature)
├── type_eval.rs                # ✅ Kept (independent feature)
└── specialize.rs              # ✅ Kept (compatibility)
```

**Problem**: `checking/` and `inference/` are essentially doing the same thing, yet they're split into two directories!

---

## Refactoring Plan: Merge checking/ into inference/

### Directory Structure (Post-Refactoring)

```
src/frontend/typecheck/
├── mod.rs                  # Unified entry point, exports all modules
│
# ✅ Merged core module inference/
├── inference/
│   ├── mod.rs             # Exports + TypeChecker main entry
│   ├── scope.rs           # 🆕 Unified scope management
│   ├── types.rs           # 🆕 Type system utilities
│   ├── statements.rs      # 🆕 Statement checking (merged from checking + inference's statement parts)
│   ├── expressions.rs     # 🆕 Expression inference (merged from existing expressions.rs)
│   #
│   # ✅ Moved from checking/
│   ├── assignment.rs      # Assignment checking
│   ├── subtyping.rs       # Subtype checking
│   ├── compatibility.rs   # Compatibility checking
│   ├── bounds.rs          # Bounds checking
│   #
│   # ✅ Kept (enhanced)
│   ├── generics.rs        # Generics inference
│   └── patterns.rs        # Pattern inference
│
# ✅ Kept: Independent feature modules (unchanged, called via interfaces)
├── specialization/         # Specialization logic
├── traits/                # trait logic
├── gat/                   # GAT logic
├── overload.rs            # Overload resolution
├── type_eval.rs           # Type evaluation
├── specialize.rs          # Compatibility
│
# ❌ Deleted checking/ directory
└── tests/                  # Tests
```

### Module Responsibilities

| Module | Responsibilities | Notes |
|--------|-----------------|-------|
| `inference/scope.rs` | Unified variable scope management | All variable CRUD operations |
| `inference/types.rs` | Type utilities | unify, infer_element_type, etc. |
| `inference/statements.rs` | Statement checking | Var, Fn, For, If, Expr statements |
| `inference/expressions.rs` | Expression inference | Lit, Var, BinOp, Call, For expressions |
| `inference/assignment.rs` | Assignment checking | Moved from checking/ |
| `inference/subtyping.rs` | Subtype checking | Moved from checking/ |
| `inference/compatibility.rs` | Compatibility checking | Moved from checking/ |
| `inference/bounds.rs` | Bounds checking | Moved from checking/ |
| `specialization/*` | Specialization | Independent plugin |
| `traits/*` | trait | Independent plugin |
| `gat/*` | GAT | Independent plugin |
| `overload.rs` | Overload resolution | Independent plugin |

### Key Design Principles

1. **Single Entry Point**: `inference/` is the only type inference entry point
2. **ScopeManager Singleton**: The entire checking flow shares the same ScopeManager
3. **Feature Modules Independent**: specialization/traits/gat/overload are plugins called as needed
4. **No Duplicate Code**: Removed duplicate scopes in BodyChecker and ExprInferrer

---

## Detailed Design

### inference/scope.rs - Unified Scope Management

```rust
/// Scope manager
/// Single responsibility: managing the variable scope stack
pub struct ScopeManager {
    scopes: Vec<HashMap<String, PolyType>>,
}

impl ScopeManager {
    pub fn new() -> Self
    pub fn enter_scope(&mut self)
    pub fn exit_scope(&mut self)
    pub fn add_var(&mut self, name: String, poly: PolyType)
    pub fn get_var(&self, name: &str) -> Option<&PolyType>
    pub fn update_var(&mut self, name: &str, poly: PolyType)
    pub fn var_in_current_scope(&self, name: &str) -> bool
    pub fn var_in_any_scope(&self, name: &str) -> bool
}
```

### inference/types.rs - Type System Utilities

```rust
/// Type system utilities
pub struct TypeSystem;

impl TypeSystem {
    /// Unify two types
    pub fn unify(ty1: &MonoType, ty2: &MonoType, solver: &mut TypeConstraintSolver) -> Result<(), Box<Diagnostic>>

    /// Infer element type from iterable type
    pub fn infer_element_type(iter_ty: &MonoType) -> MonoType

    /// Construct list type
    pub fn make_list_type(elem_ty: MonoType) -> MonoType

    /// Check if type is iterable
    pub fn is_iterable(ty: &MonoType) -> bool

    /// Call trait module to check trait bounds
    pub fn check_trait_bounds(ty: &MonoType, bounds: &[TraitBound], trait_table: &TraitTable) -> Result<(), Box<Diagnostic>>

    /// Call specialization module for instantiation
    pub fn instantiate(ty: &MonoType, args: &[MonoType]) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/statements.rs - Statement Checking

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;
use crate::inference::assignment::AssignmentChecker;
use crate::inference::subtyping::SubtypeChecker;

/// Statement checker
pub struct StatementChecker<'a> {
    scope: &'a mut ScopeManager,
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> StatementChecker<'a> {
    pub fn new(scope: &'a mut ScopeManager, solver: &'a mut TypeConstraintSolver) -> Self

    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            StmtKind::Var { .. } => self.check_var(),
            StmtKind::Fn { .. } => self.check_fn(),
            StmtKind::For { .. } => self.check_for(),
            StmtKind::If { .. } => self.check_if(),
            StmtKind::Expr { .. } => self.check_expr_stmt(),
            // ...
        }
    }

    fn check_var(&mut self, name: &str, init: Option<&Expr>, annot: Option<&Type>) -> Result<(), Box<Diagnostic>>
    fn check_fn(&mut self, ...) -> Result<(), Box<Diagnostic>>
    fn check_for(&mut self, ...) -> Result<(), Box<Diagnostic>>
}
```

### inference/expressions.rs - Expression Inference

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;

/// Expression inferrer (uses unified ScopeManager)
pub struct ExpressionInferrer<'a> {
    scope: &'a ScopeManager,  // Read-only reference
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> ExpressionInferrer<'a> {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            Expr::Lit(..) => self.infer_literal(),
            Expr::Var(..) => self.infer_var(),
            Expr::BinOp(..) => self.infer_binop(),
            Expr::Call(..) => self.infer_call(),
            Expr::For(..) => self.infer_for(),
            Expr::Lambda(..) => self.infer_lambda(),
            // ...
        }
    }

    fn infer_literal(&mut self, lit: &Literal) -> Result<MonoType, Box<Diagnostic>>
    fn infer_var(&mut self, name: &str, span: Span) -> Result<MonoType, Box<Diagnostic>>
    fn infer_binop(&mut self, op: &BinOp, left: &Expr, right: &Expr) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/mod.rs - Unified Entry Point

```rust
// Export all modules
pub mod scope;
pub mod types;
pub mod statements;
pub mod expressions;
pub mod assignment;
pub mod subtyping;
pub mod compatibility;
pub mod bounds;
pub mod generics;
pub mod patterns;

pub use scope::ScopeManager;
pub use types::TypeSystem;
pub use statements::StatementChecker;
pub use expressions::ExpressionInferrer;
pub use assignment::AssignmentChecker;
pub use subtyping::SubtypeChecker;
pub use compatibility::CompatibilityChecker;
pub use bounds::BoundsChecker;

// Unified type checker entry point
pub struct TypeChecker {
    scope: ScopeManager,
    solver: TypeConstraintSolver,
    type_system: TypeSystem,
    // Feature module references
    trait_table: TraitTable,
    specialization_context: SpecializationContext,
}

impl TypeChecker {
    pub fn new() -> Self

    pub fn check_module(&mut self, module: &Module) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        // 1. Collect type definitions
        // 2. Collect function signatures
        // 3. Check all statements
        // 4. Solve constraints
    }
}
```

---

## Refactoring Steps

### Phase 1: Create scope.rs and types.rs ✅

**Goal**: Create base modules

**Deliverables**:
- ✅ `inference/scope.rs` - ScopeManager (with enter_scope/exit_scope/add_var/get_var/update_var/var_in_current_scope/var_in_any_scope/vars/scope_level)
- ✅ `inference/types.rs` - TypeSystem (with unify/infer_element_type/make_list_type/is_iterable)

### Phase 2: Create statements.rs ✅

**Goal**: Merge BodyChecker + StmtInferrer statement checking logic

**Deliverables**:
- ✅ `inference/statements.rs` - StatementChecker (861 lines, containing complete statement checking logic)

**Implementation Details**:
- StatementChecker owns `scope: ScopeManager` and `solver: TypeConstraintSolver`
- `check_expr()` passes `&mut self.scope` and `&mut self.solver` to ExpressionInferrer via Rust partial borrowing, eliminating variable copying
- Backward compatibility alias kept: `pub type BodyChecker = StatementChecker;`

### Phase 3: Create expressions.rs ✅

**Goal**: Merge ExprInferrer expression inference logic

**Deliverables**:
- ✅ `inference/expressions.rs` - ExpressionInferrer (897 lines, uses shared ScopeManager)

**Implementation Details**:
- ExpressionInferrer borrows `scope: &'a mut ScopeManager` and `solver: &'a mut TypeConstraintSolver`
- Constructor signature: `new(scope, solver, overload_candidates)` / `with_native_signatures(scope, solver, overloads, natives)`
- Backward compatibility alias kept: `pub type ExprInferrer<'a> = ExpressionInferrer<'a>;`

### Phase 4: Move files from checking/ to inference/ ✅

**Goal**: Merge checking/ into inference/

**Moves**:
- ✅ `checking/assignment.rs` → `inference/assignment.rs`
- ✅ `checking/subtyping.rs` → `inference/subtyping.rs`
- ✅ `checking/compatibility.rs` → `inference/compatibility.rs`
- ✅ `checking/bounds.rs` → `inference/bounds.rs`

### Phase 5: Modify mod.rs entry point ✅

**File**: `src/frontend/typecheck/mod.rs`

**Changes**:
- ✅ Deleted `pub mod checking;`
- ✅ Updated `pub use inference::*;` exports
- ✅ Updated `infer_expression()` to use ScopeManager + ExpressionInferrer
- ✅ Updated `TypeChecker` references to `inference::StatementChecker`

### Phase 6: Delete old code and directory ✅

**Deletion**:
- ✅ `checking/` directory completely deleted
- ✅ Old BodyChecker code replaced with StatementChecker
- ✅ ExprInferrer.scopes replaced with shared ScopeManager

### Phase 7: Regression testing ✅

```bash
cargo test
# test result: ok. 1434 passed; 0 failed; 4 ignored
# test result: ok. 30 passed; 0 failed
# test result: ok. 5 passed; 0 failed; 11 ignored
```

**Test file updates**:
- ✅ `tests/shadowing.rs` - Updated BodyChecker import path, ExprInferrer added ScopeManager parameter
- ✅ `tests/scope.rs` - ExprInferrer added ScopeManager parameter
- ✅ `tests/infer.rs` - 39 ExprInferrer signature updates, StmtInferrer tests rewritten as StatementChecker
- ✅ `tests/constraint.rs` - 6 places checking:: → inference:: import path updates
- ✅ `tests/basic.rs` - 18 ExprInferrer signature updates

---

## Code to Clean Up

### 1. BodyChecker → statements.rs

| Original Location | Target Location |
|--------|--------|
| `checking/mod.rs` - `BodyChecker` | `inference/statements.rs` - `StatementChecker` |
| `check_stmt`, `check_var_stmt`, etc. | `StatementChecker::check_*` |

### 2. ExprInferrer → expressions.rs

| Original Location | Target Location |
|--------|--------|
| `inference/expressions.rs` - `ExprInferrer` | `inference/expressions.rs` - `ExpressionInferrer` |
| `scopes` field | Use `ScopeManager` |

### 3. checking/ → inference/

| Original Location | Target Location |
|--------|--------|
| `checking/assignment.rs` | `inference/assignment.rs` |
| `checking/subtyping.rs` | `inference/subtyping.rs` |
| `checking/compatibility.rs` | `inference/compatibility.rs` |
| `checking/bounds.rs` | `inference/bounds.rs` |

### 4. Deletions

| Item to Delete | Description |
|--------|------|
| `checking/` directory | Completely deleted |
| `BodyChecker` struct | Migrated to StatementChecker |
| `ExprInferrer.scopes` | Replaced with ScopeManager |

---

## Extensibility Design

### Adding New Statement Types

```rust
// inference/statements.rs
impl StatementChecker {
    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            // ... existing statements
            StmtKind::Match { .. } => self.check_match(),  // 🆕
            StmtKind::While { .. } => self.check_while(),  // 🆕
        }
    }
}
```

### Adding New Expression Types

```rust
// inference/expressions.rs
impl ExpressionInferrer {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            // ... existing expressions
            Expr::Macro { .. } => self.infer_macro(),  // 🆕
            Expr::Await { .. } => self.infer_await(),  // 🆕
        }
    }
}
```

---

## Acceptance Criteria

### Architecture Acceptance

- [x] `inference/scope.rs` independently handles scope management
- [x] `inference/statements.rs` independently handles statement checking
- [x] `inference/expressions.rs` independently handles expression inference
- [x] `inference/types.rs` provides type system utilities
- [x] `inference/assignment.rs`, `subtyping.rs`, `compatibility.rs`, `bounds.rs` work correctly
- [x] Feature modules (specialization/traits/gat/overload) remain independent
- [x] Deleted `checking/` directory
- [x] No manual variable synchronization logic (using shared ScopeManager's Rust partial borrowing pattern)

### Functional Acceptance

| Test Case | Expected Result |
|---------|--------|
| `nums = [1,2,3]; for n in nums { print(n) }` | Compiles successfully |
| `x = 10; for i in 1..3 { x = i }` | Compiles successfully |
| `entry: FileEntry = item` | Type annotation works correctly |

### Regression Testing

```bash
cargo test
```

Expected: All tests pass

---

## Test Plan

### Phase 1: Unit Tests

| Test Name | Module |
|---------|------|
| test_enter_scope | scope.rs |
| test_exit_scope | scope.rs |
| test_add_var | scope.rs |
| test_get_var_outer | scope.rs |
| test_unify_int | types.rs |
| test_infer_element_type | types.rs |

### Phase 2: Integration Tests

| Test Name | Test Code | Expected Result |
|---------|---------|--------|
| test_for_list | `for n in [1,2,3] { print(n) }` | Compiles successfully |
| test_var_scope | Variable scope correct | Pass |
| test_type_annotation | `x: Int = 1` | Compiles successfully |
| test_generic_fn | Generic function | Works correctly |
| test_trait_bound | trait bounds | Works correctly |

### Phase 3: Regression Testing

```bash
cargo test
```

Expected: All tests pass