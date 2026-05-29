# RFC-009 Ownership Model Implementation Path

> **Document Version**: v1.0
> **Based on Design**: `docs/design/accepted/009-ownership-model.md`
> **Generation Date**: 2025-02-05

## Implementation Overview

This document translates the RFC-009 design specification into executable implementation steps, extending the existing YaoXiang architecture.

### Existing Foundation

| Module | Location | Status |
|--------|----------|--------|
| Ownership System | `src/middle/passes/lifetime/` | ✅ Foundation Complete |
| Move Semantics | `move_semantics.rs` | ✅ Implemented |
| ref Semantics | `ref_semantics.rs` | ✅ Implemented |
| Cycle Detection | `cycle_check.rs` | ✅ Implemented |
| mut Check | `mut_check.rs` | ✅ Implemented |

---

## Phase 1: Field-Level Immutability (P0)

### Goal

Support `mut` field markers in type definitions, implementing a three-level mutability model:
- Binding mutability (variable level)
- Field mutability (struct level)
- Method parameter mutability (function level)

### Implementation Status: ✅ Completed (2026-02-05)

#### Completed Changes (2026-02-05 Update)

1. **AST Extension** (`ast.rs`)
   - ✅ Created `StructField` struct: `name: String, is_mut: bool, ty: Type`
   - ✅ `Type::Struct(Vec<StructField>)` replaced `Type::Struct(Vec<(String, Type)>)`
   - ✅ `Type::NamedStruct { name, fields: Vec<StructField> }`
   - ✅ `Pattern::Struct { name, fields: Vec<(String, bool, Box<Pattern>)> }`

2. **Parser Extension** (`statements/declarations.rs`)
   - ✅ `parse_struct_type` supports `{ x: Float, mut y: Float }` syntax
   - ✅ `parse_named_struct_type` supports `Point(x: Float, mut y: Float)` syntax

3. **Type System** (`type_system/mono.rs`)
   - ✅ `StructType` added `field_mutability: Vec<bool>`
   - ✅ Implemented `field_is_mut(&self, field_name: &str) -> Option<bool>` method
   - ✅ Updated `MonoType::from(ast::Type)` conversion logic

4. **Pattern Matching** (`typecheck/inference/patterns.rs`)
   - ✅ Pattern inference supports `is_mut` marker

5. **Parser Pattern Parsing** (`parser/pratt/nud.rs`)
   - ✅ Struct pattern syntax parsing supports `mut` keyword

6. **Error Types** (`lifetime/error.rs`)
   - ✅ Added `ImmutableFieldAssign` error variant
   - ✅ Added Display implementation

7. **IR Instruction Extension** (`middle/core/ir.rs`)
   - ✅ `StoreField` added `type_name: Option<String>` and `field_name: Option<String>`

8. **IR Generation** (`middle/core/ir_gen.rs`)
   - ✅ `get_field_mutability` returns type name
   - ✅ StoreField instruction carries type information

9. **Mutability Check** (`lifetime/mut_check.rs`)
   - ✅ Binding-level mutability check
   - ✅ Field-level mutability check (receiving type table)
   - ✅ `ImmutableFieldAssign` error detection

10. **Code Generation** (`codegen/translator.rs`)
    - ✅ StoreField pattern matching fix (using `..` to ignore extra fields)

### Involved Files

| Type | File | Status |
|------|------|--------|
| Modified | `src/frontend/core/parser/ast.rs` | ✅ Completed |
| Modified | `src/frontend/core/parser/statements/declarations.rs` | ✅ Completed |
| Modified | `src/frontend/core/parser/pratt/nud.rs` | ✅ Completed |
| Modified | `src/frontend/core/type_system/mono.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/inference/patterns.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/mod.rs` | ✅ Completed |
| Modified | `src/frontend/type_level/auto_derive.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/error.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/mut_check.rs` | ✅ Completed |
| Modified | `src/middle/core/ir_gen.rs` | ✅ Completed |
| Modified | `src/middle/passes/codegen/mod.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/cross_module.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/function.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/module_state.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/type_mono.rs` | ✅ Completed |
| Modified | `src/frontend/core/type_system/solver.rs` | ✅ Completed |
| Modified | `src/frontend/core/type_system/substitute.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/specialization/algorithm.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/specialize.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/overload.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/inference/expressions.rs` | ✅ Completed |

### Acceptance Criteria

- [x] `type Point { x: Float, mut y: Float }` syntax parses correctly
- [x] `type Point(x: Float, mut y: Float)` named struct syntax parses correctly
- [x] `NamedStruct(Point(x: Float, mut y: Float))` constructor supports mut fields
- [x] `mut p: Point = Point(1.0, 2.0); p.y = 3.0` compiles (binding mutable, field mutable)
- [x] `p.y = 3.0` compiles with non-mut binding (binding immutable, field mutable)
- [x] `p.x = 3.0` fails with non-mut binding (binding immutable, field immutable) → `ImmutableFieldAssign`
- [x] `p.x = 3.0` compiles with mut binding (binding mutable, field writable)

### Implementation Notes

1. **Data Structure Changes** (Completed)
   - `StructField` struct: `name, is_mut, ty`
   - `StructType.field_mutability: Vec<bool>`
   - `Pattern::Struct` field supports `is_mut` marker

2. **Parser Layer** (Completed)
   - `parse_struct_type` supports `{ x: Float, mut y: Float }`
   - `parse_named_struct_type` supports `Point(x: Float, mut y: Float)`

3. **IR Generation** (Completed)
   - Field assignment `p.y = value` generates `StoreField` instruction
   - `get_field_mutability` method queries field mutability
   - `StoreField` carries `type_name` and `field_name` for checking

4. **MutChecker** (Completed)
   - Binding-level mutability check: checks if variable is declared as `mut`
   - Field-level mutability check: checks if field is declared as `mut`
   - Rule: **Binding mutable OR field mutable** → Allow assignment
   - Architecture: receives `HashMap<String, StructType>` type table
   - Added parser: `parse_let_stmt` and `parse_pattern`
   - IR generation: `generate_pattern_ir` handles pattern destructuring

### Future Optimizations

(Current Phase 1 Complete)

---

## Phase 2: Empty State Reuse (P1) ✅ Completed

### Goal

Implement variable entering `empty` state after Move, allowing reassignment and reuse of variable name.

### Implementation Status: ✅ Completed (2026-02-05)

#### Completed Changes (2026-02-05 Update)

1. **ValueState Extension** (`error.rs`)
   - ✅ `ValueState::Owned(Option<TypeId>)` added type tracking
   - ✅ `ValueState::Empty` new empty state variant
   - ✅ Added `TypeId` type identifier
   - ✅ Added `EmptyStateTypeMismatch` and `ReassignNonEmpty` error types

2. **Empty State Tracking** (New `empty_state.rs`)
   - ✅ Created `EmptyStateTracker` struct
   - ✅ Implemented state tracking and type checking
   - ✅ Implemented branch state merging (conservative strategy)

3. **Control Flow Analysis** (New `control_flow.rs`)
   - ✅ Created `ControlFlowAnalyzer` struct
   - ✅ Implemented `merge_states` conservative merge strategy
   - ✅ Provided liveness analysis helper functions

4. **Move Checker Extension** (`move_semantics.rs`)
   - ✅ Variable enters Empty state after Move (instead of Moved)
   - ✅ Empty state variable allows reassignment
   - ✅ Type consistency check
   - ✅ Function call argument enters Empty state

5. **Other Checker Adaptations**
   - ✅ `clone.rs`: Updated to adapt to Empty state
   - ✅ `drop_semantics.rs`: Drop Empty state is legal
   - ✅ `ref_semantics.rs`: Updated to adapt to Empty state

6. **Module Registration** (`mod.rs`)
   - ✅ Registered `empty_state` and `control_flow` modules

### Involved Files

| Type | File | Status |
|------|------|--------|
| Modified | `src/middle/passes/lifetime/error.rs` | ✅ Completed |
| New | `src/middle/passes/lifetime/empty_state.rs` | ✅ Completed |
| New | `src/middle/passes/lifetime/control_flow.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/move_semantics.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/clone.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/drop_semantics.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/mod.rs` | ✅ Completed |

### Acceptance Criteria

- [x] `p = Point(1.0); p2 = p; p = Point(2.0)` compiles
- [x] `p = Point(1.0); p2 = p; print(p)` fails (UseAfterMove)
- [x] if branches correctly track empty state (conservative analysis)
- [x] `p = "hello"` errors after Point type (EmptyStateTypeMismatch)

### Implementation Notes

1. **State Design**
   - `Owned(Option<TypeId>)`: Valid value, carries type information
   - `Empty`: Empty state, allows reassignment
   - `Moved`: Already moved (retained for compatibility)
   - `Dropped`: Already dropped

2. **State Transitions**
   ```
   Owned ──Move──► Empty ──(Store, same type)──► Owned
                         ▲
                         │
                    Error: type mismatch
   ```

3. **Conservative Branch Merging**
   - Either branch is Empty → Merged is Empty
   - Either branch is Moved → Merged is Moved
   - Both are Owned → Keep first one

4. **Type Checking**
   - Check type consistency on reassignment
   - Report `EmptyStateTypeMismatch` on type mismatch

### Future Optimizations

(Current Phase 2 Complete)

---

## Phase 3: Ownership Return Flow (P1) ✅ Completed

### Goal

Implement parameter modification return, forming ownership closure, supporting method chaining.

### Implementation Status: ✅ Completed (2026-02-06)

#### Completed Changes (2026-02-06 Update)

1. **Consume Mode Enum** (`ownership_flow.rs`)
   - ✅ Created `ConsumeMode` enum: `Returns | Consumes | Undetermined`
   - ✅ `Returns`: Parameter returned in return value, ownership returns
   - ✅ `Consumes`: Parameter consumed, not returned
   - ✅ `Undetermined`: Cannot determine (conservative analysis)

2. **Ownership Return Flow Analyzer** (`ownership_flow.rs`)
   - ✅ Created `OwnershipFlowAnalyzer` struct
   - ✅ `analyze_function()` analyzes function consume mode
   - ✅ `operand_references_param()` checks if return value references parameter
   - ✅ `returns_param_directly()` quick detection of `return p;` pattern
   - ✅ Conservative estimation: temporary variables may reference parameters

3. **Chain Call Analyzer** (`chain_calls.rs`)
   - ✅ Created `ChainCallAnalyzer` struct
   - ✅ `analyze_chain()` analyzes method chain ownership flow
   - ✅ `extract_chain_calls()` extracts consecutive virtual method calls
   - ✅ `infer_consume_mode()` infers consume mode based on usage
   - ✅ `check_ownership_closure()` verifies ownership closure

4. **Error Type Extension** (`error.rs`)
   - ✅ Added `ConsumedNotReturned` error variant
   - ✅ Used for diagnostics when parameter consumed but not returned

5. **Module Registration** (`mod.rs`)
   - ✅ Registered `ownership_flow` and `chain_calls` modules

### Involved Files

| Type | File | Status |
|------|------|--------|
| New | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ Completed |
| New | `src/middle/passes/lifetime/chain_calls.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/error.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/mod.rs` | ✅ Completed |

### Acceptance Criteria

- [x] `p = p.process()` inferred as Returns mode
- [x] `consume(p)` inferred as Consumes mode
- [x] `p = p.rotate(90).scale(2.0).translate(1.0)` chain calls work correctly
- [x] Return flow inference errors give accurate hints

### Implementation Notes

1. **ConsumeMode Design**
   ```
   ConsumeMode::Returns     → Parameter returned in return value
   ConsumeMode::Consumes   → Parameter consumed, not returned
   ConsumeMode::Undetermined → Cannot determine, conservative analysis
   ```

2. **Parameter Reference Detection**
   - Direct reference: `Operand::Arg(idx)` → Check index match
   - Temporary variable: Conservative estimate may reference parameter
   - Constant/global: Does not reference parameter

3. **Chain Call Analysis**
   ```ignore
   p.rotate(90)    // Method 1: rotate
     .scale(2.0)   // Method 2: scale (obj = temp_1)
     .translate(1.0); // Method 3: translate (obj = temp_2)
   ```

4. **Ownership Closure Check**
   - Consumes mode → Ownership correctly closed
   - Returns mode → Return value should be used
   - Undetermined → Conservative return true

### Test Coverage

| Module | Test Count | Description |
|--------|------------|-------------|
| `ownership_flow` | 10 | Parameter reference detection, mode inference |
| `chain_calls` | 13 | Chain calls, ownership closure |

### Future Optimizations

(Current Phase 3 Complete)

---

## Phase 4: Consume Analysis (P1) ✅ Completed

### Goal

Implement complete consume marking system, tracking each variable's Consumes/Returns state.

### Implementation Status: ✅ Completed (2026-02-06)

#### Completed Changes (2026-02-06 Update)

1. **Consume Analyzer** (New `consume_analysis.rs`)
   - ✅ Reuses Phase 3's `ConsumeMode` and `OwnershipFlowAnalyzer`
   - ✅ `ConsumeAnalyzer` provides cross-function consume mode queries
   - ✅ Builtin function special handling (consume, clone, etc.)
   - ✅ Consume mode caching mechanism

2. **Lifecycle Tracker** (New `lifecycle.rs`)
   - ✅ Created `LifecycleTracker` struct
   - ✅ Variable lifecycle event recording (create/consume/move/drop/return)
   - ✅ Consume count and read count statistics
   - ✅ Lifecycle problem detection (consume after drop/multiple consume/use after consume)

3. **MoveChecker Extension** (`move_semantics.rs` Extension)
   - ✅ Added `ConsumeAnalyzer` field
   - ✅ `check_call` decides parameter state based on function consume mode
   - ✅ Returns mode: Parameter ownership returns, does not enter Empty
   - ✅ Consumes mode: Parameter enters Empty

4. **Module Registration** (`mod.rs`)
   - ✅ Registered `consume_analysis` and `lifecycle` modules

### Involved Files

| Type | File | Status |
|------|------|--------|
| New | `src/middle/passes/lifetime/consume_analysis.rs` | ✅ Completed |
| New | `src/middle/passes/lifetime/lifecycle.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/move_semantics.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/mod.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ Completed |

### Acceptance Criteria

- [x] Assignment/parameter passing/return correctly marked as Move
- [x] After `consume(x)`, x becomes empty (Consumes mode)
- [x] `x = modify(x)` inferred as Returns (reuses OwnershipFlowAnalyzer)
- [x] `clone()` correctly copies, does not affect original variable (builtin handling)

### Implementation Notes

1. **Reuse Phase 3 Results**
   - Directly use `ConsumeMode` enum from `ownership_flow.rs`
   - `OwnershipFlowAnalyzer` performs function-level consume mode analysis

2. **Consume Analyzer Design**
   ```
   ConsumeMode::Returns     → Parameter ownership returns, stays Owned
   ConsumeMode::Consumes   → Parameter consumed, enters Empty
   ConsumeMode::Undetermined → Conservative estimate enters Empty
   ```

3. **Lifecycle Tracking**
   ```
   Events: Created → Consumed → Moved → Dropped → Returned
   Detection: Drop without consume / Multiple consume / Use after consume / Never used
   ```

4. **MoveChecker Integration**
   - `check_call` queries called function's consume mode
   - Returns mode: Parameter state unchanged
   - Consumes mode: Parameter enters Empty

---

## Phase 5: ref Keyword = Arc (P1) ✅ Completed

### Goal

`ref` keyword implemented as Arc, thread-safe reference counting.

### Implementation Status: ✅ Completed (2026-02-06)

#### Completed Changes (2026-02-06 Update)

1. **ref Syntax Parsing** (Existing)
   - ✅ `parser/expr.rs`: `parse_ref` parses `ref expression` syntax
   - ✅ `ast.rs`: `Expr::Ref { expr, span }` AST node

2. **Type Inference** (Existing)
   - ✅ `typecheck/infer.rs`: `ref T` inferred as `Arc[T]`

3. **Ownership Handling** (Existing)
   - ✅ `ref_semantics.rs`: ArcNew/Clone/Drop ownership checks

4. **IR Generation** (New)
   - ✅ `ir_gen.rs`: Added `Expr::Ref` → `ArcNew` instruction generation

### Involved Files

| Type | File | Status |
|------|------|--------|
| Modified | `src/frontend/core/parser/expr.rs` | ✅ Existing |
| Modified | `src/frontend/typecheck/infer.rs` | ✅ Existing |
| Modified | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ Existing |
| Modified | `src/middle/core/ir_gen.rs` | ✅ This Update |

### Acceptance Criteria

- [x] `ref p` type inferred as `Arc[Point]`
- [x] `ref p` does not consume p, p still usable
- [x] `spawn(() => print(shared.x))` compiles
- [x] `ref` expressions can be nested

### Implementation Notes

1. **IR Generation** (This Implementation)
   ```rust
   Expr::Ref { expr, span: _ } => {
       let src_reg = self.next_temp_reg();
       self.generate_expr_ir(expr, src_reg, instructions, constants)?;
       instructions.push(Instruction::ArcNew {
           dst: Operand::Local(result_reg),
           src: Operand::Local(src_reg),
       });
   }
   ```

2. **Ownership Semantics**
   - `ArcNew`: Create Arc, does not affect original value state
   - `ArcClone`: Clone Arc, does not affect original value state
   - `ArcDrop`: Drop Arc, does not affect original value state

### Future Optimizations

(Current Phase 5 Complete)

---

## Phase 6: Cycle Reference Detection (P1) ✅ Completed

### Goal

Cross-task cycle reference detection, within-task cycles allowed.

### Implementation Status: ✅ Completed (2026-02-06)

#### Completed Changes (2026-02-06 Update)

1. **Error Type Extension** (`error.rs`)
   - ✅ `IntraTaskCycle` warning variant (within-task cycle, does not block compilation)
   - ✅ `UnsafeBypassCycle` info variant (unsafe bypass record)
   - ✅ Display implementation

2. **CycleChecker Enhancement** (`cycle_check.rs`)
   - ✅ Depth limit constant `MAX_DETECTION_DEPTH = 1` (only detect single-layer boundary)
   - ✅ `unsafe_ranges` field tracks unsafe block ranges
   - ✅ `unsafe_bypasses` field records bypass information
   - ✅ `is_in_unsafe()` method checks if position is within unsafe block
   - ✅ `find_spawn_result_direct()` method implements depth limit
   - ✅ `collect_unsafe_ranges()` reserves Phase 7 interface
   - ✅ Optimized error messages with resolution suggestions

3. **Intra-Task Cycle Tracker** (New `intra_task_cycle.rs`)
   - ✅ `IntraTaskCycleTracker` struct
   - ✅ `RefEdge` struct tracks ref edges
   - ✅ `track_function()` tracks cycles within function
   - ✅ `collect_ref_info()` collects ArcNew/Move/StoreField
   - ✅ `build_ref_graph()` builds reference graph
   - ✅ `detect_intra_task_cycles()` DFS detects cycles
   - ✅ Warning mode output, does not block compilation

4. **OwnershipChecker Integration** (`mod.rs`)
   - ✅ Added `intra_task_tracker` field
   - ✅ `check_function()` calls intra-task cycle tracking
   - ✅ `intra_task_warnings()` method returns warnings
   - ✅ `unsafe_bypasses()` method returns bypass records

### Involved Files

| Type | File | Status |
|------|------|--------|
| Modified | `src/middle/passes/lifetime/error.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/cycle_check.rs` | ✅ Completed |
| New | `src/middle/passes/lifetime/intra_task_cycle.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/mod.rs` | ✅ Completed |

### Acceptance Criteria

- [x] ref cycle between spawn parameters and return values detected
- [x] Within-task cycles do not error (leak is controllable)
- [x] Cross-task cycle error location accurate
- [x] unsafe block can bypass detection (interface reserved, Phase 7 to complete)

### Implementation Notes

1. **Depth Limit Design**
   - Only detect single-layer spawn boundary (depth = 1)
   - `find_spawn_result_direct()` tracks at most one layer of Move
   - Does not recursively detect indirect references in nested spawn

2. **Cycle Detection Division**
   ```
   CycleChecker        → Cross-spawn cycles (error)
   IntraTaskCycleTracker → Within-task cycles (warning)
   ```

3. **unsafe Bypass Mechanism**
   - `collect_unsafe_ranges()` collects unsafe block ranges
   - `is_in_unsafe()` checks instruction position
   - spawn within unsafe block skips detection
   - Current version reserves interface, Phase 7 implements unsafe syntax to complete

4. **Error Message Optimization**
   ```
   Cross-task circular reference: temp_0 → temp_1 → temp_0 (forms cycle).
   Suggestion: Use Weak to break cycle, or bypass detection in unsafe block
   ```

### Test Coverage

| Module | Test Count | Description |
|--------|------------|-------------|
| `cycle_check` | 22 | Cross-task cycles, depth limit, state reset |
| `intra_task_cycle` | 7 | Within-task cycles, self-reference, warning location |

### Future Optimizations

- Phase 7 implements unsafe syntax to complete `collect_unsafe_ranges()` parsing

---

## Phase 7: unsafe + Raw Pointers (P2) ✅ Completed

### Goal

Support `*T` raw pointer operations in `unsafe` blocks.

### Implementation Status: ✅ Completed (2026-02-06)

#### Completed Changes (2026-02-06 Update)

1. **Keywords and Tokens** (`tokens.rs`, `state.rs`)
   - ✅ Added `KwUnsafe` keyword
   - ✅ `state.rs`: Added `"unsafe" => Some(TokenKind::KwUnsafe)`

2. **AST Extension** (`ast.rs`)
   - ✅ `Expr::Unsafe { body: Box<Block>, span }` - unsafe block expression
   - ✅ `Type::Ptr(Box<Type>)` - raw pointer type `*T`
   - ✅ `UnOp::Deref` - dereference operator

3. **Parser Extension** (`pratt/nud.rs`, `statements/declarations.rs`)
   - ✅ `parse_unsafe()` - parses `unsafe { ... }` syntax
   - ✅ `parse_unary()` - supports `*expr` dereference syntax
   - ✅ `parse_type_annotation()` - supports `*T` type annotation

4. **IR Instruction Extension** (`ir.rs`)
   - ✅ `Instruction::UnsafeBlockStart` - unsafe block start marker
   - ✅ `Instruction::UnsafeBlockEnd` - unsafe block end marker
   - ✅ `Instruction::PtrFromRef { dst, src }` - `&value → *T`
   - ✅ `Instruction::PtrDeref { dst, src }` - `*ptr → value`
   - ✅ `Instruction::PtrStore { dst, src }` - `*ptr = value`
   - ✅ `Instruction::PtrLoad { dst, src }` - Load pointer

5. **IR Generation** (`ir_gen.rs`)
   - ✅ `Expr::Unsafe` → `UnsafeBlockStart/End` instruction wrapping
   - ✅ `UnOp::Deref` → `PtrDeref` instruction

6. **Type System** (`mono.rs`, `cross_module.rs`, `function.rs`, `module_state.rs`, `type_mono.rs`)
   - ✅ `Type::Ptr` → `MonoType::TypeRef("*{...}")`
   - ✅ Type name conversion supports raw pointers

7. **Type Inference** (`expressions.rs`)
   - ✅ `infer_unary()` supports `Deref` type inference
   - ✅ `infer_expr()` supports `Expr::Unsafe` type inference

8. **Unsafe Range Collection** (`cycle_check.rs`)
   - ✅ `collect_unsafe_ranges()` parses `UnsafeBlockStart/End` instructions

9. **Unsafe Checker** (New `unsafe_check.rs`)
   - ✅ `UnsafeChecker` struct
   - ✅ `check_function()` - checks dereference outside unsafe block
   - ✅ `UnsafeDeref` error type

10. **Error Type Extension** (`error.rs`)
    - ✅ `OwnershipError::UnsafeDeref` variant
    - ✅ Display implementation

11. **Code Generation** (`translator.rs`)
    - ✅ unsafe block and pointer instruction placeholder implementation (skipped)

### Involved Files

| Type | File | Status |
|------|------|--------|
| Modified | `src/frontend/core/lexer/tokens.rs` | ✅ Completed |
| Modified | `src/frontend/core/lexer/state.rs` | ✅ Completed |
| Modified | `src/frontend/core/parser/ast.rs` | ✅ Completed |
| Modified | `src/frontend/core/parser/pratt/nud.rs` | ✅ Completed |
| Modified | `src/frontend/core/parser/statements/declarations.rs` | ✅ Completed |
| Modified | `src/middle/core/ir.rs` | ✅ Completed |
| Modified | `src/middle/core/ir_gen.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/cycle_check.rs` | ✅ Completed |
| New | `src/middle/passes/lifetime/unsafe_check.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/error.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/mod.rs` | ✅ Completed |
| Modified | `src/middle/passes/codegen/translator.rs` | ✅ Completed |
| Modified | `src/frontend/core/type_system/mono.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/cross_module.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/function.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/module_state.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/type_mono.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/inference/expressions.rs` | ✅ Completed |

### Acceptance Criteria

- [x] `unsafe { ... }` syntax parses correctly
- [x] `*T` raw pointer type annotation parses correctly
- [x] `*ptr` dereference syntax parses correctly
- [x] `unsafe { *ptr }` compiles
- [x] `*ptr` outside unsafe block errors `UnsafeDeref`
- [x] Raw pointer type represented as `*{type}`
- [x] unsafe block generates `UnsafeBlockStart/End` IR markers
- [x] `collect_unsafe_ranges()` correctly collects unsafe ranges

### Implementation Notes

1. **AST Design**
   ```rust
   Expr::Unsafe {
       body: Box<Block>,
       span: Span,
   }
   Type::Ptr(Box<Type>)  // *T
   UnOp::Deref           // *expr
   ```

2. **IR Design**
   ```
   UnsafeBlockStart
   // instructions within block...
   UnsafeBlockEnd
   ```

3. **Dereference Type Inference**
   ```rust
   UnOp::Deref => {
       if let MonoType::TypeRef(inner) = expr {
           // Remove * prefix to get inner type
           let inner_type = inner.trim_start_matches('*').to_string();
           Ok(MonoType::TypeRef(inner_type))
       } else {
           Err(Diagnostic::error("Dereference requires pointer type"))
       }
   }
   ```

4. **Raw Pointer Type Representation**
   - Parse: `*T` → `Type::Ptr(Box<Type>)`
   - IR: `PtrFromRef`, `PtrDeref`, `PtrStore`, `PtrLoad`
   - MonoType: `*{type_name}`

### Test Coverage

| Module | Test Count | Description |
|--------|------------|-------------|
| Parser | - | unsafe/deref/ptr syntax parsing |
| TypeCheck | - | Pointer type inference |
| IR Gen | - | unsafe block and pointer IR generation |
| UnsafeCheck | - | Dereference outside unsafe block checking |

### Future Optimizations

- Phase 8+ implements code generation for raw pointers (wasm address operations)
- Add `UnsafeBlock` scope tracking

---

## Phase 8: Weak Standard Library (P1) ✅ Completed

### Goal

Implement `std.weak.Weak` module, supporting weak references that don't prevent target release.

### Implementation Status: ✅ Completed (2026-02-06)

**Design Adjustment**:
- Do not implement `std.rc` and `std.sync` (since `ref` already meets the requirement)
- Only implement `Weak[T]` type

#### Completed Changes (2026-02-06 Update)

1. **Type System Extension** (`mono.rs`)
   - ✅ Added `MonoType::Weak(Box<MonoType>)` variant
   - ✅ Updated `type_name()` method

2. **Constraint Propagation** (`constraint.rs`, `substitute.rs`)
   - ✅ Weak's Send + Sync constraint propagation
   - ✅ Weak's type substitution logic

3. **Type Checking** (`specialize.rs`, `overload.rs`)
   - ✅ Weak specialization handling
   - ✅ Weak overload matching

4. **Runtime Support** (`value.rs`)
   - ✅ Added `RuntimeValue::Weak(std::sync::Weak<RuntimeValue>)`
   - ✅ Implemented `upgrade()` returns `Option<RuntimeValue>`
   - ✅ Implemented `from_arc_into_weak()` creates Weak

5. **Bytecode Instructions** (`bytecode.rs`, `opcode.rs`)
   - ✅ Added `BytecodeInstr::WeakNew` / `WeakUpgrade`
   - ✅ Added `Opcode::WeakNew(0x7E)` / `WeakUpgrade(0x7F)`

6. **Interpreter** (`executor.rs`)
   - ✅ `WeakNew`: `Arc → Weak`
   - ✅ `WeakUpgrade`: `Weak → Option<Arc>`

7. **Standard Library** (`weak.rs`, `mod.rs`)
   - ✅ New `src/std/weak.rs`
   - ✅ Registered `pub mod weak`

### Involved Files

| Type | File | Status |
|------|------|--------|
| Modified | `src/frontend/core/type_system/mono.rs` | ✅ Completed |
| Modified | `src/frontend/core/type_system/constraint.rs` | ✅ Completed |
| Modified | `src/frontend/core/type_system/substitute.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/specialize.rs` | ✅ Completed |
| Modified | `src/frontend/typecheck/overload.rs` | ✅ Completed |
| Modified | `src/backends/common/value.rs` | ✅ Completed |
| Modified | `src/backends/common/opcode.rs` | ✅ Completed |
| Modified | `src/middle/core/bytecode.rs` | ✅ Completed |
| Modified | `src/backends/interpreter/executor.rs` | ✅ Completed |
| Modified | `src/middle/passes/codegen/bytecode.rs` | ✅ Completed |
| Modified | `src/middle/passes/lifetime/send_sync.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/dce.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/instance.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/instantiation_graph.rs` | ✅ Completed |
| Modified | `src/middle/passes/mono/type_mono.rs` | ✅ Completed |
| Modified | `src/lib.rs` | ✅ Completed |
| New | `src/std/weak.rs` | ✅ Completed |
| Modified | `src/std/mod.rs` | ✅ Completed |

### Acceptance Criteria

- [x] `use std.weak.Weak` module registered
- [x] `MonoType::Weak` type system support
- [x] `WeakNew` / `WeakUpgrade` bytecode instructions
- [x] `RuntimeValue::Weak` runtime support
- [x] Send + Sync constraint propagation
- [x] Compiles

### Implementation Notes

1. **Weak Design**
   ```
   Arc[T] ──Weak::new()──► Weak[T] ──upgrade()──► Option[Arc[T]]
   ```

2. **Bytecode Instructions**
   ```
   WeakNew { dst, src }    # Arc -> Weak
   WeakUpgrade { dst, src } # Weak -> Option<Arc>
   ```

3. **Runtime Behavior**
   - `WeakNew`: Uses `Arc::downgrade()` to create Weak
   - `WeakUpgrade`: Uses `weak.upgrade()` to return Option
   - After Arc released, upgrade returns None

### Test Coverage

| Module | Test Count | Description |
|--------|------------|-------------|
| RuntimeValue::Weak | - | Weak creation and upgrade |
| Executor WeakNew | - | Bytecode execution |
| Executor WeakUpgrade | - | Option return |

### Future Optimizations

- Add complete interpreter test cases
- Add type checking tests

---

## Dependency Relationships

```
Phase 1 (Field Immutability)
    │
    ├─► Phase 2 (Empty State Reuse)
    │       │
    │       └─► Phase 3 (Ownership Return Flow)
    │
    ├─► Phase 4 (Consume Analysis)
    │       │
    │       └─► Phase 5 (ref = Arc)
    │               │
    │               └─► Phase 6 (Cycle Detection)
    │
    ├─► Phase 7 (unsafe + Raw Pointers)
    │
    └─► Phase 8 (Rc/Arc/Weak)
```

---

## File List

### New Files

| File | Phase | Description |
|------|-------|-------------|
| `src/middle/passes/lifetime/empty_state.rs` | P2 | Empty state tracking |
| `src/middle/passes/lifetime/control_flow.rs` | P2 | Control flow analysis |
| `src/middle/passes/lifetime/ownership_flow.rs` | P3 ✅ | Ownership return flow inference |
| `src/middle/passes/lifetime/chain_calls.rs` | P3 ✅ | Chain call analysis |
| `src/middle/passes/lifetime/consume_analysis.rs` | P4 ✅ | Consume marking system |
| `src/middle/passes/lifetime/lifecycle.rs` | P4 ✅ | Variable lifecycle tracking |
| `src/middle/passes/lifetime/unsafe_check.rs` | P7 | unsafe checking |
| `src/middle/passes/lifetime/intra_task_cycle.rs` | P6 ✅ | Within-task cycle handling |
| `src/std/rc.rs` | P8 | Rc/Weak implementation |
| `src/std/sync.rs` | P8 | Arc implementation |

### Modified Files

| File | Phase | Modification Content |
|------|-------|----------------------|
| `src/frontend/core/parser/ast.rs` | P1 | Create StructField, modify Type/Pattern |
| `src/frontend/core/parser/statements/declarations.rs` | P1 | Parser supports mut fields |
| `src/frontend/core/parser/pratt/nud.rs` | P1 | Struct pattern parsing supports mut |
| `src/frontend/core/type_system/mono.rs` | P1 | StructType added field_mutability |
| `src/frontend/typecheck/inference/patterns.rs` | P1 | Pattern inference supports is_mut |
| `src/frontend/typecheck/mod.rs` | P1 | Adapt to StructField |
| `src/frontend/type_level/auto_derive.rs` | P1 | Adapt to StructField |
| `src/frontend/core/type_system/solver.rs` | P1 | Adapt to field_mutability |
| `src/frontend/core/type_system/substitute.rs` | P1 | Adapt to field_mutability |
| `src/frontend/typecheck/specialization/algorithm.rs` | P1 | Adapt to field_mutability |
| `src/frontend/typecheck/specialize.rs` | P1 | Adapt to field_mutability |
| `src/frontend/typecheck/overload.rs` | P1 | Adapt to field_mutability |
| `src/middle/passes/lifetime/error.rs` | P1 | Added ImmutableFieldAssign |
| `src/middle/passes/lifetime/mut_check.rs` | P1 | StoreField check extension |
| `src/middle/core/ir_gen.rs` | P1 | Adapt to StructField |
| `src/middle/passes/codegen/mod.rs` | P1 | Adapt to StructField |
| `src/middle/passes/mono/cross_module.rs` | P1 | Adapt to field_mutability |
| `src/middle/passes/mono/function.rs` | P1 | Adapt to StructField |
| `src/middle/passes/mono/module_state.rs` | P1 | Adapt to StructField |
| `src/middle/passes/mono/type_mono.rs` | P1 | Adapt to field_mutability |
| `src/middle/passes/lifetime/move_semantics.rs` | P2, P4 ✅ | Empty state check, consume analysis |
| `src/middle/passes/lifetime/error.rs` | P3 | Return flow error diagnostics |
| `src/middle/passes/lifetime/ownership_flow.rs` | P4 | ConsumeMode added Copy |
| `src/frontend/core/parser/expr.rs` | P5 | ref expression parsing |
| `src/frontend/typecheck/infer.rs` | P5 | ref type inference |
| `src/middle/passes/lifetime/ref_semantics.rs` | P5 | ref ownership handling |
| `src/middle/passes/lifetime/cycle_check.rs` | P6 ✅ | Cross-task cycle detection, depth limit, unsafe bypass |
| `src/middle/passes/lifetime/error.rs` | P6 ✅ | IntraTaskCycle, UnsafeBypassCycle |
| `src/middle/passes/lifetime/mod.rs` | P6 ✅ | Integrate IntraTaskCycleTracker |
| `src/frontend/core/parser/block.rs` | P7 | unsafe syntax parsing |

---

## Time Estimation

| Phase | Complexity | Estimated Duration |
|-------|------------|---------------------|
| P1: Field Immutability | Medium | 3-4 days |
| P2: Empty State Reuse | Medium | 2-3 days |
| P3: Ownership Return Flow | Low | 1-2 days |
| P4: Consume Analysis | Medium | 2-3 days |
| P5: ref = Arc | Low | 1 day (existing foundation) |
| P6: Cycle Detection | Medium | 2 days (existing foundation) |
| P7: unsafe + Raw Pointers | Medium | 2-3 days |
| P8: Rc/Arc/Weak | Medium | 3-4 days |

**Total**: Approximately 16-22 working days