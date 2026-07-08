# Stub Code Checklist

> Generated Date: 2026-06-13
> Scope: Entire project (`src/`)
> Check Type: `todo!()`, empty function bodies, hardcoded return values, dead code

## Statistics Overview

| Type | Count | Priority Distribution |
|------|------|-----------|
| `todo!()` | 4 occurrences | P0: 4 |
| Empty function body | 6 occurrences | P0: 2, P1: 2, P2: 2 |
| Hardcoded return value | 14 occurrences | P0: 5, P1: 8, P2: 1 |
| Dead code | 14 occurrences | P2: 14 |
| Duplicate implementations | 4 occurrences | P2: 4 |
| **Total** | **42 occurrences** | |

---

## P0 - High Priority (Core Functionality Missing)

### 1. Debugger Step Methods (4 `todo!()` occurrences)

**File**: `src/backends/interpreter/executor/debug.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 32-34 | `fn step(&mut self) -> ExecutorResult<()>` | Execute a single instruction | `todo!()` |
| 36-38 | `fn step_over(&mut self) -> ExecutorResult<()>` | Step over (do not enter function body) | `todo!()` |
| 40-42 | `fn step_out(&mut self) -> ExecutorResult<()>` | Step out (execute until current function returns) | `todo!()` |
| 44-46 | `fn run(&mut self) -> ExecutorResult<()>` | Continue running to the next breakpoint | `todo!()` |

**Context**: Other methods of the `DebuggableExecutor` trait (`set_breakpoint`, `has_breakpoint`, `current_ip`, `current_function`, `breakpoints`) are implemented; only step control is missing.

**Implementation Suggestions**:
- `step()`: Execute the instruction at the current IP, then IP++
- `step_over()`: If the current instruction is a function call, set a temporary breakpoint at the next instruction, then run
- `step_out()`: Record the current call stack depth, then run until the stack depth decreases
- `run()`: Loop execution until a breakpoint is hit or the program ends

---

## P1 - Medium Priority (Incomplete Functionality)

### 6. LSP Progress Notification (1 empty function body)

**File**: `src/frontend/events/subscribe.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 357-364 | `fn on_event(&self, _event: &dyn Any, _metadata: &EventMetadata)` | Convert event to LSP notification | Empty implementation |

**Context**: The comment says "LSP notification logic can be added here, e.g., converting progress events into `$/progress` notifications".

**Implementation Suggestions**:
- Check the event type (Progress, Diagnostic, etc.)
- For Progress events, send `window/workDoneProgress/create` and `$/progress` notifications
- For Diagnostic events, send `textDocument/publishDiagnostics` notifications

---

### 7. Old Syntax Skip Function (1 empty function body)

**File**: `src/frontend/core/parser/statements/declarations.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 171-174 | `fn skip_old_function_syntax(_state: &mut ParserState<'_>)` | Skip the entire declaration of the old function syntax | Empty implementation |

**Context**: The comment says "Old syntax has been removed; this function is no longer needed".

**Suggestion**: Check whether there are any callers. If none, delete it directly.

### 11. Boundary Checks (2 hardcoded return values)

**File**: `src/frontend/core/typecheck/inference/bounds.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 70-79 | `pub fn check_const_bounds(&self, _ty: &MonoType, _bounds: &[ConstBound]) -> Result<()>` | Check const bounds | Hardcoded `Ok(())` |
| 81-90 | `pub fn check_lifetime_bounds(&self, _ty: &MonoType, _bounds: &[LifetimeBound]) -> Result<()>` | Check lifetime bounds | Hardcoded `Ok(())` |

**Implementation Suggestions**:
- `check_const_bounds`:
  - Check whether const parameters satisfy the bound constraints
  - Check whether const expressions can be evaluated
- `check_lifetime_bounds`:
  - Check whether lifetime parameters satisfy the bound constraints
  - Check whether the lifetime is longer than the constraint

---

### 13. Generic Constraint Parsing (1 hardcoded return value)

**File**: `src/frontend/core/typecheck/inference/generics.rs`

| Line | Function Signature | Expected Functionality | Status |
|------|----------|----------|------|
| 53-59 | `pub fn infer_generic_constraints(&mut self, _constraints: &[String]) -> Result<()>` | Parse from constraint string to internal representation | Hardcoded `Ok(())` |

**Implementation Suggestions**:
- Parse the constraint string (e.g., `T: Clone + Debug`)
- Extract the type parameters and constraints
- Convert the constraints to the internal representation (TraitConstraint)
- Add them to the type environment

---

## P2 - Low Priority (Safely Deletable Dead Code)

### 28-31. Duplicate `substitute_type` Implementations

| # | File | Line | Signature | Difference | Suggestion |
|---|------|------|------|------|------|
| 28 | `types/eval/evaluator.rs` | 1040 | `fn substitute_type(body, param_name, replacement)` | TypeRef substitution only | Delete (no callers) |
| 29 | `types/solver.rs` | 685 | `fn substitute_type(&self, ty, substitution)` | Complete subnode substitution | Keep |
| 31 | `middle/passes/mono/cross_module.rs` | 609 | `fn substitute_type(generic_type, type_args, type_params)` | Replace by parameter list | Keep |

---

## Reasonable Empty Implementations (Kept)

| File | Function | Reason |
|------|------|------|
| `frontend/events/mod.rs:131` | `NullEmitter::emit/emit_with` | Null Object Pattern |
| `backends/runtime/facade.rs:306,331` | `EmbeddedRuntime::cancel/drive_until` | Embedded runtime semantics |
| `backends/common/allocator.rs:195` | `BumpAllocator::dealloc` | Bump allocator characteristic |
| `frontend/core/typecheck/passes/dead_code.rs:190` | `collect_definitions` | Already deprecated, reasonable stub |

---

## Implementation Progress Tracking

---

## Notes
- P0/P1 require careful design; it is recommended to implement them one by one and add tests
- Some functions may have hidden callers (via trait objects or macros); please confirm again before deleting
- It is recommended to unify the duplicate `substitute_type` implementations into a single one