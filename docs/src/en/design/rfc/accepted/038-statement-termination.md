---
title: 'RFC-038: Statement Termination & Newline Rules'
author: 'ChenXu233'
created: '2026-08-05'
updated: '2026-08-05'
issue: '#258'
issues_impl: '#258'
status: 'Accepted'
---

# RFC-038: Statement Termination & Newline Rules

## Summary

Define YaoXiang's **statement termination rules**: **newlines as the primary boundary**, with
**unclosed brackets, line-ending binary operators, and line-leading `.` (chain continuation)** as
explicit continuation exceptions; line-leading `(` `[` **never merge into the previous statement**.
`;` is preserved as an explicit separator (multiple statements on a single line).

This RFC also fills the specification gap in `syntax.md` that never defined statement terminators,
and fixes the parser's defect of swallowing line-leading `(` `[` `.` as suffixes of the previous
statement (#258).

## Motivation

### Why is this feature needed?

In YaoXiang, `;` is fully optional (the parser has 14 `skip(Semicolon)` sites), but **newlines have
never been defined as statement boundaries**: the lexer discards newlines, and the parser's sole
statement boundary criterion is "the expression Pratt loop naturally stops"—i.e., "whether the next
token can continue the expression." This leads to:

1. **Self-contradictory behavior**: a newline followed by an identifier/literal → normal
   termination; a newline followed by `(` `[` `.` → swallowed as a suffix of the previous expression
   (call/index/field access)
2. **Legal statements silently broken**: the destructuring `(c, d) = (3, 4)` becomes
   `1(c, d) = (3, 4)`, producing a misleading E1001; `f()(2)`, `1[99]` don't even report errors—the
   statements simply evaporate
3. **Specification gap**: `syntax.md` §2.9 `Block ::= '{' Stmt* Expr? '}'` does not define
   separators between Stmts; §1.2's separator table only contains `( ) { } ,`

### The current problem

| Code in block               | Current AST                                      | Result                            |
| --------------------------- | ------------------------------------------------ | --------------------------------- |
| `x = 1` ⏎ `(c, d) = (3, 4)` | `Assign x = BinOp(Assign, Call(1,[c,d]), Tuple)` | E1001 pointing at innocent `c`    |
| `x = f()` ⏎ `(2)`           | `Call(Call(f),[2])`                              | **Silent** (statement evaporates) |
| `x = 1` ⏎ `[99]`            | `Index(1,99)`                                    | **Silent**                        |
| `x = 1` ⏎ `.println("hi")`  | `Call(Field(1),["hi"])`                          | E1053                             |
| `x = 1 +` ⏎ `2`             | `BinOp(Add)`                                     | ✅ Legal (but no spec basis)      |
| `x = (1,` ⏎ `2)`            | `Tuple`                                          | ✅ Legal (but no spec basis)      |

## Proposal

### Core Design

**Primary rule: newlines terminate statements.**

**Continuation exceptions (three groups, all with precedents in mainstream languages):**

| #   | Exception                               | Rule                                                                           | Precedent    |
| --- | --------------------------------------- | ------------------------------------------------------------------------------ | ------------ |
| 1   | **Unclosed brackets**                   | When `(` `[` `{` depth > 0, newline does not terminate (implicit continuation) | Python/Scala |
| 2   | **Line-ending binary operator**         | A line ending with a binary operator → continuation                            | Swift/Scala  |
| 3   | **Line-leading `.` chain continuation** | Line-leading `.` and previous line ends with identifier/`)`/`]` → continuation | Swift        |

**Never merge (learned from JS's biggest mistake):**

- Line-leading `(`, `[` → **always start a new statement**. JS's famous pitfall (`a\n(b)` → `a(b)`
  call) is especially dangerous in YaoXiang: tuple destructuring `(a, b) = ...`, spawn, tuple
  literals are all bracket-leading statements.
- Line-leading binary/unary operators (`+` `-` `*` etc.) → start a new statement. Line-ending
  operator continuation already covers common line-break styles; line-leading operator continuation
  (Scala 3 leading operator) introduces unary/infix ambiguity, so it is not adopted. If line
  breaking is needed, wrap in brackets.

**`;` preserved**: explicit separator, used when multiple statements appear on a single line
(Kotlin/Swift style).

### Examples

```yaoxiang
// Newline terminates statements (vast majority of code)
a = 1
b = 2

// Continuation: line-ending binary operator
total = a +
    b + c

// Continuation: unclosed brackets
t = (1,
     2)
io.println(
    "hi")

// Continuation: line-leading . (chained call)
result = list.map(x => x * 2)
    .filter(x => x > 10)
    .sum()

// Never merge: line-leading ( is an independent destructuring statement
x = f()
(c, d) = (3, 4)      // ✅ destructuring, not f()(c, d)

// Never merge: line-leading [ is an independent list
x = 1
[1, 2, 3]            // ✅ independent expression statement

// Multiple statements on one line: semicolon
a = 1; b = 2
```

### Syntax Changes

Added after `syntax.md` §2.9:

```
StatementTerminator ::= ';' | Newline        (unless the following continuation exceptions apply)
Continuation exceptions (newline does not terminate):
  - '(' '[' '{' depth > 0
  - Line ends with a binary operator
  - Line begins with '.' and previous line ends with Identifier | ')' | ']'
Never merge: line-leading '(' '[' always start a new statement
```

| Before                                                          | After                                                                                           |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `Block ::= '{' Stmt* Expr? '}'` (no separator definition)       | `Block ::= '{' (Stmt StatementTerminator)* Expr? '}'`                                           |
| Newlines have no status; line-leading `(` `[` `.` get swallowed | Newlines terminate; line-leading `(` `[` never merge; line-leading `.` is explicit continuation |
| `;` optional but semantically ambiguous                         | `;` = explicit separator (multiple statements on one line); newline may follow `;`              |

## Detailed Design

### Statement Termination Determination (Parser Rules)

In the expression Pratt loop, before applying a postfix operator (`(` call, `[` index, `.` field
access), check:

```
continuation(prev_expr, op_token) =
    prev_expr end line == op_token start line        // same line: normal postfix
    || (op_token == '.' and prev_expr ends with Identifier/')'/']')  // line-leading . chain
    || bracket depth > 0                             // inside unclosed brackets
```

If not satisfied → expression ends, new statement starts. Infix binary operators do not participate
in line checks (line-ending operators = continuation, naturally correct).

### Syntax Impact

- The `parse_expression` postfix branch adds line number checks (Span already carries line numbers,
  no need to modify the lexer token stream)
- Statement parsing sites consume `;` and newline boundaries (the `skip(Semicolon)` semantics are
  preserved)
- Block-ending `}` and file-ending EOF naturally terminate statements, no extra handling required
- **No NEWLINE token introduced** (Plan B, see Alternatives)—Span line numbers are sufficient, with
  minimal changes

### Compiler Changes

| Component                                    | Change                                                                                                                        |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/parser_state.rs`   | Line number check in `parse_expression` postfix operators                                                                     |
| `src/frontend/core/parser/statements/*.rs`   | Minor adjustments to statement boundary consumption logic                                                                     |
| `docs/src/reference/language-spec/syntax.md` | Add "Statement Termination Rules" subsection after §2.9                                                                       |
| Tests                                        | Add newline/continuation cases in `tests/yaoxiang/01-syntax/`; add line-leading swallowing regression in `06-compile-errors/` |

### Backward Compatibility

- **Vast majority of existing code requires zero changes**: identifier/literal-leading lines,
  line-ending operator breaks, newlines inside brackets all remain consistent with the current
  behavior
- **Behavior change points** (all are bug fix directions):
  - Cross-line swallowing of `f()(2)` / `1[99]` / `1.println()` changes from "silent/error" to "two
    independent statements"—correct semantics
  - Line-leading `.` changes from "swallowed (error)" to "explicit chain continuation"—new feature
- Risk: if there is legal code depending on "cross-line swallowing" (e.g., `f()\n(2)` intended to
  call the returned function), it will become `f()` + `(2)` as two statements. Such code is either
  semantically wrong or extremely rare and needs to be confirmed during RFC review

## Trade-offs

### Advantages

- **Few rules and intuitive**: two main rules + three groups of exceptions, all from practices
  validated by mainstream languages
- **Eliminates silent errors**: `f()(2)`, `1[99]` etc. are no longer silently swallowed; statement
  boundaries are predictable
- **Chain-call friendly**: line-leading `.` continuation aligns with Swift/industry standard style
- **Zero lexer changes**: the Span line number approach has minimal intrusion

### Disadvantages

- Line-leading binary operator continuation is not supported (Scala 3 style)—line breaks need to be
  wrapped in brackets, restricting a few styles
- Line-leading `.` continuation determination depends on "previous line ends with
  identifier/`)`/`]`", rules need to be documented

## Alternatives

| Plan                                              | Description                                                                | Why not chosen                                                                                                              |
| ------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| **A. Span line number awareness (this proposal)** | Parser checks line numbers in postfix operators                            | ✅ Adopted: minimal change, clear rules                                                                                     |
| **B. Go-style automatic semicolon insertion**     | Lexer nlsemi state machine, inserts `;` after specific tokens at line ends | Forces `{` at line end, forbids line-leading `./operators`, sacrifices chain style; inconsistent with YaoXiang's free style |
| **C. Mandatory semicolons**                       | Every statement must end with `;`                                          | Violates the established ecosystem of optional semicolons and existing test files                                           |
| **D. Maintain status quo**                        | No rules, token-driven                                                     | Defect #258 persists, silent statement evaporation continues                                                                |

## Implementation Strategy

### Dependencies

- No external dependencies; related to #258 (the implementation issue of this RFC)
- Orthogonal to RFC-010 (unified type syntax): statement termination is a parser-level rule and does
  not involve type grammar

### Risks

- The boundary between line-leading `.` chain and "line-leading `.` as independent statement":
  `.foo()` cannot stand alone (`.` is not a legal statement start), so the continuation
  determination has no ambiguity
- Existing tests that depend on swallowing behavior need to be checked one by one (expected to be
  none—swallowing is all bug scenarios)

## Open Questions

- [ ] Is line-leading binary operator continuation (Scala 3 style) worth supporting? (@ChenXu233:
      leaning toward not supporting; bracket wrapping is sufficient)
- [ ] Is a newline after `;` equivalent to an empty statement? The existing parser's
      `skip(Semicolon)` followed by a newline naturally continues, no special handling required
- [ ] Should an "unused expression result warning" (Swift style) be introduced as a further safety
      net against swallowing? Discussed in a separate RFC

---

## Appendix A: Multi-language Survey Comparison

| Language   | Approach                                              | Line-leading `(`      | Chain `.`            | Line-leading operator        | Evaluation                                      |
| ---------- | ----------------------------------------------------- | --------------------- | -------------------- | ---------------------------- | ----------------------------------------------- |
| Go         | Lexer automatic semicolon insertion (2 rules)         | ✅ Safe               | ❌ Forbidden         | ❌ Forbidden                 | Most deterministic, sacrifices chain style      |
| JavaScript | ASI 3 rules + restricted productions                  | ❌ **Famous pitfall** | Possible             | ⚠️ Pitfall                   | **Negative example** (`a\n(b)` → call)          |
| Python     | NEWLINE hard boundary + bracket implicit continuation | ✅ Safe               | ❌ Requires brackets | ❌ Requires brackets         | Most predictable, weakest continuation          |
| Kotlin     | SEMI = semicolon or newline                           | ✅ Safe               | ✅                   | ❌ (trailing lambda pitfall) | Good, but rules are hidden                      |
| Swift      | Newline termination + space rules                     | ✅ Safe               | ✅                   | ✅ Space protection          | **Closest to this proposal**                    |
| Scala 3    | nl token + region rules + leading operators           | ✅                    | ✅                   | ✅                           | Most complete but most complex, over-engineered |

**This proposal = Go's determinism × Python's bracket continuation × Swift's line-leading `.`
chain**. Swift is closest to the ideal; Scala 3 is the most complete but the rules are too
heavy—what's friendly to humans is not having the most rules, but **having few rules and each one
intuitive**.
