# Dead Code and Paper Design Cleanup

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> See the main body of RFC-039 for the four-layer model, acceptance-criterion tiers, and
> execution-phase sequence; see the [directory index](index.md) for the positioning of each
> companion document.

## Positioning and Scope

This document provides an inventory of dead code and paper designs in the YaoXiang repository, the
criteria used to identify them, item-by-item disposition decisions, and the execution order. There
are four categories of cleanup targets:

| Category                                                                  | Size                                                                |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Files that have never participated in compilation                         | 24 files / 2,628 lines (of which 23 are true orphans / 1,081 lines) |
| Dead code that is compiled but never called                               | 1,500+ lines                                                        |
| Design documents describing an architecture that never existed            | Three documents under `docs/src/dev/design/check/`                  |
| Features attached to the default entry point, with no tests and no output | 1,152 lines under `src/repl/`                                       |

**The authorized scope is "complete deletion is permitted"** (decision of 2026-10-03). However, this
document recommends a disposition **opposite to "delete directly"** for several items; see
[Key Decisions and Rationale](#关键决策与理由) for the reasons.

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
phase contracts (see `02-stage-contract.md`), or criterion design (see `07-equivalence-oracle.md`).
B3 records that the entire `layers/dispatch.rs` chain is unwired; its removal is assigned to S2.

## Current State

### Three Forms of Technical Debt in Dead Code

**Form 1: Placeholder.** The code exists, appears to be doing something, but does nothing.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the "runtime check insertion" mechanism designed by `RFC-027 §4.1`, and is
  **completely unwired**.
- `check_type_equivalence` (`ProofContext` entry point) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is called only by tests. **Note:**
  `is_subtype` in the same file is used in production by `inference/assignment.rs:17`, so **the
  entire file must not be deleted**.
- `src/backends/runtime/engine.rs` (1,047 lines) is a pure DAG task scheduler. Its `pub` surface
  consists only of `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It **is not a second execution backend**—the
  `Executor` trait at `backends/mod.rs:356` currently has only one implementation (the interpreter).
  This point needs clarification because the directory name is misleading.

**Form 2: Leftovers.** An upstream decision has already been made, but removal was only half
completed.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) contains leftovers of six types:
  `FunctionInstance` (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), and `CaptureVariable`. **Clear origin:** when `RFC-023`
  deprecated the "closure capture model," it declared that it would **delete approximately 850 lines
  of code together with the closure model**. These 415 lines are the remnants that survived from
  those 850 lines—**removal was only half completed**. **Verification:** a whole-repository
  word-boundary search for these six types produced **zero matches outside `instance.rs`**
  (including tests). The `pub use passes::mono::instance::*;` at `middle/mod.rs:40` is merely a glob
  re-export and does not constitute consumption.

**Form 3: False Coverage.** This is the most dangerous form—**it makes the repository appear more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only five `pub mod` declarations (`literals` / `state` /
`symbols` / `tokenizer` / `tokens`) plus one
`#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;` (`:104-106`). **Nowhere is
`mod tests;` declared**, so the entire `lexer/tests/` subtree—13 files and 686 lines—**has never
participated in compilation**.

Seven of these are empty shells (`basic.rs` 3 / `comments.rs` 3 / `debug_lexer.rs` 1 /
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

(`fstring.rs`, 90 lines / 11 tests, is indeed compiled, but through the `#[path]` bypass rather than
`mod tests;`; `mod.rs`, 38 lines, is the declaration file and contains no assertions.)

**Only one file and 12 tests are actually running at the lexical layer, and all of them test only
the f-string brace raw protocol.**

The danger of this kind of problem lies in the fact that `cargo test` does not fail (tests that
never run cannot fail), and CI has no baseline for test counts or wiring checks.

### Paper Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not a case of "the design was completed but not implemented"; rather, **the
documents describe a different system that never existed**, and present "a possible future design"
as "a defect in an implemented system."

## Methodology and Confidence Statement

### Reachability Analysis Method

Starting from the two crate roots, `src/lib.rs` and `src/main.rs`, this analysis resolves the `mod`
/ `pub mod` / `#[path]` declarations of all 509 `.rs` files, constructs a module graph, and performs
BFS reachability analysis. `include!` targets are evaluated separately and are not counted toward
reachability.

**This method has known false-positive patterns, two of which were actually triggered while this
document was being written:**

1. `Split-Path` returns `\` on Windows, which does not match forward-slash keys → at one point, 499
   orphans were falsely reported.
2. **The child-module directory of a non-`mod.rs` file is `its-own-name/`, not `parent-directory/`**
   → the `mod tests` of `orchestrator.rs` was resolved as `src/frontend/module/tests/`, nearly
   classifying 753 lines of live tests as dead code.

Therefore, **every conclusion was confirmed twice: by actually reading the file and actually
searching for references**. Items that failed secondary confirmation were not included in this
document.

### Line-Count Convention

All line counts in this document were obtained with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines and systematically undercounts by about 5%** (for example,
`manifesto.md` is reported as 658 lines with `-Line`, but actually contains 893 lines). An earlier
round of analysis in this repository produced a low total because of this counting convention; the
figures here have been corrected.### Known Limitations

- **Reachability analysis can only discover "unreachable" code; it cannot discover "reachable but
  meaningless" code.** Forms other than Form 2 require item-by-item manual judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so unused `pub` types must be found
  through searches; this method does not cover them.
- This document does not involve running the compiler, so all "has never run" conclusions are based
  on static analysis.

## A. Files That Have Never Participated in Compilation

**Total: 24 files / 2,628 lines.** `semantic_tokens.rs` belongs to Form 4 (see Section F); the
remaining **23 files / 1,081 lines are true orphans**.

| Path                                                          | Size                    | Contents                                                                      | Basis for Classification                                                                                                                                                                                          |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entire `src/frontend/core/lexer/tests/` subtree, 13 files     | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only five `pub mod` declarations; the only test declaration at `:104-106` is `#[path]` pointing to `tests/fstring.rs`. A repository-wide `grep 'mod tests'` matches only `mod.rs:105` |
| 3 files under `src/frontend/pipeline/tests/`                  | 14 lines                | All are placeholder documentation comments, zero assertions                   | The entire 740-line `src/frontend/pipeline.rs` contains no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration (`Select-String` produced zero matches)                                                     |
| 3 files under `src/package/template/tests/`                   | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]` functions**           | The 7-line `src/package/template/mod.rs` declares only `mod gitignore; mod main_yx;`, with no `mod tests;`                                                                                                        |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden inside a **live** directory                                            | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                                                   |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                                      |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                                              |

**The last three items total 16 tests / 311 lines of code that have never run.** They are hidden
inside active directories whose `mod.rs` files appear complete, making them more deceptive than an
entire orphaned directory.

Two other files under `lexer/tests/` also belong to the "declared but not wired" form, but are
already included in the 629 lines in the first row and **are not counted again**: `lexer_mod.rs` (89
lines / 5 tests; `lexer/tests/mod.rs` declares 11 `mod` declarations but no `mod lexer_mod;`) and
`symbols.rs` (70 lines / 3 tests, likewise).

**Total real tests that have never run: 1,005 lines / 78 tests**: lexer 629/55 + pratt / passes /
emitter 311/16 + package/template 65/7.

**Pay particular attention to `precedence_inline.rs`**: it tests the `Precedence` enum and
`PrecedenceContext` in `precedence.rs`, a set of code with **zero production uses** (96 lines of
dead code). Even if it were wired up, it would still be testing a dead path.

Another item hidden among the 23 orphans is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, zero tests): `typecheck/tests/mod.rs` declares 23 test modules but omits this one—the file is
left with only a single `use` line and is a pure remnant. When S1 revives the tests, remove it along
with the wiring or add assertions.

## B. Dead Code That Is Compiled but Never Called

| #   | Location                                                                         | Size                    | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | -------------------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types     | A whole-repository word-boundary search produced zero matches outside `instance.rs`, including tests. Origin: RFC-023 declared the removal of approximately 850 lines, but only half were removed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines                | `TypeSystem` plus 4 pure functions. The only reference chain is `inference/mod.rs:11` `pub mod types;` and `:24` `pub use types::TypeSystem;` (both merely re-exports). Repository-wide, `TypeSystem` has only 5 matches: 2 definitions, 1 re-export, and 2 occurrences of the **unrelated** `lexer/symbols.rs:503 TypeSystemValidator`. The 4 methods have zero call sites. **Because they are `pub`, the compiler never reports them**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B3  | Entire chain in `src/frontend/core/typecheck/layers/dispatch.rs`                 | —                       | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` have zero call sites. The runtime check insertion specified by RFC-027 §4.1 is unwired. Note that it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                       | `check_type_equivalence` is called only by tests. **`is_subtype` in the same file must be retained** (`inference/assignment.rs:17` uses it in production)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2              | Self-described as a "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines                | `Precedence` enum + `PrecedenceContext`. **Their only external referrer is `pratt/tests/precedence_inline.rs` (95 lines / 6 tests), and that test file itself has never been compiled** (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore, **both must be deleted together**: deleting only the test leaves 96 lines of dead code, while deleting only the enum causes the test to fail to compile                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | Approximately 120 lines | `is_old_function_syntax` is called at `:719`, and `skip_old_function_syntax` is called at `:725`—**they are active gates that reject the old syntax, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) consists of only the comment `// 旧语法已移除`, but **the function does not consume any tokens when called**—this is an independent behavioral issue (whether the rejection path correctly advances the parse position), not dead code. **Disposition: review the behavior together with 05-frontend-paradigm.md (decision D27: remove naturally as part of the grammar migration), rather than deleting it directly as dead code**                                                                                                                                                                                                                                                 |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                       | The only occurrences repository-wide are match arms; there are no construction sites. The "no generation site (dead path)" comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B9  | `Switch` opcode                                                                  | —                       | `backends/common/opcode.rs` defines it, provides a variant, `opcode()`/`size()` arms, and a decoding arm, **but there are no construction sites anywhere in the repository**. The interpreter has an active implementation at `ops/control.rs:90`. **It has been explicitly registered by tests as a known dead path**: `tests/bytecode.rs:1035-1040` defines `WHITELIST: &[&str] = &["SWITCH"]` (the check fails if the difference is nonempty)                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| B10 | `UnaryOp::Not` (bytecode unary logical NOT)                                      | —                       | **Fixed (WBS 3.20.29, 2026-11-14, issue #562 closed)**. The original defect was that `opcode()` encoding ignored the `op` field (Not and Neg both emitted `I64_NEG`), and the decoder also constructed only `Neg`—the encoding/decoding round trip rewrote logical NOT as negation (`!x` became `-x` when running `.42` directly; the interpreter's in-memory Neg-on-Bool compensation arm happened to produce the right answer, so the defect affected encoding fidelity rather than current runtime results). The fix added a dedicated opcode `I64_NOT` (0x2C): `opcode()` branches on the `op` field + the decoder's new arm restores `Not` + the translator's `Not` arm emits the new opcode + `opcode_name` records it; the executor's existing `Not` execution arm connects naturally; the old `.42` `I64_NEG` decoding behavior remains unchanged. See the commit message for task 3.20.29 for the criterion chain |
| B11 | 4 occurrences of `#[allow(dead_code)]`                                           | —                       | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (both are real dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (a conservative exemption for an extern binding, **reasonable to retain**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (a test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                       | **Zero production construction sites** (the only construction site is a test manually creating a node at `parser/tests/ast.rs:828`), but it has **12 production consumers + 2 exhaustive-match arms** (for the complete list, see 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()` and `pratt/mod.rs:47` `expr_end_line`. Actual function definitions use `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Removal is assigned to P6 (the 16-item change list in 03 §5.5)                                           |
| B13 | `Instruction::Cast` / `Instruction::TypeTest` (`src/middle/core/ir.rs`)          | —                       | The only occurrences repository-wide are consumer arms, with **no construction sites** (the same form as B8), and their payloads are discarded downstream—`translate_cast` cannot obtain `target_type`, and the `TypeTest` arm emits the no-op operand placeholder `vec![0, 0, 0]`. **WBS 6.6.11 removed them** (both variants + all consuming arms in `normalize`/`verify`/`mono`/`translator` + the `substitute_block`/`substitute_instruction`/`substitute_type_ast`/`translate_cast` functions that consequently lost their only callers); grep found zero matches for this form across 216 IR snapshots. **Removal does not affect syntax-level `as` support**, but makes the existing gap that "`as` semantics are not implemented" more visible ⇒ the gap and evidence chain are recorded in the commit message for task 6.6.11; see the SPEC-side note in `docs/src/reference/language-spec/syntax.md` §2.6        |

## C. Paper Designs

| #   | Target                                                                          | Size                                        | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                    | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, and `HotReloader` have **zero matches repository-wide**. The "known limitations" at `:46-47` admit that `check_incremental` still calls `check_files_with_diagnostics` internally (the full path); `:46` also references the busy-wait debounce in `command.rs`—yet **`command.rs` contains neither `Instant` nor `recv_timeout`** (the only repository-wide occurrence of `recv_timeout` is in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                    | `:37` claims that the placeholder implementations under `traits/` (coherence / impl_check / object_safety / resolution) are incomplete—**the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero matches repository-wide. `:38` claims that `check_single_module` creates an independent Compiler for each file—**the function does not exist** and appears only in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                                         |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                    | `:14` claims that "watch mode rechecks only affected files," and `:27` claims that "check performs only the first two steps." In reality, `check_project` also performs vendor consistency checks, project-role classification, dead-code analysis, and W1006; the `Check` subcommand in `main.rs` has **no `--watch` flag** (every `#[arg]` declaration was checked individually)                                                                                                                                                                                        |
| C4  | `RFC-018` LLVM AOT                                                              | 1,037 lines of design / **0 lines of code** | The AOT matches at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` has no cranelift / llvm / inkwell dependency. `:861-876` contains the document's own implementation checklist with every item marked `- [ ]`, yet the status is `accepted`                                                                                                                                                                                                                                                           |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**   | `028-jit-compiler.md:99-126` requires 6 files under `src/backends/jit/` plus `middle/passes/ir_normalize.rs`; **none exist**. The status is `draft` (which is more honest)                                                                                                                                                                                                                                                                                                                                                                                                |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                                | **Physical evidence**: the filenames correspond precisely to the `ModuleCache` and incremental scheduling described by the C1 design. **Someone created test skeletons based on the design, found that the pipeline had no corresponding implementation, and left them unfinished**                                                                                                                                                                                                                                                                                       |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                           | The `//! §6.1: CheckSession 增量检查` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` point to this **gitignored plan document** (`.gitignore` contains `superpowers/`). These § references are **archaeological references to a deprecated API**                                                                                                                                                                                                                                                                              |

### The Actual Implementation Location of Cross-File Analysis

The cross-file analysis capabilities described by C1-C3 **do genuinely exist**, but at entirely
different paths:

| Component           | Location                              | Lines                   |
| ------------------- | ------------------------------------- | ----------------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439                     |
| Resolver            | `src/frontend/module/resolver.rs`     | 195                     |
| Role classification | `src/frontend/module/roles.rs`        | 388                     |
| Consistency checks  | `src/frontend/module/consistency.rs`  | 206                     |
| Orchestration       | `src/frontend/module/orchestrator.rs` | 1,528                   |
| **Total**           |                                       | **Approximately 2,670** |

Therefore, the disposition for C1-C3 is **not to remove the capability, but to remove misleading
documents** and rewrite the boundary table in `check/index.md` to point to the actual architecture
under `src/frontend/module/`.

### The Disposition of RFC-019 Requires Clarification

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already under the `deprecated/`
directory**, and the residual check was **clean** (`homoicon` has zero matches repository-wide, and
the document ends with the honest conclusion: "if this is restarted in the future, begin with
proofs/prototype experiments, not with the RFC"). **It requires no additional disposition**—this
item is in the authorized inventory, but verification confirmed that it is already in its terminal
state.

## D. Feature-Level Dead Code: REPL

`src/repl/` contains 4 files totaling **1,152 lines**, with **zero `#[cfg(test)]` and zero
`#[test]`**. Meanwhile, `src/main.rs:440` specifies
`args.command.unwrap_or(Commands::Repl)`—**running `yaoxiang` without arguments directly enters the
REPL**.

| #   | Fact                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D1  | **`EvalResult::Value` is never constructed** | It appears only twice in all of `src/repl/`: the definition at `eval.rs:23` and the **match arm** at `mod.rs:180`. Consequently, `Repl::format_value` (`mod.rs:493-502`) is dead code, and **the REPL never echoes any evaluation result**                                                                                                                                                                         |
| D2  | **No cross-round state**                     | `extract_definitions` (`eval.rs:345-377`) calls only `define_variable` (which stores a **type string**) and never `define_var` (which stores a `RuntimeValue`); `wrap_code` does not inject historical definitions either. The `VariableInfo::Value` branch (`eval.rs:23`) is unreachable. `REPLContext.variables` serves only completion and does not participate in evaluation                                   |
| D3  | **Wrapping syntax is likely obsolete**       | `eval.rs:314`/`316` produce `main() -> () = () => { code }` (the `name() -> Ret` old-style function declaration header), whereas the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`; the canonical table in RFC-007:94 consists entirely of `name: (a: Type) -> Ret`). **This was not tested by running the compiler; if true, every REPL evaluation fails to compile** |
| D4  | `:debug` is a stub                           | `mod.rs:454-466` prints only two prompt lines and returns; breakpoints are stored (`mod.rs:319`), but **the interpreter never queries them**                                                                                                                                                                                                                                                                       |
| D5  | `:history` is a stub                         | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                                          |
| D6  | Duplicates `lib.rs`                          | `lib.rs:85` `eval_code` and `repl/eval.rs:wrap_code` are **two independent implementations of "automatically wrap main"**, and their wrapping forms differ                                                                                                                                                                                                                                                         |

## E. Local Cleanup Items

| #   | Location                                                                                                | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                              | An empty directory tree at the root. It is almost certain that some code concatenated the string `"undefined"` into a path. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| E2  | `src/lib.rs:568`, `:582`                                                                                | `dump_type_detail` / `dump_const_detail` each have one `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` are FFI-related, so the panic path is predictable. Replace them with meaningful fallback output                                                                                                                                                                                                                                                                                                                                                                                             |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                               | `check_vendor_lock_consistency` and `check_workspace_consistency` are **approximately 60 nearly duplicated lines**, differing only in dependency-source handling and two path-dep skips. **Merge them**                                                                                                                                                                                                                                                                                                                                                                                                             |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`                  | Old function-syntax detection and rejection, approximately 120 lines. The body of `skip_old_function_syntax` (`:120-122`) contains only one comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                               |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                        | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                                 | The "Tool Design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, and `check/incremental-checking`. **If the C1-C3 document removals are performed, this location must be cleaned up at the same time**, or the documentation site will contain broken links                                                                                                                                                                                                                                                                                                                       |
| E7  | Duplicate `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | Five variants have the same names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` has inconsistent width (f32 × f64)**; there are no `From` / `TryFrom` implementations, and conversion code is handwritten at scattered call sites (`const_eval.rs` / `ir_gen.rs`). `kind()` at `const_data.rs:44` has an active `todo!()` for `LibraryRef` / `ExternRef`. **Disposition: merge into one definition + add explicit conversions + replace `todo!()` with fallback output; assign to P10 (local cleanup items of category E)**. Registered by the P0 gate on 2026-10-06 (criterion A, J=0.56) |
| E8  | Duplicate synonymous mappings downstream of `Literal` (Prohibition III A)                               | At least three identical `Literal → MonoType` arms (`inference/expressions.rs`, `checker.rs`, `inference/patterns.rs`, such as `Literal::Int(_) => MonoType::Int(64)`); `Literal → ir::ConstValue` (19 handwritten occurrences in `ir_gen.rs`) and `Literal → const_data::ConstValue` (4 handwritten occurrences in `const_eval.rs`) have no shared conversion point. **Disposition: move the 3 typecheck-side instances to a shared layer under P6; consolidate the `ir_gen` / `const_eval` side together with the E7 `ConstValue` merge under P10.** Registered by the P0 gate on 2026-10-06                      |

### Data Loss: Two Locations, Not Three

`impl From<BytecodeFile>` (now `src/middle/bytecode/mod.rs`) originally had three hardcoded
discards. The 7e recheck found two genuine losses and one false positive:

| Line (old) | Discarded Content                | Recheck Conclusion                                                                                                                                                                              |
| ---------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `2312`     | `upvalue_count: 0`               | **Genuine loss**: persisted with v5 (7e.1)                                                                                                                                                      |
| `2315`     | `exception_handlers: Vec::new()` | **False positive**: the field has no producer or consumer, and the language layer has no throw/try/catch (RFC-030/036) ⇒ it is a dead field and is not persisted; removal is assigned to P10 S6 |
| `2341`     | `globals: Vec::new()`            | **Genuine loss**: persisted with v5 (7e.3), with the carrier unified as `MonoType` (7e.4)                                                                                                       |

Criterion: an empty table with no producer is not lost data; it is a dead path (the same criterion
as D32). The `.42` `VERSION` was raised from 4 to 5 in one change covering both genuine losses
(decision D52). This section is included only to prevent them from being mistakenly cleaned up as
"dead code."

## F. Items Not to Delete, but to Mark or Decide

| #   | Target                                                                                                                                                                                                                                                                  | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (**already handled (P5 5.1)**: then 1,547 lines, now 1,553 lines, and a real `mod`)                                                                                                                            | **Not dead code**. At the time, it was spliced into the end of `checker.rs` through `include!` (the repository's only `include!`), and the file **was not declared as any module** (`grep 'mod semantic_tokens'` has zero matches repository-wide). Its first line is directly `impl TypeChecker {`, with no `use` header of its own, and it is the only file under the `checker/` directory. Consequences: no module identity, failure of visibility isolation, broken rust-analyzer navigation and symbol search, and the toolchain treating it as part of `checker.rs`. At the time, the real size of the `checker` module was 5618+1547 ≈ 7165 lines (now `checker.rs` is 4167 lines + three `checker/` submodules of 1553/1107/470 lines). **Fix: convert it to a real `mod`** (completed under [09](09-execution-wbs.md) §P5 5.1 on 2026-10-26; the B1 inventory for `check-boundary` is now zero), rather than deleting it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| F2  | wasm attributes (both positive and negative forms on the same line counted): the current `src/` tree contains the `wasm32` literal in **29** files, with actual attributes in **22** and mere mentions in **7**; `orchestrator.rs` has **10** (4 positive + 6 negative) | **Not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` `crate-type = ["cdylib"]`, `:18` `wasm-bindgen = "0.2"`, and `:17` depends on the main crate; `exclude = ["wasm", ...]` at root `Cargo.toml:3` explicitly shows that it is an independent crate. Looking only at `crate-type = ["rlib"]` at root `Cargo.toml:31` would lead to the false conclusion that these branches are defensive dead code—**that inference is invalid**. The CI wasm build is at `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), invoked by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches are load-bearing**: they determine which main-crate APIs `wasm/src/lib.rs` (now **152** lines) can call on the wasm target (`src/lib.rs:151` / `:170` / `:187` gate `run_file` / `run_project` / `build_bytecode` as a whole). **The cleanup scope has been determined**: four wasm arms are unreachable at runtime in the playground scenario but are load-bearing at compile time (their call sites are not gated); six `not(wasm32)` arms are load-bearing in the native three-stage tables ⇒ **there are no directly deletable low-risk items**—see “F2 Assessment” below for item-by-item determinations and evidence (decision D35: remove unreachable items, retain reachable ones; assign to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                                                                                                                                                                                                          | It has only a “Status” column (document status) and **no “Implementation Status” column**—it therefore **cannot represent** the error where "an RFC is marked complete but the code does not exist." Only 5 of 52 RFCs repository-wide have an `impl_status` field (`027` in-progress; `017`/`014`/`014b` complete; `review/027a` partial). **The 26 accepted RFCs do not record implementation status at all**, including 009, 009a, 013, 018, and 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                          | **Already handled (4.4.1)**: the README now describes the actual call order of `checker.rs::check_module_impl` (refinement walkthrough `predicate` → `termination` → `ownership` → type-constraint solving), with parenthetical line numbers taken from the current tree and function names/driver points used as anchors; it no longer claims that “a lower layer failing means the upper layer does not run” (there is actually no short-circuit); `equivalence` is described according to its true identity as a “shared type-comparison utility + proof entry point used only by tests.” This entry is retained only as a record of completed work and no longer points to the old line ranges                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                                 | `:79` places RFC-026a in the "Accepted RFCs" table, while its status column says "RFCs under review"; the "RFCs under review" section at `:48-49` lists only 032 and 027a, omitting 026a. This is inconsistent with the three review entries in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F6  | `tools/cargo-dist/` (`dist.exe`, 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                             | **Neither is tracked by git** (`git ls-files` returns nothing), and both are excluded by `.gitignore`. **This is not a problem**; it is recorded only to prevent later audits from mistakenly classifying them as "committed binaries"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                    | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(")` up to the next quote) that depends on the `("E1001", ...)` tuple syntax; **it is not a parser**. Its architecture (`parse` + `validate` + a build.rs gate + `--fix` remediation) is worth reusing for the opcode table, but **the extractor must be rewritten for the opcode form (approximately 30 lines)**. This belongs to follow-up work in 06-cleanup-inventory.md or to a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                                                                                                                                                                                             | 83 constants, zero duplicates, range `0x00..0xE2` (span 227, **144 holes**). The same opcode facts are expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / the decoding match (`bytecode.rs:978-2302`) / 48 `translate_*` functions (`translator.rs:676-1577`). **The compiler enforces only 2 of these** (the dispatch table at `backends/.../executor/debug.rs:194` + the exhaustive matches in each `ops/*.rs` family); a missing entry in `opcode()` / `size()` / a decoding arm / an encoder **only fails when a `.42` artifact is executed**. The comment on the `size()` table (`649-803`) states that the actual encoding differs (the `bytecode.rs:2181-2182` comment), and there is **no systematic differential test** (the existing size tests cover only Nop/Mov/Borrow/Release). Operator semantics also use a third parallel enumeration (`BinaryOp`/`UnaryOp`/`CompareOp` at `bytecode.rs:70/97/106`, with another naming set for `Rem`/`Xor`/`Sar`); consolidate these as part of the opcode-generation-time gate. The target location for the vocabulary is `middle/bytecode/opcode.rs` (migrated as part of the bytecode-domain consolidation, eliminating the L3→L4 reverse dependency; see 01)                                                                                                                                   |

### F2 Assessment: Reachability of wasm Attributes in `orchestrator.rs` (Assessment of 2026-10-18; No Cleanup Included)

**Assessment surface** = the wasm production build (the shim crate built by `wasm-pack`, as stated
in the F2 row); **assessment revision** `b5e68b3c9`. This section records only the assessment
conclusions and makes no cleanup commitments (cleanup is assigned to P10 / D35).

#### 1. Counting Convention (Retaken from the Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- The current `orchestrator.rs` tree has **10 wasm attributes** = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- The counting convention = **the `#[cfg(...)]` attribute text within the same line**, counting both
  positive and negative forms (this is also the convention used for “12” in 02); runtime macros of
  the form `cfg!(...)` are not counted—a repository-wide
  `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'` produced zero matches.
- The “12” is not an error; it is **a fact from an older revision**: at `544695ea9` (when RFC-039's
  design document was first committed), there were 5 positive + 7 negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- The difference comes from **one pair of attributes being moved out as a pair**: `6ec330d22` (WBS
  4.2.2, `check_project` was slimmed down into a `Program` constructor; this commit removed a net
  252 lines from `orchestrator.rs`) moved `role_context`, together with its `not(wasm32)`
  implementation and `wasm32` stub, **to** `src/driver/helpers.rs:360/394`—so `orchestrator.rs` fell
  from 12 to 10; the mechanism itself did not disappear.
- Revision-by-revision recheck (actually performed this time; `6ec330d22` is the old/new boundary):

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

#### 2. The Three-Question Criterion (Separating Them Prevents Load-Bearing Arms from Being Misclassified as Dead Code)

1. **Compile-time existence**—does this arm participate in compilation for the target?
2. **Runtime reachability**—is there a call chain from the playground entry point for that target?
3. **Removal cost**—will deleting it break compilation for that target (are the call sites equally
   gated)?

#### 3. Assessment Starting Point (Playground Entry Point and Phase Table)

- Entry points: `wasm/src/lib.rs` (now 152 lines) `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, constructs only `Pipeline` and does not touch
  the orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` uses `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`;
  `SingleFile | WasmPlayground` appear together in the same table): Parsing / Typecheck /
  DeadCodeAnalysis / ProofExecution / IrGeneration / Monomorphization—**excluding**
  VendorConsistency / Discovery / Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### 4. Item-by-Item Assessment (10 Attributes in the Current Tree)

| #   | Location (`orchestrator.rs`) | Gated Target                                                           | Compiled in the wasm Build                  | Reachable at Playground Runtime                                                                                                                                                                | Removal Cost (Are Call Sites Gated?)                                                                         |
| --- | ---------------------------- | ---------------------------------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 1   | `:271` positive              | wasm arm (`Ok(None)`) of `ensure_vendor_consistency` (`:267`)          | Yes                                         | **No**: the only call site, `src/driver/arms.rs:1195` (the VendorConsistency arm), is not in the playground phase table                                                                        | High: `arms.rs:1195` is not gated; deleting the arm alone ⇒ wasm compilation fails                           |
| 2   | `:418` positive              | wasm arm of `discover_with_used` (`:400`) (sets `lock_versions` empty) | Yes                                         | **No**: call sites `arms.rs:1221` (the Discovery arm) and `src/driver/helpers.rs:686` (the Check-form reference pool `collect_package_refs`, called only by the Check branch at `arms.rs:521`) | High: same as above                                                                                          |
| 3   | `:655` positive              | wasm stub for `member_context` (`:656-659`)                            | Yes                                         | **No**: the only call site is `:404` (inside the function from #2)                                                                                                                             | High: `:404` is not gated                                                                                    |
| 4   | `:753` positive              | wasm stub for `dep_import_surface` (`:754-756`)                        | Yes                                         | **No**: call sites are `:671` (`resolve_in_package_root`) and `:813` (`resolve_in_vendor_with_lock`), rooted in `:468` (`resolve_module_path`, in the BFS of `discover_with_used`)             | High: same as above                                                                                          |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                       | No (does not participate in the wasm build) | Not applicable                                                                                                                                                                                 | Load-bearing in native builds: used by `:678` / `:747`                                                       |
| 6   | `:276` negative              | native arm of `ensure_vendor_consistency`                              | No                                          | Not applicable                                                                                                                                                                                 | Load-bearing in native builds: VendorConsistency appears in the three phase tables at `program.rs:61/98/113` |
| 7   | `:413` negative              | native arm of `discover_with_used` (lock reading)                      | No                                          | Not applicable                                                                                                                                                                                 | Load-bearing in native builds: Discovery appears at `program.rs:62/99/114`                                   |
| 8   | `:607` negative              | native `member_context` (workspace resolution)                         | No                                          | Not applicable                                                                                                                                                                                 | Load-bearing in native builds: call site `:404`                                                              |
| 9   | `:677` negative              | manifest-name reading in `resolve_in_package_root`                     | No                                          | Not applicable                                                                                                                                                                                 | Load-bearing in native builds: `:544/:547` ← `:468`                                                          |
| 10  | `:745` negative              | native `dep_import_surface`                                            | No                                          | Not applicable                                                                                                                                                                                 | Load-bearing in native builds: `:671` / `:813`                                                               |

#### 5. Conclusions

1. **The 4 wasm arms are unreachable at runtime in the playground scenario** (#1-#4; evidence = the
   phase table does not include the stages containing their only call sites), but they are
   **load-bearing at compile time**. Executing D35's literal instruction to "remove unreachable
   items" would fail to compile—the feasible form is to "gate the entire project-mode subtree
   (`orchestrator.rs`'s discovery/resolution functions + the driver's VendorConsistency/Discovery
   arms + the reference-pool collection in `helpers.rs`) on wasm32"; the cost-benefit tradeoff is
   assigned to P10.
2. **The 6 `not(wasm32)` arms do not exist in the playground build** (excluded at compile time), and
   all are load-bearing on the native side (the MultiFile / Check / Lsp phase tables). The criterion
   "unreachable in the playground ⇒ delete" **does not apply**: deleting by that criterion would
   directly break the CLI/LSP.
3. ⇒ This directory contains **no "directly deletable, low-risk" items**; the cleanup benefit for F2
   is limited to "whether the wasm artifact becomes smaller"—that has not been verified (see below).

#### 6. Unverified Items (Must Not Be Treated as Conclusions)

- **The wasm artifact-size benefit has not been measured**: `wasm-pack build` and
  artifact/symbol-table tools (`twiggy` / `wasm-objdump`) were not run, so there is no data on "how
  much smaller the artifact could become by gating the project-mode subtree"; whether unreferenced
  `pub(crate)` items are already eliminated at link time has also not been verified.
- **wasm32 consumers outside the shim have not been exhaustively enumerated**: this round checked
  only the repository's CI (the only wasm32 compilation is at `_build-wasm.yml:30/77`, through the
  shim crate). If a third party compiles the main crate directly for `wasm32-unknown-unknown` and
  calls project-mode APIs, the reachable surface may exceed this assessment.
- **"Load-bearing" on the native side means static call chains + phase tables**; no line-by-line
  end-to-end runtime validation was performed (the MultiFile/Check/Lsp probes were not run to touch
  every arm individually).

#### 7. Appendix: Repository-Level Count Recheck (Same Revision)

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

The “wasm status” in `02` uses the same convention as the F2 row: the current tree has the `wasm32`
literal in **29** files / actual `#[cfg(...)]` attributes in **22** / mere mentions in **7**;
`orchestrator.rs` has **16** occurrences = **10 attributes** (4 positive + 6 negative) + 6 comments
(the 7 files that merely mention it: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Criterion Findings Registry (Registered from 2026-10-06 onward, Continuing with Each Phase)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibitions I A/B/C/D) first
ran on 2026-10-06. Following the division of responsibility in which "the gate reports only
suspected cases and people adjudicate them" ([08](08-maintenance-mechanism.md) §Known Limitations),
this section records adjudication results and disposition ownership. Unexpanded findings (25
suspected pairs under criterion A and all suspected items under criterion B) will be rechecked pair
by pair as the corresponding phase proceeds.

| #   | Target                                                                                                                                   | Adjudication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Disposition Ownership                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                                            | **Legitimate structure, not parallel representations**: a value/type-tag pair maintained in 1:1 correspondence by the exhaustive `value_type()` (`value.rs:326-369`), where a missed arm causes a compilation failure. However, external consumption of `ValueType` is effectively limited to `{:?}` formatting on std/os/fs/net error paths and the constant `Void` assignment at `executor.rs:410`; its 21 variants far exceed actual needs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | P10 S5 slimming candidate: reduce to `pub(crate)` and assess trimming; when S5 is implemented, adjudicate the necessity of the `AsyncValue.value_type` field (`value.rs:217`) as part of the same work                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| G2  | Duplicate `ConstValue` (criterion A, J=0.56)                                                                                             | **True positive**, including inconsistent f32/f64 widths and an active `todo!()` in `kind()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Recorded as E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| G3  | String-based type-name dispatch in 11 locations (true positive under criterion D)                                                        | **True positive, same root cause**: there is no single authoritative identification point for built-in container names. Inventory: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), the size table at `eval/const_eval.rs:498-511` (including the dead `"Uint"` arm), `to_type_id` at `middle/passes/codegen/bytecode.rs:744-756` (`"Set" => 25` contradicts decision 4 of #300), and `"Clone" \| "Dup"` at `trait_data.rs:187`                                                                                                                                                                                                                                                                                                           | **P6 extended verification item**: these locations are on the typecheck / eval / middle sides and are not covered by the parser-data-flow scope of 03 §5.4; when P6 is implemented, handle them under the same principle as 03 §5.3 (`NameKind` decided once, with match arms no longer performing string comparisons). The duplicate table has already been corrected to seven locations (03 §2.3)                                                                                                                                                                                                                                                       |
| G4  | 4 false positives under criterion D                                                                                                      | **Legitimate lookup / dispatch tables, not gaps to be closed**: tool-installation guidance at `package/build/requirements.rs:17-25`, build-result display names at `package/vendor/mod.rs:134-139`, REPL command aliases at `repl/mod.rs:241-396` (8 arms; user-interface aliases are legitimate UX), and `parse_runtime_mode` at `util/diagnostic/mod.rs:462-468` (a historical alias with a comment, at a single boundary point)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | When P9 becomes strict, add a `// reason:` exemption to each location; the exemptions will be reviewed as they appear in the gate report                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| G5  | Span-key mismatch in `ReleasePlan.drops` (P2 2.4.4 criterion measurement; not a P0 finding)                                              | **True positive, contract defect—empirical confirmation of D20's prediction**: ownership records keys by the span of the **last expression use** (`ownership.rs:2574 build_release_plan`), while `ir_gen.rs:1956` looks them up by the span of the **statement**; expression-level keys never match—across the full corpus, **168 files / 373 keys** went unconsumed, silently dropping Drops (see the file header of `src/middle/core/tests/release_plan_spans.rs` for case-by-case evidence)                                                                                                                                                                                                                                                                                                                                                                                                                                                             | **P4/D20** (Span key → PlanId, behavioral surface approximately 168 files; must wait for the C2 criterion and corpus differential to be in place); the `test_release_plan_spans_consumed` criterion remains permanently red (`#[ignore]`) and turns green when D20 is implemented                                                                                                                                                                                                                                                                                                                                                                         |
| G6  | Three IR metadata defects (P2 2.1.2 criterion `verify_loose`, observed during the corpus run turning green, fulfilling D38's prediction) | **True positives ×3, all fixed under 2.1** (zero behavioral difference, proven by self-differencing against the corpus baseline): ① the method IR's locals table was truncated to the number of parameters (`ir_gen.rs` originally used `take_cur_locals(param_types.len())`)—`local_count` underestimated the count, and the runtime silently tolerated this through `Frame::set_slot` resizing (`:103-105`); ② unannotated parameters in the lambda / curry intermediate layer / function path were dropped by `filter_map` (3 locations), causing the params length and frame layout to become misaligned—aligned with the existing method-path convention by filling in an `Int(64)` placeholder; ③ the match result slot depended on the frame's implicit Void preinitialization (the fallback landing point when the final arm's guard fails had no defined path on the CFG)—the `generate_match_expr_ir` entry point explicitly preinitializes Void | Already fixed (in the same batch as 2.1). **Narrowed assessment surface registered** (the header of `verify.rs` is authoritative): temporary-slot dominance is not checked (Void preinitialization is frame semantics, and ir_gen intentionally depends on it); the trustworthy source for type consistency is narrowed to parameter-slot signatures + constants (ir_gen never writes `LocalSlot.ty`; all entries use the `Int(64)` placeholder—the complete assessment surface belongs to the P7 value table, 04 §262); `init_locals` is the #368 debugging view, not an allocation upper bound, and out-of-bounds `Local` initialization is not checked |

## H. Inventory of Huge Functions (Pre-Release Audit Criterion, Effective from 2026-10-16)

> Criterion (established by `/pre-release-audit` on 2026-10-16): a function span > 150 lines is an
> **initial screening threshold**, not a command to "report every one"; listing everything would
> turn the inventory into a pile of line counts and drown out the new signal in the existing
> backlog. Every report entry must contain three numbers: **host lines / lines inserted this time /
> lines before insertion**. There are two categories: **A. existing huge functions into which
> branches were inserted** (>150 lines and >50 lines inserted this time); **B. large functions newly
> added this time** (almost entirely new and >100 lines). The line numbers below were taken from
> audit revision `1935aab53` (the `v0.8.4..1935aab53` range).

### A. Existing Huge Functions into Which Branches Were Inserted

| Function                | Location                                                    | Host Lines | Inserted This Time | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | ------------------ | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1594       | 358                | 1236             |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171                | 642              |

### B. Large Functions Newly Added This Time

| Function                     | Location                                            | Lines | Newly Added |
| ---------------------------- | --------------------------------------------------- | ----- | ----------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177         |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122         |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130         |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107         |

**Existing huge functions not included in the inventory** (≤4 lines inserted this time; recorded
only, not disposed of): `infer_expr` (`expressions.rs:2999`, 955 lines),
`collect_function_signature` (`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`,
441 lines), `generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), and others.

### Disposition Direction

- **Category A**: resolve these together with the follow-up splitting tasks for `infer_call_expr`
  and `generate_call_expr_ir`—they are the repository's heaviest hosts, and this time's
  named-argument refactoring inserted 358 lines into the former.
- **Category B**: `push_joined` is a **local** helper for formatting a `Task` work-list-style type
  (its host is the new 180-line function), and it has a **different responsibility** from
  `mono.rs:123 increment_level_string` (174 lines, which rewrites hierarchy names); they are **not
  duplicates and should not be merged**. `consumer_stage_skipped` and `local_count_of` can be split
  independently.

## Key Decisions and Rationale

The authorization is "complete deletion is permitted," but this document recommends a **different
disposition** for several items:

| Category                                                               | Disposition                                     | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions or implementation) | **Delete**                                      | B1 (415 lines), B3 (dispatch chain), B6 (96 lines), B7 (empty function), C6 (14 lines), E1 (empty directory)                                                                                                                                                                                                                                                                                                                                                                                 |
| **Paper design documents**                                             | **Delete or rewrite**                           | Delete C1 and C2, then rewrite them to point to the actual implementation; rewrite C3's boundary table; **`config.js:259-268` must be cleaned up at the same time** (E6)                                                                                                                                                                                                                                                                                                                     |
| **RFCs whose status does not match the implementation**                | **Change status**                               | Move C4 (RFC-018) back to `draft` or add `impl_status: 'not-started'`; add `impl_status: 'not-started'` to C5 (RFC-028). **Do not delete them**—the design itself may have value; the problem is the inaccurate status marker                                                                                                                                                                                                                                                                |
| **Orphaned tests with real assertions**                                | **Revive, do not delete**                       | Total **1,005 lines / 78 tests**: lexer 629 lines / 55 tests + 311 lines / 16 tests hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + 65 lines / 7 tests under `package/template/tests/`. **Revival requires a one-line `mod` declaration and restores this coverage to CI**. Deleting them would be a net loss                                                                                                                                                |
| **Unused `pub` items with important semantics**                        | **Reduce visibility rather than delete**        | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move them to `pub(crate)` so the compiler can identify them, or complete the implementation. **B4's `is_subtype` must be retained**                                                                                                                                                                                                                                                                                                         |
| **Opcodes / instructions that cannot produce values**                  | **Add construction sites or delete the opcode** | B8 (`TailCall`), B9 (`Switch`; the opcode round-trip test explicitly registers it as `WHITELIST=["SWITCH"]`). B10 was fixed under D33 on 2026-11-14 (`opcode()` distinguishes the `op` field + dedicated opcode `I64_NOT`; see the commit message for task 3.20.29) and has been removed from this todo list. The remaining items offer three choices; the current state cannot be maintained. D32/D33/D34 have been adjudicated and will be handled together with the `.42` version upgrade |
| **Data-loss defects**                                                  | ~~Separate issue~~ **P7 batch e (D52)**         | The two genuine losses in Section E + one false positive (`exception_handlers` is a dead field; removal is assigned to P10 S6)                                                                                                                                                                                                                                                                                                                                                               |
| **No module identity but valid code**                                  | **Refactor, do not delete**                     | F1 (`semantic_tokens.rs`): change `include!` → a real `mod` (see `02`)                                                                                                                                                                                                                                                                                                                                                                                                                       |
| **Items requiring decisions**                                          | **List as open questions**                      | F2's wasm32 cleanup is registered under [09](09-execution-wbs.md) §P10 **S7** (gate the project-mode subtree on wasm32, user decision B on 2026-10-18; includes the three unverified items unchanged); layer-order corrections will expose new diagnostics                                                                                                                                                                                                                                   |

### Directions Not Adopted

- **Treat everything as "complete deletion," including orphaned tests**—the 1,005 lines of tests
  with real assertions are assets. Reviving them costs one `mod` declaration and restores coverage
  to CI. The authorization means "may be deleted," not "should be deleted."
- **Only add gates without changing code**—gates can prevent regressions, but B1's 415 lines, the
  dispatch chain, and the 96 lines of dead ladder code **already exist**; gates would merely freeze
  them in the baseline.
- **Keep the `accepted` status of RFC-018 / RFC-028 and only update the
  documentation**—`rfc/index.md:131` defines "accepted" as meaning "entering the implementation
  phase," so readers will reasonably assume an implementation exists. `TRACKING.md` already has
  precedent for an `impl_status` field (5 RFCs use it), making the cost of adding the field far
  lower than the cost of a misleading status.

## Implementation Essentials

**This document does not require an equivalence criterion** (it belongs to the
[C6 pure-deletion category](07-equivalence-oracle.md)), but **each item must still be confirmed to
have no references**. In RFC-039's global phase sequence, S1 corresponds to P1, and S2-S6 correspond
to P10.

| Phase | Content                                                                                                                                         | Acceptance                                                                                                                                                                                                                                                                         |
| ----- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphaned tests with assertions (**1,005 lines / 78 tests**)                                                                              | The test counts for the lexer / pratt / passes / emitter / template modules increase under `cargo test`; **real defects are expected to be exposed** (the overflow path in `literals.rs` and invalid `\x`/`\u` escapes have never been tested), so time must be reserved for fixes |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 together with the 05 cleanup/C6/E1)                                                              | `cargo test` passes; `cargo clippy -D warnings` reports no new warnings                                                                                                                                                                                                            |
| S3    | Handle paper design documents (C1-C3 + synchronized cleanup of `config.js` for E6)                                                              | The documentation site builds without 404s; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                                |
| S4    | Correct RFC statuses (C4/C5/F3—add an "Implementation Status" column to `TRACKING.md`)                                                          | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` is regenerated automatically; **do not edit it manually**)                                                                                                                                                            |
| S5    | Reduce the visibility of `pub` items (B2/B4)                                                                                                    | The compiler can identify the genuine dead code                                                                                                                                                                                                                                    |
| S6    | Implement decisions for opcode items that cannot produce values (B8/B9/F8 remaining; execute D32/D33/D34; **B10 is complete**)                  | Delete the dead paths for `Switch` / `TailCall`; implement the F8 gate; B10 is complete (2026-11-14, WBS 3.20.29: `opcode()` distinguishes the `op` field + dedicated opcode `I64_NOT`; see the commit message for task 3.20.29)                                                   |
| S7    | Refactor `include!` (F1; implementation steps assigned to [09](09-execution-wbs.md) §P5 5.1) + correct layer order (F4, assigned to P4's 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                                                         |

**S1 must be performed first, and defect-fixing time must be reserved.** Its value lies not in
"cleanup," but in **restoring 1,005 lines / 78 previously unrun tests to CI**—until then, every
judgment about test coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners in `literals.rs` have four isomorphic
  branches that "continue consuming after overflow without reporting an error," as does the entire
  `scan_leading_dot` function and the invalid `\x`/`\u` escape paths; none has ever been tested.
  Reviving the tests will probably require code fixes, delaying the subsequent refactoring.
- **B9/B10 involve language-behavior decisions**. For `Switch` and `UnaryOp::Not`, deciding between
  "add a construction site" and "delete the opcode" requires considering whether published corpora
  depend on the surface behavior.
- **After C1-C3 are deleted, documentation of cross-file analysis will be temporarily absent**,
  until `check/cross-file-analysis.md` is rewritten to point to the actual architecture under
  `src/frontend/module/`.
- **Reachability analysis can only discover "unreachable" code, not "reachable but meaningless"
  code**. Judgments beyond Form 2 must be made item by item, and **this inventory does not
  constitute exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so unused `pub` types (B2, B4) cannot
  be discovered automatically by the compiler and must be found through searches.
- **Removing the half-finished cleanup of RFC-023** (B1) fulfills the decision that "closures have
  been removed from the language"; **correcting inaccurate statuses** (C4/C5/F3) restores the
  reference value of `TRACKING.md`.

> **All open questions previously listed in this section have been adjudicated.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) for item-by-item
> decisions (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  the source decision for the B1 remnants
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — the unwired `§4.1` runtime check insertion described by B3
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — the formal home of C1/C6
- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — the overarching framework
- [Refactoring Equivalence Criterion](07-equivalence-oracle.md) — criterion requirements for the C6
  category
- `src/frontend/core/typecheck/layers/README.md` — the actual proof-layer call order and the true
  identity of `equivalence` (F4 already handled)
- `build.rs:19-55` — an example of this repository's generation-time gates
