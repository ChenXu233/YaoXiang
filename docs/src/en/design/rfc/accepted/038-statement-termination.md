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

Defines YaoXiang's **statement termination rules**: **newlines as the primary boundary**, with
**unclosed brackets, trailing binary operators, and leading `.` (chain continuation)** as explicit
continuation exceptions; leading `(` and `[` **never merge into the previous statement**. `;` is
preserved as an explicit separator (multiple statements on one line).

This RFC simultaneously fills the specification gap in `syntax.md`, which has never defined a
statement terminator, and fixes the known parser defect of absorbing leading `(` `[` `.` on a new
line as suffixes to the previous statement.

## Motivation

### Why is this feature needed?

YaoXiang makes `;` fully optional (14 `skip(Semicolon)` calls in the parser), but **newlines have
never been defined as a statement boundary**: the lexer discards newlines, and the parser's only
statement boundary check is "the expression Pratt loop naturally stops"—i.e., "whether the next
token can continue the expression." This results in:

1. **Self-contradictory behavior**: a newline followed by an identifier/literal → normal
   termination; a newline followed by `(` `[` `.` → absorbed as a suffix of the previous expression
   (call/index/field access).
2. **Legal statements silently broken**: `(c, d) = (3, 4)` destructuring becomes `1(c, d) = (3, 4)`,
   raising the misleading E1001; `f()(2)`, `1[99]` don't even report an error—the statement simply
   evaporates.
3. **Specification gap**: `syntax.md` §2.9 `Block ::= '{' Stmt* Expr? '}'` does not define a
   separator between Stmts; §1.2's separator table only lists `( ) { } ,`.

### The Current Problem

| Code inside a block         | Current AST                                      | Result                             |
| --------------------------- | ------------------------------------------------ | ---------------------------------- |
| `x = 1` ⏎ `(c, d) = (3, 4)` | `Assign x = BinOp(Assign, Call(1,[c,d]), Tuple)` | E1001 pointing at the innocent `c` |
| `x = f()` ⏎ `(2)`           | `Call(Call(f),[2])`                              | **Silent** (statement evaporates)  |
| `x = 1` ⏎ `[99]`            | `Index(1,99)`                                    | **Silent**                         |
| `x = 1` ⏎ `.println("hi")`  | `Call(Field(1),["hi"])`                          | E1053                              |
| `x = 1 +` ⏎ `2`             | `BinOp(Add)`                                     | ✅ Legal (but no spec basis)       |
| `x = (1,` ⏎ `2)`            | `Tuple`                                          | ✅ Legal (but no spec basis)       |

## Proposal

### Core Design

**Main rule: a newline terminates a statement.**

**Continuation exceptions (three groups, all with mainstream language precedent):**

| #   | Exception                          | Rule                                                                                    | Precedent    |
| --- | ---------------------------------- | --------------------------------------------------------------------------------------- | ------------ |
| 1   | **Unclosed brackets**              | When the depth of `(` `[` `{` > 0, a newline does not terminate (implicit continuation) | Python/Scala |
| 2   | **Trailing binary operator**       | A line ending in a binary operator → continuation                                       | Swift/Scala  |
| 3   | **Leading `.` chain continuation** | Leading `.` and the previous line ends in an identifier/`)`/`]` → continuation          | Swift        |

**Never merge (absorbed JavaScript's biggest lesson):**

- Leading `(` and `[` → **always start a new statement**. JavaScript's notorious pitfall (`a\n(b)` →
  `a(b)` as a call) is especially dangerous in YaoXiang: tuple destructuring `(a, b) = ...`, spawn,
  and tuple literals are all bracket-leading statements.
- Leading binary/unary operators (`+` `-` `*` etc.) → start a new statement. Trailing-operator
  continuation already covers the common line-break style; leading-operator continuation (Scala 3's
  leading operator) introduces unary/infix ambiguity, so it's not adopted. If a line break is
  needed, wrap in parentheses.

**`;` is preserved**: an explicit separator, used for multiple statements on a single line (same as
Kotlin/Swift).

### Examples

```yaoxiang
// Newline terminates a statement (the vast majority of code)
a = 1
b = 2

// Continuation: trailing binary operator
total = a +
    b + c

// Continuation: unclosed brackets
t = (1,
     2)
io.println(
    "hi")

// Continuation: leading . (chained call)
result = list.map(x => x * 2)
    .filter(x => x > 10)
    .sum()

// Never merge: leading ( is an independent destructuring statement
x = f()
(c, d) = (3, 4)      // ✅ destructuring, not f()(c, d)

// Never merge: leading [ is an independent list
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
  - line ends in a binary operator
  - line starts with '.' and the previous line ends with Identifier | ')' | ']'
Never merge: leading '(' '[' always start a new statement
```

| Before                                                  | After                                                                                |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `Block ::= '{' Stmt* Expr? '}'` (no separator defined)  | `Block ::= '{' (Stmt StatementTerminator)* Expr? '}'`                                |
| Newline has no status; leading `(` `[` `.` are absorbed | Newline terminates; leading `(` `[` never merge; leading `.` explicitly continues    |
| `;` optional but semantically ambiguous                 | `;` = explicit separator (multiple statements on one line); a newline may follow `;` |

## Detailed Design

### Statement Termination Check (Parser Rule)

Within the expression Pratt loop, before applying a postfix operator (call `(` index `[` field
access `.`), check:

```
continuation(prev_expr, op_token) =
    prev_expr ending line == op_token starting line              // same line: normal suffix
    || (op_token == '.' and prev_expr ends in Identifier/')'/']') // leading . chain
    || bracket depth > 0                                          // inside unclosed brackets
```

If unsatisfied → expression ends, start a new statement. Infix binary operators do not participate
in line checks (trailing operator = continuation, naturally correct).

### Scope of Syntax Impact

- The `parse_expression` postfix branch adds a line-number check (Span carries line numbers, no
  lexer token-stream change needed)
- Statement parsing consumes `;` and newline boundaries (`skip(Semicolon)` semantics preserved)
- Block-ending `}` and end-of-file EOF naturally terminate statements; no extra handling needed
- **No NEWLINE token introduced** (Plan B, see Alternatives)—Span line numbers are sufficient, with
  minimal change

### Compiler Changes

| Component                                    | Change                                                                                                                               |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `src/frontend/core/parser/parser_state.rs`   | Line-number check in `parse_expression` postfix operators                                                                            |
| `src/frontend/core/parser/statements/*.rs`   | Minor adjustment to statement boundary consumption logic                                                                             |
| `docs/src/reference/language-spec/syntax.md` | Add a "Statement Termination Rules" subsection after §2.9                                                                            |
| Tests                                        | Add newline/continuation cases under `tests/yaoxiang/01-syntax/`; add leading-absorption regression cases under `06-compile-errors/` |

### Backward Compatibility

- **Vast majority of existing code requires no changes**: identifier/literal-leading lines,
  trailing-operator line breaks, and line breaks inside brackets all behave the same as before
- **Behavior changes** (all in the bug-fix direction):
  - Cross-line absorption of `f()(2)` / `1[99]` / `1.println()` changes from "silent/error" to "two
    independent statements"—the correct semantics
  - Leading `.` changes from "absorbed (error)" to "explicit chain continuation"—a new feature
- Risk: if any code legitimately depends on cross-line absorption (e.g., `f()\n(2)` intended to call
  a returned function), it will become `f()` + `(2)` as two statements. Such code is already
  semantically wrong or extremely rare; confirm during RFC review

## Trade-offs

### Advantages

- **Few rules and intuitive**: two main rules + three exception groups, all validated by mainstream
  language practice
- **Eliminates silent errors**: `f()(2)`, `1[99]` etc. are no longer silently absorbed; statement
  boundaries become predictable
- **Chain-call friendly**: leading `.` continuation aligns with Swift / industry standard style
- **Zero lexer changes**: the Span line-number approach is minimally invasive

### Disadvantages

- Leading binary-operator continuation is not supported (Scala 3 style)—line breaks require
  parentheses, restricting a few styles
- The leading `.` continuation check depends on "previous line ends in an identifier/`)`/`]`", which
  needs to be documented

## Alternatives

| Plan                                          | Description                                                               | Why not chosen                                                                                                              |
| --------------------------------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| **A. Span line-number aware (this proposal)** | Parser checks line numbers for postfix operators                          | ✅ Adopted: minimal change, clear rules                                                                                     |
| **B. Go-style automatic semicolon insertion** | Lexer nlsemi state machine, inserts `;` after specific tokens at line end | Forces `{` at line end, forbids leading `.`/operators, sacrifices chain style; incompatible with YaoXiang's free-form style |
| **C. Mandatory semicolons**                   | Every statement requires `;`                                              | Violates the existing semicolon-optional ecosystem and test-file status quo                                                 |
| **D. Status quo**                             | No rules, token-driven                                                    | Known defects persist; silent statement evaporation continues                                                               |

## Implementation Strategy

### Dependencies

- No external dependencies
- Orthogonal to RFC-010 (Unified Type Syntax): statement termination is a parser-layer rule and does
  not involve type grammar

### Risks

- Boundary between leading `.` chain continuation and "leading `.` as an independent statement":
  `.foo()` cannot form an independent statement (`.` is not a legal statement starter), so the
  continuation check is unambiguous
- Existing tests that depend on absorption behavior need to be checked one by one (none
  expected—absorption scenarios are all bug cases)

## Open Questions

- [ ] Is it worth supporting leading binary-operator continuation (Scala 3 style)? (@ChenXu233:
      leans toward no, parentheses suffice)
- [ ] Does a newline after `;` equate to an empty statement? The existing parser's `skip(Semicolon)`
      followed by a newline naturally continues, no special handling needed
- [ ] Should a "unused expression result" warning (Swift style) be introduced as further
      belt-and-suspenders against absorption? Discuss in a separate RFC

---

## Appendix A: Multi-language Survey Comparison

| Language   | Approach                                              | Leading `(`              | Chain `.`            | Leading operator             | Evaluation                                      |
| ---------- | ----------------------------------------------------- | ------------------------ | -------------------- | ---------------------------- | ----------------------------------------------- |
| Go         | Lexer automatic semicolon insertion (2 rules)         | ✅ Safe                  | ❌ Forbidden         | ❌ Forbidden                 | Most deterministic, sacrifices chain style      |
| JavaScript | ASI 3 rules + restricted productions                  | ❌ **Notorious pitfall** | Allowed              | ⚠️ Pitfall                   | **Cautionary tale** (`a\n(b)` → call)           |
| Python     | NEWLINE hard boundary + bracket implicit continuation | ✅ Safe                  | ❌ Requires brackets | ❌ Requires brackets         | Most predictable, weakest continuation          |
| Kotlin     | SEMI = semicolon or newline                           | ✅ Safe                  | ✅                   | ❌ (trailing lambda pitfall) | Good, rules hidden deep                         |
| Swift      | Newline terminates + space rules                      | ✅ Safe                  | ✅                   | ✅ Space protection          | **Closest to this proposal**                    |
| Scala 3    | nl token + region rules + leading operators           | ✅                       | ✅                   | ✅                           | Most complete but most complex, over-engineered |

**This proposal = Go's determinism × Python's bracket continuation × Swift's leading `.` chain**.
Swift is the closest to the ideal; Scala 3 is the most complete but its rules are too heavy—being
human-friendly is not about having the most rules, but about **having few rules that each match
intuition**.
