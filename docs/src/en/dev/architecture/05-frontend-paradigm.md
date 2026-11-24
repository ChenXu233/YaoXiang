# Frontend Paradigms: Lexing and Parsing

> **Companion design document**. This article is a companion to
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> See the main body of RFC-039 for the four-layer model, acceptance-criteria tiers, and
> execution-phase ordering; see the [index for this directory](index.md) for the role of each
> companion document.

## Positioning and Scope

This article covers the L2 frontend—**lexing (`src/frontend/core/lexer/`) and parsing
(`src/frontend/core/parser/`)**.

**Problem to solve**: reduce the change surface for “adding a binary operator” from the current
**6–7 locations within L2 + 17 downstream production files**—of which the compiler forces
approximately 8–9 locations to change and the rest rely on manual synchronization—to **1–2
declarations + 7–8 automatically generated locations**.

This article implements the pain point marked “binary operator” in the first row of RFC-039
Functional Routing Table A. The restructuring category is **C5 frontend paradigm change** as defined
in [07-equivalence-oracle.md](07-equivalence-oracle.md) (criteria: identical AST snapshots +
identical diagnostics + identical behavior, all three required), together with **C6 pure deletion**
(no equivalence criteria are needed; only confirmation that no references remain). The C6 items are
prerequisites for C5: deleting 96 lines of dead code, eliminating three bare magic numbers, and
restoring 629 lines of lexer tests that have never run.

**Scope boundary**: handling `ast::Type` (26 variants) and converging the three parallel type
representations belongs to [03-type-unification.md](03-type-unification.md). This article does not
repeat that work; it only declares the interface boundary in the “Detailed Design” section.

### Why This Is Worth Doing

“Adding an operator” is the most expensive frontend change. This tax has **three malignant
characteristics**:

**First, the cost does not decrease with experience.** Someone who has added ten operators will not
need to change one fewer location than someone who has added one, because there is no derivable
relationship between these locations. The `(6,7)` in `led.rs:40` and the `12` in `nud.rs:1041` have
no connection to each other, yet they must agree or expression parsing will fail.

**Second, missed updates fall into two categories, and the compiler cannot catch the worst
category.** See the tiered table in “The Change Surface for Adding an Operator”: an omitted arm in
an exhaustive `match` will be caught by the compiler, whereas **a missed update across enums can
never be caught by the compiler**.

**Third, this tax is invisible in the ledger.** Almost every commit shown by
`git log --oneline -- src/frontend/core/parser/pratt/led.rs` touches 3–4 files at once, but no tool
can identify which location that commit failed to update.

### The Correct Paradigm Already Present in This Repository

This project has two **correct solutions to the same class of problem**, both of which can be reused
directly:

- **Error codes**: `build.rs:19-55` uses `tools/code-tables` to parse the registry at **build
  time**, validate uniqueness, and compare it entry by entry with RFC-013’s Markdown code table.
  **Any inconsistency immediately causes `panic!` and rejects the build**. The cost of adding an
  error code is therefore a constant one location plus documentation.
- **Standard-library interfaces**: byte-for-byte comparison in `gen_interfaces.rs` and marked-range
  drift detection in `gen_docs.rs` (the last row of RFC-039 Routing Table B).

What these two mechanisms have in common is that they **turn “changes that must be kept
synchronized” from something remembered by people into executable assertions**. The entire goal of
this article can be reduced to one sentence: turn the correspondences among the operator table,
lexical rule table, and AST variant table into executable assertions.

### Why “Only Deleting Dead Code” Is Not Enough

Dead code does indeed exist (`precedence.rs` at 96 lines, the empty `skip_old_function_syntax`
function, and 629 lines of orphaned tests), but deleting it will not reduce the number of locations
involved in “adding an operator” by even one. Dead code is **legacy debt**, while the cross-file
change surface is **structural cost**. The two must be handled separately; see “Key Decisions and
Rationale” for their treatment.

## Current State

Everything below is a **verified fact**, with a file path and line number attached to each item. The
verification method and item-by-item measured results appear in “Verification Record”.

### Lexer Layer: Copy-and-Paste Evidence in `literals.rs`

`src/frontend/core/lexer/literals.rs` contains **1,568 lines in total**. This size does not come
from complexity, but from three forms of structural duplication.

**Duplication one: the four radix scanners are almost identical line by line.**

| Scanner               | Line range            | Number of lines |
| --------------------- | --------------------- | --------------- |
| `scan_hex_number`     | `literals.rs:77-161`  | 85              |
| `scan_octal_number`   | `literals.rs:164-247` | 84              |
| `scan_binary_number`  | `literals.rs:250-333` | 84              |
| `scan_decimal_number` | `literals.rs:336-516` | 181             |

The first three are each about 84 lines and structurally identical: accumulate digit → `has_digits`
check → `overflow` flag → `checked_mul(base)` → `checked_add` → `try_into` → report an error. **The
only differences are the base constant and the digit-extraction expression.** Across the three
copies of 84 lines, **approximately 250 lines are pure redundancy** (`scan_decimal_number` requires
additional handling for decimal points and exponents, so it cannot be merged and is listed
separately).

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

**Duplication two: escape decoding is implemented three times.**

| Implementation              | Location                       | Length                                     |
| --------------------------- | ------------------------------ | ------------------------------------------ |
| `scan_string`’s `\\` branch | `literals.rs:694-827`          | 134 lines                                  |
| `scan_char`’s `\\` branch   | `literals.rs:1015-1141`        | 127 lines                                  |
| `push_fstring_escape`       | Starting at `literals.rs:1374` | Already extracted into a separate function |

The first two are near-verbatim copies of 127–134 lines (`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` → same
push, same `\x` / `\u{...}` logic, same `c =>` error fallback), meaning **approximately 250 lines
were written twice**.

It is worth noting separately that **the f-string version has already been extracted into a
standalone function** (`push_fstring_escape:1374`). This shows that **someone already recognized the
duplication and fixed it locally, but never went back to converge the other two locations**. This is
more notable than “none of the three recognized it”: it proves that the problem is solvable, but
lacks an enforcement mechanism.

**Duplication three: two implementations of multiline string scanning.**
`scan_multi_line_string:869` and `scan_fstring_multi_line:1497` are two more implementations of the
same kind.

**Residual abnormal formatting.** The two locations `literals.rs:777-787` and `1098-1108`, each 11
lines long, have **three consecutive blank lines inserted between every field of a struct literal**.
This is residue from automated rewriting. Other locations in the same function (`1111-1115`) have
only one blank line between fields, indicating that formatting was interrupted.

> This residue is harmless in itself, but it is a **diagnostic signal**: the same code was rewritten
> twice by different tools, meaning that someone historically made a local patch here without
> pulling in the surrounding context.

**Conclusion**: approximately 500 lines in `literals.rs`—250 lines of redundant radix scanning plus
250 lines of duplicated escape handling—are **mechanical duplication**, not complexity.

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

**The parser layer runs a complete `tokenize()` separately for every f-string interpolation at
runtime and creates a new `ParserState`.**

As a result, an f-string containing N interpolations **triggers N+1 complete lexical analyses**—N
for the interpolations and one for the outer string. At the same time, each interpolation is an
**independent parsing context**, so:

- diagnostic spans inside an interpolation are relative to the interpolation text, and remapping
  them to source locations requires additional work;
- an interpolation cannot access any outer parser state (there is no current requirement for this,
  but the structure forecloses future possibilities, such as referencing an outer implicit variable
  from an interpolation);
- the locations at which lexical and syntax errors are produced are split across two separate
  stages.

This is a **defect in both performance and structure** and must be addressed (see “Target Design ·
Lexer”).

### Parser Layer: Two BP Ladders, Bare Magic Numbers, and a Handwritten Dispatch Table

#### The Two Ladders Are Not “One Live and One Dead”; They Are Intertwined

`src/frontend/core/parser/pratt/precedence.rs` contains **two binding-power ladders**, whose numeric
ranges overlap extensively.

**The first ladder (`precedence.rs:6-17`)**: `BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` /
`BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` /
`BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`.

**The second ladder (`precedence.rs:22-31`)**: `BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` /
`BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`.

**Measured conclusion: both ladders survive in production code and are already intertwined inside
`infix_info`.**

- `infix_info` (`led.rs:30-88`) references **five constants from the first ladder**: `BP_ASSIGN`
  (`:33`), `BP_CALL` (`:75` / `:77` / `:79` / `:83`), and `BP_CAST` (`:81`);
- it references **eight constants from the second ladder**: `BP_OR` (`:42`), `BP_AND` (`:44`),
  `BP_EQ` (`:47`), `BP_SHIFT` (`:51` / `:54`), `BP_BIT` (`:58`), `BP_CMP` (`:62` / `:65`), `BP_ADD`
  (`:68`), and `BP_MUL` (`:72`);
- **the only dead member of the second ladder is `BP_RANGE`**—`..` uses the bare literal `(6, 7)` in
  `led.rs:40` rather than `BP_RANGE`; `BP_RANGE` appears in production code only in the comment at
  `led.rs:157-159`.

The live status of the first ladder’s 12 constants, based on a repository-wide `grep`, is:

| Status              | Constant                                                                     | Production reference sites                                                                                                                                                                                  |
| ------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Dead (6)**        | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | Zero production hits. They appear only in `precedence.rs` itself and the orphaned test `precedence_inline.rs`                                                                                               |
| Dead (comment only) | `BP_COMPARISON`                                                              | Zero production hits; mentioned only in the comment at `declarations.rs:742`                                                                                                                                |
| **Live (6)**        | `BP_LOWEST`                                                                  | `parser/mod.rs:80`, 9 locations in `statements/control_flow.rs`, 6 locations in `statements/declarations.rs`, `pratt/led.rs:229`/`:321`/`:327`/`:415`, `pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523`, etc. |
|                     | `BP_ASSIGN`                                                                  | `led.rs:33`, `led.rs:99`, `statements/declarations.rs:18` (import), `:743` (`BP_ASSIGN + 1`)                                                                                                                |
|                     | `BP_UNARY`                                                                   | `nud.rs:32`, `:35`, `:83`, `:101`, `:223`, `:243`                                                                                                                                                           |
|                     | `BP_CALL`                                                                    | `led.rs:75`, `:77`, `:79`, `:83`                                                                                                                                                                            |
|                     | `BP_CAST`                                                                    | `led.rs:81`                                                                                                                                                                                                 |
|                     | `BP_HIGHEST`                                                                 | Approximately 20 locations in `nud.rs:38-69`—one for each literal and keyword prefix                                                                                                                        |

**Total: the six live constants have approximately 89 references in production code across 8
files.**

> This corrects an easy but incorrect conclusion: “deleting the first ladder” is **not** a 12-line
> deletion, but a rename and merge involving 89 references across 8 production files. Only six
> constants in the first ladder truly have zero references, plus `BP_COMPARISON`, which survives
> only in a comment. The second ladder is also incomplete—`BP_RANGE` is dead.

**The truly unreferenced dead code is the two types**: the `Precedence` enum (`precedence.rs:35-94`,
60 lines) and the `PrecedenceContext` struct (`precedence.rs:98-133`, 36 lines), totaling **96
lines**. Verification found zero production uses of these two types—every repository-wide grep hit
for `PrecedenceContext` and `Precedence::`, apart from `precedence.rs` itself, is in
`src/frontend/core/parser/pratt/tests/precedence_inline.rs`.

**There is one detail that must be recorded in this document**: the only file referencing the dead
code is **itself an orphan**. `pratt/tests/mod.rs:4-6` declares only
`mod led; mod nud; mod precedence;` and **does not declare `precedence_inline`**.

> In other words, the 96 lines of dead code only “appear to have test coverage” because they are
> tested by a 95-line, 6-test file that has never participated in compilation (see “Dead Code and
> Current Test State”). **Deleting the dead code must also address this orphaned test file**, or the
> code will no longer compile afterward.

In addition, the comment at `precedence.rs:33` contradicts itself as corroborating evidence:
_“Precedence rules for the Pratt parser”_—this comment describes the zero-reference `Precedence`
enum, whose ladder is not the same as the ladder actually in effect. The enum and the implementation
have already diverged.

#### Numeric Collisions

The numeric ranges of the overlapping portions of the two ladders correspond almost digit by digit,
so **cross-ladder collisions occur in pairs**:

| Conflict                 | Value    | Use site A (first ladder)      | Use site B (second ladder)     |
| ------------------------ | -------- | ------------------------------ | ------------------------------ |
| Assignment vs logical OR | `1 == 1` | `BP_ASSIGN` (`led.rs:33`)      | `BP_OR` (`led.rs:42`)          |
| Cast vs multiplication   | `8 == 8` | `BP_CAST` (`led.rs:81`)        | `BP_MUL` (`led.rs:72`)         |
| Shift vs term            | `6 == 6` | `BP_TERM` (dead)               | `BP_SHIFT` (`led.rs:51`/`:54`) |
| Bitwise vs comparison    | `5 == 5` | `BP_COMPARISON` (comment only) | `BP_BIT` (`led.rs:58`)         |

**The first ladder also collides internally**: `BP_UNARY=8` and `BP_CAST=8` (`precedence.rs:14` and
`:15`) give **two different meanings to the same value within one ladder**, and both are live—one
for prefix binding in `nud.rs`, the other for `cast` at `led.rs:81`. The second ladder also collides
internally: `BP_RANGE=7` and `BP_ADD=7`.

**The reason this has not yet caused failures is an unwritten convention**: almost all infix
operators use `bp_right = bp_left + 1`, so the right operand accepts only operators with **strictly
higher** binding power—which actually produces **left associativity** (`a - b - c` → `(a-b)-c`).
Consequently, the only effective comparison, “compare `bp_left`,” does not fail merely because two
operators have equal BP values. `FatArrow` at `led.rs:85` uses `(11, 1)`, where `bp_right` is lower
than `bp_left`; it is the only explicit “right operand greedily consumes” formulation (maximizing
the lambda body to the right).

This is **accidental correctness**: it depends on the facts that “the `bp_left` returned by
`infix_info` is unique” and that “`parse_expression_internal:100` compares only `bp_left`.” Changing
any location to use an actual `bp_left == bp_right` test to determine associativity would
immediately expose the conflicts.

#### Three Bare Magic Numbers

| Magic number | Location                             | Problem                                                                                                                           |
| ------------ | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `(6, 7)`     | `led.rs:40` `TokenKind::DotDot`      | Literal binding-power pair with no name; also **does not use `BP_RANGE` from the same ladder**, making `BP_RANGE` a dead constant |
| `(11, 1)`    | `led.rs:85` `TokenKind::FatArrow`    | 11 **exceeds the upper bound of either ladder** (the first ladder’s `BP_HIGHEST=10`)                                              |
| `12`         | `nud.rs:1041` `parse_expression(12)` | **Greater than `BP_HIGHEST=10`**, used for RFC-010b pattern probing                                                               |

The comment at `nud.rs:896-901` candidly explains the origin of this magic number: _“Lambda binding
power is 11, so we use 12 to stop before =>”_ and _“parse_expression(12) 会把 `ok` 停成裸 Var、`(`
成意外 token”_. In other words, **this 12 was found empirically rather than derived**.

#### Dispatch Is a Handwritten `match`

**Dispatch mechanism**: everything is a handwritten `match` that returns a function pointer, with
**no macros, tables, or generators**.

- `parse_prefix` (`nud.rs:14-19`) obtains the `prefix_info()` closure;
- `parse_expression_internal` (`pratt/mod.rs:79-113`) loops over the
  `(bp_left, bp_right, parser_fn)` tuple returned by `infix_info()`;
- there are two handwritten tables: `infix_info` (`led.rs:30-88`) and `prefix_info`
  (`nud.rs:26-75`).

#### `parse_assign_after_target`: A 507-Line General Dispatcher with a Misleading Name

`parse_assign_after_target` at `src/frontend/core/parser/statements/declarations.rs:136` has a
function body ending at `:642` (**507 lines**, in a file of 1,015 lines).

Despite the name “assign after target,” it is **the general dispatcher for declaration forms** and
contains eight independent responsibility sections: legacy-syntax probing / three-level nested
lookahead for annotation disambiguation / declaration legality / semantic side effects / type-body
parsing fallback / RFC-010 lambda signature parameter reconstruction / MetaType definitions /
ordinary initialization.

Its growth has three overlapping causes.

**Cause one: approximately 120 lines of dead language features.**

`is_old_function_syntax` (`declarations.rs:82-117`), `skip_old_function_syntax`
(`declarations.rs:120-122`), and the supporting lookahead at `:145-167`.

An exact qualification is needed: **these two functions are not unreferenced**. They are called from
`declarations.rs:719` and `:725` by `parse_identifier_stmt`, following this path: detect
`identifier(` followed by type parameters and `->` → report “legacy syntax has been deprecated” →
skip → return `None`.

The accurate description is therefore: **this is a still-running gate that “rejects removed
syntax,”** with approximately 120 lines of overhead spent performing a parenthesis-balancing scan
for every statement beginning with `identifier(`.

Among these:

- the body of `skip_old_function_syntax` (`:120-122`) consists of nothing but the comment
  `// 旧语法已移除，此函数不再需要`; it is a **pure empty function**;
- more notably, after calling it (`:725-726`), the code immediately returns `None` **without
  consuming any token**. In other words, this “skip” **actually skips nothing**—the subsequent
  tokens remain in the stream and can only be handled by the caller’s error recovery. This is an
  interface whose name does not match its behavior.

**Cause two: lookahead logic has not been abstracted.** “Skipping balanced parentheses” is
**handwritten four times** in the same file:

| Location                           | Line range                |
| ---------------------------------- | ------------------------- |
| Inside `is_old_function_syntax`    | `declarations.rs:96-105`  |
| Inside `parse_assign_after_target` | `declarations.rs:147-156` |
| Same                               | `declarations.rs:191-200` |
| Same                               | `declarations.rs:224-233` |

Each is approximately 12 lines with the same structure: `paren_depth = 1` →
`while paren_depth > 0 && !at_end()` → increment on `LParen`, decrement on `RParen` → `bump()`. **No
code is shared among the four copies.**

**Cause three: type-layer logic has leaked into the parser.** See RFC-039 Routing Table C:
`declarations.rs:27-80` calls the type layer’s `is_type_param_annotation` / `name_used_as_type`;
`declarations.rs:498-509` hard-codes the `"Terminates"` string and calls
`typecheck::operator_interfaces::spec()`. **This belongs to 03-type-unification.md /
02-stage-contract.md.** This article does not duplicate the treatment and only marks the boundary in
the “Detailed Design” section.

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

`Expr::Error` has been verified to exist (span extraction handles it at `pratt/mod.rs:72`, as does
`ast.rs:1048`); `StmtKind::Error` is constructed at `parser/mod.rs:50`. **These two error
placeholders are the foundation of the backward-compatible design and must be retained** (see
“Detailed Design · Backward Compatibility”).

### The Change Surface for Adding an Operator

**The downstream change surface is much larger than expected.** `BinOp::` has **534 hits across 47
files** in the repository, including **17 production files**. More importantly, these files consume
**two unrelated `BinOp` enums**:

| Enum                | Definition                | Number of variants                  | Production consumers (parentheses contain the number of `BinOp::` hits in that file)                                                                                                                                                     |
| ------------------- | ------------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ast::BinOp`        | `parser/ast.rs:191-213`   | 20                                  | `led.rs`(22) `nud.rs`(1) `checker.rs`(8) `inference/expressions.rs`(25) `inference/statements.rs`(5) `operator_interfaces.rs`(5) `ir_gen.rs`(23) `formatter/handlers/expr.rs`(25) `lsp/handlers/inlay_hint.rs`(4) `spawn/analysis.rs`(3) |
| `const_data::BinOp` | `types/const_data.rs:234` | Verified to exist (`Ne`, not `Neq`) | `const_data.rs`(36) `const_eval.rs`(64) `evaluator.rs`(13) `ownership.rs`(13) `termination.rs`(16) `proof/smt/translate.rs`(13) `proof/dep_graph.rs`(2)                                                                                  |

It has been verified that `src/backends/` has **zero hits** for either `BinOp` or `ast::`—the
interpreter consumes IR / bytecode and never sees the AST. All 17 production files therefore lie on
the L2 → L3 path.

**There is no `From`/`TryFrom` between the two enums**—a repository-wide grep for
`impl From<...BinOp`, `-> BinOp`, and `BinOp as` found no conversion implementation. The cost is
that **every file that touches both types must alias one at the import site**: `const_eval.rs:19`
(`BinOp as AstBinOp`), `ir_gen.rs:2317` (`use ast::BinOp as B`), and `CEBinOp` / `ConstBinOp` in
test files.

> **This is the most important finding in this document.** When `ast::BinOp` gains a variant, **the
> compiler cannot remind you to update `const_data::BinOp`**, because they are two independent types
> that can each compile successfully. If the update is missed, the constant-folding path does not
> understand the new operator, while the type-checking and IR-generation paths do—**the same
> operator behaves differently in two subsystems, with no diagnostic whatsoever**.
>
> This is an order of magnitude more serious than “missing one arm in a `match`.” The latter is
> caught by exhaustive matching; the former cannot be caught.

**Regarding “whether a missed update can go unnoticed,” the distinction must be made precisely**:

| Type of missed update                                       | Detected by the compiler? | Verified evidence                                                                                                                                                            |
| ----------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Missing arm in an exhaustive `match`                        | **Yes**                   | The `impl Display for BinOp` at `const_data.rs:291-317` is an 18-arm exhaustive match with no `_` fallback                                                                   |
| Missing item in an enumeration used by the `matches!` macro | **No**                    | The `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` functions at `const_data.rs:263-288` all use partial `matches!` lists; a new variant silently returns `false` |
| Missing arm in a `match` with a `_` fallback                | **No**                    | The `_ => {}` at `ownership.rs:687` silently ignores the new variant                                                                                                         |
| Missing update across `ast::BinOp` / `const_data::BinOp`    | **Never**                 | The two have no conversion and no common parent type, so the compiler has no way to relate them                                                                              |

The accurate statement is therefore not “11 locations can be missed with nobody noticing,” but:
**the compiler can cover the exhaustive-match portion; what fails silently are the `matches!`
macros, `_` fallbacks, and cross-enum updates.**

### Dead Code and Current Test State

This is **the item that most urgently needs to be addressed**.

#### Dead Code

- `precedence.rs:35-94`, the `Precedence` enum (60 lines), and `precedence.rs:98-133`, the
  `PrecedenceContext` struct (36 lines): zero production uses; their only external referencer is the
  orphan file `pratt/tests/precedence_inline.rs` (95 lines / 6 tests), which is not declared by
  `pratt/tests/mod.rs:4-6`.
- `skip_old_function_syntax` (`declarations.rs:120-122`): a pure empty function; the call site at
  `declarations.rs:725` is followed immediately by `return None`, without consuming a token.
- `BP_RANGE` (`precedence.rs:22`): zero production uses (it survives only in the comment at
  `led.rs:157-159`).

#### Only One Lexer Test File Is Running

**The only test declaration in `lexer/mod.rs:104-106` is:**

```rust
#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
```

A grep for `mod tests` throughout `src/frontend/core/lexer/` matches only this location. **`lexer/`
has never declared `mod tests;`.**

The entire `src/frontend/core/lexer/tests/` subtree therefore **has never participated in
compilation**. The directory actually contains **14 files** (including `mod.rs`):

| File                                                                              | Number of lines | Status                              |
| --------------------------------------------------------------------------------- | --------------- | ----------------------------------- |
| `mod.rs`                                                                          | 38              | Exists; declares 11 submodules      |
| `literals.rs`                                                                     | 222             | Real tests, **orphaned**            |
| `rfc010_lexer.rs`                                                                 | 167             | Real tests, **orphaned**            |
| `fstring.rs`                                                                      | 90              | Real tests, **wired via `#[path]`** |
| `lexer_mod.rs`                                                                    | 89              | Real tests, **doubly orphaned**     |
| `rfc004_lexer.rs`                                                                 | 81              | Real tests, **orphaned**            |
| `symbols.rs`                                                                      | 70              | Real tests, **doubly orphaned**     |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 1–3 lines each  | 7 empty shells                      |
| `debug_lexer.rs`                                                                  | 1               | Empty shell                         |

**Real but never-run tests: 629 lines across 5 files** (222+167+89+81+70). The only lexer test file
currently running is the f-string file.

**One wiring trap must be made explicit: `tests/mod.rs` exists, but it is not fully wired either.**

`lexer/tests/mod.rs:15-25` declares 11 submodules (`basic` / `literals` / `operators` / `delimiters`
/ `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer` / `fstring`),
and `:28-38` then `pub use`s all of them for “backward-compatible re-export.”

**However, it does not declare `lexer_mod` or `symbols`.** Therefore:

> **Adding only one `mod tests;` line is not enough.** Restoring `lexer/tests/` requires **two
> changes**: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` and `mod symbols;` to
> `tests/mod.rs`. Otherwise, these 159 lines still will not run.

**The trap of the seven empty shells**: statements such as `pub use basic::*;` at
`tests/mod.rs:28-38` **do not produce any compiler error or warning when referring to a module with
zero assertions**. After adding `mod tests;`, CI will report that all tests pass even though the
`basic` / `comments` / `delimiters` / `errors` / `keywords` / `operators` / `debug_lexer` modules
contain **not a single assertion**. **This creates the false impression that the lexer is covered**,
which is more dangerous than not wiring it at all.

**The highest-risk uncovered paths**, which will be exposed immediately once wired, are:

| Path                                                                                    | Location                                                                          | Risk                                |
| --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ----------------------------------- |
| Overflow paths of the four radix scanners                                               | `literals.rs:134-141` / `220-227` / `306-313` + the corresponding decimal section | Integer overflow is a silent error  |
| The “continue consuming without reporting an error” branch after `checked_mul` overflow | 4 isomorphic locations                                                            | May cause token-stream misalignment |
| The **entire** `scan_leading_dot` function                                              | `literals.rs:519-635` (117 lines)                                                 | Zero coverage                       |
| Invalid `\x` / `\u` escape paths                                                        | `literals.rs:730-749` / `1051-1070`                                               | Error paths are difficult to test   |

**Wiring on the parser side is correct**: `parser/mod.rs:8-9`, `pratt/mod.rs:8-9`, and
`statements/mod.rs:11-12` all have the correct `mod tests;`. `pratt/tests/mod.rs:4-6` declares `led`
(335 lines), `nud` (269 lines), and `precedence` (13 lines).

**However, parser-side coverage quality has the same problem**:

- the only assertion in `pratt/tests/precedence.rs` (**13 lines**) is
  `assert!(bp_lowest < bp_highest)`, which is **almost a no-op**—it cannot detect any of the four
  cross-ladder collisions listed above.
- `pratt/tests/precedence_inline.rs` (**95 lines / 6 tests**) is not declared by
  `pratt/tests/mod.rs` and **has never run**—and it tests **only the zero-reference `Precedence`
  enum + `PrecedenceContext`**.
- **The two ladders actually in use** (`BP_OR`…`BP_MUL` and
  `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`) **have no direct unit
  tests**.

## Target Design

### Lexer: Declarative Design

#### Evaluation of Three Options

| Option                                                      | Cost of adding a token         | Can it eliminate the lexer’s three duplications? | Can it solve f-string nested compilation?   | Introduced dependency                                                          |
| ----------------------------------------------------------- | ------------------------------ | ------------------------------------------------ | ------------------------------------------- | ------------------------------------------------------------------------------ |
| **A. Generate a DFA with `logos`**                          | 1 location (token declaration) | **Yes** (one copy each for escapes / numbers)    | No (interpolation must be handled manually) | Adds `logos`                                                                   |
| **B. Handwritten DFA with `regex`**                         | 1–2 locations                  | Partly                                           | No                                          | Adds `regex` (whether it is already a transitive dependency is **unverified**) |
| **C. Keep the handwritten lexer and enforce deduplication** | 1 location                     | **Yes** (manually extract shared functions)      | **Yes** (requires explicit design)          | None                                                                           |

**Position: adopt C as the baseline, A as a conditional alternative, and reject B.**

Reasons:

1. **C can eliminate all approximately 500 lines of lexer duplication with zero new dependencies**,
   entirely within a controllable scope. The three escape decoders—`scan_string` / `scan_char` /
   `push_fstring_escape`—are merged into a **single**
   `decode_escape(lexer, &mut String, EscapeMode)`, and the four radix scanners into a **single**
   `scan_radix_number(lexer, Radix, String)`. This is a purely mechanical refactoring, with C5’s
   “identical AST snapshots + identical diagnostics” as its criteria.
2. **The capabilities of A (`logos`) are real**—it can converge escape decoding and number scanning
   to one implementation each. **But it cannot solve f-string nested compilation**, which is the
   lexer layer’s most important defect. Introducing a new dependency would also change `Literals`’
   error type—the `InvalidEscape` / `InvalidNumber` variants of `LexError` would need to be
   remapped—which would **directly violate C5’s “identical diagnostics” criterion** unless an
   additional adaptation layer were written. That would introduce yet another manual synchronization
   point.
3. **B (a handwritten DFA using `regex`) is rejected**: it requires a hand-written state machine,
   with work comparable to A but far less controllability. If choosing between A and B, A must
   logically win; and if this article advocates C first, there is no reason to choose B.

> **Design judgment (not a verified fact)**: A may be a better long-term shape (a declarative token
> table is itself an enforcement mechanism for “change one location”). However, its migration cost
> and conflict with the “identical diagnostics” criterion make it unsuitable for this C5 change. If
> A is adopted later, it should be a separate proposal, using the test-restoration results in this
> article as a safety net.

#### f-String Nested Compilation Must Be Handled Explicitly

Regardless of whether A or C is chosen, nested compilation must be solved. **Position: eliminate
runtime nested `tokenize()` and produce the result in a single lexical pass.**

The concrete approach: when `scan_string` encounters the `f"` prefix, **within the same lexical
scan**, record the interpolation content as `FStringSegment::RawText(String)` and **do not**
recursively call `tokenize()`. The actual tokenization and parsing of the interpolations are
deferred until the parser first needs the f-string node, with the following properties:

- **performed in one batch**—for N interpolations, tokenize the interpolation segments once and
  cache the result as `Vec<Vec<Token>>`, reducing N+1 operations to one;
- **span mapping is determined during lexing**—the absolute offset of each interpolation in the
  source file is recorded, without relying on the relative semantics of `Point`.

This change is **independent of the paradigm choice** and should be performed even if A is not
adopted.

### Parser: Complete Grammar-Driven Parsing (LALRPOP)

> **Decision (2026-10-03, ChenXu233)**: **Adopt A (complete grammar-driven parsing with LALRPOP); do
> not implement a partial table-driven approach.** A’s equivalence risk—the difference between
> LALRPOP’s error-recovery model and `synchronize()`, and whether empirically discovered parsing
> behavior can be reproduced bit for bit—is eliminated by the **dual-parser differential** approach
> below; table-driven parsing is an insufficient intermediate state.

#### Option Evaluation

| Option                              | Change surface for adding a binary operator | Can it eliminate BP conflicts?                                                                                           | Can it preserve `synchronize()` and error placeholders?                  | Conclusion                           |
| ----------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ | ------------------------------------ |
| **A. LALRPOP**                      | 1 grammar location + 1 action location      | Yes (the LR table computes precedence automatically, and conflicts are exposed as grammar-conflict errors at build time) | Requires a rewrite, but **error productions can be declared explicitly** | **Adopt**                            |
| **B. tree-sitter**                  | 1 `grammar.js` location                     | Yes                                                                                                                      | Not applicable (produces a CST, not a domain AST)                        | **Reject**                           |
| C. Retain Pratt parsing with tables | 2–3 locations                               | Yes                                                                                                                      | Yes                                                                      | **Reject** (insufficient; see below) |

**Reason for rejecting B** (unchanged): tree-sitter produces a general-purpose syntax tree, while
this project’s `Expr` / `StmtKind` are **domain ASTs carrying semantic information** (`Expr::Cast`
carries the target type, `Expr::FString` carries a segment list, and `Pattern::Struct` carries field
names). A CST → domain AST conversion still requires a complete mapping layer; **tree-sitter cannot
eliminate it**. This would add a layer of roughly the same size as the existing 1,326-line `nud.rs`.

**Reason for rejecting C**: table-driven parsing solves **synchronization** (one location cannot be
missed) but not **maintainability**—the 22 variants of `Expr` would still require 22 handwritten
construction actions, merging the two BP ladders would still require changing approximately 89
references, and structural problems such as `Expr::Lambda { body: empty Block }` being borrowed as a
parameter list (`nud.rs:505-514`) would remain entirely unaddressed. **It is an unnecessary
intermediate stop between “entirely handwritten” and “complete grammar-driven.”**

#### The Original Objections to A and How They Are Eliminated

The original objection to A was **equivalence**, not insufficient capability:

- The “continue parsing where possible” semantics formed by `ParserState::synchronize()`
  (`parser_state.rs:170-186`) and the `Expr::Error` / `StmtKind::Error` placeholders use a different
  model from LALRPOP’s error recovery (token insertion / deletion), so **the resulting diagnostic
  sets must differ**.
- The exit condition `bp_left < min_bp` at `pratt/mod.rs:100-102` and the pattern probe
  `parse_expression(12)` at `nud.rs:1041` are **behaviors discovered on particular inputs**, and
  reproducing them bit for bit requires proof for each one.

**Elimination approach: dual-parser differential (double-parse diff).** This does not rely on an
argument that the criteria “should be equivalent”; it relies on **running the parsers**:

| Step | Content                                                                                                                                                                        | What it proves                                                           |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| 1    | Build the LALRPOP grammar + action code, **completely alongside the existing Pratt parser**, without changing any existing code                                                | —                                                                        |
| 2    | Run both parsers over 293 corpus files + all of `src/std/tests`, normalize the ASTs according to the rules in [07](07-equivalence-oracle.md), and **compare them bit for bit** | **AST equivalence for valid programs is empirically proven, not argued** |
| 3    | Switch traffic: change `parse()` in `parser/mod.rs` to call LALRPOP; retain Pratt as `parse_legacy()`                                                                          | —                                                                        |
| 4    | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + both BP ladders in `precedence.rs` + approximately 89 references                                                                | —                                                                        |
| 5    | Error recovery: reconstruct `synchronize()` and the `Expr::Error` / `StmtKind::Error` placeholders on the LALRPOP side using **explicit error productions**                    | See below                                                                |

**Step 2 is the core of the entire approach.** If any AST among the 293 corpus files differs, it is
discovered before the traffic switch—this is much stronger than manually arguing case by case
whether 89 references can be reproduced, and its cost is acceptable because it runs offline once.

#### Treatment of Error Recovery: Fully Preserve C5, Without Relaxation

**Error diagnostics must match entry by entry. This is a hard requirement, not negotiable.** The
approach is to **model the existing behavior explicitly in the grammar**:

| Existing behavior                                                                                                                          | LALRPOP equivalent                                                                                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `ParserState::synchronize()` (`parser_state.rs:170-186`): after statement parsing fails, skip tokens forward until a synchronization point | Explicitly declare an “error → skip to synchronization-point set → continue” production in the grammar; **the skipped set must match the existing implementation token for token**                           |
| `Expr::Error` / `StmtKind::Error` placeholders: allow the parser to continue constructing a “partial AST” after an error                   | Explicitly declare error-node productions in the grammar; **downstream match-arm positions remain unchanged** (if downstream code places `Error` in a wildcard branch, the error will be silently swallowed) |
| Diagnostic code + span                                                                                                                     | **Identical entry by entry. Wording remains unchanged.**                                                                                                                                                     |

**This is not “something that should be possible”; it must be achieved.** LALRPOP permits explicit
error productions; the synchronization-point set from `synchronize()` can be written into the
grammar. If stages 3a/3b empirically prove that the existing diagnostics cannot be reproduced under
LALRPOP, **the correct action is to report that finding honestly and reassess the approach—including
“retain Pratt”—rather than modify the criteria**.

Migration targets and treatment:

| Item                                                             | Content                                                                                                                                                                                                                                                     |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Grammar file                                                     | `src/frontend/core/parser/grammar/yaoxiang.lalrpop` (new directory; target structure in [01](01-routing.md))                                                                                                                                                |
| Action code                                                      | `grammar/actions.rs`—move construction of `Expr` / `StmtKind` from `nud.rs` / `led.rs`, with **one function per variant**                                                                                                                                   |
| Error productions                                                | Explicitly declared, preserving the “continue parsing where possible” semantics through the `Expr::Error` / `StmtKind::Error` placeholders                                                                                                                  |
| BP ladder merge                                                  | Both ladders are deleted with the grammar; the six live constants—`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`—and their **approximately 89 production references** are replaced by precedence declarations in the grammar |
| Bare magic numbers                                               | `(6,7)` / `(11,1)` / `12` disappear together with Pratt—the LR table computes precedence automatically                                                                                                                                                      |
| `is_old_function_syntax` (`declarations.rs:82-117`, 36 lines)    | **Naturally deleted**—after grammar-driven parsing, `f(Int) -> Int = ...` matches no production, so special-purpose probing for removed syntax is an anti-pattern                                                                                           |
| Borrowed `Expr::Lambda { body: empty Block }` (`nud.rs:505-514`) | **Naturally eliminated**—parameters are parameters in the grammar; there is no need to borrow an AST node                                                                                                                                                   |
| `synchronize()` (`parser_state.rs:170-186`)                      | Retain the API, but drive its call sites from error productions                                                                                                                                                                                             |

**Net effect**: delete `nud.rs`(1326) + `led.rs`(495) + both BP ladders in `precedence.rs` +
approximately 120 lines of dead legacy-syntax probing, and add a grammar file plus action code.
**The change surface for “adding a binary operator” drops from more than ten locations to one
grammar location + one action location**, and BP conflicts become **grammar build-time errors**
rather than “accidentally correct” behavior.

#### Prerequisites

1. **The 629 lines / 55 tests in `lexer/tests/` must first be restored** (P1 in
   [09](09-execution-wbs.md)). Without a frontend test safety net, replacing the entire parser
   cannot satisfy 07’s requirement that “criteria must precede refactoring.”
2. **Normalized AST snapshots must be committed first** (the second tier of P2); otherwise, step 2
   has nothing to compare.
3. **Eliminating f-string nested compilation** (the `literals.rs` refactoring) is **independent of
   this item** and should be performed even without adopting grammar-driven parsing.

### Treatment of `parse_assign_after_target`

**Position: split it into eight named functions whose bodies only dispatch.**

A 507-line function with eight responsibility sections forces every local change to be located
within 500 lines of code. The target shape after splitting is:

| Function to extract            | Responsibility                                           | Approximate line count | Ownership                                                                                |
| ------------------------------ | -------------------------------------------------------- | ---------------------- | ---------------------------------------------------------------------------------------- |
| `probe_old_function_syntax`    | Legacy-syntax probing                                    | 36                     | **Deleted by this article**                                                              |
| `resolve_annotation_ambiguity` | Annotation disambiguation (three-level nested lookahead) | To be verified         | This article (substantially shortened after extracting an independent lookahead utility) |
| `check_declaration_legality`   | Declaration legality                                     | To be verified         | This article                                                                             |
| `apply_semantic_side_effects`  | Semantic side effects                                    | To be verified         | **03-type-unification.md** (including the hard-coded `"Terminates"` at `498-509`)        |
| `parse_type_body`              | Type-body parsing fallback                               | To be verified         | This article                                                                             |
| `rebuild_lambda_params`        | RFC-010 lambda signature parameter reconstruction        | To be verified         | This article                                                                             |
| `parse_meta_type_def`          | MetaType definitions                                     | To be verified         | This article                                                                             |
| `parse_plain_init`             | Ordinary initialization                                  | To be verified         | This article                                                                             |

Supporting change: converge the four copies of “skip balanced parentheses” (`96-105` / `147-156` /
`191-200` / `224-233`) into a single `ParserState` method, `skip_balanced_parens()`, and change all
four call sites to use it.

**Net effect (design judgment)**: 507 lines → eight single-responsibility functions + one lookahead
utility; lookahead code drops from 48 lines to approximately 12; after type-layer logic is moved out
(approximately 80 lines), the portion actually owned by this article is approximately 300 lines.

### Dead Code Cleanup (C6, No Equivalence Criteria Required)

According to the C6 definition in 07-equivalence-oracle.md (“pure deletion, no equivalence criteria
required, only confirmation that no references remain”):

| Cleanup item                                     | Location                                   | Number of lines | Prerequisite                                                                                                        |
| ------------------------------------------------ | ------------------------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------------- |
| `Precedence` enum                                | `precedence.rs:35-94`                      | 60              | **Must also address** `pratt/tests/precedence_inline.rs` (the sole referencer, itself an orphan)                    |
| `PrecedenceContext` struct                       | `precedence.rs:98-133`                     | 36              | Same                                                                                                                |
| Six zero-reference constants in the first ladder | `precedence.rs:8-13`, `:11` (comment only) | 6               | Zero production references; delete directly                                                                         |
| `BP_RANGE`                                       | `precedence.rs:22`                         | 1               | Zero production references (survives only in the comment at `led.rs:157-159`); first make `..` use a named constant |
| `skip_old_function_syntax`                       | `declarations.rs:120-122`                  | 3               | Empty function; also delete its call site at `declarations.rs:725`                                                  |
| `is_old_function_syntax`                         | `declarations.rs:82-117`                   | 36              | See below                                                                                                           |
| `declarations.rs:145-167`                        | Same file                                  | 23              | Same                                                                                                                |
| Bare magic number `(6,7)`                        | `led.rs:40`                                | —               | Promote to named constants `BP_RANGE_L` / `BP_RANGE_R`                                                              |
| Bare magic number `(11,1)`                       | `led.rs:85`                                | —               | Promote to `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                            |
| Bare magic number `12`                           | `nud.rs:1041`                              | —               | Promote to `BP_PATTERN_PROBE`, and **record in a comment that it was found empirically**                            |

> **Note**: The first ladder’s `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` /
> `BP_HIGHEST` **are not within the pure-deletion scope of this table**—they have approximately 89
> production references, and merging them belongs to “Parser: Grammar-Driven · Option C” above (the
> C5 category), not to C6 deletion.

**Treatment options for `is_old_function_syntax` (design judgment)**:

It is not unreferenced dead code; it is **a still-running “reject removed syntax” gate**
(`declarations.rs:719`). There are two options:

- **Option 1 (aggressive)**: delete the entire section. Removed syntax should be “rejected” by the
  **grammar definition itself**—after grammar-driven parsing, `f(Int) -> Int = ...` matches no
  production and naturally reports an error, without dedicated probing. **This is one of the
  benefits of the paradigm change**: approximately 120 lines naturally disappear after
  grammar-driven parsing.
- **Option 2 (conservative)**: retain the gate but delete the empty `skip_old_function_syntax`
  function, and change “return `None` without consuming tokens” into an actual skip.

**Adopt Option 1**, because it is consistent with the position of retaining Pratt table-driven
parsing: if the dispatch table is the single source of truth, manually “probing a particular removed
syntax” is an anti-pattern. **The prerequisite is that the C5 criteria prove the diagnostic set is
unchanged**—empirically verify whether deleting it leaves the diagnostic code and span for
`f(Int) -> Int = x => x` identical to those before deletion. If they differ, fall back to Option 2
and record it as an open question.

### Test Reconstruction

**This is the first priority and must precede any paradigm change.**

**Stage 1: Restore `lexer/tests/`.** Two changes are required, not one:

1. Add `#[cfg(test)] mod tests;` after `:106` in `src/frontend/core/lexer/mod.rs`.
2. Add `mod lexer_mod;` and `mod symbols;` to `src/frontend/core/lexer/tests/mod.rs` (otherwise
   these 159 lines still will not run).

**Stage 2: Address the seven empty shells.** The key constraint is that `pub use xxx::*;` at
`tests/mod.rs:28-38` produces **no compiler error or warning** for an empty module. Once wiring
succeeds, CI will show “all tests pass” despite the `basic` / `comments` / `delimiters` / `errors` /
`keywords` / `operators` / `debug_lexer` modules containing **not one assertion**. This **creates
the false impression that the lexer is covered** and is more dangerous than not wiring the tests at
all.

| Option                   | Approach                                                                                              | Evaluation                                                                                                                                    |
| ------------------------ | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| **Delete (recommended)** | Delete the seven empty-shell files + corresponding `mod` declarations + corresponding `pub use` lines | **Clean.** The only value of an empty shell is “fill it in later,” and `literals.rs` / `rfc004_lexer.rs` already cover these responsibilities |
| Fill                     | Write real assertions for each empty shell                                                            | Seven files × unknown content; **what should be tested cannot be verified** (unverified)                                                      |
| Retain + mark            | Retain but add `// TODO` and a CI count warning                                                       | Creates a false impression of coverage; **rejected**                                                                                          |

**Adopt deletion**, and, together with the dead-code cleanup, delete the entire `pub use` re-export
block at `tests/mod.rs:28-38` (it existed to support a nonexistent re-export design and was
precisely what concealed the empty shells).

**Stage 3: Add associativity cases.** Add `parser/tests/associativity.rs` (WBS 8.8.1), using only
the public entry points `parser::parse` / `parse_with_probe` to assert **language-level AST
shapes**. The cases must not reference Pratt internals—binding-power constants, the `pratt`
submodule path, or `ParserState`—so they survive unchanged after the 8.4 dual-parser differential
and the 8.6 deletion of Pratt, guarding C5’s “valid programs retain identical AST shapes”:

- For every pair of adjacent precedence levels, write one minimal expression and assert its
  associativity direction—for example, `a - b - c` → `(a-b)-c`; see the “precedence level and
  associativity” columns in SPEC syntax §2.2.
- Add one case for each of the following: `..` (`a..b..c` → `(a..b)..c`, with the upper bound of
  `0..n+2` consuming the addition), the right-greedy body of `=>`, right-associative assignment, `?`
  and call-chain suffixes, and pattern `|` chain termination. The `..` and `=>` binding-power pairs
  are the two easiest omissions during this migration.
- **Do not make “binding power strictly increases within a level, with no duplicates” a gating
  assertion**: according to coding-rules 7.1, a correct implementation that reorders values within
  the same level would turn red. That freezes an implementation choice and can only be used as
  characterization. Level relationships are protected by the two groups of language-level AST shapes
  above.

**Stage 4: Add literal error-path cases.** Add cases for each of the four categories in the
uncovered-path table: overflow in all four radices, the branch that continues consuming after
`checked_mul`, `scan_leading_dot` as a whole, and invalid `\x` / `\u` escapes.

**Stage 5: Delete `precedence_inline.rs`** together with the dead code, and add a **wiring
self-check** assertion to `pratt/tests/mod.rs` to prevent another orphan.

### Before and After: How Many Locations Must Change When Adding an Operator?

| #   | Current state                                                                      | Current enforcement                             | Target                                             | Target enforcement                               |
| --- | ---------------------------------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------- | ------------------------------------------------ |
| 1   | Add a `match` arm to `keyword_from_str` at `lexer/state.rs:21` (keyword form only) | None                                            | One row in the operator declaration table          | Compile time                                     |
| 2   | Add a variant to `TokenKind` at `lexer/tokens.rs:81-162`                           | Compile time                                    | **Generated**                                      | Compile time                                     |
| 3   | Add a BP constant to either `pratt/precedence.rs:6-17` or `22-31`                  | None                                            | **Generated**                                      | Compile time (within-level uniqueness assertion) |
| 4   | Add an arm to `infix_info` at `pratt/led.rs:30-88`                                 | Compile time (exhaustive)                       | **Generated**                                      | Compile time                                     |
| 5   | Add an operator-mapping arm to `parse_binary` at `pratt/led.rs:116-136`            | Compile time (exhaustive)                       | **Generated**                                      | Compile time                                     |
| 6   | Add a variant to `BinOp` at `ast.rs:191-213`                                       | Compile time                                    | **Generated**                                      | Compile time                                     |
| 7   | New symbolic character: `tokenizer.rs:218` `next_token_inner`                      | None                                            | **Generated**                                      | Compile time                                     |
| 8   | Add a variant to `const_data::BinOp` at `const_data.rs:234`                        | **None (cross-enum; compiler cannot catch it)** | **Generated from the same source as `ast::BinOp`** | Compile time                                     |
| 9   | Sites in 17 downstream production files that rely on `matches!` / `_` fallbacks    | **None (silent)**                               | Generated accessors                                | Compile time                                     |

**Summary:**

| Stage                                 | Number of manual changes                                                                                       | Number of automatically generated changes |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| Current state                         | **11–12** (6–7 within L2 + approximately 5–6 downstream production files that must be synchronized)            | 0                                         |
| Target (purely new symbolic operator) | **1–2** (one row in the operator declaration table; one additional row in `keyword_from_str` for keyword form) | 7–8                                       |
| Target (reuse an existing token)      | **1**                                                                                                          | 6–7                                       |

All 1–2 target changes are **declarative data rows**, not logic code—the review cost changes from
“reasoning about the semantics of each `match` arm” to “reading one declaration.”

## Detailed Design

### Interface Boundary with the Type System

**Boundary declaration (important)**: convergence of `ast::Type` (26 variants, `ast.rs:427-541`),
elimination of parallel type representations, and ownership of `is_type_param_annotation` /
`name_used_as_type` **all belong to [03-type-unification.md](03-type-unification.md)**. This article
handles only three interfaces directly related to the frontend paradigm:

**Interface one: the relationship between `const_data::BinOp` and `ast::BinOp`.** This belongs to
**this article**, because it is a direct component of the “adding an operator” change surface.
Treatment: **simultaneously generate both enums from the operator declaration table** (or generate
one and derive the other), eliminating the import aliases at `const_eval.rs:19` / `ir_gen.rs:2317`.
The reason this belongs here rather than in document 03 is that it is an L2 artifact duplicated in
parallel in L3, not a problem with type representations themselves.

**Interface two: the hard-coded `"Terminates"` at `declarations.rs:498-509` and the call to
`typecheck::operator_interfaces::spec()`.** This belongs to **03-type-unification.md /
02-stage-contract.md** (eliminating the reverse parser → typecheck dependency, the first row of
RFC-039 Routing Table C). During the function split, this article **isolates this code into
`apply_semantic_side_effects`** as a handoff point, but does not change its contents.

**Interface three: whether an operator’s precedence level needs type-layer information.** It does
not currently—`infix_info` is a purely syntactic function. **Design judgment**: this property should
be preserved after table-driven parsing. The operator declaration table **must not contain any
type-related fields**, or L2 would once again depend on L3, violating RFC-039’s dependency-direction
rules.

### Runtime Behavior

**Goal: runtime behavior remains bit-for-bit identical.** Under the C5 criteria in
07-equivalence-oracle.md, the third tier—end-to-end corpus differential—compares the exit codes and
stdout/stderr of the 293 `.yx` files in `tests/yaoxiang/` + `src/std/tests/*.yx`.

Three **identified runtime effects** must be handled explicitly:

| Change                                                                         | Runtime effect                                   | Treatment                                                                                   |
| ------------------------------------------------------------------------------ | ------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| Consolidating f-string interpolation into one tokenization pass                | **Performance improvement**, output unchanged    | Measure the time savings and record them                                                    |
| Generating BP from the operator table / merging and reordering the two ladders | **May change associativity**                     | **Must write an equivalence case for every pair of levels** (Test Reconstruction · Stage 3) |
| Deleting `is_old_function_syntax`                                              | The old syntax’s diagnostic code/span may change | Prove consistency through a corpus differential                                             |

**Explicit non-goals**: do not change the **semantics** of operator precedence. BP numbers may be
reordered to eliminate dead code and collisions, but relationships such as “multiplication binds
more tightly than addition” must remain unchanged.

### Compiler Change List

File by file, with line numbers. C6 cleanup (stage 0, independently committable; **8.0.1 / 8.0.2
were executed on 2026-11-22; the line numbers below refer to the tree before execution, and the
final state is recorded in the task 8.0 commit information**):

| File                                                        | Line numbers                          | Change                                                                                                         |
| ----------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/pratt/precedence.rs`              | `8-13`, `11`                          | Delete the six zero-reference constants from the first ladder                                                  |
| Same                                                        | `22`                                  | Change `BP_RANGE` to a named `..` binding-power pair (synchronized at `:40`), or delete it                     |
| Same                                                        | `35-94`                               | Delete the `Precedence` enum                                                                                   |
| Same                                                        | `98-134`                              | Delete the `PrecedenceContext` struct                                                                          |
| Same                                                        | `6-17`, `22-31`                       | Retain live constants and merge them with the second ladder into conflict-free numbering within each level     |
| `src/frontend/core/parser/pratt/led.rs`                     | `33`                                  | Point `BP_ASSIGN` to the reordered assignment-level constant                                                   |
| Same                                                        | `40`                                  | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R`                                                                          |
| Same                                                        | `75`, `77`, `79`, `83`                | Point `BP_CALL` to the reordered suffix-level constant                                                         |
| Same                                                        | `81`                                  | Point `BP_CAST` to the reordered cast-level constant                                                           |
| Same                                                        | `85`                                  | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R`                                                                       |
| `src/frontend/core/parser/pratt/nud.rs`                     | `33`, `36`, `84`, `102`, `228`, `248` | Point `BP_UNARY` to the reordered prefix-level constant                                                        |
| Same                                                        | `39-70`                               | Point `BP_HIGHEST` to the reordered highest-level constant                                                     |
| Same                                                        | `1019`                                | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`, and add a comment explaining the value’s origin |
| `src/frontend/core/parser/statements/declarations.rs`       | `85-87`                               | Delete `skip_old_function_syntax`                                                                              |
| Same                                                        | `695`                                 | Delete its call site (deletion of `is_old_function_syntax` is covered in stage 2)                              |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | Entire file                           | Delete (the orphaned sole referencer of the dead code)                                                         |

Associated renames (the collateral surface of the BP merge, approximately 89 locations):
`parser/mod.rs:113`, 8 locations in `statements/control_flow.rs`, `statements/declarations.rs:19` /
`287` / `577` / `705` / `785` / `821` / `940` / `972`, `statements/functions.rs:9` / `:107`,
`statements/bindings.rs:170`, `statements/types.rs:16` / `:782` / `:794`, `pratt/led.rs:99` / `:227`
/ `:315` / `:321` / `:409`, and `pratt/nud.rs:200` / `:268` / `:418` / `:471` / `:478`.

Test restoration (stage 1, independently committable):

| File                                                 | Line numbers            | Change                                                                       |
| ---------------------------------------------------- | ----------------------- | ---------------------------------------------------------------------------- |
| `src/frontend/core/lexer/mod.rs`                     | `167-168`               | Add `#[cfg(test)] mod tests;`                                                |
| `src/frontend/core/lexer/tests/mod.rs`               | `12-18`                 | Add `mod lexer_mod;` and `mod symbols;`                                      |
| Same                                                 | Entire file             | Delete the `mod` declarations and `pub use` lines for the seven empty shells |
| Same                                                 | Entire file             | Delete the `pub use ... ::*;` re-export block (`28-38`)                      |
| `src/frontend/core/lexer/tests/`                     | Seven empty-shell files | Delete the files                                                             |
| `src/frontend/core/parser/pratt/tests/mod.rs`        | `4-7`                   | Add a wiring self-check assertion                                            |
| `src/frontend/core/parser/pratt/tests/precedence.rs` | `6-13`                  | Replace the no-op assertion with real associativity assertions               |

Paradigm changes (stages 2–4):

| File                                                           | Change                                                                                                                                                                                                                                     |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/frontend/core/lexer/literals.rs`                          | Merge the four radix scanners into `scan_radix_number(lexer, Radix, String)`; merge the three escape decoders into a single `decode_escape(lexer, &mut String, EscapeMode)`; merge `scan_multi_line_string` with `scan_fstring_multi_line` |
| `src/frontend/core/parser/pratt/nud.rs:367` / `:398`           | Remove runtime nested `tokenize()` and produce the result in one lexical pass                                                                                                                                                              |
| `src/frontend/core/parser/pratt/led.rs:30-88` / `nud.rs:27-76` | Replace the two handwritten tables with generated mapping tables                                                                                                                                                                           |
| `src/ast/mod.rs:194-218`                                       | Generate `BinOp` and `const_data::BinOp` from the same source                                                                                                                                                                              |
| `src/frontend/core/types/const_data.rs:290`                    | Same                                                                                                                                                                                                                                       |
| `src/frontend/core/parser/statements/declarations.rs:101-620`  | Split into eight functions + `ParserState::skip_balanced_parens()` (change the four locations at `61-70`/`112-120`/`156-164`/`186-194` to call it)                                                                                         |
| `src/frontend/core/parser/parser_state.rs`                     | Add `skip_balanced_parens()` (`synchronize()` remains at `:330`)                                                                                                                                                                           |

**Files with no changes in this article**: the **main bodies** of `src/middle/core/ir_gen.rs`,
`src/frontend/core/typecheck/**`, `src/backends/**`, and `src/frontend/core/formatter/**`. They
change only when generated accessors replace `matches!` macros, and those changes are mechanical.

### Backward Compatibility

Table-driven parsing is most likely to break **error recovery**. This project’s recovery contract
consists of three parts, each of which must be preserved:

**Contract one: `ParserState::synchronize()` (`parser_state.rs:170-186`).** After statement parsing
fails, it skips tokens forward until a synchronization point. **The C5 criteria require retaining it
and require its triggering timing and skipped set to match exactly.** After migration to LALRPOP,
its call sites are driven by explicit error productions, while the function body and
synchronization-point set **remain unchanged line for line**—the grammar reproduces the semantics,
and the implementation is retained as is.

**Contract two: `Expr::Error` (consumed at `ast.rs:1048` and `pratt/mod.rs:72`) and
`StmtKind::Error` (constructed at `parser/mod.rs:50`).** These placeholders allow the parser to
continue constructing a “partial AST” after an error, and downstream code skips it through the
`Error` branch. **The variants must be retained, and their positions in downstream matches must also
be retained**—if downstream code places `Expr::Error` in a wildcard branch, the error will be
silently swallowed.

**Contract three: diagnostic code, span, and ordering.** The comparison rules in the third tier of
07-equivalence-oracle.md sort the diagnostic list by `(code, span.file, span.line)` and **do not
include message text in the comparison**. This gives this article some latitude in wording, but
**code and span are immutable**.

**Special note on f-strings**: the interpolation refactoring changes spans from “relative to
interpolation text” to “absolute offset in the source file.” To satisfy contract three, **spans must
be recorded as absolute offsets during lexing** (Decision D25)—the `span.file/line` of diagnostics
inside interpolations must not change as a result of this refactoring.

> **Treatment**: before stage 2, establish a baseline snapshot of “diagnostic spans inside f-string
> interpolations” and compare it entry by entry after the refactoring. Any discrepancy in
> `span.file/line` is an implementation defect and must be fixed; there is no option to “accept span
> changes.” (If testing proves that the original interpolation spans were already incorrect, the
> baseline-establishment stage will expose that—an existing independent defect, to be fixed
> separately, not folded into this stage, and not a reason to relax the criteria.)

## Implementation Essentials

This article corresponds to five stages. **Every stage can be committed and rolled back
independently.** Stages 0 and 1 are C6 and test restoration and require no equivalence criteria;
stages 2–4 are C5 and are constrained by the criteria throughout.

| Stage  | Content                                                                                                                                         | Category    | Acceptance criteria                                                                                                                               | Rollback point                            |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| **0**  | Dead-code cleanup (`Precedence` enum + `PrecedenceContext`, 96 lines)                                                                           | C6          | `cargo build` passes; `cargo clippy` produces no new warnings                                                                                     | Single commit; `git revert` is sufficient |
| **1**  | Test restoration and completion                                                                                                                 | —           | Lexer actually runs tests in ≥ 5 files / 629 lines; seven empty shells deleted; associativity cases **fail before the change and pass afterward** | Single commit                             |
| **2**  | Lexer duplication convergence + elimination of f-string nested compilation                                                                      | C5          | Identical AST snapshots + identical diagnostics + identical behavior; `literals.rs` reduced by approximately 500 lines                            | Separate commit from stage 3              |
| **3a** | Build the LALRPOP grammar + action code, **completely alongside Pratt**                                                                         | —           | `cargo build` passes; the new parser can produce ASTs for the 293 corpus files (without comparison)                                               | Separate commit from stage 2              |
| **3b** | **Dual-parser differential** (run both parsers over the 293 corpus files and **compare normalized ASTs + diagnostic code+span entry by entry**) | **Full C5** | **ASTs and diagnostics are equivalent entry by entry—this step is the equivalence proof for the entire grammar migration**                        | Run offline once; repeatable              |
| **3c** | Switch traffic: change `parse()` in `parser/mod.rs` to call LALRPOP; retain Pratt as `parse_legacy()`                                           | **Full C5** | Full-corpus behavioral equivalence + diagnostic code+span identical entry by entry                                                                | Independent commit                        |
| **3d** | Delete Pratt: `nud.rs`(1326) + `led.rs`(495) + both BP ladders in `precedence.rs` + approximately 89 references                                 | **Full C5** | Entire corpus green; `git grep BP_` returns zero hits                                                                                             | Independent commit                        |
| **4**  | Split `parse_assign_after_target`                                                                                                               | C5          | The function only dispatches; `skip_balanced_parens` has a single implementation                                                                  | Independent commit                        |

**Internal dependency order**: stage 1 **must precede** stages 2/3/4—“refactor first, add tests
later” is a trap this project has already encountered. **Stage 3b (dual-parser differential) cannot
be skipped**: it must produce an entirely green result before the 3c traffic switch, or the grammar
migration has no equivalence basis.

**The 3b/3c go/no-go gate**: if the number of error productions clearly exceeds what is needed to
“model the synchronization-point set from `synchronize()`”—the parser-embedded semantic decisions,
such as `declare_predicate` accumulation and const generic-parameter filtering, are the hardest to
relocate—or if the differential still contains non-equivalent items after two consecutive
iterations, **stop the migration and reassess**, falling back to the intermediate solution “retain
Pratt + a single declarative precedence table + generate both `BinOp` enums from the same source.”
That solution already captures most of the benefit of converging the operator change surface;
LALRPOP is the means, not the goal. This fallback belongs to the “reassessment” branch reserved by
D24 and **is not a relaxation of the criteria**.

**One clarification about stage 0**: the six **live** first-ladder constants—`BP_LOWEST` /
`BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`—**are not within stage 0’s deletion
scope**. They are deleted with Pratt in stage 3d. Stage 0 deletes only the `Precedence` enum and
`PrecedenceContext` (96 lines, zero production references).

**How the criteria are executed**: see [07-equivalence-oracle.md](07-equivalence-oracle.md) for the
definitions of the three tiers and the regression gate. **Everything uses full C5, including
diagnostic code+span. There is no relaxation whatsoever.**

**Scope limitation**: this article **does not include** a `logos` migration—the lexer remains
handwritten with its duplication consolidated, for the reasons given in “Key Decisions and
Rationale.” **The LALRPOP parser migration is part of this article**; see “Parser: Complete
Grammar-Driven Parsing.”

## Key Decisions and Rationale

### Three Selected Decisions

| Decision                         | Choice                                                                                                    | Rationale                                                                                                                                                                                                                                                                                                           |
| -------------------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lexer approach                   | Adopt “retain the handwritten lexer + enforce deduplication”; retain `logos` as a conditional alternative | Approximately 500 lines of duplication can be eliminated with zero new dependencies; `logos` changes the `LexError` type and cannot solve f-string nested compilation                                                                                                                                               |
| **Parser approach**              | **Adopt complete grammar-driven parsing with LALRPOP**; reject tree-sitter and “retain Pratt with tables” | tree-sitter produces a CST, not a domain AST, and still requires a complete mapping layer; table-driven parsing solves synchronization but not maintainability and is an insufficient intermediate state. LALRPOP’s equivalence risk is eliminated by the **dual-parser differential** (see above), not by argument |
| Ownership of `const_data::BinOp` | This article                                                                                              | It is a direct component of the “adding an operator” change surface; it is handled separately from convergence of `ast::Type` (document 03)                                                                                                                                                                         |

### Explanation of “Doing It Completely”

Table-driven parsing is an insufficient intermediate state: it solves synchronization, not
maintainability; and “wait until the criteria are relaxed and reassess” is passive waiting, not an
engineering decision. This article therefore adopts **complete grammar-driven parsing**, with
equivalence guaranteed through empirical dual-parser differentials (see above), and makes no
relaxation at the diagnostic layer.

**The cost of this shift must be stated clearly**:

- Construction effort rises significantly: a grammar file must be created, construction actions for
  22 `Expr` variants must be moved, approximately 89 BP references must be merged, and error
  recovery must be rewritten.
- **Prerequisites become harder**: all 629 lines of frontend tests must first be restored (P1 in
  document 09), and normalized AST snapshots must first be committed (P2 in document 09). Without
  both, the differential in step 2 cannot begin.
- **One new dependency is introduced** (LALRPOP), which is inconsistent with the lexer layer’s “zero
  new dependencies” direction.

**The benefit is certain**: “adding an operator” drops from 11 locations to 2, BP conflicts change
from “accidentally correct” to “grammar build-time errors,” and the `Expr::Lambda` borrowing, the
`parse_expression(12)` magic number, and approximately 120 lines of dead legacy-syntax probing
**naturally disappear**.

### Rejected or Downgraded Options

**A. Delete dead code only, with no paradigm change.** Feasibility has been verified, but it is
insufficient to solve the problem. Dead-code cleanup (approximately 130 lines + three magic numbers)
can be completed at **zero criteria cost** and should be done immediately. However, it does not
reduce the L2 change surface by even one location and does not address cross-enum missed updates at
all. **Conclusion: adopt as stages 0 and 1, but continue with grammar-driven parsing. Treating A as
the complete solution is false reassurance.**

**B. Retain Pratt and add tests only.** **Not adopted as a complete solution, but adopted as a
prerequisite.** Tests **cannot detect cross-enum missed updates**—both enums can compile
independently, and whether the tests pass is unrelated to whether the enums are synchronized.
**Conclusion: adopt as stage 1; reject as a complete solution.**

**C. Retain Pratt and use table-driven parsing.** **Rejected** (2026-10-03). It is an unnecessary
intermediate stop between “entirely handwritten” and “complete grammar-driven”: the 22 variants of
`Expr` still require 22 handwritten construction actions, and the
`Expr::Lambda { body: empty Block }` borrowing problem, the `parse_expression(12)` magic number, and
all 89 references across the two ladders remain.

**D. Adopt tree-sitter.** **Rejected.** Its incremental parsing offers no material benefit to this
project (full compilation on every run, 293 corpus files); it also produces a CST rather than a
domain AST and still requires a complete mapping layer, adding roughly the same amount of code as
`nud.rs` outside that file.

**E. Replace the handwritten lexer with `logos`.** **Conditionally not adopted.** **Prerequisites
for reassessment**: after stage 2 is complete—the escape decoder has been consolidated into one
implementation—replacing that implementation with a `logos` callback will require substantially less
work.

### Net Benefit of the Approach

- **The change surface drops from “more than ten locations of logic code” to “1–2 locations of
  declarative data + 7–8 generated locations.”** The generated 7–8 locations are guaranteed to stay
  synchronized by the compiler, so missed updates are no longer possible.
- **A class of defects the compiler can never catch is eliminated.** Cross-enum missed updates
  (`ast::BinOp` vs `const_data::BinOp`) are currently the only defect type for which **no tool can
  provide any detection**, because the two types are independent. Generating both from the same
  source eliminates them directly.
- **BP collisions change from “accidentally correct” to “compile failure.”** The existing four
  cross-ladder collisions and two within-ladder collisions currently survive by chance through the
  unwritten `bp_right = bp_left + 1` convention. Within-level uniqueness assertions ensure that this
  class of problem can never again exist silently.
- **Frontend tests change from “one file running” to “all running.”** Once the existing 629 lines of
  tests are restored, stages 2–4 have a safety net; these tests **already existed** and were omitted
  only because of one missing `mod` declaration.
- **Stages 0 and 1 have zero risk and zero criteria cost.** Dead-code deletion and test restoration
  can be done immediately without blocking any other work.

## Known Limitations and Risks

### Design Risks

- **The operator table is a new “thing that must be maintained.”** It changes “change 11 locations”
  into “change one location + maintain a table.” **If the table’s fields are poorly designed, it can
  become a new centralized maintenance burden.** This is the approach’s primary risk; the mitigation
  is to protect every column with compile-time assertions.
- **The risk from renumbering BP is asymmetric.** Merging the two ladders involves renaming
  approximately 89 production references; if one is missed, **associativity may change
  silently**—without an error, merely a different expression parse result. This is the most likely
  silent regression introduced by this article and must be protected by the per-level-pair
  associativity cases in stage 1.
- **Stages 2–4 have a high C5 criteria cost.** Using all three tiers means every stage must run the
  full corpus (293 `.yx` files) + AST snapshots + diagnostic comparison. This makes every commit in
  stages 2–4 relatively heavy.
- **The f-string span change may not be entirely avoidable.** See “Backward Compatibility”; it may
  be necessary to accept one known behavioral change and update the baseline.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> decision on each item. **This article leaves no pending items.**

## Verification Record

All code facts in this article were verified by **directly reading the files on disk**. Verification
method: the `read` tool directly read the relevant line ranges of target files; line counts were
obtained with `Get-Content`; reference surfaces were scanned repository-wide with the `grep` tool
and classified into production / test / orphan. **`cargo test` / `cargo build` were not used**—this
is a pure document, and criteria execution belongs to the implementation stage.

### Item-by-Item Measured Results

| Verified item                                          | Measured result                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Total lines in `literals.rs`                           | **1568**                                                                                                                                                                                                                                                                                         |
| Total lines in `tokenizer.rs`                          | **554**                                                                                                                                                                                                                                                                                          |
| `next_token_inner` line number                         | **`:218`**                                                                                                                                                                                                                                                                                       |
| Four radix scanner line ranges                         | `77-161` (85 lines) / `164-247` (84) / `250-333` (84) / `336-516` (181)—consistent with the table above                                                                                                                                                                                          |
| Three escape-decoding implementations                  | `694-827` (134 lines) / `1015-1141` (127 lines) / `push_fstring_escape:1374`—consistent with the table above                                                                                                                                                                                     |
| `scan_leading_dot`                                     | `519-635`, **117 lines**                                                                                                                                                                                                                                                                         |
| Number of `Expr` variants                              | **22** (including `Error`), `ast.rs:16-163`                                                                                                                                                                                                                                                      |
| Number of files in `lexer/tests/`                      | **14** (including `mod.rs`)                                                                                                                                                                                                                                                                      |
| Lines in orphaned real tests                           | **629** (222+167+89+81+70); another 90 lines in `fstring.rs` are already running via `#[path]` and are not counted as orphaned                                                                                                                                                                   |
| Changes required to restore `lexer/tests/`             | **Two**: add `mod tests;` to `lexer/mod.rs`, **and** add `mod lexer_mod;` `mod symbols;` to `tests/mod.rs`. Adding only one leaves the final 159 lines unrun                                                                                                                                     |
| Dead code in `precedence.rs`                           | **96 lines** (`Precedence` enum `35-94` + `PrecedenceContext` `98-133`) with zero production uses **confirmed**; however, their sole external referencer, `precedence_inline.rs` (95 lines / 6 tests), is itself an orphan—**deleting the dead code requires deleting that file as well**        |
| Lines in the seven empty shells                        | 1–3 lines each (`debug_lexer.rs` is 1 line; the others are 3 lines)                                                                                                                                                                                                                              |
| Shape of the blank-line residue                        | **Three consecutive blank lines inserted between each field, across 11 lines** (`777-787` / `1098-1108`)                                                                                                                                                                                         |
| Lines in `pratt/tests/precedence.rs`                   | **13**; its only assertion is `assert!(bp_lowest < bp_highest)`                                                                                                                                                                                                                                  |
| Fourth “skip balanced parentheses” location            | `224-233`                                                                                                                                                                                                                                                                                        |
| Downstream change surface                              | **17 production files** match `BinOp::` (repository-wide: **534 hits / 47 files**)                                                                                                                                                                                                               |
| Whether `src/backends/` consumes `BinOp`               | **No**—`src/backends/` has **zero hits** for either `BinOp` or `ast::`; the interpreter consumes IR / bytecode and never sees the AST                                                                                                                                                            |
| “Can a missed update go undetected without any check?” | **Must be stratified**: a missed arm in an exhaustive `match` **is detected by the compiler**; what fails silently are `matches!` macros, `_` fallbacks, and **cross-enum** updates. This article stratifies the statement according to the measurements                                         |
| `matches!` blocks in `const_data.rs`                   | `263-288` (the four-function range `is_arith` / `is_comparison` / `is_logical` / `is_bitwise`)                                                                                                                                                                                                   |
| `impl Display for BinOp`                               | Starts at `291`; 18-arm exhaustive match with no `_` fallback                                                                                                                                                                                                                                    |
| The two functions in `parse_assign_after_target`       | **Not dead code**: both are still called at `declarations.rs:719` / `:725` as a running “reject removed syntax” gate; moreover, `skip_old_function_syntax` (`120-122`, whose body consists of one comment line) **consumes no token** after being called (`:725-726` immediately returns `None`) |
| Second `BinOp` enum                                    | `const_data.rs:234` does exist and has **no `From` conversion whatsoever** to `ast::BinOp`, forcing files such as `const_eval.rs:19` / `ir_gen.rs:2317` to alias it at import time                                                                                                               |
| Re-export block in `lexer/tests/mod.rs`                | Exists (38-line file); `:28-38` contains `pub use ... ::*;`, directly causing the empty shells to masquerade as coverage                                                                                                                                                                         |

### Three Corrections from This Round of Verification

The following three points conflict with the intuition that “the first BP ladder is dead”; the
measurements take precedence:

1. **The two ladders are intertwined, not “one live and one dead.”** `infix_info` (`led.rs:30-88`)
   itself references five constants from the first ladder (`BP_ASSIGN:33`, `BP_CALL:75/77/79/83`,
   `BP_CAST:81`) and eight from the second. Of the first ladder’s 12 constants, **only six truly
   have zero references** (`BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` /
   `BP_FACTOR`, plus `BP_COMPARISON`, which survives only in the comment at `declarations.rs:742`);
   the other six (`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`)
   have approximately **89 references across 8 production files**. Therefore, “deleting the first BP
   constants” is not a 12-line C6 deletion, but a cross-file rename in the C5 category.
2. **The second ladder also has a dead member.** `BP_RANGE` (`precedence.rs:22`) has zero production
   uses—`..` uses the bare literal `(6, 7)` at `led.rs:40`, and `BP_RANGE` appears only in the
   comment at `led.rs:157-159`.
3. **The “self-contradictory comment” is at `precedence.rs:33`, not `:11`.** `*:11` is
   `pub const BP_COMPARISON: u8 = 5;`. The comment (_“Precedence rules for the Pratt parser”_)
   describes the zero-reference `Precedence` enum.

Additional correction: `bp_right = bp_left + 1` produces **left associativity** (`a - b - c` →
`(a-b)-c`), not right associativity; `(11, 1)` at `led.rs:85` is the only explicit
greedy-right-operand formulation. There are also more collisions than the four cross-ladder ones:
`BP_UNARY=8 == BP_CAST=8` (a collision within the first ladder, with both live) and
`BP_RANGE=7 == BP_ADD=7` (a collision within the second ladder).

### Unverified Items

The main text marks each of the following as “unverified”:

- The precise arm count and binding power of each arm in the `prefix_info` table body at
  `pratt/nud.rs` (`26-75`)
- The exact line boundaries of the eight responsibility sections in `declarations.rs:136-642`
- The complete variant set of `const_data::BinOp` (`const_data.rs:234`)
- Whether `logos` / `regex` are already present in `Cargo.toml` as transitive dependencies
- All call sites of `synchronize()` outside `parser_state.rs:170-186`
- Whether this project is a published library (which affects the risk assessment for removing public
  API surface)

## See Also

### Companion Design Documents in the Same Batch

- [01-routing.md](01-routing.md) — Where to change when adding a feature (Functional Routing Tables
  A/B/C and dependency-direction rules)
- [03-type-unification.md](03-type-unification.md) — Ownership of `ast::Type` convergence and the
  interface boundary with this article
- [04-ssa.md](04-ssa.md) — Another user of the C4/C5 criteria
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Complete inventory of the eight orphaned test
  trees
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — Definitions of the C5/C6 categories, the
  three criteria tiers, and the regression gate

### Main RFC and Related Proposals

- [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md) —
  Governing RFC; four-layer model, glossary, Functional Routing Tables A/B/C
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Build-time gate example borrowed by this article
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
- `src/frontend/core/parser/pratt/nud.rs:896-901` / `1041` — Origin and use of the magic number `12`
- `src/frontend/core/parser/pratt/precedence.rs:6-17` and `22-31` — Two BP ladders
- `src/frontend/core/parser/pratt/precedence.rs:33` / `35-94` / `98-133` — Diverged comment,
  zero-reference enum, and struct
- `src/frontend/core/parser/pratt/led.rs:30-88` — Handwritten `infix_info` table
- `src/frontend/core/parser/pratt/tests/mod.rs:4-6` — Orphaned wiring caused by the missing
  `precedence_inline` declaration
- `src/frontend/core/parser/statements/declarations.rs:719-727` — Legacy-syntax rejection gate and
  its “skip without consuming” defect
- `src/frontend/core/types/const_data.rs:234` — The second operator enum parallel to `ast::BinOp`
- `build.rs:19-55` — Build-time error-code gate and an existing enforcement mechanism in this
  repository
- `src/middle/core/tests/bytecode.rs:369-657` — `test_every_opcode_roundtrips_not_silently_nop`, an
  existing “item-by-item round-trip” criteria example in this repository
