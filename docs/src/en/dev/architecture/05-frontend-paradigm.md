# Frontend Paradigm: Lexing and Parsing

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and implementation-phase ordering are covered in the
> main RFC-039 text; see the [index for this directory](index.md) for the positioning of each
> companion document.

## Positioning and Scope

This article covers the L2 frontend—**lexing (`src/frontend/core/lexer/`) and parsing
(`src/frontend/core/parser/`)**.

**Problem to solve**: reduce the change footprint for “adding a binary operator” from the current
**6–7 locations within L2 + 17 downstream production files** (the compiler enforces roughly 8–9 of
them, while the rest require manual synchronization) to **1–2 declarations + 7–8 automatically
generated locations**.

This article addresses the pain point identified in the first “binary operator” row of RFC-039’s
functional routing table A. The refactoring category is **C5 frontend paradigm change** as defined
in [07-equivalence-oracle.md](07-equivalence-oracle.md) (criteria: identical AST snapshots +
identical diagnostics + identical behavior, with all three used) together with **C6 pure deletion**
(no equivalence criteria required, only confirmation that there are no references). C6 items are
prerequisites for C5: delete 96 lines of dead code, eliminate three bare magic numbers, and revive
629 lines of lexer tests that have never run.

**Scope boundary**: handling `ast::Type` (26 variants) and converging the three parallel type
representations belongs to [03-type-unification.md](03-type-unification.md). This article does not
duplicate that work; it only declares the interface boundaries in the “Detailed Design” section.

### Why This Is Worth Doing

“Adding an operator” is the most expensive change in the frontend. This tax has **three pathological
characteristics**:

**First, the cost does not decrease with experience.** Someone who has added ten operators will not
need to change one fewer location than someone who has added one, because there is no derivable
relationship between these locations. The `(6,7)` in `led.rs:40` and the `12` in `nud.rs:1041` have
no connection whatsoever, yet they must agree with each other or expression parsing will fail.

**Second, missed updates fall into two categories, and the compiler cannot catch the worst
category.** See the tiered table in “The Change Footprint of Adding an Operator”: an omitted arm in
an exhaustive `match` is caught by the compiler, whereas **an omission across enums is never caught
by the compiler**.

**Third, this tax is invisible in the accounts.** Nearly every commit in
`git log --oneline -- src/frontend/core/parser/pratt/led.rs` touches three or four files
simultaneously, but no tool can identify “which location this commit failed to update.”

### Correct Patterns Already Present in This Repository

This project has two **correct solutions to the same class of problem**, both of which can be reused
directly:

- **Error codes**: `build.rs:19-55` uses `tools/code-tables` to parse the registry at **build
  time**, validate uniqueness, and compare it line by line against RFC-013’s Markdown code table.
  **Any discrepancy directly triggers `panic!` and rejects the build.** The cost of adding an error
  code is therefore a constant one location plus documentation.
- **Standard-library interfaces**: byte-for-byte comparison in `gen_interfaces.rs`, and
  marked-region drift detection in `gen_docs.rs` (the final row of RFC-039’s routing table B).

Both mechanisms share the same principle: **turn “changes that must be kept in sync” from something
people must remember into executable assertions**. The goal of this article can be reduced to one
sentence: turn the relationships among the operator table, lexical-rule table, and AST-variant table
into executable assertions.

### Why “Deleting Dead Code Alone” Is Insufficient

Dead code does exist (`precedence.rs` at 96 lines, the empty `skip_old_function_syntax` function,
and 629 lines of orphan tests), but deleting it would not reduce the number of locations affected by
“adding an operator changes more than ten places” by even one. Dead code is **legacy debt**, while
the cross-file change footprint is **structural cost**. The two must be handled separately; see “Key
Decisions and Rationale.”

## Current State

Everything below is a **verified fact**, with a file path and line number attached to each item. See
“Verification Record” for the verification method and item-by-item measured results.

### Lexical Layer: Copy-and-Paste Evidence in `literals.rs`

`src/frontend/core/lexer/literals.rs` contains **1,568 lines** in total. This size does not come
from complexity; it comes from three kinds of structural duplication.

**Duplication 1: the four base scanners are nearly line-for-line identical.**

| Scanner               | Line range            | Lines |
| --------------------- | --------------------- | ----- |
| `scan_hex_number`     | `literals.rs:77-161`  | 85    |
| `scan_octal_number`   | `literals.rs:164-247` | 84    |
| `scan_binary_number`  | `literals.rs:250-333` | 84    |
| `scan_decimal_number` | `literals.rs:336-516` | 181   |

The first three are each about 84 lines and have exactly the same structure: accumulate digit →
`has_digits` check → `overflow` flag → `checked_mul(base)` → `checked_add` → `try_into` → report an
error. **The only differences are the base constant and the digit-extraction expression.** Of the
three copies × 84 lines, **roughly 250 lines are pure redundancy** (`scan_decimal_number` must be
listed separately because it additionally handles decimal points and exponents and therefore cannot
be merged).

Sample evidence—the overflow path in `scan_hex_number` (`literals.rs:134-141`):

```rust
if overflow {
    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
        value,
        point(lexer),
    ));

    return Some(lexer.make_token(TokenKind::Error("Hex number too large".to_string())));
}
```

`scan_octal_number:220-227` and `scan_binary_number:306-313` are the same code with a different base
and message text.

**Duplication 2: escape-decoding logic is implemented three times.**

| Implementation              | Location                       | Length                                       |
| --------------------------- | ------------------------------ | -------------------------------------------- |
| `scan_string`’s `\\` branch | `literals.rs:694-827`          | 134 lines                                    |
| `scan_char`’s `\\` branch   | `literals.rs:1015-1141`        | 127 lines                                    |
| `push_fstring_escape`       | Starting at `literals.rs:1374` | Already extracted into a standalone function |

The first two are near line-for-line copies of 127–134 lines (`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` → the
same pushes, identical `\x` / `\u{...}` logic, and the same `c =>` error fallback), so **roughly 250
lines have been written twice**.

One point deserves separate emphasis: **the f-string version has already been extracted into a
standalone function** (`push_fstring_escape:1374`). This shows that **someone had already noticed
the duplication and fixed it locally, but never returned to converge the other two
implementations**. This is more noteworthy than “none of the three noticed,” because it proves that
the problem is solvable; what is missing is an enforcement mechanism.

**Duplication 3: two implementations of multiline-string scanning.** `scan_multi_line_string:869`
and `scan_fstring_multi_line:1497` are another pair of implementations of the same kind.

**Formatting residue.** The two locations `literals.rs:777-787` and `1098-1108`, each spanning 11
lines, have **three consecutive blank lines inserted between every field of a struct literal**. This
is residue from an automated rewrite. Other positions in the same function (`1111-1115`) have only
one blank line between fields, showing that formatting was interrupted.

> This residue is harmless in itself, but it is a **diagnostic signal**: the same code was rewritten
> twice by different tools, which means that someone historically made a local patch here without
> pulling the surrounding context into alignment.

**Conclusion**: roughly 500 lines in `literals.rs` (250 lines of redundant base-scanning logic + 250
lines of doubled escape handling) are **mechanical duplication**, not complexity.

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

**At runtime, the parser layer runs a complete `tokenize()` separately for every f-string
interpolation and creates a new `ParserState`.**

The consequence is that an f-string containing N interpolations **triggers N+1 complete lexical
analyses** (N interpolations + 1 outer layer). At the same time, every interpolation is an
**independent parsing context**, with the following consequences:

- Diagnostic spans inside an interpolation are relative to the interpolation text, so remapping them
  to source locations requires additional work.
- An interpolation cannot access any outer parser state. There is currently no need for this, but
  the structure forecloses future possibilities, such as an interpolation referencing an outer
  implicit variable.
- The locations at which lexical and syntactic errors are produced are split across two separate
  stages.

This is a **defect in both performance and structure**, and it must be addressed; see “Target Design
· Lexing.”

### Syntax Layer: Two BP Ladders, Bare Magic Numbers, and Handwritten Dispatch Tables

#### The Two Ladders Are Not “One Live and One Dead”; They Are Interwoven

`src/frontend/core/parser/pratt/precedence.rs` contains **two binding-power ladders** whose
numbering overlaps extensively.

**The first set (`precedence.rs:6-17`)**: `BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` /
`BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` /
`BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`.

**The second set (`precedence.rs:22-31`)**: `BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` /
`BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`.

**Measured conclusion: both sets survive in production code and are already interwoven inside
`infix_info`.**

- `infix_info` (`led.rs:30-88`) references **five constants from the first set**: `BP_ASSIGN`
  (`:33`), `BP_CALL` (`:75` / `:77` / `:79` / `:83`), and `BP_CAST` (`:81`);
- It references **eight constants from the second set**: `BP_OR` (`:42`), `BP_AND` (`:44`), `BP_EQ`
  (`:47`), `BP_SHIFT` (`:51` / `:54`), `BP_BIT` (`:58`), `BP_CMP` (`:62` / `:65`), `BP_ADD` (`:68`),
  and `BP_MUL` (`:72`);
- **The only dead member of the second set is `BP_RANGE`**: `..` uses the bare literal `(6, 7)` at
  `led.rs:40` rather than `BP_RANGE`; in production code, `BP_RANGE` appears only in the comment at
  `led.rs:157-159`.

The survival status of the first set’s 12 constants, based on a whole-repository `grep`:

| Status              | Constants                                                                    | Production reference sites                                                                                                                                                                        |
| ------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Dead (6)**        | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | Zero production hits. Appear only in `precedence.rs` itself and the orphan test `precedence_inline.rs`                                                                                            |
| Dead (comment only) | `BP_COMPARISON`                                                              | Zero production hits; mentioned only in the comment at `declarations.rs:742`                                                                                                                      |
| **Alive (6)**       | `BP_LOWEST`                                                                  | `parser/mod.rs:80`, 9 locations in `statements/control_flow.rs`, 6 in `statements/declarations.rs`, `pratt/led.rs:229`/`:321`/`:327`/`:415`, `pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523`, etc. |
|                     | `BP_ASSIGN`                                                                  | `led.rs:33`, `led.rs:99`, `statements/declarations.rs:18` (import), `:743` (`BP_ASSIGN + 1`)                                                                                                      |
|                     | `BP_UNARY`                                                                   | `nud.rs:32`, `:35`, `:83`, `:101`, `:223`, `:243`                                                                                                                                                 |
|                     | `BP_CALL`                                                                    | `led.rs:75`, `:77`, `:79`, `:83`                                                                                                                                                                  |
|                     | `BP_CAST`                                                                    | `led.rs:81`                                                                                                                                                                                       |
|                     | `BP_HIGHEST`                                                                 | Approximately 20 locations in `nud.rs:38-69` (one each for every literal and keyword prefix)                                                                                                      |

**In total, the six surviving constants have approximately 89 production references across 8
files.**

> This corrects an easy-to-reach but incorrect conclusion: “delete the first ladder” is **not** a
> 12-line deletion. It is a rename and consolidation spanning 8 production files and 89 references.
> Only six constants in the first set truly have zero references, plus `BP_COMPARISON`, which
> survives only in a comment. The second set is also incomplete—`BP_RANGE` is dead.

**The truly zero-reference dead code is the two types**: the `Precedence` enum
(`precedence.rs:35-94`, 60 lines) and the `PrecedenceContext` struct (`precedence.rs:98-133`, 36
lines), for a total of **96 lines**. Verification found that these two types have zero production
uses: aside from `precedence.rs` itself, every repository-wide grep hit for `PrecedenceContext` and
`Precedence::` is in `src/frontend/core/parser/pratt/tests/precedence_inline.rs`.

**There is a detail that must be recorded in this document**: the only file referencing this dead
code is **itself an orphan**. `pratt/tests/mod.rs:4-6` declares only
`mod led; mod nud; mod precedence;` and **does not declare `precedence_inline`**.

> In other words, the 96 lines of dead code “appear to have test coverage” because they are tested
> by a 95-line/6-test file that has never participated in compilation; see “Dead Code and Current
> Test State.” **Deleting the dead code requires handling this orphan test file at the same time**,
> or the project will no longer compile.

In addition, the comment at `precedence.rs:33` contradicts itself and serves as corroborating
evidence: _"Precedence rules for the Pratt parser"_—this comment describes precisely the
zero-reference `Precedence` enum, whose ladder is not the same as the ladder actually in effect. The
enum and implementation have become decoupled.

#### Numeric Collisions

The numbering in the overlapping portions of the two ladders corresponds almost digit for digit, so
**cross-set collisions occur in pairs**:

| Collision                   | Value    | Use site A (first set)         | Use site B (second set)        |
| --------------------------- | -------- | ------------------------------ | ------------------------------ |
| Assignment vs logical OR    | `1 == 1` | `BP_ASSIGN` (`led.rs:33`)      | `BP_OR` (`led.rs:42`)          |
| Type cast vs multiplication | `8 == 8` | `BP_CAST` (`led.rs:81`)        | `BP_MUL` (`led.rs:72`)         |
| Shift vs term               | `6 == 6` | `BP_TERM` (dead)               | `BP_SHIFT` (`led.rs:51`/`:54`) |
| Bitwise vs comparison       | `5 == 5` | `BP_COMPARISON` (comment only) | `BP_BIT` (`led.rs:58`)         |

**There is also one collision within the first set**: `BP_UNARY=8` and `BP_CAST=8`
(`precedence.rs:14` and `:15`) give **two different semantics within the same set the same numeric
value**, and both are alive (used respectively for prefix binding in `nud.rs` and for cast in
`led.rs:81`). The second set collides with itself as well: `BP_RANGE=7` and `BP_ADD=7`.

**The system does not currently fail because of an undocumented convention**: almost every infix
operator uses `bp_right = bp_left + 1`, so the right operand accepts only operators with **strictly
higher** binding power. This actually produces **left associativity** (`a - b - c` → `(a-b)-c`).
Therefore, the only effective comparison, “compare `bp_left`,” does not fail merely because two
operators have equal binding power. `led.rs:85` uses `(11, 1)` for `FatArrow`, with `bp_right` lower
than `bp_left`; this is the only explicit “right operand greedily consumes” form (the lambda body
extends as far right as possible).

This is an **accidental correctness**: it depends on the facts that “the `bp_left` returned by
`infix_info` is unique” and that “`parse_expression_internal:100` compares only `bp_left`.” If any
location is changed to use a genuine `bp_left == bp_right` test to determine associativity, the
collision will immediately become visible.

#### Three Bare Magic Numbers

| Magic number | Location                             | Problem                                                                                                                  |
| ------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `(6, 7)`     | `led.rs:40` `TokenKind::DotDot`      | Literal binding-power pair with no names; it also **does not use `BP_RANGE` from the same set**, leaving `BP_RANGE` dead |
| `(11, 1)`    | `led.rs:85` `TokenKind::FatArrow`    | 11 **exceeds the upper bound of either ladder** (the first set has `BP_HIGHEST=10`)                                      |
| `12`         | `nud.rs:1041` `parse_expression(12)` | **Greater than `BP_HIGHEST=10`**; used for RFC-010b pattern probing                                                      |

The comment at `nud.rs:896-901` explicitly explains the origin of this magic number: _"Lambda
binding power is 11, so we use 12 to stop before =>"_ and _"parse_expression(12) will stop `ok` as a
bare Var and turn `(` into an unexpected token."_ In other words, **the 12 was found experimentally
rather than derived**.

#### Dispatch Is a Handwritten `match`

**Dispatch mechanism**: everything is a handwritten `match` that returns function pointers, with
**no macros, tables, or generators**.

- `parse_prefix` (`nud.rs:14-19`) obtains the `prefix_info()` closure;
- `parse_expression_internal` (`pratt/mod.rs:79-113`) loops over the
  `(bp_left, bp_right, parser_fn)` tuple returned by `infix_info()`;
- The two handwritten tables are `infix_info` (`led.rs:30-88`) and `prefix_info` (`nud.rs:26-75`).

#### `parse_assign_after_target`: A Misleadingly Named 507-Line General Dispatcher

The `parse_assign_after_target` function at
`src/frontend/core/parser/statements/declarations.rs:136` has a body extending to `:642` (**507
lines**, in a file of 1,015 lines).

Despite its name, “assign after target” is the **general dispatcher for declaration forms**. It
contains eight independent responsibility sections: old-syntax detection / three-level nested
lookahead for annotation disambiguation / declaration legality / semantic side effects / type-body
parsing fallback / RFC-010 lambda signature parameter reconstruction / MetaType definition /
ordinary initialization.

Its growth comes from three overlapping causes:

**Cause 1: approximately 120 lines of dead language features.**

These are `is_old_function_syntax` (`declarations.rs:82-117`), `skip_old_function_syntax`
(`declarations.rs:120-122`), and the supporting lookahead at `:145-167`.

The precise explanation is important: **these two functions are not unreferenced**. They are called
from `declarations.rs:719` and `:725` in `parse_identifier_stmt`. The path is: “detect `identifier(`
followed by type parameters and `->` → report ‘old syntax is deprecated’ → skip → return None.”

The accurate statement is therefore: **this is a still-running gate that rejects removed syntax**,
with approximately 120 lines of overhead spent performing a balanced-parenthesis scan for every
statement beginning with `identifier(`.

Among these:

- `skip_old_function_syntax` (`:120-122`) has **a body consisting only of a one-line comment**,
  `// The old syntax has been removed; this function is no longer needed`, and is a **completely
  empty function**;
- More notably, after calling it (`:725-726`), the code directly executes `return None` **without
  consuming any token**. In other words, this “skip” **does not actually skip anything**—the
  subsequent tokens remain in the stream and can only be handled by the caller’s error recovery. The
  interface does not match its name.

**Cause 2: lookahead logic has not been abstracted.** “Skipping balanced parentheses” is
**handwritten four times** in the same file:

| Location                           | Line range                |
| ---------------------------------- | ------------------------- |
| Inside `is_old_function_syntax`    | `declarations.rs:96-105`  |
| Inside `parse_assign_after_target` | `declarations.rs:147-156` |
| Same                               | `declarations.rs:191-200` |
| Same                               | `declarations.rs:224-233` |

Each implementation is about 12 lines and has the same structure: `paren_depth = 1` →
`while paren_depth > 0 && !at_end()` → increment on `LParen`, decrement on `RParen` → `bump()`.
**The four implementations share no code.**

**Cause 3: type-layer logic has leaked into the parser.** See routing table C of RFC-039:
`declarations.rs:27-80` calls the type-layer functions `is_type_param_annotation` and
`name_used_as_type`; `declarations.rs:498-509` hard-codes the string `"Terminates"` and calls
`typecheck::operator_interfaces::spec()`. **This belongs to 03-type-unification.md /
02-stage-contract.md. This article does not duplicate its handling and only marks the boundary in
the “Detailed Design” section.**

#### Complete AST Enumeration

`src/frontend/core/parser/ast.rs`:

| Enum       | Location         | Variant count          |
| ---------- | ---------------- | ---------------------- |
| `Expr`     | `ast.rs:16-163`  | 22 (including `Error`) |
| `BinOp`    | `ast.rs:191-213` | 20                     |
| `UnOp`     | `ast.rs:217-223` | 4                      |
| `StmtKind` | `ast.rs:234`     | 9 (including `Error`)  |
| `Type`     | `ast.rs:427-541` | 26                     |
| `Pattern`  | `ast.rs:794-814` | 8                      |

`Expr::Error` has been verified to exist (its span is handled at `pratt/mod.rs:72`, and it also
appears at `ast.rs:1048`); `StmtKind::Error` is constructed at `parser/mod.rs:50`. **These two error
placeholders are foundational to the backward-compatible design and must be retained**; see
“Detailed Design · Backward Compatibility.”

### The Change Footprint of Adding an Operator

**The downstream change footprint is much larger than expected.** `BinOp::` has **534 hits across 47
files** in the whole repository, including **17 production files**. More importantly, these files
consume **two unrelated `BinOp` enums**:

| Enum                | Definition                | Variant count                                   | Production consumers (the number in parentheses is the number of `BinOp::` hits in that file)                                                                                                                                            |
| ------------------- | ------------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ast::BinOp`        | `parser/ast.rs:191-213`   | 20                                              | `led.rs`(22) `nud.rs`(1) `checker.rs`(8) `inference/expressions.rs`(25) `inference/statements.rs`(5) `operator_interfaces.rs`(5) `ir_gen.rs`(23) `formatter/handlers/expr.rs`(25) `lsp/handlers/inlay_hint.rs`(4) `spawn/analysis.rs`(3) |
| `const_data::BinOp` | `types/const_data.rs:234` | Verified to exist (uses `Ne` rather than `Neq`) | `const_data.rs`(36) `const_eval.rs`(64) `evaluator.rs`(13) `ownership.rs`(13) `termination.rs`(16) `proof/smt/translate.rs`(13) `proof/dep_graph.rs`(2)                                                                                  |

It has been verified that `src/backends/` has **zero hits** for either `BinOp` or `ast::`—the
interpreter consumes IR/bytecode and never sees the AST. Therefore, all 17 production files lie on
the L2 → L3 path.

**There is no `From`/`TryFrom` between the two enums**—whole-repository greps for
`impl From<...BinOp`, `-> BinOp`, and `BinOp as` find no conversion implementation. The cost is that
**every file that touches both must alias them at the import site**: `const_eval.rs:19`
(`BinOp as AstBinOp`), `ir_gen.rs:2317` (`use ast::BinOp as B`), and the test-file aliases `CEBinOp`
/ `ConstBinOp`.

> **This is the most important finding in this document.** When `ast::BinOp` gains a variant, **the
> compiler cannot remind you to update `const_data::BinOp`**, because they are independent types
> that can each compile successfully on their own. If the update is missed, the constant-folding
> path does not understand the new operator, while the type-checking and IR-generation paths
> do—**the same operator behaves differently in two subsystems, with no diagnostic whatsoever**.
>
> This is an order of magnitude more serious than “missing one `match` arm.” The latter is caught by
> exhaustive matching; the former cannot be caught.

**“No one can discover a missed update” needs to be stated precisely**:

| Kind of missed update                                    | Detected by the compiler? | Verified evidence                                                                                                                                                            |
| -------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Missing arm in an exhaustive `match`                     | **Yes**                   | `impl Display for BinOp` at `const_data.rs:291-317` is an 18-arm exhaustive match with no `_` fallback                                                                       |
| Missing item in a `matches!` macro enumeration           | **No**                    | The `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` functions at `const_data.rs:263-288` are all partial `matches!` lists; a new variant silently returns `false` |
| Missing arm in a `match` with a `_` fallback             | **No**                    | The `_ => {}` at `ownership.rs:687` silently ignores a new variant                                                                                                           |
| Missing update across `ast::BinOp` / `const_data::BinOp` | **Never**                 | They have no conversion and no common parent type, so the compiler has no way to relate them                                                                                 |

The accurate statement is therefore not “11 locations can be missed with nobody noticing,” but:
**the compiler can cover the exhaustive-match portion; what actually fails silently is the
`matches!` macro, `_` fallbacks, and cross-enum updates.**

### Dead Code and Current Test State

This is **the item that most urgently requires action**.

#### Dead Code

- The `Precedence` enum (`precedence.rs:35-94`, 60 lines) and `PrecedenceContext` struct
  (`precedence.rs:98-133`, 36 lines): zero production uses; their only external referrer is the
  orphan file `pratt/tests/precedence_inline.rs` (95 lines / 6 tests), which is not declared by
  `pratt/tests/mod.rs:4-6`.
- `skip_old_function_syntax` (`declarations.rs:120-122`): an empty function; its call site at `:725`
  is followed directly by `return None`, and it consumes no token.
- `BP_RANGE` (`precedence.rs:22`): zero production uses; it survives only in the comment at
  `led.rs:157-159`.

#### Only One Lexer Test File Is Running

**The only test declaration in `lexer/mod.rs:104-106` is:**

```rust
#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
```

A `grep 'mod tests'` over the entire `src/frontend/core/lexer/` directory matches only this
location. **The `lexer/` directory has never declared `mod tests;`.**

The entire `src/frontend/core/lexer/tests/` subtree has therefore **never participated in
compilation**. The directory actually contains **14 files** (including `mod.rs`):

| File                                                                              | Lines          | Status                                      |
| --------------------------------------------------------------------------------- | -------------- | ------------------------------------------- |
| `mod.rs`                                                                          | 38             | Present; declares 11 child modules          |
| `literals.rs`                                                                     | 222            | Real tests, **orphaned**                    |
| `rfc010_lexer.rs`                                                                 | 167            | Real tests, **orphaned**                    |
| `fstring.rs`                                                                      | 90             | Real tests, **connected through `#[path]`** |
| `lexer_mod.rs`                                                                    | 89             | Real tests, **doubly orphaned**             |
| `rfc004_lexer.rs`                                                                 | 81             | Real tests, **orphaned**                    |
| `symbols.rs`                                                                      | 70             | Real tests, **doubly orphaned**             |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 1–3 lines each | Seven empty shells                          |
| `debug_lexer.rs`                                                                  | 1              | Empty shell                                 |

**Real tests that have never run: 629 lines / 5 files** (222+167+89+81+70). The only lexer test file
actually running is the f-string file.

**A wiring trap that must be stated clearly: `tests/mod.rs` exists, but it is not fully wired
itself.**

`lexer/tests/mod.rs:15-25` declares 11 child modules (`basic` / `literals` / `operators` /
`delimiters` / `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer`
/ `fstring`), and `:28-38` then `pub use`s all of them for “backward-compatible re-export.”

**However, it does not declare `lexer_mod` or `symbols`.** Therefore:

> **Adding only one line, `mod tests;`, is not enough.** Reviving `lexer/tests/` requires **two
> changes**: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` to
> `tests/mod.rs`. Otherwise, these 159 lines still do not run.

**The trap of the seven empty shells**: statements such as `pub use basic::*;` in
`tests/mod.rs:28-38` **do not produce any compilation error or warning when they refer to a module
with zero assertions**. After adding `mod tests;`, CI will report that all tests pass even though
the `basic` / `comments` / `delimiters` / `errors` / `keywords` / `operators` / `debug_lexer`
modules **contain not a single assertion**. **This creates the false impression that the lexer layer
is covered and is more dangerous than leaving the tests disconnected.**

**The highest-risk uncovered paths** (immediately exposed once the tests are connected):

| Path                                                                                     | Location                                                                          | Risk                                |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ----------------------------------- |
| Overflow paths of the four base scanners                                                 | `literals.rs:134-141` / `220-227` / `306-313` + the corresponding decimal section | Integer overflow is a silent error  |
| The “continue consuming without reporting an error” branch after `checked_mul` overflows | 4 isomorphic locations                                                            | May cause token-stream misalignment |
| The **entire** `scan_leading_dot` function                                               | `literals.rs:519-635` (117 lines)                                                 | Zero coverage                       |
| Invalid `\x` / `\u` escape paths                                                         | `literals.rs:730-749` / `1051-1070`                                               | Error paths are difficult to test   |

**The parser side is wired correctly**: `parser/mod.rs:8-9`, `pratt/mod.rs:8-9`, and
`statements/mod.rs:11-12` all correctly declare `mod tests;`. `pratt/tests/mod.rs:4-6` declares
`led` (335 lines), `nud` (269 lines), and `precedence` (13 lines).

**However, parser-side coverage has the same problem**:

- The only assertion in `pratt/tests/precedence.rs` (**13 lines**) is
  `assert!(bp_lowest < bp_highest)`, which is **almost a no-op**—it cannot detect any of the four
  cross-set collisions listed above.
- `pratt/tests/precedence_inline.rs` (**95 lines / 6 tests**) is not declared by
  `pratt/tests/mod.rs` and **has never run**—and it tests only the zero-reference `Precedence` enum
  and `PrecedenceContext`.
- **The two ladders actually in use (`BP_OR`…`BP_MUL` and
  `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`) have no direct unit tests.**

## Target Design

### Lexing: Declarative Form

#### Evaluation of Three Options

| Option                                                         | Cost of adding a token         | Can eliminate the lexer layer’s three duplications? | Can solve f-string nested compilation?      | Introduced dependency                                                          |
| -------------------------------------------------------------- | ------------------------------ | --------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| **A. Generate a DFA with `logos`**                             | 1 location (token declaration) | **Yes** (one copy each for escapes/numbers)         | No (interpolation must be handled manually) | Adds `logos`                                                                   |
| **B. Handwrite a DFA with `regex`**                            | 1–2 locations                  | Partially                                           | No                                          | Adds `regex` (whether it is already a transitive dependency is **unverified**) |
| **C. Keep handwritten code and enforce merging of duplicates** | 1 location                     | **Yes** (manually extract common functions)         | **Yes** (requires explicit design)          | None                                                                           |

**Claim: adopt C as the baseline, retain A as a conditional alternative, and do not adopt B.**

Rationale:

1. **C can eliminate all approximately 500 lines of duplication in the lexer layer with no new
   dependency**, entirely within a controlled scope. The three copies of escape
   decoding—`scan_string` / `scan_char` / `push_fstring_escape`—are merged into a **single**
   `decode_escape(lexer, &mut String, EscapeMode)`, and the four base scanners into a **single**
   `scan_radix_number(lexer, Radix, String)`. This is a purely mechanical refactoring, judged by
   C5’s “identical AST snapshots + identical diagnostics.”
2. **A (`logos`) has real capabilities**—it can converge escape decoding and number scanning to one
   implementation each. **However, it cannot solve f-string nested compilation**, which is the lexer
   layer’s most important defect. It also introduces a new dependency that changes `Literals`’ error
   type (`LexError`’s `InvalidEscape` / `InvalidNumber` variants must be remapped), which would
   **directly violate C5’s “identical diagnostics” criterion** unless an additional adapter layer is
   written—and that would introduce yet another manual synchronization point.
3. **B (handwritten DFA with `regex`) is not adopted**: it requires a handwritten state machine,
   with workload comparable to A but far less controllability. If choosing between A and B, logic
   requires A; since this article advocates C first, there is no reason to choose B.

> **Design judgment (not a verified fact)**: A may be the better long-term form (**a declarative
> token table is itself a form of enforced “change one location”**). However, its migration cost and
> conflict with the “identical diagnostics” criterion make it unsuitable for this C5 change. If A is
> adopted later, it should be a separate proposal, using the test-revival results in this article as
> a safety net.

#### F-String Nested Compilation Must Be Addressed Explicitly

Regardless of whether A or C is selected, nested compilation must be resolved. **Claim: eliminate
runtime nested `tokenize()` and produce the result in a single lexical pass.**

Concretely: when `scan_string` encounters the `f"` prefix, it should **record the interpolation
contents as `FStringSegment::RawText(String)` during the same lexical scan** and **not** recursively
call `tokenize()`. The actual tokenization and parsing of the interpolation are deferred until the
parser first needs that f-string node, at which point:

- **All interpolations are processed at once** (for N interpolations, tokenize the interpolation
  segment only once and cache the result as `Vec<Vec<Token>>`), reducing N+1 passes to one;
- **Span mapping is determined during the lexical phase** (recording each interpolation’s absolute
  offset in the source file), rather than depending on the relative semantics of `Point`.

This change is **independent of the paradigm choice** and should be performed even if A is not
adopted.

### Parsing: An Unambiguous Language Surface + a Single Declarative Precedence Table

**Target form**: the parser has only one Pratt implementation; binding power and operator properties
are declared in only one declarative table; the language surface itself is unambiguous, so “adding a
binary operator” requires changing only the table, SPEC, and tests.

**Language surface (converged on 2026-11-25)**: SPEC syntax §2.8 / §2.9 / §2.10 / §3.1 / §3.2.1
agrees with the implementation—

- Match branch arrows use `->` (`=>` belongs only to lambda), and branches are separated by newlines
  or `,`;
- Statements are separated by newlines or `;`, and adjacent statements without a separator are
  syntax errors (E0012);
- Destructuring is written only as `(a, b) = e`; the bare-identifier form is not syntax;
- The bare single-parameter lambda `x => e` coexists with the parenthesized form.

**Precedence table**: `pratt/precedence.rs` is the sole declaration point—`INFIX_OPS` (27 lines = 25
single tokens + 2 double tokens) and `PREFIX_OPS` (6 lines) provide, line by line, the token, left
and right binding powers, `ast::BinOp`, and parsing strategy (`Binary` / `Assign` / `Shift` / `In` /
double token). `nud.rs` / `led.rs` only construct and dispatch; they do not hold operator-level
binding powers; `parse_assign` also obtains its operator from the table.

**Two `BinOp` representations**: the correspondence between `ast::BinOp` and
`types::const_data::BinOp` is derived from the table’s `op` field. The sole production mapping point
is `types/eval/const_eval.rs::convert_ast_binop`; on the test side,
`pratt/tests/operator_sources.rs` uses a cross-checking table to verify every operator and
exhaustively enumerate variants, preventing the two representations from drifting.

**Equivalence criteria**: full C5—corpus differential testing and IR snapshots are regenerated
together; diagnostic code + span must be identical item by item. Existing precedence defects (the
levels of `as` and `*`, `as` swallowing a following expression as its target type, and the levels of
`==` / `!=` and the comparison operators) are fixed in passing under D59 A/B/C: red evidence is
reproduced on the parent revision, and the green-state assertions are permanently placed in
`parser/tests/associativity.rs`.

**Boundary**: do not introduce a parser generator; retain handwritten lexing. Error recovery
(`synchronize()` / `Expr::Error` / `StmtKind::Error`) is outside the scope of this change.

### Disposition of `parse_assign_after_target`

**Claim: split it into eight named functions, with the original function body doing only dispatch.**

For a 507-line function with eight responsibility sections, every local modification requires
locating code within 500 lines. The target form after the split is:

| Extracted function             | Responsibility                                           | Approximate lines | Ownership                                                                              |
| ------------------------------ | -------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------- |
| `probe_old_function_syntax`    | Old-syntax detection                                     | 36                | **Deleted by this article**                                                            |
| `resolve_annotation_ambiguity` | Annotation disambiguation (three-level nested lookahead) | To be verified    | This article (substantially shortened after extracting a standalone lookahead utility) |
| `check_declaration_legality`   | Declaration legality                                     | To be verified    | This article                                                                           |
| `apply_semantic_side_effects`  | Semantic side effects                                    | To be verified    | **03-type-unification.md** (including the `"Terminates"` hard-coding at `498-509`)     |
| `parse_type_body`              | Type-body parsing fallback                               | To be verified    | This article                                                                           |
| `rebuild_lambda_params`        | RFC-010 lambda signature parameter reconstruction        | To be verified    | This article                                                                           |
| `parse_meta_type_def`          | MetaType definition                                      | To be verified    | This article                                                                           |
| `parse_plain_init`             | Ordinary initialization                                  | To be verified    | This article                                                                           |

Supporting change: consolidate the five occurrences of “skip balanced parentheses”
(`declarations.rs` four occurrences + `statements/types.rs:663`) into a single `ParserState` method,
`skip_balanced_parens()`.

**Net effect (design judgment)**: 507 lines → eight single-responsibility functions + one lookahead
utility; lookahead code falls from 48 lines to approximately 12; after type-layer logic is moved out
(approximately 80 lines), the portion actually owned by this article is approximately 300 lines.

### Dead-Code Cleanup (C6, No Equivalence Criteria Required)

Under C6 as defined in 07-equivalence-oracle.md (“pure deletion, no equivalence criteria required,
only confirmation that there are no references”):

| Cleanup item                                  | Location                                   | Lines | Prerequisite                                                                                                        |
| --------------------------------------------- | ------------------------------------------ | ----- | ------------------------------------------------------------------------------------------------------------------- |
| `Precedence` enum                             | `precedence.rs:35-94`                      | 60    | **Must handle `pratt/tests/precedence_inline.rs` at the same time** (its only referrer, itself an orphan)           |
| `PrecedenceContext` struct                    | `precedence.rs:98-133`                     | 36    | Same                                                                                                                |
| Six zero-reference constants in the first set | `precedence.rs:8-13`, `:11` (comment only) | 6     | Zero production references; delete directly                                                                         |
| `BP_RANGE`                                    | `precedence.rs:22`                         | 1     | Zero production references (survives only in the comment at `led.rs:157-159`); first make `..` use a named constant |
| `skip_old_function_syntax`                    | `declarations.rs:120-122`                  | 3     | Empty function; delete its call site at `declarations.rs:725` as well                                               |
| `is_old_function_syntax`                      | `declarations.rs:82-117`                   | 36    | See below                                                                                                           |
| `declarations.rs:145-167`                     | Same file                                  | 23    | Same                                                                                                                |
| Bare magic number `(6,7)`                     | `led.rs:40`                                | —     | Promote to named `BP_RANGE_L` / `BP_RANGE_R` constants                                                              |
| Bare magic number `(11,1)`                    | `led.rs:85`                                | —     | Promote to `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                            |
| Bare magic number `12`                        | `nud.rs:1041`                              | —     | Promote to `BP_PATTERN_PROBE`, and **record in a comment that it was found experimentally**                         |

> **Note**: `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` are live
> constants in the `precedence.rs` table and are maintained with that table. Phase 0 deletes only
> the `Precedence` enum and `PrecedenceContext`, not these constants.

**Disposition of `is_old_function_syntax`**: it is a still-running “reject removed syntax” gate
(`declarations.rs:689`). Its deletion is permitted only after the C5 criteria prove that diagnostic
codes and spans remain unchanged—the behavior of `f(Int) -> Int = x => x` must be measured before
and after deletion; retain the current implementation until that test has been performed.

### Test Rebuild

**This is the top priority and must precede every paradigm change.**

**Phase 1: revive `lexer/tests/`.** This requires **two changes**, not one:

1. Add `#[cfg(test)] mod tests;` after `:106` in `src/frontend/core/lexer/mod.rs`.
2. Add `mod lexer_mod;` and `mod symbols;` to `src/frontend/core/lexer/tests/mod.rs`; otherwise
   these 159 lines still do not run.

**Phase 2: handle the seven empty shells.** The key constraint is that statements such as
`pub use xxx::*;` at `tests/mod.rs:28-38` **do not produce any compilation error or warning when
referring to empty modules**. Once connected, CI will show “all green” while the `basic` /
`comments` / `delimiters` / `errors` / `keywords` / `operators` / `debug_lexer` modules **contain no
assertions whatsoever**. **This creates the false impression that the lexer layer is covered and is
more dangerous than leaving the tests disconnected.**

| Option                | Action                                                                                | Assessment                                                                                                                                    |
| --------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| **Delete (proposed)** | Delete the seven empty-shell files + their `mod` declarations + their `pub use` lines | **Clean.** The only value of empty shells is “fill them in later,” and `literals.rs` / `rfc004_lexer.rs` already cover those responsibilities |
| Fill                  | Write real assertions for each empty shell                                            | Seven files × unknown contents; **it cannot be verified what should be tested** (unverified)                                                  |
| Keep + mark           | Retain them but add `// TODO` and a CI count warning                                  | Creates the false impression of coverage; **rejected**                                                                                        |

**Proposed action:** delete them, and at the same time as the dead-code cleanup, delete **all** the
`pub use` re-exports in `tests/mod.rs:28-38`. They existed to support a nonexistent re-export design
and are precisely what concealed the empty shells.

**Phase 3: add associativity cases.** Add `parser/tests/associativity.rs` (WBS 8.8.1), using only
the public entry points `parser::parse` / `parse_with_probe` to assert the **language-level AST
shape**: the cases do not reference Pratt internals (binding-power constants, the `pratt` submodule
path, or `ParserState`), so they are decoupled from Pratt internals and protect the C5 equivalence
surface, “the AST shape of valid programs remains unchanged”:

- Write one minimal expression for each pair of adjacent levels and assert its associativity
  direction (for example, `a - b - c` → `(a-b)-c`; see the level numbers and associativity column in
  SPEC syntax §2.2).
- Add one case each for `..` (`a..b..c` → `(a..b)..c`, with the upper bound of `0..n+2` swallowing
  the addition), right-greedy `=>` bodies, right-associative assignment, the `?` and call-chain
  suffixes, and pattern `|` chain closure—the binding-power pairs for `..` and `=>` are the two
  easiest to miss during this migration.
- **Do not use “binding powers strictly increase within a level, with no duplicates” as a gate
  assertion**: according to coding-rules 7.1, a correct implementation that reorders the numeric
  values within the same level would turn red. This freezes an implementation choice and may only be
  used as characterization; the level relationships are protected by the two groups of
  language-level AST shapes above.

**Phase 4: add literal error-path cases.** Add cases for each of the four categories in the
uncovered-paths table: overflow in all four bases, the branch that continues consuming after
`checked_mul`, `scan_leading_dot` as a whole, and invalid `\x` / `\u` escapes.

**Phase 5: delete `precedence_inline.rs`** along with the dead code, and add wiring self-check
assertions to `parser/tests/wiring.rs` (`mod` ↔ file, no orphans) to prevent orphan tests from
recurring.

### How Many Locations Must Change When Adding an Operator: Before and After

| #   | Current state                                                                       | Current enforcement                                 | Target                                             | Target enforcement                              |
| --- | ----------------------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------- | ----------------------------------------------- |
| 1   | Add a `match` arm to `keyword_from_str` in `lexer/state.rs:21` (keyword forms only) | None                                                | One line in the operator declaration table         | Compile time                                    |
| 2   | Add a variant to `TokenKind` in `lexer/tokens.rs:81-162`                            | Compile time                                        | **Generated from the table**                       | Compile time                                    |
| 3   | Add a BP constant to either `pratt/precedence.rs:6-17` or `22-31`                   | None                                                | **Generated from the table**                       | Compile time (intra-level uniqueness assertion) |
| 4   | Add an arm to `infix_info` in `pratt/led.rs:30-88`                                  | Compile time (exhaustive)                           | **Generated from the table**                       | Compile time                                    |
| 5   | Add an operator-mapping arm to `parse_binary` in `pratt/led.rs:116-136`             | Compile time (exhaustive)                           | **Generated from the table**                       | Compile time                                    |
| 6   | Add a variant to `BinOp` in `ast.rs:191-213`                                        | Compile time                                        | **Generated from the table**                       | Compile time                                    |
| 7   | New symbol character: `next_token_inner` in `tokenizer.rs:218`                      | None                                                | **Generated from the table**                       | Compile time                                    |
| 8   | Add a variant to `const_data::BinOp` in `const_data.rs:234`                         | **None (cross-enum; the compiler cannot catch it)** | **Generated from the same source as `ast::BinOp`** | Compile time                                    |
| 9   | Sites in 17 downstream production files that depend on `matches!` / `_` fallbacks   | **None (silent)**                                   | Generated accessors                                | Compile time                                    |

**Summary:**

| Stage                            | Manual change locations                                                                                             | Automatically generated locations |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| Current state                    | **11–12** (6–7 within L2 + approximately 5–6 downstream production files that must be synchronized)                 | 0                                 |
| Target (new symbol operator)     | **1–2** (one line in the operator declaration table; for a keyword form, one additional line in `keyword_from_str`) | 7–8                               |
| Target (reuse an existing token) | **1**                                                                                                               | 6–7                               |

Moreover, the target’s 1–2 locations are **all declarative data rows**, not logic code—the review
cost drops from “reason about the semantics of each `match` arm” to “read one declaration.”

## Detailed Design

### Interface Boundaries with the Type System

**Boundary declaration (important)**: convergence of `ast::Type` (26 variants, `ast.rs:427-541`),
elimination of parallel type representations, and ownership of `is_type_param_annotation` /
`name_used_as_type` **all belong to [03-type-unification.md](03-type-unification.md)**. This article
handles only the three interfaces directly related to the frontend paradigm:

**Interface 1: the relationship between `const_data::BinOp` and `ast::BinOp`.** This belongs to
**this article** because it is a direct component of the “adding an operator” change footprint.
Disposition: **generate both enums from the operator declaration table** (or generate one and derive
the other), eliminating the import aliases in `const_eval.rs:19` and `ir_gen.rs:2317`. The reason
this belongs here rather than in 03 is that it is an L2 artifact copied in parallel by L3, not a
problem with the type representations themselves.

**Interface 2: the hard-coded `"Terminates"` and call to `typecheck::operator_interfaces::spec()` at
`declarations.rs:498-509`.** This belongs to **03-type-unification.md / 02-stage-contract.md**
(eliminating the parser → typecheck reverse dependency, the first row of RFC-039’s routing table C).
During the function split, this article **isolates this code into `apply_semantic_side_effects`** as
a handoff point, but does not change its contents.

**Interface 3: whether an operator’s precedence level requires type-layer information.** It does not
currently—`infix_info` is a purely syntactic function. **Design judgment**: after converting
precedence to a table, this property should be retained. The operator declaration table **must not
contain any type-related fields**, or L2 will once again depend on L3 in violation of RFC-039’s
dependency-direction rules.

### Runtime Behavior

**Target: runtime behavior remains bit-for-bit unchanged.** Under the C5 criteria in
07-equivalence-oracle.md, the third layer (end-to-end corpus differential testing) compares the exit
codes and stdout/stderr of the 293 `.yx` files in `tests/yaoxiang/` plus `src/std/tests/*.yx`.

The three **identified runtime impacts** must be handled explicitly:

| Change                                                                   | Runtime impact                                     | Disposition                                                                    |
| ------------------------------------------------------------------------ | -------------------------------------------------- | ------------------------------------------------------------------------------ |
| Merge f-string interpolation into one tokenization pass                  | **Performance improvement**, with unchanged output | Measure and record the elapsed-time benefit                                    |
| Generate BPs from the operator table / merge and reorder the two ladders | **May change associativity**                       | **Must write equivalence cases for every level pair** (Test Rebuild · Phase 3) |
| Delete `is_old_function_syntax`                                          | The diagnostic code/span for old syntax may change | Use corpus differential testing to prove they are identical                    |

**Explicit non-goals**: do not change the **semantics** of operator precedence. BP numbering may be
reordered to eliminate dead code and collisions, but relationships such as “multiplication binds
more tightly than addition” **must remain unchanged**.

### Compiler Change Checklist

File by file, with line numbers. C6 cleanup (Phase 0, independently committable; **8.0.1 / 8.0.2
were executed on 2026-11-22; the line numbers below refer to the tree before execution, and the
final state is described in the task 8.0 commit message**):

| File                                                        | Lines                                 | Change                                                                                                         |
| ----------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/pratt/precedence.rs`              | `8-13`, `11`                          | Delete the six zero-reference constants from the first set                                                     |
| Same                                                        | `22`                                  | Change `BP_RANGE` to a named `..` binding power (synchronized with `:40`), or delete it                        |
| Same                                                        | `35-94`                               | Delete the `Precedence` enum                                                                                   |
| Same                                                        | `98-134`                              | Delete the `PrecedenceContext` struct                                                                          |
| Same                                                        | `6-17`, `22-31`                       | Retain live constants and merge them with the second set into conflict-free numbering within each level        |
| `src/frontend/core/parser/pratt/led.rs`                     | `33`                                  | Point `BP_ASSIGN` to the reordered assignment-level constant                                                   |
| Same                                                        | `40`                                  | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R`                                                                          |
| Same                                                        | `75`, `77`, `79`, `83`                | Point `BP_CALL` to the reordered suffix-level constant                                                         |
| Same                                                        | `81`                                  | Point `BP_CAST` to the reordered cast-level constant                                                           |
| Same                                                        | `85`                                  | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                       |
| `src/frontend/core/parser/pratt/nud.rs`                     | `33`, `36`, `84`, `102`, `228`, `248` | Point `BP_UNARY` to the reordered prefix-level constant                                                        |
| Same                                                        | `39-70`                               | Point `BP_HIGHEST` to the reordered highest-level constant                                                     |
| Same                                                        | `1019`                                | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`, and add a comment explaining the value’s origin |
| `src/frontend/core/parser/statements/declarations.rs`       | `85-87`                               | Delete `skip_old_function_syntax`                                                                              |
| Same                                                        | `695`                                 | Delete its call site (deletion of `is_old_function_syntax` is covered in Phase 2)                              |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | Entire file                           | Delete (the orphan file that is the sole referrer of the dead code)                                            |

Supporting renames (the connected surface of the BP consolidation, approximately 89 locations):
`parser/mod.rs:113`, 8 locations in `statements/control_flow.rs`, `statements/declarations.rs:19` /
`287` / `577` / `705` / `785` / `821` / `940` / `972`, `statements/functions.rs:9` / `:107`,
`statements/bindings.rs:170`, `statements/types.rs:16` / `:782` / `:794`, `pratt/led.rs:99` / `:227`
/ `:315` / `:321` / `:409`, `pratt/nud.rs:200` / `:268` / `:418` / `:471` / `:478`.

Test revival (Phase 1, independently committable):

| File                                                 | Lines                   | Change                                                                       |
| ---------------------------------------------------- | ----------------------- | ---------------------------------------------------------------------------- |
| `src/frontend/core/lexer/mod.rs`                     | `167-168`               | Add `#[cfg(test)] mod tests;`                                                |
| `src/frontend/core/lexer/tests/mod.rs`               | `12-18`                 | Add `mod lexer_mod;` and `mod symbols;`                                      |
| Same                                                 | Entire file             | Delete the `mod` declarations and `pub use` lines for the seven empty shells |
| Same                                                 | Entire file             | Delete the `pub use ... ::*;` re-export block (`28-38`)                      |
| `src/frontend/core/lexer/tests/`                     | Seven empty-shell files | Delete the files                                                             |
| `src/frontend/core/parser/tests/wiring.rs`           | New                     | Add wiring self-check assertions (`mod` ↔ file, no orphans)                  |
| `src/frontend/core/parser/pratt/tests/precedence.rs` | `6-13`                  | Replace the no-op assertion with real associativity assertions               |

Paradigm changes (Phases 2–4):

| File                                                           | Change                                                                                                                                                                                                                                   |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/literals.rs`                          | Merge the four base scanners into `scan_radix_number(lexer, Radix, String)`; merge the three escape decoders into the sole `decode_escape(lexer, &mut String, EscapeMode)`; merge `scan_multi_line_string` and `scan_fstring_multi_line` |
| `src/frontend/core/parser/pratt/nud.rs:367` / `:398`           | Remove runtime nested `tokenize()` and instead produce the result in a single lexical pass                                                                                                                                               |
| `src/frontend/core/parser/pratt/led.rs:30-88` / `nud.rs:27-76` | Replace the two handwritten tables with generated mapping tables                                                                                                                                                                         |
| `src/ast/mod.rs:194-218`                                       | Generate `BinOp` and `const_data::BinOp` from the same source                                                                                                                                                                            |
| `src/frontend/core/types/const_data.rs:290`                    | Same                                                                                                                                                                                                                                     |
| `src/frontend/core/parser/statements/declarations.rs:101-620`  | Split into eight functions + `ParserState::skip_balanced_parens()` (replace the four occurrences at `61-70`/`112-120`/`156-164`/`186-194` with calls)                                                                                    |
| `src/frontend/core/parser/parser_state.rs`                     | Add `skip_balanced_parens()` (leave `synchronize()` at `:330` unchanged)                                                                                                                                                                 |

**Files with no changes under this article**: the **main bodies** of `src/middle/core/ir_gen.rs`,
`src/frontend/core/typecheck/**`, `src/backends/**`, and `src/frontend/core/formatter/**`. They
change only when generated accessors replace `matches!` macros, and those changes are mechanical
replacements.

### Backward Compatibility

Error recovery is not changed in this round. Three recovery contracts must be retained:

**Contract 1: `ParserState::synchronize()` (`parser_state.rs:170-186`).** It skips tokens forward
until a synchronization point after statement parsing fails. **The C5 criteria require retaining it
and require the timing at which it is triggered and the exact set of tokens it skips to remain
unchanged.** The function’s call sites and set of synchronization points remain unchanged.

**Contract 2: `Expr::Error` (`ast.rs:1048`, consumed at `pratt/mod.rs:72`) and `StmtKind::Error`
(constructed at `parser/mod.rs:50`).** These two placeholders allow the parser to continue
constructing a “partial AST” after an error, with downstream code skipping the `Error` branch. **The
variants must be retained, and their positions in downstream matches must also be retained**; if
downstream code places `Expr::Error` in a wildcard branch, errors will be silently swallowed.

**Contract 3: diagnostic code, span, and order.** The third-layer comparison rules in
07-equivalence-oracle.md state that “the diagnostic list is sorted by
`(code, span.file, span.line)`, and **message text does not participate in the comparison**.” This
gives this article some latitude over wording, but **codes and spans are immutable**.

**Special note on f-strings**: the interpolation change will alter spans from “relative to the
interpolation text” to “absolute offsets in the source file.” To satisfy Contract 3, **spans must be
recorded as absolute offsets during the lexical phase** (Decision D25)—`span.file/line` for
diagnostics inside an interpolation must not change because of this refactoring.

> **Disposition**: before Phase 2, establish a baseline snapshot of “diagnostic spans inside
> f-string interpolations” and compare it item by item after the change. Any mismatch in
> `span.file/line` is an implementation defect that must be fixed; there is no “accept the span
> change” option. (If testing proves that the original implementation’s interpolation spans were
> already wrong, the baseline-establishment phase will expose it—that is a pre-existing, independent
> defect, to be fixed separately and not merged into this phase, and it does not justify relaxing
> the criteria.)

## Implementation Essentials

This article corresponds to five phases. **Each phase can be committed and rolled back
independently.** Phases 0 and 1 are C6 and test revival and do not require equivalence criteria;
Phases 2–4 are C5 and are constrained by the criteria throughout.

| Phase | Content                                                                                                                                                | Category | Acceptance criteria                                                                                                                              | Rollback point                            |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- |
| **0** | Dead-code cleanup (`Precedence` enum + `PrecedenceContext`, 96 lines)                                                                                  | C6       | `cargo build` passes; `cargo clippy` has no new warnings                                                                                         | Single commit; `git revert` is sufficient |
| **1** | Test revival and completion                                                                                                                            | —        | The lexer layer actually runs tests from ≥ 5 files / 629 lines; the seven empty shells are deleted; associativity cases are **red before green** | Single commit                             |
| **2** | Converge lexical duplication + eliminate f-string nested compilation                                                                                   | C5       | Identical AST snapshots + identical diagnostics + identical behavior; `literals.rs` shrinks by approximately 500 lines                           | Commit separately from Phase 3            |
| **3** | Converge the syntax surface and precedence (SPEC canonical form, a single `pratt/precedence.rs` table, cross-checking the two `BinOp` representations) | C5       | Zero drift in corpus differential results and IR snapshots; diagnostic code+span unchanged; D1–D3 are red before green                           | Independent commit                        |
| **4** | Split `parse_assign_after_target`                                                                                                                      | C5       | The function only dispatches; there is a single implementation of `skip_balanced_parens`                                                         | Independent commit                        |

**Dependency ordering within this article**: Phase 1 **must precede** Phases 2/3/4—“refactor first,
add tests later” is a pitfall this project has already encountered. Phases 3 and 4 are independent
commits.

**One clarification about Phase 0**: `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST`
/ `BP_HIGHEST` are level constants in `precedence.rs` and belong to the precedence table. Phase 0
deletes only the `Precedence` enum and `PrecedenceContext` (96 lines with zero production
references).

**How the criteria are executed**: the definitions of and regression gates for the three-layer
criteria are in [07-equivalence-oracle.md](07-equivalence-oracle.md). **Everything uses the complete
C5 criteria, including diagnostic code+span. There are no relaxations.**

**Scope limitation**: this article **does not include** a `logos` migration (lexing remains
handwritten + duplicated logic is merged). The syntax surface and precedence-table convergence
belong to this article; see “Parsing: An Unambiguous Language Surface + a Single Declarative
Precedence Table.”

## Key Decisions and Rationale

### Three Selected Decisions

| Decision                         | Choice                                                                                                                                       | Rationale                                                                                                                                                                                                                                                        |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexing approach                  | Adopt “retain handwritten code + enforce merging of duplicates”; list `logos` as a conditional alternative                                   | Eliminates approximately 500 lines of duplication with no new dependency; `logos` changes the `LexError` type and cannot solve f-string nested compilation                                                                                                       |
| **Parsing approach**             | Retain Pratt; converge binding powers and operator properties into a single `pratt/precedence.rs` table; do not introduce a parser generator | The single table is the sole declaration point; “adding a binary operator” requires only the table + SPEC + tests; the two `BinOp` representations are derived from the same table’s `op` field and cross-checked; the error-recovery contracts remain unchanged |
| Ownership of `const_data::BinOp` | This article                                                                                                                                 | It is a direct component of the “adding an operator” change footprint and is handled separately from `ast::Type` convergence (03)                                                                                                                                |

### Delivery Form

- Binding powers and operator properties are declared in only one place, `pratt/precedence.rs`;
  `nud.rs` / `led.rs` only construct and dispatch.
- `parse_assign` / `parse_call` obtain their operator subtrees from the table rather than
  hard-coding them at call sites.
- The two `BinOp` representations are derived from the table’s `op` field and are cross-checked and
  exhaustively enumerated in tests.
- The syntax surface converges to the SPEC canonical form, with the corpus and documentation
  migrated together.
- Equivalence is guaranteed by the complete C5 criteria: corpus differential testing, IR snapshots,
  and diagnostic code + span.

## Known Limitations and Risks

### Design Risks

- **The operator table is a new “thing that must be maintained.”** It turns “change 11 locations”
  into “change one location + maintain a table.” **If the table’s fields are poorly designed, it can
  become a new centralized maintenance burden.** This is the main risk of the approach; the
  mitigation is to protect every column of the table with compile-time assertions.
- **The risk of reordering BP numbering is asymmetric.** Merging the two ladders entails renaming
  approximately 89 production references. If one is missed, **associativity silently changes**
  (there is no error; only the expression’s parse result differs). This is the silent regression
  most likely to be introduced by this article and must be covered by the level-pair associativity
  cases from Phase 1.
- **The C5 criteria are expensive in Phases 2–4.** Using all three layers means that every phase
  must run the full corpus (293 `.yx` files) + AST snapshots + diagnostic comparison. This makes
  every commit in Phases 2–4 relatively heavy.
- **The f-string span change may not be entirely avoidable.** See “Backward Compatibility”; it may
  be necessary to accept one known behavior change and update the baseline.

### Open Questions

> **All open questions originally listed in this section have been resolved.** The item-by-item
> decisions are recorded in
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> article leaves no pending items.**

## Verification Record

All code facts in this article were verified by **reading the files directly**. Verification method:
the `read` tool read the target file’s line ranges directly; line counts were obtained with
`Get-Content`; reference surfaces were scanned across the whole repository with `grep` and
classified separately as production / test / orphan. **`cargo test` / `cargo build` were not
used**—this is a pure document, and criteria execution belongs to the implementation phase.

### Item-by-item Measured Results

| Verification item                                    | Measured result                                                                                                                                                                                                                                                                                                                 |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Total lines in `literals.rs`                         | **1568**                                                                                                                                                                                                                                                                                                                        |
| Total lines in `tokenizer.rs`                        | **554**                                                                                                                                                                                                                                                                                                                         |
| Line number of `next_token_inner`                    | **`:218`**                                                                                                                                                                                                                                                                                                                      |
| Line ranges of the four base scanners                | `77-161` (85 lines) / `164-247` (84) / `250-333` (84) / `336-516` (181)—consistent with the table in this article                                                                                                                                                                                                               |
| Three escape-decoding implementations                | `694-827` (134 lines) / `1015-1141` (127 lines) / `push_fstring_escape:1374`—consistent with the table in this article                                                                                                                                                                                                          |
| `scan_leading_dot`                                   | `519-635`, **117 lines**                                                                                                                                                                                                                                                                                                        |
| Number of `Expr` variants                            | **22** (including `Error`), `ast.rs:16-163`                                                                                                                                                                                                                                                                                     |
| Number of files in `lexer/tests/`                    | **14** (including `mod.rs`)                                                                                                                                                                                                                                                                                                     |
| Orphaned real-test lines                             | **629** (222+167+89+81+70); another 90 lines in `fstring.rs` are already connected through `#[path]` and run, so they are not counted as orphaned                                                                                                                                                                               |
| Changes required to revive `lexer/tests/`            | **Two**: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` to `tests/mod.rs`. Adding only one leaves the final 159 lines not running                                                                                                                                                          |
| Dead code in `precedence.rs`                         | **96 lines** (`Precedence` enum `35-94` + `PrecedenceContext` `98-133`) with zero production uses is **confirmed**; however, their sole external referrer, `precedence_inline.rs` (95 lines / 6 tests), is itself an orphan—**deleting the dead code requires deleting that file at the same time**                             |
| Lines in the seven empty shells                      | 1–3 lines each (`debug_lexer.rs` has 1 line, the others 3)                                                                                                                                                                                                                                                                      |
| Shape of the blank-line residue                      | **Three consecutive blank lines inserted between every field, across 11 lines** (`777-787` / `1098-1108`)                                                                                                                                                                                                                       |
| Lines in `pratt/tests/precedence.rs`                 | **13**, with the sole assertion `assert!(bp_lowest < bp_highest)`                                                                                                                                                                                                                                                               |
| The fourth occurrence of “skip balanced parentheses” | `224-233`                                                                                                                                                                                                                                                                                                                       |
| Downstream change footprint                          | **17 production files** match `BinOp::` (whole repository: **534 hits / 47 files**)                                                                                                                                                                                                                                             |
| Is `src/backends/` a `BinOp` consumer?               | **No**—`src/backends/` has **zero hits** for `BinOp` or `ast::`; the interpreter consumes IR/bytecode and never sees the AST                                                                                                                                                                                                    |
| “No missed update can be detected”                   | **Must be tiered**: a missing arm in an exhaustive `match` **is detected by the compiler**; what fails silently is the `matches!` macro, `_` fallbacks, and **cross-enum** updates. This article states it according to the measured tiers                                                                                      |
| `matches!` blocks in `const_data.rs`                 | `263-288` (the four functions `is_arith` / `is_comparison` / `is_logical` / `is_bitwise`)                                                                                                                                                                                                                                       |
| `impl Display for BinOp`                             | Starts at `291`; 18-arm exhaustive match with no `_` fallback                                                                                                                                                                                                                                                                   |
| The two functions in `parse_assign_after_target`     | **Not dead code**: both functions are still called at `declarations.rs:719` / `:725` and form a running “reject removed syntax” gate; moreover, `skip_old_function_syntax` (`120-122`, with a body consisting only of a one-line comment) **consumes no token** after being called (`:725-726` directly executes `return None`) |
| The second `BinOp` enum                              | `const_data.rs:234` does exist and has **no `From` conversion whatsoever** to `ast::BinOp`, forcing aliases at import sites in files such as `const_eval.rs:19` and `ir_gen.rs:2317`                                                                                                                                            |
| Re-export block in `lexer/tests/mod.rs`              | Present (38-line file); `:28-38` contains `pub use ... ::*;`, directly causing the empty shells to masquerade as coverage                                                                                                                                                                                                       |

### Three Corrections Made During This Round of Verification

The following three points conflict with the intuition that “the first BP ladder is dead”; the
measurements take precedence:

1. **The two ladders are interwoven; neither is “one live and one dead.”** `infix_info`
   (`led.rs:30-88`) itself references both five constants from the first set (`BP_ASSIGN:33`,
   `BP_CALL:75/77/79/83`, `BP_CAST:81`) and eight from the second. Of the first set’s 12 constants,
   **only six truly have zero references** (`BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` /
   `BP_TERM` / `BP_FACTOR`, plus `BP_COMPARISON`, which survives only in the comment at
   `declarations.rs:742`); the other six (`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` /
   `BP_CAST` / `BP_HIGHEST`) have approximately **89 production references across 8 files**.
   Therefore, “delete the first set of BP constants” is not a 12-line C6 deletion; it is a
   cross-file rename in the C5 category.
2. **The second set has a dead member of its own.** `BP_RANGE` (`precedence.rs:22`) has zero
   production uses—`..` uses the bare literal `(6, 7)` at `led.rs:40`, and `BP_RANGE` appears only
   in the comment at `led.rs:157-159`.
3. **The “self-contradictory comment” is at `precedence.rs:33`, not `:11`.** `*:11` is
   `pub const BP_COMPARISON: u8 = 5;`. That comment (_“Precedence rules for the Pratt parser”_)
   describes the zero-reference `Precedence` enum.

Additional correction: `bp_right = bp_left + 1` produces **left associativity** (`a - b - c` →
`(a-b)-c`), not right associativity; `(11, 1)` at `led.rs:85` is the only explicit right-greedy
form. There are also collisions beyond the two cross-set cases: `BP_UNARY=8 == BP_CAST=8` (within
the first set, both alive) and `BP_RANGE=7 == BP_ADD=7` (within the second set).

### Unverified Items

The main text marks each of the following as “unverified”:

- The exact number of arms and the individual binding powers in the `prefix_info` table body
  (`26-75`) in `pratt/nud.rs`
- The precise line boundaries of the eight responsibility sections in `declarations.rs:136-642`
- The complete variant set of `const_data::BinOp` (`const_data.rs:234`)
- Whether `logos` / `regex` is already present in `Cargo.toml` as a transitive dependency
- All call sites of `synchronize()` outside `parser_state.rs:170-186`
- Whether this project is a published library, which affects the risk assessment for removing public
  API surface

## See Also

### Companion Design Documents in the Same Batch

- [01-routing.md](01-routing.md) — Where to change when adding a feature (functional routing tables
  A/B/C and dependency-direction rules)
- [03-type-unification.md](03-type-unification.md) — Ownership of `ast::Type` convergence and the
  interface boundaries with this article
- [04-ssa.md](04-ssa.md) — Another consumer of the C4/C5 criteria
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Complete inventory of the 8 orphan test trees
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — Definitions of the C5/C6 categories,
  three-layer criteria, and regression gates

### Main RFC Text and Related Proposals

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Governing framework; four-layer model, terminology, functional routing tables A/B/C
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Build-time gate for error codes; the enforcement model borrowed by this article
- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Origin of
  `ast::Type`

### Code Location Index

- `src/frontend/core/lexer/literals.rs:77-161` / `164-247` / `250-333` / `336-516` — Four base
  scanners
- `src/frontend/core/lexer/literals.rs:694-827` / `1015-1141` / `1374` — Three escape-decoding
  implementations
- `src/frontend/core/lexer/literals.rs:519-635` — `scan_leading_dot` (zero coverage)
- `src/frontend/core/lexer/mod.rs:104-106` — The lexer layer’s only test declaration
- `src/frontend/core/lexer/tests/mod.rs:15-25` / `28-38` — Child-module declarations and `pub use`
  re-export block
- `src/frontend/core/parser/pratt/nud.rs:441-446` — Runtime nested `tokenize()` for f-string
  interpolation
- `src/frontend/core/parser/pratt/nud.rs:896-901` / `1041` — Origin statement and use of the magic
  number `12`
- `src/frontend/core/parser/pratt/precedence.rs:6-17` and `22-31` — Two BP ladders
- `src/frontend/core/parser/pratt/precedence.rs:33` / `35-94` / `98-133` — Decoupled comment,
  zero-reference enum, and struct
- `src/frontend/core/parser/pratt/led.rs:30-88` — Handwritten `infix_info` table
- `src/frontend/core/parser/pratt/tests/mod.rs:4-6` — Orphan wiring caused by the missing
  `precedence_inline` declaration
- `src/frontend/core/parser/statements/declarations.rs:719-727` — Old-syntax rejection gate and its
  “skips without consuming” defect
- `src/frontend/core/types/const_data.rs:234` — The second operator enum parallel to `ast::BinOp`
- `build.rs:19-55` — Build-time error-code gate and an existing enforcement model in this repository
- `src/middle/core/tests/bytecode.rs:369-657` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “round-trip every item” criterion model in this repository
