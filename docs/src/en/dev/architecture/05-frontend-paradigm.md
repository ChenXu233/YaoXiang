# Frontend Paradigm: Lexical and Syntax

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion tiers, and execution phase sequence are in the RFC-039 main
> text; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This document handles the L2 frontend—**lexical (`src/frontend/core/lexer/`) and syntax
(`src/frontend/core/parser/`)**.

**Problem to solve**: Reduce the change surface for "adding a binary operator" from the current
**6-7 places within L2 + 17 production files downstream** (of which the compiler forces ~8-9 to be
in sync, the rest rely on manual synchronization) down to **1-2 declaration places + 7-8
auto-generated places**.

This document addresses the pain point labeled in row one ("binary operator") of RFC-039 feature
routing table A. The refactoring categories are **C5 frontend paradigm change** as defined in
[07-equivalence-oracle.md](07-equivalence-oracle.md) (criteria: AST snapshot identical + diagnostics
identical + behavior identical—all three used) and **C6 pure deletion** (no equivalence criterion
needed, only confirmation of no references). C6 items are a prerequisite for C5: delete 96 lines of
dead code, eliminate three bare magic numbers, revive 629 lines of never-run lexical tests.

**Scope boundary**: The handling of `ast::Type` (26 variants) and the convergence of three parallel
type representations belong to [03-type-unification.md](03-type-unification.md); this document does
not re-handle them, but only declares the interface boundary in the "Detailed Design" section.

### Why This Is Worth Doing

"Adding an operator" is the most expensive single change in the frontend. This tax has **three
malignant characteristics**:

**First, cost does not decrease with experience.** Someone who has added operators ten times won't
change one fewer place than someone who has done it once—because there is no derivable relationship
between these positions. There is no connection between `(6,7)` at `led.rs:40` and `12` at
`nud.rs:1041`, but they must be consistent with each other, otherwise expression parsing will fail.

**Second, the consequences of a missed edit fall into two categories, of which the worst is not
caught by the compiler.** See the layered table in the "Change surface for adding an operator"
section for details: an exhaustive `match` miss is caught by the compiler, but **a cross-enum miss
is never caught by the compiler**.

**Third, this tax is invisible on the books.** Every commit in
`git log --oneline -- src/frontend/core/parser/pratt/led.rs` almost always touches 3-4 files
simultaneously, but no tool can point out "which place was missed in this commit."

### The Correct Paradigm Already Present in the Same Repository

This project has two **correct solutions for similar problems** that can be directly reused:

- **Error codes**: `build.rs:19-55` uses `tools/code-tables` to parse the registry at **build
  time**, verify uniqueness, and compare item-by-item against the RFC-013 markdown code table—**any
  inconsistency directly `panic!` and refuse to compile**. The cost of adding an error code is
  therefore a constant 1 place + documentation.
- **Standard library interfaces**: `gen_interfaces.rs` does byte-by-byte comparison; `gen_docs.rs`
  marks range drift detection (RFC-039 routing table B, last row).

The common point of these two mechanisms is **turning "must be modified synchronously" from human
memory into executable assertions**. The entire goal of this document can be reduced to one
sentence: turn the correspondence between the operator table, the lexical rule table, and the AST
variant table into executable assertions.

### Why "Only Deleting Dead Code" Is Not Enough

Dead code does exist (`precedence.rs` 96 lines, the empty function `skip_old_function_syntax`, 629
lines of orphan tests), but after deleting them, "adding an operator requires editing a dozen
places" will not decrease by one. Dead code is **stock debt**; the cross-file change surface is
**structural cost**. The two must be handled separately (see "Key Decisions and Rationale" for
handling).

## Current State

Everything below is **verified fact**, with file path and line number for each item. See the
"Verification Record" for the verification method and item-by-item results.

### Lexical Layer: Evidence of Copy-Paste in `literals.rs`

`src/frontend/core/lexer/literals.rs` totals **1568 lines**. This size does not come from
complexity, it comes from three structural repetitions.

**Repetition 1: The four radix scanners are almost line-by-line identical.**

| Scanner               | Line Range            | Lines |
| --------------------- | --------------------- | ----- |
| `scan_hex_number`     | `literals.rs:77-161`  | 85    |
| `scan_octal_number`   | `literals.rs:164-247` | 84    |
| `scan_binary_number`  | `literals.rs:250-333` | 84    |
| `scan_decimal_number` | `literals.rs:336-516` | 181   |

The first three are each about 84 lines, with completely identical structure: accumulate digit →
`has_digits` check → `overflow` flag → `checked_mul(base)` → `checked_add` → `try_into` → report
error. **The only differences are the base constant and the digit extraction expression.** Among
three copies × 84 lines, **about 250 lines are pure redundancy** (`scan_decimal_number` needs
additional handling for decimal points and exponents so it cannot be merged, and is listed
separately).

Evidence sample—the overflow path of `scan_hex_number` (`literals.rs:134-141`):

```rust
if overflow {
    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
        value,
        point(lexer),
    ));

    return Some(lexer.make_token(TokenKind::Error("Hex number too large".to_string())));
}
```

`scan_octal_number:220-227` and `scan_binary_number:306-313` are the same code with different base
and message text.

**Repetition 2: Escape decoding is implemented 3 times.**

| Implementation              | Location                   | Length                                       |
| --------------------------- | -------------------------- | -------------------------------------------- |
| `scan_string`'s `\\` branch | `literals.rs:694-827`      | 134 lines                                    |
| `scan_char`'s `\\` branch   | `literals.rs:1015-1141`    | 127 lines                                    |
| `push_fstring_escape`       | `literals.rs:1374` onwards | Already extracted into a standalone function |

The first two are nearly line-by-line copies of 127-134 lines (`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` →
push identical, `\x` / `\u{...}` logic identical, `c =>` error fallback identical), **about 250
lines are written twice**.

It is worth pointing out separately: **the f-string version has already been extracted into a
standalone function** (`push_fstring_escape:1374`). This shows that **someone has already noticed
this duplication and fixed it locally, but never went back to converge the other two**. This is more
worth recording than "all three never noticed"—it proves the problem is solvable, it just lacks a
mandatory mechanism.

**Repetition 3: Multi-line string scanning comes in two copies.** `scan_multi_line_string:869` and
`scan_fstring_multi_line:1497` are again two similar implementations.

**Abnormal layout residue.** `literals.rs:777-787` and `1098-1108`, 11 lines each, take the form of
**inserting 3 consecutive blank lines between each field of a struct literal**. This is residue from
automated rewriting. Other positions in the same function (`1111-1115`) have only a single blank
line between fields, indicating that formatting was interrupted.

> Such residue is itself harmless, but it is a **diagnostic signal**: the same code was rewritten
> twice by different tools, meaning that historically someone made a local patch here without
> pulling the full context together.

**Conclusion**: About 500 lines in `literals.rs` (250 lines of radix scanner redundancy + 250 lines
of duplicate escape handling) is **mechanical repetition**, not complexity.

**f-string interpolation triggers nested compilation.** This is **runtime repeated compilation**,
not duplicate implementation, so it cannot be solved by merging functions.

`src/frontend/core/parser/pratt/nud.rs:441-446`:

```rust
let tokens_result = crate::frontend::core::lexer::tokenize(expr_str_trimmed);
match tokens_result {
    Ok(tokens) => {
        let mut parser = crate::frontend::core::parser::ParserState::new(&tokens);
        if let Some(expr) =
            parser.parse_expression(crate::frontend::core::parser::pratt::BP_LOWEST)
```

**The parser layer runs a complete `tokenize()` for each f-string interpolation at runtime, and
creates a new `ParserState`.**

Consequence: An f-string with N interpolations **triggers N+1 full lexical analyses** (N
interpolations + 1 outer). At the same time, each interpolation is an **independent parsing
context**, therefore:

- Diagnostic spans inside the interpolation are relative to the interpolation text, requiring extra
  work to remap to source locations;
- The interpolation cannot access any outer parser state (currently not needed, but structurally
  closes off future possibilities, such as referencing outer implicit variables inside an
  interpolation);
- The positions where lexical and syntax errors are produced are split into two stages.

This is a **dual defect in performance and structure**, and must be addressed (see "Target Design ·
Lexical").

### Syntax Layer: Two BP Ladders, Bare Magic Numbers, Hand-Written Dispatch Tables

#### The Two Ladders Are Not "One Alive, One Dead"—They Are Interwoven

`src/frontend/core/parser/pratt/precedence.rs` contains **two binding power ladders**
simultaneously, with widespread number overlap.

**First set (`precedence.rs:6-17`)**: `BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` /
`BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` /
`BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`.

**Second set (`precedence.rs:22-31`)**: `BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` /
`BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`.

**Tested conclusion: Both sets are alive in production code, and they are already interwoven within
`infix_info`.**

- `infix_info` (`led.rs:30-88`) references **5 constants from the first set**: `BP_ASSIGN` (`:33`),
  `BP_CALL` (`:75` / `:77` / `:79` / `:83`), `BP_CAST` (`:81`);
- References **8 constants from the second set**: `BP_OR` (`:42`), `BP_AND` (`:44`), `BP_EQ`
  (`:47`), `BP_SHIFT` (`:51` / `:54`), `BP_BIT` (`:58`), `BP_CMP` (`:62` / `:65`), `BP_ADD` (`:68`),
  `BP_MUL` (`:72`);
- **The only dead member in the second set is `BP_RANGE`**—`..` at `led.rs:40` uses the bare literal
  `(6, 7)` instead of `BP_RANGE`; `BP_RANGE` only appears in production code in the comment at
  `led.rs:157-159`.

Survival status of the 12 constants in the first set (`grep` whole-repository scan results):

| Status              | Constant                                                                     | Production Reference Points                                                                                                                                                                     |
| ------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Dead (6)**        | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | Zero production hits. Only appears in `precedence.rs` itself and the orphan test `precedence_inline.rs`                                                                                         |
| Dead (comment only) | `BP_COMPARISON`                                                              | Zero production hits; only mentioned in a comment at `declarations.rs:742`                                                                                                                      |
| **Alive (6)**       | `BP_LOWEST`                                                                  | `parser/mod.rs:80`, `statements/control_flow.rs` 9 places, `statements/declarations.rs` 6 places, `pratt/led.rs:229`/`:321`/`:327`/`:415`, `pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523`, etc. |
|                     | `BP_ASSIGN`                                                                  | `led.rs:33`, `led.rs:99`, `statements/declarations.rs:18` (import), `:743` (`BP_ASSIGN + 1`)                                                                                                    |
|                     | `BP_UNARY`                                                                   | `nud.rs:32`, `:35`, `:83`, `:101`, `:223`, `:243`                                                                                                                                               |
|                     | `BP_CALL`                                                                    | `led.rs:75`, `:77`, `:79`, `:83`                                                                                                                                                                |
|                     | `BP_CAST`                                                                    | `led.rs:81`                                                                                                                                                                                     |
|                     | `BP_HIGHEST`                                                                 | `nud.rs:38-69` about 20 places (one for each literal and keyword prefix)                                                                                                                        |

**Total: The 6 alive constants have about 89 production references, across 8 files.**

> This corrects an easy-to-reach wrong conclusion: "delete the first ladder" is **not** a 12-line
> deletion, but a rename and merge across 8 production files, 89 references. Only 6 constants in the
> first set have truly zero references (plus `BP_COMPARISON` which only exists in a comment). The
> second set is also incomplete—`BP_RANGE` is dead.

**The truly zero-reference dead code is those two types**: `Precedence` enum (`precedence.rs:35-94`,
60 lines) and `PrecedenceContext` struct (`precedence.rs:98-133`, 36 lines), totaling **96 lines**.
Verification result: these two types have zero use in production code—whole-repository `grep` for
`PrecedenceContext` and `Precedence::`, excluding `precedence.rs` itself, all hits fall on
`src/frontend/core/parser/pratt/tests/precedence_inline.rs`.

**There is a detail that must be written into the documentation**: the only file that references the
dead code, **is itself an orphan**. `pratt/tests/mod.rs:4-6` only declares
`mod led; mod nud; mod precedence;`, **it does not declare `precedence_inline`**.

> That is: the reason 96 lines of dead code "appear to have test coverage" is that they are being
> tested by a 95-line / 6-test file, and that file has never been compiled (see "Dead Code and Test
> Status"). **Deleting the dead code must also handle this orphan test file**, otherwise deletion
> will break compilation.

In addition, `precedence.rs:33` has a self-contradictory comment as corroborating evidence:
_"Precedence rules for the Pratt parser"_—this comment describes exactly that zero-reference
`Precedence` enum, whose ladder is not the same set as the actually effective one. The enum and the
implementation have become disconnected.

#### Value Collisions

The overlap regions of the two ladders have nearly digit-by-digit corresponding numbers, so
**cross-set collisions appear in pairs**:

| Conflict                    | Value    | Use point A (First set)        | Use point B (Second set)       |
| --------------------------- | -------- | ------------------------------ | ------------------------------ |
| Assignment vs logical or    | `1 == 1` | `BP_ASSIGN` (`led.rs:33`)      | `BP_OR` (`led.rs:42`)          |
| Type cast vs multiplication | `8 == 8` | `BP_CAST` (`led.rs:81`)        | `BP_MUL` (`led.rs:72`)         |
| Shift vs term               | `6 == 6` | `BP_TERM` (dead)               | `BP_SHIFT` (`led.rs:51`/`:54`) |
| Bitwise vs comparison       | `5 == 5` | `BP_COMPARISON` (comment only) | `BP_BIT` (`led.rs:58`)         |

**The first set also has an internal self-collision**: `BP_UNARY=8` and `BP_CAST=8`
(`precedence.rs:14` and `:15`)—**two different semantics in the same set share a number**, and both
are alive (used for `nud.rs`'s prefix binding and `led.rs:81`'s cast respectively). The second set
has an internal self-collision as well: `BP_RANGE=7` and `BP_ADD=7`.

**The reason it hasn't blown up so far is an unwritten convention**: almost all infixes use
`bp_right = bp_left + 1`, so the right operand only accepts **strictly higher** binding power
operators—this actually produces **left-associativity** (`a - b - c` → `(a-b)-c`). So the only
actually-effective comparison, "compare `bp_left`", will not fail because two operators have equal
bp. `led.rs:85`'s `FatArrow` uses `(11, 1)`, where `bp_right` is lower than `bp_left`, is the only
explicit "right operand greedily swallows" form (lambda body greedily extends to the right).

This is an **accidental correctness**: it depends on the fact that "`infix_info` returns a unique
`bp_left`, and `parse_expression_internal:100` only compares `bp_left`". Any place changed to use
`bp_left == bp_right` to distinguish associativity will immediately surface the conflicts.

#### Three Bare Magic Numbers

| Magic Number | Location                             | Problem                                                                                                                |
| ------------ | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| `(6, 7)`     | `led.rs:40` `TokenKind::DotDot`      | Literal binding power pair, no name; and **does not use the same set's `BP_RANGE`**, making `BP_RANGE` a dead constant |
| `(11, 1)`    | `led.rs:85` `TokenKind::FatArrow`    | 11 **exceeds the upper limit of both ladders** (first set `BP_HIGHEST=10`)                                             |
| `12`         | `nud.rs:1041` `parse_expression(12)` | **Greater than `BP_HIGHEST=10`**, used for RFC-010b pattern detection                                                  |

The comment at `nud.rs:896-901` frankly admits the origin of this magic number: _"Lambda binding
power is 11, so we use 12 to stop before =>"_, _"parse_expression(12) will stop `ok` as a bare Var,
`(` as an unexpected token"_. That is, **this 12 was experimentally found**, not derived.

#### Dispatch is Hand-Written `match`

**Dispatch mechanism**: all hand-written `match` returning function pointers, **no macros, no
tables, no generators**.

- `parse_prefix` (`nud.rs:14-19`) takes the `prefix_info()` closure;
- `parse_expression_internal` (`pratt/mod.rs:79-113`) loops to take the
  `(bp_left, bp_right, parser_fn)` triple from `infix_info()`;
- Two hand-written tables: `infix_info` (`led.rs:30-88`), `prefix_info` (`nud.rs:26-75`).

#### `parse_assign_after_target`: A Misleadingly-Named 507-Line Total Dispatcher

`src/frontend/core/parser/statements/declarations.rs:136`'s `parse_assign_after_target`, function
body through `:642` (**507 lines**, file total 1015 lines).

The name says "assign after target", but it is actually **a total dispatcher for declaration
forms**, containing 8 independent responsibility sections: old syntax detection / annotation
disambiguation with three nested lookaheads / declaration legality / semantic side effects / type
body parsing fallback / RFC-010 lambda signature parameter reconstruction / MetaType definition /
plain initialization.

The causes of bloat are three layers stacked:

**Cause 1: Dead language feature, about 120 lines.**

`is_old_function_syntax` (`declarations.rs:82-117`), `skip_old_function_syntax`
(`declarations.rs:120-122`), and the supporting lookahead at `:145-167`.

What needs precise explanation is: **these two functions are not unreferenced**. They are called at
`declarations.rs:719` and `:725` by `parse_identifier_stmt`, with the path "detect `identifier(`
followed by type parameters and `->` → report 'old syntax deprecated' → skip → return None".

So the accurate description is: **this is a still-running gatekeeper that "rejects removed
syntax"**, costing about 120 lines to do a parenthesis-matching scan for every statement starting
with `identifier(`. Among them:

- `skip_old_function_syntax` (`:120-122`) **the function body has only one comment line**
  `// 旧语法已移除，此函数不再需要` (Old syntax has been removed, this function is no longer
  needed), which is a **pure empty function**;
- More notably, after calling it (`:725-726`) it directly `return None`, **consuming no tokens**.
  That is, this "skip" **actually doesn't skip anything**—subsequent tokens remain in the stream,
  and can only be caught by the caller's error recovery. This is a name-mismatched-reality
  interface.

**Cause 2: Lookahead logic not abstracted.** "Skip paired parentheses" is **hand-written 4 times**
in the same file:

| Location                           | Line Range                |
| ---------------------------------- | ------------------------- |
| Inside `is_old_function_syntax`    | `declarations.rs:96-105`  |
| Inside `parse_assign_after_target` | `declarations.rs:147-156` |
| Same                               | `declarations.rs:191-200` |
| Same                               | `declarations.rs:224-233` |

Each is about 12 lines, with consistent structure: `paren_depth = 1` →
`while paren_depth > 0 && !at_end()` → on `LParen` add one, on `RParen` subtract one → `bump()`.
**The four copies share no code.**

**Cause 3: Type-layer logic leaked into parser.** See RFC-039 routing table C for details:
`declarations.rs:27-80` calls type-layer `is_type_param_annotation` / `name_used_as_type`;
`declarations.rs:498-509` hardcodes the string `"Terminates"` and calls
`typecheck::operator_interfaces::spec()`. **This part belongs to 03-type-unification.md /
02-stage-contract.md, this document does not re-handle it, but only marks the boundary in the
"Detailed Design" section.**

#### AST Enum Overview

`src/frontend/core/parser/ast.rs`:

| Enum       | Location         | Variant Count          |
| ---------- | ---------------- | ---------------------- |
| `Expr`     | `ast.rs:16-163`  | 22 (including `Error`) |
| `BinOp`    | `ast.rs:191-213` | 20                     |
| `UnOp`     | `ast.rs:217-223` | 4                      |
| `StmtKind` | `ast.rs:234`     | 9 (including `Error`)  |
| `Type`     | `ast.rs:427-541` | 26                     |
| `Pattern`  | `ast.rs:794-814` | 8                      |

`Expr::Error` has been verified to exist (`pratt/mod.rs:72`'s span extraction handles it,
`ast.rs:1048` also); `StmtKind::Error` is constructed at `parser/mod.rs:50`. **These two error
placeholders are the cornerstone of backward-compatible design and must be preserved** (see
"Detailed Design · Backward Compatibility").

### Change Surface for Adding an Operator

**The downstream change surface is much larger than expected.** `BinOp::` has **534 hits / 47
files** in the entire repository, of which **17 are production files**. More critically, these files
consume **two unrelated `BinOp` enums**:

| Enum                | Definition                | Variant Count                             | Production Consumers (in parentheses: `BinOp::` hit count in that file)                                                                                                                                                                  |
| ------------------- | ------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ast::BinOp`        | `parser/ast.rs:191-213`   | 20                                        | `led.rs`(22) `nud.rs`(1) `checker.rs`(8) `inference/expressions.rs`(25) `inference/statements.rs`(5) `operator_interfaces.rs`(5) `ir_gen.rs`(23) `formatter/handlers/expr.rs`(25) `lsp/handlers/inlay_hint.rs`(4) `spawn/analysis.rs`(3) |
| `const_data::BinOp` | `types/const_data.rs:234` | Verified to exist (`Ne` instead of `Neq`) | `const_data.rs`(36) `const_eval.rs`(64) `evaluator.rs`(13) `ownership.rs`(13) `termination.rs`(16) `proof/smt/translate.rs`(13) `proof/dep_graph.rs`(2)                                                                                  |

Verified: `src/backends/` has **zero hits** for `BinOp` and `ast::`—the interpreter consumes IR /
bytecode, never seeing AST. So all 17 production files fall on the L2 → L3 link.

**There is no `From`/`TryFrom` between the two enums**—whole-repository `grep` for
`impl From<...BinOp`, `-> BinOp`, `BinOp as` shows no conversion implementation. The cost is that
**every file that touches both must alias at the import site**: `const_eval.rs:19`
(`BinOp as AstBinOp`), `ir_gen.rs:2317` (`use ast::BinOp as B`), and `CEBinOp` / `ConstBinOp` in
test files.

> **This is the most important finding in this document.** When `ast::BinOp` adds a variant, **the
> compiler cannot remind you to change `const_data::BinOp` at all**, because they are two
> independent types, each of which can compile independently. The consequence of missing an edit is:
> the constant folding path doesn't know the new operator, but the type checking and IR generation
> paths do—**the same operator behaves differently in two subsystems, with no diagnostic**.

> This is an order of magnitude more serious than "missing one `match` arm". The latter is caught by
> exhaustive matching, the former cannot be caught.

**On "whether a missed edit goes unnoticed", a precise distinction is needed**:

| Missed Edit Type                                    | Caught by Compiler? | Verified Evidence                                                                                                                                                    |
| --------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Exhaustive `match` arm missing                      | **Yes**             | `const_data.rs:291-317`'s `impl Display for BinOp` is an 18-arm exhaustive match, no `_` fallback                                                                    |
| `matches!` macro enum item missing                  | **No**              | `const_data.rs:263-288`'s `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` are all `matches!` partial enumerations; new variant → silently returns `false` |
| `match` with `_` fallback arm missing               | **No**              | `ownership.rs:687`'s `_ => {}` silently ignores new variants                                                                                                         |
| **Cross `ast::BinOp` / `const_data::BinOp` missed** | **Never**           | No conversion, no common parent type, compiler has no way to relate them                                                                                             |

So the accurate statement is not "11 places missed, no one knows", but: **the part with exhaustive
matches can be caught by the compiler; what is truly silent is `matches!` macros, `_` fallbacks, and
cross-enum**.

### Dead Code and Test Status

This is the **most needing immediate handling** item in this document.

#### Dead Code

- `precedence.rs:35-94` `Precedence` enum (60 lines) and `precedence.rs:98-133` `PrecedenceContext`
  struct (36 lines): zero production use, the only external referrer is the orphan file
  `pratt/tests/precedence_inline.rs` (95 lines / 6 tests, not declared by `pratt/tests/mod.rs:4-6`).
- `skip_old_function_syntax` (`declarations.rs:120-122`): pure empty function, the call site at
  `:725` then directly `return None`, not consuming tokens.
- `BP_RANGE` (`precedence.rs:22`): zero production use (only exists in the `led.rs:157-159`
  comment).

#### The Lexical Layer Has Only One Test File Running

**`lexer/mod.rs:104-106` the only test declaration is:**

```rust
#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
```

`grep 'mod tests'` in `src/frontend/core/lexer/` only hits this one place across the entire
directory. **`lexer/` has never declared `mod tests;`**.

The entire `src/frontend/core/lexer/tests/` subtree therefore **has never participated in
compilation**. That directory actually has **14 files** (including `mod.rs`):

| File                                                                              | Lines          | Status                                 |
| --------------------------------------------------------------------------------- | -------------- | -------------------------------------- |
| `mod.rs`                                                                          | 38             | Exists, declares 11 submodules         |
| `literals.rs`                                                                     | 222            | Real tests, **orphan**                 |
| `rfc010_lexer.rs`                                                                 | 167            | Real tests, **orphan**                 |
| `fstring.rs`                                                                      | 90             | Real tests, **wired up via `#[path]`** |
| `lexer_mod.rs`                                                                    | 89             | Real tests, **doubly orphan**          |
| `rfc004_lexer.rs`                                                                 | 81             | Real tests, **orphan**                 |
| `symbols.rs`                                                                      | 70             | Real tests, **doubly orphan**          |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 1-3 lines each | 7 empty shells                         |
| `debug_lexer.rs`                                                                  | 1              | Empty shell                            |

**Real but never-run tests: 629 lines / 5 files** (222+167+89+81+70). The lexical layer actually
running is only the f-string one file.

**A wiring trap that must be made clear: `tests/mod.rs` exists, but it itself is not fully wired.**

`lexer/tests/mod.rs:15-25` declares 11 submodules (`basic` / `literals` / `operators` / `delimiters`
/ `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer` / `fstring`),
`:28-38` then `pub use` all of them as "backward-compatible re-exports".

**But it does not declare `lexer_mod` and `symbols`.** Therefore:

> **Just adding one line `mod tests;` is not enough.** Reviving `lexer/tests/` requires **two
> changes**: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` to
> `tests/mod.rs`. Otherwise these 159 lines still won't run.

**The trap of the 7 empty shells**: `tests/mod.rs:28-38`'s `pub use basic::*;` such statements,
**referencing a zero-assertion module produces no compile error or warning**. After adding
`mod tests;`, CI will show "all tests pass", while in reality `basic` / `comments` / `delimiters` /
`errors` / `keywords` / `operators` / `debug_lexer`—all seven modules—**have no assertions**. **This
creates the illusion of "lexical layer is covered"**, more dangerous than not wiring at all.

**Uncovered and highest-risk paths** (will be immediately exposed once wired):

| Path                                                                       | Location                                                                      | Risk                                |
| -------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------- |
| Overflow paths of the four radix scanners                                  | `literals.rs:134-141` / `220-227` / `306-313` + corresponding decimal section | Integer overflow is a silent error  |
| "Continue consuming but not reporting" branch after `checked_mul` overflow | 4 places, isomorphic                                                          | May cause token stream misalignment |
| `scan_leading_dot` **entire function**                                     | `literals.rs:519-635` (117 lines)                                             | Zero coverage                       |
| `\x` / `\u` illegal escape paths                                           | `literals.rs:730-749` / `1051-1070`                                           | Error paths are hard to test        |

**Parser-side wiring is normal**: `parser/mod.rs:8-9`, `pratt/mod.rs:8-9`, `statements/mod.rs:11-12`
all have correct `mod tests;`. `pratt/tests/mod.rs:4-6` declares `led` (335 lines), `nud` (269
lines), `precedence` (13 lines).

**But the parser-side coverage quality is also problematic**:

- `pratt/tests/precedence.rs` (**13 lines**) the only assertion is
  `assert!(bp_lowest < bp_highest)`, **almost an empty test**—it cannot even discover the four
  cross-set collisions listed above.
- `pratt/tests/precedence_inline.rs` (**95 lines / 6 tests**) is not declared by
  `pratt/tests/mod.rs`, **never run**—and what it tests is **all that zero-reference `Precedence`
  enum + `PrecedenceContext`**.
- **The actually-used two ladders (`BP_OR`…`BP_MUL` and
  `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`) have no direct unit tests.**

## Target Design

### Lexical: Declarative

#### Evaluation of Three Options

| Option                                           | Cost of Adding a Token      | Can Eliminate the Three Repetitions in the Lexical Layer | Can Solve f-string Nested Compilation        | Dependencies Introduced                                              |
| ------------------------------------------------ | --------------------------- | -------------------------------------------------------- | -------------------------------------------- | -------------------------------------------------------------------- |
| **A. `logos` generates DFA**                     | 1 place (token declaration) | **Yes** (escape / number each one copy)                  | No (manual handling of interpolation needed) | New `logos`                                                          |
| **B. `regex` hand-written DFA**                  | 1-2 places                  | Partially                                                | No                                           | New `regex` (whether already a transitive dependency **unverified**) |
| **C. Keep hand-written, force merge duplicates** | 1 place                     | **Yes** (manually extract common functions)              | **Yes** (requires explicit design)           | None                                                                 |

**Position: Adopt C as baseline, A as conditional backup, B not adopted.**

Reasons:

1. **C can eliminate all ~500 lines of repetition in the lexical layer with zero new dependencies**,
   and is fully within controllable scope. `scan_string` / `scan_char` / `push_fstring_escape`'s
   three escape decoding merge into a single `decode_escape(lexer, out) -> bool`, and the four radix
   scanners merge into a single `scan_radix_number(lexer, base, digits_pred)`. This is a pure
   mechanical refactoring, with the criterion being C5's "AST snapshot identical + diagnostics
   identical".
2. **A's (`logos`) capability is real**—it can converge escape decoding and number scanning into one
   copy each. **But it cannot solve f-string nested compilation**, which is the most important
   defect in the lexical layer. At the same time, introducing a new dependency will change the
   `Literals` error type (`LexError`'s `InvalidEscape` / `InvalidNumber` variants need to be
   remapped), which will **directly violate C5's "diagnostics identical" criterion**, unless an
   additional adaptation layer is written—that would be yet another manual synchronization point.
3. **B (`regex` hand-written DFA) is not adopted**: it requires hand-writing state machines, with
   the same workload as A but far less controllability. Since choosing between A and B, A must be
   chosen logically; since this document advocates C first, there's no reason to choose B.

> **Design judgment (not verified fact)**: A may be a better form in the **long term** (a
> declarative token table itself is a kind of "change one place" enforcement). But its migration
> cost and conflict with the "diagnostics identical" criterion make it unsuitable as the content of
> this C5. If A is to be adopted later, it should be a separate proposal, using this document's test
> revival results as a safety net.

#### f-string Nested Compilation Must Be Explicitly Handled

Whether A or C is chosen, nested compilation must be solved. **Position: Cancel the runtime nested
`tokenize()`, change to one-shot production at the lexical stage.**

Specific approach: when `scan_string` encounters the `f"` prefix, **in the same lexical scan**
record the interpolation content as `FStringSegment::RawText(String)`, while **not** recursively
calling `tokenize()`. The actual tokenize + parse of the interpolation is deferred until the parser
first needs that f-string node, and:

- **Done in one pass** (for N interpolations, only 1 tokenize of the interpolation segment, cached
  as `Vec<Vec<Token>>`), reducing N+1 times to 1 time;
- **Span mapping is determined at the lexical stage** (recording the absolute offset of each
  interpolation in the source file), no longer relying on the relative semantics of `Point`.

This change is **independent of paradigm choice** and should be executed even without A.

### Syntax: Complete Grammar-Driven (LALRPOP)

> **Decision (2026-10-03, ChenXu233)**: **Adopt A (LALRPOP complete grammar-driven), no half-baked
> table-ization.** A's equivalence risk (LALRPOP's error recovery model differs from
> `synchronize()`, whether experimentally-found parsing behavior can be reproduced bit-by-bit) has
> been resolved by the **double-parser diff** scheme below; table-ization is an insufficient
> intermediate state.

#### Option Evaluation

| Option                   | Change Surface for Adding a Binary Operator | Can Eliminate BP Conflicts                                                                          | Can Preserve `synchronize()` and Error Placeholders                      | Conclusion                             |
| ------------------------ | ------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------- |
| **A. LALRPOP**           | 1 grammar place + 1 action place            | Yes (LR table auto-computes precedence, conflicts surface as grammar conflict errors at build time) | Requires rewriting, but **error productions can be explicitly declared** | **Adopted**                            |
| **B. tree-sitter**       | 1 grammar.js place                          | Yes                                                                                                 | Not applicable (produces CST, not domain AST)                            | **Rejected**                           |
| C. Keep Pratt table-ized | 2-3 places                                  | Yes                                                                                                 | Yes                                                                      | **Rejected** (insufficient, see below) |

**B rejection reason** (unchanged): tree-sitter produces a generic syntax tree, this project's
`Expr` / `StmtKind` are **domain ASTs with semantic information** (`Expr::Cast` has a target type,
`Expr::FString` has a segment list, `Pattern::Struct` has field names). CST → domain AST still needs
a complete mapping layer, **tree-sitter cannot skip it**. Equivalent to adding a mapping layer of
the same size as the existing 1326 lines of `nud.rs`.

**C rejection reason**: table-ization solves **synchronization** (change one place, don't miss), but
doesn't solve **maintainability**—the 22 variants of `Expr` still need 22 hand-written construction
actions, the merge of the two BP ladders still needs to change about 89 references, and structural
problems like `Expr::Lambda { body: empty Block }` being used as a parameter list (`nud.rs:505-514`)
are completely untouched. **It is an unnecessary intermediate station between "completely
hand-written" and "complete grammar".**

#### Original Concerns About Rejecting A, and How They Are Resolved

The original concern about rejecting A is **equivalence**, not lack of capability:

- `ParserState::synchronize()` (`parser_state.rs:170-186`) and the "keep parsing as much as
  possible" semantics formed by the `Expr::Error` / `StmtKind::Error` placeholders, LALRPOP's error
  recovery is a different model (insert / delete tokens), **the produced diagnostic set must
  differ**
- The `bp_left < min_bp` exit condition at `pratt/mod.rs:100-102`, the `parse_expression(12)`
  pattern detection at `nud.rs:1041`, are all **behavior experimentally found on specific inputs**,
  whether they can be reproduced bit-by-bit requires item-by-item proof

**Resolution: double-parse diff.** Not relying on "the criteria should be the same" argument, but
**running**:

| Step | Content                                                                                                                                                        | What It Proves                                                          |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| 1    | Build LALRPOP grammar + action code, **coexisting completely with existing Pratt**, don't change any existing code                                             | ——                                                                      |
| 2    | 293 corpora + `src/std/tests` in full, **run each through both parsers**, after normalization per [07](07-equivalence-oracle.md) rules, **compare bit by bit** | **AST equivalence of legal programs is empirically proven, not argued** |
| 3    | Cut over: `parser/mod.rs`'s `parse()` switches to call LALRPOP; Pratt is kept as `parse_legacy()`                                                              | ——                                                                      |
| 4    | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + `precedence.rs`'s two ladders + about 89 references                                                             | ——                                                                      |
| 5    | Error recovery: `synchronize()` and `Expr::Error` / `StmtKind::Error` rebuilt in LALRPOP with **explicit error productions**                                   | See below                                                               |

**Step 2 is the core of the entire plan.** If any AST of the 293 corpora is not equivalent, it is
discovered before cutting over—this is much stronger than "manually arguing item-by-item whether 89
references can be reproduced", and the cost is acceptable (one offline run).

#### Handling of Error Recovery: Fully Preserve C5, No Relaxation

**Error diagnostics must be identical item-by-item, this is a hard requirement, not negotiable.**
The handling is **explicitly model the existing behavior in the grammar**:

| Existing Behavior                                                                                                                        | LALRPOP Counterpart                                                                                                                                                                              |
| ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `ParserState::synchronize()` (`parser_state.rs:170-186`): after statement parsing failure, skip tokens forward to synchronization points | Explicitly declare "error → skip to synchronization point set → continue" productions in the grammar, **the skip set must be identical token-by-token with the existing implementation**         |
| `Expr::Error` / `StmtKind::Error` placeholders: enable parser to continue constructing "partial AST" after an error                      | Explicitly declare error node productions in the grammar, **downstream `match` branch positions unchanged** (if downstream puts `Error` in a wildcard branch, errors will be silently swallowed) |
| Diagnostic code + span                                                                                                                   | **Identical item-by-item. Wording unchanged.**                                                                                                                                                   |

**This is not "should be doable", it's mandatory.** LALRPOP allows writing explicit error
productions; writing the synchronization point set of `synchronize()` into the grammar is
sufficient. If stage 3a/3b empirical testing proves that existing diagnostics cannot be reproduced
under LALRPOP, **the correct action is to report this finding truthfully and re-evaluate the plan
(including "keep Pratt"), not to modify the criteria**.

Migration landing points and handling:

| Item                                                              | Content                                                                                                                                                                                                                                                     |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Grammar file                                                      | `src/frontend/core/parser/grammar/yaoxiang.lalrpop` (new directory, see [01](01-routing.md) target structure)                                                                                                                                               |
| Action code                                                       | `grammar/actions.rs`—move the construction of `Expr` / `StmtKind` over from `nud.rs` / `led.rs`, **one function per variant**                                                                                                                               |
| Error productions                                                 | Explicitly declared, in conjunction with `Expr::Error` / `StmtKind::Error` placeholders to preserve the "keep parsing as much as possible" semantics                                                                                                        |
| BP ladder merge                                                   | Both ladders are deleted with the grammar; the 6 alive constants `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`, with their **about 89 production references**, are taken over by the precedence declarations in the grammar |
| Bare magic numbers                                                | `(6,7)` / `(11,1)` / `12` disappear together with Pratt—the LR table auto-computes precedence                                                                                                                                                               |
| `is_old_function_syntax` (`declarations.rs:82-117`, 36 lines)     | **Naturally deleted**—after grammar-driven, `f(Int) -> Int = ...` cannot match any production, dedicated "detection of removed syntax" is an anti-pattern                                                                                                   |
| `Expr::Lambda { body: empty Block }` borrowing (`nud.rs:505-514`) | **Naturally eliminated**—parameter list in the grammar is a parameter list, no need to borrow AST nodes                                                                                                                                                     |
| `synchronize()` (`parser_state.rs:170-186`)                       | Keep API, change to being driven by error productions at call sites                                                                                                                                                                                         |

**Net effect**: Delete `nud.rs`(1326) + `led.rs`(495) + `precedence.rs`'s two ladders + about 120
lines of dead old syntax detection, add grammar file and action code. **The change surface for
"adding a binary operator" goes from a dozen places to 1 grammar place + 1 action place**, and BP
conflicts error at **grammar build time**, no longer "accidentally correct" like now.

#### Prerequisites

1. **The 629 lines / 55 tests in `lexer/tests/` must be revived first** (P1 in
   [09](09-execution-wbs.md)). Without a frontend test safety net, replacing the entire parser
   cannot satisfy 07 "criteria must precede refactoring"
2. **AST normalization snapshots must be checked in first** (P2's second layer), otherwise step 2
   has nothing to compare
3. **f-string nested compilation elimination** (`literals.rs` refactoring) **is independent of this
   item**, should be executed even without grammar driving

### Handling of `parse_assign_after_target`

**Position: Split into 8 named functions, function bodies only do dispatch.**

A 507-line, 8-responsibility function requires searching within 500 lines for any local
modification. The target form after splitting:

| Split-out Function             | Responsibility                                      | Approximate Lines | Belongs To                                                                    |
| ------------------------------ | --------------------------------------------------- | ----------------- | ----------------------------------------------------------------------------- |
| `probe_old_function_syntax`    | Old syntax detection                                | 36                | **Deleted by this document**                                                  |
| `resolve_annotation_ambiguity` | Annotation disambiguation (three nested lookaheads) | TBD               | This document (greatly shortened after extracting independent lookahead tool) |
| `check_declaration_legality`   | Declaration legality                                | TBD               | This document                                                                 |
| `apply_semantic_side_effects`  | Semantic side effects                               | TBD               | **03-type-unification.md** (including `498-509`'s `"Terminates"` hardcode)    |
| `parse_type_body`              | Type body parsing fallback                          | TBD               | This document                                                                 |
| `rebuild_lambda_params`        | RFC-010 lambda signature parameter reconstruction   | TBD               | This document                                                                 |
| `parse_meta_type_def`          | MetaType definition                                 | TBD               | This document                                                                 |
| `parse_plain_init`             | Plain initialization                                | TBD               | This document                                                                 |

Companion: converge the 4 copies of "skip paired parentheses" (`96-105` / `147-156` / `191-200` /
`224-233`) into a single method `skip_balanced_parens()` on `ParserState`, change the 4 places to
call it.

**Net effect (design judgment)**: 507 lines → 8 single-responsibility functions + 1 lookahead tool;
lookahead code from 48 lines down to about 12 lines; after type-layer logic is moved out (about 80
lines), the part actually belonging to this document is about 300 lines.

### Dead Code Cleanup (C6, No Equivalence Criterion Needed)

According to C6 definition in 07-equivalence-oracle.md ("pure deletion, no equivalence criterion
needed, only confirmation of no references"):

| Cleanup Item                                | Location                                   | Lines | Prerequisites                                                                                           |
| ------------------------------------------- | ------------------------------------------ | ----- | ------------------------------------------------------------------------------------------------------- |
| `Precedence` enum                           | `precedence.rs:35-94`                      | 60    | **Must handle simultaneously** `pratt/tests/precedence_inline.rs` (the only referrer, itself an orphan) |
| `PrecedenceContext` struct                  | `precedence.rs:98-133`                     | 36    | Same as above                                                                                           |
| 6 zero-reference constants in the first set | `precedence.rs:8-13`, `:11` (comment only) | 6     | Zero production references, delete directly                                                             |
| `BP_RANGE`                                  | `precedence.rs:22`                         | 1     | Zero production references (only in `led.rs:157-159` comment), first change `..` to use named constant  |
| `skip_old_function_syntax`                  | `declarations.rs:120-122`                  | 3     | Empty function; `declarations.rs:725`'s call site deleted together                                      |
| `is_old_function_syntax`                    | `declarations.rs:82-117`                   | 36    | See below                                                                                               |
| `declarations.rs:145-167`                   | Same file                                  | 23    | Same as above                                                                                           |
| Bare magic number `(6,7)`                   | `led.rs:40`                                | —     | Promote to `BP_RANGE_L` / `BP_RANGE_R` named constants                                                  |
| Bare magic number `(11,1)`                  | `led.rs:85`                                | —     | Promote to `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                |
| Bare magic number `12`                      | `nud.rs:1041`                              | —     | Promote to `BP_PATTERN_PROBE`, and **record in comments that it was experimentally found**              |

> **Note**: The alive constants in the first set `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL`
> / `BP_CAST` / `BP_HIGHEST` **are not in the pure-deletion scope of this table**—they have about 89
> references in production code, and their merge belongs to the "Syntax: grammar-driven · option C"
> above (C5 category), not C6 deletion.

**Handling choice for `is_old_function_syntax` (design judgment)**:

It is not unreferenced dead code, but a **still-running "rejects removed syntax" gatekeeper**
(`declarations.rs:719`). Two options:

- **Option 1 (aggressive)**: Delete the entire section. For removed syntax, the "rejection" should
  be left to **the syntax definition itself**—after grammar-driven, `f(Int) -> Int = ...` cannot
  match any production at all, naturally reporting an error, no need for dedicated detection. **This
  is the dividend of paradigm change**: about 120 lines naturally disappear after grammar-driven.
- **Option 2 (conservative)**: Keep the gatekeeper but delete the empty function
  `skip_old_function_syntax`, and change "return None without consuming tokens" to actually skip.

**Position: Option 1**, because it is self-consistent with the "keep Pratt table-ized" position:
since the dispatch table is the single source of truth, hand-writing "detection of a removed syntax"
is an anti-pattern. **The premise is that the C5 criterion proves the diagnostic set is
unchanged**—empirical testing of "after deletion, the diagnostic code and span of
`f(Int) -> Int = x => x` are the same as before deletion". If not, fall back to Option 2 and record
as an open issue.

### Test Rebuild

**This is the first priority and must precede any paradigm change.**

**Stage 1: Revive `lexer/tests/`.** Requires **two changes**, not one:

1. In `src/frontend/core/lexer/mod.rs` after `:106`, add `#[cfg(test)] mod tests;`
2. In `src/frontend/core/lexer/tests/mod.rs` add `mod lexer_mod;` and `mod symbols;` (otherwise
   these 159 lines still won't run)

**Stage 2: Handle the 7 empty shells.** Key constraint: `tests/mod.rs:28-38`'s `pub use xxx::*;`
referencing an empty module **produces no compile error or warning**, after successful wiring CI
will show "all green" while actually zero coverage. Handling:

| Option                 | Approach                                                                                      | Evaluation                                                                                                                                            |
| ---------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Delete (advocated)** | Delete 7 empty shell files + corresponding `mod` declarations + corresponding `pub use` lines | **Clean.** The only value of empty shells is "to be filled later", and `literals.rs` / `rfc004_lexer.rs` have already taken on these responsibilities |
| Fill                   | Write real assertions for each empty shell                                                    | 7 files × content unknown, **cannot verify what should be tested** (unverified)                                                                       |
| Keep + mark            | Keep but add `// TODO` and CI count alarm                                                     | Creates "covered" illusion, **opposed**                                                                                                               |

**Position: delete**, and in the dead code cleanup simultaneously delete the `pub use` re-exports at
`tests/mod.rs:28-38` entirely (it itself exists to match a non-existent re-export design, and it is
precisely it that masks the empty shells).

**Stage 3: Add associativity test cases.** Add `pratt/tests/binding_power.rs`, asserting on the
**actually-effective two ladders**:

- After merge, intra-layer bp strictly increasing, no duplicates (**this would catch the four
  collisions above before refactoring**)
- For each adjacent layer pair, write a minimal expression asserting its associativity direction
  (e.g. `a - b - c` → `(a-b)-c`)
- `DotDot` / `FatArrow` / pattern detection three special cases each have a set of cases,
  **replacing the current empty `pratt/tests/precedence.rs:6-13`**

**Stage 4: Add literal error path tests.** For the four categories in the uncovered paths table, add
cases item by item: four radix overflows, "continue consuming after `checked_mul`" branch,
`scan_leading_dot` as a whole, `\x` / `\u` illegal escape.

**Stage 5: Delete `precedence_inline.rs`** (with dead code deletion), and add a **wiring
self-check** assertion in `pratt/tests/mod.rs` to prevent re-emergence of orphans.

### Before-After Comparison: How Many Places to Change for One Operator

| #   | Current                                                                        | Current Enforcement                          | Target                                       | Target Enforcement                              |
| --- | ------------------------------------------------------------------------------ | -------------------------------------------- | -------------------------------------------- | ----------------------------------------------- |
| 1   | `lexer/state.rs:21` `keyword_from_str` adds match arm (keyword form only)      | None                                         | Operator declaration table 1 line            | Compile-time                                    |
| 2   | `lexer/tokens.rs:81-162` `TokenKind` adds variant                              | Compile-time                                 | **Generated from table**                     | Compile-time                                    |
| 3   | `pratt/precedence.rs:6-17` and `22-31` choose one set to add BP constant       | None                                         | **Generated from table**                     | Compile-time (intra-layer uniqueness assertion) |
| 4   | `pratt/led.rs:30-88` `infix_info` adds arm                                     | Compile-time (exhaustive)                    | **Generated from table**                     | Compile-time                                    |
| 5   | `pratt/led.rs:116-136` `parse_binary` adds op mapping arm                      | Compile-time (exhaustive)                    | **Generated from table**                     | Compile-time                                    |
| 6   | `ast.rs:191-213` `BinOp` adds variant                                          | Compile-time                                 | **Generated from table**                     | Compile-time                                    |
| 7   | New symbol character: `tokenizer.rs:218` `next_token_inner`                    | None                                         | **Generated from table**                     | Compile-time                                    |
| 8   | `const_data.rs:234` `const_data::BinOp` adds variant                           | **None (cross-enum, compiler cannot catch)** | **Same-source generation with `ast::BinOp`** | Compile-time                                    |
| 9   | Sites in 17 downstream production files depending on `matches!` / `_` fallback | **None (silent)**                            | Generated accessors                          | Compile-time                                    |

**Summary:**

| Stage                             | Manual Change Places                                                                              | Auto-Generated Places |
| --------------------------------- | ------------------------------------------------------------------------------------------------- | --------------------- |
| Current                           | **11-12** (6-7 within L2 + about 5-6 production files downstream that must sync)                  | 0                     |
| Target (pure new symbol operator) | **1-2** (1 line in operator declaration table; if keyword form, add 1 line in `keyword_from_str`) | 7-8                   |
| Target (reuse existing token)     | **1**                                                                                             | 6-7                   |

And the target's 1-2 places **are all declarative data lines**, not logic code—review cost goes from
"deducing semantics of each match arm" to "reading one declaration line".

## Detailed Design

### Interface Boundary with the Type System

**Boundary declaration (important)**: The convergence of `ast::Type` (26 variants,
`ast.rs:427-541`), the elimination of parallel type representations, the ownership of
`is_type_param_annotation` / `name_used_as_type`, **all belong to
[03-type-unification.md](03-type-unification.md)**. This document only handles three interfaces
directly related to the frontend paradigm:

**Interface 1: The relationship between `const_data::BinOp` and `ast::BinOp`.** Belongs to **this
document**—because it is a direct component of the "add one operator" change surface. Handling:
**simultaneously generate** both enums from the operator declaration table (or generate one, derive
the other), eliminating import aliases at `const_eval.rs:19` / `ir_gen.rs:2317`. The reason for
assigning to this document rather than 03: it is the L2 product being parallel-copied by L3, not a
problem with the type representation itself.

**Interface 2: The hardcoded `"Terminates"` and `typecheck::operator_interfaces::spec()` call at
`declarations.rs:498-509`.** Belongs to **03-type-unification.md / 02-stage-contract.md** (eliminate
parser → typecheck reverse dependency, RFC-039 routing table C row 1). This document **isolates this
code to `apply_semantic_side_effects`** during function splitting, as a handover point, but doesn't
change its content.

**Interface 3: Whether the operator's precedence layer needs type-layer information.** Currently not
needed—`infix_info` is a pure syntax function. **Design judgment**: After table-ization, this
property should be maintained; the operator declaration table **must not** contain any type-related
fields, otherwise L2 will depend on L3 again (violating RFC-039's dependency direction spec).

### Runtime Behavior

**Target: Runtime behavior unchanged bit-by-bit.** Based on C5 criterion in
07-equivalence-oracle.md, the third layer (end-to-end corpus diff) compares the exit codes and
stdout/stderr of `tests/yaoxiang/` 293 `.yx` + `src/std/tests/*.yx`.

The three **identified runtime impacts** must be explicitly handled:

| Change                                                      | Runtime Impact                                | Handling                                                                      |
| ----------------------------------------------------------- | --------------------------------------------- | ----------------------------------------------------------------------------- |
| f-string interpolation merged into 1 tokenize               | **Performance improvement**, output unchanged | Need to empirically measure time savings and record                           |
| Operator table generates BP / two ladders merge and reorder | **May change associativity**                  | **Must write equivalence cases for each layer pair** (test rebuild · stage 3) |
| Delete `is_old_function_syntax`                             | Old syntax's diagnostic code/span may change  | Need corpus diff to prove consistency                                         |

**Explicitly not done**: do not change the precedence **semantics** of operators. BP numbers can be
reordered (eliminating dead code and collisions), but the **relationships** like "multiplication
binds tighter than addition" must remain unchanged.

### Compiler Change Manifest

File by file, with line numbers. C6 cleanup (stage 0, can be committed independently):

| File                                                        | Line Numbers                          | Change                                                                                                       |
| ----------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `src/frontend/core/parser/pratt/precedence.rs`              | `8-13`, `11`                          | Delete 6 zero-reference first set constants                                                                  |
| Same                                                        | `22`                                  | Change `BP_RANGE` to named `..` binding power (sync `:40`), otherwise delete                                 |
| Same                                                        | `35-94`                               | Delete `Precedence` enum                                                                                     |
| Same                                                        | `98-133`                              | Delete `PrecedenceContext` struct                                                                            |
| Same                                                        | `6-17`, `22-31`                       | Keep alive constants, merge with second set to layer-conflict-free numbering                                 |
| `src/frontend/core/parser/pratt/led.rs`                     | `33`                                  | `BP_ASSIGN` repointed to reordered assignment layer constant                                                 |
| Same                                                        | `40`                                  | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R`                                                                        |
| Same                                                        | `75`, `77`, `79`, `83`                | `BP_CALL` repointed to reordered suffix layer constant                                                       |
| Same                                                        | `81`                                  | `BP_CAST` repointed to reordered cast layer constant                                                         |
| Same                                                        | `85`                                  | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                     |
| `src/frontend/core/parser/pratt/nud.rs`                     | `32`, `35`, `83`, `101`, `223`, `243` | `BP_UNARY` repointed to reordered prefix layer constant                                                      |
| Same                                                        | `38-69`                               | `BP_HIGHEST` repointed to reordered highest layer constant                                                   |
| Same                                                        | `1041`                                | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`, and add comment explaining the value's origin |
| `src/frontend/core/parser/statements/declarations.rs`       | `120-122`                             | Delete `skip_old_function_syntax`                                                                            |
| Same                                                        | `725`                                 | Delete its call site (deletion of `is_old_function_syntax` see stage 2)                                      |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | Whole file                            | Delete (the only orphan file referencing dead code)                                                          |

Companion renaming (collateral of BP merge, about 89 places): `parser/mod.rs:80`,
`statements/control_flow.rs` 9 places, `statements/declarations.rs:18` / `321` / `601` / `743` /
`818` / `850` / `966` / `994`, `statements/functions.rs:9` / `:110`, `statements/bindings.rs:168`,
`statements/types.rs:15` / `:641` / `:654`, `pratt/led.rs:99` / `:229` / `:321` / `:327` / `:415`,
`pratt/nud.rs:195` / `:263` / `:446` / `:516` / `:523`.

Test revival (stage 1, can be committed independently):

| File                                                 | Line Numbers        | Change                                                        |
| ---------------------------------------------------- | ------------------- | ------------------------------------------------------------- |
| `src/frontend/core/lexer/mod.rs`                     | After `106`         | Add `#[cfg(test)] mod tests;`                                 |
| `src/frontend/core/lexer/tests/mod.rs`               | `15-25`             | Add `mod lexer_mod;` and `mod symbols;`                       |
| Same                                                 | Whole file          | Delete 7 empty shells' `mod` declarations and `pub use` lines |
| Same                                                 | Whole file          | Delete `pub use ... ::*;` re-export block (`28-38`)           |
| `src/frontend/core/lexer/tests/`                     | 7 empty shell files | Delete files                                                  |
| `src/frontend/core/parser/pratt/tests/mod.rs`        | `4-6`               | Add wiring self-check assertion                               |
| `src/frontend/core/parser/pratt/tests/precedence.rs` | `6-13`              | Replace empty assertion with real associativity assertion     |

Paradigm change (stages 2-4):

| File                                                           | Change                                                                                                                                                                                                                    |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/literals.rs`                          | Merge four radix scanners into `scan_radix_number(lexer, base, digit_pred)`; merge three escape decoding into the only `decode_escape(lexer, out)`; merge `scan_multi_line_string:869` and `scan_fstring_multi_line:1497` |
| `src/frontend/core/parser/pratt/nud.rs:441-446`                | Remove runtime nested `tokenize()`, change to one-shot production at lexical stage                                                                                                                                        |
| `src/frontend/core/parser/pratt/led.rs:30-88` / `nud.rs:26-75` | Two hand-written tables replaced with generated mapping tables                                                                                                                                                            |
| `src/frontend/core/parser/ast.rs:191-213`                      | `BinOp` and `const_data::BinOp` changed to same-source generation                                                                                                                                                         |
| `src/frontend/core/types/const_data.rs:234`                    | Same as above                                                                                                                                                                                                             |
| `src/frontend/core/parser/statements/declarations.rs:136-642`  | Split into 8 functions + `ParserState::skip_balanced_parens()` (4 places at `96-105`/`147-156`/`191-200`/`224-233` changed to calls)                                                                                      |
| `src/frontend/core/parser/parser_state.rs`                     | Add `skip_balanced_parens()` (`synchronize()` at `:170`, unchanged)                                                                                                                                                       |

**Files with zero changes in this document**: `src/middle/core/ir_gen.rs`,
`src/frontend/core/typecheck/**`, `src/backends/**`, **main body** of
`src/frontend/core/formatter/**`. They only change when "generated accessors" replace `matches!`
macros, and the change is mechanical replacement.

### Backward Compatibility

What table-ization most easily breaks is **error recovery**. This project's recovery contract
consists of three parts, each of which must be maintained:

**Contract 1: `ParserState::synchronize()` (`parser_state.rs:170-186`).** It skips tokens forward to
synchronization points after statement parsing failure. **The C5 criterion requires preserving it,
and requires the timing it triggers and the skip set to be completely identical.** After migration
to LALRPOP, the call sites of this function are changed to be driven by explicit error productions,
the function body and synchronization point set **don't change a single line**—semantics is
reproduced by the grammar, implementation is kept as-is.

**Contract 2: `Expr::Error` (`ast.rs:1048`, consumed at `pratt/mod.rs:72`) and `StmtKind::Error`
(constructed at `parser/mod.rs:50`).** These two placeholders enable the parser to continue
constructing a "partial AST" after an error, with downstream skipping by `Error` branch. **The
variant must be preserved, and their position in downstream `match` must be preserved** (if
downstream puts `Expr::Error` in a wildcard branch, errors will be silently swallowed).

**Contract 3: Diagnostic code, span, and order.** The comparison rule in 07-equivalence-oracle.md's
third layer is "diagnostic list sorted by `(code, span.file, span.line)`, **message text not
participating in comparison**". This gives this document some margin (wording can vary), but **code
and span cannot change**.

**f-string special note**: The interpolation refactoring changes span from "relative to
interpolation text" to "source file absolute offset". To satisfy contract 3, **spans must be
recorded by absolute offset at the lexical stage** (resolution D25)—the `span.file/line` of
diagnostics inside the interpolation must not change due to this refactoring.

> **Handling**: Establish a baseline snapshot of "f-string interpolation diagnostic spans" before
> stage 2, compare item-by-item after refactoring. `span.file/line` inconsistency is treated as an
> implementation defect and fixed, there is no "accept span change" option. (If empirical testing
> proves the original implementation's interpolation span was already wrong, the baseline
> establishment stage will expose it—that is an existing independent defect, fixed separately, not
> incorporated into this stage, and does not constitute a reason to relax the criteria.)

## Implementation Points

This document corresponds to five stages. **Each stage can be committed and rolled back
independently.** Stages 0 and 1 are C6 and test revival, requiring no equivalence criteria; stages
2-4 are C5, with criteria enforced throughout.

| Stage  | Content                                                                                                               | Category    | Acceptance Criterion                                                                                                            | Rollback Point                    |
| ------ | --------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| **0**  | Dead code cleanup (`Precedence` enum + `PrecedenceContext` 96 lines)                                                  | C6          | `cargo build` passes; `cargo clippy` no new warnings                                                                            | Single commit, `git revert` works |
| **1**  | Test revival and completion                                                                                           | —           | Lexical layer actually running test count ≥ 5 files / 629 lines; 7 empty shells deleted; associativity cases **red-then-green** | Single commit                     |
| **2**  | Lexical duplication convergence + f-string nested compilation elimination                                             | C5          | AST snapshot identical + diagnostics identical + behavior identical; `literals.rs` reduced by ~500 lines                        | Separate commit from stage 3      |
| **3a** | Build LALRPOP grammar + action code, **coexisting completely with Pratt**                                             | —           | `cargo build` passes; new parser can produce AST for 293 corpora (no comparison)                                                | Separate commit from stage 2      |
| **3b** | **Double-parser diff** (both parsers run 293 corpora, **AST + diagnostic code+span** normalized, compared bit by bit) | **Full C5** | **AST and diagnostics are equivalent item-by-item—this step is the equivalence proof of the entire grammar migration**          | Offline run once, repeatable      |
| **3c** | Cutover: `parser/mod.rs`'s `parse()` switches to LALRPOP, Pratt kept as `parse_legacy()`                              | **Full C5** | Full corpus behavior equivalent + diagnostic code+span identical item-by-item                                                   | Independent commit                |
| **3d** | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + `precedence.rs`'s two ladders + about 89 references                    | **Full C5** | Corpus all green; `git grep BP_` zero hits                                                                                      | Independent commit                |
| **4**  | `parse_assign_after_target` split                                                                                     | C5          | This function only does dispatch; `skip_balanced_parens` single implementation                                                  | Independent commit                |

**Internal dependency order in this document**: Stage 1 **must precede** stages 2/3/4—"refactor
first, add tests later" is a pit the project has already stepped in. **Stage 3b (double-parser diff)
cannot be skipped**: it must run all green before 3c cutover, otherwise the entire grammar migration
has no equivalence basis.

**3b/3c go/no-go checkpoint**: If the number of error productions clearly exceeds what is needed to
"model `synchronize()`'s synchronization point set" (the parser-embedded semantic
judgments—`declare_predicate` accumulation, const generic parameter filtering—are the most difficult
to migrate), or the diff still has non-equivalent items for two consecutive iterations, **stop
migration and re-evaluate**, falling back to the intermediate plan of "keep Pratt + single
declarative precedence table + both `BinOp` same-source generated". That plan has already obtained
most of the operator change surface convergence benefit; LALRPOP is a means, not the goal. The
fallback belongs to the "re-evaluation" branch reserved by D24, **not a relaxation of criteria**.

**A note on stage 0**: The 6 **alive** first set constants `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` /
`BP_CALL` / `BP_CAST` / `BP_HIGHEST` **are not in stage 0's deletion scope**—they are deleted
together with Pratt in stage 3d. Stage 0 only deletes the `Precedence` enum and `PrecedenceContext`
(96 lines, zero production references).

**Criterion execution**: The three-layer criterion definitions and regression gate are in
[07-equivalence-oracle.md](07-equivalence-oracle.md). **All go through full C5, including diagnostic
code+span. There is no relaxation whatsoever.**

**Scope limitation**: This document **does not include** the `logos` migration (lexical layer keeps
hand-written + merge duplicates, see "Key Decisions and Rationale"). **The LALRPOP syntax migration
is a component of this document**, see the "Syntax: Complete Grammar-Driven" section.

## Key Decisions and Rationale

### Three Decided Decisions

| Decision                      | Determination                                                                             | Rationale                                                                                                                                                                                                                                                                                      |
| ----------------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexical plan                  | Adopt "keep hand-written + force merge duplicates"; `logos` listed as conditional backup  | Eliminate ~500 lines of repetition with zero new dependencies; `logos` changes `LexError` error type, and cannot solve f-string nested compilation                                                                                                                                             |
| **Syntax plan**               | **Adopt LALRPOP complete grammar-driven**; reject tree-sitter and "keep Pratt table-ized" | tree-sitter produces CST not domain AST, still needs a complete mapping code; table-ization only solves synchronization, doesn't solve maintainability, is an insufficient intermediate station. LALRPOP's equivalence risk is resolved by **double-parser diff** (see above), not by argument |
| `const_data::BinOp` ownership | Belongs to this document                                                                  | It is a direct component of the "add one operator" change surface; handled separately from `ast::Type` convergence (03)                                                                                                                                                                        |

### A Note on "Doing It Completely"

Table-ization is an insufficient intermediate station: it solves synchronization, not
maintainability; and "waiting for criteria to be relaxed before re-evaluating" is passive waiting,
not an engineering decision. Therefore this document adopts **complete grammar-driven**, with
equivalence guaranteed by empirical double-parser diff (see above), and no relaxation at the
diagnostic layer.

**The cost of this transition must be made clear**:

- Construction volume significantly increases: need to build grammar file, move 22 `Expr` variant
  construction actions, merge about 89 BP references, rewrite error recovery
- **Prerequisites become harder**: 629 lines of frontend tests must be revived first (09's P1), AST
  normalization snapshots must be checked in first (09's P2). Without these two, the diff in step 2
  has no basis
- **Introduce a new dependency** (LALRPOP), inconsistent with the lexical layer's "zero new
  dependencies" orientation

**The benefit is definite**: "adding an operator" goes from 11 places to 2 places, BP conflicts go
from "accidentally correct" to "grammar build-time error", `Expr::Lambda` borrowing,
`parse_expression(12)` magic number, about 120 lines of dead old syntax detection **naturally
disappear**.

### Rejected or Downgraded Options

**A. Only delete dead code, no paradigm change.** Verified feasible, but insufficient to solve the
problem. Dead code cleanup (about 130 lines + 3 magic numbers) can be completed at **zero criterion
cost**, and should be done immediately. But its impact on "adding an operator requires editing 11
places" is: the L2 change surface **won't decrease by a single place**; cross-enum missed edits
**are completely untouched**. **Conclusion: Adopt as stages 0 and 1, but must continue with grammar
driving. Treating A as a complete plan is a wrong form of self-consolation.**

**B. Keep Pratt, only add tests.** **Not adopted as a complete plan, but adopted as a
prerequisite.** Tests **cannot discover cross-enum missed edits**—the two enums each compile
independently, whether tests pass or not is unrelated to whether they are in sync. **Conclusion:
Adopt as stage 1; reject as a complete plan.**

**C. Keep Pratt and table-ize.** **Rejected** (2026-10-03). It is an unnecessary intermediate
station between "completely hand-written" and "complete grammar": the 22 `Expr` variants still need
22 hand-written construction actions, the `Expr::Lambda { body: empty Block }` borrowing problem,
the `parse_expression(12)` magic number, the 89-reference merge of the two ladders, all still
remain.

**D. Adopt tree-sitter.** **Rejected.** tree-sitter's incremental parsing capability has no
substantial benefit for this project (full compilation each time, 293-corpus scale); and it produces
CST not domain AST, still needs a complete mapping code, equivalent to net adding the same size
outside `nud.rs`.

**E. Adopt `logos` to replace hand-written lexer.** **Conditionally not adopted.** **Prerequisite
for re-evaluation**: after stage 2 completion (escape decoding has been converged to a single
implementation), the workload of replacing that implementation with a `logos` callback will be
greatly reduced.

### Net Benefits of the Plan

- **Change surface goes from "a dozen places of logic code" to "1-2 places of declarative data + 7-8
  places generated.** And the generated 7-8 places are guaranteed synchronous by the compiler,
  missed edits are no longer possible events.
- **Eliminate a class of defects that the compiler can never catch.** Cross-enum missed edits
  (`ast::BinOp` vs `const_data::BinOp`) is currently the only **defect type that no tool can
  discover**, because the two types are independent of each other. Same-source generation directly
  eliminates it.
- **Turn BP collisions from "accidentally correct" to "compilation failure".** The current four
  cross-set collisions and two intra-set self-collisions are currently lucky not to blow up because
  of the unwritten `bp_right = bp_left + 1` convention; intra-layer uniqueness assertions make such
  problems impossible to exist silently from then on.
- **Frontend tests go from "1 file running" to "all running".** After 629 lines of existing tests
  are revived, stages 2-4 have a safety net; and this batch of tests **already exists**, just one
  line of `mod` wasn't written.
- **Stages 0 and 1 are zero-risk, zero-criterion cost.** Dead code deletion and test revival can be
  done first, blocking no other work.

## Known Limitations and Risks

### Design Risks

- **The operator table is a new "thing that must be maintained".** It turns "change 11 places" into
  "change 1 place + maintain a table". **If the table's field design itself is improper, it will
  degenerate into a new centralized maintenance burden**. This is the main risk of this plan, the
  mitigation is to protect each column of the table with compile-time assertions.
- **The risk of BP number reordering is asymmetric.** Merging the two ladders involves renaming
  about 89 production references; if one is missed, **associativity will silently change** (no
  error, just different expression parsing results). This is the most likely silent regression
  introduced by this document, must be caught by stage 1's layer-pair associativity cases.
- **The C5 criterion cost for stages 2-4 is very high.** Using all three layers means each stage
  needs to run the full corpus (293 `.yx`) + AST snapshots + diagnostic comparison. This will make
  each commit in stages 2-4 relatively heavy.
- **f-string span changes may not be completely avoidable.** See "Backward Compatibility", may need
  to accept one known behavior change and update the baseline.

### Unresolved Issues

> **The open questions originally listed in this section have all been adjudicated.** Item-by-item
> decisions are in [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## Verification Record

All code facts in this document have been **directly read from disk** for verification. Verification
method: `read` tool directly reads target file line ranges; line counts use `Get-Content`; reference
surface uses `grep` tool for whole-repository scanning and classified into production / test /
orphan categories. **`cargo test` / `cargo build` were not used**—this document is pure
documentation, criterion execution belongs to the implementation stage.

### Item-by-Item Test Results

| Verification Item                             | Test Result                                                                                                                                                                                                                                                                                      |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `literals.rs` total lines                     | **1568**                                                                                                                                                                                                                                                                                         |
| `tokenizer.rs` total lines                    | **554**                                                                                                                                                                                                                                                                                          |
| `next_token_inner` line number                | **`:218`**                                                                                                                                                                                                                                                                                       |
| Four radix scanners line ranges               | `77-161` (85 lines) / `164-247` (84) / `250-333` (84) / `336-516` (181)—consistent with this document's table                                                                                                                                                                                    |
| Three escape decoding                         | `694-827` (134 lines) / `1015-1141` (127 lines) / `push_fstring_escape:1374`—consistent with this document's table                                                                                                                                                                               |
| `scan_leading_dot`                            | `519-635`, **117 lines**                                                                                                                                                                                                                                                                         |
| `Expr` variant count                          | **22** (including `Error`), `ast.rs:16-163`                                                                                                                                                                                                                                                      |
| `lexer/tests/` file count                     | **14** (including `mod.rs`)                                                                                                                                                                                                                                                                      |
| Orphan real test lines                        | **629** (222+167+89+81+70); additionally 90 lines of `fstring.rs` already wired and running via `#[path]`, not counted as orphan                                                                                                                                                                 |
| Changes needed to revive `lexer/tests/`       | **Two**: add `mod tests;` to `lexer/mod.rs` **and** add `mod lexer_mod;` `mod symbols;` to `tests/mod.rs`. Adding only one leaves the latter 159 lines still not running                                                                                                                         |
| `precedence.rs` dead code                     | **96 lines** (`Precedence` enum `35-94` + `PrecedenceContext` `98-133`) zero production use **holds**; but the only external referrer `precedence_inline.rs` (95 lines / 6 tests) is itself an orphan—**deleting dead code must also delete that file**                                          |
| 7 empty shells line count                     | 1-3 lines each (`debug_lexer.rs` 1 line, rest 3 lines)                                                                                                                                                                                                                                           |
| Blank line residue form                       | **3 consecutive blank lines inserted between each field, across 11 lines** (`777-787` / `1098-1108`)                                                                                                                                                                                             |
| `pratt/tests/precedence.rs` line count        | **13**, the only assertion `assert!(bp_lowest < bp_highest)`                                                                                                                                                                                                                                     |
| The 4th "skip paired parentheses"             | `224-233`                                                                                                                                                                                                                                                                                        |
| Downstream change surface                     | **17 production files** match `BinOp::` (whole-repository **534 times / 47 files**)                                                                                                                                                                                                              |
| Whether `src/backends/` is a `BinOp` consumer | **No**—`src/backends/` has **zero hits** for `BinOp` and `ast::`; the interpreter consumes IR / bytecode, never seeing AST                                                                                                                                                                       |
| "Missed edits have no check to discover"      | **Needs stratification**: exhaustive `match` missed edits **are caught by the compiler**; silent ones are `matches!` macros, `_` fallbacks, and **cross-enum** three categories. This document already expresses by stratification per empirical test                                            |
| `const_data.rs`'s `matches!` block            | `263-288` (`is_arith` / `is_comparison` / `is_logical` / `is_bitwise` four function range)                                                                                                                                                                                                       |
| `impl Display for BinOp`                      | Starting at `291`, 18-arm exhaustive match, no `_` fallback                                                                                                                                                                                                                                      |
| Two functions in `parse_assign_after_target`  | **Not dead code**: the two functions are still called at `declarations.rs:719` / `:725`, are running "rejects removed syntax" gatekeepers; and `skip_old_function_syntax` (`120-122`, function body only one comment line) after call **consumes no tokens** (`:725-726` directly `return None`) |
| Second `BinOp` enum                           | `const_data.rs:234` does exist, with `ast::BinOp` **no `From` conversion**, forcing `const_eval.rs:19` / `ir_gen.rs:2317` etc. files to alias at import site                                                                                                                                     |
| `lexer/tests/mod.rs` re-export block          | Exists (38-line file), `:28-38` has `pub use ... ::*;`, the direct reason empty shells can disguise as coverage                                                                                                                                                                                  |

### Three Points Corrected by This Round of Verification

The following three points are inconsistent with the intuition that "the first set of BP ladder is
already dead", with empirical testing as the standard:

1. **The two ladders are interwoven, not "one alive, one dead".** `infix_info` (`led.rs:30-88`)
   itself references both 5 constants from the first set (`BP_ASSIGN:33`, `BP_CALL:75/77/79/83`,
   `BP_CAST:81`) and 8 from the second set. Of the 12 first-set constants, **only 6 have truly zero
   references** (`BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR`, plus
   `BP_COMPARISON` which only exists in a comment at `declarations.rs:742`); the other 6
   (`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`) have a combined
   **about 89 production references, across 8 files** in production code. So "delete the first set
   of BP constants" is not a 12-line C6 deletion, but a C5-category cross-file rename.
2. **The second set itself has dead members.** `BP_RANGE` (`precedence.rs:22`) has zero use in
   production code—`..` at `led.rs:40` uses the bare literal `(6, 7)`, and `BP_RANGE` only appears
   in the comment at `led.rs:157-159`.
3. **That "self-contradictory comment" is at `precedence.rs:33`, not `:11`.** `*:11` is
   `pub const BP_COMPARISON: u8 = 5;`. That comment (_"Precedence rules for the Pratt parser"_)
   describes the zero-reference `Precedence` enum.

Incidental correction: `bp_right = bp_left + 1` produces **left-associativity** (`a - b - c` →
`(a-b)-c`), not right-associativity; the `(11, 1)` at `led.rs:85` is the only explicit right-operand
greedy swallow form. Collisions are also not limited to two cross-set places, additionally
`BP_UNARY=8 == BP_CAST=8` (alive intra-set self-collision) and `BP_RANGE=7 == BP_ADD=7` (intra-set
self-collision in the second set).

### Unverified Items

The text has already marked "unverified" item by item:

- The exact arm count and per-binding-power of `pratt/nud.rs`'s `prefix_info` table body (`26-75`)
- The exact line boundaries of each of the 8 responsibility sections in `declarations.rs:136-642`
- The complete variant set of `const_data::BinOp` (`const_data.rs:234`)
- Whether `logos` / `regex` is already in `Cargo.toml` (as a transitive dependency)
- All call sites of `synchronize()` outside `parser_state.rs:170-186`
- Whether this project is a published library (affects risk assessment of removing public surface)

## See Also

### Co-batch Subsidiary Design Documents

- [01-routing.md](01-routing.md) — Where to change when adding a feature (feature routing table
  A/B/C, dependency direction spec)
- [03-type-unification.md](03-type-unification.md) — `ast::Type` convergence ownership, interface
  boundary with this document
- [04-ssa.md](04-ssa.md) — Also a C4/C5 criterion user
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Complete inventory of 8 orphan test trees
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C5/C6 category definitions, three-layer
  criteria, regression gate

### RFC Main Text and Related Proposals

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Upper-level master plan; four-layer model, glossary, feature routing table A/B/C
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  `build.rs` build-time gate, the gate model this document borrows from
- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Origin of
  `ast::Type`

### Code Location Index

- `src/frontend/core/lexer/literals.rs:77-161` / `164-247` / `250-333` / `336-516` — Four radix
  scanners
- `src/frontend/core/lexer/literals.rs:694-827` / `1015-1141` / `1374` — Three escape decoding
  implementations
- `src/frontend/core/lexer/literals.rs:519-635` — `scan_leading_dot` (zero coverage)
- `src/frontend/core/lexer/mod.rs:104-106` — Only test declaration in the lexical layer
- `src/frontend/core/lexer/tests/mod.rs:15-25` / `28-38` — Submodule declarations and `pub use`
  re-export block
- `src/frontend/core/parser/pratt/nud.rs:441-446` — f-string interpolation's runtime nested
  `tokenize()`
- `src/frontend/core/parser/pratt/nud.rs:896-901` / `1041` — Magic number `12`'s origin
  self-description and use point
- `src/frontend/core/parser/pratt/precedence.rs:6-17` and `22-31` — Two BP ladders
- `src/frontend/core/parser/pratt/precedence.rs:33` / `35-94` / `98-133` — Disconnected comment,
  zero-reference enum and struct
- `src/frontend/core/parser/pratt/led.rs:30-88` — `infix_info` hand-written table
- `src/frontend/core/parser/pratt/tests/mod.rs:4-6` — Orphan wiring missing `precedence_inline`
  declaration
- `src/frontend/core/parser/statements/declarations.rs:719-727` — Old syntax rejection gatekeeper
  and its "skip but don't consume" defect
- `src/frontend/core/types/const_data.rs:234` — Second operator enum parallel to `ast::BinOp`
- `build.rs:19-55` — Error code build-time gate, the existing enforcement model example in this
  repository
- `src/middle/core/tests/bytecode.rs:369-657` — `test_every_opcode_roundtrips_not_silently_nop`, the
  existing "per-item roundtrip" criterion model example in this repository
