# Stub Code Inventory

> Generation Date: 2026-06-13
> Inspection Scope: Entire project (`src/`)
> Inspection Types: `todo!()`, empty function bodies, hardcoded return values, dead code

## Statistical Overview

| Type | Count | Priority Distribution |
|------|------|-----------|
| `todo!()` | 4 | P0: 4 |
| Empty function body | 6 | P0: 2, P1: 2, P2: 2 |
| Hardcoded return value | 14 | P0: 5, P1: 8, P2: 1 |
| Dead code | 14 | P2: 14 |
| Duplicate implementation | 4 | P2: 4 |
| **Total** | **42** | |

---

## P0 - High Priority (Core Functionality Missing)

### 1. Debugger Stepping Methods (4 `todo!()` occurrences)

**File**: `src/backends/interpreter/executor/debug.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 32-34 | `fn step(&mut self) -> ExecutorResult<()>` | Execute a single instruction step | `todo!()` |
| 36-38 | `fn step_over(&mut self) -> ExecutorResult<()>` | Step over (skip into function body) | `todo!()` |
| 40-42 | `fn step_out(&mut self) -> ExecutorResult<()>` | Step out (execute until current function returns) | `todo!()` |
| 44-46 | `fn run(&mut self) -> ExecutorResult<()>` | Continue running to the next breakpoint | `todo!()` |

**Context**: Other methods of the `DebuggableExecutor` trait (`set_breakpoint`, `has_breakpoint`, `current_ip`, `current_function`, `breakpoints`) have been implemented; only stepping control is unimplemented.

**Implementation Suggestions**:
- `step()`: Execute the instruction at the current IP, then IP++
- `step_over()`: If the current instruction is a function call, set a temporary breakpoint at the next instruction, then run
- `step_out()`: Record the current call stack depth, run until the stack depth decreases
- `run()`: Loop execution until hitting a breakpoint or program end

---

### 2. Control Flow Analysis Core (2 empty function bodies)

**File**: `src/middle/passes/lifetime/control_flow.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 145-154 | `fn analyze_instruction(&self, _instr: &Instruction, _state: &mut HashMap<Operand, ValueState>, _pos: (usize, usize))` | Analyze lifetime state changes for a single instruction | Empty implementation |
| 155-163 | `fn merge_block_state(&mut self, _block_state: &HashMap<Operand, ValueState>, _block_idx: usize)` | Merge ValueState from different basic blocks | Empty implementation |

**Context**: Comments note "currently an empty implementation, can be extended as needed in the future" and "control flow analysis already has a basic implementation in MoveChecker".

**Implementation Suggestions**:
- `analyze_instruction`:
  - Match instruction type (Move, Copy, Call, Branch, etc.)
  - Update the corresponding Operand's ValueState in `_state` (Moved/Empty/Partial)
  - Handle Move of function call arguments
- `merge_block_state`:
  - For cases where multiple predecessor blocks converge, take the least upper bound (LUB) of ValueState
  - If any predecessor is Moved, the merged state is Moved
  - If predecessor states conflict, report an error or mark as MaybeMoved

**Impact**: The current lifetime pass degenerates to a no-op and cannot detect use-after-move errors.

---

### 3. Trait Object Safety Check (2 hardcoded return values)

**File**: `src/frontend/core/typecheck/traits/object_safety.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 62-71 | `fn check_associated_types(&self, _trait_name: &str) -> Result<(), ObjectSafetyError>` | Check whether associated types are object-safe | Hardcoded `Ok(())` |
| 74-85 | `fn check_method_signatures(&self, _trait_name: &str) -> Result<(), ObjectSafetyError>` | Check whether method signatures are object-safe | Hardcoded `Ok(())` |

**Context**: Comments "simplified implementation: assume no associated types or that all associated types are safe" and "assume basic Trait methods are all object-safe".

**Implementation Suggestions**:
- `check_associated_types`:
  - Get all associated types of the trait
  - Check whether an associated type has a `Self` constraint (unsafe)
  - Check whether the associated type is used in method signatures (unsafe)
- `check_method_signatures`:
  - Check whether the method's return type contains `Self` (unsafe)
  - Check whether the method has generic parameters (unsafe)
  - Check whether the method uses a `where Self: Sized` constraint (safe, but needs special handling)

---

### 4. Trait Coherence Check (3 hardcoded return values)

**File**: `src/frontend/core/typecheck/traits/coherence.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 38-43 | `fn check_conflicting_implementations(&self) -> Result<()>` | Check for conflicting trait implementations | Hardcoded `Ok(())` |
| 46-50 | `fn check_orphan_rule(&self) -> Result<()>` | Check orphan rules | Hardcoded `Ok(())` |
| 80-86 | `fn find_orphan_implementations(&self) -> Result<()>` | Scan and check all trait implementations | Hardcoded `Ok(())` |

**Context**: Comments "simplified implementation: check for duplicate Trait implementations" and "ensure Trait implementations conform to the orphan rule".

**Implementation Suggestions**:
- `check_conflicting_implementations`:
  - Collect all trait implementations
  - For multiple implementations of the same type, check for overlaps
  - Report conflicting implementations
- `check_orphan_rule`:
  - For each trait impl, check whether the trait or type is defined in the current crate
  - If neither is, report an orphan rule violation
- `find_orphan_implementations`:
  - Iterate over trait impls in all modules
  - Call `check_orphan_rule` for each implementation

---

### 5. Trait Implementation Signature Check (1 hardcoded return value)

**File**: `src/frontend/core/typecheck/traits/impl_check.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 95-103 | `fn check_signature(&self, _trait_def: &TraitDef, _params: &[Param]) -> Result<()>` | Check whether impl method signatures match the trait definition | Hardcoded `Ok(())` |

**Context**: Currently only checks whether method names exist; signature checking is empty.

**Implementation Suggestions**:
- Compare parameter types (including generic parameters)
- Compare return types
- Compare mut modifiers
- Compare lifetime parameters
- Report the specific location of any mismatch

---

## P1 - Medium Priority (Incomplete Functionality)

### 6. LSP Progress Notification (1 empty function body)

**File**: `src/frontend/events/subscribe.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 357-364 | `fn on_event(&self, _event: &dyn Any, _metadata: &EventMetadata)` | Convert events to LSP notifications | Empty implementation |

**Context**: Comment "LSP notification logic can be added here, e.g., converting progress events to `$/progress` notifications".

**Implementation Suggestions**:
- Check event type (Progress, Diagnostic, etc.)
- For Progress events, send `window/workDoneProgress/create` and `$/progress` notifications
- For Diagnostic events, send `textDocument/publishDiagnostics` notifications

---

### 7. Old Syntax Skip Function (1 empty function body)

**File**: `src/frontend/core/parser/statements/declarations.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 171-174 | `fn skip_old_function_syntax(_state: &mut ParserState<'_>)` | Skip the entire declaration of old function syntax | Empty implementation |

**Context**: Comment "old syntax has been removed; this function is no longer needed".

**Suggestion**: Check whether callers still exist; if not, delete it directly.

---

### 8. GAT Check (2 hardcoded return values)

**File**: `src/frontend/core/typecheck/traits/gat/checker.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 122-131 | `fn validate_generic_usage(&self, _ty: &MonoType) -> Result<()>` | Validate whether generic parameter usage is legal | Hardcoded `Ok(())` |
| 174-184 | `pub fn check_associated_type_constraints(...)` | Check associated type constraints | Hardcoded `Ok(())` |

**Implementation Suggestions**:
- `validate_generic_usage`:
  - Check whether generic parameters are used in allowed positions
  - Check for unused generic parameters
  - Check for usage that violates constraints
- `check_associated_type_constraints`:
  - Check whether associated types satisfy constraints
  - Check whether constraints are satisfiable

---

### 9. Higher-Rank Type Lifetime Check (1 hardcoded return value)

**File**: `src/frontend/core/typecheck/traits/gat/higher_rank.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 100-109 | `fn check_lifetime_constraints(&self, _ty: &MonoType) -> Result<()>` | Check usage of lifetime parameters | Hardcoded `Ok(())` |

**Implementation Suggestions**:
- Check whether lifetime parameters are used in allowed positions
- Check whether lifetime parameters satisfy constraints
- Check for usage that violates higher-rank lifetime rules

---

### 10. Constraint Propagation (1 hardcoded return value)

**File**: `src/frontend/core/typecheck/traits/solver.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 311-318 | `pub fn propagate_constraints_to_type_args(&self, _ty: &MonoType, _trait_name: &str) -> Vec<TraitConstraint>` | Extract sub-constraints from type parameters and propagate | Hardcoded `Vec::new()` |

**Implementation Suggestions**:
- Get the generic parameters of the type
- For each generic parameter, check its constraints
- Propagate constraints to concrete type arguments
- Return the list of propagated constraints

---

### 11. Bounds Check (2 hardcoded return values)

**File**: `src/frontend/core/typecheck/inference/bounds.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 70-79 | `pub fn check_const_bounds(&self, _ty: &MonoType, _bounds: &[ConstBound]) -> Result<()>` | Check const bounds | Hardcoded `Ok(())` |
| 81-90 | `pub fn check_lifetime_bounds(&self, _ty: &MonoType, _bounds: &[LifetimeBound]) -> Result<()>` | Check lifetime bounds | Hardcoded `Ok(())` |

**Implementation Suggestions**:
- `check_const_bounds`:
  - Check whether const parameters satisfy bound constraints
  - Check whether const expressions are evaluable
- `check_lifetime_bounds`:
  - Check whether lifetime parameters satisfy bound constraints
  - Check whether lifetimes are longer than the constraint

---

### 12. Destructuring Assignment Check (1 hardcoded return value)

**File**: `src/frontend/core/typecheck/inference/assignment.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 137-146 | `pub fn check_destructuring(&self, _lhs_patterns: &[Pattern], _rhs: &MonoType, _span: Span) -> Result<()>` | Check whether the shape of destructuring assignment matches | Hardcoded `Ok(())` |

**Implementation Suggestions**:
- Check whether the number of left-side patterns matches the right-side type
- Check whether the type of each pattern matches the type at the corresponding position on the right
- Report the specific location of any mismatch

---

### 13. Generic Constraint Parsing (1 hardcoded return value)

**File**: `src/frontend/core/typecheck/inference/generics.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 53-59 | `pub fn infer_generic_constraints(&mut self, _constraints: &[String]) -> Result<()>` | Parse constraint strings into internal representation | Hardcoded `Ok(())` |

**Implementation Suggestions**:
- Parse constraint strings (e.g., `T: Clone + Debug`)
- Extract type parameters and constraints
- Convert constraints to internal representation (TraitConstraint)
- Add to the type environment

---

## P2 - Low Priority (Dead Code Safe to Delete)

### 14-27. Dead Code List

| # | File | Line | Element | Type | Suggestion |
|---|------|------|------|------|------|
| 14 | `frontend/pipeline.rs` | 907-932 | `impl TypecheckResult` block | Dead code | Delete |
| 15 | `frontend/pipeline.rs` | 960 | `failed_proofs` field | Dead code | Delete |
| 16 | `parser/statements/declarations.rs` | 103-112 | `fn_returns_meta_type` | Dead code | Delete |
| 17 | `parser/statements/declarations.rs` | 114-132 | `generic_params_from_constructor_params` | Dead code | Delete |
| 18 | `parser/statements/types.rs` | 41-63 | `looks_like_parenthesized_lambda` | Dead code | Delete |
| 19 | `types/eval/evaluator.rs` | 1039-1093 | `substitute_type` | Dead code | Delete |
| 20 | `types/eval/evaluator.rs` | 1117-1125 | `integrate_evaluator` | Dead code | Delete |
| 21 | `types/eval/evaluator.rs` | 1131-1154 | `sync_caches` | Dead code | Delete |
| 22 | `module/cache.rs` | 35-36 | `cached_at` field | Dead code | Delete |
| 23 | `util/diagnostic/session.rs` | 13-14 | `cache` field | Dead code | Delete |
| 24 | `middle/passes/lifetime/cycle_check.rs` | 22-23 | `MAX_DETECTION_DEPTH` constant | Dead code | Delete |
| 25 | `middle/passes/lifetime/intra_task_cycle.rs` | 26-27 | `value_defs` field | Dead code | Delete |
| 26 | `typecheck/proof/budget.rs` | 59-65 | `record_time_ms` + `time_ms_used` | Dead code | Delete |
| 27 | `typecheck/layers/termination.rs` | 854-938 | 3 functions | Dead code | Delete |
| 28 | `typecheck/checker.rs` | 1661-1677 | `check_refined_binding` | Dead code | Delete |
| 29 | `typecheck/layers/ownership.rs` | 9-12 | Entire file | Dead code | Delete |
| 30 | `util/diagnostic/emitter/text.rs` | 259-262 | `hint_prefix` | Dead code | Delete |

---

### 28-31. Duplicate `substitute_type` Implementations

| # | File | Line | Signature | Difference | Suggestion |
|---|------|------|------|------|------|
| 28 | `types/eval/evaluator.rs` | 1040 | `fn substitute_type(body, param_name, replacement)` | TypeRef replacement only | Delete (no callers) |
| 29 | `types/solver.rs` | 685 | `fn substitute_type(&self, ty, substitution)` | Full subnode replacement | Keep |
| 30 | `types/traits/specialization/algorithm.rs` | 66 | `fn substitute_type(&self, ty)` | Full subnode replacement | Keep |
| 31 | `middle/passes/mono/cross_module.rs` | 609 | `fn substitute_type(generic_type, type_args, type_params)` | Replaces by parameter list | Keep |

---

## Legitimate Empty Implementations (Retained)

| File | Function | Reason |
|------|------|------|
| `frontend/events/mod.rs:131` | `NullEmitter::emit/emit_with` | Null Object Pattern |
| `backends/runtime/facade.rs:306,331` | `EmbeddedRuntime::cancel/drive_until` | Embedded runtime semantics |
| `backends/common/allocator.rs:195` | `BumpAllocator::dealloc` | Bump allocator characteristics |
| `frontend/core/typecheck/passes/dead_code.rs:190` | `collect_definitions` | Already deprecated, legitimate stub |

---

## Implementation Progress Tracking

| Priority | Total | Completed | Remaining |
|--------|------|--------|------|
| P0 | 12 | 0 | 12 |
| P1 | 11 | 0 | 11 |
| P2 | 19 | 0 | 19 |
| **Total** | **42** | **0** | **42** |

---

## Notes

- P2 dead code can be safely deleted without affecting functionality
- P0/P1 require careful design; it is recommended to implement and add tests one at a time
- Some functions may have hidden callers (via trait objects or macros); it is recommended to verify again before deletion
- Duplicate `substitute_type` implementations should be unified into a single implementation