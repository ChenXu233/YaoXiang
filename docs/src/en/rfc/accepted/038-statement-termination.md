---
title: 'RFC-038: Statement Termination & Newline Rules'
author: 'ChenXu233'
created: '2026-08-05'
updated: '2026-08-05'
issue: '#258'
issues_impl:
  - '#258'
status: 'Accepted'
---

# RFC-038: Statement Termination & Newline Rules

## Summary

Defines YaoXiang's **statement termination rules**: **newline as the primary boundary**, with
**unclosed brackets, binary operators at end of line, and leading `.` (chained continuation)** as
explicit continuation exceptions; **leading `(` and `[` NEVER merge into the previous statement**.
`;` is retained as an explicit separator (multiple statements on a single line).

This RFC also fills the spec gap in `syntax.md` where the statement terminator was never defined,
and fixes the known parser defect where leading `(` `[` `.` are swallowed as suffixes of the
previous statement.

## Motivation

### Why is this feature needed?

YaoXiang's `;` is fully optional (the parser has 14 `skip(Semicolon)` calls), but **newline has
never been defined as a statement boundary**: the lexer discards newlines, and the parser's only
statement boundary determination is "the expression Pratt loop naturally stopping" — i.e., "whether
the next token can continue the expression". This causes:

1. **Self-contradictory behavior**: a newline followed by an identifier/literal → terminates
   normally; a newline followed by `(` `[` `.` → swallowed as a suffix of the previous expression
   (call/index/field access)
2. **Valid statements silently corrupted**: `(c, d) = (3, 4)` destructuring becomes
   `1(c, d) = (3, 4)`, producing the misleading E1001; `f()(2)`, `1[99]` don't even report an error,
   the statement simply evaporates
3. **Spec gap**: `syntax.md` §2.9 `Block ::= '{' Stmt* Expr? '}'` does not define the separator
   between Stmts; §1.2's separator table only has `( ) { } ,`

### Current problems

| Code in block               | Current AST                                      | Result                            |
| --------------------------- | ------------------------------------------------ | --------------------------------- |
| `x = 1` ⏎ `(c, d) = (3, 4)` | `Assign x = BinOp(Assign, Call(1,[c,d]), Tuple)` | E1001 pointing at innocent `c`    |
| `x = f()` ⏎ `(2)`           | `Call(Call(f),[2])`                              | **silent** (statement evaporates) |
| `x = 1` ⏎ `[99]`            | `Index(1,99)`                                    | **silent**                        |
| `x = 1` ⏎ `.println("hi")`  | `Call(Field(1),["hi"])`                          | E1053                             |
| `x = 1 +` ⏎ `2`             | `BinOp(Add)`                                     | ✅ legal (but no spec basis)      |
| `x = (1,` ⏎ `2)`            | `Tuple`                                          | ✅ legal (but no spec basis)      |

## Proposal

### Core design

**Primary rule: newline terminates a statement.**

**Continuation exceptions (three groups, all with mainstream language precedent):**

| #   | Exception                            | Rule                                                                             | Precedent    |
| --- | ------------------------------------ | -------------------------------------------------------------------------------- | ------------ |
| 1   | **Unclosed brackets**                | When `(` `[` `{` depth > 0, newline does NOT terminate (implicit continuation)   | Python/Scala |
| 2   | **Binary operator at end of line**   | A line ending with a binary operator → continuation                              | Swift/Scala  |
| 3   | **Leading `.` chained continuation** | Leading `.` and the previous line ends with an Identifier/`)`/`]` → continuation | Swift        |

**NEVER merge (lesson from JS's biggest pitfall):**

- Leading `(` and `[` → **ALWAYS start a new statement**. JS's notorious pitfall (`a\n(b)` → `a(b)`
  call) is especially dangerous in YaoXiang: tuple destructuring `(a, b) = ...`, spawn, tuple
  literals are all bracket-leading statements.
- Leading binary/unary operators (`+` `-` `*` etc.) → start a new statement. End-of-line operator
  continuation already covers common line-break styles; leading-operator continuation (Scala 3
  leading operators) would introduce unary/infix ambiguity, so we do NOT adopt it. If a line break
  is needed, wrap in parentheses.

**`;` is retained**: explicit separator, used for multiple statements on a single line (same as
Kotlin/Swift).

### Examples

```yaoxiang
// Newline terminates statement (vast majority of code)
a = 1
b = 2

// Continuation: binary operator at end of line
total = a +
    b + c

// Continuation: unclosed bracket
t = (1,
     2)
io.println(
    "hi")

// Continuation: leading . (chained call)
result = list.map(x => x * 2)
    .filter(x => x > 10)
    .sum()

// NEVER merge: leading ( is an independent destructuring statement
x = f()
(c, d) = (3, 4)      // ✅ destructuring, not f()(c, d)

// NEVER merge: leading [ is an independent list
x = 1
[1, 2, 3]            // ✅ independent expression statement

// Multiple statements on one line: semicolon
a = 1; b = 2
```

### Syntax changes

Add after `syntax.md` §2.9:

```
StatementTerminator ::= ';' | Newline        (unless the following continuation exceptions apply)
Continuation exceptions (newline does not terminate):
  - '(' '[' '{' depth > 0
  - Line ending with a binary operator
  - Line starts with '.' and previous line ends with Identifier | ')' | ']'
NEVER merge: leading '(' '[' always starts a new statement
```

| Before                                                 | After                                                                              |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `Block ::= '{' Stmt* Expr? '}'` (no separator defined) | `Block ::= '{' (Stmt StatementTerminator)* Expr? '}'`                              |
| Newline has no status; leading `(` `[` `.` swallowed   | Newline terminates; leading `(` `[` never merge; leading `.` explicit continuation |
| `;` optional but semantically ambiguous                | `;` = explicit separator (multiple statements per line), newline may follow `;`    |

## Detailed design

### Statement termination determination (parser rule)

In the expression Pratt loop, before applying a postfix operator (`(` call, `[` index, `.` field
access), check:

```
continuation(prev_expr, op_token) =
    prev_expr end-line == op_token start-line        // Same line: normal suffix
    || (op_token == '.' and prev_expr ends with Identifier/')'/']')  // Leading . chained
    || bracket-depth > 0                              // Inside unclosed brackets
```

If not satisfied → expression ends, new statement begins. Infix binary operators do not participate
in line check (line-ending operator = continuation, naturally correct).

### Syntax impact surface

- `parse_expression`'s postfix branch adds line-number check (Span already has line number, no lexer
  token stream changes needed)
- Statement parsing consumes `;` and newline boundary (`skip(Semicolon)` semantics retained)
- Block-ending `}` and file-end EOF naturally terminate statements, no extra handling needed
- **No NEWLINE token introduced** (Plan B, see alternatives) — Span line numbers are sufficient,
  minimal changes

### Compiler changes

| Component                                    | Change                                                                                                                |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/parser_state.rs`   | `parse_expression` postfix operator line check                                                                        |
| `src/frontend/core/parser/statements/*.rs`   | Minor adjustments to statement boundary consumption                                                                   |
| `docs/src/reference/language-spec/syntax.md` | Add "Statement Termination Rules" subsection after §2.9                                                               |
| Tests                                        | Add newline/continuation cases in `tests/yaoxiang/01-syntax/`; add leading-swallow regression in `06-compile-errors/` |

### Backward compatibility

- **Vast majority of existing code needs zero changes**: identifier/literal-leading lines,
  end-of-line operator line breaks, line breaks inside brackets are all consistent with current
  behavior
- **Behavior changes** (all in the bug-fix direction):
  - `f()(2)` / `1[99]` / `1.println()` cross-line swallowing changes from "silent/error" to "two
    independent statements" — correct semantics
  - Leading `.` changes from "swallowed (error)" to "explicit chained continuation" — new feature
- Risk: if any legitimate code depends on "cross-line swallowing" (e.g., `f()\n(2)` intended to call
  a returned function), it will become `f()` + `(2)` as two statements. Such code is already
  semantically wrong or extremely rare; must be confirmed during RFC review

## Trade-offs

### Advantages

- **Few rules, intuitive**: two main rules + three exception groups, all validated by mainstream
  language practice
- **Eliminates silent errors**: `f()(2)`, `1[99]` etc. no longer silently swallowed, statement
  boundaries are predictable
- **Chained call friendly**: leading `.` continuation aligns with Swift/industry standard style
- **Zero lexer changes**: Span line-number approach is minimally invasive

### Disadvantages

- Leading binary operator continuation is NOT supported (Scala 3 style) — requires wrapping in
  parentheses for line breaks, a few style limitations
- Leading `.` continuation determination depends on "previous line ends with Identifier/`)`/`]`",
  rule must be documented

## Alternatives

| Plan                                              | Description                                                                  | Why not chosen                                                                                                                |
| ------------------------------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| **A. Span line-number awareness (this proposal)** | Parser postfix operators check line number                                   | ✅ Adopted: minimal changes, clear rules                                                                                      |
| **B. Go-style automatic semicolon insertion**     | Lexer nlsemi state machine, inserts `;` after specific tokens at end of line | Forces `{` at end of line, prohibits leading `.`/operators, sacrifices chained style; inconsistent with YaoXiang's free style |
| **C. Mandatory semicolons**                       | Every statement requires `;`                                                 | Violates the existing semicolon-optional ecosystem and test file reality                                                      |
| **D. Status quo**                                 | No rules, token-driven                                                       | Known defects persist, silent statement evaporation                                                                           |

## Implementation strategy

### Dependencies

- No external dependencies
- Orthogonal to RFC-010 (Unified Type Syntax): statement termination is a parser-layer rule, doesn't
  involve type grammar

### Risks

- Boundary between leading `.` chained and "leading `.` as independent statement": `.foo()` cannot
  be an independent statement (`.` is not a valid statement start), so continuation determination is
  unambiguous
- Existing tests that depend on swallow behavior must be checked case by case (expected none —
  swallow scenarios are all bug scenarios)

## Open questions

- [ ] Is leading binary operator continuation (Scala 3 style) worth supporting? (@ChenXu233: leans
      toward not, parentheses are sufficient)
- [ ] Is a newline after `;` equivalent to an empty statement? Current parser's `skip(Semicolon)`
      followed by a newline naturally continues, no special handling needed
- [ ] Should we introduce an "unused expression result warning" (Swift style) as further safety net
      for swallowing? Discuss in a separate RFC

---

## Appendix A: Cross-language survey comparison

| Language   | Approach                                              | Leading `(`              | Chained `.`          | Leading operator             | Evaluation                                      |
| ---------- | ----------------------------------------------------- | ------------------------ | -------------------- | ---------------------------- | ----------------------------------------------- |
| Go         | Lexer automatic semicolon insertion (2 rules)         | ✅ Safe                  | ❌ Prohibited        | ❌ Prohibited                | Most deterministic, sacrifices chained style    |
| JavaScript | ASI 3 rules + restricted productions                  | ❌ **Notorious pitfall** | Allowed              | ⚠️ Pitfall                   | **Cautionary tale** (`a\n(b)` → call)           |
| Python     | NEWLINE hard boundary + bracket implicit continuation | ✅ Safe                  | ❌ Requires brackets | ❌ Requires brackets         | Most predictable, weakest continuation          |
| Kotlin     | SEMI = semicolon or newline                           | ✅ Safe                  | ✅                   | ❌ (trailing lambda pitfall) | Good, rules are hidden deep                     |
| Swift      | Newline terminates + whitespace rules                 | ✅ Safe                  | ✅                   | ✅ Whitespace guard          | **Closest to this proposal**                    |
| Scala 3    | nl token + region rules + leading operators           | ✅                       | ✅                   | ✅                           | Most complete but most complex, over-engineered |

**This proposal = Go's determinism × Python's bracket continuation × Swift's leading `.` chained
style**. Swift is closest to ideal; Scala 3 is most complete but rules are too heavy — being
human-friendly is not about having the most rules, but about **having few rules and each being
intuitive**.
