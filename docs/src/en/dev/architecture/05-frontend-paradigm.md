# Frontend Paradigm: Lexical and Syntax

> **Companion Design Document**. This document is a companion to
> [RFC-039: Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution phase order are defined in the
> RFC-039 main text; the positioning of each companion document is described in
> [this directory index](index.md).

## Positioning and Scope

This document addresses the L2 frontend—**lexical (`src/frontend/core/lexer/`) and syntax
(`src/frontend/core/parser/`)**.

**Problem to be solved**: reduce the change surface for "adding a binary operator" from the current
**6-7 places within L2 + 17 downstream production files** (of which the compiler enforces 8-9, the
rest relies on manual synchronization) down to **1-2 declarations + 7-8 auto-generated places**.

This document addresses the pain point labeled on the first row "binary operator" in RFC-039's
feature routing table A. The refactoring category is **C5 frontend paradigm change** as defined in
[07-equivalence-oracle.md](07-equivalence-oracle.md) (criteria: identical AST snapshots + identical
diagnostics + identical behavior, all three used) and **C6 pure deletion** (no equivalence criteria
required, only need to confirm no references). C6 items are prerequisites for C5: delete 96 lines of
dead code, eliminate three bare magic numbers, and revive 629 lines of never-run lexical tests.

**Scope boundary**: The handling of `ast::Type` (26 variants) and the convergence of three parallel
type representations belong to [03-type-unification.md](03-type-unification.md); this document does
not address them redundantly, but only declares the interface boundary in the "Detailed Design"
section.

### Why This Is Worth Doing

"Adding an operator" is the most expensive single change in the frontend. This tax has **three
pernicious characteristics**:

**First, the cost does not decrease with experience.** Someone who has added ten operators will not
change one less place than someone who has done it once—because there is no derivable relationship
between these positions. The `(6,7)` at `led.rs:40` and the `12` at `nud.rs:1041` have no
connection, but they must be consistent with each other, or expression parsing will fail.

**Second, the consequences of missing a change fall into two categories, and the worst of them the
compiler cannot catch.** See the layered table in the "Change Surface for Adding an Operator"
section: an exhaustive `match` arm missed will be caught by the compiler, but **a cross-enum missed
change the compiler will never catch**.

**Third, this tax is invisible on the books.** Every commit in
`git log --oneline -- src/frontend/core/parser/pratt/led.rs` almost always touches 3-4 files
simultaneously, but no tool can point out "which place was missed in this commit".

### Existing Correct Paradigms in the Same Repository

This project has two **correct solutions for similar problems** that can be directly reused:

- **Error codes**: `build.rs:19-55` uses `tools/code-tables` to parse the registry, validate
  uniqueness, and compare each entry with the RFC-013 markdown code table at **build time**; **any
  inconsistency immediately `panic!`s and refuses to compile**. The cost of adding an error code is
  therefore a constant 1 place + documentation.
- **Standard library interfaces**: `gen_interfaces.rs` performs byte-by-byte comparison,
  `gen_docs.rs` detects marker range drift (last row of RFC-039 routing table B).

What these two mechanisms have in common is **turning "must modify synchronously" from human memory
into an executable assertion**. The entire goal of this document can be reduced to one sentence:
turn the correspondence between the operator table, the lexical rules table, and the AST variant
table into an executable assertion.

### Why "Just Deleting Dead Code" Is Not Enough

Dead code does exist (`precedence.rs` 96 lines, the `skip_old_function_syntax` empty function, 629
lines of orphan tests), but after deleting them, "adding an operator changes a dozen places" will
not decrease by even one. Dead code is **stock debt**; the cross-file change surface is **structural
cost**. The two must be handled separately (see "Key Decisions and Rationale").

## Current State

All of the following are **verified facts**, each with file path and line number. The verification
method and per-item verification results are in "Verification Records".

### Lexical Layer: Copy-Paste Evidence in `literals.rs`

`src/frontend/core/lexer/literals.rs` is **1568 lines in total**. This size does not come from
complexity, but from three structural duplications.

**Duplication One: Four radix scanners are nearly line-by-line identical.**

| Scanner               | Line Range            | Lines |
| --------------------- | --------------------- | ----- |
| `scan_hex_number`     | `literals.rs:77-161`  | 85    |
| `scan_octal_number`   | `literals.rs:164-247` | 84    |
| `scan_binary_number`  | `literals.rs:250-333` | 84    |
| `scan_decimal_number` | `literals.rs:336-516` | 181   |

The first three are each about 84 lines, with completely identical structure: accumulate digit →
`has_digits` check → `overflow` flag → `checked_mul(base)` → `checked_add` → `try_into` → report
error. **The only difference is the base constant and the digit extraction expression.** Out of
three copies × 84 lines, **about 250 lines are pure redundancy** (`scan_decimal_number` needs to
handle decimal point and exponent additionally, so it cannot be merged, hence listed separately).

Evidence sampling—the overflow path of `scan_hex_number` (`literals.rs:134-141`):

```rust
if overflow {
    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
        value,
        point(lexer),
    ));

    return Some(lexer.make_token(TokenKind::Error("Hex number too large".to_string())));
}
```

`scan_octal_number:220-227`, `scan_binary_number:306-313` are the same code block with different
base and message text.

**Duplication Two: Escape decoding is implemented 3 times.**

| Implementation               | Location                   | Length                                     |
| ---------------------------- | -------------------------- | ------------------------------------------ |
| `\\` branch of `scan_string` | `literals.rs:694-827`      | 134 lines                                  |
| `\\` branch of `scan_char`   | `literals.rs:1015-1141`    | 127 lines                                  |
| `push_fstring_escape`        | `literals.rs:1374` onwards | Already extracted into a separate function |

The first two are near-line-by-line copies of 127-134 lines (`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` → push
the same, `\x` / `\u{...}` logic the same, `c =>` error fallback the same), **about 250 lines
written twice**.

It is worth pointing out separately: **the f-string version has already been extracted into a
separate function** (`push_fstring_escape:1374`). This shows that **someone has already realized
this duplication and made a partial fix, but did not go back to converge the other two places**.
This is more noteworthy than "no one realized all three places"—it proves the problem is solvable;
it just lacks an enforcement mechanism.

**Duplication Three: Multi-line string scanning has two copies.** `scan_multi_line_string:869` and
`scan_fstring_multi_line:1497` are again two similar implementations.

**Abnormal formatting residue.** `literals.rs:777-787` and `1098-1108`, each 11 lines, in the form
of **3 consecutive blank lines inserted between each field of a struct literal**. This is automated
rewrite residue. Other positions in the same function (`1111-1115`) only have a single blank line
between fields, indicating that formatting was interrupted.

> Such residue itself is harmless, but it is a **diagnostic signal**: the same place has been
> rewritten by different tools twice, meaning that historically someone has made a partial fix here
> without pulling the context together.

**Conclusion**: about 500 lines in `literals.rs` (250 lines of radix scanner redundancy + 250 lines
of duplicate escape decoding) are **mechanical duplication**, not complexity.

**f-string interpolation triggers nested compilation.** This is **runtime repeated compilation**,
not repeated implementation, so it cannot be solved by merging functions.

`src/frontend/core/parser/pratt/nud.rs:441-446`:

```rust
let tokens_result = crate::frontend::core::lexer::tokenize(expr_str_trimmed);
match tokens_result {
    Ok(tokens) => {
        let mut parser = crate::frontend::core::parser::ParserState::new(&tokens);
        if let Some(expr) =
            parser.parse_expression(crate::frontend::core::parser::pratt::BP_LOWEST)
```

**The parser layer runs a full `tokenize()` for each f-string interpolation at runtime, and creates
a new `ParserState`.**

Consequence: an f-string with N interpolations **triggers N+1 full lexical analyses** (N
interpolations + 1 outer). At the same time, each interpolation is an **independent parsing
context**, so:

- The diagnostic span within the interpolation is relative to the interpolation text, and remapping
  to the source location requires additional work;
- The interpolation cannot access any outer parser state (no current need, but structurally closes
  off future possibilities, such as referencing outer implicit variables within the interpolation);
- The locations where lexical and syntax errors are produced are split across two stages.

This is a **dual defect in performance and structure**, and must be addressed (see "Target Design ·
Lexical").

### Syntax Layer: Two BP Ladders, Bare Magic Numbers, Hand-Written Dispatch Tables

#### The Two Ladders Are Not "One Alive, One Dead", but Intertwined

`src/frontend/core/parser/pratt/precedence.rs` contains **two binding power ladders**
simultaneously, with large overlaps in numbering.

**First set (`precedence.rs:6-17`)**: `BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` /
`BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` /
`BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`.

**Second set (`precedence.rs:22-31`)**: `BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` /
`BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`.

**Empirical conclusion: both sets are alive in production code, and they are already intertwined
from within `infix_info`.**

- `infix_info` (`led.rs:30-88`) references **5 constants of the first set**: `BP_ASSIGN` (`:33`),
  `BP_CALL` (`:75` / `:77` / `:79` / `:83`), `BP_CAST` (`:81`);
- References **8 constants of the second set**: `BP_OR` (`:42`), `BP_AND` (`:44`), `BP_EQ` (`:47`),
  `BP_SHIFT` (`:51` / `:54`), `BP_BIT` (`:58`), `BP_CMP` (`:62` / `:65`), `BP_ADD` (`:68`), `BP_MUL`
  (`:72`);
- **The only dead member of the second set is `BP_RANGE`**—`..` at `led.rs:40` uses the bare literal
  `(6, 7)` rather than `BP_RANGE`; `BP_RANGE` only appears in production code in the comment at
  `led.rs:157-159`.

The survival status of the 12 constants in the first set (results of `grep` across the entire
repository):

| Status              | Constant                                                                     | Production References                                                                                                                                                                          |
| ------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Dead (6)**        | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | Zero production hits. Only appear in `precedence.rs` itself and the orphan test `precedence_inline.rs`                                                                                         |
| Dead (comment-only) | `BP_COMPARISON`                                                              | Zero production hits; only mentioned in the comment at `declarations.rs:742`                                                                                                                   |
| **Alive (6)**       | `BP_LOWEST`                                                                  | `parser/mod.rs:80`, `statements/control_flow.rs` 9 places, `statements/declarations.rs` 6 places, `pratt/led.rs:229`/`:321`/`:327`/`:415`, `pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523` etc. |
|                     | `BP_ASSIGN`                                                                  | `led.rs:33`, `led.rs:99`, `statements/declarations.rs:18` (import), `:743` (`BP_ASSIGN + 1`)                                                                                                   |
|                     | `BP_UNARY`                                                                   | `nud.rs:32`, `:35`, `:83`, `:101`, `:223`, `:243`                                                                                                                                              |
|                     | `BP_CALL`                                                                    | `led.rs:75`, `:77`, `:79`, `:83`                                                                                                                                                               |
|                     | `BP_CAST`                                                                    | `led.rs:81`                                                                                                                                                                                    |
|                     | `BP_HIGHEST`                                                                 | `nud.rs:38-69` about 20 places (one for each literal and keyword prefix)                                                                                                                       |

**Total: the 6 alive constants have about 89 references in production code, spanning 8 files.**

> This corrects an easy conclusion to draw: "delete the first BP ladder" is **not** a 12-line
> deletion, but a rename and merge of 89 references across 8 production files. Only 6 constants in
> the first set truly have zero references (plus `BP_COMPARISON` which only exists in a comment).
> The second set is also incomplete—`BP_RANGE` is dead.

**The truly zero-referenced dead code is those two types**: the `Precedence` enum
(`precedence.rs:35-94`, 60 lines) and the `PrecedenceContext` struct (`precedence.rs:98-133`, 36
lines), totaling **96 lines**. Verification result: these two types have zero use in production
code—`grep` for `PrecedenceContext` and `Precedence::` across the repository, apart from
`precedence.rs` itself, all hits fall in
`src/frontend/core/parser/pratt/tests/precedence_inline.rs`.

**There is a detail that must be written into the documentation**: the only file that references the
dead code, **is itself an orphan**. `pratt/tests/mod.rs:4-6` only declares
`mod led; mod nud; mod precedence;`, **without declaring `precedence_inline`**.

> That is: the reason 96 lines of dead code "appear to have test coverage" is that it is tested by a
> 95-line / 6-test file, and that file has never participated in compilation (see "Dead Code and
> Test Status" for details). **Deleting dead code must also handle this orphan test file**,
> otherwise compilation will fail after deletion.

In addition, `precedence.rs:33` has a self-contradicting comment as evidence: _"Precedence rules for
the Pratt parser"_—this comment describes the zero-reference `Precedence` enum, whose ladder is not
the same as the actually effective ladder. The enum and implementation have already diverged.

#### Numerical Collisions

The overlapping ranges of the two ladders almost correspond digit-by-digit, so **cross-set
collisions occur in pairs**:

| Conflict                    | Value    | Use Point A (First Set)        | Use Point B (Second Set)       |
| --------------------------- | -------- | ------------------------------ | ------------------------------ |
| Assignment vs Logical Or    | `1 == 1` | `BP_ASSIGN` (`led.rs:33`)      | `BP_OR` (`led.rs:42`)          |
| Type Cast vs Multiplication | `8 == 8` | `BP_CAST` (`led.rs:81`)        | `BP_MUL` (`led.rs:72`)         |
| Shift vs Term               | `6 == 6` | `BP_TERM` (dead)               | `BP_SHIFT` (`led.rs:51`/`:54`) |
| Bitwise vs Comparison       | `5 == 5` | `BP_COMPARISON` (comment-only) | `BP_BIT` (`led.rs:58`)         |

**The first set also self-collides internally**: `BP_UNARY=8` and `BP_CAST=8` (`precedence.rs:14`
and `:15`)—**two different semantics within the same set share the same value**, and both are alive
(used for prefix binding in `nud.rs` and cast in `led.rs:81` respectively). The second set also
self-collides internally: `BP_RANGE=7` and `BP_ADD=7`.

**The reason it has not blown up so far relies on an unstated convention**: almost all infixes use
`bp_right = bp_left + 1`, so the right operand only accepts **strictly higher** binding power
operators—this actually produces **left-associativity** (`a - b - c` → `(a-b)-c`). Therefore, the
only actually effective comparison "compare `bp_left`" will not go wrong due to two operators having
the same bp. The `FatArrow` at `led.rs:85` uses `(11, 1)`, where `bp_right` is lower than `bp_left`,
being the only explicit "right operand greedy swallowing" writing style (lambda body maximizes to
the right).

This is a **coincidental correctness**: it depends on the fact that "the `bp_left` returned by
`infix_info` is unique, and `parse_expression_internal:100` only compares `bp_left`". Any change to
truly use `bp_left == bp_right` to determine associativity will immediately expose the conflict.

#### Three Bare Magic Numbers

| Magic Number | Location                             | Problem                                                                                                                   |
| ------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| `(6, 7)`     | `led.rs:40` `TokenKind::DotDot`      | Literal binding power pair, unnamed; and **does not use `BP_RANGE` from the same set**, making `BP_RANGE` a dead constant |
| `(11, 1)`    | `led.rs:85` `TokenKind::FatArrow`    | 11 **exceeds any upper limit of both ladders** (first set `BP_HIGHEST=10`)                                                |
| `12`         | `nud.rs:1041` `parse_expression(12)` | **Greater than `BP_HIGHEST=10`**, used for RFC-010b pattern probing                                                       |

The comment at `nud.rs:896-901` frankly admits the origin of this magic number: _"Lambda binding
power is 11, so we use 12 to stop before =>"_, _"parse_expression(12) will stop `ok` as a bare Var,
`(` as an unexpected token"_. In other words, **this 12 was found by trial**, not derived.

#### Dispatching is Hand-Written `match`

**Dispatch mechanism**: all are hand-written `match`es returning function pointers, **no macros, no
tables, no generators**.

- `parse_prefix` (`nud.rs:14-19`) takes the `prefix_info()` closure;
- `parse_expression_internal` (`pratt/mod.rs:79-113`) loops taking the
  `(bp_left, bp_right, parser_fn)` triples from `infix_info()`;
- Two hand-written tables: `infix_info` (`led.rs:30-88`), `prefix_info` (`nud.rs:26-75`).

#### `parse_assign_after_target`: A Misleadingly-Named 507-Line General Dispatcher

`src/frontend/core/parser/statements/declarations.rs:136`'s `parse_assign_after_target`, with the
function body up to `:642` (**507 lines**, the file is 1015 lines in total).

The name is "assign after target", **but it is actually the general dispatcher for declaration
forms**, containing 8 independent responsibility sections: old syntax probing / annotation
disambiguation three-layer nested lookahead / declaration legality / semantic side effects / type
body parsing fallback / RFC-010 lambda signature parameter reconstruction / MetaType definition /
plain initialization.

The cause of the bloat is the superposition of three types:

**Cause One: about 120 lines of dead language features.**

`is_old_function_syntax` (`declarations.rs:82-117`), `skip_old_function_syntax`
(`declarations.rs:120-122`), and the supporting lookahead at `:145-167`.

It needs to be stated precisely: **these two functions are not unreferenced**. They are called at
`declarations.rs:719` and `:725` by `parse_identifier_stmt`, with the path "detect `identifier(`
followed by type parameters and `->` → report 'old syntax deprecated' → skip → return None".

So the accurate statement is: **this is a still-running gate that "rejects removed syntax"**, with
about 120 lines of overhead for doing a bracket balancing scan for every `identifier(`-starting
statement. Among them:

- `skip_old_function_syntax` (`:120-122`) **the function body has only one line of comment**
  `// 旧语法已移除，此函数不再需要`, which is a **pure empty function**;
- More notably, after calling it (`:725-726`), it directly `return None`, **consuming no tokens**.
  That is to say, this "skip" **actually skips nothing**—subsequent tokens remain in the stream, and
  can only be saved by the caller's error recovery. This is a name-reality mismatch interface.

**Cause Two: lookahead logic is not abstracted.** "Skip matching parentheses" is **hand-written 4
times** in the same file:

| Location                           | Line Range                |
| ---------------------------------- | ------------------------- |
| Inside `is_old_function_syntax`    | `declarations.rs:96-105`  |
| Inside `parse_assign_after_target` | `declarations.rs:147-156` |
| Same as above                      | `declarations.rs:191-200` |
| Same as above                      | `declarations.rs:224-233` |

Each about 12 lines, with consistent structure: `paren_depth = 1` →
`while paren_depth > 0 && !at_end()` → on `LParen` add one, on `RParen` subtract one → `bump()`.
**The four copies share no code.**

**Cause Three: type-layer logic leaked into the parser.** See RFC-039 routing table C:
`declarations.rs:27-80` calls type-layer `is_type_param_annotation` / `name_used_as_type`;
`declarations.rs:498-509` hardcodes the string `"Terminates"` and calls
`typecheck::operator_interfaces::spec()`. **This part belongs to 03-type-unification.md /
02-stage-contract.md; this document does not address it redundantly, only marking the boundary in
the "Detailed Design" section.**

#### Full Picture of AST Enums

`src/frontend/core/parser/ast.rs`:

| Enum       | Location         | Variants               |
| ---------- | ---------------- | ---------------------- |
| `Expr`     | `ast.rs:16-163`  | 22 (including `Error`) |
| `BinOp`    | `ast.rs:191-213` | 20                     |
| `UnOp`     | `ast.rs:217-223` | 4                      |
| `StmtKind` | `ast.rs:234`     | 9 (including `Error`)  |
| `Type`     | `ast.rs:427-541` | 26                     |
| `Pattern`  | `ast.rs:794-814` | 8                      |

`Expr::Error` has been verified to exist (`pratt/mod.rs:72`'s span extraction handles it, as does
`ast.rs:1048`); `StmtKind::Error` is constructed in `parser/mod.rs:50`. **These two error
placeholders are the cornerstone of backward-compatible design and must be preserved** (see
"Detailed Design · Backward Compatibility").

### Change Surface for Adding an Operator

**The downstream change surface is much larger than expected.** `BinOp::` has **534 hits / 47
files** across the entire repository, of which **17 are production files**. More critically, these
files consume **two unrelated `BinOp` enums**:

| Enum                | Definition                | Variants                                  | Production Consumers (number in parentheses is the `BinOp::` hit count in that file)                                                                                                                                                     |
| ------------------- | ------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ast::BinOp`        | `parser/ast.rs:191-213`   | 20                                        | `led.rs`(22) `nud.rs`(1) `checker.rs`(8) `inference/expressions.rs`(25) `inference/statements.rs`(5) `operator_interfaces.rs`(5) `ir_gen.rs`(23) `formatter/handlers/expr.rs`(25) `lsp/handlers/inlay_hint.rs`(4) `spawn/analysis.rs`(3) |
| `const_data::BinOp` | `types/const_data.rs:234` | Verified to exist (`Ne` instead of `Neq`) | `const_data.rs`(36) `const_eval.rs`(64) `evaluator.rs`(13) `ownership.rs`(13) `termination.rs`(16) `proof/smt/translate.rs`(13) `proof/dep_graph.rs`(2)                                                                                  |

Verified: `src/backends/` has **zero hits** for `BinOp` and `ast::`—the interpreter consumes IR /
bytecode and never sees the AST. Therefore, all 17 production files fall on the L2 → L3 link.

**There is no `From`/`TryFrom` between the two enums**—`grep` for `impl From<...BinOp`, `-> BinOp`,
`BinOp as` across the entire repository finds no conversion implementations. The cost is that
**every file that touches both must alias at the import point**: `const_eval.rs:19`
(`BinOp as AstBinOp`), `ir_gen.rs:2317` (`use ast::BinOp as B`), `CEBinOp` / `ConstBinOp` in test
files.

> **This is the most important finding of this document.** When `ast::BinOp` adds a variant, **the
> compiler cannot remind you to change `const_data::BinOp`** at all, because they are two
> independent types, each of which can compile independently. The consequence of missing a change
> is: the constant folding path does not recognize the new operator, while the type checking and IR
> generation paths recognize it—**the same operator behaves differently in the two subsystems, with
> no diagnostics**.
>
> This is an order of magnitude more serious than "missing one `match` arm". The latter is caught by
> exhaustive matching, the former cannot be caught.

**Regarding "missing a change cannot be discovered", precise distinction is needed**:

| Type of Missed Change                                      | Discovered by Compiler? | Verified Evidence                                                                                                                                                    |
| ---------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Missing arm in exhaustive `match`                          | **Yes**                 | `const_data.rs:291-317`'s `impl Display for BinOp` is an 18-arm exhaustive match, no `_` fallback                                                                    |
| Missing item in `matches!` macro enum                      | **No**                  | `const_data.rs:263-288`'s `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` are all `matches!` partial enumerations; new variant → silently returns `false` |
| Missing arm in `match` with `_` fallback                   | **No**                  | `ownership.rs:687`'s `_ => {}` silently ignores new variants                                                                                                         |
| **Cross `ast::BinOp` / `const_data::BinOp` missed change** | **Never**               | The two have no conversion, no common parent type, compiler cannot relate them                                                                                       |

So the accurate statement is not "missing 11 places no one knows", but: **the part with exhaustive
matching the compiler can cover; what is truly silent is the `matches!` macro, `_` fallback, and
cross-enum**.

### Dead Code and Test Status

This is the **most needing immediate attention** item in this document.

#### Dead Code

- `precedence.rs:35-94` `Precedence` enum (60 lines) and `precedence.rs:98-133` `PrecedenceContext`
  struct (36 lines): zero production use, the only external referrer is the orphan file
  `pratt/tests/precedence_inline.rs` (95 lines / 6 tests, not declared by `pratt/tests/mod.rs:4-6`).
- `skip_old_function_syntax` (`declarations.rs:120-122`): pure empty function, the call point at
  `:725` directly `return None`, consuming no tokens.
- `BP_RANGE` (`precedence.rs:22`): zero production use (only exists in the comment at
  `led.rs:157-159`).

#### Only One Test File Is Running in the Lexical Layer

**`lexer/mod.rs:104-106`'s only test declaration is:**

```rust
#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
```

`grep 'mod tests'` in the `src/frontend/core/lexer/` directory only hits this one place. **`lexer/`
has never declared `mod tests;`**.

The entire `src/frontend/core/lexer/tests/` subtree therefore **has never participated in
compilation**. This directory actually has **14 files** (including `mod.rs`):

| File                                                                              | Lines          | Status                                      |
| --------------------------------------------------------------------------------- | -------------- | ------------------------------------------- |
| `mod.rs`                                                                          | 38             | Exists, declares 11 submodules              |
| `literals.rs`                                                                     | 222            | Real tests, **orphan**                      |
| `rfc010_lexer.rs`                                                                 | 167            | Real tests, **orphan**                      |
| `fstring.rs`                                                                      | 90             | Real tests, **already wired via `#[path]`** |
| `lexer_mod.rs`                                                                    | 89             | Real tests, **double orphan**               |
| `rfc004_lexer.rs`                                                                 | 81             | Real tests, **orphan**                      |
| `symbols.rs`                                                                      | 70             | Real tests, **double orphan**               |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 1-3 lines each | 7 empty shells                              |
| `debug_lexer.rs`                                                                  | 1              | Empty shell                                 |

**Real but never-run tests: 629 lines / 5 files** (222+167+89+81+70). The lexical layer actually
only has f-string running.

**A wiring trap that must be clarified: `tests/mod.rs` exists, but it is itself not fully wired.**

`lexer/tests/mod.rs:15-25` declares 11 submodules (`basic` / `literals` / `operators` / `delimiters`
/ `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer` / `fstring`),
`:28-38` then `pub use`s all of them as "backward-compatible re-exports".

**But it does not declare `lexer_mod` and `symbols`.** Therefore:

> **Adding only one line `mod tests;` is not enough.** Reviving `lexer/tests/` requires **two**
> changes: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` to
> `tests/mod.rs`. Otherwise these 159 lines will still not run.

**The trap of the 7 empty shells**: `tests/mod.rs:28-38`'s `pub use basic::*;`-type statements,
**referencing a module with zero assertions will not produce any compile errors or warnings**. After
adding `mod tests;`, CI will show "all tests pass", but in reality the seven modules `basic` /
`comments` / `delimiters` / `errors` / `keywords` / `operators` / `debug_lexer` have **not a single
assertion**. **This creates the illusion of "lexical layer is covered"**, which is more dangerous
than not wiring at all.

**Uncovered and highest-risk paths** (will be immediately exposed once wired):

| Path                                                                  | Location                                                                      | Risk                                |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------- |
| Overflow paths of the four radix scanners                             | `literals.rs:134-141` / `220-227` / `306-313` + corresponding decimal section | Integer overflow is a silent error  |
| "Continue consuming but no error" branch after `checked_mul` overflow | 4 isomorphic places                                                           | May cause token stream misalignment |
| The **entire** `scan_leading_dot` function                            | `literals.rs:519-635` (117 lines)                                             | Zero coverage                       |
| `\x` / `\u` illegal escape paths                                      | `literals.rs:730-749` / `1051-1070`                                           | Error path is hard to test          |

**Parser-side wiring is normal**: `parser/mod.rs:8-9`, `pratt/mod.rs:8-9`, `statements/mod.rs:11-12`
all have correct `mod tests;`. `pratt/tests/mod.rs:4-6` declares `led` (335 lines), `nud` (269
lines), `precedence` (13 lines).

**But the parser-side coverage quality also has problems**:

- `pratt/tests/precedence.rs` (**13 lines**) the only assertion is
  `assert!(bp_lowest < bp_highest)`, **nearly no-op**—it cannot even discover the four cross-set
  collisions listed above.
- `pratt/tests/precedence_inline.rs` (**95 lines / 6 tests**) is not declared by
  `pratt/tests/mod.rs`, **has never run**—and what it tests is **entirely the zero-reference
  `Precedence` enum + `PrecedenceContext`**.
- **The two ladders actually in use (`BP_OR`…`BP_MUL` and
  `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`) have no direct unit tests.**

## Target Design

### Lexical: Declarativization

#### Evaluation of Three Plans

| Plan                                             | Cost of Adding a Token      | Can it Eliminate the Three Lexical Duplications? | Can it Solve f-string Nested Compilation?   | Introduced Dependencies                                              |
| ------------------------------------------------ | --------------------------- | ------------------------------------------------ | ------------------------------------------- | -------------------------------------------------------------------- |
| **A. `logos` generates DFA**                     | 1 place (token declaration) | **Yes** (escape/number each one copy)            | No (interpolation must be handled manually) | New `logos`                                                          |
| **B. `regex` hand-written DFA**                  | 1-2 places                  | Partial                                          | No                                          | New `regex` (whether already a transitive dependency **unverified**) |
| **C. Keep hand-written, force-merge duplicates** | 1 place                     | **Yes** (manually extract common functions)      | **Yes** (requires explicit design)          | None                                                                 |

**Stance: adopt C as baseline, A as conditional backup, B not adopted.**

Rationale:

1. **C can eliminate all about 500 lines of duplication in the lexical layer with zero new
   dependencies**, and is entirely within controllable range. The three copies of `scan_string` /
   `scan_char` / `push_fstring_escape` are merged into the **only**
   `decode_escape(lexer, out) -> bool`, and the four radix scanners are merged into the **only**
   `scan_radix_number(lexer, base, digits_pred)`. This is pure mechanical refactoring, with the
   criteria being C5's "identical AST snapshots + identical diagnostics".
2. **A (`logos`) has real capability**—it can converge escape decoding and number scanning into one
   copy each. **But it cannot solve f-string nested compilation**, which is the most important
   defect in the lexical layer. At the same time, introducing a new dependency will change
   `Literals`' error type (the `InvalidEscape` / `InvalidNumber` variants of `LexError` need to be
   remapped), which **directly violates C5's "identical diagnostics" criterion**, unless an
   additional layer of adaptation is written—which equals introducing another manual synchronization
   point.
3. **B (`regex` hand-written DFA) not adopted**: it requires hand-writing state machines, with
   workload comparable to A but much less controllability than A. Since the choice is between A and
   B, A must be chosen logically; since this document advocates C first, there is no reason to
   choose B.

> **Design judgment (not verified fact)**: A may be a better form in the **long term** (a
> declarative token table is itself a "change one place" enforcement). But its migration cost and
> conflict with the "identical diagnostics" criterion make it unsuitable as the content of this C5.
> If A is to be adopted later, it should be as a separate proposal, using the test revival results
> of this document as a safety net.

#### f-string Nested Compilation Must Be Explicitly Addressed

Regardless of choosing A or C, nested compilation must be solved. **Stance: cancel the runtime
nested `tokenize()`, change to one-shot production in the lexical phase.**

Specific approach: when `scan_string` encounters the `f"` prefix, **within the same lexical scan**,
record the interpolation content as `FStringSegment::RawText(String)`, while **not** recursively
calling `tokenize()`. The actual tokenize + parse of the interpolation is deferred to when the
parser first needs that f-string node, and:

- **Done in one shot** (for N interpolations, only 1 tokenize of the interpolation segment is
  performed, caching the result as `Vec<Vec<Token>>`), reducing N+1 to 1;
- **Span mapping is determined in the lexical phase** (recording the absolute offset of each
  interpolation in the source file), no longer relying on the relative semantics of `Point`.

This change is **independent of the paradigm choice** and should be executed even without A.

### Syntax: Complete Grammar-Driven (LALRPOP)

> **Decision (2026-10-03, ChenXu233)**: **Adopt A (LALRPOP complete grammar-driven), no half-baked
> table-ization.** The equivalence risk of A (LALRPOP's error recovery model is different from
> `synchronize()`, whether the trial-and-error parsing behavior can be reproduced bit-by-bit) has
> been resolved by the **double-parser diff** plan below; table-ization is an insufficient
> intermediate state.

#### Plan Evaluation

| Plan                     | Change Surface for Adding a Binary Operator | Can it Eliminate BP Conflicts?                                                                            | Can it Preserve `synchronize()` and Error Placeholders?             | Conclusion                           |
| ------------------------ | ------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------ |
| **A. LALRPOP**           | 1 grammar place + 1 action place            | Yes (LR table auto-calculates precedence, conflicts are exposed as grammar conflict errors at build time) | Needs rewrite, but **error productions can be explicitly declared** | **Adopt**                            |
| **B. tree-sitter**       | 1 grammar.js place                          | Yes                                                                                                       | Not applicable (produces CST, not domain AST)                       | **Reject**                           |
| C. Keep Pratt table-ized | 2-3 places                                  | Yes                                                                                                       | Yes                                                                 | **Reject** (insufficient, see below) |

**Rejection rationale for B** (unchanged): tree-sitter produces a generic syntax tree; this
project's `Expr` / `StmtKind` are **domain ASTs with semantic information** (`Expr::Cast` carries
the target type, `Expr::FString` carries a segment list, `Pattern::Struct` carries field names). CST
→ domain AST still requires a complete mapping layer, **tree-sitter cannot save it**. It is
equivalent to adding a mapping layer of the same size outside the 1326 lines of `nud.rs`.

**Rejection rationale for C**: table-ization solves **synchronization** (changing one place does not
miss), but not **maintainability**—the 22 variants of `Expr` still need 22 hand-written construction
actions, the merge of the two BP ladders still requires changing about 89 references, and structural
issues like `Expr::Lambda { body: empty Block }` being borrowed as a parameter list
(`nud.rs:505-514`) are not touched at all. **It is an unnecessary intermediate stop between
"completely hand-written" and "complete grammar-driven".**

#### Original Concerns About Rejecting A, and How They Are Resolved

The original concern about rejecting A is **equivalence**, not capability:

- The "try to continue parsing" semantics formed by `ParserState::synchronize()`
  (`parser_state.rs:170-186`) and `Expr::Error` / `StmtKind::Error` placeholders, LALRPOP's error
  recovery is a different model (insert/delete tokens), **the resulting diagnostic set must be
  different**
- The `bp_left < min_bp` exit condition at `pratt/mod.rs:100-102`, and the `parse_expression(12)`
  pattern probe at `nud.rs:1041`, are both **trial-and-error behaviors on specific inputs**, and
  whether they can be reproduced bit-by-bit needs to be proven case by case

**Resolution: double-parse diff.** Not relying on the argument that "the criteria should be the
same", but **running it out**:

| Step | Content                                                                                                                                                                   | What It Proves                                                               |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| 1    | Build LALRPOP grammar + action code, **coexist with existing Pratt**, change no existing code                                                                             | ——                                                                           |
| 2    | 293 corpus + `src/std/tests` in their entirety, **both parsers run once each**, AST is normalized according to [07](07-equivalence-oracle.md) and **compared bit-by-bit** | **AST equivalence of legal programs is empirically proven, not by argument** |
| 3    | Switch flow: `parser/mod.rs`'s `parse()` switches to call LALRPOP; Pratt remains as `parse_legacy()`                                                                      | ——                                                                           |
| 4    | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + two BP ladders in `precedence.rs` + about 89 references                                                                    | ——                                                                           |
| 5    | Error recovery: `synchronize()` and `Expr::Error` / `StmtKind::Error` are rebuilt on the LALRPOP side using **explicit error productions**                                | See below                                                                    |

**Step 2 is the core of the entire plan.** If there is any AST non-equivalence among the 293
corpora, it will be discovered before switching flow—this is much stronger than "manually arguing
case by case whether 89 references can be reproduced", and the cost is acceptable (run offline
once).

#### Disposal of Error Recovery: Fully Preserve C5, No Relaxation

**Error diagnostics must be identical item by item; this is a hard requirement, not negotiable.**
The disposal method is to **explicitly model the existing behavior in the grammar**:

| Existing Behavior                                                                                                              | Correspondence in LALRPOP                                                                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ParserState::synchronize()` (`parser_state.rs:170-186`): after a statement parsing fails, skip tokens forward to a sync point | Explicitly declare in the grammar a production of "error → skip to sync point set → continue", **the skip set must be token-by-token identical to the existing implementation**                     |
| `Expr::Error` / `StmtKind::Error` placeholders: enable the parser to continue constructing a "partial AST" after an error      | Explicitly declare error node productions in the grammar, **downstream `match` branch positions unchanged** (if downstream puts `Error` in a wildcard branch, the error will be silently swallowed) |
| Diagnostic code + span                                                                                                         | **Identical item by item. Wording unchanged.**                                                                                                                                                      |

**This is not "should be doable", it must be done.** LALRPOP allows writing explicit error
productions; just write the sync point set of `synchronize()` into the grammar. If phase 3a/3b
empirically proves that the existing diagnostics cannot be reproduced under LALRPOP, **the correct
action is to report the finding truthfully and re-evaluate the plan (including "maintain Pratt"),
not to modify the criteria**.

Migration points and disposal:

| Item                                                              | Content                                                                                                                                                                                                                                                            |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Grammar file                                                      | `src/frontend/core/parser/grammar/yaoxiang.lalrpop` (new directory, see [01](01-routing.md) target structure)                                                                                                                                                      |
| Action code                                                       | `grammar/actions.rs`—move the construction of `Expr` / `StmtKind` from `nud.rs` / `led.rs`, **one function per variant**                                                                                                                                           |
| Error productions                                                 | Explicitly declared, cooperating with `Expr::Error` / `StmtKind::Error` placeholders to preserve the "try to continue parsing" semantics                                                                                                                           |
| BP ladder merge                                                   | The two ladders are deleted along with the grammar; the **6 alive constants** `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`, with their **about 89 production references**, are replaced by precedence declarations in the grammar |
| Bare magic numbers                                                | `(6,7)` / `(11,1)` / `12` disappear along with Pratt—the LR table automatically calculates precedence                                                                                                                                                              |
| `is_old_function_syntax` (`declarations.rs:82-117`, 36 lines)     | **Naturally deleted**—after the grammar-driven approach, `f(Int) -> Int = ...` cannot match any production; specifically "probing removed syntax" is an anti-pattern                                                                                               |
| `Expr::Lambda { body: empty Block }` borrowing (`nud.rs:505-514`) | **Naturally eliminated**—the parameter list is a parameter list in the grammar, no need to borrow an AST node                                                                                                                                                      |
| `synchronize()` (`parser_state.rs:170-186`)                       | Keep the API, change to be driven by error productions at the call site                                                                                                                                                                                            |

**Net effect**: delete `nud.rs`(1326) + `led.rs`(495) + two BP ladders in `precedence.rs` + about
120 lines of dead old syntax probing, add a grammar file and action code. **The change surface for
"adding a binary operator" drops from a dozen places to 1 grammar + 1 action**, and BP conflicts are
reported as **errors at grammar build time**, no longer "coincidentally correct" like now.

#### Prerequisites

1. **The 629 lines / 55 tests of `lexer/tests/` must be revived first** (P1 of
   [09](09-execution-wbs.md)). Without a frontend test safety net, replacing the entire parser
   cannot satisfy 07 "criteria must precede refactoring"
2. **AST normalization snapshots must be checked in first** (P2 second layer), otherwise step 2 has
   nothing to compare
3. **f-string nested compilation elimination** (`literals.rs` refactoring) **independent of this
   item**, should be executed even without grammar-driven approach

### Disposal of `parse_assign_after_target`

**Stance: split into 8 named functions, the function body only does dispatch.**

A 507-line, 8-section-responsibility function requires locating in 500 lines for any local
modification. The target form after splitting:

| Split-out Function             | Responsibility                                           | Approximate Lines | Attribution                                                                            |
| ------------------------------ | -------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------- |
| `probe_old_function_syntax`    | Old syntax probing                                       | 36                | **Deleted in this document**                                                           |
| `resolve_annotation_ambiguity` | Annotation disambiguation (three-layer nested lookahead) | TBD               | This document (significantly shortened after extracting independent lookahead utility) |
| `check_declaration_legality`   | Declaration legality                                     | TBD               | This document                                                                          |
| `apply_semantic_side_effects`  | Semantic side effects                                    | TBD               | **03-type-unification.md** (including `"Terminates"` hardcoding at `:498-509`)         |
| `parse_type_body`              | Type body parsing fallback                               | TBD               | This document                                                                          |
| `rebuild_lambda_params`        | RFC-010 lambda signature parameter reconstruction        | TBD               | This document                                                                          |
| `parse_meta_type_def`          | MetaType definition                                      | TBD               | This document                                                                          |
| `parse_plain_init`             | Plain initialization                                     | TBD               | This document                                                                          |

Companion: converge the 4 copies of "skip matching parentheses" (`96-105` / `147-156` / `191-200` /
`224-233`) into the single method `skip_balanced_parens()` on `ParserState`, with the 4 places
changed to call it.

**Net effect (design judgment)**: 507 lines → 8 single-responsibility functions + 1 lookahead
utility; lookahead code from 48 lines down to about 12 lines; after type-layer logic is moved out
(about 80 lines), the actual part belonging to this document is about 300 lines.

### Dead Code Cleanup (C6, No Equivalence Criteria Required)

According to C6 definition in 07-equivalence-oracle.md ("pure deletion, no equivalence criteria
required, only need to confirm no references"):

| Cleanup Item                                | Location                                   | Lines | Prerequisites                                                                                                   |
| ------------------------------------------- | ------------------------------------------ | ----- | --------------------------------------------------------------------------------------------------------------- |
| `Precedence` enum                           | `precedence.rs:35-94`                      | 60    | **Must handle simultaneously** `pratt/tests/precedence_inline.rs` (the only referrer, itself an orphan)         |
| `PrecedenceContext` struct                  | `precedence.rs:98-133`                     | 36    | Same as above                                                                                                   |
| 6 zero-reference constants in the first set | `precedence.rs:8-13`, `:11` (comment only) | 6     | Zero production references, delete directly                                                                     |
| `BP_RANGE`                                  | `precedence.rs:22`                         | 1     | Zero production references (only exists in `led.rs:157-159` comment); first change `..` to use a named constant |
| `skip_old_function_syntax`                  | `declarations.rs:120-122`                  | 3     | Empty function; the call point at `declarations.rs:725` deleted together                                        |
| `is_old_function_syntax`                    | `declarations.rs:82-117`                   | 36    | See below                                                                                                       |
| `declarations.rs:145-167`                   | Same file                                  | 23    | Same as above                                                                                                   |
| Bare magic number `(6,7)`                   | `led.rs:40`                                | —     | Promote to named constants `BP_RANGE_L` / `BP_RANGE_R`                                                          |
| Bare magic number `(11,1)`                  | `led.rs:85`                                | —     | Promote to `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                        |
| Bare magic number `12`                      | `nud.rs:1041`                              | —     | Promote to `BP_PATTERN_PROBE`, and **record in the comment that it was found by trial**                         |

> **Note**: the 6 **alive** constants in the first ladder `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` /
> `BP_CALL` / `BP_CAST` / `BP_HIGHEST` **are not in the pure-deletion scope of this table**—they
> have about 89 references in production code, and their merge belongs to the "Syntax:
> Grammar-Driven · Plan C" above (C5 category), not C6 deletion.

**Disposal choice for `is_old_function_syntax` (design judgment)**:

It is not unreferenced dead code, but **a still-running "rejects removed syntax" gate**
(`declarations.rs:719`). Two options:

- **Option 1 (aggressive)**: delete the entire section. The "rejection" of already-removed syntax
  should be the responsibility of **the syntax definition itself**—after grammar-driven approach,
  `f(Int) -> Int = ...` simply cannot match any production, naturally reporting an error, with no
  need for specific probing. **This is the dividend brought by paradigm change**: about 120 lines
  naturally disappear after grammar-driven approach.
- **Option 2 (conservative)**: keep the gate but delete the empty function
  `skip_old_function_syntax`, and change "return None without consuming tokens" to actually skip.

**Advocate Option 1**, because it is consistent with the "keep Pratt table-ized" stance: since the
dispatch table is the sole source of truth, hand-writing "probe some removed syntax" is an
anti-pattern. **The premise is that the C5 criteria proves the diagnostic set is unchanged**—need to
empirically test "whether the diagnostic code and span for `f(Int) -> Int = x => x` after deletion
are consistent with before deletion". If inconsistent, fall back to Option 2 and mark as an open
issue.

### Test Reconstruction

**This is the first priority and must precede any paradigm change.**

**Phase 1: revive `lexer/tests/`.** Requires **two** changes, not one:

1. `src/frontend/core/lexer/mod.rs` adds `#[cfg(test)] mod tests;` after `:106`
2. `src/frontend/core/lexer/tests/mod.rs` adds `mod lexer_mod;` and `mod symbols;` (otherwise these
   159 lines will still not run)

**Phase 2: dispose of the 7 empty shells.** Key constraint: `tests/mod.rs:28-38`'s `pub use xxx::*;`
referencing an empty module **produces no compile errors or warnings**; after successful wiring, CI
will show "all green" while actually zero coverage. Disposal method:

| Plan                   | Method                                                                                            | Evaluation                                                                                                                                       |
| ---------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Delete (advocated)** | Delete the 7 empty shell files + corresponding `mod` declarations + corresponding `pub use` lines | **Clean.** The only value of empty shells is "fill in later", and `literals.rs` / `rfc004_lexer.rs` have already taken on these responsibilities |
| Fill                   | Write real assertions for each empty shell                                                        | 7 files × content unknown, **cannot verify what should be tested** (unverified)                                                                  |
| Keep + Mark            | Keep but add `// TODO` and CI count alerts                                                        | Creates the illusion of "covered", **opposed**                                                                                                   |

**Advocate deletion**, and at the same time as dead code cleanup, delete all `pub use` re-exports at
`tests/mod.rs:28-38` (it itself is for an nonexistent re-export design, and it is precisely it that
conceals the empty shells).

**Phase 3: add associativity test cases.** Add `pratt/tests/binding_power.rs`, assert against **the
two ladders actually in effect**:

- After merge, the intra-layer bp is strictly increasing, no duplicates (**this will catch the four
  collisions above before the refactoring**)
- Write a minimal expression for each pair of adjacent layers, assert their associativity direction
  (e.g. `a - b - c` → `(a-b)-c`)
- Write a set of test cases for each of the three special cases `DotDot` / `FatArrow` / pattern
  probing, **replacing the currently no-op `pratt/tests/precedence.rs:6-13`**

**Phase 4: add literal error path tests.** Add test cases item by item for the four types in the
uncovered path table: four radix overflow, `checked_mul` post-continue-consuming branch,
`scan_leading_dot` as a whole, `\x` / `\u` illegal escapes.

**Phase 5: delete `precedence_inline.rs`** (along with dead code deletion), and add a **wiring
self-check** assertion in `pratt/tests/mod.rs` to prevent orphans from recurring.

### Before/After Comparison: How Many Places to Change When Modifying an Operator

| #   | Current State                                                                    | Enforcement of Current State                 | Target                               | Enforcement of Target                           |
| --- | -------------------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------ | ----------------------------------------------- |
| 1   | `lexer/state.rs:21` `keyword_from_str` adds a match arm (keyword form only)      | None                                         | 1 line of operator declaration table | Compile-time                                    |
| 2   | `lexer/tokens.rs:81-162` `TokenKind` adds a variant                              | Compile-time                                 | **Generated from the table**         | Compile-time                                    |
| 3   | `pratt/precedence.rs:6-17` and `22-31` choose one set to add BP constants        | None                                         | **Generated from the table**         | Compile-time (intra-layer uniqueness assertion) |
| 4   | `pratt/led.rs:30-88` `infix_info` adds an arm                                    | Compile-time (exhaustive)                    | **Generated from the table**         | Compile-time                                    |
| 5   | `pratt/led.rs:116-136` `parse_binary` adds an op mapping arm                     | Compile-time (exhaustive)                    | **Generated from the table**         | Compile-time                                    |
| 6   | `ast.rs:191-213` `BinOp` adds a variant                                          | Compile-time                                 | **Generated from the table**         | Compile-time                                    |
| 7   | New symbol character: `tokenizer.rs:218` `next_token_inner`                      | None                                         | **Generated from the table**         | Compile-time                                    |
| 8   | `const_data.rs:234` `const_data::BinOp` adds a variant                           | **None (cross-enum, compiler cannot catch)** | **Co-generated with `ast::BinOp`**   | Compile-time                                    |
| 9   | Sites in 17 downstream production files that depend on `matches!` / `_` fallback | **None (silent)**                            | Generated accessors                  | Compile-time                                    |

**Summary:**

| Stage                         | Manual Change Places                                                                                   | Auto-Generated Places |
| ----------------------------- | ------------------------------------------------------------------------------------------------------ | --------------------- |
| Current                       | **11-12** (6-7 within L2 + about 5-6 production files that must be synchronized downstream)            | 0                     |
| Target (new symbol operator)  | **1-2** (1 line of operator declaration table; if keyword form, add 1 more line of `keyword_from_str`) | 7-8                   |
| Target (reuse existing token) | **1**                                                                                                  | 6-7                   |

And the 1-2 places in the target are **all declarative data rows**, not logic code—the review cost
drops from "deducing semantics per match arm" to "reading one line of declaration".

## Detailed Design

### Interface Boundary with the Type System

**Boundary declaration (important)**: The convergence of `ast::Type` (26 variants,
`ast.rs:427-541`), the elimination of parallel type representations, and the attribution of
`is_type_param_annotation` / `name_used_as_type`, **all belong to
[03-type-unification.md](03-type-unification.md)**. This document only handles the three interfaces
directly related to the frontend paradigm:

**Interface One: the relationship between `const_data::BinOp` and `ast::BinOp`.** Attributed to
**this document**—because it is a direct component of the "add an operator" change surface.
Disposal: the operator declaration table **simultaneously generates** the two enums (or generates
one, derives the other), eliminating the import aliases at `const_eval.rs:19` / `ir_gen.rs:2317`.
The reason for attributing to this document rather than 03: it is an L2 product being
parallel-copied at L3, not a problem of type representation itself.

**Interface Two: the hardcoded `"Terminates"` at `declarations.rs:498-509` and the
`typecheck::operator_interfaces::spec()` call.** Attributed to **03-type-unification.md /
02-stage-contract.md** (eliminate the parser → typecheck reverse dependency, first row of RFC-039
routing table C). This document **isolates** this code into `apply_semantic_side_effects` in the
function split, as a handover point, but does not change its content.

**Interface Three: whether the operator's precedence layer needs type-layer information.** Currently
not needed—`infix_info` is a pure syntax function. **Design judgment**: after table-ization, this
property should be maintained; the operator declaration table **must not** contain any type-related
fields, otherwise L2 will re-depend on L3 (violating RFC-039's dependency direction specification).

### Runtime Behavior

**Target: runtime behavior is unchanged bit-by-bit.** According to C5 criteria of
07-equivalence-oracle.md, the third layer (end-to-end corpus diff) compares the exit codes and
stdout/stderr of `tests/yaoxiang/` 293 `.yx` files + `src/std/tests/*.yx`.

The three **identified runtime impacts** must be explicitly handled:

| Change                                                      | Runtime Impact                                 | Disposal                                                                                  |
| ----------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------- |
| f-string interpolation merged into 1 tokenize               | **Performance improvement**, output unchanged  | Need to empirically measure time benefit and record                                       |
| Operator table generates BP / two ladders merge and reorder | **May change associativity**                   | **Must write equivalence test cases for each layer pair** (Test Reconstruction · Phase 3) |
| Delete `is_old_function_syntax`                             | Diagnostic code/span for old syntax may change | Need corpus diff to prove consistency                                                     |

**Explicitly not done**: do not change the precedence **semantics** of operators. BP numbers can be
reordered (eliminate dead code and collisions), but the **relations** like "multiplication binds
tighter than addition" must remain unchanged.

### Compiler Change List

File by file, with line numbers. C6 cleanup (Phase 0, can be submitted independently):

| File                                                        | Line                                  | Change                                                                                                               |
| ----------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/pratt/precedence.rs`              | `8-13`, `11`                          | Delete the 6 zero-reference first-set constants                                                                      |
| Same as above                                               | `22`                                  | `BP_RANGE` changed to named `..` binding power (synchronized with `:40`), otherwise delete                           |
| Same as above                                               | `35-94`                               | Delete the `Precedence` enum                                                                                         |
| Same as above                                               | `98-133`                              | Delete the `PrecedenceContext` struct                                                                                |
| Same as above                                               | `6-17`, `22-31`                       | Keep alive constants, merge with the second set into a collision-free intra-layer numbering                          |
| `src/frontend/core/parser/pratt/led.rs`                     | `33`                                  | `BP_ASSIGN` changed to point to the reordered assignment layer constant                                              |
| Same as above                                               | `40`                                  | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R`                                                                                |
| Same as above                                               | `75`, `77`, `79`, `83`                | `BP_CALL` changed to point to the reordered suffix layer constant                                                    |
| Same as above                                               | `81`                                  | `BP_CAST` changed to point to the reordered cast layer constant                                                      |
| Same as above                                               | `85`                                  | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                             |
| `src/frontend/core/parser/pratt/nud.rs`                     | `32`, `35`, `83`, `101`, `223`, `243` | `BP_UNARY` changed to point to the reordered prefix layer constant                                                   |
| Same as above                                               | `38-69`                               | `BP_HIGHEST` changed to point to the reordered highest layer constant                                                |
| Same as above                                               | `1041`                                | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`, and add a comment explaining the source of this value |
| `src/frontend/core/parser/statements/declarations.rs`       | `120-122`                             | Delete `skip_old_function_syntax`                                                                                    |
| Same as above                                               | `725`                                 | Delete its call point (deletion of `is_old_function_syntax` see Phase 2)                                             |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | Whole file                            | Delete (the only orphan file referencing dead code)                                                                  |

Companion rename (collateral surface of BP merge, about 89 places): `parser/mod.rs:80`,
`statements/control_flow.rs` 9 places, `statements/declarations.rs:18` / `321` / `601` / `743` /
`818` / `850` / `966` / `994`, `statements/functions.rs:9` / `:110`, `statements/bindings.rs:168`,
`statements/types.rs:15` / `:641` / `:654`, `pratt/led.rs:99` / `:229` / `:321` / `:327` / `:415`,
`pratt/nud.rs:195` / `:263` / `:446` / `:516` / `:523`.

Test revival (Phase 1, can be submitted independently):

| File                                                 | Line                | Change                                                                  |
| ---------------------------------------------------- | ------------------- | ----------------------------------------------------------------------- |
| `src/frontend/core/lexer/mod.rs`                     | After `106`         | Add `#[cfg(test)] mod tests;`                                           |
| `src/frontend/core/lexer/tests/mod.rs`               | `15-25`             | Add `mod lexer_mod;` and `mod symbols;`                                 |
| Same as above                                        | Whole file          | Delete the `mod` declarations and `pub use` lines of the 7 empty shells |
| Same as above                                        | Whole file          | Delete the `pub use ... ::*;` re-export block (`28-38`)                 |
| `src/frontend/core/lexer/tests/`                     | 7 empty shell files | Delete files                                                            |
| `src/frontend/core/parser/pratt/tests/mod.rs`        | `4-6`               | Add wiring self-check assertion                                         |
| `src/frontend/core/parser/pratt/tests/precedence.rs` | `6-13`              | Replace the no-op assertion with a real associativity assertion         |

Paradigm change (Phases 2-4):

| File                                                           | Change                                                                                                                                                                                                                                                 |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/frontend/core/lexer/literals.rs`                          | The four radix scanners merged into `scan_radix_number(lexer, base, digit_pred)`; the three escape decoding implementations merged into the unique `decode_escape(lexer, out)`; `scan_multi_line_string:869` and `scan_fstring_multi_line:1497` merged |
| `src/frontend/core/parser/pratt/nud.rs:441-446`                | Remove the runtime nested `tokenize()`, change to one-shot production in the lexical phase                                                                                                                                                             |
| `src/frontend/core/parser/pratt/led.rs:30-88` / `nud.rs:26-75` | The two hand-written tables replaced with generated mapping tables                                                                                                                                                                                     |
| `src/frontend/core/parser/ast.rs:191-213`                      | `BinOp` and `const_data::BinOp` changed to co-generated                                                                                                                                                                                                |
| `src/frontend/core/types/const_data.rs:234`                    | Same as above                                                                                                                                                                                                                                          |
| `src/frontend/core/parser/statements/declarations.rs:136-642`  | Split into 8 functions + `ParserState::skip_balanced_parens()` (the 4 places `96-105`/`147-156`/`191-200`/`224-233` changed to call)                                                                                                                   |
| `src/frontend/core/parser/parser_state.rs`                     | Add `skip_balanced_parens()` (`synchronize()` is at `:170`, unchanged)                                                                                                                                                                                 |

**Files this document does not change**: `src/middle/core/ir_gen.rs`,
`src/frontend/core/typecheck/**`, `src/backends/**`, the **main body** of
`src/frontend/core/formatter/**`. They are only changed when "generated accessors" replace the
`matches!` macro, and the change is a mechanical replacement.

### Backward Compatibility

The easiest thing for table-ization to break is **error recovery**. The recovery contract of this
project is composed of three parts, each of which must be maintained individually:

**Contract One: `ParserState::synchronize()` (`parser_state.rs:170-186`).** It skips tokens forward
to a sync point after a statement parsing fails. **The C5 criteria requires it to be preserved, and
requires the trigger timing and skip set to be completely identical.** After migrating to LALRPOP,
the call point of this function is changed to be driven by explicit error productions, and the
function body and sync point set **do not change a single line**—the semantics is reproduced by the
grammar, and the implementation is kept as is.

**Contract Two: `Expr::Error` (`ast.rs:1048`, consumed at `pratt/mod.rs:72`) and `StmtKind::Error`
(constructed at `parser/mod.rs:50`).** These two placeholders enable the parser to continue
constructing a "partial AST" after an error, with downstream skipping by the `Error` branch. **The
variant must be preserved, and their position in downstream `match` must be preserved** (if
downstream puts `Expr::Error` in a wildcard branch, the error will be silently swallowed).

**Contract Three: the code, span, and order of diagnostics.** The comparison rule of the third layer
in 07-equivalence-oracle.md is "the diagnostic list is sorted by `(code, span.file, span.line)`,
**message text is not part of the comparison**". This gives this document some leeway (wording can
vary), but **code and span are immutable**.

**f-string special note**: the interpolation refactoring will change the span from "relative to
interpolation text" to "absolute offset in source file". To satisfy contract three, **the span must
be recorded by absolute offset in the lexical phase** (decision D25)—the `span.file/line` of
diagnostics within the interpolation must not change due to this refactoring.

> **Disposal**: establish a baseline snapshot of "diagnostic span within f-string interpolation"
> before phase 2, and compare item by item after refactoring. `span.file/line` inconsistency is
> considered an implementation defect and must be fixed; there is no option of "accepting span
> change". (If empirical tests prove that the interpolation span of the original implementation is
> already wrong, it will be exposed during the baseline establishment phase—that is an existing
> independent defect, fixed separately, not merged into this phase, and does not constitute a reason
> to relax the criteria.)

## Implementation Points

This document corresponds to five phases. **Each phase can be submitted and rolled back
independently.** Phases 0 and 1 are C6 and test revival, no equivalence criteria required; Phases
2-4 are C5, subject to criteria throughout.

| Phase  | Content                                                                                                                   | Category    | Acceptance Criteria                                                                                                                                     | Rollback Point                       |
| ------ | ------------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| **0**  | Dead code cleanup (`Precedence` enum + `PrecedenceContext` 96 lines)                                                      | C6          | `cargo build` passes; `cargo clippy` has no new warnings                                                                                                | Single commit, `git revert` suffices |
| **1**  | Test revival and supplementation                                                                                          | —           | Number of actually running tests in the lexical layer ≥ 5 files / 629 lines; 7 empty shells deleted; associativity test cases **red first, then green** | Single commit                        |
| **2**  | Lexical duplication convergence + f-string nested compilation elimination                                                 | C5          | Identical AST snapshots + identical diagnostics + identical behavior; `literals.rs` reduced by about 500 lines                                          | Submitted separately from Phase 3    |
| **3a** | Build LALRPOP grammar + action code, **coexist with Pratt**                                                               | —           | `cargo build` passes; new parser can produce AST for 293 corpora (not compared)                                                                         | Submitted separately from Phase 2    |
| **3b** | **Double-parser diff** (both parsers run 293 corpora, **AST + diagnostic code+span** normalized then compared bit-by-bit) | **Full C5** | **AST and diagnostics are item-by-item equivalent—this step is the equivalence proof of the entire grammar migration**                                  | Run offline once, repeatable         |
| **3c** | Switch flow: `parser/mod.rs`'s `parse()` switches to call LALRPOP, Pratt remains as `parse_legacy()`                      | **Full C5** | Full corpus behavior equivalent + diagnostic code+span item-by-item identical                                                                           | Independent commit                   |
| **3d** | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + two BP ladders in `precedence.rs` + about 89 references                    | **Full C5** | Corpus all green; `git grep BP_` zero hits                                                                                                              | Independent commit                   |
| **4**  | `parse_assign_after_target` split                                                                                         | C5          | The function only does dispatch; `skip_balanced_parens` is the single implementation                                                                    | Independent commit                   |

**Internal dependency order of this document**: Phase 1 **must precede** Phases 2/3/4—"refactor
first, then add tests" is a pit the project has stepped into. **Phase 3b (double-parser diff) is
unskippable**: it must run all green before the 3c switch, otherwise the entire grammar migration
has no equivalence basis.

**3b/3c go/no-go gate**: if the number of error productions clearly exceeds what is needed to "model
the sync point set of `synchronize()`" (parser-embedded semantic judgments—`declare_predicate`
accumulation, const generic parameter filtering—are the hardest to migrate), or if the diff still
has non-equivalent items for two consecutive iterations, **stop migration and re-evaluate**, falling
back to the intermediate plan of "keep Pratt + single declarative precedence table + two `BinOp`s
co-generated". This plan can already obtain most of the benefits of operator change surface
convergence; LALRPOP is a means, not a goal. Fallback belongs to the "re-evaluate" branch reserved
by D24, **not criteria relaxation**.

**A note on Phase 0**: the 6 **alive** first-set constants `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` /
`BP_CALL` / `BP_CAST` / `BP_HIGHEST` **are not in the deletion scope of Phase 0**—they are deleted
along with Pratt in Phase 3d. Phase 0 only deletes the `Precedence` enum and `PrecedenceContext` (96
lines, zero production references).

**Criteria execution method**: the three-layer criteria definition and regression gate are in
[07-equivalence-oracle.md](07-equivalence-oracle.md). **All go through full C5, including diagnostic
code+span. No relaxation exists.**

**Scope limitation**: this document **does not include** `logos` migration (lexical layer keeps
hand-written + merge duplicates, see "Key Decisions and Rationale"). **LALRPOP grammar migration is
a component of this document**, see the "Syntax: Complete Grammar-Driven" section.

## Key Decisions and Rationale

### Three Already-Decided Decisions

| Decision                           | Determination                                                                             | Rationale                                                                                                                                                                                                                                                                                   |
| ---------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexical plan                       | Adopt "keep hand-written + force-merge duplicates"; `logos` listed as conditional backup  | Eliminate about 500 lines of duplication with zero new dependencies; `logos` changes the `LexError` error type, and cannot solve f-string nested compilation                                                                                                                                |
| **Syntax plan**                    | **Adopt LALRPOP complete grammar-driven**; reject tree-sitter and "keep Pratt table-ized" | tree-sitter produces CST, not domain AST, still requires a complete mapping code layer; table-ization only solves synchronization, not maintainability, is an insufficient intermediate stop. LALRPOP's equivalence risk is resolved by **double-parser diff** (see above), not by argument |
| Attribution of `const_data::BinOp` | Attributed to this document                                                               | It is a direct component of the "add an operator" change surface; handled separately from `ast::Type` convergence (03)                                                                                                                                                                      |

### Note on "Doing It Completely"

Table-ization is an insufficient intermediate stop: it solves synchronization, not maintainability;
and "re-evaluate after criteria relaxation" is passive waiting, not an engineering decision.
Therefore, this document adopts **complete grammar-driven**, with equivalence guaranteed by
double-parser diff empirical testing (see above), and no relaxation in the diagnostic layer.

**The cost of this shift must be stated clearly**:

- Construction workload increases significantly: need to build a grammar file, move the construction
  actions of 22 `Expr` variants, merge about 89 BP references, rewrite error recovery
- **Prerequisites are hardened**: the 629 lines of frontend tests must be revived first (P1 of 09),
  and AST normalization snapshots must be checked in first (P2 of 09). Without these two, the diff
  of step 2 cannot be done
- **Introduces a new dependency** (LALRPOP), inconsistent with the lexical layer's "zero new
  dependencies" orientation

**The benefits are certain**: "adding an operator" drops from 11 places to 2 places, BP conflicts go
from "coincidentally correct" to "grammar build-time error", `Expr::Lambda` borrowing,
`parse_expression(12)` magic number, about 120 lines of dead old syntax probing **naturally
disappear**.

### Rejected or Downgraded Plans

**A. Only delete dead code, no paradigm change.** Verified feasible, but insufficient to solve the
problem. Dead code cleanup (about 130 lines + 3 magic numbers) can be done at **zero criteria
cost**, and should be done immediately. But its impact on "add an operator changes 11 places" is:
the L2 change surface **does not decrease by even one place**; cross-enum missed change **is not
touched at all**. **Conclusion: adopt as Phase 0 and Phase 1, but must continue to do
grammar-driven. Treating A as the complete plan is a wrong form of self-consolation.**

**B. Keep Pratt, only add tests.** **Not adopted as a complete plan, but adopted as a
prerequisite.** Tests **cannot discover cross-enum missed change**—the two enums can each compile
independently, whether the tests pass is unrelated to whether they are synchronized. **Conclusion:
adopt as Phase 1; reject as a complete plan.**

**C. Keep Pratt and table-ize.** **Rejected** (2026-10-03). It is an unnecessary intermediate stop
between "completely hand-written" and "complete grammar-driven": the 22 variants of `Expr` still
need 22 hand-written construction actions, the `Expr::Lambda { body: empty Block }` borrowing
problem, the `parse_expression(12)` magic number, the 89 references of the two ladders merge, all
remain.

**D. Adopt tree-sitter.** **Rejected.** tree-sitter's incremental parsing capability has no
substantial benefit to this project (full compilation each time, 293 corpora scale); and it produces
CST, not domain AST, still requires a complete mapping code layer, equivalent to a net increase of
the same size outside `nud.rs`.

**E. Adopt `logos` to replace the hand-written lexer.** **Conditionally not adopted.**
**Prerequisites for re-evaluation**: after Phase 2 is completed (escape decoding has converged into
a unique implementation), the workload of replacing that implementation with `logos` callbacks will
drop significantly.

### Net Benefits of the Plan

- **The change surface drops from "a dozen places of logic code" to "1-2 places of declarative
  data + 7-8 generated".** And the generated 7-8 places are guaranteed to be synchronized by the
  compiler, missed change is no longer a possible event.
- **Eliminate a class of defects that the compiler can never catch.** Cross-enum missed change
  (`ast::BinOp` vs `const_data::BinOp`) is currently the only **defect type that no tool can
  discover**, because the two types are independent of each other. Co-generation directly eliminates
  it.
- **Turn BP collisions from "coincidentally correct" to "compile failure".** The existing four
  cross-set collisions and two intra-set self-collisions currently rely on the unstated convention
  of `bp_right = bp_left + 1` to luckily not blow up; the intra-layer uniqueness assertion makes it
  impossible for such problems to silently exist thereafter.
- **Frontend tests change from "1 file running" to "all running".** After the 629 lines of existing
  tests are revived, Phases 2-4 have a safety net; and these tests **already existed**, just a `mod`
  line was not written.
- **Phases 0 and 1 are zero risk, zero criteria cost.** Dead code deletion and test revival can be
  done first, not blocking any other work.

## Known Limitations and Risks

### Design Risks

- **The operator table is a new "thing that must be maintained".** It turns "change 11 places" into
  "change 1 place + maintain a table". **If the field design of the table itself is improper, it
  will degrade into a new centralized maintenance burden**. This is the main risk of this plan; the
  mitigation is to have every column of the table protected by compile-time assertions.
- **The risk of BP number reordering is asymmetric.** Merging the two ladders involves renaming
  about 89 production references; if any one is missed, **associativity will silently change** (no
  error, just different expression parsing results). This is the most likely silent regression
  introduced by this document, and must be caught by the layer-pair associativity test cases of
  Phase 1.
- **The C5 criteria cost of Phases 2-4 is high.** Using all three layers of criteria means each
  phase must run the full corpus (293 `.yx` files) + AST snapshots + diagnostic comparison. This
  will make each commit of Phases 2-4 heavier.
- **The span change of f-string may not be completely avoidable.** See "Backward Compatibility"; it
  may be necessary to accept one known behavior change and update the baseline.

### Unresolved Issues

> **The open issues originally listed in this section have all been decided.** Item-by-item
> decisions are in [RFC-039 Decisions Registry](../../rfc/draft/039-compiler-architecture.md) (D1–D50).
> **This document leaves no pending items.**

## Verification Records

All code facts in this document are **directly read from disk** to verify. Verification method:
`read` tool directly reads the target file line range; line count uses `Get-Content`; reference
surface uses `grep` tool to scan the entire repository and classify into production / test / orphan.
**`cargo test` / `cargo build` are not used**—this document is pure documentation, and criteria
execution belongs to the implementation phase.

### Item-by-Item Empirical Results

| Verification Item                             | Empirical Result                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Total lines in `literals.rs`                  | **1568**                                                                                                                                                                                                                                                                                                         |
| Total lines in `tokenizer.rs`                 | **554**                                                                                                                                                                                                                                                                                                          |
| `next_token_inner` line                       | **`:218`**                                                                                                                                                                                                                                                                                                       |
| Line ranges of the four radix scanners        | `77-161` (85 lines) / `164-247` (84) / `250-333` (84) / `336-516` (181)—consistent with the table in this document                                                                                                                                                                                               |
| Three escape decoding implementations         | `694-827` (134 lines) / `1015-1141` (127 lines) / `push_fstring_escape:1374`—consistent with the table in this document                                                                                                                                                                                          |
| `scan_leading_dot`                            | `519-635`, **117 lines**                                                                                                                                                                                                                                                                                         |
| Number of `Expr` variants                     | **22** (including `Error`), `ast.rs:16-163`                                                                                                                                                                                                                                                                      |
| Number of files in `lexer/tests/`             | **14** (including `mod.rs`)                                                                                                                                                                                                                                                                                      |
| Orphan real test lines                        | **629** (222+167+89+81+70); another 90 lines of `fstring.rs` is already wired via `#[path]`, not counted as orphan                                                                                                                                                                                               |
| Changes required to revive `lexer/tests/`     | **Two**: add `mod tests;` to `lexer/mod.rs` **and** add `mod lexer_mod;` `mod symbols;` to `tests/mod.rs`. Adding only one keeps the latter 159 lines not running                                                                                                                                                |
| Dead code in `precedence.rs`                  | **96 lines** (`Precedence` enum `35-94` + `PrecedenceContext` `98-133`) zero production use **confirmed**; but the only external referrer `precedence_inline.rs` (95 lines / 6 tests) is itself an orphan—**deleting dead code must simultaneously delete that file**                                            |
| Lines of 7 empty shells                       | 1-3 lines each (`debug_lexer.rs` 1 line, the rest 3 lines)                                                                                                                                                                                                                                                       |
| Blank line residue form                       | **3 consecutive blank lines inserted between each field, spanning 11 lines** (`777-787` / `1098-1108`)                                                                                                                                                                                                           |
| Lines of `pratt/tests/precedence.rs`          | **13**, the only assertion `assert!(bp_lowest < bp_highest)`                                                                                                                                                                                                                                                     |
| 4th "skip matching parentheses"               | `224-233`                                                                                                                                                                                                                                                                                                        |
| Downstream change surface                     | **17 production files** match `BinOp::` (entire repository **534 hits / 47 files**)                                                                                                                                                                                                                              |
| Whether `src/backends/` is a `BinOp` consumer | **No**—`src/backends/` has **zero hits** for `BinOp` and `ast::`; the interpreter consumes IR / bytecode and never sees the AST                                                                                                                                                                                  |
| "No check can discover missing change"        | **Needs layering**: missing arm in exhaustive `match` **will be discovered by the compiler**; what is silent is the `matches!` macro, `_` fallback, and **cross-enum**. This document is now expressed in layers according to empirical tests                                                                    |
| `matches!` block in `const_data.rs`           | `263-288` (`is_arith` / `is_comparison` / `is_logical` / `is_bitwise` four-function range)                                                                                                                                                                                                                       |
| `impl Display for BinOp`                      | From `291`, 18-arm exhaustive match, no `_` fallback                                                                                                                                                                                                                                                             |
| Two functions in `parse_assign_after_target`  | **Not dead code**: the two functions are still called at `declarations.rs:719` / `:725`, being a running "rejects removed syntax" gate; and `skip_old_function_syntax` (`120-122`, function body is only one line of comment) does not consume any tokens after being called (`:725-726` directly `return None`) |
| Second `BinOp` enum                           | `const_data.rs:234` indeed exists, with **no `From` conversion** with `ast::BinOp`, forcing `const_eval.rs:19` / `ir_gen.rs:2317` and other files to alias at the import point                                                                                                                                   |
| `lexer/tests/mod.rs` re-export block          | Exists (38-line file), `:28-38` carries `pub use ... ::*;`, is the direct reason empty shells can disguise as covered                                                                                                                                                                                            |

### Three Points Corrected by This Round of Empirical Verification

The following three points do not match the intuition that "the first BP ladder is dead", and the
empirical test is the standard:

1. **The two ladders are intertwined, not "one alive, one dead".** `infix_info` (`led.rs:30-88`)
   itself references 5 constants of the first set (`BP_ASSIGN:33`, `BP_CALL:75/77/79/83`,
   `BP_CAST:81`) and 8 of the second set simultaneously. Of the 12 first-set constants, **only 6
   truly have zero references** (`BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` /
   `BP_FACTOR`, plus `BP_COMPARISON` which only exists in the comment at `declarations.rs:742`); the
   other 6 (`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`) have a
   total of about **89 references in production code, across 8 files**. Therefore "delete the first
   BP constants" is not a 12-line C6 deletion, but a cross-file rename in the C5 category.
2. **The second set also has dead members itself.** `BP_RANGE` (`precedence.rs:22`) has zero use in
   production code—`..` at `led.rs:40` uses the bare literal `(6, 7)`, and `BP_RANGE` only appears
   in the comment at `led.rs:157-159`.
3. **That "self-contradicting comment" is at `precedence.rs:33`, not `:11`.** `*:11` is
   `pub const BP_COMPARISON: u8 = 5;`. The comment (_"Precedence rules for the Pratt parser"_)
   describes the zero-reference `Precedence` enum.

Incidental correction: `bp_right = bp_left + 1` produces **left-associativity** (`a - b - c` →
`(a-b)-c`), not right-associativity; the `(11, 1)` at `led.rs:85` is the only explicit right-operand
greedy swallowing writing style. The collisions are not only two cross-set ones; there are also
`BP_UNARY=8 == BP_CAST=8` (intra-first-set self-collision, both alive) and `BP_RANGE=7 == BP_ADD=7`
(intra-second-set self-collision).

### Unverified Items

The "unverified" annotation has been made point by point in the main text:

- The specific arm count and per-arm binding power of the `prefix_info` table body (`26-75`) in
  `pratt/nud.rs`
- The exact line boundaries of the 8 responsibility sections in `declarations.rs:136-642`
- The complete variant set of `const_data::BinOp` (`const_data.rs:234`)
- Whether `logos` / `regex` is already in `Cargo.toml` (as a transitive dependency)
- All call points of `synchronize()` outside `parser_state.rs:170-186`
- Whether this project is a published library (affects the risk assessment of deleting public
  surface)

## See Also

### Companion Design Documents in the Same Batch

- [01-routing.md](01-routing.md) — Where to change when adding a feature (feature routing tables
  A/B/C, dependency direction specification)
- [03-type-unification.md](03-type-unification.md) — Attribution of `ast::Type` convergence, and the
  interface boundary with this document
- [04-ssa.md](04-ssa.md) — Also a C4/C5 criteria user
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Complete inventory of 8 orphan test trees
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C5/C6 category definition, three-layer
  criteria, regression gate

### RFC Main Text and Related Proposals

- [RFC-039: Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md) —
  Upper-level outline; four-layer model, glossary, feature routing tables A/B/C
- [RFC-013: Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Build-time
  gate for error codes, the example gate this document references
- [RFC-010: Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Source of `ast::Type`

### Code Location Index

- `src/frontend/core/lexer/literals.rs:77-161` / `164-247` / `250-333` / `336-516` — The four radix
  scanners
- `src/frontend/core/lexer/literals.rs:694-827` / `1015-1141` / `1374` — Three escape decoding
  implementations
- `src/frontend/core/lexer/literals.rs:519-635` — `scan_leading_dot` (zero coverage)
- `src/frontend/core/lexer/mod.rs:104-106` — The only test declaration in the lexical layer
- `src/frontend/core/lexer/tests/mod.rs:15-25` / `28-38` — Submodule declarations and `pub use`
  re-export block
- `src/frontend/core/parser/pratt/nud.rs:441-446` — Runtime nested `tokenize()` for f-string
  interpolation
- `src/frontend/core/parser/pratt/nud.rs:896-901` / `1041` — The source self-statement of magic
  number `12` and its use point
- `src/frontend/core/parser/pratt/precedence.rs:6-17` and `22-31` — The two BP ladders
- `src/frontend/core/parser/pratt/precedence.rs:33` / `35-94` / `98-133` — Diverged comment,
  zero-reference enum and struct
- `src/frontend/core/parser/pratt/led.rs:30-88` — Hand-written `infix_info` table
- `src/frontend/core/parser/pratt/tests/mod.rs:4-6` — Orphan wiring missing `precedence_inline`
  declaration
- `src/frontend/core/parser/statements/declarations.rs:719-727` — Old syntax rejection gate and its
  "skip but do not consume" defect
- `src/frontend/core/types/const_data.rs:234` — The second operator enum parallel to `ast::BinOp`
- `build.rs:19-55` — Build-time gate for error codes, the example enforcement mechanism existing in
  the repository
- `src/middle/core/tests/bytecode.rs:369-657` — `test_every_opcode_roundtrips_not_silently_nop`, the
  example "item-by-item roundtrip" criteria existing in the repository
