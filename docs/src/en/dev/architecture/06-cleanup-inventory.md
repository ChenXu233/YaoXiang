# Dead Code and Paper-Only Design Cleanup

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main text of RFC-039 for the four-layer model, tiered acceptance criteria, and execution-stage
> order; see the [directory index](index.md) for the role of each companion document.

## Positioning and Scope

This document provides an inventory of dead code and paper-only designs in the YaoXiang repository,
the method used to identify them, item-by-item disposition decisions, and the execution order. There
are four categories of cleanup targets:

| Category                                                                      | Size                                                                |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Files that have never participated in compilation                             | 24 files / 2,628 lines (of which 23 are true orphans / 1,081 lines) |
| Dead code that compiles but has zero call sites                               | 1,500+ lines                                                        |
| Design documents describing an architecture that never existed                | Three documents under `docs/src/dev/design/check/`                  |
| Features attached to the default entry point, with zero tests and zero output | 1,152 lines under `src/repl/`                                       |

**The authorized scope is “outright deletion is permitted”** (decision dated 2026-10-03). However,
this document recommends dispositions for several items that are **opposite to “delete
directly”**—see [Key Decisions and Rationale](#关键决策与理由).

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), or oracle design (see `07-equivalence-oracle.md`). B3
records that the entire `layers/dispatch.rs` chain is unwired; its deletion belongs to S2.

## Current State

### Three Forms of Dead-Code Debt

**Form 1: Placeholder.** The code exists, appears to be doing something, but does nothing.

- The entire `src/frontend/core/typecheck/layers/dispatch.rs` chain (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the “runtime check insertion” mechanism designed by `RFC-027 §4.1` and is
  **completely unwired**.
- `check_type_equivalence` (`ProofContext` entry point) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is called only by tests. **Note that
  `is_subtype` in the same file is used in production by `inference/assignment.rs:17`, so the file
  must not be deleted in its entirety**.
- `src/backends/runtime/engine.rs` (1,047 lines) is a pure DAG task scheduler. Its `pub` surface
  consists only of `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend**—the
  `Executor` trait at `backends/mod.rs:356` currently has only one implementation (the interpreter).
  This point needs clarification because the directory name is misleading.

**Form 2: Residue.** The upstream decision has been made, but the deletion was only half completed.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) contains six residual types:
  `FunctionInstance` (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), and `CaptureVariable`. **The source is clear**: when `RFC-023`
  deprecated the “closure capture model,” it declared that “**roughly 850 lines of code would be
  deleted along with the closure model**.” These 415 lines are the surviving remainder of those 850
  lines—**the deletion was only half completed**. **Verification**: a repository-wide word-boundary
  search for these six types finds **zero hits outside `instance.rs`** (including tests).
  `pub use passes::mono::instance::*;` at `middle/mod.rs:40` merely forwards the glob re-export and
  does not constitute consumption.

**Form 3: False Coverage.** This is the most dangerous form—**it makes the repository appear more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only five `pub mod` declarations (`literals` / `state` /
`symbols` / `tokenizer` / `tokens`) plus one
`#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;` (`:104-106`). **Nowhere is
`mod tests;` declared**, so the entire 13-file, 686-line `lexer/tests/` subtree **has never
participated in compilation**.

Seven of these files are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
`delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totaling 19 lines and zero
assertions). **629 lines / 55 real tests have never run**:

| File              | Lines   | `#[test]` |
| ----------------- | ------- | --------- |
| `literals.rs`     | 222     | 30        |
| `rfc010_lexer.rs` | 167     | 12        |
| `lexer_mod.rs`    | 89      | 5         |
| `rfc004_lexer.rs` | 81      | 5         |
| `symbols.rs`      | 70      | 3         |
| **Total**         | **629** | **55**    |

(`fstring.rs`, 90 lines / 11 tests, does compile, but through the `#[path]` bypass rather than
`mod tests;`; `mod.rs`, 38 lines, is a declaration file with no assertions.)

**Only one lexer file with 12 tests is actually running, and all of them test only the raw-brace
protocol of f-strings.**

The danger of this problem lies in the fact that `cargo test` will not fail—tests that never ran
cannot fail—and CI has no test-count baseline or wiring check.

### Paper-Only Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not a case of “the design was completed but not implemented”; rather, **the
documents describe a different system that never existed**, and present “a possible future design”
as “a defect in an implemented system.”

## Methodology and Confidence Statement

### Reachability Analysis Method

Starting from the two crate roots, `src/lib.rs` and `src/main.rs`, the method parses the `mod` /
`pub mod` / `#[path]` declarations in all 509 `.rs` files, constructs a module graph, and performs
BFS reachability analysis. `include!` targets are assessed separately and are not counted as
reachable.

**This method has known false-positive patterns, two of which were actually encountered while
writing this document:**

1. `Split-Path` returns `\` on Windows, which does not match keys using forward slashes → this
   briefly produced a false report of 499 orphan files.
2. **For a non-`mod.rs` file, a submodule directory is `its own name/`, not the
   `parent directory/`** → `mod tests` in `orchestrator.rs` was resolved to
   `src/frontend/module/tests/`, nearly classifying 753 lines of live tests as dead code.

Therefore, **every conclusion was confirmed twice: by actually reading the file and actually
searching for references**. Items that failed the second confirmation were not included in this
document.

### Line-Count Convention

All line counts in this document were obtained with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines and systematically undercounts by about 5%** (for example,
`manifesto.md` is reported as 658 lines with `-Line`, but actually has 893). An earlier round of
analysis in this repository produced an artificially low total because of this convention; the
figure has been corrected here.

### Known Limitations

- **Reachability analysis can only identify “unreachable” code; it cannot identify “reachable but
  meaningless” code.** Forms other than Form 2 require item-by-item manual judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types must be
  discovered through searches, which this method does not cover.
- This document does not run the compiler, so every “has never run” conclusion is a static
  determination.

## A. Files That Have Never Participated in Compilation

**Total: 24 files / 2,628 lines.** `semantic_tokens.rs` belongs to Form 4 (see Section F); the
remaining **23 files / 1,081 lines are true orphans**.

| Path                                                          | Size                    | Contents                                                                      | Basis for Determination                                                                                                                                                                                  |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entire `src/frontend/core/lexer/tests/` subtree, 13 files     | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only five `pub mod`s; `:104-106` is the only test declaration and uses `#[path]` to reach `tests/fstring.rs`. A repository-wide `grep 'mod tests'` matches only `mod.rs:105` |
| 3 files under `src/frontend/pipeline/tests/`                  | 14 lines                | All are placeholder documentation comments, with zero assertions              | The entire 740-line `src/frontend/pipeline.rs` has no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration (`Select-String` has zero matches)                                                      |
| 3 files under `src/package/template/tests/`                   | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`s**                    | The 7-line `src/package/template/mod.rs` declares only `mod gitignore; mod main_yx;`, with no `mod tests;`                                                                                               |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden inside a **live** directory                                            | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                                          |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                             |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                                     |

**The last three items total 16 tests / 311 lines of code that have never run.** They are hidden
inside active directories, and their `mod.rs` files look complete, making them more deceptive than
an entire orphaned directory.

Two other files under `lexer/tests/` belong to the same “declared but missing declaration” form, but
are already included in the 629 lines in the first row and are **not counted twice**: `lexer_mod.rs`
(89 lines / 5 tests; `lexer/tests/mod.rs` declares 11 `mod`s but omits `mod lexer_mod;`) and
`symbols.rs` (70 lines / 3 tests, likewise).

**Total never-run real tests: 1,005 lines / 78 tests**: lexer 629/55 + pratt / passes / emitter
311/16 + package/template 65/7.

**Pay special attention to `precedence_inline.rs`**: it tests the production-zero-use `Precedence`
enum and `PrecedenceContext` in `precedence.rs` (96 lines of dead code). Even if it were wired up,
it would still be testing a dead path.

Another item hidden among the 23 orphans is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, zero tests): `typecheck/tests/mod.rs` declares 23 test modules but omits this one. The file
contains only a `use` statement and is a pure remnant. When S1 revives the tests, delete it
alongside the wiring or add assertions.

## B. Dead Code That Compiles but Has Zero Call Sites

| #   | Location                                                                         | Size                | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | -------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | A repository-wide word-boundary search finds zero hits outside `instance.rs` (including tests). Source: RFC-023 declared the deletion of roughly 850 lines, but only half was removed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` plus four pure functions. The only reference chain is `pub mod types;` at `inference/mod.rs:11` and `pub use types::TypeSystem;` at `:24` (both merely forward it). Repository-wide, `TypeSystem` has only five hits: two definitions, one re-export, and two occurrences of the **unrelated** `lexer/symbols.rs:503 TypeSystemValidator`. The four methods have zero call sites. **Because they are `pub`, the compiler never reports them**                                                                                                                                                                                                                                                                                                                                                                                                   |
| B3  | Entire `src/frontend/core/typecheck/layers/dispatch.rs` chain                    | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` have zero call sites. The runtime check insertion specified by RFC-027 §4.1 is unwired. Note that it is declared at `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` is called only by tests. **`is_subtype` in the same file must be preserved** (used in production by `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Self-described as a “framework placeholder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | `Precedence` enum + `PrecedenceContext`. **Its only external reference is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests), and that test file has itself never been compiled** (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore, **the two must be deleted together**: deleting only the test leaves 96 lines of dead code, while deleting only the enum causes the test to fail to compile                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, and `skip_old_function_syntax` is called at `:725`—**they are active old-syntax rejection gates, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) consists of only the comment `// Old syntax has been removed`, but **the function consumes no tokens after it is called**—this is an independent behavioral issue (whether the rejection path correctly advances the parser position), not dead code. **Disposition: review the behavior together with 05-frontend-paradigm.md (Decision D27: remove it naturally with the grammar migration), rather than deleting it directly as dead code**                                                                                                                                                                              |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | The repository contains only match arms, with no construction site. The “no construction site (dead path)” comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B9  | `Switch` opcode                                                                  | —                   | It is defined in `backends/common/opcode.rs`, has a variant, has `opcode()`/`size()` arms, and has a decoding arm, **but has zero construction sites in the repository**. The interpreter has an active implementation at `ops/control.rs:90`. **It is explicitly registered as a known dead path by tests**: `tests/bytecode.rs:1035-1040` defines `WHITELIST: &[&str] = &["SWITCH"]` (the check fails if the difference is non-empty)                                                                                                                                                                                                                                                                                                                                                                                                                      |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                   | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG` and **ignores the `op` field**; decoder `:1236-1248` likewise constructs only `Neg`. **Through the `opcode()` mapping path, it silently degenerates to I64_NEG** (when running a `.42` artifact directly, `!x` becomes `-x`; by contrast, the interpreter’s in-memory path at `ops/arith.rs:46-49` is correct). It is used only by the interpreter at `ops/arith.rs:46,49` and by `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                        |
| B11 | Four `#[allow(dead_code)]` declarations                                          | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (a conservative exemption for an extern binding and **reasonable to retain**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction sites** (the only construction site is a test manually creating a node at `parser/tests/ast.rs:828`), but there are **12 production consumers + 2 exhaustive arms** (full list in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()` and `pratt/mod.rs:47` `expr_end_line`. Actual function definitions use `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion belongs to P6 (the 16-item change list in 03 §5.5) |

## C. Paper-Only Designs

| #   | Target                                                                          | Size                                        | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                    | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, and `HotReloader` have **zero repository-wide hits**. The “known limitations” at `:46-47` admit that `check_incremental` still calls `check_files_with_diagnostics` internally (the full path); `:46` also references the busy-wait debounce in `command.rs`—**`command.rs` contains neither `Instant` nor `recv_timeout`** (the only repository-wide occurrence of `recv_timeout` is in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                    | `:37` claims that the placeholder implementations under `traits/` (coherence / impl_check / object_safety / resolution) are incomplete—**the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero repository-wide hits. `:37` claims that `check_single_module` creates an independent Compiler for each file—**the function does not exist** and appears only in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                                     |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                    | `:14` claims that “watch mode rechecks only affected files,” and `:27` claims that “check performs only the first two steps.” In reality, `check_project` also performs vendor consistency checking, project-role classification, dead-code analysis, and W1006; the `Check` subcommand in `main.rs` has **no `--watch` flag** (all `#[arg]` declarations were checked individually)                                                                                                                                                                               |
| C4  | `RFC-018` LLVM AOT                                                              | 1,037 lines of design / **0 lines of code** | The AOT hits at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` contains no cranelift / llvm / inkwell. `:861-876` contains an implementation checklist with every item marked `- [ ]`, yet the status is `accepted`                                                                                                                                                                                                                                                                             |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**   | `028-jit-compiler.md:99-126` requires six files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`; **none exist**. The status is `draft` (more honest)                                                                                                                                                                                                                                                                                                                                                                                                   |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                                | **Physical evidence**: the file names precisely match the `ModuleCache` and incremental scheduler from the C1 design. **Someone created test skeletons according to the design, discovered that the pipeline had no corresponding implementation, and left them unfinished**                                                                                                                                                                                                                                                                                       |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                           | The `//! §6.1: CheckSession incremental checking` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` point to this **gitignored plan document** (`.gitignore` includes `superpowers/`). These section references are **archaeological references to deprecated APIs**                                                                                                                                                                                                                                                      |

### The Actual Location of Cross-File Analysis

The cross-file analysis capabilities described in C1-C3 **do exist**, but at entirely different
paths:

| Component           | Location                              | Lines           |
| ------------------- | ------------------------------------- | --------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439             |
| Resolver            | `src/frontend/module/resolver.rs`     | 195             |
| Role classification | `src/frontend/module/roles.rs`        | 388             |
| Consistency checks  | `src/frontend/module/consistency.rs`  | 206             |
| Orchestration       | `src/frontend/module/orchestrator.rs` | 1,528           |
| **Total**           |                                       | **About 2,670** |

Therefore, the disposition for C1-C3 is **not to delete the capability, but to delete misleading
documents** and rewrite the boundary table in `check/index.md` to point to the actual architecture
under `src/frontend/module/`.

### RFC-019 Requires Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already under `deprecated/`**, and the
residue check is **clean** (`homoicon` has zero repository-wide hits; the end of the document
contains the honest conclusion, “If restarted in the future, begin with proof/prototype experiments
rather than with an RFC”). **It requires no additional disposition**—it appears in the authorized
inventory, but verification confirmed that it is already in its terminal state.

## D. Feature-Level Dead Code: REPL

`src/repl/` contains four files totaling **1,152 lines**, with **zero `#[cfg(test)]` and zero
`#[test]`**. Meanwhile, `src/main.rs:440` specifies
`args.command.unwrap_or(Commands::Repl)`—**executing `yaoxiang` without arguments enters the REPL
directly**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed** | It appears only twice under `src/repl/`: at the definition in `eval.rs:23` and in a **match arm** in `mod.rs:180`. Therefore, `Repl::format_value` (`mod.rs:493-502`) is dead code, and **the REPL never echoes evaluation results**                                                                                                                                                           |
| D2  | **No state persists across turns**           | `extract_definitions` (`eval.rs:345-377`) calls only `define_variable` (which stores a **type string**) and never calls `define_var` (which stores a `RuntimeValue`); `wrap_code` also does not inject historical definitions. The `VariableInfo::Value` branch (`eval.rs:23`) is unreachable. `REPLContext.variables` exists solely for completion and does not participate in evaluation     |
| D3  | **The wrapper syntax appears obsolete**      | `eval.rs:314`/`316` produce `main() -> () = () => { code }` (the old-style function-declaration header `name() -> Ret`), whereas the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`; the canonical table in RFC-007:94 uses `name: (a: Type) -> Ret`). **The compiler was not run to verify this; if true, every REPL evaluation fails to compile** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` prints two lines and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                                                |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                      |
| D6  | Duplicates `lib.rs`                          | `eval_code` at `lib.rs:85` and `wrap_code` in `repl/eval.rs` are **two independent implementations of “automatically wrap main”**, and their wrapper forms differ                                                                                                                                                                                                                              |

## E. Local Cleanup Items

| #   | Location                                                                                                | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                              | Empty directory tree at the root. It is almost certain that the string `"undefined"` was mistakenly concatenated into a path somewhere. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| E2  | `src/lib.rs:568`, `:582`                                                                                | `dump_type_detail` / `dump_const_detail` each have an `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` are FFI-related, and the panic paths are predictable. Replace them with meaningful fallback output                                                                                                                                                                                                                                                                                                                                                                                              |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                               | `check_vendor_lock_consistency` and `check_workspace_consistency` are **approximately 60 lines of near-duplicate code**, differing only in dependency source and the two path-dependency skips. **Merge them**                                                                                                                                                                                                                                                                                                                                                                                                         |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`                  | Approximately 120 lines for old-function-syntax detection and rejection. The body of `skip_old_function_syntax` (`:120-122`) contains only a one-line comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                        |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                        | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                                 | The “Tool Design” sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, and `check/incremental-checking`. **If the C1-C3 document deletions are performed, this section must be cleaned up in sync**, or the documentation site will contain 404 links                                                                                                                                                                                                                                                                                                                                      |
| E7  | Duplicate `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | Five variants have the same names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **the `Float` widths differ (f32 × f64)**. There are no `From` / `TryFrom` implementations; handwritten conversions are scattered across call sites (`const_eval.rs` / `ir_gen.rs`). `kind()` at `const_data.rs:44` contains live `todo!()` implementations for `LibraryRef` / `ExternRef`. **Disposition: consolidate into one definition + explicit conversions + replace `todo!()` with fallback output, assigned to P10 (Category E local cleanup)**. Registered by the P0 gate on 2026-10-06 (Criterion A, J=0.56) |
| E8  | Duplicate downstream mappings from `Literal` (Prohibition 3-A)                                          | At least three equivalent `Literal → MonoType` arms (`Literal::Int(_) => MonoType::Int(64)` and similar in `inference/expressions.rs`, `checker.rs`, and `inference/patterns.rs`); 19 handwritten `Literal → ir::ConstValue` mappings in `ir_gen.rs` and four handwritten `Literal → const_data::ConstValue` mappings in `const_eval.rs`, with no shared conversion point. **Disposition: move the three typecheck-side copies to the shared layer under P6; consolidate the `ir_gen` / `const_eval` side together with the E7 `ConstValue` merge under P10**. Registered by the P0 gate on 2026-10-06                 |

### Not Dead Code, but the Same Class of Problem: Three Hardcoded Discard Operations

The `impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1,400 lines) contains
three hardcoded discard operations:

| Line   | Discarded Data                   | Consequence                                              |
| ------ | -------------------------------- | -------------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | Upvalue metadata is lost                                 |
| `2315` | `exception_handlers: Vec::new()` | **The `.42` artifact loses its exception table**         |
| `2341` | `globals: Vec::new()`            | **The `.42` artifact loses global-variable information** |

**These three locations are not dead code; they are data-loss defects**—the `.42` artifact is
complete only along the in-memory path. **Their disposition differs from dead code**: ~~they should
be tracked as separate issues~~ **all three have been incorporated into P7 batch e (Decision D52,
2026-10-05), with no separate issue**—the exception table (`2315`) and global variables (`2341`) are
part of the artifact’s integrity for core language semantics and are not “deferrable” items. Fix all
three together, and raise the `.42` `VERSION` from 4 to 5 in one step (the format header already has
`MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`). They are listed here only to prevent them
from being mistaken for “dead code” and cleaned up as such.

## F. Not to Be Deleted, but to Be Marked or Decided

| #   | Target                                                                                                                                                                                                                                                                  | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (**already handled under P5 5.1**: 1,547 lines at the time, now 1,553 lines and a real `mod`)                                                                                                                  | **Not dead code.** At the time, it was spliced into the end of `checker.rs` through `include!` (the repository’s only `include!` occurrence), and **the file was not declared as any module** (a repository-wide `grep 'mod semantic_tokens'` had zero matches). Its first line was directly `impl TypeChecker {`, with no `use` header, and it was the only file in the `checker/` directory. Consequences: no module identity, failure of visibility isolation, broken rust-analyzer navigation and symbol search, and the toolchain treated it as part of `checker.rs`. At the time, the actual size of the `checker` module was 5618+1547 ≈ 7165 lines (now `checker.rs` is 4167 lines + three `checker/` submodules of 1553/1107/470 lines). **Fix: make it a real `mod`** (completed on 2026-10-26 by [09](09-execution-wbs.md) §P5 5.1; the `check-boundary` B1 inventory is now zero), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| F2  | wasm attributes (both directions count when they occur on the same line): the current `src/` tree contains the literal `wasm32` in **29** files, **22** with actual attributes and **7** mentioned only; `orchestrator.rs` has **10** entries (4 positive + 6 negative) | **Not dead code.** wasm targets are carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` specifies `crate-type = ["cdylib"]`, `:18` specifies `wasm-bindgen = "0.2"`, and `:17` depends on the main crate; the root `Cargo.toml:3` `exclude = ["wasm", ...]` explicitly confirms that it is an independent crate. Looking only at `crate-type = ["rlib"]` in root `Cargo.toml:31` would lead to the false conclusion that these branches are defensive dead code—**that inference is invalid**. The wasm CI build is in `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), invoked by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches are load-bearing**: they determine which main-crate APIs `wasm/src/lib.rs` (currently **152** lines) can call under the wasm target (`src/lib.rs:151` / `:170` / `:187` gate out `run_file` / `run_project` / `build_bytecode` as a whole). **Cleanup scope has been determined**: four wasm arms are runtime-unreachable in playground scenarios but compile-time load-bearing (their call sites are not gated); six `not(wasm32)` arms are load-bearing in the native three phase tables ⇒ **there are no directly deletable low-risk items**—see “F2 Verdict” below for the item-by-item determination and evidence (Decision D35: delete unreachable items and retain reachable ones, assigned to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                                                                                                                                                                                                          | It has only a “Status” column (document status) and **no “Implementation Status” column**, so it **cannot express** the error “an RFC is marked complete, but the code does not exist.” Only 5 of 52 RFCs in the repository have an `impl_status` field (`027` in-progress; `017`/`014`/`014b` complete; `review/027a` partial). **None of the 26 accepted RFCs records implementation status**, including 009, 009a, 013, 018, and 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                          | **Already handled under 4.4.1**: the README now describes the actual call sequence of `checker.rs::check_module_impl` (refinement checks proceed through `predicate` → `termination` → `ownership` → type-constraint solving); the parenthesized line numbers are taken from the current tree, with function names and driver points as stable anchors. It no longer claims that “a lower layer failing prevents upper layers from running” (there is in fact no short-circuit); `equivalence` is described by its real identity as a “shared type-comparison utility + proof entry point used only by tests.” This row serves only as a record of the completed disposition and no longer points to the old line ranges                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                                 | `:79` places RFC-026a in the “Accepted RFCs” table, but its status column says “RFCs under review”; the “RFCs under review” section at `:48-49` lists only 032 and 027a, omitting 026a. This is inconsistent with the three review entries in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| F6  | `tools/cargo-dist/` (`dist.exe`, 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                             | **Neither is tracked by Git** (`git ls-files` returns nothing), and both are excluded by `.gitignore`. **This is not a problem**; it is recorded only to prevent later audits from mistakenly concluding that “binaries were committed”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                    | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` advances to the next quote), dependent on the `("E1001", ...)` tuple syntax; **it is not a parser**. Its architecture (`parse` + `validate` + a build.rs gate + a `--fix` cure) is worth reusing for the opcode table, but **the extractor must be rewritten for the opcode shape (about 30 lines)**. This belongs to follow-up work in 06-cleanup-inventory.md or to a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                                                                                                                                                                                             | 83 constants, zero duplicates, range `0x00..0xE2` (span 227, **144 gaps**). The same opcode facts are expressed in **five places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / the decoding match (`bytecode.rs:978-2302`) / 48 `translate_*` functions (`translator.rs:676-1577`). **The compiler enforces only two of these** (the dispatch table at `backends/.../executor/debug.rs:194` + the exhaustive matches in each `ops/*.rs` family); a missing arm in `opcode()` / `size()` / the decoder / the encoder **crashes only when a `.42` artifact is run**. The comment in the `size()` table (`649-803`) states that its encoding does not match reality (`bytecode.rs:2181-2182`), and **there is no systematic differential test** (the existing size tests cover only Nop/Mov/Borrow/Release). Operator semantics also follow a third parallel enum (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`; `Rem`/`Xor`/`Sar` use yet another naming scheme), to be consolidated under the opcode-generation-time gate. The target location for the vocabulary is `middle/bytecode/opcode.rs` (migrated with the bytecode-domain consolidation, eliminating the L3→L4 reverse dependency; see 01)                                                                                                                                                                               |

### F2 Verdict: wasm Attribute Reachability in `orchestrator.rs` (Determined 2026-10-18; Cleanup Not Included)

**Assessment surface** = wasm production build (the shim crate packaged by `wasm-pack`; see the F2
row). **Assessment revision**: `b5e68b3c9`. This section records only the determination and creates
no cleanup commitment (cleanup belongs to P10 / D35).

#### I. Counting Convention (Retaken from the Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- The current `orchestrator.rs` contains **10 wasm attributes** = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- The counting convention is **the `#[cfg(...)]` attribute text on the same line**, counting both
  positive and negative forms (the same convention used for the “12” in 02); runtime `cfg!(...)` is
  not counted—a repository-wide `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'` has zero
  matches.
- The “12” is not an error; it is **a fact of an older revision**: at `544695ea9` (when the RFC-039
  design document was first committed), there were 5 positive + 7 negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- The difference comes from **a pair of attributes being moved as a pair**: `6ec330d22` (WBS 4.2.2,
  which slimmed `check_project` into a `Program` constructor and reduced `orchestrator.rs` by a net
  252 lines) moved `role_context`, together with its `not(wasm32)` implementation and `wasm32` stub,
  to `src/driver/helpers.rs:360/394`. `orchestrator.rs` therefore dropped from 12 to 10; the
  mechanism itself did not disappear.
- Revision-by-revision review (actually performed for this assessment; excerpt below, with
  `6ec330d22` as the old/new boundary):

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

#### II. The Three-Question Criterion (Splitting It Prevents Load-Bearing Arms from Being Mistaken for Dead Code)

1. **Compile-time presence**—does the arm participate in compilation under the target?
2. **Runtime reachability**—does a call chain from the playground entry point exist under that
   target?
3. **Deletion cost**—would deleting the arm break compilation for that target (are the call sites
   gated as well)?

#### III. Starting Point (Playground Entry Points and Phase Table)

- Entry points: `wasm/src/lib.rs` (currently 152 lines) `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, which constructs only a `Pipeline` and does not
  touch the orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` follows `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`;
  `SingleFile | WasmPlayground` share the same table): Parsing / Typecheck / DeadCodeAnalysis /
  ProofExecution / IrGeneration / Monomorphization—**excluding** VendorConsistency / Discovery /
  Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### IV. Item-by-Item Determination (Current Tree: 10 Entries)

| #   | Location (`orchestrator.rs`) | Gated Target                                                           | Compiled in wasm Build                      | Playground Runtime-Reachable                                                                                                                                                                         | Deletion Cost (Is the Call Site Gated?)                                                        |
| --- | ---------------------------- | ---------------------------------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| 1   | `:271` positive              | wasm arm of `ensure_vendor_consistency` (`:267`) (`Ok(None)`)          | Yes                                         | **No**: the only call site, `src/driver/arms.rs:1195` (the VendorConsistency arm), is not in the playground phase table                                                                              | High: `arms.rs:1195` is not gated; deleting the arm alone ⇒ wasm compilation fails             |
| 2   | `:418` positive              | wasm arm of `discover_with_used` (`:400`) (sets `lock_versions` empty) | Yes                                         | **No**: call sites are `arms.rs:1221` (the Discovery arm) and `src/driver/helpers.rs:686` (the Check-form reference pool `collect_package_refs`, called only from the Check branch at `arms.rs:521`) | High: same as above                                                                            |
| 3   | `:655` positive              | wasm stub for `member_context` (`:656-659`)                            | Yes                                         | **No**: the only call site is `:404` (inside the function body in #2)                                                                                                                                | High: `:404` is not gated                                                                      |
| 4   | `:753` positive              | wasm stub for `dep_import_surface` (`:754-756`)                        | Yes                                         | **No**: call sites are `:671` (`resolve_in_package_root`) and `:813` (`resolve_in_vendor_with_lock`), rooted at `:468` (`resolve_module_path`) in the BFS of `discover_with_used`                    | High: same as above                                                                            |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                       | No (does not participate in the wasm build) | Not applicable                                                                                                                                                                                       | Native load-bearing: used at `:678` / `:747`                                                   |
| 6   | `:276` negative              | native arm of `ensure_vendor_consistency`                              | No                                          | Not applicable                                                                                                                                                                                       | Native load-bearing: VendorConsistency appears in three phase tables at `program.rs:61/98/113` |
| 7   | `:413` negative              | native arm of `discover_with_used` (lock reading)                      | No                                          | Not applicable                                                                                                                                                                                       | Native load-bearing: Discovery appears at `program.rs:62/99/114`                               |
| 8   | `:607` negative              | native `member_context` (workspace resolution)                         | No                                          | Not applicable                                                                                                                                                                                       | Native load-bearing: call site `:404`                                                          |
| 9   | `:677` negative              | manifest-name reading in `resolve_in_package_root`                     | No                                          | Not applicable                                                                                                                                                                                       | Native load-bearing: `:544/:547` ← `:468`                                                      |
| 10  | `:745` negative              | native `dep_import_surface`                                            | No                                          | Not applicable                                                                                                                                                                                       | Native load-bearing: `:671` / `:813`                                                           |

#### V. Conclusions

1. **The four wasm arms are runtime-unreachable in playground scenarios** (#1-#4; evidence = the
   phase table contains no stage containing their sole call sites), but they are **compile-time
   load-bearing**. Executing D35’s literal “delete unreachable items” directly would fail to
   compile. A feasible form is to “gate the entire project-mode subtree on wasm32 (the
   discovery/resolution functions in `orchestrator.rs` + the VendorConsistency/Discovery arms in the
   driver + the reference-pool collection in `helpers.rs`)”; the cost and benefit are for P10 to
   decide.
2. **The six `not(wasm32)` arms do not exist in the playground build** (excluded at compile time),
   and all are load-bearing on the native side (the MultiFile / Check / Lsp phase tables).
   “Unreachable in the playground ⇒ delete” **does not apply**: deleting under that criterion would
   directly break the CLI/LSP.
3. ⇒ This directory has **no “directly deletable, low-risk” items**. F2’s only potential cleanup
   benefit is “whether the wasm artifact becomes smaller,” which has not been verified (see below).

#### VI. Unverified Items (Must Not Be Treated as Conclusions)

- **The size benefit for the wasm artifact has not been measured**: neither `wasm-pack build` nor
  artifact-size/symbol-table tools (`twiggy` / `wasm-objdump`) were run, so there is no data on “how
  much smaller the artifact would become if the project-mode subtree were gated out.” It has also
  not been verified whether the linker removes unreferenced `pub(crate)` items.
- **wasm32 consumers outside the shim have not been exhaustively enumerated**: this round checked
  only CI within the repository (the only wasm32 build is at `_build-wasm.yml:30/77`, through the
  shim crate). If a third party compiles the main crate directly for `wasm32-unknown-unknown` and
  calls project-mode APIs, the reachable surface may exceed this assessment.
- **Native-side “load-bearing” means a static call chain + phase table**; no end-to-end runtime
  verification was performed for each arm (the MultiFile/Check/Lsp probes were not run one by one
  against every arm).

#### VII. Appendix: Repository-Wide Count Verification (Same Revision)

```console
$ git grep -l wasm32 b5e68b3c9 -- '*.rs' | wc -l
30        # includes the wasm/ shim crate
$ git grep -l wasm32 b5e68b3c9 -- 'src/*.rs' | wc -l
29
$ git grep -lE '#\[cfg\([^]]*wasm32' b5e68b3c9 -- 'src/*.rs' | wc -l
22        # contains actual cfg attributes
$ git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs' | wc -l
0         # no runtime cfg! convention
```

The “wasm status” in 02 and the F2 row use the same convention: the current tree contains the
literal `wasm32` in **29** files / **22** files with actual `#[cfg(...)]` attributes / **7** files
mentioned only in documentation; `orchestrator.rs` has **16** occurrences = **10** attributes (4
positive + 6 negative) + 6 comments (the seven files mentioned only are: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Criterion Findings Registry (Added 2026-10-06, Continued as Stages Are Implemented)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibitions 1 A/B/C/D) was
first run on 2026-10-06. Under the division of responsibility in which “the gate reports only
suspects, and a human adjudicates” ([08](08-maintenance-mechanism.md) §Known Limitations), this
section records the adjudication results and disposition ownership. Unexpanded matches (25 pairs at
the Criterion A violation layer, and all Criterion B suspects) are to be reviewed pair by pair as
the corresponding stages are implemented.

| #   | Target                                                                                                              | Adjudication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Disposition Owner                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                       | **Legitimate structure, not parallel representations**: a value/type-tag pair maintained at 1:1 by the exhaustive `value_type()` (`value.rs:326-369`), where a missing arm causes a compilation failure. However, external consumption of `ValueType` amounts only to `{:?}` formatting on std/os/fs/net error paths and the constant `Void` assignment at `executor.rs:410`; its 21 variants far exceed actual needs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | P10 S5 slimming candidate: reduce to `pub(crate)` and evaluate trimming; when S5 is implemented, adjudicate the necessity of the `AsyncValue.value_type` field (`value.rs:217`) at the same time                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| G2  | Duplicate `ConstValue` (Criterion A, J=0.56)                                                                        | **True positive**, including the f32/f64 width mismatch and live `todo!()` in `kind()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Recorded as E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| G3  | 11 type-name string dispatches (true positive under Criterion D)                                                    | **True positive, all with the same root cause**: there is no single authoritative identification point for built-in container names. Inventory: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), the large table in `eval/const_eval.rs:498-511` (including the dead `"Uint"` arm), `middle/passes/codegen/bytecode.rs:744-756` `to_type_id` (`"Set" => 25`, which contradicts decision 4 of #300), and `trait_data.rs:187` `"Clone" \| "Dup"`                                                                                                                                                                                                                                                                 | **P6 expanded verification item**: these points are on the typecheck / eval / middle side and are outside the parser data-flow coverage in 03 §5.4. During P6, handle them under the same principle as 03 §5.3 (`NameKind` determined once, with no further string comparisons in match arms). The duplicate table has already been corrected to seven locations (03 §2.3)                                                                                                                                                                                                                                                          |
| G4  | Four Criterion D false positives                                                                                    | **Legitimate lookup / dispatch tables, not candidates for consolidation**: tool-installation guidance in `package/build/requirements.rs:17-25`, build-result display names in `package/vendor/mod.rs:134-139`, REPL command aliases in `repl/mod.rs:241-396` (8 arms; user-interface aliases are legitimate UX), and `parse_runtime_mode` in `util/diagnostic/mod.rs:462-468` (a commented historical alias at a single boundary point)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | When P9 is converted to a hard gate, add a `// reason:` exemption at each location; the number of exemptions is reviewed with each gate report                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| G5  | `ReleasePlan.drops` span-key mismatch (P2 2.4.4 criterion measurement; not a P0 match)                              | **True positive, a contract defect—the empirical fulfillment of D20’s prediction**: ownership records keys by the span of the **last-used expression** (`ownership.rs:2574 build_release_plan`), while `ir_gen.rs:1956` looks them up by the span of the **statement**, so expression-level keys never match—**168 files / 373 keys** across the full corpus go unconsumed, silently losing Drops (see the file header of `src/middle/core/tests/release_plan_spans.rs` for per-case evidence)                                                                                                                                                                                                                                                                                                                                                                                                                                  | **P4/D20** (Span key → PlanId; the behavioral surface spans about 168 files and must wait for the C2 criterion and corpus diff to be in place); `test_release_plan_spans_consumed` remains permanently red (`#[ignore]`) and turns green when D20 is implemented                                                                                                                                                                                                                                                                                                                                                                    |
| G6  | Three IR metadata defects (discovered while the P2 2.1.2 `verify_loose` corpus passed, fulfilling D38’s prediction) | **Three true positives, all fixed under 2.1** (the corpus-difference baseline showed zero differences, proving behavioral equivalence): ① the method IR locals table was truncated to the number of parameters (`take_cur_locals(param_types.len())` in `ir_gen.rs`)—`local_count` underestimates, and the runtime silently tolerates this through out-of-bounds resizing by `Frame::set_slot` (`frames.rs:103-105`); ② unannotated parameters in the lambda / curry-intermediate / function paths were discarded by `filter_map` (three locations), misaligning parameter count and frame layout—use an `Int(64)` placeholder to align with the existing method-path convention; ③ the match-result slot depended on implicit Void pre-initialization in the frame (if the final arm’s guard fails, the landing point is undefined in the CFG)—`generate_match_expr_ir` now explicitly pre-initializes Void at its entry point | Already fixed (same batch as 2.1). **The assessment scope has been narrowed and recorded** (`verify.rs`’s file header is authoritative): temp-slot dominance is not checked (Void pre-initialization is frame semantics, on which `ir_gen` intentionally relies); the trusted source for type consistency is narrowed to parameter-slot signatures + constants (`ir_gen` never writes `LocalSlot.ty`, and the entire table uses the `Int(64)` placeholder—the full scope belongs to P7 value table 04 §262); `init_locals` is a #368 debugging-view table, not an allocation limit, and out-of-bounds `init` Locals are not checked |

## H. Inventory of Giant Functions (Pre-Release Audit Criterion, Effective 2026-10-16)

> Criterion (established by `/pre-release-audit` on 2026-10-16): a function span > 150 lines is an
> **initial screening threshold**, not a command to “report every one.” A complete list would turn
> the inventory into a pile of line counts and drown the new signal in the existing backlog. Each
> report entry must contain three numbers: **host lines / lines inserted in this change / lines
> before insertion**. There are two categories: **A. Existing giant functions into which branches
> were inserted** (>150 lines and >50 lines inserted in this change); **B. Large functions newly
> added in this change** (almost entirely new and >100 lines). The line numbers in the table below
> are taken from audit revision `1935aab53` (the `v0.8.4..1935aab53` range).

### A. Existing Giant Functions into Which Branches Were Inserted

| Function                | Location                                                    | Host Lines | Inserted This Time | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | ------------------ | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1594       | 358                | 1236             |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171                | 642              |

### B. Large Functions Newly Added in This Change

| Function                     | Location                                            | Lines | Newly Added |
| ---------------------------- | --------------------------------------------------- | ----- | ----------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177         |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122         |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130         |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107         |

**Existing giant functions not included in the inventory** (≤ 4 lines inserted this time; record
only, with no disposition): `infer_expr` (`expressions.rs:2999`, 955 lines),
`collect_function_signature` (`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`,
441 lines), `generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), and others.

### Disposition Direction

- **Category A**: resolve these through the follow-up decomposition of `infer_call_expr` /
  `generate_call_expr_ir`—they are the repository’s heaviest hosts, and the current named-argument
  refactoring inserted 358 lines into the former.
- **Category B**: `push_joined` is a **local** helper for formatting a `Task` work list (its host is
  the 180-line addition), has a different responsibility from `mono.rs:123 increment_level_string`
  (174 lines, hierarchical-name rewriting), and **does not constitute duplication; do not merge
  them**. `consumer_stage_skipped` and `local_count_of` can be split independently.

## Key Decisions and Rationale

The authorization permits outright deletion, but this document recommends **different dispositions**
for several items:

| Category                                                               | Disposition                                      | Rationale                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Pure placeholders** (skeletons with no assertions or implementation) | **Delete**                                       | B1 (415 lines), B3 (the dispatch chain), B6 (96 lines), B7 (the empty function), C6 (14 lines), E1 (the empty directory)                                                                                                                                                                                                                               |
| **Paper-only design documents**                                        | **Delete or rewrite**                            | Delete C1 and C2, then rewrite them to point to the actual implementation; rewrite C3’s boundary table; **clean up `config.js:259-268` in sync** (E6)                                                                                                                                                                                                  |
| **RFCs whose status does not match the implementation**                | **Change the status**                            | Move C4 (RFC-018) back to `draft`, or add `impl_status: 'not-started'`; add `impl_status: 'not-started'` to C5 (RFC-028). **Do not delete them**—the design may still be valuable; the problem is the inaccurate status marker                                                                                                                         |
| **Orphan tests with real assertions**                                  | **Revive, do not delete**                        | **1,005 lines / 78 tests** in total: lexer 629 lines / 55 + 311 lines / 16 hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + 65 lines / 7 under `package/template/tests/`. **Reviving them costs one `mod` declaration and restores this coverage to CI**. Deleting them is a net loss                                   |
| **Zero-call but semantically important `pub` items**                   | **Reduce visibility rather than delete**         | B2 (`TypeSystem`) and B4 (`check_type_equivalence`)—move them to `pub(crate)` so the compiler can identify them, or complete their implementation. **B4’s `is_subtype` must be preserved**                                                                                                                                                             |
| **Opcodes / instructions that cannot produce a value**                 | **Add a construction site or delete the opcode** | B8 (`TailCall`), B9 (`Switch`, whose opcode round-trip test explicitly registers it through `WHITELIST=["SWITCH"]`), and B10 (`UnaryOp::Not` silently degenerating). One of the alternatives must be chosen; the status quo cannot be maintained; D32/D33/D34 have been adjudicated in favor of deletion, handled together with the `.42` version bump |
| **Data-loss defects**                                                  | ~~Separate issue~~ **P7 batch e (D52)**          | The three hardcoded discard operations in Section E are not dead code                                                                                                                                                                                                                                                                                  |
| **Valid code without module identity**                                 | **Refactor, do not delete**                      | F1 (`semantic_tokens.rs`): change `include!` to a real `mod` (see `02`)                                                                                                                                                                                                                                                                                |
| **Items requiring a decision**                                         | **List as open questions**                       | wasm32 cleanup under F2 is registered as [09](09-execution-wbs.md) §P10 **S7** (gate the project-mode subtree on wasm32; user decision B on 2026-10-18; includes the three unverified items unchanged); layer-order correction will expose new diagnostics                                                                                             |

### Directions Not Adopted

- **Treat everything as “outright deletion,” including orphan tests**—the 1,005 lines of tests with
  real assertions are assets. The deletion cost is one `mod` declaration; the benefit is restoring
  coverage to CI. The authorization means “deletion is allowed,” not “deletion is required.”
- **Only add gates without changing code**—gates can prevent regressions, but B1’s 415 lines, the
  dispatch chain, and the 96-line dead ladder **already exist**; gates would merely freeze them in
  the baseline.
- **Retain the `accepted` status of RFC-018 / RFC-028 and only supplement the
  documentation**—`rfc/index.md:131` defines “accepted” as “entering the implementation stage,” so
  readers would reasonably assume that an implementation exists. `TRACKING.md` already has precedent
  for an `impl_status` field (5 RFCs use it), and the cost of adding the field is far lower than the
  cost of a misleading status.

## Implementation Points

**This document does not require an equivalence oracle** (it belongs to the
[C6 pure-deletion category](07-equivalence-oracle.md)), but **references must still be confirmed
item by item**. In RFC-039’s global stage sequence, S1 corresponds to P1, and S2-S6 correspond to
P10.

| Stage | Contents                                                                                                                                      | Acceptance                                                                                                                                                                                                                                                                               |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphan tests with assertions (**1,005 lines / 78 tests**)                                                                              | The number of tests for the lexer / pratt / passes / emitter / template modules increases under `cargo test`; **real defects are expected to be exposed** (the overflow paths in `literals.rs` and invalid `\x`/`\u` escapes have never been tested), so time must be reserved for fixes |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 closed out with 05/C6/E1)                                                                      | `cargo test` passes; `cargo clippy -D warnings` produces no new warnings                                                                                                                                                                                                                 |
| S3    | Handle paper-only design documents (C1-C3 + clean up `config.js` in sync with E6)                                                             | The documentation site builds with no 404s; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                                      |
| S4    | Correct RFC status (C4/C5/F3—add an “Implementation Status” column to `TRACKING.md`)                                                          | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` is regenerated automatically; **do not edit it manually**)                                                                                                                                                                  |
| S5    | Reduce the visibility of `pub` items (B2/B4)                                                                                                  | The compiler can identify the truly dead code                                                                                                                                                                                                                                            |
| S6    | Implement the decisions for opcodes that cannot produce values (B8/B9/B10/F8, according to D32/D33/D34)                                       | Delete the dead paths for `Switch` / `TailCall`; make B10’s `opcode()` distinguish the `op` field; implement the F8 gate                                                                                                                                                                 |
| S7    | Refactor `include!` (F1, implementation steps assigned to [09](09-execution-wbs.md) §P5 5.1) + correct layer order (F4, assigned to P4 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                                                               |

**S1 must be performed first, and time must be reserved for defect fixes.** Its value lies not in
“cleanup,” but in **restoring 1,005 lines / 78 never-run tests to CI**—until then, every judgment
about test coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects.** The four radix scanners in `literals.rs` have four isomorphic
  branches that “continue consuming after overflow without reporting an error”; the entire
  `scan_leading_dot` function; and the invalid `\x`/`\u` escape paths have all never been tested.
  Reviving the tests will probably require code fixes, delaying the subsequent refactoring.
- **B9/B10 involve language-behavior decisions.** Whether to “add a construction site” or “delete
  the opcode” for `Switch` and `UnaryOp::Not` requires considering whether either is part of the
  surface behavior on which released corpora depend.
- **After C1-C3 are deleted, cross-file-analysis capability documentation will be temporarily
  absent** until `check/cross-file-analysis.md` is rewritten to point to the actual architecture
  under `src/frontend/module/`.
- **Reachability analysis can only identify “unreachable” code, not “reachable but meaningless”
  code.** Judgments other than Form 2 must be made item by item, and **this inventory does not
  constitute an exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2 and B4)
  cannot be discovered automatically by the compiler and can only be found through searches.
- **Deleting the half-completed removal from RFC-023** (B1) fulfills the decision that “closures
  have been removed from the language”; **correcting status inaccuracies** (C4/C5/F3) restores the
  reference value of `TRACKING.md`.

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> decisions (D1–D50). **This document retains no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  source decision for the B1 residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — the unwired runtime check insertion in B3, `§4.1`
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — official home of C1/C6
- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — overarching specification
- [Refactoring Equivalence Oracle](07-equivalence-oracle.md) — criterion requirements for the C6
  category
- `src/frontend/core/typecheck/layers/README.md` — the proof layer’s actual call order and the true
  identity of `equivalence` (F4 handled)
- `build.rs:19-55` — an example of this repository’s generation-time gates
