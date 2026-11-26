# Frontend Paradigms: Lexing and Parsing

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> The four-layer model, acceptance-criteria tiers, and execution-phase order are defined in the main
> body of RFC-039; the role of each companion document is described in the
> [index for this directory](index.md).

## Positioning and Scope

This document covers the L2 frontend—**lexing (`src/frontend/core/lexer/`) and parsing
(`src/frontend/core/parser/`)**.

**Problem to solve**: reduce the change surface for “adding a binary operator” from the current
**6–7 locations within L2 + 17 downstream production files**—of which the compiler enforces
approximately 8–9 locations and the rest require manual synchronization—to **1–2 declaration
locations + 7–8 automatically generated locations**.

This document implements the pain point identified in the first “binary operator” row of RFC-039’s
functional routing table A. The restructuring category is **C5 frontend paradigm change** as defined
in [07-equivalence-oracle.md](07-equivalence-oracle.md) (criteria: identical AST snapshots +
identical diagnostics + identical behavior, all three required) and **C6 pure deletion** (no
equivalence criteria required; only confirmation that there are no references). The C6 item is a
prerequisite for C5: delete 96 lines of dead code, eliminate three raw magic numbers, and revive 629
lines of lexer tests that have never run.

**Scope boundary**: handling `ast::Type` (26 variants) and converging the three parallel type
representations belong to [03-type-unification.md](03-type-unification.md). This document does not
duplicate that work; it only declares the interface boundary in the “Detailed Design” section.

### Why This Work Is Worth Doing

“Adding an operator” is the most expensive change in the frontend. This tax has **three pathological
characteristics**:

**First, the cost does not decrease with experience.** Someone who has added an operator ten times
does not need to change one fewer location than someone who has added it once—because there is no
derivable relationship between these locations. The `(6,7)` in `led.rs:40` and the `12` in
`nud.rs:1041` have no connection, yet they must agree with each other or expression parsing will
fail.

**Second, missed changes fall into two categories, and the compiler cannot catch the worst
category.** See the tiered table in “Change Surface of Adding an Operator”: a missing arm in an
exhaustive `match` is caught by the compiler, whereas **a missing update across enums can never be
caught by the compiler**.

**Third, this tax is invisible on the books.** Nearly every commit in
`git log --oneline -- src/frontend/core/parser/pratt/led.rs` touches three or four files
simultaneously, but no tool can identify which location a given commit omitted.

### The Correct Paradigm Already Present in This Repository

The project has two **correct solutions to the same class of problem**, both of which can be reused
directly:

- **Error codes**: `build.rs:19-55` uses `tools/code-tables` to parse the registry **at build
  time**, validate uniqueness, and compare it entry by entry with RFC-013’s Markdown code table;
  **any discrepancy directly triggers `panic!` and rejects the build**. The cost of adding an error
  code is therefore a constant one location plus documentation.
- **Standard-library interfaces**: `gen_interfaces.rs` performs byte-for-byte comparisons, while
  `gen_docs.rs` detects drift in marked ranges (the final row of RFC-039 routing table B).

The common feature of these mechanisms is that they **turn “changes that must be synchronized” from
something people must remember into executable assertions**. The entire goal of this document can be
reduced to the same statement: turn the correspondences among the operator table, lexical-rule
table, and AST variant table into executable assertions.

### Why “Deleting Dead Code Only” Is Insufficient

Dead code does exist (`precedence.rs` is 96 lines, `skip_old_function_syntax` is an empty function,
and there are 629 lines of orphaned tests), but deleting it does not reduce the number of locations
affected by “adding an operator” by even one. Dead code is **inventory debt**, while cross-file
change surface is **structural cost**. The two must be handled separately; see “Key Decisions and
Rationale” for their disposition.

## Current State

Everything below is a **verified fact**, with a file path and line number attached. The verification
method and item-by-item measured results appear in “Verification Record”.

### Lexical Layer: Copy-and-Paste Evidence in `literals.rs`

`src/frontend/core/lexer/literals.rs` contains **1,568 lines** in total. This size comes not from
complexity, but from three structural duplications.

**Duplication 1: the four radix scanners are almost identical line by line.**

| Scanner               | Line range            | Lines |
| --------------------- | --------------------- | ----- |
| `scan_hex_number`     | `literals.rs:77-161`  | 85    |
| `scan_octal_number`   | `literals.rs:164-247` | 84    |
| `scan_binary_number`  | `literals.rs:250-333` | 84    |
| `scan_decimal_number` | `literals.rs:336-516` | 181   |

The first three are each approximately 84 lines and have exactly the same structure: accumulate
digit → `has_digits` check → `overflow` flag → `checked_mul(base)` → `checked_add` → `try_into` →
report an error. **The only differences are the base constant and the digit-extraction expression.**
Across three copies of approximately 84 lines each, **roughly 250 lines are pure redundancy**
(`scan_decimal_number` requires additional handling for decimal points and exponents, so it cannot
be merged and is listed separately).

Evidence sample—the overflow path in `scan_hex_number` (`literals.rs:134-141`):

```rust
if overflow {
    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
        value,
        point(lexer),
    ));

    return Some(lexer.make_token(TokenKind::Error("Hex number too large".to_string())));
}
```

`scan_octal_number:220-227` and `scan_binary_number:306-313` are the same block with a different
base and message text.

**Duplication 2: escape decoding is implemented three times.**

| Implementation               | Location                       | Length                                       |
| ---------------------------- | ------------------------------ | -------------------------------------------- |
| `\\` branch of `scan_string` | `literals.rs:694-827`          | 134 lines                                    |
| `\\` branch of `scan_char`   | `literals.rs:1015-1141`        | 127 lines                                    |
| `push_fstring_escape`        | starting at `literals.rs:1374` | Already extracted into a standalone function |

The first two are near-verbatim copies of 127–134 lines (`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` → same
push, identical `\x` / `\u{...}` logic, identical `c =>` error fallback), meaning **roughly 250
lines have been written twice**.

Worth calling out separately: **the f-string version has already been extracted into a standalone
function** (`push_fstring_escape:1374`). This shows that **someone already recognized this
duplication and fixed it locally, but never went back to converge the other two locations**. This is
more noteworthy than “none of the three copies was recognized”: it proves that the problem is
solvable and that only a mandatory mechanism is missing.

**Duplication 3: two multiline-string scanners.** `scan_multi_line_string:869` and
`scan_fstring_multi_line:1497` are another pair of implementations of the same kind.

**Residual abnormal formatting.** The two locations `literals.rs:777-787` and `1098-1108` each
contain 11 lines in a form where **three consecutive blank lines are inserted between every field of
a struct literal**. This is a residue from automated rewriting. Other fields in the same function
(`1111-1115`) have only one blank line between them, indicating that formatting was interrupted.

> This residue is harmless in itself, but it is a **diagnostic signal**: the same code was rewritten
> twice by different tools, meaning that someone once made a local patch here without reconciling
> the surrounding context.

**Conclusion**: roughly 500 lines in `literals.rs` (250 lines of radix-scanner redundancy + 250
lines of duplicated escape handling) are **mechanical duplication**, not complexity.

**f-string interpolation triggers nested compilation.** This is **duplicate compilation at
runtime**, not duplicate implementation, so it cannot be solved by merging functions.

`src/frontend/core/parser/pratt/nud.rs:441-446`:

```rust
let tokens_result = crate::frontend::core::lexer::tokenize(expr_str_trimmed);
match tokens_result {
    Ok(tokens) => {
        let mut parser = crate::frontend::core::parser::ParserState::new(&tokens);
        if let Some(expr) =
            parser.parse_expression(crate::frontend::core::parser::pratt::BP_LOWEST)
```

**At runtime, the parser layer runs a complete `tokenize()` independently for every f-string
interpolation and creates a new `ParserState`.**

Consequences: an f-string containing N interpolations **triggers N+1 complete lexical analyses** (N
interpolations + 1 outer layer). At the same time, each interpolation is an **independent parsing
context**, so:

- Diagnostic spans inside an interpolation are relative to the interpolation text; remapping them to
  source locations requires additional work.
- An interpolation cannot access any outer parser state (there is no current requirement for this,
  but the structure forecloses future possibilities, such as referencing an outer implicit variable
  from within an interpolation).
- The positions at which lexical and syntax errors are produced are split across two stages.

This is a **performance and structural defect** that must be addressed; see “Target Design ·
Lexing”.

### Syntax Layer: Two BP Ladders, Raw Magic Numbers, and Handwritten Dispatch Tables

#### The Two Ladders Are Not “One Live and One Dead”; They Are Intertwined

`src/frontend/core/parser/pratt/precedence.rs` currently contains **two binding-power ladders**,
whose numeric ranges overlap extensively.

**The first ladder (`precedence.rs:6-17`)**: `BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` /
`BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` /
`BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`.

**The second ladder (`precedence.rs:22-31`)**: `BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` /
`BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`.

**Measured conclusion: both ladders survive in production code, and they are already intertwined
inside `infix_info`.**

- `infix_info` (`led.rs:30-88`) references **five constants from the first ladder**: `BP_ASSIGN`
  (`:33`), `BP_CALL` (`:75` / `:77` / `:79` / `:83`), and `BP_CAST` (`:81`);
- It references **eight constants from the second ladder**: `BP_OR` (`:42`), `BP_AND` (`:44`),
  `BP_EQ` (`:47`), `BP_SHIFT` (`:51` / `:54`), `BP_BIT` (`:58`), `BP_CMP` (`:62` / `:65`), `BP_ADD`
  (`:68`), and `BP_MUL` (`:72`);
- **The only dead member of the second ladder is `BP_RANGE`**—`..` uses the raw literal `(6, 7)` in
  `led.rs:40` rather than `BP_RANGE`; `BP_RANGE` appears in production code only in the comment at
  `led.rs:157-159`.

The survival status of the first ladder’s 12 constants, based on a repository-wide `grep`:

| Status              | Constant                                                                     | Production reference points                                                                                                                                                                                      |
| ------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Dead (6)**        | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | Zero production hits. They appear only in `precedence.rs` itself and the orphaned test `precedence_inline.rs`.                                                                                                   |
| Dead (comment only) | `BP_COMPARISON`                                                              | Zero production hits. Mentioned only in a comment at `declarations.rs:742`.                                                                                                                                      |
| **Live (6)**        | `BP_LOWEST`                                                                  | `parser/mod.rs:80`, nine locations in `statements/control_flow.rs`, six locations in `statements/declarations.rs`, `pratt/led.rs:229`/`:321`/`:327`/`:415`, `pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523`, etc. |
|                     | `BP_ASSIGN`                                                                  | `led.rs:33`, `led.rs:99`, `statements/declarations.rs:18` (import), `:743` (`BP_ASSIGN + 1`)                                                                                                                     |
|                     | `BP_UNARY`                                                                   | `nud.rs:32`, `:35`, `:83`, `:101`, `:223`, `:243`                                                                                                                                                                |
|                     | `BP_CALL`                                                                    | `led.rs:75`, `:77`, `:79`, `:83`                                                                                                                                                                                 |
|                     | `BP_CAST`                                                                    | `led.rs:81`                                                                                                                                                                                                      |
|                     | `BP_HIGHEST`                                                                 | Approximately 20 locations in `nud.rs:38-69` (one for each literal and keyword prefix)                                                                                                                           |

**In total: the six live constants have approximately 89 production references across eight files.**

> This corrects an easily reached but incorrect conclusion: “delete the first ladder” is **not** a
> 12-line deletion, but a rename and consolidation spanning eight production files and 89
> references. Only six constants in the first ladder truly have zero references, plus
> `BP_COMPARISON`, which survives only in a comment. The second ladder is also incomplete—`BP_RANGE`
> is dead.

**The truly zero-reference dead code consists of two types**: the `Precedence` enum
(`precedence.rs:35-94`, 60 lines) and the `PrecedenceContext` struct (`precedence.rs:98-133`, 36
lines), totaling **96 lines**. Verification found that neither type is used in production:
repository-wide grep hits for `PrecedenceContext` and `Precedence::`, outside `precedence.rs`
itself, all occur in `src/frontend/core/parser/pratt/tests/precedence_inline.rs`.

**One detail must be documented**: the only file referencing this dead code is **itself an orphan**.
`pratt/tests/mod.rs:4-6` declares only `mod led; mod nud; mod precedence;`, and **does not declare
`precedence_inline`**.

> In other words, the 96 lines of dead code only “appear to have test coverage” because they are
> tested by a 95-line, 6-test file that has never participated in compilation (see “Dead Code and
> Test Status”). **Deleting the dead code must also handle this orphaned test file**, or the code
> will no longer compile.

Additionally, `precedence.rs:33` contains a self-contradictory comment as evidence: _“Precedence
rules for the Pratt parser”_—this comment describes the zero-reference `Precedence` enum, whose
ladder differs from the ladder actually in effect. The enum and implementation have become
disconnected.

#### Numeric Collisions

The numeric ranges of the two ladders overlap almost digit for digit, so **cross-ladder collisions
occur in pairs**:

| Collision                | Value    | Usage point A (first ladder)   | Usage point B (second ladder)  |
| ------------------------ | -------- | ------------------------------ | ------------------------------ |
| Assignment vs logical OR | `1 == 1` | `BP_ASSIGN` (`led.rs:33`)      | `BP_OR` (`led.rs:42`)          |
| Cast vs multiplication   | `8 == 8` | `BP_CAST` (`led.rs:81`)        | `BP_MUL` (`led.rs:72`)         |
| Shift vs term            | `6 == 6` | `BP_TERM` (dead)               | `BP_SHIFT` (`led.rs:51`/`:54`) |
| Bitwise vs comparison    | `5 == 5` | `BP_COMPARISON` (comment only) | `BP_BIT` (`led.rs:58`)         |

**There is also one collision within the first ladder**: `BP_UNARY=8` and `BP_CAST=8`
(`precedence.rs:14` and `:15`)—**two different semantics in the same ladder share one value**, and
both are live (used respectively for prefix binding in `nud.rs` and cast in `led.rs:81`). The second
ladder also collides internally: `BP_RANGE=7` and `BP_ADD=7`.

**The system does not currently fail because of an unwritten convention**: almost every infix
operator uses `bp_right = bp_left + 1`, so the right operand accepts only operators with **strictly
higher** binding power—this actually produces **left associativity** (`a - b - c` → `(a-b)-c`).
Therefore, the only effective comparison, “compare `bp_left`”, does not fail merely because two
operators have equal binding powers. The `FatArrow` at `led.rs:85` uses `(11, 1)`, whose `bp_right`
is lower than its `bp_left`, and is the only explicit “right operand greedily consumes” formulation
(maximizing the lambda body to the right).

This is **accidentally correct**: it depends on the facts that the `bp_left` returned by
`infix_info` is unique and that `parse_expression_internal:100` compares only `bp_left`. If any
location is changed to use a genuine `bp_left == bp_right` test to determine associativity, the
conflicts will immediately become visible.

#### Three Raw Magic Numbers

| Magic number | Location                             | Problem                                                                                                                      |
| ------------ | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| `(6, 7)`     | `led.rs:40` `TokenKind::DotDot`      | Literal binding-power pair with no names; it also **does not use `BP_RANGE` from the same ladder**, leaving `BP_RANGE` dead. |
| `(11, 1)`    | `led.rs:85` `TokenKind::FatArrow`    | 11 **exceeds the upper bound of either ladder** (the first ladder’s `BP_HIGHEST=10`).                                        |
| `12`         | `nud.rs:1041` `parse_expression(12)` | **Greater than `BP_HIGHEST=10`**; used for RFC-010b pattern probing.                                                         |

The comment at `nud.rs:896-901` explicitly explains the provenance of this magic number: _“Lambda
binding power is 11, so we use 12 to stop before =>”_ and _“parse_expression(12) turns `ok` into a
bare Var and `(` into an unexpected token”_. In other words, **the 12 was found empirically rather
than derived**.

#### Dispatch Is a Handwritten `match`

**Dispatch mechanism**: everything is a handwritten `match` returning a function pointer, with **no
macros, tables, or generators**.

- `parse_prefix` (`nud.rs:14-19`) obtains the `prefix_info()` closure;
- `parse_expression_internal` (`pratt/mod.rs:79-113`) loops over the
  `(bp_left, bp_right, parser_fn)` tuple returned by `infix_info()`;
- There are two handwritten tables: `infix_info` (`led.rs:30-88`) and `prefix_info`
  (`nud.rs:26-75`).

#### `parse_assign_after_target`: A Misleadingly Named 507-Line General Dispatcher

`parse_assign_after_target` at `src/frontend/core/parser/statements/declarations.rs:136` extends to
`:642` (**507 lines**), in a file totaling 1,015 lines.

Despite its name, “assign after target” is **the general dispatcher for declaration forms**. It
contains eight independent responsibility sections: old-syntax detection / three-level nested
lookahead for annotation disambiguation / declaration legality / semantic side effects / type-body
parsing fallback / RFC-010 lambda signature parameter reconstruction / MetaType definitions /
ordinary initialization.

Its growth comes from three overlapping causes.

**Cause 1: approximately 120 lines of dead language features.**

`is_old_function_syntax` (`declarations.rs:82-117`), `skip_old_function_syntax`
(`declarations.rs:120-122`), and the supporting lookahead at `:145-167`.

The precise point is: **these two functions are not unreferenced**. They are called at
`declarations.rs:719` and `:725` from `parse_identifier_stmt`. The path is “detect `identifier(`
followed by type parameters and `->` → report ‘old syntax has been deprecated’ → skip → return
None”.

The accurate description is therefore: **this is a still-running gate that rejects removed syntax**,
costing approximately 120 lines to perform one balanced-parenthesis scan for every statement
beginning with `identifier(`.

Among these:

- The body of `skip_old_function_syntax` (`:120-122`) contains **only the single comment**
  `// 旧语法已移除，此函数不再需要`, making it a **pure empty function**.
- More notably, after calling it (`:725-726`), the function directly executes `return None` and
  **does not consume any tokens**. In other words, this “skip” **actually skips nothing**—the
  subsequent tokens remain in the stream and can only be handled by the caller’s error recovery.
  This is an interface whose name does not match its behavior.

**Cause 2: lookahead logic is not abstracted.** “Skip balanced parentheses” is **handwritten four
times** in the same file:

| Location                           | Line range                |
| ---------------------------------- | ------------------------- |
| Inside `is_old_function_syntax`    | `declarations.rs:96-105`  |
| Inside `parse_assign_after_target` | `declarations.rs:147-156` |
| Same                               | `declarations.rs:191-200` |
| Same                               | `declarations.rs:224-233` |

Each is approximately 12 lines with the same structure: `paren_depth = 1` →
`while paren_depth > 0 && !at_end()` → increment on `LParen`, decrement on `RParen` → `bump()`.
**None of the four copies shares any code.**

**Cause 3: type-layer logic has leaked into the parser.** See RFC-039 routing table C:
`declarations.rs:27-80` calls the type layer’s `is_type_param_annotation` / `name_used_as_type`;
`declarations.rs:498-509` hardcodes the `"Terminates"` string and calls
`typecheck::operator_interfaces::spec()`. **This belongs to 03-type-unification.md /
02-stage-contract.md.** This document does not duplicate its disposition and only marks the boundary
in the “Detailed Design” section.

#### Complete AST Enum Overview

`src/frontend/core/parser/ast.rs`:

| Enum       | Location         | Variant count          |
| ---------- | ---------------- | ---------------------- |
| `Expr`     | `ast.rs:16-163`  | 22 (including `Error`) |
| `BinOp`    | `ast.rs:191-213` | 20                     |
| `UnOp`     | `ast.rs:217-223` | 4                      |
| `StmtKind` | `ast.rs:234`     | 9 (including `Error`)  |
| `Type`     | `ast.rs:427-541` | 26                     |
| `Pattern`  | `ast.rs:794-814` | 8                      |

The existence of `Expr::Error` has been verified (it is handled during span extraction at
`pratt/mod.rs:72` and likewise at `ast.rs:1048`); `StmtKind::Error` is constructed at
`parser/mod.rs:50`. **These two error placeholders are cornerstones of the backward-compatible
design and must be retained**; see “Detailed Design · Backward Compatibility”.

### Change Surface of Adding an Operator

**The downstream change surface is much larger than expected.** `BinOp::` has **534 hits across 47
files** in the repository, including **17 production files**. More importantly, these files consume
**two unrelated `BinOp` enums**:

| Enum                | Definition                | Variant count                               | Production consumers (parentheses contain the number of `BinOp::` hits in that file)                                                                                                                                                                        |
| ------------------- | ------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ast::BinOp`        | `parser/ast.rs:191-213`   | 20                                          | `led.rs` (22), `nud.rs` (1), `checker.rs` (8), `inference/expressions.rs` (25), `inference/statements.rs` (5), `operator_interfaces.rs` (5), `ir_gen.rs` (23), `formatter/handlers/expr.rs` (25), `lsp/handlers/inlay_hint.rs` (4), `spawn/analysis.rs` (3) |
| `const_data::BinOp` | `types/const_data.rs:234` | Verified to exist (`Ne`, rather than `Neq`) | `const_data.rs` (36), `const_eval.rs` (64), `evaluator.rs` (13), `ownership.rs` (13), `termination.rs` (16), `proof/smt/translate.rs` (13), `proof/dep_graph.rs` (2)                                                                                        |

Verification confirmed that `src/backends/` has **zero hits** for either `BinOp` or `ast::`—the
interpreter consumes IR / bytecode and never sees the AST. All 17 production files are therefore on
the L2 → L3 path.

**There is no `From`/`TryFrom` between the two enums**—a repository-wide grep for
`impl From<...BinOp`, `-> BinOp`, and `BinOp as` finds no conversion implementation. The cost is
that **every file touching both must alias them at the import site**: `const_eval.rs:19`
(`BinOp as AstBinOp`), `ir_gen.rs:2317` (`use ast::BinOp as B`), and the `CEBinOp` / `ConstBinOp`
aliases in test files.

> **This is the most important finding in this document.** When `ast::BinOp` gains a variant, **the
> compiler cannot remind you to update `const_data::BinOp`**, because they are independent types
> that can each compile successfully on their own. If the update is missed, the constant-folding
> path does not understand the new operator, while the type-checking and IR-generation paths
> do—**the same operator behaves differently in two subsystems, with no diagnostic whatsoever**.
>
> This is an order of magnitude more serious than “missing one `match` arm.” The latter is caught by
> an exhaustive match; the former cannot be caught.

**Whether “a missed update may go unnoticed” must be distinguished precisely**:

| Missed-update type                                           | Detected by the compiler? | Verified evidence                                                                                                                                                             |
| ------------------------------------------------------------ | ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Missing arm in an exhaustive `match`                         | **Yes**                   | `impl Display for BinOp` at `const_data.rs:291-317` is an 18-arm exhaustive match with no `_` fallback.                                                                       |
| Missing item in an enumerated `matches!` macro               | **No**                    | The `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` functions at `const_data.rs:263-288` all use partial `matches!` lists; a new variant silently returns `false`. |
| Missing arm in a `match` with a `_` fallback                 | **No**                    | `_ => {}` at `ownership.rs:687` silently ignores a new variant.                                                                                                               |
| **Missing update across `ast::BinOp` / `const_data::BinOp`** | **Never**                 | The two have no conversion and no common parent type, so the compiler cannot associate them.                                                                                  |

The accurate statement is therefore not “nobody notices when 11 locations are missed,” but: **the
compiler covers the exhaustive-match portion; the truly silent cases are `matches!` macros, `_`
fallbacks, and cross-enum updates.**

### Dead Code and Test Status

This is **the item in this document that most urgently requires action**.

#### Dead Code

- `Precedence` enum (`precedence.rs:35-94`, 60 lines) and `PrecedenceContext` struct
  (`precedence.rs:98-133`, 36 lines): zero production use; their only external referrer is the
  orphan file `pratt/tests/precedence_inline.rs` (95 lines / 6 tests, not declared by
  `pratt/tests/mod.rs:4-6`).
- `skip_old_function_syntax` (`declarations.rs:120-122`): a pure empty function; its call site at
  `:725` is followed directly by `return None`, and no token is consumed.
- `BP_RANGE` (`precedence.rs:22`): zero production use; it survives only in the comment at
  `led.rs:157-159`.

#### Only One Lexer Test File Is Running

**The only test declaration in `lexer/mod.rs:104-106` is:**

```rust
#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
```

A `grep 'mod tests'` over the entire `src/frontend/core/lexer/` directory matches only this
location. **`lexer/` has never declared `mod tests;`.**

The entire `src/frontend/core/lexer/tests/` subtree has therefore **never participated in
compilation**. The directory actually contains **14 files** (including `mod.rs`):

| File                                                                              | Lines          | Status                                      |
| --------------------------------------------------------------------------------- | -------------- | ------------------------------------------- |
| `mod.rs`                                                                          | 38             | Present; declares 11 submodules             |
| `literals.rs`                                                                     | 222            | Real tests, **orphaned**                    |
| `rfc010_lexer.rs`                                                                 | 167            | Real tests, **orphaned**                    |
| `fstring.rs`                                                                      | 90             | Real tests, **connected through `#[path]`** |
| `lexer_mod.rs`                                                                    | 89             | Real tests, **doubly orphaned**             |
| `rfc004_lexer.rs`                                                                 | 81             | Real tests, **orphaned**                    |
| `symbols.rs`                                                                      | 70             | Real tests, **doubly orphaned**             |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 1–3 lines each | 7 empty shells                              |
| `debug_lexer.rs`                                                                  | 1              | Empty shell                                 |

**Real but never-run tests: 629 lines / 5 files** (222+167+89+81+70). Only one f-string test file is
actually running in the lexical layer.

**A wiring trap that must be documented clearly: `tests/mod.rs` exists, but it is not itself fully
connected.**

`lexer/tests/mod.rs:15-25` declares 11 submodules (`basic` / `literals` / `operators` / `delimiters`
/ `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer` / `fstring`),
and `:28-38` then `pub use`s all of them for “backward-compatible re-export.”

**However, it does not declare `lexer_mod` or `symbols`.** Therefore:

> **Adding only one `mod tests;` line is not sufficient.** Reviving `lexer/tests/` requires **two
> changes**: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` to
> `tests/mod.rs`. Otherwise, those 159 lines still will not run.

**The trap of the 7 empty shells:** statements such as `pub use basic::*;` at `tests/mod.rs:28-38`
**produce neither a compilation error nor a warning when referring to a module with zero
assertions**. After adding `mod tests;`, CI will report “all tests passed” even though the seven
modules `basic` / `comments` / `delimiters` / `errors` / `keywords` / `operators` / `debug_lexer`
contain **not a single assertion**. **This creates the false impression that the lexical layer is
covered and is more dangerous than leaving the tests disconnected entirely.**

**The highest-risk uncovered paths**—immediately exposed once connected—are:

| Path                                                                                | Location                                                                      | Risk                                 |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------ |
| Overflow paths of the four radix scanners                                           | `literals.rs:134-141` / `220-227` / `306-313` + corresponding decimal section | Integer overflow is a silent error.  |
| “Continue consuming without reporting an error” branch after `checked_mul` overflow | 4 isomorphic locations                                                        | May cause token-stream misalignment. |
| The **entire** `scan_leading_dot` function                                          | `literals.rs:519-635` (117 lines)                                             | Zero coverage                        |
| Invalid `\x` / `\u` escape paths                                                    | `literals.rs:730-749` / `1051-1070`                                           | Error paths are difficult to test    |

**Parser-side wiring is correct**: `parser/mod.rs:8-9`, `pratt/mod.rs:8-9`, and
`statements/mod.rs:11-12` all correctly declare `mod tests;`. `pratt/tests/mod.rs:4-6` declares
`led` (335 lines), `nud` (269 lines), and `precedence` (13 lines).

**However, parser-side coverage quality has the same problem**:

- The only assertion in `pratt/tests/precedence.rs` (**13 lines**) is
  `assert!(bp_lowest < bp_highest)`, making it **almost a no-op**—it cannot even detect the four
  cross-ladder collisions listed above.
- `pratt/tests/precedence_inline.rs` (**95 lines / 6 tests**) is not declared by
  `pratt/tests/mod.rs` and **never runs**—and it tests only the zero-reference `Precedence` enum and
  `PrecedenceContext`.
- **The two ladders actually in use (`BP_OR`…`BP_MUL` and
  `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`) have no direct unit tests.**

## Target Design

### Lexing: Declarative-ization

#### Evaluation of Three Approaches

| Approach                                                  | Cost of adding one token       | Can eliminate the lexical layer’s three duplications | Can solve f-string nested compilation       | Introduced dependency                                                          |
| --------------------------------------------------------- | ------------------------------ | ---------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| **A. Generate a DFA with `logos`**                        | 1 location (token declaration) | **Yes** (one copy each for escapes / numbers)        | No (interpolation must be handled manually) | Adds `logos`                                                                   |
| **B. Handwrite a DFA with `regex`**                       | 1–2 locations                  | Partially                                            | No                                          | Adds `regex` (whether it is already a transitive dependency is **unverified**) |
| **C. Retain hand-written code and enforce deduplication** | 1 location                     | **Yes** (manually extract common functions)          | **Yes** (requires explicit design)          | None                                                                           |

**Position: adopt C as the baseline, A as a conditional alternative, and reject B.**

Rationale:

1. **C can eliminate all approximately 500 lines of duplication in the lexical layer with zero new
   dependencies**, entirely within a controllable scope. The three escape decoders (`scan_string` /
   `scan_char` / `push_fstring_escape`) converge into a **single**
   `decode_escape(lexer, &mut String, EscapeMode)`, and the four radix scanners converge into a
   **single** `scan_radix_number(lexer, Radix, String)`. This is a purely mechanical refactoring,
   judged by C5’s “identical AST snapshots + identical diagnostics”.
2. **A (`logos`) has real capabilities**—it can converge escape decoding and number scanning to one
   implementation each. **However, it cannot solve f-string nested compilation**, which is the
   lexical layer’s most important defect. Introducing a new dependency also changes `Literals`’
   error type (the `InvalidEscape` / `InvalidNumber` variants of `LexError` must be remapped), which
   **directly violates C5’s “identical diagnostics” criterion** unless an additional adapter is
   written—and that would introduce yet another manual synchronization point.
3. **B (“handwritten DFA with `regex`”) is rejected**: it requires a handwritten state machine, with
   workload comparable to A but far less controllability. Logically, if choosing between A and B, A
   must be selected; since this document advocates C first, there is no reason to choose B.

> **Design judgment (not a verified fact)**: A may be the better long-term shape (a declarative
> token table is itself a form of enforcement that changes only one location). However, its
> migration cost and conflict with the “identical diagnostics” criterion make it unsuitable for this
> C5 change. If A is adopted later, it should be a separate proposal, using this document’s
> test-revival results as a safety net.

#### f-String Nested Compilation Must Be Addressed Explicitly

Regardless of whether A or C is selected, nested compilation must be resolved. **Position: eliminate
the runtime nested `tokenize()` calls and produce the result in a single lexical pass.**

Specifically: when `scan_string` encounters the `f"` prefix, it **records the interpolation content
as `FStringSegment::RawText(String)` during the same lexical scan**, and does **not** recursively
call `tokenize()`. The actual tokenization and parsing of interpolations are deferred until the
parser first needs the f-string node, at which point:

- **It is completed in one pass** (for N interpolations, perform one tokenization pass over the
  interpolation segments and cache the result as `Vec<Vec<Token>>`), reducing N+1 passes to one;
- **Span mapping is determined during lexing** (recording each interpolation’s absolute offset in
  the source file), no longer depending on the relative semantics of `Point`.

This change is **independent of the paradigm choice** and should be performed even if A is not
adopted.

### Parsing: An Unambiguous Language Surface + a Single Declarative Precedence Table

**Target shape**: the parser has only one Pratt implementation; binding powers and operator
properties are declared only in one declarative table; the language surface itself is unambiguous,
so “adding a binary operator” requires changes only to the table, SPEC, and tests.

**Language surface (converged on 2026-11-25)**: SPEC syntax §2.8 / §2.9 / §2.10 / §3.1 / §3.2.1
agrees with the implementation—

- A match branch uses `->` (`=>` belongs only to lambdas), and branches are separated by a newline
  or `,`;
- Statements are separated by a newline or `;`; adjacent statements without a separator are a syntax
  error (E0012);
- Destructuring is written only as `(a, b) = e`; the bare-identifier form is not syntax;
- The bare single-parameter lambda `x => e` and the parenthesized form coexist.

**Precedence table**: `pratt/precedence.rs` is the sole declaration point—`INFIX_OPS` (27 lines = 25
single-token + 2 double-token operators) and `PREFIX_OPS` (6 lines) each provide the token, left and
right binding powers, `ast::BinOp`, and parsing strategy (`Binary` / `Assign` / `Shift` / `In` /
double token). `nud.rs` / `led.rs` only construct and dispatch; they do not hold operator-level
binding powers. `parse_assign` also obtains its operator from the table.

**Two `BinOp` representations**: the correspondence between `ast::BinOp` and
`types::const_data::BinOp` is derived from the table’s `op` field. The sole production mapping point
is `types/eval/const_eval.rs::convert_ast_binop`; on the test side,
`pratt/tests/operator_sources.rs` uses a cross-validation table to check each operator and
exhaustively enumerate variants, preventing the two representations from drifting.

**Equivalence criteria**: full C5—corpus differential testing and IR snapshots are regenerated in
the same batch, and diagnostic code + span are identical entry by entry. Existing precedence defects
(the levels of `as` and `*`, `as` target types swallowing subsequent expressions, and the levels of
`==` / `!=` relative to comparisons) are fixed along the way under D59 A/B/C: the red proof is
reproduced in the parent revision, and the green-state assertions are permanently placed in
`parser/tests/associativity.rs`.

**Boundary**: no syntax generator is introduced; lexing remains hand-written. Error recovery
(`synchronize()` / `Expr::Error` / `StmtKind::Error`) is outside this change’s scope.

### Disposition of `parse_assign_after_target`

**Position: split it into eight named functions, with the original function body doing only
dispatch.**

A 507-line function with eight responsibility sections forces every local modification to be located
within 500 lines. The target shape after splitting is:

| Extracted function             | Responsibility                                           | Approximate lines | Ownership                                                                                 |
| ------------------------------ | -------------------------------------------------------- | ----------------- | ----------------------------------------------------------------------------------------- |
| `probe_old_function_syntax`    | Old-syntax detection                                     | 36                | **Deleted by this document**                                                              |
| `resolve_annotation_ambiguity` | Annotation disambiguation (three-level nested lookahead) | To be verified    | This document (substantially shortened after extracting an independent lookahead utility) |
| `check_declaration_legality`   | Declaration legality                                     | To be verified    | This document                                                                             |
| `apply_semantic_side_effects`  | Semantic side effects                                    | To be verified    | **03-type-unification.md** (including the `"Terminates"` hardcoding at `498-509`)         |
| `parse_type_body`              | Type-body parsing fallback                               | To be verified    | This document                                                                             |
| `rebuild_lambda_params`        | RFC-010 lambda signature parameter reconstruction        | To be verified    | This document                                                                             |
| `parse_meta_type_def`          | MetaType definition                                      | To be verified    | This document                                                                             |
| `parse_plain_init`             | Ordinary initialization                                  | To be verified    | This document                                                                             |

Supporting change: converge the four copies of “skip balanced parentheses” (`96-105` / `147-156` /
`191-200` / `224-233`) into a single `ParserState` method, `skip_balanced_parens()`, and replace all
four copies with calls to it.

**Net effect (design judgment)**: 507 lines → eight single-responsibility functions + one lookahead
utility; lookahead code drops from 48 lines to approximately 12 lines; after moving type-layer logic
out (approximately 80 lines), the portion actually owned by this document is approximately 300
lines.

### Dead Code Cleanup (C6, No Equivalence Criteria Required)

Using C6 as defined in 07-equivalence-oracle.md (“pure deletion; no equivalence criteria required,
only confirmation that there are no references”):

| Cleanup item                                     | Location                                   | Lines | Prerequisite                                                                                                    |
| ------------------------------------------------ | ------------------------------------------ | ----- | --------------------------------------------------------------------------------------------------------------- |
| `Precedence` enum                                | `precedence.rs:35-94`                      | 60    | **Must handle `pratt/tests/precedence_inline.rs` at the same time** (its only referrer, itself an orphan)       |
| `PrecedenceContext` struct                       | `precedence.rs:98-133`                     | 36    | Same                                                                                                            |
| Six zero-reference constants in the first ladder | `precedence.rs:8-13`, `:11` (comment only) | 6     | Zero production references; delete directly                                                                     |
| `BP_RANGE`                                       | `precedence.rs:22`                         | 1     | Zero production references (only in the comment at `led.rs:157-159`); first change `..` to use a named constant |
| `skip_old_function_syntax`                       | `declarations.rs:120-122`                  | 3     | Empty function; remove its call site at `declarations.rs:725`                                                   |
| `is_old_function_syntax`                         | `declarations.rs:82-117`                   | 36    | See below                                                                                                       |
| `declarations.rs:145-167`                        | Same file                                  | 23    | Same                                                                                                            |
| Raw magic number `(6,7)`                         | `led.rs:40`                                | —     | Promote to named `BP_RANGE_L` / `BP_RANGE_R` constants                                                          |
| Raw magic number `(11,1)`                        | `led.rs:85`                                | —     | Promote to `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                        |
| Raw magic number `12`                            | `nud.rs:1041`                              | —     | Promote to `BP_PATTERN_PROBE`, and **record in a comment that it was found empirically**                        |

> **Note**: `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` are live
> constants in the `precedence.rs` table and are maintained with it. Phase 0 deletes only the
> `Precedence` enum and `PrecedenceContext`, not these constants.

**Disposition of `is_old_function_syntax`**: it is a still-running “reject removed syntax” gate
(`declarations.rs:689`). The prerequisite for deletion is proof under the C5 criteria that the
diagnostic code and span remain unchanged—`f(Int) -> Int = x => x` must be measured before and after
deletion. Until then, retain the current implementation.

### Test Reconstruction

**This is the first priority and must precede every paradigm change.**

**Phase 1: revive `lexer/tests/`.** This requires **two changes**, not one:

1. Add `#[cfg(test)] mod tests;` after `:106` in `src/frontend/core/lexer/mod.rs`.
2. Add `mod lexer_mod;` and `mod symbols;` to `src/frontend/core/lexer/tests/mod.rs` (otherwise,
   those 159 lines still will not run).

**Phase 2: dispose of the 7 empty shells.** The key constraint is that `pub use xxx::*;` at
`tests/mod.rs:28-38` produces **neither a compilation error nor a warning** when referring to an
empty module. Once connected, CI will report “all green” despite there being zero coverage.
Disposition options:

| Option                   | Action                                                                                            | Evaluation                                                                                                                                      |
| ------------------------ | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **Delete (recommended)** | Delete the 7 empty-shell files + corresponding `mod` declarations + corresponding `pub use` lines | **Clean.** The only value of the shells is to fill them in later, but `literals.rs` / `rfc004_lexer.rs` already perform those responsibilities. |
| Fill                     | Write real assertions for each empty shell                                                        | 7 files × unknown content; **what should be tested cannot be verified** (unverified)                                                            |
| Retain + mark            | Retain them with `// TODO` and a CI count warning                                                 | Creates the false impression of coverage; **rejected**                                                                                          |

**Recommendation: delete** them, and at the same time as the dead-code cleanup, delete **all**
`pub use` re-exports at `tests/mod.rs:28-38` (they existed to support a nonexistent re-export design
and were precisely what concealed the empty shells).

**Phase 3: add associativity cases.** Add `parser/tests/associativity.rs` (WBS 8.8.1), using only
the public entry points `parser::parse` / `parse_with_probe` to assert **language-level AST
shapes**. The cases do not reference Pratt internals (binding-power constants, the `pratt` submodule
path, or `ParserState`), decoupling them from Pratt’s internals and protecting C5’s equivalence
surface: “the AST shape of valid programs remains unchanged”:

- Write one minimal expression for each pair of adjacent levels and assert its associativity
  direction—for example, `a - b - c` → `(a-b)-c`. The level and associativity columns are in SPEC
  syntax §2.2.
- Add one case each for `..` (`a..b..c` → `(a..b)..c`, with the upper bound of `0..n+2` swallowing
  the addition), right-greedy `=>` bodies, right-associative assignment, `?` with call-chain
  suffixes, and pattern `|` chain closure. The `..` and `=>` binding-power pairs are the two
  locations most likely to be missed during this migration.
- **Do not use “binding powers strictly increase within a level, with no duplicates” as a gate
  assertion**: under coding-rules 7.1, a correct implementation that reorders values within the same
  level would turn red. This freezes an implementation choice and may only serve as
  characterization. The level relationships are protected by the two groups of language-level AST
  shapes above.

**Phase 4: add literal error-path cases.** Add cases for each of the four categories in the
uncovered-path table: four radix overflows, the “continue consuming after `checked_mul`” branch,
`scan_leading_dot` as a whole, and invalid `\x` / `\u` escapes.

**Phase 5: delete `precedence_inline.rs`** along with the dead code, and add a **wiring self-check**
assertion to `pratt/tests/mod.rs` to prevent another orphan.

### Number of Locations Changed by Adding an Operator: Before and After

| #   | Current state                                                                    | Current enforcement                                 | Target                                             | Target enforcement                               |
| --- | -------------------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------- | ------------------------------------------------ |
| 1   | Add a match arm to `keyword_from_str` at `lexer/state.rs:21` (keyword form only) | None                                                | One line in the operator declaration table         | Compile-time                                     |
| 2   | Add a variant to `TokenKind` at `lexer/tokens.rs:81-162`                         | Compile-time                                        | **Generated from the table**                       | Compile-time                                     |
| 3   | Choose one ladder at `pratt/precedence.rs:6-17` / `22-31` and add a BP constant  | None                                                | **Generated from the table**                       | Compile-time (within-level uniqueness assertion) |
| 4   | Add an arm to `infix_info` at `pratt/led.rs:30-88`                               | Compile-time (exhaustive)                           | **Generated from the table**                       | Compile-time                                     |
| 5   | Add an operator-mapping arm to `parse_binary` at `pratt/led.rs:116-136`          | Compile-time (exhaustive)                           | **Generated from the table**                       | Compile-time                                     |
| 6   | Add a variant to `BinOp` at `ast.rs:191-213`                                     | Compile-time                                        | **Generated from the table**                       | Compile-time                                     |
| 7   | Add a new symbolic character at `tokenizer.rs:218` `next_token_inner`            | None                                                | **Generated from the table**                       | Compile-time                                     |
| 8   | Add a variant to `const_data::BinOp` at `const_data.rs:234`                      | **None (cross-enum; the compiler cannot catch it)** | **Generated from the same source as `ast::BinOp`** | Compile-time                                     |
| 9   | Depend on `matches!` / `_` fallbacks at sites in 17 downstream production files  | **None (silent)**                                   | Generated accessors                                | Compile-time                                     |

**Summary:**

| Stage                            | Manual change locations                                                                                                 | Automatically generated locations |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| Current state                    | **11–12** (6–7 within L2 + approximately 5–6 downstream production files that must be synchronized manually)            | 0                                 |
| Target (new symbolic operator)   | **1–2** (one line in the operator declaration table; one additional line in `keyword_from_str` if it is a keyword form) | 7–8                               |
| Target (reuse an existing token) | **1**                                                                                                                   | 6–7                               |

All 1–2 target locations are **declarative data lines**, not logic code—reducing review cost from
“deduce the semantics of each match arm individually” to “read one declaration”.

## Detailed Design

### Interface Boundary with the Type System

**Boundary declaration (important)**: convergence of `ast::Type` (26 variants, `ast.rs:427-541`),
elimination of parallel type representations, and ownership of `is_type_param_annotation` /
`name_used_as_type` **all belong to [03-type-unification.md](03-type-unification.md)**. This
document handles only three interfaces directly related to the frontend paradigm:

**Interface 1: the relationship between `const_data::BinOp` and `ast::BinOp`.** This belongs to
**this document**, because it is a direct component of the change surface for “adding an operator”.
Disposition: **simultaneously generate both enums from the operator declaration table** (or generate
one and derive the other), eliminating the import aliases at `const_eval.rs:19` / `ir_gen.rs:2317`.
The reason it belongs here rather than to 03 is that it is an L2 artifact copied in parallel into
L3, not a problem with type representation itself.

**Interface 2: the hardcoded `"Terminates"` at `declarations.rs:498-509` and the call to
`typecheck::operator_interfaces::spec()`.** This belongs to **03-type-unification.md /
02-stage-contract.md** (eliminating the parser → typecheck reverse dependency, RFC-039 routing table
C, first row). During the function split, this document **isolates this code in
`apply_semantic_side_effects`** as the handoff point, but does not alter its contents.

**Interface 3: whether an operator’s precedence level needs type-layer information.** It does not
currently—`infix_info` is a purely syntactic function. **Design judgment**: this property should be
retained after the precedence table is introduced. The operator declaration table **must not contain
any type-related fields**, or L2 will once again depend on L3 in violation of RFC-039’s
dependency-direction rules.

### Runtime Behavior

**Goal: runtime behavior remains bit-for-bit unchanged.** Under the C5 criteria in
07-equivalence-oracle.md, the third layer (end-to-end corpus differential testing) compares the exit
codes and stdout/stderr of 293 `.yx` files in `tests/yaoxiang/` plus `src/std/tests/*.yx`.

The three **identified runtime impacts** must be handled explicitly:

| Change                                                                          | Runtime impact                                 | Disposition                                                                                           |
| ------------------------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Consolidating f-string interpolation into one tokenization pass                 | **Performance improvement**, output unchanged  | Measure and record the timing benefit                                                                 |
| Generating BPs from the operator table / merging and reordering the two ladders | **May change associativity**                   | **An equivalence case must be written for every adjacent level pair** (Test Reconstruction · Phase 3) |
| Deleting `is_old_function_syntax`                                               | Diagnostic code/span for old syntax may change | Use corpus differential testing to prove they remain identical                                        |

**Explicit non-goals**: do not change the precedence **semantics** of operators. BP numbers may be
reordered to eliminate dead code and collisions, but relationships such as “multiplication binds
more tightly than addition” **must remain unchanged**.

### Compiler Change List

File by file, with line numbers. C6 cleanup (Phase 0, independently committable; **8.0.1 / 8.0.2
were executed on 2026-11-22; the line numbers below refer to the tree before execution, and the
final state is recorded in the task 8.0 commit message**):

| File                                                        | Lines                                 | Change                                                                                                             |
| ----------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `src/frontend/core/parser/pratt/precedence.rs`              | `8-13`, `11`                          | Delete the six zero-reference constants from the first ladder                                                      |
| Same                                                        | `22`                                  | Change `BP_RANGE` to a named `..` binding power (synchronized at `:40`), or delete it                              |
| Same                                                        | `35-94`                               | Delete the `Precedence` enum                                                                                       |
| Same                                                        | `98-134`                              | Delete the `PrecedenceContext` struct                                                                              |
| Same                                                        | `6-17`, `22-31`                       | Retain live constants and merge them with the second ladder into conflict-free within-level numbering              |
| `src/frontend/core/parser/pratt/led.rs`                     | `33`                                  | Point `BP_ASSIGN` at the reordered assignment-level constant                                                       |
| Same                                                        | `40`                                  | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R`                                                                              |
| Same                                                        | `75`, `77`, `79`, `83`                | Point `BP_CALL` at the reordered suffix-level constant                                                             |
| Same                                                        | `81`                                  | Point `BP_CAST` at the reordered cast-level constant                                                               |
| Same                                                        | `85`                                  | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                           |
| `src/frontend/core/parser/pratt/nud.rs`                     | `33`, `36`, `84`, `102`, `228`, `248` | Point `BP_UNARY` at the reordered prefix-level constant                                                            |
| Same                                                        | `39-70`                               | Point `BP_HIGHEST` at the reordered highest-level constant                                                         |
| Same                                                        | `1019`                                | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`, and add a comment explaining the value’s provenance |
| `src/frontend/core/parser/statements/declarations.rs`       | `85-87`                               | Delete `skip_old_function_syntax`                                                                                  |
| Same                                                        | `695`                                 | Delete its call site (deletion of `is_old_function_syntax` is handled in Phase 2)                                  |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | Entire file                           | Delete it (the orphan file that uniquely references the dead code)                                                 |

Supporting renames resulting from BP consolidation (approximately 89 locations):
`parser/mod.rs:113`, eight locations in `statements/control_flow.rs`,
`statements/declarations.rs:19` / `287` / `577` / `705` / `785` / `821` / `940` / `972`,
`statements/functions.rs:9` / `:107`, `statements/bindings.rs:170`, `statements/types.rs:16` /
`:782` / `:794`, `pratt/led.rs:99` / `:227` / `:315` / `:321` / `:409`, and `pratt/nud.rs:200` /
`:268` / `:418` / `:471` / `:478`.

Test revival (Phase 1, independently committable):

| File                                                 | Lines               | Change                                                                   |
| ---------------------------------------------------- | ------------------- | ------------------------------------------------------------------------ |
| `src/frontend/core/lexer/mod.rs`                     | `167-168`           | Add `#[cfg(test)] mod tests;`                                            |
| `src/frontend/core/lexer/tests/mod.rs`               | `12-18`             | Add `mod lexer_mod;` and `mod symbols;`                                  |
| Same                                                 | Entire file         | Delete the `mod` declarations and `pub use` lines for the 7 empty shells |
| Same                                                 | Entire file         | Delete the `pub use ... ::*;` re-export block (`28-38`)                  |
| `src/frontend/core/lexer/tests/`                     | 7 empty-shell files | Delete the files                                                         |
| `src/frontend/core/parser/pratt/tests/mod.rs`        | `4-7`               | Add a wiring self-check assertion                                        |
| `src/frontend/core/parser/pratt/tests/precedence.rs` | `6-13`              | Replace the no-op assertion with real associativity assertions           |

Paradigm changes (Phases 2–4):

| File                                                           | Change                                                                                                                                                                                                                                      |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/literals.rs`                          | Merge the four radix scanners into `scan_radix_number(lexer, Radix, String)`; merge the three escape decoders into the single `decode_escape(lexer, &mut String, EscapeMode)`; merge `scan_multi_line_string` and `scan_fstring_multi_line` |
| `src/frontend/core/parser/pratt/nud.rs:367` / `:398`           | Remove the runtime nested `tokenize()` calls; produce the result once during lexing                                                                                                                                                         |
| `src/frontend/core/parser/pratt/led.rs:30-88` / `nud.rs:27-76` | Replace the two handwritten tables with generated mapping tables                                                                                                                                                                            |
| `src/ast/mod.rs:194-218`                                       | Generate `BinOp` and `const_data::BinOp` from the same source                                                                                                                                                                               |
| `src/frontend/core/types/const_data.rs:290`                    | Same                                                                                                                                                                                                                                        |
| `src/frontend/core/parser/statements/declarations.rs:101-620`  | Split into eight functions + `ParserState::skip_balanced_parens()` (replace the four locations at `61-70`/`112-120`/`156-164`/`186-194` with calls)                                                                                         |
| `src/frontend/core/parser/parser_state.rs`                     | Add `skip_balanced_parens()` (`synchronize()` at `:330` remains unchanged)                                                                                                                                                                  |

**Files with no changes under this document**: the **main bodies** of `src/middle/core/ir_gen.rs`,
`src/frontend/core/typecheck/**`, `src/backends/**`, and `src/frontend/core/formatter/**`. They
change only when generated accessors replace `matches!` macros, and those changes are mechanical
substitutions.

### Backward Compatibility

Error recovery is unchanged in this round. Three recovery contracts must be preserved:

**Contract 1: `ParserState::synchronize()` (`parser_state.rs:170-186`).** It advances past tokens
until a synchronization point after statement parsing fails. **The C5 criteria require retaining it
and require its trigger timing and skipped-token set to remain exactly the same.** Its call sites
and synchronization-point set remain unchanged.

**Contract 2: `Expr::Error` (`ast.rs:1048`, consumed at `pratt/mod.rs:72`) and `StmtKind::Error`
(constructed at `parser/mod.rs:50`).** These placeholders allow the parser to continue constructing
a “partial AST” after an error, with downstream code skipping the `Error` branch. **The variants
must be retained, and their position in downstream matches must also be retained** (if downstream
code puts `Expr::Error` in a wildcard branch, the error is silently swallowed).

**Contract 3: diagnostic code, span, and order.** The third-layer comparison rules in
07-equivalence-oracle.md sort the diagnostic list by `(code, span.file, span.line)` and **do not
include message text in the comparison**. This gives this document some latitude over wording, but
**code and span must remain unchanged**.

**Special note on f-strings**: the interpolation refactoring changes spans from “relative to the
interpolation text” to “absolute offsets in the source file”. To satisfy Contract 3, **spans must be
recorded as absolute offsets during lexing** (Decision D25); `span.file/line` for diagnostics inside
an interpolation must not change as a result of this refactoring.

> **Disposition**: before Phase 2, establish a baseline snapshot of “diagnostic spans inside
> f-string interpolations” and compare every entry after the refactoring. Any difference in
> `span.file/line` is an implementation defect that must be fixed; there is no option to “accept a
> span change”. (If measurement proves that the original interpolation spans were already incorrect,
> the baseline-establishment phase will expose that. It is an independent pre-existing defect and
> must be fixed separately, not folded into this phase, and does not justify relaxing the criteria.)

## Implementation Points

This document corresponds to five phases. **Each phase can be committed and rolled back
independently.** Phases 0 and 1 are C6 and test revival and do not require equivalence criteria;
Phases 2–4 are C5 and are subject to the criteria throughout.

| Phase | Content                                                                                                                                                | Category | Acceptance criteria                                                                                                                          | Rollback point                         |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- | -------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **0** | Dead-code cleanup (`Precedence` enum + `PrecedenceContext`, 96 lines)                                                                                  | C6       | `cargo build` passes; `cargo clippy` introduces no new warnings                                                                              | One commit; `git revert` is sufficient |
| **1** | Test revival and completion                                                                                                                            | —        | The lexical layer actually runs tests in ≥ 5 files / 629 lines; the 7 empty shells are deleted; associativity cases are **red before green** | One commit                             |
| **2** | Lexical duplication convergence + elimination of f-string nested compilation                                                                           | C5       | Identical AST snapshots + identical diagnostics + identical behavior; `literals.rs` shrinks by approximately 500 lines                       | Separate commit from Phase 3           |
| **3** | Syntax surface + precedence convergence (canonical SPEC form, single `pratt/precedence.rs` table, cross-validation of the two `BinOp` representations) | C5       | Zero drift in corpus differential testing and IR snapshots; diagnostic code+span unchanged; D1–D3 are red before green                       | Independent commit                     |
| **4** | Split `parse_assign_after_target`                                                                                                                      | C5       | The function only dispatches; `skip_balanced_parens` has a single implementation                                                             | Independent commit                     |

**Internal dependency order**: Phase 1 **must precede** Phases 2/3/4—“refactor first, add tests
later” is a trap this project has already encountered. Phases 3 and 4 are independent commits.

**One clarification about Phase 0**: `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST`
/ `BP_HIGHEST` are level constants in `precedence.rs` and belong to the precedence table. Phase 0
deletes only the `Precedence` enum and `PrecedenceContext` (96 lines, zero production references).

**How the criteria are executed**: the definitions and regression gates for the three-tier criteria
are in [07-equivalence-oracle.md](07-equivalence-oracle.md). **Everything uses the complete C5
criteria, including diagnostic code+span. There are no relaxations.**

**Scope limitation**: this document **does not include** a `logos` migration (lexing remains
hand-written, with duplicate implementations merged). The syntax surface and precedence-table
convergence are included; see “Parsing: An Unambiguous Language Surface + a Single Declarative
Precedence Table”.

## Key Decisions and Rationale

### Three Selected Decisions

| Decision                         | Selection                                                                                                                                    | Rationale                                                                                                                                                                                                                                                               |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexical approach                 | Adopt “retain hand-written code + enforce deduplication”; list `logos` as a conditional alternative                                          | Eliminates approximately 500 lines of duplication with zero new dependencies; `logos` changes the `LexError` error type and cannot solve f-string nested compilation                                                                                                    |
| **Parsing approach**             | Retain Pratt; converge binding powers and operator properties into a single `pratt/precedence.rs` table; do not introduce a syntax generator | The single table is the sole declaration point, so “adding a binary operator” requires changes only to the table + SPEC + tests; the two `BinOp` representations are derived from the table’s `op` field and cross-validated; error-recovery contracts remain unchanged |
| Ownership of `const_data::BinOp` | This document                                                                                                                                | It is a direct component of the “add an operator” change surface; it is handled separately from `ast::Type` convergence (03)                                                                                                                                            |

### Delivery Form

- Binding powers and operator properties are declared in exactly one place: `pratt/precedence.rs`;
  `nud.rs` / `led.rs` only construct and dispatch.
- `parse_assign` / `parse_call` obtain operator subtrees from the table rather than hard-coding them
  at call sites.
- The two `BinOp` representations are derived from the table’s `op` field, cross-validated on the
  test side, and exhaustively enumerated.
- The language surface converges on the canonical SPEC form, with the corpus and documentation
  migrated in the same batch.
- Equivalence is guaranteed by the complete C5 criteria: corpus differential testing, IR snapshots,
  and diagnostic code + span.

## Known Limitations and Risks

### Design Risks

- **The operator table is a new “thing that must be maintained.”** It turns “change 11 locations”
  into “change one location + maintain a table.” **If the table’s fields are poorly designed, it
  will become a new centralized maintenance burden.** This is the primary risk of the approach;
  mitigation is to protect every column of the table with compile-time assertions.
- **The risk from renumbering BP levels is asymmetric.** Merging the two ladders involves renaming
  approximately 89 production references; if one is missed, **associativity may silently change**
  (no error is reported; only the parsed expression differs). This is the most likely silent
  regression introduced by this document and must be guarded by Phase 1’s adjacent-level
  associativity cases.
- **The C5 criteria for Phases 2–4 are expensive.** Using all three tiers means every phase must run
  the full corpus (293 `.yx` files) + AST snapshots + diagnostic comparisons. This makes every
  commit in Phases 2–4 relatively heavy.
- **The f-string span change may not be completely avoidable.** See “Backward Compatibility”; it may
  be necessary to accept one known behavior change and update the baseline.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **This document leaves no pending items.**

## Verification Record

All code facts in this document were verified by **direct reads from disk**. Verification method:
the `read` tool directly read the relevant line ranges in the target files; line counts were
obtained with `Get-Content`; references were scanned repository-wide with the `grep` tool and
classified as production / test / orphan. **`cargo test` / `cargo build` were not used**—this is a
pure documentation artifact, and criterion execution belongs to the implementation phase.

### Item-by-item Measured Results

| Verification item                                 | Measured result                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Total lines in `literals.rs`                      | **1568**                                                                                                                                                                                                                                                                                                         |
| Total lines in `tokenizer.rs`                     | **554**                                                                                                                                                                                                                                                                                                          |
| Line number of `next_token_inner`                 | **`:218`**                                                                                                                                                                                                                                                                                                       |
| Line ranges of the four radix scanners            | `77-161` (85 lines) / `164-247` (84) / `250-333` (84) / `336-516` (181)—matching this document’s table                                                                                                                                                                                                           |
| Three escape-decoding implementations             | `694-827` (134 lines) / `1015-1141` (127 lines) / `push_fstring_escape:1374`—matching this document’s table                                                                                                                                                                                                      |
| `scan_leading_dot`                                | `519-635`, **117 lines**                                                                                                                                                                                                                                                                                         |
| `Expr` variant count                              | **22** (including `Error`), `ast.rs:16-163`                                                                                                                                                                                                                                                                      |
| Number of files in `lexer/tests/`                 | **14** (including `mod.rs`)                                                                                                                                                                                                                                                                                      |
| Orphaned real-test lines                          | **629** (222+167+89+81+70); another 90 lines in `fstring.rs` already run through the `#[path]` connection and are not counted as orphaned                                                                                                                                                                        |
| Changes required to revive `lexer/tests/`         | **Two**: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` `mod symbols;` to `tests/mod.rs`. Adding only one leaves the final 159 lines unrun                                                                                                                                                     |
| Dead code in `precedence.rs`                      | **96 lines** (the `Precedence` enum at `35-94` + `PrecedenceContext` at `98-133`) with zero production use, **confirmed**; however, their only external referrer, `precedence_inline.rs` (95 lines / 6 tests), is itself an orphan—**deleting the dead code requires deleting that file at the same time**       |
| Lines in the 7 empty shells                       | 1–3 lines each (`debug_lexer.rs` has 1 line; the others have 3)                                                                                                                                                                                                                                                  |
| Shape of the blank-line residue                   | **Three consecutive blank lines inserted between each field, spanning 11 lines** (`777-787` / `1098-1108`)                                                                                                                                                                                                       |
| Lines in `pratt/tests/precedence.rs`              | **13**, with the sole assertion `assert!(bp_lowest < bp_highest)`                                                                                                                                                                                                                                                |
| The 4th “skip balanced parentheses” location      | `224-233`                                                                                                                                                                                                                                                                                                        |
| Downstream change surface                         | **17 production files** match `BinOp::` (repository-wide: **534 hits / 47 files**)                                                                                                                                                                                                                               |
| Is `src/backends/` a `BinOp` consumer?            | **No**—`src/backends/` has **zero hits** for `BinOp` or `ast::`; the interpreter consumes IR / bytecode and never sees the AST                                                                                                                                                                                   |
| “Can an omitted update go completely undetected?” | **Must be stratified**: a missing exhaustive-`match` arm **is detected by the compiler**; the silent cases are `matches!` macros, `_` fallbacks, and **cross-enum** updates. This document now uses the measured stratification.                                                                                 |
| `matches!` blocks in `const_data.rs`              | `263-288` (the four-function range for `is_arith` / `is_comparison` / `is_logical` / `is_bitwise`)                                                                                                                                                                                                               |
| `impl Display for BinOp`                          | Starts at `291`; an 18-arm exhaustive match with no `_` fallback                                                                                                                                                                                                                                                 |
| The two functions in `parse_assign_after_target`  | **Not dead code**: both are still called at `declarations.rs:719` / `:725` and form a running “reject removed syntax” gate; moreover, `skip_old_function_syntax` (`120-122`, whose body contains only a one-line comment) **consumes no tokens** after being called (`:725-726` directly executes `return None`) |
| Second `BinOp` enum                               | `const_data.rs:234` does exist and has **no `From` conversion** from `ast::BinOp`, forcing files such as `const_eval.rs:19` / `ir_gen.rs:2317` to alias it at import time                                                                                                                                        |
| Re-export block in `lexer/tests/mod.rs`           | Present (38-line file); `:28-38` contains `pub use ... ::*;`, which is the direct reason the empty shells could masquerade as coverage                                                                                                                                                                           |

### Three Corrections Made by This Round of Reverification

The following three points conflict with the intuition that “the first BP ladder is already dead”;
the measured results take precedence:

1. **The two ladders are intertwined, not “one live and one dead.”** `infix_info` (`led.rs:30-88`)
   itself references five constants from the first ladder (`BP_ASSIGN:33`, `BP_CALL:75/77/79/83`,
   `BP_CAST:81`) and eight from the second. Of the first ladder’s 12 constants, **only six truly
   have zero references** (`BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` /
   `BP_FACTOR`, plus `BP_COMPARISON`, which survives only in the comment at `declarations.rs:742`);
   the other six (`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`)
   have approximately **89 references across 8 files** in production code. Therefore, “delete the
   first BP constants” is not a 12-line C6 deletion; it is a cross-file rename in the C5 category.
2. **The second ladder also has a dead member.** `BP_RANGE` (`precedence.rs:22`) has zero production
   references—`..` uses the raw literal `(6, 7)` at `led.rs:40`, and `BP_RANGE` appears only in the
   comment at `led.rs:157-159`.
3. **The “self-contradictory comment” is at `precedence.rs:33`, not `:11`.** `*:11` is
   `pub const BP_COMPARISON: u8 = 5;`. That comment (_“Precedence rules for the Pratt parser”_)
   describes the zero-reference `Precedence` enum.

Additional correction: `bp_right = bp_left + 1` produces **left associativity** (`a - b - c` →
`(a-b)-c`), not right associativity; `(11, 1)` at `led.rs:85` is the only explicit right-greedy
right-operand formulation. There are also collisions beyond the two cross-ladder cases:
`BP_UNARY=8 == BP_CAST=8` (an internal collision in the first ladder, with both live) and
`BP_RANGE=7 == BP_ADD=7` (an internal collision in the second ladder).

### Unverified Items

The main text marks each of the following as “unverified”:

- The specific arm count and binding power of each arm in the `prefix_info` table
  (`pratt/nud.rs:26-75`)
- The exact line boundaries of the eight responsibility sections in `declarations.rs:136-642`
- The complete variant set of `const_data::BinOp` (`const_data.rs:234`)
- Whether `logos` / `regex` already appear in `Cargo.toml` (as transitive dependencies)
- All call sites of `synchronize()` outside `parser_state.rs:170-186`
- Whether this project is a published library (which affects the risk assessment for deleting public
  API surface)

## See Also

### Companion Design Documents from the Same Batch

- [01-routing.md](01-routing.md) — Where to change when adding a feature (functional routing tables
  A/B/C and dependency-direction rules)
- [03-type-unification.md](03-type-unification.md) — Ownership of `ast::Type` convergence and the
  interface boundary with this document
- [04-ssa.md](04-ssa.md) — Another user of the C4/C5 criteria
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Complete inventory of the 8 orphaned test
  trees
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — Definitions of the C5/C6 categories, the
  three-tier criteria, and regression gates

### Main RFC and Related Proposals

- [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md) —
  Governing RFC; four-layer model, glossary, and functional routing tables A/B/C
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — The
  build-time gate in `build.rs`, an existing enforcement mechanism used as the model for this
  document
- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — The origin of
  `ast::Type`

### Code Location Index

- `src/frontend/core/lexer/literals.rs:77-161` / `164-247` / `250-333` / `336-516` — Four radix
  scanners
- `src/frontend/core/lexer/literals.rs:694-827` / `1015-1141` / `1374` — Three escape-decoding
  implementations
- `src/frontend/core/lexer/literals.rs:519-635` — `scan_leading_dot` (zero coverage)
- `src/frontend/core/lexer/mod.rs:104-106` — The lexical layer’s only test declaration
- `src/frontend/core/lexer/tests/mod.rs:15-25` / `28-38` — Submodule declarations and the `pub use`
  re-export block
- `src/frontend/core/parser/pratt/nud.rs:441-446` — Runtime nested `tokenize()` for f-string
  interpolation
- `src/frontend/core/parser/pratt/nud.rs:896-901` / `1041` — The source and use of the magic number
  `12`
- `src/frontend/core/parser/pratt/precedence.rs:6-17` and `22-31` — Two BP ladders
- `src/frontend/core/parser/pratt/precedence.rs:33` / `35-94` / `98-133` — The disconnected comment,
  zero-reference enum, and struct
- `src/frontend/core/parser/pratt/led.rs:30-88` — Handwritten `infix_info` table
- `src/frontend/core/parser/pratt/tests/mod.rs:4-6` — Orphaned wiring caused by the missing
  `precedence_inline` declaration
- `src/frontend/core/parser/statements/declarations.rs:719-727` — Old-syntax rejection gate and its
  “skip without consuming” defect
- `src/frontend/core/types/const_data.rs:234` — The second operator enum parallel to `ast::BinOp`
- `build.rs:19-55` — Build-time error-code gate, an existing enforcement mechanism in this
  repository
- `src/middle/core/tests/bytecode.rs:369-657` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “round-trip every item” criterion in this repository
