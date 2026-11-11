# Dead Code and Paper-Only Design Cleanup

> **Companion design document.** This document accompanies
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> The four-layer model, tiered acceptance criteria, and execution-stage ordering are covered in the
> main RFC-039 text; see the [index of this directory](index.md) for the positioning of each
> companion document.

## Positioning and Scope

This document provides an inventory of dead code and paper-only designs in the YaoXiang repository,
the methods used to identify them, item-by-item disposition decisions, and the execution order. The
cleanup targets fall into four categories:

| Category                                                                     | Scale                                                                    |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Files that have never participated in compilation                            | 24 files / 2,628 lines, of which 23 files / 1,081 lines are true orphans |
| Compiled but never-called dead code                                          | 1,500+ lines                                                             |
| Design documents describing an architecture that never existed               | Three documents under `docs/src/dev/design/check/`                       |
| Features attached to the default entry point with zero tests and zero output | 1,152 lines under `src/repl/`                                            |

**The authorized scope is “permission to delete outright”** (decision dated 2026-10-03). However,
for several items, this document gives **disposition recommendations contrary to “direct
deletion”**—see [Key Decisions and Rationale](#关键决策与理由).

This document **does not cover** module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), or criterion design (see `07-equivalence-oracle.md`).
B3 records the entire unwired `layers/dispatch.rs` chain; its deletion is assigned to S2.

## Current State

### Three Forms of Dead-Code Debt

**Form 1: Placeholder.** The code exists and appears to be doing something, but in reality does
nothing.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the “runtime check insertion” mechanism designed by `RFC-027 §4.1` and is
  **completely unwired**.
- `check_type_equivalence` (`ProofContext` entry point) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is called only by tests. **Note:**
  `is_subtype` in the same file is used in production by `inference/assignment.rs:17`, so **the
  entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1,047 lines) is a pure DAG task scheduler. Its `pub` surface
  consists only of `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend**—the
  `Executor` trait in `backends/mod.rs:356` currently has only one implementer, the interpreter.
  This point needs clarification because the directory name is potentially misleading.

**Form 2: Residue.** An upstream decision has been made, but its deletion was only half completed.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) retains six types: `FunctionInstance`
  (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), and `CaptureVariable`. **The source is clear:** when `RFC-023`
  deprecated the “closure capture model,” it declared that “**approximately 850 lines of code would
  be deleted together with the closure model**.” These 415 lines are the remnants that survived from
  those 850 lines—**only half of the deletion was completed**. **Verification:** a repository-wide
  word-boundary search for these six types found **zero matches outside `instance.rs`** (including
  tests). The `pub use passes::mono::instance::*;` in `middle/mod.rs:40` is only a glob re-export
  and does not constitute consumption.

**Form 3: False Coverage.** This is the most dangerous form—**it makes the repository appear more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` contains only five `pub mod` declarations (`literals` / `state` /
`symbols` / `tokenizer` / `tokens`) plus one
`#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;` (`:104-106`). **Nowhere is
`mod tests;` declared**, so the entire `lexer/tests/` subtree—13 files and 686 lines—**has never
participated in compilation**.

Seven of these are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
`delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totaling 19 lines and 0
assertions). **629 lines / 55 real tests have never run**:

| File              | Lines   | `#[test]` |
| ----------------- | ------- | --------- |
| `literals.rs`     | 222     | 30        |
| `rfc010_lexer.rs` | 167     | 12        |
| `lexer_mod.rs`    | 89      | 5         |
| `rfc004_lexer.rs` | 81      | 5         |
| `symbols.rs`      | 70      | 3         |
| **Total**         | **629** | **55**    |

(`fstring.rs`—90 lines / 11 tests—is genuinely compiled, but through the `#[path]` bypass rather
than `mod tests;`; `mod.rs`—38 lines—is a declaration file with no assertions.)

**In reality, only one file and 12 tests are running at the lexical layer, and all of them test only
the f-string brace raw protocol.**

The danger of this kind of problem lies in the fact that `cargo test` does not fail (tests that
never ran cannot fail), and CI has no test-count baseline or wiring check.

### Paper-Only Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not a case of “the design is complete but the implementation is missing”; **the
documents describe a different system that never existed**, and present a “possible future design”
as a defect in an implemented system.

## Methodology and Confidence Statement

### Reachability Analysis Method

Starting from the two crate roots, `src/lib.rs` and `src/main.rs`, this analysis resolves the `mod`
/ `pub mod` / `#[path]` declarations of all 509 `.rs` files, constructs a module graph, and performs
BFS reachability analysis. `include!` targets are adjudicated separately and are not counted toward
reachability.

**This method has known false-positive patterns, two of which were actually triggered while writing
this document:**

1. `Split-Path` returns `\` on Windows, which does not match forward-slash keys ⇒ 499 orphans were
   temporarily misreported.
2. **The submodule directory of a non-`mod.rs` file is `its-own-name/`, not `parent-directory/`** ⇒
   `orchestrator.rs`’s `mod tests` was resolved as `src/frontend/module/tests/`, nearly causing 753
   lines of live tests to be classified as dead code.

Therefore, **every conclusion was confirmed twice—through actual file reading and actual reference
searches**. Items that failed the second confirmation were not included in this document.

### Line-Count Metric

All line counts in this document were obtained with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines and systematically undercounts by approximately 5%** (for
example, `manifesto.md` is reported as 658 lines with `-Line`, versus an actual count of 893). An
earlier analysis of this repository produced an artificially low total because of this metric; the
figures here have been corrected.

### Known Limitations

- **Reachability analysis can only identify “unreachable” code; it cannot identify “reachable but
  meaningless” code.** Forms other than Form 2 require item-by-item manual judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types must be
  found through searches; this method does not cover them.
- This document does not involve running the compiler, so all conclusions of the “never ran” kind
  are static determinations.

## A. Files That Have Never Participated in Compilation

**Total: 24 files / 2,628 lines.** `semantic_tokens.rs` belongs to Form 4 (see Section F); the
remaining **23 files / 1,081 lines are true orphans**.

| Path                                                          | Scale                   | Content                                                                       | Basis for Determination                                                                                                                                                                                            |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Entire `src/frontend/core/lexer/tests/` subtree, 13 files     | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only five `pub mod` declarations; the only test declaration at `:104-106` is a `#[path]` reference to `tests/fstring.rs`. A repository-wide `grep 'mod tests'` finds only `mod.rs:105` |
| 3 files under `src/frontend/pipeline/tests/`                  | 14 lines                | All are placeholder documentation comments, with 0 assertions                 | The entire 740-line `src/frontend/pipeline.rs` contains no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declarations (`Select-String` returns zero matches)                                                      |
| 3 files under `src/package/template/tests/`                   | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]` functions**           | The 7-line `src/package/template/mod.rs` declares only `mod gitignore; mod main_yx;`, with no `mod tests;`                                                                                                         |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden inside a **live** directory                                            | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                                                    |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                                       |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                                               |

**The last three entries total 16 tests / 311 lines of code that have never run.** They are hidden
inside active directories whose `mod.rs` files appear complete, making them more deceptive than an
entirely orphaned directory.

Two other files under `lexer/tests/` also fall into the “declared but omitted from the declaration”
form, but are already included in the 629 lines in the first row and are **not counted twice**:
`lexer_mod.rs` (89 lines / 5 tests; `lexer/tests/mod.rs` declares 11 `mod` declarations but omits
`mod lexer_mod;`) and `symbols.rs` (70 lines / 3 tests, likewise omitted).

**The real tests that have never run total 1,005 lines / 78 tests**: lexer 629/55 + pratt / passes /
emitter 311/16 + package/template 65/7.

**Pay particular attention to `precedence_inline.rs`**: it tests the production-zero-use
`Precedence` enum and `PrecedenceContext` in `precedence.rs` (96 lines of dead code). Even if it
were wired in, it would still be testing a dead path.

Another item hidden among the 23 orphans is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, 0 tests): `typecheck/tests/mod.rs` declares 23 test modules but omits this one—the file now
contains only a one-line `use` and is a pure remnant. When S1 revives the test suite, either delete
it together with the wiring work or add assertions.

## B. Compiled but Never-Called Dead Code

| #   | Location                                                                         | Scale                   | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | -------------------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types     | A repository-wide word-boundary search finds zero matches outside `instance.rs`, including tests. RFC-023 declared the deletion of approximately 850 lines, but only half was removed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines                | `TypeSystem` and four pure functions. The only reference chain is `pub mod types;` at `inference/mod.rs:11` and `pub use types::TypeSystem;` at `:24` (both re-exports). Repository-wide, `TypeSystem` appears in only 5 places: 2 in the definition, 1 re-export, and 2 in the **unrelated** `lexer/symbols.rs:503 TypeSystemValidator`. The four methods have zero call sites. **Because they are `pub`, the compiler never reports them**                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B3  | Entire chain in `src/frontend/core/typecheck/layers/dispatch.rs`                 | —                       | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` have zero call sites. The runtime check insertion mechanism in RFC-027 §4.1 is unwired. Note that it is declared in `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                       | `check_type_equivalence` is called only by tests. **`is_subtype` in the same file must be retained** (used in production by `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2              | Described in the source as a “framework placeholder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines                | The `Precedence` enum and `PrecedenceContext`. **Their only external reference is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests)—and that test file has never itself been compiled** (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore, **both must be deleted together**: deleting only the test leaves 96 lines of dead code, while deleting only the enum causes the test to fail to compile                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | Approximately 120 lines | `is_old_function_syntax` is called at `:719`, and `skip_old_function_syntax` is called at `:725`—**they are active old-syntax rejection gates, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) contains only the comment `// old syntax removed`, but **the function consumes no tokens when called**—this is an independent behavioral issue (whether the rejection path correctly advances the parser position), not dead code. **Disposition: review the behavior together with 05-frontend-paradigm.md (decision D27: naturally removed with the grammar migration), rather than deleting it directly as dead code**                                                                                                                                                                                                                                         |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                       | Repository-wide, there are only match sites and no construction sites. The “no generation site (dead path)” comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B9  | `Switch` opcode                                                                  | —                       | `backends/common/opcode.rs` defines it, provides a variant, `opcode()`/`size()` arms, and decoding arms, **but there are zero construction sites repository-wide**. The interpreter has an active implementation at `ops/control.rs:90`. **It has been explicitly registered by tests as a known dead path**: `tests/bytecode.rs:1035-1040` contains `WHITELIST: &[&str] = &["SWITCH"]` (the test fails if the difference is non-empty)                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B10 | `UnaryOp::Not` (`bytecode.rs:101`)                                               | —                       | `opcode()` at `:590` uses `BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG`, **ignoring the `op` field**, and the decoder at `1236-1248` constructs only `Neg`. **Through the `opcode()` mapping path, it silently degrades to I64_NEG** (in a direct `.42` run, `!x` becomes `-x`; by contrast, the interpreter’s in-memory path is correct at `ops/arith.rs:46-49`). It is used only by the interpreter at `ops/arith.rs:46,49` and by `tests/logical_not.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B11 | Four `#[allow(dead_code)]` sites                                                 | —                       | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (both are genuine dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (a conservative exemption for an extern binding, **reasonable to retain**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                       | **Zero production construction sites** (the only construction site manually creates a node in the test at `parser/tests/ast.rs:828`), but there are **12 production consumers + 2 exhaustive-match arms** (the complete list appears in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()` and `pratt/mod.rs:47` `expr_end_line`. Actual function definitions take the `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`) path. Deletion is assigned to P6 (the 16-item change list in 03 §5.5)  |
| B13 | `Instruction::Cast` / `Instruction::TypeTest` (`src/middle/core/ir.rs`)          | —                       | Repository-wide, there are only consumer arms and **no construction sites** (the same pattern as B8), and their payloads are discarded downstream—`translate_cast` cannot obtain `target_type`, while the `TypeTest` arm emits the `vec![0, 0, 0]` empty-operand placeholder. **WBS 6.6.11 has deleted them** (both variants + all `normalize`/`verify`/`mono`/`translator` consumer arms + the consequently orphaned `substitute_block`/`substitute_instruction`/`substitute_type_ast`/`translate_cast`); a grep across 216 IR snapshots yields zero matches for this form. **The deletion does not affect syntax-level `as` support**, but makes the existing gap that “`as` semantics are not implemented” more visible ⇒ the gap and its evidence chain are registered in [09a](./09a-wbs-history.md) §P6 6.6.11, with the SPEC-side note in `docs/src/reference/language-spec/syntax.md` §2.6 |

## C. Paper-Only Designs

| #   | Object                                                                          | Scale                                       | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                    | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, and `HotReloader` have **zero repository-wide matches**. The “known limitations” at `:46-47` admit that `check_incremental` still calls `check_files_with_diagnostics` internally (the full path); `:46` also references the busy-wait debounce in `command.rs`—yet **`command.rs` contains neither `Instant` nor `recv_timeout`** (repository-wide, `recv_timeout` appears only in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                    | `:37` claims that the placeholder implementations under `traits/` (coherence / impl_check / object_safety / resolution) are incomplete—yet **the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero repository-wide matches. `:38` claims that `check_single_module` creates a separate Compiler for each file—**the function does not exist**, and appears only in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                            |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                    | `:14` claims that “watch mode rechecks only affected files,” and `:27` claims that “check only performs the first two steps.” In reality, `check_project` also performs vendor consistency checking, project role classification, dead-code analysis, and W1006; the `Check` subcommand in `main.rs` **has no `--watch` flag** (all `#[arg]` declarations were checked individually)                                                                                                                                                                          |
| C4  | RFC-018 LLVM AOT                                                                | 1,037 lines of design / **0 lines of code** | The AOT references in `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` contains no cranelift / llvm / inkwell. `:861-876` contains an implementation checklist with every item still `- [ ]`, yet the status is marked `accepted`                                                                                                                                                                                                                                                            |
| C5  | RFC-028 JIT                                                                     | 419 lines of design / **0 lines of code**   | `028-jit-compiler.md:99-126` requires 6 files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`; **none of them exist**. The status is `draft`, which is comparatively honest                                                                                                                                                                                                                                                                                                                                                                       |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                                | **Concrete, physical evidence**: the filenames correspond exactly to the `ModuleCache` and incremental scheduler from the C1 design. **Someone created test skeletons based on the design, found no corresponding implementation in the pipeline, and left them unfinished**                                                                                                                                                                                                                                                                                  |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                           | The `//! §6.1: CheckSession incremental checking` at `util/diagnostic/tests/mod.rs:4`, and the `§4.4` / `§5.2` references at `mod_tests.rs:4`, point to this **gitignored plan document** (`.gitignore` includes `superpowers/`). These section references are **archaeological references to obsolete APIs**                                                                                                                                                                                                                                                 |

### Actual Implementation Location of Cross-File Analysis

The cross-file analysis capability described in C1-C3 **does genuinely exist**, but under completely
different paths:

| Component            | Location                              | Lines                   |
| -------------------- | ------------------------------------- | ----------------------- |
| Module registry      | `src/frontend/module/registry.rs`     | 439                     |
| Resolver             | `src/frontend/module/resolver.rs`     | 195                     |
| Role classification  | `src/frontend/module/roles.rs`        | 388                     |
| Consistency checking | `src/frontend/module/consistency.rs`  | 206                     |
| Orchestration        | `src/frontend/module/orchestrator.rs` | 1,528                   |
| **Total**            |                                       | **Approximately 2,670** |

Therefore, the disposition for C1-C3 is **not to delete the capability, but to delete the misleading
documentation** and rewrite the boundary table in `check/index.md` to point to the actual
architecture under `src/frontend/module/`.

### The Disposition of RFC-019 Requires Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` **is already in the `deprecated/` directory**,
and the residual review found it **clean** (`homoicon` has zero repository-wide matches; the
document ends with the honest conclusion, “If this is restarted in the future, begin with
proof/prototype experiments, not with the RFC”). **It requires no additional disposition**—this item
was included in the authorization inventory, but review confirmed that it is already in its terminal
state.

## D. Feature-Level Dead Code: REPL

`src/repl/` contains 4 files totaling **1,152 lines**, with **zero `#[cfg(test)]` and zero `#[test]`
functions**. Meanwhile, `src/main.rs:440` contains
`args.command.unwrap_or(Commands::Repl)`—**running `yaoxiang` without arguments directly enters the
REPL**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed** | It appears only twice under `src/repl/`: the definition at `eval.rs:23` and the **match arm** at `mod.rs:180`. Therefore, `Repl::format_value` (`mod.rs:493-502`) is dead code; **the REPL does not echo any evaluation result**                                                                                                                                                                                      |
| D2  | **No state persists across rounds**          | `extract_definitions` (`eval.rs:345-377`) calls only `define_variable` (which stores a **type string**) and never calls `define_var` (which stores a `RuntimeValue`); `wrap_code` does not inject historical definitions either. The `VariableInfo::Value` branch (`eval.rs:23`) is unreachable. `REPLContext.variables` exists solely to support completion and does not participate in evaluation                   |
| D3  | **The wrapper syntax is probably obsolete**  | `eval.rs:314`/`316` produce `main() -> () = () => { code }` (the old-style `name() -> Ret` function declaration header), whereas the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`; the canonical table in RFC-007:94 uses `name: (a: Type) -> Ret`). **The compiler was not run for empirical verification; if this is correct, every REPL evaluation fails to compile** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` prints two lines and returns; breakpoints are stored (`mod.rs:319`), but **the interpreter never queries them**                                                                                                                                                                                                                                                                                      |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                                             |
| D6  | Duplicates `lib.rs`                          | `eval_code` at `lib.rs:85` and `wrap_code` in `repl/eval.rs` are **two independent implementations of “automatically wrap main”**, and their wrapper forms differ                                                                                                                                                                                                                                                     |

## E. Local Cleanup Items

| #   | Location                                                                                           | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --- | -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                         | Empty directory tree at the repository root. It is almost certain that the string `"undefined"` was concatenated into a path somewhere. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| E2  | `src/lib.rs:568`, `:582`                                                                           | `dump_type_detail` / `dump_const_detail` each have an `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, and the panic path is foreseeable. Replace it with meaningful degraded output                                                                                                                                                                                                                                                                                                                                                                                                 |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                          | `check_vendor_lock_consistency` and `check_workspace_consistency` are **approximately 60 nearly duplicated lines**, differing only in their dependency source and in two path-dep skips. **Merge them**                                                                                                                                                                                                                                                                                                                                                                                                              |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`             | Old function syntax detection and rejection, approximately 120 lines. The body of `skip_old_function_syntax` (`:120-122`) contains only one comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                   | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                            | The “Tool Design” sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, and `check/incremental-checking`. **If the C1-C3 documentation is deleted, this location must be cleaned up in sync**, or the documentation site will contain broken links                                                                                                                                                                                                                                                                                                                                        |
| E7  | Dual `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | Five variants have the same names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` has inconsistent bit widths (f32 × f64)**; there is no `From` / `TryFrom`, and conversions are handwritten and scattered across call sites (`const_eval.rs` / `ir_gen.rs`). `kind()` at `const_data.rs:44` contains an active `todo!()` for `LibraryRef` / `ExternRef`. **Disposition: merge into one definition + add explicit conversions + replace `todo!()` with degraded output; assign to P10 (local cleanup items in category E)**. Registered under the P0 gate on 2026-10-06 (criterion A, J=0.56) |
| E8  | Semantically duplicate downstream mappings for `Literal` (Prohibition 3 A)                         | At least three identical `Literal → MonoType` arms (`Literal::Int(_) => MonoType::Int(64)` and similar in `inference/expressions.rs`, `checker.rs`, and `inference/patterns.rs`); `Literal → ir::ConstValue` (19 handwritten sites in `ir_gen.rs`) and `Literal → const_data::ConstValue` (4 handwritten sites in `const_eval.rs`), with no shared conversion point. **Disposition: move the 3 typecheck-side copies to a shared layer under P6; consolidate the `ir_gen` / `const_eval` side together with the E7 `ConstValue` merge under P10.** Registered under the P0 gate on 2026-10-06                        |

### Not Dead Code, but the Same Class of Issue: Three Hard-Coded Discarded Fields

The `impl From<BytecodeFile>` (`943-2350`, 1,400 lines) in `src/middle/core/bytecode.rs` contains
three hard-coded discards:

| Line   | Discarded Content                | Consequence                                          |
| ------ | -------------------------------- | ---------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | Upvalue metadata is lost                             |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifacts lose the exception table**         |
| `2341` | `globals: Vec::new()`            | **`.42` artifacts lose global-variable information** |

**These three sites are not dead code; they are data-loss defects**—`.42` artifacts are complete
only along the in-memory path. **Their disposition differs from that of dead code**: ~~they should
be tracked as a separate issue~~ **all three have been incorporated into P7 batch e (decision D52,
2026-10-05), with no separate issue**—`2315` (exception table) and `2341` (global variables) concern
artifact integrity for core language semantics and are not deferrable. All three are fixed together,
and the `.42` `VERSION` is raised from 4 to 5 in one step (the format header already has `MAGIC` +
`VERSION` fields, `codegen/bytecode.rs:14-16`). They are listed here only to prevent them from being
mistakenly cleaned up as “dead code.”

## F. Not to Be Deleted, but to Be Marked or Subject to a Decision

| #   | Object                                                                                                                                                                                                                                                                                                    | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (**already handled (P5 5.1): 1,547 lines at the time, now 1,553 lines and a real `mod`**)                                                                                                                                                        | **This is not dead code.** At the time, it was spliced into `checker.rs` through `include!` (the only `include!` in the repository), and **the file was not declared as any module** (`grep 'mod semantic_tokens'` has zero repository-wide matches). Its first line was directly `impl TypeChecker {`, with no `use` header of its own, and it was the only file in the `checker/` directory. Consequences: no module identity, failure of visibility isolation, broken rust-analyzer navigation and symbol search, and the toolchain treats it as part of `checker.rs`. At the time, the actual size of the `checker` module was 5,618 + 1,547 ≈ 7,165 lines (now `checker.rs` is 4,167 lines + the three `checker/` submodules at 1,553/1,107/470 lines). **Change: convert it into a real `mod`** (completed via [09](09-execution-wbs.md) §P5 5.1 on 2026-10-26; the `check-boundary` B1 inventory has been cleared), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F2  | wasm attributes (the same-line `#[cfg(... wasm32 ...)]`, counting both positive and negative forms): the current `src/` tree contains the literal `wasm32` in **29** files, **22** with actual attributes, and **7** that only mention it; `orchestrator.rs` has **10** entries (4 positive + 6 negative) | **This is not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` has `crate-type = ["cdylib"]`, `:18` has `wasm-bindgen = "0.2"`, and `:17` depends on the main crate; the root `Cargo.toml:3` has `exclude = ["wasm", ...]`, precisely demonstrating that it is a separate crate. Looking only at the root `Cargo.toml:31` `crate-type = ["rlib"]` would lead to the false inference that these branches are defensive dead code—**that inference is invalid**. The CI wasm build is at `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), with callers at `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches are load-bearing**: they determine which main-crate APIs `wasm/src/lib.rs` (now **152** lines) can call under the wasm target (`src/lib.rs:151` / `:170` / `:187` gate `run_file` / `run_project` / `build_bytecode` as a whole). **The cleanup scope has been adjudicated**: 4 wasm arms are runtime-unreachable in the playground scenario but load-bearing at compile time (their call sites are not gated); the 6 `not(wasm32)` arms are load-bearing across the three native stage tables ⇒ **there are no directly deletable low-risk items**—see the “F2 Determination” section below for item-by-item decisions and evidence (decision D35: delete unreachable items and retain reachable ones; assigned to P10) |
| F3  | Column structure in `docs/src/rfc/TRACKING.md`                                                                                                                                                                                                                                                            | There is only a “Status” column (document status) and **no “Implementation Status” column**, so it **cannot express** the error “an RFC is marked complete but has no corresponding implementation.” Only 5 of 52 RFCs repository-wide have an `impl_status` field (`027` in-progress; `017`/`014`/`014b` complete; `review/027a` partial). **26 accepted RFCs record no implementation status whatsoever**, including 009, 009a, 013, 018, and 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                                                            | **Already handled (4.4.1)**: the README now describes the actual call order of `checker.rs::check_module_impl` (the refinement pass proceeds `predicate` → `termination` → `ownership` → type-constraint solving), with parenthesized line numbers taken from the current tree and locations anchored by function name/driver point; it no longer claims that “lower layers do not run after an upper-layer failure” (there is in fact no short-circuit); `equivalence` is described according to its actual identity as “a shared type-comparison utility + a proof entry point used only by tests.” This row is retained only as a record of the completed disposition and no longer points to the old line ranges                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                                                                   | `:79` places RFC-026a in the “Accepted RFCs” table, but its status column says “RFCs under review”; the “RFCs under review” section at `:48-49` lists only 032 and 027a, omitting 026a. This is inconsistent with the 3 review entries in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F6  | `tools/cargo-dist/` (`dist.exe`, 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                                                               | **Neither is tracked by Git** (`git ls-files` returns empty), and both are excluded by `.gitignore`. **This is not a problem**; they are recorded only to prevent subsequent audits from mistakenly concluding that “binaries were committed”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                                                      | `extract_code_from_entry_line` is a **line-initial prefix matcher** (`trim_start().strip_prefix("(")` extracts up to the next quote), and depends on the `("E1001", ...)` tuple syntax; **it is not a parser**. Its architecture (`parse` + `validate` + a build.rs gate + `--fix` remediation) is worth reusing for the opcode table, but **the extractor must be rewritten for the opcode form (approximately 30 lines)**. This belongs to follow-up work in 06-cleanup-inventory.md or a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                                                                                                                                                                                                                               | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 gaps**). The same opcode facts are expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`)/ `size()` (`649-803`)/ `opcode::opcode_name()` (`opcode.rs:120-206`)/ the decoding match (`bytecode.rs:978-2302`)/ 48 `translate_*` functions (`translator.rs:676-1577`). **The compiler enforces only 2 of them** (the dispatch table at `backends/.../executor/debug.rs:194` + the exhaustive matches in each `ops/*.rs` family); missing one from `opcode()` / `size()` / the decoding arms / the encoder fails only when running a `.42` artifact. The table at `size()` (649-803) has a comment that contradicts the actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic differential test** (the existing size tests cover only Nop/Mov/Borrow/Release, 4 cases). Operator semantics also follow a third parallel enumeration scheme here (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`, with another naming scheme for `Rem`/`Xor`/`Sar`); consolidate this together with the opcode-generation gate. The target location for the vocabulary is `middle/bytecode/opcode.rs` (migrated with the bytecode-domain consolidation, eliminating the L3→L4 reverse dependency; see 01)                                                                                                                                                                                              |

### F2 Determination: wasm Attribute Reachability in `orchestrator.rs` (Determined 2026-10-18; No Cleanup)

**Assessment surface** = the wasm production build (the shim crate produced by `wasm-pack`; see the
F2 row); **assessment revision** `b5e68b3c9`. This section records only the determination and makes
no cleanup commitment (cleanup belongs to P10 / D35).

#### I. Counting Metric (Retaken from the Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- The current `orchestrator.rs` contains **10** wasm attributes in total = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- The metric is the **text of the `#[cfg(...)]` attribute on the same line**, counting both positive
  and negative forms (this is also the metric used by 02’s “12”); runtime `cfg!(...)` macros are not
  counted—a repository-wide `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'` returns zero
  matches.
- “12” is not an error; it is **a fact about an older revision**: `544695ea9` (when the RFC-039
  design document was first committed) had 5 positive + 7 negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- The difference comes from **a pair of attributes being migrated as a pair**: `6ec330d22` (WBS
  4.2.2, which slimmed `check_project` into a `Program` constructor and deleted a net 252 lines from
  `orchestrator.rs`) moved `role_context`, together with its `not(wasm32)` implementation and
  `wasm32` stub, to `src/driver/helpers.rs:360/394`. Therefore, `orchestrator.rs` fell from 12 to
  10; the mechanism itself did not disappear.
- Revision-by-revision review (actually executed for this assessment; the excerpt below shows
  `6ec330d22` as the boundary between old and new):

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

#### II. The Three-Question Metric (Splitting It Up Prevents Load-Bearing Arms from Being Misclassified as Dead Code)

1. **Compile-time existence**—does this arm participate in compilation under the target in question?
2. **Runtime reachability**—under that target, is there a call chain starting from the playground
   entry point?
3. **Deletion cost**—would deleting it break compilation for that target (are the call sites gated
   as well)?

#### III. Determination Starting Point (Playground Entry Point and Stage Tables)

- Entry point: `wasm/src/lib.rs` (now 152 lines) `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, which only constructs `Pipeline` and does not
  touch the orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` follows `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`;
  `SingleFile | WasmPlayground` appear in the same table at `:135`): Parsing / Typecheck /
  DeadCodeAnalysis / ProofExecution / IrGeneration / Monomorphization—**excluding**
  VendorConsistency / Discovery / Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### IV. Item-by-Item Determination (10 Entries in the Current Tree)

| #   | Location (`orchestrator.rs`) | Gated Object                                                               | Compiled in wasm Build              | Reachable at Playground Runtime                                                                                                                                                                  | Deletion Cost (Are Call Sites Gated?)                                                          |
| --- | ---------------------------- | -------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| 1   | `:271` positive              | The wasm arm (`Ok(None)`) of `ensure_vendor_consistency` (`:267`)          | Yes                                 | **No**: the sole call site, `src/driver/arms.rs:1195` (the VendorConsistency arm), is not in the playground stage table                                                                          | High: `arms.rs:1195` is not gated; deleting the arm alone ⇒ wasm compilation fails             |
| 2   | `:418` positive              | The wasm arm of `discover_with_used` (`:400`) (sets `lock_versions` empty) | Yes                                 | **No**: call sites `arms.rs:1221` (the Discovery arm) and `src/driver/helpers.rs:686` (the Check-form reference pool `collect_package_refs`, called only from the Check branch at `arms.rs:521`) | High: same as above                                                                            |
| 3   | `:655` positive              | The wasm stub for `member_context` (`:656-659`)                            | Yes                                 | **No**: the sole call site is `:404` (inside the function body from #2)                                                                                                                          | High: `:404` is not gated                                                                      |
| 4   | `:753` positive              | The wasm stub for `dep_import_surface` (`:754-756`)                        | Yes                                 | **No**: call sites `:671` (`resolve_in_package_root`) and `:813` (`resolve_in_vendor_with_lock`), rooted at `:468` (`resolve_module_path`, inside the BFS of `discover_with_used`)               | High: same as above                                                                            |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                           | No (not included in the wasm build) | Not applicable                                                                                                                                                                                   | Native load-bearing: used at `:678` / `:747`                                                   |
| 6   | `:276` negative              | The native arm of `ensure_vendor_consistency`                              | No                                  | Not applicable                                                                                                                                                                                   | Native load-bearing: VendorConsistency appears in three stage tables at `program.rs:61/98/113` |
| 7   | `:413` negative              | The native arm of `discover_with_used` (lock reading)                      | No                                  | Not applicable                                                                                                                                                                                   | Native load-bearing: Discovery appears at `program.rs:62/99/114`                               |
| 8   | `:607` negative              | Native `member_context` (workspace resolution)                             | No                                  | Not applicable                                                                                                                                                                                   | Native load-bearing: call site `:404`                                                          |
| 9   | `:677` negative              | Manifest-name reading in `resolve_in_package_root`                         | No                                  | Not applicable                                                                                                                                                                                   | Native load-bearing: `:544/:547` ← `:468`                                                      |
| 10  | `:745` negative              | Native `dep_import_surface`                                                | No                                  | Not applicable                                                                                                                                                                                   | Native load-bearing: `:671` / `:813`                                                           |

#### V. Conclusions

1. **The 4 wasm arms are runtime-unreachable in the playground scenario** (#1-#4; evidence = the
   stage table does not contain the stages containing their sole call sites), but they are
   **load-bearing at compile time**. Executing D35 literally—“delete unreachable items”—would fail
   to compile. A feasible form is to “gate the entire project-mode subtree (the discovery/resolution
   functions in `orchestrator.rs` + the VendorConsistency/Discovery arms in the driver + the
   reference-pool collection in `helpers.rs`) on wasm32”; cost and benefit belong to the P10
   decision.
2. **The 6 `not(wasm32)` arms do not exist in the playground build** (excluded at compile time), and
   all are load-bearing on the native side (the MultiFile / Check / Lsp stage tables). “Unreachable
   in the playground ⇒ delete” **does not apply**: deleting by that criterion would directly break
   the CLI/LSP.
3. ⇒ This directory contains **“directly deletable, low-risk” items**; F2’s cleanup benefit is
   limited to “whether the wasm artifact becomes smaller”—that has not been verified (see below).

#### VI. Unverified Items (Must Not Be Treated as Conclusions)

- **The benefit in wasm artifact size has not been measured**: `wasm-pack build` and
  size/symbol-table tools (`twiggy` / `wasm-objdump`) were not run, so there is no data on “how much
  smaller the artifact would become if the project-mode subtree were gated out”; whether
  unreferenced `pub(crate)` items are already eliminated at link time has also not been verified.
- **wasm32 consumers outside the shim have not been exhaustively enumerated**: this round checked
  only CI within the repository (the only wasm32 compilation is at `_build-wasm.yml:30/77`, through
  the shim crate). If a third party compiles the main crate directly for `wasm32-unknown-unknown`
  and calls project-mode APIs, the reachable surface would exceed this determination.
- **On the native side, “load-bearing” means a static call chain + stage table**; no arm was
  verified end-to-end by running the MultiFile/Check/Lsp probes individually.

#### VII. Appendix: Repository-Wide Count Review (Same Revision)

```console
$ git grep -l wasm32 b5e68b3c9 -- '*.rs' | wc -l
30        # includes the wasm/ shim crate
$ git grep -l wasm32 b5e68b3c9 -- 'src/*.rs' | wc -l
29
$ git grep -lE '#\[cfg\([^]]*wasm32' b5e68b3c9 -- 'src/*.rs' | wc -l
22        # has actual cfg attributes
$ git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs' | wc -l
0         # no runtime cfg! metric
```

The “wasm status” in `02` uses the same metric as the F2 row: the current tree contains the literal
`wasm32` in **29** files / **22** files with actual `#[cfg(...)]` attributes / **7** files that only
mention it in documentation; `orchestrator.rs` has **16** occurrences = **10** attributes (4
positive + 6 negative) + 6 comments (the 7 files that only mention it: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Criterion Findings Registry (Effective 2026-10-06, Continuously Updated During Stage Execution)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibition 1 A/B/C/D) was
first run on 2026-10-06. Under the division of responsibility in which “the gate reports only
candidates, and people adjudicate them” ([08](08-maintenance-mechanism.md) §Known Limitations), this
section records the adjudication results and disposition ownership. Unexpanded findings (25 pairs
for criterion A violations, all criterion B candidates) are to be reviewed pair by pair as the
corresponding stage is executed.

| #   | Object                                                                                                                              | Adjudication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Disposition Ownership                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                                       | **Legitimate structure, not parallel representations**: a value/type-tag pair maintained at a 1:1 ratio by the exhaustive `value_type()` (`value.rs:326-369`), where a missing arm causes a compilation failure. However, the external consumption of `ValueType` now amounts essentially only to `{:?}` formatting on the std/os/fs/net error paths and the constant `Void` assignment at `executor.rs:410`; the 21 variants far exceed actual needs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | P10 S5 slimming candidate: reduce to `pub(crate)` and evaluate trimming; during S5, adjudicate the necessity of the `AsyncValue.value_type` field (`value.rs:217`) together with this item                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| G2  | Dual `ConstValue` (criterion A, J=0.56)                                                                                             | **True positive**, including the f32/f64 bit-width mismatch and the active `todo!()` in `kind()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Already recorded as E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| G3  | Type-name string dispatch, 11 sites (true positive for criterion D)                                                                 | **True positive, one common root cause**: there is no single authoritative identifier for built-in container names. List: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), the large table at `eval/const_eval.rs:498-511` (including the dead `"Uint"` arm), `to_type_id` at `middle/passes/codegen/bytecode.rs:744-756` (`"Set" => 25` contradicts decision #300), and `"Clone" \| "Dup"` at `trait_data.rs:187`                                                                                                                                                                                                                                                                                                                                  | **P6 extended verification item**: these sites are on the typecheck / eval / middle sides and are not covered by the parser data-flow scope in 03 §5.4. During P6, handle them under the same principle as 03 §5.3 (`NameKind` is determined once; match arms no longer perform string comparisons). The copy table has already been corrected to seven sites (03 §2.3)                                                                                                                                                                                                                                                                   |
| G4  | Four criterion D false positives                                                                                                    | **Legitimate lookup / dispatch tables, not consolidation targets**: tool-installation instructions at `package/build/requirements.rs:17-25`, the build-result display name at `package/vendor/mod.rs:134-139`, REPL command aliases at `repl/mod.rs:241-396` (8 arms; user-interface aliases are legitimate UX), and `parse_runtime_mode` at `util/diagnostic/mod.rs:462-468` (a single boundary point for historically commented aliases)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | When P9 is converted to a hard gate, add a `// reason:` exemption at each site; review the exemption count as part of the gate report                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| G5  | `ReleasePlan.drops` span-key mismatch (measured by the criterion in P2 2.4.4; not a P0 finding)                                     | **True positive, contractual defect—the empirical fulfillment of D20**: ownership records keys by the span of the last use of an **expression** (`ownership.rs:2574 build_release_plan`), whereas `ir_gen.rs:1956` looks them up by the span of a **statement**; expression-level keys never match—across the entire corpus, **168 files / 373 keys** are unconsumed, and Drops are silently lost (see the file header of `src/middle/core/tests/release_plan_spans.rs` for per-case evidence)                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | **P4/D20** (Span key → PlanId; behavioral surface of approximately 168 files; must wait for the C2 criterion and corpus differential). The `test_release_plan_spans_consumed` criterion remains permanently red (`#[ignore]`), and turns green when D20 is implemented                                                                                                                                                                                                                                                                                                                                                                    |
| G6  | Three IR metadata defects (empirically observed while the P2 2.1.2 `verify_loose` corpus passed; fulfillment of the D38 prediction) | **True positive ×3, all fixed in the 2.1 batch** (corpus differential baseline compared against itself with zero differences, proving behavioral equivalence): ① The method IR locals table is truncated to the parameter count (`ir_gen.rs` originally used `take_cur_locals(param_types.len())`)—the local count is underestimated, and the runtime silently tolerates it by resizing through `Frame::set_slot` (`frames.rs:103-105`); ② Unannotated parameters in the lambda / curry intermediate / function paths are discarded by `filter_map` (three sites), causing the parameter count and frame layout to become misaligned—fill the existing method-path convention with an `Int(64)` placeholder; ③ The match result slot depends on the frame’s implicit Void pre-initialization (the fallback landing point when the final arm’s guard fails has no defined path on the CFG)—explicitly pre-initialize Void at the `generate_match_expr_ir` entry point | Fixed in the same 2.1 batch. **The assessment scope is narrowed and registered here** (`verify.rs`’s file header is authoritative): temp-slot dominance is not checked (Void pre-initialization is frame semantics that `ir_gen` intentionally depends on); trusted type sources are narrowed to the parameter-slot signature + constants (`ir_gen` never writes `LocalSlot.ty`, and the entire table uses the `Int(64)` placeholder—the complete assessment scope belongs to P7 value table 04 §262); `init_locals` is the #368 debug-view table, not an allocation upper bound, so an `init` Local that is out of bounds is not checked |

## H. Inventory of Giant Functions (Pre-Release Audit Metric, Effective 2026-10-16)

> Metric (defined by `/pre-release-audit` on 2026-10-16): a function span > 150 lines is an
> **initial-screening threshold**, not a command to report every function individually—the full list
> would turn the inventory into a pile of line counts and drown out the new signal in the existing
> backlog. Every report entry must contain three numbers: **host line count / lines inserted this
> time / line count before insertion**. Divide the entries into two categories: **A. Existing giant
> functions with inserted branches** (>150 lines and >50 lines inserted this time); **B. Newly added
> large functions** (almost entirely new and >100 lines). The line numbers below are taken from
> audit revision `1935aab53` (range `v0.8.4..1935aab53`).

### A. Existing Giant Functions with Inserted Branches

| Function                | Location                                                    | Host Lines | Inserted This Time | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | ------------------ | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1,594      | 358                | 1,236            |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171                | 642              |

### B. Newly Added Large Functions

| Function                     | Location                                            | Lines | Newly Added |
| ---------------------------- | --------------------------------------------------- | ----- | ----------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177         |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122         |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130         |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107         |

**Existing giant functions not included in the inventory** (≤ 4 lines inserted this time; recorded
but not acted upon): `infer_expr` (`expressions.rs:2999`, 955 lines), `collect_function_signature`
(`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`, 441 lines),
`generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), and others.

### Disposition Direction

- **Category A**: Resolve through the follow-up decomposition tasks for `infer_call_expr` /
  `generate_call_expr_ir`—these are the two heaviest hosts in the repository, and the current
  named-argument refactoring inserted 358 lines into the former.
- **Category B**: `push_joined` is a **local** helper for formatting a `Task` work-list-style type
  (its host is the new 180-line function); its responsibilities differ from
  `mono.rs:123 increment_level_string` (174 lines, which rewrites hierarchical names), so **do not
  merge them**. `consumer_stage_skipped` and `local_count_of` can be decomposed independently.

## Key Decisions and Rationale

The authorization is “permission to delete outright,” but this document gives **different
recommendations** for several items:

| Category                                                               | Disposition                                     | Rationale                                                                                                                                                                                                                                                                                                                                             |
| ---------------------------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions or implementation) | **Delete**                                      | B1 (415 lines), B3 (the dispatch chain), B6 (96 lines), B7 (empty functions), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                                                     |
| **Paper-only design documents**                                        | **Delete or rewrite**                           | After deleting C1 and C2, rewrite them to point to the actual implementation; rewrite the boundary table in C3. **Must clean up `config.js:259-268` in sync** (E6)                                                                                                                                                                                    |
| **RFCs whose status does not match the implementation**                | **Change the status**                           | Move C4 (RFC-018) back to `draft` or add `impl_status: 'not-started'`; add `impl_status: 'not-started'` to C5 (RFC-028). **Do not delete them**—the design itself may be valuable; the problem is the inaccurate status marker                                                                                                                        |
| **Orphan tests with real assertions**                                  | **Revive, do not delete**                       | **1,005 lines / 78 tests** in total: lexer 629 lines / 55 + 311 lines / 16 hidden inside live directories (`precedence_inline` / `overload_inline` / `json`) + 65 lines / 7 under `package/template/tests/`. **The cost of revival is a one-line `mod` declaration; the benefit is that CI regains this coverage**. Deleting them would be a net loss |
| **Zero-call `pub` items with important semantics**                     | **Reduce visibility rather than delete**        | B2 (`TypeSystem`) and B4 (`check_type_equivalence`)—move them to `pub(crate)` so the compiler can identify them, or implement them. **The `is_subtype` in B4 must be retained**                                                                                                                                                                       |
| **Opcodes / instructions that cannot produce a value**                 | **Add construction sites or delete the opcode** | B8 (`TailCall`), B9 (`Switch`, whose round-trip test explicitly registers it through `WHITELIST=["SWITCH"]`), and B10 (`UnaryOp::Not` silently degrades). Choose one of the three; the status quo cannot be maintained. D32/D33/D34 have already decided to delete them, to be handled together with the `.42` version bump                           |
| **Data-loss defects**                                                  | ~~Separate issue~~ **P7 batch e (D52)**         | The three hard-coded discards in Section E are not dead code                                                                                                                                                                                                                                                                                          |
| **Valid but lacking module identity**                                  | **Transform, do not delete**                    | F1 (`semantic_tokens.rs`): change `include!` → a real `mod` (see `02`)                                                                                                                                                                                                                                                                                |
| **Requires a decision**                                                | **List as an open question**                    | F2’s wasm32 cleanup has been registered as [09](09-execution-wbs.md) §P10 **S7** (gate the project-mode subtree on wasm32; user decision B dated 2026-10-18; includes the three unverified items verbatim). Layer-order correction will expose new diagnostics                                                                                        |

### Directions Not Adopted

- **Treat everything as “delete outright,” including orphan tests**—the 1,005 lines of tests with
  real assertions are an asset. The deletion cost is a one-line `mod` declaration, while the benefit
  is that CI regains the coverage. The authorization means “may be deleted,” not “should be
  deleted.”
- **Only add gates without changing code**—gates can prevent regressions, but B1’s 415 lines, the
  dispatch chain, and the 96-line dead ladder **already exist**; gates would merely freeze them into
  the baseline.
- **Keep the `accepted` status of RFC-018 / RFC-028 and only supplement the
  documentation**—`rfc/index.md:131` defines “accepted” as “enters the implementation stage,” so
  readers would reasonably assume an implementation exists. `TRACKING.md` already provides precedent
  for the `impl_status` field (5 RFCs use it), and the cost of adding the field is far lower than
  the cost of a misleading status.

## Implementation Points

**This document does not require an equivalence oracle** (it belongs to the
[C6 pure-deletion category](07-equivalence-oracle.md)), but **each item must still be confirmed to
have no references**. In the global stage sequence of RFC-039, S1 corresponds to P1, and S2-S6
correspond to P10.

| Stage | Content                                                                                                                                                    | Acceptance                                                                                                                                                                                                                                                                                            |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphan tests with assertions (**1,005 lines / 78 tests**)                                                                                           | The test counts for the lexer / pratt / passes / emitter / template modules increase under `cargo test`; **real defects are expected to be exposed** (the overflow path in `literals.rs` and the illegal `\x`/`\u` escape sequences were previously never tested), so time must be reserved for fixes |
| S2    | Delete pure-placeholder dead code (B1/B3/B6/B7 together with the 05 cleanup/C6/E1)                                                                         | `cargo test` passes; `cargo clippy -D warnings` reports no new warnings                                                                                                                                                                                                                               |
| S3    | Dispose of paper-only design documents (C1-C3 + clean up `config.js` in sync with E6)                                                                      | The documentation site builds without 404s; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                                                   |
| S4    | Correct RFC statuses (C4/C5/F3—add an “Implementation Status” column to `TRACKING.md`)                                                                     | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` is regenerated automatically; **do not edit it manually**)                                                                                                                                                                               |
| S5    | Reduce the visibility of `pub` items (B2/B4)                                                                                                               | The compiler can identify genuine dead code                                                                                                                                                                                                                                                           |
| S6    | Implement the decisions for opcodes that cannot produce values (B8/B9/B10/F8, following D32/D33/D34)                                                       | Delete the `Switch` / `TailCall` dead paths; distinguish the `op` field in B10’s `opcode()`; implement the F8 gate                                                                                                                                                                                    |
| S7    | Convert `include!` to a module (F1; construction steps assigned to [09](09-execution-wbs.md) §P5 5.1) + correct the layer order (F4, assigned to P4 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                                                                            |

**S1 must be performed first, and time must be reserved for defect fixes.** Its value lies not in
“cleanup,” but in **allowing CI to regain 1,005 lines / 78 tests that have never run**—until then,
every judgment about test coverage rests on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners in `literals.rs` have four isomorphic
  branches that “continue consuming after an overflow without reporting an error,” the entire
  `scan_leading_dot` function, and the illegal `\x`/`\u` escape paths—all have never been tested.
  After the tests are revived, the code will probably need to be fixed, delaying the subsequent
  refactoring.
- **B9/B10 involve language-behavior decisions**. Whether to “add construction sites” or “delete the
  opcode” for `Switch` and `UnaryOp::Not` requires considering whether published corpora depend on
  their external behavior.
- **After C1-C3 are deleted, documentation of cross-file analysis will be temporarily absent** until
  `check/cross-file-analysis.md` is rewritten to point to the actual architecture under
  `src/frontend/module/`.
- **Reachability analysis can only identify “unreachable” code; it cannot identify “reachable but
  meaningless” code.** Judgments beyond Form 2 must be made item by item, and **this inventory does
  not constitute exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2 and B4)
  cannot be discovered automatically by the compiler and must be found through searches.
- **Deleting the half-finished residue of RFC-023** (B1) fulfills the decision that “closures have
  been removed from the language”; **correcting status inaccuracies** (C4/C5/F3) restores the
  reference value of `TRACKING.md`.

> **All open questions originally listed in this section have now been adjudicated.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> decisions (D1–D50). **This document retains no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  the source decision behind the B1 residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — the unwired `§4.1` runtime check insertion in B3
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — the formal home of C1/C6
- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — the overarching framework
- [Refactoring Equivalence Criteria](07-equivalence-oracle.md) — criterion requirements for category
  C6
- `src/frontend/core/typecheck/layers/README.md` — the actual proof-layer call order and the true
  identity of `equivalence` (F4 has been handled)
- `build.rs:19-55` — an example of this repository’s generation-time gate
