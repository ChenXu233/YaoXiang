# Frontend Paradigm: Lexical and Syntax

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance-criterion tiers, and execution-phase ordering are covered in the
> RFC-039 main text; the positioning of each subsidiary document is in the
> [directory index](index.md).

## Positioning and Scope

This document handles L2 frontend—**lexical (`src/frontend/core/lexer/`) and syntax
(`src/frontend/core/parser/`)**.

**Problem to solve**: reduce the change footprint of "adding a binary operator" from the current
**6-7 sites within L2 + 17 downstream production files** (of which the compiler enforces roughly
8-9, with the rest relying on manual synchronization) down to **1-2 declaration sites + 7-8
auto-generated sites**.

This document implements the pain point labeled in RFC-039 function routing table A, first row,
"binary operator". The refactoring category is **C5 frontend paradigm change** as defined in
[07-equivalence-oracle.md](07-equivalence-oracle.md) (criterion: same AST snapshot + same
diagnostics + same behavior, all three) and **C6 pure deletion** (no equivalence criterion needed,
just confirm no references). C6 items are prerequisites for C5: delete 96 lines of dead code,
eliminate three bare magic numbers, revive 629 lines of lexical tests that never ran.

**Scope boundary**: the disposition of `ast::Type` (26 variants) and the convergence of three
parallel type representations belong to [03-type-unification.md](03-type-unification.md); this
document does not duplicate the treatment, only declares the interface boundary in the "Detailed
Design" section.

### Why This Is Worth Doing

"Adding an operator" is the most expensive single change in the frontend. This tax has **three
malignant characteristics**:

**First, the cost does not decrease with experience.** Someone who has added ten operators does not
change one less place than someone who has added one—because there is no derivable relationship
between these locations. The `(6,7)` in `led.rs:40` and the `12` in `nud.rs:1041` have no
connection, but they must agree with each other, or expression parsing will fail.

**Second, the consequences of a missed change come in two types, and the worst type the compiler
cannot catch.** See the layered table in the "Change Footprint of Adding an Operator" section: an
exhaustive `match` miss is caught by the compiler, but **a cross-enum miss is forever uncatchable by
the compiler**.

**Third, this tax is invisible in the books.** Every commit in
`git log --oneline -- src/frontend/core/parser/pratt/led.rs` almost always touches 3-4 files at
once, but no tool can point out "which site this commit missed".

### Correct Paradigms Already in the Repository

This project has two **correct solutions to the same kind of problem** that can be reused directly:

- **Error codes**: `build.rs:19-55` uses `tools/code-tables` to parse the registry at **build
  time**, validate uniqueness, and compare against the RFC-013 markdown code table item by item,
  **rejecting the build via `panic!` on any inconsistency**. The cost of adding an error code is
  therefore a constant 1 site + documentation.
- **Standard library interfaces**: `gen_interfaces.rs` compares byte-by-byte, and `gen_docs.rs`
  detects marker range drift (RFC-039 routing table B, last row).

The common feature of these two mechanisms is **turning "must modify in sync" from human memory into
executable assertions**. The entire goal of this document can be reduced to the same sentence: turn
the correspondence between the operator table, the lexical rule table, and the AST variant table
into executable assertions.

### Why "Just Deleting Dead Code" Is Not Enough

Dead code does exist (`precedence.rs` 96 lines, `skip_old_function_syntax` empty function, 629 lines
of orphan tests), but after deleting them, "add an operator and change a dozen places" will not be
reduced by a single one. Dead code is **stock debt**; the cross-file change footprint is
**structural cost**. The two must be handled separately (see "Key Decisions and Rationale" for
treatment).

## Current State

All of the following are **verified facts**, each with file path and line number. The verification
method and item-by-item verification results are in "Verification Record".

### Lexical Layer: Copy-Paste Evidence in `literals.rs`

`src/frontend/core/lexer/literals.rs` has a total of **1568 lines**. This volume does not come from
complexity, but from three places of structural duplication.

**Duplication 1: four radix scanners are nearly identical line by line.**

| Scanner               | Line Range            | Line Count |
| --------------------- | --------------------- | ---------- |
| `scan_hex_number`     | `literals.rs:77-161`  | 85         |
| `scan_octal_number`   | `literals.rs:164-247` | 84         |
| `scan_binary_number`  | `literals.rs:250-333` | 84         |
| `scan_decimal_number` | `literals.rs:336-516` | 181        |

The first three are each about 84 lines with identical structure: accumulate digit → `has_digits`
check → `overflow` flag → `checked_mul(base)` → `checked_add` → `try_into` → error report. **The
only difference is the base constant and the digit extraction expression.** Across the three copies
× 84 lines, **about 250 lines are pure redundancy** (`scan_decimal_number` is listed separately
because it needs additional handling for decimal points and exponents and cannot be merged).

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

`scan_octal_number:220-227` and `scan_binary_number:306-313` are the same piece of code with a
different base and message text.

**Duplication 2: escape decoding is implemented 3 times.**

| Implementation              | Location                   | Length                                       |
| --------------------------- | -------------------------- | -------------------------------------------- |
| `scan_string`'s `\\` branch | `literals.rs:694-827`      | 134 lines                                    |
| `scan_char`'s `\\` branch   | `literals.rs:1015-1141`    | 127 lines                                    |
| `push_fstring_escape`       | `literals.rs:1374` onwards | Already extracted into a standalone function |

The first two are nearly identical copy-paste of 127-134 lines (`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` →
push the same, `\x` / `\u{...}` logic the same, `c =>` error fallback the same), **about 250 lines
are written twice**.

Worth pointing out separately: **the f-string version has already been extracted into a standalone
function** (`push_fstring_escape:1374`). This shows that **someone has already realized this
duplication and partially fixed it, but did not go back to consolidate the other two places**. This
is more noteworthy than "no one realized all three"—it proves the problem is solvable; what is
missing is an enforcement mechanism.

**Duplication 3: multi-line string scanning has two copies.** `scan_multi_line_string:869` and
`scan_fstring_multi_line:1497` are again two similar implementations.

**Abnormal formatting residue.** Two places, `literals.rs:777-787` and `1098-1108`, each 11 lines
long, take the form of **inserting 3 consecutive blank lines between every field of a struct
literal**. This is automated rewrite residue. Other places in the same function (`1111-1115`) have
only a single blank line between fields, indicating that formatting was interrupted.

> This kind of residue is harmless by itself, but it is a **diagnostic signal**: the same place in
> the code has been rewritten by different tools twice, meaning someone in history made a local
> patch here without pulling the broader context.

**Conclusion**: about 500 lines in `literals.rs` (250 lines of radix scanner redundancy + 250 lines
of escape duplication) is **mechanical duplication**, not complexity.

**f-string interpolation triggers nested compilation.** This is **runtime duplicate compilation**,
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

Consequence: an f-string with N interpolations **triggers N+1 complete lexical analyses** (N
interpolations + 1 outer). At the same time, each interpolation is an **independent parsing
context**, so:

- diagnostic spans inside the interpolation are relative to the interpolation text, requiring extra
  work to remap to source locations;
- the interpolation cannot access any outer parser state (no current need, but the structure
  forecloses future possibilities, e.g., referencing outer implicit variables in the interpolation);
- the production location of lexical errors and syntax errors is split across two phases.

This is a **double defect of performance and structure** and must be addressed (see "Target Design ·
Lexical").

### Syntax Layer: Two Sets of BP Ladders, Bare Magic Numbers, Hand-Written Dispatch Tables

#### The Two Ladders Are Not "One Alive, One Dead", but Interwoven

`src/frontend/core/parser/pratt/precedence.rs` contains **two binding-power ladders** at the same
time, with substantial overlap in numbering.

**First set (`precedence.rs:6-17`)**: `BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` /
`BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` /
`BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`.

**Second set (`precedence.rs:22-31`)**: `BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` /
`BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`.

**Verified conclusion: both sets are alive in production code, and they are interwoven starting from
inside `infix_info`.**

- `infix_info` (`led.rs:30-88`) references **5 constants from the first set**: `BP_ASSIGN` (`:33`),
  `BP_CALL` (`:75` / `:77` / `:79` / `:83`), `BP_CAST` (`:81`);
- references **8 constants from the second set**: `BP_OR` (`:42`), `BP_AND` (`:44`), `BP_EQ`
  (`:47`), `BP_SHIFT` (`:51` / `:54`), `BP_BIT` (`:58`), `BP_CMP` (`:62` / `:65`), `BP_ADD` (`:68`),
  `BP_MUL` (`:72`);
- **The only dead member in the second set is `BP_RANGE`**—`..` in `led.rs:40` uses the bare literal
  `(6, 7)` instead of `BP_RANGE`; `BP_RANGE` appears in production code only in the comment at
  `led.rs:157-159`.

Survival status of the 12 constants in the first set (`grep` full-repository scan results):

| Status              | Constant                                                                     | Production Reference Sites                                                                                                                                                                    |
| ------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Dead (6)**        | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | Zero production hits. Appears only in `precedence.rs` itself and in the orphan test `precedence_inline.rs`                                                                                    |
| Dead (comment-only) | `BP_COMPARISON`                                                              | Zero production hits; mentioned only in the comment at `declarations.rs:742`                                                                                                                  |
| **Alive (6)**       | `BP_LOWEST`                                                                  | `parser/mod.rs:80`, `statements/control_flow.rs` 9 sites, `statements/declarations.rs` 6 sites, `pratt/led.rs:229`/`:321`/`:327`/`:415`, `pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523`, etc. |
|                     | `BP_ASSIGN`                                                                  | `led.rs:33`, `led.rs:99`, `statements/declarations.rs:18` (import), `:743` (`BP_ASSIGN + 1`)                                                                                                  |
|                     | `BP_UNARY`                                                                   | `nud.rs:32`, `:35`, `:83`, `:101`, `:223`, `:243`                                                                                                                                             |
|                     | `BP_CALL`                                                                    | `led.rs:75`, `:77`, `:79`, `:83`                                                                                                                                                              |
|                     | `BP_CAST`                                                                    | `led.rs:81`                                                                                                                                                                                   |
|                     | `BP_HIGHEST`                                                                 | `nud.rs:38-69` about 20 sites (one per literal and keyword prefix)                                                                                                                            |

**Total: the 6 alive constants have about 89 production references, across 8 files.**

> This corrects a very easy false conclusion: "delete the first set ladder" is **not** a 12-line
> deletion, but a rename and merge of 89 references across 8 production files. Only 6 constants in
> the first set have truly zero references (plus `BP_COMPARISON` which exists only in a comment).
> The second set is also incomplete—`BP_RANGE` is dead.

**The truly zero-reference dead code is those two types**: the `Precedence` enum
(`precedence.rs:35-94`, 60 lines) and the `PrecedenceContext` struct (`precedence.rs:98-133`, 36
lines), totaling **96 lines**. Verification result: these two types have zero use in production
code—all repository-wide grep hits for `PrecedenceContext` and `Precedence::`, outside of
`precedence.rs` itself, land in `src/frontend/core/parser/pratt/tests/precedence_inline.rs`.

**There is a detail that must be written into the document**: the one file that references the dead
code **is itself an orphan**. `pratt/tests/mod.rs:4-6` only declares
`mod led; mod nud; mod precedence;`, **without declaring `precedence_inline`**.

> That is: the 96 lines of dead code appear "to have test coverage" because they are tested by a
> 95-line / 6-test file, and that file never participated in compilation (see "Dead Code and Test
> Status"). **Deleting the dead code must also handle this orphan test file**, otherwise the
> deletion will break compilation.

Additionally, `precedence.rs:33` has a self-contradictory comment as corroboration: _"Precedence
rules for the Pratt parser"_—this comment describes precisely that zero-reference `Precedence` enum,
whose ladder is not the same as the actually effective ladder. The enum and the implementation have
diverged.

#### Numeric Collisions

The numbering of the overlapping segments of the two ladders is almost digit-for-digit
corresponding, so **cross-set collisions appear in pairs**:

| Conflict                    | Value    | Site A (First Set)             | Site B (Second Set)            |
| --------------------------- | -------- | ------------------------------ | ------------------------------ |
| Assign vs logical OR        | `1 == 1` | `BP_ASSIGN` (`led.rs:33`)      | `BP_OR` (`led.rs:42`)          |
| Type cast vs multiplication | `8 == 8` | `BP_CAST` (`led.rs:81`)        | `BP_MUL` (`led.rs:72`)         |
| Shift vs term               | `6 == 6` | `BP_TERM` (dead)               | `BP_SHIFT` (`led.rs:51`/`:54`) |
| Bitwise vs comparison       | `5 == 5` | `BP_COMPARISON` (comment only) | `BP_BIT` (`led.rs:58`)         |

**The first set also has one internal self-collision**: `BP_UNARY=8` and `BP_CAST=8`
(`precedence.rs:14` and `:15`)—**two different semantics within the same set share one number**, and
both are alive (used for the prefix binding in `nud.rs` and the cast in `led.rs:81`, respectively).
The second set has the same self-collision: `BP_RANGE=7` and `BP_ADD=7`.

**The reason it has not blown up so far is an unwritten convention**: almost all infix operators use
`bp_right = bp_left + 1`, so the right operand only accepts operators of **strictly higher** binding
power—this actually produces **left associativity** (`a - b - c` → `(a-b)-c`). So the only actually
effective comparison, "compare `bp_left`", will not go wrong because two operators have equal bp.
The `FatArrow` in `led.rs:85` uses `(11, 1)`, where `bp_right` is lower than `bp_left`, the only
explicit "right operand greedy consume" notation (lambda body maximizes to the right).

This is an **accidental correctness**: it relies on the fact that "the `bp_left` returned by
`infix_info` is unique, and `parse_expression_internal:100` only compares `bp_left`". Any change to
use `bp_left == bp_right` to distinguish associativity will immediately expose the conflict.

#### Three Bare Magic Numbers

| Magic Number | Location                             | Problem                                                                                                                   |
| ------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| `(6, 7)`     | `led.rs:40` `TokenKind::DotDot`      | Literal binding-power pair, no name; and **does not use `BP_RANGE` from the same set**, making `BP_RANGE` a dead constant |
| `(11, 1)`    | `led.rs:85` `TokenKind::FatArrow`    | 11 **exceeds any upper bound of the two ladders** (first set `BP_HIGHEST=10`)                                             |
| `12`         | `nud.rs:1041` `parse_expression(12)` | **Greater than `BP_HIGHEST=10`**, used for RFC-010b pattern probing                                                       |

The comment at `nud.rs:896-901` frankly admits the origin of this magic number: _"Lambda binding
power is 11, so we use 12 to stop before =>"_, _"parse_expression(12) will stop `ok` as a bare Var,
and `(` as an unexpected token"_. In other words, **this 12 was found by trial**, not derived.

#### Dispatch Is Hand-Written `match`

**Dispatch mechanism**: all are hand-written `match` returning function pointers, **no macros, no
tables, no generators**.

- `parse_prefix` (`nud.rs:14-19`) takes the `prefix_info()` closure;
- `parse_expression_internal` (`pratt/mod.rs:79-113`) loops over the
  `(bp_left, bp_right, parser_fn)` triples from `infix_info()`;
- two hand-written tables: `infix_info` (`led.rs:30-88`) and `prefix_info` (`nud.rs:26-75`).

#### `parse_assign_after_target`: A 507-Line Total Dispatcher with a Misleading Name

`src/frontend/core/parser/statements/declarations.rs:136`, `parse_assign_after_target`, function
body to `:642` (**507 lines**, file has 1015 lines total).

The name says "assign after target", but **in fact it is a total dispatcher for declaration forms**,
containing 8 independent responsibility sections: legacy-syntax detection / annotation
disambiguation three-layer nested lookahead / declaration validity / semantic side effects /
type-body parsing fallback / RFC-010 lambda signature parameter reconstruction / MetaType definition
/ plain initialization.

The bloat is caused by the superposition of three factors:

**Cause 1: dead language feature about 120 lines.**

`is_old_function_syntax` (`declarations.rs:82-117`), `skip_old_function_syntax`
(`declarations.rs:120-122`), and the accompanying lookahead at `:145-167`.

Need to specify precisely: **these two functions are not unreferenced**. They are called at
`declarations.rs:719` and `:725` by `parse_identifier_stmt`, via the path "detect `identifier(`
followed by type parameters and `->` → report 'legacy syntax deprecated' → skip → return None".

So the accurate statement is: **this is a still-running "reject removed syntax" gate**, with about
120 lines of overhead for each statement beginning with `identifier(` doing a parenthesis-balancing
scan. Of which:

- `skip_old_function_syntax` (`:120-122`) **the function body has only one line of comment**
  `// 旧语法已移除，此函数不再需要`, a **purely empty function**;
- more notably, after calling it (`:725-726`) it directly `return None`, **without consuming any
  token**. That is, this "skip" **actually skips nothing**—the subsequent tokens remain in the
  stream, and can only be caught by the caller's error recovery. This is an interface whose name
  does not match its behavior.

**Cause 2: lookahead logic is not abstracted.** "Skip matching parens" is **hand-written 4 times**
in the same file:

| Location                           | Line Range                |
| ---------------------------------- | ------------------------- |
| Inside `is_old_function_syntax`    | `declarations.rs:96-105`  |
| Inside `parse_assign_after_target` | `declarations.rs:147-156` |
| Same                               | `declarations.rs:191-200` |
| Same                               | `declarations.rs:224-233` |

Each is about 12 lines, with consistent structure: `paren_depth = 1` →
`while paren_depth > 0 && !at_end()` → `LParen` increments, `RParen` decrements → `bump()`. **The
four copies share no code.**

**Cause 3: type-layer logic leaked into the parser.** See RFC-039 routing table C for details:
`declarations.rs:27-80` calls type-layer `is_type_param_annotation` / `name_used_as_type`;
`declarations.rs:498-509` hard-codes the `"Terminates"` string and calls
`typecheck::operator_interfaces::spec()`. **This part belongs to 03-type-unification.md /
02-stage-contract.md; this document does not duplicate the treatment, only marks the boundary in the
"Detailed Design" section.**

#### AST Enumeration Overview

`src/frontend/core/parser/ast.rs`:

| Enumeration | Location         | Variant Count          |
| ----------- | ---------------- | ---------------------- |
| `Expr`      | `ast.rs:16-163`  | 22 (including `Error`) |
| `BinOp`     | `ast.rs:191-213` | 20                     |
| `UnOp`      | `ast.rs:217-223` | 4                      |
| `StmtKind`  | `ast.rs:234`     | 9 (including `Error`)  |
| `Type`      | `ast.rs:427-541` | 26                     |
| `Pattern`   | `ast.rs:794-814` | 8                      |

`Expr::Error` has been verified to exist (`pratt/mod.rs:72` handles it in span extraction, same at
`ast.rs:1048`); `StmtKind::Error` is constructed at `parser/mod.rs:50`. **These two error
placeholders are the cornerstone of backward-compatible design and must be preserved** (see
"Detailed Design · Backward Compatibility").

### Change Footprint of Adding an Operator

**The downstream change footprint is much larger than expected.** `BinOp::` has **534 hits across 47
files** in the repository, of which **17 are production files**. More critically, these files
consume **two unrelated `BinOp` enumerations**:

| Enumeration         | Definition                | Variant Count                              | Production Consumers (count of `BinOp::` hits in that file in parentheses)                                                                                                                                                               |
| ------------------- | ------------------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ast::BinOp`        | `parser/ast.rs:191-213`   | 20                                         | `led.rs`(22) `nud.rs`(1) `checker.rs`(8) `inference/expressions.rs`(25) `inference/statements.rs`(5) `operator_interfaces.rs`(5) `ir_gen.rs`(23) `formatter/handlers/expr.rs`(25) `lsp/handlers/inlay_hint.rs`(4) `spawn/analysis.rs`(3) |
| `const_data::BinOp` | `types/const_data.rs:234` | Verified to exist (`Ne` rather than `Neq`) | `const_data.rs`(36) `const_eval.rs`(64) `evaluator.rs`(13) `ownership.rs`(13) `termination.rs`(16) `proof/smt/translate.rs`(13) `proof/dep_graph.rs`(2)                                                                                  |

Verified: `src/backends/` has **zero hits** for `BinOp` and `ast::`—the interpreter consumes
IR/bytecode and never sees the AST. Therefore all 17 production files fall on the L2 → L3 link.

**There is no `From`/`TryFrom` between the two enumerations**—all repository greps for
`impl From<...BinOp`, `-> BinOp`, `BinOp as` have no conversion implementations. The cost is that
**every file touching both must alias at the import**: `const_eval.rs:19` (`BinOp as AstBinOp`),
`ir_gen.rs:2317` (`use ast::BinOp as B`), and `CEBinOp` / `ConstBinOp` in test files.

> **This is the most important finding in this document.** When `ast::BinOp` adds a variant, **the
> compiler has no way to remind you to change `const_data::BinOp`**, because they are two
> independent types, each independently compilable. The consequence of a miss: the constant folding
> path does not know the new operator, while the type-check and IR-generation paths do—the **same
> operator behaves differently in the two subsystems, with no diagnostic**.
>
> This is an order of magnitude more severe than "miss a `match` arm". The latter is caught by
> exhaustive matching; the former cannot be caught.

**Regarding "whether a miss is undetectable", we need to distinguish precisely**:

| Miss Type                                         | Caught by Compiler? | Verified Evidence                                                                                                                                                |
| ------------------------------------------------- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Exhaustive `match` arm miss                       | **Yes**             | `const_data.rs:291-317`'s `impl Display for BinOp` is an 18-arm exhaustive match with no `_` fallback                                                            |
| `matches!` macro enumeration miss                 | **No**              | `const_data.rs:263-288`'s `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` are all `matches!` partial listings; new variant → silently returns `false` |
| `match` with `_` fallback arm miss                | **No**              | `ownership.rs:687`'s `_ => {}` silently ignores new variants                                                                                                     |
| **Cross `ast::BinOp` / `const_data::BinOp` miss** | **Never**           | No conversion, no common parent type, compiler cannot associate them                                                                                             |

So the accurate statement is not "11 missed places, no one knows", but: **the compiler can save the
exhaustive-match part; what is truly silent is the `matches!` macro, `_` fallbacks, and cross-enum —
these three types**.

### Dead Code and Test Status

This is the **item that needs to be addressed immediately** in this document.

#### Dead Code

- `precedence.rs:35-94` `Precedence` enum (60 lines) and `precedence.rs:98-133` `PrecedenceContext`
  struct (36 lines): zero production use, the only external referrer is the orphan file
  `pratt/tests/precedence_inline.rs` (95 lines / 6 tests, not declared by `pratt/tests/mod.rs:4-6`).
- `skip_old_function_syntax` (`declarations.rs:120-122`): empty function, the call site at `:725` is
  followed by direct `return None`, no token consumed.
- `BP_RANGE` (`precedence.rs:22`): zero production use (only exists in the comment at
  `led.rs:157-159`).

#### Only One Test File Is Running in the Lexical Layer

**The only test declaration in `lexer/mod.rs:104-106` is:**

```rust
#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
```

`grep 'mod tests'` hits only this one place in the entire `src/frontend/core/lexer/` directory.
**`lexer/` has never declared `mod tests;`**.

Therefore the entire `src/frontend/core/lexer/tests/` subtree **has never participated in
compilation**. The directory actually has **14 files** (including `mod.rs`):

| File                                                                              | Line Count     | Status                                 |
| --------------------------------------------------------------------------------- | -------------- | -------------------------------------- |
| `mod.rs`                                                                          | 38             | Exists, declares 11 submodules         |
| `literals.rs`                                                                     | 222            | Real tests, **orphan**                 |
| `rfc010_lexer.rs`                                                                 | 167            | Real tests, **orphan**                 |
| `fstring.rs`                                                                      | 90             | Real tests, **wired in via `#[path]`** |
| `lexer_mod.rs`                                                                    | 89             | Real tests, **double orphan**          |
| `rfc004_lexer.rs`                                                                 | 81             | Real tests, **orphan**                 |
| `symbols.rs`                                                                      | 70             | Real tests, **double orphan**          |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 1-3 lines each | 7 empty shells                         |
| `debug_lexer.rs`                                                                  | 1              | Empty shell                            |

**Real but never-run tests: 629 lines / 5 files** (222+167+89+81+70). Only the f-string one file is
actually running in the lexical layer.

**A wiring trap that must be made clear: `tests/mod.rs` exists, but it is not fully wired either.**

`lexer/tests/mod.rs:15-25` declares 11 submodules (`basic` / `literals` / `operators` / `delimiters`
/ `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer` / `fstring`),
and `:28-38` then `pub use` all of them as "backward-compatible re-exports".

**But it does not declare `lexer_mod` and `symbols`.** Therefore:

> **Just adding one line `mod tests;` is not enough.** Reviving `lexer/tests/` requires **two**
> changes: add `mod tests;` in `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` in
> `tests/mod.rs`. Otherwise these 159 lines still will not run.

**The trap of the 7 empty shells**: statements like `pub use basic::*;` in `tests/mod.rs:28-38`
**produce no compile errors or warnings when referencing a zero-assertion module**. After adding
`mod tests;`, CI will show "all tests pass", while in fact the 7 modules `basic` / `comments` /
`delimiters` / `errors` / `keywords` / `operators` / `debug_lexer` have **zero assertions**. **This
will create the illusion that the lexical layer is covered**, which is more dangerous than not
wiring at all.

**Uncovered highest-risk paths** (will be exposed immediately once wired):

| Path                                                                    | Location                                                            | Risk                                |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------- | ----------------------------------- |
| Overflow path of the four radix scanners                                | `literals.rs:134-141` / `220-227` / `306-313` + decimal counterpart | Integer overflow is a silent error  |
| "Continue consuming but not report" branch after `checked_mul` overflow | 4 isomorphic places                                                 | May cause token-stream misalignment |
| The **entire** `scan_leading_dot` function                              | `literals.rs:519-635` (117 lines)                                   | Zero coverage                       |
| Illegal escape paths for `\x` / `\u`                                    | `literals.rs:730-749` / `1051-1070`                                 | Error paths are hard to test        |

**The parser-side wiring is normal**: `parser/mod.rs:8-9`, `pratt/mod.rs:8-9`,
`statements/mod.rs:11-12` all have correct `mod tests;`. `pratt/tests/mod.rs:4-6` declares `led`
(335 lines), `nud` (269 lines), `precedence` (13 lines).

**But the parser-side coverage quality is equally problematic**:

- `pratt/tests/precedence.rs` (**13 lines**) has only one assertion
  `assert!(bp_lowest < bp_highest)`, **nearly an empty run**—it cannot even detect the four
  cross-set collisions listed above.
- `pratt/tests/precedence_inline.rs` (**95 lines / 6 tests**) is not declared by
  `pratt/tests/mod.rs`, **never runs**—and it tests **the zero-reference `Precedence` enum +
  `PrecedenceContext`**.
- **The two actually-used ladders (`BP_OR`…`BP_MUL` and
  `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`) have no direct unit tests.**

## Target Design

### Lexical: Declarativization

#### Evaluation of Three Options

| Option                                            | Cost of Adding a Token     | Can Eliminate the Three Lexical Duplications | Can Solve f-string Nested Compilation       | Introduced Dependencies                                                      |
| ------------------------------------------------- | -------------------------- | -------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------- |
| **A. `logos` generating DFA**                     | 1 site (token declaration) | **Yes** (escape / number each one copy)      | No (interpolation must be handled manually) | New `logos`                                                                  |
| **B. `regex` hand-written DFA**                   | 1-2 sites                  | Partial                                      | No                                          | New `regex` (whether it is already a transitive dependency **not verified**) |
| **C. Keep hand-written, force merge duplication** | 1 site                     | **Yes** (manually extract common functions)  | **Yes** (requires explicit design)          | None                                                                         |

**Position: use C as the baseline, A as a conditional alternative, B not adopted.**

Rationale:

1. **C can eliminate all ~500 lines of duplication in the lexical layer with zero new
   dependencies**, and is fully within control. The three copies of escape decoding in `scan_string`
   / `scan_char` / `push_fstring_escape` are merged into a **single**
   `decode_escape(lexer, out) -> bool`, and the four radix scanners are merged into a **single**
   `scan_radix_number(lexer, base, digits_pred)`. This is pure mechanical refactoring, with the C5
   criterion of "same AST snapshot + same diagnostics".
2. **A (`logos`) capability is real**—it can collapse escape decoding and number scanning into one
   each. **But it cannot solve f-string nested compilation**, which is the most important defect in
   the lexical layer. At the same time, introducing a new dependency will change the `Literals`
   error type (the `InvalidEscape` / `InvalidNumber` variants of `LexError` need to be remapped),
   which will **directly violate C5's "same diagnostics" criterion** unless an additional adaptation
   layer is written—that is, another manual sync point.
3. **B (`regex` hand-written DFA) is not adopted**: it requires hand-writing state machines, with
   workload comparable to A but far less controllability. Since the choice is between A and B,
   logically A must be chosen; since this document takes C as the priority, there is no reason to
   choose B.

> **Design judgment (not a verified fact)**: A may be a better form in the **long run** (a
> declarative token table is itself a "change in one place" enforcement). But its migration cost and
> the conflict with the "same diagnostics" criterion make it unsuitable as content for this C5. If A
> is to be adopted later, it should be a separate proposal, using the test-revival results of this
> document as a safety net.

#### f-string Nested Compilation Must Be Addressed Explicitly

Regardless of choosing A or C, nested compilation must be addressed. **Position: cancel the runtime
nested `tokenize()`, change to one-shot production in the lexical phase.**

Concrete approach: when `scan_string` encounters the `f"` prefix, **within the same lexical scan**,
record the interpolation content as `FStringSegment::RawText(String)`, while **not** recursively
calling `tokenize()`. The actual tokenize + parse of the interpolation is deferred until the parser
first needs that f-string node, and:

- **done in one pass** (for N interpolations, only 1 tokenize of the interpolation segments, caching
  the result as `Vec<Vec<Token>>`), reducing N+1 to 1;
- **span mapping is determined in the lexical phase** (record the absolute offset of each
  interpolation in the source file), no longer relying on the relative semantics of `Point`.

This change is **independent of paradigm choice** and should be executed even without A.

### Syntax: Complete Grammar-Driven (LALRPOP)

> **Decision (2026-10-03, ChenXu233)**: **Adopt A (LALRPOP complete grammar-driven), do not do a
> half-baked table-ization.** The equivalence risk of A (LALRPOP's error recovery model differs from
> `synchronize()`; whether the trialed parsing behavior can be reproduced bit-for-bit) has been
> resolved by the **double-parser diff** scheme below; table-ization is an insufficient intermediate
> state.

#### Option Evaluation

| Option                   | Change Footprint of Adding a Binary Operator | Can Eliminate BP Conflicts                                                                        | Can Preserve `synchronize()` and Error Placeholders                      | Conclusion                           |
| ------------------------ | -------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------ |
| **A. LALRPOP**           | 1 grammar site + 1 action                    | Yes (LR table auto-computes priority; conflicts surface as grammar-conflict errors at build time) | Requires rewriting, but **error productions can be declared explicitly** | **Adopt**                            |
| **B. tree-sitter**       | 1 grammar.js site                            | Yes                                                                                               | Not applicable (produces CST, not domain AST)                            | **Reject**                           |
| C. Keep Pratt table-ized | 2-3                                          | Yes                                                                                               | Yes                                                                      | **Reject** (insufficient, see below) |

**B rejection reason** (unchanged): tree-sitter produces a general syntax tree; this project's
`Expr` / `StmtKind` is a **domain AST carrying semantic information** (`Expr::Cast` carries the
target type, `Expr::FString` carries the segment list, `Pattern::Struct` carries field names). CST →
domain AST still needs a complete mapping layer, **tree-sitter cannot save it**. It is equivalent to
adding a mapping layer of the same size outside the 1326-line `nud.rs`.

**C rejection reason**: table-ization solves **synchronization** (change one place without missing),
not **maintainability**—the 22 variants of `Expr` still need 22 hand-written construction actions,
the merge of the two BP ladders still needs to change about 89 references, and structural issues
like `Expr::Lambda { body: empty Block }` being borrowed as a parameter list (`nud.rs:505-514`) are
not touched at all. **It is an unnecessary intermediate stop between "completely hand-written" and
"complete grammar".**

#### Original Objections to A, and How They Are Resolved

The original objection to A is **equivalence**, not lack of capability:

- `ParserState::synchronize()` (`parser_state.rs:170-186`) combined with the `Expr::Error` /
  `StmtKind::Error` placeholders to form the "keep parsing as much as possible" semantics; LALRPOP's
  error recovery is a different model (insert / delete tokens), **the resulting diagnostic set must
  differ**
- The `bp_left < min_bp` exit condition at `pratt/mod.rs:100-102` and the `parse_expression(12)`
  pattern probe at `nud.rs:1041` are both **behaviors tried on specific inputs**; whether they can
  be reproduced bit-for-bit must be proven case by case

**Resolution: double-parser diff.** Not based on the argument that "the criteria should be the
same", but on **actually running**:

| Step | Content                                                                                                                                               | What It Proves                                                                |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| 1    | Build LALRPOP grammar + action code, **coexisting completely with existing Pratt**, without changing any existing code                                | —                                                                             |
| 2    | All 293 corpora + `src/std/tests`, **both parsers run once each**, ASTs are normalized per [07](07-equivalence-oracle.md) and **compared bit-by-bit** | **AST equivalence for valid programs is empirically proved, not by argument** |
| 3    | Cutover: `parse()` in `parser/mod.rs` is changed to call LALRPOP; Pratt is preserved as `parse_legacy()`                                              | —                                                                             |
| 4    | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + both ladders in `precedence.rs` + about 89 references                                                  | —                                                                             |
| 5    | Error recovery: `synchronize()` and `Expr::Error` / `StmtKind::Error` rebuilt on the LALRPOP side with **explicit error productions**                 | See below                                                                     |

**Step 2 is the core of the whole plan.** If any AST in the 293 corpora is non-equivalent, it will
be discovered before cutover—this is much stronger than "manually arguing case by case whether 89
references can be reproduced", and the cost is acceptable (one offline run).

#### Error Recovery Handling: Fully Preserve C5, No Relaxation

**Error diagnostics must be identical case by case; this is a hard requirement, not negotiable.**
The way to handle it is **to model existing behavior explicitly in the grammar**:

| Existing Behavior                                                                                                             | LALRPOP Equivalent                                                                                                                                                                                    |
| ----------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ParserState::synchronize()` (`parser_state.rs:170-186`): after a statement parse failure, skip tokens forward to sync points | Explicitly declare in the grammar "error → skip to sync-point set → continue" productions, **the skip set must match the existing implementation token by token**                                     |
| `Expr::Error` / `StmtKind::Error` placeholders: enable the parser to continue constructing a "partial AST" after an error     | Explicitly declare error-node productions in the grammar, **downstream match branch positions remain unchanged** (if downstream puts `Error` in a wildcard branch, errors will be silently swallowed) |
| Diagnostic code + span                                                                                                        | **Identical case by case. Wording unchanged.**                                                                                                                                                        |

**This is not "should be achievable", it must be achieved.** LALRPOP allows writing explicit error
productions; writing the sync-point set of `synchronize()` into the grammar is sufficient. If phase
3a/3b empirically proves that the existing diagnostics cannot be reproduced under LALRPOP, **the
correct action is to honestly report that finding and re-evaluate the plan (including "maintain
Pratt"), not to modify the criteria**.

Migration landing points and treatment:

| Item                                                              | Content                                                                                                                                                                                                                                    |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Grammar file                                                      | `src/frontend/core/parser/grammar/yaoxiang.lalrpop` (new directory, see [01](01-routing.md) target structure)                                                                                                                              |
| Action code                                                       | `grammar/actions.rs`—move the construction of `Expr` / `StmtKind` from `nud.rs` / `led.rs` over, **one function per variant**                                                                                                              |
| Error productions                                                 | Explicitly declared, combined with the `Expr::Error` / `StmtKind::Error` placeholders to preserve "keep parsing as much as possible" semantics                                                                                             |
| BP ladder merge                                                   | Both ladders deleted with the grammar; the 6 alive constants `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`, with their **~89 production references**, are replaced by priority declarations in the grammar |
| Bare magic numbers                                                | `(6,7)` / `(11,1)` / `12` disappear with Pratt—the LR table auto-computes priority                                                                                                                                                         |
| `is_old_function_syntax` (`declarations.rs:82-117`, 36 lines)     | **Naturally deleted**—under grammar-driven, `f(Int) -> Int = ...` cannot match any production, and a dedicated "probe removed syntax" is an anti-pattern                                                                                   |
| `Expr::Lambda { body: empty Block }` borrowing (`nud.rs:505-514`) | **Naturally eliminated**—parameter lists in the grammar are just parameter lists, no need to borrow AST nodes                                                                                                                              |
| `synchronize()` (`parser_state.rs:170-186`)                       | Preserve the API, change the call sites to be driven by error productions                                                                                                                                                                  |

**Net effect**: delete `nud.rs`(1326) + `led.rs`(495) + both ladders of `precedence.rs` + about 120
lines of dead legacy-syntax detection, add grammar file and action code. **The change footprint of
"adding a binary operator" drops from a dozen to 1 grammar site + 1 action**, and BP conflicts will
be **reported at grammar build time**, no longer relying on "accidental correctness" like now.

#### Prerequisites

1. **The 629 lines / 55 tests in `lexer/tests/` must be revived first** ([09](09-execution-wbs.md)
   P1). Switching out the entire parser without a frontend test safety net cannot satisfy 07
   "criteria must precede refactoring".
2. **AST normalization snapshots must be checked in first** (P2 second layer), otherwise step 2 has
   nothing to compare.
3. **f-string nested compilation elimination** (`literals.rs` refactoring) **is independent of this
   item**, and should be executed even without going to grammar-driven.

### Treatment of `parse_assign_after_target`

**Position: split into 8 named functions, function bodies only do dispatch.**

A 507-line, 8-section-responsibility function means any local modification must locate within 500
lines. The target form after splitting:

| Split-Out Function             | Responsibility                                           | Approx. Line Count | Attribution                                                                             |
| ------------------------------ | -------------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------------- |
| `probe_old_function_syntax`    | Legacy syntax detection                                  | 36                 | **Deleted in this document**                                                            |
| `resolve_annotation_ambiguity` | Annotation disambiguation (three-layer nested lookahead) | TBD                | This document (significantly shortened after extracting a standalone lookahead utility) |
| `check_declaration_legality`   | Declaration validity                                     | TBD                | This document                                                                           |
| `apply_semantic_side_effects`  | Semantic side effects                                    | TBD                | **03-type-unification.md** (includes `"Terminates"` hard-coding at `498-509`)           |
| `parse_type_body`              | Type-body parsing fallback                               | TBD                | This document                                                                           |
| `rebuild_lambda_params`        | RFC-010 lambda signature parameter reconstruction        | TBD                | This document                                                                           |
| `parse_meta_type_def`          | MetaType definition                                      | TBD                | This document                                                                           |
| `parse_plain_init`             | Plain initialization                                     | TBD                | This document                                                                           |

Supporting: collapse the 4 copies of "skip matching parens" (`96-105` / `147-156` / `191-200` /
`224-233`) into a single method `skip_balanced_parens()` on `ParserState`, with the 4 sites changed
to call it.

**Net effect (design judgment)**: 507 lines → 8 single-responsibility functions + 1 lookahead
utility; lookahead code drops from 48 lines to about 12 lines; after type-layer logic is moved out
(about 80 lines), the actual part belonging to this document is about 300 lines.

### Dead Code Cleanup (C6, No Equivalence Criterion Required)

Per the C6 definition in 07-equivalence-oracle.md ("pure deletion, no equivalence criterion
required, just confirm no references"):

| Cleanup Item                                | Location                                   | Line Count | Prerequisite                                                                                                      |
| ------------------------------------------- | ------------------------------------------ | ---------- | ----------------------------------------------------------------------------------------------------------------- |
| `Precedence` enum                           | `precedence.rs:35-94`                      | 60         | **Must simultaneously handle** `pratt/tests/precedence_inline.rs` (the only referrer, itself an orphan)           |
| `PrecedenceContext` struct                  | `precedence.rs:98-133`                     | 36         | Same as above                                                                                                     |
| 6 zero-reference constants in the first set | `precedence.rs:8-13`, `:11` (comment only) | 6          | Zero production references, delete directly                                                                       |
| `BP_RANGE`                                  | `precedence.rs:22`                         | 1          | Zero production references (only in the comment at `led.rs:157-159`); first change `..` to use the named constant |
| `skip_old_function_syntax`                  | `declarations.rs:120-122`                  | 3          | Empty function; the call site at `declarations.rs:725` is also deleted                                            |
| `is_old_function_syntax`                    | `declarations.rs:82-117`                   | 36         | See below                                                                                                         |
| `declarations.rs:145-167`                   | Same file                                  | 23         | Same as above                                                                                                     |
| Bare magic number `(6,7)`                   | `led.rs:40`                                | —          | Promote to named `BP_RANGE_L` / `BP_RANGE_R`                                                                      |
| Bare magic number `(11,1)`                  | `led.rs:85`                                | —          | Promote to named `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                    |
| Bare magic number `12`                      | `nud.rs:1041`                              | —          | Promote to `BP_PATTERN_PROBE`, and **record in a comment that it was found by trial**                             |

> **Note**: the 6 **alive** constants in the first ladder, `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` /
> `BP_CALL` / `BP_CAST` / `BP_HIGHEST`, are **not within this table's pure-deletion scope**—they
> have about 89 production references, and their merge belongs to the "Syntax: grammar-driven · C
> option" (C5 category) above, not C6 deletion.

**Treatment choice for `is_old_function_syntax` (design judgment)**:

It is not unreferenced dead code, but a **still-running "reject removed syntax" gate**
(`declarations.rs:719`). Two options:

- **Option 1 (aggressive)**: delete the entire block. A removed syntax, its "rejection" should be
  left to **the grammar definition itself**—under grammar-driven, `f(Int) -> Int = ...` cannot match
  any production, and naturally errors, no need for dedicated detection. **This is a dividend of the
  paradigm change**: about 120 lines naturally disappear under grammar-driven.
- **Option 2 (conservative)**: keep the gate but delete the empty function
  `skip_old_function_syntax`, and change "return None without consuming tokens" to actually skip.

**Position: Option 1**, because it is consistent with the "keep Pratt table-ized" position: since
the dispatch table is the single source of truth, hand-writing a dedicated "probe some removed
syntax" is an anti-pattern. **The premise is that the C5 criterion proves the diagnostic set is
unchanged**—it is necessary to empirically verify whether the diagnostic code and span of
`f(Int) -> Int = x => x` after deletion is the same as before. If not, fall back to Option 2 and
record it as an open question.

### Test Reconstruction

**This is the first priority and must precede any paradigm change.**

**Phase 1: revive `lexer/tests/`.** Requires **two** changes, not one:

1. Add `#[cfg(test)] mod tests;` after `:106` in `src/frontend/core/lexer/mod.rs`
2. Add `mod lexer_mod;` and `mod symbols;` in `src/frontend/core/lexer/tests/mod.rs` (otherwise
   these 159 lines still do not run)

**Phase 2: handle the 7 empty shells.** Key constraint: `pub use xxx::*;` in `tests/mod.rs:28-38`
referencing empty modules **produces no compile errors or warnings**, so after wiring, CI will show
"all green" with actual zero coverage. Treatment:

| Option                 | Action                                                                                        | Evaluation                                                                                                                            |
| ---------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| **Delete (preferred)** | Delete 7 empty shell files + corresponding `mod` declarations + corresponding `pub use` lines | **Clean.** The only value of empty shells is "fill later", but `literals.rs` / `rfc004_lexer.rs` already cover those responsibilities |
| Fill                   | Write real assertions for each empty shell                                                    | 7 files × content unknown, **cannot verify what should be tested** (not verified)                                                     |
| Keep + mark            | Keep but add `// TODO` and CI count warnings                                                  | Creates "covered" illusion, **oppose**                                                                                                |

**Position: delete**, and at the same time as the dead-code cleanup, **delete the `pub use`
re-exports at `tests/mod.rs:28-38` in full** (it was designed to work with a non-existent re-export
scheme, and it is exactly what hides the empty shells).

**Phase 3: add associativity cases.** Add `pratt/tests/binding_power.rs`, asserting on the
**actually effective two ladders**:

- strict bp increase within a level, no duplicates (**this will catch the four collisions above
  before the change**)
- write a minimum expression for each pair of adjacent levels, asserting its associativity direction
  (e.g. `a - b - c` → `(a-b)-c`)
- one set of cases each for the three special cases `DotDot` / `FatArrow` / pattern probe,
  **replacing the current empty run at `pratt/tests/precedence.rs:6-13`**

**Phase 4: add literal error path tests.** Add cases one by one for the four types in the uncovered
path table: four-radix overflow, "continue consuming" branch after `checked_mul`, the entire
`scan_leading_dot`, illegal `\x` / `\u` escapes.

**Phase 5: delete `precedence_inline.rs`** (along with the dead code deletion), and add a **wiring
self-check** assertion in `pratt/tests/mod.rs` to prevent orphans from reappearing.

### Before/After: How Many Places to Change for an Operator

| #   | Current                                                                        | Current Enforcement                          | Target                               | Target Enforcement                               |
| --- | ------------------------------------------------------------------------------ | -------------------------------------------- | ------------------------------------ | ------------------------------------------------ |
| 1   | `lexer/state.rs:21` `keyword_from_str` add a match arm (keyword form only)     | None                                         | 1 line in operator declaration table | Compile-time                                     |
| 2   | `lexer/tokens.rs:81-162` `TokenKind` add variant                               | Compile-time                                 | **Generated from table**             | Compile-time                                     |
| 3   | `pratt/precedence.rs:6-17` and `22-31` pick a set to add BP constant           | None                                         | **Generated from table**             | Compile-time (within-level uniqueness assertion) |
| 4   | `pratt/led.rs:30-88` `infix_info` add arm                                      | Compile-time (exhaustive)                    | **Generated from table**             | Compile-time                                     |
| 5   | `pratt/led.rs:116-136` `parse_binary` add op mapping arm                       | Compile-time (exhaustive)                    | **Generated from table**             | Compile-time                                     |
| 6   | `ast.rs:191-213` `BinOp` add variant                                           | Compile-time                                 | **Generated from table**             | Compile-time                                     |
| 7   | New symbol character: `tokenizer.rs:218` `next_token_inner`                    | None                                         | **Generated from table**             | Compile-time                                     |
| 8   | `const_data.rs:234` `const_data::BinOp` add variant                            | **None (cross-enum, compiler cannot catch)** | **Co-generated with `ast::BinOp`**   | Compile-time                                     |
| 9   | Sites in 17 downstream production files depending on `matches!` / `_` fallback | **None (silent)**                            | Generated accessors                  | Compile-time                                     |

**Summary:**

| Phase                             | Manual Change Sites                                                                                | Auto-Generated Sites |
| --------------------------------- | -------------------------------------------------------------------------------------------------- | -------------------- |
| Current                           | **11-12** (6-7 within L2 + about 5-6 production files that must be synchronized)                   | 0                    |
| Target (pure new symbol operator) | **1-2** (1 line in operator declaration table; if keyword form, plus 1 line in `keyword_from_str`) | 7-8                  |
| Target (reuse existing token)     | **1**                                                                                              | 6-7                  |

And the target's 1-2 sites are **all declarative data lines**, not logic code—review cost drops from
"deducing semantics per match arm" to "reading one line of declaration".

## Detailed Design

### Interface Boundary with the Type System

**Boundary declaration (important)**: the convergence of `ast::Type` (26 variants,
`ast.rs:427-541`), the elimination of parallel type representations, and the attribution of
`is_type_param_annotation` / `name_used_as_type` **all belong to
[03-type-unification.md](03-type-unification.md)**. This document only handles three interfaces
directly related to the frontend paradigm:

**Interface 1: the relationship between `const_data::BinOp` and `ast::BinOp`.** Attributed to **this
document**—because it is a direct component of the "add an operator" change footprint. Treatment:
**simultaneously generate both enumerations** from the operator declaration table (or generate one
and derive the other), eliminating the import aliases at `const_eval.rs:19` / `ir_gen.rs:2317`. The
reason for attributing to this document rather than 03: it is an L2 product parallel-copied by L3,
not a type-representation problem itself.

**Interface 2: the hard-coded `"Terminates"` and `typecheck::operator_interfaces::spec()` call at
`declarations.rs:498-509`.** Attributed to **03-type-unification.md / 02-stage-contract.md**
(eliminating the parser → typecheck reverse dependency, RFC-039 routing table C first row). This
document **isolates this code into `apply_semantic_side_effects`** during function splitting, as a
handoff point, but does not modify its content.

**Interface 3: whether the operator's priority level needs type-layer information.** Currently
no—`infix_info` is a pure syntactic function. **Design judgment**: this property should be preserved
after table-ization, and the operator declaration table **must not** contain any type-related
fields, otherwise L2 will re-depend on L3 (violating the dependency-direction specification of
RFC-039).

### Runtime Behavior

**Goal: runtime behavior unchanged bit-for-bit.** Per the C5 criteria of 07-equivalence-oracle.md,
the third layer (end-to-end corpus diff) compares the exit codes and stdout/stderr of the 293 `.yx`
files in `tests/yaoxiang/` + `src/std/tests/*.yx`.

Three **identified runtime impacts** must be explicitly handled:

| Change                                                         | Runtime Impact                                   | Treatment                                                                            |
| -------------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------ |
| f-string interpolation consolidated into 1 tokenize            | **Performance improvement**, output unchanged    | Need to empirically measure the time benefit and record it                           |
| Operator table generates BP / two ladders merged and reordered | **May change associativity**                     | **Must write equivalence cases for each level pair** (Test Reconstruction · Phase 3) |
| Delete `is_old_function_syntax`                                | Diagnostic code/span of legacy syntax may change | Need corpus diff to prove consistency                                                |

**Explicitly not done**: not changing the **semantics** of operator priority. BP numbers can be
reordered (eliminating dead code and collisions), but **relations** like "multiplication binds
tighter than addition" must be preserved.

### Compiler Change Inventory

By file, with line numbers. C6 cleanup (Phase 0, independently committable):

| File                                                        | Line Number                           | Change                                                                                                         |
| ----------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/pratt/precedence.rs`              | `8-13`, `11`                          | Delete the 6 zero-reference first-set constants                                                                |
| Same                                                        | `22`                                  | Change `BP_RANGE` to a named `..` binding power (synchronize with `:40`), otherwise delete                     |
| Same                                                        | `35-94`                               | Delete `Precedence` enum                                                                                       |
| Same                                                        | `98-133`                              | Delete `PrecedenceContext` struct                                                                              |
| Same                                                        | `6-17`, `22-31`                       | Keep alive constants, merge with the second set into conflict-free numbering within levels                     |
| `src/frontend/core/parser/pratt/led.rs`                     | `33`                                  | `BP_ASSIGN` points to the reordered assign-level constant                                                      |
| Same                                                        | `40`                                  | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R`                                                                          |
| Same                                                        | `75`, `77`, `79`, `83`                | `BP_CALL` points to the reordered postfix-level constant                                                       |
| Same                                                        | `81`                                  | `BP_CAST` points to the reordered cast-level constant                                                          |
| Same                                                        | `85`                                  | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                       |
| `src/frontend/core/parser/pratt/nud.rs`                     | `32`, `35`, `83`, `101`, `223`, `243` | `BP_UNARY` points to the reordered prefix-level constant                                                       |
| Same                                                        | `38-69`                               | `BP_HIGHEST` points to the reordered highest-level constant                                                    |
| Same                                                        | `1041`                                | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`, and add a comment explaining the value's origin |
| `src/frontend/core/parser/statements/declarations.rs`       | `120-122`                             | Delete `skip_old_function_syntax`                                                                              |
| Same                                                        | `725`                                 | Delete its call site (deletion of `is_old_function_syntax` see Phase 2)                                        |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | Full file                             | Delete (the only orphan file referencing dead code)                                                            |

Accompanying renames (collateral of BP merge, about 89 sites): `parser/mod.rs:80`,
`statements/control_flow.rs` 9 sites, `statements/declarations.rs:18` / `321` / `601` / `743` /
`818` / `850` / `966` / `994`, `statements/functions.rs:9` / `:110`, `statements/bindings.rs:168`,
`statements/types.rs:15` / `:641` / `:654`, `pratt/led.rs:99` / `:229` / `:321` / `:327` / `:415`,
`pratt/nud.rs:195` / `:263` / `:446` / `:516` / `
