# Dead Code and Empty Design Cleanup

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion grading, and execution phase sequence are in the main
> RFC-039 body; the positioning of each companion document is in [this directory's index](index.md).

## Positioning and Scope

This document provides the YaoXiang repository's dead code and empty design inventory, determination
methods, item-by-item disposition decisions, and execution order. Cleanup targets fall into four
categories:

| Category                                                              | Scale                                                                   |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Files that never participated in compilation                          | 24 files / 2628 lines (of which 23 files / 1081 lines are real orphans) |
| Dead code that is compiled but never called                           | 1500+ lines                                                             |
| Design documents describing an architecture that never existed        | Three documents in `docs/src/dev/design/check/`                         |
| Features attached to the default entry point, zero tests, zero output | `src/repl/` 1152 lines                                                  |

**The authorized scope is "thorough deletion permitted"** (2026-10-03 decision). However, this
document gives **disposition recommendations contrary to "direct deletion"** for several items — see
[Key Decisions and Reasons](#key-decisions-and-reasons) for rationale.

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), criterion design (see `07-equivalence-oracle.md`). B3
records that the entire `layers/dispatch.rs` chain is unwired; its deletion is assigned to S2.

## Current State

### Three Liability Forms of Dead Code

**Form 1: Placeholder.** Code exists, appears to be doing something, but actually does not.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero calls** in the
  compiler. It is the "runtime check insertion" mechanism designed in `RFC-027 §4.1`, and is
  **completely unwired**.
- `check_type_equivalence` at `src/frontend/core/typecheck/layers/equivalence.rs:216` (the
  `ProofContext` entry point) is only called by tests. **Note** that `is_subtype` in the same file
  is used in production by `inference/assignment.rs:17`, so **the entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler; the `pub` surface only
  has `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` / `RuntimeStats`
  / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend** — the `Executor` trait
  at `backends/mod.rs:356` currently has only one implementer (the interpreter). This needs
  clarification, as the directory name can be misleading.

**Form 2: Residue.** Upstream decisions have been made, but deletion was only half-done.

- The 6 types remaining at `src/middle/passes/mono/instance.rs:416-830` (**415 lines**):
  `FunctionInstance` (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), `CaptureVariable`. **Source is clear**: when `RFC-023`
  deprecated the "closure capture model" it declared "**approximately 850 lines of code deleted
  together with the closure model**". These 415 lines are what survived of those 850 lines —
  **deletion was only half-done**. **Verification**: a repo-wide word-boundary search for these 6
  types shows **zero hits** outside `instance.rs` (including tests). The
  `pub use passes::mono::instance::*;` at `middle/mod.rs:40` is just a glob re-export, not
  consumption.

**Form 3: False coverage.** This is the most dangerous form — **it makes the repository look more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only 5 `pub mod` declarations (`literals` / `state` / `symbols`
/ `tokenizer` / `tokens`) + one `#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;`
(`:104-106`). **No place declares `mod tests;`**, so the entire `lexer/tests/` subtree of 13 files /
686 lines **never participated in compilation**.

Of these, 7 are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 / `delimiters.rs`
3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totalling 19 lines, 0 assertions). **629
lines / 55 real tests never ran**:

| File              | Lines   | `#[test]` |
| ----------------- | ------- | --------- |
| `literals.rs`     | 222     | 30        |
| `rfc010_lexer.rs` | 167     | 12        |
| `lexer_mod.rs`    | 89      | 5         |
| `rfc004_lexer.rs` | 81      | 5         |
| `symbols.rs`      | 70      | 3         |
| **Total**         | **629** | **55**    |

(`fstring.rs` 90 lines / 11 tests does compile, but via the `#[path]` bypass rather than
`mod tests;`; `mod.rs` 38 lines is a declaration file with no assertions.)

**The only file actually running in the lexer layer is one file with 12 tests, and they all only
test the f-string brace raw protocol.**

The insidiousness of this problem is that `cargo test` does not fail (tests that never ran cannot
fail), and CI has no test count baseline or wiring check.

### Empty Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not "designed but not done" — it is that **the documents describe a different
system that never existed**, and frame "designs that might be built" as "deficiencies in an
implemented system".

## Methodology and Confidence Statement

### Reachability Analysis Method

Starting from the two crate roots `src/lib.rs` and `src/main.rs`, parse all 509 `.rs` files' `mod` /
`pub mod` / `#[path]` declarations, build the module graph, and perform BFS reachability analysis.
`include!` targets are judged separately (not counted into reachability).

**This method has known false-positive patterns; two were actually triggered during the writing
process:**

1. `Split-Path` on Windows returns `\`, which doesn't match the forward-slash key — at one point 499
   orphans were falsely reported.
2. **The sub-module directory for non-`mod.rs` files is `self_name/` rather than `parent_dir/`** —
   `orchestrator.rs`'s `mod tests` was parsed as `src/frontend/module/tests/`, nearly misclassifying
   753 lines of live tests as dead code.

Therefore **every conclusion is double-confirmed by "actually reading the file + actually searching
for references"**; entries that did not pass secondary confirmation are not included in this
document.

### Line Count Convention

All line counts in this document are counted with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines, systematically undercounting by about 5%** (e.g.,
`manifesto.md` reports 658 with `-Line` but actually has 893). An earlier analysis of this repo gave
a low total due to this convention; it has been corrected here.

### Known Limitations

- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Patterns
  beyond Form 2 require manual judgment item by item.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types must be
  found by search; this method does not cover them.
- This document does not run the compiler, so all "never ran" conclusions are static determinations.

## A. Files That Never Participated in Compilation

**Total 24 files / 2628 lines.** Of these, `semantic_tokens.rs` is Form 4 (see section F), and the
remaining **23 files / 1081 lines are real orphans**.

| Path                                                            | Scale                   | Content                                                                       | Judgment Basis                                                                                                                                                                |
| --------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/tests/` **entire subtree of 13 files** | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` only declares 5 `pub mod`; `:104-106`'s only test declaration is `#[path]` to `tests/fstring.rs`. Directory-wide `grep 'mod tests'` only hits `mod.rs:105` |
| `src/frontend/pipeline/tests/` 3 files                          | 14 lines                | All placeholder documentation comments, 0 assertions                          | `src/frontend/pipeline.rs` (740 lines) has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declarations anywhere in the file (`Select-String` zero hits)                    |
| `src/package/template/tests/` 3 files                           | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`**                     | `src/package/template/mod.rs` (7 lines) only declares `mod gitignore; mod main_yx;`, no `mod tests;`                                                                          |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`     | 95 lines / **6 tests**  | Hidden in a **live** directory                                                | `pratt/tests/mod.rs` only declares `led` / `nud` / `precedence`                                                                                                               |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs`   | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` only declares `dead_code` / `overload`                                                                                                                  |
| `src/util/diagnostic/emitter/tests/json.rs`                     | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` only declares `ansi` / `text`                                                                                                                          |

**The last 3 items total 16 tests / 311 lines of code that never ran.** They are hidden in active
directories where the `mod.rs` looks complete, making them more deceptive than entire orphan
directories.

Within `lexer/tests/`, there are two more files in the "declared but not declared" form, but they
are already counted in the 629 lines of the first row above, **not double-counted**: `lexer_mod.rs`
(89 lines / 5 tests, `lexer/tests/mod.rs` declares 11 `mod` but no `mod lexer_mod;`), `symbols.rs`
(70 lines / 3 tests, same).

**Real tests that never ran total 1005 lines / 78 tests**: lexer 629/55 + pratt / passes / emitter
311/16 + package/template 65/7.

**Special note on `precedence_inline.rs`**: it tests the **production-zero-use** `Precedence` enum +
`PrecedenceContext` (96 lines of dead code) in `precedence.rs`. Even if it gets wired in, what it
tests is a dead path.

Another hidden item among the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, 0 tests): `typecheck/tests/mod.rs` declares 23 test modules but missed it specifically — only
a single `use` line remains, pure wreckage. When S1 restores it, it should be deleted together with
rewiring or supplemented with assertions.

## B. Compiled but Zero-Call Dead Code

| #   | Location                                                                         | Scale               | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | -------------------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | Repo-wide word-boundary search shows zero hits outside `instance.rs` (including tests). Source: RFC-023 declared deletion of ~850 lines, only half was done                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` + 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both re-exports). Repo-wide `TypeSystem` hits only 5 places: 2 definitions + 1 re-export + **unrelated** `lexer/symbols.rs:503 TypeSystemValidator` × 2. 4 methods have zero call sites. **Because it is `pub`, the compiler never warns**                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B3  | `src/frontend/core/typecheck/layers/dispatch.rs` entire chain                    | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` zero calls. Runtime check insertion from RFC-027 §4.1 is unwired. Note it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` is only called by tests. **The `is_subtype` in the same file must be kept** (production use by `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-describes as "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests) — and that test file itself has never been compiled** (`pratt/tests/mod.rs` only declares `led`/`nud`/`precedence`). Therefore the two **must be deleted together**: deleting only the test leaves 96 lines of dead code; deleting only the enum causes test compilation failure                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, `skip_old_function_syntax` is called at `:725` — **they are live old-syntax rejection gates, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) has only a one-line comment `// Old syntax removed`, but **the function does not consume any tokens after being called** — this is a separate behavior issue (whether the rejection path correctly advances the parse position), not dead code. **Disposition is to review behavior together with 05-frontend-paradigm.md (resolution D27: naturally deleted with grammar migration), not to delete directly as dead code**                                                                                                                                                                                              |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | Repo-wide only match, no construction point. The "no generation point (dead path)" comment at `bytecode.rs:2156` is **accurate** (encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B9  | `Switch` opcode                                                                  | —                   | `backends/common/opcode.rs` is defined, has variants, has `opcode()`/`size()` arms, has decode arms, **but repo-wide zero construction points**. The interpreter has a live implementation at `ops/control.rs:90`. **Explicitly registered as a known dead path by tests**: the `WHITELIST: &[&str] = &["SWITCH"]` at `tests/bytecode.rs:1035-1040` (fails if difference set is non-empty)                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` **ignoring the `op` field**; the decoder at `1236-1248` also only constructs `Neg`. **It silently degenerates to I64_NEG through the `opcode()` mapping path** (when `.42` runs directly, `!x` becomes `-x`; the interpreter's memory path `ops/arith.rs:46-49` is actually correct). Only used by interpreter `ops/arith.rs:46,49` and `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                                           |
| B11 | 4 places with `#[allow(dead_code)]`                                              | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (conservative exemption for extern binding, **reasonable to keep**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction** (the only construction point is the test manually building nodes at `parser/tests/ast.rs:828`), but has **12 production consumers + 2 exhaustive arms** (full list in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()`, `pratt/mod.rs:47` `expr_end_line`. Function definitions actually go through `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion assigned to P6 (16-item change list in 03 §5.5) |

## C. Empty Designs

| #   | Object                                                                          | Scale                                      | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                   | The described `CheckSession` (with Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, `HotReloader` have **zero repo-wide hits**. The "known limitations" at `:46-47` self-admits that `check_incremental` internally still calls `check_files_with_diagnostics` (full path); `:46` also references busy-wait debounce in `command.rs` — **there is neither `Instant` nor `recv_timeout` in `command.rs`** (repo-wide `recv_timeout` only appears in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                   | `:37` claims placeholder implementations in `traits/` (coherence / impl_check / object_safety / resolution) are incomplete — **the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero repo-wide hits. `:38` claims `check_single_module` creates a separate Compiler for each file — **that function does not exist**, only appearing in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                         |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                   | `:14` claims "watch mode only re-checks affected files", `:27` claims "check only does the first two steps". Actually: `check_project` also does vendor consistency, project role determination, dead code analysis, W1006; the `Check` subcommand in `main.rs` **has no `--watch` flag** (each `#[arg]` declaration has been checked one by one)                                                                                                                                                                               |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines of design / **0 lines of code** | AOT hits at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell. `:861-876` left itself an all-`- [ ]` implementation checklist, but the status is marked `accepted`                                                                                                                                                                                                                                                            |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**  | `028-jit-compiler.md:99-126` requires 6 files in `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **all of which do not exist**. Status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                                          |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                               | **Physical evidence**: the file names precisely correspond to the `ModuleCache` and incremental scheduler in the C1 design. **Someone built test skeletons according to the design, found no corresponding implementation in the pipeline, and left it**                                                                                                                                                                                                                                                                        |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                          | The `//! §6.1: CheckSession incremental check` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` refer to this **gitignored planning document** (`.gitignore` contains `superpowers/`). These § references are **archaeological references to a deprecated API**                                                                                                                                                                                                                       |

### Real Implementation Locations for Cross-File Analysis

The cross-file analysis capabilities described in C1-C3 **do exist**, just in completely different
paths:

| Component           | Location                              | Lines     |
| ------------------- | ------------------------------------- | --------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439       |
| Resolver            | `src/frontend/module/resolver.rs`     | 195       |
| Role classification | `src/frontend/module/roles.rs`        | 388       |
| Consistency check   | `src/frontend/module/consistency.rs`  | 206       |
| Orchestrator        | `src/frontend/module/orchestrator.rs` | 1528      |
| **Total**           |                                       | **~2670** |

Therefore the disposition of C1-C3 is **not to delete the capability, but to delete the misleading
documents**, and rewrite the boundary table in `check/index.md` to point to the actual architecture
in `src/frontend/module/`.

### RFC-019 Disposition Needs Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already in the `deprecated/` directory**,
and the residual check result is **clean** (`homoicon` has zero repo-wide hits, the end of the
document has an honest "if restarted in the future, start from proof/prototype experiments, not from
RFC" conclusion). **It does not need additional disposition** — this item is in the authorized list,
but verification confirmed it is already in a terminal state.

## D. Feature-Level Dead Code: REPL

`src/repl/` 4 files **1152 lines**, **zero `#[cfg(test)]`, zero `#[test]`**. And `src/main.rs:440`
is `args.command.unwrap_or(Commands::Repl)` — **running `yaoxiang` without arguments goes directly
into REPL**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D1  | **`EvalResult::Value` is never constructed** | Across all `src/repl/` it appears only 2 times: the definition at `eval.rs:23`, and a **match arm** at `mod.rs:180`. Therefore `Repl::format_value` (`mod.rs:493-502`) is dead code, **REPL does not echo any evaluation results**                                                                                                                                                         |
| D2  | **No cross-round state**                     | `extract_definitions` (`eval.rs:345-377`) only calls `define_variable` (stores **type strings**), never `define_var` (stores `RuntimeValue`); `wrap_code` also doesn't inject historical definitions. The `VariableInfo::Value` (`eval.rs:23`) branch is unreachable. `REPLContext.variables` only serves completion, not involved in evaluation                                           |
| D3  | **Wrapper syntax appears outdated**          | `eval.rs:314`/`316` produces `main() -> () = () => { code }` (old-style function declaration header `name() -> Ret`), but the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`, all the canonical tables in RFC-007:94 are `name: (a: Type) -> Ret`). **Compiler not actually run to test; if true, every REPL evaluation would fail to compile** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` only prints two lines of hints and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                              |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                  |
| D6  | Duplicates with `lib.rs`                     | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent "auto-wrap main" logics**, with different wrapping forms                                                                                                                                                                                                                                                        |

## E. Local Cleanup Items

| #   | Location                                                                                           | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                         | Root-level empty directory tree. Almost certainly some place used the string `"undefined"` as a path concatenation. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                                                |
| E2  | `src/lib.rs:568`, `:582`                                                                           | `dump_type_detail` / `dump_const_detail` each have a `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, the panic path is predictable. Should be changed to meaningful degraded output                                                                                                                                                                                                                                                                                                                                                   |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                          | `check_vendor_lock_consistency` and `check_workspace_consistency` are **~60 lines of near-duplicates**, differing only in dependency source and path-dep skipping. **Merge**                                                                                                                                                                                                                                                                                                                                                                                           |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`             | Old function syntax detection and rejection, about 120 lines. The body of `skip_old_function_syntax` (`:120-122`) is only a one-line comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                         |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                   | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                            | The "Tool Design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, `check/incremental-checking`. **If executing C1-C3 document deletion, this must be cleaned up synchronously**, otherwise 404 links appear in the documentation site                                                                                                                                                                                                                                                                                                 |
| E7  | Dual `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | 5 variants share names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` bit-widths are inconsistent (f32 × f64)**; no `From` / `TryFrom`, conversions are handwritten at scattered call sites (`const_eval.rs` / `ir_gen.rs`). `const_data.rs:44` `kind()` is a live `todo!()` for `LibraryRef` / `ExternRef`. **Disposition: merge into one + explicit conversion + `todo!()` changed to degraded output, assigned to P10 (Class E local cleanup items)**. Registered at P0 gate on 2026-10-06 (criterion A, J=0.56)                            |
| E8  | Synonymous repetition of `Literal` downstream mapping (Prohibition Three A)                        | `Literal → MonoType` same arms in ≥3 places (`inference/expressions.rs`, `checker.rs`, `Literal::Int(_) => MonoType::Int(64)` etc. in `inference/patterns.rs`); `Literal → ir::ConstValue` (19 handwritten places in `ir_gen.rs`) and `Literal → const_data::ConstValue` (4 handwritten places in `const_eval.rs`), no shared conversion point. **Disposition: lift the 3 typecheck-side places to a shared layer, assigned to P6; the `ir_gen` / `const_eval` side converges together with the `ConstValue` merge in E7 in P10**. Registered at P0 gate on 2026-10-06 |

### Not Dead Code, but Same Kind of Issue: Three Hardcoded Drops

The `impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1400 lines) has 3
hardcoded drops:

| Line   | Dropped Content                  | Consequence                                          |
| ------ | -------------------------------- | ---------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | Upvalue metadata lost                                |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifact loses exception table**             |
| `2341` | `globals: Vec::new()`            | **`.42` artifact loses global variable information** |

**These three are not dead code; they are data loss defects** — the `.42` artifact is only complete
under the in-memory path. **Disposition is different from dead code**: ~~should be tracked as a
separate issue~~ **All consolidated into P7 batch e (resolution D52, 2026-10-05), no separate issue
retained** — `2315` (exception table) and `2341` (globals) are part of language core semantics
artifact integrity, not "deferrable" items. Fix all three together, bumping `.42`'s `VERSION` from 4
to 5 in one go (the format header already has `MAGIC` + `VERSION` fields,
`codegen/bytecode.rs:14-16`). Listed here only to avoid being mistakenly cleaned up as "dead code".

## F. Not Deleting, but Marking or Deciding

| #   | Object                                                                                                                                                                                                                                                                | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (1547 lines)                                                                                                                                                                                                 | **Not dead code**, it is `include!`-inlined by `checker.rs:5618` — **the only `include!` in the entire repository**. The file **is not declared as any module** (`grep 'mod semantic_tokens'` has zero repo-wide hits), its first line is directly `impl TypeChecker {`, with no own `use` header, and the `checker/` directory only contains this one file. Consequences: no module identity, visibility isolation failure, rust-analyzer jump and symbol search failure, the toolchain counts it as part of `checker.rs`. **The real size of the `checker` module is 5618+1547 ≈ 7165 lines**. **Change: change to real `mod`** (construction steps assigned to [09](09-execution-wbs.md) §P5 5.1), not delete                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| F2  | wasm attributes (same-line `#[cfg(... wasm32 ...)]`, both positive and negative counted): current tree `src/` has **29** files containing the `wasm32` literal, **22** with actual attributes, **7** only mention; `orchestrator.rs` **10** (4 positive + 6 negative) | **Not dead code.** The wasm target is carried by a **separate shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, `:17` depends on the main crate; root `Cargo.toml:3` `exclude = ["wasm", ...]` precisely states it is a separate crate. Only looking at root `Cargo.toml:31` `crate-type = ["rlib"]` would misjudge these branches as defensive dead code — **this inference does not hold**. CI's wasm build is in `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), called by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches carry weight**: they determine which APIs of the main crate `wasm/src/lib.rs` (currently **152** lines) can call under the wasm target (`src/lib.rs:151` / `:170` / `:187` gate `run_file` / `run_project` / `build_bytecode` entirely). **Cleanup scope has been determined**: 4 wasm arms are unreachable at runtime in the playground scenario but compile-time load-bearing (call sites are not gated); 6 `not(wasm32)` arms are load-bearing in the native three-stage table ⇒ **no low-risk items that can be deleted directly** — item-by-item determination and evidence in the next section "F2 Determination" (resolution D35: delete unreachable, keep reachable, assigned to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                                                                                                                                                                                                        | Only the "Status" column (document status), **no "Implementation Status" column** — it **cannot express** the error "RFC marked as completed but the code is not". Only 5/52 RFCs repo-wide have the `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **26 accepted RFCs completely do not record implementation status**, including 009, 009a, 013, 018, 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                        | **Already disposed (4.4.1)**: the README now describes the actual call order of `checker.rs::check_module_impl` (refinement walks `predicate` → `termination` → `ownership` → type constraint solving), line numbers in parentheses taken from the current tree, positioning anchored to function names/drive points; no longer declares "if lower layer fails, upper layer does not run" (actually no short-circuit); `equivalence` is described according to its real identity as "shared type comparison utility + proof entry point only used by tests". This line is only a record of disposal, no longer pointing to the old line range                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                               | `:79` puts RFC-026a in the "Accepted RFC" table but the status column says "RFC under review"; `:48-49` "RFC under review" section only lists 032 and 027a, missing 026a. Inconsistent with the 3 reviews in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| F6  | `tools/cargo-dist/` (`dist.exe` 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                            | **None tracked by git** (`git ls-files` returns empty), excluded by `.gitignore`. **Not a problem**, only recorded to avoid subsequent audit misjudging as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                  | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` taken to the next quote), depending on the `("E1001", ...)` tuple syntax, **not a parser**. Its architecture (`parse` + `validate` + build.rs gate + `--fix` cure) is worth reusing for the opcode table, but **the extractor needs to be rewritten according to opcode form (~30 lines)**. Belongs to follow-up work of 06-cleanup-inventory.md or a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                                                                                                                                                                                           | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode fact is expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / decode match (`bytecode.rs:978-2302`) / 48 `translate_*` (`translator.rs:676-1577`). **The compiler only enforces 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + each `ops/*.rs` family function's exhaustive match), missing one of `opcode()` / `size()` / decode arm / encoder **only explodes when running the `.42` artifact**. The `size()` table's (649-803) comments are inconsistent with actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic cross-check test** (existing size tests only cover Nop/Mov/Borrow/Release 4). Operator semantics also go through a third parallel enum here (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`, `Rem`/`Xor`/`Sar` another set of naming), converging together with the opcode generation-time gate. The vocabulary target location is `middle/bytecode/opcode.rs` (migrating with the bytecode domain merge, eliminating L3→L4 reverse, see 01)                                                                                                                                                                               |

### F2 Determination: `orchestrator.rs` wasm Attribute Reachability (2026-10-18 determination, excluding cleanup)

**Determination surface** = wasm production build (the shim crate built by `wasm-pack`, see F2 row);
**determination revision** `b5e68b3c9`. This section only records the determination conclusion, does
not produce any cleanup commitment (cleanup assigned to P10 / D35).

#### 1. Count Convention (Re-taken from Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- Current tree `orchestrator.rs` has **10** wasm attributes = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- Convention = **`#[cfg(...)]` attribute text on the same line**, both positive and negative counted
  (this is the convention used by the "12" in 02); runtime macro `cfg!(...)` is not counted —
  repo-wide `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'` has zero hits.
- The "12" is not an error, it is **the fact of an old revision**: `544695ea9` (the first commit of
  the RFC-039 design document) had 5 positive + 7 negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- The difference source is **a pair of attributes migrated as a whole**: `6ec330d22` (WBS 4.2.2,
  `check_project` slimmed down to a `Program` constructor, this commit net-deleted 252 lines from
  `orchestrator.rs`) moved `role_context` together with its `not(wasm32)` implementation and
  `wasm32` stub **to** `src/driver/helpers.rs:360/394` — `orchestrator.rs` therefore went from 12 to
  10, the mechanism itself did not disappear.
- Revision-by-revision re-check (actually executed this time; excerpt below, `6ec330d22` is the
  old/new boundary):

```console
$ for rev in $(git log --format=%h -20 -- src/frontend/module/orchestrator.rs); do
    pos=$(git show $rev:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]')
    neg=$(git show $rev:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]')
    echo "$rev pos=$pos neg=$neg total=$((pos+neg))"
  done
… b8b3cc3f6 pos=4 neg=6 total=10   # current HEAD side
… 6ec330d22 pos=4 neg=6 total=10
… 5ba52eafb pos=5 neg=7 total=12   # earlier
```

#### 2. Three-Question Convention (Without Unbundling, Load-Bearing Arms Will Be Misjudged as Dead Code)

1. **Compile-time existence** — whether the arm participates in compilation under that target;
2. **Runtime reachability** — whether there is a call chain from the playground entry under that
   target;
3. **Deletion cost** — whether deleting it would break compilation of that target (whether call
   sites are similarly gated).

#### 3. Determination Starting Point (Playground Entry and Stage Table)

- Entry: `wasm/src/lib.rs` (currently 152 lines) `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, only constructs `Pipeline`, does not touch
  orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` takes `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`; `:135` has
  `SingleFile | WasmPlayground` in the same table): Parsing / Typecheck / DeadCodeAnalysis /
  ProofExecution / IrGeneration / Monomorphization — **does not include** VendorConsistency /
  Discovery / Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### 4. Item-by-Item Determination (Current Tree 10 Items)

| #   | Location (`orchestrator.rs`) | Gated Object                                                           | Compiles in wasm build                  | Playground runtime reachable                                                                                                                                                 | Deletion Cost (Whether Call Site is Gated)                                        |
| --- | ---------------------------- | ---------------------------------------------------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| 1   | `:271` positive              | wasm arm of `ensure_vendor_consistency` (`:267`) (`Ok(None)`)          | Yes                                     | **No**: the only call site `src/driver/arms.rs:1195` (VendorConsistency arm) is not in the playground stage table                                                            | High: `arms.rs:1195` has no gate, deleting the arm alone ⇒ wasm compilation fails |
| 2   | `:418` positive              | wasm arm of `discover_with_used` (`:400`) (lock_versions set to empty) | Yes                                     | **No**: call site `arms.rs:1221` (Discovery arm), `src/driver/helpers.rs:686` (Check form reference pool `collect_package_refs`, only called by `arms.rs:521` Check branch)  | High: same as above                                                               |
| 3   | `:655` positive              | wasm stub of `member_context` (`:656-659`)                             | Yes                                     | **No**: the only call site `:404` (inside #2's function body)                                                                                                                | High: `:404` has no gate                                                          |
| 4   | `:753` positive              | wasm stub of `dep_import_surface` (`:754-756`)                         | Yes                                     | **No**: call sites `:671` (`resolve_in_package_root`) and `:813` (`resolve_in_vendor_with_lock`), rooted in `:468` `resolve_module_path` (inside `discover_with_used`'s BFS) | High: same as above                                                               |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                       | No (does not participate in wasm build) | N/A                                                                                                                                                                          | Native load-bearing: used by `:678` / `:747`                                      |
| 6   | `:276` negative              | native arm of `ensure_vendor_consistency`                              | No                                      | N/A                                                                                                                                                                          | Native load-bearing: VendorConsistency is in `program.rs:61/98/113` three tables  |
| 7   | `:413` negative              | native arm of `discover_with_used` (lock reading)                      | No                                      | N/A                                                                                                                                                                          | Native load-bearing: Discovery is in `program.rs:62/99/114`                       |
| 8   | `:607` negative              | native `member_context` (workspace resolution)                         | No                                      | N/A                                                                                                                                                                          | Native load-bearing: call site `:404`                                             |
| 9   | `:677` negative              | manifest name reading of `resolve_in_package_root`                     | No                                      | N/A                                                                                                                                                                          | Native load-bearing: `:544/:547` ← `:468`                                         |
| 10  | `:745` negative              | native `dep_import_surface`                                            | No                                      | N/A                                                                                                                                                                          | Native load-bearing: `:671` / `:813`                                              |

#### 5. Conclusion

1. **4 wasm arms are unreachable at runtime in the playground scenario** (#1-#4, evidence = stage
   table does not include the stage where the only call site is located), but they are
   **compile-time load-bearing**. Executing D35 literally "delete unreachable ones" would cause
   compilation to fail — the feasible form is "gate the entire project-mode subtree
   (`orchestrator.rs`'s discovery/resolution functions + driver's VendorConsistency/Discovery two
   arms + reference pool collection in `helpers.rs`) on wasm32", with cost and benefits to be
   determined by P10.
2. **6 `not(wasm32)` arms do not exist in the playground build** (compile-time excluded), native
   side is all load-bearing (MultiFile / Check / Lsp three-stage table). "Playground unreachable ⇒
   delete" **does not apply** to them: deleting by this standard would directly break CLI/LSP.
3. ⇒ This directory has **no "directly deletable and low-risk" items**; F2's cleanup benefit only
   falls on "whether the wasm artifact becomes smaller as a result" — that item has not been
   verified (see below).

#### 6. Unverified (Not to Be Used as Conclusion)

- **wasm artifact size benefit not measured**: did not run `wasm-pack build` and size/symbol table
  tools (`twiggy` / `wasm-objdump`), "how much smaller the artifact would be after gating the
  project-mode subtree" has no data; whether unreferenced `pub(crate)` items are stripped at link
  time is also unverified.
- **Non-shim wasm32 consumers not exhaustively enumerated**: this round only checked the
  repo-internal CI (the only wasm32 compilation is in `_build-wasm.yml:30/77`, via the shim crate).
  If a third party compiles the main crate directly to `wasm32-unknown-unknown` and calls
  project-mode APIs, the reachable surface will exceed this determination.
- **Native-side "load-bearing" = static call chain + stage table**, not verified end-to-end one by
  one (did not run MultiFile/Check/Lsp probes to touch each arm one by one).

#### 7. Appendix: Repo-Level Count Re-check (Same Revision)

```console
$ git grep -l wasm32 b5e68b3c9 -- '*.rs' | wc -l
30        # includes wasm/ shim crate
$ git grep -l wasm32 b5e68b3c9 -- 'src/*.rs' | wc -l
29
$ git grep -lE '#\[cfg\([^]]*wasm32' b5e68b3c9 -- 'src/*.rs' | wc -l
22        # with actual cfg attributes
$ git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs' | wc -l
0         # no runtime cfg! convention
```

Same convention as 02's "wasm current state" and F2 row: current tree **29** files contain the
`wasm32` literal / **22** with actual `#[cfg(...)]` attributes / **7** only mentioned in
documentation; `orchestrator.rs` **16** occurrences = **10** attributes (4 positive + 6 negative) +
6 comments (the 7 only mentioned: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Criterion Discovery Registration (Registered with Each Phase Construction from 2026-10-06)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibitions One A/B/C/D) first
ran on 2026-10-06. According to the division of labor where "the gate only reports suspicions, and
humans make decisions" ([08](08-maintenance-mechanism.md) §Known Limitations), this section records
the decision results and disposition attribution. Unexpanded hits (criterion A violation layer 25
pairs, criterion B all suspicion items) are checked pair by pair during the corresponding phase
construction.

| #   | Object                                                                                                                           | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Disposition Attribution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                                    | **Legitimate structure, not parallel representation**: value / type label pair, maintained 1:1 by exhaustive `value_type()` (`value.rs:326-369`), missing arm causes compilation failure. But the external consumption of `ValueType` effectively only remains in the `{:?}` formatting of std/os/fs/net error paths and the constant `Void` at `executor.rs:410`, 21 variants far exceed actual need                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | P10 S5 slimming candidate: reduce to `pub(crate)` and evaluate pruning; during S5 construction, also decide the necessity of the `AsyncValue.value_type` field (`value.rs:217`)                                                                                                                                                                                                                                                                                                                                                                                                                        |
| G2  | Dual `ConstValue` (criterion A, J=0.56)                                                                                          | **True positive**, including f32/f64 bit-width inconsistency and `kind()` live `todo!()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Already in E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| G3  | 11 places of string dispatch on type names (criterion D true positive)                                                           | **True positive, same root cause**: built-in container names have no single authoritative identification point. List: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), size table at `eval/const_eval.rs:498-511` (including `"Uint"` dead arm), `to_type_id` at `middle/passes/codegen/bytecode.rs:744-756` (`"Set" => 25` contradicts decision 4 of #300), `"Clone" \| "Dup"` at `trait_data.rs:187`                                                                                                                                                                                                                                               | **P6 extended verification item**: these points are on the typecheck / eval / middle side, not within the parser data flow coverage of 03 §5.4; during P6 construction, follow the same principle as 03 §5.3 (`NameKind` decided once, match arms no longer do string comparison) for joint disposition. The duplicate table has been synchronously corrected to seven places (03 §2.3)                                                                                                                                                                                                                |
| G4  | Criterion D 4 false positives                                                                                                    | **Legitimate lookup / dispatch table, not bridging**: tool installation guidance at `package/build/requirements.rs:17-25`, build result display name at `package/vendor/mod.rs:134-139`, REPL command aliases at `repl/mod.rs:241-396` (8 arms, user interface aliases are legitimate UX), `parse_runtime_mode` at `util/diagnostic/mod.rs:462-468` (with annotation of historical aliases, single boundary point)                                                                                                                                                                                                                                                                                                                                                                                                                                                    | P9 hardening adds `// reason:` exemption one by one; exemption count goes through gate report review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| G5  | `ReleasePlan.drops` span key mismatch (P2 2.4.4 criterion actual measurement, not P0 hit)                                        | **True positive, contract defect — D20 prediction proven**: ownership records keys by "last used **expression** span" (`ownership.rs:2574 build_release_plan`), `ir_gen.rs:1956` looks up the table by "**statement** span", expression-level keys never hit — corpus measurement shows **168 files / 373 keys** are not consumed, Drops silently lost (per-case evidence in `src/middle/core/tests/release_plan_spans.rs` file header)                                                                                                                                                                                                                                                                                                                                                                                                                               | **P4/D20** (Span key → PlanId, behavior surface ~168 files, must wait for C2 criterion and corpus diff to be in place); criterion `test_release_plan_spans_consumed` is permanently red-state (`#[ignore]`), turns green when D20 lands and is removed                                                                                                                                                                                                                                                                                                                                                 |
| G6  | IR metadata three defects (P2 2.1.2 criterion `verify_loose` corpus running green process measurement, D38 prediction fulfilled) | **True positive ×3, all fixed with 2.1** (corpus diff baseline self-compare zero diff proves behavioral equivalence): ① The locals table of method IR is truncated to parameter count (ir_gen.rs original `take_cur_locals(param_types.len())`) — local_count underestimated, runtime relies on `Frame::set_slot` out-of-bounds resize (`frames.rs:103-105`) silent fault tolerance; ② Unannotated parameters of lambda / curry middle layer / function path are dropped by `filter_map` (three places), params length and frame layout mismatch — aligned with the existing method path convention to fill `Int(64)` placeholder; ③ match result slot depends on frame's implicit Void pre-initialization (the fault tolerance landing point of the last arm's guard failure has an undefined path on CFG) — `generate_match_expr_ir` entry explicitly pre-sets Void | Already fixed (same batch as 2.1). **Determination surface narrowing record** (verify.rs file header is authoritative): temp slot dominance not checked (Void pre-initialization is frame semantics, ir_gen intentionally relies on it); the trustworthy source of type consistency narrows to parameter slot signatures + constants (ir_gen never writes `LocalSlot.ty`, the whole table placeholders `Int(64)` — complete determination surface assigned to P7 value table 04 §262); `init_locals` is the #368 debug view table, not the allocation limit, init's Local out-of-bounds is not checked |

## H. Giant Function Inventory (Pre-Release Audit Convention, from 2026-10-16)

> Convention (2026-10-16 `/pre-release-audit` set): function span > 150 lines is the **initial
> screening threshold**, not a "report each one" command — listing all would turn the inventory into
> a line count pile, drowning the new signals of this review in stock. Report entries must include
> three numbers: **host line count / lines inserted this time / line count before insertion**. Two
> categories: **A. Existing giant functions with inserted branches** (>150 lines and >50 lines
> inserted this time); **B. New large functions this time** (almost entirely new and >100 lines).
> The line numbers below are from audit revision `1935aab53` (range `v0.8.4..1935aab53`).

### A. Existing Giant Functions with Inserted Branches

| Function                | Location                                                    | Host Line Count | Inserted This Time | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | --------------- | ------------------ | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1594            | 358                | 1236             |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813             | 171                | 642              |

### B. New Large Functions This Time

| Function                     | Location                                            | Line Count | New This Time |
| ---------------------------- | --------------------------------------------------- | ---------- | ------------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180        | 177           |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122        | 122           |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159        | 130           |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130        | 107           |

**Existing giant functions not in the inventory** (inserted ≤ 4 lines this time, recorded but not
disposed): `infer_expr` (`expressions.rs:2999`, 955 lines), `collect_function_signature`
(`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`, 441 lines),
`generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), etc.

### Disposition Direction

- **Class A**: dissolved together with the subsequent split tasks of `infer_call_expr` /
  `generate_call_expr_ir` — these two are the heaviest hosts in the repository, the named parameter
  refactoring this time inserted 358 lines into the former.
- **Class B**: `push_joined` is a **local** helper for `Task` work-list style type formatting (its
  host is the 180 lines of new content), with `mono.rs:123 increment_level_string` (174 lines, does
  level name rewriting) **different responsibilities, not duplicates, do not merge**;
  `consumer_stage_skipped` and `local_count_of` can be split independently.

## Key Decisions and Reasons

The authorization is "thorough deletion permitted", but this document gives **different
recommendations** for several items:

| Category                                                                | Disposition                                   | Reason                                                                                                                                                                                                                                                                                                                      |
| ----------------------------------------------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions, no implementation) | **Delete**                                    | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty function), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                                |
| **Empty design documents**                                              | **Delete or rewrite**                         | C1, C2 delete then rewrite to point to actual implementation; C3 rewrite boundary table; **must synchronously clean up `config.js:259-268`** (E6)                                                                                                                                                                           |
| **RFCs with mismatched status and implementation**                      | **Change status**                             | C4 (RFC-018) move back to `draft` or add `impl_status: 'not-started'`; C5 (RFC-028) add `impl_status: 'not-started'`. **Do not delete** — the design itself may be valuable, the problem is status marking distortion                                                                                                       |
| **Orphan tests with real assertions**                                   | **Restore, not delete**                       | Total **1005 lines / 78 tests**: lexer 629 lines / 55 + 311 lines / 16 hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7. **Restoration cost is one line of `mod` declaration, value is letting CI regain this coverage**. Deleting them is a net loss |
| **Zero-call but semantically important `pub` items**                    | **Reduce visibility, not delete**             | B2 (`TypeSystem`), B4 (`check_type_equivalence`) — move to `pub(crate)` to let the compiler identify, or add implementation. **B4's `is_subtype` must be kept**                                                                                                                                                             |
| **Opcodes / instructions that cannot produce values**                   | **Add construction points or delete opcodes** | B8 (`TailCall`), B9 (`Switch`, opcode round-trip test is explicitly registered with `WHITELIST=["SWITCH"]`), B10 (`UnaryOp::Not` silent degradation). Choose one of the three, cannot maintain status quo; D32/D33/D34 have ruled to delete, processed together with `.42` version bump                                     |
| **Data loss defects**                                                   | ~~Separate issue~~ **P7 batch e (D52)**       | Three hardcoded drops in section E — not dead code                                                                                                                                                                                                                                                                          |
| **Effective but no module identity**                                    | **Refactor, not delete**                      | F1 (`semantic_tokens.rs`) change `include!` → real `mod` (see `02`)                                                                                                                                                                                                                                                         |
| **Requires decision**                                                   | **List as open questions**                    | F2's wasm32 cleanup has been registered as [09](09-execution-wbs.md) §P10 **S7** (wasm32 gates project-mode subtree, 2026-10-18 user decision B; including three unverified items preserved as-is); layer order correction will expose new diagnostics                                                                      |

### Directions Not Adopted

- **All handled by "thorough deletion", orphan tests also deleted** — 1005 lines of real-assertion
  tests are an asset. Deletion cost is one line of `mod` declaration, benefit is CI regaining
  coverage. The authorization is "may delete", not "should delete".
- **Only add gate, do not touch code** — gates can prevent regression, but B1's 415 lines, the
  dispatch chain, the 96 lines of dead ladder **already exist**, the gate will only freeze them at
  baseline.
- **Keep RFC-018 / RFC-028 `accepted` status, only supplement documentation** — `rfc/index.md:131`
  defines "accepted = enter implementation stage", readers will reasonably assume there is an
  implementation. `TRACKING.md` has a precedent for the `impl_status` field (5 RFCs in use), the
  cost of supplementing the field is far less than the cost of status distortion.

## Implementation Points

**This document does not need an equivalence criterion** (belongs to
[C6 pure deletion category](07-equivalence-oracle.md)), but **still needs to confirm no references
item by item**. In RFC-039's global stage sequence, S1 corresponds to P1, S2-S6 correspond to P10.

| Stage | Content                                                                                                                                          | Acceptance                                                                                                                                                                                                                                        |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Restore orphan tests with assertions (**1005 lines / 78 tests**)                                                                                 | Test count in lexer / pratt / passes / emitter / template five modules rises in `cargo test`; **expected to expose real defects** (overflow paths in `literals.rs`, `\x`/`\u` illegal escape paths never tested before), need to reserve fix time |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 with 05 closure/C6/E1)                                                                            | `cargo test` passes; `cargo clippy -D warnings` has no new warnings                                                                                                                                                                               |
| S3    | Empty design document disposition (C1-C3 + E6 synchronously clean up `config.js`)                                                                | Doc site builds without 404; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                              |
| S4    | RFC status correction (C4/C5/F3 — add "Implementation Status" column to `TRACKING.md`)                                                           | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` is automatically regenerated, **do not manually edit**)                                                                                                                              |
| S5    | Reduce `pub` item visibility (B2/B4)                                                                                                             | Compiler can point out real dead code                                                                                                                                                                                                             |
| S6    | Opcode cannot produce value decision landing (B8/B9/B10/F8, executed per D32/D33/D34)                                                            | Delete `Switch` / `TailCall` dead paths; B10's `opcode()` distinguishes `op` field; F8 gate lands                                                                                                                                                 |
| S7    | `include!` refactor (F1, construction steps assigned to [09](09-execution-wbs.md) §P5 5.1) + layer order correction (F4, assigned to P4's 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                        |

**S1 must be done first, and defect fix time must be reserved.** Its value is not "cleanup", but
**letting CI regain 1005 lines / 78 tests that never ran** — before this, any judgment about test
coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners in `literals.rs` have 4 structurally
  identical "continue consuming after overflow without erroring" branches, the entire
  `scan_leading_dot` function, the `\x`/`\u` illegal escape paths, all have never been tested. After
  restoring tests, the code will most likely need to be fixed, which will delay subsequent
  refactoring.
- **B9/B10 involve language behavior decisions**. Whether `Switch` and `UnaryOp::Not` should "add
  construction points" or "delete opcodes" needs to consider whether they belong to the surface that
  released corpus depends on.
- **After C1-C3 deletion, the capability documentation for cross-file analysis will be briefly
  absent**, until `check/cross-file-analysis.md` is rewritten to point to the actual architecture in
  `src/frontend/module/`.
- **Reachability analysis can only find "unreachable", not "reachable but meaningless"**. Judgments
  beyond Form 2 must be done manually item by item, and **this inventory does not constitute an
  exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  will not be automatically discovered by the compiler, only by search.
- **Deleting the half-finished residue of RFC-023** (B1) can fulfill the decision "closures have
  been removed from the language"; **correcting status distortion** (C4/C5/F3) can restore
  `TRACKING.md`'s reference value.

> **The open questions originally listed in this section have all been decided.** Item-by-item
> decisions are in [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document retains no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  Source decision for B1 residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — B3's unwired `§4.1` runtime check insertion
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — Official carrier of C1/C6
- [RFC-039 Compiler Feature Routing Directory Design](../../rfc/accepted/039-compiler-architecture.md)
  — Top-level outline
- [Refactoring Equivalence Criterion](07-equivalence-oracle.md) — Criterion requirements for
  category C6
- `src/frontend/core/typecheck/layers/README.md` — Actual call order of the proof layer and
  `equivalence`'s real identity (F4 disposed)
- `build.rs:19-55` — Example of the generation-time gate in this repository
