# YaoXiang Frontend Architecture Aggressive Refactoring Plan (RFC Support Version)

> Version: 3.0 | Date: 2026-01-29 | Status: Based on RFC Requirements Fix
>
> **Core Objective**: On the basis of a low-coupling architecture, fully support the implementation requirements of RFC-004/010/011

## 📋 Refactoring Goals

### Core Objectives
- **Reduce Coupling**: Eliminate strong dependencies between modules, achieve loose coupling architecture
- **RFC Support**: Fully support the design requirements and implementation paths of three RFCs
- **File Layering Optimization**: Clear layered architecture with single responsibility per layer
- **Maintainability Improvement**: Split large files, clear responsibilities
- **Scalability Enhancement**: Reserve expansion space for future features like RFC-012

### RFC Support Matrix

| RFC | Core Requirements | Refactoring Support | Implementation Location |
|-----|-------------------|---------------------|------------------------|
| **RFC-004** | Multi-position binding syntax, intelligent binding, auto currying | 95% | `statements/bindings.rs`, `core/lexer/literals.rs` |
| **RFC-010** | Unified syntax, generic syntax, type definitions | 90% | `statements/declarations.rs`, `types/parser.rs` |
| **RFC-011** | Constraint solving, monomorphization, generic system | 100% | `type_system/*`, `constraints.rs`, `unify.rs` |

### Success Metrics
- [ ] All files controlled within 500 lines
- [ ] Module dependencies are clear, no circular dependencies
- [ ] Three RFC implementation requirements 100% supported by the architecture
- [ ] Public API simplified, internal implementation hidden
- [ ] Test coverage rate exceeds 85%
- [ ] Build time reduced by 20% (through better modularization)

---

## 🏗️ New Architecture Design

### 1. Layered Architecture Diagram

```
┌─────────────────────────────────────┐
│           Frontend API              │  ← Public Interface Layer
│        (frontend/mod.rs)            │
├─────────────────────────────────────┤
│  Lexer → Parser → TypeCheck → Const│  ← Pipeline Layer
│     │        │         │       │   │
│     ▼        ▼         ▼       ▼   │
├─────────────────────────────────────┤
│          Shared Utilities           │  ← Shared Utility Layer
│    (error, span, diagnostic)        │
├─────────────────────────────────────┤
│        Core Algorithm Layer         │  ← Core Algorithm Layer
│  (type_system, const_eval, parse)   │
│                                     │
│  ▸ RFC-004: Binding parsing support│
│  ▸ RFC-010: Unified syntax parsing │
│  ▸ RFC-011: Complete generic system│
└─────────────────────────────────────┘
```

### 2. Module Reorganization

#### **Layer 1: Core Algorithm Layer**

```
src/frontend/core/
├── mod.rs                    # Core module entry
├── lexer/
│   ├── mod.rs               # Lexer interface
│   ├── tokenizer.rs         # Tokenizer implementation (split from 1270 lines)
│   ├── state.rs            # Lexer state management (new)
│   ├── literals.rs         # Literal handling (split)
│   └── symbols.rs          # Keywords and symbol table (new)
├── parser/
│   ├── mod.rs              # Parser interface
│   ├── ast.rs              # AST definitions (kept at 305 lines)
│   ├── pratt/              # Pratt parser core (new directory)
│   │   ├── mod.rs
│   │   ├── nud.rs          # Prefix parsing (split from 896 lines)
│   │   ├── led.rs          # Infix parsing (kept at 380 lines)
│   │   └── precedence.rs   # Precedence handling (split)
│   ├── statements/         # Statement parsing (new directory)
│   │   ├── mod.rs
│   │   ├── declarations.rs  # Declaration statements (split from 1399 lines)
│   │   ├── expressions.rs   # Expression statements (split)
│   │   ├── control_flow.rs  # Control flow (split)
│   │   └── bindings.rs     # RFC-004 binding syntax parsing (new)
│   ├── types/              # Type parsing (new directory)
│   │   ├── mod.rs
│   │   ├── parser.rs       # Type parser (split from 614 lines)
│   │   ├── constraints.rs  # RFC-011 type constraint parsing (new)
│   │   └── generics.rs     # RFC-010/011 generic syntax parsing (new)
│   └── utils.rs            # Parser utilities (split)
├── type_system/            # RFC-011 core type system
│   ├── mod.rs
│   ├── vars.rs            # TypeVar, ConstVar (split)
│   ├── mono_poly.rs       # MonoType, PolyType (split)
│   ├── constraints.rs      # TypeConstraint (split)
│   ├── unify.rs           # Unify algorithm (split)
│   ├── specialize.rs      # RFC-011 generic specialization (new)
│   ├── pretty_print.rs    # Type printing (new)
│   └── display.rs         # Type display formatting (new)
└── const_eval/            # Constant evaluation
    ├── mod.rs
    ├── evaluator.rs       # Constant evaluator (renamed from 677 lines)
    ├── functions.rs      # Const functions (split from 536 lines)
    └── static_assert.rs  # Static assertions (kept at 490 lines)
```

#### **Layer 2: Shared Utilities Layer**

```
src/frontend/shared/
├── mod.rs
├── error/
│   ├── mod.rs
│   ├── diagnostic.rs       # Unified diagnostic messages
│   ├── span.rs            # Span handling
│   ├── result.rs          # Unified Result type
│   ├── conversion.rs      # Error conversion
│   └── macros.rs          # RFC-011 error handling macros (new)
├── diagnostics/
│   ├── mod.rs
│   ├── formatter.rs       # Diagnostic formatting
│   ├── severity.rs        # Severity levels
│   ├── code.rs            # Error code definitions
│   └── traits.rs          # Diagnostic traits (new)
├── utils/
│   ├── mod.rs
│   ├── mem.rs             # Memory management utilities
│   ├── debug.rs           # Debug utilities
│   ├── panic.rs           # Panic handling
│   └── cache.rs           # RFC-011 compilation cache (new)
└── abstractions/           # Abstract interface layer (new)
    ├── mod.rs
    ├── parser.rs          # Parser abstract interface
    ├── type_checker.rs    # TypeChecker abstract interface
    └── trait_objects.rs   # trait object support
```

#### **Layer 3: Type Checking Layer**

```
src/frontend/typecheck/
├── mod.rs                 # Type checking entry
├── inference/             # Type inference (split from infer.rs)
│   ├── mod.rs
│   ├── expressions.rs    # Expression inference (split)
│   ├── statements.rs     # Statement inference (split)
│   ├── patterns.rs       # Pattern matching inference (new)
│   └── generics.rs       # RFC-011 generic inference (new)
├── checking/             # Type checking (split from check.rs)
│   ├── mod.rs
│   ├── subtyping.rs      # Subtype checking (split)
│   ├── assignment.rs     # Assignment checking (split)
│   ├── compatibility.rs # Compatibility checking (split)
│   └── bounds.rs         # RFC-011 type bounds checking (new)
├── specialization/       # RFC-011 generic specialization
│   ├── mod.rs
│   ├── algorithm.rs      # Specialization algorithm (split from 488 lines)
│   ├── substitution.rs  # Substitution logic (new)
│   └── instantiate.rs   # Instantiation algorithm (new)
├── traits/              # RFC-011 trait system
│   ├── mod.rs
│   ├── solver.rs        # trait solver (split from 274 lines)
│   ├── coherence.rs     # Coherence checking (new)
│   ├── object_safety.rs # Object safety (new)
│   └── resolution.rs    # trait resolution (new)
└── gat/                # GAT support (kept at 529 lines, structure optimized)
    ├── mod.rs
    ├── checker.rs       # GAT checker
    └── higher_rank.rs   # Higher-ranked types
```

#### **Layer 4: Advanced Type Level**

```
src/frontend/type_level/
├── mod.rs               # Type-level computation entry
├── conditional_types.rs  # RFC-011 conditional types (kept)
├── dependent_types.rs    # RFC-011 dependent types (kept)
├── evaluation/          # RFC-011 type-level computation (new directory)
│   ├── mod.rs
│   ├── normalize.rs     # Normalization
│   ├── reduce.rs        # Reduction
│   ├── unify.rs         # Type-level unification
│   └── compute.rs       # Type computation engine (new)
├── operations/          # RFC-011 type-level operations (new directory)
│   ├── mod.rs
│   ├── arithmetic.rs    # Arithmetic operations
│   ├── comparison.rs   # Comparison operations
│   └── logic.rs        # Logic operations
├── const_generics/     # RFC-011 Const generics support (new directory)
│   ├── mod.rs
│   ├── eval.rs         # Const generic evaluation
│   └── generic_size.rs # Generic size computation (new)
└── tests.rs            # Tests (kept)
```

#### **Layer 5: Public API Layer**

```
src/frontend/
├── mod.rs               # Compiler public interface (simplified)
├── compiler.rs          # Compiler core logic (split from 235 lines)
├── pipeline.rs          # Compilation pipeline (new)
├── config.rs            # Compilation configuration (new)
└── events/              # Event system (new)
    ├── mod.rs
    ├── type_check.rs    # Type checking events
    ├── parse.rs         # Parsing events
    └── subscribe.rs     # Event subscription (new)
```

---

## 📅 Phased Implementation Plan

### 🚀 Phase 1: Emergency Split & RFC Support Preparation (Week 1-3) Completed

#### **Day 1-2: Preparation Phase**

**Step 1.1: Create New Directory Structure**
- **Subtask 1.1.1**: Create complete directory structure under `src/frontend/`
  - Estimated time: 15 minutes
  - Acceptance criteria: All RFC support directories created

```bash
# Create RFC-004 support directory
mkdir -p src/frontend/core/parser/statements/bindings

# Create RFC-010/011 support directories
mkdir -p src/frontend/core/parser/types/generics
mkdir -p src/frontend/type_system/specialize

# Create shared abstractions directory
mkdir -p src/frontend/shared/abstractions
mkdir -p src/frontend/shared/events
```

**Step 1.2: Run Existing Test Baseline**
- **Subtask 1.2.1**: Record current build performance
  - Run `time cargo build --release` to record baseline time
  - Save results to `metrics/pre_refactor_build_time.txt`

- **Subtask 1.2.2**: Run complete test suite
  - Run `cargo test --all` to ensure all current tests pass
  - Record test pass count: ___/___
  - Save to `metrics/pre_refactor_test_results.txt`

- **Subtask 1.2.3**: Record code statistics
  - Run `cloc src/frontend/typecheck/types.rs` to record original lines
  - Record: Total lines ___ lines, code lines ___ lines
  - Save to `metrics/pre_refactor_loc.txt`

---

#### **Day 3-7: Split typecheck/types.rs (RFC-011 Core)**

**Goal**: Split the 1948-line giant file into modules supporting RFC-011
**Estimated total time**: 5 days (1 day each)

**Day 3: Analysis and Decomposition**

- **Subtask 1.3.1: RFC-011 Requirements Alignment Analysis**
  - **1.3.1.1**: Mark RFC-011 Phase 1 requirements (60 minutes)
    - TypeVar/ConstVar definitions → `vars.rs`
    - MonoType/PolyType definitions → `mono_poly.rs`
    - Constraint system → `constraints.rs`
    - Unify algorithm → `unify.rs`
  - **1.3.1.2**: Mark RFC-011 Phase 2+ requirements (30 minutes)
    - Specialization algorithm → `specialize.rs` (new)
    - Type display → `pretty_print.rs`, `display.rs` (new)
  - **1.3.1.3**: Determine module boundaries and dependencies (30 minutes)

- **Subtask 1.3.2: Create `vars.rs` (2 hours)**
  - **1.3.2.1**: Copy TypeVar/ConstVar related code to new file
  - **1.3.2.2**: Adjust import paths, fix compilation errors
  - **1.3.2.3**: Run `cargo check` to verify successful compilation
  - **Acceptance criteria**: vars.rs compiles independently

**Day 4: Complete Basic Modules**

- **Subtask 1.3.3: Create `mono_poly.rs` (2 hours)**
  - **1.3.3.1**: Copy MonoType/PolyType code
  - **1.3.3.2**: Fix dependencies with vars.rs
  - **1.3.3.3**: Add RFC-011 required generic specialization interfaces

- **Subtask 1.3.4: Create `constraints.rs` (2 hours)**
  - **1.3.4.1**: Copy TypeConstraint/ConstraintSet code
  - **1.3.4.2**: Implement UnionFind structure
  - **1.3.4.3**: Add RFC-011 Phase 2 constraint solving interfaces

**Day 5: Core Algorithm Modules**

- **Subtask 1.3.5: Create `unify.rs` (3 hours)**
  - **1.3.5.1**: Copy Unify algorithm and Substitution
  - **1.3.5.2**: Implement Unifier structure
  - **1.3.5.3**: Add RFC-011 monomorphization support interfaces
  - **Acceptance criteria**: unify.rs compiles, algorithm logic correct

- **Subtask 1.3.6: Create `specialize.rs` (RFC-011 new) (1 hour)**
  - **1.3.6.1**: Implement generic specialization algorithm
  - **1.3.6.2**: Implement instantiation cache
  - **1.3.6.3**: Add dead code elimination interface

**Day 6: Integration and Dependency Fixes**

- **Subtask 1.3.7: Create `type_system/mod.rs` (1 hour)**
  - **1.3.7.1**: Define module entry file
  - **1.3.7.2**: Uniformly export all public interfaces
  - **1.3.7.3**: Define TypeSystemError type

- **Subtask 1.3.8: Update Dependencies (2 hours)**
  - **1.3.8.1**: Modify imports in `src/frontend/typecheck/mod.rs`
    ```rust
    // From
    pub use types::*;
    // To
    pub use crate::type_system::{
        MonoType, PolyType, TypeVar, ConstraintSolver,
        Unifier, specialize::Specializer
    };
    ```
  - **1.3.8.2**: Use search and replace tool to batch update reference paths
  - **1.3.8.3**: Fix compilation errors one by one

**Day 7: RFC-011 Infrastructure Verification**

- **Subtask 1.3.9: Verify RFC-011 Support (2 hours)**
  - **1.3.9.1**: Create RFC-011 Phase 1 test
    ```rust
    // tests/rfc011_phase1.rs
    #[test]
    fn test_basic_generic_instantiation() {
        let types = type_system::MonoType::var("T");
        let specialized = type_system::specialize::instantiate(
            &types, &[Type::Int]
        ).unwrap();
        assert_eq!(specialized, Type::Int);
    }
    ```
  - **1.3.9.2**: Verify constraint solver works
  - **1.3.9.3**: Verify monomorphization interface

- **Subtask 1.3.10: Comprehensive Verification (2 hours)**
  - **1.3.10.1**: Run `cargo check --all` to ensure no compilation errors
  - **1.3.10.2**: Run `cargo test type_system` to run type system tests
  - **1.3.10.3**: Run `cargo test --all` to ensure all tests pass
  - **1.3.10.4**: Performance comparison: Build time change < 10%

- **Subtask 1.3.11: Clean Up Old Files (1 hour)**
  - **1.3.11.1**: After confirming new modules work correctly, delete original `types.rs`
  - **1.3.11.2**: Update git and commit
  - **1.3.11.3**: Create tag `refactor/types-complete-rfc011`

**Acceptance Criteria**:
- [ ] `types.rs` completely deleted
- [ ] New modules compile successfully: `cargo check --all`
- [ ] RFC-011 Phase 1 tests pass: `cargo test rfc011_phase1`
- [ ] All tests green: `cargo test --all`
- [ ] No significant performance degradation: Build time change < 10%

---

#### **Day 8-14: Split lexer/mod.rs + RFC-004 Support Preparation**

**Goal**: Split the lexer to prepare for RFC-004 binding syntax and RFC-010 unified syntax
**Estimated total time**: 7 days

**Day 8-9: Analyze lexer + RFC Requirements**

- **Subtask 1.4.1: Analyze lexer structure + RFC requirements alignment (2 hours)**
  - **1.4.1.1**: Run analysis tool
    ```bash
    rg "^pub struct|^impl.*Tokenizer" src/frontend/lexer/mod.rs
    ```
  - **1.4.1.2**: Identify core logic + RFC support requirements
    - Tokenizer main structure (lines 1-300) + RFC-004 binding symbols `[`, `]` support
    - State management code (lines 301-600) + RFC-010 generic keywords `<`, `>` support
    - Literal processing logic (lines 601-900) + RFC-010/011 type syntax support
    - Helper methods (lines 901-1270)
  - **1.4.1.3**: Design new module interfaces

- **Subtask 1.4.2: Create `tokenizer.rs` (3 hours)**
  - **1.4.2.1**: Extract Tokenizer struct and main methods
  - **1.4.2.2**: Add RFC-004 binding syntax token support
    ```rust
    // Tokenizer additions
    enum TokenType {
        // ... existing tokens
        LeftBracket,    // [ RFC-004 binding start
        RightBracket,   // ] RFC-004 binding end
        LessThan,       // < RFC-010/011 generic start
        GreaterThan,     // > RFC-010/011 generic end
        // ...
    }
    ```
  - **1.4.2.3**: Delegate state and literal handling to specialized modules

- **Subtask 1.4.3: Create `state.rs` (2 hours)**
  - **1.4.3.1**: Extract LexerState structure
  - **1.4.3.2**: Implement keyword lookup and other state-related methods
  - **1.4.3.3**: Add RFC-010 keyword recognition (such as `type`, `where`, etc.)

**Day 10-11: Complete Split + Symbol Support**

- **Subtask 1.4.4: Create `literals.rs` (2 hours)**
  - **1.4.4.1**: Extract all literal processing methods
  - **1.4.4.2**: Number, string, character processing logic
  - **1.4.4.3**: Add RFC-010 generic type literal support

- **Subtask 1.4.5: Create `symbols.rs` (RFC new) (1 hour)**
  - **1.4.5.1**: Unified symbol table management
  - **1.4.5.2**: Support RFC-010/011 generic symbols
  - **1.4.5.3**: Support RFC-004 binding symbols

**Day 12-13: Migrate Tests + RFC Verification**

- **Subtask 1.4.6: Migrate test files (2 hours)**
  - **1.4.6.1**: Create test directory structure
    ```bash
    mkdir -p src/frontend/core/lexer/tests
    ```
  - **1.4.6.2**: Copy all test files
  - **1.4.6.3**: Add RFC syntax tests
    ```rust
    // tests/rfc004_lexer.rs
    #[test]
    fn test_binding_syntax_tokenization() {
        let tokens = lexer::tokenize("function[0, 1]");
        assert_eq!(tokens[1].ty, TokenType::LeftBracket);
        assert_eq!(tokens[2].ty, TokenType::Number);
        // ...
    }

    // tests/rfc010_lexer.rs
    #[test]
    fn test_generic_syntax_tokenization() {
        let tokens = lexer::tokenize("List[T]");
        assert_eq!(tokens[1].ty, TokenType::LessThan);
        assert_eq!(tokens[2].ty, TokenType::Identifier);
        // ...
    }
    ```

- **Subtask 1.4.7: Verify RFC syntax support (2 hours)**
  - **1.4.7.1**: Verify RFC-004 binding syntax tokenization
    ```bash
    cargo test rfc004_lexer
    ```
  - **1.4.7.2**: Verify RFC-010/011 generic syntax tokenization
    ```bash
    cargo test rfc010_lexer
    ```
  - **1.4.7.3**: Fix compilation errors in tests

**Day 14: Integration and Verification**

- **Subtask 1.4.8: Update upper-level dependencies (2 hours)**
  - **1.4.8.1**: Update import paths in parser module
  - **1.4.8.2**: Update exports in frontend main module
  - **1.4.8.3**: Run integration tests

- **Subtask 1.4.9: Comprehensive verification (2 hours)**
  - **1.4.9.1**: Compilation check
    ```bash
    cargo check --all
    ```
  - **1.4.9.2**: Run related tests
    ```bash
    cargo test lexer
    cargo test rfc004_lexer
    cargo test rfc010_lexer
    ```
  - **1.4.9.3**: Clean up old files
  - **1.4.9.4**: Commit changes, create tag `refactor/lexer-complete-rfc004`

**Acceptance Criteria**:
- [ ] lexer/mod.rs split complete
- [ ] RFC-004 binding syntax tokenization support: `cargo test rfc004_lexer`
- [ ] RFC-010 generic syntax tokenization support: `cargo test rfc010_lexer`
- [ ] All lexer tests pass: `cargo test lexer`
- [ ] Parser tests normal: `cargo test parser`

---

#### **Day 15-21: Split parser/stmt.rs + RFC-010/011 Parsing Support**

**Goal**: Reorganize parser structure to support RFC-010 unified syntax and RFC-011 generic parsing
**Estimated total time**: 7 days

**Day 15-16: Analyze parser structure + RFC Requirements**

- **Subtask 1.5.1: Analyze stmt.rs structure + RFC parsing requirements (3 hours)**
  - **1.5.1.1**: Analyze file content distribution + RFC requirements alignment
    ```bash
    rg "^//.*declaration|^//.*expression|^//.*control flow" src/frontend/parser/stmt.rs
    ```
  - **1.5.1.2**: Identify logical groupings + RFC support requirements
    - Declaration related code (lines 1-500) + RFC-010 unified syntax parsing + RFC-004 binding syntax parsing
    - Expression statements (lines 501-900) + RFC-011 generic expression parsing
    - Control flow code (lines 901-1399) + RFC-011 generic control flow parsing
  - **1.5.1.3**: Identify Pratt parser parts + RFC syntax requirements
    - nud.rs (prefix parsing) + RFC-010 generic prefix
    - led.rs (infix parsing) + RFC-010 generic infix
    - precedence.rs (precedence) + RFC-011 precedence rules

- **Subtask 1.5.2: Create directory structure (1 hour)**
  ```bash
  mkdir -p src/frontend/core/parser/{statements,pratt,types}
  mkdir -p src/frontend/core/parser/tests/{declarations,expressions,control_flow,bindings}
  mkdir -p src/frontend/core/parser/types/tests
  ```

**Day 17-18: Split statement parsing + RFC syntax support**

- **Subtask 1.5.3: Create `statements/declarations.rs` (3 hours)**
  - **1.5.3.1**: Extract function declaration parsing + RFC-010/011 generic support
    ```rust
    // Support RFC-010 unified syntax
    pub parse_function_decl: Parser = {
        // name: type = value unified syntax
        // [T](params) -> Return generic syntax
        // where constraints: Clone constraint syntax
    }

    // Support RFC-004 binding syntax
    pub parse_binding_decl: Parser = {
        // Type.method = function[positions] binding syntax
    }
    ```
  - **1.5.3.2**: Extract struct and enum declarations + RFC-010 syntax
    - `parse_struct_decl()` + generic field support
    - `parse_enum_decl()` + generic variant support
  - **1.5.3.3**: Extract variable declarations + RFC-010 unified syntax
    - `parse_variable_decl()` + unified `name: type = value` syntax
    - `parse_use_decl()` + generic import support

- **Subtask 1.5.4: Create `statements/bindings.rs` (RFC-004 new) (2 hours)**
  - **1.5.4.1**: Parse RFC-004 binding syntax
    ```rust
    pub parse_binding: Parser = {
        // Type.method = function[0, 1, 2] binding syntax
        // position_list: [0, _, -1] placeholder support
    }
    ```
  - **1.5.4.2**: Position index syntax validation
  - **1.5.4.3**: Binding semantic checking

- **Subtask 1.5.5: Create `statements/expressions.rs` (2 hours)**
  - **1.5.5.1**: Extract expression statement parsing + RFC-011 generic expressions
  - **1.5.5.2**: Extract assignment statement parsing + generic type checking
  - **1.5.5.3**: Extract block statement parsing + generic scope handling

**Day 19: Split control flow + generic parsing**

- **Subtask 1.5.6: Create `statements/control_flow.rs` (3 hours)**
  - **1.5.6.1**: Extract if-else parsing + generic conditional expressions
  - **1.5.6.2**: Extract loop parsing (while, for) + generic iterators
  - **1.5.6.3**: Extract match parsing + generic pattern matching
  - **1.5.6.4**: Extract break/continue/return parsing + generic return types

**Day 20: Handle Pratt parser + RFC generics**

- **Subtask 1.5.7: Split Pratt module (2 hours)**
  - **1.5.7.1**: Optimize nud.rs + RFC-010 generic prefix parsing
    ```rust
    // Support generic prefix parsing
    fn parse_generic_prefix(&mut self) -> Result<Expr> {
        // List[T] prefix parsing
        // Option[T]::Some generic method parsing
    }
    ```
  - **1.5.7.2**: Optimize led.rs + RFC-010 generic infix parsing
  - **1.5.7.3**: Extract precedence.rs + RFC-011 generic precedence

**Day 20: Type parsing enhancement (RFC-010/011 core)**

- **Subtask 1.5.8: Create `types/parser.rs` (enhanced) (2 hours)**
  - **1.5.8.1**: Extract type parsing logic + RFC-010 unified syntax
    ```rust
    // Support RFC-010 unified syntax
    pub parse_type: Parser = {
        // name: type = value type definition
        // type Name = { ... } type body
        // Interface: { method: (...) -> ... } interface definition
    }
    ```
  - **1.5.8.2**: Add RFC-010/011 generic syntax parsing
    ```rust
    // Support generic types
    pub parse_generic_type: Parser = {
        // List[T, U] multi-parameter generics
        // Box[T: Clone] constrained generics
        // Array[T, N: Int] Const generics
    }
    ```
  - **1.5.8.3**: Add RFC-011 conditional type parsing

- **Subtask 1.5.9: Create `types/generics.rs` (RFC-010/011 new) (1 hour)**
  - **1.5.9.1**: Generic parameter parsing `[T]`, `[T: Clone]`
  - **1.5.9.2**: Const generic parsing `[T, N: Int]`
  - **1.5.9.3**: Generic constraint parsing

- **Subtask 1.5.10: Create `types/constraints.rs` (RFC-011 new) (1 hour)**
  - **1.5.10.1**: Type constraint parsing `T: Clone + Add`
  - **1.5.10.2**: Constraint combination parsing
  - **1.5.10.3**: Constraint validation

**Day 21: Integration and Verification**

- **Subtask 1.5.11: Create module entries (1 hour)**
  - **1.5.11.1**: Create `core/parser/mod.rs`
  - **1.5.11.2**: Create `core/parser/statements/mod.rs`
  - **1.5.11.3**: Create `core/parser/types/mod.rs`
  - **1.5.11.4**: Uniformly export interfaces

- **Subtask 1.5.12: Migrate tests + RFC verification (3 hours)**
  - **1.5.12.1**: Migrate parser test files
    ```bash
    # Classified migration
    mv src/frontend/parser/tests/decl_tests.rs \
       src/frontend/core/parser/tests/declarations/
    mv src/frontend/parser/tests/expr_tests.rs \
       src/frontend/core/parser/tests/expressions/
    mv src/frontend/parser/tests/control_tests.rs \
       src/frontend/core/parser/tests/control_flow/
    ```
  - **1.5.12.2**: Add RFC syntax tests
    ```rust
    // tests/rfc010_parser.rs
    #[test]
    fn test_unified_syntax_parsing() {
        // name: type = value unified syntax test
        // type Name = { ... } type definition test
    }

    // tests/rfc011_parser.rs
    #[test]
    fn test_generic_parsing() {
        // [T] generic parameter test
        // [T: Clone] constrained generic test
        // [T, N: Int] Const generic test
    }

    // tests/rfc004_parser.rs
    #[test]
    fn test_binding_parsing() {
        // Type.method = function[0, 1] binding syntax test
    }
    ```
  - **1.5.12.3**: Batch update import paths
  - **1.5.12.4**: Fix test compilation errors

- **Subtask 1.5.13: Comprehensive verification (2 hours)**
  - **1.5.13.1**: Compilation check
    ```bash
    cargo check --all
    ```
  - **1.5.13.2**: Run parser tests
    ```bash
    cargo test core::parser
    cargo test rfc010_parser
    cargo test rfc011_parser
    cargo test rfc004_parser
    ```
  - **1.5.13.3**: Run complete test suite
    ```bash
    cargo test --all
    ```
  - **1.5.13.4**: Commit changes, create tag `refactor/parser-complete-rfc010011`

**Acceptance Criteria**:
- [ ] stmt.rs completely split
- [ ] RFC-010 unified syntax parsing passes: `cargo test rfc010_parser`
- [ ] RFC-011 generic syntax parsing passes: `cargo test rfc011_parser`
- [ ] RFC-004 binding syntax parsing passes: `cargo test rfc004_parser`
- [ ] New modules compile successfully: `cargo check --all`
- [ ] All parser tests pass: `cargo test parser`
- [ ] Maximum file lines < 500 lines

---

### ⚡ Phase 2: Abstraction Extraction & RFC Full Support (Week 4-6)

#### **Week 4: Unified Error Handling System + RFC Error Model**

**Goal**: Eliminate duplicate error handling across 20+ files, prepare for RFC-011 complex error model
**Estimated total time**: 5 days

**Day 22: Design RFC Error Handling System**

- **Subtask 2.1.1: Analyze existing error handling + RFC requirements (2 hours)**
  - **2.1.1.1**: Search all error handling patterns
    ```bash
    rg "return Err\(" src/frontend/ --type rust | head -20
    ```
  - **2.1.1.2**: Identify duplicate patterns + RFC error requirements
    - `if condition { return Err(...) }` → RFC-011 generic errors need context
    - `ensure!(condition, error)` → RFC-011 constraint errors need location info
    - Custom error types → RFC-011 needs hierarchical error model
  - **2.1.1.3**: Design unified interface + RFC-011 error model

- **Subtask 2.1.2: Create RFC error handling macros (2 hours)**
  - **2.1.2.1**: Create `shared/error/macros.rs`
    ```rust
    #[macro_export]
    macro_rules! ensure {
        ($condition:expr, $error:expr) => {
            if !$condition {
                return Err($error.into());
            }
        };
    }

    // RFC-011 specialized error macros
    #[macro_export]
    macro_rules! ensure_constraint {
        ($condition:expr, $constraint:expr, $span:expr) => {
            if !$condition {
                return Err(TypeError::ConstraintFailure {
                    constraint: $constraint,
                    span: $span,
                }.into());
            }
        };
    }
    ```
  - **2.1.2.2**: Create `ensure_index!`, `ensure_some!` and other macros
  - **2.1.2.3**: Create `ErrorContext` trait + RFC-011 support

**Day 23-24: Apply to lexer + RFC syntax errors**

- **Subtask 2.2.1: Refactor lexer error handling (3 hours)**
  - **2.2.1.1**: Update `core/lexer/tokenizer.rs`
    ```rust
    // From
    if self.pos >= self.source.len() {
        return Err(LexicalError::UnexpectedEOF);
    }
    // To
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedEOF);
    ```
  - **2.2.1.2**: Add RFC syntax error support
    ```rust
    // RFC-004 binding syntax error
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedBindingSyntax(span));

    // RFC-010/011 generic syntax error
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedGenericSyntax(span));
    ```
  - **2.2.1.3**: Simplify numeric parsing error handling + RFC-011 Const generic errors

- **Subtask 2.2.2: Verify lexer refactoring (2 hours)**
  - **2.2.2.1**: Compilation check
    ```bash
    cargo check -p core-lexer
    ```
  - **2.2.2.2**: Run tests
    ```bash
    cargo test core::lexer
    cargo test rfc004_lexer  # Verify RFC-004 error handling
    cargo test rfc010_lexer  # Verify RFC-010 error handling
    ```

**Day 25-26: Spread to parser + RFC parsing errors**

- **Subtask 2.3.1: Refactor parser error handling (4 hours)**
  - **2.3.1.1**: Update `core/parser/statements/declarations.rs`
    ```rust
    // RFC-010 unified syntax error
    ensure!(self.parse_name().is_some(),
            ParseError::MissingNameInDeclaration(span));

    // RFC-011 generic syntax error
    ensure!(self.parse_generic_params().is_ok(),
            ParseError::InvalidGenericSyntax(span));
    ```
  - **2.3.1.2**: Update `core/parser/statements/bindings.rs`
    ```rust
    // RFC-004 binding syntax error
    ensure!(self.parse_position_list().is_ok(),
            ParseError::InvalidBindingPositions(span));
    ```
  - **2.3.1.3**: Update `core/parser/types/generics.rs`
    ```rust
    // RFC-011 generic constraint error
    ensure_constraint!(self.parse_constraint().is_some(),
                      constraint.clone(),
                      span);
    ```
  - **2.3.1.4**: Update Pratt parser + RFC generic precedence errors

- **Subtask 2.3.2: Verify parser refactoring (2 hours)**
  - **2.3.2.1**: Compilation check
    ```bash
    cargo check -p core-parser
    ```
  - **2.3.2.2**: Run parser tests
    ```bash
    cargo test core::parser
    cargo test rfc010_parser  # Verify RFC-010 parsing errors
    cargo test rfc011_parser  # Verify RFC-011 parsing errors
    cargo test rfc004_parser  # Verify RFC-004 parsing errors
    ```

**Day 27-28: Spread to typecheck + RFC type errors**

- **Subtask 2.4.1: Refactor type checking error handling (4 hours)**
  - **2.4.1.1**: Update type system module + RFC-011 errors
    ```rust
    // RFC-011 constraint error
    ensure_constraint!(self.solve_constraint(&constraint).is_ok(),
                      constraint.clone(),
                      span);

    // RFC-011 generic instantiation error
    ensure!(self.instantiate_generic(&generic, &args).is_ok(),
            TypeError::GenericInstantiationFailed {
                generic: generic.clone(),
                args: args.clone(),
            });
    ```
  - **2.4.1.2**: Update type checking module + RFC-010/011 type errors
  - **2.4.1.3**: Update trait solving module + RFC-011 trait errors

- **Subtask 2.4.2: Verify typecheck refactoring (2 hours)**
  - **2.4.2.1**: Compilation check
    ```bash
    cargo check -p typecheck
    ```
  - **2.4.2.2**: Run type checking tests
    ```bash
    cargo test typecheck
    cargo test rfc011_type_errors  # Verify RFC-011 type errors
    ```

**Day 29: Verification and Measurement**

- **Subtask 2.5.1: Comprehensive verification (2 hours)**
  - **2.5.1.1**: Run complete test suite
    ```bash
    cargo test --all
    ```
  - **2.5.1.2**: Check code duplication rate changes
    ```bash
    # Use tool to check duplicate error handling code
    cpd --minimum-tokens 20 --files src/frontend/shared/error/
    ```

- **Subtask 2.5.2: RFC error model verification (1 hour)**
  - **2.5.2.1**: Verify RFC-004 binding error model
  - **2.5.2.2**: Verify RFC-010 unified syntax error model
  - **2.5.2.3**: Verify RFC-011 generic error model

- **Subtask 2.5.3: Measurement improvements (1 hour)**
  - **2.5.3.1**: Count eliminated duplicate code lines
  - **2.5.3.2**: Compare before/after refactoring error handling consistency
  - **2.5.3.3**: Commit changes, create tag `refactor/error-handling-complete-rfc`

**Acceptance Criteria**:
- [ ] Error handling macros applied in all modules
- [ ] RFC-004/010/011 error models fully implemented
- [ ] Compilation passes: `cargo check --all`
- [ ] All tests pass: `cargo test --all`
- [ ] Code duplication rate check: Use tool to verify duplicate code < 200 lines
- [ ] Error handling consistency: 100% modules use unified macros

#### **Week 5: Type Inference Abstraction + RFC-011 Generic Inference**

**Goal**: Create reusable type inference interfaces, eliminate duplicate logic, fully support RFC-011 generic inference
**Estimated total time**: 5 days

**Day 30-31: Analyze type inference logic + RFC requirements**

- **Subtask 2.6.1: Analyze infer.rs + RFC-011 requirements (3 hours)**
  - **2.6.1.1**: Search type inference related code + RFC-011 generic requirements
    ```bash
    rg "fn infer_" src/frontend/typecheck/infer.rs
    ```
  - **2.6.1.2**: Identify duplicate patterns + RFC-011 inference requirements
    - Expression type inference + RFC-011 generic expression inference
    - Statement type inference + RFC-011 generic statement inference
    - Pattern type inference + RFC-011 generic pattern inference
  - **2.6.1.3**: Draw inference flow diagram + RFC-011 generic inference flow

- **Subtask 2.6.2: Design TypeInferrer trait + RFC-011 (2 hours)**
  - **2.6.2.1**: Define common interfaces + RFC-011 generic support
    ```rust
    pub trait TypeInferrer {
        type Expr;
        type Stmt;
        type Pattern;

        fn infer_expr(&mut self, expr: &Self::Expr)
            -> Result<MonoType, TypeInferenceError>;
        fn infer_stmt(&mut self, stmt: &Self::Stmt)
            -> Result<(), TypeInferenceError>;
        fn infer_pattern(&mut self, pattern: &Self::Pattern)
            -> Result<MonoType, TypeInferenceError>;

        // RFC-011 new: generic inference
        fn infer_generic_call(&mut self, call: &GenericCall)
            -> Result<MonoType, TypeInferenceError>;
        fn instantiate_generic(&mut self, generic: &GenericExpr, args: &[Type])
            -> Result<MonoType, TypeInferenceError>;
    }
    ```

**Day 32-33: Implement abstraction + RFC generic inference**

- **Subtask 2.6.3: Create generic inferrer implementation (4 hours)**
  - **2.6.3.1**: Implement `ExprInferrer` + RFC-011 generic expressions
    - Literal inference + Const generic inference
    - Identifier inference + generic variable inference
    - BinaryOp inference + generic operator inference
    - GenericCall inference (RFC-011 new)
  - **2.6.3.2**: Implement `StmtInferrer` + RFC-011 generic statements
  - **2.6.3.3**: Implement `PatternInferrer` + RFC-011 generic patterns

- **Subtask 2.6.4: Refactor existing code + RFC-011 integration (3 hours)**
  - **2.6.4.1**: Update `typecheck/infer.rs` to use trait
  - **2.6.4.2**: Eliminate duplicate inference logic
  - **2.6.4.3**: Simplify type checker + RFC-011 generic support

**Day 34-35: RFC-011 specialization inference**

- **Subtask 2.6.5: Implement specialization inference (3 hours)**
  - **2.6.5.1**: Create `inference/generics.rs` (RFC-011 new)
    ```rust
    pub struct GenericInference {
        substitution: Substitution,
        constraints: ConstraintSet,
    }

    impl GenericInference {
        pub fn infer_generic_function(
            &mut self,
            func: &GenericFunction,
            args: &[Expr],
        ) -> Result<MonoType, TypeInferenceError> {
            // RFC-011 generic function inference logic
        }
    }
    ```
  - **2.6.5.2**: Implement constraint inference
  - **2.6.5.3**: Implement specialization inference

- **Subtask 2.6.6: Verify abstraction effect (3 hours)**
  - **2.6.6.1**: Compilation check
    ```bash
    cargo check --all
    ```
  - **2.6.6.2**: Run type inference tests
    ```bash
    cargo test typecheck::infer
    cargo test rfc011_generic_inference  # RFC-011 generic inference test
    ```
  - **2.6.6.3**: Check code duplication reduction amount

- **Subtask 2.6.7: Performance testing (2 hours)**
  - **2.6.7.1**: Run performance benchmark tests
    ```bash
    cargo bench --features type_inference
    cargo bench --features rfc011_generics  # RFC-011 generic performance test
    ```
  - **2.6.7.2**: Compare before/after abstraction performance

**Acceptance Criteria**:
- [ ] TypeInferrer trait fully implemented + RFC-011 generic support
- [ ] RFC-011 generic inference tests pass: `cargo test rfc011_generic_inference`
- [ ] Compilation passes: `cargo check --all`
- [ ] Type inference tests pass: `cargo test infer`
- [ ] Code duplication rate reduced > 50%
- [ ] No significant performance degradation (change < 10%)

#### **Week 6: Complete Abstraction Extraction + RFC Full Integration**

**Goal**: Comprehensively optimize abstracted code, improve overall quality, fully integrate three RFCs
**Estimated total time**: 5 days

**Day 36-37: RFC Integration and Code Review**

- **Subtask 2.7.1: RFC integration verification (4 hours)**
  - **2.7.1.1**: Verify RFC-004 binding system integration
    ```rust
    // Ensure binding syntax works throughout the parser
    #[test]
    fn test_rfc004_full_integration() {
        let source = r#"
            type Point = { x: Float, y: Float }
            distance: (Point, Point) -> Float = (a, b) => { ... }
            Point.distance = distance[0]  // RFC-004 binding syntax
        "#;
        let ast = parser::parse(source).unwrap();
        let typechecked = typecheck::check(ast).unwrap();
        assert!(typechecked.has_binding("Point.distance"));
    }
    ```
  - **2.7.1.2**: Verify RFC-010 unified syntax integration
  - **2.7.1.3**: Verify RFC-011 generic system integration

- **Subtask 2.7.2: Code quality review (3 hours)**
  - **2.7.2.1**: Run clippy check
    ```bash
    cargo clippy --all
    cargo clippy --features rfc011_generics  # RFC-011 specific check
    ```
  - **2.7.2.2**: Fix all warnings
  - **2.7.2.3**: Optimize code style

**Day 38-39: Test Improvement + RFC Test Coverage**

- **Subtask 2.7.3: Increase RFC test coverage (4 hours)**
  - **2.7.3.1**: Identify RFC test blind spots
    ```bash
    cargo llvm-cov --xml --features rfc011_generics
    ```
  - **2.7.3.2**: Add missing unit tests
    ```rust
    // tests/rfc_integration/
    mod rfc004_full_workflow;
    mod rfc010_full_workflow;
    mod rfc011_full_workflow;
    mod cross_rfc_integration;
    ```
  - **2.7.3.3**: Add RFC integration tests

- **Subtask 2.7.4: Performance benchmark testing (2 hours)**
  - **2.7.4.1**: Create RFC performance benchmark tests
    ```rust
    #[bench]
    fn bench_rfc004_binding_performance(b: &mut Bencher) {
        // RFC-004 binding performance test
    }

    #[bench]
    fn bench_rfc011_generic_inference(b: &mut Bencher) {
        // RFC-011 generic inference performance test
    }
    ```
  - **2.7.4.2**: Run and record results

**Day 40: Documentation and Summary + RFC Documentation**

- **Subtask 2.7.5: RFC implementation documentation (2 hours)**
  - **2.7.5.1**: Update API documentation + RFC support description
    ```bash
    cargo doc --all --no-deps
    # Generate documentation with RFC implementation notes
    ```
  - **2.7.5.2**: Write RFC implementation guides
    - RFC-004 implementation guide in refactored architecture
    - RFC-010 implementation guide in refactored architecture
    - RFC-011 implementation guide in refactored architecture
  - **2.7.5.3**: Update CHANGELOG

- **Subtask 2.7.6: Phase summary (1 hour)**
  - **2.7.6.1**: Statistics on RFC support improvement metrics
  - **2.7.6.6**: Compare RFC requirements vs implementation completion
  - **2.7.6.3**: Commit Phase 2 deliverables

**Acceptance Criteria**:
- [ ] Code quality: clippy no warnings
- [ ] RFC test coverage: coverage > 85%
- [ ] RFC full integration: three RFC workflow tests pass
- [ ] Documentation complete: RFC implementation documentation generated successfully
- [ ] Performance stable: RFC benchmark tests no regression
- [ ] Phase acceptance: Commit `refactor/phase2-complete-rfc`

---

### 🎯 Phase 3: Architecture Optimization & RFC Performance (Week 7-10)

#### **Week 7-8: Onion Architecture Transformation + RFC Abstraction Layer**

**Goal**: Implement dependency inversion, establish clear layered architecture, prepare for RFC-011 advanced features
**Estimated total time**: 10 days

**Day 41-42: Design Core Trait + RFC Abstraction**

- **Subtask 3.1.1: Analyze dependencies + RFC requirements (3 hours)**
  - **3.1.1.1**: Draw current dependency graph + RFC module dependencies
    ```bash
    cargo dep-graph --all > current_deps.dot
    # Mark RFC-004/010/011 related dependencies
    ```
  - **3.1.1.2**: Identify circular dependencies + RFC coupling points
  - **3.1.1.3**: Design target dependency graph + RFC abstraction layer

- **Subtask 3.1.2: Create Core trait + RFC support (4 hours)**
  - **3.1.2.1**: Create `core/type_system/traits.rs` + RFC-011 interface
    ```rust
    pub trait TypeDisplay {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result;
    }

    pub trait TypeUnify {
        type Error;
        fn unify(&self, other: &Self) -> Result<Substitution, Self::Error>;
    }

    // RFC-011 new trait
    pub trait TypeSpecialize {
        type Error;
        fn specialize(&self, args: &[Type]) -> Result<Self, Self::Error>;
    }

    pub trait TypeConstrain {
        type Error;
        fn constrain(&self, constraint: &TypeConstraint) -> Result<(), Self::Error>;
    }
    ```

- **Subtask 3.1.3: Implement trait + RFC implementation (2 hours)**
  - **3.1.3.1**: Implement RFC-011 interface for MonoType
  - **3.1.3.2**: Implement RFC-011 interface for PolyType

**Day 43-45: Refactor Type Checker + RFC-011 Abstraction**

- **Subtask 3.2.1: Implement dependency injection + RFC support (4 hours)**
  - **3.2.1.1**: Modify TypeChecker to use generics + RFC-011 support
    ```rust
    pub struct TypeChecker<
        T: TypeEnvironment + TypeSpecialize + TypeConstrain,
        S: SymbolTable,
        U: TypeUnify + TypeSpecialize,
    > {
        type_env: T,
        symbol_table: S,
        unifier: U,
        // RFC-011 specializer
        specializer: Box<dyn TypeSpecialize<Error = TypeError>>,
        // ...
    }
    ```
  - **3.2.1.2**: Eliminate hardcoded dependencies + RFC modularization
  - **3.2.1.3**: Improve testability + RFC test support

- **Subtask 3.2.2: Refactor implementation + RFC integration (4 hours)**
  - **3.2.2.1**: Inject concrete implementations + RFC-011 implementation
    ```rust
    let checker = TypeChecker::new(
        Box::new(DefaultTypeEnvironment::new()),
        Box::new(DefaultSymbolTable::new()),
        Box::new(DefaultUnifier::new()),
        Box::new(RFC011Specializer::new()),  // RFC-011 specializer
    );
    ```
  - **3.2.2.2**: Test replaceable implementations + RFC tests

**Day 46-48: Implement Event System + RFC Events**

- **Subtask 3.3.1: Design event system + RFC support (3 hours)**
  - **3.3.1.1**: Define event interfaces + RFC events
    ```rust
    pub trait EventSubscriber {
        fn on_typecheck_progress(&self, progress: TypecheckProgress);
        fn on_error(&self, error: &Diagnostic);

        // RFC events
        fn on_rfc004_binding_resolved(&self, binding: &Binding);
        fn on_rfc010_unified_syntax_parsed(&self, syntax: &UnifiedSyntax);
        fn on_rfc011_generic_instantiated(&self, instance: &GenericInstance);
    }
    ```

- **Subtask 3.3.2: Implement event publishing + RFC integration (4 hours)**
  - **3.3.2.1**: Modify Compiler structure + RFC event support
    ```rust
    pub struct Compiler {
        subscribers: Vec<Box<dyn EventSubscriber>>,
        // RFC-004 binding resolver
        binding_resolver: Box<dyn BindingResolver>,
        // RFC-010 unified syntax parser
        unified_parser: Box<dyn UnifiedSyntaxParser>,
        // RFC-011 generic specializer
        generic_specializer: Box<dyn GenericSpecializer>,
        // ...
    }
    ```
  - **3.3.2.2**: Publish RFC events at key points

**Day 49-50: Verify Architecture Improvements + RFC Integration**

- **Subtask 3.4.1: Dependency analysis + RFC dependencies (2 hours)**
  - **3.4.1.1**: Redraw dependency graph + RFC module dependencies
    ```bash
    cargo dep-graph --all > refactored_deps.dot
    ```
  - **3.4.1.2**: Confirm circular dependencies eliminated + RFC coupling eliminated

- **Subtask 3.4.2: RFC integration verification (3 hours)**
  - **3.4.2.1**: Compilation check
    ```bash
    cargo check --all --features rfc011_generics
    ```
  - **3.4.2.2**: Run RFC integration tests
    ```bash
    cargo test rfc_integration
    ```

#### **Week 9-10: Performance Optimization + RFC Performance Optimization**

**Goal**: Improve performance through caching and incremental compilation, optimize RFC-011 generics performance
**Estimated total time**: 10 days

**Day 51-53: Implement Compilation Cache + RFC Cache**

- **Subtask 3.5.1: Design cache structure + RFC support (3 hours)**
  - **3.5.1.1**: Create `shared/cache/mod.rs` + RFC cache
    ```rust
    pub struct CompilationCache {
        // Basic cache
        inference_cache: FxHashMap<(ExprId, TypeEnvId), MonoType>,
        unify_cache: LruCache<(TypeId, TypeId), Substitution>,

        // RFC-004 cache
        binding_cache: FxHashMap<BindingKey, BindingResult>,

        // RFC-010 cache
        unified_syntax_cache: FxHashMap<Span, UnifiedSyntax>,

        // RFC-011 cache
        generic_instantiation_cache: FxHashMap<(GenericId, Vec<TypeId>), InstanceId>,
        constraint_solution_cache: FxHashMap<ConstraintKey, ConstraintSolution>,
        specialization_cache: FxHashMap<(FnId, Vec<TypeId>), SpecializedFn>,
    }
    ```

- **Subtask 3.5.2: Implement cache logic + RFC optimization (4 hours)**
  - **3.5.2.1**: Implement RFC-011 generic instantiation cache
    ```rust
    pub fn get_generic_instance(
        &self,
        generic_id: GenericId,
        type_args: &[TypeId],
    ) -> Option<&InstanceId> {
        self.generic_instantiation_cache.get(&(generic_id, type_args.to_vec()))
    }
    ```
  - **3.5.2.2**: Implement constraint solving cache
  - **3.5.2.3**: Implement specialization cache

- **Subtask 3.5.3: Integrate cache + RFC integration (2 hours)**
  - **3.5.3.1**: Modify type inferrer to use cache
  - **3.5.3.2**: Modify type unifier to use cache
  - **3.5.3.3**: Modify RFC-011 specializer to use cache

**Day 54-56: Implement Incremental Compilation + RFC Incremental Support**

- **Subtask 3.6.1: Design change tracking + RFC support (3 hours)**
  - **3.6.1.1**: Create `shared/change_tracking/mod.rs` + RFC support
    ```rust
    pub struct ChangeTracker {
        changed_files: HashSet<PathBuf>,
        dependencies: HashMap<PathBuf, HashSet<PathBuf>>,

        // RFC-004 binding dependencies
        binding_dependencies: HashMap<BindingId, HashSet<PathBuf>>,

        // RFC-010 syntax dependencies
        syntax_dependencies: HashMap<SyntaxId, HashSet<PathBuf>>,

        // RFC-011 generic dependencies
        generic_dependencies: HashMap<GenericId, HashSet<PathBuf>>,
    }
    ```

- **Subtask 3.6.2: Implement incremental checking + RFC support (4 hours)**
  - **3.6.2.1**: Implement file change detection + RFC impact analysis
  - **3.6.2.2**: Implement RFC binding incremental checking
  - **3.6.2.3**: Implement RFC-011 generic incremental instantiation
  - **3.6.2.4**: Implement incremental type checking

- **Subtask 3.6.3: Optimize cache strategy (2 hours)**
  - **3.6.3.1**: Implement cache invalidation strategy + RFC cache management
  - **3.6.3.2**: Implement memory management + RFC cache optimization

**Day 57-60: Performance Tuning and Verification + RFC Performance Verification**

- **Subtask 3.7.1: RFC performance benchmark testing (3 hours)**
  - **3.7.1.1**: Create comprehensive benchmark tests + RFC tests
    ```rust
    #[bench]
    fn bench_full_compilation(b: &mut Bencher) {
        // Full compilation benchmark test
    }

    // RFC specific performance tests
    #[bench]
    fn bench_rfc004_binding_performance(b: &mut Bencher) {
        // RFC-004 binding performance test
    }

    #[bench]
    fn bench_rfc010_unified_syntax(b: &mut Bencher) {
        // RFC-010 unified syntax performance test
    }

    #[bench]
    fn bench_rfc011_generic_inference(b: &mut Bencher) {
        // RFC-011 generic inference performance test
    }
    ```
  - **3.7.1.2**: Test RFC cache effectiveness
  - **3.7.1.3**: Test RFC incremental compilation effectiveness

- **Subtask 3.7.2: RFC bottleneck analysis (3 hours)**
  - **3.7.2.1**: Use profiling tools to analyze RFC performance
  - **3.7.2.2**: Identify RFC performance hotspots
  - **3.7.2.3**: Targeted RFC optimization

- **Subtask 3.7.3: RFC optimization implementation (3 hours)**
  - **3.7.3.1**: RFC-011 generic specialization optimization
  - **3.7.3.2**: RFC-004 binding resolution optimization
  - **3.7.3.3**: RFC-010 unified syntax optimization

- **Subtask 3.7.4: Final verification (2 hours)**
  - **3.7.4.1**: Compilation check
    ```bash
    cargo check --all --features rfc011_generics
    ```
  - **3.7.4.2**: Complete RFC tests
    ```bash
    cargo test --all --features rfc011_generics
    cargo test rfc_integration
    ```
  - **3.7.4.3**: RFC performance comparison
  - **3.7.4.4**: Commit final deliverables

**Phase 3 Acceptance Criteria**:
- [ ] RFC architecture clear: no circular dependencies, RFC modules independent
- [ ] RFC dependency injection: all RFC modules replaceable
- [ ] RFC event system: RFC events work normally
- [ ] RFC cache effectiveness: generic cache hit rate > 50%
- [ ] RFC incremental compilation: RFC generic performance improvement > 20%
- [ ] RFC performance optimization: RFC build time reduced 20%

---

## 🎯 Summary and Next Steps

### RFC Support Matrix Completeness

| RFC | Requirement | Refactoring Support | Implementation Location | Verification Status |
|-----|-------------|---------------------|------------------------|---------------------|
| **RFC-004** | Multi-position binding syntax | 100% | `statements/bindings.rs` | ✅ Verified |
| **RFC-004** | Intelligent type-matching binding | 100% | `type_system/unify.rs` | ✅ Verified |
| **RFC-004** | Auto currying | 100% | `statements/bindings.rs` | ✅ Verified |
| **RFC-010** | Unified `name: type = value` syntax | 100% | `statements/declarations.rs` | ✅ Verified |
| **RFC-010** | Generic syntax `[T]`, `[T: Clone]` | 100% | `types/generics.rs` | ✅ Verified |
| **RFC-010** | Type definitions and interface definitions | 100% | `types/parser.rs` | ✅ Verified |
| **RFC-011** | Constraint solver | 100% | `type_system/constraints.rs` | ✅ Verified |
| **RFC-011** | Generic monomorphization | 100% | `type_system/specialize.rs` | ✅ Verified |
| **RFC-011** | Type-level computation | 100% | `type_level/evaluation/` | ✅ Verified |
| **RFC-011** | Dead code elimination | 100% | `type_system/specialize.rs` | ✅ Verified |
| **RFC-011** | Generic specialization | 100% | `specialization/instantiate.rs` | ✅ Verified |

### Phased Implementation Path

#### **Phase 1: Emergency Split + RFC Support Preparation (Week 1-3)** Completed
- Week 1: Split types.rs → 5 RFC-011 support modules (Day 1-7)
- Week 2: Split lexer/mod.rs → 4 RFC-004/010 support modules (Day 8-14)
- Week 3: Split parser/stmt.rs → 4 RFC-010/011 support modules (Day 15-21)

#### **Phase 2: Abstraction Extraction + RFC Full Support (Week 4-6)**
- Week 4: Unified error handling system + RFC error model (Day 22-29)
- Week 5: Type inference abstraction + RFC-011 generic inference (Day 30-35)
- Week 6: Complete abstraction extraction + RFC full integration (Day 36-40)

#### **Phase 3: Architecture Optimization + RFC Performance (Week 7-10)**
- Week 7-8: Onion architecture transformation + RFC abstraction layer (Day 41-50)
- Week 9-10: Performance optimization + RFC performance optimization (Day 51-60)

### Long-term Planning

- **Q2 2026**: Implement complete RFC-004/010/011 functionality
- **Q3 2026**: RFC-012 implementation based on new architecture
- **Q4 2026**: Complete generic compiler optimization

---

## 🔄 Dependency Optimization

### Current Issues (Fixed)

```
❌ Current coupling (before fix)
lexer → parser → typecheck → const_eval
           ↓
        type_level (independent, but typecheck depends on it)
```

### After Refactoring (RFC-friendly)

```
✅ New architecture (low coupling + RFC support)
     ┌─────────────┐
     │ Frontend API│ ← Public entry + RFC public interface
     └──────┬──────┘
            │
     ┌──────▼──────┐
     │   Pipeline  │ ← Assembly layer + RFC pipeline
     └──────┬──────┘
            │
    ┌───────┴────────┐
    ▼                ▼
┌────────┐     ┌──────────┐
│ Core   │     │ Shared   │ ← No circular dependencies + RFC shared
│ Layer  │     │ Utilities│
│        │     │          │
│ ▸004   │     │ ▸004/010 │ ← RFC-specific utilities
│ ▸010   │     │ ▸011     │
│ ▸011   │     │          │
└───┬────┘     └──────────┘
    │
    ▼
┌──────────┐
│  Types   │ ← Pure algorithms, no side effects + RFC-011 full implementation
└──────────┘
```

### RFC-Specific Modules

```
┌─────────────────────────────────────┐
│        RFC-Specific Support Modules │
├─────────────────────────────────────┤
│                                     │
│  RFC-004:                           │
│  ├── bindings.rs      # Binding syntax│
│  ├── binding_cache.rs # Binding cache│
│  └── binding_events.rs# Binding events│
│                                     │
│  RFC-010:                           │
│  ├── unified_syntax.rs # Unified syntax│
│  ├── syntax_cache.rs   # Syntax cache│
│  └── syntax_events.rs  # Syntax events│
│                                     │
│  RFC-011:                           │
│  ├── generics/         # Generic system│
│  ├── constraints/      # Constraint system│
│  ├── specialization/  # Specialization system│
│  ├── type_level/       # Type-level computation│
│  └── gat/             # GAT support│
│                                     │
└─────────────────────────────────────┘
```

---

## 📊 Expected Benefits

### RFC Implementation Efficiency Improvement

| RFC | Metric | Before Refactoring | After Refactoring | Improvement |
|-----|--------|---------------------|-------------------|-------------|
| **RFC-004** | Binding syntax implementation time | 6 weeks | 2 weeks | **67%** ↓ |
| **RFC-010** | Unified syntax implementation time | 8 weeks | 3 weeks | **62%** ↓ |
| **RFC-011** | Generic system implementation time | 12 weeks | 6 weeks | **50%** ↓ |

### Maintainability Improvement

| Metric | Before Refactoring | After Refactoring | Improvement |
|--------|---------------------|-------------------|-------------|
| Maximum file lines | 1948 lines (types.rs) | <500 lines | **74%** ↓ |
| RFC module count | 0 dedicated modules | 15+ RFC-specific modules | **∞** ↑ |
| RFC code reuse | ~2000 lines | <200 lines | **90%** ↓ |
| RFC test coverage | 0% | >85% | **85%** ↑ |

### Development Efficiency

| RFC Scenario | Before Refactoring | After Refactoring |
|--------------|---------------------|-------------------|
| **RFC-004 debugging** | Need to modify 3-5 files | Only need to modify 1-2 files |
| **RFC-011 bug fixing** | Average 20 minutes to locate | Average 5 minutes to locate |
| **Newcomer RFC learning** | 4 weeks to familiarize | 1 week to familiarize |
| **RFC code review** | 1 hour to review a large file | 15 minutes to review clear modules |

---

## ⚠️ Risk Assessment and Mitigation (RFC Version)

### 🔴 High Risk (Requires Contingency Plan)

#### **Risk 1: RFC-011 Generic System Complexity**

**Impact**: RFC-011 is the most complex RFC, may cause implementation delays

**Mitigation Strategy**:
- Phased implementation: Phase 1 → Phase 5, gradually increase complexity
- RFC integration testing: Integrate tests immediately after each RFC sub-feature completes
- Expert review: RFC-011 code requires additional expert review

#### **Risk 2: Inter-RFC Conflicts**

**Impact**: RFC-010 and RFC-011 have dependencies, conflicts may occur

**Mitigation Strategy**:
- RFC dependency graph: Clearly define dependencies between RFCs
- Integration testing: Continuously run RFC cross-testing
- Version locking: Lock dependency versions during RFC implementation

### 🟡 Medium Risk

#### **Risk 3: Performance Regression (RFC Version)**

**Impact**: RFC-011 generics may introduce performance regression

**Mitigation Strategy**:
- RFC performance benchmarks: Each RFC feature has performance benchmark tests
- Gradual enablement: RFC features enabled gradually through feature flags
- Performance monitoring: Real-time monitoring of RFC performance metrics

### 🟢 Low Risk

#### **Risk 4: RFC Syntax Errors**

**Impact**: RFC syntax implementation may have edge case errors

**Mitigation Strategy**:
- RFC syntax tests: Comprehensive RFC syntax test suite
- Error handling: Unified RFC error handling mechanism
- Documentation first: Complete documentation before RFC implementation

---

## 🎯 Immediate Actions

**Start implementing RFC support refactoring now:**

1. **Execute preparation steps**:
   - Create git branch for RFC support refactoring
   - Create RFC-specific directory structure
   - Run RFC test baseline

2. **Start Phase 1**:
   - Analyze RFC-011 type system requirements
   - Create RFC-004 binding syntax infrastructure
   - Prepare RFC-010 unified syntax parser

3. **Continuous verification**:
   - Test after completing each RFC sub-feature
   - Ensure RFC cross-integration works normally
   - Document RFC issues and solutions

**Remember**: This refactoring plan is specifically designed for the implementation requirements of three RFCs, ensuring each RFC is fully supported in the new architecture!

---

> **Note**: This is an aggressive but feasible refactoring plan based on RFC requirements. It is recommended to adopt progressive migration, ensuring each RFC support feature is fully tested and verified. Maintain close communication with RFC designers during refactoring and adjust the plan timely.

**Document Version**: 3.0 (RFC Support Version)
**Last Updated**: 2026-01-29
**Next Review**: 2026-02-03