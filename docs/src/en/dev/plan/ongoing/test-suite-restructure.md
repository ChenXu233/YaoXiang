# Test Suite Refactoring Plan

> Status: Planned
> Branch: refactor/test-suite
> Date: 2026-05-10

## I. Why Refactor

### Current Problems

1. **All 1752 tests pass, but real bugs were missed**
   - match expressions return 0 at runtime (ir_gen doesn't handle Match nodes)
   - list comprehensions return 0 (ir_gen doesn't handle ListComp nodes)
   - `x: Int = 42` typed variable declaration fails to parse

2. **Integration tests only verify compilation success, not runtime output correctness**
   - `tests/integration/interpreter.rs` only does `assert!(result.is_ok())`
   - `tests/integration/execution.rs` is completely commented out

3. **No E2E .yx file organization**
   - Old and new files mixed: `closure_test.yx` (old) and `spec_features_test.yx` (new) in the same directory
   - No naming conventions: `closure_test.yx`, `closure_test2.yx`, `mut_param_test.yx`
   - No coverage planning: no mapping to language spec chapters

4. **Inline tests are fragmented**
   - `src/frontend/typecheck/tests/` has 23 files, many testing the same thing
   - scope tests scattered across 4 files
   - infer tests scattered across 3 files
   - `typecheck_fixes.rs` appears to be a historical patch leftover

5. **Codegen tests are isolated**
   - All hand-crafted IR, not through the full parser→typecheck→ir_gen pipeline
   - Tests "can hand-written IR be translated to bytecode" instead of "does source code compilation produce correct results"

### Refactoring Goals

1. **Establish a three-layer test system**, each layer with clear responsibilities and coverage standards
2. **E2E tests can double as benchmarks** — each .yx test file can measure execution time
3. **Internal tests are normalized** — unified testing conventions, naming, and assertion patterns
4. **Coverage of key language spec paths** — ensure syntax features defined in the language spec have corresponding tests

---

## II. Three-Layer Test System

### Layer 1: E2E .yx Test Suite (tests/yaoxiang/)

Organized by language spec chapters, each file corresponds to one syntax feature.

```
tests/yaoxiang/
├── 00-smoke/                 # Smoke tests
│   └── hello.yx
│
├── 01-basics/                # Basic syntax (spec chapters 2/4/5)
│   ├── variables.yx          # Variable declaration + type inference
│   ├── typed_vars.yx         # Typed variables x: Int = 42
│   ├── operators.yx          # All operators
│   ├── literals.yx           # All literals
│   └── comments.yx           # Comments
│
├── 02-functions/             # Functions (spec chapter 6)
│   ├── definitions.yx        # name: (params) -> Ret = ...
│   ├── lambdas.yx            # Lambda expressions
│   ├── closures.yx           # Higher-order functions
│   └── generics.yx           # Generic functions
│
├── 03-control-flow/          # Control flow (spec chapters 4/5)
│   ├── if_else.yx
│   ├── while.yx
│   ├── for.yx
│   ├── match.yx
│   └── list_comp.yx          # List comprehensions
│
├── 04-types/                 # Type system (spec chapter 3)
│   ├── structs.yx            # Point: Type = { x: Float, y: Float }
│   ├── enums.yx              # Color: Type = { red | green | blue }
│   └── generics.yx           # Option: (T: Type) -> Type = ...
│
├── 05-data-structures/       # Collection types (spec section 2.6)
│   ├── lists.yx
│   ├── tuples.yx
│   └── dicts.yx
│
├── 06-modules/               # Module system (spec chapter 7)
│   ├── imports.yx
│   └── lib/
│
└── 07-errors/                # Error handling (spec chapter 9, marks unimplemented features)
    ├── result.yx
    └── option.yx
```

**File conventions**:

```yaoxiang
// 01-basics/variables.yx
// Covers: spec §5.2 Variable declarations, §6.2 Type inference
// Verifies: Basic declarations, type inference, mutability
// Branch: refactor/test-suite
// Status: ✅ Runnable

use std.io

main = {
    x = 42
    io.println(x)
    // expect: 42

    s = "hello"
    io.println(s)
    // expect: hello

    io.println("ALL TESTS PASSED")
}
```

**Assertion mechanism**: The Rust test framework captures stdout and verifies that the string `ALL TESTS PASSED` appears in the output of each .yx file.

**Benchmark extension**: `.yx` test files naturally serve as performance benchmarks — measure execution time on each run. In the future, they can be wrapped with `criterion` to track performance regressions.

### Layer 2: Integration Tests (tests/integration/)

Test the complete compile+execute pipeline, verify output values.

| Current file | Action | Description |
|-------------|--------|-------------|
| `interpreter.rs` | Rewrite | Compile source → execute → assert output values |
| `execution.rs` | Rewrite (uncomment) | Fix stack overflow, run real .yx files |
| `codegen.rs` | Keep | Bytecode serialization/deserialization |
| `codegen_extended.rs` | Keep | opcode/metadata tests |
| `fstring.rs` | Keep | Add execution verification |
| `backends.rs` | Keep | RuntimeValue type tests |

**Supplement**: `tests/yx_runner.rs` — Auto-discover and run all .yx files under `tests/yaoxiang/`.

### Layer 3: Unit Tests (src/*/tests/)

Test internal logic of individual modules, can access private APIs.

#### 3.1 Lexer Tests (src/frontend/core/lexer/tests/)

11 files → Delete 1 debug file, keep 10.

| Action | File |
|--------|------|
| Delete | `debug_lexer.rs` — Debug only |
| Keep | `basic.rs`, `comments.rs`, `keywords.rs`, `literals.rs`, `operators.rs` |
| Keep | `delimiters.rs`, `errors.rs`, `fstring.rs` |
| Keep | `rfc004_lexer.rs`, `rfc010_lexer.rs` |

#### 3.2 Parser Tests (src/frontend/core/parser/tests/)

13 files → Minor adjustments after review.

| Action | File |
|--------|------|
| Keep | `basic.rs`, `fn_def.rs`, `syntax_validation.rs`, `old_syntax_rejection.rs` |
| Keep | `boundary.rs`, `concurrency.rs`, `fstring.rs` |
| Keep | `ref_test.rs`, `unsafe_ptr.rs`, `state.rs` |
| Review | `binding_enhancements.rs` — Check for duplication with fn_def |

#### 3.3 Typecheck Tests (src/frontend/typecheck/tests/)

**Largest problem area**: 23 files → Merge into 12.

| Action | Original file | Target file |
|--------|--------------|-------------|
| Merge | `infer.rs` + `inference.rs` + `types.rs` | `type_inference.rs` |
| Merge | `scope.rs` + `shadowing.rs` + `use_scope.rs` + `use_block_scope.rs` | `scoping.rs` |
| Merge | `visibility.rs` + `pub_bind.rs` | `visibility.rs` |
| Review | `typecheck_fixes.rs` | Merge into corresponding file if it's just historical patch testing, then delete |
| Keep | `basic.rs`, `check.rs` | — |
| Keep | `constraint.rs`, `concurrency.rs`, `fstring.rs` | — |
| Keep | `gat.rs`, `ref_test.rs`, `result_try.rs` | — |
| Keep | `semantic_tokens.rs`, `traits.rs`, `type_constructor_rules.rs` | — |

#### 3.4 Middle/Codegen Tests (src/middle/passes/tests/)

| Directory | Action |
|-----------|--------|
| `codegen/` | Keep existing, **supplement integrated codegen tests** (compile from source to IR verify structure) |
| `lifetime/` | Keep as-is |
| `mono/` | Keep as-is |
| `module/` | Keep as-is |

## III. Test Standards Documentation

Create `TEST_STANDARD.md` in the same directory, with the following content:

### Naming Conventions

```
Purpose        Pattern                    Example
─────────────────────────────────────────────────────────────────────────
Test module    mod_<description>_tests     mod_parser_basic_tests
Test function  test_<feature>_<scenario>   test_parse_fn_def_no_params
E2E file       <chapter>-<feature>.yx      01-basics-variables.yx
```

### Assertion Conventions

- E2E `.yx` files: Output `ALL TESTS PASSED` at the end
- Integration tests: Verify stdout contains expected values
- Unit tests: Verify data structure field values, don't use `assert!(result.is_ok())` as the sole assertion

### Comment Conventions

```
// E2E file header:
 // Covers: spec §X.X
// Verifies: One-sentence description
// Branch: refactor/test-suite
// Status: ✅ Runnable / ⚠️ Needs Fix / 🔴 Not Implemented
```

### Handling Unimplemented Features

- E2E `.yx` for non-existent features: Don't write tests, add them after implementation
- Unit tests referencing unimplemented features: Mark with `#[ignore]`, comment "Enable after XXX is implemented"

---

## IV. Execution Plan

### Phase 0: Preparation

- [ ] Create branch `refactor/test-suite` from `dev`
- [ ] Review `typecheck_fixes.rs` and `binding_enhancements.rs`, determine if they should be deleted
- [ ] Review the stack overflow issue in `tests/integration/execution.rs`

### Phase 1: E2E Test Framework

- [ ] Create `tests/yx_runner.rs` — Auto-discover and run `tests/yaoxiang/**/*.yx`
- [ ] Create new directory structure for `tests/yaoxiang/`
- [ ] Write 00-smoke smoke tests
- [ ] Write 01-basics layer (currently runnable syntax)
- [ ] Write 02-functions layer

### Phase 2: Runtime Bug Fixes + Corresponding Tests

- [ ] Fix match expressions (ir_gen adds Match handling)
- [ ] Fix list comprehensions (ir_gen adds ListComp handling)
- [ ] Fix `x: Int = 42` variable type annotation
- [ ] Add corresponding .yx E2E tests for the above fixes

### Phase 3: Integration Test Rewrite

- [ ] Rewrite `tests/integration/interpreter.rs` (verify runtime output values)
- [ ] Rewrite `tests/integration/execution.rs` (fix stack overflow)
- [ ] Supplement integrated codegen tests (from source code to IR)

### Phase 4: Inline Test Consolidation

- [ ] typecheck tests 23→12 merge
- [ ] Delete `debug_lexer.rs`
- [ ] Review parser test duplicates

### Phase 5: Create Test Standards Documentation

- [ ] Create `TEST_STANDARDS.md` in `tests/yaoxiang/` root directory

---

## V. Verification Methods

```bash
# All tests
cargo test

# E2E tests
cargo test --test yx_runner

# Unit tests
cargo test --lib

# Manually run .yx file
cargo run -- run tests/yaoxiang/01-basics/variables.yx

# Benchmark run
cargo bench
```

---

## VI. File Checklist

### New Files

- `tests/yx_runner.rs` — E2E test runner
- `tests/yaoxiang/TEST_STANDARDS.md` — Test standards
- `tests/yaoxiang/00-smoke/hello.yx`
- `tests/yaoxiang/01-basics/variables.yx`
- `tests/yaoxiang/01-basics/typed_vars.yx`
- `tests/yaoxiang/01-basics/operators.yx`
- `tests/yaoxiang/01-basics/literals.yx`
- `tests/yaoxiang/01-basics/comments.yx`
- `tests/yaoxiang/02-functions/definitions.yx`
- `tests/yaoxiang/02-functions/lambdas.yx`
- `tests/yaoxiang/02-functions/closures.yx`
- `tests/yaoxiang/03-control-flow/if_else.yx`
- `tests/yaoxiang/03-control-flow/while.yx`
- `tests/yaoxiang/03-control-flow/for.yx`
- `tests/yaoxiang/03-control-flow/match.yx`
- `tests/yaoxiang/05-data-structures/lists.yx`
- `tests/yaoxiang/05-data-structures/tuples.yx`
- `tests/yaoxiang/06-modules/imports.yx`
- `tests/yaoxiang/06-modules/lib/math.yx`

### Files to Delete

- `tests/yaoxiang/closure_test.yx`
- `tests/yaoxiang/closure_test2.yx`
- `tests/yaoxiang/list_test.yx`
- `tests/yaoxiang/mut_param_test.yx`
- `tests/yaoxiang/mut_param_error_test.yx`
- `tests/yaoxiang/impl_status_test.yx`
- `tests/yaoxiang/spec_basics_test.yx`
- `tests/yaoxiang/spec_features_test.yx`
- `tests/yaoxiang/spec_functions_test.yx`
- `tests/yaoxiang/spec_types_test.yx`
- `src/frontend/core/lexer/tests/debug_lexer.rs` (pending confirmation)

### Files to Modify

- `tests/integration/interpreter.rs` — Rewrite
- `tests/integration/execution.rs` — Rewrite
- `src/frontend/core/ir_gen.rs` — Fix match and listcomp
- `src/frontend/typecheck/` — Fix `x: Int = 42`
- `src/frontend/typecheck/tests/` — Merge 23→12 files