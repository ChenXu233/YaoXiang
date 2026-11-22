# Frontend Paradigms: Lexing and Parsing

> **Companion design document**. This article is a companion document to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution-phase ordering are covered in the main
> RFC-039 text; the positioning of each companion document is described in the
> [index for this directory](index.md).

## Positioning and Scope

This article covers the L2 frontend—**lexing (`src/frontend/core/lexer/`) and parsing
(`src/frontend/core/parser/`)**.

**Problem to be solved**: reduce the change surface for “adding a binary operator” from the current
**6–7 locations within L2 + 17 downstream production files**—of which the compiler enforces
approximately 8–9 locations and the rest require manual synchronization—to **1–2 declarations + 7–8
automatically generated locations**.

This article implements the pain point identified in the first “Binary operator” row of RFC-039’s
feature-routing table A. The refactoring category is **C5 frontend paradigm change** as defined in
[07-equivalence-oracle.md](07-equivalence-oracle.md) (criteria: identical AST snapshots + identical
diagnostics + identical behavior, with all three used) and **C6 pure deletion** (no equivalence
criteria required, only confirmation that there are no references). C6 items are prerequisites for
C5: deleting 96 lines of dead code, eliminating three bare magic numbers, and reviving 629 lines of
lexer tests that have never run.

**Scope boundary**: handling `ast::Type` (26 variants) and converging the three parallel type
representations belongs to [03-type-unification.md](03-type-unification.md). This article does not
duplicate that work; it only declares the interface boundary in the “Detailed Design” section.

### Why This Is Worth Doing

“Adding an operator” is the most expensive kind of frontend change. This tax has **three
pathological characteristics**:

**First, the cost does not decrease with experience.** Someone who has added an operator ten times
will not need to change one location fewer than someone who has added it once, because there is no
inferable relationship between these locations. There is no connection between `(6,7)` in
`led.rs:40` and `12` in `nud.rs:1041`, yet they must agree with each other; otherwise expression
parsing breaks.

**Second, the consequences of an omission fall into two categories, and the compiler cannot catch
the worst category.** See the layered table in “The Change Surface for Adding an Operator”: an
omitted arm in an exhaustive `match` is caught by the compiler, whereas an omission across enums can
**never** be caught by the compiler.

**Third, this tax is invisible on the books.** Nearly every commit in
`git log --oneline -- src/frontend/core/parser/pratt/led.rs` touches three or four files
simultaneously, but no tool can identify “which location this commit forgot to change.”

### Existing Correct Patterns in This Repository

This project already has two **correct solutions to the same class of problem**, both of which can
be reused directly:

- **Error codes**: `build.rs:19-55` uses `tools/code-tables` to parse the registry at **build
  time**, validate uniqueness, and compare every entry against the Markdown code table in RFC-013.
  **Any inconsistency directly triggers `panic!` and rejects the build.** The cost of adding an
  error code is therefore a constant one location plus documentation.
- **Standard-library interfaces**: byte-for-byte comparison in `gen_interfaces.rs` and marked-region
  drift detection in `gen_docs.rs` (the last row of RFC-039’s routing table B).

What these two mechanisms have in common is that they **turn “changes that must be kept in sync”
from something people must remember into executable assertions**. The entire goal of this article
can be reduced to the same statement: turn the correspondences among the operator table,
lexical-rule table, and AST-variant table into executable assertions.

### Why “Just Delete the Dead Code” Is Not Enough

Dead code does exist (`precedence.rs` at 96 lines, the empty `skip_old_function_syntax` function,
and 629 lines of orphan tests), but deleting it will not reduce the number of locations changed when
adding an operator by even one. Dead code is **legacy debt**; the cross-file change surface is
**structural cost**. The two must be handled separately; see “Key Decisions and Rationale” for their
treatment.

## Current State

Everything below is a **verified fact**, with a file path and line number attached to each item. The
verification method and item-by-item measured results are documented in “Verification Record.”

### Lexical Layer: Copy-and-Paste Evidence in `literals.rs`

`src/frontend/core/lexer/literals.rs` contains **1,568 lines** in total. This size does not come
from complexity, but from three forms of structural duplication.

**Duplication 1: Four radix scanners are almost identical line by line.**

| Scanner               | Line range            | Number of lines |
| --------------------- | --------------------- | --------------- |
| `scan_hex_number`     | `literals.rs:77-161`  | 85              |
| `scan_octal_number`   | `literals.rs:164-247` | 84              |
| `scan_binary_number`  | `literals.rs:250-333` | 84              |
| `scan_decimal_number` | `literals.rs:336-516` | 181             |

The first three are each about 84 lines and have exactly the same structure: accumulate digit →
`has_digits` check → `overflow` flag → `checked_mul(base)` → `checked_add` → `try_into` → report an
error. **The only differences are the base constant and the digit-extraction expression.** Of the
three × 84 lines, **approximately 250 lines are pure redundancy** (`scan_decimal_number` must be
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

`scan_octal_number:220-227` and `scan_binary_number:306-313` are the same code with different bases
and message text.

**Duplication 2: Escape decoding is implemented three times.**

| Implementation                   | Location                       | Length                                       |
| -------------------------------- | ------------------------------ | -------------------------------------------- |
| The `\\` branch of `scan_string` | `literals.rs:694-827`          | 134 lines                                    |
| The `\\` branch of `scan_char`   | `literals.rs:1015-1141`        | 127 lines                                    |
| `push_fstring_escape`            | Starting at `literals.rs:1374` | Already extracted into a standalone function |

The first two are near-verbatim copies of 127–134 lines (`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` → push the
same value, identical `\x` / `\u{...}` logic, identical `c =>` error fallback), meaning
**approximately 250 lines have been written twice**.

A separate point deserves attention: **the f-string version has already been extracted into a
standalone function** (`push_fstring_escape:1374`). This shows that **someone already recognized the
duplication and fixed it locally, but never went back to converge the other two locations**. This is
more noteworthy than “none of the three sites recognized the problem”: it proves that the problem is
solvable and only lacks an enforcement mechanism.

**Duplication 3: Two multiline-string scanners.** `scan_multi_line_string:869` and
`scan_fstring_multi_line:1497` are another pair of similar implementations.

**Residual abnormal formatting.** The two sites at `literals.rs:777-787` and `1098-1108`, each 11
lines long, have three consecutive blank lines inserted between every field of a **struct literal**.
This is residue from an automated rewrite. Other locations in the same function (`1111-1115`) have
only a single blank line between fields, indicating that formatting was interrupted.

> This residue is harmless in itself, but it is a **diagnostic signal**: the same code was rewritten
> twice by different tools, meaning that someone once made a local patch here without reconciling
> the surrounding context.

**Conclusion**: approximately 500 lines in `literals.rs` (250 lines of redundant radix scanners +
two copies of 250 lines of escape decoding) are **mechanical duplication**, not complexity.

**f-string interpolation triggers nested compilation.** This is **repeated compilation at runtime**,
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

**At the parser layer, a complete `tokenize()` pass is run separately for every f-string
interpolation at runtime, and a new `ParserState` is created for it.**

Consequences: an f-string containing N interpolations **triggers N+1 complete lexical analyses** (N
interpolations + 1 outer string). At the same time, each interpolation is an **independent parsing
context**, so:

- Diagnostic spans within an interpolation are relative to the interpolation text; remapping them to
  source positions requires additional work.
- An interpolation cannot access any outer parser state. There is currently no need for this, but
  the structure rules out future possibilities, such as an interpolation referencing an implicit
  variable from the outer scope.
- The positions at which lexical and syntax errors are produced are split across two stages.

This is a **defect in both performance and structure** and must be addressed; see “Target Design ·
Lexing.”

### Syntax Layer: Two BP Ladders, Bare Magic Numbers, and Handwritten Dispatch Tables

#### The Two Ladders Are Not “One Live and One Dead”; They Are Interwoven

`src/frontend/core/parser/pratt/precedence.rs` currently contains **two binding-power ladders**,
whose numeric ranges overlap extensively.

**The first ladder (`precedence.rs:6-17`)**: `BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` /
`BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` /
`BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`.

**The second ladder (`precedence.rs:22-31`)**: `BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` /
`BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`.

**Measured conclusion: both ladders survive in production code and are already interwoven inside
`infix_info`.**

- `infix_info` (`led.rs:30-88`) references **five constants from the first ladder**: `BP_ASSIGN`
  (`:33`), `BP_CALL` (`:75` / `:77` / `:79` / `:83`), and `BP_CAST` (`:81`);
- It references **eight constants from the second ladder**: `BP_OR` (`:42`), `BP_AND` (`:44`),
  `BP_EQ` (`:47`), `BP_SHIFT` (`:51` / `:54`), `BP_BIT` (`:58`), `BP_CMP` (`:62` / `:65`), `BP_ADD`
  (`:68`), and `BP_MUL` (`:72`);
- **The only dead member of the second ladder is `BP_RANGE`**: `..` uses the bare literal `(6, 7)`
  at `led.rs:40` rather than `BP_RANGE`; in production code, `BP_RANGE` appears only in the comment
  at `led.rs:157-159`.

The survival status of the first ladder’s 12 constants, based on a repository-wide `grep`, is as
follows:

| Status              | Constant                                                                     | Production reference sites                                                                                                                                                                                       |
| ------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Dead (6 total)**  | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | No production references. Appears only in `precedence.rs` itself and the orphan test `precedence_inline.rs`                                                                                                      |
| Dead (comment only) | `BP_COMPARISON`                                                              | No production references. Mentioned only in the comment at `declarations.rs:742`                                                                                                                                 |
| **Live (6 total)**  | `BP_LOWEST`                                                                  | `parser/mod.rs:80`, nine locations in `statements/control_flow.rs`, six locations in `statements/declarations.rs`, `pratt/led.rs:229`/`:321`/`:327`/`:415`, `pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523`, etc. |
|                     | `BP_ASSIGN`                                                                  | `led.rs:33`, `led.rs:99`, `statements/declarations.rs:18` (import), `:743` (`BP_ASSIGN + 1`)                                                                                                                     |
|                     | `BP_UNARY`                                                                   | `nud.rs:32`, `:35`, `:83`, `:101`, `:223`, `:243`                                                                                                                                                                |
|                     | `BP_CALL`                                                                    | `led.rs:75`, `:77`, `:79`, `:83`                                                                                                                                                                                 |
|                     | `BP_CAST`                                                                    | `led.rs:81`                                                                                                                                                                                                      |
|                     | `BP_HIGHEST`                                                                 | Approximately 20 locations in `nud.rs:38-69` (one for each literal and keyword prefix)                                                                                                                           |

**Total: the six live constants have approximately 89 production references across eight files.**

> This corrects an easy but incorrect conclusion: “delete the first ladder” is **not** a 12-line
> deletion. It is a rename and consolidation affecting 89 references across eight production files.
> Only six constants in the first ladder truly have zero references (plus `BP_COMPARISON`, which
> survives only in a comment). The second ladder is also incomplete—`BP_RANGE` is dead.

**The truly zero-reference dead code is the two types**: the `Precedence` enum
(`precedence.rs:35-94`, 60 lines) and the `PrecedenceContext` struct (`precedence.rs:98-133`, 36
lines), totaling **96 lines**. Verification showed that these two types have zero production
uses—repository-wide searches for `PrecedenceContext` and `Precedence::` matched only
`precedence.rs` itself and `src/frontend/core/parser/pratt/tests/precedence_inline.rs`.

**One detail must be recorded in the documentation**: the only file referencing the dead code is
**itself an orphan**. `pratt/tests/mod.rs:4-6` declares only `mod led; mod nud; mod precedence;` and
**does not declare `precedence_inline`**.

> In other words, the 96 lines of dead code “appear to have test coverage” because they are tested
> by a 95-line file containing six tests that has never participated in compilation; see “Current
> State of Dead Code and Tests” for details. **Deleting the dead code must also handle this orphan
> test file**, or the build will fail after the deletion.

In addition, the comment at `precedence.rs:33` contradicts itself: _"Precedence rules for the Pratt
parser"_—this comment describes precisely the zero-reference `Precedence` enum, whose ladder is not
the ladder actually in effect. The enum and implementation have become disconnected.

#### Numeric Collisions

The numbers in the overlapping portions of the two ladders correspond almost digit for digit, so
**collisions occur in pairs across the ladders**:

| Collision                    | Value    | Usage site A (first ladder)    | Usage site B (second ladder)   |
| ---------------------------- | -------- | ------------------------------ | ------------------------------ |
| Assignment vs. logical OR    | `1 == 1` | `BP_ASSIGN` (`led.rs:33`)      | `BP_OR` (`led.rs:42`)          |
| Type cast vs. multiplication | `8 == 8` | `BP_CAST` (`led.rs:81`)        | `BP_MUL` (`led.rs:72`)         |
| Shift vs. term               | `6 == 6` | `BP_TERM` (dead)               | `BP_SHIFT` (`led.rs:51`/`:54`) |
| Bitwise vs. comparison       | `5 == 5` | `BP_COMPARISON` (comment only) | `BP_BIT` (`led.rs:58`)         |

**There is also one internal collision in the first ladder**: `BP_UNARY=8` and `BP_CAST=8`
(`precedence.rs:14` and `:15`) give two different semantics the same value within one ladder, yet
both are live—one is used for prefix binding in `nud.rs`, the other for the cast in `led.rs:81`. The
second ladder has an internal collision as well: `BP_RANGE=7` and `BP_ADD=7`.

**The system does not currently fail only because of an unwritten convention**: almost every infix
operator uses `bp_right = bp_left + 1`, causing the right operand to accept only operators with
**strictly higher** binding power. This actually produces **left associativity** (`a - b - c` →
`(a-b)-c`). Therefore, the only effective comparison, “compare `bp_left`,” does not fail merely
because two operators have equal binding powers. The `FatArrow` at `led.rs:85` uses `(11, 1)`, with
`bp_right` lower than `bp_left`, and is the only explicit “the right operand greedily consumes
everything” formulation (maximizing the lambda body to the right).

This is **accidentally correct**: it relies on the facts that the `bp_left` returned by `infix_info`
is unique and that `parse_expression_internal:100` compares only `bp_left`. Changing any site to use
a genuine `bp_left == bp_right` test to determine associativity would immediately expose the
collisions.

#### Three Bare Magic Numbers

| Magic number | Location                             | Problem                                                                                                                  |
| ------------ | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `(6, 7)`     | `led.rs:40` `TokenKind::DotDot`      | Literal binding-power pair with no names; also **does not use `BP_RANGE` from the same ladder**, leaving `BP_RANGE` dead |
| `(11, 1)`    | `led.rs:85` `TokenKind::FatArrow`    | 11 **exceeds the upper bound of either ladder** (the first ladder has `BP_HIGHEST=10`)                                   |
| `12`         | `nud.rs:1041` `parse_expression(12)` | **Greater than `BP_HIGHEST=10`**, used for RFC-010b pattern probing                                                      |

The comment at `nud.rs:896-901` explicitly explains the origin of this magic number: _"Lambda
binding power is 11, so we use 12 to stop before =>"_ and _"parse_expression(12) 会把 `ok`
停成裸 Var、`(` 成意外 token"_. In other words, **the 12 was determined experimentally rather than
derived**.

#### Dispatch Uses Handwritten `match` Expressions

**Dispatch mechanism**: all dispatch uses handwritten `match` expressions that return function
pointers, with **no macros, tables, or generators**.

- `parse_prefix` (`nud.rs:14-19`) obtains a closure from `prefix_info()`;
- `parse_expression_internal` (`pratt/mod.rs:79-113`) loops over the
  `(bp_left, bp_right, parser_fn)` tuple returned by `infix_info()`;
- Two handwritten tables exist: `infix_info` (`led.rs:30-88`) and `prefix_info` (`nud.rs:26-75`).

#### `parse_assign_after_target`: A 507-Line General Dispatcher with a Misleading Name

`parse_assign_after_target` at `src/frontend/core/parser/statements/declarations.rs:136` has a
function body ending at `:642` (**507 lines**, in a file totaling 1,015 lines).

Despite its name, “assign after target” is actually the **general dispatcher for declaration
forms**. It contains eight independent responsibility sections: old-syntax detection /
annotation-disambiguation lookahead with three levels of nesting / declaration legality / semantic
side effects / type-body parsing fallback / RFC-010 lambda-signature parameter reconstruction /
MetaType definitions / ordinary initialization.

Its growth results from three overlapping causes:

**Cause 1: Approximately 120 lines of dead language features.**

`is_old_function_syntax` (`declarations.rs:82-117`), `skip_old_function_syntax`
(`declarations.rs:120-122`), and the supporting lookahead at `:145-167`.

The precise explanation is important: **these two functions are not unreferenced**. They are called
at `declarations.rs:719` and `:725` by `parse_identifier_stmt`, along the path “detect `identifier(`
followed by type parameters and `->` → report ‘old syntax is deprecated’ → skip → return None.”

The accurate description is therefore: **this is a still-running gate that rejects removed syntax**,
costing approximately 120 lines to perform a parenthesis-balancing scan for every statement
beginning with `identifier(`. Specifically:

- `skip_old_function_syntax` (`:120-122`) has **only a one-line comment** in its body,
  `// 旧语法已移除，此函数不再需要`, and is a **pure empty function**;
- More notably, after calling it (`:725-726`), the code directly executes `return None` and **does
  not consume any tokens**. In other words, this “skip” **actually skips nothing**—the subsequent
  tokens remain in the stream and can only be handled by the caller’s error recovery. This is an
  interface whose name does not match its behavior.

**Cause 2: Lookahead logic has not been abstracted.** “Skip balanced parentheses” is handwritten
**four times** in the same file:

| Location                           | Line range                |
| ---------------------------------- | ------------------------- |
| Inside `is_old_function_syntax`    | `declarations.rs:96-105`  |
| Inside `parse_assign_after_target` | `declarations.rs:147-156` |
| Same                               | `declarations.rs:191-200` |
| Same                               | `declarations.rs:224-233` |

Each is approximately 12 lines with the same structure: `paren_depth = 1` →
`while paren_depth > 0 && !at_end()` → increment on `LParen`, decrement on `RParen` → `bump()`.
**None of the four copies shares any code.**

**Cause 3: Type-layer logic has leaked into the parser.** See RFC-039’s routing table C:
`declarations.rs:27-80` calls the type-layer functions `is_type_param_annotation` and
`name_used_as_type`; `declarations.rs:498-509` hardcodes the string `"Terminates"` and calls
`typecheck::operator_interfaces::spec()`. This material belongs to 03-type-unification.md /
02-stage-contract.md and is not handled again here; it is only marked as a boundary in the “Detailed
Design” section.

#### Complete AST Enum Inventory

`src/frontend/core/parser/ast.rs`:

| Enum       | Location         | Number of variants     |
| ---------- | ---------------- | ---------------------- |
| `Expr`     | `ast.rs:16-163`  | 22 (including `Error`) |
| `BinOp`    | `ast.rs:191-213` | 20                     |
| `UnOp`     | `ast.rs:217-223` | 4                      |
| `StmtKind` | `ast.rs:234`     | 9 (including `Error`)  |
| `Type`     | `ast.rs:427-541` | 26                     |
| `Pattern`  | `ast.rs:794-814` | 8                      |

`Expr::Error` has been verified to exist (its span is handled at `pratt/mod.rs:72`, and at
`ast.rs:1048`); `StmtKind::Error` is constructed at `parser/mod.rs:50`. **These two error
placeholders are the foundation of the backward-compatible design and must be preserved**; see
“Detailed Design · Backward Compatibility.”

### The Change Surface for Adding an Operator

**The downstream change surface is much larger than expected.** `BinOp::` has **534 matches across
47 files** in the repository, including **17 production files**. More importantly, these files
consume **two unrelated `BinOp` enums**:

| Enum                | Definition                | Number of variants                  | Production consumers (parentheses show the number of `BinOp::` matches in that file)                                                                                                                                                              |
| ------------------- | ------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ast::BinOp`        | `parser/ast.rs:191-213`   | 20                                  | `led.rs`(22), `nud.rs`(1), `checker.rs`(8), `inference/expressions.rs`(25), `inference/statements.rs`(5), `operator_interfaces.rs`(5), `ir_gen.rs`(23), `formatter/handlers/expr.rs`(25), `lsp/handlers/inlay_hint.rs`(4), `spawn/analysis.rs`(3) |
| `const_data::BinOp` | `types/const_data.rs:234` | Verified to exist (`Ne`, not `Neq`) | `const_data.rs`(36), `const_eval.rs`(64), `evaluator.rs`(13), `ownership.rs`(13), `termination.rs`(16), `proof/smt/translate.rs`(13), `proof/dep_graph.rs`(2)                                                                                     |

It has been verified that `src/backends/` has **zero matches** for either `BinOp` or `ast::`—the
interpreter consumes IR/bytecode and never sees the AST. Therefore, all 17 production files lie on
the L2 → L3 path.

**There is no `From`/`TryFrom` between the two enums**—repository-wide searches for
`impl From<...BinOp`, `-> BinOp`, and `BinOp as` found no conversion implementation. The cost is
that **every file that touches both must create an alias at the import site**: `const_eval.rs:19`
(`BinOp as AstBinOp`), `ir_gen.rs:2317` (`use ast::BinOp as B`), and `CEBinOp` / `ConstBinOp` in
test files.

> **This is the most important finding in this document.** When `ast::BinOp` gains a variant, **the
> compiler cannot remind you to update `const_data::BinOp`**, because they are independent types
> that can each compile independently. If the update is omitted, the constant-folding path does not
> understand the new operator, while the type-checking and IR-generation paths do—the **same
> operator behaves differently in two subsystems, with no diagnostic**.
>
> This is an order of magnitude more serious than “omitting one `match` arm.” The latter is caught
> by exhaustive matching; the former cannot be caught at all.

**Whether an omission goes undetected must be stated precisely**:

| Omission type                                          | Detected by the compiler? | Verified evidence                                                                                                                                                            |
| ------------------------------------------------------ | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Missing arm in an exhaustive `match`                   | **Yes**                   | `impl Display for BinOp` at `const_data.rs:291-317` is an 18-arm exhaustive match with no `_` fallback                                                                       |
| Missing item in a `matches!` macro enumeration         | **No**                    | The `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` functions at `const_data.rs:263-288` are all partial `matches!` lists; a new variant silently returns `false` |
| Missing arm in a `match` with a `_` fallback           | **No**                    | The `_ => {}` at `ownership.rs:687` silently ignores the new variant                                                                                                         |
| **Omission across `ast::BinOp` / `const_data::BinOp`** | **Never**                 | The two have no conversion and no common parent type, so the compiler has no way to relate them                                                                              |

The accurate statement is therefore not “11 omissions went unnoticed,” but: **the compiler covers
the exhaustive-match portion; the truly silent omissions are `matches!` macros, `_` fallbacks, and
cross-enum omissions.**

### Current State of Dead Code and Tests

This is the item in this document that **most urgently needs to be addressed**.

#### Dead Code

- `precedence.rs:35-94` `Precedence` enum (60 lines) and `precedence.rs:98-133` `PrecedenceContext`
  struct (36 lines): zero production uses; the only external referrer is the orphan file
  `pratt/tests/precedence_inline.rs` (95 lines / six tests), which is not declared by
  `pratt/tests/mod.rs:4-6`.
- `skip_old_function_syntax` (`declarations.rs:120-122`): a pure empty function; the call site at
  `:725` directly returns `None` without consuming a token.
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
location. **`lexer/` has never declared `mod tests;`.**

The entire `src/frontend/core/lexer/tests/` subtree has therefore **never participated in
compilation**. The directory actually contains **14 files** (including `mod.rs`):

| File                                                                              | Number of lines | Status                                  |
| --------------------------------------------------------------------------------- | --------------- | --------------------------------------- |
| `mod.rs`                                                                          | 38              | Exists; declares 11 submodules          |
| `literals.rs`                                                                     | 222             | Real tests, **orphaned**                |
| `rfc010_lexer.rs`                                                                 | 167             | Real tests, **orphaned**                |
| `fstring.rs`                                                                      | 90              | Real tests, **wired through `#[path]`** |
| `lexer_mod.rs`                                                                    | 89              | Real tests, **doubly orphaned**         |
| `rfc004_lexer.rs`                                                                 | 81              | Real tests, **orphaned**                |
| `symbols.rs`                                                                      | 70              | Real tests, **doubly orphaned**         |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 1–3 lines each  | Seven empty shells                      |
| `debug_lexer.rs`                                                                  | 1               | Empty shell                             |

**Real but never-run tests: 629 lines across five files** (222+167+89+81+70). The only lexer test
file currently running is the f-string file.

**One wiring trap must be made explicit: `tests/mod.rs` exists, but it does not itself wire
everything up.**

`lexer/tests/mod.rs:15-25` declares 11 submodules (`basic` / `literals` / `operators` / `delimiters`
/ `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer` / `fstring`),
and `:28-38` then `pub use`s all of them for “backward-compatible re-export.”

**However, it does not declare `lexer_mod` or `symbols`.** Therefore:

> **Adding only `mod tests;` is not enough.** Reviving `lexer/tests/` requires **two changes**: add
> `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` to `tests/mod.rs`.
> Otherwise, those 159 lines still do not run.

**Trap in the seven empty shells**: statements such as `pub use basic::*;` in `tests/mod.rs:28-38`
that reference a zero-assertion module produce neither a compilation error nor a warning. After
adding `mod tests;`, CI will report that all tests pass even though `basic` / `comments` /
`delimiters` / `errors` / `keywords` / `operators` / `debug_lexer` contain **not a single
assertion**. **This creates the false impression that the lexer is covered and is more dangerous
than not wiring the tests at all.**

**Uncovered paths with the highest risk**—all of which will be exposed immediately once the tests
are wired:

| Path                                                              | Location                                                                      | Risk                               |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------- |
| Overflow paths of the four radix scanners                         | `literals.rs:134-141` / `220-227` / `306-313` + corresponding decimal section | Integer overflow is a silent error |
| The “continue consuming after overflow from `checked_mul`” branch | Four isomorphic locations                                                     | May misalign the token stream      |
| The **entire `scan_leading_dot` function**                        | `literals.rs:519-635` (117 lines)                                             | Zero coverage                      |
| Invalid `\x` / `\u` escape paths                                  | `literals.rs:730-749` / `1051-1070`                                           | Error paths are difficult to test  |

**Wiring on the parser side is correct**: `parser/mod.rs:8-9`, `pratt/mod.rs:8-9`, and
`statements/mod.rs:11-12` all correctly declare `mod tests;`. `pratt/tests/mod.rs:4-6` declares
`led` (335 lines), `nud` (269 lines), and `precedence` (13 lines).

**However, parser-side coverage quality has the same problem**:

- The only assertion in `pratt/tests/precedence.rs` (**13 lines**) is
  `assert!(bp_lowest < bp_highest)`, which **does almost nothing**—it cannot even detect the four
  cross-ladder collisions listed above.
- `pratt/tests/precedence_inline.rs` (**95 lines / six tests**) is not declared by
  `pratt/tests/mod.rs` and **never runs**. Moreover, it tests **only the zero-reference `Precedence`
  enum and `PrecedenceContext`**.
- **The two ladders actually in use** (`BP_OR`…`BP_MUL` and
  `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`) **have no direct unit
  tests**.

## Target Design

### Lexing: Declarative Transformation

#### Evaluation of Three Options

| Option                                                   | Cost of adding a token           | Can eliminate the lexer’s three duplications?   | Can solve f-string nested compilation?      | Introduced dependency                                                                    |
| -------------------------------------------------------- | -------------------------------- | ----------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------- |
| **A. Generate a DFA with `logos`**                       | One location (token declaration) | **Yes** (one copy each for escapes and numbers) | No (interpolation requires manual handling) | New `logos` dependency                                                                   |
| **B. Handwritten DFA with `regex`**                      | 1–2 locations                    | Partially                                       | No                                          | New `regex` dependency (whether it is already a transitive dependency is **unverified**) |
| **C. Retain handwritten code and enforce deduplication** | One location                     | **Yes** (manually extract common functions)     | **Yes** (explicit design required)          | None                                                                                     |

**Position: adopt C as the baseline, A as a conditional alternative, and reject B.**

Rationale:

1. **C can eliminate all approximately 500 lines of duplication in the lexer with zero new
   dependencies**, entirely within a controllable scope. The three escape-decoding
   implementations—`scan_string` / `scan_char` / `push_fstring_escape`—are merged into a **single**
   `decode_escape(lexer, &mut String, EscapeMode)`, and the four radix scanners are merged into a
   **single** `scan_radix_number(lexer, Radix, String)`. This is a purely mechanical refactoring,
   with C5’s “identical AST snapshots + identical diagnostics” as its criterion.
2. **The capabilities of A (`logos`) are real**—it can converge escape decoding and number scanning
   to one implementation each. **However, it cannot solve f-string nested compilation**, which is
   the lexer’s most important defect. Adding a new dependency would also change `Literals`’ error
   types (`LexError`’s `InvalidEscape` / `InvalidNumber` variants would need to be remapped), which
   would **directly violate C5’s “identical diagnostics” criterion** unless an additional adapter
   were written—and that would introduce another manual synchronization point.
3. **B (handwritten DFA with `regex`) is rejected**: it requires a handwritten state machine, with
   workload comparable to A but far less controllability. If choosing between A and B, the logic
   requires choosing A; because this article advocates C first, there is no reason to choose B.

> **Design judgment, not a verified fact**: A may be the better long-term shape, because a
> declarative token table is itself a mechanism that enforces “change one location.” However, its
> migration cost and conflict with the “identical diagnostics” criterion make it unsuitable for this
> C5 change. If A is adopted later, it should be a separate proposal, using this article’s
> test-revival results as a safety net.

#### f-String Nested Compilation Must Be Addressed Explicitly

Regardless of whether A or C is chosen, nested compilation must be solved. **Position: eliminate
runtime nested `tokenize()` and produce the result in a single lexical pass.**

The concrete approach: when `scan_string` encounters the `f"` prefix, it **records the interpolation
contents as `FStringSegment::RawText(String)` within the same lexical scan** and **does not**
recursively call `tokenize()`. The actual tokenization and parsing of interpolations are deferred
until the parser first needs the f-string node, at which point:

- **It is completed in one pass** (for N interpolations, perform one tokenization pass over the
  interpolation segments and cache the result as `Vec<Vec<Token>>`), reducing N+1 passes to one;
- **Span mapping is determined during lexing** (recording the absolute offset of each interpolation
  in the source file), rather than depending on the relative semantics of `Point`.

This change is **independent of the paradigm choice** and should be performed even if A is not
adopted.

### Syntax: Complete Grammar-Driven Parsing (LALRPOP)

> **Decision (2026-10-03, ChenXu233)**: **Adopt A (complete LALRPOP grammar-driven parsing); do not
> implement a partial table-driven approach.** A’s equivalence risk—the difference between LALRPOP’s
> error-recovery model and `synchronize()`, and whether experimentally determined parsing behavior
> can be reproduced bit for bit—is resolved by the **double-parser differential** approach below.
> Table-driven parsing is an insufficient intermediate state.

#### Option Evaluation

| Option                               | Change surface for adding a binary operator | Can eliminate BP collisions?                                                                                           | Can preserve `synchronize()` and error placeholders?                     | Conclusion                           |
| ------------------------------------ | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------ |
| **A. LALRPOP**                       | One grammar location + one action location  | Yes (LR tables calculate precedence automatically, and conflicts are exposed as grammar-conflict errors at build time) | Requires rewriting, but **error productions can be declared explicitly** | **Adopt**                            |
| **B. tree-sitter**                   | One `grammar.js` location                   | Yes                                                                                                                    | Not applicable (produces a CST, not a domain AST)                        | **Reject**                           |
| C. Retain Pratt and introduce tables | 2–3 locations                               | Yes                                                                                                                    | Yes                                                                      | **Reject** (insufficient; see below) |

**Reason for rejecting B** (unchanged): tree-sitter produces a generic syntax tree, whereas this
project’s `Expr` / `StmtKind` are **domain ASTs with semantic information** (`Expr::Cast` carries a
target type, `Expr::FString` carries a segment list, and `Pattern::Struct` carries field names).
Mapping a CST to a domain AST still requires a complete mapping layer, so **tree-sitter cannot
eliminate that layer**. This would amount to adding a mapping layer of comparable size on top of the
1,326-line `nud.rs`.

**Reason for rejecting C**: table-driven parsing solves **synchronization** (“change one location
and never miss another”), but not **maintainability**—the 22 variants of `Expr` still require 22
manually written construction actions, merging the two BP ladders still requires changing
approximately 89 references, and structural problems such as `Expr::Lambda { body: empty Block }`
being borrowed as a parameter list (`nud.rs:505-514`) are not addressed at all. **It is an
unnecessary intermediate station between “entirely handwritten” and “complete grammar.”**

#### Original Objections to A and How They Are Resolved

The original objection to A was **equivalence**, not lack of capability:

- The “continue parsing where possible” semantics formed by `ParserState::synchronize()`
  (`parser_state.rs:170-186`) and the `Expr::Error` / `StmtKind::Error` placeholders use LALRPOP’s
  different error-recovery model (token insertion/deletion), so **the resulting diagnostic sets must
  differ**.
- The `bp_left < min_bp` exit condition at `pratt/mod.rs:100-102` and the `parse_expression(12)`
  pattern probe at `nud.rs:1041` are **behaviors determined experimentally on specific inputs**, and
  whether they can be reproduced bit for bit requires proof for each one.

**Resolution: double-parser differential.** This does not rely on an argument that the criteria
“should be equivalent”; it relies on **running the comparison**:

| Step | Content                                                                                                                                                                                             | What it proves                                                                |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| 1    | Build the LALRPOP grammar and action code, **fully coexisting with the existing Pratt parser**, without changing any existing code                                                                  | —                                                                             |
| 2    | Run **both parsers** over all 293 corpus files plus everything under `src/std/tests`, normalize the ASTs according to the rules in [07](07-equivalence-oracle.md), and **compare them bit for bit** | **AST equivalence for valid programs is proven by measurement, not argument** |
| 3    | Switch traffic: change `parse()` in `parser/mod.rs` to call LALRPOP; retain Pratt as `parse_legacy()`                                                                                               | —                                                                             |
| 4    | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + both ladders in `precedence.rs` + approximately 89 references                                                                                        | —                                                                             |
| 5    | Error recovery: reconstruct `synchronize()` and the `Expr::Error` / `StmtKind::Error` placeholders on the LALRPOP side using **explicit error productions**                                         | See below                                                                     |

**Step 2 is the core of the approach.** If any of the 293 corpus ASTs differs, the problem is found
before traffic is switched. This is much stronger than “manually proving whether each of the 89
references can be reproduced,” and its cost is acceptable because it is run offline once.

#### Treatment of Error Recovery: Fully Preserve C5 Without Relaxation

**Error diagnostics must be identical in every respect. This is a hard requirement, not a negotiable
one.** The approach is to **model the existing behavior explicitly in the grammar**:

| Existing behavior                                                                                                                             | LALRPOP counterpart                                                                                                                                                                                       |
| --------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ParserState::synchronize()` (`parser_state.rs:170-186`): after a statement fails to parse, skip tokens forward until a synchronization point | Declare explicit “error → skip to the synchronization-point set → continue” productions in the grammar; **the skipped-token set must be token-for-token identical to the current implementation**         |
| `Expr::Error` / `StmtKind::Error` placeholders: allow the parser to continue constructing a “partial AST” after an error                      | Declare error-node productions explicitly; **the positions of downstream match branches remain unchanged** (if downstream code places `Error` in a wildcard branch, the error will be silently swallowed) |
| Diagnostic code + span                                                                                                                        | **Identical in every respect. Wording remains unchanged.**                                                                                                                                                |

**This is not something that “should be possible”; it must be achieved.** LALRPOP permits explicit
error productions; the synchronization-point set from `synchronize()` can be written into the
grammar. If phases 3a/3b demonstrate through measurement that the existing diagnostics cannot be
reproduced under LALRPOP, **the correct action is to report the finding honestly and reassess the
approach, including “retain Pratt,” not to modify the criteria.**

Migration landing points and treatment:

| Item                                                                    | Content                                                                                                                                                                                                                                                     |
| ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Grammar file                                                            | `src/frontend/core/parser/grammar/yaoxiang.lalrpop` (new directory; see the target structure in [01](01-routing.md))                                                                                                                                        |
| Action code                                                             | `grammar/actions.rs`—move the construction of `Expr` / `StmtKind` from `nud.rs` / `led.rs`, with **one function per variant**                                                                                                                               |
| Error productions                                                       | Explicitly declared, preserving the “continue parsing where possible” semantics through the `Expr::Error` / `StmtKind::Error` placeholders                                                                                                                  |
| BP-ladder consolidation                                                 | Both ladders are deleted with the grammar; the six live constants `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` and their **approximately 89 production references** are replaced by precedence declarations in the grammar |
| Bare magic numbers                                                      | `(6,7)` / `(11,1)` / `12` disappear together with Pratt—the LR tables calculate precedence automatically                                                                                                                                                    |
| `is_old_function_syntax` (`declarations.rs:82-117`, 36 lines)           | **Naturally deleted**—with grammar-driven parsing, `f(Int) -> Int = ...` cannot match any production, making dedicated probing for removed syntax an anti-pattern                                                                                           |
| Borrowed use of `Expr::Lambda { body: empty Block }` (`nud.rs:505-514`) | **Naturally eliminated**—a parameter list is a parameter list in the grammar; there is no need to borrow an AST node                                                                                                                                        |
| `synchronize()` (`parser_state.rs:170-186`)                             | Retain the API, with its call sites driven by error productions                                                                                                                                                                                             |

**Net effect**: delete `nud.rs`(1326) + `led.rs`(495) + both ladders in `precedence.rs` +
approximately 120 lines of dead old-syntax probing, while adding a grammar file and action code.
**The change surface for “adding a binary operator” drops from more than ten locations to one
grammar location + one action location**, and BP collisions become **grammar-build-time errors**
rather than being “accidentally correct.”

#### Prerequisites

1. **The 629 lines / 55 tests in `lexer/tests/` must first be revived** (P1 in
   [09](09-execution-wbs.md)). Without a frontend-test safety net, replacing the entire parser
   cannot satisfy the requirement in 07 that “criteria must precede refactoring.”
2. **Normalized AST snapshots must first be committed** (the second layer of P2); otherwise there is
   no basis for the comparison in step 2.
3. **Eliminating f-string nested compilation** (`literals.rs` refactoring) is **independent of this
   item** and should be performed even if grammar-driven parsing is not adopted.

### Treatment of `parse_assign_after_target`

**Position: split it into eight named functions, with each function body responsible only for
dispatch.**

A 507-line function with eight responsibility sections requires every local change to be located
within 500 lines. The target shape after splitting is:

| Function after splitting       | Responsibility                                               | Approximate number of lines | Ownership                                                                              |
| ------------------------------ | ------------------------------------------------------------ | --------------------------- | -------------------------------------------------------------------------------------- |
| `probe_old_function_syntax`    | Old-syntax detection                                         | 36                          | **Deleted by this article**                                                            |
| `resolve_annotation_ambiguity` | Annotation disambiguation (three levels of nested lookahead) | To be verified              | This article (substantially shortened after extracting a standalone lookahead utility) |
| `check_declaration_legality`   | Declaration legality                                         | To be verified              | This article                                                                           |
| `apply_semantic_side_effects`  | Semantic side effects                                        | To be verified              | **03-type-unification.md** (including the `"Terminates"` hardcoding at `498-509`)      |
| `parse_type_body`              | Type-body parsing fallback                                   | To be verified              | This article                                                                           |
| `rebuild_lambda_params`        | RFC-010 lambda-signature parameter reconstruction            | To be verified              | This article                                                                           |
| `parse_meta_type_def`          | MetaType definitions                                         | To be verified              | This article                                                                           |
| `parse_plain_init`             | Ordinary initialization                                      | To be verified              | This article                                                                           |

Supporting change: converge the four copies of “skip balanced parentheses” (`96-105` / `147-156` /
`191-200` / `224-233`) into a single `ParserState` method, `skip_balanced_parens()`, and replace all
four copies with calls to it.

**Net effect (design judgment)**: 507 lines → eight single-responsibility functions + one lookahead
utility; lookahead code falls from 48 lines to approximately 12 lines; after type-layer logic is
moved out (approximately 80 lines), the portion actually owned by this article is approximately 300
lines.

### Dead-Code Cleanup (C6, No Equivalence Criteria Required)

Under the C6 definition in 07-equivalence-oracle.md (“pure deletion, no equivalence criteria
required, only confirmation that there are no references”):

| Cleanup item                                       | Location                                   | Number of lines | Prerequisite                                                                                                             |
| -------------------------------------------------- | ------------------------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `Precedence` enum                                  | `precedence.rs:35-94`                      | 60              | **Must also handle** `pratt/tests/precedence_inline.rs` (the only referrer, itself an orphan)                            |
| `PrecedenceContext` struct                         | `precedence.rs:98-133`                     | 36              | Same as above                                                                                                            |
| Six zero-reference constants from the first ladder | `precedence.rs:8-13`, `:11` (comment only) | 6               | Zero production references; delete directly                                                                              |
| `BP_RANGE`                                         | `precedence.rs:22`                         | 1               | Zero production references (survives only in the comment at `led.rs:157-159`); first change `..` to use a named constant |
| `skip_old_function_syntax`                         | `declarations.rs:120-122`                  | 3               | Empty function; remove its call site at `declarations.rs:725`                                                            |
| `is_old_function_syntax`                           | `declarations.rs:82-117`                   | 36              | See below                                                                                                                |
| `declarations.rs:145-167`                          | Same file                                  | 23              | Same as above                                                                                                            |
| Bare magic number `(6,7)`                          | `led.rs:40`                                | —               | Promote to named `BP_RANGE_L` / `BP_RANGE_R` constants                                                                   |
| Bare magic number `(11,1)`                         | `led.rs:85`                                | —               | Promote to `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                                 |
| Bare magic number `12`                             | `nud.rs:1041`                              | —               | Promote to `BP_PATTERN_PROBE`, and **document in a comment that it was determined experimentally**                       |

> **Note**: `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` from the
> first ladder **are not within the pure-deletion scope of this table**. They have approximately 89
> production references, and their consolidation belongs to “Syntax: Grammar-Driven · Option C”
> above (the C5 category), not to C6 deletion.

**Treatment options for `is_old_function_syntax` (design judgment)**:

It is not unreferenced dead code; it is **a still-running gate that rejects removed syntax**
(`declarations.rs:719`). There are two options:

- **Option 1 (aggressive)**: delete the entire section. Rejection of removed syntax should be
  delegated to **the grammar definition itself**—after grammar-driven parsing, `f(Int) -> Int = ...`
  cannot match any production and naturally produces an error, so dedicated probing is unnecessary.
  **This is a benefit of the paradigm change**: approximately 120 lines naturally disappear after
  grammar-driven parsing.
- **Option 2 (conservative)**: retain the gate but delete the empty function
  `skip_old_function_syntax`, and change “return `None` without consuming tokens” to actually skip
  them.

**Choose Option 1**, because it is consistent with the position of retaining Pratt table-driven
parsing: if the dispatch table is the single source of truth, manually probing for a particular
removed syntax is an anti-pattern. **The prerequisite is that the C5 criteria prove that the
diagnostic set remains unchanged**—the diagnostic code and span for `f(Int) -> Int = x => x` must be
measured before and after deletion. If they differ, revert to Option 2 and record it as an open
issue.

### Test Reconstruction

**This is the first priority and must precede every paradigm change.**

**Phase 1: Revive `lexer/tests/`.** This requires **two changes**, not one:

1. Add `#[cfg(test)] mod tests;` after `:106` in `src/frontend/core/lexer/mod.rs`.
2. Add `mod lexer_mod;` and `mod symbols;` to `src/frontend/core/lexer/tests/mod.rs`; otherwise,
   those 159 lines still do not run.

**Phase 2: Handle the seven empty shells.** The key constraint is that statements such as
`pub use xxx::*;` at `tests/mod.rs:28-38` that reference empty modules **produce neither compilation
errors nor warnings**. After wiring succeeds, CI will report “all green” despite the `basic` /
`comments` / `delimiters` / `errors` / `keywords` / `operators` / `debug_lexer` modules containing
**not a single assertion**. **This creates the false impression of lexer coverage and is more
dangerous than leaving the tests unwired.**

| Option                   | Approach                                                                                              | Assessment                                                                                                                                        |
| ------------------------ | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Delete (recommended)** | Delete the seven empty-shell files + corresponding `mod` declarations + corresponding `pub use` lines | **Clean.** The only value of empty shells is “fill them in later,” while `literals.rs` / `rfc004_lexer.rs` already perform these responsibilities |
| Populate                 | Write real assertions for each empty shell                                                            | The contents of the seven files are unknown, and **it cannot be verified what they should test** (unverified)                                     |
| Retain + mark            | Retain them but add `// TODO` and CI count warnings                                                   | Creates a false impression of coverage; **opposed**                                                                                               |

**Delete them**, and at the same time as the dead-code cleanup, delete the entire `pub use`
re-export block at `tests/mod.rs:28-38` (it existed to support a nonexistent re-export design and is
precisely what concealed the empty shells).

**Phase 3: Add associativity cases.** Add `pratt/tests/binding_power.rs`, asserting against **the
two ladders actually in effect**:

- Levels within each consolidated ladder strictly increase, with no duplicates (this assertion
  catches the four collisions listed above even before the refactoring);
- Write one minimal expression for each pair of adjacent levels and assert its associativity
  direction (for example, `a - b - c` → `(a-b)-c`);
- Add one test group for each of the three special cases: `DotDot`, `FatArrow`, and pattern probing,
  **replacing the currently ineffective assertion at `pratt/tests/precedence.rs:6-13`**.

**Phase 4: Add literal error-path cases.** Add a case for each of the four categories in the
uncovered-path table: overflow in all four radix scanners, the branch that continues consuming after
`checked_mul`, `scan_leading_dot` as a whole, and invalid `\x` / `\u` escapes.

**Phase 5: Delete `precedence_inline.rs`** along with the dead code, and add a **wiring self-check**
assertion to `pratt/tests/mod.rs` to prevent another orphan.

### Before-and-After Comparison: How Many Locations Must Change When Adding an Operator?

| #   | Current state                                                                      | Current enforcement                                  | Target                                             | Target enforcement                                 |
| --- | ---------------------------------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------- | -------------------------------------------------- |
| 1   | Add a `match` arm to `keyword_from_str` in `lexer/state.rs:21` (keyword form only) | None                                                 | One line in the operator declaration table         | Compile-time                                       |
| 2   | Add a variant to `TokenKind` in `lexer/tokens.rs:81-162`                           | Compile-time                                         | **Generated from the table**                       | Compile-time                                       |
| 3   | Choose one ladder and add a BP constant at `pratt/precedence.rs:6-17` or `22-31`   | None                                                 | **Generated from the table**                       | Compile-time (uniqueness assertion within a level) |
| 4   | Add an arm to `infix_info` in `pratt/led.rs:30-88`                                 | Compile-time (exhaustive)                            | **Generated from the table**                       | Compile-time                                       |
| 5   | Add an operator-mapping arm to `parse_binary` in `pratt/led.rs:116-136`            | Compile-time (exhaustive)                            | **Generated from the table**                       | Compile-time                                       |
| 6   | Add a variant to `BinOp` in `ast.rs:191-213`                                       | Compile-time                                         | **Generated from the table**                       | Compile-time                                       |
| 7   | New symbolic character: `tokenizer.rs:218` `next_token_inner`                      | None                                                 | **Generated from the table**                       | Compile-time                                       |
| 8   | Add a variant to `const_data::BinOp` in `const_data.rs:234`                        | **None (cross-enum; the compiler cannot detect it)** | **Generated from the same source as `ast::BinOp`** | Compile-time                                       |
| 9   | Sites in 17 downstream production files that rely on `matches!` / `_` fallbacks    | **None (silent)**                                    | Generated accessors                                | Compile-time                                       |

**Summary:**

| Stage                            | Number of manual changes                                                                                                | Number of automatically generated locations |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| Current state                    | **11–12** (6–7 within L2 + approximately 5–6 downstream production files that must be synchronized)                     | 0                                           |
| Target (new symbolic operator)   | **1–2** (one line in the operator declaration table; one additional line in `keyword_from_str` if it uses keyword form) | 7–8                                         |
| Target (reuse an existing token) | **1**                                                                                                                   | 6–7                                         |

Moreover, the target’s one or two locations are **all declarative data rows**, not logic code. This
reduces review cost from “deduce the semantics of each `match` arm individually” to “read one
declaration.”

## Detailed Design

### Interface Boundary with the Type System

**Boundary declaration (important)**: convergence of `ast::Type` (26 variants, `ast.rs:427-541`),
elimination of parallel type representations, and ownership of `is_type_param_annotation` /
`name_used_as_type` **all belong to [03-type-unification.md](03-type-unification.md)**. This article
handles only the three interfaces directly related to the frontend paradigm:

**Interface 1: the relationship between `const_data::BinOp` and `ast::BinOp`.** This belongs to
**this article**, because it is a direct component of the “add an operator” change surface.
Treatment: **simultaneously generate both enums from the operator declaration table** (or generate
one and derive the other), eliminating the import aliases at `const_eval.rs:19` / `ir_gen.rs:2317`.
The reason it belongs here rather than in 03 is that it is an L2 output copied in parallel into L3,
not a problem with type representation itself.

**Interface 2: the hardcoded `"Terminates"` at `declarations.rs:498-509` and the call to
`typecheck::operator_interfaces::spec()`.** This belongs to **03-type-unification.md /
02-stage-contract.md** (eliminating the parser → typecheck reverse dependency, the first row of
RFC-039’s routing table C). During function splitting, this article **isolates this code into
`apply_semantic_side_effects`** as a handoff point, but does not change its contents.

**Interface 3: whether an operator’s precedence level requires type-layer information.** It does not
currently—`infix_info` is a purely syntactic function. **Design judgment**: table-driven parsing
must preserve this property, and the operator declaration table **must not contain any type-related
fields**, or L2 will again depend on L3, violating RFC-039’s dependency-direction rules.

### Runtime Behavior

**Goal: runtime behavior remains bit-for-bit identical.** Under the C5 criteria in
07-equivalence-oracle.md, the third layer (end-to-end corpus differential) compares exit codes and
stdout/stderr for the 293 `.yx` files in `tests/yaoxiang/` and `src/std/tests/*.yx`.

The three **identified runtime impacts** must be handled explicitly:

| Change                                                              | Runtime impact                                     | Treatment                                                                               |
| ------------------------------------------------------------------- | -------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Consolidating f-string interpolation into one tokenization pass     | **Performance improvement**, with unchanged output | Measure and record the performance gain                                                 |
| Generating BPs from the operator table / reordering the two ladders | **May change associativity**                       | **Must write an equivalence case for every level pair** (Test Reconstruction · Phase 3) |
| Deleting `is_old_function_syntax`                                   | The diagnostic code/span for old syntax may change | Prove equivalence through the corpus differential                                       |

**Explicitly out of scope**: do not change the **semantics** of operator precedence. BP numbers may
be reordered to eliminate dead code and collisions, but relationships such as “multiplication binds
more tightly than addition” must remain unchanged.

### Compiler Change Checklist

File-by-file, with line numbers. C6 cleanup (phase 0, independently committable; **8.0.1 / 8.0.2
were executed on 2026-11-22; the line numbers below refer to the tree before execution, and the
final state is documented in [09a](./09a-wbs-history.md) §P8 8.0**):

| File                                                        | Lines                                 | Change                                                                                                           |
| ----------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/pratt/precedence.rs`              | `8-13`, `11`                          | Delete the six zero-reference constants from the first ladder                                                    |
| Same                                                        | `22`                                  | Change `BP_RANGE` to the named `..` binding power (`:40` updated in sync), otherwise delete it                   |
| Same                                                        | `35-94`                               | Delete the `Precedence` enum                                                                                     |
| Same                                                        | `98-134`                              | Delete the `PrecedenceContext` struct                                                                            |
| Same                                                        | `6-17`, `22-31`                       | Retain live constants and merge them with the second ladder into numbering with no conflicts within a level      |
| `src/frontend/core/parser/pratt/led.rs`                     | `33`                                  | Point `BP_ASSIGN` to the reordered assignment-level constant                                                     |
| Same                                                        | `40`                                  | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R`                                                                            |
| Same                                                        | `75`, `77`, `79`, `83`                | Point `BP_CALL` to the reordered postfix-level constants                                                         |
| Same                                                        | `81`                                  | Point `BP_CAST` to the reordered cast-level constant                                                             |
| Same                                                        | `85`                                  | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                         |
| `src/frontend/core/parser/pratt/nud.rs`                     | `33`, `36`, `84`, `102`, `228`, `248` | Point `BP_UNARY` to the reordered prefix-level constants                                                         |
| Same                                                        | `39-70`                               | Point `BP_HIGHEST` to the reordered highest-level constant                                                       |
| Same                                                        | `1019`                                | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`, with a comment explaining the origin of the value |
| `src/frontend/core/parser/statements/declarations.rs`       | `85-87`                               | Delete `skip_old_function_syntax`                                                                                |
| Same                                                        | `695`                                 | Delete its call site (deletion of `is_old_function_syntax` is handled in phase 2)                                |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | Entire file                           | Delete (the orphan file that is the only referrer of the dead code)                                              |

Supporting renames (the connected surface of BP consolidation, approximately 89 locations):
`parser/mod.rs:113`, eight locations in `statements/control_flow.rs`,
`statements/declarations.rs:19` / `287` / `577` / `705` / `785` / `821` / `940` / `972`,
`statements/functions.rs:9` / `:107`, `statements/bindings.rs:170`, `statements/types.rs:16` /
`:782` / `:794`, `pratt/led.rs:99` / `:227` / `:315` / `:321` / `:409`, and `pratt/nud.rs:200` /
`:268` / `:418` / `:471` / `:478`.

Test revival (phase 1, independently committable):

| File                                                 | Lines                   | Change                                                                       |
| ---------------------------------------------------- | ----------------------- | ---------------------------------------------------------------------------- |
| `src/frontend/core/lexer/mod.rs`                     | `167-168`               | Add `#[cfg(test)] mod tests;`                                                |
| `src/frontend/core/lexer/tests/mod.rs`               | `12-18`                 | Add `mod lexer_mod;` and `mod symbols;`                                      |
| Same                                                 | Entire file             | Delete the `mod` declarations and `pub use` lines for the seven empty shells |
| Same                                                 | Entire file             | Delete the entire `pub use ... ::*;` re-export block (`28-38`)               |
| `src/frontend/core/lexer/tests/`                     | Seven empty-shell files | Delete the files                                                             |
| `src/frontend/core/parser/pratt/tests/mod.rs`        | `4-7`                   | Add a wiring self-check assertion                                            |
| `src/frontend/core/parser/pratt/tests/precedence.rs` | `6-13`                  | Replace the ineffective assertion with real associativity assertions         |

Paradigm changes (phases 2–4):

| File                                                           | Change                                                                                                                                                                                                                                                    |
| -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/lexer/literals.rs`                          | Merge the four radix scanners into `scan_radix_number(lexer, Radix, String)`; merge the three escape-decoding implementations into a single `decode_escape(lexer, &mut String, EscapeMode)`; merge `scan_multi_line_string` and `scan_fstring_multi_line` |
| `src/frontend/core/parser/pratt/nud.rs:367` / `:398`           | Remove runtime nested `tokenize()` and produce the result in a single lexical pass                                                                                                                                                                        |
| `src/frontend/core/parser/pratt/led.rs:30-88` / `nud.rs:27-76` | Replace the two handwritten tables with generated lookup tables                                                                                                                                                                                           |
| `src/ast/mod.rs:194-218`                                       | Generate `BinOp` and `const_data::BinOp` from a common source                                                                                                                                                                                             |
| `src/frontend/core/types/const_data.rs:290`                    | Same as above                                                                                                                                                                                                                                             |
| `src/frontend/core/parser/statements/declarations.rs:101-620`  | Split into eight functions + `ParserState::skip_balanced_parens()` (replace the four locations at `61-70`/`112-120`/`156-164`/`186-194` with calls)                                                                                                       |
| `src/frontend/core/parser/parser_state.rs`                     | Add `skip_balanced_parens()` (`synchronize()` is at `:330` and remains unchanged)                                                                                                                                                                         |

**Files with no changes under this article**: the **main bodies** of `src/middle/core/ir_gen.rs`,
`src/frontend/core/typecheck/**`, `src/backends/**`, and `src/frontend/core/formatter/**`. They are
changed only when generated accessors replace `matches!` macros, and those changes are mechanical
substitutions.

### Backward Compatibility

The aspect of table-driven parsing most likely to break is **error recovery**. This project’s
recovery contract consists of three parts, each of which must be preserved:

**Contract 1: `ParserState::synchronize()` (`parser_state.rs:170-186`).** After a statement fails to
parse, it skips tokens forward until a synchronization point. **The C5 criteria require preserving
it, and require that the timing and skipped-token set remain exactly the same.** After migration to
LALRPOP, the call sites of this function are driven by explicit error productions, while the
function body and synchronization-point set **remain unchanged line for line**—the grammar
reproduces the semantics, and the implementation is retained as-is.

**Contract 2: `Expr::Error` (`ast.rs:1048`, consumed at `pratt/mod.rs:72`) and `StmtKind::Error`
(constructed at `parser/mod.rs:50`).** These two placeholders allow the parser to continue
constructing a “partial AST” after an error, with downstream code skipping through their `Error`
branches. **The variants must be preserved, and their positions in downstream matches must also be
preserved**; if downstream code places `Expr::Error` in a wildcard branch, the error will be
silently swallowed.

**Contract 3: diagnostic code, span, and order.** The comparison rules in the third layer of
07-equivalence-oracle.md sort the diagnostic list by `(code, span.file, span.line)`; **message text
does not participate in the comparison**. This gives this article some latitude over wording, but
**code and span are immutable**.

**Special note on f-strings**: the interpolation refactoring changes spans from “relative to the
interpolation text” to “absolute source offsets.” To satisfy Contract 3, **spans must be recorded
using absolute offsets during lexing** (Decision D25); diagnostic `span.file/line` values within
interpolations must not change because of this refactoring.

> **Treatment**: before phase 2, establish a baseline snapshot of “diagnostic spans within f-string
> interpolations” and compare every entry after the refactoring. Any mismatch in `span.file/line` is
> an implementation defect that must be fixed; there is no “accept the span change” option. (If
> measurement proves that the original interpolation spans were already incorrect, the
> baseline-establishment phase will expose that as an existing, independent defect. Fix it
> separately, do not fold it into this phase, and do not treat it as grounds for relaxing the
> criteria.)

## Implementation Points

This article corresponds to five phases. **Each phase can be committed and rolled back
independently.** Phases 0 and 1 are C6 and test revival and require no equivalence criteria; phases
2–4 are C5 and are constrained by the criteria throughout.

| Phase  | Content                                                                                                                                | Category    | Acceptance criteria                                                                                                                                      | Rollback point                            |
| ------ | -------------------------------------------------------------------------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| **0**  | Dead-code cleanup (`Precedence` enum + `PrecedenceContext`, 96 lines)                                                                  | C6          | `cargo build` passes; `cargo clippy` reports no new warnings                                                                                             | Single commit; `git revert` is sufficient |
| **1**  | Test revival and completion                                                                                                            | —           | Lexer actually runs tests from ≥ 5 files / 629 lines; seven empty shells deleted; associativity cases **fail before the refactoring and pass afterward** | Single commit                             |
| **2**  | Lexical deduplication + elimination of f-string nested compilation                                                                     | C5          | Identical AST snapshots + identical diagnostics + identical behavior; `literals.rs` reduced by approximately 500 lines                                   | Committed separately from phase 3         |
| **3a** | Build the LALRPOP grammar and action code, **fully coexisting with Pratt**                                                             | —           | `cargo build` passes; the new parser can produce ASTs for the 293 corpus files (without comparison)                                                      | Committed separately from phase 2         |
| **3b** | **Double-parser differential** (run both parsers over the 293 corpus files and **compare normalized ASTs + diagnostic code and span**) | **Full C5** | **ASTs and diagnostics are equivalent entry by entry—this step is the equivalence proof for the entire grammar migration**                               | Run offline once; repeatable              |
| **3c** | Switch traffic: change `parse()` in `parser/mod.rs` to call LALRPOP; retain Pratt as `parse_legacy()`                                  | **Full C5** | Full-corpus behavioral equivalence + identical diagnostic code and span                                                                                  | Independent commit                        |
| **3d** | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + both ladders in `precedence.rs` + approximately 89 references                           | **Full C5** | Entire corpus passes; `git grep BP_` has zero matches                                                                                                    | Independent commit                        |
| **4**  | Split `parse_assign_after_target`                                                                                                      | C5          | The function only dispatches; `skip_balanced_parens` has a single implementation                                                                         | Independent commit                        |

**Dependency ordering within this article**: phase 1 **must precede phases 2/3/4**—“refactor first,
add tests afterward” is a pitfall this project has already encountered. **Phase 3b (the
double-parser differential) cannot be skipped**: it must pass completely before traffic is switched
in 3c; otherwise, the entire grammar migration has no equivalence basis.

**Go/no-go gate for 3b/3c**: if the number of error productions clearly exceeds what is needed to
model “the synchronization-point set from `synchronize()`”—particularly the hardest parts to port,
such as the parser-embedded semantic decisions involving `declare_predicate` accumulation and const
generic-parameter filtering—or if the differential continues to produce non-equivalent items after
two consecutive iterations, **stop the migration and reassess**, falling back to the intermediate
approach of “retain Pratt + a single declarative precedence table + same-source generation of the
two `BinOp` enums.” This approach already captures most of the benefit of reducing the operator
change surface. LALRPOP is the means, not the goal. Falling back is part of the “reassessment”
branch reserved by D24, **not a relaxation of the criteria**.

**One clarification about phase 0**: the six **live** first-ladder constants `BP_LOWEST` /
`BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` **are not within phase 0’s deletion
scope**—they are deleted together with Pratt in phase 3d. Phase 0 deletes only the `Precedence` enum
and `PrecedenceContext` (96 lines with zero production references).

**How the criteria are executed**: see [07-equivalence-oracle.md](07-equivalence-oracle.md) for the
definitions of the three layers and the regression gate. **Use the full C5 criteria throughout,
including diagnostic code and span. No relaxation exists.**

**Scope qualification**: this article **does not include** a `logos` migration (the lexer remains
handwritten, with duplication merged; see “Key Decisions and Rationale”). **The LALRPOP syntax
migration is part of this article**, as described in “Syntax: Complete Grammar-Driven Parsing.”

## Key Decisions and Rationale

### Three Selected Decisions

| Decision                         | Resolution                                                                                                    | Rationale                                                                                                                                                                                                                                                                                                         |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexer approach                   | Adopt “retain handwritten code + enforce deduplication”; list `logos` as a conditional alternative            | Eliminates approximately 500 lines of duplication with zero new dependencies; `logos` changes the `LexError` error types and cannot solve f-string nested compilation                                                                                                                                             |
| **Syntax approach**              | **Adopt complete LALRPOP grammar-driven parsing**; reject tree-sitter and “retain Pratt table-driven parsing” | tree-sitter produces a CST, not a domain AST, and still requires a complete mapping layer; table-driven parsing solves synchronization but not maintainability and is an insufficient intermediate state. LALRPOP’s equivalence risk is resolved by a **double-parser differential** (see above), not by argument |
| Ownership of `const_data::BinOp` | Assign to this article                                                                                        | It is a direct component of the “add an operator” change surface and is handled separately from `ast::Type` convergence (03)                                                                                                                                                                                      |

### Explanation of the Commitment to “Doing It Completely”

Table-driven parsing is an insufficient intermediate station: it solves synchronization, not
maintainability. Waiting passively for the criteria to be relaxed and then reassessing is not an
engineering decision. This article therefore adopts **complete grammar-driven parsing**, with
equivalence guaranteed through measured double-parser differentials (see above) and no relaxation at
the diagnostic layer.

**The cost of this change must be stated clearly**:

- Construction effort rises significantly: a grammar file must be created, 22 `Expr` construction
  actions must be moved, approximately 89 BP references must be consolidated, and error recovery
  must be rewritten.
- **The prerequisites become stricter**: the 629 lines of frontend tests must first be revived (P1
  in 09), and normalized AST snapshots must first be committed (P2 in 09). Without both, the
  differential in step 2 cannot begin.
- **A new dependency** (LALRPOP) is introduced, which differs from the lexer layer’s “zero new
  dependencies” direction.

**The benefits are certain**: “adding an operator” drops from 11 locations to two, BP collisions
change from “accidentally correct” to “reported at grammar-build time,” and the borrowed use of
`Expr::Lambda`, the `parse_expression(12)` magic number, and approximately 120 lines of dead
old-syntax probing **naturally disappear**.

### Rejected or Downgraded Options

**A. Delete only dead code, without a paradigm change.** Verified as feasible, but insufficient.
Dead-code cleanup (approximately 130 lines + three magic numbers) can be completed at **zero
criterion cost** and should be done immediately. However, it does not reduce the L2 change surface
for “adding an operator” by even one location, and cross-enum omissions are **not addressed at
all**. **Conclusion: adopt as phases 0 and 1, but continue with grammar-driven parsing. Treating A
as the complete solution is false reassurance.**

**B. Retain Pratt and only add tests.** **Not adopted as a complete solution, but adopted as a
prerequisite.** Tests **cannot detect cross-enum omissions**—the two enums can each compile
independently, and whether the tests pass is unrelated to whether the enums are synchronized.
**Conclusion: adopt as phase 1; reject as a complete solution.**

**C. Retain Pratt and introduce tables.** **Rejected** (2026-10-03). This is an unnecessary
intermediate station between “entirely handwritten” and “complete grammar”: the 22 variants of
`Expr` still require 22 handwritten construction actions, and the borrowed use of
`Expr::Lambda { body: empty Block }`, the `parse_expression(12)` magic number, and all 89 references
involved in merging the two ladders remain.

**D. Adopt tree-sitter.** **Rejected.** Tree-sitter’s incremental parsing provides no material
benefit for this project (full compilation on every run, 293-file corpus scale), and it produces a
CST rather than a domain AST, requiring a complete mapping layer and therefore adding comparable
code volume outside `nud.rs`.

**E. Replace the handwritten lexer with `logos`.** **Conditionally not adopted.** **Prerequisites
for reassessment**: after phase 2 completes (escape decoding has converged to a single
implementation), replacing that implementation with `logos` callbacks will require substantially
less work.

### Net Benefit of the Approach

- **The change surface drops from “more than ten logic locations” to “1–2 declarative data
  locations + 7–8 generated locations.”** The generated seven or eight locations are kept
  synchronized by the compiler, making omissions impossible.
- **One class of defects that the compiler can never catch is eliminated.** Cross-enum omissions
  (`ast::BinOp` vs. `const_data::BinOp`) are currently the **only defect type that no tool can
  detect**, because the two types are independent. Same-source generation eliminates them directly.
- **BP collisions change from “accidentally correct” to “compilation failure.”** The existing four
  cross-ladder collisions and two within-ladder collisions currently survive by chance through the
  unwritten `bp_right = bp_left + 1` convention; within-level uniqueness assertions ensure that such
  problems can never again exist silently.
- **Frontend testing changes from “one file running” to “everything running.”** Once the existing
  629 lines of tests are revived, phases 2–4 have a safety net; these tests **already exist** and
  were skipped only because one `mod` declaration was missing.
- **Phases 0 and 1 have zero risk and zero criterion cost.** Dead-code deletion and test revival can
  proceed immediately without blocking other work.

## Known Limitations and Risks

### Design Risks

- **The operator table is a new “thing that must be maintained.”** It turns “change 11 locations”
  into “change one location and maintain a table.” **If the table’s fields are poorly designed, it
  can become a new centralized maintenance burden.** This is the primary risk of the approach; the
  mitigation is to protect every column with compile-time assertions.
- **The risk of renumbering BP values is asymmetric.** Merging the two ladders involves renaming
  approximately 89 production references; if one is missed, **associativity changes silently**
  without an error, merely changing the parsed expression. This is the silent regression most likely
  to be introduced by this article and must be covered by the level-pair associativity cases in
  phase 1.
- **Phases 2–4 have a high C5 criterion cost.** Using all three layers means each phase must run the
  full corpus (293 `.yx` files), AST snapshots, and diagnostic comparisons. This makes each
  phase-2–4 commit relatively heavy.
- **The f-string span change may not be entirely avoidable.** See “Backward Compatibility”; there
  may be a need to accept one known behavior change and update the baseline.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** The individual
> decisions are recorded in
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **No
> pending items remain in this article.**

## Verification Record

All code facts in this article were verified by **directly reading the files**. Verification method:
the `read` tool read the relevant line ranges directly; line counts were obtained with
`Get-Content`; reference surfaces were scanned repository-wide with `grep` and classified as
production / test / orphan. **`cargo test` / `cargo build` were not used**—this is a pure
documentation article, and criterion execution belongs to the implementation phase.

### Item-by-Item Measured Results

| Verification item                                | Measured result                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Total lines in `literals.rs`                     | **1,568**                                                                                                                                                                                                                                                                                                        |
| Total lines in `tokenizer.rs`                    | **554**                                                                                                                                                                                                                                                                                                          |
| Line number of `next_token_inner`                | **`:218`**                                                                                                                                                                                                                                                                                                       |
| Line ranges of the four radix scanners           | `77-161` (85 lines) / `164-247` (84) / `250-333` (84) / `336-516` (181)—consistent with the tables in this article                                                                                                                                                                                               |
| Three escape-decoding implementations            | `694-827` (134 lines) / `1015-1141` (127 lines) / `push_fstring_escape:1374`—consistent with the tables in this article                                                                                                                                                                                          |
| `scan_leading_dot`                               | `519-635`, **117 lines**                                                                                                                                                                                                                                                                                         |
| Number of `Expr` variants                        | **22** (including `Error`), `ast.rs:16-163`                                                                                                                                                                                                                                                                      |
| Number of files in `lexer/tests/`                | **14** (including `mod.rs`)                                                                                                                                                                                                                                                                                      |
| Number of lines in real orphan tests             | **629** (222+167+89+81+70); another 90 lines in `fstring.rs` already run through the `#[path]` wiring and are not counted as orphaned                                                                                                                                                                            |
| Changes required to revive `lexer/tests/`        | **Two locations**: add `mod tests;` to `lexer/mod.rs` **and** add `mod lexer_mod;` `mod symbols;` to `tests/mod.rs`. Adding only one leaves the latter 159 lines unexecuted                                                                                                                                      |
| Dead code in `precedence.rs`                     | **96 lines** (`Precedence` enum `35-94` + `PrecedenceContext` `98-133`) with zero production uses—**confirmed**; however, the only external referrer, `precedence_inline.rs` (95 lines / six tests), is itself an orphan—**deleting the dead code requires deleting that file as well**                          |
| Number of lines in the seven empty shells        | 1–3 lines each (`debug_lexer.rs` is 1 line; the others are 3 lines)                                                                                                                                                                                                                                              |
| Shape of the blank-line residue                  | **Three consecutive blank lines inserted between each field, spanning 11 lines** (`777-787` / `1098-1108`)                                                                                                                                                                                                       |
| Number of lines in `pratt/tests/precedence.rs`   | **13**, with the sole assertion `assert!(bp_lowest < bp_highest)`                                                                                                                                                                                                                                                |
| Fourth “skip balanced parentheses” location      | `224-233`                                                                                                                                                                                                                                                                                                        |
| Downstream change surface                        | **17 production files** match `BinOp::` (**534 matches across 47 files** repository-wide)                                                                                                                                                                                                                        |
| Is `src/backends/` a `BinOp` consumer?           | **No**—`src/backends/` has **zero matches** for `BinOp` or `ast::`; the interpreter consumes IR/bytecode and never sees the AST                                                                                                                                                                                  |
| “No check can detect an omission”                | **Must be stratified**: an omitted exhaustive `match` arm **is detected by the compiler**; the silent cases are `matches!` macros, `_` fallbacks, and **cross-enum** omissions. This article now states this distinction based on measurement                                                                    |
| `matches!` blocks in `const_data.rs`             | `263-288` (the four functions `is_arith` / `is_comparison` / `is_logical` / `is_bitwise`)                                                                                                                                                                                                                        |
| `impl Display for BinOp`                         | Starts at `291`; 18-arm exhaustive match with no `_` fallback                                                                                                                                                                                                                                                    |
| The two functions in `parse_assign_after_target` | **Not dead code**: both are still called at `declarations.rs:719` / `:725` as a running gate that rejects removed syntax; moreover, `skip_old_function_syntax` (`120-122`, whose body contains only a one-line comment) **does not consume any tokens when called** (`:725-726` directly executes `return None`) |
| Second `BinOp` enum                              | `const_data.rs:234` does exist and has **no `From` conversion whatsoever** with `ast::BinOp`, forcing files such as `const_eval.rs:19` / `ir_gen.rs:2317` to create import aliases                                                                                                                               |
| Re-export block in `lexer/tests/mod.rs`          | Present (in the 38-line file), with `pub use ... ::*;` at `:28-38`; this is the direct reason the empty shells could masquerade as coverage                                                                                                                                                                      |

### Three Corrections from This Round of Verification

The following three points conflict with the intuition that “the first BP ladder is dead”; the
measured results take precedence:

1. **The two ladders are interwoven, not “one live and one dead.”** `infix_info` (`led.rs:30-88`)
   itself references five constants from the first ladder (`BP_ASSIGN:33`, `BP_CALL:75/77/79/83`,
   `BP_CAST:81`) and eight from the second. Only six of the first ladder’s 12 constants truly have
   zero references (`BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR`,
   plus `BP_COMPARISON`, which survives only in the comment at `declarations.rs:742`); the other six
   (`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`) have
   approximately **89 references across eight files** in production code. Therefore, “delete the
   first BP constants” is not a 12-line C6 deletion, but a cross-file rename in the C5 category.
2. **The second ladder has a dead member of its own.** `BP_RANGE` (`precedence.rs:22`) has zero
   production uses—`..` uses the bare literal `(6, 7)` at `led.rs:40`, and `BP_RANGE` appears only
   in the comment at `led.rs:157-159`.
3. **The “self-contradictory comment” is at `precedence.rs:33`, not `:11`.** `*:11` is
   `pub const BP_COMPARISON: u8 = 5;`. The comment (_"Precedence rules for the Pratt parser"_)
   describes the zero-reference `Precedence` enum.

Additional correction: `bp_right = bp_left + 1` produces **left associativity** (`a - b - c` →
`(a-b)-c`), not right associativity; the `(11, 1)` at `led.rs:85` is the only explicit formulation
in which the right operand greedily consumes everything. There are also more collisions than the two
cross-ladder collisions: `BP_UNARY=8 == BP_CAST=8` (an internal collision in the first ladder, with
both constants live) and `BP_RANGE=7 == BP_ADD=7` (an internal collision in the second ladder).

### Unverified Items

The main text marks each of the following as “unverified” where it appears:

- The exact number of arms and the binding power of each arm in the `prefix_info` table body at
  `pratt/nud.rs` (`26-75`)
- The exact line boundaries of the eight responsibility sections in `declarations.rs:136-642`
- The complete variant set of `const_data::BinOp` (`const_data.rs:234`)
- Whether `logos` / `regex` already appears in `Cargo.toml` as a transitive dependency
- All call sites of `synchronize()` beyond `parser_state.rs:170-186`
- Whether this project is a published library, which affects the risk assessment for removing public
  API surface

## See Also

### Companion Design Documents in This Batch

- [01-routing.md](01-routing.md) — Where to change when adding a feature (feature-routing tables
  A/B/C, dependency-direction rules)
- [03-type-unification.md](03-type-unification.md) — Ownership of `ast::Type` convergence and the
  interface boundary with this article
- [04-ssa.md](04-ssa.md) — Another user of the C4/C5 criteria
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Complete inventory of the eight orphan test
  trees
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — Definitions of the C5/C6 categories, the
  three criterion layers, and the regression gate

### Main RFCs and Related Proposals

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Governing framework; four-layer model, glossary, feature-routing tables A/B/C
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Build-time gate for error codes, the enforcement model borrowed by this article
- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Origin of
  `ast::Type`

### Code Location Index

- `src/frontend/core/lexer/literals.rs:77-161` / `164-247` / `250-333` / `336-516` — Four radix
  scanners
- `src/frontend/core/lexer/literals.rs:694-827` / `1015-1141` / `1374` — Three escape-decoding
  implementations
- `src/frontend/core/lexer/literals.rs:519-635` — `scan_leading_dot` (zero coverage)
- `src/frontend/core/lexer/mod.rs:104-106` — The lexer layer’s only test declaration
- `src/frontend/core/lexer/tests/mod.rs:15-25` / `28-38` — Submodule declarations and the `pub use`
  re-export block
- `src/frontend/core/parser/pratt/nud.rs:441-446` — Runtime nested `tokenize()` for f-string
  interpolation
- `src/frontend/core/parser/pratt/nud.rs:896-901` / `1041` — The source and use of the magic number
  `12`
- `src/frontend/core/parser/pratt/precedence.rs:6-17` and `22-31` — The two BP ladders
- `src/frontend/core/parser/pratt/precedence.rs:33` / `35-94` / `98-133` — The disconnected comment,
  zero-reference enum, and struct
- `src/frontend/core/parser/pratt/led.rs:30-88` — Handwritten `infix_info` table
- `src/frontend/core/parser/pratt/tests/mod.rs:4-6` — Orphan wiring missing the `precedence_inline`
  declaration
- `src/frontend/core/parser/statements/declarations.rs:719-727` — Old-syntax rejection gate and its
  “skip without consuming” defect
- `src/frontend/core/types/const_data.rs:234` — The second operator enum parallel to `ast::BinOp`
- `build.rs:19-55` — Build-time error-code gate, an existing enforcement model in this repository
- `src/middle/core/tests/bytecode.rs:369-657` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “round-trip every item” criterion model in this repository
