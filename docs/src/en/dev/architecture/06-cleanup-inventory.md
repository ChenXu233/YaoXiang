# Dead Code and Hollow Design Cleanup

> **Companion design document**. This is a companion document to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution-phase order are covered in the main
> RFC-039 text; see [this directory's index](index.md) for the role of each companion document.

## Purpose and Scope

This document provides an inventory of dead code and hollow designs in the YaoXiang repository, the
methods used to identify them, the disposition decision for each item, and the execution order.
Cleanup targets fall into four categories:

| Category                                                                  | Scale                                                               |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Files that have never participated in compilation                         | 24 files / 2628 lines, of which 23 are genuine orphans / 1081 lines |
| Dead code that is compiled but never called                               | 1500+ lines                                                         |
| Design documents describing an architecture that never existed            | Three documents under `docs/src/dev/design/check/`                  |
| Features attached to the default entry point, with no tests and no output | `src/repl/`, 1152 lines                                             |

**The authorized scope is “complete deletion is permitted”** (decision dated 2026-10-03). However,
this document recommends dispositions for several items that are **the opposite of “delete
directly”**—the rationale is given under [Key Decisions and Rationale](#关键决策与理由).

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), or criterion design (see `07-equivalence-oracle.md`).
B3 records that the entire `layers/dispatch.rs` chain is unwired; its deletion is assigned to S2.

## Current State

### Three Forms of Dead-Code Debt

**Form 1: Placeholder.** The code exists and appears to be doing something, but does nothing.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the “runtime check insertion” mechanism designed in `RFC-027 §4.1` and is
  **completely unwired**.
- `check_type_equivalence` (`ProofContext` entry point) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is called only by tests. **Note** that
  `is_subtype` in the same file is used in production by `inference/assignment.rs:17`, so **the
  entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1047 lines) is a pure DAG task scheduler. Its `pub` surface
  consists only of `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It **is not a second execution backend**—the
  `Executor` trait at `backends/mod.rs:356` currently has only one implementor, the interpreter.
  This point needs clarification because the directory name is misleading.

**Form 2: Residue.** An upstream decision has already been made, but deletion was only half
completed.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) contains six residual types:
  `FunctionInstance` (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), and `CaptureVariable`. **The source is clear**: when `RFC-023`
  deprecated the “closure capture model,” it declared that “**approximately 850 lines of code would
  be deleted along with the closure model**.” These 415 lines are the residue that survived from
  those 850 lines—**deletion was only half completed**. **Verification**: A repository-wide
  word-boundary search for these six types returns **zero matches** outside `instance.rs`, including
  in tests. The glob re-export at `middle/mod.rs:40`, `pub use passes::mono::instance::*;`, merely
  passes the items along and does not constitute consumption.

**Form 3: False Coverage.** This is the most dangerous form because **it makes the repository appear
more reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only five `pub mod` declarations (`literals` / `state` /
`symbols` / `tokenizer` / `tokens`) plus one
`#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;` (`:104-106`). **Nowhere is
`mod tests;` declared**, so the entire `lexer/tests/` subtree of 13 files and 686 lines **has never
participated in compilation**.

Seven of them are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
`delimiters.rs` 3 / `errors.rs` 3 / `keywords.rs` 3 / `operators.rs` 3, totaling 19 lines and 0
assertions). **629 lines / 55 genuine tests have never run**:

| File              | Lines   | `#[test]` |
| ----------------- | ------- | --------- |
| `literals.rs`     | 222     | 30        |
| `rfc010_lexer.rs` | 167     | 12        |
| `lexer_mod.rs`    | 89      | 5         |
| `rfc004_lexer.rs` | 81      | 5         |
| `symbols.rs`      | 70      | 3         |
| **Total**         | **629** | **55**    |

(`fstring.rs`, 90 lines / 11 tests, is indeed compiled, but through the `#[path]` bypass rather than
`mod tests;`; `mod.rs`, 38 lines, is a declaration file with no assertions.)

**Only one file, with 12 tests, is actually running at the lexical layer, and every test covers only
the f-string brace raw protocol.**

The danger of this class of problem lies in the fact that `cargo test` does not fail—tests that
never ran cannot fail—and CI has no test-count baseline or wiring check.

### Hollow Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not a case of “the design was completed but not implemented”; rather, **the
documents describe an entirely different system that never existed**, while presenting “a possible
future design” as a defect in an implemented system.

## Methodology and Confidence Statement

### Reachability Analysis Method

Starting from the two crate roots, `src/lib.rs` and `src/main.rs`, all `mod` / `pub mod` / `#[path]`
declarations in 509 `.rs` files are parsed, a module graph is constructed, and breadth-first
reachability analysis is performed. `include!` targets are evaluated separately and are not counted
toward reachability.

**This method has known false-positive patterns, two of which were actually triggered while writing
this document:**

1. `Split-Path` returns `\` on Windows, which does not match forward-slash keys—this temporarily
   produced a false report of 499 orphans.
2. **The child-module directory of a non-`mod.rs` file is `自身名/` rather than `父目录/`**—the
   `mod tests` in `orchestrator.rs` was resolved to `src/frontend/module/tests/`, nearly classifying
   753 lines of live tests as dead code.

Therefore, **every conclusion was double-confirmed by “actually reading the file + actually
searching for references”**, and entries that did not pass secondary confirmation were not included
in this document.

### Line-Count Definition

All line counts in this document were obtained with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines and systematically undercounts by about 5%** (for example,
`manifesto.md` is reported as 658 with `-Line`, but actually has 893 lines). An earlier round of
analysis in this repository produced a lower total because of this counting method; the figure has
been corrected here.

### Known Limitations

- **Reachability analysis can discover only “unreachable” code, not “reachable but meaningless”
  code.** Forms beyond Form 2 require item-by-item manual judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types must be
  found through searches; this method does not cover them.
- This document does not involve running the compiler, so all “has never run” conclusions are based
  on static analysis.

## A. Files That Have Never Participated in Compilation

**Total: 24 files / 2628 lines.** `semantic_tokens.rs` belongs to Form 4 (see Section F); the
remaining **23 files / 1081 lines are genuine orphans**.

| Path                                                          | Scale                   | Content                                                                          | Basis for Determination                                                                                                                                                                                    |
| ------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entire `src/frontend/core/lexer/tests/` subtree, **13 files** | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 genuine tests** | `lexer/mod.rs:4-8` declares only five `pub mod` declarations; the only test declaration at `:104-106` is a `#[path]` to `tests/fstring.rs`. A repository-wide `grep 'mod tests'` matches only `mod.rs:105` |
| 3 files under `src/frontend/pipeline/tests/`                  | 14 lines                | All are placeholder documentation comments, with 0 assertions                    | `src/frontend/pipeline.rs` (740 lines) contains no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration (`Select-String` returns zero matches)                                                       |
| 3 files under `src/package/template/tests/`                   | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]` functions**              | `src/package/template/mod.rs` (7 lines) declares only `mod gitignore; mod main_yx;`, with no `mod tests;`                                                                                                  |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden inside a **live** directory                                               | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                                            |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                    | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                               |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                    | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                                       |

**The final three items total 16 tests / 311 lines of code that have never run.** They are hidden in
active directories, and the directories' `mod.rs` files look complete, making them more deceptive
than an entirely orphaned directory.

Two other files under `lexer/tests/` belong to the “declared but declaration omitted” form, but they
are already included in the 629 lines in the first row and **are not counted again**: `lexer_mod.rs`
(89 lines / 5 tests; `lexer/tests/mod.rs` declares 11 `mod` declarations but has no
`mod lexer_mod;`) and `symbols.rs` (70 lines / 3 tests, likewise).

**Total genuine tests that have never run: 1005 lines / 78 tests**: lexer 629/55 + pratt / passes /
emitter 311/16 + package/template 65/7.

**Pay particular attention to `precedence_inline.rs`**: it tests the **production-zero-use**
`Precedence` enum and `PrecedenceContext` in `precedence.rs` (96 lines of dead code). Even if it
were wired up, it would still be testing a dead path.

Another item hidden among the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, 0 tests): `typecheck/tests/mod.rs` declares 23 test modules but omits this one. The file
contains only one `use` line and is a pure remnant. During S1 reactivation, delete it alongside the
wiring or add assertions.

## B. Dead Code That Is Compiled but Never Called

| #   | Location                                                                         | Scale                   | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | -------------------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types     | A repository-wide word-boundary search returns zero matches outside `instance.rs`, including in tests. Source: RFC-023 declared the deletion of approximately 850 lines, but only half were deleted                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines                | `TypeSystem` and 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` + `:24` `pub use types::TypeSystem;` (both pass-throughs). Repository-wide, `TypeSystem` has only 5 matches: 2 definitions + 1 re-export + **2 occurrences of the unrelated** `lexer/symbols.rs:503 TypeSystemValidator`. The 4 methods have zero call sites. **Because they are `pub`, the compiler never reports them**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B3  | Entire `src/frontend/core/typecheck/layers/dispatch.rs` chain                    | —                       | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` have zero call sites. RFC-027 §4.1's runtime check insertion is unwired. Note that it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                       | `check_type_equivalence` is called only by tests. **The `is_subtype` in the same file must be retained** (used in production by `inference/assignment.rs:17`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2              | Self-described as a “framework placeholder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines                | `Precedence` enum + `PrecedenceContext`. **Its only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests), and that test file has never itself been compiled** (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore, **both must be deleted together**: deleting only the tests leaves 96 lines of dead code, while deleting only the enum causes the tests to fail to compile                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | Approximately 120 lines | `is_old_function_syntax` is called at `:719`, and `skip_old_function_syntax` is called at `:725`—**they are live old-syntax rejection gates, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) contains only the comment `// 旧语法已移除`, but **the function does not consume any tokens when called**—this is an independent behavioral issue (whether the rejection path correctly advances the parser position), not dead code. **The disposition is to review the behavior together with 05-frontend-paradigm.md (Decision D27: remove naturally as part of the grammar migration), rather than delete it directly as dead code**                                                                                                                                                                                                                                                    |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                       | The only repository-wide occurrences are match arms, with no construction site. The “no generation site (dead path)” comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| B9  | `Switch` opcode                                                                  | —                       | `backends/common/opcode.rs` defines it, and it has a variant, `opcode()`/`size()` arms, and decoding arms, **but there is no construction site anywhere in the repository**. The interpreter has a live implementation at `ops/control.rs:90`. **It is explicitly registered as a known dead path by tests**: `WHITELIST: &[&str] = &["SWITCH"]` at `tests/bytecode.rs:1035-1040` (the check fails when the set difference is nonempty)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B10 | `UnaryOp::Not` (bytecode unary logical NOT)                                      | —                       | **Fixed (WBS 3.20.29, 2026-11-14, issue #562 closed)**. The original defect was that the `opcode()` encoder ignored the `op` field (Not and Neg both emitted `I64_NEG`), and the decoder constructed only `Neg`—the encoding/decoding round trip rewrote logical NOT as negation (`!x` became `-x` when running `.42` directly; the interpreter's in-memory Neg-on-Bool compensation arm happened to produce the correct answer, so the defect affected encoding fidelity rather than the current result). The fix adds a dedicated instruction code, `I64_NOT` (0x2C): `opcode()` branches on the `op` field + the decoder gains an arm that restores `Not` + the translator's `Not` arm emits it + `opcode_name` is registered; the executor's existing `Not` execution arm connects naturally; the old `.42` `I64_NEG` decoding behavior remains unchanged. See the task 3.20.29 commit message for the criterion chain |
| B11 | Four instances of `#[allow(dead_code)]`                                          | —                       | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (both are genuine dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (a conservative exemption for an extern binding, **reasonable to retain**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                       | **Zero production construction sites** (the only construction site manually creates a node in the test at `parser/tests/ast.rs:828`), but there are **12 production consumers + 2 exhaustive match arms** (the complete list is in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()` and `pratt/mod.rs:47` `expr_end_line`. Actual function definitions use `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion is assigned to P6 (the 16-item change list in 03 §5.5)                                         |
| B13 | `Instruction::Cast` / `Instruction::TypeTest` (`src/middle/core/ir.rs`)          | —                       | The only occurrences repository-wide are consumer arms, with **no construction sites** (the same form as B8), and the payloads are discarded downstream—`translate_cast` cannot obtain `target_type`, and the `TypeTest` arm emits the no-op operand placeholder `vec![0, 0, 0]`. **WBS 6.6.11 deleted them** (both variants + all `normalize`/`verify`/`mono`/`translator` consumer arms + the consequently orphaned `substitute_block`/`substitute_instruction`/`substitute_type_ast`/`translate_cast`); grep finds zero matches for this form in 216 IR snapshots. **The deletion does not affect syntax-level `as` support**, but makes the existing gap—“`as` semantics are not implemented”—more visible. Therefore, the gap and evidence chain are registered in the task 6.6.11 commit message; see the SPEC-side note in `docs/src/reference/language-spec/syntax.md` §2.6                                        |

## C. Hollow Designs

| #   | Object                                                                          | Scale                                      | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                   | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, and `HotReloader` have **zero repository-wide matches**. The “known limitations” at `:46-47` admit that `check_incremental` still calls `check_files_with_diagnostics` internally (the full path); `:46` also cites `command.rs`'s busy-wait debounce—yet **`command.rs` contains neither `Instant` nor `recv_timeout`** (repository-wide, `recv_timeout` appears only in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                   | `:37` claims that the `traits/` placeholder implementations (coherence / impl_check / object_safety / resolution) are incomplete—**the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero repository-wide matches. `:38` claims that `check_single_module` creates a separate Compiler for each file—**the function does not exist** and appears only in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                             |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                   | `:14` claims that “watch mode rechecks only affected files,” and `:27` claims that “check performs only the first two steps.” In reality: `check_project` also performs vendor consistency checks, project-role classification, dead-code analysis, and W1006; the `Check` subcommand in `main.rs` has **no `--watch` flag** (every `#[arg]` declaration was checked individually)                                                                                                                                                                  |
| C4  | `RFC-018` LLVM AOT                                                              | 1037 lines of design / **0 lines of code** | All AOT matches at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **comments**. `Cargo.toml` contains no cranelift / llvm / inkwell. `:861-876` retains an implementation checklist in which every item is `- [ ]`, yet the status is marked `accepted`                                                                                                                                                                                                                                                         |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**  | `028-jit-compiler.md:99-126` requires 6 files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`, **none of which exist**. The status is `draft` (comparatively honest)                                                                                                                                                                                                                                                                                                                                                                    |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                               | **Physical evidence**: the filenames correspond precisely to the `ModuleCache` and incremental scheduler in the C1 design. **Someone created test skeletons according to the design, found that the pipeline had no corresponding implementation, and left them unfinished**                                                                                                                                                                                                                                                                        |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                          | The `//! §6.1: CheckSession 增量检查` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` point to this **gitignored plan document** (`.gitignore` includes `superpowers/`). These § references are **archaeological references to deprecated APIs**                                                                                                                                                                                                                                                         |

### Actual Implementation Location of Cross-File Analysis

The cross-file analysis capability described by C1-C3 **does genuinely exist**, but along an
entirely different path:

| Component           | Location                              | Lines                  |
| ------------------- | ------------------------------------- | ---------------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439                    |
| Resolver            | `src/frontend/module/resolver.rs`     | 195                    |
| Role classification | `src/frontend/module/roles.rs`        | 388                    |
| Consistency checks  | `src/frontend/module/consistency.rs`  | 206                    |
| Orchestration       | `src/frontend/module/orchestrator.rs` | 1528                   |
| **Total**           |                                       | **Approximately 2670** |

Therefore, the disposition of C1-C3 is **not to remove the capability, but to remove misleading
documentation** and rewrite the boundary table in `check/index.md` to point to the actual
architecture under `src/frontend/module/`.

### The Disposition of RFC-019 Requires Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` **is already in the `deprecated/` directory**,
and the residual review found it **clean** (`homoicon` has zero repository-wide matches, and the end
of the document contains the honest conclusion, “If it is restarted in the future, begin with
proof/prototype experiments, not with the RFC”). **It requires no additional disposition**—this item
is included in the authorization list, but review confirmed that it is already in its terminal
state.

## D. Feature-Level Dead Code: REPL

`src/repl/` consists of 4 files totaling **1152 lines**, with **zero `#[cfg(test)]` and zero
`#[test]`**. Meanwhile, `src/main.rs:440` is `args.command.unwrap_or(Commands::Repl)`—**running
`yaoxiang` without arguments directly enters the REPL**.

| #   | Fact                                                   | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed**           | It appears only twice under `src/repl/`: the definition at `eval.rs:23` and the **match arm** at `mod.rs:180`. Therefore, `Repl::format_value` (`mod.rs:493-502`) is dead code, and **the REPL never echoes any evaluation result**                                                                                                                                                                                         |
| D2  | **No state persists across rounds**                    | `extract_definitions` (`eval.rs:345-377`) calls only `define_variable` (which stores a **type string**), never `define_var` (which stores a `RuntimeValue`); `wrap_code` also does not inject historical definitions. The `VariableInfo::Value` (`eval.rs:23`) branch is unreachable. `REPLContext.variables` exists solely for completion and does not participate in evaluation                                           |
| D3  | **The wrapping syntax is suspected of being obsolete** | `eval.rs:314`/`316` produce `main() -> () = () => { code }` (the old-style `name() -> Ret` function declaration header), whereas the current specification form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`; the specification table in RFC-007:94 consists entirely of `name: (a: Type) -> Ret`). **The compiler was not run to verify this; if true, every REPL evaluation would fail to compile** |
| D4  | `:debug` is a stub                                     | `mod.rs:454-466` only prints two lines and returns; breakpoints are stored (`mod.rs:319`) but **the interpreter never queries them**                                                                                                                                                                                                                                                                                        |
| D5  | `:history` is a stub                                   | `mod.rs:387` prints “not yet implemented”                                                                                                                                                                                                                                                                                                                                                                                   |
| D6  | Duplicates `lib.rs`                                    | `eval_code` at `lib.rs:85` and `repl/eval.rs:wrap_code` are **two independent implementations of “automatically wrap main”**, and their wrapping forms differ                                                                                                                                                                                                                                                               |

## E. Local Cleanup Items

| #   | Location                                                                                                | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                              | Empty directory tree at the root. Almost certainly caused by concatenating the string `"undefined"` into a path somewhere. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| E2  | `src/lib.rs:568`, `:582`                                                                                | `dump_type_detail` / `dump_const_detail` each have a `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, and the panic path is predictable. Replace it with meaningful fallback output                                                                                                                                                                                                                                                                                                                                                                             |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                               | `check_vendor_lock_consistency` and `check_workspace_consistency` are **approximately 60 lines of near-duplicate code**, differing only in dependency source and in the two path-dependency skips. **Merge them**                                                                                                                                                                                                                                                                                                                                                                               |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`                  | Old function-syntax detection and rejection, approximately 120 lines. The body of `skip_old_function_syntax` (`:120-122`) contains only one comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                           |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                        | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                                 | The “tool design” sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, and `check/incremental-checking`. **If the C1-C3 document deletion is performed, this location must be cleaned up in parallel**, or the documentation site will contain 404 links                                                                                                                                                                                                                                                                                                            |
| E7  | Duplicate `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | 5 variants have the same names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` has inconsistent widths (f32 × f64)**; there is no `From` / `TryFrom`, and handwritten conversions are scattered across call sites (`const_eval.rs` / `ir_gen.rs`). `kind()` at `const_data.rs:44` has a live `todo!()` for `LibraryRef` / `ExternRef`. **Disposition: merge into one definition + add explicit conversions + replace `todo!()` with fallback output; assigned to P10 (Category E local cleanup item)**. Registered by the P0 gate on 2026-10-06 (Criterion A, J=0.56)    |
| E8  | Duplicate downstream mappings from `Literal` (Prohibition 3 A)                                          | At least 3 identical arms for `Literal → MonoType` (`Literal::Int(_) => MonoType::Int(64)` and similar in `inference/expressions.rs`, `checker.rs`, and `inference/patterns.rs`); 19 handwritten `Literal → ir::ConstValue` mappings in `ir_gen.rs` and 4 handwritten `Literal → const_data::ConstValue` mappings in `const_eval.rs`, with no shared conversion point. **Disposition: move the 3 typecheck-side locations to a shared layer under P6; consolidate the `ir_gen` / `const_eval` side together with the E7 `ConstValue` merge under P10.** Registered by the P0 gate on 2026-10-06 |

### Not Dead Code, but the Same Class of Problem: Three Hard-Coded Discards

The `impl From<BytecodeFile>` in `src/middle/core/bytecode.rs` (`943-2350`, 1400 lines) contains 3
hard-coded discards:

| Line   | Discarded Data                   | Consequence                                              |
| ------ | -------------------------------- | -------------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | Upvalue metadata is lost                                 |
| `2315` | `exception_handlers: Vec::new()` | **The `.42` artifact loses its exception table**         |
| `2341` | `globals: Vec::new()`            | **The `.42` artifact loses global-variable information** |

**These three locations are not dead code; they are data-loss defects**—`.42` artifacts are complete
only along the in-memory path. **Their disposition differs from that of dead code**: ~~they should
be tracked as separate issues~~ **they have all been incorporated into P7 batch e (Decision D52,
2026-10-05), with no separate issues remaining**—`2315` (exception table) and `2341` (global
variables) concern artifact integrity for core language semantics and are not “deferrable” items.
All three are fixed together, and the `.42` `VERSION` is raised from 4 to 5 in one step (the format
header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`). They are listed here
only to prevent them from being mistakenly cleaned up as “dead code.”

## F. Do Not Delete; Mark or Decide Instead

| #   | Object                                                                                                                                                                                                                                                                 | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (**handled under P5 5.1**: 1547 lines at the time, now 1553 lines and a genuine `mod`)                                                                                                                        | **It is not dead code.** At the time, it was spliced into the end of `checker.rs` via `include!` (the repository's only `include!`); the file **was not declared as any module** (`grep 'mod semantic_tokens'` returns zero repository-wide matches), and its first line was directly `impl TypeChecker {`, with no `use` header of its own. The `checker/` directory contained only this one file. Consequences included the absence of module identity, failure of visibility isolation, broken rust-analyzer navigation and symbol search, and the toolchain treating the file as part of `checker.rs`. The actual size of the `checker` module at the time was 5618+1547 ≈ 7165 lines (now `checker.rs` 4167 lines + three `checker/` submodules of 1553/1107/470 lines). **Fix: convert it to a genuine `mod`** (completed under [09](09-execution-wbs.md) §P5 5.1 on 2026-10-26; the B1 inventory for `check-boundary` is now zero), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F2  | wasm attributes (both positive and negative forms on the same line are counted): in the current tree, 29 files under `src/` contain the literal `wasm32`, 22 have actual attributes, and 7 only mention it; `orchestrator.rs` has 10 entries (4 positive + 6 negative) | **This is not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, and `:17` a dependency on the main crate; the root `Cargo.toml:3` `exclude = ["wasm", ...]` explicitly confirms that it is an independent crate. Looking only at the root `Cargo.toml:31` `crate-type = ["rlib"]` would incorrectly classify these branches as defensive dead code—**that inference is invalid**. CI's wasm build is at `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), invoked by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches are load-bearing**: they determine which APIs of the main crate can be called by `wasm/src/lib.rs` (now **152** lines) under the wasm target (`src/lib.rs:151` / `:170` / `:187` gate `run_file` / `run_project` / `build_bytecode` as a whole). **The cleanup scope has been determined**: four wasm arms are unreachable at runtime in the playground scenario but load-bearing at compile time (their call sites are not gated); six `not(wasm32)` arms are load-bearing on the native side across the three stage tables ⇒ **there are no directly deletable low-risk items**—the item-by-item determinations and evidence are given in the “F2 Determination” section below (Decision D35: delete unreachable items and retain reachable ones; assigned to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                                                                                                                                                                                                         | It has only a “Status” column (document status), with **no “Implementation Status” column**, so it **cannot represent** the error “an RFC is marked complete but the code does not exist.” Only 5 of 52 repository RFCs have an `impl_status` field (`027` in-progress; `017`/`014`/`014b` complete; `review/027a` partial). **26 accepted RFCs do not record implementation status at all**, including 009, 009a, 013, 018, and 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                         | **Handled (4.4.1)**: the README now describes the actual call order of `checker.rs::check_module_impl` (refinement traversal `predicate` → `termination` → `ownership` → type-constraint solving), with parenthetical line numbers taken from the current tree and function names/driver points used as anchors; it no longer claims that “a lower-layer failure prevents upper layers from running” (there is actually no short-circuit); `equivalence` is described by its true identity as a “shared type-comparison utility + proof entry point used only by tests.” This entry is retained only as a record of the completed disposition and no longer points to the old line range                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                                | `:79` places RFC-026a in the “Accepted RFCs” table, but the status column says “RFCs under review”; the “RFCs under review” subsection at `:48-49` lists only 032 and 027a, omitting 026a. This is inconsistent with the three review entries in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| F6  | `tools/cargo-dist/` (`dist.exe`, 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                            | **Neither is tracked by Git** (`git ls-files` returns empty), and both are excluded by `.gitignore`. **This is not a problem**; it is recorded only to prevent a later audit from mistakenly concluding that “binaries were committed”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                   | `extract_code_from_entry_line` is a **line-start prefix matcher** (`trim_start().strip_prefix("(")` up to the next quote), and depends on the `("E1001", ...)` tuple syntax; **it is not a parser**. Its architecture (`parse` + `validate` + a build.rs gate + `--fix` remediation) is worth reusing for the opcode table, but **the extractor must be rewritten for the opcode form (approximately 30 lines)**. This belongs to follow-up work in 06-cleanup-inventory.md or to a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                                                                                                                                                                                            | 83 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode facts are expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / decoding matches (`bytecode.rs:978-2302`) / 48 `translate_*` functions (`translator.rs:676-1577`). **The compiler enforces only 2 of them** (`backends/.../executor/debug.rs:194` dispatch table + exhaustive matches in the `ops/*.rs` family functions); a missing location in `opcode()` / `size()` / decoding arms / encoders **crashes only when a `.42` artifact is run**. The comment in the `size()` table (`649-803`) states that it differs from the actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic differential test** (existing size tests cover only Nop/Mov/Borrow/Release, four cases). Operator semantics also follow a third parallel enumeration here (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`; `Rem`/`Xor`/`Sar` use yet another naming set) and will be consolidated as part of the opcode-generation-time gate. The target location for the vocabulary is `middle/bytecode/opcode.rs` (migrating with the bytecode-domain consolidation and eliminating the L3→L4 reverse dependency; see 01)                                                                                                                                                                                              |

### F2 Determination: Reachability of wasm Attributes in `orchestrator.rs` (Determined 2026-10-18; No Cleanup Included)

**Determination surface** = wasm production build (the shim crate built by `wasm-pack`; see the F2
row); **determination revision** `b5e68b3c9`. This section records only the determination results
and makes no cleanup commitment (cleanup is assigned to P10 / D35).

#### I. Counting Basis (Retaken from the Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- The current `orchestrator.rs` contains **10 wasm attributes** in total = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- The counting basis is **the `#[cfg(...)]` attribute text on the same line**, counting both
  positive and negative forms (this is also the basis for the “12” in 02); runtime macros
  `cfg!(...)` are not counted—a repository-wide
  `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'` returns zero matches.
- The “12” is not an error; it is **a fact from an older revision**: at `544695ea9` (when the
  RFC-039 design document was first committed), there were 5 positive + 7 negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- The difference comes from **a pair of attributes being migrated out together**: `6ec330d22` (WBS
  4.2.2, where `check_project` was slimmed down to a `Program` constructor; the commit removed a net
  252 lines from `orchestrator.rs`) moved `role_context`, together with its `not(wasm32)`
  implementation and `wasm32` stub, to `src/driver/helpers.rs:360/394`—so `orchestrator.rs` dropped
  from 12 to 10, while the mechanism itself did not disappear.
- The revision-by-revision review (actually executed for this document; excerpts below, with
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

#### II. The Three-Question Criterion (Without Separating It, Load-Bearing Arms Will Be Misclassified as Dead Code)

1. **Compile-time presence**—does this arm participate in compilation under the target?
2. **Runtime reachability**—does a call chain from the playground entry point exist under that
   target?
3. **Deletion cost**—does deleting it break compilation for that target (are the call sites equally
   gated)?

#### III. Starting Point (Playground Entry Point and Stage Tables)

- Entry points: `wasm/src/lib.rs` (now 152 lines) `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, which constructs only `Pipeline` and does not
  touch the orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` follows `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`;
  `SingleFile | WasmPlayground` appears in the same table): Parsing / Typecheck / DeadCodeAnalysis /
  ProofExecution / IrGeneration / Monomorphization—**excluding** VendorConsistency / Discovery /
  Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### IV. Item-by-Item Determination (10 Entries in the Current Tree)

| #   | Location (`orchestrator.rs`) | Gated Object                                                             | Compiled in wasm Build              | Reachable at Runtime in Playground                                                                                                                                                                            | Deletion Cost (Are Call Sites Gated?)                                                             |
| --- | ---------------------------- | ------------------------------------------------------------------------ | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| 1   | `:271` positive              | wasm arm (`Ok(None)`) of `ensure_vendor_consistency` (`:267`)            | Yes                                 | **No**: the only call site, `src/driver/arms.rs:1195` (the VendorConsistency arm), is not in the playground stage table                                                                                       | High: `arms.rs:1195` is not gated; deleting the arm alone ⇒ wasm compilation fails                |
| 2   | `:418` positive              | wasm arm of `discover_with_used` (`:400`), which empties `lock_versions` | Yes                                 | **No**: call sites are the Discovery arm at `arms.rs:1221` and the Check-form reference pool `collect_package_refs` at `src/driver/helpers.rs:686`, which is called only by the Check branch at `arms.rs:521` | High: Same as above                                                                               |
| 3   | `:655` positive              | wasm stub of `member_context` (`:656-659`)                               | Yes                                 | **No**: the only call site is `:404`, inside the body of #2                                                                                                                                                   | High: `:404` is not gated                                                                         |
| 4   | `:753` positive              | wasm stub of `dep_import_surface` (`:754-756`)                           | Yes                                 | **No**: call sites are `:671` (`resolve_in_package_root`) and `:813` (`resolve_in_vendor_with_lock`), rooted at `:468` (`resolve_module_path`) in the BFS of `discover_with_used`                             | High: Same as above                                                                               |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                         | No (not included in the wasm build) | Not applicable                                                                                                                                                                                                | Load-bearing on native: used by `:678` / `:747`                                                   |
| 6   | `:276` negative              | native arm of `ensure_vendor_consistency`                                | No                                  | Not applicable                                                                                                                                                                                                | Load-bearing on native: VendorConsistency appears in three stage tables at `program.rs:61/98/113` |
| 7   | `:413` negative              | native arm of `discover_with_used` (lock reading)                        | No                                  | Not applicable                                                                                                                                                                                                | Load-bearing on native: Discovery appears at `program.rs:62/99/114`                               |
| 8   | `:607` negative              | native `member_context` (workspace resolution)                           | No                                  | Not applicable                                                                                                                                                                                                | Load-bearing on native: call site `:404`                                                          |
| 9   | `:677` negative              | manifest-name reading in native `resolve_in_package_root`                | No                                  | Not applicable                                                                                                                                                                                                | Load-bearing on native: `:544/:547` ← `:468`                                                      |
| 10  | `:745` negative              | native `dep_import_surface`                                              | No                                  | Not applicable                                                                                                                                                                                                | Load-bearing on native: `:671` / `:813`                                                           |

#### V. Conclusions

1. **The four wasm arms are unreachable at runtime in the playground scenario** (#1-#4; evidence =
   the stage table excludes the stages containing their sole call sites), but they are
   **load-bearing at compile time**. Literally executing D35's “delete unreachable items” would fail
   to compile—the feasible form is to “gate the entire project-mode subtree (`orchestrator.rs`'s
   discovery/resolution functions + the driver's VendorConsistency/Discovery arms + the
   reference-pool collection in `helpers.rs`) on wasm32”; cost and benefit are assigned to P10.
2. **The six `not(wasm32)` arms do not exist in the playground build** (excluded at compile time),
   and all are load-bearing on the native side (the MultiFile / Check / Lsp stage tables). The
   criterion “unreachable in the playground ⇒ delete” **does not apply** to them: deleting under
   that criterion would directly break the CLI/LSP.
3. ⇒ This directory contains **no “directly deletable, low-risk” items**; F2's only potential
   cleanup benefit is “whether the wasm artifact becomes smaller”—that has not been verified (see
   below).

#### VI. Unverified Items (Must Not Be Treated as Conclusions)

- **The size benefit for the wasm artifact has not been measured**: neither `wasm-pack build` nor
  size/symbol-table tools (`twiggy` / `wasm-objdump`) were run, so there is no data on “how much
  smaller would the artifact become if the project-mode subtree were gated?” It has also not been
  verified whether the linker already strips unreferenced `pub(crate)` items.
- **wasm32 consumers outside the shim have not been exhaustively identified**: this round checked
  only the repository's CI (the only wasm32 compilation is at `_build-wasm.yml:30/77`, through the
  shim crate). If a third party compiles the main crate directly for `wasm32-unknown-unknown` and
  invokes project-mode APIs, the reachable surface will exceed this determination.
- **“Load-bearing on the native side” = static call chain + stage tables**; each arm was not
  verified end-to-end by running the MultiFile/Check/Lsp probes individually.

#### VII. Appendix: Repository-Level Count Review (Same Revision)

```console
$ git grep -l wasm32 b5e68b3c9 -- '*.rs' | wc -l
30        # includes the wasm/ shim crate
$ git grep -l wasm32 b5e68b3c9 -- 'src/*.rs' | wc -l
29
$ git grep -lE '#\[cfg\([^]]*wasm32' b5e68b3c9 -- 'src/*.rs' | wc -l
22        # with actual cfg attributes
$ git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs' | wc -l
0         # no runtime cfg! criterion
```

The “wasm status” in `02` and the F2 row use the same basis: in the current tree, **29** files
contain the literal `wasm32` / **22** have actual `#[cfg(...)]` attributes / **7** only mention it
in documentation; `orchestrator.rs` has **16** occurrences = **10** attributes (4 positive + 6
negative) + 6 comments (the 7 files that mention it only in documentation: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Criterion Finding Registry (Registered from 2026-10-06, Continuing with Construction in Each Phase)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibitions 1 A/B/C/D) ran for
the first time on 2026-10-06. Under the division of responsibility “the gate reports only suspected
cases; humans adjudicate” ([08](08-maintenance-mechanism.md) §Known Limitations), this section
records the adjudication results and ownership of their disposition. Unexpanded matches (25 pairs
violating Criterion A, all Criterion B suspects) will be reviewed pairwise as construction proceeds
in the corresponding phases.

| #   | Object                                                                                                                            | Adjudication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Disposition Ownership                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                                     | **Legitimate structure, not parallel representations**: a value/type-tag pair maintained in a 1:1 relationship by the exhaustive `value_type()` (`value.rs:326-369`); a missed arm causes a compilation failure. However, external consumption of `ValueType` is substantively limited to `{:?}` formatting on std/os/fs/net error paths and the constant `Void` at `executor.rs:410`; its 21 variants greatly exceed actual needs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | P10 S5 slimming candidate: lower to `pub(crate)` and assess trimming; when S5 is constructed, jointly adjudicate the necessity of the `AsyncValue.value_type` field (`value.rs:217`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| G2  | Duplicate `ConstValue` (Criterion A, J=0.56)                                                                                      | **True positive**, including the f32/f64 width mismatch and the live `todo!()` in `kind()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Already recorded as E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| G3  | String-based type-name dispatch in 11 locations (true positive for Criterion D)                                                   | **True positive, same root cause**: built-in container names have no single authoritative identification point. List: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), the size table at `eval/const_eval.rs:498-511` (including the dead `"Uint"` arm), `to_type_id` at `middle/passes/codegen/bytecode.rs:744-756` (`"Set" => 25` contradicts Decision 4 for #300), and `"Clone" \| "Dup"` at `trait_data.rs:187`                                                                                                                                                                                                                                                                                              | **P6 extended verification item**: these points are on the typecheck / eval / middle side and are not covered by the parser data flow in 03 §5.4; during P6 construction, handle them under the same principle as 03 §5.3 (`NameKind` determined once, with match arms no longer performing string comparisons). The copy table has been corrected to seven locations (03 §2.3)                                                                                                                                                                                                                                                                            |
| G4  | Four false positives for Criterion D                                                                                              | **Legitimate lookup / dispatch tables, not consolidation targets**: tool-installation guidance at `package/build/requirements.rs:17-25`, build-result display names at `package/vendor/mod.rs:134-139`, REPL command aliases at `repl/mod.rs:241-396` (8 arms; user-interface aliases are legitimate UX), and `parse_runtime_mode` at `util/diagnostic/mod.rs:462-468` (a single boundary point with commented historical aliases)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | When P9 is hardened, add a `// reason:` exemption at each location; the number of exemptions will be reviewed through the gate report                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| G5  | `ReleasePlan.drops` span-key mismatch (Criterion measured during P2 2.4.4; not a P0 match)                                        | **True positive, a contractual defect—D20's prediction empirically confirmed**: ownership records keys by the span of the **last-used expression** (`ownership.rs:2574 build_release_plan`), while `ir_gen.rs:1956` queries the table by the span of the **statement**; expression-level keys never match—across the full corpus, **168 files / 373 keys** are not consumed, and Drops are silently lost (per-case evidence is in the header of `src/middle/core/tests/release_plan_spans.rs`)                                                                                                                                                                                                                                                                                                                                                                                                                                    | **P4/D20** (Span key → PlanId; behavioral surface approximately 168 files; must wait for the C2 criterion and corpus differencing to be in place); the criterion `test_release_plan_spans_consumed` remains permanently red (`#[ignore]`), and turns green when D20 is implemented                                                                                                                                                                                                                                                                                                                                                                         |
| G6  | Three IR metadata defects (observed while the P2 2.1.2 criterion `verify_loose` corpus run was green; D38's prediction fulfilled) | **Three true positives, all fixed under 2.1** (zero behavioral difference in the corpus differential baseline self-comparison): ① the method IR's locals table was truncated to the parameter count (`ir_gen.rs`, originally `take_cur_locals(param_types.len())`)—`local_count` underestimated, with the runtime silently tolerating out-of-bounds growth via `Frame::set_slot` resize (`frames.rs:103-105`); ② unannotated parameters in lambdas / curry intermediates / function paths were discarded by `filter_map` (three locations), causing the params length and frame layout to become misaligned—aligned with the existing method-path convention by filling `Int(64)` placeholders; ③ the match result slot depended on implicit Void pre-initialization of the frame (the landing point for a failed final-arm guard has no defined path on the CFG)—`generate_match_expr_ir` explicitly pre-populates Void at entry | **Fixed (same batch as 2.1). The determination scope is narrowed and recorded here** (the header of `verify.rs` is authoritative): temp-slot dominance is not checked (Void pre-initialization is frame semantics, and `ir_gen` intentionally depends on it); the trusted sources for type consistency are narrowed to parameter-slot signatures + constants (`ir_gen` never writes `LocalSlot.ty`; the entire table uses the `Int(64)` placeholder—the full determination scope belongs to P7 value table 04 §262); `init_locals` is the #368 debugging perspective, not an allocation upper bound, and an out-of-bounds `Local` in `init` is not checked |

## H. Inventory of Giant Functions (Pre-Release Audit Basis, from 2026-10-16)

> Basis (established by `/pre-release-audit` on 2026-10-16): a function span > 150 lines is an
> **initial screening threshold**, not a command to “report every one” individually—the full list
> would turn the inventory into a pile of line counts and drown the new signal in the existing
> backlog. Each report entry must contain three numbers: **host line count / lines inserted this
> time / line count before insertion**. Entries are divided into two categories: **A. Existing giant
> functions that received branches** (>150 lines with >50 lines inserted this time); **B. Newly
> added large functions** (almost entirely new and >100 lines). The line numbers below are taken
> from audit revision `1935aab53` (the `v0.8.4..1935aab53` range).

### A. Existing Giant Functions That Received Branches

| Function                | Location                                                    | Host Lines | Inserted This Time | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | ------------------ | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1594       | 358                | 1236             |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171                | 642              |

### B. Newly Added Large Functions

| Function                     | Location                                            | Lines | Newly Added |
| ---------------------------- | --------------------------------------------------- | ----- | ----------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177         |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122         |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130         |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107         |

**Existing giant functions not included in the inventory** (≤4 lines inserted this time; recorded
but not acted upon): `infer_expr` (`expressions.rs:2999`, 955 lines), `collect_function_signature`
(`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`, 441 lines),
`generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), and others.

### Disposition Direction

- **Category A**: Resolve through the subsequent decomposition of `infer_call_expr` /
  `generate_call_expr_ir`—these are the two heaviest hosts in the repository, and the named-argument
  conversion inserted 358 lines into the former.
- **Category B**: `push_joined` is a **local** helper for `Task`-style work-list formatting (its
  host is the 180-line addition), and its responsibilities differ from those of
  `mono.rs:123 increment_level_string` (174 lines, which rewrites hierarchical names), so **they are
  not duplicates and must not be merged**; `consumer_stage_skipped` and `local_count_of` can be
  decomposed independently.

## Key Decisions and Rationale

The authorization permits complete deletion, but this document recommends **different dispositions**
for several items:

| Category                                                                   | Disposition                                     | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions and no implementation) | **Delete**                                      | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty function), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                                                                                                                                                                                                  |
| **Hollow design documents**                                                | **Delete or rewrite**                           | Delete C1 and C2, then rewrite them to point to the actual implementation; rewrite C3's boundary table; **`config.js:259-268` must be cleaned up in parallel** (E6)                                                                                                                                                                                                                                                                                                                           |
| **RFCs whose status does not match implementation**                        | **Change status**                               | Move C4 (RFC-018) back to `draft`, or add `impl_status: 'not-started'`; add `impl_status: 'not-started'` to C5 (RFC-028). **Do not delete**—the design itself may have value; the problem is the inaccurate status marker                                                                                                                                                                                                                                                                     |
| **Orphan tests with genuine assertions**                                   | **Reactivate, do not delete**                   | Total **1005 lines / 78 tests**: lexer 629 lines / 55 tests + 311 lines / 16 tests hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + 65 lines / 7 tests under `package/template/tests/`. **Reactivation costs one `mod` declaration and gives CI back this coverage**. Deleting them would be a net loss                                                                                                                                                        |
| **Zero-call but semantically important `pub` items**                       | **Reduce visibility rather than delete**        | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move them to `pub(crate)` so the compiler can identify them, or complete the implementation. **B4's `is_subtype` must be retained**                                                                                                                                                                                                                                                                                                          |
| **Opcodes / instructions that cannot produce a value**                     | **Add construction sites or delete the opcode** | B8 (`TailCall`), B9 (`Switch`; the opcode round-trip test explicitly registers it through `WHITELIST=["SWITCH"]`). B10 was fixed under D33 (2026-11-14: `opcode()` distinguishes the `op` field + dedicated code `I64_NOT`; see the task 3.20.29 commit message) and was removed from this todo list. The remaining items require a choice among three alternatives and cannot remain as they are; D32/D33/D34 have been adjudicated and will be handled together with the `.42` version bump |
| **Data-loss defects**                                                      | ~~Separate issue~~ **P7 batch e (D52)**         | The three hard-coded discards in Section E—not dead code                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| **Valid code without module identity**                                     | **Refactor, do not delete**                     | F1 (`semantic_tokens.rs`): change `include!` to a genuine `mod` (see `02`)                                                                                                                                                                                                                                                                                                                                                                                                                    |
| **Items requiring a decision**                                             | **List as open questions**                      | F2's wasm32 cleanup has been registered as [09](09-execution-wbs.md) §P10 **S7** (gating the project-mode subtree on wasm32; user Decision B on 2026-10-18; including the three unverified items verbatim); layer-order corrections will expose new diagnostics                                                                                                                                                                                                                               |

### Directions Not Adopted

- **Treat everything as “complete deletion,” including orphan tests**—the 1005 lines of tests with
  genuine assertions are assets. Deletion costs one `mod` declaration, while the benefit is CI
  regaining coverage. The authorization means “deletion is allowed,” not “deletion is required.”
- **Only add gates and leave the code untouched**—gates can prevent regressions, but B1's 415 lines,
  the dispatch chain, and the 96-line dead staircase **already exist**; gates would merely freeze
  them in the baseline.
- **Retain the `accepted` status of RFC-018 / RFC-028 and only supplement the
  documentation**—`rfc/index.md:131` defines “accepted” as “enters the implementation phase,” so
  readers will reasonably assume an implementation exists. `TRACKING.md` already has precedent for
  the `impl_status` field (used by 5 RFCs), and adding the field costs far less than the distortion
  caused by inaccurate status.

## Implementation Essentials

**This document does not require an equivalence criterion** (it belongs to the
[C6 pure-deletion category](07-equivalence-oracle.md)), but **each item must still be confirmed to
have no references**. In RFC-039's global phase sequence, S1 corresponds to P1, and S2-S6 correspond
to P10.

| Phase | Content                                                                                                                                                | Acceptance                                                                                                                                                                                                                                                                                      |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Reactivate orphan tests with assertions (**1005 lines / 78 tests**)                                                                                    | The test counts for the lexer / pratt / passes / emitter / template modules increase under `cargo test`; **genuine defects are expected to be exposed** (the overflow path in `literals.rs` and the previously untested invalid `\x`/`\u` escape sequences), so time must be reserved for fixes |
| S2    | Delete pure-placeholder dead code (B1/B3/B6/B7 under the 05 cleanup/C6/E1)                                                                             | `cargo test` passes; `cargo clippy -D warnings` produces no new warnings                                                                                                                                                                                                                        |
| S3    | Handle hollow design documents (C1-C3 + clean up `config.js` in parallel with E6)                                                                      | The documentation site builds without 404s; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                                             |
| S4    | Correct RFC statuses (C4/C5/F3—add an “Implementation Status” column to `TRACKING.md`)                                                                 | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` is regenerated automatically, **do not edit it manually**)                                                                                                                                                                         |
| S5    | Reduce the visibility of `pub` items (B2/B4)                                                                                                           | The compiler can identify genuine dead code                                                                                                                                                                                                                                                     |
| S6    | Implement the decision for opcodes that cannot produce a value (B8/B9/F8 pending, following D32/D33/D34; **B10 is complete**)                          | Delete the dead paths for `Switch` / `TailCall`; implement the F8 gate; B10 is complete (2026-11-14, WBS 3.20.29: `opcode()` distinguishes the `op` field + dedicated code `I64_NOT`; see the task 3.20.29 commit message)                                                                      |
| S7    | Refactor `include!` (F1, with construction steps assigned to [09](09-execution-wbs.md) §P5 5.1) + correct layer order (F4, assigned to 4.4.1 under P4) | See `02-stage-contract.md`                                                                                                                                                                                                                                                                      |

**S1 must be done first, and time for defect fixes must be reserved.** Its value lies not in
“cleanup,” but in **restoring CI's access to 1005 lines / 78 tests that have never run**—until then,
every assessment of test coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose genuine defects.** The four radix scanners in `literals.rs` have four isomorphic
  branches that “continue consuming after overflow without reporting an error,” along with the
  entire `scan_leading_dot` function and the invalid `\x`/`\u` escape paths, all of which have never
  been tested. After reactivation, the code will probably need to be fixed, delaying the subsequent
  refactoring.
- **B9/B10 involve language-behavior decisions.** For `Switch` and `UnaryOp::Not`, the choice
  between “adding construction sites” and “deleting the opcode” must consider whether they are part
  of the public surface relied upon by released corpora.
- **After C1-C3 are deleted, documentation for the cross-file analysis capability will be
  temporarily absent**, until `check/cross-file-analysis.md` is rewritten to point to the actual
  architecture under `src/frontend/module/`.
- **Reachability analysis can discover only “unreachable” code, not “reachable but meaningless”
  code.** Judgments beyond Form 2 must be made item by item, and **this inventory does not
  constitute an exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  cannot be discovered automatically by the compiler and must be found through searches.
- **Deleting the half-remnant of RFC-023** (B1) fulfills the decision that “closures have been
  removed from the language”; **correcting status distortion** (C4/C5/F3) restores `TRACKING.md`'s
  reference value.

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> item-by-item decisions. **This document retains no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  Source decision for B1's residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Unwired runtime check insertion from B3, `§4.1`
- [RFC-018 LLVM AOT Compiler](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — Formal home of C1/C6
- [RFC-039 Compiler Feature Routing Directory Design](../../rfc/accepted/039-compiler-architecture.md)
  — Governing master plan
- [Refactoring Equivalence Criterion](07-equivalence-oracle.md) — Criterion requirements for
  category C6
- `src/frontend/core/typecheck/layers/README.md` — Actual proof-layer call order and the true
  identity of `equivalence` (F4 handled)
- `build.rs:19-55` — Example of this repository's generation-time gate
