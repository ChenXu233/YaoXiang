# Dead Code and Paper-Only Design Cleanup

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> The four-layer model, tiered acceptance criteria, and execution phase order are covered in the
> main RFC-039; the role of each companion document is listed in the [directory index](index.md).

## Purpose and Scope

This article provides an inventory of dead code and paper-only designs in the YaoXiang repository,
the methodology used to identify them, item-by-item disposition decisions, and the execution order.
There are four categories of cleanup targets:

| Category                                                                  | Size                                                                   |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Files that have never participated in compilation                         | 24 files / 2,628 lines (of which 23 are genuine orphans / 1,081 lines) |
| Dead code that is compiled but never called                               | 1,500+ lines                                                           |
| Design documents describing an architecture that never existed            | Three documents under `docs/src/dev/design/check/`                     |
| Features attached to the default entry point, with no tests and no output | 1,152 lines under `src/repl/`                                          |

**The authorized scope is “permission for outright deletion”** (decision dated 2026-10-03). However,
this article recommends a disposition **opposite to “delete directly”** for several items—the
reasons are given in [Key Decisions and Rationale](#关键决策与理由).

This article **does not cover**: module boundaries and dependency direction (see `01-routing.md`),
stage contracts (see `02-stage-contract.md`), or criterion design (see `07-equivalence-oracle.md`).
The entire `layers/dispatch.rs` chain recorded in B3 is unwired; its removal is assigned to S2.

## Current State

### The Three Forms of Dead-Code Debt

**Form 1: Placeholder.** The code exists and appears to be doing something, but in reality does
nothing.

- The entire chain in `src/frontend/core/typecheck/layers/dispatch.rs` (`dispatch` /
  `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck`) has **zero call sites** in
  the compiler. It is the “runtime check insertion” mechanism designed by `RFC-027 §4.1` and is
  **completely unwired**.
- `check_type_equivalence` (`ProofContext` entry point) at
  `src/frontend/core/typecheck/layers/equivalence.rs:216` is called only by tests. **Note that**
  `is_subtype` in the same file is used in production by `inference/assignment.rs:17`, so **the
  entire file cannot be deleted**.
- `src/backends/runtime/engine.rs` (1,047 lines) is a pure DAG task scheduler. Its `pub` surface
  consists only of `TaskPoll` / `ResourceKey` / `TaskMeta` / `TaskCancelReason` / `TaskOutcome` /
  `RuntimeStats` / `RuntimeError` / `LocalRuntime`. It **is not a second execution backend**—the
  `Executor` trait at `backends/mod.rs:356` currently has only one implementor (the interpreter).
  This point needs clarification because the directory name is misleading.

**Form 2: Residue.** An upstream decision has been made, but the deletion was only half completed.

- `src/middle/passes/mono/instance.rs:416-830` (**415 lines**) retains six types: `FunctionInstance`
  (416), `GenericClosureId` (562), `ClosureId` (628), `ClosureInstance` (724),
  `ClosureSpecializationKey` (760), and `CaptureVariable`. **The source is clear**: when `RFC-023`
  deprecated the “closure capture model,” it declared that “**roughly 850 lines of code would be
  deleted along with the closure model**.” These 415 lines are the residue that survived from those
  850 lines—**the deletion was only half completed**. **Verification**: a repository-wide
  word-boundary search for these six types returns **zero matches outside `instance.rs`** (including
  tests). `pub use passes::mono::instance::*;` at `middle/mod.rs:40` is merely a glob re-export and
  does not constitute consumption.

**Form 3: False Coverage.** This is the most dangerous form—**it makes the repository appear more
reliable than it actually is**.

`src/frontend/core/lexer/mod.rs` has only five `pub mod` declarations (`literals` / `state` /
`symbols` / `tokenizer` / `tokens`) plus one
`#[cfg(test)] #[path = "tests/fstring.rs"] mod fstring_tests;` (`:104-106`). **Nothing declares
`mod tests;`**, so the entire `lexer/tests/` subtree—13 files and 686 lines—**has never participated
in compilation**.

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
`mod tests;`; `mod.rs`, 38 lines, is a declaration file with no assertions.)

**Only one file is actually running at the lexical layer, with 12 tests, and all of them test only
the raw brace protocol of f-strings.**

The danger of this problem lies in the fact that `cargo test` will not fail (tests that never ran
cannot fail), and CI has no test-count baseline or wiring check.

### Paper-Only Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**. This is not a case of “designed but not implemented”; rather, **the documents describe a
different system that never existed**, and they present “a possible future design” as “a defect in
an implemented system.”

## Methodology and Confidence Statement

### Reachability Analysis Method

Starting from the two crate roots, `src/lib.rs` and `src/main.rs`, the analysis resolves the `mod` /
`pub mod` / `#[path]` declarations of all 509 `.rs` files, constructs a module graph, and performs
BFS reachability analysis. `include!` targets are evaluated separately and are not counted toward
reachability.

**This method has known false-positive patterns, two of which were encountered while writing this
article:**

1. `Split-Path` returns `\` on Windows, which does not match forward-slash keys → this briefly
   produced a false report of 499 orphans.
2. **The submodule directory for a non-`mod.rs` file is `its-own-name/`, not `parent-directory/`** →
   `orchestrator.rs`’s `mod tests` was resolved to `src/frontend/module/tests/`, nearly causing 753
   lines of live tests to be classified as dead code.

Therefore, **every conclusion was double-checked by “actually reading the file + actually searching
for references.”** Items that did not pass this secondary check are not included in this article.

### Line-Count Convention

All line counts in this article were measured with `(Get-Content).Count`. **Note that
`Measure-Object -Line` skips blank lines and systematically undercounts by about 5%** (for example,
`manifesto.md` is reported as 658 lines with `-Line`, but actually has 893). An earlier round of
analysis in this repository produced a lower total because of this discrepancy; the figures here
have been corrected.

### Known Limitations

- **Reachability analysis can only find “unreachable” code, not “reachable but meaningless” code**.
  Forms other than Form 2 require item-by-item manual judgment.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types must be
  found through searches; this method does not cover them.
- This article does not involve running the compiler, so all “never ran” conclusions are static
  determinations.

## A. Files That Have Never Participated in Compilation

**Total: 24 files / 2,628 lines.** `semantic_tokens.rs` belongs to Form 4 (see Section F); the
remaining **23 files / 1,081 lines are genuine orphans**.

| Path                                                          | Size                    | Contents                                                                      | Basis for Determination                                                                                                                                                                               |
| ------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entire `src/frontend/core/lexer/tests/` subtree, 13 files     | 686 lines               | 7 empty shells (19 lines) + `mod.rs` 38 lines + **629 lines / 55 real tests** | `lexer/mod.rs:4-8` declares only five `pub mod`s; the only test declaration at `:104-106` is `#[path]` pointing to `tests/fstring.rs`. A repository-wide `grep 'mod tests'` matches only `mod.rs:105` |
| 3 files under `src/frontend/pipeline/tests/`                  | 14 lines                | All are placeholder documentation comments, with 0 assertions                 | The full 740-line `src/frontend/pipeline.rs` contains no `mod` / `pub mod` / `#[cfg(test)]` / `#[path]` declarations (`Select-String` returns zero matches)                                           |
| 3 files under `src/package/template/tests/`                   | 65 lines                | `gitignore.rs` 27/3 + `main_yx.rs` 34/4 = **7 `#[test]`s**                    | The 7-line `src/package/template/mod.rs` declares only `mod gitignore; mod main_yx;`, with no `mod tests;`                                                                                            |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs`   | 95 lines / **6 tests**  | Hidden inside a **live** directory                                            | `pratt/tests/mod.rs` declares only `led` / `nud` / `precedence`                                                                                                                                       |
| `src/frontend/core/typecheck/passes/tests/overload_inline.rs` | 170 lines / **7 tests** | Same as above                                                                 | `passes/tests/mod.rs` declares only `dead_code` / `overload`                                                                                                                                          |
| `src/util/diagnostic/emitter/tests/json.rs`                   | 46 lines / **3 tests**  | Same as above                                                                 | `emitter/tests/mod.rs` declares only `ansi` / `text`                                                                                                                                                  |

**The last three items total 16 tests / 311 lines of code that have never run.** They are hidden
inside active directories whose `mod.rs` files look complete, making them more deceptive than an
entirely orphaned directory.

Two other files under `lexer/tests/` belong to the same “declared but missing declaration” form, but
are already included in the 629-line figure in the first row and are **not counted twice**:
`lexer_mod.rs` (89 lines / 5 tests; `lexer/tests/mod.rs` declares 11 `mod`s but omits
`mod lexer_mod;`) and `symbols.rs` (70 lines / 3 tests, likewise omitted).

**Total real tests that have never run: 1,005 lines / 78 tests**: lexer 629/55 + pratt / passes /
emitter 311/16 + package/template 65/7.

**Pay particular attention to `precedence_inline.rs`**: it tests the `Precedence` enum +
`PrecedenceContext` (96 lines of dead code) that have **zero production uses** in `precedence.rs`.
Even if the test were wired in, it would still be testing a dead path.

Another item hidden among the 23rd orphan is `src/frontend/core/typecheck/tests/semantic_db.rs` (5
lines, zero tests): `typecheck/tests/mod.rs` declares 23 test modules but omits this one—the file
has only a `use` left and is a pure remnant. When S1 revives the suite, either delete it together
with the wiring work or add assertions.

## B. Dead Code That Is Compiled but Never Called

| #   | Location                                                                         | Size                    | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | -------------------------------------------------------------------------------- | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| B1  | `src/middle/passes/mono/instance.rs:416-830`                                     | 415 lines / 6 types     | A repository-wide word-boundary search returns zero matches outside `instance.rs`, including tests. Source: RFC-023 declared the removal of roughly 850 lines, but only half was removed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| B2  | `src/frontend/core/typecheck/inference/types.rs`                                 | 45 lines                | `TypeSystem` + 4 pure functions. The only reference chain is `pub mod types;` at `inference/mod.rs:11` and `pub use types::TypeSystem;` at `:24` (both mere re-exports). The repository contains only five `TypeSystem` matches: 2 definitions, 1 re-export, and **2 unrelated** occurrences of `lexer/symbols.rs:503 TypeSystemValidator`. The four methods have zero call sites. **Because they are `pub`, the compiler never reports them**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| B3  | Entire chain in `src/frontend/core/typecheck/layers/dispatch.rs`                 | —                       | `dispatch` / `dispatch_pipeline` / `DispatchMode` / `RuntimeOutcome::InsertCheck` have zero call sites. Runtime check insertion from RFC-027 §4.1 is unwired. Note that it is declared by `layers/mod.rs:6`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| B4  | `src/frontend/core/typecheck/layers/equivalence.rs:216`                          | —                       | `check_type_equivalence` is called only by tests. **`is_subtype` in the same file must be retained** (`inference/assignment.rs:17` uses it in production)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| B5  | `src/frontend/core/typecheck/layers/termination.rs:959`                          | Strategy 2              | Described in its own source as a “framework placeholder”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| B6  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                 | 96 lines                | `Precedence` enum + `PrecedenceContext`. **Their only external referrer is `pratt/tests/precedence_inline.rs`** (95 lines / 6 tests)—and that test file has never been compiled (`pratt/tests/mod.rs` declares only `led`/`nud`/`precedence`). Therefore, the two **must be deleted together**: deleting only the test leaves 96 lines of dead code, while deleting only the enum causes the test to fail to compile                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| B7  | `src/frontend/core/parser/statements/declarations.rs:82`, `:120-122`, `:145-167` | Approximately 120 lines | `is_old_function_syntax` is called at `:719`, and `skip_old_function_syntax` is called at `:725`—**they are active gates that reject old syntax, not unreferenced dead code**. The body of `skip_old_function_syntax` (`:120-122`) contains only the comment `// Old syntax has been removed`, but **the function consumes no tokens when called**—this is an independent behavioral issue (whether the rejection path correctly advances the parser position), not dead code. **The disposition is to review the behavior together with 05-frontend-paradigm.md (Decision D27: remove it naturally as part of the grammar migration), rather than delete it directly as dead code**                                                                                                                                                                                                                                                                               |
| B8  | `Instruction::TailCall` (`src/middle/core/ir.rs`)                                | —                       | **Deleted (6753bbe, P10 S6.1)**. The repository contains only match arms and no construction sites. The “no generation point (dead path)” comment at `bytecode.rs:2156` is **accurate** (the encoding branches at `translator.rs:530`/`1109` exist but are unreachable)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B9  | `Switch` opcode                                                                  | —                       | **Completed (7e.1 `92b3229` reclaimed the opcode value and removed the variant)**. `backends/common/opcode.rs` defines it, it has a variant, `opcode()`/`size()` arms, and a decoding arm, **but there are zero construction sites anywhere in the repository**. The interpreter has an active implementation at `ops/control.rs:90`. **It is explicitly registered by tests as a known dead path**: `tests/bytecode.rs:1035-1040` defines `WHITELIST: &[&str] = &["SWITCH"]` (a non-empty difference causes failure)                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| B10 | `UnaryOp::Not` (bytecode unary logical NOT)                                      | —                       | **Fixed (WBS 3.20.29, 2026-11-14, issue #562 closed)**. The original defect = `opcode()` encoding ignored the `op` field (Not and Neg both emitted `I64_NEG`), and the decoder constructed only `Neg`—the encoding/decoding round trip rewrote logical NOT as negation (when run directly against `.42`, `!x` became `-x`; the interpreter’s in-memory path accidentally compensated through its Neg-on-Bool arm and therefore produced the right answer, so the defect surface was encoding fidelity rather than a currently incorrect result). Fix = add a dedicated instruction code `I64_NOT` (0x2C): `opcode()` branches on the `op` field + a new decoder arm reconstructs `Not` + the translator’s `Not` arm emits it + `opcode_name` registers it; the executor’s existing `Not` execution arm is naturally connected; the old `.42` `I64_NEG` decoding behavior remains unchanged. The criterion chain is recorded in the commit message for task 3.20.29 |
| B11 | Four `#[allow(dead_code)]` occurrences                                           | —                       | `ownership.rs:36` `ParamOwnership` / `emitter/text.rs:237` `hint_prefix` (these two are genuine dead code); `proof/smt/z3_ffi.rs:104` `Z3_solver_get_reason_unknown` (a conservative exemption for an extern binding, **reasonable to retain**); `std/tests/stdlib_docs.rs:250` `docs_path_for` (a test helper)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B12 | `src/frontend/core/parser/ast.rs:37-43` `Expr::FnDef`                            | —                       | **Zero production construction sites** (the only construction site manually creates a node in the test at `parser/tests/ast.rs:828`), but there are **12 production consumers + 2 exhaustive arms** (the complete list is in 03-type-unification.md §2.6): `spawn/placement.rs:122`, `spawn/analysis.rs:912`, `formatter/handlers/expr.rs:43`, `frontend/module/orchestrator.rs:1492`, `ir_gen.rs:4325/5097`, `checker.rs:1642`, `semantic_tokens.rs:1416`, `inference/expressions.rs:3374`, `inference/existential.rs:55`, `passes/dead_code.rs:305`, `layers/ownership.rs:1207`, `layers/termination.rs:752`, plus `ast.rs:1026` `Expr::span()` and `pratt/mod.rs:47` `expr_end_line`. Actual function definitions take the `Expr::Lambda`(`nud.rs:425-429`) + `StmtKind::Assign`(`declarations.rs:462-479`) path. Removal is assigned to P6 (the 16-item change list in 03 §5.5)                                                                                |
| B13 | `Instruction::Cast` / `Instruction::TypeTest` (`src/middle/core/ir.rs`)          | —                       | The repository contains only consumer arms and **no construction sites** (the same form as B8), and their payloads are discarded downstream—`translate_cast` cannot obtain `target_type`, while the `TypeTest` arm emits the empty placeholder `vec![0, 0, 0]`. **WBS 6.6.11 deleted them** (both variants + all `normalize`/`verify`/`mono`/`translator` consumer arms + `substitute_block`/`substitute_instruction`/`substitute_type_ast`/`translate_cast`, which consequently lost their only caller); a grep of 216 IR snapshots finds zero matches for this form. **The deletion does not affect syntax-level `as` support**, but it makes the existing gap that “`as` semantics are not implemented” more visible ⇒ the gap and evidence chain are registered in the commit message for task 6.6.11; see the SPEC-side note in `docs/src/reference/language-spec/syntax.md` §2.6                                                                             |

## C. Paper-Only Designs

| #   | Subject                                                                         | Size                                        | Facts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | `docs/src/dev/design/check/incremental-checking.md`                             | 53 lines                                    | The described `CheckSession` (including the Rust code draft at `:31-42`), `ModuleDependencyGraph`, `affected_modules`, `ModuleCache`, and `HotReloader` have **zero repository-wide matches**. The “known limitations” at `:46-47` admit that `check_incremental` still calls `check_files_with_diagnostics` internally (the full-analysis path); `:46` also cites the busy-wait debounce in `command.rs`—yet **`command.rs` contains neither `Instant` nor `recv_timeout`** (the only repository-wide occurrence of `recv_timeout` is in `backends/runtime/tests/facade.rs`) |
| C2  | `docs/src/dev/design/check/cross-file-analysis.md`                              | 43 lines                                    | `:37` claims that the placeholder implementations under `traits/` (coherence / impl_check / object_safety / resolution) are incomplete—but **the `traits/` directory does not exist** (`Test-Path` = False), and these identifiers have zero repository-wide matches. `:38` claims that `check_single_module` creates a separate Compiler for each file—**the function does not exist** and appears only in a comment at `util/diagnostic/tests/mod_tests.rs:248`                                                                                                             |
| C3  | `docs/src/dev/design/check/index.md`                                            | 29 lines                                    | `:14` claims that “watch mode rechecks only affected files,” and `:27` claims that “check only performs the first two steps.” In reality, `check_project` also performs vendor consistency checks, project-role classification, dead-code analysis, and W1006; the `Check` subcommand in `main.rs` **has no `--watch` flag** (all `#[arg]` declarations were checked individually)                                                                                                                                                                                            |
| C4  | `RFC-018` LLVM AOT                                                              | 1,037 lines of design / **0 lines of code** | AOT matches at `backends/mod.rs:5,16,355` + `runtime/facade.rs:139` + `diagnostic/error.rs:8` are **all comments**. `Cargo.toml` contains no cranelift / llvm / inkwell. `:861-876` contains an implementation checklist whose entries are all still `- [ ]`, yet its status is marked `accepted`                                                                                                                                                                                                                                                                             |
| C5  | `RFC-028` JIT                                                                   | 419 lines of design / **0 lines of code**   | `028-jit-compiler.md:99-126` requires six files under `src/backends/jit/` + `middle/passes/ir_normalize.rs`; **none exist**. Its status is `draft` (comparatively honest)                                                                                                                                                                                                                                                                                                                                                                                                     |
| C6  | `src/frontend/pipeline/tests/compilation_cache.rs` + `incremental_scheduler.rs` | 3 lines each                                | **Artifact-level evidence**: the file names correspond exactly to `ModuleCache` and incremental scheduling from the C1 design. **Someone created test skeletons based on the design, found that the pipeline had no corresponding implementation, and left them unfinished**                                                                                                                                                                                                                                                                                                  |
| C7  | `docs/superpowers/specs/2026-05-29-yaoxiang-check-improvement-design.md`        | —                                           | `util/diagnostic/tests/mod.rs:4` (`//! §6.1: CheckSession incremental checking`) and `mod_tests.rs:4` (`§4.4` / `§5.2`) reference this **gitignored planning document** (`.gitignore` contains `superpowers/`). These § references are **archaeological references to an obsolete API**                                                                                                                                                                                                                                                                                       |

### The Actual Location of Cross-File Analysis

The cross-file analysis capabilities described by C1-C3 **do exist**, but under completely different
paths:

| Component           | Location                              | Lines                   |
| ------------------- | ------------------------------------- | ----------------------- |
| Module registry     | `src/frontend/module/registry.rs`     | 439                     |
| Resolver            | `src/frontend/module/resolver.rs`     | 195                     |
| Role classification | `src/frontend/module/roles.rs`        | 388                     |
| Consistency checks  | `src/frontend/module/consistency.rs`  | 206                     |
| Orchestration       | `src/frontend/module/orchestrator.rs` | 1,528                   |
| **Total**           |                                       | **Approximately 2,670** |

Therefore, the disposition for C1-C3 is **not to remove the capability, but to remove misleading
documentation** and rewrite the boundary table in `check/index.md` to point to the actual
architecture under `src/frontend/module/`.

### Clarifying the Disposition of RFC-019

`docs/src/rfc/deprecated/019-typed-homoiconicity.md` is **already under `deprecated/`**, and the
residual review found it **clean** (`homoicon` has zero repository-wide matches, and the document
ends with the honest conclusion, “If this work is ever restarted in the future, begin with
proofs/prototype experiments rather than the RFC”). **It requires no additional disposition**—this
item is on the authorization list, but review confirmed that it is already in its terminal state.

## D. Feature-Level Dead Code: REPL

`src/repl/` contains four files totaling **1,152 lines**, with **zero `#[cfg(test)]` and zero
`#[test]`**. Meanwhile, `src/main.rs:440` uses `args.command.unwrap_or(Commands::Repl)`—**running
`yaoxiang` without arguments directly enters the REPL**.

| #   | Fact                                                | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **`EvalResult::Value` is never constructed**        | It occurs only twice under `src/repl/`: the definition at `eval.rs:23` and the **match arm** at `mod.rs:180`. Therefore, `Repl::format_value` (`mod.rs:493-502`) is dead code, and **the REPL does not echo any evaluation results**                                                                                                                                                                                                  |
| D2  | **No state across rounds**                          | `extract_definitions` (`eval.rs:345-377`) calls only `define_variable` (which stores a **type string**), never `define_var` (which stores a `RuntimeValue`); `wrap_code` does not inject earlier definitions either. The `VariableInfo::Value` (`eval.rs:23`) branch is unreachable. `REPLContext.variables` serves only completion and does not participate in evaluation                                                            |
| D3  | **The wrapping syntax is suspected to be obsolete** | `eval.rs:314`/`316` produce `main() -> () = () => { code }` (the old-style `name() -> Ret` function declaration header), whereas the current canonical form is `main: () -> Void = { }` (`tests/yaoxiang/00-smoke/hello.yx:12`; the canonical table in RFC-007:94 consists entirely of `name: (a: Type) -> Ret`). **This was not tested by running the compiler; if the analysis is correct, every REPL evaluation fails to compile** |
| D4  | `:debug` is a stub                                  | `mod.rs:454-466` prints two lines and returns; breakpoints are stored (`mod.rs:319`), but **the interpreter never queries them**                                                                                                                                                                                                                                                                                                      |
| D5  | `:history` is a stub                                | `mod.rs:387` prints "not yet implemented"                                                                                                                                                                                                                                                                                                                                                                                             |
| D6  | Duplicates `lib.rs`                                 | `eval_code` at `lib.rs:85` and `repl/eval.rs:wrap_code` are **two independent implementations of “automatically wrap main,”** and their wrapping forms differ                                                                                                                                                                                                                                                                         |

## E. Local Cleanup Items

| #   | Location                                                                                                | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| E1  | `undefined/temp/33-calib/`                                                                              | An empty directory tree at the root. It is almost certain that some code concatenated the string `"undefined"` into a path. **Delete directly**                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| E2  | `src/lib.rs:568`, `:582`                                                                                | `dump_type_detail` / `dump_const_detail` each have a `_ => todo!()` panic path. `ConstValue::LibraryRef \| ExternRef` is FFI-related, and the panic path is predictable. Replace it with meaningful fallback output                                                                                                                                                                                                                                                                                                                                                                                          |
| E3  | `src/frontend/module/consistency.rs:71-135` vs `:142-199`                                               | `check_vendor_lock_consistency` and `check_workspace_consistency` are **approximately 60 lines of near-duplicate code**, differing only in dependency source and the two path-dependency skips. **Merge them**                                                                                                                                                                                                                                                                                                                                                                                               |
| E4  | `src/frontend/core/parser/statements/declarations.rs:82-117` / `:120-122` / `:145-167`                  | Approximately 120 lines for detecting and rejecting the old function syntax. The body of `skip_old_function_syntax` (`:120-122`) contains only one comment. **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                 |
| E5  | `src/frontend/core/parser/pratt/precedence.rs:35-94` + `:98-133`                                        | 96 lines of dead code (see B6). **Delete together with 05-frontend-paradigm.md**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| E6  | `docs/src/.vitepress/config.js:259-268`                                                                 | The “Tool Design” sidebar explicitly lists `check/diagnostic-system`, `check/cross-file-analysis`, and `check/incremental-checking`. **If the C1-C3 document deletions are performed, this location must be cleaned up in sync**, or the documentation site will contain 404 links                                                                                                                                                                                                                                                                                                                           |
| E7  | Duplicate `ConstValue`: `src/frontend/core/types/const_data.rs:14-30` × `src/middle/core/ir.rs:738-755` | Five variants have identical names (`Int` / `Bool` / `Float` / `LibraryRef` / `ExternRef`), but **`Float` has inconsistent widths (f32 × f64)**; there is no `From` / `TryFrom`, and handwritten conversions are scattered across call sites (`const_eval.rs` / `ir_gen.rs`). `kind()` at `const_data.rs:44` has a live `todo!()` for `LibraryRef` / `ExternRef`. **Disposition: merge into one definition + add explicit conversions + replace `todo!()` with fallback output; assigned to P10 (local cleanup category E)**. Registered at the P0 gate on 2026-10-06 (Criterion A, J=0.56)                  |
| E8  | Duplicate downstream mappings for `Literal` (Prohibition III A)                                         | At least three identical `Literal → MonoType` arms (`Literal::Int(_) => MonoType::Int(64)` and so on, in `inference/expressions.rs`, `checker.rs`, and `inference/patterns.rs`); 19 handwritten `Literal → ir::ConstValue` conversions in `ir_gen.rs` and 4 handwritten `Literal → const_data::ConstValue` conversions in `const_eval.rs`, with no shared conversion point. **Disposition: move the three typecheck-side conversions to the shared layer under P6; consolidate the `ir_gen` / `const_eval` side together with the E7 `ConstValue` merge under P10**. Registered at the P0 gate on 2026-10-06 |

### Data Loss: Two Cases, Not Three

The original `impl From<BytecodeFile>` (now `src/middle/bytecode/mod.rs`) had three hard-coded
discards. After the 7e review, there were two genuine loss cases and one false positive:

| Line (Old) | Discarded Content                | Review Result                                                                                                                                                                        |
| ---------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `2312`     | `upvalue_count: 0`               | **Genuine loss**: persisted as of v5 (7e.1)                                                                                                                                          |
| `2315`     | `exception_handlers: Vec::new()` | **False positive**: the field has no producer or consumer, and the language layer has no throw/try/catch (RFC-030/036) ⇒ dead field, not serialized; **deleted (6753bbe, P10 S6.1)** |
| `2341`     | `globals: Vec::new()`            | **Genuine loss**: persisted as of v5 (7e.3), with the carrier unified as `MonoType` (7e.4)                                                                                           |

Criterion: an empty table with no producer is not lost data; it is a dead path (the same criterion
as D32). The `.42` `VERSION` increase from 4 to 5 covered the two genuine loss cases in one step
(Decision D52). This is listed only to prevent it from being mistakenly cleaned up as “dead code.”

## F. Do Not Delete, but Mark or Decide

| #   | Subject                                                                                                                                                                                                                                                                                                                 | Disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/frontend/core/typecheck/checker/semantic_tokens.rs` (**already handled (P5 5.1)**: 1,547 lines at the time, now 1,553 lines and a real `mod`)                                                                                                                                                                      | **Not dead code.** At the time it was spliced into the end of `checker.rs` via `include!` (the repository’s only `include!`), and the file **was not declared as any module** (`grep 'mod semantic_tokens'` returns zero repository-wide matches). Its first line was directly `impl TypeChecker {`, with no `use` header of its own, and it was the only file under the `checker/` directory. Consequences: no module identity, loss of visibility isolation, broken rust-analyzer navigation and symbol search, and the toolchain treated it as part of `checker.rs`. At the time, the actual `checker` module was 5618+1547 ≈ 7165 lines (now `checker.rs` is 4167 lines + three `checker/` submodules of 1553/1107/470 lines). **Fix: change it to a real `mod`** (completed on 2026-10-26 by [09](09-execution-wbs.md) §P5 5.1; the B1 inventory for `check-boundary` is now zero), rather than deleting it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| F2  | wasm attributes (both positive and negative `#[cfg(... wasm32 ...)]` attributes on the same line are counted): in the current tree, **29** files under `src/` contain the literal `wasm32`, **22** have actual attributes, and **7** only mention it; `orchestrator.rs` has **10** attributes (4 positive + 6 negative) | **Not dead code.** The wasm target is carried by an **independent shim crate**: `wasm/Cargo.toml:10-11` has `crate-type = ["cdylib"]`, `:18` has `wasm-bindgen = "0.2"`, and `:17` depends on the main crate; `exclude = ["wasm", ...]` in the root `Cargo.toml:3` explicitly shows that it is an independent crate. Looking only at `crate-type = ["rlib"]` in the root `Cargo.toml:31` would incorrectly classify these branches as defensive dead code—**that inference does not hold**. The CI wasm build is in `.github/workflows/_build-wasm.yml:30` (`targets: wasm32-unknown-unknown`) + `:77` (`cd wasm && wasm-pack build --target web`), invoked by `dist-release.yml:271` / `docs-deploy.yml:25` / `nightly.yml:112`. **These branches are load-bearing**: they determine which main-crate APIs `wasm/src/lib.rs` (now **152 lines**) can call under the wasm target (`src/lib.rs:151` / `:170` / `:187` gate out `run_file` / `run_project` / `build_bytecode` as a whole). **The cleanup scope has been determined**: four wasm arms are runtime-unreachable in the playground scenario but load-bearing at compile time (their call sites are not gated); six `not(wasm32)` arms are load-bearing across the three native stage tables ⇒ **there are no directly deletable low-risk items**—the item-by-item determinations and evidence are in “F2 Determination” below (Decision D35: delete unreachable items and retain reachable ones; assigned to P10) |
| F3  | Column structure of `docs/src/rfc/TRACKING.md`                                                                                                                                                                                                                                                                          | It has only a “Status” column (document status), **no “Implementation Status” column**—meaning it **cannot represent** the error where an RFC is marked complete but the code is absent. Only 5 of the repository’s 52 RFCs have an `impl_status` field (`027` in-progress, `017`/`014`/`014b` complete, `review/027a` partial). **All 26 accepted RFCs omit implementation status**, including 009, 009a, 013, 018, and 038                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| F4  | `src/frontend/core/typecheck/layers/README.md`                                                                                                                                                                                                                                                                          | **Already handled (4.4.1)**: the README now describes the actual call order of `checker.rs::check_module_impl` (refinement walkthrough: `predicate` → `termination` → `ownership` → type-constraint solving); the line numbers in parentheses are taken from the current tree and are anchored by function name/driver point; it no longer claims that “lower-layer failures prevent upper layers from running” (there is in fact no short-circuit); `equivalence` is described according to its actual identity as a “shared type-comparison utility + a proof entry point used only by tests.” This row serves only as a record of the completed work and no longer points to the old line ranges                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| F5  | `docs/src/rfc/index.md`                                                                                                                                                                                                                                                                                                 | `:79` places RFC-026a in the “Accepted RFCs” table, but its status column says “RFCs under review”; the “RFCs under review” subsection at `:48-49` lists only 032 and 027a, omitting 026a. This is inconsistent with the three review entries in `TRACKING.md`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F6  | `tools/cargo-dist/` (`dist.exe`, 21,108,736 bytes) and `benches/shootout/out/` (8 binaries)                                                                                                                                                                                                                             | **Neither is tracked by Git** (`git ls-files` returns empty), and both are excluded by `.gitignore`. **Not a problem**; recorded only to prevent later audits from misclassifying them as “committed binaries”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F7  | `tools/code-tables/src/lib.rs:66-78`                                                                                                                                                                                                                                                                                    | **Implemented (a2e570c, P10 S6.2)**. `extract_code_from_entry_line` is a **line-prefix matcher** (`trim_start().strip_prefix("(\"")` takes the next quote), and depends on the `("E1001", ...)` tuple syntax; it is **not a parser**. Its architecture (`parse` + `validate` + a build.rs gate + `--fix` remediation) is worth reusing for the opcode table, but **the extractor must be rewritten for the opcode form (approximately 30 lines)**. This belongs to follow-up work in 06-cleanup-inventory.md or a separate RFC                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| F8  | Gate gap in `src/backends/common/opcode.rs`                                                                                                                                                                                                                                                                             | **Implemented (a2e570c, P10 S6.2)**. 80 constants, 0 duplicates, range `0x00..0xE2` (span 227, **144 gaps**). The same opcode facts were once expressed in **5 places** (consolidated into a single `OPCODE_INFO` table on 2026-11-28): `BytecodeInstr::opcode()`(`bytecode.rs:558-646`)/ `size()`(`649-803`)/ `opcode::opcode_name()`(`opcode.rs:120-206`)/ decoding match(`bytecode.rs:978-2302`)/ 48 `translate_*`(`translator.rs:676-1577`). **The compiler enforces only two of them** (the dispatch table at `backends/.../executor/debug.rs:194` + the exhaustive matches in each `ops/*.rs` family); missing one of `opcode()` / `size()` / decoding arms / encoders fails only when running `.42` artifacts. The comments in the `size()` table (`649-803`) contradict the actual encoding (`bytecode.rs:2181-2182`), and there was **no systematic differential test** (80 fully covering differential tests have now been added). Operator semantics also use a third parallel enumeration here (`BinaryOp`/`UnaryOp`/`CompareOp`, `bytecode.rs:70/97/106`, with another naming set for `Rem`/`Xor`/`Sar`); consolidate these together with the opcode-generation gate. The target location for the vocabulary is `middle/bytecode/opcode.rs` (migrated with the bytecode-domain merge to eliminate the L3→L4 reverse dependency; see 01)                                                                                                                        |

### F2 Determination: Reachability of wasm Attributes in `orchestrator.rs` (determined 2026-10-18; no cleanup included)

**Evaluation surface** = the wasm production build (the shim crate built by `wasm-pack`; see the F2
row); **evaluated revision**: `b5e68b3c9`. This section records only the determination and does not
create any cleanup commitment (cleanup is assigned to P10 / D35).

#### I. Counting Convention (Recounted from the Current Tree)

```console
$ grep -c '#\[cfg(target_arch = "wasm32")\]' src/frontend/module/orchestrator.rs
4
$ grep -c '#\[cfg(not(target_arch = "wasm32"))\]' src/frontend/module/orchestrator.rs
6
```

- The current `orchestrator.rs` has **10 wasm attributes** in total = 4 positive
  (`target_arch = "wasm32"`) + 6 negative (`not(...)`).
- The convention = **the text of the `#[cfg(...)]` attribute on the same line**, counting both
  positive and negative forms (this is also the “12” convention used by 02); runtime `cfg!(...)`
  macros are not counted—a repository-wide `git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs'`
  returns zero matches.
- The difference between 12 and 10 is **not an error**; it is **a fact of an older revision**: at
  `544695ea9` (when the RFC-039 design document was first committed), there were 5 positive + 7
  negative = 12:

```console
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(target_arch = "wasm32")\]'
5
$ git show 544695ea9:src/frontend/module/orchestrator.rs | grep -c '#\[cfg(not(target_arch = "wasm32"))\]'
7
```

- The difference comes from **a complete pair of attributes being moved**: `6ec330d22` (WBS 4.2.2,
  which slimmed `check_project` into a `Program` constructor and deleted a net 252 lines from
  `orchestrator.rs`) moved `role_context`, together with its `not(wasm32)` implementation and
  `wasm32` stub, to `src/driver/helpers.rs:360/394`—reducing `orchestrator.rs` from 12 to 10 without
  changing the mechanism itself.
- Revision-by-revision review (actually executed for this article; excerpts below; `6ec330d22` is
  the old/new boundary):

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

#### II. Three-Question Criterion (Splitting It Prevents Load-Bearing Arms from Being Misclassified as Dead Code)

1. **Compile-time existence**—does this arm participate in compilation under the target in question?
2. **Runtime reachability**—under that target, is there a call chain from the playground entry
   point?
3. **Deletion cost**—would deleting it break compilation for that target (are the call sites
   likewise gated)?

#### III. Starting Point (Playground Entry Point and Stage Table)

- Entry point: `wasm/src/lib.rs` (now 152 lines), `run_code:122` / `test_compile:71` →
  `compile_playground:84` → `Driver::run(Program { kind: WasmPlayground })`; `test_compiler:26` →
  `Compiler::new()` (`src/frontend/compiler.rs:45`, which constructs only `Pipeline` and does not
  touch the orchestrator); `test_smt:36` → `default_solver()`.
- **`WasmPlayground` uses `SINGLE_FILE_STAGES`** (`src/driver/program.rs:51-58`;
  `SingleFile | WasmPlayground` appear together in the table at `:135`): Parsing / Typecheck /
  DeadCodeAnalysis / ProofExecution / IrGeneration / Monomorphization—**excluding**
  VendorConsistency / Discovery / Registry / RoleClassification / GlobalSlotAlloc / Linking.

#### IV. Item-by-Item Determination (10 Attributes in the Current Tree)

| #   | Location (`orchestrator.rs`) | Gated Subject                                                       | Compiled in wasm Build              | Reachable at Playground Runtime                                                                                                                                                                    | Deletion Cost (Are Call Sites Gated?)                                                             |
| --- | ---------------------------- | ------------------------------------------------------------------- | ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| 1   | `:271` positive              | wasm arm of `ensure_vendor_consistency` (`:267`) (`Ok(None)`)       | Yes                                 | **No**: its sole call site, `src/driver/arms.rs:1195` (the VendorConsistency arm), is not in the playground stage table                                                                            | High: `arms.rs:1195` is not gated; deleting the arm alone ⇒ wasm compilation fails                |
| 2   | `:418` positive              | wasm arm of `discover_with_used` (`:400`) (`lock_versions` emptied) | Yes                                 | **No**: call sites are `arms.rs:1221` (the Discovery arm) and `src/driver/helpers.rs:686` (the Check-form reference pool `collect_package_refs`, called only by the Check branch at `arms.rs:521`) | High: same as above                                                                               |
| 3   | `:655` positive              | wasm stub for `member_context` (`:656-659`)                         | Yes                                 | **No**: sole call site is `:404` (inside the function from #2)                                                                                                                                     | High: `:404` is not gated                                                                         |
| 4   | `:753` positive              | wasm stub for `dep_import_surface` (`:754-756`)                     | Yes                                 | **No**: call sites are `:671` (in `resolve_in_package_root`) and `:813` (in `resolve_in_vendor_with_lock`), rooted in `resolve_module_path` at `:468` (inside the BFS in `discover_with_used`)     | High: same as above                                                                               |
| 5   | `:41` negative               | `use crate::package::manifest::PackageManifest;`                    | No (not included in the wasm build) | Not applicable                                                                                                                                                                                     | Load-bearing on native: used by `:678` / `:747`                                                   |
| 6   | `:276` negative              | native arm of `ensure_vendor_consistency`                           | No                                  | Not applicable                                                                                                                                                                                     | Load-bearing on native: VendorConsistency appears in three stage tables at `program.rs:61/98/113` |
| 7   | `:413` negative              | native arm of `discover_with_used` (lock reading)                   | No                                  | Not applicable                                                                                                                                                                                     | Load-bearing on native: Discovery appears at `program.rs:62/99/114`                               |
| 8   | `:607` negative              | native `member_context` (workspace resolution)                      | No                                  | Not applicable                                                                                                                                                                                     | Load-bearing on native: call site is `:404`                                                       |
| 9   | `:677` negative              | manifest-name reading in `resolve_in_package_root`                  | No                                  | Not applicable                                                                                                                                                                                     | Load-bearing on native: `:544/:547` ← `:468`                                                      |
| 10  | `:745` negative              | native `dep_import_surface`                                         | No                                  | Not applicable                                                                                                                                                                                     | Load-bearing on native: `:671` / `:813`                                                           |

#### V. Conclusions

1. **The four wasm arms are runtime-unreachable in the playground scenario** (#1-#4; evidence = the
   stage table excludes the stages containing their sole call sites), but they are **load-bearing at
   compile time**. Literally executing D35’s “delete unreachable items” would fail to compile—the
   feasible form is to “gate the entire project-mode subtree under wasm32 (`orchestrator.rs`’s
   discovery/resolution functions + the driver’s VendorConsistency/Discovery arms + the
   reference-pool collection in `helpers.rs`).” Cost and benefit are assigned to P10.
2. **The six `not(wasm32)` arms do not exist in the playground build** (excluded at compile time),
   and all are load-bearing on native (the MultiFile / Check / Lsp stage tables). “Unreachable in
   the playground ⇒ delete” **does not apply**: deleting by that criterion would directly break the
   CLI/LSP.
3. ⇒ This directory has **no “directly deletable and low-risk” items**; F2’s only potential cleanup
   benefit is “whether the wasm artifact becomes smaller,” and that has not been verified (see
   below).

#### VI. Unverified Items (Must Not Be Treated as Conclusions)

- **The size benefit for the wasm artifact has not been measured**: `wasm-pack build` and
  size/symbol-table tools (`twiggy` / `wasm-objdump`) were not run, so there is no data on “how much
  smaller the artifact becomes after gating the project-mode subtree”; whether the linker removes
  unused `pub(crate)` items has also not been verified.
- **wasm32 consumers outside the shim have not been exhaustively identified**: this round checked
  only in-repository CI (the only wasm32 compilation is at `_build-wasm.yml:30/77`, through the shim
  crate). If a third party compiles the main crate directly for `wasm32-unknown-unknown` and calls
  project-mode APIs, the reachable surface may exceed this determination.
- **“Load-bearing” on native means a static call chain + stage table**; each arm was not verified
  end-to-end by running MultiFile/Check/Lsp probes that explicitly touch every arm.

#### VII. Appendix: Repository-Level Count Verification (Same Revision)

```console
$ git grep -l wasm32 b5e68b3c9 -- '*.rs' | wc -l
30        # includes the wasm/ shim crate
$ git grep -l wasm32 b5e68b3c9 -- 'src/*.rs' | wc -l
29
$ git grep -lE '#\[cfg\([^]]*wasm32' b5e68b3c9 -- 'src/*.rs' | wc -l
22        # has actual cfg attributes
$ git grep -rn 'cfg!(target_arch' b5e68b3c9 -- '*.rs' | wc -l
0         # no runtime cfg! convention
```

The “wasm status” in `02` and the F2 row use the same convention: the current tree has **29** files
containing the literal `wasm32` / **22** with actual `#[cfg(...)]` attributes / **7** mentioned only
in documentation; `orchestrator.rs` has **16 occurrences** = **10 attributes** (4 positive + 6
negative) + 6 comments (the seven files that merely mention it: `src/driver/arms.rs`,
`src/frontend/core/typecheck/proof/smt/backend.rs`, `src/frontend/core/typecheck/proof/smt/mod.rs`,
`src/frontend/core/typecheck/proof/smt/tests/feature_gate.rs`, `src/frontend/module/roles.rs`,
`src/tests/mod.rs`, `src/util/wasm_z3_shims.rs`).

## G. Gate and Criterion Findings Registry (Continuously Updated from 2026-10-06 as Each Phase Is Implemented)

`scripts/ci/check-concepts.py` ([09](09-execution-wbs.md) §P0 0.2.1, Prohibitions I A/B/C/D) was run
for the first time on 2026-10-06. In accordance with the division of responsibility—“the gate
reports only suspects; humans adjudicate” ([08](08-maintenance-mechanism.md) §Known
Limitations)—this section records the adjudication results and disposition ownership. Unexpanded
matches (Criterion A’s 25 violation-layer suspects, and all Criterion B suspects) will be reviewed
pair by pair as the corresponding phases are implemented.

| #   | Subject                                                                                                          | Adjudication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Disposition Ownership                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | `ValueType` × `RuntimeValue` (`backends/common/value.rs:86` × `:236`, J=0.95)                                    | **Legitimate structure, not parallel representations**: a value/type-tag pair, maintained at a 1:1 ratio by the exhaustive `value_type()` (`value.rs:326-369`), where a missing arm causes a compilation failure. However, the only substantive external consumption of `ValueType` is `{:?}` formatting on std/os/fs/net error paths and the constant assignment of `Void` at `executor.rs:410`; its 21 variants far exceed actual requirements                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | P10 S5 slimming candidate: lower it to `pub(crate)` and evaluate trimming it; during S5 implementation, also adjudicate whether the `AsyncValue.value_type` field (`value.rs:217`) is necessary                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| G2  | Duplicate `ConstValue` (Criterion A, J=0.56)                                                                     | **True positive**, including inconsistent f32/f64 widths and a live `todo!()` in `kind()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Recorded as E7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| G3  | Type-name string dispatch, 11 locations (true positives under Criterion D)                                       | **True positive, all from one root cause**: there is no single authoritative recognition point for built-in container names. List: `"List" \| "Vec" \| "Array"` ×4 (`inference/existential.rs:120`, `inference/expressions.rs:2921`, `ir_gen.rs:4826`, `ir_gen.rs:4652`), `"String" \| "Bytes"` ×4 (`inference/expressions.rs:1738`, `eval/normalizer.rs:230`, `trait_data.rs:180`, `ir_gen.rs:62`), the size table at `eval/const_eval.rs:498-511` (including a dead `"Uint"` arm), `to_type_id` at `middle/passes/codegen/bytecode.rs:744-756` (`"Set" => 25` contradicts Decision 4 concerning #300), and `"Clone" \| "Dup"` at `trait_data.rs:187`                                                                                                                                                                                                                                                                                                                                                                         | **P6 extended verification item**: these points are on the typecheck / eval / middle side and are outside the parser-dataflow coverage in 03 §5.4; during P6 implementation, handle them under the same principle as 03 §5.3 (`NameKind` is determined once; match arms no longer perform string comparisons). The duplicate table has already been corrected to seven locations (03 §2.3)                                                                                                                                                                                                                                                      |
| G4  | Four Criterion D false positives                                                                                 | **Legitimate lookup / dispatch tables, not cases for consolidation**: tool-installation guidance at `package/build/requirements.rs:17-25`, build-result display names at `package/vendor/mod.rs:134-139`, REPL command aliases at `repl/mod.rs:241-396` (8 arms; user-interface aliases are legitimate UX), and `parse_runtime_mode` at `util/diagnostic/mod.rs:462-468` (a single boundary point for documented historical aliases)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | When P9 becomes a hard gate, add a `// reason:` exemption to each location; the number of exemptions will be reviewed along with the gate report                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| G5  | `ReleasePlan.drops` span-key mismatch (Criterion test under P2 2.4.4, not a P0 match)                            | **True positive, a contract defect—D20’s prediction empirically confirmed**: ownership records keys by the span of the **last use of an expression** (`ownership.rs:2574 build_release_plan`), while `ir_gen.rs:1956` looks them up by the span of the **statement**; expression-level keys therefore never match—a full-corpus test found **168 files / 373 keys** unconsumed, and Drops were silently lost (case-by-case evidence is in the file header of `src/middle/core/tests/release_plan_spans.rs`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | **P4/D20** (Span key → PlanId; the behavior surface spans approximately 168 files and must wait for the C2 criterion and corpus differential to be in place); the `test_release_plan_spans_consumed` criterion remains permanently red (`#[ignore]`) and turns green when D20 is implemented                                                                                                                                                                                                                                                                                                                                                    |
| G6  | Three IR metadata defects (Criterion `verify_loose` corpus test run during P2 2.1.2, D38’s prediction fulfilled) | **Three true positives, all fixed with 2.1** (the corpus differential baseline compared against itself and showed zero differences, proving behavioral equivalence): ① the locals table for method IR was truncated to the parameter count (`ir_gen.rs` originally used `take_cur_locals(param_types.len())`), underestimating `local_count` and causing the runtime to silently tolerate the overrun through `Frame::set_slot` resizing (`frames.rs:103-105`); ② unannotated parameters in lambdas / curried intermediate layers / function paths were discarded by `filter_map` (three locations), causing the params length and frame layout to become misaligned—filled with `Int(64)` placeholders to align with the existing method-path convention; ③ the match result slot depended on implicit Void pre-initialization of the frame (the fallback location for a failed final-arm guard was undefined on the CFG on paths without a definition)—`generate_match_expr_ir` now explicitly pre-initializes Void on entry | **Fixed** (same batch as 2.1). **The evaluation surface is narrowed and recorded** (the `verify.rs` file header is authoritative): temp-slot dominance is not checked (Void pre-initialization is frame semantics and intentionally relied upon by `ir_gen`); the trusted source for type consistency is narrowed to parameter-slot signatures + constants (`ir_gen` never writes `LocalSlot.ty`, and the entire table uses the `Int(64)` placeholder—the complete evaluation surface is assigned to P7 value table 04 §262); `init_locals` is the #368 debugging view table, not an allocation limit, and `init` does not check Local overruns |

## H. Inventory of Giant Functions (Pre-Release Audit Convention, Effective 2026-10-16)

> Convention (established by `/pre-release-audit` on 2026-10-16): a function span > 150 lines is an
> **initial screening threshold**, not a command to “report every one” individually—a full list
> would turn the inventory into a pile of line counts and drown the new signal in existing
> inventory. Every report entry must contain three numbers: **host line count / lines inserted in
> this change / line count before insertion**. Divide entries into two categories: **A. Existing
> giant functions with inserted branches** (>150 lines and >50 lines inserted in this change); **B.
> Newly added large functions in this change** (almost entirely new and >100 lines). The line
> numbers below are taken from audit revision `1935aab53` (the `v0.8.4..1935aab53` range).

### A. Existing Giant Functions with Inserted Branches

| Function                | Location                                                    | Host Lines | Inserted in This Change | Before Insertion |
| ----------------------- | ----------------------------------------------------------- | ---------- | ----------------------- | ---------------- |
| `infer_call_expr`       | `src/frontend/core/typecheck/inference/expressions.rs:4045` | 1594       | 358                     | 1236             |
| `generate_call_expr_ir` | `src/middle/core/ir_gen.rs:7694`                            | 813        | 171                     | 642              |

### B. Newly Added Large Functions

| Function                     | Location                                            | Lines | Newly Added |
| ---------------------------- | --------------------------------------------------- | ----- | ----------- |
| `push_joined` (local helper) | `src/frontend/core/types/mono.rs:603`               | 180   | 177         |
| `consumer_stage_skipped`     | `src/driver/obligations.rs:650`                     | 122   | 122         |
| `push_children`              | `src/middle/passes/mono/instance.rs:191`            | 159   | 130         |
| `local_count_of`             | `src/backends/interpreter/executor/executor.rs:321` | 130   | 107         |

**Existing giant functions not included in the inventory** (≤4 lines inserted in this change;
recorded but not handled): `infer_expr` (`expressions.rs:2999`, 955 lines),
`collect_function_signature` (`checker.rs:1738`, 669 lines), `check_module_impl` (`checker.rs:1153`,
441 lines), `generate_local_stmt_ir` (`ir_gen.rs:3265`, 368 lines), and others.

### Disposition Approach

- **Category A**: resolve them together with the subsequent decomposition tasks for
  `infer_call_expr` / `generate_call_expr_ir`—these are the repository’s heaviest hosts, and the
  current named-argument refactoring inserted 358 lines into the former.
- **Category B**: `push_joined` is a **local** helper for `Task` work-list-style type formatting
  (its host is the actual new 180-line function), and its responsibilities differ from
  `increment_level_string` at `mono.rs:123` (174 lines, which rewrites level names); it is **not a
  duplicate and must not be merged**. `consumer_stage_skipped` and `local_count_of` can be
  decomposed independently.

## Key Decisions and Rationale

The authorization is “permission for outright deletion,” but this article recommends **different
dispositions** for several items:

| Category                                                               | Disposition                                     | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pure placeholders** (skeletons with no assertions or implementation) | **Delete**                                      | B1 (415 lines), B3 (the dispatch chain), B6 (96 lines), B7 (the empty function), C6 (14 lines), E1 (the empty directory)                                                                                                                                                                                                                                                                                                                                                               |
| **Paper-only design documents**                                        | **Delete or rewrite**                           | After deleting C1 and C2, rewrite them to point to the actual implementation; rewrite C3’s boundary table; **`config.js:259-268` must be cleaned up in sync** (E6)                                                                                                                                                                                                                                                                                                                     |
| **RFCs whose status does not match the implementation**                | **Change the status**                           | Move C4 (RFC-018) back to `draft` or add `impl_status: 'not-started'`; add `impl_status: 'not-started'` to C5 (RFC-028). **Do not delete them**—the design itself may be valuable; the problem is that its status is inaccurate                                                                                                                                                                                                                                                        |
| **Orphan tests with real assertions**                                  | **Revive, do not delete**                       | Total: **1,005 lines / 78 tests**: lexer 629 lines / 55 tests + 311 lines / 16 tests hidden in live directories (`precedence_inline` / `overload_inline` / `json`) + 65 lines / 7 tests under `package/template/tests/`. **The cost of revival is one `mod` declaration, and the benefit is restoring this coverage to CI**. Deleting them is a net loss                                                                                                                               |
| **Zero-call but semantically important `pub` items**                   | **Lower visibility rather than delete**         | B2 (`TypeSystem`), B4 (`check_type_equivalence`)—move them to `pub(crate)` so the compiler can identify them, or add an implementation. **`is_subtype` in B4 must be retained**                                                                                                                                                                                                                                                                                                        |
| **Opcodes / instructions that cannot produce a value**                 | **Add construction sites or remove the opcode** | B8 (`TailCall`), B9 (`Switch`, whose opcode round-trip test is explicitly registered through `WHITELIST=["SWITCH"]`). B10 was fixed under D33 (2026-11-14: `opcode()` distinguishes the `op` field + dedicated code `I64_NOT`; see the commit message for task 3.20.29) and was removed from this todo list. The remaining items require one of three choices and cannot remain as they are; D32/D33/D34 have already been adjudicated and will be handled with the `.42` version bump |
| **Data-loss defects**                                                  | ~~Separate issue~~ **P7 batch e (D52)**         | Two genuine loss cases in Section E + one false positive (`exception_handlers` is a dead field, **deleted (6753bbe, P10 S6.1)**)                                                                                                                                                                                                                                                                                                                                                       |
| **Valid code without module identity**                                 | **Transform, do not delete**                    | F1 (`semantic_tokens.rs`): change `include!` to a real `mod` (see `02`)                                                                                                                                                                                                                                                                                                                                                                                                                |
| **Items requiring a decision**                                         | **List as open questions**                      | F2’s wasm32 cleanup is recorded as [09](09-execution-wbs.md) §P10 **S7** (gate the project-mode subtree under wasm32; user decision B on 2026-10-18; includes the three unverified items verbatim); correcting the layer order will expose new diagnostics                                                                                                                                                                                                                             |

### Directions Not Adopted

- **Treat everything as “outright deletion,” including orphan tests**—the 1,005 lines of tests with
  real assertions are an asset. The deletion cost is one `mod` declaration, while the benefit is
  restoring coverage to CI. The authorization means “may be deleted,” not “should be deleted.”
- **Only add gates without changing code**—gates can prevent regressions, but B1’s 415 lines, the
  dispatch chain, and the 96 lines of dead ladder code **already exist**; gating them would merely
  freeze them in the baseline.
- **Keep RFC-018 / RFC-028 at `accepted` and only supplement the documentation**—`rfc/index.md:131`
  defines “accepted” as “entered the implementation phase,” so readers will reasonably assume that
  an implementation exists. `TRACKING.md` already provides precedent for the `impl_status` field (5
  RFCs use it), and the cost of adding the field is far lower than the cost of inaccurate status.

## Implementation Essentials

**This article does not require an equivalence criterion** (it belongs to the
[C6 pure-deletion category](07-equivalence-oracle.md)), but **each item must still be confirmed to
have no references**. In RFC-039’s global phase sequence, S1 corresponds to P1, and S2-S6 correspond
to P10.

| Phase | Content                                                                                                                                                  | Acceptance Criteria                                                                                                                                                                                                                                                          |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1    | Revive orphan tests with assertions (**1,005 lines / 78 tests**)                                                                                         | The test counts for the lexer / pratt / passes / emitter / template modules increase under `cargo test`; **real defects are expected to be exposed** (`literals.rs` overflow paths and invalid `\x`/`\u` escapes have never been tested), so time must be reserved for fixes |
| S2    | Delete pure-placeholder dead code (B1/B3/B6/B7 with the 05 cleanup/C6/E1)                                                                                | `cargo test` passes; `cargo clippy -D warnings` reports no new warnings                                                                                                                                                                                                      |
| S3    | Handle paper-only design documents (C1-C3 + synchronized cleanup of `config.js` under E6)                                                                | The documentation site builds without 404s; `scripts/ci/check-docs-truth.py` passes                                                                                                                                                                                          |
| S4    | Correct RFC statuses (C4/C5/F3—add an “Implementation Status” column to `TRACKING.md`)                                                                   | `python scripts/rfc/check_tracking.py` passes (`TRACKING.md` is regenerated automatically; **do not edit it manually**)                                                                                                                                                      |
| S5    | Lower the visibility of `pub` items (B2/B4)                                                                                                              | The compiler can identify genuine dead code                                                                                                                                                                                                                                  |
| S6    | Implement the opcode decisions for items that cannot produce values (B8/B9/F8 pending, following D32/D33/D34; **B10 is complete**)                       | Remove the dead paths for `Switch` / `TailCall`; implement the F8 gate; B10 is complete (2026-11-14, WBS 3.20.29: `opcode()` distinguishes the `op` field + dedicated code `I64_NOT`; see the commit message for task 3.20.29)                                               |
| S7    | Transform `include!` (F1; implementation steps are assigned to [09](09-execution-wbs.md) §P5 5.1) + correct the layer order (F4, assigned to P4’s 4.4.1) | See `02-stage-contract.md`                                                                                                                                                                                                                                                   |

**S1 must be performed first, and time must be reserved for defect fixes.** Its value is not
“cleanup,” but **restoring 1,005 lines / 78 previously unrun tests to CI**—before this, every
assessment of test coverage is based on false numbers.

## Known Limitations and Risks

- **S1 will expose real defects**. The four radix scanners in `literals.rs` have four isomorphic
  “continue consuming after overflow without reporting an error” branches, the entire
  `scan_leading_dot` function, and the invalid `\x`/`\u` escape paths have never been tested.
  Reviving the tests will probably require code fixes, delaying the subsequent refactoring.
- **B9/B10 involve language-behavior decisions**. Whether to “add construction sites” or “remove the
  opcode” for `Switch` and `UnaryOp::Not` requires considering whether they are part of the public
  surface on which released corpora depend.
- **After C1-C3 are deleted, cross-file-analysis capability documentation will be temporarily
  absent**, until `check/cross-file-analysis.md` is rewritten to point to the actual architecture
  under `src/frontend/module/`.
- **Reachability analysis can only find “unreachable” code, not “reachable but meaningless” code**.
  Judgments beyond Form 2 must be made item by item, and **this inventory is not an exhaustive
  proof**.
- **`pub` items do not trigger `dead_code` warnings in Rust**, so zero-call `pub` types (B2, B4)
  cannot be detected automatically by the compiler and must be found through searches.
- **Deleting the half-finished RFC-023 residue** (B1) fulfills the decision that “closures have been
  removed from the language”; **correcting inaccurate status** (C4/C5/F3) restores `TRACKING.md`’s
  reference value.

> **All open questions originally listed in this section have been adjudicated.** The item-by-item
> decisions are recorded in the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **No
> pending items remain in this article.**

## See Also

- [RFC-023 Closure Capture Model (Deprecated)](../../rfc/deprecated/023-closure-capture-model.md) —
  source decision for the B1 residue
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — unwired runtime check insertion from `§4.1` in B3
- [RFC-018 LLVM AOT](../../rfc/accepted/018-llvm-aot-compiler.md) — C4
- [RFC-028 JIT Compiler (Draft)](../../rfc/draft/028-jit-compiler.md) — C5
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../../rfc/draft/029a-module-cache-incremental.md)
  — formal home of C1/C6
- [RFC-039 Compiler Feature Routing Catalog Design](../../rfc/accepted/039-compiler-architecture.md)
  — overarching design
- [Refactoring Equivalence Criteria](07-equivalence-oracle.md) — criterion requirements for category
  C6
- `src/frontend/core/typecheck/layers/README.md` — actual proof-layer call order and the true
  identity of `equivalence` (F4 handled)
- `build.rs:19-55` — example of a generation-time gate in this repository
