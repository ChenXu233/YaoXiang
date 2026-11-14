# Dead Code and Paper-Only Design Cleanup

> **Companion design document.** This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution phase order are covered in the main
> RFC-039 text; the positioning of each companion document is described in the
> [directory index](index.md).

## Positioning and Scope

This document provides an inventory of dead code and paper-only designs in the YaoXiang repository,
the methodology used to identify them, item-by-item disposition decisions, and the execution order.
Cleanup targets fall into four categories:

| Category                                                                  | Size                                                                   |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Files that have never participated in compilation                         | 24 files / 2,628 lines (of which 23 are genuine orphans / 1,081 lines) |
| Dead code that is compiled but never called                               | 1,500+ lines                                                           |
| Design documents describing an architecture that never existed            | Three documents under `docs/src/dev/design/check/`                     |
| Features attached to the default entry point, with no tests and no output | 1,152 lines under `src/repl/`                                          |

**The authorized scope is "permission to delete outright"** (decision of 2026-10-03). However, this
document recommends dispositions for several items that run **counter to "direct deletion"**—the
rationale is given in [Key Decisions and Rationale](#关键决策与理由).

This document **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
phase contracts (see `02-stage-contract.md`), or oracle design (see `07-equivalence-oracle.md`). The
entire unwired `layers/dispatch.rs` chain recorded under B3 is handled as part of S2.

## Current State

### Three Forms of Technical Debt in Dead Code

**Form 1: Placeholder.** The code exists, appears to be doing something, but does nothing.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the "runtime check insertion" mechanism designed by `RFC-027 §4.1`, and is
  **completely unwired**.
- `check_type_equivalence` (`src/frontend/core/typecheck/layers/equivalence.rs:216`, the
  `ProofContext` entry point) is called only by tests. **Note** that `is_subtype` in the same file
  is used in production by `inference/assignment.rs:17`, so **the entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1,047 lines) is a pure DAG task scheduler. Its `pub` surface
  consists only of `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It is **not a second execution backend**—the
  `Executor` trait at `backends/mod.rs:356` currently has only one implementer, the interpreter.
  This point needs clarification because the directory name is misleading.

**Form 2: Leftovers.** An upstream decision has already been made, but its removal was only half
completed.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) retains six types: `FunctionInstance`
  (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), and `CaptureVariable`. **Origin is clear**: when `RFC-023`
  deprecated the "closure capture model," it declared that "**roughly 850 lines of code would be
  deleted along with the closure model**." These 415 lines are the remnants that survived from those
  850 lines—**the deletion was only half completed**. **Verification**: A repository-wide
  word-boundary search for these six types produced **zero hits outside `instance.rs`**, including
  in tests. The `pub use passes::mono::instance::*;` at `middle/mod.rs:40` merely forwards them via
  a glob import and does not constitute consumption.

**Form 3: False Coverage.** This is the most dangerous form—**it makes the repository appear more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only five `pub mod` declarations (`literals` / `state` /
`symbols` / `tokenizer` / `tokens`) plus one
`#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;` (`:104-106`). **Nowhere is
`mod tests;` declared**, so the entire 13-file, 686-line `lexer/tests/` subtree **has never
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

(`fstring.rs` is 90 lines / 11 tests and does participate in compilation, but through the `#[path]`
bypass rather than `mod tests;`; `mod.rs` is a 38-line declaration file with no assertions.)

**Only one lexer file actually runs, with 12 tests, and all of them test only the f-string raw brace
protocol.**

The danger of this class of problem lies in the fact that `cargo test` does not fail—tests that
never ran cannot fail—and CI has no test-count baseline or wiring check.

### Paper-Only Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not a case of "designed but not implemented"; rather, **the documents describe a
different system that never existed**, while presenting "a potentially future design" as "a defect
in an implemented system."

## Methodology and Confidence Statement

### Reachability Analysis Method

Starting from the two crate roots, `src/lib.rs` and `src/main.rs`, all `mod` / `pub mod` / `#[path]`
declarations in 509 `.rs` files are parsed, a module graph is constructed, and BFS reachability
analysis is performed. `include!` targets are assessed separately and are not counted toward
reachability.

**This method has known false-positive patterns, two of which were actually triggered while writing
this document:**

1. On Windows, `Split-Path` returns `\`, which does not match forward-slash keys → this briefly
   produced a false report of 499 orphans.
2. **Submodules of non-`mod.rs` files live in a directory named after the file itself, not its
   parent directory** → `orchestrator.rs`'s `mod tests` was resolved as
   `src/frontend/module/tests/`, nearly classifying 753 lines of live tests as dead code.

Therefore, **every conclusion was double-confirmed by actually reading the files and actually
searching for references**. Items that failed secondary confirmation are not included in this
document.

### Line-Count Definition

All line counts in this document use `(Get-Content).Count`. **Note that `Measure-Object -Line` skips
blank lines and systematically undercounts by about 5%** (for example, `manifesto.md` is reported as
658 lines with `-Line`, but actually has 893 lines). An earlier analysis round in this repository
produced an artificially low total because of this counting method; the figures here have been
corrected.### Known Limitations

- **Reachability analysis can only discover "unreachable" code, not "reachable but meaningless"
  code**. Forms beyond Form 2 require item-by-item manual judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types must be
  discovered through searches; this method does not cover them.
- This document does not involve running the compiler, so all "has never run" conclusions are static
  determinations.

## A. Files That Have Never Participated in Compilation

**Total: 24 files / 2,628 lines.** `semantic_tokens.rs` belongs to Form 4 (see Section F); the
remaining **23 files / 1,081 lines are genuine orphans**.

| Path                                                          | Size                    | Contents                                                                      | Basis for Determination                                                                                                                                                                                              |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entire `src/frontend/core/lexer/tests/` subtree, 13 files     | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only five `pub mod` declarations; the only test declaration at `:104-106` is a `#[path]` reference to `tests/fstring.rs`. A repository-wide `grep 'mod tests'` matches only `mod.rs:105` |
| `src/frontend/pipeline/tests/`, 3 files                       | 14 lines                | All are placeholder documentation comments, with 0 assertions                 | The entire 740-line `src/frontend/pipeline.rs` contains no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declaration (`Select-String` has zero matches)                                                             |
| `src/package/template/tests/`, 3 files                        | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]` functions**           | The 7-line `src/package/template/mod.rs` declares only `mod gitignore; mod main_yx;`, with no `mod tests;`                                                                                                           |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden inside a **live** directory                                            | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                                                      |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                                         |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                                                 |

**The final three items total 311 lines and 16 tests that have never run.** They are hidden inside
active directories whose `mod.rs` files appear complete, making them more deceptive than an entirely
orphaned directory.

Two other files under `lexer/tests/` belong to the same "declared but the declaration was omitted"
form, but are already included in the 629 lines in the first row and **are not counted again**:
`lexer_mod.rs` (89 lines / 5 tests; `lexer/tests/mod.rs` declares 11 `mod` declarations but omits
`mod lexer_mod;`) and `symbols.rs` (70 lines / 3 tests, likewise).

**Total never-run real tests: 1,005 lines / 78 tests**: lexer 629/55 + pratt / passes / emitter
311/16 + package/template 65/7.

**Pay particular attention to `precedence_inline.rs`**: it tests the production-zero-use
`Precedence` enum and `PrecedenceContext` in `precedence.rs` (96 lines of dead code). Even if the
test file were wired in, it would still be testing a dead path.

Another item hidden among the 23 genuine orphans is
`src/frontend/core/typecheck/tests/semantic_db.rs` (5 lines, 0 tests): `typecheck/tests/mod.rs`
declares 23 test modules but omits this one—the file consists of a single `use` statement and is a
pure remnant. It should be deleted or given assertions when S1 revives the test wiring.

## B. Dead Code That Is Compiled but Never Called

| #   | Location                                                                         | Size                | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | -------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types | A repository-wide word-boundary search produced zero hits outside `instance.rs`, including in tests. Origin: RFC-023 declared the removal of roughly 850 lines, but only half was removed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines            | `TypeSystem` plus 4 pure functions. The only reference chain is `pub mod types;` at `inference/mod.rs:11` and `pub use types::TypeSystem;` at `:24`, both mere pass-throughs. Repository-wide `TypeSystem` has only 5 hits: 2 definitions, 1 re-export, and **2 unrelated** occurrences of `lexer/symbols.rs:503 TypeSystemValidator`. The 4 methods have zero call sites. **Because they are `pub`, the compiler never warns about them**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B3  | Entire `src/frontend/core/typecheck/layers/dispatch.rs` chain                    | —                   | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` have zero call sites. The runtime check insertion in RFC-027 §4.1 is unwired. Note that it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                   | `check_type_equivalence` is called only by tests. **`is_subtype` in the same file must be retained** because production code uses it at `inference/assignment.rs:17`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2          | Described in its own source as a "framework placeholder"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines            | The `Precedence` enum + `PrecedenceContext`. **Their only external referrer is `pratt/tests/precedence_inline.rs`** (95 lines / 6 tests)—and that test file has itself never been compiled (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore, the two **must be deleted together**: deleting only the test leaves 96 lines of dead code, while deleting only the enum causes the test to fail to compile                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | About 120 lines     | `is_old_function_syntax` is called at `:719`, and `skip_old_function_syntax` is called at `:725`—**they are active legacy-syntax rejection gates, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) contains only the comment `// 旧语法已移除`, but **the function consumes no tokens when called**—this is an independent behavioral issue (whether the rejection path correctly advances the parser position), not dead code. **Disposition: review the behavior together with 05-frontend-paradigm.md** (Decision D27: remove it naturally as part of the grammar migration), rather than deleting it directly as dead code                                                                                                                                                                                                                                                                                  |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                   | The only occurrences repository-wide are match arms; there are no construction sites. The "no generation sites (dead path)" comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| B9  | `Switch` opcode                                                                  | —                   | `backends/common/opcode.rs` defines it, has a variant, has `opcode()`/`size()` arms, and has a decoding arm, but **there are zero construction sites repository-wide**. The interpreter has an active implementation at `ops/control.rs:90`. **It is explicitly registered as a known dead path by tests**: `tests/bytecode.rs:1035-1040` contains `WHITELIST: &[&str] = &["SWITCH"]` (a non-empty difference fails the test)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B10 | `UnaryOp::Not` (bytecode unary logical NOT)                                      | —                   | **Fixed (WBS 3.20.29, 2026-11-14, issue #562 closed)**. The original defect was that `opcode()` encoding ignored the `op` field (Not and Neg both emitted `I64_NEG`), and the decoder also constructed only `Neg`—the encoding/decoding round trip rewrote logical negation as negation (`.42` direct execution turned `!x` into `-x`; the interpreter's in-memory Neg-on-Bool compensation arm happened to produce the correct answer, so the defect surface was encoding fidelity rather than currently incorrect results). The fix added a dedicated instruction code, `I64_NOT` (0x2C): `opcode()` now branches on the `op` field + a new decoding arm restores `Not` + the translator's `Not` arm emits it + `opcode_name` registers it; the executor's existing `Not` execution arm connects naturally; decoding behavior for the old `.42` `I64_NEG` remains unchanged. See the decision chain in [09a](./09a-wbs-history.md) §P3 3.20.29 |
| B11 | Four `#[allow(dead_code)]` occurrences                                           | —                   | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are genuine dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (a conservative exemption for an extern binding, **reasonably retained**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                   | **Zero production construction sites** (the only construction site manually creates a node in the test at `parser/tests/ast.rs:828`), but it has **12 production consumers + 2 exhaustive-match arms** (the complete list appears in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()` and `pratt/mod.rs:47` `expr_end_line`. Actual function definitions use `Expr::Lambda` (`nud.rs:425-429`) + `StmtKind::Assign` (`declarations.rs:462-479`). Deletion belongs to P6 (the 16-item change list in 03 §5.5)                                                                 |
| B13 | `Instruction::Cast` / `Instruction::TypeTest` (`src/middle/core/ir.rs`)          | —                   | The only occurrences repository-wide are consumer arms, with **no construction sites** (the same pattern as B8), and their payloads are discarded downstream—`translate_cast` cannot obtain `target_type`, and the `TypeTest` arm emits the `vec![0, 0, 0]` no-op operand placeholder. **WBS 6.6.11 deleted these** (both variants + all `normalize`/`verify`/`mono`/`translator` consumer arms + `substitute_block`/`substitute_instruction`/`substitute_type_ast`/`translate_cast`, which consequently lost their sole caller); grep found 0 matches for this form across 216 IR snapshots. **The deletion does not affect syntactic `as` support**, but makes the existing gap that "`as` semantics are unimplemented" more visible ⇒ the gap and evidence chain are recorded in [09a](./09a-wbs-history.md) §P6 6.6.11, with the SPEC-side note in `docs/src/reference/language-spec/syntax.md` §2.6                                         |

## C. Paper-Only Designs

| #   | Target                                                                          | Size                                        | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                    | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, and `HotReloader` have **zero repository-wide matches**. The "known limitations" at `:46-47` admit that `check_incremental` still calls `check_files_with_diagnostics` internally (the full path); `:46` also references `command.rs`'s busy-wait debounce—**`command.rs` contains neither `Instant` nor `recv_timeout`** (the only repository-wide occurrence of `recv_timeout` is in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                    | `:37` claims that the placeholder implementation under `traits/` (coherence / impl_check / object_safety / resolution) is incomplete—**the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero repository-wide matches. `:37` claims that `check_single_module` creates an independent Compiler for each file—**that function does not exist** and appears only in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                                 |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                    | `:14` claims that "watch mode rechecks only affected files", and `:27` claims that "check performs only the first two steps". In reality, `check_project` also performs vendor consistency checks, project role classification, dead-code analysis, and W1006; the `Check` subcommand in `main.rs` **has no `--watch` flag** (all `#[arg]` declarations were checked individually)                                                                                                                                                                               |
| C4  | `RFC-018` LLVM AOT                                                              | 1,037 lines of design / **0 lines of code** | Every AOT match at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` is **in a comment**. `Cargo.toml` has no cranelift / llvm / inkwell dependency. `:861-876` leaves an implementation checklist with every item marked `- [ ]`, yet the status is `accepted`                                                                                                                                                                                                                                                                     |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**   | `028-jit-compiler.md:99-126` requires six files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`; **none of them exist**. The status is `draft`, which is comparatively honest                                                                                                                                                                                                                                                                                                                                                                        |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                                | **Physical evidence**: the filenames precisely correspond to the `ModuleCache` and incremental scheduler from the C1 design. **Someone created test skeletons based on the design, found that the pipeline had no corresponding implementation, and left them unfinished**                                                                                                                                                                                                                                                                                       |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                           | The `//! §6.1: CheckSession 增量检查` at `util/diagnostic/tests/mod.rs:4` and the `§4.4` / `§5.2` references at `mod_tests.rs:4` refer to this **gitignored plan document** (`.gitignore` contains `superpowers/`). These section references are **archaeological references to a deprecated API**                                                                                                                                                                                                                                                               |

### Actual Location of Cross-File Analysis

The cross-file analysis capabilities described by C1–C3 **do genuinely exist**, but at entirely
different paths:

| Component           | Location                              | Lines           |
| ------------------- | ------------------------------------- | --------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439             |
| Resolver            | `src/frontend/module/resolver.rs`     | 195             |
| Role classification | `src/frontend/module/roles.rs`        | 388             |
| Consistency checks  | `src/frontend/module/consistency.rs`  | 206             |
| Orchestration       | `src/frontend/module/orchestrator.rs` | 1,528           |
| **Total**           |                                       | **About 2,670** |

Therefore, the disposition for C1–C3 is **not to remove the capability, but to remove the misleading
documents**, and to rewrite the boundary table in `check/index.md` to point to the actual
architecture under `src/frontend/module/`.

### Clarifying the Disposition of RFC-019

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` **is already under the `deprecated/`
directory**, and the residual check was **clean** (`homoicon` has zero repository-wide matches; the
end of the document contains an honest conclusion stating that, if the work is restarted in the
future, it should begin with proof/prototype experiments rather than with the RFC). **It requires no
additional disposition**—this item appears in the authorized inventory, but verification confirmed
that it is already in its terminal state.

## D. Feature-Level Dead Code: REPL

The four files under `src/repl/` total **1,152 lines**, with **zero `#[cfg(test)]` and zero
`#[test]`**. Meanwhile, `src/main.rs:440` is `args.command.unwrap_or(Commands::Repl)`—**running
`yaoxiang` without arguments directly enters the REPL**.

| #   | Facts                                         | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed**  | It appears only twice under `src/repl/`: its definition at `eval.rs:23` and a **match arm** at `mod.rs:180`. Therefore, `Repl::format_value` (`mod.rs:493-502`) is dead code, and **the REPL does not echo any evaluation result**                                                                                                                                                                                                             |
| D2  | **No cross-turn state**                       | `extract_definitions` (`eval.rs:345-377`) calls only `define_variable` (which stores a **type string**) and never calls `define_var` (which stores a `RuntimeValue`); `wrap_code` does not inject historical definitions either. The `VariableInfo::Value` branch (`eval.rs:23`) is unreachable. `REPLContext.variables` serves completion alone and does not participate in evaluation                                                        |
| D3  | **The wrapper syntax appears to be obsolete** | `eval.rs:314`/`316` produce `main() -> () = () => { code }` (the old-style `name() -> Ret` function declaration header), whereas the current specification form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`; the specification table in RFC-007:94 consists entirely of `name: (a: Type) -> Ret`). **The compiler was not run for empirical verification; if this is the case, every REPL evaluation fails to compile** |
| D4  | `:debug` is a stub                            | `mod.rs:454-466` merely prints two lines and returns; breakpoints are stored (`mod.rs:319`), but **the interpreter never queries them**                                                                                                                                                                                                                                                                                                        |
| D5  | `:history` is a stub                          | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                                                                      |
| D6  | Duplicates `lib.rs`                           | `eval_code` at `lib.rs:85` and `repl/eval.rs:wrap_code` are **two independent implementations of "automatically wrap main"**, and their wrapper forms differ                                                                                                                                                                                                                                                                                   |

## E. Local Cleanup Items

| #   | Location                                                                                                | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1  | `undefined/temp/33-calib/`                                                                              | Empty root-level directory tree. Almost certainly the result of concatenating the string `"undefined"` into a path somewhere. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| E2  | `src/lib.rs:568`, `:582`                                                                                | `dump_type_detail` / `dump_const_detail` each have an `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, so the panic path is foreseeable. It should be changed to meaningful degraded output                                                                                                                                                                                                                                                                                                                                                                                         |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                               | `check_vendor_lock_consistency` and `check_workspace_consistency` are **nearly 60 lines of duplicated logic**, differing only in dependency source and path-dep skipping. **Merge them**                                                                                                                                                                                                                                                                                                                                                                                                                            |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`                  | Legacy function syntax detection and rejection, about 120 lines. The body of `skip_old_function_syntax` (`:120-122`) contains only one comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                    |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                        | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                                 | The "tool design" sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, and `check/incremental-checking`. **If the C1–C3 document removals are performed, this location must be cleaned up in sync**, or the documentation site will contain 404 links                                                                                                                                                                                                                                                                                                                                   |
| E7  | Duplicate `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | Five variants have the same names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` has inconsistent widths (f32 × f64)**; there are no `From` / `TryFrom` implementations, and conversions are handwritten and scattered across call sites (`const_eval.rs` / `ir_gen.rs`). `kind()` at `const_data.rs:44` has a live `todo!()` for `LibraryRef` / `ExternRef`. **Disposition: merge into one definition + add explicit conversions + change the `todo!()` to degraded output; assign to P10 (Category E local cleanup item)**. Registered under the 2026-10-06 P0 gate (criterion A, J=0.56) |
| E8  | Duplicate downstream mappings from `Literal` (Prohibition III A)                                        | `Literal → MonoType` has at least 3 identical arms (`inference/expressions.rs`, `checker.rs`, and `inference/patterns.rs`, such as `Literal::Int(_) => MonoType::Int(64)`); `Literal → ir::ConstValue` (`ir_gen.rs`, 19 handwritten instances) and `Literal → const_data::ConstValue` (`const_eval.rs`, 4 handwritten instances) have no shared conversion point. **Disposition: move the 3 typecheck-side instances to the shared layer under P6; consolidate the `ir_gen` / `const_eval` side together with the E7 `ConstValue` merge under P10.** Registered under the 2026-10-06 P0 gate                        |

### Not Dead Code, but a Similar Problem: Three Hard-Coded Discarded Fields

The `impl From<BytecodeFile>` (`943-2350`, 1,400 lines) in `src/middle/core/bytecode.rs` has three
hard-coded discards:

| Line   | Discarded Content                | Consequence                                          |
| ------ | -------------------------------- | ---------------------------------------------------- |
| `2312` | `upvalue_count: 0`               | Upvalue metadata is lost                             |
| `2315` | `exception_handlers: Vec::new()` | **`.42` artifacts lose their exception table**       |
| `2341` | `globals: Vec::new()`            | **`.42` artifacts lose global-variable information** |

**These are not dead code, but data-loss defects**—`.42` artifacts are complete only on the
in-memory path. **Their disposition differs from that of dead code**: ~~they should be tracked as a
separate issue~~ **all three have been incorporated into P7 batch e (Decision D52, 2026-10-05), with
no separate issue**—`2315` (exception table) and `2341` (global variables) concern artifact
integrity for core language semantics and are not deferrable. All three are fixed together, with the
`.42` `VERSION` raised from 4 to 5 in one change (the format header already has `MAGIC` + `VERSION`
fields at `codegen/bytecode.rs:14-16`). They are listed here only to prevent them from being
mistaken for "dead code" and removed.

## F. Not Deleted, but to Be Flagged or Decided

| #   | Target                                                                                                                                                                                                                                                                                      | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (**handled under P5 5.1**: 1,547 lines at the time, now 1,553 lines and a real `mod`)                                                                                                                                              | **This is not dead code.** At the time, it was spliced into the end of `checker.rs` via `include!` (the only `include!` in the repository). The file **was not declared as any module** (a repository-wide `grep 'mod semantic_tokens'` had zero matches), began directly with `impl TypeChecker {`, and had no `use` header of its own; `checker/` contained only this one file. Consequences included the absence of module identity, failure of visibility isolation, broken rust-analyzer navigation and symbol search, and toolchain treatment of the file as part of `checker.rs`. At the time, the actual `checker` module was 5,618 + 1,547 ≈ 7,165 lines (now `checker.rs` is 4,167 lines + three `checker/` submodules of 1,553/1,107/470 lines). **Fix: convert it to a real `mod`** (completed by [09](09-execution-wbs.md) §P5 5.1 on 2026-10-26, clearing the B1 inventory entry under `check-boundary`), not deletion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| F2  | wasm attributes (both directions counted when they occur on the same line): the current `src/` tree contains the literal `wasm32` in **29** files, with **22** carrying actual attributes and **7** merely mentioning it; `orchestrator.rs` has **10** attributes (4 positive + 6 negative) | **This is not dead code.** wasm targets are carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` sets `crate-type = ["cdylib"]`, `:18` sets `wasm-bindgen = "0.2"`, and `:17` depends on the main crate; the root `Cargo.toml:3` `exclude = ["wasm", ...]` confirms that it is a separate crate. Looking only at the root `Cargo.toml:31` `crate-type = ["rlib"]` would incorrectly classify these branches as defensive dead code—**that inference is invalid**. CI's wasm build is at `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), invoked by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches are load-bearing**: they determine which APIs of the main crate `wasm/src/lib.rs` (currently **152** lines) can call under the wasm target (`src/lib.rs:151` / `:170` / `:187` gate `run_file` / `run_project` / `build_bytecode` as a whole). **Cleanup scope has been determined**: four wasm arms are runtime-unreachable in the playground scenario but load-bearing at compile time (their call sites are not gated); six `not(wasm32)` arms are load-bearing on the native side across the three phase tables ⇒ **there are no directly deletable low-risk items**—the item-by-item determination and evidence appear in the following section, "F2 Determination" (Decision D35: delete unreachable items and retain reachable ones; assign to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                                                                                                                                                                                                                              | It has only a "Status" column (document status) and **no "Implementation Status" column**, so it **cannot express** the error where an RFC is marked complete but the code does not exist. Only 5 of 52 RFCs repository-wide have an `impl_status` field (`027` in-progress; `017`/`014`/`014b` complete; `review/027a` partial). **All 26 accepted RFCs omit implementation status**, including 009, 009a, 013, 018, and 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                                              | **Handled under 4.4.1**: the README now describes the actual call order of `checker.rs::check_module_impl` (the refinement walkthrough goes `predicate` → `termination` → `ownership` → type-constraint solving), with line numbers in parentheses taken from the current tree and function names/driver points used as location anchors; it no longer claims that "a lower-layer failure prevents upper layers from running" (there is in fact no short-circuit); `equivalence` is described according to its actual identity as a "shared type-comparison utility + proof entry point used only by tests". This entry is retained only as a record of the completed disposition and no longer points to the old line ranges                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                                                     | `:79` places RFC-026a in the "Accepted RFCs" table, but its status column says "RFCs under review"; the "RFCs under review" section at `:48-49` lists only 032 and 027a, omitting 026a. This is inconsistent with the three review entries in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| F6  | `tools/cargo-dist/` (`dist.exe`, 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                                                 | **Neither is tracked by Git** (`git ls-files` returns nothing), and both are excluded by `.gitignore`. **This is not a problem**; it is recorded only to prevent later audits from mistakenly concluding that "binaries were committed"                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                                        | `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` reads through the next quote), dependent on the `("E1001", ...)` tuple syntax; it is **not a parser**. Its architecture (`parse` + `validate` + a build.rs gate + `--fix` remediation) is worth reusing for opcode tables, but **the extractor must be rewritten for the opcode form (about 30 lines)**. This belongs to follow-up work in 06-cleanup-inventory.md or to an independent RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                                                                                                                                                                                                                 | There are 83 constants, 0 duplicates, in the range `0x00..0xE2` (span 227, **144 holes**). The same opcode facts are expressed in **5 places**: `BytecodeInstr::opcode()` (`bytecode.rs:558-646`) / `size()` (`649-803`) / `opcode::opcode_name()` (`opcode.rs:120-206`) / the decoding match (`bytecode.rs:978-2302`) / 48 `translate_*` functions (`translator.rs:676-1577`). **The compiler enforces only 2 of them** (the dispatch table in `backends/.../executor/debug.rs:194` + the exhaustive matches in the `ops/*.rs` family functions); a missing arm in `opcode()` / `size()` / the decoder / an encoder **only fails when a `.42` artifact is executed**. The comments in the `size()` table (`649-803`) state that the table does not match the actual encoding (`bytecode.rs:2181-2182`), and **there is no systematic differential test** (the existing size tests cover only Nop/Mov/Borrow/Release). Operator semantics also follow a third parallel set of enums here (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`; `Rem`/`Xor`/`Sar` use yet another naming scheme), to be consolidated as part of the opcode generation-time gate. The target location for the vocabulary is `middle/bytecode/opcode.rs` (to be moved as part of the bytecode-domain consolidation, eliminating the L3→L4 reverse dependency described in 01)                                                                                                                               |

### F2 Determination: Reachability of wasm Attributes in `orchestrator.rs` (Determination of 2026-10-18; No Cleanup)

**Determination surface** = wasm production builds (the shim crate built by `wasm-pack`, as
described in the F2 row); **determination revision** `b5e68b3c9`. This section records only the
determination and does not create any cleanup commitment (cleanup is assigned to P10 / D35).

#### I. Counting Definition (Retaken from the Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- The current `orchestrator.rs` tree has **10 wasm attributes in total** = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- The counting definition is **the `#[cfg(...)]` attribute text on the same line**, counting both
  positive and negative forms (this is also the definition used by the "12" in 02); runtime macros
  such as `cfg!(...)` are not counted—a repository-wide
  `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'` has zero matches.
- The "12" is not an error; it is **a fact from an older revision**: at `544695ea9` (when the
  RFC-039 design document was first committed), there were 5 positive + 7 negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- The difference comes from **a pair of attributes moving out as a pair**: `6ec330d22` (WBS 4.2.2,
  slimming `check_project` into a `Program` constructor) moved `role_context`, together with its
  `not(wasm32)` implementation and `wasm32` stub, to `src/driver/helpers.rs:360/394`—therefore
  `orchestrator.rs` dropped from 12 to 10; the mechanism itself did not disappear.
- Revision-by-revision verification (actually performed this time; excerpts below, with `6ec330d22`
  marking the old/new boundary):

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

#### II. The Three Questions (Splitting Them Prevents Misclassifying Load-Bearing Arms as Dead Code)

1. **Compile-time existence**—does the arm participate in compilation under the target in question?
2. **Runtime reachability**—under that target, is there a call chain from the playground entry
   point?
3. **Deletion cost**—would deleting it break compilation for that target (i.e., are the call sites
   similarly gated)?

#### III. Starting Point (Playground Entry Point and Phase Table)

- Entry points: `wasm/src/lib.rs` (currently 152 lines) `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, which constructs only `Pipeline` and does not
  touch the orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` follows `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`;
  `SingleFile | WasmPlayground` appear together in the table at `:135`): Parsing / Typecheck /
  DeadCodeAnalysis / ProofExecution / IrGeneration / Monomorphization—**not including**
  VendorConsistency / Discovery / Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### IV. Item-by-Item Determination (10 Attributes in the Current Tree)

| #   | Location (`orchestrator.rs`) | Gated Object                                                        | Compiled in wasm Build          | Reachable at Playground Runtime                                                                                                                                                                      | Deletion Cost (Are Call Sites Gated?)                                                                |
| --- | ---------------------------- | ------------------------------------------------------------------- | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 1   | `:271` positive              | wasm arm of `ensure_vendor_consistency` (`:267`) (`Ok(None)`)       | Yes                             | **No**: the sole call site, `src/driver/arms.rs:1195` (the VendorConsistency arm), is not in the playground phase table                                                                              | High: `arms.rs:1195` is not gated, so deleting the arm alone ⇒ wasm compilation fails                |
| 2   | `:418` positive              | wasm arm of `discover_with_used` (`:400`) (empties `lock_versions`) | Yes                             | **No**: call sites are `arms.rs:1221` (the Discovery arm) and `src/driver/helpers.rs:686` (the Check-form reference pool `collect_package_refs`, called only from the Check branch at `arms.rs:521`) | High: same as above                                                                                  |
| 3   | `:655` positive              | wasm stub of `member_context` (`:656-659`)                          | Yes                             | **No**: the sole call site is `:404` (inside the function from #2)                                                                                                                                   | High: `:404` is not gated                                                                            |
| 4   | `:753` positive              | wasm stub of `dep_import_surface` (`:754-756`)                      | Yes                             | **No**: call sites are `:671` (`resolve_in_package_root`) and `:813` (`resolve_in_vendor_with_lock`), rooted at `:468` (`resolve_module_path`, within the BFS of `discover_with_used`)               | High: same as above                                                                                  |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                    | No (not part of the wasm build) | Not applicable                                                                                                                                                                                       | Load-bearing natively: used at `:678` / `:747`                                                       |
| 6   | `:276` negative              | native arm of `ensure_vendor_consistency`                           | No                              | Not applicable                                                                                                                                                                                       | Load-bearing natively: VendorConsistency appears in the three phase tables at `program.rs:61/98/113` |
| 7   | `:413` negative              | native arm of `discover_with_used` (lock reading)                   | No                              | Not applicable                                                                                                                                                                                       | Load-bearing natively: Discovery appears at `program.rs:62/99/114`                                   |
| 8   | `:607` negative              | native `member_context` (workspace resolution)                      | No                              | Not applicable                                                                                                                                                                                       | Load-bearing natively: call site `:404`                                                              |
| 9   | `:677` negative              | manifest-name reading in `resolve_in_package_root`                  | No                              | Not applicable                                                                                                                                                                                       | Load-bearing natively: `:544/:547` ← `:468`                                                          |
| 10  | `:745` negative              | native `dep_import_surface`                                         | No                              | Not applicable                                                                                                                                                                                       | Load-bearing natively: `:671` / `:813`                                                               |

#### V. Conclusions

1. **The four wasm arms are runtime-unreachable in the playground scenario** (#1–#4; evidence = the
   phase table does not include the stage containing their sole call sites), but they are
   **load-bearing at compile time**. Executing D35's literal instruction to "delete unreachable
   items" would fail to compile—the feasible form is to "gate the entire project-mode subtree on
   wasm32 (the discovery/resolution functions in `orchestrator.rs` + the VendorConsistency/Discovery
   arms of the driver + the reference-pool collection in `helpers.rs`)"; the cost and benefit are
   assigned to the P10 decision.
2. **The six `not(wasm32)` arms do not exist in the playground build** (excluded at compile time),
   and all are load-bearing natively (the MultiFile / Check / Lsp phase tables). "Unreachable in the
   playground ⇒ delete" **does not apply**: deleting them by that criterion would directly break the
   CLI/LSP.
3. ⇒ There are **no items in this directory that can be deleted directly at low risk**; the only
   cleanup benefit for F2 is whether the wasm artifact becomes smaller—the point has not been
   verified (see below).

#### VI. Unverified Items (Must Not Be Treated as Conclusions)

- **The wasm artifact size benefit has not been measured**: `wasm-pack build` and size/symbol-table
  tools (`twiggy` / `wasm-objdump`) were not run, so there is no data on "how much smaller the
  artifact would be if the project-mode subtree were gated out"; whether the linker already removes
  unreferenced `pub(crate)` items has also not been verified.
- **wasm32 consumers outside the shim have not been exhaustively enumerated**: this round checked
  only in-repository CI (the sole wasm32 build is at `_build-wasm.yml:30/77`, through the shim
  crate). If a third party compiles the main crate directly for `wasm32-unknown-unknown` and calls
  project-mode APIs, the reachable surface could exceed this determination.
- **On the native side, "load-bearing" means a static call chain + phase table**; no end-to-end
  runtime verification was performed for each arm (the MultiFile/Check/Lsp probes were not run
  individually to exercise every arm).

#### VII. Appendix: Repository-Wide Count Verification (Same Revision)

```console
$ git grep -l wasm32 b5e68b3c9 -- '*.rs' | wc -l
30        # includes the wasm/ shim crate
$ git grep -l wasm32 b5e68b3c9 -- 'src/*.rs' | wc -l
29
$ git grep -lE '#\[cfg\([^]]*wasm32' b5e68b3c9 -- 'src/*.rs' | wc -l
22        # has actual cfg attributes
$ git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs' | wc -l
0         # no runtime cfg! criterion
```

The "wasm current state" in 02 uses the same counting definition as the F2 row: the current tree has
the literal `wasm32` in **29** files / **22** with actual `#[cfg(...)]` attributes / **7** mentioned
only in documentation; `orchestrator.rs` has **16** occurrences = **10** attributes (4 positive + 6
negative) + 6 comments (the 7 files that mention it only: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Oracle Findings Registry (Registered from 2026-10-06 onward as Each Phase Is Implemented)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibitions I A/B/C/D) first
ran on 2026-10-06. Under the division of responsibility in which "the gate reports only suspected
cases and people adjudicate them" ([08](08-maintenance-mechanism.md) §Known Limitations), this
section records the adjudication results and disposition ownership. Unexamined hits (25 pairs
violating criterion A, and all criterion B suspected items) are to be reviewed pair by pair as the
corresponding phases are implemented.

| #   | Target                                                                                                        | Adjudication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Disposition Ownership                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --- | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                 | **Legitimate structure, not parallel representations**: a value/type-tag pair maintained in a 1:1 relationship by the exhaustive `value_type()` (`value.rs:326-369`), so a missing updated arm causes a compilation failure. However, external consumption of `ValueType` is substantively limited to `{:?}` formatting in std/os/fs/net error paths and the constant `Void` at `executor.rs:410`; its 21 variants greatly exceed actual needs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | P10 S5 slimming candidate: reduce to `pub(crate)` and assess trimming; when S5 is implemented, adjudicate this together with the necessity of the `AsyncValue.value_type` field (`value.rs:217`)                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| G2  | Duplicate `ConstValue` (criterion A, J=0.56)                                                                  | **True positive**, including inconsistent f32/f64 widths and a live `todo!()` in `kind()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Recorded under E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| G3  | 11 string-based type-name dispatches (criterion D true positives)                                             | **True positives, all with the same root cause**: there is no single authoritative identifier for built-in container names. List: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), the size table at `eval/const_eval.rs:498-511` (including the dead `"Uint"` arm), `middle/passes/codegen/bytecode.rs:744-756` `to_type_id` (`"Set" => 25` conflicts with decision 4 of #300), and `trait_data.rs:187` `"Clone" \| "Dup"`                                                                                                                                                                                                                                                                                                                | **P6 extended verification item**: these points are on the typecheck / eval / middle side and are outside the parser data-flow coverage in 03 §5.4; when P6 is implemented, handle them under the same principle as 03 §5.3 (`NameKind` is decided once, and match arms no longer compare strings). The duplicate table has already been corrected to seven entries (03 §2.3)                                                                                                                                                                                                                                                                       |
| G4  | 4 criterion D false positives                                                                                 | **Legitimate lookup / dispatch tables, not cases to be consolidated**: `package/build/requirements.rs:17-25` tool installation instructions, `package/vendor/mod.rs:134-139` build-result display names, `repl/mod.rs:241-396` REPL command aliases (8 arms; user-interface aliases are legitimate UX), and `util/diagnostic/mod.rs:462-468` `parse_runtime_mode` (a documented historical alias with a single boundary point)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | When P9 is converted to a hard gate, add a `// reason:` exemption to each; the exemption count should be reviewed when the gate report is inspected                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| G5  | `ReleasePlan.drops` span-key mismatch (measured under P2 2.4.4, not a P0 hit)                                 | **True positive, contract defect—confirmed by D20's prediction**: ownership records keys by the span of the **last expression used** (`ownership.rs:2574 build_release_plan`), while `ir_gen.rs:1956` looks them up by the span of the **statement**; expression-level keys never match—a corpus-wide test showed **168 files / 373 keys** unconsumed, silently losing drops (per-case evidence appears in the file header of `src/middle/core/tests/release_plan_spans.rs`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | **P4/D20** (Span key → PlanId; the behavioral surface spans about 168 files and must wait for the C2 oracle and corpus differential); the `test_release_plan_spans_consumed` oracle remains permanently red (`#[ignore]`), and becomes green when D20 is implemented                                                                                                                                                                                                                                                                                                                                                                                |
| G6  | Three IR metadata defects (measured during P2 2.1.2's `verify_loose` corpus run, fulfilling D38's prediction) | **Three true positives, all fixed under 2.1** (the corpus differential baseline self-comparison produced zero differences, proving behavioral equivalence): ① the locals table for method IR was truncated to the parameter count (`ir_gen.rs`, originally `take_cur_locals(param_types.len())`)—`local_count` was underestimated, and the runtime silently accommodated it through out-of-bounds resizing in `Frame::set_slot` (`frames.rs:103-105`); ② unannotated parameters in lambda / curry intermediates / function paths were discarded by `filter_map` (three locations), misaligning parameter count and frame layout—fill them with `Int(64)` placeholders, following the existing method-path convention; ③ the match-result slot depended on implicit Void pre-initialization of the frame (the fallback point when the last arm's guard fails has no defined path on the CFG)—pre-initialize Void at the `generate_match_expr_ir` entry point | Fixed (same batch as 2.1). **The narrowed determination surface is recorded here** (`verify.rs`'s file header is authoritative): temp-slot dominance is not checked (Void pre-initialization is frame semantics on which ir_gen intentionally depends); the trusted source for type consistency is narrowed to parameter-slot signatures + constants (ir_gen never writes `LocalSlot.ty`, and the full table uses `Int(64)` placeholders—the complete determination surface belongs to P7 value table 04 §262); `init_locals` is a #368 debugging view table, not an allocation limit, and out-of-range `Local` values under `init` are not checked |

## H. Inventory of Giant Functions (Pre-Release Audit Definition, Effective 2026-10-16)

> Definition (established by `/pre-release-audit` on 2026-10-16): a function span greater than 150
> lines is an **initial-screening threshold**, not a command to "report every one" of them—listing
> everything turns the inventory into a pile of line counts and drowns the signal from this round in
> the existing backlog. Each report entry must contain three numbers: **host line count / lines
> inserted this round / lines before insertion**. Divide the entries into two categories: **A.
> Existing giant functions that gained branches** (>150 lines and >50 lines inserted this round);
> **B. Newly added large functions** (almost entirely new and >100 lines). The line numbers below
> are taken from audit revision `1935aab53` (the `v0.8.4..1935aab53` range).

### A. Existing Giant Functions That Gained Branches

| Function                | Location                                                    | Host Lines | Inserted This Round | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | ------------------- | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1,594      | 358                 | 1,236            |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171                 | 642              |

### B. Newly Added Large Functions

| Function                     | Location                                            | Lines | Newly Added |
| ---------------------------- | --------------------------------------------------- | ----- | ----------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177         |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122         |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130         |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107         |

**Existing giant functions not included in the inventory** (≤4 lines inserted this round; recorded
but not acted upon): `infer_expr` (`expressions.rs:2999`, 955 lines), `collect_function_signature`
(`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`, 441 lines),
`generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), and others.

### Disposition Approach

- **Category A**: Resolve these together with the follow-up decomposition tasks for
  `infer_call_expr` / `generate_call_expr_ir`—they are the repository's heaviest hosts, and this
  round's named-argument refactoring inserted 358 lines into the former.
- **Category B**: `push_joined` is a **local** helper for `Task` work-list-style type formatting
  (its host is the actual 180-line addition), and its responsibilities differ from those of
  `mono.rs:123 increment_level_string` (174 lines, rewriting level names), so **do not merge them**;
  `consumer_stage_skipped` and `local_count_of` can be decomposed independently.

## Key Decisions and Rationale

The authorization is "permission to delete outright", but this document recommends **different
dispositions** for several items:

| Category                                                               | Disposition                                     | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions or implementation) | **Delete**                                      | B1 (415 lines), B3 (the dispatch chain), B6 (96 lines), B7 (the empty function), C6 (14 lines), E1 (the empty directory)                                                                                                                                                                                                                                                                                                                                                                            |
| **Paper-only design documents**                                        | **Delete or rewrite**                           | Delete C1 and C2, then rewrite them to point to the actual implementation; rewrite C3's boundary table; **the `config.js:259-268` cleanup (E6) must be performed in sync**                                                                                                                                                                                                                                                                                                                          |
| **RFCs whose status does not match implementation**                    | **Change status**                               | Move C4 (RFC-018) back to `draft` or add `impl_status: 'not-started'`; add `impl_status: 'not-started'` to C5 (RFC-028). **Do not delete them**—the design itself may have value; the problem is the inaccurate status marker                                                                                                                                                                                                                                                                       |
| **Orphan tests with real assertions**                                  | **Revive, do not delete**                       | **1,005 lines / 78 tests** in total: lexer 629 lines / 55 tests + the 311 lines / 16 tests hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + `package/template/tests/` 65 lines / 7 tests. **Reviving them costs one `mod` declaration and restores this coverage to CI**. Deleting them would be a net loss                                                                                                                                                          |
| **Zero-call but semantically important `pub` items**                   | **Reduce visibility rather than delete**        | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move them to `pub(crate)` so the compiler can identify them, or add an implementation. **`is_subtype` in B4 must be retained**                                                                                                                                                                                                                                                                                                                     |
| **Opcodes / instructions that cannot produce a value**                 | **Add construction sites or delete the opcode** | B8 (`TailCall`), B9 (`Switch`; the opcode round-trip test already explicitly registers it through `WHITELIST=["SWITCH"]`). B10 was fixed under D33 (2026-11-14: `opcode()` distinguishes the `op` field + dedicated code `I64_NOT`; see [09a](./09a-wbs-history.md) §P3 3.20.29) and was removed from this backlog. The remaining items require one of three choices and cannot remain as they are; D32/D33/D34 have been adjudicated and are to be handled together with the `.42` version upgrade |
| **Data-loss defects**                                                  | ~~Separate issue~~ **P7 batch e (D52)**         | The three hard-coded discards in Section E are not dead code                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| **Valid but lacking module identity**                                  | **Refactor, do not delete**                     | F1 (`semantic_tokens.rs`): change `include!` to a real `mod` (see `02`)                                                                                                                                                                                                                                                                                                                                                                                                                             |
| **Items requiring a decision**                                         | **List as open questions**                      | F2's wasm32 cleanup is registered under [09](09-execution-wbs.md) §P10 **S7** (gate the project-mode subtree on wasm32; user decision B on 2026-10-18; the three unverified items are retained verbatim); layer-order correction will expose new diagnostics                                                                                                                                                                                                                                        |

### Directions Not Adopted

- **Treat everything as "outright deletion" and delete orphan tests as well**—the 1,005 lines of
  tests with real assertions are an asset. The deletion cost is one `mod` declaration, while the
  benefit is restoring coverage to CI. The authorization means "you may delete", not "you should
  delete".
- **Only add gates without changing code**—gates can prevent regressions, but B1's 415 lines, the
  dispatch chain, and the 96-line dead ladder **already exist**; gates would merely freeze them in
  the baseline.
- **Keep RFC-018 / RFC-028 at `accepted` and merely supplement the
  documentation**—`rfc/index.md:131` defines "accepted" as "entered the implementation phase", so
  readers would reasonably assume an implementation exists. `TRACKING.md` already provides precedent
  for the `impl_status` field (used by 5 RFCs), and the cost of adding the field is far lower than
  the cost of misleading status information.

## Implementation Notes

**This document does not require an equivalence oracle** (it belongs to the
[C6 pure-deletion category](07-equivalence-oracle.md)), but **references must still be confirmed
item by item**. In RFC-039's global phase sequence, S1 corresponds to P1, and S2–S6 correspond to
P10.

| Phase | Contents                                                                                                                                      | Acceptance                                                                                                                                                                                                                                                                                   |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphan tests with assertions (**1,005 lines / 78 tests**)                                                                              | The test counts for the lexer / pratt / passes / emitter / template modules increase under `cargo test`; **real defects are expected to be exposed** (the overflow paths in `literals.rs` and invalid `\x`/`\u` escape sequences have never been tested), so time must be reserved for fixes |
| S2    | Delete pure placeholder dead code (B1/B3/B6/B7 under the 05 closure/C6/E1)                                                                    | `cargo test` passes; `cargo clippy -D warnings` introduces no new warnings                                                                                                                                                                                                                   |
| S3    | Dispose of paper-only design documents (C1–C3 + E6 cleanup of `config.js` in sync)                                                            | The documentation site builds without 404s; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                                          |
| S4    | Correct RFC status (C4/C5/F3—add an "Implementation Status" column to `TRACKING.md`)                                                          | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` is regenerated automatically; **do not edit it manually**)                                                                                                                                                                      |
| S5    | Reduce the visibility of `pub` items (B2/B4)                                                                                                  | The compiler can identify the genuine dead code                                                                                                                                                                                                                                              |
| S6    | Decide and implement items for opcodes that cannot produce values (B8/B9/F8 pending, following D32/D33/D34; **B10 is complete**)              | Delete the dead paths for `Switch` / `TailCall`; implement the F8 gate; B10 is complete (2026-11-14, WBS 3.20.29: `opcode()` distinguishes the `op` field + dedicated code `I64_NOT`; see [09a](./09a-wbs-history.md) §P3 3.20.29)                                                           |
| S7    | Convert `include!` (F1, with implementation steps assigned to [09](09-execution-wbs.md) §P5 5.1) + correct layer order (F4, under P4's 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                                                                   |

**S1 must be done first, and time for defect fixes must be reserved.** Its value is not "cleanup";
it is **restoring 1,005 lines / 78 previously unrun tests to CI**—until then, every judgment about
test coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners in `literals.rs` have four isomorphic
  branches that "continue consuming after overflow without reporting an error", the entire
  `scan_leading_dot` function, and all invalid `\x`/`\u` escape paths have never been tested.
  Reviving the tests will probably require code fixes, delaying the subsequent refactoring.
- **B9/B10 involve language behavior decisions**. Whether to "add construction sites" or "delete the
  opcode" for `Switch` and `UnaryOp::Not` requires considering whether these are surface behaviors
  on which published corpora depend.
- **After C1–C3 are deleted, cross-file analysis capability documentation will be temporarily
  absent**, until `check/cross-file-analysis.md` is rewritten to point to the actual architecture
  under `src/frontend/module/`.
- **Reachability analysis can only discover "unreachable" code, not "reachable but meaningless"
  code**. Judgments beyond Form 2 must be made item by item, and **this inventory does not
  constitute an exhaustive proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  cannot be discovered automatically by the compiler and must be found through searches.
- **Deleting the half-finished remnants of RFC-023** (B1) fulfills the decision that "closures have
  been removed from the language"; **correcting misleading status information** (C4/C5/F3) restores
  `TRACKING.md`'s reference value.

> **All open questions originally listed in this section have now been adjudicated.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> item-by-item decisions. **This document leaves no pending items.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  the source decision for B1's remnants
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — B3's unwired `§4.1` runtime check insertion
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — the formal home of C1/C6
- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — the overarching specification
- [Refactoring Equivalence Oracle](07-equivalence-oracle.md) — oracle requirements for category C6
- `src/frontend/core/typecheck/layers/README.md` — the actual call order of the proof layers and the
  true identity of `equivalence` (F4 handled)
- `build.rs:19-55` — an example of this repository's generation-time gate
